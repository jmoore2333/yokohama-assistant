// The local host (spec §2.2; R-6.5b.4, R-6.5b.5) — one dedicated Worker per
// tab, holding the module, the local `Games`, the IndexedDB side channel and
// the Web Lock.
//
// **Why a Worker at all.** The replay on load rebuilds a state from a seed and
// its events, grows with the record, and would freeze the screen on the main
// thread; `suggest` and `why` run the same machinery. Nothing on the page can
// reach the module except through the messages below.
//
// ## The actor, in the order the rules matter
//
// 1. **ONE operation queue** (R3). `dispatch`, `persist`, `rebuild`, the
//    acquisition and `release` all run through [`serial`], so a rebuild can
//    never replace the store under a persist and two acquisitions cannot
//    interleave.
// 2. **`serial` is never re-entered.** No function reachable from inside a
//    queued operation calls it: the queued body would wait for a slot that
//    only it can free. `hold`'s acquisition is the one place that enqueues
//    from outside, and it is reached from the lock manager's callback — which
//    is not inside the queue.
// 3. **The lock's callback promise is held OPEN outside the queue**, for as
//    long as the hold lasts. A queue that awaited a lock's lifetime would
//    answer nothing else for the whole game.
// 4. **Persistence is by OUTCOME, for the documents that EXIST, for the games
//    this tab HOLDS, and BEFORE the page is answered** (R1, R2, R4). No method
//    list: one rule that covers the move, the bot's pump on a cold read, the
//    cold-rebuild repair and anything a later cycle adds.
// 5. **Every write re-reads the ownership GENERATION inside its own
//    transaction** (R2). The Web Locks specification warns that a holder that
//    is stolen from may keep executing, and a message cannot fence a wasm call
//    already running.
// 6. **A trap is answered, and the PAGE starts a fresh Worker** (R5). A
//    trapped module holds its own mutex and traps again on `init()`, so there
//    is nothing to recover in place. A STORAGE refusal is not a trap, and a
//    stored game that no longer rebuilds is not a trap either: both are
//    ordinary replies.

import { deletePair, openDb, readAll, takeGeneration, writeDocs } from "./db";
import { drawLocalId, gameIdOf, LOCAL_USER } from "./ids";
import init, * as wasm from "./wasm/yokohama_wasm.js";

/** The page's requests. Mirrors `client.ts`'s `Req`, which is the page's own
 *  statement of the same shape — the two sides of a `postMessage` cannot share
 *  a module here without putting the page's imports in the Worker's program. */
type Req = { seq: number } & (
  | { kind: "call"; method: string; path: string; query: string; body: string | null }
  | { kind: "pair"; id: string }
  | { kind: "hold"; id: string }
  | { kind: "takeover"; id: string }
  | { kind: "release"; id: string }
);

/** What this tab owns: the generation it took, the way to let the lock go, and
 *  a promise that settles once the lock actually HAS gone. */
interface Held {
  generation: number;
  release: () => void;
  done: Promise<void>;
}

const ctx = self as unknown as DedicatedWorkerGlobalScope;
const post = (m: unknown): void => ctx.postMessage(m);
const noop = (): void => undefined;

let db: IDBDatabase;
/** The bytes last committed per game, so an unchanged pair costs no write.
 *  CLEARED and reseeded on every rebuild (R3): a take-back followed by an undo
 *  can land on bytes an older cache still matched, and a skipped write there
 *  would lose the undo. */
const lastWritten = new Map<string, string>();
const held = new Map<string, Held>();
/** What the PAGE wants held right now, and the acquisitions in flight.
 *
 *  Both are about one race, which React's StrictMode makes ordinary in
 *  development: an effect is mounted, cleaned up and mounted again, so `hold`,
 *  `release` and `hold` can all be sent before the first has resolved. See
 *  [`hold`] for what each one prevents. */
const wanted = new Set<string>();
const inFlight = new Map<string, Promise<boolean>>();

let queue: Promise<unknown> = Promise.resolve();

/** The ONE operation queue. See rule 2 above: nothing called from inside `f`
 *  may call this. */
const serial = <T>(f: () => Promise<T>): Promise<T> => {
  const p = queue.then(f, f);
  queue = p.then(noop, noop);
  return p;
};

/** The module, IndexedDB, and the store loaded from it — before anything is
 *  answered. Awaited by every message; there is no readiness probe, because
 *  every message already runs after it (R-6.5b.12 leaves `/api/health` with no
 *  local caller in the page). */
const ready: Promise<void> = (async () => {
  await init();
  db = await openDb();
  await rebuild();
})();

/** A FRESH `Games` from the committed documents, and the write cache reseeded
 *  from them.
 *
 *  **Every stored pair, not only the acquired game's** (spec §2.2): `init()`
 *  empties the store, so every other local game in this tab would 404 until it
 *  was reopened. Loading bytes is cheap and the `Repl` rebuild stays lazy per
 *  game on first touch. */
async function rebuild(): Promise<void> {
  wasm.init();
  lastWritten.clear();
  const rows = await readAll(db);
  const byId = new Map<string, { record?: string; table?: string }>();
  for (const r of rows) {
    wasm.load(r.doc, r.id, r.text);
    byId.set(r.id, { ...byId.get(r.id), [r.doc]: r.text });
  }
  for (const [id, d] of byId) lastWritten.set(id, cacheKey(d.record, d.table));
}

/** One string standing for both documents. The NUL is a separator no JSON
 *  document contains, so no two distinct pairs share a key. */
const cacheKey = (record: string | undefined, table: string | undefined): string =>
  `${record ?? ""}\u0000${table ?? ""}`;

/** What became of the write that followed a call. */
type Outcome = { saved: "saved" | "unsaved" | "not-held" | "lost"; reason?: string };

/** This tab no longer owns `id`: forget it, let the lock go, and tell the page
 *  ONCE. Called from the steal's own rejection and from a write that found a
 *  newer generation — whichever notices first. */
function lose(id: string): void {
  const entry = held.get(id);
  if (!entry) return;
  held.delete(id);
  post({ kind: "lost", id });
  entry.release();
}

/**
 * Commit what the module now holds for `id` — the documents that EXIST, only
 * for a game this tab HOLDS (R1, R2).
 *
 * Reading the module and writing IndexedDB both happen inside the caller's
 * queued operation, so the snapshot cannot be replaced by a rebuild halfway
 * through.
 */
async function persist(id: string, deleted: boolean): Promise<Outcome> {
  const entry = held.get(id);
  if (!entry) return { saved: "not-held" };

  // **The MODULE's reads sit outside the `try`** (§4, R4; review finding #2).
  // `read` returns a `Result` and can throw, and the `catch` below answers
  // `unsaved` — the page's "APPLIED BUT NOT SAVED", which is about the STORE.
  // A module fault reported that way is the one classification §4 says must be
  // distinguished; escaping this function is what puts it on the trap path.
  const table = deleted ? undefined : wasm.read("table", id);
  const record = deleted ? undefined : wasm.read("record", id);

  try {
    if (deleted) {
      // **Fenced by the generation like every other write** — review finding
      // #1 (`db.ts`'s `deletePair`): a stale holder's delete removes nothing
      // and is answered as the lock loss.
      const outcome = await deletePair(db, id, entry.generation);
      if (outcome === "stale") {
        lose(id);
        return { saved: "lost" };
      }
      lastWritten.delete(id);
      return { saved: "saved" };
    }
    // Nothing to keep. A game this tab holds but the module does not know is
    // the shape of a 404, and there is no document to write.
    if (table === undefined) return { saved: "saved" };
    const key = cacheKey(record, table);
    if (lastWritten.get(id) === key) return { saved: "saved" };
    const outcome = await writeDocs(db, id, entry.generation, { record, table });
    if (outcome === "stale") {
      lose(id);
      return { saved: "lost" };
    }
    lastWritten.set(id, key);
    return { saved: "saved" };
  } catch (e) {
    // **A storage refusal is NOT a trap** (R4): quota, a private window. The
    // Worker keeps running, the game stays exportable, and the page shows the
    // server's own "APPLIED BUT NOT SAVED". Only IndexedDB is inside this
    // `try` — see the reads above.
    return { saved: "unsaved", reason: e instanceof Error ? e.message : String(e) };
  }
}

/** A create's 201, committed under generation 0 (P-4).
 *
 *  **Unconditional, and the table alone.** Nothing holds an id drawn a moment
 *  ago, so the held-games rule would keep nothing — and the hold that follows
 *  `#/t/local-…` rebuilds from IndexedDB and would 404 the game the player
 *  just opened. The id is fresh by construction, so there is no other writer
 *  to race; generation 0 is what a game no tab has held reads, so the write
 *  passes its own fence without advancing it. */
async function persistCreate(id: string): Promise<Outcome> {
  // Outside the `try`, for `persist`'s reason (review finding #2): a throw
  // from the module is a trap, not a storage refusal.
  const table = wasm.read("table", id);
  try {
    if (table === undefined) return { saved: "saved" };
    const outcome = await writeDocs(db, id, 0, { table });
    if (outcome === "stale") return { saved: "lost" };
    lastWritten.set(id, cacheKey(undefined, table));
    return { saved: "saved" };
  } catch (e) {
    return { saved: "unsaved", reason: e instanceof Error ? e.message : String(e) };
  }
}

/** What a reply says when the lock went while the call was in flight (spec
 *  §2.2): the module's answer is discarded and the page is told the loss, so
 *  nothing is shown that will not be kept. */
const LOST_SENTENCE =
  "this game was taken over in another tab — what you just did here was not kept";

const isCreate = (req: Extract<Req, { kind: "call" }>): boolean =>
  req.method === "POST" && req.path === "/api/games";

/** Give the create the id this Worker drew. The seed is NOT supplied:
 *  `draw_seed` inside the module draws it through `getrandom`. */
function withDrawnId(body: string | null, id: string): string | null {
  if (body === null) return null;
  try {
    const parsed: unknown = JSON.parse(body);
    if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return body;
    return JSON.stringify({ ...(parsed as Record<string, unknown>), id });
  } catch {
    // Not JSON: hand it on and let the module refuse it in its own words.
    return body;
  }
}

const encoder = new TextEncoder();

async function call(req: Extract<Req, { kind: "call" }>): Promise<Record<string, unknown>> {
  const drawn = isCreate(req) ? drawLocalId() : null;
  const body = drawn === null ? req.body : withDrawnId(req.body, drawn);
  const reply = wasm.dispatch(
    LOCAL_USER,
    req.method,
    req.path,
    req.query,
    body === null ? undefined : "application/json",
    body === null ? new Uint8Array() : encoder.encode(body),
  );
  let status: number;
  let text: string;
  try {
    status = reply.status;
    text = reply.body;
  } finally {
    // The module hands back a boxed struct; reading it and freeing it is the
    // whole of its lifetime here.
    reply.free();
  }

  if (drawn !== null) {
    const out = status === 201 ? await persistCreate(drawn) : { saved: "saved" as const };
    return { status, body: text, ...out };
  }
  const id = gameIdOf(req.path);
  if (id === null) return { status, body: text, saved: "saved" };
  const out = await persist(id, req.method === "DELETE" && status === 204);
  if (out.saved === "lost") {
    return {
      status: 409,
      body: JSON.stringify({ error: LOST_SENTENCE }),
      saved: "lost",
    };
  }
  return { status, body: text, ...out };
}

const lockName = (id: string): string => `yokohama:local:${id}`;

/**
 * Take this game for this tab, and REBUILD from IndexedDB while taking it
 * (spec §2.2, R-6.5b.5).
 *
 * The acquisition — the generation and the rebuild — is ONE queued operation,
 * so no call can run against a half-loaded store. The lock's own promise is
 * held open outside the queue for the life of the hold.
 *
 * **The page's INTENT is what decides, and it is tracked** (`wanted`). React's
 * StrictMode double-invokes an effect in development — mount, cleanup, mount —
 * so Task 6's `useLocalHold` really does send `hold`, `release` and `hold`
 * again before the first has resolved. Without `wanted` the release would find
 * nothing in `held` and the acquisition landing afterwards would take a lock
 * nobody ever lets go; without `inFlight` the second hold would issue a second
 * `ifAvailable` request, which this tab's own pending one answers `null` — and
 * the player would be told their own tab has the game open.
 */
function hold(id: string, steal: boolean): Promise<boolean> {
  wanted.add(id);
  // **A re-request for a game this tab already holds short-circuits** (spec
  // §2.2), rather than telling the player their own tab has it open.
  if (held.has(id)) return Promise.resolve(true);
  // A STEAL always issues its own request: it means something different from
  // the `ifAvailable` one that may be in flight.
  if (!steal) {
    const already = inFlight.get(id);
    if (already) return already;
  }

  const acquire = async (): Promise<number> =>
    serial(async () => {
      const generation = await takeGeneration(db, id);
      await rebuild();
      return generation;
    });

  /** The page let go while this acquisition was in flight.
   *
   *  **Answered `false`, and that is review finding #5.** An earlier draft
   *  said `true` — "the lock WAS taken" — and returned without recording the
   *  entry, so the page added the game to `heldByPage` for a hold the Worker
   *  does not have: a restart would re-acquire a lock nobody wants, and every
   *  write on that game would come back `not-held`. The honest answer for a
   *  hold that is not recorded is that this tab does not hold it. */
  const gone = (): boolean => !wanted.has(id);

  let attempt: Promise<boolean>;

  if (!hasLocks()) {
    // **The one place the two-tab rule is not ENFORCED by a lock** — a browser
    // with no Web Locks API (R-6.5b.5 measures support; Chrome 69, Firefox 96,
    // Safari 15.4). It is not unfenced: the acquisition still advances the
    // generation, so a second tab opening the same game still takes ownership
    // and the first tab's next write still reads STALE and is told it lost.
    // What is missing is only the polite refusal BEFORE the second tab opens —
    // last opener wins instead of being offered Take over.
    attempt = acquire().then((generation) => {
      if (gone()) return false;
      held.set(id, { generation, release: noop, done: Promise.resolve() });
      return true;
    });
  } else {
    let answered = false;
    let say!: (v: boolean) => void;
    let refuse!: (e: unknown) => void;
    attempt = new Promise<boolean>((resolve, reject) => {
      say = (v: boolean) => {
        if (!answered) {
          answered = true;
          resolve(v);
        }
      };
      // **A failed acquisition is not "open in another tab".** IndexedDB
      // refusing to give a generation is a fault to NAME, and `false` would
      // send the player to the Take-over screen for a lock nobody holds.
      refuse = (e: unknown) => {
        if (!answered) {
          answered = true;
          reject(e instanceof Error ? e : new Error(String(e)));
        }
      };
    });

    let mine: Held | null = null;
    const options: LockOptions = steal
      ? { mode: "exclusive", steal: true }
      : { mode: "exclusive", ifAvailable: true };

    const request = navigator.locks.request(lockName(id), options, async (lock) => {
      // `ifAvailable` hands the callback `null` rather than rejecting when the
      // lock is somebody else's — that `null` is "This game is open in another
      // tab".
      if (lock === null) {
        say(false);
        return;
      }
      let generation: number;
      try {
        generation = await acquire();
      } catch (e) {
        refuse(e);
        return;
      }
      if (gone()) {
        say(false);
        return;
      }
      await new Promise<void>((release) => {
        // `done` is declared below and assigned before this callback can run:
        // the lock manager invokes it asynchronously, and the assignment is the
        // statement after `request`.
        mine = { generation, release, done };
        held.set(id, mine);
        say(true);
      });
      // Identity-checked, because a steal followed by a Take back puts a
      // DIFFERENT entry under this id and this callback must not remove it.
      if (held.get(id) === mine) held.delete(id);
    });

    /** Settles when the lock is gone — a release, or a steal. What `release`
     *  waits for, so a hold taken right afterwards is not refused by this
     *  tab's own lock still being held. */
    const done: Promise<void> = request.then(noop, noop);

    request.catch((e: unknown) => {
      // **A steal rejects the previous holder's request with an `AbortError`**
      // — the Web Locks specification's own mechanism, and the only notice a
      // stolen-from tab gets.
      const entry = mine;
      if (entry && held.get(id) === entry) {
        held.delete(id);
        post({ kind: "lost", id });
        entry.release();
      }
      // A steal is the expected rejection and is answered `false` (the hold
      // already resolved `true` long ago, so this settles nothing); anything
      // else is a fault, which `refuse` names.
      if (e instanceof DOMException && e.name === "AbortError") say(false);
      else refuse(e);
    });
  }

  inFlight.set(id, attempt);
  const clear = (): void => {
    if (inFlight.get(id) === attempt) inFlight.delete(id);
  };
  attempt.then(clear, clear);
  return attempt;
}

const hasLocks = (): boolean =>
  typeof navigator !== "undefined" && typeof navigator.locks?.request === "function";

/** The acquisition messages, run OUTSIDE the queue.
 *
 *  **This is the deadlock the plan's sketch had** (P-3): a `hold` run under
 *  the queue waits for a lock whose acquisition wants the same queue. The
 *  queued part is inside [`hold`], where it belongs. */
async function acquisition(req: Extract<Req, { kind: "hold" | "takeover" }>): Promise<void> {
  try {
    await ready;
    post({ seq: req.seq, held: await hold(req.id, req.kind === "takeover") });
  } catch (err) {
    post(trapped(req.seq, err));
  }
}

const trapped = (seq: number, err: unknown): Record<string, unknown> => ({
  seq,
  status: 500,
  body: JSON.stringify({ error: `the local host stopped: ${String(err)}` }),
  trapped: true,
});

async function handle(req: Req): Promise<void> {
  await ready;
  switch (req.kind) {
    case "call":
      post({ seq: req.seq, ...(await call(req)) });
      return;
    case "pair":
      // **BOTH documents, read inside ONE queued operation** (spec §2.2's
      // "the record and table are read for export as one snapshot"; review
      // finding #6). Two messages would be two operations a move can fall
      // between, and "Send to the archive" would send a table the server finds
      // does not match the record.
      post({
        seq: req.seq,
        record: wasm.read("record", req.id) ?? null,
        table: wasm.read("table", req.id) ?? null,
      });
      return;
    case "release": {
      // **The page's intent first**, so an acquisition still in flight drops
      // the lock when it lands rather than holding it for ever (see [`hold`]).
      wanted.delete(req.id);
      const entry = held.get(req.id);
      if (entry) {
        held.delete(req.id);
        entry.release();
        // Wait for the lock to actually go, so a hold taken straight
        // afterwards — leaving a game and coming back — is not refused by this
        // tab's own release still unwinding.
        await entry.done;
      }
      post({ seq: req.seq, released: true });
      return;
    }
    // The acquisitions never reach here; `onmessage` routes them.
    case "hold":
    case "takeover":
      return;
  }
}

ctx.onmessage = (e: MessageEvent<Req>) => {
  const req = e.data;
  if (req.kind === "hold" || req.kind === "takeover") {
    void acquisition(req);
    return;
  }
  void serial(async () => {
    try {
      await handle(req);
    } catch (err) {
      // A trap inside the module, or a module that never loaded. Answered, and
      // then the PAGE recreates this Worker (R5) — a trapped instance holds its
      // own mutex and traps again on `init()`.
      post(trapped(req.seq, err));
    }
  });
};
