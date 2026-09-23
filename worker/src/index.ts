import { Archive, ARCHIVE_PREFIX } from "./archive";
import { shellConfig } from "./config";
import type { Env } from "./env";
import { House } from "./house";
import { forwardable, identityConfig, keySource, resolveIdentity, type Identity, type IdentityConfig, type KeySource } from "./identity";
import { errorBody, log, respond } from "./respond";
import { routableName, route, type Target } from "./route";
import { Table, TABLE_ID_HEADER, USER_HEADER } from "./table";

export { Archive, House, Table };

/** The largest body the Worker forwards: 2 MiB, the NAS adapter's own bound
 *  (`axum::body::to_bytes(body, 2 << 20)`). */
export const BODY_LIMIT = 2 * 1024 * 1024;

/** A body over the bound, in the NAS's own words — read off the NAS on
 *  2026-09-23: a 2,097,153-byte body answered this, and 2,097,152 passed. */
export const BODY_LIMIT_SENTENCE = "the body could not be read: length limit exceeded";

/** Every client header whose name starts so is removed before a request is
 *  forwarded (spec §2.3's strip rule): these are the names the objects
 *  trust. */
const STRIPPED = "x-yokohama-";

const NO_ONE: Identity = { kind: "none" };

/**
 * **The Worker** (spec §2.3): stateless, and in this order —
 *
 * 1. `route`. A path the asset layer had no file for answers 404 with an
 *    EMPTY body — never `index.html`; any method but GET or HEAD on `/enter`
 *    or `/api/art` is 405 with an empty body, as axum's router answers; and
 *    `/enter` is the turnstile's 302 to `/#/lobby`.
 * 2. `shellConfig`: a configuration it refuses answers every remaining
 *    request — every `/api/*` — 500 naming the variable (spec §2.6).
 * 3. The body, read ONCE for a method that carries one, under a streaming
 *    bound: over `BODY_LIMIT` it answers 400 with the NAS's sentence. What is
 *    forwarded is the bytes it bounded.
 * 4. `/api/art`: the static manifest's bytes, as JSON.
 * 5. The caller: Access's assertion, verified (`resolveIdentity`). A key set
 *    that cannot be had is no caller, so the request fails CLOSED.
 * 6. The strip rule: a fresh `Headers` without any header named
 *    `x-yokohama-…`, and `X-Yokohama-User` set when there is a caller.
 * 7. The object the target names, handed the request with its method, those
 *    headers and those bytes. An upgrade's own headers ride along, and the
 *    object's 101 is returned as it came.
 *
 * `keys` is the team's key set (`keySource`): the default export's ONE per
 * isolate, or a test's. `_ctx` is the runtime's execution context: nothing
 * here outlives the reply, so nothing reads it, and it is taken so a pool cell
 * calls `handle` exactly as the runtime's `fetch` does.
 */
export async function handle(req: Request, env: Env, _ctx: ExecutionContext, keys: KeySource): Promise<Response> {
  const url = new URL(req.url);
  const target = route(req.method, url.pathname);
  if (target.kind === "missing") return new Response(null, { status: 404 });
  if (target.kind === "not-allowed") return new Response(null, { status: 405 });
  if (target.kind === "enter") return new Response(null, { status: 302, headers: { Location: "/#/lobby" } });

  const config = shellConfig(env);
  if ("error" in config) return respond(500, errorBody(config.error));

  // GET and HEAD carry no body: the Fetch API refuses one on either.
  const carries = req.method !== "GET" && req.method !== "HEAD";
  const body = carries ? await bounded(req.body) : null;
  if (body === undefined) return respond(400, errorBody(BODY_LIMIT_SENTENCE));

  if (target.kind === "art") return art(req, url, env);

  let identity = await identify(req.headers, keys, identityConfig(env, config.devDoor, Math.floor(Date.now() / 1000)));
  if (identity.kind !== "none" && !forwardable(identity.email)) identity = NO_ONE;

  const headers = new Headers();
  for (const [name, value] of req.headers) {
    if (!name.toLowerCase().startsWith(STRIPPED)) headers.append(name, value);
  }
  if (identity.kind !== "none") headers.set(USER_HEADER, identity.email);

  return objectFor(target, env, identity, body, headers).fetch(new Request(req.url, { method: req.method, headers, body }));
}

/**
 * The caller, or no one. A key set that cannot be had, or an assertion that
 * throws while it is read, is no one, and is logged (`identity-failed`). An
 * assertion naming a `kid` the cached set lacks asks for the keys once more,
 * and is resolved ONCE more — never a loop: the key source decides whether
 * that ask may fetch (once a minute), so a forged `kid` cannot force a fetch
 * per request.
 */
async function identify(headers: Headers, keys: KeySource, config: IdentityConfig | null): Promise<Identity> {
  if (config === null) return NO_ONE;
  try {
    let found = await resolveIdentity(headers, await keys(false), config);
    if (found.kind === "none" && found.unknownKid) found = await resolveIdentity(headers, await keys(true), config);
    return found.kind === "none" ? NO_ONE : found;
  } catch (e) {
    log("identity-failed", { error: String(e) });
    return NO_ONE;
  }
}

/** The body's bytes, read once as they arrive — or `undefined` the moment
 *  they pass `BODY_LIMIT`, whatever length the request declared. */
async function bounded(stream: ReadableStream<Uint8Array> | null): Promise<Uint8Array | undefined> {
  if (stream === null) return new Uint8Array(0);
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    length += value.byteLength;
    if (length > BODY_LIMIT) {
      await reader.cancel().catch(() => {});
      return undefined;
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(length);
  let at = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, at);
    at += chunk.byteLength;
  }
  return bytes;
}

/** `GET|HEAD /api/art`: the static manifest's bytes as JSON (spec §4 — the
 *  axum route re-serializes the same manifest, so the two agree by value). */
async function art(req: Request, url: URL, env: Env): Promise<Response> {
  const asset = await env.ASSETS.fetch(new URL("/art/manifest.json", url));
  if (req.method === "HEAD") await asset.body?.cancel();
  const headers: Record<string, string> = asset.status === 200 ? { "content-type": "application/json" } : {};
  return new Response(req.method === "HEAD" ? null : asset.body, { status: asset.status, headers });
}

/** A body's `id` when it is a JSON object whose `id` is a string (spec
 *  §2.3's create row); otherwise `undefined`. The body is parsed for this one
 *  member and forwarded as the bytes it was: nothing here re-serializes it.
 *  Bytes that are not UTF-8, or that open with a byte-order mark, name no id —
 *  the layer's own parse refuses both. */
function createId(body: Uint8Array | null): string | undefined {
  if (body === null) return undefined;
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(body));
  } catch {
    return undefined;
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return undefined;
  const id = (parsed as { id?: unknown }).id;
  return typeof id === "string" ? id : undefined;
}

/** `sim-` and 16 hex digits from `crypto.getRandomValues`: a create's name
 *  when its body names none — drawn independently of any seed, as the
 *  layer's own default id is. */
function drawName(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  return `sim-${[...bytes].map((b) => b.toString(16).padStart(2, "0")).join("")}`;
}

/** The longest address the layer accepts (`User::parse`), and so the longest
 *  an archive object is ever named by. */
const ADDRESS_MAX_BYTES = 254;

/**
 * The object a target reaches — ALWAYS by name (`idFromName`; pool fact 8):
 * an object reached any other way would have no `ctx.id.name`, and `meta`
 * stores no id.
 *
 * Every public method of an object class is an RPC method on its stub —
 * `Table.sendFrames` among them. The three namespaces are bound by this
 * Worker's own configuration, and this function hands a stub nothing but
 * `fetch`.
 */
function objectFor(
  target: Exclude<Target, { kind: "missing" | "not-allowed" | "enter" | "art" }>,
  env: Env,
  identity: Identity,
  body: Uint8Array | null,
  headers: Headers,
): { fetch(req: Request): Promise<Response> } {
  const house = () => env.HOUSE.get(env.HOUSE.idFromName("house"));
  switch (target.kind) {
    case "create": {
      // A named id the layer can never use is the house's, whose `create`
      // refuses it as the layer does (the orchestrator's ruling, Task 9's
      // review): the name never makes a table object.
      const named = createId(body);
      if (named !== undefined && !routableName(named)) return house();
      const name = named ?? drawName();
      // Percent-encoded (spec §2.3), so the name travels as ASCII whatever
      // it holds; the table object decodes it.
      headers.set(TABLE_ID_HEADER, encodeURIComponent(name));
      return env.TABLE.get(env.TABLE.idFromName(name));
    }
    case "table":
      return env.TABLE.get(env.TABLE.idFromName(target.name));
    case "archive":
      // With no caller — or an address longer than the layer accepts, which
      // it refuses — the house's `dispatch` answers the layer's 401. So no
      // archive object's name passes 262 bytes, far inside the 1,024 past
      // which the platform would give it none.
      if (identity.kind === "none" || new TextEncoder().encode(identity.email).length > ADDRESS_MAX_BYTES) return house();
      return env.ARCHIVER.get(env.ARCHIVER.idFromName(ARCHIVE_PREFIX + identity.email));
    case "admin":
      // The operator surface is Task 14's (spec §2.9). Until it lands every
      // `/api/admin` path is the house's `dispatch`, which answers the layer's
      // own 404 — spec §2.9's answer with `YOKOHAMA_WEB_ADMINS` absent.
      return house();
    case "house":
      return house();
  }
}

/** ONE key source per isolate, made at the first request. */
let source: KeySource | undefined;

export default {
  fetch: (req, env, ctx) => handle(req, env, ctx, (source ??= keySource(env.ACCESS_TEAM_DOMAIN ?? ""))),
} satisfies ExportedHandler<Env>;
