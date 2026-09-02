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

// ---- M3.5: the board carries the rules information a move is chosen on ----
//
// The three cells below are the milestone's thesis in test form: what influences
// the next move should be legible ON THE BOARD. jsdom has no layout, so none of
// them asserts a pixel — each asserts the DECLARATION that produces the pixel,
// which is also the thing a regression would delete.

describe("the tile width scales to the board", () => {
  // The board's widest row IS its width: `players + 2` tiles at 14rem is
  // 915/1146/1376px against a 992px column at 1440, so a 4P board carried 384px
  // off screen behind `overflow-x: auto` with no scrollbar drawn.
  //
  // Keyed on the REALIZED widest row and never on `board.player_count`: that
  // field has no other consumer in this client, and reading it here would make
  // this file the first place to assume the pyramid's shape — against its own
  // module doc. The widest row is the term the width actually depends on, and
  // it is already computed to lay the rows out.
  it.each([
    ["2P", wire.options, COLORS, 4, "14rem"] as const,
    ["4P", wire.options4p, COLORS_4, 6, "10rem"] as const,
  ])("%s: a widest row of %s tiles asks for %s", (_n, state, colors, widest, expected) => {
    const { container } = show(state, { colors });
    const rows = [...container.querySelectorAll(".board-row")];
    const realized = Math.max(...rows.map((r) => r.querySelectorAll(".tile").length));
    expect(realized, "the fixture must have the widest row this cell is about").toBe(widest);

    const board = container.querySelector<HTMLElement>(".board")!;
    expect(board.style.getPropertyValue("--tile-w")).toBe(expected);
  });

  // Anti-vacuity for the pair above: the two fixtures must actually DISAGREE.
  // A ladder that returned one constant would satisfy both rows if they were
  // ever loosened, and this is the cell that would still fail.
  it("gives boards of different widths different tiles", () => {
    const read = (state: ClientState, colors: PlayerColor[]) =>
      show(state, { colors })
        .container.querySelector<HTMLElement>(".board")!
        .style.getPropertyValue("--tile-w");
    expect(read(wire.options, COLORS)).not.toBe(read(wire.options4p, COLORS_4));
  });
});

describe("the 5-Power token sits in its printed slot", () => {
  // The tokens are shuffled face-down and placed FACE-UP on each Area Board's
  // 5-space (`01-rules-reference.md:680`), so the face is public from setup and
  // the board has always been served it. Drawing it where the physical token
  // sits is what lets a player scan eighteen tiles for "is the bonus still
  // there" without opening a panel.
  it("draws the token face on the art while it is still there", () => {
    const tile = wire.options4p.frame.board.tiles.find((t) => t.token !== null);
    expect(tile, "the 4P fixture must carry an untaken token").toBeDefined();
    const { container } = show(wire.options4p, { colors: COLORS_4 });
    const id = `A${String(tile!.area).padStart(2, "0")}`;
    const el = [...container.querySelectorAll<HTMLElement>(".tile")].find((t) =>
      t.textContent?.startsWith(id),
    )!;
    expect(el.querySelector(".token-slot img")).not.toBeNull();
  });

  // "Taken" needs no mark of its own: the printed frame with its red 5 showing
  // IS the empty slot, exactly as on the table. The words stay for the channel
  // that cannot see the picture — and they are what carries WHICH good, which
  // the art does not resolve at a 4P tile's 160px.
  it("draws nothing in the slot once it is taken, and still says so", () => {
    const taken = {
      ...wire.options4p,
      frame: {
        ...wire.options4p.frame,
        board: {
          ...wire.options4p.frame.board,
          tiles: wire.options4p.frame.board.tiles.map((t) => ({ ...t, token: null })),
        },
      },
    };
    const { container } = show(taken, { colors: COLORS_4 });
    expect(container.querySelectorAll(".token-slot img")).toHaveLength(0);
    expect(screen.getAllByText("5-Power taken").length).toBeGreaterThan(0);
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

describe("a historical board", () => {
  // **A past position is not answerable, and the board has to SAY so.**
  //
  // `Board` turns a tile into a `<button>` only when it is handed
  // `onTileClick`, so a replay frame drawn without one is inert by
  // construction. This is the property that makes the scrubber safe to host
  // behind a modal panel: there is nothing behind it to press. Asserted over
  // the whole board rather than one tile, because a single escaping button is
  // a click that would post an input against a position the game has left.
  it("renders every tile inert when no click handler is given", () => {
    // Rendered directly rather than through `show`, which takes a
    // `ClientState`: the whole point of `HistoricalFrame` is that it is NOT one.
    const { container } = render(
      <Board
        board={wire.atK.frame.board}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        selected={[]}
        busy={false}
      />,
    );
    expect(container.querySelectorAll(".tile").length).toBeGreaterThan(0);
    expect(container.querySelectorAll("button.tile")).toHaveLength(0);
  });
});
