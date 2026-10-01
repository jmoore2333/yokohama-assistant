// **The pinned-browser instrument — ORCHESTRATOR-ONLY** (cycle E's Task 10.A/D/E,
// committed in M6.5c2 cycle F's Task 8; R-F3, Review Focus 1). It drives a real
// Chromium against a LIVE host — the update reading of a deploy — so no gate,
// no cell and no implementer runs it. Its cells (`pinned-browsers.node.test.ts`)
// run the command with no arguments, and read its decisions over fixtures.
//
//   node web/scripts/pinned-browsers.mjs <label> <origin> <state-dir>
//
// `<origin>` is the host's `https` origin (plain `http` on a loopback host, for
// a dry run against the local binary); `<state-dir>` is where everything goes —
// `web/scripts/pinned/<label>` by convention, which is gitignored (a browser
// profile holds cookies). The window's commands (Task 11.A, 11.D, 11.G.g):
//
//   node web/scripts/pinned-browsers.mjs worker https://yokohama.moore.nyc web/scripts/pinned/worker
//   node web/scripts/pinned-browsers.mjs nas https://yokohama-nas.moore.nyc web/scripts/pinned/nas
//
// **Phase 1, at once — BEFORE the deploy.** A persistent context at
// `<state-dir>/profile` opens `<origin>/`, waits for the door to read "Ready to
// play offline" (or the failure's sentence), snapshots `pinned-before-clear.json`,
// clears the `cf_clearance` cookie and snapshots `pinned.json`. Then it opens a
// SECOND TAB on the door, starts a local game against the bot, keeps through
// the deal and makes a few moves (`tab.json`), and writes `PINNED`. Both tabs
// stay open.
//
// **Phase 2, on `<state-dir>/go` — after the deploy.** `before-go.json`, then
// ONE plain reload of the door. **Touch `go` as soon as the deploy's smoke
// passes**: from the deploy on, any check either tab makes — a tab coming into
// view, a navigation, the 15-minute interval (R-F3) — finds the release, and
// the door then takes it by itself BEFORE the plain reload (measured in this
// instrument's dry run, where the game tab's own check won). The check "the
// door had not taken a release before go" says so, and that reading is then
// the interval's, not the reload's. Cycle F's door applies a waiting release ITSELF and reloads once
// (Task 1): no click. When the door has loaded twice and reads "Ready to play
// offline", and ten seconds more have passed for a second reload to show
// itself, `after-1.json` records the door's lines, its loads, the registration,
// the caches, the release `/sw.js` names, the service worker's own requests
// since the reload, and the game tab's line, loads and game — and the checks
// over them (`afterReloadChecks`).
//
// **Optional, on `<state-dir>/click` — the stale second tab.** The game tab's
// "Reload to use it" is clicked; `after-2.json` records the reload, the same
// game at the same move, one more move made, and the checks (`afterClickChecks`).
//
// **Phase 3, on `<state-dir>/done`.** The context closes, `verdict.json` holds
// every check, and the exit is 0 when every check held, 1 when any did not or
// the run broke, 2 for arguments it refuses.
//
// **No cookie VALUE is ever written**: a snapshot keeps cookie NAMES and
// domains. `log.txt` is the run's narrative, `net.jsonl` one line per request
// or response for the shell, the worker, the root and a challenge, the service
// worker's own flagged.

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

/** Playwright, from the web package this file sits in. */
const require = createRequire(new URL("../package.json", import.meta.url));

export const USAGE = "usage: node web/scripts/pinned-browsers.mjs <label> <origin> <state-dir>";

/** The door's sentences, exactly as cycle F's door draws them (Task 1's
 *  report): the readiness line's three states, and the update line — the
 *  sentence and its button's words, as the element's text reads them. */
export const SENTENCES = Object.freeze({
  installing: "Preparing to play offline…",
  ready: "Ready to play offline",
  failed: /^Could not prepare to play offline — .+\. Try again$/,
  update: "A new version is ready. Reload to use it",
});

/** How long the door is given to install a release, as the offline cells give it. */
const INSTALL_WAIT_MS = 180_000;
/** How long after the door's second load a third would still be counted. */
const SECOND_RELOAD_WAIT_MS = 10_000;
/** The committed events the game tab's few moves reach past the deal. */
const TAB_EVENTS = 3;
/** The clicks the game tab's moves may spend. */
const TAB_CLICKS = 12;
/** The hosts plain `http` is taken for. */
const LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"]);

/** The command line: `<label> <origin> <state-dir>`, each checked. */
export function pinnedArgs(argv) {
  if (argv.length !== 3 || argv.some((a) => a === "")) throw new Error(USAGE);
  const [label, raw, stateDir] = argv;
  if (!/^[A-Za-z0-9._-]+$/.test(label)) throw new Error(`pinned-browsers: the label is letters, digits, dots, dashes — got ${JSON.stringify(label)}`);
  let url;
  try {
    url = new URL(raw);
  } catch {
    url = undefined;
  }
  const scheme = url?.protocol === "https:" || (url?.protocol === "http:" && LOOPBACK.has(url.hostname));
  if (!url || !scheme || url.pathname !== "/" || url.search !== "" || url.hash !== "" || raw.endsWith("/") || url.username !== "") {
    throw new Error(`pinned-browsers: the origin is the host's https origin with no path (plain http on loopback) — got ${JSON.stringify(raw)}`);
  }
  return { label, origin: url.origin, stateDir };
}

/** Which readiness the door's lines say: one of the three sentences, matched
 *  WHOLE; `none` when the door draws no readiness line; `unknown` for a line
 *  that is none of them. */
export function doorReadiness(lines) {
  const readiness = lines.filter((l) => l !== SENTENCES.update);
  if (readiness.length === 0) return "none";
  if (readiness.length !== 1) return "unknown";
  const [line] = readiness;
  if (line === SENTENCES.ready) return "ready";
  if (line === SENTENCES.installing) return "installing";
  if (SENTENCES.failed.test(line)) return "failed";
  return "unknown";
}

const check = (name, ok, saw) => ({ check: name, ok: Boolean(ok), saw });

/**
 * The checks over `after-1.json`'s reading (Task 11.D): nothing had moved
 * before `go` (`before-go.json`'s caches are the pinned generation alone), the
 * door took the deployed release by itself with exactly one reload and says it
 * is ready, the
 * release before it is kept, the install fetched the shell and no HTML — and
 * the game tab, when there is one, draws the line, did not reload, and holds
 * its game where it was.
 */
export function afterReloadChecks(r) {
  const out = [
    check(
      "the door had not taken a release before go",
      r.cachesBeforeGo.length === 1 && r.cachesBeforeGo[0] === r.pinnedRelease,
      r.cachesBeforeGo,
    ),
    check("the deployed release is not the pinned one", r.release !== null && r.release !== r.pinnedRelease, {
      pinned: r.pinnedRelease,
      deployed: r.release,
    }),
    check("the door reloaded itself exactly once", r.doorLoads === 2, { loadsSinceTheReload: r.doorLoads }),
    check("the door reads exactly Ready to play offline", doorReadiness(r.doorLines) === "ready", r.doorLines),
    check("the door draws no update line", !r.doorLines.includes(SENTENCES.update), r.doorLines),
    check("no worker waits, and the active one is activated", r.waiting === null && r.active === "activated", {
      waiting: r.waiting,
      active: r.active,
    }),
    check(
      "both generations stand",
      r.pinnedRelease !== null && r.release !== null && r.caches.includes(r.pinnedRelease) && r.caches.includes(r.release),
      r.caches,
    ),
    check(
      "the install fetched /shell.bin and no HTML",
      r.serviceWorkerPaths.includes("/shell.bin") && !r.serviceWorkerPaths.some((p) => p === "/" || p === "/index.html"),
      r.serviceWorkerPaths,
    ),
  ];
  if (r.tab !== null && r.tabBefore !== null) {
    out.push(
      check("the game tab draws the update line", r.tab.lines.includes(SENTENCES.update), r.tab.lines),
      check("the game tab did not reload", r.tab.loads === 0, { loads: r.tab.loads }),
      check("the game tab's game is unchanged", r.tab.url === r.tabBefore.url && r.tab.events === r.tabBefore.events, {
        before: r.tabBefore,
        now: { url: r.tab.url, events: r.tab.events },
      }),
    );
  }
  return out;
}

/** The checks over `after-2.json`'s reading: the click reloaded the game tab
 *  once into the same game at the same move, the line is gone, and one more
 *  move commits. */
export function afterClickChecks(r) {
  return [
    check("the game tab reloaded once", r.tab.loads === 1, { loads: r.tab.loads }),
    check("the game tab draws no update line", !r.tab.lines.includes(SENTENCES.update), r.tab.lines),
    check("the same game, at the same move", r.tab.url === r.before.url && r.tab.events === r.before.events, {
      before: r.before,
      now: { url: r.tab.url, events: r.tab.events },
    }),
    check("the game goes on", r.eventsAfterAMove !== null && r.eventsAfterAMove > r.tab.events, {
      at: r.tab.events,
      afterAMove: r.eventsAfterAMove,
    }),
  ];
}

// ---- the browser half: the orchestrator's run ------------------------------

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** The first of `files` to exist, polled every five seconds: the
 *  orchestrator's trigger. */
async function firstOf(files) {
  for (;;) {
    const found = files.find((f) => fs.existsSync(f));
    if (found) return found;
    await sleep(5000);
  }
}

/**
 * A page's main-document loads since its last mark, counted by the main
 * frame's NAVIGATION REQUESTS — a reload makes one whether the network or the
 * worker answers it, and a hash route makes none. Not the `load` event:
 * MEASURED in this instrument's own dry run, a background tab's reload went
 * out 30 ms after its click and its `load` event was not seen for a minute.
 */
function loadCounter(page) {
  let n = 0;
  page.on("request", (req) => {
    if (req.isNavigationRequest() && req.frame() === page.mainFrame()) n += 1;
  });
  return { mark: () => void (n = 0), read: () => n };
}

/** The lines a page draws: the door's readiness and update lines, or any
 *  screen's update line — each element's text, its whitespace folded. */
const linesOf = (page, selector) =>
  page
    .evaluate(
      (sel) => [...document.querySelectorAll(sel)].map((el) => (el.textContent ?? "").replace(/\s+/g, " ").trim()),
      selector,
    )
    .catch(() => []);

/** How many events this browser has committed for local game `id`: `-1` with
 *  no record yet (the e2e's `storedEvents`). */
async function storedEvents(page, id) {
  const record = await page
    .evaluate(
      (game) =>
        new Promise((resolve) => {
          const open = indexedDB.open("yokohama-local");
          open.onerror = () => resolve(null);
          open.onsuccess = () => {
            const db = open.result;
            if (!db.objectStoreNames.contains("docs")) return resolve(null);
            const t = db.transaction("docs", "readonly");
            const rec = t.objectStore("docs").get(["record", game]);
            t.oncomplete = () => resolve(rec.result?.text ?? null);
            t.onerror = () => resolve(null);
          };
        }),
      id,
    )
    .catch(() => null);
  return record === null ? -1 : JSON.parse(record).events.length;
}

/** Every wait polls on an INTERVAL: Playwright's default polls on
 *  `requestAnimationFrame`, which a background tab does not run — MEASURED in
 *  this instrument's dry run, a wait on the reloaded game tab's screen sat out
 *  its whole minute while the screen was there. */
const POLL = { polling: 250 };

/** Wait for the screen to settle: nothing in flight, no control disabled. */
const settle = (page) =>
  page
    .waitForFunction(() => !document.querySelector("[data-busy]") && !document.querySelector(".decision button[disabled]"), null, {
      ...POLL,
      timeout: 15_000,
    })
    .catch(() => undefined);

/** One click on the game screen, the e2e policy's simplest arms: skip a
 *  walking turn, apply the only option, take the first option row, or confirm
 *  a placement of nothing (which is legal). `false` when none is on screen. */
async function oneClick(page) {
  await settle(page);
  const skip = page.locator(".decision.walking button", { hasText: "skip" });
  if (await skip.count()) {
    await skip.first().click({ timeout: 2_000 }).catch(() => undefined);
    await settle(page);
  }
  const only = page.getByRole("button", { name: "Apply the only option" });
  const option = page.locator("button.option:not([disabled])");
  const confirm = page.getByRole("button", { name: "Confirm" });
  for (const control of [only, option, confirm]) {
    if ((await control.count()) && (await control.first().isEnabled())) {
      await control.first().click();
      await settle(page);
      return true;
    }
  }
  return false;
}

/** Click until a few more events are committed: the game moves on. Answers the
 *  committed count, or `null` when nothing was committed. */
async function moveOn(page, id, from, by) {
  let events = from;
  for (let i = 0; i < TAB_CLICKS && events < from + by; i += 1) {
    if (!(await oneClick(page))) break;
    events = await storedEvents(page, id);
  }
  return events > from ? events : null;
}

/** A local game against the bot in `tab`, through the door's own screens,
 *  kept through the deal and a few moves in. */
async function openLocalGame(tab, origin, log) {
  await tab.goto(`${origin}/#/`, { waitUntil: "domcontentloaded" });
  await tab.getByRole("heading", { name: "On this device" }).waitFor({ timeout: 60_000 });
  await tab.getByRole("button", { name: "New game", exact: true }).click();
  await tab.getByRole("heading", { name: "New game against the bot", exact: true }).waitFor({ timeout: 30_000 });
  await tab.getByRole("button", { name: "Deal" }).click();
  await tab.waitForURL(/#\/t\/local-[0-9a-f]+$/, { timeout: 60_000 });
  const id = tab.url().split("#/t/")[1] ?? "";
  for (let i = 0; i < 4; i += 1) {
    await tab.waitForFunction(
      () =>
        !!document.querySelector(".app.setup-table.dealing button.deal-pick:not([disabled])") ||
        !!document.querySelector(".app:not(.setup-table)"),
      null,
      { ...POLL, timeout: 30_000 },
    );
    if (await tab.locator(".app:not(.setup-table)").count()) break;
    await tab.locator(".app.setup-table.dealing button.deal-pick:not([disabled])").first().click();
  }
  const start = await storedEvents(tab, id);
  const reached = await moveOn(tab, id, start, TAB_EVENTS);
  log(`phase 1: the game tab plays ${id}: events ${start} -> ${reached ?? start}`);
  return { id, url: tab.url(), events: await storedEvents(tab, id) };
}

async function run({ label, origin, stateDir }) {
  const { chromium } = require("playwright");
  fs.mkdirSync(stateDir, { recursive: true });
  const logf = path.join(stateDir, "log.txt");
  const log = (s) => fs.appendFileSync(logf, `${new Date().toISOString()} ${s}\n`);
  const netf = path.join(stateDir, "net.jsonl");
  const interesting = /\/sw\.js|\/shell\.bin|\/index\.html|\/cdn-cgi\/challenge|^https?:\/\/[^/]+\/(\?.*)?$/;
  const netLines = () => (fs.existsSync(netf) ? fs.readFileSync(netf, "utf8").split("\n").filter(Boolean) : []);
  let netMark = 0;
  const checks = [];

  const context = await chromium.launchPersistentContext(path.join(stateDir, "profile"), {
    headless: true,
    serviceWorkers: "allow",
    viewport: { width: 1280, height: 832 },
  });
  context.on("request", (req) => {
    const u = req.url();
    if (!interesting.test(u)) return;
    fs.appendFileSync(netf, `${JSON.stringify({ t: Date.now(), kind: "request", url: u, method: req.method(), sw: !!req.serviceWorker() })}\n`);
  });
  context.on("response", (res) => {
    const u = res.url();
    if (!interesting.test(u)) return;
    const h = res.headers();
    fs.appendFileSync(
      netf,
      `${JSON.stringify({ t: Date.now(), kind: "response", url: u, status: res.status(), ct: h["content-type"] ?? null, cc: h["cache-control"] ?? null, cf: h["cf-cache-status"] ?? null, sw: !!res.request().serviceWorker() })}\n`,
    );
  });
  const door = context.pages()[0] ?? (await context.newPage());
  const doorLoads = loadCounter(door);
  let tab = null;
  let tabLoads = null;
  let game = null;

  /** What the door, the registration, the deployed `/sw.js`, the cookies'
   *  NAMES and the game tab say now — written to `name`, with `extra`. */
  async function snapshot(name, extra = {}) {
    const doorLines = await linesOf(door, ".offline-state p");
    const sw = await door
      .evaluate(async () => {
        const reg = await navigator.serviceWorker.getRegistration();
        const s = (w) => (w ? { url: w.scriptURL, state: w.state } : null);
        return {
          installing: s(reg?.installing),
          waiting: s(reg?.waiting),
          active: s(reg?.active),
          controller: navigator.serviceWorker.controller?.scriptURL ?? null,
          caches: (await caches.keys()).sort(),
        };
      })
      .catch((e) => ({ error: e.message, caches: [], waiting: null, active: null }));
    let release = null;
    try {
      const text = await door.evaluate(async () => (await fetch("/sw.js", { cache: "no-store" })).text());
      release = (text.match(/yokohama-release-[0-9a-f]{16}/) || [null])[0];
    } catch {
      release = null;
    }
    const cookieNames = (await context.cookies()).map((c) => ({ name: c.name, domain: c.domain }));
    const tabNow =
      tab === null || game === null
        ? null
        : { lines: await linesOf(tab, "p.update-line"), loads: tabLoads.read(), url: tab.url(), events: await storedEvents(tab, game.id) };
    const net = netLines().slice(netMark).map((l) => JSON.parse(l));
    const out = { label, origin, at: new Date().toISOString(), doorLines, doorLoads: doorLoads.read(), sw, releaseInSwJs: release, cookieNames, tab: tabNow, netSinceMark: net, ...extra };
    fs.writeFileSync(path.join(stateDir, name), `${JSON.stringify(out, null, 2)}\n`);
    log(`snapshot ${name}: door=${JSON.stringify(doorLines)} loads=${doorLoads.read()} caches=${JSON.stringify(sw.caches)} waiting=${sw.waiting?.state ?? "-"} active=${sw.active?.state ?? "-"} release=${release} tab=${JSON.stringify(tabNow)}`);
    return out;
  }

  /** Record `list`'s checks and log each. */
  const record = (list) => {
    for (const c of list) {
      checks.push(c);
      log(`CHECK ${c.ok ? "ok  " : "FAIL"} ${c.check} — saw ${JSON.stringify(c.saw)}`);
    }
    return list;
  };

  try {
    log(`phase 1: open ${origin}/`);
    await door.goto(`${origin}/`, { waitUntil: "domcontentloaded" });
    const t0 = Date.now();
    while (Date.now() - t0 < INSTALL_WAIT_MS && !["ready", "failed"].includes(doorReadiness(await linesOf(door, ".offline-state p")))) await sleep(2000);
    log(`phase 1: the door reads ${doorReadiness(await linesOf(door, ".offline-state p"))}`);
    await sleep(3000);
    await snapshot("pinned-before-clear.json");
    await context.clearCookies({ name: "cf_clearance" });
    log(`phase 1: cf_clearance cleared; cookies now: ${(await context.cookies()).map((c) => c.name).join(",") || "(none)"}`);
    const pinned = await snapshot("pinned.json");

    tab = await context.newPage();
    tabLoads = loadCounter(tab);
    try {
      game = await openLocalGame(tab, origin, log);
    } catch (e) {
      log(`phase 1: NO game tab — ${e instanceof Error ? e.message : String(e)}`);
      game = null;
    }
    await snapshot("tab.json", { game });
    fs.writeFileSync(path.join(stateDir, "PINNED"), `${new Date().toISOString()}\n`);

    log("phase 2: waiting for go");
    await firstOf([path.join(stateDir, "go")]);
    const beforeGo = await snapshot("before-go.json");
    netMark = netLines().length;
    doorLoads.mark();
    tabLoads.mark();
    const tabBefore = game === null ? null : { url: tab.url(), events: await storedEvents(tab, game.id) };
    log("phase 2: go — ONE plain reload of the door");
    await door.reload({ waitUntil: "domcontentloaded" });
    const t1 = Date.now();
    while (Date.now() - t1 < INSTALL_WAIT_MS && !(doorLoads.read() >= 2 && doorReadiness(await linesOf(door, ".offline-state p")) === "ready")) await sleep(2000);
    await sleep(SECOND_RELOAD_WAIT_MS);
    const s1 = await snapshot("after-1.json");
    const swPaths = s1.netSinceMark.filter((l) => l.kind === "request" && l.sw).map((l) => new URL(l.url).pathname);
    const reading = {
      pinnedRelease: pinned.releaseInSwJs,
      release: s1.releaseInSwJs,
      caches: s1.sw.caches ?? [],
      waiting: s1.sw.waiting?.state ?? null,
      active: s1.sw.active?.state ?? null,
      doorLines: s1.doorLines,
      doorLoads: s1.doorLoads,
      serviceWorkerPaths: swPaths,
      tab: s1.tab,
      tabBefore,
    };
    const one = record(afterReloadChecks({ ...reading, cachesBeforeGo: beforeGo.sw.caches ?? [] }));
    fs.writeFileSync(path.join(stateDir, "after-1.checks.json"), `${JSON.stringify(one, null, 2)}\n`);

    log("phase 2: waiting for click (optional) or done");
    const next = await firstOf([path.join(stateDir, "click"), path.join(stateDir, "done")]);
    if (next.endsWith("click")) {
      if (game === null) {
        log("phase 2: click asked, but there is NO game tab");
      } else {
        const before = { url: tab.url(), events: await storedEvents(tab, game.id) };
        netMark = netLines().length;
        tabLoads.mark();
        // A person clicks a tab they are looking at: the tab comes forward.
        await tab.bringToFront();
        const button = tab.locator("p.update-line button", { hasText: "Reload to use it" });
        if (await button.count()) {
          log("phase 2: clicking the game tab's 'Reload to use it'");
          await button.first().click();
        } else {
          log("phase 2: NO 'Reload to use it' on the game tab");
        }
        const t2 = Date.now();
        while (Date.now() - t2 < 60_000 && tabLoads.read() < 1) await sleep(1000);
        log(`phase 2: the game tab's reload seen after ${Date.now() - t2} ms (loads ${tabLoads.read()})`);
        const t3 = Date.now();
        const drawn = await tab
          .waitForFunction(() => !!document.querySelector(".decision"), null, { ...POLL, timeout: 60_000 })
          .then(() => true, () => false);
        log(`phase 2: the game tab's decision zone ${drawn ? "drawn" : "NOT drawn"} after ${Date.now() - t3} ms`);
        await sleep(3000);
        const s2 = await snapshot("after-2.json");
        const eventsAfterAMove = s2.tab === null ? null : await moveOn(tab, game.id, s2.tab.events, 1);
        const two = record(afterClickChecks({ before, tab: s2.tab, release: s2.releaseInSwJs, eventsAfterAMove }));
        fs.writeFileSync(path.join(stateDir, "after-2.checks.json"), `${JSON.stringify(two, null, 2)}\n`);
      }
      log("phase 3: waiting for done");
      await firstOf([path.join(stateDir, "done")]);
    }
  } catch (e) {
    checks.push(check("the run finished", false, e instanceof Error ? e.message : String(e)));
    log(`BROKE: ${e instanceof Error ? e.stack : String(e)}`);
  }
  await context.close();
  const ok = checks.length > 0 && checks.every((c) => c.ok);
  fs.writeFileSync(path.join(stateDir, "verdict.json"), `${JSON.stringify({ label, origin, ok, checks }, null, 2)}\n`);
  log(`closed: ${ok ? "every check held" : "a check FAILED — verdict.json"}`);
  return ok;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  let args;
  try {
    args = pinnedArgs(process.argv.slice(2));
  } catch (e) {
    process.stderr.write(`${e instanceof Error ? e.message : String(e)}\n`);
    process.exit(2);
  }
  process.exit((await run(args)) ? 0 : 1);
}
