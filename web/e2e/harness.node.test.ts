// **The live tier's node cells** (M6.5c Task 15; spec §5.4, the orchestrator's
// ruling I-75): the one console error the live tier forgives, the switch that
// keeps it forgiven NOWHERE ELSE, and the credential's path into a browser.
//
// Node and not Playwright: every subject here is a pure function or a helper
// driven through fakes, so no browser, no server and no network is involved —
// which is why `npm run test:wasm` can run it where CI installs no Chromium.
//
// **The config is a stand-in, and deliberately.** `harness.ts` imports
// `BASE_URL` from `playwright.config.ts`, and loading the real config REFUSES
// when no Chromium is installed and EMPTIES the walk's scratch directories —
// both wrong for a command that CI's `web-server` job runs with no browser,
// and that may run beside a walk. The stand-in holds the one name the harness
// reads, computed by the config's own formula, `baseUrlFor` — whose use by the
// config a cell below reads off the config's source.

import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../playwright.config", async () => ({ BASE_URL: (await import("./live")).baseUrlFor(false) }));

import type { ConsoleMessage, Page, Response as PwResponse } from "@playwright/test";
import { BEACON_PREFIX, isBeaconCorsError } from "./harness";
import { LIVE_ORIGIN, abandonAsCreator, accessState, baseUrlFor, cookiesFrom, credential, identityHeaders } from "./live";

// ---- the error, as Chromium words it ---------------------------------------
//
// MEASURED (pinned Chromium, Playwright 1.62.1; a local page loading a
// cross-origin `<script crossorigin>` under a context-wide custom header, whose
// host answered the preflight three ways): ONE blocked load logs TWO console
// errors. The CORS sentence, located at the PAGE; and the load's own failure,
// `Failed to load resource: net::ERR_FAILED`, located at the SCRIPT — its text
// names neither the beacon nor CORS. The hosts below are the live ones; the
// sentences are the measured ones.

/** A versioned beacon address, as the edge injects it. */
const BEACON = `${BEACON_PREFIX}/v8c78df7c7c0f484497ecbca7046644da1771523124516`;
const PAGE = `${LIVE_ORIGIN}/#/lobby`;
/** The preflight answered without the header allowed. */
const CORS_HEADER = `Access to script at '${BEACON}' from origin '${LIVE_ORIGIN}' has been blocked by CORS policy: Request header field x-yokohama-claimed-user is not allowed by Access-Control-Allow-Headers in preflight response.`;
/** The preflight answered with no CORS headers at all. */
const CORS_ORIGIN = `Access to script at '${BEACON}' from origin '${LIVE_ORIGIN}' has been blocked by CORS policy: Response to preflight request doesn't pass access control check: No 'Access-Control-Allow-Origin' header is present on the requested resource.`;
/** The same load's failure, one line later. */
const ECHO = "Failed to load resource: net::ERR_FAILED";

describe("isBeaconCorsError — the one console error the live tier forgives", () => {
  it("matches Chromium's CORS sentence about a versioned beacon, whatever the preflight's reason", () => {
    expect(isBeaconCorsError(CORS_HEADER, PAGE)).toBe(true);
    expect(isBeaconCorsError(CORS_ORIGIN, PAGE)).toBe(true);
    // The class, not one resource kind: the sentence opens "Access to <kind>".
    expect(isBeaconCorsError(CORS_HEADER.replace("Access to script", "Access to fetch"), PAGE)).toBe(true);
  });
  it("matches the same load's failure, located at the beacon", () => {
    expect(isBeaconCorsError(ECHO, BEACON)).toBe(true);
  });
  it("matches no CORS error about another address: another host, a lookalike, another path", () => {
    const blocked = (url: string) => CORS_HEADER.replace(BEACON, url);
    expect(isBeaconCorsError(blocked("https://cloudflareinsights.com/cdn-cgi/rum"), PAGE)).toBe(false);
    expect(isBeaconCorsError(blocked("https://static.cloudflareinsights.com.evil.example/beacon.min.js/v1"), PAGE)).toBe(false);
    expect(isBeaconCorsError(blocked("https://static.cloudflareinsights.com/other.js"), PAGE)).toBe(false);
    expect(isBeaconCorsError(blocked("https://static.cloudflareinsights.com/beacon.min.jsx"), PAGE)).toBe(false);
    // The BLOCKED address decides, not any address the sentence carries.
    expect(isBeaconCorsError(blocked(`https://elsewhere.example/x?from=${BEACON}`), PAGE)).toBe(false);
  });
  // The sentence must BE Chromium's, from its first word: an error that
  // merely quotes it — an app's own message about the beacon — is breakage.
  it("matches no error that merely quotes the beacon's CORS sentence", () => {
    expect(isBeaconCorsError(`Uncaught Error: the analytics failed: ${CORS_HEADER}`, PAGE)).toBe(false);
    expect(isBeaconCorsError(` ${CORS_HEADER}`, PAGE)).toBe(false);
  });
  it("matches no other error on the beacon's host", () => {
    expect(isBeaconCorsError("Failed to load resource: the server responded with a status of 404 ()", BEACON)).toBe(false);
    expect(isBeaconCorsError("Failed to load resource: net::ERR_NAME_NOT_RESOLVED", BEACON)).toBe(false);
    expect(isBeaconCorsError("Uncaught SyntaxError: Unexpected token '<'", BEACON)).toBe(false);
  });
  it("matches no failure under the Worker's own origin: neither the load's failure nor a 4xx", () => {
    expect(isBeaconCorsError(ECHO, `${LIVE_ORIGIN}/api/games/sim-1/events`)).toBe(false);
    expect(
      isBeaconCorsError("Failed to load resource: the server responded with a status of 403 ()", `${LIVE_ORIGIN}/api/games/sim-1/join`),
    ).toBe(false);
  });
});

// ---- the switch, through the real `watchForBreakage` ------------------------

type Handler = (arg: unknown) => void;

/** A page that emits what the cells tell it to: `watchForBreakage` only ever
 *  calls `page.on`. */
function fakePage() {
  const handlers = new Map<string, Handler[]>();
  const page = {
    on(event: string, handler: Handler) {
      handlers.set(event, [...(handlers.get(event) ?? []), handler]);
      return page;
    },
  };
  const emit = (event: string, arg: unknown) => (handlers.get(event) ?? []).forEach((h) => h(arg));
  return {
    page: page as unknown as Page,
    error: (text: string, url: string) =>
      emit("console", { type: () => "error", text: () => text, location: () => ({ url, lineNumber: 0, columnNumber: 0 }) } as unknown as ConsoleMessage),
    uncaught: (message: string) => emit("pageerror", new Error(message)),
    response: (status: number, url: string) => emit("response", { status: () => status, url: () => url } as unknown as PwResponse),
  };
}

/** `harness.ts` loaded as a run with `live` set, and BASE_URL the origin that
 *  run would read. */
async function harnessAt(live: boolean) {
  vi.resetModules();
  vi.doMock("../playwright.config", async () => ({
    BASE_URL: (await vi.importActual<typeof import("./live")>("./live")).baseUrlFor(live),
  }));
  vi.doMock("./live", async (original) => ({ ...(await original<typeof import("./live")>()), LIVE: live }));
  return import("./harness");
}

describe("watchForBreakage — the allow-list is the live tier's alone", () => {
  afterEach(() => {
    vi.doUnmock("./live");
    vi.resetModules();
  });

  it("is inert locally: both halves of the beacon's failure are breakage", async () => {
    const { watchForBreakage } = await harnessAt(false);
    const fake = fakePage();
    const watch = watchForBreakage(fake.page);
    fake.error(CORS_HEADER, PAGE);
    fake.error(ECHO, BEACON);
    expect(watch.failures).toEqual([`console.error: ${CORS_HEADER}`, `console.error: ${ECHO}`]);
    expect(watch.forgiven).toBe(0);
  });

  it("forgives both halves live, and counts them, while everything else still fails", async () => {
    const { watchForBreakage } = await harnessAt(true);
    const fake = fakePage();
    const watch = watchForBreakage(fake.page);
    fake.error(CORS_HEADER, PAGE);
    fake.error(ECHO, BEACON);
    expect(watch.failures, "the beacon's failure is forgiven").toEqual([]);
    expect(watch.forgiven).toBe(2);
    // A cross-origin failure reaches the response arm nowhere: nothing under
    // BASE_URL answers it. What does reach it is still breakage.
    fake.error(ECHO, `${LIVE_ORIGIN}/api/games/sim-1/events`);
    fake.error(CORS_HEADER.replace(BEACON, "https://cloudflareinsights.com/cdn-cgi/rum"), PAGE);
    fake.uncaught("boom");
    fake.response(403, `${LIVE_ORIGIN}/api/games/sim-1/join`);
    expect(watch.failures).toEqual([
      `console.error: ${ECHO}`,
      `console.error: ${CORS_HEADER.replace(BEACON, "https://cloudflareinsights.com/cdn-cgi/rum")}`,
      "uncaught: boom",
      `403 ${LIVE_ORIGIN}/api/games/sim-1/join`,
    ]);
    expect(watch.forgiven).toBe(2);
  });
});

// ---- the credential, and how it reaches a browser ---------------------------

describe("credential", () => {
  let dir = "";
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "live-credential-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));
  const scratch = (text: string) => {
    const file = join(dir, ".dev.vars");
    writeFileSync(file, text);
    return file;
  };

  it("reads the two keys by name among the file's others, CRLF or not", () => {
    const file = scratch("R2_ACCESS_KEY_ID=other\r\nCF-Access-Client-Id=dummy-id.access\r\n\r\nCF-Access-Client-Secret=dummy-secret-value\r\n");
    expect(credential(file)).toEqual({ id: "dummy-id.access", secret: "dummy-secret-value" });
  });
  it("refuses an absent file, naming the file and both keys", () => {
    const file = join(dir, "nothing-here");
    expect(() => credential(file)).toThrow(file);
    expect(() => credential(file)).toThrow(/CF-Access-Client-Id.*CF-Access-Client-Secret/);
  });
  it("refuses a missing or empty key, naming the file and the key and never a value", () => {
    const missing = scratch("CF-Access-Client-Id=dummy-id.access\n");
    expect(() => credential(missing)).toThrow(`${missing} holds no CF-Access-Client-Secret= line`);
    let said = "";
    try {
      credential(scratch("CF-Access-Client-Id=\nCF-Access-Client-Secret=dummy-secret-value\n"));
    } catch (e) {
      said = String(e);
    }
    expect(said, "an empty value is absent").toContain("holds no CF-Access-Client-Id= line");
    expect(said, "no value is ever in the sentence").not.toContain("dummy-secret-value");
  });
});

describe("identityHeaders and liveContextOptions", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it("is the claimed header alone — the one header a page's socket cannot set, and no secret", () => {
    expect(identityHeaders("a@e2e.test")).toEqual({ "X-Yokohama-Claimed-User": "a@e2e.test" });
  });
  // Locally a context is exactly today's: the walk signs in through the form,
  // and the client attaches the header itself.
  it("adds nothing to a context unless the run is live", async () => {
    vi.stubEnv("YOKOHAMA_WEB_E2E_TARGET", "");
    // A fresh module, whatever an earlier cell or the shell loaded: `LIVE` is
    // read once, when the module is.
    vi.resetModules();
    const live = await import("./live");
    expect(live.LIVE).toBe(false);
    expect(await live.liveContextOptions("a@e2e.test")).toEqual({});
  });
});

describe("cookiesFrom", () => {
  const AT = Date.UTC(2026, 8, 24, 12, 0, 0);
  const HEALTH = `${LIVE_ORIGIN}/api/health`;

  it("reads Access's cookie as a browser would store it", () => {
    const set = "CF_Authorization=dummy.jwt.value; Path=/; Expires=Fri, 25 Sep 2026 12:00:00 GMT; HttpOnly; Secure; SameSite=None";
    expect(cookiesFrom([set], HEALTH, AT)).toEqual([
      {
        name: "CF_Authorization",
        value: "dummy.jwt.value",
        domain: "yokohama.moore.nyc",
        path: "/",
        expires: Date.UTC(2026, 8, 25, 12, 0, 0) / 1000,
        httpOnly: true,
        secure: true,
        sameSite: "None",
      },
    ]);
  });
  it("lets Max-Age win, dots a Domain, defaults the path to the request's directory and the rest to a session cookie", () => {
    const [aged, bare] = cookiesFrom(["a=1; Max-Age=60; Expires=Fri, 25 Sep 2026 12:00:00 GMT; Domain=moore.nyc; SameSite=Strict", "b=2"], HEALTH, AT);
    expect(aged).toMatchObject({ name: "a", value: "1", domain: ".moore.nyc", path: "/api", expires: AT / 1000 + 60, sameSite: "Strict" });
    expect(bare).toEqual({ name: "b", value: "2", domain: "yokohama.moore.nyc", path: "/api", expires: -1, httpOnly: false, secure: false, sameSite: "Lax" });
  });
  // RFC 6265 §5.3 step 6: a browser ignores a cookie whose Domain the request's
  // host is not within, so seeding one would hand the browser a cookie it
  // would never have stored.
  it("ignores a cookie for a domain the Worker's host is not within", () => {
    expect(cookiesFrom(["c=3; Domain=example.com", "d=4; Domain=yokohama.moore.nyc"], HEALTH, AT).map((c) => c.name)).toEqual(["d"]);
  });
});

describe("accessState — the credential becomes a cookie before any browser sees it", () => {
  const dummy = { id: "dummy-id.access", secret: "dummy-secret-value" };
  const answer = (status: number, cookies: string[]) => {
    const headers = new Headers();
    for (const c of cookies) headers.append("set-cookie", c);
    return new Response(status === 302 ? null : "{}", { status, headers });
  };

  it("presents the credential ONCE, to the Worker's own health route, following no redirect, and keeps what Access set", async () => {
    const calls: Array<{ url: string; init: RequestInit }> = [];
    const state = await accessState(dummy, async (url, init) => {
      calls.push({ url: String(url), init: init ?? {} });
      return answer(200, ["CF_Authorization=dummy.jwt.value; Path=/; HttpOnly; Secure; SameSite=None", "__cf_bm=x; Path=/; Domain=moore.nyc"]);
    });
    expect(calls.map((c) => c.url)).toEqual([`${LIVE_ORIGIN}/api/health`]);
    expect(calls[0]!.init.redirect).toBe("manual");
    expect(calls[0]!.init.headers).toEqual({ "CF-Access-Client-Id": "dummy-id.access", "CF-Access-Client-Secret": "dummy-secret-value" });
    expect(state.origins).toEqual([]);
    expect(state.cookies.map((c) => `${c.name} ${c.domain} ${c.path} ${c.httpOnly}`)).toEqual([
      "CF_Authorization yokohama.moore.nyc / true",
      "__cf_bm .moore.nyc / false",
    ]);
  });
  it("refuses an answer that is not 200, or a 200 with no CF_Authorization, and never names the secret", async () => {
    await expect(accessState(dummy, async () => answer(302, []))).rejects.toThrow(`GET ${LIVE_ORIGIN}/api/health answered 302`);
    await expect(accessState(dummy, async () => answer(200, ["__cf_bm=x; Path=/"]))).rejects.toThrow("set no CF_Authorization cookie");
    const said = await accessState(dummy, async () => answer(403, [])).catch((e: unknown) => String(e));
    expect(said).not.toContain("dummy-secret-value");
  });
  // A non-200 names no cause, because several read the same: the zone's Bot
  // Fight Mode answers a bot-shaped request 403 BEFORE Access, and a Worker
  // 5xx would read like Access's refusal. It says what came back instead —
  // the body's type and size, never the body, and never a redirect's address.
  it("says what came back — the body's type and size, or a redirect — and asserts no cause", async () => {
    const challenge = "<html><title>Just a moment...</title></html>";
    const page = new Response(challenge, { status: 403, headers: { "content-type": "text/html; charset=UTF-8" } });
    const said = await accessState(dummy, async () => page).catch((e: unknown) => String(e));
    expect(said).toContain(`GET ${LIVE_ORIGIN}/api/health answered 403 and not 200 — a text/html; charset=UTF-8 body of ${challenge.length} B`);
    expect(said).not.toContain("Just a moment");
    expect(said).not.toContain("Access did not admit it");
    const moved = await accessState(dummy, async () => answer(302, [])).catch((e: unknown) => String(e));
    expect(moved).toContain("answered 302 and not 200 — a redirect (its location is not printed");
  });
});

describe("abandonAsCreator", () => {
  const pageAnswering = (status: number, calls: Array<[string, unknown]>) =>
    ({
      request: {
        delete: async (url: string, options: unknown) => {
          calls.push([url, options]);
          return { ok: () => status >= 200 && status < 300, status: () => status };
        },
      },
    }) as unknown as Page;

  it("deletes the table through the creator's own page, following no redirect", async () => {
    const calls: Array<[string, unknown]> = [];
    await abandonAsCreator(pageAnswering(204, calls), "sim-00ab");
    expect(calls).toEqual([[`${LIVE_ORIGIN}/api/games/sim-00ab`, { maxRedirects: 0 }]]);
  });
  it("refuses an answer that did not abandon it, naming the table and the operator's force", async () => {
    // The force goes WITHOUT the credential, over WARP (R-6.5c.16): the
    // credential is never an operator, and would earn the gate's 403.
    await expect(abandonAsCreator(pageAnswering(403, []), "sim-00ab")).rejects.toThrow(
      `the live table sim-00ab was not abandoned: DELETE answered 403 — force it as the operator, over WARP, with no credential: ` +
        `POST ${LIVE_ORIGIN}/api/admin/tables/sim-00ab/expire?force`,
    );
  });
});

// ---- a thrown API call, live (the review's H-1; ruling I-79) ---------------
//
// MEASURED (Playwright 1.62.1): a THROWN API request's message carries a
// "Call log" listing every header it sent — the context's cookie among them,
// `CF_Authorization=<session>` live — and Playwright writes a failing test's
// error to the reporter AND to `test-results/<test>/error-context.md`,
// whatever `trace` reads. So every live `request.*` call goes through
// `liveRequest`, and these cells pin what it lets out.

/** A thrown API call's message, as Playwright words one: its first line, then
 *  the call log — with a DUMMY cookie where a live one would be. */
const THROWN = [
  "apiRequestContext.delete: connect ECONNREFUSED 127.0.0.1:9",
  "Call log:",
  `  - → DELETE ${LIVE_ORIGIN}/api/games/sim-00ab`,
  "    - user-agent: Mozilla/5.0",
  "    - cookie: CF_Authorization=DUMMY-SESSION-5678",
  "    - x-yokohama-claimed-user: a@e2e.test",
].join("\n");

/** `live.ts` loaded as a run whose target is `target`: `LIVE` is read once,
 *  when the module is. */
async function liveAt(target: string) {
  vi.resetModules();
  vi.stubEnv("YOKOHAMA_WEB_E2E_TARGET", target);
  return import("./live");
}

describe("liveRequest — a live call's log never leaves the run", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  it("live, rethrows the call's first line alone, and says why the rest is withheld", async () => {
    const live = await liveAt("live");
    const error = await live
      .liveRequest(async () => {
        throw new Error(THROWN);
      })
      .catch((e: unknown) => e as Error);
    expect(error.message).toBe(
      "apiRequestContext.delete: connect ECONNREFUSED 127.0.0.1:9 (the call log is withheld: it lists the live context's cookie)",
    );
    expect(error.message).not.toContain("DUMMY-SESSION-5678");
    // And no `cause`: MEASURED, the reporter prints a cause's message as
    // "[cause]: …" and writes it into `error-context.md` as well.
    expect(error.cause).toBeUndefined();
  });
  it("locally, lets the call's own error through untouched: there is no cookie to withhold", async () => {
    const live = await liveAt("");
    const error = new Error(THROWN);
    await expect(
      live.liveRequest(async () => {
        throw error;
      }),
    ).rejects.toBe(error);
  });
  it("hands back what the call answers", async () => {
    const live = await liveAt("live");
    expect(await live.liveRequest(async () => 7)).toBe(7);
  });
  it("carries the abandon's DELETE: live, its failure holds no cookie", async () => {
    const live = await liveAt("live");
    const page = {
      request: {
        delete: async () => {
          throw new Error(THROWN);
        },
      },
    } as unknown as Page;
    const said = await live.abandonAsCreator(page, "sim-00ab").catch((e: unknown) => (e as Error).message);
    expect(said).toContain("apiRequestContext.delete: connect ECONNREFUSED 127.0.0.1:9");
    expect(said).not.toContain("DUMMY-SESSION-5678");
  });
});

// ---- where a walk plays (plan review M9; the review's L-8) -----------------

describe("baseUrlFor", () => {
  it("is the Worker's origin live, and the local binary's own port otherwise", () => {
    expect(baseUrlFor(true)).toBe(LIVE_ORIGIN);
    expect(baseUrlFor(false)).toBe("http://127.0.0.1:8479");
  });
  // The EXPORTED constant is what `watchForBreakage` and `two_browsers` read,
  // so a config that went back to a constant local origin would watch nothing
  // live. Read from its SOURCE, since loading it empties the walk's scratch
  // directories.
  it("is what playwright.config.ts exports as BASE_URL", () => {
    const here = dirname(fileURLToPath(import.meta.url));
    const config = readFileSync(join(here, "..", "playwright.config.ts"), "utf8");
    expect(config).toContain("export const BASE_URL = baseUrlFor(LIVE);");
  });
});
