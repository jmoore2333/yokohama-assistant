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
