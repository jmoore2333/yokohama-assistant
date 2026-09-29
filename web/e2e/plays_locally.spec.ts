// **The exit: a whole game against the bot, played in this browser** (M6.5b,
// spec §6.4, §6.5).
//
// Every other instrument in this cycle stops at a boundary. The layer's cells
// drive `dispatch` natively; the component suite renders React over a FAKE
// Worker; the corpus replay runs the module in Node with no browser, no
// IndexedDB and no Web Lock, and is bot-free by construction. So the three
// things only this file can see are the three the milestone is about: that a
// real dedicated Worker holding the real module plays a whole game with
// NOTHING on the network, that the browser's own store and lock behave the way
// §2.2 says they do when they are pushed, and that the bytes a player saves
// are the bytes the server archives.
//
// It is also the PRODUCER of the determinism pin
// (`crates/yokohama-web/tests/browser_played_pin.rs`): under
// `YOKOHAMA_WEB_UPDATE_PIN=1` the first cell writes the pair it just played,
// and the sidecar that says which module played it.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { type Page, type Worker } from "@playwright/test";
// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember. A context a cell
// builds for itself is flagged by `newContextWithoutWorker`, for the same reason.
import { expect, newContextWithoutWorker, test } from "./sw";
import { archiveDir, BASE_URL, haveBrowser } from "../playwright.config";
import { act, atTheGame, capReached, deadEnd, readScreen, rng, startGame, WALKER, watchForBreakage } from "./harness";
import type { Affordance } from "./harness";
import { CAP, REV } from "./caps";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly — the guard
// `every_spec_can_skip.spec.ts` reads this directory for.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.join(HERE, "..", "..", "crates", "yokohama-web", "tests", "fixtures");
const PIN_RECORD = path.join(FIXTURES, "browser-played-2p.json");
const PIN_TABLE = path.join(FIXTURES, "browser-played-2p.table.json");
const PIN_SIDECAR = path.join(FIXTURES, "browser-played-2p.provenance.json");
/** The module the page actually ran, as `npm run e2e` built it — the source
 *  copy Vite bundles, whose bytes are the hashed asset's. */
const MODULE = path.join(HERE, "..", "src", "local", "wasm", "yokohama_wasm_bg.wasm");

/**
 * A path-safe, readable rendering of the caller's address.
 *
 * The server's own rule, in `crates/yokohama-web/src/archive.rs`'s
 * `caller_segment` (R-6.5b.15): `@` becomes `_at_`, anything outside
 * `[a-z0-9._-]` becomes `_`, capped at 64. Restated here rather than read off
 * the reply, because the reply is the thing under test — a server that filed a
 * game under the wrong prefix would be believed by a walk that took the prefix
 * from it.
 */
function callerSegment(email: string): string {
  return [...email]
    .map((c) => (c === "@" ? "_at_" : /[a-z0-9._-]/.test(c) ? c : "_"))
    .join("")
    .slice(0, 64);
}

const SEGMENT = callerSegment(WALKER);

/**
 * The page's dedicated Worker — the local host itself.
 *
 * Playwright hands it over as a `Worker` whose `evaluate` runs INSIDE it,
 * which is what makes the two fault cells below real: a quota failure has to
 * happen in the transaction the Worker commits, and a trap has to happen
 * inside the module the Worker holds. Neither is reachable from the page, and
 * `addInitScript` does not reach a dedicated Worker at all (Playwright's init
 * scripts are `Page.addScriptToEvaluateOnNewDocument`, which is per FRAME).
 */
async function localHost(page: Page): Promise<Worker> {
  const already = page.workers()[0];
  if (already) return already;
  return page.waitForEvent("worker", { timeout: 30_000 });
}

/** Both of a local game's stored documents, read from the page's own
 *  IndexedDB — the browser's copy, not the module's.
 *
 *  This is the only place a test can see what was actually COMMITTED, which is
 *  the difference §2.2 draws between a move that was shown and a move that was
 *  kept. */
async function storedPair(
  page: Page,
  id: string,
): Promise<{ record: string | null; table: string | null }> {
  return page.evaluate(
    (game) =>
      new Promise<{ record: string | null; table: string | null }>((resolve, reject) => {
        const open = indexedDB.open("yokohama-local");
        open.onerror = () => reject(new Error("IndexedDB would not open"));
        open.onsuccess = () => {
          const db = open.result;
          if (!db.objectStoreNames.contains("docs")) {
            reject(new Error("no `docs` store — has the Worker ever run?"));
            return;
          }
          const t = db.transaction("docs", "readonly");
          const s = t.objectStore("docs");
          const rec = s.get(["record", game]) as IDBRequest<{ text: string } | undefined>;
          const tab = s.get(["table", game]) as IDBRequest<{ text: string } | undefined>;
          t.oncomplete = () =>
            resolve({ record: rec.result?.text ?? null, table: tab.result?.text ?? null });
          t.onerror = () => reject(t.error ?? new Error("the read failed"));
        };
      }),
    id,
  );
}

/** How many events this browser has COMMITTED for a game. `-1` where no record
 *  is stored yet, which is a table-only pair and a real state (R1). */
async function storedEvents(page: Page, id: string): Promise<number> {
  const { record } = await storedPair(page, id);
  if (record === null) return -1;
  return (JSON.parse(record) as { events: unknown[] }).events.length;
}

/** How many events a record carries. */
const eventsOf = (record: string): number =>
  (JSON.parse(record) as { events: unknown[] }).events.length;

/** One game in a backup file — `backup.ts`'s `BackupGame`, restated rather
 *  than imported because that module would drag the page's whole local client
 *  into the node project. Only the three fields read here. */
interface BackedUp {
  id: string;
  record: string | null;
  table: string;
}

/** Click something that downloads, and answer the file: its text, and where
 *  Playwright put it (a restore takes a PATH). */
async function downloaded(
  page: Page,
  click: () => Promise<void>,
): Promise<{ text: string; file: string }> {
  const [download] = await Promise.all([page.waitForEvent("download"), click()]);
  const file = (await download.path())!;
  return { text: fs.readFileSync(file, "utf8"), file };
}

/** The one game `id` in a backup file, or a failure naming what the file did
 *  carry. */
function backedUp(envelope: string, id: string): BackedUp {
  const games = (JSON.parse(envelope) as { games: BackedUp[] }).games;
  const game = games.find((g) => g.id === id);
  if (!game) throw new Error(`the backup carries no ${id}: ${games.map((g) => g.id).join(", ")}`);
  return game;
}

/** Play `n` moves, or until the game ends. What every cell below opens with,
 *  so its baseline is a game somebody has played rather than a game that has
 *  just been dealt: a claim about what survives is worth nothing when what
 *  there is to survive is nothing. */
async function playSome(page: Page, game: { id: string }, pick: () => number, n: number) {
  for (let i = 0; i < n; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) return;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, i);
  }
  await readScreen(page);
}

/** Click until this browser COMMITS another event, and answer the new count.
 *
 *  **A click is not a move**, which three cells learned the hard way: an
 *  option row can open a sub-picker, and the move is applied by the Confirm
 *  that follows. So "play went on" is asked of the STORE — the only place that
 *  can answer it — rather than counted in clicks. */
async function playUntilCommitted(
  page: Page,
  game: { id: string },
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

/** The turn the screen is on, off the decision zone's own chain. `innerText`
 *  is what it renders (`text-transform: uppercase`), so the read is
 *  case-insensitive. */
async function turnNumber(page: Page): Promise<number> {
  const chain = await page.locator(".decision .chain").first().innerText();
  const found = /TURN (\d+)/i.exec(chain);
  if (!found) throw new Error(`no turn number in the chain: ${chain}`);
  return Number(found[1]);
}

/** Play until the screen offers an OPTION ROW, so a cell about the answer bar
 *  disappearing has an answer bar to lose. */
async function playUntilAnswerable(page: Page, game: { id: string }, pick: () => number) {
  for (let i = 0; i < 40; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before the cell could act");
    if (screen.options > 0) return;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, i);
  }
  throw new Error("forty moves and the screen never offered an option row");
}

test("a whole game against the bot plays in the browser with no /api/ request while it is played", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  /** Every request this page makes to the API, in order. The M6.3 shape, and
   *  WIDER than "no `/api/games`": a stream, another play route or a health
   *  re-read would all be counted, because the claim is that nothing on the
   *  network is involved in playing this game. */
  const apiCalls: string[] = [];
  page.on("request", (r) => {
    const url = new URL(r.url());
    if (url.origin !== new URL(BASE_URL).origin) return;
    if (!url.pathname.startsWith("/api/")) return;
    apiCalls.push(`${r.method()} ${url.pathname}`);
  });

  const game = await startGame(page, { local: true });
  expect(game.id.startsWith("local-"), `a local id, not ${game.id}`).toBe(true);
  // **The baseline is taken HERE**, with the game already on screen: this walk
  // SIGNS IN first, so the lobby's own reads — the door, the health, the words
  // and the listings — all happen before it and are the server's business.
  // `/api/archive` is the one exemption, and it is not reached until the game
  // is over.
  //
  // **What changed at M6.5b2 is which SCREEN those reads belong to** (spec
  // §2.2): they are the lobby's now and not the boot's, which is why the walk
  // still makes them and why the archive fact is already known by the time this
  // game ends. The cell that reads the door's own cost is the jsdom one; the
  // one below this test reads it in a browser, with every `/api/` refused.
  const before = apiCalls.length;
  expect(before, "the sign-in really did talk to the server, so zero below means something").toBeGreaterThan(0);

  const pick = rng(0x4c4f_4341); // "LOCA"
  const used = new Map<Affordance, number>();
  let moves = 0;
  let screen = await readScreen(page);
  while (!screen.over && moves < CAP) {
    const via = await act(page, screen, pick);
    if (via === null) throw await deadEnd(page, game, moves);
    used.set(via, (used.get(via) ?? 0) + 1);
    moves += 1;
    screen = await readScreen(page);
  }
  if (!screen.over) throw await capReached(page, game, moves);
  expect(moves, "the walk must not hit its own cap").toBeLessThan(CAP);
  await expect(page.getByText("The game is over.")).toBeVisible();

  // **A real END, and not merely a screen that says so.** `/result` is served
  // by the module here like every other route, so the ranked standing is the
  // local host's own answer — two seats, and the winner MARKED from
  // `Placing.won` rather than by comparing totals, because two seats can tie
  // on points and be split by the engine's tie-break.
  await expect(page.locator("table.outcome tbody tr")).toHaveCount(2);
  await expect(page.locator("table.outcome tr.won")).toHaveCount(1);

  // **What the ENDING does, and it is new at M6.5b2** (spec §2.6): the
  // game-over hook offers the finished game to the archive without the player
  // clicking anything, and the screen says where the copy went. That attempt is
  // the whole reason the outbox exists — a send that does not land has to be
  // REMEMBERED rather than lost, and the control that used to be the only way
  // to make one is dark on a browser that has never been told there is an
  // archive at all (spec §0.3).
  //
  // **The archive fact itself costs no request here**: the game reaching game
  // over IS one of the three read points, but the answer this browser already
  // holds is under an hour old — the lobby read it on the way in and wrote it
  // down with the time it was taken — so the read is not repeated, which is
  // what keeps the count below at exactly one.
  const archivePath = `browser/${SEGMENT}/${REV}/over/${game.id}.json`;
  await expect(page.locator(".meta.archived")).toHaveText(`archived under ${archivePath}`);
  // **And no control is offered for work already done** (review M-6): the copy
  // is made, so the button that would make it is gone rather than sitting there
  // to buy an `identical`.
  await expect(page.getByRole("button", { name: "Send to the archive" })).toHaveCount(0);
  expect(
    apiCalls.filter((c) => c === "POST /api/archive").length,
    "the ending costs exactly one archive POST, made by the hook",
  ).toBe(1);
  /** Everything this page asked the API for since the game opened, MINUS the
   *  archive's own POST — which is counted exactly, just above, so filtering it
   *  here cannot hide a second one. */
  const duringPlay = apiCalls.slice(before).filter((c) => c !== "POST /api/archive");
  expect(duringPlay, "no /api/ request of ANY kind while the game is played").toEqual([]);

  // **The download is the Worker's bytes.** `recordText` reads a local game
  // off the side channel and never parses on the way, so the file a player
  // saves is the file the terminal loads.
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("button", { name: "Save game file" }).click(),
  ]);
  expect(download.suggestedFilename()).toBe(`${game.id}.json`);
  const saved = fs.readFileSync((await download.path())!, "utf8");
  expect(saved).toContain('"terminal_digest"');

  const stored = await storedPair(page, game.id);
  expect(saved, "the download is the STORED record, byte for byte").toBe(stored.record);
  expect(stored.table, "a playing game has both documents on this device").not.toBeNull();

  // **The copy is on disk, and nobody clicked anything** (M6.5b2, spec §2.6):
  // the server replayed the record the hook sent and chose the leaf from the
  // state it reached. Nothing in the body said where the copy should go, and
  // the path the screen printed above is the server's own answer.
  const onDisk = path.join(archiveDir(), ...archivePath.split("/"));
  expect(fs.existsSync(onDisk), "the game-over hook's own send landed").toBe(true);
  const archived = fs.readFileSync(onDisk, "utf8");
  expect(archived, "the archived record is the downloaded file, byte for byte").toBe(saved);
  // The table lands beside it, stripped of a lock and a watcher roster it
  // never had. Its presence is the operator's sign that the pair is whole.
  expect(fs.existsSync(onDisk.replace(/\.json$/, ".table.json"))).toBe(true);

  // **And the reading holds to the END of the cell**, which the slice above
  // cannot say by itself: it is one moment, and the download comes after it.
  // Everything this page asked the API for since the game opened is ONE archive
  // POST and nothing else.
  expect(apiCalls.slice(before)).toEqual(["POST /api/archive"]);

  // eslint-disable-next-line no-console
  console.log(
    `LOCAL WALK: game ${game.id} finished in ${moves} clicks, ` +
      `${(JSON.parse(saved) as { events: unknown[] }).events.length} events, ` +
      `${saved.length} bytes of record, ${duringPlay.length} /api/ requests while playing ` +
      `(the ending's own archive POST excluded), via ` +
      [...used].map(([k, n]) => `${k}×${n}`).join(", "),
  );

  // **The determinism pin's pair**, written only when asked for (spec §6.4).
  // The TABLE is the local document and not the archived copy, because "what
  // Send to the archive sends" is what the pin is about.
  if (process.env.YOKOHAMA_WEB_UPDATE_PIN) {
    fs.mkdirSync(FIXTURES, { recursive: true });
    fs.writeFileSync(PIN_RECORD, saved);
    fs.writeFileSync(PIN_TABLE, stored.table!);
    const git = (...args: string[]) => execFileSync("git", args, { cwd: HERE }).toString().trim();
    const head = git("rev-parse", "--short", "HEAD");
    // On a detached checkout `--abbrev-ref HEAD` reads "HEAD" itself, which is
    // what is recorded then: a name is not read that the checkout does not have.
    const branch = git("rev-parse", "--abbrev-ref", "HEAD");
    // The release and the branch the pin is cut on, READ rather than written
    // down (M6.5c2, R-6.5c2.19): the literal that stood here named M6.5b's tag
    // and branch, and stayed as written while M6.5b2, M6.5c and M6.5c2 moved
    // the branch and M6.5c2 the version. `web/package.json` carries the
    // workspace's version — the npm packages follow the release, one number,
    // pinned by `the_two_npm_packages_carry_the_workspace_version`.
    const version = (
      JSON.parse(fs.readFileSync(path.join(HERE, "..", "package.json"), "utf8")) as { version: string }
    ).version;
    fs.writeFileSync(
      PIN_SIDECAR,
      `${JSON.stringify(
        {
          release: `yokohama-web ${version} (branch ${branch} at ${head})`,
          date: new Date().toISOString().slice(0, 10),
          browser: `Chromium ${page.context().browser()?.version() ?? "unknown"} (Playwright)`,
          module_sha256: createHash("sha256").update(fs.readFileSync(MODULE)).digest("hex"),
          module_build:
            "debug — `npm run build:wasm:debug`, which `npm run e2e` runs; the RELEASE module " +
            "(`npm run build:wasm`) has the same bot and no `debug_trap`",
          produced_by: "web/e2e/plays_locally.spec.ts under YOKOHAMA_WEB_UPDATE_PIN=1",
        },
        null,
        2,
      )}\n`,
    );
  }

  expect(watch.failures, "the page reported no errors while a whole game was played").toEqual([]);
});

test("a reload comes back from IndexedDB with every committed move, and the replay wall is recorded", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  // **The cold-pump clause stays PARKED** (spec §6.5; the M6.5b close's Task 8
  // ruling). §6.5 wanted this reload to prove the bot's trailing run is played
  // on a cold read of a record that ENDS at the bot's turn, and no local record
  // ends there. The M6.5b ruling rested on every local game seating the person
  // first; the local form has since offered "First player: The bot", and the
  // M6.5c2 cycle D dossier asked whether that reaches the arm (row 24,
  // `docs/plans/2026-09-28-web-frontend-m6.5c2d-dossier.md`). It does not: the
  // create plays the bot's opening run BEFORE the first record is written
  // (`tables.rs`, `bring_alive` ahead of the record-first write), and every
  // input persists after its own pump — so a bot-first game, too, stores its
  // first record at the person's turn. The arm is still read code with no
  // production path; the one way to it is a RESTORED pair whose record ends at
  // the bot's turn, which is not this cell's.
  const game = await startGame(page, { local: true });
  const pick = rng(0x5245_4c4f); // "RELO"
  // A long preamble on purpose: the O-3 figure below is the cost of a COLD
  // replay, and a seventeen-event record would not be a reading worth
  // recording.
  await playSome(page, game, pick, 60);
  const chainOf = () => page.locator(".decision .chain").first().innerText();
  const chain = await chainOf();
  const committed = await storedEvents(page, game.id);
  // Anti-vacuity, and it is about the BOT: a game still on its first turn
  // would be this browser's own clicks and nothing else, and what survives the
  // reload below would prove half as much. The turn is past one, so the seat
  // the module plays has had one.
  expect(committed, "the preamble must actually advance the game").toBeGreaterThan(0);
  expect(await turnNumber(page), "and the bot has had a turn of its own").toBeGreaterThan(1);

  // **O-3, RECORDED and never gated** (spec §6.4): what a cold browser costs
  // to come back — the page load, the Worker's boot, the module's `init`, the
  // IndexedDB rebuild and `Session::from_record` inside the module, to the
  // first frame on screen.
  const t0 = Date.now();
  await page.reload();
  await expect(page.locator("header .meta").first()).toContainText(game.id);
  await readScreen(page);
  const wall = Date.now() - t0;

  expect(await chainOf(), "the rebuilt game resumes at the turn it left off").toBe(chain);
  expect(
    await storedEvents(page, game.id),
    "nothing the browser committed was lost, and nothing was re-derived into it",
  ).toBe(committed);

  // And it goes on being played: the next move is committed too, which is what
  // says the rebuilt Worker HOLDS the game rather than merely showing it.
  await playUntilCommitted(page, game, pick, committed);

  // eslint-disable-next-line no-console
  console.log(
    `O-3 (browser): a ${committed}-event local game was back on screen ${wall} ms after the reload`,
  );
  expect(watch.failures).toEqual([]);
});

test("a game closed before its first card is chosen comes back on reload with its exact setup", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  // Deliberately NOT `startGame`: the whole subject is the state BEFORE the
  // keep, where the table document exists and the record does not (R1).
  //
  // **And no sign-in, because the door needs none** (M6.5b2, R-6.5b2.1): `/` is
  // the front door, the games in this browser are on it, and the local create
  // is one click from there.
  await page.goto("/");
  await page.getByRole("button", { name: "New game", exact: true }).click();
  await page.getByRole("button", { name: "Deal" }).click();
  await page.waitForURL(/#\/t\/local-[0-9a-f]+$/);
  const id = page.url().split("#/t/")[1]!;

  const picks = page.locator(".app.setup-table.dealing button.deal-pick");
  await expect(picks).toHaveCount(2);
  const dealt = await picks.allInnerTexts();

  const pair = await storedPair(page, id);
  expect(pair.table, "the table document is written at the create").not.toBeNull();
  expect(pair.record, "there is no record until the keep finalizes the table").toBeNull();

  await page.reload();
  await expect(page.locator(".app.setup-table.dealing")).toBeVisible();
  expect(await picks.allInnerTexts(), "the same two cards, dealt from the same seed").toEqual(dealt);
  expect((await storedPair(page, id)).table, "and the same document behind them").toBe(pair.table);

  // The setup is still playable: keeping one starts the game it was dealt for.
  await picks.first().click();
  await expect(atTheGame(page)).toBeVisible();
  expect(await storedEvents(page, id)).toBeGreaterThanOrEqual(0);
  expect(watch.failures).toEqual([]);
});

test("a quota failure is APPLIED BUT NOT SAVED, and the game stays exportable", async ({ page }) => {
  const watch = watchForBreakage(page);
  const game = await startGame(page, { local: true });
  const worker = await localHost(page);
  const pick = rng(0x5155_4f54); // "QUOT"
  await playSome(page, game, pick, 6);
  await playUntilAnswerable(page, game, pick);
  const committed = await storedEvents(page, game.id);
  expect(committed, "there is a played game here to keep").toBeGreaterThan(0);

  // **The store refuses, and only the store.** `IDBObjectStore.prototype.put`
  // is overridden INSIDE the Worker — the one place the commit happens — and
  // only for the `docs` store, so the ownership generation still moves and
  // what fails is exactly what a full quota fails: keeping the documents.
  await worker.evaluate(() => {
    const orig = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = function (
      this: IDBObjectStore,
      ...args: Parameters<typeof orig>
    ) {
      if (this.name === "docs") throw new DOMException("the quota is full", "QuotaExceededError");
      return orig.apply(this, args);
    };
  });

  const screen = await readScreen(page);
  expect(screen.options, "the cell needs a move to make").toBeGreaterThan(0);
  await page.locator("button.option:not([disabled])").first().click();

  // The server's own sentence, on the applied reply — the move HAPPENED and
  // was not kept, which is one rule and one wording across both hosts.
  await expect(page.locator(".deck .warn")).toContainText("APPLIED BUT NOT SAVED");
  expect(
    await storedEvents(page, game.id),
    "and the store really did keep nothing new",
  ).toBe(committed);

  // **And the game is still exportable**, which is the half that makes the
  // sentence liveable: the bytes are the module's and the module is fine.
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("button", { name: "Save game file" }).click(),
  ]);
  const saved = fs.readFileSync((await download.path())!, "utf8");
  const record = JSON.parse(saved) as { events: unknown[] };
  expect(
    record.events.length,
    "the exported record carries the move the browser would not keep",
  ).toBeGreaterThan(committed);

  // **And so does the BACKUP** (PR #37 fix wave, R-F1) — the constraint the
  // fix for a stale export must not break. A backup now takes this tab's memory
  // only where this tab OWNS the game at the current generation, and the
  // committed pair everywhere else; this tab does own it (the refusal was the
  // documents' store, and the generation still reads this tab's), so the move
  // that exists nowhere but in the module is in the file. An export that simply
  // read IndexedDB would hand back `committed` events here and lose it.
  const backup = await downloaded(page, () =>
    page.getByRole("button", { name: "Back up", exact: true }).click(),
  );
  const backed = backedUp(backup.text, game.id);
  expect(backed.record, "the backup carries the Save game file's record, byte for byte").toBe(
    saved,
  );
  expect(
    eventsOf(backed.record!),
    "the backup carries the move the browser would not keep",
  ).toBeGreaterThan(committed);
  expect(watch.failures).toEqual([]);
});

// **A store that refuses READS as well as writes** (PR #37 fix wave, fix round
// 1, review I-1). The quota cell above refuses only `put`, so every read still
// works and it cannot see what an export does when IndexedDB will not answer
// at all — a connection closed by another tab's storage upgrade, or WebKit's
// lost connection after a backgrounding. The export arm reads IndexedDB since
// R-F1, and a failed read there used to be answered as a TRAP: the page
// restarted the host, and the restart threw away the only copy of the move the
// store had just refused. The click meant to rescue the move destroyed it.
test("a store that will not even be READ still exports the move it refused, and the host is not restarted", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  let started = 0;
  page.on("worker", () => {
    started += 1;
  });
  const game = await startGame(page, { local: true });
  const worker = await localHost(page);
  const hosts = started;
  expect(hosts, "the tab has its local host").toBeGreaterThan(0);
  const pick = rng(0x5245_4144); // "READ"
  await playSome(page, game, pick, 6);
  await playUntilAnswerable(page, game, pick);
  const committed = await storedEvents(page, game.id);
  expect(committed, "there is a played game here to keep").toBeGreaterThan(0);

  // Writes first, exactly as the quota cell refuses them, so there IS a move
  // that exists only in the module.
  await worker.evaluate(() => {
    const orig = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = function (
      this: IDBObjectStore,
      ...args: Parameters<typeof orig>
    ) {
      if (this.name === "docs") throw new DOMException("the quota is full", "QuotaExceededError");
      return orig.apply(this, args);
    };
  });
  // **Until the refusal SHOWS, and not one click**: a click is not a move (see
  // `playUntilCommitted`). An option row that opens a sub-picker changes no
  // document, writes nothing and is refused nothing — measured, one run in
  // twenty of this cell's first draft did exactly that.
  const unsaved = page.locator(".deck .warn", { hasText: "APPLIED BUT NOT SAVED" });
  for (let i = 0; i < 12 && (await unsaved.count()) === 0; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before a move was refused");
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, i);
    await readScreen(page);
  }
  await expect(unsaved).toBeVisible();
  expect(await storedEvents(page, game.id), "the store kept nothing new").toBe(committed);

  // **And now every transaction the Worker asks for is refused** — the shape a
  // connection closed on `versionchange` has, where `db.transaction` throws
  // before any request is made. INSIDE the Worker only: the page's own reads
  // below are what this cell checks the exports against.
  await worker.evaluate(() => {
    IDBDatabase.prototype.transaction = function () {
      throw new DOMException("The database connection is closing.", "InvalidStateError");
    };
  });

  // The owner cannot read its generation, so it answers from memory — the last
  // owner the store knew of, holding the only copy of the refused move.
  const file = await downloaded(page, () =>
    page.getByRole("button", { name: "Save game file" }).click(),
  );
  expect(eventsOf(file.text), "Save game file carries the move the store refused").toBeGreaterThan(
    committed,
  );
  const backup = await downloaded(page, () =>
    page.getByRole("button", { name: "Back up", exact: true }).click(),
  );
  expect(backedUp(backup.text, game.id).record, "and so does the backup, byte for byte").toBe(
    file.text,
  );

  // **Nothing was restarted**: the host holding that move is the one that
  // answered both exports.
  expect(started, "the page started no second local host").toBe(hosts);
  expect(page.workers()).toContain(worker);
  await expect(page.getByText("the local host restarted")).toHaveCount(0);
  expect(watch.failures).toEqual([]);
});

// **The same refusal in a tab that does NOT hold the game** (fix round 2,
// re-review N-1). The cell above reaches only the holder's branch, which
// answers from memory and never produces the Worker's `unreadable` reply; this
// one produces it from the real Worker. A tab on the door holds nothing, so
// its host answers every export with the COMMITTED pair — and when it cannot
// read that, it must say so as an ordinary reply. Before fix round 1 the
// failed read was a trap: the page terminated this host, for an export the
// page's own read could have answered all along. (The restart re-holds nothing
// here, so another host starts only when something asks again.)
test("a tab whose host cannot read IndexedDB still backs everything up, is refused the game as a face, and its host is not restarted", async ({
  page: a,
  context,
}) => {
  const watchA = watchForBreakage(a);
  const game = await startGame(a, { local: true });
  const pick = rng(0x444f_4f52); // "DOOR"
  await playSome(a, game, pick, 6);
  await playUntilAnswerable(a, game, pick);
  expect(await storedEvents(a, game.id), "A committed a played game").toBeGreaterThan(0);

  const b = await context.newPage();
  const watchB = watchForBreakage(b);
  let startedB = 0;
  b.on("worker", () => {
    startedB += 1;
  });
  await b.goto("/");
  await expect(b.locator(`.on-this-device a[href="#/t/${game.id}"]`)).toBeVisible();
  const hostB = await localHost(b);
  const hostsB = startedB;

  // **Every transaction B's host asks for is refused**, and everything it posts
  // is copied aside so the cell can see which reply the export met — by its
  // SENTENCE, never by restating the key the two sides share.
  await hostB.evaluate(() => {
    IDBDatabase.prototype.transaction = function () {
      throw new DOMException("The database connection is closing.", "InvalidStateError");
    };
    const posted: unknown[] = [];
    (self as unknown as { __posted: unknown[] }).__posted = posted;
    const send = self.postMessage.bind(self);
    self.postMessage = ((m: unknown) => {
      posted.push(m);
      send(m);
    }) as typeof self.postMessage;
  });

  const all = await downloaded(b, () =>
    b.getByRole("button", { name: "Back up everything", exact: true }).click(),
  );
  // The premise: B's host was asked, and answered the refusal as a REPLY.
  const replies = await hostB.evaluate(() =>
    JSON.stringify((self as unknown as { __posted: unknown[] }).__posted),
  );
  expect(replies, "B's host answered the export with the storage refusal").toContain(
    "this browser's storage could not be read",
  );

  // **The file is still whole and right**: the page's own read, from a page
  // whose IndexedDB is fine, is what a refused Worker falls back to.
  const backed = backedUp(all.text, game.id);
  const stored = await storedPair(b, game.id);
  expect(backed.record, "the backup's record is IndexedDB's, byte for byte").toBe(stored.record);
  expect(backed.table, "and its table").toBe(stored.table);

  // **And nothing was restarted.**
  expect(startedB, "B started no second local host").toBe(hostsB);
  expect(b.workers()).toContain(hostB);
  await expect(b.getByText("the local host restarted")).toHaveCount(0);

  // **The same store refuses B the GAME, as a face and not a trap** (M6.5c2
  // cycle D, dossier row 14). Opening a local game takes its ownership
  // generation in IndexedDB, and B's host cannot run that transaction either.
  // That used to be a trap: B's page threw its host away and said "the local
  // host restarted" about a store it had not read, and drew no screen for the
  // game. It is the storage's own sentence now, drawn with Reload, from the
  // one host B started. A leaves the game first, and the lock is seen to go,
  // so what B meets is the store and not A's tab.
  await a.goto("/");
  const lockName = `yokohama:local:${game.id}`;
  await expect
    .poll(
      () => b.evaluate(async (name) => ((await navigator.locks.query()).held ?? []).some((l) => l.name === name), lockName),
      { message: "A's tab let the game go" },
    )
    .toBe(false);
  await b.locator(`.on-this-device a[href="#/t/${game.id}"]`).click();
  const face = b.locator(".broken-store");
  await expect(face).toContainText("this browser's storage could not be read");
  await expect(face.getByRole("button", { name: "Reload" })).toBeVisible();
  await expect(b.getByText("This game is open in another tab")).toHaveCount(0);
  expect(startedB, "and B's host is still the one it started").toBe(hostsB);
  expect(b.workers()).toContain(hostB);
  expect(watchA.failures).toEqual([]);
  expect(watchB.failures).toEqual([]);
});

test("a real trap is recovered by a fresh local host, and play continues from the last committed move", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  let started = 0;
  /** Every host this tab has started, newest last. */
  const hosts: Worker[] = [];
  page.on("worker", (w) => {
    started += 1;
    hosts.push(w);
  });
  const game = await startGame(page, { local: true });
  const worker = await localHost(page);
  expect(started, "one Worker for the tab, so far").toBe(1);
  const pick = rng(0x5452_4150); // "TRAP"
  await playSome(page, game, pick, 6);
  await playUntilAnswerable(page, game, pick);
  const committed = await storedEvents(page, game.id);
  expect(committed, "there is a played game here to come back to").toBeGreaterThan(0);

  // **A REAL trap, through the module's own `debug_trap`** — built by
  // `npm run build:wasm:debug`, which `npm run e2e` runs, and absent from the
  // release module. It panics under a held lock, which leaves the module
  // trapped: every later call throws.
  //
  // Posted into the Worker as an ordinary message, so it lands in the same
  // serial queue every call does and the move below is queued BEHIND it. The
  // hook answers without a `trapped` flag on purpose (see `worker.ts`), so
  // what the page meets is what a production trap looks like: the NEXT call
  // dies, and the pending call is what carries the restart back to the screen.
  await worker.evaluate(() => {
    (self as unknown as { onmessage: (e: { data: unknown }) => void }).onmessage({
      data: { seq: 987_654, kind: "debug_trap" },
    });
  });

  await page.locator("button.option:not([disabled])").first().click();
  await expect(page.locator(".decision.notices .error")).toContainText(
    "the local host restarted",
  );
  expect(started, "the page threw the trapped Worker away and started a fresh one").toBe(2);

  // Nothing committed was lost — a success reply is only sent after its write
  // landed — and nothing the trapped instance held was kept.
  expect(await storedEvents(page, game.id)).toBe(committed);

  // **And the game goes on.** The fresh Worker rebuilt from the committed
  // pairs and re-acquired the game, so the screen is answerable again and the
  // next move is committed like any other.
  await expect(page.getByText("This game was opened in another tab")).toHaveCount(0);
  const resumed = await playUntilCommitted(page, game, pick, committed);

  // **And a trap met by an EXPORT is still a trap** (PR #37 fix wave, fix
  // round 2, re-review N-1). Since fix round 1 the export arm answers a STORAGE
  // refusal as an ordinary reply, and it must not answer a MODULE fault the
  // same way: `wasm.read` on a trapped module throws, and that is recovered by
  // a fresh host like any other trap — the export refused, nothing downloaded,
  // and the game playable again from what was committed.
  const second = hosts[hosts.length - 1]!;
  expect(second, "the host that recovered the first trap").not.toBe(worker);
  let downloads = 0;
  page.on("download", () => {
    downloads += 1;
  });
  await second.evaluate(() => {
    (self as unknown as { onmessage: (e: { data: unknown }) => void }).onmessage({
      data: { seq: 987_655, kind: "debug_trap" },
    });
  });
  await page.getByRole("button", { name: "Save game file" }).click();
  await expect
    .poll(() => started, { message: "the export met the trap, and the page started a third host" })
    .toBe(3);
  expect(downloads, "a trapped module's export is refused, not saved").toBe(0);
  expect(await storedEvents(page, game.id)).toBe(resumed);
  await expect(page.getByText("This game was opened in another tab")).toHaveCount(0);
  await playUntilCommitted(page, game, pick, resumed);

  // The trap itself is a refused call, not a page error: the one console
  // channel a player would call broken stays empty.
  expect(watch.failures).toEqual([]);
});

test("the first screen renders On this device with the API refused, and a local game plays", async ({
  page,
}) => {
  // Every `/api/` request refused at the edge. Level 2 is "the API unreachable
  // with the page loaded" (spec §8), which is exactly this — and since M6.5b2
  // the front door makes none of them, so what the refusal is really reaching
  // is the LOBBY one hash away, below.
  await page.route("**/api/**", (r) => r.fulfill({ status: 503, body: "down" }));
  const watch = watchForBreakage(page, [{ status: 503, urlIncludes: "/api/" }]);
  /** Every same-origin path this page asks for — the same instrument the walk
   *  above uses, read here for the DOOR rather than for a game. */
  const asked: string[] = [];
  page.on("request", (r) => {
    const url = new URL(r.url());
    if (url.origin === new URL(BASE_URL).origin) asked.push(url.pathname);
  });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  await expect(page.getByRole("button", { name: "New game", exact: true })).toBeVisible();
  // **The door costs the Worker NOTHING** (R-6.5b2.7), in a real browser: not
  // `/api/me`, not the health, not the words, not the art. The jsdom cell makes
  // the same claim against a stubbed `fetch`; this one makes it against the
  // network, where the art manifest really is a static file served beside the
  // pictures rather than a route.
  expect(
    asked.filter((p) => p.startsWith("/api/")),
    "the front door asks the API for nothing",
  ).toEqual([]);
  // The anti-vacuity half: it DID fetch, just not the API — the art manifest is
  // a static file now, served beside the pictures it names.
  expect(asked, "and the manifest is a file, not a route").toContain("/art/manifest.json");
  // **The other way on is drawn from the BUILD, not from an answer** (M6.5b2,
  // R-6.5b2.7): every `/api/` request is being refused here, and the link is on
  // the screen anyway with the turnstile's own address in it. This is the half
  // no jsdom cell can make — there the refusal is a stub; here it is the edge.
  // It is READ and never followed: `/enter` is a navigation, and following it
  // would take this page off the app.
  await expect(page.getByRole("link", { name: "Go to the lobby" })).toHaveAttribute(
    "href",
    "/enter",
  );
  // **The door reports nothing about a server it never asks** (R-6.5b2.7). The
  // line that does report one is the LOBBY's, one hash away, and it is still
  // there — so the claim this cell has always made is read where it now lives.
  await expect(page.getByRole("heading", { name: "Can't reach the server" })).toHaveCount(0);
  await page.goto("/#/lobby");
  await expect(page.getByRole("heading", { name: "Can't reach the server" })).toBeVisible();

  const game = await startGame(page, { local: true, signedOut: true });
  const pick = rng(0x4f46_464c); // "OFFL"
  await playSome(page, game, pick, 6);
  expect(await storedEvents(page, game.id), "and it is kept on this device").toBeGreaterThan(0);

  // **The words on the cards survive** (R-6.5b.8): `labels` falls back to the
  // Worker, so the phase chain reads as the terminal prints it rather than as
  // the serde tags the frame carries. Both halves are asserted, because the
  // absence of one tag is not the presence of the word.
  //
  // `innerText` and a case-INSENSITIVE match: the chain is `text-transform:
  // uppercase`, so what comes back is "5-POWER" whatever the label said. The
  // rendered case is the stylesheet's business and not this cell's.
  const chain = await page.locator(".decision .chain").first().innerText();
  expect(chain, `the phase chain reads as words: ${chain}`).toMatch(/5-POWER/i);
  expect(chain, "and never the serde tag the frame carries").not.toMatch(/FivePowerBonus/i);
  expect(watch.failures).toEqual([]);
});

test("two tabs: B boots first, A plays, B takes over and sees A's moves; A goes read-only", async ({
  page: a,
  context,
}) => {
  const watchA = watchForBreakage(a);
  // **B FIRST, and that is the whole of C-2.** B's Worker boots on the lobby,
  // with a store that knows nothing of the game A is about to play — so a
  // takeover that did not rebuild from IndexedDB would show B an empty or
  // stale game, and the assertion below is what catches it.
  const b = await context.newPage();
  // The DOOR, which is where the games in this browser are listed since M6.5b2
  // and which needs no sign-in at all (R-6.5b2.1).
  await b.goto("/");
  await expect(b.locator(".on-this-device")).toBeVisible();
  await localHost(b);

  const game = await startGame(a, { local: true });
  const pick = rng(0x5457_4f54); // "TWOT"
  await playSome(a, game, pick, 8);
  await playUntilAnswerable(a, game, pick);
  expect(await storedEvents(a, game.id), "A played a real run of moves").toBeGreaterThan(0);
  const chain = await a.locator(".decision .chain").first().innerText();
  expect(chain, "A really did play past the opening turn").not.toContain("Turn 1 ·");
  expect(await a.locator("button.option:not([disabled])").count()).toBeGreaterThan(0);

  // B opens the game by its address alone and is told, rather than shown.
  await b.goto(`/#/t/${game.id}`);
  await expect(b.getByText("This game is open in another tab")).toBeVisible();
  await expect(b.locator(".decision .chain")).toHaveCount(0);

  await b.getByRole("button", { name: "Take over" }).click();
  await expect(atTheGame(b)).toBeVisible();
  await readScreen(b);
  expect(
    await b.locator(".decision .chain").first().innerText(),
    "B's FIRST frame is the game A left, not the game B's Worker booted with",
  ).toBe(chain);

  // And A is read-only: the answer bar is the lock's strip, and the board's
  // own clicks are refused in the page before the Worker is asked.
  await expect(a.getByText("This game was opened in another tab")).toBeVisible();
  await expect(a.getByRole("button", { name: "Take back" })).toBeVisible();
  await expect(a.locator("button.option")).toHaveCount(0);
  expect(watchA.failures).toEqual([]);
});

// **The two cells below are PR #37's F1** (`docs/plans/2026-09-22-web-frontend-
// m6.5b2-pr37-fixes.md`, R-F1): an export took the Worker's memory whenever the
// Worker answered, and a tab's memory is a snapshot from its own boot that only
// the tab HOLDING the game ever writes to. The review measured a backup one
// event short of IndexedDB, both describing a playing game. The rule now is one
// whole pair from one source, and the Worker's memory is that source only where
// this tab owns the game at the current ownership generation.

test("a tab that booted with a snapshot of a game backs up what was COMMITTED since, and the file restores it", async ({
  page: a,
  context,
  browser,
}) => {
  const watchA = watchForBreakage(a);
  const game = await startGame(a, { local: true });
  const pick = rng(0x5354_414c); // "STAL"
  await playSome(a, game, pick, 6);
  await playUntilAnswerable(a, game, pick);

  // **B boots AFTER the game has moves, and that is the whole precondition.**
  // The two-tab cell above boots B first, so B's store knows nothing of the
  // game and there is no snapshot to be stale — which is why it could never
  // have seen this. Here B's Worker loads every stored pair at its boot, this
  // game among them, and lists it on the door.
  const b = await context.newPage();
  const watchB = watchForBreakage(b);
  await b.goto("/");
  await expect(b.locator(`.on-this-device a[href="#/t/${game.id}"]`)).toBeVisible();
  await localHost(b);
  // Nothing moves in A while it waits on an answerable screen, so what is
  // committed now is what B's store was loaded with.
  const booted = await storedEvents(a, game.id);
  expect(booted, "B's Worker booted with a snapshot of a PLAYED game").toBeGreaterThan(0);

  // A plays on, and commits past B's snapshot.
  const now = await playUntilCommitted(a, game, pick, booted);
  expect(now, "A committed at least one move B's snapshot does not have").toBeGreaterThan(booted);

  const all = await downloaded(b, () =>
    b.getByRole("button", { name: "Back up everything", exact: true }).click(),
  );
  const backed = backedUp(all.text, game.id);
  const stored = await storedPair(b, game.id);
  expect(backed.record, "the backup's record is IndexedDB's, byte for byte").toBe(stored.record);
  expect(backed.table, "and its table is IndexedDB's — one pair, from one source").toBe(
    stored.table,
  );
  expect(eventsOf(backed.record!), "with every event A committed after B booted").toBe(now);

  // **And the file does what a backup is for**: on a profile that has never
  // seen this game, it restores to the same game, move for move.
  const fresh = await newContextWithoutWorker(browser);
  try {
    const f = await fresh.newPage();
    const watchF = watchForBreakage(f);
    await f.goto("/");
    // Anti-vacuity: an empty profile, said by the screen itself.
    await expect(f.getByText("no game on this device yet")).toBeVisible();
    await f.getByLabel("Restore from a backup").setInputFiles(all.file);
    const row = f.locator(".restore-report li").filter({
      has: f.locator(".restore-id", { hasText: game.id }),
    });
    await expect(row.locator(".meta")).toHaveText("restored", { timeout: 60_000 });
    expect(await storedEvents(f, game.id), "the restored game has every committed event").toBe(
      now,
    );
    expect((await storedPair(f, game.id)).record, "and it is the backed-up record").toBe(
      backed.record,
    );
    // It opens as a game, which is what "restored" has to mean.
    await f.goto(`/#/t/${game.id}`);
    await expect(f.locator(".app:not(.setup-table)")).toBeVisible();
    await readScreen(f);
    expect(watchF.failures).toEqual([]);
  } finally {
    await fresh.close();
  }
  expect(watchA.failures).toEqual([]);
  expect(watchB.failures).toEqual([]);
});

test("a tab that lost the game exports what the new owner COMMITTED, not its own memory", async ({
  page: a,
  context,
}) => {
  const watchA = watchForBreakage(a);
  const game = await startGame(a, { local: true });
  const pick = rng(0x4f57_4e52); // "OWNR"
  await playSome(a, game, pick, 6);
  await playUntilAnswerable(a, game, pick);
  const left = await storedEvents(a, game.id);
  expect(left, "A played a real run of moves before it lost the game").toBeGreaterThan(0);

  // B takes the game over, the two-tab cell's own flow.
  const b = await context.newPage();
  await b.goto(`/#/t/${game.id}`);
  await expect(b.getByText("This game is open in another tab")).toBeVisible();
  await b.getByRole("button", { name: "Take over" }).click();
  await expect(atTheGame(b)).toBeVisible();
  await readScreen(b);
  await expect(a.getByText("This game was opened in another tab")).toBeVisible();

  // **B commits past what A's memory holds.** A's module store stopped at
  // `left` when the lock went: nothing writes to a tab's memory but that tab's
  // own calls, and A may no longer make one.
  const now = await playUntilCommitted(b, game, rng(0x5441_4b45), left); // "TAKE"
  expect(now).toBeGreaterThan(left);
  const stored = await storedPair(a, game.id);

  // A, read-only, exports both ways the game screen offers — and both are the
  // committed pair B built, not the snapshot A was left holding.
  const file = await downloaded(a, () =>
    a.getByRole("button", { name: "Save game file" }).click(),
  );
  expect(file.text, "Save game file is B's committed record, byte for byte").toBe(stored.record);
  expect(eventsOf(file.text)).toBe(now);

  const one = await downloaded(a, () =>
    a.getByRole("button", { name: "Back up", exact: true }).click(),
  );
  const backed = backedUp(one.text, game.id);
  expect(backed.record, "the backup is B's committed record, byte for byte").toBe(stored.record);
  expect(backed.table, "and B's committed table").toBe(stored.table);
  expect(watchA.failures).toEqual([]);
});

/** Take the Web Locks API away from ONE local host — the browser `worker.ts`'s
 *  `!hasLocks()` arm is written for (Chrome before 69, Firefox before 96,
 *  Safari before 15.4).
 *
 *  **Inside the Worker, and before it is asked to hold anything**: `hasLocks()`
 *  is read on every hold (`worker.ts`'s `hold`), not once at boot, so a host
 *  that booted with locks and is shadowed afterwards takes its NEXT game the
 *  no-locks way. An own property on the navigator shadows the prototype's
 *  getter. Answers what `navigator.locks` now reads, so the cell can assert
 *  the premise rather than assume it. */
async function withoutLocks(host: Worker): Promise<string> {
  return host.evaluate(() => {
    Object.defineProperty(navigator, "locks", { value: undefined, configurable: true });
    return typeof navigator.locks;
  });
}

/** The ownership generation IndexedDB holds for a game, read from the page. */
async function storedGeneration(page: Page, id: string): Promise<number> {
  return page.evaluate(
    (game) =>
      new Promise<number>((resolve, reject) => {
        const open = indexedDB.open("yokohama-local");
        open.onerror = () => reject(new Error("IndexedDB would not open"));
        open.onsuccess = () => {
          const db = open.result;
          const get = db.transaction("meta", "readonly").objectStore("meta").get(game) as IDBRequest<
            { generation: number } | undefined
          >;
          get.onsuccess = () => {
            resolve(get.result?.generation ?? 0);
            db.close();
          };
          get.onerror = () => reject(get.error ?? new Error("the read failed"));
        };
      }),
    id,
  );
}

// **The generation's own wiring, in a browser** (fix round 1, review M-1).
// With Web Locks the steal's `AbortError` clears the old holder's `held` entry
// before any export runs, so every cell above reaches the STORE branch through
// "not held" and none of them can tell whether the Worker hands `exportPair`
// the real stored generation. Without Web Locks nothing clears it: B opening
// the game takes a newer generation and never touches A, so A still holds its
// own, older one — and only the generation read inside the export arm says
// that A's memory is no longer the game.
test("without Web Locks, a tab whose game was opened elsewhere exports the new owner's COMMITTED record", async ({
  page: a,
  context,
}) => {
  const watchA = watchForBreakage(a);
  // A boots on the DOOR, which starts its host, and loses its locks there —
  // before the game it is about to create is held.
  await a.goto("/");
  await expect(a.getByRole("heading", { name: "On this device" })).toBeVisible();
  const hostA = await localHost(a);
  expect(await withoutLocks(hostA), "A's host has no Web Locks").toBe("undefined");
  await a.getByRole("button", { name: "New game", exact: true }).click();
  await expect(a.getByRole("heading", { name: "New game against the bot", exact: true })).toBeVisible();
  await a.getByRole("button", { name: "Deal" }).click();
  await a.waitForURL(/#\/t\/local-[0-9a-f]+$/, { timeout: 60_000 });
  const game = { id: a.url().split("#/t/")[1]! };
  await a.locator(".app.setup-table.dealing button.deal-pick:not([disabled])").first().click();
  await expect(a.locator(".app:not(.setup-table)")).toBeVisible();
  expect(a.workers(), "the host told there are no locks is the one that holds the game").toEqual([
    hostA,
  ]);
  // **The premise, asked of the browser's own lock manager**: nothing holds
  // this game's lock, so A holds it by generation alone.
  const lockName = `yokohama:local:${game.id}`;
  const heldLocks = async () =>
    a.evaluate(async () => ((await navigator.locks.query()).held ?? []).map((l) => l.name));
  expect(await heldLocks(), "A took the game without a Web Lock").not.toContain(lockName);

  const pick = rng(0x4e4f_4c4b); // "NOLK"
  await playSome(a, game, pick, 6);
  await playUntilAnswerable(a, game, pick);
  const left = await storedEvents(a, game.id);
  expect(left, "A played a real run of moves").toBeGreaterThan(0);
  const generationA = await storedGeneration(a, game.id);

  // B, the same way: booted on the door, locks taken away, and only then sent
  // to the game — by its address, in the same document, so the host that was
  // shadowed is the host that holds.
  const b = await context.newPage();
  const watchB = watchForBreakage(b);
  await b.goto("/");
  await expect(b.locator(`.on-this-device a[href="#/t/${game.id}"]`)).toBeVisible();
  const hostB = await localHost(b);
  expect(await withoutLocks(hostB), "B's host has no Web Locks").toBe("undefined");
  await b.evaluate((id) => {
    location.hash = `#/t/${id}`;
  }, game.id);
  await expect(b.locator(".app:not(.setup-table)")).toBeVisible();
  // Last opener wins: nobody offered B a Take over, because nothing could say
  // the game was open elsewhere.
  await expect(b.getByText("This game is open in another tab")).toHaveCount(0);
  await readScreen(b);
  expect(b.workers()).toEqual([hostB]);

  const now = await playUntilCommitted(b, game, rng(0x4e45_5752), left); // "NEWR"
  expect(now).toBeGreaterThan(left);
  expect(
    await storedGeneration(a, game.id),
    "B took a NEWER generation — which is how its writes passed the fence",
  ).toBeGreaterThan(generationA);
  // **And A was never told.** No abort reached it, so its `held` entry still
  // names the older generation — the state this cell exists to export from.
  await expect(a.getByText("This game was opened in another tab")).toHaveCount(0);

  const stored = await storedPair(a, game.id);
  const file = await downloaded(a, () =>
    a.getByRole("button", { name: "Save game file" }).click(),
  );
  expect(file.text, "A's Save game file is B's committed record, byte for byte").toBe(
    stored.record,
  );
  expect(eventsOf(file.text)).toBe(now);
  const one = await downloaded(a, () =>
    a.getByRole("button", { name: "Back up", exact: true }).click(),
  );
  const backed = backedUp(one.text, game.id);
  expect(backed.record, "and A's backup is B's committed record").toBe(stored.record);
  expect(backed.table, "with B's committed table").toBe(stored.table);
  expect(watchA.failures).toEqual([]);
  expect(watchB.failures).toEqual([]);
});
