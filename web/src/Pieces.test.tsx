// The pieces standing on a tile, drawn as pieces and named in words.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Pieces, piecesWords } from "./Pieces";
import { bandAspect, layoutPieces } from "./piecesLayout";
import { PIECES } from "./tileRects";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

describe("the pieces on a tile", () => {
  it("say who stands there, in the words a screen reader gets", () => {
    expect(piecesWords([0], [[0, 0], [1, 2]], false, COLORS)).toBe("Red president · 2 Blue assistants");
    expect(piecesWords([], [[0, 1], [1, 0]], true, COLORS)).toBe("1 Red assistant · Station");
    expect(piecesWords([], [[0, 0], [1, 0]], false, COLORS)).toBe("");
  });

  it("draw a pawn per president and a cube per assistant up to three", () => {
    const { container } = render(
      <Pieces presidents={[0]} assistants={[[0, 0], [1, 3]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelectorAll(".piece.president")).toHaveLength(1);
    expect(container.querySelectorAll(".piece.cube")).toHaveLength(3);
    expect(container.querySelector(".piece-n")).toBeNull();
    expect(container.querySelector(".pieces")?.getAttribute("aria-label")).toBe(
      "Red president · 3 Blue assistants",
    );
  });

  // From four a colour is a PILE: two rows, the odd cube on the top row, and
  // no count — the M5.5 rule (spec §3.1.1) that replaces "one cube and a 4".
  it("draw four as a two-row pile with no count", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 4], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    const pile = container.querySelector(".piece-group.pile");
    expect(pile).not.toBeNull();
    const rows = pile!.querySelectorAll(".pile-row");
    expect(rows).toHaveLength(2);
    expect(rows[0]!.querySelectorAll(".piece.cube")).toHaveLength(2);
    expect(rows[1]!.querySelectorAll(".piece.cube")).toHaveLength(2);
    expect(container.querySelector(".piece-n")).toBeNull();
    expect(container.querySelector(".pieces")?.getAttribute("aria-label")).toBe("4 Red assistants");
  });

  it("put the odd cube on the top row", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 5], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    const rows = container.querySelectorAll(".pile-row");
    expect(rows[0]!.querySelectorAll(".piece.cube")).toHaveLength(3);
    expect(rows[1]!.querySelectorAll(".piece.cube")).toHaveLength(2);
  });

  // The 17-glyph 4P crowd is the one the band cannot hold at the floor. The
  // rule collapses the largest group, recomputes, and repeats until the glyph
  // height reaches it — so HOW MANY collapse is a function of the BAND'S WIDTH
  // and not a constant: against the rect at this commit it takes all four
  // colours (units 8.96, h 0.569), and against the wider band Task 3 measures
  // (left ≈ 0.35, aspect ≈ 6.73) it takes two, the other two drawing their
  // three cubes. So the cell asserts neither number. The decision is
  // `layoutPieces`'s: the cell reads ITS answer and checks the component drew
  // that — which colour is a count, which is a pile, and that every other one
  // is loose cubes — which is obedience to the rule at whatever width the band
  // ends up, rather than the component re-deciding.
  it("collapse to one cube and a count only where the layout says so", () => {
    const { container } = render(
      <Pieces
        presidents={[0, 1, 2, 3]}
        assistants={[[0, 3], [1, 3], [2, 3], [3, 3]]}
        hasStation
        colors={["Red", "Blue", "Yellow", "Green"]}
        art={wire.artManifest}
      />,
    );
    const expected = layoutPieces({ presidents: 4, cubes: [3, 3, 3, 3], station: true }, bandAspect(PIECES));
    expect(expected.collapsed.length, "the rule must collapse something for this cell to mean anything").toBeGreaterThan(0);
    expect(container.querySelectorAll(".piece-n")).toHaveLength(expected.collapsed.length);
    expect(container.querySelectorAll(".piece.cube")).toHaveLength(
      expected.collapsed.length + (4 - expected.collapsed.length) * 3,
    );
    expect(container.querySelector(".piece-group.pile")).toBeNull();
    // The MAPPING, not just the totals: the component draws a group per
    // president and then one per colour, so the colours are what follows the
    // four pawns, and each one has to be the shape the layout named for ITS
    // index. Counting alone cannot see two colours swapping shapes.
    const all = [...container.querySelectorAll(".piece-group")];
    expect(all, "four presidents, then four colours").toHaveLength(8);
    const colours = all.slice(4);
    const ascending = (a: number[]) => [...a].sort((x, y) => x - y);
    const indicesWhere = (p: (el: Element) => boolean) => colours.flatMap((el, i) => (p(el) ? [i] : []));
    expect(indicesWhere((el) => el.querySelector(".piece-n") !== null)).toEqual(ascending(expected.collapsed));
    expect(indicesWhere((el) => el.classList.contains("pile"))).toEqual(ascending(expected.piles));
    colours.forEach((el, i) => {
      if (expected.collapsed.includes(i) || expected.piles.includes(i)) return;
      expect(el.querySelectorAll(".piece.cube"), `colour ${i} draws its three cubes loose`).toHaveLength(3);
    });
  });

  // An empty tile draws nothing, exactly as the table does — no "no pawns".
  it("draw nothing at all on an empty tile", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 0], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelector(".pieces")).toBeNull();
  });

  it("colour a piece by its seat", () => {
    const { container } = render(
      <Pieces presidents={[1]} assistants={[[0, 0], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelector(".piece-group")?.className).toMatch(/seat-blue/);
  });

  it("draw the Station token's art where there is art, and a chip where there is none", () => {
    const withArt = render(
      <Pieces presidents={[]} assistants={[]} hasStation colors={COLORS} art={wire.artManifest} />,
    );
    expect(withArt.container.querySelector("img.piece.station")).not.toBeNull();
    const without = render(<Pieces presidents={[]} assistants={[]} hasStation colors={COLORS} art={null} />);
    expect(without.container.querySelector(".chip.station")?.textContent).toBe("St");
  });

  // The band's CSS divides its width by the row's width in glyph heights, and
  // the number it is given is the layout's own `units`, to three decimals.
  it("tell the band the row's width in units of the glyph height", () => {
    const units = (el: HTMLElement) =>
      el.querySelector<HTMLElement>(".pieces")!.style.getPropertyValue("--piece-units");
    const aspect = bandAspect(PIECES);
    const one = render(<Pieces presidents={[0]} assistants={[[0, 0], [1, 3]]} hasStation={false} colors={COLORS} art={wire.artManifest} />);
    expect(units(one.container)).toBe(layoutPieces({ presidents: 1, cubes: [3], station: false }, aspect).units.toFixed(3));
    const pile = render(<Pieces presidents={[]} assistants={[[0, 4], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />);
    expect(units(pile.container)).toBe(layoutPieces({ presidents: 0, cubes: [4], station: false }, aspect).units.toFixed(3));
    const many = render(<Pieces presidents={[0, 1]} assistants={[[0, 2], [1, 1]]} hasStation colors={COLORS} art={wire.artManifest} />);
    expect(units(many.container)).toBe(layoutPieces({ presidents: 2, cubes: [2, 1], station: true }, aspect).units.toFixed(3));
  });
});
