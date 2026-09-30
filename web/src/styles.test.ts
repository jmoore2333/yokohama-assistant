// The stylesheet's own rules, read from the source: jsdom loads no stylesheet,
// so a rule about a declaration is a rule about the file. Read the way
// `BoardMark.test.tsx` and `FrontDoor.test.tsx` read it — vitest runs from
// `web/`.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const CSS = readFileSync("src/styles.css", "utf8");

/** The file with every comment blanked and every newline kept, so a line
 *  number still names the line — a comment may say `1lh` freely. */
const code = CSS.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, " ")).split("\n");

/** The `property: value` declarations on one line, in order. */
const declarations = (line: string): { property: string; value: string }[] =>
  line
    .replace(/^[^{]*\{/, "")
    .split(";")
    .map((d) => d.replace(/}.*$/, "").trim())
    .filter((d) => /^[a-z-]+\s*:/.test(d))
    .map((d) => {
      const i = d.indexOf(":");
      return { property: d.slice(0, i).trim(), value: d.slice(i + 1).trim() };
    });

// **A browser that does not know a unit drops the whole declaration** — and
// keeps whatever the same property was given before it. Safari before 16.4
// has no `lh`, so a `1lh` declaration with nothing ahead of it leaves the box
// with no padding at all there (cycle E, straw poll row 66; the C close's
// §6.2). The fallback is the same property in `em` from the line-height the
// element inherits (the body's 1.5), declared on one of the two lines above.
describe("the 1lh declarations", () => {
  const uses = code.flatMap((line, i) =>
    declarations(line)
      .filter((d) => /(^|[^a-z0-9.-])1lh\b/.test(d.value))
      .map((d) => ({ line: i + 1, ...d })),
  );

  it("finds the declarations it is about", () => {
    // Anti-vacuity: the two thumb-size rules at 600px (padding and margin).
    expect(uses.map((u) => u.property).sort()).toEqual(["margin-block", "padding-block"]);
  });

  it("gives every 1lh declaration a line-height fallback on one of the two lines before it", () => {
    for (const use of uses) {
      const before = [use.line - 2, use.line - 1]
        .filter((n) => n >= 1)
        .flatMap((n) => declarations(code[n - 1]!))
        .filter((d) => d.property === use.property && !/\blh\b|\dlh\b/.test(d.value));
      expect(before, `${use.property} at styles.css:${use.line} has no fallback above it`).not.toEqual([]);
      // The fallback is the same box in the inherited line-height's terms.
      expect(before.at(-1)!.value, `styles.css:${use.line - 1}`).toBe(use.value.replace("1lh", "1.5em"));
    }
  });
});

// **The version tag inside a seat's name is outlined in the door's muted line**
// (cycle E, straw poll row 61): the `¥` chip beside it is the yen's gold, and a
// gold box beside a gold chip read as a second amount. The rest of the tag —
// its face, its size — is the tag's own everywhere.
describe("the version tag in a seat's name", () => {
  it("takes the muted outline and never the yen's gold", () => {
    const rule = CSS.match(/\.seat-name \.version-tag\s*\{([^}]*)\}/);
    expect(rule, "a rule for the tag inside a seat's name").not.toBeNull();
    expect(rule![1]).toMatch(/color:\s*var\(--fg-dim\)/);
    expect(rule![1]).toMatch(/border-color:\s*var\(--line-bright\)/);
    expect(rule![1]).not.toMatch(/--yen/);
  });
});

// **The who-strip wraps at every width, and "sign out" never breaks** (cycle E,
// straw poll row 72): the wrap was phone-only (480px and under), which left
// M6.4's band — at 500px on the deal's waiting face a 38-character address
// took two lines of its own box and "sign out" two more. A rule inside a media
// query would bring the band back; so it is read at the top level.
describe("the who-strip", () => {
  /** The file's top-level rules only: a rule inside `@media` is not one. */
  const topLevel = (() => {
    const text = CSS.replace(/\/\*[\s\S]*?\*\//g, "");
    const rules: { selectors: string[]; body: string }[] = [];
    let depth = 0;
    let start = 0;
    let head = "";
    for (let i = 0; i < text.length; i++) {
      if (text[i] === "{") {
        if (depth === 0) {
          head = text.slice(start, i).trim();
          start = i + 1;
        }
        depth++;
      } else if (text[i] === "}") {
        depth--;
        if (depth === 0) {
          if (!head.startsWith("@")) rules.push({ selectors: head.split(",").map((x) => x.trim()), body: text.slice(start, i) });
          start = i + 1;
        }
      }
    }
    return rules;
  })();
  const bodies = (selector: string) => topLevel.filter((r) => r.selectors.includes(selector)).map((r) => r.body).join(";");

  it("wraps at every width, its button whole", () => {
    expect(bodies(".who-strip")).toMatch(/flex-wrap:\s*wrap/);
    expect(bodies(".who-strip button")).toMatch(/white-space:\s*nowrap/);
  });
});
