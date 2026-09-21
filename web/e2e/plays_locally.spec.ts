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
import { expect, test, type Page, type Worker } from "@playwright/test";
import { archiveDir, BASE_URL, haveBrowser } from "../playwright.config";
import { act, deadEnd, readScreen, rng, startGame, watchForBreakage, WALKER } from "./harness";
import type { Affordance } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly — the guard
// `every_spec_can_skip.spec.ts` reads this directory for.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The walk's own cap, the same figure every other walk here carries: well
 *  above what a game takes, low enough that a loop fails as a loop. */
const CAP = 4000;

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

/** This build's semantics rev, which is the archive's own directory level.
 *  Asserted against the path the server reports rather than assumed. */
const REV = "rev7";

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

test("a whole game against the bot plays in the browser with no /api/ request at all", async ({
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
  // **The baseline is taken HERE**, with the game already on screen: the sign
  // in, the health read, the labels and the art all happen before it and are
  // the server's business. `/api/archive` is the one exemption, and it is not
  // reached until the game is over.
  const before = apiCalls.length;
  expect(before, "the boot really did talk to the server, so zero below means something").toBeGreaterThan(0);

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
  expect(moves, "the walk must not hit its own cap").toBeLessThan(CAP);
  await expect(page.getByText("The game is over.")).toBeVisible();

  // **A real END, and not merely a screen that says so.** `/result` is served
  // by the module here like every other route, so the ranked standing is the
  // local host's own answer — two seats, and the winner MARKED from
  // `Placing.won` rather than by comparing totals, because two seats can tie
  // on points and be split by the engine's tie-break.
  await expect(page.locator("table.outcome tbody tr")).toHaveCount(2);
  await expect(page.locator("table.outcome tr.won")).toHaveCount(1);

  const duringPlay = apiCalls.slice(before);
  expect(duringPlay, "no /api/ request of ANY kind once the local game is on screen").toEqual([]);

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

  // **Send to the archive** (R-6.5b.6): the record and its table, sent to the
  // server, which replays the record and chooses the leaf from the state it
  // reaches. Nothing in the body says where the copy goes.
  const archivePath = `browser/${SEGMENT}/${REV}/over/${game.id}.json`;
  await page.getByRole("button", { name: "Send to the archive" }).click();
  await expect(page.locator(".meta.archived")).toHaveText(`archived under ${archivePath}`);
  const onDisk = path.join(archiveDir(), ...archivePath.split("/"));
  const archived = fs.readFileSync(onDisk, "utf8");
  expect(archived, "the archived record is the downloaded file, byte for byte").toBe(saved);
  // The table lands beside it, stripped of a lock and a watcher roster it
  // never had. Its presence is the operator's sign that the pair is whole.
  expect(fs.existsSync(onDisk.replace(/\.json$/, ".table.json"))).toBe(true);

  // **And the reading holds to the END of the cell**, which the slice above
  // cannot say by itself: it is one moment, and the download and the send come
  // after it. Everything this page asked the API for since the game opened is
  // the archive's own POST and nothing else — the one exemption §6.5 names.
  expect(apiCalls.slice(before)).toEqual(["POST /api/archive"]);

  // eslint-disable-next-line no-console
  console.log(
    `LOCAL WALK: game ${game.id} finished in ${moves} clicks, ` +
      `${(JSON.parse(saved) as { events: unknown[] }).events.length} events, ` +
      `${saved.length} bytes of record, ${duringPlay.length} /api/ requests while playing, via ` +
      [...used].map(([k, n]) => `${k}×${n}`).join(", "),
  );

  // **The determinism pin's pair**, written only when asked for (spec §6.4).
  // The TABLE is the local document and not the archived copy, because "what
  // Send to the archive sends" is what the pin is about.
  if (process.env.YOKOHAMA_WEB_UPDATE_PIN) {
    fs.mkdirSync(FIXTURES, { recursive: true });
    fs.writeFileSync(PIN_RECORD, saved);
    fs.writeFileSync(PIN_TABLE, stored.table!);
    const head = execFileSync("git", ["rev-parse", "--short", "HEAD"], { cwd: HERE })
      .toString()
      .trim();
    fs.writeFileSync(
      PIN_SIDECAR,
      `${JSON.stringify(
        {
          release: `yokohama-web 0.5.0-m6.5b (branch web-room-m6.5b at ${head})`,
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
  await page.getByRole("button", { name: "Play the bot here" }).click();
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
  await expect(page.getByRole("heading", { name: "Yokohama" })).toBeVisible();
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
  expect(watch.failures).toEqual([]);
});

test("a real trap is recovered by a fresh local host, and play continues from the last committed move", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  let started = 0;
  page.on("worker", () => {
    started += 1;
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
  await playUntilCommitted(page, game, pick, committed);

  // The trap itself is a refused call, not a page error: the one console
  // channel a player would call broken stays empty.
  expect(watch.failures).toEqual([]);
});

test("the first screen renders On this device with the API refused, and a local game plays", async ({
  page,
}) => {
  // Every `/api/` request refused at the edge — the health read, the door, the
  // labels and the art. Level 2 is "the API unreachable with the page loaded"
  // (spec §8), which is exactly this.
  await page.route("**/api/**", (r) => r.fulfill({ status: 503, body: "down" }));
  const watch = watchForBreakage(page, [{ status: 503, urlIncludes: "/api/" }]);

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Play the bot here" })).toBeVisible();
  // **The other way on is drawn from the BUILD, not from an answer** (M6.5b2,
  // R-6.5b2.7): every `/api/` request is being refused here, and the link is on
  // the screen anyway with the turnstile's own address in it. This is the half
  // no jsdom cell can make — there the refusal is a stub; here it is the edge.
  // It is READ and never followed: `/enter` is a navigation, and following it
  // would take this page off the app.
  await expect(page.getByRole("link", { name: "Play with others" })).toHaveAttribute(
    "href",
    "/enter",
  );
  // **The door reports nothing about a server it never asks** (R-6.5b2.7). The
  // line that does report one is the LOBBY's, one hash away, and it is still
  // there — so the claim this cell has always made is read where it now lives.
  await expect(page.getByText("not connected")).toHaveCount(0);
  await page.goto("/#/lobby");
  await expect(page.getByText("not connected")).toBeVisible();

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
  await expect(b.getByRole("heading", { name: "Yokohama" })).toBeVisible();
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
