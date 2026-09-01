// The read-only surfaces: the four `GET`s, and the two ways they went wrong.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Surface, SURFACES } from "./Surfaces";
import { gate, stubApi, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function show(name: (typeof SURFACES)[number]["key"], onClose = vi.fn()) {
  const view = render(
    <Surface
      name={name}
      game="wire"
      art={wire.artManifest}
      labels={wire.labels}
      colors={COLORS}
      onClose={onClose}
    />,
  );
  return { view, onClose, user: userEvent.setup() };
}

describe("the payload and the branch it is rendered through", () => {
  // **The defect, exactly.** React renders BEFORE it runs effects, so switching
  // from Labs to Areas rendered the Areas branch against the Labs payload for
  // one frame, and `panel.rows.flat()` threw on a `rows` that a `LabsPanel`
  // does not have. Clearing the state inside the effect is one render too late;
  // the state has to carry its own identity, which is what `loaded.of` is.
  it("a payload is never rendered through another surface's branch", async () => {
    const held = gate<Reply>();
    stubApi((call) => {
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return held.promise;
      throw new Error(`unexpected ${call.path}`);
    });

    const { view } = show("labs");
    await screen.findByText(/Acquire ONE card/);

    // Switch. The Areas request is in flight, so the only state in hand is the
    // Labs payload — which the Areas branch must not be handed.
    view.rerender(
      <Surface
        name="areas"
        game="wire"
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        onClose={() => {}}
      />,
    );

    expect(screen.getByText("loading…")).toBeInTheDocument();
    expect(screen.queryByText(/Acquire ONE card/)).toBeNull();

    held.resolve(wire.areas);
    await waitFor(() => expect(screen.queryByText("loading…")).toBeNull());
    // And the right one does render, so the guard is not simply refusing.
    expect(screen.getAllByText(/never an action area|A0\d/).length).toBeGreaterThan(0);
  });

  it("a refusal is shown as the sentence the server refused with", async () => {
    stubApi(() => ({ status: 404, body: { error: "no such game: wire" } }));
    show("ports");
    expect(await screen.findByText("no such game: wire")).toBeInTheDocument();
  });
});

describe("the panel as a dialog", () => {
  // The panel is `position: fixed` over the whole board, so without this a
  // keyboard user who opens it is left with focus on the nav button behind it
  // and no way out but Tab — forward through every remaining nav control and,
  // during a placement prompt, all eighteen board tiles now hidden underneath.
  it("takes focus when it opens", async () => {
    stubApi(() => wire.ports);
    show("ports");
    await screen.findByText(/Take ONE card/);
    expect(screen.getByRole("dialog")).toHaveFocus();
  });

  it("closes on Escape", async () => {
    stubApi(() => wire.ports);
    const { onClose, user } = show("ports");
    await screen.findByText(/Take ONE card/);
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalled();
  });

  it("closes on its own button", async () => {
    stubApi(() => wire.ports);
    const { onClose, user } = show("ports");
    await user.click(await screen.findByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalled();
  });

  it("names itself for a screen reader", async () => {
    stubApi(() => wire.labs);
    show("labs");
    expect(screen.getByRole("dialog")).toHaveAccessibleName("Laboratory offer");
  });
});

describe("what each surface says", () => {
  // The zone gates are FOLDED FROM THE SLOTS, never typed as prose: `min_power`
  // is on the wire and the server derives it from the engine's own
  // `port_accessible_zones`, so a re-pricing moves the sentence with the chip
  // printed under it.
  it("the port offer states each zone's gate from the slots themselves", async () => {
    stubApi(() => wire.ports);
    show("ports");
    await screen.findByText(/Take ONE card/);
    const gates = new Map<string, number>();
    for (const board of wire.ports.boards) {
      for (const slot of board.slots) {
        const seen = gates.get(slot.zone);
        if (seen === undefined || slot.min_power < seen) gates.set(slot.zone, slot.min_power);
      }
    }
    expect(gates.size).toBeGreaterThan(0);
    for (const [zone, power] of gates) {
      expect(screen.getByText(new RegExp(`${zone.toUpperCase()} needs power ${power}`)))
        .toBeInTheDocument();
    }
  });

  // The log is capped by the server only when asked; `total` stays truthful
  // either way so the client can always say how many it is not showing.
  it("the log says how many events there are", async () => {
    stubApi(() => wire.log);
    show("log");
    expect(await screen.findByText(new RegExp(`${wire.log.total} event`))).toBeInTheDocument();
  });

  it("every area's ladder is drawn", async () => {
    stubApi(() => wire.areas);
    show("areas");
    await screen.findByText(/A00/);
    const cards = wire.areas.rows.flat();
    expect(cards.length).toBeGreaterThan(0);
    expect(screen.getAllByRole("heading", { level: 3 })).toHaveLength(cards.length);
    for (const card of cards) {
      expect(card.ladder.length).toBeGreaterThan(0);
    }
  });

  // **At 4P, not 2P, and that is the point.** `board_gen`'s 2P tile set has no
  // Canal, so every 2P area has a ladder and the client's "never an action
  // area" branch has no payload behind it. The first draft of this test wrapped
  // that assertion in `if (actionless.length > 0)`, which is a clause that
  // silently never ran.
  it("an actionless area says so, which only a 4P board contains", async () => {
    const actionless = wire.areas4p.rows.flat().filter((c) => c.ladder.length === 0);
    expect(actionless, "the 4P panel must carry the Canal").toHaveLength(1);
    stubApi(() => wire.areas4p);
    show("areas");
    await screen.findByText(/A00/);
    expect(screen.getAllByText("never an action area")).toHaveLength(actionless.length);
  });
});

// ---- M3.5: the panel is a real dialog, and it stops stealing focus ----
//
// The panel has asserted `aria-modal="true"` since M2 without ever being modal.
// Measured in real Chromium: six Tab presses from the open panel land on a nav
// `button.ghost` outside it, and Chrome's accessibility tree still reports the
// background controls as `ignored=false`.
//
// The fix the prep document proposed for this — `inert` on `.stage` — was
// measured NOT to work, and it is worth writing down why, because the sentence
// reads plausibly: `nav.views` and all three `.decision` blocks are SIBLINGS of
// `.stage`, not descendants. Setting it removes the board tiles and the legend
// and nothing else, and the tab walk then escapes at press 2 instead of press 6
// — strictly worse. `<dialog>` + `showModal()` was the option measured to close
// both channels at once: the tab order is contained AND the background leaves
// the accessibility tree, which no hand-rolled attribute can do.

describe("the surface is a real dialog", () => {
  it("is a <dialog>, so the browser owns the modality it claims", () => {
    const { view } = show("areas");
    const panel = view.container.querySelector(".surface");
    expect(panel, "the panel must render").not.toBeNull();
    // jsdom implements `showModal` but not focus containment, so this asserts
    // the ELEMENT — which is what makes the containment the browser's job
    // rather than an attribute this client asserts about itself. The tab order
    // itself is asserted in the browser walk, where it means something.
    expect(panel!.tagName, "a div cannot be modal, whatever it says it is").toBe("DIALOG");
  });

  // **The focus yank, and why the obvious fix is the wrong one.**
  //
  // `App.tsx` passed `onClose={() => setSurface(null)}` — a fresh identity on
  // every render — into an effect whose dependency list was `[onClose]` and
  // whose body focused the panel. Any state change in `App` therefore re-ran it
  // and dragged focus back to the panel mid-interaction; measured by blurring to
  // `<body>` and firing a programmatic click that moves no focus of its own.
  //
  // The tempting fix is `[]`. That is WRONG here and this cell is what says so:
  // `<Surface>` is not remounted when you switch panels — same component, new
  // `name` — so with `[]` the effect never re-runs and switching from Areas to
  // Log leaves focus behind on the nav button. The dependency is `[name]`.
  it("takes focus when the surface CHANGES, not on every parent render", async () => {
    stubApi((call) => {
      if (call.path.endsWith("/areas")) return wire.areas;
      if (call.path.endsWith("/log")) return wire.log;
      throw new Error(`unexpected ${call.path}`);
    });

    // A fresh `onClose` identity on every render, exactly as `App` supplies it.
    const rerenderWith = (name: "areas" | "log") => (
      <Surface
        name={name}
        game="wire"
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        onClose={() => undefined}
      />
    );

    const view = render(rerenderWith("areas"));
    const panel = view.container.querySelector<HTMLElement>(".surface")!;
    await waitFor(() => expect(document.activeElement).toBe(panel));

    // Move focus off the panel, then re-render with the SAME surface and a new
    // `onClose` identity — the parent-render case. Focus must stay put.
    (document.activeElement as HTMLElement).blur();
    view.rerender(rerenderWith("areas"));
    expect(
      document.activeElement,
      "a parent render dragged focus back into the panel",
    ).not.toBe(panel);

    // Now change the surface. Focus must follow, or a keyboard user who
    // switches panels is left behind on the nav.
    view.rerender(rerenderWith("log"));
    await waitFor(() =>
      expect(
        view.container.querySelector<HTMLElement>(".surface"),
        "switching surfaces left focus outside the panel",
      ).toBe(document.activeElement),
    );
  });
});

// ---- M3.5 / F5: the two surfaces that close the parity claim ----
//
// `guidingpath.md:204` records "the audited CLI parity gap closed" as
// OVERSTATED, and names the two rows still open. These are them.

describe("the tracks surface", () => {
  // It also fixes a live bug. Every Areas card whose area is the Church or the
  // Customs carries `companion: "Tracks"` and the panel renders "detail lives on
  // the tracks surface" — pointing at a surface the registry did not have.
  // `screen_model.rs:826` maps `Church | Customs => Tracks`, and every board has
  // exactly one of each, so that was TWO dangling pointers on every board at
  // every player count, by construction.
  it("is in the registry, so no companion pointer names a surface that is missing", () => {
    const keys = new Set(SURFACES.map((s) => s.key));
    // The pointers the Areas panel can actually render, lower-cased exactly as
    // `Surfaces.tsx` renders them.
    for (const companion of ["Ports", "Labs", "Tracks"]) {
      expect(
        keys.has(companion.toLowerCase() as (typeof SURFACES)[number]["key"]),
        `the Areas panel points at a "${companion.toLowerCase()}" surface that does not exist`,
      ).toBe(true);
    }
  });

  it("prints every space's points and rider as text, not as a tooltip", async () => {
    const tracks = wire.optionsMany.frame.dashboard.find((p) => "Tracks" in p);
    expect(tracks, "the fixture must carry a Tracks panel").toBeDefined();
    if (!tracks || !("Tracks" in tracks)) throw new Error("fixture moved");

    stubApi(() => {
      throw new Error("the tracks surface must fetch NOTHING — it is already on the frame");
    });
    const view = render(
      <Surface
        name="tracks"
        game="wire"
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        tracks={tracks.Tracks}
        onClose={() => undefined}
      />,
    );
    await screen.findByText(/Church/);
    const text = view.container.textContent ?? "";
    // The CLI prints per-space VP and riders; the Dashboard has them in `title=`
    // only, which is invisible on touch, to a screen reader, and to any
    // assertion. Every church faith number and its VP must be READABLE here.
    for (const space of tracks.Tracks.church) {
      expect(text, `faith ${space.faith} is missing from the tracks surface`).toContain(
        String(space.faith),
      );
    }
    for (const space of tracks.Tracks.customs) {
      expect(text, `customs space ${space.space} is missing`).toContain(String(space.space));
    }
    // Anti-vacuity: digits alone would pass the two loops above on almost any
    // page. The riders are what the tooltip was hiding.
    expect(text).toMatch(/faith/i);
    expect(text).toMatch(/VP|★/);
  });
});

describe("the building sites surface", () => {
  // The owner's cards: one per AREA, showing which spaces pay what and where
  // shophouses and the trading house are built.
  //
  // **It reads the AREAS frame and never `/index/sites`.** The index face is the
  // pristine registry card — `build_site_index` clones `rules.building_sites` —
  // so its `shophouses_built` is `[null,null,null,null]` on all 24 faces forever
  // and its `trading_house_owner` is null forever. An overlay built against it
  // renders a permanently empty board. `types.ts` already carried that caveat in
  // prose and the prep document steered at the fields anyway, which is why this
  // surface takes the live model instead.
  it("draws one card per area that has a site, from the live frame", async () => {
    const withSites = wire.areas.rows.flat().filter((c) => c.tile.site !== null);
    expect(withSites.length, "the fixture must carry sites").toBeGreaterThan(0);

    stubApi((call) => {
      if (call.path.endsWith("/areas")) return wire.areas;
      throw new Error(`unexpected ${call.path}`);
    });
    const view = render(
      <Surface
        name="buildings"
        game="wire"
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        onClose={() => undefined}
      />,
    );
    await screen.findByText(/One card per area/i);
    expect(view.container.querySelectorAll(".site-card")).toHaveLength(withSites.length);
  });

  it("marks a built space in the builder's seat colour, from the LIVE model", async () => {
    // A site with a shophouse built on its second space and a trading house
    // owned — the state the INDEX payload can never reach, which is the whole
    // reason this surface reads the frame.
    const built = structuredClone(wire.areas) as typeof wire.areas;
    const first = built.rows.flat().find((c) => c.tile.site !== null)!;
    first.tile.site!.slots[1]!.built_by = 1;
    first.tile.site!.trading_house_owner = 0;

    stubApi((call) => {
      if (call.path.endsWith("/areas")) return built;
      throw new Error(`unexpected ${call.path}`);
    });
    const view = render(
      <Surface
        name="buildings"
        game="wire"
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        onClose={() => undefined}
      />,
    );
    await screen.findByText(/One card per area/i);

    const card = view.container.querySelector(".site-card")!;
    const marks = card.querySelectorAll(".site-space.built");
    expect(marks.length, "the built shophouse is not marked").toBe(1);
    // The seat colour is how a player reads WHOSE it is at a glance, and it is
    // the same `seatClass` the board tile already uses — never a second palette.
    expect(marks[0]!.className, "the built mark carries no seat colour").toMatch(/seat-blue/);
    expect(
      card.querySelector(".site-trading-house.built")?.className,
      "the trading house owner is not marked",
    ).toMatch(/seat-red/);
  });
});
