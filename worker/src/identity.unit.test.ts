import { beforeAll, describe, expect, it } from "vitest";
import { credentialClaims, personClaims, signer, type Signer } from "../test/tokens";
import type { Env } from "./env";
import {
  forwardable,
  identityConfig,
  keySource,
  refreshDue,
  resolveIdentity,
  type IdentityConfig,
  type JwkSet,
} from "./identity";

const TEAM = "team.cloudflareaccess.com";
const AUD = "the-audience";
const CLIENT = "the-client.access";
const NOW = 1_789_000_000;

/** The configuration every cell starts from: the door ON, at `NOW`. */
const CONFIG: IdentityConfig = { teamDomain: TEAM, aud: AUD, devDoorClientId: CLIENT, devDoor: true, nowSeconds: NOW };

const at = { teamDomain: TEAM, aud: AUD, now: NOW };

let key: Signer;
/** A second key pair, never in any set: what a forger holds. */
let forger: Signer;

beforeAll(async () => {
  key = await signer("k1");
  forger = await signer("k-forger");
});

/** Request headers carrying `token` as the Access assertion, and any others. */
function headers(token: string | undefined, extra: Record<string, string> = {}): Headers {
  const h = new Headers(extra);
  if (token !== undefined) h.set("Cf-Access-Jwt-Assertion", token);
  return h;
}

/** What `resolveIdentity` makes of `token` against the one key's set. */
async function resolve(token: string | undefined, extra: Record<string, string> = {}, c = CONFIG) {
  return resolveIdentity(headers(token, extra), key.set(), c);
}

// R-6.5c.3, in order (spec §2.3). Battery 15 is every `none` below: an
// assertion that is not Access's, for THIS application, now, names nobody.
describe("resolveIdentity (R-6.5c.3)", () => {
  it("answers a person for a person's token, trimmed and lowercased as the layer reads a caller", async () => {
    expect(await resolve(await key.sign(personClaims(" Red@T.Test ", at)))).toEqual({ kind: "person", email: "red@t.test" });
  });

  // Mutant: the claimed header read before (or instead of) the token's email.
  it("ignores a client's X-Yokohama-Claimed-User beside a person's token", async () => {
    const token = await key.sign(personClaims("red@t.test", at));
    expect(await resolve(token, { "X-Yokohama-Claimed-User": "evil@x.test" })).toEqual({
      kind: "person",
      email: "red@t.test",
    });
  });

  it("answers dev for the credential's token with the door on and a claimed header", async () => {
    const token = await key.sign(credentialClaims(CLIENT, at));
    expect(await resolve(token, { "X-Yokohama-Claimed-User": " Probe@Deploy.Test " })).toEqual({
      kind: "dev",
      email: "probe@deploy.test",
    });
  });

  // Mutants: the door's switch not read; a claimed header honoured without a
  // credential; the client id not compared.
  it("answers none for the credential with the door off, with no claimed header, or for another client", async () => {
    const claimed = { "X-Yokohama-Claimed-User": "probe@deploy.test" };
    const credential = await key.sign(credentialClaims(CLIENT, at));
    expect(await resolve(credential, claimed, { ...CONFIG, devDoor: false })).toEqual({ kind: "none" });
    expect(await resolve(credential)).toEqual({ kind: "none" });
    expect(await resolve(credential, { "X-Yokohama-Claimed-User": "   " })).toEqual({ kind: "none" });
    const another = await key.sign(credentialClaims("another-client.access", at));
    expect(await resolve(another, claimed)).toEqual({ kind: "none" });
  });

  // BATTERY 15: the dev door is never honoured without the credential's own
  // verified assertion. Mutants: the claimed header read before the token is
  // (so it answers with no assertion at all, or beside a token that fails);
  // a token naming neither a person nor the credential let through.
  it("(battery 15) answers none for a claimed header with no credential's assertion", async () => {
    const claimed = { "X-Yokohama-Claimed-User": "probe@deploy.test" };
    expect(await resolve(undefined, claimed)).toEqual({ kind: "none" });
    expect(await resolve("not.a.token", claimed)).toEqual({ kind: "none" });
    const nobody = await key.sign({ ...credentialClaims(CLIENT, at), common_name: undefined });
    expect(await resolve(nobody, claimed)).toEqual({ kind: "none" });
    const expired = await key.sign({ ...credentialClaims(CLIENT, at), exp: NOW - 1 });
    expect(await resolve(expired, claimed)).toEqual({ kind: "none" });
    const forged = await forger.sign(credentialClaims(CLIENT, at), { alg: "RS256", kid: "k1", typ: "JWT" });
    expect(await resolve(forged, claimed)).toEqual({ kind: "none" });
  });

  // Battery 15's other half: each defect alone, over a token that is
  // otherwise a person's.
  it.each<[string, (s: Signer, f: Signer) => Promise<string | undefined>]>([
    ["another audience", (s) => s.sign({ ...personClaims("red@t.test", at), aud: ["another-audience"] })],
    ["another issuer", (s) => s.sign({ ...personClaims("red@t.test", at), iss: "https://evil.cloudflareaccess.com" })],
    ["an expired token", (s) => s.sign({ ...personClaims("red@t.test", at), exp: NOW - 1 })],
    ["a token that expires this second", (s) => s.sign({ ...personClaims("red@t.test", at), exp: NOW })],
    ["a token with no expiry", (s) => s.sign({ ...personClaims("red@t.test", at), exp: undefined })],
    ["a token not valid for another 61 seconds", (s) => s.sign({ ...personClaims("red@t.test", at), nbf: NOW + 61 })],
    // Signed by a key the set does not hold, under a kid the set DOES hold.
    ["a forged signature", (_s, f) => f.sign(personClaims("red@t.test", at), { alg: "RS256", kid: "k1", typ: "JWT" })],
    ["alg none", async (s) => (await s.sign(personClaims("red@t.test", at), { alg: "none", kid: "k1" })).replace(/\.[^.]*$/, ".")],
    ["an HS256 header", (s) => s.sign(personClaims("red@t.test", at), { alg: "HS256", kid: "k1" })],
    ["a token of two parts", async (s) => (await s.sign(personClaims("red@t.test", at))).split(".").slice(0, 2).join(".")],
    ["a token of four parts", async (s) => `${await s.sign(personClaims("red@t.test", at))}.x`],
    ["a header that is not JSON", async (s) => `bm90IGpzb24.${(await s.sign(personClaims("red@t.test", at))).split(".").slice(1).join(".")}`],
    ["no assertion at all", async () => undefined],
  ])("answers none for %s", async (_name, make) => {
    expect(await resolve(await make(key, forger))).toEqual({ kind: "none" });
  });

  // The boundaries of the two time checks: `exp` is exclusive, and `nbf`
  // allows a minute of skew. Mutants: `<` for `<=`; the skew dropped or widened.
  it("accepts a token valid for one more second, and one not valid for another 60", async () => {
    const person = { kind: "person", email: "red@t.test" };
    expect(await resolve(await key.sign({ ...personClaims("red@t.test", at), exp: NOW + 1 }))).toEqual(person);
    expect(await resolve(await key.sign({ ...personClaims("red@t.test", at), nbf: NOW + 60 }))).toEqual(person);
    expect(await resolve(await key.sign({ ...personClaims("red@t.test", at), aud: "the-audience" }))).toEqual(person);
  });

  it("answers none with unknownKid for a kid the set lacks, so the caller may refresh once", async () => {
    const token = await key.sign(personClaims("red@t.test", at), { alg: "RS256", kid: "k-rotated", typ: "JWT" });
    expect(await resolve(token)).toEqual({ kind: "none", unknownKid: true });
  });
});

describe("refreshDue (plan review m14)", () => {
  it("is due with no fetch yet, not 59,999 ms after one, and due at 60,000", () => {
    expect(refreshDue(null, 5)).toBe(true);
    expect(refreshDue(1_000, 60_999)).toBe(false);
    expect(refreshDue(1_000, 61_000)).toBe(true);
  });
});

describe("keySource (plan review m14; re-review m-d)", () => {
  const SET: JwkSet = { keys: [{ kid: "k1", kty: "RSA", n: "AQAB", e: "AQAB" }] };

  /** A fake certs endpoint that counts its fetches and the URLs asked for. */
  function endpoint(answer: () => Promise<Response>) {
    const urls: string[] = [];
    const fetcher = (async (input: RequestInfo | URL) => {
      urls.push(String(input));
      return answer();
    }) as typeof fetch;
    return { urls, fetcher };
  }

  it("fetches the team's certs once, then answers the cache; refreshes only once a minute", async () => {
    let now = 1_000_000;
    const { urls, fetcher } = endpoint(async () => Response.json(SET));
    const keys = keySource(TEAM, fetcher, () => now);
    expect(await keys(false)).toEqual(SET);
    expect(urls).toEqual([`https://${TEAM}/cdn-cgi/access/certs`]);
    expect(await keys(false)).toEqual(SET);
    expect(urls.length, "a second false answers the cache").toBe(1);
    now += 1_000;
    expect(await keys(true)).toEqual(SET);
    // The first fetch counted, so a forged kid just after a cold start cannot
    // force a second one (Mutant: only a REFRESH records the time).
    expect(urls.length, "a refresh a second after the first fetch answers the cache").toBe(1);
    now += 59_000;
    expect(await keys(true)).toEqual(SET);
    expect(urls.length, "a refresh 60 s after the last fetch fetches again").toBe(2);
    expect(await keys(true)).toEqual(SET);
    expect(urls.length, "and that fetch restarts the minute").toBe(2);
  });

  // A failed fetch rejects, and `handle` reads that as no caller: the request
  // fails CLOSED.
  it("rejects when the certs cannot be fetched, read, or are not a key set", async () => {
    const refused = endpoint(async () => {
      throw new TypeError("network down");
    });
    await expect(keySource(TEAM, refused.fetcher, () => 0)(false)).rejects.toThrow("network down");
    const status = endpoint(async () => new Response("no", { status: 503 }));
    await expect(keySource(TEAM, status.fetcher, () => 0)(false)).rejects.toThrow(/503/);
    const shape = endpoint(async () => Response.json({ public_cert: {} }));
    await expect(keySource(TEAM, shape.fetcher, () => 0)(false)).rejects.toThrow(/no key set/);
  });

  // After a failure the cache is still empty, so the next call fetches again
  // rather than answering nothing.
  it("fetches again after a failure, since there is nothing cached to answer", async () => {
    let fail = true;
    const { urls, fetcher } = endpoint(async () => {
      if (fail) throw new TypeError("network down");
      return Response.json(SET);
    });
    const keys = keySource(TEAM, fetcher, () => 0);
    await expect(keys(false)).rejects.toThrow();
    fail = false;
    expect(await keys(false)).toEqual(SET);
    expect(urls.length).toBe(2);
  });
});

describe("identityConfig", () => {
  /** An `Env` with only these variables set; the bindings are never read. */
  const env = (vars: Partial<Env>) => ({ ...vars }) as Env;
  const ACCESS = { ACCESS_TEAM_DOMAIN: TEAM, ACCESS_AUD: AUD, DEV_DOOR_CLIENT_ID: CLIENT };

  it("carries the Access values, the door and the time", () => {
    expect(identityConfig(env(ACCESS), true, NOW)).toEqual(CONFIG);
    expect(identityConfig(env(ACCESS), false, NOW)?.devDoor).toBe(false);
  });
  // Mutant: an absent value passed on as "" — nothing verifies then either,
  // but every request would try to fetch `https:///cdn-cgi/access/certs`, and
  // log that it failed.
  it("is null — no caller can be verified — without the team domain or the audience", () => {
    expect(identityConfig(env({ ...ACCESS, ACCESS_TEAM_DOMAIN: undefined }), true, NOW)).toBeNull();
    expect(identityConfig(env({ ...ACCESS, ACCESS_AUD: "" }), true, NOW)).toBeNull();
  });
  // Mutant: the door left open with no client id to compare against.
  it("shuts the door with no client id, whatever YOKOHAMA_DEV_DOOR says", () => {
    expect(identityConfig(env({ ...ACCESS, DEV_DOOR_CLIENT_ID: undefined }), true, NOW)).toMatchObject({
      devDoor: false,
      devDoorClientId: "",
    });
  });
});

// The orchestrator's ruling (Task 2's review, E1): the layer's own `resolve`
// reads a header value outside visible ASCII as ABSENT — and workerd sends
// one on as UTF-8, warning that a browser would throw — so such an address is
// no caller, and is never forwarded as a header.
describe("forwardable", () => {
  it("accepts visible ASCII, a space and a tab", () => {
    expect(forwardable("red@t.test")).toBe(true);
    expect(forwardable("a b\t@x.test")).toBe(true);
  });
  it.each(["rené@x.test", "用户@例子.广告", "red@t.test\u0000", "red@t.test\u007f", "red\n@t.test"])(
    "refuses %j",
    (email) => {
      expect(forwardable(email)).toBe(false);
    },
  );
});
