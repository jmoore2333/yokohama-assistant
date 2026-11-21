// A good's token: ONE accessible node however many copies it draws.
//
// The art manifest is the corpus's (`wire/art_manifest.json`); an address is a
// string here and no picture is ever opened.

import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Tokens } from "./Token";
import { wire } from "./wire/contract";
import type { ArtManifest } from "./types";

afterEach(cleanup);

describe("a token group", () => {
  it("draws each copy as a picture with no name of its own, inside one named node", () => {
    expect(wire.artManifest.tokens, "the corpus manifest must hold the yen key").toHaveProperty(
      "yen_1",
    );
    const { container } = render(<Tokens good="Yen" amount={2} art={wire.artManifest} />);

    const group = screen.getByRole("img", { name: "2 yen" });
    const pictures = group.querySelectorAll("img");
    expect(pictures).toHaveLength(2);
    for (const p of pictures) {
      expect(p.getAttribute("alt")).toBe("");
      expect(p.getAttribute("src")).toContain(wire.artManifest.tokens["yen_1"]!.file);
    }
    // ONE node: nothing inside it is a second reading.
    expect(screen.getAllByRole("img")).toHaveLength(1);
    expect(container.querySelectorAll("[role=img]")).toHaveLength(1);
  });

  it("draws plain marks, and no picture, when the manifest is absent or lacks the key", () => {
    const lacking: ArtManifest = {
      ...wire.artManifest,
      tokens: Object.fromEntries(
        Object.entries(wire.artManifest.tokens).filter(([k]) => k !== "yen_1"),
      ),
    };
    for (const art of [null, lacking]) {
      const { container, unmount } = render(<Tokens good="Yen" amount={2} art={art} />);
      const group = screen.getByRole("img", { name: "2 yen" });
      expect(container.querySelectorAll("img")).toHaveLength(0);
      const marks = [...group.querySelectorAll(".tok")].map((m) => m.textContent);
      expect(marks).toEqual(["¥", "¥"]);
      unmount();
    }
  });

  it("names an imported good by its own noun, one of them", () => {
    render(<Tokens good="Imports" amount={1} art={null} />);
    expect(screen.getByRole("img", { name: "1 imported good" })).toBeInTheDocument();
  });
});
