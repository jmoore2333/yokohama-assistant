// The help panel's screenshots, taken from the REAL application.
//
// **A generator, not a test.** Nothing here asserts a product property; it
// drives the shipped client in real Chromium against the real release binary
// and writes the seven `web/help-shots/*.png` — five whole screens and two
// crops of the answer bar (the choosers, at the end of the file) — each with
// the page's accessibility snapshot beside it as `<name>.aria.txt` (see
// `shot`). It lives in `e2e/` because that
// is where the browser and the server already are, and because a screenshot
// generator that needed its own harness would be a second driver of the same
// app.
//
// **Since M6.5b2 the PNG it writes is an INTERMEDIATE and is gitignored.**
// Playwright has no WebP encoder, and 4.2 MB of PNG was 29% of a release the
// service worker now precaches whole — so a second step cuts the committed
// `web/public/help/*.webp`, which is what Vite copies into `dist/` and the
// server serves at `/help/…`. The PNG is written OUTSIDE `public/` for exactly
// that reason: Vite copies that tree wholesale, so a PNG left in it would ship
// beside the WebP cut from it. Both commands, in this order:
//
// ```text
// cd web && npm run build && YOKOHAMA_WEB_SHOTS=1 npx playwright test shots.spec.ts
// python3 docs/tools/mkhelpart.py
// ```
//
// Running the first alone changes nothing a reader will see: the client reads
// the `.webp`.
//
// The env var is not optional: `playwright.config.ts` drops this file from the
// glob without it, so the command that omits it discovers zero tests and exits
// with an error. (It was written that way here first, which is the one place a
// reader would copy it from.)
//
// **Then LOOK at the pictures, and re-read what is written beside them.** The
// card-art suite already learned the first half the expensive way: its tests
// prove the join is TOTAL, not TRUE, and two real errors survived a
// complete-and-green mapping because nobody opened the images. A regenerated
// screenshot that nobody views is the same mistake with a bigger surface —
// these are what a reader is told the product looks like. The second half is
// this file's own: the server draws the seed, so every run photographs a
// DIFFERENT game, and each shot carries an `alt` and a `caption` in
// `web/src/Help.tsx` (and `screen`'s, `church`'s and `port`'s alts also in
// `docs/web-guide.md`, shorter) that claim what is in frame. Those were written against a picture and stayed behind when the
// picture moved on. Regenerating without re-reading them ships a description of
// a screenshot that no longer exists.
//
// The few assertions below are there so a BLANK or half-drawn frame cannot be
// written to disk and shipped as documentation; they check that the thing being
// photographed is on screen, not that it is correct.

import { unlinkSync, writeFileSync } from "node:fs";
import { type Locator, type Page } from "@playwright/test";
// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import { expect, test } from "./sw";
import { haveBrowser } from "../playwright.config";
import { act, CHOOSER_CLASS, LEGAL, openChooser, readScreen, rng, startGame, type ChooserArea } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

// **Outside `public/`, and that is the point** (M6.5b2). Vite copies `public/`
// into `dist/` wholesale, so a PNG written there would ship BESIDE the WebP cut
// from it — 4.2 MB of intermediate in a release the service worker precaches
// whole, which is exactly what the cut was for. This directory is gitignored
// and is read by `docs/tools/mkhelpart.py`, which writes the committed
// `web/public/help/*.webp`.
const SHOTS = "help-shots";

/** The owner's screen, and the shape every picture in the help is taken at.
 *
 *  Not a round number on purpose: 1710x869 is the viewport the M5 table was
 *  designed and gated against (`e2e/gates.spec.ts` shoots the same cell), and a
 *  help panel photographed at a width nobody uses documents a layout nobody
 *  sees. `deviceScaleFactor` is deliberately left at 1 here where the gates
 *  spec sets 2: these five whole screens (R-T14d added the strips picture)
 *  ship in `dist/` (the README stopped showing them when it took pictures of
 *  its own, S4b), so their bytes are a cost the repository pays forever, and
 *  the gates are the place that pays for pixel density. The two chooser crops
 *  at the end of the file are the exception, and their comment says why. The
 *  shape is every picture's, the crops' included: they are cut from this
 *  window, so their column is the one the owner's screen draws. */
const VIEWPORT = { width: 1710, height: 869 };

/** Play real moves so the shots show a game in progress rather than a setup. */
async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
}

/** Photograph the page as `help-shots/<name>.png`, and write its accessibility
 *  snapshot beside it as `help-shots/<name>.aria.txt`.
 *
 *  **The text is how the words beside a picture are checked without opening
 *  it.** No agent views these pictures, so the `alt` and `caption` in
 *  `Help.tsx` are re-read against this text instead, and the pictures are the
 *  owner's reading. The snapshot is taken with nothing between it and the
 *  screenshot, so it is the page that was photographed and not one a step
 *  before or after. An empty snapshot fails the run rather than writing an
 *  empty file a reader would take for "nothing on screen". The file is in the
 *  gitignored `help-shots/` with its PNG; `mkhelpart.py` reads `*.png` only. */
async function shot(page: Page, name: string, crop?: Locator): Promise<void> {
  // The earlier run's text goes first, so a run that fails after the
  // screenshot leaves no older text beside the newer picture.
  try {
    unlinkSync(`${SHOTS}/${name}.aria.txt`);
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
  }
  // `crop` photographs one element instead of the viewport; the snapshot is
  // still the PAGE's, so the text says what was around the crop as well.
  await (crop ?? page).screenshot({ path: `${SHOTS}/${name}.png` });
  const aria = await page.locator("body").ariaSnapshot();
  if (aria.trim() === "") throw new Error(`shot: the accessibility snapshot for ${name} is empty`);
  writeFileSync(`${SHOTS}/${name}.aria.txt`, `${aria}\n`);
}

const raf = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

/** Point the camera at the top: the WINDOW and the table's own scroller.
 *
 *  **Above 1201px the page does not scroll and `.table` does** (`styles.css`:
 *  `.app` is `height: 100vh` and the table is its own `overflow: auto` size
 *  container), so `window.scrollTo(0, 0)` — which is all this file used to do —
 *  resets a scroller that was never moved and leaves the table wherever the
 *  last click dragged it into view. Both are reset here, because below 1201px
 *  the page is the scroller and above it the table is.
 *
 *  **Throws rather than returning on a missing element**, the shape
 *  `gates.spec.ts` gives its own `scrollTable`: returning silently is the very
 *  failure the helper exists to prevent, since a rename would then produce a
 *  shot at whatever offset the last click left behind, under a green run. That
 *  helper's other anchors are not copied — every picture here is taken at the
 *  top, and an unused arm is an arm nothing checks. Copied rather than imported,
 *  because importing a `.spec.ts` would register that file's tests into this
 *  run. */
async function scrollToTop(page: Page): Promise<void> {
  const missing = await page.evaluate(() => {
    window.scrollTo(0, 0);
    const table = document.querySelector(".table") as HTMLElement | null;
    if (!table) return ".table";
    table.scrollTop = 0;
    return null;
  });
  if (missing) throw new Error(`scrollToTop: ${missing} is not on the page`);
  await raf(page);
}

/** Scroll the table until the pyramid's LAST ROW sits just under the top edge.
 *
 *  **Keyed on the last row, not on a pixel offset into the section block.** The
 *  fitted pyramid's row height is `--tile-w`-dependent and the block's own top
 *  moves with the game's text wrapping, so "put the sections 140px down" — the
 *  rule `gates.spec.ts` uses for a shot whose subject is only the sections —
 *  leaves the bottom row half-cut here. This shot's subject is the JOIN: the
 *  foot of the pyramid and the head of the first strip in one frame, which is
 *  what the help's sentence about boards under the pyramid claims exists.
 *
 *  Throws naming the missing selector, for the reason `scrollToTop` does. */
async function scrollToStrips(page: Page): Promise<void> {
  const missing = await page.evaluate(() => {
    window.scrollTo(0, 0);
    const table = document.querySelector(".table") as HTMLElement | null;
    if (!table) return ".table";
    // `.board`'s last child is the marks legend, not a row, so the last ROW is
    // taken from the row list rather than with `:last-child`.
    const rows = [...document.querySelectorAll<HTMLElement>(".board-row")];
    const last = rows.at(-1);
    if (!last) return ".board-row";
    if (!document.querySelector(".table-sections")) return ".table-sections";
    const max = table.scrollHeight - table.clientHeight;
    const delta = last.getBoundingClientRect().top - table.getBoundingClientRect().top - 20;
    table.scrollTop = Math.max(0, Math.min(max, table.scrollTop + delta));
    return null;
  });
  if (missing) throw new Error(`scrollToStrips: ${missing} is not on the page`);
  await raf(page);
}

/** The pointer off the board, and the hover popover with it.
 *
 *  `act()` ends on a click, so the pointer is left resting wherever the last
 *  one landed and `TilePopover` opens under it — the V2 gate photographed a
 *  popover sitting over the middle of the table before it learned this. A
 *  tooltip in a help screenshot is a picture of a feature the caption is not
 *  about. Asserted gone rather than assumed. */
async function park(page: Page): Promise<void> {
  await page.mouse.move(2, 2);
  await expect(page.getByRole("tooltip")).toHaveCount(0);
}

/** Switch a board on or off through the banner's `Boards` menu.
 *
 *  Scoped to the menu: "Goals" is also a card-index button in the `Cards`
 *  disclosure two elements along, so an unscoped role lookup is one label away
 *  from being ambiguous. Escape is `App.tsx`'s own handler and not the
 *  browser's — a `<details>` ignores the key natively — so the close is a claim
 *  worth checking, and a menu left open sits over the table in the shot. */
async function toggleBoards(page: Page, ...names: string[]): Promise<void> {
  const boards = page.locator(".boards-menu");
  await page.locator(".boards-menu > summary").click();
  for (const name of names) await boards.getByRole("button", { name }).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".boards-menu[open]")).toHaveCount(0);
}

test("the whole screen, and a placement picker", async ({ page }) => {
  await page.setViewportSize(VIEWPORT);
  await startGame(page, { players: 2 });

  // **The reading that stops the picker check below from being vacuous.**
  //
  // `placement.png` is selected by "some tile carries `role=\"button\"`", and
  // that is only a picker detector for as long as tiles do NOT carry the role
  // the rest of the time. Since M5.5's R-F2 the tag says nothing either way —
  // every tile is a `<div tabIndex={0}>` in both states, because switching the
  // tag remounted the art — and `Board.tsx` adds the role, the label and the
  // handlers only when `App` hands it an `onTileClick`, which it does only at
  // `place_a`/`place_b`. So the property holds at HEAD, and it is a property of
  // two files that a third task could quietly change (the popover, for one,
  // opens on focus from a tile with no role at all). Read here, before a move is played, where the phase machine
  // cannot be at a Placement prompt: if this ever reads non-zero the detector
  // has stopped discriminating and the run fails instead of photographing an
  // arbitrary frame under the placement caption.
  expect(
    await page.locator('.tile[role="button"]').count(),
    'a tile carries role="button" outside a placement picker, so the picker check below detects nothing',
  ).toBe(0);

  await playAFew(page, 9, 0x53_48_4f_54); // "SHOT"

  // Building sites on, so the table has all five boards under the pyramid and
  // the summary reads its own count back — the picture and the help's sentence
  // about the Boards menu are then about the same screen.
  await toggleBoards(page, "Building sites");
  await expect(page.locator(".boards-menu > summary")).toHaveText("Boards · 5 on");

  // **The FIRST VIEW: the table at its own scrollTop 0** (ruling R-T14b). Above
  // 1201px the page does not scroll and the table does, and `act()` leaves the
  // table wherever the last click scrolled it into view — the 1440-era comment
  // here reset only the window, which above this breakpoint is a scroller that
  // never moved. What is in frame is therefore what a player gets on load, and
  // the caption says that rather than naming a strip the fit rule has pushed
  // below the fold.
  await scrollToTop(page);
  await park(page);
  // Anti-vacuity: a screenshot of an empty board would still be a valid PNG.
  await expect(page.locator(".board .tile").first()).toBeVisible();
  await expect(page.locator(".deck")).toBeVisible();
  await expect(page.getByRole("button", { name: "Log", exact: true })).toBeVisible();
  await shot(page, "screen");

  // **The same table, scrolled to what is under the pyramid** (ruling R-T14d).
  // `screen.png` is the first view and stays unfaked, and at 2P/1710x869 the
  // fitted pyramid fills the table's height — the browser walk records 1404px
  // of everything-below-the-pyramid scrolling inside the table — so the help
  // could tell a reader the boards are down there and show them nothing. This
  // is the join: the last row of tiles at the top of the frame, the port strip
  // under it.
  await scrollToStrips(page);
  await park(page);
  await expect(page.locator(".boards-menu > summary")).toHaveText("Boards · 5 on");
  // Both halves of the join, asserted on CONTENT and not on the wrapper:
  // `TableSections` renders its "loading…" placeholder under the very same
  // `table-sec ports` class as the loaded strip, so a check on the section
  // element passes while the section reads "loading…" — a picture of a spinner
  // captioned as a picture of the port. `.board-strip` is mounted only by the
  // loaded branch.
  await expect(page.locator(".board-row").last().locator(".tile").first()).toBeVisible();
  await expect(page.locator(".table-sec.ports .board-strip").first()).toBeVisible();
  await shot(page, "strips");

  // Walk on until a placement picker is open, so the "the board is the input"
  // paragraph has the picture it describes. Bounded, and it says so if it never
  // finds one rather than writing a shot of something else.
  let found = false;
  const pick = rng(0x504c_4143); // "PLAC"
  for (let i = 0; i < 40 && !found; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await page.locator('.tile[role="button"]').count()) > 0) {
      found = true;
      break;
    }
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(found, "never reached a placement picker, so placement.png would be a lie").toBe(true);
  await scrollToTop(page);
  await park(page);
  await expect(page.locator('.tile[role="button"]').first()).toBeVisible();
  // The deck's own words for the picker, so the shot cannot be of a board that
  // merely has buttons on it: Option A and Option B are the only two prompts
  // that put the question on the table, and one of them is on screen. Scoped to
  // `.deck` and not to `.decision`, which the client renders three times — the
  // warnings block is `.decision` too, and it is the FIRST one.
  await expect(page.locator(".deck")).toContainText(/Option [AB] —/);
  await expect(page.getByRole("button", { name: "Confirm" })).toBeVisible();
  await shot(page, "placement");
});

test("the explanation, and the replay track", async ({ page }) => {
  await page.setViewportSize(VIEWPORT);
  await startGame(page, { players: 2 });
  await playAFew(page, 14, 0x4558_504c); // "EXPL"

  await page.getByRole("button", { name: "Suggest", exact: true }).click();
  // The chart, not just the panel: the bars ARE the paragraph's subject.
  await expect(page.locator(".surface .bar").first()).toBeVisible({ timeout: 30_000 });
  await shot(page, "explain");
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await expect(track).toBeVisible();
  await track.focus();
  for (let i = 0; i < 6; i++) await page.keyboard.press("ArrowLeft");
  // The whole point of the shot is the TABLE redrawn at an earlier position
  // beside the drawer, so both have to be on screen before it is taken.
  await expect(page.locator(".table.replaying")).toBeVisible();
  await expect(page.locator(".surface[open]")).toBeVisible();
  // The table is its own scroller above 1201px and the walk has been clicking
  // in it; without this the outlined pyramid is photographed mid-scroll.
  await scrollToTop(page);
  await park(page);
  await shot(page, "replay");
});

// ---- the area-action choosers: the answer bar alone (issue #77) -----------
//
// The help says the Church's and the Port's choosers in words, and the owner
// asked to SEE them beside those words, as the placement picker is seen. These
// two pictures are the answer bar and nothing else — the phase chain, the
// asking line, the chooser, the verdict and Confirm and Cancel — cut from the
// page as an ELEMENT screenshot of the decision panel.
//
// **At `deviceScaleFactor` 2, unlike the five above.** Those are whole screens
// whose bytes are paid by every release. A crop of the side column is a few
// hundred CSS pixels across, and the help's `.shot img` is `max-width: 100%;
// height: auto`, which never upscales: at 1 the crop would be drawn at its own
// 479 or so pixels, its 11px text soft on any high-density screen. At 2 the help
// draws it at the panel's width with pixels to spare, and a crop that small
// costs little at twice the pixels; it is the one place in this file that buys
// density. The viewport is the five's ([`VIEWPORT`]), so the column the crop
// is cut from is the one the owner's screen draws, and a table whose cards that
// column cannot hold whole is dealt again rather than photographed
// ([`CHOOSER_TABLES`]).
//
// **The frame is held to the WORDS beside it, not only to "a chooser".** The
// alts and captions in `Help.tsx` and `docs/web-guide.md` name the question,
// how many kinds or cards are switched on and how many are unavailable, and
// (at the Port) the zones and a single way to pay. [`assertChooserFrame`]
// asserts each of those before a picture is written, so a re-cut that reaches
// a different frame — a different opening, a second way to pay, one card where
// two are claimed — is a miss and is dealt again, never a picture the words
// deny. A change to what the words claim moves these claims with them.

/** The decision panel the client renders for the prompt (`Decision.tsx`), and
 *  not the two `.decision notices` strips the deck draws above and below it. */
const PANEL = ".deck > .decision:not(.notices):not(.walking)";

/** Fresh tables a chooser shot will deal before it gives up. The server draws
 *  the seed, so a table may not serve the frame: it can offer a single payment,
 *  which the list holds as a flat row rather than a chooser, and the order
 *  cards it deals set how tall the Port's cards are, so at 1710x869 some
 *  tables' second zone of cards is cut at the fold of the chooser's own
 *  scroller (measured: one table cut two of four cards, the next cut none). */
const CHOOSER_TABLES = 6;

/** What the words beside a chooser's picture claim is in frame, which the
 *  frame must show before it is photographed ([`assertChooserFrame`]). */
interface ChooserClaims {
  /** The asking line, exactly as the alts quote it. */
  asking: string;
  /** The items the chooser draws: the Church's kinds, the Port's cards. */
  items: string;
  /** How many items it draws in all. */
  total: number;
  /** How many are switched on. */
  pressed: number;
  /** How many are unavailable (`disabled` by the payload). */
  unavailable: number;
  /** The Port's zone headings, in order; none for the Church. */
  zones?: readonly string[];
  /** The verdict names exactly one way to pay (no "asked at Confirm"). */
  oneWay?: boolean;
}

/** The Church's picture's claims (`Help.tsx`'s alt and caption, the guide's
 *  alt): the question; six kinds, two switched on and one unavailable. */
const CHURCH_CLAIMS: ChooserClaims = {
  asking: "The Church — what do you pay?",
  items: "button.kind",
  total: 6,
  pressed: 2,
  unavailable: 1,
};

/** The Port's picture's claims: the question with "or two"; four cards in
 *  reach under LOW and MID, two switched on and the other two unavailable;
 *  one way to pay, so the line names the take and its payment. */
const PORT_CLAIMS: ChooserClaims = {
  asking: "The Port — take one card, or two",
  items: "button.card-pick",
  total: 4,
  pressed: 2,
  unavailable: 2,
  zones: ["LOW", "MID"],
  oneWay: true,
};

/** Reach `area`'s chooser, make a selection whose verdict is legal and check
 *  the frame against `claims` ([`assertChooserFrame`]), on a fresh table each
 *  attempt, up to [`CHOOSER_TABLES`] of them. `select` makes the selection and
 *  returns the verdict it read, or throws saying why it could not. Every miss
 *  is kept and the last one fails the run with all of them, so a picture the
 *  words deny is never written and a run that photographed nothing says why.
 *  On success the frame is the one checked, and the caller photographs it
 *  next. */
async function chooserFrame(
  page: Page,
  area: ChooserArea,
  claims: ChooserClaims,
  select: () => Promise<string>,
): Promise<{ made: string; misses: string[] }> {
  const misses: string[] = [];
  for (let table = 1; table <= CHOOSER_TABLES; table++) {
    try {
      await startGame(page, { players: 2 });
      await openChooser(page, area);
      const made = await select();
      await assertChooserFrame(page, area, claims);
      return { made, misses };
    } catch (e) {
      misses.push(`table ${table}: ${e instanceof Error ? e.message.split("\n").slice(0, 6).join(" / ") : String(e)}`);
    }
  }
  throw new Error(`no ${area} chooser in the frame its words claim on ${CHOOSER_TABLES} fresh tables, so no picture was written:\n${misses.join("\n")}`);
}

/** The verdict line's text, or "" when it is silent. */
const verdictOf = async (page: Page, area: ChooserArea) =>
  ((await page.locator(`${PANEL} .chooser.${CHOOSER_CLASS[area]} .verdict`).textContent()) ?? "").trim();

/** What must be true of the frame before a chooser is photographed, asserted:
 *  everything `claims` says (the asking line, the items drawn, switched on and
 *  unavailable, the Port's zones and its single way to pay), the verdict
 *  legal, Confirm live and Cancel there, and the crop clean — inside the window
 *  and the deck (a panel the deck clipped would photograph what lies under the
 *  clip), every item whole inside each box that clips it and inside the crop,
 *  Confirm and Cancel inside the crop, no popover, and nothing painted over
 *  it: a hit test at the panel's centre and four inset corners lands inside
 *  the panel each time, since an element screenshot photographs whatever is
 *  painted in the element's box. */
async function assertChooserFrame(page: Page, area: ChooserArea, claims: ChooserClaims): Promise<void> {
  const panel = page.locator(PANEL);
  const chooser = page.locator(`${PANEL} .chooser.${CHOOSER_CLASS[area]}`);
  await expect(chooser, `the ${area} chooser is not open`).toHaveCount(1);
  await expect(panel.locator(".asking"), `the ${area} asking line`).toHaveText(claims.asking);
  await expect(chooser.locator(claims.items), `the ${area} chooser's items`).toHaveCount(claims.total);
  await expect(chooser.locator(`${claims.items}[aria-pressed="true"]`), `the ${area} items switched on`).toHaveCount(claims.pressed);
  await expect(chooser.locator(`${claims.items}[disabled]`), `the ${area} items unavailable`).toHaveCount(claims.unavailable);
  if (claims.zones) {
    await expect(chooser.locator(".zone > .zl"), `the ${area} zones`).toHaveText([...claims.zones]);
  }
  expect(await verdictOf(page, area), `the ${area} verdict is not legal`).toMatch(new RegExp(`^${LEGAL}`));
  if (claims.oneWay) {
    await expect(chooser.locator(".verdict .asked"), `the ${area} verdict asks at Confirm`).toHaveCount(0);
    await expect(chooser.locator(".verdict .way"), `the ${area} verdict's ways`).toHaveCount(1);
  }
  const confirm = panel.getByRole("button", { name: "Confirm", exact: true });
  await expect(confirm, "Confirm is not live").toBeEnabled();
  await expect(panel.getByRole("button", { name: "Cancel", exact: true })).toBeVisible();
  await park(page);
  const cut = await page.evaluate(
    ({ panel, items }) => {
      const r = (el: Element) => el.getBoundingClientRect();
      const inside = (a: DOMRect, b: DOMRect) =>
        a.left >= b.left - 0.5 && a.top >= b.top - 0.5 && a.right <= b.right + 0.5 && a.bottom <= b.bottom + 0.5;
      const crop = document.querySelector(panel)!;
      const box = r(crop);
      const view = new DOMRect(0, 0, window.innerWidth, window.innerHeight);
      const faults: string[] = [];
      if (!inside(box, view)) faults.push(`the panel ${box.width}x${box.height} at ${box.left},${box.top} is not inside the window`);
      if (!inside(box, r(crop.closest(".deck")!))) faults.push("the panel is not inside the deck");
      // Nothing painted over the panel: what the screen shows at its centre and
      // 4px in from each corner is the panel or something inside it.
      const inset = 4;
      for (const [name, x, y] of [
        ["centre", (box.left + box.right) / 2, (box.top + box.bottom) / 2],
        ["top left", box.left + inset, box.top + inset],
        ["top right", box.right - inset, box.top + inset],
        ["bottom left", box.left + inset, box.bottom - inset],
        ["bottom right", box.right - inset, box.bottom - inset],
      ] as const) {
        const hit = document.elementFromPoint(x, y);
        if (!hit || !crop.contains(hit)) {
          faults.push(`at the panel's ${name} the screen shows ${hit ? `${hit.tagName}.${hit.className}` : "nothing"}, not the panel`);
        }
      }
      const named = (name: string) => [...crop.querySelectorAll("button")].find((b) => b.textContent?.trim() === name);
      for (const name of ["Confirm", "Cancel"]) {
        const b = named(name);
        if (!b || !inside(r(b), box)) faults.push(`${name} is not inside the panel`);
      }
      const all = [...crop.querySelectorAll(items)];
      if (all.length === 0) faults.push(`the chooser draws no ${items}`);
      for (const el of all) {
        const at = r(el);
        const what = el.getAttribute("data-good") ?? el.getAttribute("data-number") ?? el.textContent?.trim().slice(0, 20);
        if (!inside(at, box)) faults.push(`${what} is not inside the panel`);
        for (let up = el.parentElement; up && up !== crop.parentElement; up = up.parentElement) {
          const s = getComputedStyle(up);
          if ((s.overflowX !== "visible" || s.overflowY !== "visible") && !inside(at, r(up))) {
            faults.push(`${what} is cut by ${up.className || up.tagName}, which shows ${up.clientHeight} of ${up.scrollHeight} px at a ${window.innerWidth}x${window.innerHeight} window`);
            break;
          }
        }
      }
      return faults;
    },
    { panel: PANEL, items: claims.items },
  );
  expect(cut, `the ${area} crop is not clean`).toEqual([]);
}

test.describe("the area-action choosers", () => {
  test.use({ deviceScaleFactor: 2 });

  test("the Church's chooser, with a legal payment of two kinds switched on", async ({ page }) => {
    await page.setViewportSize(VIEWPORT);
    const { items } = CHURCH_CLAIMS;
    const { made, misses } = await chooserFrame(page, "Church", CHURCH_CLAIMS, async () => {
      // Kinds on, one at a time, the shape `the_table.spec.ts`'s Church payment
      // takes, until two are on and the verdict reads legal: the words beside
      // the picture claim a payment of two kinds. Anything else is a miss.
      const on = page.locator(`${PANEL} .chooser.church ${items}[aria-pressed="true"]`);
      for (let press = 0; press < 2; press++) {
        const kind = page.locator(`${PANEL} .chooser.church ${items}:not([disabled])[aria-pressed="false"]`).first();
        if ((await kind.count()) === 0) throw new Error(`no kind left to switch on with ${await on.count()} on`);
        await kind.click();
        await readScreen(page);
      }
      const verdict = await verdictOf(page, "Church");
      if (!verdict.startsWith(LEGAL)) throw new Error(`two kinds on and the verdict is not legal: "${verdict}"`);
      return verdict;
    });
    // eslint-disable-next-line no-console
    console.log(`church: two kinds on, "${made}", after ${misses.length} missed table(s)${misses.map((m) => `\n  ${m}`).join("")} (recorded)`);
    await shot(page, "church", page.locator(PANEL));
  });

  test("the Port's chooser, with a legal pair switched on", async ({ page }) => {
    await page.setViewportSize(VIEWPORT);
    const { items } = PORT_CLAIMS;
    const live = `${PANEL} .chooser.port ${items}:not([disabled]):not([aria-disabled="true"])[aria-pressed="false"]`;
    const { made, misses } = await chooserFrame(page, "Port", PORT_CLAIMS, async () => {
      // The first card on, then a second the arm can add: the words beside the
      // picture claim a pair. A table whose arm can add no second card is a
      // miss.
      for (const which of ["first", "second"]) {
        const card = page.locator(live).first();
        if ((await card.count()) === 0) throw new Error(`no ${which} card in reach can be switched on`);
        await card.click();
        await readScreen(page);
      }
      const verdict = await verdictOf(page, "Port");
      if (!verdict.startsWith(LEGAL)) throw new Error(`the pair's verdict is not legal: "${verdict}"`);
      return verdict;
    });
    // eslint-disable-next-line no-console
    console.log(`port: a pair on, "${made}", after ${misses.length} missed table(s)${misses.map((m) => `\n  ${m}`).join("")} (recorded)`);
    await shot(page, "port", page.locator(PANEL));
  });
});
