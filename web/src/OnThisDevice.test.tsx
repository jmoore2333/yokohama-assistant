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
import { api } from "./api";
import { resetLocalHostForTests } from "./local/client";
import { openDb, restoreOne } from "./local/db";
import { currentWorker, stubWorker } from "./test/fakeWorker";
import { resetInstallForTests } from "./install";
import { requestPersistence, resetPersistenceRequestForTests } from "./storage";
import { stubDesktopMac, stubIosSafari, stubStorage } from "./test/platform";

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

  it("takes you to the local form, which needs no door", async () => {
    stubWorker();
    render(<OnThisDevice />);
    await screen.findByRole("link", { name: LOCAL });
    await userEvent.setup().click(screen.getByRole("button", { name: "Play the bot here" }));
    expect(window.location.hash).toBe("#/new/local");
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
    const button = await screen.findByRole("button", { name: "Install" });
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
