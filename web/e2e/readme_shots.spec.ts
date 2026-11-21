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
// **A local-only stranger** (the whole-branch review, W-6): under
// `YOKOHAMA_WEB_README_SHOTS_STRANGER=1` a fourth person, Dave, opens a table of
// his own and posts one lobby chat line before any lobby picture, so the
// redaction meets a real stranger's row and a real chat message in a real
// browser before a live run ever does. It is REFUSED live: a live run never
// adds a table or a message to the shared lobby that is not the cut's own.
//
//     YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_README_SHOTS_STRANGER=1 YOKOHAMA_WEB_README_SHOTS_DIR=<dir> npx playwright test readme_shots
//
// **A failed run that can meet a stranger writes none of the page** (the
// whole-branch review, W-2, and its re-review). Playwright 1.62.1 has three
// ways to put a failed run's page on disk, and this file closes each:
//
//   1. `test-results/<test>/error-context.md` gets a `# Page snapshot` — an
//      unredacted accessibility snapshot of the first context's first page —
//      unless `PLAYWRIGHT_NO_COPY_PROMPT` is set. It is read from `process.env`
//      in the worker when a context closes and when the test finishes, so this
//      module's top, which the worker loads before it runs the test, is in time.
//   2. A locator assertion that fails unmatched carries a snapshot of the WHOLE
//      page on its error (`matcherResult.ariaSnapshot`), which goes into that
//      same file under `# Error details` whatever the variable says, and the
//      assertion's message prints what it received to the terminal. So on a
//      run that can meet a stranger — live, or the local stranger arm — the
//      test body catches any failure and throws a fresh `Error` in its place,
//      naming only the last picture the run took, a name this file sets: a
//      fresh `Error` carries no `matcherResult`, so nothing of the page rides
//      on it. The original is withheld because it can quote the page. On an
//      ordinary local run (a lobby holding only the run's own tables) it is
//      rethrown unchanged, so a developer can still read it.
//   3. The config's `screenshot: "only-on-failure"` shoots every open page,
//      unmasked; `test.use` below turns screenshots, video and the trace off
//      for this file, and `play` sends every page to `about:blank` in its
//      `finally`, before anything after the test body can photograph or
//      snapshot one.
//
// What remains on such a run: `error-context.md` with the withheld message and
// a frame of this file's own source, the reporter's line for the same message,
// and `.last-run.json`'s test id.
//
// It is a live spec for `live.node.test.ts`'s rules: every `request.*` through
// `liveRequest` (there are none), every create armed through `awaitCreate`, and
// the table abandoned in `afterEach`. Live the tables land in the shared lobby
// under the `readme.test` addresses, and are taken away.

import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdirSync, mkdtempSync, readdirSync, rmdirSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { Locator, Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { redactSidecar } from "./aria_redact";
import { signIn, who } from "./harness";
import { LIVE, abandonOpened, awaitCreate, createdTable, liveContextOptions, type Opening } from "./live";
import { expect, newContextWithoutWorker, test } from "./sw";

// **Before the first test, so a run cannot forget it** (W-2; the header says
// when Playwright reads each).
process.env.PLAYWRIGHT_NO_COPY_PROMPT = "1";
test.use({ screenshot: "off", video: "off", trace: "off" });

/** The local-only stranger arm (the header). */
const STRANGER = process.env.YOKOHAMA_WEB_README_SHOTS_STRANGER === "1";
if (STRANGER && LIVE) {
  throw new Error("readme_shots: YOKOHAMA_WEB_README_SHOTS_STRANGER=1 is local only — a live run opens no stranger's table in the shared lobby");
}

/** A run whose pages can hold a stranger's row or message — live, or the local
 *  stranger arm — and whose failure is therefore reported without its error
 *  (the header, way 2). */
const WITHHOLD = LIVE || STRANGER;

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

const DESKTOP = { width: 1280, height: 832 };
const PHONE = { width: 390, height: 844 };
/** **Drawn per run, never written down** (the whole-branch review, W-3): the
 *  repository is public and the lobby lists the locked table to everyone, so a
 *  committed password would be a key any reader holds to the table's seat, its
 *  watching and its chat for the length of the cut. The README's alts do not
 *  quote it, the join picture's field draws dots, and the table is abandoned
 *  at the end. */
const PASSWORD = randomBytes(12).toString("base64url");

/** Where the pictures go: the README's own directory, or the run's scratch. */
const OUT = process.env.YOKOHAMA_WEB_README_SHOTS_DIR ?? fileURLToPath(new URL("../../worker/docs/img", import.meta.url));

/** Where each picture's accessibility snapshot goes, as `<base name>.aria.txt`:
 *  the gitignored `web/help-shots/readme/`, whatever `OUT` is. Not beside the
 *  picture, because `OUT` is a committed directory (or a scratch one a dry run
 *  may discard) and the PNG intermediates' scratch is removed when the run ends.
 *  It is how the README's alts are checked without opening a picture, so an
 *  agent reads it. **It leaves out what the picture masks, for the same
 *  reason, and every chat log besides**: every lobby row that is not one of
 *  this run's tables is replaced by one `[masked: not this run's table]` line,
 *  keyed by the same row link the mask is; every chat panel's log — folded in
 *  the picture, but still in the accessibility tree, messages and all — by one
 *  `[masked: chat]` line; and the text is refused, unwritten, if a table id or
 *  an address the run did not make still survives in it (`aria_redact.ts`). */
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
 *  `masked` covers left out (`aria_redact.ts`). Three reads in this order: the
 *  screenshot (`shoot`), then the snapshot, then the masked rows' links off the
 *  page. **The order is what makes it fail closed**: a row that arrives between
 *  the snapshot and the link read is in the list and not in the text, which
 *  costs nothing; a row that leaves in between keeps its link in the text with
 *  nothing to redact it, and the guard refuses the file. An empty snapshot, a
 *  masked row with no table link to key it by, or a foreign id or address left
 *  in the text fails the run rather than writing the file. */
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

/** The local-only stranger (the header): Dave opens "Dave's table", its Blue
 *  seat open, through the New table form — but not through `openTable`, which
 *  would count it among the run's own — and posts one line in the lobby chat.
 *  Locally the server's directory is emptied at launch, so nothing is left. */
async function strangerInTheLobby(dave: Page): Promise<void> {
  await signIn(dave, who("dave", "readme"));
  await dave.goto("/#/lobby");
  await dave.getByRole("button", { name: "New table" }).click();
  await expect(dave.getByRole("heading", { name: "New table", exact: true, level: 3 })).toBeVisible();
  await dave.getByRole("combobox", { name: "Blue seat", exact: true }).selectOption("open");
  await dave.getByLabel("Name", { exact: true }).fill("Dave's table");
  opened = awaitCreate(dave);
  await dave.getByRole("button", { name: "Open table" }).click();
  await createdTable(opened);
  // Local only, so there is nothing to abandon; and `opened` is the run's own.
  opened = null;
  await dave.goto("/#/lobby");
  const chat = dave.locator("section.panel.chat");
  await expect(chat).toHaveCount(1);
  await chat.getByRole("button", { name: "show" }).click();
  await chat.getByLabel("Say something", { exact: true }).fill("hello from dave");
  await chat.getByRole("button", { name: "send" }).click();
  await expect(chat.getByText("hello from dave")).toBeVisible({ timeout: 30_000 });
}

test("the README's ten pictures", async ({ browser }) => {
  test.setTimeout(5 * 60 * 1000);
  mkdirSync(OUT, { recursive: true });
  const scratch = mkdtempSync(join(tmpdir(), "readme-shots-"));
  const written: string[] = [];
  try {
    await play(browser, scratch, written);
  } catch (e) {
    if (!WITHHOLD) throw e;
    // A fresh error, from a literal and the last picture's name alone (the
    // header, way 2): never the caught error, its message or its matcher's
    // snapshot, which can quote the page.
    const where = written.length === 0 ? "before its first picture" : `after ${basename(written[written.length - 1]!)}`;
    throw new Error(`readme_shots: the run failed ${where}; its error is withheld, since an assertion's error can quote the page and this run's pages can hold a stranger's row or message`);
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
  /** Every page the run opens, so the `finally` can empty each one. */
  const pages: Page[] = [];
  const open = async (email: string): Promise<Page> => {
    const page = await person(browser, email);
    pages.push(page);
    return page;
  };
  try {
    const alice = await open(ALICE);
    const ben = await open(BEN);
    const carol = await open(CAROL);
    if (STRANGER) await strangerInTheLobby(await open(who("dave", "readme")));

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
  } finally {
    // **Every page to `about:blank` before anything else can photograph or
    // snapshot it** (W-2): whatever Playwright takes after the test body, or a
    // later option turns on, then sees an empty page. The afterEach's abandon
    // goes through `page.request`, which reads no page's address.
    await Promise.all(pages.map((p) => p.goto("about:blank").catch(() => undefined)));
  }
}
