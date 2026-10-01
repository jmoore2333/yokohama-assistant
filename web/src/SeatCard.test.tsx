// One seat at a table that is filling (M6.5c2, spec §2.2; mocks 08 and 09):
// its colour as a band, a caption, who is there, a note, and an action.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SeatCard } from "./SeatCard";

describe("the seat card", () => {
  it("is a region named by its caption, coloured by its seat", () => {
    render(<SeatCard color="Red" caption="Red · first player" who="Ben" note="seated" />);
    const card = screen.getByRole("region", { name: "Red · first player" });
    expect(card).toHaveClass("seat-card", "seat-red");
    expect(card).not.toHaveClass("open");
    expect(card.querySelector(".seat-card-band")).toHaveAttribute("aria-hidden", "true");
    expect(card).toHaveTextContent("Ben");
    expect(card).toHaveTextContent("seated");
  });

  it("draws an open seat dashed, with the action it is given", () => {
    render(
      <SeatCard
        color="Yellow"
        caption="Yellow"
        who="Open"
        open
        action={<button type="button">Sit here</button>}
      />,
    );
    const card = screen.getByRole("region", { name: "Yellow" });
    expect(card).toHaveClass("open", "seat-yellow");
    expect(screen.getByRole("button", { name: "Sit here" }).closest(".seat-card")).toBe(card);
    expect(card.querySelector(".seat-card-note")).toBeNull();
  });
});
