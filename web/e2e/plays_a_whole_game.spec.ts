// **The gate: a whole game, played in a real browser, to a winner.**
//
// The browser analogue of `yokohama-cli`'s `piped_dogfood` and of
// `yokohama-web`'s `play_a_whole_game`, and it exists for the reason neither of
// those covers: a hundred component tests that each prove one branch renders
// still do not prove the SCREEN can be played. Every phase, every queued choice,
// every collapsed family and every sub-picker has to be reachable by CLICKING —
// and both client defects M2 turned up were found exactly that way, by hand,
// late.

import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, deadEnd, readScreen, rng, startGame, watchForBreakage } from "./harness";
import type { Affordance } from "./harness";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly. The reporter
// then says how many tests did not run, which is the difference between a skip
// and a silent absence.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The walk's own cap. Well above what a game takes (the run prints its own
 *  figure), and low enough that a loop fails as a loop rather than as a timeout. */
const CAP = 4000;

test("a whole two-player game plays through to a winner", async ({ page }) => {
  const watch = watchForBreakage(page);
  const game = await startGame(page);

  const pick = rng(0x594f_4b4f); // "YOKO"
  const used = new Map<Affordance, number>();
  let moves = 0;
  let screen = await readScreen(page);
  while (!screen.over && moves < CAP) {
    const via = await act(page, screen, pick);
    if (via === null) throw await deadEnd(page, game, moves);
    used.set(via, (used.get(via) ?? 0) + 1);
    moves += 1;
    screen = await readScreen(page);
  }
  // eslint-disable-next-line no-console
  console.log(
    `game ${game.id} (seed ${game.seed}) finished in ${moves} moves via ` +
      [...used].map(([k, n]) => `${k}×${n}`).join(", "),
  );

  expect(moves, "the walk must not hit its own cap").toBeLessThan(CAP);
  await expect(page.getByText("The game is over.")).toBeVisible();
  await expect(page.locator(".meta.turn")).toHaveText("finished");

  // **What the walk actually EXERCISED, not merely that it finished.** Without
  // this the headline claim — that every phase and sub-picker is reachable by
  // clicking — is a claim a green run does not support: a walk that only ever
  // pressed option rows would pass exactly the same assertions.
  for (const required of ["option", "placement", "confirm"] as const) {
    expect(used.get(required) ?? 0, `the walk never used the ${required} affordance`)
      .toBeGreaterThan(0);
  }
  // `count` (a guided retreat) and `only` (a sole option) are RECORDED rather
  // than required: neither is guaranteed to arise in a given random game, and a
  // required-but-occasional cell is a flake wearing a coverage badge.

  // A finished game scores both seats, on the screen and not only on the wire.
  const scores = page.locator(".options .option");
  await expect(scores).toHaveCount(2);
  for (const text of await scores.allInnerTexts()) {
    expect(text).toMatch(/\d+ VP/);
  }

  expect(watch.failures, "the page reported no errors while a whole game was played").toEqual([]);
});

test("a game survives a reload, because nothing lives only in memory", async ({ page }) => {
  const watch = watchForBreakage(page);
  const game = await startGame(page);

  // Play far enough that the state to be restored is NOT the opening state.
  const pick = rng(0x5245_4c4f); // "RELO"
  let moves = 0;
  for (let i = 0; i < 40; i += 1) {
    const screen = await readScreen(page);
    if (screen.over) break;
    if ((await act(page, screen, pick)) === null) throw await deadEnd(page, game, moves);
    moves += 1;
  }
  expect(moves, "the preamble must actually advance the game").toBeGreaterThan(5);

  // **What is compared is what CHANGES.** The first draft compared the banner —
  // `sim · seed n · id · saved` — which is constant for the life of a game, so
  // the assertion held whether or not a single move survived the round trip.
  // Turn/seat and the log length both move with play.
  // `innerText` on BOTH sides. The chain is `text-transform: uppercase`, so
  // `innerText` renders "TURN 11 · SEAT 0" while `toHaveText` compares
  // `textContent` — "Turn 11 · seat 0" — and the two never match.
  const chainOf = () => page.locator(".decision .chain").first().innerText();
  const chain = await chainOf();

  // Every applied move is persisted as its `GameRecord`, and a handle that is
  // gone is rebuilt through `Session::from_record` — the property the hosted
  // plan depends on, since both shapes under consideration sleep when idle and
  // lose their disk.
  //
  // The game is found BY NAME. `first()` would pass on a store holding one game
  // and silently open the wrong one on a store holding several, which is what a
  // second test in the same run leaves behind.
  await page.reload();
  await expect(page.getByRole("heading", { name: "New game" })).toBeVisible();
  await page.getByRole("button", { name: game.id, exact: true }).click();
  await expect(page.locator("header .meta").first()).toContainText(game.id);
  expect(await chainOf(), "the rebuilt game resumes at the turn it left off").toBe(chain);

  // And the log the server rebuilt carries the moves that were played.
  const logged = await page.evaluate(
    async (id: string) => (await (await fetch(`/api/games/${id}/log`)).json()).total as number,
    game.id,
  );
  expect(logged, "the rebuilt record carries the events the walk applied").toBeGreaterThan(0);

  expect(watch.failures).toEqual([]);
});
