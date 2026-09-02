// The visual gates (M5 spec §4.1): screenshots for a human, not assertions.
//
// **A generator, not a test**, in the shape of `shots.spec.ts`: it drives the
// shipped client and writes pictures for the owner to look at. Opt-in via
// `YOKOHAMA_WEB_GATES=1` (the config drops this file otherwise), and the output
// goes under the gitignored `docs/temp/web-review/gates/` because a picture of
// the art is the art. The few `expect`s stop a blank frame from being sent.
//
//     cd web && npm run build && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts

import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");
// The owner's display is 2×; a 1× shot understates the art's legibility.
test.use({ deviceScaleFactor: 2 });

const OUT = "../docs/temp/web-review/gates";
const VIEWPORTS = [
  [1440, 900],
  [1710, 869],
  [2560, 1440],
] as const;

async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

const raf = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

for (const players of [2, 4] as const) {
  test(`V1 — the tiles at ${players}P, three viewports, and a popover`, async ({ page }) => {
    mkdirSync(OUT, { recursive: true });
    await page.setViewportSize({ width: 1710, height: 869 });
    await startGame(page, { players });
    await playAFew(page, 14, 0x4741_5445 + players); // "GATE"
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      // `act()` leaves the page wherever the last click was, and the page is
      // taller than every viewport here. The first run photographed 2P at a
      // mid-page scroll with the board's whole TOP ROW off-frame while 4P
      // happened to land at the top — the same defect `shots.spec.ts` records
      // in its own comment, and it made the two halves of the gate
      // incomparable. The camera is pointed at the top on purpose.
      await page.evaluate(() => window.scrollTo(0, 0));
      await raf(page);
      await expect(page.locator(".board .tile").first()).toBeVisible();
      await page.screenshot({ path: `${OUT}/v1-${players}p-${w}x${h}.png` });
    }
    await page.setViewportSize({ width: 1710, height: 869 });
    await page.evaluate(() => window.scrollTo(0, 0));
    await page.locator(".tile").first().hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await page.screenshot({ path: `${OUT}/v1-${players}p-popover.png` });
  });
}
