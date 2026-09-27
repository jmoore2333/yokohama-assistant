// The door's board (M6.5c2, spec §2.2): an ORIGINAL drawing, and the one
// picture on the setup screens.
//
// What a cell can hold here is the drawing's STRUCTURE — the two-player
// pyramid, the pieces on it, that it is decoration and not an image — and that
// every colour it names is a rule in the stylesheet. What it LOOKS like is the
// headed look's (Task 3's `setup_screens.spec.ts` screenshots): jsdom never
// loads `styles.css`.

import { readFileSync } from "node:fs";
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { BoardMark, TILES } from "./BoardMark";

describe("the board mark", () => {
  it("draws the ten tiles of a two-player board in its 4/3/2/1 pyramid", () => {
    const { container } = render(<BoardMark width={360} />);
    const perRow = [0, 1, 2, 3].map(
      (row) => container.querySelectorAll(`.bm-tile[data-row="${row}"]`).length,
    );
    expect(perRow).toEqual([4, 3, 2, 1]);
    expect(container.querySelectorAll(".bm-tile")).toHaveLength(TILES.length);
  });

  // Brick-laid by centring, the way `board_gen` lays a two-player board: each
  // row starts half a tile further in than the one above it.
  it("offsets each row by half a tile", () => {
    const { container } = render(<BoardMark width={360} />);
    const firstX = [0, 1, 2, 3].map((row) =>
      Number(container.querySelector(`.bm-tile[data-row="${row}"]`)!.getAttribute("x")),
    );
    expect(firstX).toEqual([0, 54, 108, 162]);
  });

  it("puts two presidents and five assistants on it, in two seats' colours", () => {
    const { container } = render(<BoardMark width={360} />);
    expect(container.querySelectorAll(".bm-president")).toHaveLength(2);
    expect(container.querySelectorAll(".bm-cube")).toHaveLength(5);
    const seats = new Set(
      [...container.querySelectorAll(".bm-president, .bm-cube")].map(
        (el) => [...el.classList].find((c) => c.startsWith("seat-"))!,
      ),
    );
    expect(seats).toEqual(new Set(["seat-red", "seat-blue"]));
  });

  // **Decoration, and drawn**: hidden from assistive technology (the title
  // beside it is what names the product), and never an `<img>` or an SVG
  // `<image>` — `ASSETS.md`'s rule for a setup screen is no card art at all.
  it("is decoration: hidden from assistive technology, and nothing in it is an image", () => {
    const { container } = render(<BoardMark width={236} />);
    const svg = container.querySelector("svg")!;
    expect(svg).toHaveAttribute("aria-hidden", "true");
    expect(svg).toHaveAttribute("width", "236");
    expect(svg).toHaveAttribute("height", String(Math.round((236 * 296) / 420)));
    expect(container.querySelectorAll("img, image")).toHaveLength(0);
  });

  // **Every trade the drawing names is a colour the STYLESHEET gives it** — the
  // hues live in `styles.css` as classes, so a token changed there changes the
  // board, and a trade with no rule would draw black. Read from the source, the
  // lane's own idiom (`Refusal.test.tsx`); vitest runs from `web/`.
  it("tints every tile with a class the stylesheet colours", () => {
    const css = readFileSync("src/styles.css", "utf8");
    for (const trade of new Set(TILES.map((t) => t.trade))) {
      expect(css, `no rule colours .bm-tile.${trade}`).toContain(`.bm-tile.${trade} {`);
    }
  });
});
