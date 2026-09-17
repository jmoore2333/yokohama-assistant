// **The gate a LINK opens: a person who was told nothing but an address sits
// down at a locked table and plays.**
//
// `two_browsers.spec.ts` proves two people can play a table one of them named
// the other at — the seats were bound at create time, and the second browser
// found its table in its own lobby. This is the other arrival, and it is the
// one M6.2 is for: seat 1 is OPEN, the table has a password on it, and the
// second person reaches it by the address alone. B NEVER VISITS THE LOBBY, and
// that is deliberate — a walk that went through the list would be testing the
// list a second time, while the question here is whether the address is enough.
//
// Three things this can see that nothing else can:
//   1. The open table's own screen, from the OUTSIDE: a stranger's seat picker
//      with a name and a password, and the refusal a wrong password earns.
//   2. That the CREATOR's screen learns the seat filled WITHOUT A RELOAD — the
//      3 s poll driving a screen nobody touched.
//   3. That the table plays, from both browsers, once it fills.
//
// The `framenavigated` listener behind (2) is measured rather than assumed: on
// a real http origin Chromium reports a same-document hash change and a
// `history.pushState` through that event as well as a reload, so an empty list
// says A neither reloaded NOR changed its address while B sat down. (A must not
// change it: `route.ts`'s `navigate` is guarded against writing the hash it
// already holds, and `App`'s hand-over re-opens the id that is already on
// screen.)

import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, blame, deadEnd, readOverflow, readScreen, rng, signIn, watchForBreakage } from "./harness";
import type { Screen } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** How many moves the two of them play between them. Enough to say the table
 *  really is a game and that both browsers can answer it; deliberately not a
 *  whole game, which is `two_browsers.spec.ts`'s job and its twelve minutes. */
const MOVES = 6;

/** Consecutive looks at two screens with nothing pressable on either before the
 *  walk gives up. A turn boundary costs a few — the other browser has not
 *  polled yet, or is playing the turn back — and a run of them is nobody
 *  moving. */
const IDLE_LIMIT = 60;

/**
 * Whether this screen has a move on it, which is NOT the same as "not inert".
 *
 * `two_browsers.spec.ts` carries the same predicate and its doc carries the
 * measurement: a screen playing back the other seat's turn is neither watching
 * nor locked, so `inert` is `null` there while there is nothing to click. Read
 * from what can actually be pressed, so a walk in flight reads as "not yet"
 * rather than as a dead end.
 */
function hasMove(s: Screen): boolean {
  return s.inert === null && (s.options > 0 || s.tiles > 0 || s.counts > 0 || s.confirm || s.only);
}

test("a person holding only a link joins a locked open table and plays", async ({ browser }) => {
  test.setTimeout(5 * 60 * 1000);
  const started = Date.now();
  const a = await (await browser.newContext()).newPage();
  const b = await (await browser.newContext()).newPage();
  const watchA = watchForBreakage(a);
  // **B's wrong password is a 403 BY DESIGN**, and the watch counts every 4xx
  // from this origin as breakage — so this one refusal is opted into by name,
  // exactly as the containment cell opts into `/result`'s 409. Scoped to
  // `/join`: a 403 anywhere else on B's screen is still a finding.
  const watchB = watchForBreakage(b, [{ status: 403, urlIncludes: "/join" }]);

  // A opens a table with ONE SEAT OPEN and a password on it. The submit says
  // "Open table" rather than "Deal", which is the form telling the truth about
  // what it is about to do.
  await signIn(a, "a@e2e.test");
  await expect(a.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  await a.getByRole("button", { name: "New table" }).click();
  await expect(a.getByRole("heading", { name: "New game" })).toBeVisible();
  // **`exact: true` on all three, and each for its own collision.**
  // `getByLabel` matches by substring and case-insensitively: "Seat 1" also
  // matches the "Seat 1 email" field an email row reveals, "Name" also matches
  // "Your name", and the password box is labelled "Table password" precisely so
  // that it and the checkbox beside it ("Ask for a password to join") are asked
  // about by different names.
  await a.getByLabel("Seat 1", { exact: true }).selectOption("open");
  await a.getByLabel("Name", { exact: true }).fill("Link test");
  await a.getByLabel("Your name", { exact: true }).fill("Ann");
  await a.getByLabel("Ask for a password to join").check();
  await a.getByLabel("Table password", { exact: true }).fill("sesame");
  const created = a.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await a.getByRole("button", { name: "Open table" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  expect(id, "the server named the table").toBeTruthy();
  await expect(a).toHaveURL(new RegExp(`#/t/${id}$`));
  // The link A would send somebody, read off A's own screen — and then followed
  // below, which is what says the field holds an address and not a sentence.
  //
  // `exact: true` here too, and this one was MEASURED rather than reasoned:
  // beside the field sits the creator's Abandon control, whose accessible name
  // is "abandon Link test" — and a substring match is case-insensitive, so a
  // bare `getByLabel("Link")` resolved to both.
  await expect(a.getByLabel("Link", { exact: true })).toHaveValue(new RegExp(`/#/t/${id}$`));

  // **And it fits a phone** (live review I-1). This screen is the one place in
  // the client where a `.field` holds a whole URL, and at 390px it pushed the
  // page sideways by 32px: the row would not wrap, the input would not shrink,
  // and the Copy button's right edge sat off the window at 422. The lane's
  // recorded property is 0px of page overflow from 320 to 2560, and the width
  // gate in `the_screen_itself.spec.ts` cannot carry this one — its anti-vacuity
  // read is the board's widest row, and an open table has no board.
  {
    const desktop = a.viewportSize()!;
    await a.setViewportSize({ width: 390, height: 844 });
    // The layout has to be DONE before it is read: a resize and a measurement in
    // the same tick can be answered from the pre-resize box (the width gate's
    // own note).
    await a.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));

    // Anti-vacuity FIRST, and it is the whole reason this is not a bare
    // `scrollWidth` compare: a screen that drew no link row has no overflow
    // either, which is exactly the failure a width assertion cannot feel.
    const copy = a.getByRole("button", { name: "copy" });
    const box = await copy.boundingBox();
    expect(box, "the Copy button must be on the screen to be measured").not.toBeNull();
    const fieldWidth = await a
      .getByLabel("Link", { exact: true })
      .evaluate((el) => el.getBoundingClientRect().width);
    expect(fieldWidth, "the link input must have a box to have been squeezed into").toBeGreaterThan(0);

    const read = await readOverflow(a);
    expect(
      read.docScrollWidth - read.docClientWidth,
      `the seated open table scrolls sideways at 390px: ${blame(read)}`,
    ).toBeLessThanOrEqual(0);
    // The page not scrolling is not on its own the claim: a control clipped by
    // an ancestor's own scroller would satisfy it. The button's right edge is
    // read against the window directly.
    expect(
      Math.round(box!.x + box!.width),
      "the Copy button's right edge is past the window",
    ).toBeLessThanOrEqual(read.docClientWidth);
    // eslint-disable-next-line no-console
    console.log(
      `open table at 390px: page overflow ${read.docScrollWidth - read.docClientWidth}px (gated) · ` +
        `link input ${Math.round(fieldWidth)}px · copy right edge ${Math.round(box!.x + box!.width)}`,
    );

    await a.setViewportSize(desktop);
    await a.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  }

  /** Every navigation A's main frame makes from here on. See the file's own
   *  note: on a real origin this catches a reload, a `pushState` and a hash
   *  change alike, all three measured. Armed BEFORE B goes anywhere near the
   *  table, so nothing in the window it is about can be missed. */
  const navigationsOnA: string[] = [];
  a.on("framenavigated", (f) => {
    if (f === a.mainFrame()) navigationsOnA.push(f.url());
  });

  // **B arrives by the LINK alone and never sees the lobby.** The address is
  // the whole invitation: B is signed in AT the table's own hash, which is the
  // one route this API widens to a caller who holds no seat (R-2.7).
  await signIn(b, "b@e2e.test", `/#/t/${id}`);
  await expect(b.getByRole("heading", { name: "Link test" })).toBeVisible();
  await expect(b.getByText(/password required/)).toBeVisible();
  await b.getByLabel("Your name", { exact: true }).fill("Bob");
  await b.getByLabel("Password", { exact: true }).fill("wrong");
  await b.getByRole("button", { name: "sit here" }).click();
  await expect(b.getByText(/wrong password/)).toBeVisible();
  // The refusal is about the PASSWORD and not about the person: the name B
  // typed is still in the field, and the right word is all that is missing.
  await expect(b.getByLabel("Your name", { exact: true })).toHaveValue("Bob");
  await b.getByLabel("Password", { exact: true }).fill("sesame");
  await b.getByRole("button", { name: "sit here" }).click();

  // **A's screen learns, on the poll, with nobody having touched it.** The join
  // filled the last seat, so the table is dealing — and A's open table hands
  // over to the deal by itself. What is read is the SEAT B took: A's dealing
  // screen names its other seat, and the name is the one B typed.
  await expect(a.locator('.deal-seat[data-seat="1"]')).toContainText("Bob", { timeout: 15_000 });
  expect(navigationsOnA, "A did not navigate while B sat down").toEqual([]);

  // Both keep, and the last keep starts the table. The step is
  // `two_browsers.spec.ts`'s: the pick's accessible name is the card it keeps.
  for (const p of [a, b]) {
    const keep = p.getByRole("button", { name: /^keep O\d\d/ }).first();
    await expect(keep).toBeVisible({ timeout: 30_000 });
    await keep.click();
  }
  await expect(b.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  await expect(a.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });

  // Six moves between them, each played on whichever screen is offering one.
  const pick = rng(0x4c49_4e4b); // "LINK"
  let moves = 0;
  let idle = 0;
  while (moves < MOVES) {
    let acted = false;
    for (const p of [a, b]) {
      if (moves >= MOVES) break;
      const s = await readScreen(p);
      expect(s.over, "a Yokohama game cannot be over in six moves").toBe(false);
      if (!hasMove(s)) continue;
      try {
        if ((await act(p, s, pick)) !== null) {
          moves += 1;
          acted = true;
        }
      } catch {
        // The 3 s poll landed between the look and the click and took the
        // control out from under it — the race `two_browsers.spec.ts` measured.
        // Not a defect in a product that is behaving correctly; the next look
        // is what decides.
      }
    }
    if (acted) {
      idle = 0;
      continue;
    }
    idle += 1;
    if (idle > IDLE_LIMIT) throw await deadEnd(a, { id }, moves);
    await a.waitForTimeout(400);
  }

  // And the table is in A's lobby, under Your games, saying who is at it and
  // whose turn it is — the words `tables.ts` gives every list.
  await a.locator("details.game-menu > summary").click();
  await a.getByRole("button", { name: "Lobby" }).click();
  await expect(a.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  // **Scoped to YOUR GAMES by NAME, and it used to be by exclusion.** A table
  // can be listed in more than one section — in Open tables while it is still
  // filling, and since M6.3 in Games in progress once it is not — so
  // `:not(.open-tables)` stopped meaning "yours" the moment a third section
  // arrived and resolved two rows for one table. The section says which one it
  // is instead.
  const row = a
    .locator("section.your-games li")
    .filter({ has: a.getByRole("link", { name: "Link test", exact: true }) });
  await expect(row).toBeVisible();
  await expect(row, "the row names the person who followed the link").toContainText("Bob");
  await expect(row, "and says whose turn it is").toContainText(/your turn|waiting on/);

  // eslint-disable-next-line no-console
  console.log(
    `join-by-link table ${id}: ${moves} moves across two browsers in ` +
      `${Math.round((Date.now() - started) / 1000)}s`,
  );
  expect(watchA.failures, "a's page reported no errors").toEqual([]);
  expect(watchB.failures, "b's page reported no errors").toEqual([]);
});
