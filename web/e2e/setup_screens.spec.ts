// The setup screens in a real browser (M6.5c2, spec §2.2 and §3).
//
// What only a browser can say about the rebuilt screens, which jsdom — no
// stylesheet, no layout — structurally cannot: each is laid out at desktop
// width and at 390px, nothing on it scrolls sideways, its primary controls are
// thumb-sized on a phone, and no picture on it is an `<img>` (card art is never
// anything else in this client, so zero is the claim `ASSETS.md` asks of a
// setup screen). Every cell leaves its screenshots under the gitignored
// `test-results/` for the headed look spec §3 requires; the task's report names
// them beside the approved mocks.
//
// One spec for every screen this cycle rebuilds, so the width rule is written
// once: the door first, then the name step and the lobby as their tasks land.

import type { Page } from "@playwright/test";
// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import { expect, test } from "./sw";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { blame, readOverflow, watchForBreakage } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const DESKTOP = { width: 1440, height: 900 };
const PHONE = { width: 390, height: 844 };

/** Nothing on the page is wider than its window — read once the layout is DONE
 *  (the width gate's own note, `harness.ts`). */
async function noSidewaysScroll(page: Page, what: string): Promise<void> {
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  const read = await readOverflow(page);
  expect(
    read.docScrollWidth - read.docClientWidth,
    `${what} scrolls sideways: ${blame(read)}`,
  ).toBeLessThanOrEqual(0);
}

/** A picture for the headed look, under this run's own output directory. */
async function look(page: Page, name: string): Promise<void> {
  await page.screenshot({ path: test.info().outputPath(`${name}.png`), fullPage: true });
}

/** How many `<img>` the page draws. */
const images = (page: Page): Promise<number> => page.evaluate(() => document.images.length);

/** A control's rendered height — "thumb-sized" is 44px, the platforms' own floor. */
async function height(page: Page, role: "button" | "link", name: string): Promise<number> {
  return (await page.getByRole(role, { name, exact: true }).boundingBox())!.height;
}

test("the door: three cards side by side at 1440 and one column at 390, no card art, nothing asked of the API", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  const asked: string[] = [];
  page.on("request", (r) => {
    const url = new URL(r.url());
    if (url.origin === new URL(BASE_URL).origin) asked.push(url.pathname);
  });

  for (const size of [DESKTOP, PHONE]) {
    await page.setViewportSize(size);
    await page.goto("/");
    const title = page.locator(".door-hero").getByRole("heading", { level: 1, name: "Yokohama" });
    await expect(title).toBeVisible();
    const cards = page.locator(".door-cards > .door-card");
    await expect(cards).toHaveCount(3);
    await expect(page.getByRole("link", { name: "Go to the lobby" })).toHaveAttribute("href", "/enter");
    await noSidewaysScroll(page, `the door at ${size.width}px`);
    expect(await images(page), `the door draws no <img> at ${size.width}px`).toBe(0);

    const boxes = await cards.evaluateAll((els) =>
      els.map((e) => {
        const r = e.getBoundingClientRect();
        return { left: Math.round(r.left), top: Math.round(r.top) };
      }),
    );
    const board = (await page.locator(".door-hero .board-mark").boundingBox())!;
    const heading = (await title.boundingBox())!;
    if (size === DESKTOP) {
      expect(new Set(boxes.map((b) => b.top)).size, `one row of cards: ${JSON.stringify(boxes)}`).toBe(1);
      expect(boxes.map((b) => b.left)).toEqual([...boxes.map((b) => b.left)].sort((a, b) => a - b));
      expect(board.x, "the board sits right of the title").toBeGreaterThan(heading.x + heading.width);
    } else {
      expect(new Set(boxes.map((b) => b.left)).size, `one column of cards: ${JSON.stringify(boxes)}`).toBe(1);
      expect(boxes.map((b) => b.top)).toEqual([...boxes.map((b) => b.top)].sort((a, b) => a - b));
      expect(board.y + board.height, "the board sits above the title").toBeLessThanOrEqual(heading.y + 1);
      expect(await height(page, "button", "New game"), "New game, thumb-sized").toBeGreaterThanOrEqual(44);
      expect(await height(page, "link", "Go to the lobby"), "Go to the lobby, thumb-sized").toBeGreaterThanOrEqual(44);
    }
    await look(page, `door-${size.width}`);
  }

  // The help opens in place, and brings none of its screenshots.
  await page.setViewportSize(DESKTOP);
  await page.getByRole("button", { name: "How to play" }).click();
  await expect(page.getByRole("heading", { name: "The three parts of the screen" })).toBeVisible();
  expect(await images(page), "the help on the door draws none of its screenshots").toBe(0);
  await look(page, "door-help-1440");

  // **R-6.5b2.7 in a real browser, for the rebuilt door**: the art manifest is
  // a static file and the words are the local host's; nothing is `/api/`.
  expect(asked.filter((p) => p.startsWith("/api/")), "the door asks the API for nothing").toEqual([]);

  // **The device card with a game on it**, for the headed look: a fresh
  // profile draws no rows, and the rows are half of what mock 01's device card
  // shows. The game is a local one, made the way a player makes it; the claim
  // above is the DOOR's, so it is read before this and not after.
  await page.getByRole("button", { name: "New game", exact: true }).click();
  await page.getByRole("button", { name: "Deal" }).click();
  await page.waitForURL(/#\/t\/local-[0-9a-f]+$/, { timeout: 60_000 });
  for (const size of [DESKTOP, PHONE]) {
    await page.setViewportSize(size);
    await page.goto("/#/");
    const row = page.locator(".on-this-device .device-row");
    await expect(row).toHaveCount(1);
    // Which bot, then what the game waits on (`deviceWord`, mock 10).
    await expect(row).toContainText(/vs \S+ · /);
    await noSidewaysScroll(page, `the door with a game on it at ${size.width}px`);
    await look(page, `door-rows-${size.width}`);
  }
  expect(watch.failures).toEqual([]);
});
