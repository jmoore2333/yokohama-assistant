// The outbox (M6.5b2, R-6.5b2.13; spec §2.6): a finished local game that could
// not be uploaded is not lost and not forgotten — it waits, says why, and
// retries itself.
//
// **What it is for, in one measured sentence** (spec §0.3): with no boot health
// read, a browser that has never been past the door does not know whether this
// deployment keeps an archive — so a game that finishes offline, or before the
// first health read, would simply never be offered to it. Nothing was dropped
// before this existed, because nothing was ever attempted.
//
// ## The rules this module is, in the order they matter
//
// 1. **FOUR reasons, and the third is the one a first draft omitted.** A fresh
//    visitor can install the shell, go offline and finish their FIRST game with
//    the archive fact never having been read: an entry condition of "the
//    archive is known to be on" queues that game nowhere and coming back online
//    drains an empty list. `"unknown"` is that row. `"refused"` is the fourth
//    and is an END state — a 400 or a non-`identical` 409 is neither offline
//    nor signed out, and posting the same bytes for ever would never change it.
// 2. **Reasons are re-evaluated, not frozen.** When the archive fact is
//    DISCOVERED, every unsent finished local game is reconsidered: a row
//    entered as "not yet known" resolves to a send, or to nothing at all where
//    the deployment keeps no archive.
// 3. **Everything is SERIALISED.** The `online` event, the launch drain and the
//    player's own button can land in one tick, and two passes reading the store
//    before the first has written its receipt would post the same bytes twice.
// 4. **A landed send is a RECEIPT against the game's bytes.** That is what
//    makes the game-over hook idempotent against a reload — the game is still
//    over on the next visit — and what stops a game later undone or played on
//    from being taken for one already archived.
// 5. **Nothing here ever authorises a delete.** Only a landed upload does,
//    which is `Abandon`'s existing rule and is why Abandon never joins this
//    queue (R-6.5b2.13).
// 6. **Deliberately NOT the Background Sync API**, which Safari does not
//    implement: a portable mechanism beats one that works on one platform.
// 7. **Every entry point ANSWERS, with ONE named exception.** Each is called
//    with `void` from a render or an event handler, so a browser with no usable
//    IndexedDB — a private window, a blocked site — has no outbox rather than
//    an unhandled rejection on a screen that is otherwise fine. The exception
//    is an open BLOCKED by another tab holding storage version 1 (`db.ts`):
//    that open waits rather than failing, by design, so an entry point behind
//    it does not settle until the other tab goes. It is acceptable because the
//    whole local channel is unusable in that state — every game read is waiting
//    on the same open — the page says so in one sentence naming the tab, and it
//    self-heals.
// 8. **Nothing here caps how many games one pass posts**, and the spec asks for
//    no cap. What is bounded is the HEALTH read (`ensureArchiveFact`), not the
//    loop: a device with several old finished games posts them all on the first
//    discovery, one after another, never in parallel. Said out loud because
//    "bounded" is easy to read as a claim about the loop.

import { useCallback, useEffect, useState } from "react";
import { api, ApiError, ensureArchiveFact } from "./api";
import { localPair } from "./local/client";
import {
  deleteOutboxRow,
  openDb,
  putOutboxRow,
  readOutbox,
  readOutboxRow,
  type OutboxRow,
  type PendingReason,
} from "./local/db";
import { archiveAnswer, onServerFacts, type ArchiveKind } from "./local/facts";

export type { PendingReason };

/** One game waiting to reach the archive — what the door draws a row from. */
export interface Pending {
  id: string;
  reason: PendingReason;
  lastTried: number;
  /** The SERVER's own sentence, on a `refused` row alone. */
  sentence?: string;
}

/**
 * What a KNOWN archive fact licenses — three answers, not two (R-6.5b2.13).
 *
 * **`"unknown"` is not `"off"`, and that is the finding** (spec §0.3, §2.6). A
 * deployment known to keep no archive deletes as it always has, which is
 * correct and is not a bug. A browser that has never been told anything has no
 * grounds for that claim, so the answer is neither: ask.
 */
export const archiveDisposition = (kind: ArchiveKind | "unknown"): "send" | "delete" | "ask" =>
  kind === "unknown" ? "ask" : kind === "off" ? "delete" : "send";

/** What a row says when the reason is not the server's own words (spec §4). */
export const reasonWords = (reason: PendingReason, sentence?: string): string => {
  switch (reason) {
    case "offline":
      // **The sentence where the failure had one of its OWN**: a local host
      // that trapped or would not start is not a connection problem, and "no
      // connection" about a module fault is a false sentence on a screen. A
      // `fetch` that never arrived carries none and reads as the connection it
      // is.
      return sentence ?? "no connection";
    case "signed-out":
      return "not signed in";
    case "unknown":
      return "not yet known";
    case "refused":
      return sentence ?? "the archive would not take it";
  }
};

/**
 * A fingerprint of the bytes that landed.
 *
 * **Length AND a hash, because the hash alone has the dangerous failure.** A
 * collision would read as "already sent" for a record that is not the one that
 * landed, and the length is the cheap half that a genuine undo-and-replay
 * almost never preserves. FNV-1a over the string: no dependency, no async, and
 * nothing here is a security claim — a receipt is this browser's own note to
 * itself.
 */
function fingerprint(record: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < record.length; i += 1) {
    h ^= record.charCodeAt(i);
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return `${record.length}:${h.toString(16)}`;
}

/** Why a send did not land, in this module's own vocabulary.
 *
 *  **Branching on the STATUS and never on the sentence** (`ApiError`'s own
 *  rule): 401 is the one thing `identity.rs` answers, and it is also what
 *  Cloudflare Access answers when the session in front of the server has
 *  expired — two different doors, one remedy, and one of three possible
 *  sentences. A 400 or a 409 is about THIS RECORD and will not change; a 403,
 *  a 5xx or a fetch that never arrived is about the moment, and is retried. */
function reasonFor(e: unknown): { reason: PendingReason; sentence?: string } {
  if (e instanceof ApiError) {
    if (e.status === 401) return { reason: "signed-out" };
    if (e.status === 400 || e.status === 409) return { reason: "refused", sentence: e.message };
  }
  return { reason: "offline" };
}

// ---- the queue -------------------------------------------------------------

let queue: Promise<unknown> = Promise.resolve();

/** Rule 3: one operation at a time, and a failure never poisons the queue. */
function serial<T>(f: () => Promise<T>): Promise<T> {
  const run = queue.then(f, f);
  queue = run.then(
    () => undefined,
    () => undefined,
  );
  return run;
}

/** Whoever is drawing the rows. */
const listeners = new Set<() => void>();
const changed = (): void => {
  for (const f of [...listeners]) f();
};

/** Whoever is drawing the game a send just landed for (review M-6). */
const landedListeners = new Set<(id: string, path: string) => void>();

/**
 * A copy of `id` is at `path` — told to the screen that has that game open.
 *
 * **So a control is not offered for work already done.** The game screen's
 * "Send to the archive" darkens once a copy has landed, and before this the
 * only send it knew about was the one its own button made — so after the hook
 * had quietly archived the game the button was still there, and clicking it
 * bought the server's `identical`. The path is the SERVER's, carried and not
 * composed here.
 */
export const onArchived = (f: (id: string, path: string) => void): (() => void) => {
  landedListeners.add(f);
  return () => {
    landedListeners.delete(f);
  };
};

const announceLanded = (id: string, path: string): void => {
  for (const f of [...landedListeners]) f(id, path);
};

/**
 * Run `f` against this browser's database, and CLOSE the connection after it
 * (review M-2).
 *
 * **One connection per public call, and it is given back.** Every operation
 * here used to open its own and keep it: `pending()` alone runs once per
 * change, so a drain of N rows left N+ connections open for the life of the
 * page. Nothing downstream may hold the handle past `f`.
 *
 * **A browser with no usable store answers `fallback`** (rule 7): a private
 * window, a blocked site, a browser with no IndexedDB at all has no outbox
 * rather than an unhandled rejection on a screen that is otherwise fine.
 */
async function withStore<T>(f: (db: IDBDatabase) => Promise<T>, fallback: T): Promise<T> {
  let db: IDBDatabase;
  try {
    db = await openDb();
  } catch {
    return fallback;
  }
  try {
    return await f(db);
  } catch {
    return fallback;
  } finally {
    db.close();
  }
}

const isPending = (r: OutboxRow): r is Extract<OutboxRow, { state: "pending" }> =>
  r.state === "pending";

/** Every game waiting to reach the archive, receipts excluded. */
export const pending = (): Promise<Pending[]> =>
  withStore(
    async (db) =>
      (await readOutbox(db)).filter(isPending).map((r) => ({
        id: r.id,
        reason: r.reason,
        lastTried: r.lastTried,
        ...(r.sentence === undefined ? {} : { sentence: r.sentence }),
      })),
    [],
  );

/** Write one game's reason for waiting, inside the caller's own connection.
 *
 *  **It never throws** — a store that refused is a browser with no outbox, and
 *  every caller is inside a loop that must go on to the next row. */
async function putPending(
  db: IDBDatabase,
  id: string,
  reason: PendingReason,
  sentence?: string,
): Promise<void> {
  try {
    await putOutboxRow(db, {
      id,
      state: "pending",
      reason,
      lastTried: Date.now(),
      ...(sentence === undefined ? {} : { sentence }),
    });
    changed();
  } catch {
    /* a browser that cannot remember has no outbox (rule 7) */
  }
}

/** Write the receipt, inside the caller's own connection. Never throws. */
async function putReceipt(db: IDBDatabase, id: string, record: string): Promise<void> {
  try {
    await putOutboxRow(db, {
      id,
      state: "sent",
      at: Date.now(),
      fingerprint: fingerprint(record),
    });
    changed();
  } catch {
    /* rule 7 */
  }
}

/** Forget what this game was waiting to do — what a deployment with no archive
 *  leaves behind. Never throws. */
async function forget(db: IDBDatabase, id: string): Promise<void> {
  try {
    await deleteOutboxRow(db, id);
    changed();
  } catch {
    /* rule 7 */
  }
}

/** Write one game's reason for waiting. The public face of [`putPending`]. */
export const enqueue = (id: string, reason: PendingReason, sentence?: string): Promise<void> =>
  withStore((db) => putPending(db, id, reason, sentence), undefined);

/**
 * This game's bytes reached the archive — write the receipt (rule 4).
 *
 * **Public, because the manual control is a landed send too.** "Send to the
 * archive" on a finished game's own screen posts the same pair through the same
 * route, and a copy that landed there is as landed as one this module made: the
 * row goes and the hook does not offer the same bytes again.
 */
export const recordLanded = (id: string, record: string): Promise<void> =>
  withStore((db) => putReceipt(db, id, record), undefined);

/**
 * Offer one game's snapshot to the archive, and write down what came of it.
 *
 * **The pair is the CALLER's, read once** (review I-2; spec §2.2's one-snapshot
 * rule). This used to read it again for itself, so the receipt was checked
 * against one read and the bytes posted and fingerprinted came from another —
 * two Worker round-trips, and two answers to "what is this game" where the
 * server is checking that the record and the table agree.
 *
 * **Safe to retry by construction**: the server answers `identical` for a
 * second send of the same bytes, and this client treats 200 and 201 alike.
 */
async function offer(
  db: IDBDatabase,
  id: string,
  pair: { record: string; table: string },
): Promise<void> {
  try {
    const reply = await api.archive(pair.record, pair.table);
    await putReceipt(db, id, pair.record);
    announceLanded(id, reply.path);
  } catch (e) {
    const { reason, sentence } = reasonFor(e);
    await putPending(db, id, reason, sentence);
  }
}

/** Has this game's CURRENT record already landed? The receipt is about the
 *  bytes, so a game undone and played on answers `false` (rule 4). */
async function alreadySent(db: IDBDatabase, id: string, record: string): Promise<boolean> {
  const row = await readOutboxRow(db, id);
  return row?.state === "sent" && row.fingerprint === fingerprint(record);
}

/** The sentence a LOCAL failure carried, where it had one.
 *
 *  **Not `reasonFor`'s job, and the difference is which side failed.** A
 *  `fetch` that never arrived is a connection and is worded as one; a Worker
 *  that trapped, restarted or would not start is not, and "no connection"
 *  about a module fault is a false sentence on a screen. */
const localSentence = (e: unknown): string | undefined =>
  e instanceof Error && e.message ? e.message : undefined;

/**
 * ONE game's whole attempt: the snapshot, the receipt check, the send — and the
 * row it leaves behind whatever happens (review I-1, I-2).
 *
 * **Every failure is this ROW's failure, and the caller goes on to the next.**
 * `localPair` rejects whenever the local host faults, traps or is torn down
 * mid-message, and before this that rejection escaped both drain loops into one
 * outer `catch {}`: the loop exited, nothing was written down, and rows 2..N
 * were never tried. Because every later trigger re-reads the store in the same
 * order, the same row failed first every time — one game whose record reliably
 * trapped the module would have blocked every other game's send for ever, with
 * nothing on the door saying so.
 *
 * **ONE `localPair` for the whole attempt**, which is spec §2.2's rule: the
 * receipt is checked against the very bytes that are posted and fingerprinted.
 */
async function attempt(
  db: IDBDatabase,
  id: string,
  kind: ArchiveKind | "unknown",
): Promise<void> {
  try {
    const pair = await localPair(id);
    // Nothing to send is nothing owed. A game with no record is a game with no
    // moves in it — or one this device has since deleted — and the archive has
    // no use for either.
    if (pair.record === null || pair.table === null) {
      await forget(db, id);
      return;
    }
    if (await alreadySent(db, id, pair.record)) return;
    if (kind === "off") {
      await forget(db, id);
      return;
    }
    if (kind === "unknown") {
      await putPending(db, id, "unknown");
      return;
    }
    await offer(db, id, { record: pair.record, table: pair.table });
  } catch (e) {
    // The row records the attempt — which is what `lastTried` means — and the
    // loop above goes on.
    await putPending(db, id, "offline", localSentence(e));
  }
}

/**
 * A local game has reached `over` — the real game-over hook (spec §2.6).
 *
 * **Idempotent against a reload**, which is what the receipt is for: the game
 * is still over on the next visit and this fires again. It is also the moment
 * the archive fact is first USED on the local channel, so it is one of the
 * three read points (spec §2.2) — at most one request per browser, at the end
 * of a game, for a player who has already decided to keep something.
 */
export async function noteGameOver(id: string): Promise<void> {
  await serial(async () => {
    // **The READ comes first, before the store is even opened** (spec §2.2).
    // This is one of the three read points in its own right: the send control
    // on the finished game's own screen is drawn from the answer, and a browser
    // that could not open its database would otherwise never learn what this
    // deployment keeps. A browser with no usable store has no outbox and no
    // local games either, so nothing below it runs.
    const kind = await ensureArchiveFact();
    await withStore((db) => attempt(db, id, kind), undefined);
  });
}

/**
 * Try everything that is waiting (spec §2.6) — on the browser's `online` event,
 * at the next launch, and on the player's own button.
 *
 * **A `refused` row is not in the list**, and the list being empty is answered
 * BEFORE the archive fact is asked for: there is nothing an answer could
 * change, and a health read on every `online` event for a queue that will never
 * move is a request a single-player visitor should not pay.
 */
export async function drain(): Promise<void> {
  await serial(async () => {
    const waiting = await withStore(
      async (db) => (await readOutbox(db)).filter(isPending).filter((r) => r.reason !== "refused"),
      [] as Extract<OutboxRow, { state: "pending" }>[],
    );
    if (waiting.length === 0) return;
    const kind = await ensureArchiveFact();
    // **Still nothing known: the rows stay AS THEY ARE** (review M-7). An
    // earlier draft rewrote each one as "not yet known", which threw away the
    // more specific reason a row had been given.
    if (kind === "unknown") return;
    await withStore(async (db) => {
      if (kind === "off") {
        for (const r of waiting) await forget(db, r.id);
        return;
      }
      for (const r of waiting) await attempt(db, r.id, kind);
    }, undefined);
  });
}

/** One game, because the player asked for it by name. */
export async function sendNow(id: string): Promise<void> {
  await serial(async () => {
    const kind = await ensureArchiveFact();
    await withStore((db) => attempt(db, id, kind), undefined);
  });
}

/**
 * The archive fact has been DISCOVERED: reconsider every unsent finished local
 * game (rule 2, spec §2.6).
 *
 * **Wider than the queue, and that is the point.** A row entered as "not yet
 * known" resolves here — but so does a game that finished while this browser
 * believed the deployment kept no archive, which wrote no row at all. The
 * listing is the module's own `GET /api/games?scope=mine`, read from this tab's
 * Worker; a game still being played has no ending to archive and is skipped.
 *
 * **It costs the listing read only where the archive is ON.** Where nothing is
 * known there is nothing to decide, and where it is known to be off the waiting
 * rows are dropped without asking the module anything.
 */
export async function reconsiderAll(): Promise<void> {
  await serial(async () => {
    const rows = await withStore(
      async (db) => (await readOutbox(db)).filter(isPending),
      [] as Extract<OutboxRow, { state: "pending" }>[],
    );
    const waiting = rows.filter((r) => r.reason !== "refused");
    const kind = await ensureArchiveFact();
    if (kind === "unknown") return;
    if (kind === "off") {
      await withStore(async (db) => {
        for (const r of waiting) await forget(db, r.id);
      }, undefined);
      return;
    }
    const ids = new Set(waiting.map((r) => r.id));
    // **A REFUSED game is not swept back in by the listing.** `waiting` already
    // excludes it, but the listing does not know about rows at all — so without
    // this a rediscovery would post the very bytes the server has already said
    // it will not take, once per discovery, for ever.
    const refused = new Set(rows.filter((r) => r.reason === "refused").map((r) => r.id));
    try {
      const list = await api.list("mine", undefined, "local");
      for (const t of list.games) if (t.over === true && !refused.has(t.id)) ids.add(t.id);
    } catch {
      // The module could not list. The rows this browser already wrote down are
      // still owed, and are still tried below.
    }
    await withStore(async (db) => {
      for (const id of ids) await attempt(db, id, kind);
    }, undefined);
  });
}

// ---- the triggers ----------------------------------------------------------

/** The archive answer this module has already acted on, so one health read does
 *  not start a sweep per listener call. */
let actedOn: ArchiveKind | null = null;
/** Every live registration, so a cell can put the module back (see
 *  [`resetOutboxForTests`]). */
const running = new Set<() => void>();

/**
 * Wire the three triggers, and hand back the way to unwire them.
 *
 * **Mounted by `App` and not at module load**, for the reason every other
 * module-level effect in this client is: a module that added a window listener
 * when it was imported would add one per test file and per render tree, and
 * could never be taken off again.
 */
export function startOutbox(): () => void {
  const online = () => void drain();
  const facts = () => {
    const now = archiveAnswer();
    // Discovery, and not every health read: the sweep is worth a listing read
    // once per answer, not once per landing.
    if (now === "unknown" || now === actedOn) return;
    actedOn = now;
    void reconsiderAll();
  };
  window.addEventListener("online", online);
  const offFacts = onServerFacts(facts);
  // At launch. A queue that is empty answers without asking the server
  // anything, so a visitor with nothing waiting pays nothing.
  void drain();
  const stop = () => {
    window.removeEventListener("online", online);
    offFacts();
    running.delete(stop);
  };
  running.add(stop);
  return stop;
}

/** The rows, kept in step with the store. */
export function useOutbox(): Pending[] {
  const [rows, setRows] = useState<Pending[]>([]);
  const read = useCallback(() => void pending().then(setRows), []);
  useEffect(() => {
    read();
    listeners.add(read);
    return () => {
      listeners.delete(read);
    };
  }, [read]);
  return rows;
}

/** Put the module back where a fresh page load finds it — for the CELLS, the
 *  way `resetLocalHostForTests` is. The queue, the listeners and every live
 *  registration go; what is in the DATABASE is the store's and stays. */
export function resetOutboxForTests(): void {
  queue = Promise.resolve();
  listeners.clear();
  actedOn = null;
  for (const stop of [...running]) stop();
}
