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
  });
});
