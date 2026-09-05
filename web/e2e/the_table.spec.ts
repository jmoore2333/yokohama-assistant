// M5's browser cells: the things about the table that jsdom cannot see.

import { expect, test, type Locator, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { haveBrowser } from "../playwright.config";
import { pyramidFits, tileWidthFor } from "../src/fit";
import { act, readScreen, rng, startGame, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** Four screens, and the fourth is there for the BRANCH the other three miss.
 *
 *  1440x900 is the M3.5 reference, 1710x869 the owner's, 2560x1440 the cap. All
 *  three fit their four rows, so all three take the same branch below and the
 *  `else` was reached by nothing — a half-tested conditional. 1440x600 is a
 *  short laptop viewport where the table is ~516px tall and the height term
 *  works out UNDER the 10rem legibility floor at every player count (~9.7rem
 *  at 2P, less at 3P and 4P, where the same height is divided by wider rows):
 *  the floor wins, the pyramid is taller than the table, and the table has to
 *  scroll it. That is the rule behaving correctly at a size it cannot satisfy,
 *  and it is the only one of the four that exercises the other half of the
 *  cell. */
const VIEWPORTS = [
  [1440, 900],
  [1710, 869],
  [2560, 1440],
  [1440, 600],
] as const;

const settle = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

/** A few moves, so the board being measured has pieces on it.
 *
 *  The fit rule's arithmetic does not depend on them — the pyramid's shape is
 *  the same at move 0 — but the SIZE it produces is only interesting on a board
 *  that is being played, and an empty board is also the one configuration where
 *  a tile drawing nothing would still measure correctly. The pick stream is
 *  seeded so a failure names a reproducible sequence; the GAME seed is the
 *  server's, and `startGame` carries it. */
async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

for (const players of [2, 3, 4] as const) {
  test(`the pyramid is the size the rule says, and its rows are in view where it says, at ${players}P`, async ({ page }) => {
    const watch = watchForBreakage(page);
    await startGame(page, { players });
    await playAFew(page, 10, players * 1000 + 7);
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await settle(page);
      const read = await page.evaluate(() => {
        const table = document.querySelector(".table")!;
        // Scoped to the board inside the table being measured: `.board-row` is
        // not unique to it (a drawer's board would match too), and a
        // document-wide read would mix another board's rows into `widest`,
        // `rows` and `bottoms` — the three terms the rule is checked against.
        const board = table.querySelector(".board")!;
        const rows = [...board.querySelectorAll(".board-row")];
        return {
          tableW: table.clientWidth,
          tableH: table.clientHeight,
          tileW: board.querySelector(".tile")!.getBoundingClientRect().width,
          widest: Math.max(...rows.map((r) => r.children.length)),
          rows: rows.length,
          bottoms: rows.map((r) => Math.round(r.getBoundingClientRect().bottom)),
          scrollH: table.scrollHeight,
          innerH: window.innerHeight,
        };
      });
      const want = tileWidthFor(read.widest, read.rows, read.tableW, read.tableH);
      expect(
        Math.abs(read.tileW - want),
        `${players}P at ${w}x${h}: rendered ${read.tileW.toFixed(1)}px, the rule says ${want.toFixed(1)}`,
      ).toBeLessThanOrEqual(1);
      // **The branch is named in the reading, and the branch taken is a fact
      // about the VIEWPORT, not about the product.** Three of the four fit and
      // one does not, by construction, so a run in which every line says the
      // same thing is a run whose `VIEWPORTS` list has drifted.
      let branch: string;
      if (pyramidFits(want, read.rows, read.tableH)) {
        for (const b of read.bottoms) {
          expect(b, `${players}P at ${w}x${h}: a row's bottom (${b}) is below the viewport (${read.innerH})`).toBeLessThanOrEqual(read.innerH);
        }
        // RECORDED, never gated (ruling R-T8a). `.table` holds the marks legend
        // and `.board`'s `padding-bottom` under the pyramid, and Task 12's
        // sections will join them, so a scrolling table under four fitted rows
        // is the design — but the figure belongs in the record, because it is
        // how far the owner has to scroll to reach what is under the board.
        branch =
          `rows in view · the legend and the sections under the pyramid scroll ` +
          `by ${read.scrollH - read.tableH}px VERTICALLY — recorded`;
      } else {
        expect(read.scrollH, `${players}P at ${w}x${h}: the rule cannot fit, so the table must scroll`).toBeGreaterThan(read.tableH);
        // "gated" names the ASSERTION just above, and the overflow it is about
        // is the PYRAMID's — a different quantity from the branch above, where
        // the same scroll is the legend and the sections below a pyramid that
        // fits. The two lines print the same arithmetic about different things,
        // which is why each says which.
        branch =
          `the rule cannot fit its rows, so the table scrolls the pyramid ` +
          `itself (${read.scrollH} > ${read.tableH}) — gated`;
      }
      console.log(`${players}P at ${w}x${h}: tile ${(read.tileW / 16).toFixed(2)}rem (rule ${(want / 16).toFixed(2)}rem) · ${branch}`);
    }
    expect(watch.failures).toEqual([]);
  });
}

test("Undo takes the pieces back off the tile", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  // Walk to the first placement picker and place on the first eligible tile.
  const pick = rng(0x554e_444f); // "UNDO"
  let placed: string | null = null;
  for (let i = 0; i < 30 && placed === null; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if (screen.tiles > 0 && screen.confirm) {
      const tile = page.locator("button.tile:not([disabled])").first();
      placed = (await tile.getAttribute("aria-label"))!.slice(0, 3);
      await tile.click();
      await page.getByRole("button", { name: "Confirm" }).click();
      break;
    }
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(placed, "never reached a placement picker").not.toBeNull();
  const piecesOn = () =>
    page.evaluate((id) => {
      const tile = [...document.querySelectorAll(".tile")].find(
        (t) => t.querySelector(".tile-id")?.textContent === id,
      )!;
      return tile.querySelector(".pieces")?.getAttribute("aria-label") ?? "";
    }, placed!);
  await expect.poll(piecesOn, { message: "the placement put no pieces on the tile" }).toMatch(/assistant/);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect.poll(piecesOn, { message: "Undo left the pieces on the tile" }).not.toMatch(/assistant/);
  expect(watch.failures).toEqual([]);
});

test("the Game menu switches between saved games from inside a game", async ({ page }) => {
  const watch = watchForBreakage(page);
  const a = await startGame(page, { players: 2 });
  await playAFew(page, 4, 0x4d45_4e55); // "MENU"
  await page.goto("/");
  const b = await startGame(page, { players: 2 });
  await page.locator(".game-menu > summary").click();
  await page.getByRole("button", { name: a.id, exact: true }).click();
  await expect(page.locator(".game-menu > summary")).toContainText(a.id);
  // **No second click on the summary here, and this cell is the only place the
  // reason can be checked.** The menu stays OPEN across a switch, and the list
  // it is showing has already recomputed against the game just loaded, so the
  // way back is a button already on screen; a second summary click would CLOSE
  // it. `GameMenu.test.tsx` cannot see either half — jsdom ships no UA
  // stylesheet for `details`, so a closed menu's buttons are just as findable
  // there — while Chromium's are `display: none`, and the first draft of this
  // line waited fifteen seconds for a button that was never going to appear.
  await page.getByRole("button", { name: b.id, exact: true }).click();
  await expect(page.locator(".game-menu > summary")).toContainText(b.id);
  expect(watch.failures).toEqual([]);
});

// ---- M5 W5b: the sections on the table --------------------------------------

test("an enabled section re-fetches once after a move, collapses while replaying, and persists across a reload", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  const portRequests: string[] = [];
  // **The two counters are the whole point of this cell (ruling R-T12b).**
  //
  // Spec §3.4's rule is "an enabled fetched section re-fetches after every
  // `setState`", and in `App.tsx` a `setState` of the game comes from exactly
  // three places: the `/input` reply, the re-read `GET /api/games/{id}` in
  // `send`'s catch branch, and `create`. So the count that follows from the rule
  // is one `/ports` per state-changing RESPONSE, not one per "move" — and a move
  // in `act()`'s sense is not one request: a placement clicks up to three tiles
  // and then Confirm, and every one of those four is its own `/input` post with
  // its own reply. Asserting `+1` would therefore be asserting which affordance
  // the seeded walk happened to take. This counts the posts instead and asserts
  // the RELATION, which is what the spec actually says.
  //
  // **Counted on the RESPONSE, and only where it succeeded.** These are named
  // for state-changing responses, and a request is not one: a refused `/input`
  // is a 4xx that changes nothing and triggers no refetch, so counting the post
  // would leave the relation below off by one on exactly the move worth
  // catching. `/ports` stays on the request side — it is what the client ASKED
  // for that this cell is about.
  const inputPosts: string[] = [];
  const reReads: string[] = [];
  page.on("request", (r) => {
    if (/\/ports$/.test(new URL(r.url()).pathname)) portRequests.push(r.url());
  });
  page.on("response", (r) => {
    if (!r.ok()) return;
    const path = new URL(r.url()).pathname;
    if (/\/input$/.test(path) && r.request().method() === "POST") inputPosts.push(r.url());
    if (/^\/api\/games\/[^/]+$/.test(path) && r.request().method() === "GET") reReads.push(r.url());
  });
  // Building sites is off by default; turn it on and see it fetched once.
  await page.locator(".boards-menu > summary").click();
  await page.getByRole("button", { name: "Building sites" }).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".table-sec.buildings .site-card").first()).toBeVisible();
  const before = portRequests.length;
  const postsBefore = inputPosts.length;
  const readsBefore = reReads.length;
  await playAFew(page, 1, 0x5245_4652); // "REFR"
  const stateChanges = () =>
    inputPosts.length - postsBefore + (reReads.length - readsBefore);
  await expect
    .poll(() => stateChanges(), { message: "the walk applied nothing, so nothing could refresh" })
    .toBeGreaterThan(0);
  await expect
    .poll(() => portRequests.length - before, {
      message: "a move did not refresh the port",
    })
    .toBe(stateChanges());
  // eslint-disable-next-line no-console
  console.log(
    `sections refresh: one move made ${stateChanges()} state-changing responses ` +
      `and ${portRequests.length - before} /ports requests`,
  );

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await track.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator(".table-sec.ports.collapsed")).toBeVisible();
  await expect(page.locator(".table-sec.tracks")).toBeVisible();
  await page.getByRole("button", { name: "Back to live" }).click();
  await expect(page.locator(".table-sec.ports .board-strip")).toBeVisible();
  await page.keyboard.press("Escape");

  await page.reload();
  await page.getByRole("button", { name: /^sim-/ }).first().click();
  await expect(page.locator(".table-sec.buildings")).toBeVisible();
  await page.locator(".boards-menu > summary").click();
  await expect(page.getByRole("button", { name: "Building sites" })).toHaveAttribute("aria-pressed", "true");
  expect(watch.failures).toEqual([]);
});

test("the port and laboratory strips sit side by side at 2560 and stack at 1710", async ({ page }) => {
  // Every other cell in this file declares one; without it a console error or a
  // 4xx during the two viewport changes goes unrecorded.
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  const tops = () =>
    page.evaluate(() => ({
      port: document.querySelector(".table-sec.ports")!.getBoundingClientRect().top,
      lab: document.querySelector(".table-sec.labs")!.getBoundingClientRect().top,
    }));
  await page.setViewportSize({ width: 2560, height: 1440 });
  await settle(page);
  await expect.poll(async () => (await tops()).port === (await tops()).lab, { message: "not side by side at 2560" }).toBe(true);
  await page.setViewportSize({ width: 1710, height: 869 });
  await settle(page);
  const t = await tops();
  expect(t.lab, "not stacked at 1710").toBeGreaterThan(t.port);
  expect(watch.failures).toEqual([]);
});

// ---- M5.5 A: a pile of four, built on purpose ------------------------------

/** Click the first enabled option row whose words match, or say there is none. */
async function answer(page: Page, words: RegExp): Promise<boolean> {
  const rows = page.locator("button.option:not([disabled])").filter({ hasText: words });
  if ((await rows.count()) === 0) return false;
  await rows.first().click();
  return true;
}

test("four assistants of one colour stand as a pile, not as a cube and a count", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  // Option B twice into the SAME area, with the President left in hand on
  // both turns: a President that acts in an area recovers its cubes at the
  // Recovery step, so the pile forms only if it never goes there. The bot's
  // turn in between cannot recover another seat's cubes, and its President
  // parking there at most prices the second placement at two yen.
  let area: string | null = null;
  let placements = 0;
  for (let i = 0; i < 40 && placements < 2; i++) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before two placements");
    if (await answer(page, /Option B/)) {
      // The picker's tiles are `disabled` while the request is in flight, so
      // wait for the settled screen before reading tile ids off it.
      await expect(page.locator("button.tile").first()).toBeEnabled();
      const tiles = page.locator("button.tile:not([disabled])");
      const target: Locator =
        area === null
          ? tiles.first()
          : tiles.filter({ has: page.locator(`.tile-id:text-is("${area}")`) }).first();
      area ??= (await target.locator(".tile-id").innerText()).trim();
      await target.click();
      await page.getByRole("button", { name: "Confirm" }).click();
      // The pile assertion must not be satisfiable by cubes that landed
      // elsewhere, so the applied move is pinned rather than assumed: the
      // account's own line names the area it fed ("Red: 2 assistants placed in
      // Customs (A09)" — `render.rs:920`, `web/src/wire/options_priced.json`).
      // A tile being clickable is no evidence, either: while picking, `disabled`
      // is `busy` alone (`Board.tsx:361`), so every non-Canal tile is offered.
      await expect(
        page.locator(".turn-account .messages div").filter({
          hasText: new RegExp(`placed in .*\\(${area}\\)`),
        }),
      ).toHaveCount(1);
      placements += 1;
      continue;
    }
    // Two wordings for one move, at two levels: the top-level Movement row is
    // the GROUPED family (`repl.rs:1353`), which opens a guided per-area count
    // sub-prompt; "Return President to hand" is the move's own wording
    // (`describe.rs:904`) and reaches the top level only when there is nothing
    // recoverable. Answering every area 0 is what leaves the pile standing.
    if (await answer(page, /Retreat President to hand/)) {
      // `g < 8` is generous for this script rather than a general bound: the
      // sub-prompt asks once per area holding the mover's assistants, and here
      // that is the one area both Option Bs fed.
      for (let g = 0; g < 8; g++) {
        const sub = await readScreen(page);
        if (sub.confirm) {
          await page.getByRole("button", { name: "Confirm" }).click();
          break;
        }
        if (sub.counts === 0) throw new Error("the guided retreat offered no counts");
        await page.locator(".options .controls button.ghost", { hasText: /^0$/ }).first().click();
      }
      continue;
    }
    if (await answer(page, /Return President to hand/)) continue;
    if (screen.only) {
      await page.getByRole("button", { name: "Apply the only option" }).click();
      continue;
    }
    if (await answer(page, /End additional-action phase|Skip the/)) continue;
    throw new Error(
      // `.decision` is on the warnings block too, and `App.tsx:464` renders
      // that one FIRST — empty unless the server warned, so `.first()` quoted
      // nothing. `:not(.notices)` is the answer bar (`Decision.tsx:107`).
      `no scripted answer for this screen:\n${await page
        .locator(".decision:not(.notices)")
        .first()
        .innerText()}`,
    );
  }
  expect(placements, "two placements were made").toBe(2);

  const tile = page.locator(".board .tile").filter({ has: page.locator(`.tile-id:text-is("${area}")`) });
  const pile = tile.locator(".piece-group.pile");
  await expect(pile).toHaveCount(1);
  await expect(pile.locator(".piece.cube")).toHaveCount(4);
  await expect(tile.locator(".piece-n")).toHaveCount(0);
  const cube = await pile.locator(".piece.cube").first().boundingBox();
  // RECORDED, never gated (spec §5): the height depends on the fit rule's tile
  // width on this machine at this viewport.
  console.log(`pile cube at 1710×869, 2P, on ${area}: ${(cube?.height ?? 0).toFixed(1)}px tall`);
  expect(cube?.height ?? 0).toBeGreaterThan(0);
  mkdirSync("../docs/temp/web-review/gates", { recursive: true });
  await page.screenshot({ path: "../docs/temp/web-review/gates/a-pile-1710x869.png" });
  expect(watch.failures).toEqual([]);
});
