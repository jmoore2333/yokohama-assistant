// The popover: a tile at full size on hover or focus, never modal, never focused.

import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Board } from "./Board";
import { popoverPosition } from "./TilePopover";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

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
});
