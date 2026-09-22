// The precache generator's pure half (M6.5b2, spec §2.3 and §2.7).
//
// **The fixture is synthetic and that is the KNOWN limit of this file.** A
// manifest-shaped fixture agrees with a manifest-shaped bug, which is the whole
// finding spec §2.3 is built on — so the cell that matters most is not here:
// `precache.node.test.ts` runs a REAL `vite build` into a temporary directory
// and inventories what came out. These cells pin the arithmetic; that one pins
// the inventory.

import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  artFilesFrom,
  buildPrecache,
  CACHE_PREFIX,
  checkSubstitution,
  ceilingsFor,
  DEBUG_MODULE_CEILING,
  DEBUG_MODULE_MARKER,
  expectedType,
  RELEASE_MODULE_CEILING,
  RELEASE_TOTAL_CEILING,
} from "./precache";

const sha256Hex = (data: Uint8Array | string) => createHash("sha256").update(data).digest("hex");

/** A release in miniature: a shell, a bundle, a stylesheet, a Worker chunk, a
 *  module, the web app manifest, an icon — and two pictures from an art
 *  manifest shaped like the real one's groups. */
const fixture = {
  distFiles: [
    "index.html",
    "manifest.webmanifest",
    "icons/icon-192.png",
    "assets/index-abc.js",
    "assets/index-abc.css",
    "assets/worker-def.js",
    "assets/yokohama_wasm_bg-ghi.wasm",
  ],
  artManifest: {
    orders: { "1": { file: "orders/a.webp", w: 530, h: 799 } },
    techs: { Ball: { file: "techs/b.webp", w: 530, h: 799 } },
  },
  readFile: (p: string): Uint8Array | string => `the bytes of ${p}`,
  sha256Hex,
  // Generous on purpose: the ceiling cells below hand their own.
  moduleCeiling: RELEASE_MODULE_CEILING,
  totalCeiling: 100_000_000,
};

describe("the precache list", () => {
  // spec §2.3: the cache name IS the content, so nothing has to be bumped.
  it("changes the cache name when any file's bytes change", () => {
    const read = (p: string) => (p.endsWith("a.webp") ? "AAA" : "BBB");
    const a = buildPrecache({ ...fixture, readFile: read });
    const b = buildPrecache({ ...fixture, readFile: (p) => (p.endsWith("a.webp") ? "AAA!" : "BBB") });
    expect(a.cacheName).not.toEqual(b.cacheName);
  });

  // And it is DETERMINISTIC, or every build would invalidate every browser.
  it("gives the same name for the same inputs", () => {
    expect(buildPrecache(fixture).cacheName).toEqual(buildPrecache(fixture).cacheName);
  });

  // The art files have STABLE names (art.rs:112 serves them max-age=3600 for
  // exactly that reason), so a hashed-filename scheme alone would never reach a
  // browser that had already cached a re-cut card. This is the cell that proves
  // the digest earns its place.
  it("busts the cache for a re-cut card, whose filename never changes", () => {
    const before = buildPrecache({ ...fixture, readFile: () => "old card" });
    const after = buildPrecache({ ...fixture, readFile: () => "new card" });
    expect(before.entries.map((e) => e.path)).toEqual(after.entries.map((e) => e.path));
    expect(before.cacheName).not.toEqual(after.cacheName);
  });

  // Every art file the manifest names. The BUILD side is not checked here —
  // see the real-build cell below, which is the one that matters.
  it("lists every art file the manifest names", () => {
    const { entries } = buildPrecache(fixture);
    const paths = new Set(entries.map((e) => e.path));
    for (const f of Object.values(fixture.artManifest).flatMap((g) => Object.values(g)))
      expect(paths).toContain(`/art/${(f as { file: string }).file}`);
  });

  // spec §2.7: the budget lives HERE, in the thing that runs on every release
  // build, and not in a test someone might not run. The ceiling is a PARAMETER
  // because `npm run e2e` builds the DEBUG module and would otherwise fail this
  // on every run (Task 5).
  it("refuses a module over the ceiling it was given", () => {
    const big = { ...fixture, readFile: () => "x".repeat(2_000_000) };
    expect(() => buildPrecache({ ...big, moduleCeiling: 1_400_000 })).toThrow(/ceiling/);
    expect(() => buildPrecache({ ...big, moduleCeiling: 3_000_000 })).not.toThrow();
  });

  // The other ceiling, and it is a different claim: the module is what a phone
  // COMPILES, the total is what it DOWNLOADS. A release can be under one and
  // over the other in either direction.
  it("refuses a release over the total ceiling it was given", () => {
    const wide = { ...fixture, readFile: () => "y".repeat(500_000) };
    expect(() => buildPrecache({ ...wide, totalCeiling: 1_000_000 })).toThrow(/ceiling/);
    expect(() => buildPrecache({ ...wide, totalCeiling: 100_000_000 })).not.toThrow();
  });

  // The manifest itself is an entry, not merely the index of some: the door
  // reads `/art/manifest.json` as a static file (Task 2), so an offline door
  // that had every picture and not the list would draw nothing.
  it("lists the art manifest beside the art", () => {
    const paths = buildPrecache(fixture).entries.map((e) => e.path);
    expect(paths).toContain("/art/manifest.json");
    expect(paths).toContain("/index.html");
    expect(paths).toContain("/manifest.webmanifest");
  });

  // Sorted, and it is what makes the name above reproducible: a tree walk's
  // order is the filesystem's, which differs between two machines building the
  // same commit.
  it("is sorted, so two machines name the same release", () => {
    const paths = buildPrecache(fixture).entries.map((e) => e.path);
    expect(paths).toEqual([...paths].sort());
  });

  // The expected type travels WITH the entry, because the install checks it
  // against what the server answered.
  it("carries the media type the server will answer with", () => {
    const by = new Map(buildPrecache(fixture).entries.map((e) => [e.path, e.type]));
    expect(by.get("/assets/yokohama_wasm_bg-ghi.wasm")).toBe("application/wasm");
    expect(by.get("/index.html")).toBe("text/html");
    expect(by.get("/art/orders/a.webp")).toBe("image/webp");
    expect(by.get("/manifest.webmanifest")).toBe("application/manifest+json");
  });

  // The budget the generator PRINTS, so the line it writes on every build is
  // computed from the same walk rather than restated.
  it("reports the total, the module and the composition", () => {
    const out = buildPrecache(fixture);
    expect(out.totalBytes).toBe(
      out.entries.reduce((n, e) => n + `the bytes of ${e.path}`.length, 0),
    );
    expect(out.modulePath).toBe("/assets/yokohama_wasm_bg-ghi.wasm");
    expect(Object.values(out.byClass).reduce((a, b) => a + b, 0)).toBe(out.totalBytes);
    expect(Object.keys(out.byClass).sort()).toEqual(["art", "bundle", "icons", "module", "shell"]);
  });

  it("names the release with the shared prefix, so the sweep can find its own", () => {
    expect(buildPrecache(fixture).cacheName.startsWith(CACHE_PREFIX)).toBe(true);
  });
});

describe("what a release may not be", () => {
  // **The catcher for spec §6.5's named mutation** — "the precache list taken
  // from Vite's manifest instead of the tree, so the module is silently
  // absent". Vite's manifest names the bundle and the stylesheet and NOT the
  // module or the Worker chunk (measured, spec §2.3), so that swap produces
  // exactly this input and this refusal.
  it("refuses a release with no WebAssembly module in it", () => {
    const viteManifestShaped = fixture.distFiles.filter((f) => !f.endsWith(".wasm"));
    expect(() => buildPrecache({ ...fixture, distFiles: viteManifestShaped })).toThrow(
      /names no WebAssembly module/,
    );
  });

  it("refuses a release with no shell, no script, no stylesheet or no art", () => {
    const without = (drop: (f: string) => boolean) =>
      buildPrecache({ ...fixture, distFiles: fixture.distFiles.filter((f) => !drop(f)) });
    expect(() => without((f) => f === "index.html")).toThrow(/names no index\.html/);
    expect(() => without((f) => f.endsWith(".js"))).toThrow(/names no script/);
    expect(() => without((f) => f.endsWith(".css"))).toThrow(/names no stylesheet/);
    expect(() => buildPrecache({ ...fixture, artManifest: {} })).toThrow(/names no art/);
  });

  // A build that named a path its own worker never intercepts would be a
  // release disagreeing with `swRoute`. One rule, asked twice.
  it("refuses to list a path the worker bypasses", () => {
    for (const bad of ["api/health", "enter", "api/games/x/input"]) {
      expect(() => buildPrecache({ ...fixture, distFiles: [...fixture.distFiles, bad] })).toThrow(
        /never intercepts/,
      );
    }
  });

  it("refuses a path listed twice", () => {
    expect(() =>
      buildPrecache({ ...fixture, distFiles: [...fixture.distFiles, "index.html"] }),
    ).toThrow(/listed twice/);
  });
});

describe("the art manifest's own shape", () => {
  // The REAL manifest, because its groups are not uniform: `provenance` holds
  // scalars and `area_duplicates` is a list of objects with no `file` at all. A
  // reader written to the shape of one group throws on those or skips them —
  // and a reader that skipped them defensively would skip a real group the day
  // one grew a level.
  it("reads every picture out of the real manifest and nothing that is not one", () => {
    const raw = readFileSync("../crates/yokohama-web/assets/art/manifest.json", "utf8");
    const files = artFilesFrom(JSON.parse(raw));
    expect(files.length).toBe(131);
    expect(files.every((f) => f.endsWith(".webp"))).toBe(true);
    // Anti-vacuity: the two non-picture members really ARE in the document, so
    // their absence from the answer is a finding and not a tautology.
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    expect(Object.keys(parsed)).toContain("provenance");
    expect(Object.keys(parsed)).toContain("area_duplicates");
    expect(files).not.toContain("manifest.json");
  });
});

describe("the media types", () => {
  // **A cross-language pin.** `expectedType` is a mirror of
  // `assets::content_type`, and the install compares what it computed against
  // what the server answered — so a Rust arm this table does not have is an
  // install that fails on a file the server served correctly. Read the Rust and
  // compare arm for arm rather than trusting two lists to stay equal.
  it("agrees with assets.rs arm for arm, the default included", () => {
    const rust = readFileSync("../crates/yokohama-web/src/assets.rs", "utf8");
    const start = rust.indexOf("pub fn content_type");
    expect(start, "assets.rs no longer declares `pub fn content_type`").toBeGreaterThan(0);
    const body = rust.slice(start, rust.indexOf("\n}\n", start));
    const arms = [...body.matchAll(/((?:Some\("[a-z0-9]+"\)\s*\|?\s*)+)=>\s*"([^"]+)"/g)];
    expect(arms.length, "no match arms parsed out of assets.rs").toBeGreaterThan(8);
    for (const [, heads, type] of arms) {
      const essence = type!.split(";")[0]!.trim();
      for (const [, ext] of heads!.matchAll(/Some\("([a-z0-9]+)"\)/g)) {
        expect(expectedType(`x.${ext}`), `.${ext}`).toBe(essence);
      }
    }
    const fallback = /_\s*=>\s*"([^"]+)"/.exec(body);
    expect(fallback, "assets.rs has no default arm").not.toBeNull();
    // The DEFAULT, which is the half a list of arms cannot state: a path with
    // no arm at all goes out as `application/octet-stream`, and an install
    // expecting anything else would fail on a file the server served correctly.
    expect(expectedType("no-extension")).toBe(fallback![1]);
    // And `.txt` is no longer that case — the OFL copies under `dist/fonts/`
    // got an arm of their own in M6.5b2's fix wave, and the loop above is what
    // holds the two tables together across the change.
    expect(expectedType("OFL.txt")).toBe("text/plain");
  });
});

describe("which ceilings a build is judged by", () => {
  // **The marker's ABSENCE selects the release ceiling**, which is the half
  // that matters: a debug ceiling applied to a release build is a budget that
  // never fires.
  it("holds a release module to the release ceiling", () => {
    const out = ceilingsFor("no such export here");
    expect(out.label).toBe("release");
    expect(out.moduleCeiling).toBe(RELEASE_MODULE_CEILING);
    expect(out.totalCeiling).toBe(RELEASE_TOTAL_CEILING);
  });

  it("buys the debug module exactly its own slack and no more", () => {
    const debug = ceilingsFor(`exports ${DEBUG_MODULE_MARKER} for the trap cell`);
    expect(debug.label).toBe("debug");
    expect(debug.moduleCeiling).toBe(DEBUG_MODULE_CEILING);
    expect(debug.totalCeiling - RELEASE_TOTAL_CEILING).toBe(
      DEBUG_MODULE_CEILING - RELEASE_MODULE_CEILING,
    );
  });

  // The module is read as BYTES, never as text: it is a binary, and decoding it
  // as UTF-8 to run a string search is a corruption away from a wrong answer.
  it("finds the marker in the module's bytes", () => {
    const bytes = new TextEncoder().encode(`\u0000\u0001${DEBUG_MODULE_MARKER}\u0002`);
    expect(ceilingsFor(bytes).label).toBe("debug");
    expect(ceilingsFor(new TextEncoder().encode("\u0000\u0001plain\u0002")).label).toBe("release");
  });
});

// **Whether the built worker really carries the release** (review, Important 2).
//
// The first version of this guard looked for the cache name and for the literal
// `"/index.html"` — and `sw.ts` contains `"/index.html"` in its own source, at
// `fromCache("/index.html", request)`. So a RENAMED `__PRECACHE__` passed it and
// shipped a worker whose top-level `ENTRIES = __PRECACHE__` throws a
// `ReferenceError` before a single handler is registered.
describe("the substitution check", () => {
  const release = buildPrecache(fixture);
  /** What esbuild really emits: the define banner, the substituted list, and
   *  `sw.ts`'s own `"/index.html"`. */
  const built =
    `// <define:__PRECACHE__>\n` +
    `var define_PRECACHE_default = [${release.entries
      .map((e) => `{ path: ${JSON.stringify(e.path)}, digest: ${JSON.stringify(e.digest)}, type: ${JSON.stringify(e.type)} }`)
      .join(", ")}];\n` +
    `var CACHE_NAME = ${JSON.stringify(release.cacheName)};\n` +
    `fromCache("/index.html", request);\n`;

  it("accepts a worker the define really wrote the release into", () => {
    expect(checkSubstitution(built, release)).toBeNull();
  });

  // **The defect itself**, and the anti-vacuity half is what makes it a
  // finding: the text the check now refuses is one the OLD check accepted,
  // because both of its needles are still in it.
  it("refuses a worker whose list was never substituted", () => {
    const renamed = built.replace(/var define_PRECACHE_default = \[.*\];/s, "");
    expect(renamed).toContain('"/index.html"');
    expect(renamed).toContain(JSON.stringify(release.cacheName));
    expect(checkSubstitution(renamed, release)).toMatch(/digests/);
  });

  it("refuses a worker still carrying either placeholder", () => {
    expect(checkSubstitution(built + "\nENTRIES = __PRECACHE__;", release)).toMatch(/__PRECACHE__/);
    expect(checkSubstitution(built + "\nvar n = __CACHE_NAME__;", release)).toMatch(/__CACHE_NAME__/);
  });

  // The `// <define:NAME>` banner is esbuild's evidence that the define FIRED,
  // so it must not read as a leftover placeholder — which is the whole reason
  // those lines are dropped before the search rather than the search being
  // softened.
  it("does not mistake esbuild's own banner for a leftover placeholder", () => {
    expect(built).toContain("// <define:__PRECACHE__>");
    expect(checkSubstitution(built, release)).toBeNull();
  });

  it("refuses a worker that names a different release", () => {
    expect(checkSubstitution(built.replace(release.cacheName, "yokohama-release-elsewhere"), release))
      .toMatch(/does not name the release/);
  });

  it("refuses a worker carrying the wrong number of digests", () => {
    const short = built.replace(JSON.stringify(release.entries[0]!.digest), '"xx"');
    expect(checkSubstitution(short, release)).toMatch(/does not carry/);
  });
});

// ---- the licence that ships is the licence beside the font ----------------
//
// **`ASSETS.md` said the two copies "are checked identical when either is
// changed", and nothing checked them** (whole-branch review, false sentence 8;
// ledger Task 5 L150). The source copy sits beside the woff2 so a copy of that
// directory carries its own grant; the `public/` copy is the one Vite emits
// into `dist/`, which is what a deployment hands to a browser and what an image
// pushes to a registry. Two files, one grant, and until this cell a re-download
// could have updated one of them.
describe("the font licences", () => {
  it("ships a copy byte-identical to the one beside the font, for every family", () => {
    const families = readdirSync("src/fonts");
    // The premise: there really are families to compare, and each really has a
    // font beside its licence — so a renamed directory fails here rather than
    // passing as "no families disagreed".
    expect(families.length).toBeGreaterThan(1);
    for (const family of families) {
      const beside = readdirSync(`src/fonts/${family}`);
      expect(beside, family).toContain("OFL.txt");
      expect(beside.some((f) => f.endsWith(".woff2")), `${family} has a font`).toBe(true);
      expect(
        readFileSync(`public/fonts/${family}/OFL.txt`, "utf8"),
        `public/fonts/${family}/OFL.txt is not the copy beside the font`,
      ).toBe(readFileSync(`src/fonts/${family}/OFL.txt`, "utf8"));
    }
  });
});
