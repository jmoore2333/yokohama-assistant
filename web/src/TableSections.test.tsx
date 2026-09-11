// The enabled sections: fetched when on, refetched after a move, drawn from the
// frame wherever the frame already has them, and collapsed only where it does
// not.

import { render, screen, waitFor } from "@testing-library/react";
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

  it("re-fetch after the state changes, once per section", async () => {
    const stub = stubApi((call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      throw new Error(`unexpected ${call.path}`);
    });
    const view = show(["ports", "labs"]);
    await screen.findByText(/Port A/);
    expect(stub.matching("GET", "/ports")).toHaveLength(1);
    const next = { ...wire.options };
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
