// The open table (spec §5.4): the screen a join link lands on.
//
// **Two faces, and `my_seats` is the whole of the question.** A caller with no
// seat here is a STRANGER — the one person on this API allowed to read a table
// they are not at (R-2.7) — and what they are offered is a seat. A caller who
// holds one is WAITING, and what they are offered is the link to send somebody
// else and, if they opened the table, the way to abandon it.
//
// Every payload below is the corpus's. The two faces are two committed
// fixtures: `wire.tableOpen` is the table as the stranger reads it, and the
// lobby's second row is the same shape from the seat — a listing row and a
// `GET …/table` reply are one `TableView`, so neither face is built here.

import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { OpenTable } from "./OpenTable";
import { DISPLAY_NAME_KEY } from "./api";
import { fakeStream, flush, openStream, until, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import { endSentence } from "./signal";
import type { TableView } from "./types";

/** The table this caller SITS at, still filling its seats — `wire2`, the
 *  lobby's second row, committed exactly in that shape: `status: "open"`,
 *  `my_seats: [0]`, one open seat beside this caller's, and opened by the
 *  person this screen plays as. Nothing is moved; the overrides below are what
 *  each cell is ABOUT. */
const seated = (over: Partial<TableView> = {}): TableView => ({
  ...wire.games.games[1]!,
  ...over,
});

/** `show`, at the id `seated()` actually carries. */
const showSeated = (
  handler: (call: ApiCall) => Reply | undefined,
  props: Partial<Parameters<typeof OpenTable>[0]> = {},
) => show(handler, { game: "wire2", ...props });

function show(
  handler: (call: ApiCall) => Reply | undefined,
  props: Partial<Parameters<typeof OpenTable>[0]> = {},
) {
  const onStarted = vi.fn();
  const onLeave = vi.fn();
  const stub = stubApi((call) => {
    const mine = handler(call);
    if (mine !== undefined) return mine;
    // This screen watches the table's stream (M6.3). A cell about the watch
    // scripts its own; the rest get one that opens and says nothing, which is
    // what a reachable server with nothing to report does.
    if (call.path.endsWith("/stream")) return openStream();
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  render(
    <OpenTable
      game="wire"
      me="red@wire.test"
      onStarted={onStarted}
      onLeave={onLeave}
      {...props}
    />,
  );
  return { stub, onStarted, onLeave, user: userEvent.setup() };
}

// **The clock is put back after EVERY cell**, and it is not tidiness: an
// assertion that throws with fake timers still installed hangs every cell after
// it in the file, so the first failure would be reported as a dozen timeouts.
afterEach(() => {
  vi.useRealTimers();
});

/** Every `GET …/table` answered with the same table. */
const tableIs = (t: TableView) => (call: ApiCall): Reply | undefined =>
  call.path.endsWith("/table") ? t : undefined;

describe("the open table, to a stranger", () => {
  it("asks for the table itself rather than waiting out a poll interval", async () => {
    // **The hook does not ask on arm** (`signal.ts`), so a screen that armed the
    // poll and nothing else would read "loading…" for three seconds on every
    // visit — which is the lobby's own reason for an opening fetch.
    const { stub } = show(tableIs(wire.tableOpen));
    await screen.findByRole("button", { name: "sit here" });
    expect(stub.matching("GET", "/table")).toHaveLength(1);
  });

  it("offers a name, a password and one seat — the open one", async () => {
    show(tableIs(wire.tableOpen));
    expect(await screen.findByLabelText("Your name")).toBeInTheDocument();
    // The password field exists because THIS table is locked; the cell below
    // is the other half.
    expect(wire.tableOpen.locked).toBe(true);
    expect(screen.getByLabelText("Password")).toBeInTheDocument();
    // One seat is human and one is open, so one button and not two.
    const seats = screen.getAllByRole("listitem");
    expect(seats).toHaveLength(wire.tableOpen.players);
    expect(screen.getAllByRole("button", { name: "sit here" })).toHaveLength(1);
    expect(within(seats[1]!).getByRole("button", { name: "sit here" })).toBeInTheDocument();
    // And the seat somebody holds reads as its NAME (§5.6), not its address.
    expect(seats[0]).toHaveTextContent("Red");
    // Nothing a stranger holds: no link to send on, and nothing to abandon.
    expect(screen.queryByLabelText("Link")).toBeNull();
    expect(screen.queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  it("asks for no password at an unlocked table", async () => {
    show(tableIs({ ...wire.tableOpen, locked: false }));
    await screen.findByRole("button", { name: "sit here" });
    expect(screen.queryByLabelText("Password")).toBeNull();
  });

  it("posts the seat, the name and the password, and remembers the name", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return seated({ my_seats: [1] });
      return undefined;
    });
    await user.type(await screen.findByLabelText("Your name"), "Jeremy");
    await user.type(screen.getByLabelText("Password"), "hunter2");
    await user.click(screen.getByRole("button", { name: "sit here" }));

    await waitFor(() => expect(stub.matching("POST", "/join")).toHaveLength(1));
    // Seat ONE — the open one, and the only value no constant and no
    // first-seat default produces.
    expect(stub.matching("POST", "/join")[0]!.body).toEqual({
      seat: 1,
      name: "Jeremy",
      password: "hunter2",
    });
    // The name is a per-BROWSER convenience, so the next table does not ask
    // again (§5.3). It is never an identity: who you are is `/api/me`'s answer.
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Jeremy");
  });

  it("sends no name when this browser has none, and no password when the table is unlocked", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return { ...wire.tableOpen, locked: false };
      if (call.path.endsWith("/join")) return seated({ my_seats: [1] });
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "sit here" }));
    await waitFor(() => expect(stub.matching("POST", "/join")).toHaveLength(1));
    // **The KEYS, not the values.** `{name: undefined}` serializes away, so an
    // assertion on the value alone would pass for a client that sent the key.
    expect(stub.matching("POST", "/join")[0]!.body).toEqual({ seat: 1 });
  });

  it("starts from the name this browser already gave", async () => {
    localStorage.setItem(DISPLAY_NAME_KEY, "Jeremy");
    show(tableIs(wire.tableOpen));
    expect(await screen.findByLabelText("Your name")).toHaveValue("Jeremy");
  });

  it("shows a wrong password's sentence and keeps what was typed", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return { status: 403, body: wire.join403Password };
      return undefined;
    });
    await user.type(await screen.findByLabelText("Password"), "wrong");
    await user.click(screen.getByRole("button", { name: "sit here" }));
    expect(await screen.findByText(wire.join403Password.error)).toBeInTheDocument();
    // **The fields stay.** A refusal that cleared them would make the second
    // attempt a retype, and the one thing this refusal is about is the field.
    expect(screen.getByLabelText("Password")).toHaveValue("wrong");
    expect(screen.getByRole("button", { name: "sit here" })).toBeInTheDocument();
    // And the table was not re-read: a 403 is about the caller, not about the
    // table having moved.
    expect(stub.matching("GET", "/table")).toHaveLength(1);
  });

  it("re-reads the table once when the seat was taken while this screen was open", async () => {
    let tables = 0;
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) {
        tables += 1;
        // The second read is the truth the refusal is about: both seats full.
        return tables === 1
          ? wire.tableOpen
          : { ...wire.tableOpen, seats: wire.table.seats, chosen: [false, false] };
      }
      if (call.path.endsWith("/join")) return { status: 409, body: wire.join409Taken };
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "sit here" }));
    expect(await screen.findByText(wire.join409Taken.error)).toBeInTheDocument();
    await waitFor(() => expect(stub.matching("GET", "/table")).toHaveLength(2));
    // ONCE: a re-read per refusal, not a retry loop.
    expect(stub.matching("POST", "/join")).toHaveLength(1);
    // And the screen now shows the table as it really is — no seat left to take.
    expect(screen.queryByRole("button", { name: "sit here" })).toBeNull();
  });

  it("hands the table over when the join says it is no longer open", async () => {
    // **A 409 CARRYING a status is not a refusal to show** (spec §5.4): the
    // table filled and started between this screen being drawn and the click,
    // so the answer is the next screen rather than a sentence.
    const { onStarted, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return { status: 409, body: wire.join409NotOpen };
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "sit here" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledTimes(1));
    expect(screen.queryByText(wire.join409NotOpen.error)).toBeNull();
  });

  it("hands over when the join that filled the last seat started the table", async () => {
    const { onStarted, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return wire.tableDealing;
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "sit here" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledTimes(1));
  });

  it("posts one join however fast the seat is clicked", async () => {
    // The guard is `App`'s and `Dealing`'s: React batches, so two clicks in one
    // tick both read a `busy` STATE of false and both post — and the second is
    // a 409 the player would read as the screen refusing their own click.
    let release!: () => void;
    const held = new Promise<void>((r) => {
      release = r;
    });
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return held.then(() => seated({ my_seats: [1] }));
      return undefined;
    });
    const sit = await screen.findByRole("button", { name: "sit here" });
    await user.click(sit);
    await user.click(sit);
    expect(stub.matching("POST", "/join")).toHaveLength(1);
    await act(async () => {
      release();
      await held;
    });
  });
});

describe("the open table, to somebody sitting at it", () => {
  it("shows the link to send, and no seat to take", async () => {
    // `locked: true`, so the absent Password field below is the seated FACE
    // and not merely a table that asks for no password.
    showSeated(tableIs(seated({ locked: true })));
    const link = await screen.findByLabelText("Link");
    expect(link).toHaveValue(`${window.location.origin}/#/t/wire2`);
    expect(screen.getByRole("button", { name: "copy" })).toBeInTheDocument();
    // Nothing to fill in: this person has a seat, and the fields are for taking
    // one.
    expect(screen.queryByLabelText("Your name")).toBeNull();
    expect(screen.queryByLabelText("Password")).toBeNull();
    expect(screen.queryByRole("button", { name: "sit here" })).toBeNull();
  });

  it("escapes an id that is not a bare word", async () => {
    // **An unescaped `/` makes a link `parseRoute` reads as the LOBBY** — its
    // pattern is `^/t/([^/]+)$` — so a table whose id carries one would hand out
    // an address that opens nothing. The same is true of the request path, which
    // `api.ts` escapes for the same reason.
    show(tableIs(seated({ id: "a b/c" })), { game: "a b/c" });
    expect(await screen.findByLabelText("Link")).toHaveValue(
      `${window.location.origin}/#/t/a%20b%2Fc`,
    );
  });

  // **What is this screen's, and what is the CONTROL's.** The two clicks, the
  // guard and the cancel are `Abandon`'s and are tested once, in
  // `Abandon.test.tsx`. What is here is the wiring: who is offered it, and
  // where the screen goes when the table is gone.
  it("offers the abandon control to the person who opened the table", async () => {
    showSeated(tableIs(seated()));
    // `created_by` is `red@wire.test`, which is who this browser is.
    expect(await screen.findByRole("button", { name: "abandon wire2" })).toBeInTheDocument();
  });

  it("offers nothing to abandon to a person who merely sits here", async () => {
    showSeated(tableIs(seated({ created_by: "someone@else.test" })));
    await screen.findByLabelText("Link");
    expect(screen.queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  it("leaves for the lobby once the table has been abandoned", async () => {
    const { stub, onLeave, user } = showSeated((call) => {
      if (call.path.endsWith("/table")) return seated();
      if (call.method === "DELETE") return null;
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "abandon wire2" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire2" }));
    await waitFor(() => expect(stub.matching("DELETE", "/api/games/wire2")).toHaveLength(1));
    // **The leave is the point.** There is nothing to go back to: the table the
    // screen is about no longer exists.
    await waitFor(() => expect(onLeave).toHaveBeenCalledTimes(1));
  });

  it("has a way back to the lobby", async () => {
    const { onLeave, user } = showSeated(tableIs(seated()));
    await user.click(await screen.findByRole("button", { name: "Lobby" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});

describe("the open table's watch", () => {
  /** Let a reply chain settle without moving the clock — the harness's own
   *  [`flush`]: this screen is woken by a frame and answers with a read, which
   *  is a chain of turns and not an interval. */
  const settle = flush;

  it("hands over when a frame wakes a read of a table that is no longer open", async () => {
    // **A `"wake"` consumer** (M6.3 R-7.1): the frame off `…/{id}/stream` is an
    // `Events` and this screen's value is a `TableView`, so what a frame buys is
    // one `GET …/table` — and the hand-over is decided on THAT, exactly as it
    // was decided on a tick's read before.
    vi.useFakeTimers();
    let status: "open" | "dealing" = "open";
    const onStarted = vi.fn();
    const stream = fakeStream();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/stream")) return stream.reply(call);
      return call.path.endsWith("/table")
        ? status === "open"
          ? wire.tableOpen
          : wire.tableDealing
        : undefined;
    });
    render(
      <OpenTable game="wire" me="red@wire.test" onStarted={onStarted} onLeave={() => {}} />,
    );
    await until(() => stream.opens() > 0, "the table's stream was opened");
    expect(onStarted).not.toHaveBeenCalled();
    // The opening read is this screen's own, and the frame below is what buys
    // the second one.
    expect(stub.matching("GET", "/table")).toHaveLength(1);

    status = "dealing";
    stream.push("signal", wire.eventsDealing);
    await settle();
    expect(stub.matching("GET", "/table")).toHaveLength(2);
    expect(onStarted).toHaveBeenCalled();
  });

  // **This cell drives a WAKE whose re-read 404s** — the table was deleted and
  // this screen learned it from its own `once`, not from an end frame (final
  // review I-2). The sentence is the refusal's. The real abandon is the cell
  // below, and the title used to claim that path without taking it.
  it("leaves for the lobby with the refusal's sentence when the re-read 404s", async () => {
    vi.useFakeTimers();
    const onLeave = vi.fn();
    let tables = 0;
    const stream = fakeStream();
    stubApi((call) => {
      if (call.path.endsWith("/stream")) return stream.reply(call);
      if (!call.path.endsWith("/table")) return undefined;
      tables += 1;
      // The opening read succeeds and the woken read then finds it deleted,
      // which is the reachable case: somebody abandoned it while this screen
      // was open.
      return tables === 1
        ? wire.tableOpen
        : { status: 404, body: { error: "no such game" } };
    });
    render(<OpenTable game="wire" me="red@wire.test" onStarted={() => {}} onLeave={onLeave} />);
    await until(() => stream.opens() > 0, "the table's stream was opened");
    stream.push("signal", wire.eventsDealing);
    await settle();
    // Asserted straight after, not through `waitFor`: under fake timers
    // `waitFor` polls on a timer nothing is advancing, and would wait out the
    // whole test budget on a screen that is already right.
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(onLeave.mock.calls[0]![0]).toBe("no such game");
    // And it STOPS. A 404 is permanent; a watch that went on asking would leave
    // the lobby being navigated to over and over.
    stream.push("signal", wire.eventsDealing);
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  // **A REAL abandon, which is an `end` frame and carries no words** (final
  // review I-2). `Signals::end` writes `{"reason":"deleted"}` and nothing else,
  // and this screen hands `end.sentence` straight to its leave — so before the
  // fix the creator abandoning an open table took every waiting joiner to the
  // lobby in SILENCE. The same gap caught the stranger on a table that FILLED
  // without them: the bump's read is then a 403, which the server reports as
  // `gone` and which has no sentence either.
  it("leaves with a sentence when the abandon arrives as an end frame", async () => {
    vi.useFakeTimers();
    const onLeave = vi.fn();
    const stream = fakeStream();
    stubApi((call) => {
      if (call.path.endsWith("/stream")) return stream.reply(call);
      return call.path.endsWith("/table") ? wire.tableOpen : undefined;
    });
    render(<OpenTable game="wire" me="red@wire.test" onStarted={() => {}} onLeave={onLeave} />);
    await until(() => stream.opens() > 0, "the table's stream was opened");
    expect(onLeave).not.toHaveBeenCalled();

    stream.push("end", { reason: "deleted" });
    await settle();
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(onLeave.mock.calls[0]![0], "the lobby is told WHY, not nothing at all").toBe(
      endSentence("deleted"),
    );
    expect(onLeave.mock.calls[0]![0]).toMatch(/abandoned/);

    // And the OTHER reason a frame can end this screen — the table filled
    // without this caller, which the server reads as a 403 and reports as
    // `gone` — has words of its own rather than none.
    expect(endSentence("gone")).toMatch(/no longer available/);
  });
});
