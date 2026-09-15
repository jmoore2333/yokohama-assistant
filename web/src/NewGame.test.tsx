// Setup: the form that opens a table, and the seed that must survive the round
// trip.
//
// **This screen is a FORM again (M6.1 §7.3).** The deal left it: a create opens
// a DEALING table and each seat fetches its own two cards from `GET …/deal`, so
// there is no preview to draw, no hot-seat pass to sequence and no Start to
// wait for. What is left is who sits where — one row per seat, `me` / another
// email / the bot — and the saved tables underneath. The set table's own cells
// moved to `Dealing.test.tsx` with the screen.

import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NewGame } from "./NewGame";
import { gamesNamed, gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { seatName } from "./tables";
import { wire } from "./wire/contract";
import type { TableView } from "./types";

/** A seed at the top of the `u64` range — the shape `draw_seed` actually
 *  produces, and the shape a JSON number cannot hold: `Number("18446744073709551557")`
 *  is 18446744073709552000, whose last four digits are not in the seed. */
const BIG_SEED = "18446744073709551557";

/** Who this browser is signed in as. **Taken from the DOOR, not from storage.**
 *  `App` reads `/api/me` and hands the answer down; a screen that read the
 *  claimed name itself would post a seat for a name the server may not answer
 *  to, which is the defect Task 7's review found at `NewGame.tsx:179`. */
const ME = "red@wire.test";

/** The form, with its callbacks defaulted.
 *
 *  **No `sections` and no art here any more.** This screen draws no board since
 *  the deal moved to `Dealing`, so it takes neither — `App` hands the enabled
 *  set to the dealing screen and to the game, which is where R-T12-4 wanted it
 *  and is one fewer prop to keep in step. */
function Host({
  me = ME,
  onCreated = () => {},
  onOpen = () => {},
}: {
  me?: string;
  onCreated?: (t: TableView) => void;
  onOpen?: (id: string) => void;
}) {
  return <NewGame me={me} onCreated={onCreated} onOpen={onOpen} />;
}

/** The create posts, and NOT the other posts beside them.
 *
 *  `matching("POST", "/api/games")` is a substring match, so it also catches
 *  every `/api/games/{id}/…` post. The create is the only one whose path ENDS
 *  there. */
const creates = (stub: ReturnType<typeof stubApi>): ApiCall[] =>
  stub.matching("POST", "/api/games").filter((c) => c.path.split("?")[0]!.endsWith("/api/games"));

/** The form, rendered over a stub. The saved list is fetched on mount, so a
 *  handler that answers nothing else still has to answer that. */
function show(handler: (call: ApiCall) => Reply, props: Parameters<typeof Host>[0] = {}) {
  const onCreated = vi.fn();
  const onOpen = vi.fn();
  const stub = stubApi(handler);
  render(<Host onCreated={onCreated} onOpen={onOpen} {...props} />);
  return { onCreated, onOpen, stub, user: userEvent.setup() };
}

/** The opening reply: a create answers 201 with a DEALING table (§4.2). */
const created = (call: ApiCall): Reply | undefined =>
  call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games")
    ? { status: 201, body: wire.tableDealing }
    : undefined;

/** The saved list, and nothing else. */
const listOnly = (call: ApiCall): Reply | undefined =>
  call.method === "GET" && call.path.split("?")[0]!.endsWith("/api/games") ? gamesNamed() : undefined;

describe("the seats the form posts", () => {
  // **The body §4.2 reads, and nothing this client used to send.** `keeps` and
  // `bot_seats` are gone: a create can no longer choose a card for anybody, and
  // which seats are bots is said by `seats`. The server refuses either by name,
  // so a client still sending them would fail at the deal with a sentence about
  // a field the player never touched.
  it("posts one seat entry per seat, me by default, and the bot only at two players", async () => {
    const stub = stubApi((call) => {
      if (call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games"))
        return { status: 201, body: wire.tableDealing };
      if (call.path.split("?")[0]!.endsWith("/api/games")) return wire.games;
      return undefined;
    });
    const onCreated = vi.fn();
    render(<Host me={ME} onCreated={onCreated} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    const body = creates(stub)[0]!.body as { seats: unknown[]; keeps?: unknown };
    expect(body.seats).toEqual([{ user: ME }, { bot: true }]);
    expect("keeps" in body).toBe(false);
    expect(onCreated).toHaveBeenCalledWith(wire.tableDealing);
  });

  // **R-P10, on the screen and not only in the server's refusal.** The bot is
  // measured at two players only, so at three and four the option is absent —
  // and the sentence saying why is beside the choice rather than waiting to
  // arrive as a 400 after the player has filled the form in.
  it("at three players every seat defaults to me and no bot is offered", async () => {
    const stub = stubApi((call) => created(call) ?? listOnly(call));
    render(<Host me={ME} />);
    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText("Players"), "3");
    const seats = screen.getAllByLabelText(/^Seat \d/);
    expect(seats).toHaveLength(3);
    for (const s of seats) {
      expect(within(s.parentElement!).queryByRole("option", { name: "bot" })).toBeNull();
    }
    expect(screen.getByText(/measured at two players only/)).toBeInTheDocument();

    // **The BODY, because the option list alone does not say what is posted.**
    // Mutation-proven the hard way: a `defaultSeats` that seated a bot at seat 1
    // whatever the count survived the absence check above AND a check on the
    // select's own value — React marks no option selected when the value
    // matches none, and the browser then shows the first, so the row READ "me"
    // while the form held "bot". What the server would have been sent is the
    // only statement that cannot be fooled that way, and it is a seat entry the
    // server refuses by name (R-P10).
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { seats: unknown[] }).seats).toEqual([
      { user: ME },
      { user: ME },
      { user: ME },
    ]);
  });

  // Two players is where the bot IS offered, so the cell above is a difference
  // and not a screen that never offers one.
  it("offers the bot at two players, which is what makes the absence above a rule", async () => {
    show(listOnly);
    const seat1 = screen.getByLabelText("Seat 1");
    expect(within(seat1.parentElement!).queryByRole("option", { name: "bot" })).not.toBeNull();
  });

  it("a seat set to another email posts that email", async () => {
    const stub = stubApi((call) => (call.method === "POST" ? created(call) : wire.games));
    render(<Host me={ME} />);
    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText("Seat 1"), "email");
    // **Normalized on the way out (R-1.1).** The server lowercases and trims an
    // address before it seats it, so a client that sent the typed form would
    // name a seat the table then holds under a different string — and the
    // player at that address would be told they are not seated.
    await user.type(screen.getByLabelText("Seat 1 email"), "Blue@Wire.test");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { seats: unknown[] }).seats[1]).toEqual({
      user: "blue@wire.test",
    });
  });

  // At three and four players the form seats the SAME person in every seat by
  // default, which is the hot-seat table this client has always supported —
  // now served one seat at a time by `GET …/deal` rather than sequenced here.
  it("names me in every seat at four players, and the bot in none", async () => {
    const stub = stubApi((call) => (call.method === "POST" ? created(call) : wire.games));
    render(<Host me={ME} />);
    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText("Players"), "4");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    const body = creates(stub)[0]!.body as { seats: unknown[]; players: number; colors: string[] };
    expect(body.players).toBe(4);
    expect(body.colors).toEqual(["Red", "Blue", "Yellow", "Green"]);
    expect(body.seats).toEqual([{ user: ME }, { user: ME }, { user: ME }, { user: ME }]);
  });

  // **The count going back down restores the bot**, so the rows are a
  // derivation of the player count and not a one-way latch. A form that left
  // three human seats behind at two players would post a table with no bot in
  // it and no way to ask for one.
  it("puts the bot back when the count comes back down to two", async () => {
    const stub = stubApi((call) => (call.method === "POST" ? created(call) : wire.games));
    render(<Host me={ME} />);
    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText("Players"), "4");
    await user.selectOptions(screen.getByLabelText("Players"), "2");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { seats: unknown[] }).seats).toEqual([
      { user: ME },
      { bot: true },
    ]);
  });
});

// **The corpus covers RESPONSES only.** `NewGameRequest` is hand-written on this
// side and hand-written again as a `json!` literal in `wire_fixtures.rs`. The
// create body now carries `deny_unknown_fields`, so a renamed field is a 400
// rather than a silent default — but a field this client stops sending is still
// a default, and that is what these two cells pin.
describe("the request the client sends", () => {
  it("names every field the create endpoint reads, and no others", async () => {
    const { stub, user } = show((call) => created(call) ?? listOnly(call));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    // No `seed` and no `name`: both are omitted when the player leaves them
    // blank, which is what makes the server draw a seed and list the table
    // under its id.
    expect(Object.keys(creates(stub)[0]!.body as object).sort()).toEqual(
      ["colors", "players", "seats", "start_seat"].sort(),
    );
  });

  it("carries the name and the seed when the player gives them", async () => {
    const { stub, user } = show((call) => created(call) ?? listOnly(call));
    await user.type(screen.getByLabelText("Name"), "Thursday");
    await user.type(screen.getByLabelText("Seed"), "12345");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    const body = creates(stub)[0]!.body as { name: string; seed: string };
    expect(body.name).toBe("Thursday");
    expect(body.seed).toBe("12345");
  });

  // **The defect, exactly.** A seed is a `u64` and a JS number cannot hold one:
  // rounded on the way out, the server deals a DIFFERENT GAME than the one the
  // player asked for, silently and from move one. The client never converts —
  // the field is a string from the keystroke to the wire, and the server parses
  // it from a string or a number (`WireSeed`).
  it("sends a big seed to the create body as the exact string that was typed", async () => {
    // Enough of it to know it did not survive as a number.
    expect(String(Number(BIG_SEED))).not.toBe(BIG_SEED);
    const { stub, user } = show((call) => created(call) ?? listOnly(call));
    await user.type(screen.getByLabelText("Seed"), BIG_SEED);
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { seed: unknown }).seed).toBe(BIG_SEED);
  });
});

describe("a double-clicked Deal", () => {
  // **The second site of the guard `App` already documents**, and the one with
  // the worse consequence. `run()` gated on the `busy` STATE, and state is what
  // makes the bug: React batches, so two clicks in the same tick both read
  // `busy === false` and both post. Two `POST /api/games` are then two
  // DIFFERENT tables, and the client is handed whichever answered last.
  it("posts create exactly once", async () => {
    const held = gate<unknown>();
    const { stub } = show((call) => {
      if (call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games")) return held.promise;
      return listOnly(call);
    });
    // BOTH clicks inside ONE `act`, so no render happens between them — the
    // case the ref exists for. Two `user.click`s would each flush, the button
    // would already be `disabled`, and the cell would pass with the guard
    // removed, which is not a test of the guard.
    const deal = screen.getByRole("button", { name: "Deal" });
    act(() => {
      fireEvent.click(deal);
      fireEvent.click(deal);
    });
    expect(creates(stub)).toHaveLength(1);
    held.resolve({ status: 201, body: wire.tableDealing });
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
  });
});

describe("the saved tables", () => {
  // **A row hands the ID up, and no longer the game.** `App` decides what a
  // table is — a `GET …/table` says whether it is still dealing — so a screen
  // that fetched the game itself would refuse with a 409 on every table whose
  // seats have not all chosen.
  it("opens one by id, because only App knows whether it is dealing", async () => {
    const { user, onOpen, stub } = show((call) =>
      call.method === "GET" && call.path.split("?")[0]!.endsWith("/api/games") ? gamesNamed("wire") : undefined,
    );
    await user.click(await screen.findByRole("button", { name: "wire" }));
    expect(onOpen).toHaveBeenCalledWith("wire");
    // And nothing about the game was fetched here.
    expect(stub.matching("GET", "/api/games/wire")).toHaveLength(0);
  });

  it("says so when there are none", async () => {
    show(listOnly);
    expect(await screen.findByText(/none yet/)).toBeInTheDocument();
  });

  it("names each table, its size, and what it is waiting on", async () => {
    show((call) =>
      call.method === "GET" && call.path.split("?")[0]!.endsWith("/api/games") ? wire.games : undefined,
    );
    const row = (await screen.findByRole("button", { name: "wire" })).closest("li")!;
    expect(row).toHaveTextContent("2P · dealing");
    for (const seat of wire.games.games[0]!.seats) {
      expect(row).toHaveTextContent(seatName(seat));
    }
  });

  it("shows a refusal rather than swallowing it", async () => {
    const { user } = show((call) =>
      call.method === "GET" && call.path.split("?")[0]!.endsWith("/api/games")
        ? gamesNamed()
        : { status: 400, body: { error: 'a game named "wire" already exists' } },
    );
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect(await screen.findByText(/already exists/)).toBeInTheDocument();
  });
});

// ---- deleting a saved game (M5's final review I2) ---------------------------
//
// **Data loss with no guard, and M5 moved it one menu away from a running
// game.** `delete` beside each saved game removed the record on a single click;
// M5's Game menu put "New game" in the banner, so a player mid-game now lands on
// this list, where a stray click beside the game they meant to resume removes a
// different one. There is no undo — the record is gone.
describe("deleting a saved game", () => {
  const twoGames = (call: ApiCall): Reply => {
    if (call.method === "GET" && call.path.split("?")[0]!.endsWith("/api/games"))
      return gamesNamed("wire", "sim-other");
    if (call.method === "DELETE") return null;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  };

  it("does not delete on the first click", async () => {
    const { user, stub } = show(twoGames);
    await user.click(await screen.findByRole("button", { name: "delete wire" }));
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
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
