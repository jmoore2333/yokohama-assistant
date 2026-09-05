// What every browser walk needs: a game, a console watch, and a click policy.

import { expect, type ConsoleMessage, type Page } from "@playwright/test";
import { BASE_URL } from "../playwright.config";

/** A deterministic PRNG, so a failing walk reproduces from its seed alone.
 *  SplitMix64's shape, in the 32 bits JS can multiply exactly. */
export function rng(seed: number): () => number {
  let s = seed >>> 0;
  return () => {
    s = (s + 0x9e37_79b9) >>> 0;
    let z = s;
    z = Math.imul(z ^ (z >>> 16), 0x21f0_aaad) >>> 0;
    z = Math.imul(z ^ (z >>> 15), 0x735a_2d97) >>> 0;
    return ((z ^ (z >>> 15)) >>> 0) / 0x1_0000_0000;
  };
}

/**
 * Everything the page said that a player would call broken.
 *
 * Console ERRORS and uncaught exceptions only. Warnings are not collected: a
 * warning that is not a defect would make this a list nobody reads, and React's
 * own development warnings do not reach a production build anyway.
 *
 * **Failed responses are filtered to THIS ORIGIN.** `index.html` pulls two font
 * families from `fonts.googleapis.com`, so an unfiltered watch makes the whole
 * browser gate fail on somebody else's outage or on a machine with no network —
 * a red run that says nothing about this repo. Art 404s are a supported state in
 * the client, which draws its typed fallback, but not in a checkout that has the
 * art committed, which every checkout does; those are same-origin and still
 * counted.
 */
/**
 * Collect anything that looks like breakage: a console error, an uncaught
 * exception, or any 4xx/5xx from our own origin.
 *
 * `expected` names refusals the SERVER makes by design, which a caller has to
 * opt into one at a time. There is exactly one today and it exists because of
 * M4: the containment cell now derives its surface list from the nav rather
 * than a hand-written six, which correctly added **Final score** — and
 * `GET …/result` is a 409 while the game is still running, by contract. Without
 * this the cell failed three runs in four on a product that was behaving
 * exactly as specified.
 *
 * Deliberately a per-call opt-in and not a global allowlist: a 409 from
 * `/result` is expected only where a test opens that panel mid-game, and
 * anywhere else it is still a finding.
 */
export function watchForBreakage(
  page: Page,
  expected: { status: number; urlIncludes: string }[] = [],
): { failures: string[] } {
  const failures: string[] = [];
  const allowed = (status: number, url: string) =>
    expected.some((e) => e.status === status && url.includes(e.urlIncludes));
  page.on("console", (m: ConsoleMessage) => {
    // Chromium logs its own `Failed to load resource … (409 Conflict)` for the
    // same response, so an allowed status has to be forgiven in both channels
    // or the allowlist only half works.
    if (m.type() !== "error") return;
    const text = m.text();
    if (expected.some((e) => text.includes(String(e.status)))) return;
    failures.push(`console.error: ${text}`);
  });
  page.on("pageerror", (e) => failures.push(`uncaught: ${e.message}`));
  page.on("response", (r) => {
    if (r.status() >= 400 && r.url().startsWith(BASE_URL) && !allowed(r.status(), r.url())) {
      failures.push(`${r.status()} ${r.url()}`);
    }
  });
  return { failures };
}

/** What `startGame` hands back: enough to reproduce and to name a failure. */
export interface Game {
  /** The store id, which is also the button that reopens it. */
  id: string;
  /** The setup seed as a decimal string — the server drew it, so a failing walk
   *  is only reproducible if the failure says which game it was. */
  seed: string;
}

/** Start a fresh game through the setup screen, exactly as a player does. */
export async function startGame(page: Page, opts: { players?: number } = {}): Promise<Game> {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "New game" })).toBeVisible();

  if (opts.players && opts.players !== 2) {
    await page.getByLabel("Players").selectOption(String(opts.players));
  }
  // **Both facts come off the RESPONSES, not off the banner.**
  //
  // This used to read `header .meta` and split it on `·`, taking the seed from
  // position 1 and the id from position 2. That is a test parsing a display
  // string for machine-readable facts, and it broke the moment the banner
  // stopped printing the seed — silently, into `id === "saved"`, because the
  // fields shifted by one and every segment is still a non-empty string. The
  // walk then spent fifteen seconds waiting to click a saved game called
  // "saved".
  //
  // The seed no longer reaches any play payload at all (it regenerates the
  // whole hidden set), so `preview` is now the only place it is served — which
  // is fine, and better: this captures each value from the request that
  // actually carries it. The seed stays in the handle because a failed walk is
  // only reproducible if its failure message names the game it walked.
  const previewed = page.waitForResponse(
    (r) => r.url().includes("/api/games/preview") && r.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Deal" }).click();
  const seed = String(((await (await previewed).json()) as { seed: string }).seed);

  await expect(page.getByRole("button", { name: "Start" })).toBeVisible();
  const created = page.waitForResponse(
    (r) =>
      /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Start" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  await expect(page.getByRole("heading", { name: "Yokohama" })).toBeVisible();

  if (!id || !seed) throw new Error(`the server did not name the game: id=${id} seed=${seed}`);
  return { id, seed };
}

/**
 * Everything the screen offers, read in ONE round trip.
 *
 * A policy that asked the page five separate questions per move paid five round
 * trips for each of a few hundred moves, and — worse — could resolve a locator
 * and then act on it after a re-render had removed it, which waits out the whole
 * test budget on an element that is never coming back. One `evaluate` is both
 * faster and atomic with respect to a render.
 */
export interface Screen {
  over: boolean;
  options: number;
  tiles: number;
  counts: number;
  confirm: boolean;
  only: boolean;
  /** The prompt says someone else is deciding, or that the game is locked.
   *  Neither is answerable, and neither is "nothing left to click" — a walk that
   *  reported them that way would name the wrong defect. */
  inert: "watching" | "locked" | null;
}

export async function readScreen(page: Page): Promise<Screen> {
  // **Wait for the request in flight FIRST.** Every control in the decision zone
  // — and every board tile — is `disabled` exactly while `busy`, which is the
  // client's own in-flight state. A read taken between the click and the
  // response therefore sees a screen with nothing enabled on it and reports
  // "nothing left to click", which is a true statement about that instant and a
  // false one about the game. Measured: the first walk failed at its first move,
  // with the decision zone plainly showing "End additional-action phase" in the
  // failure message.
  //
  // `disabled` is the signal rather than a sleep because it is the SAME
  // condition the client renders from, so this cannot drift from what a player
  // can actually press.
  //
  // **And `data-busy` beside it, because `disabled` is only a signal on a
  // screen that HAS a control (M5.5).** The finished game's deck is a standing
  // with no buttons in it, so the disabled test passed the instant the reply
  // landed — before the turn playback's positions had arrived — and the walk
  // then replaced the deck. Measured: the whole-game cell read "The game is
  // over.", left its loop, and failed ten seconds later looking for the same
  // sentence. `data-busy` is the client's own in-flight state and covers the
  // whole window, the run fetch included, so a walk that is coming has started
  // by the time this returns.
  await page.waitForFunction(
    () =>
      !document.querySelector("[data-busy]") &&
      !document.querySelector(".decision button[disabled]"),
    null,
    { timeout: 15_000 },
  );
  // **A running walk is skipped here, once, before the screen is read.** The
  // turn playback (M5.5) replaces the deck with a strip while the opponent's
  // turn plays out, and a policy reading that strip would report "nothing to
  // click" about a game that is fine. The one cell that OBSERVES a walk
  // (`the_playback.spec.ts`) does its watching before it calls this.
  const skip = page.locator(".decision.walking button", { hasText: "skip" });
  if (await skip.count()) {
    await skip.first().click();
    await page.locator(".decision.walking").waitFor({ state: "detached", timeout: 15_000 });
  }
  return page.evaluate(() => {
    const many = (sel: string) => document.querySelectorAll(sel).length;
    const named = (name: string) =>
      [...document.querySelectorAll<HTMLButtonElement>(".decision button:not([disabled])")].some(
        (b) => b.textContent?.trim() === name,
      );
    const text = document.querySelector(".decision")?.textContent ?? "";
    return {
      over: document.body.innerText.includes("The game is over."),
      options: many("button.option:not([disabled])"),
      tiles: many("button.tile:not([disabled])"),
      counts: many(".options .controls button.ghost:not([disabled])"),
      confirm: named("Confirm"),
      only: named("Apply the only option"),
      inert: text.includes("Nothing here is yours to answer")
        ? ("watching" as const)
        : text.includes("This game is LOCKED")
          ? ("locked" as const)
          : null,
    };
  });
}

/** Which affordance a move went through — so a walk can say what it exercised
 *  rather than only that it finished. */
export type Affordance = "option" | "only" | "placement" | "count" | "confirm";

/**
 * One move, chosen from WHAT IS ON THE SCREEN.
 *
 * Deliberately not from the payload. A walk that read `/api/games/{id}` to
 * decide what to click would be testing the API a second time; the question here
 * is whether the SCREEN offers a way forward at every point a game can reach. So
 * the policy reads only the affordances a player can see, and returns `null`
 * when there is nothing left to click — which is itself the finding.
 */
export async function act(
  page: Page,
  screen: Screen,
  pick: () => number,
): Promise<Affordance | null> {
  const nth = (n: number) => Math.floor(pick() * n);

  // **"Apply the only option" gets its turn.** It is rendered exactly when there
  // is ONE option row, so a policy that always preferred `button.option` never
  // clicked it — a real control, in the shipped client, that no test had ever
  // pressed. Half the time, when both are on screen.
  if (screen.only && pick() < 0.5) {
    await page.getByRole("button", { name: "Apply the only option" }).click();
    return "only";
  }

  if (screen.options > 0) {
    await page.locator("button.option:not([disabled])").nth(nth(screen.options)).click();
    return "option";
  }

  // A placement picker. The board is the input surface, and the tiles that are
  // buttons are exactly the ones the client will let you toggle.
  //
  // Placing NOTHING is legal and is what the server-side gate does, but a walk
  // that only ever did that would never put an assistant on the board, and a
  // game in which nobody places never ends — measured on the fixture walk at
  // turn 3,644 with nothing having happened at all. Option A takes up to three
  // areas, so the count VARIES: a walk that always placed exactly one never
  // exercised a multi-area selection or the fee that grows with it.
  if (screen.tiles > 0 && screen.confirm) {
    const wanted = 1 + nth(Math.min(3, screen.tiles));
    for (let i = 0; i < wanted; i += 1) {
      const live = page.locator("button.tile:not([disabled])");
      const n = await live.count();
      if (n === 0) break;
      await live.nth(nth(n)).click();
    }
    await page.getByRole("button", { name: "Confirm" }).click();
    return "placement";
  }

  if (screen.counts > 0) {
    await page
      .locator(".options .controls button.ghost:not([disabled])")
      .nth(nth(screen.counts))
      .click();
    return "count";
  }

  if (screen.confirm) {
    await page.getByRole("button", { name: "Confirm" }).click();
    return "confirm";
  }

  if (screen.only) {
    await page.getByRole("button", { name: "Apply the only option" }).click();
    return "only";
  }

  return null;
}

/**
 * Why the walk stopped, in a sentence that names the game.
 *
 * `.decision` is `.first()` because the client renders the class more than once
 * — the warnings block and the error block are both `.decision` — so a bare
 * locator is a strict-mode violation exactly when something has gone wrong,
 * which is the worst possible time to throw a different error.
 */
export async function deadEnd(page: Page, game: Game, moves: number): Promise<Error> {
  const screen = await readScreen(page);
  const zone = await page.locator(".decision").first().innerText();
  const why =
    screen.inert === "watching"
      ? "the prompt says another seat is deciding, which a bot-free game should never show"
      : screen.inert === "locked"
        ? "the session is LOCKED after a driver or engine fault"
        : "nothing on the screen is answerable and the game is not over";
  return new Error(
    `${why}\n  game ${game.id}, seed ${game.seed}, after ${moves} moves\n\n${zone}`,
  );
}
