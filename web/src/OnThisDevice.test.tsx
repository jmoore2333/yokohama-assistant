// The first screen's first section (R-6.5b.8): the games in THIS BROWSER.
//
// **Every payload here comes from the Worker, which is the point.** The section
// reads the local dispatch's own `GET /api/games?scope=mine`, so a cell that
// stubbed `fetch` would prove nothing about the guarantee it exists for — that
// the games on this device are reachable with the API unreachable.

// **`fake-indexeddb` since M6.5b2**, because the section now reads the
// compatibility rows straight from this browser's database to decide which
// rows to hide (review M-2). Installed at module scope, as `db.test.ts` does;
// the cells that are not about it get an empty store and hide nothing, which
// is what they got before when the read simply failed.
import "fake-indexeddb/auto";
import { IDBFactory } from "fake-indexeddb";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OnThisDevice } from "./OnThisDevice";
import { api, rememberServerFacts } from "./api";
import { resetFactsForTest } from "./local/facts";
import { enqueue, resetOutboxForTests } from "./outbox";
import { stubApi } from "./test/harness";
import { wire } from "./wire/contract";
import { resetLocalHostForTests } from "./local/client";
import { openDb, restoreOne } from "./local/db";
import { currentWorker, stubWorker } from "./test/fakeWorker";
import { resetInstallForTests } from "./install";
import { requestPersistence, resetPersistenceRequestForTests } from "./storage";
import { stubDesktopMac, stubIosSafari, stubStorage } from "./test/platform";
import type { TableView } from "./types";
import productionBot from "./wire/production_bot.json";

/** The row the fake local host lists — the corpus's own `TableView`, named for
 *  this device (see `fakeWorker.ts`'s note about the one field a fixture is
 *  allowed to lose). */
const LOCAL = "local-1";

const section = (): HTMLElement =>
  screen.getByRole("heading", { name: "On this device" }).closest("section")!;

describe("On this device", () => {
  beforeEach(() => {
    globalThis.indexedDB = new IDBFactory();
    resetLocalHostForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
  });

  it("lists the games the module answers for, each a link to its table", async () => {
    stubWorker();
    render(<OnThisDevice />);
    const row = await screen.findByRole("link", { name: LOCAL });
    expect(row).toHaveAttribute("href", `#/t/${LOCAL}`);
    // The read went to the MODULE. `fetch` is the setup file's throwing stub,
    // so a section that asked the server would have failed loudly instead.
    expect(
      currentWorker().posted.filter((p) => p.kind === "call" && p.path === "/api/games"),
    ).toHaveLength(1);
    expect(fetch).not.toHaveBeenCalled();
  });

  // **A row says which bot the game is against, and what it is waiting on**
  // (R-6.5c2.10, mock 10), read off the listing's own `TableView.bot` — never a
  // parse of the record, which `db.ts` forbids. The row is DERIVED, because the
  // corpus game is bot-free by construction: `wire.table` is read by seat 1's
  // holder, so seat 0 becomes the bot, the turn (`to_move`) moves to the
  // caller's seat, and `bot` carries the contract file's own label. Those three
  // fields moved, and the id is `fakeWorker.ts`'s one permitted edit.
  it("names each game's bot and what it is waiting on, and draws no per-row backup", async () => {
    const seats = [...wire.table.seats];
    seats[0] = { kind: "bot" };
    const game: TableView = {
      ...wire.table,
      id: LOCAL,
      seats,
      to_move: 1,
      bot: { label: productionBot.label, runs_here: true },
    };
    stubWorker({
      call: (req) =>
        req.path === "/api/games" && req.method === "GET"
          ? { status: 200, body: { games: [game], next: null } }
          : undefined,
    });
    render(<OnThisDevice />);
    const link = await screen.findByRole("link", { name: wire.table.name! });
    expect(link).toHaveAttribute("href", `#/t/${LOCAL}`);
    expect(link.closest("li")).toHaveTextContent(`vs ${productionBot.label} · your move`);
    // The approved card draws no per-row "Back up"; the whole device's is in the foot.
    expect(within(section()).queryByRole("button", { name: "Back up" })).toBeNull();
    expect(within(section()).getByRole("button", { name: "Back up everything" })).toHaveTextContent("Back up");
  });

  // **P-6**: a local game is abandoned on its own screen, where the tab holds
  // it. A control here would delete a game this browser is not holding.
  it("draws no abandon control on a local row", async () => {
    stubWorker();
    render(<OnThisDevice />);
    await screen.findByRole("link", { name: LOCAL });
    expect(within(section()).queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  // **One game, one row** (review M-2). A restored cross-rev pair whose TABLE
  // document parses is a row the layer's own listing builds happily — the
  // mechanism spec §2.5 describes is for the case the listing SKIPS a document,
  // and the common case is the other one. So the door drew the same game twice,
  // once as playable and once under "Will not open here", with different
  // controls on each. The row that says why it will not open is the true one.
  //
  // **Its anti-vacuity partner is the first cell in this file**, which is this
  // arrangement WITHOUT the compatibility row and finds the link: the absence
  // below is a hidden row and not an empty listing, and the listing's own
  // request is asserted to have happened as well.
  it("hides a playable row for a game this device cannot open", async () => {
    const db = await openDb();
    await restoreOne(db, LOCAL, {
      table: "{}",
      compat: JSON.stringify({
        semantics_rev: 6,
        rules_hash: null,
        reason: "saved at semantics rev 6; this app plays rev 7",
        restored_at: 0,
      }),
    });
    stubWorker();
    render(<OnThisDevice />);
    await waitFor(() =>
      expect(
        currentWorker().posted.filter((p) => p.kind === "call" && p.path === "/api/games"),
      ).toHaveLength(1),
    );
    await waitFor(() => expect(screen.queryByRole("link", { name: LOCAL })).toBeNull());
    // **And it does NOT say there is nothing here.** The game is on this device
    // and is listed below with the reason it will not open; "no game on this
    // device yet" would be a false sentence about a game the player can see.
    expect(within(section()).queryByText(/no game on this device yet/)).toBeNull();
  });

  // ---- M6.5c2: a game against a bot this build does not run (spec §2.4) ---
  //
  // A NOT-OVER local game whose bot this build does not run is refused by the
  // module, so it is listed under "Will not open here" — built from the
  // LISTING's own `bot.runs_here`, never from parsing the record (`db.ts`
  // forbids that) — with Back up and a two-click Remove, the cross-rev row's
  // pattern (R-6.5b2.12). A FINISHED one opens as it always did (R-6.5c2.10).
  describe("a game against a bot this build does not run", () => {
    const played = wire.foreignBot409.foreign_bot.played;
    /** The corpus's own foreign row (`wire.tableForeign`, `runs_here: false`),
     *  re-addressed to this device — `fakeWorker.ts`'s one permitted edit —
     *  with its name `null`, as the corpus row already has it and as the
     *  listing's rule wants it (a row is found by its id); and `over` the ONE
     *  value a cell moves, because no committed listing carries a FINISHED
     *  foreign game. */
    const row = (id: string, over: boolean): TableView => ({
      ...wire.tableForeign,
      id,
      name: null,
      over,
    });

    it("lists a game that is not over under Will not open here, not as a link, with Back up and Remove", async () => {
      stubWorker({
        call: (req) =>
          req.method === "GET" && req.path === "/api/games"
            ? { status: 200, body: { games: [row("local-9", false)], next: null } }
            : undefined,
      });
      render(<OnThisDevice />);
      const block = (await screen.findByRole("heading", { name: "Will not open here" })).closest(".unplayable")!;
      expect(within(block as HTMLElement).getByText("local-9")).toBeInTheDocument();
      expect(within(block as HTMLElement).getByText(new RegExp(`played against ${played}`))).toBeInTheDocument();
      expect(within(block as HTMLElement).getByRole("button", { name: "Back up" })).toBeInTheDocument();
      expect(within(block as HTMLElement).getByRole("button", { name: "remove local-9" })).toBeInTheDocument();
      expect(screen.queryByRole("link", { name: "local-9" })).toBeNull();
    });

    it("keeps a FINISHED game against a foreign bot as an ordinary row: it opens as it always did", async () => {
      stubWorker({
        call: (req) =>
          req.method === "GET" && req.path === "/api/games"
            ? { status: 200, body: { games: [row("local-9", true)], next: null } }
            : undefined,
      });
      render(<OnThisDevice />);
      expect(await screen.findByRole("link", { name: "local-9" })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: "Will not open here" })).toBeNull();
    });

    it("removes it on the second click through the module's delete, and the row goes", async () => {
      let games = [row("local-9", false)];
      stubWorker({
        call: (req) => {
          if (req.method === "GET" && req.path === "/api/games")
            return { status: 200, body: { games, next: null } };
          if (req.method === "DELETE" && req.path === "/api/games/local-9") {
            games = [];
            return { status: 204, body: undefined };
          }
          return undefined;
        },
      });
      render(<OnThisDevice />);
      const user = userEvent.setup();
      await user.click(await screen.findByRole("button", { name: "remove local-9" }));
      await user.click(screen.getByRole("button", { name: "confirm removing local-9" }));
      await waitFor(() => expect(screen.queryByRole("heading", { name: "Will not open here" })).toBeNull());
      expect(
        currentWorker()
          .posted.map((p) => (p.kind === "call" ? `${p.method} ${p.path}` : p.kind))
          .filter((s) => s === "hold" || s === "release" || s.startsWith("DELETE")),
      ).toEqual(["hold", "DELETE /api/games/local-9", "release"]);
    });
  });

  it("re-reads when something local is written", async () => {
    stubWorker();
    render(<OnThisDevice />);
    await screen.findByRole("link", { name: LOCAL });
    const listings = () =>
      currentWorker().posted.filter((p) => p.kind === "call" && p.path === "/api/games" && p.method === "GET");
    expect(listings()).toHaveLength(1);

    // A write on the local host — a create, a move, a delete — is what the
    // listing has to follow: nothing else tells this section that the game it
    // is drawing has just become somebody's turn.
    await api.remove(LOCAL);
    await waitFor(() => expect(listings()).toHaveLength(2));
  });

  // **A fault is a LINE and never an empty list** (spec §2.3). The two
  // failures reach this section by different paths and both have to say
  // something: the read's own rejection, and the Worker's `error` event, which
  // arrives while no read of this section's is even in flight.
  it("says why when the local host cannot be started at all", async () => {
    // No `Worker` in this browser: `new Worker(...)` throws where the client
    // starts one, and the read rejects rather than resolving empty.
    vi.stubGlobal("Worker", undefined);
    render(<OnThisDevice />);
    expect(await screen.findByText(/could not be read/)).toBeInTheDocument();
    expect(screen.queryByText(/no game on this device yet/)).toBeNull();
  });

  it("says why when the module faults after the section is drawn", async () => {
    // The read is left in flight, so the sentence on screen can only have come
    // from the FAULT and not from a rejected listing.
    stubWorker({ call: () => "pending" });
    render(<OnThisDevice />);
    await waitFor(() => expect(currentWorker().posted).toHaveLength(1));
    currentWorker().raise("the module failed to load");
    const line = await screen.findByText(/could not be read/);
    expect(line).toHaveTextContent("the module failed to load");
    expect(screen.queryByText(/no game on this device yet/)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// M6.5b2: installable and persistent (R-6.5b2.9, spec §2.4).
//
// **The two offers live in the section the games are in**, which is the whole
// of why they are here: a promise about storage and a way to carry the games
// off this device are statements ABOUT these rows, and a player reading them
// anywhere else would have to hold two screens in their head at once.

describe("keeping the games on this device", () => {
  beforeEach(() => {
    globalThis.indexedDB = new IDBFactory();
    resetLocalHostForTests();
    resetInstallForTests();
    resetPersistenceRequestForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
    resetInstallForTests();
    resetPersistenceRequestForTests();
  });

  /** The section drawn with the local host answering from the corpus. */
  const draw = async () => {
    stubWorker();
    render(<OnThisDevice />);
    await screen.findByRole("link", { name: LOCAL });
  };

  const offer = (): HTMLElement => document.querySelector<HTMLElement>(".install-offer")!;

  // **The sentence that must be on the screen that offers the install** (spec
  // §2.4): an installed iOS app gets its own storage container and shares
  // nothing with Safari, so the games played in the browser do not travel.
  it("says the installed app will not see these games, beside a way to carry them", async () => {
    stubIosSafari();
    await draw();
    expect(screen.getByText(/will not see the games/i)).toBeInTheDocument();
    // **Beside it, and that is asserted as ADJACENCY rather than as presence.**
    // The backup controls are in this section whatever the platform, so
    // "a button exists somewhere" would pass with the sentence at the other
    // end of the page. The block that carries the sentence is the block the
    // control sits next to.
    const controls = offer().nextElementSibling as HTMLElement;
    expect(controls).toHaveClass("backup-controls");
    expect(within(controls).getByRole("button", { name: /back up/i })).toBeInTheDocument();
  });

  it("names Share and Add to Home Screen, and is not a modal", async () => {
    stubIosSafari();
    await draw();
    const text = offer().textContent ?? "";
    expect(text).toMatch(/Share/);
    expect(text).toMatch(/Add to Home Screen/);
    // Never a modal (spec §2.4): it is a block in the flow, so the games it is
    // about are still on screen and still reachable.
    expect(offer().closest("[role='dialog']")).toBeNull();
    expect(screen.getByRole("link", { name: LOCAL })).toBeVisible();
  });

  it("stops offering the instruction once it is dismissed, and stays quiet on the next visit", async () => {
    stubIosSafari();
    await draw();
    await userEvent.setup().click(within(offer()).getByRole("button", { name: /dismiss/i }));
    await waitFor(() => expect(document.querySelector(".install-offer")).toBeNull());

    // The next VISIT: everything this page held is gone and only the browser's
    // memory is left, which is what "remembered per browser" has to mean.
    cleanup();
    resetLocalHostForTests();
    resetInstallForTests();
    await draw();
    expect(document.querySelector(".install-offer")).toBeNull();
    // The anti-vacuity half: the section itself really did draw.
    expect(screen.getByRole("button", { name: "Back up everything" })).toBeInTheDocument();
  });

  it("offers a real button where the browser offers one, and prompts with it", async () => {
    await draw();
    const event = new Event("beforeinstallprompt") as Event & { prompt?: () => Promise<void> };
    const prompt = vi.fn().mockResolvedValue(undefined);
    event.prompt = prompt;
    // The offer arrives AFTER the first paint, which is when Chromium fires it.
    act(() => {
      fireEvent(window, event);
    });
    const button = await screen.findByRole("button", { name: "Install app" });
    await userEvent.setup().click(button);
    expect(prompt).toHaveBeenCalledTimes(1);
    // And the iOS instruction is nowhere near it: the two are alternatives.
    expect(screen.queryByText(/Add to Home Screen/)).toBeNull();
  });

  it("offers nothing at all on a browser that cannot install and is not iOS", async () => {
    stubDesktopMac();
    await draw();
    expect(document.querySelector(".install-offer")).toBeNull();
  });

  // ---- what the browser has promised (spec §2.4) --------------------------

  it("reports a grant, the usage and the quota", async () => {
    stubStorage({
      persisted: vi.fn().mockResolvedValue(true),
      estimate: vi.fn().mockResolvedValue({ usage: 54_103, quota: 2_000_000_000 }),
    });
    await draw();
    const line = await screen.findByText(/promised to keep/i);
    expect(line).toHaveTextContent("54 KB of 2 GB");
    // **No wording claims safety** (spec §2.4): a grant is useful and an export
    // is still necessary, so the line does not say the games are safe or
    // permanent.
    expect(line.textContent ?? "").not.toMatch(/safe|permanent|for ?ever|guarantee/i);
  });

  it("says so when the browser has NOT promised", async () => {
    stubStorage({
      persisted: vi.fn().mockResolvedValue(false),
      estimate: vi.fn().mockResolvedValue({ usage: 12_000, quota: 1_200_000_000 }),
    });
    await draw();
    const line = await screen.findByText(/has not promised/i);
    expect(line).toHaveTextContent("12 KB of 1.2 GB");
  });

  it("says the browser will not say, where there is no storage manager", async () => {
    stubStorage(undefined);
    await draw();
    expect(await screen.findByText(/does not say/i)).toBeInTheDocument();
  });

  // **The report is never the reason a game cannot be reached.** It is drawn
  // beside the rows and asks the browser two questions; a browser that throws
  // on both must still list the games.
  it("draws the games when every storage read throws", async () => {
    stubStorage({
      persisted: () => {
        throw new Error("the operation is insecure");
      },
      estimate: () => {
        throw new Error("the operation is insecure");
      },
    });
    await draw();
    expect(screen.getByRole("link", { name: LOCAL })).toBeInTheDocument();
    expect(await screen.findByText(/does not say/i)).toBeInTheDocument();
  });

  // **Never at boot** (spec §2.4). The section reads the report, which is two
  // questions; it must not ask for the grant, which is a PROMPT in some
  // browsers and a decision a visitor who never plays has not been given a
  // reason to make.
  it("reads the report without asking for the grant", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({
      persist,
      persisted: vi.fn().mockResolvedValue(false),
      estimate: vi.fn().mockResolvedValue({ usage: 1, quota: 2 }),
    });
    await draw();
    await screen.findByText(/has not promised/i);
    expect(persist).not.toHaveBeenCalled();
  });

  // **The line moves when the grant does** (review M-5). The ask happens at
  // the first committed move, which is on another screen; a report read once
  // at mount would still say "has not promised" when the player came back to
  // the door inside the same page session. `requestPersistence` tells its
  // listeners, and this is the one subscriber.
  it("re-reads the report when the grant has been asked for", async () => {
    let granted = false;
    stubStorage({
      persist: vi.fn(async () => {
        granted = true;
        return true;
      }),
      persisted: vi.fn(async () => granted),
      estimate: vi.fn().mockResolvedValue({ usage: 1_000, quota: 1_000_000 }),
    });
    await draw();
    expect(await screen.findByText(/has not promised/i)).toBeInTheDocument();

    await act(async () => {
      await requestPersistence();
    });
    expect(await screen.findByText(/has promised to keep/i)).toBeInTheDocument();
    expect(screen.queryByText(/has not promised/i)).toBeNull();
  });

  // **"These games" is a false subject with no rows** (review M-6). The line
  // is about what the browser will do with games played here, and the backup
  // instruction has nothing to be about until there is something to back up.
  it("words itself for a device with no games on it", async () => {
    stubStorage({
      persisted: vi.fn().mockResolvedValue(false),
      estimate: vi.fn().mockResolvedValue({ usage: 0, quota: 1_000_000_000 }),
    });
    stubWorker({
      call: (req) =>
        req.path === "/api/games" && req.method === "GET"
          ? { status: 200, body: { games: [], next: null } }
          : undefined,
    });
    render(<OnThisDevice />);
    await screen.findByText(/no game on this device yet/);
    const line = await screen.findByText(/has not promised/i);
    expect(line.textContent ?? "").not.toMatch(/these games/i);
    expect(line.textContent ?? "").not.toMatch(/back them up/i);
    // The anti-vacuity half: with a game listed the same line says both.
    cleanup();
    resetLocalHostForTests();
    await draw();
    const withRows = await screen.findByText(/has not promised/i);
    expect(withRows).toHaveTextContent("these games");
    expect(withRows).toHaveTextContent("Back them up");
  });
});

// ---- the outbox on the door (M6.5b2, R-6.5b2.13; spec §2.6) ----------------
//
// **This is the only screen the queue is visible on**, and that is the point:
// the send is attempted when nobody is watching — at the end of a game, on an
// `online` event, at the next launch — so the row a player comes back to is
// what tells them it did not land and why.
describe("the games waiting to reach the archive", () => {
  beforeEach(() => {
    globalThis.indexedDB = new IDBFactory();
    resetLocalHostForTests();
    resetOutboxForTests();
    resetFactsForTest();
  });
  afterEach(() => {
    resetLocalHostForTests();
    resetOutboxForTests();
    resetFactsForTest();
  });

  /** The section, once its rows have been read out of the store. */
  const waiting = async (): Promise<HTMLElement> =>
    (await screen.findByRole("heading", { name: "Waiting to reach the archive" })).closest(
      "div",
    ) as HTMLElement;

  // The ordinary state is the empty one, and a heading over an empty list would
  // be a permanent reminder of a mechanism that is working.
  it("draws nothing at all when nothing is waiting", async () => {
    stubWorker();
    render(<OnThisDevice />);
    await screen.findByRole("link", { name: LOCAL });
    expect(screen.queryByRole("heading", { name: "Waiting to reach the archive" })).toBeNull();
  });

  it("names the game and says which of the four reasons it is", async () => {
    stubWorker();
    await enqueue(LOCAL, "offline");
    render(<OnThisDevice />);
    const box = await waiting();
    expect(within(box).getByText(LOCAL)).toBeInTheDocument();
    expect(within(box).getByText("no connection")).toBeInTheDocument();
  });

  it("says not yet known for a game finished before any health answer", async () => {
    stubWorker();
    await enqueue(LOCAL, "unknown");
    render(<OnThisDevice />);
    expect(within(await waiting()).getByText("not yet known")).toBeInTheDocument();
  });

  // **"Not signed in" names BOTH doors under `open`** (spec §2.6): passing
  // the turnstile mints the Access cookie, and `POST /api/archive` then still
  // refuses in this API's own shape until the lobby's name step has supplied
  // the claimed user. Two steps, and the row says so — the second in the name
  // step's own words since M6.5c2, where the open door takes a name.
  it("names both doors under open, and one under cf-access", async () => {
    stubWorker();
    await enqueue(LOCAL, "signed-out");
    rememberServerFacts({ ...wire.health, identity: "open" });
    render(<OnThisDevice />);
    let box = await waiting();
    expect(within(box).getByText("not signed in")).toBeInTheDocument();
    expect(within(box).getByRole("link", { name: /sign in at the door/i })).toHaveAttribute(
      "href",
      "/enter",
    );
    expect(within(box).getByText(/then choose a name in the lobby/i)).toBeInTheDocument();

    cleanup();
    rememberServerFacts({ ...wire.health, identity: "cf-access" });
    render(<OnThisDevice />);
    box = await waiting();
    // The turnstile is the whole of it. A second step named here would send a
    // player looking for a form this deployment does not have.
    expect(within(box).getByRole("link", { name: /sign in at the door/i })).toBeInTheDocument();
    expect(within(box).queryByText(/then choose a name in the lobby/i)).toBeNull();
  });

  // **The remedy follows the door the server NAMES, even when it is named after
  // the row is drawn** (review M-5). The identity is read through the facts
  // subscription and not off the module during render — the same shape the
  // archive fact retired one file over, and a module read here would leave the
  // row saying whatever it mounted with.
  it("picks up the door's name when the health read lands after the row is on screen", async () => {
    stubWorker();
    await enqueue(LOCAL, "signed-out");
    render(<OnThisDevice />);
    const box = await waiting();
    // Nothing has said which door this is, so the turnstile is the whole of it.
    expect(within(box).queryByText(/then choose a name in the lobby/i)).toBeNull();

    act(() => {
      rememberServerFacts({ ...wire.health, identity: "open" });
    });
    // No re-render was asked for; the subscription is what redraws.
    expect(await within(await waiting()).findByText(/then choose a name in the lobby/i)).toBeInTheDocument();
  });

  // **A permanent refusal carries the server's own sentence and offers no
  // retry**: these bytes will be refused the same way for ever, and a button
  // that only asks again would be a button that never works.
  it("carries the server's sentence on a refusal, and offers no Send now", async () => {
    stubWorker();
    await enqueue(LOCAL, "refused", "a different record is archived under this id");
    render(<OnThisDevice />);
    const box = await waiting();
    expect(
      within(box).getByText("a different record is archived under this id"),
    ).toBeInTheDocument();
    expect(within(box).queryByRole("button", { name: /send .* now/i })).toBeNull();
  });

  // The third trigger (spec §2.6): the player's own button.
  it("sends the pair on the player's own button, and the row goes", async () => {
    const posts: unknown[] = [];
    stubWorker({ pair: () => ({ record: '{"seed":1,"events":[]}', table: "{}" }) });
    stubApi((call) => {
      if (call.path.endsWith("/api/health")) return { ...wire.health, archive: "dir" };
      if (call.method === "POST" && call.path === "/api/archive") {
        posts.push(call.body);
        return wire.archivePost;
      }
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await enqueue(LOCAL, "offline");
    render(<OnThisDevice />);
    const box = await waiting();
    await userEvent.setup().click(
      within(box).getByRole("button", { name: `send ${LOCAL} to the archive now` }),
    );
    await waitFor(() => expect(posts).toHaveLength(1));
    await waitFor(() =>
      expect(screen.queryByRole("heading", { name: "Waiting to reach the archive" })).toBeNull(),
    );
  });
});
