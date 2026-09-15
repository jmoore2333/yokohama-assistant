// The door, as a hook and a sentence.
//
// Both halves are about ROUTING: `useMe` is the one read that decides which of
// four screens the app is on, and `doorSentence` is what it says when the
// answer is "nobody" under a door this browser cannot knock on.

import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { CLAIMED_USER_KEY } from "./api";
import { stubApi } from "./test/harness";
import { wire } from "./wire/contract";
import { doorSentence, useMe } from "./whoami";

/** `/api/me` answered the way the SERVER answers it: from the header this
 *  browser is sending, which `api.ts` attaches from storage. A stub that
 *  answered `wire.me` regardless would model a server that ignores its own
 *  header, and nothing about signing out could then be tested. */
const meFromStorage = () =>
  stubApi((call) =>
    call.path.endsWith("/api/me")
      ? localStorage.getItem(CLAIMED_USER_KEY)
        ? wire.me
        : wire.meSignedOut
      : undefined,
  );

describe("useMe", () => {
  it("reads /api/me once at mount and hands back what it said", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = meFromStorage();
    const { result } = renderHook(() => useMe());
    // `null` until it answers is the whole reason the app draws nothing at
    // first: a default of "signed out" would flash the sign-in screen at every
    // player who is already signed in.
    expect(result.current.me).toBeNull();
    await waitFor(() => expect(result.current.me).toEqual(wire.me));
    expect(result.current.error).toBeNull();
    expect(stub.matching("GET", "/api/me")).toHaveLength(1);
  });

  it("re-reads it on refresh, which is how signing out reaches the screen", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = meFromStorage();
    const { result } = renderHook(() => useMe());
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    localStorage.removeItem(CLAIMED_USER_KEY);
    act(() => result.current.refresh());
    await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
    expect(stub.matching("GET", "/api/me")).toHaveLength(2);
  });

  it("carries the server's own sentence when the door refuses", async () => {
    stubApi((call) =>
      call.path.endsWith("/api/me") ? { status: 500, body: wire.error } : undefined,
    );
    const { result } = renderHook(() => useMe());
    await waitFor(() => expect(result.current.error).toBe(wire.error.error));
    expect(result.current.me).toBeNull();
  });
});

describe("doorSentence", () => {
  it("names the header the deployment's own edge was supposed to set", () => {
    // The two sentences differ in the one fact a player can act on: WHICH
    // header is missing, and therefore which way in they should have come.
    expect(doorSentence("cf-access")).toMatch(/Cf-Access-Authenticated-User-Email/);
    expect(doorSentence("worker")).toMatch(/X-Yokohama-User/);
    expect(doorSentence("cf-access")).not.toBe(doorSentence("worker"));
  });
});
