// Who owns a local game, as a hook (spec §2.2, R-6.5b.5).
//
// **The Worker is faked and the assertions are about the PAGE.** Whether
// `navigator.locks` really keeps two tabs apart is the Worker's claim and the
// browser's; what is tested here is the page's half of the contract — that a
// hold is taken before anything is read, that leaving RELEASES, and that the
// three answers the lock can give each reach the screen as their own state.

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { localCall, resetLocalHostForTests } from "./client";
import { useLocalHold } from "./useLocalHold";
import { currentWorker, FakeWorker, stubWorker } from "../test/fakeWorker";
import type { Req } from "./client";

/** Every message of one kind this tab has posted, by the id it named. */
const posted = (kind: Req["kind"]): string[] =>
  FakeWorker.instances
    .flatMap((w) => w.posted)
    .filter((p): p is Extract<Req, { id: string }> => p.kind === kind && "id" in p)
    .map((p) => p.id);

beforeEach(() => {
  resetLocalHostForTests();
});
afterEach(() => {
  resetLocalHostForTests();
});

describe("the hold on a local game", () => {
  it("takes the lock on mount and reports that it holds it", async () => {
    stubWorker();
    const { result } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("held"));
    expect(posted("hold")).toEqual(["local-1"]);
  });

  // **The second tab, and the whole reason the hold comes first** (P-5): a cold
  // GET on a local game PUMPS the bot, so a read taken before the answer can
  // change a record this tab does not own. The screen this state draws asks
  // for nothing at all.
  it("reports elsewhere when another tab has the game open", async () => {
    stubWorker({ hold: () => false });
    const { result } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("elsewhere"));
  });

  // **Navigating away RELEASES** (spec §2.2): the page holds the lock for as
  // long as the game screen is mounted, so a player who leaves for the lobby
  // frees the game for the other tab without closing anything.
  it("releases when the screen unmounts", async () => {
    stubWorker();
    const { result, unmount } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("held"));
    expect(posted("release")).toEqual([]);
    unmount();
    await waitFor(() => expect(posted("release")).toEqual(["local-1"]));
  });

  // The same rule for a SWITCH rather than a leave — the Game menu opens
  // another table without unmounting the screen, and a hold left behind would
  // lock a game nobody is looking at for the life of the tab.
  it("releases the old game and holds the new one when the id changes", async () => {
    stubWorker();
    const { result, rerender } = renderHook(({ id }) => useLocalHold(id), {
      initialProps: { id: "local-1" },
    });
    await waitFor(() => expect(result.current.state).toBe("held"));
    rerender({ id: "local-2" });
    await waitFor(() => expect(posted("hold")).toEqual(["local-1", "local-2"]));
    expect(posted("release")).toEqual(["local-1"]);
  });

  // A hosted game, and the first screen: neither is this tab's to lock, and
  // neither starts the Worker. `none` is what every screen that is not a local
  // game reads.
  it("asks for nothing at a null id, and nothing for a SERVER game", async () => {
    stubWorker();
    const { result, rerender } = renderHook(({ id }) => useLocalHold(id), {
      initialProps: { id: null as string | null },
    });
    await waitFor(() => expect(result.current.state).toBe("none"));
    rerender({ id: "wire" });
    await waitFor(() => expect(result.current.state).toBe("none"));
    expect(FakeWorker.instances).toHaveLength(0);
  });

  // **The steal, from the losing side** (spec §4). The Worker's unsolicited
  // message is what turns this tab read-only, and it is about ONE game: a tab
  // holding two local games must not go read-only on both.
  it("goes lost when the Worker says this game was taken, and not for another", async () => {
    stubWorker();
    const { result } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("held"));

    act(() => currentWorker().emit({ kind: "lost", id: "local-2" }));
    expect(result.current.state).toBe("held");

    act(() => currentWorker().emit({ kind: "lost", id: "local-1" }));
    await waitFor(() => expect(result.current.state).toBe("lost"));
  });

  // **An acquisition that could not read the store is its own answer** (M6.5c2
  // cycle D, dossier row 14). It used to be a trap, and the page restarted the
  // host into the same store; now the Worker answers in the storage's words,
  // and the hook hands the screen a state it can draw — the broken-store face
  // with Reload — and the sentence to put on it. Nothing restarts, and the
  // game is not released: the hold was never granted.
  it("reports UNREADABLE with the store's own sentence, restarts nothing, and releases nothing", async () => {
    const sentence = "this browser's storage could not be read \u2014 The database connection is closing.";
    stubWorker({ hold: () => ({ unreadable: sentence }) });
    const { result, unmount } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("unreadable"));
    expect(result.current.id).toBe("local-1");
    expect(result.current.sentence).toBe(sentence);
    expect(FakeWorker.instances).toHaveLength(1);
    expect(currentWorker().terminated).toBe(false);
    unmount();
    expect(posted("release")).toEqual([]);
  });

  // **A Take over the store refuses is the same face** (fix round 1, review
  // Minor 3): not the lock screen's refusal line, which drew the sentence
  // without the Reload that is its one remedy.
  it("reports UNREADABLE when a Take over could not read the store", async () => {
    const sentence = "this browser's storage could not be read \u2014 The database connection is closing.";
    stubWorker({ hold: (_id, steal) => (steal ? { unreadable: sentence } : false) });
    const { result, unmount } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("elsewhere"));
    await act(() => result.current.takeover());
    expect(result.current).toMatchObject({ id: "local-1", state: "unreadable", sentence });
    unmount();
    expect(posted("release"), "and the refused steal is not released").toEqual([]);
  });

  // **And a game this tab HELD whose restart could not read the store** (fix
  // round 1, review Minor 1): the client says so for that game, and the hook
  // draws the same face.
  it("reports UNREADABLE when a restart could not acquire the game again", async () => {
    const sentence = "this browser's storage could not be read \u2014 The database connection is closing.";
    let fresh = false;
    stubWorker({
      hold: () => (fresh ? { unreadable: sentence } : true),
      call: () => ({ status: 500, body: { error: "unreachable executed" }, trapped: true }),
    });
    const { result } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("held"));
    fresh = true;
    await act(() => localCall("GET", "/api/games/local-1/ports").catch(() => undefined));
    await waitFor(() => expect(result.current).toMatchObject({ id: "local-1", state: "unreadable", sentence }));
  });

  // **The hook releases only what it may be holding** (M6.5c2 cycle D, dossier
  // row 56). A hold answered "open in another tab" was never this tab's, and
  // an unconditional release read as if the page believed it held the game.
  it("releases nothing for a game another tab has open", async () => {
    stubWorker({ hold: () => false });
    const { result, unmount } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("elsewhere"));
    unmount();
    expect(posted("release")).toEqual([]);
  });

  // The partner that keeps the rule above from going too far: a hold still IN
  // FLIGHT when the screen goes is released, because the Worker's own record
  // of what the page wants (`wanted`) is what makes the acquisition drop the
  // lock when it lands — React's StrictMode sends hold, release, hold before
  // the first has answered, and a real departure can be that quick too.
  it("releases a hold still in flight when the screen goes", async () => {
    stubWorker({ hold: () => "pending" });
    const { unmount } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(posted("hold")).toEqual(["local-1"]));
    unmount();
    expect(posted("release")).toEqual(["local-1"]);
  });

  // And the other partner: a game this tab found open elsewhere and then TOOK
  // OVER is this tab's, so leaving releases it.
  it("releases a game it took over, when the screen goes", async () => {
    stubWorker({ hold: (_id, steal) => steal });
    const { result, unmount } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("elsewhere"));
    await act(() => result.current.takeover());
    expect(result.current.state).toBe("held");
    unmount();
    expect(posted("release")).toEqual(["local-1"]);
  });

  // **A takeover that lands after the route has moved on changes nothing**
  // (M6.5c2 cycle D, dossier row 56: `take`'s missing `live` guard). The
  // answer is about the game being LEFT; setting it would put that game's
  // "held" over the one the route now names, which is the stale read
  // `Hold.id` exists to stop.
  it("keeps the new game's answer when a takeover of the old one lands late", async () => {
    stubWorker({ hold: (id, steal) => (steal ? "pending" : id === "local-2") });
    const { result, rerender } = renderHook(({ id }) => useLocalHold(id), {
      initialProps: { id: "local-1" },
    });
    await waitFor(() => expect(result.current.state).toBe("elsewhere"));
    const taking = result.current.takeover();
    rerender({ id: "local-2" });
    await waitFor(() => expect(result.current).toMatchObject({ id: "local-2", state: "held" }));
    // The steal was in flight when the screen left the game, so the game IS
    // released — the one release that lets the Worker drop a lock landing
    // after the page has gone.
    expect(posted("release")).toEqual(["local-1"]);

    const steal = currentWorker().posted.find((p) => p.kind === "takeover")!;
    act(() => currentWorker().emit({ seq: steal.seq, held: true }));
    await act(() => taking);
    expect(result.current).toMatchObject({ id: "local-2", state: "held" });
  });

  // Take over and Take back are one message and one state: the lock is STOLEN,
  // and this tab holds it again.
  it("takes the game back, and says so", async () => {
    stubWorker();
    const { result } = renderHook(() => useLocalHold("local-1"));
    await waitFor(() => expect(result.current.state).toBe("held"));
    act(() => currentWorker().emit({ kind: "lost", id: "local-1" }));
    await waitFor(() => expect(result.current.state).toBe("lost"));

    await act(() => result.current.takeover());
    expect(result.current.state).toBe("held");
    expect(posted("takeover")).toEqual(["local-1"]);
  });
});
