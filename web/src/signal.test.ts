// The change signal: what it opens, when it does NOT, and what it says while it
// cannot reach the server.
//
// **The interval is gone and the cells that pinned it are gone with it**
// (M6.3 R-3.2). What replaced them is one connection per visible tab, and the
// property that says so is NEGATIVE: while the stream is connected there is no
// timer armed at all. That cell is the one that fails against the poll — every
// other rule here (nothing from a hidden tab, a server that cannot answer is
// asked less often, a refusal that ends rather than slows) survived the change
// of transport, which is what `Source`'s generalization was for.
//
// The clock is fake and the visibility is set by hand, because both are the
// thing under test. The STREAM is fake for the same reason: `fakeStream` hands
// the cell the writing end of the body, so a frame arrives when the cell says
// so rather than when a server would have sent one.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  useSignal,
  usePolled,
  nextDelay,
  endSentence,
  BACKOFF_MS,
  HELD_REASK_MS,
  type End,
  type Source,
} from "./signal";
import { ACCESS_CHALLENGE, api, setSignalKind } from "./api";
import { fakeStream, flush, gate, openStream, until, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import type { Events, TableView } from "./types";

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

/** Let the reply chain settle without moving the clock — the harness's own
 *  [`flush`]. An open, a header, a first chunk and a `setState` are a chain of
 *  turns and not all of them microtasks, and a cell that advanced TIME instead
 *  would be asserting about the ladder. */
const settle = flush;

/** An `Events` at a count, from the corpus's own payload. */
const at = (events: number): Events => ({ ...wire.events, events });

/** The usual arrangement: a stream at every `…/stream`, everything else
 *  declined (and therefore answered by the harness's own default). */
function streamed(onOpen?: (open: number) => Array<[string, unknown]>, extra?: (c: ApiCall) => Reply | undefined) {
  const stream = fakeStream(onOpen);
  const stub = stubApi((call) => {
    const mine = extra?.(call);
    if (mine !== undefined) return mine;
    return call.path.endsWith("/stream") ? stream.reply(call) : undefined;
  });
  return { stream, stub };
}

/** The game's signal, as the game and the dealing screens hold it: a `"value"`
 *  source, whose `once` is the HELD `GET …/events?wait=25`. */
const gameSource = (game: string): Source<Events> => ({
  key: game,
  once: () => api.events(game),
  stream: `/api/games/${game}/stream`,
  frames: "value",
});

/** The lobby's and the open table's shape: a `"wake"` source, whose `once` is
 *  an ordinary read that answers at once and has no waiting endpoint behind
 *  it. The distinction is what [`HELD_REASK_MS`] is about. */
const wakeSource = (game: string): Source<Events> => ({
  key: game,
  once: () => api.events(game),
  stream: `/api/games/${game}/stream`,
});

beforeEach(() => {
  vi.useFakeTimers();
  // jsdom's own default is already "visible", but a cell below leaves the
  // document HIDDEN when it ends — a property redefinition is not a global stub
  // and `unstubGlobals` does not undo it. Set here so the cells do not depend on
  // their own order.
  visibility("visible");
});
afterEach(() => {
  vi.useRealTimers();
  // The transport is a module-level value read from `/api/health` at boot, so a
  // cell that moved it would move it for every cell after it.
  setSignalKind("sse");
});

describe("the signal, over the stream", () => {
  it("hands over every frame, and arms NO timer while it is connected", async () => {
    // **The cell that fails against the poll (spec §5.1).** Everything else in
    // this file is a rule the poll also kept; this is the one that says the
    // interval left. A connected reader is a promise waiting on a body, and a
    // browser holding one has nothing scheduled — where the poll always had
    // exactly one timer pending, whatever it had just been told.
    const seen: Events[] = [];
    const { stream, stub } = streamed(() => [["signal", at(3)]]);
    renderHook(() => useSignal("wire", true, (e) => seen.push(e)));
    // **Connected before a frame is written**, gated on the fact rather than on
    // a number of turns: a frame written to a body nobody has opened reaches
    // nobody, and nothing re-delivers it.
    await until(() => stream.opens() > 0, "the stream was opened");
    await settle();
    expect(seen.map((e) => e.events)).toEqual([3]);
    expect(vi.getTimerCount()).toBe(0);

    stream.push("signal", at(4));
    stream.push("signal", at(5));
    await settle();
    expect(seen.map((e) => e.events)).toEqual([3, 4, 5]);
    // Still nothing armed, and a minute of the clock changes nothing — where a
    // three-second poll would have made twenty requests.
    expect(vi.getTimerCount()).toBe(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    // **And the events route is never asked at all** (R-3.2): the on-connect
    // frame IS the ask.
    expect(stub.matching("GET", "/events")).toHaveLength(0);
  });

  it("signs the open with the identity header", async () => {
    localStorage.setItem("yokohama.claimed_user", "red@wire.test");
    const { stub } = streamed();
    renderHook(() => useSignal("wire", true, () => {}));
    await settle();
    expect(stub.matching("GET", "/stream")[0]!.headers["x-yokohama-claimed-user"]).toBe(
      "red@wire.test",
    );
  });

  it("opens nothing from a hidden tab, aborts the reader on the way out, and re-opens on return", async () => {
    const seen: Events[] = [];
    const { stream, stub } = streamed((open) => [["signal", at(open)]]);
    const { result } = renderHook(() => useSignal("wire", true, (e) => seen.push(e)));
    await until(() => stream.opens() > 0, "the stream was opened");
    await settle();
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    expect(result.current).toBe("live");

    // Hidden: the READER is let go, which is the whole of R-P8 over a stream —
    // an open connection per backgrounded game is the cost an interval used to
    // pay in requests.
    act(() => visibility("hidden"));
    expect(stub.matching("GET", "/stream")[0]!.signal!.aborted).toBe(true);
    // And a frame written to a body nobody is reading reaches nobody.
    stream.push("signal", at(99));
    await settle();
    expect(seen.map((e) => e.events)).toEqual([1]);

    // **Back, and the state is said BEFORE the connection is made.** The board
    // on screen is as old as the tab was hidden, so "live" would be a claim this
    // client cannot make yet.
    act(() => visibility("visible"));
    expect(result.current).toBe("refreshing");
    await until(() => stream.opens() > 1, "the stream was opened again");
    await settle();
    expect(stub.matching("GET", "/stream")).toHaveLength(2);
    expect(result.current).toBe("live");
    // The reopened stream's ON-CONNECT frame is the compare: that is what the
    // return asks with, and there is no second request to make it.
    expect(seen.map((e) => e.events)).toEqual([1, 2]);
  });

  it("says reconnecting and climbs the ladder when the body ends without an end frame", async () => {
    const { stream, stub } = streamed((open) => [["signal", at(open)]]);
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    await until(() => stream.opens() > 0, "the stream was opened");
    await settle();
    expect(result.current).toBe("live");

    // A dropped connection. It is NOT an `end`: the server has not said this
    // stream is over, so the answer is to open another one — later.
    stream.close();
    await settle();
    expect(result.current).toBe("reconnecting");
    expect(stub.matching("GET", "/stream")).toHaveLength(1);

    // And not at once. The first step of the ladder, and the step before it
    // buys nothing — which is the assertion that says the wait is the ladder's
    // rather than a busy loop.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! - 100);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    await settle();
    expect(stub.matching("GET", "/stream")).toHaveLength(2);
    expect(result.current).toBe("live");
    expect(nextDelay(0)).toBe(BACKOFF_MS[0]);
    expect(nextDelay(99)).toBe(BACKOFF_MS[BACKOFF_MS.length - 1]);
  });

  it("ends on an end frame, with the reason the server gave, and opens nothing after it", async () => {
    // M6.1 close §6.2's "a DELETED game reads reconnecting forever", closed for
    // the stream: the server says so in a frame, and the consumer leaves.
    const ends: End[] = [];
    const { stream, stub } = streamed();
    renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("end", { reason: "deleted" });
    await settle();
    // **And the frame is given WORDS here** (final review I-2): `Signals::end`
    // writes a reason and nothing else, every consumer hands `end.sentence` to
    // its own leave, and only the game screen had a fallback — so a real
    // abandon took the dealing screen and the open table to the lobby in
    // silence. One place for all four, which is what this asserts.
    expect(ends).toEqual([{ reason: "deleted", sentence: endSentence("deleted") }]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 3);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
  });

  it("ends on a 404 and on a 403 with the server's sentence, and never backs off over either", async () => {
    // **Two refusals that END this rather than slowing it down.** The table is
    // gone, or it is not ours to read; neither changes by being asked again,
    // and the screen would otherwise sit saying "reconnecting…" about something
    // that is not coming back.
    for (const [status, reason, sentence] of [
      [404, "gone", 'no game named "wire"'],
      [403, "refused", "this table is locked — unlock it first"],
    ] as const) {
      const ends: End[] = [];
      const stub = stubApi(() => ({ status, body: { error: sentence } }));
      const { unmount } = renderHook(() =>
        useSignal("wire", true, () => {}, (e) => ends.push(e)),
      );
      await settle();
      expect(ends).toEqual([{ reason, sentence }]);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
      });
      expect(stub.matching("GET", "/stream")).toHaveLength(1);
      unmount();
    }
  });

  it("ends on the EDGE's expired session, with the sentence the door links from", async () => {
    // **Important 2 of the whole-branch review, and the false sentence under
    // it.** `readStream` classifies Access's 401 and throws `ACCESS_CHALLENGE`
    // (`sse.ts`), but until this arm the only consumer treated every status
    // that is not 404 or 403 as "the re-read failed" — so a player holding a
    // stream when their Access session expired read **"reconnecting…"** for
    // ever, which is the one word that says there is nothing to do. It ends
    // here, with the sentence, and the sentence brings `/enter` with it
    // wherever it is drawn (`Refusal`).
    const ends: End[] = [];
    const stub = stubApi((call) =>
      call.path.endsWith("/stream")
        ? new Response("<html>", {
            status: 401,
            headers: { "www-authenticate": 'Cloudflare-Access resource_metadata="https://x/y"' },
          })
        : undefined,
    );
    renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await settle();
    expect(ends).toEqual([{ reason: "refused", sentence: ACCESS_CHALLENGE }]);
    // And no trip round the ladder: passing the turnstile is what fixes this,
    // and asking again cannot.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 3);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
  });

  it("hands nothing over once it has been torn down", async () => {
    // `App`'s `active` includes `!busy`, so the effect is torn down and rebuilt
    // on EVERY opponent move; a frame in flight across that boundary is the
    // ordinary case rather than the edge one, and handing it over would re-read
    // a game nobody is watching.
    const seen: Events[] = [];
    const { stream } = streamed();
    const { rerender } = renderHook(({ active }) => useSignal("wire", active, (e) => seen.push(e)), {
      initialProps: { active: true },
    });
    await until(() => stream.opens() > 0, "the stream was opened");
    rerender({ active: false });
    stream.push("signal", at(7));
    await settle();
    expect(seen).toHaveLength(0);
  });
});

describe("what the link word rests on (review I-1, M-2, M-3)", () => {
  it("stays reconnecting while the reopen's request is in flight, and says live when the 200 lands", async () => {
    // **A request that has gone out is not a connection.** Saying `"live"` when
    // the `fetch` is ISSUED rather than when it is answered has two costs, and
    // both are on screen: during an outage the word blinks off at every rung of
    // the ladder — open, "live", reject, "reconnecting" — and against a server
    // that accepts a connection and then hangs, the banner reads "live"
    // indefinitely while nothing at all is connected. §5.1 ties the word to
    // CONNECTED.
    const held = gate<Reply>();
    let opens = 0;
    const stub = stubApi(() => {
      opens += 1;
      return opens === 1 ? { status: 502, body: { error: "gone" } } : held.promise;
    });
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    await until(() => result.current === "reconnecting", "the first open failed");

    // The ladder's first rung opens again, and the answer is withheld.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 10);
    });
    await until(() => opens === 2, "the reopen went out");
    expect(stub.matching("GET", "/stream")).toHaveLength(2);
    // **The request is out and the word has NOT moved.** This is the assertion
    // the old line could not pass.
    expect(result.current).toBe("reconnecting");

    held.resolve(openStream());
    await until(() => result.current === "live", "the connection was made");
    expect(result.current).toBe("live");
  });

  it("holds refreshing until the 200, and then says live with no frame at all", async () => {
    // The other half, and the one the surrounding doc claims: "refreshing" is
    // true until the tab is CONNECTED again, not until its request went out.
    // **And the answer is a 200 with an empty, hanging body** — which is what
    // the house channel is, since it writes nothing on connect (R-3.4). So the
    // word cannot be cleared by a frame here; only the connection can clear it.
    const held = gate<Reply>();
    let opens = 0;
    stubApi(() => {
      opens += 1;
      return opens === 1 ? openStream() : held.promise;
    });
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    // **Waited for by the CONNECTION and not by the word**, because `"live"` is
    // also this hook's resting state before it has opened anything: a wait on
    // the word alone returns at once, having given the effect no turn at all,
    // and every assertion after it is about a hook that never connected.
    // Measured — that is how the first draft of this cell failed.
    await until(() => opens === 1, "the first connection was made");
    expect(result.current).toBe("live");

    act(() => visibility("hidden"));
    act(() => visibility("visible"));
    expect(result.current).toBe("refreshing");
    await until(() => opens === 2, "the reopen went out");
    expect(result.current).toBe("refreshing");

    held.resolve(openStream());
    await until(() => result.current === "live", "the connection was made again");
    expect(result.current).toBe("live");
  });

  it("aborts its reader when a frame handler throws, so one hook holds one reader", async () => {
    // **A throw out of `onFrame` abandons the body.** `readStream` rejects, the
    // ladder opens another — and without this the first reader is still holding
    // its connection, so a consumer that threw once leaves a reader behind per
    // failure.
    let thrown = 0;
    const { stream, stub } = streamed(() => [["signal", at(1)]]);
    renderHook(() =>
      useSignal("wire", true, () => {
        thrown += 1;
        throw new Error("the consumer threw");
      }),
    );
    await until(() => thrown > 0, "the consumer was handed a frame");
    await until(
      () => stub.matching("GET", "/stream")[0]!.signal!.aborted,
      "the abandoned reader was let go",
    );
    expect(stub.matching("GET", "/stream")[0]!.signal!.aborted).toBe(true);
    // And a frame written to it now reaches nobody, which is what "let go"
    // means rather than merely "not read from".
    const before = thrown;
    stream.push("signal", at(2));
    await settle();
    expect(thrown).toBe(before);
  });

  it("resets the ladder on the CONNECTION, so a stream that says nothing on connect does not climb it", async () => {
    // **The house channel writes nothing on connect** (R-3.4), so a client that
    // reset its failure count only on a FRAME would have the lobby's ladder
    // climb monotonically through every reconnect of a quiet server — 2 s, 4 s,
    // 8 s and on to the ceiling — while each of those connections was made
    // perfectly well. A 200 with a stream body IS the connection.
    let opens = 0;
    const stream = fakeStream();
    const stub = stubApi((call) => {
      opens += 1;
      return opens === 1 ? { status: 502, body: { error: "gone" } } : stream.reply(call);
    });
    const { result } = renderHook(() =>
      usePolled<number>(
        { key: "lobby", once: () => Promise.resolve(0), stream: "/api/stream" },
        true,
        () => {},
      ),
    );
    await until(() => result.current === "reconnecting", "the first open failed");

    // The first rung connects, and says nothing.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 10);
    });
    await until(() => result.current === "live", "the second open connected");
    expect(stub.matching("GET", "/api/stream")).toHaveLength(2);

    // That connection drops. The next wait is the FIRST rung again — not the
    // second, which is what a count reset only by frames would have produced.
    stream.close();
    await until(() => result.current === "reconnecting", "the connection dropped");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! - 200);
    });
    expect(stub.matching("GET", "/api/stream")).toHaveLength(2);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(400);
    });
    await until(
      () => stub.matching("GET", "/api/stream").length === 3,
      "the ladder's first rung came round again",
    );
    expect(stub.matching("GET", "/api/stream")).toHaveLength(3);
  });
});

describe("the two frame modes (R-7.1)", () => {
  it("a value source reads the frame itself, and never asks its once route", async () => {
    // The game and the dealing screen: a `signal` frame IS an `Events`, so the
    // frame's own body is the answer and `GET …/events` is never reached. That
    // is what makes R-3.2's "zero requests to /events" true.
    const seen: Events[] = [];
    const { stub } = streamed(() => [["signal", at(11)]]);
    renderHook(() => usePolled<Events>(gameSource("wire"), true, (e) => seen.push(e)));
    await settle();
    expect(seen.map((e) => e.events)).toEqual([11]);
    expect(stub.matching("GET", "/events")).toHaveLength(0);
  });

  it("a wake source re-reads its own resource, and the frame's body never reaches the consumer", async () => {
    // The lobby and the open table: their value is a listing and a `TableView`,
    // and the house channel carries a SEQUENCE NUMBER (R-3.4). A frame cannot
    // be the value, so the frame is a wake-up and `once` is the answer.
    const seen: TableView[] = [];
    const { stream, stub } = streamed(undefined, (call) =>
      call.path.endsWith("/table") ? wire.tableOpen : undefined,
    );
    renderHook(() =>
      usePolled<TableView>(
        {
          key: "wire",
          once: () => api.table("wire"),
          stream: "/api/games/wire/stream",
        },
        true,
        (t) => seen.push(t),
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    // Nothing on connect here — this cell's stream says nothing until told —
    // so the re-read count is exactly the frame count.
    expect(stub.matching("GET", "/table")).toHaveLength(0);
    stream.push("house", { seq: 4 });
    await settle();
    expect(stub.matching("GET", "/table")).toHaveLength(1);
    expect(seen).toHaveLength(1);
    // The VALUE is the route's, not the frame's: a hook that handed `{seq: 4}`
    // to this consumer would have the lobby render a sequence number.
    expect(seen[0]!.id).toBe(wire.tableOpen.id);
  });

  it("coalesces a burst of frames into one re-read in flight and one repeat", async () => {
    // **A frame is cheap and the read behind it is not.** The lobby's `once` is
    // three table listings, and a game under way bumps the house on every move;
    // one read per frame would queue a listing each. One at a time — and the
    // frames that arrived during one are not LOST either, which is what the
    // repeat is for: a burst answers once more, not once per frame.
    const held: Array<(v: number) => void> = [];
    let asks = 0;
    const { stream } = streamed();
    renderHook(() =>
      usePolled<number>(
        {
          key: "lobby",
          once: () => {
            asks += 1;
            return new Promise<number>((r) => held.push(r));
          },
          stream: "/api/stream",
        },
        true,
        () => {},
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("house", { seq: 1 });
    await settle();
    expect(asks).toBe(1);
    // Two more while the first is still in flight.
    stream.push("house", { seq: 2 });
    stream.push("house", { seq: 3 });
    await settle();
    expect(asks).toBe(1);
    // The first lands, and exactly ONE repeat follows for the two frames that
    // arrived under it.
    held[0]!(1);
    await settle();
    expect(asks).toBe(2);
    held[1]!(2);
    await settle();
    expect(asks).toBe(2);
  });

  it("ends when a woken re-read is refused, and stops reading the stream with it", async () => {
    // The table this stream is about is gone, or is not ours. The frames would
    // go on arriving, so the READER is let go too — a connection kept open to a
    // resource we may not read delivers wake-ups nothing can act on.
    const ends: End[] = [];
    const { stream, stub } = streamed(undefined, (call) =>
      call.path.endsWith("/table")
        ? { status: 404, body: { error: "no such game" } }
        : undefined,
    );
    renderHook(() =>
      usePolled<TableView>(
        { key: "wire", once: () => api.table("wire"), stream: "/api/games/wire/stream" },
        true,
        () => {},
        (e) => ends.push(e),
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("signal", at(1));
    await settle();
    expect(ends).toEqual([{ reason: "gone", sentence: "no such game" }]);
    expect(stub.matching("GET", "/stream")[0]!.signal!.aborted).toBe(true);
    // And nothing is opened in its place.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
  });

  it("ends when a woken re-read meets an expired Access session", async () => {
    // The same refusal from the other side: the stream is fine and the READ
    // behind it met the edge. Before this arm it set "reconnecting" and waited
    // for a frame that would only meet the same door.
    const ends: End[] = [];
    const { stream } = streamed(undefined, (call) =>
      call.path.endsWith("/table")
        ? new Response("<html>", {
            status: 401,
            headers: { "www-authenticate": "Cloudflare-Access" },
          })
        : undefined,
    );
    renderHook(() =>
      usePolled<TableView>(
        { key: "wire", once: () => api.table("wire"), stream: "/api/games/wire/stream" },
        true,
        () => {},
        (e) => ends.push(e),
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("signal", at(1));
    await settle();
    expect(ends).toEqual([{ reason: "refused", sentence: ACCESS_CHALLENGE }]);
  });

  it("says reconnecting when a woken re-read FAILS, keeps the stream, and recovers on the next frame", async () => {
    // **Not a trip round the ladder.** The stream is still good and the read
    // behind it is what failed, so the connection is kept and the next frame
    // tries again — while the word on screen stays the true one, because what
    // is drawn is older than what the server has.
    let broken = true;
    const { stream, stub } = streamed(undefined, (call) =>
      call.path.endsWith("/table")
        ? broken
          ? { status: 500, body: { error: "the store stopped answering" } }
          : wire.tableOpen
        : undefined,
    );
    const { result } = renderHook(() =>
      usePolled<TableView>(
        { key: "wire", once: () => api.table("wire"), stream: "/api/games/wire/stream" },
        true,
        () => {},
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("signal", at(1));
    await settle();
    expect(result.current).toBe("reconnecting");
    // The same connection, still open: no second one was needed.
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    expect(stub.matching("GET", "/stream")[0]!.signal!.aborted).toBe(false);

    broken = false;
    stream.push("signal", at(2));
    await settle();
    expect(result.current).toBe("live");
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
  });
});

describe("the held-request arm (R-plan-5)", () => {
  it("re-asks a WAKE source after the named wait, and never opens the stream", async () => {
    // The fallback for an edge that will not hold a stream open: the same hook,
    // one frame per request. **For a `"wake"` source the re-ask is at
    // `HELD_REASK_MS` and not at once** — the lobby's and the open table's
    // `once` have no waiting endpoint at all, so a reopen at zero would be a
    // hot loop against three listings. The cell below is the other half, and
    // the two delays are why one number could not serve both (I-3).
    setSignalKind("held");
    const seen: Events[] = [];
    const { stub } = streamed(undefined, (call) =>
      call.path.includes("/events") ? at(2) : undefined,
    );
    renderHook(() => usePolled<Events>(wakeSource("wire"), true, (e) => seen.push(e)));
    await settle();
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    expect(stub.matching("GET", "/stream")).toHaveLength(0);
    expect(seen.map((e) => e.events)).toEqual([2]);

    // Not before the wait, and then exactly once more.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(HELD_REASK_MS - 100);
    });
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    await settle();
    expect(stub.matching("GET", "/events")).toHaveLength(2);
    expect(stub.matching("GET", "/stream")).toHaveLength(0);
  });

  // **A source whose `once` HOLDS is re-asked at ONCE** (final review I-3).
  //
  // The number that was shared cost real latency and not merely requests:
  // `?wait` subscribes and waits for a NEW bump, and nothing compares the
  // caller's count (R-3.14), so a bump landing in the 3 s gap was not seen
  // until the NEXT request's 25 s deadline — 27–28 s under a hook whose whole
  // claim is to be faster than the 3 s poll it replaced.
  //
  // The discriminator is the CLOCK: `settle` advances it by twelve
  // milliseconds in all, which fires a timer armed at 0 and cannot reach one
  // armed at 3,000. The request is held so that exactly one is ever in flight,
  // which is what makes the second ask a reading about the re-ask and not
  // about the answer racing it.
  it("re-asks a source whose own request HOLDS at once, not after the wake delay", async () => {
    setSignalKind("held");
    const holding: Array<() => void> = [];
    const { stub } = streamed(undefined, (call) =>
      call.path.includes("/events")
        ? new Promise<Reply>((resolve) => holding.push(() => resolve(at(2))))
        : undefined,
    );
    renderHook(() => usePolled<Events>(gameSource("wire"), true, () => {}));
    await until(() => holding.length === 1, "the first held request went out");
    expect(stub.matching("GET", "/events")).toHaveLength(1);

    holding[0]!();
    await until(() => holding.length === 2, "the next ask went out without waiting");
    expect(stub.matching("GET", "/events")).toHaveLength(2);
    expect(stub.matching("GET", "/stream")).toHaveLength(0);
  });

  it("asks the game's events route to HOLD, which is what makes one request a signal", async () => {
    setSignalKind("held");
    const { stub } = streamed(undefined, (call) => (call.path.includes("/events") ? at(2) : undefined));
    renderHook(() => useSignal("wire", true, () => {}));
    await settle();
    // `wait` is the server's held request (R-plan-5): without it this arm would
    // be a three-second poll again, wearing the ladder's name.
    expect(stub.matching("GET", "/events")[0]!.path).toContain("wait=25");
  });
});

describe("the hook's own rules, whatever the transport", () => {
  it("does not re-open when a consumer re-renders with a new closure", async () => {
    // **The hazard the refs exist for.** Every consumer closes over state, so
    // the `Source` is a new object on every render; an effect that depended on
    // one would abort its reader and open another each time, which over a fast
    // re-rendering screen is a connection per render.
    const seen: number[] = [];
    const { stream, stub } = streamed();
    const { rerender } = renderHook(
      ({ n }) =>
        usePolled<number>(
          {
            key: "lobby",
            once: () => Promise.resolve(n),
            stream: "/api/stream",
            frames: "value",
          },
          true,
          (v) => seen.push(v),
        ),
      { initialProps: { n: 0 } },
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    for (let n = 1; n <= 5; n += 1) rerender({ n });
    await settle();
    expect(stub.matching("GET", "/api/stream")).toHaveLength(1);
    // And the value handed over is the LAST closure's, so the callback is read
    // at delivery time rather than captured when the reader was opened.
    stream.push("signal", 5);
    await settle();
    expect(seen).toEqual([5]);
  });

  it("says nothing once it has stopped watching", async () => {
    // A word about a connection that is no longer kept is a word about nothing:
    // switch games from the menu while the server is unreachable and the new
    // game's banner would open reading "reconnecting…" about a stream nobody
    // opened.
    const stub = stubApi(() => ({ status: 502, body: { error: "gone" } }));
    const { result, rerender } = renderHook(({ active }) => useSignal("wire", active, () => {}), {
      initialProps: { active: true },
    });
    await settle();
    expect(result.current).toBe("reconnecting");
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    rerender({ active: false });
    expect(result.current).toBe("live");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
  });

  it("opens nothing at all while inactive", async () => {
    const { stub } = streamed();
    renderHook(() => useSignal("wire", false, () => {}));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(stub.calls).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// M6.5b: a LOCAL game has no signal (R-6.5b.12, spec §2.3).
//
// The cell that must fail against a client with no `host` arm is the first
// one: `vi.getTimerCount() === 0` across sixty fake seconds AND no request at
// all. Nothing to wait for is the reason — the bot's trailing run returns
// inside the reply that caused it, and nobody else sits at the table.

describe("the signal, for a game this browser answers", () => {
  it("opens nothing, asks nothing and arms NO timer, and still reads live", async () => {
    const { stub } = streamed();
    const seen: Events[] = [];
    const { result } = renderHook(() => useSignal("local-1", true, (e) => seen.push(e)));
    await settle();
    expect(stub.calls).toHaveLength(0);
    expect(vi.getTimerCount()).toBe(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(stub.calls).toHaveLength(0);
    expect(vi.getTimerCount()).toBe(0);
    expect(seen).toEqual([]);
    // "live" is TRUE and not a placeholder: the host answering this game is
    // in this tab.
    expect(result.current).toBe("live");
  });

  it("leaves the lobby's own stream open while a local game is on screen", async () => {
    // Spec §6.2 (I-6): the lobby's and the open table's sources never say
    // `host`, so the server's streams are untouched by what game is being
    // played beside them. The anti-vacuity half of the cell above — the same
    // arrangement, the same clock, one stream opened and not the other.
    const { stream, stub } = streamed();
    renderHook(() => useSignal("local-1", true, () => {}));
    renderHook(() => usePolled(wakeSource("lobby"), true, () => {}));
    await until(() => stream.opens() > 0, "the lobby's stream was opened");
    expect(stub.matching("GET", "/api/games/lobby/stream")).toHaveLength(1);
    expect(stub.matching("GET", "/api/games/local-1/stream")).toHaveLength(0);
  });
});
