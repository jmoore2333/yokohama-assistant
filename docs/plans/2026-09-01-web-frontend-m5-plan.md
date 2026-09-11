# Web frontend — M5 "the table": implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-lay the browser client above 1201px as a scrolling table (the pyramid first, the port/lab/track/goal/site strips under it) beside a side column with the answer bar pinned at its top, and make each tile the area art alone with the site card and the pieces drawn on it.

**Architecture:** Client-only. `App.tsx` becomes a CSS grid of four regions (banner, strip, table, side + deck); the tile's caption is replaced by percentage-positioned overlays whose rects live in one JSON read by both the client and a review script; the tile width is one CSS `clamp()` over container-query units with a pure TS twin used only as the test oracle; the four reference drawers that a player reads mid-turn become toggleable table sections that re-fetch after every applied move. The server, the wire and `crates/yokohama-core` are untouched.

**Tech Stack:** React 19 + TypeScript + Vite (client), vitest + Testing Library over jsdom (component tests), Playwright against the real release binary (browser walk), Pillow + PyYAML (the art generator), the existing axum server unchanged.

**Spec:** `docs/plans/2026-09-01-web-frontend-m5-spec.md` — the authority; this plan argues from it and records where it deviates (§ "Deviations" at the end, and the close in Task 16).

## Global Constraints

- **Branch:** `web-frontend`, in the worktree `.claude/worktrees/cli-web-api-port-936d56`. Every command below runs from that worktree root unless it says `cd web`. Stage with explicit paths (`git add <path>`), never `git add -A`. Commit messages end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01T6ECCjntqUQrLTCidp9e5L`.
- **Blast radius:** `git diff master..HEAD -- crates/yokohama-core` stays EMPTY; `SEMANTICS_REV` stays **7**; `crates/yokohama-web` changes only under `assets/art/**` and the one achievements row of `art.rs`'s `emitted_art_keeps_the_sheet_aspect` (`art.rs:404`). Check with `git diff --stat master..HEAD -- crates/` at Task 16.
- **Per-edit gate (after EVERY task):** `cd web && npm test && npm run typecheck`. When a task touches `crates/`: also `cargo test -p yokohama-web` and `cargo clippy --workspace --all-targets -- -D warnings`. When a task touches `web/e2e/` or the layout: also `cd web && npm run e2e` (needs `npx playwright install chromium` once; it builds `dist` and starts the release server itself).
- **Below 1201px:** the column stack and the bottom-pinned deck are unchanged (spec R8). Every `@media (max-width: 1200px)` rule that exists at HEAD stays; new layout rules go inside `@media (min-width: 1201px)`.
- **Legibility floor 10rem, ceiling 22rem** (spec §3.3). **No `location.reload`** anywhere in `web/src`.
- **Every number is arithmetic until an e2e cell reads it** (spec §5). Wall and pixel readings are machine-tagged (this Mac, Chrome, DPR 2).
- **Visual gates (spec §4.1):** Tasks 6, 13 and 14 end by sending screenshots to the owner with `SendUserFile` and STOPPING for the owner's answer. Do not proceed past a gate on your own reading.
- **The art is third-party** (`ASSETS.md`). Nothing derived from it is committed outside the paths `ASSETS.md` already lists; review sheets go under gitignored `docs/temp/web-review/`.
- Counts at HEAD `bc62080` (re-measure at Task 16, never copy): `npm test` **231**, `npm run e2e` **23** (+2 opt-in shots), `cargo test -p yokohama-web` **72**, wire corpus **43**.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `docs/tools/mkcardart.py` | The art generator: source widths, achievement rotation, provenance | 1 |
| `crates/yokohama-web/assets/art/**`, `web/src/wire/art_manifest.json` | Regenerated output | 1 |
| `crates/yokohama-web/src/art.rs:404` | The aspect the achievements now keep | 1 |
| `web/src/tile_rects.json` | THE overlay geometry, one file for code and picture | 2 |
| `web/src/tileRects.ts` (+ `.test.ts`) | Typed access, `intersects`, `inside`, `pct` | 2 |
| `docs/tools/tilerects.py` | Draws the rects on every art for the owner (gate V0) | 2 |
| `web/src/Pieces.tsx` (+ `.test.tsx`) | The president/cube/Station pieces and their words | 3 |
| `web/src/Cards.tsx` | Gains `SiteFace` (the printed site card with built marks) | 3 |
| `web/src/Board.tsx` (+ `.test.tsx`) | The art-only tile, the fit variables, the popover wiring | 4, 5, 8 |
| `web/src/TilePopover.tsx` (+ `.test.tsx`) | The hover/focus popover and its positioning | 5 |
| `web/e2e/gates.spec.ts` | The opt-in visual-gate shots | 6, 13 |
| `web/src/App.tsx` | The grid: `.table`, `.side`, the deck's new home; toggles; the Game menu | 7, 9, 10, 12 |
| `web/src/styles.css` | Layout, tile overlays, strips, side column | 4, 5, 7, 8, 9, 12 |
| `web/src/fit.ts` (+ `.test.ts`) | The fit rule as arithmetic — the test oracle | 8 |
| `web/src/LastMove.tsx` (+ `.test.tsx`) | The applied-move lines, out of the deck | 9 |
| `web/src/Decision.tsx`, `web/src/Dashboard.tsx` | Lose the messages block / gain `only` and visible VP on track spaces | 9 |
| `web/src/GameMenu.tsx` (+ `.test.tsx`) | Saved games, new game, from the banner | 10 |
| `web/src/sections.ts` (+ `.test.ts`) | Section keys, labels, defaults, persistence | 11 |
| `web/src/Strips.tsx` (+ `.test.tsx`) | The port/lab/tracks/goals/sites strips | 11 |
| `web/src/Surfaces.tsx` (+ `.test.tsx`) | Loses the four surfaces that became sections | 11 |
| `web/src/TableSections.tsx` (+ `.test.tsx`) | Enabled sections: fetch, refresh, replay collapse | 12 |
| `web/e2e/the_table.spec.ts` | M5's browser cells | 8, 10, 12 |
| `web/e2e/the_screen_itself.spec.ts`, `web/e2e/shots.spec.ts` | Re-aimed cells, new locators | 7, 11, 14 |
| `web/src/Help.tsx`, `web/public/help/*.png` | The words and pictures for the new screen | 14 |
| `ASSETS.md`, `deploy/RUNBOOK.md`, `README.md`, `CLAUDE.md`, `guidingpath.md` | Sizes, counts, the close | 1, 14, 16 |

---

### Task 1: W0 — re-cut the art at source width, achievements landscape

**Files:**
- Modify: `docs/tools/mkcardart.py:93-95` (widths), `:113-118` (provenance), `:258-266` (achievements loop)
- Modify: `crates/yokohama-web/src/art.rs:391-405` (the aspect row and its doc)
- Regenerate: `crates/yokohama-web/assets/art/**` (all 132 files + `manifest.json`), `web/src/wire/art_manifest.json`
- Modify: `web/src/art.test.ts` (two new cells), `ASSETS.md` (the derived-crops row), `deploy/RUNBOOK.md` §3.4, `CLAUDE.md:68` ("2.7 MiB of WebP")

**Interfaces:**
- Produces: manifest entries with `areas.*.w === 709`, `orders/techs/sites.*.w === 530`, `achievements.*.w === 799 && h === 530`; `provenance.achievement_width`, `provenance.achievement_rotation_ccw`.

- [ ] **Step 1: Record the payload before**

Run: `du -sk crates/yokohama-web/assets/art | cut -f1` and `ls crates/yokohama-web/assets/art/*/ | wc -l`
Write both numbers down; they go into `ASSETS.md` and the runbook as the "before".

- [ ] **Step 2: Write the failing Rust cell**

In `crates/yokohama-web/src/art.rs`, change the achievements row and the doc above it:

```rust
    /// Art keeps its source aspect ratio.
    ///
    /// Every card cell on every sheet is 530x799 and every area cell 709x531;
    /// the generator scales to a width and COMPUTES the height, so a wrong
    /// aspect here means the crop geometry moved, not that a constant was
    /// mistyped. Half a pixel of rounding is allowed and one whole one is not.
    ///
    /// The ACHIEVEMENTS are the one family stored sideways on their sheets and
    /// rotated by the generator (M5, `docs/plans/2026-09-01-web-frontend-m5-spec.md`
    /// §2), so they keep the cell's aspect turned on its side: 799x530.
    #[test]
    fn emitted_art_keeps_the_sheet_aspect() {
        let (_, m) = committed();
        for (family, entries, (cw, ch)) in [
            ("orders", &m.orders, (530.0, 799.0)),
            ("techs", &m.techs, (530.0, 799.0)),
            ("sites", &m.sites, (530.0, 799.0)),
            ("achievements", &m.achievements, (799.0, 530.0)),
            ("areas", &m.areas, (709.0, 531.0)),
        ] {
```

- [ ] **Step 3: Run it to see it fail**

Run: `cargo test -p yokohama-web emitted_art_keeps_the_sheet_aspect`
Expected: FAIL — `achievements/Tea7: 360x543 is not the 799x530 aspect`.

- [ ] **Step 4: Write the failing vitest cells**

Append to `web/src/art.test.ts`:

```ts
describe("the manifest's shapes (M5, W0)", () => {
  // The three achievement sheets store their cards SIDEWAYS; the generator
  // rotates them a quarter turn counter-clockwise before emitting, so a
  // landscape entry is the proof the rotation happened and a portrait one is
  // the old crop drawn on its side in the rail.
  it("achievements are landscape and every other card is portrait", () => {
    const m = wire.artManifest;
    for (const [key, e] of Object.entries(m.achievements)) {
      expect(e.w, `achievements/${key} is ${e.w}x${e.h}, not landscape`).toBeGreaterThan(e.h);
    }
    for (const family of [m.orders, m.techs, m.sites]) {
      for (const [key, e] of Object.entries(family)) {
        expect(e.h, `${key} is ${e.w}x${e.h}, not portrait`).toBeGreaterThan(e.w);
      }
    }
  });

  // Source widths, so the fit rule's 22rem ceiling is the art's native width on
  // a 2x display rather than a 1.6x upsample of a 440px crop.
  it("is cut at source width", () => {
    const m = wire.artManifest;
    expect(Object.values(m.areas)[0]!.w).toBe(709);
    expect(Object.values(m.orders)[0]!.w).toBe(530);
    expect(Object.values(m.achievements)[0]!.w).toBe(799);
  });
});
```

- [ ] **Step 5: Run them to see them fail**

Run: `cd web && npx vitest run src/art.test.ts`
Expected: FAIL on both new cells (`360` is not greater than `543`; `440` is not `709`).

- [ ] **Step 6: Change the generator**

In `docs/tools/mkcardart.py` replace lines 89–95 (the `CARD_W`/`TILE_W` block and its comment) with:

```python
# Source widths, since M5 (spec §3.3): a card cell IS 530x799 and a tile cell IS
# 709x531 on every sheet (`cell_box`), and emitting narrower than that is an
# upsample on a 2x display once the fit rule lets a tile reach 22rem. The old
# 360/440 were a payload choice for a 180px card; `deploy/RUNBOOK.md` §3.4
# records what the change costs on the wire.
CARD_W = 530
TILE_W = 709
# The achievement cards are LANDSCAPE on the table and stored SIDEWAYS on their
# three sheets — every numeral reads a quarter turn clockwise. Rotated
# counter-clockwise before emit, so the crop is a right-side-up 799x530 face and
# nothing downstream has to know how the sheet was scanned. PIL's `rotate` is
# counter-clockwise for a positive angle.
ACH_W = 799
ACH_ROTATE_CCW = 90
QUALITY = 82
```

In the `provenance` dict (line ~113) add two keys after `"tile_width": TILE_W,`:

```python
            "achievement_width": ACH_W,
            "achievement_rotation_ccw": ACH_ROTATE_CCW,
```

In the achievements loop (line ~264) replace the two lines inside `for (r, c), ty in cells.items():` with:

```python
            crop = sheet.crop(cell_box(sheet.size, cols, rows, r, c))
            face = crop.rotate(ACH_ROTATE_CCW, expand=True)
            achievements[ty] = em.emit(face, ACH_W, f"achievements/{ty}.webp")
```

- [ ] **Step 7: Regenerate, and regenerate the wire fixture**

Run: `python3 docs/tools/mkcardart.py && python3 docs/tools/mkcardart.py --check`
Expected: `wrote 132 images (... KiB)` then `art is current: manifest matches, 132 files present`.

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire/`
Expected: exactly one changed file, `web/src/wire/art_manifest.json`. If any other fixture moved, STOP — the server changed under you and that is not this task.

- [ ] **Step 8: Run every gate**

Run: `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings && cd web && npm test && npm run typecheck`
Expected: all green; vitest count is 231 + 2.

- [ ] **Step 9: LOOK at the achievements**

Run:
```bash
mkdir -p docs/temp/web-review && python3 - <<'EOF'
from PIL import Image
import glob, subprocess, os
S = "docs/temp/web-review"
files = sorted(glob.glob("crates/yokohama-web/assets/art/achievements/*.webp"))
ims = [Image.open(f).convert("RGB") for f in files]
w, h = ims[0].size
sheet = Image.new("RGB", (4 * w, 3 * h), (20, 20, 20))
for i, im in enumerate(ims):
    sheet.paste(im, ((i % 4) * w, (i // 4) * h))
sheet.save(f"{S}/achievements.png")
print(f"{S}/achievements.png", len(ims), ims[0].size)
EOF
```
Then `Read docs/temp/web-review/achievements.png`. Every numeral must read upright, the single-pawn silhouette at top-left, the three-pawn strip at bottom-right, the two star badges top-left (first) and top-right (later). If any face is on its side, the rotation direction is wrong: change `ACH_ROTATE_CCW` to `-90`, regenerate, look again. Do not proceed on a sheet you have not opened.

- [ ] **Step 10: Record the payload after**

Run: `du -sk crates/yokohama-web/assets/art | cut -f1`

Edit `ASSETS.md`'s row for `crates/yokohama-web/assets/art/` from `~3.0 MB` to the measured figure. Edit `deploy/RUNBOOK.md` §3.4: change the "Card art at `/art`" row to the new size and add one line under the table: *"Re-measured at M5 (W0): the crops are now cut at source width (709px tiles, 530px cards, 799px landscape achievements), which took the art from 3.0 MB to N MB across the same 132 files; the per-session figures above scale accordingly."* Edit `CLAUDE.md:68`'s "2.7 MiB of WebP" to the measured figure.

- [ ] **Step 11: Commit**

```bash
git add docs/tools/mkcardart.py crates/yokohama-web/src/art.rs crates/yokohama-web/assets/art web/src/wire/art_manifest.json web/src/art.test.ts ASSETS.md deploy/RUNBOOK.md CLAUDE.md
git commit -m "feat(web): W0 — the art re-cut at source width, and the achievements the right way up

The shipped crops were 440px tiles and 360px cards off 709px and 530px source
cells, so the fit rule's 22rem ceiling would have been a 1.6x upsample at 2x.
The three achievement sheets store their cards sideways; the generator now
turns them a quarter turn before emitting, so the face is a 799x530 landscape
card and the rail stops drawing it on its side."
```

---

### Task 2: W1 — the overlay geometry, in one file, drawn for the owner

**Files:**
- Create: `web/src/tile_rects.json`, `web/src/tileRects.ts`, `web/src/tileRects.test.ts`, `docs/tools/tilerects.py`

**Interfaces:**
- Produces: `Rect { left, top, width, height }` (fractions of the tile), `SITE_CARD`, `PIECES`, `ID_BADGE`, `TOKEN_SLOT`, `PRINTED`, `OVERLAYS`, `intersects(a, b)`, `inside(r)`, `pct(r)` → inline-style percentages. Consumed by Task 4.

- [ ] **Step 1: Write the failing test**

Create `web/src/tileRects.test.ts`:

```ts
// The overlay geometry: four rects an area tile draws its pieces in, and the
// four printed regions none of them may touch.

import { describe, expect, it } from "vitest";
import { ID_BADGE, inside, intersects, OVERLAYS, PIECES, pct, PRINTED, SITE_CARD, TOKEN_SLOT } from "./tileRects";

describe("the overlay geometry", () => {
  it("every overlay lies inside the tile", () => {
    for (const [name, r] of Object.entries(OVERLAYS)) {
      expect(inside(r), `${name} leaves the tile`).toBe(true);
    }
  });

  it("no overlay covers a printed rule", () => {
    for (const [name, r] of Object.entries(OVERLAYS)) {
      for (const [printed, p] of Object.entries(PRINTED)) {
        expect(intersects(r, p), `${name} covers the ${printed}`).toBe(false);
      }
    }
  });

  it("no two overlays cover each other", () => {
    const list = Object.entries(OVERLAYS);
    for (let i = 0; i < list.length; i++) {
      for (let j = i + 1; j < list.length; j++) {
        expect(intersects(list[i]![1], list[j]![1]), `${list[i]![0]} overlaps ${list[j]![0]}`).toBe(false);
      }
    }
  });

  // The site card is a 530x799 portrait card laid on its side, so its box on a
  // 709x531 tile must be 799:530 or the printed spaces stretch.
  it("the site card keeps the printed card's aspect on its side", () => {
    const boxAspect = (SITE_CARD.width * 709) / (SITE_CARD.height * 531);
    expect(Math.abs(boxAspect - 799 / 530)).toBeLessThan(0.02);
  });

  // Mutation proof for the cell above it: a pieces band nudged down onto the
  // 5-space IS an intersection, so the guard is not vacuous.
  it("would catch a band that drifts onto the 5-space", () => {
    expect(intersects({ ...PIECES, top: 0.5 }, TOKEN_SLOT)).toBe(true);
  });

  it("renders a rect as inline percentages", () => {
    expect(pct(ID_BADGE)).toEqual({ left: "50.00%", top: "3.00%", width: "8.50%", height: "7.00%" });
  });
});
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd web && npx vitest run src/tileRects.test.ts`
Expected: FAIL — cannot resolve `./tileRects`.

- [ ] **Step 3: Write the JSON and the module**

Create `web/src/tile_rects.json`:

```json
{
  "printed": {
    "token_slot": { "left": 0.816, "top": 0.588, "width": 0.125, "height": 0.133 },
    "ladder_band": { "left": 0, "top": 0.63, "width": 1, "height": 0.28 },
    "title_band": { "left": 0, "top": 0, "width": 0.5, "height": 0.14 },
    "icon_block": { "left": 0, "top": 0.14, "width": 0.5, "height": 0.44 }
  },
  "overlays": {
    "site_card": { "left": 0.6, "top": 0.04, "width": 0.37, "height": 0.328 },
    "pieces": { "left": 0.52, "top": 0.4, "width": 0.29, "height": 0.185 },
    "id_badge": { "left": 0.5, "top": 0.03, "width": 0.085, "height": 0.07 }
  }
}
```

Create `web/src/tileRects.ts`:

```ts
// Where the pieces sit on an area tile, in fractions of the tile's box.
//
// The numbers live in `tile_rects.json`, which is ALSO what
// `docs/tools/tilerects.py` draws onto every area art for the owner to look at
// (M5 spec §4.1, gate V0): one file read by the picture and by the code, so the
// sheet the owner approved and the rects the client draws cannot drift.
//
// The PRINTED regions are what an overlay must never cover. Two are measured —
// the 5-space (M3.5, `styles.css`'s `.token-slot`) and the ladder band (M3.5
// §2.2, y 0.630–0.910) — and two are read off the contact sheet in the spec's
// §2: the title band, whose longest title ("Employment Agency") ends at
// x≈0.49, and the icon blocks, which reach y≈0.57 on Church, Chinatown,
// Laboratory and Research Center.

import rects from "./tile_rects.json";

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export const PRINTED: Record<string, Rect> = rects.printed;
export const OVERLAYS: Record<string, Rect> = rects.overlays;
export const SITE_CARD: Rect = rects.overlays.site_card;
export const PIECES: Rect = rects.overlays.pieces;
export const ID_BADGE: Rect = rects.overlays.id_badge;
export const TOKEN_SLOT: Rect = rects.printed.token_slot;

/** Open intervals on both axes: a shared EDGE is not an overlap. */
export function intersects(a: Rect, b: Rect): boolean {
  return (
    a.left < b.left + b.width &&
    b.left < a.left + a.width &&
    a.top < b.top + b.height &&
    b.top < a.top + a.height
  );
}

export const inside = (r: Rect): boolean =>
  r.left >= 0 && r.top >= 0 && r.left + r.width <= 1 && r.top + r.height <= 1;

/** The rect as inline percentages, for an absolutely positioned overlay. */
export function pct(r: Rect): { left: string; top: string; width: string; height: string } {
  const p = (n: number) => `${(n * 100).toFixed(2)}%`;
  return { left: p(r.left), top: p(r.top), width: p(r.width), height: p(r.height) };
}
```

- [ ] **Step 4: Run the test to see it pass**

Run: `cd web && npx vitest run src/tileRects.test.ts`
Expected: PASS, 6 cells.

- [ ] **Step 5: Write the review script**

Create `docs/tools/tilerects.py`:

```python
#!/usr/bin/env python3
"""Draw the M5 tile overlay rects onto every area art, for a human to look at.

    python3 docs/tools/tilerects.py      # writes docs/temp/web-review/tile-rects.png

Reads `web/src/tile_rects.json` -- the SAME file the client draws from -- and
the thirteen non-Canal area crops, and writes one contact sheet: the printed
regions in red, the site card in magenta, the pieces band in green, the id
badge in cyan. Nothing here asserts anything; the assertion is the owner
looking at it (M5 spec §4.1, gate V0). The output goes under the gitignored
`docs/temp/` because a picture of the art is the art (`ASSETS.md`).
"""
from __future__ import annotations

import json
import pathlib
import sys

try:
    from PIL import Image, ImageDraw
except ImportError:  # pragma: no cover
    sys.exit("this script needs Pillow:  python3 -m pip install pillow")

ROOT = pathlib.Path(__file__).resolve().parents[2]
ART = ROOT / "crates" / "yokohama-web" / "assets" / "art"
OUT = ROOT / "docs" / "temp" / "web-review" / "tile-rects.png"
RECTS = json.loads((ROOT / "web" / "src" / "tile_rects.json").read_text())
PRINTED = (255, 64, 64)
OVERLAY = {"site_card": (255, 0, 255), "pieces": (0, 255, 0), "id_badge": (0, 220, 255)}


def box(draw, r, w, h, colour, width):
    draw.rectangle(
        [r["left"] * w, r["top"] * h, (r["left"] + r["width"]) * w, (r["top"] + r["height"]) * h],
        outline=colour,
        width=width,
    )


def main() -> None:
    manifest = json.loads((ART / "manifest.json").read_text())
    faces = [(k, ART / e["file"]) for k, e in sorted(manifest["areas"].items()) if k != "Canal"]
    tiles = []
    for name, path in faces:
        im = Image.open(path).convert("RGB")
        w, h = im.size
        d = ImageDraw.Draw(im)
        for r in RECTS["printed"].values():
            box(d, r, w, h, PRINTED, 2)
        for key, r in RECTS["overlays"].items():
            box(d, r, w, h, OVERLAY[key], 4)
        d.text((8, h - 20), name, fill=(255, 255, 255))
        tiles.append(im)
    w, h = tiles[0].size
    cols, rows = 4, (len(tiles) + 3) // 4
    sheet = Image.new("RGB", (cols * w, rows * h), (20, 20, 20))
    for i, im in enumerate(tiles):
        sheet.paste(im, ((i % cols) * w, (i // cols) * h))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(OUT)
    print(f"wrote {OUT.relative_to(ROOT)} ({len(tiles)} arts, {sheet.size[0]}x{sheet.size[1]})")


if __name__ == "__main__":
    main()
```

- [ ] **Step 6: Draw it, look at it, send it (gate V0)**

Run: `python3 docs/tools/tilerects.py`, then `Read docs/temp/web-review/tile-rects.png`. On every one of the thirteen arts: the magenta box must hold only illustration (no icon, no numeral, no title), the green band must sit clear of the icon blocks on Church, Chinatown, Laboratory and Research Center and above the red 5-space frame, and the cyan badge must not touch the printed title. If a rect touches printed content on ANY art, move it in `tile_rects.json` for all of them, re-run the unit test, re-draw.

Then `SendUserFile` the sheet with the caption *"Gate V0 — the overlay rects on all 13 arts: magenta = site card, green = pieces, cyan = id badge, red = printed regions. Do these sit where you'd put the pieces?"* and **STOP until the owner answers.** Record the answer in the commit message.

- [ ] **Step 7: Gate and commit**

Run: `cd web && npm test && npm run typecheck`

```bash
git add web/src/tile_rects.json web/src/tileRects.ts web/src/tileRects.test.ts docs/tools/tilerects.py
git commit -m "feat(web): W1 — the tile overlay rects, one JSON for the code and the picture

Four rects and four printed regions, with the unit cells that keep every
overlay inside the tile and off the ladder, the 5-space, the title and the
icon blocks. The review script draws them on all thirteen arts; the owner
looked at that sheet (gate V0) before these numbers were committed."
```

---

### Task 3: W4a — `SiteFace` and the pieces

**Files:**
- Modify: `web/src/Cards.tsx` (add `SiteFace`), `web/src/Surfaces.tsx:780-826` (`SiteCard` uses it)
- Create: `web/src/Pieces.tsx`, `web/src/Pieces.test.tsx`

**Interfaces:**
- Produces: `SiteFace({ art, site: SiteModel, colors })` — the printed site card with built marks, `div.site-face`; `Pieces({ presidents, assistants, hasStation, colors, art })` — `div.pieces[role=img][aria-label]` or `null`; `piecesWords(presidents, assistants, hasStation, colors): string`. Consumed by Tasks 4, 5, 11.

- [ ] **Step 1: Write the failing pieces test**

Create `web/src/Pieces.test.tsx`:

```tsx
// The pieces standing on a tile, drawn as pieces and named in words.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Pieces, piecesWords } from "./Pieces";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

describe("the pieces on a tile", () => {
  it("say who stands there, in the words a screen reader gets", () => {
    expect(piecesWords([0], [[0, 0], [1, 2]], false, COLORS)).toBe("Red president · 2 Blue assistants");
    expect(piecesWords([], [[0, 1], [1, 0]], true, COLORS)).toBe("1 Red assistant · Station");
    expect(piecesWords([], [[0, 0], [1, 0]], false, COLORS)).toBe("");
  });

  it("draw a pawn per president and a cube per assistant up to three", () => {
    const { container } = render(
      <Pieces presidents={[0]} assistants={[[0, 0], [1, 3]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelectorAll(".piece.president")).toHaveLength(1);
    expect(container.querySelectorAll(".piece.cube")).toHaveLength(3);
    expect(container.querySelector(".piece-n")).toBeNull();
    expect(container.querySelector(".pieces")?.getAttribute("aria-label")).toBe(
      "Red president · 3 Blue assistants",
    );
  });

  it("draw one cube and a count from four", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 4], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelectorAll(".piece.cube")).toHaveLength(1);
    expect(container.querySelector(".piece-n")?.textContent).toBe("4");
  });

  // An empty tile draws nothing, exactly as the table does — no "no pawns".
  it("draw nothing at all on an empty tile", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 0], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelector(".pieces")).toBeNull();
  });

  it("colour a piece by its seat", () => {
    const { container } = render(
      <Pieces presidents={[1]} assistants={[[0, 0], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelector(".piece-group")?.className).toMatch(/seat-blue/);
  });

  it("draw the Station token's art where there is art, and a chip where there is none", () => {
    const withArt = render(
      <Pieces presidents={[]} assistants={[]} hasStation colors={COLORS} art={wire.artManifest} />,
    );
    expect(withArt.container.querySelector("img.piece.station")).not.toBeNull();
    const without = render(<Pieces presidents={[]} assistants={[]} hasStation colors={COLORS} art={null} />);
    expect(without.container.querySelector(".chip.station")?.textContent).toBe("St");
  });
});
```

- [ ] **Step 2: Run it to see it fail**

Run: `cd web && npx vitest run src/Pieces.test.tsx`
Expected: FAIL — cannot resolve `./Pieces`.

- [ ] **Step 3: Write `Pieces.tsx`**

```tsx
// The pieces standing on a tile, drawn as pieces.
//
// A president is a pawn silhouette and an assistant is a cube — the shape the
// TTS mod gives it (`BlockSquare`), to be confirmed against the box (M5 spec
// §3.2.1). Both are inline SVG in the seat's colour via `currentColor`, so a
// fifth seat colour is one CSS rule and not a new picture. Everything the old
// caption said in words rides on the group's accessible name.

import type { ArtManifest, PlayerColor, PlayerId } from "./types";
import { seatClass } from "./Cards";
import { tokenArt } from "./art";

export function PresidentIcon() {
  return (
    <svg viewBox="0 0 20 28" className="piece president" aria-hidden="true">
      <path
        d="M10 1a4 4 0 0 1 2.6 7.1c2 1 3.2 2.9 3.2 5.2 0 1.6-.6 3-1.6 4.1l2.8 6.6H3l2.8-6.6A6.2 6.2 0 0 1 4.2 13.3c0-2.3 1.2-4.2 3.2-5.2A4 4 0 0 1 10 1z"
        fill="currentColor"
        stroke="#0b0e12"
        strokeWidth="1.2"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function CubeIcon() {
  return (
    <svg viewBox="0 0 20 20" className="piece cube" aria-hidden="true">
      <path d="M10 2 18 6v8l-8 4-8-4V6z" fill="currentColor" stroke="#0b0e12" strokeWidth="1.2" strokeLinejoin="round" />
      <path d="M2 6l8 4 8-4M10 10v8" fill="none" stroke="#0b0e12" strokeWidth="1" opacity=".55" />
    </svg>
  );
}

/** The words: `Red president · 2 Blue assistants · Station`, or `""` for nothing. */
export function piecesWords(
  presidents: PlayerId[],
  assistants: [PlayerId, number][],
  hasStation: boolean,
  colors: PlayerColor[],
): string {
  const seat = (s: PlayerId) => colors[s] ?? `seat ${s}`;
  return [
    ...presidents.map((s) => `${seat(s)} president`),
    ...assistants.filter(([, n]) => n > 0).map(([s, n]) => `${n} ${seat(s)} assistant${n === 1 ? "" : "s"}`),
    ...(hasStation ? ["Station"] : []),
  ].join(" · ");
}

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
  return (
    <div className="pieces" role="img" aria-label={words}>
      {presidents.map((seat) => (
        <span key={`p${seat}`} className={`piece-group ${seatClass(colors[seat])}`}>
          <PresidentIcon />
        </span>
      ))}
      {assistants
        .filter(([, n]) => n > 0)
        .map(([seat, n]) => (
          <span key={`a${seat}`} className={`piece-group ${seatClass(colors[seat])}`}>
            {Array.from({ length: n > 3 ? 1 : n }, (_, i) => (
              <CubeIcon key={i} />
            ))}
            {n > 3 && <span className="piece-n">{n}</span>}
          </span>
        ))}
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

- [ ] **Step 4: Run the pieces test to see it pass**

Run: `cd web && npx vitest run src/Pieces.test.tsx`
Expected: PASS, 6 cells.

- [ ] **Step 5: Extract `SiteFace` into `Cards.tsx`**

Add to `web/src/Cards.tsx` (after `SiteCardView`), and add `SiteModel` to its type import:

```tsx
/** The printed building-site card with what is BUILT drawn on it.
 *
 *  The overlay rects are MEASURED (`styles.css`'s `.site-space.slot-*`): each
 *  of the 24 site arts minus its own 9px box blur, averaged, so the printed
 *  rule survives and every card's own content cancels. An unbuilt space draws
 *  NOTHING, so the printed card reads exactly as it does on the table. Shared by
 *  the building-sites strip and, since M5, by every tile. */
export function SiteFace({
  art,
  site,
  colors,
}: {
  art: ArtManifest | null;
  site: SiteModel;
  colors: PlayerColor[];
}) {
  const face = siteArt(art, site.index);
  const th = site.trading_house_owner;
  return (
    <div className="site-face">
      {face ? (
        <img src={face} alt="" loading="lazy" />
      ) : (
        <div className="card-blank">{site.index == null ? "site" : `Site ${site.index}`}</div>
      )}
      {site.slots.map((slot, i) => (
        <span
          key={i}
          className={`site-space slot-${i} ${
            slot.built_by != null ? `built ${seatClass(colors[slot.built_by])}` : ""
          }`}
        >
          {slot.built_by != null && <span className="built-mark">&#8962;</span>}
        </span>
      ))}
      <span className={`site-trading-house ${th != null ? `built ${seatClass(colors[th])}` : ""}`}>
        {th != null && <span className="built-mark">&#8962;</span>}
      </span>
    </div>
  );
}
```

Then in `web/src/Surfaces.tsx`, replace the body of `SiteCard` (lines 780–826) so it reads:

```tsx
function SiteCard({ card, art, colors }: { card: AreaCard; art: ArtManifest | null; colors: PlayerColor[] }) {
  const site = card.tile.site!;
  const id = `A${String(card.tile.area).padStart(2, "0")}`;
  return (
    <figure className="site-card">
      <SiteFace art={art} site={site} colors={colors} />
      <figcaption>
        <span className="tile-id">{id}</span> <b>Site {site.index}</b>
        <span className="muted"> &middot; trading house &#9733;{site.trading_house_vp}</span>
      </figcaption>
    </figure>
  );
}
```

and add `SiteFace` to the `./Cards` import; drop the now-unused `siteArt` import from `./art` if nothing else in the file uses it (`grep -n siteArt web/src/Surfaces.tsx`).

- [ ] **Step 6: Run the gate**

Run: `cd web && npm test && npm run typecheck`
Expected: green; the two building-sites cells in `Surfaces.test.tsx:329-398` still pass unchanged (same classes, same marks).

- [ ] **Step 7: Commit**

```bash
git add web/src/Pieces.tsx web/src/Pieces.test.tsx web/src/Cards.tsx web/src/Surfaces.tsx
git commit -m "feat(web): W4a — the pieces as pieces, and the site face shared

A president is a pawn and an assistant a cube, in the seat's colour, with the
old caption's words on the group's accessible name. The printed site card with
its built marks comes out of the building-sites drawer into Cards.tsx so the
tile can draw the same thing."
```

---

### Task 4: W4b — the tile is the art

**Files:**
- Modify: `web/src/Board.tsx` (delete `SiteSlots` `:110-151`, rewrite `Tile` `:153-289`, re-word `Legend` `:292-309`)
- Modify: `web/src/styles.css` (`:161-241` tile block; delete `:244-251` `.tile-head` rules, `:264-277` `.site-strip` rules; keep `.tile-row`, `.token-row`, `.five-power`, `.pawn*` — `grep -n` shows Dashboard and the Areas surface still use them)
- Modify: `web/src/Board.test.tsx`

**Interfaces:**
- Consumes: `Pieces`, `SiteFace` (Task 3); `SITE_CARD`, `PIECES`, `ID_BADGE`, `pct` (Task 2).
- Produces: a tile whose DOM is `.tile > .tile-art-box > (img.tile-art | .tile-art-blank) + .token-slot? + .site-on-tile? + .pieces-band? + .tile-id`, followed by `.sr-only` spans carrying the id+name, the 5-Power words, `Site n`, and the reach words. Task 5 adds handlers to the same element.

- [ ] **Step 1: Write the failing tests**

In `web/src/Board.test.tsx`, add a helper after `show` and a new `describe` block at the end; also change the two token cells' tile lookup to the helper (the tile's text no longer starts with the id — a built mark or a piece count can precede the badge):

```tsx
import { SITE_CARD } from "./tileRects";

/** The tile whose id badge reads `id`. */
function tileOf(container: HTMLElement, id: string): HTMLElement {
  const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find(
    (t) => t.querySelector(".tile-id")?.textContent === id,
  );
  if (!tile) throw new Error(`no tile ${id}`);
  return tile;
}
```

In both cells of `describe("the 5-Power token sits in its printed slot")` replace

```tsx
    const el = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) =>
      t.textContent?.startsWith(id),
    )!;
```

with `const el = tileOf(container, id);` (the second cell reads `screen.getAllByText("5-Power taken")` — that text now lives in an `.sr-only` span and still resolves).

Append:

```tsx
describe("the tile is the art, and the pieces sit on it (M5)", () => {
  const withPieces = (): { state: ClientState; id: string } => {
    const state = structuredClone(wire.options) as ClientState;
    const t = state.frame.board.tiles.find((tile) => tile.site !== null)!;
    t.presidents = [0];
    t.assistants = [[0, 0], [1, 2]];
    t.has_station = true;
    t.site!.slots[0]!.built_by = 1;
    t.site!.trading_house_owner = 0;
    return { state, id: `A${String(t.area).padStart(2, "0")}` };
  };

  it("draws no caption under the art", () => {
    const { container } = show(wire.options);
    expect(container.querySelector(".tile-body")).toBeNull();
    expect(container.querySelector(".site-strip")).toBeNull();
    expect(screen.queryByText("no pawns")).toBeNull();
  });

  it("puts the site card on the tile, built spaces in the builder's colour", () => {
    const { state, id } = withPieces();
    const tile = tileOf(show(state).container, id);
    const card = tile.querySelector(".site-on-tile");
    expect(card, "no site card on the tile").not.toBeNull();
    expect(card!.getAttribute("style")).toContain(`left: ${(SITE_CARD.left * 100).toFixed(2)}%`);
    expect(card!.querySelector(".site-space.slot-0.built")?.className).toMatch(/seat-blue/);
    expect(card!.querySelector(".site-trading-house.built")?.className).toMatch(/seat-red/);
  });

  it("names the pieces for a screen reader and draws them as pieces", () => {
    const { state, id } = withPieces();
    const tile = tileOf(show(state).container, id);
    expect(tile.querySelector(".pieces")?.getAttribute("aria-label")).toBe(
      "Red president · 2 Blue assistants · Station",
    );
    expect(tile.querySelectorAll(".piece.cube")).toHaveLength(2);
  });

  it("keeps the id, the site and the token in words the caption used to carry", () => {
    const { container } = show(wire.options);
    const t = wire.options.frame.board.tiles.find((tile) => tile.site?.index != null)!;
    const id = `A${String(t.area).padStart(2, "0")}`;
    const words = tileOf(container, id).textContent ?? "";
    expect(words).toContain(id);
    expect(words).toContain(`Site ${t.site!.index}`);
    expect(words).toMatch(/5-Power/);
  });

  it("still draws a name when there is no art, and no broken image", () => {
    const { container } = render(
      <Board board={wire.options.frame.board} art={null} labels={wire.labels} colors={COLORS} selected={[]} busy={false} />,
    );
    expect(container.querySelector(".tile-art-blank .tile-name")?.textContent).toBeTruthy();
    expect(container.querySelectorAll("img")).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run src/Board.test.tsx`
Expected: the five new cells FAIL (`.tile-body` present, no `.site-on-tile`, no `.pieces`).

- [ ] **Step 3: Rewrite `Tile` and `Legend` in `Board.tsx`**

Replace the imports at the top of `web/src/Board.tsx` with:

```tsx
import type { CSSProperties } from "react";
import type { ArtManifest, BoardModel, Labels, PlayerColor, TileModel } from "./types";
import { areaArt, fivePowerArt, fivePowerWords } from "./art";
import { SiteFace } from "./Cards";
import { Pieces } from "./Pieces";
import { ID_BADGE, PIECES, pct, SITE_CARD } from "./tileRects";
```

Delete `SiteSlots` (`:110-151`) entirely. Replace `Tile` with:

```tsx
function Tile({
  tile,
  art,
  labels,
  colors,
  onClick,
  isSelected,
  busy,
}: {
  tile: TileModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  onClick?: (area: number) => void;
  isSelected: boolean;
  busy: boolean;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const isCanal = tile.area_type === "Canal";
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const face = areaArt(art, tile.area_type);
  const tokenSrc = fivePowerArt(art, tile.token);

  // **The tile is the art, and the pieces sit on it where the physical ones do**
  // (M5 spec §3.2, owner ruling R4). No caption: the printed title IS the name,
  // the site card lies rotated over the top-right quarter as in the owner's
  // photograph, the pieces stand in the band below it, the 5-Power token sits
  // in its printed slot. Every rect comes from `tile_rects.json`, never typed
  // here. What the caption used to say in words now rides in `.sr-only` spans
  // and on the pieces' accessible name, so nothing a screen reader had is gone.
  const body = (
    <>
      <div className="tile-art-box">
        {face ? (
          <img className="tile-art" src={face} alt="" loading="lazy" />
        ) : (
          <div className="tile-art-blank">
            <span className="tile-name">{name}</span>
          </div>
        )}
        {tokenSrc && (
          <span className="token-slot">
            <img src={tokenSrc} alt="" loading="lazy" />
          </span>
        )}
        {tile.site && (
          <div className="site-on-tile" style={pct(SITE_CARD)}>
            <SiteFace art={art} site={tile.site} colors={colors} />
          </div>
        )}
        {!isCanal && (
          <div className="pieces-band" style={pct(PIECES)}>
            <Pieces
              presidents={tile.presidents}
              assistants={tile.assistants}
              hasStation={tile.has_station}
              colors={colors}
              art={art}
            />
          </div>
        )}
        <span className="tile-id" style={pct(ID_BADGE)}>
          {id}
        </span>
      </div>
      <span className="sr-only">
        {id} {name}
      </span>
      {isCanal ? (
        <span className="sr-only">crossing only · 1 yen to the bank</span>
      ) : (
        <>
          <span className="sr-only">
            {tile.token ? `5-Power ${fivePowerWords(tile.token)}` : "5-Power taken"}
          </span>
          {tile.site?.index != null && <span className="sr-only">Site {tile.site.index}</span>}
          {tile.reachable && <span className="sr-only">a Movement destination the engine offers</span>}
        </>
      )}
    </>
  );

  const className = [
    "tile",
    tile.reachable ? "reachable" : "",
    isCanal ? "canal" : "",
    isSelected ? "selected" : "",
    face ? "has-art" : "",
  ]
    .filter(Boolean)
    .join(" ");

  if (!onClick || isCanal) return <div className={className}>{body}</div>;
  return (
    <button
      type="button"
      className={className}
      disabled={busy}
      aria-pressed={isSelected}
      aria-label={`${id} ${name}${isSelected ? " — selected" : ""}`}
      onClick={() => onClick(tile.area)}
    >
      {body}
    </button>
  );
}
```

Replace `Legend` with:

```tsx
/** What the marks mean. Worded for the pieces, since the tile now draws them
 *  as pieces rather than as the terminal's glyph alphabet. */
function Legend() {
  return (
    <details className="legend">
      <summary>marks</summary>
      <div>
        a pawn is a president and a cube an assistant, each in its owner&rsquo;s colour (a number
        beside a cube counts a stack) &middot; the small card on a tile is its building site, with a
        built space in its builder&rsquo;s colour &middot; the token in the printed 5-space is the
        5-Power bonus, and an empty frame means it is taken
      </div>
      <div>
        a green border is a Movement destination the engine offers &middot; a blue border is a tile
        you have selected &middot; the id badge (A00…) is what every prompt names
      </div>
    </details>
  );
}
```

- [ ] **Step 4: Restyle the tile in `styles.css`**

Replace the `.tile-art-box` rule (`:187-192`) so it no longer draws a bottom border:

```css
.tile-art-box {
  position: relative;
  aspect-ratio: var(--tile-aspect);
  overflow: hidden;
}
```

Delete the `.tile-body` rule (`:234-238`) and its comment, the `.tile-head`, `.tile-id`, `.tile-head .reach` rules (`:244-245`, `:251`) and the `.site-strip` block (`:264-277`). Keep `.tile-name` (the no-art fallback), `.tile-row`, `.tile-row.muted`, `.token-row`, `.five-power`, `.pawn*`. Add, after `.tile.canal`:

```css
/* ---- M5: the tile is the art ------------------------------------------- */
/* The rects are inline (`tile_rects.json` via `pct()`), never here: one source. */
.tile-art-blank {
  display: grid; place-items: center; height: 100%;
  background: var(--bg-raised); color: var(--fg-dim);
}
.tile-id {
  position: absolute; z-index: 3;
  display: grid; place-items: center;
  font-size: 8.5px; letter-spacing: 0.04em; color: var(--fg);
  background: rgb(9 11 14 / 0.72); border-radius: 2px;
}
/* The site card: a landscape box holding the PORTRAIT face turned a quarter
 * turn. The inner face's width is the box's height (530/799 of its width), it is
 * centred, and the rotation is what lays it flat — so the measured
 * `.site-space` rects inside it turn with it and a built mark lands on the
 * printed space it names. */
.site-on-tile { position: absolute; z-index: 1; }
.site-on-tile > .site-face {
  position: absolute; left: 50%; top: 50%;
  width: calc(100% * 530 / 799);
  aspect-ratio: 360 / 543;
  transform: translate(-50%, -50%) rotate(-90deg);
  border: 1px solid rgb(0 0 0 / 0.5); border-radius: 2px;
  box-shadow: 0 1px 3px rgb(0 0 0 / 0.5); background: var(--bg-sunken);
}
.pieces-band { position: absolute; z-index: 2; display: flex; align-items: center; }
.pieces { display: flex; flex-wrap: wrap; align-items: flex-end; gap: 3%; width: 100%; height: 100%; }
.piece-group { display: inline-flex; align-items: flex-end; gap: 1px; height: 100%; }
.piece { height: 100%; width: auto; filter: drop-shadow(0 1px 1px rgb(0 0 0 / 0.6)); }
.piece.cube { height: 72%; }
.piece.station { height: 80%; border-radius: 2px; }
.piece-n { font-size: 9px; font-weight: 700; color: #fff; text-shadow: 0 1px 2px #000; line-height: 1; }
/* Visually hidden, present to assistive technology. */
.sr-only {
  position: absolute; width: 1px; height: 1px; overflow: hidden;
  clip: rect(0 0 0 0); white-space: nowrap; border: 0; padding: 0; margin: -1px;
}
```

- [ ] **Step 5: Run the gate**

Run: `cd web && npm test && npm run typecheck`
Expected: green. `App.test.tsx`'s "a server with no art still renders the game" passes (`SiteFace` and `Pieces` draw no `<img>` without art; `.card-blank` count > 0).

- [ ] **Step 6: Look at it in a browser**

Run: `cd web && npm run build && cd .. && YOKOHAMA_WEB_PORT=8491 YOKOHAMA_WEB_DIR=docs/temp/web-review/games ./target/release/yokohama-web` (build the release binary first if `target/release/yokohama-web` is stale: `cargo build --release -p yokohama-web`). Open `http://127.0.0.1:8491/` in the DevTools MCP browser at 1710×869, deal, start, and screenshot to `docs/temp/web-review/t4-tile.png`; `Read` it. The site card must be right-side-up on its side with the trading-house band outward; the id badge must not touch the printed title. Stop the server afterwards (`pkill -f target/release/yokohama-web`).

- [ ] **Step 7: Commit**

```bash
git add web/src/Board.tsx web/src/Board.test.tsx web/src/styles.css
git commit -m "feat(web): W4b — the tile is the art; the site card and the pieces sit on it

Both caption rows go. The building-site card lies a quarter turn over the
top-right quarter with its built marks turning with it, the pieces stand in
the band below it, the id badge sits right of the printed title, and every
word the caption carried moves to .sr-only spans and the pieces' accessible
name. Rects from tile_rects.json, never typed in the stylesheet."
```

---

### Task 5: W4c — the popover

**Files:**
- Create: `web/src/TilePopover.tsx`, `web/src/TilePopover.test.tsx`
- Modify: `web/src/Board.tsx` (hover/focus state, handlers on the tile, `tabIndex` on inert tiles), `web/src/Board.test.tsx`, `web/src/styles.css`

**Interfaces:**
- Produces: `popoverPosition(anchor, viewport, size?) → { left, top }`; `TilePopover({ tile, art, labels, colors, anchor })` → `div.tile-pop[role=tooltip][id=pop-A00]`. `Board` gains no new props.

- [ ] **Step 1: Write the failing tests**

Create `web/src/TilePopover.test.tsx`:

```tsx
// The popover: a tile at full size on hover or focus, never modal, never focused.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Board } from "./Board";
import { popoverPosition } from "./TilePopover";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

describe("where the popover goes", () => {
  it("sits right of the tile when there is room, left when there is not", () => {
    const viewport = { width: 1710, height: 869 };
    expect(popoverPosition({ left: 100, top: 200, right: 340, bottom: 380 }, viewport)).toEqual({ left: 348, top: 200 });
    expect(popoverPosition({ left: 1400, top: 200, right: 1640, bottom: 380 }, viewport)).toEqual({ left: 1024, top: 200 });
  });
  it("stays inside the viewport at the bottom", () => {
    expect(popoverPosition({ left: 100, top: 800, right: 340, bottom: 980 }, { width: 1710, height: 869 }).top).toBe(869 - 420 - 8);
  });
});

describe("the popover on a tile", () => {
  const first = wire.options.frame.board.tiles.find((t) => t.site !== null)!;
  const id = `A${String(first.area).padStart(2, "0")}`;
  const show = () =>
    render(
      <Board board={wire.options.frame.board} art={wire.artManifest} labels={wire.labels} colors={COLORS} selected={[]} busy={false} />,
    );

  it("opens on hover and closes when the pointer leaves", async () => {
    const user = userEvent.setup();
    const { container } = show();
    const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) => t.querySelector(".tile-id")?.textContent === id)!;
    expect(screen.queryByRole("tooltip")).toBeNull();
    await user.hover(tile);
    const pop = screen.getByRole("tooltip");
    expect(pop.id).toBe(`pop-${id}`);
    expect(tile.getAttribute("aria-describedby")).toBe(`pop-${id}`);
    expect(pop.querySelector(".site-face")).not.toBeNull();
    expect(pop.textContent).toContain(`Site ${first.site!.index}`);
    await user.unhover(tile);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("opens on focus, closes on Escape, and is not in the tab order itself", async () => {
    const user = userEvent.setup();
    const { container } = show();
    const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) => t.querySelector(".tile-id")?.textContent === id)!;
    tile.focus();
    const pop = screen.getByRole("tooltip");
    expect(pop.getAttribute("tabindex")).toBeNull();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  // An inert tile (no click handler, or the Canal) is still inspectable by
  // keyboard: it can take focus, and focusing it does not make it a button.
  it("an inert tile can be focused for its popover and is still not a button", () => {
    const { container } = show();
    const tile = container.querySelector<HTMLElement>(".tile")!;
    expect(tile.tagName).toBe("DIV");
    expect(tile.getAttribute("tabindex")).toBe("0");
  });
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run src/TilePopover.test.tsx`
Expected: FAIL — cannot resolve `./TilePopover`.

- [ ] **Step 3: Write `TilePopover.tsx`**

```tsx
// A tile at full size, on hover or keyboard focus.
//
// At the 10rem floor the site card on a tile is ~70px wide, below what its
// printed values need (M5 spec §3.2.2). This is the Areas drawer's row for ONE
// tile brought to the tile: the art at 22rem so the printed ladder is legible,
// the site card at 12rem, and the pieces in words. It is `role="tooltip"`,
// never takes focus, never sits in the tab order, and `pointer-events: none`
// so leaving the tile closes it. Positioned from the tile's rect in JS rather
// than with CSS anchor positioning, which Safari does not have.

import type { ArtManifest, Labels, PlayerColor, TileModel } from "./types";
import { areaArt, fivePowerWords } from "./art";
import { SiteFace } from "./Cards";
import { piecesWords } from "./Pieces";

export interface Anchor {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

/** Right of the tile when there is room, else left; top-aligned, kept inside. */
export function popoverPosition(
  anchor: Anchor,
  viewport: { width: number; height: number },
  size = { width: 368, height: 420 },
): { left: number; top: number } {
  const gap = 8;
  const left =
    anchor.right + gap + size.width <= viewport.width
      ? anchor.right + gap
      : Math.max(gap, anchor.left - gap - size.width);
  const top = Math.max(gap, Math.min(anchor.top, viewport.height - size.height - gap));
  return { left, top };
}

export function TilePopover({
  tile,
  art,
  labels,
  colors,
  anchor,
}: {
  tile: TileModel;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  anchor: Anchor;
}) {
  const name = labels?.area_types[tile.area_type] ?? tile.area_type;
  const id = `A${String(tile.area).padStart(2, "0")}`;
  const face = areaArt(art, tile.area_type);
  const viewport =
    typeof window === "undefined"
      ? { width: 1440, height: 900 }
      : { width: window.innerWidth, height: window.innerHeight };
  const pos = popoverPosition(anchor, viewport);
  const pieces = piecesWords(tile.presidents, tile.assistants, tile.has_station, colors);
  return (
    <div className="tile-pop" role="tooltip" id={`pop-${id}`} style={{ left: pos.left, top: pos.top }}>
      <div className="tile-pop-head">
        <span className="tile-pop-id">{id}</span> <b>{name}</b>
      </div>
      {face && <img className="tile-pop-art" src={face} alt="" />}
      <div className="tile-pop-row">
        {tile.site && <SiteFace art={art} site={tile.site} colors={colors} />}
        <div className="tile-pop-words">
          <div>{pieces || "no pieces here"}</div>
          {tile.area_type !== "Canal" && (
            <div>{tile.token ? `5-Power: ${fivePowerWords(tile.token)}` : "5-Power taken"}</div>
          )}
          {tile.site?.index != null && (
            <div>
              Site {tile.site.index} &middot; trading house &#9733;{tile.site.trading_house_vp}
            </div>
          )}
          {tile.reachable && <div className="reachable-words">a Movement destination the engine offers</div>}
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 4: Wire it into `Board.tsx`**

Add imports: `import { useState } from "react";` and `import { TilePopover, type Anchor } from "./TilePopover";`.

In `Board`, before the `return`:

```tsx
  // The popover's whole state: which tile, and where it is. One at a time.
  const [pop, setPop] = useState<{ area: number; anchor: Anchor } | null>(null);
  const show = (area: number, el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    setPop({ area, anchor: { left: r.left, top: r.top, right: r.right, bottom: r.bottom } });
  };
  const hide = () => setPop(null);
  const popTile = pop ? board.tiles.find((t) => t.area === pop.area) : undefined;
```

Pass `onShow={show}` and `onHide={hide}` and `popOpen={pop?.area === t.area}` to each `<Tile>`, and render after the rows (before `<Legend />`):

```tsx
      {pop && popTile && (
        <TilePopover tile={popTile} art={art} labels={labels} colors={colors} anchor={pop.anchor} />
      )}
```

In `Tile`, add the three props to its signature (`onShow: (area: number, el: HTMLElement) => void; onHide: () => void; popOpen: boolean;`) and this object above the two returns:

```tsx
  const hoverable = {
    onMouseEnter: (e: React.MouseEvent<HTMLElement>) => onShow(tile.area, e.currentTarget),
    onMouseLeave: onHide,
    onFocus: (e: React.FocusEvent<HTMLElement>) => onShow(tile.area, e.currentTarget),
    onBlur: onHide,
    onKeyDown: (e: React.KeyboardEvent<HTMLElement>) => {
      if (e.key === "Escape") onHide();
    },
    ...(popOpen ? { "aria-describedby": `pop-${id}` } : {}),
  };
```

and spread it onto both elements: `<div className={className} tabIndex={0} {...hoverable}>` and `<button … {...hoverable}>`. Add `import type React from "react";` if the event types need it.

- [ ] **Step 5: Style it**

Append to `styles.css`:

```css
/* ---- M5: the tile popover ---------------------------------------------- */
.tile-pop {
  position: fixed; z-index: 40; width: 22rem; box-sizing: border-box;
  background: var(--bg-raised); border: 1px solid var(--line-bright); border-radius: 6px;
  box-shadow: 0 12px 32px rgb(0 0 0 / 0.5); padding: 0.5rem;
  pointer-events: none;
}
.tile-pop-head { display: flex; align-items: baseline; gap: 0.4rem; font-family: var(--display); font-size: 1rem; margin-bottom: 0.4rem; }
.tile-pop-id { color: var(--fg-faint); font-size: 10px; font-family: var(--mono); }
.tile-pop-art { display: block; width: 100%; aspect-ratio: var(--tile-aspect); object-fit: cover; border-radius: 3px; }
.tile-pop-row { display: flex; gap: 0.6rem; margin-top: 0.5rem; align-items: flex-start; }
.tile-pop-row .site-face { width: 12rem; flex: 0 0 12rem; }
.tile-pop-words { font-size: 11.5px; color: var(--fg-dim); display: flex; flex-direction: column; gap: 0.25rem; }
.tile-pop-words .reachable-words { color: var(--reachable); }
```

- [ ] **Step 6: Run the gate**

Run: `cd web && npm test && npm run typecheck`
Expected: green, including the earlier Board cells (a `tabIndex` on the Canal `div` does not make it a button; `queryByRole("button", …)` still finds none for it).

- [ ] **Step 7: Commit**

```bash
git add web/src/TilePopover.tsx web/src/TilePopover.test.tsx web/src/Board.tsx web/src/styles.css
git commit -m "feat(web): W4c — a tile's popover: the art and its site card at full size

Hover or focus a tile and its art draws at 22rem with the site card at 12rem
and the pieces in words; role=tooltip, never focused, closed by Escape or
leaving. Inert tiles take focus so a keyboard user can inspect them too."
```

---

### Task 6: Gate V1 — the tile, seen

**Files:**
- Create: `web/e2e/gates.spec.ts`
- Modify: `web/playwright.config.ts:113` (`testIgnore`)

- [ ] **Step 1: Write the gate shots spec**

Create `web/e2e/gates.spec.ts`:

```ts
// The visual gates (M5 spec §4.1): screenshots for a human, not assertions.
//
// **A generator, not a test**, in the shape of `shots.spec.ts`: it drives the
// shipped client and writes pictures for the owner to look at. Opt-in via
// `YOKOHAMA_WEB_GATES=1` (the config drops this file otherwise), and the output
// goes under the gitignored `docs/temp/web-review/gates/` because a picture of
// the art is the art. The few `expect`s stop a blank frame from being sent.
//
//     cd web && npm run build && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts

import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const OUT = "../docs/temp/web-review/gates";
const VIEWPORTS = [
  [1440, 900],
  [1710, 869],
  [2560, 1440],
] as const;

async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

const raf = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

for (const players of [2, 4] as const) {
  test(`V1 — the tiles at ${players}P, three viewports, and a popover`, async ({ page }) => {
    mkdirSync(OUT, { recursive: true });
    await page.setViewportSize({ width: 1710, height: 869 });
    await startGame(page, { players });
    await playAFew(page, 14, 0x4741_5445 + players); // "GATE"
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await raf(page);
      await expect(page.locator(".board .tile").first()).toBeVisible();
      await page.screenshot({ path: `${OUT}/v1-${players}p-${w}x${h}.png` });
    }
    await page.setViewportSize({ width: 1710, height: 869 });
    await page.locator(".tile").first().hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await page.screenshot({ path: `${OUT}/v1-${players}p-popover.png` });
  });
}
```

In `web/playwright.config.ts` change line 113 to:

```ts
  testIgnore: [
    ...(process.env.YOKOHAMA_WEB_SHOTS === "1" ? [] : ["**/shots.spec.ts"]),
    ...(process.env.YOKOHAMA_WEB_GATES === "1" ? [] : ["**/gates.spec.ts"]),
  ],
```

- [ ] **Step 2: Run the structural guard and the gate**

Run: `cd web && npx playwright test every_spec_can_skip.spec.ts` — Expected: PASS (the new file carries the guard and the import).
Run: `cd web && npm run build && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts`
Expected: 2 passed; eight PNGs under `docs/temp/web-review/gates/`.

- [ ] **Step 3: Look, then send, then STOP**

`Read` each of the eight PNGs. Then `SendUserFile` all eight with the caption *"Gate V1 — the art-only tile at 1440×900, 1710×869, 2560×1440 for 2P and 4P, plus the popover. Site card top-right on its side, pieces below it, id badge by the title. The layout around it is still M4's; that changes next."* **STOP and wait for the owner.** If the owner asks for a change, make it in the tasks above (rects → `tile_rects.json`; pieces → `Pieces.tsx`; sizes → `styles.css`), re-run this gate, send again.

- [ ] **Step 4: Commit**

```bash
git add web/e2e/gates.spec.ts web/playwright.config.ts
git commit -m "test(web): the visual-gate shot generator, opt-in like the help shots

Eight pictures per run for the owner to look at, written under gitignored
docs/temp/; V1 is the art-only tile at three viewports and both player
counts that matter. Not a test: nothing here asserts a property."
```

---

### Task 7: W2 — the grid: the table and the side column

**Files:**
- Modify: `web/src/App.tsx:296-318` (the stage), `web/src/styles.css:119-141` (layout), `:561-566` (delete the deck inset), `:661` (`.stage.replaying`)
- Modify: `web/e2e/the_screen_itself.spec.ts:62-200` (the pinned-deck cell), `:812-868` and `:869-940` (`.stage.replaying` → `.table.replaying`), `web/e2e/shots.spec.ts:103` (same)

**Interfaces:**
- Produces: DOM order `header.banner, div.end-strip, main.table > div.table-body > Board, aside.side > Dashboard, dialog?, div.deck`. `.table` is the size container Task 8 measures against. Under 1201 the deck stays `position: sticky; bottom: 0` (unchanged rule).

- [ ] **Step 1: Re-aim the pinned-deck cell first (the failing test)**

In `web/e2e/the_screen_itself.spec.ts`, rename the cell at `:62` to `"the answer bar is always in view — top of the side column above 1201, bottom edge below"` and replace everything from `const read = async (y: number) => {` to the end of the cell with:

```ts
  // **Two arms, because the deck has two homes (M5 spec §3.1).** Above 1201 it
  // is pinned at the top of the side column by the grid, and the thing that
  // scrolls is the TABLE; below 1201 it is M3.5's bottom-edge sticky bar and the
  // thing that scrolls is the page. Each arm scrolls its own scroller and
  // hit-tests every control at three offsets.
  const readDeck = () =>
    page.evaluate(() => {
      const deck = document.querySelector(".deck")!;
      const r = deck.getBoundingClientRect();
      return {
        top: Math.round(r.top),
        bottom: Math.round(r.bottom),
        innerHeight: window.innerHeight,
        reachable: [...deck.querySelectorAll<HTMLElement>("button")]
          .filter((b) => b.offsetParent !== null)
          .map((b) => {
            const br = b.getBoundingClientRect();
            const hit = document.elementFromPoint(br.x + br.width / 2, br.y + br.height / 2);
            return { name: (b.textContent ?? "").trim().slice(0, 24), reached: b.contains(hit) };
          }),
      };
    });
  const settle = () =>
    page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

  // Arm 1: the grid (the default Desktop Chrome viewport is 1280 wide).
  const tableBottom = await page.evaluate(() => document.querySelector(".table")!.scrollHeight);
  for (const y of [0, 400, tableBottom]) {
    await page.evaluate((to) => {
      document.querySelector(".table")!.scrollTop = to;
    }, y);
    await settle();
    const r = await readDeck();
    expect(r.top, `the deck runs off the top at table scroll ${y}`).toBeGreaterThanOrEqual(0);
    expect(r.bottom, `the deck runs below the viewport at table scroll ${y}`).toBeLessThanOrEqual(r.innerHeight);
    expect(r.reachable.length, `no control in the deck at table scroll ${y}`).toBeGreaterThan(0);
    expect(r.reachable.filter((c) => !c.reached).map((c) => c.name), `controls covered at table scroll ${y}`).toEqual([]);
  }

  // Arm 2: the column stack, exactly M3.5's assertions.
  await page.setViewportSize({ width: 1024, height: 844 });
  await settle();
  const bottomOfPage = await page.evaluate(() => document.body.scrollHeight);
  for (const y of [0, 400, bottomOfPage]) {
    await page.evaluate((to) => window.scrollTo(0, to), y);
    await settle();
    const r = await readDeck();
    expect(Math.abs(r.bottom - r.innerHeight), `the deck is not against the bottom edge at scrollY ${y}`).toBeLessThanOrEqual(1);
    expect(r.top, `the deck runs off the top of the viewport at scrollY ${y}`).toBeGreaterThanOrEqual(0);
    expect(r.reachable.filter((c) => !c.reached).map((c) => c.name), `controls covered at scrollY ${y}`).toEqual([]);
  }
  expect(watch.failures).toEqual([]);
});
```

Change every `.stage.replaying` locator to `.table.replaying` (two in `the_screen_itself.spec.ts`, one in `shots.spec.ts`).

- [ ] **Step 2: Run the e2e to see the re-aimed cell fail**

Run: `cd web && npm run e2e -- the_screen_itself.spec.ts`
Expected: the renamed cell FAILS (no `.table` element yet); the scrub cells fail on the locator. Everything else green.

- [ ] **Step 3: Restructure `App.tsx`**

Replace lines 296–318 (`<main className={\`stage …\`}>` through `</main>`) with:

```tsx
      {/* **The table and the side column (M5 spec §3.1).** Above 1201px `.app`
          is a grid: the table is column 1 across rows 3–4 and scrolls on its
          own; the deck is column 2 row 3, pinned by the grid rather than by
          `sticky`; the side column is column 2 row 4 and scrolls on its own.
          Below 1201px nothing here is a grid item — this is the DOM order the
          M3.5 column stack reads, table then side then the sticky deck, so R8
          holds by construction. `main` is the one landmark; the board never
          goes inside a panel (M4 §3.1), and it never has. */}
      <main className={`table ${replaying ? "replaying" : ""}`}>
        <div className="table-body">
          <Board
            board={view.frame.board}
            art={art}
            labels={labels}
            colors={colors}
            selected={replaying ? [] : selected}
            busy={busy}
            {...(picking && !replaying
              ? { onTileClick: (area: number) => void send(`A${String(area).padStart(2, "0")}`) }
              : {})}
          />
        </div>
      </main>
      <aside className="side">
        <Dashboard panels={view.frame.dashboard} art={art} labels={labels} colors={colors} />
      </aside>
```

- [ ] **Step 4: Replace the layout CSS**

Replace `:119-141` (the `/* ---- layout` block through the end of the `@media (max-width: 1200px)` block) with:

```css
/* ---- layout: the table and the side column (M5) ------------------------ */
/* Below 1201 the DOM order is the layout — table, side, then the sticky deck —
 * which is M3.5's column stack unchanged (spec R8). Above it, the grid. */
.table-body { padding: 1rem; }
.side { display: flex; flex-direction: column; gap: 1rem; padding: 1rem; box-sizing: border-box; }

@media (max-width: 1200px) {
  .table, .side { width: 100%; box-sizing: border-box; }
}

@media (min-width: 1201px) {
  .app {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--side);
    grid-template-rows: auto auto auto minmax(0, 1fr);
    height: 100vh; min-height: 0;
    /* 24rem is the rail's old width and what keeps a 4P board at the 10rem
     * floor (992px) inside the table at 1440; the column widens where the
     * option rows can use it (spec §3.1). */
    --side: 24rem;
  }
  .banner, .end-strip { grid-column: 1 / -1; }
  .table {
    grid-column: 1; grid-row: 3 / 5;
    overflow: auto; min-height: 0;
    /* The size container the fit rule's cqw/cqh units read (Task 8). */
    container-type: size;
    scrollbar-width: thin; scrollbar-color: var(--line-bright) transparent;
  }
  .deck {
    grid-column: 2; grid-row: 3;
    position: static; max-height: 45vh;
    box-shadow: none; border-left: 1px solid var(--line);
  }
  .side {
    grid-column: 2; grid-row: 4;
    overflow: auto; min-height: 0;
    border-left: 1px solid var(--line);
    scrollbar-width: thin; scrollbar-color: var(--line-bright) transparent;
  }
  /* A replayed board is not the live one. A border, never a dim (M4). */
  .table.replaying .board { outline: 1px solid var(--reachable); outline-offset: 4px; }
}
@media (min-width: 1600px) { .app { --side: 30rem; } }
@media (min-width: 1900px) { .app { --side: 36rem; } }
```

Delete `.stage.replaying .board-col { … }` (`:661`) and the `.app:has(.surface[open]) .deck { padding-right: … }` rule with its comment (`:563-566`); the `@media (min-width: 1201px) { .surface { … } }` rule around it stays.

- [ ] **Step 5: Run the gates**

Run: `cd web && npm test && npm run typecheck && npm run e2e`
Expected: all green, including the re-aimed cell's two arms and the scrub cells with the new locator. Record the run's printed figures.

- [ ] **Step 6: Commit**

```bash
git add web/src/App.tsx web/src/styles.css web/e2e/the_screen_itself.spec.ts web/e2e/shots.spec.ts
git commit -m "feat(web): W2 — the grid: a scrolling table, and the answer bar at the top of the side column

Above 1201px the app is a grid — the table in column 1 scrolls on its own,
the deck is pinned at the top of column 2 by the grid rather than by sticky,
the side column scrolls beneath it. Below 1201 the DOM order is the M3.5
column stack, unchanged. The pinned-deck cell now has two arms, one per
home."
```

---

### Task 8: W3 — the fit rule, and its oracle

**Files:**
- Create: `web/src/fit.ts`, `web/src/fit.test.ts`, `web/e2e/the_table.spec.ts`
- Modify: `web/src/Board.tsx:30-35` (the ladder becomes `--tile-ladder`; `--n`, `--rows` set), `web/src/Board.test.tsx:125-131`, `web/src/styles.css` (`.board`)

**Interfaces:**
- Produces: `tileWidthFor(widest, rows, tableW, tableH, remPx = 16): number` (px), `pyramidFits(tileW, rows, tableH, remPx = 16): boolean`, constants `FLOOR_REM`, `CEIL_REM`, `PAD_REM`, `GAP_REM`, `TILE_RATIO`. The `.board` element carries `--tile-ladder`, `--n`, `--rows`; the CSS derives `--tile-w`.

- [ ] **Step 1: Write the failing oracle test**

Create `web/src/fit.test.ts`:

```ts
// The fit rule as arithmetic — the oracle the browser cell checks the CSS against.

import { describe, expect, it } from "vitest";
import { pyramidFits, tileWidthFor } from "./fit";

describe("the fit rule", () => {
  // The owner's screen, minus the 84px banner+strip and a 30rem side column.
  it("is height-bound at 2P and width-bound at 4P on the owner's screen", () => {
    expect(Math.round(tileWidthFor(4, 4, 1710 - 480, 869 - 84))).toBe(245);
    expect(Math.round(tileWidthFor(6, 4, 1710 - 480, 869 - 84))).toBe(194);
  });
  it("is floored at 10rem and capped at 22rem", () => {
    expect(tileWidthFor(6, 4, 700, 500)).toBe(160);
    expect(tileWidthFor(4, 4, 4000, 3000)).toBe(352);
  });
  it("says whether four rows fit", () => {
    expect(pyramidFits(245, 4, 785)).toBe(true);
    expect(pyramidFits(352, 4, 785)).toBe(false);
  });
});
```

- [ ] **Step 2: Run to see it fail, then write `fit.ts`**

Run: `cd web && npx vitest run src/fit.test.ts` — Expected: FAIL, cannot resolve `./fit`.

Create `web/src/fit.ts`:

```ts
// The M5 fit rule (spec §3.3), as arithmetic.
//
// **This is the TEST ORACLE, not the layout.** The layout is one CSS `clamp()`
// over container-query units on `.table .board` (`styles.css`), which needs no
// resize observer and cannot drift from the container it measures. This twin
// exists so a browser cell can compute what the CSS SHOULD have produced from
// the container it measured and compare the rendered tile to it — M3.5's audit
// found a CSS mechanism "validated by a metric insensitive to the way it
// fails"; the oracle is the metric that is not.

export const TILE_RATIO = 709 / 531;
export const FLOOR_REM = 10;
export const CEIL_REM = 22;
/** 1rem of padding each side of `.table-body`. */
export const PAD_REM = 2;
/** `.board-row`'s gap, and the gap between rows. */
export const GAP_REM = 0.4;

export function tileWidthFor(
  widest: number,
  rows: number,
  tableW: number,
  tableH: number,
  remPx = 16,
): number {
  const pad = PAD_REM * remPx;
  const gap = GAP_REM * remPx;
  const byW = (tableW - pad - (widest - 1) * gap) / widest;
  const byH = ((tableH - pad - (rows - 1) * gap) / rows) * TILE_RATIO;
  return Math.min(CEIL_REM * remPx, Math.max(FLOOR_REM * remPx, Math.min(byW, byH)));
}

/** Whether `rows` rows of tiles `tileW` wide fit in `tableH`. */
export function pyramidFits(tileW: number, rows: number, tableH: number, remPx = 16): boolean {
  return (tileW / TILE_RATIO) * rows + (rows - 1) * GAP_REM * remPx + PAD_REM * remPx <= tableH + 0.5;
}
```

Run: `cd web && npx vitest run src/fit.test.ts` — Expected: PASS.

- [ ] **Step 3: Set the variables in `Board.tsx`**

Change the `.board` element's style (in `Board`) to:

```tsx
    <div
      className="board"
      style={
        {
          "--tile-ladder": tileWidth(ordered),
          "--n": Math.max(1, ...ordered.map(([, list]) => list.length)),
          "--rows": ordered.length,
        } as CSSProperties
      }
    >
```

and update `tileWidth`'s doc comment (`:7-29`) to say it is now the **sub-1201 arm** only: above 1201 `styles.css` derives `--tile-w` from the fit rule and this ladder is not read. In `Board.test.tsx:125-131` change `.style.getPropertyValue("--tile-w")` to `"--tile-ladder"`.

- [ ] **Step 4: The CSS**

Change `.board` (`:153-158`) and add the ≥1201 rule:

```css
.board {
  --tile-w: var(--tile-ladder);
  display: flex; flex-direction: column; gap: 0.4rem;
  overflow-x: auto; padding-bottom: 0.4rem;
  scrollbar-width: thin;
  scrollbar-color: var(--line-bright) transparent;
}
@media (min-width: 1201px) {
  /* **The fit rule (M5 spec §3.3).** The largest tile that fits the widest row
   * across the table AND four rows down it, floored at the measured 10rem
   * legibility floor and capped at 22rem, the art's source width on a 2x
   * display. `--n` and `--rows` are the REALIZED shape from Board.tsx; cqw/cqh
   * are `.table`'s. The 2rem is `.table-body`'s padding. `fit.ts` is this same
   * arithmetic as the test oracle — change one, change the other. */
  .table .board {
    --by-w: calc((100cqw - 2rem - (var(--n) - 1) * 0.4rem) / var(--n));
    --by-h: calc((100cqh - 2rem - (var(--rows) - 1) * 0.4rem) / var(--rows) * 709 / 531);
    --tile-w: clamp(10rem, min(var(--by-w), var(--by-h)), 22rem);
    width: max-content; margin: 0 auto; overflow: visible;
  }
}
```

- [ ] **Step 5: The browser cell**

Create `web/e2e/the_table.spec.ts`:

```ts
// M5's browser cells: the things about the table that jsdom cannot see.

import { expect, test, type Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { pyramidFits, tileWidthFor } from "../src/fit";
import { act, readScreen, rng, startGame, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const VIEWPORTS = [
  [1440, 900],
  [1710, 869],
  [2560, 1440],
] as const;

const settle = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

for (const players of [2, 3, 4] as const) {
  test(`the pyramid is the size the rule says, and fits where it says, at ${players}P`, async ({ page }) => {
    const watch = watchForBreakage(page);
    await startGame(page, { players });
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await settle(page);
      const read = await page.evaluate(() => {
        const table = document.querySelector(".table")!;
        const rows = [...document.querySelectorAll(".board-row")];
        return {
          tableW: table.clientWidth,
          tableH: table.clientHeight,
          tileW: document.querySelector(".tile")!.getBoundingClientRect().width,
          widest: Math.max(...rows.map((r) => r.children.length)),
          rows: rows.length,
          bottoms: rows.map((r) => Math.round(r.getBoundingClientRect().bottom)),
          scrollH: table.scrollHeight,
          innerH: window.innerHeight,
        };
      });
      const want = tileWidthFor(read.widest, read.rows, read.tableW, read.tableH);
      expect(
        Math.abs(read.tileW - want),
        `${players}P at ${w}x${h}: rendered ${read.tileW.toFixed(1)}px, the rule says ${want.toFixed(1)}`,
      ).toBeLessThanOrEqual(1);
      if (pyramidFits(want, read.rows, read.tableH)) {
        for (const b of read.bottoms) {
          expect(b, `${players}P at ${w}x${h}: a row's bottom (${b}) is below the viewport (${read.innerH})`).toBeLessThanOrEqual(read.innerH);
        }
      } else {
        expect(read.scrollH, `${players}P at ${w}x${h}: the rule cannot fit, so the table must scroll`).toBeGreaterThan(read.tableH);
      }
      console.log(`${players}P at ${w}x${h}: tile ${(read.tileW / 16).toFixed(2)}rem (rule ${(want / 16).toFixed(2)}rem) — recorded`);
    }
    expect(watch.failures).toEqual([]);
  });
}
```

Run: `cd web && npm run e2e -- the_table.spec.ts` — Expected: 3 passed, nine readings printed. **Mutation-prove it:** temporarily change the CSS `min(var(--by-w), var(--by-h))` to `var(--by-w)`, rebuild, re-run — the 1710×869 2P cell must FAIL (tile 18.4rem, rows below the viewport). Restore. Note the readings for the spec's §3.3 table in the close.

- [ ] **Step 6: Gate and commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e`

```bash
git add web/src/fit.ts web/src/fit.test.ts web/src/Board.tsx web/src/Board.test.tsx web/src/styles.css web/e2e/the_table.spec.ts
git commit -m "feat(web): W3 — one fit rule for the pyramid, and its oracle

Above 1201 the tile is clamp(10rem, min(by-width, by-height), 22rem) over the
table's container units; below it the M3.5 ladder stands. fit.ts is the same
arithmetic as the test oracle, and the browser cell reads the rendered tile
against it at three viewports and three player counts — mutation-proven by
dropping the height term."
```

---

### Task 9: W6 — the side column: Last move, Players, Hand

**Files:**
- Create: `web/src/LastMove.tsx`, `web/src/LastMove.test.tsx`
- Modify: `web/src/Decision.tsx:360-366` (delete the messages block), `:392-394` (delete `isRefusal`); `web/src/Decision.test.tsx:263-276` (the refusal cell moves); `web/src/Dashboard.tsx` (`only` prop; `Space` shows its VP; export `Tracks`, `Achievements`); `web/src/App.tsx`; `web/src/styles.css`

**Interfaces:**
- Produces: `LastMove({ messages })` → `section.panel.last-move > h2 + div#last-move-lines.messages[role=status]`; `isRefusal(message)` (moved); `Dashboard` gains `only?: ("Players" | "Tracks" | "Achievements" | "Hand")[]`; `Dashboard.tsx` exports `Tracks` and `Achievements` for Task 11.

- [ ] **Step 1: Write the failing tests**

Create `web/src/LastMove.test.tsx`:

```tsx
// The applied-move lines, in the side column rather than the deck.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { LastMove } from "./LastMove";
import { wire } from "./wire/contract";

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

describe("the last move", () => {
  it("holds every line the server sent, refusals as warnings", () => {
    const state = wire.refused;
    const refusals = state.messages.filter((m) => /illegal|refus|cannot|not legal|out of range/i.test(m));
    expect(refusals.length, "the fixture must carry a refusal").toBeGreaterThan(0);
    render(<LastMove messages={state.messages} />);
    for (const m of state.messages) expect(screen.getByText(new RegExp(escape(m)))).toBeInTheDocument();
    for (const m of refusals) expect(screen.getByText(new RegExp(escape(m)))).toHaveClass("warn");
  });

  // A live region has to exist BEFORE its text arrives to be announced.
  it("keeps its live region mounted while empty", () => {
    const { container } = render(<LastMove messages={[]} />);
    const region = container.querySelector('[role="status"]');
    expect(region).not.toBeNull();
    expect(region!.textContent).toBe("");
  });

  it("folds and unfolds without unmounting the region", async () => {
    const user = userEvent.setup();
    const { container } = render(<LastMove messages={["Turn 1 Red -> Turn 2 Blue"]} />);
    await user.click(screen.getByRole("button", { name: "hide" }));
    expect(container.querySelector(".messages.folded")).not.toBeNull();
    expect(container.querySelector('[role="status"]')).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "show" }));
    expect(container.querySelector(".messages.folded")).toBeNull();
  });
});
```

Move the cell `Decision.test.tsx:263-276` ("a refusal is shown as a warning rather than folded into the log") out: delete it there (its `escape` helper stays if another cell uses it; `grep -n "escape(" web/src/Decision.test.tsx`), and add to `Decision.test.tsx`:

```tsx
  // The applied-move lines left the deck for the side column (M5 §3.5); a deck
  // that still rendered them would take the board's height budget back.
  it("renders none of the applied-move lines itself", () => {
    const state = wire.refused;
    expect(state.messages.length).toBeGreaterThan(0);
    const { container } = show(state);
    for (const m of state.messages) {
      expect(container.textContent).not.toContain(m);
    }
  });
```

Add to `Dashboard.test.tsx`:

```tsx
describe("what the side column shows", () => {
  it("draws only the panels it is asked for", () => {
    const { container } = render(
      <Dashboard panels={wire.options.frame.dashboard} art={wire.artManifest} labels={wire.labels} colors={COLORS} only={["Players", "Hand"]} />,
    );
    expect(screen.getByText("Players")).toBeInTheDocument();
    expect(screen.queryByText(/Tracks/)).toBeNull();
    expect(screen.queryByText(/Achievements/)).toBeNull();
    expect(container.querySelectorAll(".panel").length).toBe(2);
  });
  it("prints every track space's points as text, not only in a tooltip", () => {
    const { container } = show(wire.options);
    const tracks = tracksOf(wire.options);
    const subs = [...container.querySelectorAll(".space .sub")].map((s) => s.textContent);
    for (const s of tracks.church) expect(subs).toContain(`★${s.vp}`);
    for (const s of tracks.customs) expect(subs).toContain(`★${s.vp}`);
  });
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run src/LastMove.test.tsx src/Decision.test.tsx src/Dashboard.test.tsx`
Expected: LastMove cannot resolve; the Decision cell fails (lines rendered); the Dashboard cells fail (`only` unknown, no `.sub`).

- [ ] **Step 3: Write `LastMove.tsx`**

```tsx
// The applied-move lines — what the CLI prints after every move — in the side
// column rather than in the deck (M5 spec §3.5).
//
// They were the deck's biggest variable: sixteen lines and 112px after a bot's
// turn, in the box that is pinned so that ANSWERING never costs a scroll. The
// answer is what the deck is for; the account of the last move is what the
// column is for. The `role="status"` container stays mounted and empty rather
// than unmounted, because a live region inserted with its text is unreliably
// announced; folding hides it with `max-height: 0`, which keeps it in the tree.

import { useState } from "react";

export function isRefusal(message: string): boolean {
  return (
    message.trimStart().startsWith("!") ||
    /illegal|refus|cannot|not legal|out of range/i.test(message)
  );
}

export function LastMove({ messages }: { messages: string[] }) {
  const [folded, setFolded] = useState(false);
  return (
    <section className="panel last-move">
      <h2>
        Last move
        <button
          type="button"
          className="ghost fold"
          aria-expanded={!folded}
          aria-controls="last-move-lines"
          onClick={() => setFolded((f) => !f)}
        >
          {folded ? "show" : "hide"}
        </button>
      </h2>
      <div id="last-move-lines" className={`messages ${folded ? "folded" : ""}`} role="status">
        {messages.map((m, i) => (
          <div key={i} className={isRefusal(m) ? "warn" : ""}>
            {m}
          </div>
        ))}
      </div>
    </section>
  );
}
```

- [ ] **Step 4: Take the block out of `Decision.tsx`**

Delete lines 360–366 (the `<div className="messages" role="status">` block and its comment) and the `isRefusal` function (`:392-394`). The `watching`/`locked` arms keep their own `.messages` blocks — those are the prompt, not the account.

- [ ] **Step 5: `Dashboard.tsx` — `only`, visible VP, exports**

Change the `Dashboard` signature and body to:

```tsx
export type PanelName = "Players" | "Tracks" | "Achievements" | "Hand";

export function Dashboard({
  panels,
  art,
  labels,
  colors,
  only,
}: {
  panels: Panel[];
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
  /** Which panels to draw; absent means every one. The side column asks for
   *  Players and Hand (M5 §3.5); Tracks and Goals live on the table. */
  only?: PanelName[];
}) {
  const wanted = (p: Panel) => !only || only.some((name) => name in p);
  return (
    <>
      {panels.filter(wanted).map((panel, i) => {
```

(the rest of the map body unchanged). Change `function Tracks(` to `export function Tracks(` and `function Achievements(` to `export function Achievements(`. In `Space`, add a `sub` line: change its signature to include `sub: string` and render `<span className="sub">{sub}</span>` after `{children}`; at the two call sites pass `sub={`★${s.vp}`}`.

- [ ] **Step 6: `App.tsx` and CSS**

In `App.tsx`, import `LastMove` and change the side column to:

```tsx
      <aside className="side">
        <LastMove messages={state.messages} />
        <Dashboard panels={view.frame.dashboard} art={art} labels={labels} colors={colors} only={["Players", "Hand"]} />
      </aside>
```

Append to `styles.css`:

```css
/* ---- M5: the side column ----------------------------------------------- */
.last-move h2 { display: flex; align-items: center; }
.last-move .fold { margin-left: auto; font-size: 10px; padding: 0 0.4rem; letter-spacing: 0; text-transform: none; }
.last-move .messages { margin-top: 0.3rem; max-height: 9rem; overflow: auto; scrollbar-width: thin; }
.last-move .messages.folded { max-height: 0; overflow: hidden; margin-top: 0; }
.space { display: inline-flex; flex-direction: column; align-items: center; line-height: 1.15; }
.space .sub { font-size: 8.5px; color: var(--vp); }
```

- [ ] **Step 7: The Undo cell on the new tile (spec §3.7)**

Append to `web/e2e/the_table.spec.ts`:

```ts
test("Undo takes the pieces back off the tile", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  // Walk to the first placement picker and place on the first eligible tile.
  const pick = rng(0x554e_444f); // "UNDO"
  let placed: string | null = null;
  for (let i = 0; i < 30 && placed === null; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if (screen.tiles > 0 && screen.confirm) {
      const tile = page.locator("button.tile:not([disabled])").first();
      placed = (await tile.getAttribute("aria-label"))!.slice(0, 3);
      await tile.click();
      await page.getByRole("button", { name: "Confirm" }).click();
      break;
    }
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(placed, "never reached a placement picker").not.toBeNull();
  const piecesOn = () =>
    page.evaluate((id) => {
      const tile = [...document.querySelectorAll(".tile")].find(
        (t) => t.querySelector(".tile-id")?.textContent === id,
      )!;
      return tile.querySelector(".pieces")?.getAttribute("aria-label") ?? "";
    }, placed!);
  await expect.poll(piecesOn, { message: "the placement put no pieces on the tile" }).toMatch(/assistant/);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect.poll(piecesOn, { message: "Undo left the pieces on the tile" }).not.toMatch(/assistant/);
  expect(watch.failures).toEqual([]);
});
```

(The `Undo` button is offered on every `options` prompt — the Movement prompt that follows a placement is one — and unwinds the placement; the pieces are the tile's only state now, so this is the cell that says the art-only tile follows the record.)

- [ ] **Step 8: Gate, e2e, commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e`
Expected: green. The pinned-deck cell's `mostCommentary` reading now comes from `.decision > .messages`, which only the `watching`/`locked` arms render — if the cell's anti-vacuity assertion (`mostCommentary > 0`) trips, change its probe to read `.last-move .messages` and keep the assertion; the squeeze it guarded is still exercised through the option list.

```bash
git add web/src/LastMove.tsx web/src/LastMove.test.tsx web/src/Decision.tsx web/src/Decision.test.tsx web/src/Dashboard.tsx web/src/Dashboard.test.tsx web/src/App.tsx web/src/styles.css
git commit -m "feat(web): W6 — the side column: last move out of the deck, players and hand under it

The applied-move lines were the deck's biggest variable; they move to a
folding panel with the live region kept mounted. The Dashboard draws only
what it is asked for, and every track space prints its points as text."
```

---

### Task 10: W2b — the Game menu

**Files:**
- Create: `web/src/GameMenu.tsx`, `web/src/GameMenu.test.tsx`
- Modify: `web/src/App.tsx:194-197` (the meta span), `web/e2e/the_table.spec.ts`

**Interfaces:**
- Produces: `GameMenu({ state, onSwitch: (s: ClientState) => void, onNew: () => void })` → `details.cards-menu.game-menu > summary + div`.

- [ ] **Step 1: Write the failing tests**

Create `web/src/GameMenu.test.tsx`:

```tsx
// The session control in the banner: saved games, new game, no reload.

import { readFileSync } from "node:fs";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { renderApp } from "./test/harness";
import { wire } from "./wire/contract";

describe("the Game menu", () => {
  it("lists the other saved games and switches to one", async () => {
    const other = { ...wire.options4p, id: "sim-other" };
    await renderApp(wire.options, (call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return [wire.options.id, "sim-other"];
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other")) return other;
      return undefined;
    });
    const user = userEvent.setup();
    await user.click(screen.getByText(new RegExp(wire.options.id)));
    const button = await screen.findByRole("button", { name: "sim-other" });
    await user.click(button);
    // The summary now names the other game, and the menu (still open) lists
    // the game we came from as the one to switch back to.
    await waitFor(() => expect(screen.getByText(/sim · sim-other/)).toBeInTheDocument());
    expect(screen.getByRole("button", { name: wire.options.id })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "sim-other" })).toBeNull();
  });

  it("returns to the setup screen without a reload", async () => {
    await renderApp(wire.options, (call) =>
      call.method === "GET" && call.path.endsWith("/api/games") ? [wire.options.id] : undefined,
    );
    const user = userEvent.setup();
    await user.click(screen.getByText(new RegExp(wire.options.id)));
    await user.click(screen.getByRole("button", { name: "New game" }));
    expect(await screen.findByRole("heading", { name: "New game" })).toBeInTheDocument();
    // A reload in jsdom is a no-op that would pass; the source is the check.
    for (const f of ["App.tsx", "GameMenu.tsx"]) {
      expect(readFileSync(new URL(`./${f}`, import.meta.url), "utf8")).not.toMatch(/location\.reload/);
    }
  });
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run src/GameMenu.test.tsx` — Expected: FAIL (no menu; the id is not a control).

- [ ] **Step 3: Write `GameMenu.tsx`**

```tsx
// The session control (M5 spec §3.7): the current game, the other saved games,
// and a way back to the setup screen that is not a page reload.
//
// The smallest thing that removes the reload, and deliberately no more: M6 is
// the multi-game layer (lobby, chat, another player's game) and it grows from
// this menu. Nothing is deleted from here; the setup screen already does that.

import { useState } from "react";
import { api, ApiError } from "./api";
import type { ClientState } from "./types";

export function GameMenu({
  state,
  onSwitch,
  onNew,
}: {
  state: ClientState;
  onSwitch: (s: ClientState) => void;
  onNew: () => void;
}) {
  const [saved, setSaved] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const said = (e: unknown) => (e instanceof ApiError ? e.message : String(e));
  const load = () => {
    setError(null);
    api.list().then(setSaved).catch((e) => setError(said(e)));
  };
  const others = (saved ?? []).filter((id) => id !== state.id);
  return (
    <details
      className="cards-menu game-menu"
      onToggle={(e) => {
        if (e.currentTarget.open) load();
      }}
    >
      <summary className="ghost meta">
        {state.mode} &middot; {state.id}
        {state.frame.title.autosaved ? " · saved" : ""}
      </summary>
      <div>
        <button type="button" className="ghost" onClick={onNew}>
          New game
        </button>
        <div className="label">Saved games</div>
        {saved === null && !error && <div className="muted">loading…</div>}
        {error && <div className="error">{error}</div>}
        {others.map((id) => (
          <button
            type="button"
            className="ghost"
            key={id}
            onClick={() => api.get(id).then(onSwitch).catch((e) => setError(said(e)))}
          >
            {id}
          </button>
        ))}
        {saved !== null && others.length === 0 && <div className="muted">no other saved game</div>}
      </div>
    </details>
  );
}
```

- [ ] **Step 4: Put it in the banner**

In `App.tsx` replace the meta `<span className="meta">…</span>` (`:194-197`) with:

```tsx
        <GameMenu
          state={state}
          onSwitch={(s) => {
            setSurface(null);
            setScrubK(null);
            setState(s);
          }}
          onNew={() => {
            setSurface(null);
            setScrubK(null);
            setState(null);
          }}
        />
```

and import it. The `NewGame` screen mounts on `state === null` as it does on first load.

- [ ] **Step 5: The browser cell**

Append to `web/e2e/the_table.spec.ts`:

```ts
test("the Game menu switches between saved games from inside a game", async ({ page }) => {
  const watch = watchForBreakage(page);
  const a = await startGame(page, { players: 2 });
  await playAFew(page, 4, 0x4d45_4e55); // "MENU"
  await page.goto("/");
  const b = await startGame(page, { players: 2 });
  await page.locator(".game-menu > summary").click();
  await page.getByRole("button", { name: a.id, exact: true }).click();
  await expect(page.locator(".game-menu > summary")).toContainText(a.id);
  await page.locator(".game-menu > summary").click();
  await page.getByRole("button", { name: b.id, exact: true }).click();
  await expect(page.locator(".game-menu > summary")).toContainText(b.id);
  expect(watch.failures).toEqual([]);
});
```

- [ ] **Step 6: Gate, e2e, commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e`

```bash
git add web/src/GameMenu.tsx web/src/GameMenu.test.tsx web/src/App.tsx web/e2e/the_table.spec.ts
git commit -m "feat(web): W2b — the Game menu: saved games and a new game, no reload

The banner's id becomes a menu listing the other saved games and a way back
to the setup screen. The seed of M6's multi-game layer, and no more."
```

---

### Task 11: W5a — the sections registry and the strips; the four drawers retire

**Files:**
- Create: `web/src/sections.ts`, `web/src/sections.test.ts`, `web/src/Strips.tsx`, `web/src/Strips.test.tsx`
- Modify: `web/src/Surfaces.tsx` (`SURFACES` `:70-100`, `SurfaceName` `:53-67`, `Surface`'s `tracks` prop and four render arms, delete `Ports` `:333-381`, `Labs` `:382-429`, the surface `Tracks` `:664-753`, `BuildingSites` `:754-779`, `SiteCard` `:780-826`; re-word `Areas`' companion line `:555`), `web/src/Surfaces.test.tsx` (`:50-82` the branch cell uses log/areas; delete `:124-148` port cell, `:271-328` tracks cells, `:329-398` sites cells — they move), `web/src/App.tsx` (no `tracks` prop to `Surface`)
- Modify: `web/e2e/the_screen_itself.spec.ts:335-370` (the four-surfaces cell), `:632-720` (labels ≥ 14 → the drawers that remain)

**Interfaces:**
- Produces: `SECTION_KEYS`, `SectionKey`, `SECTION_LABELS`, `DEFAULT_SECTIONS`, `loadSections()`, `saveSections(keys)`; `PortStrip({ panel, art })`, `LabStrip({ panel, art, labels })`, `TracksStrip({ tracks, art, colors })`, `GoalsStrip({ cards, art, labels, colors })`, `SitesStrip({ panel, art, colors })`, each rendering `section.table-sec.<key>`.

- [ ] **Step 1: Write the failing tests**

Create `web/src/sections.test.ts`:

```ts
// Which sections the table draws, and where that choice is kept.

import { describe, expect, it } from "vitest";
import { DEFAULT_SECTIONS, loadSections, saveSections, SECTION_KEYS } from "./sections";

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

describe("the enabled sections", () => {
  it("default to the port, the laboratory, the tracks and the goals", () => {
    expect(loadSections(fakeStorage())).toEqual(DEFAULT_SECTIONS);
    expect(DEFAULT_SECTIONS).toEqual(["ports", "labs", "tracks", "achievements"]);
  });
  it("round-trip through storage, in the registry's order and dropping unknown keys", () => {
    const s = fakeStorage();
    saveSections(["buildings", "ports"], s);
    expect(loadSections(s)).toEqual(["ports", "buildings"]);
    saveSections([], s);
    expect(loadSections(s)).toEqual([]);
    s.setItem("yokohama.table.sections", JSON.stringify(["ports", "nonsense"]));
    expect(loadSections(s)).toEqual(["ports"]);
  });
  it("fall back to the default when storage throws or holds garbage", () => {
    expect(loadSections(fakeStorage(undefined, true))).toEqual(DEFAULT_SECTIONS);
    expect(loadSections(fakeStorage("not json"))).toEqual(DEFAULT_SECTIONS);
    expect(loadSections(fakeStorage('{"a":1}'))).toEqual(DEFAULT_SECTIONS);
    expect(() => saveSections(["ports"], fakeStorage(undefined, true))).not.toThrow();
  });
  it("name every key", () => {
    for (const k of SECTION_KEYS) expect(typeof k).toBe("string");
  });
});
```

Create `web/src/Strips.test.tsx`:

```tsx
// The strips on the table: the physical boards' shapes, drawn from the wire.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GoalsStrip, LabStrip, PortStrip, SitesStrip, TracksStrip } from "./Strips";
import { achievementsOf, tracksOf } from "./test/panels";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

describe("the port strip", () => {
  it("draws six slots per board under their zone and gate, from the slots themselves", () => {
    const { container } = render(<PortStrip panel={wire.ports} art={wire.artManifest} />);
    expect(container.querySelectorAll("section.table-sec.ports")).toHaveLength(wire.ports.boards.length);
    expect(container.querySelectorAll(".strip-slot")).toHaveLength(6 * wire.ports.boards.length);
    const text = container.textContent ?? "";
    for (const zone of ["Low", "Mid", "High"]) expect(text).toContain(zone);
    const gates = new Map<string, number>();
    for (const s of wire.ports.boards[0]!.slots) gates.set(s.zone, Math.min(gates.get(s.zone) ?? 99, s.min_power));
    for (const [, power] of gates) expect(text).toContain(`power ${power}+`);
    expect(text).toContain(`${wire.ports.deck_remaining} in the deck`);
  });
});

describe("the laboratory strip", () => {
  it("draws six slots with the surcharge on the four that carry one", () => {
    const { container } = render(<LabStrip panel={wire.labs} art={wire.artManifest} labels={wire.labels} />);
    expect(container.querySelectorAll(".strip-slot")).toHaveLength(6 * wire.labs.boards.length);
    const marks = [...container.querySelectorAll(".chip.warn")].map((c) => c.textContent);
    const want = wire.labs.boards[0]!.slots.filter((s) => s.surcharge > 0).map((s) => `+${s.surcharge}`);
    expect(marks.slice(0, want.length)).toEqual(want);
  });
});

describe("the tracks strip", () => {
  it("prints every space with its points as text", () => {
    const tracks = tracksOf(wire.optionsMany);
    const { container } = render(<TracksStrip tracks={tracks} art={wire.artManifest} colors={COLORS} />);
    const text = container.textContent ?? "";
    for (const s of tracks.church) expect(text).toContain(`★${s.vp}`);
    for (const s of tracks.customs) expect(text).toContain(`★${s.vp}`);
    expect(container.querySelector("section.table-sec.tracks")).not.toBeNull();
  });
});

describe("the goals strip", () => {
  it("draws the three cards in a row, landscape", () => {
    const cards = achievementsOf(wire.options);
    const { container } = render(<GoalsStrip cards={cards} art={wire.artManifest} labels={wire.labels} colors={COLORS} />);
    expect(container.querySelectorAll(".card-face.achievement")).toHaveLength(cards.length);
    expect(container.querySelector(".table-sec.achievements .card-strip")).not.toBeNull();
  });
});

describe("the building sites strip", () => {
  it("draws one card per area that has a site, from the live frame", () => {
    const withSites = wire.areas.rows.flat().filter((c) => c.tile.site !== null);
    expect(withSites.length).toBeGreaterThan(0);
    const { container } = render(<SitesStrip panel={wire.areas} art={wire.artManifest} colors={COLORS} />);
    expect(container.querySelectorAll(".site-card")).toHaveLength(withSites.length);
  });
  it("marks a built space in the builder's seat colour, from the LIVE model", () => {
    const built = structuredClone(wire.areas) as typeof wire.areas;
    const first = built.rows.flat().find((c) => c.tile.site !== null)!;
    first.tile.site!.slots[1]!.built_by = 1;
    first.tile.site!.trading_house_owner = 0;
    const { container } = render(<SitesStrip panel={built} art={wire.artManifest} colors={COLORS} />);
    const card = container.querySelector(".site-card")!;
    expect(card.querySelectorAll(".site-space.built")).toHaveLength(1);
    expect(card.querySelector(".site-space.built")!.className).toMatch(/seat-blue/);
    expect(card.querySelector(".site-trading-house.built")?.className).toMatch(/seat-red/);
  });
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run src/sections.test.ts src/Strips.test.tsx` — Expected: FAIL, both modules missing.

- [ ] **Step 3: Write `sections.ts`**

```ts
// Which sections the table draws (M5 spec §3.4), and where that choice lives.
//
// The set persists per BROWSER in localStorage: a convenience, not state — a
// game opened elsewhere shows the default set, and a storage that throws (a
// private window, a blocked site) reads as the default and never as an error.

export const SECTION_KEYS = ["ports", "labs", "tracks", "achievements", "buildings"] as const;
export type SectionKey = (typeof SECTION_KEYS)[number];

export const SECTION_LABELS: Record<SectionKey, string> = {
  ports: "Port",
  labs: "Laboratory",
  tracks: "Tracks",
  achievements: "Goals",
  buildings: "Building sites",
};

export const DEFAULT_SECTIONS: readonly SectionKey[] = ["ports", "labs", "tracks", "achievements"];

const KEY = "yokohama.table.sections";

function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

/** The enabled keys in REGISTRY order, unknown keys dropped. */
export function loadSections(storage: Storage | null = browserStorage()): SectionKey[] {
  try {
    const raw = storage?.getItem(KEY);
    if (!raw) return [...DEFAULT_SECTIONS];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [...DEFAULT_SECTIONS];
    return SECTION_KEYS.filter((k) => parsed.includes(k));
  } catch {
    return [...DEFAULT_SECTIONS];
  }
}

export function saveSections(keys: readonly SectionKey[], storage: Storage | null = browserStorage()): void {
  try {
    storage?.setItem(KEY, JSON.stringify(SECTION_KEYS.filter((k) => keys.includes(k))));
  } catch {
    // A per-browser convenience; losing it loses nothing.
  }
}
```

- [ ] **Step 4: Write `Strips.tsx`**

```tsx
// The strips on the table (M5 spec §3.4): the physical boards' shapes, drawn
// from the same payloads the drawers used to draw, laid out the way the boards
// lie on the table — six slots in a row — rather than the way a 704px drawer
// stacked them. No board art yet (R6); the shape is the board.

import type {
  AchievementModel,
  AreaCard,
  AreasPanel,
  ArtManifest,
  Labels,
  LabsPanel,
  PlayerColor,
  PortsPanel,
  TracksPanel,
} from "./types";
import { AchievementCardView, OrderCardView, SiteFace, TechCardView } from "./Cards";
import { Tracks } from "./Dashboard";

/** The zone gates FOLDED FROM THE SLOTS, never typed as prose. */
function gateWords(panel: PortsPanel): string {
  const gates = new Map<string, number>();
  for (const b of panel.boards) {
    for (const s of b.slots) {
      const seen = gates.get(s.zone);
      if (seen === undefined || s.min_power < seen) gates.set(s.zone, s.min_power);
    }
  }
  return [...gates.entries()]
    .sort((a, b) => a[1] - b[1])
    .map(([zone, power]) => `${zone.toUpperCase()} needs power ${power}`)
    .join(", ");
}

export function PortStrip({ panel, art }: { panel: PortsPanel; art: ArtManifest | null }) {
  return (
    <>
      {panel.boards.map((b) => (
        <section className="table-sec ports" key={String(b.board_b)} aria-label={`Port ${b.board_b ? "B" : "A"}`}>
          <h2>
            Port {b.board_b ? "B" : "A"}
            <span className="meta">
              {gateWords(panel)} &middot; {panel.deck_remaining} in the deck
            </span>
          </h2>
          <div className="board-strip">
            {b.slots.map((s) => (
              <div className={`strip-slot zone-${s.zone.toLowerCase()}`} key={s.slot}>
                <div className="slot-head">
                  <span className={`chip zone-${s.zone.toLowerCase()}`}>{s.zone}</span>
                  <span className="chip">power {s.min_power}+</span>
                </div>
                {s.card == null ? (
                  <div className="card-blank empty">empty</div>
                ) : (
                  <OrderCardView art={art} card={s.printed} id={s.card} />
                )}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

export function LabStrip({ panel, art, labels }: { panel: LabsPanel; art: ArtManifest | null; labels: Labels | null }) {
  return (
    <>
      {panel.boards.map((b) => (
        <section className="table-sec labs" key={String(b.board_b)} aria-label={`Laboratory ${b.board_b ? "B" : "A"}`}>
          <h2>
            Laboratory {b.board_b ? "B" : "A"}
            <span className="meta">pay the industry; the slot&rsquo;s surcharge is in the figure &middot; {panel.deck_remaining} in the deck</span>
          </h2>
          <div className="board-strip">
            {b.slots.map((s) => (
              <div className="strip-slot" key={s.slot}>
                <div className="slot-head">
                  <span className="chip">slot {s.slot + 1}</span>
                  {s.surcharge > 0 && <span className="chip warn">+{s.surcharge}</span>}
                </div>
                {s.card == null ? (
                  <div className="card-blank empty">empty</div>
                ) : (
                  <TechCardView art={art} card={s.printed} labels={labels} cost={s.printed ? s.printed.industry_cost + s.surcharge : undefined} />
                )}
              </div>
            ))}
          </div>
        </section>
      ))}
    </>
  );
}

export function TracksStrip({ tracks, art, colors }: { tracks: TracksPanel; art: ArtManifest | null; colors: PlayerColor[] }) {
  return (
    <section className="table-sec tracks" aria-label="Tracks">
      <Tracks tracks={tracks} art={art} colors={colors} />
    </section>
  );
}

export function GoalsStrip({ cards, art, labels, colors }: { cards: AchievementModel[]; art: ArtManifest | null; labels: Labels | null; colors: PlayerColor[] }) {
  return (
    <section className="table-sec achievements" aria-label="Goals">
      <h2>Goals <span className="meta">one per category, first / later VP</span></h2>
      <div className="card-strip">
        {cards.map((c) => (
          <AchievementCardView key={c.achievement_type} model={c} art={art} labels={labels} colors={colors} />
        ))}
      </div>
    </section>
  );
}

function SiteCard({ card, art, colors }: { card: AreaCard; art: ArtManifest | null; colors: PlayerColor[] }) {
  const site = card.tile.site!;
  const id = `A${String(card.tile.area).padStart(2, "0")}`;
  return (
    <figure className="site-card">
      <SiteFace art={art} site={site} colors={colors} />
      <figcaption>
        <span className="tile-id-words">{id}</span> <b>Site {site.index}</b>
        <span className="muted"> &middot; trading house &#9733;{site.trading_house_vp}</span>
      </figcaption>
    </figure>
  );
}

/** Reads the AREAS frame, never `/index/sites`: the index clones the pristine
 *  registry card and nothing is ever built on it (`types.ts`'s own caveat). */
export function SitesStrip({ panel, art, colors }: { panel: AreasPanel; art: ArtManifest | null; colors: PlayerColor[] }) {
  const cards = panel.rows.flat().filter((c) => c.tile.site !== null);
  return (
    <section className="table-sec buildings" aria-label="Building sites">
      <h2>Building sites <span className="meta">a space in a seat&rsquo;s colour is built on; the band is the trading house</span></h2>
      {cards.length === 0 ? (
        <div className="muted">This board has no building sites.</div>
      ) : (
        <div className="site-grid">
          {cards.map((card) => (
            <SiteCard key={card.tile.area} card={card} art={art} colors={colors} />
          ))}
        </div>
      )}
    </section>
  );
}
```

Note the `SiteCard` figcaption uses `tile-id-words`, not `tile-id`: `.tile-id` is the tile's absolutely positioned badge now (Task 4), and `Board.test.tsx`'s `tileOf` reads `.tile-id`. Add `.tile-id-words { color: var(--fg-faint); font-size: 10px; }` to the CSS in Step 7.

- [ ] **Step 5: Retire the four drawers in `Surfaces.tsx`**

Delete `Ports`, `Labs`, the surface `Tracks`, `BuildingSites` and `SiteCard`, the four `SURFACES` entries (`ports`, `labs`, `tracks`, `buildings`), the four names from `SurfaceName`, the `tracks` prop from `Surface` and its four render arms, and the imports that become unused (`OrderCardView`, `TechCardView`, `SiteFace`, `PortsPanel`, `LabsPanel`, `TracksPanel`, `AreaCard` — check each with `grep`). `NOTHING_TO_FETCH` stays for `help`. In `Areas` (`:555`) change the companion line to:

```tsx
            <div className="muted">detail lives in the {card.companion} section of the table</div>
```

In `App.tsx` drop `tracks={tracks}` from `<Surface>` and the `tracksPanel`/`tracks` locals (`:160-162`) if nothing else reads them (Task 12 reads the tracks from the frame itself).

- [ ] **Step 6: Move and re-aim the surface tests**

In `Surfaces.test.tsx`: in the branch cell (`:50-82`) replace `labs` with `log` — the stub answers `/log` with `wire.log`, the wait becomes `await screen.findByText(new RegExp(\`${wire.log.total} event\`))` (the log's lede, exactly as the cell at `:149` reads it), and the rerender to `areas` stays — so it still shows a payload rendered through a switched branch; delete the port-gate cell (`:129-148`), the tracks `describe` (`:271-328`) and the sites `describe` (`:329-398`) — their assertions now live in `Strips.test.tsx`. Replace the registry cell (`:278-289`) with one in `Strips.test.tsx`:

```tsx
import { SECTION_KEYS } from "./sections";
describe("the Areas panel's companions", () => {
  it("name sections that exist", () => {
    for (const companion of ["Ports", "Labs", "Tracks"]) {
      expect(SECTION_KEYS).toContain(companion.toLowerCase());
    }
  });
});
```

- [ ] **Step 7: CSS for the strips**

Append:

```css
/* ---- M5: the strips on the table ---------------------------------------- */
.table-sections {
  display: flex; flex-wrap: wrap; justify-content: center; gap: 1rem;
  margin-top: 1rem;
  /* Cards grow with the room, not with the tile: 12cqw is 9.2rem at the
   * owner's 1230px table and the 12rem cap by 1600px (spec §3.4, corrected in
   * the plan — a card keyed to the tile would need Board's shape here). */
  --card-w: clamp(8rem, 12cqw, 12rem);
}
.table-sec {
  flex: 0 1 auto; max-width: 100%; box-sizing: border-box;
  background: var(--bg-raised); border: 1px solid var(--line); border-radius: var(--radius);
  padding: 0.7rem 0.85rem;
}
.table-sec > h2 {
  font-size: 10px; letter-spacing: 0.2em; text-transform: uppercase; color: var(--fg-faint);
  font-family: var(--mono); font-weight: 500; margin-bottom: 0.5rem;
  display: flex; align-items: baseline; gap: 0.8rem; flex-wrap: wrap;
}
.table-sec > h2 .meta { letter-spacing: 0; text-transform: none; font-size: 10.5px; }
.board-strip { display: flex; gap: 0.5rem; flex-wrap: wrap; }
.strip-slot { display: flex; flex-direction: column; gap: 0.25rem; }
.strip-slot .card-face { width: var(--card-w); }
.strip-slot .card-blank.empty { width: var(--card-w); box-sizing: border-box; }
.table-sec.achievements .card-face.achievement { width: calc(var(--card-w) * 1.5); }
.table-sec.achievements .card-face.achievement img { aspect-ratio: 799 / 530; }
.table-sec .site-grid { grid-template-columns: repeat(auto-fill, minmax(var(--card-w), 1fr)); }
.table-sec.tracks .panel { background: transparent; border: 0; padding: 0; }
.table-sec.collapsed { color: var(--fg-faint); }
.tile-id-words { color: var(--fg-faint); font-size: 10px; }
```

- [ ] **Step 8: Re-aim the two nav-derived browser cells**

In `the_screen_itself.spec.ts:335`: rename to `"the drawers that remain open, draw, and close"` and change the list to `["Areas", "Log", "Help"]`. In `:632-720` change the `≥ 14` to `≥ 10` (the drawers: Areas, Log, Suggest, Why, Help, and the five behind Cards) and leave the rest.

- [ ] **Step 9: Gate, e2e, commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e`

```bash
git add web/src/sections.ts web/src/sections.test.ts web/src/Strips.tsx web/src/Strips.test.tsx web/src/Surfaces.tsx web/src/Surfaces.test.tsx web/src/App.tsx web/src/styles.css web/e2e/the_screen_itself.spec.ts
git commit -m "feat(web): W5a — the strips: the port, laboratory, tracks, goals and site boards in their own shapes

Six slots in a row under their zone or surcharge, the tracks with their points
as text, the goals landscape, the sites at full size — from the same payloads
the four drawers drew, which retire. Which sections are on is a per-browser
convenience with a default and a fallback."
```

---

### Task 12: W5b — the sections on the table: toggles, refresh, replay

**Files:**
- Create: `web/src/TableSections.tsx`, `web/src/TableSections.test.tsx`
- Modify: `web/src/App.tsx` (state, toggles in the banner, `<TableSections>` in the table), `web/e2e/the_table.spec.ts`, `web/e2e/the_screen_itself.spec.ts:511` (enable every section in the overflow cell)

**Interfaces:**
- Produces: `TableSections({ enabled, game, state, replaying, view, art, labels, colors })` → `div.table-sections > section.table-sec.<key>…`. `App` holds `enabled: SectionKey[]`.

- [ ] **Step 1: Write the failing tests**

Create `web/src/TableSections.test.tsx`:

```tsx
// The enabled sections: fetched when on, refetched after a move, collapsed
// while replaying, drawn from the frame where the frame already has them.

import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TableSections } from "./TableSections";
import { stubApi } from "./test/harness";
import { wire } from "./wire/contract";
import type { ClientState, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function show(enabled: Parameters<typeof TableSections>[0]["enabled"], state: ClientState = wire.options, replaying = false) {
  return render(
    <TableSections enabled={enabled} game={state.id} state={state} replaying={replaying} view={state} art={wire.artManifest} labels={wire.labels} colors={COLORS} />,
  );
}

describe("the sections on the table", () => {
  it("fetch only what is enabled, and draw the rest from the frame", async () => {
    const stub = stubApi((call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      throw new Error(`unexpected ${call.path}`);
    });
    show(["ports", "tracks", "achievements"]);
    await screen.findByText(/Port A/);
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    expect(stub.matching("GET", "/labs")).toHaveLength(0);
    expect(stub.matching("GET", "/areas")).toHaveLength(0);
    expect(document.querySelector(".table-sec.tracks")).not.toBeNull();
    expect(document.querySelector(".table-sec.achievements")).not.toBeNull();
    expect(document.querySelector(".table-sec.labs")).toBeNull();
  });

  it("re-fetch after the state changes, once per section", async () => {
    const stub = stubApi((call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      throw new Error(`unexpected ${call.path}`);
    });
    const view = show(["ports", "labs"]);
    await screen.findByText(/Port A/);
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    const next = { ...wire.options };
    view.rerender(
      <TableSections enabled={["ports", "labs"]} game={next.id} state={next} replaying={false} view={next} art={wire.artManifest} labels={wire.labels} colors={COLORS} />,
    );
    await waitFor(() => expect(stub.matching("GET", "/ports")).toHaveLength(2));
    expect(stub.matching("GET", "/labs")).toHaveLength(2);
  });

  it("collapse the fetched sections while replaying and fetch nothing", async () => {
    const stub = stubApi(() => {
      throw new Error("nothing may be fetched while replaying");
    });
    show(["ports", "tracks"], wire.options, true);
    expect(await screen.findByText(/hidden while replaying/)).toBeInTheDocument();
    expect(document.querySelector(".table-sec.tracks")).not.toBeNull();
    expect(stub.calls).toHaveLength(0);
  });

  it("carry a refusal as the sentence the server refused with", async () => {
    stubApi((call) =>
      call.path.endsWith("/ports") ? { status: 500, body: { error: "the port is closed" } } : undefined,
    );
    show(["ports"]);
    expect(await screen.findByText(/the port is closed/)).toBeInTheDocument();
  });
});
```

Add to `App.test.tsx`:

```tsx
describe("the section toggles", () => {
  it("are pressed for the default set, and a click turns one on and fetches it", async () => {
    const stub = await renderApp(wire.options, (call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      return undefined;
    });
    const user = userEvent.setup();
    expect(screen.getByRole("button", { name: "Port" })).toHaveAttribute("aria-pressed", "true");
    const sites = screen.getByRole("button", { name: "Building sites" });
    expect(sites).toHaveAttribute("aria-pressed", "false");
    expect(stub.matching("GET", "/areas")).toHaveLength(0);
    await user.click(sites);
    expect(sites).toHaveAttribute("aria-pressed", "true");
    await waitFor(() => expect(stub.matching("GET", "/areas")).toHaveLength(1));
    await user.click(sites);
    expect(sites).toHaveAttribute("aria-pressed", "false");
  });
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run src/TableSections.test.tsx src/App.test.tsx` — Expected: FAIL.

- [ ] **Step 3: Write `TableSections.tsx`**

```tsx
// The enabled sections under the pyramid (M5 spec §3.4).
//
// **Refresh on every state change.** The drawers fetched on demand because
// "none of them changes between decisions in a way a player is watching for";
// a section on the table is exactly that watching, so an enabled fetched
// section re-fetches whenever `state` changes — every `/input`, re-read and
// create — on the scrubber's own debounce-and-drop shape: a `live` flag, a
// stale answer dropped. Tracks and Goals need no fetch; they are on the frame.
//
// **Replaying collapses the fetched ones.** `/at/{k}` is a narrow frame (M4
// Q5): board and dashboard only. The port at position k is not something the
// server serves, so the section says so rather than showing the live offer
// under a historical board. Tracks and Goals draw from the historical frame.

import { useEffect, useState } from "react";
import { api, ApiError } from "./api";
import { SECTION_KEYS, SECTION_LABELS, type SectionKey } from "./sections";
import { GoalsStrip, LabStrip, PortStrip, SitesStrip, TracksStrip } from "./Strips";
import type { AreasPanel, ArtManifest, ClientState, HistoricalFrame, Labels, LabsPanel, PlayerColor, PortsPanel } from "./types";

const FETCHED: SectionKey[] = ["ports", "labs", "buildings"];

export function TableSections({
  enabled,
  game,
  state,
  replaying,
  view,
  art,
  labels,
  colors,
}: {
  enabled: SectionKey[];
  game: string;
  /** The live state — its identity is the refresh key. */
  state: ClientState;
  replaying: boolean;
  /** What the board is drawing: the live state, or the replay position. */
  view: ClientState | HistoricalFrame;
  art: ArtManifest | null;
  labels: Labels | null;
  colors: PlayerColor[];
}) {
  const [ports, setPorts] = useState<PortsPanel | null>(null);
  const [labs, setLabs] = useState<LabsPanel | null>(null);
  const [areas, setAreas] = useState<AreasPanel | null>(null);
  const [error, setError] = useState<string | null>(null);
  const wanted = enabled.join(",");

  useEffect(() => {
    if (replaying) return;
    let live = true;
    setError(null);
    const said = (e: unknown) => live && setError(e instanceof ApiError ? e.message : String(e));
    if (enabled.includes("ports")) api.ports(game).then((p) => live && setPorts(p)).catch(said);
    if (enabled.includes("labs")) api.labs(game).then((p) => live && setLabs(p)).catch(said);
    if (enabled.includes("buildings")) api.areas(game).then((p) => live && setAreas(p)).catch(said);
    return () => {
      live = false;
    };
  }, [game, state, replaying, wanted]);

  const tracksPanel = view.frame.dashboard.find((p) => "Tracks" in p);
  const tracks = tracksPanel && "Tracks" in tracksPanel ? tracksPanel.Tracks : null;
  const goalsPanel = view.frame.dashboard.find((p) => "Achievements" in p);
  const goals = goalsPanel && "Achievements" in goalsPanel ? goalsPanel.Achievements.cards : [];

  const collapsed = (key: SectionKey) => (
    <section className={`table-sec ${key} collapsed`} key={key} aria-label={SECTION_LABELS[key]}>
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">live offer, hidden while replaying</p>
    </section>
  );
  const loading = (key: SectionKey) => (
    <section className={`table-sec ${key}`} key={key} aria-label={SECTION_LABELS[key]}>
      <h2>{SECTION_LABELS[key]}</h2>
      <p className="muted">loading…</p>
    </section>
  );

  return (
    <div className="table-sections">
      {SECTION_KEYS.filter((k) => enabled.includes(k)).map((key) => {
        if (replaying && FETCHED.includes(key)) return collapsed(key);
        switch (key) {
          case "ports":
            return ports ? <PortStrip key={key} panel={ports} art={art} /> : loading(key);
          case "labs":
            return labs ? <LabStrip key={key} panel={labs} art={art} labels={labels} /> : loading(key);
          case "buildings":
            return areas ? <SitesStrip key={key} panel={areas} art={art} colors={colors} /> : loading(key);
          case "tracks":
            return tracks ? <TracksStrip key={key} tracks={tracks} art={art} colors={colors} /> : null;
          case "achievements":
            return <GoalsStrip key={key} cards={goals} art={art} labels={labels} colors={colors} />;
        }
      })}
      {error && <div className="error">{error}</div>}
    </div>
  );
}
```

`PortStrip`/`LabStrip` render fragments of one or two sections; a `key` on a fragment-returning component is fine.

- [ ] **Step 4: Wire it into `App.tsx`**

Add state and the toggle:

```tsx
  const [enabled, setEnabled] = useState<SectionKey[]>(() => loadSections());
  const toggleSection = (k: SectionKey) =>
    setEnabled((e) => {
      const next = e.includes(k) ? e.filter((x) => x !== k) : [...e, k];
      saveSections(next);
      return next;
    });
```

In the nav, before the drawer buttons, a disclosure in the `cards-menu` idiom — **not five inline buttons**: measured at M3.5, eleven inline buttons wrapped the banner to a second row below 1280 and `the_screen_itself.spec.ts:721` pins the nav's bottom edge under 60px at 1440/1280/1024; five more would wrap it again. The summary carries the count so the state is visible closed:

```tsx
          <details className="cards-menu boards-menu">
            <summary className="ghost">Boards · {enabled.length} on</summary>
            <div>
              {SECTION_KEYS.map((k) => (
                <button
                  type="button"
                  key={k}
                  className={`ghost ${enabled.includes(k) ? "on" : ""}`}
                  aria-pressed={enabled.includes(k)}
                  onClick={() => toggleSection(k)}
                >
                  {SECTION_LABELS[k]}
                </button>
              ))}
            </div>
          </details>
```

In the `App.test.tsx` cell above, open the disclosure before reading the toggles: `await user.click(screen.getByText(/^Boards/));` as the first interaction (jsdom renders a closed `<details>`' children; Testing Library still finds them, but the click keeps the test honest about the flow).

After `<Board …/>` inside `.table-body`:

```tsx
          <TableSections
            enabled={enabled}
            game={state.id}
            state={state}
            replaying={replaying}
            view={view}
            art={art}
            labels={labels}
            colors={colors}
          />
```

with the imports (`SECTION_KEYS`, `SECTION_LABELS`, `type SectionKey`, `loadSections`, `saveSections`, `TableSections`).

- [ ] **Step 5: The browser cells**

Append to `web/e2e/the_table.spec.ts`:

```ts
test("an enabled section re-fetches once after a move, collapses while replaying, and persists across a reload", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  const portRequests: string[] = [];
  page.on("request", (r) => {
    if (/\/ports$/.test(new URL(r.url()).pathname)) portRequests.push(r.url());
  });
  // Building sites is off by default; turn it on and see it fetched once.
  await page.locator(".boards-menu > summary").click();
  await page.getByRole("button", { name: "Building sites" }).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".table-sec.buildings .site-card").first()).toBeVisible();
  const before = portRequests.length;
  await playAFew(page, 1, 0x5245_4652); // "REFR"
  await expect.poll(() => portRequests.length, { message: "a move did not refresh the port" }).toBe(before + 1);

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await track.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator(".table-sec.ports.collapsed")).toBeVisible();
  await expect(page.locator(".table-sec.tracks")).toBeVisible();
  await page.getByRole("button", { name: "Back to live" }).click();
  await expect(page.locator(".table-sec.ports .board-strip")).toBeVisible();
  await page.keyboard.press("Escape");

  await page.reload();
  await page.getByRole("button", { name: /^sim-/ }).first().click();
  await expect(page.locator(".table-sec.buildings")).toBeVisible();
  await page.locator(".boards-menu > summary").click();
  await expect(page.getByRole("button", { name: "Building sites" })).toHaveAttribute("aria-pressed", "true");
  expect(watch.failures).toEqual([]);
});

test("the port and laboratory strips sit side by side at 2560 and stack at 1710", async ({ page }) => {
  await startGame(page, { players: 2 });
  const tops = () =>
    page.evaluate(() => ({
      port: document.querySelector(".table-sec.ports")!.getBoundingClientRect().top,
      lab: document.querySelector(".table-sec.labs")!.getBoundingClientRect().top,
    }));
  await page.setViewportSize({ width: 2560, height: 1440 });
  await settle(page);
  await expect.poll(async () => (await tops()).port === (await tops()).lab, { message: "not side by side at 2560" }).toBe(true);
  await page.setViewportSize({ width: 1710, height: 869 });
  await settle(page);
  const t = await tops();
  expect(t.lab, "not stacked at 1710").toBeGreaterThan(t.port);
});
```

Three edits in `the_screen_itself.spec.ts`:

1. In the overflow cell (`:511`), after `startGame` and before the moves loop, enable everything so the width gate sees the strips: `await page.locator(".boards-menu > summary").click(); await page.getByRole("button", { name: "Building sites" }).click(); await page.keyboard.press("Escape");`.
2. In the drawer-containment cell (`:632`), the label collection must skip the toggles, which open no drawer: change the selector to `.banner button.ghost:not([aria-pressed])`.
3. In the modality cell (`:721`), both `nav.querySelector<HTMLElement>("button")` and `document.querySelector<HTMLElement>("nav.views button")` become the DIRECT-child form (`"nav.views > button"` / `nav.querySelector(":scope > button")`) — the first descendant button is now a hidden toggle inside the closed Boards disclosure, whose `elementFromPoint` would read "not reachable" in both arms and pass the modality check vacuously while failing the live-again check.

- [ ] **Step 6: Gate, e2e, commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e`

```bash
git add web/src/TableSections.tsx web/src/TableSections.test.tsx web/src/App.tsx web/src/App.test.tsx web/e2e/the_table.spec.ts web/e2e/the_screen_itself.spec.ts
git commit -m "feat(web): W5b — the sections on the table: toggles, refresh after every move, collapsed while replaying

The banner's section buttons are pressed toggles persisted per browser; an
enabled fetched section refetches on every state change and says so while a
replay position is on the board, since /at/{k} carries no offer."
```

---

### Task 13: Gate V2 — the table, seen

**Files:**
- Modify: `web/e2e/gates.spec.ts` (add the V2 flow)

- [ ] **Step 1: Add the V2 shots**

Append to `gates.spec.ts`:

```ts
for (const players of [2, 4] as const) {
  test(`V2 — the table at ${players}P: all sections, offers only, a drawer, a replay, the menu`, async ({ page }) => {
    mkdirSync(OUT, { recursive: true });
    await page.setViewportSize({ width: 1710, height: 869 });
    await startGame(page, { players });
    await playAFew(page, 14, 0x5441_424c + players); // "TABL"
    const toggle = async (...names: string[]) => {
      await page.locator(".boards-menu > summary").click();
      for (const name of names) await page.getByRole("button", { name }).click();
      await page.keyboard.press("Escape");
    };
    await toggle("Building sites");
    await expect(page.locator(".table-sec.buildings")).toBeVisible();
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await raf(page);
      await page.screenshot({ path: `${OUT}/v2-${players}p-all-${w}x${h}.png` });
    }
    await page.setViewportSize({ width: 1710, height: 869 });
    await toggle("Tracks", "Goals", "Building sites");
    await raf(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-offers-only.png` });
    await page.getByRole("button", { name: "Areas", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.screenshot({ path: `${OUT}/v2-${players}p-drawer.png` });
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Log", exact: true }).click();
    const track = page.getByLabel("Replay position");
    await track.focus();
    for (let i = 0; i < 4; i++) await page.keyboard.press("ArrowLeft");
    await expect(page.locator(".table.replaying")).toBeVisible();
    await page.screenshot({ path: `${OUT}/v2-${players}p-replay.png` });
    await page.keyboard.press("Escape");
    await page.locator(".game-menu > summary").click();
    await page.screenshot({ path: `${OUT}/v2-${players}p-menu.png` });
  });
}
```

- [ ] **Step 2: Run, look, send, STOP**

Run: `cd web && npm run build && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts`. `Read` every V2 PNG. Compare the tile readings the Task 8 cell printed against the spec's §3.3 table; **any reading more than 0.5rem from its prediction makes this a stop regardless of how it looks.** Then `SendUserFile` the V2 set with the caption *"Gate V2 — the table with every section on at three viewports, offers only, a drawer over it, a replay position with the offers collapsed, and the Game menu, at 2P and 4P."* **STOP and wait for the owner.**

- [ ] **Step 3: Commit**

```bash
git add web/e2e/gates.spec.ts
git commit -m "test(web): gate V2 — the table's flows, shot for the owner"
```

---

### Task 14: W7 + W8 — the help, the pictures, and the documents that describe them; gate V3

**Files:**
- Modify: `web/src/Help.tsx:43-139`, `web/e2e/shots.spec.ts` (viewport, captions, the placement walk, the replay shot), `web/public/help/*.png` (regenerated), `README.md:199-260` (the browser-client section), `ASSETS.md` (the `web/public/help/` row's count and size)

- [ ] **Step 1: Re-word the help**

Replace the "three parts" section and its shot, and the "Looking things up" and "Replaying" sections, in `Help.tsx`:

```tsx
      <h3>The three parts of the screen</h3>
      <p>
        The <strong>table</strong> fills the left: the area tiles laid out as the staggered pyramid
        the real board makes, each one the printed tile with its building-site card on it, its
        pieces standing on it, and the 5-Power token in its printed slot &mdash; and under the
        pyramid, whichever boards you have switched on along the top: the port with its six
        cards, the laboratory with its six, the church and customs tracks, the goals, the
        building sites at full size. On a wide screen they sit side by side; on a laptop the
        table scrolls. The <strong>side column</strong> on the right holds the{" "}
        <strong>answer bar</strong> pinned at its top &mdash; whatever the game is asking you is
        always there &mdash; then what the last move did, every seat&rsquo;s goods, money and
        pieces, and your own hand. Hover or focus any tile to see it, and its site card, at full
        size.
      </p>
      <Shot
        src="/help/screen.png"
        alt="The whole client at two players: the pyramid of area tiles on the left with the port and laboratory strips under it, and the side column on the right with the answer bar pinned at its top above the last move, both seats and the hand."
        caption="The table on the left, the side column on the right, the answer bar pinned at its top."
      />
```

```tsx
      <h3>Looking things up</h3>
      <p>
        The first five buttons along the top switch boards on and off on the table: the port,
        the laboratory, the tracks, the goals and the building sites. What you switch on stays on
        in this browser. The rest open read-only panels over the game &mdash; what every area
        pays at each power level, the log, the bot&rsquo;s advice, and behind <em>Cards</em> the
        four full indexes: all 36 orders, all 20 technologies, the achievements in play, and the
        24 printed sites, each saying where every copy currently is. A panel reads the game and
        closes again; the game waits underneath it.
      </p>
```

and in "Replaying the game", after "outlined to show you are looking at the past.", add: *"The port and laboratory on the table say they are hidden while you are in the past: the position you scrub to has no offer the server can show, so the table does not pretend."* Update the replay shot's alt to name the table.

- [ ] **Step 2: Regenerate the shots**

In `shots.spec.ts` set both tests' viewport to `{ width: 1710, height: 869 }` (the owner's), and make the first test switch on Building sites before the whole-screen shot (`await page.locator(".boards-menu > summary").click(); await page.getByRole("button", { name: "Building sites" }).click(); await page.keyboard.press("Escape");`) so the table shows a section under the pyramid at that height. Run:

```bash
cd web && npm run build && YOKOHAMA_WEB_SHOTS=1 npx playwright test shots.spec.ts
```

`Read` all four PNGs in `web/public/help/`. Each caption must describe what its picture shows.

- [ ] **Step 3: README and ASSETS**

In `README.md`'s browser-client section (`:199-260`) re-word the paragraph that describes the layout (the nav of reference panels along the top, the rail) to the table and the side column, and re-check the screenshot it embeds (`:206`) is one of the regenerated four. In `ASSETS.md`, re-count the `web/public/help/` row (`4 .png, ~2 MB` → measured).

- [ ] **Step 4: Gate V3 — send, STOP**

`SendUserFile` the four help PNGs with the caption *"Gate V3 — the regenerated help screenshots, which are what a reader is told the product looks like."* **STOP and wait for the owner.**

- [ ] **Step 5: Gate, commit**

Run: `cd web && npm test && npm run typecheck && npm run e2e`

```bash
git add web/src/Help.tsx web/e2e/shots.spec.ts web/public/help README.md ASSETS.md
git commit -m "docs(web): W7/W8 — the help says table and side column, and its pictures show them"
```

---

### Task 15: W9 — deploy to the NAS

**Files:** none in the repo beyond `deploy/RUNBOOK.md` §3.3/§3.4 readings.

- [ ] **Step 1: Build**

Follow `deploy/RUNBOOK.md` §1 exactly:

```bash
LLD="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin/rust-lld"
RUSTFLAGS="-C linker=$LLD -C link-self-contained=yes" cargo build --release --target x86_64-unknown-linux-musl -p yokohama-web
cd web && npm run build && cd ..
```

- [ ] **Step 2: Deliver and run**

§2 and §3 of the runbook, with the image tag **`yokohama-web:0.5.0-m5`** — the crate version is the workspace's release train (`Cargo.toml:14`, `0.5.0`) and a client-only milestone does not move it; the spec's "0.6.0" is corrected in the close. Recreate the container with the same `HostConfig` (network `cf-internal`, the named volume `yokohama-web-data`, restart `unless-stopped`). No tunnel change: the hostname already exists.

- [ ] **Step 3: Verify through the tunnel**

```bash
curl -sS https://yokohama.moore.nyc/api/health
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/art/areas/Church.webp
curl -sS -o /dev/null -w '%{http_code}\n' https://yokohama.moore.nyc/help/screen.png
```

Then open `https://yokohama.moore.nyc` in Claude-in-Chrome (it carries the Access cookie), open the owner's saved game read-only, screenshot at the owner's viewport, and `SendUserFile` it with the caption *"Live at yokohama.moore.nyc: M5 on your game."* The owner's game is not played on.

- [ ] **Step 4: Record**

Add the tag, the image size and the date to `deploy/RUNBOOK.md` §3 as one line under the container step (*"M5 redeployed <date> as `yokohama-web:0.5.0-m5`, <measured> MB"*, with the real date and the image size the build printed). Commit with `docs(deploy): M5 is live`.

---

### Task 16: The close

**Files:**
- Modify: `docs/plans/2026-09-01-web-frontend-m5-spec.md` (append §8), `guidingpath.md` (one decision-log row), `CLAUDE.md:30-34` (counts), the memory file `yokohama-web-lane-status.md`

- [ ] **Step 1: Re-measure**

```bash
cd web && npm test 2>&1 | tail -3 && npm run e2e 2>&1 | tail -3 && cd .. && cargo test -p yokohama-web 2>&1 | grep "test result" && ls web/src/wire/*.json | wc -l && git diff --stat master..HEAD -- crates/ && grep -n "SEMANTICS_REV" crates/yokohama-core/src/game/semantics.rs | head -2
```

Record: vitest count, e2e count, cargo count, wire count; confirm `crates/` shows only `yokohama-web/assets/art/**`, `manifest.json` and `art.rs`; `SEMANTICS_REV` 7.

- [ ] **Step 2: Append §8 to the spec**

Write `## 8. The close (<the date it lands>) — what shipped, and where the spec was wrong` with: every W item and its verdict; the nine fit readings from Task 8 beside §3.3's predictions, machine-tagged; the payload delta; the deviations listed at the end of this plan, each with its reason; the tag correction (0.5.0-m5); the crates/ diff statement; what remains not claimed (§5 restated). Commit as `docs(web): the M5 close`.

- [ ] **Step 3: The decision-log row and the counts**

Append one row to `guidingpath.md`'s decision log in the form of the M3.5 row at `:206`: date, **M5 — "THE TABLE" — LANDED ON THE WEB LANE**, the rulings R1–R11 in one paragraph, what the board gained (the four rows in view at the owner's viewport, measured), what moved (deck → side column; four drawers → strips; caption → art), the re-measured counts, "still no gate and no roster place", and in the Why column the one lesson worth carrying. Update `CLAUDE.md:30-34`'s counts and its `web/` per-edit line to name `the_table.spec.ts`, `gates.spec.ts` (opt-in) and the sections. Update the memory file's status line.

- [ ] **Step 4: Commit and report**

```bash
git add docs/plans/2026-09-01-web-frontend-m5-spec.md guidingpath.md CLAUDE.md
git commit -m "docs: M5 closes — the counts, the readings beside the predictions, the decision-log row"
```

Report to the owner: the readings table, the counts, the deviations, the live URL, and what M6 inherits.

---

## Deviations from the spec, decided while planning (to be recorded in the close)

1. **Order.** W3 (the fit rule) runs AFTER W2 (the grid): the rule is a property of the `.table` size container, which does not exist until the grid does. Spec §4 said W3 before W2.
2. **The popover shows the art, not the ladder in words.** `TileModel` carries no yields; the words live in `/areas`. The popover draws the tile art at 22rem, where the printed ladder is legible, plus the site card and the pieces in words. The ladder-in-words stays the Areas drawer's (§3.2.2 said `yieldWords`).
3. **Areas stays a drawer.** It is a reference, and the tile art carries the ladder. §1 R3 read "everything else becomes a section"; the plan keeps Areas, Log, Suggest, Why, Help, Final score and the four indexes as drawers.
4. **Goals is on by default.** `DEFAULT_SECTIONS` is `ports, labs, tracks, achievements` (§3.4 said port, laboratory, tracks) — without it the achievements would be nowhere on screen by default.
5. **The sections render at every width.** Below 1201 they flow under the board in the column stack; the deck and the stack are unchanged (R8), but "unchanged" is narrowed to the layout mechanism, not the content.
6. **Cards are sized from the table, not the tile.** `--card-w: clamp(8rem, 12cqw, 12rem)` — keying to `--tile-w` would need Board's shape on the sections' ancestor. Same growth, one fewer dependency.
7. **The Board.test ladder cell is kept, renamed** (`--tile-ladder`), because the ladder still serves below 1201; the fit oracle cells are added beside it rather than replacing it (§4 W3 said "replaced").
8. **The image tag is `0.5.0-m5`**, not `0.6.0`: the crate version is the workspace's.
9. **The `crates/` diff statement** was corrected in the spec itself before this plan (§4).
10. **No `SiteSlots` words on the tile**: the site's four space rewards are on the printed card the tile now draws and in the popover/strip; `.sr-only` carries the id, the token and `Site n`, not the per-space rewards.
11. **The section toggles sit behind a "Boards" disclosure**, not inline in the banner (§3.4 said "the five sections become toggles" in the banner): five more inline buttons would wrap the banner below 1280 and fail the one-row cell M3.5 pinned. The summary shows how many are on; toggling is a two-click act done once per browser.
