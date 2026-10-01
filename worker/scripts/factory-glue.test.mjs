import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { EXPECTED, factoryGlue } from "./factory-glue.mjs";

// wasm-bindgen's `--target web` glue in miniature: module-scope state
// (`let wasm`), exported functions over it, an `initSync` that returns early
// once `wasm` is set, and the closing export list.
const glue = (names, extra = "") =>
  [
    "let wasm;",
    "export class Reply {}",
    ...names.map((n) => `export function ${n}(a) { return wasm; }`),
    "function initSync(module) { if (wasm !== undefined) return wasm; wasm = {}; }",
    extra,
    "export { initSync, __wbg_init as default };",
  ].join("\n");

/** The transformed text, loaded as the ES module it is. */
const load = (text) => import("data:text/javascript;base64," + Buffer.from(text).toString("base64"));

describe("the factory transform (spec §2.2, §0.3 row 4)", () => {
  it("wraps the whole glue so every call builds its own instance", () => {
    const out = factoryGlue(glue(EXPECTED));
    expect(out).toMatch(/^export function createModule\(compiled\) \{/m);
    expect(out.match(/^export /gm)).toHaveLength(1);
    expect(out).toContain("initSync({ module: compiled });");
    expect(out).toContain("debug_trap: typeof debug_trap === \"function\" ? debug_trap : undefined");
  });
  it("refuses a glue that lost an expected export, naming it", () => {
    expect(() => factoryGlue(glue(EXPECTED.filter((n) => n !== "events")))).toThrow(/events/);
  });
  it("refuses an export it does not know how to wrap", () => {
    expect(() => factoryGlue(glue(EXPECTED, "export const surprise = 1;"))).toThrow(/survived/);
  });

  // The text cells above pin the SHAPE; this one runs it. Two calls must hold
  // two `wasm`s — the stock glue keeps one per module, so every object in an
  // isolate would share one `Games` (the spike's reading, spec §0.3 row 4).
  it("two instances it builds hold two states", async () => {
    const { createModule } = await load(factoryGlue(glue(EXPECTED)));
    const a = createModule("compiled");
    const b = createModule("compiled");
    expect(a.init()).toEqual({});
    expect(a.init()).not.toBe(b.init());
    expect(a.init()).toBe(a.dispatch());
  });
  it("carries debug_trap only when the glue has one", async () => {
    const debug = await load(factoryGlue(glue([...EXPECTED, "debug_trap"])));
    expect(debug.createModule("compiled").debug_trap).toBeTypeOf("function");
    const release = await load(factoryGlue(glue(EXPECTED)));
    const mod = release.createModule("compiled");
    expect(mod.debug_trap).toBeUndefined();
    expect(Object.keys(mod)).toEqual([...EXPECTED, "debug_trap"]);
  });
  // A function the transform strips of `export` and leaves out of its return
  // would be an export the shell cannot reach, dropped without a word.
  it("refuses an exported function it does not know, naming it", () => {
    expect(() => factoryGlue(glue(EXPECTED, "export function surprise(a) { return wasm; }"))).toThrow(
      /surprise/,
    );
  });
  it("refuses a glue with no initSync", () => {
    const text = glue(EXPECTED).replace(/^function initSync.*$/m, "");
    expect(() => factoryGlue(text)).toThrow(/initSync/);
  });
  // An `import` cannot live inside a function body, so a glue that grew one
  // would build a module that does not parse.
  it("refuses a top-level import", () => {
    expect(() => factoryGlue(glue(EXPECTED, 'import { x } from "./snippet.js";'))).toThrow(/import/);
  });
  // The glue's first line names ITS declarations (`yokohama_worker.d.ts`); the
  // factory's are `factory.d.ts`, so the line would be false in its output.
  it("drops the glue's @ts-self-types line", () => {
    const out = factoryGlue(`/* @ts-self-types="./yokohama_worker.d.ts" */\n${glue(EXPECTED)}`);
    expect(out).not.toContain("@ts-self-types");
  });
});

describe("the factory's declarations (the orchestrator's ruling I-22)", () => {
  // Every member but one is the generated glue's own type, by the same name
  // the transform returns — so the declarations and EXPECTED are one list.
  it("type every expected export from the generated glue, and add only debug_trap", () => {
    const template = readFileSync(new URL("./factory.d.ts.template", import.meta.url), "utf8");
    const members = [...template.matchAll(/^\s+(\w+): typeof G\.(\w+);$/gm)];
    expect(members.map(([, key, source]) => [key, source])).toEqual(EXPECTED.map((n) => [n, n]));
    expect(template).toMatch(/^\s+debug_trap\?: \(\) => never;$/m);
    expect(template).toMatch(/^import type \* as G from "\.\/yokohama_worker";$/m);
  });
});
