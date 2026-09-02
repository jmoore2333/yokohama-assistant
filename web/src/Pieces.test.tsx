// The pieces standing on a tile, drawn as pieces and named in words.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Pieces, piecesWords } from "./Pieces";
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

  it("draw one cube and a count from four", () => {
    const { container } = render(
      <Pieces presidents={[]} assistants={[[0, 4], [1, 0]]} hasStation={false} colors={COLORS} art={wire.artManifest} />,
    );
    expect(container.querySelectorAll(".piece.cube")).toHaveLength(1);
    expect(container.querySelector(".piece-n")?.textContent).toBe("4");
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
});
