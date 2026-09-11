// The overlay geometry: the three rects an area tile draws its pieces in
// (`site_card`, `pieces`, `id_badge`) and the four printed regions
// (`token_slot`, `ladder_band`, `title_band`, `icon_block`). Every overlay must
// stay off every printed region, with ONE measured exception: the pieces band
// enters the icon block, whose rect reaches y 0.58 while its printed ink stops
// at 0.395 (M5.5 ruling R6 — the rect may be entered, the ink may not).

import { describe, expect, it } from "vitest";
import { BAND_MARGIN, ICON_INK_RIGHT, ID_BADGE, inside, intersects, OVERLAYS, PIECES, pct, PRINTED, SITE_CARD, TOKEN_SLOT } from "./tileRects";

describe("the overlay geometry", () => {
  it("every overlay lies inside the tile", () => {
    for (const [name, r] of Object.entries(OVERLAYS)) {
      expect(inside(r), `${name} leaves the tile`).toBe(true);
    }
  });

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

  // **Named for what it can check, which is not the ink** (final review m6).
  // Mutation M3 set `ICON_INK_RIGHT` to 0 and left this file green 8 of 8, and
  // it always will: the constant is the eye's reading of the printed art
  // (ruling R-T3-1) and no assertion in a test runner can re-take it. What the
  // cell holds is the arithmetic BETWEEN the constant and the band, which is
  // the half a refit of either would break — and the old name promised the
  // other half.
  it("the band starts right of ICON_INK_RIGHT by the margin (the constant is the eye's reading, R-T3-1; this pins the arithmetic, not the ink)", () => {
    expect(PIECES.left).toBeGreaterThanOrEqual(ICON_INK_RIGHT + BAND_MARGIN - 1e-9);
    expect(PIECES.left, "the band was widened past the mock's own 0.30 for nothing").toBeLessThan(0.5);
    expect(PIECES.left + PIECES.width).toBeCloseTo(0.97, 6);
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

  // Mutation proof for "no overlay covers a printed rule" — the TOKEN_SLOT is a
  // printed region, not an overlay, so this is not the neighbouring cell's
  // proof. A pieces band nudged down and right onto the 5-space IS an
  // intersection, so the guard is not vacuous. (Both axes move so the proof
  // survives a future re-widen of the band: after the V0 re-fit `top` alone
  // already reaches the token slot on the committed rects, but the mutant does
  // not rely on that coincidence.)
  it("would catch a band that drifts onto the 5-space", () => {
    expect(intersects({ ...PIECES, top: 0.5, left: 0.6 }, TOKEN_SLOT)).toBe(true);
  });

  // **The fourth clause of `intersects` — `b.top < a.top + a.height` — named,
  // with the pair whose separation is that clause and nothing else.** The
  // pieces band sits directly ABOVE the 5-space and shares its columns, so the
  // other three terms are all true and only this one keeps them apart: the
  // band's bottom is 0.585 against the slot's top of 0.588, three thousandths
  // of a tile. The review that asked for this expected the clause to be
  // unexercised; it is not — replacing it with `true` fails this cell AND the
  // two above it (measured) — so what this adds is the NAME of the margin, and
  // a second arm one nudge away so the cell cannot pass by answering "false"
  // to both.
  it("keeps the pieces band clear of the 5-space by three thousandths of a tile", () => {
    expect(intersects(PIECES, TOKEN_SLOT)).toBe(false);
    expect(intersects({ ...PIECES, top: PIECES.top + 0.01 }, TOKEN_SLOT)).toBe(true);
  });

  it("renders a rect as inline percentages", () => {
    expect(pct(ID_BADGE)).toEqual({ left: "45.75%", top: "91.50%", width: "8.50%", height: "7.00%" });
  });
});
