/**
 * **The Worker's own session** (the auth cycle, design §4.2; R-A2): the cookie that names a signed-in caller on
 * every request after the sign-in, and the short login cookie that carries the sign-in's state across the
 * provider's round trip. Stateless: nothing is stored and the cookie is the whole session. A stolen one is good
 * until it expires, and renewal extends it up to the absolute cap below, so rotating `YOKOHAMA_SESSION_SECRET`
 * (every session at once) and the deny-list (one address's) are the revokes (the recorded trade, R-A2; ruling R-16
 * as amended 2026-10-19).
 *
 * **The format, one canonical byte form for each:** `v2.<payload>.<mac>`, where `<payload>` is the base64url of the
 * UTF-8 JSON `JSON.stringify` writes for an object holding the type's fields in its declared order — `address,
 * sid, first, issued, expires` for a `Session` (`sid: null` written out), `state, nonce, verifier, returnTo, expires`
 * for a `Login` — with no whitespace, and `<mac>` is the base64url of HMAC-SHA256 keyed by the secret's UTF-8 bytes
 * (as `shellConfig` counts them) over the ASCII bytes `v2.<payload>`. base64url is RFC 4648's url alphabet,
 * unpadded. The cells pin these bytes to values `openssl` computed, the reference a second minter (the plan's Task
 * 8, under `node:crypto`) is held to, so a change here is a change there. The version moved from `v1` to `v2` with
 * the absolute cap (2026-10-19): a `v1` cookie reads as nobody whatever its MAC says — no legacy branch.
 *
 * **The absolute cap** (2026-10-19, the owner's ruling after stage 1's read; ruling R-16 amended): a session carries
 * `first`, the time it was FIRST issued, which a renewal keeps while re-stamping `issued`, and no mint or renewal
 * expires later than `first` plus `YOKOHAMA_SESSION_MAX_DAYS` — the expiry is the EARLIER of `issued` plus the
 * sliding days and `first` plus the cap (`mintSession`), and a renewal is sent only when it would extend the expiry
 * (`renewalExpiry`). So the sliding days are the idle timeout and the cap the absolute one: a session used without
 * end still ends, and the player signs in again by the issuer. The minter refuses a mint at or past the cap (it
 * would be dead when written), a `first` after `issued`, and a cap below one day.
 *
 * **Both names take the `__Host-` prefix** (ruling R-14). A cookie a sibling subdomain sets for the parent domain is
 * sent to this host too, so under a bare name a site elsewhere on the domain could fix a session it signed in
 * itself onto a visitor's browser (cookie tossing). A browser refuses a `__Host-` cookie unless it is `Secure`, has
 * `Path=/` and names no `Domain`, which is exactly what `setCookie` writes. So only this host can set either cookie,
 * a browser holds at most one of each name for it, and `cookieValue`'s first match is the only one a browser sends.
 *
 * **Reading is verify-then-parse:** the MAC is checked by `crypto.subtle.verify` (constant time) over the bytes the
 * request carried, and the payload is decoded and parsed only once it verifies. The MAC segment must be 43
 * characters, refused on its length before anything decodes it, so no decoding or verifying is spent on an
 * attacker-sized MAC segment (a long body under a 43-character MAC is still hashed once, by the verify). Each
 * segment must be the one spelling the minter writes (no padding, no whitespace, no standard alphabet, no stray
 * trailing bits), the payload's bytes must be exactly UTF-8 with no
 * byte-order mark, and a payload must hold its type's fields with their types (whole seconds for the times). Any
 * exception on the way (a bad segment, bad JSON, a key the platform refuses) reads as nobody: a cookie never
 * reaches `answer` as an exception. The shape check is also what keeps one cookie from reading as the other, since
 * the two share the secret and the format and each requires a field the other never carries. A reader takes any
 * field order and drops any field it does not declare; the minters write only the canonical form.
 *
 * **The boundaries:** a cookie read at its `expires` second has expired, and at a NaN clock every cookie reads as
 * nobody (the test is written `!(now < expires)`, which fails closed). Renewal is due when the life LEFT is less
 * than half the configured days (`renewDue`), and exactly half is not yet due. Reading the life left rather than
 * the life spent keeps one promise across a change of `YOKOHAMA_SESSION_DAYS`: every cookie is due throughout the
 * last half of today's days before its expiry, whatever days it was minted under. Read by the life spent, a cookie
 * minted under fewer days than half of today's would expire before it ever came due.
 *
 * The days have no upper bound beyond `shellConfig`'s safe-integer check, and the minter refuses days below one.
 * Browsers cap a cookie's lifetime (Chrome at 400 days), so a longer setting dies in the browser first, while the
 * `expires` the MAC stamps still bounds what this reader accepts.
 *
 * The base64url helpers are this module's own: `identity.ts` keeps a private decoder that a later task may export,
 * and the two may be folded then.
 */

export const SESSION_COOKIE = "__Host-yokohama_session";
export const LOGIN_COOKIE = "__Host-yokohama_login";

/** A signed-in caller: the address the provider vouched for, already normalized by the caller; the provider's
 *  session id when it sent one (sign-out ends that session too); the time the session was FIRST issued (kept across
 *  renewals — the cap counts from it), the time this cookie was issued, and the expiry, whole seconds. */
export interface Session { address: string; sid: string | null; first: number; issued: number; expires: number }

/** The sign-in in flight: the state, the nonce and the PKCE verifier the callback needs, where to land after it,
 *  and when it stops being honoured, whole seconds. */
export interface Login { state: string; nonce: string; verifier: string; returnTo: string; expires: number }

const VERSION = "v2";
const DAY_SECONDS = 86_400;
const LOGIN_SECONDS = 600;
/** The unpadded base64url of HMAC-SHA256's 32 bytes. */
const MAC_LENGTH = 43;
const ATTRIBUTES = "Path=/; HttpOnly; Secure; SameSite=Lax";

const utf8 = new TextEncoder();

function encode(bytes: Uint8Array): string {
  let binary = "";
  for (const b of bytes) binary += String.fromCharCode(b);
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** The bytes of a base64url segment, or a throw for any spelling `encode` would not write — so a valid cookie
 *  has exactly one spelling. `atob` alone is forgiving (it skips whitespace, takes padding and the standard
 *  alphabet, and drops a last character's unused bits); the round trip is what refuses each of those. */
function decode(segment: string): Uint8Array {
  const bytes = Uint8Array.from(atob(segment.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));
  if (encode(bytes) !== segment) throw new TypeError("not the canonical base64url spelling");
  return bytes;
}

function hmacKey(secret: string, usage: "sign" | "verify"): Promise<CryptoKey> {
  return crypto.subtle.importKey("raw", utf8.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, [usage]);
}

const whole = (n: unknown): n is number => typeof n === "number" && Number.isInteger(n);

function wholeSeconds(n: number, what: string): void {
  if (!whole(n)) throw new RangeError(`${what} must be whole seconds — got ${n}`);
}

/** The cookie value for `payload`, whose own key order is the byte order. */
async function seal(payload: Session | Login, secret: string): Promise<string> {
  const signed = `${VERSION}.${encode(utf8.encode(JSON.stringify(payload)))}`;
  const mac = await crypto.subtle.sign("HMAC", await hmacKey(secret, "sign"), utf8.encode(signed));
  return `${signed}.${encode(new Uint8Array(mac))}`;
}

/** The verified payload of the cookie `name` in `header`, or `null`. Throws on a malformed segment; the callers
 *  catch. */
async function open(header: string | null, name: string, secret: string): Promise<Record<string, unknown> | null> {
  const value = cookieValue(header, name);
  if (value === null) return null;
  const segments = value.split(".");
  if (segments.length !== 3) return null;
  const [version, body, mac] = segments as [string, string, string];
  if (version !== VERSION || mac.length !== MAC_LENGTH) return null;
  const key = await hmacKey(secret, "verify");
  if (!(await crypto.subtle.verify("HMAC", key, decode(mac), utf8.encode(`${version}.${body}`)))) return null;
  // `ignoreBOM: true` KEEPS a leading byte-order mark (the default strips it), so `JSON.parse` refuses one.
  const payload: unknown = JSON.parse(new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(decode(body)));
  // An array passes this and then fails every field its reader asks for.
  return typeof payload === "object" && payload !== null ? (payload as Record<string, unknown>) : null;
}

/** Whole days above zero, or the refusal naming `what`. */
function wholeDays(n: number, what: string): void {
  if (!whole(n) || n < 1) throw new RangeError(`${what} must be whole and at least one — got ${n}`);
}

/** The expiry of a cookie issued at `issued` for a session first issued at `first`: the earlier of `issued` plus
 *  `days` and `first` plus `maxDays` — the one arithmetic `mintSession` writes and `renewalExpiry` answers, exported
 *  so a `Set-Cookie`'s `Max-Age` is the life the cookie really has. */
export function sessionExpiry(first: number, issued: number, days: number, maxDays: number): number {
  return Math.min(issued + days * DAY_SECONDS, first + maxDays * DAY_SECONDS);
}

/** The session cookie's VALUE for `s`, expiring at the earlier of `s.issued` plus `days` and `s.first` plus
 *  `maxDays`. Refuses a time that is not whole seconds, days or a cap that are not whole or are below one, a `first`
 *  after `issued`, and an expiry that would not be after `issued` (a mint at or past the cap), so it never writes a
 *  cookie that is dead when minted. */
export async function mintSession(s: Omit<Session, "expires">, days: number, maxDays: number, secret: string): Promise<string> {
  wholeSeconds(s.first, "first");
  wholeSeconds(s.issued, "issued");
  wholeDays(days, "days");
  wholeDays(maxDays, "maxDays");
  if (s.first > s.issued) throw new RangeError(`first must not be after issued — got ${s.first} after ${s.issued}`);
  const expires = sessionExpiry(s.first, s.issued, days, maxDays);
  if (!(expires > s.issued)) throw new RangeError(`the session reached its cap: issued at ${s.issued}, capped at ${expires}`);
  return seal({ address: s.address, sid: s.sid, first: s.first, issued: s.issued, expires }, secret);
}

/** The session the request's `Cookie` header carries, or `null` for none, a forged or malformed one, one whose
 *  `first` is after its `issued`, or one at or past its `expires`. Never throws. */
export async function readSession(cookieHeader: string | null, secret: string, nowSeconds: number): Promise<Session | null> {
  try {
    const p = await open(cookieHeader, SESSION_COOKIE, secret);
    if (p === null) return null;
    const { address, sid, first, issued, expires } = p;
    if (typeof address !== "string" || (sid !== null && typeof sid !== "string")) return null;
    if (!whole(first) || !whole(issued) || !whole(expires) || first > issued || !(nowSeconds < expires)) return null;
    return { address, sid, first, issued, expires };
  } catch {
    return null;
  }
}

/** Whether a request at `nowSeconds` should carry a fresh session cookie: less than half of `days` is left. */
export function renewDue(s: Session, nowSeconds: number, days: number): boolean {
  return s.expires - nowSeconds < (days * DAY_SECONDS) / 2;
}

/** The expiry a renewal at `nowSeconds` would set — `sessionExpiry` over `s.first` and now — when one is due
 *  (`renewDue`) AND it would be later than the expiry `s` already has; `null` otherwise, which is also what a cookie
 *  sitting at its cap answers: due by the days, but a renewal would set the same expiry, so none is sent. The value
 *  is exactly what `mintSession` writes for a mint at that second, which keeps the answer and the cookie one
 *  arithmetic. */
export function renewalExpiry(s: Session, nowSeconds: number, days: number, maxDays: number): number | null {
  if (!renewDue(s, nowSeconds, days)) return null;
  const expires = sessionExpiry(s.first, nowSeconds, days, maxDays);
  return expires > s.expires ? expires : null;
}

/** The login cookie's VALUE for `l`, expiring ten minutes after `nowSeconds`. */
export async function mintLogin(l: Omit<Login, "expires">, nowSeconds: number, secret: string): Promise<string> {
  wholeSeconds(nowSeconds, "nowSeconds");
  const login: Login = {
    state: l.state, nonce: l.nonce, verifier: l.verifier, returnTo: l.returnTo, expires: nowSeconds + LOGIN_SECONDS,
  };
  return seal(login, secret);
}

/** The sign-in in flight that `cookieHeader` (a `Cookie` header or a `Set-Cookie` line) carries, or `null`.
 *  Never throws. */
export async function readLogin(cookieHeader: string | null, secret: string, nowSeconds: number): Promise<Login | null> {
  try {
    const p = await open(cookieHeader, LOGIN_COOKIE, secret);
    if (p === null) return null;
    const { state, nonce, verifier, returnTo, expires } = p;
    if (typeof state !== "string" || typeof nonce !== "string" || typeof verifier !== "string") return null;
    if (typeof returnTo !== "string" || !whole(expires) || !(nowSeconds < expires)) return null;
    return { state, nonce, verifier, returnTo, expires };
  } catch {
    return null;
  }
}

/** A `Set-Cookie` line for `name`, with the attributes both cookies share. */
export function setCookie(name: string, value: string, maxAgeSeconds: number): string {
  return `${name}=${value}; Max-Age=${maxAgeSeconds}; ${ATTRIBUTES}`;
}

/** The `Set-Cookie` line that ends `name` in the browser. */
export function clearCookie(name: string): string {
  return setCookie(name, "", 0);
}

/** The value of the first pair in `cookieHeader` whose name is exactly `name`, case included, or `null`. The header
 *  is split on `;`, each pair trimmed and split on its FIRST `=`; a pair with no `=` names nothing. It reads a
 *  `Set-Cookie` line too, whose first pair is the cookie and whose attributes follow. */
export function cookieValue(cookieHeader: string | null, name: string): string | null {
  if (cookieHeader === null) return null;
  for (const raw of cookieHeader.split(";")) {
    const pair = raw.trim();
    const eq = pair.indexOf("=");
    if (eq !== -1 && pair.slice(0, eq) === name) return pair.slice(eq + 1);
  }
  return null;
}
