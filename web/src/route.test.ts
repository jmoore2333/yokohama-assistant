// The hash is the screen (spec §5.1).
//
// **Three routes, and everything else is the lobby.** A stale link, a mistyped
// fragment and an empty hash all land somewhere a player can act from, which is
// the whole reason `parseRoute` is total rather than returning `null` for a
// caller to decide about.

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { hashOf, parseRoute, useRoute } from "./route";

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

  // **The form has two addresses since M6.5b** (spec §2.5): `#/new` opens a
  // table on the server and `#/new/local` opens one in this browser, and the
  // second is reachable with no door at all — which is the whole of what makes
  // "Play the bot here" work on an unreachable API.
  it("reads #/new/local as the local create and round-trips it", () => {
    expect(parseRoute("#/new/local")).toEqual({ kind: "new", local: true });
    expect(hashOf({ kind: "new", local: true })).toBe("#/new/local");
    expect(parseRoute(hashOf({ kind: "new", local: true }))).toEqual({ kind: "new", local: true });
    // The server's create is untouched, and carries no flag rather than a false
    // one: `{kind: "new"}` is what every existing caller builds.
    expect(hashOf({ kind: "new" })).toBe("#/new");
    expect(parseRoute("#/new")).toEqual({ kind: "new" });
    // Anything else under `/new` is the lobby, like every other unrecognized
    // hash — `#/new/server` is not a third form.
    expect(parseRoute("#/new/server")).toEqual({ kind: "lobby" });
  });

  // **The two creates are two SCREENS, which `useRoute` has to notice.** The
  // hook hands back the same object for the same route so consumers' effects do
  // not re-run on a screen that has not moved — and a sameness test that read
  // the `kind` alone would call `#/new` and `#/new/local` one screen, leaving a
  // player who asked for the local form looking at the server's.
  it("hands back a NEW route object when the create changes host", () => {
    window.location.hash = "#/new";
    const { result } = renderHook(() => useRoute());
    expect(result.current).toEqual({ kind: "new" });
    act(() => {
      window.location.hash = "#/new/local";
      window.dispatchEvent(new HashChangeEvent("hashchange"));
    });
    expect(result.current).toEqual({ kind: "new", local: true });
  });
});
