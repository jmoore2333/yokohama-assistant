// What only a real browser can answer: does it LOAD, and can you leave it.
//
// jsdom has no layout, no paint, no image decoding and no real focus model, so
// every assertion in this file is one the component suite structurally cannot
// make. Each is also a defect class this lane has already met once.

import { expect, test, type Page } from "@playwright/test";
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

// ---- the art: the two rules that were deleted, and the box that must stay ----
//
// These are the only assertions in the repository that can see a CSS change to
// the tile, and they exist because the guard M3.5 was told it already had does
// not guard this: "every picture the client draws actually loads" asserts
// `naturalWidth !== 0` and `images.length > 10`, both of which a re-veiled or
// unreserved tile passes.
//
// Each cell reads a COMPUTED value that is literally one of the two rules the
// milestone removed, so neither is a magic number and neither goes stale when
// the design legitimately changes something else.

test("the tile art is not veiled and carries no scrim", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page);
  await page.waitForSelector(".tile-art");

  const read = await page.evaluate(() => {
    const art = document.querySelector(".tile-art");
    const body = document.querySelector(".tile-body");
    if (!art || !body) return null;
    return {
      artOpacity: getComputedStyle(art).opacity,
      bodyBackgroundImage: getComputedStyle(body).backgroundImage,
    };
  });

  expect(read, "a tile must draw art and a caption to be measured").not.toBeNull();
  // The printed power ladder reached the viewer at a mean transmittance of 0.100
  // under `--art-veil: 0.82` plus a six-stop scrim. It is the only on-board
  // carrier of what an Area Action pays at each power.
  expect(read!.artOpacity, "the art is dimmed again — the ladder is the cost").toBe("1");
  expect(
    read!.bodyBackgroundImage,
    "a scrim is back on the caption; anchored to the text it also covers the ladder",
  ).toBe("none");

  expect(watch.failures).toEqual([]);
});

test("a tile reserves its picture's box before the picture arrives", async ({ page }) => {
  const watch = watchForBreakage(page);

  // Hold every area image at the network so the box is measured EMPTY first.
  // The `aspect-ratio` moved from `.tile` to `.tile-art-box` when the caption
  // came out from under the picture, and the failure mode that swap invites is
  // silent: the art loads, every existing assertion passes, and the whole board
  // jumps a hundred pixels the moment it lands.
  let release: (() => void) | null = null;
  const held = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/art/areas/*", async (route) => {
    await held;
    await route.continue();
  });

  await startGame(page);
  await page.waitForSelector(".tile-art-box");
  const before = await page.evaluate(
    () => document.querySelector(".board")!.getBoundingClientRect().height,
  );

  // Release the parked handlers; do NOT `unroute`, which disposes them while
  // they are still awaiting and fails the requests it was meant to let through.
  release!();
  await page.waitForFunction(
    () => [...document.querySelectorAll<HTMLImageElement>(".tile-art")].every((i) => i.complete),
    null,
    { timeout: 15_000 },
  );
  const after = await page.evaluate(
    () => document.querySelector(".board")!.getBoundingClientRect().height,
  );

  // Anti-vacuity: a board of zero height does not jump either.
  expect(before, "the board must have drawn its reserved boxes").toBeGreaterThan(100);
  expect(
    Math.abs(after - before),
    `the board jumped ${Math.round(after - before)}px when the art landed`,
  ).toBeLessThanOrEqual(1);

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

// ---- horizontal overflow: what is measured, and what is merely recorded ----
//
// What a player experiences is the PAGE scrolling sideways; what causes it is
// ONE element wider than the window. Those are two different measurements, and
// the first one on its own is nearly blind here — the cell this replaced took
// only a `documentElement` delta, at 1440 and 1024, on a 2P game, and could not
// see either half of the defect it was written to catch:
//
//   * `.board` is `overflow-x: auto`, so ANY amount of board too wide for its
//     column is absorbed there and never reaches `documentElement`. The board
//     is the widest thing on the screen — the one element most likely to be
//     wrong is precisely the one a page delta cannot feel. Measured at 1440: a
//     4P board carries 384px outside its column — 28% of it off screen, behind
//     an overlay scrollbar — and the delta the old cell asserted on read 0.
//   * 1024 is BELOW the 1200px breakpoint, so it measured the COLUMN layout —
//     and a 2P board (915px) clears 1024 with room to spare, while 3P (1146px)
//     and 4P (1376px) were never measured at any width. The column arm shipped
//     with `align-items: flex-start` sizing `.board-col` to fit-content, which
//     is 1006px of document overflow at 390x844, recorded in three documents as
//     an accepted phone limitation and green in this file the whole time.
//
// So the read below returns the OFFENDING ELEMENTS — the outermost boxes whose
// right edge is past the window, each with its selector, its width and its
// scroll/client pair — beside the page delta, from one round trip. A failure
// then says `div.board-col is 1376px wide in a 390px window`, which is the bug,
// rather than `overflow: 1006`, which is a trip to the devtools.
//
// The two claims are deliberately of different strength:
//   (a) THE PAGE MUST NEVER SCROLL SIDEWAYS — at any of these widths, at any
//       player count. ASSERTED.
//   (b) `.board` MAY scroll inside its own column. That is its design: no
//       phone is 1376px wide. RECORDED, not gated. The one thing asserted
//       about it is that the overflow stays REACHABLE — `overflow-x: auto`,
//       never a silent `hidden` clip, which would satisfy (a) by hiding a
//       third of the board with no scrollbar and no clue.
//
// The widths bracket the 1200px breakpoint from BOTH sides — 1201 is the
// narrowest row layout, 1024 the widest column one — because a breakpoint is
// where layout bugs live and the old cell measured only one side of it. The
// player counts are all three because the board's width IS the player count:
// the widest row is `players + 2` tiles of 14rem (915px · 1146px · 1376px), so
// a 2P-only cell tests the narrowest board that exists.

/** One offending box, named well enough to fix without opening devtools. */
interface Box {
  selector: string;
  width: number;
  right: number;
  scrollWidth: number;
  clientWidth: number;
  overflowX: string;
}

interface OverflowRead {
  /** The page delta — what a player feels as a sideways scrollbar. */
  docScrollWidth: number;
  docClientWidth: number;
  /** The outermost boxes sticking out past the window: the CAUSE of that delta. */
  spills: Box[];
  /** The board's own scroller, which is allowed to have work to do. */
  board: Box | null;
  /** Anti-vacuity: the pyramid's widest row, in tiles. */
  widestRow: number;
}

/** Everything the layout question needs, in ONE round trip. */
async function readOverflow(page: Page): Promise<OverflowRead> {
  return page.evaluate(() => {
    const doc = document.documentElement;
    const viewport = doc.clientWidth;

    // A path short enough to read in a failure line and specific enough to grep
    // the stylesheet for.
    const selectorOf = (el: Element): string => {
      const hop = (n: Element) => {
        const cls = [...n.classList].join(".");
        return `${n.tagName.toLowerCase()}${n.id ? `#${n.id}` : ""}${cls ? `.${cls}` : ""}`;
      };
      const parts: string[] = [];
      for (let n: Element | null = el; n && parts.length < 3; n = n.parentElement) {
        parts.unshift(hop(n));
      }
      return parts.join(" > ");
    };

    const box = (el: Element): Box => {
      const r = el.getBoundingClientRect();
      return {
        selector: selectorOf(el),
        width: Math.round(r.width),
        right: Math.round(r.right),
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
        overflowX: getComputedStyle(el).overflowX,
      };
    };

    // 1px of slack: `clientWidth` is an integer and a border box is not, so an
    // exact compare reports fractional rounding as a defect.
    const past = (el: Element) => Math.round(el.getBoundingClientRect().right) > viewport + 1;

    const spills = [...document.querySelectorAll("*")]
      .filter((el) => el.getClientRects().length > 0 && past(el))
      // OUTERMOST only. Every descendant of an over-wide column is over-wide
      // too, and a list of forty of them buries the one that is the cause.
      .filter((el) => !el.parentElement || !past(el.parentElement))
      .map(box)
      .sort((a, b) => b.width - a.width)
      .slice(0, 5);

    const board = document.querySelector(".board");
    const rows = [...document.querySelectorAll(".board-row")].map(
      (row) => row.querySelectorAll(".tile").length,
    );

    return {
      docScrollWidth: doc.scrollWidth,
      docClientWidth: viewport,
      spills,
      board: board ? box(board) : null,
      widestRow: rows.length > 0 ? Math.max(...rows) : 0,
    };
  });
}

/** The failure line: WHAT overflowed, not merely by how much. */
function blame(read: OverflowRead): string {
  if (read.spills.length === 0) {
    return "and nothing sticks out of the window — look for a margin, a `position: absolute`, or a scroll anchor";
  }
  return read.spills
    .map(
      (s) =>
        `${s.selector} is ${s.width}px wide (right edge ${s.right} vs window ${read.docClientWidth}, ` +
        `content ${s.scrollWidth} in ${s.clientWidth}, overflow-x: ${s.overflowX})`,
    )
    .join("; ");
}

for (const players of [2, 3, 4] as const) {
  test(`nothing pushes the page sideways at ${players} players, phone to desktop`, async ({
    page,
  }) => {
    const watch = watchForBreakage(page);
    const game = await startGame(page, { players });

    // Play first: an empty opening board is the narrowest the screen ever is,
    // and the rail only fills up once there is something to report.
    const pick = rng(0x5749_4454 + players); // "WIDT", per seat count
    let moves = 0;
    for (let i = 0; i < 12; i += 1) {
      const screen = await readScreen(page);
      if (screen.over) break;
      if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, moves);
      moves += 1;
    }
    expect(moves, "the preamble must actually put something on the board").toBeGreaterThan(5);

    for (const width of [390, 1024, 1201, 1440]) {
      await page.setViewportSize({ width, height: 844 });
      // Read the layout AFTER it has been done: a resize and a measurement in
      // the same tick can be answered from the pre-resize box.
      await page.evaluate(
        () => new Promise((resolve) => requestAnimationFrame(() => resolve(null))),
      );
      const read = await readOverflow(page);

      // Anti-vacuity: a screen that drew no board has no overflow either, which
      // is exactly the failure a width assertion cannot feel. The pyramid's
      // widest row is `players + 2` tiles (`board_gen`'s row lengths: 4/3/2/1,
      // 5/4/3/2, 6/5/4/3), and that row IS the board's min-content width.
      expect(read.widestRow, `the board did not draw at ${width}px`).toBe(players + 2);

      // (a) The claim.
      expect(
        read.docScrollWidth - read.docClientWidth,
        `the page scrolls sideways at ${width}px with ${players} players: ${blame(read)}`,
      ).toBeLessThanOrEqual(0);

      // (b) The board's own scroller may have work to do — but the work must be
      // reachable.
      const board = read.board;
      expect(board, "the board element must exist to be measured").not.toBeNull();
      const inside = board ? board.scrollWidth - board.clientWidth : 0;
      if (board && inside > 0) {
        expect(
          board.overflowX,
          `the board hides ${inside}px of itself at ${width}px instead of scrolling it`,
        ).toMatch(/auto|scroll/);
      }
      // eslint-disable-next-line no-console
      console.log(
        `${players}P at ${width}px: page overflow ${read.docScrollWidth - read.docClientWidth}px ` +
          `(gated) · board scrolls ${inside}px inside its column (recorded)`,
      );
    }

    expect(watch.failures).toEqual([]);
  });
}

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
