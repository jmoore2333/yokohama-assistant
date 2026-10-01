// What the walks that SHARE one table need: whose hand is on a screen, whether
// a screen has a move, the table's own chat, the table's stream, and the
// server's count of applied events.
//
// Lifted out of `two_browsers.spec.ts` when the exit's headed game needed the
// same readings (M6.5c Task 16; ruling I-88) — a second copy would be a second
// definition of "this screen has a move", and the two would drift. The shape
// `harness.ts`'s layout reading already took (M5.5 ruling R-T12-1).
// `joins_by_link.spec.ts` reads `hasMove` from here too; it carried the third
// copy.

import { expect, type Page, type WebSocket } from "@playwright/test";
import { BASE_URL } from "../playwright.config";
import type { Screen } from "./harness";
import { liveRequest } from "./live";

/** Whether `ws` is a table's stream at `path`: the Worker's transport is a
 *  socket (`ws`, M6.5c). MEASURED: a socket the page opens raises a `websocket`
 *  event and NO `request` event, so a count read off the request log alone
 *  would read ZERO live. Locally none fires — the transport is the stream's
 *  `fetch` — and `two_browsers`' watcher checkpoint `toBe(1)` pins that: a
 *  socket counted beside the stream's request would read 2. An empty `path`
 *  (no table named yet) matches nothing. */
export const socketTo = (ws: WebSocket, path: string): boolean =>
  path !== "" && new URL(ws.url()).pathname === path;

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
export async function ownHand(page: Page): Promise<string[]> {
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
export function hasMove(s: Screen): boolean {
  return (
    s.inert === null && (s.options > 0 || s.tiles > 0 || s.counts > 0 || s.confirm || s.only)
  );
}

/** The game screen's own chat panel, in its side column. */
export const chatOf = (p: Page) => p.locator("aside.side section.chat");

/** Open the panel if it is folded — folded by default (R-6.4.10), and the fold
 *  is remembered per browser, so this is a no-op after the first time. */
export async function unfold(p: Page): Promise<void> {
  const fold = chatOf(p).getByRole("button", { name: "show" });
  if (await fold.count()) await fold.click();
  await expect(chatOf(p).getByRole("button", { name: "hide" })).toBeVisible();
}

/** Say `text` at the table, and see it land on the sayer's own panel. */
export async function say(p: Page, text: string): Promise<void> {
  await unfold(p);
  await chatOf(p).getByRole("textbox", { name: "Say something" }).fill(text);
  await chatOf(p).getByRole("textbox", { name: "Say something" }).press("Enter");
  await expect(chatOf(p).getByRole("listitem").filter({ hasText: text })).toBeVisible();
}

/** `text` arrives on `p`'s panel — over the connection the screen already
 *  holds, with no reload. */
export async function reads(p: Page, text: string): Promise<void> {
  await unfold(p);
  await expect(
    chatOf(p).getByRole("listitem").filter({ hasText: text }),
    `${text} arrives without a reload`,
  ).toBeVisible({ timeout: 15_000 });
}

/**
 * How many events table `id` has applied, asked of the SERVER through `page`'s
 * own context as `email`.
 *
 * Deliberately not read off a screen: an expected row index has to come from
 * something other than the account being checked, or the check is the account
 * agreeing with itself. A caller issues it from a SEATED page, never from the
 * watcher's, so the watcher's own request log stays a reading about the
 * watcher.
 *
 * **Through `liveRequest`**, which live withholds a thrown call's log: it
 * lists the context's Access cookie (`live.ts`). Locally it changes nothing.
 */
export async function appliedEvents(page: Page, id: string, email: string): Promise<number> {
  const res = await liveRequest(() =>
    page.request.get(`${BASE_URL}/api/games/${id}/events`, {
      headers: { "x-yokohama-claimed-user": email },
    }),
  );
  expect(res.ok(), `the event count is readable (${res.status()})`).toBeTruthy();
  return ((await res.json()) as { events: number }).events;
}
