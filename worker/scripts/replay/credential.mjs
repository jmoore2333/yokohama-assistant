// **The live replay's credential and its bounds** (M6.5c2 cycle D, Task 9):
// one of the five parts of `live-replay.mjs`, which no script but it and the
// other parts imports. What every row is sent with, and to where — the
// credential, its two keys and the ONE origin, `../dev-vars.mjs`'s, which the
// live tier shares — and how long the run waits: a request's timeout, and the
// lobby's re-reads.

export { KEYS, ORIGIN, readCredential } from "../dev-vars.mjs";

/**
 * How many times a lobby page is read again while it differs, and how long
 * apart. The mechanism it waits out: a table's slot push to the house rides
 * `waitUntil` AFTER the table's reply (spec §2.5), so a lobby GET sent right
 * after a create — or a join, or a keep — can reach the house before the push
 * does. The pool flushes with `deliverSlot` before every house row; a live run
 * cannot, so it reads again instead. Every re-read is counted and printed with
 * this bound, and a page still differing after the last is a MISMATCH.
 */
export const LOBBY_REREADS = 5;
export const LOBBY_PAUSE_MS = 500;

export const REQUEST_TIMEOUT_MS = 60_000;
