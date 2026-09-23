import { env } from "cloudflare:workers";
import { evictDurableObject, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it, vi } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule, type Exports } from "../gen/factory.js";
import * as alarmModule from "../src/alarm";
import * as configModule from "../src/config";
import { Instance, readReply } from "../src/module";
import { ensureTables, setMeta, upsertDoc } from "../src/storage";
import type { Table } from "../src/table";
import { OWED_PUMP_JSON, OWED_PUMP_TABLE_JSON } from "./fixtures.gen";
import {
  BLUE,
  CREATE_BODY,
  RED,
  call,
  logsDuring,
  seedPlaying,
  sqlOf,
  storedRows,
  tableStub,
  watchBatches,
  type Play,
} from "./helpers";

/** The host-written trap 500 (spec §2.4, §4). */
const TRAP = { error: "the table's engine stopped — try again" };

/** What `f` threw — the value itself, so a thrown string stays a string. */
function thrown(f: () => unknown): unknown {
  try {
    f();
  } catch (e) {
    return e;
  }
  throw new Error("nothing was thrown");
}

/** A fresh module in this configuration: the layer's own answer to compare
 *  an object's against. */
function referenceModule() {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  mod.set_clock(1_789_000_000);
  return mod;
}

describe("the table object (spec §2.4)", () => {
  // Battery 16: probing writes nothing — no table, no row, no alarm — and logs
  // nothing: a restore is attempted only where a record is stored. Mutants:
  // the tables created by the constructor or by a batch with nothing in it;
  // a restore at an id with no record (its 404 would be logged).
  it("probing an unknown id leaves sqlite_master empty", async () => {
    const stub = tableStub("never-created");
    const { result: res, lines } = await logsDuring(() =>
      call(stub, "red@t.test", "GET", "/api/games/never-created/table"),
    );
    expect(res.status).toBe(404);
    expect(lines).toEqual([]);
    expect(await sqlOf(stub, "SELECT count(*) AS n FROM sqlite_master")).toEqual([{ n: 0 }]);
    expect(await runInDurableObject(stub, (_o, state) => state.storage.getAlarm())).toBeNull();
  });

  // Mutants: the tables created in a batch of their own (batches 2); a write
  // outside the transaction; `written_at` in the module's seconds rather than
  // `Date.now()` (the orchestrator's ruling I-43); the create not pinned to
  // P-2's reading (the pool's is `frozen:1789000000`; unpinned, the module
  // reads the wall).
  it("a create persists its table in one batch, and the object knows its own name", async () => {
    const stub = tableStub("fresh");
    const batches = await watchBatches(stub);
    const before = Date.now();
    const res = await call(stub, "red@t.test", "POST", "/api/games", CREATE_BODY, "fresh");
    const after = Date.now();
    expect(res.status).toBe(201);
    // The reply is the table's VIEW, which carries no seed.
    expect(await res.json<{ id: string; created_at: number }>()).toMatchObject({ id: "fresh", created_at: 1_789_000_000 });
    expect(await sqlOf(stub, "SELECT kind FROM docs ORDER BY kind")).toEqual([{ kind: "table" }]);
    expect(batches.statements, "anti-vacuity: the watch saw the object's SQL").toBeGreaterThan(0);
    expect({ batches: batches.batches, outside: batches.outside }).toEqual({ batches: 1, outside: [] });
    const [row] = await sqlOf(stub, "SELECT written_at FROM docs WHERE kind = 'table'");
    expect(row?.written_at).toBeGreaterThanOrEqual(before);
    expect(row?.written_at).toBeLessThanOrEqual(after);
    // Pool fact 8: `ctx.id.name` inside the object. The alarm's side of it is
    // the three alarm cells below.
    expect(await runInDurableObject(stub, (_o, state) => state.id.name)).toBe("fresh");
  });

  // The shell compares documents after EVERY dispatch: a GET's owed pump is
  // persisted. Mutants: a compare only after a POST, or only after a 2xx; an
  // upsert that keeps a row's old `written_at`.
  it("a GET that pumps an owed bot move writes the record", async () => {
    const stub = tableStub("owed");
    // The committed pair as TEXT (never parsed: its agent seed is above 2^53),
    // written straight into `docs` under the object named after its table, at
    // an old time, so the GET's write is seen to replace it.
    await runInDurableObject(stub, (_o, state) => {
      state.storage.transactionSync(() => {
        ensureTables(state.storage.sql);
        upsertDoc(state.storage.sql, "table", OWED_PUMP_TABLE_JSON, 1);
        upsertDoc(state.storage.sql, "record", OWED_PUMP_JSON, 1);
      });
    });
    await evictDurableObject(stub);
    const before = Date.now();
    const res = await call(stub, "red@t.test", "GET", "/api/games/owed/record");
    expect(res.status, "the record route refuses a running game, after the handle is built").toBe(409);
    const [row] = await sqlOf(stub, "SELECT json, written_at FROM docs WHERE kind = 'record'");
    expect(row?.json).not.toBe(OWED_PUMP_JSON);
    expect(String(row?.json).length, "the bot's move was appended").toBeGreaterThan(OWED_PUMP_JSON.length);
    expect(row?.written_at, "the row's time is the write's").toBeGreaterThanOrEqual(before);
  });

  // Battery 9 and 21 in the pool: evictions between the clicks, a chat post
  // between them too. Mutants: the picker row never written; `restore_picker`
  // skipped, run before `load`, or run on a warm instance; a `"[]"` row kept.
  it("an open picker survives an eviction and a chat post", async () => {
    const stub = await seedPlaying("picker");
    const input = (line: string) => call(stub, RED, "POST", "/api/games/picker/input", JSON.stringify({ line }));
    const pickerRow = async () => (await sqlOf(stub, "SELECT json FROM docs WHERE kind = 'picker'")).map((r) => r.json);
    const opened = await input("1");
    expect([opened.status, (await opened.json<Play>()).prompt.kind], "premise: `1` opens PlaceA").toEqual([200, "place_a"]);
    const row = await pickerRow();
    expect(row, "the line that opened the picker is stored").toEqual([expect.stringContaining('"line":"1"')]);
    await evictDurableObject(stub);
    const chat = await call(stub, BLUE, "POST", "/api/games/picker/chat", '{"text":"still thinking?"}');
    expect(chat.status).toBe(201);
    expect((await chat.json<{ created_at: number }>()).created_at, "the dispatch was pinned too").toBe(1_789_000_000);
    expect(await pickerRow(), "a chat post leaves the row").toEqual(row);
    await evictDurableObject(stub);
    const a09 = await input("A09");
    expect([a09.status, (await a09.json<Play>()).prompt.selected], "the restored picker took A09").toEqual([200, [9]]);
    const batches = await watchBatches(stub);
    expect((await input("confirm")).status).toBe(200);
    expect({ batches: batches.batches, outside: batches.outside }, "the record and the picker row in ONE batch").toEqual({
      batches: 1,
      outside: [],
    });
    expect(await pickerRow(), "the move cleared the list, and the row is deleted").toEqual([]);
    // The STORED record's last event, as the layer words it: an eviction first,
    // so the log is read from what was written.
    await evictDurableObject(stub);
    const log = await (await call(stub, RED, "GET", "/api/games/picker/log")).json<{ entries: Array<{ text: string }> }>();
    expect(log.entries.at(-1)?.text).toBe("Red: Place 1 assistant in each of: Customs (A09)");
  });

  // The picker row takes the wake's reply first and the request's after it
  // (the plan's Task 9, step 6): a stale row the wake CLEARS gives way to the
  // picker the same request opens. The stale row is a well-formed list built at
  // another record, which `restore_picker` clears without re-feeding (Task 4).
  // Mutant: the wake's `"[]"` over the request's list — the new picker is lost
  // at the next eviction.
  it("a stale picker row the wake clears gives way to the picker the same request opens", async () => {
    const stub = await seedPlaying("picker-stale");
    const stale = JSON.stringify({ record: "0".repeat(64), lines: [{ user: RED, line: "1", expect: null }] });
    await runInDurableObject(stub, (_o, state) => {
      state.storage.transactionSync(() => upsertDoc(state.storage.sql, "picker", stale, Date.now()));
    });
    await evictDurableObject(stub);
    const input = (line: string) => call(stub, RED, "POST", "/api/games/picker-stale/input", JSON.stringify({ line }));
    const opened = await input("1");
    expect([opened.status, (await opened.json<Play>()).prompt.kind], "premise: `1` opens PlaceA").toEqual([200, "place_a"]);
    const [row] = await sqlOf(stub, "SELECT json FROM docs WHERE kind = 'picker'");
    expect(row?.json).not.toBe(stale);
    expect(row?.json).toEqual(expect.stringContaining('"line":"1"'));
    await evictDurableObject(stub);
    const a09 = await input("A09");
    expect([a09.status, (await a09.json<Play>()).prompt.selected], "the new picker survived the eviction").toEqual([200, [9]]);
  });

  // Mutants: the trap not caught; a status other than 500; anything written.
  it("a trapped dispatch persists nothing and answers 500", async () => {
    const stub = await seedPlaying("trapped");
    const before = await storedRows(stub);
    await runInDurableObject(stub, (table: Table) => {
      // The LIVE module object `Instance.get()` answers — not an RPC method of
      // the object, which the pool would refuse (pool fact 1).
      const { mod } = table["instance"]!.get();
      mod.dispatch = () => mod.debug_trap!();
    });
    const { result: res, lines } = await logsDuring(() =>
      call(stub, RED, "POST", "/api/games/trapped/input", '{"line":"1"}'),
    );
    expect(res.status).toBe(500);
    expect(await res.json()).toEqual(TRAP);
    expect(lines).toEqual([{ event: "trap", id: "trapped", call: "dispatch", error: "RuntimeError: unreachable" }]);
    expect(await storedRows(stub), "nothing persisted").toEqual(before);
    // The trap cost ONE instance: the next request builds a fresh one from
    // storage, and answers.
    expect((await call(stub, RED, "GET", "/api/games/trapped")).status).toBe(200);
    expect((await storedRows(stub)).docs).toEqual(before.docs);
  });

  // The documents are read back under the SAME guard as the dispatch (the
  // orchestrator's ruling I-42): a trap there is the trap 500, and the move the
  // module made is never kept. Mutants: the reads outside the guard (the throw
  // escapes the object); the dispatch's 200 answered anyway.
  it("a trap reading the documents back answers 500, and the move is not kept", async () => {
    const stub = await seedPlaying("trapped-read");
    const input = (line: string) => call(stub, RED, "POST", "/api/games/trapped-read/input", JSON.stringify({ line }));
    expect((await input("1")).status).toBe(200);
    expect((await input("A09")).status).toBe(200);
    const before = await storedRows(stub);
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      mod.read = () => {
        throw new WebAssembly.RuntimeError("unreachable");
      };
    });
    const { result: res, lines } = await logsDuring(() => input("confirm"));
    expect(res.status).toBe(500);
    expect(await res.json()).toEqual(TRAP);
    expect(lines).toEqual([{ event: "trap", id: "trapped-read", call: "read", error: "RuntimeError: unreachable" }]);
    expect(await storedRows(stub)).toEqual(before);
    // The client was told the move failed, and it did: a fresh instance,
    // reloaded from storage, has not placed — its picker restored, Customs
    // still selected.
    const game = await (await call(stub, RED, "GET", "/api/games/trapped-read")).json<Play>();
    expect([game.events, game.prompt.kind, game.prompt.selected]).toEqual([1, "place_a", [9]]);
  });

  // Mutants: a load trap not counted; the count not persisted; the poison
  // check after the wake (the patched `get` called a third time); the kind
  // not named.
  it("two traps during the wake poison the table, named", async () => {
    const stub = await seedPlaying("poisoned");
    const before = await storedRows(stub);
    const original = Instance.prototype.get;
    let calls = 0;
    Instance.prototype.get = function (this: Instance) {
      calls++;
      const got = original.call(this);
      // `restore_picker` builds the handle first — the replay — so this is a
      // record that traps on load.
      got.mod.restore_picker = () => {
        throw new WebAssembly.RuntimeError("unreachable");
      };
      return got;
    };
    try {
      for (const traps of [1, 2]) {
        await evictDurableObject(stub);
        const { result: res, lines } = await logsDuring(() => call(stub, RED, "GET", "/api/games/poisoned"));
        expect(res.status).toBe(500);
        expect(await res.json()).toEqual(TRAP);
        expect(lines).toEqual([
          { event: "load-trap", id: "poisoned", kind: "record", traps, error: "RuntimeError: unreachable" },
        ]);
      }
      const seen = calls;
      const { result: third, lines } = await logsDuring(() => call(stub, RED, "GET", "/api/games/poisoned"));
      expect(third.status).toBe(500);
      expect(await third.json()).toEqual({ error: "this table's stored record cannot be loaded" });
      expect(lines).toEqual([{ event: "poison", id: "poisoned", kind: "record", traps: 2 }]);
      expect(calls, "the poisoned table built no module").toBe(seen);
    } finally {
      Instance.prototype.get = original;
    }
    // Poisoned for good — the real module changes nothing — and nothing but
    // the count and the kind was written.
    const { result: still } = await logsDuring(() => call(stub, RED, "GET", "/api/games/poisoned"));
    expect(still.status).toBe(500);
    const after = await storedRows(stub);
    expect(after.docs).toEqual(before.docs);
    expect(after.meta).toEqual([
      { k: "load_traps", v: "2" },
      { k: "poison", v: "record" },
    ]);
  });

  // "A wake that succeeds resets load_traps" (spec §2.4: two CONSECUTIVE
  // traps). Mutant: the count never reset, so two traps apart would poison.
  it("a wake that succeeds clears a load trap", async () => {
    const stub = await seedPlaying("recovered");
    const traps = () => sqlOf(stub, "SELECT k, v FROM meta WHERE k IN ('load_traps', 'poison') ORDER BY k");
    const original = Instance.prototype.get;
    Instance.prototype.get = function (this: Instance) {
      Instance.prototype.get = original; // traps ONCE
      const got = original.call(this);
      got.mod.restore_picker = () => {
        throw new WebAssembly.RuntimeError("unreachable");
      };
      return got;
    };
    try {
      await evictDurableObject(stub);
      const { result: res } = await logsDuring(() => call(stub, RED, "GET", "/api/games/recovered"));
      expect(res.status).toBe(500);
      expect(await traps()).toEqual([
        { k: "load_traps", v: "1" },
        { k: "poison", v: "record" },
      ]);
    } finally {
      Instance.prototype.get = original;
    }
    expect((await call(stub, RED, "GET", "/api/games/recovered")).status).toBe(200);
    expect(await traps()).toEqual([]);
  });

  // Spec §2.4: "Any successful load resets the count" — so a load that
  // succeeds resets it even when the call after it traps. The trapped call
  // persists no document; the reset goes in a batch of its own (the
  // orchestrator's ruling, Task 9 fix round 1). Two load traps with a good
  // load between them are not consecutive, and must not poison. Mutant: the
  // reset written only by the commit, which a trapped call never reaches — the
  // second trap counts 2, and the table is poisoned.
  it("a load that succeeds resets the count even when its call then traps", async () => {
    const stub = await seedPlaying("reset-then-trap");
    const traps = () => sqlOf(stub, "SELECT k, v FROM meta WHERE k IN ('load_traps', 'poison') ORDER BY k");
    const original = Instance.prototype.get;
    // `get` patched for ONE call — the wake's — handing `f` the module it built.
    const once = (f: (mod: Exports) => void) => {
      Instance.prototype.get = function (this: Instance) {
        Instance.prototype.get = original;
        const got = original.call(this);
        f(got.mod);
        return got;
      };
    };
    const loadTrap = (mod: Exports) => {
      mod.restore_picker = () => {
        throw new WebAssembly.RuntimeError("unreachable");
      };
    };
    try {
      // A load trap: the count is 1.
      once(loadTrap);
      await evictDurableObject(stub);
      expect((await logsDuring(() => call(stub, RED, "GET", "/api/games/reset-then-trap"))).result.status).toBe(500);
      expect(await traps()).toEqual([
        { k: "load_traps", v: "1" },
        { k: "poison", v: "record" },
      ]);
      // A load that succeeds, and a call that traps.
      const docs = (await storedRows(stub)).docs;
      once((mod) => {
        mod.dispatch = () => mod.debug_trap!();
      });
      await evictDurableObject(stub);
      const trapped = await logsDuring(() => call(stub, RED, "GET", "/api/games/reset-then-trap"));
      expect(trapped.result.status).toBe(500);
      expect(trapped.lines).toEqual([
        { event: "trap", id: "reset-then-trap", call: "dispatch", error: "RuntimeError: unreachable" },
      ]);
      expect(await traps(), "the load succeeded, so the count is reset").toEqual([]);
      expect((await storedRows(stub)).docs, "the trapped call persisted no document").toEqual(docs);
      // Another load trap: the count starts again at 1.
      once(loadTrap);
      await evictDurableObject(stub);
      expect((await logsDuring(() => call(stub, RED, "GET", "/api/games/reset-then-trap"))).result.status).toBe(500);
      expect(await traps()).toEqual([
        { k: "load_traps", v: "1" },
        { k: "poison", v: "record" },
      ]);
    } finally {
      Instance.prototype.get = original;
    }
    expect((await call(stub, RED, "GET", "/api/games/reset-then-trap")).status, "not poisoned").toBe(200);
  });

  // The poison names the kind whose LOAD trapped. Every other trapping cell
  // traps in `restore_picker`, the record's replay; this one traps loading the
  // TABLE. Mutant: the kind written as `record` whatever trapped.
  it("the poison names the kind whose load trapped", async () => {
    const stub = await seedPlaying("poisoned-table");
    const original = Instance.prototype.get;
    Instance.prototype.get = function (this: Instance) {
      const got = original.call(this);
      if (got.fresh) {
        const load = got.mod.load;
        got.mod.load = (kind, id, text) => {
          if (kind === "table") throw new WebAssembly.RuntimeError("unreachable");
          load(kind, id, text);
        };
      }
      return got;
    };
    try {
      for (const traps of [1, 2]) {
        await evictDurableObject(stub);
        const { lines } = await logsDuring(() => call(stub, RED, "GET", "/api/games/poisoned-table"));
        expect(lines).toEqual([
          { event: "load-trap", id: "poisoned-table", kind: "table", traps, error: "RuntimeError: unreachable" },
        ]);
      }
      const { result: res } = await logsDuring(() => call(stub, RED, "GET", "/api/games/poisoned-table"));
      expect([res.status, await res.json()]).toEqual([500, { error: "this table's stored table cannot be loaded" }]);
    } finally {
      Instance.prototype.get = original;
    }
  });

  // An object the platform gives no name writes nothing, by construction: no
  // alarm could ever serve what it stored, because the guard skips a nameless
  // alarm. The object is reached only through `idFromString` — production's
  // nameless case, which the shell never makes — and the request names an id
  // the layer ACCEPTS, so only that rule keeps the table out of storage. The
  // reply is still the layer's, for the id the request names (the
  // orchestrator's ruling, Task 9 fix round 1). Mutant: a nameless object's
  // batch written.
  it("a nameless object answers for the id the request names, and writes nothing", async () => {
    const name = "nameless-create";
    const stub = env.TABLE.get(env.TABLE.idFromString(env.TABLE.idFromName(name).toString()));
    expect(await runInDurableObject(stub, (_o, state) => state.id.name), "premise: no name").toBeUndefined();
    const res = await call(stub, RED, "POST", "/api/games", CREATE_BODY, name);
    const body = new TextEncoder().encode(CREATE_BODY);
    const created = readReply(referenceModule().create(RED, body, name, "application/json"));
    expect(created.status, "premise: the layer accepts this id").toBe(201);
    expect([res.status, await res.text()]).toEqual([created.status, created.body]);
    expect(await sqlOf(stub, "SELECT count(*) AS n FROM sqlite_master")).toEqual([{ n: 0 }]);
  });

  // The orchestrator's ruling R-09: the plan's "ctx.id.name inside alarm()"
  // cell replaced the code under test and could not fail. This one replaces
  // the EXPORTED guard at module scope (pool fact 4), so `alarm()` passes only
  // if it consults it. Mutants: the guard inlined rather than consulted; the
  // name not passed to it; a skip that touches anything. The positive control
  // first: the real guard runs a named alarm, and nothing is logged.
  it("the alarm consults the exported guard: a skip touches nothing, and is logged", async () => {
    const stub = await seedPlaying("guarded-alarm");
    await evictDurableObject(stub);
    expect((await call(stub, RED, "GET", "/api/games/guarded-alarm")).status).toBe(200);
    const before = await storedRows(stub);
    expect(before.meta, "premise: the wake counted, so meta holds a row to keep").not.toEqual([]);
    await runInDurableObject(stub, (_o, state) => state.storage.setAlarm(Date.now() + 60_000));
    expect(await logsDuring(() => runDurableObjectAlarm(stub))).toEqual({ result: true, lines: [] });
    const skip = vi.spyOn(alarmModule, "guardAlarmName").mockReturnValue("skip");
    try {
      await runInDurableObject(stub, (_o, state) => state.storage.setAlarm(Date.now() + 60_000));
      expect(await logsDuring(() => runDurableObjectAlarm(stub))).toEqual({
        result: true,
        lines: [{ event: "alarm-without-name" }],
      });
      expect(skip).toHaveBeenCalledWith("guarded-alarm");
    } finally {
      skip.mockRestore();
    }
    expect(await storedRows(stub)).toEqual(before);
    // The alarm that ran is spent, and the skip armed none.
    expect(await runInDurableObject(stub, (_o, state) => state.storage.getAlarm())).toBeNull();
  });

  // Mutant: `onAlarm` with no guard at all.
  it("an alarm with no name logs it and touches nothing", async () => {
    const stub = await seedPlaying("nameless-call");
    // There is no slot until Task 12, so an alarm is set by hand to give
    // `getAlarm()` a value to keep.
    await runInDurableObject(stub, (_o, state) => state.storage.setAlarm(Date.now() + 60_000));
    const alarm = await runInDurableObject(stub, (_o, state) => state.storage.getAlarm());
    const before = await storedRows(stub);
    const { lines } = await logsDuring(() => runInDurableObject(stub, (table: Table) => table.onAlarm(undefined)));
    expect(lines).toEqual([{ event: "alarm-without-name" }]);
    expect(await storedRows(stub)).toEqual(before);
    expect(await runInDurableObject(stub, (_o, state) => state.storage.getAlarm())).toBe(alarm);
  });

  // MEASURED while writing this task: the pool reports a name everywhere only
  // for an object once reached BY NAME. One reached only through
  // `idFromString` has none, in `fetch` and in `alarm()` alike — production's
  // case (pool fact 8) — so here the PLATFORM delivers the nameless alarm, and
  // nothing is patched. Its rows are the owed-pump pair, so a wake and its
  // commit in the alarm's path would change the stored record. Mutant:
  // `alarm()` handing `onAlarm` anything but `ctx.id.name`.
  it("an alarm the platform delivers with no name is skipped, logged, and touches nothing", async () => {
    const stub = env.TABLE.get(env.TABLE.idFromString(env.TABLE.idFromName("nameless-platform").toString()));
    await runInDurableObject(stub, async (_o, state) => {
      expect(state.id.name, "premise: an object reached only through idFromString has no name").toBeUndefined();
      state.storage.transactionSync(() => {
        ensureTables(state.storage.sql);
        upsertDoc(state.storage.sql, "table", OWED_PUMP_TABLE_JSON, 1);
        upsertDoc(state.storage.sql, "record", OWED_PUMP_JSON, 1);
        setMeta(state.storage.sql, "wakes", "3");
      });
      await state.storage.setAlarm(Date.now() + 60_000);
    });
    const before = await storedRows(stub);
    expect(await logsDuring(() => runDurableObjectAlarm(stub))).toEqual({
      result: true,
      lines: [{ event: "alarm-without-name" }],
    });
    expect(await storedRows(stub)).toEqual(before);
    expect(await runInDurableObject(stub, (_o, state) => state.storage.getAlarm())).toBeNull();
  });

  // Battery 1 (the shell half; "before the record is read" rests on the native
  // M6.3 sweep). Mutant: the caller not passed on (a 401, not the 403).
  it("a spectator's move is refused", async () => {
    const stub = await seedPlaying("watched");
    const record = () => sqlOf(stub, "SELECT json FROM docs WHERE kind = 'record'");
    const before = await record();
    const res = await call(stub, "green@t.test", "POST", "/api/games/watched/input", '{"line":"1"}');
    expect(res.status).toBe(403);
    expect(await record()).toEqual(before);
  });

  // The orchestrator's ruling I-29: configuration, not data — the answer names
  // the variable, the line is logged, and NOTHING is written, a load trap least
  // of all. Three ways the object cannot build its module: the shell refuses
  // the configuration; the module's `init` refuses a chat value only it parses
  // (a module-scope config patch); building the module traps. Each then
  // restored, and the table answers. Mutants: any of the three counted as a
  // load trap; the sentence replaced; the object crashing instead of answering.
  it("a module the object cannot build answers 500, logged, and is never a load trap", async () => {
    const refusal = configModule.shellConfig({ ...env, YOKOHAMA_WEB_ARCHIVE: "s3" });
    if (!("error" in refusal)) throw new Error("premise: the shell refuses YOKOHAMA_WEB_ARCHIVE=s3");
    const bad = JSON.stringify({ archive: "off", caller: null, lobby_chat: "maybe", chat_keep: null, chat_text: null });
    const initSentence = thrown(() => createModule(compiled).init(bad));
    expect(initSentence, "premise: init names the variable").toEqual(expect.stringContaining("YOKOHAMA_WEB_LOBBY_CHAT"));
    const arms: Array<[string, () => () => void, unknown, unknown]> = [
      [
        "init-shell",
        () => {
          const spy = vi.spyOn(configModule, "shellConfig").mockReturnValue(refusal);
          return () => spy.mockRestore();
        },
        { error: refusal.error },
        { event: "init-refused", id: "init-shell", error: refusal.error },
      ],
      [
        "init-module",
        () => {
          const spy = vi.spyOn(configModule, "moduleConfig").mockReturnValue(bad);
          return () => spy.mockRestore();
        },
        { error: initSentence },
        { event: "init-refused", id: "init-module", error: initSentence },
      ],
      [
        "init-trap",
        () => {
          const original = Instance.prototype.get;
          Instance.prototype.get = function (this: Instance) {
            Instance.prototype.get = original;
            throw new WebAssembly.RuntimeError("unreachable");
          };
          return () => {
            Instance.prototype.get = original;
          };
        },
        TRAP,
        { event: "trap", id: "init-trap", call: "init", error: "RuntimeError: unreachable" },
      ],
    ];
    for (const [id, patch, answer, line] of arms) {
      const stub = await seedPlaying(id);
      // Read BEFORE the eviction: a `runInDurableObject` after it would build
      // the object before the patch is in, and the shell reads its
      // configuration when the object is built.
      const before = await storedRows(stub);
      await evictDurableObject(stub);
      const restore = patch();
      try {
        const { result: res, lines } = await logsDuring(() => call(stub, RED, "GET", `/api/games/${id}`));
        expect(res.status, id).toBe(500);
        expect(await res.json(), id).toEqual(answer);
        expect(lines, id).toEqual([line]);
      } finally {
        restore();
      }
      expect(await storedRows(stub), `${id}: nothing written`).toEqual(before);
      await evictDurableObject(stub);
      expect((await call(stub, RED, "GET", `/api/games/${id}`)).status, `${id}: restored, the table answers`).toBe(200);
    }
  });

  // A name no stored table can have: the store refuses it (`read` THROWS), or
  // the platform reports none — measured for a name over 1,024 bytes, which is
  // what spec §2.3's routing makes of a path's segment that long. Either way
  // the object answers what the layer answers, byte for byte, and writes
  // nothing. Mutants: the refusal of `read` escaping (the object throws); the
  // missing name passed on as `undefined` (the object throws) or `""` (the
  // create's answer differs).
  it.each([
    ["a name the store refuses", "bad!name", "bad!name"],
    ["a name over 1,024 bytes, which the platform does not report", "a".repeat(1100), undefined],
  ])("a probe at %s answers as the layer does and writes nothing", async (_what, name, reported) => {
    const stub = tableStub(name);
    expect(await runInDurableObject(stub, (_o, state) => state.id.name), "premise: the name the platform reports").toBe(
      reported,
    );
    const get = await call(stub, RED, "GET", `/api/games/${name}/table`);
    const direct = readReply(referenceModule().dispatch(RED, "GET", `/api/games/${name}/table`, "", undefined, new Uint8Array()));
    expect([get.status, await get.text()]).toEqual([direct.status, direct.body]);
    const create = await call(stub, RED, "POST", "/api/games", CREATE_BODY, name);
    const body = new TextEncoder().encode(CREATE_BODY);
    const created = readReply(referenceModule().create(RED, body, name, "application/json"));
    expect(created.status, "premise: the layer refuses this name").toBeGreaterThanOrEqual(400);
    expect([create.status, await create.text()]).toEqual([created.status, created.body]);
    expect(await sqlOf(stub, "SELECT count(*) AS n FROM sqlite_master")).toEqual([{ n: 0 }]);
  });

  // `meta.wakes` (spec §2.4): once per NEW instance, only where the object
  // holds a table, and only with the switch on. Mutants: every request
  // counted; a wake counted with the switch off.
  it("a wake is counted once per new instance, and only with the switch on", async () => {
    const stub = await seedPlaying("counted");
    const wakes = async () => (await sqlOf(stub, "SELECT v FROM meta WHERE k = 'wakes'")).map((r) => r.v);
    expect(await wakes(), "the create woke with no table stored, and the rest ran on that instance").toEqual([]);
    await evictDurableObject(stub);
    expect((await call(stub, RED, "GET", "/api/games/counted")).status).toBe(200);
    expect(await wakes()).toEqual(["1"]);
    expect((await call(stub, BLUE, "GET", "/api/games/counted/table")).status).toBe(200);
    expect(await wakes(), "the same instance: no wake").toEqual(["1"]);
    await evictDurableObject(stub);
    expect((await call(stub, RED, "GET", "/api/games/counted")).status).toBe(200);
    expect(await wakes()).toEqual(["2"]);
    const on = configModule.shellConfig(env);
    if ("error" in on) throw new Error(on.error);
    const off = vi.spyOn(configModule, "shellConfig").mockReturnValue({ ...on, countWakes: false });
    try {
      await evictDurableObject(stub);
      expect((await call(stub, RED, "GET", "/api/games/counted")).status).toBe(200);
    } finally {
      off.mockRestore();
    }
    expect(await wakes(), "the switch off: a wake is not counted").toEqual(["2"]);
  });

  // The orchestrator's ruling (Task 4's wake contract): a restore the layer
  // REFUSES — not a trap — is logged, leaves the picker row as it was, and
  // the request goes on to its own dispatch. Here the stored record does not
  // parse: `…/table` needs no replay and answers 200, and a route that needs
  // one answers the restore's own refusal. Mutants: the refusal counted as a
  // load trap; answered in place of the dispatch; the row deleted; no line.
  it("a restore the layer refuses is logged, keeps the picker row, and the request is still answered", async () => {
    const stub = await seedPlaying("unreadable");
    expect((await call(stub, RED, "POST", "/api/games/unreadable/input", '{"line":"1"}')).status).toBe(200);
    await runInDurableObject(stub, (_o, state) => {
      state.storage.transactionSync(() => upsertDoc(state.storage.sql, "record", "not json", Date.now()));
    });
    const [table] = await sqlOf(stub, "SELECT json FROM docs WHERE kind = 'table'");
    const reference = referenceModule();
    reference.load("table", "unreadable", String(table?.json));
    reference.load("record", "unreadable", "not json");
    const refused = readReply(reference.restore_picker("unreadable", "null"));
    expect(refused.status, "premise: the layer refuses a record that does not parse").not.toBe(200);
    await evictDurableObject(stub);
    const before = await storedRows(stub);
    const { result: res, lines } = await logsDuring(() => call(stub, RED, "GET", "/api/games/unreadable/table"));
    expect(res.status, "the request's own dispatch answered it").toBe(200);
    expect(lines).toEqual([{ event: "restore-refused", id: "unreadable", status: refused.status, body: refused.body }]);
    const game = await call(stub, RED, "GET", "/api/games/unreadable");
    expect([game.status, await game.text()], "a route that needs the replay").toEqual([refused.status, refused.body]);
    expect((await storedRows(stub)).docs, "the picker row and the record as they were").toEqual(before.docs);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k IN ('load_traps', 'poison')")).toEqual([]);
  });

  // A request that fails AFTER its call — here its batch — may leave the
  // module holding what storage does not, and the object outlives a throw
  // (measured). It must drop the module, or the next request would persist a
  // move its client saw fail. Driven inside the object, so the throw is caught
  // there and never printed. Mutant: no drop — the next request finds the
  // placement applied.
  it("a request whose batch fails leaves no module behind it, so its move is never applied", async () => {
    const stub = await seedPlaying("batch-failed");
    const input = (line: string) => call(stub, RED, "POST", "/api/games/batch-failed/input", JSON.stringify({ line }));
    expect((await input("1")).status).toBe(200);
    expect((await input("A09")).status).toBe(200);
    const before = await storedRows(stub);
    await runInDurableObject(stub, (table: Table, state) => {
      const transactionSync = state.storage.transactionSync;
      state.storage.transactionSync = () => {
        state.storage.transactionSync = transactionSync;
        throw new Error("the batch failed");
      };
      const confirm = new TextEncoder().encode('{"line":"confirm"}');
      expect(() =>
        table["answer"]({
          id: "batch-failed",
          user: RED,
          method: "POST",
          path: "/api/games/batch-failed/input",
          query: "",
          contentType: "application/json",
          create: false,
          body: confirm,
        }),
      ).toThrow("the batch failed");
    });
    expect(await storedRows(stub)).toEqual(before);
    const game = await (await call(stub, RED, "GET", "/api/games/batch-failed")).json<Play>();
    expect([game.events, game.prompt.kind, game.prompt.selected]).toEqual([1, "place_a", [9]]);
  });
});
