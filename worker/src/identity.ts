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

/** R-6.5c.3, in order. A missed Access destination, a forged token, another
 *  application's token and an expired one ALL fail closed: `none`, and the
 *  layer's own 401 answers. */
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
    return { kind: "person", email: claims.email.trim().toLowerCase() };
  }
  if (c.devDoor && claims.common_name === c.devDoorClientId) {
    const claimed = h.get("x-yokohama-claimed-user")?.trim().toLowerCase();
    if (claimed) return { kind: "dev", email: claimed };
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
 * - `false` answers the cache, and fetches only while there is none.
 * - `true` fetches again only when `refreshDue` says so, and otherwise answers
 *   the cache. So an assertion forging an unknown `kid` costs at most one
 *   fetch a minute, whoever sends it — the limit lives HERE, so `handle` holds
 *   no limiter state.
 * - EVERY fetch records its time, the first included, so a forged `kid` just
 *   after a cold start cannot force a second fetch.
 * - A fetch that fails, answers anything but 200, or answers no key set
 *   REJECTS, and the cache is left as it was.
 */
export function keySource(teamDomain: string, fetcher: typeof fetch = fetch, clock: () => number = Date.now): KeySource {
  let cached: JwkSet | undefined;
  let lastRefreshAt: number | null = null;
  const fetchKeys = async (): Promise<JwkSet> => {
    lastRefreshAt = clock();
    const res = await fetcher(`https://${teamDomain}/cdn-cgi/access/certs`);
    if (res.status !== 200) throw new Error(`the Access certs answered ${res.status}`);
    const body: unknown = await res.json();
    const keys = (body as { keys?: unknown } | null)?.keys;
    if (!Array.isArray(keys)) throw new Error("the Access certs hold no key set");
    cached = { keys: keys as JwkSet["keys"] };
    return cached;
  };
  return async (refresh) => {
    if (cached === undefined || (refresh && refreshDue(lastRefreshAt, clock()))) return fetchKeys();
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
 * Whether an address can travel as the objects' `X-Yokohama-User`: every
 * character a tab, a space or visible ASCII — the bytes the layer's own
 * `resolve` reads a header with (`http`'s `HeaderValue::to_str`), which reads
 * any other value as ABSENT. workerd would send such a value on regardless,
 * as UTF-8 and with a warning that a browser's `Headers` would throw. So an
 * address outside the set is no caller here: no identity header is forwarded
 * at all, and the layer answers as it does for no one (the orchestrator's
 * ruling, Task 2's review).
 */
export function forwardable(email: string): boolean {
  return /^[\t\x20-\x7e]*$/.test(email);
}
