// The name step's rule, pinned to the server's (M6.5c2, spec §2.1 and §3).
//
// The table is the LAYER's verdicts — `User::parse` over every row, emitted by
// `wire_fixtures` into `web/src/wire/identity_cases.json` and refused when it is
// stale — so this cell holds the client's copy to the server's both ways: an
// input the server keys, the client keys identically; an input the server
// refuses, the client refuses before any request is made.

import { describe, expect, it } from "vitest";
import { ADDRESS_NOT_A_NAME_HERE, NAME_MAX, WHITE, normalizeName, openDoorVerdict } from "./nameRule";
import { wire } from "./wire/contract";

const cases = wire.identityCases;

/** An input as a reader can see it: every character outside printable ASCII
 *  written as its code-point escape, so a red names the U+0085 row by its
 *  escape rather than as an invisible "ben" (review M6). */
const shown = (s: string): string =>
  `"${[...s]
    .map((c) => {
      const cp = c.codePointAt(0)!;
      if (cp >= 0x20 && cp <= 0x7e) return c;
      return cp > 0xffff ? `\\u{${cp.toString(16)}}` : `\\u${cp.toString(16).padStart(4, "0")}`;
    })
    .join("")}"`;

/** The inputs the rule turns on — each a place where a JavaScript built-in
 *  and Rust disagree, or a refusal the archive's containment rests on (spec
 *  §2.1, review H1). **The committed table's `raw` column, row for row** (the
 *  two length rows spelled from `NAME_MAX` rather than copied), so the table
 *  must carry every one, or the pin proves nothing about them. Every code
 *  point outside ASCII is written as an escape, never as the character. */
const REQUIRED = [
  "red@wire.test", "Red@Wire.TEST", " Red@Wire.TEST ", "a+b@x.test",
  "ben castle", "  Ben   Castle ", "Ben", "BEN", "b3n_c-a.stle", "a.test", "7 wonders", "x",
  "a".repeat(NAME_MAX), "a".repeat(NAME_MAX + 1),
  ".", "..", ". .", "-", ".x", "_x", "x.",
  "anonymous", "ANONYMOUS", "anonymous1",
  "", "   ", "B\u00e9n", "ben!", "a/b", "\uff42\uff45\uff4e", "\u212aelvin",
  "a@b@c", "@x.test", "a@", "a b@x.test",
  "ben\t", "ben\tcastle", "ben\r", "\u00a0Ben", "\u0085ben", "\ufeffben",
];

describe("the name rule", () => {
  it("gives the layer's own verdict on every input the layer judged", () => {
    for (const c of cases) {
      const got = normalizeName(c.raw);
      if (c.key === null) expect(got.ok, `${shown(c.raw)} is refused by the server`).toBe(false);
      else expect(got, shown(c.raw)).toEqual({ ok: true, key: c.key, display: c.display });
    }
  });

  it("is pinned on the inputs the rule turns on, so the cell above is not vacuous", () => {
    const raws = new Set(cases.map((c) => c.raw));
    for (const raw of REQUIRED) {
      expect(raws.has(raw), `identity_cases.json lacks ${shown(raw)}`).toBe(true);
    }
    expect(cases.some((c) => c.key !== null && c.key.includes("@")), "an accepted address").toBe(true);
    expect(cases.some((c) => c.key !== null && !c.key.includes("@")), "an accepted plain name").toBe(true);
    expect(cases.some((c) => c.key === null), "a refusal").toBe(true);
  });

  // **The whitespace set, pinned to the property it restates** (review I1).
  // Rust's `str::trim` and `char::is_whitespace` read the Unicode White_Space
  // property, and so does JavaScript's `\p{White_Space}` — one published list.
  // The table above has rows for only two of its members, so without this cell
  // the rest of `WHITE`, most of it invisible, could gain or lose a member in
  // silence. Both directions, over every code point in the BMP (the property
  // has no member outside it).
  it("trims exactly the White_Space property, no more and no less, over every BMP code point", () => {
    const inWhite = new RegExp(`^[${WHITE}]$`);
    const property = /^\p{White_Space}$/u;
    const gained: string[] = [];
    const lost: string[] = [];
    let members = 0;
    for (let cp = 0; cp <= 0xffff; cp += 1) {
      const ch = String.fromCharCode(cp);
      const mine = inWhite.test(ch);
      const theirs = property.test(ch);
      if (mine && theirs) members += 1;
      if (mine && !theirs) gained.push(shown(ch));
      if (theirs && !mine) lost.push(shown(ch));
    }
    expect(gained, "in WHITE, and not White_Space").toEqual([]);
    expect(lost, "White_Space, and not in WHITE").toEqual([]);
    // Anti-vacuity: the two agree on real members, not on an empty set.
    expect(members, "White_Space members WHITE holds").toBeGreaterThan(20);
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
      if (v.ok) throw new Error(`${shown(raw)} was accepted`);
      return v.reason;
    };
    expect(why("")).toBe("Type a name to play under.");
    expect(why("..")).toBe("A name begins with a letter or a digit.");
    expect(why("a".repeat(NAME_MAX + 1))).toBe(`A name is at most ${NAME_MAX} characters.`);
    expect(why("ANONYMOUS")).toMatch(/means nobody/);
    expect(why("b\u00e9n")).toMatch(/letters, numbers, spaces/);
    expect(why("a@b@c")).toMatch(/exactly one @/);
  });
});

describe("openDoorVerdict (security S2b, C1; decision D-2)", () => {
  // Mutants: the `@` test dropped (every address row admitted); the test on the KEY rather than the raw input (a
  // raw `@` that fails the grammar would get the grammar's sentence about writing an address, at a door that refuses
  // every address — AD-8 (d)).
  it("refuses every committed row whose raw input holds an @, in the door's own words, and agrees with normalizeName on every other row", () => {
    const withAt = cases.filter((c) => c.raw.includes("@"));
    expect(withAt.length).toBeGreaterThanOrEqual(4);
    for (const c of withAt) expect(openDoorVerdict(c.raw), shown(c.raw)).toEqual({ ok: false, reason: ADDRESS_NOT_A_NAME_HERE });
    for (const c of cases.filter((c) => !c.raw.includes("@"))) expect(openDoorVerdict(c.raw), shown(c.raw)).toEqual(normalizeName(c.raw));
  });
});
