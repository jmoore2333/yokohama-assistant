// `wasm-opt -Oz` over the modules `wasm-bindgen` just emitted, in place.
//
// **Why this is a build PREREQUISITE and not a nicety** (M6.5b2, R-6.5b2.15).
// Measured on the workstation, the same pair `Cargo.toml`'s profile comment
// carries: with `--profile release-wasm` the module is 1,782,864 B, this pass
// takes it to 1,169,222 B, and 452,820 B is what a phone actually downloads.
// That is the single largest file in the release and the one a cold start
// compiles before anything is playable, so a build that quietly skipped the
// pass would ship a module 53% larger than the one every recorded figure and
// every `npm run test:wasm` replay was taken on.
//
// So an absent `wasm-opt` is a REFUSAL that names the package to install, and
// `YOKOHAMA_WASM_OPT_OPTIONAL=1` turns it into a build that says out loud what
// it produced — the shape `YOKOHAMA_WEB_E2E_OPTIONAL`, `YOKOHAMA_WASM_OPTIONAL`
// and the BGA capture corpus already use, and for the same reason: a silent
// skip is a green build that means something different from what it says.
//
// **The VERSION matters for a figure and not for a build.** Every byte count
// here was taken with `wasm-opt 132`, and that is what CI and
// `deploy/Dockerfile` install, both from the same checksum-pinned upstream
// tarball. Debian and Ubuntu package binaryen 120, which emits a different
// module (measured: 1,174,709 B where 132 emits 1,173,055 B on the same input)
// — fine for developing against, not the build a recorded figure or a release
// budget should be read off. The refusal below names apt because that is the
// fastest way for a Linux developer to get going; it prints the version it
// used on every run, so nobody has to remember which they have.
//
// A named file that is ABSENT is also a refusal. This script is handed the
// paths `build:wasm` just wrote, so a missing one means the `wasm-bindgen` step
// above it did not produce what the next step will read, and carrying on would
// optimize nothing and report success.
//
// It is deliberately NOT run by `build:wasm:debug`: that module carries the
// `debug_trap` export the e2e's recovery cell raises, it is never deployed, and
// `-Oz` on it would cost the walk time for a build nobody downloads.
import { spawnSync } from "node:child_process";
import { existsSync, statSync } from "node:fs";

const targets = process.argv.slice(2);
if (targets.length === 0) {
  console.error("wasm-opt: no files named. Usage: node scripts/wasm-opt.mjs <file.wasm>...");
  process.exit(1);
}

const probe = spawnSync("wasm-opt", ["--version"], { encoding: "utf8" });
if (probe.error || probe.status !== 0) {
  if (process.env.YOKOHAMA_WASM_OPT_OPTIONAL === "1") {
    console.log(
      "wasm-opt: NOT RUN — this build produced an UNOPTIMIZED module.\n" +
        "  wasm-opt is absent and YOKOHAMA_WASM_OPT_OPTIONAL=1.\n" +
        "  The module is roughly 1.8 MB instead of 1.17 MB; do not deploy it.",
    );
    process.exit(0);
  }
  console.error(
    "wasm-opt: absent, and the release module is not built without it.\n" +
      "  macOS:  brew install binaryen\n" +
      "  Debian: apt-get install binaryen   (version 120 — fine to develop on,\n" +
      "          but the recorded figures and the release budget are version 132,\n" +
      "          which CI and deploy/Dockerfile install from a pinned tarball)\n" +
      "  Set YOKOHAMA_WASM_OPT_OPTIONAL=1 to build an UNOPTIMIZED module anyway.",
  );
  process.exit(1);
}

console.log(`wasm-opt: ${probe.stdout.trim()}`);

for (const file of targets) {
  if (!existsSync(file)) {
    console.error(
      `wasm-opt: ${file} is absent — the wasm-bindgen step that writes it did not run or wrote elsewhere.`,
    );
    process.exit(1);
  }
  const before = statSync(file).size;
  const run = spawnSync(
    "wasm-opt",
    ["-Oz", "--strip-debug", "--strip-producers", file, "-o", file],
    { stdio: "inherit" },
  );
  if (run.status !== 0) {
    console.error(`wasm-opt: refused ${file} (exit ${run.status ?? "signal"}).`);
    process.exit(1);
  }
  const after = statSync(file).size;
  const pct = (((before - after) / before) * 100).toFixed(1);
  console.log(`wasm-opt: ${file}  ${before} -> ${after} B  (-${pct}%)`);
}
