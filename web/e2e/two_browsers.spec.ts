// **The gate two people play: two browsers, one table, and neither hand on the
// other screen.**
//
// `plays_a_whole_game.spec.ts` proves the SCREEN can be played to a winner, and
// `play_a_whole_game.rs`'s second arm proves the API serves each caller its own
// seat. Neither can say what this one says, and the gap between them is exactly
// where M6.1's secrecy claim lives: one browser, one `localStorage`, one
// identity header — a hot seat, where the secrecy is the SEQUENCE and not the
// payload. Two contexts are two people. Each signs in as itself, each holds its
// own storage and sends its own header, and the table alternates between them
// on the poll (§6) with no shared state in the test but the table id.
//
// What is read on every single move: each context's OWN hand panel, and the two
// sets are disjoint. The search is scoped to the hand because a PORT card is
// public and legitimately appears on both screens — a whole-DOM search for the
// other's card art would fail on a correct product.
//
// **And since M6.3 there is a THIRD browser, which is here to WATCH** (spec
// §9.4). It signs in as a person nobody named at this table, arrives by the
// ADDRESS alone, is stopped by the password, types it, and from then on holds
// no seat and no hand while the game plays out in front of it. Three things are
// read of it after every move that applied an event, and each answers a
// different question:
//
//   * the account's newest row, BY ITS EVENT INDEX — that the move reached this
//     screen, rather than that this screen has some row on it (a check by
//     presence passes on an account that stopped updating);
//   * its own hand panel, which is empty and stays empty;
//   * its request log — **no request to `/events` at all, on ONE open
//     `/stream`**, with the tab asserted visible, because "it never polled" is
//     a claim about a browser that was actually being asked to keep up rather
//     than one the browser had let go of (spec §9.5, mutations 11 and 12). That
//     count is what found the reopen ruling R-9.1 fixed.

import { type Page, type WebSocket } from "@playwright/test";
// **`test` comes from `./sw`, not from `@playwright/test`** (M6.5b2, spec §2.3):
// the fixture flags every page in the context so the release's service worker is
// NOT registered during the walk. `every_spec_declares_its_worker.spec.ts` is what
// keeps that from being a convention anybody has to remember.
import { expect, newContextWithoutWorker, test } from "./sw";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { act, deadEnd, openFromList, readScreen, rng, signIn, watchForBreakage } from "./harness";
import type { Affordance, Screen } from "./harness";
import { LIVE, abandonAsCreator, liveContextOptions, liveRequest } from "./live";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The table this walk opened, and the page of the person who opened it. */
let opened: { page: Page; id: string } | null = null;

// **Live, the walk abandons the table it opened** (spec §5.4): the Worker's
// lobby is shared with real people and every other run, so what a walk leaves
// behind stays there. As its CREATOR, through A's own page; the game's save is
// the Worker's, under the bucket's `test/` root.
//
// **An `afterEach` and not a `finally`, and that was MEASURED** (Playwright
// 1.62.1): when a test runs out of its budget, a `finally` in its body runs
// only after the test has ENDED — the page closed, and its request refused
// "Test ended." — while this hook runs first, with the page still open. A live
// game that times out is exactly the table that must not be left behind.
// Locally it sends nothing: the local server's games are scratch, and its
// archive directory — which an abandon's save writes into — is read by other
// specs (`offline.spec.ts`, `plays_locally.spec.ts`).
test.afterEach(async () => {
  const table = opened;
  opened = null;
  if (LIVE && table) await abandonAsCreator(table.page, table.id);
});

/** Whether `ws` is this table's stream: the Worker's transport is a socket
 *  (`ws`, M6.5c). MEASURED: a socket the page opens raises a `websocket` event
 *  and NO `request` event, so a count read off the request log alone would
 *  read ZERO live. Locally none fires — the transport is the stream's `fetch`
 *  — and the watcher's checkpoint `toBe(1)` pins that: a socket counted beside
 *  the stream's request would read 2. */
const socketTo = (ws: WebSocket, path: string) => path !== "" && new URL(ws.url()).pathname === path;

/** The walk's own cap on MOVES. Idle looks have their own, below: the two
 *  failures are different — a game that will not end, and a table where nobody
 *  is moving — and a single counter would report the second as the first. */
const CAP = 4000;

/** The word the third browser has to present before it may read this table. */
const WATCH_PASSWORD = "sesame";

/** Who watches. Named at no seat, invited by nobody. */
const WATCHER = "c@e2e.test";

/**
 * The card ids in a page's OWN hand panel — the one place another seat's cards
 * could leak onto this screen.
 *
 * `aside.side` is the game screen's side column and `App` renders `Dashboard`
 * there with `only={["Players", "Hand"]}`, so the single `.card-strip` of order
 * cards under it is the viewer's hand. `data-card` is `OrderCardView`'s own
 * identifier attribute: the id is otherwise only prose in a caption and pixels
 * in a picture, neither of which compares across two browsers.
 */
async function ownHand(page: Page): Promise<string[]> {
  return page
    .locator("aside.side .card-strip .card-face.order")
    .evaluateAll((els) => els.map((e) => e.getAttribute("data-card") ?? ""));
}

/**
 * Whether this screen has a move on it — which is NOT the same as "not inert",
 * and the difference is a measured race.
 *
 * A screen that is playing back the other seat's turn is neither watching nor
 * locked, so `inert` is `null` there; it also has nothing to click, because the
 * walk has replaced the deck with its own strip. `readScreen` presses that
 * strip's skip control once before it reads, but the poll can START a walk in
 * the window between the two pages' reads — measured: the walk picked the page
 * that had begun walking, `act` found no affordance and reported a dead end on
 * a game that was fine. So the turn is chosen by what can actually be pressed.
 */
function hasMove(s: Screen): boolean {
  return (
    s.inert === null && (s.options > 0 || s.tiles > 0 || s.counts > 0 || s.confirm || s.only)
  );
}

test("two people at two browsers play to a winner and neither sees the other's hand", async ({
  browser,
}) => {
  // **Twice the config's budget, because this walk pays for two screens and a
  // three-second poll.** The vs-bot walk clicks one page and the bot answers
  // inside the same response; here every turn boundary waits on the other
  // browser to notice, and the watching browser plays the turn back before it
  // is answerable. The run prints its own move, wait and elapsed figures below,
  // which is the number to read rather than this one.
  //
  // **And twice that again live** (plan review m15): every request crosses the
  // edge, and the local run alone has taken 457 s.
  test.setTimeout((LIVE ? 40 : 20) * 60 * 1000);
  const started = Date.now();
  // **This table's stream path — empty until the SERVER has named the table.**
  // The players' listeners below are installed with their pages, long before
  // there is an id, and read this variable at REQUEST time; an exact match on
  // the whole path is also what keeps the lobby's own house channel
  // (`/api/stream`) out of these counts, as it does for the watcher's.
  let streamPath = "";
  // Two people, and live each context is its person (`live.ts`); locally a
  // context is the plain one it always was, and the sign-in form names them.
  const a = await (await newContextWithoutWorker(browser, await liveContextOptions("a@e2e.test"))).newPage();
  const b = await (await newContextWithoutWorker(browser, await liveContextOptions("b@e2e.test"))).newPage();
  /** How many times each PLAYER opened this table's stream (R-6.4.4, R-plan-7).
   *  The teardown that closed it at every turn boundary is gone, so this is a
   *  count that stops growing once each screen is up — and a reopen mid-game
   *  would show as one that never stops. Counted from the moment the id exists
   *  rather than from the first move, so the figures the run prints are the
   *  players' REAL opens and the comparison at the bottom is a reading rather
   *  than two zeroes agreeing. */
  const streamOpens = { a: 0, b: 0 };
  for (const [k, p] of [["a", a], ["b", b]] as const) {
    p.on("request", (r) => {
      if (new URL(r.url()).pathname === streamPath) streamOpens[k] += 1;
    });
    p.on("websocket", (ws) => {
      if (socketTo(ws, streamPath)) streamOpens[k] += 1;
    });
  }
  const watchA = watchForBreakage(a);
  const watchB = watchForBreakage(b);

  // A opens the table and names B in seat 1. The form's default at two players
  // seats the BOT there, so this is also the one walk that exercises the seat
  // row's "another email" arm.
  await signIn(a, "a@e2e.test");
  // The lobby is the first screen since M6.2; the form is one click on.
  await expect(a.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  await a.getByRole("button", { name: "New table" }).click();
  await expect(a.getByRole("heading", { name: "New game" })).toBeVisible();
  // `exact: true`: switching this row to an email puts a "Seat 1 email" field
  // beside it, and `getByLabel` matches by substring.
  await a.getByLabel("Seat 1", { exact: true }).selectOption("email");
  await a.getByLabel("Seat 1 email").fill("b@e2e.test");
  // **A password, because the third browser is the one it is for** (spec §9.4).
  // It costs the two players nothing — both hold a seat and a seat holder never
  // presents anything — and it is the only way to reach the unlock face, which
  // is the screen between a link and a game somebody is here to watch.
  await a.getByLabel("Ask for a password to join").check();
  await a.getByLabel("Table password").fill(WATCH_PASSWORD);
  const created = a.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await a.getByRole("button", { name: "Deal" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  expect(id, "the server named the table").toBeTruthy();
  opened = { page: a, id };
  streamPath = `/api/games/${id}/stream`;

  // A chooses, and is then waiting: the table does not start until B has
  // chosen too, and A's screen says whom it is waiting on by name.
  await a.getByRole("button", { name: /^keep O\d\d/ }).first().click();
  await expect(a.getByText(/waiting on b@e2e\.test/)).toBeVisible();

  // B finds the table in its OWN saved list — the only way into a table this
  // browser did not open — chooses, and the last keep starts the game.
  await signIn(b, "b@e2e.test");
  await openFromList(b, id);
  await b.getByRole("button", { name: /^keep O\d\d/ }).first().click();
  await expect(b.getByText(/end triggers:/)).toBeVisible();
  // And A's screen opens itself, on the poll, with nobody having touched it.
  await expect(a.getByText(/end triggers:/)).toBeVisible({ timeout: 15_000 });

  // **Anti-vacuity for the leak check, taken FIRST (spec §9.4 item 8).** At the
  // opening each seat holds exactly the starting order it kept, so both hands
  // are non-empty by construction — which is what says the selector below finds
  // a page's own cards rather than nothing. Asserted per page, so a selector
  // that worked on one screen and not the other cannot pass.
  expect((await ownHand(a)).length, "a's own hand is on a's screen").toBeGreaterThan(0);
  expect((await ownHand(b)).length, "b's own hand is on b's screen").toBeGreaterThan(0);

  // ---- the third browser, which is here to watch (spec §9.4) --------------
  //
  // It arrives at the table's own address and nothing else: C holds no seat, so
  // the lobby's Your games does not list this table for it, and the only way in
  // is the link somebody would have sent.
  const c = await (await newContextWithoutWorker(browser, await liveContextOptions(WATCHER))).newPage();
  const watchC = watchForBreakage(c);
  await signIn(c, WATCHER);
  await c.goto(`/#/t/${id}`);
  // The unlock face, and it is the SERVER's answer that puts it there: a
  // playing table this caller holds no seat at, which has not been unlocked.
  // Exact, for the reason `harness.ts` gives: the lobby's own heading, not
  // the chat panel's, which carries the word too.
  await expect(c.getByRole("heading", { name: "Lobby", exact: true })).toHaveCount(0);
  const watchButton = c.getByRole("button", { name: "Watch" });
  await expect(watchButton).toBeVisible();

  // **The request log, installed BEFORE the stream is opened**, so the one
  // connection the watcher makes for this table is inside the count. The house
  // stream the lobby held while C was signing in is not: it belongs to a screen
  // that is already gone, and counting it would make this a reading about a
  // different page.
  //
  // **Counted by THIS TABLE's whole path** (review T9 #4). A suffix test also
  // matches `/api/stream`, the lobby's own house channel — excluded here only
  // by the timing above, and a reading that rests on when a listener was
  // installed is a reading one navigation away from being about two pages.
  const eventsAsked: string[] = [];
  let streams = 0;
  c.on("request", (r) => {
    const path = new URL(r.url()).pathname;
    if (path.endsWith("/events")) eventsAsked.push(path);
    if (path === streamPath) streams += 1;
  });
  // The Worker's transport is a socket (`ws`), counted by the same whole path
  // — and a socket the page reopens is the Worker's "reconnecting…", read here
  // as the stream's second open. Its one probe after a close nobody explained
  // is a `/events` request, which the line above already counts. Locally no
  // socket opens (`socketTo` says what pins that).
  c.on("websocket", (ws) => {
    if (socketTo(ws, streamPath)) streams += 1;
  });

  await c.getByLabel("Password").fill(WATCH_PASSWORD);
  await watchButton.click();
  await expect(c.getByText("You are watching")).toBeVisible();
  await expect(c.getByText(/end triggers:/)).toBeVisible({ timeout: 15_000 });
  // The watcher's own side column is empty of cards from the first frame, and
  // it is the same selector that finds A's and B's hands on their screens —
  // asserted non-empty two lines above, which is what stops this being a
  // sentence about a selector that matches nothing anywhere.
  expect(await ownHand(c), "a watcher is dealt no hand").toEqual([]);

  // ---- M6.4: the exit — two players talk, the watcher reads it and answers,
  // and the MOVER reads the answer during their own turn (spec §9.4) --------
  //
  // Every line here arrives over the connection the screen already holds: a
  // post bumps the table, the frame carries the new count, and the panel asks
  // for the delta. Nothing below reloads a page, which is what makes "arrives"
  // a claim about the transport rather than about a refresh.
  const chatOf = (p: Page) => p.locator("aside.side section.chat");
  const unfold = async (p: Page) => {
    const fold = chatOf(p).getByRole("button", { name: "show" });
    if (await fold.count()) await fold.click();
    await expect(chatOf(p).getByRole("button", { name: "hide" })).toBeVisible();
  };
  const say = async (p: Page, text: string) => {
    await unfold(p);
    await chatOf(p).getByRole("textbox", { name: "Say something" }).fill(text);
    await chatOf(p).getByRole("textbox", { name: "Say something" }).press("Enter");
    await expect(chatOf(p).getByRole("listitem").filter({ hasText: text })).toBeVisible();
  };
  const reads = async (p: Page, text: string) => {
    await unfold(p);
    await expect(
      chatOf(p).getByRole("listitem").filter({ hasText: text }),
      `${text} arrives without a reload`,
    ).toBeVisible({ timeout: 15_000 });
  };
  await say(a, "hello from a");
  await reads(b, "hello from a");
  await say(b, "and b");
  await reads(c, "hello from a");
  await reads(c, "and b");
  await say(c, "watching you both");
  // The watcher is MARKED as one, which is the label rule R-6.4.2 asks of a
  // table. **The separator is in the needle, and that is the whole assertion:**
  // the row is selected by a text that already contains the word, so a check
  // for "watching" alone is implied by its own filter and could never fail —
  // only the marker's own " · watching" distinguishes a labelled watcher from
  // one who happened to say the word.
  await expect(
    chatOf(c).getByRole("listitem").filter({ hasText: "watching you both" }),
  ).toContainText("· watching");
  // **The mover reads it during their OWN turn** — the cell that must fail
  // under the old `active` (R-6.4.4): whichever page shows a move, that page's
  // panel receives the watcher's line while its prompt is on screen. The
  // premise is asserted rather than assumed: a page with nothing to answer
  // would make this a reading about an idle screen.
  const mover = hasMove(await readScreen(a)) ? a : b;
  expect(hasMove(await readScreen(mover)), "the premise: it is this page's turn").toBe(true);
  await reads(mover, "watching you both");
  // The players' game-screen streams, read once the game is on screen (R-plan-7)
  // and AFTER the exchange, so every mount this walk causes is already paid for
  // and what is compared at the bottom is the playing half alone.
  const streamsAtStart = { a: streamOpens.a, b: streamOpens.b };

  /** How many events the game had when the watcher was last held to one. */
  let lastApplied = 0;
  /** How many times it was held to a NEW one — the anti-vacuity count for the
   *  whole watching half, since a move that applies no event (a picker step)
   *  correctly asks the watcher for nothing. */
  let watchChecks = 0;
  /** The stream count read at each checkpoint, so the figure is in the run's
   *  own output line rather than only in an assertion message. */
  const streamReads: number[] = [];

  /** How many events this table has applied, asked of the SERVER.
   *
   *  Deliberately not read off a screen: the expected row index has to come
   *  from something other than the account being checked, or the check is the
   *  account agreeing with itself. Issued from A's context — never from C's —
   *  so the watcher's own request log stays a reading about the watcher.
   *
   *  **Through `liveRequest`**, which live withholds a thrown call's log: it
   *  lists the context's Access cookie (`live.ts`). Locally it changes
   *  nothing. */
  const appliedEvents = async (): Promise<number> => {
    const res = await liveRequest(() =>
      a.request.get(`${BASE_URL}/api/games/${id}/events`, {
        headers: { "x-yokohama-claimed-user": "a@e2e.test" },
      }),
    );
    expect(res.ok(), `the event count is readable (${res.status()})`).toBeTruthy();
    return ((await res.json()) as { events: number }).events;
  };

  /**
   * What the watching browser must show once a move has landed.
   *
   * **The row is read by its EVENT INDEX** (spec §9.5 item 12). `data-event` is
   * the record's own index for the row's event, so an account that stopped
   * updating holds rows whose indexes are the WRONG ones — where a check that
   * the account has a row, or has N rows, or contains some words, passes on
   * exactly that screen. The index expected is the newest event's, and it comes
   * from the server rather than from C.
   *
   * A move that applied nothing — selecting an area inside a placement picker —
   * asks the watcher for nothing, which is why this is keyed on the count
   * MOVING and `watchChecks` is asserted at the bottom.
   */
  const watch = async (acting: Page): Promise<void> => {
    // The mover's own request has to have landed, or the count read below is
    // the one from before the move and the wait proves nothing. `data-busy` is
    // the client's own in-flight state, the same signal `readScreen` steers by.
    await acting.waitForFunction(() => !document.querySelector("[data-busy]"), null, {
      timeout: 15_000,
    });
    const applied = await appliedEvents();
    if (applied === lastApplied) return;
    lastApplied = applied;
    await expect(
      c.locator(".turn-account li[data-event]").last(),
      `the watcher's account reaches event ${applied - 1}`,
    ).toHaveAttribute("data-event", String(applied - 1), { timeout: 30_000 });
    expect(await ownHand(c), "the watcher still holds no hand").toEqual([]);
    // **Counted AFTER the assertions, never before** (review I-1): a counter
    // incremented on the way in counts holds ATTEMPTED, and the anti-vacuity
    // line at the bottom would then pass on a run in which every one of them
    // failed.
    watchChecks += 1;

    if (watchChecks % 20 === 0) {
      // **The claim R-3.2 makes, and the two things that would make it hollow.**
      // A watcher that never asked for `/events` because its tab was hidden has
      // let go of the stream as well and is not being kept up to date at all
      // (mutation 11); the assertion above is what says it IS, and these two say
      // it is doing it on one held connection and in a tab the browser considers
      // visible.
      expect(eventsAsked, "the watcher never asks /events while a stream is open").toEqual([]);
      expect(
        await c.evaluate(() => document.visibilityState),
        "the watcher's tab is visible, so it is being asked to keep up",
      ).toBe("visible");
      // ONE connection, checked at the move rather than only as a total.
      expect(streams, "the watcher is watching on ONE open stream").toBe(1);
      streamReads.push(streams);
    }
  };

  const pick = rng(0x5457_4f50); // "TWOP"
  const pages = { a, b };
  let moves = 0;
  let waits = 0;
  /** The most cards either page ever had in hand, so the disjointness check
   *  below can say it was comparing something. */
  let mostHeld = 0;
  /** The hand cross-check and then one move, on the page the loop just looked
   *  at.
   *
   *  **The page that is read is the page that is ACTED ON, with nothing in
   *  between but that cross-check.** Reading both screens and then acting on the
   *  first left a whole `readScreen` of the other page — seconds — between the
   *  look and the click, which is more than enough for a three-second poll to
   *  start a walk under it. */
  const step = async (who: "a" | "b", screen: Screen): Promise<void> => {
    // **The cross-check, every move.** No card id is in both hands — which is
    // what "neither sees the other's hand" means on a screen, since a hand
    // another seat could see would have to be drawn in this one.
    const [handA, handB] = [await ownHand(a), await ownHand(b)];
    mostHeld = Math.max(mostHeld, handA.length, handB.length);
    expect(handA.filter((c) => handB.includes(c)), "no card is in both hands").toEqual([]);

    // **A click that misses is a finding only if the screen still SAYS there is
    // a move.** Both halves of this are the same race and both were measured: a
    // three-second poll can land between the look and the click, and then either
    // `act` finds no affordance at all or the control it resolved is detached
    // under it. Neither is a defect in a product that is behaving correctly —
    // what would be is a screen that still offers a move nobody can take.
    let acted: Affordance | null = null;
    try {
      acted = await act(pages[who], screen, pick);
    } catch (e) {
      if (hasMove(await readScreen(pages[who]))) throw e;
      return;
    }
    if (acted !== null) {
      // **The watcher is held to THIS move before the walk takes another, and
      // OUTSIDE the catch above** (review I-1).
      //
      // Asked afterwards, in a batch, it would only ever be held to the last
      // move — and an account that fell behind for fifty moves and caught up
      // would read exactly like one that never fell behind at all. But the
      // first version of this put the hold INSIDE the click-race `try`, and
      // that catch forgives anything the acting screen no longer has a move
      // for: after a move that ENDS A TURN the acting page shows the watching
      // prompt, `hasMove` is false, and every one of the watcher's assertions —
      // the account's index, its empty hand, the `/events` count, the stream
      // count, the tab's visibility — was discarded with a `return`. The catch
      // is for ONE thing, a click that raced the poll and missed, and it now
      // wraps only the click.
      await watch(pages[who]);
      return;
    }
    if (hasMove(await readScreen(pages[who]))) throw await deadEnd(pages[who], { id }, moves);
  };

  /** Consecutive looks at two screens with no move on either. A turn boundary
   *  costs a few (the mover's browser has not polled yet, or is still playing
   *  the other's turn back); a run of them is nobody moving, which is the dead
   *  end this walk can reach that a one-browser walk cannot. */
  let idle = 0;
  const IDLE_LIMIT = 60;
  /** Whoever moved last, asked first. A turn is several moves long, so looking
   *  at the other screen first costs a whole `readScreen` per move for most of a
   *  game — and reading a screen is not free, since each one waits out whatever
   *  request is in flight. */
  let first: "a" | "b" = "a";
  for (;;) {
    // **`readScreen` is what copes with a WALK**, and it is the same handling
    // the vs-bot walk already relies on: the turn playback (M5.5) replaces the
    // deck with a strip while the other seat's turn plays out, and `readScreen`
    // presses its skip control once before reading. A person's turn walks on
    // the poll exactly as a bot's does (M6.1 R-9.1), so nothing new is needed
    // here.
    const second: "a" | "b" = first === "a" ? "b" : "a";
    const s1 = await readScreen(pages[first]);
    if (s1.over) break;
    if (hasMove(s1)) {
      idle = 0;
      await step(first, s1);
      moves += 1;
      if (moves > CAP) throw await deadEnd(pages[first], { id }, moves);
      continue;
    }
    const s2 = await readScreen(pages[second]);
    if (s2.over) break;
    if (hasMove(s2)) {
      idle = 0;
      await step(second, s2);
      first = second;
      moves += 1;
      if (moves > CAP) throw await deadEnd(pages[second], { id }, moves);
      continue;
    }
    // **Short looks rather than one long sleep.** The poll runs every three
    // seconds and a walk ends whenever it ends, so sleeping a whole interval
    // spends up to three seconds per turn boundary waiting for something that
    // has already happened — and there are dozens of boundaries in a game.
    idle += 1;
    waits += 1;
    if (idle > IDLE_LIMIT) throw await deadEnd(a, { id }, moves);
    await a.waitForTimeout(400);
  }

  // eslint-disable-next-line no-console
  console.log(
    `two-browser game ${id} finished in ${moves} moves, ${waits} idle looks, ` +
      `${Math.round((Date.now() - started) / 1000)}s; largest hand seen ${mostHeld}; ` +
      `the watcher was held to ${watchChecks} applied moves, asked /events ` +
      `${eventsAsked.length} times and opened /stream ${streams} times ` +
      `(${streamReads.join(",") || "no checkpoint"} at the checkpoints); ` +
      `the players opened this table's stream a ${streamOpens.a}, b ` +
      `${streamOpens.b} times (a ${streamsAtStart.a}, b ${streamsAtStart.b} ` +
      `before the first move)` +
      // The dealing screen's designed 409, forgiven in both modes (`harness.ts`,
      // `isDealAnswer`): two lines per 409, the response and Chromium's own.
      `; the deal's designed 409 forgiven a ${watchA.dealAnswers}, b ${watchB.dealAnswers}, ` +
      `c ${watchC.dealAnswers} times` +
      // Live, how often the one allowed console error was spent.
      (LIVE
        ? `; live at ${BASE_URL}: the beacon's CORS error forgiven a ${watchA.forgiven}, ` +
          `b ${watchB.forgiven}, c ${watchC.forgiven} times`
        : ""),
  );
  expect(moves, "the walk must not hit its own cap").toBeLessThan(CAP);
  // The disjointness check above is vacuous against two empty hands, and a hand
  // CAN legitimately empty late in a game — so what is asserted is that it was
  // comparing real cards at some point, not that it was at every point.
  expect(mostHeld, "the walk compared real hands").toBeGreaterThan(0);

  await expect(a.getByText("The game is over.")).toBeVisible({ timeout: 15_000 });
  await expect(b.getByText("The game is over.")).toBeVisible({ timeout: 15_000 });
  // The watcher saw it end too, without ever holding a seat or a card.
  await expect(c.getByText("The game is over.")).toBeVisible({ timeout: 30_000 });
  expect(await ownHand(c), "the watcher ends the game holding no hand").toEqual([]);
  expect(watchA.failures, "a's page reported no errors").toEqual([]);
  expect(watchB.failures, "b's page reported no errors").toEqual([]);
  expect(watchC.failures, "the watcher's page reported no errors").toEqual([]);

  // **The watching half was doing something**: a run in which no move ever
  // applied an event would have asked the watcher for nothing and passed every
  // line above.
  expect(watchChecks, "the watcher was held to the account of real moves").toBeGreaterThan(20);
  expect(eventsAsked, "the watcher never asked for /events").toEqual([]);

  // A second open is a FINDING and not a tolerance — something reconnected on
  // its own — and the checkpoint above names the move it started on.
  expect(streams, "the watcher held ONE open stream for the whole game").toBe(1);

  // **No reopen for a player either** (R-6.4.4, R-plan-7): the per-turn
  // teardown is gone, so the count read at the start is the count at the end.
  // A player's screen is not the watcher's — it posts, it plays back the other
  // seat's turn and it goes busy — and a per-turn teardown would leave these
  // counts climbing with the game while the watcher's stayed at one.
  expect(streamOpens.a, "a never reopened this table's stream").toBe(streamsAtStart.a);
  expect(streamOpens.b, "b never reopened this table's stream").toBe(streamsAtStart.b);
});
