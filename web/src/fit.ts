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
