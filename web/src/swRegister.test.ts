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
import { describe, expect, it, vi } from "vitest";
import {
  readinessOf,
  serviceWorkerWanted,
  SW_URL,
  useReadiness,
  type RegistrationLike,
  type SwHost,
  type WorkerLike,
} from "./swRegister";

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
   *  install sets `installing` before the promise resolves and fires
   *  `updatefound`, which is the Install algorithm's order. */
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
      message({ yokohama: "install-failed", reason: "/art/orders/O01.webp answered 404" });
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
      message({ yokohama: "install-failed", reason: "/art/orders/O01.webp answered 404" });
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
      message({ yokohama: "install-failed", reason: "/art/orders/O01.webp answered 404" });
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
      message({ yokohama: "install-failed", reason: "/index.html answered 502" });
      registration.installFailed();
    });
    expect(result.current.readiness).toBe("failed");
    await act(async () => {
      result.current.retry();
    });
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toMatch(/NetworkError/);
  });

  // **A tab nothing controls hears no `controllerchange`** (PR #37 F3, E2).
  // `skipWaiting` hands the new worker only the tabs the old one CONTROLLED —
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
