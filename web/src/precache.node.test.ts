// **The cell this whole task exists for: a REAL build's output, inventoried.**
// (M6.5b2, spec §2.3 and §6.2.)
//
// `precache.test.ts` drives `buildPrecache` with a synthetic fixture, and a
// manifest-shaped fixture agrees with a manifest-shaped bug — which is exactly
// the finding spec §2.3 is built on: Vite's own manifest names
// `assets/index-<hash>.js` and its stylesheet, and the Worker chunk and the
// WebAssembly module appear NOWHERE in it, so a generator and a test that both
// read it would agree that a release with no game engine in it was complete.
//
// So this one runs `vite build` into a temporary directory and asks what came
// out. It is in the NODE project rather than beside its sibling because it
// needs the generated wasm glue under `src/local/wasm/`, which CI's
// toolchain-free `web-client` job does not build; `npm run test:wasm` runs it
// where the module exists, which is locally and in CI's `web-server` job.
//
// It writes into a temporary directory and removes it, so a run leaves
// `web/dist` and the working tree exactly as it found them. The GENERATOR runs
// there too (`scripts/precache.mjs` under `YOKOHAMA_WEB_DIST_DIR`), so the
// list read back is the one a release really prints into `sw.js` — read by the
// ONE reader the Worker's assembly and deploy readings use — and not only the
// pure half's answer.
//
// **Cycle F adds what a build emits and the release does NOT list** (R-F5,
// R-FP3, R-F12; spec §2.5, §2.7): the 404 page and the build-values stamp are
// in the tree, and the generator leaves them — and the web presence's three
// files — out of the list, so the offline count is what it was. And the stamp
// is read for what it says, through `vite.config.ts`'s own derivation, over
// three builds with the values set in the environment.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, sep } from "node:path";
import { afterAll, describe, expect, it } from "vitest";
import { readPrecacheList } from "../../worker/scripts/precache-list.mjs";
import { parseHeadersJson } from "../scripts/headers.mjs";
import { buildPrecache } from "./precache";

const web = process.cwd();
const glue = join(web, "src", "local", "wasm", "yokohama_wasm.js");
const optional = process.env.YOKOHAMA_WASM_OPTIONAL === "1";

if (!existsSync(glue) && !optional) {
  throw new Error(
    "the real-build precache cell cannot run — src/local/wasm/yokohama_wasm.js is absent.\n" +
      "  Run `npm run build:wasm` (it needs the Rust toolchain, the wasm32-unknown-unknown " +
      "target and wasm-bindgen 0.2.128).\n" +
      "  Set YOKOHAMA_WASM_OPTIONAL=1 to skip it and check nothing.",
  );
}

let built: string | null = null;
const stamped: string[] = [];
afterAll(() => {
  if (built) rmSync(built, { recursive: true, force: true });
  for (const dir of stamped.splice(0)) rmSync(dir, { recursive: true, force: true });
});

/** The `dist` files the release never lists (cycle F; spec §2.5, §2.7; and
 *  security S2a's header set) — the generator's exclusion, restated here rather
 *  than imported, for the reason `walk` is: a shared constant would let both
 *  agree wrongly. */
const NOT_PRECACHED = ["404.html", "robots.txt", "llms.txt", "sitemap.xml", "build-values.json", "headers.json"];

/** The stamp's six keys, in the order the build writes them (R-FP3). */
const STAMP_KEYS = ["project_url", "blurb", "notice", "release_feed", "site_url", "turnstile_sitekey"];

/** Every build value the stamp reports on, so a cell can clear the
 *  developer's own environment before it sets the ones it means. */
const BUILD_VALUE_VARS = [
  "YOKOHAMA_PROJECT_URL",
  "YOKOHAMA_SITE_BLURB",
  "YOKOHAMA_SITE_NOTICE",
  "YOKOHAMA_RELEASE_FEED",
  "YOKOHAMA_TURNSTILE_SITEKEY",
  "YOKOHAMA_SITE_URL",
];

/** Every file under `dir`, relative and POSIX-separated — the generator's own
 *  walk, restated here rather than imported, because the SHELL is the thing
 *  this cell is checking and a shared helper would let both agree wrongly. */
function walk(dir: string): string[] {
  const out: string[] = [];
  const visit = (at: string) => {
    for (const name of readdirSync(at).sort()) {
      const full = join(at, name);
      if (statSync(full).isDirectory()) visit(full);
      else out.push(relative(dir, full).split(sep).join("/"));
    }
  };
  visit(dir);
  return out;
}

if (!existsSync(glue)) {
  // **A skip that SAYS it checked nothing**, the house shape
  // (`YOKOHAMA_WEB_E2E_OPTIONAL`, `scripts/typecheck-worker.mjs`, the BGA
  // capture corpus). The throw above is the default; this line is only reached
  // under the opt-out, and without it a green run and a run that inventoried no
  // build would look the same.
  // `process.stdout` and not `console`: vitest intercepts console output and
  // attributes it to a TEST, so a line written at module scope by a file whose
  // only suite is then skipped is swallowed — measured.
  process.stdout.write(
    "\nprecache: the REAL-BUILD cell was SKIPPED and inventoried NOTHING — " +
      "src/local/wasm/yokohama_wasm.js is absent and YOKOHAMA_WASM_OPTIONAL=1.\n" +
      "  Run `npm run build:wasm` to check it.\n\n",
  );
}

describe.skipIf(!existsSync(glue))("what a real build actually emits", () => {
  it("lists the module, the worker chunk and the stylesheet from a real build", async () => {
    const { build } = await import("vite");
    built = mkdtempSync(join(tmpdir(), "yokohama-precache-"));
    await build({
      root: web,
      configFile: join(web, "vite.config.ts"),
      logLevel: "warn",
      build: { outDir: built, emptyOutDir: true },
    });

    const artDir = join(web, "..", "crates", "yokohama-web", "assets", "art");
    // **The premise of the exclusion below**: the three files that exist at walk
    // time ARE in the tree (`public/404.html`, and the stamp and the header set
    // the build's plugin writes), so "not listed" is the generator leaving them
    // out, not a build that never made them.
    const emitted = walk(built);
    expect(emitted, "the build emits the 404 page").toContain("404.html");
    expect(emitted, "the build emits the stamp").toContain("build-values.json");
    expect(emitted, "the build emits the header set").toContain("headers.json");
    // **The header set the build wrote** (security S2a, decision D-1): the six
    // pairs, read through the ONE reader both hosts' paths use, the policy first
    // under its Report-Only name.
    expect(existsSync(join(built, "headers.json")), "dist/headers.json exists").toBe(true);
    const headerPairs = parseHeadersJson(readFileSync(join(built, "headers.json"), "utf8"));
    expect(headerPairs, "the header set is six pairs").toHaveLength(6);
    expect(headerPairs[0]![0]).toBe("Content-Security-Policy-Report-Only");
    const distFiles = emitted.filter((f) => f !== "sw.js" && !NOT_PRECACHED.includes(f));
    const { entries } = buildPrecache({
      distFiles,
      artManifest: JSON.parse(readFileSync(join(artDir, "manifest.json"), "utf8")),
      readFile: (path) =>
        path.startsWith("/art/")
          ? readFileSync(join(artDir, path.slice("/art/".length)))
          : readFileSync(join(built!, path.slice(1))),
      sha256Hex: (data) => createHash("sha256").update(data).digest("hex"),
      moduleCeiling: Number.MAX_SAFE_INTEGER,
      totalCeiling: Number.MAX_SAFE_INTEGER,
    });
    const paths = entries.map((e) => e.path);

    // **The two Vite's own manifest does not name**, which is the measurement
    // this cell exists to hold: the module and the Worker chunk.
    expect(paths.some((p) => p.endsWith(".wasm"))).toBe(true);
    expect(paths.some((p) => /worker-.*\.js$/.test(p))).toBe(true);
    expect(paths.some((p) => p.endsWith(".css"))).toBe(true);
    expect(paths).toContain("/shell.bin");
    expect(paths).toContain("/manifest.webmanifest");

    // The icons the manifest names, because a release that installed the
    // manifest and not its icons is an install offer with no picture.
    for (const icon of [
      "/icons/icon-192.png",
      "/icons/icon-512.png",
      "/icons/icon-maskable-512.png",
      "/icons/apple-touch-icon-180.png",
    ]) {
      expect(paths, `the web app manifest names ${icon}`).toContain(icon);
    }
    const manifest = JSON.parse(readFileSync(join(built, "manifest.webmanifest"), "utf8")) as {
      icons: { src: string }[];
      id: string;
      display: string;
    };
    expect(manifest.display).toBe("standalone");
    expect(manifest.id).toBeTruthy();
    for (const { src } of manifest.icons) expect(paths, `${src} is precached`).toContain(src);

    // The fonts and the help pictures, the two cuts Task 5 made: an offline
    // launch that lost either is the thing R-6.5b2.18 exists to prevent.
    expect(paths.filter((p) => p.endsWith(".woff2")).length).toBe(2);
    expect(paths.filter((p) => p.startsWith("/help/")).length).toBe(5);

    // And the art: every picture, plus the list that names them.
    expect(paths).toContain("/art/manifest.json");
    expect(paths.filter((p) => p.startsWith("/art/") && p.endsWith(".webp")).length).toBe(131);

    // **The anti-vacuity half.** Every assertion above is a `toContain`, and a
    // walk that read the wrong directory would fail them all — but a walk that
    // read a SUPERSET (the repo, say) would pass them and be meaningless. The
    // release is a few hundred entries, not a few thousand.
    expect(entries.length).toBeGreaterThan(140);
    expect(entries.length).toBeLessThan(200);
    // Nothing from the SOURCE tree: a build's output has no `.ts` in it.
    expect(paths.filter((p) => p.endsWith(".ts") || p.endsWith(".tsx"))).toEqual([]);

    // **R-6.5c2.41: the list the GENERATOR prints, read back** (spec §10.11).
    // The edge injects a per-request script into every HTML answer, so the
    // release lists no HTML: the shell is `/shell.bin`, a byte copy of
    // `index.html` the generator writes beside it, typed as the default arm.
    execFileSync(process.execPath, [join(web, "scripts", "precache.mjs")], {
      cwd: web,
      env: { ...process.env, YOKOHAMA_WEB_DIST_DIR: built },
      stdio: "pipe",
    });
    const listed = readPrecacheList(readFileSync(join(built, "sw.js"), "utf8"), "built");
    // The generator's own walk agrees with this cell's, entry for entry.
    expect(listed).toEqual(entries);
    const shell = listed.find((e) => e.path === "/shell.bin");
    expect(shell, "the built sw.js lists the shell as /shell.bin").toBeDefined();
    expect(shell!.type).toBe("application/octet-stream");
    expect(listed.map((e) => e.path), "the built sw.js lists no /index.html").not.toContain("/index.html");
    expect(
      listed.filter((e) => e.type.startsWith("text/html")).map((e) => e.path),
      "the built sw.js lists no HTML, which the edge would rewrite",
    ).toEqual([]);
    const index = readFileSync(join(built, "index.html"));
    expect(readFileSync(join(built, "shell.bin")).equals(index), "shell.bin is index.html, byte for byte").toBe(true);
    expect(shell!.digest).toBe(createHash("sha256").update(index).digest("hex"));
    // `/shell.bin` REPLACES `/index.html` in the list rather than joining it:
    // every file the build emitted, the art manifest and its pictures — one
    // entry each, and no more.
    expect(listed.length).toBe(distFiles.length + 1 + 131);

    // **Cycle F and S2a: none of the six is an entry, and the count is what it
    // was** (spec §2.5): the list is the tree LESS the six, so the 404 page, the
    // stamp and the header set in the tree add nothing to an install's download
    // or the offline count. The 404 page is HTML too, which the edge would
    // rewrite.
    for (const name of NOT_PRECACHED) {
      expect(listed.map((e) => e.path), `/${name} is not precached`).not.toContain(`/${name}`);
    }
    expect(
      emitted.length - distFiles.length,
      "exactly the three files the build emits and the release never lists are left out",
    ).toBe(3);

    // **The stamp is exactly the six booleans** (R-FP3), in the build's order:
    // which values were set, never a value.
    const stamp = JSON.parse(readFileSync(join(built, "build-values.json"), "utf8")) as Record<string, unknown>;
    expect(Object.keys(stamp)).toEqual(STAMP_KEYS);
    for (const key of STAMP_KEYS) expect(typeof stamp[key], key).toBe("boolean");
  }, 180_000);

  // **The stamp read for what it says, through `vite.config.ts`'s DERIVATION**
  // (R-FP3; Task 2's review: no cell ran through the config's derivation of
  // the feed from the project link, and a mutant passing `null` as the link
  // survived every cell). Three real client builds, each with the developer's
  // own build values cleared and only the cell's set: every value set and the
  // feed DERIVED from a GitHub link; the same link with the feed `off`; and
  // nothing. Out of process, so the environment is the build's alone. A
  // build is under a second (measured). `site_url` (R-F12) is true exactly
  // when the site's address is set: the first build sets it, the other two do
  // not.
  // **And the header set beside the stamp** (security S2a, decision D-1): each
  // build's `headers.json` read off the SAME build — its policy names the served
  // 404 page's style by hash, and its `connect-src` carries the site's socket
  // host and the feed's origin exactly when the build values do — so the
  // config's WIRING of `headerSet`, not only the generator, is held.
  // Mutants: `null` as the feed's project link; any boolean read off the raw
  // environment rather than the checked value; a boolean hard-coded; and
  // `styleHashes: []`, `siteUrl: null` or `releaseFeed: null` in the config's
  // `headerSet` call.
  it("stamps which build values a build carries, derived as its define derives them", () => {
    const vite = join(web, "node_modules", "vite", "bin", "vite.js");
    let lastCsp = "";
    const connect = (): string[] => /connect-src ([^;]*)/.exec(lastCsp)![1]!.split(" ");
    const stampOf = (set: Record<string, string>): Record<string, unknown> => {
      const env: NodeJS.ProcessEnv = { ...process.env };
      for (const name of BUILD_VALUE_VARS) delete env[name];
      Object.assign(env, set);
      const out = mkdtempSync(join(tmpdir(), "yokohama-stamp-"));
      stamped.push(out);
      execFileSync(process.execPath, [vite, "build", "--outDir", out, "--emptyOutDir", "--logLevel", "error"], {
        cwd: web,
        env,
        stdio: "pipe",
      });
      const stamp = JSON.parse(readFileSync(join(out, "build-values.json"), "utf8")) as Record<string, unknown>;
      expect(Object.keys(stamp), "the six keys, in order").toEqual(STAMP_KEYS);
      // The build's header set, read off the SAME build: the served 404 page's style, hashed into it (after the
      // parser's newline normalization, as the generator hashes).
      const csp = parseHeadersJson(readFileSync(join(out, "headers.json"), "utf8"))[0]![1];
      const served = /<style\b[^>]*>([\s\S]*?)<\/style>/.exec(readFileSync(join(out, "404.html"), "utf8"))![1]!;
      expect(csp, "the served 404 page's style, hashed").toContain(
        `'sha256-${createHash("sha256").update(served.replace(/\r\n?/g, "\n"), "utf8").digest("base64")}'`,
      );
      lastCsp = csp;
      return stamp;
    };
    const link = "https://github.com/example/yokohama";

    expect(
      stampOf({
        YOKOHAMA_PROJECT_URL: link,
        YOKOHAMA_SITE_BLURB: "A blurb for the stamp cell.",
        YOKOHAMA_SITE_NOTICE: "A notice for the stamp cell.",
        YOKOHAMA_TURNSTILE_SITEKEY: "stamp-cell-not-a-real-key",
        YOKOHAMA_SITE_URL: "https://yokohama.example.org",
      }),
      "every value set, the feed derived from the GitHub link",
    ).toEqual({ project_url: true, blurb: true, notice: true, release_feed: true, site_url: true, turnstile_sitekey: true });
    expect(connect(), "the site's socket host and the derived feed's origin").toEqual([
      "'self'",
      "wss://yokohama.example.org",
      "https://api.github.com",
      "https://cloudflareinsights.com",
    ]);
    expect(
      stampOf({ YOKOHAMA_PROJECT_URL: link, YOKOHAMA_RELEASE_FEED: "off" }),
      "the same link with the feed off",
    ).toEqual({ project_url: true, blurb: false, notice: false, release_feed: false, site_url: false, turnstile_sitekey: false });
    expect(connect(), "no site, no feed").toEqual(["'self'", "https://cloudflareinsights.com"]);
    expect(stampOf({}), "nothing set").toEqual({
      project_url: false,
      blurb: false,
      notice: false,
      release_feed: false,
      site_url: false,
      turnstile_sitekey: false,
    });
  }, 180_000);
});
