// The abandon control (R-13), as ONE component.
//
// **These cells exist because the two copies had already drifted.** The lobby's
// row guarded its DELETE against a second click and the open table's did not,
// and neither file's cells could see it: each clicked confirm once, so the
// invariant was asserted by nobody. The guard is now held here, in the one place
// that has it — and the double-click cell below is what holds it.

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Abandon } from "./Abandon";
import { setArchiveKind } from "./api";
import { resetLocalHostForTests } from "./local/client";
import { stubWorker } from "./test/fakeWorker";
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

  // **The label is the near truth** (M6.5a, R-6.5a.13): "Abandon and save" at a
  // PLAYING table when this deployment keeps an archive, "Abandon" otherwise —
  // and the accessible name stays `abandon <table>` in every case, so nothing
  // that clicks by name moves.
  it("reads Abandon and save at a playing table when the archive is on, and Abandon otherwise", () => {
    expect(wire.table.status, "the premise: the corpus table is playing").toBe("playing");
    expect(wire.tableOpen.status).toBe("open");
    try {
      setArchiveKind("dir");
      render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);
      expect(screen.getByRole("button", { name: "abandon wire" })).toHaveTextContent("Abandon and save");
      cleanup();
      setArchiveKind("off");
      render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);
      expect(screen.getByRole("button", { name: "abandon wire" })).toHaveTextContent(/^Abandon$/);
      cleanup();
      setArchiveKind("dir");
      render(<Abandon table={wire.tableOpen} onDone={() => {}} onError={() => {}} />);
      expect(screen.getByRole("button", { name: `abandon ${wire.tableOpen.name ?? wire.tableOpen.id}` }))
        .toHaveTextContent(/^Abandon$/);
    } finally {
      setArchiveKind("off");
    }
  });
});

// ---- a LOCAL game's abandon (M6.5b, R-6.5b.6, R-6.5b.7) -------------------
//
// **The order is the whole rule.** A game on this device exists nowhere else,
// so "Abandon and save" sends the record and its table document to the
// SERVER's archive BEFORE the local delete — and a send that does not land
// keeps the game, because a delete that followed it would be the only copy
// gone. The one exception is the server's own `identical`: an earlier send
// landed and only the delete failed.
describe("the abandon control on a local game", () => {
  const LOCAL = "local-1";
  /** A record whose seed would round if anything parsed it, and its table. */
  const RECORD = '{"seed":18446744073709551615,"events":[]}';
  const TABLE = '{"schema":3,"id":"local-1"}';
  /** The row this control sits beside: the corpus's own table, re-addressed to
   *  this device — which is what routes its DELETE to the Worker. */
  const localTable = { ...wire.table, id: LOCAL, name: null };

  beforeEach(() => {
    resetLocalHostForTests();
  });
  afterEach(() => {
    resetLocalHostForTests();
    setArchiveKind("off");
  });

  /** The control over a local table, with the two hosts scripted and every
   *  step recorded in the order it happened. */
  function local(archive: "off" | "dir", onArchive: () => Reply | Promise<Reply>) {
    const steps: string[] = [];
    setArchiveKind(archive);
    stubWorker({
      pair: () => ({ record: RECORD, table: TABLE }),
      call: (req) => {
        if (req.method === "DELETE") steps.push("delete");
        return undefined;
      },
    });
    const stub = stubApi(async (call) => {
      if (call.method === "POST" && call.path === "/api/archive") {
        steps.push("archive");
        return await onArchive();
      }
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const onDone = vi.fn();
    const onError = vi.fn();
    render(<Abandon table={localTable} local onDone={onDone} onError={onError} />);
    return { steps, stub, onDone, onError, user: userEvent.setup() };
  }

  /** Both clicks. */
  const confirm = async (user: ReturnType<typeof userEvent.setup>) => {
    await user.click(screen.getByRole("button", { name: `abandon ${LOCAL}` }));
    await user.click(screen.getByRole("button", { name: `confirm abandon ${LOCAL}` }));
  };

  it("sends the pair to the archive BEFORE the local delete", async () => {
    const { steps, stub, onDone, user } = local("dir", () => wire.archivePostAbandoned);
    await confirm(user);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(steps).toEqual(["archive", "delete"]);
    // Both documents as the store holds them, one snapshot, neither parsed.
    expect(stub.matching("POST", "/api/archive")[0]?.body).toEqual({
      record: RECORD,
      table: TABLE,
    });
  });

  // **"Already archived" PROCEEDS** (spec §4). A 200 `identical` means an
  // earlier send landed and only the delete failed; refusing here would leave
  // a game that can never be abandoned.
  it("proceeds to the delete when the server answers identical", async () => {
    const { steps, onDone, user } = local("dir", () => ({
      status: 200,
      body: wire.archivePostIdentical,
    }));
    await confirm(user);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(steps).toEqual(["archive", "delete"]);
  });

  // **A 409 is a DIFFERENT record under this id, and it is not proof that this
  // one landed** — so the game is KEPT. An earlier draft deleted on it.
  it("keeps the game on a 409, and says what the server said", async () => {
    const { steps, onDone, onError, user } = local("dir", () => ({
      status: 409,
      body: wire.archive409,
    }));
    await confirm(user);
    await waitFor(() => expect(onError).toHaveBeenCalledWith(wire.archive409.error));
    expect(steps).toEqual(["archive"]);
    expect(onDone).not.toHaveBeenCalled();
    // Still asking, so the player can try again beside the sentence.
    expect(screen.getByRole("button", { name: `confirm abandon ${LOCAL}` })).toBeInTheDocument();
  });

  it("keeps the game when the record is refused, in the server's own words", async () => {
    const { steps, onDone, onError, user } = local("dir", () => ({
      status: 400,
      body: wire.archive400Replay,
    }));
    await confirm(user);
    await waitFor(() => expect(onError).toHaveBeenCalledWith(wire.archive400Replay.error));
    expect(steps).toEqual(["archive"]);
    expect(onDone).not.toHaveBeenCalled();
  });

  // **The archive unreachable** (R-6.5b.7): the client's own sentence, because
  // there is no server sentence to carry — and the game stays.
  it("keeps the game when the archive cannot be reached", async () => {
    const { steps, onDone, onError, user } = local("dir", () => {
      throw new TypeError("Failed to fetch");
    });
    await confirm(user);
    await waitFor(() =>
      expect(onError).toHaveBeenCalledWith("the archive could not be reached — the game was kept"),
    );
    expect(steps).toEqual(["archive"]);
    expect(onDone).not.toHaveBeenCalled();
  });

  // No archive on this server: nothing to send, and the delete is local only.
  it("deletes without sending anything when the server keeps no archive", async () => {
    const { steps, onDone, user } = local("off", () => {
      throw new Error("the archive must not be asked");
    });
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(/^Abandon$/);
    await confirm(user);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(steps).toEqual(["delete"]);
  });

  // **A local table with no `local` saves NOTHING, and says so** — the screen
  // for a save that no longer rebuilds is the one caller that withholds it,
  // because the archive would refuse that very record (spec §4: "Abandon still
  // works"). A label promising a copy nobody can make is what this rule exists
  // to stop.
  it("reads Abandon, and sends nothing, for a local table this control does not archive", async () => {
    setArchiveKind("dir");
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    const stub = stubApi(() => {
      throw new Error("the archive must not be asked");
    });
    const onDone = vi.fn();
    render(<Abandon table={localTable} onDone={onDone} onError={() => {}} />);
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(/^Abandon$/);
    await confirm(userEvent.setup());
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(stub.calls).toEqual([]);
  });

  it("reads Abandon and save when the server keeps an archive", () => {
    setArchiveKind("dir");
    stubWorker();
    stubApi(() => null);
    render(<Abandon table={localTable} local onDone={() => {}} onError={() => {}} />);
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(
      "Abandon and save",
    );
  });
});
