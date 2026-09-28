// The app shell: what it sends, when it sends it, and what it will not send twice.

import { act, cleanup, fireEvent, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import { App } from "./App";
import {
  CLAIMED_USER_KEY,
  DISPLAY_NAME_KEY,
  chatText,
  lobbyChat,
  rememberServerFacts,
  setChatText,
  setLobbyChat,
} from "./api";
import { resetFactsForTest } from "./local/facts";
import { resetLocalHostForTests } from "./local/client";
import { loadSections } from "./sections";
import { currentWorker, FakeWorker, stubWorker, type FakeWorkerHooks } from "./test/fakeWorker";
import { fakeStream, flush, until, gameMenu, gamesNamed, gate, isGamesList, openStream, playOneLocalMove, positionsWalked, renderApp, runRange, signedOutReply, stubApi, stubLocalGame, type ApiCall, type ApiStub, type Reply } from "./test/harness";
import { resetPersistenceRequestForTests } from "./storage";
import { stubStorage } from "./test/platform";
import { wire } from "./wire/contract";
import type { ClientState, Events, TableView } from "./types";

/** The board tile for one area, as the accessible name renders it. */
const tile = (area: number) =>
  screen.getByRole("button", { name: new RegExp(`^A${String(area).padStart(2, "0")} `) });

/** The first answer in the deck, whatever the prompt's rows happen to say.
 *
 *  A query by CLASS, because the cells that use it are about what the click
 *  SENDS rather than about which move it was: matching on the row's own words
 *  would tie them to a fixture's wording for no gain. */
const firstOptionButton = () =>
  document.querySelector<HTMLButtonElement>("button.option:not([disabled])")!;

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
    expect(gameMenu()).toHaveTextContent(wire.options.id);
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
      call.path.endsWith("/art/manifest.json") ? { status: 404, body: { error: "no art here" } } : undefined,
    );
    // The 404 branch was actually ENTERED. The first draft of this test let the
    // harness answer `/art/manifest.json` from the corpus before the handler saw it, so
    // it asserted the absence of an error message on a page that had art.
    expect(stub.matching("GET", "/art/manifest.json")).toHaveLength(1);
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

/** A Blob's text through `FileReader`, which jsdom implements where `Blob.text`
 *  may not be. */
const readBlob = (blob: Blob): Promise<string> =>
  new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsText(blob);
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

  // **Save game file** (M6.5a, R-6.5a.13). The record is served as TEXT and is
  // handed to the download as that text: the Blob's bytes are the served string,
  // and `JSON.parse` is spied to prove the string never went through it (a
  // u64 seed would come back rounded). The anchor's click is intercepted rather
  // than allowed, because jsdom cannot navigate — and its `download` and `href`
  // are what the spy reads.
  it("offers Save game file once the game is over, and hands the record text to the download untouched", async () => {
    const record = '{"seed":18446744073709551615,"events":[]}';
    const parse = vi.spyOn(JSON, "parse");
    const clicks: { href: string; download: string }[] = [];
    const click = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(function (this: HTMLAnchorElement) {
        clicks.push({ href: this.href, download: this.download });
      });
    const create = vi.fn().mockReturnValue("blob:record");
    const revoke = vi.fn();
    Object.defineProperty(URL, "createObjectURL", { value: create, configurable: true });
    Object.defineProperty(URL, "revokeObjectURL", { value: revoke, configurable: true });
    try {
      const stub = await renderApp(wire.gameOver, (call) => {
        if (call.path.endsWith("/result")) return wire.result;
        if (call.path.endsWith("/record")) {
          return new Response(record, { status: 200, headers: { "content-type": "application/json" } });
        }
        return undefined;
      });
      const user = userEvent.setup();
      await user.click(await screen.findByRole("button", { name: "Save game file" }));
      await waitFor(() => expect(clicks).toHaveLength(1));
      expect(stub.matching("GET", "/record")).toHaveLength(1);
      const [first] = clicks;
      expect(first?.download).toBe(`${wire.gameOver.id}.json`);
      expect(first?.href).toContain("blob:record");
      const blob = create.mock.calls[0]?.[0] as Blob;
      expect(blob.type).toBe("application/json");
      expect(await readBlob(blob)).toBe(record);
      expect(parse.mock.calls.some(([s]) => s === record), "the record text went through JSON.parse").toBe(false);
      await waitFor(() => expect(revoke).toHaveBeenCalledWith("blob:record"));
    } finally {
      click.mockRestore();
      parse.mockRestore();
    }
  });

  it("offers no Save game file while the game runs", async () => {
    await renderApp(wire.options);
    await screen.findByText(/to move/);
    expect(screen.queryByRole("button", { name: "Save game file" })).toBeNull();
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
      if (isGamesList(call, "mine"))
        return gamesNamed(wire.optionsMany.id, "sim-other");
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other"))
        return switchGet.promise;
      if (call.path.endsWith("/input")) return reply.promise;
      return undefined;
    });
    const user = userEvent.setup();

    // The switch goes out and is HELD, so it is still in flight for the move.
    await user.click(gameMenu());
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
      if (isGamesList(call, "mine"))
        return gamesNamed(wire.optionsMany.id, "sim-other");
      if (call.path.endsWith("/input")) return held.promise;
      return undefined;
    });
    const user = userEvent.setup();
    await user.click(gameMenu());
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

  /** R-6.5b.10's client half: a run longer than the server's cap is PAGED in
   *  cap-sized calls — `from=40&to=104` (sixty-four positions) and then
   *  `from=104&to=111` (the last seven, ending on the live frame) — with ONE
   *  log call for the whole tail. A client that sent the whole range would be
   *  refused (the server never clamps), and one that stopped at the cap would
   *  walk a run seven steps short. */
  it("pages a run longer than the cap in cap-sized run calls and reads the log once", async () => {
    const { before, at } = botReply();
    const replied = { ...before, events: 110 };
    const log = {
      total: 110,
      entries: Array.from({ length: 70 }, (_, i) => ({ index: 40 + i, text: `Blue: step ${i}` })),
    };
    const stub = await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return replied;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => ({ ...at(k), total: 110 })) };
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    vi.useFakeTimers();
    try {
      const click = (el: Element) => act(() => void fireEvent.click(el));
      click(firstRow());
      await settle();
      expect(stub.matching("GET", "/at?").map((c) => c.path.replace(/^.*\/at\?/, ""))).toEqual([
        "from=40&to=104",
        "from=104&to=111",
      ]);
      expect(positionsWalked(stub)).toHaveLength(71);
      expect(stub.matching("GET", "/log")).toHaveLength(1);
      expect(stub.matching("GET", "/log")[0]!.path).toMatch(/tail=70$/);
    } finally {
      vi.useRealTimers();
    }
  });

  it("fetches the run's positions, walks it when a bot moved, and skip lands live", async () => {
    const { before, replied, at, log } = botReply();
    const stub = await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return replied;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k)) };
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
      // Positions 40..=43, in ascending order, from ONE run call (M6.5b:
      // `to` is `next + 1`, the route being half-open), and the log's tail of three.
      expect(stub.matching("GET", "/at?")).toHaveLength(1);
      expect(stub.matching("GET", "/at?")[0]!.path).toMatch(/\/at\?from=40&to=44$/);
      expect(positionsWalked(stub)).toEqual([40, 41, 42, 43]);
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

  /** A reply whose LAST step diffs to NOTHING — the shape EVERY real bot turn
   *  has, and the one shape the R-P4 fixture above deliberately does not.
   *
   *  Three events again, but the changing one is in the MIDDLE: the human's own
   *  answer at 40, the bot's placement at 41, and the bot's "End
   *  additional-action phase" at 42, whose before and after frames are
   *  identical. Marks read off the last step alone are therefore empty — which
   *  is the defect the final review reproduced live three times, while the cell
   *  above stayed green because its cube rides the run's final step. */
  function botReplyEndingEmpty() {
    const before = { ...wire.optionsMany, events: 40 };
    const replied = { ...wire.optionsMany, events: 43 };
    const at = (k: number) => {
      const f = JSON.parse(JSON.stringify(wire.atK)) as typeof wire.atK;
      f.index = k;
      f.total = 43;
      // Position 40 is the human's own; every later one is the bot's, so the
      // run's LAST step's mover is seat 1 and the held union is taken over its
      // two steps and not over the human's.
      f.to_move = k === 40 ? before.to_move : 1;
      const players = f.frame.dashboard.find((p) => "Players" in p);
      if (!players || !("Players" in players)) throw new Error("no Players panel");
      players.Players.players[1]!.is_bot = true;
      // The cube lands at 42 — the position AFTER event 41 — so the last step
      // (42 -> 43) compares two identical frames. The SEAT's own counts move
      // with it, or the diff would name the tile and never the row.
      if (k >= 42) {
        f.frame.board.tiles[0]!.assistants[1]![1] += 1;
        const blue = players.Players.players[1]!;
        blue.assistants = {
          ...blue.assistants,
          in_hand: blue.assistants.in_hand - 1,
          on_board: blue.assistants.on_board + 1,
        };
      }
      return f;
    };
    const log = {
      total: 43,
      entries: [
        { index: 40, text: "Red: End additional-action phase" },
        { index: 41, text: "Blue: Place 1 assistant in each of: Tea Plantation (A00)" },
        { index: 42, text: "Blue: End additional-action phase" },
      ],
    };
    return { before, replied, at, log };
  }

  /** The marks on screen, in the two grains this fixture lights. */
  const marks = () => ({
    tiles: [...document.querySelectorAll(".tile.changed")].map(
      (t) => t.querySelector(".tile-id")?.textContent ?? "?",
    ),
    seats: [...document.querySelectorAll(".seat.changed .seat-name")].map((s) => s.textContent),
  });

  /** Render the empty-ending fixture and run the walk up to its last step. */
  const walkToTheEmptyStep = async (click: (el: Element) => void) => {
    click(firstRow());
    await settle();
    // Step 1 of 3 is the human's own answer, and it changes nothing.
    expect(screen.getByText(/is playing · step 1 of 3/)).toBeInTheDocument();
    expect(marks()).toEqual({ tiles: [], seats: [] });
    // Step 2 is the placement, and it is the only step of this run that moves
    // anything at all.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(450);
    });
    expect(document.querySelector("ol.account li.at")?.textContent).toMatch(/Place 1 assistant/);
    expect(marks()).toEqual({ tiles: ["A00"], seats: ["Blue"] });
    // Step 3 is the bot's "End additional-action phase": its own diff is empty,
    // and the table goes dark for it while the walk is still running. That is
    // the step's OWN diff and is correct; what must not happen is the run
    // ending there with nothing held.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2100);
    });
    expect(document.querySelector("ol.account li.at")?.textContent).toMatch(
      /Blue: End additional-action/,
    );
    expect(marks()).toEqual({ tiles: [], seats: [] });
  };

  // **The turn's marks outlive the walk, and the last step is not what they
  // are (spec §3.2.4, ruling R-F1).** Every bot turn this server serves ends
  // with "End additional-action phase", whose diff is empty, so holding the
  // LAST step's changes cleared every mark the instant the walk finished —
  // measured live three times at the final review. What is held is the UNION
  // over the steps the run's last mover played, coloured by that mover, so
  // what the opponent did is still on the board you are about to play on.
  it("holds the mover's whole turn on the live table after the walk ticks out", async () => {
    const { before, replied, at, log } = botReplyEndingEmpty();
    await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return replied;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k)) };
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    vi.useFakeTimers();
    try {
      const click = (el: Element) => act(() => void fireEvent.click(el));
      await walkToTheEmptyStep(click);

      // The walk ticks out. The table is the live position again — the live
      // payload has a piece on no tile — and the marks the turn left are on it.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(450);
      });
      expect(screen.queryByText(/is playing/)).toBeNull();
      expect(boardPieces()).toHaveLength(0);
      expect(marks()).toEqual({ tiles: ["A00"], seats: ["Blue"] });

      // And they clear at the player's NEXT answer, not before: the reply adds
      // no event, so the run goes too and the box shows the prose again.
      click(firstRow());
      await settle();
      expect(marks()).toEqual({ tiles: [], seats: [] });
      expect(document.querySelector("ol.account")).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  // `skip` is the other way a walk ends, and it ends it from a step that is not
  // the last one — so a held mark read off "the step we stopped on" would be
  // right here and wrong above. Both ways hold the same thing.
  it("holds the same marks when the walk is skipped from the empty last step", async () => {
    const { before, replied, at, log } = botReplyEndingEmpty();
    await renderApp(before, (call) => {
      if (call.path.endsWith("/input")) return replied;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k)) };
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    vi.useFakeTimers();
    try {
      const click = (el: Element) => act(() => void fireEvent.click(el));
      await walkToTheEmptyStep(click);

      const deck = within(document.querySelector<HTMLElement>(".decision.walking")!);
      click(deck.getByRole("button", { name: "skip" }));
      expect(screen.queryByText(/is playing/)).toBeNull();
      expect(boardPieces()).toHaveLength(0);
      expect(marks()).toEqual({ tiles: ["A00"], seats: ["Blue"] });
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
      if (isGamesList(call, "mine"))
        return gamesNamed(before.id, "sim-other");
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other")) return other;
      if (call.path.endsWith("/input")) return replied;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k)) };
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

      click(gameMenu());
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
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => ownOnly(k)) };
      if (call.path.includes("/log")) return log;
      return undefined;
    });
    const user = userEvent.setup();
    await user.click(firstRow());
    await waitFor(() => expect(positionsWalked(stub)).toHaveLength(4));
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
      if (runRange(call.path)) return heldAt.promise;
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
    heldAt.resolve({ frames: [40, 41, 42, 43].map((k) => at(k)) });
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
// ---- opening a table (M6.1 §7.4) -------------------------------------------
//
// **A table exists before its game does**, and every session route refuses one
// that is still dealing with a 409. So `App` asks `GET …/table` FIRST and picks
// the screen from the status — which is the whole reason a saved row hands up an
// ID rather than a game.
describe("opening a saved table", () => {
  /** The screens in front of a game, with the table's status the one thing that
   *  moves. The saved list names one table, so the row is the way in. */
  const openWith = async (table: unknown, extra: (call: ApiCall) => Reply | undefined = () => undefined) => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = stubApi((call) => {
      const mine = extra(call);
      if (mine !== undefined) return mine;
      if (call.path.endsWith("/api/me")) return wire.me;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (isGamesList(call, "mine")) return gamesNamed("wire");
      // The lobby asks for all THREE scopes (spec §5.2, R-3.9); nothing here is
      // open and nothing is being watched.
      if (isGamesList(call, "open")) return gamesNamed();
      if (isGamesList(call, "playing")) return gamesNamed();
      if (call.path.endsWith("/api/games/wire/table")) return table;
      if (call.path.endsWith("/deal")) return wire.deal;
      if (call.path.endsWith("/events")) return wire.eventsDealing;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map(() => wire.atK) };
      if (/\/log(\?|$)/.test(call.path)) return wire.log;
      return undefined;
    });
    // The server's tables are the LOBBY's since M6.5b2 — `#/` is the front
    // door, which lists the games in this browser instead (R-6.5b2.1).
    window.location.hash = "#/lobby";
    render(<App />);
    // The lobby row is a LINK since M6.2 (R-14).
    await userEvent.setup().click(await screen.findByRole("link", { name: "wire" }));
    return stub;
  };

  it("reads its table first and shows the dealing screen when it is dealing", async () => {
    const stub = await openWith(wire.tableDealing);
    await screen.findByRole("heading", { name: "Choose your card" });
    // **And the game itself was never asked for.** That request is a 409 while
    // the table is dealing, so a client that sent it would greet the player with
    // a refusal on exactly the tables this screen exists for.
    expect(
      stub.calls.filter((c) => c.method === "GET" && c.path.endsWith("/api/games/wire")),
    ).toHaveLength(0);
  });

  it("opens the game when the same table reads playing, so the branch above is a branch", async () => {
    const stub = await openWith(wire.table, (call) =>
      call.method === "GET" && call.path.endsWith("/api/games/wire") ? wire.options : undefined,
    );
    await screen.findByText(/end triggers:/);
    expect(
      stub.calls.filter((c) => c.method === "GET" && c.path.endsWith("/api/games/wire")),
    ).toHaveLength(1);
    expect(screen.queryByRole("heading", { name: "Choose your card" })).toBeNull();
  });
});

// ---- the hot-seat pass, in the game (M6.1 §7.5) ----------------------------
//
// **A move that ends a turn answers its own mover with a `watching` prompt.**
// That is the true thing to say to the person who has just played — the reply is
// built for the seat that POSTED — and at a table with other people in it the
// poll is what brings their move back. At a table where the NEXT seat is also
// this person's, it leaves a browser holding an unanswerable screen for a turn
// that never left it. `TableView.my_seats` is the server's own answer to which
// of the two this is, and it is why `App` holds the table at all.
describe("a move that ends the turn", () => {
  /** Open the game, then answer once; `seats` is what the table says this
   *  caller holds. The `/input` reply is the corpus's own `watching` payload —
   *  seat 1's view of seat 0's turn — so the re-read is asked for the seat the
   *  game is actually waiting on. */
  const answerOnce = async (seats: number[]) => {
    const stub = await renderApp(wire.options, (call) => {
      // **`my_seats` is the one field moved off the corpus payload**, and it is
      // the one the server varies per caller: the committed table can only
      // carry one caller's reading, and a hot seat is the same person in two.
      if (call.path.endsWith(`/api/games/${wire.options.id}/table`))
        return { ...wire.table, my_seats: seats };
      if (call.path.endsWith("/input")) return wire.watching;
      return undefined;
    });
    await userEvent.setup().click(firstOptionButton());
    return stub;
  };
  const reads = (stub: Awaited<ReturnType<typeof answerOnce>>) =>
    stub.calls.filter(
      (c) => c.method === "GET" && c.path.endsWith(`/api/games/${wire.options.id}`),
    );

  it("re-reads for the next seat when this browser holds it", async () => {
    const stub = await answerOnce([0, 1]);
    // Two: the read that opened the game, and the pass.
    await waitFor(() => expect(reads(stub)).toHaveLength(2));
    // And the screen is answerable again, which is the point of the re-read
    // rather than the request being made.
    expect(screen.queryByText(/Nothing here is yours to answer/)).toBeNull();
    expect(firstOptionButton()).toBeInTheDocument();
  });

  it("does not re-read when the next seat is somebody else's", async () => {
    const stub = await answerOnce([1]);
    // The opening read, and nothing since: the poll is what brings the other
    // player's move back, and a client that re-read here would spend a request
    // per move earning the same watching payload.
    await screen.findByText(/Nothing here is yours to answer/);
    expect(reads(stub)).toHaveLength(1);
  });
});

describe("the boards a player switches during setup", () => {
  it("stay switched when the game opens", async () => {
    // Signed in, because since M6.1 there is a door in front of every screen
    // and this cell drives `App` directly rather than through `renderApp`.
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    // **The table's status MOVES, because the flow crosses it twice.** `App`
    // reads `GET …/table` on the way into the dealing screen and again on the
    // way into the game, so a stub answering one fixed status would either
    // refuse to open the deal or send the dealing screen straight back to
    // itself. The keep that finishes the dealing is what flips it, which is
    // exactly what the server does.
    let dealt = false;
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/me")) return wire.me;
      if (isGamesList(call)) return gamesNamed();
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      // The opening sequence (M6.1 §§4.2–4.3): a create answers 201 with a
      // DEALING table, the dealing screen fetches the caller's own two cards,
      // and the keep that finishes the dealing answers with the table at
      // `playing` — which is when the GAME is read back.
      if (call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games"))
        return { status: 201, body: wire.tableDealing };
      if (call.path.endsWith("/table")) return dealt ? wire.table : wire.tableDealing;
      if (call.path.endsWith("/deal")) return wire.deal;
      if (call.path.endsWith("/keep")) {
        dealt = true;
        return wire.table;
      }
      if (call.path.endsWith("/events")) return wire.eventsDealing;
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.options.id}`))
        return wire.options;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map(() => wire.atK) };
      if (/\/log(\?|$)/.test(call.path)) return wire.log;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    // The form is the SERVER's, and so is the screen it is opened from: `#/` is
    // the front door since M6.5b2 (R-6.5b2.1).
    window.location.hash = "#/lobby";
    render(<App />);
    const user = userEvent.setup();

    // `find`, not `get`: until `/api/me` answers the lobby draws only
    // "Connecting to the server…", so its New table is one round trip away
    // rather than there on first paint — and the form is one click beyond it
    // since M6.2 (spec §5.1).
    await user.click(await screen.findByRole("button", { name: "New table" }));
    await user.click(await screen.findByRole("button", { name: "Deal" }));
    // The dealing screen's own heading: a create opens a TABLE, and the deal is
    // a screen of its own since M6.1, so this is what says the set table
    // arrived.
    await screen.findByRole("heading", { name: "Choose your card" });
    expect(document.querySelector(".table-sec.ports")).not.toBeNull();

    await user.click(screen.getByText(/^Boards/));
    await user.click(screen.getByRole("button", { name: "Port" }));
    expect(document.querySelector(".table-sec.ports")).toBeNull();

    // One keep, and it is the last: the table comes back at `playing`, and the
    // dealing screen hands `App` the game.
    await user.click(screen.getAllByRole("button", { name: /^keep O\d\d for seat 0/ })[0]!);
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

// ---- the hash is the screen (M6.2 §5.1) -----------------------------------
//
// **One effect turns a route into a screen**, and these are the four things it
// has to get right: where a signed-in player lands, what each hash draws, what a
// hash the server refuses does, and — the one that is not a screen at all — that
// a table is not opened twice because `open` navigates before it fetches.

describe("the route", () => {
  // **The local host is a module SINGLETON, and two cells here start one.**
  // The door draws "On this device", which asks this tab's Worker for its
  // games; `stubWorker` clears the fakes it has built but not the singleton
  // holding one, so without this the local-create cell below finds a host from
  // the door cell above it and `currentWorker()` has nothing to hand back. The
  // shape is `a local game`'s own.
  beforeEach(() => {
    resetLocalHostForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
  });

  /** `App`, signed in, over a stub that answers the bootstrap and both lobby
   *  scopes from the corpus. */
  const app = (handler: (call: ApiCall) => Reply | undefined = () => undefined) => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = stubApi((call) => {
      const mine = handler(call);
      if (mine !== undefined) return mine;
      if (call.path.endsWith("/api/me")) return wire.me;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (call.path.endsWith("/api/health")) return wire.health;
      // A screen with a signal opens one; a cell about the signal scripts its
      // own above, and this is the resting answer for the rest.
      if (call.path.endsWith("/stream")) return openStream();
      if (isGamesList(call, "open")) return gamesNamed();
      if (isGamesList(call, "playing")) return gamesNamed();
      if (isGamesList(call, "mine")) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    // **The LOBBY unless the cell says otherwise.** The suite's `beforeEach`
    // empties the hash, which reads as the front door since M6.5b2 — the one
    // screen in this client that is drawn from the browser alone. Every cell
    // below that is about the server's screens says so by starting here; the
    // two that are about the door set their own hash or assert the empty one.
    if (!window.location.hash) window.location.hash = "#/lobby";
    render(<App />);
    return { stub, user: userEvent.setup() };
  };

  // **The first screen is the DOOR, and everyone sees it** (R-6.5b2.1). It is
  // drawn from this browser alone — the Worker's own listing and a link — so it
  // renders in front of the sign-in gate rather than behind it, which is what
  // lets the shell be fetched without a login at all.
  it("lands on the door, which draws both ways on", async () => {
    stubWorker();
    // Said out loud, because `app()` above starts its other cells at the lobby:
    // this is the one that is about the screen a bare `#/` draws.
    window.location.hash = "#/";
    app();
    expect(await screen.findByRole("heading", { name: "On this device" })).toBeInTheDocument();
    // A LINK and never a fetch: Access answers a navigation with a redirect to
    // another origin, which a `fetch` follows and then fails opaquely.
    expect(screen.getByRole("link", { name: "Go to the lobby" })).toHaveAttribute(
      "href",
      "/enter",
    );
    // And the lobby is not this screen, which is the half that moved.
    expect(screen.queryByRole("heading", { name: "Lobby" })).toBeNull();
    expect(window.location.hash).toBe("#/");
  });

  it("draws the lobby at #/lobby, and #/new draws the form", async () => {
    window.location.hash = "#/lobby";
    const { user } = app();
    // The server's first screen is the LOBBY, not the form — which is M6.2's
    // product change, kept: a player with three games open used to be shown a
    // blank create.
    expect(await screen.findByRole("heading", { name: "Lobby" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "New table" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "New table" }));
    expect(window.location.hash).toBe("#/new");
    expect(await screen.findByRole("heading", { name: "New table", level: 3 })).toBeInTheDocument();
  });

  // **Each form leaves for its own channel's first screen** (M6.5c2, mock 07's
  // Cancel): the table form for the lobby it came from, the local form for the
  // door, where this browser's games are listed.
  it("leaves the table form for the lobby and the local form for the door", async () => {
    stubWorker();
    window.location.hash = "#/new";
    const { user } = app();
    await user.click(await screen.findByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
    act(() => {
      window.location.hash = "#/new/local";
    });
    await screen.findByRole("heading", { name: "New game against the bot" });
    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => expect(window.location.hash).toBe("#/"));
  });

  // **The band says who is playing on the form, in the lobby's own corner**
  // (mock 07: "Playing as …"): the identity corner, never the old "You are …"
  // strip, which stays the game banner's.
  it("draws the identity corner in the new-table form's band", async () => {
    window.location.hash = "#/new";
    app();
    await screen.findByRole("heading", { name: "New table", level: 3 });
    const band = document.querySelector<HTMLElement>(".lobby-band")!;
    expect(within(band).getByText(/Playing as/)).toBeInTheDocument();
    expect(band.textContent).not.toMatch(/You are /);
  });

  // **And on a table's two pre-game faces** (mocks 08, 09): the open table a
  // stranger arrives at, and the unlock face in front of a locked game.
  //
  // **Both corpus tables are `wire`**, so the table the handler answers is
  // switched by hand between the two visits — and the second face is pinned by
  // its Spectate button, since the open table has a "Table password" box too.
  it("draws the identity corner in the open table's band and the unlock face's", async () => {
    expect(wire.tableWatchable.id).toBe(wire.tableOpen.id);
    let table: TableView = wire.tableOpen;
    window.location.hash = `#/t/${wire.tableOpen.id}`;
    const { user } = app((call) => (call.path.endsWith("/table") ? table : undefined));
    await screen.findByRole("button", { name: "Join as Blue" });
    const band = () => document.querySelector<HTMLElement>(".lobby-band")!;
    expect(within(band()).getByText(/Playing as/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
    table = wire.tableWatchable;
    act(() => {
      window.location.hash = `#/t/${wire.tableWatchable.id}`;
    });
    await screen.findByRole("button", { name: "Spectate" });
    expect(screen.getByLabelText("Table password")).toBeInTheDocument();
    expect(within(band()).getByText(/Playing as/)).toBeInTheDocument();
  });

  // **The two forms are two components, keyed by host** (whole-branch review,
  // Minor #1). They sit at the same position in `App`'s tree, so React used to
  // reconcile one into the other and the STATE went with it: Players set to 4
  // on the server's form, then a hand-typed `#/new/local`, and the local
  // create posted `players: 4` beside the two seats a local table has — which
  // the module refuses in `check_shape`'s words.
  it("posts two players for a local table after the server's form has been set to four", async () => {
    stubWorker();
    const { user } = app();
    await user.click(await screen.findByRole("button", { name: "New table" }));
    await user.click(
      within(await screen.findByRole("group", { name: "Players" })).getByRole("radio", { name: "4" }),
    );
    // The premise, asserted: the server's form really is at four.
    expect(within(screen.getByRole("group", { name: "Players" })).getByRole("radio", { name: "4" })).toBeChecked();

    act(() => {
      window.location.hash = "#/new/local";
    });
    // The local form asks none of it: its Players group goes.
    await waitFor(() => expect(screen.queryByRole("group", { name: "Players" })).toBeNull());
    await user.click(await screen.findByRole("button", { name: "Deal" }));

    const posted = currentWorker().posted.find(
      (p) => (p as { method?: string }).method === "POST" && (p as { path?: string }).path === "/api/games",
    ) as { body: string } | undefined;
    const body = JSON.parse(posted?.body ?? "{}") as { players: number; seats: unknown[] };
    expect(body.seats).toHaveLength(2);
    expect(body.players).toBe(2);
  });

  it("draws the form for a hash typed before the app mounts", async () => {
    // **The route is read at MOUNT, not only on `hashchange`.** A link is
    // followed in a cold tab, where no `hashchange` ever fires.
    window.location.hash = "#/new";
    app();
    expect(await screen.findByRole("heading", { name: "New table", level: 3 })).toBeInTheDocument();
  });

  // **And the sentence does not follow you into the next table.** A refusal is
  // about the open that failed; carried forward it sits in the next game's error
  // strip describing nothing on screen.
  it("drops a refused table's sentence when another table is opened", async () => {
    window.location.hash = "#/t/not-yours";
    const { user } = app((call) => {
      if (call.path.endsWith("/api/games/not-yours/table"))
        return { status: 403, body: { error: "you are not seated at this table" } };
      if (isGamesList(call, "mine")) return gamesNamed(wire.options.id);
      if (call.path.endsWith(`/api/games/${wire.options.id}/table`)) return wire.table;
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.options.id}`))
        return wire.options;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map(() => wire.atK) };
      if (/\/log(\?|$)/.test(call.path)) return wire.log;
      return undefined;
    });
    await screen.findByText("you are not seated at this table");
    // **A PLAYING table, because that is the screen that would show it.** The
    // dealing screen renders no error strip at all, so a sentence "disappearing"
    // there says nothing about whether it was cleared — mutation-proven: with
    // the clear removed, this reaches the game and finds the sentence in its
    // deck.
    await user.click(await screen.findByRole("link", { name: wire.options.id }));
    await screen.findByText(/end triggers:/);
    expect(screen.queryByText("you are not seated at this table")).toBeNull();
  });

  it("shows the server's sentence over the lobby when a table refuses, and goes back to #/lobby", async () => {
    window.location.hash = "#/t/not-yours";
    app((call) =>
      call.path.endsWith("/api/games/not-yours/table")
        ? { status: 403, body: { error: "you are not seated at this table" } }
        : undefined,
    );
    const notice = await screen.findByText("you are not seated at this table");
    expect(notice).toBeInTheDocument();
    // **ABOVE the lobby, which is the spec's word and was not what the screen
    // did** (live review M-6: the sentence rendered under both lists, so on a
    // phone the explanation for a dead link sat a scroll below the fold).
    //
    // Read as DOCUMENT ORDER, and that is the honest size of this assertion:
    // jsdom has no layout engine, so nothing here can measure a pixel. The tree
    // is what the fix moved, and the tree is what this pins — between the
    // lobby's own heading and the first list's. `DOCUMENT_POSITION_FOLLOWING`
    // is set when the argument comes AFTER the node asked.
    const after = (el: Element) =>
      Boolean(notice.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING);
    expect(after(screen.getByRole("heading", { name: "Your tables" }))).toBe(true);
    expect(after(screen.getByRole("heading", { name: "Open tables" }))).toBe(true);
    expect(after(screen.getByRole("heading", { name: "Lobby" }))).toBe(false);
    // **The lobby, and the hash with it.** A screen left at `#/t/<id>` for a
    // table that answers nothing is the one place a player cannot act from —
    // and a reload would earn the same refusal again.
    expect(screen.getByRole("heading", { name: "Lobby" })).toBeInTheDocument();
    // **The lobby and not the door**, because the table that refused is the
    // SERVER's: a sentence about it belongs on the screen its siblings are
    // listed on (R-6.5b2.1). A `local-` id lands on the door instead.
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
  });

  // **And it does not outlive the SCREEN either** (final review M5). The clear
  // above happens when another table is opened; a player who instead goes to the
  // form and comes back was, until this, met by a 403 about a table they are no
  // longer trying to open. The route effect now clears on any route that is not
  // the one the notice was raised for — which is why the sentence still survives
  // the navigation that shows it, asserted first so this cannot pass by clearing
  // too eagerly.
  it("drops a refused table's sentence on a trip to the form and back", async () => {
    window.location.hash = "#/t/not-yours";
    const { user } = app((call) =>
      call.path.endsWith("/api/games/not-yours/table")
        ? { status: 403, body: { error: "you are not seated at this table" } }
        : undefined,
    );
    await screen.findByText("you are not seated at this table");
    await user.click(screen.getByRole("button", { name: "New table" }));
    await screen.findByRole("heading", { name: "New table", level: 3 });
    expect(screen.queryByText("you are not seated at this table")).toBeNull();
    window.location.hash = "#/lobby";
    expect(await screen.findByRole("heading", { name: "Lobby" })).toBeInTheDocument();
    expect(screen.queryByText("you are not seated at this table")).toBeNull();
  });

  // **The double-open, and it is the hazard `openingId` exists for.** `open`
  // sets the hash BEFORE it fetches, so the route effect runs while the first
  // `GET …/table` is still in flight — and every "is it already open?" test it
  // can make (`gameId`, `dealing`, `openTable`) is still false. Without the ref
  // a create opens its own table twice: two `GET …/table`, two deals fetched,
  // and the second reply overwriting the first.
  it("opens a table ONCE when a create navigates to it", async () => {
    // **The first `GET …/table` is HELD, and that is what makes this a witness.**
    // `hashchange` is a queued task and a stubbed fetch settles in microtasks,
    // so with an instant reply the table is already on screen by the time the
    // route effect runs and any guard at all would do. In a browser the reply is
    // a network round trip and the hashchange arrives FIRST — which is the
    // window `openingId` exists for, and the only state of the world in which
    // the second open happens. Mutation-proven: drop `openingId` from the
    // effect's condition and this cell counts two.
    const held = gate<Reply>();
    let tables = 0;
    const { stub, user } = app((call) => {
      if (call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games"))
        return { status: 201, body: wire.tableDealing };
      if (call.path.endsWith("/table")) {
        tables += 1;
        return tables === 1 ? held.promise : wire.tableDealing;
      }
      if (call.path.endsWith("/deal")) return wire.deal;
      if (call.path.endsWith("/events")) return wire.eventsDealing;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      return undefined;
    });
    await user.click(await screen.findByRole("button", { name: "New table" }));
    await user.click(await screen.findByRole("button", { name: "Deal" }));
    await waitFor(() => expect(stub.matching("GET", "/table")).toHaveLength(1));
    // The hash is set BEFORE the fetch, so it is already there; the task that
    // carries `hashchange` is what runs next.
    expect(window.location.hash).toBe(`#/t/${wire.tableDealing.id}`);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });
    // Nothing new went out while the first request was still in flight.
    expect(stub.matching("GET", "/table")).toHaveLength(1);

    held.resolve(wire.tableDealing);
    await screen.findByRole("heading", { name: "Choose your card" });
    expect(stub.matching("GET", "/table")).toHaveLength(1);
    expect(stub.matching("GET", "/deal")).toHaveLength(1);
  });

  // ---- the open table, and the hand-over BY STATUS (spec §5.4) -------------
  //
  // **`#/t/<id>` is whichever of three screens the table's own status makes
  // it**, and the third one arrived this task. What the open table knows when
  // it is done is only that the table is not open any more — the last seat
  // filled — and whether that is the deal or a game already running depends on
  // whether everyone has kept. So the hand-over RE-OPENS the table and lets
  // `open` branch, rather than naming a screen it cannot know.
  it("draws the seat picker for a table that is still filling its seats", async () => {
    window.location.hash = "#/t/wire";
    app((call) => (call.path.endsWith("/table") ? wire.tableOpen : undefined));
    expect(await screen.findByRole("button", { name: "Join as Blue" })).toBeInTheDocument();
    // And NOT the lobby, which is what this branch drew until Task 8.
    expect(screen.queryByRole("heading", { name: "Lobby" })).toBeNull();
  });

  // ---- arriving as a WATCHER (spec §5.3) ----------------------------------
  //
  // `#/t/<id>` for a caller with no seat at a playing table: `GET …/table`
  // answers and `can_watch` picks the face. `table_watchable.json` is that
  // document — a locked, playing table read by a stranger.
  it("asks for the password when a locked table is not yet watchable", async () => {
    window.location.hash = `#/t/${wire.tableWatchable.id}`;
    const { stub } = app((call) =>
      call.path.endsWith("/table") ? wire.tableWatchable : undefined,
    );
    expect(await screen.findByLabelText("Table password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Spectate" })).toBeInTheDocument();
    // **And the GAME was never asked for.** A caller who cannot read this table
    // would be refused, and a 403 in the error strip is not the screen the spec
    // asks for — the face is.
    expect(
      stub.calls.filter((c) => /\/api\/games\/[^/]+$/.test(c.path) && c.method === "GET"),
    ).toHaveLength(0);
  });

  // **Carried from Task 4's review (M-7): `can_watch` says nothing about
  // status.** An OPEN locked table answers `can_watch: false` by the
  // predicate — correctly, there is nothing to watch yet — and the other two
  // conditions are equally true of it, so a face gated on `can_watch` alone
  // would meet somebody following a JOIN link with a password box for watching
  // in place of the seat they came to take. `wire.tableOpen` is that exact
  // document, and this is the PROPERTY the review asked for.
  //
  // **Read at its real size: it is not a witness to the `status` arm.**
  // Measured — dropping `doc.status === "playing"` from the condition leaves
  // every cell in this file green, because `open`'s own arm is read FIRST and
  // an open table never reaches the unlock arm at all. The arm is written
  // anyway, so the rule is stated where the reader is rather than implied by
  // the order of two branches thirty lines apart; what this cell holds is the
  // behaviour, which is what a player meets either way.
  it("draws the seat picker, not the unlock face, for an OPEN locked table", async () => {
    expect(wire.tableOpen.locked).toBe(true);
    expect(wire.tableOpen.can_watch).toBe(false);
    expect(wire.tableOpen.my_seats).toHaveLength(0);
    window.location.hash = "#/t/wire";
    app((call) => (call.path.endsWith("/table") ? wire.tableOpen : undefined));
    expect(await screen.findByRole("button", { name: "Join as Blue" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Spectate" })).toBeNull();
  });

  // **A SEATED caller never meets a password box, whatever `can_watch` says.**
  // The document is a corpus override rather than a fixture, because the server
  // does not produce one: a caller who holds a seat may always read their own
  // table, so `can_watch` is true for them. The arm is a fence against a
  // document this client is HANDED, and this is its witness — without
  // `my_seats` in the condition a player at a locked table would be asked for a
  // password to watch a game they are playing.
  it("opens the game for a seated caller even at a table that says it cannot be watched", async () => {
    const seated = { ...wire.table, can_watch: false };
    expect(seated.my_seats.length).toBeGreaterThan(0);
    window.location.hash = `#/t/${wire.options.id}`;
    app((call) => {
      if (call.path.endsWith("/table")) return seated;
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.options.id}`))
        return wire.options;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map(() => wire.atK) };
      if (/\/log(\?|$)/.test(call.path)) return wire.log;
      return undefined;
    });
    await screen.findByText(/end triggers:/);
    expect(screen.queryByLabelText("Table password")).toBeNull();
    expect(screen.queryByRole("button", { name: "Spectate" })).toBeNull();
  });

  // The password is taken, the table is re-read, and what comes back is the
  // spectator's own frame — which is the hand-over the face makes rather than
  // assuming the next screen is a game.
  it("opens the game to watch once the password is taken", async () => {
    window.location.hash = `#/t/${wire.tableWatchable.id}`;
    let unlocked = false;
    const { user } = app((call) => {
      if (call.path.endsWith("/unlock")) {
        unlocked = true;
        return { ...wire.tableWatchable, can_watch: true };
      }
      if (call.path.endsWith("/table"))
        return unlocked ? { ...wire.tableWatchable, can_watch: true } : wire.tableWatchable;
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.spectator.id}`))
        return wire.spectator;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      return undefined;
    });
    await user.type(await screen.findByLabelText("Table password"), "hunter2");
    await user.click(screen.getByRole("button", { name: "Spectate" }));
    await screen.findByText(/end triggers:/);
    expect(screen.getByText("You are spectating")).toBeInTheDocument();
    expect(screen.queryByLabelText("Table password")).toBeNull();
  });

  it("leaves the unlock face for the lobby", async () => {
    window.location.hash = `#/t/${wire.tableWatchable.id}`;
    const { user } = app((call) =>
      call.path.endsWith("/table") ? wire.tableWatchable : undefined,
    );
    await screen.findByLabelText("Table password");
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    expect(await screen.findByRole("heading", { name: "Lobby" })).toBeInTheDocument();
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
    // The face went with the screen — a leftover would draw over the next table
    // opened.
    expect(screen.queryByLabelText("Table password")).toBeNull();
  });

  // ---- M6.5c2: a game this build's bot cannot continue (spec §2.4) -------
  //
  // **A not-over game dealt a bot this build does not run refuses to open**
  // (R-6.5c2.10), and every game route answers the 409 carrying `foreign_bot`.
  // It is a SCREEN and not a bounce, for the broken save's reason: what is
  // left is still the player's — the server's own sentence, and the Abandon
  // that archives the game — and a bounce to the lobby offered neither.
  describe("a game this build's bot cannot continue", () => {
    const refusal = wire.foreignBot409;
    /** The corpus's playing table — `wire.me` created it and sits at it — with
     *  ONE field moved, the one fact these cells are about: the bot, taken from
     *  the corpus's own foreign row (`wire.tableForeign`), run nowhere here and
     *  named as the refusal names it. */
    const dealt: TableView = { ...wire.table, bot: wire.tableForeign.bot };
    const answers = (table: TableView) => (call: ApiCall): Reply | undefined => {
      if (call.path.endsWith("/api/games/wire/table")) return table;
      if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return { status: 409, body: refusal };
      return undefined;
    };

    it("stays on the table's screen with the server's sentence, Abandon for its creator, and the way back", async () => {
      window.location.hash = "#/t/wire";
      const { user } = app(answers(dealt));
      expect(await screen.findByText(refusal.error)).toBeInTheDocument();
      // NOT the bounce every other refusal of the game takes.
      expect(window.location.hash).toBe("#/t/wire");
      // A SERVER game's screen names who is playing: "On this device" is the
      // local screens' word, and this game is the server's.
      expect(screen.getByText(`You are ${wire.me.user}`)).toBeInTheDocument();
      // `wire.me` is the table's creator, and the layer's delete is the
      // creator's (`tables.rs:1670-1677`).
      expect(dealt.created_by).toBe(wire.me.user);
      expect(screen.getByRole("button", { name: "abandon wire" })).toBeInTheDocument();
      // A server game's record is served only once it is over, so its bytes are
      // not offered here — "Save game file" is the LOCAL screen's alone.
      expect(screen.queryByRole("button", { name: "Save game file" })).toBeNull();
      await user.click(screen.getByRole("button", { name: "Lobby" }));
      await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
    });

    // **The seat half of the creator rule**: the layer's delete is the
    // creator's SEATED, so the table's creator who holds no seat at it is
    // offered none. The one field moved from `dealt` is `my_seats`;
    // `created_by` is kept, so only the seat test can withhold the control.
    it("offers no Abandon to the table's creator who holds no seat at it", async () => {
      window.location.hash = "#/t/wire";
      expect(dealt.created_by).toBe(wire.me.user);
      app(answers({ ...dealt, my_seats: [] }));
      expect(await screen.findByText(refusal.error)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^abandon / })).toBeNull();
      expect(screen.getByRole("button", { name: "Lobby" })).toBeInTheDocument();
    });

    it("offers no Abandon to a player who did not open the table", async () => {
      window.location.hash = "#/t/wire";
      app(answers({ ...dealt, created_by: "ada.bell@wire.test" }));
      expect(await screen.findByText(refusal.error)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /^abandon / })).toBeNull();
      expect(screen.getByRole("button", { name: "Lobby" })).toBeInTheDocument();
    });

    // The anti-vacuity: a 409 WITHOUT the field is the bounce it always was —
    // the arm reads the field, never the status alone.
    it("keeps the bounce for a refusal of the game that carries no foreign_bot", async () => {
      window.location.hash = "#/t/wire";
      app((call) => {
        if (call.path.endsWith("/api/games/wire/table")) return wire.table;
        if (call.method === "GET" && call.path.endsWith("/api/games/wire"))
          return { status: 409, body: wire.dealing409 };
        return undefined;
      });
      expect(await screen.findByText(wire.dealing409.error)).toBeInTheDocument();
      await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
    });

    // **Review Focus 5's third half: the refused game opens no stream.** The
    // GET's 409 body decides the screen; the transports say nothing useful
    // about a foreign game (the SSE arm writes a comment and stays open, the
    // socket sends nothing), so no stream or events read is made for it.
    //
    // **The refused game's OWN screen is asserted first**, because the bounce
    // this replaces opens no game stream either: without the address the cell
    // passes before the screen exists. The fake clock is this file's idiom for
    // `flush` (installed BEFORE the render, since the signal opens from a
    // zero-delay timer of its own), and `try`/`finally` puts the real one back.
    it("opens no stream and asks no events for a game it will not open", async () => {
      window.location.hash = "#/t/wire";
      vi.useFakeTimers();
      try {
        const { stub } = app(answers(dealt));
        await until(() => screen.queryByText(refusal.error) !== null, "the server's sentence was drawn");
        await flush();
        expect(window.location.hash).toBe("#/t/wire");
        const streams = stub.calls.filter(
          (c) => c.path.startsWith("/api/games/wire/") && /\/(stream|events)(\?|$)/.test(c.path),
        );
        expect(streams, "no stream and no events for the refused game").toEqual([]);
      } finally {
        vi.useRealTimers();
      }
    });
  });

  it("lands on the DEAL when the table flips to dealing under the screen", async () => {
    window.location.hash = "#/t/wire";
    let status: "open" | "dealing" = "open";
    // **The table's own stream, driven by hand** (M6.3). The open table is a
    // `"wake"` consumer: a frame is not its value, so what a frame buys is one
    // `GET …/table` — which is exactly what used to be bought by waiting out an
    // interval, and the flip below is the same flip.
    const stream = fakeStream();
    // `try`/`finally` is this file's own idiom for a faked clock, and it is not
    // tidiness: an assertion that throws with fake timers installed hangs every
    // cell after it in the file. **Installed BEFORE the render** since the
    // signal opens from a zero-delay timer of its own, which a clock swapped
    // afterwards would leave on the real one.
    vi.useFakeTimers();
    try {
      app((call) => {
        // **The TABLE's stream, by its whole path.** `App` draws the lobby
        // first and the lobby opens the HOUSE stream (`/api/stream`), so a
        // handler matching `/stream` answers that one too — and the wait below
        // would then be satisfied by a reader this cell is not about, with the
        // frame written to the house channel and the table screen waiting for
        // ever. Measured: that is exactly how this cell failed.
        if (call.path.endsWith("/api/games/wire/stream")) return stream.reply(call);
        if (call.path.endsWith("/table"))
          return status === "open" ? wire.tableOpen : wire.tableDealing;
        if (call.path.endsWith("/deal")) return wire.deal;
        if (call.path.endsWith("/ports")) return wire.ports;
        if (call.path.endsWith("/labs")) return wire.labs;
        if (call.path.endsWith("/areas")) return wire.areas;
        return undefined;
      });
      // **Connected BEFORE a frame is written**, and gated on the fact rather
      // than on a number of turns: a frame written to a body nobody has opened
      // reaches nobody, and nothing re-delivers it.
      await until(() => stream.opens() > 0, "the table's own stream was opened");
      expect(screen.getByRole("button", { name: "Join as Blue" })).toBeInTheDocument();

      status = "dealing";
      stream.push("signal", wire.eventsDealing);
      // **Waited for as a FACT, not for a number of turns.** The frame wakes a
      // read of the table, whose answer hands the screen over, which fetches
      // again — a chain whose length in event-loop turns is not a constant, and
      // a fixed settle under it failed as a screen that had simply not moved.
      await until(
        () => screen.queryByRole("heading", { name: "Choose your card" }) !== null,
        "the deal screen was drawn",
      );
      expect(screen.getByRole("heading", { name: "Choose your card" })).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  // **The same tick, the other answer** — and it is the case the brief's
  // `onDealing` would have got wrong: at a two-seat table the join that fills
  // the last seat can be followed by both keeps before this browser's next
  // poll, so the flip this screen sees is straight to `playing`.
  it("fetches the game when the table flips all the way to playing", async () => {
    window.location.hash = "#/t/wire";
    let status: "open" | "playing" = "open";
    const stream = fakeStream();
    vi.useFakeTimers();
    try {
      const { stub } = app((call) => {
        // The TABLE's stream by its whole path, not the lobby's — see the cell
        // above.
        if (call.path.endsWith("/api/games/wire/stream")) return stream.reply(call);
        if (call.path.endsWith("/table"))
          return status === "open" ? wire.tableOpen : wire.table;
        if (call.method === "GET" && call.path.endsWith("/api/games/wire")) return wire.options;
        if (call.path.endsWith("/ports")) return wire.ports;
        if (call.path.endsWith("/labs")) return wire.labs;
        if (call.path.endsWith("/areas")) return wire.areas;
        return undefined;
      });
      // **The GAME, by its whole path.** `matching` is a substring test and
      // `/api/games/wire` is a prefix of `/api/games/wire/table`, so the two
      // reads this cell is telling apart are indistinguishable to it.
      const gameReads = () =>
        stub.calls.filter((c) => c.method === "GET" && c.path === "/api/games/wire");
      await until(() => stream.opens() > 0, "the table's own stream was opened");
      // Anti-vacuity: the game was NOT asked for while the table was open —
      // that request is a 409 at this status, which is the whole reason `open`
      // reads the table first.
      expect(gameReads()).toHaveLength(0);

      status = "playing";
      stream.push("signal", wire.events);
      await until(() => screen.queryByText(/end triggers:/) !== null, "the game was drawn");
      expect(screen.getByText(/end triggers:/)).toBeInTheDocument();
      expect(gameReads()).toHaveLength(1);
    } finally {
      vi.useRealTimers();
    }
  });

  // **And the hand-over that finds the table refusing** (final review M1). The
  // reachable case is a stranger who pressed a seat that filled in the same
  // moment: the join answers with a table that is no longer open, the hand-over
  // runs, and `GET …/table` refuses a caller who is not seated. `open` has no
  // catch of its own, so before this the rejection was unhandled and the screen
  // recovered only when the open table's OWN poll refused a tick later.
  //
  // **The clock is what makes this a witness**: the poll is never advanced past
  // the tick that hands over, so the sentence on screen can only have come from
  // the hand-over's catch.
  it("lands on the lobby with the sentence when the hand-over's table refuses", async () => {
    window.location.hash = "#/t/wire";
    // **Flipped by hand rather than counted.** Two reads open this screen —
    // `open`'s own and `OpenTable`'s opening fetch — so a cell that numbered
    // them would hand over before the seat picker was ever drawn.
    let flipped = false;
    let afterFlip = 0;
    const stream = fakeStream();
    const rejected: unknown[] = [];
    const note = (e: unknown) => rejected.push(e);
    process.on("unhandledRejection", note);
    vi.useFakeTimers();
    try {
      app((call) => {
        // The TABLE's stream by its whole path, not the lobby's.
        if (call.path.endsWith("/api/games/wire/stream")) return stream.reply(call);
        if (!call.path.endsWith("/table")) return undefined;
        if (!flipped) return wire.tableOpen;
        afterFlip += 1;
        // The re-read that finds a table no longer open is what hands over; the
        // read the hand-over itself makes is the one that refuses.
        return afterFlip === 1
          ? wire.tableDealing
          : { status: 403, body: { error: "you are not seated at this table" } };
      });
      await until(() => stream.opens() > 0, "the table's own stream was opened");
      expect(screen.getByRole("button", { name: "Join as Blue" })).toBeInTheDocument();

      flipped = true;
      stream.push("signal", wire.eventsDealing);
      await until(
        () => screen.queryByRole("heading", { name: "Lobby" }) !== null,
        "the lobby was drawn",
      );
      expect(screen.getByRole("heading", { name: "Lobby" })).toBeInTheDocument();
      expect(screen.getByText("you are not seated at this table")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Join as Blue" })).toBeNull();
    } finally {
      vi.useRealTimers();
      process.off("unhandledRejection", note);
    }
    expect(rejected).toEqual([]);
  });

  // The other half of the same rule: a game LEFT takes the hash with it, so the
  // address never names a screen that is not on it.
  it("clears the game and the hash when the Game menu leaves for the lobby", async () => {
    await renderApp(wire.options);
    expect(window.location.hash).toBe(`#/t/${wire.options.id}`);
    const user = userEvent.setup();
    await user.click(gameMenu());
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    expect(await screen.findByRole("heading", { name: "Lobby" })).toBeInTheDocument();
    // **`#/lobby` since M6.5b2**, and the word on the button is why: every
    // control in this client that says "Lobby" goes to the lobby, whatever it
    // is leaving. What lands on the DOOR is a game that ENDS — abandoned,
    // deleted, or asked for and no longer there — and only when it is a local
    // one, because that is the screen its siblings are listed on.
    expect(window.location.hash).toBe("#/lobby");
  });
});

// ---- the door (M6.1 §7.1) --------------------------------------------------
//
// The door's states, and `/api/me` picks between them (M6.5c2, spec §2.1):
// "Connecting to the server…" while it is in flight; the name step under an
// `open` deployment with nobody named; under the other two doors a card saying
// the server couldn't tell who you are, the missing header in its operator
// fold; the expired-session and can't-reach cards when the read fails; and the
// app itself once the server names a user.

describe("the door", () => {
  it("shows the name step when the door is open and nobody has a name, then the lobby", async () => {
    const stub = stubApi((call) => {
      // Answered the way the SERVER answers it: from the header this browser
      // is sending. A stub that returned `wire.me` regardless would model a
      // server that ignores its own header, and signing in would prove nothing.
      if (call.path.endsWith("/api/me"))
        return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
      if (call.path.split("?")[0]!.endsWith("/api/games")) return wire.games;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      return undefined;
    });
    // **The lobby, because that is where the server's door now stands**
    // (R-6.5b2.1): `#/` is the front door and draws no sign-in at all — it is
    // the screen that needs no identity.
    window.location.hash = "#/lobby";
    render(<App />);
    await screen.findByRole("heading", { name: "Choose a name" });
    // **Nothing is asked for from behind the door.** Every `/api/games` route
    // is a 401 without an identity, so a client that listed first would show
    // the player a refusal as their first screen.
    expect(stub.matching("GET", "/api/games")).toHaveLength(0);

    const user = userEvent.setup();
    await user.type(screen.getByLabelText("Name"), wire.me.user!);
    await user.click(screen.getByRole("button", { name: "Continue" }));

    // The band's corner names the server's answer — its key rides as the
    // corner's title (`IdentityCorner`).
    await waitFor(() =>
      expect(document.querySelector(".identity-corner [title]")).toHaveAttribute("title", wire.me.user!),
    );
    // The list, and no individual game: signing in lands on the LOBBY, not
    // inside whatever game happens to be first.
    expect(stub.matching("GET", "/api/games/")).toHaveLength(0);
    // Read AGAIN after signing in, rather than the client deciding for itself
    // that it is now this person: a typed name is only worth what the server
    // says it is worth.
    expect(stub.matching("GET", "/api/me").length).toBeGreaterThanOrEqual(2);
  });

  // **The FRONT door is not behind the server's** (M6.5b2, R-6.5b2.1). The
  // cells in "the first screen with no server" read `#/` with the API refused;
  // this one reads it with the server perfectly reachable and NOBODY signed in,
  // which is the arrangement where a gate would still have been visible: the
  // form is one hash away and is not drawn here, and the door asks `/api/me`
  // nothing to find that out.
  it("draws the door and no sign-in at #/ when the server is up and nobody is signed in", async () => {
    stubWorker();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/me")) return wire.meSignedOut;
      if (call.path.endsWith("/api/health")) return wire.health;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/";
    render(<App />);
    expect(await screen.findByRole("heading", { name: "On this device" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Go to the lobby" })).toHaveAttribute("href", "/enter");
    expect(screen.queryByRole("heading", { name: "Choose a name" })).toBeNull();
    // And the door was never asked who this is.
    expect(stub.matching("GET", "/api/me")).toHaveLength(0);

    // One hash away, the form IS drawn — so the absence above is this screen's
    // rule and not a client that has stopped asking altogether.
    act(() => {
      window.location.hash = "#/lobby";
    });
    expect(await screen.findByRole("heading", { name: "Choose a name" })).toBeInTheDocument();
    expect(stub.matching("GET", "/api/me")).toHaveLength(1);
  });

  // **The door's FIRST answer, as content** (M6.5b, spec §2.5). The other
  // three arms are exercised by the cells around this one — the form, the
  // sentence, and the silence in "the first screen with no server" — and this
  // is the one that needs arranging: `/api/me` refusing while the server is
  // otherwise reachable, which is what tells the door's own complaint from the
  // "can't reach the server" card that replaces it when NOTHING answered.
  it("draws the door's refusal in place of the lists, with this browser's games one hash away", async () => {
    stubWorker();
    const sentence = "the store could not be read";
    stubApi((call) => {
      if (call.path.endsWith("/api/me")) return { status: 500, body: { ...wire.error, error: sentence } };
      // The server is up — that is the whole point of this arm: the door
      // itself refused, and its words are the useful ones.
      if (call.path.endsWith("/api/health")) return wire.health;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/lobby";
    render(<App />);
    expect(await screen.findByText(sentence)).toBeInTheDocument();
    // And not the line for a server nobody reached: this one answered.
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
    // **The other channel is still reachable from here** (R-6.5b2.1). "On this
    // device" is the DOOR's since this cycle, and the lobby's one link back is
    // drawn OUTSIDE the server area's three states — including this one, where
    // the door itself refused and there is nothing else on the screen to press.
    act(() => {
      window.location.hash = "#/";
    });
    expect(await screen.findByRole("heading", { name: "On this device" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "New game" })).toBeEnabled();
    // **And an ordinary refusal offers no turnstile** (M6.5b2) — the
    // anti-vacuity partner of the cell below, which draws one. This sentence
    // is the server's own and `/enter` would do nothing about it.
    expect(screen.queryByRole("link", { name: "Sign in again" })).toBeNull();
  });

  // **The EDGE's 401 is the one refusal a player can act on** (M6.5b2, spec
  // §2.1). The session in FRONT of the server expired; the name this browser
  // holds is perfectly good, and passing the turnstile once fixes it. So the
  // sentence is the expired-session line with `/enter` beside it — a
  // navigation and never a fetch, because Access answers a fetch with a 302 to
  // another origin that fails opaquely.
  //
  // It goes the whole way: the header `serverHeaders()` sends is what makes the
  // 401 same-origin and readable, `settle` tells it from this API's own 401 by
  // the `WWW-Authenticate` header, and `Door` draws it as state A: its own
  // card, with the turnstile's link.
  //
  // **Both reads meet the challenge, which is the real shape** (Review Focus
  // 3): the Access application covers the whole hostname, `/api/health`
  // included, so an expired session fails the health read too — and the card
  // must still be A, not "can't reach the server", once that read has settled.
  it("draws the turnstile's link for an EXPIRED Access session, and keeps the stored name", async () => {
    stubWorker();
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const challenge = () =>
      new Response("<html>", {
        status: 401,
        headers: {
          "www-authenticate": 'Cloudflare-Access resource_metadata="https://x/.well-known/y"',
        },
      });
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/me")) return challenge();
      if (call.path.endsWith("/api/health")) return challenge();
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/lobby";
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Your sign-in has run out" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Sign in again" })).toHaveAttribute("href", "/enter");
    // **The claimed name is NOT dropped**, which is the half a status-only read
    // would get wrong: the door this browser knocks on is fine, and signing the
    // player out of it would replace an actionable sentence with a form that
    // cannot mint the cookie that is actually missing.
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBe(wire.me.user);
    // And the sign-in form is not what they are shown.
    expect(screen.queryByRole("heading", { name: "Choose a name" })).toBeNull();
    // Once the challenged health read has SETTLED, the card is still A.
    expect(stub.matching("GET", "/api/health").length).toBeGreaterThan(0);
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    expect(screen.getByRole("heading", { name: "Your sign-in has run out" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
  });

  it("names the door rather than offering a form when an edge-door deployment has no user", async () => {
    stubApi((call) =>
      // The corpus carries only the `open` door — the fixture server runs in
      // that mode — so the MODE is the one field moved off a real payload.
      call.path.endsWith("/api/me") ? { ...wire.meSignedOut, mode: "cf-access" } : undefined,
    );
    window.location.hash = "#/lobby";
    render(<App />);
    await screen.findByText(/Cf-Access-Authenticated-User-Email/);
    expect(screen.getByRole("heading", { name: "This server couldn't tell who you are" })).toBeInTheDocument();
    // There is nothing to type: the identity arrives from an edge in front of
    // the server, and a form here would store a name that server ignores.
    expect(screen.queryByLabelText("Name")).toBeNull();
  });

  it("a 401 mid-game returns to sign-in, carrying the sentence", async () => {
    const stub = await renderApp(wire.options, (call) =>
      call.path.endsWith("/input") ? signedOutReply() : undefined,
    );
    await userEvent.setup().click(firstOptionButton());
    await screen.findByRole("heading", { name: "Choose a name" });
    // **§7.1's last bullet.** The refusal was in an error strip the sign-in
    // screen replaced, so without carrying it the player's game vanishes and a
    // bare form appears with nothing connecting the two.
    expect(screen.getByRole("status")).toHaveTextContent(signedOutReply().body.error);
    expect(stub.matching("POST", "/input")).toHaveLength(1);
    // The stored name is DROPPED, not merely re-asked: the harness answers
    // `/api/me` from storage, so a client that kept it would have been handed
    // its user straight back and walked into the game again.
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
    // And the recovery a refusal normally triggers does NOT run: re-reading the
    // game would earn the same 401, so the only `GET` for it is the one that
    // opened it. The GAME, not its board panels — `matching` is a substring
    // test and every panel's path contains the game's.
    expect(
      stub.calls.filter(
        (c) => c.method === "GET" && c.path.endsWith(`/api/games/${wire.options.id}`),
      ),
    ).toHaveLength(1);
  });

  // ---- R-7.1: EVERY call routes, because `call()` does it ------------------
  //
  // The four cells below are four different callers, and three of them handle
  // their own refusals in ways that could not have routed a 401 on their own:
  // the Game menu shows the sentence and stays put, the scrubber is an effect,
  // `/result` throws its error away entirely, and the opening walk reports into
  // an error strip on a screen it has already drawn. One `if` in `api.ts` is
  // what makes all four the same, and Task 8's rewritten screens too.

  it("a 401 from the Game menu's list returns to sign-in", async () => {
    let lists = 0;
    await renderApp(wire.options, (call) => {
      if (isGamesList(call, "mine")) {
        // The FIRST `mine` list is the LOBBY's, which `renderApp` opens the
        // game from. The menu's is the next one, and that is the one refused.
        lists += 1;
        if (lists > 1) return signedOutReply();
      }
      return undefined;
    });
    await userEvent.setup().click(gameMenu());
    await screen.findByRole("heading", { name: "Choose a name" });
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });

  // The scrubber reads the same record every other route does, and it is the
  // one catch site that is an effect rather than a click handler.
  it("a 401 from the replay scrubber returns to sign-in", async () => {
    await renderApp(wire.options, (call) =>
      // The SCRUBBER still asks `/at/{k}` one position at a time (R-6.5b.10
      // keeps that route); the walk is the run route, the cell below.
      /\/at\/\d+$/.test(call.path) ? signedOutReply() : undefined,
    );
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Log" }));
    // One step back from the present. The fetch is debounced 200 ms, so the
    // wait below is what carries it.
    await user.click(await screen.findByRole("button", { name: "Previous move" }));
    await screen.findByRole("heading", { name: "Choose a name" }, { timeout: 2000 });
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });

  it("a refusal that is not about identity leaves you in the game", async () => {
    // **What makes the cell above about 401 and not about 4xx.** Not being
    // seated at a table is a 403, and is exactly the refusal a player CAN act
    // on from inside the app; signing them out would throw away the screen
    // that explains it.
    await renderApp(wire.options, (call) =>
      call.path.endsWith("/input") ? { status: 403, body: wire.notSeated403 } : undefined,
    );
    await userEvent.setup().click(firstOptionButton());
    await screen.findByText(wire.notSeated403.error);
    expect(screen.queryByRole("heading", { name: "Choose a name" })).toBeNull();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBe(wire.me.user);
  });

  // **The cell that can only pass CENTRALLY.** `/result`'s catch is
  // `.catch(() => setOutcome(null))` — it does not receive the error, it
  // discards it — so no amount of wiring at the call sites that DO receive one
  // would route this. A finished game whose session had expired sat there
  // showing no standing and saying nothing at all.
  it("a 401 from the final score returns to sign-in", async () => {
    await renderApp(wire.gameOver, (call) =>
      call.path.endsWith("/result") ? signedOutReply() : undefined,
    );
    await screen.findByRole("heading", { name: "Choose a name" });
    expect(screen.getByRole("status")).toHaveTextContent(signedOutReply().body.error);
  });

  // **The worst of the unrouted sites, because it put a FALSE sentence on
  // screen.** The setup screen's list swallows its refusal into an empty array,
  // so a player whose session had expired was told they had no games.
  it("a 401 from the saved-game list says why, rather than that you have no games", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    stubApi((call) => {
      if (call.path.endsWith("/api/me"))
        return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      // Both scopes, because the lobby asks for both and a client that saw one
      // refuse and one answer would draw half a lobby over a closed door.
      if (isGamesList(call)) return signedOutReply();
      return undefined;
    });
    window.location.hash = "#/lobby";
    render(<App />);
    await screen.findByRole("heading", { name: "Choose a name" });
    expect(screen.getByRole("status")).toHaveTextContent(signedOutReply().body.error);
    expect(screen.queryByText(/none yet/)).toBeNull();
  });

  // The opening walk: the run a just-created game's own events are shown as. It
  // runs OUTSIDE any request of its own — `show` starts it — so its refusal
  // landed in the error strip of a game screen already drawn, and the SAME walk
  // routed correctly when it ran inside a move.
  it("a 401 from the opening walk returns to sign-in", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    let dealt = false;
    stubApi((call) => {
      if (call.path.endsWith("/api/me"))
        return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (isGamesList(call)) return gamesNamed();
      if (call.method === "POST" && call.path.split("?")[0]!.endsWith("/api/games"))
        return { status: 201, body: wire.tableDealing };
      // The table crosses from dealing to playing at the keep, which is what
      // makes the game open at all — see the boards cell above for the shape.
      if (call.path.endsWith("/table")) return dealt ? wire.table : wire.tableDealing;
      if (call.path.endsWith("/deal")) return wire.deal;
      if (call.path.endsWith("/events")) return wire.eventsDealing;
      if (call.path.endsWith("/keep")) {
        dealt = true;
        return wire.table;
      }
      // **`events` is the one field moved, and it is what makes this the
      // opening walk at all**: `show(state, opening)` builds a run only when the
      // game it opens already has events, and the corpus's playable state has
      // none. Two of them, so the run is three positions rather than the 1,036
      // the finished game would fetch.
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.options.id}`))
        return { ...wire.options, events: 2 };
      if (runRange(call.path)) return signedOutReply();
      if (/\/log(\?|$)/.test(call.path)) return wire.log;
      if (call.path.endsWith("/ports")) return wire.ports;
      if (call.path.endsWith("/labs")) return wire.labs;
      if (call.path.endsWith("/areas")) return wire.areas;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    // The create is the SERVER's, and so is the screen it starts from.
    window.location.hash = "#/lobby";
    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "New table" }));
    await user.click(await screen.findByRole("button", { name: "Deal" }));
    // Two dealt cards, so two buttons: the first, as the other create cells do.
    // The keep is the last one, so the game opens on the reply and the opening
    // walk starts — which is the run whose refusal this cell is about.
    await user.click((await screen.findAllByRole("button", { name: /^keep O\d\d for seat 0/ }))[0]!);
    await screen.findByRole("heading", { name: "Choose a name" });
    expect(screen.getByRole("status")).toHaveTextContent(signedOutReply().body.error);
  });

  // **Review Focus 3.** A browser that typed an address under the OPEN door
  // still holds its KEY when it visits a door set in front of the server — the
  // NAS after its redeploy onto `cf-access`. The stale header rides every
  // request, the server ignores it, and the corner says who ACCESS says this
  // is. Nothing asks the player to "switch player": the identity is not theirs
  // to switch there. The KEY only: a stored DISPLAY name shown behind Access is
  // as the spec mandates (ruling R-B14 — the two hosts are different origins,
  // so no browser carries one across).
  it("names the Access identity, never a stale stored key, and offers no switch player (Review Focus 3)", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, "stale@old.test");
    const stub = stubApi((call) => {
      // The corpus's user behind Access: one field moved off a real payload,
      // the precedent of the edge-door cell above.
      if (call.path.endsWith("/api/me")) return { ...wire.me, mode: "cf-access" };
      if (call.path.endsWith("/api/health")) return { ...wire.health, identity: "cf-access" };
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (call.path.endsWith("/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      if (isGamesList(call)) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/lobby";
    render(<App />);
    await waitFor(() => expect(document.querySelector(".identity-corner")).not.toBeNull());
    const corner = document.querySelector<HTMLElement>(".identity-corner")!;
    expect(within(corner).getByTitle(wire.me.user!)).toHaveTextContent(wire.me.user!);
    expect(within(corner).getByRole("button", { name: "change name" })).toBeInTheDocument();
    expect(within(corner).queryByRole("button", { name: "switch player" })).toBeNull();
    expect(document.body.textContent).not.toContain("stale@old.test");
    // The premise, read off the wire: the stale header WAS sent, and ignored.
    expect(stub.matching("GET", "/api/me")[0]!.headers["x-yokohama-claimed-user"]).toBe("stale@old.test");
  });

  // **switch player, under the open door** (spec §2.1): the stored name goes —
  // key and display both — and the player is back at the name step.
  it("drops the stored name on switch player, and lands on the name step", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    localStorage.setItem(DISPLAY_NAME_KEY, "Ben Castle");
    stubApi((call) => {
      if (call.path.endsWith("/api/me"))
        return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
      if (call.path.endsWith("/api/health")) return wire.health;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (call.path.endsWith("/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      if (isGamesList(call)) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/lobby";
    render(<App />);
    await waitFor(() => expect(document.querySelector(".identity-corner")).not.toBeNull());
    const corner = document.querySelector<HTMLElement>(".identity-corner")!;
    expect(within(corner).getByTitle(wire.me.user!)).toHaveTextContent("Ben Castle");
    await userEvent.setup().click(within(corner).getByRole("button", { name: "switch player" }));
    expect(await screen.findByRole("heading", { level: 3, name: "Choose a name" })).toBeInTheDocument();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
  });
});

// ---- M6.1 §6: the opponent's turn arrives by the signal ---------------------
//
// The poll's own properties — when it asks, when it is silent, what it says
// while it cannot reach the server — are `signal.test.ts`'s. What this cell is
// about is the CONSUMER (R-P9): the one function M6.3 replaces the transport
// under, and the only thing in the client that turns "the count moved" into a
// board the player can see.
// The game screen with the viewer ABSENT (spec §5.2). What a watcher is handed
// is a `ClientState` with `seat: null`, and that one field selects the whole
// screen: no answer bar, no Undo, no Suggest, and a strip that says what you
// are rather than who.
describe("the spectator screen", () => {
  it("says you are spectating, offers no answer and no Suggest", async () => {
    await renderApp(wire.spectator);
    expect(wire.spectator.seat).toBeNull();
    // The strip, which is the one line that says which of the two you are here.
    expect(screen.getByText("You are spectating")).toBeInTheDocument();
    expect(screen.queryByText(new RegExp(`You are ${wire.me.user}`))).toBeNull();
    // The deck: the `watching` arm's own words, and nothing to press.
    expect(screen.getByText(/Seat \d+ is deciding\./)).toBeInTheDocument();
    expect(screen.getByText(/Nothing here is yours to answer/)).toBeInTheDocument();
    expect(document.querySelector("button.option")).toBeNull();
    expect(screen.queryByRole("button", { name: "Undo" })).toBeNull();
    // Suggest ranks the moves on YOUR seat; a watcher has none, so the banner
    // does not offer it. Why is still there — it is an account of a move that
    // was played, and a watcher is here to read it.
    expect(screen.queryByRole("button", { name: "Suggest" })).toBeNull();
    expect(screen.getByRole("button", { name: "Why" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Log" })).toBeInTheDocument();
  });

  // The anti-vacuity, and it is the whole reason the cell above means anything:
  // the SAME shell over a SEATED payload draws both controls and the address.
  // Without this, a banner that had lost its Suggest button entirely, or a
  // strip that said "You are spectating" to everyone, would pass.
  it("offers Suggest and names the player at a table this caller sits at", async () => {
    await renderApp(wire.options);
    expect(wire.options.seat).not.toBeNull();
    expect(screen.getByRole("button", { name: "Suggest" })).toBeInTheDocument();
    expect(screen.getByText(new RegExp(`You are ${wire.me.user}`))).toBeInTheDocument();
    expect(screen.queryByText("You are spectating")).toBeNull();
  });
});

describe("the opponent's turn arrives by the signal", () => {
  /** A position for the run to be walked over: the corpus's `at_k`, moved to
   *  the position and the mover this cell is about, and NOT marked as a bot.
   *
   *  **No seat here is a bot, and that is the point (ruling R-9.1).** A room is
   *  people. A walk gated on `hasBotStep` therefore never runs for a person's
   *  turn, and the opponent's move SNAPS onto the table — M5.5's whole
   *  step-by-step account, the thing the room plan promises the poll will
   *  drive, silently not happening at exactly the table it was built for. The
   *  first version of this cell marked seat 0 a bot and so could not have seen
   *  it; the fixture is left honest instead. */
  const at = (k: number, total: number) => {
    const f = JSON.parse(JSON.stringify(wire.atK)) as typeof wire.atK;
    f.index = k;
    f.total = total;
    f.to_move = wire.watching.to_move;
    return f;
  };

  /** The reply chain settles without the clock moving — the harness's own
   *  [`flush`], for the reason written out in full there and the one written
   *  out in `the turn playback`: `waitFor` takes its real-timer branch under
   *  vitest's fake clock and never polls. Every POSITIVE outcome this cell
   *  waits for is gated with `until` instead; this is what the anti-vacuity
   *  reads — "and nothing has happened yet" — are given. */
  const settle = flush;

  /** Put the tab in a state and tell the document, the way `signal.test.ts`
   *  does: jsdom has no page lifecycle, so both halves are set by hand. */
  const visibility = (value: "visible" | "hidden") =>
    act(() => {
      Object.defineProperty(document, "visibilityState", { value, configurable: true });
      document.dispatchEvent(new Event("visibilitychange"));
    });

  it("walks the new events when the count grows, and reloads without a walk when it shrinks", async () => {
    let events = wire.watching.events;
    /** The signal this game is watching, as the server writes it: an `Events`
     *  on the `signal` frame, which the game declares `frames: "value"` for and
     *  therefore reads WITHOUT a second request (R-7.1, R-3.2).
     *
     *  **Built from the corpus and not by hand** (final review M-1), which is
     *  `signal.test.ts`'s `at()` and the repo's rule: an object written out
     *  here still compiles after the server renames a field, and only the
     *  committed payload moves with it. */
    const signal = (): Events => ({
      ...wire.events,
      events,
      to_move: wire.watching.to_move,
    });
    // The server writes the current count ON CONNECT (spec §3.1), which is what
    // makes the return from a hidden tab an ask with no request of its own.
    const stream = fakeStream(() => [["signal", signal()]]);
    const stub = await renderApp(wire.watching, (call) => {
      // The GAME's stream by its whole path: `renderApp` goes in through the
      // lobby, which opens the house stream, and a handler matching `/stream`
      // would hand this cell's frames to that one.
      if (call.path.endsWith(`/api/games/${wire.watching.id}/stream`)) return stream.reply(call);
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.watching.id}`))
        return { ...wire.watching, events };
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k, events)) };
      return undefined;
    });
    /** The RE-READS of the game, and the match is anchored on purpose:
     *  `matching("GET", "/api/games/wire")` is a substring test that also counts
     *  `/events`, `/at/0` and `/table`, so it would pass without a reload
     *  happening at all. */
    const reloads = () =>
      stub.calls.filter((c) => c.method === "GET" && /\/api\/games\/wire$/.test(c.path));
    expect(reloads()).toHaveLength(1);

    // **The tab is hidden and then shown again, and that is not decoration.**
    // The reader was opened while `renderApp` drove `userEvent` under REAL
    // timers — `renderApp` has to, because user-event awaits a `setTimeout(0)`
    // between interactions and a faked one never fires. Hiding the tab lets
    // that reader go (R-P8) and the return opens the next one on whichever
    // clock is installed by then, so the cell gets the signal on the fake clock
    // and the R-P8 round trip is exercised on the way.
    // How many readers had been opened before the tab went away, so the return
    // can be waited for as a FACT rather than as a number of turns.
    const opened = stream.opens();
    await visibility("hidden");
    vi.useFakeTimers();
    try {
      await visibility("visible");
      // **The strip, on the banner, before the answer.** The board on screen is
      // as old as the tab was hidden, so this is the one true thing to say until
      // the reader is connected — and it is the only assertion anywhere that
      // `App` renders the hook's state at all.
      expect(screen.getByText(/refreshing/)).toBeInTheDocument();
      // Connected again, and the ON-CONNECT frame answered: the count has NOT
      // moved, so the strip goes and nothing at all is fetched. That frame is
      // the whole of the ask — there is no request to `/events` to make it
      // with.
      await until(() => stream.opens() > opened, "the reader was opened again");
      await settle();
      expect(screen.queryByText(/refreshing/)).toBeNull();
      expect(reloads()).toHaveLength(1);
      expect(positionsWalked(stub)).toHaveLength(0);
      expect(stub.matching("GET", "/events")).toHaveLength(0);

      events = wire.watching.events + 2;
      stream.push("signal", signal());
      await until(() => reloads().length === 2, "the game was re-read");
      await settle();
      // The count moved, so the game was re-read ONCE and the run's positions
      // fetched: `prev..=next` is 0, 1, 2 — `prev` included, because a step's
      // highlight is a diff and the live payload is not a position.
      expect(reloads()).toHaveLength(2);
      expect(positionsWalked(stub)).toEqual([0, 1, 2]);
      // And the walk is on the table: the deck is the strip rather than the
      // prompt, and the account has one row per event.
      //
      // **A PERSON's turn, walked (ruling R-9.1).** Nothing in this fixture is
      // a bot; the mover is simply not the seat this browser is watching from
      // (`wire.watching` is built for seat 1 while seat 0 is to move). The
      // account's transport is the assertion that separates a walk from a
      // static list of rows — `TurnAccount` renders those controls only while
      // one is running — and the lit row is what says a STEP is on the table
      // rather than the whole run at once.
      expect(document.querySelector(".decision.walking")).not.toBeNull();
      expect(screen.getByText(/is playing · step 1 of 2/)).toBeInTheDocument();
      expect(document.querySelectorAll("ol.account li")).toHaveLength(2);
      const account = within(document.querySelector<HTMLElement>(".turn-account")!);
      expect(account.getByRole("button", { name: "Pause" })).toBeInTheDocument();
      expect(account.getByRole("button", { name: "Step forward" })).toBeInTheDocument();
      expect(account.getByRole("button", { name: "Step back" })).toBeInTheDocument();
      expect(account.getByRole("button", { name: "skip" })).toBeInTheDocument();
      expect(document.querySelector("ol.account li.at")).toBe(
        document.querySelector("ol.account li"),
      );

      // **`events` is legitimately non-monotone: an undo POPS them** (R-P5).
      // The client re-reads, and must NOT try to walk a range that runs
      // backwards — `/at/-1` is not a position and the account it would build
      // describes nothing.
      //
      // **One FRAME per step, which is what a server writes.** Until M6.3 this
      // was one advance of a fake clock long enough for several ticks, and the
      // note here recorded that three of them ran against a tree that had not
      // re-rendered. A frame is pushed by hand instead, so the harness no
      // longer has a cadence of its own to be wrong about.
      const atCalls = positionsWalked(stub).length;
      events = wire.watching.events;
      stream.push("signal", signal());
      await until(() => reloads().length === 3, "the game was re-read again");
      await settle();
      expect(reloads()).toHaveLength(3);
      expect(positionsWalked(stub)).toHaveLength(atCalls);
      expect(document.querySelector("ol.account")).toBeNull();
      expect(document.querySelector(".decision.walking")).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  // **A WATCHER's run walks too, and this is the cell for spec §9.5 mutation
  // 10** (spec §5.2). A spectator holds no seat: `wire.spectator` carries
  // `seat: null`, so the arrival's viewer is absent and every step of every run
  // the server hands them is somebody else's by construction.
  //
  // **What it catches is the CALL SITE, not the predicate.** `hasOpponentStep`
  // cannot see the difference — `0 !== null` is already true, and `walk.test.ts`
  // says so — but `landed`'s gate can: under the old encoding a `null` fourth
  // parameter meant "the reply to my own move" and selected `hasBotStep`, and
  // this fixture's table is two people with no bot in it. So the mutation is
  // exactly "the spectator's served run takes the `hasBotStep` branch", and
  // what it looks like on screen is the opponent's turn SNAPPING onto the
  // table: no lit row, no strip, no account at all.
  //
  // The fixture's mover must be a HUMAN for the two branches to answer
  // differently at all, and the cell ASSERTS that rather than stating it
  // (review T8 #2): `moverIsBot` is read off the Players panel of the position
  // before each step, so a corpus re-cut that seated a bot on the moving seat
  // would leave this cell green against the very defect it is named for.
  it("walks a run served to a watcher, who holds no seat at all", async () => {
    const live = wire.spectator;
    let events = live.events;
    const seats = wire.spectatorAtK.frame.dashboard.flatMap((p) =>
      "Players" in p ? p.Players.players : [],
    );
    expect(
      seats.find((p) => p.seat === live.to_move)?.is_bot,
      "the fixture's mover is a person, which is what makes hasBotStep the wrong gate",
    ).toBe(false);
    const signal = (): Events => ({ ...wire.events, events, to_move: live.to_move });
    const stream = fakeStream(() => [["signal", signal()]]);
    /** A position for the run, from the SPECTATOR's own corpus frame — which
     *  carries `seat: null` and every hand hidden, exactly as the server builds
     *  one for a watcher. */
    const at = (k: number) => {
      const f = JSON.parse(JSON.stringify(wire.spectatorAtK)) as typeof wire.spectatorAtK;
      f.index = k;
      f.total = events;
      f.to_move = live.to_move;
      return f;
    };
    const stub = await renderApp(live, (call) => {
      if (call.path.endsWith(`/api/games/${live.id}/stream`)) return stream.reply(call);
      if (call.method === "GET" && call.path.endsWith(`/api/games/${live.id}`))
        return { ...live, events };
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k)) };
      return undefined;
    });
    // Nothing has been walked yet: the account holds the reply's prose, which
    // for this fixture is nothing at all.
    expect(document.querySelector("ol.account")).toBeNull();

    // The hidden/visible round trip, for `the opponent's turn arrives by the
    // signal`'s reason: the first reader was opened under real timers, and the
    // return opens the next one on the fake clock this cell needs.
    const opened = stream.opens();
    await act(async () => {
      Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
      document.dispatchEvent(new Event("visibilitychange"));
    });
    vi.useFakeTimers();
    try {
      await act(async () => {
        Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
        document.dispatchEvent(new Event("visibilitychange"));
      });
      await until(() => stream.opens() > opened, "the reader was opened again");
      await flush();

      events = live.events + 1;
      stream.push("signal", signal());
      await until(
        () => document.querySelector("ol.account li.at") !== null,
        "the watcher's run was walked",
      );
      // One event arrived, so the run is one step — and it is LIT, which is
      // what says a step is on the table rather than the whole run at once.
      expect(document.querySelectorAll("ol.account li")).toHaveLength(1);
      expect(document.querySelector("ol.account li.at")).toBe(
        document.querySelector("ol.account li"),
      );
      expect(document.querySelector(".decision.walking")).not.toBeNull();
      expect(screen.getByText(/is playing · step 1 of 1/)).toBeInTheDocument();
      // `prev` is fetched with `next`, because a step's highlight is a diff.
      expect(positionsWalked(stub)).toEqual([live.events, live.events + 1]);
    } finally {
      vi.useRealTimers();
    }
  });

  // **ONE open stream across a whole arrival (ruling R-9.1, spec §9.4).**
  //
  // The browser gate MEASURED the opposite: a watching browser opened
  // `/stream` 2,643 times in one game, exactly once per applied move. The cause
  // was one term — `active` read `busy`, and the signal's own handler sets
  // `busy` around the frame it fetches, so every arrival took `active` false
  // and then true and `usePolled`'s effect tore the reader down and opened
  // another. That term was written for a POLL, where "do not ask while my own
  // request is in flight" costs nothing; under a held connection it means
  // letting go of the stream on every move somebody else makes.
  //
  // This is the cell that holds the fix: the reader count must not move across
  // a frame, its game re-read, its positions and its walk. **Mutation-proven**
  // — restore `!busy` to `active` and it fails, because the reopen is scheduled
  // at zero and `settle` advances the clock past it.
  it("holds ONE open stream across an arrival, its frame fetch and its walk", async () => {
    let events = wire.watching.events;
    const signal = (): Events => ({ ...wire.events, events, to_move: wire.watching.to_move });
    const stream = fakeStream(() => [["signal", signal()]]);
    /** The arrival's game read, HELD — and it is what makes this cell able to
     *  see anything at all. Measured while writing it: with the read answering
     *  instantly, React collapses `busy` true and `busy` false into ONE commit,
     *  the effect never sees `active` go false, and the cell passed against the
     *  defect it is named for. A real network read is a macrotask wide; this is
     *  how a jsdom cell gets the same window. */
    const holding: Array<() => void> = [];
    let hold = false;
    const stub = await renderApp(wire.watching, (call) => {
      if (call.path.endsWith(`/api/games/${wire.watching.id}/stream`)) return stream.reply(call);
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.watching.id}`)) {
        if (!hold) return { ...wire.watching, events };
        return new Promise<unknown>((resolve) => {
          holding.push(() => resolve({ ...wire.watching, events }));
        });
      }
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k, events)) };
      return undefined;
    });
    // The hidden/visible round trip, for the reason the two cells above have
    // one: the first reader was opened under real timers, and the return opens
    // the next one on the fake clock this cell needs.
    const opened = stream.opens();
    await visibility("hidden");
    vi.useFakeTimers();
    try {
      await visibility("visible");
      await until(() => stream.opens() > opened, "the reader was opened again");
      await settle();
      const held = stream.opens();

      hold = true;
      events = wire.watching.events + 2;
      stream.push("signal", signal());
      await until(() => holding.length === 1, "the arrival's game read went out");
      await settle();
      // **The window itself**: the read is in flight, and no new reader has
      // been opened under it. (Before the fix the old one was aborted here and
      // replaced the moment `busy` cleared, which is what the count below
      // catches.)
      expect(stream.opens(), "the reader was not let go while the frame was fetched").toBe(held);
      hold = false;
      act(() => holding.forEach((release) => release()));
      await until(
        () => document.querySelector("ol.account li.at") !== null,
        "the arrival was walked",
      );
      await settle();
      // The walk really did run — without this the reader could have stayed put
      // because nothing ever reached the screen.
      expect(document.querySelectorAll("ol.account li")).toHaveLength(2);
      expect(stream.opens(), "the reader was not torn down and reopened").toBe(held);
      expect(stub.matching("GET", "/events"), "and nothing fell back to polling").toHaveLength(0);
    } finally {
      vi.useRealTimers();
    }
  });

  // **A frame that lands while the last one is still being fetched** (ruling
  // R-9.1). With the stream now HELD across an arrival this is reachable for
  // the first time: the server writes a frame per move, and a room can move
  // twice while one `GET …/{id}` is in flight. Re-entered, the handler would
  // read the same stale `state.events`, fetch the game again and walk the same
  // range a second time — two accounts of one turn, and one of them drawn over
  // the other.
  //
  // **This cell is the NO-OP half of the loop, and the cell below is the other**
  // (final review I-1). Three frames arrive, two of them under a held read that
  // is answered with the count they were announcing — so by the time the first
  // pass has landed, the state it produced already carries base+3 and the
  // second pass fetches NOTHING. Two reads in total, and the pending frame is
  // consumed by the `continue` rather than by a request.
  //
  // Read at its real size: it says a coalesced frame costs nothing when the
  // read already answered it. It CANNOT see a pending frame being dropped —
  // `frame = null` in place of `frame = pending.current` passes every
  // assertion here — which is what the cell below exists for.
  it("treats a frame the held read already answered as a no-op, and fetches once", async () => {
    const base = wire.watching.events;
    let events = base;
    const signal = (): Events => ({ ...wire.events, events, to_move: wire.watching.to_move });
    const stream = fakeStream(() => [["signal", signal()]]);
    /** The held game reads, resolved by hand. Each answers the count as it
     *  stands at RELEASE, which is what a server would say. */
    const holding: Array<() => void> = [];
    let hold = false;
    const stub = await renderApp(wire.watching, (call) => {
      if (call.path.endsWith(`/api/games/${wire.watching.id}/stream`)) return stream.reply(call);
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.watching.id}`)) {
        if (!hold) return { ...wire.watching, events };
        return new Promise<unknown>((resolve) => {
          holding.push(() => resolve({ ...wire.watching, events }));
        });
      }
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k, events)) };
      return undefined;
    });
    const reloads = () =>
      stub.calls.filter((c) => c.method === "GET" && /\/api\/games\/wire$/.test(c.path));
    expect(reloads()).toHaveLength(1);

    const opened = stream.opens();
    await visibility("hidden");
    vi.useFakeTimers();
    try {
      await visibility("visible");
      await until(() => stream.opens() > opened, "the reader was opened again");
      await settle();

      hold = true;
      events = base + 1;
      stream.push("signal", signal());
      await until(() => reloads().length === 2, "the first frame's game read went out");
      // The two that land underneath it.
      events = base + 2;
      stream.push("signal", signal());
      await settle();
      events = base + 3;
      stream.push("signal", signal());
      await settle();
      // **And nothing has happened yet**: one read in flight, no second one
      // beside it, and nothing walked.
      expect(reloads(), "the frames under the read did not each fetch a game").toHaveLength(2);
      expect(holding).toHaveLength(1);
      expect(document.querySelector("ol.account")).toBeNull();

      hold = false;
      act(() => holding.forEach((release) => release()));
      await until(
        () => document.querySelector("ol.account li.at") !== null,
        "the coalesced run was walked",
      );
      await settle();

      // TWO reads in all — the one the first frame sent and no other — and the
      // newest state is the one on screen: three events, walked as one run of
      // three steps. The follow-up pass ran and fetched nothing, which is the
      // no-op this cell is about.
      expect(reloads(), "the coalesced frames cost no request of their own").toHaveLength(2);
      expect(document.querySelectorAll("ol.account li")).toHaveLength(3);
      // Each position fetched ONCE: a second pass over the same range would
      // repeat them, which is what walking the same run twice looks like.
      const positions = positionsWalked(stub);
      expect(positions).toEqual([base, base + 1, base + 2, base + 3]);
      expect(stream.opens(), "and still one reader").toBe(opened + 1);
    } finally {
      vi.useRealTimers();
    }
  });

  // **The follow-up pass FETCHES, and this is the cell that can see it** (final
  // review I-1). The loop's second pass exists for one case and the no-op cell
  // above never drives it: A moves twice, and B's game read is answered with
  // the count as it stood when the request went OUT — base+1 — while the frame
  // for base+3 is already sitting in `pending`. The pass must then read again,
  // and without it B's screen sits one move behind until somebody else moves.
  //
  // **Mutation-proven**: replace `frame = pending.current` with `frame = null`
  // and this fails at the third read, where the cell above stays green.
  //
  // The account ends at TWO rows and not three, and the arithmetic is the
  // point: the first pass walked base→base+1 (one step) and the second walks
  // base+1→base+3 (two), `setRun` REPLACES, and the three events are accounted
  // for across the two runs. The positions below are what says so.
  it("fetches again when a frame under the read announced a count the read did not carry", async () => {
    const base = wire.watching.events;
    let events = base;
    const signal = (): Events => ({ ...wire.events, events, to_move: wire.watching.to_move });
    const stream = fakeStream(() => [["signal", signal()]]);
    /** The held game read, resolved by hand — and answering the count as it
     *  stood when the REQUEST went out, which is the whole of this case and the
     *  one thing the cell above does differently. */
    const holding: Array<() => void> = [];
    let hold = false;
    const stub = await renderApp(wire.watching, (call) => {
      if (call.path.endsWith(`/api/games/${wire.watching.id}/stream`)) return stream.reply(call);
      if (call.method === "GET" && call.path.endsWith(`/api/games/${wire.watching.id}`)) {
        if (!hold) return { ...wire.watching, events };
        const answered = events;
        return new Promise<unknown>((resolve) => {
          holding.push(() => resolve({ ...wire.watching, events: answered }));
        });
      }
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map((k) => at(k, events)) };
      return undefined;
    });
    const reloads = () =>
      stub.calls.filter((c) => c.method === "GET" && /\/api\/games\/wire$/.test(c.path));
    expect(reloads()).toHaveLength(1);

    const opened = stream.opens();
    await visibility("hidden");
    vi.useFakeTimers();
    try {
      await visibility("visible");
      await until(() => stream.opens() > opened, "the reader was opened again");
      await settle();

      hold = true;
      events = base + 1;
      stream.push("signal", signal());
      await until(() => reloads().length === 2, "the first frame's game read went out");
      // The two that land underneath it, the newest of which is what `pending`
      // holds when the read is released.
      events = base + 2;
      stream.push("signal", signal());
      await settle();
      events = base + 3;
      stream.push("signal", signal());
      await settle();
      expect(reloads(), "the frames under the read did not each fetch a game").toHaveLength(2);
      expect(holding).toHaveLength(1);
      expect(document.querySelector("ol.account")).toBeNull();

      hold = false;
      act(() => holding.forEach((release) => release()));
      // **The assertion the no-op cell cannot make.** The first pass landed at
      // base+1; the pending frame says base+3; the loop reads again.
      await until(() => reloads().length === 3, "the pending frame was fetched");
      await until(
        () => document.querySelectorAll("ol.account li").length === 2,
        "the second run reached the screen",
      );
      await settle();

      expect(reloads(), "and exactly one follow-up read, not one per frame").toHaveLength(3);
      // base→base+1 for the first pass, base+1→base+3 for the second: every
      // position fetched once, no range walked twice, and the three events
      // accounted for end to end.
      const positions = positionsWalked(stub);
      expect(positions).toEqual([base, base + 1, base + 1, base + 2, base + 3]);
      expect(stream.opens(), "and still one reader").toBe(opened + 1);
    } finally {
      vi.useRealTimers();
    }
  });
});
// ---- M6.4: the chat, and the stream that is now held at every status -------
//
// Four cells, and each is about ONE sentence this milestone wrote about the
// game screen: the stream is held whatever the prompt says (R-6.4.4), a frame
// that lands during this browser's own post is PARKED rather than walked, the
// panel is last in the side column and folded (R-6.4.10), and the two health
// switches are read at boot.

describe("the game screen's chat", () => {
  // **`active` is now "there is a game on screen" (R-6.4.4), and these are the
  // two statuses the old rule excluded.** A chat message is a change like any
  // other, and a table where the only person who could hear it is the one whose
  // turn it is not would be a chat that arrives when you stop playing. The two
  // Minors the M6.3 close left (M-1 and M-3 — no stream during your own turn,
  // none after the game is over) close with this.
  //
  // **Read at its real size**: what it asserts is that the reader is OPENED at
  // all under each state, which is the whole of what changed. Under the old
  // rule `until` never returns and the cell fails naming the wait.
  it("holds the table's stream during the player's own turn and after the game is over", async () => {
    for (const state of [wire.options, wire.gameOver]) {
      const stream = fakeStream();
      await renderApp(state, (call) => {
        if (call.path.endsWith(`/api/games/${state.id}/stream`)) return stream.reply(call);
        if (call.method === "GET" && call.path.includes("/chat")) return wire.chat;
        return undefined;
      });
      await waitFor(() => expect(stream.opens()).toBeGreaterThan(0));
      // ONE reader, not one per render: the same property every other cell in
      // this file holds the transport to.
      expect(stream.opens(), `one reader at ${state.prompt.kind}`).toBe(1);
      cleanup();
      // **The hash outlives the unmount**, and `renderApp` goes in through the
      // lobby: left at `#/t/wire` the second render mounts straight into the
      // game and there is no row to click. The suite's own `beforeEach` does
      // this between cells; a loop has to do it between turns.
      window.location.hash = "";
    }
  });

  // **The guard the held stream makes necessary (spec §6.4, mutation 12).**
  // With `active` no longer falling to false during the player's own turn, the
  // server's own bump for the move being posted now reaches this browser WHILE
  // the reply is in flight — and the arrival handler would read the game again,
  // against a count the reply is about to change, and walk this browser's own
  // move as somebody else's turn.
  //
  // So the frame is parked and compared once the reply has landed, and that is
  // what this cell pins. `send` also writes `stateNow.current` beside its
  // `setState` — belt and braces for the microtask-only case, since React's
  // default lane commits on a scheduler macrotask — but this cell does NOT
  // demonstrate it: `flush()` is twelve `act` rounds and React commits at every
  // one of them, so the render itself writes the ref long before the `finally`
  // reads it. A cell for that write wants a `landed` that resolves without
  // crossing a macrotask.
  it("parks a frame that lands while its own move is in flight, and does not walk it", async () => {
    const stream = fakeStream();
    const reply = gate<Reply>();
    const stub = await renderApp(wire.options, (call) => {
      if (call.path.endsWith(`/api/games/${wire.options.id}/stream`)) return stream.reply(call);
      if (call.method === "POST" && call.path.endsWith("/input")) return reply.promise;
      if (call.method === "GET" && call.path.includes("/chat")) return wire.chat;
      // The reply's own run and its log, served: the walk this cell counts is
      // a real one, not an error path.
      const ks = runRange(call.path);
      if (ks) return { frames: ks.map(() => wire.atK) };
      if (call.method === "GET" && call.path.includes("/log")) return wire.log;
      return undefined;
    });
    await waitFor(() => expect(stream.opens()).toBeGreaterThan(0));
    /** The game RE-READ, anchored on purpose — the idiom the arrival cells
     *  above already carry. `matching("GET", "/api/games/wire")` is a substring
     *  test that also counts `/at/0`, `/log`, `/table`, `/stream` and `/chat`,
     *  so it would read as "something was fetched" whatever happened. */
    const frames = () =>
      stub.calls.filter((c) => c.method === "GET" && /\/api\/games\/wire$/.test(c.path));
    const walked = () => positionsWalked(stub).length;

    const user = userEvent.setup();
    await user.click(firstOptionButton());
    const framesBefore = frames().length;
    const walkedBefore = walked();

    vi.useFakeTimers();
    try {
      // The server's own bump for THIS move reaches the stream before the reply
      // does, which is the ordering the guard exists for.
      stream.push("signal", {
        ...wire.events,
        events: wire.options.events + 1,
        chat: wire.chat.last,
      });
      await flush();
      expect(frames(), "nothing was re-read while posting").toHaveLength(framesBefore);
      expect(walked(), "and nothing was walked").toBe(walkedBefore);

      reply.resolve({ ...wire.watching, events: wire.options.events + 1 });
      await flush();
      // Compared NOW, against the state the reply produced: the counts are
      // equal, so the parked frame is a no-op and the game is not read a second
      // time for a move this browser made. (The `/at/` positions that follow
      // are the REPLY's own run, fetched by `send` — not the parked frame's.)
      expect(frames(), "the parked frame cost no read of its own").toHaveLength(framesBefore);

      // **And the guard was RELEASED.** Without this the cell passes just as
      // well against a client that drops every frame arriving during a post,
      // which is a screen that stops learning after its first move.
      stream.push("signal", {
        ...wire.events,
        events: wire.options.events + 2,
        chat: wire.chat.last,
      });
      await until(() => frames().length === framesBefore + 1, "the next frame was read");
    } finally {
      vi.useRealTimers();
    }
  });

  // LAST in the side column and FOLDED (R-6.4.10) — a bonus under the account
  // and the players, not the main attraction — and drawn for a WATCHER as well
  // as a player, which is the one route a spectator may write to (R-6.4.1).
  it("draws the chat panel last in the side column, folded, for a player and for a watcher", async () => {
    for (const state of [wire.options, wire.spectator]) {
      await renderApp(state, (call) =>
        call.method === "GET" && call.path.includes("/chat") ? wire.chat : undefined,
      );
      const side = document.querySelector("aside.side")!;
      await waitFor(() => expect(side.lastElementChild).toHaveClass("chat"));
      expect(side.lastElementChild).toHaveClass("folded");
      expect(within(side as HTMLElement).getByRole("button", { name: "show" })).toBeInTheDocument();
      cleanup();
      // See the cell above: the hash outlives the unmount and `renderApp` goes
      // in through the lobby.
      window.location.hash = "";
    }
  });

  /** **The message count belongs to the game on screen, and is dropped when the
   *  screen leaves it** (final review, minor 7).
   *
   *  The panel is re-keyed per game, so a switched table starts from an empty
   *  list and a cursor of nothing — but the COUNT it is handed was the previous
   *  table's until that table's own first frame landed. A folded panel read the
   *  old number as "N new" about a room it had just opened, and, where the old
   *  number was above the new room's `last`, asked one empty delta for it.
   *
   *  Leaving for the lobby and coming back is the cheapest switch that is a
   *  real one: `show` is the single place a game changes identity, and it runs
   *  on both halves of that trip.
   *
   *  The frame carries the SAME `events` as the state on screen, so nothing is
   *  re-read or walked and the only thing this cell moves is the chat count. */
  it("drops the table's message count when the screen leaves the game", async () => {
    const stream = fakeStream();
    const ahead = wire.chat.last + 5;
    await renderApp(wire.options, (call) => {
      if (call.path.endsWith(`/api/games/${wire.options.id}/stream`)) return stream.reply(call);
      if (call.method === "GET" && call.path.includes("/chat")) return wire.chat;
      return undefined;
    });
    await waitFor(() => expect(stream.opens()).toBeGreaterThan(0));
    stream.push("signal", { ...wire.events, events: wire.options.events, chat: ahead });
    expect(await screen.findByLabelText("5 new")).toBeInTheDocument();

    const user = userEvent.setup();
    await user.click(gameMenu());
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    await screen.findByRole("heading", { name: "Lobby" });
    await user.click(await screen.findByRole("link", { name: wire.options.id }));
    await screen.findByText(/end triggers:/);
    await waitFor(() => expect(screen.getByRole("button", { name: "show" })).toBeInTheDocument());
    // Real turns, so the remounted panel's own mount read has landed and the
    // badge is read against a cursor rather than against nothing.
    for (let i = 0; i < 4; i += 1)
      await act(async () => {
        await new Promise((r) => setTimeout(r, 0));
      });
    expect(
      screen.queryByLabelText(/new$/),
      "the count on screen is the one this table's own frame brought, not the last table's",
    ).toBeNull();
  });

  // The two launch variables, read at boot beside the transport (R-6.4.6,
  // R-6.4.8). They live in `api.ts` rather than in state for `signalKind`'s own
  // reason: one value for the life of the page, read by components that must
  // not re-render when it is set.
  it("reads the lobby switch and the text cap from health at boot", async () => {
    try {
      await renderApp(wire.options, (call) =>
        call.path.endsWith("/api/health")
          ? { ...wire.health, lobby_chat: false, chat_text: 77 }
          : undefined,
      );
      await waitFor(() => expect(lobbyChat()).toBe(false));
      expect(chatText()).toBe(77);
    } finally {
      // Module state, not component state: left as this cell found it, or every
      // cell after it in this file runs against a server it never described.
      setLobbyChat(true);
      setChatText(500);
    }
  });
});

// ---------------------------------------------------------------------------
// M6.5b: a game played ON THIS DEVICE (R-6.5b.12, spec §6.2).

describe("a local game", () => {
  const LOCAL = "local-1";
  /** A record whose seed would ROUND if anything on the way parsed it — the
   *  same needle the hosted Save game file cell uses, for the same reason. */
  const RECORD = '{"seed":18446744073709551615,"events":[]}';
  /** The local table document, as the store holds it. Sent BESIDE the record
   *  and never instead of it: the server checks that the two agree. */
  const TABLE = '{"schema":3,"id":"local-1","seed":18446744073709551615}';

  beforeEach(() => {
    resetLocalHostForTests();
    // **And on the way IN, not only on the way out** (the whole-branch
    // review's L-13): the `afterEach` below cleans up after THIS describe's
    // cells, but a cell from another describe that rendered `App` has already
    // left a fresh record behind — the corpus's health, `archive: "off"` — and
    // under a shuffled order (seeds 22 and 44) the first cell here then read
    // that answer from memory, never asked, and never drew "Send to the
    // archive".
    resetFactsForTest();
  });
  afterEach(() => {
    resetLocalHostForTests();
    // The deployment facts are a module singleton, and since M6.5b2 they carry
    // the TIME they were read: a cell that left a fresh record behind would
    // stop the next cell's own read from happening at all, as well as leaving
    // its archive on. The whole record goes back, not one field of it.
    resetFactsForTest();
  });

  /** The boot reads answered, and the SERVER LISTING NOTHING — so every row on
   *  screen is the local section's and a click on one proves where a local game
   *  is reachable from. */
  const bootStub = (
    extra: (call: ApiCall) => Reply | undefined = () => undefined,
    archive: "off" | "dir" = "off",
  ) =>
    stubApi((call) => {
      const mine = extra(call);
      if (mine !== undefined) return mine;
      if (call.path.endsWith("/api/me")) return wire.me;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      // The archive is the ONE named cross-read a local screen makes (spec
      // §2.3), and it is read from the server's own health — never set by hand
      // here, so the cells run the path the page runs.
      if (call.path.endsWith("/api/health")) return { ...wire.health, archive };
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      if (isGamesList(call)) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });

  /** The local game screen, opened the way a player opens it: the row in "On
   *  this device", one click. Every game route is answered by the WORKER. */
  async function openLocal(
    state: ClientState,
    opts: {
      hooks?: FakeWorkerHooks;
      archive?: "off" | "dir";
      server?: (call: ApiCall) => Reply | undefined;
    } = {},
  ): Promise<{ user: ReturnType<typeof userEvent.setup>; stub: ApiStub }> {
    const hooks = opts.hooks ?? {};
    stubWorker({
      ...hooks,
      call: (req) => {
        const mine = hooks.call?.(req);
        if (mine !== undefined) return mine;
        if (req.method === "GET" && req.path === `/api/games/${LOCAL}`)
          return { status: 200, body: state };
        if (req.path === `/api/games/${LOCAL}/input`) return { status: 200, body: state };
        return undefined;
      },
    });
    const stub = bootStub(opts.server, opts.archive);
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("link", { name: LOCAL }));
    return { user, stub };
  }

  /** Every `call` this tab has posted ON one game — the listing, which carries
   *  no id, is deliberately not one of them. */
  const callsOn = (id: string): string[] =>
    currentWorker()
      .posted.filter((p) => p.kind === "call" && p.path.startsWith(`/api/games/${id}`))
      .map((p) => (p.kind === "call" ? `${p.method} ${p.path}` : ""));

  /** The download's three globals, replaced for one cell and put back. */
  function catchDownload() {
    const parse = vi.spyOn(JSON, "parse");
    const clicks: { href: string; download: string }[] = [];
    const click = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(function (this: HTMLAnchorElement) {
        clicks.push({ href: this.href, download: this.download });
      });
    const create = vi.fn().mockReturnValue("blob:record");
    Object.defineProperty(URL, "createObjectURL", { value: create, configurable: true });
    Object.defineProperty(URL, "revokeObjectURL", { value: vi.fn(), configurable: true });
    return {
      clicks,
      create,
      parse,
      restore: () => {
        click.mockRestore();
        parse.mockRestore();
      },
    };
  }

  // **R-6.5b.7: at ANY point, and that is the whole of the difference.** A
  // hosted game's record is the server's and `GET …/{id}/record` answers 409
  // while it runs; a local one is the player's own seed on the player's own
  // disk, read off the Worker's side channel. The hosted half of this claim is
  // "offers no Save game file while the game runs", above.
  it("offers Save game file while a local game is still RUNNING, and hands the bytes over untouched", async () => {
    const caught = catchDownload();
    try {
      const { user, stub } = await openLocal(
        { ...wire.optionsMany, id: LOCAL },
        { hooks: { pair: () => ({ record: RECORD, table: TABLE }) } },
      );
      await screen.findByText(/end triggers:/);
      expect(wire.optionsMany.over, "the premise: this game is still running").toBe(false);

      await user.click(screen.getByRole("button", { name: "Save game file" }));
      await waitFor(() => expect(caught.clicks).toHaveLength(1));
      expect(caught.clicks[0]?.download).toBe(`${LOCAL}.json`);
      const blob = caught.create.mock.calls[0]?.[0] as Blob;
      expect(await readBlob(blob)).toBe(RECORD);
      // Never parsed on the way — a u64 seed would come back rounded — and
      // never fetched either: the `…/record` route answers 409 while a game
      // runs, and this save does not go through it.
      expect(
        caught.parse.mock.calls.some(([s]) => s === RECORD),
        "the record text went through JSON.parse",
      ).toBe(false);
      expect(stub.matching("GET", "/record")).toHaveLength(0);
      expect(callsOn(LOCAL).some((c) => c.includes("/record"))).toBe(false);
    } finally {
      caught.restore();
    }
  });

  // **The send, and the pair it sends** (R-6.5b.6, spec §2.5). One snapshot of
  // both documents, posted to the SERVER as the strings the store holds, and
  // the control goes dark once the copy has landed.
  it("sends the record and the table to the server's archive unparsed, says where, and goes dark", async () => {
    const caught = catchDownload();
    try {
      const { user, stub } = await openLocal(
        { ...wire.gameOver, id: LOCAL },
        {
          archive: "dir",
          hooks: { pair: () => ({ record: RECORD, table: TABLE }) },
          server: (call) =>
            call.method === "POST" && call.path === "/api/archive" ? wire.archivePost : undefined,
        },
      );
      await screen.findByText(/end triggers:/);

      await user.click(await screen.findByRole("button", { name: "Send to the archive" }));
      await waitFor(() => expect(stub.matching("POST", "/api/archive")).toHaveLength(1));
      // The two documents as they are stored, not as this client could rebuild
      // them: the record's bytes are what the terminal digest is over.
      expect(stub.matching("POST", "/api/archive")[0]?.body).toEqual({
        record: RECORD,
        table: TABLE,
      });
      expect(
        caught.parse.mock.calls.some(([s]) => s === RECORD || s === TABLE),
        "a document went through JSON.parse on its way to the archive",
      ).toBe(false);
      // Where the copy went, in the server's own words for the path.
      expect(await screen.findByText(`archived under ${wire.archivePost.path}`)).toBeInTheDocument();
      // And dark for the rest of the session: there is nothing left to click.
      expect(screen.queryByRole("button", { name: "Send to the archive" })).toBeNull();
      expect(stub.matching("POST", "/api/archive")).toHaveLength(1);
    } finally {
      caught.restore();
    }
  });

  // **Two clicks in one tick are ONE send** (whole-branch review, Minor #4).
  // The control darkens on the ANSWER, so it cannot be the guard: before the
  // wave a double click posted twice, the second landed as `identical`, and a
  // FIRST send reported itself as "already archived".
  //
  // **Both clicks inside ONE `act`**, the board tile's own idiom four hundred
  // lines up and for its reason: `userEvent` awaits between interactions, and
  // with a render in between the control is already gone — the cell would pass
  // with the guard removed, which is not a test of the guard. Measured: it
  // did.
  it("sends once however fast the archive control is clicked twice", async () => {
    const caught = catchDownload();
    try {
      const landed = gate<Reply>();
      const { stub } = await openLocal(
        { ...wire.gameOver, id: LOCAL },
        {
          archive: "dir",
          hooks: { pair: () => ({ record: RECORD, table: TABLE }) },
          server: (call) =>
            call.method === "POST" && call.path === "/api/archive" ? landed.promise : undefined,
        },
      );
      await screen.findByText(/end triggers:/);

      const button = await screen.findByRole("button", { name: "Send to the archive" });
      act(() => {
        fireEvent.click(button);
        fireEvent.click(button);
      });
      landed.resolve(wire.archivePost as unknown as Reply);
      await screen.findByText(`archived under ${wire.archivePost.path}`);
      expect(stub.matching("POST", "/api/archive")).toHaveLength(1);
      // The first send's own word, never the second's.
      expect(screen.queryByText(/already archived/)).toBeNull();
    } finally {
      caught.restore();
    }
  });

  // **The second click after a reload** (spec §2.5, §8): the local document
  // carries no stamp, so the control is live again — and the same bytes at the
  // same path are the server's `identical`, which is an answer and not a
  // refusal.
  it("says already archived when the server answers identical", async () => {
    const { user } = await openLocal(
      { ...wire.gameOver, id: LOCAL },
      {
        archive: "dir",
        hooks: { pair: () => ({ record: RECORD, table: TABLE }) },
        server: (call) =>
          call.method === "POST" && call.path === "/api/archive"
            ? { status: 200, body: wire.archivePostIdentical }
            : undefined,
      },
    );
    await screen.findByText(/end triggers:/);
    await user.click(await screen.findByRole("button", { name: "Send to the archive" }));
    expect(
      await screen.findByText(`already archived under ${wire.archivePostIdentical.path}`),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Send to the archive" })).toBeNull();
  });

  // A DIFFERENT record under this id: the server's sentence, and the control
  // stays live — nothing was saved, so nothing about this game has changed.
  it("shows the server's sentence on a 409 and leaves the control where it is", async () => {
    const { user } = await openLocal(
      { ...wire.gameOver, id: LOCAL },
      {
        archive: "dir",
        hooks: { pair: () => ({ record: RECORD, table: TABLE }) },
        server: (call) =>
          call.method === "POST" && call.path === "/api/archive"
            ? { status: 409, body: wire.archive409 }
            : undefined,
      },
    );
    await screen.findByText(/end triggers:/);
    await user.click(await screen.findByRole("button", { name: "Send to the archive" }));
    expect(await screen.findByText(wire.archive409.error)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Send to the archive" })).toBeInTheDocument();
  });

  // **A store that cannot be read is named as the STORE** (PR #37 fix wave,
  // fix round 1, review I-1). The Worker answers that case as an ordinary
  // `unreadable` reply and the pair read rejects with its sentence. This
  // control used to answer EVERY failure "the archive could not be reached",
  // which about a storage fault is a false sentence: the archive was never
  // asked, and nothing was sent.
  it("names a store it could not read as the store, sends nothing, and leaves the control live", async () => {
    const sentence =
      "this browser's storage could not be read — Connection to Indexed Database server lost.";
    const { user, stub } = await openLocal(
      { ...wire.gameOver, id: LOCAL },
      { archive: "dir", hooks: { pair: () => ({ unreadable: sentence }) } },
    );
    await screen.findByText(/end triggers:/);
    await user.click(await screen.findByRole("button", { name: "Send to the archive" }));
    expect(await screen.findByText(sentence)).toBeInTheDocument();
    expect(screen.queryByText("the archive could not be reached")).toBeNull();
    expect(stub.matching("POST", "/api/archive")).toHaveLength(0);
    expect(screen.getByRole("button", { name: "Send to the archive" })).toBeInTheDocument();
  });

  // **Dark unless the SERVER reported an archive** (spec §2.5). A deployment
  // that keeps none would answer the send with a 404, and a control that
  // promised a save it cannot make is the one thing the label rule forbids.
  it("offers no Send to the archive when the server keeps none", async () => {
    await openLocal({ ...wire.gameOver, id: LOCAL }, { archive: "off" });
    await screen.findByText(/end triggers:/);
    expect(screen.queryByRole("button", { name: "Send to the archive" })).toBeNull();
    // The player's own file is still theirs to save.
    expect(screen.getByRole("button", { name: "Save game file" })).toBeInTheDocument();
  });

  // A FINISHED HOSTED game never offers it, archive or no archive: the server
  // already holds that record, and this control exists to move one that lives
  // in a browser. Its own cell, because it starts from the lobby — the cell
  // above ends inside a game, and the hash outlives a `cleanup()`.
  it("offers no Send to the archive on a hosted game, archive or not", async () => {
    stubWorker();
    await renderApp(wire.gameOver, (call) =>
      call.path.endsWith("/api/health")
        ? { ...wire.health, archive: "dir" }
        : call.path.endsWith("/result")
          ? wire.result
          : undefined,
    );
    expect(screen.queryByRole("button", { name: "Send to the archive" })).toBeNull();
    // And no abandon on a hosted game's screen either: the lobby's row is
    // where a server table is abandoned (R-13), and P-6's move is the local
    // game's alone.
    expect(screen.queryByRole("button", { name: /^abandon / })).toBeNull();
  });

  // **Nothing is read before the lock answers** (P-5, spec §2.2): a cold GET on
  // a local game PUMPS the bot, so a read taken while another tab holds it
  // would change a record this tab does not own.
  it("draws the other tab's lock instead of reading the game at all", async () => {
    await openLocal({ ...wire.optionsMany, id: LOCAL }, { hooks: { hold: () => false } });
    expect(
      await screen.findByText("This game is open in another tab"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Take over" })).toBeEnabled();
    // Not one call on this game — not the table, not the state, not a panel.
    expect(callsOn(LOCAL)).toEqual([]);
    // And no answer bar, because there is no game on screen to answer.
    expect(document.querySelector("button.option")).toBeNull();
  });

  it("opens the game when Take over is clicked", async () => {
    const { user } = await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      // The first ask is refused and the STEAL is taken: `{ steal: true }` is
      // what Take over sends, and it is the one thing that separates the two.
      { hooks: { hold: (_id, steal) => steal } },
    );
    await screen.findByText("This game is open in another tab");
    await user.click(screen.getByRole("button", { name: "Take over" }));
    await screen.findByText(/end triggers:/);
    expect(currentWorker().posted.filter((p) => p.kind === "takeover")).toHaveLength(1);
  });

  // **The losing side** (spec §4). The screen goes read-only with Take back,
  // and a move attempted after the loss is refused IN THE PAGE — the Worker
  // never hears it, so nothing is shown that will not be kept.
  it("goes read-only when another tab takes the game, and the move never reaches the Worker", async () => {
    await openLocal({ ...wire.placeA, id: LOCAL });
    await screen.findByText(/end triggers:/);
    // The premise: this prompt makes the BOARD an input, which is the one
    // control that is not inside the answer bar the loss replaces.
    expect(wire.placeA.prompt.kind).toBe("place_a");
    const before = callsOn(LOCAL).length;

    act(() => currentWorker().emit({ kind: "lost", id: LOCAL }));

    expect(await screen.findByText("This game was opened in another tab")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Take back" })).toBeEnabled();
    expect(document.querySelector("button.option")).toBeNull();

    await userEvent.setup().click(screen.getByRole("button", { name: /^A00 / }));
    expect(screen.getByRole("alert")).toHaveTextContent("opened in another tab");
    expect(callsOn(LOCAL)).toHaveLength(before);
  });

  // **A SECOND local game is not read until ITS OWN hold has answered** (P-5,
  // review Important 1). The lock's answer carries no id of its own, so a
  // state read off the previous game — `"held"` for the one being left — would
  // admit the cold `GET` that PUMPS THE BOT in a record this tab does not own.
  // No existing cell could see it: the hook's id-change cell tests the hook
  // alone, and every in-app route into a local table passes through the lobby
  // first, so the switch needs a pasted address — which is what this drives.
  it("reads NOTHING of a second local game until its own hold has answered", async () => {
    const OTHER = "local-2";
    await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      // This tab holds the first game and NOT the second.
      { hooks: { hold: (id) => id === LOCAL } },
    );
    await screen.findByText(/end triggers:/);

    act(() => {
      window.location.hash = `#/t/${OTHER}`;
    });
    expect(await screen.findByText("This game is open in another tab")).toBeInTheDocument();
    // Not the table, not the state, not a panel — the whole of what P-5 is for.
    expect(callsOn(OTHER)).toEqual([]);
    // And the game being LEFT is off the screen, rather than sitting under an
    // address that no longer names it.
    expect(screen.queryByText(/end triggers:/)).toBeNull();
  });

  // **The losing tab writes NOTHING** (spec §4, review Important 2). `send` is
  // guarded, and so must be every other control that writes: the Worker fences
  // a delete from a tab that does not hold the game (answering `not-held`), so
  // a live Abandon would report a delete that did not happen — and with an
  // archive on, would post a STALE record first.
  it("takes the header's writing controls away when the lock is lost, and keeps the read", async () => {
    await openLocal(
      { ...wire.gameOver, id: LOCAL },
      { archive: "dir", hooks: { pair: () => ({ record: RECORD, table: TABLE }) } },
    );
    await screen.findByText(/end triggers:/);
    // The premise: both controls are there while this tab is the writer.
    //
    // **AWAITED, because "Send to the archive" waits on a read.** The archive
    // fact is fetched when the game ends (`ensureArchiveFact`, spec §2.2), so
    // the control appears a tick after `end triggers:` does — and a synchronous
    // `getByRole` here was a race this cell happened to win until M6.5b2 put
    // another component on the door and it started losing. The claim is
    // unchanged; only the wait is.
    expect(screen.getByRole("button", { name: /^abandon / })).toBeInTheDocument();
    await screen.findByRole("button", { name: "Send to the archive" });

    act(() => currentWorker().emit({ kind: "lost", id: LOCAL }));

    await screen.findByRole("button", { name: "Take back" });
    expect(screen.queryByRole("button", { name: /^abandon / })).toBeNull();
    expect(screen.queryByRole("button", { name: "Send to the archive" })).toBeNull();
    // Nothing left, nothing sent.
    expect(currentWorker().posted.filter((p) => p.kind === "call" && p.method === "DELETE")).toEqual(
      [],
    );
    // **The READ stays**, which is the line between read-only and useless: the
    // bytes in this browser are still the player's to keep (spec §4's rule for
    // the save that will not rebuild, and the same reason).
    expect(screen.getByRole("button", { name: "Save game file" })).toBeInTheDocument();
  });

  // **Take back RE-READS** (spec §3: "whoever holds the lock sees every move
  // the other tab played"). The route has not changed, so the effect that
  // opens a table does nothing here — without an explicit re-read the board
  // would stay at the position it held when it lost the lock.
  it("re-reads the game when it is taken back", async () => {
    await openLocal({ ...wire.optionsMany, id: LOCAL });
    await screen.findByText(/end triggers:/);
    act(() => currentWorker().emit({ kind: "lost", id: LOCAL }));
    await screen.findByRole("button", { name: "Take back" });
    const before = callsOn(LOCAL).filter((c) => c === `GET /api/games/${LOCAL}`).length;

    await userEvent.setup().click(screen.getByRole("button", { name: "Take back" }));
    await waitFor(() =>
      expect(
        callsOn(LOCAL).filter((c) => c === `GET /api/games/${LOCAL}`).length,
      ).toBeGreaterThan(before),
    );
    // And the answer bar is back, because this tab is the writer again.
    await waitFor(() => expect(screen.queryByRole("button", { name: "Take back" })).toBeNull());
  });

  // **Three differences and no more** (spec §2.5): no stream, no live strip,
  // and no Chat panel — the house is the server's and a local table has one
  // person at it.
  it("holds no stream, shows no live strip and draws no Chat panel", async () => {
    const { stub } = await openLocal({ ...wire.optionsMany, id: LOCAL });
    await screen.findByText(/end triggers:/);
    expect(stub.calls.filter((c) => c.path.includes(`/api/games/${LOCAL}/stream`))).toEqual([]);
    expect(callsOn(LOCAL).some((c) => c.includes("/stream") || c.includes("/events"))).toBe(false);
    expect(document.querySelector("span.meta.link")).toBeNull();
    expect(document.querySelector("section.panel.chat")).toBeNull();
    // Anti-vacuity for both: the screen DID draw, and it drew panels.
    expect(document.querySelector("header.banner")).not.toBeNull();
    expect(document.querySelectorAll("section.panel").length).toBeGreaterThan(0);
  });

  // **The one control a local game has that a hosted one does not** (P-6): the
  // abandon lives on the game's own screen, where this tab holds its lock, and
  // it reads "Abandon and save" when the SERVER keeps an archive.
  //
  // **The fact is REMEMBERED here rather than read at boot** (M6.5b2, spec
  // §2.2): a RUNNING local game is on the local channel and asks the server
  // nothing, so what this browser knows about the archive is what it was told
  // last time it was past the door. This is that browser. The read that makes
  // the arrangement unnecessary is Abandon's own, at the click, and belongs to
  // the outbox cycle (§2.6) — what this cell is about is the ORDER: the send
  // first, then the delete.
  it("abandons from its own screen, saving to the server's archive first", async () => {
    rememberServerFacts({ ...wire.health, archive: "dir" });
    const { user, stub } = await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      {
        archive: "dir",
        hooks: { pair: () => ({ record: RECORD, table: TABLE }) },
        server: (call) =>
          call.method === "POST" && call.path === "/api/archive" ? wire.archivePostAbandoned : undefined,
      },
    );
    await screen.findByText(/end triggers:/);
    const abandon = screen.getByRole("button", { name: /^abandon / });
    expect(abandon).toHaveTextContent("Abandon and save");

    await user.click(abandon);
    await user.click(screen.getByRole("button", { name: /^confirm abandon / }));
    // The send FIRST, then the local delete — the order R-6.5b.6 names.
    await waitFor(() => expect(stub.matching("POST", "/api/archive")).toHaveLength(1));
    await waitFor(() =>
      expect(currentWorker().posted.filter((p) => p.kind === "call" && p.method === "DELETE")).toHaveLength(1),
    );
    // And back to the lobby, where the game is no longer listed.
    await screen.findByRole("heading", { name: "On this device" });
  });

  // **Save game file names a refusal the way Send to the archive does** (PR #37
  // fix wave, fix round 2, the re-review's nit). Both controls read the same
  // pair and can meet the same storage sentence; one printed it as it is and
  // the other as `String(e)`, "Error: this browser's storage…", on one screen.
  // Both Save game file sites are held here: the game's header, and the
  // screen for a save that no longer rebuilds.
  const UNREADABLE =
    "this browser's storage could not be read — Connection to Indexed Database server lost.";
  it("shows Save game file's storage refusal as its sentence, with no Error: prefix", async () => {
    const caught = catchDownload();
    try {
      const { user } = await openLocal(
        { ...wire.optionsMany, id: LOCAL },
        { hooks: { pair: () => ({ unreadable: UNREADABLE }) } },
      );
      await screen.findByText(/end triggers:/);
      await user.click(screen.getByRole("button", { name: "Save game file" }));
      expect(await screen.findByText(UNREADABLE)).toBeInTheDocument();
      expect(screen.queryByText(/^Error: /)).toBeNull();
      expect(caught.clicks, "and nothing was downloaded").toHaveLength(0);
    } finally {
      caught.restore();
    }
  });

  it("does the same on the screen for a save that no longer rebuilds", async () => {
    const caught = catchDownload();
    try {
      const { user } = await openLocal(
        { ...wire.optionsMany, id: LOCAL },
        {
          hooks: {
            pair: () => ({ unreadable: UNREADABLE }),
            call: (req) =>
              req.path === `/api/games/${LOCAL}` && req.method === "GET"
                ? { status: 500, body: { error: "this save cannot be resumed" } }
                : undefined,
          },
        },
      );
      await screen.findByText("this save cannot be resumed");
      await user.click(screen.getByRole("button", { name: "Save game file" }));
      expect(await screen.findByText(UNREADABLE)).toBeInTheDocument();
      expect(screen.queryByText(/^Error: /)).toBeNull();
      expect(caught.clicks).toHaveLength(0);
    } finally {
      caught.restore();
    }
  });

  // **And Abandon names it the same way** (PR #37 whole-branch review, M-1).
  // "Abandon and save" reads the same pair before it sends, and its catch
  // printed the rejection as `String(e)` — the one control left saying
  // "Error: this browser's storage…" after the nit fixed the other three. The
  // game is KEPT: the pair was never read, so nothing was sent and the delete
  // never ran.
  it("shows Abandon's storage refusal as its sentence, with no Error: prefix, and keeps the game", async () => {
    rememberServerFacts({ ...wire.health, archive: "dir" });
    const { user, stub } = await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      { archive: "dir", hooks: { pair: () => ({ unreadable: UNREADABLE }) } },
    );
    await screen.findByText(/end triggers:/);
    await user.click(screen.getByRole("button", { name: /^abandon / }));
    await user.click(screen.getByRole("button", { name: /^confirm abandon / }));
    expect(await screen.findByText(UNREADABLE)).toBeInTheDocument();
    expect(screen.queryByText(/^Error: /)).toBeNull();
    expect(stub.matching("POST", "/api/archive"), "nothing was sent").toHaveLength(0);
    expect(
      currentWorker().posted.filter((p) => p.kind === "call" && p.method === "DELETE"),
      "and nothing was deleted",
    ).toHaveLength(0);
  });

  it("shows a save that no longer rebuilds as the ENGINE's sentence, and does not restart the host", async () => {
    // Spec §4: `Session::from_record` refusing is a 500 carrying the engine's
    // own words — a REPLY, never a trap — so the trap recovery is not entered
    // and its "reloaded from the last saved move" never appears. The two are
    // deliberately different surfaces: one says the save could not be read,
    // the other says it was recovered.
    const sentence =
      "this save was recorded under game-semantics revision 6, but this build plays revision 7 — the save cannot be resumed";
    // **And this browser KNOWS it** (M6.5c2 R-B19): the label reads the
    // REMEMBERED archive fact, and a local screen reads no health before
    // Abandon is clicked — so without this the label below was "Abandon" by
    // default, and a screen that passed `local` here as well as on a foreign
    // game's would have passed it.
    rememberServerFacts({ ...wire.health, archive: "dir" });
    const caught = catchDownload();
    try {
      const { user } = await openLocal(
        { ...wire.optionsMany, id: LOCAL },
        {
          // The server DOES keep an archive here, which is what makes the
          // label assertion below a claim rather than a default.
          archive: "dir",
          hooks: {
            pair: () => ({ record: RECORD, table: TABLE }),
            call: (req) =>
              req.path === `/api/games/${LOCAL}` && req.method === "GET"
                ? { status: 500, body: { error: sentence } }
                : undefined,
          },
        },
      );

      // **The sentence in place, on the game's own screen** — not a bounce to
      // the lobby, which is what a refusal that is not a trap deserves.
      await screen.findByText(sentence);
      expect(screen.queryByText(/reloaded from the last saved move/)).toBeNull();
      // Not a trap: one Worker for the whole cell.
      expect(FakeWorker.instances).toHaveLength(1);
      expect(FakeWorker.instances[0]!.terminated).toBe(false);

      // **Save game file still works, because it reads BYTES** (spec §4) — the
      // store holds a record this build cannot replay, and it is still the
      // player's to keep.
      await user.click(screen.getByRole("button", { name: "Save game file" }));
      await waitFor(() => expect(caught.clicks).toHaveLength(1));
      expect(await readBlob(caught.create.mock.calls[0]?.[0] as Blob)).toBe(RECORD);
      // And so does Abandon, which rebuilds nothing — and it reads "Abandon"
      // rather than "Abandon and save" even here, where the server keeps an
      // archive, because the archive would refuse THIS record: sending first
      // would leave a game that can never be abandoned.
      expect(screen.getByRole("button", { name: /^abandon / })).toHaveTextContent(/^Abandon$/);

      // **The row STAYS listed** (P-6): the listing reads table documents and
      // this game's is readable, so a save that will not rebuild is still
      // something a player can find, save as a file and abandon.
      //
      // **It is listed on the DOOR since M6.5b2** (R-6.5b2.1), which is where
      // the games in this browser live. The button says "Lobby" and goes to
      // the lobby — every control in this client that names that destination
      // still reaches it — so the way to this row is the lobby's own link back,
      // taken here as the address it carries.
      await user.click(screen.getByRole("button", { name: "Lobby" }));
      expect(await screen.findByRole("heading", { name: "Lobby" })).toBeInTheDocument();
      expect(screen.getByRole("link", { name: "Games on this device" })).toHaveAttribute(
        "href",
        "#/",
      );
      act(() => {
        window.location.hash = "#/";
      });
      expect(await screen.findByRole("link", { name: LOCAL })).toBeInTheDocument();
    } finally {
      caught.restore();
    }
  });

  // ---- M6.5c2: under a single-player build, no local control leads to a lobby
  //
  // **M6.5b2 close §6.3 item 5, closed** (spec §2.2): under
  // `VITE_MULTIPLAYER=false` four "Lobby" controls on LOCAL screens led to a
  // lobby whose game routes do not exist. Each now leads to the DOOR, where this
  // browser's games are listed, and says so. The anti-vacuity partners are the
  // cells that press "Lobby" on these screens in a build that serves one — ONE
  // per control, so a control sent to the door in every build fails its own:
  // the broken save's above, and the Game menu's, the lock screen's and the
  // local deal's directly below.
  it("keeps a local game's Game-menu way out to the lobby in a build that serves one", async () => {
    const { user } = await openLocal({ ...wire.optionsMany, id: LOCAL });
    await screen.findByText(/end triggers:/);
    await user.click(gameMenu());
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
  });

  it("keeps the other tab's lock screen's way out to the lobby in a build that serves one", async () => {
    const { user } = await openLocal({ ...wire.optionsMany, id: LOCAL }, { hooks: { hold: () => false } });
    await screen.findByText("This game is open in another tab");
    expect(screen.queryByRole("button", { name: "On this device" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
  });

  it("keeps the local deal's way out to the lobby in a build that serves one", async () => {
    const { user } = await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      {
        hooks: {
          // The corpus's dealing table, re-addressed to this device — the
          // one edit `fakeWorker.ts` allows a fixture.
          call: (req) =>
            req.path === `/api/games/${LOCAL}/table`
              ? { status: 200, body: { ...wire.tableDealing, id: LOCAL } }
              : undefined,
        },
      },
    );
    await screen.findByRole("heading", { name: "Choose your card" });
    expect(screen.queryByRole("button", { name: "On this device" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    await waitFor(() => expect(window.location.hash).toBe("#/lobby"));
  });

  // **A local game this build's bot cannot continue** (M6.5c2, spec §2.4). A
  // game dealt before this deploy has no bot in its mirror, so the door lists
  // it as playable; opening it is the first moment the module refuses.
  it("shows a local game this build's bot cannot continue as a screen: the sentence, Save game file, Abandon", async () => {
    const refusal = wire.foreignBot409;
    await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      {
        hooks: {
          pair: () => ({ record: RECORD, table: TABLE }),
          call: (req) =>
            req.path === `/api/games/${LOCAL}` && req.method === "GET"
              ? { status: 409, body: refusal }
              : undefined,
        },
      },
    );
    expect(await screen.findByText(refusal.error)).toBeInTheDocument();
    expect(window.location.hash).toBe(`#/t/${LOCAL}`);
    expect(screen.getByRole("button", { name: "Save game file" })).toBeInTheDocument();
    // Named by the TABLE's name, as the control names it (`Abandon.tsx`),
    // which the corpus table carries — hence the prefix, as the broken-save
    // cell above reads it.
    expect(screen.getByRole("button", { name: /^abandon / })).toBeInTheDocument();
  });

  // **And its Abandon SENDS FIRST, as every other local Abandon does**
  // (R-6.5b.6; M6.5c2 R-B19). Unlike a save that no longer rebuilds, the
  // archive takes this record — its check is a strict replay and asks nothing
  // of the bot — so the game is sent before it is deleted. ONE timeline across
  // the two hosts is what makes the order a claim: the server's archive write
  // and the module's delete, in the order they were made.
  it("abandons a local game this build's bot cannot continue by sending it to the archive first", async () => {
    rememberServerFacts({ ...wire.health, archive: "dir" });
    const order: string[] = [];
    const { user } = await openLocal(
      { ...wire.optionsMany, id: LOCAL },
      {
        archive: "dir",
        hooks: {
          pair: () => ({ record: RECORD, table: TABLE }),
          call: (req) => {
            if (req.path === `/api/games/${LOCAL}` && req.method === "GET")
              return { status: 409, body: wire.foreignBot409 };
            if (req.path === `/api/games/${LOCAL}` && req.method === "DELETE") {
              order.push("delete");
              return { status: 204, body: undefined };
            }
            return undefined;
          },
        },
        server: (call) => {
          if (call.method === "POST" && call.path === "/api/archive") {
            order.push("archive");
            return wire.archivePostAbandoned;
          }
          return undefined;
        },
      },
    );
    await screen.findByText(wire.foreignBot409.error);
    const abandon = screen.getByRole("button", { name: /^abandon / });
    expect(abandon).toHaveTextContent("Abandon and save");
    await user.click(abandon);
    await user.click(screen.getByRole("button", { name: /^confirm abandon / }));
    await waitFor(() => expect(order).toEqual(["archive", "delete"]));
    // And back to the door, where this browser's games are listed.
    await screen.findByRole("heading", { name: "On this device" });
  });

  describe("in a single-player build", () => {
    beforeEach(() => {
      vi.stubEnv("VITE_MULTIPLAYER", "false");
    });

    /** The one way out a local screen offers, pressed, and where it lands. */
    const leavesForTheDoor = async (user: ReturnType<typeof userEvent.setup>) => {
      expect(screen.queryByRole("button", { name: "Lobby" }), "no control says Lobby").toBeNull();
      await user.click(screen.getByRole("button", { name: "On this device" }));
      await waitFor(() => expect(window.location.hash).toBe("#/"));
    };

    it("sends the Game menu's way out to the door", async () => {
      const { user } = await openLocal({ ...wire.optionsMany, id: LOCAL });
      await screen.findByText(/end triggers:/);
      await user.click(gameMenu());
      await leavesForTheDoor(user);
    });

    it("sends the other tab's lock screen to the door", async () => {
      const { user } = await openLocal({ ...wire.optionsMany, id: LOCAL }, { hooks: { hold: () => false } });
      await screen.findByText("This game is open in another tab");
      await leavesForTheDoor(user);
    });

    it("sends the screen for a save that no longer rebuilds to the door", async () => {
      const sentence =
        "this save was recorded under game-semantics revision 6, but this build plays revision 7 \u2014 the save cannot be resumed";
      const { user } = await openLocal(
        { ...wire.optionsMany, id: LOCAL },
        {
          hooks: {
            pair: () => ({ record: RECORD, table: TABLE }),
            call: (req) =>
              req.path === `/api/games/${LOCAL}` && req.method === "GET"
                ? { status: 500, body: { error: sentence } }
                : undefined,
          },
        },
      );
      await screen.findByText(sentence);
      await leavesForTheDoor(user);
    });

    it("sends the local deal to the door", async () => {
      const { user } = await openLocal(
        { ...wire.optionsMany, id: LOCAL },
        {
          hooks: {
            // The corpus's dealing table, re-addressed to this device — the
            // one edit `fakeWorker.ts` allows a fixture.
            call: (req) =>
              req.path === `/api/games/${LOCAL}/table`
                ? { status: 200, body: { ...wire.tableDealing, id: LOCAL } }
                : undefined,
          },
        },
      );
      await screen.findByRole("heading", { name: "Choose your card" });
      await leavesForTheDoor(user);
    });
  });
});

// ---- the first screen with no server (M6.5b R-6.5b.8, M6.5b2 R-6.5b2.1) ----
//
// **The guarantee, in two code paths.** An API that REFUSES and an API that
// never ANSWERS are two different states of `useMe` — the first sets the
// door's error, the second leaves it null for ever — and before M6.5b each of
// them drew nothing at all: four `return`s above every screen. The games in
// this browser have to be reachable through both.
//
// **The first screen is the DOOR since M6.5b2**, and it is drawn from this
// browser alone: the games in its own IndexedDB and a link to the turnstile.
// The line that reports an unreachable API is the LOBBY's and stays there — a
// screen that asks the server nothing has nothing to report about it — so the
// cells below read the two screens at the two addresses.
describe("the first screen with no server", () => {
  const LOCAL = "local-1";

  beforeEach(() => {
    resetLocalHostForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
    vi.useRealTimers();
  });

  /** The local section, drawn and read. */
  const localSection = async () => {
    expect(await screen.findByRole("heading", { name: "On this device" })).toBeInTheDocument();
    expect(await screen.findByRole("link", { name: LOCAL })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "New game" })).toBeEnabled();
  };

  /** And its action WORKING, which is the half of R-6.5b.8 an enabled button
   *  does not prove: the form the click opens is behind the ROUTE, and the
   *  route effect used to be gated on a door that is shut in both of these
   *  cells. */
  const playsHere = async () => {
    await userEvent.setup().click(screen.getByRole("button", { name: "New game" }));
    expect(await screen.findByRole("heading", { name: "New game against the bot" })).toBeInTheDocument();
    // The form that needs no server: two seats, you and the bot. Scoped to
    // the seats: "The bot" is a First player segment too.
    expect(within(screen.getByRole("list", { name: "Seats" })).getByText("The bot")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Deal" })).toBeEnabled();
  };

  // **R-6.5b2.7: the door costs the Worker nothing.** The art manifest is a
  // static file and the words on the cards come from the local host, so neither
  // is a server route — and `/api/me` is not read either, which is the fourth
  // request an earlier draft of this cycle did not count.
  it("the first screen asks the server for nothing", async () => {
    stubWorker();
    const spy = vi.fn().mockRejectedValue(new Error("no server in this test"));
    vi.stubGlobal("fetch", spy);
    render(<App />);
    await screen.findByText(/On this device/);
    // The listing is the Worker's and lands after the render, so every read
    // this screen makes is given a real turn before they are counted.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    const asked = spy.mock.calls.map((c) => String(c[0]));
    expect(asked.filter((u) => u.startsWith("/api/"))).toEqual([]);
    // The anti-vacuity half: it DID fetch, just not the API.
    expect(asked).toContain("/art/manifest.json");
  });

  it("renders On this device with its games when every fetch REFUSES", async () => {
    stubWorker();
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("Failed to fetch");
      }),
    );
    render(<App />);
    await localSection();
    // **The door says nothing about the server, because it asks it nothing**
    // (R-6.5b2.7).
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
    // And the line that DOES report it is one hash away, where the four server
    // sections it replaces would be: the truth is still told, on the screen
    // whose subject the server is.
    act(() => {
      window.location.hash = "#/lobby";
    });
    expect(await screen.findByRole("heading", { name: "Can't reach the server" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Your tables" })).toBeNull();
    act(() => {
      window.location.hash = "#/";
    });
    await localSection();
    await playsHere();
  });

  it("renders On this device when the server never answers", async () => {
    stubWorker();
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise<Response>(() => {})),
    );
    render(<App />);
    await localSection();
    await playsHere();
  });

  // **R-6.5b.2 names TWO ways for the read to fail — "the API refusing or not
  // answering" — and only one of them rejects** (review fix 1). A `fetch` a
  // server accepts and then says nothing about settles neither way, so before
  // the health read was bounded this arm read `connecting…` for the life of
  // the page, and the cell above pinned the absence of the line that should
  // have replaced it.
  //
  // Fake timers, and no `userEvent`: what is being read is a DEADLINE, and the
  // only way to reach a five-second one in a test is to move the clock.
  it("says it can't reach the server once the bound elapses on a read that never answers", async () => {
    vi.useFakeTimers();
    stubWorker();
    vi.stubGlobal(
      "fetch",
      vi.fn(() => new Promise<Response>(() => {})),
    );
    // The LOBBY, because the line under test is the lobby's: the door reports
    // nothing about a server it never asks (R-6.5b2.7).
    window.location.hash = "#/lobby";
    render(<App />);
    await until(() => screen.queryByRole("heading", { name: "Lobby" }) !== null, "the lobby");
    // **The anti-vacuity partner.** Nothing has come back yet, and "can't
    // reach the server" is a claim this browser cannot make until its own
    // deadline says so — without this the cell would pass against a client that
    // drew the card before it had asked.
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    await until(
      () => screen.queryByRole("heading", { name: "Can't reach the server" }) !== null,
      "the card after the bound",
    );
    // And it replaces the sections, exactly as the refused arm's does.
    expect(screen.queryByRole("heading", { name: "Your tables" })).toBeNull();
  });

  // **"Try again" after a read that HUNG asks again, under a bound of its own**
  // (M6.5c2, state C). The health read's latch is released when a read
  // REJECTS; one that never settles would hold it, and "Try again" would then
  // put "Connecting to the server…" over no health read at all, for the life of
  // the page — ruling I-86's lesson, that a wait for a fact needs a bound.
  it("asks health again on Try again after a read that never answered, and says so again at the next bound", async () => {
    vi.useFakeTimers();
    stubWorker();
    const spy = vi.fn(() => new Promise<Response>(() => {}));
    vi.stubGlobal("fetch", spy);
    const healths = () => spy.mock.calls.filter((c: unknown[]) => String(c[0]).endsWith("/api/health")).length;
    window.location.hash = "#/lobby";
    render(<App />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    await until(
      () => screen.queryByRole("heading", { name: "Can't reach the server" }) !== null,
      "the card after the bound",
    );
    expect(healths()).toBe(1);

    fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await until(() => screen.queryByText(/Connecting to the server/) !== null, "connecting, once asked again");
    expect(healths(), "the health read, asked again").toBe(2);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    await until(
      () => screen.queryByRole("heading", { name: "Can't reach the server" }) !== null,
      "the card again, at the second read's bound",
    );
  });

  // **A listing that LANDED is an answer, and the bound must not take it back**
  // (whole-branch review, Minor #2). `Lobby`'s `onReached` sets `serverUp`
  // true the moment any listing arrives — that is what brings the sections
  // back for a player who opened the page while the API was slow — and the
  // bound's `catch` used to write `false` over it unconditionally. A health
  // read that is merely the slowest of the four then replaced a working lobby
  // with "can't reach the server".
  it("keeps the lobby when a listing landed before the health read's bound elapsed", async () => {
    vi.useFakeTimers();
    stubWorker();
    stubApi((call) => {
      // The one read that never answers is `/api/health`: everything else
      // lands, so the browser HAS reached this server.
      if (call.path.endsWith("/api/health")) return new Promise<Reply>(() => {});
      if (call.path.endsWith("/api/me")) return wire.me;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (isGamesList(call)) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/lobby";
    render(<App />);
    await until(() => screen.queryByRole("heading", { name: "Your tables" }) !== null, "the lobby");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    // The bound has passed and the health read is still out; the sections stay,
    // because something else already answered.
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
    expect(screen.getByRole("heading", { name: "Your tables" })).toBeInTheDocument();
  });

  // **A late answer is still an answer** (review fix 1). The bound decides
  // what the screen SAYS; it does not cancel the read, and the unbounded chain
  // beside it is what makes a server that finally speaks stop being one this
  // browser claims it cannot reach. A `Promise.race` alone would drop that
  // answer on the floor.
  it("stops saying it can't reach the server when the server answers after the bound", async () => {
    vi.useFakeTimers();
    stubWorker();
    const health = gate<Reply>();
    stubApi((call) => {
      if (call.path.endsWith("/api/health")) return health.promise;
      // The door is held too, which is what makes the line reachable at all:
      // with nothing answered, `connecting…` is what the server area would
      // otherwise read for ever.
      return new Promise<Reply>(() => {});
    });
    window.location.hash = "#/lobby";
    render(<App />);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    await until(() => screen.queryByRole("heading", { name: "Can't reach the server" }) !== null, "the card");

    health.resolve(wire.health);
    await until(
      () => screen.queryByText(/Connecting to the server/) !== null,
      "the door's own word, once the server answered",
    );
    expect(screen.queryByRole("heading", { name: "Can't reach the server" })).toBeNull();
    // **And the way to the games on this device was there throughout.** The
    // section itself is the door's since M6.5b2; what this screen owes a player
    // whose server will not answer is a way OUT of it, and that link is drawn
    // in every one of the server area's three states.
    expect(
      within(document.querySelector<HTMLElement>("header.lobby-band")!).getByRole("link", {
        name: "Games on this device",
      }),
    ).toBeInTheDocument();
  });

  it("plays a whole local game with every fetch REFUSING, the spy flat across three moves", async () => {
    // **The tightened fetch-at-zero cell** (spec §6.2): at Task 4 the door was
    // still a gate, so the boot reads were stubbed and counted as a baseline.
    // Now the boot reads FAIL and the game is opened from the local section,
    // which is the shape R-6.5b.8 promises.
    const localState = { ...wire.optionsMany, id: LOCAL };
    stubWorker({
      call: (req) => {
        if (req.path === `/api/games/${LOCAL}` && req.method === "GET")
          return { status: 200, body: localState };
        if (req.path === `/api/games/${LOCAL}/input`) return { status: 200, body: localState };
        return undefined;
      },
    });
    const spy = vi.fn(async () => {
      throw new TypeError("Failed to fetch");
    });
    vi.stubGlobal("fetch", spy);
    render(<App />);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("link", { name: LOCAL }));
    await screen.findByText(/end triggers:/);

    // Every boot read has already failed by now; from here the count must not
    // move, and a fetch that DID go out would be one more refusal.
    const baseline = spy.mock.calls.length;
    const prompt = wire.optionsMany.prompt;
    if (prompt.kind !== "options") throw new Error("fixture moved");
    const row = () =>
      screen.getByRole("button", {
        name: new RegExp(prompt.rows[0]!.words.slice(0, 20).replace(/[.*+?^${}()|[\]\\]/g, "\\$&")),
      });
    for (let i = 0; i < 3; i += 1) await user.click(row());
    await waitFor(() =>
      expect(
        currentWorker().posted.filter((p) => p.kind === "call" && p.path.endsWith("/input")),
      ).toHaveLength(3),
    );
    expect(spy.mock.calls.length).toBe(baseline);
  });
});

// ---- the words on the cards survive an unreachable API (spec §2.3) --------
//
// **LOCAL-FIRST on the local channel since M6.5b2** (spec §2.2). `/api/labels`
// is computed from the engine's registries, is deployment-independent, and the
// corpus replay proves both hosts emit it byte-identically — so the front door
// and a local game read the module in this tab's Worker and ask the server
// nothing at all (R-6.5b2.7). Past the door the SERVER is asked first, because
// it is the one that may have been updated, and the Worker is still the answer
// when that read fails or times out.
describe("the option words", () => {
  beforeEach(() => {
    resetLocalHostForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
    resetFactsForTest();
    vi.useRealTimers();
  });

  /** Every local labels read this test has made, across every Worker.
   *
   *  **Over the INSTANCES and not over `currentWorker()`**, because a lobby
   *  visit starts no Worker at all since M6.5b2: the local channel is what
   *  starts one, and "the server answered, so the module was never asked" has
   *  to be readable as zero rather than as a throw. */
  const askedLocally = () =>
    FakeWorker.instances
      .flatMap((w) => w.posted)
      .filter((p) => p.kind === "call" && p.path === "/api/labels");

  /** The four routes a signed-in LOBBY visit makes, with the labels read left
   *  to the caller — which is the one this describe is about. */
  const pastTheDoor = (labels: (call: ApiCall) => Reply | undefined) => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    window.location.hash = "#/lobby";
    return stubApi((call) => {
      if (call.path.endsWith("/api/labels")) return labels(call);
      if (call.path.endsWith("/api/me")) return wire.me;
      if (call.path.endsWith("/api/health")) return wire.health;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      if (isGamesList(call)) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
  };

  // **The door's own rule, and the one that is new this cycle** (R-6.5b2.7):
  // the words come from the module and `/api/labels` is not asked for at all.
  // The anti-vacuity partner is the cell below it, where the same client DOES
  // ask the server once it is past the door.
  it("reads them from the Worker at the door, and asks the server for nothing", async () => {
    stubWorker();
    const spy = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));
    vi.stubGlobal("fetch", spy);
    render(<App />);
    await waitFor(() => expect(askedLocally()).toHaveLength(1));
    expect(spy.mock.calls.map((c) => String(c[0])).filter((u) => u.startsWith("/api/"))).toEqual([]);
  });

  it("reads them from the Worker when the SERVER's labels read is REFUSED", async () => {
    stubWorker();
    pastTheDoor(() => ({ status: 503, body: wire.error }));
    render(<App />);
    // The screen opens ON the lobby, so the local channel's own read never
    // happens and this ONE local read is the server refusal's fallback. The
    // anti-vacuity partners are the cell above — where the server is not asked
    // at all — and the one below, where it answers and is kept.
    await waitFor(() => expect(askedLocally()).toHaveLength(1));
  });

  it("falls back inside the five-second bound when the server's read never settles", async () => {
    vi.useFakeTimers();
    stubWorker();
    // **A read that hangs never rejects**, so a `.catch` alone would wait for
    // ever. The bound is what turns silence into an answer.
    pastTheDoor(() => new Promise<Reply>(() => {}));
    render(<App />);
    await flush();
    // The anti-vacuity partner: BEFORE the bound the server is still being
    // waited on, so a cell that only looked after it would pass against a
    // client that asked both hosts at once.
    expect(askedLocally()).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5_000);
    });
    await until(() => askedLocally().length === 1, "the Worker was asked for the words");
  });

  it("leaves the Worker's words alone when the server answers", async () => {
    stubWorker();
    pastTheDoor(() => wire.labels);
    render(<App />);
    // The screen draws, the server answers, and the module is never asked for
    // the words at all — which on this channel also means no Worker is started
    // for them.
    await screen.findByRole("heading", { name: "Lobby" });
    await act(async () => {
      await new Promise((r) => setTimeout(r, 20));
    });
    expect(askedLocally()).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// M6.5b2: persistence is asked for when a game is worth keeping (R-6.5b2.9,
// spec §2.4).
//
// **The ask is at the FIRST COMMITTED MOVE and never at boot**, so a visitor
// who never plays is never asked — which in some browsers is a prompt and in
// all of them is a decision. These cells read the WHOLE path: `App` mounted at
// the door, a game opened from the local section, one option clicked, and the
// `persist()` the browser would have been asked for counted on either side of
// it.

describe("asking the browser to keep these games", () => {
  beforeEach(() => {
    resetLocalHostForTests();
    resetPersistenceRequestForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
    resetPersistenceRequestForTests();
    resetFactsForTest();
  });

  /** The door with no server behind it, which is the shape a local game is
   *  played in: every boot read fails and the section draws from the Worker. */
  const noServer = () =>
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => {
        throw new TypeError("Failed to fetch");
      }),
    );

  it("asks for persistence at the first committed move and not at boot", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({ persist, persisted: vi.fn().mockResolvedValue(false) });
    stubLocalGame();
    noServer();

    render(<App />);
    await screen.findByText(/On this device/);
    // The door drew, and read the REPORT — which is two questions and not the
    // ask. A visitor who stops here has been asked nothing.
    expect(persist).not.toHaveBeenCalled();

    await playOneLocalMove();
    await waitFor(() => expect(persist).toHaveBeenCalledTimes(1));
  });

  // **A move the store REFUSED is not a reason to ask.** The ask is for a game
  // worth keeping, and the browser has just said it is not keeping this one:
  // the answer to that is the backup line the reply already carries.
  it("does not ask when the move was applied and not saved", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({ persist, persisted: vi.fn().mockResolvedValue(false) });
    const state = { ...wire.optionsMany, id: "local-1" };
    stubWorker({
      call: (req) => {
        if (req.method === "GET" && req.path === "/api/games/local-1")
          return { status: 200, body: state };
        if (req.path === "/api/games/local-1/input")
          return { status: 200, body: state, saved: "unsaved" as const, reason: "the quota is full" };
        return undefined;
      },
    });
    noServer();

    render(<App />);
    await playOneLocalMove();
    // The move is on screen and the browser was never asked to keep it.
    expect(await screen.findByText(/APPLIED BUT NOT SAVED/)).toBeInTheDocument();
    expect(persist).not.toHaveBeenCalled();
  });

  // **The quota failure's path is the backup control** (spec §2.4): this cycle
  // points the existing line at it rather than inventing a second screen.
  it("points a refused write at the way to keep the game", async () => {
    stubStorage({ persisted: vi.fn().mockResolvedValue(false) });
    const state = { ...wire.optionsMany, id: "local-1" };
    stubWorker({
      call: (req) => {
        if (req.method === "GET" && req.path === "/api/games/local-1")
          return { status: 200, body: state };
        if (req.path === "/api/games/local-1/input")
          return { status: 200, body: state, saved: "unsaved" as const, reason: "the quota is full" };
        return undefined;
      },
    });
    noServer();

    render(<App />);
    await playOneLocalMove();
    const warning = await screen.findByText(/APPLIED BUT NOT SAVED/);
    // The server's own wording, carried verbatim, with the reason it gave.
    expect(warning).toHaveTextContent("APPLIED BUT NOT SAVED — the quota is full");
    // And one sentence more, naming the control that gets the game off this
    // browser — which is the only thing a player can actually do about it.
    expect(warning).toHaveTextContent(/back/i);
  });

  // The ask is once per PAGE, not once per game: a player who declines on
  // their first move is not asked again on their second.
  it("asks once across two moves", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({ persist, persisted: vi.fn().mockResolvedValue(false) });
    stubLocalGame();
    noServer();

    render(<App />);
    await playOneLocalMove();
    await waitFor(() => expect(persist).toHaveBeenCalledTimes(1));
    await userEvent.setup().click(firstOptionButton());
    await waitFor(() =>
      expect(
        currentWorker().posted.filter((p) => p.kind === "call" && p.path.endsWith("/input")),
      ).toHaveLength(2),
    );
    expect(persist).toHaveBeenCalledTimes(1);
  });
});

// ---- what this page asks the server about itself (M6.5b2, review M-1) ------
//
// **The latch is about a read that FAILED.** Entering the multiplayer channel
// is the first of the three read points, and it is asked once per page — but a
// read that never landed recorded nothing, and leaving the latch set kept the
// transport, the chat caps, the door's name and the archive at their defaults
// for the life of the page, even after the server came back and the listings
// started landing again.
describe("the facts this page asks the server for", () => {
  beforeEach(() => {
    resetLocalHostForTests();
    resetFactsForTest();
  });
  afterEach(() => {
    resetLocalHostForTests();
    resetFactsForTest();
  });

  it("asks again after a read that failed, and not after one that landed", async () => {
    let fail = true;
    const healths: number[] = [];
    const listings: number[] = [];
    stubWorker();
    stubApi((call) => {
      if (call.path.endsWith("/api/health")) {
        healths.push(1);
        if (fail) throw new TypeError("Failed to fetch");
        return wire.health;
      }
      if (isGamesList(call, "mine")) listings.push(1);
      if (call.path.endsWith("/api/me")) return wire.me;
      if (call.path.endsWith("/api/labels")) return wire.labels;
      if (call.path.endsWith("/art/manifest.json")) return wire.artManifest;
      if (call.path.endsWith("/api/stream")) return openStream();
      if (call.method === "GET" && call.path.startsWith("/api/chat"))
        return { ...wire.houseChat, messages: [], last: 0 };
      if (isGamesList(call)) return gamesNamed();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    window.location.hash = "#/lobby";
    render(<App />);
    await screen.findByRole("heading", { name: "Lobby" });
    await waitFor(() => expect(healths).toHaveLength(1));

    // The server comes back, and the player goes to the door and returns.
    fail = false;
    act(() => {
      window.location.hash = "#/";
    });
    await screen.findByRole("heading", { name: "On this device" });
    act(() => {
      window.location.hash = "#/lobby";
    });
    await screen.findByRole("heading", { name: "Lobby" });
    await waitFor(() => expect(healths).toHaveLength(2));

    // **And THIS one landed**, so a third visit asks nothing: the latch is
    // about a failure and not about the channel. Without this half the cell
    // would pass against a page that asked on every visit to the lobby.
    act(() => {
      window.location.hash = "#/";
    });
    await screen.findByRole("heading", { name: "On this device" });
    const listed = listings.length;
    act(() => {
      window.location.hash = "#/lobby";
    });
    await screen.findByRole("heading", { name: "Lobby" });
    // **The anti-vacuity half**: the lobby really did re-enter the multiplayer
    // channel and read its listings again, so "no third health read" is a
    // statement about the latch and not about a screen that never came up.
    await waitFor(() => expect(listings.length).toBeGreaterThan(listed));
    expect(healths).toHaveLength(2);
  });
});
