// **The worker, RUN** (M6.5b2, spec §2.3 and §6.5).
//
// `swRoute.ts` and `swInstall.ts` hold the decisions and have cells of their
// own; `sw.ts` is the plumbing that drives them, and until this file nothing
// tested it. The review found the cost exactly: five of the eleven mutations
// spec §6.5 names for this task — the install made best-effort, `skipWaiting`
// on install, `cache: "reload"` dropped, the per-body digest CALL deleted, and
// `clients.claim()` added — all live in that one file, and every one of them
// left the whole suite green.
//
// **So this runs the real built worker.** It bundles `src/sw.ts` through the
// same esbuild path `scripts/precache.mjs` uses, with a three-entry release
// written in by `define`, and evaluates it with fakes in the five positions a
// service worker's globals occupy: `self`, `caches`, `fetch`, `crypto` and
// `Response`. Everything else — `URL`, `AbortController`, `setTimeout`,
// `Uint8Array` — is the real thing from this realm, so `instanceof` and the
// digest arithmetic are not simulated. `crypto` is node's WebCrypto, so the
// SHA-256 the install computes is a real one.
//
// A source-scanning test would have been the cheaper instrument and this
// repository uses that shape elsewhere (`platform_calls_stay_behind_the_server_feature`,
// `every_spec_declares_its_worker.spec.ts`). It is not what is here, because
// scanning for the text `clients.claim` proves nothing about whether the
// install ACTUALLY leaves the previous generation in place — and that is the
// claim the two-tab policy rests on.
//
// **It is in the NODE project, and that is forced twice over.** esbuild refuses
// to run under jsdom (`new TextEncoder().encode("") instanceof Uint8Array` is
// false across those realms, and esbuild checks); and importing `sw.ts` into
// the app project would pull `lib.webworker.d.ts` in through its own
// `/// <reference lib="webworker" />`, putting worker globals in scope for
// every component — which is the whole reason `tsconfig.sw.json` exists. So it
// runs under `npm run test:wasm`, which is a per-edit command and CI's
// `web-server` job, rather than under `npm test`.

import { createHash, webcrypto } from "node:crypto";
import { beforeEach, describe, expect, it, vi } from "vitest";

const CACHE_NAME = "yokohama-release-driventest01";
const META_PATH = "/__release__";

/** The three-entry release this worker is built with, and the bodies a server
 *  would answer with. */
const BODIES: Record<string, { body: string; type: string }> = {
  "/index.html": { body: "<!doctype html><title>Yokohama</title>", type: "text/html; charset=utf-8" },
  "/assets/app-abc.js": { body: "console.log('app')", type: "text/javascript; charset=utf-8" },
  "/art/orders/O01.webp": { body: "RIFF....WEBP", type: "image/webp" },
};
const ENTRIES = Object.entries(BODIES).map(([path, { body, type }]) => ({
  path,
  digest: createHash("sha256").update(body).digest("hex"),
  type: type.split(";")[0]!.trim(),
}));

/** `src/sw.ts`, bundled once for the whole file. */
let bundled: Promise<string> | null = null;
function workerSource(): Promise<string> {
  bundled ??= (async () => {
    const { build } = await import("esbuild");
    const out = await build({
      entryPoints: [new URL("./sw.ts", import.meta.url).pathname],
      bundle: true,
      write: false,
      format: "iife",
      platform: "browser",
      target: "es2022",
      define: {
        __CACHE_NAME__: JSON.stringify(CACHE_NAME),
        __PRECACHE__: JSON.stringify(ENTRIES),
      },
    });
    return out.outputFiles![0]!.text;
  })();
  return bundled;
}

class FakeCache {
  readonly entries = new Map<string, Response>();
  async put(key: string, value: Response): Promise<void> {
    this.entries.set(key, value);
  }
  async match(key: string): Promise<Response | undefined> {
    return this.entries.get(key);
  }
}

class FakeCaches {
  readonly generations = new Map<string, FakeCache>();
  /** Make the NEXT `open` fail, once — the transient the worker must recover
   *  from rather than remember. */
  failNextOpen = false;
  async open(name: string): Promise<FakeCache> {
    if (this.failNextOpen) {
      this.failNextOpen = false;
      throw new Error("QuotaExceededError: could not open the cache");
    }
    let cache = this.generations.get(name);
    if (!cache) {
      cache = new FakeCache();
      this.generations.set(name, cache);
    }
    return cache;
  }
  async keys(): Promise<string[]> {
    return [...this.generations.keys()];
  }
  async delete(name: string): Promise<boolean> {
    return this.generations.delete(name);
  }
}

interface Harness {
  caches: FakeCaches;
  skipWaiting: ReturnType<typeof vi.fn>;
  claim: ReturnType<typeof vi.fn>;
  posted: unknown[];
  /** Every install fetch, as (path, init). */
  asked: { path: string; init: RequestInit }[];
  install(): Promise<void>;
  activate(): Promise<void>;
  message(data: unknown): void;
  /** The response the worker answered with, or `null` when it did not
   *  intercept at all. */
  /** `method` defaults to GET. It is a parameter because the worker's own
   *  first line is `if (request.method !== "GET") return;` and a harness that
   *  hard-coded the verb could not drive it (whole-branch review, false
   *  sentence 7). */
  request(url: string, mode: string, method?: string): Promise<Response | null>;
}

/** Build the worker and wire it to fakes.
 *
 *  `answer` lets a cell make one path misbehave, which is how the all-or-nothing
 *  claims are arranged. `hold` leaves one path's fetch pending until its signal
 *  aborts — a real `fetch` rejects on abort, and a fake that did not would make
 *  a hung lane look like a healthy one. */
async function boot(
  answer: (path: string) => { status: number; body: string; type: string } = (path) => ({
    status: 200,
    body: BODIES[path]!.body,
    type: BODIES[path]!.type,
  }),
  hold?: string,
): Promise<Harness> {
  const code = await workerSource();
  const handlers = new Map<string, (event: never) => void>();
  const harness = {
    caches: new FakeCaches(),
    skipWaiting: vi.fn(),
    claim: vi.fn(),
    posted: [] as unknown[],
    asked: [] as { path: string; init: RequestInit }[],
  };

  const self = {
    addEventListener: (type: string, handler: (event: never) => void) => handlers.set(type, handler),
    skipWaiting: harness.skipWaiting,
    location: { origin: "https://yokohama.test" },
    clients: {
      claim: harness.claim,
      matchAll: async () => [
        { postMessage: (message: unknown) => harness.posted.push(message) },
      ],
    },
  };

  const fetchFake = vi.fn(async (input: string | { url: string }, init: RequestInit = {}) => {
    // `installOne` passes a path; `fromCache`'s fall-through passes the whole
    // Request, which is the shape a real `fetch` accepts too.
    const path = typeof input === "string" ? input : new URL(input.url).pathname;
    harness.asked.push({ path, init });
    if (path === hold) {
      // Pending until the signal aborts, which is what a real `fetch` does.
      return new Promise<Response>((_resolve, reject) => {
        const signal = init.signal as AbortSignal | undefined;
        signal?.addEventListener("abort", () => {
          const error = new Error("aborted");
          error.name = "AbortError";
          reject(error);
        });
      });
    }
    const said = answer(path);
    return new Response(said.body, {
      status: said.status,
      headers: { "content-type": said.type },
    });
  });

  // eslint-disable-next-line @typescript-eslint/no-implied-eval
  const run = new Function("self", "caches", "fetch", "crypto", "Response", code) as (
    ...args: unknown[]
  ) => void;
  run(self, harness.caches, fetchFake, webcrypto, Response);

  const drive = async (type: "install" | "activate") => {
    let work: Promise<unknown> = Promise.resolve();
    const handler = handlers.get(type)!;
    handler({ waitUntil: (p: Promise<unknown>) => (work = p) } as never);
    await work;
  };

  return {
    ...harness,
    install: () => drive("install"),
    activate: () => drive("activate"),
    message: (data) => handlers.get("message")!({ data } as never),
    request: async (url, mode, method = "GET") => {
      let answered: Promise<Response> | null = null;
      handlers.get("fetch")!({
        request: { url, mode, method },
        respondWith: (p: Promise<Response>) => (answered = p),
      } as never);
      return answered === null ? null : await answered;
    },
  };
}

describe("the worker's install", () => {
  beforeEach(() => vi.restoreAllMocks());

  it("caches every entry and stamps the generation", async () => {
    const worker = await boot();
    await worker.install();
    const cache = worker.caches.generations.get(CACHE_NAME)!;
    expect([...cache.entries.keys()].sort()).toEqual(
      [...ENTRIES.map((e) => e.path), META_PATH].sort(),
    );
    expect(await (await cache.match("/index.html"))!.text()).toBe(BODIES["/index.html"]!.body);
  });

  // **spec §6.5: "the install's all-or-nothing turned into best-effort".** One
  // bad file fails the WHOLE install and leaves the previous release in place;
  // swallow the rethrow and this cell sees an install that resolved.
  it("fails whole on one bad answer and leaves no generation behind", async () => {
    const worker = await boot((path) =>
      path === "/art/orders/O01.webp"
        ? { status: 404, body: "no such art", type: "text/plain" }
        : { status: 200, body: BODIES[path]!.body, type: BODIES[path]!.type },
    );
    await expect(worker.install()).rejects.toThrow(/answered 404/);
    expect(worker.caches.generations.has(CACHE_NAME)).toBe(false);
    // And it SAID why, to the page that is watching.
    expect(worker.posted).toEqual([
      { yokohama: "install-failed", reason: expect.stringContaining("answered 404") },
    ]);
  });

  // **spec §6.5: "the install's per-body digest check dropped".** Not the
  // function being made permissive — `swInstall.test.ts` has that — but the
  // CALL being deleted. A 200 of the right type whose bytes are someone else's
  // must fail.
  // **The other lanes are stopped and WAITED FOR before anything is deleted.**
  // `Promise.all` rejects on the first failure and leaves its siblings running:
  // without the abort they go on downloading a release that has already failed,
  // and without the wait the delete could land while one of them is still
  // inside `cache.put`. The held entry's own signal is the witness — a request
  // that is still in flight when the install gives up must not be.
  it("aborts the entries still in flight when one fails", async () => {
    const worker = await boot(
      (path) =>
        path === "/index.html"
          ? { status: 500, body: "down", type: "text/plain" }
          : { status: 200, body: BODIES[path]!.body, type: BODIES[path]!.type },
      "/art/orders/O01.webp",
    );
    await expect(worker.install()).rejects.toThrow(/answered 500/);
    const held = worker.asked.find((a) => a.path === "/art/orders/O01.webp");
    expect(held, "the held entry was never even asked for").toBeDefined();
    expect(
      (held!.init.signal as AbortSignal).aborted,
      "an entry still in flight when the install failed was left downloading",
    ).toBe(true);
    expect(worker.caches.generations.has(CACHE_NAME)).toBe(false);
  });

  it("refuses a body that is a valid 200 of the right type and the wrong bytes", async () => {
    const worker = await boot((path) =>
      path === "/assets/app-abc.js"
        ? { status: 200, body: "console.log('a different release')", type: "text/javascript" }
        : { status: 200, body: BODIES[path]!.body, type: BODIES[path]!.type },
    );
    await expect(worker.install()).rejects.toThrow(/not the release's/);
    expect(worker.caches.generations.has(CACHE_NAME)).toBe(false);
  });

  // **spec §6.5: "`cache: \"reload\"` dropped".** It is for the art, whose URLs
  // are stable and which the server serves `max-age=3600`: without it an
  // install minutes after a re-cut freezes the hour-old picture under the new
  // release's name. `redirect: "error"` rides beside it, for a login page
  // answering 200.
  it("asks past the HTTP cache and refuses a redirect, for every entry", async () => {
    const worker = await boot();
    await worker.install();
    expect(worker.asked.map((a) => a.path).sort()).toEqual(ENTRIES.map((e) => e.path).sort());
    for (const { path, init } of worker.asked) {
      expect(init.cache, `${path} must be fetched past the HTTP cache`).toBe("reload");
      expect(init.redirect, `${path} must not follow a redirect`).toBe("error");
    }
  });

  // **spec §6.5: "`skipWaiting` called on install".** It is registration-wide,
  // so a worker that skipped here could put itself in front of a page still
  // running the OLD bundle — mid-game. The only caller is the message the
  // player's click sends.
  it("never skips waiting by itself, and skips exactly when told to", async () => {
    const worker = await boot();
    await worker.install();
    await worker.activate();
    expect(worker.skipWaiting).not.toHaveBeenCalled();
    worker.message({ yokohama: "skip-waiting" });
    expect(worker.skipWaiting).toHaveBeenCalledTimes(1);
    // And not for any other message.
    worker.message({ yokohama: "something-else" });
    worker.message(null);
    expect(worker.skipWaiting).toHaveBeenCalledTimes(1);
  });

  // **spec §6.5: "`clients.claim()` added".** Claiming would put this worker in
  // front of pages still running the old bundle — the exact mixture the matched
  // cache generation exists to prevent.
  it("never claims a client", async () => {
    const worker = await boot();
    await worker.install();
    await worker.activate();
    worker.message({ yokohama: "skip-waiting" });
    expect(worker.claim).not.toHaveBeenCalled();
  });
});

describe("the worker's activate", () => {
  // The retention rule, driven through `sweep()` itself rather than through
  // `generationsToDelete` — which is where the review noted a mutation could
  // otherwise hide.
  it("keeps itself and the previous generation and deletes what is older", async () => {
    const worker = await boot();
    const seed = async (name: string, createdAt: number) => {
      const cache = await worker.caches.open(name);
      await cache.put(META_PATH, new Response(JSON.stringify({ createdAt })));
    };
    await seed("yokohama-release-previous", 200);
    await seed("yokohama-release-older", 100);
    await seed("yokohama-release-oldest", 50);
    await seed("someone-elses-cache", 1);
    await worker.install();
    await worker.activate();
    expect((await worker.caches.keys()).sort()).toEqual(
      [CACHE_NAME, "someone-elses-cache", "yokohama-release-previous"].sort(),
    );
  });
});

describe("the worker's fetch", () => {
  // **A cache that fails to open once must not be remembered as failed.**
  // `opened ??= caches.open(...)` alone holds the REJECTED promise for the life
  // of the worker, so one transient failure turns every later navigation into a
  // network fetch until the worker is replaced — which offline is a broken page
  // rather than a slow one. The fetch COUNT is the witness: a cache hit costs
  // none, the fall-through costs one, and recovery costs none again.
  it("recovers from a cache that would not open", async () => {
    const worker = await boot();
    await worker.install();
    const url = "https://yokohama.test/assets/app-abc.js";
    const installed = worker.asked.length;

    // **The FIRST open is the one that matters**, because the handle is
    // memoized: a later failure never happens, and a rejected first one is what
    // `??=` alone would remember for the life of the worker.
    worker.caches.failNextOpen = true;
    await worker.request(url, "cors");
    expect(worker.asked.length, "a cache that will not open falls through").toBe(installed + 1);

    await worker.request(url, "cors");
    expect(worker.asked.length, "and the NEXT request is served from the cache again").toBe(
      installed + 1,
    );
    await worker.request(url, "cors");
    expect(worker.asked.length, "and stays served from it").toBe(installed + 1);
  });

  it("answers a precached path from the cache and a navigation with the shell", async () => {
    const worker = await boot();
    await worker.install();
    expect(await (await worker.request("https://yokohama.test/assets/app-abc.js", "cors"))!.text())
      .toBe(BODIES["/assets/app-abc.js"]!.body);
    expect(await (await worker.request("https://yokohama.test/t/anything", "navigate"))!.text())
      .toBe(BODIES["/index.html"]!.body);
  });

  // Not intercepted AT ALL: no `respondWith`, so the browser makes exactly the
  // request it would have made with no worker installed.
  it("does not touch the API, the turnstile, another origin or a POST", async () => {
    const worker = await boot();
    await worker.install();
    expect(await worker.request("https://yokohama.test/api/health", "cors")).toBeNull();
    expect(await worker.request("https://yokohama.test/enter", "navigate")).toBeNull();
    expect(await worker.request("https://elsewhere.test/index.html", "navigate")).toBeNull();
    expect(await worker.request("https://yokohama.test/unknown.json", "cors")).toBeNull();
    // **The POST, which the name claimed and no arm drove** (whole-branch
    // review, false sentence 7). It is asked of a path the release DOES carry,
    // so the only thing that can be answering `null` is the verb —
    // `sw.ts`'s own first line — and not the route.
    expect(await worker.request("https://yokohama.test/index.html", "cors", "POST")).toBeNull();
    // Its anti-vacuity partner: the same path, as a GET, is served.
    expect(
      await (await worker.request("https://yokohama.test/index.html", "cors"))!.text(),
    ).toBe(BODIES["/index.html"]!.body);
  });
});
