import { existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { buildModule, SOURCES, sourceDigest } from "./build-module.mjs";
import { EXPECTED } from "./factory-glue.mjs";

// wasm-bindgen's glue in miniature, as `factory-glue.test.mjs` builds it.
const glue = (names) =>
  [
    "let wasm;",
    ...names.map((n) => `export function ${n}(a) { return wasm; }`),
    "function initSync(module) { if (wasm !== undefined) return wasm; wasm = {}; }",
    "export { initSync, __wbg_init as default };",
  ].join("\n");

const dirs = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/** A worker package inside a scratch repository root, holding `files`. */
function worker(files = {}) {
  const repo = mkdtempSync(join(tmpdir(), "build-module-"));
  dirs.push(repo);
  const root = join(repo, "worker");
  mkdirSync(root);
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
}

/**
 * Stand-ins for the three tools, recording every call. wasm-bindgen writes a
 * glue, a module and the `_bg.wasm.d.ts` the build deletes; the optimizer
 * rewrites the module unless `optimizes` is false, which is what the web
 * wrapper does under `YOKOHAMA_WASM_OPT_OPTIONAL=1` with no binaryen; `fail`
 * names the tool that fails.
 */
function tools({ fail, optimizes = true, names = EXPECTED } = {}) {
  const calls = [];
  const step = (cmd, args, cwd) => {
    const tool = cmd === "cargo" ? "cargo" : cmd === "wasm-bindgen" ? "wasm-bindgen" : "wasm-opt";
    const gen = join(cwd, "gen");
    calls.push({ tool, cmd, args, cwd, factoryExists: existsSync(join(gen, "factory.js")) });
    if (tool === fail) throw new Error(`${tool} failed`);
    if (tool === "wasm-bindgen") {
      writeFileSync(join(gen, "yokohama_worker.js"), glue(names));
      writeFileSync(join(gen, "yokohama_worker_bg.wasm"), "\0asm as wasm-bindgen wrote it");
      writeFileSync(join(gen, "yokohama_worker_bg.wasm.d.ts"), "export const memory: WebAssembly.Memory;\n");
    }
    if (tool === "wasm-opt" && optimizes) writeFileSync(join(gen, "yokohama_worker_bg.wasm"), "\0asm optimized");
  };
  return { step, calls };
}

describe("the module build (spec §2.2)", () => {
  it("names the target and the package in full, as web/'s builds do (ruling S-07)", () => {
    const root = worker();
    const release = tools();
    buildModule({ debug: false, worker: root, step: release.step });
    expect(release.calls.map((c) => [c.cmd, ...c.args])).toEqual([
      ["cargo", "build", "--profile", "release-wasm", "--target", "wasm32-unknown-unknown", "-p", "yokohama-worker"],
      [
        "wasm-bindgen",
        "--target",
        "web",
        "--out-dir",
        "gen",
        "../target/wasm32-unknown-unknown/release-wasm/yokohama_worker.wasm",
      ],
      [process.execPath, "../web/scripts/wasm-opt.mjs", "gen/yokohama_worker_bg.wasm"],
    ]);
    expect(release.calls[0].cwd).toBe(dirname(root));
    const debug = tools();
    buildModule({ debug: true, worker: worker(), step: debug.step });
    expect(debug.calls.map((c) => [c.cmd, ...c.args])).toEqual([
      ["cargo", "build", "--release", "--features", "debug", "--target", "wasm32-unknown-unknown", "-p", "yokohama-worker"],
      ["wasm-bindgen", "--target", "web", "--out-dir", "gen", "../target/wasm32-unknown-unknown/release/yokohama_worker.wasm"],
    ]);
  });
  it("a release build writes the factory beside the OPTIMIZED module, with no marker", () => {
    const root = worker();
    const built = buildModule({ debug: false, worker: root, step: tools().step });
    const gen = join(root, "gen");
    expect(readFileSync(join(gen, "factory.js"), "utf8")).toMatch(/^export function createModule\(compiled\) \{/m);
    expect(readFileSync(join(gen, "yokohama_worker_bg.wasm"), "utf8")).toBe("\0asm optimized");
    expect(existsSync(join(gen, "yokohama_worker_bg.wasm.d.ts"))).toBe(false);
    expect(existsSync(join(gen, ".unoptimized"))).toBe(false);
    expect(built.optimized).toBe(true);
  });
});

// A build KILLED during the optimizer — Ctrl-C at the wrapper — runs no
// failure arm, so what it leaves is whatever was written before: the factory,
// which is the file the deploy guard reads, must not be among it.
describe("the factory", () => {
  it("is written after the optimizer, so a build killed during it leaves none", () => {
    const release = tools();
    buildModule({ debug: false, worker: worker(), step: release.step });
    const optimizer = release.calls.find((c) => c.tool === "wasm-opt");
    expect(optimizer?.factoryExists).toBe(false);
  });
});

// The script's header: a build that stops part-way leaves NO module — neither
// the previous build's nor half of this one — so the pool and the deploy guard
// fail on an absence (task 7 review, Important 2). Each cell starts from an
// earlier build's factory, so "absent" means removed.
describe("a build that stops part-way", () => {
  it.each(["cargo", "wasm-bindgen", "wasm-opt"])("at %s leaves no gen/ at all", (tool) => {
    const root = worker({ "gen/factory.js": "// an earlier build's factory\n" });
    expect(() => buildModule({ debug: false, worker: root, step: tools({ fail: tool }).step })).toThrow(
      `${tool} failed`,
    );
    expect(existsSync(join(root, "gen"))).toBe(false);
  });
  it("at the factory leaves no gen/ at all", () => {
    const root = worker({ "gen/factory.js": "// an earlier build's factory\n" });
    const step = tools({ names: EXPECTED.filter((n) => n !== "events") }).step;
    expect(() => buildModule({ debug: false, worker: root, step })).toThrow(/events/);
    expect(existsSync(join(root, "gen"))).toBe(false);
  });
});

// The web wrapper exits 0 both when it optimized and when
// `YOKOHAMA_WASM_OPT_OPTIONAL=1` let it skip, so the module's bytes are the
// only witness — and the marker is what the deploy guard reads (the
// orchestrator's ruling, task 7 fix round 1).
describe("the unoptimized marker", () => {
  it("is left when the optimizer step changed nothing", () => {
    const root = worker();
    const built = buildModule({ debug: false, worker: root, step: tools({ optimizes: false }).step });
    expect(readFileSync(join(root, "gen", ".unoptimized"), "utf8")).toMatch(/UNOPTIMIZED/);
    expect(existsSync(join(root, "gen", "factory.js"))).toBe(true);
    expect(built.optimized).toBe(false);
  });
  it("does not survive the next build", () => {
    const root = worker({ "gen/.unoptimized": "an earlier build's marker\n" });
    buildModule({ debug: false, worker: root, step: tools().step });
    expect(existsSync(join(root, "gen", ".unoptimized"))).toBe(false);
  });
  it("is never left by the debug build, which runs no optimizer", () => {
    const root = worker();
    const debug = tools();
    const built = buildModule({ debug: true, worker: root, step: debug.step });
    expect(debug.calls.map((c) => c.tool)).toEqual(["cargo", "wasm-bindgen"]);
    expect(existsSync(join(root, "gen", "factory.js"))).toBe(true);
    expect(existsSync(join(root, "gen", ".unoptimized"))).toBe(false);
    expect(built.optimized).toBeNull();
  });
});

// **The source stamp** (R3's M-3): what the module was built FROM, digested
// before cargo runs and written as `gen/source.json`, which the deploy guard
// recomputes — so a deploy after a crate change with no rebuild is refused by
// name. The digest is ONE function, `sourceDigest`, which both call.
describe("the source digest", () => {
  /** A scratch repository holding `files` (path from its root → text). */
  function repo(files) {
    const root = mkdtempSync(join(tmpdir(), "source-digest-"));
    dirs.push(root);
    for (const [path, text] of Object.entries(files)) {
      mkdirSync(dirname(join(root, path)), { recursive: true });
      writeFileSync(join(root, path), text);
    }
    return root;
  }
  const BASE = {
    "Cargo.lock": "# the lock\n",
    "crates/yokohama-worker/build.rs": "fn main() {}\n",
    "crates/yokohama-worker/src/lib.rs": "// the module\n",
    "crates/yokohama-cli/src/repl.rs": "// the prompt\n",
  };

  // Mutants: a file's bytes, a file added, one removed, or one renamed read
  // as no change; a hidden file read as a source; the lock left out.
  it("moves with every source's bytes, name and presence, and with nothing else", () => {
    const root = repo(BASE);
    const before = sourceDigest(root);
    expect(before).toMatch(/^[0-9a-f]{64}$/);
    expect(sourceDigest(root), "the same tree, the same digest").toBe(before);
    const changed = (f) => {
      const r = repo(BASE);
      f(r);
      return sourceDigest(r);
    };
    expect(changed((r) => writeFileSync(join(r, "crates/yokohama-worker/src/lib.rs"), "// the module, changed\n"))).not.toBe(before);
    expect(
      changed((r) => {
        mkdirSync(join(r, "crates/yokohama-web/src"), { recursive: true });
        writeFileSync(join(r, "crates/yokohama-web/src/api.rs"), "// added\n");
      }),
    ).not.toBe(before);
    expect(changed((r) => rmSync(join(r, "crates/yokohama-cli/src/repl.rs")))).not.toBe(before);
    expect(
      changed((r) => renameSync(join(r, "crates/yokohama-cli/src/repl.rs"), join(r, "crates/yokohama-cli/src/repl2.rs"))),
    ).not.toBe(before);
    expect(changed((r) => writeFileSync(join(r, "Cargo.lock"), "# another lock\n"))).not.toBe(before);
    expect(changed((r) => writeFileSync(join(r, "crates/yokohama-worker/src/.DS_Store"), "Finder"))).toBe(before);
    expect(
      changed((r) => {
        mkdirSync(join(r, "web/src"), { recursive: true });
        writeFileSync(join(r, "web/src/App.tsx"), "// not a source of the module\n");
      }),
    ).toBe(before);
  });

  // Every root the stamp names exists in THIS repository: a crate renamed or
  // moved would otherwise drop out of the digest as absent, and a change to it
  // would never refuse a deploy.
  it("names only roots this repository holds", () => {
    const here = fileURLToPath(new URL("../../", import.meta.url));
    for (const source of SOURCES) {
      expect(statSync(join(here, source), { throwIfNoEntry: false }), `${source} exists`).toBeDefined();
    }
    expect(SOURCES).toEqual(expect.arrayContaining(["crates/yokohama-worker", "crates/yokohama-web/src", "crates/yokohama-cli/src"]));
    expect(sourceDigest(here)).toMatch(/^[0-9a-f]{64}$/);
  });
});

describe("the source stamp", () => {
  // The digest is taken BEFORE cargo runs: a source edited while the build
  // runs leaves a stamp of the tree the build started from, which the guard
  // then refuses. Mutants: no stamp; the stamp taken after the build.
  it("stamps gen/ with the digest of the sources the build started from", () => {
    const root = worker({ "../crates/yokohama-worker/src/lib.rs": "// the module\n" });
    const repoRoot = dirname(root);
    const started = sourceDigest(repoRoot);
    const release = tools();
    const step = (cmd, args, cwd) => {
      if (cmd === "cargo") writeFileSync(join(repoRoot, "crates/yokohama-worker/src/lib.rs"), "// edited during the build\n");
      release.step(cmd, args, cwd);
    };
    buildModule({ debug: false, worker: root, step });
    expect(JSON.parse(readFileSync(join(root, "gen", "source.json"), "utf8"))).toEqual({ digest: started });
    expect(sourceDigest(repoRoot), "premise: the tree moved during the build").not.toBe(started);
  });
  it("is written by the debug build too, and goes with gen/ when a build fails", () => {
    const root = worker();
    buildModule({ debug: true, worker: root, step: tools().step });
    expect(existsSync(join(root, "gen", "source.json"))).toBe(true);
    expect(() => buildModule({ debug: false, worker: root, step: tools({ fail: "wasm-bindgen" }).step })).toThrow();
    expect(existsSync(join(root, "gen"))).toBe(false);
  });
});
