// The door, as a hook and a sentence.
//
// Both halves are about ROUTING: `useMe` is the one read that decides which of
// four screens the app is on, and `doorSentence` is what it says when the
// answer is "nobody" under a door this browser cannot knock on.

import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ACCESS_CHALLENGE, ApiError, CLAIMED_USER_KEY } from "./api";
import type { Channel } from "./route";
import { gate, signedOutReply, stubApi, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import { doorSentence, failureOf, useMe } from "./whoami";

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
  // **The hook takes the CHANNEL since M6.5b2** (R-6.5b2.7, spec §2.2). Every
  // cell here is about the door the SERVER owns, so they all run on the
  // multiplayer channel; the cell below this describe is the one that reads the
  // other arm — the front door, where `/api/me` is not asked at all.

  it("reads /api/me once at mount and hands back what it said", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = meFromStorage();
    const { result } = renderHook(() => useMe("multiplayer"));
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
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    localStorage.removeItem(CLAIMED_USER_KEY);
    act(() => result.current.refresh());
    await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
    expect(stub.matching("GET", "/api/me")).toHaveLength(2);
  });

  // **The sentence survives the screen change**, which is the whole point of
  // holding it here rather than in whatever component caught the refusal: a 401
  // unmounts that component, and a player who arrives at a bare form has been
  // told nothing about why their game vanished.
  it("carries the refusal's sentence to the door, and drops the name with it", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    meFromStorage();
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    expect(result.current.notice).toBeNull();

    act(() => result.current.signedOut("no signed-in user: send X-Yokohama-Claimed-User"));
    await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
    expect(result.current.notice).toBe("no signed-in user: send X-Yokohama-Claimed-User");
    // Dropped, not merely re-asked: `/api/me` reads the very header storage
    // supplies, so a name kept here comes straight back.
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });

  it("says nothing when the player signs out on purpose", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    meFromStorage();
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    act(() => result.current.signedOut("stale sentence from an earlier refusal"));
    await waitFor(() => expect(result.current.notice).not.toBeNull());

    // Signing in again and then out by hand must not re-show the old refusal:
    // a notice left standing would explain a screen the player asked for.
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    act(() => result.current.refresh());
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    act(() => result.current.signOut());
    await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
    expect(result.current.notice).toBeNull();
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
  });

  // **A 401 that lands while a deliberate sign-out re-reads says nothing**
  // (dossier row 3). `signOut` drops the stored name and asks `/api/me` again,
  // but `me` goes on naming the old player until that answer lands — so a
  // request a screen sends in that window goes out with no name, is refused
  // 401, and `App` hands its sentence to `signedOut`. The player asked to
  // leave; the name step must not greet them with the refusal leaving caused.
  // The second half is the guard's other edge: once the server names somebody
  // again, a 401 is news once more and carries its sentence.
  it("says nothing about a 401 that lands while a deliberate sign-out is re-reading", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    let held: { promise: Promise<Reply>; resolve: (v: Reply) => void } | null = null;
    stubApi((call) => {
      if (!call.path.endsWith("/api/me")) return undefined;
      if (held) return held.promise;
      return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
    });
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));

    const reread = gate<Reply>();
    held = reread;
    act(() => result.current.signOut());
    // The re-read is out, and a screen's request, sent with no name, is refused.
    const sentence = signedOutReply().body.error;
    act(() => result.current.signedOut(sentence));
    held = null;
    reread.resolve(wire.meSignedOut);
    await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
    expect(result.current.notice, "the sign-out was asked for").toBeNull();

    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    act(() => result.current.refresh());
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    act(() => result.current.signedOut(sentence));
    await waitFor(() => expect(result.current.notice, "signed in again, a 401 is news").toBe(sentence));
  });

  it("carries the server's own sentence when the door refuses", async () => {
    stubApi((call) =>
      call.path.endsWith("/api/me") ? { status: 500, body: wire.error } : undefined,
    );
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.error).toBe(wire.error.error));
    expect(result.current.failure).toBe("refused");
    expect(result.current.me).toBeNull();
  });

  // **How the door failed decides which card is drawn** (M6.5c2, spec §2.1): no
  // answer at all is state C; the EDGE's challenge is state A.
  it("says HOW the door failed: no answer at all, and the edge's challenge", async () => {
    stubApi(() => {
      throw new TypeError("Failed to fetch");
    });
    const offline = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(offline.result.current.failure).toBe("network"));
    expect(offline.result.current.error).toBe("TypeError: Failed to fetch");
    offline.unmount();

    stubApi((call) =>
      call.path.endsWith("/api/me")
        ? new Response("<html>", {
            status: 401,
            headers: { "www-authenticate": 'Cloudflare-Access resource_metadata="https://x/.well-known/y"' },
          })
        : undefined,
    );
    const expired = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(expired.result.current.failure).toBe("challenge"));
    expect(expired.result.current.error).toBe(ACCESS_CHALLENGE);
  });

  // **The LOCAL channel asks nothing** (R-6.5b2.7, spec §2.2): the front door
  // draws the games in this browser and a link to the turnstile, and `/api/me`
  // is a protected path now — read there it would be a 401 on every visit.
  // `me` stays at its third state, which is exactly what "nothing has been
  // asked" should look like.
  it("asks nothing at all on the local channel, and asks once on entering the other", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = meFromStorage();
    const { result, rerender } = renderHook(({ c }: { c: Channel }) => useMe(c), {
      initialProps: { c: "local" as Channel },
    });
    await waitFor(() => expect(stub.calls.length).toBe(0));
    expect(result.current.me).toBeNull();
    expect(result.current.error).toBeNull();

    rerender({ c: "multiplayer" });
    await waitFor(() => expect(result.current.me).toEqual(wire.me));
    expect(stub.matching("GET", "/api/me")).toHaveLength(1);

    // And going BACK to the door keeps what the server said rather than asking
    // again: nothing about the door makes the answer stale.
    rerender({ c: "local" });
    await waitFor(() => expect(result.current.me).toEqual(wire.me));
    expect(stub.matching("GET", "/api/me")).toHaveLength(1);
  });
});

describe("failureOf", () => {
  // An `ApiError` is an ANSWER — the server's or the edge's — and anything else
  // is `fetch` failing with none (`api.ts`'s `call` turns every status into one).
  it("tells the edge's challenge from the server's refusal from no answer at all", () => {
    expect(failureOf(new ApiError(ACCESS_CHALLENGE, 401))).toBe("challenge");
    expect(failureOf(new ApiError("the store could not be read", 500))).toBe("refused");
    // This API's OWN 401 is a refusal, not the edge's challenge.
    expect(failureOf(new ApiError("no identity reached the server", 401))).toBe("refused");
    expect(failureOf(new TypeError("Failed to fetch"))).toBe("network");
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
