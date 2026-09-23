import { env } from "cloudflare:workers";
import { runInDurableObject } from "cloudflare:test";
import { expect, vi } from "vitest";
import type { Table } from "../src/table";

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

/**
 * A 2P table at `name`, PLAYING: red creates it with blue's seat open, blue
 * joins, both seats keep (the last keep finalizes), and red's `1` APPLIES the
 * opening move — Task 4's measured sequence at seed 20260831 (the Rust
 * `at_place_a`). Red is still to move, and red's next `1` opens the PlaceA
 * picker with nothing selected.
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
