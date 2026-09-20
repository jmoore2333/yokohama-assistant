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

/** What the page asks the Worker for. The `call` shape's fields are
 *  `ApiRequest`'s own — the Worker adds `local@browser` and nothing else. */
export type ReqBody =
  | { kind: "call"; method: string; path: string; query: string; body: string | null }
  | { kind: "read"; doc: "record" | "table"; id: string }
  | { kind: "hold"; id: string }
  | { kind: "takeover"; id: string }
  | { kind: "release"; id: string };

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

let worker: Worker | null = null;
let seq = 0;
const pending = new Map<number, { resolve: (r: Reply) => void; reject: (e: Error) => void }>();
const lostHandlers = new Set<(id: string) => void>();
const writeHandlers = new Set<() => void>();
const faultHandlers = new Set<(sentence: string) => void>();
/** The games the PAGE believes it holds — what to re-acquire after a restart.
 *  Kept here and not in the Worker for the reason a restart exists: the Worker
 *  that knew is the one that is gone. */
const heldByPage = new Set<string>();
/** Restarts with nothing answered in between.
 *
 *  **One is recovery; a second in a row is a host that cannot boot**, and
 *  restarting again is a loop — a module that fails to load, or an IndexedDB
 *  that will not open, traps on every message including the `hold` the restart
 *  itself sends. The budget is replenished by any ordinary reply, which is the
 *  only evidence that the host works. */
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
    fail(`the local host failed to start: ${e.message || "the Worker reported an error"}`);
  w.onmessageerror = () => fail("the local host sent something this page could not read");
  w.onmessage = (e: MessageEvent<Reply & { kind?: string; id?: string; trapped?: boolean }>) => {
    const data = e.data;
    if (data.kind === "lost" && typeof data.id === "string") {
      // **Unsolicited**: the lock this tab held was stolen. The page turns the
      // screen read-only; it is told BEFORE the in-flight reply that goes with
      // it, so the screen is already right when the refusal lands.
      heldByPage.delete(data.id);
      for (const f of [...lostHandlers]) f(data.id);
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
        worker?.terminate();
        worker = null;
        fail(`the local host could not be started: ${sentenceOf(data.body)}`);
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
  fail(RESTART_SENTENCE);
  const ids = [...heldByPage];
  // `start()` happens inside `ask`; asking for the holds is what starts it.
  for (const id of ids) void ask({ kind: "hold", id }).catch(() => undefined);
}

function ask<T extends Record<string, unknown>>(msg: ReqBody): Promise<T> {
  const w = start();
  const mine = (seq += 1);
  return new Promise<T>((resolve, reject) => {
    pending.set(mine, { resolve: (r) => resolve(r as unknown as T), reject });
    w.postMessage({ seq: mine, ...msg });
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
  const r = await ask<{ status: number; body: string; saved?: Saved; reason?: string }>({
    kind: "call",
    method,
    path: p,
    query: q,
    body: body === undefined ? null : JSON.stringify(body),
  });
  // **Every write, whatever it wrote**, so the local listing can re-read. A
  // GET changes nothing a list shows — except the bot's pump on a cold read,
  // which only ever happens on the way into the screen that is about to draw
  // it anyway.
  if (method !== "GET") for (const f of [...writeHandlers]) f();
  return {
    status: r.status,
    text: r.body,
    saved: r.saved !== "unsaved",
    reason: r.reason ?? null,
  };
}

/** A stored document's exact bytes, or `null`. What `recordText` reads for a
 *  local game: the record never goes through `…/record` (a 409 while the game
 *  runs — R-6.5b.7 saves at any point) and is never parsed on the way. */
export const localRead = async (doc: "record" | "table", id: string): Promise<string | null> =>
  (await ask<{ text: string | null }>({ kind: "read", doc, id })).text;

/** Take this game for this tab: `false` means another tab has it open.
 *
 *  The acquisition also advances the game's ownership generation and rebuilds
 *  the Worker's store from IndexedDB, so the holder always starts from the
 *  last state ANY tab acknowledged. */
export async function hold(id: string): Promise<boolean> {
  const ok = (await ask<{ held: boolean }>({ kind: "hold", id })).held;
  if (ok) heldByPage.add(id);
  return ok;
}

/** Take it anyway (R-6.5b.5). The lock is STOLEN, which is what turns the other
 *  tab read-only: its own request rejects and its Worker tells its page. */
export async function takeover(id: string): Promise<void> {
  await ask({ kind: "takeover", id });
  heldByPage.add(id);
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
  seq = 0;
  pending.clear();
  heldByPage.clear();
  restartsWithoutAnswer = 0;
  lostHandlers.clear();
  writeHandlers.clear();
  faultHandlers.clear();
}
