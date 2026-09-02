// The help panel's screenshots, taken from the REAL application.
//
// **A generator, not a test.** Nothing here asserts a product property; it
// drives the shipped client in real Chromium against the real release binary
// and writes `web/public/help/*.png`, which Vite copies into `dist/` and the
// server serves at `/help/…`. It lives in `e2e/` because that is where the
// browser and the server already are, and because a screenshot generator that
// needed its own harness would be a second driver of the same app.
//
// It is deliberately NOT part of `npm run e2e`'s meaning. Run it on purpose:
//
// ```text
// cd web && npm run build && YOKOHAMA_WEB_SHOTS=1 npx playwright test shots.spec.ts
// ```
//
// The env var is not optional: `playwright.config.ts` drops this file from the
// glob without it, so the command that omits it discovers zero tests and exits
// with an error. (It was written that way here first, which is the one place a
// reader would copy it from.)
//
// **Then LOOK at the pictures.** The card-art suite already learned this the
// expensive way: its tests prove the join is TOTAL, not TRUE, and two real
// errors survived a complete-and-green mapping because nobody opened the
// images. A regenerated screenshot that nobody views is the same mistake with a
// bigger surface — these are what a reader is told the product looks like.
//
// The few assertions below are there so a BLANK or half-drawn frame cannot be
// written to disk and shipped as documentation; they check that the thing being
// photographed is on screen, not that it is correct.

import { expect, test, type Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const SHOTS = "public/help";

/** Play real moves so the shots show a game in progress rather than a setup. */
async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

test("the whole screen, and a placement picker", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await startGame(page, { players: 2 });
  await playAFew(page, 9, 0x53_48_4f_54); // "SHOT"

  // **Scroll to the top before shooting.** The page is taller than the
  // viewport and `act()` leaves it wherever the last click was, so the first
  // draft photographed a mid-page scroll with the banner off-frame — while the
  // help text beside it talks about "the buttons along the top". The deck is
  // pinned, so the top of the page still shows all three regions the caption
  // names.
  await page.evaluate(() => window.scrollTo(0, 0));
  // Anti-vacuity: a screenshot of an empty board would still be a valid PNG.
  await expect(page.locator(".board .tile").first()).toBeVisible();
  await expect(page.locator(".deck")).toBeVisible();
  await expect(page.getByRole("button", { name: "Log", exact: true })).toBeVisible();
  await page.screenshot({ path: `${SHOTS}/screen.png` });

  // Walk on until a placement picker is open, so the "the board is the input"
  // paragraph has the picture it describes. Bounded, and it says so if it never
  // finds one rather than writing a shot of something else.
  let found = false;
  const pick = rng(0x504c_4143); // "PLAC"
  for (let i = 0; i < 40 && !found; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await page.locator("button.tile").count()) > 0) {
      found = true;
      break;
    }
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(found, "never reached a placement picker, so placement.png would be a lie").toBe(true);
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(page.locator("button.tile").first()).toBeVisible();
  await page.screenshot({ path: `${SHOTS}/placement.png` });
});

test("the explanation, and the replay track", async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await startGame(page, { players: 2 });
  await playAFew(page, 14, 0x4558_504c); // "EXPL"

  await page.getByRole("button", { name: "Suggest", exact: true }).click();
  // The chart, not just the panel: the bars ARE the paragraph's subject.
  await expect(page.locator(".surface .bar").first()).toBeVisible({ timeout: 30_000 });
  await page.screenshot({ path: `${SHOTS}/explain.png` });
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await expect(track).toBeVisible();
  await track.focus();
  for (let i = 0; i < 6; i++) await page.keyboard.press("ArrowLeft");
  // The whole point of the shot is the board redrawn at an earlier position
  // BESIDE the drawer, so both have to be on screen before it is taken.
  await expect(page.locator(".stage.replaying")).toBeVisible();
  await expect(page.locator(".surface[open]")).toBeVisible();
  await page.screenshot({ path: `${SHOTS}/replay.png` });
});
