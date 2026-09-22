// Where an EXPORT's pair comes from (PR #37 fix wave, R-F1;
// `docs/plans/2026-09-22-web-frontend-m6.5b2-pr37-fixes.md`).
//
// **The Worker's `pair` arm answers by this rule, and IMPORTS it** — for
// `swRoute.ts`'s reason: `worker.ts` loads the generated wasm glue and cannot
// run in a test, and a tested model beside an untested copy is two
// implementations, the tested one not being the one that runs.
//
// ## The rule
//
// **One whole pair, from one source.** A tab's module store is loaded from
// IndexedDB when its Worker boots, and written back only for a game this tab
// HOLDS (`worker.ts`'s `persist`). So any tab that does not own a game can be
// holding a snapshot of it that another tab has since moved past — and an
// export that preferred "the Worker's snapshot whenever the Worker answers"
// handed the player a backup, a saved file or an archive copy missing moves
// IndexedDB had already committed (review F1: one event on disk, none in the
// file).
//
// The memory is still the RIGHT source in exactly one place, and it is the
// place the export exists for: the tab that owns the game may hold a move the
// store refused ("APPLIED BUT NOT SAVED"), and that move is nowhere else. So:
//
// - this tab holds the game AT THE CURRENT OWNERSHIP GENERATION → its memory;
// - anything else → the COMMITTED pair, read in one transaction.
//
// **And a store that cannot be read is not a trap** (fix round 1, review I-1):
// a holder whose generation read fails answers its memory, and a committed
// read that fails rejects as [`StoreUnreadable`], which the Worker answers as
// an ordinary reply. Only a throw from the module itself goes the trap's way.
//
// **The generation decides, not `held` alone.** With Web Locks, `held` is
// cleared by the steal's `AbortError`, which arrives asynchronously. Under the
// no-Web-Locks fallback a steal NEVER clears it — nothing tells a tab without
// locks that another tab opened the game, so its entry stays until this tab's
// own next write finds the generation moved (`lose`) or the page releases it.
// The generation is advanced inside IndexedDB by every acquisition, so it is
// the one fact every tab can read.
//
// Nothing here touches a global, so it compiles under the app project and the
// Worker project alike.

/** A game's two documents, each the exact bytes or `null` where there is none. */
export interface Pair {
  record: string | null;
  table: string | null;
}

/** Which of the two a pair is taken from. */
export type PairSource = "memory" | "store";

/**
 * The decision, and only the decision.
 *
 * @param held   the generation THIS tab took for the game, or `undefined` where
 *               it holds none
 * @param stored the generation IndexedDB holds now — `0` for a game no tab has
 *               ever held — or `null` where IndexedDB could not be read. A
 *               holder then answers memory: the store cannot say anyone else
 *               owns the game, this tab knows of no owner after itself, and
 *               its memory is the only copy of anything the store refused
 *               (fix round 1, review I-1).
 */
export function pairSource(held: number | undefined, stored: number | null): PairSource {
  if (held === undefined) return "store";
  return stored === null || held === stored ? "memory" : "store";
}

/**
 * The committed read failed: this browser's storage could not be read.
 *
 * **A type of its own so the Worker can tell it from a module fault** (fix
 * round 1, review I-1; `worker.ts`'s rule 6). A trapped module is recovered by
 * a fresh Worker, and that recovery throws the old Worker away — along with the
 * one copy of any move the store refused to keep. A store that cannot be read is
 * not a module that has trapped, so the Worker answers this as an ordinary
 * reply and the page refuses the export with its sentence. Anything else that
 * escapes [`exportPair`] — a `wasm.read` that throws — is still a trap.
 */
export class StoreUnreadable extends Error {
  constructor(cause: unknown) {
    // The MESSAGE, read by shape: IndexedDB rejects with a `DOMException`,
    // which is not an `Error` in every realm a test runs in.
    const message = (cause as { message?: unknown } | null | undefined)?.message;
    const reason =
      typeof message === "string" && message ? message : String(cause ?? "no reason given");
    super(`this browser's storage could not be read — ${reason}`);
    this.name = "StoreUnreadable";
  }
}

/**
 * The Worker's `pair` answer, assembled from [`pairSource`].
 *
 * The three reads are handed in rather than imported, so the Worker supplies
 * the real ones (`readGeneration`, `wasm.read`, `readPair`) and a cell counts
 * them. **The caller runs this inside ONE queued operation** (`worker.ts`'s
 * rule 1), so no call can move the module store between the generation read
 * and the memory read — and the memory is read in the same turn the
 * generation's answer arrives.
 *
 * - A tab that holds nothing asks no generation: there is nothing to compare.
 * - A holder whose generation read FAILS answers its memory (see
 *   [`pairSource`]'s `null`).
 * - An owner whose module has NO TABLE for the id answers the store, because
 *   the page takes this answer WHOLE and two nulls would say "no such game".
 *   A table with no record is a whole pair — a game at the deal — and is
 *   answered as it stands.
 * - A committed read that fails REJECTS with [`StoreUnreadable`]; a throw from
 *   `memory` is passed through untouched, because that one is the module's.
 */
export async function exportPair(
  held: number | undefined,
  storedGeneration: () => Promise<number>,
  memory: () => Pair,
  committed: () => Promise<Pair>,
): Promise<Pair> {
  const fromStore = async (): Promise<Pair> => {
    try {
      return await committed();
    } catch (e) {
      throw new StoreUnreadable(e);
    }
  };
  if (held === undefined) return fromStore();
  let stored: number | null;
  try {
    stored = await storedGeneration();
  } catch {
    stored = null;
  }
  if (pairSource(held, stored) === "store") return fromStore();
  const mine = memory();
  return mine.table === null ? fromStore() : mine;
}
