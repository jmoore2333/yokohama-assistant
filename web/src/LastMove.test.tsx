// The applied-move lines, in the side column rather than the deck.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { LastMove } from "./LastMove";
import { wire } from "./wire/contract";

const escape = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

describe("the last move", () => {
  it("holds every line the server sent, refusals as warnings", () => {
    const state = wire.refused;
    const refusals = state.messages.filter((m) => /illegal|refus|cannot|not legal|out of range/i.test(m));
    expect(refusals.length, "the fixture must carry a refusal").toBeGreaterThan(0);
    render(<LastMove messages={state.messages} />);
    for (const m of state.messages) expect(screen.getByText(new RegExp(escape(m)))).toBeInTheDocument();
    for (const m of refusals) expect(screen.getByText(new RegExp(escape(m)))).toHaveClass("warn");
  });

  // **The leading-bang branch, which the prose regex misses entirely.**
  // `repl.rs` refuses with eighteen literals that start with a bang and contain
  // none of `illegal|refus|cannot|not legal|out of range`, so before that branch
  // every one of them rendered as an ordinary applied-move line. The regex is
  // asserted to MISS the sample first, or this cell would pass through the other
  // half of the OR and prove nothing about the branch it is named for.
  it("warns on a bang-prefixed refusal the prose regex cannot see", () => {
    const bang = "! A05 is not one of the eligible areas";
    expect(
      /illegal|refus|cannot|not legal|out of range/i.test(bang),
      "the prose half of `isRefusal` must not match this sample",
    ).toBe(false);
    render(<LastMove messages={[bang, "Turn 1 Red -> Turn 2 Blue"]} />);
    expect(screen.getByText(new RegExp(escape(bang)))).toHaveClass("warn");
    expect(screen.getByText(/Turn 1 Red/)).not.toHaveClass("warn");
  });

  // A live region has to exist BEFORE its text arrives to be announced.
  it("keeps its live region mounted while empty", () => {
    const { container } = render(<LastMove messages={[]} />);
    const region = container.querySelector('[role="status"]');
    expect(region).not.toBeNull();
    expect(region!.textContent).toBe("");
  });

  it("folds and unfolds without unmounting the region", async () => {
    const user = userEvent.setup();
    const { container } = render(<LastMove messages={["Turn 1 Red -> Turn 2 Blue"]} />);
    await user.click(screen.getByRole("button", { name: "hide" }));
    expect(container.querySelector(".messages.folded")).not.toBeNull();
    expect(container.querySelector('[role="status"]')).not.toBeNull();
    await user.click(screen.getByRole("button", { name: "show" }));
    expect(container.querySelector(".messages.folded")).toBeNull();
    // Unfolded the class list is exactly `messages`, with no trailing space
    // left behind by the folded arm — the CSS does not care, but a className
    // read back by anything else does.
    expect(container.querySelector('[role="status"]')!.className).toBe("messages");
  });
});
