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
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OnThisDevice } from "./OnThisDevice";
import { api } from "./api";
import { resetLocalHostForTests } from "./local/client";
import { openDb, restoreOne } from "./local/db";
import { currentWorker, stubWorker } from "./test/fakeWorker";

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
