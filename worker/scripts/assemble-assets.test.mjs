import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { headerSet, headersJson } from "../../web/scripts/headers.mjs";
import { assemble, assembledHeaders, precacheEntriesFrom, verifyTree } from "./assemble-assets.mjs";

const sha256 = (text) => createHash("sha256").update(text).digest("hex");

/** The client build's `headers.json` (security S2a, Task 1): every synthetic
 *  `dist` an assembly is handed carries one, as every build of 0.6.2 does. */
const HEADERS_JSON = headersJson(headerSet({ siteUrl: null, releaseFeed: null, styleHashes: [] }));

/** `_headers`' rules as `path → { header name, lowercased → value }`: each
 *  unindented line is a path, each indented `Name: value` line a header of
 *  the path above it, and `#` lines and blank lines are skipped. */
function rules(text) {
  const out = new Map();
  let current = null;
  for (const line of text.split(/\r?\n/)) {
    if (line.trim() === "" || line.trimStart().startsWith("#")) continue;
    if (!/^\s/.test(line)) {
      current = {};
      out.set(line.trim(), current);
      continue;
    }
    const m = /^\s+([A-Za-z0-9-]+):\s*(.*?)\s*$/.exec(line);
    if (!m || current === null) throw new Error(`_headers: a line that is neither a path nor a header: ${line}`);
    current[m[1].toLowerCase()] = m[2];
  }
  return out;
}

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
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": HEADERS_JSON });
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
  // Cycle F (R-F5, R-FP3; spec §2.5): the 404 page and the build-values stamp
  // are in `web/dist` and in no precache list, and the assembler copies the
  // dist WHOLE — so they land in the tree the deploy uploads, where the asset
  // layer's `404-page` and the guard's checks 11 and 12 read them, with no
  // edit here. Read byte for byte, and reported among the files no list names.
  it("lands the 404 page and the build-values stamp, which no precache list names", () => {
    const page = "<!doctype html><title>Nothing here</title>\n";
    const stamp = '{\n  "turnstile_sitekey": false\n}\n';
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "404.html": page, "build-values.json": stamp, "headers.json": HEADERS_JSON });
    const out = join(tree({}), "assets");
    const report = assemble({ ...sources(dist), out });
    expect(readFileSync(join(out, "404.html"), "utf8")).toBe(page);
    expect(readFileSync(join(out, "build-values.json"), "utf8")).toBe(stamp);
    expect(report.unlisted).toEqual(["404.html", "_headers", "_redirects", "art/manifest.json", "build-values.json", "sw.js"]);
  });
  it("starts from an empty directory, so nothing from an earlier assembly survives", () => {
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": HEADERS_JSON });
    const out = join(tree({ "assets/stale.js": "old" }), "assets");
    assemble({ ...sources(dist), out });
    expect(existsSync(join(out, "stale.js"))).toBe(false);
  });
  it("refuses a file two sources both provide, naming it", () => {
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": HEADERS_JSON, _headers: "# dist's\n" });
    const out = join(tree({}), "assets");
    expect(() => assemble({ ...sources(dist), out })).toThrow(/_headers/);
  });
  // `npm run e2e` in `web/` leaves its DEBUG module in `web/dist`, with a
  // precache list that matches it — so the digests alone would pass it.
  it("refuses a browser module built for the debug walk, naming it", () => {
    const module = "\0asm...debug_trap...";
    const entries = [...ENTRIES, { path: "/assets/m.wasm", digest: sha256(module) }];
    const dist = tree({ "index.html": "<!doctype html>", "assets/m.wasm": module, "sw.js": swWith(entries), "headers.json": HEADERS_JSON });
    const out = join(tree({}), "assets");
    expect(() => assemble({ ...sources(dist), out })).toThrow(/^assets\/m\.wasm is a DEBUG browser module/);
  });
  it("refuses a tree the release does not match", () => {
    const dist = tree({ "index.html": "<!doctype HTML>", "sw.js": swWith(ENTRIES), "headers.json": HEADERS_JSON });
    const out = join(tree({}), "assets");
    expect(() => assemble({ ...sources(dist), out })).toThrow(/\/index\.html differs/);
  });

  // A refused assembly must leave NOTHING for the deploy guard to pass: a tree
  // left behind verifies against its own `sw.js`, so `npm run build` failing
  // here and `npm run deploy` after it would ship what was refused (task 7
  // review, Important 1). Each cell starts from an earlier assembly's tree, so
  // "absent" means removed rather than never made.
  const DEBUG_MODULE = "\0asm...debug_trap...";
  it.each([
    ["a dist with no sw.js", () => ({ "index.html": "<!doctype html>", "headers.json": HEADERS_JSON }), /holds no sw\.js/],
    [
      "a file two sources both provide",
      () => ({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": HEADERS_JSON, _headers: "# dist's\n" }),
      /_headers is provided by both/,
    ],
    [
      "a DEBUG browser module",
      () => ({
        "index.html": "<!doctype html>",
        "assets/m.wasm": DEBUG_MODULE,
        "sw.js": swWith([...ENTRIES, { path: "/assets/m.wasm", digest: sha256(DEBUG_MODULE) }]),
        "headers.json": HEADERS_JSON,
      }),
      /DEBUG browser module/,
    ],
    ["a worker with no entries", () => ({ "index.html": "<!doctype html>", "sw.js": swWith([]), "headers.json": HEADERS_JSON }), /no precache entries/],
    ["a file that differs", () => ({ "index.html": "<!doctype HTML>", "sw.js": swWith(ENTRIES), "headers.json": HEADERS_JSON }), /\/index\.html differs/],
  ])("refuses %s and leaves no tree behind", (_, distFiles, refusal) => {
    const out = join(tree({ "assets/stale.js": "old", "assets/sw.js": swWith(ENTRIES) }), "assets");
    expect(() => assemble({ ...sources(tree(distFiles())), out })).toThrow(refusal);
    expect(existsSync(out)).toBe(false);
  });

  it("appends the header set's block to _headers: the static text, then /* with the pairs of web/dist/headers.json — and ships the file itself to no one", () => {
    const out = join(tree({}), "assets");
    const set = headerSet({ siteUrl: "https://yokohama.test", releaseFeed: null, styleHashes: [] });
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": headersJson(set) });
    const statics = tree({ _headers: "/sw.js\n  Cache-Control: no-store\n", _redirects: "" });
    assemble({ ...sources(dist), statics, out });
    const text = readFileSync(join(out, "_headers"), "utf8");
    expect(text.startsWith("/sw.js\n  Cache-Control: no-store\n"), "the static text first, byte for byte").toBe(true);
    expect(text).toBe(assembledHeaders("/sw.js\n  Cache-Control: no-store\n", headersJson(set)));
    const block = rules(text).get("/*");
    expect(Object.keys(block)).toEqual(set.map(([name]) => name.toLowerCase()));
    expect(block["x-frame-options"]).toBe("DENY");
    expect(block["content-security-policy-report-only"]).toContain("wss://yokohama.test");
    // An INPUT of the assembly, never an asset: the zone's WAF edge rule is derived from the tree's top level
    // (`edge-rule.test.mjs`'s drift cell), and a browser gains nothing from the file — its pairs ride every answer.
    expect(existsSync(join(out, "headers.json")), "headers.json is not copied into the assets tree").toBe(false);
  });

  it("refuses a dist without headers.json, naming the file, and leaves no tree behind", () => {
    const out = join(tree({}), "assets");
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES) });
    expect(() => assemble({ ...sources(dist), out })).toThrow(/web\/dist\/headers\.json|headers\.json is absent/);
    expect(existsSync(out)).toBe(false);
  });

  it("reads headers.json's BYTES strictly: an invalid UTF-8 byte is refused, naming the file, where readFileSync(.., 'utf8') would carry U+FFFD into the block — and leaves no tree behind", () => {
    const out = join(tree({}), "assets");
    // `[["X-A","<0xff>"]]`: one byte no UTF-8 sequence admits, inside a value — what the NAS refuses at start, so
    // an assembly that passed it would ship a file the other host refuses (Task 2's review, m-3).
    const bad = Buffer.concat([Buffer.from('[["X-A","'), Buffer.from([0xff]), Buffer.from('"]]')]);
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": bad });
    expect(() => assemble({ ...sources(dist), out })).toThrow(/headers\.json is not valid UTF-8/);
    expect(existsSync(out)).toBe(false);
  });
});

// **The committed `worker/static/_headers`, read** (M6.5c2 cycle E, Task 1's
// review). It is this host's restatement of the axum host's cache policy
// (`assets.rs`'s `cache_control`), and nothing above reads what it SAYS: the
// cells use synthetic trees, and the deploy guard's check 9 compares two copies
// of this one file. The worker and the shell it installs must be `no-store`
// here as they are there (R-6.5c2.41): `.bin` is on Cloudflare's default
// cached-extension list, and a `/shell.bin` served bare would be the previous
// release's at the edge for two hours after a deploy, failing every install's
// digest in that window.
describe("the committed static headers", () => {
  const committed = (rel) => readFileSync(fileURLToPath(new URL(rel, import.meta.url)), "utf8");

  it("serves the worker and its shell no-store, as the axum host's cache_control does", () => {
    const headers = rules(committed("../static/_headers"));
    expect(headers.get("/sw.js")?.["cache-control"], "/sw.js").toBe("no-store");
    expect(headers.get("/shell.bin")?.["cache-control"], "/shell.bin").toBe("no-store");

    // **Mirrored from the Rust, arm by arm**, so a path the axum host serves
    // `no-store` that this file does not — or the reverse — fails here rather
    // than at a live `cache-control` read after a deploy.
    const rust = committed("../../crates/yokohama-web/src/assets.rs");
    const start = rust.indexOf("fn cache_control(");
    expect(start, "assets.rs no longer declares `fn cache_control`").toBeGreaterThan(0);
    const body = rust.slice(start, rust.indexOf("\n}\n", start));
    const rustNoStore = [...body.matchAll(/uri_path == "([^"]+)"/g)].map((m) => m[1]).sort();
    // The premise: the parse really found the two arms, so the equality below
    // is not two empty lists agreeing.
    expect(rustNoStore).toEqual(["/shell.bin", "/sw.js"]);
    const hereNoStore = [...headers]
      .filter(([, h]) => h["cache-control"] === "no-store")
      .map(([path]) => path)
      .sort();
    expect(hereNoStore).toEqual(rustNoStore);
    const immutable = /starts_with\("\/assets\/"\)\.then_some\("([^"]+)"\)/.exec(body)?.[1];
    expect(immutable, "assets.rs's immutable rule for /assets/").toBeDefined();
    expect(headers.get("/assets/*")?.["cache-control"]).toBe(immutable);

    // **The web presence's three files carry the art's hour on both hosts**
    // (cycle F, R-F12; spec §2.7). The paths are parsed out of the Rust's
    // `PRESENCE_FILES` and the value out of the arm that returns for them, so
    // the two hosts cannot drift in either; the premise pins that the parse
    // found the three, so the equality below is not two empty lists agreeing.
    const presence = /const PRESENCE_FILES: \[&str; \d+\] = \[([^\]]*)\];/.exec(rust)?.[1];
    expect(presence, "assets.rs no longer declares PRESENCE_FILES").toBeDefined();
    const rustPresence = [...presence.matchAll(/"([^"]+)"/g)].map((m) => m[1]).sort();
    expect(rustPresence).toEqual(["/llms.txt", "/robots.txt", "/sitemap.xml"]);
    const hour = /PRESENCE_FILES\.contains\(&uri_path\)\s*\{\s*return Some\("([^"]+)"\)/.exec(body)?.[1];
    expect(hour, "assets.rs's rule for the presence files").toBe("public, max-age=3600");
    for (const path of rustPresence) {
      expect(headers.get(path)?.["cache-control"], path).toBe(hour);
    }

    // And nothing else carries a rule, as the Rust cell's "nothing else"
    // clause says of the axum host — the art's hour is `art.rs`'s.
    expect([...headers.keys()].sort()).toEqual(
      ["/art/*", "/assets/*", "/llms.txt", "/robots.txt", "/shell.bin", "/sitemap.xml", "/sw.js"],
    );
    expect(headers.get("/art/*")?.["cache-control"]).toBe("public, max-age=3600");

    // **No rule here names a header of the generated `/*` block** (security
    // S2a, decision D-1): two rules naming one header send it twice (measured
    // in workerd), and one rule naming it twice joins the values with a comma.
    const generated = headerSet({ siteUrl: null, releaseFeed: null, styleHashes: [] }).map(([name]) => name.toLowerCase());
    for (const [path, rule] of rules(committed("../static/_headers"))) {
      for (const name of Object.keys(rule)) expect(generated, `${path} names ${name}, which the generated /* block carries`).not.toContain(name);
    }
  });
});
