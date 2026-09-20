// The Worker's side channel (spec §2.2): the browser's own copy of every game
// played on this device.
//
// **One database, two stores, and the documents are opaque TEXT.** `docs` is
// keyed by `[doc, id]` and holds the layer's own documents exactly as the
// module's store wrote them — their schema is the layer's (`TABLE_SCHEMA`,
// `GameRecord`), so a version bump HERE is only ever a new store and never a
// migration of a document's shape. Parsing one on the way past would be a
// second statement of a format the Rust side owns, and the record's `seed` is a
// `u64` that `JSON.parse` rounds — the reason `recordText` never parses either.
//
// **`meta` is the OWNERSHIP GENERATION**, keyed by game id (R2). The Web Locks
// specification says a holder that is stolen from may keep executing, and a
// message telling it so cannot fence a wasm call already running or a write
// already queued behind one. A number the write's own transaction re-reads can:
// a holder writes only while the generation it took is still the stored one.
//
// This module runs in the Worker, so it is compiled by `tsconfig.worker.json`
// — no `document`, no `window`. Nothing here touches either; IndexedDB is on
// both globals.

/** Which of the layer's two documents a row holds. The same two words the
 *  module's `load`/`read` take, so a row is passed straight through. */
export type DocKind = "record" | "table";

/** One stored document. `text` is the store's exact bytes. */
export interface DocRow {
  doc: DocKind;
  id: string;
  text: string;
}

const NAME = "yokohama-local";
const VERSION = 1;
const DOCS = "docs";
const META = "meta";

/** Settle on an `IDBRequest`. */
const done = <T>(r: IDBRequest<T>): Promise<T> =>
  new Promise<T>((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error("the request failed"));
  });

/** Open it, creating both stores on a first visit.
 *
 *  A `blocked` event — another tab holding an older version open — is not
 *  possible at version 1 and would be a hang rather than an error if it were,
 *  so the open is given the same refusal path as everything else here: the
 *  caller turns a rejection into the visible line of §2.3. */
export function openDb(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(NAME, VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(DOCS)) db.createObjectStore(DOCS, { keyPath: ["doc", "id"] });
      if (!db.objectStoreNames.contains(META)) db.createObjectStore(META, { keyPath: "id" });
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error("IndexedDB would not open"));
    req.onblocked = () => reject(new Error("IndexedDB is held open by another tab"));
  });
}

/** Every stored document, for the rebuild that follows every `init()`. */
export const readAll = (db: IDBDatabase): Promise<DocRow[]> =>
  done(db.transaction(DOCS, "readonly").objectStore(DOCS).getAll() as IDBRequest<DocRow[]>);

/** What generation this game is on, or `0` for a game no tab has ever held. */
export const readGeneration = async (db: IDBDatabase, id: string): Promise<number> => {
  const row = await done(
    db.transaction(META, "readonly").objectStore(META).get(id) as IDBRequest<
      { id: string; generation: number } | undefined
    >,
  );
  return row?.generation ?? 0;
};

/**
 * Advance this game's ownership generation and answer the new value — what an
 * ACQUISITION does (spec §2.2, R2).
 *
 * Read and write in ONE transaction, so two tabs acquiring at the same instant
 * cannot both come away with the same number: IndexedDB serialises overlapping
 * `readwrite` transactions, and the loser reads what the winner wrote.
 */
export function takeGeneration(db: IDBDatabase, id: string): Promise<number> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(META, "readwrite");
    const store = t.objectStore(META);
    let next = 0;
    const get = store.get(id) as IDBRequest<{ id: string; generation: number } | undefined>;
    get.onsuccess = () => {
      next = (get.result?.generation ?? 0) + 1;
      store.put({ id, generation: next });
    };
    t.oncomplete = () => resolve(next);
    t.onabort = () => reject(t.error ?? new Error("the generation could not be taken"));
    t.onerror = () => reject(t.error ?? new Error("the generation could not be taken"));
  });
}

/**
 * Commit this game's documents — the pair, or the TABLE ALONE while the game
 * is dealing (R1) — if and only if `generation` is still the stored one (R2).
 *
 * **The generation is read INSIDE the write's own transaction**, which is what
 * makes the check a fence rather than a guess: a steal that lands between a
 * separate read and this write would be invisible, and the whole point is the
 * holder whose JS is still running after it lost the lock.
 *
 * **`"stale"` is an answer and not an error.** It means another tab owns this
 * game now, which the caller reports to the page as the loss; a refusal by the
 * STORE (quota, a private window) is a rejection instead, and the caller shows
 * that as "APPLIED BUT NOT SAVED" (§4). Two different things, and the Worker
 * does two different things with them.
 */
export function writeDocs(
  db: IDBDatabase,
  id: string,
  generation: number,
  docs: { record?: string; table: string },
): Promise<"written" | "stale"> {
  return new Promise((resolve, reject) => {
    const t = db.transaction([DOCS, META], "readwrite");
    let stale = false;
    const meta = t.objectStore(META).get(id) as IDBRequest<
      { id: string; generation: number } | undefined
    >;
    meta.onsuccess = () => {
      if ((meta.result?.generation ?? 0) !== generation) {
        stale = true;
        t.abort();
        return;
      }
      try {
        const store = t.objectStore(DOCS);
        store.put({ doc: "table", id, text: docs.table });
        if (docs.record !== undefined) store.put({ doc: "record", id, text: docs.record });
      } catch (e) {
        // A put that throws — a value that cannot be cloned — takes the ones
        // before it down with it, which is the cell's claim. `reject` first, so
        // the reason the caller sees is the real one and not `onabort`'s.
        reject(e);
        t.abort();
      }
    };
    t.oncomplete = () => resolve("written");
    // A settled promise ignores these, which is what makes the explicit
    // `reject` above the reported reason.
    t.onabort = () =>
      stale ? resolve("stale") : reject(t.error ?? new Error("the write was rolled back"));
    t.onerror = () => reject(t.error ?? new Error("the write failed"));
  });
}

/** Remove a game from this device: both documents and its generation, in one
 *  transaction. Called after the module answered 204 to a `DELETE`. */
export function deletePair(db: IDBDatabase, id: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction([DOCS, META], "readwrite");
    const docs = t.objectStore(DOCS);
    docs.delete(["record", id]);
    docs.delete(["table", id]);
    t.objectStore(META).delete(id);
    t.oncomplete = () => resolve();
    t.onabort = () => reject(t.error ?? new Error("the delete was rolled back"));
    t.onerror = () => reject(t.error ?? new Error("the delete failed"));
  });
}
