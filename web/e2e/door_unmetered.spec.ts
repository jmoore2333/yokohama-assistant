// **The door unmetered: the fence's client half, in a real browser** (M6.5c2
// cycle F, R-F5; spec §2.5, Review Focus 5).
//
// The fence, stated once: on the Worker a door visit with nothing owed, a
// local game to its end, and any request for a path the release does not
// serve cost zero invocations; the first metered request is `/enter` behind
// the multiplayer button, a link arrival's first API read, or a finished local
// game's archive upload. The ledger of every metered action is
// `docs/web-lane.md` §16. These three cells hold the CLIENT to it against the
// local binary; the Worker's half is the live reading (the probe,
// `worker/scripts/probe-burst.mjs`, and the Observability query API).
//
//   (a) the door, loaded cold with the release's worker ON, asks for nothing
//       but the release's own files — the precache list `web/dist/sw.js`
//       carries, plus `/`, `/sw.js` and `/shell.bin` — and nothing under
//       `/api/`, no `/enter`, no unknown path; the worker's own install
//       requests are read too (the context's, not only the page's);
//   (b) a local game against the bot, played to its end with no sign-in, asks
//       the API nothing until the move that ends it, and then at most the
//       health read and the archive — here exactly ONE health read (a fresh
//       browser does not know this door) and NO post: under the open door a
//       visitor with no name is refused for certain, so the outbox writes the
//       game down "not signed in" and posts nothing (spec §9 F-14);
//   (c) a miss — `/wp-login.php` — is the 404 page (title "Nothing here") to a
//       fetch and to a navigation in a context no worker controls, while the
//       same navigation in a context the worker CONTROLS is answered with the
//       shell (`swRoute.ts`'s rule 2): the client's routes live in the hash,
//       so a controlled browser never sees the 404 page.
//
// @yokohama-service-worker: on
//
// **The worker is ON here, deliberately**, which is why this spec takes `test`
// from `@playwright/test` and says so in the marker above: the door's fence is
// about what a REAL visitor's browser asks, and a real visitor's browser
// installs the release. `every_spec_declares_its_worker.spec.ts` names the
// specs that may carry `on`; this is the second, beside the offline cells.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test, type BrowserContext, type Page } from "@playwright/test";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { readPrecacheList } from "../../worker/scripts/precache-list.mjs";
import { CAP } from "./caps";
import { act, capReached, deadEnd, readScreen, rng, startGame, watchForBreakage } from "./harness";
import { newContextWithoutWorker } from "./sw";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly — the guard
// `every_spec_can_skip.spec.ts` reads this directory for.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const HERE = path.dirname(fileURLToPath(import.meta.url));

/** How long a cold install is given — the offline cells' budget, and its
 *  reason: the worker's own deadline plus the page's boot. */
const INSTALL_WAIT_MS = 150_000;

/** How long the door is watched after its install, for a request it would
 *  make on its own a moment later. A reading of the door's first seconds, not
 *  a proof about for ever: the update check's 15-minute interval is
 *  `swRegister.ts`'s, and it asks for `/sw.js`, a release file. */
const SETTLE_MS = 3_000;

/** The three files the door asks for that the precache list does not name:
 *  the document itself, the worker's script, and the shell the worker
 *  installs from (spec §2.5). */
const DOOR_FILES: ReadonlySet<string> = new Set(["/", "/sw.js", "/shell.bin"]);

/** A scanner's favourite, and a path the release does not serve. */
const MISS = "/wp-login.php";

/** The 404 page's title (`web/public/404.html`, cycle F's Task 5a). */
const NOT_FOUND_TITLE = "Nothing here";

/** The release's precache list, read out of the BUILT worker the walk's
 *  server serves — through the one reader both deploy scripts use. */
function precached(): Set<string> {
  const worker = fs.readFileSync(path.join(HERE, "..", "dist", "sw.js"), "utf8");
  return new Set(readPrecacheList(worker, "built").map((e) => e.path));
}

/** Every same-origin request the CONTEXT makes — its pages' and its service
 *  worker's — by path, flagged when the worker made it. The context and not
 *  the page, because the install's requests are the worker's own. */
function contextRequests(context: BrowserContext): { path: string; sw: boolean }[] {
  const origin = new URL(BASE_URL).origin;
  const seen: { path: string; sw: boolean }[] = [];
  context.on("request", (r) => {
    const u = new URL(r.url());
    if (u.origin === origin) seen.push({ path: u.pathname, sw: r.serviceWorker() !== null });
  });
  return seen;
}

/** Whether a worker serves this page (`offline.spec.ts`'s `controlled`). */
const controlled = (page: Page): Promise<boolean> =>
  page.evaluate(() => navigator.serviceWorker.controller !== null);

/** `fetch(MISS)` from the page: its status, its type and its title. */
const fetchMiss = (page: Page): Promise<{ status: number; type: string; title: string | null }> =>
  page.evaluate(async (at) => {
    const r = await fetch(at);
    const text = await r.text();
    return { status: r.status, type: r.headers.get("content-type") ?? "", title: /<title>([^<]*)<\/title>/.exec(text)?.[1] ?? null };
  }, MISS);

test("(a) the door, loaded cold with the worker on, asks for the release's own files and nothing else: no /api/, no /enter, no unknown path", async ({
  page,
  context,
}) => {
  const release = precached();
  const asked = contextRequests(context);
  const watch = watchForBreakage(page);

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  await expect(page.getByText("Ready to play offline")).toBeVisible({ timeout: INSTALL_WAIT_MS });
  await page.waitForTimeout(SETTLE_MS);

  const paths = [...new Set(asked.map((r) => r.path))].sort();
  expect(
    paths.filter((p) => p === "/enter" || p === "/api" || p.startsWith("/api/")),
    "the door asks the API nothing, and never /enter",
  ).toEqual([]);
  expect(
    paths.filter((p) => !release.has(p) && !DOOR_FILES.has(p)),
    "every path the door asks for is one of the release's own files",
  ).toEqual([]);

  // **Anti-vacuity**: the instrument saw the document, and it saw the WORKER's
  // requests — the install of the whole release, every precached path asked
  // by the worker itself — so an empty list of strangers is a reading of
  // everything the browser asked, not of a page that asked little.
  expect(paths, "the door's own document was seen").toContain("/");
  const byWorker = new Set(asked.filter((r) => r.sw).map((r) => r.path));
  expect(
    [...release].filter((p) => !byWorker.has(p)),
    "the worker's install asked for every precached path, and the instrument saw it",
  ).toEqual([]);

  // eslint-disable-next-line no-console
  console.log(
    `DOOR FENCE: ${paths.length} same-origin paths asked, ${byWorker.size} of them by the worker's install ` +
      `(${release.size} precached); none under /api/, no /enter, no unknown path`,
  );
  expect(watch.failures, "the door reported nothing broken").toEqual([]);
});

test("(b) a local game, played to its end with no sign-in, asks the API nothing until it ends — then one health read, and no post for a nameless visitor", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  /** Every request this page makes to the API or the turnstile, in order. */
  const asked: string[] = [];
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (u.origin !== new URL(BASE_URL).origin) return;
    if (u.pathname === "/enter" || u.pathname === "/api" || u.pathname.startsWith("/api/")) {
      asked.push(`${r.method()} ${u.pathname}`);
    }
  });

  const game = await startGame(page, { local: true, signedOut: true });
  expect(game.id.startsWith("local-"), `a local id, not ${game.id}`).toBe(true);
  expect(asked, "the door, the new-game form and the deal asked the API nothing").toEqual([]);

  const pick = rng(0x4645_4e43); // "FENC"
  let moves = 0;
  /** How many requests had been made before the move that ended the game:
   *  everything after it belongs to the ending. */
  let beforeTheLast = 0;
  let screen = await readScreen(page);
  while (!screen.over && moves < CAP) {
    beforeTheLast = asked.length;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, moves);
    moves += 1;
    screen = await readScreen(page);
  }
  if (!screen.over) throw await capReached(page, game, moves);
  await expect(page.getByText("The game is over.")).toBeVisible();
  expect(asked.slice(0, beforeTheLast), "no /api/ request of any kind while the game was played").toEqual([]);

  // **The ending settles where a player would see it**: the door lists the
  // finished game as waiting for a name — the outbox's `signed-out`, written
  // once the health read named the door `open` and no name was stored.
  await page.goto("/#/");
  const row = page.locator("li.outbox-row.signed-out", { hasText: game.id });
  await expect(row, "the door lists the game as waiting for a name").toBeVisible({ timeout: 30_000 });
  await expect(row.locator(".why")).toHaveText("not signed in");

  const ending = asked.slice(beforeTheLast);
  expect(
    ending.filter((c) => c !== "GET /api/health" && c !== "POST /api/archive"),
    "after the game ends, nothing but the health read and the archive",
  ).toEqual([]);
  expect(
    ending.filter((c) => c === "GET /api/health"),
    "ONE health read: a fresh browser does not know this door, and the read is how it learns it",
  ).toHaveLength(1);
  expect(
    ending.filter((c) => c === "POST /api/archive"),
    "and NO post: under the open door a visitor with no name is refused for certain (spec §9 F-14)",
  ).toEqual([]);

  // eslint-disable-next-line no-console
  console.log(
    `DOOR FENCE: local game ${game.id} played to its end in ${moves} clicks with no /api/ request; ` +
      `its ending asked ${JSON.stringify(ending)} and the door lists it "not signed in"`,
  );
  expect(watch.failures, "the page reported nothing broken").toEqual([]);
});

test("(c) a miss: /wp-login.php is the 404 page to a fetch and to a navigation no worker controls, and the shell to one the worker controls (Review Focus 5)", async ({
  page,
  browser,
}) => {
  // The fetches below are 404s by design: forgiven by status and path, and
  // nothing else.
  const watch = watchForBreakage(page, [{ status: 404, urlIncludes: MISS }]);

  // ---- a fetch from the page, before any worker controls it --------------
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  expect(await controlled(page), "the first visit is served by no worker (nothing claims)").toBe(false);
  const fetched = await fetchMiss(page);
  expect(fetched.status, "a fetch of a miss is answered 404 by the server").toBe(404);
  expect(fetched.type, "as a page").toMatch(/^text\/html/);
  expect(fetched.title, "the 404 page").toBe(NOT_FOUND_TITLE);

  // ---- a navigation in a context no worker controls -----------------------
  const fresh = await newContextWithoutWorker(browser);
  try {
    const p = await fresh.newPage();
    const res = await p.goto(MISS);
    expect(res?.status(), "the server answers a browser's miss 404").toBe(404);
    expect(res?.fromServiceWorker(), "and no worker stood in front of it").toBe(false);
    await expect(p).toHaveTitle(NOT_FOUND_TITLE);
    await expect(p.getByText("There's nothing at this address.")).toBeVisible();
    await expect(p.getByRole("link", { name: "Go to the door" })).toHaveAttribute("href", "/");
    expect(await controlled(p), "the 404 page registers no worker").toBe(false);
  } finally {
    await fresh.close();
  }

  // ---- the same navigation, in a context the worker controls --------------
  await expect(page.getByText("Ready to play offline")).toBeVisible({ timeout: INSTALL_WAIT_MS });
  // The reload puts the page in front of the release's worker (the offline
  // cells say why it has to be a reload: the worker does not claim).
  await page.reload();
  expect(await controlled(page), "the reload is served by the release's worker").toBe(true);
  // A FETCH of a miss is still the server's (rule 4: not a navigation, not
  // precached, so the network), and so still the 404 page.
  expect((await fetchMiss(page)).title, "a controlled page's fetch of a miss still reaches the server").toBe(NOT_FOUND_TITLE);
  const nav = await page.goto(MISS);
  expect(nav?.fromServiceWorker(), "the worker answered the navigation itself").toBe(true);
  expect(nav?.status(), "with the shell, not a 404").toBe(200);
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  expect(await page.title(), "the shell's title, not the 404 page's").not.toBe(NOT_FOUND_TITLE);
  expect(new URL(page.url()).pathname, "at the address the visitor typed").toBe(MISS);

  // eslint-disable-next-line no-console
  console.log(
    `DOOR FENCE: ${MISS} — a fetch 404 "${NOT_FOUND_TITLE}" (uncontrolled and controlled), an uncontrolled ` +
      `navigation 404 "${NOT_FOUND_TITLE}", a controlled navigation the shell from the worker`,
  );
  expect(watch.failures, "the page reported nothing broken beyond the designed 404s").toEqual([]);
});
