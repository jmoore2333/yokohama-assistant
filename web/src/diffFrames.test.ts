// What changed between two positions, read off the wire and never off a rule.
//
// EVERY grain the module reads is pinned by a cell that fails without it. The
// seat grains are asserted ONE EDIT PER FRESH CLONE rather than bundled,
// because a bundle cannot attribute: with four seat fields moved at once,
// dropping any one of them from the key leaves the seat lit by its siblings and
// the cell green. The field-to-cell table is in the task's fix report.

import { describe, expect, it } from "vitest";
import { diffFrames } from "./diffFrames";
import { wire } from "./wire/contract";
import type { HistoricalFrame, PlayerModel, SiteModel, TileModel } from "./types";

const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

/** A PRECONDITION, not an `if`: a fixture without the thing a cell edits would
 *  quietly weaken the claim instead of failing it. */
const siteOf = (t: TileModel): SiteModel => {
  if (!t.site) throw new Error(`tile ${t.area} carries no site to build on`);
  return t.site;
};

const playersOf = (f: HistoricalFrame): PlayerModel[] => {
  const panel = f.frame.dashboard.find((p) => "Players" in p);
  if (!panel || !("Players" in panel)) throw new Error("the fixture carries no Players panel");
  return panel.Players.players;
};

describe("diffFrames", () => {
  it("sees nothing between a position and itself", () => {
    const d = diffFrames(wire.atK, clone(wire.atK));
    expect(d.any).toBe(false);
    expect(d.tiles.size + d.seats.size + d.slots.size).toBe(0);
  });

  it("names exactly the tile whose assistants moved", () => {
    const b = clone(wire.atK);
    const tile = b.frame.board.tiles[3]!;
    tile.assistants[0]![1] += 1;
    const d = diffFrames(wire.atK, b);
    expect([...d.tiles]).toEqual([tile.area]);
    expect(d.seats.size).toBe(0);
    expect(d.any).toBe(true);
  });

  it("sees a president arriving, the Station, a space being built on, and a trading house's owner", () => {
    const b = clone(wire.atK);
    const [t0, t1, t2, t3] = b.frame.board.tiles as [TileModel, TileModel, TileModel, TileModel];
    t0.presidents = [...t0.presidents, 1];
    t1.has_station = !t1.has_station;
    siteOf(t2).slots[0]!.built_by = 0;
    siteOf(t3).trading_house_owner = 0;
    const d = diffFrames(wire.atK, b);
    expect(d.tiles).toEqual(new Set([t0.area, t1.area, t2.area, t3.area]));
    expect(d.seats.size).toBe(0);
  });

  it("names the seat whose money moved", () => {
    const b = clone(wire.atK);
    const players = playersOf(b);
    players[1]!.money += 2;
    const d = diffFrames(wire.atK, b);
    expect([...d.seats]).toEqual([players[1]!.seat]);
    expect(d.tiles.size).toBe(0);
  });

  it("names the seat whose VP, goods, warehouse or hand moved", () => {
    // One edit per grain on its OWN clone, so a field dropped from the seat key
    // is attributed to the grain that names it. The reading is a map rather
    // than six assertions so a failure prints WHICH grain went silent.
    const grains: [string, (p: PlayerModel) => void][] = [
      ["victory_points", (p) => { p.victory_points += 1; }],
      ["goods", (p) => { p.goods.tea += 1; }],
      ["assistants.in_warehouse", (p) => { p.assistants.in_warehouse += 1; }],
      ["shophouses.in_warehouse", (p) => { p.shophouses.in_warehouse += 1; }],
      ["trading_houses.in_warehouse", (p) => { p.trading_houses.in_warehouse += 1; }],
      ["orders_in_hand", (p) => { p.orders_in_hand += 1; }],
    ];
    type Read = { seats: number[]; tiles: number; slots: number };
    const seen: Record<string, Read> = {};
    const want: Record<string, Read> = {};
    for (const [grain, edit] of grains) {
      const b = clone(wire.atK);
      const seat0 = playersOf(b)[0]!;
      edit(seat0);
      const d = diffFrames(wire.atK, b);
      seen[grain] = { seats: [...d.seats], tiles: d.tiles.size, slots: d.slots.size };
      want[grain] = { seats: [seat0.seat], tiles: 0, slots: 0 };
    }
    expect(seen).toEqual(want);
  });

  it("names a port slot whose card changed, and a lab slot, by board and slot", () => {
    const b = clone(wire.atK);
    b.ports.boards[0]!.slots[2]!.card = null;
    b.labs.boards[0]!.slots[5]!.card = 999;
    const d = diffFrames(wire.atK, b);
    expect(d.slots).toEqual(new Set(["port:0:2", "lab:0:5"]));
  });

  // MUTATION PROOF: a field the diff does not read leaves it silent, so the
  // cells above are seeing the fields they name and not "anything at all".
  it("ignores what is not on the table — the reachable mark", () => {
    const b = clone(wire.atK);
    b.frame.board.tiles[0]!.reachable = !b.frame.board.tiles[0]!.reachable;
    expect(diffFrames(wire.atK, b).any).toBe(false);
  });
});
