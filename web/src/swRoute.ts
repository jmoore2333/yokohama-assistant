// **The service worker's three rules, as one pure function** (M6.5b2, spec
// §2.3; R-6.5b2.8).
//
// It lives here rather than inside `sw.ts` so it can be tested without a
// browser, and `sw.ts` IMPORTS it: a tested model beside an untested copy is
// two implementations, and the tested one would not be the one that runs.
//
// **Same-origin is the CALLER's question, not this function's.** The signature
// takes a `URL` with no origin of its own to compare against, so `sw.ts` refuses
// a cross-origin request before it asks — a worker must never serve another
// origin's response out of this origin's cache, and the check belongs where
// `self.location` is real.

/** What the worker does with one request.
 *
 *  `bypass` and `network` both reach the network today and are deliberately
 *  DISTINCT values: `bypass` is a RULE — the API and the turnstile are never
 *  read and never written, whatever a release lists — while `network` is merely
 *  the default for a path this release does not know. `sw.ts` answers neither
 *  with `respondWith`, so in both cases the browser makes the request it would
 *  have made with no worker installed at all. */
export type Route = "bypass" | "cache" | "navigate" | "network";

/** Whether this path belongs to the server rather than to the release.
 *
 *  `/api` and everything under it, and `/enter` — the Access turnstile, which
 *  answers a 302 to another origin and is how the token every later request
 *  carries gets minted. A cached answer for either is a stale game state, or a
 *  door, served as if it were live.
 *
 *  The comparison is `=== "/api"` OR `startsWith("/api/")` and never
 *  `startsWith("/api")`, which would also claim `/apiary.json`. */
export function isServerPath(pathname: string): boolean {
  return pathname === "/enter" || pathname === "/api" || pathname.startsWith("/api/");
}

/**
 * Spec §2.3's three rules, in the one order that makes them true.
 *
 * 1. `/api/*` and `/enter` are never intercepted — asked FIRST, so a release
 *    that somehow listed one still cannot have it served from a cache, and a
 *    NAVIGATION to `/enter` is not answered with the cached shell.
 * 2. A navigation is answered with the shell, whatever path it names: the
 *    client routes on the hash, so every address is `index.html`.
 * 3. A precached path is served from the cache.
 * 4. Everything else goes to the network and is not cached.
 *
 * @param url       the request's URL, already known to be same-origin
 * @param mode      `request.mode` — `"navigate"` for a document load
 * @param precached the release's paths, as the worker holds them
 */
export function swRoute(url: URL, mode: string, precached: ReadonlySet<string>): Route {
  if (isServerPath(url.pathname)) return "bypass";
  if (mode === "navigate") return "navigate";
  if (precached.has(url.pathname)) return "cache";
  return "network";
}
