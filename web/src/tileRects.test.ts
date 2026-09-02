// The overlay geometry: the three rects an area tile draws its pieces in
// (`site_card`, `pieces`, `id_badge`) and the four printed regions none of them
// may touch (`token_slot`, `ladder_band`, `title_band`, `icon_block`).

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
