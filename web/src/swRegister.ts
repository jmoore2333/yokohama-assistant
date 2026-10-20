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
// `skipWaiting` is posted only when THIS tab asks — the player's click, or,
// since M6.5c2 cycle F, the door asking by itself (`useDoorAutoApply`, below);
// the asking tab waits
// until the new worker is in charge before it reloads, rather than reloading
// straight after posting and racing activation — on `controllerchange` where
// the old worker controlled it, and on the new worker's `activated` where
// nothing did, since `skipWaiting` hands over CONTROLLED tabs and an
// uncontrolled one is not promised a `controllerchange` (Chromium sends it one
// too; the branch choice is what differs, PR #37 F3, E2); and an
// update this tab did not ask for — another tab asked, and `skipWaiting` is
// registration-wide — reloads nothing. The worker's half is `sw.ts`: no
// `clients.claim()`; `activate` retains the previous generation; and the
// fetch handler SERVES out of that generation a path this release does not
// list, which is what a tab that did not click asks for (PR #37, R-F2 and
// R-F5). That last is what keeps such a tab working, and nothing on this side
// could: `skipWaiting()` hands every tab the old worker controlled to the new
// one whatever this page does, so the tab that reloads nothing is in front of
// the NEW worker, still asking for the OLD release's hashed files.
//
// **Cycle F adds three things on this side and none on the worker's** (R-F3,
// R-FP14; spec §2.1; `sw.ts`, `swRoute.ts` and `swInstall.ts` are untouched).
// An open page asks the registration again when it comes back into view and
// every `RECHECK_MS`, so a release reaches a tab that stays open. The DOOR
// applies a waiting release itself, once in a page's life and never while a
// restore or a send holds it (`useDoorAutoApply`, `setApplyBusy`); the lobby
// and a game keep the one passive line (`UpdateLine.tsx`), whose click is the
// only switch there. And a tab handed to the new worker by ANOTHER tab's
// apply — a game open beside the door — is `staleTab`: it reloads nothing,
// plays on out of the retained generation, and draws the same line, whose
// click is a plain reload. Without that state its line would vanish with the
// waiting worker, and nothing would tell the player the tab is behind.
//
// **Security S3 adds the door's pass of the edge's challenge** (W-3; design
// §3.1 steps 3–6; D-2, D-3, D-14). Under the no-Access door Bot Fight Mode
// challenges a precache fetch from a browser whose `cf_clearance` has lapsed,
// and the worker tells `kind: "challenged"` to the one window it elects among
// those loaded at the door's address (`sw.ts`, `swMessages.ts`). On the door,
// that window opens the overlay it already has (`passChallenge`) and, once the
// widget passes, runs the same retry "Try again" runs; a rejected pass, or a
// build with no overlay, lands on the failure the door drew before S3. The
// pass is ONCE per page, because the page starts installs on its own at every
// recheck below: after it a challenged install draws the button and nothing
// else, and only a click earns the widget once more. Nothing here asks
// anything under `/api` (step 7): the widget talks to the edge's own challenge
// platform, and the retry refetches the release.

import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { parseRoute } from "./route";
import type { InstallFailed, InstallFailedKind } from "./swMessages";
import { passChallenge } from "./turnstile";

/** Where the generator writes the worker, and the one path `assets.rs` answers
 *  `Cache-Control: no-store`. */
export const SW_URL = "/sw.js";

/** How often an open page asks whether a new release is out (M6.5c2 cycle F,
 *  R-F3; spec §2.1): fifteen minutes, the owner's "15 minute is good". The
 *  page also asks each time it comes back into view. Each ask is one fetch of
 *  `/sw.js`, a static asset the Worker serves free (Workers pricing, footnote
 *  3), and the browser's own check on every navigation is unchanged. */
export const RECHECK_MS = 15 * 60 * 1000;

/** The one kind the door acts on by itself: the edge asked for a check, and this
 *  window is the one the worker elected to pass it (security S3, W-3). */
const CHALLENGED: InstallFailedKind = "challenged";

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
  /** Another tab's apply handed THIS tab to the new worker (M6.5c2 cycle F,
   *  R-FP14): it heard a `controllerchange` it did not ask for, so it runs the
   *  old bundle in front of the new release, with nothing waiting any more.
   *  It says so with the same line as `updateWaiting`, and its click is a
   *  plain `reload` — there is nothing left to take. */
  readonly staleTab: boolean;
  /** Take it: post `skipWaiting`, and reload THIS tab once the new worker is in
   *  charge — whether or not the old one controlled it. */
  readonly applyUpdate: () => void;
  /** Reload this tab: the stale line's click. */
  readonly reload: () => void;
  /** Try the install again after a failure. A person's act, so it also gives
   *  the page its automatic pass of the edge's challenge back (security S3,
   *  W-3; D-14): the retry it starts may open the overlay once more. */
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
  /** Whether a worker controls this page NOW — which decides how the page
   *  hears that an update it asked for has taken charge. */
  controlled(): boolean;
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
    controlled: () => navigator.serviceWorker.controller !== null,
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

/** What the hook's effect lends its two controls. */
interface Lent {
  /** Store a registration, watch its workers, wire `updatefound`, settle. */
  adopt(reg: RegistrationLike): void;
  /** Read the stored registration's shape again. */
  settle(): void;
  /** Reload this tab once `worker` has reached `activated`. */
  reloadWhenActivated(worker: WorkerLike): void;
}

/** Register the release's worker and report what it is doing. */
export function useReadiness(host: SwHost = browserSwHost()): ReadinessStatus {
  const [readiness, setReadiness] = useState<Readiness>("none");
  const [reason, setReason] = useState<string | null>(null);
  const [updateWaiting, setUpdateWaiting] = useState(false);
  const [staleTab, setStaleTab] = useState(false);

  const hostRef = useRef(host);
  const registration = useRef<RegistrationLike | null>(null);
  /** Set by `applyUpdate` alone: how THIS tab will hear that the worker it
   *  asked for has taken charge, or `null` while it has asked nothing. It is
   *  the one thing that makes a controller change this tab's business. */
  const asked = useRef<"controllerchange" | "activated" | null>(null);
  /** Cleared by the retry, so a failure the worker named does not outlive it. */
  const failure = useRef<string | null>(null);
  /** The effect's own wiring, lent to the retry and the update offer: set by
   *  the effect and cleared by its cleanup. Through it a page whose first
   *  registration was refused, or whose first install failed, adopts the one
   *  "Try again" produces exactly as it would have adopted the first, and
   *  every listener either control attaches lands on the effect's one `undo`.
   *  A control that resolves after the unmount finds nothing to call — though
   *  each lent function's own `live` check is the guard that holds; the clear
   *  only lets go of a dead effect's closure. */
  const lent = useRef<Lent | null>(null);
  /** **The page's one automatic pass of the edge's challenge** (security S3,
   *  W-3; D-14 over D-2's words): set when the door starts it, cleared by a
   *  click on "Try again" and by nothing else. The PAGE's and not the install
   *  attempt's, since the recheck below starts an attempt every fifteen
   *  minutes and at every return into view, and a counter per attempt would
   *  reopen a cancelled overlay without end. Held here, because `App` mounts
   *  this hook once for the page's life (`App.tsx`). */
  const passedOnce = useRef(false);
  /** **While the door's own pass runs, the door keeps what it had**: the
   *  overlay is on screen, and the install it is passing for has failed by
   *  design. The dying worker's `redundant` lands during the pass, since the
   *  worker posts BEFORE its install rejects, and `settle` holds the shape's
   *  "failed" until the pass is over; a challenge that arrives meanwhile is
   *  handed the same pass, as `passChallenge` hands every concurrent challenge
   *  one, so its retry runs once. Cleared on either outcome. */
  const passing = useRef(false);
  /** The retry's body for the message reader, which the effect registers
   *  ONCE: through this ref it reaches the body of the latest render rather
   *  than the closure of the first. Written beside the body, below. */
  const retryInternalRef = useRef<() => void>(() => undefined);

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
      // stops saying "redundant" the moment the browser drops the worker. And
      // while the door's own pass runs, the shape's "failed" is held: it is
      // the install the pass is for (`passing`, above).
      if (failure.current !== null) setReadiness("failed");
      else if (!(passing.current && next.readiness === "failed")) setReadiness(next.readiness);
    };

    const watch = (worker: WorkerLike | null) => {
      if (!worker) return;
      worker.addEventListener("statechange", settle);
      undo.push(() => worker.removeEventListener("statechange", settle));
    };

    undo.push(
      swHost.onMessage((data) => {
        const said = data as { readonly [K in keyof InstallFailed]?: unknown } | null;
        if (!live || !said || said.yokohama !== "install-failed") return;
        const named = typeof said.reason === "string" ? said.reason : null;
        const failed = () => {
          failure.current = named ?? "";
          setReason(named);
          setReadiness("failed");
        };
        // **Every failure but the edge's, and the edge's anywhere but the
        // door, is the failure the door drew before S3** (W-3, P-26). The
        // route is the page's own, read when the message arrives: the worker
        // elects by `WindowClient.url`, which Chromium was measured to leave
        // at the tab's LOAD address through a hash navigation (Playwright
        // 1.62.1's Chromium, 2026-10-20), so the page is the fence, and a game
        // screen never draws an overlay mid-play. A message with no kind is a
        // failure too.
        if (said.kind !== CHALLENGED || parseRoute(window.location.hash).kind !== "door") {
          failed();
          return;
        }
        if (passing.current) return;
        // **One automatic pass per page** (D-14): after it, the button and
        // nothing else.
        if (passedOnce.current) {
          failed();
          return;
        }
        passedOnce.current = true;
        passing.current = true;
        // **The overlay's widget, then the retry "Try again" runs** (D-3):
        // `update()` with a worker active, a new `register` where the first
        // install failed. A rejection is `EDGE_CHECK` with no overlay mounted,
        // the keyless build, or `EDGE_CHECK_REPEATED` when the check did not
        // pass; either way the door names what the worker said and offers the
        // button, as it did before S3.
        passChallenge().then(
          () => {
            passing.current = false;
            if (live) retryInternalRef.current();
          },
          () => {
            passing.current = false;
            if (live) failed();
          },
        );
      }),
    );
    undo.push(
      swHost.onControllerChange(() => {
        // **Only the tab that clicked reloads.** `skipWaiting` is
        // registration-wide, so this fires in every tab the old worker
        // controlled — and in no tab it did not, which is why an uncontrolled
        // tab that clicked listens for `activated` instead.
        if (live && asked.current === "controllerchange") swHost.reload();
        // **And a tab that asked for NOTHING is now stale** (M6.5c2 cycle F,
        // R-FP14): another tab's apply — the door's own, since cycle F — handed
        // it to the new worker while it runs the old bundle, and with the
        // waiting worker gone `updateWaiting` no longer says so. It keeps
        // playing out of the retained generation and draws the line; nothing
        // reloads it but its own click. An uncontrolled tab that clicked
        // (`"activated"`) is waiting for its own reload, and is not stale.
        if (live && asked.current === null) setStaleTab(true);
      }),
    );

    /** Store the registration, watch its workers, wire `updatefound`, and say
     *  what it means — every listener onto `undo`, whichever path got here. */
    const adoptRegistration = (reg: RegistrationLike) => {
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
    };

    const reloadWhenActivated = (worker: WorkerLike) => {
      if (!live) return;
      const onState = () => {
        if (live && worker.state === "activated") swHost.reload();
      };
      worker.addEventListener("statechange", onState);
      undo.push(() => worker.removeEventListener("statechange", onState));
    };
    lent.current = { adopt: adoptRegistration, settle, reloadWhenActivated };

    // **The re-check** (M6.5c2 cycle F, R-F3; spec §2.1): the registration is
    // asked again when the page comes back into view and every `RECHECK_MS`
    // while it is open, so a release — a reworded blurb is one — reaches a tab
    // that stays open all afternoon. Wired ONCE here and not in
    // `adoptRegistration`, which a retry runs a second time: each ask reads
    // whichever registration is stored now, and there is none to ask before
    // the first `register()` resolves. A new worker it finds arrives by
    // `updatefound`, like any other. **A refused ask changes nothing**:
    // offline, or a registration the platform no longer takes, says nothing
    // about the release this page runs, so the door keeps what it said and no
    // rejection escapes. The page's visibility is read off `document` itself,
    // which jsdom has — the seam above exists for what it lacks.
    const recheck = () => {
      const current = registration.current;
      if (!live || !current) return;
      current.update().then(undefined, () => undefined);
    };
    const onVisibility = () => {
      if (document.visibilityState === "visible") recheck();
    };
    document.addEventListener("visibilitychange", onVisibility);
    undo.push(() => document.removeEventListener("visibilitychange", onVisibility));
    const timer = setInterval(recheck, RECHECK_MS);
    undo.push(() => clearInterval(timer));

    setReadiness("installing");
    void swHost
      .register(SW_URL)
      .then(adoptRegistration)
      .catch((error: unknown) => {
        if (!live) return;
        const said = error instanceof Error ? error.message : String(error);
        failure.current = said;
        setReason(said);
        setReadiness("failed");
      });

    return () => {
      live = false;
      lent.current = null;
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
    // **Which event means "it has taken charge" is decided ONCE, here.** A tab
    // the old worker controlled is handed to the new one and hears
    // `controllerchange`; a tab nothing controls — a first visit, since
    // nothing calls `clients.claim()`, or a page whose first registration was
    // retried — is not promised a `controllerchange` (Chromium sends one anyway), so it waits for the worker
    // it asked to reach `activated` (PR #37 F3, E2). A controlled tab sees
    // both events, and deciding at the click is what keeps it from reloading
    // twice.
    if (hostRef.current.controlled()) {
      asked.current = "controllerchange";
    } else {
      asked.current = "activated";
      lent.current?.reloadWhenActivated(waiting);
    }
    waiting.postMessage({ yokohama: "skip-waiting" });
  }, []);

  // **The retry's body, whoever asks for it** (security S3, W-3; D-3): the
  // click through `retry` below, and the door's automatic pass through
  // `retryInternalRef`. It leaves the page's pass counter alone, so the
  // automatic path can never give itself a second pass.
  const retryInternal = useCallback(() => {
    failure.current = null;
    setReason(null);
    setReadiness("installing");
    // **`update()` only while the registration has a WAITING or an ACTIVE
    // worker** — an UPDATE whose install failed, with the release before it
    // still there. `update()` fetches the worker again, and a changed one
    // installs again. One that installs nothing (the server's worker went back
    // to the running one's bytes) resolves with no event at all, so the door
    // is settled from the registration's shape rather than left on
    // "installing".
    //
    // **Otherwise the retry REGISTERS, and adopts what that resolves to through
    // the effect's own wiring** (PR #37 F3, R-F3, E1 and R-orch-5). Three
    // cases arrive here. Where the FIRST `register()` was refused, there is no
    // registration at all. Where the FIRST install failed, the platform has
    // marked the worker redundant, emptied the registration and removed it
    // from the registration map, and `update()` on it is refused — measured in
    // Chromium: "The object is in an invalid state", on every click until a
    // reload. And in the few milliseconds between those two — the worker posts
    // "install-failed" BEFORE its install rejects — the page's copy still
    // names the DYING worker as `installing`, which is why `installing` does
    // not count: an `update()` taken then queues behind the failing install
    // and is refused once that install has removed the registration, while a
    // `register()` queues the same way and then registers anew. (Against an
    // install really in progress, `register()` resolves with that same
    // registration — Register's shortcut: the same script URL, a newest worker
    // present — and adopting it again adds one more `updatefound` listener,
    // removed at the unmount like the first.) Before F3 the retry registered
    // and dropped the result, which left the door on "installing" after the
    // worker activated.
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
    if (reg && (reg.waiting ?? reg.active)) {
      void reg
        .update()
        .then(() => lent.current?.settle())
        .catch(said);
      return;
    }
    void hostRef.current
      .register(SW_URL)
      .then((next) => lent.current?.adopt(next))
      .catch(said);
  }, []);
  // Written at render, so the ref holds this render's body before any effect
  // runs; the body's identity never changes, and the write keeps that true if
  // it ever does.
  retryInternalRef.current = retryInternal;

  /** **"Try again"**: a person's act, which gives the page its automatic pass
   *  back before it retries (D-14). */
  const retry = useCallback(() => {
    passedOnce.current = false;
    retryInternal();
  }, [retryInternal]);

  const reload = useCallback(() => hostRef.current.reload(), []);

  return { readiness, reason, updateWaiting, staleTab, applyUpdate, reload, retry };
}

// ---- the door applies a ready release itself (M6.5c2 cycle F) -------------

/** Whether this page has applied a release by itself — the MODULE's, so the
 *  rule is once per page life and not once per mount: a memory held in the
 *  hook would start again with any remount of the component that calls it. */
let autoApplied = false;
/** How many restores and sends are in flight. A count behind a boolean
 *  setter, so two at once are two holds and the first to finish does not
 *  release the other's. */
let busyHolds = 0;
/** Every mounted `useDoorAutoApply`'s wake, so a released hold is acted on at
 *  once rather than at the next render or a poll. */
const busyWakes = new Set<() => void>();

/**
 * Hold or release the door's own apply (M6.5c2 cycle F, R-FP14; spec §2.1).
 *
 * `true` when a restore or a send starts, `false` in its `finally`: a reload
 * in the middle of one would lose its report. Holds count, so every `true`
 * is paired with its own `false`.
 */
export function setApplyBusy(busy: boolean): void {
  busyHolds = busy ? busyHolds + 1 : Math.max(0, busyHolds - 1);
  for (const wake of [...busyWakes]) wake();
}

/** Whether a restore or a send is in flight. */
export function applyBusy(): boolean {
  return busyHolds > 0;
}

/**
 * **The door applies a ready release itself** (M6.5c2 cycle F, R-F3; spec
 * §2.1): no click, because the door is the screen between games and reloading
 * it costs nobody a move. It calls `applyUpdate` — the click's own path, so
 * the two "it has taken charge" events and the one-tab rule are the click's —
 *
 * - only while `isDoor`: the lobby and a game keep the passive line, and the
 *   click is the only switch there;
 * - only with `updateWaiting`, which needs a waiting worker AND a running one
 *   (`readinessOf`), so never on a first install — and never for a tab that is
 *   merely `staleTab`, which has nothing left to take;
 * - at most once in the page's life;
 * - never while `applyBusy()`, and at once when the last hold is released.
 */
export function useDoorAutoApply(isDoor: boolean, status: ReadinessStatus): void {
  const [woken, wake] = useReducer((n: number) => n + 1, 0);
  useEffect(() => {
    busyWakes.add(wake);
    return () => {
      busyWakes.delete(wake);
    };
  }, []);
  const { updateWaiting, applyUpdate } = status;
  useEffect(() => {
    if (!isDoor || !updateWaiting || autoApplied || applyBusy()) return;
    autoApplied = true;
    applyUpdate();
  }, [isDoor, updateWaiting, applyUpdate, woken]);
}

/** A fresh page life for the cells: the once-flag cleared and every hold
 *  released. Both are the module's, so a cell that set either would leak it
 *  into the next. */
export function resetDoorAutoApplyForTests(): void {
  autoApplied = false;
  busyHolds = 0;
}
