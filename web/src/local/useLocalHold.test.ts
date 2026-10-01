// Who owns a local game, as a hook (spec §2.2, R-6.5b.5).
//
// **The Worker is faked and the assertions are about the PAGE.** Whether
// `navigator.locks` really keeps two tabs apart is the Worker's claim and the
// browser's; what is tested here is the page's half of the contract — that a
// hold is taken before anything is read, that leaving RELEASES, and that the
// three answers the lock can give each reach the screen as their own state.

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { resetLocalHostForTests } from "./client";
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
