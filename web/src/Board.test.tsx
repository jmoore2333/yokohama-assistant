// The board: the shape it draws, and the one tile that is never an input.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Board } from "./Board";
import { wire } from "./wire/contract";
import { SITE_CARD } from "./tileRects";
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
    changed?: Set<number>;
    changedBy?: PlayerColor;
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
      changed={opts.changed}
      changedBy={opts.changedBy}
      {...(opts.onTileClick ? { onTileClick: opts.onTileClick } : {})}
    />,
  );
}

/** The tile whose id badge reads `id`. */
function tileOf(container: HTMLElement, id: string): HTMLElement {
  const tile = [...container.querySelectorAll<HTMLElement>(".tile")].find(
    (t) => t.querySelector(".tile-id")?.textContent === id,
  );
  if (!tile) throw new Error(`no tile ${id}`);
  return tile;
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

describe("the tile ladder scales to the board", () => {
  // The board's widest row IS its width: `players + 2` tiles at 14rem is
  // 915/1146/1376px against a 992px column at 1440, so a 4P board carried 384px
  // off screen behind `overflow-x: auto` with no scrollbar drawn.
  //
  // **The ladder is the SUB-1201 arm since M5 W3**, and it is read here under
  // its own name. Above 1201 the stylesheet derives `--tile-w` from the fit
  // rule over the table's container units, which jsdom has no layout engine to
  // resolve — so the browser cell in `e2e/the_table.spec.ts` is what checks
  // that arm, against `fit.ts` as its oracle. What these cells still pin is
  // the declaration this component makes, which is the ladder.
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
    expect(board.style.getPropertyValue("--tile-ladder")).toBe(expected);
  });

  // Anti-vacuity for the pair above: the two fixtures must actually DISAGREE.
  // A ladder that returned one constant would satisfy both rows if they were
  // ever loosened, and this is the cell that would still fail.
  it("gives boards of different widths different tiles", () => {
    const read = (state: ClientState, colors: PlayerColor[]) =>
      show(state, { colors })
        .container.querySelector<HTMLElement>(".board")!
        .style.getPropertyValue("--tile-ladder");
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
    const el = tileOf(container, id);
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

// ---- M5: the tile is the art, and the pieces sit on it -------------------
//
// The caption is gone. What it said in words rides in `.sr-only` spans and on
// the pieces' own accessible name on the INERT `<div>`, and in the `aria-label`
// on the clickable `<button>` (which suppresses both) — see `Board.tsx`'s own
// paragraph on the split. What it said in glyphs is drawn where the physical
// piece stands. jsdom still has no layout, so these cells assert the
// DECLARATIONS — the overlay is present, it carries the rect the shared JSON
// names, the built marks carry the builder's seat class — which is exactly what
// a regression would delete.
describe("the tile is the art, and the pieces sit on it (M5)", () => {
  const withPieces = (): { state: ClientState; id: string } => {
    const state = structuredClone(wire.options) as ClientState;
    const t = state.frame.board.tiles.find((tile) => tile.site !== null)!;
    t.presidents = [0];
    t.assistants = [
      [0, 0],
      [1, 2],
    ];
    t.has_station = true;
    t.site!.slots[0]!.built_by = 1;
    t.site!.trading_house_owner = 0;
    return { state, id: `A${String(t.area).padStart(2, "0")}` };
  };

  it("draws no caption under the art", () => {
    const { container } = show(wire.options);
    expect(container.querySelector(".tile-body")).toBeNull();
    expect(container.querySelector(".site-strip")).toBeNull();
    expect(screen.queryByText("no pawns")).toBeNull();
  });

  it("puts the site card on the tile, built spaces in the builder's colour", () => {
    const { state, id } = withPieces();
    const tile = tileOf(show(state).container, id);
    const card = tile.querySelector(".site-on-tile");
    expect(card, "no site card on the tile").not.toBeNull();
    // Read back as NUMBERS against the shared rect, because jsdom's CSSOM
    // re-serializes `50.00%` as `50%` and a string compare would be asserting
    // the serializer. The point of the cell is that the overlay is placed from
    // `tile_rects.json` and not from a figure typed into the component.
    const at = (axis: "left" | "top") => parseFloat((card as HTMLElement).style[axis]);
    expect(at("left")).toBeCloseTo(SITE_CARD.left * 100, 2);
    expect(at("top")).toBeCloseTo(SITE_CARD.top * 100, 2);
    expect(card!.querySelector(".site-space.slot-0.built")?.className).toMatch(/seat-blue/);
    expect(card!.querySelector(".site-trading-house.built")?.className).toMatch(/seat-red/);
  });

  it("names the pieces for a screen reader and draws them as pieces", () => {
    const { state, id } = withPieces();
    const tile = tileOf(show(state).container, id);
    expect(tile.querySelector(".pieces")?.getAttribute("aria-label")).toBe(
      "Red president · 2 Blue assistants · Station",
    );
    expect(tile.querySelectorAll(".piece.cube")).toHaveLength(2);
  });

  it("keeps the id, the site and the token in words the caption used to carry", () => {
    const { container } = show(wire.options);
    const t = wire.options.frame.board.tiles.find((tile) => tile.site?.index != null)!;
    const id = `A${String(t.area).padStart(2, "0")}`;
    const words = tileOf(container, id).textContent ?? "";
    expect(words).toContain(id);
    expect(words).toContain(`Site ${t.site!.index}`);
    expect(words).toMatch(/5-Power/);
  });

  // **A `<button>` is ONE node to assistive technology, and its `aria-label`
  // overrides its contents**, so the `.sr-only` spans inside a clickable tile
  // are never announced — and on an interactive board every non-Canal tile is a
  // button. The cell above renders through `show` with no `onTileClick`, so it
  // exercises the inert `<div>` path and structurally cannot see this.
  it("folds every fact into a clickable tile's accessible name", () => {
    const { state, id } = withPieces();
    const t = state.frame.board.tiles.find(
      (tile) => `A${String(tile.area).padStart(2, "0")}` === id,
    )!;
    expect(t.site!.index, "the fixture tile must carry a printed site").not.toBeNull();
    t.reachable = true;
    show(state, { onTileClick: vi.fn(), selected: [t.area] });

    const label =
      screen.getByRole("button", { name: new RegExp(`^${id} `) }).getAttribute("aria-label") ?? "";
    // The prefix is load-bearing: the browser harness and three other cells
    // find a tile by it.
    expect(label).toMatch(new RegExp(`^${id} \\S`));
    expect(label).toContain("selected");
    expect(label).toContain("Red president · 2 Blue assistants · Station");
    expect(label).toMatch(t.token ? /5-Power \S/ : /5-Power taken/);
    expect(label).toContain(`Site ${t.site!.index}`);
    expect(label).toContain("a Movement destination the engine offers");
  });

  it("still draws a name when there is no art, and no broken image", () => {
    const { container } = render(
      <Board
        board={wire.options.frame.board}
        art={null}
        labels={wire.labels}
        colors={COLORS}
        selected={[]}
        busy={false}
      />,
    );
    expect(container.querySelector(".tile-art-blank .tile-name")?.textContent).toBeTruthy();
    expect(container.querySelectorAll("img")).toHaveLength(0);
  });
});

// ---- M5.5 C: what the opponent's turn changed ------------------------------

describe("the walk's highlights", () => {
  // The set comes from `diffFrames`, which compares two wire frames and knows
  // no rule; the board's whole job is to draw what it names, in the colour of
  // the seat that moved. The class carries the colour rather than a written
  // hex, so the CSS outlines in `currentColor` and a fifth seat colour stays
  // one line in the palette.
  it("outlines the tiles the diff names, in the mover's seat class", () => {
    const { container } = show(wire.options, { changed: new Set([0]), changedBy: "Blue" });
    const lit = tileOf(container, "A00");
    expect(lit.className).toContain("changed");
    expect(lit.className).toContain("seat-blue");
    // Anti-vacuity, and the half a `className` typo would leave green: a tile
    // the diff did NOT name carries neither mark.
    const dark = tileOf(container, "A01");
    expect(dark.className).not.toContain("changed");
    expect(dark.className).not.toContain("seat-blue");
    expect(container.querySelectorAll(".tile.changed")).toHaveLength(1);
  });

  it("draws no highlight at all when nothing is passed", () => {
    const { container } = show(wire.options);
    expect(container.querySelectorAll(".tile.changed")).toHaveLength(0);
  });
});
