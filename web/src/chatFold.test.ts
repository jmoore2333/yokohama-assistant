// Which chat panels this browser has opened, and where that choice is kept
// (M6.4, R-6.4.10). The panel's own cells drive it through the component; these
// drive the module, which is the only place the storage failures are reachable.

import { describe, expect, it } from "vitest";
import { loadFolded, OPEN_KEY, saveFolded } from "./chatFold";

/** `sections.test.ts`'s own fake, which is the idiom this module copies — one
 *  key holding a set, a default that survives a storage that will not answer. */
function fakeStorage(initial?: string, throwing = false): Storage {
  let value = initial;
  return {
    getItem: () => {
      if (throwing) throw new Error("no storage here");
      return value ?? null;
    },
    setItem: (_k: string, v: string) => {
      if (throwing) throw new Error("no storage here");
      value = v;
    },
  } as unknown as Storage;
}

describe("the remembered chat folds", () => {
  it("default to folded, and remember an opened scope on one key", () => {
    const s = fakeStorage();
    expect(loadFolded("wire", s)).toBe(true);
    saveFolded("wire", false, s);
    expect(loadFolded("wire", s)).toBe(false);
    expect(loadFolded("house", s), "a scope nobody opened is still folded").toBe(true);
    // ONE key, holding a set — never a key per table.
    expect(JSON.parse(s.getItem(OPEN_KEY)!)).toEqual(["wire"]);
    saveFolded("wire", true, s);
    expect(loadFolded("wire", s)).toBe(true);
    expect(JSON.parse(s.getItem(OPEN_KEY)!)).toEqual([]);
  });

  // **Folded is the default everywhere, so a storage that will not answer loses
  // nothing** (spec §9.3). The panel is a bonus; the worst a refused read can
  // do is shut a panel somebody had opened, and the worst a refused WRITE can
  // do is forget that they opened it — neither is worth an exception reaching
  // a render.
  it("reads folded and swallows the write when storage throws", () => {
    expect(loadFolded("wire", fakeStorage(undefined, true))).toBe(true);
    expect(() => saveFolded("wire", false, fakeStorage(undefined, true))).not.toThrow();
    // And the same for a browser with no storage at all.
    expect(loadFolded("wire", null)).toBe(true);
    expect(() => saveFolded("wire", false, null)).not.toThrow();
  });

  it("reads folded over garbage rather than throwing on it", () => {
    for (const held of ["not json", '{"a":1}', '["wire", 3, null]']) {
      const s = fakeStorage(held);
      expect(loadFolded("house", s), held).toBe(true);
    }
    // The one that is not garbage: a list carrying a non-string beside a real
    // scope keeps the scope.
    expect(loadFolded("wire", fakeStorage('["wire", 3]'))).toBe(false);
  });

  // **The set is capped, newest kept**, so a browser that visits a thousand
  // tables does not carry a thousand ids on one key for ever.
  it("keeps the newest 64 scopes", () => {
    const s = fakeStorage();
    for (let i = 0; i < 70; i += 1) saveFolded(`t${i}`, false, s);
    const kept: string[] = JSON.parse(s.getItem(OPEN_KEY)!);
    expect(kept).toHaveLength(64);
    expect(kept[0]).toBe("t6");
    expect(kept.at(-1)).toBe("t69");
    expect(loadFolded("t0", s), "the oldest was dropped and reads folded again").toBe(true);
    expect(loadFolded("t69", s)).toBe(false);
  });
});
