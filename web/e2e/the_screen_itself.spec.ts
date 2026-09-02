// What only a real browser can answer: does it LOAD, and can you leave it.
//
// jsdom has no layout, no paint, no image decoding and no real focus model, so
// every assertion in this file is one the component suite structurally cannot
// make. Each is also a defect class this lane has already met once.

import { expect, test, type Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { FLOOR_REM, tileWidthFor } from "../src/fit";
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

test("the answer bar is always in view — top of the side column above 1201, bottom edge below", async ({ page }) => {
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
      // **Prose still inside the deck — recorded beside the list, never gated.**
      // Until W6 this was the applied-move commentary, and it was the thing that
      // squeezed the list. It is now in the side column, where it cannot squeeze
      // anything. What is left as a direct child of `.decision` is the `place_a`
      // / `place_b` picker instructions and the `watching` / `locked` notices —
      // and `prompt.kind` is ONE discriminant, so none of those can be on screen
      // at the same time as an answer list at all. On a screen with a list this
      // reads 0 by construction. It is kept only because a future prompt kind
      // that renders both would make it a real figure again, and the reading
      // says which of the two happened.
      const msgs = document.querySelector(".decision > .messages");
      const commentary = msgs ? Math.round(msgs.getBoundingClientRect().height) : 0;
      if (!list) return { commentary, rows: 0, height: Infinity, clamped: false, clippedBy: 0, reached: true };
      // **Scroll the list to its top before hit-testing its first row.**
      // `.options.scrolls` is a scroll CONTAINER, so once it has been scrolled
      // — which ordinary play does — row 0 sits outside its own clip and
      // `elementFromPoint` at row 0's centre returns whatever is painted there
      // instead. The cell then reported "the first answer row was covered" for
      // a screen where nothing covered anything: measured on a clean tree at
      // `list 293px, commentary 6px`, i.e. with the commentary occupying six
      // pixels. Flaky at roughly two runs in three, and pre-existing.
      //
      // The question this cell asks is whether the COMMENTARY squeezes the
      // answer list until its first row is unreachable, so the row has to be in
      // view for the hit test to be about that at all.
      list.scrollTop = 0;
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
        // **Was the list actually CLAMPED on this screen?** Its own scroller
        // overflowing is the evidence that the deck's height budget, not the
        // list's content, decided how tall it is — which is the pressure the
        // floor below exists to survive. Without a clamped probe the floor is
        // a statement about a list that fit.
        clamped: list.scrollHeight > list.clientHeight,
        clippedBy: list.scrollHeight - list.clientHeight,
        reached,
      };
    });

  const pick = rng(0x5049_4e00); // "PIN"
  let worstList = Infinity;
  let mostCommentary = 0;
  let clampedProbes = 0;
  let worstClip = 0;
  let covered: string | null = null;
  for (let i = 0; i < 30; i += 1) {
    const screenNow = await readScreen(page);
    if (screenNow.over) break;
    if ((await act(page, screenNow, pick)) === null) throw await deadEnd(page, game, i);
    const p = await probe();
    // **Every figure is folded on the SAME screens the floor is taken on.**
    // `mostCommentary` used to be folded outside this branch, over probes with
    // no answer list on them at all, so the old anti-vacuity check and the floor
    // it claimed to justify were readings of two disjoint sets of screens. A
    // maximum taken where the thing it is compared against does not exist is not
    // a measurement of anything.
    if (p.rows > 0) {
      worstList = Math.min(worstList, p.height);
      mostCommentary = Math.max(mostCommentary, p.commentary);
      if (p.clamped) {
        clampedProbes += 1;
        worstClip = Math.max(worstClip, p.clippedBy);
      }
      if (!p.reached) covered = `move ${i}: the first answer row was covered (list ${p.height}px, clipped by ${p.clippedBy}px)`;
    }
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
  //
  // **What applies the pressure changed at W6, and the check has to say so.**
  // The commentary that used to do the squeezing is in the side column now, and
  // no prose can share the deck with the answer list any more — `prompt.kind` is
  // one discriminant and the remaining `.decision > .messages` blocks belong to
  // the other kinds. What still squeezes the list is the deck's OWN height
  // budget (45vh, minus the chain, the question and the controls), which is what
  // clamps it on a long prompt. So the anti-vacuity check is that the list was
  // really clamped — its scroller overflowing — on at least one probe. That is
  // what this walk can still establish, and it is what the floor is a floor for.

  // The floor, at the worst moment the run actually produced.
  expect(covered, covered ?? "").toBeNull();
  if (worstList !== Infinity) {
    expect(
      worstList,
      `the answer list was squeezed to ${worstList}px (clipped by up to ${worstClip}px)`,
    ).toBeGreaterThanOrEqual(30);
  }
  // eslint-disable-next-line no-console
  console.log(
    `deck under pressure: answer list bottomed out at ` +
      `${worstList === Infinity ? "n/a" : `${worstList}px`}, clamped on ${clampedProbes} probe(s), ` +
      `clipped by up to ${worstClip}px · in-deck prose beside a list peaked at ${mostCommentary}px (all recorded)`,
  );
  expect(
    clampedProbes,
    "the answer list was never clamped in this run — nothing squeezed it, so the floor above proved nothing",
  ).toBeGreaterThan(0);

  // **Two arms, because the deck has two homes (M5 spec §3.1).** Above 1201 it
  // is pinned at the top of the side column by the grid, and the thing that
  // scrolls is the TABLE; below 1201 it is M3.5's bottom-edge sticky bar and the
  // thing that scrolls is the page. Each arm scrolls its own scroller and
  // hit-tests every control at three offsets.
  const readDeck = () =>
    page.evaluate(() => {
      const deck = document.querySelector(".deck")!;
      const r = deck.getBoundingClientRect();
      // **The option list is a scroll CONTAINER, and a hit test has to know it.**
      // The same correction the probe above carries, for the same reason and now
      // for a second one: a row scrolled outside `.options.scrolls`'s own clip
      // still has a box and a centre, and `elementFromPoint` there returns
      // whatever is painted at that point — so an unclipped read reports
      // "covered" for a screen where nothing covers anything. It was latent
      // while the deck ran the full width; at 24rem the option labels wrap to
      // two and three lines and the list scrolls on ordinary prompts, and the
      // cell reported two covered rows on a clean tree. So: the list goes back
      // to its top, every control OUTSIDE it is hit-tested unconditionally, and
      // a control inside it is hit-tested while the point this test SAMPLES —
      // the button's centre, which is what `elementFromPoint` reads — is inside
      // it. A control pushed out of the designed scroller is still tested,
      // because that is the defect this cell exists for.
      const list = deck.querySelector(".options.scrolls");
      if (list) list.scrollTop = 0;
      const lr = list?.getBoundingClientRect();
      return {
        top: Math.round(r.top),
        bottom: Math.round(r.bottom),
        innerHeight: window.innerHeight,
        reachable: [...deck.querySelectorAll<HTMLElement>("button")]
          .filter((b) => b.offsetParent !== null)
          .filter((b) => {
            if (!list || !lr || !list.contains(b)) return true;
            const br = b.getBoundingClientRect();
            const centre = br.y + br.height / 2;
            return centre >= lr.top && centre <= lr.bottom;
          })
          .map((b) => {
            const br = b.getBoundingClientRect();
            const hit = document.elementFromPoint(br.x + br.width / 2, br.y + br.height / 2);
            return { name: (b.textContent ?? "").trim().slice(0, 24), reached: b.contains(hit) };
          }),
      };
    });
  const settle = () =>
    page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

  // Arm 1: the grid (the default Desktop Chrome viewport is 1280 wide).
  const tableBottom = await page.evaluate(() => document.querySelector(".table")!.scrollHeight);
  for (const y of [0, 400, tableBottom]) {
    await page.evaluate((to) => {
      document.querySelector(".table")!.scrollTop = to;
    }, y);
    await settle();
    const r = await readDeck();
    expect(r.top, `the deck runs off the top at table scroll ${y}`).toBeGreaterThanOrEqual(0);
    expect(r.bottom, `the deck runs below the viewport at table scroll ${y}`).toBeLessThanOrEqual(r.innerHeight);
    expect(r.reachable.length, `no control in the deck at table scroll ${y}`).toBeGreaterThan(0);
    expect(r.reachable.filter((c) => !c.reached).map((c) => c.name), `controls covered at table scroll ${y}`).toEqual([]);
  }

  // Arm 2: the column stack, exactly M3.5's assertions.
  await page.setViewportSize({ width: 1024, height: 844 });
  await settle();
  const bottomOfPage = await page.evaluate(() => document.body.scrollHeight);
  for (const y of [0, 400, bottomOfPage]) {
    await page.evaluate((to) => window.scrollTo(0, to), y);
    await settle();
    const r = await readDeck();
    expect(Math.abs(r.bottom - r.innerHeight), `the deck is not against the bottom edge at scrollY ${y}`).toBeLessThanOrEqual(1);
    expect(r.top, `the deck runs off the top of the viewport at scrollY ${y}`).toBeGreaterThanOrEqual(0);
    // The same floor Arm 1 takes, and Arm 2 needs it for a second reason: a deck
    // whose every control is outside `.options.scrolls`'s clip reads `[]` here,
    // and "nothing is covered" over an empty list is not a fact about the screen.
    expect(r.reachable.length, `no control in the deck at scrollY ${y}`).toBeGreaterThan(0);
    expect(r.reachable.filter((c) => !c.reached).map((c) => c.name), `controls covered at scrollY ${y}`).toEqual([]);
  }
  expect(watch.failures).toEqual([]);
});

test("only the option list scrolls inside the deck", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page);
  await page.waitForSelector(".decision .options");

  const read = () =>
    page.evaluate(() => {
      const options = document.querySelector(".decision .options.scrolls");
      const deck = document.querySelector(".deck");
      if (!options || !deck) return null;
      return {
        optionsOverflowY: getComputedStyle(options).overflowY,
        deckPosition: getComputedStyle(deck).position,
        deckMaxHeight: getComputedStyle(deck).maxHeight,
        innerHeight: window.innerHeight,
      };
    });

  // **What pins the deck is not the same declaration in both of its homes, so
  // the cell reads both (M5 spec §3.1).** Above 1201 the grid holds it in
  // column 2 row 3 and `position` is deliberately `static`; below, it is
  // M3.5's `sticky` bar against the bottom edge. Read as ONE value it was a
  // cell that could only ever be right about one of them — and it was right
  // about the wrong one, silently, because the grid's `.deck` override sits
  // 400 lines above `.deck`'s own rule and lost the cascade to it until the
  // child combinator went in. The option list stays the only scroller in both.
  const grid = await read();
  expect(grid, "the opening prompt must offer options to be measured").not.toBeNull();
  expect(grid!.deckPosition, "above 1201 the grid pins the deck, not `sticky`").toBe("static");
  // 45vh, not the base rule's 62vh: the same cascade, read as a number rather
  // than trusted. Computed from the viewport it was measured in.
  expect(
    Math.round(parseFloat(grid!.deckMaxHeight)),
    "the grid arm lost its own max-height to the cascade",
  ).toBe(Math.round(0.45 * grid!.innerHeight));
  expect(grid!.optionsOverflowY, "the option list stopped being the scroller").toBe("auto");

  await page.setViewportSize({ width: 1024, height: 844 });
  const stack = await read();
  expect(stack!.deckPosition, "below 1201 the deck stopped being pinned").toBe("sticky");
  expect(stack!.optionsOverflowY, "the option list stopped being the scroller").toBe("auto");

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

  // **Re-aimed at M5's tile, which has no caption left to measure.** The cell
  // used to read `.tile-body`'s background-image, because the scrim it guards
  // against was painted on the caption — and a scrim anchored to the text is a
  // scrim on the ladder, the two occupying the same 35% of the tile. W4b
  // deleted the caption, so the only place a scrim could now live is the art
  // box itself, and that is what this reads. The other half is unchanged: the
  // art is not dimmed.
  const read = await page.evaluate(() => {
    const art = document.querySelector(".tile-art");
    const box = art?.closest(".tile-art-box");
    if (!art || !box) return null;
    return {
      artOpacity: getComputedStyle(art).opacity,
      boxOpacity: getComputedStyle(box).opacity,
      boxBackgroundImage: getComputedStyle(box).backgroundImage,
    };
  });

  expect(read, "a tile must draw art in a box to be measured").not.toBeNull();
  // The printed power ladder reached the viewer at a mean transmittance of 0.100
  // under `--art-veil: 0.82` plus a six-stop scrim. It is the only on-board
  // carrier of what an Area Action pays at each power.
  expect(read!.artOpacity, "the art is dimmed again — the ladder is the cost").toBe("1");
  expect(read!.boxOpacity, "the box dims the art the `.tile-art` opacity does not").toBe("1");
  expect(
    read!.boxBackgroundImage,
    "a scrim is back over the art, and it covers the ladder with it",
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

test("the drawers that remain open, draw, and close", async ({ page }) => {
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

  // **Three, not four, and they are the ones that have no BOARD.** W5a retired
  // the port offer, the laboratory offer, the tracks and the building sites as
  // drawers — they are boards and they belong on the table (`Strips.tsx`), so a
  // loop that still opened them would be opening buttons the nav no longer has.
  for (const name of ["Areas", "Log", "Help"]) {
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

  expect(watch.failures).toEqual([]);
});

// **SPLIT from the cell above, the way W2 split the banner cell (ruling R-T7a).**
//
// The dummy read used to ride along here, taken in the Tracks SURFACE — because
// W6 had already taken the tracks out of the side column, so the Dashboard's
// strip was on no screen at all and `.space.dummy` would have counted 0 elements
// out of 0 and passed for the wrong reason. W5a then retired that surface too:
// the tracks are a section of the table now, and the section does not mount
// until W5b. So between the two there is no screen anywhere carrying a track,
// and this half cannot pass. It is a cell of its own rather than a deleted
// assertion, so the coverage is visible while it is gone and comes back by
// removing one annotation. The four-seats and Canal half above is unaffected
// and stays live under its own name.
test("the 2P dummy is on no track at 4P", async ({ page }) => {
  test.fail(
    true,
    "The Tracks DRAWER retired in W5a and the Tracks STRIP mounts in W5b (Task 12), " +
      "so between the two there is no track on any screen for this read to take. " +
      "Task 12 removes this annotation and the cell must then pass against " +
      "`.table-sec.tracks`.",
  );
  const watch = watchForBreakage(page);
  await startGame(page, { players: 4 });

  // The dummy is 2P-only: `place_setup_dummies` returns early above two players,
  // so a browser that shows a dummy occupant here is showing a rule that cannot
  // apply. The Dashboard heading's own dummy clause is still covered, in
  // `Dashboard.test.tsx`'s 2P/4P pair.
  //
  // The spaces are asserted to EXIST first, for the reason this cell has always
  // given: an absence assertion over a locator that matches nothing is not an
  // absence, it is a blank screen.
  const strip = page.locator(".table-sec.tracks");
  await expect(strip.locator(".space").first()).toBeVisible();
  const spaces = await strip.locator(".space").count();
  expect(spaces, "the tracks strip drew no spaces").toBeGreaterThan(0);
  await expect(strip.locator(".space.dummy")).toHaveCount(0);

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
  // `/result` is a 409 by design on a running game, and deriving the surface
  // list from the nav (correctly) put **Final score** in this loop.
  const watch = watchForBreakage(page, [{ status: 409, urlIncludes: "/result" }]);
  // 1440 and 900 bracket the 1201 breakpoint from both sides. Below it the panel
  // is deliberately NOT a drawer: `min(44rem, 55vw)` shrinks faster than the old
  // modal, and measured, three panels clip inside it at 1024 and 900 while none
  // clips in the modal. There is also no board to sit beside there — the stage
  // has already gone to a column.
  for (const width of [1440, 900]) {
    await page.setViewportSize({ width, height: 900 });
    await startGame(page, { players: 4 });

    // **Read off the page, not off a list in this file.**
    //
    // This cell used to iterate six hard-coded labels while the registry held
    // thirteen surfaces, so seven were never measured — and the escape had
    // already fired: `54f0efd` added `tracks` and `buildings` to the registry
    // and touched nothing in `web/e2e/`, so two whole panels shipped with no
    // containment assertion and every suite green. The nav IS the registry
    // rendered, so enumerating the nav is the one derivation available here
    // that cannot fall behind it (`e2e/` imports nothing from `src/`, and a
    // React module is not importable into a Playwright spec without a build).
    //
    // **Scoped to `.views`, which is what the sentence above already claimed.**
    // It read the whole `.banner` and was equivalent only while the nav was the
    // only thing in it holding buttons. W2b put the Game menu in the banner
    // beside the nav — same `.cards-menu` disclosure idiom, same `ghost`
    // buttons — so the unscoped reads collected **New game** and every saved
    // game id as if they were surfaces, and `.cards-menu > summary` resolved to
    // two elements. The nav is the registry rendered; the Game menu is not a
    // surface.
    await page.locator(".views .cards-menu > summary").click();
    const labels = await page.evaluate(() =>
      [...document.querySelectorAll(".views button.ghost")]
        .map((b) => (b.textContent ?? "").trim())
        .filter((t) => t.length > 0),
    );
    await page.keyboard.press("Escape");
    // Anti-vacuity: a selector that stopped matching would make this loop
    // iterate nothing and the test would pass having measured no panel at all.
    // TEN at this head — Areas, Log, Suggest, Why and Help inline, plus the five
    // behind `Cards` — down from fourteen because W5a retired four drawers to
    // the table. Asserted as a floor rather than "some": a floor of nine
    // tolerates exactly the thing this derivation exists to stop — a surface
    // disappearing from the nav without a test noticing.
    expect(
      labels.length,
      `the nav listed ${labels.length} surfaces, not the 10 the registry holds`,
    ).toBeGreaterThanOrEqual(10);

    for (const label of labels) {
      // A card-reference surface lives behind the disclosure; opening it when it
      // is already open would close it, so this asks rather than toggles.
      const inMenu = await page
        .locator(`.views .cards-menu button.ghost:text-is("${label}")`)
        .count();
      if (inMenu > 0 && !(await page.locator(".views .cards-menu[open]").count())) {
        await page.locator(".views .cards-menu > summary").click();
      }
      await page.getByRole("button", { name: label, exact: true }).click();
      // **Wait for images, or this assertion is blind to them.** The Help panel
      // loads its screenshots with `loading="lazy"`, so measured immediately
      // after the click they still have a 0x0 box and contribute nothing to
      // `scrollWidth` — a 794px overflow passed this cell exactly that way.
      await page.evaluate(async () => {
        const imgs = [...document.querySelectorAll<HTMLImageElement>(".surface-body img")];
        imgs.forEach((i) => i.removeAttribute("loading"));
        await Promise.all(imgs.map((i) => (i.complete ? null : i.decode().catch(() => null))));
      });
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

// **These were ONE cell, and splitting them is M5 W2's ruling R-T7a.**
//
// The two halves had nothing to do with each other beyond both being read at
// three widths, and keeping them together meant a banner regression could take
// the modality assertion — which is live, green and mutation-proven — down with
// it. They are separate cells now: the one below is true today, the one after it
// is annotated as expected-to-fail with the work that clears it.

test("a panel really is modal", async ({ page }) => {
  const watch = watchForBreakage(page);

  // **This cell had to change its mind, and the reason is worth keeping.**
  //
  // It was written to assert what the review promised the nav grouping would
  // buy: that you can switch from Areas to Site cards without pressing Escape,
  // because eleven buttons wrapped the banner to a second row that sat under the
  // open panel.
  //
  // But the accessibility fix landed first and supersedes it: `showModal()` puts
  // the panel in the top layer with a backdrop over the whole viewport, so
  // NOTHING outside the dialog is clickable — that is what modal means, and it
  // is the containment the panel spent three milestones claiming without having.
  // So the nav is inert while a panel is open by design, not by defect.
  //
  // Asserting the property that is actually true, rather than quietly deleting
  // the cell, because "the nav is usable while a panel is open" is exactly the
  // kind of sentence this lane has shipped four times without checking.
  for (const width of [1440, 1280, 1024]) {
    await page.setViewportSize({ width, height: 900 });
    await startGame(page);

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

// **This WAS a cell expected to fail, and W5a is what cleared it.**
//
// The claim is M3.5's — "the banner is one row again" — and it is the half of the
// old cell that had stopped being true. What it replaces asserted
// `nav.getBoundingClientRect().bottom < 60`, a VIEWPORT-relative edge, on a page
// that `startGame` always left scrolled 258–273px: measured at HEAD, that read
// returned −230 / −170 / −185 at 1440 / 1280 / 1024, so it was satisfied by the
// scroll offset at every width and had never tested the banner anywhere. M5 W2's
// grid gives `.app` a `height: 100vh`, so the page stops scrolling above 1201 and
// the read is honest for the first time — at 1440 it now reads 43 and at 1280 it
// reads 88.
//
// **The measurement here is the banner's own HEIGHT**, and it is the right one
// because a `getBoundingClientRect()` height does not change with scroll at all,
// while any EDGE — `nav.bottom`, with or without `window.scrollY` added back —
// only becomes scroll-independent on the assumption that the banner sits at the
// document's top. Height needs no such assumption and is what "one row" means.
//
// And the one-row reference is COMPUTED from the page rather than written down:
// a banner of one row is its own padding plus its border plus its tallest child,
// so nothing here is a threshold somebody picked. That box is 24 + 1 + 31 = 56px.
//
// **The annotation is gone because the work it named landed, and it landed one
// task EARLIER than it predicted.** It said the banner needed ~49px back at 1280
// (161 title + 20 + 239 meta + 20 + 849 nav against a 1240px content box) and
// named three scheduled pieces of M5 that would supply it — W2b's compact Game
// menu, W5a retiring four drawer buttons from the nav, and W5b's Boards
// disclosure — with "Task 12 removes this annotation". W2b shipped first, and
// W5a's four buttons were then enough on their own: measured at the W5a head,
//
//     1440px → 56px   ONE ROW
//     1280px → 56px   ONE ROW
//     1024px → 56px   ONE ROW
//
// against the recorded pre-W5a figures of 56 / 101 / 101. So the cell asserts
// what it always claimed, W5b has nothing left to buy here, and 1024 — which
// this note said would "stay two rows even after the work that fixes 1280" —
// came along for free. 1024 is still measured and RECORDED rather than asserted:
// the sub-1201 column stack is not what this milestone is about, and nothing
// here should start gating it by accident.
//
// **One limit of the reference, stated so it is not mistaken for more.**
// `oneRowBox` is the banner's padding and border plus its TALLEST CHILD, and
// `nav.views` is itself `flex-wrap: wrap` — so a nav that wrapped INTERNALLY
// would grow that child, the reference would grow with it, and the comparison
// would not register the wrap. The assertion is still the right one for what it
// claims, because the regression it exists to catch was the BANNER's own line
// breaking — `.banner` is `flex-wrap: wrap` too, and at 1280 the nav moved
// below the title, which is the 101-against-56 reading recorded above. A nav
// wrapping inside its own box is a different measurement and is not this cell's.
test("the banner is one row at 1440 and 1280", async ({ page }) => {
  const watch = watchForBreakage(page);

  // Every width is measured BEFORE anything is asserted, so the record below is
  // printed whole even though the run stops at the first width that wrapped.
  const rows: { width: number; height: number; oneRowBox: number; wrapped: boolean }[] = [];
  for (const width of [1440, 1280, 1024]) {
    await page.setViewportSize({ width, height: 900 });
    await startGame(page);
    const r = await page.evaluate(() => {
      const banner = document.querySelector("header.banner");
      if (!banner) return null;
      const cs = getComputedStyle(banner);
      const box =
        parseFloat(cs.paddingTop) +
        parseFloat(cs.paddingBottom) +
        parseFloat(cs.borderTopWidth) +
        parseFloat(cs.borderBottomWidth) +
        Math.max(...[...banner.children].map((c) => c.getBoundingClientRect().height));
      const height = banner.getBoundingClientRect().height;
      // The same 1px of slack the page-overflow read takes, and for the same
      // reason: a border box is not an integer and an exact compare reports
      // fractional rounding as a wrap.
      return { height: Math.round(height), oneRowBox: Math.round(box), wrapped: height > box + 1 };
    });
    expect(r, `the banner did not draw at ${width}px`).not.toBeNull();
    rows.push({ width, ...r! });
  }

  // eslint-disable-next-line no-console
  console.log(
    "banner rows: " +
      rows.map((r) => `${r.width}px ${r.height}px in a ${r.oneRowBox}px one-row box`).join(" · "),
  );
  expect(rows.length, "a width was skipped, so the record above is short").toBe(3);

  for (const r of rows.filter((row) => row.width >= 1280)) {
    expect(
      r.wrapped,
      `the banner wrapped at ${r.width}px: ${r.height}px against a one-row box of ${r.oneRowBox}px`,
    ).toBe(false);
  }

  expect(watch.failures).toEqual([]);
});

// ---- M4: the replay scrubber -----------------------------------------------

/** Play `moves` real moves off the screen, so there is a past to scrub into. */
async function playAFew(page: Page, moves: number) {
  const pick = rng(20260901);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

test("scrubbing walks the board backwards and puts it back exactly", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1440, height: 900 });
  await startGame(page, { players: 2 });
  await playAFew(page, 14);

  // The board as it stands now, read off the SCREEN — this is the thing the
  // round trip has to restore.
  //
  // **Not `innerText` any more, and the reason is the whole of this cell's
  // teeth.** Until M5 the board's text WAS its position: the caption printed the
  // pawns and the built site slots as glyphs. W4b draws both as pieces on the
  // art, so the board's remaining text is the id badges and the hidden spans —
  // the same string at every scrub position. This cell caught that by failing;
  // its second half (`.toBe(live)` after "Back to live") would have passed
  // forever while comparing one constant to another. So it reads what the
  // screen now says instead: each tile's pieces by their accessible name, and
  // how many printed spaces are built on.
  const boardState = () =>
    page.evaluate(() =>
      [...document.querySelectorAll(".tile")]
        .map((t) => {
          const id = t.querySelector(".tile-id")?.textContent ?? "?";
          const pieces = t.querySelector(".pieces")?.getAttribute("aria-label") ?? "";
          const built = t.querySelectorAll(".site-space.built, .site-trading-house.built").length;
          return `${id}[${pieces}]${built}`;
        })
        .join(" "),
    );
  const live = await boardState();

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await expect(track).toBeVisible();
  const total = Number(await track.getAttribute("max"));
  expect(total, "the walk applied nothing, so there is no past to scrub into").toBeGreaterThan(3);

  // **Count the requests a drag makes.** A scrubber without a debounce fires one
  // per intermediate value and a cell that only checked the final board would
  // pass it. The client debounces at 200 ms, so a burst of moves must collapse.
  const atRequests: string[] = [];
  page.on("request", (r) => {
    if (/\/at\/\d+$/.test(new URL(r.url()).pathname)) atRequests.push(r.url());
  });

  // **Spaced, or this measures the wrong thing.** Eight presses in a tight loop
  // finish in ~25 ms, and React's effect cleanup collapses the pending fetches
  // on its own — so the cell passed with the debounce set to 0 ms
  // (mutation-proven). Spacing them past a frame each means an UNDEBOUNCED
  // client issues one request per press, while the 200 ms timer still coalesces
  // them into one or two.
  await track.focus();
  for (let i = 0; i < 8; i++) {
    await page.keyboard.press("ArrowLeft");
    await page.waitForTimeout(30);
  }

  await expect(page.locator(".table.replaying")).toBeVisible();
  await expect
    .poll(async () => (await boardState()) !== live, {
      message: "the board never changed, so nothing was actually replayed",
    })
    .toBe(true);

  expect(
    atRequests.length,
    `eight spaced steps made ${atRequests.length} requests — the debounce is not debouncing`,
  ).toBeLessThanOrEqual(3);

  // Back to the present, and the board is what it was. This is the property a
  // read-only projection owes: browsing the past must not change it.
  await page.getByRole("button", { name: "Back to live" }).click();
  await expect(page.locator(".table.replaying")).toHaveCount(0);
  await expect.poll(boardState).toBe(live);

  expect(watch.failures).toEqual([]);
});

test("the replayed board is full size beside the drawer, and nothing on it is pressable", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  // 1440 only: the drawer exists above 1201px, and below it the panel covers the
  // board by design (`styles.css`'s `@media (min-width: 1201px)`), so
  // board-beside-drawer is a claim this spec makes at ONE width and says so.
  await page.setViewportSize({ width: 1440, height: 900 });
  await startGame(page, { players: 4 });

  // **Walk to a LIVE placement picker first, or this cell proves nothing.**
  // `App` only passes `onTileClick` while a picker is open, so at an ordinary
  // decision the live board already has zero pressable tiles — and the
  // assertion below then holds whether or not the replay guard exists.
  // Mutation-proven: with `&& !past` deleted from App.tsx the old version of
  // this cell still passed.
  const pick = rng(0x494e_4552); // "INER"
  let pressableLive = 0;
  for (let i = 0; i < 40 && pressableLive === 0; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    pressableLive = await page.locator("button.tile").count();
    if (pressableLive > 0) break;
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(
    pressableLive,
    "never reached a live placement picker, so a board with no pressable tile proves nothing",
  ).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await expect(track).toBeVisible();
  await track.focus();
  await page.keyboard.press("ArrowLeft");
  await expect(page.locator(".table.replaying")).toBeVisible();

  const read = await page.evaluate(() => {
    const board = document.querySelector(".board") as HTMLElement | null;
    const panelBody = document.querySelector(".surface .surface-body");
    const table = document.querySelector(".table");
    if (!board || !panelBody) return null;
    return {
      // The board is on the STAGE, not inside the panel. If a future change
      // moved it in, this is the assertion that would catch it — and it is the
      // one the containment cell structurally cannot make, because `.board`
      // carries `overflow-x: auto` and a clipping scroll container contributes
      // no overflow to its ancestor's `scrollWidth`.
      insidePanel: panelBody.contains(board),
      // The RENDERED tile, in px, and the container the fit rule reads. Not
      // `--tile-w`: since M5 W3 that custom property holds an unresolved
      // `clamp(…)` token stream above 1201px, so a string compare against it
      // stopped being a measurement of anything the eye can see.
      tileW: board.querySelector(".tile")!.getBoundingClientRect().width,
      tableW: table?.clientWidth ?? 0,
      tableH: table?.clientHeight ?? 0,
      widest: Math.max(0, ...[...board.querySelectorAll(".board-row")].map((r) => r.children.length)),
      rows: board.querySelectorAll(".board-row").length,
      tiles: board.querySelectorAll(".tile").length,
      pressable: board.querySelectorAll("button.tile").length,
      boardOverflow: board.scrollWidth - board.clientWidth,
    };
  });

  expect(read, "no board and no panel body on screen").not.toBeNull();
  expect(read!.insidePanel, "the board was rendered INSIDE the panel").toBe(false);
  // **Unshrunk, and the fit rule is what "unshrunk" now means.** This asserted
  // the 4P ladder's literal `"10rem"` until M5 W3 made the tile a `clamp()` over
  // the table's container units — so it asks the same question of the rendered
  // pixel instead: the tile beside an OPEN drawer is the size `fit.ts` says for
  // the table it is actually in, and never below the legibility floor. A board
  // squeezed to fit the drawer would read something smaller on both counts, and
  // that is the outcome this host design exists to avoid.
  const wantTile = tileWidthFor(read!.widest, read!.rows, read!.tableW, read!.tableH);
  expect(
    Math.abs(read!.tileW - wantTile),
    `the replayed tile rendered ${read!.tileW.toFixed(1)}px, the fit rule says ${wantTile.toFixed(1)} ` +
      `for a ${read!.tableW}x${read!.tableH} table`,
  ).toBeLessThanOrEqual(1);
  expect(read!.tileW, "the replayed board was shrunk below the legibility floor").toBeGreaterThanOrEqual(
    FLOOR_REM * 16,
  );
  expect(read!.tiles, "a 4P board draws eighteen tiles").toBe(18);
  expect(read!.boardOverflow, "the full-size board still fits the stage").toBeLessThanOrEqual(0);
  // A past position is not answerable, so the tiles that WERE buttons a moment
  // ago are inert `<div>`s now. The `toBeGreaterThan(0)` above is what makes
  // this a measurement rather than a tautology.
  expect(
    read!.pressable,
    `a historical board offered a pressable tile (the live board had ${pressableLive})`,
  ).toBe(0);

  expect(watch.failures).toEqual([]);
});
