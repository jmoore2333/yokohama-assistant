// The name step's rule, pinned to the server's (M6.5c2, spec §2.1 and §3).
//
// The table is the LAYER's verdicts — `User::parse` over every row, emitted by
// `wire_fixtures` into `web/src/wire/identity_cases.json` and refused when it is
// stale — so this cell holds the client's copy to the server's both ways: an
// input the server keys, the client keys identically; an input the server
// refuses, the client refuses before any request is made.

import { describe, expect, it } from "vitest";
import { NAME_MAX, normalizeName } from "./nameRule";
import { wire } from "./wire/contract";

const cases = wire.identityCases;

/** The inputs the rule turns on — each a place where a JavaScript built-in
 *  and Rust disagree, or a refusal the archive's containment rests on (spec
 *  §2.1, review H1). **Copied from the committed table's `raw` column**, so the
 *  table must carry every one, or the pin proves nothing about them. Every code
 *  point outside ASCII is written as an escape, never as the character. */
const REQUIRED = [
  "red@wire.test", "Red@Wire.TEST", " Red@Wire.TEST ", "a+b@x.test",
  "ben castle", "  Ben   Castle ", "Ben", "BEN", "b3n_c-a.stle", "a.test", "7 wonders", "x",
  "a".repeat(NAME_MAX), "a".repeat(NAME_MAX + 1),
  ".", "..", ". .", "-", ".x", "_x", "x.",
  "anonymous", "ANONYMOUS", "anonymous1",
  "", "   ", "Bén", "ben!", "a/b", "ｂｅｎ", "Kelvin",
  "a@b@c", "@x.test", "a@", "a b@x.test",
  "ben\t", "ben\tcastle", "ben\r", " Ben", "\u0085ben", "﻿ben",
];

describe("the name rule", () => {
  it("gives the layer's own verdict on every input the layer judged", () => {
    for (const c of cases) {
      const got = normalizeName(c.raw);
      if (c.key === null) expect(got.ok, `${JSON.stringify(c.raw)} is refused by the server`).toBe(false);
      else expect(got, JSON.stringify(c.raw)).toEqual({ ok: true, key: c.key, display: c.display });
    }
  });

  it("is pinned on the inputs the rule turns on, so the cell above is not vacuous", () => {
    const raws = new Set(cases.map((c) => c.raw));
    for (const raw of REQUIRED) {
      expect(raws.has(raw), `identity_cases.json lacks ${JSON.stringify(raw)}`).toBe(true);
    }
    expect(cases.some((c) => c.key !== null && c.key.includes("@")), "an accepted address").toBe(true);
    expect(cases.some((c) => c.key !== null && !c.key.includes("@")), "an accepted plain name").toBe(true);
    expect(cases.some((c) => c.key === null), "a refusal").toBe(true);
  });

  // **Review Focus 1, the display half** (the key half is the layer's, in
  // `identity.rs`): one key, shown as typed.
  it("keys a name typed with capitals and extra spaces once, and shows it as typed (Review Focus 1)", () => {
    expect(normalizeName("  Ben   Castle ")).toEqual({ ok: true, key: "ben castle", display: "Ben Castle" });
    expect(normalizeName("ben castle")).toEqual({ ok: true, key: "ben castle", display: "ben castle" });
  });

  it("says why, in words a player can act on", () => {
    const why = (raw: string): string => {
      const v = normalizeName(raw);
      if (v.ok) throw new Error(`${JSON.stringify(raw)} was accepted`);
      return v.reason;
    };
    expect(why("")).toBe("Type a name to play under.");
    expect(why("..")).toBe("A name begins with a letter or a digit.");
    expect(why("a".repeat(NAME_MAX + 1))).toBe(`A name is at most ${NAME_MAX} characters.`);
    expect(why("ANONYMOUS")).toMatch(/means nobody/);
    expect(why("bén")).toMatch(/letters, numbers, spaces/);
    expect(why("a@b@c")).toMatch(/exactly one @/);
  });
});
