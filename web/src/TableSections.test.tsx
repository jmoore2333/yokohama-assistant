// The enabled sections: fetched when on, refetched after a move, drawn from the
// frame wherever the frame already has them, and collapsed only where it does
// not.

import { act, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TableSections } from "./TableSections";
import { stubApi } from "./test/harness";
import { wire } from "./wire/contract";
import type { ClientState, HistoricalFrame, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function show(
  enabled: Parameters<typeof TableSections>[0]["enabled"],
  state: ClientState | null = wire.options,
  replaying = false,
  view: ClientState | HistoricalFrame = state ?? wire.options,
  scrubbing = replaying,
) {
  return render(
    <TableSections enabled={enabled} game={state?.id ?? null} state={state} replaying={replaying} scrubbing={scrubbing} view={view} art={wire.artManifest} labels={wire.labels} colors={COLORS} />,
  );
}

describe("the sections on the table", () => {
  it("fetch only what is enabled, and draw the rest from the frame", async () => {
    const stub = stubApi((call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      throw new Error(`unexpected ${call.path}`);
    });
    show(["ports", "tracks", "achievements"]);
    await screen.findByText(/Port A/);
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    expect(stub.matching("GET", "/labs")).toHaveLength(0);
    expect(stub.matching("GET", "/areas")).toHaveLength(0);
    expect(document.querySelector(".table-sec.tracks")).not.toBeNull();
    expect(document.querySelector(".table-sec.achievements")).not.toBeNull();
    expect(document.querySelector(".table-sec.labs")).toBeNull();
  });

  // **A COUNT that moved, with no run on its way** (cycle E, R-E9): an undo's
  // shrink, a re-read that found the game moved, a run whose fetch failed —
  // the sections cannot come from a run, so they are asked for, once each.
  it("re-fetch when the count moves and no run is on its way, once per section", async () => {
    const stub = stubApi((call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      throw new Error(`unexpected ${call.path}`);
    });
    const view = show(["ports", "labs"]);
    await screen.findByText(/Port A/);
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    const next = { ...wire.options, events: wire.options.events + 1 };
    view.rerender(
      <TableSections enabled={["ports", "labs"]} game={next.id} state={next} replaying={false} scrubbing={false} view={next} art={wire.artManifest} labels={wire.labels} colors={COLORS} />,
    );
    await waitFor(() => expect(stub.matching("GET", "/ports")).toHaveLength(2));
    expect(stub.matching("GET", "/labs")).toHaveLength(2);
  });

  // **The port and the laboratory are properties of the POSITION now (M5.5).**
  // `/at/{k}` carries the offers as they stood, so the strip beside a
  // historical board is the strip that was there — and no request is needed to
  // draw it, which is what makes the walk's per-step redraw free.
  it("draw the port and the laboratory from a position that carries them, and fetch nothing", async () => {
    const stub = stubApi(() => {
      throw new Error("a position that carries its own offers needs no fetch");
    });
    show(["ports", "labs", "buildings", "tracks"], wire.options, true, wire.atK);
    expect(await screen.findByText(/Port A/)).toBeInTheDocument();
    expect(screen.getByText(/Laboratory A/)).toBeInTheDocument();
    expect(document.querySelector(".table-sec.ports.collapsed")).toBeNull();
    expect(document.querySelector(".table-sec.labs.collapsed")).toBeNull();
    // The building sites are the one fetched board with no position-shaped
    // version — their board is read from the LIVE game — so they still say so.
    expect(document.querySelector(".table-sec.buildings.collapsed")).not.toBeNull();
    expect(screen.getByText(/hidden while replaying/)).toBeInTheDocument();
    expect(document.querySelector(".table-sec.tracks")).not.toBeNull();
    expect(stub.calls).toHaveLength(0);
  });

  it("say so about the building sites when there is no game yet, and fetch nothing", async () => {
    const stub = stubApi(() => {
      throw new Error("there is no game whose board could be fetched");
    });
    show(["buildings", "tracks"], null, false, wire.options);
    expect(await screen.findByText(/available once the game starts/)).toBeInTheDocument();
    expect(document.querySelector(".table-sec.tracks")).not.toBeNull();
    expect(stub.calls).toHaveLength(0);
  });

  it("carry a refusal as the sentence the server refused with", async () => {
    stubApi((call) =>
      call.path.endsWith("/ports") ? { status: 500, body: { error: "the port is closed" } } : undefined,
    );
    show(["ports"]);
    expect(await screen.findByText(/the port is closed/)).toBeInTheDocument();
  });
});

// ---- what a frame costs: the sections follow the COUNT, and the run carries
// the offers (cycle E, Task 7; R-E9, ruling T7-a) ------------------------------
//
// The rule, arm by arm, with the fetch stub's call log as the witness: a new
// state at the SAME count fetches nothing (a picker step, a re-read); a count
// a run is on its way for fetches nothing, and the run's last frame — the live
// position, built by the same `build_ports`/`build_labs` — is what the strips
// draw; a shrink, and a run that failed, fetch each board ONCE. The building
// sites follow the board's own `site`s rather than the count.
//
// Every payload is the corpus's: the state is `options.json` at its own count
// and one past it, and the run's last frame is `at_k.json` re-indexed to that
// count — the edit every run cell in this suite makes. `/ports` and `at_k`'s
// port hold different cards (34… against 28…), which is what lets a cell say
// WHICH panel the strip drew.

describe("what a frame costs the sections (R-E9)", () => {
  const N = wire.options.events;
  const at = (events: number): ClientState => ({ ...wire.options, events });
  /** The run's last frame for count `k`: the live position. */
  const liveAt = (k: number): HistoricalFrame => ({ ...wire.atK, index: k, total: k });
  const portCards = () =>
    [...document.querySelectorAll(".table-sec.ports [data-card]")].map((e) => e.getAttribute("data-card"));
  const panels = () =>
    stubApi((call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      throw new Error(`unexpected ${call.path}`);
    });
  const sections = (
    state: ClientState,
    extra: { live?: HistoricalFrame | null; runFor?: number | null; enabled?: Parameters<typeof TableSections>[0]["enabled"] } = {},
  ) => (
    <TableSections
      enabled={extra.enabled ?? ["ports", "labs"]}
      game={state.id}
      state={state}
      replaying={false}
      scrubbing={false}
      view={state}
      art={wire.artManifest}
      labels={wire.labels}
      colors={COLORS}
      live={extra.live ?? null}
      runFor={extra.runFor ?? null}
    />
  );
  /** Let every effect and every answer land. */
  const settled = () => act(async () => {
    await new Promise((r) => setTimeout(r, 0));
  });

  it("fetch nothing for a new state at the same count — a picker step, a re-read", async () => {
    const stub = panels();
    const view = render(sections(at(N)));
    await screen.findByText(/Port A/);
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    expect(stub.matching("GET", "/labs")).toHaveLength(1);
    view.rerender(sections({ ...at(N) }));
    view.rerender(sections({ ...at(N) }));
    await settled();
    expect(stub.matching("GET", "/ports"), "a picker step asks the port nothing").toHaveLength(1);
    expect(stub.matching("GET", "/labs"), "nor the laboratory").toHaveLength(1);
  });

  it("fetch nothing while a run is on its way for the new count, and draw the run's last frame when it lands", async () => {
    const stub = panels();
    const view = render(sections(at(N)));
    await screen.findByText(/Port A/);
    expect(portCards(), "premise: the mount read's port").toEqual(wire.ports.boards.flatMap((b) => b.slots.map((s) => String(s.card))));
    // The count moved and the run for it is being fetched: nothing is asked.
    view.rerender(sections(at(N + 1), { runFor: N + 1 }));
    await settled();
    expect(stub.matching("GET", "/ports"), "a run is on its way").toHaveLength(1);
    expect(stub.matching("GET", "/labs")).toHaveLength(1);
    // The run landed: its last frame is the live position, and the strips are
    // drawn from it — still without a request.
    view.rerender(sections(at(N + 1), { live: liveAt(N + 1) }));
    await settled();
    expect(stub.matching("GET", "/ports"), "the run carried the port").toHaveLength(1);
    expect(stub.matching("GET", "/labs"), "and the laboratory").toHaveLength(1);
    expect(portCards(), "the strip draws the run's live frame").toEqual(
      wire.atK.ports.boards.flatMap((b) => b.slots.map((s) => String(s.card))),
    );
  });

  it("fetch each board once on a shrink — an undo — whatever run was held", async () => {
    const stub = panels();
    const view = render(sections(at(N + 1), { live: liveAt(N + 1) }));
    await settled();
    expect(stub.matching("GET", "/ports"), "a mount whose run is in hand asks nothing").toHaveLength(0);
    view.rerender(sections(at(N), { live: liveAt(N + 1) }));
    await waitFor(() => expect(stub.matching("GET", "/ports")).toHaveLength(1));
    await settled();
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    expect(stub.matching("GET", "/labs")).toHaveLength(1);
  });

  it("fetch each board once when the run for the new count failed", async () => {
    const stub = panels();
    const view = render(sections(at(N)));
    await screen.findByText(/Port A/);
    view.rerender(sections(at(N + 1), { runFor: N + 1 }));
    await settled();
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    // The run's fetch failed: nothing is on its way and no frame is live.
    view.rerender(sections(at(N + 1)));
    await waitFor(() => expect(stub.matching("GET", "/ports")).toHaveLength(2));
    await settled();
    expect(stub.matching("GET", "/ports"), "once").toHaveLength(2);
    expect(stub.matching("GET", "/labs"), "once").toHaveLength(2);
  });

  it("re-read the building sites only when the board's sites changed", async () => {
    const stub = panels();
    const view = render(sections(at(N), { enabled: ["buildings"] }));
    await waitFor(() => expect(stub.matching("GET", "/areas")).toHaveLength(1));
    // A move that built nothing: the count moved, the sites did not.
    view.rerender(sections(at(N + 1), { enabled: ["buildings"] }));
    await settled();
    expect(stub.matching("GET", "/areas"), "a move that built nothing").toHaveLength(1);
    // A build: one site's space now has an owner.
    const built = JSON.parse(JSON.stringify(at(N + 2))) as ClientState;
    const tile = built.frame.board.tiles.find((t) => t.site !== null)!;
    tile.site!.slots[0]!.built_by = 0;
    view.rerender(sections(built, { enabled: ["buildings"] }));
    await waitFor(() => expect(stub.matching("GET", "/areas")).toHaveLength(2));
    await settled();
    expect(stub.matching("GET", "/areas"), "a build re-reads the sites once").toHaveLength(2);
  });
});
