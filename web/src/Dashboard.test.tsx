// The rail: who has what, and the three things it must not round off.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Dashboard } from "./Dashboard";
import { wire } from "./wire/contract";
import { handOf, playersOf, tracksOf } from "./test/panels";
import { DUMMY_SEAT, type ClientState, type PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];
const COLORS_4: PlayerColor[] = ["Red", "Blue", "Yellow", "Green"];

function show(state: ClientState, colors: PlayerColor[] = COLORS) {
  return render(
    <Dashboard
      panels={state.frame.dashboard}
      art={wire.artManifest}
      labels={wire.labels}
      colors={colors}
    />,
  );
}

describe("the 2P dummy pawn", () => {
  // `occupant === 255` is `PlayerId::DUMMY`: a real scoring occupant that
  // COMPETES in the church and customs majorities, excluded from the end-game
  // pawn count, and never a seat. Indexing a colour array with 255 is exactly
  // how it would silently become one, and `colors[255]` is `undefined`, so the
  // wrong version does not throw — it renders an unstyled pawn that reads as a
  // player.
  it("is marked D and is never given a seat colour", () => {
    const { container } = show(wire.options);
    const dummies = container.querySelectorAll(".space.dummy");
    const onWire = [...tracksOf(wire.options).church, ...tracksOf(wire.options).customs].filter(
      (s) => s.occupant === DUMMY_SEAT,
    );
    expect(onWire.length).toBeGreaterThan(0);
    expect(dummies).toHaveLength(onWire.length);
    for (const d of dummies) {
      expect(d.textContent).toContain("D");
      expect(d.className).not.toMatch(/seat-(red|blue|yellow|green)/);
    }
  });

  // The clause only appears when a dummy is actually on a track:
  // `place_setup_dummies` returns early above 2 players, so a browser that
  // teaches a `D` marker at 4P is teaching a rule that cannot apply there.
  it("is explained at 2P and not mentioned at 4P", () => {
    show(wire.options);
    expect(screen.getByText(/D = dummy, not counted/)).toBeInTheDocument();
    screen.getByText(/Tracks/); // the heading exists either way

    render(
      <Dashboard
        panels={wire.options4p.frame.dashboard}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS_4}
      />,
    );
    const headings = screen.getAllByRole("heading", { name: /Tracks/ });
    expect(headings).toHaveLength(2);
    expect(headings[1]!.textContent).not.toMatch(/dummy/);
  });
});

describe("a hand", () => {
  // The mask is a property of the MODEL — `cards` is empty whenever `hidden` —
  // so a client that ignores the flag still cannot leak identities. This
  // asserts the rendering does not put one back: the count is truthful and no
  // card id appears.
  it("masked, shows the count and no identity", () => {
    const hand = handOf(wire.hiddenHand);
    expect(hand?.hidden).toBe(true);
    const { container } = show(wire.hiddenHand);
    expect(screen.getByText(new RegExp(`${hand!.card_count} card\\(s\\) — hidden`)))
      .toBeInTheDocument();
    expect(container.querySelectorAll(".card-face.order")).toHaveLength(0);
  });

  it("unmasked, draws every card it carries", () => {
    const hand = handOf(wire.options);
    expect(hand?.hidden).toBe(false);
    expect(hand!.cards.length).toBeGreaterThan(0);
    const { container } = show(wire.options);
    expect(container.querySelectorAll(".card-face.order")).toHaveLength(hand!.cards.length);
    for (const card of hand!.cards) {
      expect(screen.getAllByText(`O${String(card.id).padStart(2, "0")}`).length).toBeGreaterThan(0);
    }
  });

  it("names the starting order, which is the one card a seat begins with", () => {
    const hand = handOf(wire.options);
    const starting = hand!.cards.filter((c) => c.is_starting_order);
    expect(starting.length).toBeGreaterThan(0);
    show(wire.options);
    expect(screen.getAllByText("your starting order")).toHaveLength(starting.length);
  });
});

describe("a seat's figures", () => {
  // `panel_render::goods_line` prints every good, zeros included: a browser
  // that drops `0 imports` cannot be read as "no imports" rather than "imports
  // not shown on this panel", and imports are what the Customs track and one of
  // the church's six faith payment types are counted in.
  it("show all five goods, zeros included", () => {
    const players = playersOf(wire.options);
    // At the opening deal at least one good is zero, which is the case the rule
    // exists for — so this test is not vacuous at this fixture.
    expect(players.some((p) => Object.values(p.goods).some((n) => n === 0))).toBe(true);

    const { container } = show(wire.options);
    // Scoped PER SEAT: two seats may hold the same figure, and a page-wide text
    // query then cannot tell one seat printing it twice from two seats printing
    // it once.
    const seats = container.querySelectorAll(".seat");
    expect(seats).toHaveLength(players.length);
    players.forEach((p, i) => {
      const seat = seats[i]!;
      for (const [good, n] of Object.entries(p.goods)) {
        expect(seat.textContent).toContain(`${n} ${good}`);
      }
    });
  });

  // All three zones, because IN HAND is the one you can build from. `shop 2/8`
  // cannot distinguish two in hand from none, and a player reading it plans a
  // build they cannot make.
  it("split shophouses and trading houses into all three zones", () => {
    const p = playersOf(wire.options)[0]!;
    show(wire.options);
    expect(
      screen.getAllByTitle("in hand / in warehouse / built").length,
    ).toBeGreaterThanOrEqual(2);
    expect(
      screen.getAllByText(
        new RegExp(
          `shop ${p.shophouses.in_hand}h / ${p.shophouses.in_warehouse}w / ${p.shophouses.built}b`,
        ),
      ).length,
    ).toBeGreaterThan(0);
  });

  // Unused mercenaries score 1 VP each and used ones score nothing, so the
  // split IS the only thing about them that scores.
  it("split mercenaries into used and unused", () => {
    const { container } = show(wire.options);
    const seats = container.querySelectorAll(".seat");
    playersOf(wire.options).forEach((p, i) => {
      // BOTH numbers, always. A single total cannot say which of them scores,
      // and only the unused ones do.
      expect(seats[i]!.textContent).toContain(
        `merc ${p.mercenaries_unused} unused / ${p.mercenaries_used} used`,
      );
    });
  });

  it("say where each president is, in both arms", () => {
    show(wire.options);
    for (const p of playersOf(wire.options)) {
      if (p.president === "InHand") {
        expect(screen.getAllByText("President in hand").length).toBeGreaterThan(0);
      } else {
        expect(
          screen.getAllByText(`President at A${String(p.president.OnBoard).padStart(2, "0")}`)
            .length,
        ).toBeGreaterThan(0);
      }
    }
  });
});

describe("four seats", () => {
  it("every colour on the wire is drawn as itself", () => {
    show(wire.options4p, COLORS_4);
    for (const colour of COLORS_4) {
      expect(screen.getAllByText(colour).length).toBeGreaterThan(0);
    }
  });
});

describe("what the side column shows", () => {
  it("draws only the panels it is asked for", () => {
    const { container } = render(
      <Dashboard panels={wire.options.frame.dashboard} art={wire.artManifest} labels={wire.labels} colors={COLORS} only={["Players", "Hand"]} />,
    );
    expect(screen.getByText("Players")).toBeInTheDocument();
    expect(screen.queryByText(/Tracks/)).toBeNull();
    expect(screen.queryByText(/Achievements/)).toBeNull();
    expect(container.querySelectorAll(".panel").length).toBe(2);
  });
  it("prints every track space's points as text, not only in a tooltip", () => {
    const { container } = show(wire.options);
    const tracks = tracksOf(wire.options);
    const subs = [...container.querySelectorAll(".space .sub")].map((s) => s.textContent);
    for (const s of tracks.church) expect(subs).toContain(`★${s.vp}`);
    for (const s of tracks.customs) expect(subs).toContain(`★${s.vp}`);
  });
});
