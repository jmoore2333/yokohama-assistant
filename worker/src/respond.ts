/**
 * **How the shell answers and logs** — the Worker, the table, the house and
 * the archive object alike, so every reply the shell writes itself is in the
 * layer's shape.
 */

/** A reply as the plan's Task 9 writes it (step 7): a body is JSON, and an
 *  empty one is no body. */
export function respond(status: number, body: string): Response {
  return new Response(body || null, { status, headers: body ? { "content-type": "application/json" } : {} });
}

/** The layer's error shape, `{"error": …}`, for a refusal the shell answers
 *  itself: a sentence spec §4 lists as host-written, or a configuration's. */
export function errorBody(sentence: string): string {
  return JSON.stringify({ error: sentence });
}

/** One line of the shell's log (spec §2.8): what the layer cannot log on this
 *  host. */
export function log(event: string, fields: Record<string, unknown> = {}): void {
  console.log(JSON.stringify({ event, ...fields }));
}
