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
  type End,
  type Source,
} from "./signal";
import { ACCESS_CHALLENGE, BOUND_MS, SIGN_IN_NEEDED, api, resetHealthReadForTest, setSignalKind, signalKind } from "./api";
import { resetFactsForTest } from "./local/facts";
import { resetTurnstileForTests, setChallengeHandler } from "./turnstile";
import { FakeSocket } from "./test/fakeSocket";
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
 *  source, whose `once` is a plain `GET …/events` that answers at once. */
const gameSource = (game: string): Source<Events> => ({
  key: game,
  once: () => api.events(game),
  stream: `/api/games/${game}/stream`,
  frames: "value",
});

/** The lobby's and the open table's shape: a `"wake"` source, whose `once` is
 *  an ordinary read that answers at once, asked on every frame. */
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

  it("ends on a 429 — the stream cap's refusal — with the server's sentence, and never backs off over it", async () => {
    // **The NAS's stream cap and the Worker's socket cap end a signal alike** (security S2c, D3; decision D-2):
    // a 429 at admission is an answer about THIS caller's open streams, and asking again every few seconds would be
    // a retry storm against the very cap. The consumer draws the sentence: the table screens land in the lobby with
    // it (Review Focus 1).
    const ends: End[] = [];
    const sentence = "too many open streams from you at this table — close one and try again";
    const stub = stubApi(() => ({ status: 429, body: { error: sentence } }));
    const { unmount } = renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await settle();
    expect(ends).toEqual([{ reason: "refused", sentence }]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    unmount();
  });

  it("opens nothing after an end, even when the tab is shown again", async () => {
    // **An end ENDS the run** (security S2c, the Task 5 review's I-1): the lobby is the one consumer that stays on
    // screen after an end, and a tab hidden and shown again — what the cap's sentence asks the player to do with
    // another tab — must not open the stream behind the ended sentence.
    const ends: End[] = [];
    const sentence = "too many open streams from you at this table — close one and try again";
    const stub = stubApi(() => ({ status: 429, body: { error: sentence } }));
    const { unmount } = renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await settle();
    expect(ends).toEqual([{ reason: "refused", sentence }]);
    act(() => visibility("hidden"));
    act(() => visibility("visible"));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    expect(ends).toHaveLength(1);
    unmount();
  });

  it("says nothing beside an ended signal", async () => {
    // **The link describes a signal that is RUNNING** (the hook's own rule; the Task 5 review's I-1): a tab shown
    // again says "refreshing…", and when that reopen is refused the word is let go with the run, rather than
    // standing beside the ended sentence for ever.
    const ends: End[] = [];
    const sentence = "too many open streams from you at this table — close one and try again";
    let refuse = false;
    const { stream, stub } = streamed(undefined, (call) =>
      refuse && call.path.endsWith("/stream") ? { status: 429, body: { error: sentence } } : undefined,
    );
    const { result } = renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await until(() => stream.opens() > 0, "the stream was opened");
    await settle();
    expect(result.current).toBe("live");
    refuse = true;
    act(() => visibility("hidden"));
    act(() => visibility("visible"));
    await until(() => ends.length > 0, "the reopen was refused");
    expect(ends).toEqual([{ reason: "refused", sentence }]);
    expect(stub.matching("GET", "/stream")).toHaveLength(2);
    expect(result.current).toBe("live");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(result.current, "no word stands beside the ended sentence").toBe("live");
  });

  it("says nothing after an end when the tab is hidden and shown again", async () => {
    const ends: End[] = [];
    stubApi(() => ({ status: 429, body: { error: "too many" } }));
    const { result } = renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await settle();
    expect(ends).toHaveLength(1);
    act(() => visibility("hidden"));
    act(() => visibility("visible"));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(result.current, "no word stands beside the ended sentence").toBe("live");
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
  // **R-N3: the first connection's word** (M6.5c close §6.3 item 11). A signal
  // that has never connected says "connecting"; only a connection that was made
  // and then lost says "reconnecting" — a first visit used to read
  // "reconnecting…" about a link it never had.
  it("says connecting until its first connection, and reconnecting only once one is lost (R-N3)", async () => {
    let opens = 0;
    const stream = fakeStream();
    stubApi((call) => {
      if (!call.path.endsWith("/stream")) return undefined;
      opens += 1;
      return opens === 1 ? { status: 502, body: { error: "gone" } } : stream.reply(call);
    });
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    await until(() => result.current === "connecting", "the first open failed");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 10);
    });
    await until(() => result.current === "live", "the first connection");
    stream.close();
    await until(() => result.current === "reconnecting", "the connection it made was lost");
  });

  it("stays connecting while the reopen's request is in flight, and says live when the 200 lands", async () => {
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
    await until(() => result.current === "connecting", "the first open failed");

    // The ladder's first rung opens again, and the answer is withheld.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! + 10);
    });
    await until(() => opens === 2, "the reopen went out");
    expect(stub.matching("GET", "/stream")).toHaveLength(2);
    // **The request is out and the word has NOT moved.** This is the assertion
    // the old line could not pass.
    expect(result.current).toBe("connecting");

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
    await until(() => result.current === "connecting", "the first open failed");

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

  // **A wake the source's `peek` DECLINES is not re-read** (cycle E, Task 7;
  // R-E9, ruling T7-a lever c): the lobby's `peek` answers `false` for a house
  // frame that moved only the chat, whose listings have not changed. Two
  // frames are read whatever `peek` says, and each is a cell: the FIRST frame
  // on a connection, since nothing on it has been compared yet (a restarted
  // Worker counts its `seq` from 0 again, and an outage's missed changes are
  // owed); and any frame after a re-read that FAILED, since the listings on
  // screen are then older than the frame, and the next frame is the retry
  // (the catch below says so).
  it("does not re-read a frame its peek declines, but reads a connection's first frame whatever peek says", async () => {
    let asks = 0;
    const peeked: string[] = [];
    const { stream } = streamed();
    renderHook(() =>
      usePolled<number>(
        {
          key: "lobby",
          once: async () => (asks += 1),
          stream: "/api/stream",
          peek: (data) => {
            peeked.push(data);
            return false;
          },
        },
        true,
        () => {},
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("house", { seq: 1, chat: 0 });
    await settle();
    expect(asks, "the connection's first frame is read").toBe(1);
    stream.push("house", { seq: 1, chat: 1 });
    stream.push("house", { seq: 1, chat: 2 });
    await settle();
    expect(asks, "a declined frame is not").toBe(1);
    expect(peeked, "and every frame was peeked").toHaveLength(3);
  });

  it("re-reads a declined frame after a re-read that failed, and declines again once one lands", async () => {
    let asks = 0;
    let down = true;
    const { stream } = streamed();
    renderHook(() =>
      usePolled<number>(
        {
          key: "lobby",
          once: async () => {
            asks += 1;
            if (down) throw new Error("the listings could not be read");
            return asks;
          },
          stream: "/api/stream",
          peek: () => false,
        },
        true,
        () => {},
      ),
    );
    await until(() => stream.opens() > 0, "the stream was opened");
    stream.push("house", { seq: 1, chat: 0 });
    await settle();
    expect(asks, "the first frame's read, which fails").toBe(1);
    down = false;
    stream.push("house", { seq: 1, chat: 1 });
    await settle();
    expect(asks, "the declined frame after a failure is the retry").toBe(2);
    stream.push("house", { seq: 1, chat: 2 });
    await settle();
    expect(asks, "and once a read has landed, a declined frame is declined").toBe(2);
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

  it("keeps the stream when a woken re-read is refused 429 (AD-6: the cap answers at admission, never at a listing read)", async () => {
    const { stream, stub } = streamed(undefined, (call) =>
      call.path.endsWith("/table") ? { status: 429, body: { error: "too many requests" } } : undefined,
    );
    const ends: End[] = [];
    const { result } = renderHook(() =>
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
    expect(ends).toEqual([]);
    expect(result.current).toBe("reconnecting");
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    expect(stub.matching("GET", "/stream")[0]!.signal!.aborted).toBe(false);
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
    expect(result.current).toBe("connecting");
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

// ---------------------------------------------------------------------------
// M6.5c: the SOCKET arm (spec §2.7; R-6.5c.6).
//
// The Worker serves `"ws"`: the stream's own frames over a hibernatable
// WebSocket, handed to the SAME handler the stream reader feeds — so every rule
// above about what a frame MEANS is this arm's too, and is not restated here.
// What is new is what a socket cannot say. A browser reports a refused upgrade
// as an `error` that says nothing and a close with 1006, so a close nobody
// explained is followed by ONE plain fetch — the source's own `once()`,
// through `call` — that CAN say what happened; and a close the arm made itself
// is followed by nothing at all. These are batteries 23, 37 and 38 of spec
// §5.7 and the plan's Review Focus 2 and 3.

describe("the signal, over a WebSocket (M6.5c)", () => {
  beforeEach(() => {
    // The suite's own `afterEach` puts the transport back to `"sse"`.
    setSignalKind("ws");
    FakeSocket.made = [];
    vi.stubGlobal("WebSocket", FakeSocket);
  });

  /** The `n`th socket the hook opened, once it has. */
  const socket = async (n: number): Promise<FakeSocket> => {
    await until(() => FakeSocket.made.length >= n, `socket ${n} was opened`);
    return FakeSocket.made[n - 1]!;
  };

  // **Battery 23: the arm laddering without the probe.** The expired Access
  // session is the case the probe exists for: the edge refuses the upgrade,
  // the page hears 1006 and an `error` that says nothing, and a ladder would
  // say "reconnecting…" for ever about a door only the player can walk
  // through — M6.5b2's final-review I-2, back again. The probe is a plain
  // fetch, so the edge's 401 is READABLE on it, and it ends the source exactly
  // as the stream arm's cell above ends on it.
  it("probes ONCE after a socket that closes before it opens, and ends on the edge's expired session (battery 23)", async () => {
    const ends: End[] = [];
    const stub = stubApi((call) =>
      call.path.includes("/events")
        ? new Response("<html>", {
            status: 401,
            headers: { "www-authenticate": 'Cloudflare-Access resource_metadata="https://x/y"' },
          })
        : undefined,
    );
    renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    const refused = await socket(1);
    act(() => refused.drop());
    await until(() => ends.length > 0, "the source ended");
    expect(ends).toEqual([{ reason: "refused", sentence: ACCESS_CHALLENGE }]);
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    // Nothing is scheduled after it: passing the turnstile fixes this, and
    // asking again cannot.
    expect(vi.getTimerCount()).toBe(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 3);
    });
    expect(FakeSocket.made).toHaveLength(1);
    expect(stub.calls).toHaveLength(1);
    expect(ends).toHaveLength(1);
  });

  // **The same end on a provider-signed Worker's challenge** (the auth cycle,
  // Task 6 fix round 1, review M-1): under a sign-in arm the Worker's 401 names
  // `Yokohama-Session`, which `call` reads as `SIGN_IN_NEEDED` — and on the
  // Worker the socket IS the transport, so the probe is where a player whose
  // session ended learns it. Mutant: the probe's catch ending on Access's
  // sentence alone (the ladder says "reconnecting…" about a sign-in).
  it("probes ONCE after a socket that closes before it opens, and ends on a provider-signed Worker's session challenge", async () => {
    const ends: End[] = [];
    const stub = stubApi((call) =>
      call.path.includes("/events")
        ? new Response(JSON.stringify({ error: "no identity reached the server" }), {
            status: 401,
            headers: { "content-type": "application/json", "www-authenticate": 'Yokohama-Session realm="/enter"' },
          })
        : undefined,
    );
    renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    const refused = await socket(1);
    act(() => refused.drop());
    await until(() => ends.length > 0, "the source ended");
    expect(ends).toEqual([{ reason: "refused", sentence: SIGN_IN_NEEDED }]);
    expect(stub.matching("GET", "/events")).toHaveLength(1);
    // Nothing is scheduled after it: signing in fixes this, and asking again
    // cannot.
    expect(vi.getTimerCount()).toBe(0);
  });

  // **Battery 37 and Review Focus 2: a close the arm made ITSELF asks
  // nothing.** Hiding the tab is the arm hanging up (R-P8), and a probe then
  // would be a request from a hidden tab — the one thing R-P8 forbids — about
  // a close whose cause is already known. Shown again, the arm opens a new
  // socket and the server's first frame on accept is the resync (M6.3), with
  // no probe, no reload and no "reconnecting…".
  it("asks nothing about a close it made itself: hidden, it lets the socket go; shown, it reopens and resyncs from the first frame (battery 37)", async () => {
    const seen: Events[] = [];
    // ANY request fails this cell: no close here earns a probe.
    const stub = stubApi(() => undefined);
    const { result } = renderHook(() => useSignal("wire", true, (e) => seen.push(e)));
    const first = await socket(1);
    // The source's own stream URL, as a socket.
    expect(first.url).toBe(`ws://${location.host}/api/games/wire/stream`);
    act(() => {
      first.accept();
      first.frame("signal", at(1));
    });
    expect(seen.map((e) => e.events)).toEqual([1]);
    expect(result.current).toBe("live");

    act(() => visibility("hidden"));
    expect(first.closedWith).toBe(1000);
    await settle();
    // **The word a hidden tab keeps is the one it had** (M6.5c Task 8's Minor
    // 2): the arm's own close explains itself and asks nothing, so it says
    // nothing either. Held by code ORDER until this line — the `setLink` after
    // the own-close return (`signal.ts`) — which is what a mutant can reorder.
    expect(result.current, "hidden, the word is still live").toBe("live");
    expect(stub.calls).toHaveLength(0);
    // And nothing is opened, asked or handed over while the tab stays hidden.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 2);
    });
    expect(result.current, "and still live a minute later").toBe("live");
    expect(FakeSocket.made).toHaveLength(1);
    expect(stub.calls).toHaveLength(0);
    expect(seen.map((e) => e.events)).toEqual([1]);

    act(() => visibility("visible"));
    expect(result.current).toBe("refreshing");
    const second = await socket(2);
    // A socket that has been made is not a connection yet.
    expect(result.current).toBe("refreshing");
    act(() => {
      second.accept();
      second.frame("signal", at(2));
    });
    expect(result.current).toBe("live");
    expect(seen.map((e) => e.events)).toEqual([1, 2]);
    expect(stub.calls).toHaveLength(0);
  });

  // **Review Focus 3: a deploy during a game.** Every socket closes 1006 and
  // says nothing. The arm says so at once, probes once — and the probe's
  // answer is a free resync, handed over like a frame, so a move made while the
  // socket was down is not lost — then climbs the SAME ladder the stream does.
  it("after a close nobody explained, says reconnecting, hands the probe's answer over, and reopens on the ladder's first rung (Review Focus 3)", async () => {
    const seen: Events[] = [];
    const probe = gate<Reply>();
    const stub = stubApi((call) => (call.path.includes("/events") ? probe.promise : undefined));
    const { result } = renderHook(() => useSignal("wire", true, (e) => seen.push(e)));
    const first = await socket(1);
    act(() => {
      first.accept();
      first.frame("signal", at(4));
    });
    expect(result.current).toBe("live");

    act(() => first.drop());
    await until(() => stub.matching("GET", "/events").length === 1, "the probe went out");
    // **Not "live" while the probe is out**: nothing is connected, which is
    // what the word is tied to (spec §5.1).
    expect(result.current).toBe("reconnecting");
    // The probe is `call`'s, so it carries the header that makes Access answer
    // a readable 401 rather than a redirect (`serverHeaders`).
    expect(stub.matching("GET", "/events")[0]!.headers["x-requested-with"]).toBe("XMLHttpRequest");

    probe.resolve(at(5));
    await until(() => seen.length === 2, "the probe's answer was handed over");
    expect(seen.map((e) => e.events)).toEqual([4, 5]);
    expect(result.current).toBe("reconnecting");

    // The ladder's first rung, and not before it.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! - 100);
    });
    expect(FakeSocket.made).toHaveLength(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    const second = await socket(2);
    act(() => {
      second.accept();
      second.frame("signal", at(6));
    });
    expect(result.current).toBe("live");
    expect(seen.map((e) => e.events)).toEqual([4, 5, 6]);
    // ONE probe for one close.
    expect(stub.matching("GET", "/events")).toHaveLength(1);
  });

  // **The probe asks for the reading NOW** (M6.5c2 cycle D, R-D5). Until the
  // held arm was retired the game's `once` sent `wait=25` for that arm's sake,
  // and this probe carried it; now it is a plain `GET …/events` with no query
  // at all, which every host answers at once. The three held-arm cells this
  // replaces pinned a transport no host ever named. An older release's
  // `wait=25` is still parsed (`a_wait_is_accepted_and_answered_at_once`, in
  // the layer). Mutant: the probe asking `wait` again.
  it("sends the probe after a close nobody explained with no wait: a plain read of the game's events", async () => {
    const stub = stubApi((call) => (call.path.includes("/events") ? at(2) : undefined));
    renderHook(() => useSignal("wire", true, () => {}));
    const first = await socket(1);
    act(() => {
      first.accept();
      first.frame("signal", at(1));
    });
    act(() => first.drop());
    await until(() => stub.matching("GET", "/events").length === 1, "the probe went out");
    const [probe] = stub.matching("GET", "/events");
    expect(probe!.path, "no wait, and no query at all").toMatch(/\/api\/games\/wire\/events$/);
  });

  // **A probe that lands after the screen has gone hands nothing over** — the
  // stream arm's rule ("hands nothing over once it has been torn down"), kept
  // for the one request this arm makes on its own.
  it("hands nothing over from a probe that lands after it was torn down", async () => {
    const seen: Events[] = [];
    const probe = gate<Reply>();
    const stub = stubApi((call) => (call.path.includes("/events") ? probe.promise : undefined));
    const { rerender } = renderHook(
      ({ active }) => useSignal("wire", active, (e) => seen.push(e)),
      { initialProps: { active: true } },
    );
    const first = await socket(1);
    act(() => first.accept());
    act(() => first.drop());
    await until(() => stub.matching("GET", "/events").length === 1, "the probe went out");
    rerender({ active: false });
    probe.resolve(at(5));
    await settle();
    expect(seen).toEqual([]);
    // And nothing is opened in its place.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 2);
    });
    expect(FakeSocket.made).toHaveLength(1);
  });

  // **M6.5c Task 8's Minor 1** (M6.5c2 spec §2.8): a probe answer that lands
  // after ITS attempt was aborted is dropped. The tab hid while the probe was
  // out; `onVisibility` aborted the attempt, but the probe (`call` takes no
  // signal) still resolved, and its value was handed to a HIDDEN tab — for the
  // game, a read R-P8 forbids, and a chat badge set to an older count if the
  // tab came back before it landed.
  it("drops a probe answer that lands after a hidden tab aborted its attempt", async () => {
    const seen: Events[] = [];
    const probe = gate<Reply>();
    const stub = stubApi((call) => (call.path.includes("/events") ? probe.promise : undefined));
    renderHook(() => useSignal("wire", true, (e) => seen.push(e)));
    const first = await socket(1);
    act(() => {
      first.accept();
      first.frame("signal", at(4));
    });
    act(() => first.drop());
    await until(() => stub.matching("GET", "/events").length === 1, "the probe went out");
    act(() => visibility("hidden"));
    probe.resolve(at(5));
    await settle();
    expect(seen.map((e) => e.events), "a hidden tab is handed nothing").toEqual([4]);
    // And nothing is asked or opened while it stays hidden.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 2);
    });
    expect(stub.calls).toHaveLength(1);
    expect(FakeSocket.made).toHaveLength(1);
  });

  // **M6.5c Task 8's escalation 5** (M6.5c2 spec §2.8): the probe gets the
  // BOUND_MS every other read here has. A black-holed probe — a server that
  // accepts and never answers — held the arm with no next socket until the
  // browser gave up on the fetch; bounded, it is a failure like any other and
  // climbs the ladder.
  it("bounds the probe: one that never answers climbs the ladder after BOUND_MS", async () => {
    const probe = gate<Reply>(); // never resolved
    const stub = stubApi((call) => (call.path.includes("/events") ? probe.promise : undefined));
    const { result } = renderHook(() => useSignal("wire", true, () => {}));
    const first = await socket(1);
    act(() => first.accept());
    act(() => first.drop());
    await until(() => stub.matching("GET", "/events").length === 1, "the probe went out");
    expect(result.current).toBe("reconnecting");
    // The bound, then the ladder's first rung — and not a moment before.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOUND_MS + BACKOFF_MS[0]! - 100);
    });
    expect(FakeSocket.made, "no second socket before the bound and the first rung").toHaveLength(1);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    const second = await socket(2);
    act(() => second.accept());
    expect(result.current).toBe("live");
    // ONE probe for one close, answered or not.
    expect(stub.matching("GET", "/events")).toHaveLength(1);
  });

  // **Battery 38: the close after an `end` read as a second refusal.** The
  // host sends `end` BEFORE its 4404 close (spec §2.4). Were that close read as
  // unexplained, the probe would meet the deleted table's 404 and end the source
  // a SECOND time — which this cell's `/events` answer is arranged to make
  // visible.
  it("ends ONCE on an end frame, and the close that follows it asks nothing (battery 38)", async () => {
    const ends: End[] = [];
    const stub = stubApi((call) =>
      call.path.includes("/events")
        ? { status: 404, body: { error: 'no game named "wire"' } }
        : undefined,
    );
    renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    const s = await socket(1);
    act(() => {
      s.accept();
      s.frame("end", { reason: "deleted" });
      s.shut(4404, "this table was abandoned");
    });
    await settle();
    expect(ends).toEqual([{ reason: "deleted", sentence: endSentence("deleted") }]);
    expect(stub.calls).toHaveLength(0);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1]! * 3);
    });
    expect(FakeSocket.made).toHaveLength(1);
    expect(ends).toHaveLength(1);
  });

  // **The stream arm's first cell, for the socket** (the whole-branch review's
  // M-2): a connected reader polls NOTHING (R-3.2), and "arms no timer while it
  // is connected" is stated for this arm with its one exception COUNTED rather
  // than excused — the keep-alive, which asks the server nothing. A poll here
  // would be the Worker's binding quota spent once per interval per open tab,
  // and the only other catcher of one is the live walk's `/events` count, which
  // CI never runs.
  it("holds ONE timer while it is connected — the keep-alive, which asks nothing — and never polls", async () => {
    const seen: Events[] = [];
    // ANY request fails this cell: a connected socket asks nothing at all.
    const stub = stubApi(() => undefined);
    const { result } = renderHook(() => useSignal("wire", true, (e) => seen.push(e)));
    const first = await socket(1);
    act(() => {
      first.accept();
      first.frame("signal", at(3));
    });
    expect(result.current).toBe("live");
    expect(vi.getTimerCount(), "the keep-alive, and nothing else").toBe(1);

    // A minute of the clock: two pings down the one socket, and not one request
    // — where a three-second poll would have made twenty.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(60_000);
    });
    expect(first.sent).toEqual(["ping", "ping"]);
    expect(stub.matching("GET", "/events"), "the socket arm never asks /events while connected").toEqual([]);
    expect(stub.calls).toEqual([]);
    expect(FakeSocket.made, "and never opens a second socket").toHaveLength(1);
    expect(vi.getTimerCount()).toBe(1);
    expect(seen.map((e) => e.events)).toEqual([3]);
  });
});

// ---------------------------------------------------------------------------
// **No connection opens before the transport is KNOWN** (M6.5c Task 15, the
// live tier's finding 1; ruling I-81).
//
// The live run's lobby asked `GET /api/stream` as a STREAM and the Worker
// answered 426: `signalKind()` reads the server's facts, whose default is the
// stream until `GET /api/health` has answered, and the lobby mounts while the
// health read App makes on entering the multiplayer channel is still out. So
// the hook waits for that answer when this page has none, and a read that
// FAILED is the ladder's business — never a guess at the stream.

// ---- cycle F: a WATCHER whose clearance lapses (Review Focus 4; spec §2.6) --
//
// **A watcher makes no other call**: the stream (or the socket) is the one
// request it holds, so when the edge's clearance lapses it is the reopen the
// edge challenges. The widget is shown ONCE — through the seam, here a handler
// that passes at once — and then the signal reopens on its own ladder, under
// the clearance, and says live again. Without the one `call` the stream arm
// ENDED on the challenge's 403 ("refused"), and the socket arm laddered for
// ever against an upgrade it cannot read: F battery 25 is the stream's cell.
describe("the edge's challenge under a held signal (cycle F, Review Focus 4)", () => {
  const challenged = () =>
    new Response("<html>challenge</html>", {
      status: 403,
      headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
    });

  beforeEach(() => {
    resetHealthReadForTest();
  });
  afterEach(() => {
    resetTurnstileForTests();
    resetHealthReadForTest();
  });

  it("over the stream: the widget once at the stream's failure, one call, then the stream reopens on the ladder (F battery 25)", async () => {
    const ends: End[] = [];
    // The clearance lapsed: the edge challenges every request until the
    // widget has passed, and lets every one through after.
    let cleared = false;
    const handler = vi.fn(() => {
      cleared = true;
      return Promise.resolve();
    });
    setChallengeHandler(handler);
    const stream = fakeStream((open) => [["signal", at(open)]]);
    const stub = stubApi((call) => {
      if (!cleared) return challenged();
      if (call.path.endsWith("/stream")) return stream.reply(call);
      if (call.path === "/api/health") return wire.health;
      return undefined;
    });
    const seen: Events[] = [];
    const { result } = renderHook(() => useSignal("wire", true, (e) => seen.push(e), (e) => ends.push(e)));
    await until(() => handler.mock.calls.length > 0, "the widget was asked for");
    await settle();
    // The open the edge challenged, and ONE call: the health read it
    // challenged too, and that read's one retry.
    expect(handler).toHaveBeenCalledTimes(1);
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    expect(stub.matching("GET", "/api/health")).toHaveLength(2);
    // Not ended: a 403 that was the edge's is not the table refusing.
    expect(ends).toEqual([]);
    // The ladder's first rung, and the stream reopens under the clearance.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]!);
    });
    await until(() => stream.opens() > 0, "the stream reopened");
    await settle();
    expect(result.current).toBe("live");
    expect(seen.map((e) => e.events)).toEqual([1]);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(ends).toEqual([]);
  });

  it("over a WebSocket: the probe's call shows the widget once, hands its answer over, and the socket reopens on the ladder", async () => {
    setSignalKind("ws");
    FakeSocket.made = [];
    vi.stubGlobal("WebSocket", FakeSocket);
    let cleared = false;
    const handler = vi.fn(() => {
      cleared = true;
      return Promise.resolve();
    });
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (!cleared) return challenged();
      return call.path.includes("/events") ? at(5) : undefined;
    });
    const seen: Events[] = [];
    const ends: End[] = [];
    renderHook(() => useSignal("wire", true, (e) => seen.push(e), (e) => ends.push(e)));
    await until(() => FakeSocket.made.length === 1, "the socket was opened");
    // What a challenged upgrade is from inside a page: `error`, then 1006.
    act(() => FakeSocket.made[0]!.drop());
    await until(() => seen.length === 1, "the probe's answer was handed over");
    expect(handler).toHaveBeenCalledTimes(1);
    // The probe is the one call: challenged, then its one retry.
    expect(stub.matching("GET", "/events")).toHaveLength(2);
    expect(seen.map((e) => e.events)).toEqual([5]);
    expect(ends).toEqual([]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]!);
    });
    await until(() => FakeSocket.made.length === 2, "the socket reopened");
    act(() => {
      FakeSocket.made[1]!.accept();
      FakeSocket.made[1]!.frame("signal", at(6));
    });
    expect(seen.map((e) => e.events)).toEqual([5, 6]);
    expect(handler).toHaveBeenCalledTimes(1);
  });
});

describe("the transport is known before anything opens (M6.5c live tier)", () => {
  /** The lobby's house channel: a `"wake"` source on `/api/stream`. */
  const house: Source<null> = { key: "lobby", once: async () => null, stream: "/api/stream" };

  beforeEach(() => {
    // Nothing this page has been told: the facts at their defaults, with no
    // answer taken — and no health read out, which `resetFactsForTest` cannot
    // reach (a cell below leaves one hanging for ever).
    resetFactsForTest();
    resetHealthReadForTest();
    FakeSocket.made = [];
    vi.stubGlobal("WebSocket", FakeSocket);
  });
  afterEach(() => {
    resetFactsForTest();
    resetHealthReadForTest();
  });

  it("waits for the health read in flight, and opens the socket it names — never the stream first", async () => {
    const health = gate<Reply>();
    const stub = stubApi((call) => (call.path.endsWith("/api/health") ? health.promise : undefined));
    // App's own read on entering the multiplayer channel, still out as the
    // lobby mounts.
    const asked = api.health();
    renderHook(() => usePolled(house, true, () => {}));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(stub.matching("GET", "/api/stream"), "no stream on the default guess").toEqual([]);
    expect(FakeSocket.made, "and no socket either").toEqual([]);

    health.resolve({ ...wire.health, signal: "ws" });
    await asked;
    await until(() => FakeSocket.made.length === 1, "the socket the health read named");
    expect(new URL(FakeSocket.made[0]!.url).pathname).toBe("/api/stream");
    expect(stub.matching("GET", "/api/stream"), "never a stream at all").toEqual([]);
  });

  it("after a failed health read, climbs the ladder and asks again rather than guess", async () => {
    let healths = 0;
    const stub = stubApi((call) => {
      if (!call.path.endsWith("/api/health")) return undefined;
      healths += 1;
      return healths === 1 ? { status: 500, body: wire.error } : { ...wire.health, signal: "ws" };
    });
    // App's read, which FAILED before the lobby mounted.
    await api.health().catch(() => {});
    const { result } = renderHook(() => usePolled(house, true, () => {}));
    await settle();
    expect(result.current, "the word says so").toBe("connecting");
    // The first attempt asks nothing of its own: the entry read is App's.
    expect(healths).toBe(1);
    expect(stub.matching("GET", "/api/stream")).toEqual([]);
    expect(FakeSocket.made).toEqual([]);

    // The ladder's first rung asks the health read itself.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]);
    });
    await until(() => FakeSocket.made.length === 1, "the socket, once health answered");
    expect(healths).toBe(2);
    expect(stub.matching("GET", "/api/stream"), "never guessed").toEqual([]);
  });

  // **Ruling I-86 (the whole-branch review's H-2): the wait has a BOUND.** A
  // health read that never settles — a server that accepts the connection and
  // then says nothing, the case App's own bounded read exists for (R-6.5b.2) —
  // left every source waiting on it for ever, with no stream opened and the
  // word reading "live" over no connection at all. On the NAS the default
  // stream was right all along, and the stream opened at once before the wait
  // existed (measured by the review: 1 stream in 120 s before it, 0 after).
  // So the wait is App's own `BOUND_MS`, measured from the read's start, and
  // after it the transport this page HAS is used — the remembered one, or the
  // default.
  it("waits for a health read that never settles only as long as the bound, saying connecting and never live, then opens the transport it has (the NAS)", async () => {
    const stream = fakeStream();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/health")) return new Promise<Reply>(() => {});
      return call.path.endsWith("/api/stream") ? stream.reply(call) : undefined;
    });
    // App's read on entering the multiplayer channel — the one that hangs.
    void api.health();
    const { result } = renderHook(() => usePolled(house, true, () => {}));
    await settle();
    // **Not "live" while nothing is connected** (spec §5.1): the transport is
    // being asked, and the word is the one a connection that is not there yet
    // reads.
    expect(result.current, "the word while the transport is being asked").toBe("connecting");
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOUND_MS - 100);
    });
    expect(stub.matching("GET", "/api/stream"), "no stream before the bound").toEqual([]);
    expect(result.current).toBe("connecting");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(100);
    });
    await until(() => stream.opens() === 1, "the default transport's stream, once the bound had passed");
    await settle();
    expect(result.current, "connected, and only now live").toBe("live");
    expect(FakeSocket.made).toEqual([]);
    // Nothing failed, so nothing asks health again.
    expect(stub.matching("GET", "/api/health")).toHaveLength(1);
  });

  // **The same bound on the Worker, and the price the ruling accepted**: its
  // stream route answers a plain GET 426, so a hung health costs ONE refused
  // stream per rung — and each retry asks health AGAIN rather than re-await a
  // read already older than the bound, so the first answer that does land
  // picks the socket.
  it("on the Worker, a hung health read costs one 426, and the retry asks health anew rather than re-await the stale read", async () => {
    let healths = 0;
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/health")) {
        healths += 1;
        // App's read never settles; the retry's answers.
        return healths === 1 ? new Promise<Reply>(() => {}) : { ...wire.health, signal: "ws" };
      }
      // A plain GET on a socket route.
      if (call.path.endsWith("/api/stream")) return { status: 426, body: wire.error };
      return undefined;
    });
    void api.health();
    const { result } = renderHook(() => usePolled(house, true, () => {}));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOUND_MS);
    });
    await until(() => stub.matching("GET", "/api/stream").length === 1, "the default transport, once the bound had passed");
    await settle();
    expect(result.current).toBe("connecting");
    expect(healths, "nothing re-asked before the ladder's first rung").toBe(1);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]);
    });
    await until(() => FakeSocket.made.length === 1, "the socket the NEW health read named");
    expect(healths, "a new read, not the stale one awaited again").toBe(2);
    expect(new URL(FakeSocket.made[0]!.url).pathname).toBe("/api/stream");
    expect(stub.matching("GET", "/api/stream"), "one 426, and the stream never again").toHaveLength(1);
  });

  // **The bound is the READ's, not the screen's**: App asks on entering the
  // channel, and a screen that mounts later — behind a door that answered
  // slowly — waits only what is left of that read's bound. One that mounts
  // after it waits for nothing: the read has had its bound, and a first
  // attempt does not ask again (only a retry does, above).
  it("measures the bound from the read's asking: a screen mounted late waits only what is left of it, and one mounted past it waits for nothing", async () => {
    const stream = fakeStream();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/health")) return new Promise<Reply>(() => {});
      return call.path.endsWith("/stream") ? stream.reply(call) : undefined;
    });
    void api.health();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3_000);
    });
    renderHook(() => usePolled(house, true, () => {}));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOUND_MS - 3_000 - 100);
    });
    expect(stub.matching("GET", "/api/stream"), "not before the read's bound").toEqual([]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(200);
    });
    await until(() => stream.opens() === 1, "the stream, at the read's bound and not five seconds after the mount");

    renderHook(() =>
      usePolled<null>({ key: "game", once: async () => null, stream: "/api/games/wire/stream" }, true, () => {}),
    );
    await until(() => stream.opens() === 2, "the second screen's stream, at once");
    expect(stub.matching("GET", "/api/health"), "and nothing asked again").toHaveLength(1);
  });

  // **An answer that lands AFTER the bound is still an answer** — written down
  // when it lands, so the next open reads it. Without that, a read the wait
  // gave up on would leave nothing out and nothing known, and every later open
  // would take the default for ever. No `App` is mounted here, so nothing else
  // is listening for this read.
  it("writes down an answer that lands after the bound, so the next open reads it", async () => {
    const health = gate<Reply>();
    const stream = fakeStream();
    stubApi((call) => {
      if (call.path.endsWith("/api/health")) return health.promise;
      return call.path.endsWith("/api/stream") ? stream.reply(call) : undefined;
    });
    const asked = api.health();
    renderHook(() => usePolled(house, true, () => {}));
    await settle();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BOUND_MS);
    });
    await until(() => stream.opens() === 1, "the default transport's stream, past the bound");

    health.resolve({ ...wire.health, signal: "ws" });
    await asked;
    await settle();
    expect(signalKind(), "the late answer is written down").toBe("ws");
    // The stream drops, and the ladder's first rung opens what the answer
    // named — a socket, not the stream again.
    stream.close();
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]);
    });
    await until(() => FakeSocket.made.length === 1, "the socket the late answer named");
    expect(stream.opens()).toBe(1);
  });

  // **The check AFTER the wait** (Task 15's re-review, R2-1): a tab that hid
  // while the transport was being asked has let its attempt go, and the answer
  // landing must not open anything for it (R-P8). Until R-D5 the answer named
  // the held arm here, whose first move was a request no abort reaches. Both
  // arms left honour the abort themselves — `readSocket` returns before it
  // makes a socket, and a real `fetch` sends nothing on an aborted signal — so
  // the check is now the first of two guards. The answer names the STREAM,
  // whose aborted `fetch` the harness still RECORDS, which is what lets this
  // cell see the check go.
  it("asks nothing for a tab that hid while the transport was being asked, and asks once it is back", async () => {
    const health = gate<Reply>();
    const stub = stubApi((call) => (call.path.endsWith("/api/health") ? health.promise : undefined));
    const asked = api.health();
    renderHook(() => usePolled<null>(house, true, () => {}));
    await settle();
    act(() => visibility("hidden"));
    expect(wire.health.signal, "premise: the answer names the stream").toBe("sse");
    health.resolve(wire.health);
    await asked;
    await settle();
    expect(stub.matching("GET", "/api/stream"), "nothing asked from a hidden tab (R-P8)").toEqual([]);
    expect(FakeSocket.made).toEqual([]);

    act(() => visibility("visible"));
    await until(() => stub.matching("GET", "/api/stream").length === 1, "the stream's open, once the tab is back");
  });
});
