import { describe, expect, it } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule, type Reply } from "../gen/factory.js";
import { Instance, readReply } from "../src/module";
import { OWED_PUMP_JSON, OWED_PUMP_TABLE_JSON } from "./fixtures.gen";

const CONFIG = JSON.stringify({ archive: "off", caller: null, lobby_chat: null, chat_keep: null, chat_text: null });

/** What `f` threw — the value itself, so a cell can tell a thrown string from
 *  one wrapped in an `Error`. */
function thrown(f: () => unknown): unknown {
  try {
    f();
  } catch (e) {
    return e;
  }
  throw new Error("nothing was thrown");
}

describe("the factory (spec §2.2, §0.3 row 4)", () => {
  // Battery 12's catcher: the stock glue shares ONE instance per isolate.
  it("gives every call its own instance: two share nothing", () => {
    const a = createModule(compiled);
    const b = createModule(compiled);
    a.init(CONFIG);
    b.init(CONFIG);
    a.load("table", "x", "{}");
    expect(a.read("table", "x")).toBe("{}");
    expect(b.read("table", "x")).toBeUndefined();
  });
  it("a trapped instance leaves another usable", () => {
    const a = createModule(compiled);
    const b = createModule(compiled);
    a.init(CONFIG);
    b.init(CONFIG);
    expect(a.debug_trap, "the pool runs over the DEBUG module: npm run build:module:debug").toBeTypeOf("function");
    expect(() => a.debug_trap!()).toThrow(WebAssembly.RuntimeError);
    // The trap left `a`'s lock held, so `a` is gone for good...
    expect(() => a.read("table", "x")).toThrow(WebAssembly.RuntimeError);
    // ...and `b`, which shares nothing with it, answers.
    const r = b.dispatch("red@t.test", "GET", "/api/health", "", undefined, new Uint8Array());
    expect(r.status).toBe(200);
    r.free();
  });
});

describe("the guarded instance (the orchestrator's ruling I-29)", () => {
  it("builds and initializes its module on first use, and calls it fresh once", () => {
    const inst = new Instance(CONFIG);
    const first = inst.get();
    expect(first.fresh).toBe(true);
    // A module nobody initialized traps on its first call into the layer.
    const r = first.mod.dispatch("red@t.test", "GET", "/api/health", "", undefined, new Uint8Array());
    expect(readReply(r).status).toBe(200);
    expect(inst.get()).toEqual({ mod: first.mod, fresh: false });
  });
  it("answers what the call answers", () => {
    const inst = new Instance(CONFIG);
    inst.get().mod.load("table", "x", "{}");
    expect(inst.guarded((m) => m.read("table", "x"))).toEqual({ ok: "{}" });
  });
  it("discards a module that traps: the next one is fresh, new, and works", () => {
    const inst = new Instance(CONFIG);
    const before = inst.get().mod;
    before.load("table", "x", "{}");
    expect(inst.guarded((m) => m.debug_trap!())).toEqual({ trapped: "RuntimeError: unreachable" });
    const after = inst.get();
    expect(after.fresh).toBe(true);
    expect(after.mod).not.toBe(before);
    expect(after.mod.read("table", "x")).toBeUndefined();
  });
  // A string the module threw is a refusal it chose to make, not a trap: the
  // caller answers it, and the module that refused is still whole.
  it("passes any other throw through unchanged, and keeps the module", () => {
    const inst = new Instance(CONFIG);
    const mod = inst.get().mod;
    expect(thrown(() => inst.guarded((m) => m.read("nonsense", "x")))).toBe(
      'no such document kind: "nonsense" (record | table | chat | house)',
    );
    expect(inst.get()).toEqual({ mod, fresh: false });
  });
  it("passes init's refusal through unchanged, and keeps no module", () => {
    const bad = JSON.stringify({ archive: "s3", caller: null, lobby_chat: null, chat_keep: null, chat_text: null });
    const inst = new Instance(bad);
    const sentence = 'YOKOHAMA_WEB_ARCHIVE must be r2 or off — got "s3"';
    expect(thrown(() => inst.get())).toBe(sentence);
    // Nothing was kept: the next use builds and initializes again, and is
    // refused again, rather than being handed a module that never initialized.
    expect(thrown(() => inst.guarded((m) => m.read("table", "x")))).toBe(sentence);
  });
  it("drop() discards the module", () => {
    const inst = new Instance(CONFIG);
    const before = inst.get().mod;
    inst.drop();
    const after = inst.get();
    expect(after.fresh).toBe(true);
    expect(after.mod).not.toBe(before);
  });
});

describe("readReply (the plan's names table)", () => {
  it("reads every field, parses the bumps, and frees the reply", () => {
    const mod = new Instance(CONFIG).get().mod;
    const body = new TextEncoder().encode('{"name":"Red","text":"anyone up for a second table?"}');
    const r = mod.dispatch("red@t.test", "POST", "/api/chat", "", "application/json", body);
    expect(readReply(r)).toStrictEqual({ status: 201, body: expect.any(String), bumps: [{ bump_house_chat: null }] });
    // Freed: the glue refuses a getter on a reply it has let go of.
    expect(() => r.status).toThrow();
  });
  it("carries the archive plan and the picker row when the module sets them", () => {
    let freed = false;
    const reply = {
      status: 200,
      body: "{}",
      bumps: '[{"bump":"t"},{"bump_table":"t"},{"bump_house_chat":null},{"end":"t"}]',
      archive: '{"key":"k"}',
      picker: "[]",
      logs: "[]",
      free: () => {
        freed = true;
      },
    } as unknown as Reply;
    expect(readReply(reply)).toStrictEqual({
      status: 200,
      body: "{}",
      bumps: [{ bump: "t" }, { bump_table: "t" }, { bump_house_chat: null }, { end: "t" }],
      archive: '{"key":"k"}',
      picker: "[]",
    });
    expect(freed).toBe(true);
  });
  // A bump the shell does not know would be a frame nobody sends; refused,
  // named, instead of dropped.
  it.each(['[{"Bump":"t"}]', '[{"bump":1}]', '[{"end":"t","bump":"t"}]', '{"bump":"t"}'])(
    "refuses the bump list %s",
    (bumps) => {
      const reply = { status: 200, body: "", bumps, archive: undefined, picker: undefined, logs: "[]", free: () => {} };
      expect(() => readReply(reply as unknown as Reply)).toThrow(/^the module's bumps/);
    },
  );
});

describe("the committed fixtures reach the pool as text (Tasks 2 and 6)", () => {
  // Why a generated module of STRINGS and never a JSON import: the record's
  // seeds are above 2^53, and a JSON import would read them as numbers.
  it("keeps every digit of a seed a number would round", () => {
    const seed = /"agent_seed": (\d+)/.exec(OWED_PUMP_JSON)?.[1];
    expect(seed, "the owed-pump record's agent seed").toMatch(/^\d+$/);
    expect(BigInt(seed!) > 2n ** 53n).toBe(true);
    expect(String(Number(seed))).not.toBe(seed);
    const mod = new Instance(CONFIG).get().mod;
    mod.load("table", "owed", OWED_PUMP_TABLE_JSON);
    mod.load("record", "owed", OWED_PUMP_JSON);
    expect(mod.read("record", "owed")).toBe(OWED_PUMP_JSON);
  });
});
