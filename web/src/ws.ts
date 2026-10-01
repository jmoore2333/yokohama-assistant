// The change signal over a WebSocket — the Worker's transport (M6.5c,
// R-6.5c.6; spec §2.7).
//
// **The stream's own frames, in another envelope.** A table object sends every
// frame as one `{"event": <name>, "data": <the SSE frame's data>}` message
// (spec §2.4, "The framing"), and this module unwraps it into the `Frame`
// `readStream` produces. `signal.ts` hands both to ONE handler, so nothing
// downstream of it knows which transport a frame came over.
//
// **What a socket cannot say, and why `signal.ts` probes.** A `WebSocket`
// carries no header of the page's choosing — neither the open door's nor
// the `X-Requested-With` that makes Cloudflare Access answer a readable 401
// (`api.ts`, `serverHeaders`) — and a browser reports a refused upgrade as an
// `error` event that says nothing and a close with 1006: "user agents must not
// convey any failure information to scripts" (WHATWG). So this reader cannot
// tell an expired session from a dropped network, and does not try. It answers
// `"closed"`, and the arm asks one plain fetch what happened.
//
// **So under the open door the name rides the URL** (M6.5c2, spec §2.1): the
// upgrade carries `claimed=<the stored name>` — `api.ts`'s `identityParams`,
// the value the fetch header carries — set through `URLSearchParams`, so a `+`
// travels as `%2B` and never becomes a space (Review Focus 1). Under the
// Access doors it carries nothing: the edge's own session is the identity.
//
// What is here is the transport and NOTHING about the signal — `sse.ts`'s rule.
// It knows the framing and the keep-alive; it does not know what a `signal`
// frame carries, and it knows of `end` only that the server has said this
// socket is over.

import { identityParams } from "./api";
import type { Frame } from "./sse";

/** How often an open socket says `ping` (spec §2.7).
 *
 *  The host's auto-response answers `pong` without waking the object (spec
 *  §2.4), and the exchange keeps the connection inside the edge's 100 s idle
 *  cut. It asks the server nothing, which is why it is not a poll. */
export const PING_MS = 30_000;

/** `WebSocket.OPEN`, as the number the standard fixes it at — so a socket
 *  double need not carry the class's statics. */
const OPEN = 1;

/** One message off the socket as the stream's `Frame` — or `null` for anything
 *  that is not a text message holding an `{event: string, data}` object. The
 *  auto-response's `pong` is the one such message this host sends. `data` is
 *  handed on as TEXT, which is what `Frame.data` is: the handlers parse it
 *  exactly as they parse the stream's. */
function unwrap(message: unknown): Frame | null {
  if (typeof message !== "string") return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(message);
  } catch {
    return null;
  }
  if (typeof parsed !== "object" || parsed === null || !("data" in parsed)) return null;
  const { event, data } = parsed as { event?: unknown; data: unknown };
  return typeof event === "string" ? { event, data: JSON.stringify(data) } : null;
}

/**
 * Open a socket on `path` — the source's own stream URL, `ws:` under `http:`
 * and `wss:` under `https:`, with the open door's claim as its `claimed`
 * parameter — and read frames from it until it closes.
 *
 * Resolves `"ended"` when an `end` frame came before the close, WHATEVER the
 * close's code: this host always sends `end` before its 4403/4404 close (spec
 * §2.4), so a close read as news of its own would end the source twice. The
 * `end` is handed to `onFrame` before the close, as `readStream` hands it
 * before it resolves, and nothing after it is handed over.
 *
 * Resolves `"closed"` on any close with no `end` before it — a dropped
 * connection, a refused upgrade, the abort below. A refused upgrade never fires
 * `onOpen`.
 *
 * `onOpen` fires once, when the socket opens: the moment the caller's "live"
 * hangs on, as it hangs on `readStream`'s 200. `ping` starts then too, and
 * stops at the hang-up or at the close, whichever comes first — and a tick
 * sends only on an OPEN socket (M6.5c's live tier, finding 2): between a
 * close BEGUN, by this side or the server's, and the `close` event the socket
 * is CLOSING, and a send there is Chromium's console error "WebSocket is
 * already in CLOSING or CLOSED state."
 *
 * **The abort hangs up with 1000**, and the read resolves once the socket has
 * closed. A signal that has already aborted opens nothing.
 *
 * **A throw out of `onFrame` or `onOpen` rejects the read with that error**,
 * as a throw out of `readStream`'s handlers does, and hangs the socket up: a
 * socket left open would go on delivering to a listener nobody is reading
 * for.
 */
export function readSocket(
  path: string,
  signal: AbortSignal,
  onFrame: (f: Frame) => void,
  onOpen?: () => void,
): Promise<"ended" | "closed"> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) {
      resolve("closed");
      return;
    }
    const url = new URL(path, location.href);
    url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
    for (const [key, value] of Object.entries(identityParams())) url.searchParams.set(key, value);
    const ws = new WebSocket(url.href);
    let ended = false;
    let ping: ReturnType<typeof setInterval> | undefined;
    const stopPing = () => {
      clearInterval(ping);
      ping = undefined;
    };
    const hangUp = () => {
      stopPing();
      ws.close(1000);
    };
    signal.addEventListener("abort", hangUp, { once: true });
    const guarded = (handler: () => void) => {
      try {
        handler();
      } catch (e) {
        reject(e);
        hangUp();
      }
    };
    ws.addEventListener("open", () => {
      ping = setInterval(() => {
        if (ws.readyState === OPEN) ws.send("ping");
      }, PING_MS);
      guarded(() => onOpen?.());
    });
    ws.addEventListener("message", (e: MessageEvent) => {
      if (ended) return;
      const f = unwrap(e.data);
      if (f === null) return;
      if (f.event === "end") ended = true;
      guarded(() => onFrame(f));
    });
    ws.addEventListener("close", () => {
      stopPing();
      signal.removeEventListener("abort", hangUp);
      resolve(ended ? "ended" : "closed");
    });
  });
}
