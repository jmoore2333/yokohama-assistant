// The Worker's side channel, on a real IndexedDB implementation.
//
// **`fake-indexeddb` and not a hand-written double**, because what these cells
// are about is transaction SEMANTICS: that a pair lands or neither does, that a
// generation read and the puts that follow it commit together, and that an
// abort leaves the store as it was. A double built from a `Map` would answer
// every one of those questions the way its author expected rather than the way
// the specification says, which is the whole of what `writeDocs` leans on.
//
// The import must come FIRST: `fake-indexeddb/auto` installs the globals at
// module scope, and `db.ts` reads `indexedDB` when a function is called rather
// than when it is loaded, so order only matters against the other imports'
// side effects — it is written first anyway, as the package's own instructions
// say.
import "fake-indexeddb/auto";
import { IDBFactory } from "fake-indexeddb";
import { waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  DB_NAME,
  deleteOutboxRow,
  deletePair,
  forgetGame,
  openDb,
  putOutboxRow,
  readAll,
  readGeneration,
  readOutbox,
  readOutboxRow,
  readCompat,
  readPair,
  restoreOne,
  retryingStart,
  takeGeneration,
  writeDocs,
  type CompatNote,
} from "./db";
import { wire } from "../wire/contract";

/** Let every already-queued microtask run, with no timer advanced — what a
 *  cell asserting that a promise has NOT settled has to wait out. */
const flushMicrotasks = async (): Promise<void> => {
  for (let i = 0; i < 20; i += 1) await Promise.resolve();
};

beforeEach(() => {
  // A FRESH database per cell. `fake-indexeddb/auto` installs one factory for
  // the whole run, so without this every cell would inherit the last one's
  // documents and its generations — and the cells here are about exactly those
  // two numbers.
  vi.stubGlobal("indexedDB", new IDBFactory());
});

describe("the local document store", () => {
  it("writes a pair and reads back the same two strings", async () => {
    const db = await openDb();
    expect(await writeDocs(db, "local-1", 0, { record: "R", table: "T" })).toBe("written");
    const rows = await readAll(db);
    expect(rows.map((r) => [r.doc, r.id, r.text]).sort()).toEqual([
      ["record", "local-1", "R"],
      ["table", "local-1", "T"],
    ]);
  });

  it("writes the TABLE ALONE for a dealing game, and adds the record later", async () => {
    // R1: `create` writes no record — it appears at the last keep — so a
    // persist that insisted on a pair would keep nothing for a game still
    // dealing, and the hold that follows would rebuild without it and 404 the
    // game the player just opened.
    const db = await openDb();
    await writeDocs(db, "local-1", 0, { table: "T" });
    expect((await readAll(db)).map((r) => r.doc)).toEqual(["table"]);
    await writeDocs(db, "local-1", 0, { record: "R", table: "T2" });
    const rows = await readAll(db);
    expect(rows.find((r) => r.doc === "record")?.text).toBe("R");
    expect(rows.find((r) => r.doc === "table")?.text).toBe("T2");
  });

  it("deletes both documents AND the game's generation", async () => {
    const db = await openDb();
    await writeDocs(db, "local-1", 0, { record: "R", table: "T" });
    await writeDocs(db, "local-2", 0, { record: "R2", table: "T2" });
    const mine = await takeGeneration(db, "local-1");
    expect(await deletePair(db, "local-1", mine)).toBe("deleted");
    expect((await readAll(db)).map((r) => r.id)).toEqual(["local-2", "local-2"]);
    // The generation goes with the game. A row left behind would make the NEXT
    // game drawn under the same id (a collision a 64-bit draw does not make,
    // but the store is keyed on a string a caller supplies) start at a
    // generation no holder had taken.
    expect(await readGeneration(db, "local-1")).toBe(0);
    expect(await readGeneration(db, "local-2")).toBe(0);
  });

  it("leaves NEITHER document when a put inside the pair throws", async () => {
    const db = await openDb();
    await writeDocs(db, "local-1", 0, { record: "old", table: "old" });
    // The second put throws — a structured-clone failure is the real shape of
    // this — and the transaction must take the first one down with it.
    let puts = 0;
    const store = IDBTransaction.prototype.objectStore;
    vi.spyOn(IDBTransaction.prototype, "objectStore").mockImplementation(function (
      this: IDBTransaction,
      name: string,
    ) {
      const s = store.call(this, name);
      if (name !== "docs") return s;
      const put = s.put.bind(s);
      s.put = ((value: unknown, key?: IDBValidKey) => {
        puts += 1;
        if (puts === 2) throw new DOMException("could not be cloned", "DataCloneError");
        return put(value, key);
      }) as typeof s.put;
      return s;
    });
    await expect(writeDocs(db, "local-1", 0, { record: "new", table: "new" })).rejects.toThrow(
      /cloned/,
    );
    vi.restoreAllMocks();
    // Both documents still read what they did before the attempt: the first put
    // was rolled back with the second.
    const rows = await readAll(db);
    expect(rows.map((r) => r.text)).toEqual(["old", "old"]);
  });

  it("refuses the write as STALE when this tab's generation is not the stored one", async () => {
    // R2: the Web Locks specification warns that a stolen-from holder may keep
    // executing, and a JS notification cannot fence a running wasm call or a
    // queued write. This is the fence that can.
    const db = await openDb();
    await writeDocs(db, "local-1", 0, { record: "R", table: "T" });
    expect(await takeGeneration(db, "local-1")).toBe(1);
    expect(await takeGeneration(db, "local-1")).toBe(2);
    // The holder at generation 1 has been stolen from.
    expect(await writeDocs(db, "local-1", 1, { record: "stale", table: "stale" })).toBe("stale");
    expect((await readAll(db)).map((r) => r.text)).toEqual(["R", "T"]);
    // And the current holder writes.
    expect(await writeDocs(db, "local-1", 2, { record: "R2", table: "T2" })).toBe("written");
    expect((await readAll(db)).map((r) => r.text)).toEqual(["R2", "T2"]);
  });

  it("refuses a STALE holder's delete, and takes nothing with it", async () => {
    // Review finding #1. A removal is a write, and it was the one write that
    // carried no generation: `held` keeps its entry until the steal's
    // `AbortError` is delivered, so a holder stolen from could dispatch a
    // DELETE in that gap and take the NEW holder's documents and its meta row
    // with it — after which the new holder reads `stale` on its next write and
    // is told it lost a game that no longer exists.
    const db = await openDb();
    await writeDocs(db, "local-1", 0, { record: "R", table: "T" });
    const stolenFrom = await takeGeneration(db, "local-1");
    const theNewHolder = await takeGeneration(db, "local-1");

    expect(await deletePair(db, "local-1", stolenFrom)).toBe("stale");
    expect((await readAll(db)).map((r) => r.text)).toEqual(["R", "T"]);
    // And the generation is untouched, so the new holder still writes.
    expect(await readGeneration(db, "local-1")).toBe(theNewHolder);
    expect(await writeDocs(db, "local-1", theNewHolder, { record: "R2", table: "T2" })).toBe(
      "written",
    );
    // The holder that really does own it removes the lot.
    expect(await deletePair(db, "local-1", theNewHolder)).toBe("deleted");
    expect(await readAll(db)).toEqual([]);
    expect(await readGeneration(db, "local-1")).toBe(0);
  });

  it("takes a generation per GAME, and a fresh game starts at 0 so a create can write", async () => {
    // P-4: a create's 201 is persisted under generation 0 — nothing holds an id
    // that was drawn a moment ago, and the write must land or the hold that
    // follows rebuilds without the table and 404s the new game.
    const db = await openDb();
    expect(await readGeneration(db, "local-new")).toBe(0);
    expect(await writeDocs(db, "local-new", 0, { table: "T" })).toBe("written");
    expect(await takeGeneration(db, "local-new")).toBe(1);
    expect(await takeGeneration(db, "local-other")).toBe(1);
    expect(await readGeneration(db, "local-new")).toBe(1);
  });
});

// ---- the storage version, and an upgrade that can be BLOCKED (M6.5b2 §2.6) --
//
// **The obvious cell is not the real case.** Opening version 1, closing it and
// then opening version 2 tests nothing that matters: a real M6.5b tab holds its
// connection for the Worker's whole life and has NO `versionchange` handler to
// close it — that code is shipped and cannot be retrofitted. So the cells below
// arrange that holder by hand, with a raw `indexedDB.open(DB_NAME, 1)` that
// installs no handler at all, which is what the shipped code is.
describe("the outbox store's version bump", () => {
  /** An M6.5b-shaped holder: version 1, no `versionchange` handler, held open
   *  exactly as the shipped Worker holds it. */
  const holdVersionOne = (): Promise<IDBDatabase> =>
    new Promise((resolve, reject) => {
      const r = indexedDB.open(DB_NAME, 1);
      r.onupgradeneeded = () => {
        const db = r.result;
        if (!db.objectStoreNames.contains("docs"))
          db.createObjectStore("docs", { keyPath: ["doc", "id"] });
        if (!db.objectStoreNames.contains("meta")) db.createObjectStore("meta", { keyPath: "id" });
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });

  it("creates the outbox store on a FRESH database", async () => {
    const db = await openDb();
    expect(db.version).toBe(2);
    expect([...db.objectStoreNames].sort()).toEqual(["docs", "meta", "outbox"]);
  });

  // The upgrade path, which is the one every browser that has played a local
  // game takes: the two stores are already there with documents in them, and
  // the bump adds a store beside them WITHOUT touching a document's shape.
  it("adds the outbox store to a version-1 database and keeps its documents", async () => {
    const one = await holdVersionOne();
    await new Promise<void>((resolve, reject) => {
      const t = one.transaction("docs", "readwrite");
      t.objectStore("docs").put({ doc: "record", id: "local-1", text: "R" });
      t.oncomplete = () => resolve();
      t.onerror = () => reject(t.error);
    });
    one.close();

    const db = await openDb();
    expect(db.version).toBe(2);
    expect([...db.objectStoreNames].sort()).toEqual(["docs", "meta", "outbox"]);
    expect((await readAll(db)).map((r) => r.text)).toEqual(["R"]);
  });

  it("says what is blocking it, and recovers when that tab goes", async () => {
    const held = await holdVersionOne();
    const blocked = vi.fn();
    const open = openDb({ onBlocked: blocked });
    await waitFor(() => expect(blocked).toHaveBeenCalled());
    // The sentence names the other tab, because that is the thing the player
    // can do something about — clearing site data is not the remedy.
    expect(blocked.mock.calls[0]![0]).toMatch(/open in another tab/i);
    // **And it has NOT rejected**: a rejection here is what made the Worker
    // cache a permanent failure, so the open waits instead.
    let settled = false;
    void open.then(
      () => (settled = true),
      () => (settled = true),
    );
    await flushMicrotasks();
    expect(settled).toBe(false);

    held.close();
    const db = await open;
    expect(db.version).toBe(2);
  });

  // **This is the LAST upgrade that can be blocked this way**, and that is the
  // difference between the old code and the new: every connection this module
  // hands out closes itself when another tab asks for a newer version.
  it("closes its own connection on versionchange, so the NEXT upgrade is never blocked", async () => {
    const mine = await openDb();
    expect(mine.version).toBe(2);
    const blocked = vi.fn();
    const three = await new Promise<IDBDatabase>((resolve, reject) => {
      const r = indexedDB.open(DB_NAME, 3);
      r.onupgradeneeded = () => undefined;
      r.onblocked = blocked;
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });
    expect(blocked).not.toHaveBeenCalled();
    expect(three.version).toBe(3);
    three.close();
  });

  // **A start that fails is not the host's permanent answer** (spec §2.6). The
  // Worker's startup promise is cached for the life of the Worker, so a single
  // rejection used to make every later message fail for a reason that had gone
  // away. A resolved start IS cached — it is what "start once" means.
  it("caches a start that worked and RETRIES one that did not", async () => {
    let attempts = 0;
    const start = retryingStart(async () => {
      attempts += 1;
      if (attempts === 1) throw new Error("blocked");
      return attempts;
    });
    await expect(start()).rejects.toThrow("blocked");
    await expect(start()).resolves.toBe(2);
    await expect(start()).resolves.toBe(2);
    expect(attempts).toBe(2);
  });

  it("runs the start ONCE for callers that arrive together", async () => {
    let attempts = 0;
    const start = retryingStart(async () => {
      attempts += 1;
      return attempts;
    });
    expect(await Promise.all([start(), start(), start()])).toEqual([1, 1, 1]);
    expect(attempts).toBe(1);
  });
});

// ---- a game's rows are ONE list, and the outbox is now one of them ----------
describe("the outbox rows of a game", () => {
  it("writes, reads back and removes one row", async () => {
    const db = await openDb();
    expect(await readOutbox(db)).toEqual([]);
    await putOutboxRow(db, { id: "local-1", state: "pending", reason: "offline", lastTried: 7 });
    expect(await readOutboxRow(db, "local-1")).toEqual({
      id: "local-1",
      state: "pending",
      reason: "offline",
      lastTried: 7,
    });
    await deleteOutboxRow(db, "local-1");
    expect(await readOutboxRow(db, "local-1")).toBeNull();
  });

  // **[`GAME_DOCS`]'s rule, widened** (review I-1's lesson): a game's rows are
  // one list, and the two deletes must not disagree about what a game's rows
  // ARE. An outbox row left behind after a delete is a row naming a game that
  // is gone, and the door would list it as waiting to be sent for ever.
  it("goes with the game through BOTH deletes", async () => {
    const db = await openDb();
    await writeDocs(db, "local-1", 0, { record: "R", table: "T" });
    await putOutboxRow(db, { id: "local-1", state: "pending", reason: "offline", lastTried: 0 });
    expect(await deletePair(db, "local-1", 0)).toBe("deleted");
    expect(await readOutbox(db)).toEqual([]);

    await writeDocs(db, "local-2", 0, { record: "R", table: "T" });
    await putOutboxRow(db, { id: "local-2", state: "sent", at: 1, fingerprint: "1:2" });
    await forgetGame(db, "local-2");
    expect(await readOutbox(db)).toEqual([]);
  });
});

// ---- a compatibility row is RE-DERIVED at restore (M6.5c2 cycle D, row 16) --
//
// **The state a `SEMANTICS_REV` bump leaves behind.** A build at rev N-1 that
// was handed a rev-N game kept it with a row saying why it would not open, and
// `already-here` wrote nothing — so once this device plays rev N, the row goes
// on hiding a game the module opens happily, under a reason that has stopped
// being true. Staged today with a row naming the CURRENT rev, which is exactly
// what that row says after the bump: the note that made "no way to test" the
// reason to leave it (the old doc on `restoreOne`) was too strong.
describe("a compatibility row across a revision bump", () => {
  /** This build's revision, as the corpus's own health payload states it. */
  const REV = wire.health.semantics_rev;
  /** What the build BEFORE the bump wrote about a game saved at this one's rev. */
  const staleNote: CompatNote = {
    semantics_rev: REV,
    rules_hash: null,
    reason: `saved at semantics rev ${REV}; this app plays rev ${REV - 1}`,
    restored_at: 0,
  };

  it("drops a row naming the CURRENT rev at the next restore, and the game is listed as playable", async () => {
    const db = await openDb();
    const pair = { record: "R", table: "T" };
    expect(await restoreOne(db, "local-old", { ...pair, compat: JSON.stringify(staleNote) })).toBe(
      "restored",
    );
    expect(
      (await readCompat(db)).map((r) => r.id),
      "the premise: the row the earlier build wrote is on this device",
    ).toEqual(["local-old"]);

    // The same file, restored again on THIS build: the validator no longer
    // calls the game unplayable, so the caller hands no compatibility note.
    expect(await restoreOne(db, "local-old", pair), "the game opens here now").toBe("restored");
    // **Listed as playable**: the one thing that hides a row from the door's
    // playable list is this store's compatibility row (`unplayableIds`).
    expect(await readCompat(db)).toEqual([]);
    // The documents are the file's exact bytes, never rewritten.
    expect(await readPair(db, "local-old")).toEqual(pair);
    // And it is still safe to run twice: nothing left to re-derive.
    expect(await restoreOne(db, "local-old", pair)).toBe("already-here");
  });
});
