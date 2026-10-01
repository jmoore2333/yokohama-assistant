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

export type Identity =
  | { kind: "person"; email: string }
  | { kind: "dev"; email: string }
  | { kind: "none"; unknownKid?: true };
export interface JwkSet { keys: Array<JsonWebKey & { kid?: string }> }
export interface IdentityConfig {
  teamDomain: string; aud: string; devDoorClientId: string; devDoor: boolean; nowSeconds: number;
}

const bytes = (b64url: string): Uint8Array => {
  const b64 = b64url.replace(/-/g, "+").replace(/_/g, "/").padEnd(Math.ceil(b64url.length / 4) * 4, "=");
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
};
const json = (b64url: string): Record<string, unknown> => JSON.parse(new TextDecoder().decode(bytes(b64url)));

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

/**
 * The address a raw value names: `raw` judged by `forwardable` BEFORE
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
 *  refuses it.
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
    const claimed = address(h.get("x-yokohama-claimed-user") ?? "");
    if (claimed !== undefined && isAddress(claimed)) return { kind: "dev", email: claimed };
  }
  return { kind: "none" };
}

/**
 * The team's keys: `refresh: false` for the cached set, `refresh: true` when
 * an assertion names a `kid` the cached set lacks. Rejects when the keys
 * cannot be had, and the caller reads that as no caller at all.
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
 * The team's keys, from `https://<teamDomain>/cdn-cgi/access/certs`, cached in
 * this closure — ONE per isolate, which is how the Worker's default export
 * holds it (plan review m14; re-review m-d).
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
export function keySource(teamDomain: string, fetcher: typeof fetch = fetch, clock: () => number = Date.now): KeySource {
  let cached: JwkSet | undefined;
  let lastRefreshAt: number | null = null;
  let inflight: Promise<JwkSet> | undefined;
  const load = async (): Promise<JwkSet> => {
    const res = await fetcher(`https://${teamDomain}/cdn-cgi/access/certs`);
    if (res.status !== 200) throw new Error(`the Access certs answered ${res.status}`);
    const body: unknown = await res.json();
    const keys = (body as { keys?: unknown } | null)?.keys;
    if (!Array.isArray(keys)) throw new Error("the Access certs hold no key set");
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
 * objects' `X-Yokohama-User`: every character a tab, a space or visible
 * ASCII, the bytes the layer's own `resolve` reads a header with (`http`'s
 * `HeaderValue::to_str`), which reads any other value as ABSENT. workerd would
 * send such a value on regardless, as UTF-8, with a warning that a browser's
 * `Headers` would throw. So a raw address outside the set is no caller here:
 * `resolveIdentity` answers `none` for it, no identity header is forwarded,
 * and the layer answers as it does for no one (the orchestrator's rulings:
 * Task 2's review, E1; Task 11's review, I1 — judged raw, because JavaScript's
 * `trim()` and `toLowerCase()` would first turn some such addresses into
 * visible ASCII, and into another user's).
 */
export function forwardable(email: string): boolean {
  return /^[\t\x20-\x7e]*$/.test(email);
}
