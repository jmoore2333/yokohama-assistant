import { abortAllDurableObjects, evictDurableObject, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule } from "../gen/factory.js";
import * as configModule from "../src/config";
import type { House } from "../src/house";
import { Instance, readReply } from "../src/module";
import { cmpGen, HOUSE_NAME, type Generation, type Slot } from "../src/slot";
import { Table as TableClass } from "../src/table";
import type { Table } from "../src/table";
import { failPushes, holdApply, type Hold } from "./gates";
import {
  BLUE,
  CREATE_BODY,
  DAY_MS,
  RED,
  call,
  endMarker,
  expiryOf,
  logsDuring,
  markerOf,
  noTtl,
  operate,
  seedPlaying,
  settle,
  sqlOf,
  tableStub,
  watchBatches,
} from "./helpers";
import { walk } from "./walk";

// **The durable slot and the house over scrubbed copies** (spec §2.5,
// R-6.5c.8; the plan's Task 12). Storage is per FILE (pool fact 5): every cell
// uses ids of its own, and every house listing it asserts on is filtered to
// them. Every hold is released and restored in a `finally` (pool fact 2), and
// every revision a hold is keyed by is READ from `meta.revision` just before
// the action (the orchestrator's ruling S-12).

const STRANGER = "stranger@t.test";

/** Task 2's create body with both seats human: the create deals at once. */
const TWO_HUMANS = CREATE_BODY.replace('{"open":true}', '{"user":"blue@t.test"}');

/** The layer's host-written 409 under an unacknowledged removal (spec §2.4). */
const USED_RECENTLY = { error: "this name was used recently — try another" };

/** How long a cell waits for something it EXPECTS before failing: under
 *  vitest's 5 s per cell, so the failure names what it waited for. */
const WAIT_MS = 4_000;

/** The house, reached as the Worker reaches it: by its one name. */
function houseStub(): DurableObjectStub<House> {
  return env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME));
}

/** A request to the house as the Worker forwards one, answered whole. */
async function toHouse(user: string | undefined, method: string, path: string, body?: string) {
  const headers = new Headers();
  if (user !== undefined) headers.set("X-Yokohama-User", user);
  if (body !== undefined) headers.set("content-type", "application/json");
  const res = await houseStub().fetch(`https://yokohama.test${path}`, { method, headers, body });
  return { status: res.status, text: await res.text() };
}

/** A lobby row: the fields these cells read. A listing carries no seed. */
type Listed = { id: string; name: string | null; status: string; created_at: number; seats: Array<{ name?: string | null }> };

/** The lobby page `query` answers `user` through the house, filtered to
 *  `ids` (pool fact 5). */
async function listed(user: string, query: string, ids: string[]): Promise<Listed[]> {
  const res = await toHouse(user, "GET", `/api/games?${query}`);
  expect(res.status, `the lobby as ${user}: ${res.text}`).toBe(200);
  return (JSON.parse(res.text) as { games: Listed[] }).games.filter((g) => ids.includes(g.id));
}

/** The house's stored copy of `id`, with its `(generation, revision)`. */
async function copyRow(id: string): Promise<{ json: string; generation: Generation; revision: number } | undefined> {
  return runInDurableObject(houseStub(), (_h, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'copies'").one().n === 1;
    if (!exists) return undefined;
    const row = sql
      .exec<{ json: string; gen_at: number; gen_rand: string; revision: number }>(
        "SELECT json, gen_at, gen_rand, revision FROM copies WHERE id = ?",
        id,
      )
      .toArray()[0];
    return row && { json: row.json, generation: { at: row.gen_at, rand: row.gen_rand }, revision: row.revision };
  });
}

/** The house's retired generations for `id`. */
async function retiredOf(id: string): Promise<Generation[]> {
  return runInDurableObject(houseStub(), (_h, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'retired'").one().n === 1;
    if (!exists) return [];
    return sql
      .exec<{ gen_at: number; gen_rand: string }>("SELECT gen_at, gen_rand FROM retired WHERE id = ? ORDER BY gen_at, gen_rand", id)
      .toArray()
      .map((r) => ({ at: r.gen_at, rand: r.gen_rand }));
  });
}

/** A table object's slot state, as its `meta` holds it. */
interface SlotMeta {
  generation?: Generation;
  revision: number;
  slot?: Slot;
  acked?: { generation: Generation; revision: number; digest: string | null };
}

async function metaOf(stub: DurableObjectStub<Table>): Promise<SlotMeta> {
  const rows = await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'meta'").one().n === 1;
    return exists ? sql.exec<{ k: string; v: string }>("SELECT k, v FROM meta").toArray() : [];
  });
  const v = (k: string) => rows.find((r) => r.k === k)?.v;
  const parsed = <T>(k: string): T | undefined => {
    const text = v(k);
    return text === undefined ? undefined : (JSON.parse(text) as T);
  };
  return {
    generation: parsed<Generation>("generation"),
    revision: Number(v("revision") ?? "0"),
    slot: parsed<Slot>("slot"),
    acked: parsed<SlotMeta["acked"]>("acked"),
  };
}

/** A create of `name`, as the Worker forwards one. */
function create(name: string, body = CREATE_BODY, user = RED): Promise<Response> {
  return call(tableStub(name), user, "POST", "/api/games", body, name);
}

/** The table's stored table document, as its exact text. */
async function tableText(stub: DurableObjectStub<Table>): Promise<string> {
  const [row] = await sqlOf(stub, "SELECT json FROM docs WHERE kind = 'table'");
  if (row === undefined) throw new Error("no table document is stored");
  return String(row.json);
}

/** A fresh module in this configuration, its clock at the corpus's reading:
 *  the layer's own answers, to compare an object's against. */
function referenceModule() {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  mod.set_clock(1_789_000_000);
  return mod;
}

/** `house_copy`'s copy of a table document, from a module that holds only
 *  that document. */
function copyOf(id: string, table: string): string {
  const mod = referenceModule();
  mod.load("table", id, table);
  const copy = mod.house_copy(id);
  if (copy === undefined) throw new Error("the reference module holds no table");
  return copy;
}

/** A copy's `turn.to_move`. A copy carries no seed — the scrub zeroes it —
 *  so a cell may parse one. */
function toMove(copy: string): number | null {
  return (JSON.parse(copy) as { turn: { to_move: number } | null }).turn?.to_move ?? null;
}

/** Polls `check` until it holds; fails naming `what` after `WAIT_MS`. For a
 *  thing a cell EXPECTS, never to show that something did not happen. */
async function until(check: () => Promise<boolean>, what: string): Promise<void> {
  const deadline = Date.now() + WAIT_MS;
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(`${what}: not within ${WAIT_MS} ms`);
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

/** The pending alarm, in milliseconds, or `null`. */
function alarmOf(stub: DurableObjectStub<Table>): Promise<number | null> {
  return runInDurableObject(stub, (_t, state) => state.storage.getAlarm());
}

/** A hand-built push of `copy` (null: a REMOVAL) at `(generation, revision)`. */
function pushOf(id: string, generation: Generation, revision: number, copy: string | null): Slot {
  return { id, generation, revision, copy, last_active: 1, bytes: 1, attempt: 0 };
}

/** A push straight into the house's `apply`, inside the object. */
function applyInHouse(slot: Slot) {
  return runInDurableObject(houseStub(), (house: House) => house.apply(slot));
}

/** `house_copy` TRAPS in the next `times` NEW modules built — the module a
 *  copy is computed on is always a new one — at MODULE scope (pool fact 4).
 *  The table's live module and the house's are warm in the cells that use
 *  it, so no other module counts. The patch removes itself after the last
 *  trap; restore it in a `finally` all the same. */
function trapCopies(times: number): { restore(): void } {
  const original = Instance.prototype.get;
  let left = times;
  Instance.prototype.get = function (this: Instance) {
    const got = original.call(this);
    if (got.fresh && left > 0) {
      left -= 1;
      if (left === 0) Instance.prototype.get = original;
      got.mod.house_copy = () => {
        throw new WebAssembly.RuntimeError("unreachable");
      };
    }
    return got;
  };
  return {
    restore() {
      Instance.prototype.get = original;
    },
  };
}

/** `meta.copy_owed`, as its rows: the `house_copy` traps in a row. */
async function copyOwed(stub: DurableObjectStub<Table>): Promise<unknown[]> {
  return (await sqlOf(stub, "SELECT v FROM meta WHERE k = 'copy_owed'")).map((r) => r.v);
}

/** A client socket on the house's lobby stream as `user`, every message kept. */
async function lobbySocket(user: string) {
  const res = await houseStub().fetch("https://yokohama.test/api/stream", {
    headers: { Upgrade: "websocket", "X-Yokohama-User": user },
  });
  const ws = res.webSocket;
  if (ws === null) throw new Error(`the upgrade was answered ${res.status}, unupgraded`);
  ws.accept();
  const messages: string[] = [];
  ws.addEventListener("message", (e) => {
    messages.push(typeof e.data === "string" ? e.data : "<binary>");
  });
  return {
    ws,
    messages,
    until: async (n: number) => {
      await until(async () => messages.length >= n, `${n} lobby messages (arrived: ${JSON.stringify(messages)})`);
      return [...messages];
    },
  };
}

describe("the durable slot (spec §2.5)", () => {
  // Mutants: no slot written at the create; the slot never pushed; the house
  // not listing its copies (its module never handed them), or listing a row
  // that is not the layer's own over the original; a generation whose time is
  // not the shell's `Date.now()` in MILLISECONDS (the orchestrator's F-2; the
  // P-2 reading in seconds is the mutant); the slot's last activity written
  // as a constant, or its size as 0; a refused create at a taken name
  // writing a slot. A refused
  // create at an id that holds nothing is `table.test.ts`'s probe cell ("a
  // probe at %s …", a create at `bad!name` leaves `sqlite_master` empty).
  it("a create's copy reaches the house and the lobby lists it", async () => {
    const id = "slot-a";
    const stub = tableStub(id);
    const before = Date.now();
    expect((await create(id)).status).toBe(201);
    const after = Date.now();
    await runInDurableObject(stub, (table: Table) => table.deliverSlot());
    const rows = await listed(STRANGER, "scope=open", [id]);
    // The NAS's answer: the layer's own listing over the ORIGINAL document.
    const nas = referenceModule();
    expect(readReply(nas.create(RED, new TextEncoder().encode(CREATE_BODY), id, "application/json")).status).toBe(201);
    const original = readReply(nas.dispatch(STRANGER, "GET", "/api/games", "scope=open", undefined, new Uint8Array()));
    const want = (JSON.parse(original.body) as { games: Listed[] }).games.filter((g) => g.id === id);
    expect(want.length, "premise: the layer lists it for a stranger").toBe(1);
    expect(rows, "the house's row is the layer's row over the original").toEqual(want);
    const meta = await metaOf(stub);
    expect(meta.generation?.at, "the generation's time is the create's Date.now(), in milliseconds").toBeGreaterThanOrEqual(before);
    expect(meta.generation?.at).toBeLessThanOrEqual(after);
    expect(meta.generation?.rand).toMatch(/^[0-9a-f]{16}$/);
    expect([meta.revision, meta.slot, meta.acked?.revision]).toEqual([1, undefined, 1]);
    // The slot's last activity and size, as the house stored them: the table
    // document's own `written_at` — the only one of record, table and chat a
    // create writes — and the object's SQLite size, which is never 0.
    const [written] = await sqlOf(stub, "SELECT written_at FROM docs WHERE kind = 'table'");
    const [stats] = await runInDurableObject(houseStub(), (_h, state) =>
      state.storage.sql.exec<{ last_active: number; bytes: number }>("SELECT last_active, bytes FROM copies WHERE id = ?", id).toArray(),
    );
    expect(stats?.last_active, "the house's last activity is the table's newest written_at").toBe(written?.written_at);
    expect(stats?.bytes, "the house holds a size, and an SQLite size is never 0").toBeGreaterThan(0);

    const again = await create(id);
    expect([again.status, await again.json()], "premise: the layer refuses a taken name").toEqual([
      400,
      { error: `a game named "${id}" already exists` },
    ]);
    expect(await metaOf(stub), "a refused create writes no generation and no slot").toEqual(meta);
  });

  // Battery 5. Mutants: every push applied; the revision compared with `>=`.
  it("a lower revision after a higher one leaves the row at the higher (battery 5)", async () => {
    const id = "slot-b";
    const nas = referenceModule();
    expect(readReply(nas.create(RED, new TextEncoder().encode(CREATE_BODY), id, "application/json")).status).toBe(201);
    const open = nas.house_copy(id)!;
    expect(readReply(nas.dispatch(BLUE, "POST", `/api/games/${id}/join`, "", "application/json", new TextEncoder().encode('{"seat":1}'))).status).toBe(200);
    const dealing = nas.house_copy(id)!;
    expect(dealing, "premise: the join changed the copy").not.toBe(open);
    const g: Generation = { at: 1_789_000_000, rand: "00000000000000b5" };
    expect(await applyInHouse(pushOf(id, g, 2, dealing))).toEqual({ generation: g, revision: 2 });
    const { result, lines } = await logsDuring(() => applyInHouse(pushOf(id, g, 1, open)));
    expect(result, "the house acknowledges the push it ignores").toEqual({ generation: g, revision: 1 });
    expect(lines).toEqual([{ event: "slot-ignored", id, generation: g, revision: 1, current: { generation: g, revision: 2 } }]);
    expect(await copyRow(id)).toEqual({ json: dealing, generation: g, revision: 2 });
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status)).toEqual(["dealing"]);
  });

  // Battery 6: the slot is cleared only by the acknowledgement. Its mutant —
  // a push that deletes the slot before the house's RPC answers — fails the
  // premise that the create's slot still holds r while r's push is held. And
  // battery 35's SECOND catcher: its mutant is `acked` recorded without
  // `advances`, and r's late ack would regress it to r. This arm CANNOT see
  // battery 22 — by the time r's ack lands the slot is already empty, so a
  // mutant that "clears a newer slot" clears nothing (re-review N2, measured).
  it("a late ack of r leaves acked at r+1 (battery 35's SECOND catcher, battery 6)", async () => {
    const id = "slot-c";
    const stub = tableStub(id);
    const r = (await metaOf(stub)).revision + 1;
    const held = holdApply([[id, r]]);
    try {
      expect((await create(id)).status).toBe(201);
      await until(async () => held.reached(id, r), "the create's push reaches its gate");
      expect((await metaOf(stub)).slot?.revision, "premise: the create's slot is r").toBe(r);
      const next = (await metaOf(stub)).revision + 1;
      expect(next, "premise: the join is r+1").toBe(r + 1);
      expect((await call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}')).status).toBe(200);
      await until(async () => {
        const m = await metaOf(stub);
        return m.acked?.revision === next && m.slot === undefined;
      }, "r+1 applied and acknowledged, the slot cleared");
      held.release(id, r);
      await settle(stub);
      expect(held.settled(id, r), "premise: r's push ran through the house").toBe(true);
    } finally {
      held.restore();
      await settle(stub);
    }
    const after = await metaOf(stub);
    expect(after.slot, "the slot stays EMPTY").toBeUndefined();
    expect(after.acked?.revision, "acked stays r+1").toBe(r + 1);
    expect((await copyRow(id))?.revision, "the house holds r+1").toBe(r + 1);
  });

  // Battery 22 (re-review N2's fixed arm). BOTH the keep's push and the
  // removal's are held; the keep's is let through first, so its ack arrives
  // while the removal still waits. THE DISCRIMINATING ASSERTION is the slot
  // in between: the mutant — the slot cleared on ANY ack — empties it there.
  // The end state is the same either way, because the held removal completes
  // on release: the removal acknowledged, and the delete's marker standing
  // (spec §2.4 step 6; the object ends only after the marker's 90 days).
  it("a stale ack of r+2 leaves a REMOVAL waiting in the slot (battery 22)", async () => {
    const id = "slot-c2";
    const stub = tableStub(id);
    expect((await create(id, TWO_HUMANS)).status).toBe(201);
    await settle(stub);
    const n = (await metaOf(stub)).revision;
    const [keep, removal] = [n + 1, n + 2];
    const held = holdApply([
      [id, keep],
      [id, removal],
    ]);
    try {
      expect((await call(stub, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}')).status).toBe(200);
      await until(async () => held.reached(id, keep), "the keep's push reaches its gate");
      expect((await metaOf(stub)).slot?.revision, "premise: a keep's presence is in the scrub").toBe(keep);
      expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status, "premise: the creator deletes the dealing table").toBe(204);
      await until(async () => held.reached(id, removal), "the removal's push reaches its gate");
      const waiting = (await metaOf(stub)).slot;
      expect([waiting?.revision, waiting?.copy], "premise: the slot holds the REMOVAL").toEqual([removal, null]);
      held.release(id, keep);
      await until(async () => (await metaOf(stub)).acked?.revision === keep, "the keep's ack is read");
      const between = (await metaOf(stub)).slot;
      expect([between?.revision, between?.copy], "the removal still waits in the slot").toEqual([removal, null]);
      held.release(id, removal);
      await settle(stub);
    } finally {
      held.restore();
      await settle(stub);
    }
    expect(await copyRow(id), "the house dropped the row").toBeUndefined();
    expect((await retiredOf(id)).length, "and retired the generation").toBe(1);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k IN ('marker', 'slot') ORDER BY k"), "the removal acknowledged; the marker stands").toEqual([
      { k: "marker" },
    ]);
  });

  // The orchestrator's ruling on Task 12's question (re-review m-g): a late
  // acknowledgement's batch is written only while the object holds its SQL
  // tables. A straggler — the create's push, held — is let through after the
  // object has ended: an operator's force, which answers only once the house
  // has acknowledged its removal and `deleteAll()` has run (spec §2.9; a
  // creator's delete would leave its marker for 90 days). The house ignores
  // the straggler (its generation is retired) and acknowledges it, and the
  // table must recreate nothing. `sqlite_master` is empty before the
  // straggler and after it.
  // Mutant: the guard dropped — the straggler's `acked` recreates `meta`.
  // Battery 22's cell is the other half: there the tables are present, and a
  // stale acknowledgement leaves the REMOVAL in place.
  it("a straggling acknowledgement after the object ended writes nothing", async () => {
    const id = "slot-q";
    const stub = tableStub(id);
    const tables = async () => (await sqlOf(stub, "SELECT count(*) AS n FROM sqlite_master"))[0]?.n;
    const r = (await metaOf(stub)).revision + 1;
    const held = holdApply([[id, r]]);
    try {
      expect((await create(id)).status).toBe(201);
      await until(async () => held.reached(id, r), "the create's push reaches its gate");
      expect((await operate(stub, id, "boss@t.test", true)).status, "the force, its removal not held, ends the object").toBe(204);
      expect(await tables(), "premise: the object has ended").toBe(0);
      held.release(id, r);
      await settle(stub);
      expect(held.settled(id, r), "premise: the straggler ran through the house").toBe(true);
    } finally {
      held.restore();
      await settle(stub);
    }
    expect(await tables(), "the straggler's acknowledgement recreated nothing").toBe(0);
    expect(await copyRow(id), "and the house lists nothing for it").toBeUndefined();
  });

  // Battery 35. With two humans the scrubbed copy alternates, because
  // `turn.to_move` flips: seat 1 to move (X), then seat 0 (Y), then seat 1
  // again (X, BYTE-equal to the first). The walk's own table `wire`, in the
  // stretch the re-review measured: from row 57 to game over the copy strictly
  // alternates, with no house-answered row inside it. THE HOLD GOES IN BEFORE
  // THE ROW (re-review N1: watching the slot after a row is vacuous — the push
  // lands during the watch). Under the mutant — `acked` recorded without the
  // advance check — r's late ack regresses `acked` to X, the change back to X
  // equals it, no push is made, and the house's row still shows to_move 0.
  it("acked never regresses, so a copy that returns to an earlier value is still delivered (battery 35)", { timeout: 300_000 }, async () => {
    const id = "wire";
    const stub = tableStub(id);
    let hold: Hold | undefined;
    let armed = 0;
    let r = 0;
    let x = "";
    let stage: "seek" | "wait-y" | "seek-x" = "seek";
    let back = false;
    const arm = async () => {
      armed = (await metaOf(stub)).revision + 1;
      hold = holdApply([[id, armed]]);
    };
    try {
      await walk({
        onRow: async (index, row, reply) => {
          expect(reply.status, `row ${index}: ${row.method} ${row.path}`).toBe(row.status);
          if (index < 56) return;
          // Row 56 answered: the hold for row 57's revision goes in.
          if (index === 56) return arm();
          if (stage === "seek") {
            const m = await metaOf(stub);
            if (m.revision < armed) return;
            if (m.slot?.revision === armed && m.slot.copy !== null && toMove(m.slot.copy) === 1) {
              expect(hold!.settled(id, armed), "premise: r's push is held, not settled").toBe(false);
              expect(toMove((await copyRow(id))!.json), "premise: the house still shows the previous copy, Y").toBe(0);
              r = armed;
              x = m.slot.copy;
              stage = "wait-y";
              return;
            }
            // A Y: let it through, and hold the next revision.
            hold!.release(id, armed);
            hold!.restore();
            await settle(stub);
            return arm();
          }
          if (stage === "wait-y") {
            if ((await metaOf(stub)).revision < r + 1) return;
            await until(async () => {
              const m = await metaOf(stub);
              return m.acked?.revision === r + 1 && m.slot === undefined;
            }, "r+1 applied and acknowledged");
            expect(toMove((await copyRow(id))!.json), "premise: r+1 is Y, and the house holds it").toBe(0);
            // r's ack arrives LATE.
            hold!.release(id, r);
            await settle(stub);
            expect(hold!.settled(id, r), "premise: r's push ran through the house").toBe(true);
            hold!.restore();
            stage = "seek-x";
            return;
          }
          if (copyOf(id, await tableText(stub)) !== x) return;
          back = true;
          return "stop";
        },
      });
    } finally {
      hold?.restore();
      await settle(stub);
    }
    expect(back, "premise: the walk reached the change back to X, byte for byte").toBe(true);
    await runInDurableObject(stub, (table: Table) => table.deliverSlot());
    expect(toMove((await copyRow(id))!.json), "the house's row for wire shows X again").toBe(1);
  });

  // The request's push fails, and then the ALARM's push fails too: the alarm
  // that ran is spent, so only the failure's own re-arm — at the backoff's
  // second rung, 2 min — leaves one to run. The pushes fail on the table's
  // side (`failPushes`), so the run prints nothing. It runs with no TTL
  // (`noTtl`): under one, every alarm run ends by re-arming to everything
  // still due, this retry among them, which would stand in for the failure's
  // own re-arm. Mutants: a failure that re-arms nothing (no alarm is left
  // after the alarm's failed push); a failure not counted in the slot, or a
  // ladder off by a rung (the retry at 30 s); an alarm that does not deliver.
  it("a failed push is retried by the alarm, on the backoff", async () => {
    const id = "slot-e";
    const stub = tableStub(id);
    const ttl = noTtl();
    const failure = (attempt: number, retry: number) => ({
      event: "slot-failed",
      id,
      generation: expect.any(Object),
      revision: 1,
      attempt,
      retry_in: retry,
      error: "Error: the house is unreachable (failPushes)",
    });
    const pushed = (attempt: number) => ({ event: "slot-pushed", id, generation: expect.any(Object), revision: 1, removal: false, attempt });
    try {
      const failing = await failPushes(stub, 2);
      try {
        const { lines } = await logsDuring(async () => {
          expect((await create(id)).status).toBe(201);
          await settle(stub);
        });
        expect(lines, "premise: the request's push failed").toEqual([pushed(0), failure(0, 30_000)]);
        expect(await copyRow(id), "premise: the house holds no copy").toBeUndefined();
        expect((await metaOf(stub)).slot?.attempt, "the failure is counted").toBe(1);
        const before = Date.now();
        const { result, lines: again } = await logsDuring(() => runDurableObjectAlarm(stub));
        expect([result, again], "premise: the alarm's push failed too").toEqual([true, [pushed(1), failure(1, 120_000)]]);
        expect(await alarmOf(stub), "the failure re-armed the alarm at the second rung").toBeGreaterThanOrEqual(before + 120_000);
      } finally {
        await failing.restore();
      }
      expect(await runDurableObjectAlarm(stub)).toBe(true);
      expect((await listed(STRANGER, "scope=open", [id])).map((r) => r.id)).toEqual([id]);
      expect((await metaOf(stub)).slot).toBeUndefined();
      expect(await alarmOf(stub), "with no TTL, the acknowledgement deleted the alarm").toBeNull();
    } finally {
      ttl.restore();
    }
  });

  // `deliverSlot` pushes one slot once at a time: a second call while that
  // slot's push is in flight answers the same push — how the corpus replay's
  // flush waits for the push its request started, rather than sending
  // another. The push is held until BOTH calls have entered `deliverSlot` —
  // counted by a spy on the prototype, which calls through. Mutant: every
  // call pushes (a second `slot-pushed`, and the house ignoring the repeat).
  it("a slot already in flight is not pushed again: a second deliverSlot answers the first push", async () => {
    const id = "slot-p";
    const stub = tableStub(id);
    const r = (await metaOf(stub)).revision + 1;
    const held = holdApply([[id, r]]);
    const calls = vi.spyOn(TableClass.prototype, "deliverSlot");
    try {
      const { lines } = await logsDuring(async () => {
        expect((await create(id)).status).toBe(201);
        await until(async () => held.reached(id, r), "the create's push reaches its gate");
        const second = runInDurableObject(stub, (table: Table) => table.deliverSlot());
        await until(async () => calls.mock.calls.length >= 2, "the second deliverSlot is under way");
        held.release(id, r);
        await second;
        await settle(stub);
      });
      expect(lines.map((l) => (l as { event: string }).event)).toEqual(["slot-pushed", "slot-acked"]);
    } finally {
      calls.mockRestore();
      held.restore();
      await settle(stub);
    }
    expect((await metaOf(stub)).acked?.revision).toBe(r);
  });

  // Battery 25. The FIRST text used evictDurableObject, which drains the push
  // (vacuous); the SECOND patched the instance, which makes the RPC THROW
  // rather than hang (vacuous again — measured true/true by the plan's
  // review). Here the push hangs on a PROTOTYPE gate, neither failing nor
  // succeeding, and `abortAllDurableObjects()` does not drain it. The alarm
  // the earlier acknowledgements left — the table's expiry, 90 days out, which
  // the pool would run on demand all the same — is cleared first, so the only
  // alarm left is one the finalizing keep's batch set. Mutant: the alarm armed
  // only after a failure — no alarm exists, and the run is FALSE.
  it("the alarm is set IN the batch: an abort mid-push is finished by the alarm", async () => {
    const id = "slot-f";
    const before = tableStub(id);
    expect((await create(id, TWO_HUMANS)).status).toBe(201);
    expect((await call(before, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}')).status).toBe(200);
    await settle(before);
    await runInDurableObject(before, (_t, state) => state.storage.deleteAlarm());
    expect(await alarmOf(before), "premise: no alarm but the keep's batch can arm one").toBeNull();
    const n = (await metaOf(before)).revision;
    const held = holdApply();
    try {
      // Blue's FINALIZING keep: the status becomes playing, so the copy
      // changes (red's opening move does NOT change it: re-review m-f).
      expect((await call(before, BLUE, "POST", `/api/games/${id}/keep`, '{"seat":1,"card":1}')).status).toBe(200);
      await until(async () => held.reached(id, n + 1), "the finalizing keep's push reaches its gate");
      expect((await metaOf(before)).slot?.revision, "premise: the slot holds the new revision").toBe(n + 1);
      await abortAllDurableObjects();
    } finally {
      held.restore();
      // The abort left the object to a new instance, which this stub
      // reaches: the push the aborted one held is not its to settle.
      await settle(tableStub(id));
    }
    const fresh = tableStub(id);
    expect(await runDurableObjectAlarm(fresh)).toBe(true);
    await settle(fresh);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status)).toEqual(["playing"]);
  });

  // Spec §2.4: "a write that makes nothing due … leaves the alarm where it
  // is". The alarm is set BY HAND to a value of the cell's own — not the
  // expiry deadline the last acknowledgement armed, which a write that
  // re-armed to the new last activity would move only by milliseconds — so a
  // write that touched it is seen. Mutants: a slot written on every request;
  // the alarm set, or deleted, by every write.
  //
  // **Red's SECOND mid-turn move, since cycle E** (ruling T5-a): the table's
  // mirror carries `has_events`, which the FIRST applied move of a game flips
  // — the one table write, and so the one copy pushed to the house, that the
  // bit costs per game. The opening move is asserted as exactly that; the
  // move after it, with seat 0 still to move, is the one that must leave
  // everything alone. A count in its place (a write per move) is the mutant
  // this second half catches.
  it("a move that leaves the copy alone writes no slot and leaves the alarm where it was", async () => {
    const id = "slot-g";
    const stub = tableStub(id);
    expect((await create(id)).status).toBe(201);
    expect((await call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}')).status).toBe(200);
    expect((await call(stub, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}')).status).toBe(200);
    expect((await call(stub, BLUE, "POST", `/api/games/${id}/keep`, '{"seat":1,"card":1}')).status).toBe(200);
    await settle(stub);
    const dealt = await metaOf(stub);
    expect(dealt.slot, "premise: delivered").toBeUndefined();
    const unmoved = await tableText(stub);
    const opening = await call(stub, RED, "POST", `/api/games/${id}/input`, '{"line":"1"}');
    const opened = await opening.json<{ events: number; to_move: number }>();
    expect([opening.status, opened.events, opened.to_move], "premise: red's opening move applies").toEqual([200, 1, 0]);
    await settle(stub);
    const first = await tableText(stub);
    expect(first, "the opening move flips the mirror's has_events: the one write per game").not.toBe(unmoved);
    expect(JSON.parse(first).turn.has_events).toBe(true);
    expect((await metaOf(stub)).revision, "and the one copy it pushes").toBe(dealt.revision + 1);

    const a = Date.now() + 3_600_000;
    await runInDurableObject(stub, (_t, state) => state.storage.setAlarm(a));
    const before = await metaOf(stub);
    expect(before.slot, "premise: the opening move's copy was delivered").toBeUndefined();
    // "3" is "Place no assistants": a move with no picker, after which red
    // is still to move (the movement phase).
    const moved = await call(stub, RED, "POST", `/api/games/${id}/input`, '{"line":"3"}');
    const second = await moved.json<{ events: number; to_move: number }>();
    expect([moved.status, second.events, second.to_move], "premise: red's second move applies, and red is still to move").toEqual([200, 2, 0]);
    await settle(stub);
    expect(await tableText(stub), "premise: seat 0 is still to move, so the table document is unchanged").toBe(first);
    const after = await metaOf(stub);
    expect([after.slot, after.revision]).toEqual([undefined, before.revision]);
    expect(await alarmOf(stub)).toBe(a);
    await runInDurableObject(stub, (_t, state) => state.storage.deleteAlarm());
  });

  // Battery 36 (spec §2.4): a new table's first copy can never overwrite a
  // removal the house has not received. After the house's acknowledgement the
  // delete's marker still stands, and a create is still 409; once its 90 days
  // are over the object ends, and the id is free. The re-create runs under
  // the FROZEN clock, the same second as the table it replaced, and its
  // generation still orders after the retired one by its time — the shell's
  // milliseconds — never by the coin toss of its random half (the
  // orchestrator's F-2; plan review m7 stepped around it with a ticking
  // clock). Mutants: no 409 (the create lands and its copy would race the
  // removal); the object never ended after the marker (the re-create meets
  // the layer's "already exists", or finds the old slot); the generation's
  // time the P-2 reading in seconds (the two times tie).
  it("a create while a removal is unacknowledged is 409; once the marker has ended the object is gone and the id is free", async () => {
    const id = "slot-h";
    const stub = tableStub(id);
    const created = await create(id);
    expect(created.status).toBe(201);
    const firstCreatedAt = (await created.json<{ created_at: number }>()).created_at;
    await settle(stub);
    const first = (await metaOf(stub)).generation!;
    const held = holdApply();
    try {
      expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status).toBe(204);
      const pending = await metaOf(stub);
      expect(pending.slot?.copy, "premise: a REMOVAL waits").toBeNull();
      const again = await create(id);
      expect([again.status, await again.json()]).toEqual([409, USED_RECENTLY]);
      expect(await metaOf(stub), "the 409 wrote nothing").toEqual(pending);
      held.release();
      await runInDurableObject(stub, (table: Table) => table.deliverSlot());
    } finally {
      held.restore();
      await settle(stub);
    }
    expect((await metaOf(stub)).slot, "premise: the house acknowledged the removal").toBeUndefined();
    const marked = await create(id);
    expect([marked.status, await marked.json()], "after the ack the marker stands, and a create is still 409").toEqual([409, USED_RECENTLY]);
    await endMarker(stub);
    const recreated = await create(id);
    expect(recreated.status).toBe(201);
    const createdAt = (await recreated.json<{ created_at: number }>()).created_at;
    expect(createdAt, "premise: the re-create is in the same second as the table it replaced").toBe(firstCreatedAt);
    await settle(stub);
    const second = (await metaOf(stub)).generation!;
    expect(second.at, "the new generation's time is later, in milliseconds").toBeGreaterThan(first.at);
    expect(cmpGen(second, first), "so it orders after the retired one").toBeGreaterThan(0);
    expect((await copyRow(id))?.generation, "the house applied the NEW table's copy").toEqual(second);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.created_at), "and the lobby lists it").toEqual([createdAt]);
  });

  // Battery 18 through the objects (the orchestrator's ruling S-12): a removed
  // generation's late push is ignored BOTH before and after a re-created
  // table's first push. The late pushes are built by hand from the removed
  // generation and its last copy. Mutants: the retired set not consulted
  // (before: the removed table's row comes back); a push of another
  // generation applied over the current row when its revision is higher
  // (the third push, of a newer generation never retired).
  it("a removed generation's late push is ignored before and after the re-created table's first push (battery 18)", async () => {
    const id = "slot-k";
    const stub = tableStub(id);
    expect((await create(id)).status).toBe(201);
    await settle(stub);
    const old = (await metaOf(stub)).generation!;
    const lastCopy = (await copyRow(id))!.json;
    expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status).toBe(204);
    await settle(stub);
    expect(await copyRow(id), "premise: the removal was applied").toBeUndefined();
    expect(await retiredOf(id), "premise: its generation is retired").toEqual([old]);

    const early = await logsDuring(() => applyInHouse(pushOf(id, old, 50, lastCopy)));
    expect(early.lines.map((l) => (l as { event: string }).event), "before the re-create").toEqual(["slot-ignored"]);
    expect(await copyRow(id)).toBeUndefined();
    expect(await listed(RED, "scope=mine", [id])).toEqual([]);

    await endMarker(stub);
    expect((await create(id)).status).toBe(201);
    await settle(stub);
    const current = await copyRow(id);
    expect(cmpGen(current!.generation, old), "premise: the re-created table's copy is current").toBeGreaterThan(0);
    const late = await logsDuring(() => applyInHouse(pushOf(id, old, 51, lastCopy)));
    expect(late.lines.map((l) => (l as { event: string }).event), "after the re-create").toEqual(["slot-ignored"]);
    expect(await copyRow(id), "the row is still the re-created table's").toEqual(current);
    // A current row is replaced only through its own generation, even by one
    // that is newer and was never retired: it waits for the current one's
    // removal (spec §2.5).
    const newer: Generation = { at: current!.generation.at + 1, rand: "0000000000000000" };
    const other = await logsDuring(() => applyInHouse(pushOf(id, newer, 99, lastCopy)));
    expect(other.lines.map((l) => (l as { event: string }).event), "another generation while one is current").toEqual(["slot-ignored"]);
    expect(await copyRow(id)).toEqual(current);
  });

  // Spec §2.4 step 2: a delete's 204 writes ONE batch — the docs rows and the
  // picker row removed, the marker written, the slot a REMOVAL at
  // `revision + 1`, with its alarm. The removal's push is held so no
  // acknowledgement's batch runs inside the watch. Its acknowledgement then
  // re-arms the alarm to the marker's end (step 6). Mutants: the removal, or
  // the marker, in a batch of its own; the picker row left; no alarm for the
  // removal; the acknowledgement's alarm deleted, or left at the removal's
  // retry.
  it("a delete removes every docs row, the picker's too, and writes its marker and its REMOVAL in the same one batch", async () => {
    const id = "slot-n";
    const stub = await seedPlaying(id);
    expect((await call(stub, RED, "POST", `/api/games/${id}/input`, '{"line":"1"}')).status).toBe(200);
    const kinds = async () => (await sqlOf(stub, "SELECT kind FROM docs ORDER BY kind")).map((r) => r.kind);
    expect(await kinds(), "premise: an open picker is stored").toEqual(["picker", "record", "table"]);
    const n = (await metaOf(stub)).revision;
    const held = holdApply([[id, n + 1]]);
    try {
      const batches = await watchBatches(stub);
      const deletedAt = Date.now();
      expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status).toBe(204);
      expect(batches.statements, "anti-vacuity: the watch saw the object's SQL").toBeGreaterThan(0);
      expect({ batches: batches.batches, outside: batches.outside }).toEqual({ batches: 1, outside: [] });
      expect(await kinds()).toEqual([]);
      const slot = (await metaOf(stub)).slot;
      expect([slot?.revision, slot?.copy, slot?.attempt]).toEqual([n + 1, null, 0]);
      expect(await markerOf(stub), "the marker, in that batch").toMatchObject({ reason: "abandoned", kept: "archive" });
      expect(await alarmOf(stub)).toBeGreaterThanOrEqual(deletedAt + 30_000);
      held.release(id, n + 1);
      await settle(stub);
    } finally {
      held.restore();
      await settle(stub);
    }
    const marker = (await markerOf(stub))!;
    expect((await metaOf(stub)).slot, "premise: the removal acknowledged").toBeUndefined();
    expect(await alarmOf(stub), "the acknowledgement re-armed the alarm to the marker's end").toBe((marker.at as number) + 90 * DAY_MS);
  });

  // Task 5's escalation 1, RULED: a copy `house_copy` refuses is logged —
  // its wrapped sentence VERBATIM — and writes no slot and arms nothing, so
  // the house keeps listing the LAST copy it received (only a corrupted store
  // reaches it; the operator's force is the path). The refusal is made at
  // MODULE scope (pool fact 4), in every NEW module built while it is in —
  // the module a copy is computed on is always a new one. The next change is
  // delivered as usual. Mutants: the refusal thrown out of the request; a
  // slot written without a copy; a second prefix on the sentence.
  it("a copy house_copy refuses is logged verbatim, writes no slot, and the house keeps the last copy", async () => {
    const id = "slot-l";
    const stub = tableStub(id);
    expect((await create(id)).status).toBe(201);
    await settle(stub);
    const before = await metaOf(stub);
    const armed = await alarmOf(stub);
    expect(armed, "premise: the create's acknowledgement armed the table's expiry").toBe(await expiryOf(stub));
    const sentence = `"${id}"'s table document is malformed, so the house would not list it: a keep of 3`;
    const original = Instance.prototype.get;
    Instance.prototype.get = function (this: Instance) {
      const got = original.call(this);
      if (got.fresh) {
        got.mod.house_copy = () => {
          throw sentence;
        };
      }
      return got;
    };
    try {
      const { result, lines } = await logsDuring(() => call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}'));
      expect(result.status, "the reply stands").toBe(200);
      expect(lines).toEqual([{ event: "copy-refused", id, error: sentence }]);
    } finally {
      Instance.prototype.get = original;
    }
    const after = await metaOf(stub);
    expect([after.slot, after.revision], "no slot").toEqual([undefined, before.revision]);
    expect(await alarmOf(stub), "and no retry armed — a refusal would repeat: the alarm is where it was").toBe(armed);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status), "the last copy the house received").toEqual(["open"]);
    expect((await call(stub, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}')).status).toBe(200);
    await settle(stub);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status), "the next change is delivered").toEqual(["dealing"]);
  });

  // The orchestrator's ruling I-42: a TRAP in `house_copy` costs the request
  // nothing — its documents commit, the reply stands, the slot is left as it
  // was — and the alarm, armed at +30 s, recomputes the copy on a FRESH
  // module and delivers it. The copy is computed on a module of its own, so
  // the request's module is untouched and its frames still go out: a socket
  // open on the table hears the join. The trap is made ONCE, at module scope
  // (pool fact 4), in the next NEW module built. Mutants: the trap thrown out
  // of the request (a 500); the
  // documents not committed; no alarm armed; an alarm that does not
  // recompute; the frames read from an emptied module (the socket is ended).
  it("a copy that traps in the request is recomputed and delivered by the alarm, and the frames still go out", async () => {
    const id = "slot-m";
    const stub = tableStub(id);
    expect((await create(id)).status).toBe(201);
    await settle(stub);
    const before = await metaOf(stub);
    const upgrade = await stub.fetch(`https://yokohama.test/api/games/${id}/stream`, {
      headers: { Upgrade: "websocket", "X-Yokohama-User": RED },
    });
    const ws = upgrade.webSocket!;
    ws.accept();
    const messages: string[] = [];
    ws.addEventListener("message", (e) => {
      messages.push(String(e.data));
    });
    await until(async () => messages.length >= 1, "the socket's first frame");
    const original = Instance.prototype.get;
    Instance.prototype.get = function (this: Instance) {
      const got = original.call(this);
      if (got.fresh) {
        got.mod.house_copy = () => {
          Instance.prototype.get = original;
          throw new WebAssembly.RuntimeError("unreachable");
        };
      }
      return got;
    };
    const joinedAt = Date.now();
    let joinedBy = joinedAt;
    try {
      const { result, lines } = await logsDuring(() => call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}'));
      joinedBy = Date.now();
      expect(result.status, "the reply stands").toBe(200);
      expect(lines).toEqual([{ event: "trap", id, call: "house_copy", error: "RuntimeError: unreachable" }]);
    } finally {
      Instance.prototype.get = original;
    }
    expect(await tableText(stub), "the documents committed").toMatch(/"status":\s*"dealing"/);
    const after = await metaOf(stub);
    expect([after.slot, after.revision], "the slot is left as it was").toEqual([undefined, before.revision]);
    // A window, not a floor: the table's expiry, 90 days out, satisfies any
    // floor, and it is where the alarm would stand had the trap armed nothing.
    const recompute = await alarmOf(stub);
    expect(recompute, "the alarm waits to recompute it").toBeGreaterThanOrEqual(joinedAt + 30_000);
    expect(recompute, "at the first rung").toBeLessThanOrEqual(joinedBy + 30_000);
    await until(async () => messages.length >= 2, "the join's frame");
    expect(JSON.parse(messages[1]!).event, "the socket heard the join, and is not ended").toBe("signal");
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status), "premise: the house still lists the old copy").toEqual(["open"]);
    expect(await runDurableObjectAlarm(stub)).toBe(true);
    await settle(stub);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status)).toEqual(["dealing"]);
    const delivered = await metaOf(stub);
    expect([delivered.slot, delivered.acked?.revision]).toEqual([undefined, before.revision + 1]);
    expect(await alarmOf(stub), "the acknowledgement re-armed the alarm to the table's expiry").toBe(await expiryOf(stub));
    ws.close(1000, "done");
  });

  // Spec §2.4 ("Traps"): a side channel's trap retries "on the alarm or the
  // next request". A `house_copy` trap is counted in `meta.copy_owed` (the
  // orchestrator's ruling on Task 12's review): the alarm retries it on the
  // backoff — 30 s, 2 min, 10 min, then hourly, the ceiling a failing push
  // has — and every request computes the copy while it is owed, whether or
  // not it changes the table document. Three traps in a row — the join's,
  // then two alarms' — climb the ladder, and a plain GET, which changes
  // nothing, delivers the copy. It runs with no TTL (`noTtl`): under one, an
  // acknowledgement leaves the table's expiry armed, which a floor on the
  // alarm cannot tell from a rung, and every alarm run ends by re-arming to
  // everything still due, the copy's rung among them. Mutants: a trap in the
  // alarm's retry that arms nothing; the ladder not climbed; a request that
  // computes the copy only when the document changes (the table stays listed
  // as open for good).
  it("a copy that keeps trapping is retried on the backoff, and the next request delivers it", async () => {
    const id = "slot-r";
    const stub = tableStub(id);
    const ttl = noTtl();
    try {
      expect((await create(id)).status).toBe(201);
      await settle(stub);
      const trapLine = { event: "trap", id, call: "house_copy", error: "RuntimeError: unreachable" };
      const traps = trapCopies(3);
      try {
        const t0 = Date.now();
        const joined = await logsDuring(() => call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}'));
        expect([joined.result.status, joined.lines]).toEqual([200, [trapLine]]);
        expect(await copyOwed(stub)).toEqual(["1"]);
        expect(await alarmOf(stub), "the first rung, 30 s").toBeGreaterThanOrEqual(t0 + 30_000);
        for (const [owed, rung] of [
          ["2", 120_000],
          ["3", 600_000],
        ] as const) {
          const t = Date.now();
          const ran = await logsDuring(() => runDurableObjectAlarm(stub));
          expect([ran.result, ran.lines], `the alarm's retry traps again (${owed})`).toEqual([true, [trapLine]]);
          expect(await copyOwed(stub)).toEqual([owed]);
          expect(await alarmOf(stub), `the rung after ${owed} traps`).toBeGreaterThanOrEqual(t + rung);
        }
      } finally {
        traps.restore();
      }
      expect((await listed(RED, "scope=mine", [id])).map((r) => r.status), "premise: the house still lists the create's copy").toEqual([
        "open",
      ]);
      const table = await tableText(stub);
      expect((await call(stub, RED, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
      expect(await tableText(stub), "premise: the GET changed no document").toBe(table);
      expect((await listed(RED, "scope=mine", [id])).map((r) => r.status), "the GET delivered the owed copy").toEqual(["dealing"]);
      expect(await copyOwed(stub), "and nothing is owed").toEqual([]);
      expect(await alarmOf(stub), "with no TTL, the acknowledgement deleted the alarm").toBeNull();
    } finally {
      ttl.restore();
    }
  });

  // A throw that is not a trap while the copy's own module is built — a
  // `RangeError` when its memory cannot be allocated — is a side channel's
  // failure like a trap (the orchestrator's ruling on Task 12's review): the
  // request commits and answers, the failure is logged with its class, and
  // the copy is owed. The throw is made at MODULE scope (pool fact 4), once,
  // in the next NEW module built. Mutant: the throw let out of `copyOf` —
  // it leaves `commit` before the batch, and the request fails.
  it("a throw building the copy's module costs the request nothing", async () => {
    const id = "slot-s";
    const stub = tableStub(id);
    expect((await create(id)).status).toBe(201);
    await settle(stub);
    const original = Instance.prototype.get;
    Instance.prototype.get = function (this: Instance) {
      const got = original.call(this);
      if (got.fresh) {
        Instance.prototype.get = original;
        this.drop();
        throw new RangeError("could not allocate the module's memory");
      }
      return got;
    };
    try {
      const { result, lines } = await logsDuring(() => call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}'));
      expect(result.status, "the reply stands").toBe(200);
      expect(lines).toEqual([{ event: "trap", id, call: "house_copy", error: "RangeError: could not allocate the module's memory" }]);
    } finally {
      Instance.prototype.get = original;
    }
    expect(await tableText(stub), "the join committed").toMatch(/"status":\s*"dealing"/);
    expect(await copyOwed(stub), "the copy is owed").toEqual(["1"]);
    expect(await runDurableObjectAlarm(stub)).toBe(true);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status)).toEqual(["dealing"]);
  });

  // With a copy owed, an acknowledgement that clears the slot re-arms the
  // alarm to the copy's retry instead of deleting it: the slot was all its
  // batch discharged. The create's push is held, the join's copy traps, and
  // the create's acknowledgement lands after. Mutant: the clearing
  // acknowledgement deletes the alarm whatever is owed — the copy then waits
  // for a request that may never come.
  it("an acknowledgement that clears the slot keeps the alarm while a copy is still owed", async () => {
    const id = "slot-t";
    const stub = tableStub(id);
    const r = (await metaOf(stub)).revision + 1;
    const held = holdApply([[id, r]]);
    let traps: { restore(): void } | undefined;
    let released = 0;
    let settled = 0;
    try {
      expect((await create(id)).status).toBe(201);
      await until(async () => held.reached(id, r), "the create's push reaches its gate");
      traps = trapCopies(1);
      const { result, lines } = await logsDuring(() => call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}'));
      expect([result.status, lines.map((l) => (l as { event: string }).event)], "premise: the join's copy trapped").toEqual([200, ["trap"]]);
      expect((await metaOf(stub)).slot?.revision, "premise: the slot still holds the create's copy").toBe(r);
      released = Date.now();
      held.release(id, r);
      await settle(stub);
      settled = Date.now();
      const m = await metaOf(stub);
      expect([m.slot, m.acked?.revision], "premise: the create's acknowledgement cleared the slot").toEqual([undefined, r]);
    } finally {
      traps?.restore();
      held.restore();
      await settle(stub);
    }
    expect(await copyOwed(stub), "premise: the copy is owed").toEqual(["1"]);
    // A window, not "not null": with a TTL the acknowledgement would re-arm
    // to the table's expiry had it forgotten the copy.
    const alarm = await alarmOf(stub);
    expect(alarm, "the owed copy keeps its alarm, at its first rung").toBeGreaterThanOrEqual(released + 30_000);
    expect(alarm).toBeLessThanOrEqual(settled + 30_000);
    expect(await runDurableObjectAlarm(stub)).toBe(true);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status)).toEqual(["dealing"]);
  });

  // One convention for the retry rung (the orchestrator's ruling on Task 12's
  // review): `nextRetry(n)` is the retry after the (n + 1)-th failure, and a
  // waiting slot's retry stays where its own batch or failure put it. The
  // create's push is held; the join's push fails (its retry at the first
  // rung, 30 s); then the create's acknowledgement lands while the join's
  // slot waits. Mutant: the older acknowledgement re-arms the waiting slot
  // with its stored failure count — one rung later, 2 min.
  it("an older acknowledgement leaves a waiting slot's retry where its failure put it", async () => {
    const id = "slot-u";
    const stub = tableStub(id);
    const r = (await metaOf(stub)).revision + 1;
    const held = holdApply([[id, r]]);
    let failing: { restore(): Promise<void> } | undefined;
    try {
      expect((await create(id)).status).toBe(201);
      await until(async () => held.reached(id, r), "the create's push reaches its gate");
      failing = await failPushes(stub, 1);
      expect((await call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}')).status).toBe(200);
      await until(async () => (await metaOf(stub)).slot?.attempt === 1, "the join's push failed, and its failure is counted");
      const retry = await alarmOf(stub);
      held.release(id, r);
      await until(async () => (await metaOf(stub)).acked?.revision === r, "the create's acknowledgement is read");
      const m = await metaOf(stub);
      expect([m.slot?.revision, m.slot?.attempt], "premise: the join's slot still waits").toEqual([r + 1, 1]);
      expect(await alarmOf(stub), "its retry is where its failure put it").toBe(retry);
    } finally {
      await failing?.restore();
      held.restore();
      await settle(stub);
    }
    expect(await runDurableObjectAlarm(stub)).toBe(true);
    expect((await listed(RED, "scope=mine", [id])).map((r) => r.status)).toEqual(["dealing"]);
  });

  // Review Focus 4, through the objects: the characters of a table name and a
  // display name — an accent, an emoji, CJK — survive the create, the scrub,
  // the slot, the house's store and its listing, byte for byte, and the rows
  // are the layer's own over the original. The house is evicted first, so the
  // listing is read from the copies it STORED. Mutants: a copy re-encoded on
  // the way (an escape, a normalization, lost bytes); the display name
  // dropped by the scrub.
  it("a non-ASCII table name and display name list exactly as created", async () => {
    const id = "slot-i";
    const body = CREATE_BODY.replace(/}$/, ',"name":"Café ☕ 東京","display_name":"Zoë"}');
    expect((await create(id, body)).status).toBe(201);
    await runInDurableObject(tableStub(id), (table: Table) => table.deliverSlot());
    await evictDurableObject(houseStub());
    const nas = referenceModule();
    expect(readReply(nas.create(RED, new TextEncoder().encode(body), id, "application/json")).status).toBe(201);
    for (const [user, query] of [
      [RED, "scope=mine"],
      [STRANGER, "scope=open"],
    ] as const) {
      const res = await toHouse(user, "GET", `/api/games?${query}`);
      expect(res.text, `${user}: the characters, unescaped`).toContain('"name":"Café ☕ 東京"');
      const rows = (JSON.parse(res.text) as { games: Listed[] }).games.filter((g) => g.id === id);
      expect(rows.map((r) => [r.name, r.seats[0]?.name])).toEqual([["Café ☕ 東京", "Zoë"]]);
      const original = readReply(nas.dispatch(user, "GET", "/api/games", query, undefined, new Uint8Array()));
      expect(rows, `${user}: the layer's row over the original`).toEqual(
        (JSON.parse(original.body) as { games: Listed[] }).games.filter((g) => g.id === id),
      );
    }
  });

  // A table with a bot seat: since M6.5c2 its copy carries the bot's
  // descriptor (R-6.5c2.10), whose `weights` is above 2^53, where a parse
  // rounds it. The shell parses a copy only to read it (the operator's
  // listing; `copyFacts`), so the copy the house STORES must be the text
  // `house_copy` answered, byte for byte, those digits intact. Mutant: the
  // copy re-encoded through a parse on the way, in the table's push or the
  // house's store.
  it("a copy whose bot weights are above 2^53 is stored as house_copy's text, byte for byte", async () => {
    const id = "slot-weights";
    const stub = tableStub(id);
    expect((await create(id, CREATE_BODY.replace('{"open":true}', '{"bot":true}'))).status).toBe(201);
    expect((await call(stub, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}')).status, "premise: red keeps, the table's one human seat").toBe(200);
    await settle(stub);
    const want = copyOf(id, await tableText(stub));
    const digits = /"weights":\s*(\d+)/.exec(want)?.[1];
    expect(digits, "premise: house_copy's text carries the bot's weights").toBe("6957643967275421640");
    expect(String(Number(digits)), "premise: a parse rounds those digits").not.toBe(digits);
    expect((await copyRow(id))?.json, "the house stores house_copy's text over the stored table, byte for byte").toBe(want);
  });

  // Spec §2.5: the lobby socket is silent on connect, as the SSE lobby stream
  // is, and hears every copy the house applies — ONE frame per applied copy,
  // `{"event":"house","data":<house_frame(seq, chat, rev)>}`. A house chat post is
  // the positive control (ruling R-10): its frame arrives right after, so an
  // extra frame would have arrived between. Mutants: a frame on connect; no
  // frame for an applied copy; a frame for the push AND for the flush's
  // repeat; `seq` not raised.
  it("the lobby socket is silent on connect and hears every applied copy", async () => {
    const id = "slot-j";
    const tab = await lobbySocket(RED);
    expect((await toHouse(BLUE, "GET", "/api/health")).status).toBe(200);
    expect(tab.messages, "nothing arrived on connect").toEqual([]);
    expect((await create(id)).status).toBe(201);
    await runInDurableObject(tableStub(id), (table: Table) => table.deliverSlot());
    const [first] = await tab.until(1);
    const { event, data } = JSON.parse(first!) as { event: string; data: { seq: number; chat: number; rev: number } };
    expect(event).toBe("house");
    expect(data.seq, "an applied copy raised seq").toBeGreaterThan(0);
    expect(first).toBe(`{"event":"house","data":${referenceModule().house_frame(data.seq, data.chat, data.rev)}}`);
    const posted = await toHouse(BLUE, "POST", "/api/chat", '{"text":"a table is up"}');
    expect(posted.status).toBe(201);
    const chat = (JSON.parse(posted.text) as { seq: number }).seq;
    expect(await tab.until(2)).toEqual([
      first,
      `{"event":"house","data":${referenceModule().house_frame(data.seq, chat, data.rev + 1)}}`,
    ]);
    tab.ws.close(1000, "done");
  });

  // M6.5c2 cycle D (R-D4): a house message's DELETE is framed like a post,
  // ONE frame per lobby socket — with `rev` moved and `chat` NOT, since a
  // delete adds no message — and read off the STORED document, so the
  // tombstone is committed before the frame goes. The author alone: the house
  // has no creator, so another caller's delete is the 403, framed nowhere —
  // the author's own delete is the positive control whose frame arrives next
  // (ruling R-10). Mutants: the rev bump not framed (`bump_house_chat_rev`
  // left out of the house's check); `rev` read as `last`; `chat` moved by the
  // delete.
  it("a house message's delete frames every lobby with the revision moved and the count not", async () => {
    const tab = await lobbySocket(RED);
    const posted = await toHouse(BLUE, "POST", "/api/chat", '{"text":"soon gone"}');
    expect(posted.status, posted.text).toBe(201);
    const seq = (JSON.parse(posted.text) as { seq: number }).seq;
    const [first] = await tab.until(1);
    const { data } = JSON.parse(first!) as { data: { seq: number; chat: number; rev: number } };
    expect(data.chat, "premise: the post's own frame").toBe(seq);
    const refused = await toHouse(RED, "DELETE", `/api/chat/${seq}`);
    expect([refused.status, refused.text]).toEqual([403, '{"error":"only the message\'s author or the table\'s creator may delete it"}']);
    const deleted = await toHouse(BLUE, "DELETE", `/api/chat/${seq}`);
    expect([deleted.status, deleted.text]).toEqual([204, ""]);
    expect(await tab.until(2)).toEqual([
      first,
      `{"event":"house","data":${referenceModule().house_frame(data.seq, data.chat, data.rev + 1)}}`,
    ]);
    const page = JSON.parse((await toHouse(RED, "GET", `/api/chat?after=${seq - 1}`)).text) as {
      messages: { seq: number; text: string; deleted: boolean }[];
    };
    expect(page.messages.map((m) => [m.seq, m.text, m.deleted])).toEqual([[seq, "", true]]);
    tab.ws.close(1000, "done");
  });
});

describe("the house's own store (spec §2.5)", () => {
  // Task 11's review, ruled: the house has no poison count — at the SECOND
  // CONSECUTIVE load trap of its stored chat document it deletes that row,
  // logging `house-chat-dropped`, and serves without it, so the lobby never
  // stays 500. A load that succeeds in between resets the count. The trap is
  // made at MODULE scope (pool fact 4) in every module built while it is in.
  // Mutants: no count (every trap a 500, forever); the row dropped at the
  // FIRST trap; the count never reset; the copies not loaded after the drop.
  it("two consecutive traps loading the house chat drop it, logged, and the lobby answers", async () => {
    const id = "slot-o";
    expect((await create(id)).status).toBe(201);
    await settle(tableStub(id));
    expect((await toHouse(RED, "POST", "/api/chat", '{"text":"doomed"}')).status).toBe(201);
    const original = Instance.prototype.get;
    const trapChat = () => {
      Instance.prototype.get = function (this: Instance) {
        const got = original.call(this);
        if (got.fresh) {
          const load = got.mod.load;
          got.mod.load = (kind, key, text) => {
            if (kind === "house") throw new WebAssembly.RuntimeError("unreachable");
            load(kind, key, text);
          };
        }
        return got;
      };
    };
    const trapLine = (traps: number) => ({ event: "load-trap", id: "house", kind: "house", traps, error: "RuntimeError: unreachable" });
    const count = () => runInDurableObject(houseStub(), (_h, state) => state.storage.sql.exec("SELECT v FROM meta WHERE k = 'load_traps'").toArray());
    try {
      trapChat();
      await evictDurableObject(houseStub());
      const first = await logsDuring(() => toHouse(RED, "GET", "/api/chat"));
      expect(first.result.status).toBe(500);
      expect(first.lines).toEqual([trapLine(1)]);
      // A good load between two traps: not consecutive.
      Instance.prototype.get = original;
      await evictDurableObject(houseStub());
      expect((await toHouse(RED, "GET", "/api/chat")).status).toBe(200);
      expect(await count(), "a good load resets the count").toEqual([]);
      trapChat();
      await evictDurableObject(houseStub());
      const again = await logsDuring(() => toHouse(RED, "GET", "/api/chat"));
      expect([again.result.status, again.lines]).toEqual([500, [trapLine(1)]]);
      await evictDurableObject(houseStub());
      const dropped = await logsDuring(() => toHouse(RED, "GET", "/api/chat"));
      expect(dropped.lines).toEqual([{ event: "house-chat-dropped", id: "house", traps: 2, error: "RuntimeError: unreachable" }]);
      expect([dropped.result.status, (JSON.parse(dropped.result.text) as { messages: unknown[] }).messages]).toEqual([200, []]);
      expect((await listed(RED, "scope=mine", [id])).map((r) => r.id), "the lobby still lists the copies").toEqual([id]);
    } finally {
      Instance.prototype.get = original;
    }
    expect(
      await runInDurableObject(houseStub(), (_h, state) => state.storage.sql.exec("SELECT kind FROM docs").toArray()),
      "the row is gone",
    ).toEqual([]);
    expect(await count()).toEqual([]);
  });
});
