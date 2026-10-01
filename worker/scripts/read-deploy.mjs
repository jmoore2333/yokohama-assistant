// **The deploy's readings** (spec §3.2; the plan's Task 11, Step 7) — run by
// the ORCHESTRATOR against the LIVE Worker after a deploy, never by a gate:
//
//   node scripts/read-deploy.mjs assets  <base-url> [--headers <file>]
//   node scripts/read-deploy.mjs install <base-url> [--headers <file>]
//   node scripts/read-deploy.mjs console <base-url> [--headers <file>]
//
// `--headers` names a FILE of `Name: value` lines — the Service Auth
// credential, as `worker/RUNBOOK.md` §4's recipe writes it — never a value on argv. The
// file's headers go to the base URL's ORIGIN only, and no value of theirs is
// ever printed: a secret sent to a third-party origin would leak, and would
// add a CORS preflight that is itself a console error. Without the file the
// requests go out bare, which over WARP the owner's own device answers.
//
// - `assets` (spec §0.3 row 7): every entry of the DEPLOYED `/sw.js`'s precache
//   list is fetched and checked — exactly 200, the listed type's essence, the
//   listed SHA-256 — and each is byte-exact or WRONG. **The inventory holds no
//   HTML** (R-6.5c2.41, spec §10.11): the zone's edge injects a per-request
//   script into every HTML answer, so the shell is listed as `/shell.bin`, a
//   name the edge does not rewrite, and nothing listed is excused for a
//   rewritten body. Then `/index.html` — no entry now, but the page every
//   navigation to the host is served — must answer 200, never a redirect, and
//   a missing `/assets/x.js` 404 with an empty body.
// - `install`: headless Chromium, `web/`'s Playwright, loads `/` in a fresh
//   profile and waits for the service worker: active, its release cache
//   holding every precache entry, `/shell.bin` cached as the release's bytes,
//   the door reading "Ready to play offline" (the door's own sentence, which
//   carries no period).
// - `console` (Step 7.6): the same browser, two loads of `/` — the first in a
//   fresh profile, the second once the service worker controls the page —
//   recording every console message, and every request hop that failed,
//   answered 400 or more, was redirected or came of a redirect, went to
//   another origin or to the edge's own `/cdn-cgi/`: its URL, method and type,
//   its status and response headers if any, its initiator, and whether the
//   service worker answered it — each redirect hop its own record. One JSON
//   document on stdout. No bearer token is printed: `set-cookie` and `cookie`
//   never are, no URL goes past its `?` or `#` — a record's own, or one inside
//   a console message's text — and a `location` header keeps its origin and
//   path alone. Access's eager redirects carry its token in a hop's query
//   (Task 11's re-review, N1).
//
// Exit 0 when the reading holds, 1 when it does not; `console` only reports.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";
import { readPrecacheList } from "./precache-list.mjs";

// ---- the pure half (`read-deploy.test.mjs`) ----

/** The deployed release's precache list, out of its `sw.js`: every entry's
 *  path, digest and type, by the ONE reader (`readPrecacheList`,
 *  `precache-list.mjs`). The count of digests is the anti-vacuity: a list
 *  whose printed shape drifted must not read as a shorter list. */
export function precacheList(swText) {
  return readPrecacheList(swText, "deployed");
}

/** A media type's essence: its parameters dropped, lowercased — how the
 *  service worker compares it (`web/src/swInstall.ts`). */
export function essence(contentType) {
  return (contentType ?? "").split(";")[0].trim().toLowerCase();
}

/** The header file's `Name: value` pairs, blank lines skipped. A line that is
 *  not a header is refused by its NUMBER alone: its text may be the secret. */
export function parseHeaderFile(text) {
  const pairs = [];
  text.split(/\r?\n/).forEach((line, i) => {
    if (line.trim() === "") return;
    const m = /^([A-Za-z0-9-]+):[ \t]*(\S.*?)[ \t]*$/.exec(line);
    if (!m) throw new Error(`the header file's line ${i + 1} is not \`Name: value\``);
    pairs.push([m[1], m[2]]);
  });
  if (pairs.length === 0) throw new Error("the header file holds no header");
  return pairs;
}

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");

/** The credential's headers for a request to `url` — only when it goes to
 *  `origin`, the deployment's own; `null` for any other origin, another port
 *  or another subdomain included. */
export function credentialFor(url, origin, pairs) {
  return pairs.length > 0 && new URL(url).origin === origin ? Object.fromEntries(pairs) : null;
}

/**
 * One precache entry against what the deployment answered for it: `ok` when
 * the status is exactly 200, the type's essence is the listed one and the body
 * hashes to the listed digest — and otherwise the reasons. There is no third
 * verdict: the release lists no HTML (R-6.5c2.41), so no entry is a body the
 * edge rewrites, and one that is rewritten is wrong.
 */
export function checkEntry(entry, answer) {
  const reasons = [];
  if (answer.status !== 200) reasons.push(`answered ${answer.status}`);
  const type = essence(answer.contentType);
  if (type !== entry.type) reasons.push(`answered ${type || "no content type"}, not ${entry.type}`);
  const digest = sha256(answer.body);
  if (digest !== entry.digest) reasons.push(`its body is ${digest}`);
  return { ok: reasons.length === 0, reasons };
}

// ---- the live half ----

/** The credential's pairs, from the `--headers` file — or none. */
function credentialFrom(file) {
  if (file === undefined) return [];
  return parseHeaderFile(readFileSync(file, "utf8"));
}

/** How the run presents itself: which header NAMES go where — never a value. */
function presenting(file, pairs) {
  return file === undefined
    ? "bare requests (over WARP, the owner's device answers Access)"
    : `${pairs.map(([name]) => name).join(" and ")} from ${file}, to this origin only`;
}

/** A GET of `path` against `base`, following no redirect, with the
 *  credential only where the URL is `base`'s own origin (`credentialFor` —
 *  Task 11's review, M8): a path that resolves elsewhere gets none of it. */
export async function get(base, path, pairs, fetcher = fetch) {
  const url = new URL(path, base);
  return fetcher(url, { redirect: "manual", headers: credentialFor(url.href, new URL(base).origin, pairs) ?? {} });
}

/** The deployed `sw.js`, and its precache list and cache name. An answer that
 *  is not the release — an Access login, the edge's challenge — is named by
 *  its status, type and size, never by what it says (R3's L-3; the file's own
 *  rule, above, and `web/e2e/live.ts`'s `accessState`, which names the
 *  credential's one answer the same way). */
export async function deployedRelease(base, pairs, fetcher = fetch) {
  const res = await get(base, "/sw.js", pairs, fetcher);
  const body = new Uint8Array(await res.arrayBuffer());
  if (res.status !== 200) {
    throw new Error(`/sw.js answered ${res.status} — ${answerShape(res.status, res.headers.get("content-type"), body.length)}`);
  }
  const text = new TextDecoder().decode(body);
  const cacheName = /yokohama-release-[0-9a-f]{16}/.exec(text)?.[0];
  return { entries: precacheList(text), cacheName, cacheControl: res.headers.get("cache-control") };
}

/** What an answer that is not the release was, for a line about it (twins of
 *  this wording: `said` in `replay/judge.mjs` and `what` in `web/e2e/live.ts`; a
 *  change to one is a change to all three): a
 *  redirect with no address — Access's carries a token — or a body's type and
 *  size in bytes, never its bytes. */
export function answerShape(status, contentType, bytes) {
  if (status >= 300 && status < 400) return "a redirect (its location is not printed: Access's carries a token)";
  return `a ${contentType ?? "typeless"} body of ${bytes} B`;
}

async function assets(base, file) {
  const pairs = credentialFrom(file);
  console.log(`read-deploy assets: ${base} — ${presenting(file, pairs)}`);
  const release = await deployedRelease(base, pairs);
  console.log(`/sw.js: 200, ${release.entries.length} precache entries, ${release.cacheName}, cache-control ${release.cacheControl}`);
  let exact = 0;
  const wrong = [];
  for (const entry of release.entries) {
    const res = await get(base, entry.path, pairs);
    const verdict = checkEntry(entry, {
      status: res.status,
      contentType: res.headers.get("content-type"),
      body: new Uint8Array(await res.arrayBuffer()),
    });
    if (verdict.ok) exact++;
    else wrong.push(`${entry.path} — ${verdict.reasons.join("; ")}`);
  }
  console.log(`entries: ${exact} byte-exact, ${wrong.length} wrong`);
  for (const line of wrong) console.log(`  WRONG  ${line}`);

  const index = await get(base, "/index.html", pairs);
  await index.arrayBuffer();
  const indexOk = index.status === 200;
  const indexLocation = index.headers.get("location");
  console.log(
    `/index.html: ${index.status}${indexLocation ? ` → ${locationOf(indexLocation, new URL("/index.html", base).href)}` : ""} ` +
      "(200 wanted, never a redirect)",
  );
  const missing = await get(base, "/assets/x.js", pairs);
  const missingBody = new Uint8Array(await missing.arrayBuffer());
  const missingOk = missing.status === 404 && missingBody.length === 0;
  console.log(`/assets/x.js: ${missing.status}, ${missingBody.length} B, content-type ${missing.headers.get("content-type")} (404 with an empty body wanted)`);

  const holds = wrong.length === 0 && indexOk && missingOk;
  console.log(`read-deploy assets: ${holds ? "HOLDS" : "DOES NOT HOLD"}`);
  return holds;
}

/** `web/`'s Playwright — the lane's pinned one, never a copy of its own. */
function playwright() {
  const require = createRequire(new URL("../../web/package.json", import.meta.url));
  try {
    return require("playwright");
  } catch {
    throw new Error("read-deploy: web/'s Playwright is not installed — run `cd web && npm ci && npx playwright install chromium`");
  }
}

/**
 * In the page: the active service worker's state once it is `activated` (or
 * `redundant`), `"none"` with no active worker, or `null` after `ms`.
 * `navigator.serviceWorker.ready` alone resolves while the worker may still be
 * `activating`.
 */
const ACTIVATED = (ms) =>
  Promise.race([
    navigator.serviceWorker.ready.then(
      (registration) =>
        new Promise((resolve) => {
          const worker = registration.active;
          if (!worker || worker.state === "activated") return resolve(worker ? "activated" : "none");
          worker.addEventListener("statechange", () => {
            if (worker.state === "activated" || worker.state === "redundant") resolve(worker.state);
          });
        }),
    ),
    new Promise((resolve) => setTimeout(() => resolve(null), ms)),
  ]);

/** A fresh browser context whose requests to `base`'s origin — and those
 *  only — carry the credential's headers. */
async function contextFor(browser, base, pairs) {
  const context = await browser.newContext();
  if (pairs.length > 0) {
    const origin = new URL(base).origin;
    await context.route("**/*", (route) => {
      const request = route.request();
      const extra = credentialFor(request.url(), origin, pairs);
      return extra === null ? route.continue() : route.continue({ headers: { ...request.headers(), ...extra } });
    });
  }
  return context;
}

async function install(base, file) {
  const pairs = credentialFrom(file);
  console.log(`read-deploy install: ${base} — ${presenting(file, pairs)}`);
  const release = await deployedRelease(base, pairs);
  const browser = await playwright().chromium.launch({ headless: true });
  try {
    const context = await contextFor(browser, base, pairs);
    const page = await context.newPage();
    // An install that fails posts its reason to every page (`web/src/sw.ts`).
    await page.addInitScript(() => {
      navigator.serviceWorker?.addEventListener("message", (e) => {
        if (e.data?.yokohama === "install-failed") window.__installFailed = e.data.reason;
      });
    });
    await page.goto(new URL("/", base).href, { waitUntil: "load" });
    const active = await page.evaluate(ACTIVATED, 120_000);
    // The shell is cached at `/shell.bin` (R-6.5c2.41), and the door's sentence
    // carries no period (`web/src/OnThisDevice.tsx`); `read-deploy.test.mjs`
    // holds both literals below to what the release and the door say.
    const state = await page.evaluate(
      async ({ cacheName, paths, shellDigest }) => {
        const names = await caches.keys();
        const cache = cacheName && names.includes(cacheName) ? await caches.open(cacheName) : null;
        const cached = cache ? (await cache.keys()).map((r) => new URL(r.url).pathname) : [];
        const shell = cache ? await cache.match("/shell.bin") : undefined;
        const shellBytes = shell ? await shell.arrayBuffer() : null;
        const hex = shellBytes
          ? [...new Uint8Array(await crypto.subtle.digest("SHA-256", shellBytes))].map((b) => b.toString(16).padStart(2, "0")).join("")
          : null;
        return {
          caches: names,
          cachedCount: cached.length,
          missing: paths.filter((p) => !cached.includes(p)),
          shellBytes: shellBytes?.byteLength ?? null,
          shellClean: hex === shellDigest,
          installFailed: window.__installFailed ?? null,
          door: document.body.innerText.includes("Ready to play offline"),
        };
      },
      {
        cacheName: release.cacheName,
        paths: release.entries.map((e) => e.path),
        shellDigest: release.entries.find((e) => e.path === "/shell.bin")?.digest,
      },
    );
    console.log(`service worker: ${active ?? "NOT ACTIVE after 120 s"}${state.installFailed ? ` — the install failed: ${state.installFailed}` : ""}`);
    console.log(`caches: ${state.caches.join(", ") || "none"}; ${release.cacheName}: ${state.cachedCount} entries for ${release.entries.length} listed`);
    if (state.missing.length > 0) console.log(`  missing from the cache: ${state.missing.slice(0, 10).join(", ")}${state.missing.length > 10 ? `, and ${state.missing.length - 10} more` : ""}`);
    console.log(`/shell.bin cached: ${state.shellBytes ?? "absent"} B, ${state.shellClean ? "the release's bytes" : "NOT the release's bytes"}`);
    console.log(`the door: ${state.door ? '"Ready to play offline"' : "not ready"}`);
    const holds = active === "activated" && state.missing.length === 0 && state.shellClean && state.door;
    console.log(`read-deploy install: ${holds ? "HOLDS" : "DOES NOT HOLD"}`);
    return holds;
  } finally {
    await browser.close();
  }
}

/** `url` cut at its first `?` or `#`: no query and no fragment, so no token
 *  a URL carries is printed (Task 11's re-review, N1). */
export function cutAtQuery(url) {
  if (typeof url !== "string") return url;
  const at = url.search(/[?#]/);
  return at === -1 ? url : url.slice(0, at);
}

/** `text` with every URL in it cut at its query (`cutAtQuery`): a console
 *  message quotes the URL it is about — a CORS error, a failed fetch. */
export function cutUrlsIn(text) {
  return text.replace(/\bhttps?:\/\/[^\s'"<>]+/g, (url) => cutAtQuery(url));
}

/** A `location` header's origin and path alone, a relative one resolved
 *  against `base`, the URL of the hop that answered it. */
export function locationOf(value, base) {
  try {
    const url = new URL(value, base);
    return `${url.origin}${url.pathname}`;
  } catch {
    return cutAtQuery(value);
  }
}

/** The response headers worth printing, for a hop at `base`: no cookie, and a
 *  `location` cut to its origin and path. */
function printable(headers, base) {
  return Object.fromEntries(
    Object.entries(headers ?? {})
      .filter(([name]) => !/^(set-)?cookie$/i.test(name))
      .map(([name, value]) => [name, /^location$/i.test(name) ? locationOf(value, base) : value]),
  );
}

/**
 * CDP's `Network` events folded into ONE record per HOP (Task 11's review,
 * M9). A redirect reuses its request's id: the next `requestWillBeSent`
 * carries the 3xx as `redirectResponse`, which closes the hop before it with
 * its own status and headers, and names where it went. A response or a
 * failure lands on the request's CURRENT hop. Cookies are never kept, and
 * every URL is cut at its query (`cutAtQuery`, `locationOf`).
 */
export function requestLog() {
  const records = [];
  const current = new Map();
  return {
    records,
    sent(e, load) {
      const url = cutAtQuery(e.request.url);
      const previous = current.get(e.requestId);
      if (previous && e.redirectResponse) {
        Object.assign(previous, {
          status: e.redirectResponse.status,
          headers: printable(e.redirectResponse.headers, previous.url),
          fromServiceWorker: e.redirectResponse.fromServiceWorker,
          redirectedTo: url,
        });
      }
      const top = e.initiator?.stack?.callFrames?.[0];
      const record = {
        load,
        url,
        method: e.request.method,
        type: e.type,
        initiator: {
          type: e.initiator?.type,
          url: cutAtQuery(e.initiator?.url ?? top?.url),
          line: e.initiator?.lineNumber ?? top?.lineNumber,
        },
        ...(previous && e.redirectResponse ? { redirectedFrom: previous.url } : {}),
      };
      records.push(record);
      current.set(e.requestId, record);
    },
    received(e) {
      const r = current.get(e.requestId);
      if (r) {
        Object.assign(r, {
          status: e.response.status,
          headers: printable(e.response.headers, r.url),
          fromServiceWorker: e.response.fromServiceWorker,
        });
      }
    },
    failed(e) {
      const r = current.get(e.requestId);
      if (r) Object.assign(r, { failed: e.errorText, blockedReason: e.blockedReason, corsError: e.corsErrorStatus });
    },
  };
}

async function consoleReading(base, file) {
  const pairs = credentialFrom(file);
  // On stderr, so stdout is one JSON document a reader can hand to `jq`.
  console.error(`read-deploy console: ${base} — ${presenting(file, pairs)}`);
  const origin = new URL(base).origin;
  const browser = await playwright().chromium.launch({ headless: true });
  try {
    const context = await contextFor(browser, base, pairs);
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send("Network.enable");
    /** Every request, one record per hop: what it was, how it was answered
     *  or why it failed. */
    const requests = requestLog();
    let load = 0;
    cdp.on("Network.requestWillBeSent", (e) => requests.sent(e, load));
    cdp.on("Network.responseReceived", (e) => requests.received(e));
    cdp.on("Network.loadingFailed", (e) => requests.failed(e));
    const consoleLines = [];
    page.on("console", (m) =>
      consoleLines.push({
        load,
        type: m.type(),
        text: cutUrlsIn(m.text()),
        location: { ...m.location(), url: cutAtQuery(m.location().url) },
      }),
    );
    page.on("pageerror", (e) => consoleLines.push({ load, type: "pageerror", text: cutUrlsIn(String(e)) }));
    const failed = [];
    page.on("requestfailed", (r) =>
      failed.push({
        load,
        url: cutAtQuery(r.url()),
        method: r.method(),
        resourceType: r.resourceType(),
        failure: r.failure()?.errorText ?? null,
      }),
    );

    // Load 1: a fresh profile, so no service worker controls the page yet.
    load = 1;
    await page.goto(new URL("/", base).href, { waitUntil: "load" });
    await page.waitForTimeout(8_000);
    const controlled = (await page.evaluate(ACTIVATED, 120_000)) === "activated";
    // Load 2: the service worker controls the page.
    load = 2;
    await page.reload({ waitUntil: "load" });
    await page.waitForTimeout(8_000);
    const swControls = await page.evaluate(() => navigator.serviceWorker.controller !== null);
    // Leaving fires the page's unload-time beacons, which load 2 still owns.
    await page.goto("about:blank");
    await page.waitForTimeout(2_000);

    const notable = requests.records.filter((r) => {
      const url = new URL(r.url);
      return (
        r.failed !== undefined ||
        (r.status !== undefined && r.status >= 400) ||
        r.redirectedTo !== undefined ||
        r.redirectedFrom !== undefined ||
        url.origin !== origin ||
        url.pathname.startsWith("/cdn-cgi/")
      );
    });
    console.log(
      JSON.stringify(
        { base, serviceWorkerReady: controlled, loadTwoControlled: swControls, console: consoleLines, requestfailed: failed, requests: notable },
        null,
        2,
      ),
    );
    return true;
  } finally {
    await browser.close();
  }
}

const MODES = { assets, install, console: consoleReading };

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [mode, base, ...rest] = process.argv.slice(2);
  const flag = rest.indexOf("--headers");
  const file = flag === -1 ? undefined : rest[flag + 1];
  const run = MODES[mode];
  if (!run || !base || (flag !== -1 && !file) || rest.length !== (flag === -1 ? 0 : 2)) {
    console.error("usage: node scripts/read-deploy.mjs assets|install|console <base-url> [--headers <file>]");
    process.exit(2);
  }
  try {
    process.exit((await run(base, file)) ? 0 : 1);
  } catch (e) {
    console.error(`read-deploy ${mode}: ${e instanceof Error ? e.message : String(e)}`);
    process.exit(1);
  }
}
