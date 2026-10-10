import { env } from "cloudflare:workers";
import { evictDurableObject, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { expect, vi } from "vitest";
import * as configModule from "../src/config";
import * as markerModule from "../src/marker";
import type { Env } from "../src/env";
import type { House } from "../src/house";
import { ensureTables, upsertDoc } from "../src/storage";
import type { Table } from "../src/table";
import { OWED_GAME_OVER_JSON, OWED_GAME_OVER_TABLE_JSON } from "./fixtures.gen";

export const RED = "red@t.test";
export const BLUE = "blue@t.test";

/** Task 2's create body without its `id`: the Worker names the object, and
 *  the module inserts that name into the body (spec §2.2). */
export const CREATE_BODY =
  '{"players":2,"colors":["Red","Blue"],"start_seat":0,"seed":20260831,"seats":[{"user":"red@t.test"},{"open":true}]}';

/** The table object named `name`, reached as the Worker reaches it: by name. */
export function tableStub(name: string): DurableObjectStub<Table> {
  return env.TABLE.get(env.TABLE.idFromName(name));
}

/**
 * One request to a table object, as the Worker forwards it (the plan's
 * internal headers): the caller in `X-Yokohama-User`, a body as JSON, and for
 * a create the object's name, percent-encoded, in `X-Yokohama-Table-Id`.
 *
 * The object's reply is read here, whole, and handed back as a new `Response`
 * with the same status and headers, so no cell leaves an object's stream
 * unread and every cell can read the body when it likes.
 */
export async function call(
  stub: DurableObjectStub<Table>,
  user: string,
  method: string,
  path: string,
  body?: string,
  tableId?: string,
): Promise<Response> {
  const headers = new Headers({ "X-Yokohama-User": user });
  if (body !== undefined) headers.set("content-type", "application/json");
  if (tableId !== undefined) headers.set("X-Yokohama-Table-Id", encodeURIComponent(tableId));
  const res = await stub.fetch(`https://yokohama.test${path}`, { method, headers, body });
  const text = await res.text();
  return new Response(text === "" ? null : text, { status: res.status, headers: res.headers });
}

/** The rows `query` answers inside the object: a `runInDurableObject` read. */
export function sqlOf(stub: DurableObjectStub<Table>, query: string): Promise<Record<string, SqlStorageValue>[]> {
  return runInDurableObject(stub, (_o, state) => state.storage.sql.exec(query).toArray());
}

/** Every `docs` and `meta` row, whole — what "touched nothing" compares. */
export async function storedRows(
  stub: DurableObjectStub<Table>,
): Promise<{ docs: Record<string, SqlStorageValue>[]; meta: Record<string, SqlStorageValue>[] }> {
  return {
    docs: await sqlOf(stub, "SELECT kind, json, written_at FROM docs ORDER BY kind"),
    meta: await sqlOf(stub, "SELECT k, v FROM meta ORDER BY k"),
  };
}

/** The fields of a play payload these cells read. It carries no seed, so a
 *  cell may parse it; a record or a table DOCUMENT is never parsed. */
export type Play = { events: number; prompt: { kind: string; selected?: number[] } };

/** The table's `meta.revision`: 0 before its first slot, and before its first
 *  batch creates `meta` (the orchestrator's ruling S-12 reads every revision a
 *  hold is keyed by through this, just before the action). */
export async function revisionOf(stub: DurableObjectStub<Table>): Promise<number> {
  const rows = await runInDurableObject(stub, (_o, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'meta'").one().n === 1;
    return exists ? sql.exec<{ v: string }>("SELECT v FROM meta WHERE k = 'revision'").toArray() : [];
  });
  return Number(rows[0]?.v ?? "0");
}

/**
 * Every side channel `stub`'s table has in flight, run through: each house
 * push's acknowledgement — or its failure — read and its batch written (spec
 * §2.5), and each archive upload's outcome and what it owes written (spec
 * §2.6). Both leave after their request's reply, so without this a cell's next
 * assertion on the table's rows, or on the lines the shell logs, would race
 * one an earlier request started. It awaits the table's own records of what
 * is in flight, which `deliverSlot` and `deliverArchive` keep — until neither
 * holds anything, because an upload that lands can start a push (the stamped
 * table's changed copy).
 */
export async function settle(stub: DurableObjectStub<Table>): Promise<void> {
  await runInDurableObject(stub, async (table: Table) => {
    for (;;) {
      const pending = [...table["uploads"].values(), ...table["pushes"].values()];
      if (pending.length === 0) return;
      await Promise.all(pending);
    }
  });
}

/**
 * A 2P table at `name`, PLAYING: red creates it with blue's seat open, blue
 * joins, both seats keep (the last keep finalizes), and red's `1` APPLIES the
 * opening move — Task 4's measured sequence at seed 20260831 (the Rust
 * `at_place_a`). Red is still to move, and red's next `1` opens the PlaceA
 * picker with nothing selected. The house pushes the create, the join and the
 * two keeps started are run through (`settle`) before it answers.
 */
export async function seedPlaying(name: string): Promise<DurableObjectStub<Table>> {
  const stub = tableStub(name);
  const steps: Array<[string, string, string, string, number]> = [
    [RED, "POST", "/api/games", CREATE_BODY, 201],
    [BLUE, "POST", `/api/games/${name}/join`, '{"seat":1}', 200],
    [RED, "POST", `/api/games/${name}/keep`, '{"seat":0,"card":1}', 200],
    [BLUE, "POST", `/api/games/${name}/keep`, '{"seat":1,"card":1}', 200],
  ];
  for (const [user, method, path, body, status] of steps) {
    const res = await call(stub, user, method, path, body, path === "/api/games" ? name : undefined);
    expect(res.status, `${user} ${method} ${path}: ${await res.clone().text()}`).toBe(status);
  }
  const moved = await call(stub, RED, "POST", `/api/games/${name}/input`, '{"line":"1"}');
  const state = await moved.json<Play>();
  expect([moved.status, state.events, state.prompt.kind], "premise: red's `1` applies the opening move").toEqual([
    200,
    1,
    "options",
  ]);
  await settle(stub);
  return stub;
}

/**
 * Spec §2.4's "every write the request makes goes in ONE synchronous batch",
 * made observable: from now until the object's next eviction, counts every
 * `transactionSync` the object runs, every statement it runs, and each
 * statement that is not a `SELECT` and runs OUTSIDE a transaction.
 *
 * It wraps the LIVE object's `storage.transactionSync` and `storage.sql.exec`
 * — own properties on two objects that are not RPC receivers, so the pool's
 * resolver does not refuse them (pool fact 1) — and an eviction discards them
 * with the instance (pool fact 4). `statements` is the anti-vacuity partner:
 * zero would mean the wrap never saw the object's SQL.
 */
export async function watchBatches(
  stub: DurableObjectStub<Table>,
): Promise<{ batches: number; statements: number; outside: string[] }> {
  const seen = { batches: 0, statements: 0, outside: [] as string[] };
  await runInDurableObject(stub, (_o, state) => {
    const storage = state.storage;
    const transactionSync = storage.transactionSync.bind(storage);
    const exec = storage.sql.exec.bind(storage.sql);
    let open = 0;
    storage.transactionSync = <T>(closure: () => T): T => {
      seen.batches++;
      open++;
      try {
        return transactionSync(closure);
      } finally {
        open--;
      }
    };
    storage.sql.exec = ((query: string, ...bindings: unknown[]) => {
      seen.statements++;
      if (open === 0 && !/^\s*SELECT\b/i.test(query)) seen.outside.push(query);
      return exec(query, ...bindings);
    }) as typeof storage.sql.exec;
  });
  return seen;
}

/**
 * Every line the shell logs while `f` runs, parsed — the shell logs one JSON
 * object per line (spec §2.8) — and nothing printed. A line that is not JSON
 * fails the cell that met it.
 */
export async function logsDuring<T>(f: () => Promise<T>): Promise<{ result: T; lines: unknown[] }> {
  const spy = vi.spyOn(console, "log").mockImplementation(() => {});
  try {
    const result = await f();
    return { result, lines: spy.mock.calls.map(([line]) => JSON.parse(String(line))) };
  } finally {
    spy.mockRestore();
  }
}

/** The committed owed-game-over pair's creator, and its one human seat. */
export const LOCAL = "local@browser";
/** The committed owed-game-over pair's own id. */
export const OWED = "local-242a5429453c593c";

/**
 * The segment the layer files a browser's save under, `browser/<segment>/`
 * (`crates/yokohama-web/src/archive.rs`'s `caller_segment`; security S1c,
 * finding D26): the address made readable — `@` as `_at_`, anything outside
 * `[a-z0-9._-]` as `_`, cut at 48 — then `-` and the first twelve hex
 * characters of a SHA-256 over the address. Restated here rather than read
 * off a reply, because the reply is what the cells test; workerd's own
 * WebCrypto digests it, since this project's types declare no node module.
 * The digest is over the layer's KEY, so the address must already be one:
 * lowercase and trimmed, as every address these cells pass is.
 */
export async function segmentOf(address: string): Promise<string> {
  if (address !== address.trim().toLowerCase()) throw new Error(`segmentOf(${JSON.stringify(address)}): not a normalized key`);
  const readable = [...address]
    .map((c) => (c === "@" ? "_at_" : /[a-z0-9._-]/.test(c) ? c : "_"))
    .join("")
    .slice(0, 48);
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(address)));
  const hex = [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
  return `${readable}-${hex.slice(0, 12)}`;
}

/** A record no game here makes: what another game's copy, archived under
 *  this id, holds. */
export const ANOTHER_RECORD = '{"engine_version":"another","events":[]}';

/** `text` with `from` replaced by `to`, where `from` occurs exactly once — an
 *  edit of a document's TEXT, never a parse, so a seed above 2^53 keeps its
 *  digits. */
export function replaceOnce(text: string, from: string, to: string): string {
  const at = text.indexOf(from);
  if (at < 0 || text.includes(from, at + 1)) throw new Error(`premise: ${JSON.stringify(from)} occurs exactly once`);
  return text.slice(0, at) + to + text.slice(at + from.length);
}

/** A table document's `archived_at`, read off its text: `null` while it is
 *  unstamped. */
export function archivedAt(table: string | null | undefined): number | null {
  const m = /"archived_at": (null|\d+)/.exec(table ?? "");
  if (m === null) throw new Error(`premise: a table document naming archived_at — got ${table?.slice(0, 80)}`);
  return m[1] === "null" ? null : Number(m[1]);
}

/** A table object named `name` holding `docs` as their exact text, written in
 *  one batch before its first request — whose WAKE loads them. */
export async function holding(name: string, docs: { table: string; record: string }): Promise<DurableObjectStub<Table>> {
  const stub = tableStub(name);
  await runInDurableObject(stub, (_t, state) => {
    state.storage.transactionSync(() => {
      ensureTables(state.storage.sql);
      upsertDoc(state.storage.sql, "table", docs.table, Date.now());
      upsertDoc(state.storage.sql, "record", docs.record, Date.now());
    });
  });
  return stub;
}

/**
 * The committed owed-game-over pair (`crates/yokohama-worker`'s fixture): the
 * finished browser-played game cut back to the start of the bot's trailing
 * run, so the first request's WAKE pumps the bot to the end and reaches the
 * game-over site (plan review M4). Its creator is `local@browser`, not a
 * `.test` address, so its save is keyed at the bucket's top — the test
 * file's local bucket.
 */
export function owedGameOver(name: string = OWED): Promise<DurableObjectStub<Table>> {
  const table = name === OWED ? OWED_GAME_OVER_TABLE_JSON : replaceOnce(OWED_GAME_OVER_TABLE_JSON, `"id": "${OWED}"`, `"id": "${name}"`);
  return holding(name, { table, record: OWED_GAME_OVER_JSON });
}

/** A `docs` row's exact text — `undefined` when none is stored. */
export async function docOf(stub: DurableObjectStub<Table>, kind: "table" | "record" | "chat"): Promise<string | undefined> {
  const rows = await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'docs'").one().n === 1;
    return exists ? sql.exec<{ json: string }>("SELECT json FROM docs WHERE kind = ?", kind).toArray() : [];
  });
  return rows[0]?.json;
}

/** The object at `key` in the test file's bucket, as its text: `null` when
 *  absent. */
export async function bucketText(key: string): Promise<string | null> {
  const object = await env.ARCHIVE!.get(key);
  return object === null ? null : object.text();
}

/** The pending alarm, or `null`. */
export function alarmOf(stub: DurableObjectStub<Table>): Promise<number | null> {
  return runInDurableObject(stub, (_t, state) => state.storage.getAlarm());
}

/**
 * The switch set to `off` at MODULE scope (pool fact 4), in both of the
 * shell's readers — `shellConfig`, and the module's `init` configuration,
 * which reads its own copy — as if `YOKOHAMA_WEB_ARCHIVE` read `off`. An object
 * reads its configuration when it is built, so a cell rebuilds the objects it
 * asks (`rebuilt`).
 */
export function archiveOff(): { restore(): void } {
  const { shellConfig, moduleConfig } = configModule;
  const off = (e: Env): Env => ({ ...e, YOKOHAMA_WEB_ARCHIVE: "off" });
  const shell = vi.spyOn(configModule, "shellConfig").mockImplementation((e) => shellConfig(off(e)));
  const init = vi.spyOn(configModule, "moduleConfig").mockImplementation((e, caller) => moduleConfig(off(e), caller));
  return {
    restore() {
      shell.mockRestore();
      init.mockRestore();
    },
  };
}

/** `stub`'s object built afresh at its next event, so it reads the
 *  configuration in force: running, then evicted. */
export async function rebuilt(stub: DurableObjectStub<Table> | DurableObjectStub<House>): Promise<void> {
  await runInDurableObject(stub as unknown as DurableObjectStub, () => {});
  await evictDurableObject(stub);
}

/**
 * An operator's `POST /api/admin/tables/{id}/expire` — `?force` when `force`
 * — at `id`'s table object, as the Worker forwards one past its gate (spec
 * §2.9; the plan's internal headers): `X-Yokohama-Operator` names the
 * operator, and `X-Yokohama-User` the same address, as the Worker sets it for
 * any caller. Answered whole, as `call` answers.
 */
export async function operate(stub: DurableObjectStub<Table>, id: string, operator: string, force = false): Promise<Response> {
  const headers = new Headers({ "X-Yokohama-Operator": operator, "X-Yokohama-User": operator });
  const res = await stub.fetch(`https://yokohama.test/api/admin/tables/${id}/expire${force ? "?force" : ""}`, { method: "POST", headers });
  const text = await res.text();
  return new Response(text === "" ? null : text, { status: res.status, headers: res.headers });
}

/** Every `docs` row's `written_at` set to `at` — how a cell moves a table's
 *  last activity back in time, since workerd's clock cannot be moved (spec
 *  §5.2). */
export async function backdate(stub: DurableObjectStub<Table>, at: number): Promise<void> {
  await runInDurableObject(stub, (_t, state) => {
    state.storage.transactionSync(() => state.storage.sql.exec("UPDATE docs SET written_at = ?", at));
  });
}

/** `meta.marker` — the shell's own JSON — or `undefined` when none stands. */
export async function markerOf(stub: DurableObjectStub<Table>): Promise<Record<string, unknown> | undefined> {
  const rows = await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'meta'").one().n === 1;
    return exists ? sql.exec<{ v: string }>("SELECT v FROM meta WHERE k = 'marker'").toArray() : [];
  });
  return rows[0] === undefined ? undefined : (JSON.parse(rows[0].v) as Record<string, unknown>);
}

/** The number of tables `sqlite_master` lists: 0 once an object has ended,
 *  and for one that never wrote. */
export async function tablesIn(stub: DurableObjectStub<Table>): Promise<number> {
  const [row] = await sqlOf(stub, "SELECT count(*) AS n FROM sqlite_master");
  return Number(row?.n);
}

/** A day, written here rather than taken from the shell, so a wrong day in
 *  the shell fails the cells that read it. */
export const DAY_MS = 86_400_000;

/** Where a table's alarm stands when nothing but its expiry is due (spec
 *  §2.4; R-6.5c2.42): its last activity — the newest `written_at` of its
 *  record, table and chat — plus the SHORTER of `wrangler.test.jsonc`'s two
 *  TTLs, the waiting-table TTL's 7 days: a batch arms the expiry from SQL
 *  alone, which cannot read whether the table still waits for players, and
 *  the alarm decides. */
export async function expiryOf(stub: DurableObjectStub<Table>): Promise<number> {
  const [row] = await sqlOf(stub, "SELECT max(written_at) AS at FROM docs WHERE kind IN ('record', 'table', 'chat')");
  return Number(row?.at) + 7 * DAY_MS;
}

/** The standing marker moved back past its 90 days, and the alarm run: with
 *  its removal acknowledged, the object ends (spec §2.4 step 6). */
export async function endMarker(stub: DurableObjectStub<Table>): Promise<void> {
  const marker = await markerOf(stub);
  if (marker === undefined) throw new Error("premise: a marker stands");
  await runInDurableObject(stub, (_t, state) => {
    const moved = JSON.stringify({ ...marker, at: (marker.at as number) - 91 * DAY_MS });
    state.storage.transactionSync(() => state.storage.sql.exec("UPDATE meta SET v = ? WHERE k = 'marker'", moved));
  });
  await logsDuring(() => runDurableObjectAlarm(stub));
  expect(await tablesIn(stub), "the marker's 90 days over and its removal acknowledged: the object has ended").toBe(0);
}

/**
 * No TTL, as a deployment that keeps every table runs (R-6.5c.15): the ONE
 * TTL reader replaced at MODULE scope (the orchestrator's ruling I-34; pool
 * fact 4), so it survives an eviction. `wrangler.test.jsonc` sets production's
 * 90 days, under which an acknowledgement leaves the table's expiry deadline
 * armed and every alarm run ends by re-arming to everything still due (spec
 * §2.4) — which stands in for a retry's own re-arm. A cell about that re-arm
 * runs without a TTL, where nothing else arms the alarm. Restore it in a
 * `finally`.
 */
export function noTtl(): { restore(): void } {
  const spy = vi.spyOn(markerModule, "ttlDaysOf").mockReturnValue(null);
  return { restore: () => spy.mockRestore() };
}
