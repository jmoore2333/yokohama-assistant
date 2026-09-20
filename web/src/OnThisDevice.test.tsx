// The first screen's first section (R-6.5b.8): the games in THIS BROWSER.
//
// **Every payload here comes from the Worker, which is the point.** The section
// reads the local dispatch's own `GET /api/games?scope=mine`, so a cell that
// stubbed `fetch` would prove nothing about the guarantee it exists for — that
// the games on this device are reachable with the API unreachable.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { OnThisDevice } from "./OnThisDevice";
import { api } from "./api";
import { resetLocalHostForTests } from "./local/client";
import { currentWorker, stubWorker } from "./test/fakeWorker";

/** The row the fake local host lists — the corpus's own `TableView`, named for
 *  this device (see `fakeWorker.ts`'s note about the one field a fixture is
 *  allowed to lose). */
const LOCAL = "local-1";

const section = (): HTMLElement =>
  screen.getByRole("heading", { name: "On this device" }).closest("section")!;

describe("On this device", () => {
  beforeEach(() => {
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
