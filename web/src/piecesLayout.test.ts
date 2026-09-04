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
