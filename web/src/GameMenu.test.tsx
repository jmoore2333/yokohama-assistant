// The session control in the banner: saved games, new game, no reload.

import { readFileSync } from "node:fs";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { gamesNamed, renderApp } from "./test/harness";
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
    await user.click(screen.getByText(new RegExp(wire.options.id)));
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
    await user.click(screen.getByText(new RegExp(wire.options.id)));
    await user.click(screen.getByRole("button", { name: "New game" }));
    expect(await screen.findByRole("heading", { name: "New game" })).toBeInTheDocument();
    // A reload in jsdom is a no-op that would pass; the source is the check.
    for (const f of ["App.tsx", "GameMenu.tsx"]) {
      expect(readFileSync(new URL(`./${f}`, here), "utf8")).not.toMatch(/location\.reload/);
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
    await userEvent.setup().click(screen.getByText(new RegExp(wire.options.id)));
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
