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
// `web/dist` and the working tree exactly as it found them.

import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, sep } from "node:path";
import { afterAll, describe, expect, it } from "vitest";
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
afterAll(() => {
  if (built) rmSync(built, { recursive: true, force: true });
});

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
    const distFiles = walk(built).filter((f) => f !== "sw.js");
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
    expect(paths).toContain("/index.html");
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
  }, 180_000);
});
