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
import { ACCESS_CHALLENGE, SIGN_IN_NEEDED, SIGN_IN_TITLE } from "./api";
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
    // `/enter` with the screen as `return_to` since the auth cycle (R-40): no
    // hash here, so the lobby. Access ignores the query.
    expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter?return_to=%2F%23%2Flobby");
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
    expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter?return_to=%2F%23%2Flobby");
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

// **The door card is a sign-in link site** (the auth cycle, Task 6; ruling
// R-40). Under a sign-in arm a browser with no session asks `/api/me` and the
// Worker answers its own 401 with `WWW-Authenticate: Yokohama-Session` (ruling
// R-44); `call` reads it as `SIGN_IN_NEEDED`, `failureOf` as a challenge, and
// the door draws state A's first-visit card — what a FIRST visitor meets at a
// shared table link (the end-to-end cell is `App.test.tsx`'s). Under Access the
// module's 200 `user: null` still draws state B, the operator's screen. So the
// link is `/enter` with the table as `return_to` (design §4.3: the sign-in
// lands the player at the table they were sent), the build's transparency
// notice is under the link (R-A14), and under `SIGN_IN_NEEDED` the card says
// what a first visit needs — its title the sentence's words with no period
// (`SIGN_IN_TITLE`, ruling R-46), the link "Sign in" — where Access's expired
// session keeps "Your sign-in has run out". Mutants: the link without
// `return_to`; the notice drawn when unset, or left off a card; the Access
// wording drawn for a first visit; the title spelled with its period.
describe("the door card's sign-in link (auth cycle, Task 6; R-40)", () => {
  const NOTICE = "Sign-in is by WorkOS. This site keeps only your verified email address, to seat you.";
  const TABLE = "/enter?return_to=%2F%23%2Ft%2Fabc-123";

  it("A under SIGN_IN_NEEDED: the first-visit sentence, Sign in to /enter with the table as return_to, the notice under it", () => {
    window.location.hash = "#/t/abc-123";
    vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", NOTICE);
    const { unmount } = draw(state({ failure: "challenge", error: SIGN_IN_NEEDED }), false);
    // The heading is the sentence's words with no period (ruling R-46); the
    // strips keep the sentence.
    expect(screen.getByRole("heading", { level: 3, name: SIGN_IN_TITLE })).toBeInTheDocument();
    expect(`${SIGN_IN_TITLE}.`, "one spelling: the sentence is the title and its period").toBe(SIGN_IN_NEEDED);
    // Nothing on it says a sign-in ran out: there was none.
    expect(screen.queryByRole("heading", { name: "Your sign-in has run out" })).toBeNull();
    expect(screen.queryByText(/Sign in again/)).toBeNull();
    const link = screen.getByRole("link", { name: "Sign in" });
    expect(link).toHaveAttribute("href", TABLE);
    const notice = screen.getByText(NOTICE);
    expect(notice.tagName).toBe("P");
    expect(notice).toHaveClass("muted");
    expect(link.compareDocumentPosition(notice) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
    unmount();

    vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", null);
    const bare = draw(state({ failure: "challenge", error: SIGN_IN_NEEDED }), false);
    expect(screen.getByRole("link", { name: "Sign in" })).toHaveAttribute("href", TABLE);
    expect(bare.container.querySelector("p.muted"), "no notice set, nothing drawn").toBeNull();
  });

  it("A under ACCESS_CHALLENGE keeps its wording, with the same link and the notice", () => {
    window.location.hash = "#/t/abc-123";
    vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", NOTICE);
    const { unmount } = draw(state({ failure: "challenge", error: ACCESS_CHALLENGE }), false);
    expect(screen.getByRole("heading", { level: 3, name: "Your sign-in has run out" })).toBeInTheDocument();
    const link = screen.getByRole("link", { name: "Sign in again" });
    expect(link).toHaveAttribute("href", TABLE);
    const notice = screen.getByText(NOTICE);
    expect(notice).toHaveClass("muted");
    expect(link.compareDocumentPosition(notice) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    unmount();

    vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", null);
    const bare = draw(state({ failure: "challenge", error: ACCESS_CHALLENGE }), false);
    expect(bare.container.querySelector("p.muted"), "no notice set, nothing drawn").toBeNull();
  });

  it("B carries the same link and the notice", () => {
    window.location.hash = "#/t/abc-123";
    vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", NOTICE);
    const { unmount } = draw(state({ me: { ...wire.meSignedOut, mode: "cf-access" } }));
    const link = screen.getByRole("link", { name: "Sign in again" });
    expect(link).toHaveAttribute("href", TABLE);
    const notice = screen.getByText(NOTICE);
    expect(notice).toHaveClass("muted");
    expect(link.compareDocumentPosition(notice) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    unmount();

    vi.stubGlobal("__YOKOHAMA_SIGNIN_NOTICE__", null);
    const bare = draw(state({ me: { ...wire.meSignedOut, mode: "cf-access" } }));
    expect(bare.container.querySelector("p.muted"), "no notice set, nothing drawn").toBeNull();
  });
});
