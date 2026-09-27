// **The nine offline cells: where this cycle's claims actually live** (M6.5b2,
// spec §6.3's six, and PR #37's seventh, eighth and ninth).
//
// Every other instrument in this cycle stops short of the thing the milestone
// is about. `swRoute.ts` and `swInstall.ts` are pure functions with cells;
// `swRegister.test.ts` drives the lifecycle over a seam; `precache.test.ts`
// builds a release out of a fixture tree. None of them installs a service
// worker, none of them takes the network away, and none of them closes a
// browser and opens it again. So the three things only this file can see are
// the three the cycle claims: that a release really is cached whole and serves
// a whole game with the network off, that it survives the browser closing, and
// that a second release waits behind the running one until somebody asks for
// it.
//
// @yokohama-service-worker: on
//
// **This is the ONE spec that registers the worker**, which is why it takes
// `test` from `@playwright/test` and not from `./sw`, and why it says so in the
// marker above — `every_spec_declares_its_worker.spec.ts` reads this directory
// at run time and names any file that does neither. Cells 4 and 5 are about a
// restore and not about the shell, so they flag the worker OFF for themselves
// with the same string the fixture uses: a ten-megabyte precache per context
// that no assertion reads is wall nobody is buying anything with.
//
// **A seventh cell is PR #37's F2** (`docs/plans/2026-09-22-web-frontend-
// m6.5b2-pr37-fixes.md`, R-F2): the tab that did NOT click an update, handed to
// the new worker still running the old bundle, restarting its local host with
// the network off. It sits beside cell 3 because it is cell 3's machinery with
// a second tab and a genuinely renamed file.
//
// **Cells 8 and 9 are PR #37's F3** (R-F3, and the F3 task review's E1): a
// "Try again" that has to SUCCEED, in the two ways the first attempt can fail.
// Cell 8 refuses the first `/sw.js`, so the REGISTRATION fails; cell 9 lets it
// through and refuses one precache file once, so the FIRST INSTALL fails —
// which empties the registration and removes it from the map, so `update()`
// on it is refused. Both refuse through `context.route`, which at Playwright
// 1.62.1 intercepts the registration's own script fetch AND the worker's
// precache fetches (measured before either cell was written): the refusal is
// one request, and every other byte is the real server's.
//
// **Cells 3 and 7 MUTATE `web/dist` in place**, put it back in a `finally`, and
// CHECK the restoration there too — the files' own bytes and the restored
// worker's cache name, read back from disk. `dist` is what the walk's server
// serves per request, so a cell that left it changed would hand every spec
// after it a release the build did not produce; the check therefore has to run
// on the failing path, which is where it matters most. **`git status` is not
// evidence for any of this**: `web/dist/` is gitignored, so it is clean whether
// the build was restored or not.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, expect, test, type BrowserContext, type Page } from "@playwright/test";
import { archiveDir, BASE_URL, haveBrowser } from "../playwright.config";
// The flag every OTHER spec gets from the `context` fixture. Imported as the
// string rather than restated, so the two can never disagree about what the
// client reads.
import { NO_SERVICE_WORKER } from "./sw";
// The cache-name prefix the worker and the generator share. Read from the
// module that defines it rather than written out again here: a sweep that
// looked for a prefix this file had guessed would report "no generations" for
// ever and every retention assertion below would pass by finding nothing.
import { CACHE_PREFIX } from "../src/precache";
import { act, atTheGame, capReached, deadEnd, readScreen, rng, startGame, WALKER, type Game } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly — the guard
// `every_spec_can_skip.spec.ts` reads this directory for.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The walk's own cap, the same figure every other walk here carries.
 *  A hit throws `capReached`, which names the cap, the turn and the
 *  end-trigger strip — never `deadEnd`'s stuck-table sentence (M6.5c2). */
const CAP = 4000;

/** How long a cold install is given. `INSTALL_DEADLINE_MS` in `swInstall.ts` is
 *  120 s for the install itself, and this is the same budget plus the page's
 *  own boot — a wait shorter than the worker's own deadline would report a slow
 *  machine as a failed release. */
const INSTALL_WAIT_MS = 150_000;

const HERE = path.dirname(fileURLToPath(import.meta.url));
const DIST = path.join(HERE, "..", "dist");
/** The web app manifest: small, valid JSON, precached, and read by nothing the
 *  app's behaviour depends on — which is what makes it the file cell 3 can
 *  change to tell two releases apart. */
const MARKED = path.join(DIST, "manifest.webmanifest");
const SW = path.join(DIST, "sw.js");
const ASSETS = path.join(DIST, "assets");
/** The name cell 7 gives the local host's chunk in release 2 — Vite's own
 *  `<name>-<8 characters>.js` shape, spelling what it is for. */
const RENAMED = "worker-F2retain.js";
/** The committed envelope cells 4 and 5 restore: a finished 2P game, a
 *  table-only game at the deal, a cross-rev pair and a record that is not one.
 *  `crates/yokohama-web`'s own backup cells read the same file. */
const ENVELOPE = path.join(HERE, "..", "src", "wire-script", "backup-envelope.json");

/**
 * A path-safe rendering of the caller's address — the server's own rule in
 * `archive.rs`'s `caller_segment`, restated here for `plays_locally.spec.ts`'s
 * reason: the reply is the thing under test, so a server that filed a game
 * under the wrong prefix would be believed by a walk that took the prefix from
 * it.
 */
const SEGMENT = [...WALKER]
  .map((c) => (c === "@" ? "_at_" : /[a-z0-9._-]/.test(c) ? c : "_"))
  .join("");

/** This build's semantics rev, which is the archive's own directory level. */
const REV = "rev7";

/**
 * Where `api.ts` keeps the name this browser signs its requests with
 * (`CLAIMED_USER_KEY`).
 *
 * **Restated rather than imported**, because importing `src/api.ts` here would
 * pull React and the whole client into the node project this file is
 * type-checked in. A rename on the client side makes cell 6 fail LOUDLY — the
 * archive POST answers 401 and the row reads "not signed in" instead of going
 * away — which is the direction a restated constant has to fail in to be worth
 * restating.
 */
const CLAIMED_USER_KEY = "yokohama.claimed_user";

// ---------------------------------------------------------------- helpers

/** Every release generation this origin holds, sorted. */
const releaseCaches = (page: Page, prefix: string = CACHE_PREFIX): Promise<string[]> =>
  page.evaluate(
    (p) => caches.keys().then((names) => names.filter((n) => n.startsWith(p)).sort()),
    prefix,
  );

/** Whether one generation holds one path. */
const cacheHas = (page: Page, name: string, at: string): Promise<boolean> =>
  page.evaluate(
    ({ name: n, at: a }) =>
      caches
        .open(n)
        .then((c) => c.match(a))
        .then(Boolean),
    { name, at },
  );

/**
 * Wait for the door to say the release is cached, and CHECK that it is.
 *
 * **The line alone would be a claim about a string.** `readinessOf` reads
 * "ready" from the lifecycle's `installed` state, which is what an
 * all-or-nothing install reaches only after every body has been fetched and
 * digested — so the line is a real signal. The cache read beside it is what
 * says the signal is about this release rather than about a worker that
 * registered and cached nothing.
 */
async function installed(page: Page): Promise<string> {
  await expect(page.getByText("Ready to play offline.")).toBeVisible({ timeout: INSTALL_WAIT_MS });
  const names = await releaseCaches(page);
  // Every caller is a FRESH profile installing its first release, so "the one
  // generation" is unambiguous — asserted rather than assumed, because a helper
  // that quietly picked the alphabetically last of several would be reporting
  // about whichever name sorted highest.
  expect(names, "a fresh profile holds exactly one release generation").toHaveLength(1);
  const release = names[0]!;
  expect(await cacheHas(page, release, "/index.html"), "the shell is in the cache").toBe(true);
  expect(
    await cacheHas(page, release, "/art/manifest.json"),
    "and so is the art, which is what makes this a whole release rather than a shell",
  ).toBe(true);
  return release;
}

/** Play `n` moves, or until the game ends. */
async function playSome(page: Page, game: Game, pick: () => number, n: number): Promise<number> {
  let moves = 0;
  for (let i = 0; i < n; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, i);
    moves += 1;
  }
  await readScreen(page);
  return moves;
}

/** Play to a winner, the same loop `plays_locally.spec.ts` uses. */
async function playToOver(page: Page, game: Game, pick: () => number): Promise<number> {
  let moves = 0;
  let screen = await readScreen(page);
  while (!screen.over && moves < CAP) {
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, moves);
    moves += 1;
    screen = await readScreen(page);
  }
  if (!screen.over) throw await capReached(page, game, moves);
  expect(moves, "the walk must not hit its own cap").toBeLessThan(CAP);
  await expect(page.getByText("The game is over.")).toBeVisible();
  return moves;
}

/** The turn the screen is on, off the decision zone's own chain — what says a
 *  resumed game came back where it was left rather than at the start. */
const chainOf = (page: Page): Promise<string> =>
  page.locator(".decision .chain").first().innerText();

/** Whether this page is being served by a worker at all. Offline, the answer
 *  is the difference between a cached release and a lucky HTTP cache. */
const controlled = (page: Page): Promise<boolean> =>
  page.evaluate(() => navigator.serviceWorker.controller !== null);

/** Every request this page made to the API, split by what became of it.
 *
 *  **A response of ANY status counts as "the network answered"**, which is
 *  what the offline claim is about: a 404 from the server is as much a
 *  contradiction of "nothing succeeded" as a 200 would be. A request that
 *  failed is recorded separately and is the anti-vacuity half — a page that
 *  TRIED and could not reach the server is a page whose instrument is wired to
 *  something. */
function watchTheApi(page: Page): { answered: string[]; failed: string[] } {
  const answered: string[] = [];
  const failed: string[] = [];
  const mine = (url: string) => {
    const u = new URL(url);
    return u.origin === new URL(BASE_URL).origin && u.pathname.startsWith("/api/") ? u.pathname : null;
  };
  page.on("response", (r) => {
    const at = mine(r.url());
    if (at) answered.push(`${r.request().method()} ${at} ${r.status()}`);
  });
  page.on("requestfailed", (r) => {
    const at = mine(r.url());
    if (at) failed.push(`${r.method()} ${at}`);
  });
  return { answered, failed };
}

/**
 * Uncaught exceptions, and console errors the network did not cause.
 *
 * **`watchForBreakage` cannot be used here and the reason is the subject.**
 * Chromium logs every failed resource load as a console error, and four of
 * these nine cells take the network away on purpose — so the shared watch would
 * report the very state the cell arranged. What is still worth watching is a
 * page that THREW, which no amount of offline explains.
 */
function watchForFaults(page: Page): { failures: string[] } {
  const failures: string[] = [];
  const network = /net::ERR_|Failed to load resource|Failed to fetch|NetworkError/i;
  page.on("console", (m) => {
    if (m.type() !== "error") return;
    if (network.test(m.text())) return;
    failures.push(`console.error: ${m.text()}`);
  });
  page.on("pageerror", (e) => failures.push(`uncaught: ${e.message}`));
  return { failures };
}

/**
 * Hand the file input a path and wait for THIS restore's report, and read it as
 * {id → what the row says}.
 *
 * **The previous report is marked before the file goes in**, because a bare
 * wait for `.restore-report li` is already satisfied by the one still on screen
 * — cell 5 restores three times, and a read taken a moment too early would be a
 * read of the last answer. `Restore` clears its report before it awaits
 * anything (`BackupControls.tsx`'s `take`), so the list really is unmounted and
 * the one that comes back is a new element that carries no mark: the wait is
 * race-free in both directions, where a wait for the old list to VANISH would
 * time out whenever the new one arrived first.
 */
async function restore(page: Page, file: string): Promise<Map<string, string>> {
  await page.evaluate(() => {
    document.querySelector(".restore-report")?.setAttribute("data-e2e-stale", "");
  });
  const fresh = page.locator(".restore-report:not([data-e2e-stale])");
  await page.getByLabel("Restore from a backup").setInputFiles(file);
  await expect(fresh.locator("li").first()).toBeVisible({ timeout: 60_000 });
  return new Map(
    await fresh.locator("li").evaluateAll((els) =>
      els.map(
        (el) =>
          [
            el.querySelector(".restore-id")?.textContent?.trim() ?? "",
            el.querySelector(".meta")?.textContent?.trim() ?? "",
          ] as [string, string],
      ),
    ),
  );
}

// ------------------------------------------------------------------ cells

test("offline: the release installs, the page comes back with the network off, and a whole game plays", async ({
  page,
  context,
}) => {
  const watch = watchForFaults(page);
  const api = watchTheApi(page);

  await page.goto("/");
  await installed(page);

  // **The network goes away, and the page is loaded again from nothing but the
  // cache.** The reload is what makes the page CONTROLLED — the worker does not
  // call `clients.claim()`, so the visit that installed it is not served by it
  // — which is also the state a returning player is in.
  await context.setOffline(true);
  await page.reload();
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  expect(await controlled(page), "the shell came from the worker and not from the network").toBe(
    true,
  );

  // A whole game, played with the network off. `signedOut` because there is no
  // door to type into and a local game needs none.
  const game = await startGame(page, { local: true, signedOut: true });
  expect(game.id.startsWith("local-"), `a local id, not ${game.id}`).toBe(true);
  const moves = await playToOver(page, game, rng(0x4f46_464c)); // "OFFL"

  // **The anti-vacuity half, and it is the whole reading.** "Nothing
  // succeeded" is trivially true of a page that never asked — so the game had
  // to have been PLAYED (the move count and the ranked standing below), and the
  // page had to have TRIED to reach the API and failed: the game-over hook asks
  // this deployment what it keeps, which offline is a request that never
  // arrives.
  expect(moves, "a whole game really was played offline").toBeGreaterThan(20);
  await expect(page.locator("table.outcome tbody tr")).toHaveCount(2);
  await expect(page.locator("table.outcome tr.won")).toHaveCount(1);
  // **The window is the WHOLE cell and not only the offline half**, which is a
  // stronger reading and the true one: the front door asks the API for nothing
  // (R-6.5b2.7), so not one request of this page's was answered from the first
  // visit to the last move.
  expect(api.answered, "not one /api/ request of this page's was ever answered").toEqual([]);
  expect(
    api.failed.filter((f) => f.endsWith("/api/health")).length,
    "and the page did ask: the game-over hook's own health read never arrived",
  ).toBeGreaterThan(0);

  // eslint-disable-next-line no-console
  console.log(
    `OFFLINE WALK: game ${game.id} finished in ${moves} clicks with the network off; ` +
      `${api.failed.length} /api/ requests failed, ${api.answered.length} were answered`,
  );
  expect(watch.failures, "the page threw nothing while it played with no network").toEqual([]);
});

test("close and reopen: the profile keeps the release and the game, and both come back offline", async () => {
  // **`launchPersistentContext` and nothing else will do** (spec §6.3).
  // `storageState` carries cookies, localStorage and — at 1.62.1 — optionally
  // IndexedDB, but never the Cache Storage or the worker registration, which
  // are the two things this cell is about. A profile directory carries all
  // four, because it is what a browser actually keeps.
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), "yokohama-e2e-profile-"));
  let first: BrowserContext | null = null;
  let second: BrowserContext | null = null;
  try {
    first = await chromium.launchPersistentContext(profile, { baseURL: BASE_URL });
    const a = first.pages()[0] ?? (await first.newPage());
    await a.goto("/");
    const release = await installed(a);

    const game = await startGame(a, { local: true, signedOut: true });
    const pick = rng(0x434c_4f53); // "CLOS"
    const opening = await playSome(a, game, pick, 24);
    expect(opening, "there is a played game here to come back to").toBeGreaterThan(0);
    const left = await chainOf(a);

    // Offline BEFORE the close, so nothing can quietly re-fetch on the way out.
    await first.setOffline(true);
    await first.close();
    first = null;

    // **The same profile, opened again with the network still off.** Nothing is
    // carried across in this process: the release, the registration and the
    // game are whatever Chromium wrote into that directory.
    second = await chromium.launchPersistentContext(profile, { baseURL: BASE_URL, offline: true });
    const b = second.pages()[0] ?? (await second.newPage());
    const watch = watchForFaults(b);
    const api = watchTheApi(b);

    await b.goto("/");
    await expect(b.getByRole("heading", { name: "On this device" })).toBeVisible();
    expect(await controlled(b), "the reopened browser was served by its own worker").toBe(true);
    expect(
      await releaseCaches(b),
      "and it is the same release generation the first session installed",
    ).toEqual([release]);

    // The game is LISTED, by its own id, on a door drawn from this browser alone.
    const row = b.locator(".on-this-device").getByRole("link", { name: game.id, exact: true });
    await expect(row).toBeVisible();
    await row.click();

    // …and RESUMED at the turn it was left on, which is the claim a listing
    // cannot make by itself.
    await expect(b.locator(".decision .chain").first()).toBeVisible();
    expect(await chainOf(b), "the reopened game resumes where the closed one stopped").toBe(left);

    const moves = await playToOver(b, game, pick);
    expect(moves, "and it plays on, offline, to a winner").toBeGreaterThan(0);

    // **Exported.** The raw record is the file the terminal loads, and it is
    // reachable with no server: the bytes come off the Worker's own snapshot.
    const [download] = await Promise.all([
      b.waitForEvent("download"),
      b.getByRole("button", { name: "Save game file" }).click(),
    ]);
    expect(download.suggestedFilename()).toBe(`${game.id}.json`);
    const saved = fs.readFileSync((await download.path())!, "utf8");
    const record = JSON.parse(saved) as { events: unknown[]; terminal_digest?: unknown };
    expect(record.terminal_digest, "a finished record carries the engine's own digest").toBeTruthy();
    expect(
      record.events.length,
      "and it carries at least the moves the first session committed",
    ).toBeGreaterThanOrEqual(opening);

    expect(api.answered, "the whole reopened session answered nothing from the network").toEqual([]);
    // eslint-disable-next-line no-console
    console.log(
      `REOPEN: ${game.id} — ${opening} moves before the close, ${moves} after, ` +
        `${record.events.length} events exported, release ${release}`,
    );
    expect(watch.failures).toEqual([]);
  } finally {
    await first?.close();
    await second?.close();
    fs.rmSync(profile, { recursive: true, force: true });
  }
});

test("the update: a second release waits, nothing switches until the click, and the previous generation is kept — past a waiting one another replaced", async ({
  page,
}) => {
  const was = { marked: fs.readFileSync(MARKED), worker: fs.readFileSync(SW) };
  const firstName = cacheNameOf(was.worker.toString("utf8"));
  const watch = watchForFaults(page);
  /** Whatever the cell itself threw, held so the `finally` can restore AND
   *  check before anything propagates. */
  let failure: unknown = null;
  try {
    await page.goto("/");
    const one = await installed(page);
    expect(one, "the release on disk is the release in the cache").toBe(firstName);
    expect(await releaseCaches(page), "a fresh profile holds exactly one generation").toEqual([one]);

    // **The page has to be CONTROLLED before an update can wait, and that is a
    // fact about the platform rather than a step of convenience.** A new worker
    // waits only while some client is still being served by the old one — with
    // none, it activates immediately and there is nothing to offer. The visit
    // that installs the release is never controlled, because the worker does not
    // call `clients.claim()` (spec §2.3), so the reload here is what puts this
    // page in the state a returning player is in. Measured: without it the
    // second release installed, activated on its own and the door never showed
    // an offer — which is the platform behaving correctly for a tab nobody is
    // playing in.
    await page.reload();
    expect(await controlled(page), "a player's open tab is served by the worker").toBe(true);
    expect(await markOnScreen(page), "release 1 is the build's own manifest").toBeNull();

    // ---- release 2 -------------------------------------------------------
    const two = shipRelease(2);
    expect(two, "a changed file is a different release").not.toBe(one);
    await checkForUpdate(page);
    await expect(page.getByText("A new version is ready.")).toBeVisible({ timeout: INSTALL_WAIT_MS });

    // **Nothing has switched.** The offer is on screen and the controlling
    // worker is still release 1's, so the file this page reads is still the one
    // it cached — which is the claim spec §2.3 makes about an open tab.
    expect(
      await markOnScreen(page),
      "the running page still reads its OWN release while the new one waits",
    ).toBeNull();
    expect(
      await releaseCaches(page),
      "and release 2 is cached whole, beside the one being served",
    ).toEqual([one, two].sort());

    // The player asks for it. `applyUpdate` posts skip-waiting and waits for
    // `controllerchange` before it reloads, which is what makes the reload land
    // on the new worker rather than race it.
    //
    // **The wait is the LOAD EVENT and deliberately not a poll, which this cell
    // measured the hard way.** An `expect.poll` over `page.evaluate` started in
    // the same tick as the click left the reload unlanded for the whole 60 s
    // budget — the offer still on screen, the old controller still serving —
    // and the same click with a `load` wait instead lands it in about a second.
    // Whatever the exact mechanism (a CDP evaluate per 100 ms against a
    // document that is trying to navigate), the lesson generalises: do not
    // interrogate a page you have just asked to reload.
    await Promise.all([
      page.waitForEvent("load"),
      page.getByRole("button", { name: "Reload to use it" }).click(),
    ]);
    expect(await markOnScreen(page), "the reloaded page is served release 2").toBe(2);
    await expect(page.getByText("A new version is ready.")).toHaveCount(0);

    // **`activate` RETAINS the previous generation** (spec §2.3): a tab still
    // running release 1 asks for release 1's hashed module by name, and a sweep
    // that deleted it would take the bundle out from under a game in progress.
    await expect
      .poll(() => releaseCaches(page), { timeout: 30_000 })
      .toEqual([one, two].sort());

    // ---- release 3, which is what makes the retention above a reading ----
    //
    // A generation kept after ONE activation could be a sweep that never runs.
    // Release 3 is the other half: release 1 is now two generations back, and
    // it goes.
    const three = shipRelease(3);
    expect([one, two]).not.toContain(three);
    await checkForUpdate(page);
    await expect(page.getByText("A new version is ready.")).toBeVisible({ timeout: INSTALL_WAIT_MS });
    await Promise.all([
      page.waitForEvent("load"),
      page.getByRole("button", { name: "Reload to use it" }).click(),
    ]);
    expect(await markOnScreen(page), "the reloaded page is served release 3").toBe(3);
    await expect
      .poll(() => releaseCaches(page), { timeout: 30_000 })
      .toEqual([two, three].sort());

    // ---- releases 4 and 5: a WAITING release replaced before any click ----
    //
    // **PR #37's R-F5, and the case install-time ranking got wrong.** Release 4
    // installs and waits; release 5 ships before anyone clicks, and the platform
    // puts it in 4's place — 4's worker redundant, its cache left behind with
    // the NEWEST install stamp and no activation. The click then activates 5,
    // and "previous" has to mean the release this page was RUNNING, 3 — not 4,
    // which nothing ever ran. Ranked by install time, the sweep kept 4 and
    // deleted 3.
    const four = shipRelease(4);
    await checkForUpdate(page);
    await expect(page.getByText("A new version is ready.")).toBeVisible({ timeout: INSTALL_WAIT_MS });
    expect(
      await page.evaluate(async () => {
        const w = window as unknown as { __four?: ServiceWorker | null };
        w.__four = (await navigator.serviceWorker.getRegistration())?.waiting ?? null;
        return w.__four !== null;
      }),
      "release 4's worker is the one waiting",
    ).toBe(true);
    const five = shipRelease(5);
    expect(new Set([one, two, three, four, five]).size, "five releases, five names").toBe(5);
    await checkForUpdate(page);
    // **5 has taken 4's place**, read off the platform rather than assumed: the
    // worker that was waiting is redundant, and the one waiting now is another,
    // installed whole.
    await expect
      .poll(
        () =>
          page.evaluate(async () => {
            const was = (window as unknown as { __four?: ServiceWorker | null }).__four ?? null;
            const now = (await navigator.serviceWorker.getRegistration())?.waiting ?? null;
            return (
              was?.state === "redundant" && now !== null && now !== was && now.state === "installed"
            );
          }),
        { timeout: INSTALL_WAIT_MS },
      )
      .toBe(true);
    expect(
      await releaseCaches(page),
      "release 4's cache is left behind, never activated, beside release 5's",
    ).toEqual([two, three, four, five].sort());
    await Promise.all([
      page.waitForEvent("load"),
      page.getByRole("button", { name: "Reload to use it" }).click(),
    ]);
    expect(await markOnScreen(page), "the reloaded page is served release 5").toBe(5);
    await expect
      .poll(() => releaseCaches(page), { timeout: 30_000 })
      .toEqual([three, five].sort());

    // eslint-disable-next-line no-console
    console.log(
      `UPDATE: ${one} -> ${two} -> ${three}; release 1 swept, release 2 retained; ` +
        `${four} replaced waiting by ${five}, then clicked: release 3 (last ACTIVE) retained, ` +
        `2 and 4 swept`,
    );
    expect(watch.failures).toEqual([]);
  } catch (e) {
    // Held rather than thrown, so the restoration below runs and is CHECKED on
    // the failing path too — which is the path where it matters most.
    failure = e;
  } finally {
    // **The build goes back byte for byte, and the restoration is proven HERE**,
    // inside the `finally`, because `dist` is what the walk's server serves and
    // every spec after this one reads it. A check placed after the `try` would
    // be skipped by the very failure that makes it urgent: a cell that died
    // after `shipRelease(3)` would hand the rest of the run a release the build
    // never produced, with nothing saying so.
    //
    // **And `git status` is no evidence for any of this** — `web/dist/` is
    // gitignored (`.gitignore:50`), so it is clean whether the build was put
    // back or not. The evidence is these two reads of the files themselves.
    //
    // `scripts/precache.mjs` writes only `dist/sw.js`, so these two files are
    // the whole footprint of what this cell changes.
    fs.writeFileSync(MARKED, was.marked);
    fs.writeFileSync(SW, was.worker);
    const back = cacheNameOf(fs.readFileSync(SW, "utf8"));
    const same = fs.readFileSync(MARKED).equals(was.marked);
    if (back !== firstName || !same) {
      // Thrown rather than asserted, and it REPLACES the cell's own failure on
      // purpose: a poisoned `dist` breaks every spec that follows, so it is the
      // more urgent fact. The original is carried in the sentence so neither is
      // lost.
      throw new Error(
        `dist was NOT restored: sw.js names ${back} (expected ${firstName}), ` +
          `manifest.webmanifest ${same ? "is" : "is NOT"} its original bytes. ` +
          `Rebuild with \`npm run build:client\` before trusting any later spec.` +
          (failure ? ` The cell itself had already failed: ${String(failure)}` : ""),
      );
    }
  }
  if (failure) throw failure;
});

test("the tab that did not click: handed to release 2's worker still running release 1, it restarts its local host offline", async ({
  page: a,
  context,
}) => {
  // **What the design knew, and the rule it lacked** (PR #37 F2). Spec §2.3
  // KNEW the handover (lines 333–335): `skipWaiting()` is registration-wide, so
  // the click in tab A puts tab B in front of the NEW worker too — with B still
  // running release 1's bundle and asking for release 1's hashed names, which
  // release 2 does not list. It retained release 1's generation for exactly
  // that tab (lines 337–339). What it lacked was a fetch rule that READS that
  // generation: rule 3 (line 326) sent every unlisted path to the network. The
  // misreading was the CODE's — its comments said that because nothing calls
  // `clients.claim()`, such a tab stays with the old worker. This cell is the
  // first thing that asks whether anything reads the retained generation, at
  // the moment it matters most: B's local host restarting, offline, after the
  // origin has dropped the file.
  //
  // **Cell 3 could not see this, and not for want of trying.** Its releases
  // differ in the stable-named manifest, so every name release 1's bundle knows
  // is a name release 2 lists too; and it uses one page, which is the tab that
  // clicked.
  const before = distInventory();
  const was = { worker: fs.readFileSync(SW) };
  const firstName = cacheNameOf(was.worker.toString("utf8"));

  // **The local host's chunk, and its ONE reference**, found rather than
  // assumed, so a build that grew a second reference fails HERE by name rather
  // than shipping a release 2 that half-renamed it. `sw.js` names it too, and
  // is regenerated below rather than patched.
  const chunks = fs.readdirSync(ASSETS).filter((f) => /^worker-[\w-]+\.js$/.test(f));
  expect(chunks, "the build emits exactly one local-host chunk").toHaveLength(1);
  const oldName = chunks[0]!;
  const oldPath = `/assets/${oldName}`;
  const newPath = `/assets/${RENAMED}`;
  expect(
    fs.existsSync(path.join(ASSETS, RENAMED)),
    `the new name is not already taken — an interrupted earlier run left ${RENAMED} behind. ` +
      `Rebuild with \`npm run build:client\` before trusting any later spec.`,
  ).toBe(false);
  const referrers = [...before.keys()].filter(
    (f) => f !== "sw.js" && fs.readFileSync(path.join(DIST, f)).includes(oldName),
  );
  expect(referrers, "exactly one built file names the chunk").toHaveLength(1);
  expect(referrers[0], "and it is the bundle that starts the local host").toMatch(
    /^assets\/index-[\w-]+\.js$/,
  );
  const bundlePath = `/${referrers[0]!}`;
  const referrer = path.join(DIST, referrers[0]!);
  const referrerBytes = fs.readFileSync(referrer);
  const referrerText = referrerBytes.toString("utf8");
  expect(referrerText.split(oldName).length - 1, "and it names it exactly once").toBe(1);
  const chunkBytes = fs.readFileSync(path.join(ASSETS, oldName));

  const watchA = watchForFaults(a);
  /** Held so the `finally` restores and CHECKS before anything propagates —
   *  cell 3's reason, and its shape. */
  let failure: unknown = null;
  try {
    // ---- release 1, and TWO tabs it controls ------------------------------
    await a.goto("/");
    const one = await installed(a);
    expect(one, "the release on disk is the release in the cache").toBe(firstName);
    // The install visit is never controlled (cell 3 says why, measured); the
    // reload is what puts A in front of release 1's worker.
    await a.reload();
    expect(await controlled(a), "A is served by release 1's worker").toBe(true);

    const b = await context.newPage();
    const watchB = watchForFaults(b);
    /** Every local host B starts, by the path its script was asked for. */
    const started: string[] = [];
    b.on("worker", (w) => started.push(new URL(w.url()).pathname));
    await b.goto("/");
    expect(await controlled(b), "B loads in front of release 1's worker, as any returning tab does").toBe(
      true,
    );

    // B's game: local, held by B, played past its opening and left on a screen
    // with an option to click — which is what the restart below needs.
    const game = await startGame(b, { local: true, signedOut: true });
    const pick = rng(0x5245_5441); // "RETA"
    await playSome(b, game, pick, 6);
    await playUntilAnswerable(b, game, pick);
    const committed = await storedEvents(b, game.id);
    expect(committed, "B holds a played game to go on with").toBeGreaterThan(0);
    expect(started, "B's local host is release 1's chunk, and so far the only one").toEqual([oldPath]);

    // **Release 1's worker, held by B's own window.** Two readings come off it
    // after the update: that B's document was never replaced — the property
    // survives only in the document that set it — and that B's controller by
    // then is a DIFFERENT worker from this one.
    await b.evaluate(() => {
      const w = window as unknown as { __release1?: ServiceWorker | null; __handovers?: number };
      w.__release1 = navigator.serviceWorker.controller;
      w.__handovers = 0;
      navigator.serviceWorker.addEventListener("controllerchange", () => {
        w.__handovers = (w.__handovers ?? 0) + 1;
      });
    });

    // ---- release 2: the local host's chunk under a NEW name ---------------
    fs.renameSync(path.join(ASSETS, oldName), path.join(ASSETS, RENAMED));
    fs.writeFileSync(referrer, referrerText.replace(oldName, RENAMED));
    execFileSync("node", ["scripts/precache.mjs"], { cwd: path.join(HERE, ".."), stdio: "pipe" });
    const listing = fs.readFileSync(SW, "utf8");
    const two = cacheNameOf(listing);
    expect(two, "a renamed file is a different release").not.toBe(one);
    expect(listing.includes(oldName), "release 2 does not list release 1's chunk").toBe(false);
    expect(listing.includes(RENAMED), "and does list its own").toBe(true);
    // Asked of the server from OUTSIDE the browser, so no cache of any kind
    // stands between the question and the origin.
    expect((await fetch(`${BASE_URL}${oldPath}`)).status, "release 1's chunk is gone from the origin").toBe(
      404,
    );
    expect((await fetch(`${BASE_URL}${newPath}`)).status, "and release 2's is on it").toBe(200);

    // ---- A takes the update ------------------------------------------------
    /** Every local host A starts from here on. */
    const aStarted: string[] = [];
    a.on("worker", (w) => aStarted.push(new URL(w.url()).pathname));
    await checkForUpdate(a);
    await expect(a.getByText("A new version is ready.")).toBeVisible({ timeout: INSTALL_WAIT_MS });
    expect(await releaseCaches(a), "release 2 is cached whole, beside release 1").toEqual(
      [one, two].sort(),
    );
    // The LOAD event and not a poll — cell 3 measured why.
    await Promise.all([
      a.waitForEvent("load"),
      a.getByRole("button", { name: "Reload to use it" }).click(),
    ]);
    // **A is on release 2, read off the bundle it RUNS**: its door starts the
    // local host from the new name, which only release 2's bundle knows.
    await expect.poll(() => aStarted.includes(newPath), { timeout: 30_000 }).toBe(true);
    expect(aStarted, "and never from release 1's").not.toContain(oldPath);
    await expect
      .poll(() => releaseCaches(a), { timeout: 30_000 })
      .toEqual([one, two].sort());

    // ---- B: never reloaded, and in front of release 2's worker -------------
    //
    // **The finding, read off the platform.** `controllerchange` fired in B,
    // which did not click; the worker B held is redundant; and B's controller
    // is the registration's ACTIVE worker — the one A reloaded onto. The two
    // workers share a script URL (`/sw.js`), so a URL comparison would say
    // nothing; object identity does, because a window hands back the same
    // `ServiceWorker` object for the same worker.
    await expect
      .poll(
        () => b.evaluate(() => (window as unknown as { __handovers?: number }).__handovers ?? -1),
        { timeout: 30_000 },
      )
      .toBe(1);
    const handed = await b.evaluate(async () => {
      const w = window as unknown as { __release1?: ServiceWorker | null };
      const now = navigator.serviceWorker.controller;
      const active = (await navigator.serviceWorker.getRegistration())?.active ?? null;
      return {
        sameDocument: w.__release1 !== undefined,
        changed: now !== null && now !== w.__release1,
        isActive: now !== null && now === active,
        oldState: w.__release1?.state ?? null,
      };
    });
    expect(handed.sameDocument, "B was never reloaded").toBe(true);
    expect(handed.changed, "B's controller is no longer the worker it loaded under").toBe(true);
    expect(handed.isActive, "it is the registration's active worker, the one A reloaded onto").toBe(
      true,
    );
    expect(handed.oldState, "and release 1's worker is gone").toBe("redundant");

    // ---- no HTTP cache, and no network -------------------------------------
    //
    // **The HTTP cache goes FIRST, because it MASKS the defect — measured.**
    // Release 1's install fetched every entry with `cache: "reload"`, which
    // writes the HTTP cache, and the server serves `/assets/` `immutable` — so a
    // request the worker let through to "the network" can be answered from
    // there without the network at all. With this one call removed, the cell
    // PASSED against the pre-fix worker: the chunk was gone from the origin and
    // the network was off, so by elimination the HTTP cache served it. The
    // clear takes that and nothing else, which is asserted — Cache Storage is
    // the thing under test.
    const cdp = await context.newCDPSession(b);
    await cdp.send("Network.clearBrowserCache");
    expect(await releaseCaches(b), "clearing the HTTP cache left both generations").toEqual(
      [one, two].sort(),
    );
    expect(await cacheHas(b, one, oldPath), "release 1's chunk is in the retained generation").toBe(true);
    expect(await cacheHas(b, two, oldPath), "and not in release 2's").toBe(false);
    await context.setOffline(true);

    // **Which worker B's controller is, asked of something only ONE of them
    // could answer that way** — the second reading, independent of object
    // identity. The bundle's path is the SAME in both releases and its bytes
    // are not: release 2's names the new chunk. With no network and no HTTP
    // cache, a fetch through B's controller is one of the two workers answering
    // from its own generation, and the body says which.
    const served = await b.evaluate(
      (at) =>
        fetch(at, { cache: "no-store" }).then(
          (r) => r.text(),
          (e: unknown) => `failed: ${String(e)}`,
        ),
      bundlePath,
    );
    expect(served.includes(RENAMED), "B's controller serves release 2's bundle").toBe(true);

    // ---- B's local host restarts, and asks for release 1's name ------------
    //
    // A REAL trap through the module's own `debug_trap`, exactly as
    // `plays_locally.spec.ts`'s trap cell raises one (`npm run e2e` builds the
    // debug module): the next call dies, the page throws the Worker away and
    // starts a fresh one from the URL ITS bundle carries — release 1's.
    const host = b.workers().find((w) => new URL(w.url()).pathname === oldPath);
    expect(host, "B's running local host is release 1's chunk").toBeDefined();
    await host!.evaluate(() => {
      (self as unknown as { onmessage: (e: { data: unknown }) => void }).onmessage({
        data: { seq: 987_654, kind: "debug_trap" },
      });
    });
    await b.locator("button.option:not([disabled])").first().click();
    await expect(b.locator(".decision.notices .error")).toContainText("the local host restarted");
    await expect
      .poll(() => started.length, { message: "B started a fresh local host", timeout: 30_000 })
      .toBe(2);
    expect(started[1], "from release 1's name: the only one B's bundle knows").toBe(oldPath);
    expect(await storedEvents(b, game.id), "nothing committed was lost").toBe(committed);

    // **And B's game goes on, offline.** The script the fresh host was started
    // from exists nowhere now but in release 1's retained generation — gone
    // from the origin, absent from release 2's list, out of the HTTP cache, and
    // the network off — so a move COMMITTED here is the retained generation
    // answering.
    await expect(b.getByText("This game was opened in another tab")).toHaveCount(0);
    const now = await playUntilCommitted(b, game, pick, committed);
    expect(now, "a move committed after the restart").toBeGreaterThan(committed);

    // eslint-disable-next-line no-console
    console.log(
      `RETAINED: ${one} -> ${two}; ${oldName} renamed ${RENAMED}; B, never reloaded, restarted ` +
        `its local host offline from ${oldPath} and committed event ${now} (was ${committed})`,
    );
    expect(watchA.failures).toEqual([]);
    expect(watchB.failures).toEqual([]);
  } catch (e) {
    failure = e;
  } finally {
    // **Every file this cell renamed, patched or regenerated goes back byte for
    // byte, and the WHOLE tree is proven against its own inventory HERE**,
    // inside the `finally` — cell 3's reason: `dist` is what the walk's server
    // serves and every later spec reads it, and `git status` says nothing about
    // a gitignored directory. Proving the whole tree rather than the four files
    // also says the cell changed nothing it did not mean to.
    fs.writeFileSync(path.join(ASSETS, oldName), chunkBytes);
    fs.rmSync(path.join(ASSETS, RENAMED), { force: true });
    fs.writeFileSync(referrer, referrerBytes);
    fs.writeFileSync(SW, was.worker);
    const drift = inventoryDrift(before, distInventory());
    const back = cacheNameOf(fs.readFileSync(SW, "utf8"));
    if (drift.length > 0 || back !== firstName) {
      throw new Error(
        `dist was NOT restored: ${drift.join("; ") || "every file is its original bytes"}; ` +
          `sw.js names ${back} (expected ${firstName}). ` +
          `Rebuild with \`npm run build:client\` before trusting any later spec.` +
          (failure ? ` The cell itself had already failed: ${String(failure)}` : ""),
      );
    }
  }
  if (failure) throw failure;
});

test("a restore on a fresh profile lands four verdicts, and the cross-rev pair is LISTED with its reason", async ({
  page,
  context,
}) => {
  // This cell is about the envelope and not about the shell, so it costs no
  // precache — the same flag the `./sw` fixture sets for every other spec.
  await context.addInitScript(NO_SERVICE_WORKER);
  const watch = watchForFaults(page);

  const t0 = Date.now();
  await page.goto("/");
  // Anti-vacuity: an empty profile, said by the screen itself. Without this,
  // every row below could be a game some earlier cell left behind.
  await expect(page.getByText("no game on this device yet")).toBeVisible();
  const doorAt = Date.now() - t0;

  const t1 = Date.now();
  const report = await restore(page, ENVELOPE);
  const restoreAt = Date.now() - t1;
  expect([...report.keys()].sort(), "one row per game in the file").toEqual([
    "local-242a5429453c593c",
    "local-another-rev",
    "local-at-the-deal",
    "local-not-a-record",
  ]);
  expect(report.get("local-242a5429453c593c")).toBe("restored");
  expect(report.get("local-at-the-deal"), "a game with a table and no record is a game").toBe(
    "restored",
  );
  expect(report.get("local-another-rev")).toBe(
    "restored, but it will not open here — saved at semantics rev 6; this app plays rev 7",
  );
  expect(report.get("local-not-a-record")).toContain("not restored — the record does not parse");

  // **The LISTING is a different claim from the report** (spec §2.5). The
  // report is what one restore said; this is what the door says on every visit
  // afterwards, and it exists because the layer's own listing skips a document
  // whose shape check fails — so without the compatibility row the game would
  // be in no list at all.
  const will = page.locator(".unplayable");
  await expect(will.getByRole("heading", { name: "Will not open here" })).toBeVisible();
  await expect(will.locator("li")).toHaveCount(1);
  await expect(will.locator("li")).toContainText("local-another-rev");
  await expect(will.locator("li")).toContainText("saved at semantics rev 6");

  // …and it is listed ONCE: the playable rows are the two that open here.
  await expect(page.locator(".on-this-device > ul > li")).toHaveCount(2);
  await expect(
    page.locator(".on-this-device > ul"),
    "and it is listed ONCE: it is not also among the games that do open here",
  ).not.toContainText("local-another-rev");

  // **The finished game opens at game over**, which is what "restored" has to
  // mean: the pair came out of a file, went into this browser's store, and the
  // module read both.
  await page.goto("/#/t/local-242a5429453c593c");
  await expect(page.getByText("The game is over.")).toBeVisible();
  await expect(page.locator("table.outcome tbody tr")).toHaveCount(2);

  // **And the game restored from a TABLE DOCUMENT ALONE is played**, which is
  // the half that says a record is genuinely optional in the envelope: this
  // pair arrived with no record at all, and the module built a live game out of
  // the setup the document carries.
  //
  // It opens at the GAME and not at the deal, which is the fixture and not a
  // surprise: its `keeps` are already filled, so there is no card left to
  // choose and the layer hands over a session. Written down because the
  // document's `status` still reads `dealing` and a reader would expect the
  // deal screen.
  //
  // **Which screen it is has to be asked of something only ONE of them has.**
  // `atTheGame` and the banner's `.meta` are both satisfied by the deal screen
  // too — `Dealing.tsx` renders the same `header.banner`, the same `<h1>` and
  // the same id — so they say the page loaded and nothing more. **Exactly one
  // of the two reads below is exclusive** (whole-branch review, false sentence
  // 11; ledger Task 10 L225): the game root is the one `.app` without
  // `setup-table`, which is `harness.ts`'s own "am I in a game" locator. The
  // count-0 read of "Choose your card" is NOT — `Dealing.tsx` gates that
  // heading, so a deal screen can be drawn without it and the read would pass
  // on the screen it is meant to exclude. It is kept as a cheap sanity read
  // and claims nothing. **The WITNESS that this is playable is the move loop
  // underneath them**: the deal screen has no option rows and no pickable
  // tiles, so `act` would answer `null` and `deadEnd` would throw, which is
  // why `playSome === 3` is the assertion that carries the claim.
  await page.goto("/#/t/local-at-the-deal");
  await expect(atTheGame(page)).toBeVisible();
  await expect(page.locator(".app:not(.setup-table)")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Choose your card" })).toHaveCount(0);
  await expect(page.locator("header .meta").first()).toContainText("local-at-the-deal");
  const played = await playSome(page, { id: "local-at-the-deal" }, rng(0x5245_5354), 3); // "REST"
  expect(played, "and a restored game takes moves like any other").toBe(3);

  // **RECORDED, never gated** — and recorded because this cell's whole wall
  // reads as a fraction of a second and that is surprising enough to be worth
  // decomposing. Four games validated by the module, a 595-event strict replay
  // among them, and it is the page load that dominates.
  // eslint-disable-next-line no-console
  console.log(`RESTORE: the door was up in ${doorAt} ms, the four-game restore took ${restoreAt} ms`);
  expect(watch.failures).toEqual([]);
});

test("the clash: the same bytes are already here, different bytes under one name are refused by row", async ({
  page,
  context,
}) => {
  await context.addInitScript(NO_SERVICE_WORKER);
  const watch = watchForFaults(page);
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "yokohama-e2e-envelope-"));
  const walls: number[] = [];
  /** One restore, with its own wall RECORDED — see the printed line below. */
  const timed = async (file: string) => {
    const at = Date.now();
    const rows = await restore(page, file);
    walls.push(Date.now() - at);
    return rows;
  };
  try {
    const t0 = Date.now();
    await page.goto("/");
    const first = await timed(ENVELOPE);
    const doorAndFirst = Date.now() - t0;
    expect(first.get("local-242a5429453c593c"), "the first restore lands it").toBe("restored");

    // **Different BYTES under the same name, which is exactly what the rule
    // compares** (spec §2.5): present and byte-identical is "already here",
    // present and different is a refusal. A trailing newline is the smallest
    // change that still leaves the record VALID — a corrupted one would be
    // rejected by the validator and never reach the clash check at all, which
    // is a different refusal and would prove nothing about this one.
    const envelope = JSON.parse(fs.readFileSync(ENVELOPE, "utf8")) as {
      games: { id: string; record: string | null }[];
    };
    const target = envelope.games.find((g) => g.id === "local-242a5429453c593c")!;
    expect(target.record, "the game being mutated has a record to mutate").not.toBeNull();
    target.record = `${target.record!}\n`;
    const mutated = path.join(scratch, "mutated.json");
    fs.writeFileSync(mutated, JSON.stringify(envelope, null, 2));

    const second = await timed(mutated);
    expect(
      second.get("local-242a5429453c593c"),
      "different bytes under a name this device holds are REFUSED, and the row says which",
    ).toBe("not restored — a different game is already on this device under that name");
    // **The contrast is inside ONE report**, which is what says the refusal is
    // about the bytes: three games in the same file were not touched and read
    // as already here.
    expect(second.get("local-at-the-deal")).toBe("already on this device — nothing changed");
    expect(second.get("local-another-rev")).toBe("already on this device — nothing changed");

    // And the original file, run a third time, reads "already here" for the
    // very game that clashed — so the answer follows the bytes and not the
    // number of times a restore has been run.
    const third = await timed(ENVELOPE);
    expect(third.get("local-242a5429453c593c")).toBe("already on this device — nothing changed");

    // **RECORDED, never gated.** This cell's wall reads as a fifth of a second
    // and that is worth decomposing rather than leaving as a figure a reader
    // cannot believe: the door and the first restore together are most of it,
    // and the two restores after it are the same four games judged again by a
    // module that is already up.
    // eslint-disable-next-line no-console
    console.log(
      `CLASH: the door and the first restore took ${doorAndFirst} ms; ` +
        `the three restores themselves ${walls.join(" / ")} ms`,
    );
    expect(watch.failures).toEqual([]);
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
});

test("the outbox: a game finished before the archive was ever heard of waits, and coming back online sends it", async ({
  page,
  context,
}) => {
  // **The name is SEEDED rather than typed, and that is this cell's one
  // deviation.** The sign-in screen stands on the lobby, and reaching the
  // lobby is one of the three health read points — so typing into it would
  // land the very answer whose ABSENCE the cell is about. What is being tested
  // here is the outbox, not the door, and `signIn` has its own cells.
  await context.addInitScript(
    ([key, who]) => {
      try {
        window.localStorage.setItem(key!, who!);
      } catch {
        /* a browser that cannot store is the fault the assertions will name */
      }
    },
    [CLAIMED_USER_KEY, WALKER],
  );
  const watch = watchForFaults(page);
  const api = watchTheApi(page);

  await page.goto("/");
  await installed(page);
  // **Nothing has been read from the server**, which is the state that makes
  // "not yet known" reachable: the front door asks the API for nothing
  // (R-6.5b2.7), so this browser has never been told whether the deployment
  // keeps an archive.
  expect(api.answered, "the door asked the server nothing").toEqual([]);

  await context.setOffline(true);
  const game = await startGame(page, { local: true, signedOut: true });
  const moves = await playToOver(page, game, rng(0x4f55_5442)); // "OUTB"
  expect(moves, "a whole game was finished with no connection").toBeGreaterThan(20);

  // The row, on the door, in the words spec §4 gives it.
  await page.goto("/#/");
  await expect(page.getByRole("heading", { name: "Waiting to reach the archive" })).toBeVisible();
  const row = page.locator(".outbox-row").filter({ hasText: game.id });
  await expect(row).toHaveCount(1);
  await expect(row).toContainText("not yet known");
  await expect(row, "and the reason is the row's own class, not only its words").toHaveClass(
    /\bunknown\b/,
  );

  // **Whether the browser tells the page it is back**, observed rather than
  // assumed. `setOffline(false)` is a devtools emulation and the `online` event
  // is the trigger spec §2.6 names, so a run where it does not fire is a
  // reading worth having rather than a silent fallback.
  await page.evaluate(() => {
    const w = window as unknown as { __wentOnline?: number };
    w.__wentOnline = 0;
    window.addEventListener("online", () => {
      w.__wentOnline = (w.__wentOnline ?? 0) + 1;
    });
  });
  const beforeOnline = api.answered.length;
  expect(beforeOnline, "nothing had been sent while the network was off").toBe(0);

  await context.setOffline(false);
  const real = await page
    .waitForFunction(() => ((window as unknown as { __wentOnline?: number }).__wentOnline ?? 0) > 0, {
      timeout: 15_000,
    })
    .then(
      () => true,
      () => false,
    );
  // **Recorded where the REPORTER carries it**, and not only in a console line
  // nobody scans: if this run's drain was started by the test rather than by
  // the browser, the reader of the run has to be able to see that without
  // grepping stdout.
  test.info().annotations.push({
    type: "outbox-online-trigger",
    description: real
      ? "Chromium fired `online` in the page after setOffline(false); the drain is the browser's"
      : "Chromium did NOT fire `online` after setOffline(false) — the event was DISPATCHED BY HAND, " +
        "so this run does not witness the browser's own trigger",
  });
  if (!real) {
    await page.evaluate(() => window.dispatchEvent(new Event("online")));
  }

  // **Nobody clicked anything.** The only thing that happened between the row
  // being drawn and the assertion below is the network coming back: no control
  // on this page was pressed, and the POST is the drain's.
  await expect
    .poll(() => api.answered.filter((c) => c.startsWith("POST /api/archive")), { timeout: 60_000 })
    .toHaveLength(1);
  expect(
    api.answered.find((c) => c.startsWith("POST /api/archive")),
    "and the server took it",
  ).toMatch(/ 20[01]$/);

  // The copy is on the server's disk, under the layout the archive chose from
  // the record it replayed — nothing in the body said where it should go.
  const at = path.join(archiveDir(), "browser", SEGMENT, REV, "over", `${game.id}.json`);
  await expect.poll(() => fs.existsSync(at), { timeout: 30_000 }).toBe(true);
  expect(fs.existsSync(at.replace(/\.json$/, ".table.json")), "the pair landed whole").toBe(true);

  // …and the row is gone, because the receipt says it landed.
  await expect(page.locator(".outbox")).toHaveCount(0);

  // **"Exactly one" is re-read at the END, which is what makes it exactly
  // one.** The poll above stops the instant the count reaches 1, so a second
  // POST arriving during the file check and the row-gone wait would never have
  // been looked at — and a drain that posted the same bytes twice is precisely
  // what the receipt exists to prevent.
  expect(
    api.answered.filter((c) => c.startsWith("POST /api/archive")),
    "still exactly one archive POST, now that the row has gone and the copy is on disk",
  ).toHaveLength(1);

  // eslint-disable-next-line no-console
  console.log(
    `OUTBOX: ${game.id} finished offline in ${moves} clicks, waited as "not yet known", ` +
      `and was sent by the ${real ? "browser's own `online` event" : "dispatched `online` event"} ` +
      `in ${api.answered.length} API requests`,
  );
  expect(watch.failures).toEqual([]);
});

test("a refused registration: the first /sw.js answers 503, the door names it in ONE sentence, and Try again makes the release ready", async ({
  page,
  context,
}) => {
  const watch = watchForFaults(page);
  let scripts = 0;
  await context.route("**/sw.js", async (route) => {
    scripts += 1;
    if (scripts === 1) await route.fulfill({ status: 503, body: "temporary" });
    else await route.continue();
  });

  await page.goto("/");
  await expect(page.getByRole("button", { name: "Try again", exact: true })).toBeVisible({
    timeout: INSTALL_WAIT_MS,
  });
  const refused = await doorLine(page);
  // **The browser's own sentence, and ONE full stop**: Chromium's ends in a
  // period and the door appends one, which read "…the script.. Try again"
  // until the fix round (measured, the review's own script).
  expect(refused).toContain("Could not prepare to play offline — Failed to register a ServiceWorker");
  expect(refused).toContain("(503)");
  expect(refused).toMatch(/[^.]\. Try again$/);
  expect(scripts, "the refusal really was the registration's own script fetch").toBe(1);

  // **The retry REGISTERS, and what that resolves to is adopted**: the door
  // leaves "Preparing…" for "Ready…" and the release is whole in the cache.
  expect(await retried(page)).toBe("Ready to play offline.");
  const release = await installed(page);
  expect(scripts, "the retry fetched the worker again").toBe(2);

  // eslint-disable-next-line no-console
  console.log(`REFUSED REGISTRATION: retried into ${release} after "${refused}"`);
  // **Chromium logs the arranged refusal itself** as a console error, in the
  // same words the door carries — so that ONE sentence is this cell's own
  // doing, and is the only line let through; the shared watch's network filter
  // is left as it is for the cells it was written for.
  const arranged = "console.error: A bad HTTP response code (503) was received when fetching the script.";
  expect(
    watch.failures.filter((f) => f !== arranged),
    "the page threw nothing while it failed and retried",
  ).toEqual([]);
});

test("a failed FIRST install: one file answers 503 once, the release fails whole, and Try again registers again and makes it ready", async ({
  page,
  context,
}) => {
  const watch = watchForFaults(page);
  // A file the door never draws, so the one refusal is the WORKER's fetch and
  // not the page's; asserted to be in this release's list, so a renamed art
  // file fails here by name rather than as an install that never failed.
  const refusedPath = "/art/achievements/Copper5.webp";
  expect(fs.readFileSync(SW, "utf8"), "the refused file is one the release precaches").toContain(
    JSON.stringify(refusedPath),
  );
  let asks = 0;
  await context.route(`**${refusedPath}`, async (route) => {
    asks += 1;
    if (asks === 1) await route.fulfill({ status: 503, body: "temporary" });
    else await route.continue();
  });
  let scripts = 0;
  await context.route("**/sw.js", async (route) => {
    scripts += 1;
    await route.continue();
  });

  await page.goto("/");
  await expect(page.getByRole("button", { name: "Try again", exact: true })).toBeVisible({
    timeout: INSTALL_WAIT_MS,
  });
  const failed = await doorLine(page);
  expect(failed).toBe(`Could not prepare to play offline — ${refusedPath}: answered 503. Try again`);
  // **E1's premise, read off the platform**: a failed FIRST install leaves no
  // newest worker, so the registration is gone from the map — which is why
  // `update()` on the page's copy of it is refused.
  expect(
    await page.evaluate(async () => (await navigator.serviceWorker.getRegistration()) === undefined),
    "the failed first install left no registration behind",
  ).toBe(true);
  expect(await releaseCaches(page), "and no half-release in the cache").toEqual([]);

  // Before the fix this line read "…with script ('Unknown'): The object is in
  // an invalid state.. Try again" (measured at 0a4f7223, two periods, the
  // door's own defect as well), on every click until a reload.
  expect(await retried(page)).toBe("Ready to play offline.");
  const release = await installed(page);
  expect(scripts, "the retry REGISTERED — it fetched the worker again").toBe(2);
  expect(asks, "and fetched the refused file again").toBe(2);

  // eslint-disable-next-line no-console
  console.log(`FAILED FIRST INSTALL: retried into ${release} after "${failed}"`);
  expect(watch.failures, "the page threw nothing while it failed and retried").toEqual([]);
});

// ----------------------------------------------------- cells 8 and 9's tools

/** The door's readiness line as it reads now. */
const doorLine = (page: Page): Promise<string> => page.locator(".offline-state p").first().innerText();

/**
 * Press "Try again" and answer the line the door SETTLES on — ready, or failed
 * again — so a retry that fails names its sentence at once instead of timing
 * out on "Ready…" two and a half minutes later.
 *
 * **The line on screen is marked before the click**, for `restore`'s reason:
 * every readiness state is its own `<p>`, so the failed line is unmounted and
 * the one that comes back is a new element carrying no mark — a wait that
 * could otherwise read the failure it is retrying.
 */
async function retried(page: Page): Promise<string> {
  await page.evaluate(() => {
    document.querySelector(".offline-state p")?.setAttribute("data-e2e-stale", "");
  });
  await page.getByRole("button", { name: "Try again", exact: true }).click();
  const fresh = page.locator(".offline-state p:not([data-e2e-stale])").first();
  await expect(fresh).toHaveText(/^(Ready to play offline\.|Could not prepare to play offline)/, {
    timeout: INSTALL_WAIT_MS,
  });
  return fresh.innerText();
}

// ----------------------------------------------------- cell 3's own tools

/** The release a built worker carries, read out of the generator's own banner
 *  and the substituted constant. */
function cacheNameOf(worker: string): string {
  const found = new RegExp(`${CACHE_PREFIX}[0-9a-f]{16}`).exec(worker);
  if (!found) throw new Error("no release name in dist/sw.js — was it built by precache.mjs?");
  return found[0];
}

/** Put a marked copy of the manifest in `dist` and rebuild the worker over it,
 *  which is a whole new release: the cache name is a digest over every file's
 *  bytes, so one changed file is one changed name with nothing to bump. */
function shipRelease(mark: number): string {
  const manifest = JSON.parse(fs.readFileSync(MARKED, "utf8")) as Record<string, unknown>;
  manifest["yokohama_e2e_release"] = mark;
  fs.writeFileSync(MARKED, `${JSON.stringify(manifest, null, 2)}\n`);
  execFileSync("node", ["scripts/precache.mjs"], { cwd: path.join(HERE, ".."), stdio: "pipe" });
  return cacheNameOf(fs.readFileSync(SW, "utf8"));
}

/** Which release the PAGE is being served, read through whatever worker is
 *  controlling it.
 *
 *  `cache: "no-store"` so an UNCONTROLLED page would read the server's current
 *  bytes and fail loudly, rather than pass from the browser's own HTTP cache
 *  while proving nothing about the worker. */
const markOnScreen = (page: Page): Promise<number | null> =>
  page.evaluate(async () => {
    const r = await fetch("/manifest.webmanifest", { cache: "no-store" });
    const body = (await r.json()) as { yokohama_e2e_release?: number };
    return body.yokohama_e2e_release ?? null;
  });

/** Ask the browser to re-fetch `sw.js`. It is served `no-store`, so the check
 *  is a real byte comparison rather than one against a cached copy. */
const checkForUpdate = (page: Page): Promise<void> =>
  page.evaluate(async () => {
    const reg = await navigator.serviceWorker.getRegistration();
    await reg?.update();
  });

// ---------------------------------------------------- cell 7's own tools

/** Every file under `dist`, by its path there, with its SHA-256 — what cell 7
 *  proves its restoration against. */
function distInventory(): Map<string, string> {
  const out = new Map<string, string>();
  const visit = (dir: string) => {
    for (const name of fs.readdirSync(dir).sort()) {
      const full = path.join(dir, name);
      if (fs.statSync(full).isDirectory()) visit(full);
      else {
        const at = path.relative(DIST, full).split(path.sep).join("/");
        out.set(at, createHash("sha256").update(fs.readFileSync(full)).digest("hex"));
      }
    }
  };
  visit(DIST);
  return out;
}

/** How `now` differs from `was`, one sentence per file; empty when the two
 *  trees are the same bytes under the same names. */
function inventoryDrift(was: Map<string, string>, now: Map<string, string>): string[] {
  const out: string[] = [];
  for (const [at, digest] of was) {
    const found = now.get(at);
    if (found === undefined) out.push(`${at} is missing`);
    else if (found !== digest) out.push(`${at} is not its original bytes`);
  }
  for (const at of now.keys()) if (!was.has(at)) out.push(`${at} was not there before`);
  return out;
}

/** How many events this browser has COMMITTED for a game, read from the page's
 *  own IndexedDB — `plays_locally.spec.ts`'s `storedEvents`, restated because
 *  importing a spec file would register its tests here. `-1` where no record is
 *  stored yet. */
async function storedEvents(page: Page, id: string): Promise<number> {
  const record = await page.evaluate(
    (game) =>
      new Promise<string | null>((resolve, reject) => {
        const open = indexedDB.open("yokohama-local");
        open.onerror = () => reject(new Error("IndexedDB would not open"));
        open.onsuccess = () => {
          const db = open.result;
          if (!db.objectStoreNames.contains("docs")) {
            reject(new Error("no `docs` store — has the Worker ever run?"));
            return;
          }
          const t = db.transaction("docs", "readonly");
          const rec = t.objectStore("docs").get(["record", game]) as IDBRequest<{ text: string } | undefined>;
          t.oncomplete = () => resolve(rec.result?.text ?? null);
          t.onerror = () => reject(t.error ?? new Error("the read failed"));
        };
      }),
    id,
  );
  if (record === null) return -1;
  return (JSON.parse(record) as { events: unknown[] }).events.length;
}

/** Play until the screen offers an OPTION ROW, so the click that meets the
 *  trap has something to click — `plays_locally.spec.ts`'s helper. */
async function playUntilAnswerable(page: Page, game: Game, pick: () => number): Promise<void> {
  for (let i = 0; i < 40; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before the cell could act");
    if (screen.options > 0) return;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, i);
  }
  throw new Error("forty moves and the screen never offered an option row");
}

/** Click until this browser COMMITS another event, and answer the new count —
 *  `plays_locally.spec.ts`'s helper, and its reason: a click is not a move (an
 *  option row can open a sub-picker), so "play went on" is asked of the STORE. */
async function playUntilCommitted(
  page: Page,
  game: Game,
  pick: () => number,
  from: number,
): Promise<number> {
  for (let i = 0; i < 12; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before another move was committed");
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, i);
    await readScreen(page);
    const now = await storedEvents(page, game.id);
    if (now > from) return now;
  }
  throw new Error(`twelve clicks and nothing past event ${from} was committed`);
}
