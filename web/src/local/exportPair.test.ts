// Where an export's pair comes from (PR #37 fix wave, R-F1).
//
// **The Worker's `pair` arm, without the Worker.** `worker.ts` imports the
// generated wasm glue and cannot be loaded in jsdom, so the rule it answers by
// lives in `exportPair.ts` and the Worker IMPORTS it — `swRoute.ts`'s reason: a
// tested model beside an untested copy is two implementations, and the tested
// one would not be the one that runs.
//
// The two pairs a cell below can be handed are different STRINGS, and nothing
// else about them matters: the claim is only ever which one came back, and that
// it came back WHOLE.

import { describe, expect, it } from "vitest";
import { exportPair, pairSource, type Pair } from "./exportPair";

/** This tab's module store: a move applied here and perhaps not yet written. */
const MEMORY: Pair = { record: '{"events":[1,2,3]}', table: '{"id":"local-1","memory":true}' };
/** What IndexedDB holds: the committed pair, whoever committed it. */
const STORE: Pair = { record: '{"events":[1,2]}', table: '{"id":"local-1","store":true}' };

/** The arm's inputs — this tab's generation and the three reads, each read
 *  COUNTED — so a cell can say not only which pair came back but which reads
 *  it took to get there. */
function arm(held: number | undefined, stored: number, memory: Pair = MEMORY) {
  const reads = { generation: 0, memory: 0, store: 0 };
  const answer = exportPair(
    held,
    async () => {
      reads.generation += 1;
      return stored;
    },
    () => {
      reads.memory += 1;
      return memory;
    },
    async () => {
      reads.store += 1;
      return STORE;
    },
  );
  return { answer, reads };
}

describe("where an export's pair comes from", () => {
  // **The recovery the export exists for** (spec §2.5): the tab that owns the
  // game may hold a move the store refused, and that move exists nowhere but
  // in this Worker's memory. Owning means the generation this tab TOOK is still
  // the one IndexedDB holds.
  it("answers this tab's memory when it holds the game at the current generation", async () => {
    expect(pairSource(3, 3)).toBe("memory");
    const { answer, reads } = arm(3, 3);
    expect(await answer).toEqual(MEMORY);
    expect(reads, "the memory, and never the store").toEqual({ generation: 1, memory: 1, store: 0 });
  });

  // **The finding itself** (review F1): a tab that took the game once and was
  // stolen from still has an entry in `held` until the steal's `AbortError`
  // is delivered — and **under the no-Web-Locks fallback (`worker.ts`'s
  // `!hasLocks()` arm) a steal never clears it at all**: nothing tells a tab
  // without locks that another tab opened the game, so `held` alone would say
  // "mine" until this tab's own next WRITE found the generation moved — and an
  // export is not a write. Only the generation tells, because every
  // acquisition advances it inside IndexedDB. So an OLDER generation is the
  // store, and this tab's memory — a snapshot the new owner has moved past — is
  // not even read.
  it("answers the STORE when it holds the game at an older generation — stolen, or the no-locks case", async () => {
    expect(pairSource(2, 3)).toBe("store");
    const { answer, reads } = arm(2, 3);
    expect(await answer).toEqual(STORE);
    expect(reads, "the stale memory is not consulted").toEqual({ generation: 1, memory: 0, store: 1 });
  });

  // A tab that does not hold the game may still have a snapshot of it from its
  // own boot — one nothing here writes back, which another tab may have moved
  // past, and which a cold read in this tab can even have advanced by a bot
  // move nobody committed. The committed pair is the one to hand out. There is
  // no generation to compare, so none is read.
  it("answers the STORE when this tab does not hold the game", async () => {
    expect(pairSource(undefined, 3)).toBe("store");
    expect(pairSource(undefined, 0), "a game no tab has ever held").toBe("store");
    const { answer, reads } = arm(undefined, 3);
    expect(await answer).toEqual(STORE);
    expect(reads).toEqual({ generation: 0, memory: 0, store: 1 });
  });

  // The owner's module has no table for the id — the shape of a 404 — and an
  // answer of two nulls would read as "no such game" to a page that now takes
  // the answer WHOLE. The committed pair is what there is.
  it("answers the store when the owner's module has no table for the game", async () => {
    const { answer, reads } = arm(3, 3, { record: null, table: null });
    expect(await answer).toEqual(STORE);
    expect(reads).toEqual({ generation: 1, memory: 1, store: 1 });
  });

  // And a game at the DEAL is a table with no record — a whole pair, taken from
  // memory as it stands rather than completed from the store.
  it("keeps the owner's table-only pair whole", async () => {
    const dealing: Pair = { record: null, table: MEMORY.table };
    const { answer } = arm(3, 3, dealing);
    expect(await answer).toEqual(dealing);
  });
});
