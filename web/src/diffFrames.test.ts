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
import type { HandPanel, HistoricalFrame, PlayerModel, SiteModel, TileModel } from "./types";

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
    expect(d.tiles.size + d.seats.size + d.slots.size + d.techs.size + d.hand.size).toBe(0);
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
    type Read = { seats: number[]; tiles: number; slots: number; techs: number; hand: number };
    const seen: Record<string, Read> = {};
    const want: Record<string, Read> = {};
    for (const [grain, edit] of grains) {
      const b = clone(wire.atK);
      const seat0 = playersOf(b)[0]!;
      edit(seat0);
      const d = diffFrames(wire.atK, b);
      seen[grain] = { seats: [...d.seats], tiles: d.tiles.size, slots: d.slots.size, techs: d.techs.size, hand: d.hand.size };
      want[grain] = { seats: [seat0.seat], tiles: 0, slots: 0, techs: 0, hand: 0 };
    }
    expect(seen).toEqual(want);
  });

  /** A TAKE, as the engine performs one: the card at `slot` leaves, everything
   *  to its right compacts LEFT, and the rightmost slot refills from the deck
   *  (`crates/yokohama-core/src/game/cards.rs:253-259`). So every slot from the
   *  taken one rightward holds a DIFFERENT card afterwards, which is why the
   *  arrival rule exists (R-GC-1) and why these cells simulate the whole shift
   *  rather than editing one slot. */
  const take = (slots: { card: number | null }[], at: number, refill: number) => {
    const after = [...slots.slice(0, at), ...slots.slice(at + 1)].map((s) => s.card);
    slots.forEach((s, i) => {
      s.card = i < after.length ? after[i]! : refill;
    });
  };

  it("names only the LAB slot a take refilled, never the slots that shifted under it", () => {
    const b = clone(wire.atK);
    const slots = b.labs.boards[0]!.slots;
    expect(slots.map((s) => s.card), "the fixture must offer six lab cards to shift").not.toContain(null);
    take(slots, 1, 999);
    const d = diffFrames(wire.atK, b);
    // FAILS under the old per-slot rule, which lights `lab:0:1` through
    // `lab:0:5` — five slots, none of them the card that arrived.
    expect(d.slots).toEqual(new Set(["lab:0:5"]));
  });

  it("names only the PORT slot a take refilled", () => {
    const b = clone(wire.atK);
    const slots = b.ports.boards[0]!.slots;
    expect(slots.map((s) => s.card), "the fixture must offer six port cards to shift").not.toContain(null);
    take(slots, 0, 99);
    const d = diffFrames(wire.atK, b);
    // FAILS under the old per-slot rule: all six port slots differ.
    expect(d.slots).toEqual(new Set(["port:0:5"]));
  });

  // MUTATION PROOF for the SCOPE of "new": order ids and tech ids are two
  // separate numberings that OVERLAP — this very fixture offers order 11 in
  // the port and tech 11 in the laboratory — so a rule pooling both strips
  // into one set of seen cards goes silent exactly when a port refill draws an
  // order id some lab slot already shows. The card refilled here is one the
  // laboratory holds and the port does not.
  it("reads a card as new to its OWN kind of strip, never to the other one", () => {
    const ports = wire.atK.ports.boards[0]!.slots.map((s) => s.card);
    const onLabOnly = wire.atK.labs.boards
      .flatMap((board) => board.slots.map((s) => s.card))
      .find((card) => card !== null && !ports.includes(card));
    expect(onLabOnly, "no card sits in the laboratory and not in the port").not.toBeUndefined();

    const b = clone(wire.atK);
    take(b.ports.boards[0]!.slots, 0, onLabOnly!);
    // A pooled rule reads the refill as a card it has already seen and lights
    // NOTHING; the kind-scoped rule lights the slot it arrived in.
    expect(diffFrames(wire.atK, b).slots).toEqual(new Set(["port:0:5"]));
  });

  // A DEPARTURE alone: the deck is spent, the taken card is not replaced. The
  // strip below has already lost the card, and there is nothing new to point
  // at. FAILS under the old per-slot rule, which lights the emptied slot.
  it("says nothing about a slot a take emptied and no deck refilled", () => {
    const b = clone(wire.atK);
    expect(b.ports.boards[0]!.slots[2]!.card).not.toBeNull();
    b.ports.boards[0]!.slots[2]!.card = null;
    expect(diffFrames(wire.atK, b).slots.size).toBe(0);
  });

  it("says nothing when two slots merely swap their cards", () => {
    const b = clone(wire.atK);
    const slots = b.labs.boards[0]!.slots;
    const [first, second] = [slots[0]!.card, slots[1]!.card];
    expect(first).not.toBe(second);
    slots[0]!.card = second;
    slots[1]!.card = first;
    // FAILS under the old per-slot rule, which lights both.
    expect(diffFrames(wire.atK, b).slots.size).toBe(0);
  });

  // The other half of a take: the card that LEFT the laboratory, marked where
  // it landed. `techs` is keyed `<seat>:<techId>` so one set serves every seat.
  it("names the seat and the tech a take added, and no other seat", () => {
    const b = clone(wire.atK);
    const players = playersOf(b);
    const taker = players[1]!;
    expect(taker.technologies.length, "the fixture's seat 1 must own a tech to copy").toBeGreaterThan(0);
    expect(playersOf(wire.atK)[0]!.technologies).toHaveLength(0);
    taker.technologies.push({ ...taker.technologies[0]!, id: 777 });
    const d = diffFrames(wire.atK, b);
    expect(d.techs).toEqual(new Set([`${taker.seat}:777`]));
    // The row changed too — a tech is part of the seat key — and nothing else did.
    expect(d.seats).toEqual(new Set([taker.seat]));
    expect(d.tiles.size + d.slots.size).toBe(0);
    expect(d.any).toBe(true);
  });

  /** The viewer's own hand, off the one `Hand` panel a frame carries. `atK`'s
   *  is seat 0's, face up, holding orders 34 and 4 — the replay route pins the
   *  lowest non-bot seat for the life of the game, so within one run the panel
   *  is always the same seat's. */
  const handOf = (f: HistoricalFrame): HandPanel => {
    const panel = f.frame.dashboard.find((x) => "Hand" in x);
    if (!panel || !("Hand" in panel)) throw new Error("the fixture carries no Hand panel");
    return panel.Hand;
  };

  // A port take's other end, for an ORDER: the card lands in the acquirer's
  // hand, and this is the only grain that can name it — `PlayerModel` carries
  // `orders_in_hand` as a COUNT.
  it("names the order card that arrived in the viewer's hand", () => {
    const b = clone(wire.atK);
    const hand = handOf(b);
    expect(hand.hidden, "the fixture's hand must be face up to name a card").toBe(false);
    expect(hand.cards.length).toBeGreaterThan(0);
    const arrived = { ...hand.cards[0]!, id: 31, is_starting_order: false };
    expect(hand.cards.map((c) => c.id)).not.toContain(arrived.id);
    hand.cards = [...hand.cards, arrived];
    hand.card_count += 1;
    const d = diffFrames(wire.atK, b);
    expect(d.hand).toEqual(new Set([31]));
    expect(d.any).toBe(true);
    // The Hand panel is its OWN panel: `seatKey` never reads it, so this edit
    // moves no seat row. A take in a real game moves the row for other reasons
    // (the yen it cost), and that is the seat grain's business, not this one.
    expect(d.seats.size + d.tiles.size + d.slots.size + d.techs.size).toBe(0);
  });

  // **A MASKED hand marks nothing, and that is the ruling, not a limitation.**
  // The bot's identities never reach the wire — `cards` is empty in the MODEL
  // whenever `hidden` — so a bot's port take lights the slot the refill arrived
  // in, tints the seat row and is named in the account line, and points at no
  // card. A rule reading `card_count` would light "a card" with nothing to
  // light.
  it("says nothing about a hand it cannot see", () => {
    const masked = clone(wire.atK);
    const before = handOf(masked);
    before.hidden = true;
    before.cards = [];
    const after = clone(masked);
    handOf(after).card_count += 1;
    const d = diffFrames(masked, after);
    expect(d.hand.size).toBe(0);
  });

  it("says nothing about a card that left the hand", () => {
    const b = clone(wire.atK);
    handOf(b).cards = handOf(b).cards.slice(1);
    expect(diffFrames(wire.atK, b).hand.size).toBe(0);
  });

  // A tech LEAVING a seat is silent, for the reason the module doc gives about
  // every grain: the walk is over `b`, and this one cannot happen in Yokohama
  // anyway — a technology is never returned.
  it("says nothing about a tech a seat no longer holds", () => {
    const b = clone(wire.atK);
    playersOf(b)[1]!.technologies = [];
    expect(diffFrames(wire.atK, b).techs.size).toBe(0);
  });

  // MUTATION PROOF: a field the diff does not read leaves it silent, so the
  // cells above are seeing the fields they name and not "anything at all".
  it("ignores what is not on the table — the reachable mark", () => {
    const b = clone(wire.atK);
    b.frame.board.tiles[0]!.reachable = !b.frame.board.tiles[0]!.reachable;
    const d = diffFrames(wire.atK, b);
    expect(d.any).toBe(false);
    expect(d.techs.size + d.hand.size).toBe(0);
  });
});
