// The strips on the table: the physical boards' shapes, drawn from the wire.

import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GoalsStrip, LabStrip, PortStrip, SitesStrip, TracksStrip } from "./Strips";
import { riderWords } from "./Dashboard";
import { SECTION_KEYS, SECTION_LABELS, type SectionKey } from "./sections";
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

  // **Spec §3.4 asks for the VP AND THE RIDER on every space, and the rider was
  // the half that lived only in `title=`** — invisible on a touch screen, to a
  // screen reader, and to any assertion, while the CLI's own `tracks` view
  // prints it (`view_render.rs`'s `+N assistant moves` / `+N warehouse takes` /
  // `+N yen`). This is the cell that says it is on the space.
  //
  // The words are read from `riderWords`, not typed here: the strip's compact
  // form and the tooltip's long one come from that ONE function, and a test
  // that restated either would be a third copy free to drift from both.
  it("prints every space's RIDER on the space, not only in its tooltip", () => {
    const tracks = tracksOf(wire.optionsMany);
    const { container } = render(<TracksStrip tracks={tracks} art={wire.artManifest} colors={COLORS} />);
    const subs = [...container.querySelectorAll(".space .sub")].map((s) => s.textContent ?? "");

    // Anti-vacuity, in three parts: this fixture must carry a movement rider,
    // a warehouse rider AND a yen rider, or the loop below tests one kind and
    // reads as if it tested all three.
    expect(tracks.church.some((s) => s.movement_rider > 0)).toBe(true);
    expect(tracks.customs.some((s) => s.warehouse_rider > 0)).toBe(true);
    expect(tracks.customs.some((s) => s.yen_rider > 0)).toBe(true);

    const carrying = [...tracks.church, ...tracks.customs].filter((s) => riderWords(s).short !== "");
    for (const s of carrying) {
      const want = riderWords(s).short;
      expect(
        subs.some((t) => t.includes(`★${s.vp}`) && t.includes(want)),
        `no space prints ★${s.vp} beside its rider "${want}"`,
      ).toBe(true);
    }

    // And a space that carries NO rider prints none — without this the loop
    // above passes on a strip that stamped the same rider onto every space.
    const bare = tracks.customs.filter((s) => riderWords(s).short === "");
    expect(bare.length, "the fixture must also carry a rider-free space").toBeGreaterThan(0);
    for (const s of bare) expect(subs).toContain(`★${s.vp}`);
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
  // **The key has to exist AND the label has to be the one on the section's own
  // heading.** `companion` is the server's word — "Labs" — and the section it
  // points at is headed "Laboratory", so a pointer that printed the raw word
  // sent the reader looking for a section by a name nothing on screen uses.
  it("name sections that exist, and are shown by those sections' own labels", () => {
    for (const [companion, heading] of [
      ["Ports", "Port"],
      ["Labs", "Laboratory"],
      ["Tracks", "Tracks"],
    ] as const) {
      const key = companion.toLowerCase() as SectionKey;
      expect(SECTION_KEYS).toContain(key);
      expect(SECTION_LABELS[key]).toBe(heading);
    }
  });
});
