// The WebSocket a cell plays from the SERVER's side — ONE fake for the socket
// reader's cells (`ws.test.ts`) and the signal's (`signal.test.ts`), hoisted
// here in M6.5c2 (M6.5c Task 8's item 3) because two fakes of the same three
// browser rules are two things that drift. This is `ws.test.ts`'s, the fuller
// of the two: it records what a listener THROWS, which jsdom's `EventTarget`
// swallows without a trace.

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
export class FakeSocket {
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

  /** The server has BEGUN its closing handshake: CLOSING, and no `close` event
   *  yet — the window a keep-alive's tick can fall into. */
  closing(): void {
    if (this.readyState === 1) this.readyState = 2;
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
