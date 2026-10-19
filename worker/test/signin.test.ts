import { createExecutionContext } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { Archive } from "../src/archive";
import type { Env } from "../src/env";
import { House } from "../src/house";
import type { KeySource } from "../src/identity";
import { NOT_AN_OPERATOR } from "../src/admin";
import { CROSS_SITE, handle, NOT_SIGNED_IN, SESSION_CHALLENGE } from "../src/index";
import type { Limiter } from "../src/limits";
import { callerTag } from "../src/redact";
import { clearCookie, cookieValue, LOGIN_COOKIE, mintLogin, mintSession, readLogin, readSession, SESSION_COOKIE } from "../src/session";
import { WORKOS_AUTHENTICATE, WORKOS_AUTHORIZE, WORKOS_JWKS } from "../src/signin";
import { Table } from "../src/table";
import { CREATE_BODY, logsDuring, RED, seedPlaying } from "./helpers";
import { b64url, personClaims, signer, type Signer } from "./tokens";

// **The sign-in on `/enter`, the session cookie on every request, sign-out and the 401 header** (the auth cycle,
// Task 4). Every request goes through `handle`, as the Worker's own `fetch` does, under an env override that makes the
// SHELL a `workos` deployment; the objects read `wrangler.test.jsonc` (`access`), and nothing they do moves for it,
// since the shell forwards a session's address in `X-Yokohama-User` exactly as a person's. The "under access" rows
// pass the plain `env`.
//
// The clock (plan correction C-13): the shell reads the REAL clock, so every cookie here is minted at
// `Math.floor(Date.now() / 1000)`, never the unit files' fixed second.
//
// Two caches outlive a cell, by design: `discovery`'s per issuer (`signin.ts`) and the arms' key sources per JWKS URL
// (`index.ts`), each served by the FIRST caller's fetcher for the isolate's life. So every fetcher here answers every
// URL the same way, over ONE signer, and each `oidc` issuer below is named by no other cell's row that expects a
// different document.

const ORIGIN = "https://yokohama.test";
const FOREIGN = "https://evil.example";
const SECRET = "a-session-secret-of-at-least-32-bytes!";
const DAYS = 30;
const DAY = 86_400;
const JSON_TYPE = { "content-type": "application/json" };
/** The archive post that names no device and no safe game id: since Task 5 the anonymous archive's one remaining 401,
 *  the house refusing a malformed body, which carries no session challenge (ruling R-36). */
const NAMELESS_ARCHIVE = '{"record":"","table":""}';
const LOBBY = "/#/lobby";
/** The failed landing (ruling R-30): the query OUTSIDE the hash, so the client's `parseRoute`, which reads the hash
 *  alone, draws the lobby. */
const FAILED = "/?signin=failed#/lobby";

/** The shell as a `workos` deployment (the brief's override): Access's pair dropped, the arm's three set. */
const ENV: Env = {
  ...env,
  YOKOHAMA_IDENTITY_SOURCE: "workos",
  YOKOHAMA_OIDC_CLIENT_ID: "c",
  YOKOHAMA_OIDC_CLIENT_SECRET: "k",
  YOKOHAMA_SESSION_SECRET: SECRET,
  ACCESS_TEAM_DOMAIN: undefined,
  ACCESS_AUD: undefined,
};

const now = (): number => Math.floor(Date.now() / 1000);

/** The one key every issuer here signs with: the WorkOS access token's and every ID token's. */
let key: Signer;
beforeAll(async () => {
  key = await signer("issuer-k1");
});

/** Access's key source, which no row under a sign-in arm may ask: each ask is counted and refused. */
let accessAsked = 0;
const keys: KeySource = async () => {
  accessAsked += 1;
  throw new Error("the Access key source was asked under a sign-in arm");
};

interface Call { url: string; method: string; body: string }

/** What the recording fetcher answers for one issuer: the WorkOS user, or the `oidc` ID token's claims. */
interface Answers {
  /** The WorkOS `authenticate` answer's status; 200 when absent. */
  status?: number;
  /** The WorkOS `user`; RED, verified, when absent. */
  user?: Record<string, unknown>;
  /** An `oidc` ID token's claims beyond the ones read off the exchange (`nonce` comes from `nonceOf`). */
  claims?: Record<string, unknown>;
  /** The nonce the `oidc` ID token carries: the login cookie's, read by the cell. */
  nonce?: () => string;
}

/** An `oidc` issuer's discovery document, every endpoint under the issuer. */
const doc = (issuer: string) => ({
  issuer,
  authorization_endpoint: `${issuer}/authorize`,
  token_endpoint: `${issuer}/token`,
  jwks_uri: `${issuer}/jwks`,
  end_session_endpoint: `${issuer}/logout`,
});

/**
 * A fetcher that records every call (url, method, body) and answers as an issuer would: WorkOS's `authenticate`
 * with `answers.user` and an access token carrying `sid` `s1`, WorkOS's JWKS, and, for any `oidc` issuer under
 * `https://*.issuer.test`, its discovery document (a `down.` issuer answers 500), its token endpoint (an ID token
 * over `answers.claims`) and its JWKS.
 */
function issuer(answers: Answers = {}): typeof fetch & { calls: Call[] } {
  const calls: Call[] = [];
  const fetcher = async (input: RequestInfo | URL, init?: RequestInit): Promise<Response> => {
    const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
    calls.push({ url, method: init?.method ?? "GET", body: typeof init?.body === "string" ? init.body : "" });
    const at = now();
    if (url === WORKOS_AUTHENTICATE) {
      const accessToken = await key.sign({ sid: "s1", iss: "https://api.workos.com", exp: at + 300, iat: at });
      return Response.json({ user: answers.user ?? { email: RED, email_verified: true }, access_token: accessToken }, { status: answers.status ?? 200 });
    }
    if (url === WORKOS_JWKS("c")) return Response.json(key.set());
    const m = /^(https:\/\/[a-z-]+\.issuer\.test)(\/.*)$/.exec(url);
    if (m !== null) {
      const [, base, path] = m as unknown as [string, string, string];
      if (base.startsWith("https://down.")) return new Response("unavailable", { status: 500 });
      if (path === "/.well-known/openid-configuration") return Response.json(doc(base));
      if (path === "/jwks") return Response.json(key.set());
      if (path === "/token") {
        const claims = { iss: base, aud: "c", exp: at + 300, iat: at, email: RED, email_verified: true, nonce: answers.nonce?.(), ...answers.claims };
        return Response.json({ id_token: await key.sign(claims) });
      }
    }
    return new Response("no such endpoint", { status: 404 });
  };
  return Object.assign(fetcher as typeof fetch, { calls });
}

function get(path: string, headers: Record<string, string> = {}): Request {
  return new Request(ORIGIN + path, { headers });
}

function post(path: string, headers: Record<string, string> = {}, body: string | null = null): Request {
  return new Request(ORIGIN + path, { method: "POST", headers, body });
}

async function call(req: Request, e: Env = ENV, fetcher: typeof fetch = issuer()): Promise<Response> {
  return handle(req, e, createExecutionContext(), keys, fetcher);
}

/** A session cookie's VALUE for `address`, issued `ago` seconds before now. */
function session(address = RED, sid: string | null = "s1", ago = 0): Promise<string> {
  return mintSession({ address, sid, issued: now() - ago }, DAYS, SECRET);
}

/** A login cookie's VALUE, minted now unless `at` says otherwise. */
function login(l: { state?: string; nonce?: string; verifier?: string; returnTo?: string } = {}, at = now(), secret = SECRET): Promise<string> {
  return mintLogin({ state: l.state ?? "st", nonce: l.nonce ?? "n", verifier: l.verifier ?? "", returnTo: l.returnTo ?? "/#/t/sim-1" }, at, secret);
}

/** Which objects `f`'s requests reached: a spy on each class's `fetch`, on the PROTOTYPE (pool fact 1). */
async function reaching<T>(f: () => Promise<T>): Promise<{ result: T; reached: number; house: number }> {
  const spies = [Table, House, Archive].map((c) => vi.spyOn(c.prototype, "fetch"));
  try {
    const result = await f();
    return { result, reached: spies.reduce((n, s) => n + s.mock.calls.length, 0), house: spies[1]!.mock.calls.length };
  } finally {
    for (const s of spies) s.mockRestore();
  }
}

/** A fake rate-limit binding that records every key and lets everything through. */
function limiter(): Limiter & { asked: string[] } {
  const asked: string[] = [];
  return { asked, limit: async ({ key: k }) => { asked.push(k); return { success: true }; } };
}

/** The sign-in events `f` logged. */
async function signinLines<T>(f: () => Promise<T>): Promise<{ result: T; lines: Array<Record<string, unknown>> }> {
  const { result, lines } = await logsDuring(f);
  const events = new Set(["signin-refused", "signin-failed"]);
  return { result, lines: (lines as Array<Record<string, unknown>>).filter((l) => events.has(String(l.event))) };
}

describe("/enter starts the sign-in", () => {
  // Mutants: no login cookie set; the state in the cookie not the state in the redirect; return_to dropped; the
  // redirect URI another path; a verifier stored under `workos` (R-25: the PKCE decision has ONE site).
  it("GET /enter starts the sign-in: a 302 to WorkOS carrying state, a login cookie, and return_to kept", async () => {
    const fetcher = issuer();
    const res = await call(get("/enter?return_to=%2F%23%2Ft%2Fsim-1"), ENV, fetcher);
    expect(res.status).toBe(302);
    const to = new URL(res.headers.get("Location")!);
    expect(to.origin + to.pathname).toBe(WORKOS_AUTHORIZE);
    const state = to.searchParams.get("state")!;
    expect(state, "32 random bytes, base64url").toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(Object.fromEntries(to.searchParams)).toEqual({
      client_id: "c",
      redirect_uri: `${ORIGIN}/enter`,
      response_type: "code",
      provider: "authkit",
      state,
    });
    const line = res.headers.get("Set-Cookie")!;
    expect(line).toMatch(new RegExp(`^${LOGIN_COOKIE}=v1\\.[A-Za-z0-9_-]+\\.[A-Za-z0-9_-]{43}; Max-Age=600; Path=/; HttpOnly; Secure; SameSite=Lax$`));
    const kept = await readLogin(line, SECRET, now());
    expect(kept).toMatchObject({ state, returnTo: "/#/t/sim-1", verifier: "" });
    expect(kept!.nonce, "a nonce is drawn too").toMatch(/^[A-Za-z0-9_-]{43}$/);
    expect(kept!.nonce).not.toBe(state);
    expect(fetcher.calls, "a start asks the issuer nothing").toEqual([]);
    // Each start draws its own state.
    const again = new URL((await call(get("/enter"), ENV, fetcher)).headers.get("Location")!);
    expect(again.searchParams.get("state")).not.toBe(state);
    expect(accessAsked).toBe(0);
  });

  // The redirect URI is the request's own origin (`url.origin`), never a header a caller or a proxy may set; the
  // `oidc` issuer is this cell's own. Mutants: `Host` or `X-Forwarded-Host` read when present.
  it("a start carrying Host and X-Forwarded-Host headers still names this origin's /enter as the redirect URI", async () => {
    const forged = { Host: "evil.test", "X-Forwarded-Host": "evil.test" };
    const sent = get("/enter", forged);
    expect([sent.headers.get("Host"), sent.headers.get("X-Forwarded-Host")], "both headers ride the request").toEqual(["evil.test", "evil.test"]);
    const oidc: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://host.issuer.test" };
    for (const [label, e] of [["workos", ENV], ["oidc", oidc]] as const) {
      const res = await call(get("/enter", forged), e);
      expect([res.status, new URL(res.headers.get("Location")!).searchParams.get("redirect_uri")], label).toEqual([302, `${ORIGIN}/enter`]);
    }
  });

  it("a return_to that is another origin, a plain path, or an unsafe hash becomes /#/lobby", async () => {
    const landing = async (query: string) =>
      (await readLogin((await call(get(`/enter${query}`))).headers.get("Set-Cookie"), SECRET, now()))?.returnTo;
    for (const raw of ["https://evil.test/#/t/x", "/api/me", "/#/t/../x", "#/t/sim-1", "", "/#/t/x/y", "/#/t/", "//evil.test/#/lobby"]) {
      expect(await landing(`?return_to=${encodeURIComponent(raw)}`), JSON.stringify(raw)).toBe(LOBBY);
    }
    expect(await landing(""), "no return_to at all").toBe(LOBBY);
    for (const kept of ["/#/t/sim.1", "/#/new", "/#/new/local", "/#/lobby"]) {
      expect(await landing(`?return_to=${encodeURIComponent(kept)}`), kept).toBe(kept);
    }
  });

  // Ruling R-25: the challenge under `oidc` with no secret, none with one. The issuers are this cell's own (N-1).
  it("under oidc the start sends a PKCE challenge exactly when no client secret is set", async () => {
    const pkce: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://pkce.issuer.test", YOKOHAMA_OIDC_CLIENT_SECRET: undefined };
    const res = await call(get("/enter?return_to=%2F%23%2Flobby"), pkce);
    expect(res.status).toBe(302);
    const to = new URL(res.headers.get("Location")!);
    expect(to.origin + to.pathname).toBe("https://pkce.issuer.test/authorize");
    const kept = (await readLogin(res.headers.get("Set-Cookie"), SECRET, now()))!;
    expect(kept.verifier, "a verifier is drawn").toMatch(/^[A-Za-z0-9_-]{43}$/);
    const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(kept.verifier)));
    expect(Object.fromEntries(to.searchParams)).toEqual({
      client_id: "c",
      redirect_uri: `${ORIGIN}/enter`,
      response_type: "code",
      scope: "openid email",
      state: kept.state,
      nonce: kept.nonce,
      code_challenge: b64url(digest),
      code_challenge_method: "S256",
    });

    const secret: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://secret.issuer.test" };
    const confidential = await call(get("/enter"), secret);
    const params = new URL(confidential.headers.get("Location")!).searchParams;
    expect([params.has("code_challenge"), params.has("code_challenge_method")]).toEqual([false, false]);
    expect((await readLogin(confidential.headers.get("Set-Cookie"), SECRET, now()))!.verifier, "no verifier stored").toBe("");
  });

  // Fail closed on the redirect: anything that throws while the start is built — here the issuer's document cannot
  // be had — is the failed landing with the login cookie cleared, never the platform's 500.
  it("a start whose issuer document cannot be had lands on the failed lobby, with the login cookie cleared", async () => {
    const down: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://down.issuer.test", YOKOHAMA_OIDC_CLIENT_SECRET: undefined };
    const { result: res, lines } = await signinLines(() => call(get("/enter"), down));
    expect([res.status, res.headers.get("Location"), res.headers.getSetCookie()]).toEqual([302, FAILED, [clearCookie(LOGIN_COOKIE)]]);
    expect(lines.map((l) => [l.event, l.step])).toEqual([["signin-failed", "start"]]);
  });
});

describe("/enter completes the sign-in", () => {
  // Mutants: the exchange skipped or run twice; no session cookie; the login cookie left; the landing not the
  // cookie's return_to; the caller's IP or user agent not handed to the exchange.
  it("GET /enter?code&state completes it: the exchange runs once, the session cookie is set, the login cookie cleared, and the 302 lands on return_to", async () => {
    const fetcher = issuer();
    const res = await call(
      get("/enter?code=the-code&state=st", { cookie: `${LOGIN_COOKIE}=${await login()}`, "cf-connecting-ip": "203.0.113.7", "user-agent": "UA/1" }),
      ENV,
      fetcher,
    );
    expect(res.status).toBe(302);
    expect(res.headers.get("Location")).toBe("/#/t/sim-1");
    const cookies = res.headers.getSetCookie();
    expect(cookies.some((c) => c.startsWith(`${SESSION_COOKIE}=v1.`))).toBe(true);
    expect(cookies.some((c) => c.startsWith(`${LOGIN_COOKIE}=; Max-Age=0`))).toBe(true);
    expect(cookies).toHaveLength(2);
    const exchanges = fetcher.calls.filter((c) => c.url === WORKOS_AUTHENTICATE);
    expect(exchanges).toHaveLength(1);
    expect(JSON.parse(exchanges[0]!.body)).toEqual({
      client_id: "c",
      client_secret: "k",
      grant_type: "authorization_code",
      code: "the-code",
      ip_address: "203.0.113.7",
      user_agent: "UA/1",
    });
    const line = cookies.find((c) => c.startsWith(`${SESSION_COOKIE}=`))!;
    expect(line).toContain(`; Max-Age=${DAYS * DAY}; Path=/; HttpOnly; Secure; SameSite=Lax`);
    const s = await readSession(line, SECRET, now());
    expect(s).toMatchObject({ address: RED, sid: "s1" });
    expect(s!.expires - s!.issued).toBe(DAYS * DAY);
    expect(Math.abs(s!.issued - now()), "issued at the real clock").toBeLessThanOrEqual(2);
  });

  // The configured lifetime (`YOKOHAMA_SESSION_DAYS`) is what the callback mints, what renewal is read due against and
  // what a renewal re-mints. Mutants: 30 days hard-coded at the callback's mint or its `Max-Age`, at the reader's
  // `renewDue`, or at the renewal's mint.
  it("under YOKOHAMA_SESSION_DAYS 7 the callback mints a seven-day cookie, and renewal is due past three and a half days and not before", async () => {
    const week: Env = { ...ENV, YOKOHAMA_SESSION_DAYS: "7" };
    const res = await call(get("/enter?code=x&state=st", { cookie: `${LOGIN_COOKIE}=${await login()}` }), week);
    expect([res.status, res.headers.get("Location")]).toEqual([302, "/#/t/sim-1"]);
    const minted = res.headers.getSetCookie().find((c) => c.startsWith(`${SESSION_COOKIE}=`))!;
    expect(minted).toContain(`; Max-Age=${7 * DAY}; `);
    const s = await readSession(minted, SECRET, now());
    expect(s!.expires - s!.issued).toBe(7 * DAY);
    const week7 = (ago: number) => mintSession({ address: RED, sid: "s1", issued: now() - ago }, 7, SECRET);
    const due = await call(get("/api/me", { cookie: `${SESSION_COOKIE}=${await week7(3.5 * DAY + 60)}` }), week);
    const renewal = due.headers.getSetCookie();
    expect([due.status, renewal.length], "due: one renewal").toEqual([200, 1]);
    expect(renewal[0]).toContain(`; Max-Age=${7 * DAY}; `);
    const r = await readSession(renewal[0]!, SECRET, now());
    expect(r!.expires - r!.issued).toBe(7 * DAY);
    const early = await call(get("/api/me", { cookie: `${SESSION_COOKIE}=${await week7(3.5 * DAY - 60)}` }), week);
    expect([early.status, early.headers.has("Set-Cookie")], "not yet due").toEqual([200, false]);
  });

  // The arms' key sources are one per isolate and JWKS URL (`armKeys`), so a second callback reads the issuer's keys
  // from the first one's cache. The issuer is this cell's own, so no earlier cell warmed its entry. Mutant: a fresh
  // `keySourceAt` per callback.
  it("two callbacks under one oidc issuer fetch its JWKS once", async () => {
    const issuerUrl = "https://jwks-once.issuer.test";
    const once: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: issuerUrl };
    const fetcher = issuer({ nonce: () => "n" });
    for (let i = 0; i < 2; i++) {
      const res = await call(get("/enter?code=x&state=st", { cookie: `${LOGIN_COOKIE}=${await login()}` }), once, fetcher);
      expect([res.headers.get("Location"), res.headers.getSetCookie().some((c) => c.startsWith(`${SESSION_COOKIE}=v1.`))], `callback ${i + 1}`).toEqual(["/#/t/sim-1", true]);
    }
    const asked = (path: string) => fetcher.calls.filter((c) => c.url === `${issuerUrl}${path}`).length;
    expect([asked("/token"), asked("/jwks")], "two exchanges, one key fetch").toEqual([2, 1]);
  });

  // The landing is held to `safeReturnTo` again at the callback: a login cookie this deployment mints holds a safe
  // one already, so the row needs a cookie minted elsewhere under the same secret (an older rule, another minter).
  // Mutant: the callback following the cookie's landing as written.
  it("a login cookie whose landing is unsafe still lands the sign-in on the lobby", async () => {
    for (const returnTo of ["https://evil.test/", "/api/me", "//evil.test/#/lobby"]) {
      const res = await call(get("/enter?code=x&state=st", { cookie: `${LOGIN_COOKIE}=${await login({ returnTo })}` }));
      expect([res.status, res.headers.get("Location")], returnTo).toEqual([302, LOBBY]);
      expect(res.headers.getSetCookie().some((c) => c.startsWith(`${SESSION_COOKIE}=v1.`)), "and the sign-in still completes").toBe(true);
    }
  });

  // The `oidc` callback with PKCE (R-25's second half): the stored verifier is what the token request carries.
  it("under oidc with no secret the callback sends the login cookie's verifier and no secret", async () => {
    const pkce: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://callback.issuer.test", YOKOHAMA_OIDC_CLIENT_SECRET: undefined };
    const started = await call(get("/enter?return_to=%2F%23%2Fnew"), pkce);
    const kept = (await readLogin(started.headers.get("Set-Cookie"), SECRET, now()))!;
    const fetcher = issuer({ nonce: () => kept.nonce });
    const sent = cookieValue(started.headers.get("Set-Cookie"), LOGIN_COOKIE)!;
    const res = await call(get(`/enter?code=c0&state=${kept.state}`, { cookie: `${LOGIN_COOKIE}=${sent}` }), pkce, fetcher);
    expect([res.status, res.headers.get("Location")]).toEqual([302, "/#/new"]);
    const token = fetcher.calls.filter((c) => c.url === "https://callback.issuer.test/token");
    expect(token).toHaveLength(1);
    expect(Object.fromEntries(new URLSearchParams(token[0]!.body))).toEqual({
      grant_type: "authorization_code",
      code: "c0",
      redirect_uri: `${ORIGIN}/enter`,
      client_id: "c",
      code_verifier: kept.verifier,
    });
    const line = res.headers.getSetCookie().find((c) => c.startsWith(`${SESSION_COOKIE}=`));
    expect(await readSession(line ?? null, SECRET, now())).toMatchObject({ address: RED, sid: null });
  });

  // Mutants: the state compared on a prefix or not at all; the exchange run before the state check; a failure that
  // sets the session cookie or leaves the login cookie; the landing another place.
  it("a state mismatch, a missing login cookie, or an exchange answering nobody lands on /?signin=failed#/lobby with no session cookie and no exchange where the state failed", async () => {
    const cookie = { cookie: `${LOGIN_COOKIE}=${await login()}` };
    const rows: Array<[string, Request, Answers, number, string]> = [
      ["a state mismatch", get("/enter?code=x&state=su", cookie), {}, 0, "state"],
      ["a state one character longer", get("/enter?code=x&state=stt", cookie), {}, 0, "state"],
      ["a state that is a prefix", get("/enter?code=x&state=s", cookie), {}, 0, "state"],
      ["no state at all", get("/enter?code=x", cookie), {}, 0, "state"],
      ["no login cookie", get("/enter?code=x&state=st"), {}, 0, "no-login"],
      ["a login cookie past its ten minutes", get("/enter?code=x&state=st", { cookie: `${LOGIN_COOKIE}=${await login({}, now() - 601)}` }), {}, 0, "no-login"],
      ["a login cookie another secret signed", get("/enter?code=x&state=st", { cookie: `${LOGIN_COOKIE}=${await login({}, now(), "another-secret-of-at-least-32-bytes")}` }), {}, 0, "no-login"],
      ["the session cookie's name carrying a login", get("/enter?code=x&state=st", { cookie: `${SESSION_COOKIE}=${await login()}` }), {}, 0, "no-login"],
      ["an empty code", get("/enter?code=&state=st", cookie), {}, 0, "no-code"],
      ["the issuer's error answer", get("/enter?error=access_denied&state=st", cookie), {}, 0, "issuer-error"],
      ["an exchange answering an unverified address", get("/enter?code=x&state=st", cookie), { user: { email: RED, email_verified: false } }, 1, ""],
      ["an exchange refused", get("/enter?code=x&state=st", cookie), { status: 400 }, 1, ""],
    ];
    for (const [label, req, answers, exchanges, reason] of rows) {
      const fetcher = issuer(answers);
      const { result: res, lines } = await signinLines(() => call(req, ENV, fetcher));
      expect([res.status, res.headers.get("Location")], label).toEqual([302, FAILED]);
      expect(res.headers.getSetCookie(), `${label}: the login cookie cleared and nothing else`).toEqual([clearCookie(LOGIN_COOKIE)]);
      expect(fetcher.calls.filter((c) => c.url === WORKOS_AUTHENTICATE), `${label}: exchanges`).toHaveLength(exchanges);
      // The shell's own refusal names its reason; a refused exchange is the arm's own `signin-failed` line.
      expect(lines.map((l) => (l.event === "signin-refused" ? l.reason : l.event)), label).toEqual([reason === "" ? "signin-failed" : reason]);
    }
  });

  // A browser that follows the callback twice (a reload, a replayed URL): the first answer cleared the login cookie,
  // so the second carries none and is refused before any exchange.
  it("a replayed callback is refused: the first answer cleared the login cookie, so the second exchanges nothing", async () => {
    const fetcher = issuer();
    const jar = new Map<string, string>([[LOGIN_COOKIE, await login()]]);
    const send = async () => {
      const cookie = [...jar].map(([n, v]) => `${n}=${v}`).join("; ");
      const res = await call(get("/enter?code=once&state=st", cookie === "" ? {} : { cookie }), ENV, fetcher);
      for (const line of res.headers.getSetCookie()) {
        const [pair] = line.split(";");
        const [name, value] = [pair!.slice(0, pair!.indexOf("=")), pair!.slice(pair!.indexOf("=") + 1)];
        if (/Max-Age=0(;|$)/.test(line)) jar.delete(name);
        else jar.set(name, value);
      }
      return res;
    };
    expect((await send()).headers.get("Location")).toBe("/#/t/sim-1");
    expect([...jar.keys()], "the jar after the first: the session alone").toEqual([SESSION_COOKIE]);
    expect((await send()).headers.get("Location")).toBe(FAILED);
    expect(fetcher.calls.filter((c) => c.url === WORKOS_AUTHENTICATE)).toHaveLength(1);
  });

  it("GET /enter under access stays the 302 to /#/lobby, code or no code, with no cookie", async () => {
    const fetcher = issuer();
    for (const path of ["/enter", "/enter?code=x&state=st", "/enter?return_to=%2F%23%2Ft%2Fsim-1", "/enter?error=x"]) {
      for (const method of ["GET", "HEAD"]) {
        const res = await handle(new Request(ORIGIN + path, { method, headers: { cookie: `${LOGIN_COOKIE}=${await login()}` } }), env, createExecutionContext(), keys, fetcher);
        expect([res.status, res.headers.get("Location"), res.headers.has("Set-Cookie")], `${method} ${path}`).toEqual([302, LOBBY, false]);
      }
    }
    expect(fetcher.calls).toEqual([]);
  });
});

describe("the session cookie names the caller", () => {
  it("a session cookie names the caller on /api/me, with no header and no issuer call", async () => {
    const fetcher = issuer();
    const v = await session();
    const res = await call(get("/api/me", { cookie: `other=1; ${SESSION_COOKIE}=${v}` }), ENV, fetcher);
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ mode: "cf-access", user: RED });
    expect(fetcher.calls).toHaveLength(0);
    expect(res.headers.has("Set-Cookie"), "a fresh cookie is not renewed").toBe(false);
    expect(accessAsked).toBe(0);
  });

  // Mutants: `identify` run under every source (the Access assertion names RED here); the cookie read under access.
  it("under a sign-in arm an Access assertion names no one, and under access a session cookie names no one", async () => {
    const withAccess: Env = { ...ENV, ACCESS_TEAM_DOMAIN: env.ACCESS_TEAM_DOMAIN, ACCESS_AUD: env.ACCESS_AUD };
    const assertion = await key.sign(personClaims(RED, { teamDomain: env.ACCESS_TEAM_DOMAIN!, aud: env.ACCESS_AUD!, now: now() }));
    let asked = 0;
    const accessKeys: KeySource = async () => {
      asked += 1;
      return key.set();
    };
    const me = (e: Env, headers: Record<string, string>) => handle(get("/api/me", headers), e, createExecutionContext(), accessKeys, issuer());
    // Premise: the same assertion names RED under access.
    expect(await (await me(env, { "Cf-Access-Jwt-Assertion": assertion })).json()).toEqual({ mode: "cf-access", user: RED });
    asked = 0;
    // Under workos the assertion names no one, so `/api/me` is the shell's 401 with the challenge (ruling R-44).
    const named = await me(withAccess, { "Cf-Access-Jwt-Assertion": assertion });
    expect([named.status, named.headers.get("WWW-Authenticate")]).toEqual([401, SESSION_CHALLENGE]);
    expect(asked, "the Access keys are never asked under workos").toBe(0);
    expect(await (await me(env, { cookie: `${SESSION_COOKIE}=${await session()}` })).json()).toEqual({ mode: "cf-access", user: null });
  });

  // Under an arm a caller-less `/api/me` is the shell's own 401 (ruling R-44; the next cell), and since Task 5 a
  // caller-less archive post is the anonymous archive's (ruling R-36; the cell after it), so the challenge on the
  // layer's own 401 is read on three requests that reach an object with no caller: the lobby stream's upgrade
  // (unupgraded), a create, and a table's frame. Each is read under the arm and, on the same request, under access.
  // Mutants: the challenge under access (the plan's row); the archive's exemption widened to the table target.
  it("no cookie is the layer's 401 with the session challenge header; under access the header is absent", async () => {
    expect(SESSION_CHALLENGE).toBe('Yokohama-Session realm="/enter"');
    const id = "signin-challenge-table";
    await seedPlaying(id);
    const requests: Array<[string, () => Request]> = [
      ["the stream's upgrade", () => get("/api/stream", { Upgrade: "websocket" })],
      ["a create", () => post("/api/games", JSON_TYPE, CREATE_BODY)],
      ["a table's frame", () => get(`/api/games/${id}/table`)],
    ];
    for (const [label, req] of requests) {
      const res = await call(req());
      expect([res.status, res.webSocket, res.headers.get("WWW-Authenticate")], label).toEqual([401, null, 'Yokohama-Session realm="/enter"']);
      const under = await handle(req(), env, createExecutionContext(), keys, issuer());
      expect([under.status, under.headers.has("WWW-Authenticate")], `${label}, under access`).toEqual([401, false]);
    }
    // On a 401 alone: a 200 and a 403 carry none. (`/api/me` with no caller is a 401 under an arm since R-44.)
    const health = await call(get("/api/health"));
    expect([health.status, health.headers.has("WWW-Authenticate")], "a 200").toEqual([200, false]);
    const foreign = await call(post("/api/games", { ...JSON_TYPE, Origin: FOREIGN }, CREATE_BODY));
    expect([foreign.status, foreign.headers.has("WWW-Authenticate")], "a 403").toEqual([403, false]);
  });

  // Ruling R-44, reversing C-1 for `/api/me` under a sign-in arm ONLY: a caller-less `GET`/`HEAD /api/me` is the shell's
  // own 401 with the session challenge, so a first visitor at a shared table link meets the client's sign-in card,
  // never the door's "This server couldn't tell who you are" card, which is for an operator. The path alone is read,
  // so a query changes nothing; any other method stays the house's. Under access nothing moves: the module's 200
  // `user: null` (the row here, and `test/worker.test.ts`'s "no identity: /api/me reads user null…"). A session cookie
  // on the same route is the 200 naming the caller (the cells above and below). Mutants: the rule dropped (the arm
  // rows); the rule under access too (the access row); the rule on any method (the POST row); the rule over a session
  // too (the "names the caller" cell above).
  it("under a sign-in arm a caller-less /api/me is the shell's 401 with the challenge; under access it stays the 200 naming nobody", async () => {
    const v = await session();
    const flipped = v.slice(0, -1) + (v.endsWith("A") ? "B" : "A");
    const rows: Array<[string, Request]> = [
      ["no cookie", get("/api/me")],
      ["a query", get("/api/me?fresh=1")],
      ["a bad-MAC cookie", get("/api/me", { cookie: `${SESSION_COOKIE}=${flipped}` })],
    ];
    for (const [label, req] of rows) {
      const { result: res, reached } = await reaching(() => call(req));
      expect([res.status, await res.text()], label).toEqual([401, JSON.stringify({ error: NOT_SIGNED_IN })]);
      expect([res.headers.get("WWW-Authenticate"), res.headers.has("Set-Cookie"), res.headers.get("Cache-Control")], label).toEqual([
        'Yokohama-Session realm="/enter"',
        false,
        "no-store",
      ]);
      expect(reached, `${label}: the shell answers, no object`).toBe(0);
    }
    const head = await call(new Request(`${ORIGIN}/api/me`, { method: "HEAD" }));
    expect([head.status, head.headers.get("WWW-Authenticate"), await head.text()], "HEAD").toEqual([401, SESSION_CHALLENGE, ""]);
    const posted = await reaching(() => call(post("/api/me", JSON_TYPE, "{}")));
    expect([posted.house, await posted.result.text() === JSON.stringify({ error: NOT_SIGNED_IN })], "a POST is the house's").toEqual([1, false]);
    const under = await handle(get("/api/me"), env, createExecutionContext(), keys, issuer());
    expect([under.status, await under.json(), under.headers.has("WWW-Authenticate")], "under access").toEqual([200, { mode: "cf-access", user: null }, false]);
  });

  // Ruling R-36: since Task 5 a caller-less archive post files anonymously, so the archive target's one remaining 401
  // is the house refusing a body that names no device and no safe game id: a defect to surface, never "sign in"
  // (design §4.5), and no session challenge rides it — with no cookie, or with one that is nobody. Mutant: the
  // challenge on every 401 under an arm.
  it("a caller-less archive post whose body names no game is the house's 401 without the session challenge", async () => {
    const { result: res, house } = await reaching(() => call(post("/api/archive", JSON_TYPE, NAMELESS_ARCHIVE)));
    expect([res.status, res.headers.has("WWW-Authenticate"), house], "no cookie").toEqual([401, false, 1]);
    const v = await session();
    const flipped = v.slice(0, -1) + (v.endsWith("A") ? "B" : "A");
    const bad = await call(post("/api/archive", { ...JSON_TYPE, cookie: `${SESSION_COOKIE}=${flipped}` }, NAMELESS_ARCHIVE));
    expect([bad.status, bad.headers.has("WWW-Authenticate")], "a bad-MAC cookie").toEqual([401, false]);
  });

  // Mutants: renewal on every status (the 404 row); on a 101 (the upgrade row); a renewal that keeps the old `issued`
  // (the minted cookie's `issued` is now); the renewal on the leave's answer, which would set the session it clears.
  it("a cookie past half its life is renewed on a 200, never on a 401, and never on a 101", async () => {
    const old = await session(RED, "s1", 16 * DAY);
    const renewed = await call(get("/api/me", { cookie: `${SESSION_COOKIE}=${old}` }));
    expect([renewed.status, await renewed.json()]).toEqual([200, { mode: "cf-access", user: RED }]);
    const lines = renewed.headers.getSetCookie();
    expect(lines).toHaveLength(1);
    expect(lines[0]).toMatch(new RegExp(`^${SESSION_COOKIE}=v1\\.[^;]+; Max-Age=${DAYS * DAY}; Path=/; HttpOnly; Secure; SameSite=Lax$`));
    const fresh = await readSession(lines[0]!, SECRET, now());
    expect(fresh).toMatchObject({ address: RED, sid: "s1" });
    expect(Math.abs(fresh!.issued - now()), "renewal re-stamps the time (R-16)").toBeLessThanOrEqual(2);
    expect(renewed.headers.get("Cache-Control"), "and the API's headers still ride").toBe("no-store");

    // Not yet due: fifteen days in is not past half.
    expect((await call(get("/api/me", { cookie: `${SESSION_COOKIE}=${await session(RED, "s1", 15 * DAY - 60)}` }))).headers.has("Set-Cookie")).toBe(false);

    // A bad MAC is nobody: the stream's upgrade is the 401, unupgraded, with the challenge and no cookie.
    const flipped = old.slice(0, -1) + (old.endsWith("A") ? "B" : "A");
    const bad = await call(get("/api/stream", { Upgrade: "websocket", cookie: `${SESSION_COOKIE}=${flipped}` }));
    expect([bad.status, bad.webSocket, bad.headers.get("WWW-Authenticate"), bad.headers.has("Set-Cookie")]).toEqual([401, null, SESSION_CHALLENGE, false]);

    // A due session on an answer that is not 2xx: the table no one made answers 404, and nothing is renewed.
    const missing = await call(get("/api/games/signin-no-such-table/table", { cookie: `${SESSION_COOKIE}=${old}` }));
    expect([missing.status, missing.headers.has("Set-Cookie")]).toEqual([404, false]);

    // A due session on an upgrade: the 101 carries its socket and no cookie.
    const upgraded = await call(get("/api/stream", { Upgrade: "websocket", cookie: `${SESSION_COOKIE}=${old}` }));
    expect(upgraded.status).toBe(101);
    expect(upgraded.headers.has("Set-Cookie")).toBe(false);
    upgraded.webSocket!.accept();
    upgraded.webSocket!.close(1000, "done");

    // A due session signing out: the one session line is the clear.
    const left = await call(post("/api/leave", { cookie: `${SESSION_COOKIE}=${old}`, Origin: ORIGIN }));
    expect(left.headers.getSetCookie()).toEqual([clearCookie(SESSION_COOKIE)]);
  });

  // The limiter key a session caller spends (`limitKey`): its address's digest, as a person's.
  it("a session caller's write is counted under its address's digest", async () => {
    const write = limiter();
    const limited: Env = { ...ENV, WRITE_LIMIT: write as unknown as RateLimit };
    const res = await call(post("/api/chat", { ...JSON_TYPE, cookie: `${SESSION_COOKIE}=${await session()}` }, '{"text":"hi"}'), limited);
    expect(res.status).toBe(201);
    expect(write.asked).toEqual([await callerTag(RED)]);
  });

  it("a session caller plays at a table: the address reaches the object as X-Yokohama-User", async () => {
    const id = "signin-seat";
    await seedPlaying(id);
    const res = await call(post(`/api/games/${id}/chat`, { ...JSON_TYPE, cookie: `${SESSION_COOKIE}=${await session()}` }, '{"text":"hi"}'));
    expect(res.status).toBe(201);
    const frame = await call(get(`/api/games/${id}/table`, { cookie: `${SESSION_COOKIE}=${await session()}` }));
    expect(frame.status).toBe(200);
  });
});

describe("POST /api/leave", () => {
  it("POST /api/leave clears the cookie and answers the leave URL; under access it answers Access's logout and clears nothing", async () => {
    const v = await session();
    const { result: res, reached } = await reaching(() => call(post("/api/leave", { cookie: `${SESSION_COOKIE}=${v}`, Origin: ORIGIN })));
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ leave: `https://api.workos.com/user_management/sessions/logout?session_id=s1&return_to=${encodeURIComponent(`${ORIGIN}/`)}` });
    expect(res.headers.get("Set-Cookie")).toBe(`${SESSION_COOKIE}=; Max-Age=0; Path=/; HttpOnly; Secure; SameSite=Lax`);
    expect([res.headers.get("Cache-Control"), res.headers.get("X-Content-Type-Options")]).toEqual(["no-store", "nosniff"]);
    expect(reached, "the Worker answers it: no object").toBe(0);
    const under = await handle(post("/api/leave", { Origin: ORIGIN }), env, createExecutionContext(), keys, issuer());
    expect(await under.json()).toEqual({ leave: "/cdn-cgi/access/logout" });
    expect(under.headers.has("Set-Cookie")).toBe(false);
    const withCookie = await handle(post("/api/leave", { Origin: ORIGIN, cookie: `${SESSION_COOKIE}=${v}` }), env, createExecutionContext(), keys, issuer());
    expect([await withCookie.json(), withCookie.headers.has("Set-Cookie")]).toEqual([{ leave: "/cdn-cgi/access/logout" }, false]);
  });

  it("with no session the leave lands on the site and still clears the cookie; under oidc it is the issuer's end-session URL with the client named", async () => {
    const none = await call(post("/api/leave", { Origin: ORIGIN }));
    expect([await none.json(), none.headers.getSetCookie()]).toEqual([{ leave: `${ORIGIN}/` }, [clearCookie(SESSION_COOKIE)]]);
    const oidc: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://leave.issuer.test" };
    const v = await session();
    const res = await call(post("/api/leave", { Origin: ORIGIN, cookie: `${SESSION_COOKIE}=${v}` }), oidc);
    expect(await res.json()).toEqual({
      leave: `https://leave.issuer.test/logout?post_logout_redirect_uri=${encodeURIComponent(`${ORIGIN}/`)}&client_id=c`,
    });
    expect(res.headers.getSetCookie()).toEqual([clearCookie(SESSION_COOKIE)]);
    // An issuer whose document cannot be had: the leave is the site's root, logged, and the cookie cleared all the same.
    const down: Env = { ...ENV, YOKOHAMA_IDENTITY_SOURCE: "oidc", YOKOHAMA_OIDC_ISSUER: "https://down.issuer.test" };
    const { result: left, lines } = await signinLines(() => call(post("/api/leave", { Origin: ORIGIN, cookie: `${SESSION_COOKIE}=${v}` }), down));
    expect([left.status, await left.json(), left.headers.getSetCookie()]).toEqual([200, { leave: `${ORIGIN}/` }, [clearCookie(SESSION_COOKIE)]]);
    expect(lines.map((l) => [l.event, l.step])).toEqual([["signin-failed", "leave"]]);
  });

  // The leave is a POST under `/api`, so it spends the write limiter (step 6a) under the caller's key before it is
  // answered: the limiter is its only cost bound (an `oidc` issuer that is down is asked again on every leave).
  // Mutant: the leave answered before `limited`.
  it("a session's leave spends the write limiter once under its address's digest, and a refusal is the 429 that clears nothing", async () => {
    const write = limiter();
    const counted: Env = { ...ENV, WRITE_LIMIT: write as unknown as RateLimit };
    const cookie = `${SESSION_COOKIE}=${await session()}`;
    const res = await call(post("/api/leave", { cookie, Origin: ORIGIN }), counted);
    expect([res.status, res.headers.getSetCookie()]).toEqual([200, [clearCookie(SESSION_COOKIE)]]);
    expect(write.asked).toEqual([await callerTag(RED)]);
    const shut: Env = { ...ENV, WRITE_LIMIT: { limit: async () => ({ success: false }) } as unknown as RateLimit };
    const refused = await call(post("/api/leave", { cookie, Origin: ORIGIN }), shut);
    expect([refused.status, refused.headers.has("Set-Cookie")]).toEqual([429, false]);
  });

  // Under the open door no source signs anyone in, so there is nothing of the Worker's to end: the request is the
  // house's, as it was before this route existed, and its 404 is the client's cue to drop the claimed name.
  it("under the open door POST /api/leave is the house's, and sets nothing", async () => {
    const open: Env = { ...env, YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_DEV_DOOR: "off", YOKOHAMA_IDENTITY_SOURCE: undefined };
    const { result: res, house } = await reaching(() => handle(post("/api/leave", { Origin: ORIGIN }), open, createExecutionContext(), keys, issuer()));
    expect([res.status, house, res.headers.has("Set-Cookie")]).toEqual([404, 1, false]);
  });

  // Mutants: the cross-site check after identity, after the limiter, or skipped for the leave.
  it("POST /api/leave from another site is the cross-site 403 before anything else", async () => {
    const write = limiter();
    const limited: Env = { ...ENV, WRITE_LIMIT: write as unknown as RateLimit };
    const fetcher = issuer();
    const cookie = `${SESSION_COOKIE}=${await session()}`;
    const senders: Array<Record<string, string>> = [{ Origin: FOREIGN, cookie }, { "Sec-Fetch-Site": "cross-site", cookie }, { Origin: "null", cookie }];
    for (const headers of senders) {
      const { result: res, reached } = await reaching(() => call(post("/api/leave", headers), limited, fetcher));
      expect([res.status, await res.text()], JSON.stringify(headers)).toEqual([403, JSON.stringify({ error: CROSS_SITE })]);
      expect([res.headers.has("Set-Cookie"), reached], "nothing cleared, no object").toEqual([false, 0]);
    }
    // A session cookie on another site's write is the same 403, before identity.
    const id = "signin-cross-site";
    await seedPlaying(id);
    const write403 = await call(post(`/api/games/${id}/input`, { ...JSON_TYPE, Origin: FOREIGN, cookie }, '{"line":"1"}'), limited, fetcher);
    expect([write403.status, write403.headers.has("Set-Cookie")]).toEqual([403, false]);
    expect([write.asked, fetcher.calls], "no limiter spent, no issuer asked").toEqual([[], []]);
  });
});

// Ruling R-31: the login cookie is stateless, so the CALLBACK — each one an `authenticate` POST under the deployment's
// API key — spends the create limiter by IP, the key a caller-less request gets (`limitKey`), AFTER the checks that
// cost only a MAC and BEFORE any network call. The start spends nothing. A refusal is the failed landing, never a
// JSON 429 on a navigation; a limiter that throws is logged and the callback proceeds (AD-6). Mutants: the limiter
// skipped on the callback; spent on the start too; spent before the state check (a refused callback counts); its
// answer not read; a throwing limiter that refuses.
describe("the callback spends the create limiter by IP (R-31)", () => {
  /** A fake binding that records every key and answers `answer`'s verdict. */
  function creates(answer: () => boolean): Limiter & { asked: string[] } {
    const asked: string[] = [];
    return { asked, limit: async ({ key: k }) => { asked.push(k); return { success: answer() }; } };
  }
  const from = (ip: string | null, cookie: string): Record<string, string> => (ip === null ? { cookie } : { cookie, "cf-connecting-ip": ip });

  it("two callbacks from one IP count one key twice; a start and a refused callback count nothing; a refusal lands the failure with no exchange", async () => {
    const create = creates(() => true);
    const write = limiter();
    const counted: Env = { ...ENV, CREATE_LIMIT: create as unknown as RateLimit, WRITE_LIMIT: write as unknown as RateLimit };
    const fetcher = issuer();
    expect((await call(get("/enter", { "cf-connecting-ip": "1.1.1.1" }), counted, fetcher)).status).toBe(302);
    expect(create.asked, "a start spends nothing").toEqual([]);
    for (let i = 0; i < 2; i++) {
      const res = await call(get("/enter?code=x&state=st", from("1.1.1.1", `${LOGIN_COOKIE}=${await login()}`)), counted, fetcher);
      expect(res.headers.get("Location"), `callback ${i + 1}`).toBe("/#/t/sim-1");
    }
    expect(create.asked, "both callbacks under the one IP key").toEqual(["ip:1.1.1.1", "ip:1.1.1.1"]);
    expect(new Set(create.asked).size).toBe(1);
    const mismatch = await call(get("/enter?code=x&state=su", from("1.1.1.1", `${LOGIN_COOKIE}=${await login()}`)), counted, fetcher);
    expect(mismatch.headers.get("Location")).toBe(FAILED);
    expect(create.asked, "a callback refused on its state costs no limiter").toHaveLength(2);
    await call(get("/enter?code=x&state=st", from(null, `${LOGIN_COOKIE}=${await login()}`)), counted, fetcher);
    expect(create.asked.at(-1), "no connecting IP reads unknown").toBe("ip:unknown");
    expect(write.asked, "the write limiter is never asked").toEqual([]);
    expect(fetcher.calls.filter((c) => c.url === WORKOS_AUTHENTICATE)).toHaveLength(3);

    const refusing = creates(() => false);
    const shut: Env = { ...ENV, CREATE_LIMIT: refusing as unknown as RateLimit };
    const refusedFetcher = issuer();
    const loginValue = await login();
    const { result: res, lines } = await signinLines(() =>
      call(get("/enter?code=x&state=st", from("1.1.1.1", `${LOGIN_COOKIE}=${loginValue}`)), shut, refusedFetcher),
    );
    expect([res.status, res.headers.get("Location"), res.headers.getSetCookie()]).toEqual([302, FAILED, [clearCookie(LOGIN_COOKIE)]]);
    expect(res.headers.get("Retry-After"), "a navigation, never the JSON 429").toBeNull();
    expect(refusedFetcher.calls, "no exchange").toEqual([]);
    expect(lines.map((l) => [l.event, l.source, l.reason])).toEqual([["signin-refused", "workos", "limited"]]);
  });

  it("a limiter that throws is logged and the callback proceeds", async () => {
    const broken = { limit: async () => { throw new Error("binding unavailable"); } };
    const e: Env = { ...ENV, CREATE_LIMIT: broken as unknown as RateLimit };
    const fetcher = issuer();
    const { result: res, lines } = await logsDuring(async () =>
      call(get("/enter?code=x&state=st", from("1.1.1.1", `${LOGIN_COOKIE}=${await login()}`)), e, fetcher),
    );
    expect(res.headers.get("Location"), "the sign-in completes").toBe("/#/t/sim-1");
    expect(fetcher.calls.filter((c) => c.url === WORKOS_AUTHENTICATE)).toHaveLength(1);
    expect((lines as Array<{ event: string }>).filter((l) => l.event === "limiter-failed"), "logged once").toHaveLength(1);
  });
});

// Ruling R-27: a session is an operator candidate exactly as a person is. Mutant: the gate still person-only.
describe("a session caller and the operator surface (R-27)", () => {
  const OPS_ENV: Env = { ...ENV, YOKOHAMA_WEB_ADMINS: "chief@t.test, Boss@T.test" };
  it("a session caller on the list reaches the operator route; one off the list is the operator 403", async () => {
    const house = vi.spyOn(House.prototype, "fetch");
    try {
      const listed = await call(get("/api/admin/tables", { cookie: `${SESSION_COOKIE}=${await session("boss@t.test")}` }), OPS_ENV);
      expect(listed.status).toBe(200);
      expect(Array.isArray((await listed.json<{ tables: unknown }>()).tables), "the house's listing").toBe(true);
      expect(house.mock.calls.map(([r]) => (r as Request).headers.get("x-yokohama-operator")), "the gate's word, the address").toEqual(["boss@t.test"]);
    } finally {
      house.mockRestore();
    }
    const unlisted = await call(get("/api/admin/tables", { cookie: `${SESSION_COOKIE}=${await session(RED)}` }), OPS_ENV);
    expect([unlisted.status, await unlisted.text()]).toEqual([403, JSON.stringify({ error: NOT_AN_OPERATOR })]);
    const nobody = await call(get("/api/admin/tables"), OPS_ENV);
    expect(nobody.status, "no session is no operator").toBe(403);
  });
});
