// The reader's two halves: a grammar, and a request.
//
// **The grammar is tested on CHUNKS and not on frames**, because a chunk is
// what a socket delivers and a frame is what the server meant. The split is
// arbitrary — a `data:` line can arrive in three pieces with the newline in the
// fourth — so a parser tested only on whole frames is a parser tested on the
// one case the network does not guarantee.

import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiError, CLAIMED_USER_KEY } from "./api";
import { parseSse, readStream, type Frame } from "./sse";
import { fakeStream, stubApi } from "./test/harness";
import { EDGE_CHECK, EDGE_CHECK_REPEATED, resetTurnstileForTests, setChallengeHandler } from "./turnstile";
import { wire } from "./wire/contract";

describe("the event-stream grammar", () => {
  it("yields a frame whole however the chunks fall across it", () => {
    const parse = parseSse();
    // Split mid-WORD in the event name, mid-value in the data line, and again
    // between a frame's last newline and the blank line that ends it.
    expect(parse("eve")).toEqual([]);
    expect(parse('nt: signal\ndata: {"eve')).toEqual([]);
    expect(parse('nts":3}\n')).toEqual([]);
    expect(parse("\nevent: signal\n")).toEqual([{ event: "signal", data: '{"events":3}' }]);
    expect(parse('data: {"events":4}\n\nevent: end\ndata: {"reason":"deleted"}\n\n')).toEqual([
      { event: "signal", data: '{"events":4}' },
      { event: "end", data: '{"reason":"deleted"}' },
    ]);
  });

  it("drops the heartbeat, and a heartbeat between two frames does not join them", () => {
    const parse = parseSse();
    // The server's keep-alive is a bare `:` comment line every 25 s. It is not
    // a frame and it does not end one: a parser treating it as either would
    // either emit an empty frame every 25 s or lose the frame it interrupts.
    expect(parse(":\n\n")).toEqual([]);
    expect(parse('event: signal\n:\ndata: {"events":1}\n\n')).toEqual([
      { event: "signal", data: '{"events":1}' },
    ]);
  });

  it("joins a multi-line data payload with a newline and defaults the event name", () => {
    // This server writes one `data:` line per frame, so the join is the SPEC's
    // rule rather than an observed one — written down because a parser that
    // dropped all but the last line would pass every cell above.
    const parse = parseSse();
    expect(parse("data: one\ndata: two\n\n")).toEqual([{ event: "message", data: "one\ntwo" }]);
  });
});

describe("the reader", () => {
  const collect = () => {
    const frames: Frame[] = [];
    return { frames, onFrame: (f: Frame) => frames.push(f) };
  };

  it("reads the frames in order and stops on the end frame", async () => {
    const stream = fakeStream();
    stubApi((call) => (call.path.endsWith("/stream") ? stream.reply(call) : undefined));
    const { frames, onFrame } = collect();
    const reading = readStream("/api/games/wire/stream", new AbortController().signal, onFrame);
    stream.push("signal", { events: 1 });
    stream.push("signal", { events: 2 });
    stream.push("end", { reason: "deleted" });
    // **The `end` frame is handed over BEFORE the read resolves**, and that is
    // the contract the hook depends on: `onEnd`'s reason is carried on that
    // frame's own body, so a reader that swallowed it would leave the caller
    // knowing only that the stream had stopped.
    await expect(reading).resolves.toBe("ended");
    expect(frames.map((f) => f.event)).toEqual(["signal", "signal", "end"]);
    expect(JSON.parse(frames[1]!.data)).toEqual({ events: 2 });
  });

  it("calls a body that ends without an end frame CLOSED", async () => {
    const stream = fakeStream();
    stubApi((call) => (call.path.endsWith("/stream") ? stream.reply(call) : undefined));
    const { frames, onFrame } = collect();
    const reading = readStream("/api/games/wire/stream", new AbortController().signal, onFrame);
    stream.push("signal", { events: 1 });
    stream.close();
    // A dropped connection and an ended stream are two different answers, which
    // is why this resolves rather than throwing: only the caller knows that one
    // is worth reopening and the other is not.
    await expect(reading).resolves.toBe("closed");
    expect(frames).toHaveLength(1);
  });

  it("throws the server's own sentence and status when the open is refused", async () => {
    stubApi(() => ({ status: 403, body: { error: "this table is locked — unlock it first" } }));
    const err = await readStream(
      "/api/games/wire/stream",
      new AbortController().signal,
      () => {},
    ).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect((err as ApiError).status).toBe(403);
    expect((err as ApiError).message).toBe("this table is locked — unlock it first");
  });

  it("falls back to the status when the refusal is not this API's JSON", async () => {
    // The documented deployment sits behind Cloudflare Access, which answers an
    // expired session with an HTML interstitial: `call` already says so rather
    // than reporting a parser's complaint, and the reader must not be the one
    // place that regresses to `Unexpected token '<'`.
    stubApi(() => new Response("<html>nope</html>", { status: 502 }));
    const err = await readStream(
      "/api/games/wire/stream",
      new AbortController().signal,
      () => {},
    ).catch((e: unknown) => e);
    expect((err as ApiError).status).toBe(502);
    expect((err as ApiError).message).toContain("502");
  });

  it("reports the OPEN once, after the 200 and before any frame", async () => {
    // **What the caller needs and could not otherwise know.** A `fetch` that
    // has been issued is not a connection: the reader's own `"live"` word hangs
    // on the 200 arriving, and a source that receives no frame on connect (the
    // house channel) has nothing else to learn it from.
    const stream = fakeStream(() => [["signal", { events: 1 }]]);
    stubApi((call) => stream.reply(call));
    const order: string[] = [];
    const reading = readStream(
      "/api/stream",
      new AbortController().signal,
      () => order.push("frame"),
      () => order.push("open"),
    );
    stream.close();
    await expect(reading).resolves.toBe("closed");
    // Once, and FIRST: a frame cannot arrive before the body it rides on.
    expect(order).toEqual(["open", "frame"]);
  });

  it("does not report an open the server refused", async () => {
    // Anti-vacuity for the cell above, and the property the link word rests on:
    // a refusal is not a connection, so nothing may clear "reconnecting…".
    stubApi(() => ({ status: 403, body: { error: "not yours" } }));
    const opens: number[] = [];
    await readStream(
      "/api/stream",
      new AbortController().signal,
      () => {},
      () => opens.push(1),
    ).catch(() => {});
    expect(opens).toEqual([]);
  });

  it("signs the request and asks for an event stream", async () => {
    // **The whole reason this is a `fetch` and not an `EventSource`**: the
    // `open` door is a header, and `EventSource` cannot carry one.
    localStorage.setItem(CLAIMED_USER_KEY, "red@wire.test");
    const stream = fakeStream();
    const stub = stubApi((call) => stream.reply(call));
    const controller = new AbortController();
    const reading = readStream("/api/games/wire/stream", controller.signal, () => {});
    const opened = stub.calls[0]!;
    expect(opened.headers["x-yokohama-claimed-user"]).toBe("red@wire.test");
    expect(opened.headers.accept).toBe("text/event-stream");
    // And the caller's abort is the one the request carries, which is what lets
    // a hidden tab stop reading.
    controller.abort();
    expect(opened.signal!.aborted).toBe(true);
    await expect(reading).rejects.toThrow();
  });

  it("sends no identity header when this browser is signed out", async () => {
    const stream = fakeStream();
    const stub = stubApi((call) => stream.reply(call));
    void readStream("/api/games/wire/stream", new AbortController().signal, () => {});
    expect(stub.calls[0]!.headers["x-yokohama-claimed-user"]).toBeUndefined();
  });
});

// ---- cycle F: a stream the EDGE challenges (R-F6, R-FP8; spec §2.6) ---------
//
// **A watcher who makes no other call is not stranded** (Review Focus 4). The
// stream is the one request a watcher holds, so when the clearance lapses it is
// the stream's open the edge challenges — and a stream cannot show a widget.
// The reader makes ONE `call` (`GET /api/health`, interactive), which is where
// the widget is shown and the retry made, and then fails into the ladder, which
// reopens the stream under the clearance (`signal.test.ts` holds the reopen).
// A second challenge on that one call is the refusal it is: the source ends.
describe("the reader, under the edge's challenge", () => {
  const challenged = () =>
    new Response("<html>challenge</html>", {
      status: 403,
      headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
    });

  afterEach(() => {
    resetTurnstileForTests();
  });

  it("makes ONE call to pass the check, and then fails into the ladder rather than ending (Review Focus 4)", async () => {
    const handler = vi.fn(() => Promise.resolve());
    setChallengeHandler(handler);
    let healths = 0;
    const stub = stubApi((call) => {
      if (call.path.endsWith("/stream")) return challenged();
      if (call.path === "/api/health") {
        healths += 1;
        return healths === 1 ? challenged() : wire.health;
      }
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const failed = await readStream("/api/games/wire/stream", new AbortController().signal, () => {}).catch(
      (e: unknown) => e,
    );
    // Not a refusal — a 403 would END the source — but a failure, which the
    // ladder is for.
    expect(failed).toBeInstanceOf(Error);
    expect(failed).not.toBeInstanceOf(ApiError);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    // ONE call: the health read the edge challenged, and its one retry.
    expect(stub.matching("GET", "/api/health")).toHaveLength(2);
  });

  it("ends on the refusal when that one call is challenged again", async () => {
    setChallengeHandler(() => Promise.resolve());
    const stub = stubApi(() => challenged());
    const refused = await readStream("/api/games/wire/stream", new AbortController().signal, () => {}).catch(
      (e: unknown) => e,
    );
    expect(refused).toBeInstanceOf(ApiError);
    expect((refused as ApiError).status).toBe(403);
    expect((refused as ApiError).message).toBe(EDGE_CHECK_REPEATED);
    expect(stub.calls).toHaveLength(3);
  });

  it("ends on EDGE_CHECK where no widget can be shown", async () => {
    stubApi(() => challenged());
    const refused = await readStream("/api/games/wire/stream", new AbortController().signal, () => {}).catch(
      (e: unknown) => e,
    );
    expect((refused as ApiError).status).toBe(403);
    expect((refused as ApiError).message).toBe(EDGE_CHECK);
  });
});
