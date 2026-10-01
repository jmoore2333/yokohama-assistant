import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { SMOKE_CONFIG, smokeRelease } from "./smoke-release.mjs";

/** The smallest valid WebAssembly module: the magic number and version 1. */
const EMPTY_WASM = Uint8Array.of(0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00);

const dirs = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/**
 * A `gen/` holding a fake factory. Its module records every call on
 * `globalThis.__smoke`, and answers the health request with `status` and
 * `body` — or throws `initThrows` from `init`. `debug` makes it the debug
 * build's shape: `debug_trap` is a function.
 */
function gen({ status = 200, body = '{"server_version":"0.5.0"}', initThrows, debug = false, factory = true, wasm = true } = {}) {
  const root = mkdtempSync(join(tmpdir(), "smoke-release-"));
  dirs.push(root);
  const dir = join(root, "gen");
  mkdirSync(dir);
  if (wasm) writeFileSync(join(dir, "yokohama_worker_bg.wasm"), EMPTY_WASM);
  if (factory) {
    writeFileSync(
      join(dir, "factory.js"),
      `export function createModule(compiled) {
  const calls = (globalThis.__smoke = []);
  calls.push(["createModule", compiled instanceof WebAssembly.Module]);
  return {
    init(config) {
      calls.push(["init", config]);
      ${initThrows === undefined ? "" : `throw ${JSON.stringify(initThrows)};`}
    },
    dispatch(...args) {
      calls.push(["dispatch", ...args.map((a) => (a instanceof Uint8Array ? "bytes:" + a.length : a))]);
      return { status: ${status}, body: ${JSON.stringify(body)}, free() { calls.push(["free"]); } };
    },
    debug_trap: ${debug ? "function () {}" : "undefined"},
  };
}
`,
    );
  }
  return dir;
}

describe("the release module's smoke (the orchestrator's ruling, task 7's review)", () => {
  it("passes a release module that answers the health request 200, having asked it exactly that", async () => {
    const line = await smokeRelease(gen());
    expect(line).toBe('smoke-release: the release module answered GET /api/health 200: {"server_version":"0.5.0"}');
    expect(globalThis.__smoke).toEqual([
      ["createModule", true],
      ["init", SMOKE_CONFIG],
      ["dispatch", "smoke@x.test", "GET", "/api/health", "", undefined, "bytes:0"],
      ["free"],
    ]);
    expect(JSON.parse(SMOKE_CONFIG)).toEqual({ archive: "off", caller: null, lobby_chat: null, chat_keep: null, chat_text: null });
  });

  // The refusal path the ruling asks for. Mutant: the status never read.
  it("refuses a health answer that is not 200, naming the status and the body", async () => {
    await expect(smokeRelease(gen({ status: 500, body: '{"error":"no"}' }))).rejects.toThrow(
      'smoke-release: GET /api/health answered 500, not 200: {"error":"no"}',
    );
  });
  it("refuses when init refuses the off configuration, naming the module's sentence", async () => {
    await expect(smokeRelease(gen({ initThrows: "YOKOHAMA_WEB_ARCHIVE must be r2 or off" }))).rejects.toThrow(
      "smoke-release: init refused the off configuration — YOKOHAMA_WEB_ARCHIVE must be r2 or off",
    );
  });
  // `npm run build` builds the release, but the script can be run over any
  // `gen/`; the pool's debug module is not the one that deploys.
  it("refuses the DEBUG module", async () => {
    await expect(smokeRelease(gen({ debug: true }))).rejects.toThrow(/is the DEBUG module/);
  });
  it("refuses when no module has been built", async () => {
    await expect(smokeRelease(gen({ factory: false }))).rejects.toThrow(
      /^smoke-release: gen\/factory\.js is absent — build the release module: npm run build$/,
    );
    await expect(smokeRelease(gen({ wasm: false }))).rejects.toThrow(/gen\/yokohama_worker_bg\.wasm is absent/);
  });
});
