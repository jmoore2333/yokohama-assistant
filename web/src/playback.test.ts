// The playback cadence: three settings, remembered per browser, the default
// when storage is absent, throwing or holding nonsense (the `sections.ts` idiom).

import { describe, expect, it } from "vitest";
import { CADENCES, DEFAULT_CADENCE, loadCadence, QUICK_DWELL, saveCadence } from "./playback";

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

describe("the cadence", () => {
  it("is one, two or three seconds, two by default, with a short dwell for an unchanged frame", () => {
    expect(CADENCES).toEqual([1000, 2000, 3000]);
    expect(DEFAULT_CADENCE).toBe(2000);
    expect(QUICK_DWELL).toBe(400);
    expect(loadCadence(fakeStorage())).toBe(2000);
  });
  it("round-trips through storage", () => {
    const s = fakeStorage();
    saveCadence(3000, s);
    expect(loadCadence(s)).toBe(3000);
    expect(s.getItem("yokohama.playback.cadence")).toBe("3000");
  });
  it("reads the default when storage throws or holds a value that is not a cadence", () => {
    expect(loadCadence(fakeStorage(undefined, true))).toBe(2000);
    expect(loadCadence(fakeStorage("1500"))).toBe(2000);
    expect(loadCadence(fakeStorage("fast"))).toBe(2000);
    expect(() => saveCadence(1000, fakeStorage(undefined, true))).not.toThrow();
  });
});
