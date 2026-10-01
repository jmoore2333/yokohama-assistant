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
 *  `bypass` and `network` are deliberately DISTINCT values: `bypass` is a RULE
 *  — the API, the turnstile and Cloudflare's `/cdn-cgi/` are never read and
 *  never written, whatever a release lists, and neither is a DevTools
 *  `only-if-cached` probe outside `same-origin` mode, which `fetch()` refuses
 *  outright; `sw.ts` answers it by NOT answering, so the browser makes the
 *  request it would have made with no worker installed at all — while
 *  `network` is merely a path this release does not list. **`sw.ts` DOES answer
 *  that one** (PR #37 F2, R-F2): from a retained generation of ours that holds
 *  the path, else from the network, and it caches nothing. Which generation
 *  holds a path is cache knowledge, so it is asked there and this function
 *  stays pure; the value keeps its name because the network is still where a
 *  path goes when no generation of ours has it. */
export type Route = "bypass" | "cache" | "navigate" | "network";

/** Whether this path belongs to the server rather than to the release.
 *
 *  `/api` and everything under it, and `/enter` — the Access turnstile, which
 *  answers a 302 to another origin and is how the token every later request
 *  carries gets minted. A cached answer for either is a stale game state, or a
 *  door, served as if it were live.
 *
 *  **And `/cdn-cgi` and everything under it** (PR #37, R-F6): Cloudflare's
 *  reserved prefix on the app's OWN hostname. Access's return from the identity
 *  provider is a NAVIGATION to `/cdn-cgi/access/authorized`, the hop that sets
 *  `CF_Authorization` — and rule 2 answers every navigation with the cached
 *  shell, so a worker that reached it would swallow the login: the cookie is
 *  never set and `/enter` loops. The app serves nothing there, so the bypass
 *  costs nothing.
 *
 *  The comparison is `=== "/api"` OR `startsWith("/api/")` and never
 *  `startsWith("/api")`, which would also claim `/apiary.json` — and the same
 *  for `/cdn-cgi`. */
export function isServerPath(pathname: string): boolean {
  return (
    pathname === "/enter" ||
    pathname === "/api" ||
    pathname.startsWith("/api/") ||
    pathname === "/cdn-cgi" ||
    pathname.startsWith("/cdn-cgi/")
  );
}

/**
 * Spec §2.3's three rules, in the one order that makes them true.
 *
 * 1. `/api/*`, `/enter` and `/cdn-cgi/*` are never intercepted — asked FIRST,
 *    so a release that somehow listed one still cannot have it served from a
 *    cache, and a NAVIGATION to `/enter` or to Access's
 *    `/cdn-cgi/access/authorized` is not answered with the cached shell (R-F6
 *    amends spec §2.3's rule 1 forward). Nor is DevTools' `only-if-cached`
 *    probe outside `same-origin` mode: such a Request cannot be handed to
 *    `fetch()` at all — the constructor throws — so a worker that answered one
 *    through the network would print an error for a request the page never
 *    made. It is the browser's, as it was before any worker existed.
 * 2. A navigation is answered with the shell, whatever path it names: the
 *    client routes on the hash, so every address is `index.html`.
 * 3. A precached path is served from the cache.
 * 4. Everything else is `network`, and is never cached. `sw.ts` answers it from
 *    a RETAINED generation that holds it, else the network — which is how the
 *    tab that did not click an update still reaches its own release's hashed
 *    names (R-F2 amends spec §2.3's "goes to the network" forward).
 *
 * @param url       the request's URL, already known to be same-origin
 * @param mode      `request.mode` — `"navigate"` for a document load
 * @param precached the release's paths, as the worker holds them
 * @param cache     `request.cache` — `"only-if-cached"` is the one value read
 */
export function swRoute(
  url: URL,
  mode: string,
  precached: ReadonlySet<string>,
  cache: string = "default",
): Route {
  if (isServerPath(url.pathname)) return "bypass";
  if (cache === "only-if-cached" && mode !== "same-origin") return "bypass";
  if (mode === "navigate") return "navigate";
  if (precached.has(url.pathname)) return "cache";
  return "network";
}
