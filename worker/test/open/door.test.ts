import { createExecutionContext, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import compiled from "../../gen/yokohama_worker_bg.wasm";
import { createModule } from "../../gen/factory.js";
import { Archive, ARCHIVE_PREFIX } from "../../src/archive";
import * as configModule from "../../src/config";
import type { Env } from "../../src/env";
import { House } from "../../src/house";
import type { KeySource } from "../../src/identity";
import { handle } from "../../src/index";
import type { Limiter } from "../../src/limits";
import { callerDigest, callerTag } from "../../src/redact";
import { readReply } from "../../src/module";
import { HOUSE_NAME } from "../../src/slot";
import { Table } from "../../src/table";
import { bucketText, logsDuring, replaceOnce, segmentOf, settle, tableStub } from "../helpers";
import { ARCHIVE_WALK } from "../walk";

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

/** The layer's 401 for an address claimed under the open door (security S2b,
 *  finding C1; `identity::ADDRESS_NOT_A_NAME`), pinned at its words. */
const ADDRESS_NOT_A_NAME = "under the open door a name is a name — an address is not accepted here: choose a plain name";

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
  return identityOf(undefined).body;
}

/** A fresh module in this project's configuration, asked `identity` for
 *  `claim`: the layer's own status and body for that caller. */
function identityOf(claim: string | undefined): { status: number; body: string } {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  const { status, body } = readReply(mod.identity(claim));
  return { status, body };
}

/** A fake binding that records every key and answers as told — this
 *  project's own copy of `test/door.test.ts`'s: the pool's projects share no
 *  file (security S2b, C2). */
function limiter(answer: (key: string) => boolean): Limiter & { asked: string[] } {
  const asked: string[] = [];
  return { asked, limit: async ({ key }) => { asked.push(key); return { success: answer(key) }; } };
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

  // **(Review Focus 1)** A name with a space and an address with a `+`, carried
  // on the upgrade as the client carries them: `URLSearchParams` encodes the
  // parameter (`web/src/ws.ts`, whose own cell pins the client's half), and
  // the Worker reads it back decoded. The name's socket's caller is exactly
  // the fetch caller: at a PLAYING table each socket's first frame IS that
  // caller's own `…/events` reading — seat 0 is to move and seat 1 is not, so
  // the two readings differ and the socket is told apart by the frame it is
  // sent. The address is refused at the upgrade, unupgraded, in the layer's
  // own sentence (security S2b, C1: under the open door an address is no
  // caller), so every misreading of it is told apart by the sentence it meets.
  // Mutants: the parameter read raw off `url.search` (`a%2Bb%40x.test` and
  // `ben+castle`, which the key grammar refuses); decoded with
  // `decodeURIComponent` (`ben+castle`); a `+` turned into a space
  // (`a b@x.test`, the grammar's refusal); the parameter never read (the
  // absent caller's 401).
  it("(Review Focus 1) a space in a name reaches a table's socket exactly as the fetch caller; a + in an address is refused at the upgrade", async () => {
    const id = "rf1-open";
    const [partner, spaced, plus] = ["red", "ben castle", "a+b@x.test"];
    const create = {
      id,
      players: 2,
      colors: ["Red", "Blue"],
      start_seat: 0,
      seed: 20260831,
      seats: [{ user: partner }, { user: spaced }],
    };
    const created = await through("/api/games", { method: "POST", claim: partner, headers: JSON_TYPE, body: JSON.stringify(create) });
    expect(created.status, created.text).toBe(201);
    for (const [who, seat] of [
      [partner, 0],
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
    const readings = { [partner]: await reading(partner), [spaced]: await reading(spaced) };
    expect(readings[partner], "premise: seat 0 is to move and seat 1 is not, so the two readings differ").not.toBe(readings[spaced]);
    const path = claimedAt(`/api/games/${id}/stream`, spaced);
    expect(path.endsWith("?claimed=ben+castle"), `premise: the client's own encoding — ${path}`).toBe(true);
    const tab = await socket(await through(path, { headers: UPGRADE }));
    expect(await tab.until(1), spaced).toEqual([`{"event":"signal","data":${readings[spaced]}}`]);
    tab.ws.close(1000, "done");
    await settle(tableStub(id));

    // The address: the layer's refusal, probed off a fresh module and pinned at its words.
    const refusal = identityOf(plus);
    expect(refusal, "premise: the layer refuses an address at the open door").toEqual({ status: 401, body: JSON.stringify({ error: ADDRESS_NOT_A_NAME }) });
    const refusedAt = claimedAt("/api/stream", plus);
    expect(refusedAt.endsWith("?claimed=a%2Bb%40x.test"), `premise: the client's own encoding — ${refusedAt}`).toBe(true);
    const refused = await through(refusedAt, { headers: UPGRADE });
    expect([refused.status, refused.webSocket, refused.text], "an address's upgrade, unupgraded").toEqual([401, null, refusal.body]);
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

  // The S2 stack review (lens 1, Minor 2): once the Worker has read the claim off an upgrade's URL, the header it sets
  // carries it, and the object's own request names no `claimed` — `forwardedCaller` reads only the header, and the
  // URL is what an invocation record keeps. Only `claimed` is dropped: another parameter rides through. Mutant: the
  // parameter forwarded (the object's URL ends `claimed=zqxjv9+maple`).
  it("forwards an upgrade to its object without the claimed parameter, the claim in the header alone", async () => {
    const res = await reaching(() => through(claimedAt("/api/stream?other=1", "zqxjv9 maple"), { headers: UPGRADE }));
    const tab = await socket(res.result);
    expect(res.house.length, "the house alone").toBe(1);
    const reached = new URL(res.house[0]!.url);
    expect([reached.pathname, reached.search], "no claimed parameter; the other one kept").toEqual(["/api/stream", "?other=1"]);
    expect(yokohamaHeaders(res.house[0]!), "the claim, in the header").toEqual({ "x-yokohama-claimed-user": "zqxjv9 maple" });
    tab.ws.close(1000, "done");
  });
});

describe("what the open door refuses (M6.5c2, spec §2.1)", () => {
  // Under `open` no one is an operator: the gate answers `off`, and the
  // operator routes are unrouted `/api/*` — the house's, which answers the
  // layer's own 404 — whatever name is claimed, with no operator header
  // forwarded. (This configuration sets no list, which `shellConfig` refuses
  // beside `open`, so the gate's own reading of the door with a list set is
  // `admin.unit.test.ts`'s cell.) Mutants: the gate answering `refused` under
  // `open` (every claim meets the 403); a claim read as an operator.
  it("the operator routes are the layer's 404 whatever name is claimed, ?force among them", async () => {
    for (const claim of ["boss@t.test", "ben", undefined]) {
      const res = await reaching(() => through("/api/admin/tables", { claim }));
      expect([res.result.status, res.result.text], String(claim)).toEqual([404, '{"error":"no such endpoint: /api/admin/tables"}']);
      expect([res.house.length, res.table.length, res.archive.length], `${String(claim)}: the house alone`).toEqual([1, 0, 0]);
      expect(res.house.map(yokohamaHeaders), `${String(claim)}: the claim, and no operator`).toEqual([
        claim === undefined ? {} : { "x-yokohama-claimed-user": claim },
      ]);
    }
    const force = await reaching(() => through("/api/admin/tables/some-table/expire?force", { method: "POST", claim: "boss@t.test" }));
    expect([force.result.status, force.result.text]).toEqual([404, '{"error":"no such endpoint: /api/admin/tables/some-table/expire"}']);
    expect([force.house.length, force.table.length], "the house alone, never the table the path names").toEqual([1, 0]);
  });

  // M6.5c2 cycle D, straw poll row 8 (R-C10 fixed): a claim a header can
  // carry and the key grammar refuses — `..`, or a name too long for it —
  // names an Archive object whose `init` refuses it as that object's caller.
  // That was the object's 500; the NAS refuses the same request with the
  // identity 401, and now so does the Worker, in the layer's own body — the
  // one its `identity` writes for that caller, which this cell asks a fresh
  // module for. The anti-vacuity half: a claim the grammar takes files its
  // save (the cell below does), so the 401 is about the claim. Mutant: the
  // object's init refusal answered 500 again (`callerRefused` never asked).
  it("a claim the key grammar refuses is the layer's 401 at the archive, as on the NAS", async () => {
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    // `..`, and 40 letters: over the key grammar's 32 and under the 254 bytes
    // past which the Worker hands the house the request instead.
    for (const claim of ["..", "x".repeat(40)]) {
      const layer = identityOf(claim);
      expect(layer.status, `premise: the layer refuses ${JSON.stringify(claim.slice(0, 8))} as a caller`).toBe(401);
      const res = await reaching(() => through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save }));
      expect([res.result.status, res.result.text], claim.slice(0, 8)).toEqual([401, layer.body]);
      expect([res.archive.length, res.house.length], "the claim's own archive object answered it").toEqual([1, 0]);
    }
  });

  // Security S2b, D23 (Task 7's concern 1, promoted to the after review's wave): `init`'s refusal of a claim the key
  // grammar refuses repeats the claim, echoed and quoted, and the `init-refused` line persists — so the line names
  // the claim by its digest in the quoted span's place, and no persisted line spells it. Mutant: the refusal logged
  // whole (`error: e`).
  it("logs a claim the key grammar refuses by its digest, and no line spells the claim", async () => {
    const claim = "a@b@c";
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    const { result, lines } = await logsDuring(() => through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save }));
    expect(result.status, result.text).toBe(401);
    expect(JSON.stringify(lines), "no line spells the claim").not.toContain(claim);
    const digest = await callerDigest(claim);
    const refused = lines.filter((l) => (l as { event?: unknown }).event === "init-refused") as Array<{ id: string; error: string }>;
    expect(refused.map((l) => l.id), "one init-refused line, naming the object by the digest").toEqual([ARCHIVE_PREFIX + digest]);
    expect(refused[0]!.error, "the refusal names the claim by its digest").toContain(`"${digest}"`);
  });

  // Security S2c, R-4 (S2b's re-review parked it): `claimRedacted` rewrites the quoted span that IS the claim and no
  // other, so a refusal whose sentence quotes a VALUE that is not the claim (a configuration variable) is logged
  // whole. Drive the archive object as the cell above does, with the module's configuration refused as
  // `table.test.ts`'s `init-module` arm refuses it. Mutant: the first quoted span rewritten whatever it holds.
  it("rewrites only the quoted span that IS the claim, and leaves a configuration refusal's value as it is", async () => {
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    const bad = JSON.stringify({ identity: "open", archive: "off", caller: null, lobby_chat: "maybe", chat_keep: null, chat_text: null });
    const spy = vi.spyOn(configModule, "moduleConfig").mockReturnValue(bad);
    try {
      // **Claims no other cell uses**: the Archive object caches the module it built (`this.instance ??=`), so the
      // refused configuration stays in THIS name's object for the rest of the file. `may` is a prefix of the value
      // the sentence quotes and `maybe-x` is prefixed BY it: a prefix test in either direction rewrites the value.
      for (const claim of ["cfg refused", "may", "maybe-x"]) {
        const { lines } = await logsDuring(() => through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save }));
        const digest = await callerDigest(claim);
        const refused = lines.filter((l) => (l as { event?: unknown }).event === "init-refused") as Array<{ id: string; error: string }>;
        expect(refused.map((l) => l.id), claim).toEqual([ARCHIVE_PREFIX + digest]);
        expect(refused[0]!.error, `${claim}: the value is the configuration's, kept verbatim`).toBe('YOKOHAMA_WEB_LOBBY_CHAT must be on or off \u2014 got "maybe"');
        expect(refused[0]!.error, `${claim}: and no digest stands in its place`).not.toContain(digest);
      }
    } finally {
      spy.mockRestore();
    }
  });

  // Security S2c, R-4: equality with the raw claim alone would let a refused claim the echo CUT (over 64 characters)
  // or ESCAPED (a quote, a backslash) into the persisted line, so the span is compared as the echo wrote it, read
  // back. Mutants: `claimRedacted` back to `sentence.includes(`"${claim}"`)`; `echoed`'s cut at `>= 64` (the
  // 64-character row, the boundary the echo does NOT cut); the `t` entry dropped from `unescaped`'s map (the tab row).
  // Each row names a part of the claim its spelled echo would keep, so "not spelled" can fail for every row.
  it("logs a refused claim the echo cut or escaped by its digest too", async () => {
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    const rows: Array<[claim: string, spelled: string]> = [
      ["a@b@" + "x".repeat(80), "a@b@xx"],
      ["a@b@" + "x".repeat(60), "a@b@xx"],
      ['a@b@"quoted', "quoted"],
      ["a@b@back\\slash", "a@b@ba"],
      ["a@b@x\ty", "a@b@x"],
    ];
    for (const [claim, spelled] of rows) {
      const { lines } = await logsDuring(() => through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save }));
      const digest = await callerDigest(claim);
      const refused = lines.filter((l) => (l as { event?: unknown }).event === "init-refused") as Array<{ error: string }>;
      expect(refused, claim.slice(0, 12)).toHaveLength(1);
      expect(refused[0]!.error, "the claim is named by its digest").toContain(`"${digest}"`);
      expect(refused[0]!.error, "and not spelled").not.toContain(spelled);
    }
  });

  // **An address is no caller under the open door** (security S2b, finding
  // C1; decision D-2): the module runs the production key rule, `NamesOnly`,
  // so an address claimed here meets the layer's one sentence wherever it
  // knocks — `/api/me` answers no one, a create and the archive answer the
  // layer's 401 (the archive's from the claim's own Archive object, whose
  // `init` takes the address as its caller and whose `dispatch` refuses it:
  // `identity_of`, not `callerRefused`), and an upgrade is refused
  // unupgraded — while a plain name keeps every one of those doors. A name's
  // save files at the bucket's TOP root. Mutant: the module's `resolve`
  // passing `Diagnostic` (M42b: every door opens to the address).
  it("an address claimed at the open door is no caller at /api/me, a create, the archive or an upgrade; a name keeps every door", async () => {
    const [address, name] = ["red@t.test", "ben"];
    const refusal = JSON.stringify({ error: ADDRESS_NOT_A_NAME });
    expect(identityOf(address), "premise: the layer's own refusal").toEqual({ status: 401, body: refusal });
    // `/api/me` never refuses: it answers no one.
    expect((await through("/api/me", { claim: address })).text).toBe('{"mode":"open","user":null}');
    expect((await through("/api/me", { claim: name })).text).toBe('{"mode":"open","user":"ben"}');
    // A create.
    const create = (user: string, id: string) => JSON.stringify({ id, players: 2, colors: ["Red", "Blue"], start_seat: 0, seats: [{ user }, { open: true }] });
    try {
      const addressed = await through("/api/games", { method: "POST", claim: address, headers: JSON_TYPE, body: create(address, "c1-addressed") });
      expect([addressed.status, addressed.text], "an address's create").toEqual([401, refusal]);
      const named = await through("/api/games", { method: "POST", claim: name, headers: JSON_TYPE, body: create(name, "c1-named") });
      expect(named.status, named.text).toBe(201);
    } finally {
      await settle(tableStub("c1-addressed"));
      await settle(tableStub("c1-named"));
    }
    // The archive: the claim's own Archive object answers, and a name's save lands at the top root.
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    const archived = await reaching(() => through("/api/archive", { method: "POST", claim: address, headers: JSON_TYPE, body: save }));
    expect([archived.result.status, archived.result.text], "an address's save").toEqual([401, refusal]);
    expect([archived.archive.length, archived.house.length], "the claim's own archive object answered it").toEqual([1, 0]);
    const path = `browser/${await segmentOf(name)}/rev7/over/wire.json`;
    const saved = await through("/api/archive", { method: "POST", claim: name, headers: JSON_TYPE, body: save });
    expect([saved.status, (JSON.parse(saved.text) as { path: string }).path], saved.text).toEqual([201, path]);
    expect(await bucketText(path), `the name's save at ${path}`).not.toBeNull();
    expect(await bucketText(`test/${path}`), "nothing under test/").toBeNull();
    // An upgrade.
    const refused = await through(claimedAt("/api/stream", address), { headers: UPGRADE });
    expect([refused.status, refused.webSocket, refused.text], "an address's upgrade, unupgraded").toEqual([401, null, refusal]);
    const tab = await socket(await through(claimedAt("/api/stream", name), { headers: UPGRADE }));
    tab.ws.close(1000, "done");
  });

  // The anonymous archive is the `worker` door's (the auth cycle, Task 5; design §4.4: "Under `open` nothing changes
  // (a claim is the caller)"): a post with no claim, a device header and a body naming a safe id is the house's answer
  // exactly as without the header, and no archive object is made for it. Mutant: the anonymous caller formed under
  // either door (an `archive:anon-…` object answers instead of the house); the device forwarded to the house.
  it("a post with no claim is the house's answer at the archive, device header or not: no anonymous caller here", async () => {
    const { record, table } = ARCHIVE_WALK[0]!.body as { record: string; table: string };
    const body = JSON.stringify({ record, table: replaceOnce(table, '"id": "wire"', '"id": "local-0f0f0f0f0f0f0f0f"') });
    const bare = await reaching(() => through("/api/archive", { method: "POST", headers: JSON_TYPE, body }));
    const device = await reaching(() =>
      through("/api/archive", { method: "POST", headers: { ...JSON_TYPE, "X-Yokohama-Device": "0123456789abcdef" }, body }),
    );
    expect(bare.result.status, "premise: no claim is the layer's 401").toBe(401);
    expect([device.result.status, device.result.text], "the same answer").toEqual([bare.result.status, bare.result.text]);
    expect([device.house.length, device.archive.length], "the house alone").toEqual([1, 0]);
    expect(device.house.map(yokohamaHeaders), "handed no device: the header is the archive object's alone").toEqual([{}]);
  });

  // Review R2.2-m4: a plain name ending in `.test` is legal under the key
  // grammar, and the `test/` prefix EXPIRES after 30 days (R-6.5c2.12): an
  // open-door player named `a.test` keeps their saves at the bucket's top, at
  // both of the archive's roots — a server game's creator's, and a browser
  // save's caller's. Mutant: the test root read off `.test` alone.
  it("a plain name ending in .test files its saves outside test/", async () => {
    const other = (root: string) => (root === "" ? "test/" : "");
    // A server game's abandon: its creator's root.
    for (const [creator, id, root] of [["a.test", "plain-abandon", ""]] as const) {
      const create = { id, players: 2, colors: ["Red", "Blue"], start_seat: 0, seed: 20260831, seats: [{ user: creator }, { user: "joiner" }] };
      try {
        const created = await through("/api/games", { method: "POST", claim: creator, headers: JSON_TYPE, body: JSON.stringify(create) });
        expect(created.status, created.text).toBe(201);
        for (const [who, seat] of [
          [creator, 0],
          ["joiner", 1],
        ] as const) {
          const kept = await through(`/api/games/${id}/keep`, { method: "POST", claim: who, headers: JSON_TYPE, body: JSON.stringify({ seat, card: 1 }) });
          expect(kept.status, `${who} keeps: ${kept.text}`).toBe(200);
        }
        const moved = await through(`/api/games/${id}/input`, { method: "POST", claim: creator, headers: JSON_TYPE, body: '{"line":"1"}' });
        expect([moved.status, (JSON.parse(moved.text) as { events: number }).events], "premise: one event, so the abandon archives").toEqual([200, 1]);
        const abandoned = await through(`/api/games/${id}`, { method: "DELETE", claim: creator });
        expect(abandoned.status, abandoned.text).toBe(204);
      } finally {
        await settle(tableStub(id));
      }
      const key = `rev7/abandoned/${id}.json`;
      expect(await bucketText(`${root}${key}`), `${creator}: the abandon at ${root}${key}`).not.toBeNull();
      expect(await bucketText(`${other(root)}${key}`), `${creator}: nothing at ${other(root)}${key}`).toBeNull();
    }
    // A browser's save: its Archive object's caller's root.
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    for (const [claim, root] of [["a.test", ""]] as const) {
      const path = `browser/${await segmentOf(claim)}/rev7/over/wire.json`;
      const res = await through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save });
      expect([res.status, (JSON.parse(res.text) as { path: string }).path], claim).toEqual([201, path]);
      expect(await bucketText(`${root}${path}`), `${claim}: the save at ${root}${path}`).not.toBeNull();
      expect(await bucketText(`${other(root)}${path}`), `${claim}: nothing at ${other(root)}${path}`).toBeNull();
    }
  });

  it("keys an open-door create by the connecting IP alone, and a write by IP and the claim's digest", async () => {
    const create = limiter(() => true);
    const write = limiter(() => true);
    const limited: Env = { ...env, CREATE_LIMIT: create as unknown as RateLimit, WRITE_LIMIT: write as unknown as RateLimit };
    const ip = { "cf-connecting-ip": "203.0.113.9" };
    const body = { players: 2, colors: ["Red", "Blue"], start_seat: 0, id: "open-c2-keys", seats: [{ user: "ben" }, { open: true }] };
    expect((await through("/api/games", { method: "POST", claim: "ben", headers: { ...JSON_TYPE, ...ip }, body: JSON.stringify(body), env: limited })).status).toBe(201);
    expect((await through("/api/games/open-c2-keys/chat", { method: "POST", claim: "ben", headers: { ...JSON_TYPE, ...ip }, body: '{"text":"hi"}', env: limited })).status).toBe(201);
    expect(create.asked).toEqual(["ip:203.0.113.9"]);
    expect(write.asked).toEqual([`ip:203.0.113.9|${await callerTag("ben")}`]);
  });
});

describe("one name, one Archive object (security S2b, AD-3)", () => {
  // Two spellings of one open-door name — `race y` and `race   y` — are ONE
  // key to the layer (`typed_form` collapses each run of spaces), and the
  // shell collapses them too (`claimedIdentity`), so they name ONE Archive
  // object: `blockConcurrencyWhile` puts the two saves in a line and the
  // second counts the first. In a share one record short of full, one save
  // lands and the other is refused full. Two objects counted one share at
  // once and both landed — `[201, 201]`, 257 records in a 256-record share
  // (measured before the collapse). That row rests on R2's listing timing, so
  // without the collapse it is red only about half the time (measured); the
  // one-object row — which Archive objects the two saves reached — is red
  // every run. Mutant: `claimedIdentity` keeping space runs (M44c).
  it("two spellings of one name, saved at once into a share one record short of full, land one save and refuse the other", async () => {
    const prefix = `browser/${await segmentOf("race y")}/`;
    for (let i = 0; i < 255; i += 1) {
      await env.ARCHIVE!.put(`${prefix}rev7/over/g${i}.json`, "{}");
      await env.ARCHIVE!.put(`${prefix}rev7/over/g${i}.table.json`, "{}");
    }
    const { record, table } = ARCHIVE_WALK[0]!.body as { record: string; table: string };
    const first = JSON.stringify({ record, table });
    const second = JSON.stringify({ record, table: replaceOnce(table, '"id": "wire"', '"id": "wire-2"') });
    const fetched = vi.spyOn(Archive.prototype, "fetch");
    let objects: (string | undefined)[] = [];
    const [a, b] = await Promise.all([
      through("/api/archive", { method: "POST", claim: "race y", headers: JSON_TYPE, body: first }),
      through("/api/archive", { method: "POST", claim: "race   y", headers: JSON_TYPE, body: second }),
    ]).finally(() => {
      // Read before `mockRestore`, which clears what the spy recorded.
      objects = fetched.mock.contexts.map((o) => (o as unknown as { ctx: DurableObjectState }).ctx.id.name);
      fetched.mockRestore();
    });
    expect(objects, "the two saves reached ONE Archive object").toEqual(["archive:race y", "archive:race y"]);
    const listed = await env.ARCHIVE!.list({ prefix, limit: 1000 });
    const records = listed.objects.filter((o) => o.key.endsWith(".json") && !o.key.endsWith(".table.json")).length;
    expect({ statuses: [a.status, b.status].sort((x, y) => x - y), records }, `${a.text} / ${b.text}`).toEqual({ statuses: [201, 507], records: 256 });
  });
});
