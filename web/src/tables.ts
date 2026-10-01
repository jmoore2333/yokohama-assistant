// The words a lobby row says about a table.
//
// Two screens list the same tables — the setup screen's saved games and the
// banner's Game menu — and both are one line of markup around these two
// functions. Written inline they would be two ternaries drifting apart, and the
// same table would read differently depending on which screen you opened.

import type { PlayerId, SeatDoc, TableView } from "./types";

/** One seat, in the words a row reads: a person's email, or the bot. */
export function seatName(seat: SeatDoc): string {
  return seat.kind === "bot" ? "the bot" : seat.user;
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
 *  **The order is the rule.** `over` is read first because a finished game
 *  still carries the seat that moved last, so reading the turn first would
 *  invite you back into a game that has ended. `dealing` is read next because a
 *  dealing table has no `to_move` at all — the turn words have nothing to say
 *  about it. */
export function tableWord(t: TableView): string {
  if (t.over) return "over";
  if (t.status === "dealing") return "dealing";
  // `to_move` is present once playing (`TableView`'s own rule), and this is
  // what the row says if that ever stops being true — rather than printing a
  // seat number, or the name of seat `undefined`.
  if (t.to_move === null) return "playing";
  return t.my_seats.includes(t.to_move) ? "your turn" : `waiting on ${holder(t, t.to_move)}`;
}
