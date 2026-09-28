// What every browser walk needs: a game, a console watch, and a click policy.

import { expect, test, type ConsoleMessage, type Locator, type Page } from "@playwright/test";
import { BASE_URL } from "../playwright.config";
import { LIVE } from "./live";

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
 * How many of the deal's designed 409s one watched page may spend (R3's
 * L-7): ONE — its two lines, the response and Chromium's console line, each
 * forgiven once, so `dealAnswers` reads at most 2 a page. A page meets the 409
 * once: the table flipped to playing between two of its requests, and the
 * screen goes to the board. A second is breakage, so a client that looped on
 * the refusal fails the walk rather than having its lines forgiven by the
 * hundred.
 */
export const DEAL_ANSWERS_PER_PAGE = 1;

/**
 * Everything the page said that a player would call broken.
 *
 * Console ERRORS and uncaught exceptions only. Warnings are not collected: a
 * warning that is not a defect would make this a list nobody reads, and React's
 * own development warnings do not reach a production build anyway.
 *
 * **Failed responses are filtered to THIS ORIGIN.** The reason this was written
 * has since been REMOVED — `index.html` pulled two font families from
 * `fonts.googleapis.com`, and an unfiltered watch made the whole browser gate
 * fail on somebody else's outage or on a machine with no network. M6.5b2
 * self-hosted both (R-6.5b2.18), so the release fetches nothing off-origin at
 * all and this filter drops nothing today. It stays as a POLICY rather than a
 * workaround: a gate that can go red because a third party is down is a gate
 * that says nothing about this repository, and the next off-origin thing
 * somebody adds should not be able to do that. Art 404s are a supported state
 * in the client, which draws its typed fallback, but not in a checkout that has
 * the art committed, which every checkout does; those are same-origin and still
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
 *
 * **Under `LIVE`, and only there, ONE more error is forgiven** — the Web
 * Analytics beacon's CORS block ([`isBeaconCorsError`]) — and COUNTED in
 * `forgiven`, so a live run says how often the allow-list was spent. Locally
 * the switch is off and the line below it never runs.
 *
 * **And in BOTH modes, the dealing screen's designed 409** ([`isDealAnswer`],
 * M6.5c's live tier, finding 3): both of its lines — the response and
 * Chromium's console line — counted apart, in `dealAnswers`, so a run says
 * which forgiveness it spent — ONE a page, its two lines once each
 * ([`DEAL_ANSWERS_PER_PAGE`]; R3's L-7).
 */
export function watchForBreakage(
  page: Page,
  expected: { status: number; urlIncludes: string }[] = [],
): { failures: string[]; forgiven: number; dealAnswers: number } {
  const failures: string[] = [];
  const watch = { failures, forgiven: 0, dealAnswers: 0 };
  /** The deal's designed 409s this page has spent, arm by arm. */
  const dealt = { response: 0, console: 0 };
  const allowed = (status: number, url: string) =>
    expected.some((e) => e.status === status && url.includes(e.urlIncludes));
  page.on("console", (m: ConsoleMessage) => {
    // Chromium logs its own `Failed to load resource … (409 Conflict)` for the
    // same response, so an allowed status has to be forgiven in both channels
    // or the allowlist only half works.
    if (m.type() !== "error") return;
    const text = m.text();
    if (expected.some((e) => text.includes(String(e.status)))) return;
    // The CONSOLE arm is the only one the beacon reaches: its load fails
    // cross-origin, so nothing under `BASE_URL` answers it and the response
    // arm below never sees it.
    if (LIVE && isBeaconCorsError(text, m.location().url)) {
      watch.forgiven += 1;
      return;
    }
    if (isDealAnswerLine(text, m.location().url, BASE_URL) && dealt.console < DEAL_ANSWERS_PER_PAGE) {
      dealt.console += 1;
      watch.dealAnswers += 1;
      return;
    }
    failures.push(`console.error: ${text}`);
  });
  page.on("pageerror", (e) => failures.push(`uncaught: ${e.message}`));
  page.on("response", (r) => {
    if (r.status() >= 400 && r.url().startsWith(BASE_URL) && !allowed(r.status(), r.url())) {
      if (isDealAnswer(r.status(), r.request().method(), r.url(), BASE_URL) && dealt.response < DEAL_ANSWERS_PER_PAGE) {
        dealt.response += 1;
        watch.dealAnswers += 1;
        return;
      }
      failures.push(`${r.status()} ${r.url()}`);
    }
  });
  return watch;
}

/** A table's deal route on `base`'s origin: `/api/games/<id>/deal` and
 *  nothing after it. */
function isDealRoute(url: string, base: string): boolean {
  let at: URL;
  try {
    at = new URL(url);
  } catch {
    return false;
  }
  return at.origin === new URL(base).origin && /^\/api\/games\/[^/]+\/deal$/.test(at.pathname);
}

/**
 * **The dealing screen's designed 409** (M6.5c Task 15, the live tier's finding
 * 3; ruling I-81): a `GET …/api/games/<id>/deal` answered 409 on `base`.
 *
 * `Dealing.tsx`'s own doc: "A 409 here is an ANSWER, not a failure" — the
 * typed refusal of a table that flipped to playing between two of the
 * browser's requests, which the client answers by going to the board. The live
 * run met it on the join spec's first page; locally the race falls the other
 * way. So it is forgiven in BOTH modes — the race is the product's on every
 * host — and nothing broader is: no other status, no other method, no other
 * path (`/deal2`, `/deals`, `/deal/x`), no other origin. Pure, so its cells are
 * node cells (`harness.node.test.ts`).
 */
export function isDealAnswer(status: number, method: string, url: string, base: string): boolean {
  return status === 409 && method === "GET" && isDealRoute(url, base);
}

/** Chromium's console line for that response — "Failed to load resource: the
 *  server responded with a status of 409 (<reason>)", the reason empty over
 *  HTTP/2 — matched only where Chromium LOCATES it at a deal route. */
const DEAL_409_LINE = /^Failed to load resource: the server responded with a status of 409 \([^)]*\)$/;

/** The console half of [`isDealAnswer`]: its line, at a deal route on
 *  `base`. */
export function isDealAnswerLine(text: string, url: string, base: string): boolean {
  return DEAL_409_LINE.test(text) && isDealRoute(url, base);
}

/**
 * The Web Analytics beacon's STABLE address: every release is served below
 * it, at a versioned path (`…/beacon.min.js/v<hash>`) the next release will
 * change.
 */
export const BEACON_PREFIX = "https://static.cloudflareinsights.com/beacon.min.js";

/** Chromium's CORS sentence — "Access to <kind> at '<address>' from origin
 *  '<origin>' has been blocked by CORS policy: <reason>" — and the address it
 *  names as BLOCKED. */
const CORS_BLOCKED = /^Access to .+? at '([^']+)' from origin '[^']*' has been blocked by CORS policy\b/;

/** What Chromium logs for the same load a line later. */
const LOAD_FAILED = "Failed to load resource: net::ERR_FAILED";

/**
 * **Whether a console error is the live tier's one known error** (the
 * orchestrator's ruling I-75): the Web Analytics beacon, which the zone's edge
 * injects into every page, refused by CORS.
 *
 * Why it happens, and only live: the live contexts carry the claimed header
 * as `extraHTTPHeaders` (`live.ts` says why), which Chromium adds to the
 * beacon's cross-origin script load as well; a custom header makes that load
 * ask a CORS preflight, and the beacon's host does not grant it (Task 11's
 * diagnosis). A real player sends no such header and sees no error. The zone
 * stays as it is, by the owner's ruling.
 *
 * MEASURED (pinned Chromium, a local page and a local "beacon"): ONE blocked
 * load logs TWO errors, so both are this error —
 *
 *   1. the CORS sentence, located at the PAGE, naming the blocked address:
 *      matched by the CLASS (any resource kind, any reason after its colon),
 *      and the address it names as BLOCKED must be the beacon's;
 *   2. `Failed to load resource: net::ERR_FAILED`, whose text names neither
 *      the beacon nor CORS: matched only where Chromium LOCATES it at the
 *      beacon's address.
 *
 * "The beacon's address" is the stable prefix itself or anything below it
 * (`/`) or on it (`?`): never another host, a lookalike host, another path, or
 * a longer name. A pure function, so its cells are node cells
 * (`harness.node.test.ts`).
 */
export function isBeaconCorsError(text: string, url: string): boolean {
  const blocked = CORS_BLOCKED.exec(text)?.[1];
  if (blocked !== undefined) return fromTheBeacon(blocked);
  return text === LOAD_FAILED && fromTheBeacon(url);
}

function fromTheBeacon(url: string): boolean {
  return url === BEACON_PREFIX || url.startsWith(`${BEACON_PREFIX}/`) || url.startsWith(`${BEACON_PREFIX}?`);
}

/**
 * The GAME screen's own heading, and why it is scoped.
 *
 * **The front door has an `<h1>Yokohama</h1>` of its own since M6.5b2** — the
 * product, named on the first screen everyone sees (Task 3 review, Minor 5) —
 * and Playwright matches an accessible name as a SUBSTRING by default. So a
 * bare `getByRole("heading", { name: "Yokohama" })` is now satisfied by the
 * DOOR, and a walk that never reached a game would pass the assertion that says
 * it did. `header.banner` is the element the door does not draw.
 *
 * The deal screen draws one too (`Dealing.tsx`), which is deliberate and
 * unchanged: every caller below either is already past the deal or is asserting
 * the move from the deal into the game.
 */
export function atTheGame(page: Page): Locator {
  return page.locator("header.banner").getByRole("heading", { level: 1, name: "Yokohama" });
}

/** What `startGame` hands back: enough to name a failure.
 *
 *  **The seed is gone, and its absence is the point (M6.1).** It used to ride
 *  here so a failing walk could be reproduced, read off the preview response —
 *  the one payload that still carried it. There is no preview in the flow any
 *  more and no play payload carries a seed at all: it regenerates both decks,
 *  the boxed pile and every deal, so a client that held one would be holding the
 *  hidden set. A failure names the table, and the table's record on the server
 *  is what reproduces it. */
export interface Game {
  /** The table id, which is also the button that reopens it. */
  id: string;
}

/** Who the walk plays as. */
export const WALKER = "walker@e2e.test";

/**
 * Go to the app and get through the door, exactly as a player does.
 *
 * **Through the SCREEN, not around it (M6.1 §7.1).** Until the sign-in screen
 * existed this seeded `localStorage` through `addInitScript`, which was a
 * stand-in and said so. It is a form now, so the walk types into it — which is
 * the only thing that also tests the screen that puts the name there.
 *
 * The form is the NAME STEP since M6.5c2 (the open door): it takes an address
 * as well as a name, and the walk types the address it has always used.
 *
 * **`email` is a parameter because two browsers are two people.** Every walk but
 * one signs in as [`WALKER`], which is why it is the default; `two_browsers`
 * signs its two contexts in under two different addresses, and the identity a
 * context holds is the whole subject there.
 *
 * Idempotent, because several specs navigate more than once in a test and the
 * name survives inside a page's storage: if the door is already open this just
 * waits for the corner, or the strip, that says so. The wait is on EITHER, because until
 * `/api/me` answers the lobby draws only "Connecting to the server…" — neither
 * the name step nor the corner — so a bare `count()` would read zero for a
 * player who is signed in perfectly well.
 *
 * **The default lands on `#/lobby` since M6.5b2** (R-6.5b2.1). `/` is the front
 * door, which is drawn from this browser alone and has no sign-in on it at all:
 * a walk that went there and waited for a form would wait for ever. The server's
 * own door stands on the lobby, which is where every caller of this helper is
 * headed anyway.
 */
export async function signIn(page: Page, email: string = WALKER, url = "/#/lobby"): Promise<void> {
  await page.goto(url);
  const step = page.getByRole("heading", { name: "Choose a name" });
  // Signed in reads two ways: the lobby's corner, whose title is the identity
  // (M6.5c2), and the "You are …" strip every other screen still draws.
  const escaped = email.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const signedIn = page
    .locator(".identity-corner")
    .getByTitle(email, { exact: true })
    .or(page.getByText(new RegExp(`^You are ${escaped}$`)));
  await expect(step.or(signedIn).first()).toBeVisible();
  if (await step.count()) {
    await page.getByLabel("Name", { exact: true }).fill(email);
    await page.getByRole("button", { name: "Continue" }).click();
  }
  await expect(signedIn.first()).toBeVisible();
}

/**
 * Set what this browser calls itself — the name a seat it takes is labelled
 * with (M6.5c2, R-6.5c2.8). The forms and the join no longer ask for one — the
 * lobby's corner owns it — so a walk that reads a seat's label says, here, what
 * it is. The key is `api.ts`'s `DISPLAY_NAME_KEY`, spelled as a literal the way
 * `plays_a_whole_game` spells the claimed user's.
 */
export async function nameThisBrowser(page: Page, name: string): Promise<void> {
  await page.evaluate((n) => localStorage.setItem("yokohama.display_name", n), name);
}

/**
 * Open a table from the LOBBY's Your tables, BY ID.
 *
 * The lobby is the only way into a table this browser did not create, which is
 * how the second person at a two-browser table gets to it (M6.2 §5.2). A row's
 * button is labelled with the table's name, and a table opened without one is
 * named by its id — so `exact: true` is load-bearing: `getByRole` matches by
 * substring by default, and another row's name can contain this one. The read
 * is scoped to Your tables, where a table you sit at is listed (and, since
 * M6.5c2, listed only there).
 *
 * **The row is a LINK** (§5.2, R-14), so the click only sets the HASH; what the
 * table then draws is `App`'s route effect asking the server, and the caller
 * waits for the screen it wants. The row also carries an Abandon button for a
 * table this browser opened, which is why the query names the role as well as
 * the text.
 */
export async function openFromList(page: Page, id: string): Promise<void> {
  // **`exact: true`, because Playwright matches an accessible name by
  // SUBSTRING** (unlike the component suite, which matches it whole). M6.4
  // put a "Lobby chat" panel on this screen, whose header reads "Lobby chat
  // show" with its fold control in it — so the loose name resolves to two
  // headings and every walk in this directory fails in strict mode.
  await expect(page.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  // **`.your-games` and no longer `.saved` first.** M6.5b puts "On this
  // device" at the top of this screen, in the same `.saved` shape — so
  // `.first()` stopped meaning "Your games" the moment the games in the
  // browser were listed above it, which is the same drift `:not(.open-tables)`
  // met in `joins_by_link`. The section names itself; use the name.
  await page.locator(".lobby .your-games").getByRole("link", { name: id, exact: true }).click();
}

/**
 * Get to the setup FORM, the way a player does: through the door, then the
 * lobby, then New table (M6.2 §5.1).
 *
 * Its own helper because four specs stop at the form rather than playing on —
 * the deal is the picture they are about — and the path to it moved this cycle.
 */
export async function newGameForm(page: Page, email: string = WALKER): Promise<void> {
  await signIn(page, email);
  // **`exact: true`, because Playwright matches an accessible name by
  // SUBSTRING** (unlike the component suite, which matches it whole). M6.4
  // put a "Lobby chat" panel on this screen, whose header reads "Lobby chat
  // show" with its fold control in it — so the loose name resolves to two
  // headings and every walk in this directory fails in strict mode.
  await expect(page.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "New table" }).click();
  // **`level: 3`**: the band draws its last crumb as the page's `<h2>`, which
  // reads "New table" on this screen too, and strict mode refuses a locator
  // that matches two (M6.5c2).
  await expect(page.getByRole("heading", { name: "New table", exact: true, level: 3 })).toBeVisible();
}

/** Start a fresh game through the lobby and the setup screen, exactly as a
 *  player does (M6.2 §5.8).
 *
 *  **`local` is the game played in THIS BROWSER** (M6.5b, R-6.5b.3): two
 *  seats, you and the bot, created by the module in this tab's own Worker.
 *  Everything after the create is the same screen and the same clicks — the
 *  deal, the keep, the game — which is the point of the option rather than a
 *  second helper.
 *
 *  **`signedOut` is for the one walk that has no door** (spec §6.5): with the
 *  API refused there is no sign-in form to type into, and a local game needs
 *  none. It is an explicit parameter and not a probe, so a walk that meant to
 *  be signed in fails at the door rather than quietly playing without one. */
export async function startGame(
  page: Page,
  opts: { players?: number; local?: boolean; signedOut?: boolean } = {},
): Promise<Game> {
  const players = opts.players ?? 2;
  if (opts.local) {
    if (players !== 2) throw new Error("a local game is two seats, you and the bot (R-6.5b.3)");
    const id = await createLocal(page, opts.signedOut ?? false);
    await keepThroughTheDeal(page, 2);
    return { id };
  }
  // **The lobby is the first screen since M6.2**, and the form is one click
  // beyond it.
  await newGameForm(page);

  if (players !== 2) {
    await page
      .getByRole("group", { name: "Players" })
      .getByRole("radio", { name: String(players), exact: true })
      .check();
    // **The hot seat is ASKED FOR since M6.2** (§5.3): above two players the
    // form opens the other seats, so a walk that plays 3P or 4P by itself says,
    // in the form, that it holds every seat. Each seat is named by its colour
    // since M6.5c2 (mock 07), and `exact`, because a colour is also a First
    // player choice.
    for (const color of ["Blue", "Yellow", "Green"].slice(0, players - 1)) {
      await page.getByRole("combobox", { name: `${color} seat`, exact: true }).selectOption("me");
    }
  }
  // **The id comes off the RESPONSE, not off the banner.**
  //
  // This used to read `header .meta` and split it on `·`, taking the id from
  // position 2. That is a test parsing a display string for machine-readable
  // facts, and it broke the moment the banner stopped printing the seed —
  // silently, into `id === "saved"`, because the fields shifted by one and every
  // segment is still a non-empty string. The walk then spent fifteen seconds
  // waiting to click a saved game called "saved".
  //
  // Deal now CREATES the table (M6.1 §4.2) rather than asking for a preview, so
  // this is the create's own 201 and the id is the table's.
  const created = page.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await page.getByRole("button", { name: "Deal" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  if (!id) throw new Error("the server did not name the table");

  await keepThroughTheDeal(page, players);
  return { id };
}

/**
 * A local table, created through the screens a player uses (M6.5b §2.5).
 *
 * **The id comes off the ROUTE and not off a response**, because there is no
 * response: a local create is a message to this tab's Worker, which draws the
 * id itself. The address the screen lands on is where that id is readable, and
 * reading it there also says the route was taken.
 */
async function createLocal(page: Page, signedOut: boolean): Promise<string> {
  if (!signedOut) await signIn(page);
  // **The FRONT DOOR since M6.5b2** (R-6.5b2.1), whichever arm got here: the
  // games in this browser and the button that starts one live at `#/`, and
  // nothing on that screen needs a server — which is the state `signedOut`
  // exists for, and the reason the signed-in arm comes back here too.
  await page.goto("/#/");
  await expect(page.getByRole("heading", { name: "On this device" })).toBeVisible();
  await page.getByRole("button", { name: "New game", exact: true }).click();
  await expect(page.getByRole("heading", { name: "New game against the bot", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Deal" }).click();
  await page.waitForURL(/#\/t\/local-[0-9a-f]+$/, { timeout: 60_000 });
  const id = page.url().split("#/t/")[1] ?? "";
  if (!id.startsWith("local-")) throw new Error(`the local create landed at ${page.url()}`);
  return id;
}

/**
 * Keep one card per seat this browser holds, until the game is on screen.
 *
 * Shared by both hosts since M6.5b, and it is the same screen on both: the
 * local deal is served by the module through the same route the server's is.
 */
async function keepThroughTheDeal(page: Page, players: number): Promise<void> {
  // **The deal, one seat at a time, served by `GET …/deal` (§4.3).** Every seat
  // this browser holds is asked in turn — one bot at two players, and above that
  // the walker in every seat, which the loop above ASKS FOR since M6.2 — and the
  // screen hands over to the game when the last keep finalizes the table. So the loop is "keep whatever I am
  // being asked, until the game is on screen", and the COUNT afterwards is what
  // says the sequence really happened.
  //
  // `:not([disabled])` matters: the pick stays in the DOM while the keep is in
  // flight, so a plain selector would find a button that cannot be clicked and
  // wait out the action timeout on the very last keep.
  const pick = page.locator(".app.setup-table.dealing button.deal-pick:not([disabled])");
  const inGame = page.locator(".app:not(.setup-table)");
  const humans = players === 2 ? 1 : players;
  let kept = 0;
  for (;;) {
    await page.waitForFunction(
      () =>
        !!document.querySelector(".app.setup-table.dealing button.deal-pick:not([disabled])") ||
        !!document.querySelector(".app:not(.setup-table)"),
      null,
      { timeout: 30_000 },
    );
    if (await inGame.count()) break;
    await pick.first().click();
    kept += 1;
    if (kept > humans) {
      throw new Error(`the deal asked for more than ${humans} keeps at ${players} players`);
    }
  }
  // Anti-vacuity, and it is the whole loop's: a screen that drew no pick at all
  // would leave this at zero and the walk would go on into a game it never dealt
  // for. It is also what says the per-seat sequence is EXERCISED above two
  // players, where this browser holds every seat.
  if (kept !== humans) {
    throw new Error(`the deal took ${kept} keeps at ${players} players, expected ${humans}`);
  }
  await expect(atTheGame(page)).toBeVisible();
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
  /** Tiles the open picker has SELECTED — `aria-pressed="true"` (M6.5c2). The
   *  walk confirms every picker it fills, and an accepted Confirm closes it, so
   *  a picker that opens with some already selected is one whose Confirm was
   *  REFUSED. */
  pressed: number;
  /** The open picker is Option B's, which takes ONE area (M6.5c2). */
  single: boolean;
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
    // **Bounded, and a miss is swallowed.** The walk's last dwell can expire in
    // the gap between the count and the click, which detaches the strip — and
    // an unbounded click then waits out the whole 15 s action timeout for a
    // control whose ABSENCE is the goal. The wait below is what actually holds
    // the property, so this only has to try.
    await skip
      .first()
      .click({ timeout: 2_000 })
      .catch(() => {});
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
      // **The pickable tile, and its shape is not a tag** (M5.5 ruling R-F2):
      // `Board` draws every tile as a `<div>` and marks the answerable ones
      // `role="button"`, because switching the element type when a picker
      // opened remounted the art. The in-flight guard rides as `aria-disabled`
      // for the same reason. This selector is what every walk in this suite
      // steers by, so it is the one to follow if that shape ever moves again.
      tiles: many('.tile[role="button"]:not([aria-disabled="true"])'),
      pressed: many('.tile[role="button"][aria-pressed="true"]'),
      // Every `.decision`, because the class is drawn more than once
      // (`deadEnd`'s note) and the prompt's need not be the first.
      single: [...document.querySelectorAll(".decision")].some((d) =>
        (d.textContent ?? "").includes("click one area on the board"),
      ),
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
export type Affordance = "option" | "only" | "placement" | "count" | "confirm" | "cancel";

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

  // A placement picker. The board is the input surface.
  //
  // Placing NOTHING is legal, but a walk that only ever did that would never
  // put an assistant on the board, and a game in which nobody places never ends
  // — measured on the fixture walk at turn 3,644 with nothing having happened
  // at all, and the server gate's vs-bot arm livelocked on exactly that until
  // M6.5c2. Option A takes up to three areas, so the count VARIES: a walk that
  // always placed exactly one never exercised a multi-area selection or the fee
  // that grows with it. Option B takes one.
  //
  // **Each tile is ASKED, as the server gate's placement probe asks** (M6.5c2,
  // spec §2.6). Every tile the client lets you toggle is clickable, but the
  // SERVER decides which it takes, and a refusal is a 200 carrying the engine's
  // line with the selection unchanged. So the walk clicks one UNPRESSED tile at
  // a time — a pressed one would toggle OFF — waits for the reply, and keeps
  // count of what the server took. A refusal costs one ask and never a Confirm.
  //
  // **And the server gate's stuck-picker rule, for its reason**: a set the server
  // took one area at a time can be unaffordable together, and the same refused
  // Confirm then comes back for ever (measured there 40,000 times).
  // - A picker that opens with a selection already made had its Confirm refused,
  //   since this walk confirms every picker it fills, so it backs OUT with Cancel.
  // - An Option B picker that took no area is backed out of the same way, since
  //   confirming it would be refused.
  // - An Option A picker that took none is confirmed empty: placing none is legal
  //   there, and it is how a walk with no assistants left gets past the phase.
  if (screen.tiles > 0 && screen.confirm) {
    if (screen.pressed > 0) return cancelPicker(page);
    const wanted = screen.single ? 1 : 1 + nth(Math.min(3, screen.tiles));
    let placed = 0;
    for (let asked = 0; placed < wanted && asked < wanted + 3; asked += 1) {
      const free = page.locator('.tile[role="button"][aria-pressed="false"]:not([aria-disabled="true"])');
      const n = await free.count();
      if (n === 0) break;
      await free.nth(nth(n)).click();
      placed = (await readScreen(page)).pressed;
    }
    if (placed === 0 && screen.single) return cancelPicker(page);
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
  return new Error(`${why}\n  game ${game.id}, after ${moves} moves\n\n${zone}`);
}

/** Back out of an open placement picker, through its own Cancel — always live
 *  there (`Decision.tsx`'s controls). */
async function cancelPicker(page: Page): Promise<Affordance> {
  await page.locator(".decision .controls button.ghost", { hasText: "Cancel" }).first().click();
  return "cancel";
}

/**
 * A walk that reached its cap, in a sentence that says so (M6.5c2, spec §2.6).
 *
 * **Never `deadEnd`'s sentence.** A cap hit is a game still RUNNING — the two
 * recorded hits (M6.3, M6.5c) were 4,001-move games with their end triggers
 * unmet — and "nothing on the screen is answerable" named a stuck table
 * instead. This names the cap, the turn and the end-trigger strip, so a hit
 * says how far the game got.
 */
export async function capReached(page: Page, game: Game, moves: number): Promise<Error> {
  const read = async (selector: string) =>
    (await page.locator(selector).first().innerText({ timeout: 2_000 }).catch(() => "")).trim();
  return new Error(capSentence(game.id, moves, await read(".end-strip"), await read(".decision .chain")));
}

/** `capReached`'s words — pure, so a node cell holds them apart from
 *  `deadEnd`'s. `strip` is printed as the screen draws it: `.end-strip`
 *  renders `view.end_triggers`, which already begins `end triggers: `, so only
 *  the fallback carries the label. */
export function capSentence(id: string, moves: number, strip: string, chain: string): string {
  return (
    `the walk reached its cap at ${moves} moves and the game is still running — ` +
    `a game this walk has not ended, not a screen with nothing to click\n` +
    `  game ${id}, ${chain || "no turn on screen"}\n` +
    `  ${strip || "end triggers: no end-trigger strip drawn"}`
  );
}

// ---- the layout measurement (M3.5's width gate, shared since M5.5) --------
//
// Lifted out of `the_screen_itself.spec.ts` verbatim when the set table needed
// the same reading (M5.5 ruling R-T12-1). A second copy would be a second
// definition of "does the page scroll sideways", and the two would drift.

/** One offending box, named well enough to fix without opening devtools. */
export interface Box {
  selector: string;
  width: number;
  right: number;
  scrollWidth: number;
  clientWidth: number;
  overflowX: string;
}

export interface OverflowRead {
  /** The page delta — what a player feels as a sideways scrollbar. */
  docScrollWidth: number;
  docClientWidth: number;
  /** The outermost boxes sticking out past the window: the CAUSE of that delta. */
  spills: Box[];
  /** The board's own scroller, which is allowed to have work to do. */
  board: Box | null;
  /** Anti-vacuity: the pyramid's widest row, in tiles. */
  widestRow: number;
}

/** Everything the layout question needs, in ONE round trip. */
export async function readOverflow(page: Page): Promise<OverflowRead> {
  return page.evaluate(() => {
    const doc = document.documentElement;
    const viewport = doc.clientWidth;

    // A path short enough to read in a failure line and specific enough to grep
    // the stylesheet for.
    const selectorOf = (el: Element): string => {
      const hop = (n: Element) => {
        const cls = [...n.classList].join(".");
        return `${n.tagName.toLowerCase()}${n.id ? `#${n.id}` : ""}${cls ? `.${cls}` : ""}`;
      };
      const parts: string[] = [];
      for (let n: Element | null = el; n && parts.length < 3; n = n.parentElement) {
        parts.unshift(hop(n));
      }
      return parts.join(" > ");
    };

    const box = (el: Element): Box => {
      const r = el.getBoundingClientRect();
      return {
        selector: selectorOf(el),
        width: Math.round(r.width),
        right: Math.round(r.right),
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
        overflowX: getComputedStyle(el).overflowX,
      };
    };

    // 1px of slack: `clientWidth` is an integer and a border box is not, so an
    // exact compare reports fractional rounding as a defect.
    const past = (el: Element) => Math.round(el.getBoundingClientRect().right) > viewport + 1;

    const spills = [...document.querySelectorAll("*")]
      .filter((el) => el.getClientRects().length > 0 && past(el))
      // OUTERMOST only. Every descendant of an over-wide column is over-wide
      // too, and a list of forty of them buries the one that is the cause.
      .filter((el) => !el.parentElement || !past(el.parentElement))
      .map(box)
      .sort((a, b) => b.width - a.width)
      .slice(0, 5);

    const board = document.querySelector(".board");
    const rows = [...document.querySelectorAll(".board-row")].map(
      (row) => row.querySelectorAll(".tile").length,
    );

    return {
      docScrollWidth: doc.scrollWidth,
      docClientWidth: viewport,
      spills,
      board: board ? box(board) : null,
      widestRow: rows.length > 0 ? Math.max(...rows) : 0,
    };
  });
}

/** The failure line: WHAT overflowed, not merely by how much. */
export function blame(read: OverflowRead): string {
  if (read.spills.length === 0) {
    return "and nothing sticks out of the window — look for a margin, a `position: absolute`, or a scroll anchor";
  }
  return read.spills
    .map(
      (s) =>
        `${s.selector} is ${s.width}px wide (right edge ${s.right} vs window ${read.docClientWidth}, ` +
        `content ${s.scrollWidth} in ${s.clientWidth}, overflow-x: ${s.overflowX})`,
    )
    .join("; ");
}

// ---- the setup screens' two widths and their two reads (M6.5c2) -----------
//
// Lifted out of `setup_screens.spec.ts` when a second spec needed them (cycle
// C, R-C7: the lobby's Finished group, read at the end of a whole game). The
// bodies are the ones that spec defined; a second copy would drift.

/** The desktop the setup screens are read at. */
export const DESKTOP = { width: 1440, height: 900 };
/** The phone the setup screens are read at (mock 11). */
export const PHONE = { width: 390, height: 844 };

/** Nothing on the page is wider than its window — read once the layout is DONE
 *  (the width gate's own note, above). */
export async function noSidewaysScroll(page: Page, what: string): Promise<void> {
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => r(null))));
  const read = await readOverflow(page);
  expect(
    read.docScrollWidth - read.docClientWidth,
    `${what} scrolls sideways: ${blame(read)}`,
  ).toBeLessThanOrEqual(0);
}

/** A picture for the headed look, under this run's own output directory. */
export async function look(page: Page, name: string): Promise<void> {
  await page.screenshot({ path: test.info().outputPath(`${name}.png`), fullPage: true });
}
