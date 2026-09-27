// The door's states (M6.5c2, R-6.5c2.4; spec §2.1; mocks 03 and 05): A–D, the
// name step and the server's own refusal, each drawn from what `useMe` and the
// health read say — the discriminators spec §2.1 names.
//
// `me` is a payload, so it comes from the corpus (`wire.me`, `wire.meSignedOut`,
// and the Access door as the one field moved — `App.test.tsx`'s precedent). The
// `Door` object around it is the CLIENT's own hook state, not a payload.
//
// **Every card titles itself `<h3>`**: the door is drawn under the lobby's band,
// whose last crumb is the page's `<h2>` (`Band.tsx`), so the positive heading
// queries here pin the level too.

import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ACCESS_CHALLENGE } from "./api";
import { Door } from "./Door";
import type { Door as DoorState } from "./whoami";
import { wire } from "./wire/contract";

const state = (over: Partial<DoorState> = {}): DoorState => ({
  me: null,
  error: null,
  failure: null,
  notice: null,
  refresh: vi.fn(),
  signOut: vi.fn(),
  signedOut: vi.fn(),
  ...over,
});

const draw = (door: DoorState, serverUp: boolean | null = true, onRetry = vi.fn()) =>
  render(<Door door={door} serverUp={serverUp} onSignedIn={vi.fn()} onRetry={onRetry} />);

describe("the door's states", () => {
  // **A, and before C (Review Focus 3)**: the Access application fronts
  // `/api/health` too, so an expired session fails BOTH reads — and "can't reach
  // the server" would send a player looking at their network when one click
  // through the turnstile fixes it.
  it("A: an expired Access session is one card, with the turnstile its one way through (Review Focus 3)", () => {
    draw(state({ failure: "challenge", error: ACCESS_CHALLENGE }), false);
    expect(screen.getByRole("heading", { level: 3, name: "Your sign-in has run out" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter");
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
  });

  it("C: no answer at all — from the door's own read, or from health with the door unanswered", async () => {
    const retry = vi.fn();
    const { unmount } = draw(state({ failure: "network", error: "TypeError: Failed to fetch" }), null, retry);
    expect(screen.getByRole("heading", { level: 3, name: "Can't reach the server" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Games on this device" })).toHaveAttribute("href", "#/");
    await userEvent.setup().click(screen.getByRole("button", { name: "Try again" }));
    expect(retry).toHaveBeenCalledTimes(1);
    // The platform's words are not the player's.
    expect(screen.queryByText(/TypeError/)).toBeNull();
    unmount();
    draw(state(), false);
    expect(screen.getByRole("heading", { level: 3, name: "Can't reach the server" })).toBeInTheDocument();
  });

  // "Any other refusal of `/api/me` keeps today's `Refusal` rendering inside the
  // new card" (spec §2.1): the server's own words, and no turnstile.
  it("keeps the server's own refusal, in its own words, inside a card", () => {
    draw(state({ failure: "refused", error: wire.error.error }));
    expect(screen.getByRole("heading", { level: 3, name: "The server refused" })).toBeInTheDocument();
    expect(screen.getByText(wire.error.error)).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Sign in again" })).toBeNull();
  });

  it("D: nothing answered yet reads as connecting, over skeleton rows", () => {
    const { container } = draw(state(), null);
    expect(screen.getByRole("status")).toHaveTextContent("Connecting to the server…");
    expect(container.querySelectorAll(".skeleton-rows > div")).toHaveLength(3);
  });

  it("the open door with no name is the name step", () => {
    expect(wire.meSignedOut.mode).toBe("open");
    draw(state({ me: wire.meSignedOut }));
    expect(screen.getByRole("heading", { level: 3, name: "Choose a name" })).toBeInTheDocument();
  });

  // **B: the server answered and nobody reached it** — a player's sentence on
  // the card, and the header the edge was meant to set in a fold for whoever
  // runs the server.
  it("B: a door in front of the server that named nobody — a card for the player, a fold for the operator", () => {
    draw(state({ me: { ...wire.meSignedOut, mode: "cf-access" } }));
    expect(
      screen.getByRole("heading", { level: 3, name: "This server couldn't tell who you are" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter");
    const fold = screen.getByText("For the operator").closest("details")!;
    expect(fold).not.toHaveAttribute("open");
    expect(within(fold).getByText(/Cf-Access-Authenticated-User-Email/)).toBeInTheDocument();
    expect(screen.queryByLabelText("Name")).toBeNull();
  });

  it("draws nothing once the server names the caller", () => {
    const { container } = draw(state({ me: wire.me }));
    expect(container).toBeEmptyDOMElement();
  });
});
