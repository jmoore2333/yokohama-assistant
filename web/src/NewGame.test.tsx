// Setup: the form that opens a table, and the seed that must survive the round
// trip.
//
// **A FORM, and nothing else (spec §5.3).** The deal left it at M6.1 and the
// saved-tables list left it at M6.2 — the lobby is the first screen and lists
// both your games and the open ones, so the cells about rows, facts and
// deleting are `Lobby.test.tsx`'s and `OpenTable.test.tsx`'s. What is asked
// here is what a create asks: how many seats, who holds each one (`me` /
// another email / an open seat / the bot), what the table is called, what YOU
// are called at it, whether it wants a password, and the seed.

import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NewGame } from "./NewGame";
import { DISPLAY_NAME_KEY } from "./api";
import { gate, stubApi, type ApiCall, type Reply } from "./test/harness";
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
function Host({ me = ME, onCreated = () => {} }: { me?: string; onCreated?: (t: TableView) => void }) {
  return <NewGame me={me} onCreated={onCreated} />;
}

/** The create posts, and NOT the other posts beside them.
 *
 *  `matching("POST", "/api/games")` is a substring match, so it also catches
 *  every `/api/games/{id}/…` post. The create is the only one whose path ENDS
 *  there. */
const creates = (stub: ReturnType<typeof stubApi>): ApiCall[] =>
  stub.matching("POST", "/api/games").filter((c) => c.path.split("?")[0]!.endsWith("/api/games"));

/** The form, rendered over a stub.
 *
 *  **It fetches NOTHING on mount now**: the saved list went to the lobby, so a
 *  handler here only has to answer the create. */
function show(handler: (call: ApiCall) => Reply, props: Parameters<typeof Host>[0] = {}) {
  const onCreated = vi.fn();
  const stub = stubApi(handler);
  render(<Host onCreated={onCreated} {...props} />);
  return { onCreated, stub, user: userEvent.setup() };
}

/** The opening reply: a create answers 201 with a DEALING table (§4.2). */
const created = (call: ApiCall): Reply | undefined =>
  call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games")
    ? { status: 201, body: wire.tableDealing }
    : undefined;

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
  it("at three players no bot is offered, and the other seats open", async () => {
    const stub = stubApi((call) => created(call));
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
    //
    // The seats themselves are the OPEN default since M6.2 (spec §5.3) — see
    // the `open seats` block below, which is where that is the subject.
    await user.click(screen.getByRole("button", { name: "Open table" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { seats: unknown[] }).seats).toEqual([
      { user: ME },
      { open: true },
      { open: true },
    ]);
  });

  // Two players is where the bot IS offered, so the cell above is a difference
  // and not a screen that never offers one.
  it("offers the bot at two players, which is what makes the absence above a rule", async () => {
    show(created);
    const seat1 = screen.getByLabelText("Seat 1");
    expect(within(seat1.parentElement!).queryByRole("option", { name: "bot" })).not.toBeNull();
  });

  it("a seat set to another email posts that email", async () => {
    const stub = stubApi(created);
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

  // **The hot seat is still reachable, and is now ASKED FOR rather than the
  // default** (spec §5.3). One operator driving four human seats is what this
  // client has always supported and the terminal still does; what changed is
  // which of the two a 4P form opens without being told.
  it("seats me in every seat at four players when every row says so", async () => {
    const stub = stubApi(created);
    render(<Host me={ME} />);
    const user = userEvent.setup();
    await user.selectOptions(screen.getByLabelText("Players"), "4");
    for (const i of [1, 2, 3]) {
      await user.selectOptions(screen.getByLabelText(`Seat ${i}`), "me");
    }
    // Every seat is a person's, so the button is back to promising the deal.
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
    const stub = stubApi(created);
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
    const { stub, user } = show((call) => created(call));
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
    const { stub, user } = show((call) => created(call));
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
    const { stub, user } = show((call) => created(call));
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
      return undefined;
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

// **The refusal, which is the one thing from the old saved-tables block that is
// still about this screen.** A create can fail — a name the server will not
// take, a seat it refuses — and the sentence belongs here rather than in an
// unhandled rejection.
describe("a create that is refused", () => {
  it("shows the sentence rather than swallowing it", async () => {
    const { user } = show(() => ({
      status: 400,
      body: { error: "a seat names neither a user nor the bot" },
    }));
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect(await screen.findByText(/neither a user nor the bot/)).toBeInTheDocument();
  });
});

// ---- M6.2: open seats, a display name, and a password ----------------------

describe("open seats", () => {
  // **The default above two players, and it is a product change** (spec §5.3):
  // until M6.2 every seat defaulted to the person filling the form in, which
  // made a hot seat the only 3P and 4P table this client could open.
  it("seats me, then open seats, at three and four players", async () => {
    const stub = stubApi(created);
    render(<Host me={ME} />);
    const user = userEvent.setup();
    const rows = [
      [3, [{ user: ME }, { open: true }, { open: true }]],
      [4, [{ user: ME }, { open: true }, { open: true }, { open: true }]],
    ] as const;
    for (const [n, [players, seats]] of rows.entries()) {
      await user.selectOptions(screen.getByLabelText("Players"), String(players));
      await user.click(screen.getByRole("button", { name: "Open table" }));
      // The COUNT, not "at least one": `toBeGreaterThan(0)` is already true on
      // the second pass before its own create has landed, and the read below
      // would then be the first pass's body again.
      await waitFor(() => expect(creates(stub)).toHaveLength(n + 1));
      expect((creates(stub)[n]!.body as { seats: unknown[] }).seats).toEqual(seats);
    }
  });

  // Two players still defaults to the bot, which is what makes the rows above a
  // rule about the count rather than a screen that always opens seats.
  it("still seats the bot at two, and posts {open: true} when a row is switched", async () => {
    const { stub, user } = show(created);
    await user.selectOptions(screen.getByLabelText("Seat 1"), "open");
    await user.click(screen.getByRole("button", { name: "Open table" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { seats: unknown[] }).seats).toEqual([
      { user: ME },
      { open: true },
    ]);
  });

  // **The button says which of the two things the click does.** A table with an
  // open seat is not dealt when it is created — it waits for somebody to sit
  // down — so "Deal" there would promise the set table and hand over a waiting
  // room. Both arms, because either alone passes on a constant.
  it("reads Deal with every seat filled and Open table with one open", async () => {
    const { user } = show(created);
    expect(screen.getByRole("button", { name: "Deal" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Open table" })).toBeNull();
    await user.selectOptions(screen.getByLabelText("Seat 1"), "open");
    expect(screen.getByRole("button", { name: "Open table" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Deal" })).toBeNull();
  });
});

describe("the display name", () => {
  it("is read from this browser on mount and written back on submit", async () => {
    localStorage.setItem(DISPLAY_NAME_KEY, "Jeremy");
    const { stub, user } = show(created);
    // Prefilled, so the second table does not ask again.
    expect(screen.getByLabelText("Your name")).toHaveValue("Jeremy");
    await user.clear(screen.getByLabelText("Your name"));
    await user.type(screen.getByLabelText("Your name"), "  Jem  ");
    // **Not yet.** Remembered on submit, not on every keystroke: a name half
    // typed and then abandoned is not what this browser is called.
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Jeremy");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { display_name: string }).display_name).toBe("Jem");
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Jem");
  });

  it("sends no display_name when the field is blank", async () => {
    const { stub, user } = show(created);
    expect(screen.getByLabelText("Your name")).toHaveValue("");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect("display_name" in (creates(stub)[0]!.body as object)).toBe(false);
  });
});

describe("the password", () => {
  it("is not asked for until the box is ticked", async () => {
    const { user } = show(created);
    expect(screen.queryByLabelText("Table password")).toBeNull();
    await user.click(screen.getByLabelText("Ask for a password to join"));
    expect(screen.getByLabelText("Table password")).toBeInTheDocument();
  });

  it("rides the create only when the box is ticked and something was typed", async () => {
    const { stub, user } = show(created);
    await user.click(screen.getByLabelText("Ask for a password to join"));
    await user.type(screen.getByLabelText("Table password"), "  rosebud  ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect((creates(stub)[0]!.body as { password: string }).password).toBe("rosebud");
  });

  // **A ticked box over an empty field opens an UNLOCKED table**, which is the
  // honest of the two: the alternative is a table locked behind the empty
  // string, which nobody — including the person who made it — can join.
  it("sends nothing for a ticked box with an empty field, and nothing after it is unticked", async () => {
    const { stub, user } = show(created);
    const box = screen.getByLabelText("Ask for a password to join");
    await user.click(box);
    await user.type(screen.getByLabelText("Table password"), "   ");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
    expect("password" in (creates(stub)[0]!.body as object)).toBe(false);

    // Typed, then unticked: the word is cleared with the box, so an unticked
    // form cannot carry a password the player can no longer see. The box is
    // still ticked from above, so this types into the field it left open.
    await user.clear(screen.getByLabelText("Table password"));
    await user.type(screen.getByLabelText("Table password"), "rosebud");
    expect(screen.getByLabelText("Table password")).toHaveValue("rosebud");
    await user.click(box);
    expect(screen.queryByLabelText("Table password")).toBeNull();
    await user.click(box);
    expect(screen.getByLabelText("Table password")).toHaveValue("");
  });
});
