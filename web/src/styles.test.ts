// The stylesheet's own rules, read from the source: jsdom loads no stylesheet,
// so a rule about a declaration is a rule about the file. Read the way
// `BoardMark.test.tsx` and `FrontDoor.test.tsx` read it — vitest runs from
// `web/`.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { topLevelRules } from "./test/css";

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
  const topLevel = topLevelRules(CSS);
  const bodies = (selector: string) => topLevel.filter((r) => r.selectors.includes(selector)).map((r) => r.body).join(";");

  it("wraps at every width, its button whole", () => {
    expect(bodies(".who-strip")).toMatch(/flex-wrap:\s*wrap/);
    expect(bodies(".who-strip button")).toMatch(/white-space:\s*nowrap/);
  });
});

/** Every rule in the file — inside `@media` blocks too — as its selector text
 *  and body: the innermost `{…}` pairs, comments stripped. */
const everyRule = (): { selector: string; body: string }[] =>
  [...CSS.replace(/\/\*[\s\S]*?\*\//g, "").matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({
    selector: m[1]!.trim(),
    body: m[2]!,
  }));

// **The phone band's grid is for a band whose corner holds something** (Task
// 4b's review, Important 1): the title and the pill on one line and the path
// under both is row 60's rule, and the three screens that pass no corner — the
// form for a game on this device, the lock screen, the broken store — have no
// pill, so their one crumb keeps the title's line. `:has()` reads what the
// corner actually drew, so a corner handed in but empty counts as none.
describe("the band on a phone", () => {
  const SCOPE = ".lobby-band:has(> .band-corner:not(:empty))";

  it("makes a grid only of a band whose corner holds something", () => {
    const grids = everyRule().filter((r) => /\.lobby-band\b/.test(r.selector) && /display:\s*grid/.test(r.body));
    expect(grids.length, "the sweep found the band's grid").toBeGreaterThan(0);
    for (const g of grids) expect(g.selector, "scoped to a filled corner").toBe(SCOPE);
  });

  it("moves the title, the corner and the path only inside that scope", () => {
    const placed = everyRule().filter(
      (r) => /(band-crumbs|band-corner|band-brand|\bh1\b)/.test(r.selector) && /grid-(row|column)\s*:/.test(r.body),
    );
    expect(placed.length, "the sweep found the placements").toBeGreaterThanOrEqual(3);
    for (const p of placed) expect(p.selector.startsWith(SCOPE), p.selector).toBe(true);
  });
});

// **What gives in the banner from 1000px to 1200px is the game's id alone**
// (ruling 4b-2): the ellipsis is the id's element's, and the summary never
// breaks a line, so the mode and "saved" — the save state — stay whole.
describe("the Game menu's summary", () => {
  it("ends the id in an ellipsis and keeps the rest of the summary whole", () => {
    const id = everyRule().find((r) => r.selector === ".game-menu-id");
    expect(id, "a rule for the id's element").toBeDefined();
    expect(id!.body).toMatch(/text-overflow:\s*ellipsis/);
    expect(id!.body).toMatch(/overflow:\s*hidden/);
    expect(id!.body).toMatch(/min-width:\s*0/);
    const summary = everyRule().find((r) => r.selector === ".game-menu > summary");
    expect(summary?.body).toMatch(/white-space:\s*nowrap/);
    // Nothing else in the summary is given an ellipsis.
    const others = everyRule().filter((r) => /game-menu/.test(r.selector) && r.selector !== ".game-menu-id" && /ellipsis/.test(r.body));
    expect(others.map((r) => r.selector)).toEqual([]);
  });
});
