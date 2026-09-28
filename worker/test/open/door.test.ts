import { createExecutionContext, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import compiled from "../../gen/yokohama_worker_bg.wasm";
import { createModule } from "../../gen/factory.js";
import { Archive } from "../../src/archive";
import * as configModule from "../../src/config";
import type { Env } from "../../src/env";
import { House } from "../../src/house";
import type { KeySource } from "../../src/identity";
import { handle } from "../../src/index";
import { readReply } from "../../src/module";
import { HOUSE_NAME } from "../../src/slot";
import { Table } from "../../src/table";
import { settle, tableStub } from "../helpers";

// **The open door, end to end** (M6.5c2, spec §2.1; R-6.5c2.3, R-6.5c2.5) —
// the pool's `open` project, over `wrangler.open.test.jsonc`. The Worker and
// every object read `YOKOHAMA_WEB_IDENTITY=open` from their OWN configuration,
// which is why this is a project and not a cell's env override (review
// R2.2-M2). Every request goes through `handle`, as the Worker's own `fetch`
// does, so what reaches an object is what the door forwards.
//
// Storage, the house and the bucket are per FILE and per project (pool fact
// 5): every cell uses ids and names of its own.

const ORIGIN = "https://yokohama.test";
const JSON_TYPE = { "content-type": "application/json" };
const UPGRADE = { Upgrade: "websocket" };

/** How long a cell waits for a message it EXPECTS (the orchestrator's ruling
 *  R-10: nothing waits out a timer to show that something did not arrive). */
const WAIT_MS = 2_000;

/** A key source that fails the request that asks it: under the open door the
 *  Worker reads no Access assertion, so no key is ever wanted. */
const NO_KEYS: KeySource = async () => {
  throw new Error("the open door asked for Access's keys");
};

interface Sent {
  method?: string;
  headers?: Record<string, string>;
  body?: BodyInit | null;
  /** The claimed name, as `X-Yokohama-Claimed-User`: a fetch's carrier. */
  claim?: string;
  env?: Env;
  keys?: KeySource;
}

interface Answer {
  status: number;
  text: string;
  webSocket: WebSocket | null;
}

/** One request through the Worker's `handle`, answered whole. An upgrade's
 *  socket is handed back unread. */
async function through(path: string, sent: Sent = {}): Promise<Answer> {
  const headers = new Headers(sent.headers);
  if (sent.claim !== undefined) headers.set("X-Yokohama-Claimed-User", sent.claim);
  const req = new Request(ORIGIN + path, { method: sent.method ?? "GET", headers, body: sent.body ?? null });
  const res = await handle(req, sent.env ?? env, createExecutionContext(), sent.keys ?? NO_KEYS);
  const webSocket = res.webSocket;
  return { status: res.status, text: webSocket ? "" : await res.text(), webSocket };
}

/** `path` with `name` as the upgrade's `claimed` parameter, encoded as the
 *  client encodes it (`URLSearchParams`, `web/src/ws.ts`). */
function claimedAt(path: string, name: string): string {
  const url = new URL(path, ORIGIN);
  url.searchParams.set("claimed", name);
  return url.pathname + url.search;
}

/** A socket through the Worker, accepted, with every message kept. */
async function socket(res: Answer): Promise<{ ws: WebSocket; until(n: number): Promise<string[]> }> {
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
    until: (n) =>
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

/** Which objects `f`'s requests reached, and what each was handed: a spy on
 *  each class's `fetch`, on the PROTOTYPE (pool fact 1), calling through. */
async function reaching<T>(f: () => Promise<T>): Promise<{ result: T; table: Request[]; house: Request[]; archive: Request[] }> {
  const table = vi.spyOn(Table.prototype, "fetch");
  const house = vi.spyOn(House.prototype, "fetch");
  const archive = vi.spyOn(Archive.prototype, "fetch");
  try {
    const result = await f();
    return {
      result,
      table: table.mock.calls.map(([r]) => r),
      house: house.mock.calls.map(([r]) => r),
      archive: archive.mock.calls.map(([r]) => r),
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

/** The layer's own answer to no caller at this door: a fresh module in this
 *  project's configuration, asked `identity` with none. */
function noCaller(): string {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  return readReply(mod.identity(undefined)).body;
}

describe("the open door (M6.5c2, spec §2.1)", () => {
  // Mutants: the claim not read (user null); the claim forwarded as
  // `X-Yokohama-User`, which an open object does not read (user null, and the
  // headers show it); an assertion read at the open door (a key asked for);
  // a forged `x-yokohama-*` header let through.
  it("a claim by the header on a fetch is the caller, with no assertion read; the objects are handed it under its own name", async () => {
    const asked: boolean[] = [];
    const keys: KeySource = async (refresh) => {
      asked.push(refresh);
      throw new Error("the open door asked for Access's keys");
    };
    const forged = { "x-yokohama-user": "evil@x.test", "x-yokohama-operator": "evil@x.test", "x-yokohama-future": "evil@x.test" };
    const me = await reaching(() =>
      through("/api/me", { claim: "  Ben ", headers: { ...forged, "Cf-Access-Jwt-Assertion": "a.b.c" }, keys }),
    );
    expect(me.result.text).toBe('{"mode":"open","user":"ben"}');
    expect(me.house.map(yokohamaHeaders), "the claim under its own name, and nothing forged").toEqual([
      { "x-yokohama-claimed-user": "ben" },
    ]);
    expect(asked, "no key asked for: the open door reads no assertion").toEqual([]);
    expect((await through("/api/me")).text, "no claim: no one").toBe('{"mode":"open","user":null}');
    expect((JSON.parse((await through("/api/health")).text) as { identity: string }).identity).toBe("open");
  });

  // **(Review Focus 1)** An address with a `+` and a name with a space, carried
  // on the upgrade as the client carries them: `URLSearchParams` encodes the
  // parameter (`web/src/ws.ts`, whose own cell pins the client's half), and
  // the Worker reads it back decoded. The socket's caller is exactly the
  // fetch caller: at a PLAYING table each socket's first frame IS that
  // caller's own `…/events` reading — seat 0 is to move and seat 1 is not, so
  // the two readings differ and each socket is told apart by the frame it is
  // sent. Mutants: the parameter read raw off `url.search` (`a%2Bb%40x.test`,
  // which is no caller); decoded with `decodeURIComponent` (`ben+castle`, which
  // the key grammar refuses); a `+` turned into a space (`a b@x.test`, no
  // caller); the parameter never read (the upgrade answered 401).
  it("(Review Focus 1) a + in an address and a space in a name reach a table's socket exactly as the fetch caller", async () => {
    const id = "rf1-open";
    const [plus, spaced] = ["a+b@x.test", "ben castle"];
    const create = {
      id,
      players: 2,
      colors: ["Red", "Blue"],
      start_seat: 0,
      seed: 20260831,
      seats: [{ user: plus }, { user: spaced }],
    };
    const created = await through("/api/games", { method: "POST", claim: plus, headers: JSON_TYPE, body: JSON.stringify(create) });
    expect(created.status, created.text).toBe(201);
    for (const [who, seat] of [
      [plus, 0],
      [spaced, 1],
    ] as const) {
      const kept = await through(`/api/games/${id}/keep`, { method: "POST", claim: who, headers: JSON_TYPE, body: JSON.stringify({ seat, card: 1 }) });
      expect(kept.status, `${who} keeps: ${kept.text}`).toBe(200);
    }
    const reading = async (who: string) => {
      const res = await through(`/api/games/${id}/events`, { claim: who });
      expect(res.status, `${who}: ${res.text}`).toBe(200);
      return res.text;
    };
    const readings = { [plus]: await reading(plus), [spaced]: await reading(spaced) };
    expect(readings[plus], "premise: seat 0 is to move and seat 1 is not, so the two readings differ").not.toBe(readings[spaced]);
    for (const [who, query] of [
      [plus, "?claimed=a%2Bb%40x.test"],
      [spaced, "?claimed=ben+castle"],
    ] as const) {
      const path = claimedAt(`/api/games/${id}/stream`, who);
      expect(path.endsWith(query), `premise: the client's own encoding — ${path}`).toBe(true);
      const tab = await socket(await through(path, { headers: UPGRADE }));
      expect(await tab.until(1), who).toEqual([`{"event":"signal","data":${readings[who]}}`]);
      tab.ws.close(1000, "done");
    }
    await settle(tableStub(id));
  });

  // Review R2.2-m3: the parameter is percent-DECODED, so it can hold any
  // character — and `headers.set` THROWS on a value a header cannot carry,
  // which, before any `try` in `handle`, would be the platform's plain 500.
  // Judged RAW first (`claimedIdentity`), such a claim is no caller, and the
  // house answers as the layer answers no one. Mutants: the claim forwarded
  // unjudged (the call rejects, or an identity header reaches the house);
  // normalized before it is judged (a Kelvin sign's `\u212Aevin` becomes
  // the caller `kevin`).
  it("a claimed parameter no header can carry is no caller — the layer's own answer, never a platform 500", async () => {
    const body = noCaller();
    // A Kelvin sign, a no-break space, a carriage return, a NUL, an e-acute.
    for (const raw of ["\u212Aevin", "\u00A0ben", "ben\r", "ben\u0000", "ren\u00E9"]) {
      const res = await reaching(() => through(claimedAt("/api/stream", raw), { headers: UPGRADE }));
      expect([res.result.status, res.result.webSocket, res.result.text], JSON.stringify(raw)).toEqual([401, null, body]);
      expect(res.house.map(yokohamaHeaders), `${JSON.stringify(raw)}: no identity header`).toEqual([{}]);
    }
    // The anti-vacuity partner: a name a header CAN carry is admitted, as itself.
    const tab = await socket(await through(claimedAt("/api/stream", "kevin"), { headers: UPGRADE }));
    const attached = await runInDurableObject(env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME)), (_o, state) =>
      state.getWebSockets("lobby").map((ws) => ws.deserializeAttachment() as { user?: string }),
    );
    expect(attached).toContainEqual({ user: "kevin" });
    tab.ws.close(1000, "done");
  });

  // Spec §2.1: "on upgrades only". ONE carrier per request kind, as R-1.2 has
  // each mode read one header. Mutants: the parameter read on a fetch; the
  // header read on an upgrade.
  it("the claim rides ONE carrier per request: the header on a fetch, the parameter on an upgrade", async () => {
    expect((await through("/api/me", { claim: "ben" })).text).toBe('{"mode":"open","user":"ben"}');
    const tab = await socket(await through(claimedAt("/api/stream", "ben"), { headers: UPGRADE }));
    tab.ws.close(1000, "done");
    expect((await through("/api/me?claimed=ben")).text, "a fetch's parameter is no claim").toBe('{"mode":"open","user":null}');
    const refused = await through("/api/stream", { claim: "ben", headers: UPGRADE });
    expect([refused.status, refused.webSocket], "an upgrade's header is no claim").toEqual([401, null]);
  });
});
