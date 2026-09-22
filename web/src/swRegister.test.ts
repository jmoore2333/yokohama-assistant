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
  private readonly found = new Set<() => void>();
  addEventListener(_type: "updatefound", listener: () => void): void {
    this.found.add(listener);
  }
  removeEventListener(_type: "updatefound", listener: () => void): void {
    this.found.delete(listener);
  }
  async update(): Promise<void> {
    this.updates += 1;
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
  const host: SwHost & { reloads: number[] } = {
    wanted,
    reloads,
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
  it("names the failure the worker reported and retries on the control", async () => {
    const registration = new FakeRegistration();
    const worker = new FakeWorker("installing");
    registration.installing = worker;
    const { host, message } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => message({ yokohama: "install-failed", reason: "/art/orders/O01.webp answered 404" }));
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toBe("/art/orders/O01.webp answered 404");
    await act(async () => {
      result.current.retry();
    });
    expect(registration.updates).toBe(1);
    expect(result.current.readiness).toBe("installing");
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
    registration.installing = new FakeWorker("installing");
    registration.update = async () => {
      throw new Error("NetworkError: the script could not be fetched");
    };
    const { host, message } = fakeHost(registration);
    const { result } = renderHook(() => useReadiness(host));
    await act(async () => undefined);
    act(() => message({ yokohama: "install-failed", reason: "/index.html answered 502" }));
    expect(result.current.readiness).toBe("failed");
    await act(async () => {
      result.current.retry();
    });
    expect(result.current.readiness).toBe("failed");
    expect(result.current.reason).toMatch(/NetworkError/);
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
});
