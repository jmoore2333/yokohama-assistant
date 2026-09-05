// M5.5 C: the opponent's turn plays out on the table when the reply lands.

import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

test("a bot turn plays out on the table, every position in order, and ends on the live prompt", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  await page.getByLabel("Cadence").selectOption("1000");
  const positions: number[] = [];
  page.on("request", (r) => {
    const m = /\/at\/(\d+)$/.exec(new URL(r.url()).pathname);
    if (m) positions.push(Number(m[1]));
  });
  const pick = rng(0x504c_4159); // "PLAY"
  let observed = false;
  for (let i = 0; i < 40 && !observed; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    positions.length = 0;
    if ((await act(page, screen, pick)) === null) throw new Error("dead end before a bot turn was seen");
    await page
      .locator(".decision.walking, .decision button.option:not([disabled]), .decision button.primary:not([disabled])")
      .first()
      .waitFor({ timeout: 15_000 });
    if ((await page.locator(".decision.walking").count()) === 0) continue;
    observed = true;
    const strip = page.locator(".decision.walking");
    await expect(strip).toContainText(/is playing · step \d+ of \d+/);

    // **The strip is ONE LINE (spec §3.2.2, ruling R-F3).** `.decision.walking`
    // set `display: flex` and inherited `flex-direction: column` from
    // `.decision`, so the deck stacked "Blue" / "is playing · step 3 of 8" /
    // "skip" — 123px of deck where the spec's own words are "Blue is playing ·
    // skip". jsdom has no layout engine, so this is the only place that can
    // read it.
    //
    // Two readings, and the second is the one a font change cannot fool: the
    // declared direction, and whether every child actually SHARES a row —
    // each child's vertical midpoint inside every other child's box. A wrap at
    // some narrower width is allowed by the ruling and would fail neither, since
    // this cell states its viewport.
    const line = await strip.evaluate((el) => {
      const kids = [...el.children].map((c) => c.getBoundingClientRect());
      return {
        direction: getComputedStyle(el).flexDirection,
        height: el.getBoundingClientRect().height,
        children: kids.length,
        oneRow: kids.every((a) =>
          kids.every((b) => {
            const mid = a.top + a.height / 2;
            return mid >= b.top && mid <= b.bottom;
          }),
        ),
      };
    });
    // eslint-disable-next-line no-console
    console.log(
      `the walk strip at 1710x869: flex-direction ${line.direction}, ${line.children} children on ` +
        `${line.oneRow ? "one row" : "more than one row"}, ${line.height.toFixed(1)}px tall`,
    );
    expect(line.children, "the strip must have children for a row check to mean anything").toBeGreaterThan(1);
    expect(line.direction).toBe("row");
    expect(line.oneRow, "the walk strip stacked its parts instead of laying them on one line").toBe(true);
    const steps = Number(/of (\d+)/.exec(await strip.innerText())![1]);
    // eslint-disable-next-line no-console
    console.log(`the turn playback: the observed bot turn walked ${steps} steps`);
    // One position per event PLUS the one before the run, requested in
    // ascending order and contiguous.
    const sorted = [...positions].sort((a, b) => a - b);
    expect(positions).toEqual(sorted);
    expect(sorted).toHaveLength(steps + 1);
    for (let j = 1; j < sorted.length; j++) expect(sorted[j]).toBe(sorted[j - 1]! + 1);
    await expect(page.locator("ol.account li.at")).toHaveCount(1);
    if (steps >= 2) await expect(page.locator(".table.replaying")).toBeVisible();
    // Let it run out at 1 s a step, with slack; the walk ends by itself.
    await expect(strip).toHaveCount(0, { timeout: steps * 1000 + 8000 });
    await expect(page.locator(".table.replaying")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "replay" })).toBeVisible();
    const live = await readScreen(page);
    expect(live.options + live.tiles + Number(live.only) + Number(live.confirm)).toBeGreaterThan(0);
  }
  expect(observed, "no bot turn was observed in 40 actions").toBe(true);
  expect(watch.failures).toEqual([]);
});
