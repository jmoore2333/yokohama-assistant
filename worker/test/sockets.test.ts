import { abortAllDurableObjects, evictDurableObject, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import compiled from "../gen/yokohama_worker_bg.wasm";
import { createModule, type Reply } from "../gen/factory.js";
import * as configModule from "../src/config";
import { closeReason, frame } from "../src/frames";
import { readReply, type Bump } from "../src/module";
import { ensureTables, upsertDoc } from "../src/storage";
import { Table } from "../src/table";
import { OWED_PUMP_JSON, OWED_PUMP_TABLE_JSON } from "./fixtures.gen";
import { BLUE, CREATE_BODY, RED, call, logsDuring, seedPlaying, settle, sqlOf, tableStub, type Play } from "./helpers";

/** A signed-in caller seated at no table in this file. */
const GREEN = "green@t.test";

/** What `GET …/events` answers, and so what a `signal` frame carries. It holds
 *  no seed, so a cell may parse it. */
type Events = { events: number; to_move: number | null; status: string; your_turn: boolean; chat: number; chat_rev: number };

/** A `signal` frame's reading, parsed. */
function dataOf(message: string | undefined): Events {
  if (message === undefined) throw new Error("no message to read");
  return (JSON.parse(message) as { data: Events }).data;
}

/** The SSE's two `end` frames as a socket carries them, as LITERALS: a frame
 *  the object sends is compared with these bytes, not with the shell's own
 *  constants. */
const END_DELETED = '{"event":"end","data":{"reason":"deleted"}}';
const END_GONE = '{"event":"end","data":{"reason":"gone"}}';

/** The close reason a deleted table's sockets are closed with. */
const DELETED_REASON = "this table was deleted";

/** How long a cell waits for a message or a close it EXPECTS before failing.
 *  No cell waits out a timer to show that something did NOT arrive: that is
 *  read off the order of one socket's messages, after a later action whose
 *  frame does arrive (the orchestrator's ruling R-10). */
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

/** `GET …/{id}/events` as `user`, which must answer 200: the reading a
 *  `signal` frame carries for that viewer. */
async function eventsText(stub: DurableObjectStub<Table>, user: string, id: string): Promise<string> {
  const res = await call(stub, user, "GET", `/api/games/${id}/events`);
  const text = await res.text();
  expect(res.status, `GET …/events as ${user}: ${text}`).toBe(200);
  return text;
}

/** `POST …/{id}/input` with `line`, as `user`. */
function input(stub: DurableObjectStub<Table>, user: string, id: string, line: string): Promise<Response> {
  return call(stub, user, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
}

/** A chat post as `user`, which must answer 201: a `bump_table`, so the
 *  positive control a negative cell reads its order against (ruling R-10). */
async function chat(stub: DurableObjectStub<Table>, user: string, id: string, text: string): Promise<void> {
  const res = await call(stub, user, "POST", `/api/games/${id}/chat`, JSON.stringify({ text }));
  expect(res.status, `chat as ${user}: ${await res.clone().text()}`).toBe(201);
}

/**
 * The bump list each `sendFrames` call received while `f` ran, in order — how
 * a cell asserts its premise that a reply carried bumps (the orchestrator's
 * ruling I-39). The spy is on the PROTOTYPE, which the live object reads
 * through (pool fact 1), and it calls through.
 */
async function framed<T>(f: () => Promise<T>): Promise<{ result: T; bumps: Bump[][] }> {
  const spy = vi.spyOn(Table.prototype, "sendFrames");
  try {
    const result = await f();
    return { result, bumps: spy.mock.calls.map(([, bumps]) => bumps) };
  } finally {
    spy.mockRestore();
  }
}

/**
 * Red's measured PlaceA after `seedPlaying` (the orchestrator's ruling I-39):
 * `1` opens the picker and `A09` selects Customs — neither changes the record,
 * so neither bumps — and `confirm` applies the placement, whose reply carries
 * the move's bump. Answers the confirm's play payload.
 */
async function placeA(stub: DurableObjectStub<Table>, id: string): Promise<Play> {
  for (const line of ["1", "A09"]) {
    const { result, bumps } = await framed(() => input(stub, RED, id, line));
    expect([result.status, bumps.flat()], `premise: \`${line}\` answers 200 and bumps nothing`).toEqual([200, []]);
  }
  const { result, bumps } = await framed(() => input(stub, RED, id, "confirm"));
  expect(result.status).toBe(200);
  expect(bumps, "premise: the move's reply carries its bump").toEqual([[{ bump: id }]]);
  const play = await result.json<Play>();
  expect(play.events, "premise: the placement applied").toBe(2);
  return play;
}

/** The `wakes` count, as `meta` holds it. */
async function wakes(stub: DurableObjectStub<Table>): Promise<number> {
  const [row] = await sqlOf(stub, "SELECT v FROM meta WHERE k = 'wakes'");
  return Number(row?.v ?? "0");
}

/** A fresh module in this configuration: the layer's own answer to compare an
 *  object's against. */
function referenceModule() {
  const mod = createModule(compiled);
  mod.init(configModule.moduleConfig(env, null));
  mod.set_clock(1_789_000_000);
  return mod;
}

describe("the table's sockets (spec §2.4)", () => {
  // Mutants: no frame on accept; the frame computed for one fixed viewer
  // rather than the socket's own; the frame's envelope keyed differently.
  it("the first frame on accept is the events route's answer, framed", async () => {
    const id = "first-frame";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const blue = await openTab(stub, BLUE, id);
    expect([red.res.status, blue.res.status]).toEqual([101, 101]);
    const [redFirst] = await red.until(1);
    const [blueFirst] = await blue.until(1);
    const redEvents = await eventsText(stub, RED, id);
    const blueEvents = await eventsText(stub, BLUE, id);
    expect(redEvents, "premise: the two viewers read differently").not.toBe(blueEvents);
    expect(redFirst).toBe(`{"event":"signal","data":${redEvents}}`);
    expect(blueFirst).toBe(frame("signal", blueEvents));
  });

  // Spec §2.4: a socket is accepted with the table's id as its tag. Nothing
  // reads the tag today — `sendFrames` frames every socket the object holds —
  // so this cell keeps a later `getWebSockets(id)` honest. Mutant: any other
  // tag.
  it("every socket is accepted under the table's id as its tag", async () => {
    const id = "tagged";
    const stub = await seedPlaying(id);
    const tabs = [await openTab(stub, RED, id), await openTab(stub, BLUE, id)];
    await Promise.all(tabs.map((tab) => tab.until(1)));
    const tags = await runInDurableObject(stub, (_o, state) => state.getWebSockets().map((ws) => state.getTags(ws)));
    expect(tags).toEqual([[id], [id]]);
    expect(await runInDurableObject(stub, (_o, state) => state.getWebSockets(id).length)).toBe(2);
  });

  // Each request here carries ONE bump, so this cell is about the viewer; a
  // frame per bump is the finalizing keep's cell. Mutants: every socket sent
  // the first socket's reading, or the request's caller's; the viewer lost
  // with an attachment never written (nothing is sent). The chat post is the
  // positive control: its frame arrives last, so any extra frame the move
  // sent would have arrived before it.
  it("one frame per socket per reply, each for its own viewer", async () => {
    const id = "per-viewer";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const blue = await openTab(stub, BLUE, id);
    const [redFirst] = await red.until(1);
    const [blueFirst] = await blue.until(1);
    await placeA(stub, id);
    const moved = { red: await eventsText(stub, RED, id), blue: await eventsText(stub, BLUE, id) };
    const read = (text: string) => JSON.parse(text) as Events;
    expect([read(moved.red).your_turn, read(moved.blue).your_turn], "red is still to move; blue is not").toEqual([true, false]);
    await chat(stub, BLUE, id, "nice");
    const chatted = { red: await eventsText(stub, RED, id), blue: await eventsText(stub, BLUE, id) };
    expect(await red.until(3)).toEqual([redFirst, frame("signal", moved.red), frame("signal", chatted.red)]);
    expect(await blue.until(3)).toEqual([blueFirst, frame("signal", moved.blue), frame("signal", chatted.blue)]);
  });

  // Plan review m5: a finalizing keep carries `bump_table` then `bump`
  // (Task 2), and two identical frames would each wake a screen. Mutant: a
  // frame per bump — red's socket then holds the keep's frame twice before
  // the chat post's.
  it("a finalizing keep's two bumps are ONE frame per socket", async () => {
    const id = "finalizing";
    const stub = tableStub(id);
    expect((await call(stub, RED, "POST", "/api/games", CREATE_BODY, id)).status).toBe(201);
    expect((await call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}')).status).toBe(200);
    const red = await openTab(stub, RED, id);
    const [first] = await red.until(1);
    expect(dataOf(first).status, "premise: the table is dealing").toBe("dealing");
    const keep = await framed(() => call(stub, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}'));
    expect([keep.result.status, keep.bumps], "premise: a keep that does not finalize").toEqual([200, [[{ bump_table: id }]]]);
    const kept = await eventsText(stub, RED, id);
    const last = await framed(() => call(stub, BLUE, "POST", `/api/games/${id}/keep`, '{"seat":1,"card":1}'));
    expect([last.result.status, last.bumps], "premise: the finalizing keep's reply carries both bumps").toEqual([
      200,
      [[{ bump_table: id }, { bump: id }]],
    ]);
    const finalized = await eventsText(stub, RED, id);
    expect((JSON.parse(finalized) as Events).status, "premise: the keep finalized").toBe("playing");
    await chat(stub, RED, id, "good luck");
    const chatted = await eventsText(stub, RED, id);
    expect(await red.until(4)).toEqual([
      first,
      frame("signal", kept),
      frame("signal", finalized),
      frame("signal", chatted),
    ]);
  });

  // Review Focus 5. "Blue moves" is red's move (the orchestrator's ruling
  // I-39: blue's turn comes only when red's ends). Mutants: one frame per
  // USER rather than per socket; sockets kept in a map keyed by user, so the
  // second tab replaces the first.
  it("one person with two tabs gets every frame on both", async () => {
    const id = "two-tabs";
    const stub = await seedPlaying(id);
    const tabs = [await openTab(stub, RED, id), await openTab(stub, RED, id)];
    const first = frame("signal", await eventsText(stub, RED, id));
    await placeA(stub, id);
    const moved = frame("signal", await eventsText(stub, RED, id));
    await chat(stub, BLUE, id, "your move");
    const chatted = frame("signal", await eventsText(stub, RED, id));
    for (const tab of tabs) expect(await tab.until(3)).toEqual([first, moved, chatted]);
  });

  // Battery 24: an admission lost under a live socket. `events` answers any
  // signed-in caller at an OPEN table, and nobody without a seat at a deal.
  // The mid-stream `end` is the SSE's own, byte for byte — no sentence (the
  // orchestrator's ruling: only the admission-time `end` carries one). Red,
  // seated, is the control: framed, and left open. Mutants: a 403 read as a
  // failure and sent nothing; the sentence added to this `end`; the close
  // code 4404 for a 403; the end sent without a close.
  it("a stranger holding an OPEN table's stream is ended when the last seat fills", async () => {
    const id = "filled";
    const stub = tableStub(id);
    expect((await call(stub, RED, "POST", "/api/games", CREATE_BODY, id)).status).toBe(201);
    const green = await openTab(stub, GREEN, id);
    const red = await openTab(stub, RED, id);
    const open = frame("signal", await eventsText(stub, GREEN, id));
    expect(await green.until(1), "premise: a stranger is admitted at an open table").toEqual([open]);
    const [redFirst] = await red.until(1);
    const join = await framed(() => call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}'));
    expect([join.result.status, join.bumps], "premise: the join that fills the table bumps it").toEqual([200, [[{ bump: id }]]]);
    const refused = await call(stub, GREEN, "GET", `/api/games/${id}/events`);
    const sentence = (await refused.json<{ error: string }>()).error;
    expect(refused.status, "premise: the stranger is refused at a deal").toBe(403);
    expect(await green.closed()).toEqual({ code: 4403, reason: sentence });
    expect(green.messages).toEqual([open, END_GONE]);
    const dealing = frame("signal", await eventsText(stub, RED, id));
    expect(await red.until(2)).toEqual([redFirst, dealing]);
    expect(red.ws.readyState, "the seated socket stays open").toBe(WebSocket.OPEN);
  });

  // Spec §5.2's other mid-stream half: a reading lost to a 404 under a live
  // socket. No known reading turns into a 404 under a live socket — a
  // delete's `end` comes first — so the live module's `events` answers ONCE
  // with the layer's own 404 for this id: a fresh module's, which holds no
  // table. The close carries the
  // sentence, cut, as its only carrier: this `end` is the SSE's own. Mutants:
  // every mid-stream refusal closed 4403; a 404 read as a failure (nothing
  // sent); the close's reason dropped.
  it("a reading lost mid-stream to a 404 is ended gone and closed 4404 with the layer's sentence", async () => {
    const id = "lost-404";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const [first] = await red.until(1);
    const missing = readReply(referenceModule().events(id, RED, ""));
    expect(missing.status, "premise: the layer's 404 for a table it does not hold").toBe(404);
    const sentence = (JSON.parse(missing.body) as { error: string }).error;
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      const events = mod.events;
      mod.events = () => {
        mod.events = events;
        return { status: missing.status, body: missing.body, bumps: "[]", logs: "[]", free() {} } as unknown as Reply;
      };
    });
    const posted = await framed(() => chat(stub, BLUE, id, "anyone?"));
    expect(posted.bumps, "premise: the post bumped the table").toEqual([[{ bump_table: id }]]);
    expect(await red.closed()).toEqual({ code: 4404, reason: closeReason(sentence) });
    expect(red.messages).toEqual([first, END_GONE]);
  });

  // Spec §2.4: a refused upgrade is ACCEPTED, ended and closed, because a
  // browser reports a refused upgrade as an anonymous failure it cannot tell
  // from the network. The admission-time `end` carries the layer's sentence
  // (the orchestrator's ruling, Task 8's review). A probe of an unknown id
  // still writes nothing. Mutants: the refusal answered unupgraded; no `end`
  // before the close; the sentence left out of the frame, or the close's
  // reason; 4403 and 4404 swapped.
  it("admission refused at the upgrade is accepted, ended and closed with the layer's code", async () => {
    const id = "dealt";
    const stub = tableStub(id);
    expect((await call(stub, RED, "POST", "/api/games", CREATE_BODY, id)).status).toBe(201);
    expect((await call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}')).status).toBe(200);
    const refused = await call(stub, GREEN, "GET", `/api/games/${id}/events`);
    const sentence = (await refused.json<{ error: string }>()).error;
    expect(refused.status, "premise: a stranger is refused at a deal").toBe(403);
    const stranger = await openTab(stub, GREEN, id);
    expect(stranger.res.status).toBe(101);
    expect(await stranger.closed()).toEqual({ code: 4403, reason: sentence });
    expect(stranger.messages).toEqual([`{"event":"end","data":{"reason":"gone","sentence":${JSON.stringify(sentence)}}}`]);

    const unknown = "never-streamed";
    const nobody = tableStub(unknown);
    const missing = await call(nobody, RED, "GET", `/api/games/${unknown}/events`);
    const missingSentence = (await missing.json<{ error: string }>()).error;
    expect(missing.status, "premise: no table has this id").toBe(404);
    const lost = await openTab(nobody, RED, unknown);
    expect(lost.res.status).toBe(101);
    expect(await lost.closed()).toEqual({ code: 4404, reason: missingSentence });
    expect(lost.messages).toEqual([
      `{"event":"end","data":{"reason":"gone","sentence":${JSON.stringify(missingSentence)}}}`,
    ]);
    expect(await sqlOf(nobody, "SELECT count(*) AS n FROM sqlite_master"), "the probe wrote nothing").toEqual([{ n: 0 }]);
  });

  // Spec §2.4: "on any other status it answers that status unupgraded, as the
  // layer answered it" — the 401 among them, which a browser cannot read and
  // spec §2.7's probe does. Mutant: every refusal accepted and closed.
  it("an upgrade the layer answers otherwise is answered unupgraded, as the layer answered it", async () => {
    const id = "unupgraded";
    const stub = await seedPlaying(id);
    const anonymous = await upgrade(stub, undefined, id);
    const noCaller = readReply(referenceModule().events(id, undefined, ""));
    expect(noCaller.status, "premise: the layer's 401").toBe(401);
    expect([anonymous.status, anonymous.webSocket, await anonymous.text()]).toEqual([401, null, noCaller.body]);
    const bad = "bad!stream";
    const unusable = await upgrade(tableStub(bad), RED, bad);
    const refusal = readReply(referenceModule().events(bad, RED, ""));
    expect(refusal.status, "premise: the layer's refusal of an id the store will not use").toBe(500);
    expect([unusable.status, unusable.webSocket, await unusable.text()]).toEqual([500, null, refusal.body]);
  });

  // Mutants: the `end` never read, so nothing is sent and no close comes; no
  // close, or another code.
  it("a delete ends every socket as deleted, 4404", async () => {
    const id = "deleted";
    const stub = await seedPlaying(id);
    const tabs = [await openTab(stub, RED, id), await openTab(stub, BLUE, id)];
    const firsts = await Promise.all(tabs.map(async (tab) => (await tab.until(1))[0]));
    const deleted = await framed(() => call(stub, RED, "DELETE", `/api/games/${id}`));
    expect([deleted.result.status, deleted.bumps], "premise: the creator's delete ends the table").toEqual([
      204,
      [[{ end: id }]],
    ]);
    for (const [n, tab] of tabs.entries()) {
      expect(await tab.closed()).toEqual({ code: 4404, reason: DELETED_REASON });
      // Read at the close, after which nothing can arrive: no signal came.
      expect(tab.messages).toEqual([firsts[n], END_DELETED]);
    }
  });

  // M6.5c2 cycle D (R-D4; Review Focus 2, the objects' half): a chat message's
  // DELETE is an ORDINARY request — never the abandon, though it is a DELETE
  // under the table's path — answered 403 for a seat that neither wrote it nor
  // created the table, and 204 for its author, whose reply frames every socket
  // ONCE with `chat_rev` moved and `chat` not. The refusal frames nothing: the
  // author's delete is the positive control whose frame arrives next (ruling
  // R-10). Evicted and read again, the tombstone is STORED.
  //
  // **Which path it took is read off a spy on the prototype's `abandon`**
  // (pool fact 1), because the answers alone cannot tell: the abandon's own
  // dispatch reaches the layer too, which answers a chat delete correctly
  // there — measured, this cell's answers held on a shell that sent every
  // DELETE to the abandon — only under `blockConcurrencyWhile`, beside an
  // `events` reading and the house copy's facts a delete of a TABLE needs.
  // The table's own DELETE at the end is the anti-vacuity half: the spy sees
  // that one. Mutants: every DELETE the abandon (`abandons` reading the method
  // alone); the tombstone kept in the module and never committed; `chat`
  // moved by the delete.
  it("a chat message's delete is a request, not the abandon: 403 for another seat, 204 for its author, one frame", async () => {
    const id = "unsaid";
    const stub = await seedPlaying(id);
    await chat(stub, RED, id, "oops");
    const red = await openTab(stub, RED, id);
    const blue = await openTab(stub, BLUE, id);
    const [redFirst] = await red.until(1);
    const [blueFirst] = await blue.until(1);
    expect([dataOf(redFirst).chat, dataOf(redFirst).chat_rev], "premise: one message, one revision").toEqual([1, 1]);
    const abandon = vi.spyOn(Table.prototype as unknown as { abandon(incoming: unknown): Promise<Response> }, "abandon");
    try {
      await deleteAMessage(stub, id, red, blue, redFirst, blueFirst);
      expect(abandon, "a chat message's delete never runs the abandon").not.toHaveBeenCalled();
      expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status, "the table's own delete").toBe(204);
      expect(abandon, "and the table's own DELETE does").toHaveBeenCalledTimes(1);
    } finally {
      abandon.mockRestore();
      // The delete's push to the house rides `waitUntil` after its reply.
      await settle(stub);
    }
  });

  /** `a chat message's delete is a request, not the abandon`'s body, with the
   *  two sockets it reads. */
  async function deleteAMessage(
    stub: DurableObjectStub<Table>,
    id: string,
    red: Tab,
    blue: Tab,
    redFirst: string | undefined,
    blueFirst: string | undefined,
  ): Promise<void> {
    const refused = await framed(() => call(stub, BLUE, "DELETE", `/api/games/${id}/chat/1`));
    expect([refused.result.status, await refused.result.text(), refused.bumps.flat()]).toEqual([
      403,
      '{"error":"only the message\'s author or the table\'s creator may delete it"}',
      [],
    ]);
    const deleted = await framed(() => call(stub, RED, "DELETE", `/api/games/${id}/chat/1`));
    expect([deleted.result.status, await deleted.result.text(), deleted.bumps.flat()]).toEqual([204, "", [{ bump_table: id }]]);
    const after = { red: await eventsText(stub, RED, id), blue: await eventsText(stub, BLUE, id) };
    expect([dataOf(frame("signal", after.red)).chat, dataOf(frame("signal", after.red)).chat_rev]).toEqual([1, 2]);
    expect(await red.until(2)).toEqual([redFirst, frame("signal", after.red)]);
    expect(await blue.until(2)).toEqual([blueFirst, frame("signal", after.blue)]);

    await evictDurableObject(stub);
    const page = await call(stub, BLUE, "GET", `/api/games/${id}/chat`);
    const read = (await page.json()) as { messages: { seq: number; text: string; deleted: boolean }[]; last: number };
    expect([read.last, read.messages.map((m) => [m.seq, m.text, m.deleted])]).toEqual([1, [[1, "", true]]]);
  }

  // The orchestrator's ruling (Task 9's review, escalation 2): a request's
  // frames are computed over ALL the bumps its commit carried — the wake's
  // `restore_picker`, which pumps a bot move a cold rebuild owes, as well as
  // the call's own. The owed pair (a bot to move) is written back into
  // storage before each step and the object evicted, so each step's WAKE
  // pumps. Three requests read it: a second socket's upgrade (the socket
  // already open gets the pump's frame; the upgrading one only its first
  // frame, which already reads the pump), a GET, and a delete (the wake's
  // `bump` and the delete's `end` in one request: `end` wins). Mutants: the
  // wake's bumps not framed; a socket accepted before the request's frames
  // are sent, so it gets two for one request; the bumps framed in order, so
  // the delete's sockets are sent the reading after it — a 404, `gone`.
  it("the wake's bumps are framed with the call's, once per socket, and an end among them wins", async () => {
    const id = "owed";
    const stub = tableStub(id);
    await runInDurableObject(stub, (_o, state) => {
      state.storage.transactionSync(() => {
        ensureTables(state.storage.sql);
        upsertDoc(state.storage.sql, "table", OWED_PUMP_TABLE_JSON, 1);
        upsertDoc(state.storage.sql, "record", OWED_PUMP_JSON, 1);
      });
    });
    const owe = async () => {
      await runInDurableObject(stub, (_o, state) => {
        state.storage.transactionSync(() => upsertDoc(state.storage.sql, "record", OWED_PUMP_JSON, 1));
      });
      await evictDurableObject(stub);
    };
    const a = await openTab(stub, RED, id);
    const [pumped] = await a.until(1);
    expect(dataOf(pumped).events, "premise: the upgrade's own wake pumped").toBeGreaterThan(4);

    await owe();
    const second = await framed(() => openTab(stub, RED, id));
    expect(second.bumps, "premise: the upgrade's wake pumped again").toEqual([[{ bump: id }]]);
    const b = second.result;
    const read = frame("signal", await eventsText(stub, RED, id));
    expect(read, "premise: the pump is deterministic").toBe(pumped);

    await owe();
    const got = await framed(() => call(stub, RED, "GET", `/api/games/${id}/table`));
    expect([got.result.status, got.bumps], "premise: a GET's wake pumped").toEqual([200, [[{ bump: id }]]]);

    await owe();
    const deleted = await framed(() => call(stub, RED, "DELETE", `/api/games/${id}`));
    expect([deleted.result.status, deleted.bumps], "premise: the wake's pump and the delete's end, in ONE request").toEqual([
      204,
      [[{ bump: id }, { end: id }]],
    ]);
    for (const tab of [a, b]) expect(await tab.closed()).toEqual({ code: 4404, reason: DELETED_REASON });
    expect(a.messages).toEqual([pumped, read, read, END_DELETED]);
    expect(b.messages).toEqual([read, read, END_DELETED]);
  });

  // Spec §2.4: no client message reaches `dispatch`. The picker is open with
  // Customs selected first, so a `confirm` that reached it would APPLY a move
  // — `1` alone opens a picker and changes no record. The `ping` sent last is
  // answered `pong` by the platform's auto-response, and a socket's messages
  // are read in order, so the `pong` arrives only after every message before
  // it was read; blue's chat post is the positive control. Mutants: a message
  // dispatched as an input line; no auto-response (the `ping` would reach
  // `webSocketMessage`, which answers nothing).
  it("a socket message is never a move", async () => {
    const id = "no-moves";
    const stub = await seedPlaying(id);
    for (const line of ["1", "A09"]) expect((await input(stub, RED, id, line)).status).toBe(200);
    const game = () => sqlOf(stub, "SELECT kind, json FROM docs WHERE kind IN ('record', 'picker') ORDER BY kind");
    const before = await game();
    expect(before.map((row) => row.kind), "premise: a record, and an open picker").toEqual(["picker", "record"]);
    const red = await openTab(stub, RED, id);
    const [first] = await red.until(1);
    for (const message of ['{"line":"1"}', "1", '{"line":"confirm"}', "confirm", "ping"]) red.ws.send(message);
    expect(await red.until(2)).toEqual([first, "pong"]);
    await chat(stub, BLUE, id, "well?");
    const chatted = frame("signal", await eventsText(stub, RED, id));
    expect(await red.until(3)).toEqual([first, "pong", chatted]);
    expect(await game()).toEqual(before);
    const play = await (await call(stub, RED, "GET", `/api/games/${id}`)).json<Play>();
    expect([play.events, play.prompt.kind, play.prompt.selected]).toEqual([1, "place_a", [9]]);
  });

  // The SSE's comment arm: a reading that fails for any reason but a 403 or a
  // 404 is not a table that went, so the socket is sent nothing and stays
  // open. The live module's `events` answers 500 ONCE — the framing's call
  // for the one socket — and the next bump is the positive control. Mutants:
  // the 500 sent as a `signal`, or ended as `gone`; the socket closed.
  it("a failure other than 403/404 sends nothing", async () => {
    const id = "failed-read";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const [first] = await red.until(1);
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      const events = mod.events;
      mod.events = () => {
        mod.events = events;
        const failed = { status: 500, body: '{"error":"the store could not be read"}', bumps: "[]", logs: "[]", free() {} };
        return failed as unknown as Reply;
      };
    });
    const failed = await framed(() => chat(stub, BLUE, id, "one"));
    expect(failed.bumps, "premise: the post bumped the table").toEqual([[{ bump_table: id }]]);
    await chat(stub, BLUE, id, "two");
    const chatted = frame("signal", await eventsText(stub, RED, id));
    expect(await red.until(2)).toEqual([first, chatted]);
    expect(red.ws.readyState).toBe(WebSocket.OPEN);
  });

  // Spec §2.4 "Traps": a trap in a SIDE CHANNEL comes after the batch and the
  // reply. The live module's `events` traps, so the move's framing traps on
  // its first socket. The move answers 200 and its batch is stored; ONE trap
  // is logged, and NO socket is framed after it — `guarded` would answer the
  // next socket from an EMPTY module, whose reading of this table is a 404
  // that would end a live socket, and would leave that empty module for the
  // next request to answer from. The next GET is a new instance (`wakes`
  // rises), reloaded from storage. The chat post is the positive control.
  // Mutants: framing going on after the trap (the other socket is ended, and
  // the GET answers 404 from the empty module left behind); the trap caught
  // without dropping the module (the GET traps); the framing's call made
  // outside `guarded` (the trap reaches the `frame-failed` arm and is logged
  // as that).
  it("a trap while framing discards the instance, sends nothing, and the next request is answered afresh", async () => {
    const id = "frame-trap";
    const stub = await seedPlaying(id);
    const tabs = [await openTab(stub, RED, id), await openTab(stub, BLUE, id)];
    const firsts = await Promise.all(tabs.map(async (tab) => (await tab.until(1))[0]));
    for (const line of ["1", "A09"]) expect((await input(stub, RED, id, line)).status).toBe(200);
    const [record] = await sqlOf(stub, "SELECT json FROM docs WHERE kind = 'record'");
    const woken = await wakes(stub);
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      mod.events = () => mod.debug_trap!();
    });
    const { result: confirm, lines } = await logsDuring(() => framed(() => input(stub, RED, id, "confirm")));
    expect(confirm.bumps, "premise: the move bumped the table").toEqual([[{ bump: id }]]);
    expect(confirm.result.status, "the reply stands").toBe(200);
    expect(lines).toEqual([{ event: "trap", id, call: "frame", error: "RuntimeError: unreachable" }]);
    const [stored] = await sqlOf(stub, "SELECT json FROM docs WHERE kind = 'record'");
    expect(stored?.json, "the move's batch is stored").not.toBe(record?.json);
    const game = await call(stub, RED, "GET", `/api/games/${id}`);
    expect([game.status, (await game.json<Play>()).events], "a new instance, reloaded: the move is there").toEqual([200, 2]);
    expect(await wakes(stub), "the GET woke a new instance").toBe(woken + 1);
    await chat(stub, BLUE, id, "still here");
    const chatted = [frame("signal", await eventsText(stub, RED, id)), frame("signal", await eventsText(stub, BLUE, id))];
    for (const [n, tab] of tabs.entries()) expect(await tab.until(2)).toEqual([firsts[n], chatted[n]]);
  });

  // A throw that is not a trap, while framing, is handled as a trap is: the
  // batch is committed, so the reply stands, the module is dropped (it may hold
  // what storage does not) and the failure is logged. Mutants: the throw
  // escaping `sendFrames` (the post's request fails after its batch); the
  // module kept (the GET wakes no new one).
  it("a throw while framing is logged, and the reply stands", async () => {
    const id = "frame-throw";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const [first] = await red.until(1);
    const woken = await wakes(stub);
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      mod.events = () => {
        throw new Error("the reading could not be taken");
      };
    });
    const { result: posted, lines } = await logsDuring(() => framed(() => chat(stub, BLUE, id, "one")));
    expect(posted.bumps, "premise: the post bumped the table").toEqual([[{ bump_table: id }]]);
    expect(lines).toEqual([{ event: "frame-failed", id, error: "Error: the reading could not be taken" }]);
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status).toBe(200);
    expect(await wakes(stub), "the module was dropped: the GET woke a new one").toBe(woken + 1);
    await chat(stub, BLUE, id, "two");
    const chatted = frame("signal", await eventsText(stub, RED, id));
    expect(await red.until(2)).toEqual([first, chatted]);
  });

  // A framing call only reads, and no known case makes one write. Should one
  // ever carry bumps or an archive plan, the pass neither frames nor owes
  // them, and a log line records that it happened. The live module's `events`
  // answers ONCE with a bump beside its reading. Mutant: the line dropped.
  it("a framing call that carries bumps is logged", async () => {
    const id = "carried";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const [first] = await red.until(1);
    const reading = await eventsText(stub, RED, id);
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      const events = mod.events;
      mod.events = () => {
        mod.events = events;
        return { status: 200, body: reading, bumps: JSON.stringify([{ bump: id }]), logs: "[]", free() {} } as unknown as Reply;
      };
    });
    const { lines } = await logsDuring(() => chat(stub, BLUE, id, "hm"));
    expect(lines).toEqual([{ event: "frame-carried-bumps", id, bumps: [{ bump: id }], archive: false }]);
    expect(await red.until(2)).toEqual([first, frame("signal", reading)]);
  });

  // The plan's ruling P-2: `events` can reach the layer's clock (a cold
  // handle's pump), so the shell pins the module's clock before every call to
  // it — each one the framing makes included. The live module's calls are
  // recorded in order: every `events` must follow a `set_clock` of its own.
  // Mutant: the framing's `events` called unpinned — two `events` after one
  // `set_clock`, the dispatch's.
  it("every events call the framing makes is pinned to P-2's reading", async () => {
    const id = "pinned";
    const stub = await seedPlaying(id);
    const tabs = [await openTab(stub, RED, id), await openTab(stub, BLUE, id)];
    await Promise.all(tabs.map((tab) => tab.until(1)));
    const calls: string[] = [];
    await runInDurableObject(stub, (table: Table) => {
      const { mod } = table["instance"]!.get();
      const watched = mod as unknown as Record<"set_clock" | "dispatch" | "events", (...args: unknown[]) => unknown>;
      for (const name of ["set_clock", "dispatch", "events"] as const) {
        const original = watched[name];
        watched[name] = (...args) => {
          calls.push(name);
          return original(...args);
        };
      }
    });
    await chat(stub, BLUE, id, "hello");
    expect(calls).toEqual(["set_clock", "dispatch", "set_clock", "events", "set_clock", "events"]);
  });

  // A socket this object has closed stays in `getWebSockets()` until its peer
  // answers the close. Measured: in the turn of the close it is listed, with
  // readyState 2 (CLOSING), and `send` on it THROWS ("Can't call WebSocket
  // send() after close()"). In the pool the peer answers before the next
  // request, so the cell closes one socket and frames in ONE turn, inside the
  // object. Mutant: no readyState check — the send throws, the framing ends
  // `frame-failed`, and either the open socket is framed nothing or the
  // failure is logged.
  it("a socket the object has closed is passed over", async () => {
    const id = "closing";
    const stub = await seedPlaying(id);
    const red = await openTab(stub, RED, id);
    const blue = await openTab(stub, BLUE, id);
    const [redFirst] = await red.until(1);
    const [blueFirst] = await blue.until(1);
    const { lines } = await logsDuring(() =>
      runInDurableObject(stub, (table: Table, state) => {
        const sockets = state.getWebSockets();
        const closing = sockets.find((ws) => (ws.deserializeAttachment() as { user: string }).user === RED);
        if (closing === undefined || sockets.length !== 2) throw new Error("premise: red's and blue's sockets are open");
        closing.close(4000, "closing");
        table.sendFrames(id, [{ bump_table: id }]);
      }),
    );
    expect(lines).toEqual([]);
    expect(await red.closed()).toEqual({ code: 4000, reason: "closing" });
    expect(red.messages).toEqual([redFirst]);
    expect(await blue.until(2)).toEqual([blueFirst, frame("signal", await eventsText(stub, BLUE, id))]);
  });

  // Spec §2.3's transport parity. Mutants: the 405s answered by the layer's
  // dispatch (its 404 "no such endpoint"); the 426 missing (the layer's 404
  // again); the `…/events` query dropped (its reading, 200); the
  // stream's query handed to its admission (the upgrade would be refused 400,
  // unupgraded); a path BELOW a transport route read as one (its reading,
  // 200). An `Upgrade: WebSocket` is admitted too, as RFC 6455 §4.2.1 reads
  // the token case-insensitively — but MEASURED, the runtime hands the object
  // `websocket` whatever case the caller sent, so that line holds with or
  // without the shell's own lowercase compare: a defense no cell here reaches.
  it("the transport routes answer as axum's router does", async () => {
    const id = "routes";
    const stub = await seedPlaying(id);
    for (const route of ["events", "stream"]) {
      const res = await call(stub, RED, "POST", `/api/games/${id}/${route}`, "{}");
      expect([res.status, await res.text()], `POST …/${route}`).toEqual([405, ""]);
    }
    const plain = await call(stub, RED, "GET", `/api/games/${id}/stream`);
    expect([plain.status, await plain.json()]).toEqual([426, { error: "this host streams over WebSocket" }]);
    const long = await call(stub, RED, "GET", `/api/games/${id}/events?wait=26`);
    expect([long.status, await long.json()]).toEqual([400, { error: "a request can be held for at most 25 seconds, not 26" }]);
    const bogus = await call(stub, RED, "GET", `/api/games/${id}/events?bogus=1`);
    const unknownKey = readReply(referenceModule().events(id, RED, "bogus=1"));
    expect(unknownKey.status, "premise: the layer's own 400 for an unknown key").toBe(400);
    expect([bogus.status, await bogus.text()]).toEqual([400, unknownKey.body]);
    const held = await call(stub, RED, "GET", `/api/games/${id}/events?wait=25`);
    expect([held.status, await held.text()], "a wait within the cap answers at once").toEqual([200, await eventsText(stub, RED, id)]);
    const streamed = await openTab(stub, RED, id, "?wait=26");
    expect(await streamed.until(1), "the stream reads no query").toEqual([frame("signal", await eventsText(stub, RED, id))]);
    const deeper = await call(stub, RED, "GET", `/api/games/${id}/events/more`);
    const noRoute = readReply(referenceModule().dispatch(RED, "GET", `/api/games/${id}/events/more`, "", undefined, new Uint8Array()));
    expect(noRoute.status, "premise: the layer has no such route").toBe(404);
    expect([deeper.status, await deeper.text()]).toEqual([404, noRoute.body]);
    const mixed = await stub.fetch(`https://yokohama.test/api/games/${id}/stream`, {
      headers: { Upgrade: "WebSocket", "X-Yokohama-User": RED },
    });
    expect(mixed.status, "the Upgrade token is case-insensitive").toBe(101);
    mixed.webSocket?.accept();
    mixed.webSocket?.close(1000);
  });

  // Hibernation, and battery 8. The eviction hibernates blue's socket; red's
  // move wakes the object. Mutants: the viewer kept anywhere but the
  // socket's attachment (lost at the eviction, so blue is sent nothing); the
  // frame computed for the request's caller, red; a wake counted per
  // request, or the eviction's not counted.
  it("a hibernated socket keeps its viewer and receives the next frame; wakes count the eviction", async () => {
    const id = "hibernated";
    const stub = await seedPlaying(id);
    const blue = await openTab(stub, BLUE, id);
    const [first] = await blue.until(1);
    const woken = await wakes(stub);
    await evictDurableObject(stub);
    await placeA(stub, id);
    const moved = await eventsText(stub, BLUE, id);
    expect((JSON.parse(moved) as Events).your_turn, "premise: blue is not to move").toBe(false);
    expect(moved, "premise: blue's reading is not red's").not.toBe(await eventsText(stub, RED, id));
    expect(await blue.until(2)).toEqual([first, frame("signal", moved)]);
    expect(await wakes(stub)).toBe(woken + 1);
  });

  // Review Focus 3, the object's half: a deploy closes every socket 1006 and
  // restarts every object. A stub taken before the abort is dead after it
  // (pool fact 3). Red's next move is option 1 of the prompt the placement
  // left — the President placed in Customs, measured to apply. Mutant: the
  // move's record held only in the module and never committed — the abort
  // loses it, and the new socket's first frame cannot read it.
  it("an abort with sockets open loses no acknowledged move", async () => {
    const id = "aborted";
    const before = await seedPlaying(id);
    await placeA(before, id);
    const tabs = [await openTab(before, RED, id), await openTab(before, BLUE, id)];
    await Promise.all(tabs.map((tab) => tab.until(1)));
    await abortAllDurableObjects();
    for (const tab of tabs) expect((await tab.closed()).code).toBe(1006);
    const stub = tableStub(id);
    const again = await openTab(stub, RED, id);
    const [first] = await again.until(1);
    expect(dataOf(first).events, "the acknowledged move survived").toBe(2);
    const next = await framed(() => input(stub, RED, id, "1"));
    expect([next.result.status, (await next.result.json<Play>()).events, next.bumps]).toEqual([200, 3, [[{ bump: id }]]]);
    expect(await again.until(2)).toEqual([first, frame("signal", await eventsText(stub, RED, id))]);
  });
});
