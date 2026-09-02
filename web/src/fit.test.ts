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
