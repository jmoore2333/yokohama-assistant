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

import { expect, test, type Page } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, deadEnd, openFromList, readScreen, rng, signIn, watchForBreakage } from "./harness";
import type { Screen } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The walk's own cap on MOVES. Idle looks have their own, below: the two
 *  failures are different — a game that will not end, and a table where nobody
 *  is moving — and a single counter would report the second as the first. */
const CAP = 4000;

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
  test.setTimeout(20 * 60 * 1000);
  const started = Date.now();
  const a = await (await browser.newContext()).newPage();
  const b = await (await browser.newContext()).newPage();
  const watchA = watchForBreakage(a);
  const watchB = watchForBreakage(b);

  // A opens the table and names B in seat 1. The form's default at two players
  // seats the BOT there, so this is also the one walk that exercises the seat
  // row's "another email" arm.
  await signIn(a, "a@e2e.test");
  await a.getByLabel("Seat 1").selectOption("email");
  await a.getByLabel("Seat 1 email").fill("b@e2e.test");
  const created = a.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await a.getByRole("button", { name: "Deal" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  expect(id, "the server named the table").toBeTruthy();

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
    try {
      if ((await act(pages[who], screen, pick)) !== null) return;
    } catch (e) {
      if (hasMove(await readScreen(pages[who]))) throw e;
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
      `${Math.round((Date.now() - started) / 1000)}s; largest hand seen ${mostHeld}`,
  );
  expect(moves, "the walk must not hit its own cap").toBeLessThan(CAP);
  // The disjointness check above is vacuous against two empty hands, and a hand
  // CAN legitimately empty late in a game — so what is asserted is that it was
  // comparing real cards at some point, not that it was at every point.
  expect(mostHeld, "the walk compared real hands").toBeGreaterThan(0);

  await expect(a.getByText("The game is over.")).toBeVisible({ timeout: 15_000 });
  await expect(b.getByText("The game is over.")).toBeVisible({ timeout: 15_000 });
  expect(watchA.failures, "a's page reported no errors").toEqual([]);
  expect(watchB.failures, "b's page reported no errors").toEqual([]);
});
