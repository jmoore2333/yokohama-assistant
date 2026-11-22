// The strips on the table: the physical boards' shapes, drawn from the wire.

import { render, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GoalsStrip, LabStrip, PortStrip, SitesStrip, TracksStrip } from "./Strips";
import { riderWords } from "./Dashboard";
import { SECTION_KEYS, SECTION_LABELS, type SectionKey } from "./sections";
import { achievementsOf, tracksOf } from "./test/panels";
import { wire } from "./wire/contract";
import type { AchievementModel, AchievementType, ArtManifest, Panel, PlayerColor } from "./types";

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

// ---- M5.5: what the walk lights on a strip ------------------------------
//
// **The mark M19 could delete with the whole vitest suite still green** (final
// review m3): `litSlot` returning "" for every slot failed nothing, because no
// component cell named `.strip-slot.changed` and the browser gate RECORDS the
// count in an annotation rather than gating it.
//
// What is really being pinned is a JOIN, and it is the one place in this client
// where two files have to agree about a string: `diffFrames` writes the key as
// `<kind>:<board INDEX>:<slot>` — the index into `panel.boards`, never the
// `board_b` flag — and `Strips` reads it back by mapping the same list the same
// way. At 2P there is one board and the two readings coincide, so a cell that
// only counted marks would not see them come apart; naming the LIT SLOT is what
// makes the position part of the key load-bearing here.
describe("the walk's marks on a strip", () => {
  it("marks the port slot the diff names, and no other", () => {
    const board = wire.ports.boards[0]!;
    expect(board.slots.length, "the fixture must draw a strip to mark").toBe(6);
    const lit = board.slots[2]!;
    const { container } = render(
      <PortStrip
        panel={wire.ports}
        art={wire.artManifest}
        changed={new Set([`port:0:${lit.slot}`])}
      />,
    );
    const slots = [...container.querySelectorAll(".strip-slot")];
    expect(slots).toHaveLength(6 * wire.ports.boards.length);
    expect(container.querySelectorAll(".strip-slot.changed")).toHaveLength(1);
    expect(slots[2]!.className, "the mark landed on a slot the key does not name").toContain("changed");
  });

  it("marks the laboratory slot the diff names, and no other", () => {
    const board = wire.labs.boards[0]!;
    const lit = board.slots[4]!;
    const { container } = render(
      <LabStrip
        panel={wire.labs}
        art={wire.artManifest}
        labels={wire.labels}
        changed={new Set([`lab:0:${lit.slot}`])}
      />,
    );
    const slots = [...container.querySelectorAll(".strip-slot")];
    expect(slots).toHaveLength(6 * wire.labs.boards.length);
    expect(container.querySelectorAll(".strip-slot.changed")).toHaveLength(1);
    expect(slots[4]!.className).toContain("changed");
  });

  // The two strips are separate numberings that OVERLAP: a port slot and a lab
  // slot both have a slot 2. The kind is what keeps a port take from lighting
  // the laboratory, and this is the cell that says so.
  it("does not let a port key light a laboratory slot", () => {
    const lit = wire.labs.boards[0]!.slots[2]!;
    const { container } = render(
      <LabStrip
        panel={wire.labs}
        art={wire.artManifest}
        labels={wire.labels}
        changed={new Set([`port:0:${lit.slot}`])}
      />,
    );
    expect(container.querySelector(".strip-slot.changed")).toBeNull();
  });

  it("marks nothing when the walk is not running", () => {
    const { container } = render(<PortStrip panel={wire.ports} art={wire.artManifest} />);
    expect(container.querySelectorAll(".strip-slot").length).toBeGreaterThan(0);
    expect(container.querySelector(".strip-slot.changed")).toBeNull();
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

// ---- #75: the claim drawn on the card ------------------------------------
//
// `goals_claimed_4p.json` is a 4P position whose first card was claimed by
// seats 2, 0, 3, 1 in that order: neither seat order nor "the lowest seat
// first" can pass for claim order on it, and the first claim's VP (8) differs
// from a later one's (6), so a first clause priced at `subsequent_vp` reads
// wrong. The premise is asserted, not assumed: every cell below is only as
// strong as these numbers.
const G = wire.goalsClaimed4p;
const G_COLORS = G.players.players.map((p) => p.color);
const [FOUR, ONE, NONE] = G.achievements.cards as [AchievementModel, AchievementModel, AchievementModel];
const FOUR_WORDS = ["Yellow claimed first — 8 VP", "Red claimed — 6 VP", "Green claimed — 6 VP", "Blue claimed — 6 VP"];

function premise() {
  expect(G_COLORS, "the fixture's seats").toEqual(["Red", "Blue", "Yellow", "Green"]);
  expect(G.achievements.cards).toHaveLength(3);
  expect(FOUR.claimed_by).toEqual([2, 0, 3, 1]);
  expect([FOUR.first_place_vp, FOUR.subsequent_vp]).toEqual([8, 6]);
  expect(ONE.claimed_by).toEqual([3]);
  expect(ONE.first_place_vp).toBe(9);
  expect(NONE.claimed_by).toEqual([]);
}

const strip = (art: ArtManifest | null, colors: PlayerColor[] = G_COLORS) =>
  render(<GoalsStrip cards={G.achievements.cards} art={art} labels={wire.labels} colors={colors} />).container;
const faces = (root: Element) => [...root.querySelectorAll("figure.card-face.achievement")];
const seatOf = (el: Element) => [...el.classList].filter((k) => k.startsWith("seat-"));
/** What each node of the band is, in its order: the cubes, the two numbers and
 *  the divider between the first claim and the later ones. */
const kind = (el: Element) =>
  el.classList.contains("claim")
    ? el.classList.contains("first") ? "cube first" : "cube"
    : el.classList.contains("claims-vp")
      ? el.classList.contains("first") ? "vp first" : el.classList.contains("later") ? "vp later" : "vp ?"
      : el.classList.contains("claims-sep") ? "divider" : `? ${el.className}`;

/** The picture box and the band on it. The box is asserted to hold the
 *  picture the art calls for, so a cell "without art" really draws the blank. */
function pictureOf(card: Element, art: ArtManifest | null) {
  const pic = card.querySelector(":scope > .card-pic");
  expect(pic, "the card draws its picture box").not.toBeNull();
  if (art) {
    expect(pic!.querySelector(":scope > img"), "the picture is the art").not.toBeNull();
    expect(pic!.querySelector(".card-blank")).toBeNull();
  } else {
    expect(pic!.querySelector(":scope > .card-blank"), "the picture is the blank").not.toBeNull();
    expect(pic!.querySelector("img")).toBeNull();
  }
  return { pic: pic!, band: pic!.querySelector(":scope > .claims") };
}

describe.each([
  ["with art", wire.artManifest],
  ["without art", null],
] as const)("the claim band on the Goals strip, %s", (_, art) => {
  it("four claims: a cube per claimant in claim order, the first in the ringed place, the words in that order", () => {
    premise();
    const card = faces(strip(art))[0]!;
    const { band } = pictureOf(card, art);
    expect(band, "a claimed card draws its band on the picture box").not.toBeNull();
    expect(band!.getAttribute("role")).toBe("img");
    expect(band!.getAttribute("aria-label")).toBe(FOUR_WORDS.join(" · "));
    expect(within(card as HTMLElement).getByRole("img", { name: FOUR_WORDS.join(" · ") })).toBe(band);
    const cubes = [...band!.querySelectorAll(":scope > .claim")];
    expect(cubes.map(seatOf), "claim order: seats 2, 0, 3, 1").toEqual([["seat-yellow"], ["seat-red"], ["seat-green"], ["seat-blue"]]);
    expect(cubes.map((c) => c.classList.contains("first")), "only the first claim is ringed").toEqual([true, false, false, false]);
    expect(cubes.map((c) => c.getAttribute("title"))).toEqual(FOUR_WORDS);
    for (const c of cubes) expect(c.querySelector("svg.piece.cube"), "the board's cube").not.toBeNull();
    // The label is the one reading a screen reader gets: every node inside the
    // band is hidden from it, the cubes included.
    for (const c of band!.children) expect(c.getAttribute("aria-hidden"), `${kind(c)} is hidden`).toBe("true");
    expect([...band!.children].map(kind)).toEqual(["cube first", "vp first", "divider", "cube", "cube", "cube", "vp later"]);
    expect(band!.querySelector(".claims-vp.first")!.textContent).toBe("8");
    expect(band!.querySelector(".claims-vp.later")!.textContent).toBe("6");
    // The caption keeps its VP chip.
    expect(card.querySelector("figcaption .chip.vp")!.textContent).toBe("8 / 6 VP");
    expect(card.textContent).not.toContain("unclaimed");
  });

  it("one claim: the ringed place holds seat 3's colour, and nothing follows it", () => {
    premise();
    const card = faces(strip(art))[1]!;
    const { band } = pictureOf(card, art);
    expect(band).not.toBeNull();
    expect(band!.getAttribute("aria-label")).toBe("Green claimed first — 9 VP");
    const cubes = [...band!.querySelectorAll(":scope > .claim")];
    expect(cubes.map(seatOf)).toEqual([["seat-green"]]);
    expect(cubes[0]!.classList.contains("first")).toBe(true);
    expect(cubes[0]!.getAttribute("title")).toBe("Green claimed first — 9 VP");
    expect([...band!.children].map(kind), "no divider and no later VP").toEqual(["cube first", "vp first"]);
    expect(band!.querySelector(".claims-vp.later")).toBeNull();
    expect(band!.querySelector(".claims-vp.first")!.textContent).toBe("9");
  });

  it("no claim: no band, and the caption says unclaimed", () => {
    premise();
    const card = faces(strip(art))[2]!;
    const { pic, band } = pictureOf(card, art);
    expect(band).toBeNull();
    expect(pic.querySelector(".claim, .claims-vp")).toBeNull();
    expect([...card.querySelectorAll("figcaption .chip")].map((c) => c.textContent)).toContain("unclaimed");
  });
});

describe("the claim band's words", () => {
  it("name a claimant the colours do not as seat N, and still draw its cube", () => {
    premise();
    const card = faces(strip(wire.artManifest, ["Red", "Blue"]))[0]!;
    const { band } = pictureOf(card, wire.artManifest);
    const words = ["seat 2 claimed first — 8 VP", "Red claimed — 6 VP", "seat 3 claimed — 6 VP", "Blue claimed — 6 VP"];
    expect(band!.getAttribute("aria-label")).toBe(words.join(" · "));
    const cubes = [...band!.querySelectorAll(":scope > .claim")];
    expect(cubes.map((c) => c.getAttribute("title"))).toEqual(words);
    expect(cubes.map(seatOf)).toEqual([[], ["seat-red"], [], ["seat-blue"]]);
    for (const c of cubes) expect(c.querySelector("svg.piece.cube")).not.toBeNull();
    expect(cubes[0]!.classList.contains("first")).toBe(true);
  });

  it("are the band's alone: no caption carries a first or an also chip", () => {
    premise();
    const root = strip(wire.artManifest);
    const chips = [...root.querySelectorAll(".chip")];
    expect(chips.length, "the captions draw their chips").toBeGreaterThan(0);
    expect(root.querySelectorAll(".chip.claimed")).toHaveLength(0);
    for (const c of chips) expect(c.textContent).not.toMatch(/^(first|also) /);
    expect(root.querySelectorAll(".claims"), "two of the three cards are claimed").toHaveLength(2);
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

// ---- #75: the walk lights the card whose claims grew ---------------------
/** `wire.atK`'s Achievements panel: three cards, `Seafood7` claimed by seat 1. */
const atKGoals = (): AchievementModel[] => {
  const panel = wire.atK.frame.dashboard.find((p): p is Extract<Panel, { Achievements: unknown }> => "Achievements" in p);
  if (!panel) throw new Error("the fixture carries no Achievements panel");
  return panel.Achievements.cards;
};

describe("the walk's marks on the goals strip", () => {
  const cards = atKGoals();
  const lit = (changed?: Set<AchievementType>) => {
    const { container } = render(
      <GoalsStrip cards={cards} art={wire.artManifest} labels={wire.labels} colors={COLORS} changed={changed} />,
    );
    return [...container.querySelectorAll("figure.card-face.achievement")].map((f) => f.classList.contains("changed"));
  };

  it("lights the card the set names, and no other", () => {
    expect(cards.map((c) => c.achievement_type)).toEqual(["Seafood7", "Technologies4", "Commercial4"]);
    expect(lit(new Set<AchievementType>(["Technologies4"]))).toEqual([false, true, false]);
  });

  it("lights nothing with no set, and nothing with an empty one", () => {
    expect(lit()).toEqual([false, false, false]);
    expect(lit(new Set())).toEqual([false, false, false]);
  });
});
