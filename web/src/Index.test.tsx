// The card indexes and the final score, over server-emitted payloads.

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { IndexView, OutcomeView } from "./Index";
import { wire } from "./wire/contract";
import type { PlayerColor } from "./types";

const COLORS: PlayerColor[] = ["Red", "Blue"];

function index(kind: string, first = wire.indexOrders) {
  return render(
    <IndexView
      first={first}
      kind={kind}
      game="wire"
      art={wire.artManifest}
      labels={wire.labels}
      colors={COLORS}
    />,
  );
}

describe("an index is a view, and it says how much of the population it shows", () => {
  it.each([
    ["orders", wire.indexOrders, 36],
    ["techs", wire.indexTechs, 20],
    ["goals", wire.indexGoals, 3],
    ["sites", wire.indexSites, 24],
  ])("%s draws its whole population without throwing", (kind, model, total) => {
    index(kind, model);
    expect(screen.getByText(`showing ${total} of ${total}`)).toBeInTheDocument();
  });

  it("numbers nothing, because this API has no card capture", () => {
    // In the terminal an index open inside a capture numbers its cards. Here
    // the input grammar refuses the index words outright, so `target` is null
    // and so is every selection — and a number on screen would be a number that
    // answers nothing.
    for (const m of [wire.indexOrders, wire.indexTechs, wire.indexGoals, wire.indexSites]) {
      expect(m.target).toBeNull();
      for (const g of m.groups) for (const c of g.cards) expect(c.selection).toBeNull();
    }
  });

  it("shows how close every seat is, which the nav blurb promises", () => {
    // **The first draft did not, and the blurb beside it said it did.** That is
    // the M2.5 lesson in miniature: a claim a milestone makes about itself that
    // the screen does not keep. `current >= needed` is "ready by the numbers"
    // and not "claimable" — a claim also needs an assistant in hand — so the row
    // is marked, never relabelled.
    index("goals", wire.indexGoals);
    const goals = wire.indexGoals.groups.flatMap((g) => g.cards);
    expect(goals.length).toBeGreaterThan(0);
    for (const c of goals) {
      const face = c.face;
      if (!("Achievement" in face)) throw new Error("the goals index carries achievements");
      expect(face.Achievement.progress.length).toBeGreaterThan(0);
      for (const [seat, current, needed] of face.Achievement.progress) {
        void seat;
        expect(screen.getAllByText(`${current}/${needed}`).length).toBeGreaterThan(0);
      }
    }
  });

  it("sends the filter to the SERVER rather than narrowing in the browser", async () => {
    // There is exactly one card-entry grammar in this project. A client-side
    // filter would be a second one, and the two would disagree on the first
    // ambiguous prefix a player typed.
    const fetchMock = vi.fn(
      async (input: RequestInfo | URL): Promise<Response> => {
        void input;
        return new Response(JSON.stringify(wire.indexFiltered), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      },
    );
    vi.stubGlobal("fetch", fetchMock);
    const user = userEvent.setup();
    index("orders");
    await user.type(screen.getByLabelText("Filter"), "copper");
    await screen.findByText(`showing ${wire.indexFiltered.shown} of ${wire.indexFiltered.total}`);
    const url = String(fetchMock.mock.calls.at(-1)?.[0]);
    expect(url).toContain("/index/orders");
    expect(url).toContain("filter=copper");
  });
});

describe("the final score shows the parts, not just the total", () => {
  it("ranks the seats and itemises every category the engine scored", () => {
    render(<OutcomeView outcome={wire.result} colors={COLORS} />);
    for (const p of wire.result.standings) {
      expect(screen.getByText(String(p.breakdown.total))).toBeInTheDocument();
    }
    for (const label of ["VP", "Church", "Customs", "Tech", "Country", "Remainder"]) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });

  it("marks the winner rather than letting a margin imply one", () => {
    // A margin of 0 is not a win: two seats can tie on points and be separated
    // by the engine's own tie-break, so `won` is what the row is marked from.
    const { container } = render(<OutcomeView outcome={wire.result} colors={COLORS} />);
    const won = container.querySelectorAll("tr.won");
    expect(won).toHaveLength(1);
    expect(wire.result.standings.filter((p) => p.won)).toHaveLength(1);
    expect(wire.result.standings.find((p) => p.won)!.seat).toBe(wire.result.winner);
  });

  it("the margins are mirror images, because they are measured against each other", () => {
    const [first, second] = wire.result.standings;
    expect(first!.margin).toBe(-second!.margin);
    expect(first!.margin).toBeGreaterThan(0);
  });
});
