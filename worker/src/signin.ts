/**
 * **The two sign-in arms** (the auth cycle, design §4.1; rulings R-A1 and R-A7): how the Worker learns a caller's
 * address at the end of a sign-in, once, at the callback on `/enter`. No request after it touches the issuer: the
 * session cookie (`session.ts`) carries the address from then on.
 *
 * - **`workos`** runs WorkOS's own first-party API with two `fetch` calls and no SDK: the browser goes to
 *   `WORKOS_AUTHORIZE` with `provider=authkit`, and the callback POSTs the code to `WORKOS_AUTHENTICATE` as JSON
 *   with the API key. The answer's `user.email` is the address and `user.email_verified` must be `true`. The
 *   answer's `access_token` carries the WorkOS session id (`sid`) that sign-out ends, and it is read only once the
 *   token's signature verifies against `WORKOS_JWKS(clientId)`. That check is belt and braces: TLS and the API key
 *   already authenticate the answer, so a token that fails it costs the `sid`, never the sign-in. The token
 *   carries no `email` claim (the pre-flight's reading), so the address is never the token's.
 * - **`oidc`** is a stock OpenID Connect relying party for any issuer with a discovery document: the code flow with
 *   `openid email`, a nonce, and PKCE when no client secret is set (`client_secret_post` when one is). Its ID token
 *   is verified as `resolveIdentity` verifies an Access assertion (`verifyIdToken`), plus the nonce and
 *   `email_verified`.
 *
 * **The address is judged RAW** by `forwardable` before anything is done to it, then trimmed and lowercased in
 * ASCII, then held to `isAddress`: the Kelvin rule `identity.ts` records, so `\u212Aevin@x.test` is no caller rather
 * than another user's `kevin@x.test`.
 *
 * **Every failure of `exchange` is `null`, never an exception**, after one `signin-failed` line naming the arm and
 * the step. A line never carries the exchange's body, a token, a secret or an address: a parse failure is logged as
 * a fixed sentence, since V8's `JSON.parse` message quotes the text it failed on. `verifyIdToken` answers `null`
 * without a line (its caller logs). `discovery` REJECTS, as `keySourceAt` does, and its caller must read a rejection
 * as no document; `authorizeUrl` THROWS under `oidc` when it is handed no document, a caller's error.
 *
 * Pure but for the `fetch` the caller injects: every cell runs with test keys and no network
 * (`src/signin.unit.test.ts`).
 */
import type { ShellConfig, Source } from "./config";
import { asciiLower, bytes, forwardable, isAddress, json, type JwkSet, type KeySource } from "./identity";
import { log } from "./respond";

/** Who signed in: the address the issuer vouched for, normalized, and the issuer's session id when it sent one. */
export interface SignedIn { address: string; sid: string | null }

/** The endpoints an OpenID Connect issuer's discovery document names, as the `oidc` arm reads them. */
export interface Discovery {
  issuer: string;
  authorization_endpoint: string;
  token_endpoint: string;
  jwks_uri: string;
  end_session_endpoint: string | null;
}

/** A sign-in arm: every source but `access`, whose sign-in is Access's own. */
type Arm = Exclude<Source, "access">;
type Signin = NonNullable<ShellConfig["signin"]>;

export const WORKOS_AUTHORIZE = "https://api.workos.com/user_management/authorize";
export const WORKOS_AUTHENTICATE = "https://api.workos.com/user_management/authenticate";
export const WORKOS_JWKS = (clientId: string): string => `https://api.workos.com/sso/jwks/${clientId}`;
export const WORKOS_LOGOUT = (sid: string): string =>
  `https://api.workos.com/user_management/sessions/logout?session_id=${encodeURIComponent(sid)}`;

/** How far ahead of the clock a token's `nbf` may be: `resolveIdentity`'s skew. */
const SKEW_SECONDS = 60;

/**
 * Where the browser goes to sign in. `workos`: `WORKOS_AUTHORIZE` with exactly the client id, the redirect URI,
 * `response_type=code`, `provider=authkit` and the state (no nonce and no challenge: neither is part of WorkOS's
 * first-party flow). `oidc`: the discovery document's `authorization_endpoint`, keeping any query it carries, with
 * the client id, the redirect URI, `response_type=code`, `scope=openid email`, the state and the nonce, and
 * `code_challenge` with `code_challenge_method=S256` only when `p.challenge` is set. Under `oidc` the document is
 * required, and its absence THROWS: the caller fetches it before it starts a sign-in.
 */
export function authorizeUrl(
  source: Arm,
  signin: Signin,
  p: { redirectUri: string; state: string; nonce: string; challenge: string | null },
  discovery: Discovery | null,
): string {
  if (source === "workos") {
    return withParams(WORKOS_AUTHORIZE, {
      client_id: signin.clientId,
      redirect_uri: p.redirectUri,
      response_type: "code",
      provider: "authkit",
      state: p.state,
    });
  }
  if (discovery === null) throw new Error("the oidc arm's authorize URL needs the issuer's discovery document");
  return withParams(discovery.authorization_endpoint, {
    client_id: signin.clientId,
    redirect_uri: p.redirectUri,
    response_type: "code",
    scope: "openid email",
    state: p.state,
    nonce: p.nonce,
    ...(p.challenge === null ? {} : { code_challenge: p.challenge, code_challenge_method: "S256" }),
  });
}

/** `endpoint` with `params` set on its query, any parameter it already carries kept. */
function withParams(endpoint: string, params: Record<string, string>): string {
  const url = new URL(endpoint);
  for (const [name, value] of Object.entries(params)) url.searchParams.set(name, value);
  return url.toString();
}

/**
 * The callback's code exchange: who signed in, or `null` after one `signin-failed` line. `workos` POSTs JSON to
 * `WORKOS_AUTHENTICATE` (`ip_address` and `user_agent` only when given; WorkOS's Radar reads them) and reads the
 * answer's `user`; `oidc` POSTs a form to the document's `token_endpoint` and verifies the answer's `id_token`
 * against the document's issuer, the client id and `p.nonce`. `net.keys` is the arm's key source: the WorkOS JWKS
 * under `workos`, the document's `jwks_uri` under `oidc`. A token naming a `kid` the cached set lacks asks it to
 * refresh once, as `identify` does for Access. `net.discovery` is ignored under `workos`.
 */
export async function exchange(
  source: Arm,
  signin: Signin,
  p: { code: string; redirectUri: string; verifier: string; nonce: string; ip: string | null; userAgent: string | null },
  net: { fetcher: typeof fetch; keys: KeySource; discovery: Discovery | null; nowSeconds: number },
): Promise<SignedIn | null> {
  try {
    return source === "workos" ? await workos(signin, p, net) : await oidc(signin, p, net);
  } catch (e) {
    // Nothing above throws by design; this keeps an unforeseen exception from reaching `answer`.
    return failed(source, "unexpected", String(e));
  }
}

async function workos(
  signin: Signin,
  p: Parameters<typeof exchange>[2],
  net: Parameters<typeof exchange>[3],
): Promise<SignedIn | null> {
  if (signin.clientSecret === null) return failed("workos", "config", "no API key to authenticate the exchange with");
  const body: Record<string, string> = {
    client_id: signin.clientId,
    client_secret: signin.clientSecret,
    grant_type: "authorization_code",
    code: p.code,
  };
  if (p.ip !== null) body.ip_address = p.ip;
  if (p.userAgent !== null) body.user_agent = p.userAgent;
  const answer = await post("workos", WORKOS_AUTHENTICATE, "application/json", JSON.stringify(body), net.fetcher);
  if (answer === null) return null;
  const user = answer.user;
  if (!isRecord(user)) return failed("workos", "user", "the answer names no user");
  if (user.email_verified !== true) return failed("workos", "user", "the user's email is not verified");
  const address = vouched(user.email);
  if (address === null) return failed("workos", "user", "the user's email is not an address this site accepts");
  return { address, sid: await workosSid(answer.access_token, net.keys) };
}

/** The WorkOS session id, read from the access token only once its signature verifies; otherwise `null`, logged. */
async function workosSid(token: unknown, keys: KeySource): Promise<string | null> {
  const unverified = (error: string): null => {
    log("signin-sid-unverified", { source: "workos", error });
    return null;
  };
  if (typeof token !== "string") return unverified("the answer holds no access token");
  let claims: Record<string, unknown> | null;
  try {
    claims = await verifyJwtSignature(token, await keysFor(token, keys));
  } catch (e) {
    return unverified(String(e));
  }
  return claims === null ? unverified("the access token's signature does not verify") : sidOf(claims.sid);
}

async function oidc(
  signin: Signin,
  p: Parameters<typeof exchange>[2],
  net: Parameters<typeof exchange>[3],
): Promise<SignedIn | null> {
  const doc = net.discovery;
  if (doc === null) return failed("oidc", "discovery", "no discovery document");
  const form = new URLSearchParams({
    grant_type: "authorization_code",
    code: p.code,
    redirect_uri: p.redirectUri,
    client_id: signin.clientId,
  });
  if (signin.clientSecret !== null) form.set("client_secret", signin.clientSecret);
  else form.set("code_verifier", p.verifier);
  const answer = await post("oidc", doc.token_endpoint, "application/x-www-form-urlencoded", form.toString(), net.fetcher);
  if (answer === null) return null;
  const token = answer.id_token;
  if (typeof token !== "string") return failed("oidc", "id-token", "the answer holds no id_token");
  let set: JwkSet;
  try {
    set = await keysFor(token, net.keys);
  } catch (e) {
    return failed("oidc", "keys", String(e));
  }
  const c = { iss: doc.issuer, aud: signin.clientId, nonce: p.nonce, nowSeconds: net.nowSeconds };
  return (await verifyIdToken(token, set, c)) ?? failed("oidc", "id-token", "the id_token does not verify");
}

/** The exchange's POST: the answer as a JSON object, or `null` after a `signin-failed` line. */
async function post(
  source: Arm,
  url: string,
  type: string,
  body: string,
  fetcher: typeof fetch,
): Promise<Record<string, unknown> | null> {
  let res: Response;
  try {
    res = await fetcher(url, { method: "POST", headers: { "content-type": type, accept: "application/json" }, body });
  } catch (e) {
    return failed(source, "exchange", String(e));
  }
  if (res.status !== 200) return failed(source, "exchange", `the exchange at ${url} answered ${res.status}`);
  let answer: unknown;
  try {
    answer = await res.json();
  } catch {
    return failed(source, "exchange", "the exchange's answer is not JSON");
  }
  return isRecord(answer) ? answer : failed(source, "exchange", "the exchange's answer is not a JSON object");
}

/** Log one `signin-failed` line and answer `null`. `error` is a sentence, never a body, a token or an address. */
function failed(source: Arm, step: string, error: string): null {
  log("signin-failed", { source, step, error });
  return null;
}

/**
 * **An ID token, verified** — `resolveIdentity`'s steps for a token in hand, plus what Access never needed: the
 * header's `alg` is `RS256` or `ES256` (issuers vary) with a string `kid`; the key the `kid` names is in `keys` and
 * fits that algorithm; the signature holds over `<header>.<payload>`; `iss` is `c.iss` exactly; `aud` is `c.aud` or
 * an array holding it; `exp` is a number after now; `nbf`, when a number, is at most 60 seconds ahead; `nonce` is
 * `c.nonce` exactly (an empty expected nonce matches nothing); `email_verified` is `true` and nothing else; and the
 * `email` claim passes `forwardable` RAW, then trimmed and lowercased in ASCII passes `isAddress`. The `sid` claim
 * rides along when it is a non-empty string. Anything short of that, a malformed token included, is `null`.
 *
 * A `kid` that `keys` lacks is `null` here too: the caller that holds a key source refreshes it first (`keysFor`).
 */
export async function verifyIdToken(
  token: string,
  keys: JwkSet,
  c: { iss: string; aud: string; nonce: string; nowSeconds: number },
): Promise<SignedIn | null> {
  const claims = await verifyJwtSignature(token, keys);
  if (claims === null) return null;
  if (claims.iss !== c.iss) return null;
  const aud = claims.aud;
  if (!(typeof aud === "string" ? aud === c.aud : Array.isArray(aud) && aud.includes(c.aud))) return null;
  // Written so a NaN clock fails closed: every comparison with NaN is false.
  if (typeof claims.exp !== "number" || !(claims.exp > c.nowSeconds)) return null;
  if (typeof claims.nbf === "number" && !(claims.nbf <= c.nowSeconds + SKEW_SECONDS)) return null;
  if (c.nonce === "" || claims.nonce !== c.nonce) return null;
  if (claims.email_verified !== true) return null;
  const address = vouched(claims.email);
  return address === null ? null : { address, sid: sidOf(claims.sid) };
}

/** The address a raw `email` value names: judged by `forwardable` first, then trimmed, lowercased in ASCII, and held
 *  to `isAddress` — or `null`. */
function vouched(raw: unknown): string | null {
  if (typeof raw !== "string" || !forwardable(raw)) return null;
  const address = asciiLower(raw.trim());
  return isAddress(address) ? address : null;
}

/** A session id worth keeping: a non-empty string, else `null`. */
function sidOf(raw: unknown): string | null {
  return typeof raw === "string" && raw !== "" ? raw : null;
}

/** The algorithms a token may name, with the key type each needs and how WebCrypto imports and verifies under it. */
const ALGORITHMS = {
  RS256: {
    kty: "RSA",
    crv: undefined,
    importAs: { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
    verifyAs: { name: "RSASSA-PKCS1-v1_5" },
  },
  ES256: {
    kty: "EC",
    crv: "P-256",
    importAs: { name: "ECDSA", namedCurve: "P-256" },
    verifyAs: { name: "ECDSA", hash: "SHA-256" },
  },
} as const;
type Alg = keyof typeof ALGORITHMS;

/** A compact JWS taken apart: the header's algorithm and `kid`, the claims, the signed bytes and the signature. */
interface Jws { alg: Alg; kid: string; claims: Record<string, unknown>; signed: Uint8Array; signature: Uint8Array }

/** `token` as a JWS this module can verify, or `null`: three parts, a JSON object header naming `RS256` or `ES256`
 *  and a string `kid`, a JSON object payload, and a base64url signature. Never throws. */
function parseJws(token: string): Jws | null {
  const parts = token.split(".");
  if (parts.length !== 3) return null;
  const [head, payload, signature] = parts as [string, string, string];
  try {
    const header: unknown = json(head);
    const claims: unknown = json(payload);
    if (!isRecord(header) || !isRecord(claims)) return null;
    if (header.alg !== "RS256" && header.alg !== "ES256") return null;
    // A header with no string `kid` names no key: comparing `undefined` would pick a key that names none.
    if (typeof header.kid !== "string") return null;
    return {
      alg: header.alg,
      kid: header.kid,
      claims,
      signed: new TextEncoder().encode(`${head}.${payload}`),
      signature: bytes(signature),
    };
  } catch {
    return null;
  }
}

/**
 * The signature half of `verifyIdToken`, and nothing of the claims: the token's claims when its signature holds
 * under the key in `keys` its `kid` names, else `null`. A key whose type (or curve) does not fit the header's
 * algorithm, or that declares another `alg`, is refused before WebCrypto sees it, and anything WebCrypto throws on
 * (a key it cannot import, a signature it cannot read) is `null`: never an exception into the caller.
 */
async function verifyJwtSignature(token: string, keys: JwkSet): Promise<Record<string, unknown> | null> {
  const jws = parseJws(token);
  if (jws === null) return null;
  const jwk = keys.keys.find((k) => k.kid === jws.kid);
  if (jwk === undefined) return null;
  const a = ALGORITHMS[jws.alg];
  if (jwk.kty !== a.kty || (a.crv !== undefined && jwk.crv !== a.crv)) return null;
  if (jwk.alg !== undefined && jwk.alg !== jws.alg) return null;
  try {
    const key = await crypto.subtle.importKey("jwk", jwk, a.importAs, false, ["verify"]);
    return (await crypto.subtle.verify(a.verifyAs, key, jws.signature, jws.signed)) ? jws.claims : null;
  } catch {
    return null;
  }
}

/** The key set to verify `token` against: the cached set, or, when it lacks the `kid` the token names, the
 *  refreshed one (a `keySourceAt` source holds a refresh to one fetch a minute). Rejects when the source does. */
async function keysFor(token: string, keys: KeySource): Promise<JwkSet> {
  const kid = parseJws(token)?.kid;
  const cached = await keys(false);
  return kid === undefined || cached.keys.some((k) => k.kid === kid) ? cached : keys(true);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** The discovery documents this isolate has fetched, or is fetching, by issuer (a trailing slash dropped). */
const discovered = new Map<string, Promise<Discovery>>();

/**
 * **The issuer's discovery document**, from `<issuer>/.well-known/openid-configuration` (a trailing slash on the
 * issuer dropped), cached per isolate as `keySourceAt` caches a key set: the calls that arrive while a fetch is out
 * JOIN it, so one fetch serves them all; a fetch that fails, answers anything but 200, or answers a document without
 * the four endpoints REJECTS every call that joined it and caches nothing, so the next call fetches again. A
 * document that loaded is kept for the isolate's life (there is no refresh: a key rotation moves the JWKS, not the
 * document). `fetcher` is the first caller's; a call that finds the issuer cached or in flight does not use its own.
 *
 * The document must be a JSON object with `issuer`, `authorization_endpoint`, `token_endpoint` and `jwks_uri` each a
 * string that parses as a URL, and `end_session_endpoint` a URL, `null` or absent (absent reads `null`), so the
 * URLs the arm builds from it never throw.
 */
export function discovery(issuer: string, fetcher: typeof fetch): Promise<Discovery> {
  const base = issuer.replace(/\/$/, "");
  const known = discovered.get(base);
  if (known !== undefined) return known;
  const pending = loadDiscovery(`${base}/.well-known/openid-configuration`, fetcher);
  discovered.set(base, pending);
  // Registered before any caller can await `pending`, so the entry is gone before a joiner sees the rejection.
  pending.catch(() => {
    if (discovered.get(base) === pending) discovered.delete(base);
  });
  return pending;
}

async function loadDiscovery(url: string, fetcher: typeof fetch): Promise<Discovery> {
  const res = await fetcher(url, { headers: { accept: "application/json" } });
  if (res.status !== 200) throw new Error(`the discovery document at ${url} answered ${res.status}`);
  let doc: unknown;
  try {
    doc = await res.json();
  } catch {
    throw new Error(`the discovery document at ${url} is not JSON`);
  }
  if (!isRecord(doc)) throw new Error(`the discovery document at ${url} is not a JSON object`);
  const endpoint = (name: string): string => {
    const value = doc[name];
    if (typeof value !== "string" || !parsesAsUrl(value)) throw new Error(`the discovery document at ${url} names no ${name}`);
    return value;
  };
  const end = doc.end_session_endpoint;
  if (end !== undefined && end !== null && (typeof end !== "string" || !parsesAsUrl(end))) {
    throw new Error(`the discovery document at ${url} names an end_session_endpoint that is no URL`);
  }
  return Object.freeze({
    issuer: endpoint("issuer"),
    authorization_endpoint: endpoint("authorization_endpoint"),
    token_endpoint: endpoint("token_endpoint"),
    jwks_uri: endpoint("jwks_uri"),
    end_session_endpoint: typeof end === "string" ? end : null,
  });
}

function parsesAsUrl(value: string): boolean {
  try {
    new URL(value);
    return true;
  } catch {
    return false;
  }
}

/**
 * Where sign-out sends the browser, `returnTo` encoded into it. `workos` with a session id: `WORKOS_LOGOUT(sid)` and
 * `&return_to=` (load-bearing: with no `return_to` and no app homepage set, WorkOS lands on its own error page, the
 * pre-flight's reading); with none, `returnTo` itself. `oidc`: the document's `end_session_endpoint` with
 * `post_logout_redirect_uri=` (no `id_token_hint`: the session keeps no ID token), joined with `&` when the endpoint
 * already carries a query; with no such endpoint, or no document, `returnTo` itself.
 */
export function leaveUrl(source: Arm, signedInSid: string | null, discovery: Discovery | null, returnTo: string): string {
  if (source === "workos") {
    return signedInSid === null || signedInSid === ""
      ? returnTo
      : `${WORKOS_LOGOUT(signedInSid)}&return_to=${encodeURIComponent(returnTo)}`;
  }
  const end = discovery?.end_session_endpoint ?? null;
  if (end === null) return returnTo;
  return `${end}${end.includes("?") ? "&" : "?"}post_logout_redirect_uri=${encodeURIComponent(returnTo)}`;
}
