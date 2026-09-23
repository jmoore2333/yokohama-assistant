import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { assemble, precacheEntriesFrom, verifyTree } from "./assemble-assets.mjs";

const sha256 = (text) => createHash("sha256").update(text).digest("hex");

/** A built service worker's text, as esbuild prints the substituted
 *  `__PRECACHE__`: unquoted keys, and a `type` after each digest
 *  (`web/scripts/precache.mjs`, `web/src/precache.ts`). */
const swWith = (entries) =>
  "var define_PRECACHE_default = [" +
  entries
    .map((e) => `{ path: ${JSON.stringify(e.path)}, digest: ${JSON.stringify(e.digest)}, type: "text/plain" }`)
    .join(", ") +
  "];\n";

const dirs = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A temporary directory holding `files` (path → text). */
function tree(files) {
  const root = mkdtempSync(join(tmpdir(), "assemble-"));
  dirs.push(root);
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
}

const FILES = { "index.html": "<!doctype html>", "art/orders/1.webp": "AAA" };
const ENTRIES = [
  { path: "/index.html", digest: sha256("<!doctype html>") },
  { path: "/art/orders/1.webp", digest: sha256("AAA") },
];

describe("the release's own precache list (spec §2.3)", () => {
  it("reads the path and digest pairs out of the built worker, and two that verify pass", () => {
    const entries = precacheEntriesFrom(swWith(ENTRIES));
    expect(entries).toEqual(ENTRIES);
    expect(() => verifyTree(tree(FILES), entries)).not.toThrow();
  });
  it("refuses an entry whose file is missing, naming it", () => {
    const root = tree(FILES);
    rmSync(join(root, "art/orders/1.webp"));
    expect(() => verifyTree(root, ENTRIES)).toThrow(/\/art\/orders\/1\.webp is absent/);
  });
  it("refuses an entry whose file differs by one byte, naming it", () => {
    const root = tree({ ...FILES, "art/orders/1.webp": "AAB" });
    expect(() => verifyTree(root, ENTRIES)).toThrow(/\/art\/orders\/1\.webp differs/);
  });
  it("refuses a worker with no entries", () => {
    expect(() => precacheEntriesFrom(swWith([]))).toThrow(/no precache entries/);
  });
  it("refuses a digest it cannot pair with a path", () => {
    const text = swWith(ENTRIES) + 'var stray = { digest: "not-a-sha256" };\n';
    expect(() => precacheEntriesFrom(text)).toThrow(/3 digests but paired 2/);
  });
});

describe("the assembly (spec §2.3)", () => {
  const sources = (dist) => ({
    dist,
    art: tree({ "manifest.json": '{"orders":{}}', "orders/1.webp": "AAA" }),
    statics: tree({ _headers: "# headers\n", _redirects: "/ /index.html 200\n" }),
  });

  it("lays out dist, the art under art/ and the two static files, then verifies them", () => {
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES) });
    const out = join(tree({}), "assets");
    const report = assemble({ ...sources(dist), out });
    for (const f of ["index.html", "sw.js", "art/manifest.json", "art/orders/1.webp", "_headers", "_redirects"]) {
      expect(existsSync(join(out, f)), f).toBe(true);
    }
    expect(report.entries).toBe(2);
    expect(report.files).toBe(6);
    expect(report.bytes).toBe(
      ["index.html", "sw.js", "art/manifest.json", "art/orders/1.webp", "_headers", "_redirects"]
        .map((f) => readFileSync(join(out, f)).length)
        .reduce((a, b) => a + b, 0),
    );
    expect(report.unlisted).toEqual(["_headers", "_redirects", "art/manifest.json", "sw.js"]);
  });
  it("starts from an empty directory, so nothing from an earlier assembly survives", () => {
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES) });
    const out = join(tree({ "assets/stale.js": "old" }), "assets");
    assemble({ ...sources(dist), out });
    expect(existsSync(join(out, "stale.js"))).toBe(false);
  });
  it("refuses a file two sources both provide, naming it", () => {
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), _headers: "# dist's\n" });
    const out = join(tree({}), "assets");
    expect(() => assemble({ ...sources(dist), out })).toThrow(/_headers/);
  });
  // `npm run e2e` in `web/` leaves its DEBUG module in `web/dist`, with a
  // precache list that matches it — so the digests alone would pass it.
  it("refuses a browser module built for the debug walk, naming it", () => {
    const module = "\0asm...debug_trap...";
    const entries = [...ENTRIES, { path: "/assets/m.wasm", digest: sha256(module) }];
    const dist = tree({ "index.html": "<!doctype html>", "assets/m.wasm": module, "sw.js": swWith(entries) });
    const out = join(tree({}), "assets");
    expect(() => assemble({ ...sources(dist), out })).toThrow(/^assets\/m\.wasm is a DEBUG browser module/);
  });
  it("refuses a tree the release does not match", () => {
    const dist = tree({ "index.html": "<!doctype HTML>", "sw.js": swWith(ENTRIES) });
    const out = join(tree({}), "assets");
    expect(() => assemble({ ...sources(dist), out })).toThrow(/\/index\.html differs/);
  });
});
