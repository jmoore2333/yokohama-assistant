// The unlock face (spec §5.3): the screen a would-be SPECTATOR lands on when the
// table they followed asks for a password.
//
// Every payload here is the corpus's — `table_watchable.json` is a locked
// playing table read by a stranger (`my_seats: []`, `can_watch: false`), which
// is the exact document that draws this screen, and `unlock_403.json` is the
// server's own refusal.

import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Unlock } from "./Unlock";
import { stubApi, type ApiCall, type Reply } from "./test/harness";
import { seatName } from "./tables";
import { wire } from "./wire/contract";

const TABLE = wire.tableWatchable;

function show(handler: (call: ApiCall) => Reply | undefined = () => undefined) {
  const onUnlocked = vi.fn();
  const onLeave = vi.fn();
  const stub = stubApi((call) => {
    const mine = handler(call);
    if (mine !== undefined) return mine;
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  render(
    <Unlock
      game={TABLE.id}
      table={TABLE}
      onUnlocked={onUnlocked}
      onLeave={onLeave}
      who={<span className="meta who">You are red@wire.test</span>}
    />,
  );
  return { stub, onUnlocked, onLeave, user: userEvent.setup() };
}

describe("the unlock face", () => {
  // What a person deciding whether to spectate needs: which table this is, who is
  // at it, and that it wants a password. The facts are `TableFacts`' — the same
  // line the lobby and the Game menu draw — so a table cannot read one way here
  // and another there.
  it("names the table, its seats and the password it wants", () => {
    show();
    // `level: 3`: the band's last crumb is the page's `<h2>` and carries the
    // table's name too, so the page's own title is the heading a level under.
    expect(screen.getByRole("heading", { name: TABLE.name ?? TABLE.id, level: 3 })).toBeInTheDocument();
    for (const seat of TABLE.seats) {
      expect(screen.getByText(new RegExp(seatName(seat)))).toBeInTheDocument();
    }
    expect(screen.getByText(new RegExp(`${TABLE.players}P`))).toBeInTheDocument();
    expect(screen.getByLabelText("Table password")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Spectate" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Lobby" })).toBeInTheDocument();
    // **"Spectate", in every word a person reads** (R-6.5c2.7). The wire keeps
    // `can_watch`; the screen says nothing of watching.
    expect(screen.getByText(/password required to spectate/)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/\bwatch/i);
  });

  // **Nothing is sent for an empty box.** The server answers a blank password
  // at a locked table with the same 403 as a wrong one, so a live button there
  // spends a request to be told what the screen already knows.
  it("will not send an empty password", () => {
    show();
    expect(screen.getByRole("button", { name: "Spectate" })).toBeDisabled();
  });

  it("posts the password and hands over when the server takes it", async () => {
    const { stub, onUnlocked, user } = show((call) =>
      call.path.endsWith("/unlock") ? { ...TABLE, can_watch: true } : undefined,
    );
    await user.type(screen.getByLabelText("Table password"), "  hunter2  ");
    await user.click(screen.getByRole("button", { name: "Spectate" }));
    await waitFor(() => expect(onUnlocked).toHaveBeenCalledTimes(1));
    const posts = stub.matching("POST", "/unlock");
    expect(posts).toHaveLength(1);
    expect(posts[0]!.path).toContain(`/api/games/${TABLE.id}/unlock`);
    // TRIMMED, the way every other password box on this client sends one: a
    // pasted password carries the spaces around it and the server compares
    // bytes.
    expect(posts[0]!.body).toEqual({ password: "hunter2" });
  });

  // The server's own sentence, and the screen stays on this face: a spectator who
  // typed the wrong password has another go, and a hand-over here would have
  // opened a table they cannot see.
  it("shows the server's refusal for a wrong password and does not hand over", async () => {
    const { onUnlocked, onLeave, user } = show((call) =>
      call.path.endsWith("/unlock") ? { status: 403, body: wire.unlock403 } : undefined,
    );
    await user.type(screen.getByLabelText("Table password"), "nope");
    await user.click(screen.getByRole("button", { name: "Spectate" }));
    expect(await screen.findByText(wire.unlock403.error)).toBeInTheDocument();
    expect(onUnlocked).not.toHaveBeenCalled();
    expect(onLeave).not.toHaveBeenCalled();
    // And the face is still here to try again on.
    expect(screen.getByRole("button", { name: "Spectate" })).toBeInTheDocument();
  });

  it("goes back to the lobby", async () => {
    const { onLeave, user } = show();
    await user.click(screen.getByRole("button", { name: "Lobby" }));
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  // **The double click, and a ref is what stops it** (`App`'s `inFlight` and
  // `OpenTable`'s carry the same reasoning). React batches, so two handlers
  // running before a re-render both read a `busy` STATE as false and both post
  // — and two unlocks is two requests where the second can only race the
  // hand-over the first is about to make.
  //
  // `fireEvent` twice inside one `act`, because `userEvent` awaits between
  // interactions, which is precisely the race not happening.
  it("posts once when the button is double-clicked", async () => {
    const { stub, user } = show((call) =>
      call.path.endsWith("/unlock") ? { ...TABLE, can_watch: true } : undefined,
    );
    await user.type(screen.getByLabelText("Table password"), "hunter2");
    const spectate = screen.getByRole("button", { name: "Spectate" });
    act(() => {
      fireEvent.click(spectate);
      fireEvent.click(spectate);
    });
    await waitFor(() => expect(stub.matching("POST", "/unlock")).toHaveLength(1));
    expect(stub.matching("POST", "/unlock")).toHaveLength(1);
  });
});
