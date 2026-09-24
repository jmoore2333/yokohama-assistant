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
// reads, with the value the local config gives it.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../playwright.config", () => ({ BASE_URL: "http://127.0.0.1:8479" }));

import type { ConsoleMessage, Page, Response as PwResponse } from "@playwright/test";
import { BEACON_PREFIX, isBeaconCorsError } from "./harness";
import { LIVE_ORIGIN, abandonAsCreator, accessState, cookiesFrom, credential, identityHeaders } from "./live";

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
  vi.doMock("../playwright.config", () => ({ BASE_URL: live ? LIVE_ORIGIN : "http://127.0.0.1:8479" }));
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
    await expect(abandonAsCreator(pageAnswering(403, []), "sim-00ab")).rejects.toThrow(
      `the live table sim-00ab was not abandoned: DELETE answered 403 — force it as the operator: POST ${LIVE_ORIGIN}/api/admin/tables/sim-00ab/expire?force`,
    );
  });
});
