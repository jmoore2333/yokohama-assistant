// Which sections the table draws, and where that choice is kept.

import { describe, expect, it } from "vitest";
import { DEFAULT_SECTIONS, loadSections, saveSections, SECTION_KEYS, SECTION_LABELS } from "./sections";

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

describe("the enabled sections", () => {
  it("default to the port, the laboratory, the tracks and the goals", () => {
    expect(loadSections(fakeStorage())).toEqual(DEFAULT_SECTIONS);
    expect(DEFAULT_SECTIONS).toEqual(["ports", "labs", "tracks", "achievements"]);
  });
  it("round-trip through storage, in the registry's order and dropping unknown keys", () => {
    const s = fakeStorage();
    saveSections(["buildings", "ports"], s);
    expect(loadSections(s)).toEqual(["ports", "buildings"]);
    saveSections([], s);
    expect(loadSections(s)).toEqual([]);
    s.setItem("yokohama.table.sections", JSON.stringify(["ports", "nonsense"]));
    expect(loadSections(s)).toEqual(["ports"]);
  });
  it("fall back to the default when storage throws or holds garbage", () => {
    expect(loadSections(fakeStorage(undefined, true))).toEqual(DEFAULT_SECTIONS);
    expect(loadSections(fakeStorage("not json"))).toEqual(DEFAULT_SECTIONS);
    expect(loadSections(fakeStorage('{"a":1}'))).toEqual(DEFAULT_SECTIONS);
    expect(() => saveSections(["ports"], fakeStorage(undefined, true))).not.toThrow();
  });
  // **A label for every key and no more.** The cell here before this asserted
  // that every key was a `string`, over a `const` tuple of string literals —
  // true by the type, unfailable by construction. What can actually go wrong is
  // a key added to the registry with no label, which renders `undefined` as a
  // section heading and as a Boards toggle.
  it("label every key, and label nothing else", () => {
    expect(Object.keys(SECTION_LABELS).sort()).toEqual([...SECTION_KEYS].sort());
    for (const k of SECTION_KEYS) expect(SECTION_LABELS[k]).toBeTruthy();
  });
});
