// What changed between two positions (M5.5 spec §3.2.4).
//
// A pure comparison of two wire frames and nothing else: the walk lights the
// tiles, seats and slots this names, and it must never need a rule to do so —
// a rule restated here would be a second dialect of the engine.

import type { AreaId, HistoricalFrame, Panel, PlayerId, PlayerModel, TileModel } from "./types";

export interface Changes {
  tiles: Set<AreaId>;
  seats: Set<PlayerId>;
  /** `port:<board>:<slot>` and `lab:<board>:<slot>`. */
  slots: Set<string>;
  any: boolean;
}

/** The parts of a tile a piece or a build can change. `reachable` is a
 *  property of the legal list, not of the table, and is deliberately absent. */
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

const players = (f: HistoricalFrame): PlayerModel[] =>
  f.frame.dashboard.find((p): p is Extract<Panel, { Players: unknown }> => "Players" in p)
    ?.Players.players ?? [];

const slotKeys = (f: HistoricalFrame): [string, number | null][] => [
  ...f.ports.boards.flatMap((b, i) => b.slots.map((s): [string, number | null] => [`port:${i}:${s.slot}`, s.card])),
  ...f.labs.boards.flatMap((b, i) => b.slots.map((s): [string, number | null] => [`lab:${i}:${s.slot}`, s.card])),
];

export function diffFrames(a: HistoricalFrame, b: HistoricalFrame): Changes {
  const tiles = new Set<AreaId>();
  const wasTile = new Map(a.frame.board.tiles.map((t) => [t.area, tileKey(t)]));
  for (const t of b.frame.board.tiles) if (wasTile.get(t.area) !== tileKey(t)) tiles.add(t.area);

  const seats = new Set<PlayerId>();
  const wasSeat = new Map(players(a).map((p) => [p.seat, seatKey(p)]));
  for (const p of players(b)) if (wasSeat.get(p.seat) !== seatKey(p)) seats.add(p.seat);

  const slots = new Set<string>();
  const wasSlot = new Map(slotKeys(a));
  for (const [key, card] of slotKeys(b)) if (wasSlot.get(key) !== card) slots.add(key);

  return { tiles, seats, slots, any: tiles.size + seats.size + slots.size > 0 };
}
