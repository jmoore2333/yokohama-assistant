import { evictDurableObject, runInDurableObject } from "cloudflare:test";
import { describe, expect, it, vi } from "vitest";
import * as storageModule from "../src/storage";
import type { Table } from "../src/table";
import { RED, call, logsDuring, seedPlaying, settle } from "./helpers";

// **What a request reads** (F-4; spec §2.7 as amended; the M6.5c close §6.2).
// The live tier read ~470 rows a move at M6.5c. Most of them were not the
// documents: `hasTables` scanned the whole of `sqlite_master` inside every
// `getMeta` and every `readDocs`. These cells count what the table object's
// own SQLite answers during one request.
//
// ONE file, and its ids are its own (pool fact 5).

/**
 * Every statement `stub`'s LIVE instance runs from now until its next
 * eviction, with its cursor, so a cell can sum the rows each one read once
 * the request is over. It wraps `storage.sql.exec`, an own property of an
 * object that is no RPC receiver, as `watchBatches` wraps it (pool fact 1),
 * and an eviction discards the wrap with the instance (pool fact 4).
 */
async function watchStatements(
  stub: DurableObjectStub<Table>,
): Promise<Array<{ query: string; cursor: SqlStorageCursor<Record<string, SqlStorageValue>> }>> {
  const seen: Array<{ query: string; cursor: SqlStorageCursor<Record<string, SqlStorageValue>> }> = [];
  await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const exec = sql.exec.bind(sql);
    sql.exec = ((query: string, ...bindings: unknown[]) => {
      const cursor = exec(query, ...bindings);
      seen.push({ query, cursor });
      return cursor;
    }) as typeof sql.exec;
  });
  return seen;
}

/**
 * Every `readDocs` call made on `stub`'s OWN storage while `f` runs, counted
 * by a spy at MODULE scope (`archiveOff`'s technique in `helpers.ts`), which
 * calls through. It is filtered to the table's `SqlStorage`, because the house
 * answers in the same isolate and reads its own documents through the same
 * function. The storage is taken AFTER any eviction the cell made, so it is
 * the one the request reads.
 */
async function readsDuring(stub: DurableObjectStub<Table>, f: () => Promise<unknown>): Promise<number> {
  let own: SqlStorage | undefined;
  await runInDurableObject(stub, (_t, state) => {
    own = state.storage.sql;
  });
  const spy = vi.spyOn(storageModule, "readDocs");
  try {
    await f();
    return spy.mock.calls.filter(([sql]) => sql === own).length;
  } finally {
    spy.mockRestore();
  }
}

describe("the tables' existence is read once per object (F-4's rows; spec §2.7)", () => {
  // `hasTables` scanned the whole of `sqlite_master` (`docs`, `meta`, their
  // two autoindexes and the platform's `_cf_METADATA`) inside every `getMeta`
  // and `readDocs`. MEASURED before the memo: six scans and 32 rows read by a
  // warm `GET …/events`. Once found, the tables exist until `deleteAll()`,
  // whose one caller forgets them. Mutant: the memo removed (six scans, 32
  // rows).
  it("a warm request asks sqlite_master nothing, and reads only the rows it uses", async () => {
    const stub = await seedPlaying("reads-master");
    const seen = await watchStatements(stub);
    const res = await call(stub, RED, "GET", "/api/games/reads-master/events");
    expect(res.status, "premise: the seat's events read").toBe(200);
    const rows = seen.reduce((n, s) => n + s.cursor.rowsRead, 0);
    expect(seen.length, "premise: the wrap saw the request's SQL").toBeGreaterThan(0);
    expect(seen.filter((s) => s.query.includes("sqlite_master")).map((s) => s.query)).toEqual([]);
    expect(rows, "the documents and the meta rows the request asked for (32 before the memo)").toBeLessThanOrEqual(4);
  });

  // The guard: the memo moves no read of the documents. A warm request read
  // them once before it (the batch's comparison) and still does.
  it("a warm request reads the documents once", async () => {
    const stub = await seedPlaying("reads-warm");
    const reads = await readsDuring(stub, async () => {
      expect((await call(stub, RED, "GET", "/api/games/reads-warm/events")).status, "premise: the seat's events read").toBe(200);
    });
    expect(reads).toBe(1);
  });
});

describe("a delete asks one row whether a record is stored (Task 14's L-8; spec §2.7)", () => {
  // `deleting`, the receipt loop of every delete through the layer, read
  // every document (the record's whole text among them) only to ask whether
  // a record is stored. It asks one row now (`hasDoc`), so a cold abandon
  // reads them twice: the wake of a fresh module, and the batch's comparison.
  // The creator's abandon of a started game owes its save, so the loop
  // uploads to this file's bucket and dispatches again. MEASURED before this
  // change: 3. Mutant: `deleting` reading every document again (3).
  it("a cold abandon that saves first reads the documents at most twice: the wake and the batch", async () => {
    const id = "reads-abandon";
    const stub = await seedPlaying(id);
    await evictDurableObject(stub);
    let lines: unknown[] = [];
    const reads = await readsDuring(stub, async () => {
      ({ lines } = await logsDuring(async () => {
        const res = await call(stub, RED, "DELETE", `/api/games/${id}`);
        expect(res.status, "premise: the creator's abandon of a started game").toBe(204);
        await settle(stub);
      }));
    });
    expect(lines, "premise: the abandon saved first").toContainEqual(
      expect.objectContaining({ event: "archive-landed", id, site: "abandon", record_key: `test/rev7/abandoned/${id}.json` }),
    );
    expect(reads, "premise: the spy saw the table's own reads").toBeGreaterThan(0);
    expect(reads, "the wake and the batch (3 when the loop read its own)").toBeLessThanOrEqual(2);
  });
});
