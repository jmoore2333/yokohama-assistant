// **The precache generator: the release, digested, and the worker that installs
// it** (M6.5b2, spec §2.3; R-6.5b2.8).
//
// Runs after `vite build` as the third stage of `npm run build:client`. It is
// the THIN half — the arithmetic, the budget and every refusal live in
// `src/precache.ts`, which is pure and has cells; this walks the real tree,
// reads the real bytes, and writes `dist/sw.js` — and `dist/shell.bin`, the
// byte copy of `dist/index.html` the worker installs the shell from.
//
// **The inventory is the finished `dist` tree plus the art, and Vite's manifest
// is not consulted at all** (spec §2.3, measured): a build with
// `build.manifest: true` names `assets/index-<hash>.js` and its stylesheet, and
// the Worker chunk and the WebAssembly module appear NOWHERE in it. A generator
// trusting that would report a complete release with no game engine in it — and
// a completeness test written against the same manifest would agree with it.
// `buildPrecache` refuses a list with no `.wasm` in it, so that swap is a build
// that stops rather than a release that ships.
//
// **The worker is BUILT, not templated.** `src/sw.ts` imports `swRoute.ts` and
// `swInstall.ts`, which the component suite tests without a browser; esbuild
// bundles that file and writes the release's list and name in through `define`.
// A template with the routing rule copied into it would be a tested model
// beside an untested copy, and the tested one would not be the one that runs.
//
// The art root is the SERVER's (`crates/yokohama-web/assets/art`), never a copy
// under `web/public/`: `vite.config.ts` says why there is one copy of the card
// art and not two. `deploy/Dockerfile`'s client stage places it at
// `/crates/yokohama-web/assets/art`, which is what `../crates/...` resolves to
// from its `/app` working directory.

import { build } from "esbuild";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const web = fileURLToPath(new URL("..", import.meta.url));
const dist = process.env.YOKOHAMA_WEB_DIST_DIR ?? join(web, "dist");
const artDir = process.env.YOKOHAMA_ART_DIR ?? join(web, "..", "crates", "yokohama-web", "assets", "art");

/** Every file under `dir`, relative and POSIX-separated. */
function walk(dir) {
  /** @type {string[]} */
  const out = [];
  const visit = (at) => {
    for (const name of readdirSync(at).sort()) {
      const full = join(at, name);
      if (statSync(full).isDirectory()) visit(full);
      else out.push(relative(dir, full).split(sep).join("/"));
    }
  };
  visit(dir);
  return out;
}

/** `src/precache.ts`, compiled and imported.
 *
 *  The pure module is TypeScript because it is compiled into the app project
 *  (its cells) and the service-worker project (`sw.ts` imports it) as well as
 *  into this script. esbuild bundles it to a string and node imports that
 *  string — no temporary file, and no second copy of the arithmetic in JS. */
async function loadPure() {
  const built = await build({
    entryPoints: [join(web, "src", "precache.ts")],
    bundle: true,
    write: false,
    format: "esm",
    platform: "node",
    target: "es2022",
  });
  const code = built.outputFiles[0].text;
  return import("data:text/javascript;base64," + Buffer.from(code).toString("base64"));
}

/** `n` with thousands separators, because these are numbers a person reads. */
const n = (x) => x.toLocaleString("en-US");

const {
  buildPrecache,
  artFilesFrom,
  ceilingsFor,
  checkSubstitution,
  RELEASE_MODULE_CEILING,
  DEBUG_MODULE_CEILING,
  SHELL_PATH,
  SHELL_SOURCE,
} = await loadPure();

// **The shell, as a file the edge does not rewrite** (R-6.5c2.41; spec
// §10.11). The zone's JavaScript Detections inject a per-request script into
// every HTML answer, so the worker's install never fetches `index.html`: it
// fetches this byte copy, which `buildPrecache` lists at `SHELL_PATH` with
// `index.html`'s digest. Written BEFORE the walk and the digests, and only
// where there is an `index.html` to copy — without one, `buildPrecache`
// refuses the release by name.
const shellFile = SHELL_PATH.slice(1);
if (existsSync(join(dist, SHELL_SOURCE))) copyFileSync(join(dist, SHELL_SOURCE), join(dist, shellFile));

// **`sw.js` is never precached by itself.** `vite.config.ts` sets
// `emptyOutDir: true`, so a previous build's copy is already gone; this is the
// belt to that brace, and it is the one entry whose presence in the list would
// be a release that could never update — the worker would serve its own stale
// bytes to the update check.
//
// **Nor is the shell's copy walked**: `index.html` is listed AS `SHELL_PATH`,
// so a walk that kept `shell.bin` would name it twice — and `offline.spec.ts`
// re-runs this script over its own output, where the copy is already there.
const distFiles = walk(dist).filter((f) => f !== "sw.js" && f !== shellFile);
const artManifest = JSON.parse(readFileSync(join(artDir, "manifest.json"), "utf8"));

/** Bytes for a URL path: `/art/...` is the server's art directory, everything
 *  else is `dist`. */
const readFile = (path) =>
  path.startsWith("/art/")
    ? readFileSync(join(artDir, path.slice("/art/".length)))
    : readFileSync(join(dist, path.slice(1)));

// **Which ceilings this build is judged by** (orchestrator ruling; spec §2.7).
// `npm run e2e` builds the DEBUG module — 2.4× the release one, carrying the
// `debug_trap` export its recovery cell raises — and running the walk must not
// be failed by a budget written for a release. The marker is read off the
// MODULE's bytes and never off the bundled JS, where `worker.ts` names
// `debug_trap` in a `switch` arm whichever module is present.
const modulePath = distFiles.find((f) => f.endsWith(".wasm"));
const ceilings = ceilingsFor(modulePath ? readFileSync(join(dist, modulePath)) : "");

const release = buildPrecache({
  distFiles,
  artManifest,
  readFile,
  sha256Hex: (data) => createHash("sha256").update(data).digest("hex"),
  moduleCeiling: ceilings.moduleCeiling,
  totalCeiling: ceilings.totalCeiling,
});

// **The worker, bundled with the release written into it.** A `define` that
// found no identifier to replace would leave the worker reading a global that
// does not exist — a `ReferenceError` before the first handler is registered —
// so the output is checked for values only the SUBSTITUTED release can carry
// (`checkSubstitution`, which has its own cells), and not merely for a build
// that succeeded.
const worker = await build({
  entryPoints: [join(web, "src", "sw.ts")],
  bundle: true,
  write: false,
  format: "iife",
  platform: "browser",
  target: "es2022",
  define: {
    __CACHE_NAME__: JSON.stringify(release.cacheName),
    __PRECACHE__: JSON.stringify(release.entries),
  },
  banner: {
    js:
      `// GENERATED by web/scripts/precache.mjs — do not edit, and do not commit.\n` +
      `// The source is web/src/sw.ts; the release is ${release.cacheName}.\n`,
  },
});
const text = worker.outputFiles[0].text;
const wrong = checkSubstitution(text, release);
if (wrong) {
  throw new Error(
    `precache: ${wrong}. The generator writes the release in through esbuild's \`define\` on ` +
      `\`__CACHE_NAME__\` and \`__PRECACHE__\`; renaming either in src/sw.ts leaves the worker ` +
      `reading a global that does not exist.`,
  );
}
writeFileSync(join(dist, "sw.js"), text);

const order = ["module", "bundle", "fonts", "shell", "icons", "help", "art"];
const composition = order
  .filter((k) => release.byClass[k] !== undefined)
  .map((k) => `${k} ${n(release.byClass[k])}`)
  .concat(
    Object.keys(release.byClass)
      .filter((k) => !order.includes(k))
      .sort()
      .map((k) => `${k} ${n(release.byClass[k])}`),
  )
  .join(" · ");

console.log(
  `precache: ${release.entries.length} entries, ${n(release.totalBytes)} B ` +
    `(${ceilings.label} total ceiling ${n(ceilings.totalCeiling)} B)\n` +
    `precache: module ${n(release.moduleBytes)} B at ${release.modulePath} ` +
    `(${ceilings.label} module ceiling ${n(ceilings.moduleCeiling)} B; release ` +
    `${n(RELEASE_MODULE_CEILING)}, debug ${n(DEBUG_MODULE_CEILING)})\n` +
    `precache: ${composition}\n` +
    `precache: ${release.cacheName} -> dist/sw.js (${n(Buffer.byteLength(text))} B), ` +
    `${artFilesFrom(artManifest).length} art files`,
);
