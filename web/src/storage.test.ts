// What this browser has promised about the games on it (R-6.5b2.9, spec §2.4).
//
// **Three states and none of them claims safety**: granted, refused, and a
// browser that will not say. The wording is the component's; what these cells
// pin is that each of the three is REACHABLE and that none of the three paths
// throws — a report is drawn on the door's first paint, and a door that died
// on a missing `navigator.storage` would take the local games with it.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { stubStorage } from "./test/platform";
import {
  humanBytes,
  requestPersistence,
  requestPersistenceOnce,
  resetPersistenceRequestForTests,
  storageReport,
} from "./storage";

describe("the storage report", () => {
  beforeEach(() => {
    resetPersistenceRequestForTests();
  });
  afterEach(() => {
    resetPersistenceRequestForTests();
  });

  it("reads the grant BACK rather than assuming it", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({
      persist,
      persisted: vi.fn().mockResolvedValue(false),
      estimate: vi.fn().mockResolvedValue({ usage: 12_000, quota: 1_200_000_000 }),
    });
    await requestPersistence();
    expect(persist).toHaveBeenCalledTimes(1);
    // `persist()` said true and `persisted()` says false. The REPORT is the
    // second one: a resolved promise is not a promise about the games, and a
    // report built from the request would tell a player their games are kept
    // on a browser that has not agreed to keep them.
    expect(await storageReport()).toEqual({ persisted: false, usage: 12_000, quota: 1_200_000_000 });
  });

  it("reports a grant", async () => {
    stubStorage({
      persist: vi.fn().mockResolvedValue(true),
      persisted: vi.fn().mockResolvedValue(true),
      estimate: vi.fn().mockResolvedValue({ usage: 54_103, quota: 2_000_000_000 }),
    });
    expect(await storageReport()).toEqual({ persisted: true, usage: 54_103, quota: 2_000_000_000 });
  });

  it("answers null where the browser has no storage manager at all", async () => {
    stubStorage(undefined);
    expect(await storageReport()).toBeNull();
  });

  it("answers null where the manager cannot say whether it is persisted", async () => {
    stubStorage({ estimate: vi.fn().mockResolvedValue({ usage: 1, quota: 2 }) });
    expect(await storageReport()).toBeNull();
  });

  // **A half-implemented manager still reports the grant.** `estimate()` is
  // the newer half; a browser that answers `persisted()` and not `estimate()`
  // knows the one thing the line is mostly about.
  it("reports the grant with no sizes when estimate is absent", async () => {
    stubStorage({ persisted: vi.fn().mockResolvedValue(true) });
    expect(await storageReport()).toEqual({ persisted: true, usage: 0, quota: 0 });
  });

  it("swallows a persisted() that throws, rather than taking the screen down", async () => {
    stubStorage({
      persisted: vi.fn().mockRejectedValue(new Error("the operation is insecure")),
      estimate: vi.fn().mockResolvedValue({ usage: 1, quota: 2 }),
    });
    expect(await storageReport()).toBeNull();
  });

  it("swallows an estimate() that throws and still reports the grant", async () => {
    stubStorage({
      persisted: vi.fn().mockResolvedValue(true),
      estimate: vi.fn().mockRejectedValue(new Error("the operation is insecure")),
    });
    expect(await storageReport()).toEqual({ persisted: true, usage: 0, quota: 0 });
  });

  it("asks for persistence without throwing where there is nothing to ask", async () => {
    stubStorage(undefined);
    await expect(requestPersistence()).resolves.toBeUndefined();
  });

  it("swallows a refusal from persist() itself", async () => {
    stubStorage({ persist: vi.fn().mockRejectedValue(new Error("not allowed")) });
    await expect(requestPersistence()).resolves.toBeUndefined();
  });

  // **Once per page**, which is what keeps the ask off every move: a player
  // who declines the browser's prompt on move one is not asked again on move
  // two, and a browser that grants it silently is not asked a second time
  // either.
  it("asks at most once per page", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({ persist, persisted: vi.fn().mockResolvedValue(true) });
    await requestPersistenceOnce();
    await requestPersistenceOnce();
    await requestPersistenceOnce();
    expect(persist).toHaveBeenCalledTimes(1);
  });

  // Two calls in ONE tick, which is the shape a queued run of moves takes: the
  // flag has to be set before the await, not after it.
  it("asks once when two callers arrive in the same tick", async () => {
    const persist = vi.fn().mockResolvedValue(true);
    stubStorage({ persist, persisted: vi.fn().mockResolvedValue(true) });
    await Promise.all([requestPersistenceOnce(), requestPersistenceOnce()]);
    expect(persist).toHaveBeenCalledTimes(1);
  });
});

describe("the sizes a player reads", () => {
  it("names the unit and keeps the number short", () => {
    expect(humanBytes(0)).toBe("0 B");
    expect(humanBytes(999)).toBe("999 B");
    expect(humanBytes(12_000)).toBe("12 KB");
    expect(humanBytes(54_103)).toBe("54 KB");
    expect(humanBytes(1_200_000_000)).toBe("1.2 GB");
    expect(humanBytes(2_000_000_000)).toBe("2 GB");
  });
});
