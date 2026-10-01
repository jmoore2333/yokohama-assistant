// The lobby (spec §5.2): the tables you sit at, and the tables waiting for
// somebody to sit down.
//
// **Two lists behind one path**, so every stub here goes through the harness's
// scope-aware matcher rather than the path test the rest of this suite grew up
// with — `?scope=open` walks straight through a path-only check, and a cell
// that stubbed the open list would be handed the `mine` reply and pass while
// asserting the wrong thing.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Lobby } from "./Lobby";
import { setLobbyChat, setSignalKind } from "./api";
import { BACKOFF_MS } from "./signal";
import { FakeSocket } from "./test/fakeSocket";
import { OPEN_KEY } from "./chatFold";
import { fakeStream, flush, gate, until, isGamesList, openStream, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import productionBot from "./wire/production_bot.json";
import type { ChatPage, TableView } from "./types";

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

/** Who this browser is — the door's answer, which the band's corner draws.
 *  Since M6.5c2 cycle D (R-6.5c2.35) no row control compares it with anything:
 *  the creator's Abandon follows each row's `mine`, as the server said it for
 *  that row's reader, and the corpus's `mine` rows are the walk's creator's
 *  readings (`wire.games`, `wire.tableForeign`). */
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
    // the chat has none of it in its DOM. A cell that IS answers first. Its
    // `rev` is moved to 0 with its `last`: a room nobody has spoken in has
    // moved neither, and the panel — a count host here since M6.5c2 cycle D —
    // reads a `rev` ahead of `last` as a delete.
    if (call.method === "GET" && call.path.startsWith("/api/chat"))
      return { ...wire.houseChat, messages: [], last: 0, rev: 0 };
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  render(<Lobby me={ME} corner={<span className="identity-corner">{ME}</span>} />);
  return { stub, user: userEvent.setup() };
}

/** One section, by its heading. The three sections draw the same row markup,
 *  and the corpus's open and playing listings were read by ANOTHER caller
 *  (`my_seats: []`), so `notMine` keeps their `wire` row while the caller's own
 *  listing carries a `wire` too. Every read below is therefore scoped to one
 *  section: an unscoped `getByRole` would find two. (On a server, a table the
 *  caller sits at is listed under Your tables only.) */
const section = (heading: string): HTMLElement =>
  screen.getByRole("heading", { name: heading }).closest("section")!;

// **"On this device" left this screen at M6.5b2** (R-6.5b2.1): single player is
// a channel of its own at `#/` and the lobby is the SERVER's screen at
// `#/lobby`. Nothing rendered here talks to the local Worker any more, so this
// suite stubs none — `FrontDoor.test.tsx` and `OnThisDevice.test.tsx` are where
// that section is read.

describe("the lobby", () => {
  it("draws Your tables and Open tables from their own two scopes", async () => {
    const { stub } = show();
    await screen.findByRole("heading", { name: "Your tables" });
    // **Both scopes were really asked for.** Without this the cell passes on a
    // client that asked once and drew the same answer twice.
    await waitFor(() => expect(stub.calls.filter((c) => isGamesList(c, "open"))).toHaveLength(1));
    expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1);

    const mine = section("Your tables");
    const open = section("Open tables");
    // `wire.games` is the caller's two tables; `wire.gamesOpen` is every open
    // table on the server, here the same two read from OUTSIDE (`my_seats:
    // []`), which is why `notMine` keeps them in Open tables.
    expect(within(mine).getAllByRole("listitem")).toHaveLength(wire.games.games.length);
    expect(within(open).getAllByRole("listitem")).toHaveLength(wire.gamesOpen.games.length);
    expect(within(mine).getByRole("link", { name: "wire" })).toBeInTheDocument();
    expect(within(open).getByRole("link", { name: "wire2" })).toBeInTheDocument();
  });

  // **The third section (R-3.9): a game you can SPECTATE, found without a
  // link.** The rows are the other two sections' (the chips say who is moving,
  // the password tag says a password will be wanted) and the action is
  // Spectate; the one thing this section must NOT carry is the Abandon control,
  // which is the creator's and belongs to Your tables alone (M6.2 ruling #13).
  it("lists the games in progress, with no abandon control on them", async () => {
    const { stub } = show((call) =>
      isGamesList(call, "playing") ? wire.gamesPlaying : undefined,
    );
    const playing = section("Being played");
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
    expect(row).toHaveTextContent("2 players");
    expect(within(row).getByRole("button", { name: "Spectate: wire" })).toBeInTheDocument();
    // The corpus's playing table is locked, so the mark a watcher needs before
    // they click is on the row.
    expect(wire.gamesPlaying.games[0]!.locked).toBe(true);
    expect(within(row).getByLabelText("password required")).toBeInTheDocument();
    // **Not yours to abandon.** The row is a watcher's reading, `mine: false`
    // (R-6.5c2.35) — and it is under Being played, not Your tables. The cell
    // that tells the SECTION rule from the `mine` rule is "draws the control
    // in Your tables and not in Open tables", below.
    expect(wire.gamesPlaying.games[0]!.mine).toBe(false);
    expect(within(playing).queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  // **A row is its size and what matters about it, then a chip per seat**
  // (mock 02). The words are `tables.ts`'s (`rowMeta`, `seatChips`), so the row
  // and the cells that read it have one source.
  it("says what each table is doing and who is at it: its size, then a chip per seat", async () => {
    show();
    const chips = (li: Element) => [...li.querySelectorAll(".seat-chip")].map((c) => c.textContent);
    const mine = section("Your tables");
    const dealing = (await within(mine).findByRole("link", { name: "wire" })).closest("li")!;
    expect(dealing.querySelector(".row-meta")).toHaveTextContent("2 players \u00b7 dealing");
    // The caller's own seat is "you", the other the name it joined under.
    expect(chips(dealing)).toEqual(["you", "Blue"]);
    const filling = within(mine).getByRole("link", { name: "wire2" }).closest("li")!;
    expect(filling.querySelector(".row-meta")).toHaveTextContent("2 players \u00b7 waiting for 1 more");
    expect(chips(filling)).toEqual(["you", "open seat"]);
    // Read from OUTSIDE (the open list is a stranger's view), the unnamed seat
    // reads "invited" — never the address it is keyed by, which the server no
    // longer sends a stranger (M6.5c2 cycle D, R-6.5c2.35).
    const stranger = within(section("Open tables")).getByRole("link", { name: "wire2" }).closest("li")!;
    expect(chips(stranger)).toEqual(["invited", "open seat"]);
    expect(stranger.textContent).not.toContain("@");
    expect(stranger.querySelector(".row-meta")!.textContent).toMatch(/^2 players \u00b7 opened /);
  });

  // **The two rows a player comes back to the lobby for**, neither of which the
  // corpus can carry: the fixture walk is bot-free and drives one side, so no
  // committed payload has the turn inside `my_seats` or a finished game with a
  // winner on it. `yours` moves one field, `my_seats`; `done` moves two,
  // `over` and `winner`. Both also move `id` and `name`, only so the two rows
  // are two tables named apart.
  it("marks whose turn it is in gold, and says who won", async () => {
    const yours = { ...wire.table, id: "yours", name: null, my_seats: [wire.table.to_move!] };
    const done = { ...wire.table, id: "done", name: null, over: true, winner: 1 };
    show((call) =>
      isGamesList(call, "mine") ? { ...wire.games, games: [yours, done] } : undefined,
    );
    const mine = section("Your tables");
    const turn = (await within(mine).findByRole("link", { name: "yours" })).closest("li")!;
    expect(turn.querySelector(".seat-chip.to-move")).toHaveTextContent("you \u00b7 your move");
    expect(within(turn).getByRole("button", { name: "Play: yours" })).toHaveClass("primary");
    const over = within(mine).getByRole("link", { name: "done" }).closest("li")!;
    expect(over.querySelector(".row-meta")).toHaveTextContent("2 players \u00b7 over \u00b7 Blue won");
    expect(over.querySelector(".seat-chip.to-move")).toBeNull();
    expect(within(over).getByRole("button", { name: "View: done" })).toBeInTheDocument();
  });

  it("marks the table that needs a password, and only that one", async () => {
    show();
    await screen.findByRole("heading", { name: "Your tables" });
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

  it("pages Your tables with the cursor the server handed it, once", async () => {
    const second = { ...wire.table, id: "older", name: null };
    const { stub, user } = show((call) => {
      if (!isGamesList(call, "mine")) return undefined;
      // The first page carries a cursor; the page it names carries none, which
      // is what takes "more" off the screen.
      return call.path.includes("before=")
        ? { ...wire.games, games: [second], next: null }
        : wire.gamesNext;
    });
    const mine = section("Your tables");
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
    await screen.findByRole("heading", { name: "Your tables" });
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
    const mine = section("Your tables");
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

  it("offers one action per row: Play and Copy link for yours, Take a seat and Spectate for the others'", async () => {
    const { stub, user } = show((call) => (isGamesList(call, "playing") ? wire.gamesPlaying : undefined));
    const mine = section("Your tables");
    const dealing = (await within(mine).findByRole("link", { name: "wire" })).closest("li")!;
    // `wire` is dealing and this caller's card is already chosen: Play, and not gold.
    expect(wire.games.games[0]!.chosen[0]).toBe(true);
    const play = within(dealing).getByRole("button", { name: "Play: wire" });
    expect(play).not.toHaveClass("primary");
    const filling = within(mine).getByRole("link", { name: "wire2" }).closest("li")!;
    expect(within(filling).getByRole("button", { name: "Copy link: wire2" })).toBeInTheDocument();
    expect(within(section("Open tables")).getByRole("button", { name: "Take a seat: wire2" })).toBeInTheDocument();
    expect(within(section("Being played")).getByRole("button", { name: "Spectate: wire" })).toBeInTheDocument();
    // ONE action per row: every row's action cell holds one of them.
    for (const li of document.querySelectorAll(".lobby-row")) {
      expect(li.querySelectorAll(".row-action .row-go"), li.textContent ?? "").toHaveLength(1);
    }
    // The action goes where the name goes, and fetches nothing itself.
    await user.click(play);
    expect(window.location.hash).toBe("#/t/wire");
    expect(stub.calls.filter((c) => c.path.includes("/api/games/"))).toHaveLength(0);
    // **One LINK per row**, the name, so a walk that counts the links to a
    // table (`exit_game.spec.ts`) counts one.
    for (const li of document.querySelectorAll(".lobby-row")) {
      expect(li.querySelectorAll("a"), li.textContent ?? "").toHaveLength(1);
      expect(li.querySelector("a")).toHaveClass("row-link");
    }
  });

  it("draws a gold edge on your own tables, and on no other row", async () => {
    show((call) => (isGamesList(call, "playing") ? wire.gamesPlaying : undefined));
    const mine = section("Your tables");
    await within(mine).findByRole("link", { name: "wire" });
    for (const li of within(mine).getAllByRole("listitem")) expect(li).toHaveClass("mine");
    for (const other of [section("Open tables"), section("Being played")]) {
      const rows = within(other).getAllByRole("listitem");
      expect(rows.length).toBeGreaterThan(0);
      for (const li of rows) expect(li).not.toHaveClass("mine");
    }
  });

  // **The password flag** (R-6.5c2.7): the tag after the name on every locked
  // row, and the padlock on the action only where a password is still wanted.
  // `wire.tableWatchable` is a locked game this caller has not unlocked.
  it("puts the padlock on the action only where a password is still wanted", async () => {
    show((call) =>
      isGamesList(call, "playing") ? { ...wire.gamesPlaying, games: [wire.tableWatchable] } : undefined,
    );
    const open = section("Open tables");
    const locked = (await within(open).findByRole("link", { name: "wire" })).closest("li")!;
    expect(within(locked).getByRole("button", { name: "Take a seat: wire" }).querySelector("svg.padlock")).not.toBeNull();
    const unlocked = within(open).getByRole("link", { name: "wire2" }).closest("li")!;
    expect(within(unlocked).getByRole("button", { name: "Take a seat: wire2" }).querySelector("svg.padlock")).toBeNull();
    expect(within(unlocked).queryByLabelText("password required")).toBeNull();
    expect(
      within(section("Being played")).getByRole("button", { name: "Spectate: wire" }).querySelector("svg.padlock"),
    ).not.toBeNull();
    // Yours asks you for nothing: the tag, and no padlock on Play.
    const yours = within(section("Your tables")).getByRole("link", { name: "wire" }).closest("li")!;
    expect(within(yours).getByLabelText("password required")).toBeInTheDocument();
    expect(within(yours).getByRole("button", { name: "Play: wire" }).querySelector("svg.padlock")).toBeNull();
  });

  // **The bot's version** (R-6.5c2.10, mock 10), from the row's own
  // `TableView.bot`: the corpus's undealt vs-bot table (`table_bot.json`,
  // which shows the production bot) and its foreign one (`table_foreign.json`,
  // a bot this build does not run). No field moved: the foreign row is its
  // CREATOR's own reading (`mine: true`, R-6.5c2.35), so its creator's Abandon
  // has a row to be on.
  it("names the bot and its version, and says so where this server cannot continue a game", async () => {
    expect(wire.tableBot.bot).toEqual({ label: productionBot.label, runs_here: true });
    expect(wire.tableForeign.bot!.runs_here).toBe(false);
    const foreign = wire.tableForeign;
    expect(foreign.mine).toBe(true);
    show((call) =>
      isGamesList(call, "mine") ? { ...wire.games, games: [wire.tableBot, foreign] } : undefined,
    );
    const mine = section("Your tables");
    await within(mine).findByRole("link", { name: wire.tableBot.id });
    const row = (name: string) => within(mine).getByRole("link", { name }).closest("li")!;
    expect(within(row(wire.tableBot.id)).getByText(`the bot \u00b7 ${productionBot.label}`)).toHaveClass("seat-chip");
    // Not over, and its bot is one this build does not run: nothing to open,
    // and its creator can still abandon it.
    expect(row(foreign.id)).toHaveTextContent("can\u2019t continue here");
    expect(row(foreign.id).querySelector("span.stranded")).not.toBeNull();
    expect(within(row(foreign.id)).queryByRole("button", { name: /^(Play|View):/ })).toBeNull();
    expect(within(row(foreign.id)).getByRole("button", { name: `abandon ${foreign.id}` })).toBeInTheDocument();
    cleanup();
    // Finished, it opens as ever (spec section 2.4). One field moved, `over`.
    show((call) =>
      isGamesList(call, "mine") ? { ...wire.games, games: [{ ...wire.tableForeign, over: true }] } : undefined,
    );
    const done = (await within(section("Your tables")).findByRole("link", { name: foreign.id })).closest("li")!;
    expect(within(done).getByRole("button", { name: `View: ${foreign.id}` })).toBeInTheDocument();
    expect(done).not.toHaveTextContent("can\u2019t continue here");
  });

  it("copies a table's address, and says it did", async () => {
    const { user } = show();
    const mine = section("Your tables");
    await user.click(await within(mine).findByRole("button", { name: "Copy link: wire2" }));
    // `userEvent.setup()` installs the clipboard this reads back.
    expect(await navigator.clipboard.readText()).toBe(new URL("#/t/wire2", window.location.href).href);
    expect(within(mine).getByRole("button", { name: "Copied: wire2" })).toBeInTheDocument();
    // Heard as well as seen: the button is a polite live region, so the word's
    // change is announced (the whole-branch review's triage, Task 5 M7).
    expect(within(mine).getByRole("button", { name: "Copied: wire2" })).toHaveAttribute("aria-live", "polite");
  });

  // **A copy that lands takes the refusal away** (M6.5c2 cycle C; the Opus
  // re-review's one Minor, on both hosts of a Copy link): a clipboard refused
  // and then granted within one visit left the refusal standing in the strip
  // beside a row that says "Copied".
  it("clears the refusal when a later copy lands, and says Copied", async () => {
    const { user } = show();
    const mine = section("Your tables");
    const url = new URL("#/t/wire2", window.location.href).href;
    const refused = `the link could not be copied \u2014 it is ${url}`;
    const write = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockRejectedValueOnce(new DOMException("Write permission denied.", "NotAllowedError"));
    await user.click(await within(mine).findByRole("button", { name: "Copy link: wire2" }));
    expect(await screen.findByText(refused), "the premise: the first copy was refused").toBeInTheDocument();
    await user.click(within(mine).getByRole("button", { name: "Copy link: wire2" }));
    expect(await within(mine).findByRole("button", { name: "Copied: wire2" })).toBeInTheDocument();
    expect(write).toHaveBeenCalledTimes(2);
    expect(screen.queryByText(refused), "the refusal is gone once a copy landed").toBeNull();
  });

  // **A lobby that goes away leaves no "Copied" timer to fire** (the
  // whole-branch review's Minor 11). The two-second timer is found by its
  // delay among every timer the page set, and the unmount must clear it.
  it("clears the two-second Copied timer when the lobby goes away", async () => {
    const set = vi.spyOn(globalThis, "setTimeout");
    const cleared = vi.spyOn(globalThis, "clearTimeout");
    try {
      const { user } = show();
      await user.click(await within(section("Your tables")).findByRole("button", { name: "Copy link: wire2" }));
      await within(section("Your tables")).findByRole("button", { name: "Copied: wire2" });
      const twoSeconds = set.mock.calls.flatMap((c, i) => (c[1] === 2000 ? [set.mock.results[i]!.value] : []));
      expect(twoSeconds, "the premise: one two-second timer was set").toHaveLength(1);
      cleanup();
      expect(cleared).toHaveBeenCalledWith(twoSeconds[0]);
    } finally {
      set.mockRestore();
      cleared.mockRestore();
    }
  });

  // **A table you sit at is YOURS**, listed under Your tables and not again
  // under Open tables, whose rows are for a table you could take a seat at
  // (mock 02). `wire.games.games[1]` is this caller's open table read AS this
  // caller: `my_seats` is [0].
  it("lists a table you sit at under Your tables only", async () => {
    const yoursOpen = wire.games.games[1]!;
    expect(yoursOpen.my_seats).toEqual([0]);
    show((call) => (isGamesList(call, "open") ? { ...wire.gamesOpen, games: [yoursOpen] } : undefined));
    await within(section("Your tables")).findByRole("link", { name: yoursOpen.name! });
    expect(within(section("Open tables")).queryByRole("link", { name: yoursOpen.name! })).toBeNull();
    expect(screen.getByText(/no table is waiting for players/)).toBeInTheDocument();
  });

  // **The headings at once, the rows when they land.** This is the lobby's own
  // wait, not the door's state D: the door has answered by the time the lists
  // are asked for. Until the three scopes land, Your tables reads "loading..."
  // over skeleton rows, and never "none yet", which would be a claim.
  it("draws skeleton rows under Your tables until the lists land, and never an empty list", async () => {
    const held = gate<Reply>();
    show((call) => (isGamesList(call, "mine") ? held.promise : undefined));
    const mine = section("Your tables");
    expect(within(mine).getByText("loading\u2026")).toBeInTheDocument();
    expect(mine.querySelector(".skeleton-rows")).not.toBeNull();
    expect(within(mine).queryByText(/none yet/)).toBeNull();
    held.resolve(wire.games);
    expect(await within(mine).findByRole("link", { name: "wire" })).toBeInTheDocument();
    expect(mine.querySelector(".skeleton-rows")).toBeNull();
    expect(within(mine).queryByText("loading\u2026")).toBeNull();
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
      const mine = section("Your tables");
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

    // **The creator's, SEATED — the layer's rule** (R-13; `mayAbandon`, as
    // ruled after the Task 11 review's I-1). The layer's delete refuses a
    // caller who holds no seat before it asks who created the table —
    // `delete_locked`: "you are not seated at this table" — so a creator's
    // SEATLESS reading carries no control, and a seated one does. A table you
    // sit at is listed under Your tables alone (`notMine`), so on a server the
    // control is drawn there and nowhere else. The corpus's open listing was
    // read by a caller who did not open the table (`mine: false`), so ONE field
    // is moved on its rows, `mine`, which puts the same table, `mine: true`, in
    // both sections — seated under Your tables, seatless under Open tables: the
    // one case that tells the seat rule from a `mine` rule. And the control
    // sits in its own cell under the action (`.row-abandon`), never in the
    // action's.
    it("draws the control for the seated creator in Your tables, and none for a seatless creator's reading in Open tables", async () => {
      const opened = { ...wire.gamesOpen, games: wire.gamesOpen.games.map((g) => ({ ...g, mine: true })) };
      expect(opened.games[0]!.my_seats, "the premise: a seatless reading").toHaveLength(0);
      show((call) => (isGamesList(call, "open") ? opened : undefined));
      const mine = section("Your tables");
      const open = section("Open tables");
      const control = await within(mine).findByRole("button", { name: "abandon wire" });
      expect(control.closest(".row-abandon")).not.toBeNull();
      expect(control.closest(".row-action")).toBeNull();
      expect(within(open).getByRole("link", { name: "wire" })).toBeInTheDocument();
      expect(within(open).queryByRole("button", { name: /abandon/i })).toBeNull();
    });

    // Read, not moved: the corpus's playing table as its SECOND seat reads it
    // — seated (`my_seats: [1]`), so listed under Your tables, and not its
    // creator (`mine: false`).
    it("offers nothing to abandon at a table this browser did not open", async () => {
      const theirs = wire.table;
      expect([theirs.mine, theirs.my_seats.length > 0]).toEqual([false, true]);
      show((call) =>
        isGamesList(call, "mine") ? { ...wire.games, games: [theirs] } : undefined,
      );
      const mine = section("Your tables");
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

  // **One scope refused leaves the other two standing** (dossier row 4). The
  // three listings were ONE `Promise.all`, so a refusal of any one blanked all
  // three on the first read: Your tables said nothing over a strip that was
  // about Open tables. Each section takes its own answer now, and the refusal
  // is said once, in the section it is about.
  it("draws the two lists that answered on the first read, and one strip in the section that was refused", async () => {
    show((call) => {
      if (isGamesList(call, "open")) return { status: 500, body: wire.error };
      if (isGamesList(call, "playing")) return wire.gamesPlaying;
      return undefined;
    });
    const mine = section("Your tables");
    const open = section("Open tables");
    const playing = section("Being played");
    await waitFor(() => expect(within(mine).getAllByRole("listitem")).toHaveLength(wire.games.games.length));
    expect(within(playing).getAllByRole("listitem")).toHaveLength(wire.gamesPlaying.games.length);
    expect(within(open).getByText(wire.error.error)).toBeInTheDocument();
    expect(within(open).queryAllByRole("listitem")).toHaveLength(0);
    expect(screen.getAllByText(wire.error.error), "said once").toHaveLength(1);
    expect(screen.queryByText(/loading/)).toBeNull();
  });

  // **And a LATER read with one scope refused keeps that section's rows under
  // its strip while the other two are refreshed** — a wake's re-read is the
  // same three listings, and one refusal is not a reason to leave the rest of
  // the screen older than the store. The second `mine` answer is the corpus's
  // listing cut to its first row, so a refresh is visible as one row fewer.
  it("keeps a refused section's rows under its strip on a later read, and refreshes the others", async () => {
    const stream = fakeStream();
    vi.useFakeTimers();
    try {
      let reads = 0;
      show((call) => {
        if (call.path.endsWith("/api/stream")) return stream.reply(call);
        if (isGamesList(call, "mine")) {
          reads += 1;
          return reads === 1 ? wire.games : { ...wire.games, games: wire.games.games.slice(0, 1) };
        }
        if (isGamesList(call, "open") && reads > 1) return { status: 500, body: wire.error };
        return undefined;
      });
      const mine = () => section("Your tables");
      const open = () => section("Open tables");
      await until(() => within(mine()).queryAllByRole("listitem").length === wire.games.games.length, "the first read");
      const openRows = wire.gamesOpen.games.length;
      expect(within(open()).getAllByRole("listitem")).toHaveLength(openRows);
      await until(() => stream.opens() > 0, "the house stream is open");

      stream.push("house", { seq: 1, chat: 0 });
      await until(() => within(mine()).queryAllByRole("listitem").length === 1, "Your tables was refreshed");
      expect(within(open()).getAllByRole("listitem"), "the refused section keeps its rows").toHaveLength(openRows);
      expect(within(open()).getByText(wire.error.error)).toBeInTheDocument();
      expect(screen.getAllByText(wire.error.error), "said once").toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });
});

// ---- M6.5c2 cycle C: the Finished group (R-6.5c2.22; spec section 2.2 as amended)
//
// The `playing` scope lists finished games too ("over" is not a status), so the
// lobby splits its third list: Being played is the games still under way, and
// the finished ones sit in ONE fold below it, closed, with a count and View on
// every row (R-C6). **Every table here is a corpus table with `over` moved**,
// because the corpus's walk finishes no game; a cell that moves more names why.
describe("the Finished group", () => {
  /** The group's one fold, or null where the lobby draws none. */
  const fold = (): HTMLDetailsElement | null =>
    document.querySelector<HTMLDetailsElement>("section.lobby-section.finished-tables > details.fold");
  /** `wire.gamesPlaying`'s one table: under way, read by a caller with no seat
   *  at it who has given its password (`can_watch`). */
  const playing = wire.gamesPlaying.games[0]!;
  /** The same table, finished. One field moved, `over`. */
  const done: TableView = { ...playing, over: true };
  /** The playing scope answering these tables, in this order. */
  const listing =
    (...games: TableView[]) =>
    (call: ApiCall): Reply | undefined =>
      isGamesList(call, "playing") ? { ...wire.gamesPlaying, games } : undefined;
  /** The summary's heading, once the lists have landed. */
  const finishedHeading = () => screen.findByRole("heading", { level: 3, name: "Finished" });

  it("folds a finished game you do not sit at under Finished, closed, and not under Being played", async () => {
    expect(playing.my_seats).toEqual([]);
    show(listing(done));
    const heading = await finishedHeading();
    const f = fold()!;
    expect(heading.closest("summary")?.parentElement, "the heading is the fold's summary").toBe(f);
    expect(f.open, "folded by default").toBe(false);
    expect(f).not.toHaveAttribute("open");
    expect(f.querySelector(":scope > summary > .section-hint")).toHaveTextContent(/^1 game$/);
    expect([...f.querySelectorAll("a.row-link")].map((a) => a.getAttribute("href"))).toEqual(["#/t/wire"]);
    // Not under Being played: that list is the games still under way, and it
    // has none, so it says so.
    const beingPlayed = section("Being played");
    expect(within(beingPlayed).queryByRole("link", { name: "wire" })).toBeNull();
    expect(within(beingPlayed).getByText("nothing is being played right now.")).toBeInTheDocument();
  });

  it("opens to the finished game's row: its size, who won, and View", async () => {
    // Two fields moved, `over` and `winner`: no committed payload is over with
    // a winner on it, and "who won" is the one thing this row is read for.
    const won: TableView = { ...playing, over: true, winner: 1 };
    const { user } = show(listing(won));
    await user.click(await finishedHeading());
    const f = fold()!;
    expect(f.open).toBe(true);
    const row = within(f).getByRole("link", { name: "wire" }).closest("li")!;
    expect(row.querySelector(".row-meta")).toHaveTextContent("2 players \u00b7 over \u00b7 Blue won");
    const view = within(row).getByRole("button", { name: "View: wire" });
    expect(within(row).queryByRole("button", { name: /^Spectate/ })).toBeNull();
    // The password was given (`can_watch`), so no padlock; the tag stays on
    // the name, as on every locked row.
    expect(view.querySelector("svg.padlock")).toBeNull();
    expect(within(row).getByLabelText("password required")).toBeInTheDocument();
    // The other lists' row: one link, not gold-edged, and no Abandon, which is
    // the creator's under Your tables — and this row is a watcher's reading
    // (`mine: false`, R-6.5c2.35).
    expect(won.mine).toBe(false);
    expect(row).toHaveClass("lobby-row");
    expect(row).not.toHaveClass("mine");
    expect(row.querySelectorAll("a")).toHaveLength(1);
    expect(within(row).queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  it("counts two finished games, in the listing's newest-first order", async () => {
    // The second is a second TABLE: `id` and `name` moved so the two rows are
    // named apart, and `created_at` a day earlier so the pair is in the order
    // the server lists (`created_at` descending, then `id`).
    const older: TableView = { ...done, id: "older", name: null, created_at: done.created_at - 86_400 };
    show(listing(done, older));
    await finishedHeading();
    const f = fold()!;
    expect(f.querySelector(":scope > summary > .section-hint")).toHaveTextContent(/^2 games$/);
    expect([...f.querySelectorAll("a.row-link")].map((a) => a.textContent)).toEqual(["wire", "older"]);
  });

  // **R-C6: an empty fold is clutter.** The group is absent, not empty, while
  // no finished game the caller does not sit at is listed — including when
  // the only finished game is the caller's own, which the playing scope lists
  // too and Your tables keeps, with View.
  it("draws no Finished group while no finished game is listed but yours", async () => {
    show(listing(playing));
    await within(section("Being played")).findByRole("button", { name: "Spectate: wire" });
    expect(document.querySelector("section.finished-tables")).toBeNull();
    expect(screen.queryByRole("heading", { name: "Finished" })).toBeNull();
    cleanup();

    const yours: TableView = { ...wire.table, over: true }; // one field moved, `over`
    expect(yours.my_seats).toEqual([1]);
    show((call) =>
      isGamesList(call, "mine")
        ? { ...wire.games, games: [yours] }
        : isGamesList(call, "playing")
          ? { ...wire.gamesPlaying, games: [yours] }
          : undefined,
    );
    const row = (await within(section("Your tables")).findByRole("link", { name: "wire" })).closest("li")!;
    expect(within(row).getByRole("button", { name: "View: wire" })).toBeInTheDocument();
    expect(document.querySelector("section.finished-tables")).toBeNull();
    expect(within(section("Being played")).queryByRole("link", { name: "wire" })).toBeNull();
  });

  // **Review Focus 5**: a locked finished game this caller has not unlocked.
  // Its View carries the padlock Spectate carried, and goes to the same
  // address — where the unlock face answers, because the face is the server's
  // `can_watch` and not the route's.
  it("puts the padlock on a locked finished game's View, and View opens the table's address", async () => {
    const locked: TableView = { ...wire.tableWatchable, over: true }; // one field moved, `over`
    expect(locked.can_watch).toBe(false);
    window.location.hash = "#/lobby";
    const { stub, user } = show(listing(locked));
    await user.click(await finishedHeading());
    const row = within(fold()!).getByRole("link", { name: "wire" }).closest("li")!;
    const view = within(row).getByRole("button", { name: "View: wire" });
    expect(view.querySelector("svg.padlock")).not.toBeNull();
    expect(window.location.hash).toBe("#/lobby");
    await user.click(view);
    expect(window.location.hash).toBe("#/t/wire");
    expect(stub.calls.filter((c) => c.path.includes("/api/games/"))).toHaveLength(0);
  });
});
// ---- M6.4: the lobby chat -------------------------------------------------

describe("the lobby's chat", () => {
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
    expect(panel.closest("section.lobby-chat"), "wrapped by the lobby's own rail").not.toBeNull();
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

  // **The lobby chat is a COUNT host off the house frame** (M6.5c2 cycle D,
  // row 130). The house frame carries the house chat's newest number beside
  // the sequence every table's move bumps; the panel used to be handed a tick
  // per wake and asked for a delta on every one — every move of every game on
  // the server — and made a second read at mount off the lobby's own opening
  // read. It reads its delta now only when `chat` passes what it holds, and
  // the mount read is the only read until then. The panel is OPEN, because a
  // folded count host reads nothing whatever the count says.
  //
  // **Where the house frames come from**: no corpus fixture carries one — a
  // frame is a stream's, not a route's payload — so each is built in the shape
  // the layer pins (`signal.rs`'s `house_frame`, `{"seq":…,"chat":…,"rev":…}`,
  // the same on the Worker) from the corpus's own numbers: `house_chat.json`'s
  // `last` and `rev`, and one past each for the message that moves them. The
  // page behind that message is `house_chat.json` with its first message
  // renumbered to it.
  it("reads the lobby chat once at mount, not on a table's move, and again when the house chat moves", async () => {
    setLobbyChat(true);
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    const stream = fakeStream();
    const next = { ...wire.houseChat.messages[0]!, seq: wire.houseChat.last + 1 };
    let page: ChatPage = wire.houseChat;
    vi.useFakeTimers();
    try {
      const { stub } = show((call) => {
        if (call.path.endsWith("/api/stream")) return stream.reply(call);
        if (call.method === "GET" && call.path.startsWith("/api/chat")) return page;
        return undefined;
      });
      await until(() => stream.opens() > 0, "the house stream is open");
      await until(() => within(section("Your tables")).queryAllByRole("listitem").length > 0, "the lists landed");
      await flush();
      expect(stub.matching("GET", "/api/chat"), "one read at mount, the lists' landing none").toHaveLength(1);
      // A table moved: the sequence moves and the house chat does not.
      stream.push("house", { seq: 1, chat: wire.houseChat.last, rev: wire.houseChat.rev });
      await flush();
      expect(stub.matching("GET", "/api/chat"), "a table's move asks the chat nothing").toHaveLength(1);
      page = { ...wire.houseChat, messages: [...wire.houseChat.messages, next], last: next.seq, rev: wire.houseChat.rev + 1 };
      stream.push("house", { seq: 2, chat: next.seq, rev: wire.houseChat.rev + 1 });
      await until(() => stub.matching("GET", "/api/chat").length === 2, "the chat delta was asked for");
      await flush();
      expect(stub.matching("GET", "/api/chat"), "one read for the one message").toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });

  // **A restarted server's first frame is read** (M6.5c2 cycle D, the final
  // review's C1; the reviewer's probe, kept). The rule this panel reads by — a
  // frame's `chat` above the `last` of the page it holds — trusts the frame's
  // numbers to be the house chat DOCUMENT's. On the Worker they are read off
  // the document. On the NAS they were a count the process kept from 0, so
  // after a restart the first message over a room at 57 framed `chat: 1`, and
  // the panel read nothing, by its own rule: a count below what it holds is
  // not news. The NAS now seeds its counters from the document when it starts
  // (the lib cell `after_a_restart_the_house_frame_counts_on_from_the_document`),
  // so the same message frames `chat: 58` beside the restarted process's `seq`
  // of 0. Both frames are pushed here, the old one first: it reads nothing,
  // and the seeded one reads the message. The page is `house_chat.json`
  // renumbered, as the cells above build theirs, served above the `after` the
  // panel asks from.
  it("reads the house message a restarted server's first frame counts, and nothing for a count below what it holds", async () => {
    setLobbyChat(true);
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    const stream = fakeStream();
    const held = { ...wire.houseChat.messages[0]!, seq: 57 };
    const next = { ...wire.houseChat.messages[0]!, seq: 58 };
    let page: ChatPage = { ...wire.houseChat, messages: [held], last: 57, rev: 57 };
    vi.useFakeTimers();
    try {
      const { stub } = show((call) => {
        if (call.path.endsWith("/api/stream")) return stream.reply(call);
        if (call.method === "GET" && call.path.startsWith("/api/chat")) {
          const after = Number(new URL(call.path, "http://x").searchParams.get("after") ?? 0);
          return { ...page, messages: page.messages.filter((m) => m.seq > after) };
        }
        return undefined;
      });
      const panel = () => document.querySelector<HTMLElement>("section.lobby-chat")!;
      await until(() => stream.opens() > 0, "the house stream is open");
      await until(() => stub.matching("GET", "/api/chat").length === 1, "the mount read");
      await flush();
      expect(within(panel()).getAllByRole("listitem"), "premise: the room at 57").toHaveLength(1);
      page = { ...page, messages: [held, next], last: 58, rev: 58 };
      // The frame a restarted NAS sent before the seed: a count from 0.
      stream.push("house", { seq: 0, chat: 1, rev: 1 });
      await flush();
      expect(stub.matching("GET", "/api/chat"), "a count below what the panel holds is not news").toHaveLength(1);
      // The frame it sends now: the document's numbers.
      stream.push("house", { seq: 0, chat: 58, rev: 58 });
      await until(() => stub.matching("GET", "/api/chat").length === 2, "the chat delta was asked for");
      await flush();
      expect(within(panel()).getAllByRole("listitem"), "the 58th message is drawn").toHaveLength(2);
    } finally {
      vi.useRealTimers();
    }
  });

  // **A house message its socket's reconnect missed is read once the reader's
  // own post shows the gap** (the Task 11 re-review's lost-message finding;
  // the production path of `Chat.test.tsx`'s "reads the messages its own
  // post's number shows it missed"). On the socket arm, a close this client did
  // not make is followed by ONE probe, and the lobby's probe reads the
  // listings, which carry no chat count; the house says nothing on connect. So
  // a house message that landed while the socket was down reaches this browser
  // as no frame at all. The reader's next post is numbered two past what the
  // panel holds, and taking that 201 alone moved the cursor past the missed
  // message for good. The pages are built from the corpus as the cells above
  // build theirs: `house_chat.json`'s message renumbered to 2 (the missed one)
  // and `house_chat_post.json` renumbered to 3 (the post), served up to what
  // the server holds; the frame on the new socket in the layer's pinned shape.
  it("reads a house message its socket's reconnect missed once the reader's own post shows the gap", async () => {
    setLobbyChat(true);
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    setSignalKind("ws");
    FakeSocket.made = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    vi.useFakeTimers();
    try {
      const missed = { ...wire.houseChat.messages[0]!, seq: wire.houseChat.last + 1 };
      const post = { ...wire.houseChatPost, seq: wire.houseChat.last + 2 };
      const all = [...wire.houseChat.messages, missed, post];
      let held = wire.houseChat.last;
      const { stub } = show((call) => {
        if (call.method === "POST" && call.path === "/api/chat") {
          held = post.seq;
          return { status: 201, body: post };
        }
        if (call.method === "GET" && call.path.startsWith("/api/chat")) {
          const after = Number(new URL(call.path, "http://x").searchParams.get("after") ?? 0);
          return { ...wire.houseChat, messages: all.filter((m) => m.seq > after && m.seq <= held), last: held, rev: held };
        }
        return undefined;
      });
      const panel = () => document.querySelector<HTMLElement>("section.lobby-chat")!;
      await until(() => FakeSocket.made.length === 1, "the house socket was opened");
      act(() => FakeSocket.made[0]!.accept());
      await until(() => stub.matching("GET", "/api/chat").length === 1, "the mount read");

      // The missed message lands while the socket is down: no frame for it.
      held = missed.seq;
      const listed = stub.calls.filter((c) => isGamesList(c, "mine")).length;
      act(() => FakeSocket.made[0]!.drop());
      await until(() => stub.calls.filter((c) => isGamesList(c, "mine")).length > listed, "the probe read the listings");
      await flush();
      expect(stub.matching("GET", "/api/chat"), "the probe read no chat").toHaveLength(1);

      // The reader posts, and the 201 is numbered two past what the panel holds.
      const box = within(panel()).getByRole("textbox", { name: "Say something" });
      fireEvent.change(box, { target: { value: post.text } });
      fireEvent.submit(box.closest("form")!);
      await until(() => stub.matching("POST", "/api/chat").length === 1, "the post went out");
      await flush();

      // The socket comes back, and the post's own house frame counts it.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]);
      });
      await until(() => FakeSocket.made.length === 2, "the house socket reopened");
      act(() => FakeSocket.made[1]!.accept());
      act(() => FakeSocket.made[1]!.frame("house", { seq: 2, chat: post.seq, rev: post.seq }));
      await flush();
      expect(within(panel()).getAllByRole("listitem"), "the missed message is drawn").toHaveLength(all.length);
    } finally {
      vi.useRealTimers();
      setSignalKind("sse");
      vi.unstubAllGlobals();
    }
  });

  // **And nothing for its own post** (row 130): the 201 has already moved the
  // cursor to the message the house frame then counts. The room starts empty
  // (the resting `show` answer), so the post's number lands above it. The
  // frame is built as the cell above builds its own, from
  // `house_chat_post.json`'s number.
  it("asks the lobby chat nothing for its own post", async () => {
    setLobbyChat(true);
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    const stream = fakeStream();
    vi.useFakeTimers();
    try {
      const { stub } = show((call) => {
        if (call.path.endsWith("/api/stream")) return stream.reply(call);
        if (call.method === "POST" && call.path === "/api/chat") return { status: 201, body: wire.houseChatPost };
        return undefined;
      });
      await until(() => stream.opens() > 0, "the house stream is open");
      await until(() => screen.queryByRole("textbox", { name: "Say something" }) !== null, "the chat's input");
      const box = screen.getByRole("textbox", { name: "Say something" });
      fireEvent.change(box, { target: { value: "hi all" } });
      fireEvent.submit(box.closest("form")!);
      await until(() => stub.matching("POST", "/api/chat").length === 1, "the post went out");
      await flush();
      const before = stub.matching("GET", "/api/chat").length;
      expect(before, "the mount read").toBe(1);
      stream.push("house", { seq: 1, chat: wire.houseChatPost.seq, rev: wire.houseChatPost.seq });
      await flush();
      expect(stub.matching("GET", "/api/chat"), "the frame counts the post the 201 drew").toHaveLength(before);
      expect(screen.getByText(wire.houseChatPost.text)).toBeInTheDocument();
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
  // **The band is the lobby's heading, and its corner is who is playing**
  // (M6.5c2, spec §2.2): the corner is the node the host hands in, and the way
  // back is not in it once somebody is signed in.
  it("draws the band — the product, the lobby, and the corner it is handed", async () => {
    show();
    expect(await screen.findByRole("heading", { level: 2, name: "Lobby" })).toBeInTheDocument();
    const band = document.querySelector<HTMLElement>("header.lobby-band")!;
    expect(within(band).getByRole("heading", { level: 1, name: "Yokohama" })).toBeInTheDocument();
    expect(band.querySelector(".band-corner .identity-corner")).toHaveTextContent(ME);
    expect(within(band).queryByRole("link", { name: "Games on this device" })).toBeNull();
  });

  it("carries one way back to the door, beside New table and above the lists", async () => {
    show();
    const back = await screen.findByRole("link", { name: "Games on this device" });
    // An ADDRESS, and in the actions row once somebody is signed in: the band's
    // corner is the identity's then.
    expect(back).toHaveAttribute("href", "#/");
    expect(back.closest(".lobby-actions")).not.toBeNull();
    const lists = await screen.findByRole("heading", { name: "Your tables" });
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
    // their own browser need no door at all. In the band's corner, where the
    // identity would be: the one control on this screen that needs no door.
    const back = screen.getByRole("link", { name: "Games on this device" });
    expect(back.closest("header.lobby-band")).not.toBeNull();
    // **Every `/api/games` route is a 401 without an identity** (M6.1), so a
    // lobby that listed first would show a refusal as the player's first
    // screen — the reason the door was a gate in the first place, kept.
    expect(screen.queryByRole("heading", { name: "Your tables" })).toBeNull();
    expect(screen.queryByRole("button", { name: "New table" })).toBeNull();
    expect(stub.calls).toHaveLength(0);
  });

  it("says it can't reach the server in place of the lists, offers to try again, and goes on reading", async () => {
    const stub = stubApi((call) => {
      const l = lists(call);
      if (l !== undefined) return l;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0, rev: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const retry = vi.fn();
    render(<Lobby me={ME} serverUp={false} onRetry={retry} />);
    const card = await screen.findByRole("heading", { level: 3, name: "Can't reach the server" });
    // In the same centring box the door's own C stands in (review M1): drawn
    // bare, the card sat flush left under the band.
    expect(card.closest(".lobby-door"), "the card is centred as the door's C is").not.toBeNull();
    expect(screen.queryByRole("heading", { name: "Your tables" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Open tables" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Being played" })).toBeNull();
    expect(screen.queryByRole("button", { name: "New table" })).toBeNull();
    await userEvent.setup().click(screen.getByRole("button", { name: "Try again" }));
    expect(retry).toHaveBeenCalledTimes(1);
    // **The read is still going out**: the card says what is true of the last
    // health read, not that this screen has given up — the sections come back
    // when a read lands.
    await waitFor(() => expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1));
  });

  // **A listing that got NO answer is said in a player's words** (the door's C
  // says "Can't reach the server"). `fetch` failing rejects with the
  // platform's own `TypeError: Failed to fetch`, which is a message to a
  // programmer. And under the signed-in C card, which already says it, the
  // strip does not say it a second time.
  it("words a listing that got no answer as the server out of reach, never the platform's error", async () => {
    const stub = stubApi((call) => {
      if (isGamesList(call, "open")) throw new TypeError("Failed to fetch");
      const l = lists(call);
      if (l !== undefined) return l;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0, rev: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const { rerender } = render(<Lobby me={ME} />);
    const strip = await screen.findByText("Can't reach the server. The lists come back as soon as it answers.");
    expect(strip.closest(".lobby-body")).not.toBeNull();
    expect(stub.calls.filter((c) => isGamesList(c, "open")).length).toBeGreaterThan(0);
    expect(screen.queryByText(/TypeError|Failed to fetch/)).toBeNull();
    expect(screen.queryByText(/none yet/)).toBeNull();
    // The health read failed too: the card says it, once.
    rerender(<Lobby me={ME} serverUp={false} />);
    expect(screen.getByRole("heading", { level: 3, name: "Can't reach the server" })).toBeInTheDocument();
    expect(screen.getAllByText(/Can't reach the server/)).toHaveLength(1);
    expect(screen.queryByText(/TypeError|Failed to fetch/)).toBeNull();
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
        return { ...wire.houseChat, messages: [], last: 0, rev: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const { rerender } = render(<Lobby me={ME} onReached={() => {}} />);
    await waitFor(() => expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1));
    for (let i = 0; i < 3; i += 1) rerender(<Lobby me={ME} onReached={() => {}} />);
    await waitFor(() => expect(screen.getByRole("heading", { name: "Your tables" })).toBeInTheDocument());
    expect(stub.calls.filter((c) => isGamesList(c, "mine"))).toHaveLength(1);
  });

  it("tells the app when a read lands, which is what brings the sections back", async () => {
    const onReached = vi.fn();
    stubApi((call) => {
      const l = lists(call);
      if (l !== undefined) return l;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0, rev: 0 };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    render(<Lobby me={ME} serverUp={false} onReached={onReached} />);
    await waitFor(() => expect(onReached).toHaveBeenCalled());
  });
});
