// The small marks the setup screens share (M6.5c2, spec §2.2): a seat, the
// password, the bot's version. Presentational, so the props are literal.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Padlock, PasswordTag, SeatChip, VersionTag } from "./Chips";

describe("the seat chip", () => {
  it("draws a dot in the seat's colour, named for it, beside the label", () => {
    const { container } = render(<SeatChip color="Blue" label="Alice" />);
    const chip = container.querySelector(".seat-chip")!;
    expect(chip).toHaveTextContent("Alice");
    const dot = chip.querySelector(".seat-dot")!;
    expect(dot).toHaveClass("seat-blue");
    // A colour a screen reader can read, and not text a row's words include.
    expect(screen.getByRole("img", { name: "Blue" })).toBe(dot);
    expect(chip.textContent).toBe("Alice");
  });

  it("is dashed and dotless when the seat is open, and gold when it is to move", () => {
    const { container } = render(
      <>
        <SeatChip color="Yellow" label="open seat" open />
        <SeatChip color="Red" label="you · your move" toMove />
      </>,
    );
    const [open, moving] = [...container.querySelectorAll(".seat-chip")];
    expect(open).toHaveClass("open");
    expect(open!.querySelector(".seat-dot")).toBeNull();
    expect(moving).toHaveClass("to-move");
    expect(moving).not.toHaveClass("open");
  });
});

describe("the password tag", () => {
  // A DRAWN padlock (spec §2.2), not an emoji: the lock glyph announces as
  // nothing useful and renders differently on every system. The tag is named
  // for what it means, which is also the name the lobby's cells already read.
  it("is a drawn padlock and the word, named for what it means", () => {
    const { container } = render(<PasswordTag />);
    const tag = screen.getByRole("img", { name: "password required" });
    expect(tag).toHaveTextContent("Password");
    expect(tag.querySelector("svg.padlock")).not.toBeNull();
    expect(container.textContent).not.toMatch(/\u{1F512}/u);
  });

  it("draws the padlock as decoration wherever it is used", () => {
    const { container } = render(<Padlock />);
    expect(container.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
  });
});

describe("the version tag", () => {
  it("prints the label it is given, and says what it is", () => {
    render(<VersionTag label="V4" />);
    expect(screen.getByText("V4")).toHaveAttribute("title", "the bot's version");
  });
});
