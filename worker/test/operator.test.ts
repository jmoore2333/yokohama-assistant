import { abortAllDurableObjects, createExecutionContext, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { beforeAll, describe, expect, it, vi } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule } from "../gen/factory.js";
import { Archive } from "../src/archive";
import * as configModule from "../src/config";
import type { Env } from "../src/env";
import { House } from "../src/house";
import type { KeySource } from "../src/identity";
import { handle } from "../src/index";
import { readReply } from "../src/module";
import { HOUSE_NAME } from "../src/slot";
import { Table } from "../src/table";
import { failingPuts, failPushes, holdApply, R2_UNREACHABLE, swapBucket } from "./gates";
import { OWED_GAME_OVER_JSON, OWED_GAME_OVER_TABLE_JSON } from "./fixtures.gen";
import {
  ANOTHER_RECORD,
  CREATE_BODY,
  DAY_MS,
  RED,
  LOCAL,
  OWED,
  alarmOf,
  archivedAt,
  backdate,
  call,
  docOf,
  holding,
  logsDuring,
  markerOf,
  owedGameOver,
  replaceOnce,
  revisionOf,
  seedPlaying,
  settle,
  sqlOf,
  tableStub,
  tablesIn,
} from "./helpers";
import { credentialClaims, personClaims, signer, type Signer } from "./tokens";

// **The operator surface** (R-6.5c.16; spec §2.9; the plan's Task 14). Every
// request goes through the Worker's own `handle`, so the gate is the one
// production runs. The pool's configuration sets NO operators — spec §2.9's
// off-path, which the absent-variable cell reads — so every other cell hands
// `handle` a copy of the pool's `env` with `YOKOHAMA_WEB_ADMINS` set to `.test`
// addresses (the plan's ruling P-1: the owner's address is in no committed
// file). The objects need no copy: an object trusts `X-Yokohama-Operator`,
// which only the Worker's gate sets. Storage and the house are per FILE
// (pool fact 5): every cell uses ids of its own.

const ORIGIN = "https://yokohama.test";

/** A listed operator; the list names it in capitals, as a person might
 *  write it, and the shell reads it trimmed and lowercased. */
const BOSS = "boss@t.test";
const OPS: Env = { ...env, YOKOHAMA_WEB_ADMINS: "chief@t.test, Boss@T.test" };

/** Spec §2.9's two host-written refusals, ruling I-72's, and §2.4's 409, as
 *  LITERALS. */
const NOT_AN_OPERATOR = '{"error":"this address is not an operator here"}';
const FORCE_VALUE = "?force takes no value — send ?force alone, or no query for an expire that saves first";
const UNCONFIRMED = { error: "the lobby has not confirmed the removal — the alarm will finish it" };
const USED_RECENTLY = { error: "this name was used recently — try another" };

/** The key the pool's Access "signs" with. */
let key: Signer;
beforeAll(async () => {
  key = await signer("k1");
});

function at() {
  return { teamDomain: env.ACCESS_TEAM_DOMAIN!, aud: env.ACCESS_AUD!, now: Math.floor(Date.now() / 1000) };
}
const person = (email: string) => key.sign(personClaims(email, at()));
const credential = () => key.sign(credentialClaims(env.DEV_DOOR_CLIENT_ID!, at()));
const keys: KeySource = async () => key.set();

interface Sent {
  method?: string;
  headers?: Record<string, string>;
  body?: string;
  token?: string;
  env?: Env;
}

/** One request through the Worker's `handle`, answered whole. */
async function worker(path: string, sent: Sent = {}): Promise<{ status: number; text: string }> {
  const headers = new Headers(sent.headers);
  if (sent.token !== undefined) headers.set("Cf-Access-Jwt-Assertion", sent.token);
  const req = new Request(ORIGIN + path, { method: sent.method ?? "GET", headers, body: sent.body ?? null });
  const res = await handle(req, sent.env ?? env, createExecutionContext(), keys);
  return { status: res.status, text: await res.text() };
}

/** An operator's expire of `id` through the Worker, as BOSS. */
async function expire(id: string, force = false): Promise<{ status: number; text: string }> {
  return worker(`/api/admin/tables/${id}/expire${force ? "?force" : ""}`, { method: "POST", token: await person(BOSS), env: OPS });
}

/** Which objects `f`'s requests reached: a spy on each class's `fetch`, on the
 *  PROTOTYPE (pool fact 1), calling through. */
async function reaching<T>(f: () => Promise<T>) {
  const spies = [Table, House, Archive].map((c) => vi.spyOn(c.prototype, "fetch"));
  try {
    const result = await f();
    return { result, reached: spies.reduce((n, s) => n + s.mock.calls.length, 0) };
  } finally {
    for (const s of spies) s.mockRestore();
  }
}

/** The layer's own answer to `method path`, from a fresh module in this
 *  configuration: what the house's `dispatch` answers for a path it has no
 *  route for. */
function layerAnswer(method: string, path: string, query = ""): { status: number; text: string } {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  const r = readReply(mod.dispatch(BOSS, method, path, query, undefined, new Uint8Array()));
  return { status: r.status, text: r.body };
}

/** The house, reached as the Worker reaches it. */
function houseStub(): DurableObjectStub<House> {
  return env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME));
}

/** The house's stored row for `id`: its generation's time, last activity and
 *  size — `undefined` when it lists none. */
async function copyOf(id: string): Promise<{ gen_at: number; last_active: number; bytes: number } | undefined> {
  return runInDurableObject(houseStub(), (_h, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'copies'").one().n === 1;
    if (!exists) return undefined;
    return sql
      .exec<{ gen_at: number; last_active: number; bytes: number }>("SELECT gen_at, last_active, bytes FROM copies WHERE id = ?", id)
      .toArray()[0];
  });
}

/** The shell's lines among `lines`, less the slot's push and acknowledgement,
 *  which `test/slot.test.ts` pins, and a counted wake's `wake` line, which
 *  `test/table.test.ts` pins (the orchestrator's ruling I-84; the pool counts
 *  wakes). */
function ends(lines: unknown[]): unknown[] {
  return lines.filter((l) => !["slot-pushed", "slot-acked", "wake"].includes(String((l as { event?: unknown }).event)));
}

/** The house's retired generations' times for `id`. */
async function retiredAt(id: string): Promise<number[]> {
  return runInDurableObject(houseStub(), (_h, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'retired'").one().n === 1;
    return exists ? sql.exec<{ gen_at: number }>("SELECT gen_at FROM retired WHERE id = ?", id).toArray().map((r) => r.gen_at) : [];
  });
}

/**
 * A create at `id`, as red, under the pool's FROZEN clock — the same second
 * as the table it replaces — its copy delivered before this answers, with the
 * generation it drew. A generation's time is the shell's `Date.now()` in
 * milliseconds (the orchestrator's F-2), so the new one orders after the one
 * the house retired by its time, and the house lists it, every time; the
 * P-2 reading in seconds tied the two, and the random half decided.
 */
async function recreate(id: string): Promise<{ status: number; generation: { at: number; rand: string } }> {
  const stub = tableStub(id);
  const res = await call(stub, RED, "POST", "/api/games", CREATE_BODY, id);
  await settle(stub);
  const [row] = await sqlOf(stub, "SELECT v FROM meta WHERE k = 'generation'");
  return { status: res.status, generation: JSON.parse(String(row?.v)) as { at: number; rand: string } };
}

describe("the gate (spec §2.9)", () => {
  // Battery 32. The credential claiming a LISTED operator's address is that
  // caller everywhere else — the premise — and never an operator: the gate
  // counts a PERSON only. A person not listed, and no one at all, are the same
  // 403, and no refused request reaches an object. A listed person passes,
  // whatever case the list or the token writes the address in. Mutants: a
  // dev-door identity counted; any signed-in caller counted; no one counted;
  // the list compared unnormalized; the 403 answered by an object.
  it("(battery 32) a dev-door identity is never an operator, even claiming a listed address; nor is a person not listed, or no one", async () => {
    const claimed = { "X-Yokohama-Claimed-User": BOSS };
    const token = await credential();
    expect((await worker("/api/me", { token, headers: claimed, env: OPS })).text, "premise: the credential is the caller BOSS").toBe(
      `{"mode":"cf-access","user":"${BOSS}"}`,
    );
    const refused: Array<[string, Sent]> = [
      ["the credential claiming a listed address", { token, headers: claimed }],
      ["a person not listed", { token: await person(RED) }],
      ["no one", {}],
    ];
    for (const [who, sent] of refused) {
      for (const [method, path] of [
        ["GET", "/api/admin/tables"],
        ["POST", "/api/admin/tables/op-gate/expire?force"],
      ] as const) {
        const res = await reaching(() => worker(path, { ...sent, method, env: OPS }));
        expect([res.result.status, res.result.text], `${who}: ${method} ${path}`).toEqual([403, NOT_AN_OPERATOR]);
        expect(res.reached, `${who}: no object reached`).toBe(0);
      }
    }
    for (const address of [BOSS, "Boss@T.Test"]) {
      expect((await worker("/api/admin/tables", { token: await person(address), env: OPS })).status, address).toBe(200);
    }
  });

  // Spec §2.9's clean off-path: with the variable absent the routes are
  // unrouted `/api/*`, the house's `dispatch`, and the layer's own 404 — for
  // the would-be operator too — and no table is touched. Mutants: the gate run
  // with no list (a 403); the expire forwarded to the table.
  it("with YOKOHAMA_WEB_ADMINS absent the routes are the layer's 404, and no table is touched", async () => {
    const id = "op-unset";
    const stub = await seedPlaying(id);
    const token = await person(BOSS);
    const list = await worker("/api/admin/tables", { token });
    expect(list).toEqual(layerAnswer("GET", "/api/admin/tables"));
    expect(list.status, "premise: the layer's 404").toBe(404);
    const path = `/api/admin/tables/${id}/expire`;
    const forced = await reaching(() => worker(`${path}?force`, { token, method: "POST" }));
    expect(forced.result).toEqual(layerAnswer("POST", path));
    expect(forced.result.status).toBe(404);
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status, "the table is untouched").toBe(200);
  });

  // An operator's path the surface does not name is unrouted: the layer's own
  // 404, as for anyone — and so is a query a route does not take, its keys
  // compared exactly (the orchestrator's ruling I-72). Mutants: a loose route
  // (a GET of an expire, a POST of the listing, `/api/admin` itself answered
  // as the listing); the listing taking any query; a key beside `force`
  // ignored.
  it("an operator's path the surface does not name is the layer's 404", async () => {
    const token = await person(BOSS);
    for (const [method, path, query] of [
      ["GET", "/api/admin", ""],
      ["GET", "/api/admin/tables/op-none/expire", ""],
      ["POST", "/api/admin/tables", ""],
      ["POST", "/api/admin/tables/op-none/expire/now", ""],
      ["GET", "/api/admin/tables", "scope=all"],
      ["POST", "/api/admin/tables/op-none/expire", "force&dry=1"],
    ] as const) {
      const res = await worker(query === "" ? path : `${path}?${query}`, { token, method, env: OPS });
      expect(res, `${method} ${path}?${query}`).toEqual(layerAnswer(method, path, query));
      expect(res.status).toBe(404);
    }
    expect(await tablesIn(tableStub("op-none")), "no table object was made").toBe(0);
  });
});

describe("the listing (spec §2.9)", () => {
  // Every table the house holds a copy of, a DEALING one among them — which
  // the lobby's own pages never show a stranger — with the ten fields, its
  // size and its last activity as the house last received them. Logged with
  // the operator's address. Two tables carry a name — the open one through
  // its create, and a FINISHED game through its document — and the finished
  // one is stamped at game over (the committed owed-game-over pair, named in
  // its text: winner seat 1, created 1_789_956_559, stamped at the pool's
  // frozen reading), so every field is read at a value other than its
  // default. Mutants: a status filtered out; a field missing or renamed; the
  // name, `over`, the winner or the stamp read as its default whatever the
  // copy says (the review's M-2); the size or last activity from anywhere but
  // the slot; no line.
  it("lists every table the house holds, a dealing one among them, with its size and last activity", async () => {
    const [open, dealing, playing, won] = ["op-list-open", "op-list-deal", "op-list-play", "op-list-won"];
    const named = CREATE_BODY.replace(/}$/, ',"name":"Café ☕ 東京"}');
    expect((await call(tableStub(open), RED, "POST", "/api/games", named, open)).status).toBe(201);
    expect((await call(tableStub(dealing), RED, "POST", "/api/games", CREATE_BODY, dealing)).status).toBe(201);
    expect((await call(tableStub(dealing), "blue@t.test", "POST", `/api/games/${dealing}/join`, '{"seat":1}')).status).toBe(200);
    await seedPlaying(playing);
    const finished = replaceOnce(OWED_GAME_OVER_TABLE_JSON, `"id": "${OWED}",\n  "name": null`, `"id": "${won}",\n  "name": "Finished ☕"`);
    const wonStub = await holding(won, { table: finished, record: OWED_GAME_OVER_JSON });
    // Its save's lines are the archive cells' to read; here they are quiet.
    await logsDuring(async () => {
      expect((await call(wonStub, LOCAL, "GET", `/api/games/${won}/table`)).status, "premise: the wake plays the game to its end").toBe(200);
      for (const id of [open, dealing, won]) await settle(tableStub(id));
    });
    expect(archivedAt(await docOf(wonStub, "table")), "premise: stamped at game over, at the frozen reading").toBe(1_789_000_000);
    const token = await person(BOSS);
    const { result, lines } = await logsDuring(() => worker("/api/admin/tables", { token, env: OPS }));
    expect(result.status).toBe(200);
    expect(lines).toEqual([{ event: "operator", action: "list", address: BOSS }]);
    const rows = (JSON.parse(result.text) as { tables: Array<Record<string, unknown>> }).tables.filter((r) =>
      [open, dealing, playing, won].includes(String(r.id)),
    );
    const want = async (id: string, status: string, read: Record<string, unknown> = {}) => {
      const stored = await copyOf(id);
      expect(stored?.bytes, `premise: the house holds ${id}'s size`).toBeGreaterThan(0);
      return {
        id,
        name: null,
        status,
        players: 2,
        created_at: 1_789_000_000,
        last_active: stored!.last_active,
        bytes: stored!.bytes,
        over: false,
        winner: null,
        archived_at: null,
        ...read,
      };
    };
    expect(rows).toEqual([
      await want(dealing, "dealing"),
      await want(open, "open", { name: "Café ☕ 東京" }),
      await want(playing, "playing"),
      await want(won, "playing", { name: "Finished ☕", created_at: 1_789_956_559, over: true, winner: 1, archived_at: 1_789_000_000 }),
    ]);
  });
});

describe("the operator's expire (spec §2.9)", () => {
  // Spec §2.4's path, as the creator's own abandon, with reason `operator`:
  // saved first, the marker after, and the answer the layer's 204; a second
  // expire then meets the marker's 404, as every request does. Mutants: a
  // marker under another reason; the expire made without the save; the
  // operator's action not logged, or logged without its address; a plain
  // expire passed through to the layer under a marker (its bare "no game
  // named …").
  it("ends a table as its creator's abandon, saved first, and its marker names the operator", async () => {
    const id = "op-expire";
    const stub = await seedPlaying(id);
    const recordKey = `test/rev7/abandoned/${id}.json`;
    const { result, lines } = await logsDuring(async () => {
      const res = await expire(id);
      await settle(stub);
      return res;
    });
    expect(result).toEqual({ status: 204, text: "" });
    expect(ends(lines)).toEqual([
      { event: "operator", id, action: "expire", address: BOSS },
      { event: "archive-landed", id, site: "operator", record_key: recordKey, detail: recordKey },
      { event: "expired", id, reason: "operator", kept: "archive" },
    ]);
    const marker = await markerOf(stub);
    expect(marker).toEqual({ at: expect.any(Number), reason: "operator", kept: "archive" });
    const read = await worker(`/api/games/${id}/table`, { token: await person(RED) });
    const day = new Date(marker!.at as number).toISOString().slice(0, 10);
    const sentence = `this table was removed by an operator on ${day} — its game is in the archive`;
    expect([read.status, JSON.parse(read.text)]).toEqual([404, { error: sentence }]);
    const again = await logsDuring(() => expire(id));
    expect([again.result.status, JSON.parse(again.result.text)], "a second expire meets the marker, as every request does").toEqual([
      404,
      { error: sentence },
    ]);
  });

  // R-6.5.5 for the operator: a put that fails REFUSES the expire with the
  // layer's own sentence, and nothing is written — no marker, and nothing
  // the alarm's expiry keeps (`held`, a retry count). Mutant: the refusal
  // read as an expiry's failure, or the table deleted anyway.
  it("answers the layer's refusal when its save fails, and the table stays", async () => {
    const id = "op-expire-fail";
    const stub = await seedPlaying(id);
    const failing = await swapBucket(stub, failingPuts(Infinity));
    try {
      const { result } = await logsDuring(() => expire(id));
      expect([result.status, JSON.parse(result.text)]).toEqual([
        500,
        { error: `the archive could not be written (${R2_UNREACHABLE}) — the table was not deleted` },
      ]);
    } finally {
      await failing.restore();
    }
    expect(await markerOf(stub)).toBeUndefined();
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k IN ('held', 'expiry_failed')")).toEqual([]);
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status).toBe(200);
  });
});

describe("the operator's force (spec §2.9)", () => {
  /** A table at `id` in the state `kind` names. */
  async function arrange(kind: "poisoned" | "held" | "live" | "marker" | "waiting", id: string) {
    const stub = await seedPlaying(id);
    let socket: { messages: string[]; closed: Promise<{ code: number; reason: string }> } | undefined;
    if (kind === "poisoned") {
      await runInDurableObject(stub, (_t, s) => {
        s.storage.transactionSync(() => s.storage.sql.exec("INSERT INTO meta (k, v) VALUES ('load_traps', '2'), ('poison', 'record')"));
      });
      const res = await logsDuring(() => call(stub, RED, "GET", `/api/games/${id}`));
      expect([res.result.status, await res.result.json()], "premise: poisoned").toEqual([500, { error: "this table's stored record cannot be loaded" }]);
    }
    if (kind === "held") {
      await env.ARCHIVE!.put(`test/rev7/abandoned/${id}.json`, ANOTHER_RECORD);
      await backdate(stub, Date.now() - 91 * DAY_MS);
      await logsDuring(async () => {
        await runDurableObjectAlarm(stub);
        await settle(stub);
      });
      expect((await sqlOf(stub, "SELECT k FROM meta WHERE k = 'held'")).length, "premise: held").toBe(1);
    }
    if (kind === "live") {
      const res = await stub.fetch(`${ORIGIN}/api/games/${id}/stream`, { headers: { Upgrade: "websocket", "X-Yokohama-User": RED } });
      const ws = res.webSocket!;
      ws.accept();
      const messages: string[] = [];
      ws.addEventListener("message", (e) => {
        messages.push(String(e.data));
      });
      const closed = new Promise<{ code: number; reason: string }>((resolve) =>
        ws.addEventListener("close", (e) => resolve({ code: e.code, reason: e.reason })),
      );
      socket = { messages, closed };
    }
    if (kind === "marker") {
      expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status).toBe(204);
      await settle(stub);
      expect(await sqlOf(stub, "SELECT k FROM meta WHERE k IN ('marker', 'slot') ORDER BY k"), "premise: a marker, its removal acknowledged").toEqual([
        { k: "marker" },
      ]);
    }
    if (kind === "waiting") {
      const failing = await failPushes(stub, 1);
      try {
        expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status).toBe(204);
        await logsDuring(() => settle(stub));
      } finally {
        await failing.restore();
      }
      expect(await sqlOf(stub, "SELECT k FROM meta WHERE k IN ('marker', 'slot') ORDER BY k"), "premise: a marker, its removal still waiting").toEqual([
        { k: "marker" },
        { k: "slot" },
      ]);
    }
    return { stub, socket };
  }

  // Spec §2.9: force ends the object WHATEVER it holds — with no layer, no
  // save and NO marker — once the house has the removal; a marker's removal
  // the house already acknowledged is not sent again, and one still waiting
  // is sent with the marker gone. A create at the id then succeeds and the
  // house lists the new table. A live table's sockets end as a delete's do.
  // Mutants: a marker left (its removal's acknowledgement would then keep the
  // object for the marker's 90 days: 503); the storage kept; the removal sent
  // again for a marker the house acknowledged; the layer or a wake run on a
  // poisoned table (its load would trap); the sockets left open; the force
  // not logged, or logged without its address.
  it.each(["poisoned", "held", "live", "marker", "waiting"] as const)("ends a %s object: no marker, an empty sqlite_master, and the id free again", async (kind) => {
    const id = `op-force-${kind}`;
    const { stub, socket } = await arrange(kind, id);
    const { result, lines } = await logsDuring(async () => {
      const res = await expire(id, true);
      await settle(stub);
      return res;
    });
    expect(result).toEqual({ status: 204, text: "" });
    expect(ends(lines)).toEqual([
      { event: "operator", id, action: "force", address: BOSS },
      { event: "forced", id },
    ]);
    const pushed = lines.filter((l) => (l as { event: string }).event === "slot-pushed");
    expect(pushed, "the removal sent unless the house already had it").toEqual(
      kind === "marker" ? [] : [expect.objectContaining({ id, removal: true })],
    );
    expect(await tablesIn(stub), "the object has ceased to exist").toBe(0);
    expect(await alarmOf(stub)).toBeNull();
    expect(await copyOf(id), "the house lists nothing for it").toBeUndefined();
    if (socket !== undefined) {
      expect(await socket.closed).toEqual({ code: 4404, reason: "this table was deleted" });
      expect(socket.messages.at(-1)).toBe('{"event":"end","data":{"reason":"deleted"}}');
    }
    const retired = await retiredAt(id);
    expect(retired.length, "premise: the house retired the forced table's generation").toBeGreaterThan(0);
    const again = await recreate(id);
    expect(again.status, "a create at the id").toBe(201);
    expect(again.generation.at, "the new generation orders after every retired one, by its time").toBeGreaterThan(Math.max(...retired));
    expect((await copyOf(id))?.gen_at, "the house lists the NEW table").toBe(again.generation.at);
  });

  // The orchestrator's ruling I-72: `?force` is the BARE key. With any value
  // it is a 400 naming the switch — answered by the Worker, reaching no
  // object, the table untouched — never a force and never a plain expire,
  // because `?force=false` forcing would drop a save the operator meant to
  // keep. A key in another case is unrouted: the layer's own 404, as for an
  // unknown path. `?force=` is the bare key written with its `=`, and forces.
  // Mutants: a force read from any value; a value's refusal answered as a
  // plain expire (the table then ends, saved, with a marker); `?Force` taken
  // as the key.
  it("takes ?force bare: a value is a 400 naming the switch, another key is unrouted (ruling I-72)", async () => {
    const id = "op-force-value";
    const stub = await seedPlaying(id);
    const token = await person(BOSS);
    const path = `/api/admin/tables/${id}/expire`;
    for (const value of ["false", "0", "1", "true"]) {
      const res = await reaching(() => worker(`${path}?force=${value}`, { method: "POST", token, env: OPS }));
      expect([res.result.status, JSON.parse(res.result.text)], value).toEqual([400, { error: FORCE_VALUE }]);
      expect(res.reached, `${value}: no object reached`).toBe(0);
    }
    const unrouted = await worker(`${path}?Force`, { method: "POST", token, env: OPS });
    expect(unrouted).toEqual(layerAnswer("POST", path, "Force"));
    expect(unrouted.status, "premise: the layer's 404").toBe(404);
    expect(await markerOf(stub), "the table untouched: no marker").toBeUndefined();
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status, "and it still answers").toBe(200);
    const forced = await logsDuring(async () => {
      const res = await worker(`${path}?force=`, { method: "POST", token, env: OPS });
      await settle(stub);
      return res;
    });
    expect(forced.result).toEqual({ status: 204, text: "" });
    expect(await tablesIn(stub), "`?force=` forced").toBe(0);
  });

  // Spec §2.9: "at once, writing nothing, when the object holds nothing".
  // Mutants: a removal written for a table that never existed; a batch that
  // creates the tables.
  it("answers 204 at once on an object holding nothing, and writes nothing", async () => {
    const id = "op-force-empty";
    const { result, lines } = await logsDuring(() => expire(id, true));
    expect(result).toEqual({ status: 204, text: "" });
    expect(lines).toEqual([{ event: "operator", id, action: "force", address: BOSS }]);
    expect(await tablesIn(tableStub(id))).toBe(0);
    expect(await alarmOf(tableStub(id))).toBeNull();
  });

  // A force ends a table with no save, so a game-over save the table still
  // owes goes with it — logged, as a delete's while the archive is off (spec
  // §2.6; the orchestrator's ruling I-70) — and logged in ONE order on both of
  // the force's paths: the dropped save, then `forced` (the review's L-9). The
  // owed save is arranged as battery 27's is. "At once": the table was never
  // sent a copy, so it has no generation and ends at once. "After its push":
  // the alarm's retry (its put failing again) first delivers the table's copy,
  // so the force has a REMOVAL to send, and ends at its acknowledgement.
  // Mutants: the dropped save not logged; `forced` logged before it on the
  // path that ends at once.
  it.each([
    ["at once", 1, false],
    ["after its push", 2, true],
  ] as const)("drops a save the table still owes, and logs it before it ends: %s", async (how, attempt, delivered) => {
    const id = `op-force-owed-${delivered ? "pushed" : "at-once"}`;
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const failing = await swapBucket(stub, failingPuts(attempt));
    try {
      await logsDuring(async () => {
        expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
        await settle(stub);
        if (delivered) {
          expect(await runDurableObjectAlarm(stub)).toBe(true);
          await settle(stub);
        }
      });
    } finally {
      await failing.restore();
    }
    const owed = (await sqlOf(stub, "SELECT v FROM meta WHERE k = 'archive_owed'")).map((r) => (JSON.parse(String(r.v)) as { attempt: number }).attempt);
    expect(owed, `premise: a save is owed, failed ${attempt} time(s)`).toEqual([attempt]);
    expect((await revisionOf(stub)) > 0, `premise (${how}): the house ${delivered ? "was" : "was never"} sent the table's copy`).toBe(delivered);
    const { result, lines } = await logsDuring(async () => {
      const res = await expire(id, true);
      await settle(stub);
      return res;
    });
    expect(result).toEqual({ status: 204, text: "" });
    expect(ends(lines)).toEqual([
      { event: "operator", id, action: "force", address: BOSS },
      { event: "archive-owed-dropped", id, record_key: recordKey, attempt },
      { event: "forced", id },
    ]);
    expect(await tablesIn(stub)).toBe(0);
  });

  // Spec §2.4: the alarm a write makes due is set IN its batch, so a shutdown
  // between the batch and the side channel leaves the alarm to finish it — a
  // deploy mid-force among them. The force's push is held on a PROTOTYPE gate
  // (pool fact 1), the objects aborted mid-push (which does not drain it), the
  // gate restored, and a fresh stub's alarm then delivers the removal and
  // ends the object — battery 25's arrangement. The table's own alarm — its
  // expiry, 90 days out, which the pool would run on demand all the same — is
  // cleared first, so the only alarm left is one the force's batch set.
  // Mutant: the force's batch arms nothing (the alarm run is false, and the
  // object never ends; in production the removal would wait for the expiry
  // alarm, and a create at the id would be 409 until then).
  it("sets the removal's alarm in its batch: a force cut off mid-push is finished by the alarm", async () => {
    const id = "op-force-cut";
    const stub = await seedPlaying(id);
    await runInDurableObject(stub, (_t, s) => s.storage.deleteAlarm());
    expect(await alarmOf(stub), "premise: no alarm but the force's can run").toBeNull();
    const removal = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, removal]]);
    try {
      const forcing = expire(id, true).catch((e: unknown) => e);
      for (let i = 0; i < 400 && !held.reached(id, removal); i++) await new Promise((r) => setTimeout(r, 5));
      expect(held.reached(id, removal), "premise: the force's push is held").toBe(true);
      await abortAllDurableObjects();
      await forcing;
    } finally {
      held.restore();
    }
    const fresh = tableStub(id);
    expect(await logsDuring(() => runDurableObjectAlarm(fresh)).then((r) => r.result)).toBe(true);
    await settle(fresh);
    expect(await tablesIn(fresh), "the alarm delivered the removal, and the object has ended").toBe(0);
  });

  // Spec §2.9: force answers 204 only once the storage is gone, so a push the
  // house does not answer is 503 in the host's words. The object then holds
  // only the REMOVAL: a create meanwhile is the 409, and the alarm finishes
  // the force. Mutants: 204 before the house acknowledged; no retry armed
  // (the alarm would not run); the documents kept until the ack; no 409.
  it("answers 503 when the house has not confirmed the removal; a create meanwhile is 409, and the alarm finishes it", async () => {
    const id = "op-force-503";
    const stub = await seedPlaying(id);
    const failing = await failPushes(stub, 1);
    let answered: { status: number; text: string };
    try {
      answered = (await logsDuring(() => expire(id, true))).result;
    } finally {
      await failing.restore();
    }
    expect([answered.status, JSON.parse(answered.text)]).toEqual([503, UNCONFIRMED]);
    expect(await sqlOf(stub, "SELECT kind FROM docs"), "the documents are gone").toEqual([]);
    expect(await markerOf(stub)).toBeUndefined();
    const slot = JSON.parse(String((await sqlOf(stub, "SELECT v FROM meta WHERE k = 'slot'"))[0]?.v)) as { copy: unknown; attempt: number };
    expect([slot.copy, slot.attempt], "the REMOVAL waits, its failure counted").toEqual([null, 1]);
    const body = CREATE_BODY.replace(/}$/, `,"id":"${id}"}`);
    const meanwhile = await worker("/api/games", { method: "POST", token: await person(RED), headers: { "content-type": "application/json" }, body });
    expect([meanwhile.status, JSON.parse(meanwhile.text)]).toEqual([409, USED_RECENTLY]);
    const { lines } = await logsDuring(async () => {
      expect(await runDurableObjectAlarm(stub)).toBe(true);
      await settle(stub);
    });
    expect(ends(lines)).toEqual([{ event: "forced", id }]);
    expect(await tablesIn(stub)).toBe(0);
    expect((await recreate(id)).status).toBe(201);
  });
});
