// The read-only surfaces: the drawers that remain, and the two ways they went
// wrong.
//
// The port offer, the laboratory offer, the tracks and the building sites
// retired as drawers in M5 W5a — they are boards, and they are drawn on the
// table now. What those four cells asserted lives in `Strips.test.tsx`.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Surface, SURFACES } from "./Surfaces";
import { gate, stubApi, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import { SECTION_LABELS, type SectionKey } from "./sections";
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

/** The log surface with the replay track wired, as `App` wires it. */
function showLog(scrubK: number | null, onScrub = vi.fn()) {
  const view = render(
    <Surface
      name="log"
      game="wire"
      art={wire.artManifest}
      labels={wire.labels}
      colors={COLORS}
      scrubK={scrubK}
      onScrub={onScrub}
      onClose={vi.fn()}
    />,
  );
  return { view, onScrub, user: userEvent.setup() };
}

describe("the payload and the branch it is rendered through", () => {
  // **The defect, exactly.** React renders BEFORE it runs effects, so switching
  // from Labs to Areas rendered the Areas branch against the Labs payload for
  // one frame, and `panel.rows.flat()` threw on a `rows` that a `LabsPanel`
  // does not have. Clearing the state inside the effect is one render too late;
  // the state has to carry its own identity, which is what `loaded.of` is.
  //
  // The Labs drawer retired in W5a, so the pair is Log→Areas now. It is the
  // same shape and the same guard: a `LogPanel` has no `rows` either.
  it("a payload is never rendered through another surface's branch", async () => {
    const held = gate<Reply>();
    stubApi((call) => {
      if (call.path.endsWith("/log")) return wire.log;
      if (call.path.endsWith("/areas")) return held.promise;
      throw new Error(`unexpected ${call.path}`);
    });

    const { view } = show("log");
    await screen.findByText(new RegExp(`${wire.log.total} event`));

    // Switch. The Areas request is in flight, so the only state in hand is the
    // Log payload — which the Areas branch must not be handed.
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
    expect(screen.queryByText(new RegExp(`${wire.log.total} event`))).toBeNull();

    held.resolve(wire.areas);
    await waitFor(() => expect(screen.queryByText("loading…")).toBeNull());
    // And the right one does render, so the guard is not simply refusing.
    expect(screen.getAllByText(/never an action area|A0\d/).length).toBeGreaterThan(0);
  });

  it("a refusal is shown as the sentence the server refused with", async () => {
    stubApi(() => ({ status: 404, body: { error: "no such game: wire" } }));
    show("log");
    expect(await screen.findByText("no such game: wire")).toBeInTheDocument();
  });
});

describe("the panel as a dialog", () => {
  // The panel is `position: fixed` over the whole board, so without this a
  // keyboard user who opens it is left with focus on the nav button behind it
  // and no way out but Tab — forward through every remaining nav control and,
  // during a placement prompt, all eighteen board tiles now hidden underneath.
  it("takes focus when it opens", async () => {
    stubApi(() => wire.log);
    show("log");
    await screen.findByText(new RegExp(`${wire.log.total} event`));
    expect(screen.getByRole("dialog")).toHaveFocus();
  });

  it("closes on Escape", async () => {
    stubApi(() => wire.log);
    const { onClose, user } = show("log");
    await screen.findByText(new RegExp(`${wire.log.total} event`));
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalled();
  });

  it("closes on its own button", async () => {
    stubApi(() => wire.log);
    const { onClose, user } = show("log");
    await user.click(await screen.findByRole("button", { name: "Close" }));
    expect(onClose).toHaveBeenCalled();
  });

  it("names itself for a screen reader", async () => {
    stubApi(() => wire.areas);
    show("areas");
    expect(screen.getByRole("dialog")).toHaveAccessibleName("Areas");
  });
});

describe("what each surface says", () => {
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

  // **The pointer names the SECTION'S OWN LABEL, and that is the whole bug.**
  // `companion` is the server's tag, so the raw word gives "the Labs section"
  // while the section on the table is headed "Laboratory" — a reader sent
  // looking for a name nothing on screen uses. The expected words are read from
  // `SECTION_LABELS`, so this cannot become a second list of names; the
  // registry side of the same claim is `Strips.test.tsx`'s companions cell.
  it("points at each companion by the label its section is headed with", async () => {
    const companions = wire.areas.rows.flat().map((c) => c.companion).filter(Boolean);
    expect(new Set(companions).size, "the fixture must carry more than one companion").toBeGreaterThan(1);
    stubApi(() => wire.areas);
    show("areas");
    await screen.findByText(/A00/);
    for (const companion of new Set(companions)) {
      const label = SECTION_LABELS[companion!.toLowerCase() as SectionKey];
      expect(
        screen.getAllByText(new RegExp(`detail lives in the ${label} section`)).length,
        `no card points at the "${label}" section`,
      ).toBeGreaterThan(0);
    }
    // Anti-vacuity: "Labs" and "Laboratory" differ, so the raw word must be
    // absent — without this the loop passes on a client that prints both.
    expect(screen.queryByText(/detail lives in the Labs section/)).toBeNull();
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

describe("the replay track", () => {
  // **`total + 1` stops, and the last one is the PRESENT.** A replay position
  // counts events APPLIED, so stop `n` is the position event `n` was decided in
  // and stop `total` is the game as it now stands — a real position with no
  // event to name. A track that ran `0..total` would be off by one at the end,
  // and one that called the last stop an event would be lying about it.
  it("runs from 0 to total inclusive, and names the last stop the present", async () => {
    stubApi(() => wire.log);
    showLog(null);
    const track = await screen.findByLabelText("Replay position");
    expect(track).toHaveAttribute("min", "0");
    expect(track).toHaveAttribute("max", String(wire.log.total));
    // Not scrubbing yet, so the control sits at the present and says so.
    expect(track).toHaveValue(String(wire.log.total));
    expect(screen.getByText("now")).toBeInTheDocument();
  });

  it("says which stop it is on once you are somewhere else", async () => {
    stubApi(() => wire.log);
    showLog(3);
    expect(await screen.findByText(`after 3 of ${wire.log.total}`)).toBeInTheDocument();
  });

  // The step buttons are the keyboard-and-touch path to the same stops the
  // range gives a mouse, so they are asserted to MOVE, not merely to exist.
  // 10 s: 5.9-6.1 s under the close sequence's load (E's close); green alone.
  it("steps one event at a time and cannot walk off either end", async () => {
    stubApi(() => wire.log);
    const { onScrub, user } = showLog(0);
    await screen.findByLabelText("Replay position");

    const back = screen.getByRole("button", { name: "Previous move" });
    expect(back).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Next move" }));
    expect(onScrub).toHaveBeenCalledWith(1);
  }, 10_000);

  it("stops at the present rather than past it", async () => {
    stubApi(() => wire.log);
    showLog(wire.log.total);
    await screen.findByLabelText("Replay position");
    expect(screen.getByRole("button", { name: "Next move" })).toBeDisabled();
  });

  // The log is the index: pressing a line goes to the position that line was
  // decided in. Asserted through the ROLE, because a div with an onClick is
  // unreachable by keyboard and invisible to a screen reader.
  it("a log line is a control that goes to its own position", async () => {
    stubApi(() => wire.log);
    const { onScrub, user, view } = showLog(null);
    // **Not entry 0.** Its index IS 0, so asserting `toHaveBeenCalledWith(0)`
    // is satisfied by a handler hard-coded to `move(0)` — mutation-proven: the
    // cell stayed green with `onClick={() => move(0)}`. A line whose index is
    // not zero is the difference between "goes to its own position" and "goes
    // to the top".
    const { container } = view;
    await screen.findByLabelText("Replay position");
    const lines = container.querySelectorAll<HTMLButtonElement>(".log-jump");
    expect(lines.length).toBe(wire.log.entries.length);

    const nth = 7;
    const target = wire.log.entries[nth]!;
    expect(target.index).toBe(nth);
    await user.click(lines[nth]!);
    expect(onScrub).toHaveBeenCalledWith(target.index);

    // And a second, different line — so a handler returning any single constant
    // fails whichever constant it picked.
    const other = wire.log.entries[12]!;
    await user.click(lines[12]!);
    expect(onScrub).toHaveBeenLastCalledWith(other.index);
  });

  // **"Back to live" is offered only while you are away from the present — in
  // three cells, one per case.** They were one cell that rendered the log three
  // times, and a render of the log is the heavy part of every cell here: that
  // one took over a second unloaded, and under three concurrent full suites it
  // ran out of vitest's 5 s budget in all three (measured: 5,325–5,490 ms,
  // "Test timed out in 5000ms" — the whole-branch review's L-11). Each case
  // now pays for one render.
  // 10 s: 5.9-6.1 s under the close sequence's load (E's close); green alone.
  it("offers a way back to the present while you are away from it", async () => {
    stubApi(() => wire.log);
    const { user, onScrub } = showLog(2);
    await user.click(await screen.findByRole("button", { name: "Back to live" }));
    expect(onScrub).toHaveBeenCalledWith(null);
  }, 10_000);

  it("offers no way back while you are at the present", async () => {
    stubApi(() => wire.log);
    showLog(null);
    await screen.findByLabelText("Replay position");
    expect(screen.queryByRole("button", { name: "Back to live" })).not.toBeInTheDocument();
  });

  // …and not at the last stop either, which this component labels "now".
  // Offering "Back to live" there asks the reader to return to where they
  // already are.
  it("offers no way back at the last stop, which it labels now", async () => {
    stubApi(() => wire.log);
    showLog(wire.log.total);
    await screen.findByText("now");
    expect(screen.queryByRole("button", { name: "Back to live" })).not.toBeInTheDocument();
  });

  // **The log still renders without the scrubber.** `onScrub` is optional, and
  // a surface that threw or drew a dead control without it would break the
  // panel for every caller that does not scrub.
  it("renders as a plain log when no scrubber is wired", async () => {
    stubApi(() => wire.log);
    render(
      <Surface
        name="log"
        game="wire"
        art={wire.artManifest}
        labels={wire.labels}
        colors={COLORS}
        onClose={vi.fn()}
      />,
    );
    expect(await screen.findByText(new RegExp(`${wire.log.total} event`))).toBeInTheDocument();
    expect(screen.queryByLabelText("Replay position")).not.toBeInTheDocument();
  });
});
