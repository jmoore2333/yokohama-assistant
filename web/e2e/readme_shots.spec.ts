// **The README's screenshot generator** (Task 8; R-6.5c2.44). It is not a
// test: it plays a short table with three people and writes the TEN pictures
// of `worker/README.md` §1.4 under their names. Alice (`alice@readme.test`)
// opens a table named "Friday game" with a password, Ben (`ben@readme.test`)
// joins it, Carol (`carol@readme.test`) watches; before any of that, a scratch
// table Alice abandons under Ben's eyes gives the marker's picture. Those are
// the LIVE identities, through the Worker's dev door; `who` spells each for the
// target (security S2b, C1), so at the local binary's open door they are the
// plain names `alice`, `ben` and `carol`.
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
import { mkdirSync, mkdtempSync, readdirSync, rmdirSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Locator, Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { redactSidecar } from "./aria_redact";
import { signIn, who } from "./harness";
import { abandonOpened, awaitCreate, createdTable, liveContextOptions, type Opening } from "./live";
import { expect, newContextWithoutWorker, test } from "./sw";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const DESKTOP = { width: 1280, height: 832 };
const PHONE = { width: 390, height: 844 };
const PASSWORD = "sesame";

/** Where the pictures go: the README's own directory, or the run's scratch. */
const OUT = process.env.YOKOHAMA_WEB_README_SHOTS_DIR ?? fileURLToPath(new URL("../../worker/docs/img", import.meta.url));

/** Where each picture's accessibility snapshot goes, as `<base name>.aria.txt`:
 *  the gitignored `web/help-shots/readme/`, whatever `OUT` is. Not beside the
 *  picture, because `OUT` is a committed directory (or a scratch one a dry run
 *  may discard) and the PNG intermediates' scratch is removed when the run ends.
 *  It is how the README's alts are checked without opening a picture, so an
 *  agent reads it. **It leaves out exactly what the picture masks, for the
 *  same reason**: every
 *  lobby row that is not one of this run's tables is replaced by one
 *  `[masked: not this run's table]` line, keyed by the same row link the mask
 *  is, and the text is refused, unwritten, if a table id or an address the run
 *  did not make still survives in it (`aria_redact.ts`). */
const ARIA = fileURLToPath(new URL("../help-shots/readme", import.meta.url));

/** The sidecar's path for the README picture `name`. */
const ariaPath = (name: string): string => join(ARIA, `${name.replace(/\.(png|webp)$/, "")}.aria.txt`);

/** Remove `name`'s earlier sidecar, if there is one, before its picture is
 *  taken: a run that fails after the screenshot then leaves no older text
 *  beside the newer picture, and a dry run's text cannot pass for a live run's
 *  at a picture this run re-took. */
function forgetAria(name: string): void {
  try {
    unlinkSync(ariaPath(name));
  } catch (e) {
    if ((e as NodeJS.ErrnoException).code !== "ENOENT") throw e;
  }
}

/** Write `page`'s accessibility snapshot as `ariaPath(name)`, with every row
 *  `masked` covers left out (`aria_redact.ts`). The masked rows' links are read
 *  off the page at the same moment as the snapshot. An empty snapshot, a masked
 *  row with no table link to key it by, or a foreign id or address left in the
 *  text fails the run rather than writing the file. */
async function ariaBeside(page: Page, name: string, masked: Locator): Promise<void> {
  const aria = await page.locator("body").ariaSnapshot();
  if (aria.trim() === "") throw new Error(`readme_shots: the accessibility snapshot for ${name} is empty`);
  const hrefs = await masked.evaluateAll((rows) => rows.map((r) => r.querySelector("a.row-link")?.getAttribute("href") ?? null));
  if (hrefs.some((h) => h === null)) {
    throw new Error(`readme_shots: a masked lobby row in ${name} has no table link, so its sidecar cannot leave it out; nothing was written`);
  }
  const people = ["alice", "ben", "carol"].map((n) => who(n, "readme"));
  const text = redactSidecar(aria, hrefs as string[], { ids: ownTables, addresses: people });
  mkdirSync(ARIA, { recursive: true });
  writeFileSync(ariaPath(name), `${text}\n`);
}

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

/** The ids of the tables this run opened: the only lobby rows left unmasked. */
const ownTables = new Set<string>();

/** Shoots `page` as the README's `name` (a `.png` or a `.webp`).
 *
 *  **Every lobby row that is not one of THIS run's tables is masked, by the
 *  table's id in its link** (never by text: the spec's own row shows a name and
 *  no address): live, the lobby is SHARED, and a stranger's table with its seat
 *  addresses must never reach a committed picture. A screen with no lobby row masks nothing. */
async function shoot(page: Page, name: string, scratch: string): Promise<string> {
  await settled(page);
  const own = [...ownTables].map((id) => `a.row-link[href$="#/t/${id}"]`).join(", ");
  const masked = page.locator(own === "" ? "li.lobby-row" : `li.lobby-row:not(:has(${own}))`);
  const mask = [masked];
  const target = join(OUT, name);
  forgetAria(name);
  if (name.endsWith(".webp")) {
    const png = join(scratch, name.replace(/\.webp$/, ".png"));
    await page.screenshot({ path: png, animations: "disabled", mask });
    await ariaBeside(page, name, masked);
    toWebp(png, target);
    unlinkSync(png);
  } else {
    await page.screenshot({ path: target, animations: "disabled", mask });
    await ariaBeside(page, name, masked);
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
  ownTables.add(id);
  await expect(alice).toHaveURL(new RegExp(`#/t/${id}$`));
  await expect(alice.getByLabel("Invite link", { exact: true })).toBeVisible();
  return id;
}

test("the README's ten pictures", async ({ browser }) => {
  test.setTimeout(5 * 60 * 1000);
  mkdirSync(OUT, { recursive: true });
  const scratch = mkdtempSync(join(tmpdir(), "readme-shots-"));
  const written: string[] = [];
  try {
    await play(browser, scratch, written);
  } finally {
    // The scratch directory holds only the PNG intermediates: each file by
    // its explicit path, then the empty directory (never a recursive remove).
    for (const f of readdirSync(scratch)) unlinkSync(join(scratch, f));
    rmdirSync(scratch);
  }
  expect(written, "ten pictures").toHaveLength(10);
  // eslint-disable-next-line no-console
  console.log(`readme_shots: wrote ${written.length} pictures into ${OUT}`);
});

async function play(browser: Parameters<typeof person>[0], scratch: string, written: string[]): Promise<void> {
  const ALICE = who("alice", "readme");
  const BEN = who("ben", "readme");
  const CAROL = who("carol", "readme");
  const alice = await person(browser, ALICE);
  const ben = await person(browser, BEN);
  const carol = await person(browser, CAROL);

  // ---- the marker: a scratch table Alice abandons under Ben's eyes ---------
  await signIn(alice, ALICE);
  await signIn(ben, BEN);
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
  await expect(alice.locator('.deal-seat[data-seat="1"]')).toContainText(BEN, { timeout: 30_000 });
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
  await signIn(carol, CAROL, `/#/t/${id}`);
  await carol.getByLabel("Table password", { exact: true }).fill(PASSWORD);
  await carol.getByRole("button", { name: "Spectate" }).click();
  await expect(carol.getByText("You are spectating")).toBeVisible({ timeout: 30_000 });
  written.push(await shoot(carol, "game-desktop-watcher.webp", scratch));
}
