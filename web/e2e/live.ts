// **The live tier: two specs against the deployed Worker** (M6.5c, spec §5.4;
// R-6.5c.12; the orchestrator's ruling I-75).
//
//   cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link
//
// `two_browsers.spec.ts` and `joins_by_link.spec.ts` play a table at
// `https://yokohama.moore.nyc` when `YOKOHAMA_WEB_E2E_TARGET=live`; every other
// run is the local walk, unchanged. `playwright.config.ts` reads `LIVE` for its
// origin, its specs, its server and its trace; the two specs for their budget,
// their cleanup, their summary line and the one reading the edge's door
// changes; and every context they open asks `liveContextOptions` who it is.
// It is the ORCHESTRATOR's run, never a gate's: it reads the Service Auth
// credential and plays on the owner's account.
//
// **How a live context is somebody** (I-75). Access admits the credential —
// the two `CF-Access-Client-*` headers — and sets a `CF_Authorization` cookie
// on its answer; with that cookie a request is the credential's, and the dev
// door then names its caller by `X-Yokohama-Claimed-User`. So a live context
// carries two things:
//
//   - the COOKIE, seeded into its storage. It is origin-scoped by the browser,
//     so the credential it stands for never reaches another host — the Web
//     Analytics beacon's among them;
//   - the claimed header, as the context's `extraHTTPHeaders`: the one header
//     the page's own socket needs and cannot set on its upgrade (§0.4 measured
//     context headers reaching it; I-75 records that request routing does
//     not). It is not a secret, and it is also why the beacon's CORS error
//     recurs live — `harness.ts`'s `isBeaconCorsError` is that one error.
//
// **The credential never reaches Playwright.** It is presented once per
// worker process, which is once per green run — Playwright restarts the
// worker after a failing test, and `seeded` goes with it — by Node's own
// `fetch`, and only the cookie Access answers with goes on. MEASURED
// (Playwright 1.62.1, a local server): a trace records an
// `APIRequestContext`'s `extraHTTPHeaders` and a page's cookies alike, so a
// credential handed to Playwright would be written into any trace a failure
// kept — and the config turns tracing off under `LIVE` for the cookie's sake.
//
// **And the COOKIE stays out of every error** (the review's H-1). MEASURED
// (Playwright 1.62.1): a THROWN API request's message carries a "Call log"
// listing every header it sent, the context's cookie among them, and
// Playwright writes a failing test's error to the reporter and to
// `test-results/<test>/error-context.md` whatever `trace` reads. So every live
// `request.*` call goes through [`liveRequest`]. And never
// `expect(res).toBeOK()` on a live context's response: its failure message
// prints the call log too — `expect(res.ok())` is the form to use.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { BrowserContextOptions, Page } from "@playwright/test";

/** The deployed Worker's origin: `BASE_URL` under `LIVE`. */
export const LIVE_ORIGIN = "https://yokohama.moore.nyc";

/** The local binary's port — `playwright.config.ts` says why it is its own. */
export const LOCAL_PORT = 8479;

/** Whether this run targets the Worker. */
export const LIVE = process.env.YOKOHAMA_WEB_E2E_TARGET === "live";

/**
 * The origin a walk plays at: the Worker's live, the local binary's
 * otherwise. `playwright.config.ts` exports it AS `BASE_URL` — the constant
 * `watchForBreakage` and `two_browsers.spec.ts` read — and a cell reads the
 * config's source for that, so a constant local origin cannot come back
 * unnoticed (plan review M9).
 */
export function baseUrlFor(live: boolean): string {
  return live ? LIVE_ORIGIN : `http://127.0.0.1:${LOCAL_PORT}`;
}

/** The repository root's `.dev.vars`, which holds the credential (the plan's
 *  Global Constraints: two `KEY=VALUE` lines, named as the headers). */
export const DEV_VARS = fileURLToPath(new URL("../../.dev.vars", import.meta.url));

const KEYS = ["CF-Access-Client-Id", "CF-Access-Client-Secret"] as const;

/** A context's storage state, as Playwright seeds one. */
type StorageState = Exclude<BrowserContextOptions["storageState"], string | undefined>;
type Cookie = StorageState["cookies"][number];

/**
 * The credential in `file`: its `CF-Access-Client-Id=` and
 * `CF-Access-Client-Secret=` lines, among any others. A file or a key that is
 * absent — or a value that is empty — is refused by NAME: the file and the
 * key, never a value.
 */
export function credential(file: string = DEV_VARS): { id: string; secret: string } {
  let text: string;
  try {
    text = readFileSync(file, "utf8");
  } catch {
    throw new Error(`the credential file ${file} is absent or unreadable — it holds the ${KEYS[0]}= and ${KEYS[1]}= lines (RUNBOOK §6.4)`);
  }
  const values = new Map<string, string>();
  for (const line of text.split(/\r?\n/)) {
    const eq = line.indexOf("=");
    if (eq !== -1) values.set(line.slice(0, eq).trim(), line.slice(eq + 1).trim());
  }
  const read = (key: (typeof KEYS)[number]) => {
    const value = values.get(key);
    if (!value) throw new Error(`${file} holds no ${key}= line — the credential is its ${KEYS[0]}= and ${KEYS[1]}= lines (RUNBOOK §6.4)`);
    return value;
  };
  return { id: read(KEYS[0]), secret: read(KEYS[1]) };
}

/** Who a context is, to the dev door: the claimed header, in both modes. */
export function identityHeaders(email: string): Record<string, string> {
  return { "X-Yokohama-Claimed-User": email };
}

/**
 * The cookies a `Set-Cookie` answer to `url` stores, as a browser stores them
 * (RFC 6265 §5.2–5.3, the attributes Access and the zone use): a host-only
 * cookie on `url`'s host, or a `Domain` one dotted — and none at all for a
 * `Domain` the host is not within; `Path`, or the request path's directory;
 * `Max-Age` over `Expires`, and a session cookie (`-1`) with neither;
 * `SameSite` as given, else `Lax`. It prints nothing itself; the value is the
 * credential's session, and what would otherwise print it is a thrown API
 * call's call log — which [`liveRequest`] withholds.
 */
export function cookiesFrom(headers: string[], url: string, now: number = Date.now()): Cookie[] {
  const at = new URL(url);
  const directory = at.pathname.lastIndexOf("/") > 0 ? at.pathname.slice(0, at.pathname.lastIndexOf("/")) : "/";
  const cookies: Cookie[] = [];
  for (const header of headers) {
    const [pair = "", ...attributes] = header.split(";");
    const eq = pair.indexOf("=");
    if (eq <= 0) continue;
    const cookie: Cookie = {
      name: pair.slice(0, eq).trim(),
      value: pair.slice(eq + 1).trim(),
      domain: at.hostname,
      path: directory,
      expires: -1,
      httpOnly: false,
      secure: false,
      sameSite: "Lax",
    };
    let maxAge: number | undefined;
    for (const attribute of attributes) {
      const i = attribute.indexOf("=");
      const key = (i === -1 ? attribute : attribute.slice(0, i)).trim().toLowerCase();
      const value = i === -1 ? "" : attribute.slice(i + 1).trim();
      if (key === "domain" && value !== "") cookie.domain = `.${value.replace(/^\./, "").toLowerCase()}`;
      else if (key === "path" && value.startsWith("/")) cookie.path = value;
      else if (key === "expires" && !Number.isNaN(Date.parse(value))) cookie.expires = Math.floor(Date.parse(value) / 1000);
      else if (key === "max-age" && /^-?[0-9]+$/.test(value)) maxAge = Number(value);
      else if (key === "httponly") cookie.httpOnly = true;
      else if (key === "secure") cookie.secure = true;
      else if (key === "samesite") cookie.sameSite = value.toLowerCase() === "strict" ? "Strict" : value.toLowerCase() === "none" ? "None" : "Lax";
    }
    if (maxAge !== undefined) cookie.expires = maxAge <= 0 ? 0 : Math.floor(now / 1000) + maxAge;
    const within = cookie.domain === at.hostname || `.${at.hostname}`.endsWith(cookie.domain);
    if (within) cookies.push(cookie);
  }
  return cookies;
}

/**
 * The credential, presented ONCE — `GET /api/health` on the Worker's origin,
 * following no redirect — and the storage Access's answer seeds: every cookie
 * it set, `CF_Authorization` among them. Refuses an answer that is not 200 and
 * one that set no `CF_Authorization`; neither sentence carries a value.
 *
 * A refusal says WHAT came back and asserts no cause, because several read
 * alike: the zone's Bot Fight Mode answers a bot-shaped request 403 before
 * Access does, and a Worker 5xx is no refusal of the credential. It prints the
 * body's type and size — never the body — and a redirect without its address,
 * which Access's carries a token in.
 */
export async function accessState(cred: { id: string; secret: string }, fetcher: typeof fetch = fetch): Promise<StorageState> {
  const url = `${LIVE_ORIGIN}/api/health`;
  const res = await fetcher(url, { headers: { [KEYS[0]]: cred.id, [KEYS[1]]: cred.secret }, redirect: "manual" });
  const bytes = (await res.arrayBuffer()).byteLength;
  if (res.status !== 200) {
    const what =
      res.status >= 300 && res.status < 400
        ? "a redirect (its location is not printed: Access's carries a token)"
        : `a ${res.headers.get("content-type") ?? "typeless"} body of ${bytes} B`;
    throw new Error(`the credential's one request, GET ${url} answered ${res.status} and not 200 — ${what}`);
  }
  const cookies = cookiesFrom(res.headers.getSetCookie(), url);
  if (!cookies.some((c) => c.name === "CF_Authorization")) {
    throw new Error(`Access answered GET ${url} but set no CF_Authorization cookie — without it every page load would meet Access's login`);
  }
  return { cookies, origins: [] };
}

/** The seeded storage: one presentation of the credential per worker
 *  process, which is once per green run — Playwright restarts the worker
 *  after a failing test, and this goes with it. */
let seeded: Promise<StorageState> | undefined;

/**
 * What a spec hands `newContextWithoutWorker` for `email`'s context.
 *
 * Locally it is NOTHING — exactly today's context. The local server's door is
 * `claimed`: the walk types the address into the sign-in form, and the client
 * keeps it and attaches the claimed header to its own requests.
 *
 * Under `LIVE` it is the seeded cookie and, context-wide, the claimed header —
 * because neither half of the local arrangement holds there. The Worker's
 * door is `cf-access`, where the client draws no sign-in form (`Door.tsx`
 * draws one under `claimed` only), so it never holds an address and attaches
 * no claimed header of its own; and the header must reach the socket the page
 * opens, whose upgrade only a context header reaches (§0.4; I-75).
 */
export async function liveContextOptions(email: string): Promise<BrowserContextOptions> {
  if (!LIVE) return {};
  seeded ??= (async () => accessState(credential()))();
  return { storageState: await seeded, extraHTTPHeaders: identityHeaders(email) };
}

/**
 * Abandon `id` as its creator, through the creator's own page (spec §5.4):
 * the lobby is SHARED live, so a spec takes away what it opened. The game's
 * save is the Worker's, under the bucket's `test/` root. Refuses an answer
 * that did not abandon it, naming the table and the operator's force, which
 * ends it whatever it holds.
 */
export async function abandonAsCreator(page: Page, id: string): Promise<void> {
  const res = await liveRequest(() => page.request.delete(`${LIVE_ORIGIN}/api/games/${encodeURIComponent(id)}`, { maxRedirects: 0 }));
  if (!res.ok()) {
    // The force goes WITHOUT the credential, over WARP (R-6.5c.16): the
    // credential is never an operator, and would earn the gate's 403.
    throw new Error(
      `the live table ${id} was not abandoned: DELETE answered ${res.status()} — force it as the operator, over WARP, ` +
        `with no credential: POST ${LIVE_ORIGIN}/api/admin/tables/${id}/expire?force`,
    );
  }
}

/**
 * **Every live `request.*` call, made through this** (the review's H-1;
 * ruling I-79): `call`'s answer, or — when it THROWS under `LIVE` — a fresh
 * `Error` whose message is the thrown message's FIRST line alone, the one
 * before its "Call log", which lists every header the request sent, the
 * context's `CF_Authorization` cookie among them. No `cause` is attached, since
 * a reporter prints a cause's message too.
 *
 * Locally the call's own error goes through untouched: a local context holds
 * no such cookie, and nothing about a local run changes.
 */
export async function liveRequest<T>(call: () => Promise<T>): Promise<T> {
  try {
    return await call();
  } catch (e) {
    if (!LIVE) throw e;
    const first = (e instanceof Error ? e.message : String(e)).split("\n", 1)[0];
    throw new Error(`${first} (the call log is withheld: it lists the live context's cookie)`);
  }
}
