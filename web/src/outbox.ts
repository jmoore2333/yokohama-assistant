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
// 1. **FOUR reasons, and the third is the one a first draft omitted** (a fifth,
//    `"challenged"`, is cycle F's — rule 6a). A fresh
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
// 6a. **The edge's challenge waits for a PERSON** (cycle F, spec §2.6). Under
//    the no-Access door a request with no clearance meets a Managed Challenge
//    that only the widget can pass. The passes nobody is watching — the
//    `online` event, a launch in a hidden tab, the sweep — send with
//    `interactive: false`, and a challenge met by their health read OR a send
//    marks EVERY row still waiting `"challenged"` and stops; the next `online`
//    does not retry those rows. A VISIBLE tab's launch and the row's own button
//    are `drainVisible`, which shows the widget through the seam and sends
//    again. And a visitor who has chosen no name under the open door is never
//    posted for: the layer's refusal is certain (F-14).
// 6b. **A keyed build is the open door by R-FP4 (one build per door state)**
//    (Task 6 fix round 1): the deploy guard's check 12 holds the declaration
//    to the stamp and, from Task 7, to the open identity. So under a keyed build a
//    visitor with no claimed name is refused an upload for CERTAIN without
//    anything being asked — and nothing is: no health read, no widget, no post,
//    at game over (a fresh context's included), at a launch or in any drain.
//    Their games are written down `"signed-out"`, and a `"signed-out"` row is
//    retried only once a name is stored. A KEYLESS build (the Access builds,
//    the NAS) cannot know its door without the read, and makes it as E did.
// 6c. **The archive fact's wait is the caller's, and the read has its own
//    bound** (Task 6 fix rounds 1 and 2): a check that takes longer than the
//    read's 5 s wait still lands its answer, which is written down, and the
//    pass that waited sends its rows again then — with the door's own apply
//    held until it has. The read past the wait is bounded too (M6.5c I-86: a
//    read that never settles is a state, not a delay): any pass in progress is
//    waited out (the overlay's own 90 s ends it), then the transport's
//    `BOUND_MS`; past that the hold goes and the rows read "waiting for a human
//    check", as a late refusal leaves them.
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
//    "bounded" is easy to read as a claim about the loop. One thing does stop
//    a pass early: the first 404 (L-6), after which every remaining row is
//    dropped unposted.

import { useCallback, useEffect, useState } from "react";
import { api, ApiError, BOUND_MS, claimedUser, ensureArchiveFact, isEdgeRefusal } from "./api";
import { openDbBounded } from "./backup";
import { localPair } from "./local/client";
import {
  deleteOutboxRow,
  putOutboxRow,
  readOutbox,
  readOutboxRow,
  type OutboxRow,
  type PendingReason,
} from "./local/db";
import {
  archiveAnswer,
  onServerFacts,
  rememberArchiveOff,
  serverFacts,
  type ArchiveKind,
} from "./local/facts";
import { setApplyBusy } from "./swRegister";
import { turnstileSiteKey } from "./buildValues";
import { passInProgress } from "./turnstile";

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
    case "challenged":
      return "waiting for a human check";
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
 *
 * **And so does a store another tab is HOLDING** (M6.5c2 cycle D, straw poll
 * row 12): the open goes through `backup.ts`'s `openDbBounded`, because a plain
 * open waits for as long as an M6.5b-era tab holds the old version, and the
 * fallback never arrived.
 */
async function withStore<T>(f: (db: IDBDatabase) => Promise<T>, fallback: T): Promise<T> {
  let db: IDBDatabase;
  try {
    db = await openDbBounded();
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
 *
 * **This module's one call of `api.archive`**, so every send the outbox makes
 * — the game-over hook, the drain, the sweep and a row's own "Send now" —
 * meets the 404 branch below through `attempt`. The game screen's "Send to the
 * archive" and Abandon's save post for themselves, and put the server's
 * sentence in front of the player who clicked.
 */
async function offer(
  db: IDBDatabase,
  id: string,
  pair: { record: string; table: string },
  interactive: boolean,
): Promise<Outcome> {
  try {
    const reply = await api.archive(pair.record, pair.table, { interactive });
    await putReceipt(db, id, pair.record);
    announceLanded(id, reply.path);
  } catch (e) {
    // **The EDGE asked for a person, and none passed** (cycle F, rule 6a): the
    // row waits for one, and the pass stops — every row behind this one would
    // meet the same challenge.
    if (isEdgeRefusal(e)) {
      await putPending(db, id, "challenged");
      return "challenged";
    }
    // **A 404 is the deployment's answer, not the moment's** (M6.5c,
    // R-6.5c.14; spec §2.6): the layer's "this server keeps no archive". Filed
    // as "offline", the row was posted again on every reconnect until the
    // remembered health went stale. The archive is OFF: that is written down —
    // its age untouched — and the row goes, exactly as a known "off" drops
    // one. By the STATUS, never the sentence, `reasonFor`'s own rule.
    if (e instanceof ApiError && e.status === 404) {
      rememberArchiveOff();
      await forget(db, id);
      return undefined;
    }
    const { reason, sentence } = reasonFor(e);
    await putPending(db, id, reason, sentence);
  }
  return undefined;
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
  interactive = false,
): Promise<Outcome> {
  try {
    const pair = await localPair(id);
    // Nothing to send is nothing owed. A game with no record is a game with no
    // moves in it — or one this device has since deleted — and the archive has
    // no use for either.
    if (pair.record === null || pair.table === null) {
      await forget(db, id);
      return undefined;
    }
    if (await alreadySent(db, id, pair.record)) return undefined;
    // **L-6: a 404 earlier in THIS pass answers for every row after it**
    // (M6.5c2). `kind` is read once per operation, and it is the REMEMBERED
    // fact that moves when the route says "this server keeps no archive"
    // (`offer`'s 404 arm) — so a row behind that one is dropped the way a
    // known "off" drops one, and never posted to be told the same thing.
    if (kind === "off" || archiveAnswer() === "off") {
      await forget(db, id);
      return undefined;
    }
    // **Rule 6b: a keyed build is the open door by R-FP4**, so a nameless
    // visitor's upload is refused for certain — whatever this browser has or
    // has not been told — and the game waits for a name.
    if (namelessUnderKey()) {
      await putPending(db, id, "signed-out");
      return undefined;
    }
    if (kind === "unknown") {
      await putPending(db, id, "unknown");
      return undefined;
    }
    // **A visitor with no name under the open door is not posted for** (cycle
    // F, F-14; spec §2.6): the layer answers that post 401 for certain, so it
    // would buy an invocation to be told what is already known. The row says
    // "not signed in", whose remedy names the name step. Only a door KNOWN to
    // be open: under any other, or one never heard of, the name is not what
    // the server reads.
    if (serverFacts().identity === "open" && claimedUser() === null) {
      await putPending(db, id, "signed-out");
      return undefined;
    }
    return await offer(db, id, { record: pair.record, table: pair.table }, interactive);
  } catch (e) {
    // The row records the attempt — which is what `lastTried` means — and the
    // loop above goes on.
    await putPending(db, id, "offline", localSentence(e));
    return undefined;
  }
}

/** What one attempt tells the pass it is in: `"challenged"` when the edge
 *  asked for a person and none passed, which stops the pass (rule 6a). */
type Outcome = "challenged" | undefined;

/** Rule 6b: whether this visitor's upload is refused for CERTAIN with nothing
 *  asked — a keyed build is the open door by R-FP4, and under the open door a
 *  visitor with no claimed name is not one the layer will take a post from. */
const namelessUnderKey = (): boolean => Boolean(turnstileSiteKey()) && claimedUser() === null;

/** Rule 6b's pass: write each game down for what it is waiting on, asking the
 *  server NOTHING. The remembered archive fact still drops a game a deployment
 *  known to keep none has no use for (`attempt`'s order). */
async function holdForAName(ids: readonly string[]): Promise<void> {
  const kind = archiveAnswer();
  await withStore(async (db) => {
    for (const id of ids) await attempt(db, id, kind);
  }, undefined);
}

/** Mark every row in `ids` "waiting for a human check" — what a challenge met
 *  by a pass's health read, or by one of its sends, says of the rest (F-15). */
async function markChallenged(ids: readonly string[]): Promise<void> {
  await withStore(async (db) => {
    for (const id of ids) await putPending(db, id, "challenged");
  }, undefined);
}

/** Try `ids` in order and stop at the first the edge challenged, marking it and
 *  every row after it (rule 6a). The caller holds the queue. */
async function attemptAll(ids: readonly string[], kind: ArchiveKind, interactive: boolean): Promise<void> {
  await withStore(async (db) => {
    for (let i = 0; i < ids.length; i += 1) {
      if ((await attempt(db, ids[i]!, kind, interactive)) === "challenged") {
        for (const rest of ids.slice(i + 1)) await putPending(db, rest, "challenged");
        return;
      }
    }
  }, undefined);
}

/** The archive fact for a pass — or `"challenged"` when the edge challenged the
 *  health read and no check passed for it (rule 6a; F-15: a fresh context's
 *  FIRST request is this one).
 *
 *  **And a read that outlives its wait is followed** (rule 6c): `ids` are the
 *  rows this pass was for, and [`followLate`] sends them again when the late
 *  answer lands. */
async function archiveFactFor(
  interactive: boolean,
  ids: readonly string[],
): Promise<ArchiveKind | "unknown" | "challenged"> {
  try {
    return await ensureArchiveFact({ interactive, onLate: (landed) => followLate(landed, interactive, ids) });
  } catch (e) {
    if (isEdgeRefusal(e)) return "challenged";
    throw e;
  }
}

/**
 * Rule 6c: the archive fact's read went on past its wait — a person still
 * passing the widget, or a slow server.
 *
 * **The door's own apply is held until it settles** (R-FP14): a waiting release
 * must not reload the page under the overlay. When the answer lands it has
 * been written down, and the waiting rows go through the ordinary pass — a
 * VISIBLE one where this was, so a challenged row is among them — which now
 * finds the fact fresh and the clearance set: no second read, no second
 * widget. A read the edge refused after all (Cancel, a second challenge) marks
 * the rows it was for "waiting for a human check", as an answer inside the
 * wait would have.
 *
 * **And the read is BOUNDED** (Task 6 fix round 2; M6.5c I-86): a read that
 * never answers would otherwise hold the door's apply for as long as it is
 * out. [`lateBound`] waits out any pass in progress, then `BOUND_MS`; when it
 * elapses first, the hold goes and the rows are marked as a late refusal marks
 * them.
 */
function followLate(landed: Promise<unknown>, interactive: boolean, ids: readonly string[]): void {
  setApplyBusy(true);
  const bound = lateBound();
  const settled = landed.then(
    () => "landed" as const,
    (e: unknown) => (isEdgeRefusal(e) ? ("refused" as const) : ("failed" as const)),
  );
  void Promise.race([settled, bound.elapsed.then(() => "out of time" as const)])
    .then((how) => {
      bound.stop();
      if (how === "landed") return interactive ? drainVisible() : drain({ interactive: false });
      if (how === "refused" || how === "out of time") return serial(() => markChallenged(ids));
      return undefined;
    })
    .catch(() => undefined)
    .finally(() => setApplyBusy(false));
}

/**
 * How long a late read is given (Task 6 fix round 2): every pass in progress
 * waited out first — the overlay's widget ends by its own 90 s, so a person
 * mid-check is never cut off — and then `BOUND_MS`, the transport's own bound,
 * with no pass running. A pass that starts during that wait (a read answered
 * with the challenge late) is waited out in turn. `stop` clears what is armed
 * once the read has settled first.
 */
function lateBound(): { elapsed: Promise<void>; stop: () => void } {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let wake: (() => void) | undefined;
  const elapsed = (async () => {
    while (!stopped) {
      const pass = passInProgress();
      if (pass) {
        await pass.then(
          () => undefined,
          () => undefined,
        );
        continue;
      }
      await new Promise<void>((resolve) => {
        wake = resolve;
        timer = setTimeout(resolve, BOUND_MS);
      });
      if (!stopped && !passInProgress()) return;
    }
  })();
  return {
    elapsed,
    stop: () => {
      stopped = true;
      clearTimeout(timer);
      wake?.();
    },
  };
}

/** Whether this pass may show the widget: a person is looking at the tab. */
const visible = (): boolean => typeof document !== "undefined" && document.visibilityState === "visible";

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
    //
    // **A person is at the game when it ends**, so the edge's challenge shows
    // the widget here (cycle F) — unless the tab is hidden, as a reload of a
    // finished game in a background tab is: then the row waits for a visible
    // one, like every background pass's.
    //
    // **Rule 6b first**: under a keyed build a nameless visitor's game is
    // written down for its name, and the server is asked nothing — a fresh
    // context's first game over included.
    if (namelessUnderKey()) {
      await holdForAName([id]);
      return;
    }
    const interactive = visible();
    const kind = await archiveFactFor(interactive, [id]);
    if (kind === "challenged") {
      await markChallenged([id]);
      return;
    }
    await withStore((db) => attempt(db, id, kind, interactive), undefined);
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
 *
 * **Nor is a `challenged` row, unless the pass is `interactive`** (cycle F,
 * rule 6a): a background pass cannot show the widget the row is waiting for,
 * so asking again is a request the edge refuses again. A background pass that
 * meets the challenge — at its health read or at a send — marks every row
 * still waiting and stops. The default is the background pass; a visible tab's
 * is [`drainVisible`].
 */
export async function drain(opts: { interactive?: boolean } = {}): Promise<void> {
  const interactive = opts.interactive ?? false;
  await serial(async () => {
    // **Rule 6b**: a `signed-out` row under a keyed build with no name stored
    // is waiting on a NAME, which no read and no widget can supply — it is
    // left out, and retried once a name is stored.
    const nameless = namelessUnderKey();
    const waiting = await withStore(
      async (db) =>
        (await readOutbox(db))
          .filter(isPending)
          .filter(
            (r) =>
              r.reason !== "refused" &&
              (interactive || r.reason !== "challenged") &&
              !(nameless && r.reason === "signed-out"),
          ),
      [] as Extract<OutboxRow, { state: "pending" }>[],
    );
    if (waiting.length === 0) return;
    // Any other row of a nameless visitor's under a keyed build is written down
    // for its name, with nothing asked (rule 6b).
    if (nameless) {
      await holdForAName(waiting.map((r) => r.id));
      return;
    }
    // **The door's own apply waits while rows are being sent** (cycle F,
    // R-FP14): a reload in the middle of a send would lose the row's answer.
    // Held only where there is something to send — an empty queue, the
    // ordinary state, never touches it.
    setApplyBusy(true);
    try {
      const kind = await archiveFactFor(
        interactive,
        waiting.map((r) => r.id),
      );
      if (kind === "challenged") {
        await markChallenged(waiting.map((r) => r.id));
        return;
      }
      // **Still nothing known: the rows stay AS THEY ARE** (review M-7). An
      // earlier draft rewrote each one as "not yet known", which threw away the
      // more specific reason a row had been given.
      if (kind === "unknown") return;
      if (kind === "off") {
        await withStore(async (db) => {
          for (const r of waiting) await forget(db, r.id);
        }, undefined);
        return;
      }
      await attemptAll(
        waiting.map((r) => r.id),
        kind,
        interactive,
      );
    } finally {
      setApplyBusy(false);
    }
  });
}

/**
 * Try everything that is waiting — the `challenged` rows included — in a tab a
 * person is looking at (cycle F, spec §2.6): the door's launch when the
 * document is visible, and a `challenged` row's Check now.
 *
 * **The widget is shown here and nowhere in the background**: a challenge this
 * pass meets goes through the seam (`passChallenge`, the overlay), once, and
 * the request is sent again. In a hidden tab this is the background pass, so a
 * challenged row is never retried — and never shows a widget — where nobody
 * can see it.
 */
export async function drainVisible(): Promise<void> {
  await drain({ interactive: visible() });
}

/** One game, because the player asked for it by name. */
export async function sendNow(id: string): Promise<void> {
  await serial(async () => {
    // Rule 6b: a nameless visitor under a keyed build is asked nothing.
    if (namelessUnderKey()) {
      await holdForAName([id]);
      return;
    }
    // A click: a person is here, and a challenge shows the widget (cycle F).
    const kind = await archiveFactFor(true, [id]);
    if (kind === "challenged") {
      await markChallenged([id]);
      return;
    }
    await withStore((db) => attempt(db, id, kind, true), undefined);
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
    // Rule 6b: a nameless visitor under a keyed build is asked nothing, and
    // their `signed-out` rows stay as they are.
    if (namelessUnderKey()) {
      await holdForAName(waiting.filter((r) => r.reason !== "signed-out").map((r) => r.id));
      return;
    }
    // **A background pass** (cycle F, rule 6a): a discovery is neither a
    // launch nor a click, so it shows no widget, and a challenge marks the
    // rows it would have sent.
    const kind = await archiveFactFor(
      false,
      waiting.map((r) => r.id),
    );
    if (kind === "challenged") {
      await markChallenged(waiting.map((r) => r.id));
      return;
    }
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
    await attemptAll([...ids], kind, false);
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
  // The browser's `online` event is a background pass: it shows no widget
  // and retries no `challenged` row (rule 6a).
  const online = () => void drain({ interactive: false });
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
  // anything, so a visitor with nothing waiting pays nothing. A VISIBLE
  // launch is where a `challenged` row is offered the widget (rule 6a); a
  // launch in a hidden tab is a background pass.
  void drainVisible();
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
