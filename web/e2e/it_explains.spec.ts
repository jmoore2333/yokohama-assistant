// The explanation surface, in a real browser.
//
// **Why this is a spec of its own rather than a branch of the whole-game walk.**
// The walk is deliberately BOT-FREE: `Games::create` draws its bot seed from OS
// entropy, so a game with a bot in it does not reproduce, and the walk's whole
// value is that it plays the same game every run. A `why` panel needs a bot
// move to account for. Rather than trade the walk's reproducibility away, this
// spec creates its own game with a bot on the start seat — `create` pumps it, so
// a bot turn exists before the first click — and asserts only what is
// structural.
//
// What it proves that jsdom cannot: that the chart has real geometry (bars with
// nonzero width, on the correct side of a zero line that is actually painted),
// that the panel takes focus and Escape leaves it, and that a page carrying two
// hundred candidate rows does not overflow sideways.

import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { watchForBreakage } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly. The reporter
// then says how many tests did not run, which is the difference between a skip
// and a silent absence.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** Seat 0 is the bot AND the start seat, so `create`'s pump plays a whole bot
 *  turn before anything is on screen. The setup seed is fixed; the BOT seed is
 *  the server's own entropy and is deliberately not pinned — nothing here
 *  asserts a number that depends on it. */
async function aGameWithABot(request: import("@playwright/test").APIRequestContext, name: string) {
  const res = await request.post("/api/games", {
    data: {
      players: 2,
      colors: ["Red", "Blue"],
      start_seat: 0,
      seed: "20260831",
      keeps: [1, 1],
      bot_seats: [0],
      name,
    },
  });
  expect(res.status(), await res.text()).toBe(201);
}

/**
 * Answer the one forced move `create`'s pump leaves in front of the human.
 *
 * The opening additional-action decision has exactly one legal move at turn 1 —
 * nobody owns a technology yet — so a panel opened on it ranks a field of ONE.
 * A test that wants a wide field has to move past it first. That the decision is
 * forced is CHECKED here rather than assumed, so if it ever stops being forced
 * this says so instead of silently answering something else.
 */
async function pastTheForcedOpening(
  request: import("@playwright/test").APIRequestContext,
  game: string,
) {
  const state = await (await request.get(`/api/games/${game}`)).json();
  expect(state.prompt?.kind, "the pump leaves an option list").toBe("options");
  expect(state.prompt.rows.length, "…with exactly one move in it").toBe(1);
  const res = await request.post(`/api/games/${game}/input`, {
    data: { line: String(state.prompt.rows[0].number), expect: state.prompt.kind },
  });
  expect(res.status(), await res.text()).toBe(200);
}

async function openPanel(page: import("@playwright/test").Page, game: string, label: string) {
  await page.goto(`/?game=${game}`);
  const pick = page.getByRole("button", { name: game, exact: false }).first();
  if (await pick.count()) await pick.click().catch(() => {});
  await expect(page.getByRole("heading", { name: "Yokohama" })).toBeVisible();
  await page.getByRole("button", { name: label, exact: true }).click();
  await expect(page.getByRole("dialog", { name: label })).toBeVisible();
  // **Wait for the ANSWER, not for the dialog.** These two panels are the only
  // ones on this API that run a real search, so unlike the four read-only
  // surfaces they are not answered in the same tick the dialog opens. A read
  // taken between the click and the response sees `loading…` and reports "the
  // panel drew nothing" — a true statement about that instant and a false one
  // about the panel. This waits on the same condition the client renders from:
  // a chart, a standing, or a refusal, whichever the answer is.
  await expect(page.locator(".surface-body .muted", { hasText: "loading" })).toHaveCount(0, {
    timeout: 30_000,
  });
}

test("the why panel draws a real chart, and the numbers on it add up", async ({ page, request }) => {
  const watch = watchForBreakage(page);
  await aGameWithABot(request, "explains");
  await openPanel(page, "explains", "Why");

  // A refusal is a legitimate answer here and would be a different assertion,
  // so say which one this is rather than failing obscurely inside the chart.
  const refusal = page.locator(".refusal code");
  if (await refusal.count()) {
    throw new Error(`why refused instead of accounting: ${await refusal.innerText()}`);
  }

  // **The chart has geometry.** jsdom has no layout, so "the bar element
  // exists" is all it can ever say; this asserts the bars were actually
  // painted, that the zero line is visible, and that every bar is anchored to
  // it on the side its sign says.
  const rows = page.locator(".bar-row");
  await expect(rows.first()).toBeVisible();
  const geometry = await page.evaluate(() => {
    const zero = document.querySelector(".bar-zero") as HTMLElement | null;
    const spine = zero?.getBoundingClientRect();
    return {
      spineWidth: spine?.width ?? 0,
      spineHeight: spine?.height ?? 0,
      bars: [...document.querySelectorAll(".bar-row")].map((row) => {
        const bar = row.querySelector(".bar") as HTMLElement | null;
        const r = bar?.getBoundingClientRect();
        const text = (row.querySelector(".bar-value") as HTMLElement).innerText;
        return {
          value: Number(text.replace("+", "")),
          width: r?.width ?? 0,
          right: r?.right ?? 0,
          left: r?.left ?? 0,
          helps: bar?.classList.contains("helps") ?? false,
        };
      }),
      zeroX: spine ? spine.left : 0,
      zeroRight: spine ? spine.right : 0,
    };
  });

  expect(geometry.spineWidth, "the zero line is painted").toBeGreaterThan(0);
  expect(geometry.spineHeight, "and it spans the rows").toBeGreaterThan(20);
  expect(geometry.bars.length).toBeGreaterThan(0);
  for (const bar of geometry.bars) {
    expect(bar.width, `a ${bar.value} bar has width`).toBeGreaterThan(0);
    // The FREE end is what says which side of zero a row is on; the anchored
    // end sits on the spine to within a pixel or two of subpixel layout, so
    // asserting it exactly would be asserting the renderer's rounding.
    if (bar.value > 0) {
      expect(bar.helps).toBe(true);
      expect(bar.right, `a +${bar.value} bar extends right of zero`).toBeGreaterThan(
        geometry.zeroRight,
      );
    } else {
      expect(bar.helps).toBe(false);
      expect(bar.left, `a ${bar.value} bar extends left of zero`).toBeLessThan(geometry.zeroX);
    }
  }

  // The printed total is the sum of the printed rows: the claim the surface
  // rests on, read off the SCREEN rather than off the payload.
  const total = Number((await page.locator(".total .num").innerText()).replace("+", ""));
  const sum = geometry.bars.reduce((a, b) => a + b.value, 0);
  expect(sum, "the visible rows add up to the printed total").toBe(total);

  expect(watch.failures).toEqual([]);
});

test("the collapsed features come back, and the total does not move when they do", async ({
  page,
  request,
}) => {
  await aGameWithABot(request, "collapses");
  await openPanel(page, "collapses", "Why");

  const before = await page.locator(".total .num").innerText();
  const rowsBefore = await page.locator(".bar-row").count();
  const disclosure = page.getByRole("button", { name: /Show the \d+ features contributing nothing/ });
  await expect(disclosure).toBeVisible();
  await disclosure.click();

  // Collapsing is not dropping. The identity holds over all 34 features, so the
  // total printed under the eight visible rows is the same number printed under
  // all 34 — which is what makes the collapse honest rather than a rounding.
  expect(await page.locator(".bar-row").count()).toBeGreaterThan(rowsBefore);
  expect(await page.locator(".total .num").innerText()).toBe(before);
});

test("suggest answers for the human seat, and the panel is escapable", async ({ page, request }) => {
  await aGameWithABot(request, "advises");
  await openPanel(page, "advises", "Suggest");

  // **A refusal is the WRONG answer here, so it is not an accepted one.**
  // `aGameWithABot` puts the bot on seat 0 AND makes it the start seat, so
  // `create`'s pump has already played the bot's turn and the HUMAN is to move
  // — which is exactly the seat `suggest` answers for. The first draft accepted
  // "an account OR a refusal, either is an answer", and that clause holds
  // whatever the server says: make `suggest` return `SuggestRefusal::BotSeat`
  // unconditionally and it stays green while the panel this test is named after
  // never draws. So say which answer this position must give, and fail naming
  // the refusal that came instead — the shape the `why` cell above already uses.
  const refusal = page.locator(".refusal code");
  if (await refusal.count()) {
    throw new Error(`suggest refused on the HUMAN seat: ${await refusal.innerText()}`);
  }
  expect(
    await page.locator(".bar-row").count(),
    "the suggest panel drew its account",
  ).toBeGreaterThan(0);

  // Real focus, which jsdom cannot have: the panel is `position: fixed` over
  // the whole board, so a keyboard user who opens it and cannot leave it is
  // trapped behind eighteen hidden tiles.
  await expect(page.locator(".surface")).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(page.locator(".surface")).toHaveCount(0);
});

test("a panel holding two hundred candidates still does not scroll sideways", async ({
  page,
  request,
}) => {
  await aGameWithABot(request, "wide");
  // **The opening decision is FORCED, which is the wrong place to ask this.**
  // Measured while fixing the assertion below: at the state this test used to
  // open the panel on, `suggest`'s ranking held ONE candidate, so `RunnerUps`
  // rendered nothing at all — no list, no disclosure — and the overflow check
  // ran over a panel with no candidates in it, under the name "two hundred
  // candidates". One move on, the seat is in Placement and the engine offers
  // ~186. The figure moves with the bot's own entropy, so nothing below asserts
  // it; the disclosure's own label is what the row count is checked against.
  await pastTheForcedOpening(request, "wide");
  await openPanel(page, "wide", "Suggest");

  // **The wide case has to actually BE wide.** `if (await more.count()) await
  // more.click()` was a clause that could silently never run: delete the
  // disclosure and the count is 0, the click is skipped, and the overflow check
  // below runs over the six collapsed rows — a page nothing here is about,
  // passing under the name "two hundred candidates". A refusal did the same
  // thing, with no rows at all. `Surfaces.test.tsx` records this exact shape
  // ("the first draft wrapped that assertion in `if (actionless.length > 0)`,
  // which is a clause that silently never ran"), so it is asserted here rather
  // than guarded: the button EXISTS, and the click puts every candidate the
  // button counted on screen.
  const more = page.getByRole("button", { name: /Show all \d+ candidates/ });
  expect(await more.count(), "the disclosure this test exists to open").toBe(1);
  const label = await more.innerText();
  const promised = Number(/Show all (\d+) candidates/.exec(label)?.[1]);
  expect(promised, `the disclosure names how many it is hiding: ${label}`).toBeGreaterThan(0);
  await more.click();
  await expect(more).toHaveCount(0);
  expect(
    await page.locator(".runner-ups li").count(),
    `every one of the ${promised} candidates the button named is on screen`,
  ).toBe(promised);

  for (const width of [1440, 1024]) {
    await page.setViewportSize({ width, height: 900 });
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
    );
    expect(overflow, `${width}px wide`).toBeLessThanOrEqual(0);
  }
});
