import { createExecutionContext, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import type { KeySource } from "../src/identity";
import { handle } from "../src/index";
import { route } from "../src/route";
import { HOUSE_NAME } from "../src/slot";
import type { Table } from "../src/table";
import { WALK_HOST_JSON, WALK_JSON } from "./corpus.gen";
import { tableStub } from "./helpers";

/**
 * **The corpus replay's driver** (spec §5.3's pool half; the plan's Task 12
 * Step 3): the walk the corpus was cut from, sent to the objects as the Worker
 * would send it — `test/corpus.test.ts` is its main client, and the battery-35
 * cell in `test/slot.test.ts` its other.
 *
 * - **The rows**: `walk.json`'s, and when asked `walk-host.json`'s, each run
 *   after the walk row its `after` names (`-1`: before the first — the art
 *   row), in the file's order.
 * - **Where each goes**: to the object `route` names — a table object by its
 *   name, a create's by its body's `id` (with `X-Yokohama-Table-Id` set to
 *   it, percent-encoded), the house by its name — and the art row through
 *   the Worker's own `handle`. `X-Yokohama-User` is the row's `who`, as the
 *   Worker sets it; a row whose `who` is `null` carries none.
 * - **The body**: `JSON.stringify` of the row's `body`, with a JSON content
 *   type, exactly as the node replay sends it; a `null` body is no body.
 * - **The flush**: before any row the HOUSE answers, `deliverSlot()` runs on
 *   every table a row has reached since the last such row — how the pool
 *   models "the push landed" before the lobby is read. Nothing else about the
 *   order changes.
 *
 * `onRow(index, row, reply)` is AWAITED after each row (the orchestrator's
 * ruling S-12): `index` is the row's place in its own script — `walk.json`'s,
 * or `walk-host.json`'s for a host row, which carries `after` — and `reply` is
 * the answer, read whole. Answering `"stop"` ends the walk after that row.
 *
 * The walk's own ids are fixed (`wire`, `wire2`, `wire4`), and storage is
 * per test FILE (pool fact 5), so a file that walks creates nothing else
 * under those names.
 */

/** One row of `walk.json` — `ScriptRow` in `wire_fixtures.rs`. */
export interface Row {
  who: string | null;
  method: string;
  path: string;
  query: string;
  body: unknown;
  status: number;
  fixture: string | null;
}

/** One row of `walk-host.json` — `HostRow` in `wire_fixtures.rs`: a `Row`
 *  run after the walk row `after`. */
export interface HostRow extends Row {
  after: number;
}

/** A row's answer: its status and its body's text. */
export interface Reply {
  status: number;
  text: string;
}

export const WALK = JSON.parse(WALK_JSON) as readonly Row[];
export const WALK_HOST = JSON.parse(WALK_HOST_JSON) as readonly HostRow[];

/** `handle`'s key source for the art row, which never reads it: `/api/art`
 *  is answered before the caller is read. */
const NO_KEYS: KeySource = async () => {
  throw new Error("the walk's art row asks for no key: /api/art is answered before the caller is read");
};

/** The rows in the order they run, each with its index in its own script. */
function order(host: boolean): Array<{ index: number; row: Row }> {
  const hosted = host ? WALK_HOST.map((row, index) => ({ index, row })) : [];
  const at = (after: number) => hosted.filter(({ row }) => (row as HostRow).after === after);
  const rows: Array<{ index: number; row: Row }> = [...at(-1)];
  WALK.forEach((row, index) => rows.push({ index, row }, ...at(index)));
  if (host && rows.length !== WALK.length + WALK_HOST.length) {
    throw new Error("a host row names no walk row to follow");
  }
  return rows;
}

/** One row, sent where the Worker would send it; answers its reply and the
 *  table object it reached, if any. */
async function send(row: Row): Promise<{ reply: Reply; table?: string }> {
  const url = `https://yokohama.test${row.path}${row.query ? `?${row.query}` : ""}`;
  const headers = new Headers();
  if (row.who !== null) headers.set("X-Yokohama-User", row.who);
  const body = row.body === null ? undefined : JSON.stringify(row.body);
  if (body !== undefined) headers.set("content-type", "application/json");
  const init = { method: row.method, headers, body };
  const target = route(row.method, row.path);
  let res: Response;
  let table: string | undefined;
  switch (target.kind) {
    case "art":
      res = await handle(new Request(url, init), env, createExecutionContext(), NO_KEYS);
      break;
    case "create": {
      const id = (row.body as { id?: unknown } | null)?.id;
      if (typeof id !== "string") throw new Error(`a walk create names no id: ${JSON.stringify(row.body)}`);
      headers.set("X-Yokohama-Table-Id", encodeURIComponent(id));
      table = id;
      res = await tableStub(id).fetch(url, init);
      break;
    }
    case "table":
      table = target.name;
      res = await tableStub(target.name).fetch(url, init);
      break;
    case "house":
      res = await env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME)).fetch(url, init);
      break;
    default:
      throw new Error(`the walk has no row for a ${target.kind} target: ${row.method} ${row.path}`);
  }
  return { reply: { status: res.status, text: await res.text() }, table };
}

/** Replays the walk; answers how many rows it sent. */
export async function walk(
  options: { host?: boolean; onRow?: (index: number, row: Row, reply: Reply) => unknown } = {},
): Promise<number> {
  const touched = new Set<string>();
  let sent = 0;
  for (const { index, row } of order(options.host ?? false)) {
    if (route(row.method, row.path).kind === "house") {
      for (const name of touched) await runInDurableObject(tableStub(name), (table: Table) => table.deliverSlot());
      touched.clear();
    }
    const { reply, table } = await send(row);
    if (table !== undefined) touched.add(table);
    sent++;
    if ((await options.onRow?.(index, row, reply)) === "stop") break;
  }
  return sent;
}
