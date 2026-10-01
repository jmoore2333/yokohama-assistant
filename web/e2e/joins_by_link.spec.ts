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

// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import type { Page } from "@playwright/test";
import { expect, newContextWithoutWorker, test } from "./sw";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { act, blame, deadEnd, readOverflow, readScreen, rng, signIn, watchForBreakage } from "./harness";
import { LIVE, abandonAsCreator, liveContextOptions } from "./live";
// Whether a screen has a move on it — read from what can actually be pressed,
// so a walk in flight reads as "not yet" rather than as a dead end. One
// predicate for every walk that shares a table; its doc carries the
// measurement (`shared_table.ts`).
import { hasMove } from "./shared_table";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The table this walk opened, and the page of the person who opened it. */
let opened: { page: Page; id: string } | null = null;

// **Live, the walk abandons the table it opened**, as its creator — in an
// `afterEach` rather than a `finally`, for the measured reason
// `two_browsers.spec.ts` gives: a timed-out body's `finally` runs only after
// its pages are closed. Locally it sends nothing: the local server's games are
// scratch, and its archive directory — which an abandon's save writes into —
// is read by other specs (`offline.spec.ts`, `plays_locally.spec.ts`).
test.afterEach(async () => {
  const table = opened;
  opened = null;
  if (LIVE && table) await abandonAsCreator(table.page, table.id);
});

/** How many moves the two of them play between them. Enough to say the table
 *  really is a game and that both browsers can answer it; deliberately not a
 *  whole game, which is `two_browsers.spec.ts`'s job and its twelve minutes. */
const MOVES = 6;

/** Consecutive looks at two screens with nothing pressable on either before the
 *  walk gives up. A turn boundary costs a few — the other browser has not
 *  polled yet, or is playing the turn back — and a run of them is nobody
 *  moving. */
const IDLE_LIMIT = 60;

test("a person holding only a link joins a locked open table and plays", async ({ browser }) => {
  test.setTimeout(5 * 60 * 1000);
  const started = Date.now();
  // Live, each context is its person (`live.ts`); locally a context is the
  // plain one it always was, and the sign-in form names them.
  const a = await (await newContextWithoutWorker(browser, await liveContextOptions("a@e2e.test"))).newPage();
  const b = await (await newContextWithoutWorker(browser, await liveContextOptions("b@e2e.test"))).newPage();
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

  // **The who-strip still reads as one line of small print on a phone** (live
  // review, M-1). M6.4 put "Your name" (R-6.4.3 — a label plus a 9rem input)
  // into a `flex` row with no `flex-wrap`, so at 390px the lobby's first line
  // became a 51px block with the address broken across THREE lines and "sign
  // out" across two. Nothing overflowed and nothing was clipped, which is why
  // neither the width gate nor the overflow read below can see it.
  //
  // **The oracle is the SAME elements at desktop width**, not a pixel constant:
  // what is claimed is that narrowing the window does not make these two pieces
  // of text taller, and a magic number would be a second design figure to keep
  // in step. Read before the resize and again after it.
  {
    const desktop = a.viewportSize()!;
    const strip = a.locator(".who-strip");
    const parts = () =>
      strip.evaluate((el) => {
        const who = el.querySelector(".who")!;
        // `null` where the strip draws no sign-out — which is the edge's door,
        // live (asserted below).
        const out = el.querySelector("button");
        const name = el.querySelector(".your-name input")!;
        const box = (n: Element) => Math.round(n.getBoundingClientRect().height);
        return {
          who: box(who),
          out: out ? box(out) : null,
          name: Math.round(name.getBoundingClientRect().width),
          nameTop: Math.round(name.getBoundingClientRect().top),
          whoBottom: Math.round(who.getBoundingClientRect().bottom),
          strip: Math.round(el.getBoundingClientRect().height),
          wrap: getComputedStyle(el).flexWrap,
        };
      });
    const wide = await parts();
    await a.setViewportSize({ width: 390, height: 844 });
    // The layout has to be DONE before it is read (the width gate's own note).
    await a.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
    const narrow = await parts();

    // Anti-vacuity: every piece of the strip is really on the phone screen. A
    // strip that had dropped its name field would satisfy the heights below
    // for the wrong reason.
    expect(wide.who, "the address has a box at desktop width").toBeGreaterThan(0);
    expect(narrow.name, "the name field is still on the screen at 390px").toBeGreaterThan(0);

    expect(
      narrow.who,
      `the address wraps at 390px: ${narrow.who}px against ${wide.who}px at ${desktop.width}`,
    ).toBeLessThanOrEqual(wide.who);
    if (LIVE) {
      // **The edge's door draws no sign-out**: the Worker's door is
      // `cf-access`, and `App.tsx` draws the button under `claimed` ONLY. So
      // live there is none to measure — and its absence is the reading: a
      // sign-out under Access would look like it logged you out while Access
      // kept letting you in.
      expect(wide.out, "no sign-out under the edge's door").toBeNull();
      expect(narrow.out, "no sign-out under the edge's door").toBeNull();
    } else {
      expect(
        narrow.out,
        `"sign out" wraps at 390px: ${narrow.out}px against ${wide.out}px at ${desktop.width}`,
      ).toBeLessThanOrEqual(wide.out!);
    }
    // And the field is on its OWN line rather than sharing one — the other half
    // of the fix, and the half that says the row wrapped where it was told to.
    // A field that lands beside a short address on one screen and under it on
    // the next is worse than one that is always in the same place.
    expect(
      narrow.nameTop,
      `the name field shares a line with the address at 390px (top ${narrow.nameTop}, ` +
        `the address ends at ${narrow.whoBottom})`,
    ).toBeGreaterThanOrEqual(narrow.whoBottom);

    const read = await readOverflow(a);
    expect(
      read.docScrollWidth - read.docClientWidth,
      `the lobby scrolls sideways at 390px: ${blame(read)}`,
    ).toBeLessThanOrEqual(0);
    // eslint-disable-next-line no-console
    console.log(
      `who-strip at 390px: address ${narrow.who}px (desktop ${wide.who}) · ` +
        `${narrow.out === null ? "no sign out" : `sign out ${narrow.out}px (desktop ${wide.out})`} · strip ${narrow.strip}px ` +
        `(desktop ${wide.strip}) · flex-wrap ${narrow.wrap} · name field ${narrow.name}px`,
    );

    await a.setViewportSize(desktop);
    await a.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  }

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
  // **A LONG name, deliberately** — it is what the chat row below is measured
  // under (live review, I-1). The review's own worst case is shorter than this
  // and needs no arranging at all: an un-named watcher is labelled by their
  // email and marked " · watching", which is the DEFAULT case at a table.
  await a.getByLabel("Your name", { exact: true }).fill("Annabelle the Cartographer");
  await a.getByLabel("Ask for a password to join").check();
  await a.getByLabel("Table password", { exact: true }).fill("sesame");
  const created = a.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await a.getByRole("button", { name: "Open table" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  expect(id, "the server named the table").toBeTruthy();
  opened = { page: a, id };
  await expect(a).toHaveURL(new RegExp(`#/t/${id}$`));
  // The link A would send somebody, read off A's own screen — and then followed
  // below, which is what says the field holds an address and not a sentence.
  //
  // `exact: true` here too, and this one was MEASURED rather than reasoned:
  // beside the field sits the creator's Abandon control, whose accessible name
  // is "abandon Link test" — and a substring match is case-insensitive, so a
  // bare `getByLabel("Link")` resolved to both.
  await expect(a.getByLabel("Link", { exact: true })).toHaveValue(new RegExp(`/#/t/${id}$`));

  // **One line of chat on the open table**, which is what the two readings in
  // the next block are taken against. Folded by default (R-6.4.10), so it is
  // opened first — the panel is shown to the SEATED creator only, which is why
  // this happens here and not on B's seat picker.
  const chat = a.locator("section.chat");
  await chat.getByRole("button", { name: "show" }).click();
  await chat.getByRole("textbox", { name: "Say something" }).fill("first one here");
  await chat.getByRole("textbox", { name: "Say something" }).press("Enter");
  await expect(chat.getByRole("listitem").filter({ hasText: "first one here" })).toBeVisible();

  // **The panel's heading is the PANEL's, on a screen that is also `.setup`**
  // (live review, M-3). `.setup h2` is declared after `.panel h2` and wins on
  // source order, so the open table drew a large letter-spaced CHAT where every
  // other host draws 10px mono small caps — and `.setup h3`'s `margin-top` gave
  // the lobby's folded box 26px of dead space above its one line. Read as a
  // COMPUTED value, which is the only thing that can see a cascade defect: the
  // component suite never loads `styles.css` at all.
  const headingFont = await chat
    .locator("h2")
    .evaluate((el) => getComputedStyle(el).fontSize);
  expect(headingFont, "the open table's chat heading is the panel's, not the setup screen's").toBe(
    "10px",
  );

  // **And a WIDE panel keeps the one-row layout it has always had.** The fix
  // for I-1 is a CONTAINER query, so it opens on the row's own width and not on
  // the window's: this panel is ~800px inside the `.setup` column at desktop
  // width, the label costs a fraction of it, and label, clock and text stay on
  // one line. Read as three tops, which is what says "one row" without naming a
  // pixel — and it is what would fail if the threshold were ever raised far
  // enough to fold the hosts the review did not ask about.
  const oneRow = await chat
    .locator("ol.chat-lines li")
    .first()
    .evaluate((li) => {
      const top = (sel: string) => Math.round(li.querySelector(sel)!.getBoundingClientRect().top);
      return {
        who: top(".chat-who"),
        text: top(".chat-text"),
        row: Math.round(li.getBoundingClientRect().height),
        line: Math.round(li.querySelector(".chat-who")!.getBoundingClientRect().height),
      };
    });
  // The label and the message share a top — both are 11.5px text on the same
  // baseline, so this is exact. (The clock is not: it is 10px, and `baseline`
  // alignment puts its top 2px lower. Asserting on it would be asserting a
  // font size.)
  expect(oneRow.text, "the wide panel's message shares the label's row").toBe(oneRow.who);
  // And the row really is ONE line tall, which is what a top-compare alone
  // cannot say: two rows whose first lines happened to align would satisfy it.
  expect(
    oneRow.row,
    `the wide panel's row is ${oneRow.row}px against a ${oneRow.line}px line`,
  ).toBeLessThan(2 * oneRow.line);

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
    // **A long label must not squeeze the message into a column eight
    // characters wide** (live review, I-1). The row is a three-track grid with
    // the label `nowrap`, so at 390px the label and the clock took the width
    // and the text got what was left: measured on the live site at 158px under
    // a 15-character ADDRESS (22 characters per line), and 76px less again
    // under a watcher's " · watching" marker — eleven. Nothing is clipped and
    // nothing overflows, so only a reading of the two boxes can see it.
    //
    // The floor is the TEXT against its own ROW rather than a pixel count,
    // because the row's width is a layout figure the design may change and the
    // share is the property: a message is the point of the line.
    const line = await a
      .locator("section.chat ol.chat-lines li")
      .first()
      .evaluate((li) => ({
        row: Math.round(li.getBoundingClientRect().width),
        text: Math.round(li.querySelector(".chat-text")!.getBoundingClientRect().width),
        who: Math.round(li.querySelector(".chat-who")!.getBoundingClientRect().width),
        time: Math.round(li.querySelector("time")!.getBoundingClientRect().width),
      }));
    // Anti-vacuity, and it is the same shape as the Copy button above: a row
    // that drew no label leaves the text the whole width and passes for the
    // wrong reason.
    expect(line.who, "the long label is really on the row").toBeGreaterThan(60);
    expect(line.row, "the row has a box").toBeGreaterThan(0);
    expect(
      line.text / line.row,
      `the message got ${line.text}px of a ${line.row}px row under a ${line.who}px label ` +
        `and a ${line.time}px clock`,
    ).toBeGreaterThanOrEqual(0.6);
    // eslint-disable-next-line no-console
    console.log(
      `open table at 390px: page overflow ${read.docScrollWidth - read.docClientWidth}px (gated) · ` +
        `link input ${Math.round(fieldWidth)}px · copy right edge ${Math.round(box!.x + box!.width)} · ` +
        `chat row ${line.row}px = label ${line.who} + clock ${line.time} + text ${line.text} ` +
        `(${Math.round((100 * line.text) / line.row)}% of the row, floor 60%)`,
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

  // **And the same reading in the SIDE COLUMN at desktop width** (live review,
  // I-1, the half the review predicted and did not see).
  //
  // The squeeze is a property of the ROW's width and not of the window's: the
  // side column is ~322px of row at 1280 and at 2560 alike, so a viewport media
  // query would have fixed the phone and left this. Photographed at 27px of
  // text under a watcher's label before the fix; measured here under the
  // creator's own long name, which needs no third browser.
  {
    const gameChat = a.locator("aside.side section.chat");
    await expect(gameChat).toBeVisible();
    const show = gameChat.getByRole("button", { name: "show" });
    if (await show.count()) await show.click();
    await gameChat.getByRole("textbox", { name: "Say something" }).fill("and a line on the game screen");
    await gameChat.getByRole("textbox", { name: "Say something" }).press("Enter");
    await expect(
      gameChat.getByRole("listitem").filter({ hasText: "and a line on the game screen" }),
    ).toBeVisible();
    const line = await gameChat
      .locator("ol.chat-lines li")
      .last()
      .evaluate((li) => ({
        row: Math.round(li.getBoundingClientRect().width),
        text: Math.round(li.querySelector(".chat-text")!.getBoundingClientRect().width),
        who: Math.round(li.querySelector(".chat-who")!.getBoundingClientRect().width),
        time: Math.round(li.querySelector("time")!.getBoundingClientRect().width),
      }));
    expect(line.who, "the long label is really on the row").toBeGreaterThan(60);
    expect(
      line.text / line.row,
      `the message got ${line.text}px of a ${line.row}px side-column row under a ` +
        `${line.who}px label and a ${line.time}px clock`,
    ).toBeGreaterThanOrEqual(0.6);
    // eslint-disable-next-line no-console
    console.log(
      `side column at ${a.viewportSize()!.width}px: chat row ${line.row}px = label ${line.who} + ` +
        `clock ${line.time} + text ${line.text} ` +
        `(${Math.round((100 * line.text) / line.row)}% of the row, floor 60%)`,
    );
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
      `${Math.round((Date.now() - started) / 1000)}s` +
      // The dealing screen's designed 409, forgiven in both modes (`harness.ts`,
      // `isDealAnswer`): two lines per 409, the response and Chromium's own.
      `; the deal's designed 409 forgiven a ${watchA.dealAnswers}, b ${watchB.dealAnswers} times` +
      // Live, how often the one allowed console error was spent.
      (LIVE ? `; live at ${BASE_URL}: the beacon's CORS error forgiven a ${watchA.forgiven}, b ${watchB.forgiven} times` : ""),
  );
  expect(watchA.failures, "a's page reported no errors").toEqual([]);
  expect(watchB.failures, "b's page reported no errors").toEqual([]);
});
