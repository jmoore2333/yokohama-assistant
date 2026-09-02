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

/** Whether `rows` rows of tiles `tileW` wide are IN VIEW within `tableH`.
 *
 *  **A claim about the PYRAMID, never about `.table`'s scroll** (controller
 *  ruling R-T8a). `.table` holds more than the pyramid — the marks legend and
 *  `.board`'s own `padding-bottom` sit under it today, and Task 12's sections
 *  will sit under it tomorrow — so the table scrolling is the design (owner
 *  ruling R2) and not a failure of this rule. Measured on the owner's screen at
 *  1710x869 with `.table` 785px tall: all four rows in view at every player
 *  count, while the table's own `scrollHeight` runs 825 at 2P and 794 at 3P.
 *  What this function answers, and all it answers, is whether the ROWS fit —
 *  which is what spec §3.3 claims and what the browser cell branches on.
 *
 *  Neither this nor `tileWidthFor` accounts for what is below the pyramid, and
 *  that is deliberate: adding ~1.4rem to `PAD_REM` for a collapsed legend line
 *  would shrink every tile by ~0.3rem to buy back nothing the owner asked for.
 *
 *  **The `+ 0.5` is a half-pixel tolerance, and it is LOAD-BEARING** — not
 *  slack anyone added to be safe. The rows are laid out by the browser at
 *  sub-pixel precision from a tile width the CSS derived with its own rounding,
 *  so an exact `<=` against a whole-pixel container height decides a tie on
 *  noise. The cell above proves the case: `pyramidFits(245, 4, 785)` computes
 *  785.163329 against 785 and is TRUE only because of this term — and it is
 *  true because those four rows really are in view, measured. Removing the
 *  term flips that cell and, with it, which branch the browser cell takes on
 *  the owner's own screen.
 */
export function pyramidFits(tileW: number, rows: number, tableH: number, remPx = 16): boolean {
  return (tileW / TILE_RATIO) * rows + (rows - 1) * GAP_REM * remPx + PAD_REM * remPx <= tableH + 0.5;
}
