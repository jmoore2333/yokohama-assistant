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
import { beforeEach, describe, expect, it, vi } from "vitest";
import { deletePair, openDb, readAll, readGeneration, takeGeneration, writeDocs } from "./db";

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
    await takeGeneration(db, "local-1");
    await deletePair(db, "local-1");
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
