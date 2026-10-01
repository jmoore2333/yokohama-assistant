// The seed check the new-table form runs AS A SEED IS TYPED (M6.5c2, spec
// §2.2 "The seed"; plan Review Focus 2), held to the SERVER's rule.
//
// **One table, two readers.** `wire/seed_cases.json` is `WireSeed`'s own verdict
// on every input, emitted by `wire_fixtures.rs` and refused there when the rule
// moves; this file asserts `checkSeed` gives each one. Neither side can drift
// while both believe they agree — `site_space_words.json`'s shape, applied to a
// rule the client has to run itself.

import { describe, expect, it } from "vitest";
import { checkSeed, trimSeed } from "./seedRule";
import { wire } from "./wire/contract";

describe("the seed rule", () => {
  it("gives every verdict the server gives, input by input", () => {
    // Anti-vacuity: a table of one verdict would be satisfied by a constant.
    expect(wire.seedCases.some((c) => c.ok)).toBe(true);
    expect(wire.seedCases.some((c) => !c.ok)).toBe(true);
    for (const c of wire.seedCases) {
      expect(checkSeed(c.raw).ok, `checkSeed(${JSON.stringify(c.raw)})`).toBe(c.ok);
    }
  });

  // **Review Focus 2's inputs, by name**, so a table that quietly lost one fails
  // here rather than leaving that case pinned by nothing.
  it("carries the inputs the review named", () => {
    const raws = wire.seedCases.map((c) => c.raw);
    for (const raw of [
      "+007 ", " 7", "007", "\uFF17", "1e3", "-1",
      "18446744073709551616", "18446744073709551615", "", " ",
    ]) {
      expect(raws, `${JSON.stringify(raw)} is in the committed table`).toContain(raw);
    }
  });

  // **The trim is Rust's, not JavaScript's.** `String.prototype.trim` strips
  // U+FEFF and keeps U+0085; `str::trim` does the opposite. A rule written on
  // the obvious trim fails this cell and the table's two rows for them.
  it("trims the way the server trims", () => {
    expect("\uFEFF12".trim(), "the premise: JavaScript strips U+FEFF").toBe("12");
    expect(trimSeed("\uFEFF12")).toBe("\uFEFF12");
    expect("\u008512".trim(), "the premise: JavaScript keeps U+0085").toBe("\u008512");
    expect(trimSeed("\u008512")).toBe("12");
  });

  it("says which way a seed is wrong, in the words the fold shows", () => {
    expect(checkSeed("18446744073709551616")).toEqual({
      ok: false,
      reason:
        "Too large: a seed is a whole number from 0 to 18446744073709551615. Leave it blank to have the server draw one.",
    });
    expect(checkSeed("1e3")).toEqual({
      ok: false,
      reason:
        "Not a whole number: a seed is a whole number from 0 to 18446744073709551615. Leave it blank to have the server draw one.",
    });
    expect(checkSeed(" +007 ")).toEqual({ ok: true });
  });
});
