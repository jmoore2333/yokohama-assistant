// Setup: the deal, and the seed that must survive the round trip.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useSections } from "./BoardsMenu";
import { NewGame } from "./NewGame";
import type { ClientState } from "./types";
import { loadSections } from "./sections";
import { gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

/** A seed at the top of the `u64` range — the shape `draw_seed` actually
 *  produces, and the shape a JSON number cannot hold: `Number("18446744073709551557")`
 *  is 18446744073709552000, whose last four digits are not in the seed. */
const BIG_SEED = "18446744073709551557";

/** The screen with a parent holding the sections state, which is where it lives
 *  (R-T12-4: `App` owns it, so the set table and the game agree about which
 *  boards are on). A cell here can therefore still watch a toggle take effect,
 *  and `App.test.tsx` carries the end-to-end claim that the choice survives
 *  Start. */
function Host({ onStarted }: { onStarted: (s: ClientState, opening: boolean) => void }) {
  const [sections, onToggleSection] = useSections();
  return (
    <NewGame
      art={wire.artManifest}
      labels={wire.labels}
      sections={sections}
      onToggleSection={onToggleSection}
      onStarted={onStarted}
    />
  );
}

function show(handler: (call: ApiCall) => Reply) {
  const onStarted = vi.fn();
  const stub = stubApi(handler);
  render(<Host onStarted={onStarted} />);
  return { onStarted, stub, user: userEvent.setup() };
}

/** Choose the first card for every human seat.
 *
 *  Start waits for a choice on EVERY human seat, so a single click is not
 *  enough on `wire.preview`: that is the bot-free corpus game and BOTH of its
 *  seats are human. The cells that use this are about what the create request
 *  carries and not about which card was chosen, so they take the first of each
 *  seat's two. */
async function chooseAll(user: ReturnType<typeof userEvent.setup>) {
  for (const seat of document.querySelectorAll(".deal-seat.human")) {
    await user.click(seat.querySelector("button.deal-pick")!);
  }
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

    await chooseAll(user);

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

    await chooseAll(user);
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
    await chooseAll(user);
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

// The set table (M5.5 spec §3.3.3): the deal happens over the position the
// rulebook has on the table at step 12, and the choice is per HUMAN seat.
describe("the set table", () => {
  const dealt = async (fixture: typeof wire.preview) => {
    const shown = show((call) => {
      if (call.method === "GET") return [];
      if (call.path.endsWith("preview")) return fixture;
      return wire.options;
    });
    await shown.user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    return shown;
  };

  it("shows the table the rulebook has set at step 12: the pyramid and the strips, not a list", async () => {
    await dealt(wire.preview);
    expect(document.querySelectorAll(".board .tile")).toHaveLength(wire.preview.table.frame.board.tiles.length);
    // `.board-strip` and not the section wrapper: `TableSections` renders its
    // "loading…" placeholder under the very same `table-sec ports` class, so a
    // check on the section passes while the strip says it is still fetching —
    // and a set table with no game to fetch FROM is exactly where that would
    // happen. The offers ride on `preview.table`, so they must be drawn.
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    expect(document.querySelector(".table-sec.labs .board-strip")).not.toBeNull();
    expect(document.querySelector(".table-sec.achievements")).not.toBeNull();
    expect(screen.getByText(/setting up/)).toBeInTheDocument();
    expect(screen.getByText(wire.preview.seed)).toBeInTheDocument();
  });

  it("shows every human seat's two cards with the bot's Keep score and the CLI's hedge, and Start waits for a choice", async () => {
    const { user } = await dealt(wire.preview);
    for (const deal of wire.preview.deals) {
      for (const c of deal.cards) {
        expect(screen.getByRole("button", { name: `keep O${String(c.card.id).padStart(2, "0")} for seat ${deal.seat}` })).toBeInTheDocument();
        expect(screen.getAllByText(new RegExp(`Keep score ${c.keep_score}\\b`)).length).toBeGreaterThan(0);
      }
    }
    expect(screen.getByText(/hand-set, unmeasured, and no substitute for your own read/)).toBeInTheDocument();
    // **No chip rows on a deal card (R-T12-3).** The card ART already prints
    // both the requirements and the rewards, and the two chip rows under it
    // are what pushed the bot's own deal below the fold at 1710x869. The id,
    // the country and the Keep score stay.
    expect(document.querySelectorAll(".deal .chips")).toHaveLength(0);
    expect(document.querySelectorAll(".deal .card-note")).toHaveLength(4);
    const start = screen.getByRole("button", { name: "Start" });
    expect(start).toBeDisabled();
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 0/ })[1]!);
    expect(start).toBeDisabled();
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 1/ })[0]!);
    expect(start).toBeEnabled();
  });

  it("shows the bot's two cards, marks the one it keeps, and offers no choice on them", async () => {
    await dealt(wire.previewBot);
    const bot = wire.previewBot.deals.find((d) => d.is_bot)!;
    expect(bot.bot_keep === 1 || bot.bot_keep === 2, "the fixture's bot seat carries a keep").toBe(true);
    const seat = document.querySelector(`.deal-seat.bot[data-seat="${bot.seat}"]`)!;
    expect(seat).not.toBeNull();
    expect(seat.querySelectorAll("button.deal-pick")).toHaveLength(0);
    expect(seat.querySelectorAll(".card-face")).toHaveLength(2);
    expect(seat.querySelector(".bot-keeps")?.textContent).toMatch(/bot keeps this/);
    const kept = bot.cards[bot.bot_keep! - 1]!.card.id;
    expect(seat.querySelector(".bot-keeps")?.closest(".deal-card")?.textContent).toContain(`O${String(kept).padStart(2, "0")}`);
  });

  // **The mark has to READ `bot_keep`, and one fixture can only prove half of
  // that.** The corpus payload's bot keeps card 1, so a client that marked the
  // first card unconditionally passes the cell above. This flips that one field
  // on the committed payload — the derivation `BIG_SEED` already makes in this
  // file — and asks for the mark on the other card.
  it("marks the second card when that is the one the bot keeps", async () => {
    const bot = wire.previewBot.deals.find((d) => d.is_bot)!;
    await dealt({
      ...wire.previewBot,
      deals: wire.previewBot.deals.map((d) => (d.is_bot ? { ...d, bot_keep: 2 as const } : d)),
    });
    const seat = document.querySelector(`.deal-seat.bot[data-seat="${bot.seat}"]`)!;
    expect(seat.querySelector(".bot-keeps")?.closest(".deal-card")?.textContent).toContain(
      `O${String(bot.cards[1]!.card.id).padStart(2, "0")}`,
    );
  });

  // **R-T12-2.** Spec §3.3.3 asks for the Boards menu here "so a player can read
  // a site card at full size before choosing". Same component as the game
  // banner's, same storage: what is switched here is what the game opens with.
  it("carries the Boards menu, and switching a board off here is remembered", async () => {
    const { user } = await dealt(wire.preview);
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    await user.click(screen.getByText(/^Boards/));
    await user.click(screen.getByRole("button", { name: "Port" }));
    expect(document.querySelector(".table-sec.ports")).toBeNull();
    // The strip going is not the claim on its own — a component that dropped it
    // from its own state would pass that. The claim is that the CHOICE survives
    // this screen, which is what `saveSections` is for.
    expect(loadSections()).not.toContain("ports");
    // And the summary counts what is on, with the menu open or shut.
    expect(screen.getByText(/^Boards/)).toHaveTextContent(`Boards · ${loadSections().length} on`);
  });

  it("sends a keep per human seat and null per bot seat, numbered as on screen", async () => {
    const { stub, user, onStarted } = await dealt(wire.previewBot);
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 0/ })[1]!);
    await user.click(screen.getByRole("button", { name: "Start" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, true));
    const create = stub.matching("POST", "/api/games").filter((c) => !c.path.endsWith("preview"));
    expect((create[0]!.body as { keeps: unknown }).keeps).toEqual([2, null]);
  });

  // **The preview request's own `keeps`, which nothing on this side pinned.**
  // The server refuses `null` on a human seat and a value on a bot seat with a
  // 400 naming the seat — so a preview that sent the wrong shape would fail at
  // the deal with a sentence about a field the player never touched, and only
  // the server would have caught it. The preview keeps NEITHER card (it is the
  // deal, not the choice); the field is there to pass the shape check.
  //
  // **An all-human `[1, 1]` is not reachable from this screen and is not
  // asserted**: the wizard always names exactly one bot seat (`bot_seats:
  // [botSeat]`, and the select cannot be empty), so the shape is always one
  // `null` among values. What IS worth pinning is that the `null` tracks the
  // CHOSEN seat rather than sitting at index 1 — hence the second read.
  it("previews with a value on every human seat and null on the bot's, wherever the bot sits", async () => {
    const previewKeeps = async (seats?: { players: number; bot: number }) => {
      const { stub, user } = show((call) => {
        if (call.method === "GET") return [];
        return wire.previewBot;
      });
      if (seats) {
        await user.selectOptions(screen.getByLabelText("Players"), String(seats.players));
        await user.selectOptions(screen.getByLabelText("Bot seat"), String(seats.bot));
      }
      await user.click(screen.getByRole("button", { name: "Deal" }));
      await waitFor(() => expect(stub.matching("POST", "preview")).toHaveLength(1));
      return (stub.matching("POST", "preview")[0]!.body as { keeps: unknown }).keeps;
    };
    expect(await previewKeeps()).toEqual([1, null]);
    expect(await previewKeeps({ players: 3, bot: 2 })).toEqual([1, 1, null]);
  });

  it("resumes a saved game as NOT an opening", async () => {
    const { user, onStarted } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return ["wire"];
      if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return wire.options;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await user.click(await screen.findByRole("button", { name: "wire" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, false));
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
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, false));
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

// ---- the fix wave: deleting a saved game (final review I2) ------------------
//
// **Data loss with no guard, and M5 moved it one menu away from a running
// game.** `delete` beside each saved game removed the record on a single
// click; M5's Game menu put "New game" in the banner, so a player mid-game now
// lands on this list, where a stray click beside the game they meant to resume
// removes a different one. There is no undo — the record is gone.
//
// A two-step BUTTON and not `window.confirm`: a native dialog blocks the
// browser walk this repository tests with, and a modal the page cannot see is
// a modal no cell can assert. The step lives in the row, which is also where
// the mistake is made.
describe("deleting a saved game", () => {
  const twoGames = (call: ApiCall): Reply => {
    if (call.method === "GET" && call.path.endsWith("/api/games")) return ["wire", "sim-other"];
    if (call.method === "DELETE") return null;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  };

  it("does not delete on the first click", async () => {
    const { user, stub } = show(twoGames);
    await user.click(await screen.findByRole("button", { name: "delete wire" }));
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
    // The row is now asking, which is what makes the click above a step and
    // not a no-op.
    expect(screen.getByRole("button", { name: "confirm delete wire" })).toBeInTheDocument();
  });

  it("deletes the game the row names when the second click confirms it", async () => {
    const { user, stub } = show(twoGames);
    await user.click(await screen.findByRole("button", { name: "delete sim-other" }));
    await user.click(screen.getByRole("button", { name: "confirm delete sim-other" }));
    await waitFor(() => expect(stub.matching("DELETE", "/api/games/sim-other")).toHaveLength(1));
    // The OTHER game's row was never touched — the whole point of naming the
    // game in the button rather than confirming "a game".
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
    // And the list is re-read, so the screen agrees with the store.
    expect(stub.matching("GET", "/api/games").length).toBeGreaterThan(1);
  });

  it("puts the row back when the second click cancels", async () => {
    const { user, stub } = show(twoGames);
    await user.click(await screen.findByRole("button", { name: "delete wire" }));
    await user.click(screen.getByRole("button", { name: "cancel deleting wire" }));
    expect(screen.getByRole("button", { name: "delete wire" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "confirm delete wire" })).toBeNull();
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
  });
});
