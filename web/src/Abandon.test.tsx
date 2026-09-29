// The abandon control (R-13), as ONE component.
//
// **These cells exist because the two copies had already drifted.** The lobby's
// row guarded its DELETE against a second click and the open table's did not,
// and neither file's cells could see it: each clicked confirm once, so the
// invariant was asserted by nobody. The guard is now held here, in the one place
// that has it — and the double-click cell below is what holds it.

// **`fake-indexeddb` since M6.5b2**, because the refusal face's "Back it up to
// a file" runs the real `backupOne`, which opens this browser's own database.
import "fake-indexeddb/auto";
import { IDBFactory } from "fake-indexeddb";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Abandon, CANNOT_ASK, NO_ANSWER } from "./Abandon";
import { rememberServerFacts } from "./api";
import { resetLocalHostForTests } from "./local/client";
import { archiveAnswer, resetFactsForTest } from "./local/facts";
import { stubWorker } from "./test/fakeWorker";
import { gate, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

/** What this browser was last TOLD about the deployment (M6.5b2).
 *
 *  **`rememberServerFacts` and not a one-field setter**, because the third
 *  state is the TIMESTAMP: a fact patched by hand is not a fact the server
 *  answered, and "nothing has ever answered" is exactly what this cycle's
 *  Abandon has to be able to tell from `"off"`. */
const serverSays = (archive: "off" | "dir") =>
  rememberServerFacts({ ...wire.health, archive });

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

beforeEach(() => {
  globalThis.indexedDB = new IDBFactory();
  resetFactsForTest();
});
afterEach(() => {
  resetFactsForTest();
});

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

  // **A request that got NO answer is said in a player's words** (M6.5c2; the
  // Task 5 review's note). `fetch` failing rejects with the platform's own
  // `TypeError: Failed to fetch`, a message to a programmer, and a server
  // table's DELETE — or the lobby's re-read after it, awaited in the same
  // attempt — handed that text to the host's strip as it stood. The Lobby's
  // own listings already say it in words (`Lobby.tsx`'s `UNREACHED`).
  it("words a delete that got no answer as the server out of reach, never the platform's error", async () => {
    const { onDone, onError, user } = show((call) => {
      if (call.method === "DELETE") throw new TypeError("Failed to fetch");
      return null;
    });
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() => expect(onError).toHaveBeenCalledTimes(1));
    expect(onError).toHaveBeenCalledWith(NO_ANSWER);
    expect(onDone).not.toHaveBeenCalled();
    // Still asking: the retry is one click beside the sentence.
    expect(screen.getByRole("button", { name: "confirm abandon wire" })).toBeInTheDocument();
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
      serverSays("dir");
      render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);
      expect(screen.getByRole("button", { name: "abandon wire" })).toHaveTextContent("Abandon and save");
      cleanup();
      serverSays("off");
      render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);
      expect(screen.getByRole("button", { name: "abandon wire" })).toHaveTextContent(/^Abandon$/);
      cleanup();
      serverSays("dir");
      render(<Abandon table={wire.tableOpen} onDone={() => {}} onError={() => {}} />);
      expect(screen.getByRole("button", { name: `abandon ${wire.tableOpen.name ?? wire.tableOpen.id}` }))
        .toHaveTextContent(/^Abandon$/);
    } finally {
      resetFactsForTest();
    }
  });

  // **The THIRD state on the label** (M6.5b2, R-6.5b2.13). Nothing has ever
  // answered, so nothing can be promised — and the word this control chooses is
  // the one that promises less, not the one that promises a save it cannot
  // make.
  it("reads Abandon, not Abandon and save, when nothing has ever answered", () => {
    stubApi(() => null);
    render(<Abandon table={wire.table} onDone={() => {}} onError={() => {}} />);
    expect(screen.getByRole("button", { name: "abandon wire" })).toHaveTextContent(/^Abandon$/);
  });

  // A SERVER table's abandon asks the server nothing: the server archives it
  // inside its own delete, so there is no decision here to get wrong — and a
  // health read for one would be a request this control has never made.
  it("asks the server nothing about the archive for a table the server owns", async () => {
    const { stub, onDone, user } = show();
    await user.click(screen.getByRole("button", { name: "abandon wire" }));
    await user.click(screen.getByRole("button", { name: "confirm abandon wire" }));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(stub.matching("GET", "/api/health")).toEqual([]);
    expect(deletes(stub)).toEqual(["/api/games/wire"]);
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
    resetFactsForTest();
  });

  /** The control over a local table, with the two hosts scripted and every
   *  step recorded in the order it happened.
   *
   *  **The health read is the SERVER's own, at the click** (M6.5b2, spec
   *  §2.2): `archive` is what `GET /api/health` answers here, and it is also
   *  remembered so the LABEL — a render-time read — has something to say
   *  before the click. `null` is a server that never answers at all. */
  function local(
    archive: "off" | "dir" | null,
    onArchive: () => Reply | Promise<Reply>,
    opts: { remember?: boolean } = {},
  ) {
    const steps: string[] = [];
    if (archive !== null && opts.remember !== false) serverSays(archive);
    stubWorker({
      pair: () => ({ record: RECORD, table: TABLE }),
      call: (req) => {
        if (req.method === "DELETE") steps.push("delete");
        return undefined;
      },
    });
    const stub = stubApi(async (call) => {
      if (call.path.endsWith("/api/health")) {
        steps.push("health");
        if (archive === null) throw new TypeError("Failed to fetch");
        return { ...wire.health, archive };
      }
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
    expect(steps).toEqual(["health", "archive", "delete"]);
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
    expect(steps).toEqual(["health", "archive", "delete"]);
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
    expect(steps).toEqual(["health", "archive"]);
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
    expect(steps).toEqual(["health", "archive"]);
    expect(onDone).not.toHaveBeenCalled();
  });

  // **A REFUSING archive is not a reason to keep a game for ever** (M6.5c2
  // cycle D, straw poll row 100). The two cells above hold the first answer —
  // the game is kept, the server's sentence shown, the confirm face still up —
  // and that answer stands. But a refusal is the archive's settled word about
  // THIS record (a 409 names a different record under the id; a 400 is a
  // record it will not replay), so trying again meets it again, and the game's
  // own screen had no way to be rid of it. The third deliberate click the `ask`
  // face already offers is offered here too, and it sends nothing.
  it("offers Abandon without saving after a refused send, and deletes on it without sending again", async () => {
    const { steps, onDone, onError, user } = local("dir", () => ({
      status: 409,
      body: wire.archive409,
    }));
    await confirm(user);
    await waitFor(() => expect(onError).toHaveBeenCalledWith(wire.archive409.error));
    // The first answer, unchanged: kept, and still asking.
    expect(steps).toEqual(["health", "archive"]);
    expect(onDone).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: `confirm abandon ${LOCAL}` })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: `abandon ${LOCAL} without saving` }));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(steps).toEqual(["health", "archive", "delete"]);
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
    expect(steps).toEqual(["health", "archive"]);
    expect(onDone).not.toHaveBeenCalled();
    // **No third choice for a send that never got an answer**: trying again is
    // the remedy there, and the refusal cell above is the one that offers it.
    expect(screen.queryByRole("button", { name: `abandon ${LOCAL} without saving` })).toBeNull();
  });

  // **A 404 is the deployment's answer, not the moment's** (M6.5c2 spec §2.8;
  // the M6.5c Task 8 review's escalation 3). The route says this server keeps
  // no archive, and the outbox writes that down (`rememberArchiveOff`); this
  // control showed the sentence and left the fact at "dir" for up to an hour,
  // so its label went on promising a save the server cannot make. The game is
  // KEPT, as on every refusal: the next confirm asks the server again.
  it("remembers the archive OFF when the send meets the route's 404, and keeps the game", async () => {
    const { steps, onDone, onError, user } = local("dir", () => ({
      status: 404,
      body: { error: "this server keeps no archive" },
    }));
    await confirm(user);
    await waitFor(() => expect(onError).toHaveBeenCalledWith("this server keeps no archive"));
    expect(steps).toEqual(["health", "archive"]);
    expect(onDone).not.toHaveBeenCalled();
    expect(archiveAnswer()).toBe("off");
    // Nor after a 404: the next confirm deletes, because the archive is off.
    expect(screen.queryByRole("button", { name: `abandon ${LOCAL} without saving` })).toBeNull();
    // The label redraws from the fact: no "and save" over nowhere to save to.
    await user.click(screen.getByRole("button", { name: `cancel abandoning ${LOCAL}` }));
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(/^Abandon$/);
  });

  // **On the STATUS, never the sentence** (the whole-branch review's Minor 2):
  // the two cells below are the witnesses. A 404 whose sentence says nothing
  // about an archive still writes the fact down — the corpus's lobby-chat 404,
  // another route switched off, is that sentence — and a refusal that is not a
  // 404 leaves the fact alone however loudly its sentence names the archive:
  // the corpus's 409 reads "archived".
  it("remembers the archive OFF on a 404 whatever its sentence says", async () => {
    const { onDone, onError, user } = local("dir", () => ({
      status: 404,
      body: wire.houseChatOff404,
    }));
    await confirm(user);
    await waitFor(() => expect(onError).toHaveBeenCalledWith(wire.houseChatOff404.error));
    expect(wire.houseChatOff404.error, "the premise: a sentence that names no archive").not.toMatch(/archive/i);
    expect(archiveAnswer()).toBe("off");
    expect(onDone).not.toHaveBeenCalled();
  });

  it("leaves the archive fact alone on a refusal that is not a 404, whatever its sentence names", async () => {
    const { onDone, onError, user } = local("dir", () => ({
      status: 409,
      body: wire.archive409,
    }));
    await confirm(user);
    await waitFor(() => expect(onError).toHaveBeenCalledWith(wire.archive409.error));
    expect(wire.archive409.error, "the premise: a sentence that names the archive").toMatch(/archive/i);
    expect(archiveAnswer()).toBe("dir");
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
    expect(steps).toEqual(["health", "delete"]);
  });

  // **A local table with no `local` saves NOTHING, and says so** — the screen
  // for a save that no longer rebuilds is the one caller that withholds it,
  // because the archive would refuse that very record (spec §4: "Abandon still
  // works"). A label promising a copy nobody can make is what this rule exists
  // to stop.
  it("reads Abandon, and sends nothing, for a local table this control does not archive", async () => {
    serverSays("dir");
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
    serverSays("dir");
    stubWorker();
    stubApi(() => null);
    render(<Abandon table={localTable} local onDone={() => {}} onError={() => {}} />);
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(
      "Abandon and save",
    );
  });

  // ---- THREE states, and the middle one is the finding (R-6.5b2.13) --------
  //
  // **Spec §0.3 measured this at HEAD**: with no boot health read, a browser
  // that has never been past the door reads the archive's DEFAULT `"off"`,
  // which is indistinguishable from an answer — so a first-visit abandon of a
  // local game deleted it without ever trying to send it. The cells below are
  // the three answers, and the first of them is the one that must not regress.

  it("REFUSES when it cannot learn whether there is an archive, and offers two named choices", async () => {
    // Nothing remembered (the `beforeEach` reset) and nothing answering.
    const { steps, stub, onDone, onError, user } = local("dir", () => wire.archivePost, {
      remember: false,
    });
    // The premise: the label promises nothing, because nothing is known.
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(/^Abandon$/);
    stub.calls.length = 0;
    // The server stops answering between the render and the click.
    stubApi(() => {
      throw new TypeError("Failed to fetch");
    });
    await confirm(user);

    await screen.findByText(CANNOT_ASK);
    expect(screen.getByRole("button", { name: `back it up to a file: ${LOCAL}` })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: `abandon ${LOCAL} without saving` }),
    ).toBeInTheDocument();
    // **Nothing was deleted and nothing was sent.** A delete here is the one
    // thing in this area that must not happen.
    expect(steps).toEqual([]);
    expect(onDone).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
  });

  it("still deletes where the deployment is KNOWN to keep no archive", async () => {
    const { steps, onDone, user } = local("off", () => {
      throw new Error("the archive must not be asked");
    });
    await confirm(user);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    // Correct, and not the bug: the server SAID it keeps none.
    expect(steps).toEqual(["health", "delete"]);
  });

  // **The regression cell for review I-1.** Nothing is remembered, so a control
  // reading the module's `"off"` default would delete without sending — and the
  // server, asked, says it keeps an archive.
  it("sends first on a FIRST visit, where nothing was remembered and the server keeps one", async () => {
    const { steps, onDone, user } = local("dir", () => wire.archivePostAbandoned, {
      remember: false,
    });
    await confirm(user);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(steps).toEqual(["health", "archive", "delete"]);
  });

  // **The read is FORCED, however fresh the remembered answer is** (spec §2.2).
  // A deployment that turned its archive on since this browser last asked would
  // otherwise have it deleting finished games for the rest of the hour.
  it("asks again at the click even when the remembered answer is minutes old", async () => {
    // What this browser was told LAST TIME, before the control is drawn.
    serverSays("off");
    const { steps, onDone, user } = local("dir", () => wire.archivePostAbandoned, {
      remember: false,
    });
    // The premise: what is remembered says there is nowhere to send it.
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toHaveTextContent(/^Abandon$/);
    await confirm(user);
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    expect(steps).toEqual(["health", "archive", "delete"]);
  });

  it("writes the game to a file on the first choice, and deletes nothing", async () => {
    const caught = catchDownload();
    try {
      const { steps, onDone, user } = local("dir", () => wire.archivePost, { remember: false });
      stubApi(() => {
        throw new TypeError("Failed to fetch");
      });
      await confirm(user);
      await screen.findByText(CANNOT_ASK);

      await user.click(screen.getByRole("button", { name: `back it up to a file: ${LOCAL}` }));
      await waitFor(() => expect(caught.clicks).toHaveLength(1));
      // **The ENVELOPE's name, not the record's** (whole-branch review, false
      // sentence 10): `<id>.json` is what "Save game file" writes and what the
      // terminal loads, and this control wrote its envelope under that very
      // name — one file a player could not tell from the other, and one the
      // terminal would refuse.
      expect(caught.clicks[0]!.download).toBe(`${LOCAL}.backup.json`);
      // The envelope carries this device's own record, seed and all — the file
      // this app restores, which is what makes the choice a real alternative
      // to throwing the game away.
      const file = await readBlob(caught.create.mock.calls[0]?.[0] as Blob);
      expect(file).toContain('"kind": "yokohama-backup"');
      expect(file).toContain("18446744073709551615");
      // And the game is still here.
      expect(steps).toEqual([]);
      expect(onDone).not.toHaveBeenCalled();
      // The two choices are still on screen: backing up is not abandoning.
      expect(
        screen.getByRole("button", { name: `abandon ${LOCAL} without saving` }),
      ).toBeInTheDocument();
    } finally {
      caught.restore();
    }
  });

  it("deletes on the third deliberate click when the player abandons without saving", async () => {
    const { steps, onDone, user } = local("dir", () => wire.archivePost, { remember: false });
    stubApi(() => {
      throw new TypeError("Failed to fetch");
    });
    await confirm(user);
    await screen.findByText(CANNOT_ASK);
    await user.click(screen.getByRole("button", { name: `abandon ${LOCAL} without saving` }));
    await waitFor(() => expect(onDone).toHaveBeenCalledTimes(1));
    // The delete, and NOTHING sent: the player said so.
    expect(steps).toEqual(["delete"]);
  });

  it("puts the control back when the refusal is cancelled", async () => {
    const { steps, onDone, user } = local("dir", () => wire.archivePost, { remember: false });
    stubApi(() => {
      throw new TypeError("Failed to fetch");
    });
    await confirm(user);
    await screen.findByText(CANNOT_ASK);
    await user.click(screen.getByRole("button", { name: `cancel abandoning ${LOCAL}` }));
    expect(screen.getByRole("button", { name: `abandon ${LOCAL}` })).toBeInTheDocument();
    expect(screen.queryByText(CANNOT_ASK)).toBeNull();
    expect(steps).toEqual([]);
    expect(onDone).not.toHaveBeenCalled();
  });
});

/** The download's globals, replaced for one cell and put back — `App.test`'s
 *  own helper, which this file needs for the backup choice. */
function catchDownload() {
  const clicks: { href: string; download: string }[] = [];
  const click = vi
    .spyOn(HTMLAnchorElement.prototype, "click")
    .mockImplementation(function (this: HTMLAnchorElement) {
      clicks.push({ href: this.href, download: this.download });
    });
  const create = vi.fn().mockReturnValue("blob:envelope");
  Object.defineProperty(URL, "createObjectURL", { value: create, configurable: true });
  Object.defineProperty(URL, "revokeObjectURL", { value: vi.fn(), configurable: true });
  return { clicks, create, restore: () => click.mockRestore() };
}

/** jsdom's `Blob` has `text()`, and this is what reads back what was written
 *  into the download. */
const readBlob = (b: Blob): Promise<string> => b.text();
