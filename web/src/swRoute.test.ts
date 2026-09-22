// The service worker's routing decision (M6.5b2, spec §2.3's three rules).
//
// **A pure function so it can be tested without a browser**, which is the
// spec's own wording, and so that `sw.ts` has one implementation of the rule
// rather than a tested model beside an untested copy.

import { describe, expect, it } from "vitest";
import { swRoute } from "./swRoute";

const PRE = new Set(["/index.html", "/assets/index-abc.js", "/art/orders/O01.webp"]);

describe("what the worker does with a request", () => {
  // The three rules, and the first is the one that matters most: a cached API
  // reply would be a stale game state served as if it were live.
  it("never intercepts the API or the turnstile", () => {
    for (const u of ["/api/health", "/api/games/x/input", "/api/stream", "/enter"])
      expect(swRoute(new URL(u, "https://h"), "cors", PRE)).toBe("bypass");
  });
  it("serves a precached path from the cache and a navigation from the shell", () => {
    expect(swRoute(new URL("/assets/index-abc.js", "https://h"), "cors", PRE)).toBe("cache");
    expect(swRoute(new URL("/t/anything", "https://h"), "navigate", PRE)).toBe("navigate");
  });
  // The ROUTE's answer. What the worker then does with `network` — a retained
  // generation that holds the path, else the network (PR #37 F2, R-F2) — is
  // cache knowledge this function does not have, and `sw.node.test.ts` reads
  // it off the real worker.
  it("lets anything else go to the network uncached", () => {
    expect(swRoute(new URL("/unknown.json", "https://h"), "cors", PRE)).toBe("network");
  });

  // **The strongest form of rule 1**, and the catcher for spec §6.5's "`/api/*`
  // allowed into the cache": the bypass is asked BEFORE the list is consulted,
  // so a build that somehow listed an API path still cannot have it served from
  // a cache.
  it("bypasses the API even if a release listed it", () => {
    const wrong = new Set([...PRE, "/api/health", "/enter"]);
    expect(swRoute(new URL("/api/health", "https://h"), "cors", wrong)).toBe("bypass");
    expect(swRoute(new URL("/enter", "https://h"), "cors", wrong)).toBe("bypass");
  });

  // **And a NAVIGATION to the turnstile is still a bypass**, which is the one
  // ordering that matters on this screen: `/enter` is how Access mints the
  // token, it answers a 302 to another origin, and a cached shell served there
  // would take a player to a door they had just left.
  it("bypasses a navigation to the turnstile rather than serving the shell", () => {
    expect(swRoute(new URL("/enter", "https://h"), "navigate", PRE)).toBe("bypass");
    expect(swRoute(new URL("/api/games", "https://h"), "navigate", PRE)).toBe("bypass");
  });

  // A query string is not part of the path a release names, and a navigation
  // carries one whenever a link does.
  it("reads the path and ignores the query", () => {
    expect(swRoute(new URL("/assets/index-abc.js?v=2", "https://h"), "cors", PRE)).toBe("cache");
    expect(swRoute(new URL("/api/games?scope=open", "https://h"), "cors", PRE)).toBe("bypass");
  });

  // `/apiary.json` is not `/api/...`, and a prefix test that forgot the slash
  // would stop caching a file whose name merely starts the same way.
  it("does not mistake a path that merely begins like the API", () => {
    expect(swRoute(new URL("/apiary.json", "https://h"), "cors", PRE)).toBe("network");
    expect(swRoute(new URL("/entertainment", "https://h"), "cors", PRE)).toBe("network");
  });

  // The bare `/api` is the API too — the router owns the prefix, not only its
  // children.
  it("bypasses the bare API root", () => {
    expect(swRoute(new URL("/api", "https://h"), "cors", PRE)).toBe("bypass");
  });

  // **Cloudflare's `/cdn-cgi/` is the server's too** (PR #37, R-F6). Access's
  // return from the identity provider is a NAVIGATION to
  // `/cdn-cgi/access/authorized`, the hop that sets `CF_Authorization` on this
  // hostname; answered with the shell, the login is swallowed and `/enter`
  // loops. `/cdn-cgiX` is not the prefix, which is the same slash rule as
  // `/apiary.json`.
  it("bypasses Cloudflare's /cdn-cgi/, the login hop above all", () => {
    const hop = new URL("/cdn-cgi/access/authorized?token=abc&redirect_url=%2F", "https://h");
    expect(swRoute(hop, "navigate", PRE)).toBe("bypass");
    expect(swRoute(new URL("/cdn-cgi/access/get-identity", "https://h"), "cors", PRE)).toBe("bypass");
    expect(swRoute(new URL("/cdn-cgi", "https://h"), "navigate", PRE)).toBe("bypass");
    expect(swRoute(new URL("/cdn-cgiX", "https://h"), "navigate", PRE)).toBe("navigate");
    expect(swRoute(new URL("/cdn-cgiX/y.js", "https://h"), "cors", PRE)).toBe("network");
  });
});
