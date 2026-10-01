// The dealing screen: the set table, and the ONE seat this browser is being
// asked about.
//
// **What moved, and why the cells moved with it (M6.1 §7.4).** Until this cycle
// the deal was a hot-seat pass inside the setup screen: one payload carried
// every seat's two cards, and the only thing keeping seat 1's deal from seat 0
// was that the client had not drawn it yet. The server now serves `GET …/deal`
// per CALLER — `seat` is the lowest seat this person holds that has not kept —
// so the secrecy is the payload's, and the sequence a person holding two seats
// still walks is what the server hands back after each keep, not a `turn`
// index this component maintains.
//
// So the cells here are about three things the old ones could not be about:
// what the DOM holds (my two faces and no other seat's), what the client posts
// for one seat, and how this screen learns that the table has started.

import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useSections } from "./BoardsMenu";
import { Dealing } from "./Dealing";
import { ApiError, DISPLAY_NAME_KEY } from "./api";
import { BACKOFF_MS, endSentence } from "./signal";
import { fakeStream, flush, until, stubApi, type ApiCall, type Reply } from "./test/harness";
import { seatName } from "./tables";
import { wire } from "./wire/contract";
import type { DealView } from "./types";

/** The screen with a parent holding the sections state, the shape
 *  `NewGame.test.tsx`'s own host has and for the same reason (R-T12-4): `App`
 *  owns the enabled set, so a cell here can still watch a toggle take effect
 *  without this component keeping a second copy of it. */
function Host({
  game = "wire",
  me = "red@wire.test",
  onPlaying = () => {},
  onLeave = () => {},
}: {
  game?: string;
  me?: string;
  onPlaying?: () => void;
  onLeave?: (sentence?: string) => void;
}) {
  const [sections, onToggleSection] = useSections();
  return (
    <Dealing
      game={game}
      me={me}
      art={wire.artManifest}
      labels={wire.labels}
      sections={sections}
      onToggleSection={onToggleSection}
      onPlaying={onPlaying}
      onLeave={onLeave}
    />
  );
}

/** The two deals the corpus carries, in the order a keep walks them: my seat's
 *  cards, then the "every seat I hold has chosen" reading. */
const dealThen = (...replies: DealView[]) => {
  const queue = [...replies];
  return (call: ApiCall): Reply | undefined => {
    if (call.path.endsWith("/deal")) return queue.shift() ?? replies[replies.length - 1];
    const q = quietChat(call);
    if (q !== undefined) return q;
    // The signal is a stream since M6.3, and a cell that is not about it does
    // not answer one at all: `stubApi` holds an unscripted stream open and
    // silent, which is what a reachable server with nothing to report does.
    return undefined;
  };
};

/** The table's change stream, and the frames a cell writes on it. `Dealing`
 *  declares `frames: "value"`, so the frame's own body IS the `Events` this
 *  screen reads — there is no second request behind it (R-7.1). */
const signalling = (onOpen?: (open: number) => Array<[string, unknown]>) => {
  const stream = fakeStream(onOpen);
  return {
    stream,
    at: (call: ApiCall): Reply | undefined =>
      call.path.endsWith("/stream") ? stream.reply(call) : quietChat(call),
  };
};

/** **The side column draws a chat panel since M6.4 (spec §6.2)**, and it asks
 *  for the table's messages on mount whatever its fold state — so every cell in
 *  this file now makes that request whether or not it is about the chat. A
 *  table where nothing has been said, derived from the corpus's own page rather
 *  than typed out here, so a cell that is not about the chat has none of it in
 *  its DOM. The one cell that IS answers the route itself. */
const quietChat = (call: ApiCall): Reply | undefined =>
  call.method === "GET" && call.path.includes("/chat")
    ? { ...wire.chat, messages: [], last: 0 }
    : undefined;

/** Let a reply chain settle without moving the clock — the harness's own
 *  [`flush`]: a frame arrives over turns that are not all microtasks, and NOT
 *  after an interval. */
const settle = flush;

afterEach(() => {
  vi.useRealTimers();
});

describe("the dealing screen", () => {
  it("draws my two cards over the set table, posts my keep, and re-asks for the next of my seats", async () => {
    // **Built ONCE, outside the handler**: `dealThen` holds the queue that makes
    // the second `/deal` a different answer from the first, and a factory called
    // per request would hand back the first answer for ever.
    const deals = dealThen(wire.deal, wire.dealWaiting);
    const stub = stubApi((call) => {
      // **The keep's reply is the table it left behind, still DEALING**, which
      // is the case this cell is about: another seat is owed a deal. The
      // fixture is the corpus's own `deal_waiting.table` — a table whose first
      // seat has kept and whose second has not — rather than `wire.table`,
      // which reads `playing` and would contradict the "waiting on" line two
      // lines below.
      if (call.path.endsWith("/keep")) return wire.dealWaiting.table;
      return deals(call);
    });
    render(<Host />);

    const picks = await screen.findAllByRole("button", { name: /^keep O\d\d/ });
    expect(picks).toHaveLength(2);
    // The hedge is the SERVER's `keep_note` — `wizard::KEEP_SCORE_NOTE`, served
    // rather than copied (§4.3) — so the sentence beside the Keep score cannot
    // drift from the terminal's. Collapsed, because the note is wrapped for a
    // terminal and arrives with its newlines in it.
    const note = wire.deal.keep_note.trim().replace(/\s+/g, " ");
    expect(screen.getByText(note.slice(0, 30), { exact: false })).toBeInTheDocument();

    await userEvent.setup().click(picks[1]!);
    // The SECOND card, which no constant and no first-card default produces.
    expect(stub.matching("POST", "/keep")[0]!.body).toEqual({ seat: wire.deal.seat, card: 2 });

    // And the screen re-asks: one more `GET …/deal`, whose answer here is the
    // caller's seats all kept, so the screen waits on the ones that have not.
    await screen.findByText(/waiting on/);
    expect(stub.matching("GET", "/deal")).toHaveLength(2);
  });

  // **The signal, and it is the only way a browser learns that SOMEBODY ELSE
  // finished the dealing.** A keep of my own answers with the table (the cell
  // below); a keep at another browser does not reach this one at all, so the
  // screen listens. M6.3 replaced the transport under it and the contract it
  // holds — open the game when `status` leaves `dealing` — is the same one.
  it("opens the game when the signal says playing", async () => {
    vi.useFakeTimers();
    const onPlaying = vi.fn();
    const { stream, at } = signalling();
    stubApi((call) => {
      if (call.path.endsWith("/deal")) return wire.dealWaiting;
      return at(call);
    });
    render(<Host onPlaying={onPlaying} />);
    // **Connected first**, gated on the fact: a frame written to a body nobody
    // has opened reaches nobody, and nothing re-delivers it.
    await until(() => stream.opens() > 0, "the table's stream was opened");

    stream.push("signal", wire.eventsDealing);
    await settle();
    expect(onPlaying).not.toHaveBeenCalled();

    stream.push("signal", { ...wire.eventsDealing, status: "playing" });
    await settle();
    expect(onPlaying).toHaveBeenCalledTimes(1);

    // Once, and then never again: the screen this component is on is being
    // replaced, and a second call would open the game a second time.
    stream.push("signal", { ...wire.eventsDealing, status: "playing" });
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(9000);
    });
    expect(onPlaying).toHaveBeenCalledTimes(1);
  });

  // **A hand-over that FAILS must be retried, and the latch is what nearly
  // stopped it.** `started` exists so `App` is told exactly once — the keep's
  // reply, a 409 from `/deal` and the poll can all notice the same table — but
  // set before `onPlaying` has succeeded it also latches on a FAILURE. The game
  // is running server-side by then (the last keep started it), so a 500 or a
  // dropped connection on the read that opens it would strand this browser on a
  // dealing screen for a table that is not dealing, with no way forward but the
  // "Lobby" button. The latch is released on failure and taken again on the next
  // attempt, so its purpose — one hand-over per SUCCESS — is unchanged.
  it("retries the hand-over when it fails, and stops asking once it succeeds", async () => {
    vi.useFakeTimers();
    const onPlaying = vi
      .fn()
      // A server error, not a 401: `api.ts` moves the door on a 401 before the
      // refusal is thrown, so that case never reaches this screen at all.
      .mockRejectedValueOnce(new ApiError("the game could not be read", 500))
      .mockResolvedValue(undefined);
    const { stream, at } = signalling();
    stubApi((call) => {
      if (call.path.endsWith("/deal")) return wire.dealWaiting;
      return at(call);
    });
    render(<Host onPlaying={onPlaying} />);
    await until(() => stream.opens() > 0, "the table's stream was opened");

    stream.push("signal", { ...wire.eventsDealing, status: "playing" });
    await settle();
    expect(onPlaying).toHaveBeenCalledTimes(1);
    // The player is told, rather than left on a screen that has quietly given up.
    expect(screen.getByText(/could not be read/)).toBeInTheDocument();

    stream.push("signal", { ...wire.eventsDealing, status: "playing" });
    await settle();
    expect(onPlaying).toHaveBeenCalledTimes(2);

    // And now it stops: the game is open, and a third attempt would open it
    // again. This is the half the latch is actually for.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(9000);
    });
    expect(onPlaying).toHaveBeenCalledTimes(2);
  });

  // **The keep that FINISHES the dealing opens the game at once.** The last
  // human keep finalizes the table server-side and the reply says so, so a
  // client that waited for the poll would sit on a dealing screen for up to
  // three seconds with a game already running behind it — and would then ask
  // `GET …/deal`, which is a 409 once the table is playing.
  it("opens the game when my own keep was the last one", async () => {
    const onPlaying = vi.fn();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/keep")) return wire.table;
      return dealThen(wire.deal)(call);
    });
    render(<Host onPlaying={onPlaying} />);
    const picks = await screen.findAllByRole("button", { name: /^keep O\d\d/ });
    await userEvent.setup().click(picks[0]!);

    await waitFor(() => expect(onPlaying).toHaveBeenCalledTimes(1));
    // And the deal is NOT asked for again — the table is playing, and that
    // route refuses a playing table by contract (§4.3).
    expect(stub.matching("GET", "/deal")).toHaveLength(1);
  });

  // **The whole point of the route (§4.3), read off the DOM.** One browser used
  // to hold every seat's cards and draw the ones whose turn it was; now it
  // holds ONE seat's, and the others are a line of text. So the absence is
  // asserted as a count of card faces rather than as "the other section is not
  // rendered" — a screen that drew nothing satisfies that on its own, which is
  // why the two faces are counted in the same breath.
  it("never draws another seat's cards: the DOM holds exactly my two card faces", async () => {
    stubApi(dealThen(wire.deal));
    const { container } = render(<Host />);
    await screen.findAllByRole("button", { name: /^keep/ });
    expect(container.querySelectorAll(".deal .card-face.order:not(.back)")).toHaveLength(2);
    expect(container.querySelectorAll(".deal-seat.human .card-face.back")).toHaveLength(0);
    // The other human seat is on screen as a ROW — who holds it, and whether it
    // has answered — which is what `TableView.chosen` is for: whether a seat
    // has chosen is public at any table; which card it took is not.
    const other = container.querySelector('.deal-seat.human.other[data-seat="1"]')!;
    expect(other).not.toBeNull();
    // **The NAME that seat's holder gave, not the address the table is keyed
    // by** (spec §5.6). Derived through `seatName` rather than restated, so the
    // one place a seat is worded is the one place this can drift from.
    const seat1 = wire.deal.table.seats[1]!;
    expect(seatName(seat1)).toBe("Blue");
    expect(other.textContent).toContain(seatName(seat1));
    expect(other.textContent).toContain("waiting");
  });

  it("draws the set table the choice is made over: the pyramid, the port, and the two withheld sections", async () => {
    stubApi(dealThen(wire.deal));
    render(<Host />);
    await screen.findAllByRole("button", { name: /^keep/ });
    expect(document.querySelectorAll(".board .tile")).toHaveLength(
      wire.deal.frame.frame.board.tiles.length,
    );
    // `.board-strip` and not the section wrapper: `TableSections` renders its
    // "loading…" placeholder under the very same class, and a set table with no
    // game to fetch FROM is exactly where that would pass while saying nothing.
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    // R-C3: the owner's edition deals and takes the secret choice at its step 9,
    // and the technologies (10) and the achievements (11) are not on the table
    // yet — so the server sends neither and both sections carry a line.
    for (const key of ["labs", "achievements"]) {
      const sec = document.querySelector(`.table-sec.${key}`)!;
      expect(sec, `the ${key} section is on the set table`).not.toBeNull();
      expect(sec.textContent).toContain("revealed once you have chosen your card");
      expect(sec.querySelectorAll(".card-face"), `${key} draws no cards`).toHaveLength(0);
    }
    // **No seed anywhere on this screen.** The setup seed regenerates both
    // decks, the boxed pile and every deal, and `DealView` carries none — so
    // this is the client half of the fence the corpus holds on the server's.
    expect(document.body.textContent).not.toMatch(/seed/i);
  });

  // The bot's two cards are face down and the line says so — the picture a
  // player has of the seat across the table (R-C1). **The seats are the one
  // field moved off the corpus payload**, because no committed `deal` fixture
  // seats a bot: `SeatDoc`'s `{kind: "bot"}` arm is type-checked against
  // `types.ts` here exactly as the corpus is under `tsc`, and it is the
  // smallest override that makes the claim.
  it("shows a bot seat's two cards face down, with no id and no Keep score", async () => {
    const withABot: DealView = {
      ...wire.deal,
      table: {
        ...wire.deal.table,
        seats: [wire.deal.table.seats[0]!, { kind: "bot" }],
        chosen: [false, true],
      },
    };
    stubApi(dealThen(withABot));
    const { container } = render(<Host />);
    await screen.findAllByRole("button", { name: /^keep/ });
    const bot = container.querySelector('.deal-seat.bot[data-seat="1"]')!;
    expect(bot).not.toBeNull();
    expect(bot.querySelectorAll("img.back")).toHaveLength(2);
    expect(bot.querySelectorAll(".card-id")).toHaveLength(0);
    expect(bot.textContent).not.toMatch(/Keep score/);
    expect(bot.textContent).toMatch(/keeps one of its two cards . unseen/);
    // Anti-vacuity on the same payload: my own two ids are still printed, so
    // the absences above are the bot's and not the screen's.
    expect(container.querySelectorAll(".deal-seat.human.current .card-id")).toHaveLength(2);
  });

  it("carries the Boards menu, so a board can be read at full size before the choice", async () => {
    stubApi(dealThen(wire.deal));
    render(<Host />);
    await screen.findAllByRole("button", { name: /^keep/ });
    const user = userEvent.setup();
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    await user.click(screen.getByText(/^Boards/));
    await user.click(screen.getByRole("button", { name: "Port" }));
    expect(document.querySelector(".table-sec.ports")).toBeNull();
  });

  // **A screen with a way off it.** The M5.5 set table had "Re-deal"; a table is
  // persisted now, so leaving abandons nothing — the row is in the lobby and
  // opens straight back here. Without this the only way out of a table that is
  // waiting on somebody else is a page reload.
  it("has a way back to the lobby", async () => {
    const onLeave = vi.fn();
    stubApi(dealThen(wire.deal));
    render(<Host onLeave={onLeave} />);
    await screen.findAllByRole("button", { name: /^keep/ });
    // The word itself is the assertion (live review M-3): every other screen's
    // control back to this destination says "Lobby", and this one said "Games".
    await userEvent.setup().click(screen.getByRole("button", { name: "Lobby" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  // **A table that is GONE stops being polled, and says so** (final review M2).
  // The reachable case is the creator abandoning from a lobby row (R-13) while
  // the deal is up at somebody ELSE's browser: without an `onEnd` the poll ends
  // silently and the deal sits there until the next click 404s. `App` and
  // `OpenTable` already carried these three lines; this consumer did not.
  //
  // **This cell drives a DROPPED CONNECTION and a 404 on the reopen** — a
  // server restart, not a delete (final review I-2). The abandon itself writes
  // an `end` FRAME, which is the cell below; the title used to claim that path
  // and this one never took it. The sentence here is the server's own, off the
  // refusal that ends the ladder.
  it("leaves with the refusal's sentence when the reopen finds the table gone", async () => {
    vi.useFakeTimers();
    const onLeave = vi.fn();
    // **The SERVER's sentence, over the stream's own refusal.** A table
    // abandoned under this screen refuses the open with a 404, and the reason
    // and the sentence both come off that refusal.
    let gone = false;
    const { stream, at } = signalling();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/deal")) return wire.dealWaiting;
      if (gone && call.path.endsWith("/stream"))
        return { status: 404, body: { error: "no such game" } };
      return at(call);
    });
    render(<Host onLeave={onLeave} />);
    await until(() => stream.opens() > 0, "the table's stream was opened");

    stream.push("signal", wire.eventsDealing);
    await settle();
    // Anti-vacuity: a live table is still being waited on.
    expect(onLeave).not.toHaveBeenCalled();

    // The table goes, and the connection this screen is holding ends with it.
    gone = true;
    stream.close();
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 100);
    });
    await settle();
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(onLeave.mock.calls[0]![0]).toBe("no such game");

    // And it is over: a permanent 404 asked again is a client waiting for an
    // answer it already has.
    const opens = stub.matching("GET", "/stream").length;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(stub.matching("GET", "/stream")).toHaveLength(opens);
  });

  // **A REAL abandon, which is an `end` frame and carries no words** (final
  // review I-2). `Signals::end` writes `{"reason":"deleted"}` and nothing else,
  // and this screen hands `end.sentence` straight to `onLeave` — so before the
  // fix the creator abandoning a table took everyone else's dealing screen to
  // the lobby in SILENCE, while the cell above claimed otherwise about a path
  // it did not drive. The words are `signal.ts`'s `endSentence`, one place for
  // all four consumers.
  it("leaves with a sentence when the abandon arrives as an end frame", async () => {
    vi.useFakeTimers();
    const onLeave = vi.fn();
    const { stream, at } = signalling();
    stubApi((call) => {
      if (call.path.endsWith("/deal")) return wire.dealWaiting;
      return at(call);
    });
    render(<Host onLeave={onLeave} />);
    await until(() => stream.opens() > 0, "the table's stream was opened");
    expect(onLeave).not.toHaveBeenCalled();

    stream.push("end", { reason: "deleted" });
    await settle();
    expect(onLeave).toHaveBeenCalledTimes(1);
    expect(onLeave.mock.calls[0]![0], "the lobby is told WHY, not nothing at all").toBe(
      endSentence("deleted"),
    );
    expect(onLeave.mock.calls[0]![0]).toMatch(/abandoned/);
  });

  // ---- the name on the keep (spec §5.6) ------------------------------------
  //
  // **The last moment this browser can say what it is called.** The form asks
  // the creator and the open table's seat picker asks whoever sits down there,
  // but a seat filled at create time BY EMAIL reaches this screen never having
  // been asked — and the table would then list that person by their address for
  // the life of the game. The server writes the name onto a seat that has none,
  // so a keep can never rename somebody.
  it("posts the name this browser remembers with the keep", async () => {
    localStorage.setItem(DISPLAY_NAME_KEY, "Jeremy");
    const stub = stubApi((call) => {
      if (call.path.endsWith("/keep")) return wire.dealWaiting.table;
      return dealThen(wire.deal, wire.dealWaiting)(call);
    });
    render(<Host />);
    const picks = await screen.findAllByRole("button", { name: /^keep O\d\d/ });
    await userEvent.setup().click(picks[0]!);
    await waitFor(() => expect(stub.matching("POST", "/keep")).toHaveLength(1));
    expect(stub.matching("POST", "/keep")[0]!.body).toEqual({
      seat: wire.deal.seat,
      card: 1,
      name: "Jeremy",
    });
  });

  it("posts no name key at all when this browser has none", async () => {
    // **The KEY, not the value.** `{name: undefined}` serializes away, so an
    // assertion on the value would pass for a client that sent the key — and
    // this API refuses an unknown key rather than defaulting it.
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/keep")) return wire.dealWaiting.table;
      return dealThen(wire.deal, wire.dealWaiting)(call);
    });
    render(<Host />);
    const picks = await screen.findAllByRole("button", { name: /^keep O\d\d/ });
    await userEvent.setup().click(picks[0]!);
    await waitFor(() => expect(stub.matching("POST", "/keep")).toHaveLength(1));
    expect(stub.matching("POST", "/keep")[0]!.body).toEqual({
      seat: wire.deal.seat,
      card: 1,
    });
  });

  // And the other end of §5.6, on the screen that reads it: the seats this
  // table is waiting on are NAMED. The corpus's second seat is "Blue" at
  // `blue@wire.test`, so a client reading the wrong field would say the address.
  it("names the seats it is waiting on, rather than their addresses", async () => {
    stubApi(dealThen(wire.dealWaiting));
    render(<Host />);
    const waiting = await screen.findByText(/waiting on/);
    const blue = wire.dealWaiting.table.seats[1]!;
    expect(blue.kind).toBe("human");
    expect(seatName(blue)).toBe("Blue");
    expect(waiting).toHaveTextContent("waiting on Blue");
    expect(waiting.textContent).not.toMatch(/@/);
  });

  it("shows a refusal rather than a blank screen", async () => {
    stubApi((call) =>
      call.path.endsWith("/deal")
        ? { status: 403, body: { error: "you do not hold a seat at this table" } }
        : undefined,
    );
    render(<Host />);
    expect(await screen.findByText(/you do not hold a seat/)).toBeInTheDocument();
  });
});
// ---- M6.4: the chat on the dealing screen ---------------------------------

describe("the dealing screen's chat", () => {
  // **The table exists before the game does**, which is the whole reason this
  // screen hosts a panel at all: the people at it are waiting on each other,
  // and the count rides the same `useSignal` frame the screen already reads for
  // its own hand-over. LAST in the side column and FOLDED (R-6.4.10), the game
  // screen's own rule — and `sendName={false}`, because everyone here holds a
  // seat and a seat is named by its seat (R-6.4.2).
  it("draws the chat panel last in the side column, folded", async () => {
    stubApi((call) =>
      call.method === "GET" && call.path.includes("/chat") ? wire.chat : dealThen(wire.deal)(call),
    );
    render(<Host />);
    await screen.findAllByRole("button", { name: /^keep O\d\d/ });
    const side = document.querySelector("aside.side")!;
    await waitFor(() => expect(side.lastElementChild).toHaveClass("chat"));
    expect(side.lastElementChild).toHaveClass("folded");
    expect(within(side as HTMLElement).getByRole("button", { name: "show" })).toBeInTheDocument();
  });
});
