// What changed between two positions (M5.5 spec §3.2.4).
//
// A pure comparison of two wire frames and nothing else: the walk lights the
// tiles, seats, slots, technologies and hand cards this names, and it must
// never need a rule to do so — a rule restated here would be a second dialect of the engine.
//
// The ONE place that comes close is the offer strips, and it is a reading of
// the wire rather than a rule: a take shifts every card right of it, so "this
// slot's card differs" names five slots and the wrong ones. The arrival rule
// below reads the same two frames and asks what is NEW, which needs no notion
// of compaction at all.

import type {
  AreaId,
  HandPanel,
  HistoricalFrame,
  OrderCardId,
  Panel,
  PlayerId,
  PlayerModel,
  TileModel,
} from "./types";

export interface Changes {
  tiles: Set<AreaId>;
  seats: Set<PlayerId>;
  /** `port:<board>:<slot>` and `lab:<board>:<slot>` — the slots a card
   *  ARRIVED in, never the ones a take shifted. See `slotKeys`. */
  slots: Set<string>;
  /** `<seat>:<techId>` — the technologies a seat holds in `b` and did not hold
   *  in `a`: the other half of a laboratory take, marked where the card went. */
  techs: Set<string>;
  /** The order ids in the VIEWER's own hand in `b` and not in `a` — the other
   *  half of a port take, for the one hand this client can see.
   *
   *  **A masked hand contributes nothing, and that is the ruling rather than a
   *  gap (R-GC-2).** The wire carries a bot's hand as a COUNT and never as
   *  identities (`HandPanel.cards` is empty in the model whenever `hidden`), so
   *  a bot's port take lights the slot its refill arrived in, tints the seat
   *  row and is named in the account line — and points at no card, because
   *  there is no card on screen to point at. `PlayerModel.orders_in_hand` is a
   *  count too, so no seat's hand but the viewer's is ever nameable here. */
  hand: Set<OrderCardId>;
  any: boolean;
}

/** The parts of a tile a piece or a build can change. `reachable` is a
 *  property of the legal list, not of the table, and is deliberately absent.
 *
 *  The comparison is `JSON.stringify`, so it is key-ORDER dependent for the
 *  objects it carries. Sound for the two things this is ever handed — frames
 *  parsed from this server's serde output, and JSON round-trip clones of one —
 *  and NOT sound for a hand-built object, which this lane's harness rule
 *  forbids anyway (a payload is never hand-authored; it comes from
 *  `web/src/wire/`). */
const tileKey = (t: TileModel): string =>
  JSON.stringify([
    t.presidents,
    t.assistants,
    t.has_station,
    t.site?.slots.map((s) => s.built_by) ?? null,
    t.site?.trading_house_owner ?? null,
  ]);

const seatKey = (p: PlayerModel): string =>
  JSON.stringify([
    p.money, p.victory_points, p.goods, p.assistants, p.shophouses, p.trading_houses,
    p.technologies.map((t) => t.id), p.mercenaries_unused, p.mercenaries_used,
    p.five_power_tokens, p.president, p.orders_in_hand, p.orders_filled,
  ]);

/** The ONE hand a frame carries: the viewer's. The replay route pins the lowest
 *  non-bot seat for the life of the game, so within a run this is always the
 *  same seat's panel — and `undefined` where the payload holds none at all. */
const hand = (f: HistoricalFrame): HandPanel | undefined =>
  f.frame.dashboard.find((p): p is Extract<Panel, { Hand: HandPanel }> => "Hand" in p)?.Hand;

const players = (f: HistoricalFrame): PlayerModel[] =>
  f.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> => "Players" in p)
    ?.Players.players ?? [];

/** One offer slot: which strip it belongs to, what to call it, what is in it.
 *
 *  `<board>` is the ARRAY INDEX into `boards`, NOT the `board_b` flag. At 2P
 *  there is one board and the two readings coincide; at 3–4P there are two, and
 *  a reader that wants to name board A or B goes back to `boards[i].board_b`
 *  rather than reading this key's middle field as that flag. */
type Offer = { kind: "port" | "lab"; key: string; card: number | null };

const slotKeys = (f: HistoricalFrame): Offer[] => [
  ...f.ports.boards.flatMap((b, i) =>
    b.slots.map((s): Offer => ({ kind: "port", key: `port:${i}:${s.slot}`, card: s.card })),
  ),
  ...f.labs.boards.flatMap((b, i) =>
    b.slots.map((s): Offer => ({ kind: "lab", key: `lab:${i}:${s.slot}`, card: s.card })),
  ),
];

/** The cards a strip is offering anywhere, by strip.
 *
 *  Scoped by KIND and not pooled, because order ids and tech ids are two
 *  separate numberings that overlap: a port refill drawing order 11 while a lab
 *  slot shows tech 11 is a card that arrived, and a pooled set would call it
 *  one already on the table. Across BOARDS rather than per board, so at 3–4P a
 *  card is new to the laboratory rather than new to laboratory B. */
const cardsOnOffer = (f: HistoricalFrame): Record<"port" | "lab", Set<number>> => {
  const seen = { port: new Set<number>(), lab: new Set<number>() };
  for (const o of slotKeys(f)) if (o.card !== null) seen[o.kind].add(o.card);
  return seen;
};

/** The walk is over `b`: a tile, seat, slot, technology or hand card present in
 *  `a` and ABSENT from `b` is silent. Acceptable because the first three sets are fixed
 *  at setup — the board's areas, the seats, and the six slots per port and
 *  laboratory board do not come and go during a game — so the two frames of one
 *  game always carry the same keys, and a disappearance would be a server bug
 *  rather than a move. The fourth is not a fixed set but only ever GROWS: a
 *  technology, once owned, is never returned. */
export function diffFrames(a: HistoricalFrame, b: HistoricalFrame): Changes {
  const tiles = new Set<AreaId>();
  const wasTile = new Map(a.frame.board.tiles.map((t) => [t.area, tileKey(t)]));
  for (const t of b.frame.board.tiles) if (wasTile.get(t.area) !== tileKey(t)) tiles.add(t.area);

  const seats = new Set<PlayerId>();
  const wasSeat = new Map(players(a).map((p) => [p.seat, seatKey(p)]));
  for (const p of players(b)) if (wasSeat.get(p.seat) !== seatKey(p)) seats.add(p.seat);

  // **THE ARRIVAL RULE (R-GC-1, owner, at gate C).** A slot lights when the
  // card in it is new to its strip — never merely because the slot's card
  // differs from what it held.
  //
  // A take COMPACTS the strip left and refills the rightmost slot
  // (`crates/yokohama-core/src/game/cards.rs:253-259`), so every slot from the
  // taken one rightward shows a different card and the per-slot rule lit all of
  // them, none of them the card taken: at gate C a laboratory take outlined
  // five slots and marked nothing about the card that left. What a reader wants
  // is the two ENDS of the move — the account line names the take, this names
  // the card the deck turned over, and `techs` names where the taken card
  // landed. The card that merely SHIFTED is on the table either way.
  //
  // A departure alone is silent: a slot emptied and not refilled (a spent deck)
  // has nothing new in it, and the strip below has already lost the card.
  const slots = new Set<string>();
  const wasOffering = cardsOnOffer(a);
  for (const o of slotKeys(b)) if (o.card !== null && !wasOffering[o.kind].has(o.card)) slots.add(o.key);

  // The order that arrived in the viewer's own hand. `cards` is empty whenever
  // the panel is masked, and absent whenever the payload holds no hand, so both
  // read as "nothing new" without a special case.
  const handNow = new Set<OrderCardId>();
  const wasInHand = new Set(hand(a)?.cards.map((c) => c.id) ?? []);
  for (const c of hand(b)?.cards ?? []) if (!wasInHand.has(c.id)) handNow.add(c.id);

  // The taken card, marked where it went. A technology is never returned in
  // Yokohama, so this set only ever grows within one game — but the read is
  // still "in `b` and not in `a`", the same direction as every grain above.
  const techs = new Set<string>();
  const wasTechs = new Map(players(a).map((p) => [p.seat, new Set(p.technologies.map((t) => t.id))]));
  for (const p of players(b)) {
    const had = wasTechs.get(p.seat);
    for (const t of p.technologies) if (!had?.has(t.id)) techs.add(`${p.seat}:${t.id}`);
  }

  // **The two halves of `any` that behave differently, and the difference is
  // worth knowing.** `hand` CAN be the only non-empty set — the Hand panel is
  // its own panel and `seatKey` never reads it — so dropping it from `any`
  // makes a card arriving in your hand a step that dwells 400 ms and pulses
  // nothing; a cell fails on it. `techs` cannot: `seatKey` carries
  // `technologies.map(t => t.id)`, so a technology arriving always moves its
  // owner's row, `techs` non-empty implies `seats` non-empty, and dropping that
  // term leaves every cell in this module green (measured). It is written
  // anyway, because `any` is a statement about the five sets and not about
  // which of them happen to be correlated today.
  return {
    tiles,
    seats,
    slots,
    techs,
    hand: handNow,
    any: tiles.size + seats.size + slots.size + techs.size + handNow.size > 0,
  };
}
