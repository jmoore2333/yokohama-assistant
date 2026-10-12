// **The live replay's sends and forces** (M6.5c2 cycle D, Task 9): one of the
// five parts of `live-replay.mjs`, which alone imports it. A row sent with the
// credential to the one origin, its answer read whole, and the operator's
// force, sent with only the JSON type.

import { KEYS, ORIGIN, REQUEST_TIMEOUT_MS } from "./credential.mjs";
import { errorText, said } from "./judge.mjs";
import { urlOf } from "./scripts.mjs";

/** One row, sent with the credential and its `who`, and its answer read
 *  whole. */
export async function send(row, credential, fetcher) {
  const url = urlOf(row);
  const headers = { [KEYS[0]]: credential.id, [KEYS[1]]: credential.secret };
  if (row.who !== null) headers["X-Yokohama-Claimed-User"] = row.who;
  let body;
  if (row.body !== null) {
    body = JSON.stringify(row.body);
    headers["content-type"] = "application/json";
  }
  const res = await fetcher(url, { method: row.method, headers, body, redirect: "manual", signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) });
  const bytes = new Uint8Array(await res.arrayBuffer());
  return { status: res.status, text: new TextDecoder().decode(bytes), bytes: bytes.length, contentType: res.headers.get("content-type") };
}

/** The operator's force on each id, with only the JSON type: no credential
 *  and no claimed header. */
export async function forceAll(ids, fetcher) {
  const forced = [];
  for (const id of ids) {
    try {
      const res = await fetcher(`${ORIGIN}/api/admin/tables/${encodeURIComponent(id)}/expire?force`, {
        method: "POST",
        // The operator's POST needs application/json since security S2b (B5): a bare POST is a 400.
        headers: { "content-type": "application/json" },
        redirect: "manual",
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
      });
      const bytes = new Uint8Array(await res.arrayBuffer());
      forced.push({ id, status: res.status, said: said({ status: res.status, text: new TextDecoder().decode(bytes), bytes: bytes.length, contentType: res.headers.get("content-type") }) });
    } catch (e) {
      forced.push({ id, status: null, said: ` — no answer: ${errorText(e)}` });
    }
  }
  return forced;
}
