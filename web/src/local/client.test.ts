// The page's side of the local host: the request/reply wire, the lock's three
// answers, and the two failures that must never leave a pending list.
//
// **The Worker is faked and the assertions are about the CLIENT.** What the
// module does with a call is the corpus replay's claim (Task 7) and the
// Worker's own; what is tested here is what this page does with an answer —
// which call is in flight, what a trap costs, and who is told.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  hold,
  localAdopt,
  localCall,
  localHostStarted,
  localPair,
  localRestoreRead,
  LocalStoreUnreadable,
  onLocalFault,
  onLocalWrite,
  onLost,
  onStoreUnreadable,
  pendingCallsForTests,
  release,
  resetLocalHostForTests,
  RESTART_SENTENCE,
  takeover,
  TAKEOVER_REFUSED_SENTENCE,
} from "./client";
import { TAKEN_OVER_SENTENCE } from "./lost";
import { currentWorker, FakeWorker, refuseToStart, stubWorker } from "../test/fakeWorker";

/** Let the microtask chain settle. The fake answers on a microtask, as a real
 *  Worker answers on a task, and nothing here moves the clock. */
const tick = async (rounds = 6): Promise<void> => {
  for (let i = 0; i < rounds; i += 1) await Promise.resolve();
};

beforeEach(() => {
  resetLocalHostForTests();
  stubWorker();
});
afterEach(() => {
  resetLocalHostForTests();
});

describe("the local host's wire", () => {
  it("starts NO Worker until something asks for one", async () => {
    // The module is 2.8 MB. A visitor who only plays hosted games never
    // downloads it, which is why `start()` is lazy and this is asserted
    // rather than assumed.
    expect(localHostStarted()).toBe(false);
    expect(FakeWorker.instances).toHaveLength(0);
    await localCall("GET", "/api/games/local-1");
    expect(localHostStarted()).toBe(true);
    expect(FakeWorker.instances).toHaveLength(1);
  });

  it("takes the path and its query APART, because `ApiRequest` does", async () => {
    await localCall("GET", "/api/games/local-1/at?from=2&to=5");
    const sent = currentWorker().posted[0]!;
    expect(sent).toMatchObject({
      kind: "call",
      method: "GET",
      path: "/api/games/local-1/at",
      query: "from=2&to=5",
      body: null,
    });
  });

  it("sends a body as a STRING and reports the module's status and bytes", async () => {
    stubWorker({
      call: (req) =>
        req.path.endsWith("/input") ? { status: 409, body: { error: "not your turn" } } : undefined,
    });
    const out = await localCall("POST", "/api/games/local-1/input", { line: "3" });
    expect(currentWorker().posted[0]).toMatchObject({ body: '{"line":"3"}' });
    // The client does not refuse here — `api.ts`'s `settle` is what turns a
    // status into an `ApiError`, once, for both hosts.
    expect(out.status).toBe(409);
    expect(JSON.parse(out.text)).toEqual({ error: "not your turn" });
  });

  it("reads BOTH stored documents in ONE message, and answers null for what is not there", async () => {
    // Review finding #6 / spec §2.2: "the record and table are read for export
    // as one snapshot inside the queue". Two messages would be two queued
    // operations that a move can fall between, and "Send to the archive" sends
    // the pair to a server that checks they agree.
    stubWorker({
      pair: (id) =>
        id === "local-1"
          ? { record: '{"seed":1}', table: '{"schema":3}' }
          : { record: null, table: null },
    });
    expect(await localPair("local-1")).toEqual({ record: '{"seed":1}', table: '{"schema":3}' });
    expect(currentWorker().posted).toEqual([{ seq: 1, kind: "pair", id: "local-1" }]);
    // A game still dealing has a table and no record; one that is not there
    // has neither.
    expect(await localPair("local-2")).toEqual({ record: null, table: null });
  });

  it("fires onLocalWrite for every non-GET and for no GET", async () => {
    const wrote = vi.fn();
    const off = onLocalWrite(wrote);
    await localCall("GET", "/api/games/local-1");
    expect(wrote).not.toHaveBeenCalled();
    await localCall("POST", "/api/games/local-1/input", { line: "1" });
    await localCall("DELETE", "/api/games/local-1");
    expect(wrote).toHaveBeenCalledTimes(2);
    off();
    await localCall("POST", "/api/games/local-1/input", { line: "1" });
    expect(wrote).toHaveBeenCalledTimes(2);
  });

  it("reports a write the store REFUSED as unsaved, carrying its reason", async () => {
    stubWorker({
      call: () => ({
        status: 200,
        body: { ...{}, ok: true },
        saved: "unsaved",
        reason: "the quota is full",
      }),
    });
    const out = await localCall("POST", "/api/games/local-1/input", { line: "1" });
    expect(out.saved).toBe(false);
    expect(out.reason).toBe("the quota is full");
  });

  it("counts a READ on a game this tab does not hold as saved, and a WRITE as NOT saved", async () => {
    // Review finding #4. `"saved"` means the transaction LANDED (spec §2.2).
    // A GET on a game this tab does not hold wrote nothing and was never meant
    // to — a second tab's cold read is answered from its own store — so it is
    // saved. A MOVE on one is applied in the module and never written, and
    // reporting that as kept is the dangerous direction.
    stubWorker({ call: () => ({ status: 200, body: { ok: true }, saved: "not-held" }) });
    expect((await localCall("GET", "/api/games/local-1")).saved).toBe(true);
    const write = await localCall("POST", "/api/games/local-1/input", { line: "1" });
    expect(write.saved).toBe(false);
    // And it is not merely unsaved: see the cell below for what it says.
    expect(write.status).toBe(409);
  });

  it("answers a WRITE the Worker did not keep as the LOCK LOSS, not as an unsaved move", async () => {
    // **The in-flight rule's other path** (spec §2.2, whole-branch review
    // Important #1). §2.2 says a reply from a call that was in flight when the
    // lock was lost "is answered to the page as the lock loss, so nothing is
    // shown that will not be kept". The Worker keeps that on the GENERATION
    // path — a write whose transaction reads a newer owner becomes a 409. It
    // could not keep it on the other one: when the steal's abort has already
    // removed the hold, the queued write is answered `not-held`, which used to
    // reach the page as the module's own 200 with `saved: false`. The page
    // then DREW the move — applied in a store about to be discarded — under
    // "APPLIED BUT NOT SAVED — the browser refused the write", a sentence
    // about the store. Nothing about the store refused it; the lock did.
    //
    // So a non-GET answered `not-held` is a refusal here, in the same words
    // the generation path uses, and `settle` raises it as the refusal `send`
    // already shows.
    let holds = true;
    stubWorker({
      call: () => (holds ? undefined : { status: 200, body: { ok: true }, saved: "not-held" }),
    });
    const lost = vi.fn();
    onLost(lost);
    expect(await hold("local-1")).toBe(true);

    holds = false;
    const out = await localCall("POST", "/api/games/local-1/input", { line: "1" });
    expect(out.status).toBe(409);
    expect(out.saved).toBe(false);
    expect((JSON.parse(out.text) as { error: string }).error).toBe(TAKEN_OVER_SENTENCE);
    // The screen turns read-only ONCE, and by the same statement every other
    // path to the loss goes through.
    expect(lost).toHaveBeenCalledExactlyOnceWith("local-1");
  });

  it("reads `not-held` on a game this page BELIEVED it held as the lock loss", async () => {
    // Review finding #4's other half. The Worker posts its own unsolicited
    // `lost` on the paths it notices; this is the arm for a reply that got
    // here first, or a hold the Worker no longer has. One statement of it, so
    // the screen turns read-only once.
    let holds = true;
    stubWorker({
      call: () => (holds ? undefined : { status: 200, body: { ok: true }, saved: "not-held" }),
    });
    const lost = vi.fn();
    onLost(lost);
    expect(await hold("local-1")).toBe(true);
    await localCall("GET", "/api/games/local-1");
    expect(lost).not.toHaveBeenCalled();

    holds = false;
    await localCall("GET", "/api/games/local-1");
    expect(lost).toHaveBeenCalledExactlyOnceWith("local-1");
    // Told once: the page no longer believes it holds the game, so a second
    // `not-held` says nothing more — and a restart would not re-acquire it.
    await localCall("GET", "/api/games/local-1");
    expect(lost).toHaveBeenCalledTimes(1);
  });

  it("says nothing about a game the page never held", async () => {
    stubWorker({ call: () => ({ status: 200, body: { ok: true }, saved: "not-held" }) });
    const lost = vi.fn();
    onLost(lost);
    await localCall("GET", "/api/games/local-9");
    expect(lost).not.toHaveBeenCalled();
  });
});

describe("the lock, from the page", () => {
  it("hands back what the Worker says, and remembers only what it got", async () => {
    stubWorker({ hold: (id) => id === "local-1" });
    expect(await hold("local-1")).toBe(true);
    expect(await hold("local-2")).toBe(false);
    expect(currentWorker().posted.map((p) => p.kind)).toEqual(["hold", "hold"]);
  });

  it("takes over by asking for the steal, and releases by name", async () => {
    await takeover("local-1");
    await release("local-1");
    expect(currentWorker().posted).toEqual([
      { seq: 1, kind: "takeover", id: "local-1" },
      { seq: 2, kind: "release", id: "local-1" },
    ]);
  });

  // **A steal the Worker did not grant is not a hold** (M6.5c2 cycle D,
  // dossier row 56). The acquisition answers `held: false` when another tab's
  // steal of the same game aborts this one before it lands; a client that
  // recorded the game anyway told the screen it played here, and its next
  // restart re-acquired a lock the other tab had just taken. The last half is
  // what a restart with nothing held looks like: no Worker of its own.
  it("refuses a takeover the Worker did not grant, and does not believe it holds the game", async () => {
    stubWorker({
      hold: (_id, steal) => !steal,
      call: (req) =>
        req.path.endsWith("/ports")
          ? { status: 500, body: { error: "unreachable executed" }, trapped: true }
          : undefined,
    });
    const refused = await takeover("local-1").then(
      () => null,
      (e: unknown) => e,
    );
    expect(refused, "the steal was answered not held, and the page said so").toBeInstanceOf(Error);
    expect((refused as Error).message).toBe(TAKEOVER_REFUSED_SENTENCE);

    await expect(localCall("GET", "/api/games/local-1/ports")).rejects.toThrow(RESTART_SENTENCE);
    await tick();
    expect(FakeWorker.instances, "the restart had nothing to re-hold").toHaveLength(1);
  });

  it("hands an unsolicited `lost` to every listener, and stops re-holding it", async () => {
    const lost = vi.fn();
    onLost(lost);
    await hold("local-1");
    await hold("local-2");
    currentWorker().emit({ kind: "lost", id: "local-1" });
    expect(lost).toHaveBeenCalledWith("local-1");

    // And the page no longer believes it holds it: a restart re-acquires
    // local-2 alone.
    currentWorker().onmessage!({
      data: { seq: 99, status: 500, body: "{}", trapped: true },
    } as MessageEvent<unknown>);
    await tick();
    expect(
      currentWorker()
        .posted.filter((p) => p.kind === "hold")
        .map((p) => (p as { id: string }).id),
    ).toEqual(["local-2"]);
  });
});

describe("the two failures that must never leave a pending list", () => {
  it("restarts on a trap: terminate, settle EVERY call, start again, re-hold", async () => {
    // R5: a trapped module holds its own mutex and traps again on `init()`, so
    // recovery is a fresh instance. The cell that would fail against a client
    // that answered the trapped call and left the others hanging is the second
    // rejection below — the call that was merely IN FLIGHT.
    stubWorker({
      call: (req) => {
        if (req.path.endsWith("/log")) return "pending";
        if (req.path.endsWith("/ports"))
          return { status: 500, body: { error: "unreachable executed" }, trapped: true };
        return undefined;
      },
    });
    const fault = vi.fn();
    onLocalFault(fault);
    expect(await hold("local-1")).toBe(true);
    const first = currentWorker();

    const inFlight = localCall("GET", "/api/games/local-1/log");
    const trapping = localCall("GET", "/api/games/local-1/ports");

    await expect(trapping).rejects.toThrow(RESTART_SENTENCE);
    await expect(inFlight).rejects.toThrow(RESTART_SENTENCE);
    expect(fault).toHaveBeenCalledWith(RESTART_SENTENCE);
    expect(first.terminated).toBe(true);
    expect(FakeWorker.instances).toHaveLength(2);
    await tick();
    const second = currentWorker();
    expect(second).not.toBe(first);
    expect(second.posted).toEqual([{ seq: 4, kind: "hold", id: "local-1" }]);

    // The fresh Worker answers, and the page is playing again.
    expect((await localCall("GET", "/api/games/local-1")).status).toBe(200);
  });

  it("posts NOTHING after a restart until the re-hold has been answered", async () => {
    // **The order spec §4 states**: the page starts a new Worker, "re-acquires
    // the game's ownership, and only then says 'reloaded from the last saved
    // move'". A call that overtakes the acquisition is answered `not-held` by
    // a Worker that is about to own the game, and the page reads that as the
    // lock being LOST — so the screen tells the player their game was opened
    // in another tab, about its own restart.
    //
    // Measured in a real browser before it was fixed (`plays_locally.spec.ts`,
    // the trap cell): three reads came back `not-held` while
    // `navigator.locks.query()` showed the game held by that very Worker.
    // Settling the pending calls is what makes the page ask again, and the
    // first draft gated on the re-hold's own promise — which [`fail`] had just
    // rejected, so the gate was open before it was closed.
    let asked = 0;
    stubWorker({
      hold: () => (asked++ === 0 ? true : "pending"),
      call: (req) =>
        req.path.endsWith("/ports")
          ? { status: 500, body: { error: "unreachable executed" }, trapped: true }
          : undefined,
    });
    expect(await hold("local-1")).toBe(true);

    const refusal = await localCall("GET", "/api/games/local-1/ports").then(
      () => null,
      (e: Error) => e,
    );
    expect(refusal?.message).toContain(RESTART_SENTENCE);

    // `App`'s own shape: the refusal handler re-reads the game. It reaches the
    // fresh Worker, which has been ASKED for the game and does not hold it
    // yet — so nothing is sent.
    const reread = localCall("GET", "/api/games/local-1");
    await tick();
    const second = currentWorker();
    expect(second.posted).toHaveLength(1);
    expect(second.posted[0]).toMatchObject({ kind: "hold", id: "local-1" });

    // The acquisition answers, and only then does the waiting call go out.
    second.emit({ seq: (second.posted[0] as unknown as { seq: number }).seq, held: true });
    await tick();
    expect(second.posted.map((p) => p.kind)).toEqual(["hold", "call"]);
    expect((await reread).status).toBe(200);
  });

  it("reports a host that CANNOT boot after one restart, instead of restarting for ever", async () => {
    // A module that fails to load traps on every message — including the
    // `hold` the restart itself sends — so an unbounded "restart on a trap" is
    // a loop that never draws a sentence. One restart is recovery; the second
    // trap with nothing answered in between is the host, and it is named. (An
    // IndexedDB that will not open is not this: since M6.5c2 cycle D it is
    // answered in the storage's own words, below, and restarts nothing.)
    stubWorker({
      call: () => ({
        status: 500,
        body: { error: "the local host stopped: CompileError: WebAssembly.instantiate(): expected magic word" },
        trapped: true,
      }),
    });
    const fault = vi.fn();
    onLocalFault(fault);
    await expect(localCall("GET", "/api/games/local-1")).rejects.toThrow(RESTART_SENTENCE);
    await tick();
    await expect(localCall("GET", "/api/games/local-1")).rejects.toThrow(/expected magic word/);
    await tick();
    // Two Workers and no third: the loop stopped, and the reason is the
    // module's own sentence rather than this client's.
    expect(FakeWorker.instances).toHaveLength(2);
    expect(fault.mock.calls.map((c) => c[0])).toEqual([
      RESTART_SENTENCE,
      "the local host could not be started: the local host stopped: CompileError: WebAssembly.instantiate(): expected magic word",
    ]);
  });

  it("gives the budget back on an ordinary answer, so a later trap still recovers", async () => {
    let trap = true;
    stubWorker({
      call: () => (trap ? { status: 500, body: { error: "boom" }, trapped: true } : undefined),
    });
    await expect(localCall("GET", "/api/games/local-1")).rejects.toThrow(RESTART_SENTENCE);
    trap = false;
    expect((await localCall("GET", "/api/games/local-1")).status).toBe(200);
    trap = true;
    // The RESTART sentence and not "could not be started": the budget came
    // back with the answer above.
    await expect(localCall("GET", "/api/games/local-1")).rejects.toThrow(RESTART_SENTENCE);
    trap = false;
    expect((await localCall("GET", "/api/games/local-1")).status).toBe(200);
    // Three hosts over the cell: the first trapped, the second answered and
    // then trapped, the third is playing. (A restart with nothing HELD starts
    // no Worker of its own — the next call does.)
    expect(FakeWorker.instances).toHaveLength(3);
  });

  // **The gate a restart raises must come down even when the fresh Worker
  // cannot be BUILT** (M6.5c2 cycle D, dossier row 56). The restart's re-holds
  // are what start the new Worker, and `new Worker(…)` can throw — a script
  // the page may not load. That throw used to escape `ask` before `Promise.all`
  // was reached, so the gate that opens on the re-holds' answers never opened,
  // and every later call waited on it for ever: the screen froze on a host
  // that had already failed. Now the refusal is a rejection like any other,
  // the gate opens, and the next call is told why in the start's own words.
  it("lowers the restart's gate when the fresh Worker cannot even be built, and says why", async () => {
    stubWorker({
      call: (req) =>
        req.path.endsWith("/ports")
          ? { status: 500, body: { error: "unreachable executed" }, trapped: true }
          : undefined,
    });
    const fault = vi.fn();
    onLocalFault(fault);
    // Held, so the restart has a game to re-acquire — and that is what starts
    // the Worker it cannot build.
    expect(await hold("local-1")).toBe(true);
    refuseToStart("the Worker's script was refused");

    await expect(localCall("GET", "/api/games/local-1/ports")).rejects.toThrow(RESTART_SENTENCE);
    await tick();
    const next = await Promise.race([
      localCall("GET", "/api/games/local-1").then(
        () => "answered",
        (e: Error) => e.message,
      ),
      tick(12).then(() => "still waiting on the gate"),
    ]);
    const sentence = "the local host failed to start: the Worker's script was refused";
    expect(next).toBe(sentence);
    expect(fault).toHaveBeenCalledWith(sentence);
  });

  // **A message that could not be posted leaves nothing waiting** (M6.5c2
  // cycle D, dossier row 56). `postMessage` can throw — a body the structured
  // clone refuses — and the call is rejected by the throw; the entry it had
  // already put in the pending list stayed there, waiting for a reply to a
  // message that was never sent. The fake throws for a route it was not told
  // about, from inside `postMessage`, which is the same place.
  it("leaves no call waiting when its message could not even be posted", async () => {
    await expect(localCall("GET", "/api/games/local-1/nowhere")).rejects.toThrow(
      "the fake local host was not told about GET /api/games/local-1/nowhere",
    );
    expect(pendingCallsForTests(), "a call that was never sent waits for nothing").toBe(0);
  });

  it("turns a Worker that will not start into a sentence, and THROWS THE DEAD ONE AWAY", async () => {
    // Review finding #3. Settling the pending list left the dead instance
    // installed, and `start()` handed it to the next `ask` — whose reply never
    // came. The second half of this cell is the one that fails against that:
    // a call made after the fault must reach a NEW Worker and answer.
    let alive = false;
    stubWorker({ call: () => (alive ? undefined : "pending") });
    const fault = vi.fn();
    onLocalFault(fault);
    const waiting = localCall("GET", "/api/games/local-1");
    const dead = currentWorker();
    dead.raise("Failed to fetch dynamically imported module");
    await expect(waiting).rejects.toThrow(/the local host failed to start/);
    expect(fault.mock.calls[0]![0]).toContain("Failed to fetch dynamically imported module");
    expect(dead.terminated).toBe(true);

    alive = true;
    expect((await localCall("GET", "/api/games/local-1")).status).toBe(200);
    expect(FakeWorker.instances).toHaveLength(2);
    expect(currentWorker()).not.toBe(dead);
  });

  it("does the same for a message this page cannot read", async () => {
    stubWorker({ call: () => "pending" });
    const waiting = localCall("GET", "/api/games/local-1");
    const dead = currentWorker();
    dead.onmessageerror!({ data: undefined } as MessageEvent<unknown>);
    await expect(waiting).rejects.toThrow(/could not read/);
    expect(dead.terminated).toBe(true);
  });

  // **And a store that cannot be READ is neither** (PR #37 fix wave, fix round
  // 1, review I-1; `worker.ts`'s rule 6). The export arm reads IndexedDB, and a
  // read it refuses is answered as an ordinary reply naming the storage. The
  // page must refuse the export with that sentence and leave the host ALONE:
  // a restart terminates the Worker, and the Worker is holding the one copy of
  // any move the store has refused to keep — exactly what the export is for.
  it("refuses an export whose storage could not be read, and does NOT restart the host", async () => {
    const sentence = "this browser's storage could not be read — Connection lost.";
    stubWorker({ pair: () => ({ unreadable: sentence }) });
    const fault = vi.fn();
    onLocalFault(fault);
    expect(await hold("local-1")).toBe(true);
    const host = currentWorker();

    await expect(localPair("local-1")).rejects.toThrow(sentence);
    expect(host.terminated, "the Worker holding the unsaved move is not thrown away").toBe(false);
    expect(FakeWorker.instances).toHaveLength(1);
    expect(fault, "and nothing reports the host as restarted").not.toHaveBeenCalled();
    // The host is still the one this page talks to, and still answers.
    expect((await localCall("GET", "/api/games/local-1")).status).toBe(200);
    expect(currentWorker()).toBe(host);
  });

  // **And the same for the three other places the store is read** (M6.5c2
  // cycle D, dossier row 14). A game's ACQUISITION reads and writes IndexedDB
  // — its generation, then the rebuild's read of every pair — and so do the
  // Worker's startup (`ready`) and a restore's reconciliation (`adopt`). Each
  // used to go the trap's way: the page threw its host away and said "the
  // local host restarted — reloaded from the last saved move" about a store
  // that had not been read, and where the store would not even open, the
  // fresh host trapped again and read "could not be started". None of them is
  // a module fault, and the export's rule is the right one for all four: an
  // ordinary reply in the storage's own words, and the host left alone.
  describe("a store that cannot be read, outside the export", () => {
    const sentence = "this browser's storage could not be read \u2014 The database connection is closing.";

    it("answers a hold as UNREADABLE, in its sentence, and neither restarts the host nor believes it holds the game", async () => {
      stubWorker({ hold: (id) => (id === "local-1" ? { unreadable: sentence } : true) });
      const fault = vi.fn();
      onLocalFault(fault);
      const refused = await hold("local-1").then(
        (v) => v,
        (e: unknown) => e,
      );
      expect(refused, "a typed refusal, so the screen can draw its face").toBeInstanceOf(LocalStoreUnreadable);
      expect((refused as Error).message).toBe(sentence);
      const host = currentWorker();
      expect(host.terminated, "the host is not thrown away").toBe(false);
      expect(fault, "and nothing reports it as restarted").not.toHaveBeenCalled();

      // The page does not believe it holds the game: a restart re-acquires the
      // one it DOES hold, and only that one.
      expect(await hold("local-2")).toBe(true);
      host.onmessage!({ data: { seq: 99, status: 500, body: "{}", trapped: true } } as MessageEvent<unknown>);
      await tick();
      expect(
        currentWorker()
          .posted.filter((p) => p.kind === "hold")
          .map((p) => (p as { id: string }).id),
      ).toEqual(["local-2"]);
    });

    it("answers a call as a 503 in the store's words, tells no listing it wrote, and restarts nothing", async () => {
      stubWorker({ call: () => ({ unreadable: sentence }) });
      const wrote = vi.fn();
      onLocalWrite(wrote);
      const read = await localCall("GET", "/api/games");
      expect(read.status).toBe(503);
      expect(JSON.parse(read.text)).toEqual({ error: sentence });
      // A write the host could not even start on wrote nothing.
      const write = await localCall("POST", "/api/games/local-1/input", { line: "1" });
      expect(write.status).toBe(503);
      expect(wrote, "nothing was written, so no listing is told to re-read").not.toHaveBeenCalled();
      expect(FakeWorker.instances).toHaveLength(1);
      expect(currentWorker().terminated).toBe(false);
    });

    // **The restore's first step reads the store's answer too** (fix round 1,
    // review Important 1). `restore_read` is queued like every other message,
    // so it waits on the Worker's startup; when the startup cannot read the
    // store the reply is `{ unreadable }`, and a reader that looked only for a
    // report or a refusal told the player "the local host could not read this
    // file" — blaming the FILE for the STORE.
    it("refuses a restore's reading in the STORE's words, never the file's, and restarts nothing", async () => {
      stubWorker({ restore: () => ({ unreadable: sentence }) });
      const refused = await localRestoreRead("{}").then(
        () => null,
        (e: unknown) => e,
      );
      expect(refused, "the storage's refusal, typed").toBeInstanceOf(LocalStoreUnreadable);
      expect((refused as Error).message).toBe(sentence);
      expect(FakeWorker.instances).toHaveLength(1);
      expect(currentWorker().terminated).toBe(false);
    });

    // **A restart's re-hold is read by the hold's own rule** (fix round 1,
    // review Minor 1). The restart re-acquires every game the page held, and
    // before this round it ignored each answer. A fresh Worker whose store
    // cannot be read answered `{ unreadable }`, an ordinary reply, so the id
    // stayed on the page's list; the next write came back `not-held`, and the
    // page said "This game was opened in another tab", which is false. Now the
    // storage's refusal is SAID for that game, which the screen draws as the
    // broken-store face, and the game is dropped from the list. A re-hold
    // answered not held is the loss, which is true: another tab has the game.
    it("reads a restart's re-holds by the hold's rule: the store's refusal is said, a refused hold is the loss", async () => {
      let fresh = false;
      stubWorker({
        hold: (id) => (!fresh ? true : id === "local-1" ? { unreadable: sentence } : false),
        call: (req) =>
          req.path.endsWith("/ports")
            ? { status: 500, body: { error: "unreachable executed" }, trapped: true }
            : undefined,
      });
      const store = vi.fn();
      onStoreUnreadable(store);
      const lost = vi.fn();
      onLost(lost);
      expect(await hold("local-1")).toBe(true);
      expect(await hold("local-2")).toBe(true);

      fresh = true;
      await expect(localCall("GET", "/api/games/local-1/ports")).rejects.toThrow(RESTART_SENTENCE);
      await tick();
      expect(store).toHaveBeenCalledExactlyOnceWith("local-1", sentence);
      expect(lost).toHaveBeenCalledExactlyOnceWith("local-2");

      // Neither is re-held by the next restart: nothing is left on the list,
      // and a restart with nothing held starts no Worker of its own.
      currentWorker().onmessage!({ data: { seq: 99, status: 500, body: "{}", trapped: true } } as MessageEvent<unknown>);
      await tick();
      expect(FakeWorker.instances).toHaveLength(2);
    });

    it("refuses a restore's adoption in its sentence, tells no listing it wrote, and restarts nothing", async () => {
      stubWorker({ adopt: () => ({ unreadable: sentence }) });
      const wrote = vi.fn();
      onLocalWrite(wrote);
      await expect(localAdopt(["local-1"])).rejects.toThrow(sentence);
      expect(wrote, "nothing was adopted").not.toHaveBeenCalled();
      expect(FakeWorker.instances).toHaveLength(1);
      expect(currentWorker().terminated).toBe(false);
    });
  });
});

// ---- a notice, which is not a fault (M6.5b2, spec §2.6) --------------------
//
// **The distinction is whether anything is settled.** A fault means this host
// cannot answer and everything waiting on it is rejected; a notice means it is
// still working on it and here is why it is slow. The storage version's blocked
// open is the first of the second kind, and a client that routed it through
// `fail` would have turned a wait that ends by itself — the other tab closes —
// into a screenful of refusals on a page that then had nothing left to retry.
describe("an unsolicited notice", () => {
  it("reaches the fault line without settling anything in flight", async () => {
    stubWorker({ call: (req) => (req.path.endsWith("/log") ? "pending" : undefined) });
    const fault = vi.fn();
    onLocalFault(fault);
    const inFlight = localCall("GET", "/api/games/local-1/log");
    let settled = false;
    void inFlight.then(
      () => (settled = true),
      () => (settled = true),
    );

    currentWorker().emit({ kind: "notice", sentence: "another tab is holding this open" });
    await tick();
    expect(fault).toHaveBeenCalledWith("another tab is holding this open");
    // The call is STILL waiting, which is the whole difference from a fault.
    expect(settled).toBe(false);
    // And the Worker was not thrown away: a notice is not a restart.
    expect(FakeWorker.instances).toHaveLength(1);
    expect(currentWorker().terminated).toBe(false);
  });
});
