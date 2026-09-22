// The front door at `#/` (R-6.5b2.1), which everyone sees.
//
// **The two entries are not two of a kind, and that is what these cells pin.**
// "Play on this device" is a SCREEN this browser draws from its own IndexedDB
// and reaches with no server at all; "Play with others" is a plain link to
// `/enter`, so the login is an ordinary navigation the browser already knows
// how to do. A `fetch` there would follow Access's redirect to another origin
// and fail opaquely (spec §0.3), which is why the `href` is asserted by value
// rather than a click handler being asserted to exist.
//
// **The multiplayer boolean is BUILD-TIME** (R-6.5b2.7): the door asks the
// server nothing, because a fresh profile has nothing remembered and a request
// here is the one thing the ruling forbids. The runtime switch is what
// ENFORCES; this only informs the door.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrontDoor, multiplayerOffered } from "./FrontDoor";
import type { ReadinessStatus } from "./swRegister";
import { resetLocalHostForTests } from "./local/client";
import { stubWorker } from "./test/fakeWorker";

/** The row the fake local host lists — the corpus's own `TableView`, named for
 *  this device (`fakeWorker.ts`'s one permitted edit). */
const LOCAL = "local-1";

describe("the front door", () => {
  beforeEach(() => {
    resetLocalHostForTests();
    stubWorker();
  });
  afterEach(() => {
    resetLocalHostForTests();
  });

  // "Play with others" is a LINK and never a fetch, so the browser handles the
  // Access login the way it handles any navigation.
  it("offers multiplayer as a navigation to the turnstile", () => {
    render(<FrontDoor multiplayer />);
    const link = screen.getByRole("link", { name: /play with others/i });
    expect(link).toHaveAttribute("href", "/enter");
    // **The two arms are EXCLUSIVE**, or a component that drew both would pass
    // this cell and its partner below (Task 3 review, Minor 4).
    expect(screen.queryByText(/single player only/i)).toBeNull();
  });

  // **The first screen everyone sees says what the site is** (Task 3 review,
  // Minor 5): before this the landing screen's highest heading was
  // `OnThisDevice`'s `<h3>`, so the document started at level 3 and a stranger
  // at `#/` was told nothing at all.
  it("names the product at the top of the document", () => {
    render(<FrontDoor multiplayer />);
    expect(screen.getByRole("heading", { level: 1, name: "Yokohama" })).toBeInTheDocument();
  });

  // **The local channel is FIRST, and that is a claim about the door.**
  // `FrontDoor.tsx` argues it in prose — "the entry that needs nothing" — and
  // until this cell a refactor that put the turnstile above it passed
  // everything here (Task 3 review, Minor 3).
  it("puts the channel that needs nothing above the one that needs a login", () => {
    render(<FrontDoor multiplayer />);
    const local = screen.getByRole("heading", { name: "On this device" });
    const others = screen.getByRole("link", { name: /play with others/i });
    expect(local.compareDocumentPosition(others) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  // R-6.5b2.4 on the client side: nothing to enter, nothing offered, and one
  // line saying so rather than a control that 404s.
  it("draws no multiplayer entry when the deployment serves none", () => {
    render(<FrontDoor multiplayer={false} />);
    expect(screen.queryByRole("link", { name: /play with others/i })).toBeNull();
    expect(screen.getByText(/single player only/i)).toBeInTheDocument();
  });

  // **The other way on, and it is a whole channel rather than a link.** The
  // games in this browser are read from the module in this tab's own Worker —
  // `fetch` is the setup file's throwing stub, so a door that asked the server
  // for any part of itself fails here rather than merely being slower.
  it("draws the games on this device, and asks the server for nothing", async () => {
    render(<FrontDoor multiplayer />);
    expect(await screen.findByRole("link", { name: LOCAL })).toHaveAttribute(
      "href",
      `#/t/${LOCAL}`,
    );
    expect(screen.getByRole("button", { name: "Play the bot here" })).toBeInTheDocument();
    expect(fetch).not.toHaveBeenCalled();
  });

  // The sentence a game left behind — a local table that no longer opens — is
  // shown where its siblings are listed, which for a local game is here.
  it("carries a sentence from the screen a game was left for", () => {
    render(<FrontDoor multiplayer notice="this game is no longer available" />);
    expect(screen.getByText("this game is no longer available")).toBeInTheDocument();
  });

  // **The build value, read once and never asked for.** Absent is ON, so a
  // deployment that sets nothing serves both channels; the one string that
  // turns it off is the literal `"false"`.
  it("reads the multiplayer entry off the build and nothing else", () => {
    expect(multiplayerOffered()).toBe(true);
    vi.stubEnv("VITE_MULTIPLAYER", "false");
    expect(multiplayerOffered()).toBe(false);
    // Anti-vacuity, and the reason the test is `!== "false"` rather than a
    // truthiness read: any other value is a deployment that did not ask for the
    // door to be shut.
    vi.stubEnv("VITE_MULTIPLAYER", "true");
    expect(multiplayerOffered()).toBe(true);
    vi.stubEnv("VITE_MULTIPLAYER", "0");
    expect(multiplayerOffered()).toBe(true);
  });
});

// **What the door says about the release** (M6.5b2, spec §4's four sentences).
//
// The status is a PROP, so these cells render each state directly rather than
// arranging a service worker jsdom does not have; `swRegister.test.ts` is where
// the lifecycle that produces each one is driven.
describe("the door's readiness line", () => {
  const status = (over: Partial<ReadinessStatus> = {}): ReadinessStatus => ({
    readiness: "none",
    reason: null,
    updateWaiting: false,
    applyUpdate: vi.fn(),
    retry: vi.fn(),
    ...over,
  });

  it("says the install is happening, and then that it is done", () => {
    const { unmount } = render(<FrontDoor multiplayer readiness={status({ readiness: "installing" })} />);
    expect(screen.getByText(/preparing to play offline/i)).toBeInTheDocument();
    unmount();
    render(<FrontDoor multiplayer readiness={status({ readiness: "ready" })} />);
    expect(screen.getByText(/ready to play offline/i)).toBeInTheDocument();
    expect(screen.queryByText(/preparing to play offline/i)).toBeNull();
  });

  // **A NAMED failure with a retry** (spec §4), because the install is
  // all-or-nothing and a door that only said "failed" would leave a player with
  // nothing to do and nothing to report.
  it("names a failure and offers the install again", async () => {
    const retry = vi.fn();
    render(
      <FrontDoor
        multiplayer
        readiness={status({ readiness: "failed", reason: "/art/orders/O01.webp answered 404", retry })}
      />,
    );
    expect(screen.getByText(/\/art\/orders\/O01\.webp answered 404/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /try again/i }));
    expect(retry).toHaveBeenCalledTimes(1);
  });

  // "none" is a development build, a browser without service workers, or a page
  // the walk has flagged — three ways of having nothing to say, none of which
  // is a failure and none of which should read as one.
  it("says nothing at all when there is nothing to say", () => {
    render(<FrontDoor multiplayer readiness={status()} />);
    expect(screen.queryByText(/play offline/i)).toBeNull();
    render(<FrontDoor multiplayer />);
    expect(screen.queryByText(/play offline/i)).toBeNull();
  });

  // **The switch is the CLICK and only the click** (spec §2.3): the offer is
  // one quiet line, and nothing happens until a player presses it.
  it("offers the update and switches only when it is pressed", async () => {
    const applyUpdate = vi.fn();
    render(
      <FrontDoor multiplayer readiness={status({ readiness: "ready", updateWaiting: true, applyUpdate })} />,
    );
    const button = screen.getByRole("button", { name: /reload/i });
    expect(applyUpdate).not.toHaveBeenCalled();
    await userEvent.click(button);
    expect(applyUpdate).toHaveBeenCalledTimes(1);
  });

  it("offers no update when none is waiting", () => {
    render(<FrontDoor multiplayer readiness={status({ readiness: "ready" })} />);
    expect(screen.queryByRole("button", { name: /reload/i })).toBeNull();
  });
});
