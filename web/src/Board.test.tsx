// The board: the shape it draws, and the one tile that is never an input.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Board } from "./Board";
import { wire } from "./wire/contract";
import type { ClientState, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];
const COLORS_4: PlayerColor[] = ["Red", "Blue", "Yellow", "Green"];

function show(
  state: ClientState,
  opts: {
    colors?: PlayerColor[];
    onTileClick?: (area: number) => void;
    selected?: number[];
    busy?: boolean;
  } = {},
) {
  return render(
    <Board
      board={state.frame.board}
      art={wire.artManifest}
      labels={wire.labels}
      colors={opts.colors ?? COLORS}
      selected={opts.selected ?? []}
      busy={opts.busy ?? false}
      {...(opts.onTileClick ? { onTileClick: opts.onTileClick } : {})}
    />,
  );
}

describe("the Canal", () => {
  // The Canal is a CROSSING, never a stop: it is never placeable and never
  // stoppable, single-sourced in the engine's two shared movement predicates.
  // A tile that is a button there offers a placement the engine will refuse,
  // and the 2P tile set has no Canal at all — which is why this reads the 4P
  // fixture and would silently pass on the 2P one.
  it("is drawn, and is never a button", async () => {
    const canal = wire.options4p.frame.board.tiles.filter((t) => t.area_type === "Canal");
    expect(canal, "the 4P fixture must carry a Canal").toHaveLength(1);
    const onTileClick = vi.fn();
    const { container } = show(wire.options4p, { colors: COLORS_4, onTileClick });

    expect(container.querySelectorAll(".tile.canal")).toHaveLength(1);
    expect(screen.getByText(/crossing only/)).toBeInTheDocument();
    const id = `A${String(canal[0]!.area).padStart(2, "0")}`;
    expect(screen.queryByRole("button", { name: new RegExp(`^${id} `) })).toBeNull();
    expect(onTileClick).not.toHaveBeenCalled();
  });

  it("has no site and no 5-Power token to draw", () => {
    const canal = wire.options4p.frame.board.tiles.find((t) => t.area_type === "Canal")!;
    expect(canal.site).toBeNull();
    expect(canal.token).toBeNull();
  });
});

describe("the staggered pyramid", () => {
  // Drawn from `grid_position` and nothing else, which is the same property the
  // CLI's live renderer has: any geometry the engine hands over renders, so the
  // board cannot drift from the data. The row lengths differ per player count —
  // 4/3/2/1 at 2P, 6/5/4/3 at 4P — and both come out of the same code.
  it.each([
    ["2P", wire.options, COLORS] as const,
    ["4P", wire.options4p, COLORS_4] as const,
  ])("%s draws one row per grid row, in order", (_name, state, colors) => {
    const { container } = show(state, { colors });
    const rows = [...container.querySelectorAll(".board-row")];
    const expected = new Map<number, number>();
    for (const t of state.frame.board.tiles) {
      expected.set(t.grid_position[0], (expected.get(t.grid_position[0]) ?? 0) + 1);
    }
    expect(rows).toHaveLength(expected.size);
    [...expected.entries()]
      .sort((a, b) => a[0] - b[0])
      .forEach(([, count], i) => {
        expect(rows[i]!.querySelectorAll(".tile")).toHaveLength(count);
      });
  });

  it("draws every tile the wire carries and no more", () => {
    const { container } = show(wire.options4p, { colors: COLORS_4 });
    expect(container.querySelectorAll(".tile")).toHaveLength(
      wire.options4p.frame.board.tiles.length,
    );
  });
});

describe("a tile as an input", () => {
  it("reports its selection to a screen reader", async () => {
    const area = wire.options.frame.board.tiles[0]!.area;
    show(wire.options, { onTileClick: vi.fn(), selected: [area] });
    const id = `A${String(area).padStart(2, "0")}`;
    const button = screen.getByRole("button", { name: new RegExp(`^${id} .* selected`) });
    expect(button).toHaveAttribute("aria-pressed", "true");
  });

  it("sends the area it names", async () => {
    const onTileClick = vi.fn();
    const area = wire.options.frame.board.tiles[2]!.area;
    show(wire.options, { onTileClick });
    const id = `A${String(area).padStart(2, "0")}`;
    await userEvent.setup().click(screen.getByRole("button", { name: new RegExp(`^${id} `) }));
    expect(onTileClick).toHaveBeenCalledExactlyOnceWith(area);
  });

  // The tile is a mutation control like any option row and takes the same
  // guard: it was the one that did not, and a double click on a placement
  // toggle could leave the highlight disagreeing with the server.
  it("is disabled while a request is in flight", () => {
    const onTileClick = vi.fn();
    show(wire.options, { onTileClick, busy: true });
    for (const b of screen.getAllByRole("button")) {
      if (/^A\d\d /.test(b.getAttribute("aria-label") ?? "")) expect(b).toBeDisabled();
    }
  });
});
