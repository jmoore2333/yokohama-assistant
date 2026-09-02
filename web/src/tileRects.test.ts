// The overlay geometry: three rects an area tile draws its pieces in, and the
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

  // The site card is a landscape 799x530 card, so its box on a 709x531 tile
  // must be 799:530 or the printed spaces stretch.
  it("the site card keeps the printed card's aspect on its side", () => {
    const boxAspect = (SITE_CARD.width * 709) / (SITE_CARD.height * 531);
    expect(Math.abs(boxAspect - 799 / 530)).toBeLessThan(0.02);
  });

  // Mutation proof for the cell above it: a pieces band nudged down and right
  // onto the 5-space IS an intersection, so the guard is not vacuous. (Both
  // axes move so the proof survives a future re-widen of the band: after the
  // V0 re-fit `top` alone already reaches the token slot on the committed
  // rects, but the mutant does not rely on that coincidence.)
  it("would catch a band that drifts onto the 5-space", () => {
    expect(intersects({ ...PIECES, top: 0.5, left: 0.6 }, TOKEN_SLOT)).toBe(true);
  });

  it("renders a rect as inline percentages", () => {
    expect(pct(ID_BADGE)).toEqual({ left: "45.75%", top: "91.50%", width: "8.50%", height: "7.00%" });
  });
});
