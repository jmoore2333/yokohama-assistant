// M5's browser cells: the things about the table that jsdom cannot see.

import { type Locator, type Page } from "@playwright/test";
// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import { expect, test } from "./sw";
import { mkdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { haveBrowser } from "../playwright.config";
import { pyramidFits, tileWidthFor } from "../src/fit";
import { act, atTheGame, blame, DESKTOP, newGameForm, PHONE, readOverflow, readScreen, rng, startGame, WALKER, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** Four screens, and the fourth is there for the BRANCH the other three miss.
 *
 *  1440x900 is the M3.5 reference, 1710x869 the owner's, 2560x1440 the cap. All
 *  three fit their four rows, so all three take the same branch below and the
 *  `else` was reached by nothing — a half-tested conditional. 1440x600 is a
 *  short laptop viewport where the table is ~516px tall and the height term
 *  works out UNDER the 10rem legibility floor at every player count (~9.7rem
 *  at 2P, less at 3P and 4P, where the same height is divided by wider rows):
 *  the floor wins, the pyramid is taller than the table, and the table has to
 *  scroll it. That is the rule behaving correctly at a size it cannot satisfy,
 *  and it is the only one of the four that exercises the other half of the
 *  cell. */
const VIEWPORTS = [
  [1440, 900],
  [1710, 869],
  [2560, 1440],
  [1440, 600],
] as const;

const settle = (page: Page) =>
  page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));

/** A few moves, so the board being measured has pieces on it.
 *
 *  The fit rule's arithmetic does not depend on them — the pyramid's shape is
 *  the same at move 0 — but the SIZE it produces is only interesting on a board
 *  that is being played, and an empty board is also the one configuration where
 *  a tile drawing nothing would still measure correctly. The pick stream is
 *  seeded so a failure names a reproducible sequence; the GAME seed is the
 *  server's and never reaches this client at all (M6.1), so a failure names the
 *  table and the record on the server is what reproduces it. */
async function playAFew(page: Page, moves: number, seed: number) {
  const pick = rng(seed);
  for (let i = 0; i < moves; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) break;
  }
  // **The last move's walk is settled before anything is measured (M5.5).** A
  // reply that ends the turn plays the bot's, and the walk then holds the table
  // at an earlier position with the answer bar replaced by its strip.
  // `readScreen` skips a running walk, so this is the settle; every OTHER call
  // in the loop above already had it.
  await readScreen(page);
}

for (const players of [2, 3, 4] as const) {
  test(`the pyramid is the size the rule says, and its rows are in view where it says, at ${players}P`, async ({ page }) => {
    const watch = watchForBreakage(page);
    await startGame(page, { players });
    await playAFew(page, 10, players * 1000 + 7);
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await settle(page);
      const read = await page.evaluate(() => {
        const table = document.querySelector(".table")!;
        // Scoped to the board inside the table being measured: `.board-row` is
        // not unique to it (a drawer's board would match too), and a
        // document-wide read would mix another board's rows into `widest`,
        // `rows` and `bottoms` — the three terms the rule is checked against.
        const board = table.querySelector(".board")!;
        const rows = [...board.querySelectorAll(".board-row")];
        return {
          tableW: table.clientWidth,
          tableH: table.clientHeight,
          tileW: board.querySelector(".tile")!.getBoundingClientRect().width,
          widest: Math.max(...rows.map((r) => r.children.length)),
          rows: rows.length,
          bottoms: rows.map((r) => Math.round(r.getBoundingClientRect().bottom)),
          scrollH: table.scrollHeight,
          innerH: window.innerHeight,
        };
      });
      const want = tileWidthFor(read.widest, read.rows, read.tableW, read.tableH);
      expect(
        Math.abs(read.tileW - want),
        `${players}P at ${w}x${h}: rendered ${read.tileW.toFixed(1)}px, the rule says ${want.toFixed(1)}`,
      ).toBeLessThanOrEqual(1);
      // **The branch is named in the reading, and the branch taken is a fact
      // about the VIEWPORT, not about the product.** Three of the four fit and
      // one does not, by construction, so a run in which every line says the
      // same thing is a run whose `VIEWPORTS` list has drifted.
      let branch: string;
      if (pyramidFits(want, read.rows, read.tableH)) {
        for (const b of read.bottoms) {
          expect(b, `${players}P at ${w}x${h}: a row's bottom (${b}) is below the viewport (${read.innerH})`).toBeLessThanOrEqual(read.innerH);
        }
        // RECORDED, never gated (ruling R-T8a). `.table` holds the marks legend
        // and `.board`'s `padding-bottom` under the pyramid, and Task 12's
        // sections will join them, so a scrolling table under four fitted rows
        // is the design — but the figure belongs in the record, because it is
        // how far the owner has to scroll to reach what is under the board.
        branch =
          `rows in view · the legend and the sections under the pyramid scroll ` +
          `by ${read.scrollH - read.tableH}px VERTICALLY — recorded`;
      } else {
        expect(read.scrollH, `${players}P at ${w}x${h}: the rule cannot fit, so the table must scroll`).toBeGreaterThan(read.tableH);
        // "gated" names the ASSERTION just above, and the overflow it is about
        // is the PYRAMID's — a different quantity from the branch above, where
        // the same scroll is the legend and the sections below a pyramid that
        // fits. The two lines print the same arithmetic about different things,
        // which is why each says which.
        branch =
          `the rule cannot fit its rows, so the table scrolls the pyramid ` +
          `itself (${read.scrollH} > ${read.tableH}) — gated`;
      }
      console.log(`${players}P at ${w}x${h}: tile ${(read.tileW / 16).toFixed(2)}rem (rule ${(want / 16).toFixed(2)}rem) · ${branch}`);
    }
    expect(watch.failures).toEqual([]);
  });
}

test("Undo takes the pieces back off the tile", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  // Walk to the first placement picker and place on the first eligible tile.
  const pick = rng(0x554e_444f); // "UNDO"
  let placed: string | null = null;
  for (let i = 0; i < 30 && placed === null; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if (screen.tiles > 0 && screen.confirm) {
      const tile = page.locator('.tile[role="button"]:not([aria-disabled="true"])').first();
      placed = (await tile.getAttribute("aria-label"))!.slice(0, 3);
      await tile.click();
      await page.getByRole("button", { name: "Confirm" }).click();
      break;
    }
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(placed, "never reached a placement picker").not.toBeNull();
  const piecesOn = () =>
    page.evaluate((id) => {
      const tile = [...document.querySelectorAll(".tile")].find(
        (t) => t.querySelector(".tile-id")?.textContent === id,
      )!;
      return tile.querySelector(".pieces")?.getAttribute("aria-label") ?? "";
    }, placed!);
  await expect.poll(piecesOn, { message: "the placement put no pieces on the tile" }).toMatch(/assistant/);
  await page.getByRole("button", { name: "Undo" }).click();
  await expect.poll(piecesOn, { message: "Undo left the pieces on the tile" }).not.toMatch(/assistant/);
  expect(watch.failures).toEqual([]);
});

// **The picker's own Cancel, walked** (M6.5c2 cycle D, straw poll row 30). The
// harness's walk backs out of a picker through Cancel when it must
// (`cancelPicker`), but only when the picker happens to strand it, and a walk
// that REQUIRED the occasional affordance would be a flake wearing a coverage
// badge. So this is the targeted cell: open the first placement picker, press a
// tile, press Cancel, and read that the picker closed, the press was let go and
// nothing was applied.
test("Cancel closes a placement picker with a tile pressed and applies nothing", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  const pick = rng(0x4341_4e43); // "CANC"
  let opened = false;
  for (let i = 0; i < 30 && !opened; i++) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if (screen.tiles > 0 && screen.confirm) {
      opened = true;
      break;
    }
    if ((await act(page, screen, pick)) === null) break;
  }
  expect(opened, "never reached a placement picker").toBe(true);

  // The picker is open with nothing selected (the walk stopped at its arrival).
  const before = await readScreen(page);
  expect(before.pressed, "a picker that opens with a tile already pressed is a refused Confirm").toBe(0);
  const piecesOn = (id: string) =>
    page.evaluate((want) => {
      const tile = [...document.querySelectorAll(".tile")].find(
        (t) => t.querySelector(".tile-id")?.textContent === want,
      )!;
      return tile.querySelector(".pieces")?.getAttribute("aria-label") ?? "";
    }, id);

  // Press tiles one at a time until the server takes one, as the walk does.
  const free = page.locator('.tile[role="button"][aria-pressed="false"]:not([aria-disabled="true"])');
  let pressedId: string | null = null;
  const n = await free.count();
  for (let i = 0; i < n && pressedId === null; i++) {
    const tile = free.nth(i);
    const id = (await tile.getAttribute("aria-label"))!.slice(0, 3);
    await tile.click();
    if ((await readScreen(page)).pressed > 0) pressedId = id;
  }
  expect(pressedId, "the server took no tile, so there is nothing for Cancel to let go").not.toBeNull();
  const piecesBefore = await piecesOn(pressedId!);

  await page.locator(".decision .controls button.ghost", { hasText: "Cancel" }).first().click();

  const after = await readScreen(page);
  expect(after.pressed, "Cancel let the pressed tile go").toBe(0);
  expect(after.tiles, "and the picker is closed: no tile is a control any more").toBe(0);
  expect(after.confirm, "and there is nothing to Confirm").toBe(false);
  await expect(page.getByRole("button", { name: "Confirm" })).toHaveCount(0);
  expect(await piecesOn(pressedId!), "and nothing was applied to the tile that was pressed").toBe(piecesBefore);
  expect(after.options, "the choice the picker came from is answerable again").toBeGreaterThan(0);
  expect(watch.failures).toEqual([]);
});

test("the Game menu switches between saved games from inside a game", async ({ page }) => {
  const watch = watchForBreakage(page);
  const a = await startGame(page, { players: 2 });
  await playAFew(page, 4, 0x4d45_4e55); // "MENU"
  await page.goto("/");
  const b = await startGame(page, { players: 2 });
  await page.locator(".game-menu > summary").click();
  await page.getByRole("button", { name: a.id, exact: true }).click();
  await expect(page.locator(".game-menu > summary")).toContainText(a.id);
  // **The menu CLOSES once a switch has landed** (M6.5c2 cycle D, dossier row
  // 2), and this is the cell where closed means hidden. It used to stay open,
  // and this cell leaned on that — it clicked the way back without reopening.
  // `GameMenu.test.tsx` reads the `open` attribute; only Chromium applies the
  // UA stylesheet that hides a closed menu's rows (`display: none`), so the
  // way back is asserted NOT visible before the summary click and visible
  // after it.
  const menu = page.locator("details.game-menu");
  const back = page.getByRole("button", { name: b.id, exact: true });
  await expect(menu).not.toHaveAttribute("open", "");
  await expect(back).toBeHidden();
  await page.locator(".game-menu > summary").click();
  await expect(back).toBeVisible();
  await back.click();
  await expect(page.locator(".game-menu > summary")).toContainText(b.id);
  await expect(menu).not.toHaveAttribute("open", "");
  expect(watch.failures).toEqual([]);
});

// ---- M5 W5b: the sections on the table --------------------------------------

test("an enabled section asks nothing after a move its run covers, collapses while replaying, and persists across a reload", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  const { id } = await startGame(page, { players: 2 });
  // **The rule this cell pins moved in cycle E (Task 7; R-E9, ruling T7-a).**
  //
  // M5's rule was "an enabled fetched section re-fetches after every
  // `setState`", and this cell counted one `/ports` per state-changing
  // RESPONSE. Measured over the two-browser walk, those reads were 7.12 of
  // 16.74 requests per applied move, and every one of them was a copy: a
  // picker step's reply applies no event and cannot move a board, and a reply
  // that applied events is followed by its RUN, whose last frame is the live
  // position with the port and the laboratory built by the same
  // `build_ports`/`build_labs` those two routes answer with. So the port and
  // the laboratory are asked for only where no run carries them — the mount,
  // an undo, a re-read that found the game moved, a run that failed — and a
  // move this walk makes is none of those (`act` never undoes).
  //
  // **The relation, then, is ZERO, with two things that stop it being
  // vacuous**: the walk made state-changing responses (counted on the
  // response, and only where it succeeded, as before), and at least one of
  // them applied events and fetched its run (`/at?from=`). And the strip is
  // read against the SERVER's own `/ports` afterwards, which is what says the
  // run's frame IS the live board rather than merely a board.
  const boardRequests: string[] = [];
  const runRequests: string[] = [];
  const inputPosts: string[] = [];
  const reReads: string[] = [];
  page.on("request", (r) => {
    const u = new URL(r.url());
    if (/\/(ports|labs)$/.test(u.pathname)) boardRequests.push(u.pathname);
    if (/\/at$/.test(u.pathname) && u.searchParams.has("from")) runRequests.push(r.url());
  });
  page.on("response", (r) => {
    if (!r.ok()) return;
    const path = new URL(r.url()).pathname;
    if (/\/input$/.test(path) && r.request().method() === "POST") inputPosts.push(r.url());
    if (/^\/api\/games\/[^/]+$/.test(path) && r.request().method() === "GET") reReads.push(r.url());
  });
  // Building sites is off by default; turn it on and see it fetched once.
  await page.locator(".boards-menu > summary").click();
  await page.getByRole("button", { name: "Building sites" }).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".table-sec.buildings .site-card").first()).toBeVisible();
  // The mount's reads have landed: the strips are drawn.
  await expect(page.locator(".table-sec.ports .board-strip").first()).toBeVisible();
  const boardsBefore = boardRequests.length;
  const runsBefore = runRequests.length;
  const postsBefore = inputPosts.length;
  const readsBefore = reReads.length;
  // `playAFew` settles the last move's walk (see its own note): the counts
  // below are read off a still screen, and the replay assertions further down
  // cannot pass because a WALK rather than the scrub had the table in the past.
  await playAFew(page, 1, 0x5245_4652); // "REFR"
  const stateChanges = () =>
    inputPosts.length - postsBefore + (reReads.length - readsBefore);
  await expect
    .poll(() => stateChanges(), { message: "the walk applied nothing, so there was nothing to ask for" })
    .toBeGreaterThan(0);
  await expect
    .poll(() => runRequests.length - runsBefore, { message: "no move applied events, so no run was fetched" })
    .toBeGreaterThan(0);
  expect(boardRequests.slice(boardsBefore), "a move's run carries the port and the laboratory").toEqual([]);
  // The strip beside the live board is the server's live port.
  const served = await page.request.get(`/api/games/${id}/ports`, {
    headers: { "x-yokohama-claimed-user": WALKER },
  });
  expect(served.ok(), `the port is readable (${served.status()})`).toBeTruthy();
  const livePort = ((await served.json()) as { boards: { slots: { card: number | null }[] }[] }).boards.flatMap(
    (b) => b.slots.flatMap((sl) => (sl.card === null ? [] : [String(sl.card)])),
  );
  expect(livePort.length, "the server's port holds cards").toBeGreaterThan(0);
  await expect
    .poll(() =>
      page.locator(".table-sec.ports [data-card]").evaluateAll((els) => els.map((e) => e.getAttribute("data-card"))),
    )
    .toEqual(livePort);
  // eslint-disable-next-line no-console
  console.log(
    `sections after a move: ${stateChanges()} state-changing responses, ` +
      `${runRequests.length - runsBefore} runs, ${boardRequests.length - boardsBefore} /ports or /labs requests`,
  );

  await page.getByRole("button", { name: "Log", exact: true }).click();
  const track = page.getByLabel("Replay position");
  await track.focus();
  await page.keyboard.press("ArrowLeft");
  // **What collapses is the building sites, and only those (M5.5).** `/at/{k}`
  // carries the port and the laboratory now, so those two draw the offer that
  // WAS there; the sites' board is read from the live game and has no
  // position-shaped version, so it is the one that says it is hidden.
  await expect(page.locator(".table-sec.buildings.collapsed")).toBeVisible();
  await expect(page.locator(".table-sec.ports .board-strip")).toBeVisible();
  await expect(page.locator(".table-sec.tracks")).toBeVisible();
  await page.getByRole("button", { name: "Back to live" }).click();
  await expect(page.locator(".table-sec.ports .board-strip")).toBeVisible();
  await page.keyboard.press("Escape");

  // The reload comes back to the same table by its hash (M6.2 §5.1), so the
  // section this cell is about is read on the game it was switched off in —
  // which is what the sentence "persists across a reload" always meant.
  await page.reload();
  await expect(page.locator(".table-sec.buildings")).toBeVisible();
  await page.locator(".boards-menu > summary").click();
  await expect(page.getByRole("button", { name: "Building sites" })).toHaveAttribute("aria-pressed", "true");
  expect(watch.failures).toEqual([]);
});

test("the port and laboratory strips sit side by side at 2560 and stack at 1710", async ({ page }) => {
  // Every other cell in this file declares one; without it a console error or a
  // 4xx during the two viewport changes goes unrecorded.
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  const tops = () =>
    page.evaluate(() => ({
      port: document.querySelector(".table-sec.ports")!.getBoundingClientRect().top,
      lab: document.querySelector(".table-sec.labs")!.getBoundingClientRect().top,
    }));
  await page.setViewportSize({ width: 2560, height: 1440 });
  await settle(page);
  await expect.poll(async () => (await tops()).port === (await tops()).lab, { message: "not side by side at 2560" }).toBe(true);
  await page.setViewportSize({ width: 1710, height: 869 });
  await settle(page);
  const t = await tops();
  expect(t.lab, "not stacked at 1710").toBeGreaterThan(t.port);
  expect(watch.failures).toEqual([]);
});

// ---- M5.5 A: a pile of four, built on purpose ------------------------------

/** Click the first enabled option row whose words match, or say there is none. */
async function answer(page: Page, words: RegExp): Promise<boolean> {
  const rows = page.locator("button.option:not([disabled])").filter({ hasText: words });
  if ((await rows.count()) === 0) return false;
  await rows.first().click();
  return true;
}

test("four assistants of one colour stand as a pile, not as a cube and a count", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await startGame(page, { players: 2 });
  // Option B twice into the SAME area, with the President left in hand on
  // both turns: a President that acts in an area recovers its cubes at the
  // Recovery step, so the pile forms only if it never goes there. The bot's
  // turn in between cannot recover another seat's cubes, and its President
  // parking there at most prices the second placement at two yen.
  let area: string | null = null;
  let placements = 0;
  for (let i = 0; i < 40 && placements < 2; i++) {
    const screen = await readScreen(page);
    if (screen.over) throw new Error("the game ended before two placements");
    if (await answer(page, /Option B/)) {
      // The picker's tiles carry `aria-disabled` while the request is in
      // flight (M5.5 R-F2 — they stopped being `<button disabled>` when the
      // tag switch was found to remount the art), so wait for the settled
      // screen before reading tile ids off it.
      await expect(page.locator('.tile[role="button"]').first()).not.toHaveAttribute("aria-disabled", "true");
      const tiles = page.locator('.tile[role="button"]:not([aria-disabled="true"])');
      const target: Locator =
        area === null
          ? tiles.first()
          : tiles.filter({ has: page.locator(`.tile-id:text-is("${area}")`) }).first();
      area ??= (await target.locator(".tile-id").innerText()).trim();
      await target.click();
      await page.getByRole("button", { name: "Confirm" }).click();
      // The pile assertion must not be satisfiable by cubes that landed
      // elsewhere, so the applied move is pinned rather than assumed: the
      // account's own row names the area it fed. A tile being clickable is no
      // evidence, either: while picking, `disabled` is `busy` alone
      // (`Board.tsx`), so every non-Canal tile is offered.
      //
      // **Re-pointed at the run's ROWS (M5.5).** A reply that applies an event
      // now builds a run, and the account renders it as `ol.account > li`
      // rather than as the reply's prose `div`s — the old locator would match
      // nothing here and the pin would assert about a screen that is not on
      // screen. The WORDING moves with it, and the two are not the same: the
      // reply's delta reads "Red: 2 assistants placed in Customs (A09)"
      // (`render.rs:922`) while the log line the row carries reads "Red: Place
      // 2 assistants in Customs (A09)" (`web/src/wire/log.json`). The area id
      // is what both have in common and is what this cell is about.
      //
      // Still exactly one row: a placement Confirm mid-turn hands the prompt
      // back to the same seat, so no bot step rides in the same run.
      await expect(
        page.locator(".turn-account .messages li").filter({
          hasText: new RegExp(`assistants? in .*\\(${area}\\)`),
        }),
      ).toHaveCount(1);
      placements += 1;
      continue;
    }
    // Two wordings for one move, at two levels: the top-level Movement row is
    // the GROUPED family (`repl.rs:1353`), which opens a guided per-area count
    // sub-prompt; "Return President to hand" is the move's own wording
    // (`describe.rs:904`) and reaches the top level only when there is nothing
    // recoverable. Answering every area 0 is what leaves the pile standing.
    if (await answer(page, /Retreat President to hand/)) {
      // `g < 8` is generous for this script rather than a general bound: the
      // sub-prompt asks once per area holding the mover's assistants, and here
      // that is the one area both Option Bs fed.
      for (let g = 0; g < 8; g++) {
        const sub = await readScreen(page);
        if (sub.confirm) {
          await page.getByRole("button", { name: "Confirm" }).click();
          break;
        }
        if (sub.counts === 0) throw new Error("the guided retreat offered no counts");
        await page.locator(".options .controls button.ghost", { hasText: /^0$/ }).first().click();
      }
      continue;
    }
    if (await answer(page, /Return President to hand/)) continue;
    if (screen.only) {
      await page.getByRole("button", { name: "Apply the only option" }).click();
      continue;
    }
    if (await answer(page, /End additional-action phase|Skip the/)) continue;
    throw new Error(
      // `.decision` is on the warnings block too, and `App.tsx:464` renders
      // that one FIRST — empty unless the server warned, so `.first()` quoted
      // nothing. `:not(.notices)` is the answer bar (`Decision.tsx:107`).
      `no scripted answer for this screen:\n${await page
        .locator(".decision:not(.notices)")
        .first()
        .innerText()}`,
    );
  }
  expect(placements, "two placements were made").toBe(2);

  const tile = page.locator(".board .tile").filter({ has: page.locator(`.tile-id:text-is("${area}")`) });
  const pile = tile.locator(".piece-group.pile");
  await expect(pile).toHaveCount(1);
  await expect(pile.locator(".piece.cube")).toHaveCount(4);
  await expect(tile.locator(".piece-n")).toHaveCount(0);
  const cube = await pile.locator(".piece.cube").first().boundingBox();
  // RECORDED, never gated (spec §5): the height depends on the fit rule's tile
  // width on this machine at this viewport.
  console.log(`pile cube at 1710×869, 2P, on ${area}: ${(cube?.height ?? 0).toFixed(1)}px tall`);
  expect(cube?.height ?? 0).toBeGreaterThan(0);
  mkdirSync("../docs/temp/web-review/gates", { recursive: true });
  await page.screenshot({ path: "../docs/temp/web-review/gates/a-pile-1710x869.png" });
  expect(watch.failures).toEqual([]);
});

// **The asking seat stays reachable when this browser holds four of them.**
//
// The claim is inherited from the 2026-09-06 sequence, and the mechanism under
// it changed: that version put every human seat's cards in the deck at once, and
// at 4P the block ran 931px inside a 539px window with the asking seat's own
// label 28px ABOVE the fold on the middle turn — a player scrolling up to find
// the cards they were just handed. `GET …/deal` now serves ONE seat, so the deck
// holds one card row at every player count and the other seats are a line each
// in the side column. The property is the same and worth keeping: the seat being
// asked, and both of its cards, are in the frame unscrolled.
//
// The MIDDLE seat specifically: it is the worst case, with answered seats above
// it in the column and unanswered ones below. jsdom has no layout, so this claim
// can only be made here.
test("at four players the asking seat is in the frame on the middle turn", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  // The lobby, then the form (M6.2 §5.1) — and every seat said to be this
  // browser's, because above two players the form now defaults them to OPEN and
  // an open table is not dealt at all (§5.3).
  await newGameForm(page);
  await page
    .getByRole("group", { name: "Players" })
    .getByRole("radio", { name: "4", exact: true })
    .check();
  // Each seat by its colour since M6.5c2 (mock 07) — the same lines
  // `startGame` runs — and `exact`, because a colour is also a First player
  // choice.
  for (const color of ["Blue", "Yellow", "Green"]) {
    await page.getByRole("combobox", { name: `${color} seat`, exact: true }).selectOption("me");
  }
  await page.getByRole("button", { name: "Deal" }).click();
  // Four seats, all this browser's (R-P10: no bot above two players), and the
  // dealing screen asks for the lowest one that has not kept.
  const current = page.locator(".deal-seat.human.current");
  await expect(current).toHaveCount(1);
  await expect(page.locator(".deal-seat.human.other")).toHaveCount(3);

  // Two keeps in, so the seat being asked is the middle one: two answered above
  // it in the column and one still waiting below.
  for (let i = 0; i < 2; i++) {
    await page.locator(".deal-seat.human.current button.deal-pick:not([disabled])").first().click();
    await expect(page.locator(".deal-seat.human.other.chosen")).toHaveCount(i + 1);
  }
  await expect(current).toHaveCount(1);

  // The label AND both buttons, at `ratio: 1`: a seat whose cards are in the
  // frame while its own heading is not still leaves the player looking for it,
  // and the default (any intersection) passes on one visible pixel.
  await expect(current.locator("label")).toBeInViewport({ ratio: 1 });
  await expect(current.locator("button.deal-pick")).toHaveCount(2);
  for (let i = 0; i < 2; i++) {
    await expect(current.locator("button.deal-pick").nth(i)).toBeInViewport({ ratio: 1 });
  }

  const read = await page.evaluate(() => {
    const deck = document.querySelector(".deck > .decision") as HTMLElement;
    const cur = document.querySelector(".deal-seat.human.current") as HTMLElement;
    const box = deck.getBoundingClientRect();
    return {
      window: Math.round(box.height),
      block: deck.scrollHeight,
      top: Math.round(cur.getBoundingClientRect().top - box.top),
    };
  });
  // RECORDED, never gated: heights are layout- and font-dependent, and what is
  // gated is the property above. The figures are here so a future reader can
  // re-measure against the two the CSS comment states from the hot-seat deck
  // this replaced (931/539/-28 with three card rows, 738/539/+25 once the two
  // that were not choosing shrank).
  // eslint-disable-next-line no-console
  console.log(
    `4P set table at 1710x869, middle turn: deck window ${read.window}px holding ${read.block}px, ` +
      `asking seat's top at ${read.top}px (recorded)`,
  );
  expect(watch.failures).toEqual([]);
});

// ---- M5.5 part B: the set table --------------------------------------------
//
// The two halves jsdom cannot see. (1) The port strip on the dealing screen
// draws SIX cards from `deal.frame` rather than the "loading…" line a fetch with
// no game to name would leave — the component suite asserts the element, this
// asserts it against a real server. (2) The card the player pressed is the card
// that reaches the hand of the game the last keep starts, which spans the
// table's seed, the keep this browser posts and the deal the engine makes from
// them; nothing short of the real round trip can see that join.
//
// **And the bot's own deal is HIDDEN on both sides of that (R-C1).** The
// component suite reads the absence off a committed fixture; this reads it off
// what a real server sent a real browser, which is the only reading that can say
// the payload itself carried nothing — and since M6.1 it is a stronger reading
// still, because `GET …/deal` serves ONE seat and the other seat's two cards are
// not in any payload this browser received.
test("the set table shows the port, hides the bot's cards and withholds the technologies and goals before the first choice, and the chosen card lands in hand", async ({ page }) => {
  const watch = watchForBreakage(page);
  await page.setViewportSize({ width: 1710, height: 869 });
  await newGameForm(page);
  await page.getByRole("button", { name: "Deal" }).click();
  await expect(page.locator(".table-sec.ports .strip-slot .card-face")).toHaveCount(6);
  await expect(page.locator(".board .tile")).toHaveCount(10);
  // Two backs and the line under the bot, and no id and no Keep score anywhere
  // in its panel — against the human seat's own two ids on the same screen, so
  // this is the bot's deal being withheld and not a deal that failed to draw.
  await expect(page.locator(".deal-seat.human .card-id")).toHaveCount(2);
  await expect(page.locator(".deal-seat.bot img.back")).toHaveCount(2);
  await expect(page.locator(".deal-seat.bot .card-id")).toHaveCount(0);
  await expect(page.locator(".deal-seat.bot")).not.toContainText("Keep score");
  await expect(page.locator(".deal-seat.bot")).toContainText("keeps one of its two cards");
  // **The technologies and the goals are withheld until the choice (R-C3).**
  // Against a real server, so this reads what the payload actually carried: the
  // owner's edition deals the two orders and takes the secret choice at step 9,
  // and the technologies reach the Laboratory at step 10 and the achievements
  // the score track at step 11. The port above and the tracks below are R-C4's
  // half — what stays — and are why a set table that drew nothing fails here.
  await expect(page.locator(".table-sec.labs")).toContainText(
    "revealed once you have chosen your card",
  );
  await expect(page.locator(".table-sec.labs .card-face")).toHaveCount(0);
  await expect(page.locator(".table-sec.achievements")).toContainText(
    "revealed once you have chosen your card",
  );
  await expect(page.locator(".table-sec.achievements .card-face")).toHaveCount(0);
  await expect(page.locator(".table-sec.tracks")).toBeVisible();

  // **A section holding one sentence does not take the row's height.**
  // `.table-sections` is a wrapping flex row, so before `align-self:
  // flex-start` on `.table-sec.collapsed` the two withheld boxes stretched to
  // the height of whatever shared their line — measured here against the TRACKS
  // panel, which sits between them on this row and is the tall one. The three
  // figures are printed; what is gated is the property, which is what a reader
  // of them cares about, and it needs no threshold of its own.
  const boxes = await page.evaluate(() => {
    const h = (sel: string) =>
      Math.round(document.querySelector(sel)!.getBoundingClientRect().height);
    return {
      labs: h(".table-sec.labs"),
      goals: h(".table-sec.achievements"),
      tracks: h(".table-sec.tracks"),
    };
  });
  // eslint-disable-next-line no-console
  console.log(
    `set table withheld boxes at 1710x869: laboratory ${boxes.labs}px · goals ${boxes.goals}px · ` +
      `tracks ${boxes.tracks}px (the row's tall one) — the two lined boxes are gated below it`,
  );
  expect(boxes.labs, "the withheld laboratory stretches to the row").toBeLessThan(boxes.tracks);
  expect(boxes.goals, "the withheld goals stretch to the row").toBeLessThan(boxes.tracks);

  // **There is nothing to press but a card (M6.1 §7.4).** The keep IS the
  // commit: the last one finalizes the table server-side and the game opens, so
  // there is no Start and no Re-deal to go back to a preview that no longer
  // exists. One seat is asking, and it is this browser's.
  await expect(page.getByRole("button", { name: "Start" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Re-deal" })).toHaveCount(0);
  await expect(page.locator(".deal-seat.human.current")).toHaveCount(1);

  // **Inert, and still readable (spec §3.3.3).** `.tile[role="button"]` is the placement
  // picker's own marker — `Board` gives a tile that role only when a caller
  // hands it an `onTileClick`, which `shots.spec.ts` relies on too — so zero of
  // them IS what "inert" means here; and the hover popover is what lets a
  // player read a site card at full size before choosing. (The marker was
  // `button.tile` until M5.5's R-F2 made every tile a `<div>`.)
  await expect(page.locator('.tile[role="button"]')).toHaveCount(0);
  await page.locator(".board .tile").first().hover();
  await expect(page.getByRole("tooltip")).toBeVisible();
  await page.mouse.move(2, 2);
  await expect(page.getByRole("tooltip")).toHaveCount(0);

  // **R-T12-3: the bot's whole panel is in the frame at 1710x869, unscrolled.**
  // The deck's block and the side column share the 808px under the banner, and
  // with the generic card (two chip rows) and the generic paddings they wanted
  // 831 — so the bot's own captions sat below the fold and only a scrolled
  // camera could photograph them. Asserted in the VIEWPORT rather than in the
  // DOM, because the DOM had them all along — and at `ratio: 1`, because the
  // default (any intersection at all) passes on an element whose top pixel is
  // showing. **The PANEL is what carries the claim**: measured in M5.5,
  // restoring the bot card's chip rows overflowed the column by 38px while both
  // the default assertion and `ratio: 1` on a single child stayed green, and
  // `ratio: 1` on the panel failed. The budget got EASIER in the 2026-09-06
  // corrections — two backs with no caption where two captioned cards and a
  // badge stood — and the assertion is kept at its old strength rather than
  // relaxed to match, so a caption growing back is still caught here.
  await expect(page.locator(".deal-seat.bot")).toBeInViewport({ ratio: 1 });
  await expect(page.locator(".deal-seat.bot .deal-unseen")).toBeInViewport({ ratio: 1 });
  for (let i = 0; i < 2; i++) {
    await expect(page.locator(".deal-seat.bot img.back").nth(i)).toBeInViewport({ ratio: 1 });
  }
  // The hedge under the picks rather than a controls row: there is no button in
  // this deck at all now, and the claim — the bottom of the deck's block is in
  // the frame — is the same one.
  await expect(page.locator(".deck .keep-hedge")).toBeInViewport({ ratio: 1 });
  const budget = await page.evaluate(() => {
    const q = (x: string) => document.querySelector(x) as HTMLElement;
    return {
      deck: Math.round(q(".deck").getBoundingClientRect().height),
      deckContent: q(".deck > .decision").scrollHeight,
      side: Math.round(q(".side").clientHeight),
      sideContent: q(".side").scrollHeight,
    };
  });
  // RECORDED, never gated: the heights are layout- and font-dependent. What is
  // gated is the property above, which is what a reader of them cares about.
  // eslint-disable-next-line no-console
  console.log(
    `set table at 1710x869: deck ${budget.deck}px holding ${budget.deckContent}px · ` +
      `side ${budget.side}px holding ${budget.sideContent}px (recorded)`,
  );

  // **Ruling R-T12-1: the set table does not widen the page either.** Spec §4.3
  // names the same four widths the M3.5 gate uses, and this screen is a
  // different layout — a full table with cards in the deck — reached before any
  // game exists. Measured with the width gate's own instrument (`readOverflow`,
  // shared from the harness since this cell needed it); the page delta is the
  // only thing ASSERTED, everything else is recorded.
  for (const width of [390, 1024, 1201, 1440]) {
    await page.setViewportSize({ width, height: 844 });
    await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => resolve(null))));
    const read = await readOverflow(page);
    // Anti-vacuity, the gate's own: a set table that drew no board has no
    // overflow either. 2P's widest row is four tiles.
    expect(read.widestRow, `the set table drew no board at ${width}px`).toBe(4);
    expect(
      read.docScrollWidth - read.docClientWidth,
      `the set table scrolls the page sideways at ${width}px: ${blame(read)}`,
    ).toBeLessThanOrEqual(0);
    // eslint-disable-next-line no-console
    console.log(
      `set table at ${width}px: page overflow ${read.docScrollWidth - read.docClientWidth}px ` +
        `(gated) · board scrolls ${read.board ? read.board.scrollWidth - read.board.clientWidth : 0}px ` +
        `inside its column (recorded)`,
    );
  }
  await page.setViewportSize({ width: 1710, height: 869 });

  // **The keep IS the commit now.** This browser holds the only human seat, so
  // the card pressed here is the last keep at the table: the server finalizes,
  // the reply says `playing`, and the client opens the game — no Start, and no
  // three-second wait on the poll. The SECOND card, so the join below cannot be
  // satisfied by a first-card default anywhere in the chain.
  const pick = page.locator(".deal-seat.human.current button.deal-pick").nth(1);
  const chosen = /keep (O\d\d)/.exec((await pick.getAttribute("aria-label"))!)![1]!;
  await pick.click();
  await expect(atTheGame(page)).toBeVisible();
  await readScreen(page);
  await expect(page.locator(".panel", { hasText: /'S HAND|’s hand/i }).first()).toContainText(chosen);
  // **And they come OUT once the card is chosen**, which is the half that makes
  // the withholding a delay rather than a deletion. Six technologies on the
  // laboratory and the line gone — asserted against the same server that sent
  // neither one screen ago.
  await expect(page.locator(".table-sec.labs .strip-slot .card-face")).toHaveCount(6);
  await expect(page.locator(".table-sec.labs")).not.toContainText(
    "revealed once you have chosen your card",
  );
  // Three achievement cards in a game: one each from the A, B and C decks
  // (the owner's edition's step 11). The exact count, for the reason the six
  // above are exact — "more than none" would pass on a strip that lost two.
  await expect(page.locator(".table-sec.achievements .card-face")).toHaveCount(3);
  expect(watch.failures).toEqual([]);
});

// ---- issues #75 and #76: the claim band and the flag tally, measured -------
//
// Both are drawn and unit-tested in jsdom, which has no layout: whether four
// cubes and both numbers stand on ONE line inside the picture box, and whether
// five flag cells with a `+n` beside a count fit a phone's seat row, are
// questions only a real layout answers. Boxes are read with
// `getBoundingClientRect`; what is not asserted is printed "(recorded)".

/** A box as numbers, carried out of the page. */
interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

/** **Rounding tolerance for "inside": 0.5 px on each of the four edges.** A
 *  border box is fractional and two boxes laid out from the same `cqw` lengths
 *  can disagree in the last sub-pixel; half a pixel is under anything a reader
 *  can see and well under any real overflow (a cube one step too wide, a row
 *  one cell too long). */
const INSIDE_TOLERANCE_PX = 0.5;

function outside(inner: Rect, outer: Rect): string | null {
  const t = INSIDE_TOLERANCE_PX;
  const misses = [
    inner.left < outer.left - t && `left ${inner.left.toFixed(2)} < ${outer.left.toFixed(2)}`,
    inner.top < outer.top - t && `top ${inner.top.toFixed(2)} < ${outer.top.toFixed(2)}`,
    inner.right > outer.right + t && `right ${inner.right.toFixed(2)} > ${outer.right.toFixed(2)}`,
    inner.bottom > outer.bottom + t && `bottom ${inner.bottom.toFixed(2)} > ${outer.bottom.toFixed(2)}`,
  ].filter(Boolean);
  return misses.length ? misses.join(", ") : null;
}

/** The committed four-claim card (Task 3): the Achievements PANEL's content as a
 *  server emits it, card 0 claimed by seats 2, 0, 3, 1 in that order. */
const GOALS_CLAIMED_4P = JSON.parse(
  readFileSync(fileURLToPath(new URL("../src/wire/goals_claimed_4p.json", import.meta.url)), "utf8"),
) as { achievements: { cards: { claimed_by: number[] }[] } };

/** **The floor is a READING halved, set after it was measured** (the plan's
 *  Task 6). Measured on the first run of this cell, with the floor recording
 *  only (2026-11-20, macOS, Chromium headless): the cube's box at 390 px read
 *  21.27 px wide — and the same at 1024, 1201 and 1440, since the strip's card
 *  is one width at all four (a 177.22 px picture box). The floor is half that
 *  reading, 10.635 px. It is a floor and not an equality because the width is
 *  the layout's to choose; what it must not do is collapse, which is how a
 *  nowrap band that cannot fit gives way (its cubes shrink, `flex: 0 1`). */
const CUBE_FLOOR_PX = 21.27 / 2;

test("four claims stand on one line inside the picture box of the Goals strip's first card, at every width", async ({ page }) => {
  const watch = watchForBreakage(page);
  // **The game's state reaches the page as `GET /api/games/{id}`'s reply**
  // (`api.get`, a `ClientState`), and the Goals strip draws its cards from that
  // reply's `frame.dashboard` Achievements panel (`TableSections.tsx`) — no
  // fetch of its own. So the rewrite is of that reply, and of nothing in it but
  // that one panel's content: the rest stays the server's own. A four-seat
  // game is a SERVER game (a local game is two seats), so the reply is a real
  // HTTP response `page.route` can see.
  let rewritten = 0;
  let serverClaims: number | null = null;
  await page.route(
    (url) => /^\/api\/games\/[^/]+$/.test(url.pathname),
    async (route) => {
      if (route.request().method() !== "GET") return route.fallback();
      const response = await route.fetch();
      const body = (await response.json()) as { frame?: { dashboard?: Record<string, unknown>[] } };
      const panel = body.frame?.dashboard?.find((p) => "Achievements" in p);
      if (!panel) return route.fulfill({ response, json: body });
      const own = (panel.Achievements as { cards: { claimed_by: number[] }[] }).cards;
      serverClaims = own[0]?.claimed_by.length ?? 0;
      panel.Achievements = GOALS_CLAIMED_4P.achievements;
      rewritten += 1;
      return route.fulfill({ response, json: body });
    },
  );
  await startGame(page, { players: 4 });

  // Anti-vacuity. The rewrite RAN, the server's own first card had fewer than
  // four claims (a fresh game has none), and the strip's first card now draws
  // the four — so the four boxes below are the fixture's and nothing else's.
  const card = page.locator(".table-sec.achievements .card-face.achievement").first();
  await expect(card.locator(".card-pic > .claims > .claim")).toHaveCount(4);
  expect(rewritten, "no game-state reply was rewritten").toBeGreaterThan(0);
  expect(serverClaims, "the server's own reply carried no Achievements panel").not.toBeNull();
  expect(serverClaims!, "the server's own first card already had four claims").toBeLessThan(4);
  expect(GOALS_CLAIMED_4P.achievements.cards[0]!.claimed_by).toEqual([2, 0, 3, 1]);

  // **The four widths walk ONE card size, and it is the narrowest.** The strip's
  // achievement card is `calc(var(--card-w) * 1.5)`. At 390, 1024, 1201 and
  // 1440 px the rule in force is `.table-sections`' own `--card-w: clamp(8rem,
  // 12cqw, 12rem)` (`styles.css`, the M5 strips block), and its `12cqw` term
  // is under 8rem at all four, so it sits at its 8rem floor: the card is 192 px
  // wide with a 177.22 px picture box at all four (measured; the review's
  // export read the same). It is wider only on wider screens, by the review:
  // 237.59 px at 1800, still under that rule with `12cqw` past the floor, and
  // 222 px at 2560, where the table is at least 108rem wide and the
  // `@container (min-width: 108rem)` arm below it takes over with `--card-w:
  // clamp(8rem, (100cqw - 13rem) / 12, 12rem)`, the regime that puts two
  // strips side by side. That 8rem floor IS "the Goals strip's narrowest card
  // width" the spec names (§5.1, §6), so this walk shows the band holds at the
  // narrowest card at every viewport the file walks — not at four different
  // card sizes.
  for (const width of [390, 1024, 1201, 1440]) {
    await page.setViewportSize({ width, height: 844 });
    await settle(page);
    await card.scrollIntoViewIfNeeded();
    await settle(page);
    const read = await card.evaluate((face) => {
      const rect = (el: Element | null) => {
        if (!el) return null;
        const r = el.getBoundingClientRect();
        return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
      };
      const pic = face.querySelector(":scope > .card-pic");
      const band = pic?.querySelector(":scope > .claims") ?? null;
      return {
        client: document.documentElement.clientWidth,
        inner: window.innerWidth,
        face: rect(face)!,
        pic: rect(pic ?? null),
        band: rect(band),
        claims: [...(band?.querySelectorAll(":scope > .claim") ?? [])].map((c) => rect(c)!),
        firstIsFirst: !!band?.querySelector(":scope > .claim:first-of-type.first"),
        vpFirst: rect(band?.querySelector(":scope > .claims-vp.first") ?? null),
        vpLater: rect(band?.querySelector(":scope > .claims-vp.later") ?? null),
        // What is PAINTED at each cube's and each number's centre: the topmost
        // element there, which must be inside the band. A box says where a
        // thing is laid out, not that it is seen — a band painted under the
        // art, or hidden, keeps every box above and puts the picture here.
        hits: [...(band?.querySelectorAll(":scope > .claim, :scope > .claims-vp") ?? [])].map((el) => {
          const r = el.getBoundingClientRect();
          const hit = document.elementFromPoint((r.left + r.right) / 2, (r.top + r.bottom) / 2);
          return {
            what: el.getAttribute("class") ?? "",
            inBand: !!hit && !!band && band.contains(hit),
            hit: hit ? `${hit.tagName}.${hit.getAttribute("class") ?? ""}` : "nothing",
          };
        }),
      };
    });
    const at = `at ${width}px`;
    // The viewport the page laid out at is the one asked for.
    expect(read.inner, `${at}: the window is not the width asked for`).toBe(width);
    expect(read.client, `${at}: the document lays out wider than the window`).toBeLessThanOrEqual(width);
    expect(read.pic, `${at}: no picture box`).not.toBeNull();
    expect(read.band, `${at}: no claim band in the picture box`).not.toBeNull();
    expect(read.vpFirst, `${at}: no first-place number`).not.toBeNull();
    expect(read.vpLater, `${at}: no later-claims number`).not.toBeNull();
    expect(read.claims, `${at}: the band does not hold four cubes`).toHaveLength(4);
    expect(read.firstIsFirst, `${at}: the leading cube is not the ringed first place`).toBe(true);
    const pic = read.pic!;
    const band = read.band!;
    // No zero-size box can pass a containment check vacuously.
    for (const [name, r] of [["picture box", pic], ["band", band], ["first VP", read.vpFirst!], ["later VP", read.vpLater!]] as const) {
      expect(r.width > 0 && r.height > 0, `${at}: the ${name} has no size (${r.width}x${r.height})`).toBe(true);
    }
    // ONE line: the four share a top, and they are side by side — a stack at
    // one point shares a top too.
    const tops = read.claims.map((c) => c.top);
    expect(Math.max(...tops) - Math.min(...tops), `${at}: the cubes do not share a top (${tops.map((t) => t.toFixed(2)).join(", ")})`).toBeLessThanOrEqual(1);
    for (let i = 1; i < 4; i++) {
      expect(read.claims[i]!.left, `${at}: cube ${i} is not to the right of cube ${i - 1}`).toBeGreaterThan(read.claims[i - 1]!.left);
    }
    read.claims.forEach((c, i) => {
      expect(c.height, `${at}: cube ${i} has no height`).toBeGreaterThan(0);
      expect(outside(c, band), `${at}: cube ${i} is not inside the band`).toBeNull();
      expect(c.width, `${at}: cube ${i} is ${c.width.toFixed(2)}px wide, under the ${CUBE_FLOOR_PX}px floor`).toBeGreaterThanOrEqual(CUBE_FLOOR_PX);
    });
    expect(outside(band, pic), `${at}: the band is not inside the picture box`).toBeNull();
    expect(outside(read.vpFirst!, pic), `${at}: the first-place number is not inside the picture box`).toBeNull();
    expect(outside(read.vpLater!, pic), `${at}: the later number is not inside the picture box`).toBeNull();
    // Painted, not only laid out: the hit test at each of the six centres.
    expect(read.hits, `${at}: the hit test did not read four cubes and two numbers`).toHaveLength(6);
    for (const h of read.hits) {
      expect(h.inBand, `${at}: at the centre of the ${h.what} the page paints ${h.hit}, not the band`).toBe(true);
    }
    // **And both numbers are DRAWN, on the cubes' line** (spec §5.1, §6: "four
    // cubes and both numbers on one line"). Inside the picture box is not
    // evidence a number is seen: the band clips with `overflow: hidden`, so a
    // number pushed past its edge is inside the picture box and invisible — hence
    // inside the BAND too. And the shared top above reads only the cubes, so a
    // wrap that moves only a number goes unseen there — hence the line. And a
    // box inside the band is not yet a band that is SEEN — one painted under the
    // art, or hidden, keeps every box — hence the hit test just above, which
    // reads the band, not the picture, at each number's and each cube's centre
    // (6 of 6 at baseline; the review's two such mutants put the `IMG` there).
    // The glyphs' colour is the one thing about "drawn" no box or hit reads: it
    // is the stylesheet's `--vp` on the band's own ground, figured in its
    // comment.
    //
    // **The line is the cubes' vertical span, and a number lies within it** (±1
    // px). Measured before this was set (2026-11-20, every width alike): a cube
    // 21.27 px tall, a number 14.17 px, the band 27.52 px. A number is SHORTER
    // than a cube and `align-items: center` centres it on the cubes' line, so it
    // sits 3.55 px inside their span at top and bottom alike — the centre offset
    // read 0.00 px, which is the other form and holds too (recorded below, not
    // gated). (One run's page positions at 390 px, for the record only: the
    // cubes spanned 394.09–415.36 and each number 397.64–411.81; they move with
    // the scroll.) Containment is the form chosen
    // because it is the one a number on a second line cannot meet: dropped
    // under the cubes, its bottom passes theirs.
    const lineTop = Math.min(...read.claims.map((c) => c.top));
    const lineBottom = Math.max(...read.claims.map((c) => c.bottom));
    const centre = (r: Rect) => (r.top + r.bottom) / 2;
    for (const [name, vp] of [["first-place number", read.vpFirst!], ["later number", read.vpLater!]] as const) {
      expect(outside(vp, band), `${at}: the ${name} is not inside the band that clips it`).toBeNull();
      expect(vp.top, `${at}: the ${name}'s top is above the cubes' line (${lineTop.toFixed(2)})`).toBeGreaterThanOrEqual(lineTop - 1);
      expect(vp.bottom, `${at}: the ${name}'s bottom is below the cubes' line (${lineBottom.toFixed(2)})`).toBeLessThanOrEqual(lineBottom + 1);
    }
    const centreOff = Math.max(
      Math.abs(centre(read.vpFirst!) - (lineTop + lineBottom) / 2),
      Math.abs(centre(read.vpLater!) - (lineTop + lineBottom) / 2),
    );
    // eslint-disable-next-line no-console
    console.log(
      `claim band at ${width}px: cube ${read.claims[0]!.width.toFixed(2)}px wide · band ${band.width.toFixed(2)}x${band.height.toFixed(2)}px ` +
        `in a ${pic.width.toFixed(2)}px picture box of a ${read.face.width.toFixed(2)}px card · numbers ${read.vpFirst!.height.toFixed(2)}px tall, ` +
        `centred ${centreOff.toFixed(2)}px off the cubes' · document ${read.client}px (recorded)`,
    );
  }
  expect(watch.failures).toEqual([]);
});

test("each seat's flag row stays inside its seat at a phone's width and widens nothing", async ({ page }) => {
  const watch = watchForBreakage(page);
  // **Two seats, you and the bot**: the viewer holds exactly one seat, so the
  // frame's hand — and with it the `+n` cells, the widest a row gets — is on
  // one row, the case the design names; the other row is a seat whose hand is
  // hidden and draws its five cells bare. A few moves make it a running game.
  //
  // **The hand is rewritten to hold every country** (the whole-branch review,
  // MB-1). A six-move game's hand carries one or two countries, and a row with
  // one or two `+n` fits one line at 390 px whether the row wraps or not, so
  // the containment below could not tell `flex-wrap: wrap` from `nowrap`. With
  // a `+n` in all five cells the row does not fit one line (measured by the
  // review: 381.9 px at max-content against a 328.8 px seat), so only a row
  // that wraps stays inside its seat. As in the claim cell above, the rewrite
  // is of the game-state replies (`GET /api/games/{id}` and the move's `POST
  // …/input`, both a `ClientState`) and of nothing in them but the open Hand
  // panel's `flags_in_hand`: the rest stays the server's own.
  const FIVE_IN_HAND = { english: 1, american: 1, french: 1, german: 1, dutch: 2 };
  let rewritten = 0;
  // Cast, not annotated: the route assigns it, and TypeScript does not follow
  // an assignment inside a callback, so an annotated `null` stays narrowed.
  let handSeat = null as number | null;
  await page.route(
    (url) => /^\/api\/games\/[^/]+(\/input)?$/.test(url.pathname),
    async (route) => {
      const method = route.request().method();
      if (method !== "GET" && method !== "POST") return route.fallback();
      const response = await route.fetch();
      if (!response.ok()) return route.fulfill({ response });
      const body = (await response.json()) as { frame?: { dashboard?: Record<string, unknown>[] } };
      const panel = body.frame?.dashboard?.find((p) => "Hand" in p);
      const hand = panel?.Hand as { seat: number; hidden: boolean; flags_in_hand: unknown } | undefined;
      if (hand && !hand.hidden) {
        hand.flags_in_hand = { ...FIVE_IN_HAND };
        handSeat = hand.seat;
        rewritten += 1;
      }
      return route.fulfill({ response, json: body });
    },
  );
  await startGame(page, { players: 2 });
  await playAFew(page, 6, 0x464c_4147); // "FLAG"
  await page.setViewportSize({ width: 390, height: 844 });
  await settle(page);
  expect(rewritten, "no game-state reply's open hand was rewritten").toBeGreaterThan(0);

  const read = () =>
    page.evaluate(() => {
      const rect = (el: Element) => {
        const r = el.getBoundingClientRect();
        return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
      };
      return {
        inner: window.innerWidth,
        seats: [...document.querySelectorAll(".seat")].map((s) => ({
          seat: rect(s),
          rows: [...s.querySelectorAll(":scope .flags")].map(rect),
          cellBoxes: [...s.querySelectorAll(".flags > .flag")].map(rect),
          cells: s.querySelectorAll(".flags > .flag[data-country]").length,
          hand: s.querySelectorAll(".flags .flag-hand").length,
          // Each `+n`'s EFFECTIVE opacity: the product of its own computed
          // `opacity` and every ancestor's up to its `.seat` — what a reader's
          // eye gets, which no one element's value says.
          handOpacity: [...s.querySelectorAll(".flags .flag-hand")].map((h) => {
            let o = 1;
            for (let e: Element | null = h; e && e !== s; e = e.parentElement) o *= Number(getComputedStyle(e).opacity);
            return o;
          }),
        })),
      };
    });
  const before = await read();
  // Anti-vacuity: the page is a phone's width, each seat draws one row of five
  // cells, and the viewer's row — the hand's seat, the rows being in seat
  // order — carries a `+n` in all five, the rewritten hand; the other row
  // carries none.
  expect(before.inner, "the window is not 390px").toBe(390);
  expect(before.seats, "the players panel draws a row per seat").toHaveLength(2);
  for (const [i, s] of before.seats.entries()) {
    expect(s.rows, `seat ${i} draws no flag row, or more than one`).toHaveLength(1);
    expect(s.cells, `seat ${i}'s flag row is not five cells`).toBe(5);
  }
  expect(handSeat, "the rewritten hand names no seat").not.toBeNull();
  for (const [i, s] of before.seats.entries()) {
    expect(s.hand, `seat ${i} draws ${s.hand} flags in hand (the hand is seat ${handSeat}'s, five countries)`).toBe(i === handSeat ? 5 : 0);
  }
  // **A `+n` is never dimmed** (MB-5): a zero's word and count are, and the
  // cell is not, so a `+n` inside a zero's cell keeps its colour's contrast
  // (`styles.css`, the flag row's comment). Dimming the cell would multiply
  // the `+n` by it, which only the effective opacity sees.
  before.seats[handSeat!]!.handOpacity.forEach((o, j) => {
    expect(o, `seat ${handSeat}'s flag in hand ${j} is drawn at an effective opacity of ${o}`).toBe(1);
  });
  for (const [i, s] of before.seats.entries()) {
    const row = s.rows[0]!;
    expect(row.width > 0 && row.height > 0, `seat ${i}'s flag row has no size`).toBe(true);
    expect(s.seat.width, `seat ${i} has no width`).toBeGreaterThan(0);
    expect(outside(row, s.seat), `seat ${i}'s flag row is not inside its seat`).toBeNull();
    // And its cells inside it: a row is a block as wide as its seat, so a row
    // whose cells ran past it (a `nowrap` that cannot fit) would still pass
    // the line above.
    s.cellBoxes.forEach((c, j) => {
      expect(c.width > 0 && c.height > 0, `seat ${i}'s flag cell ${j} has no size (${c.width}x${c.height})`).toBe(true);
      expect(outside(c, row), `seat ${i}'s flag cell ${j} is not inside its row`).toBeNull();
    });
  }

  // The page delta, with the rows and without them.
  const withRows = await readOverflow(page);
  // Hidden through the CSSOM, one element at a time: the page's CSP refuses an
  // injected `<style>` (`style-src 'self'` and a hash), and a property set from
  // script is not an inline style sheet. Restored the same way.
  const setRows = (display: string) =>
    page.locator(".flags").evaluateAll((els, d) => els.forEach((e) => ((e as HTMLElement).style.display = d)), display);
  await setRows("none");
  await settle(page);
  expect(await page.locator(".flags").evaluateAll((els) => els.filter((e) => e.getClientRects().length > 0).length), "the rows did not hide").toBe(0);
  const withoutRows = await readOverflow(page);
  await setRows("");
  await settle(page);
  expect(await page.locator(".flags:visible").count(), "the rows did not come back").toBe(2);
  const overflowWith = withRows.docScrollWidth - withRows.docClientWidth;
  const overflowWithout = withoutRows.docScrollWidth - withoutRows.docClientWidth;
  expect(overflowWith, `the flag rows widen the page at 390px: ${blame(withRows)}`).toBeLessThanOrEqual(overflowWithout);
  // eslint-disable-next-line no-console
  console.log(
    `flag rows at 390px: page overflow ${overflowWith}px with the rows, ${overflowWithout}px without (gated: with <= without) · ` +
      before.seats
        .map((s, i) => `seat ${i} row ${s.rows[0]!.width.toFixed(2)}x${s.rows[0]!.height.toFixed(2)} in ${s.seat.width.toFixed(2)}px, ${s.hand} in hand`)
        .join(" · ") +
      " (recorded)",
  );
  expect(watch.failures).toEqual([]);
});

// ---- issue #77 (plan 2a): the goods choosers, reached and measured --------
//
// The Church's payment and Chinatown's exchanges are each ONE entry on the main
// list that opens a chooser in the answer bar (Tasks 5 and 6). jsdom drew both
// and has no layout; these cells reach each chooser through the screen, as a
// player does, measure its boxes at a phone's and a desktop's width, answer it
// through the screen, and drive the walks' own policy at it.

type ChooserArea = "Church" | "Chinatown";

/** The chooser's own class, beside `.chooser`. */
const CHOOSER_CLASS: Record<ChooserArea, string> = { Church: "church", Chinatown: "chinatown" };

/** The entry that opens it, as the plan's words section spells it. */
const CHOOSER_ENTRY: Record<ChooserArea, RegExp> = {
  Church: /Church — choose what to pay \(/,
  Chinatown: /Chinatown — choose exchanges \(up to /,
};

/**
 * Reach `area`'s chooser on a fresh table, through the screen: end the
 * additional phase, Option B's two assistants on the area's tile, the Movement
 * row naming the tile with the President, then the entry — the route the wire
 * corpus's own Chinatown capture takes (`wire_fixtures.rs`). The tile is found
 * by its label's printed name (`A05 Church · …`, `Board.tsx`'s load-bearing
 * prefix) and its id read off it, so the President's row is the one naming
 * THAT tile. Returns the tile's id. Any screen the route does not expect is an
 * error quoting the answer bar, never a guess.
 */
async function openChooser(page: Page, area: ChooserArea): Promise<string> {
  const chooser = page.locator(`.decision .chooser.${CHOOSER_CLASS[area]}`);
  let tile: string | null = null;
  for (let step = 0; step < 12; step++) {
    await readScreen(page);
    if (await chooser.count()) {
      if (tile === null) throw new Error(`the ${area} chooser opened before this route placed anything`);
      return tile;
    }
    if (await answer(page, CHOOSER_ENTRY[area])) continue;
    if (tile !== null && (await answer(page, new RegExp(`President.*\\(${tile}\\)`)))) continue;
    if (tile === null && (await answer(page, /Option B/))) {
      await expect(page.locator('.tile[role="button"]').first()).not.toHaveAttribute("aria-disabled", "true");
      const labels = await page
        .locator('.tile[role="button"]')
        .evaluateAll((els) => els.map((e) => e.getAttribute("aria-label") ?? ""));
      const at = labels.findIndex((l) => new RegExp(`^A\\d\\d ${area}( |$)`).test(l));
      if (at < 0) throw new Error(`no ${area} tile to place on: ${labels.map((l) => l.slice(0, 14)).join(" | ")}`);
      tile = labels[at]!.slice(0, 3);
      await page.locator('.tile[role="button"]').nth(at).click();
      expect((await readScreen(page)).pressed, `Option B did not take ${tile} ${area}`).toBe(1);
      await page.getByRole("button", { name: "Confirm" }).click();
      continue;
    }
    if (await answer(page, /End additional-action phase/)) continue;
    break;
  }
  throw new Error(
    `no scripted answer on the way to the ${area} chooser (tile ${tile ?? "not placed"}):\n` +
      (await page.locator(".decision:not(.notices)").first().innerText()),
  );
}

/** The window as a box, for "inside the viewport" (with `outside`'s 0.5 px). */
const VIEWPORT_OF = (w: number, h: number): Rect => ({ left: 0, top: 0, right: w, bottom: h, width: w, height: h });

/** Two boxes overlap when they share more than the rounding tolerance on both
 *  axes: kinds laid out in a grid with a 6px gap touch nowhere. */
function overlaps(a: Rect, b: Rect): boolean {
  const t = INSIDE_TOLERANCE_PX;
  return a.left < b.right - t && b.left < a.right - t && a.top < b.bottom - t && b.top < a.bottom - t;
}

/** The thumb's floor for a kind's height (the plan's cell 1): 44 px, WCAG's
 *  target size; the stylesheet's `min-height: 48px` sits above it. */
const KIND_MIN_HEIGHT_PX = 44;
/** A stepper's floor, both ways (the plan's cell 2): the stylesheet's 28 px,
 *  read with `outside`'s 0.5 px of rounding. */
const STEP_MIN_PX = 28;

/** The chooser's controls, read in ONE round trip: Confirm and Cancel found by
 *  their words anywhere in the answer bar — not by their place in it, so a
 *  Confirm that moved (into the rows' scrolling box, say) is still the one
 *  measured, and fails on where it is rather than on not being found. */
const controlsRead = () => {
  const rect = (el: Element) => {
    const r = el.getBoundingClientRect();
    return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
  };
  const named = (name: string) =>
    [...document.querySelectorAll<HTMLButtonElement>(".decision:not(.notices) button")].find(
      (b) => b.textContent?.trim() === name,
    ) ?? null;
  const confirm = named("Confirm");
  const cancel = named("Cancel");
  const hitAt = (el: Element | null) => {
    if (!el) return false;
    const r = el.getBoundingClientRect();
    const hit = document.elementFromPoint((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    return !!hit && (hit === el || el.contains(hit));
  };
  return {
    w: window.innerWidth,
    h: window.innerHeight,
    confirm: confirm ? rect(confirm) : null,
    cancel: cancel ? rect(cancel) : null,
    confirmHit: hitAt(confirm),
    cancelHit: hitAt(cancel),
  };
};

test("the Church's chooser lays its six kinds inside it, apart, thumb-tall and unclipped, with Confirm and Cancel in the frame, at 390 and 1440", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  await openChooser(page, "Church");
  for (const vp of [PHONE, DESKTOP]) {
    await page.setViewportSize(vp);
    await settle(page);
    const at = `at ${vp.width}x${vp.height}`;
    const read = await page.evaluate(() => {
      const rect = (el: Element) => {
        const r = el.getBoundingClientRect();
        return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
      };
      const chooser = document.querySelector(".decision .chooser.church")!;
      return {
        chooser: rect(chooser),
        kinds: [...chooser.querySelectorAll(".kinds > button.kind")].map((k) => {
          const lbl = k.querySelector(".lbl")!;
          return { good: k.getAttribute("data-good") ?? "", box: rect(k), scrollWidth: lbl.scrollWidth, clientWidth: lbl.clientWidth };
        }),
      };
    });
    const controls = await page.evaluate(controlsRead);
    expect(controls.w, `${at}: the window is not the width asked for`).toBe(vp.width);
    // Anti-vacuity: six kinds, each with a size, so no containment below passes
    // over a box that was never drawn.
    expect(read.kinds.map((k) => k.good), `${at}: the kinds`).toEqual(["Copper", "Silk", "Tea", "Seafood", "Yen", "Imports"]);
    for (const k of read.kinds) {
      expect(k.box.width > 0 && k.box.height > 0, `${at}: the ${k.good} kind has no size`).toBe(true);
      expect(outside(k.box, read.chooser), `${at}: the ${k.good} kind is not inside the chooser`).toBeNull();
      expect(k.box.height, `${at}: the ${k.good} kind is ${k.box.height.toFixed(2)}px tall`).toBeGreaterThanOrEqual(KIND_MIN_HEIGHT_PX);
      // 1px of slack: `clientWidth` is an integer and `scrollWidth` rounds up.
      expect(k.scrollWidth, `${at}: the ${k.good} kind's label is clipped (${k.scrollWidth} in ${k.clientWidth})`).toBeLessThanOrEqual(k.clientWidth + 1);
    }
    for (let i = 0; i < read.kinds.length; i++) {
      for (let j = i + 1; j < read.kinds.length; j++) {
        expect(overlaps(read.kinds[i]!.box, read.kinds[j]!.box), `${at}: ${read.kinds[i]!.good} overlaps ${read.kinds[j]!.good}`).toBe(false);
      }
    }
    const frame = VIEWPORT_OF(controls.w, controls.h);
    expect(controls.confirm, `${at}: no Confirm`).not.toBeNull();
    expect(controls.cancel, `${at}: no Cancel`).not.toBeNull();
    expect(outside(controls.confirm!, frame), `${at}: Confirm is not inside the viewport`).toBeNull();
    expect(outside(controls.cancel!, frame), `${at}: Cancel is not inside the viewport`).toBeNull();
  }
  expect(watch.failures).toEqual([]);
});

test("Chinatown's chooser keeps each stepper and first token inside its row, steppers 28px square, and Confirm pressable with the rows at their top, at 390 and 1440", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 4 });
  await openChooser(page, "Chinatown");
  for (const vp of [PHONE, DESKTOP]) {
    await page.setViewportSize(vp);
    await settle(page);
    const at = `at ${vp.width}x${vp.height}`;
    const read = await page.evaluate(() => {
      const rect = (el: Element) => {
        const r = el.getBoundingClientRect();
        return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
      };
      return [...document.querySelectorAll(".decision .chooser.chinatown .rows > .ex")].map((ex) => ({
        number: ex.getAttribute("data-number") ?? "",
        ex: rect(ex),
        step: rect(ex.querySelector(":scope > .step")!),
        minus: rect(ex.querySelector(".step > button.minus")!),
        plus: rect(ex.querySelector(".step > button.plus")!),
        token: rect(ex.querySelector(".what .tokens")!),
      }));
    });
    // Anti-vacuity: the nine exchanges, in the Sell-then-Buy order the panel draws.
    expect(read.map((r) => r.number), `${at}: the rows`).toEqual(["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
    for (const r of read) {
      expect(r.ex.width > 0 && r.ex.height > 0, `${at}: row ${r.number} has no size`).toBe(true);
      expect(outside(r.step, r.ex), `${at}: row ${r.number}'s stepper is not inside its row`).toBeNull();
      expect(outside(r.token, r.ex), `${at}: row ${r.number}'s first token is not inside its row`).toBeNull();
      for (const [name, b] of [["minus", r.minus], ["plus", r.plus]] as const) {
        expect(Math.min(b.width, b.height), `${at}: row ${r.number}'s ${name} is ${b.width.toFixed(2)}x${b.height.toFixed(2)}px`).toBeGreaterThanOrEqual(STEP_MIN_PX - INSIDE_TOLERANCE_PX);
      }
    }
    // With the rows scrolled to their top — the chooser as it opens — Confirm is
    // in the frame and is what a press at its centre lands on.
    await page.locator(".decision .chooser.chinatown .rows").evaluate((el) => (el.scrollTop = 0));
    await settle(page);
    const controls = await page.evaluate(controlsRead);
    expect(controls.confirm, `${at}: no Confirm`).not.toBeNull();
    expect(outside(controls.confirm!, VIEWPORT_OF(controls.w, controls.h)), `${at}: Confirm is not inside the viewport`).toBeNull();
    expect(controls.confirmHit, `${at}: a press at Confirm's centre lands on something else`).toBe(true);
  }
  expect(watch.failures).toEqual([]);
});

/** The verdict's `legal — ` tag, as the panel draws it (the plan's words). */
const LEGAL = "legal — ";

test("a Church payment through the screen: kinds pressed to a legal verdict, Confirm, and the turn's account holds its words", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 2 });
  await openChooser(page, "Church");
  const verdict = page.locator(".decision .chooser.church .verdict");
  // Anti-vacuity: the chooser opens on a verdict that is NOT legal (power 3
  // reaches no space on its own), so the words below are earned by a press.
  expect((await verdict.textContent()) ?? "", "the chooser opened already legal").not.toContain(LEGAL);
  let words: string | null = null;
  for (let press = 0; press < 6 && words === null; press++) {
    const kind = page.locator('.decision .chooser.church button.kind:not([disabled])[aria-pressed="false"]').first();
    expect(await kind.count(), "no kind left to press, and the verdict was never legal").toBe(1);
    await kind.click();
    await readScreen(page);
    const text = (await verdict.textContent()) ?? "";
    if (text.startsWith(LEGAL)) words = text.slice(LEGAL.length);
  }
  expect(words, "no selection of kinds read legal").not.toBeNull();
  await page.getByRole("button", { name: "Confirm" }).click();
  await readScreen(page);
  await expect(page.locator(".decision .chooser.church"), "the chooser closes on its commit").toHaveCount(0);
  await expect(page.locator(".turn-account .messages li").filter({ hasText: words! })).toHaveCount(1);
  expect(watch.failures).toEqual([]);
});

test("a Chinatown exchange through the screen on a four-player table: plus, legal, minus, Confirm disabled, plus, Confirm, and the account holds it", async ({ page }) => {
  const watch = watchForBreakage(page);
  await startGame(page, { players: 4 });
  await openChooser(page, "Chinatown");
  const rows = page.locator(".decision .chooser.chinatown .rows > .ex");
  const first = rows.filter({ has: page.locator("button.plus:not([disabled])") }).first();
  const number = await first.getAttribute("data-number");
  expect(number, "no exchange can be added").not.toBeNull();
  const row = page.locator(`.decision .chooser.chinatown .rows > .ex[data-number="${number}"]`);
  const rowWords = ((await row.locator(".words").textContent()) ?? "").trim();
  expect(rowWords, `row ${number} has no words`).not.toBe("");
  const verdict = page.locator(".decision .chooser.chinatown .verdict");
  const confirm = page.getByRole("button", { name: "Confirm" });

  await row.locator("button.plus").click();
  await readScreen(page);
  await expect(row.locator(".n")).toHaveText("1");
  await expect(verdict).toHaveClass(/\bok\b/);
  expect((await verdict.textContent()) ?? "").toMatch(new RegExp(`^${LEGAL}`));

  await row.locator("button.minus").click();
  await readScreen(page);
  await expect(row.locator(".n")).toHaveText("0");
  await expect(confirm).toBeDisabled();
  await expect(confirm).toHaveAttribute("data-unavailable", "true");

  await row.locator("button.plus").click();
  await readScreen(page);
  await expect(confirm).toBeEnabled();
  await confirm.click();
  await readScreen(page);
  await expect(page.locator(".decision .chooser.chinatown"), "the chooser closes on its commit").toHaveCount(0);
  await expect(
    page.locator(".turn-account .messages li").filter({ hasText: "Chinatown: " }).filter({ hasText: rowWords }),
  ).toHaveCount(1);
  expect(watch.failures).toEqual([]);
});

test("the walks' policy settles on a chooser and answers it: an add where Confirm is disabled, Confirm where nothing is left to add", async ({ page }) => {
  const watch = watchForBreakage(page);
  // Chinatown, opened: nothing chosen, so Confirm is disabled by the payload and
  // `readScreen`'s wait must still return.
  await startGame(page, { players: 4 });
  await openChooser(page, "Chinatown");
  const chosen = () =>
    page
      .locator(".decision .chooser.chinatown .rows > .ex .n")
      .evaluateAll((ns) => ns.reduce((sum, n) => sum + Number(n.textContent), 0));
  const town = await readScreen(page);
  expect(town.chooser, "readScreen does not see the chooser").toBe(true);
  expect(town.confirm, "Confirm is pressable with nothing chosen").toBe(false);
  expect(town.adds, "the chooser offers nothing to add").toBeGreaterThan(0);
  expect(town.adds, "adds counts the enabled pluses").toBe(await page.locator(".decision .chooser.chinatown button.plus:not([disabled])").count());
  expect(await chosen()).toBe(0);
  expect(await act(page, town, rng(0x4348_4f4f)), "the policy's answer at an opened chooser").toBe("chooser"); // "CHOO"
  await readScreen(page);
  expect(await chosen(), "the policy pressed an add, so one exchange is chosen").toBe(1);
  await expect(page.locator(".decision .chooser.chinatown"), "and the chooser is still open").toHaveCount(1);

  // The Church, with every offered kind pressed: nothing is left to add, so the
  // policy confirms — without asking the coin, which is asked only while an add
  // is still on offer.
  await startGame(page, { players: 2 });
  await openChooser(page, "Church");
  const unpressed = page.locator('.decision .chooser.church button.kind:not([disabled])[aria-pressed="false"]');
  for (let press = 0; press < 6 && (await unpressed.count()) > 0; press++) {
    await unpressed.first().click();
    await readScreen(page);
  }
  expect(await unpressed.count(), "an offered kind is still unpressed").toBe(0);
  const church = await readScreen(page);
  expect(church.chooser).toBe(true);
  expect(church.adds, "every offered kind is pressed").toBe(0);
  expect(church.confirm, "the whole offered set is not a payment the list holds").toBe(true);
  const noCoin = () => {
    throw new Error("the policy asked the coin with nothing left to add");
  };
  expect(await act(page, church, noCoin)).toBe("chooser");
  await readScreen(page);
  await expect(page.locator(".decision .chooser.church"), "the policy's Confirm closed the chooser").toHaveCount(0);
  await expect(page.locator(".turn-account .messages li").filter({ hasText: "Church: pay " })).toHaveCount(1);
  expect(watch.failures).toEqual([]);
});
