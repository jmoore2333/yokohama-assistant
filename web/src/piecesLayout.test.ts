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
    // The glyph height IS the band's aspect over the row's units — the identity
    // the stylesheet's shrink-to-fit rule restates on its own side. Scaling the
    // module's `aspect / u` by 0.99 passes every other cell in this file.
    expect(c4.h).toBeCloseTo(MOCK / c4.units, 12);
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
    // The crowd above collapses EVERY group, so it exits through the
    // floor-missed return and never exercises the filter. This one collapses a
    // pile while two singles survive and comes back through the floor-MET
    // return: deleting the module's `.filter(...)` leaves every other cell in
    // this file green and reads `piles: [0]` here.
    const mixed = layoutPieces({ presidents: 4, cubes: [9, 3, 3, 3], station: true }, narrow);
    expect(mixed.collapsed).toEqual([0, 1, 2]);
    expect(mixed.piles).toEqual([]);
    expect(mixed.piles.filter((i) => mixed.collapsed.includes(i))).toEqual([]);
  });

  // R-T1-1: "largest group first" by CUBE COUNT can WIDEN the row. A four-cube
  // pile costs 1.00 units drawn (two columns of 0.5) and 1.13 collapsed (cube +
  // inner + one digit), so collapsing it moves the row the wrong way — and a
  // collapse that widens the row can never bring the height up to the floor.
  // The plan's largest-first code collapsed index 0 here (units 12.22 -> 12.35,
  // h 0.5419 -> 0.5362, FURTHER from the floor), then index 1, and returned
  // `collapsed: [0, 1]`. The rule skips index 0 and collapses index 1 alone.
  it("never collapses a group that would widen the row", () => {
    const band = bandAspect({ width: 0.61, height: 0.123 });
    const wide = layoutPieces({ presidents: 4, cubes: [4, 3, 3, 3], station: true }, band);
    expect(wide.piles).toEqual([0]);
    expect(wide.collapsed).toEqual([1]);
    expect(wide.h).toBeGreaterThanOrEqual(FLOOR);
    expect(wide.h).toBeCloseTo(band / wide.units, 12);
    expect(wide.piles.filter((i) => wide.collapsed.includes(i))).toEqual([]);
  });

  it("reports the floor missed when no group is left to collapse", () => {
    const hopeless = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, 2);
    expect(hopeless.collapsed).toEqual([0, 1, 2, 3]);
    expect(hopeless.h).toBeLessThan(FLOOR);
    expect(hopeless.h).toBeGreaterThan(0);
  });

  // The same exit, reached with a pile still standing — which only R-T1-1 makes
  // reachable: before it, no candidate left meant every group had collapsed, so
  // hardcoding `piles: []` there was right by construction. Now a lone pile can
  // be the group the rule refuses to collapse, and `units` prices it AS a pile
  // (1.00, not the 3.03 four loose cubes cost), so the pile has to be reported.
  it("keeps a pile it could not collapse when the floor is missed", () => {
    const lone = layoutPieces({ presidents: 0, cubes: [4], station: false }, 0.5);
    expect(lone.collapsed).toEqual([]);
    expect(lone.piles).toEqual([0]);
    expect(lone.units).toBeCloseTo(2 * UNIT.pileColumn, 6);
    expect(lone.h).toBeLessThan(FLOOR);
    expect(lone.h).toBeCloseTo(0.5 / lone.units, 12);
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
