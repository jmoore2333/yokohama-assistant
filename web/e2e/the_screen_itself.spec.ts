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

// ---- the answer bar stays on screen, wherever the page is scrolled ----
//
// Two offsets, not one, and that is the whole assertion: a zone that happens to
// sit at the bottom of a short page satisfies a single read at scrollY 0 and
// tells you nothing. Pinned means "the same place at every offset".
//
// The numbers deliberately NOT asserted: the zone's viewport top (727px at 900,
// but that is `innerHeight − zoneHeight`, two design figures), its height, and
// the page height. Each is a choice the design is allowed to change. What must
// hold under any redesign is that the bar is against the bottom edge and fully
// on screen, which is what is written below.

test("the answer bar is pinned to the bottom at every scroll offset", async ({ page }) => {
  const watch = watchForBreakage(page);
  // 4P: the worst case, and the one measured at 1,106px below the fold before
  // the deck existed.
  const game = await startGame(page, { players: 4 });

  // **Play, and check the deck after EVERY move — not once at the end.**
  //
  // The squeeze this is really about is transient: the applied-move commentary
  // grows when the bot has just played and shrinks again on the next prompt, so
  // a single read taken whenever the loop happens to stop is a coin flip. Both
  // the floor and the hit test are therefore evaluated at every step, and the
  // worst case seen is what gets asserted and recorded — with an anti-vacuity
  // check that the run actually reached a state where the commentary was
  // competing for space at all.
  const probe = () =>
    page.evaluate(() => {
      const list = document.querySelector(".decision .options.scrolls");
      const msgs = document.querySelector(".decision > .messages");
      const commentary = msgs ? Math.round(msgs.getBoundingClientRect().height) : 0;
      if (!list) return { commentary, rows: 0, height: Infinity, reached: true };
      const first = list.querySelector(".option");
      let reached = true;
      if (first) {
        const b = first.getBoundingClientRect();
        const hit = document.elementFromPoint(b.x + b.width / 2, b.y + b.height / 2);
        reached = first.contains(hit);
      }
      return {
        commentary,
        rows: list.querySelectorAll(".option").length,
        height: Math.round(list.getBoundingClientRect().height),
        reached,
      };
    });

  const pick = rng(0x5049_4e00); // "PIN"
  let worstList = Infinity;
  let mostCommentary = 0;
  let covered: string | null = null;
  for (let i = 0; i < 30; i += 1) {
    const screenNow = await readScreen(page);
    if (screenNow.over) break;
    if ((await act(page, screenNow, pick)) === null) throw await deadEnd(page, game, i);
    const p = await probe();
    mostCommentary = Math.max(mostCommentary, p.commentary);
    if (p.rows > 0) {
      worstList = Math.min(worstList, p.height);
      if (!p.reached) covered = `move ${i}: the first answer row was covered (list ${p.height}px, commentary ${p.commentary}px)`;
    }
  }

  // The floor, at the worst moment the run actually produced.
  expect(covered, covered ?? "").toBeNull();
  if (worstList !== Infinity) {
    expect(
      worstList,
      `the answer list was squeezed to ${worstList}px (commentary peaked at ${mostCommentary}px)`,
    ).toBeGreaterThanOrEqual(30);
  }
  // Anti-vacuity: if the commentary never grew, the squeeze was never on and the
  // floor above proved nothing. Say so rather than reporting a green run.
  // eslint-disable-next-line no-console
  console.log(
    `deck under pressure: commentary peaked at ${mostCommentary}px, ` +
      `answer list bottomed out at ${worstList === Infinity ? "n/a" : `${worstList}px`} (recorded)`,
  );
  expect(
    mostCommentary,
    "this run never produced applied-move commentary, so the squeeze was never exercised",
  ).toBeGreaterThan(0);

  const read = async (y: number) => {
    await page.evaluate((to) => window.scrollTo(0, to), y);
    await page.evaluate(
      () => new Promise((resolve) => requestAnimationFrame(() => resolve(null))),
    );
    return page.evaluate(() => {
      const deck = document.querySelector(".deck")!;
      const r = deck.getBoundingClientRect();
      return {
        scrollY: Math.round(window.scrollY),
        top: Math.round(r.top),
        bottom: Math.round(r.bottom),
        innerHeight: window.innerHeight,
        // Every control the player can press, and whether the click would
        // actually reach it — the defect that made this a deck and not a rule.
        reachable: [...deck.querySelectorAll<HTMLElement>("button")]
          .filter((b) => b.offsetParent !== null)
          .map((b) => {
            const br = b.getBoundingClientRect();
            const hit = document.elementFromPoint(br.x + br.width / 2, br.y + br.height / 2);
            return { name: (b.textContent ?? "").trim().slice(0, 24), reached: b.contains(hit) };
          }),
      };
    });
  };

  const bottomOfPage = await page.evaluate(() => document.body.scrollHeight);
  for (const y of [0, 400, bottomOfPage]) {
    const r = await read(y);
    expect(
      Math.abs(r.bottom - r.innerHeight),
      `the deck is not against the bottom edge at scrollY ${r.scrollY} (bottom ${r.bottom} vs ${r.innerHeight})`,
    ).toBeLessThanOrEqual(1);
    expect(r.top, `the deck runs off the top of the viewport at scrollY ${r.scrollY}`).toBeGreaterThanOrEqual(0);

    // Anti-vacuity, and the real defect: a deck with no pressable control is
    // "pinned" by any geometric reading and useless to a player.
    expect(r.reachable.length, `no control in the deck at scrollY ${r.scrollY}`).toBeGreaterThan(0);
    const buried = r.reachable.filter((c) => !c.reached).map((c) => c.name);
    expect(buried, `controls in the deck are covered at scrollY ${r.scrollY}`).toEqual([]);
  }

  // **The answer list must never be the thing that gets squeezed to nothing.**
  //
  // This is the second defect the deck had, and it is the subtler one. The deck
  // is capped, so something inside it yields; the first version let the ANSWER
  // LIST yield, and against a bot seat the applied-move commentary reached
  // sixteen lines and 312px, clamped the list to a height of exactly 0, and made
  // the first option unclickable — its 34px box still had a centre, and the
  // centre landed on `.controls`. A player would have met that on move six.
  //
  // Asserted as a FLOOR and a hit test, not as a height: how tall the list is
  // depends on the prompt and on the viewport, but "at least one row, and you
  // can actually press it" holds under any redesign.
  expect(watch.failures).toEqual([]);
});

test("only the option list scrolls inside the deck", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page);
  await page.waitForSelector(".decision .options");

  const read = await page.evaluate(() => {
    const options = document.querySelector(".decision .options.scrolls");
    const deck = document.querySelector(".deck");
    if (!options || !deck) return null;
    return {
      optionsOverflowY: getComputedStyle(options).overflowY,
      deckPosition: getComputedStyle(deck).position,
    };
  });

  expect(read, "the opening prompt must offer options to be measured").not.toBeNull();
  // The two declarations the deck depends on, and nothing else. The component
  // suite cannot see either — the stylesheet is imported by `main.tsx` alone and
  // jsdom has no layout engine — so this is the only place they are checked.
  expect(read!.deckPosition, "the deck stopped being pinned").toBe("sticky");
  expect(read!.optionsOverflowY, "the option list stopped being the scroller").toBe("auto");

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

// ---- the panel: contained focus, a readable board beside it, a usable nav ----

test("focus cannot leave an open panel", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 4 });

  await page.getByRole("button", { name: "Areas", exact: true }).click();
  await expect(page.getByRole("dialog")).toBeVisible();

  // **Twelve presses, because six was enough to escape the old panel.**
  // Measured before this: the sixth Tab from the open panel landed on a nav
  // `button.ghost` labelled "Port offer" with `closest('.surface') === null`,
  // while the panel asserted `aria-modal="true"`. A claim of containment the DOM
  // does not have is worse than no claim, because a screen reader believes it.
  const walk: string[] = [];
  for (let i = 0; i < 12; i += 1) {
    await page.keyboard.press("Tab");
    walk.push(
      await page.evaluate(() => {
        const a = document.activeElement;
        if (!a || a === document.body) return "<body>";
        const inside = a.closest("dialog.surface") !== null;
        const name = (a.getAttribute("aria-label") ?? a.textContent ?? "").trim().slice(0, 24);
        return `${inside ? "in" : "OUT"}:${a.tagName.toLowerCase()}:${name}`;
      }),
    );
  }
  const escaped = walk.filter((w) => w.startsWith("OUT"));
  expect(escaped, `focus left the panel: ${walk.join(" → ")}`).toEqual([]);

  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toBeHidden();

  expect(watch.failures).toEqual([]);
});

test("every panel fits its drawer, and the board stays readable beside it", async ({ page }) => {
  const watch = watchForBreakage(page);
  // 1440 and 900 bracket the 1201 breakpoint from both sides. Below it the panel
  // is deliberately NOT a drawer: `min(44rem, 55vw)` shrinks faster than the old
  // modal, and measured, three panels clip inside it at 1024 and 900 while none
  // clips in the modal. There is also no board to sit beside there — the stage
  // has already gone to a column.
  for (const width of [1440, 900]) {
    await page.setViewportSize({ width, height: 900 });
    await startGame(page, { players: 4 });

    for (const label of ["Port offer", "Laboratory offer", "Areas", "Log", "Orders", "Goals"]) {
      if (["Orders", "Goals"].includes(label)) {
        await page.locator(".cards-menu > summary").click();
      }
      await page.getByRole("button", { name: label, exact: true }).click();
      const read = await page.evaluate(() => {
        const body = document.querySelector(".surface .surface-body");
        if (!body) return null;
        const br = body.getBoundingClientRect();
        const spills = [...body.querySelectorAll("*")]
          .filter((e) => {
            const r = e.getBoundingClientRect();
            return r.width > 0 && Math.round(r.right) > Math.round(br.right) + 1;
          })
          .map((e) => `${e.tagName.toLowerCase()}.${[...e.classList].join(".")}`);
        return { overflow: body.scrollWidth - body.clientWidth, spills: [...new Set(spills)] };
      });
      expect(read, `${label} did not draw a body at ${width}px`).not.toBeNull();
      // A CONTAINMENT relation, not a width. How wide the drawer is and how many
      // columns a grid lands on are design choices; "nothing sticks out of it"
      // has to hold under any of them.
      expect(
        read!.overflow,
        `${label} overflows its panel by ${read!.overflow}px at ${width}px`,
      ).toBeLessThanOrEqual(0);
      expect(
        read!.spills,
        `${label} has content past the panel's right edge at ${width}px`,
      ).toEqual([]);
      await page.keyboard.press("Escape");
    }
  }

  expect(watch.failures).toEqual([]);
});

test("a panel really is modal, and the banner is one row again", async ({ page }) => {
  const watch = watchForBreakage(page);

  // **This cell had to change its mind, and the reason is worth keeping.**
  //
  // It was written to assert what the review promised the nav grouping would
  // buy: that you can switch from Areas to Site cards without pressing Escape,
  // because eleven buttons wrapped the banner to a second row that sat under the
  // open panel. Measured, the grouping DOES fix the geometry — banner 93 → 56 at
  // 1440, nav bottom edge 43px against the panel's 48px top at 1440, 1280 AND
  // 1024, where before it was 88/88/125.
  //
  // But the accessibility fix landed first and supersedes it: `showModal()` puts
  // the panel in the top layer with a backdrop over the whole viewport, so
  // NOTHING outside the dialog is clickable — that is what modal means, and it
  // is the containment the panel spent three milestones claiming without having.
  // So the nav is inert while a panel is open by design, not by defect, and the
  // grouping now earns its place on the banner's height alone.
  //
  // Asserting the property that is actually true, rather than quietly deleting
  // the cell, because "the nav is usable while a panel is open" is exactly the
  // kind of sentence this lane has shipped four times without checking.
  for (const width of [1440, 1280, 1024]) {
    await page.setViewportSize({ width, height: 900 });
    await startGame(page);

    // One row: the banner does not wrap, at any laptop width.
    const oneRow = await page.evaluate(() => {
      const banner = document.querySelector("header.banner")!.getBoundingClientRect();
      const nav = document.querySelector("nav.views")!.getBoundingClientRect();
      return { banner: Math.round(banner.height), navBottom: Math.round(nav.bottom) };
    });
    expect(
      oneRow.navBottom,
      `the banner wrapped at ${width}px (nav bottom ${oneRow.navBottom})`,
    ).toBeLessThan(60);

    await page.getByRole("button", { name: "Areas", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();

    // The background is inert — the modality the panel now actually has.
    const reachableOutside = await page.evaluate(() => {
      const nav = document.querySelector("nav.views")!;
      const b = nav.querySelector<HTMLElement>("button")!;
      const r = b.getBoundingClientRect();
      const hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
      return b.contains(hit);
    });
    expect(reachableOutside, "the panel claims to be modal but the page is still live").toBe(false);

    // And the board is still READABLE behind it, which is why the backdrop is
    // nearly clear and why the panel is a drawer rather than a full-width sheet.
    const cover = await page.evaluate(() => {
      const s = document.querySelector(".surface")!.getBoundingClientRect();
      const b = document.querySelector(".board")!.getBoundingClientRect();
      const overlap = Math.max(0, Math.min(b.right, s.right) - Math.max(b.left, s.left));
      return { covered: Math.round(overlap), board: Math.round(b.width) };
    });
    if (width > 1200) {
      expect(
        cover.covered / cover.board,
        `the drawer covers ${cover.covered} of ${cover.board}px of board at ${width}px`,
      ).toBeLessThan(0.5);
    }

    // Escape, and the page is live again.
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toBeHidden();
    const liveAgain = await page.evaluate(() => {
      const b = document.querySelector<HTMLElement>("nav.views button")!;
      const r = b.getBoundingClientRect();
      return b.contains(document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2));
    });
    expect(liveAgain, "the page did not come back after the panel closed").toBe(true);
  }

  expect(watch.failures).toEqual([]);
});
