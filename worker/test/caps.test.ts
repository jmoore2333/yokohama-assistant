import { runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it } from "vitest";
import { CLOSE_TOO_MANY, closeReason, TOO_MANY_OWN_LOBBY, TOO_MANY_OWN_TABLE, TOO_MANY_TABLE } from "../src/frames";
import { type House, LOBBY_PER_CALLER, LOBBY_TAG, LOBBY_TOTAL } from "../src/house";
import { callerTag } from "../src/redact";
import { HOUSE_NAME } from "../src/slot";
import type { Table } from "../src/table";
import { SOCKETS_PER_CALLER, SOCKETS_PER_TABLE } from "../src/table/sockets";
import { BLUE, CREATE_BODY, RED, call, tableStub } from "./helpers";

// **The socket caps** (security S2b, finding D3; decision D-4), in the pool. `Tab`, `upgrade` and `openTab` are
// COPIED from `test/sockets.test.ts`: the pool isolates storage and the house per FILE (pool fact 5), so a helper
// module shared with that file would not share a cell's world — this file has its own ids.

/** A signed-in caller seated at no table in this file. */
const GREEN = "green@t.test";

/** How long a cell waits for a message or a close it EXPECTS before failing. */
const WAIT_MS = 2_000;

/** One client socket on a table's stream, as a browser tab holds one. */
interface Tab {
  /** The upgrade's answer: 101, carrying the socket. */
  res: Response;
  ws: WebSocket;
  /** Every message, in arrival order. */
  messages: string[];
  /** Every message so far, once at least `n` have arrived. */
  until(n: number): Promise<string[]>;
  /** The close, once it has come. */
  closed(): Promise<{ code: number; reason: string }>;
}

/** An upgrade of `…/{id}/stream` as `user` (none: no `X-Yokohama-User`),
 *  answered as the object answered it. */
function upgrade(stub: DurableObjectStub<Table>, user: string | undefined, id: string, query = ""): Promise<Response> {
  const headers = new Headers({ Upgrade: "websocket" });
  if (user !== undefined) headers.set("X-Yokohama-User", user);
  return stub.fetch(`https://yokohama.test/api/games/${id}/stream${query}`, { headers });
}

/** A socket on `…/{id}/stream` as `user`, accepted, with every message and
 *  the close kept. Throws when the upgrade is answered unupgraded. */
async function openTab(stub: DurableObjectStub<Table>, user: string, id: string, query = ""): Promise<Tab> {
  const res = await upgrade(stub, user, id, query);
  const ws = res.webSocket;
  if (ws === null) throw new Error(`the upgrade was answered ${res.status}, unupgraded: ${await res.text()}`);
  ws.accept();
  const messages: string[] = [];
  let close: { code: number; reason: string } | undefined;
  const watchers = new Set<() => void>();
  const notify = () => [...watchers].forEach((w) => w());
  ws.addEventListener("message", (e) => {
    messages.push(typeof e.data === "string" ? e.data : "<binary>");
    notify();
  });
  ws.addEventListener("close", (e) => {
    close = { code: e.code, reason: e.reason };
    notify();
  });
  const waitFor = <T>(ready: () => T | undefined, missing: () => string): Promise<T> =>
    new Promise((resolve, reject) => {
      const check = () => {
        const value = ready();
        if (value === undefined) return;
        clearTimeout(timer);
        watchers.delete(check);
        resolve(value);
      };
      const timer = setTimeout(() => {
        watchers.delete(check);
        reject(new Error(missing()));
      }, WAIT_MS);
      watchers.add(check);
      check();
    });
  const seen = () => `${JSON.stringify(messages)}${close ? `, then a close ${close.code} ${JSON.stringify(close.reason)}` : ""}`;
  return {
    res,
    ws,
    messages,
    until: (n) => waitFor(() => (messages.length >= n ? [...messages] : undefined), () => `${n} messages awaited; arrived: ${seen()}`),
    closed: () => waitFor(() => close, () => `a close awaited; arrived: ${seen()}`),
  };
}

/** The `end` frame a cap sends, as the socket carries it, byte for byte. */
const endWith = (sentence: string) => `{"event":"end","data":${JSON.stringify({ reason: "gone", sentence })}}`;

/** A table at `name` with red seated and one seat OPEN, so any signed-in caller may open its stream. */
async function openTable(name: string): Promise<DurableObjectStub<Table>> {
  const stub = tableStub(name);
  const res = await call(stub, RED, "POST", "/api/games", CREATE_BODY, name);
  expect(res.status).toBe(201);
  return stub;
}

/** How many sockets the object holds under `tag` — or in all with none — read inside it. */
function held(stub: DurableObjectStub<Table> | DurableObjectStub<House>, tag?: string): Promise<number> {
  return runInDurableObject(stub as DurableObjectStub, (_o, state) => state.storage.sql !== undefined ? (tag === undefined ? state.getWebSockets().length : state.getWebSockets(tag).length) : -1);
}

/** How many OPEN sockets the object holds under `tag` — the count the caps read (`openCount`). */
function heldOpen(stub: DurableObjectStub<Table> | DurableObjectStub<House>, tag?: string): Promise<number> {
  return runInDurableObject(stub as DurableObjectStub, (_o, state) => (tag === undefined ? state.getWebSockets() : state.getWebSockets(tag)).filter((ws) => ws.readyState === WebSocket.OPEN).length);
}

describe("D3 — the socket caps (security S2b, decision D-4)", () => {
  // Mutants: the per-caller arm dropped (the ninth admitted); the tag omitted from `acceptWebSocket` (the count
  // reads 0 and the ninth is admitted); the refusal answered unupgraded (no 101: the client's ladder would hammer);
  // the close code 4404 (a deleted table's code); the literal 9.
  it("admits a caller's eighth socket and refuses the ninth with an end frame and a 4429 close, another caller unaffected, and frees the slot once a socket has closed", async () => {
    const id = "caps-own";
    const stub = await openTable(id);
    const tabs: Tab[] = [];
    for (let i = 0; i < SOCKETS_PER_CALLER; i += 1) tabs.push(await openTab(stub, GREEN, id));
    expect(await held(stub, await callerTag(GREEN))).toBe(SOCKETS_PER_CALLER);
    const ninth = await openTab(stub, GREEN, id);
    expect(ninth.res.status, "accepted, so the browser reads the frame").toBe(101);
    expect(await ninth.until(1)).toEqual([endWith(TOO_MANY_OWN_TABLE)]);
    expect(await ninth.closed()).toEqual({ code: CLOSE_TOO_MANY, reason: closeReason(TOO_MANY_OWN_TABLE) });
    expect(await held(stub, await callerTag(GREEN)), "the refused socket is not kept").toBe(SOCKETS_PER_CALLER);
    const other = await openTab(stub, BLUE, id);
    expect((await other.until(1))[0]).toContain('"event":"signal"');
    // Review Focus 4: a slot frees once the close has been processed (polled, bounded: no event marks it in the pool —
    // a client-initiated close delivers no `close` event, so `await tabs[0]!.closed()` would time out; measured).
    tabs[0]!.ws.close(1000);
    for (let i = 0; i < 20 && (await held(stub, await callerTag(GREEN))) >= SOCKETS_PER_CALLER; i += 1) await new Promise((r) => setTimeout(r, 50));
    const again = await openTab(stub, GREEN, id);
    expect((await again.until(1))[0], "admitted once a socket has gone").toContain('"event":"signal"');
  });

  it("refuses the sixty-fifth socket of a table whatever its caller, with the table's own sentence", async () => {
    const id = "caps-total";
    const stub = await openTable(id);
    const callers = Array.from({ length: SOCKETS_PER_TABLE / SOCKETS_PER_CALLER }, (_, i) => `c${i}@t.test`);
    const tabs: Tab[] = [];
    for (const who of callers) for (let i = 0; i < SOCKETS_PER_CALLER; i += 1) tabs.push(await openTab(stub, who, id));
    expect(await held(stub)).toBe(SOCKETS_PER_TABLE);
    const more = await openTab(stub, "late@t.test", id);
    expect(await more.until(1)).toEqual([endWith(TOO_MANY_TABLE)]);
    expect((await more.closed()).code).toBe(CLOSE_TOO_MANY);
    // The review's Important 1: a refused socket whose client never answers the close stays CLOSING and is NOT
    // counted — counted, a stranger's unanswered refusals would lock the seated players out (measured: 60 dangling
    // refusals, 68 held). Eight refusals left dangling (their client side never accepted), then one socket closes: the
    // next caller is admitted past them.
    const dangling: Response[] = [];
    for (let i = 0; i < 8; i += 1) dangling.push(await upgrade(stub, "dangling@t.test", id));
    expect(dangling.map((r) => r.status), "accepted, closing").toEqual(Array(8).fill(101));
    expect(await held(stub), "the dangling refusals are held, CLOSING").toBeGreaterThan(SOCKETS_PER_TABLE);
    expect(await heldOpen(stub), "and not counted").toBe(SOCKETS_PER_TABLE);
    tabs[0]!.ws.close(1000);
    for (let i = 0; i < 20 && (await heldOpen(stub)) >= SOCKETS_PER_TABLE; i += 1) await new Promise((r) => setTimeout(r, 50));
    const back = await openTab(stub, "late@t.test", id);
    expect((await back.until(1))[0], "admitted past the dangling refusals").toContain('"event":"signal"');
    expect(dangling.length, "held for the cell's life, like the lobby's kept").toBe(8);
  });

  it("caps the lobby per caller the same way, under its own sentence, and counts only OPEN sockets toward its total", async () => {
    const house = env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME));
    const lobby = (who: string) => house.fetch("https://yokohama.test/api/stream", { headers: { Upgrade: "websocket", "X-Yokohama-User": who } });
    const kept: Response[] = []; // held for the cell's life: an unaccepted client socket that is dropped is garbage-collected, and its slot with it (measured: 8 held, then 0 — the cell flaked 2 of 5 runs without this)
    for (let i = 0; i < 8; i += 1) kept.push(await lobby("lob@t.test"));
    expect(kept.map((r) => r.status)).toEqual(Array(8).fill(101));
    const ninth = await lobby("lob@t.test");
    expect(ninth.status).toBe(101);
    const ws = ninth.webSocket!;
    ws.accept();
    const first = await new Promise<string>((resolve) => ws.addEventListener("message", (e) => resolve(String(e.data)), { once: true }));
    expect(first).toBe(endWith(TOO_MANY_OWN_LOBBY));
    const closed = await new Promise<{ code: number; reason: string }>((resolve) => ws.addEventListener("close", (e) => resolve({ code: e.code, reason: e.reason }), { once: true }));
    expect(closed, "the same close as a table's").toEqual({ code: CLOSE_TOO_MANY, reason: closeReason(TOO_MANY_OWN_LOBBY) });
    // The review's Important 2: the cap is PER CALLER — a second caller is admitted while the first is full (under a
    // site-wide lobby of eight this reads 0 OPEN under the second caller's tag; measured at the review).
    const other = await lobby("lob2@t.test");
    expect(other.status).toBe(101);
    kept.push(other);
    const otherWs = other.webSocket!;
    otherWs.accept();
    const listing = new Promise<string>((resolve) => otherWs.addEventListener("message", (e) => resolve(String(e.data)), { once: true }));
    // A lobby socket is silent on connect (spec §2.5; `slot.test.ts`'s cell), so a house chat post is the positive
    // control (ruling R-10): its frame reaches every admitted lobby socket, where a refused one's first frame is `end`.
    const posted = await house.fetch("https://yokohama.test/api/chat", { method: "POST", headers: { "X-Yokohama-User": "lob2@t.test", "content-type": "application/json" }, body: '{"text":"hello"}' });
    expect(posted.status, await posted.text()).toBe(201);
    expect(await listing, "the second caller hears the lobby's frame, not an end").toContain('"event":"house"');
    expect(await heldOpen(house, await callerTag("lob2@t.test")), "one OPEN socket under the second caller's tag").toBe(1);
    expect(await held(house, await callerTag("lob@t.test"))).toBe(8);
    expect(await held(house, LOBBY_TAG), "every lobby socket still carries the lobby tag: the first caller's eight and the second's one").toBe(9);
    // The lobby's TOTAL count site (the re-review's gap — a mutant per count site): the first caller leaves 1,024
    // refusals dangling, never answering their closes, and a third caller is still admitted; a total that counted
    // CLOSING sockets would answer the lobby's sentence (measured: one identity filled the lobby in 1.9 s; these rows
    // cost about 2 s, hence the cell's own 15 s bound).
    for (let i = 0; i < LOBBY_TOTAL; i += 1) kept.push(await lobby("lob@t.test"));
    expect(await held(house), "held, CLOSING").toBeGreaterThanOrEqual(LOBBY_TOTAL);
    expect(await heldOpen(house), "and not counted: the first caller's eight and the second's one").toBe(9);
    const third = await lobby("lob3@t.test");
    expect(third.status).toBe(101);
    kept.push(third);
    const thirdWs = third.webSocket!;
    thirdWs.accept();
    const heard = new Promise<string>((resolve) => thirdWs.addEventListener("message", (e) => resolve(String(e.data)), { once: true }));
    const again = await house.fetch("https://yokohama.test/api/chat", { method: "POST", headers: { "X-Yokohama-User": "lob3@t.test", "content-type": "application/json" }, body: '{"text":"again"}' });
    expect(again.status, await again.text()).toBe(201);
    expect(await heard, "admitted past 1,024 dangling refusals").toContain('"event":"house"');
    expect([LOBBY_PER_CALLER, LOBBY_TOTAL], "the lobby's literals (node cannot import house.ts: it loads cloudflare:workers)").toEqual([8, 1024]);
  }, 15_000);
});
