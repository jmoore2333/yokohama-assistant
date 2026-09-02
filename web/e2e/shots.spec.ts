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
// **Then LOOK at the pictures, and re-read what is written beside them.** The
// card-art suite already learned the first half the expensive way: its tests
// prove the join is TOTAL, not TRUE, and two real errors survived a
// complete-and-green mapping because nobody opened the images. A regenerated
// screenshot that nobody views is the same mistake with a bigger surface —
// these are what a reader is told the product looks like. The second half is
// this file's own: the server draws the seed, so every run photographs a
// DIFFERENT game, and each shot carries an `alt` and a `caption` in
// `web/src/Help.tsx` (two of them also in `README.md`) that claim what is in
// frame. Those were written against a picture and stayed behind when the
// picture moved on. Regenerating without re-reading them ships a description of
// a screenshot that no longer exists.
//
// The few assertions below are there so a BLANK or half-drawn frame cannot be
// written to disk and shipped as documentation; they check that the thing being
// photographed is on screen, not that it is correct.

import { expect, test, type Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const SHOTS = "public/help";

/** The owner's screen, and the shape every picture in the help is taken at.
 *
 *  Not a round number on purpose: 1710x869 is the viewport the M5 table was
 *  designed and gated against (`e2e/gates.spec.ts` shoots the same cell), and a
 *  help panel photographed at a width nobody uses documents a layout nobody
 *  sees. `deviceScaleFactor` is deliberately left at 1 here where the gates
 *  spec sets 2: these four files ship in `dist/` and in `README.md`, so their
 *  bytes are a cost the repository pays forever, and the gates are the place
 *  that pays for pixel density. */
const VIEWPORT = { width: 1710, height: 869 };

/** Play real moves so the shots show a game in progress rather than a setup. */
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

/** Point the camera at the top: the WINDOW and the table's own scroller.
 *
 *  **Above 1201px the page does not scroll and `.table` does** (`styles.css`:
 *  `.app` is `height: 100vh` and the table is its own `overflow: auto` size
 *  container), so `window.scrollTo(0, 0)` — which is all this file used to do —
 *  resets a scroller that was never moved and leaves the table wherever the
 *  last click dragged it into view. Both are reset here, because below 1201px
 *  the page is the scroller and above it the table is.
 *
 *  **Throws rather than returning on a missing element**, the shape
 *  `gates.spec.ts` gives its own `scrollTable`: returning silently is the very
 *  failure the helper exists to prevent, since a rename would then produce a
 *  shot at whatever offset the last click left behind, under a green run. That
 *  helper's other anchors are not copied — every picture here is taken at the
 *  top, and an unused arm is an arm nothing checks. Copied rather than imported,
 *  because importing a `.spec.ts` would register that file's tests into this
 *  run. */
async function scrollToTop(page: Page): Promise<void> {
  const missing = await page.evaluate(() => {
    window.scrollTo(0, 0);
    const table = document.querySelector(".table") as HTMLElement | null;
    if (!table) return ".table";
    table.scrollTop = 0;
    return null;
  });
  if (missing) throw new Error(`scrollToTop: ${missing} is not on the page`);
  await raf(page);
}

/** The pointer off the board, and the hover popover with it.
 *
 *  `act()` ends on a click, so the pointer is left resting wherever the last
 *  one landed and `TilePopover` opens under it — the V2 gate photographed a
 *  popover sitting over the middle of the table before it learned this. A
 *  tooltip in a help screenshot is a picture of a feature the caption is not
 *  about. Asserted gone rather than assumed. */
async function park(page: Page): Promise<void> {
  await page.mouse.move(2, 2);
  await expect(page.getByRole("tooltip")).toHaveCount(0);
}

/** Switch a board on or off through the banner's `Boards` menu.
 *
 *  Scoped to the menu: "Goals" is also a card-index button in the `Cards`
 *  disclosure two elements along, so an unscoped role lookup is one label away
 *  from being ambiguous. Escape is `App.tsx`'s own handler and not the
 *  browser's — a `<details>` ignores the key natively — so the close is a claim
 *  worth checking, and a menu left open sits over the table in the shot. */
async function toggleBoards(page: Page, ...names: string[]): Promise<void> {
  const boards = page.locator(".boards-menu");
  await page.locator(".boards-menu > summary").click();
  for (const name of names) await boards.getByRole("button", { name }).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".boards-menu[open]")).toHaveCount(0);
}

test("the whole screen, and a placement picker", async ({ page }) => {
  await page.setViewportSize(VIEWPORT);
  await startGame(page, { players: 2 });

  // **The reading that stops the picker check below from being vacuous.**
  //
  // `placement.png` is selected by "some tile is a `<button>`", and that is
  // only a picker detector for as long as tiles are NOT buttons the rest of the
  // time. `Board.tsx` renders a tile as a `<div tabIndex={0}>` unless `App`
  // hands it an `onTileClick`, which it does only at `place_a`/`place_b` — so
  // the property holds at HEAD, and it is a property of two files that a third
  // task could quietly change (the popover, for one, opens on focus from the
  // inert `<div>`). Read here, before a move is played, where the phase machine
  // cannot be at a Placement prompt: if this ever reads non-zero the detector
  // has stopped discriminating and the run fails instead of photographing an
  // arbitrary frame under the placement caption.
  expect(
    await page.locator("button.tile").count(),
    "a tile is a button outside a placement picker, so the picker check below detects nothing",
  ).toBe(0);

  await playAFew(page, 9, 0x53_48_4f_54); // "SHOT"

  // Building sites on, so the table has all five boards under the pyramid and
  // the summary reads its own count back — the picture and the help's sentence
  // about the Boards menu are then about the same screen.
  await toggleBoards(page, "Building sites");
  await expect(page.locator(".boards-menu > summary")).toHaveText("Boards · 5 on");

  // **The FIRST VIEW: the table at its own scrollTop 0** (ruling R-T14b). Above
  // 1201px the page does not scroll and the table does, and `act()` leaves the
  // table wherever the last click scrolled it into view — the 1440-era comment
  // here reset only the window, which above this breakpoint is a scroller that
  // never moved. What is in frame is therefore what a player gets on load, and
  // the caption says that rather than naming a strip the fit rule has pushed
  // below the fold.
  await scrollToTop(page);
  await park(page);
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
  await scrollToTop(page);
  await park(page);
  await expect(page.locator("button.tile").first()).toBeVisible();
  // The deck's own words for the picker, so the shot cannot be of a board that
  // merely has buttons on it: Option A and Option B are the only two prompts
  // that put the question on the table, and one of them is on screen. Scoped to
  // `.deck` and not to `.decision`, which the client renders three times — the
  // warnings block is `.decision` too, and it is the FIRST one.
  await expect(page.locator(".deck")).toContainText(/Option [AB] —/);
  await expect(page.getByRole("button", { name: "Confirm" })).toBeVisible();
  await page.screenshot({ path: `${SHOTS}/placement.png` });
});

test("the explanation, and the replay track", async ({ page }) => {
  await page.setViewportSize(VIEWPORT);
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
  // The whole point of the shot is the TABLE redrawn at an earlier position
  // beside the drawer, so both have to be on screen before it is taken.
  await expect(page.locator(".table.replaying")).toBeVisible();
  await expect(page.locator(".surface[open]")).toBeVisible();
  // The table is its own scroller above 1201px and the walk has been clicking
  // in it; without this the outlined pyramid is photographed mid-scroll.
  await scrollToTop(page);
  await park(page);
  await page.screenshot({ path: `${SHOTS}/replay.png` });
});
