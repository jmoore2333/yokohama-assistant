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
 *  module's `load`/`read` take, so a row is passed straight through.
 *
 *  **`compat` is a THIRD kind and the module never sees one** (M6.5b2,
 *  R-6.5b2.12): it is this browser's own note about a restored pair it cannot
 *  open, in the SAME store because it is keyed the same way — `[doc, id]` —
 *  and a new store would be a version bump for one small row. Everything that
 *  hands a row to `wasm.load` filters on [`MODULE_DOCS`] rather than on
 *  "not compat", so a FOURTH kind is refused by that list rather than
 *  silently passed through. */
export type DocKind = "record" | "table" | "compat";

/** The two kinds the module's store holds. A row of any other kind is this
 *  browser's own and is never loaded. */
export const MODULE_DOCS = ["record", "table"] as const;

/** Is this a document the module can be handed? A type guard, so a caller that
 *  narrows with it may pass the result to `wasm.load` without a cast. */
export const isModuleDoc = (d: DocKind): d is "record" | "table" =>
  (MODULE_DOCS as readonly string[]).includes(d);

/** What a compatibility row says (M6.5b2, spec §2.5): what the EXPORTER said
 *  about this game, why it will not open here, and when it was put here.
 *
 *  **The pair's own bytes are never rewritten**, which is what "never migrated"
 *  means — this row sits beside them and is the only thing the listing reads
 *  for a game whose documents this build cannot parse. */
export interface CompatNote {
  /** What the file said the game was saved at, where it said anything. */
  semantics_rev: number | null;
  rules_hash: string | null;
  /** The validator's own sentence, carried and not reworded. */
  reason: string;
  /** Seconds, the clock this API uses everywhere. */
  restored_at: number;
}

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

/** What generation this game is on, or `0` for a game no tab has ever held.
 *
 *  **No production caller**: the Worker never needs to ask, because every
 *  write carries the generation it took and re-reads the stored one inside its
 *  own transaction. It exists so `db.test.ts` can read the fence's own state,
 *  the way `resetLocalHostForTests` exists for the cells. */
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

/**
 * Remove a game from this device: both documents and its generation, in one
 * transaction — and ONLY if `generation` is still the stored one.
 *
 * **The same fence as [`writeDocs`], and for a worse case** (spec §2.2, R2;
 * review finding #1). A removal is a write, and it was the one write that
 * carried no generation: a holder stolen from — `held` keeps its entry until
 * the steal's `AbortError` is delivered — could dispatch a `DELETE` in that
 * gap and take both documents AND the meta row with it. The new holder would
 * then read `stale` on its next write, be told it had lost a game that no
 * longer existed, and nothing would be recoverable. A stale delete now
 * removes nothing and is answered as the lock loss.
 */
export function deletePair(
  db: IDBDatabase,
  id: string,
  generation: number,
): Promise<"deleted" | "stale"> {
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
      const docs = t.objectStore(DOCS);
      docs.delete(["record", id]);
      docs.delete(["table", id]);
      t.objectStore(META).delete(id);
    };
    t.oncomplete = () => resolve("deleted");
    t.onabort = () =>
      stale ? resolve("stale") : reject(t.error ?? new Error("the delete was rolled back"));
    t.onerror = () => reject(t.error ?? new Error("the delete failed"));
  });
}

/** BOTH of a game's stored documents, or `null` where there is none.
 *
 *  Read in ONE transaction, for `localPair`'s reason: a pair read as two
 *  operations can fall either side of a move, and the two halves would not
 *  describe one game. */
export function readPair(
  db: IDBDatabase,
  id: string,
): Promise<{ record: string | null; table: string | null }> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(DOCS, "readonly");
    const store = t.objectStore(DOCS);
    let record: string | null = null;
    let table: string | null = null;
    const r = store.get(["record", id]) as IDBRequest<DocRow | undefined>;
    const b = store.get(["table", id]) as IDBRequest<DocRow | undefined>;
    r.onsuccess = () => {
      record = r.result?.text ?? null;
    };
    b.onsuccess = () => {
      table = b.result?.text ?? null;
    };
    t.oncomplete = () => resolve({ record, table });
    t.onabort = () => reject(t.error ?? new Error("the pair could not be read"));
    t.onerror = () => reject(t.error ?? new Error("the pair could not be read"));
  });
}

/** What became of one game in a restore (M6.5b2, R-6.5b2.11). */
export type RestoreOutcome = "restored" | "already-here" | "clash";

/**
 * Put one restored game on this device — **the check and BOTH writes in ONE
 * `readwrite` transaction** (spec §2.5).
 *
 * Reading for absence and writing afterwards is two operations, and two tabs
 * restoring at once fall between them; IndexedDB serialises overlapping
 * `readwrite` transactions over the same store, so the loser reads what the
 * winner wrote and reports `already-here` rather than overwriting it.
 *
 * The three answers, and the middle one is the point:
 *
 * - **present and byte-identical** on both halves → `already-here`, nothing
 *   written. This is what makes a restore safe to run twice when you are not
 *   sure the first one worked.
 * - **present and different, or HALF present** → `clash`, nothing written. A
 *   half-present pair is a clash and NOT an empty slot: writing the missing
 *   half beside someone else's would produce a pair that agrees with nothing.
 * - **absent** → the pair is written, with the compatibility row beside it
 *   where one was given.
 *
 * **The ownership generation is deliberately not touched.** `writeDocs`' fence
 * exists to stop a tab that has been stolen from overwriting the new holder's
 * moves; this function only ever writes where BOTH halves are absent, and a
 * game with no documents has no moves and no holder. A restored game's first
 * hold takes generation 1 exactly as a freshly created one does.
 */
export function restoreOne(
  db: IDBDatabase,
  id: string,
  docs: { record?: string; table: string; compat?: string },
): Promise<RestoreOutcome> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(DOCS, "readwrite");
    const store = t.objectStore(DOCS);
    let outcome: RestoreOutcome = "restored";
    let here: { record?: string; table?: string } = {};
    const got = (kind: "record" | "table", req: IDBRequest<DocRow | undefined>) => {
      req.onsuccess = () => {
        const text = req.result?.text;
        if (text !== undefined) here = { ...here, [kind]: text };
      };
    };
    got("record", store.get(["record", id]) as IDBRequest<DocRow | undefined>);
    got("table", store.get(["table", id]) as IDBRequest<DocRow | undefined>);
    // **The third row counts as presence too**, and this request is also what
    // sequences the decision: IndexedDB delivers a transaction's requests in
    // the order they were made, so both handlers above have run by the time
    // this one does. A lone compatibility row is a half-present game and is a
    // CLASH rather than an empty slot — the same rule as half a pair, and the
    // conservative answer for a state no path in this client produces.
    const decide = store.get(["compat", id]) as IDBRequest<DocRow | undefined>;
    decide.onsuccess = () => {
      const present =
        here.record !== undefined || here.table !== undefined || decide.result !== undefined;
      if (present) {
        const same = here.table === docs.table && here.record === docs.record;
        outcome = same ? "already-here" : "clash";
        return;
      }
      try {
        store.put({ doc: "table", id, text: docs.table });
        if (docs.record !== undefined) store.put({ doc: "record", id, text: docs.record });
        if (docs.compat !== undefined) store.put({ doc: "compat", id, text: docs.compat });
      } catch (e) {
        // A put that throws takes the ones before it down with it. `reject`
        // first, so the reason the caller sees is the real one and not
        // `onabort`'s.
        reject(e);
        t.abort();
      }
    };
    t.oncomplete = () => resolve(outcome);
    t.onabort = () => reject(t.error ?? new Error("the restore was rolled back"));
    t.onerror = () => reject(t.error ?? new Error("the restore failed"));
  });
}

/** Every compatibility row on this device — the order is the store's, and the
 *  caller sorts if it cares.
 *
 *  **A key range and not a filter over [`readAll`]**: the rows are keyed
 *  `[doc, id]`, so the store can answer this without reading a single record,
 *  and the door draws this list on every visit. */
export const readCompat = (db: IDBDatabase): Promise<DocRow[]> =>
  done(
    db
      .transaction(DOCS, "readonly")
      .objectStore(DOCS)
      .getAll(IDBKeyRange.bound(["compat", ""], ["compat", "\uffff"])) as IDBRequest<DocRow[]>,
  );

/**
 * Remove a game the module cannot open: all three rows and its generation.
 *
 * **Unfenced, and that is the difference from [`deletePair`]** — the fence is
 * about a tab that HOLDS the game, and a game the module never loaded is a game
 * no tab can hold. It is the only way to get rid of an unplayable row, which
 * spec §2.5 requires be possible without ever rewriting its bytes.
 */
export function forgetGame(db: IDBDatabase, id: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction([DOCS, META], "readwrite");
    const docs = t.objectStore(DOCS);
    docs.delete(["record", id]);
    docs.delete(["table", id]);
    docs.delete(["compat", id]);
    t.objectStore(META).delete(id);
    t.oncomplete = () => resolve();
    t.onabort = () => reject(t.error ?? new Error("the game could not be forgotten"));
    t.onerror = () => reject(t.error ?? new Error("the game could not be forgotten"));
  });
}
