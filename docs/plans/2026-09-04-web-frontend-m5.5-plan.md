# Web frontend — M5.5 "the table reads": implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close M5, then make the table read at a glance — a crowd of cubes drawn as a pile in a wider band, the opponent's turn played out on the table event by event with a structured turn account beside it, and the first card chosen over the set table with the bot keeping its own card by the arena's rule — then deploy it to the NAS and review it live.

**Architecture:** Client-first with two additive server touches. A pure layout function decides pile-or-collapse from a glyph budget and the CSS divides the band by its sum; the walk is a client-side sequence over the existing replay route, whose frame gains the port and lab offers and whose live payload gains its event count; the setup preview becomes a throwaway position built by the same replay projection, with bot keeps settled by `score_starting_order`'s argmax reached through core. `crates/yokohama-core` and `crates/yokohama-cli` are untouched.

**Tech Stack:** React 19 + TypeScript + Vite (client), vitest + Testing Library over jsdom (component tests), Playwright against the real release binary (browser walk), the axum server (`crates/yokohama-web`), Pillow (the rect-measuring tool), the Portainer Docker proxy (deploy).

**Spec:** `docs/plans/2026-09-04-web-frontend-m5.5-spec.md` — the authority; this plan argues from it and records where it deviates (§ "Deviations" at the end, and the close in Task 16).

## Global Constraints

- **Branch:** `web-frontend`, in the worktree `.claude/worktrees/cli-web-api-port-936d56`. Every command runs from that worktree root unless it says `cd web`. Stage with explicit paths (`git add <path>`), never `git add -A`. Commit messages end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and the EXECUTING session's `Claude-Session:` trailer (M5 ruling R-P3: never the planning session's).
- **Blast radius (spec §4.5):** at the close, `git diff e4f41e4..HEAD -- crates/yokohama-core crates/yokohama-cli crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge` is EMPTY and `SEMANTICS_REV` is **7**. Only `crates/yokohama-web/**`, `web/**`, `docs/**`, `deploy/**`, `CLAUDE.md`, `README.md`, `guidingpath.md` move.
- **Per-edit gate (after EVERY task):** `cd web && npm run typecheck && npm test`. When a task touches `crates/`: also `cargo test -p yokohama-web` and `cargo clippy --workspace --all-targets -- -D warnings`. When a task touches `web/e2e/`, the layout, or the wire: also `cd web && npm run e2e` (needs `npx playwright install chromium` once; it builds `dist` and starts the release server itself).
- **The wire corpus is a contract.** Any task that changes a payload runs `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then `cargo test -p yokohama-web --test wire_fixtures` WITHOUT the variable, then `cd web && npm run typecheck`, and its report names every fixture file whose diff changed and why.
- **Below 1201px** the column stack and the bottom-pinned deck are unchanged (M5 spec R8). New layout rules go inside `@media (min-width: 1201px)` or apply at every width without changing the stack's mechanism.
- **Every number is arithmetic until a cell reads it** (spec §5). Wall and pixel readings are machine-tagged (this Mac, headless Chromium, DPR as stated).
- **Visual gates (spec §4.4): pictures delivered, no stop.** Tasks 4, 9 and 12 end with pictures sent to the owner by the orchestrator with `SendUserFile`; the run continues.
- **The art is third-party** (`ASSETS.md`). Review sheets go under gitignored `docs/temp/web-review/`; nothing derived from the art is committed outside the paths `ASSETS.md` lists.
- **No engine or CLI change** (spec R2, R3). If a task finds it needs one, STOP and report — it is a spec question, not a plan detail.
- Counts at HEAD `652badc` (the handover's §5 readings at `ca982ea`; re-measure at Task 0 and again at Task 16, never copy): `npm test` **296** in 19 files, `npm run e2e` **32** (+2 opt-in generators), `cargo test -p yokohama-web` **72**, wire corpus **43**, help pictures **5** (4,199,746 B), art **132** files.

---

## Execution model

| Seat | Model | Tasks |
|---|---|---|
| Implementer | **Opus** (xhigh) | 0, 1–12, 15, 16 |
| Per-task reviewer | **Opus**, fresh context | every task above |
| Orchestrator, adjudication, gate pictures | **Fable** | all |
| Deploy (outward-facing, owner-authorized) | the orchestrator through the Portainer MCP, as M5's Task 15 was | 13 |
| The final review against the LIVE version | **Fable** at max, a CLEAN subagent with the brief in Task 14 | 14 |

No Sonnet or Haiku dispatch: nothing here is a scripted chore (owner, 2026-09-02). Task 0 is docs-only and still gets a reviewer, because the close's counts and rows are the kind of sentence this lane has shipped false three times.

**Pre-flight (the orchestrator, before Task 0):** `git status` clean at `652badc` or later on `web-frontend`; `cd web && npm ci` done; `npx playwright install chromium` done; `python3 -c "import PIL"` succeeds; `cargo build --release -p yokohama-web` warm; `mkdir -p docs/temp/web-review/gates`. Record the executing session's `Claude-Session` URL for every commit trailer.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `docs/plans/2026-09-01-web-frontend-m5-spec.md` (§8 appended), `guidingpath.md`, `CLAUDE.md:33-34`, `README.md` | M5's close | 0 |
| `web/src/piecesLayout.ts` (+ `.test.ts`) | The pile-or-collapse rule as a pure function: `layoutPieces`, `bandAspect`, the unit table | 1 |
| `web/src/Pieces.tsx` (+ `.test.tsx`), `web/src/styles.css` (the `.pieces` block) | Draw piles; `--piece-units` replaces `--piece-n` | 2 |
| `docs/tools/tilerects.py`, `web/src/tile_rects.json`, `web/src/tileRects.ts` (+ `.test.ts`) | The measured left edge; `ICON_INK_RIGHT`; the retired disjointness cell | 3 |
| `web/e2e/the_table.spec.ts`, `web/e2e/gates.spec.ts` | The deterministic pile cell; gate A pictures | 4 |
| `crates/yokohama-web/src/client.rs`, `crates/yokohama-web/src/lib.rs` (tests), `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/*.json`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/types.ts`, `web/src/api.ts` | `events` on the live payload; `ports`/`labs` on the replay position; `log(tail)` | 5 |
| `web/src/diffFrames.ts` (+ `.test.ts`) | What changed between two positions, with no rules knowledge | 6 |
| `web/src/playback.ts` (+ `.test.ts`), `web/src/walk.ts` (+ `.test.ts`) | The cadence per browser; the run, the reducer, the dwell rule | 7 |
| `web/src/TurnAccount.tsx` (+ `.test.tsx`), `web/src/LastMove.tsx` (deleted), `web/src/LastMove.test.tsx` (deleted) | The turn account: rows, the lit step, the controls, prose when there is no run | 8 |
| `web/src/App.tsx` (+ `.test.tsx`), `web/src/TableSections.tsx` (+ `.test.tsx`), `web/src/Board.tsx`, `web/src/Dashboard.tsx`, `web/src/Strips.tsx`, `web/src/Help.tsx`, `web/src/test/harness.tsx`, `web/e2e/harness.ts`, `web/e2e/the_playback.spec.ts`, `web/e2e/gates.spec.ts`, `web/src/styles.css` | The walk on the table: the run fetch, the deck strip, strips from a frame, highlights, gate C | 9 |
| `crates/yokohama-web/src/games.rs`, `crates/yokohama-web/src/lib.rs` (tests), `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/preview*.json`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/types.ts`, `web/src/api.ts` | The preview as a position; `keeps` as `Option`; the bot's argmax | 11 |
| `web/src/NewGame.tsx` (+ `.test.tsx`), `web/src/App.tsx`, `web/src/TableSections.tsx`, `web/e2e/harness.ts`, `web/e2e/the_table.spec.ts`, `web/e2e/gates.spec.ts`, `web/e2e/shots.spec.ts`, `web/src/Help.tsx`, `web/public/help/*.png`, `README.md`, `web/src/styles.css` | The set table; the harness chooses a card; gate B; the help | 12 |
| `deploy/RUNBOOK.md` (§3.6) | The third deployment, `yokohama-web:0.5.0-m5.5` | 13 |
| `.superpowers/sdd/…/final-review.md` (gitignored) | The clean-subagent review against the live site | 14 |
| the fix wave's files, `deploy/RUNBOOK.md` (§3.6 addendum) | The findings, the scoped re-review, the redeploy | 15 |
| `docs/plans/2026-09-xx-web-frontend-m5.5-close.md`, `guidingpath.md`, `CLAUDE.md`, `README.md`, the memory file | The close, M6's entry artifact | 16 |

---

### Task 0: M5's close, with R9 as its verdict (spec §3.0)

**Files:**
- Modify: `docs/plans/2026-09-01-web-frontend-m5-spec.md` (append `## 8.`), `guidingpath.md` (one decision-log row, appended after the last row), `CLAUDE.md:33-34` (the two browser-half count lines) and the M4 bullet's neighbourhood (an M5 bullet beside it), `README.md:199-260` (re-check only; edit only what is false of `web/public/help/screen.png`)
- Read first: `docs/plans/2026-09-02-web-frontend-m5-execution-handover.md` §5–§7 (the counts, the review, what the close owes), `.superpowers/sdd/2026-09-01-web-frontend-m5-plan/task-16-brief.md` (the original brief this task executes), the M3.5 row at `guidingpath.md:206` (the row's shape)

**Interfaces:**
- Produces: the M5 spec's §8; the guidingpath row headed **M5 — "THE TABLE" — LANDED ON THE WEB LANE**; `CLAUDE.md` counts that Task 16 re-measures again.

- [ ] **Step 1: Re-measure at HEAD, never copy**

```bash
cd web && npm test 2>&1 | tail -3 && npm run e2e 2>&1 | tail -3 && cd .. \
  && cargo test -p yokohama-web 2>&1 | grep "test result" \
  && ls web/src/wire/*.json | wc -l \
  && ls web/public/help/*.png | wc -l && du -b web/public/help/*.png | awk '{s+=$1} END {print s}' \
  && find crates/yokohama-web/assets/art -type f | wc -l && du -sk crates/yokohama-web/assets/art \
  && git diff --stat 054e762..HEAD -- crates/ \
  && git diff --stat master..HEAD -- crates/yokohama-core | tail -1 \
  && grep -n "SEMANTICS_REV" crates/yokohama-core/src/game/semantics.rs | head -2
```

Write every figure down with the HEAD hash. Expected at `652badc`: vitest 296 in 19 files; e2e 32 passed; cargo 72 (66 lib + 1 gate + 5 wire_fixtures); 43 fixtures; 5 pictures, 4,199,746 B; 132 art files; the `crates/` diff since `054e762` only `assets/art/**` and `art.rs`; the core diff against master EMPTY; `SEMANTICS_REV` 7. If a figure differs, the figure is the truth and this plan's number is not.

- [ ] **Step 2: Append §8 to the M5 spec**

Append to `docs/plans/2026-09-01-web-frontend-m5-spec.md`:

```markdown
---

## 8. The close (2026-09-xx) — what shipped, and where the spec was wrong

**Verdict (owner, 2026-09-04, ruling R9 of the M5.5 spec):** the owner's two rounds of play on the deployed client are the V2/V3 verdict this close waited for. Pictures accepted; the pieces band's legibility is carried to M5.5 part A; the log box's shape to M5.5 part C.

**What shipped:** every W item of §4, with the verdict per item — W0 re-cut (+W0b sites turned), W1 rects (moved twice on the owner's V0/V1 answers, R-G1/R-G3), W2 grid, W2b Game menu, W3 fit rule, W4 art tile (+4b), W4c popover, W5a strips, W5b sections, W6 side column, W7/W8 help — LANDED; deployed as `yokohama-web:0.5.0-m5` then `0.5.0-m5.1` (the fix wave), 2026-09-02.

**The fit readings beside §3.3's predictions** (this Mac, headless Chromium, DPR 2): 2P 15.70 / 15.31 / 22.00 rem at 1440×900 / 1710×869 / 2560×1440; 3P 12.48 / 14.65 / 22.00; 4P 10.33 / 12.15 / 20.00; 10.00 (the floor) at 1440×600 for every count. §3.3's own 15.4 rem for 2P at 1710×869 does not follow from its formula, which gives 15.31 — a 0.09 rem slip in the spec, recorded here, not edited there.

**The payload delta:** the art went from 3.0 MB to 7.7 MB across the same 132 files (source-width crops); first paint 297,120 B; help 4,199,746 B across five pictures (runbook §3.4).

**Deviations from this spec, decided while planning and executing** — the eleven at the end of the plan, plus the ten in the execution handover §4 (Task 4b inserted for the site crops' quarter turn; the gates did not stop; opus from Task 3; the aspect test's two rows; the rects moved; two `--card-w` regimes; six V2 states; five help pictures; the deploy run by the controller; Task 16 deferred to M5.5's Task 0).

**Spec defects recorded forward:** S1–S7 and P1–P3 of the execution handover §6, verbatim by reference.

**Counts at the close** (HEAD `<hash>`): <the Step 1 figures>. `crates/` since `054e762`: only `assets/art/**` and `art.rs`'s two aspect rows; core against master EMPTY; `SEMANTICS_REV` 7.

**Not claimed:** §5 stands. M5 has no gate on the fifteen-gate roster and no roster place; the browser walk stays local by ruling.
```

Fill `<hash>` and the figures from Step 1. Commit: `docs(web): the M5 close — §8: the verdict by play, the readings beside the predictions, the counts`.

- [ ] **Step 3: The guidingpath row**

Append ONE row to the decision-log table in `guidingpath.md`, in the shape of the M3.5 row at `:206` (date column, a bold headline, one paragraph, a Why column). Content: **M5 — "THE TABLE" — LANDED ON THE WEB LANE; spec `docs/plans/2026-09-01-web-frontend-m5-spec.md`, its §8 the close.** The rulings R1–R11 in one sentence each; what the board gained (four rows in view at the owner's 1710×869, 15.31 rem tiles, measured); what moved (deck → side column; four drawers → strips under the pyramid; caption → art with the site card and pieces on it); the fit readings; the counts from Step 1; "still no gate and no roster place"; the verdict by play (R9) and the two findings carried to M5.5. Why column: *the top yield of the final review was again the milestone's own sentences (the Canal tile disproved the help's "each tile"; two captions were false of fresh frames) — try the mechanism, not the sentence.*

- [ ] **Step 4: CLAUDE.md and README**

In `CLAUDE.md`: line 33's `**231** component tests over jsdom in 10 files (M3.5: +27; M4: +9)` → the Step 1 figure `**296** … in 19 files (M3.5: +27; M4: +9; M5: +65)`; line 34's `**23** tests in 4 spec files (M3.5: +7; M4: +2)` → `**32** tests in 5 spec files (M3.5: +7; M4: +2; M5: +9), plus the two opt-in generators \`shots.spec.ts\` and \`gates.spec.ts\``, and name `the_table.spec.ts` in that line's list of what the walk covers. Add an M5 bullet directly after the M4 bullet (the one beginning `**M4 — the replay scrubber — IS BUILT`): *"**M5 — the table — IS BUILT and LIVE** (spec `…m5-spec.md`, its §8 the close; execution handover `…m5-execution-handover.md`): above 1201px a scrolling table of art-only tiles with the site card and the pieces on them beside a side column with the answer bar pinned at its top; the fit rule `clamp(10rem, min(byW, byH), 22rem)` with `web/src/fit.ts` as the oracle; the strips under the pyramid from ONE Boards menu; the help rewritten with five pictures. Deployed as `0.5.0-m5.1`. The rects live in `web/src/tile_rects.json`, one file for the code and the review sheet."* In `README.md` §"The browser client" (line 199 on): read it against `web/public/help/screen.png` and change only a sentence that is false of the picture; expect none.

- [ ] **Step 5: Verify, commit, delete the SDD workspace**

```bash
cd web && npm run typecheck && cd .. && git add docs/plans/2026-09-01-web-frontend-m5-spec.md guidingpath.md CLAUDE.md README.md \
  && git commit -m "docs: M5 closes — the verdict by play, the counts, the decision-log row" \
  && rm -rf .superpowers/sdd/2026-09-01-web-frontend-m5-plan && git status --short
```

Expected: a clean tree (the SDD directory was gitignored). Report the Step 1 figures and the row's text to the orchestrator.

---

### Task 1: `layoutPieces` — the pile-or-collapse rule as a pure function (spec §3.1.1)

**Files:**
- Create: `web/src/piecesLayout.ts`
- Test: `web/src/piecesLayout.test.ts`

**Interfaces:**
- Produces: `layoutPieces(crowd: Crowd, aspect: number, floor?: number): Layout`, `bandAspect(rect: {width; height}): number`, `PILE_FROM = 4`, `FLOOR = 0.55`, `UNIT`, `TILE_ASPECT = 709 / 531`, and the types `Crowd { presidents: number; cubes: number[]; station: boolean }`, `Layout { h; units; piles: number[]; collapsed: number[] }` (indices into `crowd.cubes`). Task 2 consumes all of them.

- [ ] **Step 1: Write the failing tests**

Create `web/src/piecesLayout.test.ts`:

```ts
// The rule for a crowd of pieces in a tile's band (M5.5 spec §3.1.1): every
// cube while the row stays legible, a two-row pile from four, one cube and a
// count only when the band cannot hold the crowd.

import { describe, expect, it } from "vitest";
import { bandAspect, FLOOR, layoutPieces, PILE_FROM, TILE_ASPECT, UNIT } from "./piecesLayout";

/** The mock's band (spec §0.2): left 0.30, width 0.67, height 0.123 of a
 *  709×531 tile — the aspect its readings were taken at. */
const MOCK = bandAspect({ width: 0.67, height: 0.123 });

describe("the band's aspect", () => {
  it("is the rect's aspect scaled by the tile's", () => {
    expect(TILE_ASPECT).toBeCloseTo(709 / 531, 6);
    expect(MOCK).toBeCloseTo((0.67 / 0.123) * (709 / 531), 3);
  });
});

describe("layoutPieces", () => {
  it("piles a colour from four and leaves three as singles", () => {
    expect(PILE_FROM).toBe(4);
    const one = layoutPieces({ presidents: 1, cubes: [4], station: false }, MOCK);
    expect(one.piles).toEqual([0]);
    expect(one.collapsed).toEqual([]);
    expect(one.h).toBe(1);
    // pawn 0.71 + a 2×2 pile (two columns of 0.5) + one gap between the two groups
    expect(one.units).toBeCloseTo(UNIT.pawn + 2 * UNIT.pileColumn + UNIT.gap, 6);
    const three = layoutPieces({ presidents: 0, cubes: [3], station: false }, MOCK);
    expect(three.piles).toEqual([]);
    expect(three.units).toBeCloseTo(3 * UNIT.cube + 2 * UNIT.inner, 6);
  });

  // The mock's five crowds at its band (spec §0.2's table, V3 column).
  it("draws every 2P crowd of the mock at full height", () => {
    const c2 = layoutPieces({ presidents: 2, cubes: [4, 3], station: false }, MOCK);
    expect(c2).toMatchObject({ piles: [0], collapsed: [], h: 1 });
    const c3 = layoutPieces({ presidents: 1, cubes: [6, 3], station: false }, MOCK);
    expect(c3).toMatchObject({ piles: [0], collapsed: [], h: 1 });
    const c5 = layoutPieces({ presidents: 2, cubes: [5, 4], station: false }, MOCK);
    expect(c5).toMatchObject({ piles: [0, 1], collapsed: [], h: 1 });
  });

  // The 17-glyph 4P crowd: four pawns, four threes, the Station. No group is
  // a pile (three each), the row misses the floor, and the rule collapses the
  // LARGEST group first — ties to the first — and stops as soon as the floor
  // holds, which here is after one.
  it("collapses the largest group first, and only until the floor holds", () => {
    const c4 = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, MOCK);
    expect(c4.piles).toEqual([]);
    expect(c4.collapsed).toEqual([0]);
    expect(c4.h).toBeGreaterThanOrEqual(FLOOR);
    expect(c4.h).toBeLessThan(0.62);
    // A collapsed group costs the cube, the inner gap and one digit.
    const singles = 3 * UNIT.cube + 2 * UNIT.inner;
    const collapsed = UNIT.cube + UNIT.inner + UNIT.digit;
    expect(c4.units).toBeCloseTo(4 * UNIT.pawn + 3 * singles + collapsed + UNIT.station + 8 * UNIT.gap, 6);
  });

  it("collapses more in a narrower band, and a pile is never also a collapse", () => {
    const narrow = bandAspect({ width: 0.55, height: 0.123 });
    const c4 = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, narrow);
    expect(c4.collapsed).toEqual([0, 1, 2]);
    const piles = layoutPieces({ presidents: 4, cubes: [9, 9, 9, 9], station: true }, 2);
    expect(piles.collapsed).toEqual([0, 1, 2, 3]);
    expect(piles.piles).toEqual([]);
  });

  it("reports the floor missed when no group is left to collapse", () => {
    const hopeless = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, 2);
    expect(hopeless.collapsed).toEqual([0, 1, 2, 3]);
    expect(hopeless.h).toBeLessThan(FLOOR);
    expect(hopeless.h).toBeGreaterThan(0);
  });

  // MUTATION PROOF of the floor: the same crowd at the same band does not
  // collapse when the floor is 0.5, so the decision above is the floor's and
  // not an accident of the arithmetic.
  it("owes its collapse to the floor", () => {
    expect(FLOOR).toBe(0.55);
    const lax = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, MOCK, 0.5);
    expect(lax.collapsed).toEqual([]);
  });

  it("an empty crowd is one unit at full height, so the CSS never divides by zero", () => {
    const none = layoutPieces({ presidents: 0, cubes: [], station: false }, MOCK);
    expect(none.units).toBe(1);
    expect(none.h).toBe(1);
  });
});
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd web && npx vitest run src/piecesLayout.test.ts`
Expected: FAIL — `Cannot find module './piecesLayout'`.

- [ ] **Step 3: Write the module**

Create `web/src/piecesLayout.ts`:

```ts
// How a crowd of pieces is drawn in a tile's band (M5.5 spec §3.1.1).
//
// Every cube is drawn while the row stays legible; a colour with four or more
// becomes a two-row PILE; a group collapses to one cube and a count only when
// the band cannot hold the crowd at the floor. The decision is made HERE, as a
// pure function of the crowd and the band's shape, and the CSS is handed the
// row's width in units of the glyph height (`--piece-units`) — so the
// stylesheet's shrink-to-fit rule and this function agree by construction:
// `h = min(H, W / units)` on both sides.
//
// Every horizontal cost is a multiple of the glyph height h, which is what
// keeps the fit PROVABLE (styles.css, the `.pieces` block): the row's width is
// `units × h` by definition, and h is chosen so that is at most W.

/** A colour with at least this many cubes is drawn as a two-row pile. */
export const PILE_FROM = 4;
/** The legibility floor: the glyph height must stay at or above this fraction
 *  of the band's height, else the largest group collapses to a count. The
 *  mock's number (spec §0.2); a knob, named here and nowhere else. */
export const FLOOR = 0.55;
/** Widths in units of the glyph height h. The pawn is the 20:28 viewBox at
 *  height h; a single cube is 0.72 h tall and 1:1; a pile column is two cubes
 *  of 0.5 h; the Station is 0.8 h and square; the gaps are the stylesheet's
 *  (0.1 h between groups, 0.05 h within one); a count digit is a 0.6 h font at
 *  about 0.6 em per digit. */
export const UNIT = {
  pawn: 0.71,
  cube: 0.72,
  pileColumn: 0.5,
  station: 0.8,
  gap: 0.1,
  inner: 0.05,
  digit: 0.36,
} as const;
/** The area art's cell aspect (709×531). Both rects in `tile_rects.json` are
 *  fractions of the tile, so a rect's width-to-height ratio in PIXELS is its
 *  fraction ratio times this. */
export const TILE_ASPECT = 709 / 531;

export interface Crowd {
  presidents: number;
  /** One entry per colour that has cubes here, each > 0. */
  cubes: number[];
  station: boolean;
}

export interface Layout {
  /** The glyph height as a fraction of the band's height; 1 is the whole band. */
  h: number;
  /** The row's width in units of h — the CSS divides the band's width by it. */
  units: number;
  /** Indices into `crowd.cubes` drawn as two-row piles. */
  piles: number[];
  /** Indices into `crowd.cubes` drawn as one cube and a count. */
  collapsed: number[];
}

/** The band's width-to-height ratio in pixels, from its rect. */
export const bandAspect = (rect: { width: number; height: number }): number =>
  (rect.width / rect.height) * TILE_ASPECT;

function groupWidth(n: number, pile: boolean, collapsed: boolean): number {
  if (collapsed) return UNIT.cube + UNIT.inner + UNIT.digit * String(n).length;
  if (pile) return UNIT.pileColumn * Math.ceil(n / 2);
  return UNIT.cube * n + UNIT.inner * (n - 1);
}

export function layoutPieces(crowd: Crowd, aspect: number, floor: number = FLOOR): Layout {
  const groups = crowd.presidents + crowd.cubes.length + (crowd.station ? 1 : 0);
  if (groups === 0) return { h: 1, units: 1, piles: [], collapsed: [] };
  const piles = crowd.cubes.map((n, i) => (n >= PILE_FROM ? i : -1)).filter((i) => i >= 0);
  const collapsed: number[] = [];
  const units = () =>
    UNIT.pawn * crowd.presidents +
    crowd.cubes.reduce(
      (sum, n, i) => sum + groupWidth(n, piles.includes(i), collapsed.includes(i)),
      0,
    ) +
    (crowd.station ? UNIT.station : 0) +
    UNIT.gap * (groups - 1);
  for (;;) {
    const u = units();
    const h = Math.min(1, aspect / u);
    if (h >= floor) {
      return { h, units: u, piles: piles.filter((i) => !collapsed.includes(i)), collapsed };
    }
    // The largest group not yet collapsed, ties to the first. None left means
    // the floor cannot be met and the caller gets the honest h.
    let largest = -1;
    crowd.cubes.forEach((n, i) => {
      if (!collapsed.includes(i) && (largest < 0 || n > crowd.cubes[largest]!)) largest = i;
    });
    if (largest < 0) return { h, units: u, piles: [], collapsed };
    collapsed.push(largest);
  }
}
```

- [ ] **Step 4: Run the tests**

Run: `cd web && npx vitest run src/piecesLayout.test.ts && npm run typecheck`
Expected: 7 passed; typecheck clean. If the `c4` cell's `h` bound (`< 0.62`) fails, print `c4.h` and check the arithmetic in the test's own comment before touching the module — the number is 7.273 / 12.35 ≈ 0.589.

- [ ] **Step 5: Commit**

```bash
git add web/src/piecesLayout.ts web/src/piecesLayout.test.ts
git commit -m "feat(web): A — the pile-or-collapse rule as a pure function over a glyph budget"
```

---

### Task 2: `Pieces` draws piles, and the band divides by the sum (spec §3.1.1, §3.1.3)

**Files:**
- Modify: `web/src/Pieces.tsx` (the `Pieces` component and the `CUBES_DRAWN` block, lines 52–120), `web/src/styles.css:392-431` (the proof comment and the `.pieces` block)
- Test: `web/src/Pieces.test.tsx` (three cells change, two are added)

**Interfaces:**
- Consumes: `layoutPieces`, `bandAspect`, `UNIT` from Task 1; `PIECES` from `web/src/tileRects.ts`.
- Produces: `.pieces` carries `style="--piece-units: <n>"` (three decimals); a pile is `span.piece-group.pile > span.pile-row × 2`; a collapsed group is exactly today's `one CubeIcon + span.piece-n`. `piecesWords` is unchanged and Task 9's `TilePopover`/`Board` callers keep working.

- [ ] **Step 1: Rewrite the failing cells**

In `web/src/Pieces.test.tsx` replace the cell `"draw one cube and a count from four"` and the cell `"tell the band how many glyphs stand in it"` with these four, and add the import:

```ts
import { bandAspect, layoutPieces } from "./piecesLayout";
import { PIECES } from "./tileRects";
```

```ts
  // From four a colour is a PILE: two rows, the odd cube on the top row, and
  // no count — the M5.5 rule (spec §3.1.1) that replaces "one cube and a 4".
  it("draw four as a two-row pile with no count", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 4], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    const pile = container.querySelector(".piece-group.pile");
    expect(pile).not.toBeNull();
    const rows = pile!.querySelectorAll(".pile-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]!.querySelectorAll(".piece.cube")).toHaveLength(2);
    expect(rows[1]!.querySelectorAll(".piece.cube")).toHaveLength(2);
    expect(container.querySelector(".piece-n")).toBeNull();
    expect(container.querySelector(".pieces")?.getAttribute("aria-label")).toBe("4 Red assistants");
  });

  it("put the odd cube on the top row", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 5], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    const rows = container.querySelectorAll(".pile-row");
    expect(rows[0]!.querySelectorAll(".piece.cube")).toHaveLength(3);
    expect(rows[1]!.querySelectorAll(".piece.cube")).toHaveLength(2);
  });

  // The 17-glyph 4P crowd is the one the band cannot hold at the floor: the
  // rule collapses its largest group (the first three) to a cube and a count,
  // and draws the other three colours' cubes. The decision is `layoutPieces`'s;
  // this cell proves the component OBEYS it rather than re-deciding.
  it("collapse to one cube and a count only where the layout says so", () => {
    const { container } = render(
      <Pieces
        presidents={[0, 1, 2, 3]}
        assistants={[[0, 3], [1, 3], [2, 3], [3, 3]]}
        hasStation
        colors={["Red", "Blue", "Yellow", "Green"]}
        art={wire.artManifest}
      />,
    );
    const expected = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, bandAspect(PIECES));
    expect(expected.collapsed.length, "the rule must collapse something for this cell to mean anything").toBeGreaterThan(0);
    expect(container.querySelectorAll(".piece-n")).toHaveLength(expected.collapsed.length);
    expect(container.querySelectorAll(".piece.cube")).toHaveLength(
      expected.collapsed.length + (4 - expected.collapsed.length) * 3,
    );
    expect(container.querySelector(".piece-group.pile")).toBeNull();
  });

  // The band's CSS divides its width by the row's width in glyph heights, and
  // the number it is given is the layout's own `units`, to three decimals.
  it("tell the band the row's width in units of the glyph height", () => {
    const units = (el: HTMLElement) =>
      el.querySelector<HTMLElement>(".pieces")!.style.getPropertyValue("--piece-units");
    const aspect = bandAspect(PIECES);
    const one = render(<Pieces presidents={[0]} assistants={[[0, 0], [1, 3]]} hasStation={false} colors={COLORS} art={wire.artManifest} />);
    expect(units(one.container)).toBe(layoutPieces({ presidents: 1, cubes: [3], station: false }, aspect).units.toFixed(3));
    const pile = render(<Pieces presidents={[]} assistants={[[0, 4], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />);
    expect(units(pile.container)).toBe(layoutPieces({ presidents: 0, cubes: [4], station: false }, aspect).units.toFixed(3));
    const many = render(<Pieces presidents={[0, 1]} assistants={[[0, 2], [1, 1]]} hasStation colors={COLORS} art={wire.artManifest} />);
    expect(units(many.container)).toBe(layoutPieces({ presidents: 2, cubes: [2, 1], station: true }, aspect).units.toFixed(3));
  });
```

- [ ] **Step 2: Run to see the four fail**

Run: `cd web && npx vitest run src/Pieces.test.tsx`
Expected: FAIL — no `.piece-group.pile`, `.piece-n` reads "4", `--piece-units` empty.

- [ ] **Step 3: Rewrite the component**

In `web/src/Pieces.tsx`, delete the `CUBES_DRAWN` constant and its comment (lines 52–57) and replace the `Pieces` function with:

```tsx
import { bandAspect, layoutPieces } from "./piecesLayout";
import { PIECES } from "./tileRects";

/** The band's shape, fixed by its rect: what `layoutPieces` fits the crowd to. */
const BAND_ASPECT = bandAspect(PIECES);

export function Pieces({
  presidents,
  assistants,
  hasStation,
  colors,
  art,
}: {
  presidents: PlayerId[];
  assistants: [PlayerId, number][];
  hasStation: boolean;
  colors: PlayerColor[];
  art: ArtManifest | null;
}) {
  const words = piecesWords(presidents, assistants, hasStation, colors);
  if (!words) return null;
  const station = tokenArt(art, "station");
  const groups = assistants.filter(([, n]) => n > 0);
  // The decision — pile, singles, or a cube and a count per colour — is the
  // pure rule's (M5.5 spec §3.1.1), and the band's CSS is handed the same
  // row width the rule computed, so the two cannot disagree about the fit.
  const layout = layoutPieces(
    { presidents: presidents.length, cubes: groups.map(([, n]) => n), station: hasStation },
    BAND_ASPECT,
  );
  return (
    // `title` carries the same words as the accessible name: the caption W4b
    // deleted was also the hover text, and a mouse user lost the only reading
    // of the pieces there was. `role="img"` keeps it one object either way.
    <div
      className="pieces"
      role="img"
      aria-label={words}
      title={words}
      style={{ "--piece-units": layout.units.toFixed(3) } as CSSProperties}
    >
      {presidents.map((seat) => (
        <span key={`p${seat}`} className={`piece-group ${seatClass(colors[seat])}`}>
          <PresidentIcon />
        </span>
      ))}
      {groups.map(([seat, n], i) => {
        const cls = `piece-group ${seatClass(colors[seat])}`;
        if (layout.collapsed.includes(i)) {
          return (
            <span key={`a${seat}`} className={cls}>
              <CubeIcon />
              <span className="piece-n">{n}</span>
            </span>
          );
        }
        if (layout.piles.includes(i)) {
          // Two rows, the odd cube on top; cubes in a pile touch, as on a table.
          const top = Math.ceil(n / 2);
          return (
            <span key={`a${seat}`} className={`${cls} pile`}>
              <span className="pile-row">
                {Array.from({ length: top }, (_, j) => <CubeIcon key={j} />)}
              </span>
              <span className="pile-row">
                {Array.from({ length: n - top }, (_, j) => <CubeIcon key={j} />)}
              </span>
            </span>
          );
        }
        return (
          <span key={`a${seat}`} className={cls}>
            {Array.from({ length: n }, (_, j) => <CubeIcon key={j} />)}
          </span>
        );
      })}
      {hasStation &&
        (station ? (
          <img className="piece station" src={station} alt="" loading="lazy" />
        ) : (
          <span className="chip station" title="Station — +1 Power for everyone here">
            St
          </span>
        ))}
    </div>
  );
}
```

Update the file's header comment: the cap is gone; the rule lives in `piecesLayout.ts`.

- [ ] **Step 4: The stylesheet**

In `web/src/styles.css`, replace the proof comment (lines 392–417) and the `.pieces` rule (419–423) so that the block reads:

```css
/* **THE FIT IS PROVED, NOT TUNED.** `--piece-units` (Pieces.tsx, from
 * `piecesLayout.ts`) is the row's width in units of the glyph height h — every
 * glyph, gap, pile column and count digit priced as a multiple of h — so with
 * h = min(H, W / units) the row is `units × h ≤ W` by definition, at every
 * crowd and every tile size. The clip on `.pieces-band` stays as a belt (a
 * pixel of drop shadow), never as the mechanism. The RULE that decides which
 * colour is a pile, a row of singles or a cube and a count is the pure
 * function's, and the same `units` it computed is what this block divides by,
 * which is what keeps the decision and the drawing from disagreeing.
 *
 * Pieces pack to the band's RIGHT edge: a small crowd sits where it always
 * did, under the site card, and only a large one grows leftwards over the art
 * (owner ruling R6 of the M5.5 spec). */
.pieces-band { position: absolute; z-index: 2; display: flex; align-items: center; container-type: size; overflow: hidden; }
.pieces {
  display: flex; flex-wrap: nowrap; align-items: flex-end; justify-content: flex-end; width: 100%; height: 100%;
  --piece-h: min(100cqh, calc(100cqw / var(--piece-units, 1)));
  gap: calc(var(--piece-h) * 0.1);
}
.piece-group { display: inline-flex; align-items: flex-end; gap: calc(var(--piece-h) * 0.05); height: 100%; }
/* A pile: two rows of half-height cubes that touch, the odd cube on top,
 * right-aligned so the top row's short end faces the open band. */
.piece-group.pile { flex-direction: column; align-items: flex-end; justify-content: flex-end; gap: 0; }
.pile-row { display: flex; gap: 0; }
.pile .piece.cube { height: calc(var(--piece-h) * 0.5); }
```

Keep the three sizing rules and `.piece-n` that follow exactly as they are.

- [ ] **Step 5: Run the gate**

Run: `cd web && npm test && npm run typecheck`
Expected: all green, the count up by two over Task 1's. `Board.test.tsx:295` and the two e2e reads of `.pieces`'s `aria-label` are unaffected (the words did not change).

- [ ] **Step 6: Commit**

```bash
git add web/src/Pieces.tsx web/src/Pieces.test.tsx web/src/styles.css
git commit -m "feat(web): A — piles from four, and the band divides by the row's width in glyph heights"
```

---

### Task 3: The band's left edge, measured against printed ink (spec §3.1.2)

**Files:**
- Modify: `docs/tools/tilerects.py` (two flags), `web/src/tile_rects.json:10`, `web/src/tileRects.ts` (two constants and their doc), `web/src/tileRects.test.ts` (one cell retired, two added)

**Interfaces:**
- Produces: `ICON_INK_RIGHT` and `BAND_MARGIN` exported from `tileRects.ts`; `PIECES.left` = the measured value; the contact sheet `docs/temp/web-review/tile-rects.png` with the new band drawn.

- [ ] **Step 1: The measurement, in the tool**

Add to `docs/tools/tilerects.py`, above `main()`:

```python
def is_pale(h: int, s: int, v: int) -> bool:
    """The pale-ground test the site-card rect was measured with (M5 spec §2)."""
    return v > 0.784 * 255 and s < 0.18 * 255


def icon_ink_right(im, band) -> float | None:
    """The rightmost printed-icon pixel inside the band's rows, as a fraction of
    the width, or None when the rows hold no icon.

    Only INK ON THE PALE PANEL counts: a non-pale pixel counts when a pale run
    of at least 3% of the width lies immediately to its right (inside x < 0.5).
    That is what separates an icon printed on the icon panel from the
    illustration, which is not pale and carries no data — the owner's
    criterion for the band (M5.5 spec R6: the rect may be entered, the ink may
    not be covered)."""
    w, h = im.size
    px = im.convert("HSV").load()
    y0, y1 = round(band["top"] * h), round((band["top"] + band["height"]) * h)
    needed = int(0.03 * w)
    right = None
    for y in range(y0, y1):
        run = 0
        for x in range(int(0.5 * w) - 1, -1, -1):
            if is_pale(*px[x, y]):
                run += 1
                continue
            if run >= needed:
                right = x / w if right is None else max(right, x / w)
                break
            run = 0
    return right
```

And in `main()`, before the loop, parse two flags, and inside the loop print and draw:

```python
    ink_mode = "--ink" in sys.argv
    candidates = []
    for arg in sys.argv[1:]:
        if arg.startswith("--candidates="):
            candidates = [float(c) for c in arg.split("=", 1)[1].split(",")]
    band = RECTS["overlays"]["pieces"]
    # …inside `for name, path in faces:` after `d = ImageDraw.Draw(im)`:
        if ink_mode:
            edge = icon_ink_right(im, band)
            print(f"{name:18s} icon ink right edge in the band's rows: {edge if edge is None else f'{edge:.3f}'}")
        for c in candidates:
            for yy in range(0, h, 6):
                d.line([(c * w, yy), (c * w, yy + 3)], fill=(255, 255, 0), width=2)
```

- [ ] **Step 2: Measure, and read the sheet**

Run: `python3 docs/tools/tilerects.py --ink --candidates=0.30,0.35,0.40,0.45`
Record the thirteen readings in the task report. L = the largest reading + 0.02, rounded UP to the nearest 0.005. Open `docs/temp/web-review/tile-rects.png` and LOOK: on every art the dashed candidate line nearest L must sit right of every printed icon in the band's rows. If the eye disagrees with the number (an icon the pale test missed), the eye wins, L moves right to clear it, and the report says which art and why. Expected, from the mock: Church binds near 0.33 and L lands near 0.35.

- [ ] **Step 3: Set the rect and the constants**

`web/src/tile_rects.json:10` → `"pieces": { "left": L, "top": 0.462, "width": <0.97 − L>, "height": 0.123 }` with both numbers written out to three decimals. In `web/src/tileRects.ts`, after `TOKEN_SLOT`:

```ts
/** The rightmost printed-icon ink inside the pieces band's rows, across the
 *  thirteen arts — `docs/tools/tilerects.py --ink`, measured <date> (Church
 *  binds at <reading>); the band's left edge is this plus the margin. The
 *  band deliberately ENTERS the icon-block rect (M5.5 spec R6): what it must
 *  not cover is ink, and the illustration is not data. */
export const ICON_INK_RIGHT = <the largest reading>;
export const BAND_MARGIN = 0.02;
```

Update the file's header comment: PIECES "takes the full width under the card" → "runs from the measured ink edge to the card's right edge, packed right".

- [ ] **Step 4: The geometry cells**

In `web/src/tileRects.test.ts`, replace `"no overlay covers a printed rule"` with:

```ts
  // The pieces band may enter the ICON BLOCK — M5.5's ruling R6, on a measured
  // ink edge — and nothing else may enter anything.
  it("no overlay covers a printed rule, except the band over the icon block's bare ground", () => {
    for (const [name, r] of Object.entries(OVERLAYS)) {
      for (const [printed, p] of Object.entries(PRINTED)) {
        if (name === "pieces" && printed === "icon_block") continue;
        expect(intersects(r, p), `${name} covers the ${printed}`).toBe(false);
      }
    }
  });

  it("the band starts right of the measured icon ink, by the margin", () => {
    expect(PIECES.left).toBeGreaterThanOrEqual(ICON_INK_RIGHT + BAND_MARGIN - 1e-9);
    expect(PIECES.left, "the band was widened past the mock's own 0.30 for nothing").toBeLessThan(0.5);
    expect(PIECES.left + PIECES.width).toBeCloseTo(0.97, 6);
  });
```

Import `BAND_MARGIN` and `ICON_INK_RIGHT`. Keep every other cell; `"would catch a band that drifts onto the 5-space"` and the three-thousandths cell still hold (the top edge did not move).

- [ ] **Step 5: Gate, regenerate the sheet, commit**

Run: `cd web && npm test && npm run typecheck && cd .. && python3 docs/tools/tilerects.py`
Expected: green; the sheet regenerated with the wider band in green. Commit:

```bash
git add docs/tools/tilerects.py web/src/tile_rects.json web/src/tileRects.ts web/src/tileRects.test.ts
git commit -m "feat(web): A — the band's left edge measured against printed ink, and the tool that measured it"
```

Report the thirteen readings, L, and the sheet's path; the orchestrator sends the sheet to the owner with Task 4's pictures.

---

### Task 4: A pile exists deterministically, and gate A's pictures (spec §4.3, §4.4)

**Files:**
- Modify: `web/e2e/the_table.spec.ts` (one cell appended), `web/e2e/gates.spec.ts` (nothing — the V1 flow is re-run as is)

**Interfaces:**
- Consumes: `.piece-group.pile`, `.pile-row`, `.piece.cube`, `.piece-n` from Task 2; `startGame`, `readScreen`, `watchForBreakage` from `web/e2e/harness.ts`.

- [ ] **Step 1: Write the cell**

Append to `web/e2e/the_table.spec.ts` (add `import { mkdirSync } from "node:fs";` at the top):

```ts
// ---- M5.5 A: a pile of four, built on purpose ------------------------------

/** Click the first enabled option row whose words match, or say there is none. */
async function answer(page: Page, words: RegExp): Promise<boolean> {
  const rows = page.locator("button.option:not([disabled])").filter({ hasText: words });
  if ((await rows.count()) === 0) return false;
  await rows.first().click();
  return true;
}

test("four assistants of one colour stand as a pile, not as a cube and a count", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  // Option B twice into the SAME area, with the President left in hand on
  // both turns: a President that acts in an area recovers its cubes at the
  // Recovery step, so the pile forms only if it never goes there. The bot's
  // turn in between cannot recover another seat's cubes, and its President
  // parking there at most prices the second placement at two yen.
  let area: string | null = null;
  let placements = 0;
  for (let i = 0; i < 40 && placements < 2; i++) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before two placements");
    if (await answer(page, /Option B/)) {
      const tiles = page.locator("button.tile:not([disabled])");
      const target =
        area === null
          ? tiles.first()
          : tiles.filter({ has: page.locator(`.tile-id:text-is("${area}")`) }).first();
      if (area !== null && (await target.count()) === 0) {
        throw new Error(`${area} is not offered for the second placement`);
      }
      area ??= (await target.locator(".tile-id").innerText()).trim();
      await target.click();
      await page.getByRole("button", { name: "Confirm" }).click();
      placements += 1;
      continue;
    }
    if (await answer(page, /Return President to hand/)) continue;
    if (screen.only) {
      await page.getByRole("button", { name: "Apply the only option" }).click();
      continue;
    }
    if (await answer(page, /End additional-action phase|Skip the/)) continue;
    throw new Error(
      `no scripted answer for this screen:\n${await page.locator(".decision").first().innerText()}`,
    );
  }
  expect(placements, "two placements were made").toBe(2);

  const tile = page.locator(".board .tile").filter({ has: page.locator(`.tile-id:text-is("${area}")`) });
  const pile = tile.locator(".piece-group.pile");
  await expect(pile).toHaveCount(1);
  await expect(pile.locator(".piece.cube")).toHaveCount(4);
  await expect(tile.locator(".piece-n")).toHaveCount(0);
  const cube = await pile.locator(".piece.cube").first().boundingBox();
  // RECORDED, never gated (spec §5): the height depends on the fit rule's tile
  // width on this machine at this viewport.
  console.log(`pile cube at 1710×869, 2P, on ${area}: ${(cube?.height ?? 0).toFixed(1)}px tall`);
  expect(cube?.height ?? 0).toBeGreaterThan(0);
  mkdirSync("../docs/temp/web-review/gates", { recursive: true });
  await page.screenshot({ path: "../docs/temp/web-review/gates/a-pile-1710x869.png" });
  expect(watch.failures).toEqual([]);
});
```

- [ ] **Step 2: Run it**

Run: `cd web && npm run e2e -- the_table.spec.ts -g "pile"`
Expected: PASS, with the recorded height printed (expect ~11px at the 2P fit if L landed near 0.35; the mock read 11.19 at 0.30). If the loop throws "no scripted answer", the report quotes the screen — a phase the script did not foresee is a finding, not a reason to loosen the assertion.

- [ ] **Step 3: The whole browser suite, then the gate pictures**

Run: `cd web && npm run e2e` (expected: the previous count + 1, all green) and then
`cd web && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts -g "V1"`.
Expected: `docs/temp/web-review/gates/v1-{2,4}p-1710x869.png` regenerated with the new band.

- [ ] **Step 4: Commit and hand over the pictures**

```bash
git add web/e2e/the_table.spec.ts
git commit -m "test(web): A — a four-cube pile built on purpose, its height recorded, and the gate pictures"
```

Report the recorded height and the four files: `a-pile-1710x869.png`, `v1-2p-1710x869.png`, `v1-4p-1710x869.png`, `docs/temp/web-review/tile-rects.png`. The orchestrator sends them to the owner (no stop).

---

### Task 5: The wire — `events` on the live payload, the offers on the replay position (spec §3.2.1, §4.1)

**Files:**
- Modify: `crates/yokohama-web/src/client.rs` (`ClientState`, `ClientState::of`, `HistoricalFrame`, `HistoricalFrame::at`), `crates/yokohama-web/src/lib.rs` (two route tests in `mod tests`), `crates/yokohama-web/tests/wire_fixtures.rs` (nothing — regenerated), `web/src/wire/*.json` (regenerated), `web/src/wire/contract.ts` (`CLIENT_STATE`, `HISTORICAL_FRAME`), `web/src/wire/contract.test.ts` (one anti-vacuity cell), `web/src/types.ts` (`ClientState.events`, `HistoricalFrame.ports/labs`), `web/src/api.ts` (`log(game, tail?)`)

**Interfaces:**
- Produces: `ClientState.events: usize` (the record's length); `HistoricalFrame.ports: PortsPanel`, `HistoricalFrame.labs: LabsPanel`; client types to match; `api.log(game, tail?)` → `/log?tail=n`. Tasks 6–9 consume all four.

- [ ] **Step 1: The failing route cells**

In `crates/yokohama-web/src/lib.rs`'s `mod tests`, beside `the_replay_pin_does_not_move_when_the_turn_does`, using that test's own helpers (`a_game`, `play`, `total_events`, `call`):

```rust
    /// The live payload counts its events, so a client that answered from
    /// `events: n` and receives `events: m > n` knows the reply applied
    /// `[n, m)` — what the turn playback walks (M5.5 spec §3.2.2).
    #[tokio::test]
    async fn the_live_payload_counts_its_events() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "count", &[]).await;
        let (_, fresh) = call(&router, "GET", "/api/games/count", None).await;
        assert_eq!(fresh["events"], serde_json::json!(0), "{fresh}");
        play(&router, "count", 5).await;
        let total = total_events(&router, "count").await;
        assert!(total > 0, "`play` applied nothing, so this cell proves nothing");
        let (_, later) = call(&router, "GET", "/api/games/count", None).await;
        assert_eq!(later["events"].as_u64(), Some(total as u64));
    }

    /// The replay position carries the port and lab offers AT k — built from
    /// `state_at(k)`, never copied from the live state (M5.5 spec §3.2.1).
    #[tokio::test]
    async fn the_replay_position_carries_the_offers_at_k() {
        let router = app(Arc::new(AppState::in_memory()));
        a_game(&router, "offers", &[]).await;
        play(&router, "offers", 6).await;
        let total = total_events(&router, "offers").await;
        let (_, at_end) = call(&router, "GET", &format!("/api/games/offers/at/{total}"), None).await;
        let (_, live_ports) = call(&router, "GET", "/api/games/offers/ports", None).await;
        let (_, live_labs) = call(&router, "GET", "/api/games/offers/labs", None).await;
        assert_eq!(at_end["ports"], live_ports, "at k == total the offers are the live ones");
        assert_eq!(at_end["labs"], live_labs);
        let (_, at_start) = call(&router, "GET", "/api/games/offers/at/0", None).await;
        let full = at_start["ports"]["boards"][0]["slots"]
            .as_array()
            .expect("six slots")
            .iter()
            .filter(|s| !s["card"].is_null())
            .count();
        assert_eq!(full, 6, "the port is full after setup: {}", at_start["ports"]);
        // Built from the POSITION: the panel's own deck count is the position's.
        for k in 0..=total {
            let (status, h) = call(&router, "GET", &format!("/api/games/offers/at/{k}"), None).await;
            assert_eq!(status, StatusCode::OK, "k={k}");
            assert_eq!(h["ports"]["deck_remaining"], h["deck_remaining"], "k={k}: {}", h["ports"]);
        }
    }
```

Run: `cargo test -p yokohama-web the_live_payload_counts_its_events the_replay_position_carries_the_offers_at_k`
Expected: FAIL — `fresh["events"]` is `Null`; `at_end["ports"]` is `Null`.

- [ ] **Step 2: The server**

In `crates/yokohama-web/src/client.rs`:

```rust
// in `pub struct ClientState`, after `warnings`:
    /// The record's length — how many events this game has applied.
    ///
    /// A client that answered from a payload carrying `events: n` and gets
    /// back `events: m > n` knows the reply applied the events `[n, m)`,
    /// which is exactly the run the turn playback walks (M5.5 spec §3.2.2). A
    /// count and not an event list: the walk reads the positions from
    /// `/at/{k}` and the words from `/log`, both of which already exist.
    pub events: usize,

// in `ClientState::of`, in the struct literal:
            events: session.record().events.len(),

// in `pub struct HistoricalFrame`, after `scores`:
    /// The port and laboratory offers AT `k` — face up at the time, and
    /// properties of the position, which is the test M4's Q5 applied to keep
    /// this frame narrow (the fields it dropped were present-tense lies at a
    /// past position; these are not). The turn playback needs them so a port
    /// take shows the card leaving (M5.5 spec §3.2.1).
    pub ports: screen_model::PortsPanel,
    pub labs: screen_model::LabsPanel,

// in `HistoricalFrame::at`, in the struct literal:
            ports: screen_model::build_ports(&state, rules),
            labs: screen_model::build_labs(&state, rules),
```

Run the two cells again: PASS. Then `cargo test -p yokohama-web` — expect `wire_fixtures` to FAIL naming `at_k.json` and every `ClientState` fixture as drifted, which is the contract doing its job.

- [ ] **Step 3: Regenerate the corpus and READ the diff**

```bash
UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures \
  && cargo test -p yokohama-web --test wire_fixtures \
  && git diff --stat web/src/wire/
```

Expected: every `ClientState` fixture gains one `"events": n` line; `at_k.json` gains `ports` and `labs` objects; no other file moves. The report lists the files and states the `at_k.json` k and its `deck_remaining`.

- [ ] **Step 4: The client's half of the contract**

`web/src/types.ts`: in `ClientState` add `/** The record's length; see the server's doc. */ events: number;` and in `HistoricalFrame` add `ports: PortsPanel; labs: LabsPanel;` with the M4-Q5 sentence from Step 2 as a comment. `web/src/wire/contract.ts`: add `"events"` to `CLIENT_STATE` and `"ports", "labs"` to `HISTORICAL_FRAME`. `web/src/api.ts`: 

```ts
  log: (game: string, tail?: number) =>
    call<LogPanel>("GET", `/api/games/${id(game)}/log${tail === undefined ? "" : `?tail=${tail}`}`),
```

In `web/src/wire/contract.test.ts`, add beside the `at_k` root check:

```ts
  // Anti-vacuity for the widened replay frame: the position the corpus
  // captured (the game's midpoint) must hold a port that DIFFERS from the
  // opening offer in `ports.json`, or the two fields could be a copy of the
  // live panels and nothing here would know.
  it("at_k carries the port offer of ITS position, not the opening's", () => {
    expect(wire.atK.ports).not.toEqual(wire.ports);
    expect(wire.atK.ports.boards[0]!.slots).toHaveLength(6);
    expect(wire.atK.labs.boards[0]!.slots).toHaveLength(6);
  });
```

If that cell fails because no port card moved by the midpoint, change the capture in `wire_fixtures.rs` (the `at_k` push near line 505) from `total / 2` to the first `k` at which the port differs — found by a loop in the test — and say so in the report; do not weaken the cell.

- [ ] **Step 5: The gate, and commit**

```bash
cd web && npm run typecheck && npm test && cd .. \
  && cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings
```

Expected: green (the vitest count +1). Commit:

```bash
git add crates/yokohama-web/src/client.rs crates/yokohama-web/src/lib.rs web/src/wire web/src/types.ts web/src/api.ts
git commit -m "feat(web): C — the live payload counts its events, the replay position carries the offers at k"
```

---

### Task 6: `diffFrames` — what changed between two positions (spec §3.2.4)

**Files:**
- Create: `web/src/diffFrames.ts`
- Test: `web/src/diffFrames.test.ts`

**Interfaces:**
- Consumes: `HistoricalFrame` (with `ports`/`labs`, Task 5), `wire.atK`.
- Produces: `diffFrames(a, b): Changes` with `Changes { tiles: Set<number>; seats: Set<number>; slots: Set<string>; any: boolean }`; slot keys `port:<board>:<slot>` and `lab:<board>:<slot>`. Tasks 7 and 9 consume it.

- [ ] **Step 1: The failing tests**

Create `web/src/diffFrames.test.ts`:

```ts
// What changed between two positions, read off the wire and never off a rule.

import { describe, expect, it } from "vitest";
import { diffFrames } from "./diffFrames";
import { wire } from "./wire/contract";
import type { HistoricalFrame } from "./types";

const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

describe("diffFrames", () => {
  it("sees nothing between a position and itself", () => {
    const d = diffFrames(wire.atK, clone(wire.atK));
    expect(d.any).toBe(false);
    expect(d.tiles.size + d.seats.size + d.slots.size).toBe(0);
  });

  it("names exactly the tile whose assistants moved", () => {
    const b = clone(wire.atK);
    const tile = b.frame.board.tiles[3]!;
    tile.assistants[0]![1] += 1;
    const d = diffFrames(wire.atK, b);
    expect([...d.tiles]).toEqual([tile.area]);
    expect(d.seats.size).toBe(0);
    expect(d.any).toBe(true);
  });

  it("sees a president arriving, the Station, and a space being built on", () => {
    const b = clone(wire.atK);
    const [t0, t1, t2] = b.frame.board.tiles as [typeof b.frame.board.tiles[0], typeof b.frame.board.tiles[0], typeof b.frame.board.tiles[0]];
    t0.presidents = [...t0.presidents, 1];
    t1.has_station = !t1.has_station;
    if (t2.site) t2.site.slots[0]!.built_by = 0;
    const d = diffFrames(wire.atK, b);
    expect(d.tiles).toEqual(new Set([t0.area, t1.area, ...(t2.site ? [t2.area] : [])]));
  });

  it("names the seat whose money or goods moved", () => {
    const b = clone(wire.atK);
    const players = b.frame.dashboard.find((p) => "Players" in p);
    if (!players || !("Players" in players)) throw new Error("the fixture carries no Players panel");
    players.Players.players[1]!.money += 2;
    const d = diffFrames(wire.atK, b);
    expect([...d.seats]).toEqual([players.Players.players[1]!.seat]);
    expect(d.tiles.size).toBe(0);
  });

  it("names a port slot whose card changed, and a lab slot, by board and slot", () => {
    const b = clone(wire.atK);
    b.ports.boards[0]!.slots[2]!.card = null;
    b.labs.boards[0]!.slots[5]!.card = 999;
    const d = diffFrames(wire.atK, b);
    expect(d.slots).toEqual(new Set(["port:0:2", "lab:0:5"]));
  });

  // MUTATION PROOF: a field the diff does not read leaves it silent, so the
  // cells above are seeing the fields they name and not "anything at all".
  it("ignores what is not on the table — the reachable mark", () => {
    const b = clone(wire.atK);
    b.frame.board.tiles[0]!.reachable = !b.frame.board.tiles[0]!.reachable;
    expect(diffFrames(wire.atK, b).any).toBe(false);
  });
});
```

Run: `cd web && npx vitest run src/diffFrames.test.ts` — expected FAIL, module missing.

- [ ] **Step 2: The module**

Create `web/src/diffFrames.ts`:

```ts
// What changed between two positions (M5.5 spec §3.2.4).
//
// A pure comparison of two wire frames and nothing else: the walk lights the
// tiles, seats and slots this names, and it must never need a rule to do so —
// a rule restated here would be a second dialect of the engine.

import type { AreaId, HistoricalFrame, Panel, PlayerId, PlayerModel, TileModel } from "./types";

export interface Changes {
  tiles: Set<AreaId>;
  seats: Set<PlayerId>;
  /** `port:<board>:<slot>` and `lab:<board>:<slot>`. */
  slots: Set<string>;
  any: boolean;
}

/** The parts of a tile a piece or a build can change. `reachable` is a
 *  property of the legal list, not of the table, and is deliberately absent. */
const tileKey = (t: TileModel): string =>
  JSON.stringify([
    t.presidents,
    t.assistants,
    t.has_station,
    t.site?.slots.map((s) => s.built_by) ?? null,
    t.site?.trading_house_owner ?? null,
  ]);

const seatKey = (p: PlayerModel): string =>
  JSON.stringify([
    p.money, p.victory_points, p.goods, p.assistants, p.shophouses, p.trading_houses,
    p.technologies.map((t) => t.id), p.mercenaries_unused, p.mercenaries_used,
    p.five_power_tokens, p.president, p.orders_in_hand, p.orders_filled,
  ]);

const players = (f: HistoricalFrame): PlayerModel[] =>
  f.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> => "Players" in p)
    ?.Players.players ?? [];

const slotKeys = (f: HistoricalFrame): [string, number | null][] => [
  ...f.ports.boards.flatMap((b, i) => b.slots.map((s): [string, number | null] => [`port:${i}:${s.slot}`, s.card])),
  ...f.labs.boards.flatMap((b, i) => b.slots.map((s): [string, number | null] => [`lab:${i}:${s.slot}`, s.card])),
];

export function diffFrames(a: HistoricalFrame, b: HistoricalFrame): Changes {
  const tiles = new Set<AreaId>();
  const wasTile = new Map(a.frame.board.tiles.map((t) => [t.area, tileKey(t)]));
  for (const t of b.frame.board.tiles) if (wasTile.get(t.area) !== tileKey(t)) tiles.add(t.area);

  const seats = new Set<PlayerId>();
  const wasSeat = new Map(players(a).map((p) => [p.seat, seatKey(p)]));
  for (const p of players(b)) if (wasSeat.get(p.seat) !== seatKey(p)) seats.add(p.seat);

  const slots = new Set<string>();
  const wasSlot = new Map(slotKeys(a));
  for (const [key, card] of slotKeys(b)) if (wasSlot.get(key) !== card) slots.add(key);

  return { tiles, seats, slots, any: tiles.size + seats.size + slots.size > 0 };
}
```

- [ ] **Step 3: Run, gate, commit**

Run: `cd web && npx vitest run src/diffFrames.test.ts && npm run typecheck` — expected: 6 passed.

```bash
git add web/src/diffFrames.ts web/src/diffFrames.test.ts
git commit -m "feat(web): C — diffFrames, what changed between two positions, read off the wire"
```

---

### Task 7: The cadence, the run and the walk's reducer (spec §3.2.2, §3.2.3)

**Files:**
- Create: `web/src/playback.ts`, `web/src/walk.ts`
- Test: `web/src/playback.test.ts`, `web/src/walk.test.ts`

**Interfaces:**
- Consumes: `diffFrames`/`Changes` (Task 6); `HistoricalFrame`, `LogPanel` (Task 5).
- Produces (Tasks 8 and 9 consume every name):
  - `playback.ts`: `CADENCES = [1000, 2000, 3000]`, `type Cadence`, `DEFAULT_CADENCE = 2000`, `QUICK_DWELL = 400`, `loadCadence(storage?)`, `saveCadence(ms, storage?)`, key `yokohama.playback.cadence`.
  - `walk.ts`: `Step { event; line; mover; moverIsBot; phase: TurnPhase | null; before; after; changes }`, `Run { from; to; steps }`, `buildRun(from, frames, log)`, `hasBotStep(run)`, `dwellFor(step, cadence)`, `WalkState { at; playing }`, `WalkAction`, `walkReducer(state, action, last): WalkState | null` (`null` = the walk is over).

- [ ] **Step 1: The cadence's failing tests**

Create `web/src/playback.test.ts`:

```ts
// The playback cadence: three settings, remembered per browser, the default
// when storage is absent, throwing or holding nonsense (the `sections.ts` idiom).

import { describe, expect, it } from "vitest";
import { CADENCES, DEFAULT_CADENCE, loadCadence, QUICK_DWELL, saveCadence } from "./playback";

function fakeStorage(initial?: string, throwing = false): Storage {
  let value = initial;
  return {
    getItem: () => {
      if (throwing) throw new Error("no storage here");
      return value ?? null;
    },
    setItem: (_k: string, v: string) => {
      if (throwing) throw new Error("no storage here");
      value = v;
    },
  } as unknown as Storage;
}

describe("the cadence", () => {
  it("is one, two or three seconds, two by default, with a short dwell for an unchanged frame", () => {
    expect(CADENCES).toEqual([1000, 2000, 3000]);
    expect(DEFAULT_CADENCE).toBe(2000);
    expect(QUICK_DWELL).toBe(400);
    expect(loadCadence(fakeStorage())).toBe(2000);
  });
  it("round-trips through storage", () => {
    const s = fakeStorage();
    saveCadence(3000, s);
    expect(loadCadence(s)).toBe(3000);
    expect(s.getItem("yokohama.playback.cadence")).toBe("3000");
  });
  it("reads the default when storage throws or holds a value that is not a cadence", () => {
    expect(loadCadence(fakeStorage(undefined, true))).toBe(2000);
    expect(loadCadence(fakeStorage("1500"))).toBe(2000);
    expect(loadCadence(fakeStorage("fast"))).toBe(2000);
    expect(() => saveCadence(1000, fakeStorage(undefined, true))).not.toThrow();
  });
});
```

- [ ] **Step 2: The cadence module**

Create `web/src/playback.ts`:

```ts
// How fast the opponent's turn plays out (M5.5 spec §3.2.3), and where that
// choice lives: per BROWSER, in localStorage, the `sections.ts` idiom — a
// convenience, never state; a storage that throws reads as the default.

export const CADENCES = [1000, 2000, 3000] as const;
export type Cadence = (typeof CADENCES)[number];
export const DEFAULT_CADENCE: Cadence = 2000;
/** The dwell on a step whose frame is identical to the previous one — the
 *  bookkeeping events ("End additional-action phase", a skipped 5-Power). */
export const QUICK_DWELL = 400;

const KEY = "yokohama.playback.cadence";

function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

export function loadCadence(storage: Storage | null = browserStorage()): Cadence {
  try {
    const n = Number(storage?.getItem(KEY));
    return (CADENCES as readonly number[]).includes(n) ? (n as Cadence) : DEFAULT_CADENCE;
  } catch {
    return DEFAULT_CADENCE;
  }
}

export function saveCadence(ms: Cadence, storage: Storage | null = browserStorage()): void {
  try {
    storage?.setItem(KEY, String(ms));
  } catch {
    // A per-browser convenience; losing it loses nothing.
  }
}
```

Run: `cd web && npx vitest run src/playback.test.ts` — 3 passed.

- [ ] **Step 3: The walk's failing tests**

Create `web/src/walk.test.ts`:

```ts
// The run a reply added, and the walk over it: a pure reducer, so the timing
// in `App` is one `setTimeout` and every decision is testable without a clock.

import { describe, expect, it } from "vitest";
import { QUICK_DWELL } from "./playback";
import { buildRun, dwellFor, hasBotStep, walkReducer, type WalkState } from "./walk";
import { wire } from "./wire/contract";
import type { HistoricalFrame, LogPanel } from "./types";

const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

/** Three positions after events 10, 11 and 12 of a game: the first step moves
 *  a cube, the second changes nothing on the table. */
function fixture(): { frames: HistoricalFrame[]; log: LogPanel } {
  const f10 = clone(wire.atK);
  f10.index = 10;
  const f11 = clone(f10);
  f11.index = 11;
  f11.frame.board.tiles[2]!.assistants[1]![1] += 1;
  f11.to_move = 1;
  f11.frame.decision.current_phase = 1;
  const f12 = clone(f11);
  f12.index = 12;
  return {
    frames: [f10, f11, f12],
    log: {
      total: 12,
      entries: [
        { index: 10, text: "Red: Place 1 assistant in each of: Port A (A02)" },
        { index: 11, text: "Blue: End additional-action phase" },
      ],
    },
  };
}

describe("buildRun", () => {
  it("makes one step per event, moved by whoever was to move BEFORE it, in the phase it was in", () => {
    const { frames, log } = fixture();
    const run = buildRun(10, frames, log);
    expect(run.from).toBe(10);
    expect(run.to).toBe(12);
    expect(run.steps).toHaveLength(2);
    expect(run.steps[0]!.event).toBe(10);
    expect(run.steps[0]!.line).toMatch(/Place 1 assistant/);
    expect(run.steps[0]!.mover).toBe(frames[0]!.to_move);
    expect(run.steps[0]!.phase).toBe(
      frames[0]!.frame.decision.phase_chain[frames[0]!.frame.decision.current_phase],
    );
    expect(run.steps[0]!.after).toBe(frames[1]);
    expect(run.steps[1]!.mover).toBe(1);
    expect(run.steps[1]!.before).toBe(frames[1]);
  });

  it("knows a bot's step from the Players panel of the position before it", () => {
    const { frames, log } = fixture();
    expect(hasBotStep(buildRun(10, frames, log))).toBe(false);
    const players = frames[1]!.frame.dashboard.find((p) => "Players" in p);
    if (!players || !("Players" in players)) throw new Error("no Players panel");
    players.Players.players.find((p) => p.seat === 1)!.is_bot = true;
    const run = buildRun(10, frames, log);
    expect(run.steps[1]!.moverIsBot).toBe(true);
    expect(run.steps[0]!.moverIsBot).toBe(false);
    expect(hasBotStep(run)).toBe(true);
  });

  it("dwells the cadence on a changed frame and the short dwell on an unchanged one", () => {
    const { frames, log } = fixture();
    const run = buildRun(10, frames, log);
    expect(run.steps[0]!.changes.any).toBe(true);
    expect(run.steps[1]!.changes.any).toBe(false);
    expect(dwellFor(run.steps[0]!, 3000)).toBe(3000);
    expect(dwellFor(run.steps[1]!, 3000)).toBe(QUICK_DWELL);
  });
});

describe("walkReducer", () => {
  const last = 2; // three steps
  const s = (at: number, playing = true): WalkState => ({ at, playing });
  it("ticks forward and ends after the last step", () => {
    expect(walkReducer(s(0), { type: "tick" }, last)).toEqual(s(1));
    expect(walkReducer(s(2), { type: "tick" }, last)).toBeNull();
  });
  it("pauses, plays, steps back and forward, and stepping pauses", () => {
    expect(walkReducer(s(1), { type: "pause" }, last)).toEqual(s(1, false));
    expect(walkReducer(s(1, false), { type: "play" }, last)).toEqual(s(1, true));
    expect(walkReducer(s(1), { type: "back" }, last)).toEqual(s(0, false));
    expect(walkReducer(s(0), { type: "back" }, last)).toEqual(s(0, false));
    expect(walkReducer(s(1), { type: "forward" }, last)).toEqual(s(2, false));
    expect(walkReducer(s(2), { type: "forward" }, last)).toBeNull();
  });
  it("jumps within the run, paused, and skip ends it", () => {
    expect(walkReducer(s(0), { type: "jump", to: 7 }, last)).toEqual(s(2, false));
    expect(walkReducer(s(2), { type: "jump", to: -3 }, last)).toEqual(s(0, false));
    expect(walkReducer(s(1), { type: "skip" }, last)).toBeNull();
  });
});
```

Run: `cd web && npx vitest run src/walk.test.ts` — FAIL, module missing.

- [ ] **Step 4: The walk module**

Create `web/src/walk.ts`:

```ts
// The run a reply added, and the walk over it (M5.5 spec §3.2.2).
//
// A RUN is the events `[from, to)` a reply applied, each with the position
// before and after it and the words the log gave it. The mover and the phase
// of an event are read off the position BEFORE it — what `/at/{k}` already
// serves — so no seat is ever parsed out of "Blue:" prose. The WALK over a run
// is a pure reducer; `App` owns the one timer.

import { diffFrames, type Changes } from "./diffFrames";
import { QUICK_DWELL } from "./playback";
import type { HistoricalFrame, LogPanel, Panel, PlayerId, TurnPhase } from "./types";

export interface Step {
  /** The event's index in the record. */
  event: number;
  /** The log's line for it, worded as it read when it applied. */
  line: string;
  mover: PlayerId;
  moverIsBot: boolean;
  /** The phase the mover was in when it decided — `null` only on a payload
   *  whose chain is empty, which no served frame has. */
  phase: TurnPhase | null;
  before: HistoricalFrame;
  after: HistoricalFrame;
  changes: Changes;
}

export interface Run {
  from: number;
  to: number;
  steps: Step[];
}

/** `frames[i]` is the position after `from + i` events, for `i` in
 *  `0..=to−from`; `log` carries at least the lines for `from..to`. */
export function buildRun(from: number, frames: HistoricalFrame[], log: LogPanel): Run {
  const to = from + frames.length - 1;
  const steps: Step[] = [];
  for (let e = from; e < to; e++) {
    const before = frames[e - from]!;
    const after = frames[e - from + 1]!;
    const seats =
      before.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> => "Players" in p)
        ?.Players.players ?? [];
    const mover = before.to_move;
    const d = before.frame.decision;
    steps.push({
      event: e,
      line: log.entries.find((x) => x.index === e)?.text ?? `event ${e}`,
      mover,
      moverIsBot: seats.some((p) => p.seat === mover && p.is_bot),
      phase: d.phase_chain[d.current_phase] ?? null,
      before,
      after,
      changes: diffFrames(before, after),
    });
  }
  return { from, to, steps };
}

/** The walk plays only when the opponent moved: a run of your own events
 *  applies at once (spec §3.2.2). */
export const hasBotStep = (run: Run): boolean => run.steps.some((s) => s.moverIsBot);

/** How long a step stays on the table. */
export const dwellFor = (step: Step, cadence: number): number =>
  step.changes.any ? cadence : QUICK_DWELL;

export interface WalkState {
  /** The step on the table: an index into `run.steps`. */
  at: number;
  playing: boolean;
}

export type WalkAction =
  | { type: "tick" }
  | { type: "play" }
  | { type: "pause" }
  | { type: "back" }
  | { type: "forward" }
  | { type: "jump"; to: number }
  | { type: "skip" };

/** `last` is the index of the run's final step. `null` means the walk is over
 *  and the live state is what is on screen. */
export function walkReducer(s: WalkState, a: WalkAction, last: number): WalkState | null {
  switch (a.type) {
    case "tick":
      return s.at >= last ? null : { ...s, at: s.at + 1 };
    case "play":
      return { ...s, playing: true };
    case "pause":
      return { ...s, playing: false };
    case "back":
      return { at: Math.max(0, s.at - 1), playing: false };
    case "forward":
      return s.at >= last ? null : { at: s.at + 1, playing: false };
    case "jump":
      return { at: Math.max(0, Math.min(last, a.to)), playing: false };
    case "skip":
      return null;
  }
}
```

- [ ] **Step 5: Gate and commit**

Run: `cd web && npx vitest run src/walk.test.ts src/playback.test.ts && npm run typecheck` — 9 passed.

```bash
git add web/src/playback.ts web/src/playback.test.ts web/src/walk.ts web/src/walk.test.ts
git commit -m "feat(web): C — the cadence per browser, the run a reply added, and the walk's reducer"
```

---

### Task 8: `TurnAccount` replaces `LastMove` (spec §3.2.5)

**Files:**
- Create: `web/src/TurnAccount.tsx`, `web/src/TurnAccount.test.tsx`
- Delete: `web/src/LastMove.tsx`, `web/src/LastMove.test.tsx`
- Modify: `web/src/App.tsx` (the import and the one call site, minimally — Task 9 wires the rest), `web/src/styles.css` (the `.last-move` block at lines 1165–1176 becomes `.turn-account`)

**Interfaces:**
- Consumes: `Run` (Task 7), `Cadence`/`CADENCES` (Task 7), `seatClass` (`Cards.tsx`), `Labels.phases`.
- Produces: `TurnAccount` with the props below; `isRefusal` moves here unchanged. Task 9 supplies every callback.

```ts
export interface TurnAccountProps {
  messages: string[];
  run: Run | null;
  /** The step on the table while the walk runs; `null` when it is over or never ran. */
  at: number | null;
  playing: boolean;
  cadence: Cadence;
  labels: Labels | null;
  colors: PlayerColor[];
  onCadence: (c: Cadence) => void;
  onPlay: () => void;
  onPause: () => void;
  onBack: () => void;
  onForward: () => void;
  onJump: (step: number) => void;
  onSkip: () => void;
  onReplay: () => void;
}
```

- [ ] **Step 1: The failing tests**

Create `web/src/TurnAccount.test.tsx` — the four `LastMove` cells carried over with the new name, plus these:

```ts
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { TurnAccount, isRefusal } from "./TurnAccount";
import { buildRun } from "./walk";
import { wire } from "./wire/contract";
import type { HistoricalFrame, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];
const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

const noop = () => {};
const handlers = {
  onCadence: noop, onPlay: noop, onPause: noop, onBack: noop, onForward: noop,
  onJump: noop, onSkip: noop, onReplay: noop,
};

/** A two-step run by seat 1, whose Players row says it is a bot. */
function botRun() {
  const f0 = clone(wire.atK);
  f0.to_move = 1;
  const players = f0.frame.dashboard.find((p) => "Players" in p);
  if (!players || !("Players" in players)) throw new Error("no Players panel");
  players.Players.players.find((p) => p.seat === 1)!.is_bot = true;
  const f1 = clone(f0);
  f1.frame.board.tiles[0]!.assistants[1]![1] += 2;
  const f2 = clone(f1);
  f2.frame.decision.current_phase += 1;
  return buildRun(20, [f0, f1, f2], {
    total: 22,
    entries: [
      { index: 20, text: "Blue: Place 2 assistants in Tea Plantation (A00)" },
      { index: 21, text: "Blue: Return President to hand (recover no assistants)" },
    ],
  });
}

function show(over: Partial<Parameters<typeof TurnAccount>[0]>) {
  return render(
    <TurnAccount
      messages={[]}
      run={null}
      at={null}
      playing={false}
      cadence={2000}
      labels={wire.labels}
      colors={COLORS}
      {...handlers}
      {...over}
    />,
  );
}

describe("the turn account with a run", () => {
  it("is headed by the mover's turn, one row per event with its phase and its line", () => {
    const run = botRun();
    show({ run });
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent(/Blue's turn \d+/);
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    expect(rows[0]).toHaveTextContent(/Place 2 assistants in Tea Plantation/);
    const phaseKey = run.steps[0]!.phase!;
    expect(rows[0]).toHaveTextContent(wire.labels.phases[phaseKey] ?? phaseKey);
    expect(rows[0]!.querySelector(".mover")?.className).toMatch(/seat-blue/);
  });

  it("lights the step on the table and offers the walk's controls while it runs", async () => {
    const run = botRun();
    const onPause = vi.fn();
    const onSkip = vi.fn();
    const onForward = vi.fn();
    show({ run, at: 1, playing: true, onPause, onSkip, onForward });
    const rows = screen.getAllByRole("listitem");
    expect(rows[1]).toHaveClass("at");
    expect(rows[0]).not.toHaveClass("at");
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Pause" }));
    await user.click(screen.getByRole("button", { name: "Step forward" }));
    await user.click(screen.getByRole("button", { name: "skip" }));
    expect(onPause).toHaveBeenCalledTimes(1);
    expect(onForward).toHaveBeenCalledTimes(1);
    expect(onSkip).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("button", { name: "replay" })).toBeNull();
  });

  it("offers replay once the walk is over, and a row jumps to its step", async () => {
    const run = botRun();
    const onReplay = vi.fn();
    const onJump = vi.fn();
    show({ run, at: null, onReplay, onJump });
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "replay" }));
    expect(onReplay).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole("button", { name: /Return President to hand/ }));
    expect(onJump).toHaveBeenCalledWith(1);
    expect(screen.queryByRole("button", { name: "skip" })).toBeNull();
  });

  it("remembers the cadence the player picks", async () => {
    const onCadence = vi.fn();
    show({ onCadence });
    await userEvent.setup().selectOptions(screen.getByLabelText("Cadence"), "3000");
    expect(onCadence).toHaveBeenCalledWith(3000);
  });
});

describe("the turn account without a run", () => {
  it("holds every prose line the server sent, refusals as warnings", () => {
    const state = wire.refused;
    const refusals = state.messages.filter(isRefusal);
    expect(refusals.length, "the fixture must carry a refusal").toBeGreaterThan(0);
    show({ messages: state.messages });
    for (const m of state.messages) expect(screen.getByText(new RegExp(escape(m)))).toBeInTheDocument();
    for (const m of refusals) expect(screen.getByText(new RegExp(escape(m)))).toHaveClass("warn");
    expect(screen.getByRole("heading", { level: 2 })).toHaveTextContent("Last move");
  });

  it("warns on a bang-prefixed refusal the prose regex cannot see", () => {
    const bang = "! A05 is not one of the eligible areas";
    expect(/illegal|refus|cannot|not legal|out of range/i.test(bang)).toBe(false);
    show({ messages: [bang, "Turn 1 Red -> Turn 2 Blue"] });
    expect(screen.getByText(new RegExp(escape(bang)))).toHaveClass("warn");
    expect(screen.getByText(/Turn 1 Red/)).not.toHaveClass("warn");
  });

  it("keeps its live region mounted while empty, and folds without unmounting it", async () => {
    const { container } = show({});
    const region = container.querySelector('[role="status"]');
    expect(region).not.toBeNull();
    expect(region!.textContent).toBe("");
    await userEvent.setup().click(screen.getByRole("button", { name: "hide" }));
    expect(container.querySelector(".messages.folded")).not.toBeNull();
    expect(container.querySelector('[role="status"]')).not.toBeNull();
  });
});
```

Run: `cd web && npx vitest run src/TurnAccount.test.tsx` — FAIL, module missing.

- [ ] **Step 2: The component**

Create `web/src/TurnAccount.tsx`:

```tsx
// The account of the last turn (M5.5 spec §3.2.5), in the side column where
// `LastMove` was: one row per event of the run the reply added, the step on
// the table lit while the walk runs, and the walk's controls in the header.
// With no run — an undo, a picker step, a refusal, a cold load — the reply's
// prose lines, exactly as `LastMove` showed them.
//
// The `role="status"` container stays mounted and empty rather than
// unmounted, because a live region inserted with its text is unreliably
// announced; folding hides it with `max-height: 0`, which keeps it in the tree.

import { useState } from "react";
import { seatClass } from "./Cards";
import { CADENCES, type Cadence } from "./playback";
import type { Labels, PlayerColor } from "./types";
import type { Run } from "./walk";

/** Does this line REFUSE something the player asked for, or merely report it?
 *  A HEURISTIC over prose, carried from `LastMove` unchanged; its two
 *  corrections and its stated gap are in that file's history. */
export function isRefusal(message: string): boolean {
  return (
    message.trimStart().startsWith("!") ||
    /illegal|refus|cannot|not legal|out of range/i.test(message)
  );
}

export function TurnAccount({
  messages, run, at, playing, cadence, labels, colors,
  onCadence, onPlay, onPause, onBack, onForward, onJump, onSkip, onReplay,
}: {
  messages: string[];
  run: Run | null;
  at: number | null;
  playing: boolean;
  cadence: Cadence;
  labels: Labels | null;
  colors: PlayerColor[];
  onCadence: (c: Cadence) => void;
  onPlay: () => void;
  onPause: () => void;
  onBack: () => void;
  onForward: () => void;
  onJump: (step: number) => void;
  onSkip: () => void;
  onReplay: () => void;
}) {
  const [folded, setFolded] = useState(false);
  const last = run?.steps[run.steps.length - 1];
  const walking = run !== null && at !== null;
  const seat = (s: number) => colors[s] ?? `Seat ${s}`;
  return (
    <section className="panel turn-account">
      <h2>
        {last ? (
          <span className={seatClass(colors[last.mover])}>
            {seat(last.mover)}&rsquo;s turn {last.before.frame.decision.turn_number}
          </span>
        ) : (
          "Last move"
        )}
        <select
          aria-label="Cadence"
          value={cadence}
          onChange={(e) => onCadence(Number(e.currentTarget.value) as Cadence)}
        >
          {CADENCES.map((c) => (
            <option key={c} value={c}>
              {c / 1000} s
            </option>
          ))}
        </select>
        {walking && (
          <>
            <button type="button" className="ghost" aria-label="Step back" onClick={onBack} disabled={at <= 0}>
              &#9664;
            </button>
            {playing ? (
              <button type="button" className="ghost" aria-label="Pause" onClick={onPause}>pause</button>
            ) : (
              <button type="button" className="ghost" aria-label="Play" onClick={onPlay}>play</button>
            )}
            <button type="button" className="ghost" aria-label="Step forward" onClick={onForward}>
              &#9654;
            </button>
            <button type="button" className="ghost" onClick={onSkip}>skip</button>
          </>
        )}
        {run && !walking && (
          <button type="button" className="ghost" onClick={onReplay}>replay</button>
        )}
        <button
          type="button"
          className="ghost fold"
          aria-expanded={!folded}
          aria-controls="turn-account-body"
          onClick={() => setFolded((f) => !f)}
        >
          {folded ? "show" : "hide"}
        </button>
      </h2>
      <div id="turn-account-body" className={`messages${folded ? " folded" : ""}`} role="status">
        {run ? (
          <ol className="account">
            {run.steps.map((s, i) => (
              <li key={s.event} className={i === at ? "at" : ""}>
                <span className="chip phase">{(s.phase && labels?.phases[s.phase]) ?? s.phase ?? ""}</span>
                <span className={`mover ${seatClass(colors[s.mover])}`}>{seat(s.mover)}</span>
                <button type="button" className="log-jump" onClick={() => onJump(i)}>
                  {s.line}
                </button>
              </li>
            ))}
          </ol>
        ) : (
          messages.map((m, i) => (
            <div key={i} className={isRefusal(m) ? "warn" : ""}>
              {m}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
```

- [ ] **Step 3: Swap it in, minimally, and the styles**

In `web/src/App.tsx`: replace `import { LastMove } from "./LastMove";` with `import { TurnAccount } from "./TurnAccount";` and `import { DEFAULT_CADENCE } from "./playback";`, and replace `<LastMove messages={state.messages} />` with

```tsx
        <TurnAccount
          messages={state.messages}
          run={null}
          at={null}
          playing={false}
          cadence={DEFAULT_CADENCE}
          labels={labels}
          colors={colors}
          onCadence={() => {}}
          onPlay={() => {}}
          onPause={() => {}}
          onBack={() => {}}
          onForward={() => {}}
          onJump={() => {}}
          onSkip={() => {}}
          onReplay={() => {}}
        />
```

(Task 9 replaces every no-op.) Delete `web/src/LastMove.tsx` and `web/src/LastMove.test.tsx`. In `web/src/styles.css` rename the block at 1165–1176 from `.last-move` to `.turn-account`, drop the `max-height: 9rem` on `.messages` (the account takes the height it needs; the side column scrolls), keep the folded rule, and add:

```css
.turn-account h2 { display: flex; align-items: center; gap: 0.35rem; flex-wrap: wrap; }
.turn-account h2 select { margin-left: auto; font: inherit; font-size: 10px; background: var(--bg-sunken); color: var(--fg); border: 1px solid var(--line); border-radius: 3px; }
.turn-account h2 .ghost { font-size: 10px; padding: 0 0.4rem; letter-spacing: 0; text-transform: none; }
ol.account { list-style: none; margin: 0.3rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.15rem; }
ol.account li { display: grid; grid-template-columns: auto auto 1fr; align-items: baseline; gap: 0.4rem; font-size: 11.5px; padding: 0.15rem 0.2rem; border-top: 1px solid var(--line); }
ol.account li.at { background: rgb(105 194 95 / 0.12); }
ol.account .mover { font-weight: 700; }
```

Also search `web/e2e/` and `web/src/` for `last-move` (the M3.5 pinned-deck cell's `mostCommentary` probe reads `.last-move .messages`, per the M5 ledger) and rename every selector to `.turn-account .messages`.

- [ ] **Step 4: Gate and commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e -- the_screen_itself.spec.ts` — green (the renamed probe included).

```bash
git add web/src/TurnAccount.tsx web/src/TurnAccount.test.tsx web/src/App.tsx web/src/styles.css web/e2e
git rm -q web/src/LastMove.tsx web/src/LastMove.test.tsx
git commit -m "feat(web): C — the turn account replaces the last-move box: rows, a lit step, the walk's controls"
```

---

### Task 9: The walk on the table (spec §3.2.2–§3.2.5, §4.2–§4.4)

**Files:**
- Modify: `web/src/App.tsx` (the run fetch after a reply, the walk state and its one timer, the deck strip, the view during the walk, the highlights, the account's callbacks), `web/src/TableSections.tsx` (strips from a frame that carries offers; `game` nullable), `web/src/Board.tsx` (`changed`, `changedBy` props → `.tile.changed`), `web/src/Dashboard.tsx` (`changedSeats` → `.seat.changed`), `web/src/Strips.tsx` (`changedSlots` → `.strip-slot.changed`), `web/src/styles.css` (the highlight rules, `.decision.walking`), `web/src/test/harness.tsx` (`/at/` and `/log` in `bootstrapReply`), `web/src/Help.tsx` (two paragraphs), `web/e2e/harness.ts` (`readScreen` skips a walk), `web/e2e/gates.spec.ts` (gate C)
- Create: `web/e2e/the_playback.spec.ts`
- Test: `web/src/App.test.tsx` (three cells), `web/src/TableSections.test.tsx` (one cell replaced, one added), `web/src/Board.test.tsx` (one cell)

**Interfaces:**
- Consumes: everything Tasks 5–8 produce.
- Produces: `Board` props `changed?: Set<number>`, `changedBy?: PlayerColor`; `Dashboard` prop `changedSeats?: Set<number>`; `PortStrip`/`LabStrip` prop `changed?: Set<string>`; `TableSections` props `game: string | null`, `state: ClientState | null`; the deck strip `.decision.walking` with a `skip` button; `readScreen` skipping a walk. Task 12 reuses the `TableSections` nullable props.

- [ ] **Step 1: The failing App cells**

Add to `web/src/App.test.tsx` (imports: `vi` from vitest, `buildRun` is not needed — the cells drive the app):

```ts
describe("the turn playback", () => {
  /** A reply that applied THREE events after the state the app answered from,
   *  the second of them by a bot: the fixtures for the run's positions are
   *  the corpus's `at_k` with one seat marked as a bot and a cube moved. */
  function botReply() {
    const before = { ...wire.optionsMany, events: 40 };
    const replied = { ...wire.optionsMany, events: 43 };
    const at = (k: number) => {
      const f = JSON.parse(JSON.stringify(wire.atK)) as typeof wire.atK;
      f.index = k;
      f.total = 43;
      f.to_move = k === 40 ? before.to_move : 1;
      const players = f.frame.dashboard.find((p) => "Players" in p);
      if (players && "Players" in players) players.Players.players[1]!.is_bot = true;
      if (k >= 42) f.frame.board.tiles[0]!.assistants[1]![1] += 1;
      return f;
    };
    const log = {
      total: 43,
      entries: [
        { index: 40, text: "Red: End additional-action phase" },
        { index: 41, text: "Blue: End additional-action phase" },
        { index: 42, text: "Blue: Place 1 assistant in each of: Tea Plantation (A00)" },
      ],
    };
    return { before, replied, at, log };
  }

  it("fetches the run's positions, walks it when a bot moved, and skip lands live", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { before, replied, at, log } = botReply();
      const stub = await renderApp(before, (call) => {
        if (call.path.endsWith("/input")) return replied;
        const m = /\/at\/(\d+)$/.exec(call.path);
        if (m) return at(Number(m[1]));
        if (call.path.includes("/log")) return log;
        return undefined;
      });
      const state = wire.optionsMany;
      if (state.prompt.kind !== "options") throw new Error("fixture moved");
      await userEvent.setup({ advanceTimers: vi.advanceTimersByTime }).click(
        screen.getByRole("button", {
          name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
        }),
      );
      // Positions 40..=43, in ascending order, and the log's tail of three.
      await waitFor(() => expect(stub.matching("GET", "/at/")).toHaveLength(4));
      expect(stub.matching("GET", "/at/").map((c) => Number(/\/at\/(\d+)$/.exec(c.path)![1]))).toEqual([40, 41, 42, 43]);
      expect(stub.matching("GET", "/log")[0]!.path).toMatch(/tail=3$/);
      // The walk is on: the deck says who is playing, the table is outlined,
      // the account lights step 1 of 3.
      await waitFor(() => expect(screen.getByText(/Blue is playing/)).toBeInTheDocument());
      expect(document.querySelector(".table.replaying")).not.toBeNull();
      expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
      expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Red: End additional-action/);
      // An unchanged frame dwells the short dwell, a changed one the cadence.
      await act(async () => { await vi.advanceTimersByTimeAsync(450); });
      expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Blue: End additional-action/);
      await act(async () => { await vi.advanceTimersByTimeAsync(450); });
      expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Place 1 assistant/);
      expect(document.querySelector(".tile.changed")).not.toBeNull();
      // Skip: live prompt back, no outline, the account keeps the run and offers replay.
      await userEvent.setup({ advanceTimers: vi.advanceTimersByTime }).click(screen.getByRole("button", { name: "skip" }));
      expect(screen.queryByText(/is playing/)).toBeNull();
      expect(document.querySelector(".table.replaying")).toBeNull();
      expect(screen.getByRole("button", { name: "replay" })).toBeInTheDocument();
      expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
    } finally {
      vi.useRealTimers();
    }
  });

  it("does not walk a run of the player's own events, and shows the prose when nothing was applied", async () => {
    const { before, replied, at, log } = botReply();
    const ownOnly = (k: number) => {
      const f = at(k);
      f.to_move = before.to_move;
      const players = f.frame.dashboard.find((p) => "Players" in p);
      if (players && "Players" in players) for (const p of players.Players.players) p.is_bot = false;
      return f;
    };
    let reply: unknown = replied;
    const stub = await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return reply;
      const m = /\/at\/(\d+)$/.exec(call.path);
      if (m) return ownOnly(Number(m[1]));
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    const row = () => screen.getByRole("button", {
      name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
    });
    const user = userEvent.setup();
    await user.click(row());
    await waitFor(() => expect(stub.matching("GET", "/at/")).toHaveLength(4));
    expect(screen.queryByText(/is playing/)).toBeNull();
    expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
    // A reply with the SAME count (a picker step) shows its prose instead.
    reply = { ...replied, messages: ["Selected: A01", "-> legal — Place 1 assistant in each of: Church (A01)"] };
    await user.click(row());
    await waitFor(() => expect(screen.getByText(/Selected: A01/)).toBeInTheDocument());
    expect(document.querySelector("ol.account")).toBeNull();
  });

  it("keeps the cadence the player chose, through storage", async () => {
    await renderApp(wire.optionsMany);
    await userEvent.setup().selectOptions(screen.getByLabelText("Cadence"), "1000");
    expect(localStorage.getItem("yokohama.playback.cadence")).toBe("1000");
  });
});
```

Run: `cd web && npx vitest run src/App.test.tsx -t "turn playback"` — FAIL on every cell.

- [ ] **Step 2: The harness fallbacks**

In `web/src/test/harness.tsx`'s `bootstrapReply`, add before `return undefined`:

```ts
  // The walk's two reads (M5.5): a position and the log's tail. Answered from
  // the corpus so a cell that is not about the walk does not have to be — a
  // corpus reply marks no seat as a bot, so no walk starts from it.
  if (/\/at\/\d+$/.test(call.path)) return wire.atK;
  if (/\/log(\?|$)/.test(call.path)) return wire.log;
```

- [ ] **Step 3: App — the run, the walk, the view**

In `web/src/App.tsx`:

```tsx
// imports
import { DEFAULT_CADENCE, loadCadence, saveCadence, type Cadence } from "./playback";
import { buildRun, dwellFor, hasBotStep, walkReducer, type Run, type WalkAction, type WalkState } from "./walk";

// state, beside `scrubK`/`past`:
  // **The last run a reply added, and the walk over it (M5.5 spec §3.2).**
  // `run` outlives the walk: the account keeps showing it until the next run
  // replaces it. `walk` is the step on the table and whether the timer runs;
  // `null` is "over" — the live state is on screen. `fresh` is true from the
  // moment a run lands until the player's next answer, and is what keeps the
  // last step's highlights on the live table (spec §3.2.4).
  const [run, setRun] = useState<Run | null>(null);
  const [walk, setWalk] = useState<WalkState | null>(null);
  const [fresh, setFresh] = useState(false);
  const [cadence, setCadence] = useState<Cadence>(() => loadCadence());
  const dispatch = useCallback(
    (a: WalkAction) => setWalk((w) => (w && run ? walkReducer(w, a, run.steps.length - 1) : w)),
    [run],
  );
  // ONE timer. `dwellFor` is the whole of the timing rule.
  useEffect(() => {
    if (!run || !walk || !walk.playing) return;
    const step = run.steps[walk.at];
    if (!step) return;
    const t = setTimeout(() => dispatch({ type: "tick" }), dwellFor(step, cadence));
    return () => clearTimeout(t);
  }, [run, walk, cadence, dispatch]);

  /** The positions `prev..=next` and the log's tail, in parallel; `prev` is
   *  fetched too, because the live payload carries no port or lab and the
   *  first step's diff needs a full position on both sides. */
  const fetchRun = async (game: string, prev: number, next: number): Promise<Run> => {
    const ks = Array.from({ length: next - prev + 1 }, (_, i) => prev + i);
    const [frames, log] = await Promise.all([Promise.all(ks.map((k) => api.at(game, k))), api.log(game, next - prev)]);
    return buildRun(prev, frames, log);
  };
  /** A reply landed: if it added events, build the run and start the walk
   *  when a bot moved. Called INSIDE the busy window, so the deck stays
   *  disabled until the walk is ready to show. */
  const landed = async (game: string, prev: number, replied: ClientState) => {
    setFresh(false);
    if (replied.events <= prev) return;
    const r = await fetchRun(game, prev, replied.events);
    if (gameId.current !== game) return;
    setRun(r);
    setFresh(true);
    setWalk(hasBotStep(r) ? { at: 0, playing: true } : null);
  };
```

In `send`, after `if (replied.id === gameId.current) setState(replied);` add `if (replied.id === gameId.current) await landed(game, state.events, replied);` — inside the `try`, so a failed fetch of the run reports as the error strip does today and the live state stays on screen (the reply was already applied).

`show()` gains a second parameter, `opening = false`; when `opening && s.events > 0` it calls `void landed(s.id, 0, s)` after `setState(s)` — the bot's opening run on a game whose start seat is a bot. `NewGame`'s `onStarted` passes `true` for a created game and `false` for a saved one (change its two call sites: `onStarted(await api.create(req), true)` and `onStarted(await api.get(name), false)`; the prop type becomes `(state: ClientState, opening: boolean) => void`). The Game menu's switch calls `show(s)` and clears `run`, `walk`, `fresh`.

The view:

```tsx
  const stepNow = run && walk ? run.steps[walk.at] : undefined;
  const walking = stepNow !== undefined;
  const atPresent = past !== null && past.index >= past.total;
  const replaying = walking ? stepNow.after.index < stepNow.after.total : past !== null && !atPresent;
  const view = walking ? stepNow.after : replaying ? past! : state;
  const highlights = walking ? stepNow.changes : fresh && run ? run.steps[run.steps.length - 1]!.changes : null;
  const highlightBy = walking ? colors[stepNow.mover] : fresh && run ? colors[run.steps[run.steps.length - 1]!.mover] : undefined;
```

`Board` receives `changed={highlights?.tiles} changedBy={highlightBy}` and `onTileClick` only when `picking && !replaying && !walking`; `Dashboard` receives `changedSeats={highlights?.seats}`; `TableSections` receives `changedSlots={highlights?.slots}` and the nullable `game`/`state` as today's values. The deck:

```tsx
        {walking ? (
          <div className="decision walking" role="status">
            <span className={seatClass(colors[stepNow.mover])}>{colors[stepNow.mover] ?? `Seat ${stepNow.mover}`}</span>
            {" is playing · step "}{walk!.at + 1}{" of "}{run!.steps.length}
            <button type="button" className="ghost" onClick={() => dispatch({ type: "skip" })}>skip</button>
          </div>
        ) : (
          <Decision … />
        )}
```

The account: `run={run} at={walk?.at ?? null} playing={walk?.playing ?? false} cadence={cadence}`, `onCadence={(c) => { saveCadence(c); setCadence(c); }}`, `onPlay/onPause/onBack/onForward/onSkip` dispatching their action, `onJump={(i) => setWalk({ at: i, playing: false })}`, `onReplay={() => setWalk({ at: 0, playing: true })}`. `messages={state.messages}`. The `replaying` class on `main.table` uses the new `replaying`.

- [ ] **Step 4: TableSections from a frame, and the highlights**

`web/src/TableSections.tsx`: `game: string | null`, `state: ClientState | null`, new `changedSlots?: Set<string>`. The effect returns early when `game === null` as well as when `replaying`. Rendering: `const offers = "ports" in view ? view : null;` then for `ports`: `offers ? <PortStrip panel={offers.ports} art={art} changed={changedSlots} /> : replaying ? collapsed("ports") : ports ? <PortStrip panel={ports} … /> : loading("ports")`; the same for `labs`; `buildings` stays collapsed while replaying and reads "available once the game starts" when `game === null`. Rewrite the header comment: the port and lab draw from a position that carries them; the building sites still collapse.

`web/src/Strips.tsx`: `PortStrip`/`LabStrip` take `changed?: Set<string>` and add ` changed` to a `.strip-slot`'s class when `changed?.has(\`port:${i}:${s.slot}\`)` (board index `i` from `panel.boards.map((b, i) => …)`), `lab:` likewise.

`web/src/Board.tsx`: `Board` takes `changed?: Set<number>` and `changedBy?: PlayerColor`, passes `isChanged={changed?.has(t.area) ?? false}` and `changedBy` to `Tile`, whose `className` gains `changed ${seatClass(changedBy)}` when `isChanged`. `web/src/Dashboard.tsx`: `Dashboard` and `Players` take `changedSeats?: Set<number>`; the seat row's class gains ` changed` when the set has `p.seat`.

`web/src/styles.css`:

```css
/* ---- M5.5: the walk's highlights --------------------------------------- */
/* The mover's seat colour, via `.seat-*`'s `color`, so one rule serves four
 * seats and a fifth colour is still one line elsewhere. */
.tile.changed { outline: 3px solid currentColor; outline-offset: -3px; }
.seat.changed { background: rgb(105 194 95 / 0.10); }
.strip-slot.changed .card-face { outline: 3px solid var(--vp); outline-offset: 2px; }
.decision.walking { display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 0.8rem; font-family: var(--display); font-size: 1.05rem; }
.decision.walking .ghost { margin-left: auto; }
```

Cells: in `web/src/TableSections.test.tsx` replace `"collapse the fetched sections while replaying and fetch nothing"` with two — the port and lab draw from a `HistoricalFrame` view (`wire.atK`) with NO fetch (`stub.calls` length 0) and the building sites collapse; a `null` game fetches nothing and says "available once the game starts". In `web/src/Board.test.tsx` add: a tile in `changed` carries `changed` and the seat class. Run: `cd web && npm test` — green, including Step 1's three cells.

- [ ] **Step 5: The browser harness skips a walk; the playback cell**

In `web/e2e/harness.ts`'s `readScreen`, after the `waitForFunction`:

```ts
  // **A running walk is skipped here, once, before the screen is read.** The
  // turn playback (M5.5) replaces the deck with a strip while the opponent's
  // turn plays out, and a policy reading that strip would report "nothing to
  // click" about a game that is fine. The one cell that OBSERVES a walk
  // (`the_playback.spec.ts`) does its watching before it calls this.
  const skip = page.locator(".decision.walking button", { hasText: "skip" });
  if (await skip.count()) {
    await skip.first().click();
    await page.locator(".decision.walking").waitFor({ state: "detached", timeout: 15_000 });
  }
```

Create `web/e2e/the_playback.spec.ts`:

```ts
// M5.5 C: the opponent's turn plays out on the table when the reply lands.

import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

test("a bot turn plays out on the table, every position in order, and ends on the live prompt", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  await page.getByLabel("Cadence").selectOption("1000");
  const positions: number[] = [];
  page.on("request", (r) => {
    const m = /\/at\/(\d+)$/.exec(new URL(r.url()).pathname);
    if (m) positions.push(Number(m[1]));
  });
  const pick = rng(0x504c_4159); // "PLAY"
  let observed = false;
  for (let i = 0; i < 40 && !observed; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    positions.length = 0;
    if ((await act(page, screen, pick)) === null) throw new Error("dead end before a bot turn was seen");
    await page
      .locator(".decision.walking, .decision button.option:not([disabled]), .decision button.primary:not([disabled])")
      .first()
      .waitFor({ timeout: 15_000 });
    if ((await page.locator(".decision.walking").count()) === 0) continue;
    observed = true;
    const strip = page.locator(".decision.walking");
    await expect(strip).toContainText(/is playing · step \d+ of \d+/);
    const steps = Number(/of (\d+)/.exec(await strip.innerText())![1]);
    // One position per event PLUS the one before the run, requested in
    // ascending order and contiguous.
    const sorted = [...positions].sort((a, b) => a - b);
    expect(positions).toEqual(sorted);
    expect(sorted).toHaveLength(steps + 1);
    for (let j = 1; j < sorted.length; j++) expect(sorted[j]).toBe(sorted[j - 1]! + 1);
    await expect(page.locator("ol.account li.at")).toHaveCount(1);
    if (steps >= 2) await expect(page.locator(".table.replaying")).toBeVisible();
    // Let it run out at 1 s a step, with slack; the walk ends by itself.
    await expect(strip).toHaveCount(0, { timeout: steps * 1000 + 8000 });
    await expect(page.locator(".table.replaying")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "replay" })).toBeVisible();
    const live = await readScreen(page);
    expect(live.options + live.tiles + Number(live.only) + Number(live.confirm)).toBeGreaterThan(0);
  }
  expect(observed, "no bot turn was observed in 40 actions").toBe(true);
  expect(watch.failures).toEqual([]);
});
```

Run: `cd web && npm run e2e` — expected: every spec green (the harness skip keeps the whole-game walk and the table cells fast) plus this cell; the run prints how many steps the observed turn had.

- [ ] **Step 6: The help, and gate C**

`web/src/Help.tsx`: in "The three parts of the screen" replace *"then what the last move did"* with *"then the account of the last turn — which, when the opponent has just moved, first plays out on the table one step at a time, at the pace the account's cadence control sets, with the pieces and offers that moved outlined; skip lands you on your prompt, replay runs it again"*. In "Replaying the game" replace the two sentences about hidden offers with *"The port and the laboratory draw from that same past position; the building sites say they are hidden while you are in the past, since their board is read from the live game."*

In `web/e2e/gates.spec.ts` add gate C: start a 2P game at 1710×869, set the cadence to 1 s, act until `.decision.walking` appears, press "Pause", then screenshot `c-step-1.png`, press "Step forward" and screenshot up to `c-step-6.png`, each with the account in frame; one more shot after "skip" as `c-live.png`. Run it with `YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts -g "gate C"`.

- [ ] **Step 7: Gate and commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e` — green; counts recorded.

```bash
git add web/src web/e2e
git commit -m "feat(web): C — the opponent's turn plays out on the table, the strips at every past position, the harness skips a walk"
```

Report the counts, the observed turn's step count, and the gate C files; the orchestrator sends the sheet to the owner (no stop).

---

*(There is no Task 10: the help paragraphs and gate C were folded into Task 9, where the code they describe lands.)*

### Task 11: The preview as a position, and the bot keeps its own card (spec §3.3.1, §3.3.2, §4.1)

**Files:**
- Modify: `crates/yokohama-web/src/games.rs` (`NewGame.keeps`, `Deal`, `DealCard`, `Preview`, `config_of` → `config_of` + `choices_of`, `bot_keep`, `preview`, `create`, tests), `crates/yokohama-web/src/lib.rs` (two route cells), `crates/yokohama-web/tests/wire_fixtures.rs` (the `preview_bot` capture; the seed exemption's `table` assertion), `web/src/wire/*.json` (regenerated), `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/types.ts`, `web/src/api.ts`

**Interfaces:**
- Consumes: `HistoricalFrame::at` (Task 5's widened frame), `yokohama_core::score_starting_order` (pub via `game::greedy`, re-exported at the crate root through `pub use game::*`; if the path does not resolve, use `yokohama_core::game::greedy::score_starting_order`).
- Produces (Task 12 consumes): wire `Preview { seed: string; table: HistoricalFrame; deals: Deal[] }`, `Deal { seat; is_bot; cards: DealCard[]; bot_keep: 1 | 2 | null }`, `DealCard { card: OrderCard; keep_score: number }`; `NewGameRequest.keeps: (number | null)[]`; the fixture `web/src/wire/preview_bot.json` and `wire.previewBot`.

- [ ] **Step 1: The failing unit cells (`games.rs`)**

In `mod tests`, add a request builder and four cells:

```rust
    fn req_with_bot(name: &str) -> NewGame {
        NewGame {
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seed: Some(WireSeed(20_260_831)),
            keeps: vec![Some(1), None],
            bot_seats: vec![1],
            name: Some(name.to_string()),
        }
    }

    /// The bot keeps the higher-scored of its two cards — the argmax every
    /// evaluator contestant applies in the arena (`greedy.rs:733`), reached
    /// through core. The human's keep is honoured beside it, and the cell is
    /// mutation-proven by the human's choice: swapping 1 for 2 moves the
    /// human's card and not the bot's.
    #[test]
    fn the_bot_keeps_the_higher_scored_card_and_the_human_keeps_theirs() {
        let rules = Rules::standard();
        let pending = new_game(GameConfig::for_player_count(2), &rules, 20_260_831).unwrap();
        let deal = pending.deals[1];
        let score = |id| yokohama_core::score_starting_order(rules.orders.get(id).unwrap(), &rules);
        assert_ne!(score(deal[0]), score(deal[1]), "seed 20260831 deals seat 1 two cards of equal score; pick another seed for this cell");
        let expected = if score(deal[1]) > score(deal[0]) { deal[1] } else { deal[0] };

        let g = games();
        let (_, handle) = g.create(&req_with_bot("keep")).unwrap();
        let r = handle.lock().unwrap();
        assert_eq!(r.session.state().players[1].order_cards_in_hand, vec![expected]);
        assert_eq!(r.session.state().players[0].order_cards_in_hand, vec![pending.deals[0][0]]);
        drop(r);

        let mut two = req_with_bot("keep2");
        two.keeps = vec![Some(2), None];
        let (_, handle) = g.create(&two).unwrap();
        let r = handle.lock().unwrap();
        assert_eq!(r.session.state().players[0].order_cards_in_hand, vec![pending.deals[0][1]]);
        assert_eq!(r.session.state().players[1].order_cards_in_hand, vec![expected], "the bot's card does not follow the human's choice");
    }

    #[test]
    fn a_keep_on_a_bot_seat_and_a_missing_keep_on_a_human_seat_are_refused_by_name() {
        let g = games();
        let mut on_bot = req_with_bot("b");
        on_bot.keeps = vec![Some(1), Some(1)];
        let e = g.create(&on_bot).unwrap_err();
        assert!(e.contains("seat 1") && e.contains("bot"), "{e}");
        let mut on_human = req_with_bot("h");
        on_human.keeps = vec![None, None];
        let e = g.create(&on_human).unwrap_err();
        assert!(e.contains("seat 0"), "{e}");
        let mut bad = req_with_bot("n");
        bad.keeps = vec![Some(3), None];
        assert!(g.create(&bad).unwrap_err().contains("1 and 2"));
        let mut short = req_with_bot("s");
        short.keeps = vec![Some(1)];
        assert!(g.create(&short).unwrap_err().contains("2 seats"));
    }

    /// The preview is the position after setup — a throwaway, never a game —
    /// with every human hand empty (nothing has been chosen) and the bot's
    /// kept card in the bot's hand; each dealt card carries its Keep score.
    #[test]
    fn the_preview_is_the_position_after_setup() {
        let rules = Rules::standard();
        let p = preview(&req_with_bot("p"), &rules).unwrap();
        assert_eq!(p.table.index, 0);
        assert_eq!(p.table.total, 0);
        assert!(p.table.ports.boards[0].slots.iter().all(|s| s.card.is_some()), "the port is full");
        let players = p.table.frame.dashboard.iter().find_map(|panel| match panel {
            yokohama_cli::screen_model::Panel::Players(pp) => Some(pp),
            _ => None,
        }).expect("a Players panel");
        assert!(players.players[1].is_bot);
        assert_eq!(players.players[1].orders_in_hand, 1, "the bot's kept card is in its hand");
        assert_eq!(players.players[0].orders_in_hand, 0, "nothing chosen yet");
        assert_eq!(p.deals.len(), 2);
        assert!(!p.deals[0].is_bot && p.deals[0].bot_keep.is_none());
        assert!(p.deals[1].is_bot);
        let scores: Vec<i64> = p.deals[1].cards.iter().map(|c| c.keep_score).collect();
        let argmax = if scores[1] > scores[0] { 2 } else { 1 };
        assert_eq!(p.deals[1].bot_keep, Some(argmax));
        for d in &p.deals {
            for c in &d.cards {
                assert_eq!(c.keep_score, yokohama_core::score_starting_order(&c.card, &rules));
            }
        }
        // Nothing was persisted or made resident.
        let g = games();
        assert!(g.list().unwrap().is_empty());
    }

    /// The preview and the created game put the same card in the bot's hand.
    #[test]
    fn the_preview_and_the_game_agree_on_the_bots_card() {
        let rules = Rules::standard();
        let p = preview(&req_with_bot("agree"), &rules).unwrap();
        let kept = p.deals[1].cards[usize::from(p.deals[1].bot_keep.unwrap() - 1)].card.id;
        let g = games();
        let (_, handle) = g.create(&req_with_bot("agree")).unwrap();
        assert_eq!(handle.lock().unwrap().session.state().players[1].order_cards_in_hand, vec![kept]);
    }
```

The `Panel` enum's variant names come from `yokohama_cli::screen_model` — check them (`grep -n 'pub enum Panel' -A6 crates/yokohama-cli/src/screen_model.rs`) and adapt the match; `orders_in_hand` is the `PlayerModel` field the client reads.

Run: `cargo test -p yokohama-web games::tests` — FAIL to compile (`keeps` is `Vec<u8>`; `preview` returns no `table`).

- [ ] **Step 2: The server**

In `crates/yokohama-web/src/games.rs`:

```rust
use crate::client::HistoricalFrame;
use yokohama_core::{score_starting_order, OrderCardId};

// NewGame.keeps:
    /// One entry per seat: `1` or `2` for a human seat, as the cards are
    /// numbered on screen; `null` for a bot seat, whose keep is the bot's
    /// (M5.5 spec §3.3.2). A value on a bot seat, or `null` on a human one,
    /// is refused by name — the client never chooses for the bot again.
    pub keeps: Vec<Option<u8>>,

/// One dealt card and what the built-in bot would score it at.
#[derive(Debug, Clone, Serialize)]
pub struct DealCard {
    pub card: OrderCard,
    /// `score_starting_order` — the printed economics, hand-set and
    /// unmeasured (its own doc); shown beside the card under the CLI's hedge.
    pub keep_score: i64,
}

/// The two cards a seat was dealt, so a client can show them before choosing.
#[derive(Debug, Clone, Serialize)]
pub struct Deal {
    pub seat: PlayerId,
    pub is_bot: bool,
    pub cards: Vec<DealCard>,
    /// The card a bot seat keeps, `1` or `2`; `None` on a human seat.
    pub bot_keep: Option<u8>,
}

/// The set table (M5.5 spec §3.3.1): the position after setup, a throwaway
/// projection over a session that is never persisted, beside the deals.
#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub seed: WireSeed,
    pub table: HistoricalFrame,
    pub deals: Vec<Deal>,
}

/// The bot's Keep: the argmax of `score_starting_order` over its two dealt
/// cards, ties to the first — the rule every evaluator contestant applies in
/// the arena (`greedy.rs:733`), so the web bot opens exactly as the bot that
/// produced the V4 figures. 1-based, as the wire numbers the cards.
pub fn bot_keep(deal: &[OrderCardId; 2], rules: &Rules) -> u8 {
    let score = |id: OrderCardId| rules.orders.get(id).map_or(i64::MIN, |c| score_starting_order(c, rules));
    if score(deal[1]) > score(deal[0]) { 2 } else { 1 }
}
```

`config_of` keeps every check it has except the `keeps` mapping, and returns `(GameConfig, BotSeats)`; its `keeps` check becomes: length equals `n` (`"{} keep entries for {n} seats"`), and per seat: a bot seat with `Some` → `Err(format!("seat {i} is a bot; its keep is the bot's — send null"))`, a human seat with `None` → `Err(format!("seat {i} is human and needs a keep of 1 or 2"))`, a human seat with `Some(k)` not in `1..=2` → `Err(format!("keep {k} — the dealt cards are numbered 1 and 2"))`. Then:

```rust
/// Every seat's starting-order choice: the human's as sent, the bot's by
/// [`bot_keep`] over the deal the seed produced.
fn choices_of(req: &NewGame, bots: &BotSeats, deals: &[[OrderCardId; 2]], rules: &Rules) -> Vec<StartingOrderChoice> {
    deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            let seat = PlayerId::new(i as u8);
            let keep = if bots.is_bot(seat) { bot_keep(deal, rules) } else { req.keeps[i].expect("config_of checked it") };
            StartingOrderChoice::Keep(keep - 1)
        })
        .collect()
}

pub fn preview(req: &NewGame, rules: &Rules) -> Result<Preview, String> {
    let (config, bots) = config_of(req)?;
    let seed = req.seed.map_or_else(draw_seed, |s| s.0);
    let pending = new_game(config.clone(), rules, seed).map_err(|e| e.to_string())?;
    // A human seat previews as `Unknown` — no card in a hand nobody has chosen
    // from — and a bot seat at its argmax, the card the game will deal it.
    let choices: Vec<StartingOrderChoice> = pending
        .deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            if bots.is_bot(PlayerId::new(i as u8)) {
                StartingOrderChoice::Keep(bot_keep(deal, rules) - 1)
            } else {
                StartingOrderChoice::Unknown
            }
        })
        .collect();
    let mut session = Session::new_sim(config, seed, choices, rules.clone())?;
    session.set_run_seats(bots.seat_provenance());
    let repl = Repl::new(session);
    let table = HistoricalFrame::at(&repl, 0)
        .map_err(|e| format!("the setup position did not build: {}", e.message))?;
    let deals = pending
        .deals
        .iter()
        .enumerate()
        .map(|(i, deal)| {
            let seat = PlayerId::new(i as u8);
            let is_bot = bots.is_bot(seat);
            Deal {
                seat,
                is_bot,
                cards: deal
                    .iter()
                    .filter_map(|id| rules.orders.get(*id).cloned())
                    .map(|card| DealCard { keep_score: score_starting_order(&card, rules), card })
                    .collect(),
                bot_keep: is_bot.then(|| bot_keep(deal, rules)),
            }
        })
        .collect();
    Ok(Preview { seed: WireSeed(seed), table, deals })
}
```

In `create`: `let (config, bots) = config_of(req)?;` then after the id is settled and the store checked, `let pending = new_game(config.clone(), &self.rules, seed).map_err(|e| e.to_string())?; let choices = choices_of(req, &bots, &pending.deals, &self.rules);` and `Session::new_sim(config, seed, choices, (*self.rules).clone())?` as before. If `Repl::new` does not mark the bot seat in `seat_facts()` from the record's run seats alone (the preview cell's `is_bot` assertion will say), attach the bots the way `create` does — read `create` and `bring_alive` and mirror exactly what they do to the `Repl` short of pumping.

Run: `cargo test -p yokohama-web games::tests` — the four cells pass; the existing `req()` builder's `keeps: vec![1, 1]` must become `vec![Some(1), None]` (it has a bot at seat 1).

- [ ] **Step 3: The route cells, the corpus, the contract**

In `lib.rs`'s tests, beside the `deny_unknown_fields` cell:

```rust
    #[tokio::test]
    async fn the_preview_serves_the_set_table_and_refuses_a_keep_on_a_bot_seat() {
        let body = serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": 20260831,
            "keeps": [1, null], "bot_seats": [1]
        });
        let (status, preview) = post_json("/api/games/preview", body.clone()).await;
        assert_eq!(status, StatusCode::OK, "{preview}");
        assert_eq!(preview["table"]["index"], serde_json::json!(0));
        assert!(preview["deals"][1]["bot_keep"].is_u64(), "{}", preview["deals"][1]);
        assert!(preview["deals"][0]["bot_keep"].is_null());
        assert!(preview["deals"][0]["cards"][0]["keep_score"].is_i64());
        let mut on_bot = body;
        on_bot["keeps"] = serde_json::json!([1, 1]);
        let (status, err) = post_json("/api/games/preview", on_bot).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{err}");
        assert!(err["error"].as_str().unwrap_or("").contains("seat 1"), "{err}");
    }
```

In `tests/wire_fixtures.rs`: in `walk()`, after the `preview` capture, add a second preview with a bot (no create — a preview pumps nothing and is deterministic):

```rust
    let mut with_bot = new_game_body();
    with_bot["bot_seats"] = json!([1]);
    with_bot["keeps"] = json!([1, null]);
    with_bot["name"] = json!("wire-bot");
    let (status, preview_bot) = api.call("POST", "/api/games/preview", Some(with_bot)).await;
    assert_eq!(status, StatusCode::OK, "{preview_bot}");
```

and push `("preview_bot".to_string(), preview_bot)` into `out` beside `preview`. In `no_committed_payload_carries_the_setup_seed_except_the_preview`'s exemption branch, add before `continue`:

```rust
            let table = serde_json::to_string(&value["table"]).expect("serializable");
            assert!(!table.contains(&decimal), "{name}.table carries the setup seed — the position is served seed-free like every frame");
```

Regenerate and check: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && cargo test -p yokohama-web --test wire_fixtures && git diff --stat web/src/wire/`. Expected: `preview.json` changes shape, `preview_bot.json` appears, nothing else moves.

Client: `web/src/types.ts` — `Preview` becomes `{ seed: string; table: HistoricalFrame; deals: Deal[] }` with the new `Deal` and `DealCard` interfaces (from this task's Interfaces block). `web/src/api.ts` — `NewGameRequest.keeps: (number | null)[]`. `web/src/wire/contract.ts` — import `previewBot from "./preview_bot.json"`, `const _previewBot: Widen<Preview> = previewBot;`, `PREVIEW = ["seed", "table", "deals"]`, add `DEAL = ["seat", "is_bot", "cards", "bot_keep"]` and `DEAL_CARD = ["card", "keep_score"]` with their `KeysOf` pins and `KEYS.deal`/`KEYS.dealCard`, and `previewBot: as<Preview>(previewBot)` in `wire`. `web/src/wire/contract.test.ts` — add `["preview_bot", wire.previewBot, KEYS.preview]`, `["preview.deal", wire.preview.deals[0], KEYS.deal]`, `["preview.dealCard", wire.preview.deals[0]!.cards[0], KEYS.dealCard]` to the roots table, and one cell: `wire.previewBot.deals[1]!.bot_keep` is 1 or 2 and `wire.preview.deals.every((d) => d.bot_keep === null)`.

- [ ] **Step 4: Gate and commit**

`NewGame.test.tsx` will FAIL here (it still sends `keeps: number[]` and reads the old `Preview`) — expected; Task 12 rewrites it. Everything else: `cd web && npm run typecheck && npx vitest run --exclude src/NewGame.test.tsx && cd .. && cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings` — green.

```bash
git add crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire web/src/types.ts web/src/api.ts
git commit -m "feat(web): B — the preview is the position after setup, and the bot keeps its card by the arena's argmax"
```

---

### Task 12: The set table (spec §3.3.3, §4.2–§4.4)

**Files:**
- Modify: `web/src/NewGame.tsx` (the second step becomes the set table; `onStarted(state, opening)`; the keeps it sends), `web/src/NewGame.test.tsx` (rewritten around `wire.preview`/`wire.previewBot`), `web/src/App.tsx` (passes `labels` to `NewGame`; `show(s, opening)`), `web/src/TableSections.tsx` (nothing new — Task 9's nullable props), `web/src/styles.css` (`.deal-seat`, `.bot-keeps`, the keep hedge), `web/e2e/harness.ts` (`startGame` chooses a card per human seat), `web/e2e/the_table.spec.ts` (one cell), `web/e2e/gates.spec.ts` (gate B), `web/e2e/shots.spec.ts` (regenerate the five help pictures), `web/src/Help.tsx` (a "Starting a game" paragraph), `web/public/help/*.png`, `README.md` (the browser-client section's replay sentence and a set-table sentence)

**Interfaces:**
- Consumes: `wire.preview`/`wire.previewBot` (Task 11), `Board`, `TableSections` with `game={null}` and `state={null}` (Task 9), `OrderCardView` (`Cards.tsx`, with its `note` prop), `seatClass`.
- Produces: the DOM the harness clicks — `section.deal-seat.human[data-seat]` holding `button.deal-pick[aria-label="keep Oxx for seat n"]` per card; `section.deal-seat.bot` holding a `.bot-keeps` mark on the kept card; a `Start` button enabled only when every human seat has chosen.

- [ ] **Step 1: The failing cells**

Rewrite `web/src/NewGame.test.tsx`'s `describe("the deal")` and adjust the two cells that click Start (`"posts create exactly once"`, `"goes back to the server as the exact string"`) and `"names every field the create endpoint reads"` to choose a card first with `await user.click(screen.getAllByRole("button", { name: /^keep O\d\d for seat/ })[0]!)`. The new `describe("the deal")`:

```ts
describe("the set table", () => {
  const dealt = async (fixture: typeof wire.preview) => {
    const shown = show((call) => {
      if (call.method === "GET") return [];
      if (call.path.endsWith("preview")) return fixture;
      return wire.options;
    });
    await shown.user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    return shown;
  };

  it("shows the table the rulebook has set at step 12: the pyramid and the strips, not a list", async () => {
    await dealt(wire.preview);
    expect(document.querySelectorAll(".board .tile")).toHaveLength(wire.preview.table.frame.board.tiles.length);
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    expect(document.querySelector(".table-sec.labs .board-strip")).not.toBeNull();
    expect(document.querySelector(".table-sec.achievements")).not.toBeNull();
    expect(screen.getByText(/setting up/)).toBeInTheDocument();
    expect(screen.getByText(wire.preview.seed)).toBeInTheDocument();
  });

  it("shows every human seat's two cards with the bot's Keep score and the CLI's hedge, and Start waits for a choice", async () => {
    const { user } = await dealt(wire.preview);
    for (const deal of wire.preview.deals) {
      for (const c of deal.cards) {
        expect(screen.getByRole("button", { name: `keep O${String(c.card.id).padStart(2, "0")} for seat ${deal.seat}` })).toBeInTheDocument();
        expect(screen.getAllByText(new RegExp(`Keep score ${c.keep_score}\\b`)).length).toBeGreaterThan(0);
      }
    }
    expect(screen.getByText(/hand-set, unmeasured, and no substitute for your own read/)).toBeInTheDocument();
    const start = screen.getByRole("button", { name: "Start" });
    expect(start).toBeDisabled();
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 0/ })[1]!);
    expect(start).toBeDisabled();
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 1/ })[0]!);
    expect(start).toBeEnabled();
  });

  it("shows the bot's two cards, marks the one it keeps, and offers no choice on them", async () => {
    await dealt(wire.previewBot);
    const bot = wire.previewBot.deals.find((d) => d.is_bot)!;
    expect(bot.bot_keep === 1 || bot.bot_keep === 2, "the fixture's bot seat carries a keep").toBe(true);
    const seat = document.querySelector(`.deal-seat.bot[data-seat="${bot.seat}"]`)!;
    expect(seat).not.toBeNull();
    expect(seat.querySelectorAll("button.deal-pick")).toHaveLength(0);
    expect(seat.querySelectorAll(".card-face")).toHaveLength(2);
    expect(seat.querySelector(".bot-keeps")?.textContent).toMatch(/bot keeps this/);
    const kept = bot.cards[bot.bot_keep! - 1]!.card.id;
    expect(seat.querySelector(".bot-keeps")?.closest(".deal-card")?.textContent).toContain(`O${String(kept).padStart(2, "0")}`);
  });

  it("sends a keep per human seat and null per bot seat, numbered as on screen", async () => {
    const { stub, user, onStarted } = await dealt(wire.previewBot);
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 0/ })[1]!);
    await user.click(screen.getByRole("button", { name: "Start" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, true));
    const create = stub.matching("POST", "/api/games").filter((c) => !c.path.endsWith("preview"));
    expect((create[0]!.body as { keeps: unknown }).keeps).toEqual([2, null]);
  });

  it("resumes a saved game as NOT an opening", async () => {
    const { user, onStarted } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return ["wire"];
      if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return wire.options;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await user.click(await screen.findByRole("button", { name: "wire" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, false));
  });
});
```

`show()` in that file renders `<NewGame art={wire.artManifest} labels={wire.labels} onStarted={onStarted} />` — add the `labels` prop. The existing `"resumes one with a GET"` cell's `toHaveBeenCalledWith(wire.options)` becomes `toHaveBeenCalledWith(wire.options, false)`.

Run: `cd web && npx vitest run src/NewGame.test.tsx` — FAIL throughout.

- [ ] **Step 2: The screen**

`web/src/NewGame.tsx` — props gain `labels: Labels | null`; `onStarted: (state: ClientState, opening: boolean) => void`; state `keeps` becomes `Record<number, 1 | 2>` (chosen per human seat, empty after Deal); `base()` sends `keeps: Array.from({ length: players }, (_, s) => (s === botSeat ? null : (chosen[s] ?? null)))` — for the preview any value passes because the server ignores keeps on preview only through `config_of`'s shape check, so send `1` for every human seat there: `previewKeeps = seats.map((s) => (s === botSeat ? null : 1))`; for create send the chosen ones. The first step (players, seed, bot seat, Deal, saved games) is unchanged apart from `onStarted(await api.get(name), false)`.

When `preview` is set, return the set table instead of the `.setup` block:

```tsx
  const colors = COLORS.slice(0, players) as PlayerColor[];
  const humans = preview.deals.filter((d) => !d.is_bot);
  const ready = humans.every((d) => chosen[d.seat] !== undefined);
  const pad = (id: number) => `O${String(id).padStart(2, "0")}`;
  return (
    <div className="app setup-table">
      <header className="banner">
        <h1>Yokohama</h1>
        <span className="meta">
          setting up &middot; seed <code>{preview.seed}</code>
        </span>
        <span className="meta turn">choose your card</span>
      </header>
      <div className="end-strip" role="status" />
      <main className="table">
        <div className="table-body">
          <Board board={preview.table.frame.board} art={art} labels={labels} colors={colors} />
          <TableSections
            enabled={loadSections()}
            game={null}
            state={null}
            replaying={false}
            view={preview.table}
            art={art}
            labels={labels}
            colors={colors}
          />
        </div>
      </main>
      <aside className="side">
        {preview.deals.filter((d) => d.is_bot).map((d) => (
          <section className="panel deal-seat bot" data-seat={d.seat} key={d.seat}>
            <h2>
              <span className={seatClass(colors[d.seat])}>{colors[d.seat]}</span> &middot; bot, seat {d.seat}
            </h2>
            <div className="deal">
              {d.cards.map((c, i) => (
                <div className="deal-card" key={c.card.id}>
                  <OrderCardView art={art} card={c.card} id={c.card.id} size="sm" note={`Keep score ${c.keep_score}`} />
                  {d.bot_keep === i + 1 && <div className="bot-keeps">bot keeps this</div>}
                </div>
              ))}
            </div>
          </section>
        ))}
      </aside>
      <div className="deck">
        <div className="decision">
          <div className="asking">Keep one card per seat. The table is set as the rulebook has it at step 12.</div>
          {humans.map((d) => (
            <section className="deal-seat human" data-seat={d.seat} key={d.seat}>
              <div className="field">
                <label>
                  <span className={seatClass(colors[d.seat])}>{colors[d.seat]}</span> &middot; seat {d.seat}
                </label>
              </div>
              <div className="deal">
                {d.cards.map((c, i) => (
                  <button
                    type="button"
                    className={`deal-pick ${chosen[d.seat] === i + 1 ? "chosen" : ""}`}
                    aria-pressed={chosen[d.seat] === i + 1}
                    aria-label={`keep ${pad(c.card.id)} for seat ${d.seat}`}
                    key={c.card.id}
                    onClick={() => setChosen((k) => ({ ...k, [d.seat]: (i + 1) as 1 | 2 }))}
                  >
                    <OrderCardView art={art} card={c.card} id={c.card.id} size="sm" note={`Keep score ${c.keep_score}`} />
                  </button>
                ))}
              </div>
            </section>
          ))}
          <p className="muted keep-hedge">
            the Keep score is what the built-in bot would prefer, from the card&rsquo;s printed
            economics alone &mdash; hand-set, unmeasured, and no substitute for your own read
          </p>
          <div className="controls">
            <button type="button" className="primary" disabled={busy || !ready} onClick={() => run(async () => onStarted(await api.create({ ...base(), seed: preview.seed, keeps: createKeeps() }), true))}>
              Start
            </button>
            <button type="button" className="ghost" disabled={busy} onClick={() => { setPreview(null); setChosen({}); }}>
              Re-deal
            </button>
          </div>
          {error && <div className="error">{error}</div>}
        </div>
      </div>
    </div>
  );
```

Imports: `Board`, `TableSections`, `loadSections`, `seatClass`, `Labels`, `PlayerColor`. The hedge is the CLI's `KEEP_SCORE_NOTE` (`wizard.rs:1384-1385`) verbatim modulo line breaks. `App.tsx`: `<NewGame art={art} labels={labels} onStarted={show} />` and `show(s, opening)` per Task 9. Styles:

```css
/* ---- M5.5: the set table ------------------------------------------------ */
.deal-seat .deal { flex-wrap: nowrap; }
.deal-card { position: relative; }
.bot-keeps { position: absolute; left: 0.3rem; bottom: 0.3rem; padding: 0.1rem 0.4rem; border-radius: 3px; background: var(--vp); color: #111; font-size: 10px; font-weight: 700; }
.keep-hedge { font-size: 11px; margin: 0.4rem 0; }
```

Run: `cd web && npm test && npm run typecheck` — green.

- [ ] **Step 3: The browser harness, the cell, the gate, the help**

`web/e2e/harness.ts`'s `startGame`, after `await expect(page.getByRole("button", { name: "Start" })).toBeVisible();`:

```ts
  // The set table (M5.5): Start waits until every human seat has chosen a
  // card, and the walk chooses the first one — the bot's is the bot's.
  for (const seat of await page.locator(".deal-seat.human").all()) {
    await seat.locator("button.deal-pick").first().click();
  }
```

Append to `web/e2e/the_table.spec.ts`:

```ts
test("the set table shows the port and the bot's kept card before the first choice, and the chosen card lands in hand", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await page.goto("/");
  await page.getByRole("button", { name: "Deal" }).click();
  await expect(page.locator(".table-sec.ports .strip-slot .card-face")).toHaveCount(6);
  await expect(page.locator(".board .tile")).toHaveCount(10);
  await expect(page.locator(".deal-seat.bot .bot-keeps")).toHaveCount(1);
  await expect(page.getByRole("button", { name: "Start" })).toBeDisabled();
  const pick = page.locator(".deal-seat.human button.deal-pick").nth(1);
  const chosen = /keep (O\d\d)/.exec((await pick.getAttribute("aria-label"))!)![1]!;
  await pick.click();
  await page.getByRole("button", { name: "Start" }).click();
  await expect(page.getByRole("heading", { name: "Yokohama" })).toBeVisible();
  await readScreen(page);
  await expect(page.locator(".panel", { hasText: /'S HAND|’s hand/i }).first()).toContainText(chosen);
  expect(watch.failures).toEqual([]);
});
```

(The hand panel's heading is `RED'S HAND` in the live client — read `Dashboard.tsx`'s `Hand` for its exact class and use that selector instead of the regex if it has one.)

`web/e2e/gates.spec.ts` gate B: at 1710×869, `page.goto("/")`, Deal, wait for the six port cards, screenshot `b-set-table-1710x869.png`. `web/src/Help.tsx`: a paragraph under "Saving, and coming back" titled "Starting a game": the deal happens over the set table — the areas with their site cards and tokens, the port, the laboratory and the goals, exactly what the rulebook has on the table when the two cards are drawn — with the built-in bot's Keep score under each card and its own card marked "bot keeps this". Regenerate the five help pictures (`YOKOHAMA_WEB_SHOTS=1 npx playwright test shots.spec.ts`) since `replay.png` now shows the strips at a past position; RE-READ every alt and caption in `Help.tsx` against the fresh frames and correct any that is false. `README.md` §"The browser client": replace the sentence "The port and laboratory offers and the building sites collapse … the tracks and the goals keep drawing" with "The port and the laboratory draw from that same past position; only the building sites collapse."; add one sentence to the first paragraph about choosing the first card over the set table.

Run: `cd web && npm run e2e && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts -g "gate B"` — green.

- [ ] **Step 4: Commit**

```bash
git add web/src web/e2e web/public/help README.md
git commit -m "feat(web): B — the first card is chosen over the set table, the bot's card marked; the help and its pictures"
```

Report the counts and `b-set-table-1710x869.png`; the orchestrator sends it to the owner (no stop).

---

### Task 13: Deploy `yokohama-web:0.5.0-m5.5` to the NAS over Portainer (owner-authorized)

**Run by the orchestrator, not a subagent** — an outward-facing action, exactly as M5's Task 15 was; the owner authorized it in this plan's ratification (2026-09-04) for a dev window. Record: `deploy/RUNBOOK.md` §3.5 is the recipe; this task adds §3.6.

**Files:**
- Modify: `deploy/RUNBOOK.md` (append §3.6)

- [ ] **Step 1: Build on the workstation** (runbook §1)

```bash
LLD="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin/rust-lld"
RUSTFLAGS="-C linker=$LLD -C link-self-contained=yes" cargo build --release --target x86_64-unknown-linux-musl -p yokohama-web
file target/x86_64-unknown-linux-musl/release/yokohama-web      # static-pie linked
cd web && npm run build && cd ..
rm -rf /tmp/yokohama-ctx && mkdir /tmp/yokohama-ctx \
  && cp target/x86_64-unknown-linux-musl/release/yokohama-web /tmp/yokohama-ctx/ \
  && cp -R web/dist /tmp/yokohama-ctx/dist && cp -R crates/yokohama-web/assets/art /tmp/yokohama-ctx/art \
  && cp deploy/Dockerfile.runtime /tmp/yokohama-ctx/Dockerfile \
  && tar czf /tmp/ctx.tar.gz -C /tmp/yokohama-ctx . && ls -l /tmp/ctx.tar.gz
```

Record the binary's size and the tarball's. Serve it on the LAN: `cd /tmp && python3 -m http.server 8899 --bind 0.0.0.0` in the background (the workstation is 192.168.1.16; the NAS 192.168.1.173).

- [ ] **Step 2: Build the image on the NAS** (runbook §2), through the Portainer MCP's Docker proxy (environment id 2):

`POST /build?remote=http://192.168.1.16:8899/ctx.tar.gz&t=yokohama-web:0.5.0-m5.5&dockerfile=Dockerfile&rm=true` — record the image size and build time. Stop the file server afterwards.

- [ ] **Step 3: Swap the container** (runbook §3.5)

`GET /containers/yokohama-web/json` → note `NetworkSettings.Networks["cf-internal"].IPAddress` (172.21.0.4 at M5) and the two labels. Then `POST /containers/yokohama-web/stop`, `DELETE /containers/yokohama-web`, `POST /containers/create?name=yokohama-web` with body:

```json
{ "Image": "yokohama-web:0.5.0-m5.5",
  "Labels": { "maintainer": "<as before>", "purpose": "<as before>" },
  "HostConfig": { "NetworkMode": "cf-internal",
                  "RestartPolicy": { "Name": "unless-stopped" },
                  "Mounts": [{ "Type": "volume", "Source": "yokohama-web-data", "Target": "/data" }] } }
```

then `POST /containers/yokohama-web/start`. Re-read the container's IP: if it differs from the one noted, `POST /containers/cloudflared-nas/restart` (cloudflared caches origin DNS — the memory's rule; at M5 the address was stable and no restart was needed).

- [ ] **Step 4: Verify through the tunnel** (runbook §3.2)

```bash
curl -sS https://yokohama.moore.nyc/api/health
curl -sS https://yokohama.moore.nyc/ | grep -o 'assets/index-[A-Za-z0-9_-]*\.js'   # the bundle THIS build ships
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/help/screen.png
```

Then, in Claude-in-Chrome (it carries the Access cookie): open the site, Deal, see the set table, choose, Start, play one answer, watch the bot's turn play out, skip. The owner's saved games must still be listed; open one read-only, do not play on it.

- [ ] **Step 5: Record**

Append `### 3.6 M5.5 is live — the third deployment, <date>` to `deploy/RUNBOOK.md` in §3.5's shape: the build commit, the sizes, the image tag and build time, the swap, the address, the health time, what was verified, the rollback tags now on the NAS (`0.5.0`, `0.5.0-m5`, `0.5.0-m5.1`, `0.5.0-m5.5`). Commit: `docs(deploy): M5.5 is live — yokohama-web:0.5.0-m5.5 on the NAS, the third deployment recorded`.

---

### Task 14: The final review, against the live version, by a clean subagent (owner direction, 2026-09-04)

**Run by the orchestrator: ONE fresh-context subagent (Fable at max), no task-review context, no prior conversation.** Its report goes to `.superpowers/sdd/2026-09-04-web-frontend-m5.5-plan/final-review.md` (gitignored) and its verdict is the gate for Task 15.

- [ ] **Step 1: Dispatch with this brief, verbatim**

> You are the final reviewer of the M5.5 milestone of the Yokohama browser client. Read `CLAUDE.md`, then the spec `docs/plans/2026-09-04-web-frontend-m5.5-spec.md`, then the plan `docs/plans/2026-09-04-web-frontend-m5.5-plan.md`. The branch is `web-frontend` in the worktree `.claude/worktrees/cli-web-api-port-936d56`; the range under review is `652badc..HEAD`. The LIVE deployment of this HEAD is `https://yokohama.moore.nyc` (Cloudflare Access; use the Claude-in-Chrome tools, which carry the cookie — load them with ToolSearch first). Two halves, in this order:
>
> **1. Play the live site.** Start a new 2P game against the bot: on the set table, check that the port strip holds six cards and that the bot's two cards are shown with one marked "bot keeps this"; choose a card; Start. Play until the bot's turn plays out: count the steps, watch the outline and the highlights, try pause, step back, step forward, skip, replay, and the cadence control (change it, reload the page, check it stuck). Place two assistants twice into one area with the President left in hand (the Movement row "Return President to hand") and check the four cubes stand as a 2×2 pile with no count. Open the Log drawer, scrub into the past, and check the port strip draws from that position. Undo a move. Switch games from the Game menu and back. Note anything that looks wrong ON SCREEN — this lane's last three reviews each found defects visible only there.
>
> **2. Review the code, with vacuity as the FIRST lens.** For every test the milestone added, ask what mutation would leave it green; try at least ten mutations you consider most likely to pass (a floor set to 0, a diff that reads nothing, a walk that never ticks, a keep that ignores the argmax, a `--piece-units` that is always 1, a harness that never reads the strip) and report each with its result. Then read every sentence the milestone wrote about itself — doc comments, the help, the README, the spec's §3 and §5 — and try the mechanism each promises. Then the ordinary lenses: correctness, the wire contract's two halves, hidden-information fences (no play payload carries the setup seed; the preview's `table` is seed-free; `/at/{k}` still pins one seat), the blast radius (`git diff e4f41e4..HEAD -- crates/yokohama-core crates/yokohama-cli` EMPTY), accessibility of the new controls, and the below-1201px stack.
>
> Run the gate yourself and report the numbers: `cd web && npm run typecheck && npm test && npm run e2e; cd .. && cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings`.
>
> Report as Critical / Important / Minor, each finding with a reproduction you actually ran (a command, a click sequence, a mutation and its result), then a LEAVE list of what you saw and chose not to raise, then a verdict: ready to close, ready with fixes, or not ready. Write it to `.superpowers/sdd/2026-09-04-web-frontend-m5.5-plan/final-review.md`. Do not fix anything; do not commit.

- [ ] **Step 2: Adjudicate**

The orchestrator reads the report, checks every finding's reproduction against the claim, and rules fix-now / leave / not-a-defect per finding, recording each ruling in the SDD ledger. Every Critical and Important is fix-now unless the ruling says why not.

---

### Task 15: The fix wave, the scoped re-review, the redeploy

**Files:** whatever the findings name; `deploy/RUNBOOK.md` (§3.6 addendum) if anything redeploys.

- [ ] **Step 1:** One implementer per finding group (Opus), TDD as everywhere: the failing cell that reproduces the finding first, then the fix, then the gate. One commit per finding, message `fix(web): <the finding in one line>`.
- [ ] **Step 2:** A scoped re-review (Opus, fresh) over the wave's commits only, with the original findings and their fixes as the brief; every finding ADDRESSED or explicitly LEFT with the orchestrator's ruling.
- [ ] **Step 3:** If any code changed, Task 13's Steps 1–5 again as `yokohama-web:0.5.0-m5.5.1`, recorded as an addendum to §3.6 (M5's addendum is the shape). If only docs changed, no redeploy; say so in the ledger.

---

### Task 16: The M5.5 close, M6's entry artifact (spec §6)

**Files:**
- Create: `docs/plans/2026-09-xx-web-frontend-m5.5-close.md`
- Modify: `guidingpath.md` (three rows), `CLAUDE.md` (the browser-half counts and an M5.5 bullet after the M5 one), `README.md` (re-check), the memory file `yokohama-web-lane-status.md` (+ its `MEMORY.md` line)

- [ ] **Step 1: Re-measure at HEAD** — Task 0's Step 1 command, plus `git diff --stat e4f41e4..HEAD -- crates/` (expected: only `crates/yokohama-web/src/{client,games,lib}.rs` and `tests/wire_fixtures.rs`) and the blast-radius command from the Global Constraints (expected EMPTY).

- [ ] **Step 2: The close record** — `docs/plans/2026-09-xx-web-frontend-m5.5-close.md` with, in this order: (1) what shipped per part with the gate pictures' file names and the recorded readings (the pile cube's px, the ink readings and L, the observed turn's step count, the deploy sizes); (2) the rulings taken during execution with their cost (the SDD ledger's list); (3) deviations from the spec — the plan's list below plus any added in execution; (4) the final review's findings and their rulings; (5) the counts at the close beside the counts at the open; (6) the blast-radius statement; (7) **the handover into M6**, verbatim from spec §6.3: the walk as the spectator primitive, M4's hosting list by reference (`m4-spec.md:507-524`), M6's first three questions with the stated leans, M5's LEAVE list items M6 inherits, and this milestone's own LEAVE list; (8) what is not claimed (spec §5 restated with any change).

- [ ] **Step 3: The three guidingpath rows**, appended in order: the M5.5 close row (the shape of Task 0's row; Why column: the lesson the final review taught); the draw-order note (spec §6.1 item 3, verbatim); the V5 notes row (spec §6.2's two items, verbatim, marked "for the next AI cycle's entry artifact").

- [ ] **Step 4: CLAUDE.md, README, memory** — the counts on lines 30, 33, 34 (the server count moves this time); an M5.5 bullet after the M5 one naming the walk, the set table, the pile rule and the two wire additions; README re-read against the regenerated `screen.png` and `replay.png`. The memory file `yokohama-web-lane-status.md`: M5 closed, M5.5 closed and deployed as `0.5.0-m5.5[.1]`, M6 next with the close doc as its entry, the deploy recipe unchanged; update its `MEMORY.md` index line.

- [ ] **Step 5: Commit, and delete the SDD workspace**

```bash
git add docs/plans/2026-09-*-web-frontend-m5.5-close.md guidingpath.md CLAUDE.md README.md
git commit -m "docs(web): M5.5 closes — the record, the rows, the counts, and M6's entry"
rm -rf .superpowers/sdd/2026-09-04-web-frontend-m5.5-plan && git status --short
```

Report to the owner: the counts, the deviations, the live tag, the final review's verdict, and the three M6 questions.

---

## Deviations from the spec, decided while planning (recorded at the close)

1. **There is no Task 10** — the help paragraphs and gate C are inside Task 9, where the code they describe lands.
2. **The ink measurement gets a by-eye check.** The pale-ground test cannot tell illustration from icon ink on its own, so `tilerects.py --ink` counts only ink with a pale run to its right AND the tool draws candidate lines for the implementer to check on every art; the eye wins a disagreement (spec §3.1.2 named only the measurement).
3. **The collapse rule stops as soon as the floor holds**, one group at a time, largest first — the spec's own sentence read strictly; the mock's V3 column collapsed all four colours of the 4P crowd at once, so the plan's cells expect ONE collapse there.
4. **M5's close writes §8 into the M5 spec** (the task-16 brief's shape) rather than the separate close file spec §3.0 named; one document, no duplicate.
5. **Gate A re-runs the V1 flow** rather than adding a flow; the pile cell takes its own picture.
6. **`readScreen` skips a running walk** — a harness behaviour the spec did not name, needed so every existing browser cell stays fast; the playback cell observes before it reads.
7. **The e2e `startGame` chooses the first card for every human seat** — spec §4.3 did not list the harness change the set table forces.
8. **The account shows rows for a run of the player's own events** (the walk does not play); spec §3.2.5's "when there is no run" is read as "when the reply added no event".
9. **A created game's opening bot run walks; a resumed game's does not** — `onStarted(state, opening)` carries the distinction the spec's §3.2.2 last bullet implies.
10. **The final review is one clean subagent against the live site** (owner, 2026-09-04) followed by a scoped re-review of the fix wave — the M5 shape with the live half added.

## Self-review (the planner's, 2026-09-04)

- **Spec coverage:** §3.0 → Task 0; §3.1.1/§3.1.3 → Tasks 1–2; §3.1.2 → Task 3; §3.2.1 → Task 5; §3.2.4 → Task 6; §3.2.2/§3.2.3 → Tasks 7, 9; §3.2.5 → Task 8; §3.3.1/§3.3.2 → Task 11; §3.3.3 → Task 12; §3.4 → the task order; §4.1 → Tasks 5, 11; §4.2 → Tasks 1, 2, 3, 6, 7, 8, 9, 12; §4.3 → Tasks 4, 9, 12; §4.4 → Tasks 4, 9, 12; §4.5 → Tasks 0, 16; §5 → Task 16; §6.1–§6.4 → Task 16; the owner's two plan additions → Tasks 13, 14–15.
- **Placeholder scan:** none; every `<…>` is a value the executing task measures (a date, a hash, a reading), named as such.
- **Type consistency:** the names each task produces are the names the later tasks consume (`layoutPieces`/`bandAspect`/`PIECES`; `ICON_INK_RIGHT`/`BAND_MARGIN`; `diffFrames`/`Changes`; `buildRun`/`hasBotStep`/`dwellFor`/`walkReducer`/`WalkState`/`WalkAction`; `CADENCES`/`Cadence`/`DEFAULT_CADENCE`/`QUICK_DWELL`/`loadCadence`/`saveCadence`; the `TurnAccount` props; `Board.changed`/`changedBy`, `Dashboard.changedSeats`, `PortStrip`/`LabStrip.changed`, `TableSections.game|state` nullable; `.decision.walking`; `ClientState.events`, `HistoricalFrame.ports/labs`, `api.log(game, tail?)`; `Preview`/`Deal`/`DealCard`, `NewGameRequest.keeps`, `bot_keep`, `choices_of`, `wire.previewBot`, `KEYS.deal`/`dealCard`; `.deal-seat.human|bot`, `button.deal-pick`, `.bot-keeps`; `onStarted(state, opening)`).
