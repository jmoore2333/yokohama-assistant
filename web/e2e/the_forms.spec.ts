// **The two setup forms, drawn by a real browser** (M6.5c2, spec §2.2 "The
// forms", "The seed", "The phone"; R-6.5c2.8; mock 07, mock 11).
//
// jsdom never loads `styles.css`, so what is asserted here is what only a
// stylesheet decides: nothing scrolls sideways at a phone's 390 px or at 1280,
// the control that sends the form is a thumb's size on the phone, and the
// seed's refusal is drawn INSIDE the open fold (Review Focus 2's other half —
// `seedRule.test.ts` holds the rule itself).
//
// Each cell writes a picture of the form to its own output directory: the
// headed look the spec asks for (§3) is a person comparing those with mock 07
// and mock 11, and the task's report says what differs.

import type { Page } from "@playwright/test";
import { expect, test } from "./sw";
import { haveBrowser } from "../playwright.config";
import { blame, newGameForm, readOverflow, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium \u2014 YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** Lay the page out at `width`, and read nothing until the layout is DONE — a
 *  resize and a read in one tick can be answered from the pre-resize box. */
async function atWidth(page: Page, width: number): Promise<void> {
  await page.setViewportSize({ width, height: 844 });
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
}

/** The page may not scroll sideways — the lane's recorded property. */
async function fits(page: Page, what: string, width: number): Promise<void> {
  const read = await readOverflow(page);
  expect(
    read.docScrollWidth - read.docClientWidth,
    `${what} scrolls sideways at ${width}px: ${blame(read)}`,
  ).toBeLessThanOrEqual(0);
}

for (const width of [1280, 390] as const) {
  test(`the new-table form fits ${width} px at its widest, and refuses a bad seed in its fold`, async ({
    page,
  }, info) => {
    const watch = watchForBreakage(page);
    await newGameForm(page);
    await atWidth(page, width);
    // **The form at its WIDEST** — four seats, one of them a specific person
    // (whose field drops under the row), four colours and Random to start, the
    // password open and the fold open — because a width claim about the
    // narrowest form proves nothing about this one.
    await page.getByRole("group", { name: "Players" }).getByRole("radio", { name: "4", exact: true }).check();
    await page.getByRole("combobox", { name: "Green seat", exact: true }).selectOption("person");
    await page.getByLabel(/^Green player's (name|address)$/).fill("someone else");
    await page.getByLabel("Ask for a password to join or spectate", { exact: true }).check();
    await page.getByText("Advanced", { exact: true }).click();
    const seed = page.getByLabel("Seed", { exact: true });
    const submit = page.getByRole("button", { name: "Open table" });
    // Anti-vacuity: the controls this claim is about are really drawn.
    await expect(seed).toBeVisible();
    await expect(submit).toBeEnabled();

    // One past the top of a u64 (Review Focus 2): refused IN PLACE, the button held.
    await seed.fill("18446744073709551616");
    const fold = page.locator("details.fold");
    const refusal = fold.getByText(/^Too large: a seed is a whole number from 0 to 18446744073709551615\./);
    await expect(refusal).toBeVisible();
    await expect(seed).toHaveAttribute("aria-invalid", "true");
    await expect(submit).toBeDisabled();
    // **Inside the fold's own box**, which is what "in place" means on a
    // screen: a sentence the stylesheet put somewhere else sits beside nothing.
    const foldBox = (await fold.boundingBox())!;
    const sentenceBox = (await refusal.boundingBox())!;
    expect(sentenceBox.y).toBeGreaterThanOrEqual(foldBox.y);
    expect(sentenceBox.y + sentenceBox.height).toBeLessThanOrEqual(foldBox.y + foldBox.height + 1);
    await fits(page, "the new-table form", width);
    if (width === 390) {
      const box = (await submit.boundingBox())!;
      expect(box.height, "the submit is a thumb's size on a phone").toBeGreaterThanOrEqual(44);
      expect(box.x + box.width, "the submit's right edge is on the screen").toBeLessThanOrEqual(390);
    }
    await page.screenshot({ path: info.outputPath(`new-table-${width}.png`), fullPage: true });

    // And a seed the SERVER takes — a `+`, leading zeros and a space — lets the
    // form go: the fold agrees with `WireSeed`, not with a stricter guess.
    await seed.fill("+007 ");
    await expect(refusal).toHaveCount(0);
    await expect(submit).toBeEnabled();
    expect(watch.failures).toEqual([]);
  });

  test(`the form for a game on this device fits ${width} px`, async ({ page }, info) => {
    const watch = watchForBreakage(page);
    // **No sign-in**: the local form needs no door (R-6.5b.8).
    await page.goto("/#/new/local");
    await atWidth(page, width);
    await expect(page.getByRole("heading", { name: "New game against the bot", exact: true })).toBeVisible();
    const deal = page.getByRole("button", { name: "Deal" });
    await expect(deal).toBeEnabled();
    await page.getByRole("group", { name: "First player" }).getByRole("radio", { name: "Random", exact: true }).check();
    await fits(page, "the local form", width);
    if (width === 390) {
      const box = (await deal.boundingBox())!;
      expect(box.height, "Deal is a thumb's size on a phone").toBeGreaterThanOrEqual(44);
      // **A band with no corner keeps its crumb on the title's line** (Task
      // 4b's review, Important 1): row 60's phone grid — the pill beside the
      // title, the path under both — is for a band whose corner holds
      // something, and this screen passes none.
      const band = await page.evaluate(() => {
        const r = (e: Element) => {
          const b = e.getBoundingClientRect();
          return { top: Math.round(b.top), bottom: Math.round(b.bottom) };
        };
        const header = document.querySelector("header.lobby-band")!;
        return {
          corner: header.querySelector(".band-corner")!.childElementCount,
          title: r(header.querySelector("h1")!),
          crumb: r(header.querySelector(".crumb")!),
          crumbText: header.querySelector(".crumb")!.textContent,
        };
      });
      expect(band.corner, "the premise: this band's corner is empty").toBe(0);
      expect(band.crumbText).toBe("On this device");
      expect(band.crumb.top, "the crumb is on the title's line").toBeLessThan(band.title.bottom);
      expect(band.crumb.bottom, "and not under it").toBeGreaterThan(band.title.top);
    }
    await page.screenshot({ path: info.outputPath(`new-local-${width}.png`), fullPage: true });
    expect(watch.failures).toEqual([]);
  });
}
