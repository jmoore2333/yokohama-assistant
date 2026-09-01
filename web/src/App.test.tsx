// The app shell: what it sends, when it sends it, and what it will not send twice.

import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { gate, renderApp } from "./test/harness";
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
