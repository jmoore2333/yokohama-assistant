// What every list says about a table, beside whatever control each offers.
//
// **The facts, not the whole row.** The lists genuinely differ around this: the
// Game menu's row is a `.game-row` with a switch button, the lobby's is an
// `<li>` with a link button and, in the open section, a lock marker. What is the
// same is this line — the size, what the table is waiting on, and who is at it —
// and stating it twice is how one list ends up saying something the other does
// not. The WORDS live in `tables.ts`, shared by all of them for the same reason;
// this is their markup.

import { seatNames, tableWord } from "./tables";
import type { TableView } from "./types";

export function TableFacts({ table }: { table: TableView }) {
  return (
    <span className="muted table-facts">
      {table.players}P &middot; {tableWord(table)} &middot; {seatNames(table)}
    </span>
  );
}
