// The hash is the screen (spec §5.1).
//
// **Three routes, and everything else is the lobby.** A stale link, a mistyped
// fragment and an empty hash all land somewhere a player can act from, which is
// the whole reason `parseRoute` is total rather than returning `null` for a
// caller to decide about.

import { describe, expect, it } from "vitest";
import { hashOf, parseRoute } from "./route";

describe("the hash routes", () => {
  it("parses the three routes and reads anything else as the lobby", () => {
    expect(parseRoute("")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/new")).toEqual({ kind: "new" });
    expect(parseRoute("#/t/sim-0123")).toEqual({ kind: "table", id: "sim-0123" });
    expect(parseRoute("#/t/")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/nope")).toEqual({ kind: "lobby" });
    // **Decoded, because `hashOf` encodes.** A table id is server-drawn and
    // today never holds a space; a hand-typed link is not bound by that, and a
    // route that handed the encoded form to `GET …/table` would ask for a table
    // whose id contains a literal `%20`.
    expect(parseRoute("#/t/a%20b")).toEqual({ kind: "table", id: "a b" });
  });

  it("round-trips through hashOf", () => {
    for (const r of [{ kind: "lobby" }, { kind: "new" }, { kind: "table", id: "x-1" }] as const) {
      expect(parseRoute(hashOf(r))).toEqual(r);
    }
    // The encoding really happens, so the round trip above is not two identity
    // functions agreeing with each other.
    expect(hashOf({ kind: "table", id: "a b" })).toBe("#/t/a%20b");
  });
});
