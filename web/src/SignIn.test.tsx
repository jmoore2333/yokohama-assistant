// The development door: one field, and what it leaves behind for `api.ts`.
//
// The KEY is the contract, not the component. `api.ts` reads
// `CLAIMED_USER_KEY` on every call and attaches the header from it (R-plan-2),
// so a screen that stored the name under a key of its own would sign you in on
// screen and send every request anonymously. The cell imports the key from
// `api.ts` rather than restating it, which is what makes that a join and not
// two strings that happen to match today.

import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SignIn } from "./SignIn";
import { CLAIMED_USER_KEY, DISPLAY_NAME_KEY } from "./api";

describe("the sign-in screen", () => {
  it("stores the typed email under the key api.ts reads and hands it up", async () => {
    const onSignedIn = vi.fn();
    render(<SignIn onSignedIn={onSignedIn} />);
    const user = userEvent.setup();
    // Typed as a person types it: padded, and in the case their keyboard was
    // in. The server normalizes the same way (spec R-1.1), so a client that
    // stored the raw text would attach a header the server answers under a
    // DIFFERENT name than the one on screen.
    await user.type(screen.getByLabelText("Email"), "  Red@Wire.TEST ");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBe("red@wire.test");
    expect(onSignedIn).toHaveBeenCalledWith("red@wire.test");
  });

  it("refuses a value with no @ and stores nothing", async () => {
    const onSignedIn = vi.fn();
    render(<SignIn onSignedIn={onSignedIn} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Email"), "nobody");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(screen.getByText(/is not an email/)).toBeInTheDocument();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
    // **And the screen does not hand it up either.** Storing nothing while
    // still calling back would put the player into the app under a name the
    // header cannot carry, and every request would then 401.
    expect(onSignedIn).not.toHaveBeenCalled();
  });

  // §7.1's last bullet: the player is returned here WITH the sentence. Without
  // it a game vanishes and a bare form appears, and nothing on screen connects
  // the two.
  it("shows the sentence that sent the player back, above the form", () => {
    render(<SignIn onSignedIn={vi.fn()} notice="no signed-in user: send X-Yokohama-Claimed-User" />);
    const said = screen.getByRole("status");
    expect(said).toHaveTextContent("no signed-in user: send X-Yokohama-Claimed-User");
    // Above the form, not after it: the reason you are here is read before the
    // thing it is asking you to do.
    expect(said.compareDocumentPosition(screen.getByRole("button", { name: "Sign in" }))).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  it("says nothing of its own when there is no notice", () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    expect(screen.queryByRole("status")).toBeNull();
  });

  // **The name at the door (R-6.4.3), and it is OPTIONAL.** One key, read by
  // the lobby, the New table form, the seat picker and the chat's own post — so
  // a person who types it here is named from their first message onwards
  // without visiting any of the four screens that also offer it.
  //
  // The second half is what makes it optional rather than merely defaulted: a
  // blank field leaves whatever was there ALONE. Clearing is the lobby's field,
  // which has a value to clear; a door that wrote an empty string here would
  // sign a returning player in under no name at all.
  it("remembers an optional name typed at the door, and leaves the key alone when it is blank", async () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Email"), "red@wire.test");
    // Padded, as a person types it: the name is trimmed the way the address is.
    await user.type(screen.getByLabelText("Your name (optional)"), "  Captain Red  ");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Captain Red");

    cleanup();
    render(<SignIn onSignedIn={vi.fn()} />);
    await user.type(screen.getByLabelText("Email"), "blue@wire.test");
    await user.click(screen.getByRole("button", { name: "Sign in" }));
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Captain Red");
  });

  // Not in the plan's two cells, and here because the plan's own message reads
  // " is not an email" when the field is empty — a sentence with a hole in it,
  // shown on the likeliest mistake there is (clicking the button first).
  it("asks for something rather than calling an empty field an email", async () => {
    render(<SignIn onSignedIn={vi.fn()} />);
    await userEvent.setup().click(screen.getByRole("button", { name: "Sign in" }));
    expect(screen.getByText("Type the email you want to play under.")).toBeInTheDocument();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });
});
