import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parse } from "jsonc-parser";
import { afterEach, describe, expect, it } from "vitest";
import { ACTION, allowList, DESCRIPTION, expression, HOST, ruleJson, verify } from "./edge-rule.mjs";

// **The no-Access door's edge rule** (cycle F, R-F6, R-FP7; spec §2.5, §2.6):
// ONE WAF Managed Challenge rule on the Worker's hostname, for every request
// whose path is not one of the release's own static names. Its expression is
// DERIVED from the assets tree's top level, so these cells read trees: a
// fixture tree for the derivation, and the assembled `worker/assets/` for the
// drift cell, which pins the names by hand.

const worker = fileURLToPath(new URL("..", import.meta.url));

const dirs = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A scratch directory holding `files` (path → text). Its REAL path: macOS's
 *  temporary directory sits behind a symlink, and the CLI guard compares the
 *  module's resolved URL with the path it was started by. */
function tree(files) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "edge-rule-")));
  dirs.push(root);
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
}

/** The production build's top level (the window's build, spec §2.6: the
 *  site URL set, so the sitemap is written), one file in each directory. */
const PRODUCTION_TREE = {
  "404.html": "<!doctype html>",
  "build-values.json": "{}\n",
  "index.html": "<!doctype html>",
  "llms.txt": "# Yokohama\n",
  "manifest.webmanifest": "{}",
  "robots.txt": "User-agent: *\n",
  "shell.bin": "<!doctype html>",
  "sitemap.xml": "<urlset/>",
  "sw.js": "// the worker\n",
  _headers: "/*\n",
  _redirects: "/ /index.html 200\n",
  "art/manifest.json": "{}",
  "assets/index.js": "",
  "fonts/a.woff2": "",
  "help/a.webp": "",
  "icons/a.png": "",
};

/** The expression the brief gives (the plan's names table, row 7) for that
 *  tree, VERBATIM — in the spec's order, which a derivation does not keep. */
const BRIEF =
  '(http.host eq "yokohama.moore.nyc" and not (http.request.uri.path in {"/" "/index.html" "/sw.js" "/shell.bin" "/manifest.webmanifest" "/404.html" "/robots.txt" "/llms.txt" "/sitemap.xml" "/build-values.json"} or starts_with(http.request.uri.path, "/assets/") or starts_with(http.request.uri.path, "/art/") or starts_with(http.request.uri.path, "/icons/") or starts_with(http.request.uri.path, "/fonts/") or starts_with(http.request.uri.path, "/help/") or starts_with(http.request.uri.path, "/cdn-cgi/")))';

/** The same names in the derivation's order: the files sorted (code units,
 *  so `/` first), then the directories sorted, then `/cdn-cgi/` LAST. */
const DERIVED =
  '(http.host eq "yokohama.moore.nyc" and not (http.request.uri.path in {"/" "/404.html" "/build-values.json" "/index.html" "/llms.txt" "/manifest.webmanifest" "/robots.txt" "/shell.bin" "/sitemap.xml" "/sw.js"} or starts_with(http.request.uri.path, "/art/") or starts_with(http.request.uri.path, "/assets/") or starts_with(http.request.uri.path, "/fonts/") or starts_with(http.request.uri.path, "/help/") or starts_with(http.request.uri.path, "/icons/") or starts_with(http.request.uri.path, "/cdn-cgi/")))';

/** Every quoted name in an expression, sorted: what it allows, order aside. */
const members = (text) => [...text.matchAll(/"(\/[^"]*)"/g)].map((m) => m[1]).sort();

describe("the allow-list, read off the tree's top level", () => {
  // Mutants: `_headers`/`_redirects` listed; `/` not added for `index.html`;
  // a directory read as a file; `/cdn-cgi/` not added.
  it("lists every top-level file and directory, `/` for index.html, never _headers or _redirects, and /cdn-cgi/ last", () => {
    expect(allowList(tree(PRODUCTION_TREE))).toEqual({
      files: [
        "/",
        "/404.html",
        "/build-values.json",
        "/index.html",
        "/llms.txt",
        "/manifest.webmanifest",
        "/robots.txt",
        "/shell.bin",
        "/sitemap.xml",
        "/sw.js",
      ],
      prefixes: ["/art/", "/assets/", "/fonts/", "/help/", "/icons/", "/cdn-cgi/"],
    });
  });
  it("adds /cdn-cgi/ to a tree with no directory, and `/` only beside index.html", () => {
    expect(allowList(tree({ "a.txt": "a" }))).toEqual({ files: ["/a.txt"], prefixes: ["/cdn-cgi/"] });
  });
  // Cloudflare's own prefix is listed once, last, whatever the tree holds.
  it("lists /cdn-cgi/ once, last, even beside a directory of that name", () => {
    expect(allowList(tree({ "index.html": "", "cdn-cgi/x": "", "zeta/y": "" })).prefixes).toEqual(["/zeta/", "/cdn-cgi/"]);
  });
  // The expression is the zone's: a name the rules language would need to
  // escape is refused rather than written into a string literal.
  it("refuses a name it cannot write into the rule, naming it", () => {
    expect(() => allowList(tree({ 'a"b.txt': "" }))).toThrow('edge-rule: the tree\'s top-level name "a\\"b.txt" is not one the rule can carry');
    expect(() => allowList(tree({ "a b.txt": "" }))).toThrow(/top-level name "a b\.txt" is not one the rule can carry/);
  });
  it("refuses a tree that is not there, naming it", () => {
    const gone = join(tree({}), "assets");
    expect(() => allowList(gone)).toThrow(`edge-rule: ${gone} is absent — assemble the assets: npm run build:assets`);
  });
});

describe("the expression, the rule and the comparison", () => {
  // **Battery row 30** (`/cdn-cgi/` dropped from the allow-list): this cell.
  // Byte-exact, since `--verify` compares the zone's copy as a string.
  // Mutants: `/cdn-cgi/` dropped; the order unsorted; a separator changed.
  it("is byte-exact for the production tree, the brief's names in the derivation's order", () => {
    expect(expression(allowList(tree(PRODUCTION_TREE)), HOST)).toBe(DERIVED);
    expect(members(DERIVED)).toEqual(members(BRIEF));
    expect(DERIVED.length).toBe(BRIEF.length);
  });
  it("writes no set for a tree with no file, and the host as given", () => {
    expect(expression({ files: [], prefixes: ["/cdn-cgi/"] }, "example.test")).toBe(
      '(http.host eq "example.test" and not (starts_with(http.request.uri.path, "/cdn-cgi/")))',
    );
  });
  it("refuses a host that is not a plain hostname", () => {
    expect(() => expression({ files: ["/"], prefixes: ["/cdn-cgi/"] }, 'x" or true or "')).toThrow(/is not a hostname/);
  });
  // The body the zone's `http_request_firewall_custom` ruleset takes for a
  // rule; `enabled` is the operator's to flip by the rule's id.
  it("builds the rule's JSON: the expression, the managed challenge, the description, enabled", () => {
    const allow = allowList(tree(PRODUCTION_TREE));
    expect(ruleJson(allow, HOST)).toEqual({
      expression: DERIVED,
      action: "managed_challenge",
      description: "yokohama — the no-Access door's edge check (F)",
      enabled: true,
    });
    expect(Object.keys(ruleJson(allow, HOST))).toEqual(["expression", "action", "description", "enabled"]);
    expect([ACTION, DESCRIPTION]).toEqual(["managed_challenge", "yokohama — the no-Access door's edge check (F)"]);
  });
  // `--verify`: the zone adds its own keys (`id`, `version`, `last_updated`,
  // `ref`), and `enabled` is flipped by id, so three are compared. Mutants: a
  // key not compared; `enabled` compared; an extra key reported.
  it("reports a differing expression, action or description, and nothing for an equal rule", () => {
    const allow = allowList(tree(PRODUCTION_TREE));
    const zone = { id: "abc", version: "3", ref: "abc", last_updated: "2026-10-01T00:00:00Z", ...ruleJson(allow, HOST), enabled: false };
    expect(verify(zone, allow, HOST)).toEqual([]);
    const shorter = DERIVED.replace(' or starts_with(http.request.uri.path, "/cdn-cgi/")', "");
    expect(verify({ ...zone, expression: shorter }, allow, HOST)).toEqual([
      `expression: the zone's ${JSON.stringify(shorter)} — the tree's ${JSON.stringify(DERIVED)}`,
    ]);
    expect(verify({ ...zone, action: "block", description: "other" }, allow, HOST)).toEqual([
      `action: the zone's "block" — the tree's "managed_challenge"`,
      `description: the zone's "other" — the tree's ${JSON.stringify(DESCRIPTION)}`,
    ]);
    expect(verify({ id: "abc" }, allow, HOST)).toEqual([
      `expression: the zone's nothing — the tree's ${JSON.stringify(DERIVED)}`,
      `action: the zone's nothing — the tree's "managed_challenge"`,
      `description: the zone's nothing — the tree's ${JSON.stringify(DESCRIPTION)}`,
    ]);
    expect(verify([], allow, HOST)).toEqual(["the zone's rule is not a JSON object — pipe in the one rule, not a list or the API's envelope"]);
  });
});

/** The CLI, run as the window runs it — over a COPY of the script beside a
 *  fixture tree, since it reads `../assets/` from where it sits. */
function cli(files, args = [], input = undefined) {
  const root = tree(Object.fromEntries(Object.entries(files).map(([p, t]) => [`assets/${p}`, t])));
  mkdirSync(join(root, "scripts"));
  copyFileSync(join(worker, "scripts", "edge-rule.mjs"), join(root, "scripts", "edge-rule.mjs"));
  return spawnSync(process.execPath, [join(root, "scripts", "edge-rule.mjs"), ...args], { input, encoding: "utf8" });
}

describe("the command line (Task 11.G.4's two commands)", () => {
  it("prints the rule's JSON for the tree beside it and exits 0", () => {
    const run = cli(PRODUCTION_TREE);
    expect(run.stderr).toBe("");
    expect(run.status).toBe(0);
    expect(run.stdout).toBe(`${JSON.stringify({ expression: DERIVED, action: ACTION, description: DESCRIPTION, enabled: true }, null, 2)}\n`);
  });
  it("--verify exits 0 on an equal rule and 1 on a difference, naming it", () => {
    const zone = { id: "abc", ...JSON.parse(cli(PRODUCTION_TREE).stdout), enabled: false };
    const same = cli(PRODUCTION_TREE, ["--verify"], JSON.stringify(zone));
    expect([same.status, same.stdout, same.stderr]).toEqual([
      0,
      "edge-rule: the zone's rule equals the tree's — expression, action and description\n",
      "",
    ]);
    const differs = cli(PRODUCTION_TREE, ["--verify"], JSON.stringify({ ...zone, action: "block" }));
    expect([differs.status, differs.stdout]).toEqual([1, ""]);
    expect(differs.stderr).toBe(`edge-rule: the zone's rule differs from the tree's —\n  action: the zone's "block" — the tree's "managed_challenge"\n`);
    // A new top-level name in the tree is a difference against the zone's copy.
    const grown = cli({ ...PRODUCTION_TREE, "new/x": "" }, ["--verify"], JSON.stringify(zone));
    expect(grown.status).toBe(1);
    expect(grown.stderr).toMatch(/^edge-rule: the zone's rule differs from the tree's —\n {2}expression: /);
  });
  // A run that cannot judge is not a difference: 2, naming why.
  it("refuses, with 2, input that is not JSON, any other argument, and a missing tree", () => {
    const garbled = cli(PRODUCTION_TREE, ["--verify"], "not json");
    expect([garbled.status, garbled.stdout]).toEqual([2, ""]);
    expect(garbled.stderr).toMatch(/^edge-rule: --verify reads the zone's rule as JSON on standard input — /);
    const other = cli(PRODUCTION_TREE, ["--host", "x"]);
    expect([other.status, other.stderr]).toEqual([2, 'edge-rule: expected no arguments or --verify — got ["--host","x"]\n']);
    const bare = spawnSync(process.execPath, [join(worker, "scripts", "edge-rule.mjs"), "--verify", "extra"], { encoding: "utf8" });
    expect(bare.status).toBe(2);
    const root = tree({});
    mkdirSync(join(root, "scripts"));
    copyFileSync(join(worker, "scripts", "edge-rule.mjs"), join(root, "scripts", "edge-rule.mjs"));
    const none = spawnSync(process.execPath, [join(root, "scripts", "edge-rule.mjs")], { encoding: "utf8" });
    expect([none.status, none.stdout]).toEqual([2, ""]);
    expect(none.stderr).toMatch(/assets is absent — assemble the assets: npm run build:assets\n$/);
  });
});

// **The drift cell — battery row 31** (a top-level name added to the tree).
// The tree's top-level names, written out BY HAND: a release that adds one
// fails here until this list and the zone's rule move together (regenerate
// the JSON, update the rule by its id, `--verify` it). The two halves:
// - the ASSEMBLED tree (`worker/assets/`, gitignored, built by `npm run
//   build:assets`), read through `allowList` as the CLI reads it. A checkout
//   that has built nothing has no tree — CI's `worker` job among them, and
//   `test:scripts` runs on such a checkout by design
//   (`vitest.scripts.config.ts`) — so that half is SKIPPED there, out loud in
//   its name; the deploy path reads the tree through `--verify` regardless.
// - the SOURCES that put a name at the tree's top level by copying —
//   `web/public/` (vite copies it to `dist/` whole) and `worker/static/` —
//   which every checkout has, so CI is not vacuous here.
// `sitemap.xml` is written only when the build's `YOKOHAMA_SITE_URL` is set
// (web/scripts/presence.mjs), so the assembled half reads it present exactly
// when the tree's own stamp says `site_url: true`.
const BY_HAND = {
  files: ["404.html", "build-values.json", "index.html", "llms.txt", "manifest.webmanifest", "robots.txt", "shell.bin", "sitemap.xml", "sw.js"],
  directories: ["art", "assets", "fonts", "help", "icons"],
  neverListed: ["_headers", "_redirects"],
};

const assembled = join(worker, "assets");
const isAssembled = existsSync(join(assembled, "sw.js"));

describe("the drift cell: the tree's top-level names, by hand", () => {
  it.skipIf(!isAssembled)(
    `the assembled worker/assets/ allows exactly the names written here${isAssembled ? "" : " (SKIPPED: worker/assets/ is not assembled — npm run build:assets)"}`,
    () => {
      const stamp = JSON.parse(readFileSync(join(assembled, "build-values.json"), "utf8"));
      const files = BY_HAND.files.filter((f) => f !== "sitemap.xml" || stamp.site_url === true);
      const want = {
        files: ["/", ...files.map((f) => `/${f}`)].sort(),
        prefixes: [...BY_HAND.directories.map((d) => `/${d}/`).sort(), "/cdn-cgi/"],
      };
      expect(
        allowList(assembled),
        "worker/assets/'s top level moved: if the tree is stale, build the client and assemble it again " +
          "(cd web && npm run build:client; cd ../worker && npm run build:assets); if a name is new, list it here " +
          "and regenerate the zone's rule (node scripts/edge-rule.mjs; --verify)",
      ).toEqual(want);
      const top = readdirSync(assembled).sort();
      expect(top, "and nothing else is at the top level but the two files the asset layer reads").toEqual(
        [...files, ...BY_HAND.directories, ...BY_HAND.neverListed].sort(),
      );
    },
  );
  it("every name web/public/ and worker/static/ put at the tree's top level is written here", () => {
    const repo = join(worker, "..");
    for (const name of readdirSync(join(repo, "web", "public"))) {
      const isDir = statSync(join(repo, "web", "public", name)).isDirectory();
      expect(isDir ? BY_HAND.directories : BY_HAND.files, `web/public/${name}`).toContain(name);
    }
    expect(readdirSync(join(worker, "static")).sort()).toEqual(BY_HAND.neverListed);
  });
  // The rule names the Worker's hostname, which the open door's configuration
  // routes (and production's, which it equals there). Mutant: HOST edited alone.
  it("names the hostname the open door's configuration routes", () => {
    for (const name of ["wrangler.open.jsonc", "wrangler.jsonc"]) {
      const value = parse(readFileSync(join(worker, name), "utf8"), [], { allowTrailingComma: true, disallowComments: false });
      expect(value.routes, name).toEqual([{ pattern: HOST, custom_domain: true }]);
    }
  });
});
