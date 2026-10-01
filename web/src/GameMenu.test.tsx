// The session control in the banner: saved games, new game, no reload.

import { readFileSync } from "node:fs";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { gameMenu, gamesNamed, isGamesList, renderApp } from "./test/harness";
import { seatName } from "./tables";
import { wire } from "./wire/contract";

/** This file's own directory, as a `file:` URL.
 *
 *  **Hoisted into a constant, and that is not a style choice.** Written inline,
 *  `new URL(<expr>, import.meta.url)` is Vite's own static-asset reference
 *  pattern and is REWRITTEN at transform time: the first draft of the check
 *  below resolved to `http://localhost:3000/src/App.tsx` and died with ENOENT
 *  before it could assert anything. Through a variable it is an ordinary URL
 *  again, and names the file on disk. */
const here = import.meta.url;

describe("the Game menu", () => {
  it("lists the other saved games and switches to one", async () => {
    const other = { ...wire.options4p, id: "sim-other" };
    await renderApp(wire.options, (call) => {
      if (isGamesList(call, "mine"))
        return gamesNamed(wire.options.id, "sim-other");
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other")) return other;
      return undefined;
    });
    const user = userEvent.setup();
    await user.click(gameMenu());
    const button = await screen.findByRole("button", { name: "sim-other" });
    await user.click(button);
    // The summary now names the other game, and the list has RECOMPUTED: the
    // game we came from is the one now on offer.
    //
    // The menu is also still open — but jsdom cannot see that, and saying so
    // here would be a claim this file does not check. jsdom ships no UA
    // stylesheet for `details`, so a CLOSED menu's buttons are exactly as
    // findable (measured: `display: block`, `getByRole` resolves). The browser
    // cell in `e2e/the_table.spec.ts` is where the openness is really tested —
    // it clicks the way back without reopening the menu, which only works in
    // Chromium if the menu stayed open across the switch.
    await waitFor(() => expect(screen.getByText(/sim · sim-other/)).toBeInTheDocument());
    expect(screen.getByRole("button", { name: wire.options.id })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "sim-other" })).toBeNull();
  });

  it("returns to the lobby without a reload", async () => {
    await renderApp(wire.options, (call) =>
      isGamesList(call, "mine")
        ? gamesNamed(wire.options.id)
        : undefined,
    );
    const user = userEvent.setup();
    await user.click(gameMenu());
    // **The way out goes to the LOBBY since M6.2** (spec §5.1), not straight to
    // the form: the lobby is where a player picks between their tables and the
    // open ones, and its own "New table" is one click further on.
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    expect(await screen.findByRole("heading", { name: "Lobby" })).toBeInTheDocument();
    // And the hash says so, so Back works and a reload lands where the screen
    // does. **`#/lobby` since M6.5b2** (R-6.5b2.1): `#/` is the front door,
    // which lists the games in this browser, and a control labelled "Lobby"
    // goes to the lobby.
    expect(window.location.hash).toBe("#/lobby");
    // A reload in jsdom is a no-op that would pass; the source is the check.
    for (const f of ["App.tsx", "GameMenu.tsx"]) {
      expect(readFileSync(new URL(`./${f}`, here), "utf8")).not.toMatch(/location\.reload/);
    }
  });

  // The list is a list of TABLES since M6.1, and a row has to say enough for a
  // player with three games open to pick the right one: what it is CALLED, how
  // many seats it has, and whether it is waiting on them. Before this it said
  // an opaque id and nothing else.
  it("names each table, its size, and what it is waiting on", async () => {
    // Three rows off two corpus payloads. The menu hides the game you are
    // already in, so each has to be a table other than this one — `id` is
    // moved, and on two of them `name` is cleared, which is the `?? id`
    // fallback a table opened without a name takes.
    //
    // `dealing` keeps a name, and it is INVENTED rather than the corpus's
    // "wire": a table's name is free text a player typed, so there is no
    // server-authored value to preserve, and the corpus's happens to collide
    // with the id of the game this cell is playing.
    const dealing = { ...wire.games.games[0]!, id: "sim-dealing", name: "Tuesday game" };
    const waiting = { ...wire.table, id: "sim-waiting", name: null };
    // `my_seats` is the one field the server varies per caller, and the corpus
    // can only carry one caller's reading — so the row every player sees most
    // often, their own turn, is the corpus's table read from the other side.
    const yours = { ...wire.table, id: "sim-yours", name: null, my_seats: [wire.table.to_move!] };
    await renderApp(wire.options, (call) =>
      isGamesList(call, "mine")
        ? // The game being played is in the list too — `renderApp` opens it
          // from the setup screen's copy of this same list — and the menu is
          // then asserted NOT to offer it back.
          { games: [...gamesNamed(wire.options.id).games, dealing, waiting, yours] }
        : undefined,
    );
    await userEvent.setup().click(gameMenu());
    expect(screen.queryByRole("button", { name: wire.options.id })).toBeNull();

    const row = async (name: string) =>
      (await screen.findByRole("button", { name })).closest(".game-row")!;
    // The `name`, not the id: a table opened with a name is listed under it.
    expect(await row(dealing.name)).toHaveTextContent("2P · dealing");
    // The seat's NAME since M6.2 (spec §5.6): "Red" is what seat 0 called
    // itself, and `red@wire.test` is only how the table is keyed.
    expect(await row("sim-waiting")).toHaveTextContent("2P · waiting on Red");
    expect(await row("sim-yours")).toHaveTextContent("2P · your turn");

    // §7.2: WHO is at the table, which is how a player picks between three of
    // them. `tableWord` names at most one seat, and on the dealing row it names
    // none at all — so without this a lobby says nothing about the company.
    for (const seat of dealing.seats) {
      expect(await row(dealing.name)).toHaveTextContent(seatName(seat));
    }
  });

  // **The way out lives here on the game screen, and the reason is measured.**
  // The banner is gated at ONE ROW at 1440 and 1280 (`the_screen_itself.spec.ts`),
  // and M6.1's door put a strip and a button in it: 1325px of children and gaps
  // against a 1240px content box at 1280, which broke the line. The FACT ("You
  // are …") is read at a glance and stays in the banner; the CONTROL joins this
  // menu, where the screen's other session controls already are. The screens
  // with no Game menu — the form and the deal — keep both.
  it("carries the way out, beside the way to another game", async () => {
    await renderApp(wire.options, (call) =>
      isGamesList(call, "mine")
        ? gamesNamed(wire.options.id)
        : undefined,
    );
    // The strip stays in the banner, and the button is no longer a CHILD of it
    // — which is the half that keeps the banner one row. A direct-child query,
    // because the menu is itself a banner child and a subtree query would find
    // the button through it and prove nothing.
    expect(document.querySelector("header.banner")).toHaveTextContent(`You are ${wire.me.user}`);
    expect(
      document.querySelector("header.banner > button"),
      "the door's button is no longer a banner child",
    ).toBeNull();
    await userEvent.setup().click(gameMenu());
    const out = await screen.findByRole("button", { name: "sign out" });
    expect(out.closest(".game-menu")).not.toBeNull();
  });

  // **The same defect the app guards at its two other mutation controls**, in the
  // one place M5 added: `App`'s `inFlight` and `NewGame`'s carry the reasoning.
  // React batches, so two clicks in the same tick both read the guard's STATE as
  // false; the request that answers last wins, and it is not the one clicked
  // last. `fireEvent` twice inside one `act` is the only shape that reproduces
  // it — two `userEvent` clicks each flush, and the row is already `disabled`.
  it("a double-clicked saved game is fetched once", async () => {
    const other = { ...wire.options4p, id: "sim-other" };
    const stub = await renderApp(wire.options, (call) => {
      if (isGamesList(call, "mine"))
        return gamesNamed(wire.options.id, "sim-other");
      if (call.method === "GET" && call.path.endsWith("/api/games/sim-other")) return other;
      return undefined;
    });
    await userEvent.setup().click(gameMenu());
    const row = await screen.findByRole("button", { name: "sim-other" });
    // The GAME, not its board panels: `matching` is a substring test, and the
    // switched-to game's `/ports` and `/labs` fetches contain the id too.
    const gets = () =>
      stub.calls.filter((c) => c.method === "GET" && c.path.endsWith("/api/games/sim-other"));
    // **The FIRST request the switch makes, and since M6.1 that is `…/table`
    // rather than the game** — `App.open` asks what a table is before it fetches
    // the game, because a table still dealing refuses the game route. It is what
    // this cell has to count in the same tick: the game fetch is now one await
    // later, so counting it here would read zero however many clicks landed and
    // the guard could be deleted with the cell still green.
    const tableReads = () =>
      stub.calls.filter((c) => c.path.endsWith("/api/games/sim-other/table"));
    act(() => {
      fireEvent.click(row);
      fireEvent.click(row);
    });
    expect(tableReads()).toHaveLength(1);
    await waitFor(() => expect(screen.getByText(/sim · sim-other/)).toBeInTheDocument());
    expect(tableReads()).toHaveLength(1);
    expect(gets()).toHaveLength(1);
  });
});
