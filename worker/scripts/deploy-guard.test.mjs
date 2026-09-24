import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { configFailures, deployGuard } from "./deploy-guard.mjs";

const sha256 = (text) => createHash("sha256").update(text).digest("hex");

const INDEX = "<!doctype html>";
const SW = `var define_PRECACHE_default = [{ path: "/index.html", digest: "${sha256(INDEX)}", type: "text/html" }];\n`;

/** The release factory's return, as `factory-glue.mjs` writes it: the name
 *  `debug_trap` appears, the call `debug_trap(` does not. */
const RELEASE_FACTORY =
  "export function createModule(compiled) {\n  initSync({ module: compiled });\n" +
  '  return { init, debug_trap: typeof debug_trap === "function" ? debug_trap : undefined };\n}\n';

const ASSETS_BLOCK = (directory) => `"assets": {
    "directory": "${directory}",
    "binding": "ASSETS",
    "html_handling": "none",
    "not_found_handling": "none",
    "run_worker_first": ["/api/*", "/enter"]
  }`;

/** The three object classes, bound and migrated as spec §3.1 has them. */
const OBJECTS = `"durable_objects": {
    "bindings": [
      { "name": "TABLE", "class_name": "Table" },
      { "name": "HOUSE", "class_name": "House" },
      { "name": "ARCHIVER", "class_name": "Archive" }
    ]
  },
  "migrations": [{ "tag": "v1", "new_sqlite_classes": ["Table", "House", "Archive"] }]`;

/** The archive's bucket, bound as spec §3.1 has it. */
const BUCKET = `"r2_buckets": [{ "binding": "ARCHIVE", "bucket_name": "yokohama-archive" }]`;

const PRODUCTION = `// production
{
  "name": "yokohama-web",
  "main": "src/index.ts",
  "workers_dev": false,
  "preview_urls": false,
  ${ASSETS_BLOCK("./assets")},
  ${OBJECTS},
  ${BUCKET},
}
`;

const TEST_CONFIG = `{
  "name": "yokohama-web-test",
  ${ASSETS_BLOCK("./.test-assets")},
  ${OBJECTS},
  ${BUCKET},
  "vars": { "YOKOHAMA_WEB_TEST_CLOCK": "frozen:1789000000" }
}
`;

const dirs = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A worker package that passes every check, with `changes` (path → text, or
 *  `null` to leave the file out) laid over it. */
function worker(changes = {}) {
  const root = mkdtempSync(join(tmpdir(), "deploy-guard-"));
  dirs.push(root);
  const files = {
    "gen/factory.js": RELEASE_FACTORY,
    "assets/index.html": INDEX,
    "assets/sw.js": SW,
    "wrangler.jsonc": PRODUCTION,
    "wrangler.test.jsonc": TEST_CONFIG,
    ...changes,
  };
  for (const [path, text] of Object.entries(files)) {
    if (text === null) continue;
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
}

describe("the deploy guard", () => {
  it("passes a release module, a verified tree and a production configuration", () => {
    expect(deployGuard(worker())).toEqual([]);
  });

  it("refuses when no module has been built", () => {
    expect(deployGuard(worker({ "gen/factory.js": null }))).toEqual([
      expect.stringMatching(/^gen\/factory\.js is absent/),
    ]);
  });
  it("refuses the DEBUG module", () => {
    const debug = RELEASE_FACTORY.replace("  initSync", "  function debug_trap() { wasm.debug_trap(); }\n  initSync");
    expect(deployGuard(worker({ "gen/factory.js": debug }))).toEqual([
      expect.stringMatching(/^gen\/factory\.js is the DEBUG module/),
    ]);
  });

  // `YOKOHAMA_WASM_OPT_OPTIONAL=1` lets a release build finish without the
  // optimizer, exiting 0 with "do not deploy it"; the build leaves this marker
  // beside the module (task 7 review, Important 2).
  it("refuses an UNOPTIMIZED module, naming its marker", () => {
    expect(deployGuard(worker({ "gen/.unoptimized": "the wasm-opt step changed nothing\n" }))).toEqual([
      expect.stringMatching(/^gen\/\.unoptimized marks an UNOPTIMIZED module/),
    ]);
  });

  it("refuses when the assets have not been assembled", () => {
    expect(deployGuard(worker({ "assets/sw.js": null }))).toEqual([
      expect.stringMatching(/^assets\/sw\.js is absent/),
    ]);
  });
  it("refuses a tree that is not the release its sw.js lists", () => {
    expect(deployGuard(worker({ "assets/index.html": "<!doctype HTML>" }))).toEqual([
      expect.stringMatching(/\/index\.html differs/),
    ]);
  });
  // However the tree was made: its own `sw.js` lists the debug module, so the
  // digests alone pass it (task 7 review, Important 1).
  it("refuses a DEBUG browser module in the assets tree", () => {
    const module = "\0asm...debug_trap...";
    const sw =
      `var define_PRECACHE_default = [{ path: "/index.html", digest: "${sha256(INDEX)}", type: "text/html" }, ` +
      `{ path: "/assets/m.wasm", digest: "${sha256(module)}", type: "application/wasm" }];\n`;
    expect(deployGuard(worker({ "assets/sw.js": sw, "assets/assets/m.wasm": module }))).toEqual([
      expect.stringMatching(/^assets\/assets\/m\.wasm is a DEBUG browser module/),
    ]);
  });
  // The tree checked is the one wrangler uploads: the directory production's
  // `assets` block names.
  it("verifies the directory wrangler.jsonc names, not a fixed one", () => {
    const moved = PRODUCTION.replace('"./assets"', '"./elsewhere"');
    expect(deployGuard(worker({ "wrangler.jsonc": moved }))).toEqual([
      expect.stringMatching(/^elsewhere\/sw\.js is absent/),
    ]);
  });

  it("refuses a workers.dev URL", () => {
    const open = PRODUCTION.replace('"workers_dev": false', '"workers_dev": true');
    expect(deployGuard(worker({ "wrangler.jsonc": open }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc must set "workers_dev": false/),
    ]);
  });
  // Absent is not false: wrangler leaves an existing Worker's Version URLs as
  // they were.
  it("refuses preview URLs left at their default", () => {
    const open = PRODUCTION.replace('"preview_urls": false,', "");
    expect(deployGuard(worker({ "wrangler.jsonc": open }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc must set "preview_urls": false/),
    ]);
  });

  it("refuses the test clock anywhere in the production configuration", () => {
    const clocked = PRODUCTION.replace('"main"', '"vars": { "YOKOHAMA_WEB_TEST_CLOCK": "tick:1" },\n  "main"');
    expect(deployGuard(worker({ "wrangler.jsonc": clocked }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc names YOKOHAMA_WEB_TEST_CLOCK/),
    ]);
  });

  // Battery rows 13 and 14 (plan review M5): the pool reads only
  // wrangler.test.jsonc, and an asset-layer fallback answers without invoking
  // the Worker, so no pool cell can see production's block change; this can.
  it("refuses production's assets block when it is not spec §2.3's", () => {
    const spa = PRODUCTION.replace('"not_found_handling": "none"', '"not_found_handling": "single-page-application"');
    expect(deployGuard(worker({ "wrangler.jsonc": spa }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc's assets\.not_found_handling must be "none"/),
    ]);
  });
  it("refuses the pool's assets block when it drifts from production's", () => {
    const drift = TEST_CONFIG.replace('["/api/*", "/enter"]', '["/api/*"]');
    expect(deployGuard(worker({ "wrangler.test.jsonc": drift }))).toEqual([
      expect.stringMatching(/^wrangler\.test\.jsonc's assets\.run_worker_first must be \["\/api\/\*","\/enter"\]/),
    ]);
  });
  // `/api/art` reads the manifest through `env.ASSETS` (spec §2.3).
  it("refuses an assets binding under another name", () => {
    const renamed = PRODUCTION.replace('"binding": "ASSETS"', '"binding": "STATIC"');
    expect(deployGuard(worker({ "wrangler.jsonc": renamed }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc's assets\.binding must be "ASSETS"/),
    ]);
  });
  it("refuses an assets key spec §2.3 does not name", () => {
    const extra = PRODUCTION.replace('"binding": "ASSETS",', '"binding": "ASSETS",\n    "experimental_serve_directly": true,');
    expect(deployGuard(worker({ "wrangler.jsonc": extra }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc's assets block has the keys .*experimental_serve_directly/),
    ]);
  });

  // Check 6 (the orchestrator's ruling, task 9's review): the pool reads only
  // wrangler.test.jsonc, so a class or a binding production lacks — or a
  // migration production does not ship — would be tested green and never
  // deployed.
  it("refuses a pool whose object bindings differ from production's", () => {
    const drift = TEST_CONFIG.replace(',\n      { "name": "ARCHIVER", "class_name": "Archive" }', "");
    expect(deployGuard(worker({ "wrangler.test.jsonc": drift }))).toEqual([
      expect.stringMatching(/^wrangler\.test\.jsonc's durable_objects differs from wrangler\.jsonc's/),
    ]);
  });
  it("refuses a migration the pool runs and production does not ship", () => {
    const drift = PRODUCTION.replace('["Table", "House", "Archive"]', '["Table", "House"]');
    expect(deployGuard(worker({ "wrangler.jsonc": drift }))).toEqual([
      expect.stringMatching(/^wrangler\.test\.jsonc's migrations differs from wrangler\.jsonc's/),
    ]);
  });
  it("reads the two blocks by value, whatever their keys' order or spacing", () => {
    const reordered = TEST_CONFIG.replace(
      '"migrations": [{ "tag": "v1", "new_sqlite_classes": ["Table", "House", "Archive"] }]',
      '"migrations": [{"new_sqlite_classes": ["Table", "House", "Archive"], "tag": "v1"}]',
    );
    expect(deployGuard(worker({ "wrangler.test.jsonc": reordered }))).toEqual([]);
  });
  // The same rule for the bucket (Task 13): a pool whose `ARCHIVE` is bound
  // where production's is not would test an archive production cannot reach
  // — and production's `YOKOHAMA_WEB_ARCHIVE=r2` with no binding is a 500 on
  // every `/api/*`.
  it("refuses a pool whose bucket binding differs from production's", () => {
    const drift = TEST_CONFIG.replace('"bucket_name": "yokohama-archive"', '"bucket_name": "yokohama-archive-test"');
    expect(deployGuard(worker({ "wrangler.test.jsonc": drift }))).toEqual([
      expect.stringMatching(/^wrangler\.test\.jsonc's r2_buckets differs from wrangler\.jsonc's/),
    ]);
  });
  it("refuses a production configuration with no bucket binding", () => {
    const bare = PRODUCTION.replace(`${BUCKET},\n`, "");
    expect(deployGuard(worker({ "wrangler.jsonc": bare }))).toEqual([
      "wrangler.jsonc has no r2_buckets block — spec §3.1 binds the bucket yokohama-archive as ARCHIVE",
    ]);
  });
  it("refuses a configuration with no object bindings or no migrations", () => {
    const bare = PRODUCTION.replace(`${OBJECTS},\n`, "");
    expect(deployGuard(worker({ "wrangler.jsonc": bare }))).toEqual([
      "wrangler.jsonc has no durable_objects block — spec §3.1 binds the three object classes",
      "wrangler.jsonc has no migrations block — spec §3.1 migrates the three object classes",
    ]);
  });

  it("reports a configuration that does not parse, with its position", () => {
    const broken = PRODUCTION.replace('"main": "src/index.ts",', '"main": "src/index.ts"');
    expect(deployGuard(worker({ "wrangler.jsonc": broken }))).toEqual([
      expect.stringMatching(/^wrangler\.jsonc does not parse: CommaExpected at line 5, column 3/),
    ]);
  });
  // The reason the reader is `jsonc-parser` and not "strip // then JSON.parse":
  // a `//` INSIDE a string is data, and a trailing comma is legal JSONC.
  it("reads a // inside a string as data", () => {
    const routed = PRODUCTION.replace(
      '"main"',
      '"routes": [{ "pattern": "https://yokohama.moore.nyc/*", "custom_domain": true },],\n  "main"',
    );
    expect(deployGuard(worker({ "wrangler.jsonc": routed }))).toEqual([]);
  });

  it("names every check that fails, not only the first", () => {
    const failures = deployGuard(worker({ "gen/factory.js": null, "assets/sw.js": null }));
    expect(failures).toHaveLength(2);
  });
});

describe("the committed configurations", () => {
  // The per-commit catcher for battery rows 13 and 14: the checks that need no
  // build, over the two files this commit ships.
  it("pass every check that needs no build", () => {
    expect(configFailures(fileURLToPath(new URL("..", import.meta.url)))).toEqual([]);
  });
});
