// `api.recordText` (M6.5a, R-6.5a.13): the one entry beside `call` that reads a
// body as TEXT and never parses it — because `GameRecord.seed` is a u64 and
// `JSON.parse` would round it. Its refusals are `call`'s: the server's sentence,
// the status, and the door on a 401.

import { afterEach, describe, expect, it, vi } from "vitest";
import { api, ApiError, onUnauthorized } from "./api";
import { stubApi } from "./test/harness";

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
