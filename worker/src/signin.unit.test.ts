import { afterEach, beforeAll, beforeEach, describe, expect, it, vi, type MockInstance } from "vitest";
import { b64url, ecSigner, signer, type Signer } from "../test/tokens";
import type { ShellConfig } from "./config";
import { keySourceAt, type JwkSet, type KeySource } from "./identity";
import {
  authorizeUrl,
  discovery,
  exchange,
  leaveUrl,
  verifyIdToken,
  WORKOS_AUTHENTICATE,
  WORKOS_AUTHORIZE,
  WORKOS_JWKS,
  WORKOS_LOGOUT,
  type Discovery,
} from "./signin";

const NOW = 1_789_000_000;
const ISS = "https://issuer.test";
const AUD = "client_1";
const NONCE = "n1";

type Signin = NonNullable<ShellConfig["signin"]>;
/** The `workos` arm's configuration: no issuer, the client id, the API key. */
const SIGNIN: Signin = { issuer: null, clientId: "c", clientSecret: "k" };
/** The `oidc` arm's, with a client secret, and with none (the PKCE flow). */
const OIDC_SECRET: Signin = { issuer: ISS, clientId: AUD, clientSecret: "sec" };
const OIDC_PKCE: Signin = { issuer: ISS, clientId: AUD, clientSecret: null };

const DOC: Discovery = {
  issuer: ISS,
  authorization_endpoint: "https://issuer.test/authorize",
  token_endpoint: "https://issuer.test/token",
  jwks_uri: "https://issuer.test/jwks",
  end_session_endpoint: "https://issuer.test/logout",
};

/** What `verifyIdToken` is told to expect. */
const C = { iss: ISS, aud: AUD, nonce: NONCE, nowSeconds: NOW };

let rsa: Signer;
let ec: Signer;
/** A key pair in no set: what a forger holds. */
let forger: Signer;
/** The issuer's published set: the RS256 key and the ES256 key. */
let SET: JwkSet;
/** A key source answering `SET`, cached or refreshed. */
let keys: KeySource;

beforeAll(async () => {
  rsa = await signer("k1");
  ec = await ecSigner("e1");
  forger = await signer("k-forger");
  SET = { keys: [rsa.jwk, ec.jwk] };
  keys = async () => SET;
});

/** An ID token's claims as an issuer writes them, with `over` laid on top. */
function claims(over: Record<string, unknown> = {}): Record<string, unknown> {
  return { iss: ISS, aud: AUD, exp: NOW + 60, nonce: NONCE, email: "Red@T.test", email_verified: true, sid: "s1", ...over };
}

/** `claims()` with `key` removed. */
function without(key: string): Record<string, unknown> {
  const c = claims();
  delete c[key];
  return c;
}

/** `token` with its signature's first character changed. */
function flipped(token: string): string {
  const [h, p, s] = token.split(".") as [string, string, string];
  return `${h}.${p}.${s.startsWith("A") ? "B" : "A"}${s.slice(1)}`;
}

/** One call the recording fetcher received: the body parsed as the arm sent it, JSON or a form. */
interface Call { url: string; method: string; type: string | null; body: unknown }
type Recording = typeof fetch & { calls: Call[] };

/** A test fetcher answering `answer()` to every call, and recording each. */
function answering(answer: () => Response | Promise<Response>): Recording {
  const calls: Call[] = [];
  const fetcher = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const type = new Headers(init?.headers).get("content-type");
    const raw = typeof init?.body === "string" ? init.body : null;
    const body =
      raw === null ? null : type === "application/json" ? (JSON.parse(raw) as unknown) : Object.fromEntries(new URLSearchParams(raw));
    calls.push({ url: String(input), method: init?.method ?? "GET", type, body });
    return answer();
  };
  return Object.assign(fetcher as typeof fetch, { calls });
}

/** The brief's `recording(status, body)`: `Response.json(body, { status })` for every call. */
function recording(status: number, body: unknown): Recording {
  return answering(() => Response.json(body, { status }));
}

/** A key source answering `set` for `false` and `refreshed` for `true`, recording what it was asked. */
function source(set: JwkSet, refreshed: JwkSet = set): { keys: KeySource; asked: boolean[] } {
  const asked: boolean[] = [];
  return {
    asked,
    keys: async (refresh) => {
      asked.push(refresh);
      return refresh ? refreshed : set;
    },
  };
}

// Every line the module logs is caught here, so a failure cell prints nothing and asserts what it logged.
let spy: MockInstance<typeof console.log>;
beforeEach(() => {
  spy = vi.spyOn(console, "log").mockImplementation(() => {});
});
afterEach(() => {
  spy.mockRestore();
});
/** The lines logged so far in this cell, parsed (the shell logs one JSON object per line). */
function lines(): Array<Record<string, unknown>> {
  return spy.mock.calls.map(([line]) => JSON.parse(String(line)) as Record<string, unknown>);
}
/** Each logged line as `[event, source, step]`. */
function steps(): unknown[][] {
  return lines().map((l) => [l.event, l.source, l.step]);
}

describe("verifyIdToken", () => {
  it("accepts RS256 and ES256 tokens with iss, aud, exp, nonce and a verified email, lowercased in ASCII", async () => {
    expect(await verifyIdToken(await rsa.sign(claims()), SET, C)).toEqual({ address: "red@t.test", sid: "s1" });
    expect(await verifyIdToken(await ec.sign(claims()), SET, C)).toEqual({ address: "red@t.test", sid: "s1" });
    // Trimmed as the layer reads a caller, after `forwardable` judged it raw.
    expect(await verifyIdToken(await ec.sign(claims({ email: " Red@T.Test " })), SET, C)).toEqual({
      address: "red@t.test",
      sid: "s1",
    });
  });

  // Mutants: `email_verified` absent accepted (the plan's battery); any claim check dropped; `forwardable` skipped
  // (the Kelvin sign survives `asciiLower` and `isAddress`; the no-break space is stripped by `trim()` into
  // another user's address).
  it("refuses: wrong iss, aud missing the client, exp past, nonce mismatch, email_verified false or absent, a Kelvin-sign email, alg none, kid unknown, bad signature", async () => {
    const good = await rsa.sign(claims());
    const [h, p] = (await rsa.sign(claims(), { alg: "none", kid: "k1", typ: "JWT" })).split(".") as [string, string];
    const kidless: JwkSet = { keys: [{ ...rsa.jwk, kid: undefined }] };
    const cases: Array<[string, string, JwkSet]> = [
      ["wrong iss", await rsa.sign(claims({ iss: "https://other.test" })), SET],
      ["aud missing the client", await rsa.sign(claims({ aud: "client_2" })), SET],
      ["exp past", await rsa.sign(claims({ exp: NOW - 1 })), SET],
      ["exp absent", await rsa.sign(without("exp")), SET],
      ["nonce mismatch", await rsa.sign(claims({ nonce: "n2" })), SET],
      ["nonce absent", await rsa.sign(without("nonce")), SET],
      ["email_verified false", await rsa.sign(claims({ email_verified: false })), SET],
      ["email_verified absent", await rsa.sign(without("email_verified")), SET],
      ["email_verified the string true", await rsa.sign(claims({ email_verified: "true" })), SET],
      ["a Kelvin-sign email", await rsa.sign(claims({ email: "\u212Aevin@x.test" })), SET],
      ["a no-break space before the email", await rsa.sign(claims({ email: "\u00A0red@t.test" })), SET],
      ["alg none, signature kept", `${h}.${p}.${good.split(".")[2]}`, SET],
      ["alg none, signature empty", `${h}.${p}.`, SET],
      ["alg HS256", await rsa.sign(claims(), { alg: "HS256", kid: "k1", typ: "JWT" }), SET],
      ["kid unknown", await forger.sign(claims()), SET],
      ["kid absent, against a key that names none", await rsa.sign(claims(), { alg: "RS256", typ: "JWT" }), kidless],
      ["bad signature: the forger signs under k1", await forger.sign(claims(), { alg: "RS256", kid: "k1", typ: "JWT" }), SET],
      ["bad signature: one character changed", flipped(good), SET],
    ];
    for (const [label, token, set] of cases) {
      expect(await verifyIdToken(token, set, C), label).toBeNull();
    }
  });

  it("aud: an array containing the client id is accepted; an array without it, or a number, is refused", async () => {
    expect(await verifyIdToken(await rsa.sign(claims({ aud: ["other", AUD] })), SET, C)).toEqual({ address: "red@t.test", sid: "s1" });
    expect(await verifyIdToken(await rsa.sign(claims({ aud: ["other", "client_2"] })), SET, C)).toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ aud: [] })), SET, C)).toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ aud: 1 })), SET, C)).toBeNull();
  });

  // Mutants: no skew; `<` for `<=`; a wider skew; `>=` for `>` on `exp`.
  it("nbf: 60 s of skew is allowed and 61 s is not; exp is refused at now and accepted a second after", async () => {
    expect(await verifyIdToken(await rsa.sign(claims({ nbf: NOW + 60 })), SET, C), "nbf now + 60").not.toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ nbf: NOW + 61 })), SET, C), "nbf now + 61").toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ nbf: "later" })), SET, C), "nbf not a number is not read").not.toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ exp: NOW })), SET, C), "exp now").toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ exp: NOW + 1 })), SET, C), "exp now + 1").not.toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims()), SET, { ...C, nowSeconds: Number.NaN }), "a NaN clock").toBeNull();
  });

  // The key the `kid` names must fit the algorithm the header names; a key that does not is refused BEFORE WebCrypto
  // sees it (the spy), and whatever WebCrypto throws on is refused too, never thrown into the caller (`resolves`: a
  // rejection reads red). Mutants: the key-type or curve check dropped, the declared-alg check dropped (each masked
  // by the `try` but for the spy); both the check and the `try` dropped.
  it("a key with the right kid but the wrong algorithm family, curve or declared alg refuses rather than throws", async () => {
    // Keys as many issuers publish them, with no `alg` of their own, so only the key type or curve can refuse them.
    const bare = (jwk: JsonWebKey & { kid: string }) => ({ ...jwk, alg: undefined });
    const refusedEarly: Array<[string, string, JwkSet]> = [
      ["an RSA key under ES256", await rsa.sign(claims(), { alg: "ES256", kid: "k1", typ: "JWT" }), { keys: [bare(rsa.jwk)] }],
      ["an EC key under RS256", await ec.sign(claims(), { alg: "RS256", kid: "e1", typ: "JWT" }), { keys: [bare(ec.jwk)] }],
      ["an EC key on another curve", await ec.sign(claims()), { keys: [{ ...ec.jwk, crv: "P-384" }] }],
      ["an RSA key declared RS384", await rsa.sign(claims()), { keys: [{ ...rsa.jwk, alg: "RS384" }] }],
    ];
    const importKey = vi.spyOn(crypto.subtle, "importKey");
    try {
      for (const [label, token, set] of refusedEarly) {
        await expect(verifyIdToken(token, set, C), label).resolves.toBeNull();
      }
      expect(importKey, "refused before WebCrypto sees the key").not.toHaveBeenCalled();
    } finally {
      importKey.mockRestore();
    }
    const broken: JwkSet = { keys: [{ kty: "RSA", kid: "k1", n: "!!", e: "AQAB" }] };
    await expect(verifyIdToken(await rsa.sign(claims()), broken, C), "a key the platform cannot import").resolves.toBeNull();
    // And the right key, declaring its alg or not, still reads.
    await expect(verifyIdToken(await rsa.sign(claims()), { keys: [{ ...rsa.jwk, alg: "RS256" }] }, C)).resolves.not.toBeNull();
    await expect(verifyIdToken(await ec.sign(claims()), { keys: [bare(ec.jwk)] }, C)).resolves.not.toBeNull();
  });

  it("a token that is not three base64url JSON parts answers null, never throws", async () => {
    const good = await rsa.sign(claims());
    const [h, p, s] = good.split(".") as [string, string, string];
    const nullHead = b64url("null");
    const arrayBody = b64url("[]");
    for (const token of ["", "a.b", `${good}.x`, `x.${p}.${s}`, `${nullHead}.${p}.${s}`, `${h}.${arrayBody}.${s}`, `${h}.${p}.!!!`]) {
      await expect(verifyIdToken(token, SET, C), token).resolves.toBeNull();
    }
  });

  it("sid absent, empty or not a string is null; an email absent, blank or not a string refuses", async () => {
    expect(await verifyIdToken(await rsa.sign(without("sid")), SET, C)).toEqual({ address: "red@t.test", sid: null });
    expect(await verifyIdToken(await rsa.sign(claims({ sid: "" })), SET, C)).toEqual({ address: "red@t.test", sid: null });
    expect(await verifyIdToken(await rsa.sign(claims({ sid: 7 })), SET, C)).toEqual({ address: "red@t.test", sid: null });
    expect(await verifyIdToken(await rsa.sign(without("email")), SET, C)).toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ email: "   " })), SET, C)).toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ email: ["red@t.test"] })), SET, C)).toBeNull();
    expect(await verifyIdToken(await rsa.sign(claims({ email: "no-at-sign" })), SET, C)).toBeNull();
  });

  // A caller that lost its nonce must not match a token that carries none worth the name.
  it("an empty expected nonce refuses, even beside a token carrying an empty nonce", async () => {
    expect(await verifyIdToken(await rsa.sign(claims({ nonce: "" })), SET, { ...C, nonce: "" })).toBeNull();
  });
});

describe("authorizeUrl", () => {
  it("workos: the five parameters and provider=authkit, no nonce, no code_challenge", () => {
    const u = new URL(authorizeUrl("workos", { issuer: null, clientId: "c", clientSecret: "k" }, { redirectUri: "https://h/enter", state: "st", nonce: "no", challenge: null }, null));
    expect(u.origin + u.pathname).toBe(WORKOS_AUTHORIZE);
    expect(Object.fromEntries(u.searchParams)).toEqual({ client_id: "c", redirect_uri: "https://h/enter", response_type: "code", provider: "authkit", state: "st" });
    // A challenge or a discovery document handed to the workos arm changes nothing.
    const v = new URL(authorizeUrl("workos", SIGNIN, { redirectUri: "https://h/enter", state: "st", nonce: "no", challenge: "ch" }, DOC));
    expect(v.toString()).toBe(u.toString());
  });

  it("oidc: the discovery document's endpoint with scope, nonce, and code_challenge S256 only when given", () => {
    const p = { redirectUri: "https://h/enter", state: "st", nonce: "no" };
    const pkce = new URL(authorizeUrl("oidc", OIDC_PKCE, { ...p, challenge: "ch" }, DOC));
    expect(pkce.origin + pkce.pathname).toBe(DOC.authorization_endpoint);
    expect(Object.fromEntries(pkce.searchParams)).toEqual({
      client_id: AUD,
      redirect_uri: "https://h/enter",
      response_type: "code",
      scope: "openid email",
      state: "st",
      nonce: "no",
      code_challenge: "ch",
      code_challenge_method: "S256",
    });
    const secret = new URL(authorizeUrl("oidc", OIDC_SECRET, { ...p, challenge: null }, DOC));
    expect(Object.fromEntries(secret.searchParams)).toEqual({
      client_id: AUD,
      redirect_uri: "https://h/enter",
      response_type: "code",
      scope: "openid email",
      state: "st",
      nonce: "no",
    });
  });

  it("oidc: an authorization endpoint that already carries a query keeps it", () => {
    const doc = { ...DOC, authorization_endpoint: "https://issuer.test/authorize?tenant=t1" };
    const u = new URL(authorizeUrl("oidc", OIDC_SECRET, { redirectUri: "https://h/enter", state: "st", nonce: "no", challenge: null }, doc));
    expect(u.origin + u.pathname).toBe("https://issuer.test/authorize");
    expect(u.searchParams.get("tenant")).toBe("t1");
    expect(u.searchParams.get("state")).toBe("st");
  });

  it("oidc with no discovery document throws: the caller always has one under oidc", () => {
    expect(() => authorizeUrl("oidc", OIDC_SECRET, { redirectUri: "https://h/enter", state: "st", nonce: "no", challenge: null }, null)).toThrow(
      /discovery document/,
    );
  });
});

describe("exchange", () => {
  const WORKOS_P = { code: "c1", redirectUri: "https://h/enter", verifier: "", nonce: "", ip: "1.2.3.4", userAgent: "ua" };
  const OIDC_P = { code: "c1", redirectUri: "https://h/enter", verifier: "v1", nonce: NONCE, ip: null, userAgent: null };
  const VERIFIED = { email: "Red@T.test", email_verified: true };

  it("workos: POSTs the JSON body with client_secret and returns the user's lowercased address and the access token's sid", async () => {
    const fetcher = recording(200, { user: { email: "Red@T.test", email_verified: true }, access_token: await rsa.sign({ sid: "session_01", exp: NOW + 60 }), refresh_token: "r" });
    expect(await exchange("workos", SIGNIN, { code: "c1", redirectUri: "https://h/enter", verifier: "", nonce: "", ip: "1.2.3.4", userAgent: "ua" }, { fetcher, keys, discovery: null, nowSeconds: NOW })).toEqual({ address: "red@t.test", sid: "session_01" });
    expect(fetcher.calls[0]).toMatchObject({ url: WORKOS_AUTHENTICATE, method: "POST", body: { client_id: "c", client_secret: "k", grant_type: "authorization_code", code: "c1", ip_address: "1.2.3.4", user_agent: "ua" } });
    expect(fetcher.calls[0]?.type).toBe("application/json");
    expect(fetcher.calls).toHaveLength(1);
    expect(lines()).toEqual([]);
  });

  // Mutants: the status not read (each failing answer below carries a success-shaped body); `email_verified`
  // read loosely; the user's email taken without a check.
  it("workos: email_verified false, a non-200, a body that is not JSON, or a user with no email each answer null", async () => {
    const token = await rsa.sign({ sid: "session_01" });
    const net = (fetcher: typeof fetch) => ({ fetcher, keys, discovery: null, nowSeconds: NOW });
    const answers: Array<[string, Recording, string]> = [
      ["email_verified false", recording(200, { user: { email: "red@t.test", email_verified: false }, access_token: token }), "user"],
      ["email_verified absent", recording(200, { user: { email: "red@t.test" }, access_token: token }), "user"],
      ["email_verified the string true", recording(200, { user: { email: "red@t.test", email_verified: "true" }, access_token: token }), "user"],
      ["a 400", recording(400, { user: VERIFIED, access_token: token }), "exchange"],
      ["a 500", recording(500, { user: VERIFIED, access_token: token }), "exchange"],
      ["a body that is not JSON", answering(() => new Response("<html>red@t.test</html>", { status: 200 })), "exchange"],
      ["a JSON body that is no object", recording(200, [VERIFIED]), "exchange"],
      ["no user", recording(200, { access_token: token }), "user"],
      ["a user with no email", recording(200, { user: { email_verified: true }, access_token: token }), "user"],
      ["a user whose email is no address", recording(200, { user: { email: "red", email_verified: true }, access_token: token }), "user"],
      [
        "a network failure",
        answering(() => {
          throw new TypeError("network down");
        }),
        "exchange",
      ],
    ];
    for (const [label, fetcher, step] of answers) {
      spy.mockClear();
      expect(await exchange("workos", SIGNIN, WORKOS_P, net(fetcher)), label).toBeNull();
      expect(steps(), label).toEqual([["signin-failed", "workos", step]]);
    }
  });

  // The Kelvin case (the plan's battery: `exchange("workos")` reading `email` without `forwardable`). Judged RAW,
  // `\u212Aevin@x.test` is no caller; lowercased first by JavaScript it would be `kevin@x.test`, another user, and
  // `asciiLower` alone leaves a Kelvin sign that `isAddress` accepts. A no-break space would be trimmed away.
  it("workos: a user.email carrying a non-ASCII character answers null (judged raw by forwardable)", async () => {
    for (const email of ["\u212Aevin@x.test", "\u00A0red@t.test", "r\u00E9d@t.test"]) {
      spy.mockClear();
      const fetcher = recording(200, { user: { email, email_verified: true }, access_token: await rsa.sign({ sid: "s" }) });
      expect(await exchange("workos", SIGNIN, WORKOS_P, { fetcher, keys, discovery: null, nowSeconds: NOW }), email).toBeNull();
      expect(steps(), email).toEqual([["signin-failed", "workos", "user"]]);
    }
  });

  it("workos: an ip or a user agent that is null leaves its key out of the body, and a discovery document is ignored", async () => {
    const fetcher = recording(200, { user: VERIFIED, access_token: await rsa.sign({ sid: "s" }) });
    const net = { fetcher, keys, discovery: DOC, nowSeconds: NOW };
    expect(await exchange("workos", SIGNIN, { ...WORKOS_P, ip: null, userAgent: null }, net)).toEqual({ address: "red@t.test", sid: "s" });
    expect(fetcher.calls).toHaveLength(1);
    expect(fetcher.calls[0]?.url).toBe(WORKOS_AUTHENTICATE);
    expect(fetcher.calls[0]?.body).toEqual({ client_id: "c", client_secret: "k", grant_type: "authorization_code", code: "c1" });
    await exchange("workos", SIGNIN, { ...WORKOS_P, ip: "1.2.3.4", userAgent: null }, net);
    expect(fetcher.calls[1]?.body).toEqual({ client_id: "c", client_secret: "k", grant_type: "authorization_code", code: "c1", ip_address: "1.2.3.4" });
  });

  // Belt and braces: the answer is already authenticated by TLS and the API key, so a token that fails its own
  // check costs the sign-out's sid, never the sign-in.
  it("workos: an access token whose signature fails the JWKS still answers the address, with sid null", async () => {
    const good = await rsa.sign({ sid: "session_01" });
    const down: KeySource = async () => {
      throw new Error("the key set at https://api.workos.com/sso/jwks/c answered 503");
    };
    const tokens: Array<[string, unknown, KeySource]> = [
      ["signed by a key in no set", await forger.sign({ sid: "session_01" }), keys],
      ["signed by a forger under the set's kid", await forger.sign({ sid: "session_01" }, { alg: "RS256", kid: "k1", typ: "JWT" }), keys],
      ["one signature character changed", flipped(good), keys],
      ["the signature stripped", `${good.split(".").slice(0, 2).join(".")}.`, keys],
      ["opaque", "opaque-token", keys],
      ["absent", undefined, keys],
      ["the key source rejects", good, down],
    ];
    for (const [label, token, source] of tokens) {
      spy.mockClear();
      const fetcher = recording(200, { user: VERIFIED, access_token: token });
      expect(await exchange("workos", SIGNIN, WORKOS_P, { fetcher, keys: source, discovery: null, nowSeconds: NOW }), label).toEqual({
        address: "red@t.test",
        sid: null,
      });
      expect(steps(), label).toEqual([["signin-sid-unverified", "workos", undefined]]);
    }
  });

  // `identify`'s rule for Access, here: a kid the cached set lacks asks the source to refresh once.
  it("workos: an access token whose kid the cached set lacks refreshes the keys once, then reads its sid", async () => {
    const fetcher = recording(200, { user: VERIFIED, access_token: await rsa.sign({ sid: "session_01" }) });
    const rotated = source({ keys: [] }, SET);
    expect(await exchange("workos", SIGNIN, WORKOS_P, { fetcher, keys: rotated.keys, discovery: null, nowSeconds: NOW })).toEqual({
      address: "red@t.test",
      sid: "session_01",
    });
    expect(rotated.asked).toEqual([false, true]);
    const cached = source(SET);
    await exchange("workos", SIGNIN, WORKOS_P, { fetcher, keys: cached.keys, discovery: null, nowSeconds: NOW });
    expect(cached.asked, "a kid the cache holds asks nothing more").toEqual([false]);
  });

  // Mutant: the catch-all dropped. Nothing in the arms throws by design; a fetcher that breaks its contract is the
  // one way left to reach it.
  it("an exception nothing foresaw is still null, logged as unexpected, never thrown into the caller", async () => {
    const broken = answering(() => undefined as unknown as Response);
    await expect(exchange("workos", SIGNIN, WORKOS_P, { fetcher: broken, keys, discovery: null, nowSeconds: NOW })).resolves.toBeNull();
    await expect(exchange("oidc", OIDC_SECRET, OIDC_P, { fetcher: broken, keys, discovery: DOC, nowSeconds: NOW })).resolves.toBeNull();
    expect(steps()).toEqual([
      ["signin-failed", "workos", "unexpected"],
      ["signin-failed", "oidc", "unexpected"],
    ]);
  });

  it("workos: with no API key nothing is sent", async () => {
    const fetcher = recording(200, { user: VERIFIED });
    expect(await exchange("workos", { ...SIGNIN, clientSecret: null }, WORKOS_P, { fetcher, keys, discovery: null, nowSeconds: NOW })).toBeNull();
    expect(fetcher.calls).toHaveLength(0);
    expect(steps()).toEqual([["signin-failed", "workos", "config"]]);
  });

  // Mutant: the parse error's message logged (V8 quotes the body in it), or the body itself.
  it("logs the source, the step and a sentence: never a body, a token, a secret or an address", async () => {
    const token = await rsa.sign({ sid: "session_01", email: "red@t.test" });
    const secretful: Signin = { ...SIGNIN, clientSecret: "sk_test_SECRET" };
    const failing: Recording[] = [
      answering(() => new Response(`red@t.test ${token}`, { status: 200 })),
      recording(401, { error: "invalid_grant", user: VERIFIED, access_token: token }),
      recording(200, { user: { email: "red@t.test", email_verified: false }, access_token: token }),
      recording(200, { user: { email: "\u212Aevin@x.test", email_verified: true }, access_token: token }),
      recording(200, { user: VERIFIED, access_token: await forger.sign({ sid: "session_01" }) }),
    ];
    for (const fetcher of failing) {
      await exchange("workos", secretful, WORKOS_P, { fetcher, keys, discovery: null, nowSeconds: NOW });
    }
    const oidcFailing: Recording[] = [
      answering(() => new Response(`red@t.test ${token}`, { status: 200 })),
      recording(200, { id_token: await rsa.sign(claims({ nonce: "other" })) }),
    ];
    for (const fetcher of oidcFailing) {
      await exchange("oidc", { ...OIDC_SECRET, clientSecret: "sk_test_SECRET" }, OIDC_P, { fetcher, keys, discovery: DOC, nowSeconds: NOW });
    }
    const logged = JSON.stringify(spy.mock.calls);
    expect(lines()).toHaveLength(7);
    for (const line of lines()) expect(Object.keys(line).sort()).toEqual(line.event === "signin-failed" ? ["error", "event", "source", "step"] : ["error", "event", "source"]);
    for (const forbidden of ["red@t.test", "Red@T.test", "evin@x.test", token, token.split(".")[1]!, "sk_test_SECRET", "c1", "session_01", "invalid_grant"]) {
      expect(logged, forbidden).not.toContain(forbidden);
    }
  });

  it("oidc: POSTs form-encoded grant_type=authorization_code with client_secret_post, or with code_verifier and no secret, and verifies the id_token", async () => {
    const answer = { access_token: "at", token_type: "Bearer", id_token: await ec.sign(claims()) };
    const net = (fetcher: typeof fetch) => ({ fetcher, keys, discovery: DOC, nowSeconds: NOW });

    const secret = recording(200, answer);
    expect(await exchange("oidc", OIDC_SECRET, OIDC_P, net(secret))).toEqual({ address: "red@t.test", sid: "s1" });
    expect(secret.calls).toHaveLength(1);
    expect(secret.calls[0]).toMatchObject({ url: DOC.token_endpoint, method: "POST", type: "application/x-www-form-urlencoded" });
    expect(secret.calls[0]?.body).toEqual({ grant_type: "authorization_code", code: "c1", redirect_uri: "https://h/enter", client_id: AUD, client_secret: "sec" });

    const pkce = recording(200, answer);
    expect(await exchange("oidc", OIDC_PKCE, OIDC_P, net(pkce))).toEqual({ address: "red@t.test", sid: "s1" });
    expect(pkce.calls[0]?.body).toEqual({ grant_type: "authorization_code", code: "c1", redirect_uri: "https://h/enter", client_id: AUD, code_verifier: "v1" });
    expect(lines()).toEqual([]);

    // The id_token is verified against the document's issuer, the client id and the login's nonce.
    const refused: Array<[string, string, Discovery]> = [
      ["the login's nonce differs", await ec.sign(claims({ nonce: "n2" })), DOC],
      ["another client's token", await ec.sign(claims({ aud: "client_2" })), DOC],
      ["the document names another issuer", await ec.sign(claims()), { ...DOC, issuer: "https://other.test" }],
      ["a forger's token", await forger.sign(claims()), DOC],
      ["email_verified absent", await ec.sign(without("email_verified")), DOC],
    ];
    for (const [label, idToken, doc] of refused) {
      spy.mockClear();
      const fetcher = recording(200, { ...answer, id_token: idToken });
      expect(await exchange("oidc", OIDC_SECRET, OIDC_P, { fetcher, keys, discovery: doc, nowSeconds: NOW }), label).toBeNull();
      expect(steps(), label).toEqual([["signin-failed", "oidc", "id-token"]]);
    }
  });

  it("oidc: a token response with no id_token answers null", async () => {
    const good = await ec.sign(claims());
    const net = (fetcher: typeof fetch) => ({ fetcher, keys, discovery: DOC, nowSeconds: NOW });
    const answers: Array<[string, Recording, string]> = [
      ["no id_token", recording(200, { access_token: "at", token_type: "Bearer" }), "id-token"],
      ["an id_token that is no string", recording(200, { id_token: { value: good } }), "id-token"],
      ["a 400 with a success-shaped body", recording(400, { id_token: good }), "exchange"],
      ["a body that is not JSON", answering(() => new Response("nope", { status: 200 })), "exchange"],
    ];
    for (const [label, fetcher, step] of answers) {
      spy.mockClear();
      expect(await exchange("oidc", OIDC_SECRET, OIDC_P, net(fetcher)), label).toBeNull();
      expect(steps(), label).toEqual([["signin-failed", "oidc", step]]);
    }
    spy.mockClear();
    const unasked = recording(200, { id_token: good });
    expect(await exchange("oidc", OIDC_SECRET, OIDC_P, { fetcher: unasked, keys, discovery: null, nowSeconds: NOW }), "no document").toBeNull();
    expect(unasked.calls, "no document, no exchange").toHaveLength(0);
    expect(steps()).toEqual([["signin-failed", "oidc", "discovery"]]);
  });

  it("oidc: an id_token whose kid the cached set lacks refreshes the keys once; a key source that rejects answers null", async () => {
    const fetcher = recording(200, { id_token: await ec.sign(claims()) });
    const rotated = source({ keys: [rsa.jwk] }, SET);
    expect(await exchange("oidc", OIDC_SECRET, OIDC_P, { fetcher, keys: rotated.keys, discovery: DOC, nowSeconds: NOW })).toEqual({
      address: "red@t.test",
      sid: "s1",
    });
    expect(rotated.asked).toEqual([false, true]);
    const down: KeySource = async () => {
      throw new Error("the key set at https://issuer.test/jwks answered 503");
    };
    expect(await exchange("oidc", OIDC_SECRET, OIDC_P, { fetcher, keys: down, discovery: DOC, nowSeconds: NOW })).toBeNull();
    expect(steps()).toEqual([["signin-failed", "oidc", "keys"]]);
  });
});

describe("discovery", () => {
  /** A document as an issuer serves it, for `issuer`. */
  const served = (issuer: string, over: Record<string, unknown> = {}) => ({
    issuer,
    authorization_endpoint: `${issuer}/authorize`,
    token_endpoint: `${issuer}/token`,
    jwks_uri: `${issuer}/jwks`,
    end_session_endpoint: `${issuer}/logout`,
    scopes_supported: ["openid", "email"],
    ...over,
  });

  // Each cell names its own issuer: the cache is the isolate's, keyed by issuer, so no cell reads another's entry.
  it("fetches /.well-known/openid-configuration once per isolate and rejects every joiner on a non-200", async () => {
    const one = "https://one.discovery.test";
    const ok = recording(200, served(one));
    const joined = await Promise.all([discovery(one, ok), discovery(one, ok)]);
    expect(await discovery(one, ok)).toEqual({ ...joined[0] });
    expect(joined[0]).toEqual({
      issuer: one,
      authorization_endpoint: `${one}/authorize`,
      token_endpoint: `${one}/token`,
      jwks_uri: `${one}/jwks`,
      end_session_endpoint: `${one}/logout`,
    });
    expect(ok.calls).toEqual([{ url: `${one}/.well-known/openid-configuration`, method: "GET", type: null, body: null }]);

    const slash = "https://slash.discovery.test/";
    const trailing = recording(200, served("https://slash.discovery.test"));
    await discovery(slash, trailing);
    expect(trailing.calls[0]?.url, "one slash, not two").toBe("https://slash.discovery.test/.well-known/openid-configuration");

    const two = "https://two.discovery.test";
    let status = 503;
    const flaky = answering(() => Response.json(served(two), { status }));
    const failed = await Promise.allSettled([discovery(two, flaky), discovery(two, flaky)]);
    expect(failed.map((r) => r.status)).toEqual(["rejected", "rejected"]);
    expect(String((failed[0] as PromiseRejectedResult).reason)).toMatch(/503/);
    expect(flaky.calls, "the two joined one fetch").toHaveLength(1);
    status = 200;
    expect((await discovery(two, flaky)).issuer, "a failed fetch cached nothing").toBe(two);
    expect(flaky.calls).toHaveLength(2);
  });

  it("refuses a document that is not JSON, lacks an endpoint, or names one that is no URL; reads end_session_endpoint absent as null", async () => {
    const refused: Array<[string, Recording]> = [
      ["not JSON", answering(() => new Response("<html/>", { status: 200 }))],
      ["no token_endpoint", recording(200, served("https://a.discovery.test", { token_endpoint: undefined }))],
      ["no jwks_uri", recording(200, served("https://b.discovery.test", { jwks_uri: undefined }))],
      ["an authorization_endpoint that is no URL", recording(200, served("https://c.discovery.test", { authorization_endpoint: "authorize" }))],
      ["an issuer that is no string", recording(200, served("https://d.discovery.test", { issuer: 7 }))],
      ["an end_session_endpoint that is no string", recording(200, served("https://e.discovery.test", { end_session_endpoint: 7 }))],
      ["an array", recording(200, [])],
    ];
    let n = 0;
    for (const [label, fetcher] of refused) {
      await expect(discovery(`https://refused-${n++}.discovery.test`, fetcher), label).rejects.toThrow();
    }
    const quiet = recording(200, served("https://f.discovery.test", { end_session_endpoint: undefined }));
    expect((await discovery("https://f.discovery.test", quiet)).end_session_endpoint).toBeNull();
    const nulled = recording(200, served("https://g.discovery.test", { end_session_endpoint: null }));
    expect((await discovery("https://g.discovery.test", nulled)).end_session_endpoint).toBeNull();
  });
});

describe("leaveUrl", () => {
  const RETURN = "https://yokohama.test/?a=b&c=d#/lobby";

  it("workos with a sid: WorkOS's logout with the session id and return_to, each encoded", () => {
    expect(WORKOS_JWKS("client_1")).toBe("https://api.workos.com/sso/jwks/client_1");
    const u = new URL(leaveUrl("workos", "session_01&x=y", null, RETURN));
    expect(u.origin + u.pathname).toBe("https://api.workos.com/user_management/sessions/logout");
    expect(Object.fromEntries(u.searchParams)).toEqual({ session_id: "session_01&x=y", return_to: RETURN });
    expect(u.hash, "the return_to's fragment stays inside it").toBe("");
    expect(leaveUrl("workos", "s1", DOC, RETURN)).toBe(`${WORKOS_LOGOUT("s1")}&return_to=${encodeURIComponent(RETURN)}`);
  });

  it("workos with no sid lands on return_to itself", () => {
    expect(leaveUrl("workos", null, null, RETURN)).toBe(RETURN);
    expect(leaveUrl("workos", "", null, RETURN)).toBe(RETURN);
  });

  it("oidc: the end_session_endpoint with post_logout_redirect_uri and no id_token_hint, joined with & when it already has a query", () => {
    const u = new URL(leaveUrl("oidc", "s1", DOC, RETURN));
    expect(u.origin + u.pathname).toBe(DOC.end_session_endpoint);
    expect(Object.fromEntries(u.searchParams)).toEqual({ post_logout_redirect_uri: RETURN });
    expect(u.hash).toBe("");
    const queried = new URL(leaveUrl("oidc", null, { ...DOC, end_session_endpoint: "https://issuer.test/logout?x=1" }, RETURN));
    expect(Object.fromEntries(queried.searchParams)).toEqual({ x: "1", post_logout_redirect_uri: RETURN });
  });

  it("oidc with no end_session_endpoint, or no document, lands on return_to itself", () => {
    expect(leaveUrl("oidc", "s1", { ...DOC, end_session_endpoint: null }, RETURN)).toBe(RETURN);
    expect(leaveUrl("oidc", "s1", null, RETURN)).toBe(RETURN);
  });
});

describe("keySourceAt", () => {
  it("fetches the URL it is given, and its refusals name that URL", async () => {
    const url = "https://api.workos.com/sso/jwks/c";
    const ok = recording(200, { keys: [{ kid: "k1", kty: "RSA" }] });
    expect(await keySourceAt(url, ok, () => 0)(false)).toEqual({ keys: [{ kid: "k1", kty: "RSA" }] });
    expect(ok.calls.map((c) => c.url)).toEqual([url]);
    await expect(keySourceAt(url, recording(503, {}), () => 0)(false)).rejects.toThrow(`the key set at ${url} answered 503`);
    await expect(keySourceAt(url, recording(200, { keys: {} }), () => 0)(false)).rejects.toThrow(`the answer at ${url} holds no key set`);
  });
});
