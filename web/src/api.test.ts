// `api.recordText` (M6.5a, R-6.5a.13): the one entry beside `call` that reads a
// body as TEXT and never parses it — because `GameRecord.seed` is a u64 and
// `JSON.parse` would round it. Its refusals are `call`'s: the server's sentence,
// the status, and the door on a 401.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ACCESS_CHALLENGE, api, ApiError, BOUND_MS, bounded, onUnauthorized } from "./api";
import { readStream } from "./sse";
import { resetLocalHostForTests } from "./local/client";
import { TAKEN_OVER_SENTENCE } from "./local/lost";
import { stubApi } from "./test/harness";
import { currentWorker, FakeWorker, stubWorker } from "./test/fakeWorker";
import { wire } from "./wire/contract";

describe("api.recordText", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    onUnauthorized(null);
  });

  it("returns the 200 body as the very string it arrived as, unparsed", async () => {
    const record = '{"seed":18446744073709551615,"events":[],"terminal_digest":"abc"}';
    const parse = vi.spyOn(JSON, "parse");
    const stub = stubApi((call) =>
      call.path.endsWith("/api/games/g-1/record")
        ? new Response(record, { status: 200, headers: { "content-type": "application/json" } })
        : undefined,
    );
    try {
      expect(await api.recordText("g-1")).toBe(record);
      expect(stub.matching("GET", "/api/games/g-1/record")).toHaveLength(1);
      expect(parse.mock.calls.some(([s]) => s === record)).toBe(false);
      // A number that large would not survive a parse — which is the point.
      expect(String(JSON.parse(record).seed)).not.toBe("18446744073709551615");
    } finally {
      parse.mockRestore();
    }
  });

  it("refuses with the server's sentence and status, and reaches the door on a 401", async () => {
    stubApi((call) =>
      call.path.endsWith("/record")
        ? { status: 409, body: { error: "the record is available once the game is over" } }
        : undefined,
    );
    const refused = await api.recordText("g-2").catch((e) => e);
    expect(refused).toBeInstanceOf(ApiError);
    expect((refused as ApiError).status).toBe(409);
    expect((refused as ApiError).message).toBe("the record is available once the game is over");

    const door = vi.fn();
    onUnauthorized(door);
    stubApi(() => ({ status: 401, body: { error: "no signed-in user: send X-Yokohama-Claimed-User" } }));
    const out = await api.recordText("g-3").catch((e) => e);
    expect((out as ApiError).status).toBe(401);
    expect(door).toHaveBeenCalledWith("no signed-in user: send X-Yokohama-Claimed-User");
  });
});

// ---------------------------------------------------------------------------
// M6.5b: which HOST a call goes to (R-6.5b.12).
//
// **One predicate and one place.** `isLocal(id)` is the `local-` prefix, and
// every reader that HAS an id goes through it; the five routes that carry no
// id take the caller's intent instead. The cells below are about which
// transport was reached, so `fetch` is replaced by something that THROWS —
// a call that leaks to the network is a crash, not a soft assertion.

describe("call routes by host", () => {
  const forbidFetch = () =>
    vi.stubGlobal(
      "fetch",
      vi.fn(() => {
        throw new Error("a local call reached the network");
      }),
    );

  beforeEach(() => {
    resetLocalHostForTests();
    stubWorker();
  });
  afterEach(() => {
    resetLocalHostForTests();
    vi.unstubAllGlobals();
  });

  it("sends a LOCAL game's call to the Worker and never to fetch", async () => {
    forbidFetch();
    const state = await api.get("local-1");
    expect(state.id).toBe("local-1");
    expect(currentWorker().posted).toHaveLength(1);
    expect(fetch).not.toHaveBeenCalled();
  });

  it("raises a local write the lock did not keep as a refusal, never as an applied move", async () => {
    // **The page half of §2.2's in-flight rule** (whole-branch review,
    // Important #1), end to end: `localCall` turns a non-GET answered
    // `not-held` into a 409, and `settle` raises it — so `send`'s own catch
    // shows the loss and nothing of that move is drawn. Before the wave this
    // resolved: the module's 200 came back as an applied state with
    // "APPLIED BUT NOT SAVED — the browser refused the write" appended, a
    // sentence about a store that had refused nothing.
    forbidFetch();
    stubWorker({ call: () => ({ status: 200, body: { ok: true }, saved: "not-held" }) });
    const refused = await api.input("local-1", "1", "options").catch((e: unknown) => e);
    expect(refused).toBeInstanceOf(ApiError);
    expect((refused as ApiError).status).toBe(409);
    expect((refused as ApiError).message).toBe(TAKEN_OVER_SENTENCE);
  });

  it("sends every OTHER game's call to fetch and starts no Worker", async () => {
    stubApi((call) =>
      call.path.endsWith("/api/games/sim-1") ? { ...wire.options, id: "sim-1" } : undefined,
    );
    expect((await api.get("sim-1")).id).toBe("sim-1");
    expect(FakeWorker.instances).toHaveLength(0);
  });

  it("routes the five ID-LESS callers by the host they are given", async () => {
    forbidFetch();
    await api.me("local");
    await api.health("local");
    await api.labels("local");
    await api.list("mine", undefined, "local");
    await api.create(
      {
        players: 2,
        colors: ["Red", "Blue"],
        start_seat: 0,
        seats: [{ user: "local@browser" }, { bot: true }],
      },
      "local",
    );
    expect(
      currentWorker().posted.map((p) => (p as { path: string; method: string }).method + " " + (p as { path: string }).path),
    ).toEqual([
      "GET /api/me",
      "GET /api/health",
      "GET /api/labels",
      "GET /api/games",
      "POST /api/games",
    ]);
    expect(fetch).not.toHaveBeenCalled();
  });

  it("reads a LOCAL record off the side channel, as the very string, unparsed", async () => {
    forbidFetch();
    // The record's `seed` is a u64 that `JSON.parse` would round — the same
    // reason the hosted `recordText` never parses either.
    const record = '{"seed":18446744073709551615,"events":[]}';
    stubWorker({
      pair: (id) =>
        id === "local-1" ? { record, table: '{"schema":3}' } : { record: null, table: null },
    });
    const parse = vi.spyOn(JSON, "parse");
    try {
      expect(await api.recordText("local-1")).toBe(record);
      // ONE message for both documents — the export snapshot (spec §2.2).
      expect(currentWorker().posted).toEqual([{ seq: 1, kind: "pair", id: "local-1" }]);
      expect(parse.mock.calls.some(([s]) => s === record)).toBe(false);
    } finally {
      parse.mockRestore();
    }
    // And a game with no record yet — still dealing — is a 404 in words.
    const refused = await api.recordText("local-2").catch((e: unknown) => e);
    expect(refused).toBeInstanceOf(ApiError);
    expect((refused as ApiError).status).toBe(404);
  });

  it("sends a local game to the SERVER's archive, both documents as strings", async () => {
    const stub = stubApi((call) =>
      call.path.endsWith("/api/archive") ? { status: 201, body: wire.archivePost } : undefined,
    );
    const reply = await api.archive('{"seed":1}', '{"schema":3}');
    expect(reply).toEqual(wire.archivePost);
    const [sent] = stub.matching("POST", "/api/archive");
    expect(sent!.body).toEqual({ record: '{"seed":1}', table: '{"schema":3}' });
    // The one call a local game makes over the network — and it starts no
    // Worker, because the caller already holds both strings.
    expect(FakeWorker.instances).toHaveLength(0);
  });

  it("says APPLIED BUT NOT SAVED, in the server's own words, when the browser refused the write", async () => {
    forbidFetch();
    // `api.rs:1105` pushes exactly this shape onto the reply's `warnings`: the
    // move HAPPENED, and the failure is said out loud rather than reported as
    // a refusal. One rule and one wording across both hosts.
    stubWorker({
      call: () => ({
        status: 200,
        body: wire.options,
        saved: "unsaved",
        reason: "the quota is full",
      }),
    });
    const state = await api.input("local-1", "1");
    // **The prefix and the reason verbatim, and one sentence more** (M6.5b2,
    // spec §2.4): the quota failure's path is the backup control, pointed at
    // rather than restated on a screen of its own.
    //
    // **By VALUE, the whole line** (review M-2): a prefix regex plus a
    // `toContain` left the pointer's own wording pinned nowhere, and this line
    // is read by a player at the worst moment they will have — it is worth a
    // test that fails when somebody rewrites it without meaning to.
    expect(state.warnings).toEqual([
      'APPLIED BUT NOT SAVED — the quota is full — back this game up — the "Back up" button on ' +
        "it writes a file this app can restore.",
    ]);
    // The corpus fixture itself carries none, so the line came from here.
    expect(wire.options.warnings).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// M6.5b2: an expired Access session is READ, on all THREE fetch paths (§2.1).
//
// **One header is what makes it readable at all** (spec §0.3, measured against
// the live deployment on 2026-09-21): a request carrying
// `X-Requested-With: XMLHttpRequest` is answered by Cloudflare Access with a
// same-origin `401` and `WWW-Authenticate: Cloudflare-Access …`, which this
// client can read; without it the answer is a `302` to another origin that a
// `fetch` follows and then fails opaquely. `Accept: application/json`,
// `Sec-Fetch-Mode` and `Sec-Fetch-Dest` were each measured NOT to trigger it.
//
// **Three sites and three cells, because one site looks like three.** `call`,
// `recordText` and `readStream` each build their own request: a header added to
// `call` alone would leave a stream reconnect and Save-game-file reporting an
// expired session as an ordinary network failure, and every cell below would
// still be green.

describe("the expired-session read", () => {
  /** What the EDGE answers a signed request whose session has gone: an HTML
   *  interstitial's body, a 401, and the header that names Access. */
  const edge = () =>
    new Response("<html>", {
      status: 401,
      headers: {
        "www-authenticate": 'Cloudflare-Access resource_metadata="https://x/.well-known/y"',
      },
    });

  /** What the SERVER answers when nobody is named: this API's own refusal
   *  shape, and no `WWW-Authenticate` at all. */
  const ownRefusal = () =>
    new Response(JSON.stringify({ error: "no signed-in user" }), {
      status: 401,
      headers: { "content-type": "application/json" },
    });

  const sentHeader = (spy: ReturnType<typeof vi.fn>, n = 0): string | undefined => {
    const init = spy.mock.calls[n]![1] as RequestInit;
    return (init.headers as Record<string, string>)["X-Requested-With"];
  };

  afterEach(() => {
    vi.unstubAllGlobals();
    onUnauthorized(null);
  });

  it("sends X-Requested-With on every server call, and tells the EDGE's 401 from the API's own", async () => {
    const spy = vi.fn().mockResolvedValue(new Response("{}", { status: 200 }));
    vi.stubGlobal("fetch", spy);
    await api.health();
    expect(sentHeader(spy)).toBe("XMLHttpRequest");

    // The two 401s mean different things and must not render the same sentence.
    const door = vi.fn();
    onUnauthorized(door);
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(edge()));
    await expect(api.health()).rejects.toThrow(ACCESS_CHALLENGE);
    // **And the door is NOT moved.** The name this browser holds is fine; it is
    // the edge's cookie that expired, so dropping the claimed user would sign
    // the player out of a deployment that never asked them to.
    expect(door).not.toHaveBeenCalled();

    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(ownRefusal()));
    await expect(api.health()).rejects.toThrow("no signed-in user");
    expect(door).toHaveBeenCalledWith("no signed-in user");
  });

  it("signs the RECORD read, which builds its own request", async () => {
    const spy = vi.fn().mockResolvedValue(new Response('{"seed":1}', { status: 200 }));
    vi.stubGlobal("fetch", spy);
    expect(await api.recordText("sim-1")).toBe('{"seed":1}');
    expect(sentHeader(spy)).toBe("XMLHttpRequest");

    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(edge()));
    await expect(api.recordText("sim-1")).rejects.toThrow(ACCESS_CHALLENGE);
    // The API's own 401 keeps its own words here too.
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(ownRefusal()));
    await expect(api.recordText("sim-1")).rejects.toThrow("no signed-in user");
  });

  it("signs the STREAM, which builds its own request and parses its own refusals", async () => {
    const closed = () =>
      new Response(
        new ReadableStream<Uint8Array>({
          start(c) {
            c.close();
          },
        }),
        { status: 200, headers: { "content-type": "text/event-stream" } },
      );
    const spy = vi.fn().mockResolvedValue(closed());
    vi.stubGlobal("fetch", spy);
    const ac = new AbortController();
    expect(await readStream("/api/games/g/stream", ac.signal, () => {})).toBe("closed");
    expect(sentHeader(spy)).toBe("XMLHttpRequest");

    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(edge()));
    await expect(readStream("/api/games/g/stream", ac.signal, () => {})).rejects.toThrow(
      ACCESS_CHALLENGE,
    );
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(ownRefusal()));
    await expect(readStream("/api/games/g/stream", ac.signal, () => {})).rejects.toThrow(
      "no signed-in user",
    );
  });
});

// ---- M6.5c2: a bound that is over holds no timer (spec §2.8) ---------------
describe("bounded", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  // **A read that answered leaves nothing armed.** The signal's probe is bounded
  // since M6.5c2, inside a hook whose cells COUNT timers — battery 23 asserts
  // none is left once the source has ended — so a bound that outlived its read
  // would read as a rung of the ladder nobody scheduled.
  it("clears its timer the moment the read settles, either way", async () => {
    vi.useFakeTimers();
    await expect(bounded(Promise.resolve("answered"), BOUND_MS)).resolves.toBe("answered");
    expect(vi.getTimerCount()).toBe(0);
    await expect(bounded(Promise.reject(new Error("refused")), BOUND_MS)).rejects.toThrow("refused");
    expect(vi.getTimerCount()).toBe(0);
  });

  it("still rejects once the bound has passed on a read that never settles", async () => {
    vi.useFakeTimers();
    const settled = expect(bounded(new Promise<never>(() => {}), BOUND_MS)).rejects.toThrow("timed out");
    await vi.advanceTimersByTimeAsync(BOUND_MS);
    await settled;
  });
});
