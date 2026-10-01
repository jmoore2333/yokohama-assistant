// The abandon control (R-13), as ONE component.
//
// **These cells exist because the two copies had already drifted.** The lobby's
// row guarded its DELETE against a second click and the open table's did not,
// and neither file's cells could see it: each clicked confirm once, so the
// invariant was asserted by nobody. The guard is now held here, in the one place
// that has it — and the double-click cell below is what holds it.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Abandon } from "./Abandon";
import { gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

/** The control over the corpus's own table, named `wire`. */
function show(handler: (call: ApiCall) => Reply | Promise<Reply> = () => null) {
  const onDone = vi.fn();
  const onError = vi.fn();
  const stub = stubApi(handler);
  render(<Abandon table={wire.table} onDone={onDone} onError={onError} />);
  return { stub, onDone, onError, user: userEvent.setup() };
}

/** Every DELETE this stub was sent, by whole path — `matching` is a substring
 *  test, and a table id is a prefix of longer ones. */
const deletes = (stub: { calls: ApiCall[] }) =>
  stub.calls.filter((c) => c.method === "DELETE").map((c) => c.path);

describe("the abandon control", () => {
  it("asks before it does anything", async () => {
    const { stub, user } = show();
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    // Nothing posted, and the question NAMES the table — so a confirmation
    // cannot be about a row other than the one that opened it. There is no
    // undo: the record is gone.
    expect(deletes(stub)).toEqual([]);
    expect(screen.getByRole("button", { name: "confirm abandon wire" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "cancel abandoning wire" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "abandon wire" })).toBeNull();
  });

  it("posts one DELETE on the second click, and says it is done", async () => {
    const { stub, onDone, user } = show();
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() => expect(deletes(stub)).toEqual(["/api/games/wire"]));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
  });

  // **The guard, and it is the finding this component was extracted for.** The
  // open table's copy had none: two clicks on confirm in one tick posted twice,
  // the second 404ing and setting a refusal on a screen the first was already
  // navigating away from. A ref and not a `busy` state, for the reason every
  // other guard in this client is one — React batches, so two clicks in a tick
  // both read a state flag as false.
  it("posts ONE DELETE however fast confirm is clicked twice", async () => {
    const held = gate<Reply>();
    const { stub, user } = show(() => held.promise);
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    const confirm = screen.getByRole("button", { name: "confirm abandon wire" });
    await user.click(confirm);
    await user.click(confirm);
    expect(deletes(stub)).toEqual(["/api/games/wire"]);
    held.resolve(null);
    await waitFor(() => expect(deletes(stub)).toEqual(["/api/games/wire"]));
  });

  it("puts the control back when the second click cancels", async () => {
    const { stub, onDone, user } = show();
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    await user.click(screen.getByRole("button", { name: "cancel abandoning wire" }));
    expect(screen.getByRole("button", { name: "abandon wire" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "confirm abandon wire" })).toBeNull();
    expect(deletes(stub)).toEqual([]);
    expect(onDone).not.toHaveBeenCalled();
  });

  // **A refusal is reported and the control stays where it is**, rather than
  // becoming a button that has silently stopped working. The guard is released
  // with it — without that, one failed attempt would latch the control shut for
  // the life of the screen.
  it("reports a refusal, stays asking, releases the guard, and can be tried again", async () => {
    let refuse = true;
    const { stub, onDone, onError, user } = show(() =>
      refuse ? { status: 500, body: { error: "the store is not writable" } } : null,
    );
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() => expect(onError).toHaveBeenCalledWith("the store is not writable"));
    expect(onDone).not.toHaveBeenCalled();
    // **Still confirming, so the retry is ONE click beside the sentence that
    // explains it.** Falling back to "Abandon" would make the player re-answer
    // a question they have already answered, about a table that is still there.
    expect(screen.getByRole("button", { name: "confirm abandon wire" })).toBeInTheDocument();

    refuse = false;
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    // Two attempts, two DELETEs — which is what says the guard was RELEASED on
    // the failure rather than latching the control shut for the life of the
    // screen. The double-click cell above is what says it was ever closed.
    expect(deletes(stub)).toEqual(["/api/games/wire", "/api/games/wire"]);
  });

  // `onDone` is awaited INSIDE the attempt, so a host whose follow-up work
  // fails — the lobby re-reads both listings after a delete — reports through
  // the same channel rather than into an unhandled rejection.
  it("reports a failure in what the host does afterwards", async () => {
    const onDone = vi.fn().mockRejectedValue(new Error("the lists could not be read"));
    const onError = vi.fn();
    stubApi(() => null);
    render(<Abandon table={wire.table} onDone={onDone} onError={onError} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() =>
      expect(onError).toHaveBeenCalledWith(expect.stringContaining("the lists could not be read")),
    );
  });

  it("names a table that was opened without a name by its id", async () => {
    stubApi(() => null);
    render(
      <Abandon table={{ ...wire.table, name: null }} onDone={() => {}} onError={() => {}} />,
    );
    // The id, and the same fallback the lobby row itself reads — so the button
    // and the row it sits beside cannot name the table two different ways.
    expect(screen.getByRole("button", { name: `abandon ${wire.table.id}` })).toBeInTheDocument();
  });
});
