// **The live tier: two specs against the deployed Worker** (M6.5c, spec §5.4;
// R-6.5c.12; the orchestrator's ruling I-75) — and a third, the exit's headed
// game, run only when asked for by name (ruling I-88; [`EXIT_SPEC`]).
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

/** The specs a live run plays (spec §5.4): the two multiplayer walks. ONE
 *  list, read by `playwright.config.ts` for what a live run matches and by
 *  `live.node.test.ts` for which specs the request rule below reaches — so a
 *  spec added here is a spec that rule checks. */
export const LIVE_SPECS = ["two_browsers.spec.ts", "joins_by_link.spec.ts"] as const;

/** The exit's headed game (M6.5c Task 16; ruling I-88): a live spec as well,
 *  but one that runs only when asked for by name — [`EXIT`] beside the live
 *  target — and is kept out of every other run, `npm run e2e` and a plain live
 *  run among them (`playwright.config.ts`). */
export const EXIT_SPEC = "exit_game.spec.ts";

/** Whether the exit's game was asked for: `YOKOHAMA_WEB_EXIT=1`. */
export const EXIT = process.env.YOKOHAMA_WEB_EXIT === "1";

/**
 * The two Chromium switches the exit's run launches with (R3's R-N6; ruling
 * I-91), meant to keep its headed windows VISIBLE when one covers another: a
 * covered window that Chromium backgrounded would read `hidden` and let its
 * socket go (R-P8). Playwright 1.62.1 already passes both to every Chromium it
 * launches (its `chromiumSwitches`); `playwright.config.ts` names them for the
 * exit's run all the same, so its precondition does not rest on a default an
 * upgrade could drop. **On macOS they are not enough**: on the exit's first
 * headed run (deployment 19) the three windows stacked and the covered one
 * read hidden at move 5 — which is why the spec also lays its windows out so
 * that none covers another ([`exitLayout`], [`placeWindow`], [`mustBeApart`]).
 * The spec checks the precondition itself ([`mustBeVisible`]).
 */
export const EXIT_LAUNCH_ARGS = ["--disable-backgrounding-occluded-windows", "--disable-renderer-backgrounding"] as const;

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
 * `open`: the walk types the address into the sign-in form, and the client
 * keeps it and attaches the claimed header to its own requests.
 *
 * Under `LIVE` it is the seeded cookie and, context-wide, the claimed header —
 * because neither half of the local arrangement holds there. The Worker's
 * door is `cf-access`, where the client draws no sign-in form (`Door.tsx`
 * draws one under `open` only), so it never holds an address and attaches
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

// ---- a live table, known from the moment its create is SENT (R3's L-9) ----

/**
 * **A table a walk is opening**: known from the moment its create is SENT
 * (R3's L-9), for as long as [`awaitCreate`]'s wait lasts. A walk used to arm
 * its own wait for the create's reply and set its `opened` only once it had
 * READ that reply, so a failure between the click and that read — a budget run
 * out, a reply it never parsed — left the table in the shared lobby. Its
 * `afterEach` hands this to [`abandonOpened`] instead.
 */
export type Opening = {
  /** The creator's page, whose context the abandon goes through: the DELETE
   *  must carry that context's cookie and claimed header ([`abandonAsCreator`];
   *  ruling R-C19). */
  page: Page;
  /** The table's id, read off the create's 201 the moment it lands, whether
   *  or not the walk gets that far — or `null` for a create that answered
   *  anything else, or a wait that ended with no reply at all. It never
   *  rejects. */
  id: Promise<string | null>;
  /** Why `id` is `null`, or `null` when it is not (Task 7's review, I-1): a
   *  SAFE line — the status the create answered, or the NAME of the error
   *  the wait ended in, never that error's message, which could carry what
   *  the request sent. [`createdTable`] throws it. It never rejects. */
  reason: Promise<string | null>;
};

/** The one line for a wait that ended in `e` before any reply: its error's
 *  NAME alone. In Playwright 1.62.1 the wait ends three ways — its timeout, a
 *  `TimeoutError`; the page closing, and the page crashing, both named plain
 *  `Error` — so a name other than `TimeoutError` is one of those two. */
function waitEnded(e: unknown): string {
  const name = e instanceof Error ? e.name : typeof e;
  return name === "TimeoutError"
    ? "the wait for its reply timed out (TimeoutError): no reply within the context's default timeout, the config's " +
        "actionTimeout — the create was never sent, or was answered later, and a table answered later is left in the lobby"
    : `the wait for its reply ended in ${name} before any reply: the page closed, or crashed`;
}

/**
 * Arms the wait for `page`'s create — `POST /api/games` — and answers its
 * [`Opening`] at once. A walk calls it BEFORE the click that sends the create
 * (`live.node.test.ts` holds every live spec to that), so the reply cannot
 * land before the wait is armed.
 *
 * The wait is `page.waitForResponse`'s, which ends on its own: when the page
 * closes or crashes, or when its timeout passes — the context's default,
 * which Playwright 1.62.1 sets from the config's `actionTimeout` (15 s) on
 * every context a run opens, a spec's hand-made ones included. So a create
 * that was never sent reads `null` at the latest 15 s after the wait was
 * armed, and an `afterEach` awaiting it never hangs. Every end is caught into
 * `null` and a reason: neither promise rejects.
 *
 * **A create that WAS sent and answered after that wait reads `null` as
 * well** (Task 7's review, M-4): the server made the table, and nothing here
 * abandons it. Its reason names `TimeoutError` and says so; the table is
 * taken away by the operator's force, which [`abandonAsCreator`]'s refusal
 * names.
 */
export function awaitCreate(page: Page): Opening {
  const settled: Promise<{ id: string | null; reason: string | null }> = page
    .waitForResponse((r) => r.request().method() === "POST" && new URL(r.url()).pathname === "/api/games")
    .then(async (r) => {
      if (r.status() !== 201) return { id: null, reason: `it answered ${r.status()}, not 201` };
      let body: unknown;
      try {
        body = await r.json();
      } catch (e) {
        return { id: null, reason: `its 201 could not be read (${e instanceof Error ? e.name : typeof e})` };
      }
      const named = (body as { id?: unknown } | null)?.id;
      return typeof named === "string" && named !== ""
        ? { id: named, reason: null }
        : { id: null, reason: "it answered 201 naming no table" };
    })
    .catch((e: unknown) => ({ id: null, reason: waitEnded(e) }));
  return { page, id: settled.then((s) => s.id), reason: settled.then((s) => s.reason) };
}

/**
 * The table `opening` created, for the walk that goes on to play it: its id,
 * or a throw that says why there is none — `opening.reason`, which names a
 * status or an error's NAME and never a message (Task 7's review, I-1: the
 * one sentence the three live specs used to throw, "the server named no
 * table", was false for a wait that timed out and for a page that closed).
 */
export async function createdTable(opening: Opening): Promise<string> {
  const id = await opening.id;
  if (id !== null) return id;
  throw new Error(`no table id came back from the create: ${await opening.reason}`);
}

/**
 * Every live spec's `afterEach` for the table it opened (spec §5.4; R3's
 * L-9): the table's id awaited — whether or not the walk read the create's
 * reply — and the table abandoned as its creator, through the creator's own
 * page ([`abandonAsCreator`]), where there is one. Nothing for `null`, for a
 * create that named no table, or locally: the local server's games are
 * scratch (`two_browsers.spec.ts`'s `afterEach` says why).
 */
export async function abandonOpened(opening: Opening | null): Promise<void> {
  if (!LIVE || opening === null) return;
  const id = await opening.id;
  if (id !== null) await abandonAsCreator(opening.page, id);
}

// ---- the exit's precondition: its windows stay visible (R3's R-N6) --------

/** The binding a watched page reports its visibility through. */
const VISIBILITY_BINDING = "__yokohamaExitVisibility";

/** What a page has reported of its own visibility since [`watchVisibility`]
 *  was called on it: how many reports, and each report of a state other than
 *  `visible`, as `<state> at <UTC time>`. */
export interface VisibilityWatch {
  reports: number;
  hidden: string[];
}

/**
 * Starts recording `page`'s visibility — across its navigations, and from
 * before its first — through a binding the page calls and a script every
 * document it loads runs before its own ([`reportVisibility`]).
 */
export async function watchVisibility(page: Page, now: () => Date = () => new Date()): Promise<VisibilityWatch> {
  const watch: VisibilityWatch = { reports: 0, hidden: [] };
  await page.exposeFunction(VISIBILITY_BINDING, (state: string) => {
    watch.reports += 1;
    if (state !== "visible") watch.hidden.push(`${state} at ${now().toISOString().slice(11, 19)}Z`);
  });
  await page.addInitScript(reportVisibility, VISIBILITY_BINDING);
  return watch;
}

/**
 * The page's half of [`watchVisibility`]: the document's `visibilityState`,
 * reported through the binding named `binding` as the document starts and
 * again at every change. It runs IN the page — Playwright sends its source —
 * so it closes over nothing but its argument.
 */
export function reportVisibility(binding: string): void {
  const report = () => {
    const send = (window as unknown as Record<string, unknown>)[binding];
    if (typeof send === "function") send(document.visibilityState);
  };
  report();
  document.addEventListener("visibilitychange", report);
}

/**
 * **The exit's own precondition**, read at a checkpoint (R3's R-N6; ruling
 * I-91): every page's `document.visibilityState` reads `visible` now, and the
 * page has reported no other state since it was watched — which a read of the
 * state alone would miss: a window hidden and shown again between two
 * checkpoints. A hidden page lets its table's socket go by design (R-P8) and
 * opens another once it is visible again, so a window minimized, or left on
 * another Space, fails the exit's one-socket claims for a reason that is not
 * the product's. This throws at the first page that breaks the precondition,
 * saying so — or that has reported nothing at all, since then the watch is
 * not running and the precondition cannot be read.
 */
export async function mustBeVisible(
  pages: ReadonlyArray<readonly [who: string, page: Page, watch: VisibilityWatch]>,
  where: string,
): Promise<void> {
  for (const [who, page, watch] of pages) {
    if (watch.reports === 0) {
      throw new Error(
        `the exit's own precondition cannot be read ${where}: ${who}'s page has reported no visibility at all, ` +
          "so this run cannot tell whether its window was hidden",
      );
    }
    const now = await page.evaluate(() => document.visibilityState);
    const seen = [...(now === "visible" ? [] : [`${now} now`]), ...watch.hidden];
    if (seen.length > 0) {
      throw new Error(
        `the exit's own precondition failed ${where}: ${who}'s window was not visible (${seen.join(", ")}). ` +
          "A hidden page lets its table's socket go, by design (R-P8), and opens another once it is visible again, " +
          "so this run can say nothing about the sockets surviving a hibernation — it is not a finding about the " +
          "product. Keep all three windows visible, none minimized and none on another Space, and run it again.",
      );
    }
  }
}

// ---- the exit's windows, none covering another (exit game runs 1 and 2) --
//
// MEASURED on the exit's first headed run (deployment 19): the three windows
// opened STACKED over one another, and macOS's occlusion marked the covered
// ones hidden whatever the two launch switches read — x's before the pause
// after move 5, which the precondition above reported as it should. On the
// second, three columns of 420 px were asked for, and Chromium widened every
// window to its narrowest, 500 ([`CHROMIUM_MIN_WINDOW_WIDTH`]) — so they
// overlapped by 74 px, the last ran past the screen's edge — and the window
// manager kept every window below the menu bar, at top 38. So the layout is
// chosen by what Chromium will draw, under the top inset the window manager
// keeps; the placement is read back and refused if two windows overlap or
// one leaves the screen ([`mustBeApart`]); and what a window reported hidden
// while the three were opening and being placed is forgotten once the table
// is seated ([`forgetHidden`]).

/** A window's place on the screen, in the screen's logical pixels. */
export interface WindowBounds {
  left: number;
  top: number;
  width: number;
  height: number;
}

/** Where the exit's three windows go — x, y, w — the top inset they were laid
 *  out under, and the viewport each context emulates inside its window. */
export interface ExitLayout {
  /** `columns`: three side by side. `grid`: x and y side by side on the top
   *  row, w below x on the bottom row. */
  kind: "columns" | "grid";
  screen: { width: number; height: number };
  inset: number;
  windows: [WindowBounds, WindowBounds, WindowBounds];
  viewport: { width: number; height: number };
}

/** The screen the exit lays its windows out on when `EXIT_SCREEN` says
 *  nothing: the orchestrator's Mac, "2560 x 1664 Retina", in logical pixels. */
export const DEFAULT_EXIT_SCREEN = "1280x832";

/** Chromium's narrowest window, in logical pixels: a window asked to be
 *  narrower is drawn this wide (the exit's second headed run asked 420 and
 *  read back 500). */
export const CHROMIUM_MIN_WINDOW_WIDTH = 500;

/** The space kept between two windows, and at the screen's edge below them. */
const GUTTER = 6;

/** A window's own frame and toolbar, which its viewport is laid out inside. */
const CHROME = { width: 16, height: 90 };

/** The smallest viewport a window may leave: 320 wide is the narrowest the
 *  client is laid out for (its width gate reads 320 to 2560); 280 tall, since
 *  two rows of windows on a laptop's screen, under its menu bar, leave about
 *  300 (1280x832 under 38 px: 301). */
const MIN_VIEWPORT = { width: 320, height: 280 };

/**
 * The exit's three windows on the screen `screen` names (`EXIT_SCREEN`,
 * `<width>x<height>` in logical pixels; unset or empty is the default), laid
 * out under a top inset of `inset` px — a menu bar the window manager keeps
 * windows below, read off the first window placed.
 *
 * - **Three columns** where three of Chromium's narrowest windows and their
 *   gutters fit across: a third of the width apart, each a gutter narrower,
 *   and as tall as the screen under the inset, less a gutter above and below.
 * - **Else a grid**: x and y side by side on the top row, each half the width
 *   less a gutter; w on the bottom row, under x; each row half the height
 *   under the inset, less a gutter.
 *
 * Each context's viewport is its window less 16 px across and 90 px down —
 * room for the window's own frame and toolbar — because Playwright emulates a
 * viewport whatever the window's size, so the two must agree or the page is
 * clipped. Refused by NAME: a value that is not `<width>x<height>`, a screen
 * that cannot hold two of Chromium's windows side by side, and one that
 * leaves a viewport under 320 by 280.
 */
export function exitLayout(screen: string | undefined, inset: number): ExitLayout {
  const raw = screen === undefined || screen === "" ? DEFAULT_EXIT_SCREEN : screen;
  const read = /^([1-9][0-9]*)x([1-9][0-9]*)$/.exec(raw);
  if (read === null) {
    throw new Error(
      `EXIT_SCREEN must be the screen's logical size as <width>x<height>, e.g. ${DEFAULT_EXIT_SCREEN} — got ${JSON.stringify(raw)}`,
    );
  }
  const width = Number(read[1]);
  const height = Number(read[2]);
  const usable = height - inset;
  const third = Math.floor(width / 3);
  const half = Math.floor(width / 2);
  let kind: ExitLayout["kind"];
  let windows: ExitLayout["windows"];
  if (third - GUTTER >= CHROMIUM_MIN_WINDOW_WIDTH) {
    kind = "columns";
    const column = (i: number): WindowBounds => ({
      left: i * third,
      top: inset,
      width: third - GUTTER,
      height: usable - 2 * GUTTER,
    });
    windows = [column(0), column(1), column(2)];
  } else if (half - GUTTER >= CHROMIUM_MIN_WINDOW_WIDTH) {
    kind = "grid";
    const row = Math.floor(usable / 2);
    const cell = (col: number, line: number): WindowBounds => ({
      left: col * half,
      top: inset + line * row,
      width: half - GUTTER,
      height: row - GUTTER,
    });
    windows = [cell(0, 0), cell(1, 0), cell(0, 1)];
  } else {
    throw new Error(
      `EXIT_SCREEN ${raw} cannot hold two of Chromium's narrowest windows, ${CHROMIUM_MIN_WINDOW_WIDTH} px each, side by side — ` +
        `give the screen's logical size (the default is ${DEFAULT_EXIT_SCREEN})`,
    );
  }
  const viewport = { width: windows[0].width - CHROME.width, height: windows[0].height - CHROME.height };
  if (viewport.width < MIN_VIEWPORT.width || viewport.height < MIN_VIEWPORT.height) {
    throw new Error(
      `EXIT_SCREEN ${raw}${inset > 0 ? ` under a ${inset} px top inset` : ""} leaves each window a ${viewport.width} by ` +
        `${viewport.height} viewport, and the windows need at least ${MIN_VIEWPORT.width} by ${MIN_VIEWPORT.height} — ` +
        `give the screen's logical size (the default is ${DEFAULT_EXIT_SCREEN})`,
    );
  }
  return { kind, screen: { width, height }, inset, windows, viewport };
}

/** A window's bounds as the exit's lines print them: `<w>x<h> at <left>,<top>`,
 *  with `?` for what was not reported. */
export function describeBounds(b: Partial<WindowBounds>): string {
  return `${b.width ?? "?"}x${b.height ?? "?"} at ${b.left ?? "?"},${b.top ?? "?"}`;
}

/**
 * **The exit's own precondition, at placement** (the second headed run): every
 * window, where it was READ BACK, lies on the screen and covers no part of
 * another — edges that touch are not an overlap. What was asked is not what
 * this reads: Chromium widens a window narrower than it draws, and a window
 * manager moves one clear of its menu bar. A window another covers can read
 * hidden, and a hidden page lets its table's socket go, so this throws —
 * naming every window that leaves the screen and every pair that overlaps —
 * saying it is not a finding about the product. A window that reported no
 * bounds cannot be read at all, and is refused as that.
 */
export function mustBeApart(
  placed: ReadonlyArray<readonly [who: string, bounds: Partial<WindowBounds>]>,
  screen: { width: number; height: number },
  where: string,
): void {
  const read: Array<readonly [string, WindowBounds]> = [];
  for (const [who, b] of placed) {
    for (const field of ["left", "top", "width", "height"] as const) {
      if (b[field] === undefined) {
        throw new Error(
          `the exit's own precondition cannot be read ${where}: ${who}'s window reported no ${field}, ` +
            "so this run cannot tell whether a window covers another",
        );
      }
    }
    read.push([who, b as WindowBounds]);
  }
  const problems: string[] = [];
  for (const [who, b] of read) {
    if (b.left < 0 || b.top < 0 || b.left + b.width > screen.width || b.top + b.height > screen.height) {
      problems.push(`${who}'s window (${describeBounds(b)}) leaves the ${screen.width}x${screen.height} screen`);
    }
  }
  read.forEach(([a, A], i) => {
    for (const [b, B] of read.slice(i + 1)) {
      const apart =
        A.left + A.width <= B.left || B.left + B.width <= A.left || A.top + A.height <= B.top || B.top + B.height <= A.top;
      if (!apart) problems.push(`${a}'s window (${describeBounds(A)}) and ${b}'s (${describeBounds(B)}) overlap`);
    }
  });
  if (problems.length > 0) {
    throw new Error(
      `the exit's own precondition failed ${where}: ${problems.join("; ")}. A window another covers can read hidden, and a ` +
        "hidden page lets its table's socket go, by design (R-P8), so this run could say nothing about the sockets " +
        "surviving a hibernation — it is not a finding about the product. Set EXIT_SCREEN to the screen's logical size, " +
        "and run it again.",
    );
  }
}

/**
 * Forget what `watches` recorded as not visible until now, and say what it
 * was, as `<who>: <what>` — keeping their report counts, which say the watch
 * is running. The exit calls this once its three windows are placed and its
 * table seated: a window reported hidden while the three were opening over one
 * another, and before they were placed, is not the claim (the second headed
 * run), and the precondition is read from the first move on.
 */
export function forgetHidden(
  watches: ReadonlyArray<readonly [who: string, page: Page, watch: VisibilityWatch]>,
): string[] {
  const forgotten: string[] = [];
  for (const [who, , watch] of watches) {
    for (const was of watch.hidden.splice(0)) forgotten.push(`${who}: ${was}`);
  }
  return forgotten;
}

/**
 * Put `page`'s window at `bounds`, in the normal state, through the browser
 * protocol, and read back where the window manager left it. That may differ
 * from what was asked — a window manager may keep a window clear of a menu
 * bar or a screen's edge — which is why the spec prints what it READ. Chromium
 * only (`newCDPSession` is Chromium's), and it sends nothing to any server.
 * The session is detached whatever happens.
 */
export async function placeWindow(page: Page, bounds: WindowBounds): Promise<Partial<WindowBounds>> {
  const cdp = await page.context().newCDPSession(page);
  try {
    const { windowId } = await cdp.send("Browser.getWindowForTarget");
    await cdp.send("Browser.setWindowBounds", { windowId, bounds: { ...bounds, windowState: "normal" } });
    const { bounds: got } = await cdp.send("Browser.getWindowBounds", { windowId });
    return { left: got.left, top: got.top, width: got.width, height: got.height };
  } finally {
    await cdp.detach();
  }
}
