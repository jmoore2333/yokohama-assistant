// The words a lobby row says about a table.
//
// One responsibility, and it is the reason these are not two inline ternaries
// in `GameMenu` and `NewGame`: both screens list the same tables, and a lobby
// where the same table reads differently depending on which screen you are
// looking at is a lobby nobody can trust.

import { describe, expect, it } from "vitest";
import { ago, deviceWord, holder, rowAction, rowMeta, seatChips, seatName, seatNames, tableWord } from "./tables";
import { wire } from "./wire/contract";
import type { TableView } from "./types";
import productionBot from "./wire/production_bot.json";

/** The corpus's playing table, with the seat to move handed to the CALLER.
 *
 *  **One field moved, because the corpus cannot carry this row.** The fixture
 *  walk is bot-free and drives seat order from one side, so no payload it emits
 *  has `to_move` inside `my_seats` — the row every player will see most often.
 *  `wire.table` is the shape; `my_seats` is what the server varies per caller. */
const mine: TableView = { ...wire.table, my_seats: [wire.table.to_move!] };

describe("tableWord", () => {
  it("says what a table is doing before it says whose turn it is", () => {
    // A dealing table has no `to_move` at all, so the turn words cannot apply.
    expect(tableWord(wire.tableDealing)).toBe("dealing");
    expect(tableWord(wire.games.games[0]!)).toBe("dealing");
    // **`over` wins over the turn**, and that is not cosmetic: a finished game
    // still carries the seat that moved last, so a row reading the turn first
    // would invite you back into a game that has ended.
    expect(tableWord({ ...wire.table, over: true })).toBe("over");
  });

  // M6.2's two new words (spec §5.2).
  it("counts the seats at an open table, and names the winner at a finished one", () => {
    // `open` is read BEFORE `over`, and the corpus's open table carries
    // `over: null` — so a row that read the turn words first would print
    // "playing" for a table nobody has sat down at.
    expect(tableWord(wire.tableOpen)).toBe("open · 1 of 2 seated");
    expect(tableWord(wire.games.games[1]!)).toBe("open · 1 of 2 seated");
    // **The winner is named**, which is the whole difference from "over": a
    // player comes back to the lobby to find out who won, and `winner` is the
    // engine's own answer rather than the margin.
    expect(tableWord({ ...wire.table, over: true, winner: 1 })).toBe("over · Blue won");
    expect(tableWord({ ...wire.table, over: true, winner: 0 })).toBe("over · Red won");
    // A finished table with no winner on it still reads, rather than printing
    // "over · nobody won".
    expect(tableWord({ ...wire.table, over: true, winner: null })).toBe("over");
  });

  it("says your turn only when the seat to move is one of yours", () => {
    expect(tableWord(mine)).toBe("your turn");
    // The same table, read by the other player: same `to_move`, different
    // `my_seats`. The corpus row IS that reading.
    expect(wire.table.my_seats).not.toContain(wire.table.to_move);
    // The NAME, not the address (spec §5.6): the corpus's seats are called
    // "Red" and "Blue" since M6.2, and those are what a person reads.
    expect(tableWord(wire.table)).toBe("waiting on Red");
  });

  it("names the bot rather than an email when the bot holds the turn", () => {
    // **Derived, because no corpus payload can hold a bot seat**: the fixture
    // game is bot-free by construction (`wire_fixtures.rs` — a bot seed is
    // drawn from OS entropy and a golden over it would churn every run). The
    // shape is the corpus's and `{kind: "bot"}` is the whole of the union's
    // other member, so there is nothing here a server could add a field to.
    const seats = [...wire.table.seats];
    seats[wire.table.to_move!] = { kind: "bot" };
    const vsBot: TableView = { ...wire.table, seats };
    expect(tableWord(vsBot)).toBe("waiting on the bot");
    expect(holder(vsBot, wire.table.to_move!)).toBe("the bot");
    expect(holder(wire.table, 1)).toBe("Blue");
  });
});

// §7.2: a row lists the seats as names, with a bot marked. `tableWord` names at
// most ONE seat — the one holding the turn — so on a dealing table it names
// nobody, which is exactly the table a player is deciding whether to join.
describe("seatNames", () => {
  it("lists every seat in seat order, in the words a row reads", () => {
    expect(seatNames(wire.games.games[0]!)).toBe("Red, Blue");
    // Seat ORDER, not the order the server happens to have listed them in:
    // which colour sits where is what a returning player is looking for.
    expect(seatNames(wire.table)).toBe("Red, Blue");
  });

  it("marks a bot seat rather than inventing a name for it", () => {
    // Derived for the same reason the cell above it is: the corpus game is
    // bot-free by construction, so no committed payload can carry this seat.
    const seats = [...wire.table.seats];
    seats[1] = { kind: "bot" };
    expect(seatNames({ ...wire.table, seats })).toBe("Red, the bot");
    expect(seatName({ kind: "bot" })).toBe("the bot");
    // M6.2's third arm. A seat nobody has taken is named rather than left to
    // print as `undefined`, which is what a ternary over `kind === "bot"` did.
    expect(seatName({ kind: "open" })).toBe("open seat");
  });

  // **The name where there is one, the address where there is not** (spec
  // §5.6), read off ONE payload that carries both: `wire.gamesOpen` lists a
  // table whose seat 0 is named and a table whose seat 0 is not.
  it("prefers the name a player gave and falls back to the address", () => {
    const named = wire.gamesOpen.games[0]!;
    const unnamed = wire.gamesOpen.games[1]!;
    const first = named.seats[0]!;
    const second = unnamed.seats[0]!;
    if (first.kind !== "human" || second.kind !== "human") throw new Error("fixture moved");
    // Anti-vacuity: the two rows differ in exactly the field under test, and
    // both name the same person — so what changes below is the wording and not
    // the seat.
    expect(first.name).toBe("Red");
    expect(second.name).toBeNull();
    expect(first.user).toBe(second.user);
    expect(seatName(first)).toBe("Red");
    expect(seatName(second)).toBe("red@wire.test");
    expect(seatNames(named)).toBe("Red, open seat");
    expect(seatNames(unnamed)).toBe("red@wire.test, open seat");
  });
});

// The door's row (M6.5c2, mock 10): which bot the game is against, then what it
// is waiting on — in the door's words, which are a player's and not a lobby's.
describe("deviceWord", () => {
  /** `wire.table` is read by seat 1's holder, so the bot takes seat 0 — derived
   *  for `seatNames`' reason (the corpus game is bot-free by construction), with
   *  the label the build ships. */
  const vsBot = (over: Partial<TableView> = {}): TableView => {
    const seats = [...wire.table.seats];
    seats[0] = { kind: "bot" };
    return { ...wire.table, seats, bot: { label: productionBot.label, runs_here: true }, ...over };
  };
  const v = productionBot.label;

  it("names the bot the game is played against, then what the game is waiting on", () => {
    expect(wire.table.my_seats).toEqual([1]);
    expect(deviceWord(vsBot())).toBe(`vs ${v} · waiting on the bot`);
    expect(deviceWord(vsBot({ to_move: 1 }))).toBe(`vs ${v} · your move`);
    expect(deviceWord(vsBot({ over: true, winner: 1 }))).toBe(`vs ${v} · you won`);
    expect(deviceWord(vsBot({ over: true, winner: 0 }))).toBe(`vs ${v} · the bot won`);
    const dealing = { status: "dealing" as const, to_move: null, over: null };
    expect(deviceWord(vsBot({ ...dealing, chosen: [true, false] }))).toBe(`vs ${v} · choose a card`);
    expect(deviceWord(vsBot({ ...dealing, chosen: [true, true] }))).toBe(`vs ${v} · dealing`);
    // The three branches a local game rarely shows, each ONE field moved (the
    // review's M-7). Over with no winner: `vsBot()` with `over` alone moved
    // (its `winner` is the corpus's null).
    expect(deviceWord(vsBot({ over: true }))).toBe(`vs ${v} · over`);
    // Playing with no seat to move: `vsBot()` with `to_move` alone moved.
    expect(deviceWord(vsBot({ to_move: null }))).toBe(`vs ${v} · playing`);
    // Open: the corpus's own open table with `bot` alone moved, so the row
    // reads the lobby's seat count after the version.
    expect(deviceWord({ ...wire.tableOpen, bot: { label: v, runs_here: true } })).toBe(
      `vs ${v} · open · 1 of 2 seated`,
    );
  });

  // A game dealt before its table recorded which bot (spec §2.4: "no `bot` in
  // its mirror") names none, rather than a version it cannot know.
  it("names no version where the listing carries none", () => {
    expect(deviceWord(vsBot({ bot: null, to_move: 1 }))).toBe("your move");
  });
});

// The lobby row's words (M6.5c2, R-6.5c2.7; mock 02): how long ago a table was
// opened, the line under its name, a chip per seat, and its one action.
describe("the lobby row", () => {
  /** Two days after the corpus's own tables were opened. */
  const NOW = (wire.table.created_at + 2 * 86_400) * 1000;

  it("says how long ago a table was opened", () => {
    const at = wire.table.created_at;
    expect(ago(at, (at + 30) * 1000)).toBe("just now");
    expect(ago(at, (at + 20 * 60) * 1000)).toBe("20 min ago");
    expect(ago(at, (at + 3600) * 1000)).toBe("an hour ago");
    expect(ago(at, (at + 5 * 3600) * 1000)).toBe("5 hours ago");
    expect(ago(at, (at + 86_400) * 1000)).toBe("a day ago");
    expect(ago(at, NOW)).toBe("2 days ago");
    // A device clock behind the table's reads "just now", never a negative age;
    // a stored time that is not a number still reads (`Chat.tsx`'s `when` rule).
    expect(ago(at, (at - 600) * 1000)).toBe("just now");
    expect(ago(Number.NaN, NOW)).toBe("a while ago");
  });

  it("says the table's size, then what matters from where it is read", () => {
    expect(rowMeta(wire.table, "mine", NOW)).toBe("2 players \u00b7 opened 2 days ago");
    expect(rowMeta(wire.table, "playing", NOW)).toBe("2 players");
    expect(rowMeta(wire.tableDealing, "mine", NOW)).toBe("2 players \u00b7 dealing");
    expect(rowMeta(wire.tableOpen, "mine", NOW)).toBe("2 players \u00b7 waiting for 1 more");
    expect(rowMeta(wire.tableOpen, "open", NOW)).toBe("2 players \u00b7 opened 2 days ago");
    expect(rowMeta({ ...wire.table, over: true, winner: 1 }, "mine", NOW)).toBe(
      "2 players \u00b7 over \u00b7 Blue won",
    );
  });

  // A chip per seat, in SEAT ORDER. `table_foreign.json` is the corpus's
  // playing table with the turn on its bot seat, and it names a bot this build
  // does not run: the chip reads the LISTING's label, not the build's.
  it("draws a chip per seat: you, the bot with its version, an open seat, and the turn in gold", () => {
    expect(seatChips(wire.tableForeign)).toEqual([
      { color: "Red", label: "you", open: false, toMove: false },
      { color: "Blue", label: `the bot \u00b7 ${wire.tableForeign.bot!.label} \u00b7 to move`, open: false, toMove: true },
    ]);
    // One field moved, `bot`: the same table against the bot this build ships.
    const vsBot: TableView = { ...wire.tableForeign, bot: { label: productionBot.label, runs_here: true } };
    expect(seatChips(vsBot)[1]!.label).toBe(`the bot \u00b7 ${productionBot.label} \u00b7 to move`);
    // Dealt before its table recorded which bot: no version to show (spec section
    // 2.4). One field moved, `bot`.
    expect(seatChips({ ...vsBot, bot: null })[1]!.label).toBe("the bot \u00b7 to move");
    // The corpus's undealt vs-bot table: the version, and no turn to mark.
    expect(seatChips(wire.tableBot)[1]).toEqual({
      color: "Blue",
      label: `the bot \u00b7 ${productionBot.label}`,
      open: false,
      toMove: false,
    });
    expect(seatChips(wire.tableOpen)[1]).toEqual({ color: "Blue", label: "open seat", open: true, toMove: false });
    // A dealing table has no turn to mark, and a finished one none either.
    expect(seatChips(wire.tableDealing).some((c) => c.toMove)).toBe(false);
    expect(seatChips({ ...wire.table, over: true }).some((c) => c.toMove)).toBe(false);
  });

  it("offers one action per row, and nothing to open where this server cannot continue the game", () => {
    expect(rowAction(wire.tableOpen, "mine")).toEqual({ kind: "copy" });
    expect(rowAction(mine, "mine")).toEqual({ kind: "go", label: "Play", primary: true, padlock: false });
    expect(rowAction(wire.table, "mine")).toEqual({ kind: "go", label: "Play", primary: false, padlock: false });
    // A card still owed at the deal is gold too; `wire.tableDealing`'s caller
    // (seat 1) has not chosen.
    expect(wire.tableDealing.chosen[1]).toBe(false);
    expect(rowAction(wire.tableDealing, "mine")).toMatchObject({ label: "Play", primary: true });
    expect(rowAction({ ...wire.table, over: true }, "mine")).toMatchObject({ kind: "go", label: "View" });
    expect(rowAction(wire.tableOpen, "open")).toEqual({ kind: "go", label: "Take a seat", primary: false, padlock: true });
    expect(rowAction(wire.tableWatchable, "playing")).toEqual({
      kind: "go",
      label: "Spectate",
      primary: false,
      padlock: true,
    });
    expect(rowAction({ ...wire.tableWatchable, can_watch: true }, "playing")).toMatchObject({ padlock: false });
    // A bot this build does not run: nothing to open while the game runs, and
    // the finished game opens as ever (spec section 2.4, R-6.5c2.10). The corpus's
    // own foreign table; the second read moves one field, `over`.
    expect(wire.tableForeign.bot!.runs_here).toBe(false);
    expect(rowAction(wire.tableForeign, "mine")).toEqual({ kind: "stranded" });
    expect(rowAction({ ...wire.tableForeign, over: true }, "mine")).toMatchObject({ kind: "go", label: "View" });
  });
});
