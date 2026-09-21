// The change stream, read through `fetch` (spec §5.1).
//
// **`EventSource` is not used, and the reason is one line long** (spec §0.1):
// it cannot carry a header. The `claimed` door is `X-Yokohama-Claimed-User` and
// Cloudflare Access is a cookie the browser would send anyway, so an
// `EventSource` would sign exactly one of the two deployments this server has.
// A `fetch` carries `serverHeaders()` like every other call, and gets an
// `AbortController` with it — which is also what a hidden tab needs, and what
// `EventSource` does not offer either.
//
// **Since M6.5b2 that header set carries `X-Requested-With` too** (spec §2.1),
// and this is one of the three sites that has to: the open is built here, and
// the refusal is parsed here, so a change made in `api.ts` alone would leave a
// reconnect after an expired Access session reporting an ordinary network
// failure. `EventSource` could not have sent that header either.
//
// What is here is the transport and NOTHING about the signal: this module knows
// `event:` and `data:` lines, and does not know that a `signal` frame carries an
// `Events` or that an `end` frame means a table was deleted. That belongs to
// `signal.ts`, which is the one place the four consumers meet.

import { ACCESS_CHALLENGE, ApiError, isAccessChallenge, serverHeaders } from "./api";

/** One frame off the wire: its name, and its payload as TEXT.
 *
 *  Text and not a parsed object, because the two frame kinds carry two
 *  different shapes and this module is not the one that knows which. */
export interface Frame {
  event: string;
  data: string;
}

/** A stateful parser over the `text/event-stream` grammar this server writes:
 *  `event:` and `data:` lines, a blank line ends a frame, and a line starting
 *  with `:` is the heartbeat comment and is dropped.
 *
 *  **Stateful because a chunk is not a frame.** A socket hands over whatever
 *  arrived, which can be half a line, so the parser holds the tail of the last
 *  chunk until a newline finishes it. A frame boundary is a blank LINE, so the
 *  buffer is consumed a line at a time and never by splitting on `\n\n`.
 *
 *  Multi-line `data:` is joined with `\n`, which is the specification's rule
 *  rather than an observed one: this server writes a single line per frame.
 *  A frame with no `data:` line at all is not emitted — there is no such frame
 *  on this API, and emitting one would hand `signal.ts` an empty body to parse.
 */
export function parseSse(): (chunk: string) => Frame[] {
  let buffer = "";
  let event = "message";
  let data: string[] = [];
  return (chunk: string) => {
    buffer += chunk;
    const out: Frame[] = [];
    let nl: number;
    while ((nl = buffer.indexOf("\n")) >= 0) {
      const line = buffer.slice(0, nl).replace(/\r$/, "");
      buffer = buffer.slice(nl + 1);
      if (line === "") {
        if (data.length) out.push({ event, data: data.join("\n") });
        event = "message";
        data = [];
      } else if (line.startsWith(":")) {
        /* the heartbeat: it keeps the connection from being dropped and says
           nothing, so it neither ends a frame nor becomes one */
      } else if (line.startsWith("event:")) {
        event = line.slice(6).trim();
      } else if (line.startsWith("data:")) {
        // One optional space after the colon is part of the framing, not of the
        // payload — `axum` writes `data: {…}` and the space is not JSON.
        data.push(line.slice(5).replace(/^ /, ""));
      }
    }
    return out;
  };
}

/**
 * Open `url` and read frames from it until it ends.
 *
 * Resolves `"ended"` when an `end` frame arrives — the server said this stream
 * is over and there is nothing to reopen — and `"closed"` when the body simply
 * stops, which is a dropped connection and IS worth reopening. Two answers and
 * not one, because only the caller knows which of those to act on and a reader
 * that collapsed them would have the ladder retry a deleted table for ever.
 *
 * Rejects with an `ApiError` when the open is refused, carrying the server's own
 * sentence where the body is this API's `{"error": …}` and the status
 * otherwise — the same rule `api.ts`'s `call` follows, and for the same
 * reason: the documented deployment answers an expired session with an HTML
 * interstitial, and "Unexpected token '<'" tells a player nothing.
 *
 * The `end` frame is handed to `onFrame` BEFORE this resolves: its body carries
 * the reason, and the caller needs it.
 *
 * `onOpen` fires ONCE, after the response is known to be a readable 200 and
 * before any frame. **A request that has gone out is not a connection**, and
 * this is the only moment at which the difference is visible from outside: the
 * caller's "live" word hangs on it, and a source that receives no frame on
 * connect — the house channel writes nothing (R-3.4) — has nothing else to
 * learn it from. A refused open never fires it.
 */
export async function readStream(
  url: string,
  signal: AbortSignal,
  onFrame: (f: Frame) => void,
  onOpen?: () => void,
): Promise<"ended" | "closed"> {
  const res = await fetch(url, {
    headers: { ...serverHeaders(), accept: "text/event-stream" },
    signal,
  });
  if (!res.ok || !res.body) {
    // **The EDGE's 401 is not this API's** (spec §2.1): the session in front of
    // the server expired, and the sentence for that is the one with the
    // turnstile's link. Read the same way `call` reads it, through the same
    // two functions, so the three sites cannot drift.
    if (res.status === 401 && isAccessChallenge(res)) throw new ApiError(ACCESS_CHALLENGE, 401);
    let message = `${res.status} ${res.statusText}`.trim();
    try {
      const parsed = (await res.json()) as { error?: unknown };
      if (parsed && typeof parsed.error === "string") message = parsed.error;
    } catch {
      /* not this API's JSON: the status is what there is to say */
    }
    throw new ApiError(message, res.status);
  }
  onOpen?.();
  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  const parse = parseSse();
  for (;;) {
    const { value, done } = await reader.read();
    if (done) return "closed";
    for (const f of parse(decoder.decode(value, { stream: true }))) {
      onFrame(f);
      if (f.event === "end") return "ended";
    }
  }
}
