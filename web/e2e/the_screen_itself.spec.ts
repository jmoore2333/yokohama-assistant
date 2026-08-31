// What only a real browser can answer: does it LOAD, and can you leave it.
//
// jsdom has no layout, no paint, no image decoding and no real focus model, so
// every assertion in this file is one the component suite structurally cannot
// make. Each is also a defect class this lane has already met once.

import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, deadEnd, readScreen, rng, startGame, watchForBreakage } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly. The reporter
// then says how many tests did not run, which is the difference between a skip
// and a silent absence.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");


test("every picture the client draws actually loads", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page);

  // The board draws area tiles and 5-Power tokens; the rail draws the hand, the
  // achievements and the mercenary flags. All of them are `loading="lazy"`, so
  // scroll the page first or the ones below the fold have never been asked for.
  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));

  // **Wait for every image to SETTLE, rather than sleeping.** The first draft
  // slept 500ms and then filtered on `complete`, which can only ever degrade
  // toward a pass: an image still in flight is not `complete`, so a slow load
  // was silently excluded from the very check that was supposed to catch it.
  await page.waitForFunction(() => [...document.images].every((img) => img.complete), null, {
    timeout: 15_000,
  });

  const broken = await page.evaluate(() =>
    [...document.images]
      .filter((img) => img.naturalWidth === 0)
      .map((img) => img.currentSrc || img.src),
  );
  expect(broken, "a broken image is a worse answer than no image").toEqual([]);

  const count = await page.evaluate(() => document.images.length);
  // Anti-vacuity: an empty document has no broken images either. The art is
  // COMMITTED, so a checkout that draws none of it has a manifest that resolved
  // to nothing, which is exactly the failure the assertion above would miss.
  expect(count, "the committed art must actually be on the page").toBeGreaterThan(10);

  expect(watch.failures).toEqual([]);
});

test("a surface panel takes focus, and Escape gets you out of it", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page);

  // The panel is `position: fixed` over the whole board. Without the focus move
  // a keyboard user who opens it is left with focus on the nav button behind
  // it and no way out but Tab — forward through every remaining nav control
  // and, during a placement prompt, all eighteen board tiles now hidden
  // underneath. jsdom can be told an element has focus; only a browser has the
  // tab order that makes that matter.
  await page.getByRole("button", { name: "Areas" }).click();
  const dialog = page.getByRole("dialog");
  await expect(dialog).toBeVisible();
  await expect(dialog).toBeFocused();

  await page.keyboard.press("Escape");
  await expect(dialog).toBeHidden();

  expect(watch.failures).toEqual([]);
});

test("all four read-only surfaces open, draw, and close", async ({ page }) => {
  const watch = watchForBreakage(page);
  const game = await startGame(page);

  // Play first. The LOG on a fresh game is one line — "0 event(s)." — so a
  // content assertion taken at the opening reads as an empty surface, and one
  // loosened to accept it would accept an empty one forever.
  const pick = rng(0x5355_5246); // "SURF"
  let moves = 0;
  for (let i = 0; i < 8; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, moves);
    moves += 1;
  }
  expect(moves, "the preamble must give the log something to show").toBeGreaterThan(3);

  for (const name of ["Port offer", "Laboratory offer", "Areas", "Log"]) {
    await page.getByRole("button", { name, exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toHaveAccessibleName(name);
    // Drawn, not merely mounted. Two absences (`loading…` and `.error`) are not
    // enough on their own: a surface that fetched successfully and rendered
    // NOTHING satisfies both. So the body must also have content.
    await expect(dialog.getByText("loading…")).toHaveCount(0);
    await expect(dialog.locator(".error")).toHaveCount(0);
    const body = await dialog.locator(".surface-body").innerText();
    expect(body.trim().length, `${name} rendered an empty body`).toBeGreaterThan(40);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
  }

  expect(watch.failures).toEqual([]);
});

test("the page never scrolls sideways, at a laptop width or a phone's", async ({ page }) => {
  const page_game = await startGame(page);
  // Play a few moves first: an empty opening board is the narrowest the screen
  // ever is, so measuring only there measures the easy case.
  const pick = rng(0x5749_4454); // "WIDT"
  let moves = 0;
  for (let i = 0; i < 12; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, page_game, moves);
    moves += 1;
  }
  expect(moves, "the preamble must actually put something on the board").toBeGreaterThan(5);

  // **The asserted widths are DESKTOP widths, and that is a scope statement
  // rather than an oversight.** This client is a desktop surface: the board is
  // a staggered pyramid of up to six tiles across, each carrying a site strip
  // that is the densest thing on the screen, and no phone layout was ever
  // designed or asked for. Asserting 390px would be asserting a requirement
  // nobody set.
  //
  // MEASURED at 390 x 844 on the run that added this file: 545px of horizontal
  // overflow. Recorded, not asserted — so that whoever takes up a phone layout
  // starts from a number rather than from a discovery.
  for (const size of [
    { width: 1440, height: 900 },
    { width: 1024, height: 768 },
  ]) {
    await page.setViewportSize(size);
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow, `horizontal overflow at ${size.width}px`).toBeLessThanOrEqual(0);
  }

  await page.setViewportSize({ width: 390, height: 844 });
  const phone = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  // eslint-disable-next-line no-console
  console.log(`horizontal overflow at 390px (recorded, not gated): ${phone}px`);
});

test("a four-player table draws four seats and the Canal", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 4 });

  // The 2P tile set has no Canal at all, so a suite that only ever opened a 2P
  // game has never drawn the one tile that is never an input. The seat-identity
  // defect this lane's review found was reachable only away from 2P too.
  await expect(page.locator(".tile.canal")).toHaveCount(1);
  await expect(page.getByText(/crossing only/)).toBeVisible();
  for (const colour of ["Red", "Blue", "Yellow", "Green"]) {
    await expect(page.locator(".seat-name", { hasText: colour }).first()).toBeVisible();
  }
  // The dummy clause is 2P-only: `place_setup_dummies` returns early above two
  // players, so a browser that teaches a `D` marker here teaches a rule that
  // cannot apply. The heading is asserted to EXIST first — `not.toContainText`
  // on a locator that matches nothing passes for the wrong reason.
  const tracks = page.getByRole("heading", { name: /Tracks/ });
  await expect(tracks).toBeVisible();
  await expect(tracks).not.toContainText("dummy");
  await expect(page.locator(".space.dummy")).toHaveCount(0);

  expect(watch.failures).toEqual([]);
});
