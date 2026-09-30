// **The open door's plain two-person game** (cycle E Task 10.K; the straw-poll
// row 114: "a plain two-person game never measured"). Two PLAIN contexts — no
// credential, no claimed header — each named at the client's own name step
// (`signIn`), so on a host running `YOKOHAMA_WEB_IDENTITY=open` (the door
// window's state 1, or the local binary) the typed address IS the identity.
// The names are address-shaped, `state1-a@window.test` and
// `state1-b@window.test`: the name step takes an address as well as a plain
// name (`nameRule.ts`), and an address is the shape the tally's other walks use.
//
// A opens "Window game" (Blue seat open, no password); B opens the link and
// joins as Blue; both keep a card; the pages then play until EACH player has
// applied ten moves, counted by the newest `data-event` row (the record's own
// event index) on the acting page before and after each click, so a picker
// step that applies nothing counts for nothing. A then abandons the table from the lobby row.
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
// through the query API. It makes no `request.*` call at all.
//
// It is a live spec for `live.node.test.ts`'s rules: the create is armed
// through `awaitCreate`, and the table is abandoned in `afterEach` if the walk
// died before its own abandon.

import { writeFileSync } from "node:fs";
import type { Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, capReached, deadEnd, readScreen, rng, signIn, tally } from "./harness";
import { abandonOpened, awaitCreate, createdTable, type Opening } from "./live";
import { hasMove } from "./shared_table";
import { expect, newContextWithoutWorker, test } from "./sw";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const NAME_A = "state1-a@window.test";
const NAME_B = "state1-b@window.test";
const TABLE = "Window game";
/** Applied moves each player must make. */
const EACH = 10;
/** Iterations of the loop below before it gives up (the game is short). */
const CAP = 1500;

let opened: Opening | null = null;

test.afterEach(async () => {
  const table = opened;
  opened = null;
  await abandonOpened(table);
});

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
  await b.goto(`/#/t/${id}`);
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
      const before = await shown(pages[who]);
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
      if (after > before) applied[who] += after - before;
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
  await a.goto("/#/lobby");
  await a.getByRole("button", { name: new RegExp(`^abandon( and save)? ${TABLE}$`) }).click();
  await a.getByRole("button", { name: new RegExp(`^confirm abandon( and save)? ${TABLE}$`) }).click();
  await expect(a).toHaveURL(/#\/lobby$/);
  opened = null;

  // eslint-disable-next-line no-console
  console.log(
    `state1_game: table ${id} (${link || "no link read"}); applied moves a ${applied.a}, b ${applied.b}; ` +
      `${iterations} loop iterations; ${Math.round((Date.now() - started) / 1000)}s`,
  );
  expect(applied.a).toBeGreaterThanOrEqual(EACH);
  expect(applied.b).toBeGreaterThanOrEqual(EACH);
});
