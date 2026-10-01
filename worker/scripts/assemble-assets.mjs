// **The static assets** (spec §2.3, R-6.5c.10): `npm run build:assets` builds
// `worker/assets/` — the directory `wrangler.jsonc`'s `assets` block uploads —
// and VERIFIES it against the release's own precache list.
//
// 1. `worker/assets/` is removed FIRST, so nothing an earlier assembly left
//    survives — and removed again when any step below refuses, so a refused
//    assembly leaves no tree for the deploy guard to pass: a tree left behind
//    would verify against its own `sw.js` (task 7 review, Important 1).
// 2. `web/dist/**` is copied in; then `crates/yokohama-web/assets/art/**` under
//    `art/`, because the art is not in `web/dist` (`web/vite.config.ts` says
//    why: it is the server's asset, and a copy in the client would drift);
//    then `worker/static/_headers` and
//    `_redirects`. A file two sources both provide is refused, naming it,
//    rather than one silently replacing the other.
// 3. A browser module built for the DEBUG walk is refused (`debugModuleIn`):
//    `npm run e2e` in `web/` leaves one in `web/dist`, carrying `debug_trap`,
//    with a precache list that matches it. The marker is read off the module's
//    own bytes, as `web/src/precache.ts` reads it.
// 4. The built service worker's precache list is read out of `sw.js`, and every
//    entry's SHA-256 is checked against the assembled file — so the tree IS the
//    release the service worker installs, not merely a tree that has a `sw.js`.
//
// `precacheEntriesFrom`, `verifyTree` and `debugModuleIn` are also the deploy
// guard's (`scripts/deploy-guard.mjs`), which re-checks the tree before a
// deploy however the tree was made.

import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

/** One entry as esbuild prints the substituted `__PRECACHE__` into `sw.js`
 *  (`web/scripts/precache.mjs`): unquoted keys, a quoted path with no escape
 *  in it, and a SHA-256 in lowercase hex. */
const PAIR = /path: "([^"\\]*)", digest: "([0-9a-f]{64})"/g;

/**
 * The release's precache list, read out of the built service worker's text.
 * Refuses a worker with no entry, and one carrying a `digest: "` this reader
 * could not pair with a path — the count is its own anti-vacuity: a list whose
 * printed shape drifted would otherwise read as a shorter list.
 */
export function precacheEntriesFrom(swText) {
  const entries = [...swText.matchAll(PAIR)].map(([, path, digest]) => ({ path, digest }));
  if (entries.length === 0) {
    throw new Error("the built sw.js carries no precache entries — is it web/scripts/precache.mjs's output?");
  }
  const digests = swText.split('digest: "').length - 1;
  if (digests !== entries.length) {
    throw new Error(
      `the built sw.js carries ${digests} digests but paired ${entries.length} of them with a path — ` +
        "the list's printed shape is not the one this reader knows",
    );
  }
  return entries;
}

/** Throws, naming every entry of `entries` whose file under `root` is absent
 *  or whose SHA-256 is not the one listed (the first ten, and a count). */
export function verifyTree(root, entries) {
  const wrong = [];
  for (const { path, digest } of entries) {
    const file = join(root, ...path.split("/").filter((s) => s !== ""));
    if (!path.startsWith("/") || !statSync(file, { throwIfNoEntry: false })?.isFile()) {
      wrong.push(`${path} is absent from ${root}`);
      continue;
    }
    const actual = createHash("sha256").update(readFileSync(file)).digest("hex");
    if (actual !== digest) wrong.push(`${path} differs from the release (sha256 ${actual}, listed ${digest})`);
  }
  if (wrong.length > 0) {
    const more = wrong.length > 10 ? `\n…and ${wrong.length - 10} more` : "";
    throw new Error(`the tree is not the release its sw.js lists:\n${wrong.slice(0, 10).join("\n")}${more}`);
  }
}

/** Every file under `dir`, relative, POSIX-separated and sorted. */
function walk(dir) {
  const out = [];
  const visit = (rel) => {
    for (const name of readdirSync(join(dir, rel)).sort()) {
      const child = rel === "" ? name : `${rel}/${name}`;
      if (statSync(join(dir, child)).isDirectory()) visit(child);
      else out.push(child);
    }
  };
  visit("");
  return out;
}

/** The first browser module under `root` built for the DEBUG walk — a `.wasm`
 *  whose bytes contain `debug_trap` — or `undefined`. */
export function debugModuleIn(root) {
  return walk(root).find((f) => f.endsWith(".wasm") && readFileSync(join(root, f)).includes("debug_trap"));
}

/**
 * Builds `out` from the three sources and verifies it (the steps above).
 * Answers what it assembled: the precache entries it verified, every file and
 * its bytes, and the files the precache list does not name. Whatever it
 * refuses, it leaves no `out` behind.
 */
export function assemble({ dist, art, statics, out }) {
  rmSync(out, { recursive: true, force: true });
  try {
    return assembleInto({ dist, art, statics, out });
  } catch (e) {
    rmSync(out, { recursive: true, force: true });
    throw e;
  }
}

function assembleInto({ dist, art, statics, out }) {
  if (!existsSync(join(dist, "sw.js"))) {
    throw new Error(`${dist} holds no sw.js — build the release first: cd web && npm run build`);
  }
  mkdirSync(out, { recursive: true });

  const from = new Map();
  for (const [source, dir, under] of [
    ["web/dist", dist, ""],
    ["the art", art, "art/"],
    ["worker/static", statics, ""],
  ]) {
    for (const file of walk(dir)) {
      const target = under + file;
      if (from.has(target)) throw new Error(`${target} is provided by both ${from.get(target)} and ${source}`);
      from.set(target, source);
      mkdirSync(dirname(join(out, target)), { recursive: true });
      copyFileSync(join(dir, file), join(out, target));
    }
  }

  const files = walk(out);
  const debug = debugModuleIn(out);
  if (debug) {
    throw new Error(
      `${debug} is a DEBUG browser module (it exports debug_trap) — npm run e2e leaves one in web/dist; ` +
        "build the release: cd web && npm run build",
    );
  }

  const entries = precacheEntriesFrom(readFileSync(join(out, "sw.js"), "utf8"));
  verifyTree(out, entries);
  const listed = new Set(entries.map((e) => e.path.slice(1)));
  return {
    entries: entries.length,
    files: files.length,
    bytes: files.reduce((sum, f) => sum + statSync(join(out, f)).size, 0),
    unlisted: files.filter((f) => !listed.has(f)).sort(),
  };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const worker = fileURLToPath(new URL("..", import.meta.url));
  const repo = join(worker, "..");
  try {
    const r = assemble({
      dist: join(repo, "web", "dist"),
      art: join(repo, "crates", "yokohama-web", "assets", "art"),
      statics: join(worker, "static"),
      out: join(worker, "assets"),
    });
    console.log(
      "assemble-assets: worker/assets from web/dist, crates/yokohama-web/assets/art (under art/) and worker/static\n" +
        `assemble-assets: ${r.entries} precache entries verified — every file present, every SHA-256 the release's\n` +
        `assemble-assets: ${r.files} files, ${r.bytes.toLocaleString("en-US")} B; ` +
        `not in the precache list: ${r.unlisted.join(", ")}`,
    );
  } catch (e) {
    console.error(`assemble-assets: ${e instanceof Error ? e.message : String(e)}`);
    process.exit(1);
  }
}
