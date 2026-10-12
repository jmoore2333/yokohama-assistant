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
//   1. The open table's own screen, from the OUTSIDE: a stranger's one button
//      and the password above the seats, that arriving seats nobody, and the
//      refusal a wrong password earns.
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
import { expect, newContextWithoutWorker, test } from "./sw";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { act, blame, deadEnd, nameThisBrowser, readOverflow, readScreen, rng, signIn, watchForBreakage, who } from "./harness";
import { LIVE, abandonOpened, awaitCreate, createdTable, liveContextOptions, type Opening } from "./live";
// Whether a screen has a move on it — read from what can actually be pressed,
// so a walk in flight reads as "not yet" rather than as a dead end. One
// predicate for every walk that shares a table; its doc carries the
// measurement (`shared_table.ts`).
import { hasMove } from "./shared_table";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The table this walk is opening, and the page of the person opening it:
 *  known from the moment its create is SENT ([`awaitCreate`]). */
let opened: Opening | null = null;

// **Live, the walk abandons the table it opened**, as its creator — in an
// `afterEach` rather than a `finally`, for the measured reason
// `two_browsers.spec.ts` gives: a timed-out body's `finally` runs only after
// its pages are closed. It knows the table from the moment the create is SENT,
// for as long as `awaitCreate`'s wait lasts (R3's L-9), so a walk that fails
// before it reads the create's reply still takes its table away. Locally it
// sends nothing: the local server's games are scratch, and its archive
// directory — which an abandon's save writes into — is read by other specs
// (`offline.spec.ts`, `plays_locally.spec.ts`).
test.afterEach(async () => {
  const table = opened;
  opened = null;
  await abandonOpened(table);
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
  const a = await (await newContextWithoutWorker(browser, await liveContextOptions(who("a")))).newPage();
  const b = await (await newContextWithoutWorker(browser, await liveContextOptions(who("b")))).newPage();
  const watchA = watchForBreakage(a);
  // **B's wrong password is a 403 BY DESIGN**, and the watch counts every 4xx
  // from this origin as breakage — so this one refusal is opted into by name,
  // exactly as the containment cell opts into `/result`'s 409. Scoped to
  // `/join`: a 403 anywhere else on B's screen is still a finding.
  const watchB = watchForBreakage(b, [{ status: 403, urlIncludes: "/join" }]);

  // A opens a table with ONE SEAT OPEN and a password on it. The submit says
  // "Open table" rather than "Deal", which is the form telling the truth about
  // what it is about to do.
  await signIn(a, who("a"));
  await expect(a.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();

  // **The corner is a pill on a phone, and nothing on the lobby scrolls
  // sideways** (M6.5c2, spec §2.2). It replaces M6.4's who-strip measure, whose
  // "Your name" field the corner's own control replaced. At desktop width the
  // corner reads "Playing as …" beside its one action; at 390px the same corner
  // is a pill and the action is one tap away. Live the door is the Worker's, so
  // the action is "change name"; locally it is the open door's "switch player".
  {
    const desktop = a.viewportSize()!;
    const corner = a.locator(".identity-corner");
    const action = corner.getByRole("button", { name: LIVE ? "change name" : "switch player" });
    const pill = corner.locator(".identity-pill");
    // The MENU's words: the pill says them too, unseen (M6.5c2 cycle C, the
    // a11y pass), so a corner-wide text read would find two.
    await expect(corner.locator(".identity-menu").getByText("Playing as")).toBeVisible();
    await expect(action).toBeVisible();
    await expect(pill).toBeHidden();

    await a.setViewportSize({ width: 390, height: 844 });
    await a.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
    await expect(pill).toBeVisible();
    await expect(action, "the action waits behind the pill").toBeHidden();
    const read = await readOverflow(a);
    expect(
      read.docScrollWidth - read.docClientWidth,
      `the lobby scrolls sideways at 390px: ${blame(read)}`,
    ).toBeLessThanOrEqual(0);
    await pill.click();
    await expect(action, "one tap opens the corner").toBeVisible();
    const openCorner = await readOverflow(a);
    expect(
      openCorner.docScrollWidth - openCorner.docClientWidth,
      `the open corner pushes the lobby sideways at 390px: ${blame(openCorner)}`,
    ).toBeLessThanOrEqual(0);
    await pill.click();

    await a.setViewportSize(desktop);
    await a.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  }

  await a.getByRole("button", { name: "New table" }).click();
  await expect(a.getByRole("heading", { name: "New table", exact: true, level: 3 })).toBeVisible();
  // **Each seat is its colour's since M6.5c2** (mock 07), and `exact`, because
  // "Blue" is also a First player choice.
  await a.getByRole("combobox", { name: "Blue seat", exact: true }).selectOption("open");
  await a.getByLabel("Name", { exact: true }).fill("Link test");
  // **A LONG name, deliberately** — it is what the chat row below is measured
  // under (live review, I-1). The form no longer asks for one (R-6.5c2.8: the
  // lobby's corner owns it), so it is what this browser KEEPS, set where the
  // create reads it.
  await nameThisBrowser(a, "Annabelle the Cartographer");
  await a.getByLabel("Ask for a password to join or spectate", { exact: true }).check();
  await a.getByLabel("Table password", { exact: true }).fill("sesame");
  opened = awaitCreate(a);
  await a.getByRole("button", { name: "Open table" }).click();
  const id = await createdTable(opened);
  await expect(a).toHaveURL(new RegExp(`#/t/${id}$`));
  // The link A would send somebody, read off A's own screen — and then followed
  // below, which is what says the field holds an address and not a sentence.
  //
  // `exact: true` here too, and it was MEASURED rather than reasoned when the
  // field was labelled "Link": beside it sits the creator's Abandon control,
  // whose accessible name is "abandon Link test" — and a substring match is
  // case-insensitive, so a bare `getByLabel("Link")` resolved to both.
  await expect(a.getByLabel("Invite link", { exact: true })).toHaveValue(new RegExp(`/#/t/${id}$`));
  // The creator's reminder that the password is theirs to send (mock 08).
  await expect(a.getByText("They'll need the password you set. Send it separately.")).toBeVisible();

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
  // the window's: below 976px this panel is ~790px inside the `.setup` column,
  // the label costs a fraction of it, and label, clock and text stay on one
  // line. Read as three tops, which is what says "one row" without naming a
  // pixel — and it is what would fail if the threshold were ever raised far
  // enough to fold the hosts the review did not ask about.
  //
  // **Read at 960px since cycle E** (straw poll row 67): from 976px the open
  // table's chat sits in mock 08's 300px rail, a NARROW host like the side
  // column, where the same rule stacks the row — read below as the partner.
  const rowRead = () =>
    chat
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
  const wide = a.viewportSize()!;
  await a.setViewportSize({ width: 960, height: wide.height });
  const oneRow = await rowRead();
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
  // The rail, at this context's own desktop width: the message takes the line
  // under its label, as it does in the game's side column.
  await a.setViewportSize(wide);
  expect(wide.width, "the premise: a desktop width, where the chat is the rail's").toBeGreaterThanOrEqual(976);
  const railRow = await rowRead();
  expect(railRow.text, "in the rail the message is on the line under its label").toBeGreaterThan(railRow.who);

  // **And it fits a phone** (live review I-1). This screen is the one place in
  // the client where a row holds a whole URL, and at 390px it pushed the
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
    const copy = a.getByRole("button", { name: "Copy link" });
    const box = await copy.boundingBox();
    expect(box, "the Copy button must be on the screen to be measured").not.toBeNull();
    const fieldWidth = await a
      .getByLabel("Invite link", { exact: true })
      .evaluate((el) => el.getBoundingClientRect().width);
    expect(fieldWidth, "the link input must have a box to have been squeezed into").toBeGreaterThan(0);

    // **The seat cards stack into one column on a phone** (mock 11), each the
    // width of the column: two cards side by side at 390 px are two cards a
    // thumb cannot read.
    const seatBoxes = await a
      .getByRole("list", { name: "Seats" })
      .getByRole("listitem")
      .evaluateAll((els) =>
        els.map((e) => {
          const r = e.getBoundingClientRect();
          return { left: Math.round(r.left), top: Math.round(r.top), bottom: Math.round(r.bottom) };
        }),
      );
    expect(seatBoxes.length, "the seat cards are on the screen").toBe(2);
    expect(seatBoxes[1]!.left, "one column").toBe(seatBoxes[0]!.left);
    expect(seatBoxes[1]!.top, "the second card is under the first").toBeGreaterThanOrEqual(seatBoxes[0]!.bottom);

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
  /** Every join B's page makes. **Arriving seats nobody** (R-6.5c2.9, Review
   *  Focus 4: one click, never zero): a link preview or a curious click must
   *  not take a seat. Armed BEFORE B goes to the address, so the arrival is
   *  inside the count. */
  const joinsByB: string[] = [];
  b.on("request", (r) => {
    if (new URL(r.url()).pathname.endsWith("/join")) joinsByB.push(r.url());
  });
  /** What B's page asks of the LOBBY on the way in (cycle E, R-E7): nothing.
   *  A cold `#/t/<id>` drew the lobby for one round trip while the table was
   *  read, which fired its three listings and opened the house stream; the
   *  screen of that window is "opening…" now. Armed before B goes to the
   *  address, so the whole arrival is inside it. */
  const lobbyReadsByB: string[] = [];
  b.on("request", (r) => {
    const u = new URL(r.url());
    if ((u.pathname === "/api/games" && u.searchParams.has("scope")) || u.pathname === "/api/stream")
      lobbyReadsByB.push(`${u.pathname}${u.search}`);
  });
  await signIn(b, who("b"), `/#/t/${id}`);
  // `level: 3`: the band's last crumb is the page's `<h2>` and carries the
  // table's name too.
  await expect(b.getByRole("heading", { name: "Link test", exact: true, level: 3 })).toBeVisible();
  expect(lobbyReadsByB, "arriving by the link read no listing and opened no house stream").toEqual([]);
  // The lock, as its tag, and ONE way in: exactly one seat is free.
  await expect(b.getByRole("img", { name: "password required" })).toBeVisible();
  const join = b.getByRole("button", { name: "Join as Blue" });
  await expect(join).toBeVisible();
  await expect(b.getByRole("button", { name: /^Sit here/ })).toHaveCount(0);
  expect(joinsByB, "arriving by the link seated nobody").toEqual([]);

  // **The joiner's screen at 390 px** (mock 11, "Joining: one seat left"): one
  // column, the button a thumb's size and on the screen, nothing sideways.
  {
    const desktop = b.viewportSize()!;
    await b.setViewportSize({ width: 390, height: 844 });
    await b.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
    const box = await join.boundingBox();
    expect(box, "Join as Blue must be on the screen to be measured").not.toBeNull();
    expect(box!.height, "Join as Blue is a thumb's size").toBeGreaterThanOrEqual(44);
    const read = await readOverflow(b);
    expect(
      read.docScrollWidth - read.docClientWidth,
      `the joiner's open table scrolls sideways at 390px: ${blame(read)}`,
    ).toBeLessThanOrEqual(0);
    expect(Math.round(box!.x + box!.width), "Join as Blue's right edge is on the screen").toBeLessThanOrEqual(
      read.docClientWidth,
    );
    await b.setViewportSize(desktop);
    await b.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  }

  // What B is called at the table is what this browser KEEPS: the join asks no
  // name (R-6.5c2.8), and A's dealing screen reads "Bob" below.
  await nameThisBrowser(b, "Bob");
  await b.getByLabel("Table password", { exact: true }).fill("wrong");
  // Still nobody seated after all that reading: the first join is the click.
  expect(joinsByB, "nothing joined while B read the screen").toEqual([]);
  await join.click();
  await expect(b.getByText(/wrong password/)).toBeVisible();
  // The refusal is about the PASSWORD: the field keeps what was typed, and the
  // one way in is still there.
  await expect(b.getByLabel("Table password", { exact: true })).toHaveValue("wrong");
  await expect(join).toBeVisible();
  await b.getByLabel("Table password", { exact: true }).fill("sesame");
  await join.click();
  // Anti-vacuity for the count above: the listener sees a join when there is
  // one — the refused click and this one, and no third.
  await expect.poll(() => joinsByB.length, { message: "the two clicks are the two joins" }).toBe(2);

  // **A's screen learns, on the poll, with nobody having touched it.** The join
  // filled the last seat, so the table is dealing — and A's open table hands
  // over to the deal by itself. What is read is the SEAT B took: A's dealing
  // screen names its other seat, and the name is the one B's browser keeps.
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

    // **The line taken down, on the same page, by the frame** (M6.5c2 cycle
    // D, R-D4). A's Delete answers an empty 204 and changes nothing on screen
    // by itself: the table's next frame carries `chat_rev` one past its
    // `chat`, and the panel re-reads what it holds — so "message removed"
    // appearing in A's own row is a reading of the SIGNAL, and the text is gone
    // from the page.
    const said = gameChat.getByRole("listitem").filter({ hasText: "and a line on the game screen" });
    await said.getByRole("button", { name: "Delete this message" }).click();
    await expect(
      gameChat.locator("ol.chat-lines li").last(),
      "the tombstone, in the author's own row",
    ).toContainText("message removed");
    await expect(said, "and the words are gone from the page").toHaveCount(0);
    await expect(
      gameChat.locator("ol.chat-lines li").last().getByRole("button"),
      "a tombstone has no Delete of its own",
    ).toHaveCount(0);
  }

  // And the table is in A's lobby, under Your tables, saying who is at it and
  // whose turn it is — the words `tables.ts` gives every list.
  await a.locator("details.game-menu > summary").click();
  await a.getByRole("button", { name: "Lobby" }).click();
  await expect(a.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  // **Scoped to YOUR TABLES by NAME, and it used to be by exclusion.** Until
  // M6.5c2 a table was listed in more than one section (in Open tables while it
  // was still filling, and from M6.3 in Games in progress once it was not), so
  // `:not(.open-tables)` stopped meaning "yours" the moment a third section
  // arrived and resolved two rows for one table. A table you sit at is now
  // listed under Your tables only, and the section still says which one it is.
  const row = a
    .locator("section.your-games li")
    .filter({ has: a.getByRole("link", { name: "Link test", exact: true }) });
  await expect(row).toBeVisible();
  await expect(row, "the row names the person who followed the link").toContainText("Bob");
  await expect(row, "and says whose turn it is").toContainText(/your move|to move/);

  // eslint-disable-next-line no-console
  console.log(
    `join-by-link table ${id}: ${moves} moves across two browsers in ` +
      `${Math.round((Date.now() - started) / 1000)}s` +
      // The dealing screen's designed 409, forgiven in both modes (`harness.ts`,
      // `isDealAnswer`): two lines per 409, the response and Chromium's own.
      `; the deal's designed 409 forgiven a ${watchA.dealAnswers}, b ${watchB.dealAnswers} times` +
      // Live, how often each allowed console error was spent, each its own count.
      (LIVE
        ? `; live at ${BASE_URL}: the beacon's CORS error forgiven a ${watchA.forgiven}, b ${watchB.forgiven} times` +
          `; the release feed's failure forgiven a ${watchA.feedForgiven}, b ${watchB.feedForgiven} times` +
          `; the detection script's Report-Only line forgiven a ${watchA.reportOnlyForgiven}, b ${watchB.reportOnlyForgiven} times`
        : ""),
  );
  expect(watchA.failures, "a's page reported no errors").toEqual([]);
  expect(watchB.failures, "b's page reported no errors").toEqual([]);
});
