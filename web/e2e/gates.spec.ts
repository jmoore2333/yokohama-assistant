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
    await page.locator(".tile").first().hover();
    await expect(page.getByRole("tooltip")).toBeVisible();
    await page.screenshot({ path: `${OUT}/v1-${players}p-popover.png` });
  });
}

// The V2 flow (M5 spec §4.1's second gate): the finished table, in the five
// states worth looking at — every section on at the three viewports, the two
// offers alone, a drawer over it, a replay position with the fetched sections
// collapsed, and the Game menu open. Same discipline as V1, a different stream
// so the two gates do not photograph the same fourteen moves.
//
// **Two things about the camera that the first run forced, both measured.**
//
// (1) `park()`. `playAFew` ends on a click, so the pointer is left resting on a
// tile and `TilePopover` opens under it — the first V2 run photographed a
// "Laboratory A" popover sitting over the middle of the 2P table at every
// viewport. V1 wants that popover and shoots it deliberately; here it is a lens
// cap. The pointer goes to the banner's top-left corner before every shot.
//
// (2) `showSections()`. Above 1201px `.app` is `height: 100vh` and `.table` is
// its own scroll container (`styles.css`), so the page does not scroll and the
// sections sit under a pyramid that has been FITTED to the table — which is the
// design (owner ruling R2), and which means a shot at the top of the table
// contains no section at all. Measured with all five on: `.table` scrolls
// 2290–3679px against a client height of 785–1356, and the section block starts
// 586–844px below the fold at every one of the three viewports. So the shots
// whose subject IS the sections scroll the table until the block's top sits
// ~140px down, keeping the pyramid's foot in frame; the shots whose subject is
// the banner or the drawer stay at the top. The pyramid at these same three
// viewports is what the V1 pictures are, and this run rewrites those too.
const park = (page: Page) => page.mouse.move(2, 2);

/** Scroll `.table` until the section block's top is ~140px down. */
async function showSections(page: Page) {
  await page.evaluate(() => {
    const table = document.querySelector(".table") as HTMLElement | null;
    const secs = document.querySelector(".table-sections") as HTMLElement | null;
    if (!table || !secs) return;
    const delta = secs.getBoundingClientRect().top - table.getBoundingClientRect().top - 140;
    const max = table.scrollHeight - table.clientHeight;
    table.scrollTop = Math.max(0, Math.min(max, table.scrollTop + delta));
  });
  await raf(page);
}

for (const players of [2, 4] as const) {
  test(`V2 — the table at ${players}P: all sections, offers only, a drawer, a replay, the menu`, async ({ page }) => {
    mkdirSync(OUT, { recursive: true });
    await page.setViewportSize({ width: 1710, height: 869 });
    await startGame(page, { players });
    await playAFew(page, 14, 0x5441_424c + players); // "TABL"
    const toggle = async (...names: string[]) => {
      await page.locator(".boards-menu > summary").click();
      for (const name of names) await page.getByRole("button", { name }).click();
      await page.keyboard.press("Escape");
    };
    await toggle("Building sites");
    await expect(page.locator(".table-sec.buildings")).toBeVisible();
    for (const [w, h] of VIEWPORTS) {
      await page.setViewportSize({ width: w, height: h });
      await raf(page);
      await showSections(page);
      await park(page);
      await page.screenshot({ path: `${OUT}/v2-${players}p-all-${w}x${h}.png` });
    }
    await page.setViewportSize({ width: 1710, height: 869 });
    await toggle("Tracks", "Goals", "Building sites");
    await showSections(page);
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-offers-only.png` });
    await page.getByRole("button", { name: "Areas", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.evaluate(() => ((document.querySelector(".table") as HTMLElement).scrollTop = 0));
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-drawer.png` });
    await page.keyboard.press("Escape");
    // Tracks and Goals go back on for the replay shot, and that is the whole
    // point of the picture: `TableSections`' `FETCHED` list is ports/labs/
    // buildings, so those three collapse to "live offer, hidden while replaying"
    // while Tracks and Goals keep drawing from the historical frame. Shot with
    // only the two offers on — where the flow had left it — the replay picture
    // is two one-line placeholders and no contrast at all.
    await toggle("Tracks", "Goals");
    await page.getByRole("button", { name: "Log", exact: true }).click();
    const track = page.getByLabel("Replay position");
    await track.focus();
    for (let i = 0; i < 4; i++) await page.keyboard.press("ArrowLeft");
    await expect(page.locator(".table.replaying")).toBeVisible();
    await showSections(page);
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-replay.png` });
    await page.keyboard.press("Escape");
    await page.locator(".game-menu > summary").click();
    await page.evaluate(() => ((document.querySelector(".table") as HTMLElement).scrollTop = 0));
    await park(page);
    await page.screenshot({ path: `${OUT}/v2-${players}p-menu.png` });
  });
}
