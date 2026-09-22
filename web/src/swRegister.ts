// **Registering the release's worker, and what the door says about it**
// (M6.5b2, spec §2.3 and §4; R-6.5b2.8).
//
// Registration lives in a HOOK and not in `main.tsx`, because the state the
// door reports and the registration that produces it are one thing: a
// `register()` in the entry point would leave the hook hunting for the
// registration afterwards with `getRegistration()`, racing the call it was
// trying to observe. The cost is that the worker is registered when `App`
// mounts rather than when the module loads, which is the same tick in practice.
//
// **The update policy's page half is here, and it is three rules** (spec §2.3):
// `skipWaiting` is posted only on the player's click; the clicking tab waits
// for `controllerchange` before it reloads, rather than reloading straight
// after posting and racing activation; and a `controllerchange` this tab did
// not ask for — another tab clicked, and `skipWaiting` is registration-wide —
// reloads nothing. The worker's half is `sw.ts`: no `clients.claim()`, and
// `activate` retains the previous generation.

import { useCallback, useEffect, useRef, useState } from "react";

/** Where the generator writes the worker, and the one path `assets.rs` answers
 *  `Cache-Control: no-store`. */
export const SW_URL = "/sw.js";

/** What the door reports about the release.
 *
 *  `"none"` is a development build, a browser without service workers, or a
 *  page the e2e has flagged — all three are "there is nothing to say", which is
 *  not the same as a failure and must not read as one. */
export type Readiness = "installing" | "ready" | "failed" | "none";

/** Everything the door needs from this hook. */
export interface ReadinessStatus {
  readonly readiness: Readiness;
  /** What went wrong, in the worker's own words, or `null` when it did not say
   *  — spec §4 asks for a NAMED failure. */
  readonly reason: string | null;
  /** A new release is installed and waiting behind the running one. */
  readonly updateWaiting: boolean;
  /** Take it: post `skipWaiting` and reload THIS tab once it has taken over. */
  readonly applyUpdate: () => void;
  /** Try the install again after a failure. */
  readonly retry: () => void;
}

// The three platform objects this hook touches, named as the narrowest thing it
// uses rather than as `ServiceWorker*`. That is what lets a cell drive the
// lifecycle: jsdom has no `navigator.serviceWorker` at all, so a hook reaching
// for the real one directly could only ever be tested by not running.

export interface WorkerLike {
  readonly state: string;
  addEventListener(type: "statechange", listener: () => void): void;
  removeEventListener(type: "statechange", listener: () => void): void;
  postMessage(message: unknown): void;
}

export interface RegistrationLike {
  readonly installing: WorkerLike | null;
  readonly waiting: WorkerLike | null;
  readonly active: WorkerLike | null;
  addEventListener(type: "updatefound", listener: () => void): void;
  removeEventListener(type: "updatefound", listener: () => void): void;
  update(): Promise<unknown>;
}

export interface SwHost {
  /** Whether this build, this browser and this page want a worker at all. */
  readonly wanted: boolean;
  register(url: string): Promise<RegistrationLike>;
  /** Returns its own removal. */
  onControllerChange(listener: () => void): () => void;
  /** Returns its own removal. */
  onMessage(listener: (data: unknown) => void): () => void;
  reload(): void;
}

/**
 * Whether to register at all.
 *
 * **Production only**, because Vite's dev server serves no `dist`: a worker
 * registered there would precache files that do not exist and then serve a
 * stale module tree from what it did manage to cache.
 *
 * **And never when the page carries the flag.** `npm run e2e` runs
 * `build:client`, which IS a production build, so without this every one of the
 * walk's existing cells would register a worker and start a ten-megabyte
 * precache per context, and every reload-based cell would start reading a
 * cache-first navigation. `e2e/sw.ts` sets it through `addInitScript`, and
 * `e2e/every_spec_declares_its_worker.spec.ts` is what keeps that convention
 * from being one anybody has to remember.
 */
export function serviceWorkerWanted(
  win: { readonly __YOKOHAMA_NO_SW__?: unknown; readonly navigator: { readonly serviceWorker?: unknown } },
  prod: boolean,
): boolean {
  if (!prod) return false;
  if (win.__YOKOHAMA_NO_SW__) return false;
  return Boolean(win.navigator.serviceWorker);
}

/** A registration's shape, as this hook reads it. */
export interface RegSnapshot {
  readonly installingState: string | null;
  readonly hasWaiting: boolean;
  readonly hasActive: boolean;
}

/**
 * What a registration's shape means for the door.
 *
 * Pure, and it is where the policy is: the hook only wires events and asks this
 * again. `"redundant"` on the installing worker is the lifecycle's word for
 * "the install threw", which is what an all-or-nothing install does on one bad
 * file.
 *
 * **The offer needs a waiting worker AND a running one.** A waiting worker with
 * nothing active is a first install that has not activated yet, and offering to
 * reload into it would be offering the player nothing.
 */
export function readinessOf(snapshot: RegSnapshot): {
  readiness: Readiness;
  updateWaiting: boolean;
} {
  const updateWaiting = snapshot.hasWaiting && snapshot.hasActive;
  const state = snapshot.installingState;
  if (state === "redundant") return { readiness: "failed", updateWaiting };
  if (state !== null && state !== "installed" && state !== "activating" && state !== "activated") {
    return { readiness: "installing", updateWaiting };
  }
  if (state !== null || snapshot.hasActive || snapshot.hasWaiting) {
    return { readiness: "ready", updateWaiting };
  }
  return { readiness: "installing", updateWaiting };
}

/** The real browser, behind the seam above. */
export function browserSwHost(): SwHost {
  const wanted =
    typeof window !== "undefined" && serviceWorkerWanted(window, import.meta.env.PROD);
  return {
    wanted,
    register: (url) =>
      navigator.serviceWorker.register(url) as unknown as Promise<RegistrationLike>,
    onControllerChange(listener) {
      navigator.serviceWorker.addEventListener("controllerchange", listener);
      return () => navigator.serviceWorker.removeEventListener("controllerchange", listener);
    },
    onMessage(listener) {
      const wrapped = (event: MessageEvent) => listener(event.data);
      navigator.serviceWorker.addEventListener("message", wrapped);
      return () => navigator.serviceWorker.removeEventListener("message", wrapped);
    },
    reload: () => window.location.reload(),
  };
}

/** Register the release's worker and report what it is doing. */
export function useReadiness(host: SwHost = browserSwHost()): ReadinessStatus {
  const [readiness, setReadiness] = useState<Readiness>("none");
  const [reason, setReason] = useState<string | null>(null);
  const [updateWaiting, setUpdateWaiting] = useState(false);

  const hostRef = useRef(host);
  const registration = useRef<RegistrationLike | null>(null);
  /** Set by `applyUpdate` alone: the one thing that makes a controller change
   *  this tab's business. */
  const asked = useRef(false);
  /** Cleared by the retry, so a failure the worker named does not outlive it. */
  const failure = useRef<string | null>(null);

  useEffect(() => {
    const swHost = hostRef.current;
    if (!swHost.wanted) return;
    let live = true;
    /** Everything wired to a worker or a registration, so the unmount is total. */
    const undo: (() => void)[] = [];

    const settle = () => {
      const current = registration.current;
      if (!live || !current) return;
      const next = readinessOf({
        installingState: current.installing?.state ?? null,
        hasWaiting: current.waiting !== null,
        hasActive: current.active !== null,
      });
      setUpdateWaiting(next.updateWaiting);
      // A failure the worker REPORTED outranks the shape, because the shape
      // stops saying "redundant" the moment the browser drops the worker.
      setReadiness(failure.current !== null ? "failed" : next.readiness);
    };

    const watch = (worker: WorkerLike | null) => {
      if (!worker) return;
      worker.addEventListener("statechange", settle);
      undo.push(() => worker.removeEventListener("statechange", settle));
    };

    undo.push(
      swHost.onMessage((data) => {
        const said = data as { yokohama?: unknown; reason?: unknown } | null;
        if (!live || !said || said.yokohama !== "install-failed") return;
        failure.current = typeof said.reason === "string" ? said.reason : "";
        setReason(typeof said.reason === "string" ? said.reason : null);
        setReadiness("failed");
      }),
    );
    undo.push(
      swHost.onControllerChange(() => {
        // **Only the tab that clicked reloads.** `skipWaiting` is
        // registration-wide, so this fires in every tab of the origin.
        if (live && asked.current) swHost.reload();
      }),
    );

    setReadiness("installing");
    void swHost
      .register(SW_URL)
      .then((reg) => {
        if (!live) return;
        registration.current = reg;
        watch(reg.installing);
        watch(reg.waiting);
        const onUpdateFound = () => {
          watch(reg.installing);
          settle();
        };
        reg.addEventListener("updatefound", onUpdateFound);
        undo.push(() => reg.removeEventListener("updatefound", onUpdateFound));
        settle();
      })
      .catch((error: unknown) => {
        if (!live) return;
        const said = error instanceof Error ? error.message : String(error);
        failure.current = said;
        setReason(said);
        setReadiness("failed");
      });

    return () => {
      live = false;
      for (const off of undo) off();
    };
  }, []);

  const applyUpdate = useCallback(() => {
    const waiting = registration.current?.waiting;
    // **`asked` is set only when there really is something to take.** It is
    // what makes a `controllerchange` this tab's business, and a click with no
    // waiting worker that armed it anyway would reload this tab the next time
    // ANOTHER one took an update — which is precisely the two-tab behaviour
    // spec §2.3 is written to avoid.
    if (!waiting) return;
    asked.current = true;
    waiting.postMessage({ yokohama: "skip-waiting" });
  }, []);

  const retry = useCallback(() => {
    failure.current = null;
    setReason(null);
    setReadiness("installing");
    // `update()` and not `register()`: after a failed install the registration
    // has no newest worker, so the update algorithm's byte-for-byte shortcut
    // does not apply and it really does install again.
    //
    // **A REJECTED retry goes back to failed rather than staying "installing"**
    // — that is the same defect the install's own deadline exists to prevent,
    // one level up: a door that says "Preparing to play offline" for ever has
    // nothing on it a player can act on.
    const said = (error: unknown) => {
      const text = error instanceof Error ? error.message : String(error);
      failure.current = text;
      setReason(text);
      setReadiness("failed");
    };
    const reg = registration.current;
    if (reg) void reg.update().catch(said);
    else void hostRef.current.register(SW_URL).catch(said);
  }, []);

  return { readiness, reason, updateWaiting, applyUpdate, retry };
}
