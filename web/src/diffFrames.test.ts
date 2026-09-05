// What changed between two positions, read off the wire and never off a rule.

import { describe, expect, it } from "vitest";
import { diffFrames } from "./diffFrames";
import { wire } from "./wire/contract";
import type { HistoricalFrame } from "./types";

const clone = (f: HistoricalFrame): HistoricalFrame => JSON.parse(JSON.stringify(f));

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

  it("sees a president arriving, the Station, and a space being built on", () => {
    const b = clone(wire.atK);
    const [t0, t1, t2] = b.frame.board.tiles as [typeof b.frame.board.tiles[0], typeof b.frame.board.tiles[0], typeof b.frame.board.tiles[0]];
    t0.presidents = [...t0.presidents, 1];
    t1.has_station = !t1.has_station;
    if (t2.site) t2.site.slots[0]!.built_by = 0;
    const d = diffFrames(wire.atK, b);
    expect(d.tiles).toEqual(new Set([t0.area, t1.area, ...(t2.site ? [t2.area] : [])]));
  });

  it("names the seat whose money or goods moved", () => {
    const b = clone(wire.atK);
    const players = b.frame.dashboard.find((p) => "Players" in p);
    if (!players || !("Players" in players)) throw new Error("the fixture carries no Players panel");
    players.Players.players[1]!.money += 2;
    const d = diffFrames(wire.atK, b);
    expect([...d.seats]).toEqual([players.Players.players[1]!.seat]);
    expect(d.tiles.size).toBe(0);
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
