import { ADMIN_NEEDS_JSON, adminRoute, isJsonEssence, NOT_AN_OPERATOR, OPERATOR_HEADER, operatorGate } from "./admin";
import { Archive, ARCHIVE_PREFIX } from "./archive";
import { shellConfig, type ShellConfig, type Source } from "./config";
import type { Env } from "./env";
import { House } from "./house";
import {
  callerName,
  CLAIMED_PARAM,
  CLAIMED_USER_HEADER,
  claimedIdentity,
  identityConfig,
  keySource,
  keySourceAt,
  resolveIdentity,
  type Identity,
  type IdentityConfig,
  type KeySource,
} from "./identity";
import { limited } from "./limits";
import { errorBody, log, respond } from "./respond";
import { isSafeId, route, safeReturnTo, type Target } from "./route";
import {
  clearCookie,
  LOGIN_COOKIE,
  mintLogin,
  mintSession,
  readLogin,
  readSession,
  renewDue,
  SESSION_COOKIE,
  setCookie,
} from "./session";
import { authorizeUrl, discovery, exchange, leaveUrl, WORKOS_JWKS, type Discovery } from "./signin";
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
 *  edge (S1c, B9); `nosniff`, so an API body is never read as a script. The whole set the static release carries
 *  (`_headers`' `/*` block) buys nothing on a JSON body, and the shell holds no copy of it (decision D-3). */
export const API_HEADERS: ReadonlyArray<readonly [string, string]> = [
  ["Cache-Control", "no-store"],
  ["X-Content-Type-Options", "nosniff"],
];

/** Whether `pathname` is the API's — `/api` exactly, or under `/api/` — and never `startsWith("/api")`, which
 *  would claim `/apiary` (the NAS's layer and `swRoute.ts` draw the same line). */
export function isApiPath(pathname: string): boolean {
  return pathname === "/api" || pathname.startsWith("/api/");
}

/** The challenge every 401 under `/api` carries when a sign-in arm (`oidc`, `workos`) vouches for callers (the auth
 *  cycle, design §4.3): the client's cue to draw its sign-in link beside the refusal, as Access's own
 *  `Cloudflare-Access` challenge is under Access. The wire's only addition; `/api/me`'s `mode` stays `cf-access`
 *  (R2.2-m2). */
export const SESSION_CHALLENGE = 'Yokohama-Session realm="/enter"';

/** `res` with [`API_HEADERS`] set, and with `WWW-Authenticate: SESSION_CHALLENGE` when `challenge` and `res` is a
 *  401 — a fresh `Response` over the same body and status, since an object's answer may be immutable — or `res`
 *  ITSELF for a 101, which carries its socket, cannot be rebuilt (the constructor refuses the status) and is sent
 *  nothing a cache could keep. */
export function withApiHeaders(res: Response, challenge: boolean): Response {
  if (res.status === 101) return res;
  const headers = new Headers(res.headers);
  for (const [name, value] of API_HEADERS) headers.set(name, value);
  if (challenge && res.status === 401) headers.set("WWW-Authenticate", SESSION_CHALLENGE);
  return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
}

/** What [`answer`] hands [`handle`]: the reply; whether a 401 under `/api` carries the session challenge — the
 *  configuration's source is a sign-in arm, read once in `answer` and threaded here rather than read twice; and,
 *  for a session past half its life, what a renewal re-mints. */
interface Answered { res: Response; challenge: boolean; renew: Renewal | null }

/** A session to re-mint on a 2xx answer: its address and session id, under the deployment's secret and days. */
interface Renewal { address: string; sid: string | null; secret: string; days: number }

/** An answer given before the configuration is read, or one that takes neither the challenge nor a renewal. */
function plain(res: Response): Answered {
  return { res, challenge: false, renew: null };
}

/**
 * The Worker's entry: [`answer`]'s reply, renewed and then given the API's headers. A session past half its life
 * gets a fresh cookie on a 2xx answer only — never a 101, which carries its socket and no cookie, and never a
 * refusal — and every answer under `/api` gets [`API_HEADERS`], with the session challenge on a 401 when a sign-in
 * arm is the source. `fetcher` is the sign-in's network — the issuer's discovery document, keys and code exchange —
 * the runtime's `fetch`, or a test's.
 */
export async function handle(
  req: Request,
  env: Env,
  ctx: ExecutionContext,
  keys: KeySource,
  fetcher: typeof fetch = fetch,
): Promise<Response> {
  const { res, challenge, renew } = await answer(req, env, ctx, keys, fetcher);
  const out = renew !== null && res.status >= 200 && res.status < 300 ? await renewed(res, renew) : res;
  return isApiPath(new URL(req.url).pathname) ? withApiHeaders(out, challenge) : out;
}

/** `res` with a fresh session cookie appended, issued NOW — a renewal re-stamps the time (ruling R-16) — over the
 *  same body and status, since an object's answer may be immutable. */
async function renewed(res: Response, r: Renewal): Promise<Response> {
  const value = await mintSession({ address: r.address, sid: r.sid, issued: nowSeconds() }, r.days, r.secret);
  const headers = new Headers(res.headers);
  headers.append("Set-Cookie", setCookie(SESSION_COOKIE, value, r.days * DAY_SECONDS));
  return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
}

/** The real clock, in whole seconds: what every token, cookie and renewal here is read and minted at. */
function nowSeconds(): number {
  return Math.floor(Date.now() / 1000);
}

const DAY_SECONDS = 86_400;

/**
 * **The Worker** (spec §2.3), wrapped by [`handle`], which renews a session
 * and sets the API's headers: stateless, and in this order —
 *
 * 1. `route`. A path the asset layer had no file for answers 404 with an
 *    EMPTY body — never `index.html`; any method but GET or HEAD on `/enter`
 *    or `/api/art`, and any but POST on `/api/leave`, is 405 with an empty
 *    body, as axum's router answers.
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
 *    request — every `/api/*`, and `/enter` since the auth cycle (pre-flight
 *    note R-4: its sign-in reads the configuration) — 500 with its refusal
 *    sentence, which names the variable, the open door's pair of variables or
 *    the Rate Limiting binding refused (spec §2.6).
 * 2a. `/enter` (`enter`; the auth cycle, design §4.3). Under `access`, and
 *    under the open door, the turnstile's 302 to `/#/lobby`, whatever the
 *    query, setting nothing. Under a sign-in arm (`oidc`, `workos`), with no
 *    `code` and no `error` in the query, the sign-in's START: a 302 to the
 *    issuer's authorize URL, setting the login cookie; with either, its
 *    CALLBACK: the state checked against the login cookie, the code
 *    exchanged, and a 302 to the cookie's `return_to` setting the session
 *    cookie — or `/#/lobby?signin=failed` — with the login cookie cleared
 *    either way.
 * 3. The body, read ONCE for a method that carries one, under a streaming
 *    bound: over `BODY_LIMIT` it answers 400 with the NAS's sentence. What is
 *    forwarded is the bytes it bounded.
 * 4. `/api/art`: the static art manifest projected onto the nine maps the
 *    layer's `ArtManifest` declares, as JSON (`art`).
 * 5. The caller, by the door (M6.5c2, spec §2.1) and the source (the auth
 *    cycle, design §4.2). Under `worker` with `access`: Access's
 *    assertion, verified (`resolveIdentity`), whose address was judged raw
 *    and then trimmed and ASCII-lowercased as the layer reads one. No
 *    assertion asks nothing of the key source; a key set that cannot be had
 *    is no caller, so the request fails CLOSED. Under `worker` with `oidc` or
 *    `workos`: the session cookie, its MAC and its expiry verified
 *    (`readSession`), and nothing else — no Access assertion is read and no
 *    issuer is asked, since the cookie is the whole session. Under `open`: no
 *    assertion is read and no key is asked for; the caller is the name claimed —
 *    `X-Yokohama-Claimed-User` on a `fetch`, the `claimed` query parameter on
 *    an upgrade, where a browser can put no header, and never the other
 *    carrier — held to what a header can carry (`claimedIdentity`).
 * 6. The strip rule: a fresh `Headers` without any header named
 *    `x-yokohama-…`, and the caller set, when there is one, as the ONE header
 *    its door's objects read (`forwardedCaller`): `X-Yokohama-User` for a
 *    caller the Worker verified, `X-Yokohama-Claimed-User` for a claim.
 * 6a. The rate limit (security S2b, finding C2; decision D-3; `limited`):
 *    any method but GET or HEAD spends a Rate Limiting binding — the
 *    creates' for `POST /api/games`, the writes' for every other — counted
 *    under the caller's digest at the `worker` door, and at the `open` door
 *    under the connecting IP for a create and IP plus claim for a write; a
 *    request with no caller under its IP at either. Past the limit it is 429
 *    `too many requests from you just now — wait a moment and try again`,
 *    with `Retry-After` the refused kind's own window, and no object is
 *    reached. A limiter that throws is logged `limiter-failed` and the
 *    request proceeds: the limit bounds cost, never access (AD-6).
 * 6a'. `POST /api/leave` (`leave`; the auth cycle, design §4.2): under
 *    `access`, `{"leave":"/cdn-cgi/access/logout"}`, clearing nothing; under
 *    a sign-in arm, the issuer's sign-out URL (`leaveUrl`) returning to this
 *    site's root, with the session cookie cleared. It is a POST under `/api`,
 *    so the cross-site check (1b) refuses another site's page before any of
 *    this, and the write limiter (6a) is spent under the caller's key. Under
 *    the open door no source signs anyone in: the request is the house's, as
 *    it was before the route existed (the layer's own 404).
 * 6b. `/api/admin` and below — the operator surface (spec §2.9) — pass the
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
 * isolate, or a test's. `fetcher` is the sign-in's network (`handle`'s).
 * `_ctx` is the runtime's execution context: nothing here outlives the reply,
 * so nothing reads it, and it is taken so a pool cell calls `handle` exactly as
 * the runtime's `fetch` does.
 */
async function answer(req: Request, env: Env, _ctx: ExecutionContext, keys: KeySource, fetcher: typeof fetch): Promise<Answered> {
  const url = new URL(req.url);
  const target = route(req.method, url.pathname);
  if (target.kind === "missing") return plain(new Response(null, { status: 404 }));
  if (target.kind === "not-allowed") return plain(new Response(null, { status: 405 }));
  const origin = req.headers.get("Origin");
  if (isUpgrade(req.headers) && origin !== null && origin !== url.origin) {
    return plain(respond(403, errorBody(FOREIGN_SOCKET)));
  }
  if (crossSite(req.method, req.headers, url.origin)) return plain(respond(403, errorBody(CROSS_SITE)));

  const config = shellConfig(env);
  if ("error" in config) return plain(respond(500, errorBody(config.error)));
  const now = nowSeconds();
  if (target.kind === "enter") return plain(await enter(req, url, config, fetcher, now));
  const challenge = config.source !== null && config.source !== "access";
  const done = (res: Response, renew: Renewal | null = null): Answered => ({ res, challenge, renew });

  // GET and HEAD carry no body: the Fetch API refuses one on either.
  const carries = req.method !== "GET" && req.method !== "HEAD";
  const body = carries ? await bounded(req.body) : null;
  if (body === undefined) return done(respond(400, errorBody(BODY_LIMIT_SENTENCE)));

  if (target.kind === "art") return done(await art(req, url, env));

  const identity =
    config.door === "open"
      ? claimedIdentity(isUpgrade(req.headers) ? url.searchParams.get(CLAIMED_PARAM) : req.headers.get(CLAIMED_USER_HEADER))
      : config.source === "access"
        ? await identify(req.headers, keys, identityConfig(env, config.devDoor, now))
        : await sessionIdentity(req.headers.get("cookie"), config.session, now);
  // Renewed on a 2xx (`handle`); never on the leave's answer, which clears the cookie a renewal would set again.
  const renew: Renewal | null =
    identity.kind === "session" && identity.renew && config.session !== null
      ? { address: identity.email, sid: identity.sid, secret: config.session.secret, days: config.session.days }
      : null;

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
  if (refused !== undefined) return done(refused, renew);

  if (target.kind === "leave") {
    if (config.source === "access") return done(respond(200, JSON.stringify({ leave: ACCESS_LOGOUT })));
    const arm = armOf(config);
    if (arm !== null) return done(await leave(url, arm, identity, fetcher));
    // The open door: no source signs anyone in, so the request is the house's, below.
  }

  let forwarded: Forwarded;
  if (target.kind === "admin") {
    const gate = operatorGate(config.door, config.admins, identity);
    if (gate.kind === "refused") return done(respond(403, errorBody(NOT_AN_OPERATOR)), renew);
    forwarded = { kind: "house" };
    if (gate.kind === "operator") {
      headers.set(OPERATOR_HEADER, gate.email);
      const admin = adminRoute(req.method, url.pathname, url.search.slice(1));
      if (admin.kind === "refused") return done(respond(400, errorBody(admin.sentence)), renew);
      if (admin.kind === "expire" && !isJsonEssence(req.headers.get("content-type"))) return done(respond(400, errorBody(ADMIN_NEEDS_JSON)), renew);
      if (admin.kind === "expire") forwarded = { kind: "table", name: admin.name };
    }
  } else if (target.kind === "leave") {
    forwarded = { kind: "house" };
  } else {
    forwarded = target;
  }

  // An open-door upgrade's claim rode its URL, and the header above now carries it: the object's request drops the
  // parameter, so its URL names no caller (`forwardedCaller` reads the header alone; the S2 stack review, lens 1,
  // Minor 2). Every other parameter rides through.
  const forwardedUrl = new URL(req.url);
  if (config.door === "open" && isUpgrade(req.headers)) forwardedUrl.searchParams.delete(CLAIMED_PARAM);
  const object = objectFor(forwarded, env, identity, body, headers);
  try {
    return done(await object.fetch(new Request(forwardedUrl, { method: req.method, headers, body })), renew);
  } catch (e) {
    log("object-failed", { target: forwarded.kind, path: url.pathname, error: String(e) });
    return done(respond(500, errorBody(OBJECT_FAILED)), renew);
  }
}

/** A target `handle` forwards to an object: every route but the Worker's own
 *  answers, with an operator's route resolved to the house or a table, and the
 *  open door's `leave` to the house. */
type Forwarded = Exclude<Target, { kind: "missing" | "not-allowed" | "enter" | "art" | "admin" | "leave" }>;

/** A sign-in arm's configuration: the source and its sign-in and session halves, all three present. */
interface Arm {
  source: Exclude<Source, "access">;
  signin: NonNullable<ShellConfig["signin"]>;
  session: NonNullable<ShellConfig["session"]>;
}

/** The sign-in arm `config` runs, or `null` under `access` and the open door. `shellConfig` sets `signin` and
 *  `session` exactly when the source is `oidc` or `workos`. */
function armOf(config: ShellConfig): Arm | null {
  const { source, signin, session } = config;
  return source === null || source === "access" || signin === null || session === null ? null : { source, signin, session };
}

/** Every failed sign-in's landing (the auth cycle, design §4.3). */
export const SIGNIN_FAILED = "/#/lobby?signin=failed";

/** Access's own sign-out path: it ends the Access session and its cookie. */
const ACCESS_LOGOUT = "/cdn-cgi/access/logout";

/** A login cookie's `Max-Age`: `mintLogin`'s ten minutes, which the payload's `expires` also bounds. */
const LOGIN_SECONDS = 600;

/**
 * `GET|HEAD /enter` (step 2a of `answer`). Under `access` and the open door: the 302 to `/#/lobby`, setting nothing.
 * Under a sign-in arm: the CALLBACK when the query carries a `code` or an `error` — an issuer that refuses a sign-in
 * answers the redirect URI with `error` and no `code` (RFC 6749 §4.1.2.1), and read as a fresh START that answer
 * would send the browser straight back to an issuer that refuses again — and the START otherwise.
 */
async function enter(req: Request, url: URL, config: ShellConfig, fetcher: typeof fetch, now: number): Promise<Response> {
  const arm = armOf(config);
  if (arm === null) return redirect("/#/lobby", []);
  const isCallback = url.searchParams.has("code") || url.searchParams.has("error");
  return isCallback ? callback(req, url, arm, fetcher, now) : start(url, arm, fetcher, now);
}

/** The redirect URI the issuer registered: this origin's `/enter`, the one path both halves live on. */
function redirectUri(url: URL): string {
  return `${url.origin}/enter`;
}

/**
 * The sign-in's START: `state` and `nonce` drawn, and a PKCE verifier when the arm is `oidc` with no client secret —
 * the ONE place the PKCE decision is made (ruling R-25): a challenge beside a secret would have the issuer refuse
 * every sign-in, so with a secret, and under `workos`, there is no challenge and the stored verifier is empty. The
 * answer is a 302 to the issuer's authorize URL (`authorizeUrl`), setting the login cookie, which carries the three
 * and the landing (`safeReturnTo` of `?return_to=`) for ten minutes. Anything that throws on the way — the issuer's
 * document that cannot be had, a `Location` no header can carry — is the failed landing, logged `signin-failed`,
 * never the platform's 500.
 */
async function start(url: URL, arm: Arm, fetcher: typeof fetch, now: number): Promise<Response> {
  try {
    const state = drawn();
    const nonce = drawn();
    const pkce = arm.source === "oidc" && arm.signin.clientSecret === null;
    const verifier = pkce ? drawn() : "";
    const challenge = pkce ? base64url(new Uint8Array(await crypto.subtle.digest("SHA-256", utf8.encode(verifier)))) : null;
    const doc = await documentOf(arm, fetcher);
    const location = authorizeUrl(arm.source, arm.signin, { redirectUri: redirectUri(url), state, nonce, challenge }, doc);
    const login = await mintLogin({ state, nonce, verifier, returnTo: safeReturnTo(url.searchParams.get("return_to")) }, now, arm.session.secret);
    return redirect(location, [setCookie(LOGIN_COOKIE, login, LOGIN_SECONDS)]);
  } catch (e) {
    log("signin-failed", { source: arm.source, step: "start", error: String(e) });
    return failedSignin();
  }
}

/**
 * The sign-in's CALLBACK, refused before any exchange — logged `signin-refused` with its reason — when the request
 * carries no live login cookie this deployment signed (`no-login`: absent, forged, expired, or already cleared by an
 * earlier callback, which is what makes a replayed callback fail), when the issuer answered an `error`
 * (`issuer-error`), when the `code` is empty (`no-code`), or when the `state` is not the login cookie's, compared in
 * constant time (`state`). Then ONE exchange (`exchange`, with the caller's IP and user agent, which WorkOS's Radar
 * reads, the stored verifier and nonce, and the arm's key source); a `null` from it — logged by the arm itself — or
 * anything that throws is the failed landing. On success: a 302 to the login cookie's landing, held to
 * `safeReturnTo` once more, setting the session cookie for the configured days. Every outcome clears the login cookie.
 */
async function callback(req: Request, url: URL, arm: Arm, fetcher: typeof fetch, now: number): Promise<Response> {
  const refused = (reason: string): Response => {
    log("signin-refused", { source: arm.source, reason });
    return failedSignin();
  };
  const login = await readLogin(req.headers.get("cookie"), arm.session.secret, now);
  if (login === null) return refused("no-login");
  if (url.searchParams.has("error")) return refused("issuer-error");
  const code = url.searchParams.get("code");
  if (code === null || code === "") return refused("no-code");
  if (!sameState(url.searchParams.get("state"), login.state)) return refused("state");
  try {
    const doc = await documentOf(arm, fetcher);
    const jwks = doc === null ? WORKOS_JWKS(arm.signin.clientId) : doc.jwks_uri;
    const who = await exchange(
      arm.source,
      arm.signin,
      {
        code,
        redirectUri: redirectUri(url),
        verifier: login.verifier,
        nonce: login.nonce,
        ip: req.headers.get("cf-connecting-ip"),
        userAgent: req.headers.get("user-agent"),
      },
      { fetcher, keys: armKeys(jwks, fetcher), discovery: doc, nowSeconds: now },
    );
    if (who === null) return failedSignin();
    const value = await mintSession({ address: who.address, sid: who.sid, issued: now }, arm.session.days, arm.session.secret);
    return redirect(safeReturnTo(login.returnTo), [
      setCookie(SESSION_COOKIE, value, arm.session.days * DAY_SECONDS),
      clearCookie(LOGIN_COOKIE),
    ]);
  } catch (e) {
    log("signin-failed", { source: arm.source, step: "callback", error: String(e) });
    return failedSignin();
  }
}

/** The `oidc` arm's discovery document (`discovery`, cached per isolate and issuer), or `null` under `workos`, which
 *  has none. Rejects as `discovery` does. */
async function documentOf(arm: Arm, fetcher: typeof fetch): Promise<Discovery | null> {
  if (arm.source !== "oidc") return null;
  // `shellConfig` refuses `oidc` without an issuer, so this never throws; it keeps the type honest.
  if (arm.signin.issuer === null) throw new Error("the oidc arm has no issuer");
  return discovery(arm.signin.issuer, fetcher);
}

/** Whether the callback's `state` is the login cookie's: `timingSafeEqual` over equal-length bytes, and `false` at
 *  once for an absent state or another length (a drawn state's length is public). */
function sameState(sent: string | null, kept: string): boolean {
  if (sent === null) return false;
  const a = utf8.encode(sent);
  const b = utf8.encode(kept);
  return a.byteLength === b.byteLength && crypto.subtle.timingSafeEqual(a, b);
}

/** A 302 to `location`, setting each of `cookies`. Throws when `location` is no header value, which every caller
 *  handing it an issuer's URL catches. */
function redirect(location: string, cookies: readonly string[]): Response {
  const headers = new Headers({ Location: location });
  for (const cookie of cookies) headers.append("Set-Cookie", cookie);
  return new Response(null, { status: 302, headers });
}

/** The failed landing: `SIGNIN_FAILED`, with the login cookie cleared and no session cookie. */
function failedSignin(): Response {
  return redirect(SIGNIN_FAILED, [clearCookie(LOGIN_COOKIE)]);
}

/**
 * `POST /api/leave` under a sign-in arm (step 6a' of `answer`): `{ leave }`, the issuer's sign-out URL returning to
 * this site's root (`leaveUrl`; the WorkOS session id when the session carries one), with the session cookie cleared.
 * With no session, or no document for an `oidc` issuer (logged `signin-failed`), the leave is the site's root, and the
 * cookie is cleared all the same.
 */
async function leave(url: URL, arm: Arm, identity: Identity, fetcher: typeof fetch): Promise<Response> {
  let doc: Discovery | null = null;
  try {
    doc = await documentOf(arm, fetcher);
  } catch (e) {
    log("signin-failed", { source: arm.source, step: "leave", error: String(e) });
  }
  const sid = identity.kind === "session" ? identity.sid : null;
  const res = respond(200, JSON.stringify({ leave: leaveUrl(arm.source, sid, doc, `${url.origin}/`, arm.signin.clientId) }));
  res.headers.append("Set-Cookie", clearCookie(SESSION_COOKIE));
  return res;
}

/** The caller a sign-in arm's session cookie names, or no one (`readSession` never throws), with `renew` set when the
 *  cookie is past half its life (`renewDue`). */
async function sessionIdentity(cookie: string | null, session: ShellConfig["session"], now: number): Promise<Identity> {
  if (session === null) return NO_ONE;
  const s = await readSession(cookie, session.secret, now);
  return s === null ? NO_ONE : { kind: "session", email: s.address, sid: s.sid, renew: renewDue(s, now, session.days) };
}

const utf8 = new TextEncoder();

/** 32 bytes from `crypto.getRandomValues`, base64url: a state, a nonce or a PKCE verifier (43 characters, inside RFC
 *  7636's 43 to 128). */
function drawn(): string {
  return base64url(crypto.getRandomValues(new Uint8Array(32)));
}

/** RFC 4648's url alphabet, unpadded. */
function base64url(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

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
      // reads a caller — so one SPELLING is one object. Under `worker` that
      // is one caller; under `open` a claim's space runs are collapsed
      // (`claimedIdentity`), so every spelling of one key names ONE object.
      // With no caller, or a name longer than the longest address the layer
      // accepts (which it refuses), the house's `dispatch` answers the
      // layer's 401: no archive object's name passes 262 bytes, far inside
      // the 1,024 past which the platform would give it none. Under `open` a
      // claim is held only to what a header can carry (spec §2.1), so a claim
      // the key grammar refuses — `..`, a name of 40 letters — still names an
      // object, whose `init` refuses it: that object answers the layer's own
      // 401 for the caller, as the NAS does (M6.5c2 cycle D, row 8;
      // `Archive.callerRefused`).
      const who = callerName(identity);
      if (who === undefined || new TextEncoder().encode(who).length > ADDRESS_MAX_BYTES) return house();
      return env.ARCHIVER.get(env.ARCHIVER.idFromName(ARCHIVE_PREFIX + who));
    }
    case "house":
      return house();
  }
}

/**
 * The sign-in arms' key sources: ONE per isolate and JWKS URL (`keySourceAt`'s doc), made at the first callback that
 * needs each — `WORKOS_JWKS(clientId)` under `workos`, the discovery document's `jwks_uri` under `oidc` — and kept,
 * as the Access `source` below is, so a token naming an unknown `kid` costs at most one fetch a minute per URL,
 * whoever sends it. Each URL comes from the configuration or the issuer's cached document, so a deployment holds one
 * entry. The FIRST caller's `fetcher` serves a URL for the isolate's life, as `discovery`'s cache does.
 */
const armKeySources = new Map<string, KeySource>();

function armKeys(url: string, fetcher: typeof fetch): KeySource {
  let keys = armKeySources.get(url);
  if (keys === undefined) {
    keys = keySourceAt(url, fetcher);
    armKeySources.set(url, keys);
  }
  return keys;
}

/** ONE key source per isolate, made at the first request. */
let source: KeySource | undefined;

export default {
  fetch: (req, env, ctx) => handle(req, env, ctx, (source ??= keySource(env.ACCESS_TEAM_DOMAIN ?? ""))),
} satisfies ExportedHandler<Env>;
