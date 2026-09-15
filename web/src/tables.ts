// The words a lobby row says about a table.
//
// Two screens list the same tables — the setup screen's saved games and the
// banner's Game menu — and both are one line of markup around these two
// functions. Written inline they would be two ternaries drifting apart, and the
// same table would read differently depending on which screen you opened.

import type { PlayerId, SeatDoc, TableView } from "./types";

/** One seat, in the words a row reads: what the person there is CALLED, the
 *  bot, or a seat nobody has taken yet.
 *
 *  **The name, and the email only when there is no name (spec §5.6).** A
 *  display name is what a player typed about themselves, and an address is what
 *  a table is keyed by — so a lobby reading addresses tells three people at one
 *  table whose turn it is in a string none of them chose. `name` is null for a
 *  seat filled before M6.2 and for anyone who left the field blank, which is
 *  what the fallback is for.
 *
 *  A `switch` over the tag rather than a ternary, so a fourth kind of seat is a
 *  compile error here instead of a row that prints `undefined`. */
export function seatName(seat: SeatDoc): string {
  switch (seat.kind) {
    case "bot":
      return "the bot";
    case "open":
      return "open seat";
    case "human":
      return seat.name ?? seat.user;
  }
}

/** Every seat, in SEAT ORDER (spec §7.2).
 *
 *  `tableWord` names at most ONE seat — the one holding the turn — and on a
 *  dealing table it names none, which is exactly the table a player is deciding
 *  whether to sit down at. This is the other half of the question "which of my
 *  three games is this": who is at it. */
export function seatNames(t: TableView): string {
  return t.seats.map(seatName).join(", ");
}

/** Who holds one seat, in the words a row uses: a person's email, or the bot.
 *
 *  A seat number past the end reads as "nobody" rather than throwing: this is
 *  display text, and a lobby that goes blank because one row is odd is worse
 *  than a lobby with one odd row. `TableView.seats` is as long as `players` on
 *  every payload the server sends, so nothing today reaches it. */
export function holder(t: TableView, seat: PlayerId): string {
  const s = t.seats[seat];
  return s === undefined ? "nobody" : seatName(s);
}

/** What one row says the table is doing.
 *
 *  **The order is the rule.** `open` is read first because an open table is not
 *  playing, not dealing and not over, and the one thing a player wants from its
 *  row is how many seats are left. `over` comes next because a finished game
 *  still carries the seat that moved last, so reading the turn first would
 *  invite you back into a game that has ended. `dealing` is read after that
 *  because a dealing table has no `to_move` at all — the turn words have
 *  nothing to say about it. */
export function tableWord(t: TableView): string {
  if (t.status === "open") {
    const filled = t.seats.filter((s) => s.kind !== "open").length;
    return `open · ${filled} of ${t.players} seated`;
  }
  // **The winner is NAMED, through `holder`.** "over" alone is the one row a
  // player comes back to the lobby for and learns nothing from; `winner` is
  // null at every status but this one, and a tie is separated by the engine's
  // own tie-break rather than by the margin, so this is the only honest source
  // for it.
  if (t.over) return t.winner === null ? "over" : `over · ${holder(t, t.winner)} won`;
  if (t.status === "dealing") return "dealing";
  // `to_move` is present once playing (`TableView`'s own rule), and this is
  // what the row says if that ever stops being true — rather than printing a
  // seat number, or the name of seat `undefined`.
  if (t.to_move === null) return "playing";
  return t.my_seats.includes(t.to_move) ? "your turn" : `waiting on ${holder(t, t.to_move)}`;
}
