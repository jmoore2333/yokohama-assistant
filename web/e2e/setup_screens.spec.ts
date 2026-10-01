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
import { DESKTOP, look, noSidewaysScroll, PHONE, signIn, WALKER, watchForBreakage } from "./harness";

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
    const { text, board } = await page.evaluate(async () => {
      await document.fonts.ready;
      const range = document.createRange();
      range.selectNodeContents(document.querySelector(".door-hero h1")!);
      const box = (r: DOMRect) => ({ left: r.left, right: r.right, top: r.top, bottom: r.bottom });
      return {
        text: box(range.getBoundingClientRect()),
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
        const lines = await page.locator(".lobby li.lobby-row").evaluateAll((rows) =>
          rows.map((li) => {
            const mid = (e: Element) => {
              const r = e.getBoundingClientRect();
              return Math.round(r.top + r.height / 2);
            };
            const middles = [
              mid(li.querySelector(".row-title")!),
              ...[...li.querySelectorAll(".row-seats .seat-chip")].map(mid),
              mid(li.querySelector(".row-action > :first-child")!),
            ];
            return {
              name: li.querySelector("a.row-link")!.textContent,
              abandon: li.querySelector('button[aria-label^="abandon "]') !== null,
              middles,
              spread: Math.max(...middles) - Math.min(...middles),
            };
          }),
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
        // review's M-2) ~18px. Read with the corner OPEN and the chat SHOWN, so
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
                  height: e.getBoundingClientRect().height,
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
