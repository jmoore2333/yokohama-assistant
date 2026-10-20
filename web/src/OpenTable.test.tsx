// The open table (spec §5.4): the screen a join link lands on.
//
// **Two faces, and `my_seats` is the whole of the question.** A caller with no
// seat here is a STRANGER — the one person on this API allowed to read a table
// they are not at (R-2.7) — and what they are offered is a seat. A caller who
// holds one is WAITING, and what they are offered is the link to send somebody
// else and, if they opened the table, the way to abandon it and a Reopen beside
// each seat somebody else took.
//
// Every payload below is the corpus's. The two faces are two committed
// fixtures: `wire.tableOpen` is the table as the stranger reads it, and the
// lobby's second row is the same shape from the seat — a listing row and a
// `GET …/table` reply are one `TableView`, so neither face is built here. The
// one derived table is `fourWithTwoOpen`, and it moves only the SIZE: its own
// doc says why.

import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NO_ANSWER } from "./Abandon";
import { OpenTable } from "./OpenTable";
import { DISPLAY_NAME_KEY } from "./api";
import { OPEN_KEY } from "./chatFold";
import { fakeStream, flush, openStream, until, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import { endSentence } from "./signal";
import type { ChatPage, TableView } from "./types";

/** The table this caller SITS at, still filling its seats — `wire2`, the
 *  lobby's second row, committed exactly in that shape: `status: "open"`,
 *  `my_seats: [0]`, one open seat beside this caller's, and opened by the
 *  person this screen plays as. Nothing is moved; the overrides below are what
 *  each cell is ABOUT. */
const seated = (over: Partial<TableView> = {}): TableView => ({
  ...wire.games.games[1]!,
  ...over,
});

/** `seated()` as a GUEST reads it — somebody who sits at the table and did not
 *  open it. The corpus has no such reading of an OPEN table (the one guest the
 *  walk seats fills the last seat, and the table deals), so the two fields the
 *  server answers differently to a guest are moved (M6.5c2 cycle D,
 *  R-6.5c2.35): `mine`, and each seat's `invited`, which is the creator's
 *  alone. */
const asGuest = (t: TableView): TableView => ({
  ...t,
  mine: false,
  seats: t.seats.map((s) => (s.kind === "human" ? { ...s, invited: null } : s)),
});

/** A FOUR-seat table with two seats still open, as a stranger reads it. **Every
 *  value is the corpus's** — the stranger's own open table, with Blue's seat
 *  as a stranger reads it (`table_watchable.json`: named, not `you`) and the
 *  open seat's shape from `table_open.json` — and only the SIZE moves: the
 *  walk's tables are two-seat, so no committed payload has two seats free, and
 *  the several-seat face is the one R-6.5c2.9 is about. */
const fourWithTwoOpen = (): TableView => ({
  ...wire.tableOpen,
  players: 4,
  colors: ["Red", "Blue", "Yellow", "Green"],
  seats: [wire.tableOpen.seats[0]!, wire.tableWatchable.seats[1]!, wire.tableOpen.seats[1]!, wire.tableOpen.seats[1]!],
  chosen: [false, false, false, false],
});

/** The seat cards, in seat order. */
const cards = () => within(screen.getByRole("list", { name: "Seats" })).getAllByRole("listitem");

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
    // **A seated caller's screen draws a chat panel since M6.4** (spec §6.2),
    // and it asks on mount whatever its fold state — so every `showSeated` cell
    // now makes that request whether or not it is about the chat. A table where
    // nothing has been said, derived from the corpus's own page rather than
    // typed out here, so a cell that is not about the chat has none of it in
    // its DOM. A STRANGER's screen draws no panel and asks for nothing, which
    // is the half the cell below pins.
    if (call.method === "GET" && call.path.includes("/chat"))
      return { ...wire.chat, messages: [], last: 0, rev: 0 };
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  render(
    <OpenTable
      game="wire"
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

describe("the open table, to a stranger with one seat to take", () => {
  it("asks for the table itself rather than waiting out a poll interval", async () => {
    // **The hook does not ask on arm** (`signal.ts`), so a screen that armed the
    // poll and nothing else would read "loading…" for three seconds on every
    // visit — which is the lobby's own reason for an opening fetch.
    const { stub } = show(tableIs(wire.tableOpen));
    await screen.findByRole("button", { name: "Join as Blue" });
    expect(stub.matching("GET", "/table")).toHaveLength(1);
  });

  // **One free seat, one button** (R-6.5c2.9) — and the password ABOVE the
  // seats (spec §2.2): typed once, then the choice.
  it("offers ONE way in, Join as Blue, with the table's password above the seats", async () => {
    show(tableIs(wire.tableOpen));
    const join = await screen.findByRole("button", { name: "Join as Blue" });
    expect(screen.queryAllByRole("button", { name: /^Sit here/ })).toEqual([]);
    // The password field exists because THIS table is locked; the cell below
    // is the other half.
    expect(wire.tableOpen.locked).toBe(true);
    const password = screen.getByLabelText("Table password");
    const list = screen.getByRole("list", { name: "Seats" });
    expect(password.compareDocumentPosition(list) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(list.compareDocumentPosition(join) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    // Two cards; the taken one by its holder's NAME (§5.6), not its address,
    // the open one telling the stranger it is theirs for the asking; Red plays
    // first.
    expect(cards()).toHaveLength(wire.tableOpen.players);
    expect(cards()[0]!).toHaveTextContent("Red \u00b7 first player");
    expect(cards()[0]!).toHaveTextContent("Red");
    expect(cards()[0]!).not.toHaveTextContent("red@wire.test");
    expect(cards()[1]!).toHaveTextContent("Open seat");
    expect(cards()[1]!).toHaveTextContent("yours if you join");
    // Whose table, how big, how much room — and the lock, as its tag.
    expect(screen.getByText(/Red's table \u00b7 2 players \u00b7 one seat left/)).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "password required" })).toBeInTheDocument();
    // Nothing a stranger holds: no link to send on, nothing to abandon, no name to type.
    expect(screen.queryByLabelText("Invite link")).toBeNull();
    expect(screen.queryByRole("button", { name: /abandon/i })).toBeNull();
    expect(screen.queryByLabelText("Your name")).toBeNull();
  });

  // **An unnamed seat reads "invited" to a stranger, and no address reaches
  // the page** (M6.5c2 cycle D, R-6.5c2.35). `wire2` as the lobby's stranger
  // read it (`games_open.json`): its creator sits at seat 0 and gave no name,
  // so the server sends no name, no `you`, no `invited` and no `creator` —
  // and the page says "invited" and "the creator's table" rather than the
  // address the seat is keyed by, which it was never handed.
  it("shows a stranger an unnamed seat as invited and never an address", async () => {
    const t = wire.gamesOpen.games[1]!;
    const seat = t.seats[0]!;
    if (seat.kind !== "human") throw new Error("wire2's seat 0 is its creator's");
    expect([seat.name, seat.you, seat.invited, t.creator, t.mine]).toEqual([null, false, null, null, false]);
    show(tableIs(t), { game: t.id });
    await screen.findByRole("button", { name: "Join as Blue" });
    expect(cards()[0]!).toHaveTextContent("invited");
    expect(screen.getByText(/the creator's table \u00b7 2 players \u00b7 one seat left/)).toBeInTheDocument();
    expect(document.body.textContent).not.toContain("@");
  });

  // **The first player is the table's `start_seat`, not the first card** (the
  // whole-branch review's Minor 3): every committed table starts at seat 0, so
  // a caption hard-wired to the first card passed every cell. One field moved,
  // `start_seat`, from 0 to 1.
  it("marks the first player at the table's start seat when that is not the first card", async () => {
    show(tableIs({ ...wire.tableOpen, start_seat: 1 }));
    await screen.findByRole("button", { name: "Join as Blue" });
    const seats = screen.getByRole("list", { name: "Seats" });
    expect(cards()[1]!).toHaveTextContent("Blue \u00b7 first player");
    expect(seats).not.toHaveTextContent("Red \u00b7 first player");
  });

  it("asks for no password at an unlocked table", async () => {
    show(tableIs({ ...wire.tableOpen, locked: false }));
    await screen.findByRole("button", { name: "Join as Blue" });
    expect(screen.queryByLabelText("Table password")).toBeNull();
    expect(screen.queryByRole("img", { name: "password required" })).toBeNull();
  });

  // **One click, never zero** (R-6.5c2.9, Review Focus 4): arriving by a link
  // seats nobody — a link preview or a curious click must not take a seat.
  it("seats nobody on arrival: nothing is posted until the button is pressed", async () => {
    // The file's clock idiom (`flush` advances a FAKED clock): an arrival that
    // joined off an effect or a timer would have posted within these turns.
    vi.useFakeTimers();
    const { stub } = show(tableIs(wire.tableOpen));
    await until(
      () => screen.queryByRole("button", { name: "Join as Blue" }) !== null,
      "the table was drawn",
    );
    await flush();
    expect(stub.matching("POST", "/join")).toEqual([]);
  });

  // **The name is the one this browser KEEPS, read at the click** — the form
  // asks for none since M6.5c2 (R-6.5c2.8), and a name set in the corner a
  // moment before is the one the seat takes.
  it("posts the seat, the name this browser keeps and the password", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return seated({ my_seats: [1] });
      return undefined;
    });
    const join = await screen.findByRole("button", { name: "Join as Blue" });
    localStorage.setItem(DISPLAY_NAME_KEY, "  Ben  ");
    await user.type(screen.getByLabelText("Table password"), "hunter2");
    await user.click(join);
    await waitFor(() => expect(stub.matching("POST", "/join")).toHaveLength(1));
    // Seat ONE — the open one, and the only value no constant and no
    // first-seat default produces.
    expect(stub.matching("POST", "/join")[0]!.body).toEqual({
      seat: 1,
      name: "Ben",
      password: "hunter2",
    });
  });

  it("sends no name when this browser keeps none, and no password when the table is unlocked", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return { ...wire.tableOpen, locked: false };
      if (call.path.endsWith("/join")) return seated({ my_seats: [1] });
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "Join as Blue" }));
    await waitFor(() => expect(stub.matching("POST", "/join")).toHaveLength(1));
    // **The KEYS, not the values.** `{name: undefined}` serializes away, so an
    // assertion on the value alone would pass for a client that sent the key.
    expect(stub.matching("POST", "/join")[0]!.body).toEqual({ seat: 1 });
  });

  it("shows a wrong password's sentence and keeps what was typed", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return { status: 403, body: wire.join403Password };
      return undefined;
    });
    await user.type(await screen.findByLabelText("Table password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Join as Blue" }));
    expect(await screen.findByText(wire.join403Password.error)).toBeInTheDocument();
    // **The field stays.** A refusal that cleared it would make the second
    // attempt a retype, and the one thing this refusal is about is the field.
    expect(screen.getByLabelText("Table password")).toHaveValue("wrong");
    expect(screen.getByRole("button", { name: "Join as Blue" })).toBeInTheDocument();
    // And the table was not re-read: a 403 is about the caller, not about the
    // table having moved.
    expect(stub.matching("GET", "/table")).toHaveLength(1);
  });

  // **The password counter's 429 is drawn as the 403 is** (security S3, D2): the
  // server's own sentence in the join form's strip, what was typed kept, Join
  // still there to press once the wait it names is over, and the table not
  // re-read. A join is a call, not the stream, so its 429 is not the stream
  // cap's refusal and the screen does not leave on it.
  it("shows the password counter's 429 in the server's words and stays usable", async () => {
    const tooMany = "too many wrong passwords for this table — wait ten seconds and try again";
    const { stub, onLeave, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return { status: 429, body: { error: tooMany } };
      return undefined;
    });
    await user.type(await screen.findByLabelText("Table password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Join as Blue" }));
    expect(await screen.findByText(tooMany)).toBeInTheDocument();
    expect(screen.getByLabelText("Table password")).toHaveValue("wrong");
    expect(stub.matching("GET", "/table")).toHaveLength(1);
    expect(onLeave).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "Join as Blue" }));
    await waitFor(() => expect(stub.matching("POST", "/join")).toHaveLength(2));
  });

  // **A join sent with no password at a locked table is refused in the
  // server's words** (dossier row 38): the corpus's `join_403_locked` had
  // contract cells only, and no screen was shown drawing it. Nothing typed,
  // so the join carries no password key at all.
  it("says a locked table wants its password when Join is pressed with none typed", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return { status: 403, body: wire.join403Locked };
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "Join as Blue" }));
    expect(await screen.findByText(wire.join403Locked.error)).toBeInTheDocument();
    expect(stub.matching("POST", "/join")[0]!.body).toEqual({ seat: 1 });
    expect(screen.getByLabelText("Table password")).toHaveValue("");
  });

  // **The refusal sits by the field it is about** (M6.5c2 cycle C, polish 4):
  // directly after the join block — the password, the seats and "Join as" —
  // and above the screen's controls, not at the page's foot under them, a
  // screen-height from the field a wrong password is about.
  it("puts a join's refusal right after the seats and the Join button, above the controls", async () => {
    const { user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return { status: 403, body: wire.join403Password };
      return undefined;
    });
    await user.type(await screen.findByLabelText("Table password"), "wrong");
    await user.click(screen.getByRole("button", { name: "Join as Blue" }));
    const strip = (await screen.findByText(wire.join403Password.error)).closest(".error")!;
    const after = (a: Node, b: Node) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(after(document.querySelector("ol.seat-cards")!, strip), "after the seats").toBe(true);
    expect(after(screen.getByRole("button", { name: "Join as Blue" }), strip), "after Join as Blue").toBe(true);
    expect(after(strip, document.querySelector(".open-table-controls")!), "above the controls").toBe(true);
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
    await user.click(await screen.findByRole("button", { name: "Join as Blue" }));
    expect(await screen.findByText(wire.join409Taken.error)).toBeInTheDocument();
    await waitFor(() => expect(stub.matching("GET", "/table")).toHaveLength(2));
    // ONCE: a re-read per refusal, not a retry loop.
    expect(stub.matching("POST", "/join")).toHaveLength(1);
    // The table as it really is: no seat left, so no way in of either kind.
    expect(screen.queryByRole("button", { name: /^Join as/ })).toBeNull();
    expect(screen.queryAllByRole("button", { name: /^Sit here/ })).toEqual([]);
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
    await user.click(await screen.findByRole("button", { name: "Join as Blue" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledTimes(1));
    expect(screen.queryByText(wire.join409NotOpen.error)).toBeNull();
  });

  it("hands over when the join that filled the last seat started the table", async () => {
    const { onStarted, user } = show((call) => {
      if (call.path.endsWith("/table")) return wire.tableOpen;
      if (call.path.endsWith("/join")) return wire.tableDealing;
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "Join as Blue" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledTimes(1));
  });

  it("posts one join however fast the button is clicked", async () => {
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
    const join = await screen.findByRole("button", { name: "Join as Blue" });
    await user.click(join);
    await user.click(join);
    expect(stub.matching("POST", "/join")).toHaveLength(1);
    await act(async () => {
      release();
      await held;
    });
  });
});

// ---- two or more free seats: pick a colour (R-6.5c2.9, mock 09) ------------
describe("the open table, to a stranger with a choice of seats", () => {
  it("offers Sit here in each open seat, the password once above them, and no single button", async () => {
    show(tableIs(fourWithTwoOpen()));
    const sits = await screen.findAllByRole("button", { name: /^Sit here as / });
    expect(sits.map((b) => b.getAttribute("aria-label"))).toEqual(["Sit here as Yellow", "Sit here as Green"]);
    expect(sits.map((b) => b.textContent)).toEqual(["Sit here", "Sit here"]);
    expect(within(cards()[2]!).getByRole("button", { name: "Sit here as Yellow" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^Join as/ })).toBeNull();
    const password = screen.getByLabelText("Table password");
    expect(
      password.compareDocumentPosition(screen.getByRole("list", { name: "Seats" })) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(screen.getByText(/Red's table \u00b7 4 players \u00b7 two seats left/)).toBeInTheDocument();
    // The seat Blue holds is a taken card, by the holder's name.
    expect(cards()[1]!).toHaveTextContent("Blue");
    expect(cards()[1]!).toHaveTextContent("seated");
  });

  it("takes THE seat that was chosen", async () => {
    const { stub, user } = show((call) => {
      if (call.path.endsWith("/table")) return fourWithTwoOpen();
      if (call.path.endsWith("/join")) return { ...fourWithTwoOpen(), my_seats: [3] };
      return undefined;
    });
    await user.type(await screen.findByLabelText("Table password"), "hunter2");
    await user.click(screen.getByRole("button", { name: "Sit here as Green" }));
    await waitFor(() => expect(stub.matching("POST", "/join")).toHaveLength(1));
    expect(stub.matching("POST", "/join")[0]!.body).toEqual({ seat: 3, password: "hunter2" });
  });
});

describe("the open table, to somebody sitting at it", () => {
  // **The creator's invite** (mock 08): the link, Copy link, and — at a locked
  // table — the reminder that the password travels separately. `locked: true`,
  // so the absent password field below is the seated FACE and not merely a
  // table that asks for no password.
  it("shows the creator the link to send, and says the password is theirs to send", async () => {
    showSeated(tableIs(seated({ locked: true })));
    const link = await screen.findByLabelText("Invite link");
    expect(link).toHaveValue(`${window.location.origin}/#/t/wire2`);
    expect(screen.getByRole("button", { name: "Copy link" })).toBeInTheDocument();
    expect(screen.getByText("They'll need the password you set. Send it separately.")).toBeInTheDocument();
    // Nothing to fill in: this person has a seat, and the fields are for taking
    // one.
    expect(screen.queryByLabelText("Table password")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Join as/ })).toBeNull();
    expect(screen.queryAllByRole("button", { name: /^Sit here/ })).toEqual([]);
  });

  // Somebody ELSE seated invites too, and is not told they set a password they
  // did not set.
  it("reminds a seated guest of the table's password without claiming they set it", async () => {
    showSeated(tableIs(asGuest(seated({ locked: true }))));
    await screen.findByLabelText("Invite link");
    expect(screen.getByText("They'll need the table's password.")).toBeInTheDocument();
    expect(screen.queryByText(/password you set/)).toBeNull();
  });

  it("says nothing about a password at an unlocked table", async () => {
    showSeated(tableIs(seated({ locked: false })));
    await screen.findByLabelText("Invite link");
    expect(screen.queryByText(/They'll need/)).toBeNull();
  });

  // **The seats as cards** (mock 08): yours says You and the name the table
  // shows for you; the open one says who may take it; the first player is marked.
  it("draws each seat as a card: you, the open seat and who may take it, the first player marked", async () => {
    showSeated(tableIs(seated({ locked: true })));
    await screen.findByLabelText("Invite link");
    const mine = wire.games.games[1]!.seats[0]!;
    if (mine.kind !== "human") throw new Error("wire2's seat 0 is the creator's");
    expect(cards()[0]!).toHaveTextContent("Red \u00b7 first player");
    expect(cards()[0]!).toHaveTextContent("You");
    // No name given, so the card is "You" and nothing else: not the address
    // the creator's own `invited` carries (M6.5c2 cycle D, R-6.5c2.35).
    expect(mine.name).toBeNull();
    expect(cards()[0]!.textContent).not.toContain("@");
    expect(cards()[1]!).toHaveTextContent("Open seat");
    expect(cards()[1]!).toHaveTextContent("anyone in the lobby, with the password");
    expect(screen.getByText(/Waiting for 1 more player \u00b7 2 players/)).toBeInTheDocument();
  });

  // **`userEvent.setup()` installs its own clipboard on `navigator`** (and
  // puts it back after every cell), so the copy is read back through it — a
  // stub defined before `show` is replaced by the setup and never called.
  // **"Copied" is said ONCE** (cycle E, straw poll row 68): the button was
  // itself the live region and its words — which were its name — changed, so
  // a screen reader could announce the copy twice. The button's name stays
  // "Copy link", and ONE status beside it, there from the first render,
  // carries the word.
  it("says Copied once the link has been copied: one status beside the button, whose name does not change", async () => {
    const { user } = showSeated(tableIs(seated()));
    const button = await screen.findByRole("button", { name: "Copy link" });
    const invite = screen.getByRole("region", { name: "Invite people" });
    const statuses = () => within(invite).getAllByRole("status");
    expect(statuses(), "the status is in the invite before anything is copied").toHaveLength(1);
    expect(statuses()[0]).toHaveTextContent(/^$/);
    await user.click(button);
    expect(await navigator.clipboard.readText()).toBe(`${window.location.origin}/#/t/wire2`);
    expect(statuses(), "exactly one status in the invite").toHaveLength(1);
    expect(statuses()[0]).toHaveTextContent(/^Copied$/);
    expect(button.contains(statuses()[0]!), "the status is beside the button, not in it").toBe(false);
    expect(button).toHaveAccessibleName("Copy link");
    expect(button).toHaveTextContent(/^Copy link$/);
    expect(button).not.toHaveAttribute("aria-live");
    expect(invite.querySelectorAll("[aria-live]"), "no other live region in the invite").toHaveLength(0);
  });

  // **A refused clipboard is said, and the button claims no copy** (the
  // whole-branch review's Minor 8): the clipboard is refused in a browser that
  // has not granted it and on a page served over plain HTTP, and the button
  // said "Copied" over nothing. Refused here as a browser refuses it, through
  // the clipboard `userEvent.setup()` installs, and HELD first: nothing is
  // claimed while the clipboard has not answered either. The sentence is the
  // lobby's.
  it("says the link could not be copied when the clipboard refuses, and keeps Copy link", async () => {
    const { user } = showSeated(tableIs(seated()));
    const copy = await screen.findByRole("button", { name: "Copy link" });
    let refuse!: (e: unknown) => void;
    const refused = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockReturnValue(new Promise<void>((_, reject) => (refuse = reject)));
    await user.click(copy);
    const url = `${window.location.origin}/#/t/wire2`;
    expect(refused).toHaveBeenCalledWith(url);
    const said = within(screen.getByRole("region", { name: "Invite people" })).getByRole("status");
    expect(said, "no copy is claimed before an answer").toHaveTextContent(/^$/);
    await act(async () => refuse(new DOMException("Write permission denied.", "NotAllowedError")));
    expect(await screen.findByText(`the link could not be copied \u2014 it is ${url}`)).toBeInTheDocument();
    expect(said, "no copy is claimed after a refusal").toHaveTextContent(/^$/);
    expect(screen.getByRole("button", { name: "Copy link" })).toBeInTheDocument();
  });

  // **A copy that lands takes the refusal away** (M6.5c2 cycle C; the Opus
  // re-review's one Minor): a clipboard refused and then granted within one
  // visit left "the link could not be copied" standing under a button that
  // says "Copied", two sentences contradicting each other.
  it("clears the refusal when a later copy lands, and says Copied", async () => {
    const { user } = showSeated(tableIs(seated()));
    const url = `${window.location.origin}/#/t/wire2`;
    const refused = `the link could not be copied \u2014 it is ${url}`;
    const write = vi
      .spyOn(navigator.clipboard, "writeText")
      .mockRejectedValueOnce(new DOMException("Write permission denied.", "NotAllowedError"));
    await user.click(await screen.findByRole("button", { name: "Copy link" }));
    expect(await screen.findByText(refused), "the premise: the first copy was refused").toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Copy link" }));
    expect(
      await within(screen.getByRole("region", { name: "Invite people" })).findByText("Copied"),
    ).toHaveAttribute("role", "status");
    expect(write).toHaveBeenCalledTimes(2);
    expect(screen.queryByText(refused), "the refusal is gone once a copy landed").toBeNull();
  });

  it("escapes an id that is not a bare word", async () => {
    // **An unescaped `/` makes a link `parseRoute` reads as the LOBBY** — its
    // pattern is `^/t/([^/]+)$` — so a table whose id carries one would hand out
    // an address that opens nothing. The same is true of the request path, which
    // `api.ts` escapes for the same reason.
    show(tableIs(seated({ id: "a b/c" })), { game: "a b/c" });
    expect(await screen.findByLabelText("Invite link")).toHaveValue(
      `${window.location.origin}/#/t/a%20b%2Fc`,
    );
  });

  // **What is this screen's, and what is the CONTROL's.** The two clicks, the
  // guard and the cancel are `Abandon`'s and are tested once, in
  // `Abandon.test.tsx`. What is here is the wiring: who is offered it, and
  // where the screen goes when the table is gone.
  it("offers the abandon control to the person who opened the table", async () => {
    showSeated(tableIs(seated()));
    // `wire2` is its creator's own reading: `mine` is true (R-6.5c2.35).
    expect(seated().mine).toBe(true);
    expect(await screen.findByRole("button", { name: "abandon wire2" })).toBeInTheDocument();
  });

  // **A PIN: the table's creator who holds no seat is offered NO Abandon**
  // (dossier row 17, as ruled after cycle D's close `2026-09-29-web-frontend-m6.5c2d-close.md` §5, ruling 34). The layer's
  // delete refuses a caller who holds no seat before it asks who created the
  // table — `delete_locked`: "you are not seated at this table" — so a control
  // here would be drawn over a guaranteed refusal; the rule is `mayAbandon`,
  // the one the lobby and the broken-game screen read. The state is
  // unreachable through `create` ("the creator must hold a seat", R-1.6); the
  // M6.2 close's "reachable if a create fills every seat with other people's
  // emails" is false at HEAD. The corpus's stranger reading with ONE field
  // moved, `mine`, so only the seat can withhold the control.
  it("offers no abandon control to the table's creator who holds no seat at it, whose delete the layer refuses", async () => {
    const creator = { ...wire.tableOpen, mine: true };
    expect(creator.my_seats).toHaveLength(0);
    show(tableIs(creator));
    await screen.findByRole("button", { name: "Join as Blue" });
    expect(screen.queryByRole("button", { name: /abandon/i })).toBeNull();
  });

  it("offers nothing to abandon to a person who merely sits here", async () => {
    showSeated(tableIs(asGuest(seated())));
    await screen.findByLabelText("Invite link");
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

  // **Abandon's refusal has a strip of its own, directly under the controls**
  // (M6.5c2 cycle C, polish 4; the task review's I-1, ruled R-C24). The join
  // block's strip is a screen-height ABOVE Abandon on a seated face — the
  // Invite panel and the chat sit between them, ~350px at 390 with the chat
  // folded — so each refusal sits beside its own control: the join's, the
  // password's and the copy's up there, Abandon's down here, and never in the
  // other's. A DELETE that got no answer is the refusal the control words
  // itself (`NO_ANSWER`).
  it("puts Abandon's refusal in its own strip directly after the controls, and not in the join block's", async () => {
    const { user } = showSeated((call) => {
      if (call.path.endsWith("/table")) return seated();
      if (call.method === "DELETE") throw new TypeError("Failed to fetch");
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "abandon wire2" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire2" }));
    const strip = (await screen.findByText(NO_ANSWER)).closest(".error")!;
    const controls = document.querySelector(".open-table-controls")!;
    expect(controls.nextElementSibling, "the element right after the controls").toBe(strip);
    // And not in the upper strip: nothing between the seats and the controls
    // carries it.
    const after = (a: Node, b: Node) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    // The join block's strip lives in the table's own part (`.open-table-main`)
    // since the chat took mock 08's rail (cycle E, row 67).
    const upper = [...document.querySelectorAll(".open-table-main > .error")].filter(
      (e) => after(document.querySelector("ol.seat-cards")!, e) && after(e, controls),
    );
    expect(upper.map((e) => e.textContent), "the join block's strip does not carry it").toEqual([]);
  });

  // **And the join block's strip keeps the copy's refusal, above the Invite
  // panel whose button raised it** (R-C24): the split moves Abandon's alone.
  it("keeps a refused copy's sentence in the join block's strip, above the controls", async () => {
    const { user } = showSeated(tableIs(seated()));
    vi.spyOn(navigator.clipboard, "writeText").mockRejectedValueOnce(
      new DOMException("Write permission denied.", "NotAllowedError"),
    );
    await user.click(await screen.findByRole("button", { name: "Copy link" }));
    const url = `${window.location.origin}/#/t/wire2`;
    const strip = (await screen.findByText(`the link could not be copied \u2014 it is ${url}`)).closest(".error")!;
    const after = (a: Node, b: Node) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(after(document.querySelector("ol.seat-cards")!, strip), "after the seats").toBe(true);
    expect(after(strip, document.querySelector("section.invite")!), "above the Invite panel").toBe(true);
    expect(after(strip, document.querySelector(".open-table-controls")!), "above the controls").toBe(true);
  });

  it("has a way back to the lobby", async () => {
    const { onLeave, user } = showSeated(tableIs(seated()));
    await user.click(await screen.findByRole("button", { name: "Lobby" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});

// **The creator's Reopen** (security S3, D9; design §3.2 rule 1): beside a
// JOINER's seat on the creator's own roster, and nowhere else — not beside the
// creator's own seat (D-15), a bot's, an open one, and never on a joiner's or
// a stranger's screen. The layer refuses every one of those cases itself; the
// control is drawn only where a press can land.
describe("the open table's Reopen, for its creator", () => {
  /** The creator's three-seat table with a JOINER at seat 1: the corpus's
   *  `table_two_open.json`, the creator's own reading, with seat 1 taken by the
   *  named guest as anyone but its holder reads it (`table_watchable.json`'s
   *  Blue). Only seat 1 moves. */
  const withAJoiner = (): TableView => ({
    ...wire.tableTwoOpen,
    seats: [wire.tableTwoOpen.seats[0]!, wire.tableWatchable.seats[1]!, wire.tableTwoOpen.seats[2]!],
  });
  /** Every kind of seat on one roster: the creator's, the joiner's, a bot's
   *  (`table_bot.json`'s) and an open one. A bot at an open table is no
   *  create's shape — a bot sits only at two players — so this is the
   *  control's rule asked of every kind, not a table the server makes. */
  const everyKind = (): TableView => ({
    ...wire.tableTwoOpen,
    players: 4,
    colors: ["Red", "Blue", "Yellow", "Green"],
    seats: [
      wire.tableTwoOpen.seats[0]!,
      wire.tableWatchable.seats[1]!,
      wire.tableBot.seats[1]!,
      wire.tableTwoOpen.seats[1]!,
    ],
    chosen: [false, false, true, false],
  });
  const reopens = () => screen.queryAllByRole("button", { name: /^Reopen/ });
  const game = wire.tableTwoOpen.id;

  it("draws Reopen beside a joiner's seat for the creator, and none beside their own, a bot's or an open one", async () => {
    show(tableIs(everyKind()), { game });
    await screen.findByLabelText("Invite link");
    expect(everyKind().mine, "premise: the creator's reading").toBe(true);
    expect(reopens()).toHaveLength(1);
    expect(within(cards()[1]!).getByRole("button", { name: "Reopen the Blue seat" })).toHaveTextContent("Reopen");
    for (const i of [0, 2, 3]) expect(within(cards()[i]!).queryByRole("button", { name: /^Reopen/ })).toBeNull();
  });

  it("draws no Reopen for a joiner, who did not open the table, nor for a stranger", async () => {
    const joiner: TableView = {
      ...withAJoiner(),
      mine: false,
      my_seats: [1],
      seats: [wire.tableOpen.seats[0]!, { kind: "human", name: "Blue", you: true, invited: null }, { kind: "open" }],
    };
    show(tableIs(joiner), { game });
    await screen.findByLabelText("Invite link");
    expect(reopens()).toEqual([]);
    cleanup();
    show(tableIs(wire.tableOpen));
    await screen.findByRole("button", { name: "Join as Blue" });
    expect(reopens()).toEqual([]);
  });

  it("posts the seat and redraws the roster from the answer", async () => {
    const { stub, user } = show(
      (call) => {
        if (call.path.endsWith("/table")) return withAJoiner();
        // The answer is the table with seat 1 open again: the corpus's own.
        if (call.path.endsWith("/reopen")) return wire.tableTwoOpen;
        return undefined;
      },
      { game },
    );
    await user.click(await screen.findByRole("button", { name: "Reopen the Blue seat" }));
    await waitFor(() => expect(stub.matching("POST", "/reopen")).toHaveLength(1));
    const posted = stub.matching("POST", "/reopen")[0]!;
    expect(posted.path).toBe(`/api/games/${game}/reopen`);
    // **0-based, as `keep` and `join` send it**: the second seat is seat 1.
    expect(posted.body).toEqual({ seat: 1 });
    await waitFor(() => expect(cards()[1]!).toHaveTextContent("Open seat"));
    expect(reopens()).toEqual([]);
    // The answer IS the table: nothing is read again for it.
    expect(stub.matching("GET", "/table")).toHaveLength(1);
  });

  it("puts a refusal's sentence in the join block's strip, in the server's words", async () => {
    const dealt = "the deal has begun \u2014 abandon the table and set it again";
    const { onStarted, onLeave, user } = show(
      (call) => {
        if (call.path.endsWith("/table")) return withAJoiner();
        if (call.path.endsWith("/reopen")) return { status: 409, body: { error: dealt } };
        return undefined;
      },
      { game },
    );
    await user.click(await screen.findByRole("button", { name: "Reopen the Blue seat" }));
    const strip = (await screen.findByText(dealt)).closest(".error")!;
    const after = (a: Node, b: Node) => (a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    expect(after(document.querySelector("ol.seat-cards")!, strip), "after the seats").toBe(true);
    expect(after(strip, document.querySelector(".open-table-controls")!), "above the controls").toBe(true);
    expect(document.querySelector(".abandon-refusal"), "not Abandon's strip").toBeNull();
    expect(onStarted).not.toHaveBeenCalled();
    expect(onLeave).not.toHaveBeenCalled();
  });

  it("posts one reopen however fast the button is clicked", async () => {
    let answer: (t: TableView) => void = () => {};
    const { stub } = show(
      (call) => {
        if (call.path.endsWith("/table")) return withAJoiner();
        if (call.path.endsWith("/reopen")) return new Promise<TableView>((r) => (answer = r));
        return undefined;
      },
      { game },
    );
    const button = await screen.findByRole("button", { name: "Reopen the Blue seat" });
    fireEvent.click(button);
    fireEvent.click(button);
    await waitFor(() => expect(stub.matching("POST", "/reopen")).toHaveLength(1));
    await act(async () => answer(wire.tableTwoOpen));
    expect(stub.matching("POST", "/reopen")).toHaveLength(1);
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
      <OpenTable game="wire" onStarted={onStarted} onLeave={() => {}} />,
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
    render(<OpenTable game="wire" onStarted={() => {}} onLeave={onLeave} />);
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
    render(<OpenTable game="wire" onStarted={() => {}} onLeave={onLeave} />);
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
// ---- M6.4: the chat, for a seated caller only -----------------------------

describe("the open table's chat", () => {
  // **A stranger on the seat picker is 403 at this route and must not be shown
  // a panel that refuses** (spec §6.2). The table read is the one route widened
  // to somebody who is not seated (R-2.7); the chat is NOT — so the panel is
  // rendered under the invite, which only a seated caller has, and the
  // stranger's screen asks for nothing at all.
  it("draws the chat panel for a seated caller under the link, and not for a stranger", async () => {
    const { stub } = showSeated(tableIs(seated()));
    const link = await screen.findByLabelText("Invite link");
    const panel = document.querySelector("section.panel.chat")!;
    expect(panel).not.toBeNull();
    expect(panel).toHaveClass("folded");
    // UNDER the invite, in document order — the place the spec names below
    // 976px; from 976px the stylesheet puts it in mock 08's rail (cycle E, row
    // 67), and the cell below is the structure that rests on.
    expect(link.closest(".invite")!.compareDocumentPosition(panel)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
    // The panel's first GET is an effect that runs AFTER the panel mounts, so
    // it is waited for as the neighbours wait (the S1 stack review's
    // re-review: read in the same breath as the link, it raced the effect).
    await waitFor(() => expect(stub.matching("GET", "/chat").length).toBeGreaterThan(0));
    cleanup();

    const stranger = show(tableIs(wire.tableOpen));
    await screen.findByRole("button", { name: "Join as Blue" });
    expect(screen.queryByText("Chat")).toBeNull();
    expect(document.querySelector("section.panel.chat")).toBeNull();
    expect(stranger.stub.matching("GET", "/chat"), "and nothing was asked for").toHaveLength(0);
  });

  // **Three parts, so the chat can take mock 08's rail** (cycle E, straw poll
  // row 67): the table, the chat and the way out, in that order — the page as
  // it always read below 976px — with `with-rail` on a seated caller's page for
  // the stylesheet's two columns from 976px. A stranger has no chat and no
  // rail. jsdom has no layout; `setup_screens.spec.ts` reads the columns.
  it("draws a seated caller's page in three parts, the chat between the table and the way out", async () => {
    showSeated(tableIs(seated()));
    await screen.findByLabelText("Invite link");
    const page = document.querySelector(".setup.open-table")!;
    expect(page).toHaveClass("with-rail");
    expect([...page.children].map((c) => c.className)).toEqual([
      "open-table-main",
      expect.stringMatching(/(^| )chat( |$)/),
      "open-table-foot",
    ]);
    expect(page.querySelector(".open-table-main .invite"), "the invite is the table's").not.toBeNull();
    expect(page.querySelector(".open-table-foot .open-table-controls"), "the controls are the way out").not.toBeNull();
    cleanup();

    show(tableIs(wire.tableOpen));
    await screen.findByRole("button", { name: "Join as Blue" });
    const strangers = document.querySelector(".setup.open-table")!;
    expect(strangers).not.toHaveClass("with-rail");
    expect([...strangers.children].map((c) => c.className)).toEqual(["open-table-main", "open-table-foot"]);
  });

  // **One chat read at mount** (M6.5c2 cycle D, row 130). The panel was handed
  // a tick per wake, and the stream's ON-CONNECT frame is a wake — so every
  // visit read the chat twice before anything was said. The panel reads the
  // frame's own `chat`/`chat_rev` now (`peek`) and is a count host: the
  // connection's resync frame reads the chat only when one of the two has
  // moved past what the mount read holds. **The panel is OPEN** — a folded
  // count host reads nothing whatever the count says, so a folded panel could
  // not tell "nothing moved" from "folded". The frame is the corpus's
  // open-table `Events`; the page, the empty room with the same two numbers.
  // **The frame is written AFTER the panel's mount read**, the order a network
  // gives it (the stream opens behind the transport's health read): written at
  // connect in a stubbed tick, it lands before the panel exists and the cell
  // would witness nothing. The partner: a frame whose count HAS moved (the
  // corpus's own playing reading, `events.json`'s two counters, over the
  // corpus's page) is read once.
  it("reads the chat once at mount, and not again for the connection's own first frame", async () => {
    localStorage.setItem(OPEN_KEY, JSON.stringify(["wire2"]));
    vi.useFakeTimers();
    const stream = fakeStream();
    let page: ChatPage = { ...wire.chat, messages: [], last: wire.eventsOpen.chat, rev: wire.eventsOpen.chat_rev };
    const { stub } = showSeated((call) => {
      if (call.path.endsWith("/table")) return seated();
      if (call.path.endsWith("/stream")) return stream.reply(call);
      if (call.method === "GET" && call.path.includes("/chat")) return page;
      return undefined;
    });
    await until(() => stream.opens() > 0, "the table's stream was opened");
    await until(() => stub.matching("GET", "/chat").length === 1, "the panel's mount read");
    stream.push("signal", wire.eventsOpen);
    await until(() => stub.matching("GET", "/table").length >= 2, "the frame's table read");
    await flush();
    expect(stub.matching("GET", "/chat"), "the resync frame moved nothing").toHaveLength(1);

    page = wire.chat;
    stream.push("signal", { ...wire.eventsOpen, chat: wire.chat.last, chat_rev: wire.chat.rev });
    await until(() => stub.matching("GET", "/chat").length === 2, "a moved count is read");
    await flush();
    expect(stub.matching("GET", "/chat")).toHaveLength(2);
  });

  // **And nothing for its own post** (row 130): the 201 has already moved the
  // cursor to the message the frame then counts. The room starts empty, the
  // post is `chat_post.json`, and the frame the post causes is the corpus's
  // open-table `Events` with its two counters moved to the post's number.
  it("asks the chat nothing for its own post", async () => {
    localStorage.setItem(OPEN_KEY, JSON.stringify(["wire2"]));
    vi.useFakeTimers();
    const stream = fakeStream();
    const { stub } = showSeated((call) => {
      if (call.path.endsWith("/table")) return seated();
      if (call.path.endsWith("/stream")) return stream.reply(call);
      if (call.method === "POST" && call.path.endsWith("/chat")) return { status: 201, body: wire.chatPost };
      if (call.method === "GET" && call.path.includes("/chat")) return { ...wire.chat, messages: [], last: 0, rev: 0 };
      return undefined;
    });
    await until(() => stream.opens() > 0, "the table's stream was opened");
    await until(() => screen.queryByRole("textbox", { name: "Say something" }) !== null, "the chat's input");
    const box = screen.getByRole("textbox", { name: "Say something" });
    fireEvent.change(box, { target: { value: wire.chatPost.text } });
    fireEvent.submit(box.closest("form")!);
    await until(() => stub.matching("POST", "/chat").length === 1, "the post went out");
    await flush();
    expect(stub.matching("GET", "/chat"), "the mount read").toHaveLength(1);
    stream.push("signal", { ...wire.eventsOpen, chat: wire.chatPost.seq, chat_rev: wire.chatPost.seq });
    await until(() => stub.matching("GET", "/table").length >= 2, "the frame's table read");
    await flush();
    expect(stub.matching("GET", "/chat"), "the frame counts the post the 201 drew").toHaveLength(1);
    expect(screen.getByText(wire.chatPost.text)).toBeInTheDocument();
  });
});
