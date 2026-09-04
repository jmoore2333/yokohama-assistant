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
//
// SITE_CARD is the tiles' own printed top-right panel — not a box we chose,
// but the pale ground the illustration's silhouette already leaves bare up
// there for a physical card to sit on. Measured 2026-09-02 (gate V0 owner
// answer, R-G1): per art, the largest 4-connected HSV-pale region (V>0.784,
// S<0.18) within x>0.45,y<0.62, intersected across all 13; the intersection's
// own left edge is a search-window artifact narrower than the icon block's
// real right edge, so it is floored at 0.5. That leaves left=0.5, top≈0.032,
// right≈0.975, bottom≈0.495 (Laboratory's tower is the tightest art on both
// of the last two); SITE_CARD is the largest 799:530 box that fits inside it,
// anchored top-right — and it holds a LANDSCAPE 799x530 card, since W0b turned
// the site crops upright to match the printing.
//
// PIECES then runs from the measured ink edge to the card's right edge, packed
// right, and its glyphs shrink to fit
// (owner ruling R-G3, 2026-09-02, on the first tile picture: "three or four
// assistants from multiple colors is a real probability, especially in games
// over 2p but even in 2p games"). That displaced ID_BADGE, which now sits in
// the tile's bottom margin, centred under the ladder — the one unprinted strip
// left, since bottom-right carries Port and Dock's HIGH ★3 bonus and
// bottom-left the tiny printed area name.

import rects from "./tile_rects.json";

export interface Rect {
  left: number;
  top: number;
  width: number;
  height: number;
}

// No `Record<string, Rect>` annotation on either: it WIDENS the JSON import's
// key literals to `string`, and those literals are what let `SITE_CARD` and the
// three below be named rather than looked up. The shape is checked by the uses.
export const PRINTED = rects.printed;
export const OVERLAYS = rects.overlays;
export const SITE_CARD: Rect = rects.overlays.site_card;
export const PIECES: Rect = rects.overlays.pieces;
export const ID_BADGE: Rect = rects.overlays.id_badge;
export const TOKEN_SLOT: Rect = rects.printed.token_slot;

/** The rightmost printed-icon ink inside the pieces band's rows — the
 *  EYE-CONFIRMED reading, measured 2026-09-04 with `docs/tools/tilerects.py
 *  --ink`; the band's left edge is this plus the margin. The band deliberately
 *  ENTERS the icon-block rect (M5.5 spec R6): what it must not cover is ink,
 *  and the illustration is not data.
 *
 *  The tool's raw run does NOT give you this number. Its pale-run rule reads
 *  illustration as ink on NINE of the thirteen arts — PortA's band rows are
 *  55/66 pale sky, so its dark shapes score as ink on the panel — and reads up
 *  to 0.468 there. Printed icons reach these rows on exactly FOUR arts, the
 *  same four the icon-block note above names: Chinatown 0.320, Church 0.379,
 *  Laboratory 0.395, Research Center 0.392. LABORATORY BINDS. Each was
 *  confirmed against a 0.005 gridline zoom, and the band was then looked at on
 *  all thirteen arts (ruled during execution, recorded in the M5.5 close). */
export const ICON_INK_RIGHT = 0.395;
export const BAND_MARGIN = 0.02;

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
