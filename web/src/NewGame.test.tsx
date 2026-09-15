// Setup: the deal, and the seed that must survive the round trip.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useSections } from "./BoardsMenu";
import { NewGame } from "./NewGame";
import type { ClientState } from "./types";
import { loadSections } from "./sections";
import { CLAIMED_USER_KEY } from "./api";
import { gamesNamed, gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

/** A seed at the top of the `u64` range — the shape `draw_seed` actually
 *  produces, and the shape a JSON number cannot hold: `Number("18446744073709551557")`
 *  is 18446744073709552000, whose last four digits are not in the seed. */
const BIG_SEED = "18446744073709551557";

/** The screen with a parent holding the sections state, which is where it lives
 *  (R-T12-4: `App` owns it, so the set table and the game agree about which
 *  boards are on). A cell here can therefore still watch a toggle take effect,
 *  and `App.test.tsx` carries the end-to-end claim that the choice survives
 *  Start. */
function Host({ onStarted }: { onStarted: (s: ClientState, opening: boolean) => void }) {
  const [sections, onToggleSection] = useSections();
  return (
    <NewGame
      art={wire.artManifest}
      labels={wire.labels}
      sections={sections}
      onToggleSection={onToggleSection}
      onStarted={onStarted}
    />
  );
}

/** Who this browser is signed in as.
 *
 *  **Every `/api/games` route is a 401 without an identity (M6.1)**, and the
 *  create body now names a PERSON in every human seat — so a component test that
 *  left storage empty would post `{"user": ""}` and assert against a seat nobody
 *  holds. The sign-in screen is a later task of this cycle; this is where its
 *  output already has to exist. */
const ME = "red@wire.test";

function show(handler: (call: ApiCall) => Reply) {
  localStorage.setItem(CLAIMED_USER_KEY, ME);
  const onStarted = vi.fn();
  const stub = stubApi(handler);
  render(<Host onStarted={onStarted} />);
  return { onStarted, stub, user: userEvent.setup() };
}

/** The opening sequence's two replies: a create answers with a DEALING table
 *  and a keep with the table it left behind (M6.1 §4.3). `undefined` for
 *  anything else, so a caller chains it the way `renderApp` chains its own
 *  fallback. */
/** The create posts, and NOT the keeps or the preview beside them.
 *
 *  `matching("POST", "/api/games")` is a substring match, so since M6.1 it also
 *  catches `/api/games/{id}/keep` — three posts where a cell means one. The
 *  create is the only one whose path ENDS there. */
const creates = (stub: ReturnType<typeof stubApi>): ApiCall[] =>
  stub.matching("POST", "/api/games").filter((c) => c.path.endsWith("/api/games"));

function openingReply(call: ApiCall): Reply | undefined {
  if (call.method === "POST" && call.path.endsWith("/api/games")) return wire.tableDealing;
  if (call.path.endsWith("/keep")) return wire.table;
  return undefined;
}

/** Walk the whole set-table sequence: choose the first card at every human
 *  seat, passing the screen on between them.
 *
 *  Start waits for a choice on EVERY human seat, so one click is not enough on
 *  `wire.preview`: that is the bot-free corpus game and BOTH of its seats are
 *  human. Since R-C2 the seats are also taken ONE AT A TIME — only the current
 *  seat draws buttons — so this presses "Next seat" between them rather than
 *  clicking two seats' cards in one pass. The cells that use it are about what
 *  the create request carries and not about which card was chosen, so they take
 *  the first of each seat's two; the sequence itself has its own cell. */
async function chooseAll(user: ReturnType<typeof userEvent.setup>) {
  for (;;) {
    const current = document.querySelector(".deal-seat.human.current");
    if (!current) break;
    await user.click(current.querySelector("button.deal-pick")!);
    const next = screen.queryByRole("button", { name: "Next seat" });
    if (!next) break;
    await user.click(next);
  }
}

/** The set table is on screen. **Not "Start is in the document"**, which is
 *  what these cells used to wait for: since R-C2 Start is not drawn until every
 *  human seat has chosen, so waiting for it would wait for a click that has not
 *  happened. `Re-deal` sits beside it and is there from the first frame. */
const setTableIsUp = () =>
  waitFor(() => expect(screen.getByRole("button", { name: "Re-deal" })).toBeInTheDocument());

describe("a double-clicked Start", () => {
  // **The second site of the guard `App` already documents**, and the one with
  // the worse consequence. `run()` gated on the `busy` STATE, and state is what
  // makes the bug: React batches, so two clicks in the same tick both read
  // `busy === false` and both post. Two `POST /api/games` are then two
  // DIFFERENT games — the server draws a fresh bot seed per request, so with a
  // bot on the start seat each plays its own opening — and before the server
  // learned to serialize creates, the loser's record landed on disk while the
  // client was handed the winner's.
  it("posts create exactly once", async () => {
    const held = gate<unknown>();
    const { stub, user } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed();
      if (call.path.endsWith("/api/games/preview")) return wire.preview;
      if (call.method === "POST" && call.path.endsWith("/api/games")) return held.promise;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await setTableIsUp();

    await chooseAll(user);

    // BOTH clicks inside ONE `act`, so no render happens between them — the
    // case the ref exists for. Two `user.click`s would each flush, the button
    // would already be `disabled`, and the cell would pass with the guard
    // removed, which is not a test of the guard. `App.test.tsx` records the
    // same reasoning for the same mistake.
    const start = screen.getByRole("button", { name: "Start" });
    act(() => {
      fireEvent.click(start);
      fireEvent.click(start);
    });

    expect(creates(stub)).toHaveLength(1);

    held.resolve(wire.tableDealing);
    await waitFor(() => expect(creates(stub)).toHaveLength(1));
  });
});

describe("the seed", () => {
  // **The defect, exactly.** The setup flow ROUND-TRIPS the seed: preview draws
  // one and deals from it, the client shows the deal, and the client sends the
  // seed back to create the game. A seed that went through a JS number on the
  // way would deal a DIFFERENT GAME than the one whose cards the player chose
  // between — the rounding is silent and the game is wrong from move one.
  it("goes back to the server as the exact string the preview returned", async () => {
    // Enough of it to know it did not survive as a number.
    expect(String(Number(BIG_SEED))).not.toBe(BIG_SEED);

    const { stub, user, onStarted } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed();
      if (call.path.endsWith("/api/games/preview")) return { ...wire.preview, seed: BIG_SEED };
      return openingReply(call) ?? wire.options;
    });

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await setTableIsUp();
    expect(screen.getByText(BIG_SEED)).toBeInTheDocument();

    await chooseAll(user);
    await user.click(screen.getByRole("button", { name: "Start" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalled());

    const create = creates(stub);
    expect(create).toHaveLength(1);
    expect((create[0]!.body as { seed: unknown }).seed).toBe(BIG_SEED);
  });

  it("is sent as typed when the player names one, and omitted when they do not", async () => {
    const { stub, user } = show((call) => {
      if (call.method === "GET") return gamesNamed();
      return { ...wire.preview, seed: BIG_SEED };
    });

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(stub.matching("POST", "preview")).toHaveLength(1));
    // Blank means "the server draws one", which is the right default: the board
    // is an injective fingerprint of the setup seed, so a small or sequential
    // seed hands over the hidden set.
    expect(stub.matching("POST", "preview")[0]!.body).not.toHaveProperty("seed");

    await user.click(screen.getByRole("button", { name: "Re-deal" }));
    await user.type(screen.getByLabelText("Seed"), "12345");
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(stub.matching("POST", "preview")).toHaveLength(2));
    expect((stub.matching("POST", "preview")[1]!.body as { seed: string }).seed).toBe("12345");
  });
});

// **The corpus covers RESPONSES only.** `NewGameRequest` and the `{line, expect}`
// input body are hand-written on this side and hand-written again as a `json!`
// literal in `wire_fixtures.rs`, and the server marks `start_seat`, `seed`,
// `bot_seats` and `name` `#[serde(default)]` with no `deny_unknown_fields` — so
// a renamed REQUEST field is not a 400, it is a default, and the game starts
// wrong. Nothing here can close that (it needs a server decision, recorded in
// the M2.5 record §5); what it can do is pin the shape this client actually
// sends, so the client half cannot drift unnoticed.
describe("the request the client sends", () => {
  it("names every field the create endpoint reads, and no others", async () => {
    const { stub, user } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed();
      if (call.path.endsWith("preview")) return wire.preview;
      return openingReply(call) ?? wire.options;
    });
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await setTableIsUp();
    await chooseAll(user);
    await user.click(screen.getByRole("button", { name: "Start" }));

    const create = creates(stub);
    await waitFor(() => expect(create.length).toBeGreaterThan(0));
    // `keeps` and `bot_seats` are gone (M6.1 §4.2): a create can no longer
    // choose a card for anybody, and which seats are bots is said by `seats`.
    expect(Object.keys(create[0]!.body as object).sort()).toEqual(
      ["colors", "players", "seats", "seed", "start_seat"].sort(),
    );
  });
});

// The set table (M5.5 spec §3.3.3, amended by the 2026-09-06 corrections): the
// deal happens over the position the owner's edition has on the table at its
// step 9 — the areas, the port and the tracks, with the two orders dealt and
// the choice taken in secret — and the choice is per HUMAN seat. The
// technologies (its step 10) and the achievements (its step 11) are not on that
// table yet, which is what `withholds the laboratory and the goals` below
// asserts.
// **The form's own subtitle counts the seats (R-C12).** It read "One human
// seat against v4-step9-draft1" at every player count, which the set table one
// click later contradicts: this form seats exactly one bot, so 3P and 4P are
// two and three human seats choosing in turn.
describe("the setup form's subtitle", () => {
  it("says one human seat at two players and counts them above that", async () => {
    // An EMPTY LIST, not a bare `[]`: `GET /api/games` answers `{games: […]}`
    // and the screen reads `.games` off it. A stub answering an array models a
    // server that does not exist — it used to be caught only because the old
    // code called `.map` on `undefined` and the throw landed in a catch.
    const { user } = show(() => gamesNamed());
    const lede = () => document.querySelector("p.lede")!.textContent ?? "";
    expect(lede()).toMatch(/^One human seat against v4-step9-draft1/);
    expect(lede()).not.toMatch(/in turn/);

    await user.selectOptions(screen.getByLabelText("Players"), "3");
    expect(lede()).toMatch(/^Two human seats choosing in turn, against v4-step9-draft1/);

    await user.selectOptions(screen.getByLabelText("Players"), "4");
    expect(lede()).toMatch(/^Three human seats choosing in turn, against v4-step9-draft1/);

    // Back down, so the cell reads a live derivation and not a one-way latch.
    await user.selectOptions(screen.getByLabelText("Players"), "2");
    expect(lede()).toMatch(/^One human seat against/);
  });
});

describe("the set table", () => {
  const dealt = async (fixture: typeof wire.preview) => {
    const shown = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed();
      if (call.path.endsWith("preview")) return fixture;
      return openingReply(call) ?? wire.options;
    });
    await shown.user.click(screen.getByRole("button", { name: "Deal" }));
    await setTableIsUp();
    return shown;
  };

  it("shows the table the rulebook has set when the two orders are dealt: the pyramid and the boards, not a list", async () => {
    await dealt(wire.preview);
    expect(document.querySelectorAll(".board .tile")).toHaveLength(wire.preview.table.frame.board.tiles.length);
    // `.board-strip` and not the section wrapper: `TableSections` renders its
    // "loading…" placeholder under the very same `table-sec ports` class, so a
    // check on the section passes while the strip says it is still fetching —
    // and a set table with no game to fetch FROM is exactly where that would
    // happen. The offers ride on `preview.table`, so they must be drawn.
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    // The laboratory and the goals are WITHHELD until the choice (R-C3) and
    // have their own cell below; the two sections are still on the screen, so
    // this asserts their presence and leaves what they say to that cell.
    expect(document.querySelector(".table-sec.labs")).not.toBeNull();
    expect(document.querySelector(".table-sec.achievements")).not.toBeNull();
    expect(screen.getByText(/setting up/)).toBeInTheDocument();
    expect(screen.getByText(wire.preview.seed)).toBeInTheDocument();
  });

  // The CHOOSING seat's two cards, each with its Keep score and the CLI's
  // hedge under the pair. Since R-C2 that is one seat at a time, so this cell
  // reads the seat that is asking; the sequence that brings the others up is
  // the cell below.
  it("shows the choosing seat's two cards with their Keep scores and the CLI's hedge", async () => {
    await dealt(wire.preview);
    const asking = wire.preview.deals[0]!;
    for (const c of asking.cards) {
      expect(screen.getByRole("button", { name: `keep O${String(c.card.id).padStart(2, "0")} for seat ${asking.seat}` })).toBeInTheDocument();
      expect(screen.getAllByText(new RegExp(`Keep score ${c.keep_score}\\b`)).length).toBeGreaterThan(0);
    }
    expect(screen.getByText(/hand-set, unmeasured, and no substitute for your own read/)).toBeInTheDocument();
    // **No chip rows on a deal card (R-T12-3).** The card ART already prints
    // both the requirements and the rewards, and the two chip rows under it
    // are what pushed the bot's own deal below the fold at 1710x869. The id,
    // the country and the Keep score stay.
    expect(document.querySelectorAll(".deal .chips")).toHaveLength(0);
    // TWO notes, not four: the seat that is not choosing shows backs, which
    // carry no caption at all (R-C2).
    expect(document.querySelectorAll(".deal .card-note")).toHaveLength(2);
  });

  // **The bot's deal is not on this screen (2026-09-06, R-C1).** It is not on
  // the WIRE either — `contract.test.ts` holds that half — and this is what the
  // player sees instead: two face-down cards and a line saying the bot keeps
  // one of them. Nothing here reads a card id, a Keep score or a mark, because
  // the payload carries none.
  //
  // The assertions are stated as ABSENCES on the bot's own section, each paired
  // with the human section's presence, because a screen that drew no deal at
  // all satisfies every absence on its own.
  it("shows the bot's two cards face down, with no id, no score and no mark", async () => {
    await dealt(wire.previewBot);
    const bot = wire.previewBot.deals.find((d) => d.is_bot)!;
    expect(bot.cards, "the fixture's bot seat carries no cards").toHaveLength(0);
    const seat = document.querySelector(`.deal-seat.bot[data-seat="${bot.seat}"]`)!;
    expect(seat).not.toBeNull();
    expect(seat.querySelectorAll("button.deal-pick")).toHaveLength(0);
    expect(seat.querySelectorAll(".card-id")).toHaveLength(0);
    expect(seat.querySelectorAll(".card-note")).toHaveLength(0);
    expect(seat.querySelectorAll(".bot-keeps")).toHaveLength(0);
    expect(seat.textContent).not.toMatch(/Keep score/);
    expect(seat.querySelectorAll("img.back")).toHaveLength(2);
    expect(seat.textContent).toMatch(/keeps one of its two cards . unseen/);
    // Anti-vacuity: the human seat on the SAME payload still prints both of its
    // own card ids, so the absences above are the bot's and not the screen's.
    const human = wire.previewBot.deals.find((d) => !d.is_bot)!;
    const mine = document.querySelector(`.deal-seat.human[data-seat="${human.seat}"]`)!;
    const ids = [...mine.querySelectorAll(".card-id")].map((e) => e.textContent);
    expect(ids).toEqual(human.cards.map((c) => `O${String(c.card.id).padStart(2, "0")}`));
  });

  // **Human seats choose ONE AT A TIME (2026-09-06, R-C2).** One browser serves
  // every seat at a hot-seat table, so the payload carries all their cards and
  // the secrecy is the SEQUENCE: only the seat whose turn it is has its two
  // cards face up, the seats before and after it show backs, and "Next seat"
  // passes the screen on. Hot-seat remains hot-seat — the earlier chooser's
  // cards were on the screen before it was passed — and this is what makes the
  // pass a real one.
  //
  // **Over the committed bot-free preview, which already has two human seats**
  // (R-C8): no payload is constructed here. Three seats would ride the same
  // code path — the seat LIST is what varies — and would need a fixture that
  // does not exist.
  //
  // The two picks are DIFFERENT cards (seat 0's second, seat 1's first), so the
  // `keeps` the create carries cannot be satisfied by a constant.
  it("takes the human seats one at a time, with the seats that are not choosing face down", async () => {
    expect(wire.preview.deals.filter((d) => !d.is_bot), "the premise: two human seats")
      .toHaveLength(2);
    const { user, stub, onStarted } = await dealt(wire.preview);
    const seat = (n: number) => document.querySelector(`.deal-seat.human[data-seat="${n}"]`)!;
    const nextSeat = () => screen.queryByRole("button", { name: "Next seat" });
    const start = () => screen.queryByRole("button", { name: "Start" });

    // Seat 0 is asking; seat 1 is waiting its turn behind two backs.
    expect(seat(0).className).toMatch(/\bcurrent\b/);
    expect(seat(0).querySelectorAll("button.deal-pick")).toHaveLength(2);
    expect(seat(1).className).toMatch(/\bwaiting\b/);
    expect(seat(1).querySelectorAll("button.deal-pick")).toHaveLength(0);
    expect(seat(1).querySelectorAll("img.back")).toHaveLength(2);
    expect(seat(1).querySelectorAll(".card-id")).toHaveLength(0);
    expect(seat(1).textContent).toMatch(/chooses after you/);
    expect(nextSeat(), "nothing to pass on before a card is chosen").toBeNull();
    expect(start(), "Start waits for every seat").toBeNull();

    // Seat 0 chooses its SECOND card, and the screen offers to pass on.
    await user.click(seat(0).querySelectorAll("button.deal-pick")[1]!);
    expect(nextSeat()).not.toBeNull();
    expect(start(), "one seat of two is not every seat").toBeNull();

    // Passed on: seat 0's cards go face down, seat 1's come up.
    await user.click(nextSeat()!);
    expect(seat(0).className).toMatch(/\bchosen\b/);
    expect(seat(0).querySelectorAll("button.deal-pick")).toHaveLength(0);
    expect(seat(0).querySelectorAll("img.back")).toHaveLength(2);
    expect(seat(0).querySelectorAll(".card-id")).toHaveLength(0);
    expect(seat(0).textContent).toMatch(/seat 0 · chosen/);
    expect(seat(1).className).toMatch(/\bcurrent\b/);
    expect(seat(1).querySelectorAll("button.deal-pick")).toHaveLength(2);
    // **Focus follows the pass.** "Next seat" unmounts itself on click, so
    // without this the caret falls to the body and a keyboard player tabs from
    // the top of the page once per seat.
    expect(document.activeElement).toBe(seat(1).querySelectorAll("button.deal-pick")[0]);
    expect(nextSeat(), "the last seat has nobody to pass to").toBeNull();
    expect(start()).toBeNull();

    // The last seat chooses, and only then is there a game to start.
    //
    // **Start's APPEARANCE is the proof that both picks were recorded**, and it
    // is not a weaker claim than reading them off the request: `ready` and
    // `createKeeps` read the same `chosen` map, and `ready` is every human seat
    // having answered. A component that forgot seat 0's pick when the screen
    // was passed on would leave Start undrawn here.
    await user.click(seat(1).querySelectorAll("button.deal-pick")[0]!);
    expect(start()).not.toBeNull();
    await user.click(start()!);
    await waitFor(() => expect(onStarted).toHaveBeenCalled());
    expect(creates(stub)).toHaveLength(1);
    // **The keeps are their own requests now (M6.1 §4.3)**, one per seat, in
    // seat order. Both seats of this fixture are human, and the two differ —
    // seat 0 took its SECOND card and seat 1 its first — so neither a constant
    // nor a first-card default produces this pair, and a form that forgot the
    // earlier seat's pick when the screen was passed on would post one row.
    expect(stub.matching("POST", "/keep").map((c) => c.body)).toEqual([
      { seat: 0, card: 2 },
      { seat: 1, card: 1 },
    ]);
  });

  // **A fresh deal starts at the FIRST seat again (R-C9).** `turn` survives a
  // component that is never unmounted — Re-deal only clears the preview — so
  // without a reset the next deal opens with the second seat asking and the
  // first marked "chosen" though it chose nothing, and `ready` can never become
  // true: Start would never appear at all. The `turn` doc claims the reset;
  // this is what holds it.
  //
  // The walk is the whole round trip, because that is the only way to reach a
  // second deal: pass once, Re-deal back to the form, Deal again.
  it("starts a fresh deal at the first seat after a pass and a re-deal", async () => {
    const { user } = await dealt(wire.preview);
    const seat = (n: number) => document.querySelector(`.deal-seat.human[data-seat="${n}"]`)!;
    await user.click(seat(0).querySelectorAll("button.deal-pick")[0]!);
    await user.click(screen.getByRole("button", { name: "Next seat" }));
    expect(seat(1).className, "the premise: the pass moved the screen on").toMatch(/\bcurrent\b/);

    await user.click(screen.getByRole("button", { name: "Re-deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Deal" })).toBeInTheDocument());
    await user.click(screen.getByRole("button", { name: "Deal" }));
    await setTableIsUp();

    expect(seat(0).className, "a fresh deal asks the first seat").toMatch(/\bcurrent\b/);
    expect(seat(0).querySelectorAll("button.deal-pick")).toHaveLength(2);
    expect(seat(1).className).toMatch(/\bwaiting\b/);
    expect(screen.queryByRole("button", { name: "Next seat" })).toBeNull();
    // And the sequence still finishes: the choice the first deal recorded is
    // gone with it, so Start waits for both seats again rather than appearing
    // one pick early.
    expect(screen.queryByRole("button", { name: "Start" })).toBeNull();
    await user.click(seat(0).querySelectorAll("button.deal-pick")[0]!);
    expect(screen.queryByRole("button", { name: "Start" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Next seat" }));
    await user.click(seat(1).querySelectorAll("button.deal-pick")[0]!);
    expect(screen.queryByRole("button", { name: "Start" })).not.toBeNull();
  });

  // **At one human seat the flow is unchanged in feel** (R-C2's own clause):
  // your two cards, then Start. The sequence exists but has one step, so no
  // "Next seat" is ever drawn — a screen that showed one would be asking a
  // player to pass the table to a bot.
  it("asks the single human seat straight away when the other seat is a bot", async () => {
    const { user } = await dealt(wire.previewBot);
    const humans = document.querySelectorAll(".deal-seat.human");
    expect(humans, "the premise: one human seat and one bot").toHaveLength(1);
    expect(humans[0]!.className).toMatch(/\bcurrent\b/);
    expect(humans[0]!.querySelectorAll("button.deal-pick")).toHaveLength(2);
    expect(screen.queryByRole("button", { name: "Next seat" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Start" })).toBeNull();
    await user.click(humans[0]!.querySelectorAll("button.deal-pick")[0]!);
    expect(screen.queryByRole("button", { name: "Next seat" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Start" })).not.toBeNull();
  });

  // **The technologies and the goals wait for the choice (2026-09-06, R-C3).**
  // The owner's edition deals the two starting orders and takes the secret
  // choice at its step 9; the technologies reach the Laboratory at step 10 and
  // the achievements the score track at step 11 — so neither is on the table
  // while the card is being chosen, and this screen says so rather than drawing
  // an empty strip that reads as a bug.
  //
  // **The withholding is the SERVER's** (the fixture carries no lab slots and
  // no Achievements panel) and the LINE is this screen's, so both halves are
  // asserted: the payload's shape is the premise, the words are the claim.
  //
  // The port and the tracks are the anti-vacuity AND R-C4: they are what stays,
  // so a set table that drew no sections at all fails here.
  it("withholds the laboratory and the goals until the card is chosen, and keeps the port and the tracks", async () => {
    expect(
      wire.preview.table.labs.boards.every((b) => b.slots.length === 0),
      "the premise: the fixture's laboratory is already empty",
    ).toBe(true);
    await dealt(wire.preview);
    for (const key of ["labs", "achievements"]) {
      const sec = document.querySelector(`.table-sec.${key}`)!;
      expect(sec, `the ${key} section is on the set table`).not.toBeNull();
      expect(sec.textContent).toContain("revealed once you have chosen your card");
      expect(sec.querySelectorAll(".card-face"), `${key} draws no cards`).toHaveLength(0);
    }
    // R-C4: what stays. Six order cards in the port, and the tracks drawn.
    expect(document.querySelectorAll(".table-sec.ports .card-face")).toHaveLength(6);
    expect(document.querySelector(".table-sec.tracks")).not.toBeNull();
    expect(document.querySelectorAll(".table-sec.tracks .track")!.length).toBeGreaterThan(0);
  });

  // The backs are the real picture, not a styled placeholder: the manifest's
  // own `backs/order_back.webp`, the file `art.ts` resolves. A back that 404s
  // is a broken image on the set table, and jsdom cannot see that — but it can
  // see the client asking for the wrong file.
  it("draws the bot's backs from the manifest's order back", async () => {
    await dealt(wire.previewBot);
    const backs = [...document.querySelectorAll<HTMLImageElement>(".deal-seat.bot img.back")];
    expect(backs).toHaveLength(2);
    expect(backs.every((b) => b.getAttribute("src") === "/art/backs/order_back.webp")).toBe(true);
    expect(backs.every((b) => b.getAttribute("alt") === "")).toBe(true);
  });

  // **R-T12-2.** Spec §3.3.3 asks for the Boards menu here "so a player can read
  // a site card at full size before choosing". Same component as the game
  // banner's, same storage: what is switched here is what the game opens with.
  it("carries the Boards menu, and switching a board off here is remembered", async () => {
    const { user } = await dealt(wire.preview);
    expect(document.querySelector(".table-sec.ports .board-strip")).not.toBeNull();
    await user.click(screen.getByText(/^Boards/));
    await user.click(screen.getByRole("button", { name: "Port" }));
    expect(document.querySelector(".table-sec.ports")).toBeNull();
    // The strip going is not the claim on its own — a component that dropped it
    // from its own state would pass that. The claim is that the CHOICE survives
    // this screen, which is what `saveSections` is for.
    expect(loadSections()).not.toContain("ports");
    // And the summary counts what is on, with the menu open or shut.
    expect(screen.getByText(/^Boards/)).toHaveTextContent(`Boards · ${loadSections().length} on`);
  });

  it("posts one keep per human seat and none for the bot, numbered as on screen", async () => {
    const { stub, user, onStarted } = await dealt(wire.previewBot);
    await user.click(screen.getAllByRole("button", { name: /keep O\d\d for seat 0/ })[1]!);
    await user.click(screen.getByRole("button", { name: "Start" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, true));
    // **A keep is its own request (M6.1 §4.3)** and the bot has none: its card
    // is the server's own argmax, computed when the table finalizes. `2` is the
    // SECOND card, which no constant and no first-card default produces.
    expect(stub.matching("POST", "/keep").map((c) => c.body)).toEqual([{ seat: 0, card: 2 }]);
  });

  // **The seats the request names, which nothing on this side pinned.**
  //
  // The server refuses a seat that is both a person and the bot, a creator who
  // holds no seat (R-1.6) and a bot at more than two players (R-P10) — each by
  // name — so a client that built this list wrong would fail at the deal with a
  // sentence about a field the player never touched, and only the server would
  // have caught it.
  //
  // **The bot's POSITION is what the second read is for**: the entry has to
  // track the seat the form chose rather than sit at index 1. The screen has no
  // "no bot" option, so every request it can make names exactly one.
  it("names a person in every human seat and the bot in its own, wherever the bot sits", async () => {
    const previewSeats = async (seats?: { players: number; bot: number }) => {
      const { stub, user } = show((call) => {
        if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed();
        return wire.previewBot;
      });
      if (seats) {
        await user.selectOptions(screen.getByLabelText("Players"), String(seats.players));
        await user.selectOptions(screen.getByLabelText("Bot seat"), String(seats.bot));
      }
      await user.click(screen.getByRole("button", { name: "Deal" }));
      await waitFor(() => expect(stub.matching("POST", "preview")).toHaveLength(1));
      return (stub.matching("POST", "preview")[0]!.body as { seats: unknown }).seats;
    };
    expect(await previewSeats()).toEqual([{ user: ME }, { bot: true }]);
    expect(await previewSeats({ players: 3, bot: 2 })).toEqual([
      { user: ME },
      { user: ME },
      { bot: true },
    ]);
  });

  it("resumes a saved game as NOT an opening", async () => {
    const { user, onStarted } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed("wire");
      if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return wire.options;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await user.click(await screen.findByRole("button", { name: "wire" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, false));
  });
});

describe("the saved games", () => {
  it("resumes one with a GET, because a record is not a restore", async () => {
    const { user, onStarted, stub } = show((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return gamesNamed("wire");
      if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return wire.options;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await user.click(await screen.findByRole("button", { name: "wire" }));
    await waitFor(() => expect(onStarted).toHaveBeenCalledWith(wire.options, false));
    expect(stub.matching("GET", "/api/games/wire")).toHaveLength(1);
  });

  it("says so when there are none", async () => {
    show((call) => {
      if (call.method === "GET") return gamesNamed();
      throw new Error("unexpected");
    });
    expect(await screen.findByText(/none yet/)).toBeInTheDocument();
  });

  it("shows a refusal rather than swallowing it", async () => {
    const { user } = show((call) => {
      if (call.method === "GET") return gamesNamed();
      return { status: 400, body: { error: "a game named \"wire\" already exists" } };
    });
    await user.click(screen.getByRole("button", { name: "Deal" }));
    expect(await screen.findByText(/already exists/)).toBeInTheDocument();
  });
});

// ---- the fix wave: deleting a saved game (final review I2) ------------------
//
// **Data loss with no guard, and M5 moved it one menu away from a running
// game.** `delete` beside each saved game removed the record on a single
// click; M5's Game menu put "New game" in the banner, so a player mid-game now
// lands on this list, where a stray click beside the game they meant to resume
// removes a different one. There is no undo — the record is gone.
//
// A two-step BUTTON and not `window.confirm`: a native dialog blocks the
// browser walk this repository tests with, and a modal the page cannot see is
// a modal no cell can assert. The step lives in the row, which is also where
// the mistake is made.
describe("deleting a saved game", () => {
  const twoGames = (call: ApiCall): Reply => {
    if (call.method === "GET" && call.path.endsWith("/api/games"))
      return gamesNamed("wire", "sim-other");
    if (call.method === "DELETE") return null;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  };

  it("does not delete on the first click", async () => {
    const { user, stub } = show(twoGames);
    await user.click(await screen.findByRole("button", { name: "delete wire" }));
    expect(stub.matching("DELETE", "/api/games/wire")).toHaveLength(0);
    // The row is now asking, which is what makes the click above a step and
    // not a no-op.
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
