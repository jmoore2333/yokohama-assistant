// The app shell: what it sends, when it sends it, and what it will not send twice.

import { act, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import { App } from "./App";
import { loadSections } from "./sections";
import { gate, renderApp, stubApi } from "./test/harness";
import { wire } from "./wire/contract";

/** The board tile for one area, as the accessible name renders it. */
const tile = (area: number) =>
  screen.getByRole("button", { name: new RegExp(`^A${String(area).padStart(2, "0")} `) });

describe("the in-flight guard", () => {
  // **The defect, exactly.** The board tile was the one mutation control with
  // no guard. React batches state updates, so two clicks in the same tick both
  // read `busy === false` and both post; the two are then applied in whatever
  // order the server's per-game mutex takes them, and whichever RESPONSE
  // settles last becomes the client's truth. Measured on a double-clicked
  // placement toggle: the client showed `Selected: A03` while the server held
  // an empty selection, and Confirm then advanced the turn having placed
  // nothing.
  //
  // `fireEvent` twice, not `userEvent.click` twice: `userEvent` awaits between
  // interactions, which is precisely the race not happening.
  it("a double-clicked board tile posts exactly once", async () => {
    const held = gate<unknown>();
    const stub = await renderApp(wire.placeA, (call) =>
      call.path.endsWith("/input") ? held.promise : undefined,
    );

    // BOTH clicks inside ONE `act`, so no render happens between them. That is
    // the case the ref exists for and the one `busy` cannot cover: `busy` is
    // state, and a second handler running before React has re-rendered still
    // reads `false`. Two separate `fireEvent` calls would each flush, the tile
    // would already be `disabled`, and the test would pass with the guard
    // removed — which is not a test of the guard.
    const target = tile(0);
    act(() => {
      fireEvent.click(target);
      fireEvent.click(target);
    });

    expect(stub.matching("POST", "/input")).toHaveLength(1);

    held.resolve(wire.placeASelected);
    await waitFor(() => expect(screen.getByText(/This placement is/)).toBeInTheDocument());
    expect(stub.matching("POST", "/input")).toHaveLength(1);
  });

  it("a double-clicked option row posts exactly once", async () => {
    const held = gate<unknown>();
    const stub = await renderApp(wire.optionsMany, (call) =>
      call.path.endsWith("/input") ? held.promise : undefined,
    );

    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    const row = screen.getByRole("button", {
      name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
    });
    act(() => {
      fireEvent.click(row);
      fireEvent.click(row);
    });

    expect(stub.matching("POST", "/input")).toHaveLength(1);
    held.resolve(wire.optionsMany);
    await waitFor(() => expect(stub.matching("POST", "/input")).toHaveLength(1));
  });
});

describe("what goes on the wire", () => {
  // `expect` is what stops a number meaning a different move. An open
  // sub-prompt lives only in the server's memory, so a restart or an eviction
  // rebuilds with no picker open and a stale area id is then read as an option
  // number. Measured across two processes over one store: a client holding
  // `place_a` with areas 3 and 7 selected clicked area 3, and the rebuilt REPL
  // applied option 3 — "Place no assistants".
  it("every input names the prompt the client was looking at", async () => {
    const stub = await renderApp(wire.placeA, (call) =>
      call.path.endsWith("/input") ? wire.placeASelected : undefined,
    );
    await userEvent.setup().click(tile(0));
    await waitFor(() => expect(stub.matching("POST", "/input")).toHaveLength(1));
    expect(stub.matching("POST", "/input")[0]!.body).toEqual({
      line: "A00",
      expect: "place_a",
    });
  });

  it("a board click sends the padded area id the server parses", async () => {
    const stub = await renderApp(wire.placeA, (call) =>
      call.path.endsWith("/input") ? wire.placeASelected : undefined,
    );
    await userEvent.setup().click(tile(7));
    await waitFor(() => expect(stub.matching("POST", "/input")).toHaveLength(1));
    expect((stub.matching("POST", "/input")[0]!.body as { line: string }).line).toBe("A07");
  });
});

describe("the seed on screen", () => {
  // **Inverted, deliberately.** This cell used to assert that the banner
  // printed the lossless string rather than the frame's rounded number. Both
  // fields are gone: the setup seed regenerates both decks, the boxed pile, the
  // board and every deal, so serving it published the whole hidden set to
  // whoever held the payload, and the banner was the ONE line that rendered it.
  //
  // The coverage is not dropped, it is turned around — the corpus fence
  // (`contract.test.ts`) proves no play payload carries the value; this proves
  // the screen does not print one, which is the half a client could reintroduce
  // on its own by deriving it from something else.
  it("is not printed, because no play payload carries one", async () => {
    await renderApp(wire.options);
    expect(screen.queryByText(/seed/i)).not.toBeInTheDocument();
    // The banner is still there and still names the game — the assertion above
    // would pass on a blank page without this.
    expect(screen.getByText(new RegExp(wire.options.id))).toBeInTheDocument();
  });
});

describe("a refusal", () => {
  it("is shown, and the game stays on screen", async () => {
    await renderApp(wire.optionsMany, (call) =>
      call.path.endsWith("/input")
        ? { status: 400, body: { error: "this game is asking \"options\", not \"place_a\"" } }
        : undefined,
    );
    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    await userEvent
      .setup()
      .click(
        screen.getByRole("button", {
          name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
        }),
      );
    await waitFor(() =>
      expect(screen.getByText(/this game is asking/)).toBeInTheDocument(),
    );
    // Still playable: the refusal did not replace the game with an error page.
    // The end-trigger strip is the marker, not the seed — no play payload
    // carries a seed any more, and the strip exists only while a game is on
    // screen, which is exactly the property being asserted.
    expect(screen.getByText(/end triggers:/)).toBeInTheDocument();
  });
});

describe("the board as an input surface", () => {
  // The board is answerable ONLY while a placement picker is open — the one
  // place the engine is asking "which areas" rather than "which option". A
  // tile that is a button at every other prompt is a control that sends a line
  // the server refuses.
  it("is inert outside a placement picker", async () => {
    await renderApp(wire.optionsMany);
    expect(screen.queryByRole("button", { name: /^A00 / })).toBeNull();
  });

  it("is answerable during Option A", async () => {
    await renderApp(wire.placeA);
    expect(tile(0)).toBeEnabled();
  });
});

describe("art", () => {
  // A checkout with no cropped art is a SUPPORTED state: every card falls back
  // to its typed values, which is the surface that existed before there was any
  // art. A 404 must therefore not be an error the player sees.
  it("a server with no art still renders the game", async () => {
    const stub = await renderApp(wire.optionsMany, (call) =>
      call.path.endsWith("/api/art") ? { status: 404, body: { error: "no art here" } } : undefined,
    );
    // The 404 branch was actually ENTERED. The first draft of this test let the
    // harness answer `/api/art` from the corpus before the handler saw it, so
    // it asserted the absence of an error message on a page that had art.
    expect(stub.matching("GET", "/api/art")).toHaveLength(1);
    expect(screen.queryByText(/no art here/)).toBeNull();
    expect(screen.getByRole("heading", { name: "Yokohama" })).toBeInTheDocument();
    // And the typed fallback is what is drawn: the card face carries the id
    // rather than an <img> that would 404.
    expect(document.querySelectorAll("img")).toHaveLength(0);
    expect(document.querySelectorAll(".card-blank").length).toBeGreaterThan(0);
  });

  it("a server WITH art draws it, so the test above is not vacuous", async () => {
    await renderApp(wire.optionsMany);
    expect(document.querySelectorAll("img").length).toBeGreaterThan(0);
  });
});

// ---- M3.5: the answer bar is pinned, and it is ONE bar ----
//
// Measured before this: at 4P the decision zone began at y=2006 on a 2179px
// page — 1,106px below the fold — so answering a prompt meant scrolling to it
// after every move. `.stage` is `max(board, rail)` and the rail is 2.1–2.5× the
// board's height, so no realistic change to the board shortens the page; the
// pin is what makes the page's height stop mattering.
//
// The reason this is a DECK and not a sticky rule on the decision zone is a
// measured defect in the obvious version. `App` renders `className="decision"`
// three times — the warnings strip, the decision itself, the error strip — and
// pinning them independently at one `bottom: 0` puts them on the SAME edge at
// the same z-index. The error strip is last in the DOM, so it paints over the
// bottom of the decision zone: hit-tested, `elementFromPoint` on "Apply the
// only option" and on "Undo" returned `div.error`. Scoping the rule to the
// decision alone then hides the error strip below the fold instead — the player
// gets a permanently visible answer bar and no visible reason their click did
// nothing. One container pins all three together and neither can happen.

describe("the decision deck", () => {
  /** The one state the naive fix breaks: warnings AND an error AND a prompt. */
  const withWarnings = {
    ...wire.optionsMany,
    warnings: ["APPLIED BUT NOT SAVED: the move is in memory only."],
  };

  it("holds the warnings, the prompt and the error in one pinned container", async () => {
    await renderApp(withWarnings, (call) =>
      call.path.endsWith("/input")
        ? { status: 409, body: { error: "Re-read the game and answer what it asks." } }
        : undefined,
    );

    // The warning is on screen before anything is clicked.
    expect(screen.getByText(/APPLIED BUT NOT SAVED/)).toBeInTheDocument();

    // Provoke the error strip, so all three blocks exist at once.
    const state = withWarnings;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    await userEvent.setup().click(
      screen.getByRole("button", {
        name: new RegExp(
          state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
        ),
      }),
    );
    await waitFor(() => expect(screen.getByText(/Re-read the game/)).toBeInTheDocument());

    const deck = document.querySelectorAll(".deck");
    expect(deck, "the three blocks pin as one, or they pin over each other").toHaveLength(1);

    const blocks = [...document.querySelectorAll(".decision")];
    expect(blocks, "warnings, the prompt, and the error").toHaveLength(3);
    for (const b of blocks) {
      expect(deck[0]!.contains(b), `a .decision escaped the deck: ${b.className}`).toBe(true);
    }
  });

  // The clamp goes on the OPTIONS and not on the deck, and that is the whole
  // point of it. Real prompts driven through this API measured 146, 201 and 562
  // option rows — natural heights of 5,721 / 7,827 / 21,646px. With the zone
  // clamped as a block, Confirm and Undo sit at internal scroll offset 22,088
  // inside a 405px box with nothing on the page saying so. With only the list
  // scrolling, every control stays on screen at 562 rows exactly as at three.
  // **The invariant, stated at the width the defect actually had.**
  //
  // The first version of this cell asserted only that `.controls` is not inside
  // `.options`, read on an `options` fixture — and passed while the client was
  // broken. `.options` is OVERLOADED: it is the answer list for `options` and
  // `pick`, and on a `retreat` prompt it is a plain wrapper holding a message
  // and a `.controls` of count buttons. Turning every `.options` into a clipped
  // scroller therefore put the retreat counts inside the clip, and the browser
  // walk found it as `<div class="controls"> intercepts pointer events` — four
  // failing specs, none of them the one this cell was written for.
  //
  // So the claim is now made over EVERY prompt the corpus carries, and it is the
  // claim that actually matters: nothing you can press is inside the box that
  // clips, except the answer rows themselves.
  it.each([
    ["options", wire.options],
    ["many options", wire.optionsMany],
    ["a pick", wire.pick],
    ["a retreat", wire.retreat],
    ["a retreat, answered", wire.retreatAnswered],
  ])("keeps every pressable control out of the scroller: %s", async (_name, state) => {
    await renderApp(state);
    const scrollers = [...document.querySelectorAll<HTMLElement>(".decision .scrolls")];
    for (const box of scrollers) {
      const inside = [...box.querySelectorAll("button")].filter(
        (b) => !b.classList.contains("option"),
      );
      expect(
        inside.map((b) => (b.textContent ?? "").trim()),
        "a control other than an answer row is inside the clipped list",
      ).toEqual([]);
    }
  });

  // Anti-vacuity for the row above: a client that marks NOTHING as scrollable
  // satisfies it trivially, and that is the state this milestone started from.
  // `pick` is the corpus's longest prompt at ten rows — `optionsMany` is named
  // for its two hundred ranked CANDIDATES, not for its three answer rows.
  it("marks the answer list as the thing that scrolls", async () => {
    await renderApp(wire.pick);
    const box = document.querySelector(".decision .options.scrolls");
    expect(box, "the answer list must be the element that scrolls").not.toBeNull();
    expect(
      box!.querySelectorAll(".option").length,
      "the corpus's longest prompt must actually carry rows",
    ).toBeGreaterThan(5);
  });
});

// ---- M3.5: the dead ends this milestone would otherwise have made worse ----

describe("a refused move", () => {
  // **The 409 dead end.** `api.get` had exactly one call site — the setup
  // screen — so once a game was running there was no re-read path at all. The
  // catch branch never called `setState`, so `prompt.kind` stayed stale and
  // every subsequent click re-sent the same `expect` for the same 409, while the
  // server's own message read "Re-read the game and answer what it asks." The
  // only escape was a page reload back into setup.
  //
  // M3.5 makes it worse rather than neutral: with the answer bar pinned, the
  // player stares at a permanently visible row of buttons every one of which
  // refuses.
  it("re-reads the game, and keeps the refusal on screen", async () => {
    const stub = await renderApp(wire.optionsMany, (call) =>
      call.path.endsWith("/input")
        ? { status: 409, body: { error: "Re-read the game and answer what it asks." } }
        : undefined,
    );

    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");

    // **Counted as a DELTA, because the setup screen already did one.**
    // `renderApp` loads the game through `GET /api/games/{id}` to get onto the
    // board at all, so an absolute count is >= 1 before anything is clicked —
    // and the first version of this cell asserted exactly that and passed with
    // the whole re-read deleted. Measured, not assumed.
    const before = stub.matching("GET", `/api/games/${state.id}`).length;
    expect(before, "the setup load is what makes an absolute count vacuous").toBeGreaterThan(0);

    await userEvent.setup().click(
      screen.getByRole("button", {
        name: new RegExp(
          state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
        ),
      }),
    );

    // The refusal is shown — it is the sentence that explains what happened.
    await waitFor(() => expect(screen.getByText(/Re-read the game/)).toBeInTheDocument());
    // AND the truth was fetched, so the screen now agrees with the server.
    await waitFor(() =>
      expect(
        stub.matching("GET", `/api/games/${state.id}`).length,
        "the client never re-read the game the server told it to re-read",
      ).toBeGreaterThan(before),
    );
  });
});

describe("the finished game", () => {
  // `state.scores` is `{seat, total}[]` in SEAT order with no rank and no `won`,
  // so the inline block could only print a colour and a sorted list — a winner
  // mark taken from it would have to be inferred from the margin, and a margin
  // of 0 is not a win. `/result` carries `Placing.won` and the breakdown.
  it("fetches the ranked breakdown and draws the winner", async () => {
    const stub = await renderApp(wire.gameOver, (call) =>
      call.path.endsWith("/result") ? wire.result : undefined,
    );
    await waitFor(() =>
      expect(stub.matching("GET", "/result").length, "the result was never asked for").toBe(1),
    );
    // The same `OutcomeView` the Final score surface draws — never a second
    // table that could disagree with it.
    await waitFor(() =>
      expect(document.querySelector("table.outcome tr.won")).not.toBeNull(),
    );
  });

  it("still shows a standing before /result answers, so the block is never empty", async () => {
    await renderApp(wire.gameOver, (call) =>
      call.path.endsWith("/result") ? { status: 409, body: { error: "still running" } } : undefined,
    );
    await waitFor(() =>
      expect(document.querySelectorAll(".decision .option").length).toBeGreaterThan(0),
    );
  });
});

// ---- M5 W5b: the sections on the table ------------------------------------
//
// The Boards disclosure is the banner's fifth control and the only one whose
// buttons do not open a panel: each is a pressed TOGGLE over `sections.ts`'s
// registry, persisted per browser. The fetched sections then follow `state`,
// which is what makes an enabled Port strip agree with the board beside it.
describe("the section toggles", () => {
  it("are pressed for the default set, and a click turns one on and fetches it", async () => {
    const stub = await renderApp(wire.options, (call) => {
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      return undefined;
    });
    const user = userEvent.setup();
    // Open the disclosure first. jsdom ships no UA stylesheet for `details`, so
    // Testing Library finds a closed menu's buttons either way; the click keeps
    // the test honest about the flow a player actually takes.
    await user.click(screen.getByText(/^Boards/));
    expect(screen.getByRole("button", { name: "Port" })).toHaveAttribute("aria-pressed", "true");
    const sites = screen.getByRole("button", { name: "Building sites" });
    expect(sites).toHaveAttribute("aria-pressed", "false");
    expect(stub.matching("GET", "/areas")).toHaveLength(0);
    await user.click(sites);
    expect(sites).toHaveAttribute("aria-pressed", "true");
    await waitFor(() => expect(stub.matching("GET", "/areas")).toHaveLength(1));
    await user.click(sites);
    expect(sites).toHaveAttribute("aria-pressed", "false");

    // **Escape closes it (ruling R-T12a).** An uncontrolled `<details>` ignores
    // Escape entirely — the key is the browser's own affordance for a `<select>`
    // and a `<dialog>`, not for a disclosure — so a menu left open sits over the
    // table until you click the summary again. Every flow that touches these
    // toggles presses Escape afterwards (this cell, the two browser cells, the
    // screenshot generator), which is what made the absence visible.
    await user.keyboard("{Escape}");
    expect(document.querySelector("details.boards-menu")).not.toHaveAttribute("open");
  });
});

// ---- the fix wave: a reply for a game you left ------------------------------
//
// **Measured against the live client before this cell existed** (final review
// I1): start game A, start game B, delay `/input` by 3s, click B's first
// option, then switch to A from the Game menu — the banner read A after 181ms,
// B's reply landed at 3,104ms, and the client reverted to B. The switch undid
// itself, and the window is exactly a bot turn's think time.
//
// Two fixes, and this cell is about the one the other cannot cover. Disabling
// the saved-game rows while a request is in flight closes the ordering the
// review measured; it does NOT close this one, where the SWITCH is what is in
// flight and the move goes out while the client is still waiting for it. So
// the reply is checked against the game on screen when it lands.
describe("a reply for a game you left", () => {
  it("is dropped, and the game you switched to stays on screen", async () => {
    // Both games are corpus payloads; only the id is renamed, which is the
    // pattern `GameMenu.test.tsx` already uses for a second game. A 4P fixture
    // for the game switched TO, so the board on screen is a fact about which
    // game is drawn and not only about the banner's text.
    const other = { ...wire.options4p, id: "sim-other" };
    const switchGet = gate<unknown>();
    const reply = gate<unknown>();
    const stub = await renderApp(wire.optionsMany, (call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games"))
        return [wire.optionsMany.id, "sim-other"];
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other"))
        return switchGet.promise;
      if (call.path.endsWith("/input")) return reply.promise;
      return undefined;
    });
    const user = userEvent.setup();

    // The switch goes out and is HELD, so it is still in flight for the move.
    await user.click(screen.getByText(new RegExp(wire.optionsMany.id)));
    await user.click(await screen.findByRole("button", { name: "sim-other" }));

    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    await user.click(
      screen.getByRole("button", {
        name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
      }),
    );
    expect(stub.matching("POST", "/input")).toHaveLength(1);

    // The switch lands: the other game is on screen.
    switchGet.resolve(other);
    await waitFor(() => expect(screen.getByText(/sim · sim-other/)).toBeInTheDocument());

    // And now the reply to the move made in the game we left.
    reply.resolve(wire.options);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });

    expect(screen.getByText(/sim · sim-other/)).toBeInTheDocument();
    expect(screen.queryByText(new RegExp(`sim · ${wire.optionsMany.id}$`))).toBeNull();
    // The board is still the game we switched to. Without this the cell would
    // pass on a banner that agreed with a board it no longer describes.
    expect(document.querySelectorAll(".tile")).toHaveLength(other.frame.board.tiles.length);
  });

  it("cannot be started from the Game menu while a move is in flight", async () => {
    const held = gate<unknown>();
    await renderApp(wire.optionsMany, (call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games"))
        return [wire.optionsMany.id, "sim-other"];
      if (call.path.endsWith("/input")) return held.promise;
      return undefined;
    });
    const user = userEvent.setup();
    await user.click(screen.getByText(new RegExp(wire.optionsMany.id)));
    const row = await screen.findByRole("button", { name: "sim-other" });
    expect(row).toBeEnabled();

    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    await user.click(
      screen.getByRole("button", {
        name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
      }),
    );
    expect(row).toBeDisabled();

    held.resolve(wire.options);
    await waitFor(() => expect(row).toBeEnabled());
  });
});

// ---- M5.5 C: the opponent's turn plays out on the table --------------------
//
// The run is the events a reply added; the walk is the run on the table, one
// step at a time, under ONE timer. What these cells drive is `App` — the fetch
// of the positions, the walk's state, and what the screen says while it runs —
// because the reducer and the diff have their own cells already and a second
// statement of them here would pass without the wiring existing.
describe("the turn playback", () => {
  /** A reply that applied THREE events after the state the app answered from,
   *  the second of them by a bot: the fixtures for the run's positions are
   *  the corpus's `at_k` with one seat marked as a bot and a cube moved. */
  /** The id of the technology the bot's take puts in seat 1's list at 43.
   *  Not one the fixture already holds — a repeat would be a second copy of a
   *  card a player may own only one of, and the diff would be right to ignore
   *  it. */
  const TAKEN_TECH = 777;
  /** The order the bot's take puts in the viewer's hand at 43 — not one the
   *  fixture already deals, for the same reason. */
  const TAKEN_ORDER = 31;

  function botReply() {
    const before = { ...wire.optionsMany, events: 40 };
    const replied = { ...wire.optionsMany, events: 43 };
    const at = (k: number) => {
      const f = JSON.parse(JSON.stringify(wire.atK)) as typeof wire.atK;
      f.index = k;
      f.total = 43;
      f.to_move = k === 40 ? before.to_move : 1;
      const players = f.frame.dashboard.find((p) => "Players" in p);
      if (players && "Players" in players) players.Players.players[1]!.is_bot = true;
      // **The cube appears at 43 and not at 42 (orchestrator ruling R-P4).**
      // The diff that carries it then rides the LAST step, so both advances
      // below cross a 400 ms quick dwell and `.tile.changed` is asserted on
      // the step whose own diff names the tile rather than on a later one.
      if (k >= 43) f.frame.board.tiles[0]!.assistants[1]![1] += 1;
      // **And so does a technology (R-GC-1)**, on the same step: a laboratory
      // take marks the card where it LANDED, and that lands in the rail, which
      // is a different component from every other mark on screen. The fixture's
      // seat 1 already owns one, so the panel draws two chips and only one of
      // them is the arrival.
      if (k >= 43) {
        const seats = players && "Players" in players ? players.Players.players : [];
        const owner = seats[1]!;
        const had = owner.technologies[0]!;
        owner.technologies = [
          had,
          { id: TAKEN_TECH, printed: had.printed ? { ...had.printed, id: TAKEN_TECH, name: "Cannery" } : null },
        ];
      }
      // **And an ORDER card lands in the viewer's own hand (R-GC-2)**, which is
      // a THIRD component again — the Hand panel, whose card is marked on its
      // root rather than by a wrapper. The fixture's hand is seat 0's and face
      // up; a masked hand carries no identities and marks nothing by design.
      if (k >= 43) {
        const hand = f.frame.dashboard.find((p) => "Hand" in p);
        if (hand && "Hand" in hand) {
          hand.Hand.cards = [
            ...hand.Hand.cards,
            { ...hand.Hand.cards[0]!, id: TAKEN_ORDER, is_starting_order: false },
          ];
          hand.Hand.card_count += 1;
        }
      }
      return f;
    };
    const log = {
      total: 43,
      entries: [
        { index: 40, text: "Red: End additional-action phase" },
        { index: 41, text: "Blue: End additional-action phase" },
        { index: 42, text: "Blue: Place 1 assistant in each of: Tea Plantation (A00)" },
      ],
    };
    return { before, replied, at, log };
  }

  /** Let the reply's promise chain settle WITHOUT moving the clock.
   *
   *  `waitFor` cannot do this job here, and the reason is measured rather than
   *  stylistic. `@testing-library/dom` decides a clock is fake by looking for a
   *  `jest` global; under vitest there is none, so with `vi.useFakeTimers()`
   *  installed it takes its REAL-timer branch — where its poll `setInterval`
   *  and its own timeout are both faked and neither ever fires, leaving a
   *  MutationObserver as the only thing still running. `shouldAdvanceTime`
   *  fixes the polling and breaks the measurement instead: it races the walk's
   *  400 ms dwell against however long the poll took, so the step the
   *  assertions land on depends on the machine. One real macrotask per round
   *  flushes every pending microtask and advances the fake clock by nothing,
   *  which is what makes each 450 ms advance below mean exactly one step. */
  const settle = async (rounds = 10) => {
    for (let i = 0; i < rounds; i++) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(0);
      });
    }
  };

  /** The pieces the BOARD is drawing, by their accessible names.
   *
   *  This is what says WHICH FRAME is on the table, and it is the only thing
   *  that does: `.table.replaying` is derived from the step's own index and
   *  `ol.account` from the run, so both survive a `view` that has stopped
   *  reading the walk. The fixture is built for it — `wire.optionsMany`, the
   *  LIVE payload here, has a piece on no tile at all, and `wire.atK`, which
   *  every position is made from, has two. Scoped to `.tile` so an open
   *  popover (a second `role="img"`, inside `.board`) can never be counted. */
  const boardPieces = () =>
    [...document.querySelectorAll('.board .tile [role="img"]')].map((e) =>
      e.getAttribute("aria-label"),
    );
  /** The pulse's two attributes, read off the element that carries them.
   *
   *  **`.app`, not `main.table`, and the reason is structural**: `.tile` and
   *  `.strip-slot` are inside the table, but `.seat` and the tech chips are in
   *  the SIDE column, a sibling of it. A custom property cannot be read out of
   *  a sibling subtree, so the duration has to live at the one ancestor of
   *  both — the same element that already carries `data-busy`.
   *
   *  jsdom runs no CSS at all, so what these cells pin is the ATTRIBUTES the
   *  keyframes are selected and timed by; the pulse itself is seen live and in
   *  the gate's pictures. */
  const pulse = () => {
    const app = document.querySelector<HTMLElement>(".app");
    return {
      beat: app?.getAttribute("data-beat") ?? null,
      dwell: app?.style.getPropertyValue("--dwell-ms") || null,
    };
  };

  /** The tile whose id badge reads `id`, as `Board.test.tsx` finds one. */
  const tileOf = (id: string) =>
    [...document.querySelectorAll<HTMLElement>(".tile")].find(
      (t) => t.querySelector(".tile-id")?.textContent === id,
    );

  /** The row this fixture's prompt offers first. */
  const firstRow = () => {
    const state = wire.optionsMany;
    if (state.prompt.kind !== "options") throw new Error("fixture moved");
    return screen.getByRole("button", {
      name: new RegExp(state.prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
    });
  };

  it("fetches the run's positions, walks it when a bot moved, and skip lands live", async () => {
    const { before, replied, at, log } = botReply();
    const stub = await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return replied;
      const m = /\/at\/(\d+)$/.exec(call.path);
      if (m) return at(Number(m[1]));
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    // **The clock goes fake only once the game is on screen.** `renderApp`
    // drives its own `userEvent` with no `advanceTimers`, and user-event awaits
    // a `setTimeout(0)` between interactions — faked and unfired, that hangs.
    vi.useFakeTimers();
    try {
      // **`fireEvent`, not `userEvent`, for both clicks in this cell**, and the
      // reason is measured: `user.click` never returns under vitest's fake
      // clock, `advanceTimers` supplied or not — probed on this fixture, the
      // row found and the click awaited for the whole 8 s budget. `fireEvent`
      // dispatches the same event synchronously and needs no timer at all.
      const click = (el: Element) => act(() => void fireEvent.click(el));
      click(firstRow());
      await settle();
      // Positions 40..=43, in ascending order, and the log's tail of three.
      expect(stub.matching("GET", "/at/")).toHaveLength(4);
      expect(stub.matching("GET", "/at/").map((c) => Number(/\/at\/(\d+)$/.exec(c.path)![1]))).toEqual([40, 41, 42, 43]);
      expect(stub.matching("GET", "/log")[0]!.path).toMatch(/tail=3$/);
      // The walk is on: the deck says who is playing, the table is outlined,
      // the account lights step 1 of 3. The mover of step ONE is the player's
      // own seat — a run is walked whole whenever it contains a bot step — so
      // the strip is read by its count and not by a colour (ruling R-P4).
      expect(screen.getByText(/is playing · step 1 of 3/)).toBeInTheDocument();
      expect(document.querySelector(".table.replaying")).not.toBeNull();
      expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
      expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Red: End additional-action/);
      // **The pulse's beat and its length (R-GC-1).** Step 1 changes nothing on
      // the table — the two "End additional-action phase" events move no piece —
      // so it dwells the quick 400 ms and the pulse runs for exactly that.
      expect(pulse()).toEqual({ beat: "0", dwell: "400ms" });
      // **And the TABLE is drawing the run's frame.** The live payload carries
      // a piece on no tile; every position in the run carries two.
      expect(boardPieces()).toHaveLength(2);
      // An unchanged frame dwells the short dwell, a changed one the cadence.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(450);
      });
      expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Blue: End additional-action/);
      // **The beat ALTERNATES, and that is its whole job**: an element that
      // stays lit across two steps restarts its animation only when the
      // animation NAME changes, and the name is chosen by this attribute.
      expect(pulse()).toEqual({ beat: "1", dwell: "400ms" });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(450);
      });
      expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Place 1 assistant/);
      expect(document.querySelector(".tile.changed")).not.toBeNull();
      // **The take's other end reaches the RAIL.** `App` is the only thing
      // that carries `changes.techs` across from the table's diff to the
      // Players panel, and without that one prop every other assertion in this
      // cell still passes — measured.
      const marked = [...document.querySelectorAll(".chip.tech.changed")];
      expect(document.querySelectorAll(".chip.tech").length).toBe(2);
      expect(marked.map((c) => c.textContent)).toEqual(["Cannery"]);
      // **And the hand card, on a third component again.** One prop each, and
      // dropping either leaves every other assertion in this cell passing —
      // measured for both.
      const handCards = [...document.querySelectorAll(".card-face.order")];
      expect(handCards.filter((c) => c.classList.contains("changed"))).toHaveLength(1);
      expect(
        document.querySelector(".card-face.order.changed")?.querySelector(".card-id")?.textContent,
      ).toBe(`O${String(TAKEN_ORDER).padStart(2, "0")}`);
      // The step that MOVES something dwells the cadence — 2000 ms, the
      // default this browser has no stored choice against — so the pulse is
      // five times the bookkeeping step's and the marks stay up between.
      expect(pulse()).toEqual({ beat: "0", dwell: "2000ms" });
      // The step's own `after` frame, read off the tile: the cube this fixture
      // adds at position 43 and nowhere else is ON A00, and the live payload
      // says that tile is empty.
      expect(tileOf("A00")?.querySelector('[role="img"]')?.getAttribute("aria-label")).toBe(
        "1 Blue assistant",
      );
      expect(boardPieces()).toHaveLength(3);
      // Skip: live prompt back, no outline, the account keeps the run and
      // offers replay. Scoped to the deck strip because the account's header
      // carries a `skip` of its own while the walk runs (Task 8), and both
      // dispatch the same action — an unscoped role lookup is ambiguous, not
      // wrong.
      const deck = within(document.querySelector<HTMLElement>(".decision.walking")!);
      click(deck.getByRole("button", { name: "skip" }));
      expect(screen.queryByText(/is playing/)).toBeNull();
      expect(document.querySelector(".table.replaying")).toBeNull();
      expect(screen.getByRole("button", { name: "replay" })).toBeInTheDocument();
      expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
      // **The board is the LIVE state again.** Only the marks outlive the walk
      // (`fresh`, spec §3.2.4); the frame does not, which is what "skip lands
      // you on your prompt" means for the table.
      expect(boardPieces()).toHaveLength(0);
      expect(document.querySelector(".tile.changed")).not.toBeNull();
      // **And the pulse does not re-run under `fresh`.** The marks held on the
      // live table are the LAST step's, which is the step the walk was on, so
      // both attributes keep the values they already had — a changed beat here
      // would restart the animation on a step that already played.
      expect(pulse()).toEqual({ beat: "0", dwell: "2000ms" });
    } finally {
      vi.useRealTimers();
    }
  });

  // **The switch clears the walk, and the bug it stops is visible.** Without
  // it the account of a turn in the game being left goes on rendering under
  // the new banner, and — because the walk owns `view` — the OLD game's board
  // is what the table draws.
  it("drops the run and the walk when the Game menu switches games", async () => {
    const { before, replied, at, log } = botReply();
    const other = { ...wire.options4p, id: "sim-other" };
    await renderApp(before, (call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return [before.id, "sim-other"];
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other")) return other;
      if (call.path.endsWith("/input")) return replied;
      const m = /\/at\/(\d+)$/.exec(call.path);
      if (m) return at(Number(m[1]));
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    vi.useFakeTimers();
    try {
      const click = (el: Element) => act(() => void fireEvent.click(el));
      click(firstRow());
      await settle();
      // The walk is running, and the table is drawing its frame.
      expect(screen.getByText(/is playing · step 1 of 3/)).toBeInTheDocument();
      expect(boardPieces()).toHaveLength(2);

      click(screen.getByText(new RegExp(before.id)));
      await settle();
      click(screen.getByRole("button", { name: "sim-other" }));
      await settle();

      expect(screen.queryByText(/is playing/)).toBeNull();
      expect(document.querySelector("ol.account")).toBeNull();
      expect(document.querySelector(".table.replaying")).toBeNull();
      expect(document.querySelector(".tile.changed")).toBeNull();
      // And the board is the game switched TO — the assertion the other three
      // would pass without, since `view` is what the walk actually holds.
      expect(document.querySelectorAll(".tile")).toHaveLength(other.frame.board.tiles.length);
    } finally {
      vi.useRealTimers();
    }
  });

  it("does not walk a run of the player's own events, and shows the prose when nothing was applied", async () => {
    const { before, replied, at, log } = botReply();
    const ownOnly = (k: number) => {
      const f = at(k);
      f.to_move = before.to_move;
      const players = f.frame.dashboard.find((p) => "Players" in p);
      if (players && "Players" in players) for (const p of players.Players.players) p.is_bot = false;
      return f;
    };
    let reply: unknown = replied;
    const stub = await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return reply;
      const m = /\/at\/(\d+)$/.exec(call.path);
      if (m) return ownOnly(Number(m[1]));
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    const user = userEvent.setup();
    await user.click(firstRow());
    await waitFor(() => expect(stub.matching("GET", "/at/")).toHaveLength(4));
    expect(screen.queryByText(/is playing/)).toBeNull();
    expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
    // A reply with the SAME count (a picker step) shows its prose instead.
    reply = { ...replied, messages: ["Selected: A01", "-> legal — Place 1 assistant in each of: Church (A01)"] };
    await user.click(firstRow());
    await waitFor(() => expect(screen.getByText(/Selected: A01/)).toBeInTheDocument());
    expect(document.querySelector("ol.account")).toBeNull();
  });

  // **The root's in-flight marker, and the gap it closes.**
  //
  // The browser harness reads a screen only once the client is settled, and its
  // only signal used to be "no disabled control in the deck" — which says
  // nothing on a screen that has no control. The finished game is exactly that
  // screen: its deck is a standing. So a read could land after the reply and
  // before the run's positions arrived, and the walk then replaced the deck a
  // moment later. Measured on the whole-game browser cell: it reached its
  // winner, read "The game is over.", and failed ten seconds afterwards
  // looking for the same sentence.
  it("marks the root until the RUN has landed, not just until the reply has", async () => {
    const { before, replied, at, log } = botReply();
    const heldAt = gate<unknown>();
    await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return replied;
      if (/\/at\/\d+$/.test(call.path)) return heldAt.promise;
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    expect(document.querySelector("[data-busy]")).toBeNull();
    const user = userEvent.setup();
    await user.click(firstRow());
    // The reply has landed and the positions have been asked for; the marker is
    // what says this answer is not finished with.
    await waitFor(() => expect(document.querySelectorAll("[data-busy]")).toHaveLength(1));
    expect(document.querySelector(".app[data-busy]")).not.toBeNull();
    heldAt.resolve(at(40));
    await waitFor(() => expect(document.querySelector("[data-busy]")).toBeNull());
  });

  it("keeps the cadence the player chose, through storage", async () => {
    await renderApp(wire.optionsMany);
    await userEvent.setup().selectOptions(screen.getByLabelText("Cadence"), "1000");
    expect(localStorage.getItem("yokohama.playback.cadence")).toBe("1000");
  });
});

// ---- the fix wave: one sections state (M5.5 ruling R-T12-4) -----------------
//
// **The defect the extraction introduced, exactly.** `useSections()` seeds from
// storage at MOUNT. Calling it in `App` and again in `NewGame` made two
// independent copies: the set table's menu wrote storage and updated its own,
// while `App` — which never remounts, `NewGame` being its child — went on
// holding the array it read at start-up and handed THAT to the game's
// `TableSections`. So a strip switched off during setup came back the moment
// the game opened, and stayed until a reload. Both files carried a comment
// saying the opposite.
//
// The cell drives the whole flow rather than the storage, because storage was
// the half that was already right.
describe("the boards a player switches during setup", () => {
  it("stay switched when the game opens", async () => {
    const stub = stubApi((call) => {
      if (call.method === "GET" && call.path.endsWith("/api/games")) return [];
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/api/art")) return wire.artManifest;
      if (call.path.endsWith("preview")) return wire.previewBot;
      if (call.method === "POST" && call.path.endsWith("/api/games")) return wire.options;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      if (/\/at\/\d+$/.test(call.path)) return wire.atK;
      if (/\/log(\?|$)/.test(call.path)) return wire.log;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    render(<App />);
    const user = userEvent.setup();

    await user.click(screen.getByRole("button", { name: "Deal" }));
    await waitFor(() => expect(screen.getByRole("button", { name: "Start" })).toBeInTheDocument());
    expect(document.querySelector(".table-sec.ports")).not.toBeNull();

    await user.click(screen.getByText(/^Boards/));
    await user.click(screen.getByRole("button", { name: "Port" }));
    expect(document.querySelector(".table-sec.ports")).toBeNull();

    await user.click(screen.getAllByRole("button", { name: /^keep O\d\d for seat 0/ })[0]!);
    await user.click(screen.getByRole("button", { name: "Start" }));
    await waitFor(() => expect(screen.getByText(/end triggers:/)).toBeInTheDocument());

    // The claim: the GAME opens with the board the player switched off still
    // off. The laboratory is asserted present in the same breath, so a cell
    // that passed because no section rendered at all would fail here.
    expect(document.querySelector(".table-sec.labs")).not.toBeNull();
    expect(document.querySelector(".table-sec.ports")).toBeNull();
    expect(loadSections()).not.toContain("ports");
    // And the port was never fetched for the game either — the section being
    // absent is not the same as its request not going out.
    expect(stub.matching("GET", "/ports")).toHaveLength(0);
  });
});
