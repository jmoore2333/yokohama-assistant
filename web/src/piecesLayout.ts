// How a crowd of pieces is drawn in a tile's band (M5.5 spec §3.1.1).
//
// Every cube is drawn while the row stays legible; a colour with four or more
// becomes a two-row PILE; a group collapses to one cube and a count only when
// the band cannot hold the crowd at the floor, and only when that collapse
// NARROWS the row — a four-cube pile costs 1.00 units drawn and 1.13 collapsed,
// so collapsing it moves the row the wrong way, and a collapse that widens the
// row can never bring the glyph height up to the floor (ruling R-T1-1). The
// decision is made HERE, as a pure function of the crowd and the band's shape,
// and the CSS is handed the row's width in units of the glyph height
// (`--piece-units`) — so the stylesheet's shrink-to-fit rule and this function
// agree by construction: `h = min(H, W / units)` on both sides.
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
  // What is actually DRAWN as a pile: a collapsed group is one cube and a
  // count, never a pile. Both returns below use this — including the
  // floor-missed one, where under R-T1-1 a pile can survive uncollapsed
  // (a collapse that widens the row is never taken), so reporting no piles
  // there would price a pile in `units` and then draw it as loose cubes.
  const drawnPiles = () => piles.filter((i) => !collapsed.includes(i));
  for (;;) {
    const u = units();
    const h = Math.min(1, aspect / u);
    if (h >= floor) {
      return { h, units: u, piles: drawnPiles(), collapsed };
    }
    // The largest group not yet collapsed, ties to the first — among only
    // those a collapse would NARROW (R-T1-1). None qualifying means the floor
    // cannot be met and the caller gets the honest h.
    let largest = -1;
    crowd.cubes.forEach((n, i) => {
      if (collapsed.includes(i)) return;
      const pile = piles.includes(i);
      if (groupWidth(n, pile, true) >= groupWidth(n, pile, false)) return;
      if (largest < 0 || n > crowd.cubes[largest]!) largest = i;
    });
    if (largest < 0) return { h, units: u, piles: drawnPiles(), collapsed };
    collapsed.push(largest);
  }
}
