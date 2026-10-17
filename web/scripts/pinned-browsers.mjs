// **The pinned-browser instrument — ORCHESTRATOR-ONLY** (cycle E's Task 10.A/D/E,
// committed in M6.5c2 cycle F's Task 8; R-F3, Review Focus 1). It drives a real
// Chromium against a LIVE host — the update reading of a deploy — so no gate,
// no cell and no implementer runs it. Its cells (`pinned-browsers.node.test.ts`)
// run the command with no arguments, and read its decisions over fixtures.
//
//   node web/scripts/pinned-browsers.mjs <label> <origin> <state-dir> [--no-second-tab] [--go-wait <minutes>]
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
// the deal and makes a few moves (`tab.json`), and writes `PINNED` — ONLY over
// a door that reads "Ready to play offline" (cycle F's fix wave): a door
// still installing, failed, or drawing a line it does not know ends the run
// there, no `PINNED` written, `verdict.json` naming what the door read, exit
// 1. Both tabs stay open. **The second tab is the reading's stale-tab half, so a tab that
// does not open FAILS the verdict** ("the second tab holds a local game"), its
// three checks not run — unless the run was started with `--no-second-tab`,
// the one way to read the door alone: its three checks are then NOT RUN, none
// fails, and the verdict never says every check held (Task 8's review,
// Important 2).
//
// **Phase 2, on `<state-dir>/go` — after the deploy.** The wait for `go` is
// `--go-wait <minutes>` at most — whole minutes, 1 to 1440 — and three hours
// when it is not given (cycle F's fix wave and its residual fix: E's own
// pin-to-go gaps were 31.6 minutes on the Worker and 35.4 on the NAS, so
// thirty would have closed both readings early). Without a `go` in that time
// the run ends, `verdict.json` naming the wait and its bound, exit 1. On
// `go`: `before-go.json`, then
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
// every check and its one sentence (`verdictOf`): "N checks, every check held"
// only when every check ran and held, else "N checks: H held, F failed, R not
// run (why)". The exit is 0 when no check failed — a `--no-second-tab` run
// included, whose sentence still names what did not run — 1 when one did or the
// run broke, 2 for arguments it refuses.
//
// **No cookie VALUE is ever written**: a snapshot keeps cookie NAMES and
// domains, and the profile keeps no cookie past `done`: the context clears them before it closes, and a browser
// that died first is a FAILED check naming the profile (security S2a, D15). `log.txt` is the run's narrative, `net.jsonl` one line per request
// or response for the shell, the worker, the root and a challenge, the service
// worker's own flagged.

import { createRequire } from "node:module";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

/** Playwright, from the web package this file sits in. */
const require = createRequire(new URL("../package.json", import.meta.url));

export const USAGE =
  "usage: node web/scripts/pinned-browsers.mjs <label> <origin> <state-dir> [--no-second-tab] [--go-wait <minutes>]";

/** The flag that reads the door alone, with no second tab. */
const NO_SECOND_TAB = "--no-second-tab";
/** The option that bounds the wait for `go`, in whole minutes. */
const GO_WAIT = "--go-wait";
/** The wait for `go` when `--go-wait` is not given: three hours (cycle F's
 *  residual fix), well past E's measured pin-to-go gaps of 31.6 and 35.4
 *  minutes. */
export const GO_WAIT_DEFAULT_MINUTES = 180;
/** The longest wait `--go-wait` takes: a day. */
const GO_WAIT_MAX_MINUTES = 1440;

/** `--go-wait`'s value, checked: whole minutes from 1 to a day. */
function goWaitOf(raw) {
  if (!/^[1-9][0-9]*$/.test(raw) || Number(raw) > GO_WAIT_MAX_MINUTES) {
    throw new Error(`pinned-browsers: ${GO_WAIT} takes whole minutes from 1 to ${GO_WAIT_MAX_MINUTES} — got ${JSON.stringify(raw)}`);
  }
  return Number(raw);
}

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

/** The command line: `<label> <origin> <state-dir>`, each checked, then
 *  `--no-second-tab` and `--go-wait <minutes>` in either order, each at most
 *  once. A state directory that looks like a flag is refused, so a flag in the
 *  wrong place is never taken for one. */
export function pinnedArgs(argv) {
  const args = argv.slice(0, 3);
  if (args.length !== 3 || args.some((a) => a === "") || args[2].startsWith("-")) throw new Error(USAGE);
  let secondTab = true;
  let goWaitMinutes;
  for (let i = 3; i < argv.length; i += 1) {
    if (argv[i] === NO_SECOND_TAB && secondTab) {
      secondTab = false;
    } else if (argv[i] === GO_WAIT && goWaitMinutes === undefined && i + 1 < argv.length) {
      goWaitMinutes = goWaitOf(argv[i + 1]);
      i += 1;
    } else {
      throw new Error(USAGE);
    }
  }
  const [label, raw, stateDir] = args;
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
  return { label, origin: url.origin, stateDir, secondTab, goWaitMinutes: goWaitMinutes ?? GO_WAIT_DEFAULT_MINUTES };
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

/** A check that did not run, and why: `ok` is `null`, never a pass. */
const notRun = (name, why) => ({ check: name, ok: null, saw: why });

/** The game tab's three checks after the reload, by name. */
const TAB_CHECKS = ["the game tab draws the update line", "the game tab did not reload", "the game tab's game is unchanged"];

/** The click's four checks, by name — not run when there is no tab to click. */
export const CLICK_CHECKS = [
  "the game tab reloaded once",
  "the game tab draws no update line",
  "the same game, at the same move",
  "the game goes on",
];

/**
 * The run's one sentence over its checks: `ok` when none FAILED, `complete`
 * when every one RAN, and "every check held" only when both — so a run that
 * skipped a half never reads as a whole pass. No check at all is a failure.
 */
export function verdictOf(checks) {
  if (checks.length === 0) return { ok: false, complete: false, summary: "no check ran" };
  const held = checks.filter((c) => c.ok === true).length;
  const failed = checks.filter((c) => c.ok === false).length;
  const skipped = checks.filter((c) => c.ok === null);
  const ok = failed === 0;
  const complete = skipped.length === 0;
  if (ok && complete) return { ok, complete, summary: `${checks.length} checks, every check held` };
  const why = [...new Set(skipped.map((c) => String(c.saw)))].join("; ");
  return {
    ok,
    complete,
    summary: `${checks.length} checks: ${held} held, ${failed} failed, ${skipped.length} not run${why ? ` (${why})` : ""}`,
  };
}

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
      check(TAB_CHECKS[0], r.tab.lines.includes(SENTENCES.update), r.tab.lines),
      check(TAB_CHECKS[1], r.tab.loads === 0, { loads: r.tab.loads }),
      check(TAB_CHECKS[2], r.tab.url === r.tabBefore.url && r.tab.events === r.tabBefore.events, {
        before: r.tabBefore,
        now: { url: r.tab.url, events: r.tab.events },
      }),
    );
  } else if (r.secondTab) {
    // Wanted and not there: the stale-tab half is MISSING, which is a failure.
    out.push(check("the second tab holds a local game", false, "the second tab did not open a local game"));
    out.push(...TAB_CHECKS.map((name) => notRun(name, "the second tab did not open")));
  } else {
    out.push(...TAB_CHECKS.map((name) => notRun(name, "no second tab, by --no-second-tab")));
  }
  return out;
}

/** The checks over `after-2.json`'s reading: the click reloaded the game tab
 *  once into the same game at the same move, the line is gone, and one more
 *  move commits. */
export function afterClickChecks(r) {
  return [
    check(CLICK_CHECKS[0], r.tab.loads === 1, { loads: r.tab.loads }),
    check(CLICK_CHECKS[1], !r.tab.lines.includes(SENTENCES.update), r.tab.lines),
    check(CLICK_CHECKS[2], r.tab.url === r.before.url && r.tab.events === r.before.events, {
      before: r.before,
      now: { url: r.tab.url, events: r.tab.events },
    }),
    check(CLICK_CHECKS[3], r.eventsAfterAMove !== null && r.eventsAfterAMove > r.tab.events, {
      at: r.tab.events,
      afterAMove: r.eventsAfterAMove,
    }),
  ];
}

// ---- the browser half: the orchestrator's run ------------------------------

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** The first of `files` to exist, polled every `pollMs`: the orchestrator's
 *  trigger — or `null` once `boundMs` has passed with none (no bound by
 *  default). */
async function firstOf(files, boundMs = Infinity, pollMs = 5000) {
  const until = Date.now() + boundMs;
  for (;;) {
    const found = files.find((f) => fs.existsSync(f));
    if (found) return found;
    if (Date.now() >= until) return null;
    await sleep(Math.min(pollMs, Math.max(0, until - Date.now())));
  }
}

/** The check a `go` that never came within `minutes` of `PINNED` fails —
 *  named with its bound, so `verdict.json` says which wait gave up. */
export function goCheck(minutes) {
  return `go was touched within ${minutes} minutes of PINNED`;
}

/** The check a door that never read ready fails, with no `PINNED` written. */
export const PIN_CHECK = "the door read Ready to play offline before PINNED";

/** **Wait for `<stateDir>/go`, at most `minutes`** (cycle F's fix wave; Task
 *  8's review, Minor 1; the bound an argument since the residual fix,
 *  `--go-wait`, three hours by default): `null` once it is there, else the
 *  failed check naming the wait and its bound. A deploy that failed, or was
 *  never made, then ends the run with a `verdict.json` and exit 1 — never a
 *  browser left open that only a kill could close. `pollMs` and `minuteMs`
 *  (how long a minute is) are the cells' seams. */
export async function awaitGo(stateDir, minutes = GO_WAIT_DEFAULT_MINUTES, { pollMs = 5000, minuteMs = 60_000 } = {}) {
  const found = await firstOf([path.join(stateDir, "go")], minutes * minuteMs, pollMs);
  if (found !== null) return null;
  return check(goCheck(minutes), false, `no go in ${stateDir} within ${minutes} minutes: the deploy's reading never began`);
}

/** **`PINNED` only over a ready door** (cycle F's fix wave; Task 8's review,
 *  Minor 2). `PINNED` is the orchestrator's word to deploy, and a door still
 *  installing, failed, or drawing a line this instrument does not know holds
 *  no pinned release to read an update from. So it is written ONLY when
 *  `doorLines` read exactly "Ready to play offline": `null` then, else the
 *  failed check naming what the door read, and nothing written. */
export function pin(stateDir, doorLines, now = () => new Date()) {
  if (doorReadiness(doorLines) !== "ready") return check(PIN_CHECK, false, doorLines);
  fs.writeFileSync(path.join(stateDir, "PINNED"), `${now().toISOString()}\n`);
  return null;
}

/** Write `verdict.json` in `stateDir` over `checks`, under `head`
 *  (`{ label, origin, secondTab, goWaitMinutes }`, so the verdict names the
 *  bound the run waited with): the verdict, whose `ok` is the instrument's
 *  exit. */
export function writeVerdict(stateDir, head, checks) {
  const verdict = verdictOf(checks);
  fs.writeFileSync(path.join(stateDir, "verdict.json"), `${JSON.stringify({ ...head, ...verdict, checks }, null, 2)}\n`);
  return verdict;
}

/** **No session leaves with the profile** (security S2a, finding D15): the context's cookies — the deployment's
 *  `CF_Authorization` among them — are cleared before the profile is closed, so the gitignored state directory
 *  holds no credential at rest once a run is done. A browser that died first cannot be cleared: that is a FAILED
 *  check naming the profile to delete by hand, and the close and the verdict still happen — a throw here would
 *  lose `verdict.json` exactly when a run broke (measured: a SIGKILLed browser, no verdict). */
export async function leaveNoSession(context, log, checks, stateDir) {
  try {
    await context.clearCookies();
    log("cookies: cleared before close");
  } catch (e) {
    checks.push(check("the profile's cookies were cleared", false, e instanceof Error ? e.message : String(e)));
    log(`cookies: NOT cleared — delete ${path.join(stateDir, "profile")} by hand`);
  }
  await context.close();
}

/** Thrown to end the run early with its checks already recorded (a door that
 *  never read ready, a `go` that never came): the context still closes and
 *  `verdict.json` is still written — what a bare exit would lose. */
class Stop extends Error {}

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

async function run({ label, origin, stateDir, secondTab, goWaitMinutes }) {
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
      log(`CHECK ${c.ok === null ? "NOT RUN" : c.ok ? "ok  " : "FAIL"} ${c.check} — saw ${JSON.stringify(c.saw)}`);
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

    if (secondTab) {
      tab = await context.newPage();
      tabLoads = loadCounter(tab);
      try {
        game = await openLocalGame(tab, origin, log);
      } catch (e) {
        log(`phase 1: NO game tab — the verdict will fail — ${e instanceof Error ? e.message : String(e)}`);
        game = null;
      }
    } else {
      log("phase 1: no second tab, by --no-second-tab — its checks will read NOT RUN");
    }
    await snapshot("tab.json", { game });
    const doorRead = await linesOf(door, ".offline-state p");
    const unpinned = pin(stateDir, doorRead);
    if (unpinned !== null) {
      record([unpinned]);
      log(`phase 1: NO PINNED — the door reads ${JSON.stringify(doorRead)}, not Ready to play offline`);
      throw new Stop();
    }

    log(`phase 2: waiting for go, at most ${goWaitMinutes} minutes`);
    const late = await awaitGo(stateDir, goWaitMinutes);
    if (late !== null) {
      record([late]);
      log("phase 2: no go — closing with the verdict");
      throw new Stop();
    }
    const beforeGo = await snapshot("before-go.json");
    netMark = netLines().length;
    doorLoads.mark();
    tabLoads?.mark();
    const tabBefore = game === null ? null : { url: tab.url(), events: await storedEvents(tab, game.id) };
    log("phase 2: go — ONE plain reload of the door");
    await door.reload({ waitUntil: "domcontentloaded" });
    const t1 = Date.now();
    while (Date.now() - t1 < INSTALL_WAIT_MS && !(doorLoads.read() >= 2 && doorReadiness(await linesOf(door, ".offline-state p")) === "ready")) await sleep(2000);
    await sleep(SECOND_RELOAD_WAIT_MS);
    const s1 = await snapshot("after-1.json");
    const swPaths = s1.netSinceMark.filter((l) => l.kind === "request" && l.sw).map((l) => new URL(l.url).pathname);
    const reading = {
      secondTab,
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
        const why = secondTab ? "the second tab did not open" : "no second tab, by --no-second-tab";
        log(`phase 2: click asked, but there is NO game tab (${why})`);
        record(CLICK_CHECKS.map((name) => notRun(name, why)));
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
    // A Stop has recorded its own failed check; anything else is the run breaking.
    if (!(e instanceof Stop)) {
      checks.push(check("the run finished", false, e instanceof Error ? e.message : String(e)));
      log(`BROKE: ${e instanceof Error ? e.stack : String(e)}`);
    }
  }
  await leaveNoSession(context, log, checks, stateDir);
  const verdict = writeVerdict(stateDir, { label, origin, secondTab, goWaitMinutes }, checks);
  log(`closed: ${verdict.summary}`);
  return verdict.ok;
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
