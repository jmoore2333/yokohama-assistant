// The words a lobby row says about a table.
//
// One responsibility, and it is the reason these are not two inline ternaries
// in `GameMenu` and `NewGame`: both screens list the same tables, and a lobby
// where the same table reads differently depending on which screen you are
// looking at is a lobby nobody can trust.

import { describe, expect, it } from "vitest";
import { holder, seatName, seatNames, tableWord } from "./tables";
import { wire } from "./wire/contract";
import type { TableView } from "./types";

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

  it("says your turn only when the seat to move is one of yours", () => {
    expect(tableWord(mine)).toBe("your turn");
    // The same table, read by the other player: same `to_move`, different
    // `my_seats`. The corpus row IS that reading.
    expect(wire.table.my_seats).not.toContain(wire.table.to_move);
    expect(tableWord(wire.table)).toBe("waiting on red@wire.test");
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
    expect(holder(wire.table, 1)).toBe("blue@wire.test");
  });
});

// §7.2: a row lists the seats as names, with a bot marked. `tableWord` names at
// most ONE seat — the one holding the turn — so on a dealing table it names
// nobody, which is exactly the table a player is deciding whether to join.
describe("seatNames", () => {
  it("lists every seat in seat order, in the words a row reads", () => {
    expect(seatNames(wire.games.games[0]!)).toBe("red@wire.test, blue@wire.test");
    // Seat ORDER, not the order the server happens to have listed them in:
    // which colour sits where is what a returning player is looking for.
    expect(seatNames(wire.table)).toBe("red@wire.test, blue@wire.test");
  });

  it("marks a bot seat rather than inventing a name for it", () => {
    // Derived for the same reason the cell above it is: the corpus game is
    // bot-free by construction, so no committed payload can carry this seat.
    const seats = [...wire.table.seats];
    seats[1] = { kind: "bot" };
    expect(seatNames({ ...wire.table, seats })).toBe("red@wire.test, the bot");
    expect(seatName({ kind: "bot" })).toBe("the bot");
    expect(seatName({ kind: "human", user: "red@wire.test", name: null })).toBe("red@wire.test");
    // M6.2's third arm. A seat nobody has taken is named rather than left to
    // print as `undefined`, which is what a ternary over `kind === "bot"` did.
    expect(seatName({ kind: "open" })).toBe("an open seat");
  });
});
