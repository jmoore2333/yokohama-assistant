// The name step: the open door's one question (M6.5c2, spec §2.1; mock 03).
//
// The KEY is the contract, not the component. `api.ts` reads
// `CLAIMED_USER_KEY` on every call and sends it as the header, so the screen
// must store what the SERVER will key — `nameRule.ts`'s key, pinned to
// `User::parse` — and keep what was typed for display on its own key. The
// cells import both keys from `api.ts` rather than restating them.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { CLAIMED_USER_KEY, DISPLAY_NAME_KEY } from "./api";
import { SignIn } from "./SignIn";

describe("the name step", () => {
  // **Review Focus 1, on the screen**: one key, shown as typed.
  it("stores the key the server will use, keeps the name as typed, and hands the key up (Review Focus 1)", async () => {
    // **What storage held WHEN the callback ran** (review M2), recorded rather
    // than asserted inside it: an `expect` that threw there would throw inside
    // React's event dispatch. The order is load-bearing — the callback re-reads
    // `/api/me`, and `api.ts` sends the header from storage — so a key stored
    // after it would ask the server as nobody.
    const seen: (string | null)[] = [];
    const onSignedIn = vi.fn(() => {
      seen.push(localStorage.getItem(CLAIMED_USER_KEY));
    });
    render(<SignIn onSignedIn={onSignedIn} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Name"), "  Ben   Castle ");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBe("ben castle");
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Ben Castle");
    expect(onSignedIn).toHaveBeenCalledWith("ben castle");
    expect(seen, "the key was stored before the callback ran").toEqual(["ben castle"]);
  });

  it("takes an address too, keyed exactly as the server keys it", async () => {
    const onSignedIn = vi.fn();
    render(<SignIn onSignedIn={onSignedIn} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Name"), " Red@Wire.TEST ");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBe("red@wire.test");
    expect(onSignedIn).toHaveBeenCalledWith("red@wire.test");
  });

  // Refused BEFORE a request: a name the server would refuse is never stored and
  // never handed up — every request would then be a 401.
  it("refuses what the server refuses, says why, and stores nothing", async () => {
    const onSignedIn = vi.fn();
    render(<SignIn onSignedIn={onSignedIn} />);
    const user = userEvent.setup();
    const field = screen.getByLabelText("Name");
    expect(field).not.toHaveAttribute("aria-invalid", "true");
    await user.type(field, "..");
    await user.click(screen.getByRole("button", { name: "Continue" }));
    // **Announced, and tied to the field** (review M7): a screen-reader user who
    // presses Continue hears the reason, and the field says it is the one
    // refused and carries the reason as its description beside the rule.
    expect(screen.getByRole("alert")).toHaveTextContent("A name begins with a letter or a digit.");
    expect(field).toHaveAttribute("aria-invalid", "true");
    expect(field).toHaveAccessibleDescription(/Up to 32 characters\. A name begins with a letter or a digit\./);
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
    expect(onSignedIn).not.toHaveBeenCalled();
  });

  it("asks for something rather than refusing an empty field", async () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    await userEvent.setup().click(screen.getByRole("button", { name: "Continue" }));
    expect(screen.getByText("Type a name to play under.")).toBeInTheDocument();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });

  // §7.1's last bullet: the player is returned here WITH the sentence. Without
  // it a game vanishes and a bare form appears, and nothing connects the two.
  it("shows the sentence that sent the player back, above the form", () => {
    render(<SignIn onSignedIn={vi.fn()} notice="no signed-in user: send X-Yokohama-Claimed-User" />);
    const said = screen.getByRole("status");
    expect(said).toHaveTextContent("no signed-in user: send X-Yokohama-Claimed-User");
    expect(said.compareDocumentPosition(screen.getByRole("button", { name: "Continue" }))).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  it("says nothing of its own when there is no notice", () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    expect(screen.queryByRole("status")).toBeNull();
  });

  // **The trust model, on the screen and not only in the docs** (R-6.5c2.3):
  // the open door checks nobody, and a player's tables stay with the name.
  it("says the server checks nobody, and that the tables stay with the name", () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    expect(screen.getByText(/doesn.t check who anyone is/)).toHaveTextContent(
      "switching names later leaves them behind",
    );
    expect(screen.getByText(/Up to 32 characters/)).toBeInTheDocument();
  });
});
