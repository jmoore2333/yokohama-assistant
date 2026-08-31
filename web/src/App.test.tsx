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
  // A `u64` does not fit a JS number and the setup flow ROUND-TRIPS the value,
  // so a rounded seed starts a different game than the one whose cards the
  // player chose between. `ClientState.seed` is the string to read;
  // `frame.title.seed` is `screen_model`'s number and is not safe here.
  it("is the lossless string, never the frame's rounded number", async () => {
    await renderApp(wire.options);
    expect(screen.getByText(new RegExp(`seed ${wire.options.seed}`))).toBeInTheDocument();
    // The two agree at this small fixture seed, which is what makes the
    // assertion above insufficient on its own: assert the FIELD, not the value.
    expect(typeof wire.options.seed).toBe("string");
    expect(typeof wire.options.frame.title.seed).toBe("number");
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
    expect(screen.getByText(/seed /)).toBeInTheDocument();
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
