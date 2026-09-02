// The strips on the table: the physical boards' shapes, drawn from the wire.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GoalsStrip, LabStrip, PortStrip, SitesStrip, TracksStrip } from "./Strips";
import { SECTION_KEYS } from "./sections";
import { achievementsOf, tracksOf } from "./test/panels";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

describe("the port strip", () => {
  it("draws six slots per board under their zone and gate, from the slots themselves", () => {
    const { container } = render(<PortStrip panel={wire.ports} art={wire.artManifest} />);
    expect(container.querySelectorAll("section.table-sec.ports")).toHaveLength(wire.ports.boards.length);
    expect(container.querySelectorAll(".strip-slot")).toHaveLength(6 * wire.ports.boards.length);
    const text = container.textContent ?? "";
    for (const zone of ["Low", "Mid", "High"]) expect(text).toContain(zone);
    const gates = new Map<string, number>();
    for (const s of wire.ports.boards[0]!.slots) gates.set(s.zone, Math.min(gates.get(s.zone) ?? 99, s.min_power));
    for (const [, power] of gates) expect(text).toContain(`power ${power}+`);
    expect(text).toContain(`${wire.ports.deck_remaining} in the deck`);
  });
});

describe("the laboratory strip", () => {
  it("draws six slots with the surcharge on the four that carry one", () => {
    const { container } = render(<LabStrip panel={wire.labs} art={wire.artManifest} labels={wire.labels} />);
    expect(container.querySelectorAll(".strip-slot")).toHaveLength(6 * wire.labs.boards.length);
    const marks = [...container.querySelectorAll(".chip.warn")].map((c) => c.textContent);
    const want = wire.labs.boards[0]!.slots.filter((s) => s.surcharge > 0).map((s) => `+${s.surcharge}`);
    expect(marks.slice(0, want.length)).toEqual(want);
  });
});

describe("the tracks strip", () => {
  it("prints every space with its points as text", () => {
    const tracks = tracksOf(wire.optionsMany);
    const { container } = render(<TracksStrip tracks={tracks} art={wire.artManifest} colors={COLORS} />);
    const text = container.textContent ?? "";
    for (const s of tracks.church) expect(text).toContain(`★${s.vp}`);
    for (const s of tracks.customs) expect(text).toContain(`★${s.vp}`);
    expect(container.querySelector("section.table-sec.tracks")).not.toBeNull();
  });
});

describe("the goals strip", () => {
  it("draws the three cards in a row, landscape", () => {
    const cards = achievementsOf(wire.options);
    const { container } = render(<GoalsStrip cards={cards} art={wire.artManifest} labels={wire.labels} colors={COLORS} />);
    expect(container.querySelectorAll(".card-face.achievement")).toHaveLength(cards.length);
    expect(container.querySelector(".table-sec.achievements .card-strip")).not.toBeNull();
  });
});

describe("the building sites strip", () => {
  it("draws one card per area that has a site, from the live frame", () => {
    const withSites = wire.areas.rows.flat().filter((c) => c.tile.site !== null);
    expect(withSites.length).toBeGreaterThan(0);
    const { container } = render(<SitesStrip panel={wire.areas} art={wire.artManifest} colors={COLORS} />);
    expect(container.querySelectorAll(".site-card")).toHaveLength(withSites.length);
  });
  it("marks a built space in the builder's seat colour, from the LIVE model", () => {
    const built = structuredClone(wire.areas) as typeof wire.areas;
    const first = built.rows.flat().find((c) => c.tile.site !== null)!;
    first.tile.site!.slots[1]!.built_by = 1;
    first.tile.site!.trading_house_owner = 0;
    const { container } = render(<SitesStrip panel={built} art={wire.artManifest} colors={COLORS} />);
    const card = container.querySelector(".site-card")!;
    expect(card.querySelectorAll(".site-space.built")).toHaveLength(1);
    expect(card.querySelector(".site-space.built")!.className).toMatch(/seat-blue/);
    expect(card.querySelector(".site-trading-house.built")?.className).toMatch(/seat-red/);
  });
});

// **The Areas panel's `companion` pointer moved with the surfaces it names.**
//
// It used to point at four DRAWERS, and this cell used to live in
// `Surfaces.test.tsx` asserting the registry held a key for each. The drawers
// retired in W5a; the pointer now reads "detail lives in the … section of the
// table", so the list it has to match is the SECTION registry.
describe("the Areas panel's companions", () => {
  it("name sections that exist", () => {
    for (const companion of ["Ports", "Labs", "Tracks"]) {
      expect(SECTION_KEYS).toContain(companion.toLowerCase());
    }
  });
});
