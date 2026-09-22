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
 *               ever held
 */
export function pairSource(held: number | undefined, stored: number): PairSource {
  return held !== undefined && held === stored ? "memory" : "store";
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
 * - An owner whose module has NO TABLE for the id answers the store, because
 *   the page takes this answer WHOLE and two nulls would say "no such game".
 *   A table with no record is a whole pair — a game at the deal — and is
 *   answered as it stands.
 */
export async function exportPair(
  held: number | undefined,
  storedGeneration: () => Promise<number>,
  memory: () => Pair,
  committed: () => Promise<Pair>,
): Promise<Pair> {
  if (held === undefined) return committed();
  if (pairSource(held, await storedGeneration()) === "store") return committed();
  const mine = memory();
  return mine.table === null ? committed() : mine;
}
