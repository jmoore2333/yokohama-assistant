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

/** What a cell may answer for itself. `undefined` from any of them falls
 *  through to the committed-corpus table below. */
export interface FakeWorkerHooks {
  /** `"pending"` answers NOTHING and leaves the call in flight — how a cell
   *  arranges a waiting request for the trap to settle. */
  call?: (req: Extract<Req, { kind: "call" }>) => CallReply | "pending" | undefined;
  /** `"pending"` leaves the ACQUISITION unanswered — how a cell arranges the
   *  window in which a Worker has been asked for a game and does not yet hold
   *  it. */
  hold?: (id: string, steal: boolean) => boolean | "pending";
  /** Both documents of one game, as the `pair` message answers them. */
  pair?: (id: string) => { record?: string | null; table?: string | null } | undefined;
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
  if (path === "/api/games" && method === "GET") {
    // The lobby row shape, re-addressed and with the corpus row's own NAME
    // cleared — the `gamesNamed` rule: a row is listed as `name ?? id`, so a
    // row that kept `wire` would be found by no cell that looks for its id.
    return {
      status: 200,
      body: { games: [{ ...named(wire.games.games[0]!, "local-1"), name: null }], next: null },
    };
  }
  if (path === "/api/games" && method === "POST") {
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

  readonly posted: Req[] = [];
  terminated = false;
  onmessage: ((e: MessageEvent<unknown>) => void) | null = null;
  onerror: ((e: ErrorEvent) => void) | null = null;
  onmessageerror: ((e: MessageEvent<unknown>) => void) | null = null;

  constructor(readonly url?: unknown) {
    FakeWorker.instances.push(this);
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
        reply = {
          status: answer.status,
          body: answer.body === undefined ? "" : JSON.stringify(answer.body),
          ...(answer.saved ? { saved: answer.saved } : {}),
          ...(answer.reason ? { reason: answer.reason } : {}),
          ...(answer.trapped ? { trapped: true } : {}),
        };
        break;
      }
      case "pair": {
        const both = hooks.pair?.(req.id) ?? {};
        reply = { record: both.record ?? null, table: both.table ?? null };
        break;
      }
      case "hold":
      case "takeover": {
        const held = hooks.hold?.(req.id, req.kind === "takeover") ?? true;
        if (held === "pending") return;
        reply = { held };
        break;
      }
      case "release":
        reply = { released: true };
        break;
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
  vi.stubGlobal("Worker", FakeWorker);
  return FakeWorker;
}

/** The newest fake built, which is the one the page is talking to. */
export const currentWorker = (): FakeWorker => {
  const w = FakeWorker.instances[FakeWorker.instances.length - 1];
  if (!w) throw new Error("no local host has been started");
  return w;
};
