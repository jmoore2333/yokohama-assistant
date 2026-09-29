// The words a screen says about a table: what one seat is called, what the
// table is doing, and a lobby row's line, chips and one action.
//
// Several screens read the same tables, and each reads them through here:
// `TableFacts` (the banner's Game menu and the unlock face) through `tableWord`
// and `seatNames`; the dealing and open-table screens through `seatName` and
// `holder`; the front door's device card through `deviceWord`, which words the
// same facts as a player's rather than a lobby's (M6.5c2); and the lobby's rows
// through `rowMeta`, `seatChips` and `rowAction` (M6.5c2, R-6.5c2.7). Written
// inline they would be ternaries drifting apart, and the same table would read
// differently depending on which screen you opened.

import type { PlayerColor, PlayerId, SeatView, TableView } from "./types";

/** One seat, in the words a row reads: what the person there is CALLED, the
 *  bot, or a seat nobody has taken yet.
 *
 *  **The name; and with no name, "you", the address the creator typed, or
 *  "invited"** (spec §5.6; M6.5c2 cycle D, R-6.5c2.35). A display name is what a
 *  player typed about themselves. The server hands this reader no other
 *  person's address: `invited` is the one it does — to the table's creator,
 *  for a seat nobody has named — and every other reader of an unnamed seat
 *  reads the word. `name` is null for a seat filled before M6.2 and for anyone
 *  who left the field blank, which is what the fallbacks are for.
 *
 *  A `switch` over the tag rather than a ternary, so a fourth kind of seat is a
 *  compile error here instead of a row that prints `undefined`. */
export function seatName(seat: SeatView): string {
  switch (seat.kind) {
    case "bot":
      return "the bot";
    case "open":
      return "open seat";
    case "human":
      return seat.name ?? (seat.you ? "you" : seat.invited ?? "invited");
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

/** Who holds one seat, in the words a row uses: `seatName`'s, never an
 *  address this reader was not handed.
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

/** What a game ON THIS DEVICE is waiting on, in the door's words (M6.5c2,
 *  mocks 01 and 10): the bot it is played against, then its state.
 *
 *  A local game is you and the bot, so its words are a player's rather than a
 *  lobby's: "your move", "you won", "the bot won". The version is the
 *  LISTING's (`TableView.bot`), never a parse of the record; a game dealt
 *  before its table recorded which bot names none (spec §2.4). */
export function deviceWord(t: TableView): string {
  const vs = t.bot ? `vs ${t.bot.label} · ` : "";
  if (t.over) {
    if (t.winner === null) return `${vs}over`;
    return `${vs}${t.my_seats.includes(t.winner) ? "you won" : `${holder(t, t.winner)} won`}`;
  }
  if (t.status === "dealing") {
    return `${vs}${t.my_seats.some((s) => !t.chosen[s]) ? "choose a card" : "dealing"}`;
  }
  if (t.status === "open") return `${vs}${tableWord(t)}`;
  if (t.to_move === null) return `${vs}playing`;
  return `${vs}${t.my_seats.includes(t.to_move) ? "your move" : `waiting on ${holder(t, t.to_move)}`}`;
}

/** Which of the lobby's lists a row is in (M6.5c2, R-6.5c2.7): its three,
 *  and the Finished group folded under the third (cycle C, R-6.5c2.22) — the
 *  finished games the caller does NOT sit at. A finished game you sit at is
 *  read under `"mine"`, because the lobby filters `notMine` first. */
export type Section = "mine" | "open" | "playing" | "finished";

/** How long ago a table was opened, in a row's words (mock 02: "opened 2 days
 *  ago"). `createdAt` is the wire's Unix SECONDS and `now` milliseconds, the
 *  clock's own. A stored time is DATA, so a value that is not a number reads
 *  rather than throws (`Chat.tsx`'s `when` rule), and a device clock behind
 *  the server's reads "just now" rather than a negative age. */
export function ago(createdAt: number, now: number = Date.now()): string {
  const seconds = Math.floor(now / 1000) - createdAt;
  if (!Number.isFinite(seconds)) return "a while ago";
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return hours === 1 ? "an hour ago" : `${hours} hours ago`;
  const days = Math.floor(hours / 24);
  return days === 1 ? "a day ago" : `${days} days ago`;
}

/** The line under a row's name: the table's size, then what matters about it
 *  from the list it is read in (mock 02). Yours still filling says how many it
 *  waits for; somebody else's says how fresh it is; a game being played says
 *  its size alone, because its chips say who is moving. */
export function rowMeta(t: TableView, section: Section, now: number = Date.now()): string {
  const size = `${t.players} players`;
  if (t.over) return `${size} \u00b7 ${tableWord(t)}`;
  if (t.status === "dealing") return `${size} \u00b7 dealing`;
  if (t.status === "open") {
    const free = t.seats.filter((s) => s.kind === "open").length;
    return section === "mine" ? `${size} \u00b7 waiting for ${free} more` : `${size} \u00b7 opened ${ago(t.created_at, now)}`;
  }
  return section === "playing" ? size : `${size} \u00b7 opened ${ago(t.created_at, now)}`;
}

/** One seat's chip, as a row draws it (`Chips.tsx`'s `SeatChip` props). */
export interface ChipModel {
  color: PlayerColor;
  label: string;
  open: boolean;
  toMove: boolean;
}

/** A chip per seat, in SEAT ORDER (mock 02): your own seats say "you", the bot
 *  says WHICH bot where the table records it (R-6.5c2.10, mock 10: the label
 *  is the LISTING's, `TableView.bot`, never the build's), an empty seat says
 *  so, and the seat holding the turn says so in gold: "your move" when it is
 *  yours. A dealing table has no turn to mark, and a finished one none either. */
export function seatChips(t: TableView): ChipModel[] {
  const turn = t.status === "playing" && !t.over ? t.to_move : null;
  return t.seats.map((seat, i) => {
    const mine = t.my_seats.includes(i);
    const who =
      seat.kind === "bot"
        ? t.bot
          ? `the bot \u00b7 ${t.bot.label}`
          : "the bot"
        : seat.kind === "human" && mine
          ? "you"
          : seatName(seat);
    const toMove = turn === i;
    return {
      color: t.colors[i]!,
      label: toMove ? `${who} \u00b7 ${mine ? "your move" : "to move"}` : who,
      open: seat.kind === "open",
      toMove,
    };
  });
}

/** Is this a game THIS BUILD cannot continue? Dealt a bot this build does not
 *  run, and not over — every game route refuses it (spec section 2.4), while a
 *  finished one opens as it always did (R-6.5c2.10).
 *
 *  **One sentence, read everywhere a game is offered** (M6.5c2 cycle C; the
 *  fix wave review's concern (a)): the lobby's row (`rowAction`), the door's
 *  "Will not open here" (`OnThisDevice`) and the Game menu's saved games, so a
 *  game one of them sets aside is never offered by another. */
export const isStranded = (t: TableView): boolean => Boolean(t.bot && !t.bot.runs_here && !t.over);

/** A row's one control (R-6.5c2.7), or why it has none. */
export type RowAction =
  | { kind: "go"; label: "Play" | "View" | "Take a seat" | "Spectate"; primary: boolean; padlock: boolean }
  | { kind: "copy" }
  | { kind: "stranded" };

/** What a row's one control does.
 *
 *  **A game this server cannot continue** (its bot is one this build does not
 *  run, and it is not over) offers nothing to open: every game route refuses
 *  it (spec section 2.4), and its creator can still abandon it from the row. A
 *  finished one opens as ever. Yours: a card owed or your move is the gold
 *  Play; a table still filling is its link, to send. Somebody else's finished
 *  game is VIEWED rather than spectated (R-6.5c2.22): nothing on it is still
 *  happening. The padlock says a password is still wanted: at an open table
 *  whenever it is locked, at a game (finished or not) only until this caller
 *  has given it (`can_watch`). */
export function rowAction(t: TableView, section: Section): RowAction {
  if (isStranded(t)) return { kind: "stranded" };
  if (section === "mine") {
    if (t.status === "open") return { kind: "copy" };
    if (t.over) return { kind: "go", label: "View", primary: false, padlock: false };
    const owed =
      t.status === "dealing"
        ? t.my_seats.some((s) => !t.chosen[s])
        : t.to_move !== null && t.my_seats.includes(t.to_move);
    return { kind: "go", label: "Play", primary: owed, padlock: false };
  }
  if (section === "open") return { kind: "go", label: "Take a seat", primary: false, padlock: t.locked };
  if (section === "finished") return { kind: "go", label: "View", primary: false, padlock: t.locked && !t.can_watch };
  return { kind: "go", label: "Spectate", primary: false, padlock: t.locked && !t.can_watch };
}
