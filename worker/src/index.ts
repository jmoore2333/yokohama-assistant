import { ADMIN_NEEDS_JSON, adminRoute, isJsonEssence, NOT_AN_OPERATOR, OPERATOR_HEADER, operatorGate } from "./admin";
import { Archive, ARCHIVE_PREFIX } from "./archive";
import { shellConfig } from "./config";
import type { Env } from "./env";
import { House } from "./house";
import {
  callerName,
  CLAIMED_PARAM,
  CLAIMED_USER_HEADER,
  claimedIdentity,
  identityConfig,
  keySource,
  resolveIdentity,
  type Identity,
  type IdentityConfig,
  type KeySource,
} from "./identity";
import { limited } from "./limits";
import { errorBody, log, respond } from "./respond";
import { isSafeId, route, type Target } from "./route";
import { HOUSE_NAME } from "./slot";
import { isUpgrade, Table, TABLE_ID_HEADER, USER_HEADER } from "./table";

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

/** The 403 an upgrade from another site's page answers (host-written; the
 *  orchestrator's ruling on Task 11's review, escalation 1). */
const FOREIGN_SOCKET = "this socket is for the site's own pages";

/** The methods a browser sends without a preflight that only READ: `crossSite` never refuses one. */
const SAFE_METHODS: ReadonlySet<string> = new Set(["GET", "HEAD"]);

/** The 403 an unsafe method from another site answers (security S2b, finding B5; host-written). */
export const CROSS_SITE = "this request is for the site's own pages";

/**
 * **Whether an unsafe request is another site's** (security S2b, finding B5; decision D-9). Every layer POST
 * parses `application/json` by MIME essence, so a cross-site page cannot reach one without a preflight nobody
 * answers; the operator's expire and force were the shell's own, answered on method, path and query alone, and
 * a bodyless, form or text/plain POST is a CORS simple request a page sends with ambient authentication — the
 * `CF_Authorization` cookie, or the owner's WARP identity, which no cookie attribute touches. So, for every method
 * but GET and HEAD: an `Origin` present and not this origin — `null` included, a sandboxed frame's or a
 * redirected sender's — or a `Sec-Fetch-Site` of `cross-site` or `same-site` (a sibling `*.moore.nyc` host) is
 * refused. A request with no `Origin` and no `Sec-Fetch-Site` is not a page's — curl, Node's `fetch`, the live
 * replay, Playwright's request context — and passes; a page's own `fetch` carries this origin and `same-origin`.
 * Read BEFORE identity, like the upgrade's own `Origin` check above it: a refused sender learns nothing of who the
 * cookie names, and a sender that is both another site's and nobody reads this sentence (the order pin).
 */
export function crossSite(method: string, headers: Headers, origin: string): boolean {
  if (SAFE_METHODS.has(method)) return false;
  const sent = headers.get("Origin");
  if (sent !== null && sent !== origin) return true;
  const site = headers.get("Sec-Fetch-Site");
  return site === "cross-site" || site === "same-site";
}

/** The 500 of an object whose `fetch` rejected — ONE host-written sentence for
 *  the table, the house and the archive alike (the orchestrator's F-1). */
export const OBJECT_FAILED = "the server could not answer — try again";

const NO_ONE: Identity = { kind: "none" };

/** The two headers every answer under `/api` carries (security S2a, finding B9's Worker half; the S1 stack
 *  review's DC-F4). A `_headers` block never applies to a Worker-generated answer — Cloudflare's rule for static
 *  assets — so the shell sets them itself, ON THE WAY OUT, for its own answers and the objects' alike:
 *  `no-store`, so no cache in reach keeps an API answer — the browser's, a proxy's, the back/forward cache: a
 *  routed request reaches this Worker BEFORE the zone's cache, which applies to the Worker's own `fetch`
 *  subrequests and never to the answer it returns (Cloudflare's rule; the Workers Cache (static-asset requests
 *  included) is the one setting that could keep a frame, and the guard's check 10 refuses it), while the NAS's one
 *  `Router::layer` says the same for the host where the zone's extension rule DID keep a private frame at the
 *  edge (S1c, B9); `nosniff`, so an API body is never read as a script. The whole set the static release carries (`_headers`' `/*` block) buys
 *  nothing on a JSON body, and the shell holds no copy of it (decision D-3). */
export const API_HEADERS: ReadonlyArray<readonly [string, string]> = [
  ["Cache-Control", "no-store"],
  ["X-Content-Type-Options", "nosniff"],
];

/** Whether `pathname` is the API's — `/api` exactly, or under `/api/` — and never `startsWith("/api")`, which
 *  would claim `/apiary` (the NAS's layer and `swRoute.ts` draw the same line). */
export function isApiPath(pathname: string): boolean {
  return pathname === "/api" || pathname.startsWith("/api/");
}

/** `res` with [`API_HEADERS`] set — a fresh `Response` over the same body and status, since an object's answer
 *  may be immutable — or `res` ITSELF for a 101, which carries its socket, cannot be rebuilt (the constructor
 *  refuses the status) and is sent nothing a cache could keep. */
export function withApiHeaders(res: Response): Response {
  if (res.status === 101) return res;
  const headers = new Headers(res.headers);
  for (const [name, value] of API_HEADERS) headers.set(name, value);
  return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
}

/** The Worker's entry: [`answer`]'s reply, with the API's two headers on every answer under `/api`. */
export async function handle(req: Request, env: Env, ctx: ExecutionContext, keys: KeySource): Promise<Response> {
  const res = await answer(req, env, ctx, keys);
  return isApiPath(new URL(req.url).pathname) ? withApiHeaders(res) : res;
}

/**
 * **The Worker** (spec §2.3), wrapped by [`handle`]: stateless, and in this order —
 *
 * 1. `route`. A path the asset layer had no file for answers 404 with an
 *    EMPTY body — never `index.html`; any method but GET or HEAD on `/enter`
 *    or `/api/art` is 405 with an empty body, as axum's router answers; and
 *    `/enter` is the turnstile's 302 to `/#/lobby`.
 * 1a. An upgrade whose `Origin` is present and not this site's own is 403
 *    `this socket is for the site's own pages`, unupgraded (the
 *    orchestrator's ruling on Task 11's review, escalation 1). A WebSocket is
 *    outside CORS, so without it a hostile page in a signed-in player's
 *    browser could open an authenticated socket and read its frames. A
 *    request with no `Origin` — curl, the deploy readings — is not a page's,
 *    and passes.
 * 1b. Any method but GET or HEAD from another site's page is 403 `this
 *    request is for the site's own pages`, read BEFORE identity (security
 *    S2b, finding B5; `crossSite`): an `Origin` present and not this site's
 *    own — `null` included — or a `Sec-Fetch-Site` of `cross-site` or
 *    `same-site`. A bodyless, form or text/plain POST is a CORS simple
 *    request a hostile page could send with a player's ambient
 *    authentication, and the operator's expire was answered on method, path
 *    and query alone. A request with neither header — curl, the live replay —
 *    is not a page's, and passes; a refused sender's assertion is never
 *    verified.
 * 2. `shellConfig`: a configuration it refuses answers every remaining
 *    request — every `/api/*` — 500 naming the variable (spec §2.6).
 * 3. The body, read ONCE for a method that carries one, under a streaming
 *    bound: over `BODY_LIMIT` it answers 400 with the NAS's sentence. What is
 *    forwarded is the bytes it bounded.
 * 4. `/api/art`: the static art manifest projected onto the nine maps the
 *    layer's `ArtManifest` declares, as JSON (`art`).
 * 5. The caller, by the door (M6.5c2, spec §2.1). Under `worker`: Access's
 *    assertion, verified (`resolveIdentity`), whose address was judged raw
 *    and then trimmed and ASCII-lowercased as the layer reads one. No
 *    assertion asks nothing of the key source; a key set that cannot be had
 *    is no caller, so the request fails CLOSED. Under `open`: no assertion is
 *    read and no key is asked for; the caller is the name claimed —
 *    `X-Yokohama-Claimed-User` on a `fetch`, the `claimed` query parameter on
 *    an upgrade, where a browser can put no header, and never the other
 *    carrier — held to what a header can carry (`claimedIdentity`).
 * 5a. The rate limit (security S2b, finding C2; decision D-3; `limited`):
 *    any method but GET or HEAD spends a Rate Limiting binding — the
 *    creates' for `POST /api/games`, the writes' for every other — counted
 *    under the caller's digest at the `worker` door, and at the `open` door
 *    under the connecting IP for a create and IP plus claim for a write; a
 *    request with no caller under its IP at either. Past the limit it is 429
 *    `too many requests from you just now — wait a moment and try again`,
 *    with `Retry-After` the refused kind's own window, and no object is
 *    reached. A limiter that throws is logged `limiter-failed` and the
 *    request proceeds: the limit bounds cost, never access (AD-6).
 * 6. The strip rule: a fresh `Headers` without any header named
 *    `x-yokohama-…`, and the caller set, when there is one, as the ONE header
 *    its door's objects read (`forwardedCaller`): `X-Yokohama-User` for a
 *    caller the Worker verified, `X-Yokohama-Claimed-User` for a claim.
 * 6a. `/api/admin` and below — the operator surface (spec §2.9) — pass the
 *    gate (`operatorGate`): under the open door, or with
 *    `YOKOHAMA_WEB_ADMINS` absent, they are unrouted `/api/*`, the house's
 *    (M6.5c2: a claimed name is never an operator's rights); a caller who is
 *    not a PERSON on the list is 403 `this address is not an operator here`,
 *    a dev-door identity among them whatever address it claims; a listed
 *    person passes with `X-Yokohama-Operator` set to their address, and the
 *    route names the object (`adminRoute`): the listing is the house's, an
 *    expire the table's, `?force` given a value the 400 `adminRoute` names
 *    (the orchestrator's ruling I-72), an expire whose type is not
 *    `application/json` the 400 `ADMIN_NEEDS_JSON`, in the layer's own words
 *    (security S2b, finding B5), and any other path or query the house's —
 *    the layer's own 404.
 * 7. The object the target names, handed the request with its method, those
 *    headers and those bytes. An upgrade's own headers ride along, and the
 *    object's 101 is returned as it came. An object whose `fetch` REJECTS —
 *    a throw that is not a trap: a reply the shell cannot read, a batch that
 *    did not commit, a reset under `blockConcurrencyWhile` — answers 500
 *    `the server could not answer — try again` in the API's error shape,
 *    logged `object-failed {target, path, error}`, where the platform's own
 *    plain 500 answered before, which a client cannot read as `{"error": …}`
 *    (the orchestrator's F-1). A trap is not such a throw: each object
 *    answers its own trap 500.
 *
 * `keys` is the team's key set (`keySource`): the default export's ONE per
 * isolate, or a test's. `_ctx` is the runtime's execution context: nothing
 * here outlives the reply, so nothing reads it, and it is taken so a pool cell
 * calls `handle` exactly as the runtime's `fetch` does.
 */
async function answer(req: Request, env: Env, _ctx: ExecutionContext, keys: KeySource): Promise<Response> {
  const url = new URL(req.url);
  const target = route(req.method, url.pathname);
  if (target.kind === "missing") return new Response(null, { status: 404 });
  if (target.kind === "not-allowed") return new Response(null, { status: 405 });
  if (target.kind === "enter") return new Response(null, { status: 302, headers: { Location: "/#/lobby" } });
  const origin = req.headers.get("Origin");
  if (isUpgrade(req.headers) && origin !== null && origin !== url.origin) {
    return respond(403, errorBody(FOREIGN_SOCKET));
  }
  if (crossSite(req.method, req.headers, url.origin)) return respond(403, errorBody(CROSS_SITE));

  const config = shellConfig(env);
  if ("error" in config) return respond(500, errorBody(config.error));

  // GET and HEAD carry no body: the Fetch API refuses one on either.
  const carries = req.method !== "GET" && req.method !== "HEAD";
  const body = carries ? await bounded(req.body) : null;
  if (body === undefined) return respond(400, errorBody(BODY_LIMIT_SENTENCE));

  if (target.kind === "art") return art(req, url, env);

  const identity =
    config.door === "open"
      ? claimedIdentity(isUpgrade(req.headers) ? url.searchParams.get(CLAIMED_PARAM) : req.headers.get(CLAIMED_USER_HEADER))
      : await identify(req.headers, keys, identityConfig(env, config.devDoor, Math.floor(Date.now() / 1000)));

  const headers = new Headers();
  for (const [name, value] of req.headers) {
    if (!name.toLowerCase().startsWith(STRIPPED)) headers.append(name, value);
  }
  const who = callerName(identity);
  if (who !== undefined) headers.set(identity.kind === "claimed" ? CLAIMED_USER_HEADER : USER_HEADER, who);

  // The rate limit (security S2b, finding C2; decision D-3): after the caller is known, before any object. A
  // limiter that throws is logged and the request proceeds: the limit bounds cost, never access (AD-6) — a binding
  // that rejected would otherwise reject `handle`, the platform's own error page, and nothing logged (measured).
  let refused: Response | undefined;
  try {
    refused = await limited(req, target, identity, config.door, config.limits);
  } catch (e) {
    log("limiter-failed", { error: String(e) });
  }
  if (refused !== undefined) return refused;

  let forwarded: Forwarded;
  if (target.kind === "admin") {
    const gate = operatorGate(config.door, config.admins, identity);
    if (gate.kind === "refused") return respond(403, errorBody(NOT_AN_OPERATOR));
    forwarded = { kind: "house" };
    if (gate.kind === "operator") {
      headers.set(OPERATOR_HEADER, gate.email);
      const admin = adminRoute(req.method, url.pathname, url.search.slice(1));
      if (admin.kind === "refused") return respond(400, errorBody(admin.sentence));
      if (admin.kind === "expire" && !isJsonEssence(req.headers.get("content-type"))) return respond(400, errorBody(ADMIN_NEEDS_JSON));
      if (admin.kind === "expire") forwarded = { kind: "table", name: admin.name };
    }
  } else {
    forwarded = target;
  }

  const object = objectFor(forwarded, env, identity, body, headers);
  try {
    return await object.fetch(new Request(req.url, { method: req.method, headers, body }));
  } catch (e) {
    log("object-failed", { target: forwarded.kind, path: url.pathname, error: String(e) });
    return respond(500, errorBody(OBJECT_FAILED));
  }
}

/** A target `handle` forwards to an object: every route but the Worker's own
 *  answers, with an operator's route resolved to the house or a table. */
type Forwarded = Exclude<Target, { kind: "missing" | "not-allowed" | "enter" | "art" | "admin" }>;

/**
 * The caller, or no one. A request with no assertion is no one without a word
 * to the key source (Task 11's review, M1). A key set that cannot be had, or
 * an assertion that throws while it is read, is no one, and is logged
 * (`identity-failed`). An assertion naming a `kid` the cached set lacks asks
 * for the keys once more, and is resolved ONCE more — never a loop: the key
 * source decides whether that ask may fetch (once a minute), so a forged
 * `kid` cannot force a fetch per request.
 */
async function identify(headers: Headers, keys: KeySource, config: IdentityConfig | null): Promise<Identity> {
  if (config === null || !headers.has("cf-access-jwt-assertion")) return NO_ONE;
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

/**
 * The nine maps the layer's `ArtManifest` declares, in its declaration order —
 * all that the NAS's `/api/art` answers. The manifest FILE also carries two
 * keys of its generator's, `area_duplicates` and `provenance`; the NAS's route
 * serializes the typed struct, which has no field for either, so serde drops
 * both, and the corpus's `art_manifest.json` was cut the same way. The Worker
 * PROJECTS the file onto these nine (the orchestrator's ruling on Task 12's
 * finding: streaming the file's bytes answered the two generator keys
 * besides).
 */
const ART_MAPS = ["orders", "techs", "sites", "achievements", "areas", "five_power", "mercenaries", "backs", "tokens"] as const;

/**
 * `GET|HEAD /api/art` (spec §4): the static art manifest projected onto
 * `ART_MAPS` — a map the file lacks is `{}`, as the struct's
 * `#[serde(default)]` makes it — serialized compactly, as axum's `Json` writes
 * it. It is the NAS's answer by VALUE, not byte for byte: within a map,
 * `JSON.stringify` enumerates integer-like keys (`orders`, `sites`)
 * numerically, where serde's `BTreeMap` orders them as strings. The manifest
 * carries no seed, so it may be parsed. Any other answer of the asset layer's
 * is passed on with its status.
 */
async function art(req: Request, url: URL, env: Env): Promise<Response> {
  const asset = await env.ASSETS.fetch(new URL("/art/manifest.json", url));
  if (asset.status !== 200) {
    if (req.method === "HEAD") await asset.body?.cancel();
    return new Response(req.method === "HEAD" ? null : asset.body, { status: asset.status });
  }
  const manifest = (await asset.json()) as Record<string, unknown>;
  const body = JSON.stringify(Object.fromEntries(ART_MAPS.map((key) => [key, manifest[key] ?? {}])));
  return new Response(req.method === "HEAD" ? null : body, { status: 200, headers: { "content-type": "application/json" } });
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
  target: Forwarded,
  env: Env,
  identity: Identity,
  body: Uint8Array | null,
  headers: Headers,
): { fetch(req: Request): Promise<Response> } {
  const house = () => env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME));
  switch (target.kind) {
    case "create": {
      // A named id the layer can never use is the house's, whose `create`
      // refuses it as the layer does (the orchestrator's ruling, Task 9's
      // review): the name never makes a table object.
      const named = createId(body);
      if (named !== undefined && !isSafeId(named)) return house();
      const name = named ?? drawName();
      // Percent-encoded (spec §2.3), so the name travels as ASCII whatever
      // it holds; the table object decodes it.
      headers.set(TABLE_ID_HEADER, encodeURIComponent(name));
      return env.TABLE.get(env.TABLE.idFromName(name));
    }
    case "table":
      return env.TABLE.get(env.TABLE.idFromName(target.name));
    case "archive": {
      // Named by the caller's name — an address, or an open-door claim —
      // exactly as `resolveIdentity` or `claimedIdentity` answered it: judged
      // raw, then trimmed and ASCII-lowercased as the layer's `User::parse`
      // reads a caller — so one SPELLING is one object. Under `worker` that is
      // one caller; under `open` a claim's space runs are left for the key
      // grammar, so two spellings of one key name two objects
      // (`claimedIdentity`'s doc, `identity.ts`). With no caller, or a name
      // longer than the longest address the layer accepts (which it
      // refuses), the house's `dispatch` answers the layer's 401: no archive
      // object's name passes 262 bytes, far inside the 1,024 past which the
      // platform would give it none. Under `open` a claim is held only to what
      // a header can carry (spec §2.1), so a claim the key grammar refuses —
      // `..`, a name of 40 letters — still names an object, whose `init`
      // refuses it: that object answers the layer's own 401 for the caller,
      // as the NAS does (M6.5c2 cycle D, row 8; `Archive.callerRefused`).
      const who = callerName(identity);
      if (who === undefined || new TextEncoder().encode(who).length > ADDRESS_MAX_BYTES) return house();
      return env.ARCHIVER.get(env.ARCHIVER.idFromName(ARCHIVE_PREFIX + who));
    }
    case "house":
      return house();
  }
}

/** ONE key source per isolate, made at the first request. */
let source: KeySource | undefined;

export default {
  fetch: (req, env, ctx) => handle(req, env, ctx, (source ??= keySource(env.ACCESS_TEAM_DOMAIN ?? ""))),
} satisfies ExportedHandler<Env>;
