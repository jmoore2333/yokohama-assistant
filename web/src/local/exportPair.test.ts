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
import {
  exportPair,
  failureReply,
  fromStorage,
  pairSource,
  StoreUnreadable,
  unreadableOf,
  unreadableReply,
  type Pair,
} from "./exportPair";

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

// **An unreadable STORE is not a trap** (fix round 1, review I-1; `worker.ts`'s
// rule 6). Before R-F1 this arm read only the module, so the only thing that
// could throw here was a real module fault. It now reads IndexedDB too, and a
// read the store refuses — a connection closed by another tab's upgrade, or
// WebKit's "Connection to Indexed Database server lost" — went down the trap
// path: the page terminated the Worker, and with it the one copy of a move
// the store had already refused to keep. That is the move these exports exist
// to rescue, lost by the click meant to rescue it.
describe("when IndexedDB will not answer", () => {
  /** [`arm`], with the named read REJECTING. */
  function failing(which: "generation" | "store", held: number | undefined) {
    const reads = { generation: 0, memory: 0, store: 0 };
    const answer = exportPair(
      held,
      async () => {
        reads.generation += 1;
        if (which === "generation") {
          throw new DOMException("The database connection is closing.", "InvalidStateError");
        }
        return held ?? 0;
      },
      () => {
        reads.memory += 1;
        return MEMORY;
      },
      async () => {
        reads.store += 1;
        if (which === "store") {
          throw new DOMException("Connection to Indexed Database server lost.", "UnknownError");
        }
        return STORE;
      },
    );
    return { answer, reads };
  }

  // The store cannot say who owns the game, and this tab is the last owner it
  // knows of: a generation it TOOK. Its memory is also the only copy of
  // anything the store refused — and in both cases the review names, a store
  // that will not be read would not write either. So the owner answers memory,
  // which is the answer this arm gave before R-F1, and does not ask the store
  // that just failed.
  it("a holder whose generation read fails answers its MEMORY, and never asks the store", async () => {
    expect(pairSource(3, null), "no stored generation: the holder is the last known owner").toBe(
      "memory",
    );
    const { answer, reads } = failing("generation", 3);
    expect(await answer).toEqual(MEMORY);
    expect(reads).toEqual({ generation: 1, memory: 1, store: 0 });
  });

  // A tab that does not hold the game has no copy it can vouch for, so an
  // unreadable store is an answer it cannot give — and it is REFUSED as a
  // storage failure by name, never as a module fault, so the Worker answers it
  // as a reply and the page does not restart the host.
  it("a non-holder whose committed read fails REJECTS as unreadable storage, and memory is never read", async () => {
    expect(pairSource(undefined, null)).toBe("store");
    const { answer, reads } = failing("store", undefined);
    await expect(answer).rejects.toBeInstanceOf(StoreUnreadable);
    await expect(answer).rejects.toThrow(
      "this browser's storage could not be read — Connection to Indexed Database server lost.",
    );
    expect(reads).toEqual({ generation: 0, memory: 0, store: 1 });
  });

  // **A MODULE fault is still one** — the other half of the split, and the
  // reason the rejection is typed rather than every failure being caught: a
  // `wasm.read` that throws is a trapped module, and it must reach the trap
  // path the page recovers from.
  it("a module that throws while its memory is read is NOT reported as unreadable storage", async () => {
    const trap = new Error("RuntimeError: unreachable executed");
    const answer = exportPair(
      3,
      async () => 3,
      () => {
        throw trap;
      },
      async () => STORE,
    );
    await expect(answer).rejects.toBe(trap);
  });
});

// **The reply, and its ONE spelling** (fix round 2, re-review N-1). The Worker
// writes it and the page reads it through the two functions below, so the key
// is stated once; this is the round trip between them, and the reading of an
// ordinary pair as not-a-refusal.
describe("the unreadable reply", () => {
  it("carries the refusal's sentence from the Worker's side to the page's", () => {
    const refusal = new StoreUnreadable(new Error("Connection lost."));
    const posted = { seq: 7, ...unreadableReply(refusal) };
    expect(unreadableOf(posted)).toBe("this browser's storage could not be read — Connection lost.");
  });
  it("reads an ordinary pair, with or without a record, as no refusal at all", () => {
    expect(unreadableOf({ ...STORE })).toBeNull();
    expect(unreadableOf({ record: null, table: null })).toBeNull();
  });
});

// ---- the actor's answer to a failure, everywhere else it reads the store ----
//
// **The export arm's rule, carried to the other three places the Worker reads
// IndexedDB** (M6.5c2 cycle D, dossier row 14): a game's acquisition (its
// generation, then the rebuild's read of every pair), the Worker's startup
// (`ready`: the open, then the same rebuild) and a restore's reconciliation
// (`adopt`). `worker.ts` wraps each storage call in [`fromStorage`] and answers
// every failed request through [`failureReply`] — both imported from here for
// this file's own reason: the Worker cannot be loaded in a cell, and a tested
// model beside an untested copy would be two implementations.
describe("a failure, answered", () => {
  it("names a storage read that failed as unreadable storage, in the storage's own words", async () => {
    const failed = fromStorage(async () => {
      throw new DOMException("The database connection is closing.", "InvalidStateError");
    });
    await expect(failed).rejects.toBeInstanceOf(StoreUnreadable);
    await expect(failed).rejects.toThrow(
      "this browser's storage could not be read \u2014 The database connection is closing.",
    );
  });

  it("hands a read that worked straight through, and never re-wraps a refusal it is given", async () => {
    expect(await fromStorage(async () => 7)).toBe(7);
    const already = new StoreUnreadable(new Error("Connection lost."));
    await expect(
      fromStorage(async () => {
        throw already;
      }),
    ).rejects.toBe(already);
  });

  // The split the whole rule is for: unreadable storage is an ORDINARY reply
  // the page can draw; anything else is the module's trap, which the page
  // recovers from with a fresh Worker.
  it("answers unreadable storage as an ordinary reply, and a module fault as the trap", () => {
    const refusal = new StoreUnreadable(new Error("Connection lost."));
    const unreadable = failureReply(4, refusal);
    expect(unreadable).toEqual({ seq: 4, ...unreadableReply(refusal) });
    expect(unreadable, "never the trap").not.toHaveProperty("trapped");
    expect(unreadableOf(unreadable)).toBe("this browser's storage could not be read \u2014 Connection lost.");

    const trap = failureReply(5, new Error("RuntimeError: unreachable executed"));
    expect(trap).toEqual({
      seq: 5,
      status: 500,
      body: JSON.stringify({ error: "the local host stopped: Error: RuntimeError: unreachable executed" }),
      trapped: true,
    });
    expect(unreadableOf(trap)).toBeNull();
  });
});
