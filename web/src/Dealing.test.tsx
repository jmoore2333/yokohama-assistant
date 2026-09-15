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

import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useSections } from "./BoardsMenu";
import { Dealing } from "./Dealing";
import { stubApi, type ApiCall, type Reply } from "./test/harness";
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
  onLeave?: () => void;
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
    if (call.path.endsWith("/events")) return wire.eventsDealing;
    return undefined;
  };
};

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

  // **The poll, and it is the only way a browser learns that SOMEBODY ELSE
  // finished the dealing.** A keep of my own answers with the table (the cell
  // below); a keep at another browser does not reach this one at all, so the
  // screen asks. Task 9 replaces this `setInterval` with the shared hook; the
  // contract it holds — open the game when `status` leaves `dealing` — is the
  // same one.
  it("opens the game when the signal says playing", async () => {
    vi.useFakeTimers();
    const onPlaying = vi.fn();
    let status = "dealing";
    stubApi((call) => {
      if (call.path.endsWith("/deal")) return wire.dealWaiting;
      if (call.path.endsWith("/events")) return { ...wire.eventsDealing, status };
      return undefined;
    });
    render(<Host onPlaying={onPlaying} />);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(3100);
    });
    expect(onPlaying).not.toHaveBeenCalled();

    status = "playing";
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3100);
    });
    expect(onPlaying).toHaveBeenCalledTimes(1);

    // Once, and then never again: the screen this component is on is being
    // replaced, and a second call would open the game a second time.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(9000);
    });
    expect(onPlaying).toHaveBeenCalledTimes(1);
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
    expect(other.textContent).toContain("blue@wire.test");
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
  it("has a way back to the games list", async () => {
    const onLeave = vi.fn();
    stubApi(dealThen(wire.deal));
    render(<Host onLeave={onLeave} />);
    await screen.findAllByRole("button", { name: /^keep/ });
    await userEvent.setup().click(screen.getByRole("button", { name: "Games" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
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
