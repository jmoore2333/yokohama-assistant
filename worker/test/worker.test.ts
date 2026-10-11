import { createExecutionContext, evictDurableObject, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { beforeAll, describe, expect, it, vi } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule, type Reply } from "../gen/factory.js";
import { Archive } from "../src/archive";
import * as configModule from "../src/config";
import type { Env } from "../src/env";
import { House } from "../src/house";
import type { JwkSet, KeySource } from "../src/identity";
import { BODY_LIMIT, handle } from "../src/index";
import * as moduleModule from "../src/module";
import { readReply } from "../src/module";
import { Table } from "../src/table";
import { WIRE } from "./corpus.gen";
import { ART_MANIFEST_JSON } from "./fixtures.gen";
import { BLUE, CREATE_BODY, RED, logsDuring, seedPlaying, sqlOf, tableStub } from "./helpers";
import { credentialClaims, personClaims, signer, type Signer } from "./tokens";

const ORIGIN = "https://yokohama.test";
const JSON_TYPE = { "content-type": "application/json" };
const UPGRADE = { Upgrade: "websocket" };
const NO_ONE = '{"mode":"cf-access","user":null}';

/** How long a cell waits for a message it EXPECTS (the orchestrator's ruling
 *  R-10: nothing waits out a timer to show that something did not arrive). */
const WAIT_MS = 2_000;

/** The pool's Access values (`wrangler.test.jsonc`), and the time now. */
function at() {
  return { teamDomain: env.ACCESS_TEAM_DOMAIN!, aud: env.ACCESS_AUD!, now: Math.floor(Date.now() / 1000) };
}

/** The key the pool's Access "signs" with; `key.set()` is the team's keys. */
let key: Signer;
beforeAll(async () => {
  key = await signer("k1");
});

const person = (email: string) => key.sign(personClaims(email, at()));
const credential = () => key.sign(credentialClaims(env.DEV_DOOR_CLIENT_ID!, at()));

/** A test key source: `set` for `keys(false)`, `refreshed` for `keys(true)`,
 *  and every ask recorded — no network, no backdoor. */
function keysOf(set: JwkSet, refreshed: JwkSet = set): { keys: KeySource; asked: boolean[] } {
  const asked: boolean[] = [];
  return {
    asked,
    keys: async (refresh) => {
      asked.push(refresh);
      return refresh ? refreshed : set;
    },
  };
}

interface Sent {
  method?: string;
  headers?: Record<string, string>;
  body?: BodyInit | null;
  /** The Access assertion, as `Cf-Access-Jwt-Assertion`. */
  token?: string;
  env?: Env;
  keys?: KeySource;
}

interface Answer {
  status: number;
  headers: Headers;
  text: string;
  webSocket: WebSocket | null;
}

/** One request through the Worker's `handle`, answered whole. An upgrade's
 *  socket is handed back unread. */
async function worker(path: string, sent: Sent = {}): Promise<Answer> {
  const headers = new Headers(sent.headers);
  if (sent.token !== undefined) headers.set("Cf-Access-Jwt-Assertion", sent.token);
  const req = new Request(ORIGIN + path, { method: sent.method ?? "GET", headers, body: sent.body ?? null });
  const res = await handle(req, sent.env ?? env, createExecutionContext(), sent.keys ?? keysOf(key.set()).keys);
  const webSocket = res.webSocket;
  return { status: res.status, headers: res.headers, text: webSocket ? "" : await res.text(), webSocket };
}

/** The house object, reached as the Worker reaches it. */
function houseStub(): DurableObjectStub<House> {
  return env.HOUSE.get(env.HOUSE.idFromName("house"));
}

/** Rows `query` answers inside the house. */
function houseSql(query: string): Promise<Record<string, SqlStorageValue>[]> {
  return runInDurableObject(houseStub(), (_o, state) => state.storage.sql.exec(query).toArray());
}

/**
 * Which objects `f`'s requests reached, and what each was handed: a spy on
 * each class's `fetch`, on the PROTOTYPE (pool fact 1), calling through. The
 * archive objects' names are read off the instances the spy saw.
 */
async function reaching<T>(f: () => Promise<T>) {
  const table = vi.spyOn(Table.prototype, "fetch");
  const house = vi.spyOn(House.prototype, "fetch");
  const archive = vi.spyOn(Archive.prototype, "fetch");
  try {
    const result = await f();
    const name = (o: unknown) => (o as { ctx: DurableObjectState }).ctx.id.name;
    return {
      result,
      table: table.mock.calls.map(([r]) => r),
      house: house.mock.calls.map(([r]) => r),
      archive: archive.mock.calls.map(([r]) => r),
      archiveNames: archive.mock.contexts.map(name),
    };
  } finally {
    table.mockRestore();
    house.mockRestore();
    archive.mockRestore();
  }
}

/** The `x-yokohama-*` headers an object was handed: the names it trusts. */
function yokohamaHeaders(req: Request): Record<string, string> {
  return Object.fromEntries([...req.headers].filter(([name]) => name.startsWith("x-yokohama-")));
}

/** The layer's own answer, from a fresh module in this configuration: what
 *  the house answers for a request no table object may take. */
function layerAnswer(user: string, method: string, path: string, body = ""): { status: number; body: string } {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  mod.set_clock(1_789_000_000);
  const r = readReply(mod.dispatch(user, method, path, "", body ? "application/json" : undefined, new TextEncoder().encode(body)));
  return { status: r.status, body: r.body };
}

/** A socket through the Worker, accepted, with every message kept. */
async function socket(res: Answer) {
  const ws = res.webSocket;
  if (ws === null) throw new Error(`the upgrade was answered ${res.status}, unupgraded: ${res.text}`);
  ws.accept();
  const messages: string[] = [];
  const waiting = new Set<() => void>();
  ws.addEventListener("message", (e) => {
    messages.push(typeof e.data === "string" ? e.data : "<binary>");
    for (const w of [...waiting]) w();
  });
  return {
    ws,
    messages,
    until: (n: number) =>
      new Promise<string[]>((resolve, reject) => {
        const check = () => {
          if (messages.length < n) return;
          clearTimeout(timer);
          waiting.delete(check);
          resolve([...messages]);
        };
        const timer = setTimeout(() => {
          waiting.delete(check);
          reject(new Error(`${n} messages awaited; arrived: ${JSON.stringify(messages)}`));
        }, WAIT_MS);
        waiting.add(check);
        check();
      }),
  };
}

describe("the Worker's own answers (spec §2.3)", () => {
  // Battery 14's mutant lives in the asset layer and is Task 7's guard's and
  // the live reading's; this is the Worker's half. `/index.html` and `/` never
  // reach the Worker in production (the asset layer answers them), so handing
  // them to `handle` shows it has no fallback to the shell of its own. Mutants:
  // a fallback to `index.html`; a JSON 404.
  it.each(["/assets/x.js", "/sw2.js", "/t/some-table", "/api", "/index.html", "/"])(
    "a missing %s answers 404 with an empty body, never index.html",
    async (path) => {
      const res = await worker(path);
      expect([res.status, res.text, res.headers.get("content-type")]).toEqual([404, "", null]);
    },
  );

  // Mutants: another redirect status or target; a POST reaching the house,
  // whose `dispatch` would answer the layer's JSON 404 for `/enter`.
  it("/enter answers 302 to /#/lobby; any other method on it, or on /api/art, is 405 with an empty body", async () => {
    for (const method of ["GET", "HEAD"]) {
      const res = await worker("/enter", { method });
      expect([res.status, res.headers.get("location"), res.text]).toEqual([302, "/#/lobby", ""]);
    }
    for (const path of ["/enter", "/api/art"]) {
      const res = await worker(path, { method: "POST", body: "x" });
      expect([res.status, res.text]).toEqual([405, ""]);
    }
  });

  // The NAS's own bound and sentence, read live on 2026-09-23: 2,097,153 bytes
  // is 400 with this text, and exactly 2,097,152 passes the bound (a 401, the
  // caller-first answer). Mutants: `>=` for `>`; the sentence reworded; no
  // bound, so the create's 401 answers instead.
  it("a body over 2 MiB answers 400 with the NAS's sentence; exactly 2 MiB passes the bound", async () => {
    expect(BODY_LIMIT).toBe(2_097_152);
    const over = await worker("/api/games", { method: "POST", headers: JSON_TYPE, body: new Uint8Array(BODY_LIMIT + 1).fill(97) });
    expect([over.status, over.text]).toEqual([400, '{"error":"the body could not be read: length limit exceeded"}']);
    const exact = await worker("/api/games", { method: "POST", headers: JSON_TYPE, body: new Uint8Array(BODY_LIMIT).fill(97) });
    expect(exact.status).toBe(401);
  });

  // The bound is on the bytes as they arrive, not on a declared length.
  // Mutant: only `content-length` checked — this body declares none.
  it("bounds a streamed body that declares no length", async () => {
    const chunk = new Uint8Array(64 * 1024).fill(97);
    let sent = 0;
    const body = new ReadableStream<Uint8Array>({
      pull(controller) {
        if (sent > BODY_LIMIT) return controller.close();
        controller.enqueue(chunk);
        sent += chunk.byteLength;
      },
    });
    const res = await worker("/api/chat", { method: "POST", headers: JSON_TYPE, body });
    expect([res.status, res.text]).toEqual([400, '{"error":"the body could not be read: length limit exceeded"}']);
  });

  // Spec §2.6: the Worker's form of the NAS refusing to start, on every
  // `/api/*` — and after the three routes that need no configuration. The
  // other half (`/` untouched) never reaches `handle` and is the live reading.
  // Mutants: no check (the house, configured by the pool, answers 200); the
  // check before routing (`/enter` answers 500).
  it("a bad YOKOHAMA_WEB_ARCHIVE answers 500 naming it on every /api/* path", async () => {
    const bad = { ...env, YOKOHAMA_WEB_ARCHIVE: "s3" };
    for (const path of ["/api/health", "/api/art", "/api/games/wire/table"]) {
      const res = await worker(path, { env: bad });
      expect([res.status, res.text], path).toEqual([500, '{"error":"YOKOHAMA_WEB_ARCHIVE must be r2 or off — got \\"s3\\""}']);
    }
    expect((await worker("/enter", { env: bad })).status).toBe(302);
    expect((await worker("/assets/x.js", { env: bad })).status).toBe(404);
  });

  // M6.5c2, spec §2.1 (R-6.5c2.5): the door has NO default. The Worker refuses
  // before any object is reached, as it refuses every variable it cannot read
  // — and a cell's override reaches the Worker's own `shellConfig` alone,
  // which is exactly this instrument (review R2.2-M2): the objects read their
  // configuration file's. Mutants: a default door (the absence read as
  // `worker`); the check before routing (`/enter` answers 500).
  it("an unset YOKOHAMA_WEB_IDENTITY answers 500 naming it on every /api/* path", async () => {
    const unset = { ...env, YOKOHAMA_WEB_IDENTITY: undefined };
    const body = JSON.stringify({
      error: "YOKOHAMA_WEB_IDENTITY must be worker or open — it is not set, and there is no default: a door is a deployment decision",
    });
    for (const path of ["/api/health", "/api/me", "/api/art", "/api/games/wire/table"]) {
      const res = await worker(path, { env: unset, token: await person(RED) });
      expect([res.status, res.text], path).toEqual([500, body]);
    }
    expect((await worker("/enter", { env: unset })).status).toBe(302);
    expect((await worker("/assets/x.js", { env: unset })).status).toBe(404);
  });

  // M6.5c2, spec §2.1 (review R2.2-M1): what the OPEN door refuses beside it —
  // an operator list, and the dev door on — answers every `/api/*` the 500
  // naming both variables, as the unset door does, before any object is
  // reached. This configuration is the worker door's, whose dev door is ON, so
  // an override of the door alone meets the dev door's refusal, and one that
  // also sets a list meets the list's first. The override reaches the Worker's
  // own `shellConfig` alone (review R2.2-M2), which is where both are read.
  // The partner: the same override with the dev door off and no list is no
  // refusal. Mutants: either refusal missing; the list's value echoed.
  it("an operator list, or the dev door on, beside YOKOHAMA_WEB_IDENTITY=open answers 500 naming both on every /api/* path", async () => {
    const opened = { ...env, YOKOHAMA_WEB_IDENTITY: "open" };
    const refusals: Array<[Env, string]> = [
      [
        { ...opened, YOKOHAMA_WEB_ADMINS: "boss@t.test" },
        "YOKOHAMA_WEB_ADMINS is set beside YOKOHAMA_WEB_IDENTITY=open — a claimed name is never an operator: " +
          "delete the operator list (wrangler secret delete YOKOHAMA_WEB_ADMINS), or deploy the worker door",
      ],
      [
        opened,
        "YOKOHAMA_DEV_DOOR=on beside YOKOHAMA_WEB_IDENTITY=open — the dev door is the worker door's test instrument, " +
          "and under open a claim needs no credential: set YOKOHAMA_DEV_DOOR to off, or deploy the worker door",
      ],
    ];
    expect(env.YOKOHAMA_DEV_DOOR, "premise: this configuration's dev door is on").toBe("on");
    for (const [overridden, sentence] of refusals) {
      for (const path of ["/api/health", "/api/me", "/api/admin/tables", "/api/games/wire/table"]) {
        const res = await reaching(() => worker(path, { env: overridden }));
        expect([res.result.status, res.result.text], path).toEqual([500, JSON.stringify({ error: sentence })]);
        expect(res.result.text, `${path}: the list's value is echoed nowhere`).not.toContain("boss@t.test");
        expect([res.table.length, res.house.length, res.archive.length], `${path}: no object reached`).toEqual([0, 0, 0]);
      }
      expect((await worker("/enter", { env: overridden })).status).toBe(302);
    }
    const accepted = await worker("/api/health", { env: { ...opened, YOKOHAMA_DEV_DOOR: "off" } });
    expect(accepted.status, accepted.text).toBe(200);
  });

  // The NAS's answer is its typed `ArtManifest`: the file's nine maps, in the
  // struct's declaration order, and NOT the file's two generator keys — the
  // committed `art_manifest.json` (the orchestrator's ruling on Task 12's
  // finding). Mutants: the manifest answered with no JSON type, or not the
  // manifest; the file's bytes streamed whole (the two generator keys); a map
  // dropped or altered by the projection; the keys in another order; a HEAD
  // answered with the body.
  it("GET /api/art is the NAS's art manifest by value, as JSON; HEAD has no body", async () => {
    const res = await worker("/api/art");
    expect([res.status, res.headers.get("content-type")]).toEqual([200, "application/json"]);
    const got = JSON.parse(res.text) as Record<string, unknown>;
    const committed = JSON.parse(WIRE.art_manifest!) as Record<string, unknown>;
    const file = JSON.parse(ART_MANIFEST_JSON) as Record<string, unknown>;
    expect(Object.keys(file), "premise: the file carries the two generator keys the NAS drops").toEqual(
      expect.arrayContaining(["area_duplicates", "provenance"]),
    );
    expect(Object.keys(got).sort(), "exactly the committed payload's keys").toEqual(Object.keys(committed).sort());
    expect(Object.keys(got), "in the struct's order, as the NAS's route writes them").toEqual([
      "orders",
      "techs",
      "sites",
      "achievements",
      "areas",
      "five_power",
      "mercenaries",
      "backs",
      "tokens",
    ]);
    expect(got, "the committed payload, by value").toEqual(committed);
    for (const [key, map] of Object.entries(got)) expect(map, `${key}: the file's own map`).toEqual(file[key]);
    const head = await worker("/api/art", { method: "HEAD" });
    expect([head.status, head.text]).toEqual([200, ""]);
  });
});

describe("identity at the Worker (R-6.5c.3; spec §2.3)", () => {
  // BATTERY 2: the four names the objects trust, forged by a client beside a
  // real person's assertion — and a fifth no object reads yet, which pins the
  // PREFIX rule (Task 11's review, M2). Mutants: the old rule (only
  // `x-yokohama-user` stripped); nothing stripped; the four trusted names
  // stripped by name; the table's name taken from the client's header; the
  // identity header set only where the client sent none; a strip that compares
  // names case-sensitively in capitals.
  it("(battery 2) a client's forged x-yokohama-* headers reach no object; the person is the caller", async () => {
    const forged = {
      "x-yokohama-user": "evil@x.test",
      "x-yokohama-claimed-user": "evil@x.test",
      "x-yokohama-operator": "evil@x.test",
      "x-yokohama-table-id": "elsewhere",
      "x-yokohama-future": "evil@x.test",
    };
    const token = await person(RED);
    const me = await reaching(() => worker("/api/me", { token, headers: forged }));
    expect(me.result.text).toBe(`{"mode":"cf-access","user":"${RED}"}`);
    expect(me.house.map(yokohamaHeaders)).toEqual([{ "x-yokohama-user": RED }]);

    const body = CREATE_BODY.replace(/}$/, ',"id":"forged-create"}');
    const created = await reaching(() => worker("/api/games", { method: "POST", token, headers: { ...forged, ...JSON_TYPE }, body }));
    expect(created.result.status).toBe(201);
    expect((JSON.parse(created.result.text) as { id: string }).id).toBe("forged-create");
    expect(created.table.map(yokohamaHeaders)).toEqual([{ "x-yokohama-user": RED, "x-yokohama-table-id": "forged-create" }]);
    expect(await sqlOf(tableStub("forged-create"), "SELECT kind FROM docs")).toEqual([{ kind: "table" }]);
    expect(await sqlOf(tableStub("elsewhere"), "SELECT count(*) AS n FROM sqlite_master")).toEqual([{ n: 0 }]);
  });

  // Mutant: the door's switch ignored at the Worker. With no client id the
  // door stays shut by two mechanisms — `identityConfig`'s guard, whose node
  // cell pins it, and a `common_name` that can then match nothing — so that
  // half holds the property end to end rather than pinning one mechanism.
  it("the dev door: the credential with a claimed address is that caller; with the door off, or no client id, no one", async () => {
    const token = await credential();
    const claimed = { "X-Yokohama-Claimed-User": "Probe@Deploy.Test" };
    expect((await worker("/api/me", { token, headers: claimed })).text).toBe('{"mode":"cf-access","user":"probe@deploy.test"}');
    for (const shut of [{ ...env, YOKOHAMA_DEV_DOOR: "off" }, { ...env, DEV_DOOR_CLIENT_ID: undefined }]) {
      expect((await worker("/api/me", { token, headers: claimed, env: shut })).text).toBe(NO_ONE);
    }
  });

  // BATTERY 15 through the Worker: a claimed address with no credential's
  // assertion — none at all, or a person's — is never the caller.
  it("(battery 15) a claimed address with no credential's assertion is no one, or the person", async () => {
    const claimed = { "X-Yokohama-Claimed-User": "evil@x.test" };
    expect((await worker("/api/me", { headers: claimed })).text).toBe(NO_ONE);
    expect((await worker("/api/me", { headers: claimed, token: await person(BLUE) })).text).toBe(
      `{"mode":"cf-access","user":"${BLUE}"}`,
    );
  });

  // M6.5c2, spec §2.1: under `worker` the claim's carriers name no one without
  // the credential — the header (battery 15, above) and the upgrade's
  // `claimed` parameter alike — and a person's assertion is the caller
  // whatever the parameter says. The anonymous half claims an ADDRESS: the
  // worker mode reads a plain name as absent however it is forwarded, so only
  // an address can show a parameter that reached `X-Yokohama-User`. Mutants:
  // the parameter read under `worker` (forwarded as the worker door's header,
  // the anonymous half; displacing the person, the person half); the door read
  // per request rather than from the configuration.
  it("under worker an upgrade's claimed parameter is no caller, and never outranks a person", async () => {
    const anonymous = await worker(`/api/stream?${new URLSearchParams({ claimed: "evil@x.test" })}`, { headers: UPGRADE });
    expect([anonymous.status, anonymous.webSocket]).toEqual([401, null]);
    const who = "param-person@t.test";
    const res = await worker(`/api/stream?${new URLSearchParams({ claimed: "evil@x.test" })}`, {
      token: await person(who),
      headers: UPGRADE,
    });
    const tab = await socket(res);
    const attached = await runInDurableObject(houseStub(), (_o, state) =>
      state.getWebSockets("lobby").map((ws) => ws.deserializeAttachment() as { user?: string }),
    );
    expect(attached).toContainEqual({ user: who });
    expect(attached).not.toContainEqual({ user: "evil@x.test" });
    tab.ws.close(1000, "done");
  });

  // Mutant: an archive with no caller sent to an Archive object of its own —
  // which answers the same 401, so the cell reads WHERE it went.
  it("no identity: /api/me reads user null, and POST /api/archive is the house's 401", async () => {
    expect((await worker("/api/me")).text).toBe(NO_ONE);
    const res = await reaching(() =>
      worker("/api/archive", { method: "POST", headers: JSON_TYPE, body: '{"record":"","table":""}' }),
    );
    expect(res.result.status).toBe(401);
    expect((JSON.parse(res.result.text) as { error: string }).error).toContain("X-Yokohama-User from the edge Worker");
    expect([res.house.length, res.archive.length]).toEqual([1, 0]);
  });

  // R1's L-2, end to end: the dev door's claimed value must be an ADDRESS by
  // the layer's email shape (`User::is_email`), or it is no one. Before, `not-an-email`
  // was the dev identity, named an Archive object whose `init` refused it
  // under `r2`, and answered that object's 500 — a sentence no spec lists.
  // Now it is the house's 401, as it is with no caller at all. Mutant: the
  // rule dropped at the dev door.
  it("(L-2) a claimed value that is no address is no one: POST /api/archive is the house's 401", async () => {
    const token = await credential();
    const claimed = { "X-Yokohama-Claimed-User": "not-an-email" };
    expect((await worker("/api/me", { token, headers: claimed })).text).toBe(NO_ONE);
    const res = await reaching(() =>
      worker("/api/archive", { method: "POST", token, headers: { ...JSON_TYPE, ...claimed }, body: '{"record":"","table":""}' }),
    );
    expect(res.result.status).toBe(401);
    expect((JSON.parse(res.result.text) as { error: string }).error).toContain("X-Yokohama-User from the edge Worker");
    expect([res.house.length, res.archive.length]).toEqual([1, 0]);
  });

  // Spec §2.3: the Archive object named `"archive:" + address`, answering as
  // the layer answers. The pool's archive is `r2`, so the layer reads the body
  // before any put, and refuses a record that does not parse. Mutants: the
  // archive sent to the house; the object named without its prefix, or by an
  // address not trimmed and lowercased as the layer reads one.
  it("a signed-in POST /api/archive reaches the caller's own Archive object, which answers as the layer does", async () => {
    const body = '{"record":"","table":""}';
    const res = await reaching(async () =>
      worker("/api/archive", { method: "POST", token: await person(" Red@T.Test "), headers: JSON_TYPE, body }),
    );
    const layer = layerAnswer(RED, "POST", "/api/archive", body);
    expect(layer.status, "premise: the layer refuses the record before any put").toBe(400);
    expect([res.result.status, res.result.text]).toEqual([layer.status, layer.body]);
    expect(res.archiveNames).toEqual([`archive:${RED}`]);
    expect(res.house).toEqual([]);
  });

  // The key rotation (plan review m14; re-review m-d): ONE refresh per
  // request, and only for a kid the cached set lacks. Mutants: no refresh; a
  // refresh for every token; a loop until the kid appears.
  it("(key rotation) a kid the cached set lacks is refreshed once, and the person is named", async () => {
    const rotated = await signer("k2");
    const { keys, asked } = keysOf(key.set(), { keys: [key.jwk, rotated.jwk] });
    const res = await worker("/api/me", { token: await rotated.sign(personClaims(RED, at())), keys });
    expect(res.text).toBe(`{"mode":"cf-access","user":"${RED}"}`);
    expect(asked).toEqual([false, true]);
  });
  it("(key rotation) a kid no set holds is one refresh and no one — never a loop", async () => {
    const stranger = await signer("k-unknown");
    const { keys, asked } = keysOf(key.set());
    const res = await worker("/api/me", { token: await stranger.sign(personClaims(RED, at())), keys });
    expect(res.text).toBe(NO_ONE);
    expect(asked).toEqual([false, true]);
  });
  // Mutants: a refresh on every request, which would let any token force a
  // certs fetch (the once-a-minute limit is the key source's, not `handle`'s);
  // the keys asked for a request that carries no assertion (Task 11's review,
  // M1), which on a cold isolate is a fetch for nothing.
  it("a known kid asks the cached keys once and refreshes nothing; no assertion asks nothing", async () => {
    const withToken = keysOf(key.set());
    expect((await worker("/api/me", { token: await person(RED), keys: withToken.keys })).text).toContain(RED);
    expect(withToken.asked).toEqual([false]);
    const without = keysOf(key.set());
    expect((await worker("/api/me", { keys: without.keys })).text).toBe(NO_ONE);
    expect(without.asked).toEqual([]);
  });

  // A certs fetch that fails is no caller: the request fails CLOSED, and the
  // shell says why (spec §2.8).
  it("a key source that fails leaves the caller no one, and is logged", async () => {
    const keys: KeySource = async () => {
      throw new Error("certs unreachable");
    };
    const token = await person(RED);
    const { result, lines } = await logsDuring(() => worker("/api/me", { token, keys }));
    expect(result.text).toBe(NO_ONE);
    expect(lines).toEqual([{ event: "identity-failed", error: "Error: certs unreachable" }]);
  });

  // The orchestrator's rulings (Task 2's review, E1; Task 11's review, I1):
  // an address the layer cannot read as a header is no caller, is never
  // forwarded as one, and is judged RAW. Mutants: the check dropped — workerd
  // forwards such an address as UTF-8 bytes (warning that a browser would
  // throw), the layer reads them as absent, and only the forwarded headers
  // show it; the check after the normalization — the Kelvin sign is then
  // lowercased to `k` and `\u212Aevin@x.test` IS `kevin@x.test`, another
  // user.
  it("a person whose raw address is not visible ASCII is no caller, and no identity header is forwarded", async () => {
    for (const email of ["rené@x.test", "用户@例子.广告", "\u212Aevin@x.test", "\u00A0red@t.test"]) {
      const res = await reaching(async () => worker("/api/me", { token: await person(email) }));
      expect([res.result.status, res.result.text], JSON.stringify(email)).toEqual([200, NO_ONE]);
      expect(res.house.map(yokohamaHeaders), JSON.stringify(email)).toEqual([{}]);
    }
  });
});

describe("routing to the objects (spec §2.3)", () => {
  // Mutants: a constant name (the second create would find the first's
  // table); fewer than 16 hex digits; the name not handed to the object.
  it("a create with no body id lands at a drawn sim- name and answers 201 naming it; two creates draw two", async () => {
    const token = await person(RED);
    const ids: string[] = [];
    for (let i = 0; i < 2; i++) {
      const res = await reaching(() => worker("/api/games", { method: "POST", token, headers: JSON_TYPE, body: CREATE_BODY }));
      expect(res.result.status).toBe(201);
      const { id } = JSON.parse(res.result.text) as { id: string };
      expect(id).toMatch(/^sim-[0-9a-f]{16}$/);
      expect(res.table.map((r) => r.headers.get("x-yokohama-table-id"))).toEqual([id]);
      expect(await sqlOf(tableStub(id), "SELECT kind FROM docs")).toEqual([{ kind: "table" }]);
      ids.push(id);
    }
    expect(ids[0]).not.toBe(ids[1]);
  });

  // M4 (Task 11's review): every body that names no string `id` — not JSON,
  // not an object, an `id` that is null or a number, bytes that open with a
  // byte-order mark or are not UTF-8 — is created at a DRAWN name, and the
  // layer answers the body as it answers it anywhere. Mutants: `null` read as
  // an object (a throw); a non-string `id` taken as the name; a byte-order
  // mark skipped, or bad UTF-8 decoded leniently — either would name the
  // object by the body's `id`. (A JSON array or string has no `id` to take, so
  // those two rows pin the answer rather than a branch.)
  it.each<[string, Uint8Array]>([
    ["an array", new TextEncoder().encode("[]")],
    ["a string", new TextEncoder().encode('"x"')],
    ["null", new TextEncoder().encode("null")],
    ["an id that is null", new TextEncoder().encode(CREATE_BODY.replace(/}$/, ',"id":null}'))],
    ["an id that is a number", new TextEncoder().encode(CREATE_BODY.replace(/}$/, ',"id":5}'))],
    ["a byte-order mark", new TextEncoder().encode(`\uFEFF${CREATE_BODY.replace(/}$/, ',"id":"named"}')}`)],
    // The bad byte INSIDE the id's string, where a lenient decoder would read
    // U+FFFD and name the object by it.
    [
      "bytes that are not UTF-8",
      Uint8Array.from([
        ...new TextEncoder().encode(CREATE_BODY.replace(/}$/, ',"id":"nam')),
        0xff,
        ...new TextEncoder().encode('ed"}'),
      ]),
    ],
  ])("a create whose body is %s lands at a drawn sim- name", async (_what, body) => {
    const res = await reaching(async () => worker("/api/games", { method: "POST", token: await person(RED), headers: JSON_TYPE, body }));
    expect(res.table.map((r) => r.headers.get("x-yokohama-table-id"))).toEqual([expect.stringMatching(/^sim-[0-9a-f]{16}$/)]);
    expect(res.result.status, "the layer refuses every one of these bodies").toBe(400);
  });

  // The orchestrator's ruling (Task 9's review): a name the layer can never
  // use goes to the house, whose `dispatch` answers the layer's OWN refusal —
  // and no nameless object (a name over 1,024 bytes) is ever made. Mutants:
  // the bound off by one (the 64-byte name would reach the house); the empty
  // segment given a table object.
  it("an empty segment, or one over 64 bytes, is the house's — the layer's own answer — and no table object is made", async () => {
    const token = await person(RED);
    for (const segment of ["", "a".repeat(65), "a".repeat(1_100)]) {
      const path = `/api/games/${segment}/table`;
      const res = await reaching(() => worker(path, { token }));
      expect([res.table.length, res.house.length], path).toEqual([0, 1]);
      const layer = layerAnswer(RED, "GET", path);
      expect([res.result.status, res.result.text], path).toEqual([layer.status, layer.body]);
    }
    const bound = await reaching(() => worker(`/api/games/${"a".repeat(64)}/table`, { token }));
    expect([bound.table.length, bound.house.length], "64 bytes is a table's name").toEqual([1, 0]);
  });

  // The same rule for a create body's `id`: a name over 64 bytes is the
  // house's, whose `create` refuses it — the house never holds a table.
  it("a create naming an id over 64 bytes is the house's, which refuses it as the layer does", async () => {
    const body = CREATE_BODY.replace(/}$/, `,"id":"${"b".repeat(65)}"}`);
    const res = await reaching(async () => worker("/api/games", { method: "POST", token: await person(RED), headers: JSON_TYPE, body }));
    expect([res.table.length, res.house.length]).toEqual([0, 1]);
    const layer = layerAnswer(RED, "POST", "/api/games", body);
    expect(layer.status, "premise: the layer refuses the id").toBe(400);
    expect([res.result.status, res.result.text]).toEqual([layer.status, layer.body]);
  });

  // The strip rule removes `x-yokohama-*` and nothing else: the upgrade's own
  // headers ride along. Mutant: `upgrade` or `connection` stripped as hop-by-
  // hop headers, which the old Worker did — the table then answers 426.
  it("an upgrade to a table's stream forwards, and the object's 101 comes back with its first frame", async () => {
    await seedPlaying("through-worker");
    const tab = await socket(await worker("/api/games/through-worker/stream", { token: await person(RED), headers: UPGRADE }));
    const [first] = await tab.until(1);
    expect(first).toMatch(/^\{"event":"signal","data":\{/);
    tab.ws.close(1000, "done");
  });

  // ESCALATION 1 of Task 11's review, ruled a host-local hardening: an
  // upgrade from ANOTHER site's page — a hostile page in a signed-in player's
  // browser, which WebSockets' lack of CORS would otherwise let read turn and
  // timing frames — is refused 403 unupgraded; the site's own pages, and a
  // client that sends no `Origin` (curl, the deploy readings), pass. A plain
  // request is not an upgrade and is never refused for its `Origin`. Mutants:
  // no check; a prefix compare (`https://yokohama.test.evil.test`); a host
  // compare (`http://yokohama.test`); an absent `Origin` refused; every request
  // checked.
  it("(the Origin check) an upgrade from another site's page is refused 403, unupgraded", async () => {
    const token = await person(RED);
    /** The upgrade, with any socket it wrongly opened closed at once — an
     *  unaccepted socket would hold the house's next eviction open. */
    const upgradeFrom = async (path: string, origin: string) => {
      const res = await worker(path, { token, headers: { ...UPGRADE, Origin: origin } });
      if (res.webSocket) {
        res.webSocket.accept();
        res.webSocket.close(1000, "the cell expected no socket");
      }
      return [res.status, res.webSocket === null, res.text];
    };
    for (const origin of ["https://evil.test", "https://yokohama.test.evil.test", "http://yokohama.test", "null"]) {
      expect(await upgradeFrom("/api/stream", origin), origin).toEqual([403, true, '{"error":"this socket is for the site\'s own pages"}']);
    }
    await seedPlaying("origin-t");
    const table = await upgradeFrom("/api/games/origin-t/stream", "https://evil.test");
    expect(table.slice(0, 2), "a table's stream as well as the lobby's").toEqual([403, true]);
  });
  it("(the Origin check) the site's own page, or no Origin at all, is upgraded; a plain request is never refused for its Origin", async () => {
    const token = await person(RED);
    for (const headers of [{ ...UPGRADE, Origin: ORIGIN }, UPGRADE]) {
      const tab = await socket(await worker("/api/stream", { token, headers }));
      tab.ws.close(1000, "done");
    }
    const plain = await worker("/api/me", { token, headers: { Origin: "https://evil.test" } });
    expect(plain.text).toBe(`{"mode":"cf-access","user":"${RED}"}`);
  });

  // F-1 (the orchestrator's reading of the shell): an object whose `fetch`
  // REJECTS — a throw that is not a trap, such as a reply the shell cannot
  // read, or a batch that did not commit — answers the API's 500 in ONE
  // host-written sentence whichever object threw, logged `object-failed`.
  // Before, it was the platform's own plain 500, which the client cannot read
  // as `{"error": …}`, and no line said so. `readReply` refuses at MODULE
  // scope (pool fact 4), in the table and the house alike. Mutants: no catch;
  // another shape or sentence; no line.
  it("(F-1) an object that throws answers the API's 500, logged, for the table and the house alike", async () => {
    const refusing = vi.spyOn(moduleModule, "readReply").mockImplementation(() => {
      throw new Error("a reply the shell cannot read");
    });
    try {
      for (const [target, path] of [
        ["table", "/api/games/f1-throws/table"],
        ["house", "/api/games"],
      ] as const) {
        const token = await person(RED);
        const { result, lines } = await logsDuring(() => worker(path, { token }));
        expect([result.status, result.headers.get("content-type"), JSON.parse(result.text)], target).toEqual([
          500,
          "application/json",
          { error: "the server could not answer — try again" },
        ]);
        const failed = lines.filter((l) => (l as { event: string }).event === "object-failed");
        expect(failed, target).toEqual([{ event: "object-failed", target, path, error: expect.stringContaining("a reply the shell cannot read") }]);
      }
    } finally {
      refusing.mockRestore();
    }
  });

  // Spec §2.9's off-path: the pool sets no `YOKOHAMA_WEB_ADMINS`, so the
  // operator's paths are unrouted `/api/*` — the house's, which answers the
  // layer's 404. `test/operator.test.ts` is the surface with operators set.
  it("/api/admin paths are the house's: the layer's 404", async () => {
    const res = await reaching(async () => worker("/api/admin/tables", { token: await person(RED) }));
    expect([res.result.status, res.result.text]).toEqual([404, '{"error":"no such endpoint: /api/admin/tables"}']);
    expect(res.house.length).toBe(1);
  });
});

describe("the house, first cut (spec §2.5)", () => {
  // Spec §4's second class — this host's facts, which are the module's own —
  // read through the house, as the live reading 5 reads them.
  it("answers /api/health with this host's facts through the layer's dispatch", async () => {
    const res = await worker("/api/health");
    expect(res.status).toBe(200);
    for (const fact of ['"identity":"cf-access"', '"signal":"ws"', '"archive":"r2"', '"multiplayer":true']) {
      expect(res.text).toContain(fact);
    }
  });

  // R-6.5c2.42: the two TTLs this Worker reads — `wrangler.test.jsonc`'s,
  // production's values — through the house's module, which `init` handed
  // them (`moduleConfig`). Health carries no seed, so it may be parsed. The
  // NAS reports both `null` (the layer's own cell). Mutants: either TTL
  // missing from the module's configuration, or from the host's facts; the
  // two swapped.
  it("answers /api/health with the two TTLs this Worker reads: 90 days, and 7 for a table waiting for players", async () => {
    const res = await worker("/api/health");
    expect(res.status).toBe(200);
    const health = JSON.parse(res.text) as Record<string, unknown>;
    expect([health.table_ttl_days, health.open_table_ttl_days]).toEqual([90, 7]);
  });

  // Mutants: the chat never stored; stored but not loaded on the next wake.
  it("keeps the house chat across an eviction, as ONE docs row of kind house", async () => {
    const text = `kept ${crypto.randomUUID()}`;
    const posted = await worker("/api/chat", { method: "POST", token: await person(RED), headers: JSON_TYPE, body: JSON.stringify({ text }) });
    expect(posted.status).toBe(201);
    await evictDurableObject(houseStub());
    const page = await worker("/api/chat", { token: await person(BLUE) });
    expect((JSON.parse(page.text) as { messages: Array<{ text: string }> }).messages.map((m) => m.text)).toContain(text);
    expect(await houseSql("SELECT kind FROM docs")).toEqual([{ kind: "house" }]);
  });

  // Spec §2.5: the lobby socket is admitted by `identity`, accepted under the
  // tag `lobby` with `{ user }`, silent on connect, and framed on every house
  // chat post with `chat` = the house chat's `last` and (M6.5c2 cycle D) `rev`
  // = its revision, both off the stored document — read back here through the
  // page, which carries the document's `rev` too. The house is evicted
  // after a first post, so a count kept in memory would read 1 where `last`
  // reads more. Mutants: a frame on connect (the first message would be it);
  // an in-memory chat count; no frame; lobby sockets under another tag.
  it("an upgrade to /api/stream: 401 unupgraded with no one; 101 with a caller, silent until a house chat post", async () => {
    const refused = await worker("/api/stream", { headers: UPGRADE });
    expect([refused.status, refused.webSocket]).toEqual([401, null]);
    expect((JSON.parse(refused.text) as { error: string }).error).toContain("X-Yokohama-User from the edge Worker");

    const first = await worker("/api/chat", { method: "POST", token: await person(RED), headers: JSON_TYPE, body: '{"text":"first"}' });
    expect(first.status).toBe(201);
    await evictDurableObject(houseStub());
    const red = await socket(await worker("/api/stream", { token: await person(RED), headers: UPGRADE }));
    const blue = await socket(await worker("/api/stream", { token: await person(BLUE), headers: UPGRADE }));
    const posted = await worker("/api/chat", { method: "POST", token: await person(BLUE), headers: JSON_TYPE, body: '{"text":"anyone?"}' });
    expect(posted.status).toBe(201);
    const { seq } = JSON.parse(posted.text) as { seq: number };
    expect(seq, "premise: the post follows another").toBeGreaterThan(1);
    const page = await worker("/api/chat", { token: await person(BLUE) });
    const { last, rev } = JSON.parse(page.text) as { last: number; rev: number };
    expect([last, rev], "premise: nothing deleted in this house, so the revision is the count").toEqual([seq, seq]);
    const want = `{"event":"house","data":{"seq":0,"chat":${seq},"rev":${rev}}}`;
    expect(await red.until(1)).toEqual([want]);
    expect(await blue.until(1)).toEqual([want]);
    const attached = await runInDurableObject(houseStub(), (_o, state) =>
      state.getWebSockets("lobby").map((ws) => ws.deserializeAttachment() as { user?: string }),
    );
    expect(attached).toEqual(expect.arrayContaining([{ user: RED }, { user: BLUE }]));
    red.ws.close(1000, "done");
    blue.ws.close(1000, "done");
  });

  // The client's 30 s keep-alive, answered by the platform without waking
  // the house (the orchestrator's rulings I-36/I-48). Mutant: no auto-response.
  it("answers a lobby socket's ping with pong", async () => {
    const tab = await socket(await worker("/api/stream", { token: await person(RED), headers: UPGRADE }));
    tab.ws.send("ping");
    expect(await tab.until(1)).toEqual(["pong"]);
    tab.ws.close(1000, "done");
  });

  // The table's `…/stream` refusals, for the house's (ruling I-48). Mutants:
  // either refusal dropped — without the 405 a POST meets the 426, and without
  // the 426 a plain GET reaches the socket's admission.
  it("/api/stream: a GET that is not an upgrade is 426 with the table's sentence; any other method 405 empty", async () => {
    const token = await person(RED);
    const plain = await worker("/api/stream", { token });
    expect([plain.status, plain.text]).toEqual([426, '{"error":"this host streams over WebSocket"}']);
    const posted = await worker("/api/stream", { method: "POST", token, body: "x" });
    expect([posted.status, posted.text]).toEqual([405, ""]);
  });
});

describe("the layer's log (M6.5c2, spec §2.6)", () => {
  // **A line the layer logs during a call is a line of the shell's log.** The
  // layer's `eprintln!` printed nothing on this host. Now the module records
  // each line (`crates/yokohama-worker/src/log.rs`), the reply carries it
  // (`Reply.logs`), and `readReply` logs it as one of the shell's own. A copy
  // the layer's listing cannot read is the one line a bare module can be made
  // to write: `put_copy` stores what it is handed, unread (its doc), and the
  // listing then skips it and says so. Mutants: `readReply` dropping `logs`;
  // the drain left out of `from_the_layer`; `init` handing the layer no log.
  it("a table the listing skips is one line of the shell's log", async () => {
    const mod = createModule(compiled);
    mod.init(configModule.moduleConfig(env, null));
    mod.put_copy("unreadable", '{"schema":3}');
    const { result, lines } = await logsDuring(async () =>
      readReply(mod.dispatch(RED, "GET", "/api/games", "", undefined, new Uint8Array())),
    );
    expect(result.status, result.body).toBe(200);
    expect(lines).toEqual([
      { event: "listing-skipped", id: "unreadable", why: "does not parse", error: expect.stringContaining("missing field") },
    ]);
  });

  // **A field that is not a string is refused, named** — the strings rule's
  // reason made structural: the shell parses these lines, and a string rounds
  // nothing, where this number would. The reply is a stand-in built to the
  // glue's shape, since the module writes no such line. Mutant: the check
  // dropped (the line is logged with the number rounded).
  it("a log line whose field is not a string is refused", () => {
    const fake = {
      status: 200,
      body: "",
      bumps: "[]",
      archive: undefined,
      picker: undefined,
      dropped: undefined,
      logs: '[{"event":"listing-skipped","fields":{"id":"x","seed":18446744073709551615}}]',
      free() {},
    } as unknown as Reply;
    expect(() => readReply(fake)).toThrow(/which is no line/);
  });
});

describe("every answer under /api carries no-store and nosniff (security S2a, B9's Worker half; the S1 stack review's DC-F4)", () => {
  // `_headers` never applies to a Worker-generated answer (Cloudflare's rule), so the shell sets the two headers
  // a JSON body needs on the way out — the shell's own answers and the objects' alike (decision D-3).
  it("on the shell's own answers and the house's; never on /enter or a path outside /api", async () => {
    const rows: [string, Sent][] = [
      // `/api` exactly — the path a `startsWith("/api/")` alone would miss (the wrapper's own first arm).
      ["/api", {}],
      ["/api/health", {}],
      ["/api/me", {}],
      ["/api/art", {}],
      ["/api/art", { method: "HEAD" }],
      ["/api/nothing", {}],
      ["/api/admin/tables", {}],
      ["/api/games", { method: "POST", headers: JSON_TYPE, body: new Uint8Array(BODY_LIMIT + 1).fill(97) }],
      ["/api/games/x", { method: "DELETE" }],
    ];
    for (const [path, init] of rows) {
      const res = await worker(path, init);
      expect(
        [res.headers.get("cache-control"), res.headers.get("x-content-type-options")],
        `${init.method ?? "GET"} ${path} answered ${res.status}`,
      ).toEqual(["no-store", "nosniff"]);
    }
    for (const path of ["/enter", "/t/some-table", "/assets/x.js", "/apiary"]) {
      const res = await worker(path);
      expect(res.headers.get("cache-control"), `${path} is not the API's`).toBeNull();
    }
  });

  it("on a table object's answer through the Worker, signed in", async () => {
    // The pool's person, as the identity cells mint one; a create, then the table's frame.
    const token = await person(RED);
    const created = await worker("/api/games", { method: "POST", token, headers: JSON_TYPE, body: CREATE_BODY });
    expect(created.status).toBe(201);
    const id = (JSON.parse(created.text) as { id: string }).id;
    const frame = await worker(`/api/games/${id}/table`, { token });
    expect(frame.status).toBe(200);
    expect([frame.headers.get("cache-control"), frame.headers.get("x-content-type-options")]).toEqual(["no-store", "nosniff"]);
  });
});
