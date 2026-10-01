// The local host, faked — what a jsdom cell talks to instead of the module.
//
// **The real Worker is never loaded here, and could not be.** It imports the
// generated wasm glue, which fetches a `.wasm` Vite emits at build time; jsdom
// has no `Worker` at all, and a cell that started one would be testing the
// module rather than the client. So `vi.stubGlobal("Worker", …)` puts this
// class in its place and the cells assert what the CLIENT does with the
// answers.
//
// **Nothing here fabricates a payload**, which is `harness.tsx`'s rule and the
// reason it exists: every body this fake returns comes from `src/wire/`. The
// ONE edit is a fixture's `id` rewritten to a `local-…` one, because a local
// game is named for its host and the corpus's games are the server's — said
// at each site.

import { vi } from "vitest";
import { wire } from "../wire/contract";
import type { Req } from "../local/client";
import { isCreate, isGamesList } from "./matchers";
import { openDb, readPair } from "../local/db";
import type { UnreadableReply } from "../local/exportPair";
import type { Envelope, ReportRow } from "../backup";
import committedEnvelope from "../wire-script/backup-envelope.json";
import committedReport from "../wire-script/backup-report.json";

/** What a `call` is answered with: the module's two fields. `body` is given
 *  as a VALUE and stringified here, because that is what the Worker does. */
export interface CallReply {
  status: number;
  body: unknown;
  /** Whether the Worker committed it (spec §2.2). Omitted means `"saved"`. */
  saved?: "saved" | "unsaved" | "not-held" | "lost";
  reason?: string;
  /** The module TRAPPED: the page terminates this Worker and starts a fresh
   *  one (R5). */
  trapped?: boolean;
}

/** One game's two documents as the Worker answers them — BOTH keys, always.
 *
 *  **Both required** (fix round 1, review M-2). The page takes this answer
 *  WHOLE since R-F1, so a half answer would not be completed from the store:
 *  it would be a pair with a `null` in it, which a cell writing `{ record }`
 *  alone almost certainly did not mean. The real Worker sends both keys
 *  whenever it answers with a pair, and this is the one pair shape a cell can
 *  write. */
export interface FakePair {
  record: string | null;
  table: string | null;
}

/** What a cell may answer for itself. `undefined` from any of them falls
 *  through to the committed-corpus table below. */
export interface FakeWorkerHooks {
  /** `"pending"` answers NOTHING and leaves the call in flight — how a cell
   *  arranges a waiting request for the trap to settle. */
  call?: (req: Extract<Req, { kind: "call" }>) => CallReply | UnreadableReply | "pending" | undefined;
  /** `"pending"` leaves the ACQUISITION unanswered — how a cell arranges the
   *  window in which a Worker has been asked for a game and does not yet hold
   *  it.
   *
   *  `{ unreadable }` is the Worker's answer when IndexedDB would not be read
   *  or written during the acquisition — its generation, or the rebuild's read
   *  (M6.5c2 cycle D, dossier row 14): an ordinary reply, never a trap. The
   *  same answer on `call` is a call made while the Worker's startup could not
   *  read the store (its `ready`), and on `adopt` a restore's reconciliation
   *  that could not read the pairs it was told about. */
  hold?: (id: string, steal: boolean) => boolean | "pending" | UnreadableReply;
  /** What `adopt` answers — how many games it took, or `{ unreadable }`.
   *  Omitted: every id it was given. */
  adopt?: (ids: string[]) => number | UnreadableReply;
  /** Both documents of one game, as the `pair` message answers them.
   *
   *  `undefined` falls through to the COMMITTED pair in this page's
   *  IndexedDB, where the cell has one — the real Worker's answer for a game
   *  it does not own (PR #37 fix wave, R-F1).
   *
   *  `{ unreadable }` is the Worker's answer when IndexedDB would not be read
   *  (fix round 1, review I-1): an ordinary reply, never a trap.
   *
   *  `"pending"` answers NOTHING and leaves the read in flight — the `call`
   *  hook's own idiom, and how a cell arranges the OTHER half of "the module
   *  will not load": a Worker that started and whose module never came up
   *  answers no message at all (M6.5b2, review M-3). */
  pair?: (id: string) => FakePair | UnreadableReply | "pending" | undefined;
  /** What the module's `restore_read` says about a file — the rows, or a
   *  refusal about the FILE. A cell that is about ONE verdict sets this and
   *  says so; everything else falls through to [`fakeVerdicts`]. */
  restore?: (envelope: string) => ReportRow[] | { refused: string } | UnreadableReply | undefined;
}

/**
 * **A FAKE, and not the validator.**
 *
 * The real `restore_read` is `crates/yokohama-web/src/backup.rs` compiled into
 * the module, and it is pinned in three places that are not this one: that
 * file's own cells, `crates/yokohama-wasm`'s native cell, and
 * `web/src/backup.node.test.ts`, which runs the COMPILED module over the very
 * fixture read below. Nothing in jsdom can run it — there is no `Worker` and no
 * `.wasm` — so what the page cells assert is what the PAGE does with a verdict,
 * never what the verdict should have been.
 *
 * The mapping is DERIVED from the committed pair of files rather than written
 * out: a game's record text is looked up in the committed envelope and answered
 * with the verdict the committed report gives for it. So the four arms a cell
 * can reach are the four arms this build really produces, and a game whose
 * record this fixture does not carry is `accepted` — the premise most cells
 * want, and one they can override.
 */
function fakeVerdicts(envelope: string): ReportRow[] {
  const env = JSON.parse(envelope) as Envelope;
  const committed = committedEnvelope as unknown as Envelope;
  const rows = committedReport as unknown as ReportRow[];
  const NONE = "\u0000no record";
  const byRecord = new Map(
    committed.games.map((g, i) => [g.record ?? NONE, rows[i]!.verdict] as const),
  );
  return env.games.map((g) => ({
    id: g.id,
    verdict: byRecord.get(g.record ?? NONE) ?? {
      kind: "accepted" as const,
      players: 2,
      events: 0,
      over: false,
    },
  }));
}

/** What this page's IndexedDB COMMITTED for `id` — the real Worker's `pair`
 *  answer for a game it does not own (PR #37 fix wave, R-F1).
 *
 *  Two nulls where the cell has no database at all: jsdom has no IndexedDB,
 *  and a cell that wants one imports `fake-indexeddb`. A read that FAILS is
 *  left to reject, so it surfaces as the unhandled rejection it is rather than
 *  as a game that is not there. */
async function committedPair(id: string): Promise<{ record: string | null; table: string | null }> {
  if (typeof indexedDB === "undefined") return { record: null, table: null };
  const db = await openDb();
  try {
    return await readPair(db, id);
  } finally {
    db.close();
  }
}

const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v)) as T;

/** A corpus payload re-addressed to a local game. The id is the ONE field a
 *  fixture is allowed to lose here (see the file's own note). */
const named = <T extends { id: string }>(fixture: T, id: string): T => ({ ...clone(fixture), id });

/** What the module answers, from the committed corpus.
 *
 *  A path this table does not know is a THROW at the call site (see
 *  `postMessage`), which is `stubApi`'s rule: a screen that asked for
 *  something the cell did not think about should say so in the cell that
 *  caused it. */
function corpusReply(req: Extract<Req, { kind: "call" }>): CallReply | undefined {
  const { method, path, query } = req;
  const game = /^\/api\/games\/([^/]+)/.exec(path)?.[1];
  const rest = game ? path.slice(`/api/games/${game}`.length) : null;

  if (path === "/api/health") return { status: 200, body: wire.health };
  if (path === "/api/me") return { status: 200, body: wire.me };
  if (path === "/api/labels") return { status: 200, body: wire.labels };
  // The one listing the local host is asked for: this browser's own, `mine`.
  // Any other scope falls through to the throw a cell would want to see.
  if (isGamesList(req, "mine")) {
    // The lobby row shape, re-addressed and with the corpus row's own NAME
    // cleared — the `gamesNamed` rule: a row is listed as `name ?? id`, so a
    // row that kept `wire` would be found by no cell that looks for its id.
    return {
      status: 200,
      body: { games: [{ ...named(wire.games.games[0]!, "local-1"), name: null }], next: null },
    };
  }
  if (isCreate(req)) {
    return { status: 201, body: named(wire.tableDealing, "local-1") };
  }
  if (game === undefined || rest === null) return undefined;

  if (rest === "" && method === "GET") return { status: 200, body: named(wire.options, game) };
  if (rest === "" && method === "DELETE") return { status: 204, body: undefined };
  if (rest === "/table") return { status: 200, body: named(wire.table, game) };
  if (rest === "/deal") return { status: 200, body: wire.deal };
  if (rest === "/ports") return { status: 200, body: wire.ports };
  if (rest === "/labs") return { status: 200, body: wire.labs };
  if (rest === "/areas") return { status: 200, body: wire.areas };
  if (rest === "/log") return { status: 200, body: wire.log };
  if (rest === "/at" && query.startsWith("from=")) return { status: 200, body: wire.run };
  if (rest === "/result") return { status: 200, body: wire.result };
  // A room where nothing has been said, from the corpus's own page — so a cell
  // that is not about the chat has none of another room's messages in its DOM.
  if (rest === "/chat" && method === "GET")
    return { status: 200, body: { ...wire.chat, messages: [], last: 0 } };
  if (rest === "/input") return { status: 200, body: named(wire.options, game) };
  return undefined;
}

/** One instance of the fake. Cells read `posted` and call `emit`. */
export class FakeWorker {
  /** Every fake built since the last [`stubWorker`], newest last — what a
   *  restart is counted by. */
  static instances: FakeWorker[] = [];
  static hooks: FakeWorkerHooks = {};
  /** Every instance dies as soon as it is built (see the constructor). */
  static broken = false;
  /** `new Worker(…)` itself THROWS with this message ([`refuseToStart`]). */
  static refusal: string | null = null;

  readonly posted: Req[] = [];
  terminated = false;
  onmessage: ((e: MessageEvent<unknown>) => void) | null = null;
  onerror: ((e: ErrorEvent) => void) | null = null;
  onmessageerror: ((e: MessageEvent<unknown>) => void) | null = null;

  constructor(readonly url?: unknown) {
    // **Before anything is built**, as a browser refuses the constructor: no
    // instance exists to count, to post to or to terminate.
    if (FakeWorker.refusal !== null) throw new Error(FakeWorker.refusal);
    FakeWorker.instances.push(this);
    // **The module will not load** ([`breakTheWorker`]). Raised on a
    // microtask, because `client.ts`'s `start()` assigns `onerror` on the
    // statement after `new Worker(...)` — a synchronous raise would reach
    // nobody and the cell would hang instead of failing.
    if (FakeWorker.broken) queueMicrotask(() => this.raise("the module would not load"));
  }

  postMessage(req: Req): void {
    this.posted.push(req);
    const hooks = FakeWorker.hooks;
    let reply: Record<string, unknown>;
    switch (req.kind) {
      case "call": {
        const answer = hooks.call?.(req) ?? corpusReply(req);
        if (answer === "pending") return;
        if (answer === undefined) {
          // Loud, and at the CALL SITE: `ask` runs this inside its promise
          // executor, so the throw rejects the very call that asked.
          throw new Error(`the fake local host was not told about ${req.method} ${req.path}`);
        }
        // Told apart by the reply's own key, for `pair`'s reason below: the
        // refusal's key is spelled once, in `exportPair.ts`.
        if (!("status" in answer)) {
          reply = { ...answer };
          break;
        }
        reply = {
          status: answer.status,
          body: answer.body === undefined ? "" : JSON.stringify(answer.body),
          // **`saved` is always present, because the real Worker always sends
          // it** (`worker.ts:304-322`: every arm of the call path returns an
          // outcome). Omitting the key left `localCall` reading `undefined`,
          // which is not `"saved"` — so every corpus-driven local move in this
          // suite was reported to the page as APPLIED BUT NOT SAVED, and the
          // default state of the fake was a failure the real host cannot
          // produce. The field's own doc already said "omitted means saved";
          // this is what makes that true.
          saved: answer.saved ?? "saved",
          ...(answer.reason ? { reason: answer.reason } : {}),
          ...(answer.trapped ? { trapped: true } : {}),
        };
        break;
      }
      case "pair": {
        const both = hooks.pair?.(req.id);
        if (both === "pending") return;
        if (both === undefined) {
          // **The committed pair, by default** (PR #37 fix wave, R-F1). This
          // fake holds no game at any generation, and the real Worker's answer
          // for a game it does not own is what IndexedDB committed — so that is
          // what a cell that says nothing gets, rather than two nulls the page
          // would now take WHOLE as "no such game".
          const seq = req.seq;
          void committedPair(req.id).then((pair) => this.deliver({ seq, ...pair }));
          return;
        }
        // Told apart by the PAIR's key, so this fake never restates the
        // refusal's: that key is spelled once, in `exportPair.ts`.
        reply = "record" in both ? { record: both.record, table: both.table } : { ...both };
        break;
      }
      case "hold":
      case "takeover": {
        const held = hooks.hold?.(req.id, req.kind === "takeover") ?? true;
        if (held === "pending") return;
        reply = typeof held === "boolean" ? { held } : { ...held };
        break;
      }
      case "release":
        reply = { released: true };
        break;
      case "restore_read": {
        const answer = hooks.restore?.(req.envelope) ?? fakeVerdicts(req.envelope);
        // `{ unreadable }` is the Worker's startup failing to read the store
        // before the validator was ever asked (its `ready`), which is not an
        // answer about the FILE.
        reply = Array.isArray(answer)
          ? { report: JSON.stringify(answer) }
          : "refused" in answer
            ? { refused: answer.refused }
            : { ...answer };
        break;
      }
      case "adopt": {
        const adopted = hooks.adopt?.(req.ids) ?? req.ids.length;
        reply = typeof adopted === "number" ? { adopted } : { ...adopted };
        break;
      }
    }
    queueMicrotask(() => this.deliver({ seq: req.seq, ...reply }));
  }

  /** An unsolicited message — the lock this tab held was taken. */
  emit(message: unknown): void {
    this.deliver(message);
  }

  /** The Worker failed to start, or died. */
  raise(message: string): void {
    this.onerror?.({ message } as ErrorEvent);
  }

  terminate(): void {
    this.terminated = true;
  }

  private deliver(data: unknown): void {
    // A terminated Worker says nothing more, which is what makes the restart
    // cell's "the first instance's late reply reaches nobody" true.
    if (this.terminated) return;
    this.onmessage?.({ data } as MessageEvent<unknown>);
  }
}

/** Install the fake as the global `Worker`, and reset the local host's
 *  singleton so a cell does not inherit the last one's instance.
 *
 *  Returns the constructor, whose `instances` a cell reads. */
export function stubWorker(hooks: FakeWorkerHooks = {}): typeof FakeWorker {
  FakeWorker.instances = [];
  FakeWorker.hooks = hooks;
  FakeWorker.broken = false;
  FakeWorker.refusal = null;
  vi.stubGlobal("Worker", FakeWorker);
  return FakeWorker;
}

/** **The module will not load** — every Worker this page starts from now on
 *  dies at once, which is the state a backup has to work in (spec §2.5).
 *
 *  Called after [`stubWorker`], which clears the flag. */
export function breakTheWorker(): void {
  FakeWorker.broken = true;
}

/** **The browser will not even BUILD the Worker** — `new Worker(…)` throws
 *  `message` from now on, which is a `SecurityError` or a script the page may
 *  not load, and is not the same failure as [`breakTheWorker`]'s: that one
 *  builds and then dies, this one never exists (M6.5c2 cycle D, dossier row 56).
 *
 *  Called after [`stubWorker`], which clears it. */
export function refuseToStart(message: string): void {
  FakeWorker.refusal = message;
}

/** What the Worker's `pair` message answers for ONE game — standing for THIS
 *  tab's Worker's snapshot where it owns the game, which a backup PREFERS over
 *  the database because a move applied and not yet written is the very move a
 *  recovery is for. Without it the fake answers the committed pair, which is
 *  what the real Worker answers for a game it does not own (R-F1). */
export function workerAnswersWith(id: string, pair: FakePair): void {
  const previous = FakeWorker.hooks.pair;
  FakeWorker.hooks = {
    ...FakeWorker.hooks,
    pair: (asked) => (asked === id ? pair : previous?.(asked)),
  };
}

/** The newest fake built, which is the one the page is talking to. */
export const currentWorker = (): FakeWorker => {
  const w = FakeWorker.instances[FakeWorker.instances.length - 1];
  if (!w) throw new Error("no local host has been started");
  return w;
};
