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
  const show = () =>
    render(
      <Board board={wire.options.frame.board} art={wire.artManifest} labels={wire.labels} colors={COLORS} selected={[]} busy={false} />,
    );

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
});
