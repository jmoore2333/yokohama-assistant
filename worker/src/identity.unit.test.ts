import { beforeAll, describe, expect, it } from "vitest";
import { credentialClaims, personClaims, signer, type Signer } from "../test/tokens";
import type { Env } from "./env";
import {
  asciiLower,
  callerName,
  claimedIdentity,
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

  // I1 (Task 11's review; the orchestrator's ruling, the spec over the
  // brief's text): the address is checked RAW, before any trim or lowercase.
  // JavaScript's `toLowerCase()` maps U+212A KELVIN SIGN to ASCII `k`, and its
  // `trim()` strips non-ASCII whitespace, so an address normalized first
  // became ANOTHER user's — a verified `\u212Aevin@x.test` (U+212A, the Kelvin
  // sign) was forwarded as `kevin@x.test`. The NAS reads each of these as no
  // one.
  // Mutants: the check after the trim, or after the lowercase.
  it.each([
    ["a Kelvin sign", "\u212Aevin@x.test"],
    ["a leading no-break space", "\u00A0red@t.test"],
    ["a trailing ideographic space", "red@t.test\u3000"],
    ["a leading byte-order mark", "\uFEFFred@t.test"],
  ])("(I1) answers none for a person whose raw email holds %s", async (_what, email) => {
    expect(await resolve(await key.sign(personClaims(email, at)))).toEqual({ kind: "none" });
  });
  // The claimed header's half. Node's `Headers` refuses a character above
  // U+00FF outright, so the Kelvin sign's claimed case is the next cell's,
  // over a stand-in; these two a node `Headers` can carry.
  it.each([
    ["a leading no-break space", "\u00A0probe@deploy.test"],
    ["a trailing no-break space", "probe@deploy.test\u00A0"],
  ])("(I1) answers none for a claimed address holding %s", async (_what, claimed) => {
    const token = await key.sign(credentialClaims(CLIENT, at));
    expect(await resolve(token, { "X-Yokohama-Claimed-User": claimed })).toEqual({ kind: "none" });
  });
  // The Kelvin sign in the claimed header. A request carries it as UTF-8, but
  // node's `Headers` refuses to hold a character above U+00FF (and workerd's
  // warns on it), so the headers here are a stand-in with `get` alone — all
  // `resolveIdentity` reads — checked against a plain address first, so the
  // stand-in cannot pass by reading nothing.
  it("(I1) answers none for a claimed address holding a Kelvin sign", async () => {
    const token = await key.sign(credentialClaims(CLIENT, at));
    const headersOf = (claimed: string) => {
      const values: Record<string, string> = { "cf-access-jwt-assertion": token, "x-yokohama-claimed-user": claimed };
      return { get: (name: string) => values[name.toLowerCase()] ?? null } as unknown as Headers;
    };
    expect(await resolveIdentity(headersOf("probe@deploy.test"), key.set(), CONFIG), "premise").toEqual({
      kind: "dev",
      email: "probe@deploy.test",
    });
    expect(await resolveIdentity(headersOf("\u212Aevin@x.test"), key.set(), CONFIG)).toEqual({ kind: "none" });
  });
  // The ruling's second half: only A–Z are lowercased. `address` checks first,
  // so no Kelvin sign reaches this, and the rule is pinned here, where a
  // JavaScript `toLowerCase()` would fail it. Mutant: `toLowerCase()`.
  it("lowercases A to Z and nothing else: a Kelvin sign stays itself", () => {
    expect(asciiLower("Red@T.TEST")).toBe("red@t.test");
    expect(asciiLower("\u212AEVIN")).toBe("\u212Aevin");
    expect(asciiLower("İ")).toBe("İ");
  });
  // Only ASCII is lowercased, as the layer's `User::parse` does
  // (`to_ascii_lowercase`); ASCII spaces and tabs around a visible-ASCII
  // address are still trimmed, as the layer trims them.
  it("trims ASCII space and tab, and lowercases ASCII letters", async () => {
    expect(await resolve(await key.sign(personClaims("\t Red@T.Test \t", at)))).toEqual({ kind: "person", email: "red@t.test" });
  });

  // M3 (Task 11's review): an `email` claim that is empty, blank or null names
  // no person, so a credential carrying one is still the dev door. Mutant:
  // the blank check dropped — such a credential would become a person with no
  // address, and the door would shut.
  it.each<[string, unknown]>([
    ["an empty", ""],
    ["a blank", "   "],
    ["a null", null],
  ])("a credential whose assertion carries %s email is still the dev door", async (_what, email) => {
    const token = await key.sign({ ...credentialClaims(CLIENT, at), email });
    expect(await resolve(token, { "X-Yokohama-Claimed-User": "probe@deploy.test" })).toEqual({
      kind: "dev",
      email: "probe@deploy.test",
    });
  });
  // Mutant: the type check dropped — `(5).trim` throws, and `resolve` rejects.
  it("answers none for an email claim that is not a string", async () => {
    expect(await resolve(await key.sign({ ...personClaims("red@t.test", at), email: 5 }))).toEqual({ kind: "none" });
  });

  it("answers none with unknownKid for a kid the set lacks, so the caller may refresh once", async () => {
    const token = await key.sign(personClaims("red@t.test", at), { alg: "RS256", kid: "k-rotated", typ: "JWT" });
    expect(await resolve(token)).toEqual({ kind: "none", unknownKid: true });
  });

  // F-3 (the orchestrator's reading of the shell): an assertion whose header
  // names no `kid` matches no key — never a key that names none, which a set
  // could hold (Access's does not). No refresh can find a key for it, so it is
  // plain `none`. Mutant: the kid's type check dropped — `undefined ===
  // undefined` picks the kid-less key, and the token verifies against it.
  it("(F-3) answers none for a token that names no kid, even beside a key that names none", async () => {
    const { kid: _named, ...bare } = key.jwk;
    const kidless: JwkSet = { keys: [bare] };
    const token = await key.sign(personClaims("red@t.test", at), { alg: "RS256", typ: "JWT" });
    expect(await resolveIdentity(headers(token), kidless, CONFIG)).toEqual({ kind: "none" });
    const numbered = await key.sign(personClaims("red@t.test", at), { alg: "RS256", kid: 1, typ: "JWT" });
    expect(await resolveIdentity(headers(numbered), kidless, CONFIG), "a kid that is not a string").toEqual({ kind: "none" });
    // Anti-vacuity: the same key under its kid, and a token naming it, is the person.
    expect(await resolve(await key.sign(personClaims("red@t.test", at)))).toEqual({ kind: "person", email: "red@t.test" });
  });

  // R1's L-2: the dev door's claimed value is an ADDRESS by the layer's email
  // shape (`User::is_email`, the mode rule's test) — one `@` with something on both sides, no
  // whitespace or control character, at most 254 bytes — or it is no one, and
  // the layer's own 401 answers. A non-address was the dev identity, and at
  // `POST /api/archive` it named an Archive object whose `init` refused it: a
  // 500 no spec lists. Mutant: the rule dropped.
  it.each([
    ["no @", "not-an-email"],
    ["two @", "a@b@c.test"],
    ["nothing before the @", "@t.test"],
    ["nothing after the @", "red@"],
    ["an inner space", "red @t.test"],
    ["an inner tab", "red@t.\ttest"],
    ["255 bytes", `${"a".repeat(248)}@t.test`],
  ])("(L-2) answers none for a claimed value that is no address: %s", async (_what, claimed) => {
    const token = await key.sign(credentialClaims(CLIENT, at));
    expect(await resolve(token, { "X-Yokohama-Claimed-User": claimed })).toEqual({ kind: "none" });
  });
  // The boundary's other side, and the anti-vacuity partner: 254 bytes is an
  // address, as it is to the layer's email shape.
  it("(L-2) takes a claimed address of exactly 254 bytes", async () => {
    const token = await key.sign(credentialClaims(CLIENT, at));
    const claimed = `${"a".repeat(247)}@t.test`;
    expect(claimed.length, "premise").toBe(254);
    expect(await resolve(token, { "X-Yokohama-Claimed-User": claimed })).toEqual({ kind: "dev", email: claimed });
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

  /** A second key set: what the certs answer after a rotation. */
  const ROTATED: JwkSet = { keys: [{ kid: "k2", kty: "RSA", n: "AQAB", e: "AQAB" }] };

  /** A fake certs endpoint whose every answer waits for `release()`: its
   *  first fetch answers `SET` and every later one `ROTATED`; after
   *  `failNext()`, the next fetch it receives fails. */
  function gated() {
    const urls: string[] = [];
    const waiting: Array<() => void> = [];
    let fail = false;
    const fetcher = (async (input: RequestInfo | URL) => {
      urls.push(String(input));
      const answer = urls.length === 1 ? SET : ROTATED;
      const failing = fail;
      fail = false;
      await new Promise<void>((r) => waiting.push(r));
      if (failing) throw new TypeError("network down");
      return Response.json(answer);
    }) as typeof fetch;
    return {
      urls,
      fetcher,
      failNext: () => {
        fail = true;
      },
      release: () => {
        for (const r of waiting.splice(0)) r();
      },
    };
  }

  // M1 (Task 11's review): the requests of a cold isolate that arrive while
  // the first fetch is out JOIN it — a forged kid among them included — so the
  // once-a-minute limit holds from the first request, not from the first
  // fetch that lands. Mutant: the cache filled only when a fetch completes,
  // with nothing held for the fetch in flight.
  it("shares ONE fetch among the calls that arrive while it is in flight", async () => {
    const endpoint = gated();
    const keys = keySource(TEAM, endpoint.fetcher, () => 0);
    const calls = [keys(false), keys(false), keys(true)];
    endpoint.release();
    expect(await Promise.all(calls)).toEqual([SET, SET, SET]);
    expect(endpoint.urls.length).toBe(1);
  });
  // Mutant: a refresh that is not due answering the OLD cache while a refresh
  // is in flight — the kid the rotation brings would be missed.
  it("lets a refresh join the refresh already in flight", async () => {
    let now = 0;
    const endpoint = gated();
    const keys = keySource(TEAM, endpoint.fetcher, () => now);
    const first = keys(false);
    endpoint.release();
    expect(await first).toEqual(SET);
    now = 60_000;
    const refreshes = [keys(true), keys(true)];
    // A `false` meanwhile answers the cache at once rather than waiting.
    expect(await keys(false)).toEqual(SET);
    endpoint.release();
    expect(await Promise.all(refreshes), "both refreshes read the rotated set").toEqual([ROTATED, ROTATED]);
    expect(endpoint.urls.length).toBe(2);
  });
  it("fails every call that joined a failed fetch, and fetches again after it", async () => {
    const endpoint = gated();
    const keys = keySource(TEAM, endpoint.fetcher, () => 0);
    endpoint.failNext();
    const joined = [keys(false), keys(false)];
    endpoint.release();
    await expect(joined[0]).rejects.toThrow("network down");
    await expect(joined[1]).rejects.toThrow("network down");
    expect(endpoint.urls.length, "the two calls shared the fetch that failed").toBe(1);
    const next = keys(false);
    endpoint.release();
    expect(await next).toEqual(ROTATED);
    expect(endpoint.urls.length, "nothing cached and nothing in flight: a new fetch").toBe(2);
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

// M6.5c2, spec §2.1 (review R2.2-m3, R2.2-m6): an open-door claim — the header
// on a fetch, the percent-DECODED `claimed` parameter on an upgrade, so any
// character at all — judged as `address` judges an assertion's `email`: RAW by
// `forwardable`, then trimmed and ASCII-lowercased, each run of spaces made one
// (security S2b, AD-3: one name, one Archive object). The shell never judges it
// further: the layer's key grammar does, in the objects. Mutants: normalized
// before it is judged (a Kelvin sign becomes `k`, an NBSP-led `ben` becomes
// `ben`); a blank claim made a caller; the claim made a `person` (an
// operator's rights); space runs kept (M44c).
describe("claimedIdentity (M6.5c2, spec §2.1)", () => {
  it("is a claimed name, trimmed and ASCII-lowercased, and never judged further", () => {
    expect(claimedIdentity(" Ben ")).toEqual({ kind: "claimed", name: "ben" });
    expect(claimedIdentity(" Ben  Castle "), "each run of spaces is one, as the key grammar's typed_form").toEqual({
      kind: "claimed",
      name: "ben castle",
    });
    expect(claimedIdentity("a+b@x.test")).toEqual({ kind: "claimed", name: "a+b@x.test" });
    expect(claimedIdentity(".."), "the key grammar refuses it, in the objects").toEqual({ kind: "claimed", name: ".." });
  });
  it("is no one for a value a header cannot carry, judged RAW", () => {
    // A Kelvin sign, a no-break space, a carriage return, a NUL, an e-acute.
    for (const raw of ["\u212Aevin", "\u00A0ben", "ben\r", "ben\u0000", "ren\u00E9"]) {
      expect(claimedIdentity(raw), JSON.stringify(raw)).toEqual({ kind: "none" });
    }
  });
  it("is no one with no claim, or a blank one", () => {
    for (const raw of [null, "", "   ", "\t"]) expect(claimedIdentity(raw), JSON.stringify(raw)).toEqual({ kind: "none" });
  });
});

// The one reader of the value an identity is forwarded as, and names its
// archive object by. Mutant: a claim forwarded as nothing (the open door's
// caller lost between the Worker and the object).
describe("callerName (M6.5c2)", () => {
  it("is the person's or the dev door's address, the claimed name, or nothing", () => {
    expect(callerName({ kind: "person", email: "red@t.test" })).toBe("red@t.test");
    expect(callerName({ kind: "dev", email: "probe@deploy.test" })).toBe("probe@deploy.test");
    expect(callerName({ kind: "claimed", name: "ben" })).toBe("ben");
    expect(callerName({ kind: "none" })).toBeUndefined();
  });
});
