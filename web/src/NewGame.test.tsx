// Setup: the deal, and the seed that must survive the round trip.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NewGame } from "./NewGame";
import { gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

/** A seed at the top of the `u64` range — the shape `draw_seed` actually
 *  produces, and the shape a JSON number cannot hold: `Number("18446744073709551557")`
 *  is 18446744073709552000, whose last four digits are not in the seed. */
const BIG_SEED = "18446744073709551557";

function show(handler: (call: ApiCall) => Reply) {
  const onStarted = vi.fn();
  const stub = stubApi(handler);
  render(<NewGame art={wire.artManifest} onStarted={onStarted} />);
  return { onStarted, stub, user: userEvent.setup() };
}

describe("a double-clicked Start", () => {
  // **The second site of the guard `App` already documents**, and the one with
  // the worse consequence. `run()` gated on the `busy` STATE, and state is what
  // makes the bug: React batches, so two clicks in the same tick both read
  // `busy === false` and both post. Two `POST /api/games` are then two
  // DIFFERENT games — the server draws a fresh bot seed per request, so with a
  // bot on the start seat each plays its own opening — and before the server
  // learned to serialize creates, the loser's record landed on disk while the
  // client was handed the winner's.
  it("posts create exactly once", async () => {
    const held = gate<unknown>();
    const { stub, user } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return [];
      if (call.path.endsWith("/api/games/preview")) return wire.preview;
      if (call.method === "POST" && call.path.endsWith("/api/games")) return held.promise;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());

    // BOTH clicks inside ONE `act`, so no render happens between them — the
    // case the ref exists for. Two `user.click`s would each flush, the button
    // would already be `disabled`, and the cell would pass with the guard
    // removed, which is not a test of the guard. `App.test.tsx` records the
    // same reasoning for the same mistake.
    const start = screen.getByRole("button", { name: "Start" });
    act(() => {
      fireEvent.click(start);
      fireEvent.click(start);
    });

    const creates = () =>
      stub.matching("POST", "/api/games").filter((c) => !c.path.endsWith("preview"));
    expect(creates()).toHaveLength(1);

    held.resolve(wire.options);
    await waitFor(() => expect(creates()).toHaveLength(1));
  });
});

describe("the seed", () => {
  // **The defect, exactly.** The setup flow ROUND-TRIPS the seed: preview draws
  // one and deals from it, the client shows the deal, and the client sends the
  // seed back to create the game. A seed that went through a JS number on the
  // way would deal a DIFFERENT GAME than the one whose cards the player chose
  // between — the rounding is silent and the game is wrong from move one.
  it("goes back to the server as the exact string the preview returned", async () => {
    // Enough of it to know it did not survive as a number.
    expect(String(Number(BIG_SEED))).not.toBe(BIG_SEED);

    const { stub, user, onStarted } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return [];
      if (call.path.endsWith("/api/games/preview")) return { ...wire.preview, seed: BIG_SEED };
      if (call.path.endsWith("/api/games")) return wire.options;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    expect(screen.getByText(BIG_SEED)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Start" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalled());

    const create = stub.matching("POST", "/api/games").filter((c) => !c.path.endsWith("preview"));
    expect(create).toHaveLength(1);
    expect((create[0]!.body as { seed: unknown }).seed).toBe(BIG_SEED);
  });

  it("is sent as typed when the player names one, and omitted when they do not", async () => {
    const { stub, user } = show((call) => {
      if (call.method === "GET") return [];
      return { ...wire.preview, seed: BIG_SEED };
    });

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(stub.matching("POST", "preview")).toHaveLength(1));
    // Blank means "the server draws one", which is the right default: the board
    // is an injective fingerprint of the setup seed, so a small or sequential
    // seed hands over the hidden set.
    expect(stub.matching("POST", "preview")[0]!.body).not.toHaveProperty("seed");

    await user.click(screen.getByRole("button", { name: "Re-deal" }));
    await user.type(screen.getByLabelText("Seed"), "12345");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(stub.matching("POST", "preview")).toHaveLength(2));
    expect((stub.matching("POST", "preview")[1]!.body as { seed: string }).seed).toBe("12345");
  });
});

// **The corpus covers RESPONSES only.** `NewGameRequest` and the `{line, expect}`
// input body are hand-written on this side and hand-written again as a `json!`
// literal in `wire_fixtures.rs`, and the server marks `start_seat`, `seed`,
// `bot_seats` and `name` `#[serde(default)]` with no `deny_unknown_fields` — so
// a renamed REQUEST field is not a 400, it is a default, and the game starts
// wrong. Nothing here can close that (it needs a server decision, recorded in
// the M2.5 record §5); what it can do is pin the shape this client actually
// sends, so the client half cannot drift unnoticed.
describe("the request the client sends", () => {
  it("names every field the create endpoint reads, and no others", async () => {
    const { stub, user } = show((call) => {
      if (call.method === "GET") return [];
      if (call.path.endsWith("preview")) return wire.preview;
      return wire.options;
    });
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Start" }));

    const create = stub
      .matching("POST", "/api/games")
      .filter((c) => !c.path.endsWith("preview"));
    await waitFor(() => expect(create.length).toBeGreaterThan(0));
    expect(Object.keys(create[0]!.body as object).sort()).toEqual(
      ["bot_seats", "colors", "keeps", "players", "seed", "start_seat"].sort(),
    );
  });
});

describe("the deal", () => {
  it("shows every seat's two cards before anyone chooses", async () => {
    const { user } = show((call) => {
      if (call.method === "GET") return [];
      return wire.preview;
    });
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    for (const deal of wire.preview.deals) {
      for (const card of deal.cards) {
        expect(
          screen.getAllByText(`O${String(card.id).padStart(2, "0")}`).length,
        ).toBeGreaterThan(0);
      }
    }
  });

  it("sends one keep per seat, numbered as the cards are on screen", async () => {
    const { stub, user } = show((call) => {
      if (call.method === "GET") return [];
      if (call.path.endsWith("preview")) return wire.preview;
      return wire.options;
    });
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Start" }));

    const create = stub.matching("POST", "/api/games").filter((c) => !c.path.endsWith("preview"));
    await waitFor(() => expect(create.length).toBeGreaterThan(0));
    const body = create[0]!.body as { keeps: number[]; players: number };
    expect(body.keeps).toHaveLength(body.players);
    for (const k of body.keeps) expect([1, 2]).toContain(k);
  });
});

describe("the saved games", () => {
  it("resumes one with a GET, because a record is not a restore", async () => {
    const { user, onStarted, stub } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return ["wire"];
      if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return wire.options;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await user.click(await screen.findByRole("button", { name: "wire" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options));
    expect(stub.matching("GET", "/api/games/wire")).toHaveLength(1);
  });

  it("says so when there are none", async () => {
    show((call) => {
      if (call.method === "GET") return [];
      throw new Error("unexpected");
    });
    expect(await screen.findByText(/none yet/)).toBeInTheDocument();
  });

  it("shows a refusal rather than swallowing it", async () => {
    const { user } = show((call) => {
      if (call.method === "GET") return [];
      return { status: 400, body: { error: "a game named \"wire\" already exists" } };
    });
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect(await screen.findByText(/already exists/)).toBeInTheDocument();
  });
});
