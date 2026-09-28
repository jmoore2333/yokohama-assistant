// **The release, as a list of paths and the digest of each** (M6.5b2, spec
// §2.3; R-6.5b2.8).
//
// This is the PURE half of the precache generator: it takes an inventory and
// a way to read bytes, and returns the list the service worker installs plus
// the cache name that IS the release. `scripts/precache.mjs` is the shell that
// walks the real `dist`, reads the real art, calls this, and writes `dist/sw.js`.
//
// **The inventory is the finished tree and Vite's manifest is not consulted,
// and that is MEASURED rather than cautious** (spec §2.3). At this HEAD a build
// with `build.manifest: true` emits a manifest naming exactly
// `assets/index-<hash>.js` with the stylesheet in its `css` array; the Worker
// chunk and the WebAssembly module appear NOWHERE in it. A generator trusting
// it would report a complete release with no game engine in it, and a
// completeness test written against the same manifest would agree — a test that
// cannot fail. So this module takes `distFiles` as a list from a tree walk, and
// the refusals below turn "the module is missing" into a build that stops.
//
// **The cache name is the content, so nothing has to be bumped.** It is a
// digest over every (path, digest) pair, which also catches what a version
// number would miss: the art files have STABLE names rather than hashed ones
// (`art.rs:112` serves them `max-age=3600` for exactly that reason), so a
// re-cut card changes no filename and a scheme keyed on names alone would never
// reach a browser that had already cached the old picture.
//
// **This file imports nothing but its sibling `swRoute.ts`, on purpose.**
// `scripts/precache.mjs` bundles it with esbuild and imports the result, and it
// is also compiled into the app project by `tsconfig.json` and the
// service-worker project by `tsconfig.sw.json`. A `node:crypto` import here
// would be a node dependency in a file three programs compile, so the hash is
// INJECTED (`sha256Hex`) and the only thing this module knows about hashing is
// that the answer is hex. `swRoute.ts` is safe in all three for the same
// reason it is safe in the worker: it is `URL`, `Set` and string comparison.

import { isServerPath } from "./swRoute";

/** One entry the worker installs. */
export interface PrecacheEntry {
  /** The URL path a browser asks for, e.g. `/assets/index-abc.js`. */
  readonly path: string;
  /** SHA-256 of the bytes at build time, lowercase hex. The install digests the
   *  body it received and compares it against this. */
  readonly digest: string;
  /** The media type's ESSENCE the install expects — the part before any
   *  `;charset=`. Mirrors `assets::content_type`, which serves both `dist` and
   *  the art (`art.rs:124` calls it). */
  readonly type: string;
}

/** What a build produced. */
export interface Precache {
  /** The Cache Storage name for this release: [`CACHE_PREFIX`] plus a digest
   *  over the whole list. */
  readonly cacheName: string;
  /** Sorted by path, so two builds of the same bytes name the same release. */
  readonly entries: readonly PrecacheEntry[];
  /** Every entry's bytes, summed — what a cold install downloads. */
  readonly totalBytes: number;
  /** The WebAssembly module's bytes, which has a ceiling of its own because it
   *  is the one file a phone COMPILES on every cold start. */
  readonly moduleBytes: number;
  /** Its path, for the line the generator prints. */
  readonly modulePath: string;
  /** Bytes by class, for the same line. Keys are stable names, not extensions. */
  readonly byClass: Readonly<Record<string, number>>;
}

export interface PrecacheInputs {
  /** Every file in the finished `dist`, relative and POSIX-separated
   *  (`index.html`, `assets/index-abc.js`, `help/screen.webp`). */
  readonly distFiles: readonly string[];
  /** The art manifest, parsed. Every object anywhere inside it carrying a
   *  string `file` names one picture; see [`artFilesFrom`]. */
  readonly artManifest: unknown;
  /** Bytes for a URL PATH — `/art/...` is the server's art directory and
   *  everything else is `dist`. Keyed by the path rather than by a source
   *  location so this module needs to know about neither directory. */
  readonly readFile: (path: string) => Uint8Array | string;
  /** SHA-256 as lowercase hex. Injected; see the header. */
  readonly sha256Hex: (data: Uint8Array | string) => string;
  /** The module's ceiling in bytes. A PARAMETER because `npm run e2e` builds
   *  the DEBUG module, which is ~2.4× the release one and would otherwise fail
   *  every walk; [`ceilingsFor`] is what chooses. */
  readonly moduleCeiling: number;
  /** The whole release's ceiling in bytes. */
  readonly totalCeiling: number;
}

/** Every Cache Storage generation this client owns starts with this.
 *
 *  A prefix rather than a bare digest, so `activate`'s sweep can tell OUR
 *  caches from anything else the origin holds and delete only its own. */
export const CACHE_PREFIX = "yokohama-release-";

/** The reserved path inside a generation holding when it was created and,
 *  once a worker serving it has activated, when that was.
 *
 *  `activate` retains the PREVIOUS generation (spec §2.3) and deleting "older
 *  than that" needs an order; the caches themselves carry no timestamp, so each
 *  generation writes `createdAt` into itself at install, and its worker adds
 *  `activatedAt` at activation — the order the sweep actually keeps by, since
 *  "previous" means previously ACTIVE (PR #37, R-F5). It is under a path no
 *  server serves, so it can never shadow a real entry, and the fetch handler
 *  never serves it from a retained generation. */
export const META_PATH = "/__release__";

/** The string a DEBUG module carries and a release one does not.
 *
 *  `build:wasm:debug` compiles `-p yokohama-wasm --features debug`, which
 *  exports `debug_trap` for the e2e's recovery cell; the release module exports
 *  nothing of the sort. **Measured at this HEAD**: the release
 *  `yokohama_wasm_bg.wasm` contains it zero times, the debug one seven. It is
 *  the same marker `deploy/RUNBOOK.md` §1's preflight greps for — read off the
 *  MODULE rather than off the glue, because `worker.ts` names `debug_trap` in a
 *  `switch` arm either way and the bundled JS therefore always contains it. */
export const DEBUG_MODULE_MARKER = "debug_trap";

/** The release module's ceiling, in bytes.
 *
 *  `--profile release-wasm` plus `wasm-opt -Oz --strip-debug --strip-producers`
 *  is the module's size; the figure is the root `Cargo.toml`'s `release-wasm`
 *  comment, and every other document cites it. The Docker stage's module reads
 *  a few KB larger than the workstation's. The ceiling clears both with room,
 *  so ordinary growth does not trip it and a dependency that doubles the
 *  module does. */
export const RELEASE_MODULE_CEILING = 1_400_000;

/** The DEBUG module's ceiling, in bytes.
 *
 *  **Measured 2026-09-21**: `build:wasm:debug` emits ~2.84 MB (no `wasm-opt`
 *  pass, and the `debug` feature's own code). This arm exists so `npm run e2e`
 *  — which builds that module and then runs `build:client` — is not failed by a
 *  budget written for a release, while a release build is still held to the
 *  real number. */
export const DEBUG_MODULE_CEILING = 3_200_000;

/** The whole release's ceiling, in bytes.
 *
 *  **Re-measured 2026-09-22 at the fix-wave head**: 153 entries,
 *  **9,935,340 B** on this workstation — `dist` at 2,481,013 B (the module, whose
 *  figure is `Cargo.toml`'s to keep and which has since moved; the bundle and the Worker chunk at 410,991, the CSS, two fonts at
 *  98,728, five help pictures at 764,742, two OFL copies, the shell at 11,873,
 *  the web app manifest, two icon sources and four rasters at 19,456) plus the
 *  art at 7,454,327 B (131 pictures and their manifest). It was 9,903,378 B at
 *  Task 6, and the module is the whole of the difference. The ceiling is that
 *  figure with **20% of headroom**, per spec §2.7's "set with headroom, so
 *  ordinary growth does not trip them"; spec §2.7's own table predicted ~10 MB.
 *
 *  **The DEBUG arm buys back exactly the module's slack and nothing else**
 *  ([`ceilingsFor`]), so a help picture that doubled trips the budget during an
 *  `npm run e2e` as readily as during a release build. */
export const RELEASE_TOTAL_CEILING = 11_900_000;

/** Which pair of ceilings a build is judged by, and the label to print.
 *
 *  **The debug arm buys back EXACTLY the module's slack and nothing else**, so
 *  the rest of the release is held to the same number in both arms: a help
 *  picture that doubled would trip the budget during an `npm run e2e` as
 *  readily as during a release build. */
export function ceilingsFor(moduleBody: Uint8Array | string): {
  readonly label: "release" | "debug";
  readonly moduleCeiling: number;
  readonly totalCeiling: number;
} {
  if (!contains(moduleBody, DEBUG_MODULE_MARKER)) {
    return {
      label: "release",
      moduleCeiling: RELEASE_MODULE_CEILING,
      totalCeiling: RELEASE_TOTAL_CEILING,
    };
  }
  return {
    label: "debug",
    moduleCeiling: DEBUG_MODULE_CEILING,
    totalCeiling: RELEASE_TOTAL_CEILING + (DEBUG_MODULE_CEILING - RELEASE_MODULE_CEILING),
  };
}

/** Whether `data` contains `needle`, for bytes as well as text. */
function contains(data: Uint8Array | string, needle: string): boolean {
  if (typeof data === "string") return data.includes(needle);
  const n = needle.length;
  outer: for (let i = 0; i + n <= data.length; i++) {
    for (let j = 0; j < n; j++) if (data[i + j] !== needle.charCodeAt(j)) continue outer;
    return true;
  }
  return false;
}

/** The media type `assets::content_type` answers for this path, essence only.
 *
 *  **A mirror of the Rust table and pinned to it by a cell** that reads
 *  `crates/yokohama-web/src/assets.rs` and compares arm for arm — including the
 *  DEFAULT, because a path with no arm really does go out as
 *  `application/octet-stream` and an install expecting anything else would fail
 *  on a file the server served correctly. (`.txt` was that case until this
 *  cycle's fix wave, when the two OFL copies under `dist/fonts/` got an arm of
 *  their own and this table moved with it.) */
export function expectedType(path: string): string {
  const dot = path.lastIndexOf(".");
  const ext = dot < 0 ? "" : path.slice(dot + 1).toLowerCase();
  switch (ext) {
    case "html":
      return "text/html";
    case "js":
    case "mjs":
      return "text/javascript";
    case "css":
      return "text/css";
    case "json":
      return "application/json";
    case "svg":
      return "image/svg+xml";
    case "png":
      return "image/png";
    case "webp":
      return "image/webp";
    case "jpg":
    case "jpeg":
      return "image/jpeg";
    case "woff2":
      return "font/woff2";
    case "ico":
      return "image/x-icon";
    case "wasm":
      return "application/wasm";
    case "webmanifest":
      return "application/manifest+json";
    case "txt":
      return "text/plain";
    default:
      return "application/octet-stream";
  }
}

/** Every picture the art manifest names, as paths RELATIVE to the art root.
 *
 *  A walk rather than `Object.values(m).flatMap(Object.values)`, because the
 *  real manifest is not uniform: beside the ten groups of `{file, w, h}` it
 *  carries `provenance` (scalars) and `area_duplicates` (a list of objects with
 *  no `file` at all). A shape-shaped reader would either throw on those or, if
 *  it skipped them defensively, would skip a real group that grew a level. */
export function artFilesFrom(manifest: unknown): string[] {
  const out = new Set<string>();
  const walk = (v: unknown): void => {
    if (Array.isArray(v)) {
      for (const x of v) walk(x);
      return;
    }
    if (v === null || typeof v !== "object") return;
    const record = v as Record<string, unknown>;
    const file = record["file"];
    if (typeof file === "string" && file.length > 0) out.add(file);
    for (const x of Object.values(record)) walk(x);
  };
  walk(manifest);
  return [...out].sort();
}

/** Bytes in a body, whichever form the reader handed back. */
function byteLength(data: Uint8Array | string): number {
  return typeof data === "string" ? new TextEncoder().encode(data).length : data.length;
}

/** Which line of the printed composition a path belongs to. */
function classOf(path: string): string {
  if (path.startsWith("/art/")) return "art";
  if (path.startsWith("/help/")) return "help";
  if (path.startsWith("/icons/")) return "icons";
  if (path.endsWith(".wasm")) return "module";
  if (path.endsWith(".woff2")) return "fonts";
  if (path.startsWith("/assets/")) return "bundle";
  return "shell";
}

/**
 * The list, the digests and the name of one release.
 *
 * **Pure over its inputs**, so the cells below drive it with a fixture and the
 * shell drives it with a real tree, and both exercise the same arithmetic.
 *
 * It REFUSES rather than reports on six things, because each of them is a
 * release that would install cleanly and be wrong:
 *
 * - no `index.html` — a shell-less release answers every navigation with a miss;
 * - no `.wasm` — **this is the catcher for the spec's own named mutation**
 *   ("the precache list taken from Vite's manifest instead of the tree, so the
 *   module is silently absent"): that swap makes the BUILD stop rather than ship
 *   a release with no game engine in it;
 * - no script, no stylesheet, or no art — the same failure one step down;
 * - a path the worker would bypass, which is a build disagreeing with its own
 *   routing rule;
 * - a duplicate path, which would install one file twice and hide the other;
 * - either ceiling.
 */
export function buildPrecache(inputs: PrecacheInputs): Precache {
  const { distFiles, artManifest, readFile, sha256Hex, moduleCeiling, totalCeiling } = inputs;

  const artFiles = artFilesFrom(artManifest);
  const paths = [
    ...distFiles.map((f) => "/" + f.replace(/^\/+/, "")),
    "/art/manifest.json",
    ...artFiles.map((f) => "/art/" + f.replace(/^\/+/, "")),
  ].sort();

  const seen = new Set<string>();
  for (const path of paths) {
    if (seen.has(path)) throw new Error(`precache: ${path} is listed twice`);
    seen.add(path);
    if (isServerPath(path)) {
      throw new Error(
        `precache: ${path} is a path the service worker never intercepts, so a release must ` +
          `not name it`,
      );
    }
  }

  const need = (what: string, ok: (p: string) => boolean) => {
    if (!paths.some(ok)) {
      throw new Error(
        `precache: this release names no ${what}. The inventory is the finished dist TREE plus ` +
          `the art manifest (spec §2.3) — a list that lost one of these is not a release.`,
      );
    }
  };
  need("index.html", (p) => p === "/index.html");
  need("WebAssembly module", (p) => p.endsWith(".wasm"));
  need("script", (p) => p.endsWith(".js"));
  need("stylesheet", (p) => p.endsWith(".css"));
  need("art", (p) => p.startsWith("/art/") && p !== "/art/manifest.json");

  const entries: PrecacheEntry[] = [];
  const byClass: Record<string, number> = {};
  let totalBytes = 0;
  let moduleBytes = 0;
  let modulePath = "";
  for (const path of paths) {
    const body = readFile(path);
    const bytes = byteLength(body);
    totalBytes += bytes;
    const cls = classOf(path);
    byClass[cls] = (byClass[cls] ?? 0) + bytes;
    if (path.endsWith(".wasm") && bytes > moduleBytes) {
      moduleBytes = bytes;
      modulePath = path;
    }
    entries.push({ path, digest: sha256Hex(body), type: expectedType(path) });
  }

  if (moduleBytes > moduleCeiling) {
    throw new Error(
      `precache: the module is ${moduleBytes} B, over the ${moduleCeiling} B ceiling ` +
        `(${modulePath}). It is the one file a phone compiles on every cold start; see ` +
        `spec §2.7 and RELEASE_MODULE_CEILING's own note for what the measurement was.`,
    );
  }
  if (totalBytes > totalCeiling) {
    throw new Error(
      `precache: the release is ${totalBytes} B, over the ${totalCeiling} B ceiling. ` +
        `A cold install downloads all of it; see spec §2.7.`,
    );
  }

  // The NAME is a digest over the list, so it moves when any file's bytes move
  // and stays put when nothing does. Sixteen hex characters of SHA-256 — a
  // cache name is a local identifier, not a security claim, and the whole digest
  // would only make the sweep's log harder to read.
  const cacheName =
    CACHE_PREFIX + sha256Hex(entries.map((e) => `${e.path} ${e.digest}`).join("\n")).slice(0, 16);

  return { cacheName, entries, totalBytes, moduleBytes, modulePath, byClass };
}

/**
 * Whether the bundled worker really carries this release, or the sentence
 * saying what is missing.
 *
 * **The first version of this check was half-vacuous and that is the point of
 * the shape here** (review, Important 2). It looked for the cache name and for
 * the literal `"/index.html"` — but `sw.ts` contains `"/index.html"` in its own
 * source, at `fromCache("/index.html", request)`, so a RENAMED `__PRECACHE__`
 * passed it and shipped a worker whose top-level `ENTRIES = __PRECACHE__`
 * throws `ReferenceError` before a single handler is registered.
 *
 * So the check asks for something only the SUBSTITUTED list can contain: one
 * `digest: "…"` per entry, and the first entry's digest by value. Neither
 * appears anywhere in `sw.ts`'s source, and a define that replaced nothing
 * produces zero of them.
 *
 * And both placeholders must be GONE. esbuild writes a `// <define:NAME>`
 * banner above each substitution, which is a mention and not a reference, so
 * those lines are dropped before the search — a banner is exactly the evidence
 * that the define fired, and leaving it in would make the absence check
 * impossible to state.
 */
export function checkSubstitution(built: string, release: Precache): string | null {
  const body = built.replace(/^\s*\/\/ <define:[^>]*>\s*$/gm, "");
  for (const placeholder of ["__PRECACHE__", "__CACHE_NAME__"]) {
    if (body.includes(placeholder)) {
      return (
        `${placeholder} is still in the built worker, so esbuild's \`define\` replaced nothing ` +
        `and the worker reads a global that does not exist`
      );
    }
  }
  if (!body.includes(JSON.stringify(release.cacheName))) {
    return `the built worker does not name the release ${release.cacheName}`;
  }
  const digests = body.match(/digest: "/g)?.length ?? 0;
  if (digests !== release.entries.length) {
    return `the built worker carries ${digests} digests, not this release's ${release.entries.length}`;
  }
  const first = release.entries[0];
  if (first && !body.includes(JSON.stringify(first.digest))) {
    return `the built worker does not carry ${first.path}'s digest`;
  }
  return null;
}
