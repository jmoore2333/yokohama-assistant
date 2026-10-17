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
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { FETCH_TIMEOUT_MS, INSTALL_DEADLINE_MS } from "./swInstall";

const CACHE_NAME = "yokohama-release-driventest01";
const META_PATH = "/__release__";

/** The three-entry release this worker is built with, and the bodies a server
 *  would answer with. The shell is `/shell.bin`, `index.html`'s bytes under a
 *  non-HTML name, typed as both hosts serve it (R-6.5c2.41): the release lists
 *  no HTML, and no `/index.html`. */
const BODIES: Record<string, { body: string; type: string }> = {
  "/shell.bin": { body: "<!doctype html><title>Yokohama</title>", type: "application/octet-stream" },
  "/assets/app-abc.js": { body: "console.log('app')", type: "text/javascript; charset=utf-8" },
  "/art/orders/O01.webp": { body: "RIFF....WEBP", type: "image/webp" },
};
/** What the fake origin answers for a path it does not have. */
const NOT_ON_THE_ORIGIN = "not on this origin";
const ENTRIES = Object.entries(BODIES).map(([path, { body, type }]) => ({
  path,
  digest: createHash("sha256").update(body).digest("hex"),
  type: type.split(";")[0]!.trim(),
}));

type Entry = { path: string; digest: string; type: string };

/** `src/sw.ts`, bundled once per release list for the whole file. */
const bundled = new Map<string, Promise<string>>();
function workerSource(entries: readonly Entry[] = ENTRIES): Promise<string> {
  const key = JSON.stringify(entries);
  if (!bundled.has(key)) {
    bundled.set(
      key,
      (async () => {
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
            __PRECACHE__: JSON.stringify(entries),
          },
        });
        return out.outputFiles![0]!.text;
      })(),
    );
  }
  return bundled.get(key)!;
}

class FakeCache {
  readonly entries = new Map<string, Response>();
  /** `puts` is the owning `FakeCaches`' log, so a write anywhere is visible in
   *  one place — which is what "nothing is written at fetch time" is read off. */
  constructor(
    private readonly name: string,
    private readonly puts: string[],
  ) {}
  async put(key: string, value: Response): Promise<void> {
    this.puts.push(`${this.name} ${key}`);
    this.entries.set(key, value);
  }
  /** A CLONE, as a real cache hands back a fresh `Response` per match — so a
   *  cell may read the same entry twice without meeting a used body. */
  async match(key: string): Promise<Response | undefined> {
    return this.entries.get(key)?.clone();
  }
}

class FakeCaches {
  readonly generations = new Map<string, FakeCache>();
  /** Every `put` into any generation, as "<cache> <path>", in order. */
  readonly puts: string[] = [];
  /** Make the NEXT `open` fail, once — the transient the worker must recover
   *  from rather than remember. */
  failNextOpen = false;
  /** A generation whose `match` REJECTS — a storage fault in one cache, which
   *  must not keep the worker from reading the next one. */
  failMatchIn: string | null = null;
  async open(name: string): Promise<FakeCache> {
    if (this.failNextOpen) {
      this.failNextOpen = false;
      throw new Error("QuotaExceededError: could not open the cache");
    }
    let cache = this.generations.get(name);
    if (!cache) {
      cache = new FakeCache(name, this.puts);
      this.generations.set(name, cache);
    }
    return cache;
  }
  /** `CacheStorage.match` as the platform defines it: with a `cacheName`, that
   *  one cache and `undefined` where it does not exist — never creating it —
   *  and without one, every cache in the order `keys()` lists them. The second
   *  arm is here so a worker that dropped the name would be READING the other
   *  caches in this fake, which is the mutation the prefix cell exists for. */
  async match(key: string, options: { cacheName?: string } = {}): Promise<Response | undefined> {
    if (options.cacheName !== undefined && options.cacheName === this.failMatchIn) {
      throw new Error(`UnknownError: ${options.cacheName} could not be read`);
    }
    if (options.cacheName !== undefined) return this.generations.get(options.cacheName)?.match(key);
    for (const cache of this.generations.values()) {
      const hit = await cache.match(key);
      if (hit) return hit;
    }
    return undefined;
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
  /** Every fetch the worker makes, at install or at fetch time, as (path, init). */
  asked: { path: string; init: RequestInit }[];
  /** The options of every `clients.matchAll` the worker made. */
  matched: unknown[];
  install(): Promise<void>;
  activate(): Promise<void>;
  message(data: unknown): void;
  /** The response the worker answered with, or `null` when it did not
   *  intercept at all. */
  /** `method` defaults to GET. It is a parameter because the worker's own
   *  first line is `if (request.method !== "GET") return;` and a harness that
   *  hard-coded the verb could not drive it (whole-branch review, false
   *  sentence 7). `cache` defaults to `"default"`, and is a parameter for the
   *  same reason: the `only-if-cached` guard reads it. */
  request(url: string, mode: string, method?: string, cache?: string): Promise<Response | null>;
}

/** Build the worker and wire it to fakes.
 *
 *  `answer` lets a cell make one path misbehave, which is how the all-or-nothing
 *  claims are arranged. `hold` leaves one path's fetch pending until its signal
 *  aborts — a real `fetch` rejects on abort, and a fake that did not would make
 *  a hung lane look like a healthy one. */
async function boot(
  answer: (path: string) => { status: number; body: string; type: string; headers?: Record<string, string> } = (path) => {
    const known = BODIES[path];
    // A path the origin does not have answers as the origin does after a
    // release dropped the file: a 404, which is what makes "answered from the
    // network" readable in a cell.
    return known
      ? { status: 200, body: known.body, type: known.type }
      : { status: 404, body: NOT_ON_THE_ORIGIN, type: "text/plain" };
  },
  hold?: string,
  /** `entries`: a different release list (the deadline cell needs more than a
   *  lane's worth). `delayMs`: every answer arrives that long after it is
   *  asked, on the clock the cell has faked, and is refused on abort. */
  options: { entries?: readonly Entry[]; delayMs?: number } = {},
): Promise<Harness> {
  const code = await workerSource(options.entries);
  const handlers = new Map<string, (event: never) => void>();
  const harness = {
    caches: new FakeCaches(),
    skipWaiting: vi.fn(),
    claim: vi.fn(),
    posted: [] as unknown[],
    asked: [] as { path: string; init: RequestInit }[],
    matched: [] as unknown[],
  };

  const self = {
    addEventListener: (type: string, handler: (event: never) => void) => handlers.set(type, handler),
    skipWaiting: harness.skipWaiting,
    location: { origin: "https://yokohama.test" },
    clients: {
      claim: harness.claim,
      matchAll: async (matchOptions?: unknown) => {
        harness.matched.push(matchOptions);
        return [{ postMessage: (message: unknown) => harness.posted.push(message) }];
      },
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
    if (options.delayMs !== undefined) {
      await new Promise<void>((resolve, reject) => {
        const signal = init.signal as AbortSignal | undefined;
        const timer = setTimeout(resolve, options.delayMs);
        signal?.addEventListener("abort", () => {
          clearTimeout(timer);
          const error = new Error("aborted");
          error.name = "AbortError";
          reject(error);
        });
      });
    }
    const said = answer(path);
    return new Response(said.body, {
      status: said.status,
      headers: { "content-type": said.type, ...(said.headers ?? {}) },
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
    request: async (url, mode, method = "GET", cache = "default") => {
      let answered: Promise<Response> | null = null;
      handlers.get("fetch")!({
        request: { url, mode, method, cache },
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
    expect(await (await cache.match("/shell.bin"))!.text()).toBe(BODIES["/shell.bin"]!.body);
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
        path === "/shell.bin"
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
  // front of pages NO worker served — the first visit among them, whose bundle
  // came from the network rather than from this release's cache. (The pages the
  // previous worker controlled are handed to this one by `skipWaiting()`
  // itself, claim or no claim; `sw.ts`'s activate comment says so.)
  it("never claims a client", async () => {
    const worker = await boot();
    await worker.install();
    await worker.activate();
    worker.message({ yokohama: "skip-waiting" });
    expect(worker.claim).not.toHaveBeenCalled();
  });
});

// **The install's clocks and its small decisions** (M6.5c2 cycle D; `sw.ts`'s
// header listed these as "pinned by NO cell" — each is a mutation every cell
// survived). Fake timers for the CLOCK only (`setTimeout` and `Date`), so the
// digest and the cache writes still run for real and `setImmediate` is left
// alone. Every clock cell asserts BEFORE it moves the clock too: a deadline is
// read at its edge, not after it.
describe("the worker's install: its clocks and its small decisions", () => {
  const realSetTimeout = globalThis.setTimeout;
  const sleep = (ms: number) => new Promise<void>((r) => realSetTimeout(r, ms));
  beforeEach(() => vi.restoreAllMocks());
  afterEach(() => vi.useRealTimers());

  /** Start an install and report whether it has settled, and how. */
  const start = (worker: Harness) => {
    const state: { settled: boolean; outcome: "resolved" | { error: Error } | null } = {
      settled: false,
      outcome: null,
    };
    const done = worker.install().then(
      () => {
        state.settled = true;
        state.outcome = "resolved";
      },
      (error: Error) => {
        state.settled = true;
        state.outcome = { error };
      },
    );
    return { state, done };
  };
  /** Wait, in REAL time, up to `ms` for the install to settle. */
  const settles = async (state: { settled: boolean }, ms = 1_000) => {
    for (let waited = 0; waited < ms && !state.settled; waited += 5) await sleep(5);
    return state.settled;
  };
  /** Give real async work (digests, cache writes) a moment to run. */
  const quiet = () => sleep(60);

  it("aborts the entries in flight the moment one fails, and does not wait for their own timeout", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date"] });
    const worker = await boot(
      (path) =>
        path === "/shell.bin"
          ? { status: 500, body: "down", type: "text/plain" }
          : { status: 200, body: BODIES[path]!.body, type: BODIES[path]!.type },
      "/art/orders/O01.webp",
    );
    const { state } = start(worker);
    // **No clock is moved.** The per-fetch timeout is fifteen seconds off, so
    // an install that settles here was ended by the ABORT alone; a worker that
    // dropped the abort would sit on the held lane until that timer.
    expect(await settles(state), "the install did not end without the clock moving").toBe(true);
    expect(state.outcome).toMatchObject({ error: expect.objectContaining({ message: expect.stringMatching(/answered 500/) }) });
    expect(worker.caches.generations.has(CACHE_NAME), "and the failed generation is deleted after the wait").toBe(false);
  });

  it("times one fetch out at the per-fetch bound, naming the file and saying it took too long", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date"] });
    const worker = await boot(undefined, "/art/orders/O01.webp");
    const { state } = start(worker);
    await quiet();
    expect(state.settled, "one fetch is held, so the install is still going").toBe(false);
    await vi.advanceTimersByTimeAsync(FETCH_TIMEOUT_MS - 1);
    await quiet();
    expect(state.settled, "one millisecond inside the bound").toBe(false);
    await vi.advanceTimersByTimeAsync(1);
    expect(await settles(state), "and at the bound it ends").toBe(true);
    const message = (state.outcome as { error: Error }).error.message;
    expect(message).toBe("/art/orders/O01.webp: took too long");
    expect(message, "the FETCH's clock ended it, not the install's").not.toMatch(/did not finish/);
    // And the page that is watching is told the same words.
    expect(worker.posted).toEqual([
      { yokohama: "install-failed", reason: "/art/orders/O01.webp: took too long" },
    ]);
  });

  it("ends the whole install at its deadline, though no single fetch was late", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout", "Date"] });
    // Fifty-four entries on six lanes: nine rounds, each answered 14 s after it
    // is asked, one second inside the per-fetch bound — 126 s of install in all,
    // past the two-minute deadline that the last round is inside.
    const body = BODIES["/art/orders/O01.webp"]!;
    const many: Entry[] = Array.from({ length: 54 }, (_, i) => ({
      path: `/art/many/${i}.webp`,
      digest: createHash("sha256").update(body.body).digest("hex"),
      type: "image/webp",
    }));
    const worker = await boot(() => ({ status: 200, body: body.body, type: body.type }), undefined, {
      entries: many,
      delayMs: 14_000,
    });
    const { state } = start(worker);
    let elapsed = 0;
    const step = async () => {
      await quiet();
      await vi.advanceTimersByTimeAsync(14_000);
      elapsed += 14_000;
    };
    for (let round = 0; round < 8; round += 1) await step();
    await quiet();
    expect(elapsed, "the clock is inside the deadline").toBeLessThan(INSTALL_DEADLINE_MS);
    expect(state.settled, "eight rounds in, nothing has failed").toBe(false);
    for (let round = 0; round < 3 && !state.settled; round += 1) await step();
    expect(await settles(state), "past the deadline the install ends").toBe(true);
    const message = (state.outcome as { error: Error }).error.message;
    expect(message).toBe(`the install did not finish inside ${INSTALL_DEADLINE_MS} ms`);
    expect(worker.caches.generations.has(CACHE_NAME)).toBe(false);
  });

  it("fetches every entry with same-origin credentials", async () => {
    const worker = await boot();
    await worker.install();
    expect(worker.asked.length).toBe(ENTRIES.length);
    for (const { path, init } of worker.asked) {
      expect(init.credentials, `${path} must send the site's own cookies and no one else's`).toBe("same-origin");
    }
  });

  it("writes the generation's stamp before any entry", async () => {
    const worker = await boot();
    await worker.install();
    expect(worker.caches.puts[0], "the first write into the cache is the stamp").toBe(`${CACHE_NAME} ${META_PATH}`);
    expect(worker.caches.puts.length, "and the entries follow it").toBe(ENTRIES.length + 1);
  });

  it("tells uncontrolled pages too when the install fails", async () => {
    const worker = await boot((path) =>
      path === "/art/orders/O01.webp"
        ? { status: 404, body: "no such art", type: "text/plain" }
        : { status: 200, body: BODIES[path]!.body, type: BODIES[path]!.type },
    );
    await expect(worker.install()).rejects.toThrow(/answered 404/);
    expect(worker.matched, "asked once, for windows, uncontrolled ones included").toEqual([
      { includeUncontrolled: true, type: "window" },
    ]);
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

  /** One generation's stamp, as the worker left it. */
  const stampOf = async (worker: Harness, name: string) =>
    (await (await worker.caches.generations.get(name)!.match(META_PATH))!.json()) as {
      createdAt?: number;
      activatedAt?: number;
    };

  // **R-F5, the half that WRITES** (PR #37): `activate` stamps its own
  // generation with when it activated, and keeps the install time it found —
  // the next release's sweep ranks by both.
  it("stamps its own generation activatedAt and keeps its createdAt", async () => {
    const worker = await boot();
    await worker.install();
    const installed = await stampOf(worker, CACHE_NAME);
    expect(installed.activatedAt, "installed, not yet activated").toBeUndefined();
    await worker.activate();
    const activated = await stampOf(worker, CACHE_NAME);
    expect(activated.createdAt, "the install time is kept").toBe(installed.createdAt);
    expect(typeof activated.activatedAt).toBe("number");
    expect(activated.activatedAt!).toBeGreaterThanOrEqual(installed.createdAt!);
  });

  // **R-F5, the half that READS, through `sweep()` itself.** N was active; N+1
  // installed and waited and was replaced before anyone clicked, so it carries
  // the NEWEST install stamp and no activation. This release's sweep keeps N —
  // the generation the open tabs are running — and deletes N+1.
  it("keeps the generation activated last, not the one installed last", async () => {
    const worker = await boot();
    const seed = async (name: string, stamp: { createdAt: number; activatedAt?: number }) => {
      const cache = await worker.caches.open(name);
      await cache.put(META_PATH, new Response(JSON.stringify(stamp)));
    };
    await seed("yokohama-release-n", { createdAt: 200, activatedAt: 250 });
    await seed("yokohama-release-n-plus-1", { createdAt: 300 });
    await seed("yokohama-release-n-minus-1", { createdAt: 100, activatedAt: 150 });
    await worker.install();
    await worker.activate();
    expect((await worker.caches.keys()).sort()).toEqual([CACHE_NAME, "yokohama-release-n"].sort());
  });

  // **`markActivated` writes nothing where the install's stamp is absent**
  // (PR #37 F2 re-review, N-N3). A generation evicted between install and
  // activate must stay gone: `caches.open` would bring an empty one back into
  // being, and the next release's sweep would keep it as "previous".
  it("does not bring an evicted generation back into being when it activates", async () => {
    const worker = await boot();
    await worker.install();
    await worker.caches.delete(CACHE_NAME);
    await worker.activate();
    expect(await worker.caches.keys(), "the evicted generation stays gone").not.toContain(CACHE_NAME);
  });

  // **A failed install deletes only a generation it CREATED.** The cache name is
  // a digest over the list, so a code-only change to `sw.ts` installs under the
  // name the running worker serves from; a failure there that deleted the
  // generation would take out every open tab's release.
  it("leaves the running generation in place when a re-install of it fails", async () => {
    const worker = await boot((path) =>
      path === "/art/orders/O01.webp"
        ? { status: 404, body: "no such art", type: "text/plain" }
        : { status: 200, body: BODIES[path]!.body, type: BODIES[path]!.type },
    );
    const running = await worker.caches.open(CACHE_NAME);
    await running.put(META_PATH, new Response(JSON.stringify({ createdAt: 5, activatedAt: 7 })));
    await running.put("/shell.bin", new Response(BODIES["/shell.bin"]!.body));
    await expect(worker.install()).rejects.toThrow(/answered 404/);
    expect(await worker.caches.keys(), "the generation the running worker serves is still there").toContain(
      CACHE_NAME,
    );
  });

  // **A re-install of the SAME generation keeps its activation.** A code-only
  // change to `sw.ts` ships a new worker under the same cache name; the install
  // rewrites the stamp, and a stamp rewritten without `activatedAt` would rank
  // the running generation below an older activated one — so a release that
  // replaced this waiting worker would sweep the bytes every open tab runs.
  it("keeps the activation through a re-install of the same generation", async () => {
    const worker = await boot();
    const same = await worker.caches.open(CACHE_NAME);
    await same.put(META_PATH, new Response(JSON.stringify({ createdAt: 5, activatedAt: 7 })));
    await worker.install();
    const stamp = await stampOf(worker, CACHE_NAME);
    expect(stamp.activatedAt, "the running generation's activation survived").toBe(7);
    expect(stamp.createdAt, "and the install time is this install's").toBeGreaterThan(5);
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

  it("answers a precached path from the cache", async () => {
    const worker = await boot();
    await worker.install();
    expect(await (await worker.request("https://yokohama.test/assets/app-abc.js", "cors"))!.text())
      .toBe(BODIES["/assets/app-abc.js"]!.body);
  });

  // **R-6.5c2.41** (spec §10.11): the shell is precached from `/shell.bin`,
  // which the edge does not rewrite and both hosts serve as
  // `application/octet-stream` — so the NAVIGATION's HTML type is the
  // worker's own, set over the cached bytes, and the network is not asked. The
  // catcher for the navigation served with the cached entry's own type: the
  // `/shell.bin` read below is that type, and a browser handed it would
  // download the page rather than render it.
  it("answers a navigation from /shell.bin's bytes with the HTML type it sets itself", async () => {
    // The premise: the release this worker is built with lists the shell as
    // `/shell.bin`, typed as the hosts serve it, and lists no `/index.html`.
    expect(ENTRIES.find((e) => e.path === "/shell.bin")?.type).toBe("application/octet-stream");
    expect(ENTRIES.map((e) => e.path)).not.toContain("/index.html");
    const worker = await boot();
    await worker.install();
    const installed = worker.asked.length;
    const page = (await worker.request("https://yokohama.test/t/anything", "navigate"))!;
    expect(page.status).toBe(200);
    expect(page.headers.get("content-type")).toBe("text/html; charset=utf-8");
    expect(await page.text()).toBe(BODIES["/shell.bin"]!.body);
    expect(worker.asked.length, "the navigation asked the network nothing").toBe(installed);
    // Anti-vacuity: the cached entry keeps the SERVER's type, so the HTML type
    // above can only be the worker's.
    const stored = (await worker.request("https://yokohama.test/shell.bin", "cors"))!;
    expect(stored.headers.get("content-type")).toBe("application/octet-stream");
    expect(await stored.text()).toBe(BODIES["/shell.bin"]!.body);
    expect(worker.asked.length, "and neither did the stored entry's read").toBe(installed);
  });

  // **D13 (security S2a; decision D-5): a navigation the worker synthesizes carries the headers the host set on
  // `/shell.bin`** — the policy, `X-Frame-Options` and the rest — else every header the host sends protects a
  // first visit only, and the service worker undoes B4 on every returning visit a SAME-site page frames (a
  // cross-site framer meets the network's `X-Frame-Options` in default Chrome, which partitions the worker away
  // from it). Three are NOT carried: the type, which is the worker's HTML; and `content-length` and
  // `content-encoding`, which describe the ENCODED bytes the server sent, where the stored body is the decoded
  // bytes the install digested. A header no cell names is carried too: the pass-through is not an allow-list.
  it("carries the stored shell's headers onto a navigation, the HTML type over them, and not the three the bytes no longer match", async () => {
    const worker = await boot((path) => {
      const known = BODIES[path];
      if (!known) return { status: 404, body: NOT_ON_THE_ORIGIN, type: "text/plain" };
      const headers: Record<string, string> =
        path === "/shell.bin"
          ? {
              "x-frame-options": "DENY",
              "content-security-policy-report-only": "default-src 'self'",
              "cache-control": "no-store",
              "x-a-header-the-set-has-not-got-yet": "kept",
              "content-encoding": "br",
              "content-length": "9999",
            }
          : {};
      return { status: 200, body: known.body, type: known.type, headers };
    });
    await worker.install();
    const page = (await worker.request("https://yokohama.test/t/anything", "navigate"))!;
    expect(page.status).toBe(200);
    expect(page.headers.get("content-type")).toBe("text/html; charset=utf-8");
    expect(page.headers.get("x-frame-options")).toBe("DENY");
    expect(page.headers.get("content-security-policy-report-only")).toBe("default-src 'self'");
    expect(page.headers.get("cache-control")).toBe("no-store");
    expect(page.headers.get("x-a-header-the-set-has-not-got-yet"), "carried although no cell names it: not an allow-list").toBe("kept");
    expect(page.headers.get("content-encoding"), "the stored body is decoded bytes").toBeNull();
    expect(page.headers.get("content-length")).toBeNull();
    expect(await page.text()).toBe(BODIES["/shell.bin"]!.body);
    // Anti-vacuity: the stored entry DOES carry the three, so their absence above is the worker's doing.
    const stored = (await worker.request("https://yokohama.test/shell.bin", "cors"))!;
    expect(stored.headers.get("content-encoding")).toBe("br");
    expect(stored.headers.get("content-type")).toBe("application/octet-stream");
  });

  // Not intercepted AT ALL: no `respondWith`, so the browser makes exactly the
  // request it would have made with no worker installed.
  //
  // **A path the release does not list is no longer in this list** (PR #37 F2,
  // R-F2): the worker now ANSWERS it, from a retained generation that holds it
  // or else from the network, and the block below is where that is read.
  it("does not touch the API, the turnstile, another origin or a POST", async () => {
    const worker = await boot();
    await worker.install();
    expect(await worker.request("https://yokohama.test/api/health", "cors")).toBeNull();
    expect(await worker.request("https://yokohama.test/enter", "navigate")).toBeNull();
    expect(await worker.request("https://elsewhere.test/index.html", "navigate")).toBeNull();
    // **The POST, which the name claimed and no arm drove** (whole-branch
    // review, false sentence 7). It is asked of a path the release DOES carry,
    // so the only thing that can be answering `null` is the verb —
    // `sw.ts`'s own first line — and not the route.
    expect(await worker.request("https://yokohama.test/shell.bin", "cors", "POST")).toBeNull();
    // Its anti-vacuity partner: the same path, as a GET, is served.
    expect(
      await (await worker.request("https://yokohama.test/shell.bin", "cors"))!.text(),
    ).toBe(BODIES["/shell.bin"]!.body);
  });

  // **Cloudflare's `/cdn-cgi/` is left to the browser** (PR #37, R-F6). Access's
  // return from the identity provider is a NAVIGATION to
  // `/cdn-cgi/access/authorized`, the hop that sets `CF_Authorization` on this
  // hostname — and every other navigation is answered with the cached shell,
  // which here would swallow the login.
  it("does not answer Cloudflare's /cdn-cgi/, the login hop above all", async () => {
    const worker = await boot();
    await worker.install();
    const before = worker.asked.length;
    expect(
      await worker.request("https://yokohama.test/cdn-cgi/access/authorized?token=abc", "navigate"),
    ).toBeNull();
    expect(await worker.request("https://yokohama.test/cdn-cgi/access/get-identity", "cors")).toBeNull();
    expect(worker.asked.length, "and the worker fetched neither itself").toBe(before);
    // Anti-vacuity: a navigation one character away IS answered with the shell.
    expect(await (await worker.request("https://yokohama.test/cdn-cgiX", "navigate"))!.text()).toBe(
      BODIES["/shell.bin"]!.body,
    );
  });

  // **DevTools' `only-if-cached` probes are the browser's.** The RULE is
  // `swRoute`'s and has its cells there; this is the worker honouring it, which
  // needs `sw.ts` to hand `request.cache` to the route — a call that dropped
  // the argument would route every probe as an ordinary request.
  it("leaves an only-if-cached request outside same-origin mode to the browser", async () => {
    const worker = await boot();
    await worker.install();
    const listed = "https://yokohama.test/assets/app-abc.js";
    expect(await worker.request(listed, "no-cors", "GET", "only-if-cached")).toBeNull();
    expect(
      await worker.request("https://yokohama.test/assets/unlisted.js", "cors", "GET", "only-if-cached"),
    ).toBeNull();
    // Anti-vacuity: in `same-origin` mode the same request is legal and served.
    expect(
      await (await worker.request(listed, "same-origin", "GET", "only-if-cached"))!.text(),
    ).toBe(BODIES["/assets/app-abc.js"]!.body);
  });
});

// **PR #37's F2** (`docs/plans/2026-09-22-web-frontend-m6.5b2-pr37-fixes.md`,
// R-F2): the tab that did not click. `skipWaiting()` is registration-wide, so
// the click in one tab hands EVERY tab the previous worker controlled to the
// new one — including a tab still running the previous release's bundle, which
// asks for its own hashed names. `activate` keeps the previous generation for
// exactly that tab (spec §2.3), and until this fix nothing READ it: the new
// worker knew only its own list and its own cache, so a name it did not list
// went to the network — which offline, or once the origin had dropped the file,
// is a failure. These cells run the real worker against that arrangement.
describe("the worker's fetch, for a path its own release does not list", () => {
  /** The generation `activate` retained: under the prefix, not this release. */
  const PREVIOUS = "yokohama-release-previous";
  /** A hashed name the previous release listed and this one does not. */
  const OLD = "/assets/worker-old.js";
  const OLD_BODY = "the previous release's local host";

  /** A worker with its own release installed, and the previous release's
   *  generation beside it holding `plant`'s paths. */
  async function beside(plant: Record<string, string>) {
    const worker = await boot();
    await worker.install();
    const previous = await worker.caches.open(PREVIOUS);
    await previous.put(META_PATH, new Response(JSON.stringify({ createdAt: 1 })));
    for (const [path, body] of Object.entries(plant)) await previous.put(path, new Response(body));
    return worker;
  }

  // **(a) The defect itself.** The retained generation holds the name and the
  // worker answers it from there — and the NETWORK is not asked: the retained
  // copy is the answer, not a fallback waiting behind a request that offline
  // cannot succeed.
  it("answers it from the retained generation that holds it, and asks the network nothing", async () => {
    const worker = await beside({ [OLD]: OLD_BODY });
    const before = worker.asked.length;
    const answered = await worker.request(`https://yokohama.test${OLD}`, "cors");
    expect(answered, "the worker answered the old release's name").not.toBeNull();
    expect(await answered!.text()).toBe(OLD_BODY);
    expect(worker.asked.length, "and not one request reached the network").toBe(before);
  });

  // **(b) A path no generation holds is the network's**, answered through the
  // worker now rather than by not answering: whatever the origin says — here
  // the 404 of a file it dropped — is what the page gets, once.
  it("sends a path no generation holds to the network, once, and hands back its answer", async () => {
    const worker = await beside({ [OLD]: OLD_BODY });
    const before = worker.asked.length;
    const answered = await worker.request("https://yokohama.test/assets/nobody-has-it.js", "cors");
    expect(answered, "answered, from the network").not.toBeNull();
    expect(answered!.status).toBe(404);
    expect(await answered!.text()).toBe(NOT_ON_THE_ORIGIN);
    expect(worker.asked.slice(before).map((a) => a.path)).toEqual(["/assets/nobody-has-it.js"]);
  });

  // **(c) Only a cache of OURS is ever read.** The origin's Cache Storage is
  // shared (`swInstall.ts`'s sweep says the same about deleting), so a cache
  // some other script put there holding the very path is not a release and is
  // not an answer. The network is.
  //
  // **A retained generation that does NOT hold the path is planted beside it,
  // and that is what gives the cell its reach.** With no generation of ours to
  // look in, the lookup never runs and a worker reading EVERY cache would pass
  // for the wrong reason — measured: `caches.match` with its `cacheName`
  // dropped survived this cell until the previous generation was here.
  it("never reads a cache outside its own prefix, even one holding the path", async () => {
    const worker = await beside({ "/assets/something-else.js": "not the path asked for" });
    const foreign = await worker.caches.open("someone-elses-cache");
    await foreign.put(OLD, new Response("somebody else's bytes"));
    const before = worker.asked.length;
    const answered = await worker.request(`https://yokohama.test${OLD}`, "cors");
    expect(await answered!.text(), "the answer is the network's, not the foreign cache's").toBe(
      NOT_ON_THE_ORIGIN,
    );
    expect(worker.asked.slice(before).map((a) => a.path)).toEqual([OLD]);
  });

  // **One generation that will not answer does not skip the next.** A storage
  // fault in the first retained generation is that generation's; the second,
  // which holds the path, still answers — and the network is not asked.
  it("reads past a generation whose read fails to the one that holds the path", async () => {
    const worker = await boot();
    await worker.install();
    const broken = await worker.caches.open("yokohama-release-broken");
    await broken.put(OLD, new Response("never read"));
    const holding = await worker.caches.open(PREVIOUS);
    await holding.put(OLD, new Response(OLD_BODY));
    worker.caches.failMatchIn = "yokohama-release-broken";
    const before = worker.asked.length;
    const answered = await worker.request(`https://yokohama.test${OLD}`, "cors");
    expect(await answered!.text(), "the second generation answered").toBe(OLD_BODY);
    expect(worker.asked.length, "and the network was not asked").toBe(before);
  });

  // **And never the stamp**, which is the one entry every generation holds that
  // no precache listed. Without this the sentence R-F2 rests its cost on — "the
  // only paths a generation can hold are ones a precache listed" — would be
  // false by one path, and the previous generation's install time would be
  // served as if it were a file.
  it("does not serve a retained generation's stamp as if it were a file", async () => {
    const worker = await beside({ [OLD]: OLD_BODY });
    const before = worker.asked.length;
    const answered = await worker.request(`https://yokohama.test${META_PATH}`, "cors");
    expect(await answered!.text()).toBe(NOT_ON_THE_ORIGIN);
    expect(worker.asked.slice(before).map((a) => a.path)).toEqual([META_PATH]);
  });

  // **(d) Rule 1 is still FIRST.** A retained generation somehow holding the
  // API or the turnstile — planted here, since no generator can list either
  // (`precache.ts` refuses them) — still cannot have them answered: no
  // `respondWith` at all, so the browser makes the request itself. A retained
  // lookup asked before the bypass is the mutation this cell catches.
  it("does not answer the API or the turnstile even from a generation that holds them", async () => {
    const worker = await beside({
      "/api/health": '{"stale":true}',
      "/api/games/x/input": '{"stale":true}',
      "/enter": "a door served from a cache",
    });
    const before = worker.asked.length;
    expect(await worker.request("https://yokohama.test/api/health", "cors")).toBeNull();
    expect(await worker.request("https://yokohama.test/api/games/x/input", "cors")).toBeNull();
    expect(await worker.request("https://yokohama.test/enter", "cors")).toBeNull();
    expect(await worker.request("https://yokohama.test/enter", "navigate")).toBeNull();
    expect(worker.asked.length, "and the worker fetched none of them itself").toBe(before);
  });

  // **(e) A navigation is still THIS release's shell**, whatever an older
  // generation holds under the same name: the retained lookup is for a path
  // this release does not list, and `/shell.bin` is listed. The previous
  // generation holds both shells a release has been listed under — a release
  // before R-6.5c2.41 listed `/index.html`.
  it("answers a navigation with this release's shell, not the retained one", async () => {
    const previous = "<!doctype html><title>the previous release</title>";
    const worker = await beside({ "/shell.bin": previous, "/index.html": previous });
    const shell = BODIES["/shell.bin"]!.body;
    expect(await (await worker.request("https://yokohama.test/t/anything", "navigate"))!.text()).toBe(
      shell,
    );
    expect(await (await worker.request("https://yokohama.test/shell.bin", "cors"))!.text()).toBe(
      shell,
    );
    // And a non-navigation GET of `/index.html` is a path this release does not
    // list, so it is R-F2's like any other: the retained generation's copy.
    expect(await (await worker.request("https://yokohama.test/index.html", "cors"))!.text()).toBe(
      previous,
    );
  });

  // **(f) Nothing is written at fetch time** — not into this release's cache,
  // not into the retained one, and no generation brought into being. Every
  // branch is driven: a retained hit, a miss to the network, a listed path, a
  // navigation, the stamp and the API.
  it("writes nothing into any cache while it answers", async () => {
    const worker = await beside({ [OLD]: OLD_BODY, "/api/health": "{}" });
    const puts = worker.caches.puts.length;
    const names = await worker.caches.keys();
    for (const [url, mode] of [
      [`https://yokohama.test${OLD}`, "cors"],
      ["https://yokohama.test/assets/nobody-has-it.js", "cors"],
      ["https://yokohama.test/assets/app-abc.js", "cors"],
      ["https://yokohama.test/t/anything", "navigate"],
      [`https://yokohama.test${META_PATH}`, "cors"],
      ["https://yokohama.test/api/health", "cors"],
    ] as const) {
      await worker.request(url, mode);
    }
    expect(worker.caches.puts.slice(puts), "not one put at fetch time").toEqual([]);
    expect(await worker.caches.keys(), "and no generation created or removed").toEqual(names);
  });
});
