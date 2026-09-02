// M5's browser cells: the things about the table that jsdom cannot see.

import { expect, test, type Page } from "@playwright/test";
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
 *  works out at ~9.7rem, UNDER the 10rem legibility floor: the floor wins, the
 *  pyramid is taller than the table, and the table has to scroll it. That is
 *  the rule behaving correctly at a size it cannot satisfy, and it is the only
 *  one of the four that exercises the other half of the cell. */
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
        const rows = [...document.querySelectorAll(".board-row")];
        return {
          tableW: table.clientWidth,
          tableH: table.clientHeight,
          tileW: document.querySelector(".tile")!.getBoundingClientRect().width,
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
          `rows in view · the legend and anything under the pyramid scroll by ` +
          `${read.scrollH - read.tableH}px — recorded`;
      } else {
        expect(read.scrollH, `${players}P at ${w}x${h}: the rule cannot fit, so the table must scroll`).toBeGreaterThan(read.tableH);
        branch =
          `the rule cannot fit its rows, so the table scrolls the pyramid ` +
          `(${read.scrollH} > ${read.tableH}) — gated`;
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
