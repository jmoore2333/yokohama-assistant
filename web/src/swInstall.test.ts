// The install's verification and the activate sweep's retention rule
// (M6.5b2, spec §2.3).
//
// **Pure, because these are the two decisions the whole update policy rests
// on** and neither can be read off a running browser: an install that accepted
// a body it did not digest, or an activate that deleted the generation an open
// tab is still running from, both look exactly like a healthy release until the
// day they do not.

import { describe, expect, it } from "vitest";
import { checkHead, checkDigest, generationsToDelete, hex, retainedGenerations } from "./swInstall";
import { CACHE_PREFIX } from "./precache";

describe("what the install accepts", () => {
  it("accepts a 200 of the expected type", () => {
    expect(checkHead(200, "application/wasm", "application/wasm")).toBeNull();
    // The essence, not the header: the server answers `text/html; charset=utf-8`
    // and the release names `text/html`.
    expect(checkHead(200, "text/html; charset=utf-8", "text/html")).toBeNull();
    expect(checkHead(200, "TEXT/HTML;CHARSET=UTF-8", "text/html")).toBeNull();
  });

  // Not `res.ok`: a 206 is a partial body, and a redirect chain that ended at a
  // sign-in page is a 200 of the wrong type.
  it("refuses anything that is not exactly 200", () => {
    for (const status of [0, 199, 201, 206, 301, 304, 403, 404, 500])
      expect(checkHead(status, "application/wasm", "application/wasm")).toMatch(String(status));
  });

  it("refuses the wrong type, and an absent one", () => {
    expect(checkHead(200, "text/html; charset=utf-8", "application/wasm")).toMatch(/text\/html/);
    expect(checkHead(200, null, "application/wasm")).toMatch(/application\/wasm/);
  });

  // **The per-body digest, and it is the catcher for spec §6.5's "the install's
  // per-body digest check dropped".** It is not redundant with `cache:
  // "reload"`: a deployment that changes between the first fetch and the last
  // answers every request with a valid 200 of the right type, from two
  // different releases.
  it("refuses a body whose bytes are not the release's", () => {
    expect(checkDigest("abc", "abc")).toBeNull();
    const said = checkDigest("abc", "abd");
    expect(said).toMatch(/abc/);
    expect(said).toMatch(/abd/);
  });
});

describe("a digest as the build wrote it", () => {
  // A byte under 16 renders as ONE character without the padding, so an
  // unpadded digest is both short and ambiguous — [0x0a, 0xbc] and [0xab, 0xc0]
  // would both read "abc" and compare equal.
  it("pads every byte, so two different bodies never read the same", () => {
    const of = (...bytes: number[]) => hex(new Uint8Array(bytes).buffer);
    expect(of(0x0a, 0xbc)).toBe("0abc");
    expect(of(0xab, 0xc0)).toBe("abc0");
    expect(of(0x0a, 0xbc)).not.toBe(of(0xab, 0xc0));
    expect(of(0, 0, 255)).toBe("0000ff");
    expect(of().length).toBe(0);
  });
});

describe("which cache generations activate deletes", () => {
  const g = (name: string, createdAt: number) => ({ name: CACHE_PREFIX + name, createdAt });
  /** A generation a worker ACTIVATED from, at `activatedAt` (R-F5). */
  const on = (name: string, createdAt: number, activatedAt: number) => ({
    ...g(name, createdAt),
    activatedAt,
  });
  const mine = CACHE_PREFIX + "new";

  // **R-F5's case, and the reason the stamp exists** (PR #37). N was active
  // with a game tab open; N+1 installed and waited; N+2 installed later and
  // took its place, so N+1 was never activated and carries the NEWEST install
  // stamp. Ranking by install time kept N+1 and deleted N — the generation the
  // open tab is running. The previously ACTIVE one is what "previous" means.
  it("keeps the generation activated last, not the one installed last", () => {
    const out = generationsToDelete(mine, [
      g("new", 300),
      on("n", 100, 150),
      g("n-plus-1", 200),
    ]);
    expect(out).toEqual([CACHE_PREFIX + "n-plus-1"]);
  });

  // With no activation stamp anywhere — every generation installed before R-F5
  // — the install order stands exactly as it did. The cells below this one are
  // that case too; this is it said out loud.
  it("with no activation stamp anywhere, keeps the newest install, as before R-F5", () => {
    const out = generationsToDelete(mine, [g("new", 300), g("prev", 200), g("older", 100)]);
    expect(out).toEqual([CACHE_PREFIX + "older"]);
  });

  // **The transition, and a mix.** Any generation that ever activated outranks
  // every one that never did, however new; among the activated, the newest
  // activation wins even over a newer install. And a generation stamped
  // before R-F5 is kept by its install time when nothing activated outranks it
  // — which is how R-F5's first activation keeps the release it replaces.
  it("ranks every activated generation above every unactivated one, newest activation first", () => {
    const out = generationsToDelete(mine, [
      g("new", 900),
      g("legacy", 800),
      g("replaced", 700),
      on("activated-early", 600, 610),
      on("activated-late", 100, 650),
    ]);
    expect(out.sort()).toEqual(
      [CACHE_PREFIX + "activated-early", CACHE_PREFIX + "legacy", CACHE_PREFIX + "replaced"].sort(),
    );
    expect(generationsToDelete(mine, [g("new", 900), g("legacy-active", 800)])).toEqual([]);
    expect(
      generationsToDelete(mine, [on("new", 900, 950), g("legacy-active", 800), g("older", 100)]),
      "R-F5's first activation keeps the pre-stamp generation it replaces",
    ).toEqual([CACHE_PREFIX + "older"]);
  });

  // **`activate` retains the PREVIOUS generation** (spec §2.3), because an open
  // tab still running the old bundle asks for its own hashed module by name.
  // This is the catcher for the battery's "activate deleting the previous
  // generation".
  it("keeps mine and the newest other, and deletes the rest", () => {
    const out = generationsToDelete(mine, [
      g("new", 300),
      g("prev", 200),
      g("older", 100),
      g("oldest", 50),
    ]);
    expect(out).toEqual([CACHE_PREFIX + "older", CACHE_PREFIX + "oldest"]);
  });

  it("keeps mine even when it is the oldest thing there", () => {
    const out = generationsToDelete(mine, [g("new", 1), g("prev", 900)]);
    expect(out).toEqual([]);
  });

  it("deletes nothing when there is only one other, or none at all", () => {
    expect(generationsToDelete(mine, [g("new", 3), g("prev", 2)])).toEqual([]);
    expect(generationsToDelete(mine, [g("new", 3)])).toEqual([]);
    expect(generationsToDelete(mine, [])).toEqual([]);
  });

  // **It never names a cache this client does not own.** The Cache Storage of
  // an origin is shared, and a sweep that deleted by "everything that is not
  // mine" would take out anything else the origin ever put there.
  it("touches nothing outside its own prefix", () => {
    const out = generationsToDelete(mine, [
      g("new", 300),
      g("prev", 200),
      g("older", 100),
      { name: "someone-elses-cache", createdAt: 1 },
      { name: "workbox-precache", createdAt: 2 },
    ]);
    expect(out).toEqual([CACHE_PREFIX + "older"]);
  });

  // A generation written before this scheme carries no stamp; it reads as the
  // oldest, which is what it is.
  it("reads a stampless generation as the oldest", () => {
    const out = generationsToDelete(mine, [g("new", 300), g("nostamp", 0), g("prev", 200)]);
    expect(out).toEqual([CACHE_PREFIX + "nostamp"]);
  });

  // Deterministic on a tie, so two runs of the sweep agree about which one
  // survived.
  it("breaks a tie by name rather than by listing order", () => {
    const a = generationsToDelete(mine, [g("new", 9), g("aaa", 5), g("bbb", 5)]);
    const b = generationsToDelete(mine, [g("new", 9), g("bbb", 5), g("aaa", 5)]);
    expect(a).toEqual(b);
    expect(a.length).toBe(1);
  });
});

// **Which caches the fetch handler may read a path this release does not list
// from** (PR #37 F2, R-F2). The tab that did not click is handed to the new
// worker still running the old bundle, and asks for the old names; the retained
// generation is where they are. `sw.node.test.ts` drives the worker over the
// same rule — this is the rule by itself.
describe("which generations a path this release does not list may be read from", () => {
  const mine = CACHE_PREFIX + "new";

  it("every generation of ours except this release's own, in the order they were listed", () => {
    expect(
      retainedGenerations(mine, [CACHE_PREFIX + "prev", mine, CACHE_PREFIX + "older"]),
    ).toEqual([CACHE_PREFIX + "prev", CACHE_PREFIX + "older"]);
    expect(retainedGenerations(mine, [mine])).toEqual([]);
    expect(retainedGenerations(mine, [])).toEqual([]);
  });

  // **Never a cache outside the prefix.** The Cache Storage of an origin is
  // shared, so "every cache that is not mine" would read whatever else the
  // origin ever put there — and serve it as if a release had verified it.
  it("never a cache outside its own prefix", () => {
    expect(
      retainedGenerations(mine, ["someone-elses-cache", CACHE_PREFIX + "prev", "workbox-precache"]),
    ).toEqual([CACHE_PREFIX + "prev"]);
  });
});
