import { createExecutionContext } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { Archive } from "../src/archive";
import { ADMIN_NEEDS_JSON } from "../src/admin";
import type { Env } from "../src/env";
import { House } from "../src/house";
import type { KeySource } from "../src/identity";
import { CROSS_SITE, handle } from "../src/index";
import { Table } from "../src/table";
import { callerTag } from "../src/redact";
import { RETRY_AFTER_CREATE, RETRY_AFTER_WRITE, TOO_MANY } from "../src/limits";
import type { Limiter } from "../src/limits";
import { shellConfig } from "../src/config";
import { logsDuring, RED, seedPlaying } from "./helpers";
import { personClaims, signer, type Signer } from "./tokens";

// **The Worker's door** (security S2b): what `handle` refuses before any object is reached — a cross-site
// unsafe method (B5), the operator's POST without `application/json` (B5), and in Task 2 the rate limit (C2).
// Every request goes through `handle`, as the Worker's own `fetch` does. Storage and the house are per FILE
// (pool fact 5): every cell uses ids of its own.

const ORIGIN = "https://yokohama.test";
const FOREIGN = "https://evil.example";
const JSON_TYPE = { "content-type": "application/json" };
const BOSS = "boss@t.test";
const OPS: Env = { ...env, YOKOHAMA_WEB_ADMINS: BOSS };

let key: Signer;
beforeAll(async () => {
  key = await signer("k1");
});
const at = () => ({ teamDomain: env.ACCESS_TEAM_DOMAIN!, aud: env.ACCESS_AUD!, now: Math.floor(Date.now() / 1000) });
const person = (email: string) => key.sign(personClaims(email, at()));
const keys: KeySource = async () => key.set();

interface Sent {
  method?: string;
  headers?: Record<string, string>;
  body?: string | null;
  token?: string;
  env?: Env;
}

async function worker(path: string, sent: Sent = {}): Promise<{ status: number; text: string; headers: Headers }> {
  const headers = new Headers(sent.headers);
  if (sent.token !== undefined) headers.set("Cf-Access-Jwt-Assertion", sent.token);
  const req = new Request(ORIGIN + path, { method: sent.method ?? "GET", headers, body: sent.body ?? null });
  const res = await handle(req, sent.env ?? env, createExecutionContext(), keys);
  return { status: res.status, text: await res.text(), headers: res.headers };
}

/** Which objects `f`'s requests reached: a spy on each class's `fetch`, on the PROTOTYPE (pool fact 1). */
async function reaching<T>(f: () => Promise<T>): Promise<{ result: T; reached: number; tables: number }> {
  const spies = [Table, House, Archive].map((c) => vi.spyOn(c.prototype, "fetch"));
  try {
    const result = await f();
    return { result, reached: spies.reduce((n, s) => n + s.mock.calls.length, 0), tables: spies[0]!.mock.calls.length };
  } finally {
    for (const s of spies) s.mockRestore();
  }
}

describe("B5 — an unsafe method from another site is refused before identity", () => {
  // Mutants: the `Sec-Fetch-Site` arm dropped (the no-Origin cross-site row passes); `startsWith` for the origin
  // compare (`https://yokohama.test.evil` passes); the safe-method set emptied (the GET row is refused); the check
  // placed after the operator gate (cell 1's operator rows read the gate's 403) or after identity (the key-source cell).
  it("refuses a bodyless, a form and a text/plain POST from a foreign origin, and a cross-site or same-site sender with no Origin, reaching no object", async () => {
    const id = "door-b5-foreign";
    await seedPlaying(id);
    const shapes: Array<[string, Record<string, string>, string | null]> = [
      ["no body", { Origin: FOREIGN }, null],
      ["a form", { Origin: FOREIGN, "content-type": "application/x-www-form-urlencoded" }, "force=1"],
      ["text/plain", { Origin: FOREIGN, "content-type": "text/plain" }, "{}"],
      ["JSON from a foreign origin", { Origin: FOREIGN, ...JSON_TYPE }, "{}"],
      ["a sibling host", { Origin: "https://yokohama-nas.moore.nyc" }, null],
      ["an origin that merely starts with ours", { Origin: `${ORIGIN}.evil` }, null],
      ["Origin: null", { Origin: "null" }, null],
      ["cross-site with no Origin", { "Sec-Fetch-Site": "cross-site" }, null],
      ["same-site with no Origin", { "Sec-Fetch-Site": "same-site" }, null],
    ];
    for (const [label, headers, body] of shapes) {
      const { result, reached } = await reaching(() =>
        worker(`/api/admin/tables/${id}/expire?force`, { method: "POST", headers, body, token: undefined, env: OPS }),
      );
      expect([result.status, result.text], label).toEqual([403, JSON.stringify({ error: CROSS_SITE })]);
      expect(reached, `${label}: no object reached`).toBe(0);
    }
    // Order, pinned two ways: a cross-site sender who is also nobody reads CROSS_SITE (never the object's 401), and a
    // refused sender's assertion is never verified (the key source is not asked — the next cell).
    const both = await worker(`/api/games/${id}/input`, { method: "POST", headers: { Origin: FOREIGN, ...JSON_TYPE }, body: '{"line":"1"}' });
    expect([both.status, both.text]).toEqual([403, JSON.stringify({ error: CROSS_SITE })]);
    // And the table stands.
    const table = await worker(`/api/games/${id}/table`, { token: await person(RED) });
    expect(table.status).toBe(200);
  });

  // The "before identity" pin (the M6.2 lesson, the two-defect probe): with no assertion `identify` returns NO_ONE
  // without asking the key source, so a 401 alone pins nothing — this cell asks the key source whether it was asked.
  it("never verifies the assertion of a cross-site sender: the key source is not asked", async () => {
    const id = "door-b5-keys";
    await seedPlaying(id);
    let asked = 0;
    const spy: KeySource = async () => { asked += 1; return key.set(); };
    const req = new Request(ORIGIN + `/api/games/${id}/input`, {
      method: "POST",
      headers: { Origin: FOREIGN, ...JSON_TYPE, "Cf-Access-Jwt-Assertion": await person(RED) },
      body: '{"line":"1"}',
    });
    const res = await handle(req, env, createExecutionContext(), spy);
    expect([res.status, await res.text(), asked]).toEqual([403, JSON.stringify({ error: CROSS_SITE }), 0]);
  });

  it("passes the site's own pages, bare clients and every read", async () => {
    const id = "door-b5-own";
    await seedPlaying(id);
    const own = { Origin: ORIGIN, "Sec-Fetch-Site": "same-origin", ...JSON_TYPE };
    const posted = await worker(`/api/games/${id}/input`, { method: "POST", headers: own, body: '{"line":"1"}', token: await person(RED) });
    expect(posted.status, "a page's own POST").toBe(200);
    // A chat POST answers 201 (`api.rs`'s `chat_post`; `sockets.test.ts` says the same).
    const bare = await worker(`/api/games/${id}/chat`, { method: "POST", headers: JSON_TYPE, body: '{"text":"hi"}', token: await person(RED) });
    expect(bare.status, "curl sends no Origin and no Sec-Fetch-Site").toBe(201);
    const none = await worker(`/api/games/${id}/chat`, { method: "POST", headers: { "Sec-Fetch-Site": "none", ...JSON_TYPE }, body: '{"text":"hi"}', token: await person(RED) });
    expect(none.status, "a user-initiated send is not a page's").toBe(201);
    const read = await worker("/api/health", { headers: { Origin: FOREIGN, "Sec-Fetch-Site": "cross-site" } });
    expect(read.status, "a GET is never refused here").toBe(200);
    const head = await worker("/api/health", { method: "HEAD", headers: { Origin: FOREIGN } });
    expect(head.status, "nor a HEAD").toBe(200);
  });
});

describe("B5 — the operator's POST needs application/json", () => {
  // Mutants: the essence compared whole (`application/json; charset=utf-8` refused); the gate dropped (a bare POST
  // expires); the gate placed before the operator gate (a stranger's bare POST reads this 400, not the gate's 403).
  it("refuses the expire without the type, in the layer's own words, and takes it with the type, parameters and case folded", async () => {
    const id = "door-b5-json";
    await seedPlaying(id);
    const boss = await person(BOSS);
    for (const [label, headers] of [["no type", {}], ["text/plain", { "content-type": "text/plain" }], ["a form", { "content-type": "application/x-www-form-urlencoded" }]] as Array<[string, Record<string, string>]>) {
      const { result, reached } = await reaching(() => worker(`/api/admin/tables/${id}/expire`, { method: "POST", headers, token: boss, env: OPS }));
      expect([result.status, result.text], label).toEqual([400, JSON.stringify({ error: ADMIN_NEEDS_JSON })]);
      expect(reached, `${label}: no object reached`).toBe(0);
    }
    // A stranger meets the gate first: the type is asked of an operator.
    const stranger = await worker(`/api/admin/tables/${id}/expire`, { method: "POST", token: await person(RED), env: OPS });
    expect(stranger.status).toBe(403);
    const typed = await worker(`/api/admin/tables/${id}/expire`, { method: "POST", headers: { "content-type": "Application/JSON; charset=utf-8" }, token: boss, env: OPS });
    expect(typed.status, "the essence, parameters dropped and case folded").toBe(204);
  });
});

describe("C2's name rule — a name the store refuses never makes a Table object", () => {
  it("routes an unusable segment and an unusable create id to the house, which answers the layer's refusal", async () => {
    const red = await person(RED);
    // ".." is normalised away by the URL (to /api/table) before routing: route.unit.test.ts holds it.
    for (const bad of ["a%20b", "_x", "x.table", "%C3%BC", "a".repeat(65)]) {
      const { result, tables } = await reaching(() => worker(`/api/games/${bad}/table`, { token: red }));
      expect(tables, `${bad}: no table object`).toBe(0);
      expect(result.status, bad).toBe(404);
    }
    const body = { players: 2, colors: ["Red", "Blue"], start_seat: 0, id: "x.table", seats: [{ user: RED }, { open: true }] };
    const { result, tables } = await reaching(() => worker("/api/games", { method: "POST", headers: JSON_TYPE, body: JSON.stringify(body), token: red }));
    expect(tables, "a create naming an unusable id reaches no table object").toBe(0);
    expect([result.status, result.text], "the layer's own refusal of the id (probed at the plan review: 400, its sentence, echo-bounded)").toEqual([
      400,
      JSON.stringify({ error: '"x.table" is not a usable game id' }),
    ]);
  });
});

/** A fake binding that records every key and answers as told. */
function limiter(answer: (key: string) => boolean): Limiter & { asked: string[] } {
  const asked: string[] = [];
  return { asked, limit: async ({ key }) => { asked.push(key); return { success: answer(key) }; } };
}

describe("C2 — the rate limit, before any object", () => {
  // Mutants: `limiterFor` spending the write limiter on a create; `limited` ignoring `success`; the 429 without
  // `Retry-After`; the call placed after `object.fetch` returns (a refused request still reaches the house or a table —
  // after `objectFor` alone is an equivalent mutant: it builds a stub); a GET asking the limiter.
  it("spends the create limiter on a create and the write limiter on a write, keyed by the caller's digest, and asks nothing for a read", async () => {
    const create = limiter(() => true);
    const write = limiter(() => true);
    const limited: Env = { ...env, CREATE_LIMIT: create as unknown as RateLimit, WRITE_LIMIT: write as unknown as RateLimit };
    const red = await person(RED);
    const body = { players: 2, colors: ["Red", "Blue"], start_seat: 0, id: "door-c2-keys", seats: [{ user: RED }, { open: true }] };
    expect((await worker("/api/games", { method: "POST", headers: JSON_TYPE, body: JSON.stringify(body), token: red, env: limited })).status).toBe(201);
    expect((await worker("/api/games/door-c2-keys/chat", { method: "POST", headers: JSON_TYPE, body: '{"text":"hi"}', token: red, env: limited })).status).toBe(201);
    expect((await worker("/api/games/door-c2-keys/table", { token: red, env: limited })).status).toBe(200);
    expect(create.asked).toEqual([await callerTag(RED)]);
    expect(write.asked).toEqual([await callerTag(RED)]);
  });

  it("answers a refused request 429 with the kind's own Retry-After and the sentence, under the API's two headers, reaching no object", async () => {
    const limited: Env = { ...env, CREATE_LIMIT: limiter(() => false) as unknown as RateLimit, WRITE_LIMIT: limiter(() => false) as unknown as RateLimit };
    const red = await person(RED);
    const id = "door-c2-refused";
    await seedPlaying(id);
    const { result, reached } = await reaching(() => worker(`/api/games/${id}/input`, { method: "POST", headers: JSON_TYPE, body: '{"line":"1"}', token: red, env: limited }));
    expect([result.status, result.text]).toEqual([429, JSON.stringify({ error: TOO_MANY })]);
    expect(result.headers.get("Retry-After"), "a write's window").toBe(RETRY_AFTER_WRITE);
    expect(result.headers.get("Cache-Control")).toBe("no-store");
    expect(result.headers.get("X-Content-Type-Options")).toBe("nosniff");
    expect(reached, "no object reached").toBe(0);
    const create = await reaching(() => worker("/api/games", { method: "POST", headers: JSON_TYPE, body: '{"players":2,"colors":["Red","Blue"],"start_seat":0,"seats":[{"user":"red@t.test"},{"open":true}]}', token: red, env: limited }));
    expect([create.result.status, create.reached]).toEqual([429, 0]);
    expect(create.result.headers.get("Retry-After"), "a create's window").toBe(RETRY_AFTER_CREATE);
  });

  // AD-6 (b): a limiter that throws is logged and the request proceeds — the limit bounds cost, never access.
  it("logs a limiter that throws once and lets the request through", async () => {
    const broken = { asked: [] as string[], limit: async () => { throw new Error("binding unavailable"); } };
    const limited: Env = { ...env, WRITE_LIMIT: broken as unknown as RateLimit };
    const id = "door-c2-broken";
    await seedPlaying(id);
    const { result, lines } = await logsDuring(() => reaching(async () => worker(`/api/games/${id}/input`, { method: "POST", headers: JSON_TYPE, body: '{"line":"1"}', token: await person(RED), env: limited })));
    expect([result.result.status, result.reached], "the request reached its object").toEqual([200, 1]);
    expect(lines.filter((l) => (l as { event: string }).event === "limiter-failed"), "exactly one line").toHaveLength(1);
  });

  it("keys a request with no caller by the connecting IP, and by `unknown` without one", async () => {
    const write = limiter(() => true);
    const limited: Env = { ...env, WRITE_LIMIT: write as unknown as RateLimit };
    await worker("/api/chat", { method: "POST", headers: { ...JSON_TYPE, "cf-connecting-ip": "203.0.113.9" }, body: '{"text":"hi"}', env: limited });
    await worker("/api/chat", { method: "POST", headers: JSON_TYPE, body: '{"text":"hi"}', env: limited });
    expect(write.asked).toEqual(["ip:203.0.113.9", "ip:unknown"]);
  });

  it("binds both limiters in the pool, a lenient one answers success, and shellConfig hands the bindings through by identity", async () => {
    expect(typeof env.CREATE_LIMIT!.limit).toBe("function");
    expect(await env.WRITE_LIMIT!.limit({ key: "door-c2-smoke" })).toEqual({ success: true });
    const c = shellConfig(env);
    if ("error" in c) throw new Error(c.error); // the narrowing `tsc` needs: `shellConfig` answers `ShellConfig | { error }`
    expect(c.limits.create).toBe(env.CREATE_LIMIT);
    expect(c.limits.write).toBe(env.WRITE_LIMIT);
  });
});
