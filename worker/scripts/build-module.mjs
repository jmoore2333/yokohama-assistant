// **The module build** — `npm run build:module` (the release) and
// `npm run build:module:debug` (the pool's) — spec §2.2: `web/`'s
// `build:wasm` and `build:wasm:debug` for this crate, with the same profiles,
// the same wasm-bindgen and the same optimizer wrapper.
//
//   release  cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker
//            wasm-bindgen --target web --out-dir gen ../target/wasm32-unknown-unknown/release-wasm/yokohama_worker.wasm
//            (and its `yokohama_worker_bg.wasm.d.ts` deleted — below says why)
//            wasm-opt, through web/scripts/wasm-opt.mjs — ONE source for the binaryen rule,
//            which REFUSES when wasm-opt is absent unless YOKOHAMA_WASM_OPT_OPTIONAL=1
//            the factory transform (scripts/factory-glue.mjs), LAST
//
//   debug    cargo build --release --features debug --target wasm32-unknown-unknown -p yokohama-worker
//            wasm-bindgen --target web --out-dir gen ../target/wasm32-unknown-unknown/release/yokohama_worker.wasm
//            (the same deletion)
//            the factory transform
//            and NO wasm-opt, as `web/`'s debug build: the module carries `debug_trap` for the
//            pool's trap cells and is never deployed (`scripts/deploy-guard.mjs` refuses it).
//
// cargo runs in the repository root, where `rust-toolchain.toml` pins the
// compiler; everything else runs here.
//
// **`gen/` is emptied FIRST, and again at any step that fails.** A build that
// fails part-way therefore leaves no module at all — neither the previous
// build's nor half of this one — and the pool and the deploy guard fail on an
// absence instead of passing over it. The factory is written LAST, after the
// optimizer, because `factory.js` is the file the guard reads: a build KILLED
// during a step (Ctrl-C at the wrapper), where no failure arm runs, leaves no
// factory either.
//
// **A release build whose optimizer changed nothing leaves `gen/.unoptimized`**
// (the orchestrator's ruling, task 7 fix round 1). The web wrapper exits 0
// both when it optimized and when `YOKOHAMA_WASM_OPT_OPTIONAL=1` let it skip
// with binaryen absent — printing "do not deploy it" — so the module's bytes
// are the only witness: unchanged across that step, the build writes the
// marker, and the deploy guard refuses while it exists. Emptying `gen/` first
// is what removes an earlier build's marker.

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { writeFactory } from "./factory-glue.mjs";

/** The marker's text: what was observed, and what reads it. */
const UNOPTIMIZED =
  "The module beside this file is UNOPTIMIZED: the release build's optimizer step left its bytes unchanged,\n" +
  "which web/scripts/wasm-opt.mjs does only when binaryen is absent and YOKOHAMA_WASM_OPT_OPTIONAL=1.\n" +
  "scripts/deploy-guard.mjs refuses while this file exists. Install binaryen and run npm run build.\n";

/** SHA-256 of a file's bytes. */
const digest = (file) => createHash("sha256").update(readFileSync(file)).digest("hex");

/** Runs one step, printing it first; a step that fails throws, naming it. */
export function spawnStep(cmd, args, cwd, where) {
  const command = [cmd === process.execPath ? "node" : cmd, ...args].join(" ");
  console.log(`build-module: ${command}${where ? `   (in ${where})` : ""}`);
  const r = spawnSync(cmd, args, { cwd, stdio: "inherit" });
  if (r.error || r.status !== 0) {
    throw new Error(`build-module: \`${command}\` failed — ${r.error ? r.error.message : `exit ${r.status ?? "by signal"}`}.`);
  }
}

/**
 * The build, in order, over the worker package at `worker`; `step` runs each
 * command and throws when one fails. Any failure empties `gen/` before it is
 * rethrown. Answers the factory's summary, and whether the release module was
 * optimized (`null` for the debug build, which runs no optimizer).
 */
export function buildModule({ debug, worker, step = spawnStep }) {
  const repo = join(worker, "..");
  const gen = join(worker, "gen");
  rmSync(gen, { recursive: true, force: true });
  mkdirSync(gen);
  try {
    const cargo = debug
      ? ["build", "--release", "--features", "debug", "--target", "wasm32-unknown-unknown", "-p", "yokohama-worker"]
      : ["build", "--profile", "release-wasm", "--target", "wasm32-unknown-unknown", "-p", "yokohama-worker"];
    step("cargo", cargo, repo, "the repository root");

    const profileDir = debug ? "release" : "release-wasm";
    step(
      "wasm-bindgen",
      ["--target", "web", "--out-dir", "gen", `../target/wasm32-unknown-unknown/${profileDir}/yokohama_worker.wasm`],
      worker,
    );

    // **wasm-bindgen's `yokohama_worker_bg.wasm.d.ts` is deleted.** It declares the
    // module's RAW instance exports, as though the file were imported under the
    // WebAssembly ESM integration — and TypeScript resolves an import of
    // `gen/yokohama_worker_bg.wasm` to it ahead of `src/wasm.d.ts`, so `compiled`
    // would be typed as that export list rather than as the `WebAssembly.Module`
    // the `CompiledWasm` rule hands over. Nothing reports it: workers-types'
    // `WebAssembly.Module` has no instance members, so any object passes for one.
    // Without the file, the import takes `src/wasm.d.ts`'s declaration.
    rmSync(join(gen, "yokohama_worker_bg.wasm.d.ts"), { force: true });

    let optimized = null;
    if (!debug) {
      const module = join(gen, "yokohama_worker_bg.wasm");
      const before = digest(module);
      step(process.execPath, ["../web/scripts/wasm-opt.mjs", "gen/yokohama_worker_bg.wasm"], worker);
      optimized = digest(module) !== before;
      if (!optimized) writeFileSync(join(gen, ".unoptimized"), UNOPTIMIZED);
    }

    return { factory: writeFactory(gen), optimized };
  } catch (e) {
    rmSync(gen, { recursive: true, force: true });
    throw e;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const { factory, optimized } = buildModule({
      debug: process.argv.slice(2).includes("--debug"),
      worker: fileURLToPath(new URL("..", import.meta.url)),
    });
    console.log(factory);
    if (optimized === false) {
      console.log(
        "build-module: gen/.unoptimized written — the optimizer step changed nothing, so this module is " +
          "UNOPTIMIZED and the deploy guard refuses it. Install binaryen and run npm run build.",
      );
    }
  } catch (e) {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(1);
  }
}
