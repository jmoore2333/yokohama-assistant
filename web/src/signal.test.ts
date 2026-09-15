// The polled change signal: when it asks, when it does NOT, and what it says
// while it cannot reach the server.
//
// **Three properties, and each of them is a thing a plain `setInterval` gets
// wrong.** An interval asks from a tab nobody is looking at, which is a request
// every three seconds per backgrounded game for as long as the browser lives
// (R-P8). An interval asks again three seconds after a failure, and again, and
// again, at exactly the moment the server is least able to answer. And an
// interval says nothing on screen either way, so a player whose connection has
// gone sees a board that is simply never updated and no reason for it.
//
// The clock here is fake and the visibility is set by hand, because both are
// the thing under test: a cell that waited on real time would take nine
// seconds to prove one of these and could not prove the backoff at all.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSignal, usePolled, nextDelay, BACKOFF_MS, VISIBLE_MS, type End } from "./signal";
import { api } from "./api";
import { gate, stubApi, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import type { Events } from "./types";

/** Put the tab in a state and tell the document about it.
 *
 *  jsdom has no page lifecycle, so `visibilityState` is a plain read-only
 *  getter that never changes and no event is ever dispatched. Redefining the
 *  property and firing the event by hand is what a browser does between them;
 *  the hook reads exactly those two things and nothing else, which is why it
 *  can be driven this way at all. */
function visibility(state: "visible" | "hidden") {
  Object.defineProperty(document, "visibilityState", { value: state, configurable: true });
  document.dispatchEvent(new Event("visibilitychange"));
}

describe("the poll", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // jsdom's own default is already "visible", but the cell below leaves the
    // document HIDDEN when it ends — a property redefinition is not a global
    // stub and `unstubGlobals` does not undo it. Set here so the cells do not
    // depend on their own order.
    visibility("visible");
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("asks every three seconds while visible and active, and not at all when inactive", async () => {
    const stub = stubApi((c) => (c.path.endsWith("/events") ? wire.events : undefined));
    const { rerender } = renderHook(({ active }) => useSignal("wire", active, () => {}), {
      initialProps: { active: true },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS * 3 + 10);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(3);
    // **`active` false is the whole of "it is your turn".** The one request
    // this poll exists to make is the one that tells you somebody ELSE moved,
    // so a screen holding the prompt has nothing to ask about — and a timer
    // left running under it would re-fetch the game under the player's hand.
    rerender({ active: false });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS * 3);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(3);
  });

  it("asks nothing from a hidden tab, then once on return with a refreshing state (R-P8)", async () => {
    const stub = stubApi((c) => (c.path.endsWith("/events") ? wire.events : undefined));
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    visibility("hidden");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS * 5);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(0);
    // **Back, and the state is said BEFORE the answer arrives.** The board on
    // screen is as old as the tab was hidden, so "live" would be a claim this
    // client cannot make yet; "refreshing" is the true one, and it is true for
    // exactly one round trip.
    act(() => visibility("visible"));
    expect(result.current).toBe("refreshing");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(10);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    expect(result.current).toBe("live");
  });

  it("backs off on failure and says reconnecting, then recovers", async () => {
    let fail = true;
    const stub = stubApi((c) =>
      c.path.endsWith("/events") ? (fail ? { status: 502, body: { error: "gone" } } : wire.events) : undefined,
    );
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10);
    });
    expect(result.current).toBe("reconnecting");
    // The second ask is the FIRST backoff step and not another three seconds:
    // a server that just refused is the one thing a client should ask less
    // often, not the same amount.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 10);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(2);
    fail = false;
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[1]! + 10);
    });
    expect(result.current).toBe("live");
    expect(nextDelay(0)).toBe(BACKOFF_MS[0]);
    expect(nextDelay(99)).toBe(BACKOFF_MS[BACKOFF_MS.length - 1]);
  });

  // **A word about a poll that has stopped is a word about nothing.** Nothing
  // clears it once the effect is torn down, so the last thing said stands: the
  // reachable case is switching games from the menu while the server is
  // unreachable, where the game just opened would draw a banner reading
  // "reconnecting…" about a request no timer is going to make.
  it("stops claiming anything once it is no longer polling", async () => {
    const stub = stubApi((c) =>
      c.path.endsWith("/events") ? { status: 502, body: { error: "gone" } } : undefined,
    );
    const { result, rerender } = renderHook(({ active }) => useSignal("wire", active, () => {}), {
      initialProps: { active: true },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10);
    });
    expect(result.current).toBe("reconnecting");
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    rerender({ active: false });
    expect(result.current).toBe("live");
  });

  // ---- the reply that lands after the teardown ------------------------------
  //
  // **The two `if (stopped) return` lines are what these two cells are about,
  // and nothing above reaches them**: every cell here lets the request finish
  // before it tears anything down. The torn-down case is not exotic — `App`'s
  // `active` includes `!busy`, so the effect is torn down and rebuilt on EVERY
  // opponent move, and a reply in flight across that boundary is the ordinary
  // case rather than the edge one.
  //
  // **What a missing guard costs, at its real size.** `ask` ends by calling
  // `schedule`, and the timer it sets belongs to a closure whose cleanup has
  // already run, so nothing will ever clear it — but the successor is not
  // immortal either: `ask` itself opens with `if (stopped || !visible()) return`,
  // so the orphan fires once, returns, and arms nothing. One orphan timer and
  // one dead callback per teardown, on a page that tears down once a move.
  //
  // The two costs the cells below actually assert are the ones that are
  // observable: the CONSUMER is never handed an answer about a screen that has
  // stopped watching (which would re-read a game nobody asked about), and no
  // further request is made — the request count stays at one through two full
  // intervals, and through four backoff waits on the refused path.
  //
  // Both exits are covered because they are two different lines: the resolved
  // path at the top of `try`, and the refused path at the top of `catch`.

  it("drops a reply that lands after it was torn down, and arms nothing from it", async () => {
    const held = gate<Reply>();
    const seen: Events[] = [];
    const stub = stubApi((c) => (c.path.endsWith("/events") ? held.promise : undefined));
    const { rerender } = renderHook(
      ({ active }) => useSignal("wire", active, (e) => seen.push(e)),
      { initialProps: { active: true } },
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    // Torn down with the request still in flight, and only THEN answered.
    rerender({ active: false });
    await act(async () => {
      held.resolve(wire.events);
      await vi.advanceTimersByTimeAsync(0);
    });
    // The consumer is never told: this answer is about a screen that has
    // stopped watching, and handing it over would re-read a game nobody asked
    // about.
    expect(seen).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS * 2 + 20);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
  });

  it("drops a REFUSAL that lands after it was torn down, and arms nothing from it", async () => {
    const held = gate<Reply>();
    const stub = stubApi((c) => (c.path.endsWith("/events") ? held.promise : undefined));
    const { result, rerender } = renderHook(({ active }) => useSignal("wire", active, () => {}), {
      initialProps: { active: true },
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    rerender({ active: false });
    await act(async () => {
      held.resolve({ status: 502, body: { error: "gone" } });
      await vi.advanceTimersByTimeAsync(0);
    });
    // Nothing is said about a link that is no longer being kept, and the
    // backoff arms no timer.
    expect(result.current).toBe("live");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
  });
  // ---- the generalized hook (spec §5.5) -------------------------------------
  //
  // **`useSignal` is now ONE INSTANCE of `usePolled`**, and every cell above
  // runs through it unchanged — which is what says the generalization did not
  // move the contract. The two below are about what only the generic hook can
  // be asked, because `useSignal` has no second consumer to vary and no
  // refusal it acts on.

  it("does not re-arm the timer when a consumer re-renders with a new closure", async () => {
    // **The hazard the refs exist for, as an executable cell.** Every consumer
    // of this hook closes over state, so `once` is a NEW function on every
    // render; an effect that depended on it would tear the timer down and arm a
    // fresh one each time, and a poll re-armed faster than its own interval
    // never reaches its first tick at all. Measured here as ONE request across
    // five re-renders inside one interval — and the value it saw is the LAST
    // closure's, so the ref is read at ask time rather than captured at arm
    // time.
    const seen: number[] = [];
    const asked: number[] = [];
    const { rerender } = renderHook(
      ({ n }) =>
        usePolled<number>(
          {
            key: "lobby",
            once: () => {
              asked.push(n);
              return Promise.resolve(n);
            },
            stream: "/api/stream",
          },
          true,
          (v) => seen.push(v),
        ),
      { initialProps: { n: 0 } },
    );
    // **The re-renders happen PART WAY through the interval**, which is what
    // makes this a witness rather than a tautology: five re-renders at t=0 all
    // arm a timer that fires at the same moment whether or not it was re-armed,
    // so the cell would pass with the hazard present. Mutation-proven by adding
    // `source?.once` to the effect's dependency list — the ask below then never
    // happens, because each re-arm pushes the first tick a further interval out.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS - 500);
    });
    expect(asked).toEqual([]);
    for (let n = 1; n <= 5; n += 1) rerender({ n });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(600);
    });
    // ONE ask, and it saw the LAST closure's value — so the ref is read at ask
    // time rather than captured when the timer was armed.
    expect(asked).toEqual([5]);
    expect(seen).toEqual([5]);
  });

  it("ends the poll on a 404 and hands the sentence to onEnd", async () => {
    // **A refusal this poll can act on, which the backoff cannot.** A game that
    // has been deleted answers 404 for as long as the tab is open, so retrying
    // it forever is a client asking a question that has a permanent answer —
    // and the screen would sit reading "reconnecting…" about a game that is
    // gone. `onEnd` is how the consumer says so and leaves.
    const ends: End[] = [];
    const stub = stubApi((c) =>
      c.path.endsWith("/events")
        ? { status: 404, body: { error: 'no game named "wire"' } }
        : undefined,
    );
    renderHook(() =>
      usePolled<Events>(
        { key: "wire", once: () => api.events("wire"), stream: "/api/games/wire/stream" },
        true,
        () => {},
        (e) => ends.push(e),
      ),
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS + 10);
    });
    expect(ends).toEqual([{ reason: "gone", sentence: 'no game named "wire"' }]);
    // ENDED, not backed off: nothing is asked again, through three more
    // intervals and four backoff waits.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(VISIBLE_MS * 3 + BACKOFF_MS[0]! * 4);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
  });
});
