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
// **Every DECISION here is somewhere else, and on purpose.** The routing rule
// is `swRoute.ts`, the install's verification and the sweep's retention rule are
// `swInstall.ts`, and both are tested in the component suite without a browser.
// What is left in this file is the plumbing those decisions drive — which is
// the half a browser must run to check, so the less of it there is the better.

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

/** The cache handle, opened once rather than per request. */
let opened: Promise<Cache> | null = null;
function releaseCache(): Promise<Cache> {
  opened ??= caches.open(CACHE_NAME);
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
  try {
    const cache = await caches.open(CACHE_NAME);
    // The stamp the sweep orders generations by. Written FIRST, so a generation
    // that exists at all can be placed in time.
    await cache.put(
      META_PATH,
      new Response(JSON.stringify({ createdAt: Date.now() }), {
        headers: { "content-type": "application/json" },
      }),
    );

    const queue = ENTRIES.slice();
    const lanes = Math.max(1, Math.min(INSTALL_CONCURRENCY, queue.length));
    await Promise.all(
      Array.from({ length: lanes }, async () => {
        for (;;) {
          const entry = queue.shift();
          if (!entry) return;
          await installOne(cache, entry, deadline.signal);
        }
      }),
    );
  } catch (error) {
    const reason = deadline.signal.aborted
      ? `the install did not finish inside ${INSTALL_DEADLINE_MS} ms`
      : messageOf(error);
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

// **`activate` retains the PREVIOUS generation** and deletes only ones older
// than that (spec §2.3), because an open tab still running the old bundle asks
// for its own hashed module by name.
//
// **And it does NOT call `clients.claim()`.** Claiming would put this worker in
// front of pages still running the old bundle — the exact mixture the matched
// cache generation exists to prevent. A page gets this release by reloading,
// which is what the update offer's click does.
self.addEventListener("activate", (event) => {
  event.waitUntil(sweep());
});

async function sweep(): Promise<void> {
  const generations: Generation[] = [];
  for (const name of await caches.keys()) {
    if (!name.startsWith(CACHE_PREFIX)) continue;
    generations.push({ name, createdAt: await stampOf(name) });
  }
  for (const name of generationsToDelete(CACHE_NAME, generations)) {
    await caches.delete(name).catch(() => undefined);
  }
}

async function stampOf(name: string): Promise<number> {
  try {
    const cache = await caches.open(name);
    const stamp = await cache.match(META_PATH);
    if (!stamp) return 0;
    const meta = (await stamp.json()) as { createdAt?: unknown };
    return typeof meta.createdAt === "number" ? meta.createdAt : 0;
  } catch {
    return 0;
  }
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

  const route = swRoute(url, request.mode, PRECACHED);
  // `bypass` and `network` are both answered by NOT answering: no
  // `respondWith`, so the browser makes exactly the request it would have made
  // with no worker installed.
  if (route === "cache") event.respondWith(fromCache(url.pathname, request));
  else if (route === "navigate") event.respondWith(fromCache("/index.html", request));
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
