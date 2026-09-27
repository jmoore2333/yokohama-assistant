// **The browser host's log is the console** (M6.5c2, spec §2.6).
//
// The layer's lines were `eprintln!`, which prints NOTHING on
// `wasm32-unknown-unknown`: a local game the listing skipped — a document
// another build wrote, or a store gone bad — left no trace anywhere, the defect
// `Games::list`'s own doc names ("skipped, but never silently"). Since M6.5c2
// the browser host hands the layer a console log (`crates/yokohama-wasm`'s
// `ConsoleLog`). Natively that is a stub — the crate's own cells have no
// console to read — so the line is read HERE, through the compiled module,
// where it is real.
//
// Its own project (`vitest.node.config.ts`, `npm run test:wasm`), for
// `corpus.node.test.ts`'s reason.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));
const modulePath = path.resolve(here, "../../wasm-node/yokohama_wasm.js");

/** The three exports this file drives, declared HERE — `corpus.node.test.ts`'s
 *  ruling P-2, for its reason. */
interface WasmHost {
  init(): void;
  load(doc: string, id: string, text: string): void;
  dispatch(
    user: string | null | undefined,
    method: string,
    path: string,
    query: string,
    content_type: string | null | undefined,
    body: Uint8Array,
  ): { status: number; body: string; free(): void };
}

const have = fs.existsSync(modulePath);
if (!have && !process.env.YOKOHAMA_WASM_OPTIONAL) {
  throw new Error(
    "the browser host's module is not built, so this command tests nothing — run `npm run " +
      "build:wasm` first (or set YOKOHAMA_WASM_OPTIONAL=1 to skip it, which tests NOTHING)",
  );
}
if (!have) {
  // Straight to the file descriptor, for `corpus.node.test.ts`'s measured reason.
  process.stderr.write(
    "\nYOKOHAMA_WASM_OPTIONAL is set and the module is absent: the browser host's log ran NO\n" +
      "assertions at all.\n\n",
  );
}

describe.skipIf(!have)("the browser host's log", () => {
  // Mutants: `init` handing the layer no log (no warning); `console.error` for
  // `console.warn` (the spy sees nothing — and the browser walk would fail a
  // run on it); a line that is not one JSON object.
  it("a table the listing skips is one console warning, and the listing still answers", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
    wasm.init();
    wasm.load("table", "local-0000000000000bad", '{"schema":3}');
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      const reply = wasm.dispatch("local@browser", "GET", "/api/games", "", undefined, new Uint8Array());
      const [status, body] = [reply.status, reply.body];
      reply.free();
      expect(status, body).toBe(200);
      expect((JSON.parse(body) as { games: unknown[] }).games).toEqual([]);
      expect(warn.mock.calls.map(([line]) => JSON.parse(String(line)) as unknown)).toEqual([
        {
          event: "listing-skipped",
          id: "local-0000000000000bad",
          why: "does not parse",
          error: expect.stringContaining("missing field"),
        },
      ]);
    } finally {
      warn.mockRestore();
    }
  });
});
