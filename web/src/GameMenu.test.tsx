// The session control in the banner: saved games, new game, no reload.

import { readFileSync } from "node:fs";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { gameMenu, gamesNamed, renderApp } from "./test/harness";
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
      if (call.method === "GET" && call.path.endsWith("/api/games"))
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

  it("returns to the setup screen without a reload", async () => {
    await renderApp(wire.options, (call) =>
      call.method === "GET" && call.path.endsWith("/api/games")
        ? gamesNamed(wire.options.id)
        : undefined,
    );
    const user = userEvent.setup();
    await user.click(gameMenu());
    await user.click(screen.getByRole("button", { name: "New game" }));
    expect(await screen.findByRole("heading", { name: "New game" })).toBeInTheDocument();
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
      call.method === "GET" && call.path.endsWith("/api/games")
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
    expect(await row("sim-waiting")).toHaveTextContent("2P · waiting on red@wire.test");
    expect(await row("sim-yours")).toHaveTextContent("2P · your turn");

    // §7.2: WHO is at the table, which is how a player picks between three of
    // them. `tableWord` names at most one seat, and on the dealing row it names
    // none at all — so without this a lobby says nothing about the company.
    for (const seat of dealing.seats) {
      expect(await row(dealing.name)).toHaveTextContent(
        seat.kind === "bot" ? "the bot" : seat.user,
      );
    }
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
      if (call.method === "GET" && call.path.endsWith("/api/games"))
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
    act(() => {
      fireEvent.click(row);
      fireEvent.click(row);
    });
    expect(gets()).toHaveLength(1);
    await waitFor(() => expect(screen.getByText(/sim · sim-other/)).toBeInTheDocument());
    expect(gets()).toHaveLength(1);
  });
});
