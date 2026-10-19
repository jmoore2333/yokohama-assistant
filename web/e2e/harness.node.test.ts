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
// when no Chromium is installed — wrong for a command that CI's `web-server`
// job runs with no browser. (It no longer empties the walk's scratch
// directories at a load: `fresh_dirs.node.test.ts`.) The stand-in holds the
// one name the harness reads, computed by the config's own formula,
// `baseUrlFor` — whose use by the config a cell below reads off the config's
// source.

import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("../playwright.config", async () => ({ BASE_URL: (await import("./live")).baseUrlFor(false) }));

import type { ConsoleMessage, Page, Response as PwResponse } from "@playwright/test";
import { BEACON_PREFIX, capSentence, isBeaconCorsError, isDealAnswer, isDealAnswerLine, isReleaseFeedError, tally } from "./harness";
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
  sessionCookies,
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

// ---- the release feed's failure: the live tier's second known error --------
//
// MEASURED (pinned Chromium, Playwright 1.62.1; a local page fetching a local
// "feed" that answers 404 with `Access-Control-Allow-Origin: *` and a
// preflight allowing GitHub's own header list, which does not name the claimed
// header — GitHub's API answered a preflight with exactly that list on
// 2026-10-01). WITHOUT a custom header the fetch logs ONE error, the 404 line,
// located at the feed. WITH the live contexts' context-wide claimed header it
// is preflighted and refused: TWO errors, the CORS sentence located at the
// PAGE and naming the feed as blocked, then `net::ERR_FAILED` located at the
// feed — the beacon's shape. The hosts below are the live ones; the sentences
// are the measured ones (the 404's reason is empty over HTTP/2).

/** The feed a production build derives from its GitHub project link. */
const FEED = "https://api.github.com/repos/jmoore2333/yokohama-assistant/releases/latest";
const FEED_404 = "Failed to load resource: the server responded with a status of 404 ()";
const FEED_CORS = `Access to fetch at '${FEED}' from origin '${LIVE_ORIGIN}' has been blocked by CORS policy: Request header field x-yokohama-claimed-user is not allowed by Access-Control-Allow-Headers in preflight response.`;

describe("isReleaseFeedError — the release feed's failure while the repository is private", () => {
  it("matches the feed's 404, located at the feed, whatever its reason phrase", () => {
    expect(isReleaseFeedError(FEED_404, FEED)).toBe(true);
    expect(isReleaseFeedError("Failed to load resource: the server responded with a status of 404 (Not Found)", FEED)).toBe(true);
  });
  it("matches the feed's CORS refusal and the load's failure located at the feed", () => {
    expect(isReleaseFeedError(FEED_CORS, PAGE)).toBe(true);
    expect(isReleaseFeedError(ECHO, FEED)).toBe(true);
  });
  it("matches any repository's feed, as any build derives it", () => {
    expect(isReleaseFeedError(FEED_404, "https://api.github.com/repos/someone/fork/releases/latest")).toBe(true);
  });
  it("matches no other status from the feed", () => {
    expect(isReleaseFeedError(FEED_404.replace("404", "403"), FEED)).toBe(false);
    expect(isReleaseFeedError(FEED_404.replace("404", "500"), FEED)).toBe(false);
    expect(isReleaseFeedError("Failed to load resource: net::ERR_NAME_NOT_RESOLVED", FEED)).toBe(false);
  });
  it("matches no other address: another API path, another host, a lookalike, a query, the Worker's own origin", () => {
    for (const url of [
      "https://api.github.com/repos/jmoore2333/yokohama-assistant/releases",
      "https://api.github.com/repos/jmoore2333/yokohama-assistant/releases/latest/x",
      "https://api.github.com/repos/jmoore2333/releases/latest",
      "https://api.github.com.evil.example/repos/a/b/releases/latest",
      "http://api.github.com/repos/a/b/releases/latest",
      `${FEED}?x=1`,
      `${LIVE_ORIGIN}/api/health`,
      `${LIVE_ORIGIN}/repos/a/b/releases/latest`,
    ]) {
      expect(isReleaseFeedError(FEED_404, url), url).toBe(false);
      expect(isReleaseFeedError(ECHO, url), url).toBe(false);
      expect(isReleaseFeedError(FEED_CORS.replace(FEED, url), PAGE), url).toBe(false);
    }
  });
  it("matches no error that merely quotes the feed's sentence", () => {
    expect(isReleaseFeedError(`Uncaught Error: ${FEED_CORS}`, PAGE)).toBe(false);
    expect(isReleaseFeedError(` ${FEED_404}`, FEED)).toBe(false);
  });
  it("is not the beacon's, and the beacon's is not it", () => {
    expect(isReleaseFeedError(CORS_HEADER, PAGE)).toBe(false);
    expect(isBeaconCorsError(FEED_CORS, PAGE)).toBe(false);
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

  it("is inert locally for the feed too: its 404 and its CORS refusal are breakage", async () => {
    const { watchForBreakage } = await harnessAt(false);
    const fake = fakePage();
    const watch = watchForBreakage(fake.page);
    fake.error(FEED_404, FEED);
    fake.error(FEED_CORS, PAGE);
    fake.error(ECHO, FEED);
    expect(watch.failures).toEqual([`console.error: ${FEED_404}`, `console.error: ${FEED_CORS}`, `console.error: ${ECHO}`]);
    expect(watch.forgiven).toBe(0);
    expect(watch.feedForgiven).toBe(0);
  });

  it("forgives the feed's failure live in both its shapes, counted in its OWN counter, and nothing beside it", async () => {
    const { watchForBreakage } = await harnessAt(true);
    const fake = fakePage();
    const watch = watchForBreakage(fake.page);
    fake.error(FEED_404, FEED);
    fake.error(FEED_CORS, PAGE);
    fake.error(ECHO, FEED);
    expect(watch.failures, "the feed's failure is forgiven").toEqual([]);
    expect(watch.feedForgiven, "and counted as the feed's").toBe(3);
    expect(watch.forgiven, "never as the beacon's, which the live specs print as the beacon's").toBe(0);
    fake.error(FEED_404.replace("404", "500"), FEED);
    fake.error(FEED_404, `${LIVE_ORIGIN}/api/health`);
    expect(watch.failures).toEqual([
      `console.error: ${FEED_404.replace("404", "500")}`,
      `console.error: ${FEED_404}`,
    ]);
    expect([watch.forgiven, watch.feedForgiven]).toEqual([0, 3]);
  });

  it("keeps the beacon's count and the feed's apart: each moves only with its own error", async () => {
    const { watchForBreakage } = await harnessAt(true);
    const fake = fakePage();
    const watch = watchForBreakage(fake.page);
    fake.error(CORS_HEADER, PAGE);
    fake.error(ECHO, BEACON);
    expect([watch.forgiven, watch.feedForgiven], "the beacon's two lines").toEqual([2, 0]);
    fake.error(FEED_CORS, PAGE);
    fake.error(ECHO, FEED);
    expect([watch.forgiven, watch.feedForgiven], "then the feed's two").toEqual([2, 2]);
    fake.error(FEED_404, FEED);
    expect([watch.forgiven, watch.feedForgiven], "then the feed's 404").toEqual([2, 3]);
    fake.error(CORS_HEADER, PAGE);
    expect([watch.forgiven, watch.feedForgiven], "then the beacon's again").toEqual([3, 3]);
    expect(watch.failures).toEqual([]);
    expect(watch.dealAnswers, "and neither is the deal's").toBe(0);
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
  // **The minted session** (auth cycle, Task 8; rulings R-39, R-50, R-53, R-54): under a sign-in arm the Worker names a
  // caller by its own cookie. The session rides the context's COOKIE JAR (`sessionCookies`), never `extraHTTPHeaders`
  // (the beacon's cross-origin load carries a context header), and no claimed name rides beside it.
  const SECRET = "0123456789abcdef0123456789abcdef";
  const payload = (line: string) => JSON.parse(Buffer.from(line.split("=")[1]!.split(".")[1]!, "base64url").toString("utf8"));
  /** A minted-shaped cookie line for `address` (a fake mac: the instrument reads its own cookie, the Worker verifies). */
  const lineFor = (address: string) =>
    `__Host-yokohama_session=v1.${Buffer.from(JSON.stringify({ address, sid: null, issued: 1_789_000_000, expires: 1_791_592_000 }), "utf8").toString("base64url")}.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`;

  describe("under YOKOHAMA_SESSION_COOKIE", () => {
    it("is the cookie's own address and no other: its jar cookie, and no header at all", () => {
      const line = lineFor("b@e2e.test");
      vi.stubEnv("YOKOHAMA_SESSION_COOKIE", line);
      expect(identityHeaders("b@e2e.test"), "no claimed header and no Cookie header").toEqual({});
      const [c] = sessionCookies("b@e2e.test");
      expect(c!.name).toBe("__Host-yokohama_session");
      expect(c!.value).toBe(line.slice("__Host-yokohama_session=".length));
    });
    it("refuses a context for anyone else with the two-identities sentence, before any context opens (R-53)", () => {
      vi.stubEnv("YOKOHAMA_SESSION_COOKIE", lineFor("b@e2e.test"));
      expect(() => identityHeaders("a@e2e.test")).toThrow("the session cookie names the caller — a claimed name beside it is two identities");
      expect(() => sessionCookies("a@e2e.test")).toThrow("a@e2e.test");
      expect(() => sessionCookies("a@e2e.test")).toThrow("YOKOHAMA_SESSION_COOKIE names b@e2e.test");
      // Only the cookie's own address, exactly: not one that shares a start, an end or its case with it.
      for (const near of ["bb@e2e.test", "b@e2e.test.test", "B@E2E.TEST", "b@e2e.test "]) {
        expect(() => identityHeaders(near), near).toThrow("two identities");
      }
    });
    it("refuses a cookie for an address that is not a .test one, in either helper", () => {
      vi.stubEnv("YOKOHAMA_SESSION_COOKIE", lineFor("jamoore@gmail.com"));
      for (const call of [() => identityHeaders("jamoore@gmail.com"), () => sessionCookies("jamoore@gmail.com")]) {
        expect(call).toThrow("YOKOHAMA_SESSION_COOKIE: mint-session: only a .test address may be minted — got jamoore@gmail.com");
      }
    });
    it("an empty variable is unset: the dev door's claimed header", () => {
      vi.stubEnv("YOKOHAMA_SESSION_COOKIE", "");
      expect(identityHeaders("a@e2e.test")).toEqual({ "X-Yokohama-Claimed-User": "a@e2e.test" });
      expect(sessionCookies("a@e2e.test")).toEqual([]);
    });
  });

  // **The secret** (ruling R-50): a session MINTED per email from the deployment's secret — two emails, two callers.
  describe("under YOKOHAMA_SESSION_SECRET", () => {
    const lineOf = (email: string) => `${sessionCookies(email)[0]!.name}=${sessionCookies(email)[0]!.value}`;
    it("is a different jar cookie per email, each minted for its own address, with no header", () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      expect(identityHeaders("a@e2e.test")).toEqual({});
      const [ca, cb] = [lineOf("a@e2e.test"), lineOf("b@e2e.test")];
      expect(ca).toMatch(/^__Host-yokohama_session=v1\./);
      expect(ca).not.toBe(cb);
      expect(payload(ca).address).toBe("a@e2e.test");
      expect(payload(cb).address).toBe("b@e2e.test");
      expect(payload(ca).sid).toBeNull();
      expect(payload(ca).expires - payload(ca).issued).toBe(30 * 86_400);
    });
    it("is host-only, Secure, HttpOnly, Lax, Path / on the Worker's origin, with the session's own expiry (R-54)", () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      const cookies = sessionCookies("a@e2e.test");
      expect(cookies).toHaveLength(1);
      const c = cookies[0]!;
      expect(c).toEqual({
        name: "__Host-yokohama_session",
        value: c.value,
        domain: new URL(LIVE_ORIGIN).hostname,
        path: "/",
        expires: payload(lineOf("a@e2e.test")).expires,
        httpOnly: true,
        secure: true,
        sameSite: "Lax",
      });
      expect(c.domain, "host-only: no leading dot").not.toMatch(/^\./);
    });
    it("is minted at the real clock, in seconds", () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      const before = Math.floor(Date.now() / 1000);
      const issued = payload(lineOf("a@e2e.test")).issued;
      const after = Math.floor(Date.now() / 1000);
      // A millisecond clock would stand for millennia, a fixed one for nothing.
      expect(issued).toBeGreaterThanOrEqual(before);
      expect(issued).toBeLessThanOrEqual(after);
    });
    it("is the same bytes the minting script's own function writes, signed with the secret", async () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      const { cookieLine } = await import("../../worker/scripts/mint-session.mjs");
      const line = lineOf("a@e2e.test");
      expect(line).toBe(cookieLine("a@e2e.test", SECRET, payload(line).issued));
    });
    it("refuses an email that is not a .test address, in the script's fence sentence, in either helper", () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      for (const call of [() => identityHeaders("jamoore@gmail.com"), () => sessionCookies("jamoore@gmail.com")]) {
        expect(call).toThrow("mint-session: only a .test address may be minted — got jamoore@gmail.com");
      }
      expect(() => sessionCookies("A@E2E.TEST")).toThrow("only a .test address may be minted");
    });
    it("wins over the one cookie, which then names nobody and refuses nobody", () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      vi.stubEnv("YOKOHAMA_SESSION_COOKIE", lineFor("b@e2e.test"));
      expect(payload(lineOf("a@e2e.test")).address).toBe("a@e2e.test");
    });
    it("an empty secret is unset", () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", "");
      expect(identityHeaders("a@e2e.test")).toEqual({ "X-Yokohama-Claimed-User": "a@e2e.test" });
    });
  });

  // **The context's storage state carries the session; its headers carry no Cookie** (ruling R-54). A live run with no
  // `.dev.vars` on the workstation — read here through a stand-in `readIdentity`, so no cell reaches the network or the
  // workstation's own file.
  describe("liveContextOptions under a minted session", () => {
    async function liveWith(credential: { id: string; secret: string } | null) {
      vi.stubEnv("YOKOHAMA_WEB_E2E_TARGET", "live");
      vi.resetModules();
      vi.doMock("../../worker/scripts/dev-vars.mjs", async (original) => ({
        ...(await original<typeof import("../../worker/scripts/dev-vars.mjs")>()),
        readIdentity: () => ({ secret: null, cookie: null, credential }),
      }));
      return import("./live");
    }
    afterEach(() => vi.doUnmock("../../worker/scripts/dev-vars.mjs"));

    it("seeds the cookie into storageState and puts no Cookie (and no claimed header) in extraHTTPHeaders", async () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      const live = await liveWith(null);
      const options = await live.liveContextOptions("a@e2e.test");
      const state = options.storageState as { cookies: Array<{ name: string; value: string; httpOnly: boolean; secure: boolean }>; origins: unknown[] };
      expect(state.cookies.map((c) => c.name)).toEqual(["__Host-yokohama_session"]);
      expect(payload(`${state.cookies[0]!.name}=${state.cookies[0]!.value}`).address).toBe("a@e2e.test");
      expect(options.extraHTTPHeaders).toEqual({});
      expect(JSON.stringify(options.extraHTTPHeaders ?? {})).not.toMatch(/cookie/i);
    });
    it("two emails, two contexts, two distinct cookies", async () => {
      vi.stubEnv("YOKOHAMA_SESSION_SECRET", SECRET);
      const live = await liveWith(null);
      const value = async (email: string) => ((await live.liveContextOptions(email)).storageState as { cookies: Array<{ value: string }> }).cookies[0]!.value;
      expect(await value("a@e2e.test")).not.toBe(await value("b@e2e.test"));
    });
    it("refuses the second context's email under the one cookie, before any context opens", async () => {
      vi.stubEnv("YOKOHAMA_SESSION_COOKIE", lineFor("b@e2e.test"));
      const live = await liveWith(null);
      await expect(live.liveContextOptions("b@e2e.test")).resolves.toBeDefined();
      await expect(live.liveContextOptions("a@e2e.test")).rejects.toThrow("a claimed name beside it is two identities");
    });
    it("with no minted session and no credential the context carries the claimed header alone", async () => {
      const live = await liveWith(null);
      expect(await live.liveContextOptions("a@e2e.test")).toEqual({ extraHTTPHeaders: { "X-Yokohama-Claimed-User": "a@e2e.test" } });
    });
  });
  it("refuses a variable that is not a minted line, naming it and never the value", () => {
    vi.stubEnv("YOKOHAMA_SESSION_COOKIE", "red@t.test");
    expect(() => identityHeaders("a@e2e.test")).toThrow("YOKOHAMA_SESSION_COOKIE");
    expect(() => identityHeaders("a@e2e.test")).not.toThrow("red@t.test");
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

// **`who`: an identity follows the target** (security S2b, C1; Task 5b). The
// local tier's binary is `production`, `NamesOnly`, so a caller there is a plain
// NAME; the live tier's dev door names a `.test` ADDRESS, and a spec whose live
// identities carry their own domain keeps it — `readme_shots`' `ben@readme.test`
// is what the README's alt text names. Through the ENV, as a run sets it: `LIVE`
// is read once, when `live.ts` is, so each cell loads a fresh `harness.ts` (and
// with it a fresh `live.ts`) after stubbing the variable.
describe("who — an identity follows the target", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  /** `harness.ts` as a run whose target is `target` loads it. */
  async function harnessUnder(target: string) {
    vi.stubEnv("YOKOHAMA_WEB_E2E_TARGET", target);
    vi.resetModules();
    return import("./harness");
  }

  it("live, a `.test` address: the default domain, or the spec's own", async () => {
    const h = await harnessUnder("live");
    expect(h.who("x")).toBe("x@e2e.test");
    expect(h.who("ben", "readme")).toBe("ben@readme.test");
    expect(h.WALKER).toBe("walker@e2e.test");
  });

  it("locally, the bare name, whatever the domain", async () => {
    const h = await harnessUnder("");
    expect(h.who("x")).toBe("x");
    expect(h.who("ben", "readme")).toBe("ben");
    expect(h.WALKER).toBe("walker");
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

// ---- a live table, known from the moment its create is SENT (R3's L-9) ------

describe("awaitCreate and abandonOpened — a live table is taken away whether or not the walk read its create's reply", () => {
  afterEach(() => {
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  /** A page whose responses the cell lands one at a time: `awaitCreate` calls
   *  `waitForResponse`, which settles on the first response its predicate
   *  takes, or fails as Playwright 1.62.1's does — a `TimeoutError` at its
   *  timeout, and an error named plain `Error` when the page closes or
   *  crashes; the abandon calls `request.delete`. A body that is an `Error`
   *  is one `json()` cannot read. */
  function creatingPage() {
    let predicate: ((r: PwResponse) => boolean) | undefined;
    let answer: ((r: PwResponse) => void) | undefined;
    let fail: ((e: Error) => void) | undefined;
    const deleted: string[] = [];
    const page = {
      waitForResponse: (p: (r: PwResponse) => boolean) => {
        predicate = p;
        return new Promise<PwResponse>((resolve, reject) => {
          answer = resolve;
          fail = reject;
        });
      },
      request: {
        delete: async (url: string) => {
          deleted.push(url);
          return { ok: () => true, status: () => 204 };
        },
      },
    };
    return {
      page: page as unknown as Page,
      deleted,
      armed: () => predicate !== undefined,
      land: (status: number, body: unknown, method = "POST", url = `${LIVE_ORIGIN}/api/games`) => {
        const json = async () => {
          if (body instanceof Error) throw body;
          return body;
        };
        const r = { status: () => status, url: () => url, request: () => ({ method: () => method }), json } as unknown as PwResponse;
        if (predicate?.(r)) answer?.(r);
      },
      end: (name: string, message: string) => fail?.(Object.assign(new Error(message), { name })),
    };
  }

  // Mutant: the wait armed after the call returns — or only once the walk
  // reads the reply — which a response landing first would pass by.
  it("arms the wait at once, reads the table's id off the create's 201 past a listing and a join, and abandons it live", async () => {
    const live = await liveAt("live");
    const fake = creatingPage();
    const opening = live.awaitCreate(fake.page);
    expect(fake.armed(), "the wait is armed before awaitCreate returns, so before the click").toBe(true);
    fake.land(200, { games: [] }, "GET", `${LIVE_ORIGIN}/api/games?scope=open`);
    fake.land(200, {}, "POST", `${LIVE_ORIGIN}/api/games/sim-1/join`);
    fake.land(201, { id: "sim-00000000000000aa" });
    await expect(opening.id).resolves.toBe("sim-00000000000000aa");
    await live.abandonOpened(opening);
    expect(fake.deleted).toEqual([`${LIVE_ORIGIN}/api/games/sim-00000000000000aa`]);
  });

  // Mutant: a refusal's body read for an id, or a wait's failure thrown into
  // the afterEach.
  it("answers null for a refused create, a 201 naming no table, or a wait that ended without one — and abandons nothing then, nor for null, nor locally", async () => {
    const live = await liveAt("live");
    const refused = creatingPage();
    const refusedOpening = live.awaitCreate(refused.page);
    refused.land(400, { error: "no", id: "sim-00000000000000cc" });
    await expect(refusedOpening.id).resolves.toBeNull();
    await live.abandonOpened(refusedOpening);
    const unnamed = creatingPage();
    const unnamedOpening = live.awaitCreate(unnamed.page);
    unnamed.land(201, { id: 7 });
    await expect(unnamedOpening.id).resolves.toBeNull();
    await live.abandonOpened(unnamedOpening);
    // A create never sent ends as Playwright ends the wait: its timeout (the
    // config's actionTimeout), or the page closing.
    const ended = creatingPage();
    const endedOpening = live.awaitCreate(ended.page);
    ended.end("TimeoutError", 'Timeout 15000ms exceeded while waiting for event "response"');
    await expect(endedOpening.id).resolves.toBeNull();
    await live.abandonOpened(endedOpening);
    await expect(live.abandonOpened(null)).resolves.toBeUndefined();
    const local = await liveAt("");
    const here = creatingPage();
    const hereOpening = local.awaitCreate(here.page);
    here.land(201, { id: "sim-00000000000000bb" });
    await expect(hereOpening.id, "locally the id is read all the same").resolves.toBe("sim-00000000000000bb");
    await local.abandonOpened(hereOpening);
    expect([refused, unnamed, ended, here].flatMap((f) => f.deleted), "no DELETE was sent").toEqual([]);
  });

  // Task 7's review, I-1: a `null` id folds five causes into one, and the
  // walk's one sentence used to be "the server named no table" for all of
  // them — false for a wait that timed out, or a page that closed. The reason
  // is SAFE: a status, or the wait's error NAME, never its message, which
  // could carry what the request sent.
  // Mutant: every cause worded as a refusal, or the error's message printed.
  it("says why a create named no table — its status, or the wait's error NAME and never its message — and the walk's throw names it", async () => {
    const live = await liveAt("live");
    const reasonOf = async (act: (fake: ReturnType<typeof creatingPage>) => void) => {
      const fake = creatingPage();
      const opening = live.awaitCreate(fake.page);
      act(fake);
      const thrown = await live.createdTable(opening).then(
        (id) => `named ${id}`,
        (e: unknown) => (e as Error).message,
      );
      return { reason: await opening.reason, thrown };
    };
    const made = await reasonOf((f) => f.land(201, { id: "sim-00000000000000aa" }));
    expect(made).toEqual({ reason: null, thrown: "named sim-00000000000000aa" });
    // Access's answer when WARP's device login has expired (`docs/web-lane.md` §10; `worker/RUNBOOK.md` §7).
    const moved = await reasonOf((f) => f.land(302, null));
    expect(moved.reason).toBe("it answered 302, not 201");
    expect(moved.thrown).toBe("no table id came back from the create: it answered 302, not 201");
    const unnamed = await reasonOf((f) => f.land(201, { id: "" }));
    expect(unnamed.reason).toBe("it answered 201 naming no table");
    const unreadable = await reasonOf((f) => f.land(201, Object.assign(new Error("Unexpected token < in JSON"), { name: "SyntaxError" })));
    expect(unreadable.reason).toBe("its 201 could not be read (SyntaxError)");
    const secret = "cookie: CF_Authorization=DUMMY-SESSION-5678";
    const timedOut = await reasonOf((f) => f.end("TimeoutError", `Timeout 15000ms exceeded while waiting for event "response"\n${secret}`));
    expect(timedOut.reason).toContain("the wait for its reply timed out (TimeoutError)");
    expect(timedOut.reason, "a create answered after the wait is named as a table left behind").toContain("left in the lobby");
    expect(timedOut.thrown, "a timeout is not worded as a refusal").not.toMatch(/it answered \d/);
    const closed = await reasonOf((f) => f.end("Error", `Target page, context or browser has been closed\n${secret}`));
    expect(closed.reason).toBe("the wait for its reply ended in Error before any reply: the page closed, or crashed");
    for (const read of [timedOut, closed, unreadable]) {
      expect(`${read.reason} ${read.thrown}`, "no error message is printed").not.toMatch(/DUMMY-SESSION-5678|Timeout 15000ms|Target page|Unexpected token/);
    }
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
  // live. Read from its SOURCE, since loading it refuses where no Chromium is
  // installed.
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

  for (const live of [false, true]) {
    // R3's L-7: the forgiveness is exact in COUNT as well as in shape. A page
    // meets the designed 409 once — the deal flipped to playing between two
    // of its requests, and the screen goes to the board — so one response and
    // one console line are forgiven, and a second of either is breakage. The
    // cap is a PAGE's: another page's watch forgives its own one.
    // Mutant: no cap — a client looping on the 409 passes the walk.
    it(`forgives ONE designed 409 a page ${live ? "live" : "locally"}: a second, its response and its line, is breakage`, async () => {
      const { watchForBreakage, DEAL_ANSWERS_PER_PAGE } = await harnessAt(live);
      const base = baseUrlFor(live);
      const fake = fakePage();
      const watch = watchForBreakage(fake.page);
      fake.response(409, DEAL(base));
      fake.error(DEAL_LINE, DEAL(base));
      fake.response(409, DEAL(base));
      fake.error(DEAL_LINE, DEAL(base));
      expect(watch.dealAnswers, "one 409's two lines, and no more").toBe(2);
      expect(watch.failures).toEqual([`409 ${DEAL(base)}`, `console.error: ${DEAL_LINE}`]);

      const other = fakePage();
      const otherWatch = watchForBreakage(other.page);
      other.response(409, DEAL(base));
      other.error(DEAL_LINE, DEAL(base));
      expect(otherWatch.dealAnswers, "another page spends its own").toBe(2);
      expect(otherWatch.failures).toEqual([]);
      expect(DEAL_ANSWERS_PER_PAGE, "the cap this cell reads").toBe(1);
    });
  }
});

describe("capSentence — a walk that reached its cap says so (M6.5c2, spec §2.6)", () => {
  // Never `deadEnd`'s words: a cap hit is a game still RUNNING — the two
  // recorded hits were 4,001-move games with their end triggers unmet — and
  // "nothing on the screen is answerable" named a stuck table instead, which
  // sent the reader after the wrong defect. Mutant: the sentence reverting to
  // `deadEnd`'s. The strip is fed as the screen draws it — `.end-strip` renders
  // `view.end_triggers`, which begins with its own label — so a sentence that
  // labels it again says "end triggers: end triggers: …".
  const labels = (text: string) => text.split("end triggers:").length - 1;

  it("names the cap, the game, its turn and its end triggers, and no dead end", () => {
    const strip = "end triggers: deck 3 · church 3/4 · customs 0/4 · shophouses 1/8 · trade houses 0/4";
    const text = capSentence("sim-00000000000000aa", 4000, strip, "TURN 212 · SEAT 1");
    expect(text).toContain("reached its cap at 4000 moves");
    expect(text).toContain("sim-00000000000000aa");
    expect(text).toContain("TURN 212 · SEAT 1");
    expect(text).toContain(strip);
    expect(labels(text)).toBe(1);
    expect(text).not.toContain("nothing on the screen is answerable");
  });

  it("says what it could not read rather than printing nothing", () => {
    const text = capSentence("local-1", 4001, "", "");
    expect(text).toContain("no turn on screen");
    expect(text).toContain("no end-trigger strip drawn");
    expect(labels(text)).toBe(1);
  });
});

// ---- the request tally (cycle E, Task 7) ------------------------------------
//
// `tally` driven through a page that emits what the cell tells it to, into a
// scratch file, with BASE_URL the stand-in's local origin.

describe("tally — one line per request a browser makes", () => {
  let dir = "";
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "yokohama-tally-"));
  });
  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  /** A page emitting `request`, `response`, `requestfailed` and `websocket`
   *  for requests the cell makes. */
  function tallyPage() {
    const handlers = new Map<string, Handler[]>();
    const page = {
      on(event: string, handler: Handler) {
        handlers.set(event, [...(handlers.get(event) ?? []), handler]);
        return page;
      },
    };
    const emit = (event: string, arg: unknown) => (handlers.get(event) ?? []).forEach((h) => h(arg));
    const request = (url: string, method = "GET") => {
      const r = { url: () => url, method: () => method };
      emit("request", r);
      return {
        answer: (status: number) => emit("response", { status: () => status, request: () => r }),
        fail: () => emit("requestfailed", r),
      };
    };
    return { page: page as unknown as Page, request, socket: (url: string) => emit("websocket", { url: () => url }) };
  }

  const read = (path: string) =>
    readFileSync(path, "utf8")
      .split("\n")
      .filter((l) => l !== "")
      .map((l) => JSON.parse(l) as { t: number; page: string; method: string; route: string; status: number });

  it("names the route by the report's regex, drops the query, and writes the page, method and status", () => {
    const path = join(dir, "t.jsonl");
    const origin = baseUrlFor(false);
    const fake = tallyPage();
    tally(fake.page, "watcher", path);
    fake.request(`${origin}/api/games/sim-abc/join?x=1`, "POST").answer(409);
    fake.request(`${origin}/api/games`).answer(200);
    const lines = read(path);
    expect(lines.map(({ page, method, route, status }) => ({ page, method, route, status }))).toEqual([
      { page: "watcher", method: "POST", route: "/api/games/<id>/join", status: 409 },
      { page: "watcher", method: "GET", route: "/api/games", status: 200 },
    ]);
    expect(lines.every((l) => typeof l.t === "number" && l.t >= 0)).toBe(true);
  });

  it("writes a request once, a failed one as status 0, a socket as 101, and an off-origin one with its origin", () => {
    const path = join(dir, "t.jsonl");
    const origin = baseUrlFor(false);
    const fake = tallyPage();
    tally(fake.page, "a", path);
    const stream = fake.request(`${origin}/api/games/sim-abc/stream`);
    stream.answer(200);
    stream.fail(); // aborted after its headers: still ONE request
    fake.request(`${origin}/api/games/sim-abc/events`).fail();
    fake.socket(`${origin.replace(/^http/, "ws")}/api/games/sim-abc/stream`);
    fake.request("https://elsewhere.example/api/games/x/join").answer(200);
    fake.request("data:image/png;base64,AAAA").answer(200);
    expect(read(path).map(({ method, route, status }) => [method, route, status])).toEqual([
      ["GET", "/api/games/<id>/stream", 200],
      ["GET", "/api/games/<id>/events", 0],
      ["GET", "/api/games/<id>/stream", 101],
      ["GET", "https://elsewhere.example/api/games/<id>/join", 200],
    ]);
  });

  it("truncates the file at its first tally and appends every page after, on one clock", () => {
    const path = join(dir, "t.jsonl");
    writeFileSync(path, '{"left":"by an earlier run"}\n');
    const origin = baseUrlFor(false);
    const a = tallyPage();
    const b = tallyPage();
    tally(a.page, "a", path);
    tally(b.page, "b", path);
    a.request(`${origin}/api/me`).answer(200);
    b.request(`${origin}/api/me`).answer(200);
    expect(read(path).map((l) => l.page)).toEqual(["a", "b"]);
  });

  it("installs nothing when no file is named", () => {
    vi.stubEnv("YOKOHAMA_WEB_TALLY", "");
    try {
      const fake = tallyPage();
      const on = vi.spyOn(fake.page, "on");
      tally(fake.page, "a");
      tally(fake.page, "a", "");
      expect(on).not.toHaveBeenCalled();
    } finally {
      vi.unstubAllEnvs();
    }
  });
});

// ---- the CSP lines, as Chromium 143–153 words and levels them (MEASURED, the S2a plan review) ---------------

/** A page whose console emits a line of the type the cell chooses — `watchForBreakage` reads the type, so the
 *  `fakePage` above, which emits `error` alone, cannot drive a CSP line at `info`. */
function consolePage() {
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
    line: (type: string, text: string, url: string) =>
      emit("console", { type: () => type, text: () => text, location: () => ({ url, lineNumber: 0, columnNumber: 0 }) } as unknown as ConsoleMessage),
  };
}

/** Chromium's Report-Only refusal of an inline script under 0.6.2's `script-src` (the shipped one carries the nonce
 *  since 0.6.3), word for word (the hash is the injected script's own, and varies). */
const RO_INLINE =
  "Executing inline script violates the following Content Security Policy directive 'script-src 'self' 'wasm-unsafe-eval' https://challenges.cloudflare.com https://static.cloudflareinsights.com'. Either the 'unsafe-inline' keyword, a hash ('sha256-aBV9DC+smatNUlnkSmZOl+jfLQV1VY8nHmiO1eoMzIU='), or a nonce ('nonce-...') is required to enable inline execution. The policy is report-only, so the violation has been logged but no further action has been taken.";
/** The same refusal of an inline STYLE — another directive's line. */
const RO_STYLE = RO_INLINE.replace("Executing inline script", "Applying inline style").replace("'script-src ", "'style-src ");
/** The ENFORCED refusal: Chromium's last sentence changes, and the line is an `error`. */
const ENFORCED_INLINE = RO_INLINE.replace(
  "The policy is report-only, so the violation has been logged but no further action has been taken.",
  "The action has been blocked.",
);

describe("the CSP lines (any console type) are read on every tier, and nothing is forgiven (security S2a, decision D-9; S2c)", () => {
  afterEach(() => {
    vi.doUnmock("./live");
    vi.resetModules();
  });
  it("locally every CSP line is breakage, whatever its console type", async () => {
    const { watchForBreakage } = await harnessAt(false);
    const fake = consolePage();
    const watch = watchForBreakage(fake.page);
    const local = baseUrlFor(false);
    fake.line("info", RO_INLINE, `${local}/`);
    expect(watch.failures[0], "the label is the message's own type").toBe(`console.info: ${RO_INLINE}`);
    fake.line("info", RO_STYLE, `${local}/`);
    expect(watch.failures).toHaveLength(2);
    // The wording with `report-only` before the policy's name (the regex's optional clause), at `info`: read too.
    fake.line("info", RO_INLINE.replace("following Content Security Policy", "following report-only Content Security Policy"), `${local}/`);
    expect(watch.failures, "the report-only wording, read at info").toHaveLength(3);
  });
  it("locally a CSP line is breakage even when a cell's expected status digits appear in its hash — the status filter never swallows it", async () => {
    const { watchForBreakage } = await harnessAt(false);
    const fake = consolePage();
    const watch = watchForBreakage(fake.page, [{ status: 409, urlIncludes: "/result" }]);
    fake.line("error", ENFORCED_INLINE.replace("aBV9DC+", "a409DC+"), `${baseUrlFor(false)}/`);
    expect(watch.failures, "an enforced refusal whose hash spells 409").toHaveLength(1);
  });
  it("live forgives NO CSP line, whatever its level or its page (security S2c: the enforced policy carries the nonce)", async () => {
    const { watchForBreakage } = await harnessAt(true);
    const fake = consolePage();
    const watch = watchForBreakage(fake.page);
    fake.line("info", RO_INLINE, `${LIVE_ORIGIN}/`);
    fake.line("info", RO_STYLE, `${LIVE_ORIGIN}/`);
    fake.line("error", ENFORCED_INLINE, `${LIVE_ORIGIN}/`);
    fake.line("info", RO_INLINE, "https://elsewhere.test/");
    expect([watch.failures.length, watch.forgiven, watch.feedForgiven]).toEqual([4, 0, 0]);
    fake.line("info", "an ordinary info line", `${LIVE_ORIGIN}/`);
    expect(watch.failures, "an ordinary info line is still read as nothing").toHaveLength(4);
  });
});
