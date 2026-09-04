// The popover: a tile at full size on hover or focus, never modal, never focused.

import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Board } from "./Board";
import { piecesWords } from "./Pieces";
import { popoverPosition } from "./TilePopover";
import { wire } from "./wire/contract";
import type { BoardModel, PlayerColor, TileModel } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];
const COLORS_4: PlayerColor[] = ["Red", "Blue", "Yellow", "Green"];

/** How many pieces stand on a tile — presidents plus every colour's cubes. The
 *  Station is deliberately NOT counted: it is a piece in the words and in the
 *  band, but it makes no `.piece-group`, so a tile carrying only a Station
 *  would satisfy a crowd test and then compare nothing. */
const crowd = (t: TileModel): number =>
  t.presidents.length + t.assistants.reduce((n, [, k]) => n + k, 0);

/** The tile whose id badge reads `id`. */
function tileOf(container: HTMLElement, id: string): HTMLElement {
  const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find(
    (t) => t.querySelector(".tile-id")?.textContent === id,
  );
  if (!tile) throw new Error(`no tile ${id}`);
  return tile;
}

const tileId = (t: TileModel): string => `A${String(t.area).padStart(2, "0")}`;

const showBoard = (model: BoardModel, colors: PlayerColor[]) =>
  render(
    <Board
      board={model}
      art={wire.artManifest}
      labels={wire.labels}
      colors={colors}
      selected={[]}
      busy={false}
    />,
  );

describe("where the popover goes", () => {
  it("sits right of the tile when there is room, left when there is not", () => {
    const viewport = { width: 1710, height: 869 };
    expect(popoverPosition({ left: 100, top: 200, right: 340, bottom: 380 }, viewport)).toEqual({ left: 348, top: 200 });
    expect(popoverPosition({ left: 1400, top: 200, right: 1640, bottom: 380 }, viewport)).toEqual({ left: 1024, top: 200 });
  });
  // The 434 is the popover's MEASURED height in Chromium at 1710x869 (433.875,
  // rounded up), not a guess — see `popoverPosition`. The constant follows the
  // default; the cell is about the clamp arithmetic.
  it("stays inside the viewport at the bottom", () => {
    expect(popoverPosition({ left: 100, top: 800, right: 340, bottom: 980 }, { width: 1710, height: 869 }).top).toBe(869 - 434 - 8);
  });

  // **The flip is taken against the TABLE's right edge, not the viewport's.**
  // Measured on the top-right tile at 1710x869 before this: the popover spanned
  // x 1122–1474 while `.table` ended at 1230, so it lay over the pinned answer
  // bar's options. The anchor below is that tile — room to the right of it in
  // the WINDOW, none inside the table.
  it("flips left when the table ends before the viewport does", () => {
    const viewport = { width: 1710, height: 869 };
    const anchor = { left: 970, top: 200, right: 1114, bottom: 380 };
    const size = { width: 368, height: 434 };
    // Against the viewport (the old rule, and the fallback where there is no
    // table): right of the tile.
    expect(popoverPosition(anchor, viewport, size).left).toBe(1122);
    // Against the table's own edge: left of it, and clear of the deck.
    expect(popoverPosition(anchor, viewport, size, 1230).left).toBe(anchor.left - 8 - 368);
  });

  // **And below 1201px nothing moves** (spec R8). There `.table` is
  // `width: 100%` in the column stack with no padding of its own, so its right
  // edge IS the viewport's — the same number goes in, and the same position
  // comes out.
  it("is unchanged where the table spans the whole width", () => {
    const viewport = { width: 1024, height: 800 };
    const anchor = { left: 100, top: 200, right: 340, bottom: 380 };
    expect(popoverPosition(anchor, viewport, undefined, viewport.width)).toEqual(
      popoverPosition(anchor, viewport),
    );
  });
});

describe("the popover on a tile", () => {
  const first = wire.options.frame.board.tiles.find((t) => t.site !== null)!;
  const id = `A${String(first.area).padStart(2, "0")}`;
  const board = (opts: { busy?: boolean; clickable?: boolean } = {}) => (
    <Board
      board={wire.options.frame.board}
      art={wire.artManifest}
      labels={wire.labels}
      colors={COLORS}
      selected={[]}
      busy={opts.busy ?? false}
      {...(opts.clickable ? { onTileClick: () => {} } : {})}
    />
  );
  const show = () => render(board());

  it("opens on hover and closes when the pointer leaves", async () => {
    const user = userEvent.setup();
    const { container } = show();
    const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) => t.querySelector(".tile-id")?.textContent === id)!;
    expect(screen.queryByRole("tooltip")).toBeNull();
    await user.hover(tile);
    const pop = screen.getByRole("tooltip");
    expect(pop.id).toBe(`pop-${id}`);
    expect(tile.getAttribute("aria-describedby")).toBe(`pop-${id}`);
    expect(pop.querySelector(".site-face")).not.toBeNull();
    expect(pop.textContent).toContain(`Site ${first.site!.index}`);
    await user.unhover(tile);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("opens on focus, closes on Escape, and is not in the tab order itself", async () => {
    const user = userEvent.setup();
    const { container } = show();
    const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) => t.querySelector(".tile-id")?.textContent === id)!;
    // `act` because a bare `.focus()` is a React state update outside the act
    // environment: React 19 defers it, and it would also print a warning.
    act(() => tile.focus());
    const pop = screen.getByRole("tooltip");
    expect(pop.getAttribute("tabindex")).toBeNull();
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  // An inert tile (no click handler, or the Canal) is still inspectable by
  // keyboard: it can take focus, and focusing it does not make it a button.
  it("an inert tile can be focused for its popover and is still not a button", () => {
    const { container } = show();
    const tile = container.querySelector<HTMLElement>(".tile")!;
    expect(tile.tagName).toBe("DIV");
    expect(tile.getAttribute("tabindex")).toBe("0");
  });

  // ---- the two ways a popover used to get STUCK (ruling R-T5-fold) ---------
  //
  // Both are about the popover outliving the thing that opened it, and neither
  // is reachable from the cells above: those open on FOCUS, where the tile is
  // the event target and its own handlers are enough.

  // A popover opened by HOVER leaves nothing focused, so `keydown` goes to the
  // body and the tile's own `onKeyDown` — which is what the focus cell above
  // exercises — never sees the key at all. Escape is the documented way out of
  // a tooltip for a keyboard user who reached for it after pointing.
  it("closes on Escape when it was opened by hover, with nothing focused", async () => {
    const user = userEvent.setup();
    const { container } = show();
    const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) => t.querySelector(".tile-id")?.textContent === id)!;
    await user.hover(tile);
    expect(screen.getByRole("tooltip")).toBeTruthy();
    expect(document.activeElement).toBe(document.body);
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  // Clicking a tile sets `busy`, which puts `disabled` on every tile button —
  // and Chrome does not fire `mouseleave` on an element that became disabled
  // under the pointer. So the popover the hover opened stayed on screen over a
  // board that had already moved on. The tile is a button here because that is
  // the only shape the defect has: an inert `div` never takes `disabled`.
  it("closes when the board goes busy under the pointer", async () => {
    const user = userEvent.setup();
    const { container, rerender } = render(board({ clickable: true }));
    const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) => t.querySelector(".tile-id")?.textContent === id)!;
    await user.hover(tile);
    expect(screen.getByRole("tooltip")).toBeTruthy();
    rerender(board({ clickable: true, busy: true }));
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  // ---- the pieces on the enlarged tile (owner ruling R-GA-1, gate A) -------
  //
  // The popover calls itself a tile at full size, so it draws what the tile
  // draws — the same band, at the same rect, from the same component. Every
  // number below is read off the BOARD tile rather than typed in: a popover
  // that re-decided the layout (a different crowd, a different pile rule, a
  // different `--piece-units`) fails here even though both drawings would look
  // plausible on their own. That is the whole property, and it is not one a
  // hand-written expectation could hold.
  it("draws the pieces on the enlarged tile, the same glyphs the board tile shows", async () => {
    const user = userEvent.setup();
    // The replay frame, which is mid-game and therefore has pieces on it; the
    // 2P options frame is move 1 and its board is bare. The fattest tile, so
    // the comparison has both a pawn and a cube to compare.
    const model = wire.atK.frame.board;
    const busiest = [...model.tiles].sort((a, b) => crowd(b) - crowd(a))[0]!;
    // The precondition, asserted: on a bare board every count below would be
    // zero and the cell would compare nothing to nothing.
    expect(crowd(busiest), "the replay corpus has pieces on some tile").toBeGreaterThan(0);

    const { container } = showBoard(model, COLORS);
    const tile = tileOf(container, tileId(busiest));
    const onTile = tile.querySelector<HTMLElement>(".pieces-band .pieces");
    expect(onTile, "the board tile draws its own pieces").not.toBeNull();
    // Anti-vacuity on both compared counts, not just on the crowd: a tile
    // carrying only presidents would make the cube compare 0 === 0.
    expect(onTile!.querySelectorAll(".piece-group").length).toBeGreaterThan(0);
    expect(onTile!.querySelectorAll(".piece.cube").length).toBeGreaterThan(0);

    await user.hover(tile);
    const pop = container.querySelector<HTMLElement>('[role="tooltip"]')!;
    const inPop = pop.querySelector<HTMLElement>(".tile-pop-art-box .pieces-band .pieces");
    expect(inPop, "the popover's enlarged art carries the band too").not.toBeNull();

    // The accessible name IS the sentence the words row prints, so the glyphs
    // and the words cannot drift apart.
    const words = piecesWords(busiest.presidents, busiest.assistants, busiest.has_station, COLORS);
    expect(inPop!.getAttribute("aria-label")).toBe(words);
    expect(pop.textContent).toContain(words);

    // What the tile drew, drawn again.
    expect(inPop!.querySelectorAll(".piece-group")).toHaveLength(onTile!.querySelectorAll(".piece-group").length);
    expect(inPop!.querySelectorAll(".piece.cube")).toHaveLength(onTile!.querySelectorAll(".piece.cube").length);
    // The same crowd at the same rect aspect prices the same row, and the band
    // is handed the same width in glyph heights it is on the tile.
    const units = (el: HTMLElement) => el.style.getPropertyValue("--piece-units");
    expect(units(onTile!)).not.toBe("");
    expect(units(inPop!)).toBe(units(onTile!));
  });

  // The two ways the band draws nothing, and they are NOT the same thing: an
  // empty tile has a band that holds no pieces (the component returns null for
  // no words, exactly as on the board), while the Canal has no band at all
  // (nothing ever stands there). The popover makes neither decision itself.
  it("draws no pieces for an empty tile, and no band on the Canal", async () => {
    const user = userEvent.setup();
    const bare = wire.options.frame.board.tiles.find((t) => t.area_type !== "Canal" && crowd(t) === 0);
    expect(bare, "the 2P options frame is move 1, so some tile is bare").toBeTruthy();
    const { container } = showBoard(wire.options.frame.board, COLORS);
    const bareTile = tileOf(container, tileId(bare!));
    await user.hover(bareTile);
    const pop = container.querySelector<HTMLElement>('[role="tooltip"]')!;
    expect(pop.querySelector(".tile-pop-art-box .pieces-band"), "the band is the tile's band").not.toBeNull();
    expect(pop.querySelector(".pieces")).toBeNull();
    expect(pop.textContent).toContain("no pieces here");

    const model = wire.options4p.frame.board;
    const canal = model.tiles.find((t) => t.area_type === "Canal");
    expect(canal, "the 4P corpus carries a Canal").toBeTruthy();
    const four = showBoard(model, COLORS_4);
    const canalTile = tileOf(four.container, tileId(canal!));
    expect(canalTile.querySelector(".pieces-band"), "nor does the board tile draw one there").toBeNull();
    await user.hover(canalTile);
    const canalPop = four.container.querySelector<HTMLElement>('[role="tooltip"]');
    expect(canalPop, "the Canal is still inspectable").not.toBeNull();
    expect(canalPop!.querySelector(".pieces-band")).toBeNull();
  });
});
