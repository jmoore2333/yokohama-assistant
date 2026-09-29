// **The RELEASE module plays the browser-played pair back** (M6.5c2 cycle D,
// straw poll row 28).
//
// The determinism pin (`crates/yokohama-web/tests/browser_played_pin.rs`)
// reconstructs a pair a real browser played, and says in its sidecar that the
// game was played by the DEBUG module (`npm run build:wasm:debug`, which
// `npm run e2e` always rebuilds), with the release module "having the same
// bot". That sentence was a claim nothing held: the two builds differ by more
// than a feature (`opt-level = "z"`, fat LTO, `wasm-opt -Oz`), and it is the
// release build a player runs. Re-cutting the pair under the release module is
// blocked by the e2e's own build, so this holds the claim the other way round:
// the pair's recorded bot moves are replayed through `web/wasm-node/` — which
// ONLY `npm run build:wasm` writes — and every one of them reproduces.
//
// **Each `why` is the honesty gate, not a "the route answered".** `why`
// re-runs the bot at the same `(seed, position)` and REFUSES when its move is
// not the recorded one, so an answer with counters is a move the release
// module makes again, and any refusal but the human's own is a disagreement.
//
// **And it refuses to run against the debug module**, by name. A replay fed
// the debug build would be the old claim again under a new title, so
// [`requireReleaseModule`] reads the module's exports (`debug_trap` exists in
// the debug build only) and throws naming `npm run build:wasm`.

import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const here = path.dirname(fileURLToPath(import.meta.url));
const fixtures = path.resolve(here, "../../../crates/yokohama-web/tests/fixtures");
const modulePath = path.resolve(here, "../../wasm-node/yokohama_wasm.js");

/** The address a local game is played under (`ids.ts`'s `LOCAL_USER`). */
const LOCAL_USER = "local@browser";

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
  debug_trap?: unknown;
}

/** Throws unless `wasm` is the release module. */
export function requireReleaseModule(wasm: object): void {
  if ("debug_trap" in wasm) {
    throw new Error(
      "this replay was handed the DEBUG module (it exports debug_trap), so it would prove the " +
        "debug build against itself — run `npm run build:wasm` to build the release module the " +
        "browser ships, then run this again",
    );
  }
}

const have = fs.existsSync(modulePath);
if (!have && !process.env.YOKOHAMA_WASM_OPTIONAL) {
  throw new Error(
    "the browser host's module is not built, so this command tests nothing — run `npm run " +
      "build:wasm` first (or set YOKOHAMA_WASM_OPTIONAL=1 to skip it, which tests NOTHING)",
  );
}

describe("the release-module guard", () => {
  it("refuses a module that exports debug_trap, naming the build to run", () => {
    expect(() => requireReleaseModule({ init() {}, debug_trap() {} })).toThrow(/npm run build:wasm/);
    expect(() => requireReleaseModule({ init() {}, debug_trap() {} })).toThrow(/DEBUG module/);
  });

  it("accepts a module without it", () => {
    expect(() => requireReleaseModule({ init() {}, dispatch() {} })).not.toThrow();
  });
});

describe.skipIf(!have)("the release module replays the browser-played pair", () => {
  it("is the release module that is loaded", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
    requireReleaseModule(wasm);
  });

  it("reconstructs every bot move the browser played, and accounts for the human's", () => {
    const wasm = createRequire(import.meta.url)(modulePath) as WasmHost;
    requireReleaseModule(wasm);
    wasm.init();

    const recordText = fs.readFileSync(path.join(fixtures, "browser-played-2p.json"), "utf8");
    const tableText = fs.readFileSync(path.join(fixtures, "browser-played-2p.table.json"), "utf8");
    const id = (JSON.parse(tableText) as { id: string }).id;
    const events = (JSON.parse(recordText) as { events: unknown[] }).events.length;
    expect(events, "the pair is a played game").toBeGreaterThan(100);
    wasm.load("record", id, recordText);
    wasm.load("table", id, tableText);

    const ask = (route: string, query = "") => {
      const reply = wasm.dispatch(LOCAL_USER, "GET", route, query, undefined, new Uint8Array());
      const answer = { status: reply.status, body: reply.body };
      reply.free();
      return answer;
    };
    const opened = ask(`/api/games/${id}`);
    expect(opened.status, opened.body).toBe(200);

    let reconstructed = 0;
    let human = 0;
    const failures: string[] = [];
    for (let e = 0; e < events; e += 1) {
      const { status, body } = ask(`/api/games/${id}/why`, `event=${e}`);
      if (status !== 200) {
        failures.push(`event ${e}: HTTP ${status} ${body}`);
        continue;
      }
      const why = JSON.parse(body) as {
        refusal?: { kind?: string } | null;
        ok?: { counters?: unknown[] } | null;
      };
      if (why.refusal) {
        // The one refusal a healthy browser-played record carries: the
        // person's own move. Anything else, a mismatch above all, is the
        // release module's bot disagreeing with the one that played.
        if (why.refusal.kind === "human_seat") human += 1;
        else failures.push(`event ${e} was REFUSED: ${body}`);
      } else if (why.ok?.counters?.length === 15) {
        reconstructed += 1;
      } else {
        failures.push(`event ${e}: neither a refusal nor a granted account: ${body}`);
      }
    }
    expect(failures, "every event is a reconstructed bot move or the human's own").toEqual([]);
    expect(reconstructed, "anti-vacuity: bot moves were reconstructed").toBeGreaterThan(0);
    expect(human, "anti-vacuity: the human's moves were seen and excused").toBeGreaterThan(0);
    expect(reconstructed + human).toBe(events);
  });
});
