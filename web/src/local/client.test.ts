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
  localCall,
  localHostStarted,
  localPair,
  onLocalFault,
  onLocalWrite,
  onLost,
  release,
  resetLocalHostForTests,
  RESTART_SENTENCE,
  takeover,
} from "./client";
import { TAKEN_OVER_SENTENCE } from "./lost";
import { currentWorker, FakeWorker, stubWorker } from "../test/fakeWorker";

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
      pair: (id) => (id === "local-1" ? { record: '{"seed":1}', table: '{"schema":3}' } : {}),
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
    // A module that fails to load and an IndexedDB that will not open both
    // trap on every message — including the `hold` the restart itself sends —
    // so an unbounded "restart on a trap" is a loop that never draws a
    // sentence. One restart is recovery; the second trap with nothing
    // answered in between is the host, and it is named.
    stubWorker({
      call: () => ({
        status: 500,
        body: { error: "the local host stopped: Error: IndexedDB would not open" },
        trapped: true,
      }),
    });
    const fault = vi.fn();
    onLocalFault(fault);
    await expect(localCall("GET", "/api/games/local-1")).rejects.toThrow(RESTART_SENTENCE);
    await tick();
    await expect(localCall("GET", "/api/games/local-1")).rejects.toThrow(/IndexedDB would not open/);
    await tick();
    // Two Workers and no third: the loop stopped, and the reason is the
    // module's own sentence rather than this client's.
    expect(FakeWorker.instances).toHaveLength(2);
    expect(fault.mock.calls.map((c) => c[0])).toEqual([
      RESTART_SENTENCE,
      "the local host could not be started: the local host stopped: Error: IndexedDB would not open",
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
