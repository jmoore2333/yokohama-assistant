// The door, as a hook and a sentence.
//
// Both halves are about ROUTING: `useMe` is the one read that decides which of
// four screens the app is on, and `doorSentence` is what it says when the
// answer is "nobody" under a door this browser cannot knock on.

import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ACCESS_CHALLENGE, ApiError, CLAIMED_USER_KEY, DISPLAY_NAME_KEY, SIGN_IN_NEEDED } from "./api";
import type { Channel } from "./route";
import { gate, signedOutReply, stubApi, type Reply } from "./test/harness";
import { wire } from "./wire/contract";
import { doorSentence, failureOf, useMe } from "./whoami";

/** What the OPEN door answers `POST /api/leave`: the house's 404, since no
 *  source signs anyone in there (the auth cycle, Task 4) — which is what sends
 *  `signOut` down its failure path, the sign-out the open door always had. */
const OPEN_DOOR_LEAVE = { status: 404, body: wire.error };

/** `/api/me` answered the way the SERVER answers it: from the header this
 *  browser is sending, which `api.ts` attaches from storage. A stub that
 *  answered `wire.me` regardless would model a server that ignores its own
 *  header, and nothing about signing out could then be tested. And
 *  `/api/leave` as the open door answers it. */
const meFromStorage = () =>
  stubApi((call) =>
    call.path.endsWith("/api/me")
      ? localStorage.getItem(CLAIMED_USER_KEY)
        ? wire.me
        : wire.meSignedOut
      : call.path === "/api/leave"
        ? OPEN_DOOR_LEAVE
        : undefined,
  );

/** `location`, standing in for jsdom's — whose `assign` is not implemented —
 *  and recording every navigation it is asked for (`FrontDoor.test.tsx`'s). */
function countNavigations(): string[] {
  const real = window.location;
  const went: string[] = [];
  vi.stubGlobal(
    "location",
    new Proxy(
      {},
      {
        get(_t, key) {
          if (key === "assign") return (url: string) => went.push(url);
          const value = Reflect.get(real, key, real);
          return typeof value === "function" ? value.bind(real) : value;
        },
      },
    ),
  );
  return went;
}

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
  //
  // **Sequenced for the asynchronous sign-out** (the auth cycle, Task 6 fix
  // round 1, review I-2): the leave is asked first, and only its answer (the
  // open door's 404) drops the name and sends the re-read. A 401 landing while
  // the leave is still out was cleared by that answer whatever the guard did,
  // so the cell waits until the re-read is OUT — the window the guard exists
  // for — before the 401 lands. Mutant: `leaving` not set at the click.
  it("says nothing about a 401 that lands while a deliberate sign-out is re-reading", async () => {
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    let held: { promise: Promise<Reply>; resolve: (v: Reply) => void } | null = null;
    const stub = stubApi((call) => {
      if (call.path === "/api/leave") return OPEN_DOOR_LEAVE;
      if (!call.path.endsWith("/api/me")) return undefined;
      if (held) return held.promise;
      return localStorage.getItem(CLAIMED_USER_KEY) ? wire.me : wire.meSignedOut;
    });
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));

    const reread = gate<Reply>();
    held = reread;
    act(() => result.current.signOut());
    await waitFor(() => expect(stub.matching("GET", "/api/me")).toHaveLength(2));
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

  // **A provider-signed Worker's 401 is a challenge too** (the auth cycle, Task
  // 6): `call` reads its `WWW-Authenticate: Yokohama-Session` as
  // `SIGN_IN_NEEDED`, and the door draws the way in rather than the server's
  // refusal. Mutants: `isSessionExpired` reading the Access sentence alone; the
  // status unread (the sentence on a 403 is no sign-in).
  it("failureOf reads SIGN_IN_NEEDED as a challenge", () => {
    expect(failureOf(new ApiError(SIGN_IN_NEEDED, 401))).toBe("challenge");
    expect(failureOf(new ApiError(ACCESS_CHALLENGE, 401))).toBe("challenge");
    expect(failureOf(new ApiError(SIGN_IN_NEEDED, 403))).toBe("refused");
    expect(failureOf(new ApiError("no signed-in user", 401))).toBe("refused");
  });
});

// **Sign-out is the server's to end** (the auth cycle, Task 6; ruling R-37):
// `POST /api/leave` answers where the player goes to end the session that
// signed them in — the issuer's logout under a sign-in arm, Access's own under
// Access — and the client goes there. Under the open door there is no server
// session: the route is the house's 404, and the sign-out is the one the open
// door always had — the stored name dropped, `/api/me` asked again.
describe("signOut through /api/leave", () => {
  const LEAVE =
    "https://api.workos.com/user_management/sessions/logout?session_id=s&return_to=https%3A%2F%2Fyokohama.example%2F";

  // Mutants: the navigation made before the answer lands (or to anything but
  // the answer's `leave`); `/api/me` asked again on the way out; the leave
  // never asked.
  it("signOut POSTs /api/leave and navigates to the answer's leave", async () => {
    const went = countNavigations();
    const answer = gate<Reply>();
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/me")) return { ...wire.me, mode: "cf-access" };
      if (call.method === "POST" && call.path === "/api/leave") return answer.promise;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));

    act(() => result.current.signOut());
    await waitFor(() => expect(stub.matching("POST", "/api/leave")).toHaveLength(1));
    expect(went, "not before the server has said where").toEqual([]);
    answer.resolve({ leave: LEAVE });
    await waitFor(() => expect(went).toEqual([LEAVE]));
    // The navigation IS the sign-out: nothing is asked again on the way out.
    expect(stub.matching("GET", "/api/me")).toHaveLength(1);
  });

  // **The NAS answers no `/api/leave`** (the auth cycle, Task 6; ruling R-41):
  // this cycle moves no Rust, and the layer's 404 there would leave the button
  // re-reading `/api/me` — which Access answers with the same player, a
  // sign-out that visibly does nothing. The NAS sits behind Access (B10's one
  // rule), so under a `cf-access` door a 404 goes to Access's own logout; every
  // other failure keeps the open door's sign-out. Mutants: the fallback taken
  // on every 404 whatever the door (the open door navigates); the fallback
  // taken on any failure under `cf-access` (a network failure navigates); the
  // fallback never taken (the NAS re-reads).
  it("goes to Access's own logout when a cf-access door answers the leave 404, and keeps today's path otherwise", async () => {
    const went = countNavigations();
    const behindAccess = (leave: () => Reply) =>
      stubApi((call) => {
        if (call.path.endsWith("/api/me")) return { ...wire.me, mode: "cf-access" };
        if (call.method === "POST" && call.path === "/api/leave") return leave();
        throw new Error(`unexpected ${call.method} ${call.path}`);
      });

    // The NAS: behind Access, and no such endpoint.
    const nas = behindAccess(() => ({ status: 404, body: { error: "no such endpoint: /api/leave" } }));
    const first = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(first.result.current.me?.user).toBe(wire.me.user));
    act(() => first.result.current.signOut());
    await waitFor(() => expect(went).toEqual(["/cdn-cgi/access/logout"]));
    expect(nas.matching("GET", "/api/me"), "nothing re-read on the way out").toHaveLength(1);
    first.unmount();

    // The open door: the same 404, and the sign-out it always had.
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const open = meFromStorage();
    const second = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(second.result.current.me?.user).toBe(wire.me.user));
    act(() => second.result.current.signOut());
    await waitFor(() => expect(second.result.current.me).toEqual(wire.meSignedOut));
    expect(open.matching("POST", "/api/leave")).toHaveLength(1);
    expect(went, "the open door navigates nowhere").toEqual(["/cdn-cgi/access/logout"]);
    second.unmount();

    // Behind Access, but nothing answered at all: the same path, no navigation.
    const offline = behindAccess(() => {
      throw new TypeError("Failed to fetch");
    });
    const third = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(third.result.current.me?.user).toBe(wire.me.user));
    act(() => third.result.current.signOut());
    await waitFor(() => expect(offline.matching("GET", "/api/me")).toHaveLength(2));
    expect(went, "a network failure navigates nowhere").toEqual(["/cdn-cgi/access/logout"]);
  });

  // Mutants: the failure path navigating anyway; the stored name kept; the
  // door not re-read; a 200 that names nowhere navigated to.
  it("keeps the open door's sign-out when the leave is refused: the name dropped, /api/me asked again, nothing navigated", async () => {
    const went = countNavigations();
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const stub = meFromStorage();
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    act(() => result.current.signOut());
    await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
    expect(stub.matching("POST", "/api/leave")).toHaveLength(1);
    expect(stub.matching("GET", "/api/me")).toHaveLength(2);
    expect(localStorage.getItem(CLAIMED_USER_KEY)).toBeNull();
    expect(result.current.notice).toBeNull();
    expect(went).toEqual([]);

    // A 200 that names nowhere is no answer to navigate on: the same path —
    // no `leave` at all, and an EMPTY one, which navigated to would reload this
    // page instead of signing out (review M-2).
    for (const nowhere of [{}, { leave: "" }]) {
      localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
      stubApi((call) =>
        call.path.endsWith("/api/me")
          ? localStorage.getItem(CLAIMED_USER_KEY)
            ? wire.me
            : wire.meSignedOut
          : call.path === "/api/leave"
            ? nowhere
            : undefined,
      );
      act(() => result.current.refresh());
      await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
      act(() => result.current.signOut());
      await waitFor(() => expect(result.current.me).toEqual(wire.meSignedOut));
      expect(went, JSON.stringify(nowhere)).toEqual([]);
    }
  });

  // **The display name goes with the sign-out, in the ONE wiring** (the auth
  // cycle, Task 6 fix round 1, review M-3): it labels whoever signs in next on
  // this browser, and behind an edge door the sign-out is a navigation, so
  // nothing after it runs — it is dropped at the click, whatever the leave
  // answers. The cell waits for the navigation the answer triggers, so nothing
  // it started runs after its last assertion. Mutants: the drop left to one
  // control (the game menu's sign-out keeps the name); the drop after the
  // answer (a navigation leaves it).
  it("drops the display name at the click, whatever the leave answers", async () => {
    const went = countNavigations();
    const answer = gate<Reply>();
    stubApi((call) => {
      if (call.path.endsWith("/api/me")) return { ...wire.me, mode: "cf-access" };
      if (call.method === "POST" && call.path === "/api/leave") return answer.promise;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    localStorage.setItem(DISPLAY_NAME_KEY, "Ben Castle");
    const { result } = renderHook(() => useMe("multiplayer"));
    await waitFor(() => expect(result.current.me?.user).toBe(wire.me.user));
    act(() => result.current.signOut());
    expect(localStorage.getItem(DISPLAY_NAME_KEY), "gone before the server answers").toBeNull();
    answer.resolve({ leave: "/cdn-cgi/access/logout" });
    await waitFor(() => expect(went).toEqual(["/cdn-cgi/access/logout"]));
    expect(localStorage.getItem(DISPLAY_NAME_KEY), "and still gone once it has").toBeNull();
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
