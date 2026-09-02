// The enabled sections: fetched when on, refetched after a move, collapsed
// while replaying, drawn from the frame where the frame already has them.

import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { TableSections } from "./TableSections";
import { stubApi } from "./test/harness";
import { wire } from "./wire/contract";
import type { ClientState, PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function show(enabled: Parameters<typeof TableSections>[0]["enabled"], state: ClientState = wire.options, replaying = false) {
  return render(
    <TableSections enabled={enabled} game={state.id} state={state} replaying={replaying} view={state} art={wire.artManifest} labels={wire.labels} colors={COLORS} />,
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
      <TableSections enabled={["ports", "labs"]} game={next.id} state={next} replaying={false} view={next} art={wire.artManifest} labels={wire.labels} colors={COLORS} />,
    );
    await waitFor(() => expect(stub.matching("GET", "/ports")).toHaveLength(2));
    expect(stub.matching("GET", "/labs")).toHaveLength(2);
  });

  it("collapse the fetched sections while replaying and fetch nothing", async () => {
    const stub = stubApi(() => {
      throw new Error("nothing may be fetched while replaying");
    });
    show(["ports", "tracks"], wire.options, true);
    expect(await screen.findByText(/hidden while replaying/)).toBeInTheDocument();
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
