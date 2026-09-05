// The playback cadence: three settings, remembered per browser, the default
// when storage is absent, throwing or holding nonsense (the `sections.ts` idiom).

import { describe, expect, it } from "vitest";
import { CADENCES, DEFAULT_CADENCE, loadCadence, QUICK_DWELL, saveCadence } from "./playback";

/** The module's own private key, RESTATED here rather than imported — that is
 *  the pin. Rename the key in `playback.ts` and the round-trip cell below
 *  fails, which is the point; an imported constant would move with it and
 *  assert nothing. */
const KEY = "yokohama.playback.cadence";

/** Keyed, because a fake that ignores its key makes the assertion on WHERE the
 *  cadence is stored decoration: the module could write under any name and the
 *  round-trip would still read it back. */
function fakeStorage(initial?: string, throwing = false): Storage {
  const cells = new Map<string, string>();
  if (initial !== undefined) cells.set(KEY, initial);
  return {
    getItem: (k: string) => {
      if (throwing) throw new Error("no storage here");
      return cells.get(k) ?? null;
    },
    setItem: (k: string, v: string) => {
      if (throwing) throw new Error("no storage here");
      cells.set(k, v);
    },
  } as unknown as Storage;
}

describe("the cadence", () => {
  it("is one, two or three seconds, two by default, with a short dwell for an unchanged frame", () => {
    expect(CADENCES).toEqual([1000, 2000, 3000]);
    expect(DEFAULT_CADENCE).toBe(2000);
    expect(QUICK_DWELL).toBe(400);
    expect(loadCadence(fakeStorage())).toBe(2000);
  });
  it("round-trips through storage, under that key and no other", () => {
    const s = fakeStorage();
    saveCadence(3000, s);
    expect(loadCadence(s)).toBe(3000);
    expect(s.getItem(KEY)).toBe("3000");
    // The inverse: the fake is key-SENSITIVE, so the line above is a real
    // reading of where the module wrote and not of what it was handed.
    expect(s.getItem("some.other.key")).toBeNull();
  });
  it("reads the default when storage throws or holds a value that is not a cadence", () => {
    expect(loadCadence(fakeStorage(undefined, true))).toBe(2000);
    expect(loadCadence(fakeStorage("1500"))).toBe(2000);
    expect(loadCadence(fakeStorage("fast"))).toBe(2000);
    expect(() => saveCadence(1000, fakeStorage(undefined, true))).not.toThrow();
  });
});
