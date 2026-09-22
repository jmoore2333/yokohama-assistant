// M5.5 C: the opponent's turn plays out on the table when the reply lands.

// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import { expect, test } from "./sw";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

test("a bot turn plays out on the table, every position in order, and ends on the live prompt", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  await page.getByLabel("Cadence").selectOption("1000");
  // **The positions the walk ASKED FOR, off the run route** (M6.5b, R-6.5b.10).
  // Until this cycle the walk fetched `/at/{k}` once per position and this
  // counted the requests; it now makes ONE half-open `…/at?from=&to=` call and
  // the server replays the record once for the whole run. What the cell is
  // about has not moved — one position per event plus the one before the run,
  // ascending and contiguous — so the range is expanded here and every
  // assertion below reads exactly as it did.
  const positions: number[] = [];
  page.on("request", (r) => {
    const url = new URL(r.url());
    if (!url.pathname.endsWith("/at")) return;
    const from = Number(url.searchParams.get("from"));
    const to = Number(url.searchParams.get("to"));
    if (!Number.isFinite(from) || !Number.isFinite(to)) return;
    for (let k = from; k < to; k += 1) positions.push(k);
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

// **The walk must not scroll the PAGE (ruling R-F11).** Below 1201px the
// columns stack, the account sits under the table, and `.turn-account
// .messages` is capped at 9rem with `overflow: auto` — so the account is a
// scroller AND the window is one of its ancestors. `scrollIntoView` scrolls
// every scrollable ancestor by specification, and it did: measured live at
// 1000x523, `window.scrollY` went 0 → 1826 → 1851 → 1877 over three steps,
// dragging the board the walk was highlighting off screen and leaving the lit
// row at the viewport's bottom edge under the sticky walk strip.
//
// Only a browser can see this. jsdom computes no layout, so the component cells
// have to be handed a geometry, and a component cell cannot have an ancestor
// scroller at all.
//
// **The steps are driven by hand, and the clicks go through `evaluate`.**
// Playwright scrolls an element into view before clicking it, which would move
// the page for a reason that has nothing to do with the walk and make the
// reading meaningless. A DOM `click()` is the same event to React and moves
// nothing.
test("in the stacked layout the walk scrolls the account and never the page", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1000, height: 600 });
  await startGame(page, { players: 2 });
  await page.getByLabel("Cadence").selectOption("3000");

  /** The four readings this cell is about, in one round trip. */
  const read = () =>
    page.evaluate(() => {
      const box = document.querySelector<HTMLElement>("#turn-account-body");
      const row = document.querySelector<HTMLElement>("ol.account li.at");
      if (!box) return null;
      const b = box.getBoundingClientRect();
      const r = row?.getBoundingClientRect();
      return {
        pageY: Math.round(window.scrollY),
        scrollTop: Math.round(box.scrollTop),
        overflows: box.scrollHeight > box.clientHeight,
        maxHeight: getComputedStyle(box).maxHeight,
        rowInside: r ? r.top >= b.top - 1 && r.bottom <= b.bottom + 1 : null,
        rowTop: r ? Math.round(r.top) : null,
        boxTop: Math.round(b.top),
        boxBottom: Math.round(b.bottom),
      };
    });

  const pick = rng(0x5354_4143); // "STAC"
  let observed = false;
  for (let i = 0; i < 40 && !observed; i++) {
    // `readScreen` skips any walk still on screen and waits for the strip to
    // detach, so a turn this loop passed over is cleaned up here rather than
    // left running under the next action.
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
    // Any enabled deck control, not the three this file's other cell names: a
    // different RNG stream reaches decks whose only control is a retreat count,
    // and waiting for an option row there times out on a screen that is fine.
    await page
      .locator(".decision.walking, .decision button:not([disabled])")
      .first()
      .waitFor({ timeout: 15_000 });
    const strip = page.locator(".decision.walking");
    if ((await strip.count()) === 0) continue;

    // Pause where it is, then ask whether this turn can carry the reading at
    // all: an account that actually overflows its cap, and enough steps left to
    // walk the lit row OUT of the box. A turn that fits has nothing to scroll
    // and could not tell the two mechanisms apart — and neither could the first
    // few steps of one that does, since about five rows fit the 144px cap.
    // Measured on the first attempt at exactly three steps: every reading came
    // back `scrollTop 0` with the row inside the box, which is correct and
    // proves nothing.
    await page.evaluate(() => {
      document.querySelector<HTMLElement>('.turn-account [aria-label="Pause"]')?.click();
    });
    const where = /step (\d+) of (\d+)/.exec(await strip.innerText());
    if (!where) continue;
    const [k, n] = [Number(where[1]), Number(where[2])];
    const before = await read();
    const forwards = Math.min(n - k, 6);
    if (forwards < 5 || !before?.overflows) continue;
    observed = true;

    // The page at the top, so a page move of any size is unmistakable. In the
    // stack that puts the account below the fold, which changes nothing here:
    // both rects are read in the same coordinate space, so "the row is inside
    // the box" means the same thing wherever the box is.
    await page.evaluate(() => window.scrollTo(0, 0));
    const readings = [await read()];
    for (let s = 1; s <= forwards; s++) {
      await page.evaluate(() => {
        document.querySelector<HTMLElement>('.turn-account [aria-label="Step forward"]')?.click();
      });
      await expect(strip).toContainText(new RegExp(`step ${k + s} of `));
      readings.push(await read());
    }
    // eslint-disable-next-line no-console
    console.log(
      `the account's scroll at 1000x600, steps ${k}-${k + forwards} of ${n} ` +
        `(the box's max-height ${readings[0]!.maxHeight}): ` +
        readings
          .map((r) => `pageY ${r!.pageY} · scrollTop ${r!.scrollTop} · row top ${r!.rowTop} in [${r!.boxTop}, ${r!.boxBottom}]`)
          .join(" | "),
    );

    // **The page never moves.** This is the whole ruling.
    for (const r of readings) expect(r!.pageY, "the walk scrolled the page").toBe(0);
    // **The account's own scroller does**, or the line above would hold for an
    // account that scrolls nothing at all.
    expect(
      new Set(readings.map((r) => r!.scrollTop)).size,
      "the account's scroller never moved, so a page that stayed put proves nothing",
    ).toBeGreaterThan(1);
    // And the row the walk is lighting ends up inside the box that lit it.
    for (const r of readings) expect(r!.rowInside, "the lit row is outside the account's box").toBe(true);
  }
  expect(observed, "no bot turn long enough to overflow the account was seen in 40 actions").toBe(true);
  expect(watch.failures).toEqual([]);
});
