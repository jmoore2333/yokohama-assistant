/**
 * **Who is calling** (R-6.5c.3; spec §2.3): the Access application's
 * assertion, VERIFIED — its RS256 signature against the team's keys, its
 * audience, its issuer and its times — never merely decoded, as the spike's
 * routing Worker decoded it.
 *
 * Pure but for `keySource`'s fetch, which is injected: every cell runs with a
 * test key set and no network (`src/identity.unit.test.ts`; `handle`'s half is
 * `test/worker.test.ts`).
 */
import type { Env } from "./env";
import { isSafeId } from "./route";

export type Identity =
  | { kind: "person"; email: string }
  | { kind: "dev"; email: string }
  /** The open door's caller (M6.5c2, spec §2.1; review R2.2-m6): a name a
   *  caller claimed, held only to what a header can carry. Its own kind, so
   *  the operator gate (a person or a session only) and the archive object's
   *  naming read it without a special case, and `dev` stays the live tier's kind
   *  under `worker`. */
  | { kind: "claimed"; name: string }
  /** A signed-in caller under a sign-in arm (`oidc`, `workos`; the auth cycle, design §4.2): the address the
   *  session cookie carries, which the issuer vouched for at the sign-in, and the issuer's session id when it sent
   *  one, which sign-out ends. `renew` says the cookie is past half its life (`renewDue`), so a 2xx answer carries a
   *  fresh one. The operator gate admits it exactly as a person (ruling R-27). */
  | { kind: "session"; email: string; sid: string | null; renew: boolean }
  /** A caller-less upload to `POST /api/archive` under the `worker` door (the auth cycle, design §4.4; `anonymousIdentity`):
   *  `device` is the post's device id, or with none the game's own id from its table document (R-A11). Formed in
   *  `answer` for that one target, so no game route ever sees it; `callerName` renders it as a synthetic ADDRESS the
   *  key grammar already accepts, so the layer and the module are untouched. Never an operator, and counted by IP. */
  | { kind: "anonymous"; device: string }
  | { kind: "none"; unknownKid?: true };
export interface JwkSet { keys: Array<JsonWebKey & { kid?: string }> }
export interface IdentityConfig {
  teamDomain: string; aud: string; devDoorClientId: string; devDoor: boolean; nowSeconds: number;
}

/** The header a claim rides on a `fetch`: the dev door's under `worker`, and
 *  the open door's (M6.5c2, spec §2.1) — which the Worker also hands the
 *  objects under `open` (`forwardedCaller`, `table.ts`). */
export const CLAIMED_USER_HEADER = "X-Yokohama-Claimed-User";

/** The query parameter an open-door claim rides on a WebSocket UPGRADE, which
 *  a browser can put no header on (M6.5c2, spec §2.1; `web/src/ws.ts`). */
export const CLAIMED_PARAM = "claimed";

/** A JWT segment's bytes: base64url, padding restored. Throws on a character
 *  outside the alphabet (`atob`). Shared with the sign-in arms (`signin.ts`). */
export const bytes = (b64url: string): Uint8Array => {
  const b64 = b64url.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(b64url.length / 4) * 4, "=");
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
};
/** A JWT segment parsed as JSON. Throws on bytes that are not JSON. The
 *  return type is not checked: `null`, a number or an array parse too, so a
 *  caller that needs an object tests for one. */
export const json = (b64url: string): Record<string, unknown> => JSON.parse(new TextDecoder().decode(bytes(b64url)));

/** ASCII letters lowercased and nothing else — `User::parse`'s
 *  `to_ascii_lowercase`, never JavaScript's `toLowerCase()`, which maps U+212A
 *  KELVIN SIGN to ASCII `k`. `address` runs it only on what `forwardable` let
 *  through, where the two agree; it is ASCII-only anyway, so a check moved
 *  later can never again turn a Kelvin sign into another user's `k`. */
export function asciiLower(s: string): string {
  return s.replace(/[A-Z]/g, (ch) => ch.toLowerCase());
}

/**
 * Whether `email` — trimmed and ASCII-lowercased, as `address` answers it and
 * as `shellConfig` reads an operator — is an ADDRESS by the layer's email
 * shape (`User::is_email`, the mode rule's test): at most 254 bytes, no
 * whitespace or control character, and exactly one `@` with something on both
 * sides. `User::parse`'s key grammar also takes a plain name (M6.5c2), but
 * under `worker` the layer's mode rule admits an address and nothing else, so
 * a value outside this rule names no caller the layer would accept (R1's L-2)
 * and no operator one could match. Wherever a CALLER is read the value is
 * visible ASCII with tabs and spaces (`forwardable`), where this test and the
 * layer's email shape agree character for character.
 */
export function isAddress(email: string): boolean {
  return new TextEncoder().encode(email).length <= 254 && /^[^@\s\p{Cc}]+@[^@\s\p{Cc}]+$/u.test(email);
}

/** The reserved domain every test identity's address ends in — `crates/yokohama-worker/src/receipt.rs`'s
 *  `TEST_DOMAIN`, restated. */
const TEST_DOMAIN = ".test";

/** Whether a normalized address is a TEST identity's (security S2b, finding B6; R-SEC8(d)): an `@` and the suffix
 *  `.test` — the rule is exactly the bucket's, the receipt archive's own rule for the `test/` root; what the dev
 *  door can name is narrower still (`isAddress` judges it first), so every name it can produce lands in the root
 *  the bucket keeps apart from the owner's training data. A plain name ending `.test` (`a.test`) is not one, as
 *  the module reads it. */
export function isTestAddress(email: string): boolean {
  return email.includes("@") && email.endsWith(TEST_DOMAIN);
}

/**
 * The address a raw value names — or, at the open door, the name a caller
 * claimed (`claimedIdentity`, M6.5c2): `raw` judged by `forwardable` BEFORE
 * anything else is done to it, then trimmed and ASCII-lowercased as the
 * layer's `User::parse` reads a caller — or `undefined` for a raw value that
 * is not forwardable, or blank.
 */
function address(raw: string): string | undefined {
  if (!forwardable(raw)) return undefined;
  const email = asciiLower(raw.trim());
  return email === "" ? undefined : email;
}

/** R-6.5c.3, in order. A missed Access destination, a forged token, another
 *  application's token and an expired one ALL fail closed: `none`, and the
 *  layer's own 401 answers. So does a token whose header names no `kid` —
 *  none is ever matched to a key that names none (the orchestrator's F-3) —
 *  and, at the dev door, a claimed value that is no address (`isAddress`;
 *  R1's L-2), which would otherwise be forwarded as a caller the layer
 *  refuses, and at `POST /api/archive` name an Archive object whose `init`
 *  refuses it — and, at the dev door, a claimed address outside `.test` (B6): the
 *  credential names test identities and nobody else.
 *
 *  The brief's text, with ONE change the orchestrator ruled at Task 11's
 *  review (I1 — the spec over the brief): the address, the `email` claim or
 *  the claimed header, is judged RAW by `forwardable` before any trim, and is
 *  lowercased in ASCII only (`address`). Normalized first, a verified
 *  `\u212Aevin@x.test` (a Kelvin sign) became `kevin@x.test`, another user,
 *  and `\u00A0red@t.test` became `red@t.test` — JavaScript's `trim()` strips
 *  non-ASCII whitespace, and its `toLowerCase()` maps the Kelvin sign to `k`.
 *  The NAS reads both as no one. */
export async function resolveIdentity(h: Headers, keys: JwkSet, c: IdentityConfig): Promise<Identity> {
  const token = h.get("cf-access-jwt-assertion");
  const parts = token?.split(".");
  if (!parts || parts.length !== 3) return { kind: "none" };
  let head: Record<string, unknown>, claims: Record<string, unknown>;
  try {
    head = json(parts[0]!);
    claims = json(parts[1]!);
  } catch {
    return { kind: "none" };
  }
  if (head.alg !== "RS256") return { kind: "none" };
  // A header with no string `kid` names no key: comparing `undefined` would
  // pick a key that names none. No refresh can find one, so it is plain `none`.
  if (typeof head.kid !== "string") return { kind: "none" };
  const jwk = keys.keys.find((k) => k.kid === head.kid);
  if (!jwk) return { kind: "none", unknownKid: true }; // the caller may refresh the keys once
  const key = await crypto.subtle.importKey("jwk", jwk, { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" }, false, ["verify"]);
  const signed = new TextEncoder().encode(`${parts[0]}.${parts[1]}`);
  if (!(await crypto.subtle.verify("RSASSA-PKCS1-v1_5", key, bytes(parts[2]!), signed))) return { kind: "none" };
  const aud = Array.isArray(claims.aud) ? claims.aud : [claims.aud];
  if (!aud.includes(c.aud)) return { kind: "none" };
  if (claims.iss !== `https://${c.teamDomain}`) return { kind: "none" };
  if (typeof claims.exp !== "number" || claims.exp <= c.nowSeconds) return { kind: "none" };
  if (typeof claims.nbf === "number" && claims.nbf > c.nowSeconds + 60) return { kind: "none" };
  if (typeof claims.email === "string" && claims.email.trim() !== "") {
    const email = address(claims.email);
    return email === undefined ? { kind: "none" } : { kind: "person", email };
  }
  if (c.devDoor && claims.common_name === c.devDoorClientId) {
    const claimed = address(h.get(CLAIMED_USER_HEADER) ?? "");
    if (claimed !== undefined && isAddress(claimed) && isTestAddress(claimed)) return { kind: "dev", email: claimed };
  }
  return { kind: "none" };
}

/**
 * **An open-door claim** (M6.5c2, spec §2.1): the raw name a caller sent —
 * `handle` picks the carrier, the header on a `fetch` and the `claimed`
 * parameter on an upgrade — judged by `address`: RAW by `forwardable` first,
 * so a percent-decoded parameter holding a character no header can carry is
 * no caller, rather than a `headers.set` that throws into the platform's plain
 * 500 (review R2.2-m3); then trimmed and ASCII-lowercased (the Kelvin lesson),
 * and each run of spaces collapsed to one — the key grammar's `typed_form`, so
 * every spelling of a key names ONE Archive object (security S2b). The shell
 * never judges a name beyond what a header can carry: the layer's key grammar
 * does, in the objects. A blank or absent claim is no one.
 */
export function claimedIdentity(raw: string | null): Identity {
  const lowered = raw === null ? undefined : address(raw);
  // The key grammar's own `typed_form`: each run of spaces is one, so every spelling of a key names ONE Archive object
  // (security S2b, D-5's race) and spends ONE write-limiter key (D-3).
  const name = lowered?.split(" ").filter((part) => part !== "").join(" ");
  return name === undefined || name === "" ? { kind: "none" } : { kind: "claimed", name };
}

/** The value an identity is forwarded as, and names its archive object by —
 *  a person's, the dev door's or a session's address, an open-door claim, or
 *  an anonymous upload's synthetic address — or `undefined` for no one
 *  (M6.5c2; the auth cycle). */
export function callerName(identity: Identity): string | undefined {
  switch (identity.kind) {
    case "person":
    case "dev":
    case "session":
      return identity.email;
    case "claimed":
      return identity.name;
    case "anonymous":
      // ASCII-lowercased as every caller name is (`address`; ruling R-32): a device id is lowercase hex already, and a
      // game id may carry ASCII capitals, so every spelling of one game names ONE archive object and one prefix.
      return `${ANON_LOCAL_PREFIX}${asciiLower(identity.device)}@${ANON_DOMAIN}`;
    case "none":
      return undefined;
  }
}

/** The header a browser's archive post names its device by (the auth cycle, design §4.4): sixteen lowercase hex digits
 *  the client draws once. Read RAW before the strip rule drops every `x-yokohama-*` header, and set again on the
 *  archive object's request when it is a device id (`isDeviceId`), for the object's metadata. */
export const DEVICE_HEADER = "X-Yokohama-Device";

/** The anonymous address's domain: `.invalid` is reserved by RFC 6761, so no player the Worker verifies can hold it. */
export const ANON_DOMAIN = "anon.invalid";

/** The anonymous address's local part begins so. */
const ANON_LOCAL_PREFIX = "anon-";

/** Whether `s` is a device id: exactly sixteen LOWERCASE hex digits and nothing else — no capital, no other length, no
 *  surrounding space, and no case mapping first (so a Kelvin sign is never read as anything). Anything else is an
 *  absent header. */
export function isDeviceId(s: string): boolean {
  return /^[0-9a-f]{16}$/.test(s);
}

/** Whether `address` is an anonymous upload's synthetic address (`callerName` of the `anonymous` kind): the archive
 *  object writes `anonymous` in its metadata for one, never the address. */
export function isAnonymousAddress(address: string): boolean {
  return address.startsWith(ANON_LOCAL_PREFIX) && address.endsWith(`@${ANON_DOMAIN}`);
}

/**
 * **The anonymous caller** of a caller-less `POST /api/archive` (the auth cycle, design §4.4; R-A4, R-A11): the device
 * header's value when it is a device id (`isDeviceId`); else the GAME's own id, read from the body's table document —
 * an archive post is `{"record": "…", "table": "…"}`, two strings (the layer's `ArchivePost`), so the id is the
 * `table` string's own `id`, read as a create's is (`bodyId`) — when the store would accept it (`isSafeId`); else no
 * one, and the house's `dispatch` answers the layer's 401 as before. `headers` are the request's as sent, before the
 * strip rule. The body is parsed for this one member and forwarded as the bytes it was.
 */
export function anonymousIdentity(headers: Headers, body: Uint8Array | null): Identity {
  const device = headers.get(DEVICE_HEADER);
  if (device !== null && isDeviceId(device)) return { kind: "anonymous", device };
  const table = bodyMember(body, "table");
  const game = table === undefined ? undefined : bodyId(new TextEncoder().encode(table));
  return game !== undefined && isSafeId(game) ? { kind: "anonymous", device: game } : { kind: "none" };
}

/** A body's `id` when it is a JSON object whose `id` is a string (spec §2.3's create row); otherwise `undefined`. It
 *  READS, and `isSafeId` judges. The body is parsed for this one member and forwarded as the bytes it was: nothing here
 *  re-serializes it. Bytes that are not UTF-8, or that open with a byte-order mark, name no id — the layer's own parse
 *  refuses both. */
export function bodyId(body: Uint8Array | null): string | undefined {
  return bodyMember(body, "id");
}

/** The string member `key` of a body that is a JSON object, or `undefined` — `bodyId`'s reading, for any member. */
function bodyMember(body: Uint8Array | null, key: string): string | undefined {
  if (body === null) return undefined;
  let parsed: unknown;
  try {
    parsed = JSON.parse(new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(body));
  } catch {
    return undefined;
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) return undefined;
  const value = (parsed as Record<string, unknown>)[key];
  return typeof value === "string" ? value : undefined;
}

/**
 * A key set — the team's, or a sign-in arm's issuer's: `refresh: false` for
 * the cached set, `refresh: true` when a token names a `kid` the cached set
 * lacks. Rejects when the keys cannot be had, and the caller reads that as no
 * caller at all.
 */
export type KeySource = (refresh: boolean) => Promise<JwkSet>;

/** How long a refetch of the keys waits after the last fetch. */
const REFRESH_MS = 60_000;

/** Whether a refresh may fetch the keys again: there has been no fetch, or
 *  the last one was at least a minute ago. */
export function refreshDue(lastRefreshAt: number | null, now: number): boolean {
  return lastRefreshAt === null || now - lastRefreshAt >= REFRESH_MS;
}

/**
 * The team's keys, from `https://<teamDomain>/cdn-cgi/access/certs`: the
 * Access certs URL handed to `keySourceAt`, which says how they are cached.
 */
export function keySource(teamDomain: string, fetcher: typeof fetch = fetch, clock: () => number = Date.now): KeySource {
  return keySourceAt(`https://${teamDomain}/cdn-cgi/access/certs`, fetcher, clock);
}

/**
 * The key set at `url`, cached in this closure: the Access certs (`keySource`)
 * or a sign-in arm's issuer's JWKS (the auth cycle, Task 3). The Worker holds
 * ONE closure per isolate and URL (plan review m14; re-review m-d).
 *
 * - `false` answers the cache; with none, it joins the fetch in flight, or
 *   starts one.
 * - `true` joins the fetch in flight; with none, it fetches again only when
 *   `refreshDue` says so, and otherwise answers the cache — or, with no cache
 *   at all, fetches as `false` does. So an assertion forging an unknown `kid`
 *   costs at most one fetch a minute, whoever sends it — the limit lives
 *   HERE, so `handle` holds no limiter state.
 * - EVERY fetch records its time when it STARTS, the first included, and the
 *   calls that arrive while it is out join it (Task 11's review, M1): a
 *   forged `kid` just after a cold start cannot force a second fetch, not
 *   even before the first has landed.
 * - A fetch that fails, answers anything but 200, or answers no key set
 *   REJECTS every call that joined it, and the cache is left as it was.
 */
export function keySourceAt(url: string, fetcher: typeof fetch = fetch, clock: () => number = Date.now): KeySource {
  let cached: JwkSet | undefined;
  let lastRefreshAt: number | null = null;
  let inflight: Promise<JwkSet> | undefined;
  const load = async (): Promise<JwkSet> => {
    const res = await fetcher(url);
    if (res.status !== 200) throw new Error(`the key set at ${url} answered ${res.status}`);
    const body: unknown = await res.json();
    const keys = (body as { keys?: unknown } | null)?.keys;
    if (!Array.isArray(keys)) throw new Error(`the answer at ${url} holds no key set`);
    cached = { keys: keys as JwkSet["keys"] };
    return cached;
  };
  const refetch = (): Promise<JwkSet> => {
    lastRefreshAt = clock();
    // Cleared when it settles — asynchronously, so always after this
    // assignment — and only if no later fetch has replaced it.
    const pending = load().finally(() => {
      if (inflight === pending) inflight = undefined;
    });
    inflight = pending;
    return pending;
  };
  return async (refresh) => {
    if (inflight !== undefined && (cached === undefined || refresh)) return inflight;
    if (cached === undefined || (refresh && refreshDue(lastRefreshAt, clock()))) return refetch();
    return cached;
  };
}

/**
 * The configuration `resolveIdentity` reads, from the Worker's variables and
 * the dev door's switch (`shellConfig`'s `devDoor`) — or `null` when no
 * assertion can be verified at all: with no team domain or no audience,
 * every caller is no one, and no key is ever fetched.
 *
 * The dev door opens only with a client id to compare against: with none,
 * no credential's `common_name` is the one, whatever `YOKOHAMA_DEV_DOOR`
 * says.
 */
export function identityConfig(env: Env, devDoor: boolean, nowSeconds: number): IdentityConfig | null {
  const teamDomain = env.ACCESS_TEAM_DOMAIN ?? "";
  const aud = env.ACCESS_AUD ?? "";
  if (teamDomain === "" || aud === "") return null;
  const devDoorClientId = env.DEV_DOOR_CLIENT_ID ?? "";
  return { teamDomain, aud, devDoorClientId, devDoor: devDoor && devDoorClientId !== "", nowSeconds };
}

/**
 * Whether a RAW address — an assertion's `email` claim, or the claimed
 * header, before anything is trimmed or lowercased — can travel as the
 * objects' `X-Yokohama-User`; or, at the open door (M6.5c2), whether a
 * claimed name — the header, or an upgrade's percent-decoded `claimed`
 * parameter, which can hold a character on which `headers.set` throws, such
 * as a carriage return (review R2.2-m3) — can travel as their
 * `X-Yokohama-Claimed-User`: every character a tab, a space or visible
 * ASCII, the bytes the layer's own `resolve` reads a header with (`http`'s
 * `HeaderValue::to_str`), which reads any other value as ABSENT. workerd would
 * send a non-ASCII value on regardless, as UTF-8, with a warning that a
 * browser's `Headers` would throw. So a raw value outside the set is no caller
 * here: `resolveIdentity` or `claimedIdentity` answers `none` for it, no
 * identity header is forwarded, and the layer answers as it does for no one
 * (the orchestrator's rulings:
 * Task 2's review, E1; Task 11's review, I1 — judged raw, because JavaScript's
 * `trim()` and `toLowerCase()` would first turn some such addresses into
 * visible ASCII, and into another user's).
 */
export function forwardable(email: string): boolean {
  return /^[\t\x20-\x7e]*$/.test(email);
}
