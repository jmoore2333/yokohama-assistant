// The open table (spec §5.4): the screen a join link lands on.
//
// **Two faces, and `my_seats` is the whole of the question.** A caller with no
// seat here is a STRANGER — the one person on this API allowed to read a table
// they are not at (R-2.7) — and what they are offered is a seat. A caller who
// holds one is WAITING, and what they are offered is the link to send somebody
// else and, if they opened the table, the way to abandon it.
//
// Every payload below is the corpus's, or the corpus's with one field moved:
// no committed fixture carries a table that is open AND seated (the fixture
// walk's stranger never sits), so the seated face is `wire.table` read at the
// status it passed through.

import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { OpenTable } from "./OpenTable";
import { DISPLAY_NAME_KEY } from "./api";
import { stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import type { TableView } from "./types";

/** The table this caller SITS at, still filling its seats.
 *
 *  `wire.table` is the same table once it is playing, so the four fields moved
 *  here are exactly the ones a status carries — and the second seat is reopened,
 *  because a table with no open seat is not one anybody is waiting at. */
const seated = (over: Partial<TableView> = {}): TableView => ({
  ...wire.table,
  status: "open",
  my_seats: [0],
  seats: [wire.table.seats[0]!, { kind: "open" }],
  chosen: [false, false],
  to_move: null,
  over: null,
  ...over,
});

function show(
  handler: (call: ApiCall) => Reply | undefined,
  props: Partial<Parameters<typeof OpenTable>[0]> = {},
) {
  const onStarted = vi.fn();
  const onLeave = vi.fn();
  const stub = stubApi((call) => {
    const mine = handler(call);
    if (mine !== undefined) return mine;
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
    expect(screen.queryByRole("button", { name: "Abandon" })).toBeNull();
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
    show(tableIs(seated()));
    const link = await screen.findByLabelText("Link");
    expect(link).toHaveValue(`${window.location.origin}/#/t/wire`);
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

  it("offers the creator a two-click Abandon", async () => {
    const { user } = show(tableIs(seated()));
    // `created_by` is `red@wire.test`, which is who this browser is.
    expect(await screen.findByRole("button", { name: "Abandon" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Abandon" }));
    expect(screen.getByRole("button", { name: /^confirm abandon/ })).toBeInTheDocument();
  });

  it("offers no Abandon to a person who merely sits here", async () => {
    show(tableIs(seated({ created_by: "someone@else.test" })));
    await screen.findByLabelText("Link");
    expect(screen.queryByRole("button", { name: "Abandon" })).toBeNull();
  });

  it("does not abandon on the first click, and does on the second", async () => {
    const { stub, onLeave, user } = show((call) => {
      if (call.path.endsWith("/table")) return seated();
      if (call.method === "DELETE") return null;
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "Abandon" }));
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
    // The confirming button NAMES the table, so the question cannot be about a
    // different one.
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() => expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(1));
    await waitFor(() => expect(onLeave).toHaveBeenCalledTimes(1));
  });

  it("puts the control back when the second click cancels", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return seated();
      if (call.method === "DELETE") return null;
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "Abandon" }));
    await user.click(screen.getByRole("button", { name: "cancel" }));
    expect(screen.getByRole("button", { name: "Abandon" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^confirm abandon/ })).toBeNull();
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
  });

  it("has a way back to the lobby", async () => {
    const { onLeave, user } = show(tableIs(seated()));
    await user.click(await screen.findByRole("button", { name: "Lobby" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});

describe("the open table's poll", () => {
  it("hands over when a tick reads a table that is no longer open", async () => {
    vi.useFakeTimers();
    let status: "open" | "dealing" = "open";
    const onStarted = vi.fn();
    stubApi((call) =>
      call.path.endsWith("/table")
        ? status === "open"
          ? wire.tableOpen
          : wire.tableDealing
        : undefined,
    );
    render(
      <OpenTable game="wire" me="red@wire.test" onStarted={onStarted} onLeave={() => {}} />,
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3100);
    });
    expect(onStarted).not.toHaveBeenCalled();

    status = "dealing";
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3100);
    });
    expect(onStarted).toHaveBeenCalled();
  });

  it("leaves for the lobby, with the sentence, when the table is gone", async () => {
    vi.useFakeTimers();
    const onLeave = vi.fn();
    let tables = 0;
    stubApi((call) => {
      if (!call.path.endsWith("/table")) return undefined;
      tables += 1;
      // The opening read succeeds and the poll then finds it deleted, which is
      // the reachable case: somebody abandoned it while this screen was open.
      return tables === 1
        ? wire.tableOpen
        : { status: 404, body: { error: "no such game" } };
    });
    render(<OpenTable game="wire" me="red@wire.test" onStarted={() => {}} onLeave={onLeave} />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3100);
    });
    // Asserted straight after the advance, not through `waitFor`: under fake
    // timers `waitFor` polls on a timer nothing is advancing, and would wait
    // out the whole test budget on a screen that is already right.
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(onLeave.mock.calls[0]![0]).toBe("no such game");
    // And it STOPS. A 404 is permanent; a poll that went on asking would leave
    // the lobby being navigated to over and over.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(9000);
    });
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});
