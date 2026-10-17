/**
 * **The framing** (spec §2.4): a table object's socket carries the SSE
 * stream's own frames, each as ONE text message `{"event": <name>, "data":
 * <the SSE frame's data>}`, so the client's socket arm unwraps it into the
 * `(event, data)` pair its stream reader already produces (spec §2.7;
 * `web/src/ws.ts`).
 *
 * Pure, and nothing here reaches a Workers API: its cells run in node
 * (`frames.unit.test.ts`).
 */

/** The SSE's event names — the only three a frame carries. */
export type FrameEvent = "signal" | "house" | "end";

/**
 * One socket message: `event` with `dataText` embedded RAW, so the data is
 * byte-identical to the SSE frame's — never parsed and re-serialized.
 */
export function frame(event: FrameEvent, dataText: string): string {
  return `{"event":${JSON.stringify(event)},"data":${dataText}}`;
}

/** The SSE's `end` data for an admission lost under a live stream — a 403 or
 *  a 404 where the reading used to answer — byte for byte (the `stream`
 *  handler in `crates/yokohama-web/src/lib.rs`). */
export const END_GONE = '{"reason":"gone"}';

/** The SSE's `end` data for a deleted table, byte for byte (the same
 *  handler). */
export const END_DELETED = '{"reason":"deleted"}';

/**
 * The `end` data for an upgrade refused 403 or 404 — a case the SSE has no
 * frame for, since axum answers that refusal unupgraded. It carries the
 * layer's sentence, which the client's `onFrame` keeps in place of its own
 * words (the orchestrator's ruling, Task 8's review); with no sentence to
 * carry it is the SSE's own `gone`.
 */
export function endAtAdmission(sentence: string | undefined): string {
  return sentence === undefined ? END_GONE : JSON.stringify({ reason: "gone", sentence });
}

/** The layer's sentence out of its error body `{"error": …}` — `undefined`
 *  for a body that is not that shape. The body is a refusal, never a record,
 *  so it may be parsed. */
export function sentenceOf(body: string): string | undefined {
  let parsed: unknown;
  try {
    parsed = JSON.parse(body);
  } catch {
    return undefined;
  }
  if (parsed === null || typeof parsed !== "object") return undefined;
  const error = (parsed as { error?: unknown }).error;
  return typeof error === "string" ? error : undefined;
}

/** The most UTF-8 bytes a WebSocket close reason may carry: one more makes
 *  `close()` THROW and leaves the socket open (pool fact 6). */
export const CLOSE_REASON_BYTES = 123;

/**
 * `sentence`, cut at a character boundary to at most 123 UTF-8 bytes. Every
 * close goes through it. At an admission the `end` frame before the close
 * carries the whole sentence; mid-stream the `end` is the SSE's own, which
 * carries none, so there the close's reason, cut, is the layer's sentence's
 * only carrier. A delete's close carries this host's own short reason.
 *
 * It walks code points, never UTF-16 units, so a surrogate pair is never
 * split. A lone surrogate counts three bytes, which is what `TextEncoder`
 * writes for it (U+FFFD).
 */
export function closeReason(sentence: string): string {
  let bytes = 0;
  let end = 0;
  for (const ch of sentence) {
    const cp = ch.codePointAt(0) ?? 0;
    const size = cp < 0x80 ? 1 : cp < 0x800 ? 2 : cp < 0x10000 ? 3 : 4;
    if (bytes + size > CLOSE_REASON_BYTES) break;
    bytes += size;
    end += ch.length;
  }
  return sentence.slice(0, end);
}

/** The close code of a socket refused for its count (security S2b, finding D3): the shape of the 4403/4404
 *  closes, one more mirror of an HTTP status. */
export const CLOSE_TOO_MANY = 4429;

/** The four sentences a cap's `end` frame carries (host-written): the caller's own count, and the object's. */
export const TOO_MANY_OWN_TABLE = "you have too many connections open to this table — close another tab and try again";
export const TOO_MANY_TABLE = "this table has as many connections as it can take — try again later";
export const TOO_MANY_OWN_LOBBY = "you have too many connections open to the lobby — close another tab and try again";
export const TOO_MANY_LOBBY = "the lobby has as many connections as it can take — try again later";
