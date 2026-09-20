// `api.recordText` (M6.5a, R-6.5a.13): the one entry beside `call` that reads a
// body as TEXT and never parses it — because `GameRecord.seed` is a u64 and
// `JSON.parse` would round it. Its refusals are `call`'s: the server's sentence,
// the status, and the door on a 401.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api, ApiError, onUnauthorized } from "./api";
import { resetLocalHostForTests } from "./local/client";
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
    stubWorker({ read: (doc, id) => (doc === "record" && id === "local-1" ? record : null) });
    const parse = vi.spyOn(JSON, "parse");
    try {
      expect(await api.recordText("local-1")).toBe(record);
      expect(currentWorker().posted[0]).toMatchObject({ kind: "read", doc: "record", id: "local-1" });
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
    expect(state.warnings).toEqual(["APPLIED BUT NOT SAVED — the quota is full"]);
    // The corpus fixture itself carries none, so the line came from here.
    expect(wire.options.warnings).toEqual([]);
  });
});
