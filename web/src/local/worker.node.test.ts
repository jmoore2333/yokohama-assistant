// **The local host's actor, RUN** (M6.5c2 cycle D, Task 13 fix round 1; the
// review's Important 2).
//
// `exportPair.ts` holds the rule the Worker answers a storage failure by
// (`fromStorage` names it, `failureReply` answers it) and its cells prove the
// RULE. They cannot prove the Worker CALLS it: no jsdom cell can load
// `worker.ts`, which imports the generated wasm glue, so every `fromStorage`
// wrap in it could be deleted with the whole suite green — the review's mutant
// 33a (the acquisition answering the trap) survived all 1279 cells, and the one
// e2e cell that reaches the Worker stages its refusal after the startup has
// run, so it reaches the generation read alone.
//
// **So this runs the real Worker**, `sw.node.test.ts`'s way: `src/local/worker.ts`
// bundled by esbuild, the ONE module it cannot run here — the wasm glue —
// replaced by a stand-in (below), and the result evaluated with fakes in the
// positions a dedicated Worker's globals occupy: `self` (its `onmessage` and
// `postMessage`), `indexedDB` and `IDBKeyRange` (`fake-indexeddb`, a real
// IndexedDB in memory), and `navigator` with no Web Locks — which is the
// Worker's own fallback path, the acquisition without a lock. `db.ts`,
// `exportPair.ts`, `ids.ts` and `lost.ts` are the real ones.
//
// **Each storage site the Worker wraps has a cell that stages its failure** and
// asserts the `{ unreadable }` answer, so removing any ONE wrap fails a named
// cell: the startup's open, the startup's read, a hold's generation, the
// acquisition's rebuild, and a restore's `adopt`. A refused transaction is the
// e2e's own staging ("The database connection is closing.", the shape a
// connection closed on `versionchange` has), narrowed here to the store and
// mode one call uses. The partners keep the rule from going too far: a module
// fault is still the trap, and a store that works still answers.
//
// **The module is a stand-in, and says so.** The layer's own behaviour is the
// corpus replay's claim (`corpus.node.test.ts`) and the browser walk's; what is
// tested here is the Worker around it, so the stand-in keeps documents in a map
// and answers every dispatch with an empty 200 — nothing here reads a payload.
//
// **In the NODE project** for `sw.node.test.ts`'s reason: esbuild will not run
// under jsdom.

import { IDBDatabase as FakeDatabase, IDBFactory, IDBKeyRange } from "fake-indexeddb";
import { afterAll, afterEach, beforeAll, describe, expect, it } from "vitest";
import { unreadableOf } from "./exportPair";

/** The wasm glue's surface as `worker.ts` uses it: the default export (the
 *  module's own async init) and five named exports. Each forwards to the
 *  harness's stand-in, which the bundle reaches as the free name `__module`. */
const GLUE = `
export default async function initModule() {}
export const init = () => __module.init();
export const load = (doc, id, text) => __module.load(doc, id, text);
export const read = (doc, id) => __module.read(doc, id);
export const dispatch = (...args) => __module.dispatch(...args);
export const restore_read = (envelope) => __module.restore_read(envelope);
`;

/** `src/local/worker.ts`, bundled once for the whole file, with the glue
 *  replaced. */
let bundled: Promise<string> | null = null;
function workerSource(): Promise<string> {
  bundled ??= (async () => {
    const { build } = await import("esbuild");
    const out = await build({
      entryPoints: [new URL("./worker.ts", import.meta.url).pathname],
      bundle: true,
      write: false,
      format: "iife",
      platform: "browser",
      target: "es2022",
      plugins: [
        {
          name: "the-module-stands-in",
          setup(b) {
            b.onResolve({ filter: /\/wasm\/yokohama_wasm\.js$/ }, () => ({ path: "glue", namespace: "stand-in" }));
            b.onLoad({ filter: /.*/, namespace: "stand-in" }, () => ({ contents: GLUE, loader: "js" }));
          },
        },
      ],
    });
    return out.outputFiles![0]!.text;
  })();
  return bundled;
}

/** The layer, standing in: documents in a map, `load` a create as the real
 *  one is, and `init` able to FAULT — the partner cell's module trap. */
class StandIn {
  readonly docs = new Map<string, string>();
  faultOnInit = false;
  init(): void {
    if (this.faultOnInit) throw new Error("RuntimeError: unreachable executed");
    this.docs.clear();
  }
  load(doc: string, id: string, text: string): void {
    const key = `${doc} ${id}`;
    if (this.docs.has(key)) throw new Error(`${key} is already loaded`);
    this.docs.set(key, text);
  }
  read(doc: string, id: string): string | undefined {
    return this.docs.get(`${doc} ${id}`);
  }
  dispatch(): { status: number; body: string; free(): void } {
    return { status: 200, body: "{}", free: () => undefined };
  }
  restore_read(): string {
    return "[]";
  }
}

/** Which transactions the store refuses right now — by store name and mode, so
 *  a cell can refuse the one call it is about and let the others through. */
let refuse: ((store: string, mode: string) => boolean) | null = null;
const CLOSING = "The database connection is closing.";
const realTransaction = FakeDatabase.prototype.transaction;

beforeAll(() => {
  FakeDatabase.prototype.transaction = function (
    this: InstanceType<typeof FakeDatabase>,
    ...args: Parameters<typeof realTransaction>
  ) {
    const [stores, mode = "readonly"] = args;
    const name = Array.isArray(stores) ? stores.join(",") : String(stores);
    if (refuse?.(name, mode)) throw new DOMException(CLOSING, "InvalidStateError");
    return realTransaction.apply(this, args);
  } as typeof realTransaction;
});
afterEach(() => {
  refuse = null;
});
afterAll(() => {
  FakeDatabase.prototype.transaction = realTransaction;
});

/** A factory whose every open FAILS, as a browser's does when it will not give
 *  this origin a database at all. */
const OPEN_REFUSED = "Internal error opening backing store for indexedDB.open.";
const refusingFactory = {
  open() {
    const request: { error: DOMException; onerror: (() => void) | null } = {
      error: new DOMException(OPEN_REFUSED, "UnknownError"),
      onerror: null,
    };
    setTimeout(() => request.onerror?.(), 0);
    return request;
  },
};

type Reply = Record<string, unknown> & { seq: number };

/** One Worker, evaluated fresh — its startup, its queue and its store are
 *  module state, so no cell may inherit another's. */
async function boot(factory: unknown = new IDBFactory()) {
  const code = await workerSource();
  const posted: Reply[] = [];
  const self: { postMessage: (m: Reply) => void; onmessage: ((e: { data: unknown }) => void) | null } = {
    postMessage: (m) => posted.push(m),
    onmessage: null,
  };
  const module = new StandIn();
  // eslint-disable-next-line @typescript-eslint/no-implied-eval
  new Function("self", "indexedDB", "IDBKeyRange", "navigator", "__module", code)(
    self,
    factory,
    IDBKeyRange,
    {},
    module,
  );
  let seq = 0;
  /** Post one request and wait for ITS reply — IndexedDB answers on later
   *  turns, so this yields to the event loop until the reply is there. */
  const ask = async (req: Record<string, unknown>): Promise<Reply> => {
    const mine = (seq += 1);
    self.onmessage!({ data: { seq: mine, ...req } });
    for (let i = 0; i < 5000; i += 1) {
      const reply = posted.find((p) => p.seq === mine);
      if (reply) return reply;
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    throw new Error(`the Worker never answered ${JSON.stringify(req)}`);
  };
  return { ask, module };
}

const health = { kind: "call", method: "GET", path: "/api/health", query: "", body: null };

/** The storage's refusal, by the one spelling (`exportPair.ts`), and never the
 *  trap. */
function expectUnreadable(reply: Reply, reason: string): void {
  expect(reply, "an ordinary reply, not the trap").not.toHaveProperty("trapped");
  expect(unreadableOf(reply)).toBe(`this browser's storage could not be read — ${reason}`);
}

describe("the local host's answer to a store it cannot read, at every site it reads one", () => {
  it("the startup's OPEN", async () => {
    const worker = await boot(refusingFactory);
    expectUnreadable(await worker.ask(health), OPEN_REFUSED);
  });

  it("the startup's READ of every stored pair", async () => {
    refuse = (store, mode) => store === "docs" && mode === "readonly";
    const worker = await boot();
    expectUnreadable(await worker.ask(health), CLOSING);
  });

  it("a hold's GENERATION", async () => {
    const worker = await boot();
    expect((await worker.ask(health)).status, "the startup read the store").toBe(200);
    refuse = (store) => store === "meta";
    expectUnreadable(await worker.ask({ kind: "hold", id: "local-1" }), CLOSING);
  });

  it("a hold's REBUILD, after its generation was taken", async () => {
    const worker = await boot();
    expect((await worker.ask(health)).status).toBe(200);
    refuse = (store, mode) => store === "docs" && mode === "readonly";
    expectUnreadable(await worker.ask({ kind: "hold", id: "local-1" }), CLOSING);
  });

  it("a restore's ADOPT of a pair it was told about", async () => {
    const worker = await boot();
    expect((await worker.ask(health)).status).toBe(200);
    refuse = (store, mode) => store === "docs" && mode === "readonly";
    expectUnreadable(await worker.ask({ kind: "adopt", ids: ["local-1"] }), CLOSING);
  });

  // ---- the partners ------------------------------------------------------

  it("still answers a MODULE fault as the trap, which the page recovers from", async () => {
    const worker = await boot();
    worker.module.faultOnInit = true;
    const reply = await worker.ask(health);
    expect(reply.trapped).toBe(true);
    expect(unreadableOf(reply)).toBeNull();
  });

  it("answers as it always did over a store that works", async () => {
    const worker = await boot();
    expect((await worker.ask(health)).status).toBe(200);
    expect(await worker.ask({ kind: "hold", id: "local-1" })).toMatchObject({ held: true });
    expect(await worker.ask({ kind: "adopt", ids: ["local-2"] })).toMatchObject({ adopted: 0 });
  });
});
