// The visual gates (M5 spec §4.1): screenshots for a human, not assertions.
//
// **A generator, not a test**, in the shape of `shots.spec.ts`: it drives the
// shipped client and writes pictures for the owner to look at. Opt-in via
// `YOKOHAMA_WEB_GATES=1` (the config drops this file otherwise), and the output
// goes under the gitignored `docs/temp/web-review/gates/` because a picture of
// the art is the art. The few `expect`s stop a blank frame from being sent.
//
//     cd web && npm run build && YOKOHAMA_WEB_GATES=1 npx playwright test gates.spec.ts

import { expect, test, type Page } from "@playwright/test";
import { mkdirSync } from "node:fs";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, startGame } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");
// The owner's display is 2×; a 1× shot understates the art's legibility.
test.use({ deviceScaleFactor: 2 });

const OUT = "../docs/temp/web-review/gates";
const VIEWPORTS = [
  [1440, 900],
  [1710, 869],
  [2560, 1440],
] as const;

async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
  // **The last move's walk is skipped before the camera opens (M5.5).**
  // `readScreen` is what does it. Without this the shots that follow can be of
  // a table mid-playback — at an earlier position, outlined, with the answer
  // bar replaced by the playback strip — which is a picture of the wrong thing
  // under every caption in this file. Gate C drives its own loop precisely
  // because it wants that state.
  await readScreen(page);
}

const raf = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

for (const players of [2, 4] as const) {
  test(`V1 — the tiles at ${players}P, three viewports, and a popover`, async ({ page }) => {
    mkdirSync(OUT, { recursive: true });
    await page.setViewportSize({ width: 1710, height: 869 });
    await startGame(page, { players });
    await playAFew(page, 14, 0x4741_5445 + players); // "GATE"
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      // `act()` leaves the page wherever the last click was, and the page is
      // taller than every viewport here. The first run photographed 2P at a
      // mid-page scroll with the board's whole TOP ROW off-frame while 4P
      // happened to land at the top — the same defect `shots.spec.ts` records
      // in its own comment, and it made the two halves of the gate
      // incomparable. The camera is pointed at the top on purpose.
      await page.evaluate(() => window.scrollTo(0, 0));
      await raf(page);
      await expect(page.locator(".board .tile").first()).toBeVisible();
      await page.screenshot({ path: `${OUT}/v1-${players}p-${w}x${h}.png` });
    }
    await page.setViewportSize({ width: 1710, height: 869 });
    await page.evaluate(() => window.scrollTo(0, 0));
    // `.board .tile`, the same element the line above asserted visible: a bare
    // `.tile` is also a drawer's tile and a popover's, so the two reads could
    // name different elements and the picture would be of whichever came first
    // in the document.
    await page.locator(".board .tile").first().hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await page.screenshot({ path: `${OUT}/v1-${players}p-popover.png` });
  });
}

// The V2 flow (M5 spec §4.1's second gate): the finished table, in the six
// states worth looking at — every section on at the three viewports, the same
// table anchored at its bottom so the buildings strip is whole (ruling R-T13a),
// the two offers alone, a drawer over it, a replay position with the fetched
// sections collapsed, and the Game menu open. Same discipline as V1, a different
// stream so the two gates do not photograph the same fourteen moves.
//
// **Every shot asserts the state it claims to photograph, and the assertion is
// on CONTENT rather than on the wrapper.** `TableSections` renders its
// `loading()` placeholder under the very same `table-sec ${key}` class as the
// loaded strip (`web/src/TableSections.tsx:83-93`), so a check on the section
// element passes while the section reads "loading…" — which is a picture of a
// spinner captioned as a picture of the port. The strips are asserted through
// the elements only the loaded branch mounts: `.board-strip` for a port or a
// laboratory, `.site-card` for a building site.
//
// **Two things about the camera that the first run forced, both measured.**
//
// (1) `park()`. `playAFew` ends on a click, so the pointer is left resting on a
// tile and `TilePopover` opens under it — the first V2 run photographed a
// "Laboratory A" popover sitting over the middle of the 2P table at every
// viewport. V1 wants that popover and shoots it deliberately (:61); here it is a
// lens cap. The pointer goes to the banner's top-left corner before every shot,
// and `park` asserts the tooltip is gone rather than trusting that it went.
//
// (2) `scrollTable()`. Above 1201px `.app` is `height: 100vh` and `.table` is
// its own scroll container (`styles.css`), so the page does not scroll and the
// sections sit under a pyramid that has been FITTED to the table — which is the
// design (owner ruling R2), and which means a shot at the top of the table
// contains almost none of the section block. Measured at scrollTop 0 with all
// five sections on, over the six player-count × viewport cells: `.table`'s
// CLIENT height is 816 / 785 / 1356 at the three viewports (layout-determined,
// so it is the same every run) and its scrollHeight runs ≈2.3k–3.7k, which is
// game-dependent because the cards' text wraps differently — 2290–3658 on the
// run that produced these pictures, 2290–3679 on the first. The block itself is
// 1128–3056px tall and begins 587–1146px below the table's own CONTENT top, so
// at scrollTop 0 the frame holds between 0 and 306px of it. The per-cell table
// is in the task report.
//
// (An earlier draft of this comment said the block "starts 586–844px below the
// fold". Two errors in one figure: the quantity is the offset from the table's
// content TOP and not from the fold, and the range was taken over four of the
// six cells. It is restated above with its definition rather than patched.)
//
// So the shots whose subject IS the sections scroll the table until the block's
// top sits ~140px down, keeping the pyramid's foot in frame; the shots whose
// subject is the banner or the drawer go back to the top. The pyramid at these
// same three viewports is what the V1 pictures are, and this run rewrites those
// too.

/** The pointer off the board, and the hover popover with it. */
async function park(page: Page) {
  await page.mouse.move(2, 2);
  await expect(page.getByRole("tooltip")).toHaveCount(0);
}

/** Scroll `.table` to a named anchor.
 *
 *  **Throws rather than returning on a missing element.** The first draft
 *  returned silently when `.table-sections` was absent, which is the very
 *  failure the helper exists to prevent: a rename would have produced a shot at
 *  the top of the table, with no section in frame, under a green run. One
 *  helper and one failure mode also replaces the two hand-written
 *  `scrollTop = 0` evaluates that had a third behaviour again. */
async function scrollTable(page: Page, to: "top" | "sections" | "bottom") {
  const missing = await page.evaluate((where) => {
    const table = document.querySelector(".table") as HTMLElement | null;
    if (!table) return ".table";
    const max = table.scrollHeight - table.clientHeight;
    if (where === "top") table.scrollTop = 0;
    else if (where === "bottom") table.scrollTop = max;
    else {
      const secs = document.querySelector(".table-sections") as HTMLElement | null;
      if (!secs) return ".table-sections";
      const delta = secs.getBoundingClientRect().top - table.getBoundingClientRect().top - 140;
      table.scrollTop = Math.max(0, Math.min(max, table.scrollTop + delta));
    }
    return null;
  }, to);
  if (missing) throw new Error(`scrollTable("${to}"): ${missing} is not on the page`);
  await raf(page);
}

for (const players of [2, 4] as const) {
  test(`V2 — the table at ${players}P: all sections, the bottom anchor, offers only, a drawer, a replay, the menu`, async ({ page }) => {
    mkdirSync(OUT, { recursive: true });
    await page.setViewportSize({ width: 1710, height: 869 });
    // The server draws the seed, so every run photographs a different game.
    // `startGame` hands back the id and the seed it read off the responses that
    // carry them; recording it here is what lets a reader of the pictures say
    // WHICH game they are looking at without anyone transcribing it by hand.
    const game = await startGame(page, { players });
    test.info().annotations.push({
      type: "game",
      description: `${players}P ${game.id} seed ${game.seed}`,
    });
    await playAFew(page, 14, 0x5441_424c + players); // "TABL"

    const boards = page.locator(".boards-menu");
    const summary = page.locator(".boards-menu > summary");
    const toggle = async (...names: string[]) => {
      await summary.click();
      // Scoped to the menu: no strip renders a `<button>` today, but "Goals" is
      // also a card-index button in the Cards menu two elements along, and an
      // unscoped role lookup is one strip away from being ambiguous.
      for (const name of names) await boards.getByRole("button", { name }).click();
      await page.keyboard.press("Escape");
      // Escape is `App.tsx`'s own handler, not the browser's — a disclosure
      // ignores the key natively — so the close is a claim worth checking. A
      // menu left open sits over the table in every shot that follows.
      await expect(page.locator(".boards-menu[open]")).toHaveCount(0);
    };
    const ports = page.locator(".table-sec.ports .board-strip");
    const labs = page.locator(".table-sec.labs .board-strip");
    const sites = page.locator(".table-sec.buildings .site-card");

    await toggle("Building sites");
    await expect(summary).toHaveText("Boards · 5 on");
    await expect(ports.first()).toBeVisible();
    await expect(labs.first()).toBeVisible();
    await expect(sites.first()).toBeVisible();
    await expect(page.locator(".table-sec.tracks")).toHaveCount(1);
    await expect(page.locator(".table-sec.achievements")).toHaveCount(1);

    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await raf(page);
      // The brief's §3.3 comparison, answered at THIS head and RECORDED, never
      // asserted (spec §5): the realized tile width in rem, off the rendered
      // box, at each cell the fit rule is claimed for.
      const box = await page.locator(".board .tile").first().boundingBox();
      if (!box) throw new Error(`no tile box at ${players}P ${w}x${h}`);
      test.info().annotations.push({
        type: "fit",
        description: `${players}P ${w}x${h} tile ${(box.width / 16).toFixed(2)}rem`,
      });
      await scrollTable(page, "sections");
      await park(page);
      await page.screenshot({ path: `${OUT}/v2-${players}p-all-${w}x${h}.png` });
    }

    // **The bottom-anchored shot (ruling R-T13a).** The 4P section block runs
    // ~1.7k px against a 1356px frame, so the buildings strip is cut in every
    // other picture the owner gets. Anchored at `scrollTop = max` the strip is
    // whole by construction — the frame's bottom IS the content's bottom — and
    // that is asserted here rather than left to the eye.
    await scrollTable(page, "bottom");
    await expect(sites.first()).toBeVisible();
    const cut = await page.evaluate(() => {
      const table = (document.querySelector(".table") as HTMLElement).getBoundingClientRect();
      const strip = (document.querySelector(".table-sec.buildings") as HTMLElement).getBoundingClientRect();
      return strip.top < table.top || strip.bottom > table.bottom;
    });
    expect(cut, "the buildings strip is whole in the bottom-anchored frame").toBe(false);
    await park(page);
    // The viewport is whatever the loop above left, which is the LAST entry in
    // `VIEWPORTS`; the file name says so by reading it rather than by repeating
    // it, so a new last viewport renames the picture instead of mislabelling
    // it.
    const [lastW, lastH] = VIEWPORTS[VIEWPORTS.length - 1]!;
    await page.screenshot({ path: `${OUT}/v2-${players}p-bottom-${lastW}x${lastH}.png` });

    await page.setViewportSize({ width: 1710, height: 869 });
    await toggle("Tracks", "Goals", "Building sites");
    // "Offers only" is a claim about what is NOT there, so that is what is
    // checked; a missed toggle click would otherwise ship a five-section
    // picture under this file name. The summary's own count is the toggle
    // state read back from the app rather than assumed.
    await expect(summary).toHaveText("Boards · 2 on");
    await expect(page.locator(".table-sec.tracks, .table-sec.achievements, .table-sec.buildings")).toHaveCount(0);
    await expect(ports.first()).toBeVisible();
    await expect(labs.first()).toBeVisible();
    await scrollTable(page, "sections");
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-offers-only.png` });

    await page.getByRole("button", { name: "Areas", exact: true }).click();
    const drawer = page.getByRole("dialog");
    await expect(drawer).toBeVisible();
    await expect(drawer.locator(".area-card").first()).toBeVisible();
    await scrollTable(page, "top");
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-drawer.png` });
    await page.keyboard.press("Escape");

    // Everything goes back on for the replay shot, and that is the whole point
    // of the picture — but the contrast MOVED at M5.5. `/at/{k}` now carries
    // the port and the laboratory, so those two draw the offer that WAS there
    // beside the historical board; the building sites' board is read from the
    // live game and has no position-shaped version, so they are the one section
    // that collapses to "live offer, hidden while replaying". Shot with only
    // the two offers on — where the flow had left it — the replay picture has
    // no collapsed section in it at all.
    await toggle("Tracks", "Goals", "Building sites");
    await expect(summary).toHaveText("Boards · 5 on");
    await page.getByRole("button", { name: "Log", exact: true }).click();
    const track = page.getByLabel("Replay position");
    await track.focus();
    for (let i = 0; i < 4; i++) await page.keyboard.press("ArrowLeft");
    await expect(page.locator(".table.replaying")).toBeVisible();
    // Both halves of the contrast, asserted: the one fetched board with no
    // position behind it collapsed, and everything that CAN draw from the
    // position still drawing.
    await expect(page.locator(".table-sec.buildings.collapsed")).toHaveCount(1);
    await expect(page.getByText("live offer, hidden while replaying").first()).toBeVisible();
    await expect(ports.first()).toBeVisible();
    await expect(labs.first()).toBeVisible();
    await expect(page.locator(".table-sec.tracks:not(.collapsed)")).toHaveCount(1);
    await expect(page.locator(".table-sec.achievements:not(.collapsed)")).toHaveCount(1);
    await scrollTable(page, "sections");
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-replay.png` });
    await page.keyboard.press("Escape");

    await page.locator(".game-menu > summary").click();
    await expect(page.locator(".game-menu[open]")).toHaveCount(1);
    const menu = page.locator(".game-menu > div");
    await expect(menu.getByRole("button", { name: "New game" })).toBeVisible();
    // **The list is asserted RESOLVED, not non-empty.** `GameMenu` fetches on
    // toggle and renders "loading…" until the answer lands, which is the state
    // this shot could otherwise photograph. It is deliberately NOT
    // "at least one saved-game row": `others` filters out the current game
    // (`GameMenu.tsx`), so on a store with no earlier game the loaded list is
    // legitimately empty and prints "no other saved game" — an emptiness
    // assertion here would pass only because a previous run happened to leave a
    // file behind, and would fail on a clean machine.
    // Both, because either alone passes on the other: `GameMenu` REPLACES the
    // "loading…" line with an error when the list refuses, so a count of zero
    // is also what a failed fetch looks like.
    await expect(menu.getByText("loading…")).toHaveCount(0);
    await expect(menu.locator(".error")).toHaveCount(0);
    const rows = await menu.locator("button.ghost").count();
    test.info().annotations.push({
      type: "menu",
      description: `${players}P saved-game rows beside "New game": ${rows - 1}`,
    });
    await scrollTable(page, "top");
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-menu.png` });
  });
}

// The B gate (M5.5 spec §4.3): the set table, before the first choice.
//
// The picture the owner is asked to look at is the SECOND setup step — the
// position the rulebook has on the table when the two cards are drawn, with the
// player's own deal in the deck and the bot's two cards face down (R-C1).
// Everything asserted here is there so a blank or half-drawn frame cannot be
// sent under that caption; the judgement is the owner's.
test("gate B — the set table before the first choice", async ({ page }) => {
  mkdirSync(OUT, { recursive: true });
  await page.setViewportSize({ width: 1710, height: 869 });
  await page.goto("/");
  await page.getByRole("button", { name: "Deal" }).click();

  // On CONTENT, not on the wrapper, for `TableSections`' own reason: the
  // "loading…" placeholder carries the same `table-sec ports` class as the
  // loaded strip, and a set table has no game to fetch from — so the six cards
  // are what says the offers came off the preview's frame.
  await expect(page.locator(".table-sec.ports .strip-slot .card-face")).toHaveCount(6);
  await expect(page.locator(".table-sec.labs .board-strip")).toBeVisible();
  await expect(page.locator(".board .tile")).toHaveCount(10);
  await expect(page.locator(".deal-seat.human button.deal-pick")).toHaveCount(2);
  // **The bot's deal is face down (R-C1).** Two backs and the line, and no card
  // id anywhere under the bot — paired with the human's own two ids, so the
  // absence is the bot seat's and not a set table that drew no deal.
  await expect(page.locator(".deal-seat.human .card-id")).toHaveCount(2);
  await expect(page.locator(".deal-seat.bot img.back")).toHaveCount(2);
  await expect(page.locator(".deal-seat.bot .card-id")).toHaveCount(0);
  await expect(page.locator(".deal-seat.bot")).toContainText("keeps one of its two cards");
  // Before the first choice, which is what this gate is a picture OF.
  await expect(page.getByRole("button", { name: "Start" })).toBeDisabled();
  const seed = await page.locator("header.banner code").innerText();
  test.info().annotations.push({ type: "game", description: `2P set table, seed ${seed}` });

  // **Both frames are taken at scroll 0 — nothing is scrolled to make the
  // picture work (R-T12-3).** The deal cards drop their chip rows and the set
  // table trims its own paddings, so at 1710x869 the deck's block and the bot's
  // panel both fit under the banner. The figures are recorded, and the two
  // things the frame is a picture OF are asserted to be IN it: an element
  // scrolled out of the window still satisfies `toHaveText`, which is exactly
  // the picture this gate must not send.
  const layout = await page.evaluate(() => {
    const q = (x: string) => document.querySelector(x) as HTMLElement | null;
    const deck = q(".deck"), side = q(".side"), dec = q(".deck > .decision"), bot = q(".deal-seat.bot");
    if (!deck || !side || !dec || !bot) return null;
    return {
      deck: Math.round(deck.getBoundingClientRect().height),
      deckContent: dec.scrollHeight,
      side: Math.round(side.clientHeight),
      sideContent: side.scrollHeight,
      botPanel: Math.round(bot.getBoundingClientRect().height),
    };
  });
  if (!layout) throw new Error("gate B: the set table has no .deck / .side / bot deal to measure");
  test.info().annotations.push({
    type: "layout",
    description:
      `deck ${layout.deck}px holding ${layout.deckContent}px · side ${layout.side}px holding ` +
      `${layout.sideContent}px (bot panel ${layout.botPanel}px) — nothing scrolled`,
  });
  await expect(page.locator(".deal-seat.bot")).toBeInViewport({ ratio: 1 });
  await expect(page.locator(".deal-seat.bot .deal-unseen")).toBeInViewport({ ratio: 1 });
  for (let i = 0; i < 2; i++) {
    await expect(page.locator(".deal-seat.bot img.back").nth(i)).toBeInViewport({ ratio: 1 });
  }
  await expect(page.getByRole("button", { name: "Start" })).toBeInViewport({ ratio: 1 });

  await park(page);
  await page.screenshot({ path: `${OUT}/b-set-table-1710x869.png` });

  // **A second frame, scrolled to the strips.** The help and the README both say
  // the port and the laboratory are on the set table, and the first frame is
  // the first view — where the fitted pyramid fills the table and no strip is
  // in it. This is the one that shows the sentence is true. `.table` is its own
  // scroller above 1201px, so the side column and the deck do not move.
  await scrollTable(page, "sections");
  await expect(page.locator(".table-sec.ports .board-strip")).toBeInViewport();
  await expect(page.locator(".table-sec.labs .board-strip")).toBeInViewport();
  await park(page);
  await page.screenshot({ path: `${OUT}/b-set-table-strips-1710x869.png` });
});

// The C gate (M5.5 spec §4.4): the opponent's turn playing out on the table,
// paused and stepped so a still picture can carry it, and the live prompt it
// lands on. Six steps at most — a bot turn is usually shorter — each shot
// showing the table, the deck's strip and the account's lit row together,
// which is the only frame in which the three can be checked against each other.
//
// **This gate drives its own loop rather than `playAFew`.** `readScreen` SKIPS
// a running walk (harness, M5.5), which is the right thing everywhere else in
// this file and is the one thing that would make this gate photograph nothing.

// The title carries the words "gate C" because the documented invocation is
// `-g "gate C"`; V1 and V2 predate that and are reached by their own tokens.
test("gate C — the opponent's turn plays out on the table, stepped", async ({ page }) => {
  mkdirSync(OUT, { recursive: true });
  await page.setViewportSize({ width: 1710, height: 869 });
  const game = await startGame(page, { players: 2 });
  test.info().annotations.push({ type: "game", description: `2P ${game.id} seed ${game.seed}` });
  // One second a step: the pictures are stills, but a paused walk is reached
  // through a running one and the slowest cadence would spend the budget.
  await page.getByLabel("Cadence").selectOption("1000");

  const strip = page.locator(".decision.walking");
  const pick = rng(0x5354_4550); // "STEP"
  let found = false;
  for (let i = 0; i < 40 && !found; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
    // Either the walk has started or the next prompt is up; both are settled
    // states and neither is a sleep.
    await page
      .locator(".decision.walking, .decision button.option:not([disabled]), .decision button.primary:not([disabled])")
      .first()
      .waitFor({ timeout: 15_000 });
    found = (await strip.count()) > 0;
  }
  expect(found, "no bot turn to photograph in 40 actions").toBe(true);

  // Paused at once, so the camera is not racing the cadence.
  await page.getByRole("button", { name: "Pause" }).click();
  const steps = Number(/of (\d+)/.exec(await strip.innerText())![1]);
  const shots = Math.min(6, steps);
  test.info().annotations.push({ type: "walk", description: `${steps} steps, ${shots} photographed` });

  const lit = page.locator("ol.account li.at");
  for (let n = 1; n <= shots; n++) {
    // Every picture asserts the three things it claims to show, so a shot of a
    // finished walk or an empty account cannot be sent under this name.
    await expect(strip).toContainText(new RegExp(`is playing · step ${n} of ${steps}`));
    await expect(lit).toHaveCount(1);
    // What the step changed, RECORDED rather than gated: a bookkeeping event
    // legitimately changes nothing on the table, and a gate on it would be a
    // gate on which turn the seeded walk happened to find.
    const marks = await page.evaluate(() => ({
      tiles: document.querySelectorAll(".tile.changed").length,
      seats: document.querySelectorAll(".seat.changed").length,
      slots: document.querySelectorAll(".strip-slot.changed").length,
      outlined: document.querySelectorAll(".table.replaying").length,
    }));
    // **The camera points at what the step changed.** Above 1201px the table is
    // its own scroll container and the side column is not, so anchoring at the
    // sections keeps the deck's strip and the account's lit row in frame while
    // bringing the port and laboratory into it — which is the only way a lit
    // SLOT reaches a picture at all. A step that lights a tile keeps the
    // pyramid; one that lights only a slot would otherwise be a photograph of
    // ten unchanged tiles.
    const anchor = marks.slots > 0 && marks.tiles === 0 ? "sections" : "top";
    await scrollTable(page, anchor);
    await park(page);
    test.info().annotations.push({
      type: "step",
      description: `step ${n}/${steps} (anchored at the ${anchor}): ${marks.tiles} tiles, ${marks.seats} seats, ${marks.slots} slots lit, table outlined ${marks.outlined}`,
    });
    await page.screenshot({ path: `${OUT}/c-step-${n}.png` });
    if (n < shots) await page.getByRole("button", { name: "Step forward" }).click();
  }

  // Skipped from the DECK's own button — the account carries a `skip` too while
  // the walk runs, and an unscoped lookup is ambiguous rather than wrong.
  await strip.getByRole("button", { name: "skip" }).click();
  await expect(strip).toHaveCount(0);
  await expect(page.locator(".table.replaying")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "replay" })).toBeVisible();
  await scrollTable(page, "top");
  await park(page);
  await page.screenshot({ path: `${OUT}/c-live.png` });
});
