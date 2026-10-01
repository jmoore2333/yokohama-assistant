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
import { BEACON_PREFIX, isBeaconCorsError, isDealAnswer, isDealAnswerLine } from "./harness";
import {
  EXIT_LAUNCH_ARGS,
  LIVE_ORIGIN,
  abandonAsCreator,
  accessState,
  baseUrlFor,
  cookiesFrom,
  credential,
  identityHeaders,
  CHROMIUM_MIN_WINDOW_WIDTH,
  exitLayout,
  forgetHidden,
  mustBeApart,
  mustBeVisible,
  placeWindow,
  reportVisibility,
  watchVisibility,
} from "./live";

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
    response: (status: number, url: string, method = "GET") =>
      emit("response", { status: () => status, url: () => url, request: () => ({ method: () => method }) } as unknown as PwResponse),
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

// ---- the exit's precondition: its windows stay visible (R3's R-N6; I-91) --
//
// A hidden page lets its table's socket go by design (R-P8), so a window of
// the exit's headed run that was minimized, or left on another Space, fails
// the one-socket claims for a reason that is not the product's. The spec
// watches every page's visibility from before its first navigation and
// checks it at every checkpoint; these cells hold the two halves through
// fakes — the page's own script included, over a stand-in document.

/** A page as `watchVisibility` and `mustBeVisible` use one: the binding it
 *  was handed, the init script it was handed, and the state `evaluate`
 *  answers — `state`, which a cell sets. */
function visibilityPage() {
  let binding: ((state: string) => unknown) | undefined;
  let script: { fn: unknown; arg: unknown } | undefined;
  const fake = {
    state: "visible",
    page: {
      exposeFunction: async (_name: string, fn: (state: string) => unknown) => {
        binding = fn;
      },
      addInitScript: async (fn: unknown, arg: unknown) => {
        script = { fn, arg };
      },
      evaluate: async () => fake.state,
    } as unknown as Page,
    /** The page reporting `state` through its binding, as its document would. */
    report: (state: string) => binding!(state),
    script: () => script!,
  };
  return fake;
}

describe("watchVisibility and mustBeVisible — the exit's own precondition", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });
  const at = (s: number) => () => new Date(Date.UTC(2026, 8, 24, 6, 30, s));

  it("records every report of a state other than visible, with its time, and counts every report", async () => {
    const fake = visibilityPage();
    let s = 5;
    const watch = await watchVisibility(fake.page, () => at(s)());
    fake.report("visible");
    s = 6;
    fake.report("hidden");
    fake.report("visible");
    expect(watch).toEqual({ reports: 3, hidden: ["hidden at 06:30:06Z"] });
  });

  it("installs the page's half, which reports the document's state as it starts and at every change", async () => {
    const fake = visibilityPage();
    await watchVisibility(fake.page);
    const { fn, arg } = fake.script();
    expect(fn, "the script every document runs first").toBe(reportVisibility);
    const reports: string[] = [];
    const changed: Array<() => void> = [];
    const doc = {
      visibilityState: "visible",
      addEventListener: (type: string, listener: () => void) => {
        if (type === "visibilitychange") changed.push(listener);
      },
    };
    vi.stubGlobal("document", doc);
    vi.stubGlobal("window", { [String(arg)]: (state: string) => reports.push(state) });
    reportVisibility(String(arg));
    doc.visibilityState = "hidden";
    for (const listener of changed) listener();
    doc.visibilityState = "visible";
    for (const listener of changed) listener();
    expect(reports).toEqual(["visible", "hidden", "visible"]);
  });

  it("passes when every page reads visible now and has reported nothing else", async () => {
    const [x, y] = [visibilityPage(), visibilityPage()];
    const [wx, wy] = [await watchVisibility(x.page), await watchVisibility(y.page)];
    x.report("visible");
    y.report("visible");
    await expect(mustBeVisible([["x", x.page, wx], ["y", y.page, wy]], "before the pause after move 5")).resolves.toBeUndefined();
  });

  it("fails as the spec's own precondition, naming the page: hidden now, or hidden since it was watched", async () => {
    const x = visibilityPage();
    const wx = await watchVisibility(x.page, at(0));
    x.report("visible");
    x.state = "hidden";
    await expect(mustBeVisible([["x", x.page, wx]], "after the pause at move 15")).rejects.toThrow(
      /^the exit's own precondition failed after the pause at move 15: x's window was not visible \(hidden now\)\. .*it is not a finding about the product/s,
    );
    // Hidden between two reads and visible again at the second: what a
    // checkpoint's read of the state alone would miss.
    const y = visibilityPage();
    const wy = await watchVisibility(y.page, at(0));
    y.report("hidden");
    y.report("visible");
    await expect(mustBeVisible([["y", y.page, wy]], "at the end of play")).rejects.toThrow(
      "the exit's own precondition failed at the end of play: y's window was not visible (hidden at 06:30:00Z)",
    );
  });

  it("fails when a page has reported nothing at all: the watch itself is not running", async () => {
    const x = visibilityPage();
    const wx = await watchVisibility(x.page);
    await expect(mustBeVisible([["x", x.page, wx]], "before the pause after move 5")).rejects.toThrow(
      "the exit's own precondition cannot be read before the pause after move 5: x's page has reported no visibility at all",
    );
  });

  // The config's side, read from its SOURCE as `BASE_URL` is above: the
  // exit's run launches Chromium with the two switches meant to keep a covered
  // window visible — not enough on macOS, which is why the spec also lays its
  // windows out so that none covers another (below).
  it("is launched, in the exit's run, with the switches meant to keep a covered window visible", () => {
    expect(EXIT_LAUNCH_ARGS).toEqual(["--disable-backgrounding-occluded-windows", "--disable-renderer-backgrounding"]);
    const here = dirname(fileURLToPath(import.meta.url));
    const config = readFileSync(join(here, "..", "playwright.config.ts"), "utf8");
    expect(config).toContain("...(EXIT ? { launchOptions: { args: [...EXIT_LAUNCH_ARGS] } } : {}),");
  });
});

// ---- the exit's windows, none covering another (exit game runs 1 and 2) --
//
// MEASURED on the exit's first headed run (deployment 19): its three windows
// STACKED, and macOS's occlusion marked a covered one hidden whatever the two
// launch switches read. On the second, the three columns asked 420 px each —
// and Chromium widened every window to its narrowest, 500, so they overlapped
// by 74 px, the last ran past the screen's edge, and every window was kept
// below the menu bar at top 38. So the layout is chosen by what Chromium will
// actually draw — three columns where three of its narrowest windows fit,
// else a grid of two rows — under the window manager's top inset, read off
// the first window placed; the placement is READ BACK and refused as the
// spec's own precondition if two windows overlap or one leaves the screen;
// and each context's viewport matches its window, since Playwright emulates a
// viewport whatever the window's size.

describe("exitLayout — three windows, none covering another, sized by what Chromium draws", () => {
  it("is a GRID on this Mac's 1280x832, where three of Chromium's narrowest windows do not fit side by side", () => {
    expect(CHROMIUM_MIN_WINDOW_WIDTH).toBe(500);
    expect(exitLayout(undefined, 0)).toEqual({
      kind: "grid",
      screen: { width: 1280, height: 832 },
      inset: 0,
      windows: [
        { left: 0, top: 0, width: 634, height: 410 },
        { left: 640, top: 0, width: 634, height: 410 },
        { left: 0, top: 416, width: 634, height: 410 },
      ],
      viewport: { width: 618, height: 320 },
    });
    expect(exitLayout("", 0), "an empty variable is the default").toEqual(exitLayout(undefined, 0));
  });

  it("lays the grid under the window manager's top inset — run 2's menu bar, 38 px — so the rows do not meet", () => {
    expect(exitLayout("1280x832", 38)).toEqual({
      kind: "grid",
      screen: { width: 1280, height: 832 },
      inset: 38,
      windows: [
        { left: 0, top: 38, width: 634, height: 391 },
        { left: 640, top: 38, width: 634, height: 391 },
        { left: 0, top: 435, width: 634, height: 391 },
      ],
      viewport: { width: 618, height: 301 },
    });
  });

  it("is three COLUMNS on a screen wide enough for three of Chromium's narrowest windows and their gutters", () => {
    expect(exitLayout("1600x900", 0)).toEqual({
      kind: "columns",
      screen: { width: 1600, height: 900 },
      inset: 0,
      windows: [
        { left: 0, top: 0, width: 527, height: 888 },
        { left: 533, top: 0, width: 527, height: 888 },
        { left: 1066, top: 0, width: 527, height: 888 },
      ],
      viewport: { width: 511, height: 798 },
    });
    // The line between the two: three windows of 500 and their gutters.
    expect(exitLayout("1518x900", 0).kind).toBe("columns");
    expect(exitLayout("1517x900", 0).kind).toBe("grid");
  });

  it("never lays a window over another or off the screen, nor a viewport past its window, at any screen or inset", () => {
    for (const screen of ["1100x832", "1280x832", "1440x900", "1517x982", "1518x982", "1600x900", "1920x1080", "2560x1440"]) {
      for (const inset of [0, 25, 38]) {
        const { screen: s, windows, viewport } = exitLayout(screen, inset);
        const at = `${screen} under ${inset}`;
        windows.forEach((a, i) => {
          expect(a.width, `${at}: window ${i} narrower than Chromium draws`).toBeGreaterThanOrEqual(CHROMIUM_MIN_WINDOW_WIDTH);
          expect(a.top, `${at}: window ${i} above the inset`).toBeGreaterThanOrEqual(inset);
          expect(a.left + a.width, `${at}: window ${i} past the right edge`).toBeLessThanOrEqual(s.width);
          expect(a.top + a.height, `${at}: window ${i} past the bottom`).toBeLessThanOrEqual(s.height);
          expect(viewport.width, `${at}: the viewport is wider than its window`).toBeLessThan(a.width);
          expect(viewport.height, `${at}: the viewport is taller than its window`).toBeLessThan(a.height);
          windows.slice(i + 1).forEach((b, j) => {
            const apart = a.left + a.width <= b.left || b.left + b.width <= a.left || a.top + a.height <= b.top || b.top + b.height <= a.top;
            expect(apart, `${at}: windows ${i} and ${i + 1 + j} overlap`).toBe(true);
          });
        });
      }
    }
  });

  it("refuses a screen it cannot read, one too narrow for two of Chromium's windows, and one too short for two rows, naming EXIT_SCREEN", () => {
    for (const bad of ["1280", "1280x", "x832", "1280\u00d7832", "1280x832x2", "-1280x832", "1280.5x832", "wide"]) {
      expect(() => exitLayout(bad, 0), bad).toThrow(/^EXIT_SCREEN must be the screen's logical size/);
    }
    expect(() => exitLayout("1000x832", 0)).toThrow(
      "EXIT_SCREEN 1000x832 cannot hold two of Chromium's narrowest windows, 500 px each, side by side",
    );
    expect(() => exitLayout("1280x600", 38)).toThrow(
      "EXIT_SCREEN 1280x600 under a 38 px top inset leaves each window a 618 by 185 viewport, and the windows need at least 320 by 280",
    );
  });
});

describe("mustBeApart — the placement READ BACK, as the spec's own precondition", () => {
  const screen = { width: 1280, height: 832 };

  it("passes a grid read back apart and on the screen — edges that touch are not an overlap", () => {
    expect(() =>
      mustBeApart(
        [
          ["x", { left: 0, top: 38, width: 634, height: 391 }],
          ["y", { left: 640, top: 38, width: 634, height: 391 }],
          ["w", { left: 0, top: 435, width: 634, height: 391 }],
        ],
        screen,
        "once the windows were placed",
      ),
    ).not.toThrow();
    expect(() =>
      mustBeApart(
        [
          ["x", { left: 0, top: 0, width: 640, height: 400 }],
          ["y", { left: 640, top: 0, width: 640, height: 400 }],
          ["w", { left: 0, top: 400, width: 640, height: 400 }],
        ],
        screen,
        "once the windows were placed",
      ),
    ).not.toThrow();
  });

  it("refuses run 2's read-back — three windows of 500 overlapping, the last past the edge, all below it — naming each", () => {
    const run2 = [
      ["x", { left: 0, top: 38, width: 500, height: 820 }],
      ["y", { left: 426, top: 38, width: 500, height: 820 }],
      ["w", { left: 852, top: 38, width: 500, height: 820 }],
    ] as const;
    let said = "";
    try {
      mustBeApart(run2, screen, "once the windows were placed");
    } catch (e) {
      said = (e as Error).message;
    }
    expect(said).toMatch(/^the exit's own precondition failed once the windows were placed: /);
    expect(said).toContain("x's window (500x820 at 0,38) leaves the 1280x832 screen");
    expect(said).toContain("w's window (500x820 at 852,38) leaves the 1280x832 screen");
    expect(said).toContain("x's window (500x820 at 0,38) and y's (500x820 at 426,38) overlap");
    expect(said).toContain("y's window (500x820 at 426,38) and w's (500x820 at 852,38) overlap");
    expect(said).not.toContain("x's window (500x820 at 0,38) and w's");
    expect(said).toContain("it is not a finding about the product");
  });

  it("refuses a window that reported no bounds: then the placement cannot be read at all", () => {
    expect(() =>
      mustBeApart([["x", { left: 0, top: 38, width: 634 }]], screen, "once the windows were placed"),
    ).toThrow("the exit's own precondition cannot be read once the windows were placed: x's window reported no height");
  });
});

describe("forgetHidden — the precondition read from the first move on", () => {
  it("forgets what the seating reported and says what it was, keeps the count of reports, and fails on the next hidden", async () => {
    const x = visibilityPage();
    const wx = await watchVisibility(x.page, () => new Date(Date.UTC(2026, 8, 26, 6, 8, 10)));
    x.report("visible");
    x.report("hidden");
    x.report("visible");
    const y = visibilityPage();
    const wy = await watchVisibility(y.page);
    y.report("visible");
    const windows = [["x", x.page, wx], ["y", y.page, wy]] as const;
    expect(forgetHidden(windows)).toEqual(["x: hidden at 06:08:10Z"]);
    expect(wx).toEqual({ reports: 3, hidden: [] });
    await expect(mustBeVisible(windows, "at the start of play")).resolves.toBeUndefined();
    x.report("hidden");
    await expect(mustBeVisible(windows, "before the pause after move 5")).rejects.toThrow(
      "the exit's own precondition failed before the pause after move 5: x's window was not visible (hidden at 06:08:10Z)",
    );
  });
});

describe("placeWindow — a window put in its place through the browser protocol", () => {
  /** A page whose context hands out ONE fake session, for THIS page only. */
  function protocolPage(answer: (method: string, params: unknown) => unknown) {
    const sent: Array<[string, unknown]> = [];
    const fake = { sent, detached: 0, page: undefined as unknown as Page };
    const session = {
      send: async (method: string, params?: unknown) => {
        sent.push([method, params]);
        return answer(method, params);
      },
      detach: async () => {
        fake.detached += 1;
      },
    };
    fake.page = {
      context: () => ({ newCDPSession: async (p: unknown) => (p === fake.page ? session : undefined) }),
    } as unknown as Page;
    return fake;
  }

  it("asks for the page's own window, sets its bounds in the normal state, reads back where it went, and detaches", async () => {
    const fake = protocolPage((method) =>
      method === "Browser.getWindowForTarget"
        ? { windowId: 7, bounds: {} }
        : method === "Browser.getWindowBounds"
          ? { bounds: { left: 426, top: 37, width: 420, height: 795, windowState: "normal" } }
          : {},
    );
    const got = await placeWindow(fake.page, { left: 426, top: 0, width: 420, height: 820 });
    expect(fake.sent).toEqual([
      ["Browser.getWindowForTarget", undefined],
      ["Browser.setWindowBounds", { windowId: 7, bounds: { left: 426, top: 0, width: 420, height: 820, windowState: "normal" } }],
      ["Browser.getWindowBounds", { windowId: 7 }],
    ]);
    // What the window manager reported back — in this fake, a window moved
    // below a menu bar and shortened — which is what the spec prints.
    expect(got).toEqual({ left: 426, top: 37, width: 420, height: 795 });
    expect(fake.detached).toBe(1);
  });

  it("detaches when the protocol refuses as well, and lets the refusal through", async () => {
    const fake = protocolPage((method) => {
      if (method === "Browser.setWindowBounds") throw new Error("Browser window not found");
      return { windowId: 7, bounds: {} };
    });
    await expect(placeWindow(fake.page, { left: 0, top: 0, width: 420, height: 820 })).rejects.toThrow("Browser window not found");
    expect(fake.detached).toBe(1);
  });
});

// ---- the dealing screen's designed 409 (the live tier's finding 3; I-81) ---
//
// `Dealing.tsx`'s own doc: "A 409 here is an ANSWER, not a failure" — the
// typed refusal of a table that flipped to playing between two of the
// browser's requests, which the client answers by going to the board. The
// live run met it on the join spec's page A; locally the race falls the other
// way. Chromium logs any response of 400 or more as a console error, and the
// harness records every one under `BASE_URL`, so this one is forgiven — in
// BOTH modes, since the race is the product's on every host — and nothing
// broader is.

/** A table's deal route, on `base`. */
const DEAL = (base: string) => `${base}/api/games/sim-367fcf21183e035d/deal`;
/** Chromium's line for the 409, as the live run logged it (HTTP/2 sends no
 *  reason) and as HTTP/1.1 words it. */
const DEAL_LINE = "Failed to load resource: the server responded with a status of 409 ()";
const DEAL_LINE_REASON = "Failed to load resource: the server responded with a status of 409 (Conflict)";

describe("isDealAnswer and isDealAnswerLine — the designed 409, exactly", () => {
  it("forgive the 409 on GET …/api/games/<id>/deal, and the line Chromium logs at it, on either host", () => {
    for (const base of [baseUrlFor(false), baseUrlFor(true)]) {
      expect(isDealAnswer(409, "GET", DEAL(base), base), base).toBe(true);
      expect(isDealAnswerLine(DEAL_LINE, DEAL(base), base), base).toBe(true);
      expect(isDealAnswerLine(DEAL_LINE_REASON, DEAL(base), base), base).toBe(true);
    }
  });
  it("forgive no 409 on any other path", () => {
    for (const path of ["/api/games/sim-1", "/api/games/sim-1/keep", "/api/games/sim-1/table", "/api/games/sim-1/join", "/api/stream"]) {
      expect(isDealAnswer(409, "GET", `${LIVE_ORIGIN}${path}`, LIVE_ORIGIN), path).toBe(false);
      expect(isDealAnswerLine(DEAL_LINE, `${LIVE_ORIGIN}${path}`, LIVE_ORIGIN), path).toBe(false);
    }
  });
  it("forgive no other status on /deal, and no other method", () => {
    for (const status of [400, 403, 404, 426, 500]) {
      expect(isDealAnswer(status, "GET", DEAL(LIVE_ORIGIN), LIVE_ORIGIN), String(status)).toBe(false);
      expect(isDealAnswerLine(DEAL_LINE.replace("409", String(status)), DEAL(LIVE_ORIGIN), LIVE_ORIGIN), String(status)).toBe(false);
    }
    expect(isDealAnswer(409, "POST", DEAL(LIVE_ORIGIN), LIVE_ORIGIN)).toBe(false);
  });
  it("forgive no lookalike: /deal2, /deals, a deeper or shallower path, another origin", () => {
    for (const url of [
      `${LIVE_ORIGIN}/api/games/sim-1/deal2`,
      `${LIVE_ORIGIN}/api/games/sim-1/deals`,
      `${LIVE_ORIGIN}/api/games/sim-1/deal/x`,
      `${LIVE_ORIGIN}/api/games/sim-1/x/deal`,
      `${LIVE_ORIGIN}/api/games//deal`,
      `${LIVE_ORIGIN}/api/deal`,
      "https://elsewhere.example/api/games/sim-1/deal",
      "https://yokohama.moore.nyc.evil.example/api/games/sim-1/deal",
    ]) {
      expect(isDealAnswer(409, "GET", url, LIVE_ORIGIN), url).toBe(false);
      expect(isDealAnswerLine(DEAL_LINE, url, LIVE_ORIGIN), url).toBe(false);
    }
  });
  it("forgive no line but Chromium's own: an error that merely quotes it is breakage", () => {
    expect(isDealAnswerLine(`Uncaught Error: ${DEAL_LINE}`, DEAL(LIVE_ORIGIN), LIVE_ORIGIN)).toBe(false);
  });
});

describe("watchForBreakage — the designed 409, in both modes", () => {
  afterEach(() => {
    vi.doUnmock("./live");
    vi.resetModules();
  });

  for (const live of [false, true]) {
    it(`forgives both of its arms ${live ? "live" : "locally"}, counted apart from the beacon, and every other 400 or more still fails`, async () => {
      const { watchForBreakage } = await harnessAt(live);
      const base = baseUrlFor(live);
      const fake = fakePage();
      const watch = watchForBreakage(fake.page);
      fake.response(409, DEAL(base));
      fake.error(DEAL_LINE, DEAL(base));
      expect(watch.failures).toEqual([]);
      expect(watch.dealAnswers).toBe(2);
      expect(watch.forgiven, "the beacon's counter is not this one").toBe(0);

      fake.response(409, `${base}/api/games/sim-1/keep`, "POST");
      fake.response(403, DEAL(base));
      fake.error(DEAL_LINE, `${base}/api/games/sim-1/keep`);
      expect(watch.failures).toEqual([
        `409 ${base}/api/games/sim-1/keep`,
        `403 ${DEAL(base)}`,
        `console.error: ${DEAL_LINE}`,
      ]);
      expect(watch.dealAnswers).toBe(2);
    });
  }
});
