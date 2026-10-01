// The PAGE's side of the local host (spec §2.2, §2.3; R-6.5b.4, R-6.5b.5).
//
// One dedicated Worker per tab, started lazily on the first local call and kept
// for the page's life. Everything crossing the boundary is a `{ seq }` request
// answered by a `{ seq }` reply, plus one unsolicited message — the lock this
// tab held was taken.
//
// **The page must HOLD a local game before it reads it** (P-5, spec §2.2).
// Ownership is derived from the ROUTE's table id and never from a
// `ClientState` that has already been fetched: a cold read PUMPS the bot
// (`games.rs:405-424`), so a GET made before the lock is taken can change a
// record this tab does not own. `useLocalHold` (Task 6) is what mounts
// [`hold`]; this module only provides it.
//
// **Nothing here knows a rule, a route or a document's shape.** It moves
// strings. The module in the Worker is the layer; this is a wire.

import { requestPersistenceOnce } from "../storage";
import { unreadableOf, type UnreadableReply } from "./exportPair";
import { gameIdOf } from "./ids";
import { TAKEN_OVER_SENTENCE } from "./lost";

/** What the page asks the Worker for. The `call` shape's fields are
 *  `ApiRequest`'s own — the Worker adds `local@browser` and nothing else. */
export type ReqBody =
  | { kind: "call"; method: string; path: string; query: string; body: string | null }
  | { kind: "pair"; id: string }
  | { kind: "hold"; id: string }
  | { kind: "takeover"; id: string }
  | { kind: "release"; id: string }
  /** The module's validator over a backup file (M6.5b2, R-6.5b2.10). */
  | { kind: "restore_read"; envelope: string }
  /** Bring restored games into this Worker's live store (spec §2.5). */
  | { kind: "adopt"; ids: string[] };

/** One numbered request. The number is added by [`ask`]; `Omit<Req, "seq">`
 *  would not do the job in its place, because `Omit` over a union collapses it
 *  into one object type with none of the members. */
export type Req = { seq: number } & ReqBody;

/** Whether the Worker committed what the module just did (spec §2.2).
 *
 *  `"not-held"` is not a failure: a call on a game this tab does not hold is
 *  answered from the module's store and never written back, which is what
 *  keeps a second tab's cold read out of the owner's documents. */
export type Saved = "saved" | "unsaved" | "not-held" | "lost";

/** One answer. Which fields are present depends on what was asked; the page
 *  reads only the ones its own request implies. */
export type Reply = { seq: number } & Record<string, unknown>;

/** What a pending call is told when the module TRAPPED and the page had to
 *  start a fresh Worker (spec §4, R5).
 *
 *  **The words matter and are asserted**: a stored game that no longer
 *  rebuilds is a 500 carrying the ENGINE's sentence and is NOT a trap, so the
 *  one surface that says "reloaded from the last saved move" is this one. A
 *  client that said it on both would tell a player their save had been
 *  recovered when what happened was that it could not be read. */
export const RESTART_SENTENCE = "the local host restarted — reloaded from the last saved move";

/** What the Worker's answer to a game's acquisition or a restore's adoption
 *  said when this browser's storage could not be read (M6.5c2 cycle D,
 *  dossier row 14) — its sentence, carried whole.
 *
 *  **A type of its own, so a screen can tell it from every other rejection.**
 *  A Worker that will not start, a pending call settled by a restart and a
 *  store that cannot be read all arrive here as rejections; only the last is
 *  a thing the player can be shown a face for, with Reload, and the first two
 *  already have their own lines. */
export class LocalStoreUnreadable extends Error {
  constructor(sentence: string) {
    super(sentence);
    this.name = "LocalStoreUnreadable";
  }
}

/** A Take over the Worker answered `held: false` — a steal of the same game
 *  from another tab landed first, and aborted this one before it was granted.
 *  The player is told, rather than shown a screen that believes it plays the
 *  game here (M6.5c2 cycle D, dossier row 56). */
export const TAKEOVER_REFUSED_SENTENCE = "another tab took this game first \u2014 take it over again to play it here";

let worker: Worker | null = null;
let seq = 0;
const pending = new Map<number, { resolve: (r: Reply) => void; reject: (e: Error) => void }>();
const lostHandlers = new Set<(id: string) => void>();
const storeHandlers = new Set<(id: string, sentence: string) => void>();
const writeHandlers = new Set<() => void>();
const faultHandlers = new Set<(sentence: string) => void>();
/** The games the PAGE believes it holds — what to re-acquire after a restart.
 *  Kept here and not in the Worker for the reason a restart exists: the Worker
 *  that knew is the one that is gone. */
const heldByPage = new Set<string>();
/** Restarts with nothing answered in between.
 *
 *  **One is recovery; a second in a row is a host that cannot boot**, and
 *  restarting again is a loop — a module that fails to load traps on every
 *  message including the `hold` the restart itself sends. (An IndexedDB that
 *  will not open is NOT this since M6.5c2 cycle D: the Worker answers it in
 *  the storage's own words, an ordinary reply, and nothing restarts.) The
 *  budget is replenished by any ordinary reply, which is the only evidence
 *  that the host works. */
let restartsWithoutAnswer = 0;

/** Settle every waiting call with one sentence, and tell whoever is listening.
 *
 *  **Never leaves a pending list**, which is the failure §2.3 names: a Worker
 *  that fails to start, a module that will not load and an IndexedDB that will
 *  not open all arrive here, and each of them would otherwise be a screen
 *  waiting for a reply nobody is going to send. */
function fail(sentence: string): void {
  const waiting = [...pending.values()];
  pending.clear();
  for (const p of waiting) p.reject(new Error(sentence));
  for (const f of [...faultHandlers]) f(sentence);
}

/**
 * The Worker is DEAD: settle everything waiting on it, and throw the instance
 * away so the next call starts a fresh one.
 *
 * **Review finding #3.** Settling the pending list alone left the dead
 * instance installed, and `start()` handed it to the next `ask` — whose reply
 * never came. That is the permanently pending call §2.3 forbids, one step
 * further along than the one the sentence was written for. The next `ask`
 * now builds a new Worker; if THAT one cannot start either, it arrives here
 * again with its own sentence rather than silently waiting.
 */
function drop(sentence: string): void {
  worker?.terminate();
  worker = null;
  fail(sentence);
}

/** The `{"error": …}` sentence inside a reply body, or the body itself where
 *  it is not this API's shape. */
function sentenceOf(body: unknown): string {
  if (typeof body !== "string") return String(body);
  try {
    const parsed: unknown = JSON.parse(body);
    if (parsed && typeof parsed === "object" && "error" in parsed) {
      return String((parsed as { error: unknown }).error);
    }
  } catch {
    /* not JSON: the body is the sentence */
  }
  return body;
}

function start(): Worker {
  if (worker) return worker;
  // `new URL(…, import.meta.url)` is what makes Vite bundle the Worker as its
  // own module graph — and with it the wasm glue, which is what emits the
  // `.wasm` into `dist/assets`.
  const w = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  worker = w;
  w.onerror = (e: ErrorEvent) =>
    drop(`the local host failed to start: ${e.message || "the Worker reported an error"}`);
  w.onmessageerror = () => drop("the local host sent something this page could not read");
  w.onmessage = (
    e: MessageEvent<Reply & { kind?: string; id?: string; sentence?: string; trapped?: boolean }>,
  ) => {
    const data = e.data;
    // **Unsolicited, and NOT a failure** (M6.5b2, spec §2.6). The Worker has
    // something to say about its own state while it is still working on it —
    // today, that another tab is holding the old storage version open and this
    // one is waiting for it to go. It reaches the same handlers a fault does,
    // because it is the same line on the same screen; what it must NOT do is
    // settle the pending calls, which is what `fail` would have done and which
    // would have turned a wait that ends by itself into a screenful of
    // refusals.
    if (data.kind === "notice" && typeof data.sentence === "string") {
      for (const f of [...faultHandlers]) f(data.sentence);
      return;
    }
    if (data.kind === "lost" && typeof data.id === "string") {
      // **Unsolicited**: the lock this tab held was stolen. The page turns the
      // screen read-only; it is told BEFORE the in-flight reply that goes with
      // it, so the screen is already right when the refusal lands.
      noteLost(data.id);
      return;
    }
    if (data.trapped) {
      // **Not deleted from `pending` first.** An earlier draft took this
      // request out before restarting, which left the one call that actually
      // met the trap waiting for ever while every other call was settled.
      if (restartsWithoutAnswer >= 1) {
        // The fresh Worker trapped too, and it has answered nothing: this is
        // not a trap to recover from, it is a host that will not start. Say so
        // in ITS own words (§2.3's visible line) rather than restarting again.
        drop(`the local host could not be started: ${sentenceOf(data.body)}`);
        return;
      }
      restartsWithoutAnswer += 1;
      restart();
      return;
    }
    // An ordinary answer is the only evidence the host works, and is what
    // gives the next trap its one restart back.
    restartsWithoutAnswer = 0;
    const p = pending.get(data.seq);
    pending.delete(data.seq);
    p?.resolve(data);
  };
  return w;
}

/** This tab does not hold `id` any more. ONE statement of it, reached from the
 *  Worker's unsolicited message and from a reply that says `not-held` for a
 *  game this page believed it held (review finding #4). */
function noteLost(id: string): void {
  heldByPage.delete(id);
  for (const f of [...lostHandlers]) f(id);
}

/** Raised while a restart is re-acquiring what this tab held, and opened when
 *  the re-holds have answered.
 *
 *  **Everything else waits for them** (spec §4: the page "starts a new Worker
 *  …, re-acquires the game's ownership, and only then says 'reloaded from the
 *  last saved move'"). The order is not decoration. A `hold` is answered
 *  OUTSIDE the Worker's serial queue, so a call sent in the same tick can be
 *  served before the lock is granted — and a call on a game the Worker does
 *  not yet hold is answered `not-held`, which the rule below reads as the lock
 *  being lost. The page then told the player their game had been opened in
 *  another tab, about its own restart, and took the answer bar away.
 *
 *  Measured in a real browser (`web/e2e/plays_locally.spec.ts`, the trap
 *  cell): after the restart `navigator.locks.query()` showed the game held by
 *  the FRESH Worker's client id, while the screen read "This game was opened
 *  in another tab" — the re-read `App`'s own refusal handler makes had
 *  overtaken the re-hold. */
let reacquiring: Promise<void> | null = null;

/** R5: a trapped module holds its own mutex and traps again on `init()`, so
 *  recovery is a FRESH INSTANCE — terminate, settle, start, re-hold.
 *
 *  The new Worker rebuilds from the committed pairs on boot, and nothing
 *  committed is lost, because a success reply is only sent after its write
 *  landed (§2.2). Re-acquiring is what makes the next call writable again:
 *  ownership lives in the Worker, and the Worker that held it is gone. */
function restart(): void {
  worker?.terminate();
  worker = null;
  const ids = [...heldByPage];
  // **The gate is raised BEFORE the pending calls are settled, and the holds
  // are sent AFTER.** Both halves of that order were measured, and each fixes
  // a different way of getting this wrong:
  //
  //  - Settling is what makes the page ask again — `App`'s own refusal handler
  //    re-reads the game — so a gate raised afterwards is raised behind the
  //    very call it exists for.
  //  - [`fail`] rejects EVERYTHING pending, and a hold sent first is pending.
  //    The first draft sent the holds first and gated on THEIR promises: `fail`
  //    rejected them a line later, the gate opened at once, and three reads
  //    went out and came back `not-held` while the acquisition was still in
  //    flight. That is the defect, in the log, with `navigator.locks.query()`
  //    showing the game held by this very Worker.
  //
  // So the gate is its own promise, opened when the re-holds have ANSWERED.
  let open!: () => void;
  const gate = new Promise<void>((resolve) => {
    open = resolve;
  });
  reacquiring = gate;
  fail(RESTART_SENTENCE);
  // `start()` happens inside `ask`; asking for the holds is what starts the new
  // Worker. Nothing else can be posted ahead of them: every other `ask` is now
  // waiting on the gate. **A Worker that cannot even be BUILT rejects its
  // `ask` rather than throwing out of it** (M6.5c2 cycle D, dossier row 56),
  // so this `Promise.all` is always reached and the gate always opens; before,
  // the throw escaped `ids.map` and every later call waited on a gate nothing
  // would ever open.
  void Promise.all(
    ids.map((id) =>
      ask<{ held?: boolean } & Partial<UnreadableReply>>({ kind: "hold", id })
        .then((r) => reheld(r, id))
        .catch(() => undefined),
    ),
  ).then(() => {
    // Only the LATEST restart's gate is cleared here: a second trap during the
    // first recovery installs its own, and clearing unconditionally would drop
    // it.
    if (reacquiring === gate) reacquiring = null;
    open();
  });
}

/**
 * A restart's re-hold of `id`, read by the hold's own rule (fix round 1,
 * review Minor 1). It used to be ignored. So a fresh Worker whose store could
 * not be read answered `{ unreadable }`, an ordinary reply, and the id stayed
 * on the page's list; the next write came back `not-held`, and the page said
 * the game "was opened in another tab" — false.
 *
 * - **The store could not be read**: the game leaves the page's list and every
 *   listener is told the storage's sentence for that game — the screen draws
 *   the broken-store face ([`onStoreUnreadable`]).
 * - **Not granted**: another tab took the game while this one restarted, and
 *   that IS the loss — [`noteLost`], one statement for every path to it.
 *
 * Read before the gate opens, so a call waiting on the gate meets a page that
 * already knows.
 */
function reheld(r: { held?: boolean } & Partial<UnreadableReply>, id: string): void {
  const unreadable = unreadableOf(r);
  if (unreadable !== null) {
    heldByPage.delete(id);
    for (const f of [...storeHandlers]) f(id, unreadable);
    return;
  }
  if (r.held !== true) noteLost(id);
}

function ask<T extends Record<string, unknown>>(msg: ReqBody): Promise<T> {
  // A restart's own re-holds are what the gate is waiting FOR, so they pass it;
  // everything else waits, and asks again once the ownership question has an
  // answer. A `hold` never blocks — the acquisition is `ifAvailable`, which is
  // refused rather than queued — so this cannot become a wait with no end.
  //
  // **The residual, named**: a call that overtakes an acquisition is still
  // answered `not-held`, and the page still reads that as the loss. This
  // closes the one path that reached it without another tab being involved.
  const gate = msg.kind === "hold" ? null : reacquiring;
  if (gate) return gate.then(() => ask<T>(msg));
  // **`ask` never throws; it rejects** (M6.5c2 cycle D, dossier row 56). Its
  // callers chain on the promise — the restart's re-holds above most of all —
  // and a synchronous throw skipped every one of them.
  let w: Worker;
  try {
    w = start();
  } catch (e) {
    // The browser would not build the Worker at all — a script the page may
    // not load. The same sentence and the same consequence as a Worker that
    // was built and then failed to start (`onerror`, below `start`): every
    // listener is told, and the next call tries again.
    const sentence = `the local host failed to start: ${e instanceof Error ? e.message : String(e)}`;
    drop(sentence);
    return Promise.reject(new Error(sentence));
  }
  const mine = (seq += 1);
  return new Promise<T>((resolve, reject) => {
    pending.set(mine, { resolve: (r) => resolve(r as unknown as T), reject });
    try {
      w.postMessage({ seq: mine, ...msg });
    } catch (e) {
      // A message that was never sent has no reply to wait for: out of the
      // pending list, and the call refused with what `postMessage` said.
      pending.delete(mine);
      reject(e instanceof Error ? e : new Error(String(e)));
    }
  });
}

/** One call on the local API. `path` may carry a query string; the Worker takes
 *  the two apart because `ApiRequest` does.
 *
 *  `saved` is FALSE only where the store refused the write — the page then
 *  shows the server's own "APPLIED BUT NOT SAVED" beside the move, which is
 *  one rule and one wording across both hosts. */
export async function localCall(
  method: string,
  path: string,
  body?: unknown,
): Promise<{ status: number; text: string; saved: boolean; reason: string | null }> {
  const cut = path.indexOf("?");
  const p = cut === -1 ? path : path.slice(0, cut);
  const q = cut === -1 ? "" : path.slice(cut + 1);
  const r = await ask<
    { status: number; body: string; saved?: Saved; reason?: string } & Partial<UnreadableReply>
  >({
    kind: "call",
    method,
    path: p,
    query: q,
    body: body === undefined ? null : JSON.stringify(body),
  });
  // **The Worker could not read this browser's storage to answer at all** —
  // its startup (`ready`) failed on the open or the read (M6.5c2 cycle D,
  // dossier row 14). Nothing was applied and nothing written, so no listing is
  // told to re-read; the answer is a refusal in the storage's own words, as a
  // 503 — the host is up and its store is not — so `settle` raises it the way
  // every other refusal is raised. (No GAME read meets it: a game is read only
  // after its hold is granted, and a hold made while the startup fails is
  // refused by the same store first — so `App`'s broken-save arm, which takes
  // any local 5xx on a game read, never sees this one.)
  const unreadable = unreadableOf(r);
  if (unreadable !== null) {
    return { status: 503, text: JSON.stringify({ error: unreadable }), saved: false, reason: null };
  }
  // **Every write, whatever it wrote**, so the local listing can re-read. A
  // GET changes nothing a list shows — except the bot's pump on a cold read,
  // which only ever happens on the way into the screen that is about to draw
  // it anyway.
  if (method !== "GET") for (const f of [...writeHandlers]) f();

  // **The browser is asked to KEEP these games at the first committed move,
  // and never at boot** (M6.5b2, R-6.5b2.9; spec §2.4). A visitor who never
  // plays is never asked; a player who has just committed a move has given the
  // browser a reason to say yes, and in some browsers the answer is decided
  // from exactly that kind of engagement.
  //
  // **Three conditions and each one is load-bearing.** `"saved"` alone, because
  // a store that has just REFUSED a write is not a store to ask for a promise —
  // that reply carries the backup line instead. `POST …/input` alone, because a
  // create is a table and not yet a game, and a GET is a read. And
  // `requestPersistenceOnce` holds the page flag, so a run of moves asks once.
  //
  // This is the one route this wire knows the meaning of, which the header's
  // "nothing here knows a route" is otherwise true of — it sits here because
  // this is the only place a COMMITTED local write is distinguishable from an
  // applied-but-unsaved one, and `api.ts` sees the same reply with that
  // distinction already flattened into a warning.
  if (r.saved === "saved" && method === "POST" && p.endsWith("/input")) {
    void requestPersistenceOnce();
  }

  // **`not-held` is a lock loss when this page believed it held the game**
  // (review finding #4). The Worker posts its own unsolicited `lost` on the
  // paths it notices — the steal's abort, a write that reads a newer
  // generation — and this is the arm for a reply that got here first, or for
  // a hold this tab let go of without the Worker saying so. `noteLost` is the
  // same statement both reach, so the screen turns read-only once.
  const game = gameIdOf(path);
  if (r.saved === "not-held" && game !== null && heldByPage.has(game)) noteLost(game);

  // **A WRITE the Worker did not keep is the LOCK LOSS, and is refused here**
  // (spec §2.2; whole-branch review, Important #1). §2.2's in-flight rule is
  // that a reply from a call in flight when the lock went "is answered to the
  // page as the lock loss, so nothing is shown that will not be kept". The
  // Worker holds that on the generation path, where a stale write becomes a
  // 409 with these very words. It cannot hold it on the other path: once the
  // steal's abort has removed the hold, the queued write is answered
  // `not-held`, and that reply carried the module's own 200 — so the page drew
  // a move applied in a store about to be discarded, under "APPLIED BUT NOT
  // SAVED", whose reason names the store. The store refused nothing.
  //
  // **AFTER `noteLost`, deliberately**, which is the Worker's own order: the
  // screen is already read-only when the refusal lands. A GET is untouched —
  // it wrote nothing and was never meant to, and a second tab's cold read is
  // answered from its own store.
  if (r.saved === "not-held" && method !== "GET") {
    return {
      status: 409,
      text: JSON.stringify({ error: TAKEN_OVER_SENTENCE }),
      saved: false,
      reason: null,
    };
  }

  return {
    status: r.status,
    text: r.body,
    // **"Saved" means the transaction LANDED** (spec §2.2), so only `"saved"`
    // is saved — and a GET, which wrote nothing and was never meant to (a
    // second tab's cold read is answered from its own store). The first draft
    // read `!== "unsaved"`, which reported an unpersisted MOVE as kept: the
    // dangerous direction, and reachable without a page bug.
    saved: r.saved === "saved" || (r.saved === "not-held" && method === "GET"),
    reason: r.reason ?? null,
  };
}

/**
 * BOTH of a game's documents, read as ONE SNAPSHOT inside the Worker's queue
 * (spec §2.2; review finding #6) — each the exact bytes, or `null` where there
 * is none.
 *
 * **Which snapshot is the Worker's to decide, and it is not "whatever this
 * tab's module holds"** (PR #37 fix wave, R-F1; `exportPair.ts`). A tab's
 * module is loaded at its Worker's boot and written back only for a game this
 * tab holds, so it can be a copy another tab has moved past. The answer is the
 * module's pair only where this tab OWNS the game at the current ownership
 * generation — the one place a move applied and not yet written is worth
 * keeping — and the pair IndexedDB committed everywhere else. Every export goes
 * through here, so every export takes that rule: both backups, Save game file,
 * Send to the archive, Abandon and save, and the outbox. A caller takes the
 * answer WHOLE; a `null` in it is part of the answer, not a gap to fill from
 * elsewhere.
 *
 * **A store that cannot be read REJECTS, and restarts nothing** (fix round 1,
 * review I-1). The Worker answers that case `unreadable` rather than
 * `trapped`, and this rejects with its sentence: a restart would terminate the
 * Worker, and an owner's Worker is holding the one copy of any move the store
 * refused. Each caller already names a rejection — the backups fall back to
 * this page's own read, Abandon keeps the game, the outbox writes the sentence
 * on the game's row, and Save game file and Send to the archive show it.
 *
 * **One message and not two**, which is the whole of why it exists. "Send to
 * the archive" sends the record and the table together and the server checks
 * that they agree; two queued reads can fall either side of a move, and the
 * pair would be refused for a mismatch this client caused. `recordText`'s
 * local arm reads the record off it for the same reason it costs nothing to.
 *
 * Neither document is parsed on the way: the record's `seed` is a `u64` that
 * `JSON.parse` would round, and the record never goes through `…/record`
 * either, which the local dispatch still answers with a 409 while the game
 * runs (R-6.5b.7 saves at any point).
 */
export const localPair = async (
  id: string,
): Promise<{ record: string | null; table: string | null }> => {
  const r = await ask<{ record: string | null; table: string | null } & Partial<UnreadableReply>>({
    kind: "pair",
    id,
  });
  const unreadable = unreadableOf(r);
  if (unreadable !== null) throw new Error(unreadable);
  return { record: r.record, table: r.table };
};

/**
 * Ask the module what is in a backup file (M6.5b2, R-6.5b2.10 – .12).
 *
 * **The report as the module's own JSON string**, unparsed here: this module
 * moves strings and knows no shape, and the page's `backup.ts` is the one
 * reader. A file the validator refuses comes back as a REJECTION carrying its
 * sentence — that is an answer about the file and not a fault, so it does not
 * go through the trap path.
 */
export async function localRestoreRead(envelope: string): Promise<string> {
  const r = await ask<{ report?: string; refused?: string } & Partial<UnreadableReply>>({
    kind: "restore_read",
    envelope,
  });
  // **The STORE's refusal first, and never as the file's** (fix round 1,
  // review Important 1). `restore_read` is queued like every other message,
  // so it waits on the Worker's startup; when that cannot read the store the
  // reply is `{ unreadable }` and carries neither a report nor a refusal —
  // and the fallback below would have told the player their FILE could not
  // be read.
  const unreadable = unreadableOf(r);
  if (unreadable !== null) throw new LocalStoreUnreadable(unreadable);
  if (typeof r.report === "string") return r.report;
  throw new Error(r.refused ?? "the local host could not read this file");
}

/**
 * Bring games a restore has just written into the Worker's live store, and tell
 * every local listing to re-read (spec §2.5's "the running tab reconciles").
 *
 * The write handlers are fired HERE rather than by the Worker, for the reason
 * `localCall` fires them: a listing re-reads because something local changed,
 * and a restore is the one change that does not arrive through a call.
 */
export async function localAdopt(ids: string[]): Promise<number> {
  const r = await ask<{ adopted: number } & Partial<UnreadableReply>>({ kind: "adopt", ids });
  // **A store that could not be read is refused, and restarts nothing**
  // (M6.5c2 cycle D, dossier row 14). The refusal is about the STORE and not
  // a count: a later id's read can fail after earlier ones were loaded (fix
  // round 1, review Minor 5), so some games may be in the module already. No
  // listing is told to re-read here; the restore's own re-read afterwards, and
  // the next start, list whatever the module holds — the games are on the
  // device either way.
  const unreadable = unreadableOf(r);
  if (unreadable !== null) throw new LocalStoreUnreadable(unreadable);
  for (const f of [...writeHandlers]) f();
  return r.adopted;
}

/** Take this game for this tab: `false` means another tab has it open.
 *
 *  The acquisition also advances the game's ownership generation and rebuilds
 *  the Worker's store from IndexedDB, so the holder always starts from the
 *  last state ANY tab acknowledged. */
export async function hold(id: string): Promise<boolean> {
  return acquired(await ask<{ held: boolean } & Partial<UnreadableReply>>({ kind: "hold", id }), id);
}

/** Take it anyway (R-6.5b.5). The lock is STOLEN, which is what turns the other
 *  tab read-only: its own request rejects and its Worker tells its page.
 *
 *  **A steal the Worker did not grant is REFUSED, and is not recorded**
 *  (M6.5c2 cycle D, dossier row 56): the reply's `held` used to be ignored and
 *  the game added to what this page holds whatever it said — so the screen
 *  believed it played a game another tab had just taken, and a restart would
 *  re-acquire it. */
export async function takeover(id: string): Promise<void> {
  if (!acquired(await ask<{ held: boolean } & Partial<UnreadableReply>>({ kind: "takeover", id }), id)) {
    throw new Error(TAKEOVER_REFUSED_SENTENCE);
  }
}

/** An acquisition's answer, read ONCE for both: the storage's refusal as a
 *  [`LocalStoreUnreadable`] (M6.5c2 cycle D, dossier row 14), and a granted
 *  hold recorded as this page's. */
function acquired(r: { held?: boolean } & Partial<UnreadableReply>, id: string): boolean {
  const unreadable = unreadableOf(r);
  if (unreadable !== null) throw new LocalStoreUnreadable(unreadable);
  const ok = r.held === true;
  if (ok) heldByPage.add(id);
  return ok;
}

/** Let it go — what the game screen does when it unmounts, so navigating away
 *  frees the game for another tab. Resolves once the lock is actually gone. */
export async function release(id: string): Promise<void> {
  heldByPage.delete(id);
  await ask({ kind: "release", id });
}

/** This tab lost a game it held. */
export const onLost = (f: (id: string) => void): (() => void) => {
  lostHandlers.add(f);
  return () => lostHandlers.delete(f);
};

/** A game this page held could not be acquired again after a restart,
 *  because this browser's storage could not be read — the game and the
 *  storage's sentence (fix round 1, review Minor 1; see [`reheld`]). The hook
 *  draws it as the broken-store face. */
export const onStoreUnreadable = (f: (id: string, sentence: string) => void): (() => void) => {
  storeHandlers.add(f);
  return () => storeHandlers.delete(f);
};

/** Something local was written — the cue to re-read a local listing. */
export const onLocalWrite = (f: () => void): (() => void) => {
  writeHandlers.add(f);
  return () => writeHandlers.delete(f);
};

/** The local host is in trouble, in one sentence to show. */
export const onLocalFault = (f: (sentence: string) => void): (() => void) => {
  faultHandlers.add(f);
  return () => faultHandlers.delete(f);
};

/** Whether this tab has a Worker at all — what tells "the module never loaded"
 *  from "nobody has played locally yet". */
export const localHostStarted = (): boolean => worker !== null;

/** How many calls are waiting for an answer — **for the CELLS**, and named
 *  so, as [`resetLocalHostForTests`] is: a waiting call is otherwise visible
 *  only when it is finally settled, and one that is never going to be answered
 *  is exactly what a cell has to be able to count. */
export const pendingCallsForTests = (): number => pending.size;

/**
 * Terminate the Worker and forget everything this page held.
 *
 * **For the CELLS, and named so.** The Worker is a module-level singleton with
 * a page's lifetime, so there is no production caller: a page that is done
 * with local play is a page that is closing. A test file, though, gets one
 * module registry for all its cells, and a fake Worker left installed by one
 * cell would answer the next one's calls.
 */
export function resetLocalHostForTests(): void {
  worker?.terminate();
  worker = null;
  reacquiring = null;
  seq = 0;
  pending.clear();
  heldByPage.clear();
  restartsWithoutAnswer = 0;
  lostHandlers.clear();
  storeHandlers.clear();
  writeHandlers.clear();
  faultHandlers.clear();
}
