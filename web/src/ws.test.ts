// The socket reader, alone (M6.5c, spec §2.7; R-6.5c.6).
//
// **Small, and every part of it is a mutant's target** (spec §5.7). The frame
// it hands over is the stream's own `Frame`; `pong` and anything that is not
// `{event, data}` reach nobody; the keep-alive runs only while the socket is
// open; an `end` makes the close after it an ENDING whatever its code (battery
// 38); and a close with no `end` before it is just "closed" — the answer that
// makes `signal.ts` ask its one probe (`signal.test.ts` holds those cells).
//
// The socket is FAKE and the clock is fake, for `signal.test.ts`'s reason: both
// are the thing under test. The fake (`test/fakeSocket.ts`, shared with
// `signal.test.ts`) is played from the SERVER's side — the cell accepts the
// upgrade, says frames, and closes — and keeps the three rules of a browser's
// socket this reader leans on.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { CLAIMED_USER_KEY, identityHeaders } from "./api";
import { patchServerFacts, resetFactsForTest } from "./local/facts";
import type { Frame } from "./sse";
import { FakeSocket } from "./test/fakeSocket";
import { wire } from "./wire/contract";
import { PING_MS, readSocket } from "./ws";

beforeEach(() => {
  vi.useFakeTimers();
  FakeSocket.made = [];
  vi.stubGlobal("WebSocket", FakeSocket);
});
afterEach(() => {
  vi.useRealTimers();
});

/** One read over a fresh socket, and everything it handed over. */
function read(path = "/api/games/wire/stream", signal = new AbortController().signal) {
  const frames: Frame[] = [];
  let opens = 0;
  const done = readSocket(path, signal, (f) => frames.push(f), () => {
    opens += 1;
  });
  return { frames, done, opens: () => opens, socket: () => FakeSocket.made.at(-1)! };
}

describe("the socket reader", () => {
  it("opens the source's own stream URL, over ws: under http: and wss: under https:", () => {
    // The premise: the page this suite runs on is plain http.
    expect(location.protocol).toBe("http:");
    read("/api/games/wire/stream");
    expect(FakeSocket.made[0]!.url).toBe(`ws://${location.host}/api/games/wire/stream`);
    read("https://example.test/api/stream");
    expect(FakeSocket.made[1]!.url).toBe("wss://example.test/api/stream");
  });

  it("hands a message over as the stream's own frame: the event, and the data as text", () => {
    const r = read();
    r.socket().accept();
    expect(r.opens()).toBe(1);
    r.socket().frame("signal", wire.events);
    expect(r.frames).toEqual([{ event: "signal", data: JSON.stringify(wire.events) }]);
    // What a handler does with it is what it does with the stream's.
    expect(JSON.parse(r.frames[0]!.data)).toEqual(wire.events);
  });

  it("drops pong, and every message that is not an {event, data} object, without a throw", () => {
    const r = read();
    const s = r.socket();
    s.accept();
    for (const m of [
      "pong", // the auto-response to `ping` (spec §2.4)
      "not json",
      "null",
      "[1,2]",
      '"signal"',
      JSON.stringify({ event: 3, data: {} }),
      JSON.stringify({ data: {} }),
      JSON.stringify({ event: "signal" }),
    ]) {
      s.raw(m);
    }
    s.raw(new ArrayBuffer(4));
    expect(r.frames).toEqual([]);
    // DROPPED, and not thrown over: a throw out of a socket's listener is no
    // rejection anybody holds — it is an uncaught error on the page, once per
    // `pong`, every thirty seconds.
    expect(s.thrown).toEqual([]);
    // And the reader is still reading: a well-formed frame after all of them
    // arrives, so the silence above is the drops' and not a dead socket's.
    // (The lobby's frame, `{seq, chat}` — spec §2.5.)
    s.frame("house", { seq: 1, chat: 0 });
    expect(r.frames).toEqual([{ event: "house", data: '{"seq":1,"chat":0}' }]);
  });

  it("pings every 30 s from the open, and never before it or after the close", async () => {
    expect(PING_MS).toBe(30_000);
    const r = read();
    const s = r.socket();
    await vi.advanceTimersByTimeAsync(PING_MS * 2);
    expect(s.sent).toEqual([]);
    s.accept();
    await vi.advanceTimersByTimeAsync(PING_MS - 1);
    expect(s.sent).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(s.sent).toEqual(["ping"]);
    await vi.advanceTimersByTimeAsync(PING_MS);
    expect(s.sent).toEqual(["ping", "ping"]);
    s.shut(1000);
    await r.done;
    // The keep-alive went with the socket.
    expect(vi.getTimerCount()).toBe(0);
    await vi.advanceTimersByTimeAsync(PING_MS * 3);
    expect(s.sent).toEqual(["ping", "ping"]);
  });

  // **The live tier's finding 2** (M6.5c Task 15; ruling I-81). Chromium logs
  // "WebSocket is already in CLOSING or CLOSED state." for a `send` on a socket
  // that is closing — the server's close at game over, or this side's own
  // hang-up — and the keep-alive's tick can land in that window: the live run
  // logged it twice on one page. This fake records every `send` whatever the
  // state, so a ping into the window is seen.
  it("sends no ping to a socket that is CLOSING, whoever began the close", async () => {
    const r = read();
    const s = r.socket();
    s.accept();
    s.closing();
    await vi.advanceTimersByTimeAsync(PING_MS);
    expect(s.sent).toEqual([]);
    s.shut(1000);
    await r.done;
  });

  it("stops the keep-alive the moment it hangs up, before the close arrives", async () => {
    const abort = new AbortController();
    const r = read("/api/games/wire/stream", abort.signal);
    const s = r.socket();
    s.accept();
    expect(vi.getTimerCount(), "the premise: the keep-alive is armed").toBe(1);
    abort.abort();
    // CLOSING now, and the close event a turn away: nothing may tick between.
    expect(s.readyState).toBe(2);
    expect(vi.getTimerCount(), "hanging up stopped it at once").toBe(0);
    await r.done;
    expect(s.sent).toEqual([]);
  });

  // **Battery 38.** This host always sends `end` BEFORE its 4403/4404 close
  // (spec §2.4), so a reader that read the close as news of its own would end
  // the source twice. And a close after an `end` is an ending WHATEVER its code
  // — a dropped connection after the server said "over" is still over.
  it("resolves ENDED on an end frame whatever the close after it says, and hands the end over once", async () => {
    for (const close of [
      (s: FakeSocket) => s.shut(4404, 'no game named "wire"'),
      (s: FakeSocket) => s.shut(4403, "this table is locked — unlock it first"),
      (s: FakeSocket) => s.shut(1000),
      (s: FakeSocket) => s.drop(),
    ]) {
      const r = read();
      const s = r.socket();
      s.accept();
      s.frame("end", { reason: "gone" });
      // Nothing after an `end` reaches anyone — the stream's reader stops
      // reading at one, and this one stops handing over.
      s.frame("signal", wire.events);
      s.frame("end", { reason: "gone" });
      close(s);
      await expect(r.done).resolves.toBe("ended");
      expect(r.frames).toEqual([{ event: "end", data: '{"reason":"gone"}' }]);
    }
  });

  // The other half, and the answer that earns the probe: with no `end` before
  // it, a close says nothing about why — whatever its code.
  it("resolves CLOSED when the socket closes with no end before it, whatever the code", async () => {
    for (const close of [
      (s: FakeSocket) => s.drop(),
      (s: FakeSocket) => s.shut(1000),
      (s: FakeSocket) => s.shut(4404, 'no game named "wire"'),
    ]) {
      const r = read();
      const s = r.socket();
      s.accept();
      s.frame("signal", wire.events);
      close(s);
      await expect(r.done).resolves.toBe("closed");
      expect(r.frames).toHaveLength(1);
    }
  });

  it("resolves CLOSED, and never says it opened, when the upgrade is refused", async () => {
    const r = read();
    // What a refused upgrade is from inside a page: `error`, then 1006.
    r.socket().drop();
    await expect(r.done).resolves.toBe("closed");
    expect(r.opens()).toBe(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it("hangs up with 1000 when its signal aborts, and resolves closed once the socket has closed", async () => {
    const abort = new AbortController();
    const r = read(undefined, abort.signal);
    const s = r.socket();
    s.accept();
    abort.abort();
    expect(s.closedWith).toBe(1000);
    await expect(r.done).resolves.toBe("closed");
    expect(vi.getTimerCount()).toBe(0);
  });

  it("opens nothing for a signal that has already aborted", async () => {
    const abort = new AbortController();
    abort.abort();
    const r = read(undefined, abort.signal);
    await expect(r.done).resolves.toBe("closed");
    expect(FakeSocket.made).toHaveLength(0);
  });

  // `readStream`'s own rule, kept: a throw out of a caller's handler rejects the
  // read with that error. A socket then has one more thing to do that a body
  // does not — it would go on delivering to a listener nobody is reading for —
  // so the reader hangs it up.
  it("ends the read when a handler throws: rejected with that error, and the socket hung up", async () => {
    const boom = new Error("the consumer threw");
    const frames: Frame[] = [];
    const onFrameThrows = readSocket("/api/stream", new AbortController().signal, (f) => {
      frames.push(f);
      throw boom;
    });
    const s = FakeSocket.made[0]!;
    s.accept();
    s.frame("signal", wire.events);
    // Carried to the read, and not left to escape the listener.
    expect(s.thrown).toEqual([]);
    await expect(onFrameThrows).rejects.toBe(boom);
    expect(s.closedWith).toBe(1000);
    s.frame("signal", wire.events);
    expect(frames).toHaveLength(1);

    const onOpenThrows = readSocket(
      "/api/stream",
      new AbortController().signal,
      () => {},
      () => {
        throw boom;
      },
    );
    const t = FakeSocket.made[1]!;
    t.accept();
    expect(t.thrown).toEqual([]);
    await expect(onOpenThrows).rejects.toBe(boom);
    expect(t.closedWith).toBe(1000);
  });
});

// M6.5c2, spec §2.1: a browser puts no header on a WebSocket upgrade, so under
// the OPEN door the stored name rides the upgrade's URL as `claimed=` —
// encoded by `URLSearchParams`, so a `+` is `%2B` and a space `+`, and the
// Worker's `searchParams.get` reads back the very name the fetch header
// carries. Under any other door, nothing: the edge's own session is the
// identity there. Mutants: the name concatenated raw (`a+b` reads back
// `a b`); the parameter under every door; a second statement of the stored
// name that could drift from the header's.
describe("the open door's claim on the upgrade (M6.5c2; Review Focus 1)", () => {
  afterEach(() => {
    localStorage.removeItem(CLAIMED_USER_KEY);
    resetFactsForTest();
  });

  it.each([
    ["a+b@x.test", "?claimed=a%2Bb%40x.test"],
    ["ben castle", "?claimed=ben+castle"],
  ])("(Review Focus 1) carries %j as the fetch header's own value, URLSearchParams-encoded", (name, search) => {
    localStorage.setItem(CLAIMED_USER_KEY, name);
    patchServerFacts({ identity: "open" });
    read("/api/games/wire/stream");
    const url = new URL(FakeSocket.made.at(-1)!.url);
    expect(url.search).toBe(search);
    expect(url.searchParams.get("claimed")).toBe(name);
    expect(url.searchParams.get("claimed"), "the value the fetch carries").toBe(identityHeaders()["x-yokohama-claimed-user"]);
  });

  it("carries no claim under any other door, nor under the open door with no stored name", () => {
    localStorage.setItem(CLAIMED_USER_KEY, "ben");
    for (const identity of ["cf-access", "worker", ""]) {
      patchServerFacts({ identity });
      read("/api/stream");
      expect(FakeSocket.made.at(-1)!.url, JSON.stringify(identity)).toBe(`ws://${location.host}/api/stream`);
    }
    localStorage.removeItem(CLAIMED_USER_KEY);
    patchServerFacts({ identity: "open" });
    read("/api/stream");
    expect(FakeSocket.made.at(-1)!.url).toBe(`ws://${location.host}/api/stream`);
  });
});
