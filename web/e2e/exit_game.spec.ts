// **The exit's headed game** (M6.5c Task 16; the orchestrator's TASK 16 EXIT
// INSTRUMENT and ruling I-88): two people and a watcher at ONE locked table on
// the deployed Worker, playing a short game at a HUMAN pace — with four pauses
// long enough for the table's object to be evicted, each followed by the move
// that wakes it — which its creator then abandons.
//
//   cd web && YOKOHAMA_WEB_EXIT=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test exit_game --headed
//
// `WAKE_PAUSE_MS=<ms>` sets the four long pauses: 45 000 by default, past the
// ~35 s of idleness after which the spike measured an object evicted.
// `EXIT_SCREEN=<width>x<height>` is the screen the three windows are laid out
// on, in its logical pixels: 1280x832 by default — three columns where three
// of Chromium's narrowest windows fit across, else a grid of two rows. No
// other window may cover one of the three entirely while it runs — the
// terminal with the tail included — or that window reads hidden and the run
// fails its own precondition.
//
// **Opt-in twice over.** It plays on the owner's account and spends real
// minutes, so no run that did not ask for it by name plays it:
// `playwright.config.ts` ignores this file unless `YOKOHAMA_WEB_EXIT=1`, and
// matches it live only then; and this file skips itself anywhere but live,
// saying so. The two structural specs still read it as a file, like every
// other spec here.
//
// **What it is, and what it is not** (ruling I-88): FORTY moves, abandoned —
// NOT the whole game spec §5.5 words. The whole live two-person game is live
// run 1's (`sim-552c786c4ba671ff`, played to a winner), and the console-clean
// whole-game witness is the deferred `two_browsers` live re-run.
//
// **What a page can see of a wake, and what it cannot.** ASSERTED here: the
// three people at the table — x creates it locked, y joins through the
// lobby's Open tables link with the password, w unlocks it to watch; each
// hand on its own screen and none on the watcher's; every event the moves
// apply reaching the watcher's account, by its index, on ONE socket for the
// whole game while the players never reopen theirs — which is the sockets
// surviving every hibernation; every table-chat line reaching the watcher, and
// y reading x's line in the lobby's chat; the abandon — the marker saying the
// game is in the archive, the lobby listing it no more, y and w told; and the
// three consoles clean under the specs' rules (`watchForBreakage`: the
// beacon's CORS error and the deal's designed 409, each forgiven and
// counted). PRINTED: the round trip of the move's own request after each long
// pause — normally the first request the table hears after it, and the line
// names the first when it is not — which is the wake's cost as a page sees it,
// beside the median of the other moves'; and what the three pages asked the
// table DURING each pause (nothing, if the pause meant anything). NOT seen here at all: the eviction and the wake
// themselves. The orchestrator reads those off `wrangler tail` — the
// `wake {id, wakes}` line under `YOKOHAMA_WEB_COUNT_WAKES=on` (ruling I-84),
// the window RUNBOOK §6.7 opens and closes — with each request's CPU beside
// it.
//
// **Its own precondition: every window VISIBLE** (R3's R-N6; ruling I-91). A
// hidden page lets its table's socket go by design (R-P8), so a window
// minimized, or left on another Space, would fail the one-socket claims for a
// reason that is not the product's. Each page's visibility is watched from
// before its first navigation; what it reported while the three windows were
// opening and being placed, and the table seated, is forgotten (and printed)
// once play starts; and at every checkpoint from then — at the start of play,
// before each long pause, once the move after it has landed, and at the end
// of play — all three pages must read `visible` and have reported nothing
// else since; a run that breaks that fails saying it is not a finding about
// the product (`live.ts`'s `mustBeVisible`). The exit's run launches Chromium
// with the two switches meant to keep a covered window visible
// (`playwright.config.ts`), and on macOS they do not: on the first headed run
// (deployment 19) the three windows stacked and x's read hidden at move 5. On
// the second, three columns too narrow for Chromium overlapped. So the spec
// lays its windows out by what Chromium draws, under the menu bar the window
// manager keeps them below, and READS the placement BACK: two windows that
// overlap, or one that leaves the screen, fail the same precondition
// (`exitLayout`, `mustBeApart`).
//
// **What it leaves on the account**: what any live spec's abandon leaves —
// the game's save under the bucket's `test/` root (all three people are
// `.test` identities) and the table's 90-day marker (RUNBOOK §6) — and TWO
// posts in the REAL house chat, one each from x and y, which real people see
// in the lobby until newer messages trim them.

import { type Page, type Request, type WebSocket } from "@playwright/test";
import { expect, newContextWithoutWorker, test } from "./sw";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { act, deadEnd, readScreen, rng, signIn, watchForBreakage, type Affordance } from "./harness";
import {
  EXIT,
  LIVE,
  abandonAsCreator,
  describeBounds,
  exitLayout,
  forgetHidden,
  liveContextOptions,
  liveRequest,
  mustBeApart,
  mustBeVisible,
  placeWindow,
  watchVisibility,
  type ExitLayout,
} from "./live";
import { appliedEvents, chatOf, hasMove, ownHand, reads, say, socketTo, unfold } from "./shared_table";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");
// And not asked for, or not live: SKIP, saying how it is asked for.
test.skip(
  !(EXIT && LIVE),
  "the exit's headed game plays only against the deployed Worker, and only when asked for by name: " +
    "YOKOHAMA_WEB_EXIT=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test exit_game --headed",
);

/** The three people: `.test` identities, so the game's save files under the
 *  bucket's `test/` root. */
const X = "x@exit.test";
const Y = "y@exit.test";
const W = "w@exit.test";

/** How many moves the two of them play before x abandons the table. */
const MOVES = 40;

/** The moves AFTER which the pause is a long one, so the next move is the one
 *  that wakes the object. */
const WAKE_AFTER = [5, 15, 25, 35] as const;

/** The long pause's default: past the ~35 s of idleness after which the spike
 *  measured an object evicted. */
const DEFAULT_WAKE_PAUSE_MS = 45_000;

/** Every other pause: a person reading the screen before they click. */
const HUMAN_PAUSE_MS = [1_000, 2_000] as const;

/** Consecutive looks at both screens with nothing pressable on either before
 *  the walk gives up — `joins_by_link`'s bound, and for its reason. */
const IDLE_LIMIT = 60;

/** How long the walk waits for the first request after a pause. */
const REQUEST_WAIT_MS = 30_000;

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** `WAKE_PAUSE_MS`, or the default — and a value that is not a whole number
 *  of milliseconds is refused by NAME rather than read as something else. */
function wakePause(): number {
  const raw = process.env.WAKE_PAUSE_MS;
  if (raw === undefined || raw === "") return DEFAULT_WAKE_PAUSE_MS;
  const ms = Number(raw);
  if (!Number.isInteger(ms) || ms < 0) {
    throw new Error(`WAKE_PAUSE_MS must be a whole number of milliseconds, 0 or more — got ${JSON.stringify(raw)}`);
  }
  return ms;
}

/** Every request, and every socket, that `pages` open to table `id` while
 *  this is installed — what "the table was left alone" is read from. */
function tap(pages: Page[], id: string): { seen: string[]; stop: () => void } {
  const base = `/api/games/${id}`;
  const seen: string[] = [];
  const onRequest = (r: Request) => {
    const path = new URL(r.url()).pathname;
    if (path === base || path.startsWith(`${base}/`)) seen.push(`${r.method()} ${path}`);
  };
  const onSocket = (ws: WebSocket) => {
    const path = new URL(ws.url()).pathname;
    if (path.startsWith(`${base}/`)) seen.push(`socket ${path}`);
  };
  for (const p of pages) {
    p.on("request", onRequest);
    p.on("websocket", onSocket);
  }
  return {
    seen,
    stop: () => {
      for (const p of pages) {
        p.off("request", onRequest);
        p.off("websocket", onSocket);
      }
    },
  };
}

/** The first request any of `pages` makes that `wanted` accepts, from the
 *  moment this is called — `null` if none comes within `REQUEST_WAIT_MS` of
 *  `settle()` being asked. */
function firstRequest(pages: Page[], wanted: (r: Request) => boolean): { settle: () => Promise<Request | null> } {
  let found: Request | null = null;
  let resolve: ((r: Request) => void) | null = null;
  const on = (r: Request) => {
    if (found !== null || !wanted(r)) return;
    found = r;
    resolve?.(r);
  };
  for (const p of pages) p.on("request", on);
  return {
    settle: async () => {
      let timer: ReturnType<typeof setTimeout> | undefined;
      const got =
        found ??
        (await Promise.race([
          new Promise<Request>((r) => {
            resolve = r;
          }),
          new Promise<null>((r) => {
            timer = setTimeout(() => r(null), REQUEST_WAIT_MS);
          }),
        ]));
      clearTimeout(timer);
      for (const p of pages) p.off("request", on);
      return got;
    },
  };
}

/** A request's own round trip, as the browser timed it: request start to the
 *  last byte of its answer, and the status it was answered with. */
async function roundTrip(r: Request): Promise<{ ms: number | null; status: number | null }> {
  const res = await r.response();
  if (res === null) return { ms: null, status: null };
  await res.finished();
  const end = r.timing().responseEnd;
  return { ms: end >= 0 ? Math.round(end) : null, status: res.status() };
}

/** One long pause and the request that followed it. */
interface Checkpoint {
  /** The moves played before the pause. */
  after: number;
  pausedMs: number;
  /** When the pause began, UTC — what the tail's `wake` line is read beside. */
  from: string;
  /** What the three pages asked the table while it was left alone. */
  meanwhile: string[];
  /** The first request of ANY kind to the table after the pause, and the
   *  move's own — normally one and the same request. */
  first: string | null;
  move: { ms: number | null; status: number | null } | null;
  /** Whether the move applied an event, and the watcher's account reached it. */
  applied: boolean;
}

/** The table this walk opened, and the page of the person who opened it. */
let opened: { page: Page; id: string } | null = null;

// **The table is abandoned whatever happens**, as its creator, in an
// `afterEach` and not a `finally` — for the measured reason
// `two_browsers.spec.ts` gives: a timed-out body's `finally` runs only after
// its pages are closed. The walk abandons it itself at its end, which is one
// of the things it is here to see; this is for a walk that did not get there.
test.afterEach(async () => {
  const table = opened;
  opened = null;
  if (LIVE && table) await abandonAsCreator(table.page, table.id);
});

test("two people and a watcher play a paced game across four hibernations, and its creator abandons it", async ({
  browser,
}) => {
  const WAKE_PAUSE = wakePause();
  // Fifteen minutes at the default pause, and four times any longer pause on
  // top: the long pauses are most of the wall, and an override must not run
  // the walk out of its budget by arithmetic.
  test.setTimeout(15 * 60 * 1000 + 4 * Math.max(0, WAKE_PAUSE - DEFAULT_WAKE_PAUSE_MS));
  const started = Date.now();
  const stamp = started.toString(36);
  const NAME = `exit test ${stamp}`;
  const PASSWORD = `exit-${stamp}`;
  const pick = rng(0x4558_4954); // "EXIT"

  // **Three windows, none covering another** (exit game runs 1 and 2,
  // deployment 19): stacked, a covered window read hidden; in three columns
  // too narrow for Chromium, they overlapped. So the layout is chosen by what
  // Chromium draws (`live.ts`, `exitLayout`), under the top inset the window
  // manager keeps windows below — read off x's first placement — and every
  // window is placed before its page navigates, with the viewport of its
  // window, and READ BACK: two that overlap, or one that leaves the screen,
  // fail the spec's own precondition (`mustBeApart`). `EXIT_SCREEN` is the
  // screen's logical size; a value that cannot be read, or a screen too small,
  // is refused before a table is opened.
  const probeLayout = exitLayout(process.env.EXIT_SCREEN, 0);
  const open = async (email: string, viewport: ExitLayout["viewport"]) =>
    (await newContextWithoutWorker(browser, { ...(await liveContextOptions(email)), viewport })).newPage();
  const x = await open(X, probeLayout.viewport);
  const probe = await placeWindow(x, probeLayout.windows[0]);
  const layout = exitLayout(process.env.EXIT_SCREEN, Math.max(0, (probe.top ?? 0) - probeLayout.windows[0].top));
  await x.setViewportSize(layout.viewport);
  const y = await open(Y, layout.viewport);
  const w = await open(W, layout.viewport);
  const placed = [
    ["x", await placeWindow(x, layout.windows[0])],
    ["y", await placeWindow(y, layout.windows[1])],
    ["w", await placeWindow(w, layout.windows[2])],
  ] as const;
  mustBeApart(placed, layout.screen, "once the three windows were placed");
  // The precondition's record starts here, before any page navigates.
  const windows = [
    ["x", x, await watchVisibility(x)],
    ["y", y, await watchVisibility(y)],
    ["w", w, await watchVisibility(w)],
  ] as const;
  /** How many checkpoints read all three windows visible. */
  let visibleReads = 0;
  const visible = async (where: string) => {
    await mustBeVisible(windows, where);
    visibleReads += 1;
  };
  const watchX = watchForBreakage(x);
  const watchY = watchForBreakage(y);
  const watchW = watchForBreakage(w);

  /** This table's stream path — empty until the server has named the table —
   *  and how often each page opened it, read at request time. */
  let streamPath = "";
  const opens = { x: 0, y: 0, w: 0 };
  for (const [k, p] of [["x", x], ["y", y], ["w", w]] as const) {
    p.on("request", (r) => {
      if (new URL(r.url()).pathname === streamPath) opens[k] += 1;
    });
    p.on("websocket", (ws) => {
      if (socketTo(ws, streamPath)) opens[k] += 1;
    });
  }

  // ---- phase 1: seating --------------------------------------------------
  //
  // x in the lobby, and the one line x says there: the house chat is the
  // lobby's own object, so nothing said there touches the table's.
  const lobbyChat = (p: Page) => p.locator(".lobby section.lobby-chat");
  const sayInLobby = async (p: Page, text: string) => {
    const fold = lobbyChat(p).getByRole("button", { name: "show" });
    if (await fold.count()) await fold.click();
    await lobbyChat(p).getByRole("textbox", { name: "Say something" }).fill(text);
    await lobbyChat(p).getByRole("textbox", { name: "Say something" }).press("Enter");
    await expect(lobbyChat(p).getByRole("listitem").filter({ hasText: text })).toBeVisible();
  };
  const lineX = `exit test ${stamp}: x here, in an automated game — back shortly`;
  const lineY = `exit test ${stamp}: y here, joining x's table`;
  await signIn(x, X);
  await expect(x.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  await sayInLobby(x, lineX);

  // x opens a two-seat table with seat 1 OPEN and a password on it — the
  // `joins_by_link` form, with a name only this run uses.
  await x.getByRole("button", { name: "New table" }).click();
  await expect(x.getByRole("heading", { name: "New game" })).toBeVisible();
  await x.getByLabel("Seat 1", { exact: true }).selectOption("open");
  await x.getByLabel("Name", { exact: true }).fill(NAME);
  await x.getByLabel("Ask for a password to join").check();
  await x.getByLabel("Table password", { exact: true }).fill(PASSWORD);
  const created = x.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await x.getByRole("button", { name: "Open table" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  expect(id, "the server named the table").toBeTruthy();
  opened = { page: x, id };
  streamPath = `/api/games/${id}/stream`;
  await expect(x).toHaveURL(new RegExp(`#/t/${id}$`));

  // y in the lobby: reads x's line there, says its own, and finds the table
  // under Open tables — the join link a person who was told nothing would
  // follow. Its row is found by the NAME and read back by the ADDRESS, which
  // is how the end of this walk asks whether the lobby still lists it.
  await signIn(y, Y);
  await expect(y.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  const foldY = lobbyChat(y).getByRole("button", { name: "show" });
  if (await foldY.count()) await foldY.click();
  await expect(
    lobbyChat(y).getByRole("listitem").filter({ hasText: lineX }),
    "y reads x's line in the lobby's chat",
  ).toBeVisible({ timeout: 15_000 });
  await sayInLobby(y, lineY);
  const listed = (p: Page) => p.locator(`.lobby a[href$="#/t/${id}"]`);
  const joinLink = y.locator(".lobby .open-tables").getByRole("link", { name: NAME, exact: true });
  await expect(joinLink, "the table is under Open tables in y's lobby").toBeVisible({ timeout: 30_000 });
  await expect(listed(y), "one row links to the table: this is what the end reads").toHaveCount(1);
  await joinLink.click();
  await expect(y.getByRole("heading", { name: NAME, exact: true })).toBeVisible();
  await expect(y.getByText(/password required/)).toBeVisible();
  await y.getByLabel("Your name", { exact: true }).fill("y (exit test)");
  await y.getByLabel("Password", { exact: true }).fill(PASSWORD);
  await y.getByRole("button", { name: "sit here" }).click();

  // Both keep, and the last keep starts the table.
  for (const p of [x, y]) {
    const keep = p.getByRole("button", { name: /^keep O\d\d/ }).first();
    await expect(keep).toBeVisible({ timeout: 30_000 });
    await keep.click();
  }
  await expect(y.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  await expect(x.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  // Anti-vacuity for every hand read below: each seat holds the order it
  // kept, so the selector finds a page's own cards rather than nothing.
  expect((await ownHand(x)).length, "x's own hand is on x's screen").toBeGreaterThan(0);
  expect((await ownHand(y)).length, "y's own hand is on y's screen").toBeGreaterThan(0);

  // w arrives by the table's address, is stopped by the password, and
  // watches.
  await signIn(w, W);
  await w.goto(`/#/t/${id}`);
  await expect(w.getByRole("heading", { name: "Lobby", exact: true })).toHaveCount(0);
  const watchButton = w.getByRole("button", { name: "Watch" });
  await expect(watchButton).toBeVisible();
  await w.getByLabel("Password").fill(PASSWORD);
  await watchButton.click();
  await expect(w.getByText("You are watching")).toBeVisible();
  await expect(w.getByText(/end triggers:/)).toBeVisible({ timeout: 15_000 });
  expect(await ownHand(w), "a watcher is dealt no hand").toEqual([]);
  await unfold(w);
  const seated = Math.round((Date.now() - started) / 1000);
  // **The precondition is read from the first move on** (the second headed
  // run): what a window reported as not visible while the three were opening
  // over one another and being placed, and while the table was seated, is
  // not the claim. It is forgotten here — and said in the line below — and
  // every checkpoint from now reads what came after it, starting at once.
  const forgotten = forgetHidden(windows);
  // eslint-disable-next-line no-console
  console.log(
    `exit phase 1, seating: table ${id} ("${NAME}") opened locked by ${X}; ${Y} read x's lobby line, ` +
      `said its own, and joined through the lobby's Open tables link with the password; both kept; ` +
      `${W} unlocked it and is watching with no hand; the windows — ` +
      `${layout.kind === "grid" ? "a grid, x and y above and w below x" : "three columns"}, under a ` +
      `${layout.inset} px top inset — read back x ${describeBounds(placed[0][1])}, y ${describeBounds(placed[1][1])}, ` +
      `w ${describeBounds(placed[2][1])} (EXIT_SCREEN ${layout.screen.width}x${layout.screen.height}; viewport ` +
      `${layout.viewport.width}x${layout.viewport.height}); reported not visible while they opened and the table ` +
      `was seated, and forgotten: ${forgotten.length === 0 ? "nothing" : forgotten.join(", ")}; ${seated}s`,
  );
  await visible("at the start of play");

  // ---- phase 2: the paced game -------------------------------------------
  const opensAtStart = { x: opens.x, y: opens.y };
  let lastApplied = 0;
  /** How many applied moves the watcher was held to — its account, by index. */
  let heldTo = 0;
  /** What the watcher's account is checked against after a move: the event
   *  count, asked of the SERVER from x's context (`shared_table.ts`). Whether
   *  THIS move applied one is the answer. */
  const watchMove = async (mover: Page): Promise<boolean> => {
    // The mover's own request has to have landed, or the count read below is
    // the one from before the move.
    await mover.waitForFunction(() => !document.querySelector("[data-busy]"), null, { timeout: 15_000 });
    const applied = await appliedEvents(x, id, X);
    if (applied === lastApplied) return false;
    lastApplied = applied;
    await expect(
      w.locator(".turn-account li[data-event]").last(),
      `the watcher's account reaches event ${applied - 1}`,
    ).toHaveAttribute("data-event", String(applied - 1), { timeout: 30_000 });
    expect(await ownHand(w), "the watcher still holds no hand").toEqual([]);
    heldTo += 1;
    return true;
  };

  const said = { x: 0, y: 0 };
  /** One table-chat line, and the watcher's count of lines rising with it. */
  const chat = async (p: Page, who: "x" | "y", text: string) => {
    const before = await chatOf(w).locator("ol.chat-lines li").count();
    await say(p, text);
    await reads(w, text);
    expect(await chatOf(w).locator("ol.chat-lines li").count(), "the watcher's chat count rises").toBeGreaterThan(before);
    said[who] += 1;
  };

  const checkpoints: Checkpoint[] = [];
  /** The round trips of the moves that did NOT follow a long pause. */
  const awake: number[] = [];
  let idleLooks = 0;
  const playStarted = Date.now();
  for (let played = 0; played < MOVES; ) {
    const wake = (WAKE_AFTER as readonly number[]).includes(played);
    let meanwhile: ReturnType<typeof tap> | null = null;
    if (wake) {
      // Everything the last move set off has landed on all three screens —
      // `data-busy` covers a screen's fetch and its walk — before the table is
      // left alone, so what is tapped below is the pause and not its tail.
      for (const p of [x, y, w]) {
        await p.waitForFunction(() => !document.querySelector("[data-busy]"), null, { timeout: 60_000 });
      }
      await visible(`before the pause after move ${played}`);
      meanwhile = tap([x, y, w], id);
    }
    const pause = wake ? WAKE_PAUSE : HUMAN_PAUSE_MS[0] + Math.floor(pick() * (HUMAN_PAUSE_MS[1] - HUMAN_PAUSE_MS[0] + 1));
    const from = new Date().toISOString().slice(11, 19) + "Z";
    await sleep(pause);
    meanwhile?.stop();

    // The first request to the table from here on, of any kind; and the
    // move's own POST, which is normally the same request.
    const base = `/api/games/${id}`;
    const anyFirst = firstRequest([x, y, w], (r) => {
      const path = new URL(r.url()).pathname;
      return path === base || path.startsWith(`${base}/`);
    });
    const moveFirst = firstRequest(
      [x, y],
      (r) => r.method() === "POST" && new URL(r.url()).pathname === `${base}/input`,
    );

    // One move, on whichever screen offers one — looked at until one does.
    let mover: Page | null = null;
    for (let looks = 0; mover === null; looks += 1) {
      for (const p of [x, y]) {
        const s = await readScreen(p);
        if (s.over) throw new Error(`the exit's game ended after ${played} moves — a Yokohama game cannot be over this soon`);
        if (!hasMove(s)) continue;
        // **A click that misses is a finding only if the screen still SAYS
        // there is a move** — the race `two_browsers.spec.ts` measured.
        let acted: Affordance | null = null;
        try {
          acted = await act(p, s, pick);
        } catch (e) {
          if (hasMove(await readScreen(p))) throw e;
          continue;
        }
        if (acted !== null) {
          mover = p;
          break;
        }
        if (hasMove(await readScreen(p))) throw await deadEnd(p, { id }, played);
      }
      if (mover !== null) break;
      idleLooks += 1;
      if (looks > IDLE_LIMIT) throw await deadEnd(x, { id }, played);
      await sleep(400);
    }
    played += 1;

    const [firstOfAll, firstOfMove] = [await anyFirst.settle(), await moveFirst.settle()];
    const trip = firstOfMove === null ? null : await roundTrip(firstOfMove);
    const applied = await watchMove(mover);
    if (wake) {
      await visible(`once the move after the pause at ${played - 1} had landed`);
      checkpoints.push({
        after: played - 1,
        pausedMs: pause,
        from,
        meanwhile: meanwhile!.seen,
        first: firstOfAll === null ? null : `${firstOfAll.method()} ${new URL(firstOfAll.url()).pathname}`,
        move: trip,
        applied,
      });
    } else if (trip?.ms != null) {
      awake.push(trip.ms);
    }

    // The table's chat, between moves, from both seats in turn — the move
    // zero-padded, so no line is a substring of another (`hasText` matches by
    // substring, and "move 2" is inside "move 22").
    const n = String(played).padStart(2, "0");
    if (played % 4 === 2) await chat(x, "x", `x: after move ${n}`);
    else if (played % 4 === 0) await chat(y, "y", `y: after move ${n}`);
  }
  await visible("at the end of play");
  /** The stream counts at the END of play, which is what the claim is about:
   *  the abandon below ends every screen's stream, and nothing after it is
   *  play. */
  const opensAtEnd = { ...opens };
  const sorted = [...awake].sort((a, b) => a - b);
  const median = sorted.length === 0 ? null : sorted[Math.floor(sorted.length / 2)]!;
  const watcherLines = await chatOf(w).locator("ol.chat-lines li").count();
  const describeCheckpoint = (c: Checkpoint) =>
    `after move ${c.after}: paused ${(c.pausedMs / 1000).toFixed(1)} s from ${c.from}, ` +
    `${c.meanwhile.length === 0 ? "nothing asked meanwhile" : `asked meanwhile: ${c.meanwhile.join(", ")}`} → ` +
    (c.move === null
      ? "no move request within the wait"
      : `the move's POST …/input ${c.move.status ?? "unanswered"} in ${c.move.ms ?? "an untimed"} ms`) +
    (c.first !== null && !c.first.endsWith("/input") ? ` (the first request was ${c.first})` : "") +
    (c.applied ? ", the watcher reached its event" : ", which applied no event");
  // eslint-disable-next-line no-console
  console.log(
    `exit phase 2, play: ${MOVES} moves in ${Math.round((Date.now() - playStarted) / 1000)}s ` +
      `(pauses ${HUMAN_PAUSE_MS[0] / 1000}–${HUMAN_PAUSE_MS[1] / 1000} s; ${idleLooks} idle looks); ` +
      `the long pauses — ${checkpoints.map(describeCheckpoint).join("; ")}; ` +
      `the other moves' POST round trip, median ${median ?? "n/a"} ms over ${awake.length}; ` +
      `table chat x ${said.x}, y ${said.y}, and the watcher's panel holds ${watcherLines} lines; ` +
      `the watcher was held to ${heldTo} applied moves with no hand; all three windows read visible at ` +
      `${visibleReads} checkpoints; this table's stream opened ` +
      `w ${opensAtEnd.w}, x ${opensAtStart.x}→${opensAtEnd.x}, y ${opensAtStart.y}→${opensAtEnd.y} times`,
  );

  // ---- phase 3: the abandon ----------------------------------------------
  //
  // x abandons its table with the DELETE a player's Abandon sends, through x's
  // own context (`abandonAsCreator`): with the archive on, the Worker saves the
  // game to the bucket first, then deletes the table and leaves its marker in
  // its place.
  await abandonAsCreator(x, id);
  opened = null;
  // The marker answers the table's own address, and says where the game is.
  const marker = await liveRequest(() => x.request.get(`${BASE_URL}/api/games/${id}`, { maxRedirects: 0 }));
  const markerText = await marker.text();
  let sentence = markerText;
  try {
    sentence = (JSON.parse(markerText) as { error?: string }).error ?? markerText;
  } catch {
    /* not this API's JSON: the text is what there is */
  }
  expect(marker.status(), "the abandoned table's address answers its marker").toBe(404);
  expect(sentence, "the marker says it was abandoned and its game is in the archive").toMatch(
    /^this table was abandoned on \d{4}-\d{2}-\d{2} — its game is in the archive$/,
  );
  // Every screen at the table is told, and taken to the lobby.
  for (const [who, p] of [["x", x], ["y", y], ["w", w]] as const) {
    await expect(p.getByText("this table was abandoned"), `${who} is told the table was abandoned`).toBeVisible({
      timeout: 30_000,
    });
  }
  const gone = Date.now();
  await expect(x.getByRole("heading", { name: "Lobby", exact: true })).toBeVisible();
  await expect(listed(x), "the lobby no longer lists the abandoned table").toHaveCount(0, { timeout: 30_000 });
  // eslint-disable-next-line no-console
  console.log(
    `exit phase 3, abandon: ${X} abandoned ${id}; its address answers ${marker.status()} "${sentence}"; ` +
      `x, y and w were told; the lobby lists it no more (${Math.round((Date.now() - gone) / 1000)}s after)`,
  );

  // ---- the line the run is read by ---------------------------------------
  const consoleOf = (failures: string[]) =>
    failures.length === 0 ? "clean" : `${failures.length} failure(s), the first: ${failures[0]}`;
  // eslint-disable-next-line no-console
  console.log(
    `exit game ${id}: ${MOVES} moves; the long pauses ` +
      checkpoints
        .map((c) => `${c.after}: ${(c.pausedMs / 1000).toFixed(1)} s → ${c.move?.ms ?? "untimed"} ms`)
        .join(", ") +
      ` (awake median ${median ?? "n/a"} ms); chat — table x ${said.x} / y ${said.y} (the watcher's panel ` +
      `${watcherLines}), lobby x 1 / y 1; forgiven — the beacon's CORS error x ${watchX.forgiven}, ` +
      `y ${watchY.forgiven}, w ${watchW.forgiven}; the deal's 409 x ${watchX.dealAnswers}, ` +
      `y ${watchY.dealAnswers}, w ${watchW.dealAnswers}; console — x ${consoleOf(watchX.failures)}, ` +
      `y ${consoleOf(watchY.failures)}, w ${consoleOf(watchW.failures)}; ` +
      `${Math.round((Date.now() - started) / 1000)}s`,
  );

  // What the lines above say, asserted — after they are printed, so a red run
  // still says everything it saw.
  expect(checkpoints.map((c) => c.after), "four long pauses, each followed by a move").toEqual([...WAKE_AFTER]);
  for (const c of checkpoints) {
    expect(c.move, `the move after the pause at ${c.after} made a request`).not.toBeNull();
    expect(c.move!.status, `the move after the pause at ${c.after} was answered`).toBeLessThan(400);
  }
  // Anti-vacuity for the watching half: a walk in which no move applied an
  // event would have held the watcher to nothing.
  expect(heldTo, "the watcher was held to the account of real moves").toBeGreaterThan(0);
  // **The sockets survived every hibernation**: ONE for the watcher, and none
  // reopened by a player while the game was played.
  expect(opensAtEnd.w, "the watcher held ONE socket to this table for the whole game").toBe(1);
  expect(opensAtEnd.x, "x never reopened this table's stream while the game was played").toBe(opensAtStart.x);
  expect(opensAtEnd.y, "y never reopened this table's stream while the game was played").toBe(opensAtStart.y);
  expect(watchX.failures, "x's page reported no errors").toEqual([]);
  expect(watchY.failures, "y's page reported no errors").toEqual([]);
  expect(watchW.failures, "w's page reported no errors").toEqual([]);
});
