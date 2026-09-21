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
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FrontDoor, multiplayerOffered } from "./FrontDoor";
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
