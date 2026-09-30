// **The README's screenshot generator** (Task 8; R-6.5c2.44). It is not a
// test: it plays a short table with three people and writes the TEN pictures
// of `worker/README.md` §1.4 under their names. Alice (`alice@readme.test`)
// opens a table named "Friday game" with a password, Ben (`ben@readme.test`)
// joins it, Carol (`carol@readme.test`) watches; before any of that, a scratch
// table Alice abandons under Ben's eyes gives the marker's picture.
//
// **Opt-in, like the exit's game**: collected only under
// `YOKOHAMA_WEB_README_SHOTS=1` (`playwright.config.ts`), and it runs on
// whichever target the run names —
//
//     # the dry run, against the local binary (writes into a scratch directory)
//     YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_README_SHOTS_DIR=<dir> npx playwright test readme_shots
//     # the committed pictures, on the Worker (the orchestrator's alone; the
//     # quota guard first — `worker/RUNBOOK.md` §7)
//     YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test readme_shots
//
// The output directory is `worker/docs/img/` unless
// `YOKOHAMA_WEB_README_SHOTS_DIR` names another. Playwright writes PNG only, so
// the five pictures the README names `.webp` are shot as PNG in a scratch
// directory and converted through Pillow (`docs/tools/mkhelpart.py`'s
// precedent); a run without Pillow FAILS, naming it.
//
// It is a live spec for `live.node.test.ts`'s rules: every `request.*` through
// `liveRequest` (there are none), every create armed through `awaitCreate`, and
// the table abandoned in `afterEach`. Live the tables land in the shared lobby
// under the `readme.test` addresses, and are taken away.

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, rmdirSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Locator, Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { signIn } from "./harness";
import { abandonOpened, awaitCreate, createdTable, liveContextOptions, type Opening } from "./live";
import { expect, newContextWithoutWorker, test } from "./sw";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const DESKTOP = { width: 1280, height: 832 };
const PHONE = { width: 390, height: 844 };
const PASSWORD = "sesame";

/** Where the pictures go: the README's own directory, or the run's scratch. */
const OUT = process.env.YOKOHAMA_WEB_README_SHOTS_DIR ?? fileURLToPath(new URL("../../worker/docs/img", import.meta.url));

/** The table this run is opening — the module's one, as `live.node.test.ts`
 *  requires; the scratch table is abandoned on screen and then set to null. */
let opened: Opening | null = null;

test.afterEach(async () => {
  const table = opened;
  opened = null;
  await abandonOpened(table);
});

/** A page whose layout is done and whose fetches have ended. */
async function settled(page: Page): Promise<void> {
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  await page.waitForFunction(() => !document.querySelector("[data-busy]"), null, { timeout: 30_000 });
  await page.waitForTimeout(400);
}

/** Pillow's conversion of one PNG to a WebP at quality 85. */
function toWebp(png: string, webp: string): void {
  try {
    execFileSync(
      "python3",
      ["-c", "from PIL import Image; import sys; Image.open(sys.argv[1]).save(sys.argv[2], 'WEBP', quality=85)", png, webp],
      { stdio: "pipe" },
    );
  } catch (e) {
    throw new Error(
      `the WebP conversion needs python3 with Pillow (pip install Pillow): ${e instanceof Error ? e.message.split("\n", 1)[0] : String(e)}`,
    );
  }
}

/** Shoots `page` as the README's `name` (a `.png` or a `.webp`). */
async function shoot(page: Page, name: string, scratch: string, mask: Locator[] = []): Promise<string> {
  await settled(page);
  const target = join(OUT, name);
  if (name.endsWith(".webp")) {
    const png = join(scratch, name.replace(/\.webp$/, ".png"));
    await page.screenshot({ path: png, animations: "disabled", mask });
    toWebp(png, target);
    unlinkSync(png);
  } else {
    await page.screenshot({ path: target, animations: "disabled", mask });
  }
  return target;
}

/** A context for `email`, at the README's window size, through the door the
 *  target has: the dev door live, and the open door's sign-in form locally. */
async function person(browser: Parameters<typeof newContextWithoutWorker>[0], email: string): Promise<Page> {
  const options = await liveContextOptions(email);
  const context = await newContextWithoutWorker(browser, { ...options, viewport: DESKTOP });
  return context.newPage();
}

/** Alice's New table form, filled and sent: the table's id. `password` locks it. */
async function openTable(alice: Page, name: string, password: string | null): Promise<string> {
  await alice.goto("/#/lobby");
  await alice.getByRole("button", { name: "New table" }).click();
  await expect(alice.getByRole("heading", { name: "New table", exact: true, level: 3 })).toBeVisible();
  await alice.getByRole("combobox", { name: "Blue seat", exact: true }).selectOption("open");
  await alice.getByLabel("Name", { exact: true }).fill(name);
  if (password !== null) {
    await alice.getByLabel("Ask for a password to join or spectate", { exact: true }).check();
    await alice.getByLabel("Table password", { exact: true }).fill(password);
  }
  opened = awaitCreate(alice);
  await alice.getByRole("button", { name: "Open table" }).click();
  const id = await createdTable(opened);
  await expect(alice).toHaveURL(new RegExp(`#/t/${id}$`));
  await expect(alice.getByLabel("Invite link", { exact: true })).toBeVisible();
  return id;
}

test("the README's ten pictures", async ({ browser }) => {
  test.setTimeout(5 * 60 * 1000);
  mkdirSync(OUT, { recursive: true });
  const scratch = mkdtempSync(join(tmpdir(), "readme-shots-"));
  const written: string[] = [];

  const alice = await person(browser, "alice@readme.test");
  const ben = await person(browser, "ben@readme.test");
  const carol = await person(browser, "carol@readme.test");

  // ---- the marker: a scratch table Alice abandons under Ben's eyes ---------
  await signIn(alice, "alice@readme.test");
  await signIn(ben, "ben@readme.test");
  const scratchId = await openTable(alice, "Scratch table", null);
  await ben.goto(`/#/t/${scratchId}`);
  await expect(ben.getByRole("heading", { name: "Scratch table", exact: true, level: 3 })).toBeVisible();
  await alice.getByRole("button", { name: /^abandon( and save)? Scratch table$/ }).click();
  await alice.getByRole("button", { name: /^confirm abandon( and save)? Scratch table$/ }).click();
  await expect(alice).toHaveURL(/#\/lobby$/);
  opened = null;
  await expect(ben.getByText(/was abandoned/)).toBeVisible({ timeout: 30_000 });
  const markerPath = await shoot(ben, "marker-404.png", scratch);
  written.push(markerPath);

  // ---- the door ------------------------------------------------------------
  await alice.goto("/#/");
  await expect(alice.getByRole("heading", { name: "Yokohama", level: 1 })).toBeVisible();
  await expect(alice.locator(".door-cards")).toBeVisible();
  written.push(await shoot(alice, "door-desktop.png", scratch));
  await alice.setViewportSize(PHONE);
  written.push(await shoot(alice, "door-phone.png", scratch));
  await alice.setViewportSize(DESKTOP);

  // ---- Friday game: Alice opens it, locked, one seat open ------------------
  const id = await openTable(alice, "Friday game", PASSWORD);
  written.push(await shoot(alice, "open-table-desktop.png", scratch));

  // ---- Ben's lobby lists it ------------------------------------------------
  await ben.goto("/#/lobby");
  await ben.reload();
  await expect(ben.getByText("Friday game").first()).toBeVisible({ timeout: 30_000 });
  written.push(await shoot(ben, "lobby-desktop.png", scratch));

  // ---- Ben, at the locked table --------------------------------------------
  await ben.goto(`/#/t/${id}`);
  await expect(ben.getByRole("heading", { name: "Friday game", exact: true, level: 3 })).toBeVisible();
  await ben.getByLabel("Table password", { exact: true }).fill(PASSWORD);
  written.push(await shoot(ben, "join-desktop.png", scratch));
  await ben.getByRole("button", { name: "Join as Blue" }).click();

  // ---- the deal: Alice chooses her card ------------------------------------
  await expect(alice.locator('.deal-seat[data-seat="1"]')).toContainText("ben@readme.test", { timeout: 30_000 });
  const keepA = alice.getByRole("button", { name: /^keep O\d\d/ }).first();
  await expect(keepA).toBeVisible({ timeout: 30_000 });
  written.push(await shoot(alice, "deal-desktop.webp", scratch));
  for (const p of [alice, ben]) {
    const keep = p.getByRole("button", { name: /^keep O\d\d/ }).first();
    await expect(keep).toBeVisible({ timeout: 30_000 });
    await keep.click();
  }
  await expect(alice.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  await expect(ben.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });

  // ---- Alice's first turn, at desktop width and at a phone's ---------------
  written.push(await shoot(alice, "game-desktop-player.webp", scratch));
  await alice.setViewportSize(PHONE);
  written.push(await shoot(alice, "game-phone.webp", scratch));
  await alice.setViewportSize(DESKTOP);

  // ---- Carol watches -------------------------------------------------------
  await signIn(carol, "carol@readme.test", `/#/t/${id}`);
  await carol.getByLabel("Table password", { exact: true }).fill(PASSWORD);
  await carol.getByRole("button", { name: "Spectate" }).click();
  await expect(carol.getByText("You are spectating")).toBeVisible({ timeout: 30_000 });
  written.push(await shoot(carol, "game-desktop-watcher.webp", scratch));

  rmdirSync(scratch);
  expect(written, "ten pictures").toHaveLength(10);
  // eslint-disable-next-line no-console
  console.log(`readme_shots: wrote ${written.length} pictures into ${OUT}`);
});
