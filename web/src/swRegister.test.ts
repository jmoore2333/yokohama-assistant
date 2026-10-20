// Registration, the readiness state and the update offer (M6.5b2, spec §2.3
// and §4).
//
// **The lifecycle is driven through a FAKE host rather than through jsdom's
// navigator**, which has no `serviceWorker` at all: the hook takes its
// registration surface as one parameter, so a cell can put a worker into
// `installed` or `redundant` and read what the door would say. The pure half
// (`serviceWorkerWanted`, `readinessOf`) is asserted directly beside it,
// because those two are where the policy actually lives.

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  applyBusy,
  readinessOf,
  RECHECK_MS,
  resetDoorAutoApplyForTests,
  serviceWorkerWanted,
  setApplyBusy,
  SW_URL,
  useDoorAutoApply,
  useReadiness,
  type Readiness,
  type ReadinessStatus,
  type RegistrationLike,
  type SwHost,
  type WorkerLike,
} from "./swRegister";
import { resetTurnstileForTests, setChallengeHandler } from "./turnstile";

class FakeWorker implements WorkerLike {
  state: string;
  readonly posted: unknown[] = [];
  private readonly listeners = new Set<() => void>();
  constructor(state = "installing") {
    this.state = state;
  }
  addEventListener(_type: "statechange", listener: () => void): void {
    this.listeners.add(listener);
  }
  removeEventListener(_type: "statechange", listener: () => void): void {
    this.listeners.delete(listener);
  }
  postMessage(message: unknown): void {
    this.posted.push(message);
  }
  /** The `statechange` listeners attached now — what an unmount must bring back
   *  to zero. */
  get listening(): number {
    return this.listeners.size;
  }
  /** Drive the lifecycle the way a browser does. */
  become(state: string): void {
    this.state = state;
    for (const listener of [...this.listeners]) listener();
  }
}

class FakeRegistration implements RegistrationLike {
  installing: WorkerLike | null = null;
  waiting: WorkerLike | null = null;
  active: WorkerLike | null = null;
  updates = 0;
  /** What the server's worker does when `update()` fetches it: `undefined` is
   *  the byte-for-byte shortcut, which installs nothing; a cell that wants a
   *  new install passes the worker it starts. */
  nextInstall: FakeWorker | undefined;
  private readonly found = new Set<() => void>();
  addEventListener(_type: "updatefound", listener: () => void): void {
    this.found.add(listener);
  }
  removeEventListener(_type: "updatefound", listener: () => void): void {
    this.found.delete(listener);
  }
  /** **The platform's `update()`, including its refusal** (the Service Worker
   *  spec's `update()` step 3): a registration with no newest worker — which
   *  is what a failed FIRST install leaves — is refused with an
   *  `InvalidStateError`, the sentence Chromium was measured to give. A new
   *  install sets `installing` before the promise resolves, as the platform
   *  does. The fake fires `updatefound` BEFORE resolving, where the platform
   *  resolves the job promise first and queues `updatefound` after it on the
   *  same task source; the hook reaches the same state either way, because
   *  `settle` and the `watch` that `updatefound` adds commute. */
  async update(): Promise<void> {
    if (!this.installing && !this.waiting && !this.active) {
      throw new DOMException(
        "Failed to update a ServiceWorker for scope ('http://127.0.0.1/') with script ('Unknown'): The object is in an invalid state.",
        "InvalidStateError",
      );
    }
    this.updates += 1;
    if (this.nextInstall) this.updateFound(this.nextInstall);
  }
  /** **An install that failed**, in the platform's order: the worker goes
   *  `redundant` (its `statechange` fires while `installing` still names it),
   *  then `installing` is emptied. A first install leaves all three empty. */
  installFailed(): void {
    const worker = this.installing as FakeWorker;
    worker.become("redundant");
    this.installing = null;
  }
  /** The `updatefound` listeners attached now. */
  get listening(): number {
    return this.found.size;
  }
  /** A new worker appears for this registration. */
  updateFound(worker: FakeWorker): void {
    this.installing = worker;
    for (const listener of [...this.found]) listener();
  }
}

function fakeHost(registration: FakeRegistration, wanted = true) {
  let onController: (() => void) | null = null;
  let onMessage: ((data: unknown) => void) | null = null;
  const reloads: number[] = [];
  const host: SwHost & { reloads: number[]; isControlled: boolean } = {
    wanted,
    reloads,
    // A returning player's page, which is what every update cell below is
    // about unless it says otherwise.
    isControlled: true,
    controlled: () => host.isControlled,
    register: vi.fn(async () => registration as RegistrationLike),
    onControllerChange(listener) {
      onController = listener;
      return () => {
        onController = null;
      };
    },
    onMessage(listener) {
      onMessage = listener;
      return () => {
        onMessage = null;
      };
    },
    reload() {
      reloads.push(1);
    },
  };
  return {
    host,
    controllerChange: () => onController?.(),
    message: (data: unknown) => onMessage?.(data),
  };
}

/** A first install walked `installing → installed → activating → activated`,
 *  each step announced by `statechange` the way a browser moves the one worker
 *  from `installing` through `waiting` to `active` — and the door read as
 *  ready once it has installed, which is when the release is whole. */
function walkToActivated(registration: FakeRegistration, worker: FakeWorker, readiness: () => string) {
  act(() => {
    registration.installing = null;
    registration.waiting = worker;
    worker.become("installed");
  });
  expect(readiness()).toBe("ready");
  act(() => {
    registration.waiting = null;
    registration.active = worker;
    worker.become("activating");
  });
  act(() => worker.become("activated"));
}

describe("whether this build registers a worker at all", () => {
  const supported = { navigator: { serviceWorker: {} } };
  // **Production only** (spec §2.3): Vite's dev server serves no `dist`, and a
  // worker registered there would serve a stale module tree from a cache built
  // out of files that do not exist.
  it("registers in a production build and never in development", () => {
    expect(serviceWorkerWanted(supported, true)).toBe(true);
    expect(serviceWorkerWanted(supported, false)).toBe(false);
  });

  // **The e2e flag**, without which each of the walk's existing cells would
  // register a worker and start a ten-megabyte precache per context.
  it("stands down when the page has been flagged", () => {
    expect(serviceWorkerWanted({ ...supported, __YOKOHAMA_NO_SW__: true }, true)).toBe(false);
  });

  it("stands down where the browser has none", () => {
    expect(serviceWorkerWanted({ navigator: {} }, true)).toBe(false);
  });
});

describe("what a registration's shape means", () => {
  const snap = (
    installingState: string | null,
    hasWaiting: boolean,
    hasActive: boolean,
  ) => readinessOf({ installingState, hasWaiting, hasActive });

  it("is installing while a worker is installing", () => {
    expect(snap("installing", false, false)).toEqual({
      readiness: "installing",
      updateWaiting: false,
    });
  });

  it("is ready once it has installed, and once it is active", () => {
    expect(snap("installed", false, false).readiness).toBe("ready");
    expect(snap("activated", false, true).readiness).toBe("ready");
    expect(snap(null, false, true).readiness).toBe("ready");
  });

  // A redundant installing worker is the lifecycle's word for "the install
  // threw", which is what the all-or-nothing install does on one bad file.
  it("is failed when the installing worker went redundant", () => {
    expect(snap("redundant", false, false).readiness).toBe("failed");
  });

  // **The offer needs BOTH**: a waiting worker with no active one is a first
  // install, and offering to reload into it would be offering nothing.
  it("offers the update only when a new worker waits behind a running one", () => {
    expect(snap(null, true, true)).toEqual({ readiness: "ready", updateWaiting: true });
    expect(snap(null, true, false).updateWaiting).toBe(false);
  });
});

describe("the hook", () => {
  it("does nothing at all when the build does not want one", async () => {
    const registration = new FakeRegistration();
    const { host } = fakeHost(registration, false);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.readiness).toBe("none");
    expect(host.register).not.toHaveBeenCalled();
  });

  it("walks a first install from installing to ready", async () => {
    const registration = new FakeRegistration();
    const worker = new FakeWorker("installing");
    registration.installing = worker;
    const { host } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(host.register).toHaveBeenCalledWith(SW_URL);
    expect(result.current.readiness).toBe("installing");
    act(() => {
      registration.active = worker;
      worker.become("activated");
    });
    expect(result.current.readiness).toBe("ready");
    expect(result.current.updateWaiting).toBe(false);
  });

  // The install is all-or-nothing, so one bad file is a FAILED door with a
  // retry — never a door that says "installing" for ever.
  //
  // **And the retry must be able to SUCCEED** (PR #37 F3, E1). A failed FIRST
  // install leaves the registration with no newest worker and removes it from
  // the registration map, so `update()` on it is refused on every click — the
  // door went back to failed with "The object is in an invalid state", measured
  // in Chromium. The retry registers again and adopts what that resolves to.
  it("names the failure the worker reported, and a retry after a failed FIRST install registers again and reaches ready", async () => {
    const first = new FakeRegistration();
    first.installing = new FakeWorker("installing");
    const second = new FakeRegistration();
    const worker = new FakeWorker("installing");
    second.installing = worker;
    const { host, message } = fakeHost(first);
    host.register = vi
      .fn<SwHost["register"]>()
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => {
      message({ yokohama: "install-failed", kind: "failed", reason: "/art/orders/O01.webp answered 404" });
      first.installFailed();
    });
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBe("/art/orders/O01.webp answered 404");

    await act(async () => {
      result.current.retry();
    });
    expect(result.current.reason).toBeNull();
    expect(host.register).toHaveBeenCalledTimes(2);
    expect(first.updates).toBe(0);
    expect(result.current.readiness).toBe("installing");
    walkToActivated(second, worker, () => result.current.readiness);
    expect(result.current.readiness).toBe("ready");
  });

  // **The dying worker's window** (PR #37, R-orch-5). The worker posts
  // "install-failed" BEFORE its install rejects, so for a few milliseconds the
  // page's copy of the registration still names the dying worker as
  // `installing`. An `update()` taken there queues behind the failing install
  // and is refused once that install has removed the registration; a
  // `register()` queues the same way and then registers anew. So `installing`
  // is no evidence of a registration worth updating — only a waiting or an
  // active worker is.
  it("registers, and does not update, while the only worker is the one still dying", async () => {
    const first = new FakeRegistration();
    first.installing = new FakeWorker("installing");
    const second = new FakeRegistration();
    const worker = new FakeWorker("installing");
    second.installing = worker;
    const { host, message } = fakeHost(first);
    host.register = vi
      .fn<SwHost["register"]>()
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    // The message has arrived; the install has not yet been failed.
    act(() => message({ yokohama: "install-failed", kind: "failed", reason: "/art/orders/O01.webp answered 404" }));
    expect(result.current.readiness).toBe("failed");
    expect(first.installing, "the premise: the dying worker is still named").not.toBeNull();

    await act(async () => {
      result.current.retry();
    });
    expect(first.updates).toBe(0);
    expect(host.register).toHaveBeenCalledTimes(2);
    walkToActivated(second, worker, () => result.current.readiness);
    expect(result.current.readiness).toBe("ready");
  });

  // **Where the release before it is still ACTIVE, the retry updates**: an
  // UPDATE whose install failed leaves a newest worker, so `update()` fetches
  // the worker again and a changed one installs again.
  it("retries a failed UPDATE through update(), which installs again", async () => {
    const registration = new FakeRegistration();
    registration.active = new FakeWorker("activated");
    registration.installing = new FakeWorker("installing");
    const { host, message } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => {
      message({ yokohama: "install-failed", kind: "failed", reason: "/art/orders/O01.webp answered 404" });
      registration.installFailed();
    });
    expect(result.current.readiness).toBe("failed");

    const next = new FakeWorker("installing");
    registration.nextInstall = next;
    await act(async () => {
      result.current.retry();
    });
    expect(registration.updates).toBe(1);
    expect(host.register).toHaveBeenCalledTimes(1);
    expect(result.current.readiness).toBe("installing");
    act(() => {
      registration.installing = null;
      registration.waiting = next;
      next.become("installed");
    });
    expect(result.current.readiness).toBe("ready");
    expect(result.current.updateWaiting).toBe(true);
  });

  // **An `update()` that installs nothing still settles the door.** The
  // byte-for-byte shortcut — the server's worker went back to the running
  // one's bytes between the failure and the click — resolves with no new
  // worker and no event, and the retry had already said "installing".
  it("reads the registration again when the retry's update() installs nothing", async () => {
    const registration = new FakeRegistration();
    registration.active = new FakeWorker("activated");
    registration.installing = new FakeWorker("installing");
    const { host, message } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => {
      message({ yokohama: "install-failed", kind: "failed", reason: "/art/orders/O01.webp answered 404" });
      registration.installFailed();
    });
    expect(result.current.readiness).toBe("failed");
    await act(async () => {
      result.current.retry();
    });
    expect(registration.updates).toBe(1);
    expect(result.current.readiness).toBe("ready");
    expect(result.current.reason).toBeNull();
  });

  // A worker that went redundant without saying why is still a failure, and the
  // door still has something to offer.
  it("fails when the installing worker goes redundant, with or without a reason", async () => {
    const registration = new FakeRegistration();
    const worker = new FakeWorker("installing");
    registration.installing = worker;
    const { host } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => worker.become("redundant"));
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBeNull();
    expect(host.reloads).toEqual([]);
  });

  it("offers the update when a second worker installs behind the running one", async () => {
    const registration = new FakeRegistration();
    const running = new FakeWorker("activated");
    registration.active = running;
    const { host } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.readiness).toBe("ready");
    expect(result.current.updateWaiting).toBe(false);

    const next = new FakeWorker("installing");
    act(() => registration.updateFound(next));
    expect(result.current.updateWaiting).toBe(false);
    act(() => {
      registration.installing = null;
      registration.waiting = next;
      next.become("installed");
    });
    expect(result.current.updateWaiting).toBe(true);
    expect(result.current.readiness).toBe("ready");
  });

  // **The switch is the CLICK and nothing else** (spec §2.3): the message goes
  // to the waiting worker, and the reload waits for `controllerchange` rather
  // than racing activation.
  it("posts skip-waiting on the click and reloads only once the worker has taken over", async () => {
    const registration = new FakeRegistration();
    const running = new FakeWorker("activated");
    const next = new FakeWorker("installed");
    registration.active = running;
    registration.waiting = next;
    const { host, controllerChange } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.updateWaiting).toBe(true);

    act(() => result.current.applyUpdate());
    expect(next.posted).toEqual([{ yokohama: "skip-waiting" }]);
    // Not yet: a reload posted straight after the message races activation and
    // lands on the OLD worker.
    expect(host.reloads).toEqual([]);
    act(() => controllerChange());
    expect(host.reloads.length).toBe(1);
  });

  // A controller change this tab did not ask for — another tab clicked the
  // offer, and `skipWaiting` is registration-wide — must NOT reload this one.
  // That is the whole of the two-tab policy on the page's side.
  it("does not reload on a controller change nobody here clicked for", async () => {
    const registration = new FakeRegistration();
    registration.active = new FakeWorker("activated");
    const { host, controllerChange } = fakeHost(registration);
    renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => controllerChange());
    expect(host.reloads).toEqual([]);
  });

  // **A retry that itself fails goes back to FAILED**, not to a door stuck on
  // "Preparing to play offline" — the install's own deadline exists for that
  // shape, and a rejected retry is the same shape one level up.
  it("goes back to failed when the retry itself is refused", async () => {
    const registration = new FakeRegistration();
    registration.active = new FakeWorker("activated");
    registration.installing = new FakeWorker("installing");
    registration.update = async () => {
      throw new Error("NetworkError: the script could not be fetched");
    };
    const { host, message } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => {
      message({ yokohama: "install-failed", kind: "failed", reason: "/index.html answered 502" });
      registration.installFailed();
    });
    expect(result.current.readiness).toBe("failed");
    await act(async () => {
      result.current.retry();
    });
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toMatch(/NetworkError/);
  });

  // **A tab nothing controls is not promised a `controllerchange`** (PR #37 F3,
  // E2; Chromium sends one anyway). `skipWaiting` hands the new worker the tabs
  // the old one CONTROLLED —
  // a first visit is not one, since nothing calls `clients.claim()`, and nor is
  // a page whose first registration was retried — so such a tab that clicks
  // reloads when the worker it asked reaches `activated`. Before this it
  // reloaded nothing, and the offer vanished with the tab still on the old
  // bundle.
  it("reloads an UNCONTROLLED tab that clicked once the worker it asked has activated", async () => {
    const registration = new FakeRegistration();
    const next = new FakeWorker("installed");
    registration.active = new FakeWorker("activated");
    registration.waiting = next;
    const { host } = fakeHost(registration);
    host.isControlled = false;
    const { result, unmount } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.updateWaiting).toBe(true);
    const watching = next.listening;

    act(() => result.current.applyUpdate());
    expect(next.posted).toEqual([{ yokohama: "skip-waiting" }]);
    act(() => {
      registration.waiting = null;
      registration.active = next;
      next.become("activating");
    });
    // Not at `activating`: the worker is not in charge until it has activated.
    expect(host.reloads).toEqual([]);
    act(() => next.become("activated"));
    expect(host.reloads.length).toBe(1);

    // The listener the click attached is the effect's to remove, like every
    // other: one more than the hook was already watching with, and none after.
    expect(next.listening).toBe(watching + 1);
    unmount();
    expect(next.listening).toBe(0);
  });

  // A controlled tab hears BOTH events — its controller changes AND the worker
  // it asked activates — so which one it reloads on is decided once, at the
  // click, or it would reload twice.
  it("reloads a CONTROLLED tab that clicked on controllerchange, and not again on activated", async () => {
    const registration = new FakeRegistration();
    const next = new FakeWorker("installed");
    registration.active = new FakeWorker("activated");
    registration.waiting = next;
    const { host, controllerChange } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);

    act(() => result.current.applyUpdate());
    act(() => {
      registration.waiting = null;
      registration.active = next;
      next.become("activating");
    });
    act(() => controllerChange());
    expect(host.reloads.length).toBe(1);
    act(() => next.become("activated"));
    expect(host.reloads.length).toBe(1);
  });

  // **Another tab's click reloads nothing here, controlled or not.** The
  // uncontrolled half is the one the E2 fix could break: this tab also sees the
  // waiting worker activate, and it did not ask.
  it("reloads neither a controlled nor an uncontrolled tab that did not click", async () => {
    for (const controlled of [true, false]) {
      const registration = new FakeRegistration();
      const next = new FakeWorker("installed");
      registration.active = new FakeWorker("activated");
      registration.waiting = next;
      const { host, controllerChange } = fakeHost(registration);
      host.isControlled = controlled;
      const { unmount } = renderHook(() => useReadiness(host));
      await act(async () => undefined);
      act(() => {
        registration.waiting = null;
        registration.active = next;
        next.become("activating");
      });
      if (controlled) act(() => controllerChange());
      act(() => next.become("activated"));
      expect(host.reloads, controlled ? "controlled" : "uncontrolled").toEqual([]);
      unmount();
    }
  });

  // **A click with nothing waiting must not arm the reload.** Otherwise the
  // next controller change — another tab taking an update, which `skipWaiting`
  // makes registration-wide — would reload THIS tab out of a game.
  it("arms nothing when there is no update to take", async () => {
    const registration = new FakeRegistration();
    registration.active = new FakeWorker("activated");
    const { host, controllerChange } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.updateWaiting).toBe(false);
    act(() => result.current.applyUpdate());
    act(() => controllerChange());
    expect(host.reloads).toEqual([]);
  });

  // A registration that throws is a failed door with the browser's own
  // sentence, not an unhandled rejection and not a door stuck on "installing".
  it("fails with the browser's sentence when registration is refused", async () => {
    const registration = new FakeRegistration();
    const { host } = fakeHost(registration);
    host.register = vi.fn(async () => {
      throw new Error("SecurityError: the origin is not secure");
    });
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toMatch(/SecurityError/);
  });

  // The first `register()` and the retry's reach ONE wiring, so its own `live`
  // check is what stops a registration that resolves after the unmount from
  // being adopted on either path; this is that check on the first one.
  it("adopts nothing when the first registration resolves after the page has gone", async () => {
    const registration = new FakeRegistration();
    const waiting = new FakeWorker("installed");
    registration.installing = new FakeWorker("installing");
    registration.waiting = waiting;
    registration.active = new FakeWorker("activated");
    let resolve: (registration: RegistrationLike) => void = () => undefined;
    const { host } = fakeHost(registration);
    host.register = vi.fn<SwHost["register"]>(
      () =>
        new Promise<RegistrationLike>((settle) => {
          resolve = settle;
        }),
    );
    const { result, unmount } = renderHook(() => useReadiness(host));
    const { applyUpdate } = result.current;

    unmount();
    await act(async () => {
      resolve(registration);
    });
    expect(registration.listening).toBe(0);
    applyUpdate();
    expect(waiting.posted).toEqual([]);
  });
});

// **A "Try again" whose own `register()` succeeds is ADOPTED** (PR #37 F3,
// R-F3). When the FIRST `register()` is refused there is no registration to
// `update()`, so the retry registers again — and until this fix it discarded
// what that resolved to: never stored, no worker watched, no `updatefound`
// wired. The worker went on to activate while the door said "Preparing to play
// offline" until a reload, and a later update was never offered in that page.
describe("a retry after the registration itself was refused", () => {
  /** The browser's shape for the reviewer's script, whose first `/sw.js`
   *  answers 503 and every later one the real file. */
  const refused = () =>
    new Error("Failed to register a ServiceWorker: a bad HTTP response code (503) was received");

  /** A host whose first `register()` is refused and whose next one answers
   *  `next`. */
  function refusedOnce(registration: FakeRegistration, next: () => Promise<RegistrationLike>) {
    const fake = fakeHost(registration);
    fake.host.register = vi
      .fn<SwHost["register"]>()
      .mockRejectedValueOnce(refused())
      .mockImplementationOnce(next);
    return fake;
  }

  /** Refused, retried, and walked through a first install to `activated`. */
  async function retriedToReady() {
    const registration = new FakeRegistration();
    const worker = new FakeWorker("installing");
    registration.installing = worker;
    const { host } = refusedOnce(registration, async () => registration);
    const hook = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(hook.result.current.readiness).toBe("failed");
    expect(hook.result.current.reason).toMatch(/503/);

    await act(async () => {
      hook.result.current.retry();
    });
    expect(host.register).toHaveBeenCalledTimes(2);
    expect(host.register).toHaveBeenLastCalledWith(SW_URL);
    expect(hook.result.current.readiness).toBe("installing");
    expect(hook.result.current.reason).toBeNull();

    walkToActivated(registration, worker, () => hook.result.current.readiness);
    return { registration, worker, ...hook };
  }

  // The cell the review's browser reading turns into: the worker `activated`
  // and the door still "installing".
  it("reaches ready once the worker the retry registered has activated", async () => {
    const { result } = await retriedToReady();
    expect(result.current.readiness).toBe("ready");
    expect(result.current.reason).toBeNull();
    expect(result.current.updateWaiting).toBe(false);
  });

  // **Stored, not merely watched**: the click reaches the worker waiting on the
  // registration the RETRY resolved to, which `applyUpdate` can only find
  // through `registration.current`.
  it("offers a later update, and the click reaches the worker waiting behind it", async () => {
    const { registration, result } = await retriedToReady();
    const next = new FakeWorker("installing");
    act(() => registration.updateFound(next));
    expect(result.current.updateWaiting).toBe(false);
    act(() => {
      registration.installing = null;
      registration.waiting = next;
      next.become("installed");
    });
    expect(result.current.updateWaiting).toBe(true);
    expect(result.current.readiness).toBe("ready");

    act(() => result.current.applyUpdate());
    expect(next.posted).toEqual([{ yokohama: "skip-waiting" }]);
  });

  it("removes every listener the retry attached when it unmounts", async () => {
    const { registration, worker, unmount } = await retriedToReady();
    const next = new FakeWorker("installing");
    act(() => registration.updateFound(next));
    // The premise, without which zero below would say nothing: the retry's
    // wiring really is on the registration and on both of its workers.
    expect(registration.listening).toBeGreaterThan(0);
    expect(worker.listening).toBeGreaterThan(0);
    expect(next.listening).toBeGreaterThan(0);

    unmount();
    expect(registration.listening).toBe(0);
    expect(worker.listening).toBe(0);
    expect(next.listening).toBe(0);
  });

  // **A retry whose own `register()` is refused goes back to FAILED, with its
  // own sentence** (PR #37 F3 re-review, m1). After E1 this is the arm every
  // first-visit retry takes — a refused registration and a failed first
  // install alike — and a rejection it swallowed would leave the door on
  // "Preparing to play offline…" for good, which the cell "goes back to failed
  // when the retry itself is refused" forbids on the other arm.
  it("goes back to failed with the retry's own sentence when its register() is refused too", async () => {
    const registration = new FakeRegistration();
    const { host } = refusedOnce(registration, async () => {
      throw new Error("Failed to register a ServiceWorker: the retry was refused as well");
    });
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toMatch(/503/);

    await act(async () => {
      result.current.retry();
    });
    expect(host.register).toHaveBeenCalledTimes(2);
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBe(
      "Failed to register a ServiceWorker: the retry was refused as well",
    );
  });

  // A page that has gone adopts nothing. The late registration carries a
  // worker waiting behind a running one, so a registration STORED after the
  // unmount would show through `applyUpdate` even with no listener attached.
  it("adopts nothing when the retry resolves after the page has gone", async () => {
    const late = new FakeRegistration();
    const installing = new FakeWorker("installing");
    const waiting = new FakeWorker("installed");
    late.installing = installing;
    late.waiting = waiting;
    late.active = new FakeWorker("activated");
    let resolve: (registration: RegistrationLike) => void = () => undefined;
    const { host } = refusedOnce(
      late,
      () =>
        new Promise<RegistrationLike>((settle) => {
          resolve = settle;
        }),
    );
    const { result, unmount } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.readiness).toBe("failed");
    await act(async () => {
      result.current.retry();
    });
    expect(host.register).toHaveBeenCalledTimes(2);
    const { applyUpdate } = result.current;

    unmount();
    await act(async () => {
      resolve(late);
    });
    expect(late.listening).toBe(0);
    expect(installing.listening).toBe(0);
    expect(waiting.listening).toBe(0);
    applyUpdate();
    expect(waiting.posted).toEqual([]);
  });
});

// ---- the re-check (F, R-F3; spec §2.1) ------------------------------------
//
// **An open page asks again whether a release is out**: when it comes back
// into view, and every fifteen minutes while it is open. Each ask is one fetch
// of `/sw.js` — a static asset, served free — so a wording change reaches a
// tab that stays open all afternoon without a single Worker invocation. The
// browser's own check on every navigation stays; these are the two it does
// not make.

/** Say the page's visibility changed, the way a browser does: the state
 *  first, then the event. jsdom HAS `document.visibilityState` (it reads
 *  "visible"), so the hook reads the real document and a cell only moves it. */
function turnPage(state: "visible" | "hidden"): void {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
  document.dispatchEvent(new Event("visibilitychange"));
}

describe("the re-check", () => {
  afterEach(() => {
    // Back to jsdom's own getter on the prototype.
    delete (document as { visibilityState?: unknown }).visibilityState;
    vi.useRealTimers();
  });

  /** A returning player's page: release 1 running, nothing waiting. */
  const running = () => {
    const registration = new FakeRegistration();
    registration.active = new FakeWorker("activated");
    return registration;
  };

  it("asks for the worker again when the page comes back into view, and not when it goes out of it", async () => {
    const registration = running();
    let resolve: (registration: RegistrationLike) => void = () => undefined;
    const { host } = fakeHost(registration);
    host.register = vi.fn<SwHost["register"]>(
      () =>
        new Promise<RegistrationLike>((settle) => {
          resolve = settle;
        }),
    );
    renderHook(() => useReadiness(host));
    // Before the registration has resolved there is nothing to ask.
    act(() => turnPage("visible"));
    expect(registration.updates, "no registration yet, so no ask").toBe(0);
    await act(async () => {
      resolve(registration);
    });

    act(() => turnPage("hidden"));
    expect(registration.updates, "going out of view asks nothing").toBe(0);
    act(() => turnPage("visible"));
    expect(registration.updates, "coming back into view asks once").toBe(1);
  });

  it("asks every fifteen minutes while the page is open, and never after it has gone", async () => {
    vi.useFakeTimers();
    const registration = running();
    const { host } = fakeHost(registration);
    const { unmount } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(RECHECK_MS).toBe(15 * 60 * 1000);

    act(() => vi.advanceTimersByTime(RECHECK_MS - 1));
    expect(registration.updates, "not before the interval").toBe(0);
    act(() => vi.advanceTimersByTime(1));
    expect(registration.updates, "at the interval").toBe(1);
    act(() => vi.advanceTimersByTime(RECHECK_MS));
    expect(registration.updates, "and at twice it").toBe(2);

    unmount();
    act(() => vi.advanceTimersByTime(3 * RECHECK_MS));
    expect(registration.updates, "a page that has gone asks nothing").toBe(2);
  });

  // **A re-check that fails says nothing about the release this page runs.**
  // Offline, a server that did not answer, a registration the platform no
  // longer accepts: the door that read "Ready to play offline" still does, and
  // no rejection escapes to the page.
  it("leaves the door as it was, and throws nothing, when the ask is refused", async () => {
    const registration = running();
    let asked = 0;
    registration.update = async () => {
      asked += 1;
      throw new Error("NetworkError: the script could not be fetched");
    };
    const { host } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    expect(result.current.readiness).toBe("ready");

    const escaped: unknown[] = [];
    const onEscape = (reason: unknown) => escaped.push(reason);
    process.on("unhandledRejection", onEscape);
    try {
      act(() => turnPage("visible"));
      await act(async () => {
        await new Promise((r) => setTimeout(r, 0));
      });
    } finally {
      process.off("unhandledRejection", onEscape);
    }
    expect(asked, "the premise: the ask was made, and refused").toBe(1);
    expect(escaped, "no rejection escaped").toEqual([]);
    expect(result.current.readiness).toBe("ready");
    expect(result.current.reason).toBeNull();
    expect(result.current.updateWaiting).toBe(false);
  });
});

// ---- the edge's challenge at the door (security S3, W-3) -------------------
//
// **The door passes the edge's challenge once per page and retries on its
// own** (design §3.1 steps 3–6; D-2, D-3, D-14, D-17). Under the no-Access
// door Bot Fight Mode challenges a precache fetch from a browser whose
// `cf_clearance` has lapsed, and the worker names that failure "challenged" to
// the one door window it elects (`sw.ts`). The door opens the overlay it
// already has (`passChallenge`) and, once the widget passes, runs its existing
// retry: `update()` with a worker active, the returning browser's case, and a
// new `register` where the FIRST install failed. A rejected pass, and a build
// with no overlay, land on today's failed. ONE automatic pass per page, since
// the page starts installs on its own at every recheck: after it, a challenged
// install draws the button and nothing else, and only a click earns the widget
// once more. The page is its own fence: a challenge read anywhere but the door
// is a failure (P-26).

/** The worker's sentence for a challenged entry: `sw.ts`'s `CHALLENGED`, after the path. */
const CHALLENGED_REASON = "/art/manifest.json: the edge asked for a check before it would answer";
const challenged = { yokohama: "install-failed", kind: "challenged", reason: CHALLENGED_REASON } as const;

/** The overlay's handler as a spy: each call opens ONE pass, which the cell settles. A person takes a second or
 *  two in the widget, and the platform's events land meanwhile. */
function overlay() {
  const passes: { resolve: () => void; reject: (e: Error) => void }[] = [];
  const handler = vi.fn(
    () =>
      new Promise<void>((resolve, reject) => {
        passes.push({ resolve, reject });
      }),
  );
  setChallengeHandler(handler);
  return { handler, passes };
}

/** The hook, with every readiness it ever RENDERED kept: what "never failed" is a claim about. */
function mountSeeing(host: SwHost) {
  const seen: Readiness[] = [];
  const hook = renderHook(() => {
    const status = useReadiness(host);
    seen.push(status.readiness);
    return status;
  });
  return { ...hook, seen };
}

/** A returning browser's door: release 1 running and release 2's install started, as a recheck or a navigation's
 *  own check finds it; with the overlay mounted unless the cell says the build has none. */
async function returningDoor(withOverlay = true) {
  const registration = new FakeRegistration();
  registration.active = new FakeWorker("activated");
  const fake = fakeHost(registration);
  const pass = withOverlay ? overlay() : { handler: vi.fn(), passes: [] };
  const hook = mountSeeing(fake.host);
  await act(async () => undefined);
  act(() => registration.updateFound(new FakeWorker("installing")));
  expect(hook.result.current.readiness, "the premise: release 2 is installing").toBe("installing");
  return { registration, ...fake, ...pass, ...hook };
}

/** An install meeting the edge, in the platform's order: the worker posts BEFORE its install rejects, and then the
 *  dying worker goes `redundant`, both while a person is in the overlay. */
function meetsTheEdge(registration: FakeRegistration, message: (data: unknown) => void): void {
  act(() => {
    message(challenged);
    registration.installFailed();
  });
}

describe("the edge's challenge at the door", () => {
  afterEach(() => {
    resetTurnstileForTests();
    window.location.hash = "";
    delete (document as { visibilityState?: unknown }).visibilityState;
  });

  it("passes the challenge once and retries through update() when a worker is active, never reading failed", async () => {
    const { registration, host, message, handler, passes, result, seen } = await returningDoor();
    meetsTheEdge(registration, message);
    expect(handler, "the overlay opened, once").toHaveBeenCalledTimes(1);
    expect(registration.updates, "nothing is retried before the pass lands").toBe(0);
    expect(result.current.readiness, "the door keeps what it had while the overlay is up").toBe("installing");

    const next = new FakeWorker("installing");
    registration.nextInstall = next;
    await act(async () => {
      passes[0]!.resolve();
    });
    expect(registration.updates, "the retry is update(), the returning browser's case").toBe(1);
    expect(host.register, "and no second register").toHaveBeenCalledTimes(1);
    act(() => {
      registration.installing = null;
      registration.waiting = next;
      next.become("installed");
    });
    expect(result.current.readiness).toBe("ready");
    expect(result.current.updateWaiting, "release 2 waits for the door's own apply").toBe(true);
    expect(handler).toHaveBeenCalledTimes(1);
    expect(seen, "the door never read failed").not.toContain("failed");
  });

  it("passes once and registers anew when the first install failed", async () => {
    const first = new FakeRegistration();
    first.installing = new FakeWorker("installing");
    const second = new FakeRegistration();
    const worker = new FakeWorker("installing");
    second.installing = worker;
    const { host, message } = fakeHost(first);
    host.register = vi
      .fn<SwHost["register"]>()
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second);
    const { handler, passes } = overlay();
    const { result, seen } = mountSeeing(host);
    await act(async () => undefined);
    meetsTheEdge(first, message);
    expect(handler, "the overlay opened, once").toHaveBeenCalledTimes(1);
    expect(host.register, "nothing is retried before the pass lands").toHaveBeenCalledTimes(1);

    await act(async () => {
      passes[0]!.resolve();
    });
    expect(host.register, "the retry REGISTERS: a failed first install empties the registration").toHaveBeenCalledTimes(2);
    expect(first.updates).toBe(0);
    walkToActivated(second, worker, () => result.current.readiness);
    expect(result.current.readiness).toBe("ready");
    expect(handler).toHaveBeenCalledTimes(1);
    expect(seen, "the door never read failed").not.toContain("failed");
  });

  it("a rejected pass lands on failed with the reason and no retry", async () => {
    const { registration, host, message, handler, passes, result } = await returningDoor();
    meetsTheEdge(registration, message);
    await act(async () => {
      passes[0]!.reject(new Error("the check was cancelled"));
    });
    expect(handler).toHaveBeenCalledTimes(1);
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason, "the worker's sentence, which names the path and the edge").toBe(CHALLENGED_REASON);
    expect(registration.updates, "no retry").toBe(0);
    expect(host.register, "no register either").toHaveBeenCalledTimes(1);
  });

  it("with no overlay mounted the door lands on failed at once", async () => {
    const { registration, host, message, result } = await returningDoor(false);
    await act(async () => {
      message(challenged);
    });
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBe(CHALLENGED_REASON);
    expect(registration.updates, "no retry").toBe(0);
    expect(host.register).toHaveBeenCalledTimes(1);
  });

  it("a second challenged message on the same page draws the button and runs no second pass", async () => {
    const { registration, message, handler, passes, result } = await returningDoor();
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[0]!.resolve();
    });
    expect(registration.updates, "the premise: the pass's retry ran, and installed again").toBe(1);

    // The retry's own install meets the edge again.
    meetsTheEdge(registration, message);
    expect(handler, "no second widget").toHaveBeenCalledTimes(1);
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBe(CHALLENGED_REASON);

    // A recheck the page starts by itself is silent too.
    registration.nextInstall = new FakeWorker("installing");
    act(() => turnPage("visible"));
    expect(registration.updates, "the premise: the recheck asked, and installed again").toBe(2);
    meetsTheEdge(registration, message);
    expect(handler, "the recheck opened no widget").toHaveBeenCalledTimes(1);
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBe(CHALLENGED_REASON);
  });

  it("a click on Try again resets the counter so its own challenged retry may pass once more", async () => {
    const { registration, message, handler, passes, result } = await returningDoor();
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[0]!.resolve();
    });
    meetsTheEdge(registration, message);
    expect(handler, "the premise: the automatic pass is spent").toHaveBeenCalledTimes(1);
    expect(result.current.readiness).toBe("failed");

    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      result.current.retry();
    });
    expect(registration.updates, "the click's retry").toBe(2);
    meetsTheEdge(registration, message);
    expect(handler, "the click earned the widget once more").toHaveBeenCalledTimes(2);

    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[1]!.resolve();
    });
    expect(registration.updates, "and that pass retried").toBe(3);
    // Once more, and not once more for each challenge after it.
    meetsTheEdge(registration, message);
    expect(handler).toHaveBeenCalledTimes(2);
    expect(result.current.readiness).toBe("failed");
  });

  it("a challenged message at a page whose route is not the door lands on failed and runs no pass", async () => {
    for (const hash of ["#/t/abc", "#/lobby", "#/new"]) {
      const { registration, message, handler, result, unmount } = await returningDoor();
      // Read at MESSAGE time: the page mounted on the door and has left it by a hash navigation, which the worker's
      // `WindowClient.url` may not have followed.
      window.location.hash = hash;
      await act(async () => {
        message(challenged);
      });
      expect(handler, hash).not.toHaveBeenCalled();
      expect(result.current.readiness, hash).toBe("failed");
      expect(result.current.reason, hash).toBe(CHALLENGED_REASON);
      expect(registration.updates, hash).toBe(0);
      unmount();
      resetTurnstileForTests();
      window.location.hash = "";
    }
  });

  it("kind failed takes today's path, and so does a message with no kind", async () => {
    const reason = "/art/orders/O01.webp: answered 503";
    for (const said of [
      { yokohama: "install-failed", kind: "failed", reason },
      { yokohama: "install-failed", reason },
    ]) {
      const { registration, message, handler, result, unmount } = await returningDoor();
      await act(async () => {
        message(said);
      });
      expect(handler, JSON.stringify(said)).not.toHaveBeenCalled();
      expect(result.current.readiness).toBe("failed");
      expect(result.current.reason).toBe(reason);
      expect(registration.updates).toBe(0);
      unmount();
      resetTurnstileForTests();
    }
  });

  it("the automatic pass and retry request nothing under /api", async () => {
    const fetched = vi
      .spyOn(globalThis, "fetch")
      .mockImplementation(async () => new Response(null, { status: 204 }));
    const { registration, message, handler, passes, result } = await returningDoor();
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[0]!.resolve();
    });
    // And the click's path, which runs the same retry and may pass once more.
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      result.current.retry();
    });
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[1]!.resolve();
    });
    expect(handler, "the premise: two passes ran").toHaveBeenCalledTimes(2);
    expect(registration.updates, "and three retries").toBe(3);
    const paths = fetched.mock.calls.map(
      ([input]) => new URL(input instanceof Request ? input.url : String(input), window.location.href).pathname,
    );
    expect(paths.filter((p) => p.startsWith("/api")), "nothing under /api").toEqual([]);
  });

  // **A challenge that arrives while the pass runs is handed that pass**, as `passChallenge` hands every
  // concurrent challenge one: the person switched tabs and came back with the overlay up, and the recheck's
  // install met the edge as well. One widget, one retry when it lands, and the door never read failed.
  it("a challenge arriving while the pass runs shares it: one pass, one retry, never failed", async () => {
    const { registration, message, handler, passes, result, seen } = await returningDoor();
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    act(() => turnPage("visible"));
    expect(registration.updates, "the premise: the recheck installed again").toBe(1);
    meetsTheEdge(registration, message);
    expect(handler, "one widget").toHaveBeenCalledTimes(1);
    expect(result.current.readiness).toBe("installing");

    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[0]!.resolve();
    });
    expect(registration.updates, "ONE retry once the pass lands").toBe(2);
    expect(seen, "the door never read failed").not.toContain("failed");
  });

  // **A click while the pass runs** — the overlay is modal, but a keyboard can still reach "Try again" behind it.
  // Its install meets the edge with no clearance yet, and is handed the pass in flight: one retry when it lands.
  it("a click while the pass runs is handed the same pass: one retry when it lands", async () => {
    const { registration, message, handler, passes, result } = await returningDoor();
    meetsTheEdge(registration, message);
    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      result.current.retry();
    });
    expect(registration.updates, "the click's own retry").toBe(1);
    meetsTheEdge(registration, message);
    expect(handler, "the same widget").toHaveBeenCalledTimes(1);

    registration.nextInstall = new FakeWorker("installing");
    await act(async () => {
      passes[0]!.resolve();
    });
    expect(registration.updates, "ONE retry once the pass lands").toBe(2);
  });

  it("a pass that lands after the page has gone retries nothing", async () => {
    const { registration, host, message, passes, unmount } = await returningDoor();
    meetsTheEdge(registration, message);
    unmount();
    await act(async () => {
      passes[0]!.resolve();
    });
    expect(registration.updates).toBe(0);
    expect(host.register).toHaveBeenCalledTimes(1);
  });
});

// ---- the door applies a ready release itself (F, R-F3, R-FP14) -------------
//
// **The DOOR takes a waiting release without a click; nothing else does.** The
// door is the screen between games, so reloading it costs nobody a move; the
// lobby and a game keep the one passive line, and the click is the only switch
// there. The door calls `applyUpdate` — the click's own path — at most once in
// a page's life, never on a first install (the offer needs a waiting worker
// AND a running one), and never while a restore or a send is in flight.

/** A status the hook would report, with the two controls as spies. */
const statusOf = (over: Partial<ReadinessStatus> = {}): ReadinessStatus => ({
  readiness: "ready",
  reason: null,
  updateWaiting: true,
  staleTab: false,
  applyUpdate: vi.fn(),
  reload: vi.fn(),
  retry: vi.fn(),
  ...over,
});

describe("the door's own apply", () => {
  // Both are the MODULE's — one page, one life — so each cell starts a fresh one.
  beforeEach(() => resetDoorAutoApplyForTests());
  afterEach(() => resetDoorAutoApplyForTests());

  const mount = (isDoor: boolean, status: ReadinessStatus) =>
    renderHook((props: { isDoor: boolean; status: ReadinessStatus }) => useDoorAutoApply(props.isDoor, props.status), {
      initialProps: { isDoor, status },
    });

  it("applies a waiting release once, and not again on a re-render or a remount", () => {
    const status = statusOf();
    const { rerender, unmount } = mount(true, status);
    expect(status.applyUpdate).toHaveBeenCalledTimes(1);
    rerender({ isDoor: true, status: { ...status } });
    expect(status.applyUpdate, "a re-render does not apply again").toHaveBeenCalledTimes(1);
    // **Once per PAGE life, not per mount**: a memory held in the hook would
    // start again with a remount of the component that calls it, and apply
    // again.
    unmount();
    mount(true, status);
    expect(status.applyUpdate, "a remount does not apply again").toHaveBeenCalledTimes(1);
  });

  it("never applies off the door, and applies when the page arrives at it", () => {
    const status = statusOf();
    const { rerender } = mount(false, status);
    expect(status.applyUpdate, "a lobby or a game applies nothing").not.toHaveBeenCalled();
    rerender({ isDoor: false, status: { ...status } });
    expect(status.applyUpdate).not.toHaveBeenCalled();
    rerender({ isDoor: true, status });
    expect(status.applyUpdate, "the door, reached with a release waiting").toHaveBeenCalledTimes(1);
  });

  it("applies nothing when no release is waiting", () => {
    const status = statusOf({ updateWaiting: false });
    const { rerender } = mount(true, status);
    rerender({ isDoor: true, status: { ...status } });
    expect(status.applyUpdate).not.toHaveBeenCalled();
    // A tab another tab's apply handed over is STALE, not waiting: its line is
    // a plain reload, and the door does not take it by itself.
    mount(true, statusOf({ updateWaiting: false, staleTab: true }));
    expect(status.applyUpdate).not.toHaveBeenCalled();
  });

  // **A restore or a send in flight holds the apply** (R-FP14; spec §2.1, the
  // review's F-28): a reload in the middle of one would lose its report. The
  // hold is released by the control's own `finally`, and the apply follows at
  // once, with no poll. Two holds at once are two: the first to finish does
  // not release the other's.
  it("waits while a restore or a send is in flight, and applies when the last one finishes", () => {
    const status = statusOf();
    act(() => setApplyBusy(true));
    act(() => setApplyBusy(true));
    expect(applyBusy()).toBe(true);
    mount(true, status);
    expect(status.applyUpdate, "held while busy").not.toHaveBeenCalled();
    act(() => setApplyBusy(false));
    expect(applyBusy(), "one of two holds released").toBe(true);
    expect(status.applyUpdate, "still held by the other").not.toHaveBeenCalled();
    act(() => setApplyBusy(false));
    expect(applyBusy()).toBe(false);
    expect(status.applyUpdate, "applied as the last hold is released").toHaveBeenCalledTimes(1);
  });
});

// **Two tabs of one browser, one registration** (Review Focus 1; R-FP14; spec
// §2.1, the review's F-7). `skipWaiting` is registration-wide, so the door's
// apply hands EVERY tab the old worker controlled to the new one — a game tab
// included, which goes on running the old bundle out of the retained
// generation. That tab's `updateWaiting` falls to false with the waiting
// worker gone, so without a state of its own it would lose its line and have
// no way to learn it is behind. It hears a `controllerchange` it did not ask
// for: it reloads NOTHING, and draws the same line, whose click reloads it.
describe("two tabs of one browser when the door applies", () => {
  beforeEach(() => resetDoorAutoApplyForTests());
  afterEach(() => resetDoorAutoApplyForTests());

  it("reloads the door once, and leaves the game tab playing with its line until its own click", async () => {
    const registration = new FakeRegistration();
    const next = new FakeWorker("installed");
    registration.active = new FakeWorker("activated");
    registration.waiting = next;
    const door = fakeHost(registration);
    const game = fakeHost(registration);
    const doorTab = renderHook(() => {
      const status = useReadiness(door.host);
      useDoorAutoApply(true, status);
      return status;
    });
    const gameTab = renderHook(() => {
      const status = useReadiness(game.host);
      useDoorAutoApply(false, status);
      return status;
    });
    await act(async () => undefined);

    // The door took it, with no click, once; the game tab asked for nothing.
    expect(next.posted, "the door posted skip-waiting by itself, once").toEqual([{ yokohama: "skip-waiting" }]);
    expect(gameTab.result.current.updateWaiting, "the game tab saw the same release waiting").toBe(true);
    expect(gameTab.result.current.staleTab).toBe(false);

    // The platform: the new worker activates, and EVERY controlled tab is
    // handed to it — both tabs hear `controllerchange`.
    act(() => {
      registration.waiting = null;
      registration.active = next;
      next.become("activating");
    });
    act(() => {
      door.controllerChange();
      game.controllerChange();
    });
    act(() => next.become("activated"));

    expect(door.host.reloads.length, "the door reloads itself, once").toBe(1);
    expect(game.host.reloads, "the game tab reloads nothing by itself").toEqual([]);
    expect(gameTab.result.current.updateWaiting, "nothing waits any more").toBe(false);
    expect(gameTab.result.current.staleTab, "so the game tab is stale, and says so").toBe(true);
    expect(doorTab.result.current.staleTab, "the tab that asked is not stale").toBe(false);

    // Its click reloads it, and nothing else does.
    act(() => gameTab.result.current.reload());
    expect(game.host.reloads.length).toBe(1);
    expect(door.host.reloads.length, "and the door is not reloaded again").toBe(1);
  });
});
