// The lobby (spec §5.2): the tables you sit at, and the tables waiting for
// somebody to sit down.
//
// **Two lists behind one path**, so every stub here goes through the harness's
// scope-aware matcher rather than the path test the rest of this suite grew up
// with — `?scope=open` walks straight through a path-only check, and a cell
// that stubbed the open list would be handed the `mine` reply and pass while
// asserting the wrong thing.

import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Lobby } from "./Lobby";
import { DISPLAY_NAME_KEY, setLobbyChat } from "./api";
import { OPEN_KEY } from "./chatFold";
import { fakeStream, until, isGamesList, openStream, stubApi, type ApiCall, type Reply } from "./test/harness";
import { seatName } from "./tables";
import { wire } from "./wire/contract";

/** All three scopes, from the corpus's own listings.
 *
 *  **`playing` answers EMPTY by default, and that is deliberate.** The corpus's
 *  playing listing carries the same `wire` table the other two do, so answering
 *  it here would put that row in a THIRD section and every unscoped read in this
 *  file would fail on an ambiguity that has nothing to do with the cell. The
 *  section's own cell supplies the listing, which is the shape `renderApp`'s
 *  stub already uses for the open scope. */
const lists = (call: ApiCall): Reply | undefined => {
  if (isGamesList(call, "open")) return wire.gamesOpen;
  if (isGamesList(call, "playing")) return { ...wire.gamesPlaying, games: [] };
  if (isGamesList(call, "mine")) return wire.games;
  return undefined;
};

/** Who this browser is, and who opened every table the corpus carries — so the
 *  abandon control is on every row, and the cell that wants it absent moves
 *  `created_by` instead. */
const ME = "red@wire.test";

function show(handler: (call: ApiCall) => Reply | undefined = () => undefined) {
  const stub = stubApi((call) => {
    const mine = handler(call);
    if (mine !== undefined) return mine;
    const l = lists(call);
    if (l !== undefined) return l;
    // The lobby listens to the HOUSE stream (M6.3 R-3.4). A cell about that
    // scripts its own; this is the resting answer for the rest, and without it
    // every cell in this file would have the lobby's open refused and its link
    // reading "reconnecting…" while the cell asserted about something else.
    if (call.path.endsWith("/api/stream")) return openStream();
    // **The lobby draws a fourth section since M6.4**, and its panel asks for
    // the house chat on mount whatever its fold state — so every cell in this
    // file now makes that request whether or not it is about the chat. A lobby
    // where nothing has been said, derived from the corpus's own page rather
    // than typed out here (the `gamesNamed` rule), so a cell that is not about
    // the chat has none of it in its DOM. A cell that IS answers first.
    if (call.method === "GET" && call.path.startsWith("/api/chat"))
      return { ...wire.houseChat, messages: [], last: 0 };
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  render(
    <Lobby me={ME} who={<span className="meta who">You are {ME}</span>} />,
  );
  return { stub, user: userEvent.setup() };
}

/** One section, by its heading. The two sections draw the same row markup, so
 *  every read below is scoped to one of them — an unscoped `getByRole` would
 *  find the corpus's `wire` row in both. */
const section = (heading: string): HTMLElement =>
  screen.getByRole("heading", { name: heading }).closest("section")!;

// **"On this device" left this screen at M6.5b2** (R-6.5b2.1): single player is
// a channel of its own at `#/` and the lobby is the SERVER's screen at
// `#/lobby`. Nothing rendered here talks to the local Worker any more, so this
// suite stubs none — `FrontDoor.test.tsx` and `OnThisDevice.test.tsx` are where
// that section is read.

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
    expect(within(mine).getByRole("link", { name: "wire" })).toBeInTheDocument();
    expect(within(open).getByRole("link", { name: "wire2" })).toBeInTheDocument();
  });

  // **The third section (R-3.9): a game you can WATCH, found without a link.**
  // The rows are the other two sections' — `tableWord` says "waiting on <name>"
  // or names the winner, the lock marker says a password will be wanted — and
  // the one thing this section must NOT carry is the Abandon control, which is
  // the creator's and belongs to Your games alone (M6.2 ruling #13).
  it("lists the games in progress, with no abandon control on them", async () => {
    const { stub } = show((call) =>
      isGamesList(call, "playing") ? wire.gamesPlaying : undefined,
    );
    const playing = section("Games in progress");
    const row = (await within(playing).findByRole("link", { name: "wire" })).closest("li")!;
    // The scope was really asked for, alongside the other two.
    expect(stub.calls.filter((c) => isGamesList(c, "playing"))).toHaveLength(1);
    expect(within(playing).getAllByRole("listitem")).toHaveLength(
      wire.gamesPlaying.games.length,
    );
    // The row is a LINK to the table, which is what makes it openable in a new
    // tab and readable as a destination.
    expect(within(playing).getByRole("link", { name: "wire" })).toHaveAttribute(
      "href",
      "#/t/wire",
    );
    expect(row).toHaveTextContent("2P ·");
    // The corpus's playing table is locked, so the mark a watcher needs before
    // they click is on the row.
    expect(wire.gamesPlaying.games[0]!.locked).toBe(true);
    expect(within(row).getByLabelText("password required")).toBeInTheDocument();
    // **Not yours to abandon.** `created_by` on the corpus row is this browser's
    // own address, so the control would be drawn by a `created_by === me` test
    // alone — the section is what withholds it.
    expect(wire.gamesPlaying.games[0]!.created_by).toBe(ME);
    expect(within(playing).queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  it("says what each table is doing, and who is at it", async () => {
    show();
    const mine = section("Your games");
    const dealing = (await within(mine).findByRole("link", { name: "wire" })).closest("li")!;
    expect(dealing).toHaveTextContent("2P · dealing");
    // §5.6, on the surface that prints it: the row reads the NAME each seat
    // gave, and the address only where there is none.
    for (const seat of wire.games.games[0]!.seats) {
      expect(dealing).toHaveTextContent(seatName(seat));
    }
    expect(dealing).toHaveTextContent("Red, Blue");
    // The open row counts the seats rather than guessing a turn.
    const filling = within(mine).getByRole("link", { name: "wire2" }).closest("li")!;
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
    expect((await within(mine).findByRole("link", { name: "yours" })).closest("li")).toHaveTextContent(
      "your turn",
    );
    expect(within(mine).getByRole("link", { name: "done" }).closest("li")).toHaveTextContent(
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
    // **The LISTING, awaited before any row is read.** The heading above is
    // drawn before the three scopes land — it is static, and "loading…" sits
    // under it until they do — so the synchronous reads below raced the
    // listing and lost under load (the whole-branch review's L-12).
    await within(open).findByRole("link", { name: locked.id });
    const row = (id: string) => within(open).getByRole("link", { name: id }).closest("li")!;
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
    await within(mine).findByRole("link", { name: "wire" });
    await user.click(within(mine).getByRole("button", { name: "more" }));
    await within(mine).findByRole("link", { name: "older" });

    const paged = stub.calls.filter((c) => isGamesList(c, "mine") && c.path.includes("before="));
    expect(paged).toHaveLength(1);
    // The cursor is sent BACK UNTOUCHED — it is the server's own opaque
    // `<created_at>,<id>`, and a client that parsed or rebuilt it would page
    // from somewhere else.
    expect(paged[0]!.path).toContain(`before=${encodeURIComponent(wire.gamesNext.next!)}`);
    // The first page is still there, above the one just fetched.
    expect(within(mine).getByRole("link", { name: "wire" })).toBeInTheDocument();
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

  // **The row is a LINK, and the hash is what it sets** (spec §5.1, R-14).
  // Nothing is fetched here: which screen a table opens is the route's
  // question, and `App`'s route effect is what asks the server.
  it("opens a table by setting the hash, and fetches nothing itself", async () => {
    const { stub, user } = show();
    const mine = section("Your games");
    const link = await within(mine).findByRole("link", { name: "wire" });
    // **The address is in the markup, not only in a handler.** That is what
    // makes the row openable in a new tab and copyable from a context menu —
    // and it is the half a click assertion alone cannot see, because a button
    // calling `navigate` reaches the same hash.
    expect(link).toHaveAttribute("href", "#/t/wire");
    await user.click(link);
    expect(window.location.hash).toBe("#/t/wire");
    expect(stub.calls.filter((c) => c.path.includes("/api/games/"))).toHaveLength(0);
  });

  // ---- abandoning a table, from the row (R-13) ------------------------------
  //
  // **Re-homed, not new.** This control lived beside each saved game on the
  // setup screen until the list moved here; M5's own final review is why it
  // asks twice — the record is gone and there is no undo, and a stray click
  // beside the table you meant to open would take a different one. The open
  // table's own screen carries the same control for the table you are looking
  // at; this is the only way to reach a DEALING or PLAYING one.
  describe("abandoning a table", () => {
    // **What is the LOBBY's here.** The two clicks, the in-flight guard and the
    // cancel belong to the control and are tested once, in `Abandon.test.tsx`.
    // These three are the row's own wiring: which row gets one, which table it
    // deletes, and what the lobby does afterwards.
    it("abandons the table the row names, and re-reads the lists", async () => {
      const { stub, user } = show((call) => (call.method === "DELETE" ? null : undefined));
      const mine = section("Your games");
      await user.click(await within(mine).findByRole("button", { name: "abandon wire2" }));
      await user.click(within(mine).getByRole("button", { name: "confirm abandon wire2" }));
      // **Matched on the WHOLE path.** `matching` is a substring test and
      // `/api/games/wire` is a prefix of `/api/games/wire2`, so the two rows
      // here cannot be told apart by it — which is exactly the confusion this
      // cell exists to rule out.
      const deleted = () =>
        stub.calls.filter((c) => c.method === "DELETE").map((c) => c.path);
      await waitFor(() => expect(deleted()).toEqual(["/api/games/wire2"]));
      // And the lists are re-read, so the screen agrees with the store.
      await waitFor(() =>
        expect(stub.calls.filter((c) => isGamesList(c, "mine")).length).toBeGreaterThan(1),
      );
    });

    // **Your games only** (R-13, and the reason the row takes a flag). A table
    // that is both yours and open is listed in BOTH sections, and a control
    // drawn in each puts two buttons with the same accessible name on one
    // screen — which is also an ambiguity every query here would have to work
    // around.
    it("draws the control in Your games and not in Open tables", async () => {
      show();
      const mine = section("Your games");
      const open = section("Open tables");
      // Anti-vacuity: the SAME table is in both sections and was opened by this
      // browser, so the two reads below differ only in which section they are.
      expect(await within(mine).findByRole("button", { name: "abandon wire" })).toBeInTheDocument();
      expect(within(open).getByRole("link", { name: "wire" })).toBeInTheDocument();
      expect(within(open).queryByRole("button", { name: /abandon/i })).toBeNull();
    });

    it("offers nothing to abandon at a table this browser did not open", async () => {
      const theirs = { ...wire.games.games[0]!, created_by: "someone@else.test" };
      show((call) =>
        isGamesList(call, "mine") ? { ...wire.games, games: [theirs] } : undefined,
      );
      const mine = section("Your games");
      // The row is there — this caller sits at the table — and the control is
      // not, which is the whole claim.
      expect(await within(mine).findByRole("link", { name: theirs.name! })).toBeInTheDocument();
      expect(within(mine).queryByRole("button", { name: /abandon/ })).toBeNull();
    });
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
// ---- M6.4: the lobby chat and the name beside the address -----------------

describe("the lobby's chat and its name field", () => {
  // **A fourth section, and ABSENT rather than empty when the deployment has
  // the house chat off (R-6.4.6).** The switch is a launch variable read once
  // from `/api/health`, so what the client owes it is not to draw a panel whose
  // every request would be a 404 — and the wrapper carries the section's own
  // class while the panel keeps its own root, which is what lets the lobby
  // style a section it did not write.
  it("draws the lobby chat as a fourth section, folded, and not at all when the switch is off", async () => {
    setLobbyChat(true);
    show((call) =>
      call.method === "GET" && call.path.startsWith("/api/chat") ? wire.houseChat : undefined,
    );
    // The heading by a PREFIX, because its accessible name is computed from
    // everything inside it — the fold control lives in the header, so the exact
    // name is "Lobby chat show" and an exact match would find nothing.
    const heading = await screen.findByRole("heading", { name: /^Lobby chat/ });
    // The panel's own root, INSIDE the lobby's section: two elements, because
    // the panel is the same component the game screen draws and the section is
    // this screen's.
    const panel = heading.closest("section")!;
    expect(panel).toHaveClass("chat");
    expect(panel).toHaveClass("folded");
    expect(panel.closest("section.saved.lobby-chat"), "wrapped by the lobby's own section").not.toBeNull();
    expect(within(panel).getByRole("button", { name: "show" })).toBeInTheDocument();
    cleanup();

    setLobbyChat(false);
    try {
      show();
      await screen.findByRole("heading", { name: "Lobby" });
      expect(screen.queryByRole("heading", { name: /^Lobby chat/ })).toBeNull();
      expect(document.querySelector("section.chat")).toBeNull();
    } finally {
      setLobbyChat(true);
    }
  });

  // **The name is the LOBBY's, and it is one key (R-6.4.3).** The New table
  // form and the open table's seat picker already read it; this is where a
  // person who has not opened either can set it, and it is written as typed
  // rather than on a submit this screen does not have. An emptied field REMOVES
  // the key, so "leave it blank and your address is used" is true of the
  // browser as well as of the sentence.
  it("offers Your name beside the address and remembers it as typed", async () => {
    const { user } = show();
    const field = await screen.findByLabelText("Your name");
    await user.type(field, "Captain Red");
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Captain Red");
    await user.clear(field);
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
  });

  // **The house frame is a WAKE, and the panel's tick is this screen's own**
  // (spec §6.2). The house channel coalesces and carries no table, so the lobby
  // refetches its lists on any frame; the chat is told to ask for its delta the
  // same way, off the same wake, rather than being handed a count it cannot
  // read from a frame that names no table.
  it("asks the house chat for its delta on a house frame", async () => {
    setLobbyChat(true);
    // Remembered OPEN, because a folded count-driven panel fetches nothing —
    // but a wake-driven one asks whatever its fold state, and this cell would
    // pass for the wrong reason if the fold were the thing being read.
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    const stream = fakeStream();
    vi.useFakeTimers();
    try {
      const { stub } = show((call) => {
        if (call.path.endsWith("/api/stream")) return stream.reply(call);
        if (call.method === "GET" && call.path.startsWith("/api/chat")) return wire.houseChat;
        return undefined;
      });
      await until(() => stream.opens() > 0, "the house stream is open");
      const before = stub.matching("GET", "/api/chat").length;
      stream.push("house", { seq: 1, chat: 1 });
      await until(
        () => stub.matching("GET", "/api/chat").length > before,
        "the chat delta was asked for",
      );
    } finally {
      vi.useRealTimers();
    }
  });
});

// ---- the server AREA (M6.5b, spec §2.5; M6.5b2, R-6.5b2.1) -----------------
//
// What used to be four gates in front of the whole page — the door's error, its
// silence, the sign-in form and the door sentence — is the content of the part
// of the screen the server owns. The games in THIS BROWSER are no longer stacked
// above it: they are the other channel now, at `#/`, and what this screen owes a
// player is one link back to them — drawn OUTSIDE the three states, because the
// two states a player most needs it in are the two where nothing else on this
// screen is pressable.
describe("the server area", () => {
  it("carries one way back to the door, above the lists", async () => {
    show();
    const back = await screen.findByRole("link", { name: "Play on this device" });
    // An ADDRESS and not a button, for `lobbyRow`'s reason: the control's whole
    // job is to go to `#/`, so the browser does the navigating.
    expect(back).toHaveAttribute("href", "#/");
    const lists = await screen.findByRole("heading", { name: "Your games" });
    // Order, and not merely presence: a way out that sits below two listings is
    // a way out a player on a phone scrolls past.
    expect(back.compareDocumentPosition(lists) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it("renders the door's content in place of the lists, and asks the server for nothing", async () => {
    const stub = stubApi(() => {
      throw new Error("a signed-out lobby asked the server for something");
    });
    render(<Lobby me={null} door={<p>the door says so</p>} />);
    expect(await screen.findByText("the door says so")).toBeInTheDocument();
    // **And the way back is drawn in THIS state too.** A player who is not
    // signed in has nothing else on this screen to press, and the games in
    // their own browser need no door at all.
    expect(screen.getByRole("link", { name: "Play on this device" })).toBeInTheDocument();
    // **Every `/api/games` route is a 401 without an identity** (M6.1), so a
    // lobby that listed first would show a refusal as the player's first
    // screen — the reason the door was a gate in the first place, kept.
    expect(screen.queryByRole("heading", { name: "Your games" })).toBeNull();
    expect(screen.queryByRole("button", { name: "New table" })).toBeNull();
    expect(stub.calls).toHaveLength(0);
  });

  it("says not connected in place of the four sections, and goes on reading", async () => {
    const stub = stubApi((call) => {
      const l = lists(call);
      if (l !== undefined) return l;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    render(<Lobby me={ME} serverUp={false} />);
    expect(await screen.findByText("not connected")).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Your games" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Open tables" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Games in progress" })).toBeNull();
    expect(screen.queryByRole("button", { name: "New table" })).toBeNull();
    // **The read is still going out.** The line says what is true of the last
    // health read, not that this screen has given up: the sections come back
    // when a read lands.
    await waitFor(() => expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1));
  });

  it("does not re-read because its host re-rendered with a new callback", async () => {
    // **The trap this prop walks into.** `take` is what the opening read and
    // the poll both call, and a `useCallback` that closed over the prop would
    // get a new identity every time `App` re-rendered — which re-runs the
    // opening read's effect, one `fetchLists` per render of a screen that has
    // not changed. The host builds the callback inline, as hosts do.
    const stub = stubApi((call) => {
      const l = lists(call);
      if (l !== undefined) return l;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const { rerender } = render(<Lobby me={ME} onReached={() => {}} />);
    await waitFor(() => expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1));
    for (let i = 0; i < 3; i += 1) rerender(<Lobby me={ME} onReached={() => {}} />);
    await waitFor(() => expect(screen.getByRole("heading", { name: "Your games" })).toBeInTheDocument());
    expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1);
  });

  it("tells the app when a read lands, which is what brings the sections back", async () => {
    const onReached = vi.fn();
    stubApi((call) => {
      const l = lists(call);
      if (l !== undefined) return l;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    render(<Lobby me={ME} serverUp={false} onReached={onReached} />);
    await waitFor(() => expect(onReached).toHaveBeenCalled());
  });
});
