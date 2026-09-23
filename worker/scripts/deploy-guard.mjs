// **The deploy guard.** `npm run deploy` is `node scripts/deploy-guard.mjs &&
// wrangler deploy`, so nothing ships unless all five hold. Every check that
// fails is named, not only the first.
//
// 1. `gen/factory.js` exists and does not contain `debug_trap(` — the RELEASE
//    module. M6.5b's `grep -c debug_trap` rule, for this module: the release
//    factory still NAMES `debug_trap` in the object it returns (`typeof
//    debug_trap === "function" ? …`), and only the debug glue defines and calls
//    it. And no `gen/.unoptimized` sits beside it: the build leaves that marker
//    when its optimizer step changed nothing (`scripts/build-module.mjs`).
// 2. The assets directory `wrangler.jsonc` names holds a `sw.js`, `verifyTree`
//    passes over it, and no browser module in it is a DEBUG build
//    (`debugModuleIn`) — so the tree wrangler uploads IS the release the
//    service worker installs, however the tree was made. A debug `web/dist`
//    carries a precache list that matches it, so the digests alone would pass
//    one (task 7 review, Important 1).
// 3. `wrangler.jsonc` sets `"workers_dev": false` and `"preview_urls": false`.
//    Absent is not false: with no route, an absent `workers_dev` is a public
//    `workers.dev` URL, and an absent `preview_urls` leaves the Worker's
//    Version URLs as they were — public, if they were ever on.
// 4. `YOKOHAMA_WEB_TEST_CLOCK` appears nowhere in `wrangler.jsonc` — read as
//    text, so an environment's `vars` or a comment cannot carry it past.
// 5. The `assets` block is EXACTLY spec §2.3's, in `wrangler.jsonc` AND in
//    `wrangler.test.jsonc`: `html_handling` and `not_found_handling` `none`,
//    `run_worker_first` `["/api/*", "/enter"]`, the binding `ASSETS`, and no key
//    beyond those and `directory`. Battery rows 13 and 14 (plan review M5): an
//    asset-layer fallback answers a missing file WITHOUT invoking the Worker,
//    and the pool reads only `wrangler.test.jsonc`, so no pool cell can see
//    production's block drift. Checks 3–5 need no build, and
//    `deploy-guard.test.mjs` runs them over the committed files — the
//    per-commit catcher; the live reading at each deploy is the second.
//
// Both files are read with `jsonc-parser`, which knows a string from a
// comment: stripping `//` before `JSON.parse` would corrupt every string that
// holds one (every URL), and would choke on a trailing comma.

import { existsSync, readFileSync } from "node:fs";
import { join, posix } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parse, printParseErrorCode } from "jsonc-parser";
import { debugModuleIn, precacheEntriesFrom, verifyTree } from "./assemble-assets.mjs";

/** Spec §2.3's handling, and the binding `/api/art` reads the manifest
 *  through. */
const ASSETS = {
  html_handling: "none",
  not_found_handling: "none",
  run_worker_first: ["/api/*", "/enter"],
  binding: "ASSETS",
};
const ASSET_KEYS = [...Object.keys(ASSETS), "directory"].sort();

/** 1-based line and column of `offset` in `text`. */
function position(text, offset) {
  const before = text.slice(0, offset).split("\n");
  return `line ${before.length}, column ${before.at(-1).length + 1}`;
}

/** The configuration named `name` under `root`, parsed — or `undefined`, with
 *  the reason in `failures`. */
function readConfig(root, name, failures) {
  const file = join(root, name);
  if (!existsSync(file)) {
    failures.push(`${name} is absent`);
    return undefined;
  }
  const text = readFileSync(file, "utf8");
  const errors = [];
  const value = parse(text, errors, { allowTrailingComma: true, disallowComments: false });
  if (errors.length > 0) {
    const each = errors.map((e) => `${printParseErrorCode(e.error)} at ${position(text, e.offset)}`);
    failures.push(`${name} does not parse: ${each.join("; ")}`);
    return undefined;
  }
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    failures.push(`${name} is not an object`);
    return undefined;
  }
  return { text, value };
}

/** Check 5, over one configuration. */
function assetsBlock(name, value, failures) {
  const block = value.assets;
  if (block === null || typeof block !== "object" || Array.isArray(block)) {
    failures.push(`${name} has no assets block — spec §2.3's is required`);
    return;
  }
  const keys = Object.keys(block).sort();
  if (JSON.stringify(keys) !== JSON.stringify(ASSET_KEYS)) {
    failures.push(`${name}'s assets block has the keys ${keys.join(", ")} — spec §2.3 names ${ASSET_KEYS.join(", ")}`);
  }
  for (const [key, want] of Object.entries(ASSETS)) {
    if (JSON.stringify(block[key]) !== JSON.stringify(want)) {
      failures.push(`${name}'s assets.${key} must be ${JSON.stringify(want)} — got ${JSON.stringify(block[key]) ?? "nothing"}`);
    }
  }
  if (typeof block.directory !== "string" || block.directory === "") {
    failures.push(`${name}'s assets.directory must name the directory to upload`);
  }
}

/** Why an absent key is not `false` (check 3). */
const ABSENT = {
  workers_dev: "with no route, wrangler serves an absent one as a public workers.dev URL",
  preview_urls: "wrangler leaves an absent one as the Worker already has it — public, if it was ever on",
};

/** Checks 3, 4 and 5: the ones that need no build. */
export function configFailures(root) {
  const failures = [];
  const production = readConfig(root, "wrangler.jsonc", failures);
  if (production) {
    for (const [key, why] of Object.entries(ABSENT)) {
      if (production.value[key] !== false) {
        failures.push(
          `wrangler.jsonc must set "${key}": false — got ${JSON.stringify(production.value[key]) ?? "nothing"}; ${why}`,
        );
      }
    }
    if (production.text.includes("YOKOHAMA_WEB_TEST_CLOCK")) {
      failures.push("wrangler.jsonc names YOKOHAMA_WEB_TEST_CLOCK — the test clock is set in wrangler.test.jsonc and nowhere else");
    }
    assetsBlock("wrangler.jsonc", production.value, failures);
  }
  const test = readConfig(root, "wrangler.test.jsonc", failures);
  if (test) assetsBlock("wrangler.test.jsonc", test.value, failures);
  return failures;
}

/** All five checks over the worker package at `root`: `[]` when every one
 *  holds. */
export function deployGuard(root) {
  const failures = [];

  const factory = join(root, "gen", "factory.js");
  if (!existsSync(factory)) {
    failures.push("gen/factory.js is absent — build the release module: npm run build");
  } else if (readFileSync(factory, "utf8").includes("debug_trap(")) {
    failures.push("gen/factory.js is the DEBUG module (it defines debug_trap) — build the release: npm run build");
  }
  if (existsSync(join(root, "gen", ".unoptimized"))) {
    failures.push(
      "gen/.unoptimized marks an UNOPTIMIZED module — the release build's optimizer step changed nothing " +
        "(web/scripts/wasm-opt.mjs skips when binaryen is absent and YOKOHAMA_WASM_OPT_OPTIONAL=1); " +
        "install binaryen and build the release: npm run build",
    );
  }

  // Checks 3–5 report a configuration that does not parse; check 2 only needs
  // to know where the tree is, and says nothing more when it cannot.
  const directory = readConfig(root, "wrangler.jsonc", [])?.value.assets?.directory;
  if (typeof directory === "string" && directory !== "") {
    const shown = posix.normalize(directory);
    const tree = join(root, directory);
    const sw = join(tree, "sw.js");
    if (!existsSync(sw)) {
      failures.push(`${shown}/sw.js is absent — assemble the assets: npm run build`);
    } else {
      try {
        verifyTree(tree, precacheEntriesFrom(readFileSync(sw, "utf8")));
      } catch (e) {
        failures.push(`${shown}: ${e instanceof Error ? e.message : String(e)}`);
      }
    }
    const debug = existsSync(tree) ? debugModuleIn(tree) : undefined;
    if (debug) {
      failures.push(
        `${shown}/${debug} is a DEBUG browser module (it exports debug_trap) — ` +
          "build the release (cd web && npm run build), then assemble the assets again: npm run build:assets",
      );
    }
  }

  failures.push(...configFailures(root));
  return failures;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const failures = deployGuard(fileURLToPath(new URL("..", import.meta.url)));
  if (failures.length > 0) {
    console.error(`deploy-guard: refused —\n${failures.map((f) => `  ${f}`).join("\n")}`);
    process.exit(1);
  }
  console.log(
    "deploy-guard: the optimized release module; the assets tree is the release, with no debug browser module; " +
      "no workers.dev or preview URL; no test clock; spec §2.3's assets block in both configurations",
  );
}
