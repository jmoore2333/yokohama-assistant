// **The open door's plain two-person game** (cycle E Task 10.K; the straw-poll
// row 114: "a plain two-person game never measured"). Two PLAIN contexts — no
// credential, no claimed header — each named at the client's own name step
// (`signIn`), so on a host running `YOKOHAMA_WEB_IDENTITY=open` (the door
// window's state 1, or the local binary) the typed name IS the identity.
// The names are plain, `state1-a` and `state1-b`, on every target: under the
// open door a name is a name (security S2b, C1), and the name step refuses an
// address before any request (`openDoorVerdict`, `nameRule.ts`).
//
// A opens "Window game" (Blue seat open, no password); B opens the link and
// joins as Blue; both keep a card; the pages then play until EACH player has
// applied ten moves, counted by ONE high-water mark of the newest `data-event`
// row (the record's own event index) shared by both pages: a reply that adds
// no event (a picker opening) clears a page's account, and a per-page baseline
// would then credit that page with the game's whole history. The mark is the
// game's event count, printed for `tally-report --moves`. A then abandons the
// table from the lobby row, and the row is asserted GONE. The tally carries the
// setup and the teardown as fixed overhead, so a per-move figure is an upper
// bound on the play itself.
//
// **Opt-in**, collected only under `YOKOHAMA_WEB_STATE1_GAME=1`
// (`playwright.config.ts`), on whichever target the run names:
//
//     YOKOHAMA_WEB_STATE1_GAME=1 [YOKOHAMA_WEB_TALLY=<file>] [YOKOHAMA_WEB_STATE1_OUT=<file>] \
//       [YOKOHAMA_WEB_E2E_TARGET=live] npx playwright test state1_game
//
// `YOKOHAMA_WEB_TALLY` writes the request tally for both pages
// (`scripts/tally-report.mjs`); `YOKOHAMA_WEB_STATE1_OUT` names a file that
// receives the table id, one line, so the orchestrator can count the game
// through the query API. The walk makes no `request.*` call; its `afterEach`
// tries A's lobby row first and falls back to `abandonOpened` only if the row
// did not go. A plain context sends no claimed header, so the fallback first
// hands A's context the name the walk signed in with (`setExtraHTTPHeaders`):
// without it the DELETE `abandonOpened` sends would be anonymous under the open
// door, and refused (cycle F, E's m-7).
//
// **The walk ends on the ARCHIVED record** (E's m-6): locally the abandon's save
// is on this machine's disk, and the walk asserts the record's event count is
// the one the pages showed. Live, the Worker's bucket is not readable from here
// and the count is the orchestrator's, through the query API.
//
// It is a live spec for `live.node.test.ts`'s rules: the create is armed
// through `awaitCreate`, and the table is abandoned in `afterEach` if the walk
// died before its own abandon.

import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Page } from "@playwright/test";
import { archiveDir, haveBrowser } from "../playwright.config";
import { REV } from "./caps";
import { act, capReached, deadEnd, readScreen, rng, signIn, tally } from "./harness";
import { LIVE, abandonOpened, awaitCreate, createdTable, identityHeaders, type Opening } from "./live";
import { hasMove } from "./shared_table";
import { expect, newContextWithoutWorker, test } from "./sw";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const NAME_A = "state1-a";
const NAME_B = "state1-b";
const TABLE = "Window game";
/** Applied moves each player must make. */
const EACH = 10;
/** Iterations of the loop below before it gives up (the game is short). */
const CAP = 1500;

let opened: Opening | null = null;

test.afterEach(async () => {
  const table = opened;
  opened = null;
  if (table !== null && LIVE) {
    const id = await table.id;
    try {
      if (id !== null) {
        await abandonRow(table.page, id);
        return;
      }
    } catch {
      // the row did not go: the helper's DELETE is the fallback
    }
    // **The fallback's DELETE goes through A's context, which holds no claimed
    // header** (a plain context: the client attaches it itself, and only to its
    // own requests). Under the open door an anonymous DELETE is refused, so the
    // fallback would abandon nothing. A's context is handed the name the walk
    // signed in with; nothing else is using it by now.
    await table.page.context().setExtraHTTPHeaders(identityHeaders(NAME_A));
  }
  await abandonOpened(table);
});

/** **The lobby reached the server** (cycle F, E's m-5). `Lobby.tsx`'s state C —
 *  "Can't reach the server" — REPLACES the actions and every list, so a row that
 *  is "gone" proves nothing on a lobby that is showing it. The lists are drawn
 *  (the positive anchor: the card is the only thing that removes them), the
 *  first read has settled, and the card is not drawn. */
async function expectLobbyReached(page: Page, when: string): Promise<void> {
  await expect(page.getByRole("heading", { name: "Your tables", level: 3 }), `${when}: the lobby's lists are drawn`).toBeVisible();
  await expect(page.locator(".your-games").getByText(/^loading/), `${when}: the lobby's first read has settled`).toHaveCount(0);
  await expect(page.getByText("Can't reach the server"), `${when}: the unreachable-server card is not drawn`).toHaveCount(0);
}

/** A's lobby row for table `id`, found by the id in its link. */
const rowOf = (page: Page, id: string) =>
  page.locator("li.lobby-row", { has: page.locator(`a.row-link[href$="#/t/${id}"]`) });

/** Abandon `id` from A's lobby row (a playing table's screen has no Abandon)
 *  and require the row to be gone. */
async function abandonRow(page: Page, id: string): Promise<void> {
  await page.goto("/#/lobby");
  const row = rowOf(page, id);
  await row.getByRole("button", { name: new RegExp(`^abandon( and save)? ${TABLE}$`) }).click();
  await row.getByRole("button", { name: new RegExp(`^confirm abandon( and save)? ${TABLE}$`) }).click();
  await expect(row, "the abandoned table's row is gone from A's lobby").toHaveCount(0, { timeout: 30_000 });
  // …on a lobby that reached the server: a card that replaced the lists would
  // read the same.
  await expectLobbyReached(page, "after the abandon");
}

/** The number of events the page's account shows: the newest row's index + 1. */
async function shown(page: Page): Promise<number> {
  const last = page.locator(".turn-account li[data-event]").last();
  if ((await last.count()) === 0) return 0;
  const n = Number(await last.getAttribute("data-event"));
  return Number.isFinite(n) ? n + 1 : 0;
}

test("a plain two-person game through the open door: ten applied moves each, then abandoned", async ({ browser }) => {
  test.setTimeout(20 * 60 * 1000);
  const started = Date.now();
  const a = await (await newContextWithoutWorker(browser, {})).newPage();
  const b = await (await newContextWithoutWorker(browser, {})).newPage();
  tally(a, "a");
  tally(b, "b");
  const pages = { a, b };

  // ---- A opens the table ---------------------------------------------------
  await signIn(a, NAME_A);
  await expectLobbyReached(a, "after sign-in");
  await a.getByRole("button", { name: "New table" }).click();
  await expect(a.getByRole("heading", { name: "New table", exact: true, level: 3 })).toBeVisible();
  await a.getByRole("combobox", { name: "Blue seat", exact: true }).selectOption("open");
  await a.getByLabel("Name", { exact: true }).fill(TABLE);
  opened = awaitCreate(a);
  await a.getByRole("button", { name: "Open table" }).click();
  const id = await createdTable(opened);
  await expect(a).toHaveURL(new RegExp(`#/t/${id}$`));
  await expect(a.getByLabel("Invite link", { exact: true })).toBeVisible();
  const link = await a.getByLabel("Invite link", { exact: true }).inputValue().catch(() => "");
  if (process.env.YOKOHAMA_WEB_STATE1_OUT) writeFileSync(process.env.YOKOHAMA_WEB_STATE1_OUT, `${id}\n`);

  // ---- B opens the link and joins as Blue ----------------------------------
  await signIn(b, NAME_B, `/#/t/${id}`);
  await expect(b.getByRole("heading", { name: TABLE, exact: true, level: 3 })).toBeVisible();
  await b.getByRole("button", { name: "Join as Blue" }).click();

  // ---- both keep a card ----------------------------------------------------
  for (const p of [a, b]) {
    const keep = p.getByRole("button", { name: /^keep O\d\d/ }).first();
    await expect(keep).toBeVisible({ timeout: 30_000 });
    await keep.click();
  }
  await expect(a.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  await expect(b.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });

  // ---- play until each has applied EACH moves ------------------------------
  const pick = rng(0x5354_4531); // "STE1"
  const applied = { a: 0, b: 0 };
  /** The game's event count: the highest `data-event` + 1 either page has shown. */
  let seen = 0;
  const done = () => applied.a >= EACH && applied.b >= EACH;
  let iterations = 0;
  let idle = 0;
  let first: "a" | "b" = "a";
  while (!done()) {
    let moved = false;
    const order: ("a" | "b")[] = first === "a" ? ["a", "b"] : ["b", "a"];
    for (const who of order) {
      const screen = await readScreen(pages[who]);
      if (screen.over) throw new Error(`the game ended before each player applied ${EACH} moves (a ${applied.a}, b ${applied.b})`);
      if (!hasMove(screen)) continue;
      let acted: Awaited<ReturnType<typeof act>> = null;
      try {
        acted = await act(pages[who], screen, pick);
      } catch (e) {
        // A click that raced the poll and missed is forgiven only while the
        // screen no longer offers a move (two_browsers' rule).
        if (hasMove(await readScreen(pages[who]))) throw e;
      }
      if (acted === null && hasMove(await readScreen(pages[who]))) throw await deadEnd(pages[who], { id }, iterations);
      await readScreen(pages[who]); // settles: waits out the request in flight
      const after = await shown(pages[who]);
      if (after > seen) {
        applied[who] += after - seen;
        seen = after;
      }
      first = who;
      moved = true;
      break;
    }
    iterations += 1;
    if (iterations > CAP) throw await capReached(a, { id }, iterations);
    if (moved) idle = 0;
    else {
      idle += 1;
      if (idle > 60) throw await deadEnd(a, { id }, iterations);
      await a.waitForTimeout(400);
    }
  }

  // ---- A abandons, from the lobby's row (a playing table's screen has none) -
  await abandonRow(a, id);
  opened = null;

  // eslint-disable-next-line no-console
  console.log(
    `state1_game: table ${id} (${link || "no link read"}); applied moves a ${applied.a}, b ${applied.b}; ${seen} events in the game (tally-report --moves ${seen}); ` +
      `${iterations} loop iterations; ${Math.round((Date.now() - started) / 1000)}s`,
  );

  // ---- the archived record is the game (cycle F, E's m-6) ------------------
  // The two asserts that stood here, `applied.a >= EACH` and `applied.b >= EACH`,
  // could not fail: the loop above ends only when both hold. The record the
  // abandon saved can disagree with what the pages showed, so it is read.
  if (LIVE) {
    // eslint-disable-next-line no-console
    console.log(`state1_game: the archived record is the Worker's, not readable from here; ${seen} events counted by the pages — the query API has the record's`);
    return;
  }
  const at = join(archiveDir(), REV, "abandoned", `${id}.json`);
  await expect.poll(() => existsSync(at), { message: `the abandon's save ${at} is on disk`, timeout: 15_000 }).toBe(true);
  const archived = JSON.parse(readFileSync(at, "utf8")) as { events?: unknown };
  expect(Array.isArray(archived.events), "the archived record carries its events").toBe(true);
  const recorded = (archived.events as unknown[]).length;
  expect(recorded, `the archived record's event count is the count the pages showed (${seen})`).toBe(seen);
  // Implied by the line above and the loop's end, and stated against the
  // RECORD so a change to the counter cannot quietly weaken what the header
  // claims: the saved game holds every player's ten.
  expect(recorded, "the archived record holds at least each player's ten events").toBeGreaterThanOrEqual(2 * EACH);
});
