// The rail: who has what, and the three things it must not round off.

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Dashboard, riderWords } from "./Dashboard";
import { wire } from "./wire/contract";
import { handOf, playersOf, tracksOf } from "./test/panels";
import {
  DUMMY_SEAT,
  type ClientState,
  type HandPanel,
  type Panel,
  type PlayerColor,
  type PlayerModel,
} from "./types";

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

describe("the walk's marks on a seat", () => {
  /** The Players panel alone, with a set of `<seat>:<techId>` keys lit.
   *
   *  `wire.atK` is the fixture because it is the one whose seat 1 OWNS a
   *  technology (`Ball`, id 12) — `wire.options` deals none, and a cell that
   *  marked a chip no fixture renders would pass while showing nothing. */
  const showAtK = (changedTechs?: Set<string>) =>
    render(
      <Dashboard
        panels={wire.atK.frame.dashboard}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        only={["Players"]}
        changedTechs={changedTechs}
      />,
    );

  const techChips = (c: HTMLElement) => [...c.querySelectorAll(".chip.tech")];

  /** `playersOf` in `test/panels` takes a `ClientState`; `atK` is the narrower
   *  `HistoricalFrame`, which carries the same `frame` and none of the rest. */
  const seatsAtK = (): PlayerModel[] => {
    const panel = wire.atK.frame.dashboard.find(
      (p): p is Extract<Panel, { Players: unknown }> => "Players" in p,
    );
    if (!panel) throw new Error("the atK fixture carries no Players panel");
    return panel.Players.players;
  };

  it("marks the tech chip whose key the walk lit, and only that one", () => {
    const owner = seatsAtK().find((p) => p.technologies.length > 0);
    if (!owner) throw new Error("the fixture carries no seat holding a technology");
    const id = owner.technologies[0]!.id;

    const { container } = showAtK(new Set([`${owner.seat}:${id}`]));
    const chips = techChips(container);
    expect(chips.length, "the panel must draw the tech it is marking").toBeGreaterThan(0);
    expect(chips.filter((c) => c.classList.contains("changed"))).toHaveLength(1);
    expect(container.querySelector(".chip.tech.changed")?.textContent).toBe(
      owner.technologies[0]!.printed?.name ?? `#${id}`,
    );
  });

  // The key is SEAT AND ID, not the id alone: two seats may own the same
  // technology TITLE — ten of the twenty circulate as two physical cards, which
  // is what makes that legal — so a mark keyed on anything a title shares would
  // land on both rows. The two copies do carry distinct ids, so this cell asks
  // the narrower question the key is actually for: does a key naming ANOTHER
  // seat reach this seat's chip?
  it("leaves the chip unmarked when the set names another seat's copy of it", () => {
    const owner = seatsAtK().find((p) => p.technologies.length > 0)!;
    const other = seatsAtK().find((p) => p.seat !== owner.seat)!;
    const { container } = showAtK(new Set([`${other.seat}:${owner.technologies[0]!.id}`]));
    expect(container.querySelector(".chip.tech.changed")).toBeNull();
  });

  it("marks nothing when the walk is not running", () => {
    const { container } = showAtK(undefined);
    expect(techChips(container).length).toBeGreaterThan(0);
    expect(container.querySelector(".chip.tech.changed")).toBeNull();
  });

  /** The Players panel with a set of SEAT numbers lit — the row's own mark,
   *  which is a different prop and a different grain from the chip's above. */
  const showSeats = (changedSeats?: Set<number>) =>
    render(
      <Dashboard
        panels={wire.atK.frame.dashboard}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        only={["Players"]}
        changedSeats={changedSeats}
      />,
    );

  // **This is the mark M20 could delete with the whole suite still green**
  // (final review m3): no `*.test.tsx` named `.seat.changed`, and the e2e gate
  // RECORDS the count in an annotation rather than gating it. A seat row is
  // where money, goods and pieces move, so it is the grain that says "this
  // player's turn did something" when nothing on the board moved at all.
  it("marks the seat row the diff names, and no other", () => {
    const seats = seatsAtK();
    expect(seats.length, "the fixture must seat more than one player").toBeGreaterThan(1);
    const lit = seats[1]!;
    const { container } = showSeats(new Set([lit.seat]));
    const rows = [...container.querySelectorAll(".seat")];
    expect(rows).toHaveLength(seats.length);
    expect(container.querySelectorAll(".seat.changed")).toHaveLength(1);
    // Named by the seat it belongs to, not by its position: a mark keyed on the
    // row's index would pass a `toHaveLength(1)` while lighting the wrong player.
    expect(container.querySelector(".seat.changed .seat-name")?.textContent).toBe(lit.color);
  });

  it("marks no seat row when the walk is not running", () => {
    const { container } = showSeats(undefined);
    expect(container.querySelectorAll(".seat").length).toBeGreaterThan(0);
    expect(container.querySelector(".seat.changed")).toBeNull();
  });
});

describe("the walk's marks on the viewer's hand", () => {
  /** The Hand panel alone. `wire.atK`'s is seat 0's, FACE UP, holding two
   *  orders — the fixture the ruling needs, since a masked hand carries no
   *  identities to mark. */
  const showHand = (changedHand?: Set<number>) =>
    render(
      <Dashboard
        panels={wire.atK.frame.dashboard}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        only={["Hand"]}
        changedHand={changedHand}
      />,
    );

  const handAtK = (): HandPanel => {
    const panel = wire.atK.frame.dashboard.find((p): p is Extract<Panel, { Hand: HandPanel }> => "Hand" in p);
    if (!panel) throw new Error("the atK fixture carries no Hand panel");
    return panel.Hand;
  };

  it("marks the hand card whose id the walk lit, and only that one", () => {
    const hand = handAtK();
    expect(hand.hidden).toBe(false);
    expect(hand.cards.length, "the fixture must deal the viewer more than one card").toBeGreaterThan(1);
    const arrived = hand.cards[1]!.id;

    const { container } = showHand(new Set([arrived]));
    const marked = [...container.querySelectorAll(".card-face.order.changed")];
    expect(container.querySelectorAll(".card-face.order")).toHaveLength(hand.cards.length);
    expect(marked).toHaveLength(1);
    // The class is on the CARD'S ROOT, and the id printed on it says which.
    expect(marked[0]!.querySelector(".card-id")?.textContent).toBe(`O${String(arrived).padStart(2, "0")}`);
  });

  it("marks no hand card when the walk is not running", () => {
    const { container } = showHand(undefined);
    expect(container.querySelectorAll(".card-face.order").length).toBeGreaterThan(0);
    expect(container.querySelector(".card-face.order.changed")).toBeNull();
  });
});

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
  // **`only` must not defeat the unknown-panel fallback.** `wanted` keeps a
  // panel by NAME, so a fifth `screen_model::Panel` variant — or a rename of one
  // of the four — would be filtered out of the side column entirely, which is
  // the exact vanishing the raw-JSON fallback exists to prevent. `only` is set
  // on the app's ONE live Dashboard render, so the guard would be defeated
  // precisely where it matters.
  //
  // The unknown panel is hand-built, which this suite otherwise never does: a
  // shape the wire cannot currently produce is the whole point of the cell, and
  // the committed corpus by construction contains no such shape.
  it("keeps a panel it does not know, even while drawing only the named ones", () => {
    const unknown = { Weather: { outlook: "fine" } } as unknown as Panel;
    const { container } = render(
      <Dashboard
        panels={[...wire.options.frame.dashboard, unknown]}
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        only={["Players", "Hand"]}
      />,
    );
    expect(screen.getByText("Unknown panel")).toBeInTheDocument();
    const raw = container.querySelector(".raw-panel");
    expect(raw, "the unknown panel was filtered out by `only`").not.toBeNull();
    expect(raw!.textContent).toContain("Weather");
    // Still only the panels asked for, plus the one that must never vanish.
    expect(container.querySelectorAll(".panel").length).toBe(3);
  });
  it("prints every track space's points as text, not only in a tooltip", () => {
    const { container } = show(wire.options);
    const tracks = tracksOf(wire.options);
    const subs = [...container.querySelectorAll(".space .sub")].map((s) => s.textContent ?? "");
    // `includes`, not equality: since the riders joined the line a space that
    // carries one reads `★3 · +1mv`, and the points are still on it.
    for (const s of tracks.church) expect(subs.some((t) => t.includes(`★${s.vp}`))).toBe(true);
    for (const s of tracks.customs) expect(subs.some((t) => t.includes(`★${s.vp}`))).toBe(true);
  });

  // The mirror of `Strips.test.tsx`'s rider cell. `Tracks` is SHARED — the
  // table's strip wraps this component — so the property belongs to the
  // component, and the side column no longer drawing Tracks is not a reason for
  // the component's own suite to stop checking it.
  it("prints every track space's rider on the space, not only in a tooltip", () => {
    const { container } = show(wire.options);
    const tracks = tracksOf(wire.options);
    const subs = [...container.querySelectorAll(".space .sub")].map((s) => s.textContent ?? "");

    expect(tracks.church.some((s) => s.movement_rider > 0)).toBe(true);
    expect(tracks.customs.some((s) => s.warehouse_rider > 0)).toBe(true);
    expect(tracks.customs.some((s) => s.yen_rider > 0)).toBe(true);

    for (const s of [...tracks.church, ...tracks.customs]) {
      const want = riderWords(s).short;
      if (want === "") continue;
      expect(
        subs.some((t) => t.includes(`★${s.vp}`) && t.includes(want)),
        `no space prints ★${s.vp} beside its rider "${want}"`,
      ).toBe(true);
    }
  });

  // **Anti-vacuity on the helper, and it was MEASURED to be needed.** Every
  // assertion above reads `riderWords` on both sides, so a compact form that
  // dropped the COUNT — a bare `+mv` on every church space — moves the
  // expectation with it and passes. Mutation-tested: that change was green
  // against the two cells above and red only against this one.
  //
  // The counts are read off the fixture rather than written down: this church
  // carries riders of 1, 2 and 3, and this customs carries warehouse riders of
  // 2 and 1, so distinct sizes must produce distinct words or the space is not
  // saying what the terminal says.
  it("keeps the rider's COUNT in the compact form", () => {
    const tracks = tracksOf(wire.options);

    const mvCounts = new Set(tracks.church.map((s) => s.movement_rider).filter((n) => n > 0));
    expect(mvCounts.size, "the fixture must carry more than one movement-rider size").toBeGreaterThan(1);
    const mvWords = new Set(
      tracks.church.filter((s) => s.movement_rider > 0).map((s) => riderWords(s).short),
    );
    expect(mvWords.size, "different movement riders collapsed to the same words").toBe(mvCounts.size);

    // The warehouse rider, on the spaces that carry it ALONE — a space carrying
    // a yen rider too would differ for that reason instead.
    const wh = tracks.customs.filter((s) => s.warehouse_rider > 0 && s.yen_rider === 0);
    const whCounts = new Set(wh.map((s) => s.warehouse_rider));
    expect(whCounts.size, "the fixture must carry more than one warehouse-rider size").toBeGreaterThan(1);
    expect(
      new Set(wh.map((s) => riderWords(s).short)).size,
      "different warehouse riders collapsed to the same words",
    ).toBe(whCounts.size);
  });

  // The tooltip keeps the LONGER words, and they are the terminal's own
  // (`view_render.rs`: "+N assistant moves", "+N warehouse takes", "+N yen").
  // Both lengths come from `riderWords`, so this cell is what says the two
  // cannot drift apart.
  it("keeps the terminal's longer rider words in the tooltip", () => {
    const { container } = show(wire.options);
    const tracks = tracksOf(wire.options);
    const titles = [...container.querySelectorAll(".space")].map((s) => s.getAttribute("title") ?? "");
    for (const s of [...tracks.church, ...tracks.customs]) {
      const long = riderWords(s).long;
      if (long === "") continue;
      expect(titles.some((t) => t.includes(long)), `no tooltip carries "${long}"`).toBe(true);
    }
    expect(titles.some((t) => t.includes("assistant moves"))).toBe(true);
    expect(titles.some((t) => t.includes("warehouse take"))).toBe(true);
    expect(titles.some((t) => t.includes("yen"))).toBe(true);
  });
});
