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
// 7. **Every entry point ANSWERS.** Each is called with `void` from a render or
//    an event handler, so a browser with no usable IndexedDB — a private
//    window, a blocked site — has no outbox rather than an unhandled rejection
//    on a screen that is otherwise fine.

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
      return "no connection";
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

/** The database, or `null` where this browser has no usable one (rule 7). */
async function store(): Promise<IDBDatabase | null> {
  try {
    return await openDb();
  } catch {
    return null;
  }
}

const isPending = (r: OutboxRow): r is Extract<OutboxRow, { state: "pending" }> =>
  r.state === "pending";

/** Every game waiting to reach the archive, receipts excluded. */
export async function pending(): Promise<Pending[]> {
  const db = await store();
  if (!db) return [];
  try {
    return (await readOutbox(db)).filter(isPending).map((r) => ({
      id: r.id,
      reason: r.reason,
      lastTried: r.lastTried,
      ...(r.sentence === undefined ? {} : { sentence: r.sentence }),
    }));
  } catch {
    return [];
  }
}

/** Write one game's reason for waiting. */
export async function enqueue(
  id: string,
  reason: PendingReason,
  sentence?: string,
): Promise<void> {
  const db = await store();
  if (!db) return;
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

/**
 * This game's bytes reached the archive — write the receipt (rule 4).
 *
 * **Public, because the manual control is a landed send too.** "Send to the
 * archive" on a finished game's own screen posts the same pair through the same
 * route, and a copy that landed there is as landed as one this module made: the
 * row goes and the hook does not offer the same bytes again.
 */
export async function recordLanded(id: string, record: string): Promise<void> {
  const db = await store();
  if (!db) return;
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
 *  leaves behind. */
async function forget(db: IDBDatabase, id: string): Promise<void> {
  try {
    await deleteOutboxRow(db, id);
    changed();
  } catch {
    /* rule 7 */
  }
}

/**
 * Offer one game's pair to the archive, and say what came of it.
 *
 * **One snapshot of both documents** (spec §2.2): the server checks that the
 * record and the table agree, and two queued reads can fall either side of a
 * move. Neither is parsed on the way — the record's `seed` is a `u64` that
 * `JSON.parse` would round.
 *
 * **Safe to retry by construction**: the server answers `identical` for a
 * second send of the same bytes, and this client treats 200 and 201 alike.
 */
async function offer(db: IDBDatabase, id: string): Promise<void> {
  const pair = await localPair(id);
  // Nothing to send is nothing owed. A game with no record is a game with no
  // moves in it — or one this device has since deleted — and the archive has
  // no use for either.
  if (pair.record === null || pair.table === null) {
    await forget(db, id);
    return;
  }
  try {
    await api.archive(pair.record, pair.table);
    await recordLanded(id, pair.record);
  } catch (e) {
    const { reason, sentence } = reasonFor(e);
    await enqueue(id, reason, sentence);
  }
}

/** Has this game's CURRENT record already landed? The receipt is about the
 *  bytes, so a game undone and played on answers `false` (rule 4). */
async function alreadySent(db: IDBDatabase, id: string, record: string): Promise<boolean> {
  const row = await readOutboxRow(db, id);
  return row?.state === "sent" && row.fingerprint === fingerprint(record);
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
    try {
      // **The READ comes first, before the store is even opened** (spec §2.2).
      // This is one of the three read points in its own right: the send
      // control on the finished game's own screen is drawn from the answer,
      // and a browser that could not open its database would otherwise never
      // learn what this deployment keeps. A browser with no usable store has
      // no outbox and no local games either, so nothing below it runs.
      const kind = await ensureArchiveFact();
      const db = await store();
      if (!db) return;
      const pair = await localPair(id);
      if (pair.record === null || pair.table === null) return;
      if (await alreadySent(db, id, pair.record)) return;
      if (kind === "off") {
        // Nothing is owed here, and anything this game was waiting for is owed
        // no longer.
        await forget(db, id);
        return;
      }
      if (kind === "unknown") {
        await enqueue(id, "unknown");
        return;
      }
      await offer(db, id);
    } catch {
      /* rule 7 */
    }
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
    const db = await store();
    if (!db) return;
    try {
      const waiting = (await readOutbox(db)).filter(isPending).filter((r) => r.reason !== "refused");
      if (waiting.length === 0) return;
      const kind = await ensureArchiveFact();
      if (kind === "unknown") {
        // Still nothing known. The rows stay, and say so.
        for (const r of waiting) if (r.reason !== "unknown") await enqueue(r.id, "unknown");
        return;
      }
      if (kind === "off") {
        for (const r of waiting) await forget(db, r.id);
        return;
      }
      for (const r of waiting) await offer(db, r.id);
    } catch {
      /* rule 7 */
    }
  });
}

/** One game, because the player asked for it by name. */
export async function sendNow(id: string): Promise<void> {
  await serial(async () => {
    const db = await store();
    if (!db) return;
    try {
      const kind = await ensureArchiveFact();
      if (kind === "unknown") {
        await enqueue(id, "unknown");
        return;
      }
      if (kind === "off") {
        await forget(db, id);
        return;
      }
      await offer(db, id);
    } catch {
      /* rule 7 */
    }
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
    const db = await store();
    if (!db) return;
    try {
      const waiting = (await readOutbox(db)).filter(isPending).filter((r) => r.reason !== "refused");
      const kind = await ensureArchiveFact();
      if (kind === "unknown") return;
      if (kind === "off") {
        for (const r of waiting) await forget(db, r.id);
        return;
      }
      const ids = new Set(waiting.map((r) => r.id));
      // **A REFUSED game is not swept back in by the listing.** `waiting`
      // already excludes it, but the listing does not know about rows at all —
      // so without this a rediscovery would post the very bytes the server has
      // already said it will not take, once per discovery, for ever.
      const refused = new Set(
        (await readOutbox(db))
          .filter(isPending)
          .filter((r) => r.reason === "refused")
          .map((r) => r.id),
      );
      try {
        const list = await api.list("mine", undefined, "local");
        for (const t of list.games) if (t.over === true && !refused.has(t.id)) ids.add(t.id);
      } catch {
        // The module could not list. The rows this browser already wrote down
        // are still owed, and are still tried below.
      }
      for (const id of ids) {
        const pair = await localPair(id);
        if (pair.record === null || pair.table === null) {
          await forget(db, id);
          continue;
        }
        if (await alreadySent(db, id, pair.record)) continue;
        await offer(db, id);
      }
    } catch {
      /* rule 7 */
    }
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
