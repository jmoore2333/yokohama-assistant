// **The service worker: one release, cached whole, keyed by its own contents**
// (M6.5b2, spec §2.3; R-6.5b2.8).
//
// This file is not bundled by Vite and is never imported by the page.
// `scripts/precache.mjs` builds it with esbuild AFTER `vite build`, substituting
// the release's list and name through esbuild's `define`, and writes the result
// to `dist/sw.js`. The server answers that one path `Cache-Control: no-store`
// (`assets.rs`'s `cache_control`), because update detection is a byte
// comparison of this file and a heuristically cached copy delays every update
// by however long the browser chose to keep it.
//
// **Where the decisions live.** The routing rule is `swRoute.ts`; the install's
// verification, the sweep's retention rule and which retained generations a
// fetch may read are `swInstall.ts`; both are pure and tested in the component
// suite without a browser. What this file holds is the plumbing those
// decisions drive — the half a browser must run to check, so the less of it
// the better — and the rules below, which it makes on its own.
//
// **These are pinned by `sw.node.test.ts`**, which bundles this file and RUNS
// it; each was checked by mutating it and watching a cell there fail:
//
// - the request filter: GET only, and same-origin only — the caller's question,
//   as `swRoute.ts` says, asked here where `self.location` is real — with the
//   request's cache mode handed to `swRoute` for its `only-if-cached` rule;
// - the install is all-or-nothing: one failed entry fails it, and the door is
//   told why;
// - every entry is fetched in parallel lanes with `cache: "reload"` and
//   `redirect: "error"`, its head checked and its body's digest CALLED;
// - a failed install deletes its generation only where the install CREATED it
//   (`alreadyThere`), never the one a running worker serves under the same name;
// - a re-install of the same generation keeps the `activatedAt` it finds (R-F5);
// - the update policy's worker half: `skipWaiting()` only on the player's
//   message, never `clients.claim()`;
// - `activate` stamps its own generation `activatedAt`, keeping `createdAt`
//   (R-F5), and writes nothing where the install's stamp is absent — reading it
//   without opening the cache — so it never brings an evicted generation back;
// - the release's own cache: a failed open is not remembered, and a request it
//   cannot answer falls through to the network;
// - R-F2's order for a path this release does not list: a retained generation
//   before the network, one `try` per generation, nothing written;
// - `fromRetained` never serves the stamp (`META_PATH`).
//
// **And these are made here and pinned by NO cell** — the same mutations, each
// of which every cell survived:
//
// - on a failed install the other lanes are ABORTED and AWAITED before the
//   delete. The abort cell does not see its own mutation: the per-fetch timeout
//   aborts the held lane instead, fifteen seconds later, and the cell passes;
// - the per-fetch timeout and the whole-install deadline themselves;
// - `credentials: "same-origin"` on every install fetch;
// - the stamp written FIRST, before any entry;
// - a failed install telling uncontrolled pages too (`includeUncontrolled`),
//   and a timeout worded "took too long".

/// <reference lib="webworker" />

import { CACHE_PREFIX, META_PATH, type PrecacheEntry } from "./precache";
import {
  checkDigest,
  checkHead,
  FETCH_TIMEOUT_MS,
  hex,
  INSTALL_CONCURRENCY,
  INSTALL_DEADLINE_MS,
  generationsToDelete,
  retainedGenerations,
  type Generation,
} from "./swInstall";
import { swRoute } from "./swRoute";

declare const self: ServiceWorkerGlobalScope;

/** The release's Cache Storage name, written in by the generator. */
declare const __CACHE_NAME__: string;
/** The release's entries, written in by the generator. */
declare const __PRECACHE__: readonly PrecacheEntry[];

const CACHE_NAME = __CACHE_NAME__;
const ENTRIES = __PRECACHE__;
const PRECACHED: ReadonlySet<string> = new Set(ENTRIES.map((e) => e.path));

/** The cache handle, opened once rather than per request.
 *
 *  **A rejection is not remembered.** `??=` alone caches the rejected promise
 *  for the life of the worker, so one transient failure to open the cache would
 *  turn every later navigation into a network fetch until the worker was
 *  replaced — which offline means a broken page rather than a slow one. */
let opened: Promise<Cache> | null = null;
function releaseCache(): Promise<Cache> {
  opened ??= caches.open(CACHE_NAME).catch((error: unknown) => {
    opened = null;
    throw error;
  });
  return opened;
}

// ---------------------------------------------------------------- install

// **All-or-nothing** (spec §2.3): one failure fails the whole install and
// leaves the previous release in place, because a cache holding the shell but
// not the art is exactly the half-release this design refuses to call offline.
//
// **Nothing calls `skipWaiting()` here.** A worker that skipped on install
// would switch a player mid-game — `skipWaiting` is registration-wide, so it
// can put the new worker in front of a page still running the OLD bundle. It is
// called only from the message below, which only the player's click sends.
self.addEventListener("install", (event) => {
  event.waitUntil(installRelease());
});

async function installRelease(): Promise<void> {
  // **Whether this generation is OURS to delete on failure.** The cache name is
  // a digest over the list, so a code-only change to this file ships a new
  // worker under the SAME name — and a failed install that deleted it would
  // take out the generation the active worker is serving from. Re-verifying
  // that generation cannot corrupt it either: the same list has the same
  // digests, so every entry this pass manages to put back is byte-identical to
  // the one it replaced.
  const alreadyThere = (await caches.keys()).includes(CACHE_NAME);

  const deadline = new AbortController();
  const overdue = setTimeout(() => deadline.abort(), INSTALL_DEADLINE_MS);
  /** The fetch-and-put lanes, held so the failure path can wait for them. */
  let lanes: Promise<void>[] = [];
  try {
    const cache = await caches.open(CACHE_NAME);
    // The stamp the sweep orders generations by. Written FIRST, so a generation
    // that exists at all can be placed in time.
    //
    // **A re-install of the SAME generation keeps the activation it carries**
    // (R-F5). A code-only change to this file ships a new worker under the same
    // name, and while it waits the generation is still the one the running
    // worker serves; a stamp rewritten without its `activatedAt` would rank it
    // below an older activated generation at the next sweep, and a release that
    // replaced this waiting worker would then delete the bytes every open tab
    // is running.
    const kept = alreadyThere ? (await stampIn(CACHE_NAME)).activatedAt : undefined;
    await writeStamp(cache, {
      createdAt: Date.now(),
      ...(kept === undefined ? {} : { activatedAt: kept }),
    });

    const queue = ENTRIES.slice();
    const width = Math.max(1, Math.min(INSTALL_CONCURRENCY, queue.length));
    lanes = Array.from({ length: width }, async () => {
      for (;;) {
        const entry = queue.shift();
        if (!entry) return;
        await installOne(cache, entry, deadline.signal);
      }
    });
    await Promise.all(lanes);
  } catch (error) {
    const reason = deadline.signal.aborted
      ? `the install did not finish inside ${INSTALL_DEADLINE_MS} ms`
      : messageOf(error);
    // **Stop the other lanes, and WAIT for them, before deleting.**
    // `Promise.all` rejects on the FIRST failure and leaves its siblings
    // running: without the abort they would go on downloading a release that
    // has already failed, and without the wait the delete below could land
    // while one of them was still inside `cache.put`. `allSettled` never
    // rejects, and every lane is already handled by the `Promise.all` above,
    // so neither call can raise an unhandled rejection.
    deadline.abort();
    await Promise.allSettled(lanes);
    if (!alreadyThere) await caches.delete(CACHE_NAME).catch(() => undefined);
    await tellTheClients(reason);
    // Rethrown, which is what FAILS the install: the browser marks this worker
    // redundant and the previous one goes on serving.
    throw new Error(reason);
  } finally {
    clearTimeout(overdue);
  }
}

async function installOne(cache: Cache, entry: PrecacheEntry, deadline: AbortSignal): Promise<void> {
  const stop = new AbortController();
  const onDeadline = () => stop.abort();
  if (deadline.aborted) onDeadline();
  else deadline.addEventListener("abort", onDeadline);
  const late = setTimeout(() => stop.abort(), FETCH_TIMEOUT_MS);
  try {
    // **`cache: "reload"` and `redirect: "error"`, and neither is the other's
    // understudy** (spec §2.3). `reload` goes past the HTTP cache, which the art
    // needs: its URLs are stable rather than hashed and the server serves them
    // `max-age=3600`, so an install minutes after a re-cut would otherwise
    // freeze the hour-old picture under the new release's name. `redirect:
    // "error"` refuses a login page answering 200 for a file.
    const response = await fetch(entry.path, {
      cache: "reload",
      redirect: "error",
      credentials: "same-origin",
      signal: stop.signal,
    });
    const head = checkHead(response.status, response.headers.get("content-type"), entry.type);
    if (head) throw new Error(head);
    const body = await response.arrayBuffer();
    const wrong = checkDigest(hex(await crypto.subtle.digest("SHA-256", body)), entry.digest);
    if (wrong) throw new Error(`its body ${wrong}`);
    await cache.put(
      entry.path,
      new Response(body, { status: 200, statusText: response.statusText, headers: response.headers }),
    );
  } catch (error) {
    throw new Error(`${entry.path}: ${messageOf(error)}`);
  } finally {
    clearTimeout(late);
    deadline.removeEventListener("abort", onDeadline);
  }
}

/** A sentence out of whatever was thrown — an `AbortError` included, which is
 *  what a timeout looks like from inside a `fetch`. */
function messageOf(error: unknown): string {
  if (error instanceof Error) {
    return error.name === "AbortError" ? "took too long" : error.message;
  }
  return String(error);
}

/** Tell every open page why, so the door can NAME the failure and offer a
 *  retry rather than saying "installing" for ever.
 *
 *  `includeUncontrolled`, because on a first visit there is no controlled
 *  client at all — the page that is watching this install is precisely one this
 *  worker does not control yet. */
async function tellTheClients(reason: string): Promise<void> {
  try {
    const windows = await self.clients.matchAll({ includeUncontrolled: true, type: "window" });
    for (const client of windows) client.postMessage({ yokohama: "install-failed", reason });
  } catch {
    // A page that cannot be told is not a reason to turn a failed install into
    // a different failure; the lifecycle still marks this worker redundant.
  }
}

// --------------------------------------------------------------- activate

// **`activate` stamps its own generation, then retains the PREVIOUS one** and
// deletes the rest (spec §2.3; PR #37, R-F5). "Previous" is the generation
// ACTIVATED most recently before this one, because that is what the open tabs
// are running — and `fromRetained` below is what answers them from it (PR #37
// F2). The stamp is what lets the NEXT release's sweep tell that generation
// from a waiting one that was replaced before anyone clicked.
//
// **And it does NOT call `clients.claim()`.** Claiming would put this worker in
// front of pages NO worker served — the first visit among them, whose bundle
// came from the network. A page gets this release by reloading, which is what
// the update offer's click does. **What not claiming does NOT buy, and what the
// first version of this comment misread:** the pages the previous worker
// controlled are handed to this one by `skipWaiting()` itself, which is
// registration-wide — so a tab that did not click is in front of this worker,
// still running the old bundle, whether or not anything claims it.
self.addEventListener("activate", (event) => {
  event.waitUntil(markActivated().then(sweep));
});

/** Write `activatedAt` into this release's own stamp, keeping `createdAt`.
 *
 *  **Only where the install's stamp is already there**: the install writes it
 *  first, so a generation without one is not a release this worker installed —
 *  and `caches.open` on a generation evicted since would bring an empty one
 *  back into being, which the next sweep would then keep as "previous".
 *
 *  **A stamp that cannot be written is a known gap, accepted rather than
 *  guarded** (the PR #37 fix wave's ruling). The generation then carries no
 *  `activatedAt`, so it ranks BELOW every activated generation, and at the
 *  NEXT activation the sweep can keep an older activated one and delete this
 *  one while tabs still run it — R-F5's defect, for one release, on the device
 *  whose write failed. It is accepted because that device has stored this
 *  whole release, about ten megabytes, and the activation stamp's `cache.put`
 *  REPLACES an entry of about the same size — a hundred bytes or so — so it
 *  adds almost nothing to the quota; and the guard that would cover it — keep
 *  an unactivated generation installed after the newest activation — would
 *  also keep every replaced waiting release for a release. */
async function markActivated(): Promise<void> {
  try {
    const { createdAt } = await stampIn(CACHE_NAME);
    if (createdAt === undefined) return;
    await writeStamp(await caches.open(CACHE_NAME), { createdAt, activatedAt: Date.now() });
  } catch {
    // The gap named above: unstamped, this generation ranks below every
    // activated one at the next sweep.
  }
}

async function sweep(): Promise<void> {
  const generations: Generation[] = [];
  for (const name of await caches.keys()) {
    if (!name.startsWith(CACHE_PREFIX)) continue;
    const { createdAt, activatedAt } = await stampIn(name);
    generations.push({
      name,
      createdAt: createdAt ?? 0,
      ...(activatedAt === undefined ? {} : { activatedAt }),
    });
  }
  for (const name of generationsToDelete(CACHE_NAME, generations)) {
    await caches.delete(name).catch(() => undefined);
  }
}

/** A generation's stamp, each field present only where it is a number. Read
 *  with `caches.match` and a `cacheName`, which never creates the cache. */
async function stampIn(name: string): Promise<{ createdAt?: number; activatedAt?: number }> {
  try {
    const stamp = await caches.match(META_PATH, { cacheName: name });
    if (!stamp) return {};
    const meta = (await stamp.json()) as { createdAt?: unknown; activatedAt?: unknown };
    return {
      ...(typeof meta.createdAt === "number" ? { createdAt: meta.createdAt } : {}),
      ...(typeof meta.activatedAt === "number" ? { activatedAt: meta.activatedAt } : {}),
    };
  } catch {
    return {};
  }
}

function writeStamp(cache: Cache, stamp: { createdAt: number; activatedAt?: number }): Promise<void> {
  return cache.put(
    META_PATH,
    new Response(JSON.stringify(stamp), { headers: { "content-type": "application/json" } }),
  );
}

// ---------------------------------------------------------------- message

// The ONE way this worker ever takes over: the player clicked the offer.
self.addEventListener("message", (event) => {
  const data = event.data as { yokohama?: unknown } | null;
  if (data && data.yokohama === "skip-waiting") void self.skipWaiting();
});

// ------------------------------------------------------------------ fetch

self.addEventListener("fetch", (event) => {
  const request = event.request;
  // A POST to `/api/...` is the server's anyway, and a cache holds only GETs.
  if (request.method !== "GET") return;
  let url: URL;
  try {
    url = new URL(request.url);
  } catch {
    return;
  }
  // **Same-origin is asked HERE**, where `self.location` is real: a worker must
  // never serve another origin's response out of this origin's cache.
  if (url.origin !== self.location.origin) return;

  const route = swRoute(url, request.mode, PRECACHED, request.cache);
  // `bypass` is answered by NOT answering — no `respondWith`, so the browser
  // makes exactly the request it would have made with no worker installed,
  // which is what rule 1 means by "never intercepted". It is asked FIRST, in
  // `swRoute`, so nothing below can reach the API, the turnstile or
  // Cloudflare's `/cdn-cgi/` (R-F6) — nor a DevTools `only-if-cached` probe
  // outside `same-origin` mode, which `fetch()` would refuse outright.
  //
  // **`network` IS answered** (PR #37 F2, R-F2): from a retained generation
  // that holds the path, else from the network. It used to be left alone like
  // `bypass`, and that is what stranded the tab that did not click.
  if (route === "cache") event.respondWith(fromCache(url.pathname, request));
  else if (route === "navigate") event.respondWith(fromCache("/index.html", request));
  else if (route === "network") event.respondWith(fromRetained(url.pathname, request));
});

/** The cached copy, or the network if this generation somehow lacks it.
 *
 *  A miss should be impossible — the install is all-or-nothing and the sweep
 *  never deletes the running generation — so the fall-through is for the case
 *  the platform can still produce: storage pressure evicting a whole cache
 *  between two requests. Answering from the network there is a slow page; not
 *  answering is a broken one. */
async function fromCache(path: string, request: Request): Promise<Response> {
  try {
    const hit = await (await releaseCache()).match(path);
    if (hit) return hit;
  } catch {
    // fall through
  }
  return fetch(request);
}

/** A path this release does not list: from a RETAINED generation of ours that
 *  holds it, else the network — and written nowhere (PR #37 F2, R-F2, which
 *  amends spec §2.3's third rule forward).
 *
 *  **This is the tab the retention exists for.** `skipWaiting()` is
 *  registration-wide, so one player's click hands every tab the previous worker
 *  controlled to this one, including a tab that did not click and is still
 *  running the previous release's bundle (spec §2.3, lines 337–339). That tab
 *  asks for its own hashed names — a Worker it restarts, most of all — which
 *  this release does not list. Before this branch nothing read the generation
 *  `activate` keeps for it, so such a request went to the network and failed
 *  offline, or once the origin had dropped the file.
 *
 *  **Only what a release of ours listed can come out of here.** The caches are
 *  [`retainedGenerations`]' — ours by prefix, never this release's own — and
 *  the stamp is skipped, being the one entry every generation holds that no
 *  precache named. `caches.match` with a `cacheName` reads and never creates,
 *  so a generation swept between the listing and the read is a miss rather
 *  than an empty cache brought back into being. */
async function fromRetained(path: string, request: Request): Promise<Response> {
  if (path !== META_PATH) {
    let names: string[] = [];
    try {
      names = retainedGenerations(CACHE_NAME, await caches.keys());
    } catch {
      // fall through: a Cache Storage that will not list leaves the network
    }
    for (const name of names) {
      // One `try` per generation, so one that will not answer does not skip
      // the one after it that would.
      try {
        const hit = await caches.match(path, { cacheName: name });
        if (hit) return hit;
      } catch {
        // the next generation, and then the network
      }
    }
  }
  return fetch(request);
}
