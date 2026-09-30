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

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import type { Page } from "@playwright/test";
// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import { expect, test } from "./sw";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { DESKTOP, look, noSidewaysScroll, noticeIsDrawn, PHONE, signIn, startGame, WALKER, watchForBreakage } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The label the build ships for its bot (`production_bot.json`), read the way
 *  this directory reads a committed file (`offline.spec.ts`): the package is
 *  an ES module, where a bare JSON `import` needs an import attribute. */
const PRODUCTION_LABEL = (
  JSON.parse(
    fs.readFileSync(
      path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "src", "wire", "production_bot.json"),
      "utf8",
    ),
  ) as { label: string }
).label;

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
    await noticeIsDrawn(page, `the door at ${size.width}px`);
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
      // **The notice sits at the foot of the window** (cycle E; Task 3's review
      // M-9): under the door's short content at 1440x900 it floated at 609px
      // with ~260px of empty page below it. The door keeps the window's height
      // less the footer's, as the lobby does, so the footer ends the window.
      const foot = await page.evaluate(() => ({
        bottom: document.querySelector("footer.site-notice")!.getBoundingClientRect().bottom,
        window: window.innerHeight,
        page: document.documentElement.scrollHeight,
      }));
      expect(Math.abs(foot.bottom - foot.window), `the footer ends the window: ${JSON.stringify(foot)}`).toBeLessThanOrEqual(0.5);
      expect(foot.page, `and the page is the window: ${JSON.stringify(foot)}`).toBe(foot.window);
    } else {
      expect(new Set(boxes.map((b) => b.left)).size, `one column of cards: ${JSON.stringify(boxes)}`).toBe(1);
      expect(boxes.map((b) => b.top)).toEqual([...boxes.map((b) => b.top)].sort((a, b) => a - b));
      expect(board.y + board.height, "the board sits above the title").toBeLessThanOrEqual(heading.y + 1);
      expect(await height(page, "button", "New game"), "New game, thumb-sized").toBeGreaterThanOrEqual(44);
      expect(await height(page, "link", "Go to the lobby"), "Go to the lobby, thumb-sized").toBeGreaterThanOrEqual(44);
      // The door's quiet control too (M6.5c2 cycle C, polish 2): it drew 17px.
      expect(await height(page, "button", "How to play"), "How to play, thumb-sized").toBeGreaterThanOrEqual(44);
      // And the help's Close (the task review's M-2), read open; Close hands
      // focus back to How to play, in Chromium as in jsdom (the a11y pass).
      await page.getByRole("button", { name: "How to play" }).click();
      expect(await height(page, "button", "Close"), "the help's Close, thumb-sized").toBeGreaterThanOrEqual(44);
      await page.getByRole("button", { name: "Close" }).click();
      await expect(page.getByRole("button", { name: "How to play" })).toBeFocused();
    }
    await look(page, `door-${size.width}`);
  }

  // **Between the phone and the desktop the title never runs into the board**
  // (Task 3 fix round 1; the review's I-1 measured the word drawn over the
  // pyramid from 820px down to 601px). One width either side of the hero's
  // 860px stack: at 861 the row, the text left of the board; at 700 the stack,
  // the board above it. The TEXT is measured with a `Range`, because the
  // heading's own box could shrink below its word — which is how the defect
  // passed every width read — and the two boxes must not intersect.
  for (const width of [861, 700]) {
    await page.setViewportSize({ width, height: 900 });
    await page.goto("/");
    await expect(page.locator(".door-hero").getByRole("heading", { level: 1, name: "Yokohama" })).toBeVisible();
    // The blurb is DRAWN (anti-vacuity: a dist built without it would pass
    // this loop over a title that never had company), and its own text is
    // measured against the board as the title's is.
    await expect(page.locator(".door-hero .blurb"), `at ${width}px the blurb is drawn`).toBeVisible();
    const { text, board, blurb } = await page.evaluate(async () => {
      await document.fonts.ready;
      const range = document.createRange();
      range.selectNodeContents(document.querySelector(".door-hero h1")!);
      const box = (r: DOMRect) => ({ left: r.left, right: r.right, top: r.top, bottom: r.bottom });
      return {
        text: box(range.getBoundingClientRect()),
        blurb: (() => {
          const r = document.createRange();
          r.selectNodeContents(document.querySelector(".door-hero .blurb")!);
          return box(r.getBoundingClientRect());
        })(),
        board: box(document.querySelector(".door-hero .board-mark")!.getBoundingClientRect()),
      };
    });
    // **The text has a width before anything is compared with it** (the
    // whole-branch review's Minor 4): an empty `Range` reads as a zero rect at
    // the origin, and that rect "ends left of the board" at 861.
    expect(text.right - text.left, `at ${width}px the title's text has a width`).toBeGreaterThan(100);
    const clear =
      text.right <= board.left || text.left >= board.right || text.bottom <= board.top || text.top >= board.bottom;
    expect(clear, `at ${width}px the title's text ${JSON.stringify(text)} meets the board ${JSON.stringify(board)}`).toBe(
      true,
    );
    const blurbClear =
      blurb.right <= board.left || blurb.left >= board.right || blurb.bottom <= board.top || blurb.top >= board.bottom;
    expect(blurbClear, `at ${width}px the blurb ${JSON.stringify(blurb)} meets the board ${JSON.stringify(board)}`).toBe(
      true,
    );
    if (width === 861) {
      expect(text.right, "at 861px the hero is a row: the text ends left of the board").toBeLessThanOrEqual(board.left);
    } else {
      expect(text.top, "at 700px the hero is stacked: the board sits above the text").toBeGreaterThanOrEqual(board.bottom);
      await look(page, "door-700");
    }
    await noSidewaysScroll(page, `the door at ${width}px`);
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
  // The anti-vacuity half (the review's M-6): the listener DID see this page's
  // requests — the art manifest is a static file, fetched beside the pictures
  // it names — so the empty list above is a door that asked nothing, and not a
  // listener that heard nothing.
  expect(asked, "and the manifest is a file, not a route").toContain("/art/manifest.json");

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

test("the name step at 390: one field, Continue, the honest sentence, nothing sideways — and a name typed with capitals and spaces signs in under its key", async ({
  page,
}) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize(PHONE);
  await page.goto("/#/lobby");
  // An `h3`: the band's last crumb, "Lobby", is the page's `h2` (`Band.tsx`).
  await expect(page.getByRole("heading", { level: 3, name: "Choose a name" })).toBeVisible();
  await expect(page.getByText(/doesn.t check who anyone is/)).toBeVisible();
  await noSidewaysScroll(page, "the name step at 390px");
  expect(await height(page, "button", "Continue"), "Continue, thumb-sized").toBeGreaterThanOrEqual(44);
  expect(await images(page), "the name step draws no <img>").toBe(0);
  await look(page, "name-step-390");

  // **Review Focus 1 against the real server**: one key, shown as typed. The
  // header carries the key; `/api/me` answers it; the corner shows the name.
  await page.getByLabel("Name", { exact: true }).fill("  Ben   Castle ");
  await page.getByRole("button", { name: "Continue" }).click();
  const pill = page.locator(".identity-pill");
  await expect(pill).toBeVisible();
  await expect(pill).toContainText("Ben Castle");
  await expect(page.locator(".identity-corner").getByTitle("ben castle", { exact: true })).toBeAttached();
  expect(await page.evaluate(() => localStorage.getItem("yokohama.claimed_user"))).toBe("ben castle");
  await noSidewaysScroll(page, "the signed-in lobby at 390px");
  await look(page, "lobby-signed-in-390");
  expect(watch.failures).toEqual([]);
});

// **The lobby's lists** (M6.5c2, R-6.5c2.7; mocks 02 and 11): three tables
// opened over the API, one locked, one against the bot, one somebody else's,
// read at both widths. The rows' words are `tables.ts`'s and jsdom pins them;
// what only a browser can say is the layout: name, seats and action on one line
// at desktop width, the rail beside the lists and then below them, the corner a
// pill on the phone, and the controls thumb-sized.
test("the lobby at 1440 and at 390: rows of name, seats and one action; the chat beside the lists, then below them; the corner a pill on the phone", async ({
  page,
  request,
}) => {
  const watch = watchForBreakage(page);
  const as = (who: string) => ({ "x-yokohama-claimed-user": who });
  /** A table, opened over the API as `who` (the open door reads the header;
   *  `it_explains.spec.ts`'s shape), and its id. */
  const create = async (who: string, data: Record<string, unknown>): Promise<string> => {
    const res = await request.post("/api/games", { headers: as(who), data });
    expect(res.status(), await res.text()).toBe(201);
    return String(((await res.json()) as { id: string }).id);
  };
  const OTHER = "someone@e2e.test";
  const opened: [string, string][] = [];
  try {
    opened.push([
      await create(WALKER, {
        players: 2,
        colors: ["Red", "Blue"],
        start_seat: 0,
        seats: [{ user: WALKER }, { open: true }],
        name: "Width check",
        password: "pw",
      }),
      WALKER,
    ]);
    opened.push([
      await create(WALKER, {
        players: 2,
        colors: ["Red", "Blue"],
        start_seat: 0,
        seats: [{ user: WALKER }, { bot: true }],
        name: "Against the bot",
      }),
      WALKER,
    ]);
    opened.push([
      await create(OTHER, {
        players: 3,
        colors: ["Red", "Blue", "Yellow"],
        start_seat: 0,
        seats: [{ user: OTHER }, { open: true }, { open: true }],
        name: "Someone else's",
      }),
      OTHER,
    ]);
    await signIn(page, WALKER);
    for (const size of [DESKTOP, PHONE]) {
      await page.setViewportSize(size);
      await page.goto("/#/lobby");
      const mine = page.locator(".lobby section.your-games");
      const rowOf = (name: string) =>
        mine.locator("li.lobby-row").filter({ has: page.getByRole("link", { name, exact: true }) });
      await expect(rowOf("Width check")).toBeVisible();
      await expect(rowOf("Width check").getByRole("img", { name: "password required" })).toBeVisible();
      // The bot's version, from the table's own `TableView.bot` (an undealt
      // table shows the production bot, spec section 2.4).
      await expect(rowOf("Against the bot")).toContainText(`the bot \u00b7 ${PRODUCTION_LABEL}`);
      await expect(
        page.locator(".lobby section.open-tables").getByRole("button", { name: "Take a seat: Someone else's" }),
      ).toBeVisible();
      await noSidewaysScroll(page, `the lobby at ${size.width}px`);
      expect(await images(page), `the lobby draws no <img> at ${size.width}px`).toBe(0);

      /** The rows this cell opened, by name (the run's server is shared). */
      const OURS_AT_390 = ["Width check", "Against the bot", "Someone else's"];
      const lists = (await page.locator(".lobby .lobby-lists").boundingBox())!;
      const rail = (await page.locator(".lobby section.lobby-chat").boundingBox())!;
      const pill = page.locator(".identity-pill");
      if (size === DESKTOP) {
        expect(rail.x, "the chat sits in a rail beside the lists").toBeGreaterThanOrEqual(lists.x + lists.width - 1);
        await expect(pill).toBeHidden();
        // The MENU's words: the pill says them too, unseen (cycle C), and a
        // bare corner-wide read would find two.
        await expect(page.locator(".identity-corner .identity-menu").getByText("Playing as")).toBeVisible();
        // **Name, seats and action on ONE line, on EVERY row**, measured as the
        // elements a player reads: the title block, each chip, and the action
        // itself (the button, or the "can't continue here" words). Never the
        // grid cells, which `align-items: center` centres by construction
        // whatever they hold: measured that way this read level while the
        // action button of every creator row sat 16px above the line (Task 5
        // review, I-1). The creator's small Abandon hangs under the action and
        // is not on the line.
        // **Only the rows THIS cell opened**: the run's server is shared, so a
        // sibling spec's table listed beside these is not this cell's to
        // measure, and its layout must not fail this cell.
        const OURS = ["Width check", "Against the bot", "Someone else's"];
        const lines = await page.locator(".lobby li.lobby-row").evaluateAll((all, ours) => {
          const rows = all.filter((li) => ours.includes(li.querySelector("a.row-link")?.textContent ?? ""));
          return rows.map((li) => {
            const mid = (e: Element) => {
              const r = e.getBoundingClientRect();
              return Math.round(r.top + r.height / 2);
            };
            const middles = [
              mid(li.querySelector(".row-title")!),
              ...[...li.querySelectorAll(".row-seats .seat-chip")].map(mid),
              // The action itself: since cycle E a copy row's first child is
              // its "Copied" status (row 68), empty and not the action.
              mid(li.querySelector(".row-action > button, .row-action > .stranded")!),
            ];
            return {
              name: li.querySelector("a.row-link")!.textContent,
              abandon: li.querySelector('button[aria-label^="abandon "]') !== null,
              middles,
              spread: Math.max(...middles) - Math.min(...middles),
            };
          });
        }, OURS);
        expect(lines.map((l) => l.name).sort(), "the sweep read exactly the rows this cell opened").toEqual(
          [...OURS].sort(),
        );
        // eslint-disable-next-line no-console
        console.log(`lobby rows at 1440 (title, chips..., action middles): ${JSON.stringify(lines)}`);
        // Anti-vacuity: rows WITH the creator's Abandon and rows without it.
        expect(lines.filter((l) => l.abandon).length, JSON.stringify(lines)).toBeGreaterThan(0);
        expect(lines.filter((l) => !l.abandon).length, JSON.stringify(lines)).toBeGreaterThan(0);
        const off = lines.filter((l) => l.spread > 12);
        expect(off, `rows whose action, chips and title are not on one line: ${JSON.stringify(off)}`).toEqual([]);
      } else {
        expect(rail.y, "the chat sits below the lists on a phone").toBeGreaterThanOrEqual(lists.y + lists.height - 1);
        await expect(pill).toBeVisible();
        // Named for what it is, in Chromium's own reading (cycle C, the a11y
        // pass): its face is only the name and a caret.
        await expect(pill).toHaveAccessibleName(`Playing as ${WALKER}`);
        await expect(page.getByRole("button", { name: "switch player" })).toBeHidden();
        await pill.click();
        await expect(page.getByRole("button", { name: "switch player" })).toBeVisible();
        await noSidewaysScroll(page, "the lobby at 390px with the corner open");
        // **Every control a thumb presses here is 44px tall** (M6.5c2 cycle C,
        // polish 2): the quiet ones — the way back to this device, a row's
        // name and its Abandon, the pill and its action — drew 18-36px beside
        // primaries at 44, and the lobby chat's show and send (the task
        // review's M-2) ~18px by eye — a figure this cell does not measure. Read with the corner OPEN and the chat SHOWN, so
        // the corner's action and the chat's send are drawn; a control with no
        // box (a shut fold's rows) is one nobody can press, and is left out
        // rather than read as zero. The listings' "more" is drawn only past a
        // page of tables, which this cell does not make: its rule is styled,
        // not read here.
        await page.locator(".lobby-chat").getByRole("button", { name: "show" }).click();
        await expect(page.locator(".lobby-chat").getByRole("button", { name: "send" })).toBeVisible();
        const controls = await page
          .locator(".lobby-actions, .lobby-rows, .identity-corner, .lobby-chat")
          .evaluateAll((scopes) =>
            scopes.flatMap((scope) =>
              [...scope.querySelectorAll("button, a")]
                .filter((e) => e.getClientRects().length > 0)
                .map((e) => ({
                  name: e.getAttribute("aria-label") ?? (e.textContent ?? "").trim(),
                  // A row's name is its ONE link, and on a phone the link's
                  // box covers the row (row 64): what a thumb presses is the
                  // row, so that is the height read for it — and the hit test
                  // below is what makes this substitution true, not assumed.
                  height: (e.matches(".lobby-row a.row-link") ? e.closest("li")! : e).getBoundingClientRect().height,
                })),
            ),
          );
        // eslint-disable-next-line no-console
        console.log(`lobby controls at 390 (name, height): ${JSON.stringify(controls)}`);
        // Anti-vacuity: the sweep saw each kind it is about.
        const names = controls.map((c) => c.name);
        for (const want of ["New table", "switch player", "abandon Width check", "Copy link: Width check", "hide", "send"]) {
          expect(names, `the sweep read ${want}`).toContain(want);
        }
        expect(names.some((n) => n.endsWith("Games on this device")), "the sweep read the way back").toBe(true);
        expect(names.some((n) => n === "Width check"), "the sweep read a row's name").toBe(true);
        const short = controls.filter((c) => c.height < 44);
        expect(short, `controls under 44px at 390: ${JSON.stringify(short)}`).toEqual([]);
        // Fold the chat back: its fold is remembered per browser (`chatFold.ts`).
        await page.locator(".lobby-chat").getByRole("button", { name: "hide" }).click();
        await pill.click();
        expect(await height(page, "button", "Copy link: Width check"), "a row's action, thumb-sized").toBeGreaterThanOrEqual(44);
        expect(await height(page, "button", "New table"), "New table, thumb-sized").toBeGreaterThanOrEqual(44);
        // **The row is the hit box on a phone** (cycle E, straw poll row 64):
        // a creator's row stood ~146px tall at 390 because its name, its action
        // and its Abandon each padded to 44px, stacked. The name's link now
        // covers the row, so the row's own corners — clear of its two buttons
        // — press the name's link, and the creator's Abandon is a 44px square
        // at the row's end, on the action's line.
        for (const name of OURS_AT_390) {
          const row = page.locator(".lobby li.lobby-row").filter({ has: page.getByRole("link", { name, exact: true }) });
          await row.scrollIntoViewIfNeeded();
          const corners = await row.evaluate((li) => {
            const r = li.getBoundingClientRect();
            const link = li.querySelector("a.row-link");
            const at = (x: number, y: number) => document.elementFromPoint(x, y)?.closest("a.row-link") ?? null;
            return [at(r.left + 5, r.top + 5), at(r.left + 5, r.bottom - 5)].map((hit) => hit === link);
          });
          expect(corners, `${name}: the row's corners press its link`).toEqual([true, true]);
        }
        for (const name of ["Width check", "Against the bot"]) {
          const row = rowOf(name);
          const li = (await row.boundingBox())!;
          const abandon = (await row.getByRole("button", { name: `abandon ${name}`, exact: true }).boundingBox())!;
          const action = (await row.locator(".row-action button").first().boundingBox())!;
          const boxes = JSON.stringify({ li, abandon, action });
          expect(Math.abs(abandon.width - 44), `${name}'s Abandon is 44px wide: ${boxes}`).toBeLessThanOrEqual(0.5);
          expect(Math.abs(abandon.height - 44), `${name}'s Abandon is 44px tall: ${boxes}`).toBeLessThanOrEqual(0.5);
          expect(
            Math.abs(abandon.y + abandon.height / 2 - (action.y + action.height / 2)),
            `${name}'s Abandon sits on the action's line: ${boxes}`,
          ).toBeLessThanOrEqual(1);
          expect(abandon.x, `${name}'s Abandon follows the action: ${boxes}`).toBeGreaterThanOrEqual(action.x + action.width);
          expect(li.x + li.width - (abandon.x + abandon.width), `${name}'s Abandon ends the row: ${boxes}`).toBeLessThanOrEqual(16);
        }
        // Measured before: 148px. A creator's row whose name takes one line is
        // now its action's line and its seats' line, and no taller.
        const againstTheBot = (await rowOf("Against the bot").boundingBox())!.height;
        // eslint-disable-next-line no-console
        console.log(`a creator's row at 390 (Against the bot): ${againstTheBot}px`);
        expect(againstTheBot, "a creator's one-line row at 390").toBeLessThan(112);
      }
      await look(page, `lobby-${size.width}`);
      if (size === PHONE) {
        // **The creator's Abandon at its confirm step, on a phone** (Task 5
        // review: unverified at 390). Its two controls stay inside the row
        // and the window, and cancelling leaves the table where it was.
        const row = rowOf("Width check");
        await row.getByRole("button", { name: "abandon Width check" }).click();
        const confirm = row.getByRole("button", { name: "confirm abandon Width check" });
        const cancel = row.getByRole("button", { name: "cancel abandoning Width check" });
        await expect(confirm).toBeVisible();
        const li = (await row.boundingBox())!;
        for (const control of [confirm, cancel]) {
          const box = (await control.boundingBox())!;
          expect(
            box.x >= li.x && box.x + box.width <= li.x + li.width && box.y + box.height <= li.y + li.height,
            `the confirm step's controls stay inside their row: ${JSON.stringify({ box, li })}`,
          ).toBe(true);
        }
        await noSidewaysScroll(page, "the lobby at 390px with an abandon asked");
        await look(page, "lobby-390-abandon-confirm");
        await cancel.click();
        await expect(row.getByRole("button", { name: "abandon Width check" })).toBeVisible();
      }
    }
  } finally {
    // The run's server is scratch, but a later spec's lobby should not list
    // this cell's tables.
    for (const [id, who] of opened) await request.delete(`/api/games/${id}`, { headers: as(who) });
  }
  expect(watch.failures).toEqual([]);
});

/** Every visible button, link and file field under `scope`, by name and
 *  rendered height: the lobby sweep's reading, for the two screens that sweep
 *  does not reach. A control with no box (a shut fold's rows) is one nobody can
 *  press, and is left out rather than read as zero. */
async function controlsUnder(page: Page, scope: string): Promise<{ name: string; height: number }[]> {
  return page.locator(scope).evaluateAll((scopes) =>
    scopes.flatMap((root) =>
      [...root.querySelectorAll("button, a, .restore-field")]
        .filter((e) => e.getClientRects().length > 0)
        .map((e) => ({
          name: e.getAttribute("aria-label") ?? (e.textContent ?? "").trim(),
          // A device row's name is the ONE link, and its box covers the whole
          // row (`styles.css`): what a thumb presses is the row, so that is
          // the height read for it.
          height: (e.matches("a.device-name") ? e.closest("li")! : e).getBoundingClientRect().height,
        })),
    ),
  );
}

// **The 44px rule on the two screens the lobby's sweep does not reach** (M6.5c2
// cycle C, polish 2; the cycle's close left "the one-off log witnesses it"): the
// OPEN TABLE (Copy link, Abandon, the chat's show and send, a joiner's seat)
// and the DEVICE CARD on the door (its rows, Back up, Restore). jsdom loads no
// stylesheet, so a px rule can be held by nothing but a browser.
test("the open table and the device card at 390: every control a thumb presses is 44px tall", async ({
  page,
  request,
}) => {
  const watch = watchForBreakage(page);
  const OTHER = "someone@e2e.test";
  const res = await request.post("/api/games", {
    headers: { "x-yokohama-claimed-user": WALKER },
    data: {
      players: 2,
      colors: ["Red", "Blue"],
      start_seat: 0,
      seats: [{ user: WALKER }, { open: true }],
      name: "Thumb check",
    },
  });
  expect(res.status(), await res.text()).toBe(201);
  const id = String(((await res.json()) as { id: string }).id);
  try {
    // The name step is signed through at desktop width, where the corner shows
    // the identity `signIn` waits for; the phone is the reading that follows.
    const asAt390 = async (who: string) => {
      await page.setViewportSize(DESKTOP);
      // A different person in the same browser: forget the claimed name and
      // load the page afresh, since a hash-only navigation reloads nothing.
      await page.goto("/");
      await page.evaluate(() => localStorage.removeItem("yokohama.claimed_user"));
      await page.goto("about:blank");
      await signIn(page, who, `/#/t/${id}`);
      await page.setViewportSize(PHONE);
    };

    // ---- The open table, as the creator (Copy link, Abandon, the chat).
    await asAt390(WALKER);
    await expect(page.locator(".open-table").getByRole("heading", { name: "Thumb check" })).toBeVisible();
    // **The band on a phone: the product and the pill share a line, and the
    // path to the page takes the line under both** (cycle E, straw poll row
    // 60). Two crumbs are the case that broke: "Yokohama / Lobby" beside the
    // pill and "/ Thumb check" at its foot, measured at 390.
    const band = await page.evaluate(() => {
      const box = (e: Element | null) => {
        const r = e!.getBoundingClientRect();
        return { top: Math.round(r.top), bottom: Math.round(r.bottom), left: Math.round(r.left), right: Math.round(r.right) };
      };
      return {
        title: box(document.querySelector(".lobby-band h1")),
        pill: box(document.querySelector(".lobby-band .identity-pill")),
        crumbs: [...document.querySelectorAll(".lobby-band .crumb")].map((c) => ({ text: c.textContent, ...box(c) })),
      };
    });
    // eslint-disable-next-line no-console
    console.log(`the band at 390: ${JSON.stringify(band)}`);
    expect(band.crumbs.map((c) => c.text), "the sweep read both crumbs").toEqual(["Lobby", "Thumb check"]);
    expect(band.pill.top < band.title.bottom && band.pill.bottom > band.title.top, "the pill is on the title's line").toBe(true);
    for (const c of band.crumbs) {
      expect(c.top, `"${c.text}" is under the pill, never beside it`).toBeGreaterThanOrEqual(band.pill.bottom - 1);
      expect(c.top, `"${c.text}" is under the title`).toBeGreaterThanOrEqual(band.title.bottom - 1);
    }
    const show = page.locator(".open-table > .chat").getByRole("button", { name: "show" });
    if (await show.count()) await show.click();
    await expect(page.locator(".open-table > .chat").getByRole("button", { name: "send" })).toBeVisible();
    const creator = await controlsUnder(page, ".open-table");
    // eslint-disable-next-line no-console
    console.log(`open table (creator) controls at 390 (name, height): ${JSON.stringify(creator)}`);
    // Anti-vacuity: the sweep saw the kinds the rules name.
    const seen = creator.map((c) => c.name);
    expect(seen.some((n) => /^Copy link/.test(n)), `the sweep read Copy link: ${seen}`).toBe(true);
    expect(seen, "the sweep read the chat's send").toContain("send");
    expect(seen.some((n) => /abandon/i.test(n)), `the sweep read Abandon: ${seen}`).toBe(true);
    const shortCreator = creator.filter((c) => c.height < 44);
    expect(shortCreator, `open-table controls under 44px at 390: ${JSON.stringify(shortCreator)}`).toEqual([]);
    await noSidewaysScroll(page, "the open table at 390px");

    // ---- The same table as a joiner: the seat's own button.
    await asAt390(OTHER);
    await expect(page.locator(".open-table").getByRole("button", { name: /^Join as/ })).toBeVisible();
    const joiner = await controlsUnder(page, ".open-table");
    expect(
      joiner.some((c) => /^Join as/.test(c.name)),
      `the sweep read the joiner's button: ${JSON.stringify(joiner)}`,
    ).toBe(true);
    const shortJoiner = joiner.filter((c) => c.height < 44);
    expect(shortJoiner, `joiner controls under 44px at 390: ${JSON.stringify(shortJoiner)}`).toEqual([]);

    // ---- The device card, with a game on it so its rows and foot are drawn.
    await page.goto("/");
    const game = await startGame(page, { local: true, signedOut: true });
    await page.goto("/");
    await expect(page.locator(".on-this-device").getByRole("link", { name: game.id, exact: true })).toBeVisible();
    const device = await controlsUnder(page, ".on-this-device");
    // eslint-disable-next-line no-console
    console.log(`device card controls at 390 (name, height): ${JSON.stringify(device)}`);
    const names = device.map((c) => c.name);
    expect(names, "the sweep read the game's row").toContain(game.id);
    expect(names.some((n) => /^Back up/.test(n)), `the sweep read Back up: ${names}`).toBe(true);
    expect(names.some((n) => /restore/i.test(n)), `the sweep read the restore field: ${names}`).toBe(true);
    // **The game's ROW is held to 44px too** (cycle E, straw poll's device
    // row; the exemption cycle D's ruling 42 left here is gone). Its link's box
    // covers the row (`styles.css`, `.device-row`), so the row's height is the
    // one read for it — and the row's own corner is pressed on the link, which
    // is what makes reading the row for the link true.
    const deviceRow = page.locator(".on-this-device .device-row").first();
    // In view first: `elementFromPoint` answers nothing outside the viewport.
    await deviceRow.scrollIntoViewIfNeeded();
    const pressed = await deviceRow.evaluate((li) => {
      const r = li.getBoundingClientRect();
      return document.elementFromPoint(r.left + 3, r.bottom - 3)?.closest("a.device-name") === li.querySelector("a.device-name");
    });
    expect(pressed, "the device row's corner presses the game's link").toBe(true);
    const shortDevice = device.filter((c) => c.height < 44);
    expect(shortDevice, `device-card controls under 44px at 390: ${JSON.stringify(shortDevice)}`).toEqual([]);
    await noSidewaysScroll(page, "the device card at 390px");
  } finally {
    await request.delete(`/api/games/${id}`, { headers: { "x-yokohama-claimed-user": WALKER } });
  }
  expect(watch.failures).toEqual([]);
});

// **The open table's chat, in mock 08's rail from 976px** (cycle E, straw poll
// row 67). It sat in the main column at every width, under the invite. From
// 976px a seated caller's page is two columns — the table and the way out on
// the left, the chat on the right at the lobby rail's width, level with the
// page's top and ending where the band's corner ends; below it the chat is
// back under the invite, above the way out, as the page always read.
test("the open table's chat sits in a rail beside the table from 976px, and under the invite below it", async ({
  page,
  request,
}) => {
  const watch = watchForBreakage(page);
  const res = await request.post("/api/games", {
    headers: { "x-yokohama-claimed-user": WALKER },
    data: { players: 2, colors: ["Red", "Blue"], start_seat: 0, seats: [{ user: WALKER }, { open: true }], name: "Rail check" },
  });
  expect(res.status(), await res.text()).toBe(201);
  const id = String(((await res.json()) as { id: string }).id);
  try {
    await page.setViewportSize(DESKTOP);
    await signIn(page, WALKER, `/#/t/${id}`);
    await expect(page.locator(".open-table").getByRole("heading", { name: "Rail check" })).toBeVisible();
    for (const width of [1440, 976, 975]) {
      await page.setViewportSize({ width, height: 900 });
      const r = await page.evaluate(() => {
        const box = (s: string) => {
          const b = document.querySelector(s)!.getBoundingClientRect();
          return { left: Math.round(b.left), right: Math.round(b.right), top: Math.round(b.top), bottom: Math.round(b.bottom) };
        };
        return {
          main: box(".open-table-main"),
          chat: box(".open-table > .chat"),
          invite: box(".invite"),
          controls: box(".open-table-controls"),
          corner: box(".lobby-band .band-corner"),
        };
      });
      // eslint-disable-next-line no-console
      console.log(`the open table at ${width}px: ${JSON.stringify(r)}`);
      if (width >= 976) {
        expect(r.chat.left, `the chat beside the table at ${width}px`).toBeGreaterThanOrEqual(r.main.right);
        expect(Math.abs(r.chat.top - r.main.top), `level with the page's top at ${width}px`).toBeLessThanOrEqual(2);
        expect(Math.abs(r.chat.right - r.corner.right), `ending where the corner ends at ${width}px`).toBeLessThanOrEqual(2);
        expect(r.controls.top, `the way out under the table at ${width}px`).toBeGreaterThanOrEqual(r.invite.bottom);
        expect(r.controls.right, `and in the table's column at ${width}px`).toBeLessThanOrEqual(r.main.right);
      } else {
        expect(r.chat.top, `the chat under the invite at ${width}px`).toBeGreaterThanOrEqual(r.invite.bottom);
        expect(r.controls.top, `the way out under the chat at ${width}px`).toBeGreaterThanOrEqual(r.chat.bottom);
      }
      await noSidewaysScroll(page, `the open table at ${width}px`);
    }
  } finally {
    await request.delete(`/api/games/${id}`, { headers: { "x-yokohama-claimed-user": WALKER } });
  }
  expect(watch.failures).toEqual([]);
});
