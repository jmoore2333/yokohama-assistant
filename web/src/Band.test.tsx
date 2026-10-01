// The band across the top of every setup screen but the door, and the corner
// that says who is playing (M6.5c2, spec §2.1 and §2.2; mocks 02, 03, 11).
//
// `me` is a PAYLOAD, so it comes from the corpus (the M2.5 rule): `wire.me` is
// the open door with a user, `wire.meSignedOut` the open door with none, and the
// Access door is `wire.me` with its one field moved — the corpus's server runs
// the open door, the precedent `App.test.tsx`'s own cf-access cell set.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DISPLAY_NAME_KEY } from "./api";
import { IdentityCorner, LobbyBand } from "./Band";
import type { Me } from "./types";
import { wire } from "./wire/contract";

const behindAccess: Me = { ...wire.me, mode: "cf-access" };

describe("the band", () => {
  it("names the product, then where you are, and only the last crumb is a heading", () => {
    render(<LobbyBand crumbs={["Lobby", "New table"]} corner={<a href="#/">back</a>} />);
    expect(screen.getByRole("heading", { level: 1, name: "Yokohama" })).toBeInTheDocument();
    // The page is the LAST crumb: it is this screen's heading, which is what
    // App.test and the browser walk read as "the lobby" at `crumbs={["Lobby"]}`.
    expect(screen.getByRole("heading", { level: 2, name: "New table" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Lobby" })).toBeNull();
    expect(screen.getByText("Lobby")).toHaveClass("crumb");
    // The separators are drawn, and read as nothing.
    const seps = document.querySelectorAll(".crumb-sep");
    expect(seps).toHaveLength(2);
    seps.forEach((s) => expect(s).toHaveAttribute("aria-hidden", "true"));
    // The corner is the caller's node, in the corner.
    expect(screen.getByRole("link", { name: "back" }).closest(".band-corner")).not.toBeNull();
    // **Not `header.banner`**: that is the GAME's header, and the browser
    // walk's `atTheGame` is scoped to it (`e2e/harness.ts:213-215`).
    expect(document.querySelector("header.lobby-band")).not.toBeNull();
    expect(document.querySelector("header.banner")).toBeNull();
  });
});

describe("the identity corner", () => {
  // **Under the open door the name IS the identity** (spec §2.1), so the one
  // action is to become somebody else: the stored name goes, and the host is
  // told, which re-asks `/api/me` and lands on the name step.
  it("offers switch player under the open door, and drops the name before telling the host", async () => {
    expect(wire.me.mode).toBe("open");
    localStorage.setItem(DISPLAY_NAME_KEY, "Red");
    // What storage held at the moment the host was told: the ORDER, which two
    // assertions after the click cannot see. No `expect` in the handler — it
    // would throw inside React's event dispatch.
    const seen: (string | null)[] = [];
    const onChange = vi.fn(() => seen.push(localStorage.getItem(DISPLAY_NAME_KEY)));
    render(<IdentityCorner me={wire.me} onChange={onChange} />);
    expect(screen.getByText("Playing as", { exact: false })).toBeInTheDocument();
    // The name shown is the display name; the identity key rides as its title.
    expect(screen.getByTitle(wire.me.user!)).toHaveTextContent("Red");
    expect(screen.queryByRole("button", { name: "change name" })).toBeNull();
    await userEvent.setup().click(screen.getByRole("button", { name: "switch player" }));
    expect(seen).toEqual([null]);
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it("shows the identity itself when there is no display name", () => {
    render(<IdentityCorner me={wire.me} onChange={vi.fn()} />);
    expect(screen.getByTitle(wire.me.user!)).toHaveTextContent(wire.me.user!);
  });

  // **Behind Access the identity is Access's** (spec §2.1): only the name others
  // see can change, the address stays beside it, and nothing offers to switch.
  it("offers change name behind Access, edits only the display name, and never switch player", async () => {
    const onChange = vi.fn();
    render(<IdentityCorner me={behindAccess} onChange={onChange} />);
    expect(screen.queryByRole("button", { name: "switch player" })).toBeNull();
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "change name" }));
    await user.type(screen.getByLabelText("Your name"), "  Captain Red ");
    await user.click(screen.getByRole("button", { name: "save" }));
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Captain Red");
    expect(screen.getByTitle(behindAccess.user!)).toHaveTextContent("Captain Red");
    // The address, beside the name, once they differ.
    expect(screen.getByText(behindAccess.user!)).toHaveClass("identity-address");
    // The identity did not change, so the host is not told (amendment 2).
    expect(onChange).not.toHaveBeenCalled();
  });

  it("clears the display name when the field is emptied, and cancels without writing", async () => {
    localStorage.setItem(DISPLAY_NAME_KEY, "Red");
    render(<IdentityCorner me={behindAccess} onChange={vi.fn()} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "change name" }));
    await user.clear(screen.getByLabelText("Your name"));
    await user.click(screen.getByRole("button", { name: "cancel" }));
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Red");
    await user.click(screen.getByRole("button", { name: "change name" }));
    await user.clear(screen.getByLabelText("Your name"));
    await user.click(screen.getByRole("button", { name: "save" }));
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
    expect(screen.getByTitle(behindAccess.user!)).toHaveTextContent(behindAccess.user!);
  });

  // **The phone's pill** (mock 11): the same corner, collapsed. The pill is a
  // disclosure over the corner's own menu — ONE set of controls, so a query for
  // "switch player" never finds two (jsdom draws both halves: it has no CSS).
  it("is a pill that opens its menu at phone width", async () => {
    const { container } = render(<IdentityCorner me={wire.me} onChange={vi.fn()} />);
    const pill = screen.getByRole("button", { expanded: false });
    expect(pill).toHaveTextContent(wire.me.user!);
    expect(container.querySelector(".identity-corner")).toHaveAttribute("data-open", "false");
    await userEvent.setup().click(pill);
    expect(pill).toHaveAttribute("aria-expanded", "true");
    expect(container.querySelector(".identity-corner")).toHaveAttribute("data-open", "true");
    expect(document.getElementById(pill.getAttribute("aria-controls")!)).toHaveClass("identity-menu");
    expect(screen.getAllByRole("button", { name: "switch player" })).toHaveLength(1);
  });

  it("draws nothing for a caller the server does not name", () => {
    const { container } = render(<IdentityCorner me={wire.meSignedOut} onChange={vi.fn()} />);
    expect(container).toBeEmptyDOMElement();
  });
});
