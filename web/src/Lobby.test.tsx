// The lobby (spec §5.2): the tables you sit at, and the tables waiting for
// somebody to sit down.
//
// **Two lists behind one path**, so every stub here goes through the harness's
// scope-aware matcher rather than the path test the rest of this suite grew up
// with — `?scope=open` walks straight through a path-only check, and a cell
// that stubbed the open list would be handed the `mine` reply and pass while
// asserting the wrong thing.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Lobby } from "./Lobby";
import { isGamesList, stubApi, type ApiCall, type Reply } from "./test/harness";
import { seatName } from "./tables";
import { wire } from "./wire/contract";

/** Both scopes, from the corpus's own two listings. */
const lists = (call: ApiCall): Reply | undefined => {
  if (isGamesList(call, "open")) return wire.gamesOpen;
  if (isGamesList(call, "mine")) return wire.games;
  return undefined;
};

function show(handler: (call: ApiCall) => Reply | undefined = () => undefined) {
  const stub = stubApi((call) => {
    const mine = handler(call);
    if (mine !== undefined) return mine;
    const l = lists(call);
    if (l !== undefined) return l;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  render(<Lobby who={<span className="meta who">You are red@wire.test</span>} />);
  return { stub, user: userEvent.setup() };
}

/** One section, by its heading. The two sections draw the same row markup, so
 *  every read below is scoped to one of them — an unscoped `getByRole` would
 *  find the corpus's `wire` row in both. */
const section = (heading: string): HTMLElement =>
  screen.getByRole("heading", { name: heading }).closest("section")!;

describe("the lobby", () => {
  it("draws both sections from the two scopes, with the same row in each", async () => {
    const { stub } = show();
    await screen.findByRole("heading", { name: "Your games" });
    // **Both scopes were really asked for.** Without this the cell passes on a
    // client that asked once and drew the same answer twice.
    await waitFor(() => expect(stub.calls.filter((c) => isGamesList(c, "open"))).toHaveLength(1));
    expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1);

    const mine = section("Your games");
    const open = section("Open tables");
    // `wire.games` is the caller's two tables; `wire.gamesOpen` is every open
    // table on the server, which here is the same two read from outside.
    expect(within(mine).getAllByRole("listitem")).toHaveLength(wire.games.games.length);
    expect(within(open).getAllByRole("listitem")).toHaveLength(wire.gamesOpen.games.length);
    expect(within(mine).getByRole("button", { name: "wire" })).toBeInTheDocument();
    expect(within(open).getByRole("button", { name: "wire2" })).toBeInTheDocument();
  });

  it("says what each table is doing, and who is at it", async () => {
    show();
    const mine = section("Your games");
    const dealing = (await within(mine).findByRole("button", { name: "wire" })).closest("li")!;
    expect(dealing).toHaveTextContent("2P · dealing");
    // §5.6, on the surface that prints it: the row reads the NAME each seat
    // gave, and the address only where there is none.
    for (const seat of wire.games.games[0]!.seats) {
      expect(dealing).toHaveTextContent(seatName(seat));
    }
    expect(dealing).toHaveTextContent("Red, Blue");
    // The open row counts the seats rather than guessing a turn.
    const filling = within(mine).getByRole("button", { name: "wire2" }).closest("li")!;
    expect(filling).toHaveTextContent("2P · open · 1 of 2 seated");
    // And the unnamed seat falls back to the address, on the same screen as the
    // named one above — so the row is reading the field and not a constant.
    expect(filling).toHaveTextContent("red@wire.test, open seat");
  });

  // **The two rows a player comes back to the lobby for**, neither of which the
  // corpus can carry: the fixture walk is bot-free and drives one side, so no
  // committed payload has the turn inside `my_seats` or a finished game with a
  // winner on it. One field moved on a real payload, which is the pattern
  // `tables.test.ts` already uses.
  it("says whose turn it is, and who won", async () => {
    const yours = { ...wire.table, id: "yours", name: null, my_seats: [wire.table.to_move!] };
    const done = { ...wire.table, id: "done", name: null, over: true, winner: 1 };
    show((call) =>
      isGamesList(call, "mine") ? { ...wire.games, games: [yours, done] } : undefined,
    );
    const mine = section("Your games");
    expect((await within(mine).findByRole("button", { name: "yours" })).closest("li")).toHaveTextContent(
      "your turn",
    );
    expect(within(mine).getByRole("button", { name: "done" }).closest("li")).toHaveTextContent(
      "over · Blue won",
    );
  });

  it("marks the table that needs a password, and only that one", async () => {
    show();
    await screen.findByRole("heading", { name: "Your games" });
    const open = section("Open tables");
    const locked = wire.gamesOpen.games[0]!;
    const unlocked = wire.gamesOpen.games[1]!;
    // Anti-vacuity: the two rows differ in exactly the field under test.
    expect(locked.locked).toBe(true);
    expect(unlocked.locked).toBe(false);
    const row = (id: string) => within(open).getByRole("button", { name: id }).closest("li")!;
    expect(
      within(row(locked.id)).getByLabelText("password required"),
    ).toBeInTheDocument();
    expect(within(row(unlocked.id)).queryByLabelText("password required")).toBeNull();
  });

  it("pages Your games with the cursor the server handed it, once", async () => {
    const second = { ...wire.table, id: "older", name: null };
    const { stub, user } = show((call) => {
      if (!isGamesList(call, "mine")) return undefined;
      // The first page carries a cursor; the page it names carries none, which
      // is what takes "more" off the screen.
      return call.path.includes("before=")
        ? { ...wire.games, games: [second], next: null }
        : wire.gamesNext;
    });
    const mine = section("Your games");
    await within(mine).findByRole("button", { name: "wire" });
    await user.click(within(mine).getByRole("button", { name: "more" }));
    await within(mine).findByRole("button", { name: "older" });

    const paged = stub.calls.filter((c) => isGamesList(c, "mine") && c.path.includes("before="));
    expect(paged).toHaveLength(1);
    // The cursor is sent BACK UNTOUCHED — it is the server's own opaque
    // `<created_at>,<id>`, and a client that parsed or rebuilt it would page
    // from somewhere else.
    expect(paged[0]!.path).toContain(`before=${encodeURIComponent(wire.gamesNext.next!)}`);
    // The first page is still there, above the one just fetched.
    expect(within(mine).getByRole("button", { name: "wire" })).toBeInTheDocument();
    // And the control is gone, because the page it fetched named no successor.
    expect(within(mine).queryByRole("button", { name: "more" })).toBeNull();
  });

  it("offers no more when the first page is the last", async () => {
    show();
    await screen.findByRole("heading", { name: "Your games" });
    expect(wire.games.next).toBeNull();
    expect(screen.queryByRole("button", { name: "more" })).toBeNull();
  });

  it("says so when you sit at no table, and when nobody is waiting", async () => {
    show((call) => (isGamesList(call) ? { ...wire.games, games: [] } : undefined));
    expect(await screen.findByText(/none yet/)).toBeInTheDocument();
    expect(screen.getByText(/no table is waiting for players/)).toBeInTheDocument();
  });

  // **The row is a LINK, and the hash is what it sets** (spec §5.1). Nothing is
  // fetched here: which screen a table opens is the route's question, and
  // `App`'s route effect is what asks the server.
  it("opens a table by setting the hash, and fetches nothing itself", async () => {
    const { stub, user } = show();
    const mine = section("Your games");
    await user.click(await within(mine).findByRole("button", { name: "wire" }));
    expect(window.location.hash).toBe("#/t/wire");
    expect(stub.calls.filter((c) => c.path.includes("/api/games/"))).toHaveLength(0);
  });

  it("sends New table to the form", async () => {
    const { user } = show();
    await user.click(screen.getByRole("button", { name: "New table" }));
    expect(window.location.hash).toBe("#/new");
  });

  it("shows a refusal rather than an empty lobby", async () => {
    show((call) =>
      isGamesList(call, "open")
        ? { status: 500, body: { error: "the store is not readable" } }
        : undefined,
    );
    expect(await screen.findByText("the store is not readable")).toBeInTheDocument();
    // **And it is not reported as "you have no games"**, which is the shape the
    // setup screen's own list used to fail in: a swallowed refusal told a
    // signed-out player they had nothing.
    expect(screen.queryByText(/none yet/)).toBeNull();
  });
});
