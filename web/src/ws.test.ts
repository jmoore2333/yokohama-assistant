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
// are the thing under test. The fake is played from the SERVER's side — the
// cell accepts the upgrade, says frames, and closes — and keeps the three rules
// of a browser's socket this reader leans on (see `FakeSocket`).

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Frame } from "./sse";
import { wire } from "./wire/contract";
import { PING_MS, readSocket } from "./ws";

/**
 * A `WebSocket` a cell plays from the server's side.
 *
 * It keeps three rules of a browser's socket, because each is one this reader
 * leans on and a looser fake would let a mutant through:
 * - a message is delivered only while OPEN — after `close()` a browser drops
 *   what arrives (WHATWG, "when a WebSocket message has been received": if the
 *   ready state is not OPEN, return);
 * - `close()` makes the socket CLOSING at once and fires `close` a turn later,
 *   never inside the call;
 * - an upgrade that fails fires `error` and then `close` with 1006, and nothing
 *   that says why.
 *
 * `sent` records EVERY `send` after the open, whatever the state — a browser
 * would silently discard a ping sent to a closed socket, and a keep-alive that
 * outlived its socket has to be seen. A `send` before the open throws, as a
 * browser's does.
 *
 * **It calls its listeners itself, and `thrown` keeps what one threw.** A
 * browser reports a throw out of a socket's listener as an uncaught error and
 * goes on; jsdom's `EventTarget` drops it without a trace — measured: neither
 * an `error` event on the window nor an uncaught exception — so a fake built on
 * it could not see a reader whose listener throws at every `pong`.
 */
class FakeSocket {
  static made: FakeSocket[] = [];
  readonly url: string;
  readyState = 0;
  readonly sent: string[] = [];
  /** The code THIS side hung up with — `null` while it has not. */
  closedWith: number | null = null;
  /** What a listener threw, in order. */
  readonly thrown: unknown[] = [];
  private readonly listeners = new Map<string, Set<(e: Event) => void>>();

  constructor(url: string | URL) {
    this.url = String(url);
    FakeSocket.made.push(this);
  }

  addEventListener(type: string, listener: (e: Event) => void): void {
    let set = this.listeners.get(type);
    if (!set) this.listeners.set(type, (set = new Set()));
    set.add(listener);
  }

  removeEventListener(type: string, listener: (e: Event) => void): void {
    this.listeners.get(type)?.delete(listener);
  }

  send(data: string): void {
    if (this.readyState === 0) {
      throw new DOMException("Still in CONNECTING state.", "InvalidStateError");
    }
    this.sent.push(data);
  }

  close(code?: number, reason?: string): void {
    if (this.readyState >= 2) return;
    this.closedWith = code ?? null;
    this.readyState = 2;
    queueMicrotask(() => this.closes(code ?? 1005, reason ?? "", true));
  }

  // ---- the server's side, and the network's ----------------------------

  /** The upgrade succeeds. */
  accept(): void {
    this.readyState = 1;
    this.emit(new Event("open"));
  }

  /** One message in the host's framing (spec §2.4): `{event, data}`. */
  frame(event: string, data: unknown): void {
    this.raw(JSON.stringify({ event, data }));
  }

  /** Any message at all — the auto-response's `pong`, or something malformed. */
  raw(data: unknown): void {
    if (this.readyState === 1) this.emit(new MessageEvent("message", { data }));
  }

  /** The server closes, cleanly, with a code of its own. */
  shut(code: number, reason = ""): void {
    this.closes(code, reason, true);
  }

  /** The connection is lost — or, before `accept`, the upgrade is refused. */
  drop(): void {
    if (this.readyState === 0) this.emit(new Event("error"));
    this.closes(1006, "", false);
  }

  private closes(code: number, reason: string, wasClean: boolean): void {
    if (this.readyState === 3) return;
    this.readyState = 3;
    this.emit(new CloseEvent("close", { code, reason, wasClean }));
  }

  private emit(e: Event): void {
    for (const listener of [...(this.listeners.get(e.type) ?? [])]) {
      try {
        listener(e);
      } catch (err) {
        this.thrown.push(err);
      }
    }
  }
}

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
    read("https://yokohama.moore.nyc/api/stream");
    expect(FakeSocket.made[1]!.url).toBe("wss://yokohama.moore.nyc/api/stream");
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
