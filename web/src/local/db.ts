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

/** **Every row a game has, in ONE place** (M6.5b2, review I-1).
 *
 *  Both deletes remove exactly these, and they used to disagree: `deletePair`
 *  took the pair and the generation while `forgetGame` took the compatibility
 *  row too, so abandoning a restored cross-rev game through the fenced path
 *  left a row in "Will not open here" with no documents behind it — listed,
 *  with a Back up that threw. The list is the rule, and a fifth kind is added
 *  here rather than at two call sites. */
export const GAME_DOCS = [...MODULE_DOCS, "compat"] as const;

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

/** What a game is WAITING to do, keyed by game id (M6.5b2, spec §2.6).
 *
 *  **Two shapes in one store, because they are one statement about one game**:
 *  either the archive has not been reached yet and this row says why, or a
 *  send has LANDED and this row is the receipt. A separate receipt store would
 *  let a game be both at once, which is a state no reader knows what to do
 *  with.
 *
 *  `PendingReason` lives here rather than in `outbox.ts` for the reason
 *  [`CompatNote`] does: this module owns every shape this database holds, and
 *  the Worker compiles it without the page's imports. */
export type PendingReason = "offline" | "signed-out" | "unknown" | "refused";

export type OutboxRow =
  | {
      id: string;
      state: "pending";
      reason: PendingReason;
      /** Milliseconds, the clock a "last tried" line is read from. */
      lastTried: number;
      /** The SERVER's own sentence, carried and never reworded — present only
       *  on a `refused` row, which is the one reason with nothing to retry. */
      sentence?: string;
    }
  | {
      id: string;
      state: "sent";
      at: number;
      /** What LANDED, so a game later undone or played on is not taken for one
       *  already archived. See `outbox.ts`'s `fingerprint`. */
      fingerprint: string;
    };

/** The database every local game lives in. Exported for the cells that arrange
 *  a version-1 holder by hand — the shipped M6.5b Worker, which this version's
 *  upgrade has to survive. */
export const DB_NAME = "yokohama-local";
/** **2 since M6.5b2**, for the `outbox` store and nothing else.
 *
 *  A version bump here is only ever a NEW STORE: the documents are the layer's
 *  own and their schema is `TABLE_SCHEMA`/`GameRecord`, so this module never
 *  migrates one. */
const VERSION = 2;
const DOCS = "docs";
const META = "meta";
const OUTBOX = "outbox";

/** Every STORE a game has a row in — what a delete opens (M6.5b2).
 *
 *  **A second list beside [`GAME_DOCS`], and it is not the same list.** That
 *  one names the kinds inside `docs`; this one names the stores, and the
 *  outbox is the first thing a game has that is not a document. Both deletes
 *  take this and neither names a store of its own. */
const GAME_STORES = [DOCS, META, OUTBOX];

/** Take every row this game has out, inside the caller's own transaction.
 *
 *  **ONE function and not two identical loops** (review I-1's lesson, applied
 *  before it could bite twice): `deletePair` and `forgetGame` differ only in
 *  their fence, and they have already disagreed once about what a game's rows
 *  ARE. A row left behind by one of them names a game that is gone — which for
 *  the outbox means a door listing a send it can never make. */
function removeGameRows(t: IDBTransaction, id: string): void {
  const docs = t.objectStore(DOCS);
  for (const doc of GAME_DOCS) docs.delete([doc, id]);
  t.objectStore(META).delete(id);
  t.objectStore(OUTBOX).delete(id);
}

/** Settle on an `IDBRequest`. */
const done = <T>(r: IDBRequest<T>): Promise<T> =>
  new Promise<T>((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error("the request failed"));
  });

/** What a player reads while another tab is holding the old version open
 *  (M6.5b2, spec §2.6).
 *
 *  **It names the other TAB, because that is the thing they can act on**, and
 *  because the remedy a player would otherwise reach for — clearing this site's
 *  data — throws away every game on the device to fix a wait that ends by
 *  itself. */
export const BLOCKED_SENTENCE =
  "this app is open in another tab running an older version \u2014 close that tab and this one will carry on";

/**
 * Open it, creating every store a first visit needs and adding the ones a
 * later version brought.
 *
 * **Two things this does that version 1 did not, and both are about the
 * upgrade to version 2** (spec §2.6):
 *
 * - **A `blocked` open REPORTS and keeps waiting.** A shipped M6.5b tab holds
 *   version 1 for its Worker's whole life and installs no `versionchange`
 *   handler — that code cannot be retrofitted — so a version-2 open while one
 *   is alive is blocked. Rejecting there made the Worker's cached startup
 *   promise a permanent failure that closing the other tab could not undo. So
 *   the caller is TOLD, in a sentence naming the tab, and the open settles on
 *   its own when the blocker goes.
 * - **Every connection closes itself on `versionchange`.** That is the handler
 *   the old code lacks, and it is what makes this the last upgrade that can be
 *   blocked this way.
 */
export function openDb(opts?: {
  /** Another tab is holding an older version open. Called with
   *  [`BLOCKED_SENTENCE`]; the open is still in flight. */
  onBlocked?: (sentence: string) => void;
}): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      // **Each store is created if it is ABSENT**, which is one statement for
      // a fresh database and for every earlier version: a version-1 database
      // arrives here with `docs` and `meta` already in it and leaves with
      // `outbox` beside them, its documents untouched.
      if (!db.objectStoreNames.contains(DOCS)) db.createObjectStore(DOCS, { keyPath: ["doc", "id"] });
      if (!db.objectStoreNames.contains(META)) db.createObjectStore(META, { keyPath: "id" });
      if (!db.objectStoreNames.contains(OUTBOX)) db.createObjectStore(OUTBOX, { keyPath: "id" });
    };
    req.onsuccess = () => {
      const db = req.result;
      // The handler M6.5b did not have. A connection that stays open through
      // another tab's upgrade is what a `blocked` event IS.
      db.onversionchange = () => db.close();
      resolve(db);
    };
    req.onerror = () => reject(req.error ?? new Error("IndexedDB would not open"));
    req.onblocked = () => opts?.onBlocked?.(BLOCKED_SENTENCE);
  });
}

/**
 * Run `start` at most once for as long as it KEEPS WORKING, and let a failed
 * start be tried again (M6.5b2, spec §2.6).
 *
 * **Here rather than in `worker.ts` because it is the reason this module's
 * open can fail at all**, and because the Worker's project compiles exactly
 * two files. Its one caller is the Worker's startup — the module, this
 * database and the rebuild — which was a promise built at module scope and
 * therefore cached whatever it settled as. A blocked open then made every
 * later message fail for a reason that had already gone away.
 *
 * A pending start is shared, so callers that arrive together run it once.
 */
export function retryingStart<T>(start: () => Promise<T>): () => Promise<T> {
  let attempt: Promise<T> | null = null;
  return () => {
    if (attempt) return attempt;
    const mine = start();
    attempt = mine;
    // **Cleared on REJECTION only**: a start that worked is the answer for the
    // life of this Worker, and re-running it would re-open a database and
    // rebuild a store that is already there.
    mine.catch(() => {
      if (attempt === mine) attempt = null;
    });
    return mine;
  };
}

/** Every stored document, for the rebuild that follows every `init()`. */
export const readAll = (db: IDBDatabase): Promise<DocRow[]> =>
  done(db.transaction(DOCS, "readonly").objectStore(DOCS).getAll() as IDBRequest<DocRow[]>);

/** What generation this game is on, or `0` for a game no tab has ever held.
 *
 *  **No WRITE asks it**: every write carries the generation it took and
 *  re-reads the stored one inside its own transaction, which is what makes that
 *  check a fence. **Its one production caller is a READ** — the Worker's `pair`
 *  arm (PR #37 fix wave, R-F1; `exportPair.ts`), which asks whether this tab
 *  still OWNS a game before it hands out its memory as the game's export. A
 *  read needs no fence, only the answer. `db.test.ts` also reads the fence's
 *  own state through it. */
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
 * Remove a game from this device: [`GAME_DOCS`] and its generation, in one
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
    const t = db.transaction([...GAME_STORES], "readwrite");
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
      removeGameRows(t, id);
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
 * **A NOTE FOR THE NEXT `SEMANTICS_REV` BUMP** (whole-branch review, Minor 1),
 * because this is the function it will be about. `already-here` answers on
 * identical bytes and writes NOTHING, so a game restored at rev N as
 * "unplayable" — its record being a rev N+1 one — keeps its compatibility row
 * after this device moves to rev N+1: the module opens the game happily, and
 * `OnThisDevice` still hides it from the playable list and lists it under "Will
 * not open here" with a reason that has stopped being true. Only "Remove" and a
 * fresh restore clear it. The fix, when that bump comes, is one of two: delete
 * the compat row inside this transaction on an `already-here` whose verdict is
 * not `unplayable`, or have the listing re-derive the verdict rather than trust
 * the row. It is deliberately NOT done now — today no rev N+1 record exists to
 * restore, so there is no way to reach the state and no way to test the fix.
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
 * Remove a game the module cannot open: [`GAME_DOCS`] and its generation.
 *
 * **Unfenced, and that is the difference from [`deletePair`]** — the fence is
 * about a tab that HOLDS the game, and a game the module never loaded is a game
 * no tab can hold. It is the only way to get rid of an unplayable row, which
 * spec §2.5 requires be possible without ever rewriting its bytes.
 */
export function forgetGame(db: IDBDatabase, id: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction([...GAME_STORES], "readwrite");
    removeGameRows(t, id);
    t.oncomplete = () => resolve();
    t.onabort = () => reject(t.error ?? new Error("the game could not be forgotten"));
    t.onerror = () => reject(t.error ?? new Error("the game could not be forgotten"));
  });
}

/** Every outbox row on this device, in the store's own order.
 *
 *  Both shapes: the caller sorts them out, because "what is waiting" and "what
 *  has landed" are one statement about a game and are read together. */
export const readOutbox = (db: IDBDatabase): Promise<OutboxRow[]> =>
  done(db.transaction(OUTBOX, "readonly").objectStore(OUTBOX).getAll() as IDBRequest<OutboxRow[]>);

/** One game's row, or `null`. */
export const readOutboxRow = async (db: IDBDatabase, id: string): Promise<OutboxRow | null> =>
  (await done(
    db.transaction(OUTBOX, "readonly").objectStore(OUTBOX).get(id) as IDBRequest<
      OutboxRow | undefined
    >,
  )) ?? null;

/** Write one game's row, replacing whatever was there.
 *
 *  **A replace and not a merge**, which is what keeps the two shapes from ever
 *  being half of each other: a landed send REPLACES the reason it was waiting
 *  for, and a fresh reason replaces a receipt whose bytes no longer match. */
export function putOutboxRow(db: IDBDatabase, row: OutboxRow): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(OUTBOX, "readwrite");
    t.objectStore(OUTBOX).put(row);
    t.oncomplete = () => resolve();
    t.onabort = () => reject(t.error ?? new Error("the outbox row was rolled back"));
    t.onerror = () => reject(t.error ?? new Error("the outbox row could not be written"));
  });
}

/** Forget what this game was waiting to do — what a deployment with no
 *  archive leaves behind, and what a send that landed on a game with nothing
 *  further to say does. */
export function deleteOutboxRow(db: IDBDatabase, id: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const t = db.transaction(OUTBOX, "readwrite");
    t.objectStore(OUTBOX).delete(id);
    t.oncomplete = () => resolve();
    t.onabort = () => reject(t.error ?? new Error("the outbox row was rolled back"));
    t.onerror = () => reject(t.error ?? new Error("the outbox row could not be removed"));
  });
}
