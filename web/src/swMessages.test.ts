// The service worker's message to the page, and the door it elects to carry a challenge (security S3, W-3; D-13).
//
// `isDoorUrl` is a RESTATEMENT of `route.ts`'s rule, because the worker's bundle cannot import `route.ts` (it
// imports React). The first cell holds it to `parseRoute` itself, which is imported HERE and nowhere in the
// worker; the others are what a window's URL can be that a hash cannot.

import { describe, expect, it } from "vitest";

import { parseRoute } from "./route";
import { isDoorUrl } from "./swMessages";

describe("which address is the door's", () => {
  it("reads the hash exactly as parseRoute does", () => {
    const hashes = [
      "",
      "#",
      "#/",
      "#/lobby",
      "#/new",
      "#/new/local",
      "#/t/abc",
      "#/t/local-abc",
      "#/t/not safe!",
      "#/t/..",
      "#/t/%zz",
      "#/t/abc/more",
      "#/garbage",
      "#/lobby/extra",
      "#/new/local/x",
      "#lobby",
    ];
    for (const hash of hashes) {
      expect(isDoorUrl("https://x.test/" + hash), `the hash ${JSON.stringify(hash)}`).toBe(
        parseRoute(hash).kind === "door",
      );
    }
    // Anti-vacuity: the table holds both answers.
    expect(hashes.filter((h) => parseRoute(h).kind === "door").length).toBeGreaterThan(0);
    expect(hashes.filter((h) => parseRoute(h).kind !== "door").length).toBeGreaterThan(0);
  });

  // The app answers a navigation at any path with the shell and routes on the hash alone (`swRoute.ts`'s rule
  // 2), so a path is the door wherever the shell is what draws there.
  it("is the door at any path the shell draws, with or without a hash", () => {
    expect(isDoorUrl("https://x.test")).toBe(true);
    expect(isDoorUrl("https://x.test/index.html")).toBe(true);
    expect(isDoorUrl("https://x.test/t/abc")).toBe(true);
    expect(isDoorUrl("https://x.test/somewhere#/t/abc")).toBe(false);
    // Plain `http:` too: the local e2e serves the client at `http://127.0.0.1:<port>`.
    expect(isDoorUrl("http://127.0.0.1:4173/#/")).toBe(true);
  });

  // **What a hash rule alone would call the door, and is not.** `clients.matchAll` lists every window of the
  // origin, and a window on a server path — the sign-in at `/enter`, a JSON answer under `/api`, Cloudflare's
  // `/cdn-cgi/` — or a blank window the origin opened is not running the app at all: elected, it would swallow
  // the challenge where nothing reads the message.
  it("is not the door where the app does not run", () => {
    expect(isDoorUrl("about:blank")).toBe(false);
    expect(isDoorUrl("https://x.test/enter")).toBe(false);
    expect(isDoorUrl("https://x.test/enter#/")).toBe(false);
    expect(isDoorUrl("https://x.test/api/me")).toBe(false);
    expect(isDoorUrl("https://x.test/cdn-cgi/access/authorized")).toBe(false);
    expect(isDoorUrl("not a url")).toBe(false);
    expect(isDoorUrl("")).toBe(false);
    // Anti-vacuity, one character from the server's paths: the app's.
    expect(isDoorUrl("https://x.test/enterprise")).toBe(true);
    expect(isDoorUrl("https://x.test/apiary")).toBe(true);
  });
});
