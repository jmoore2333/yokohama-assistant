// **The module build** — `npm run build:module` (the release) and
// `npm run build:module:debug` (the pool's) — spec §2.2: `web/`'s
// `build:wasm` and `build:wasm:debug` for this crate, with the same profiles,
// the same wasm-bindgen and the same optimizer wrapper.
//
//   release  cargo build --profile release-wasm --target wasm32-unknown-unknown -p yokohama-worker
//            wasm-bindgen --target web --out-dir gen ../target/wasm32-unknown-unknown/release-wasm/yokohama_worker.wasm
//            (and its `yokohama_worker_bg.wasm.d.ts` deleted — below says why)
//            the factory transform (scripts/factory-glue.mjs)
//            wasm-opt, through web/scripts/wasm-opt.mjs — ONE source for the binaryen rule,
//            which REFUSES when wasm-opt is absent unless YOKOHAMA_WASM_OPT_OPTIONAL=1
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
// **`gen/` is emptied FIRST.** A build that stops part-way then leaves no
// module rather than the previous one, so the pool and the deploy guard fail
// on an absence instead of passing over a module some other build made.

import { spawnSync } from "node:child_process";
import { mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { writeFactory } from "./factory-glue.mjs";

const debug = process.argv.slice(2).includes("--debug");
const worker = fileURLToPath(new URL("..", import.meta.url));
const repo = join(worker, "..");

/** Runs one step, printing it first; any failure stops the build. */
function run(cmd, args, cwd, where) {
  const shown = cmd === process.execPath ? "node" : cmd;
  console.log(`build-module: ${[shown, ...args].join(" ")}${where ? `   (in ${where})` : ""}`);
  const r = spawnSync(cmd, args, { cwd, stdio: "inherit" });
  if (r.error || r.status !== 0) {
    console.error(`build-module: ${cmd} failed — ${r.error ? r.error.message : `exit ${r.status ?? "by signal"}`}.`);
    process.exit(1);
  }
}

rmSync(join(worker, "gen"), { recursive: true, force: true });
mkdirSync(join(worker, "gen"));

const cargo = debug
  ? ["build", "--release", "--features", "debug", "--target", "wasm32-unknown-unknown", "-p", "yokohama-worker"]
  : ["build", "--profile", "release-wasm", "--target", "wasm32-unknown-unknown", "-p", "yokohama-worker"];
run("cargo", cargo, repo, "the repository root");

const profileDir = debug ? "release" : "release-wasm";
run(
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
rmSync(join(worker, "gen", "yokohama_worker_bg.wasm.d.ts"), { force: true });

try {
  writeFactory(join(worker, "gen"));
} catch (e) {
  console.error(e instanceof Error ? e.message : String(e));
  process.exit(1);
}

if (!debug) {
  run(process.execPath, ["../web/scripts/wasm-opt.mjs", "gen/yokohama_worker_bg.wasm"], worker);
}
