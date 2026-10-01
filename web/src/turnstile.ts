// Turnstile's client half (cycle F, R-F6; spec §2.6; R-FP8): how a PERSON
// passes the edge's one Managed Challenge rule.
//
// **The fence is the edge, and this is only the gate in it.** Under the
// no-Access door ONE WAF rule challenges every request outside the release's
// static names — `/api/*` and `/enter` among them — at the edge, before the
// Worker is invoked (Task 7 builds it). A browser passes it ONCE through the
// widget with pre-clearance, which sets the zone's `cf_clearance` cookie, and
// every later same-origin request — a fetch, the stream, the WebSocket
// upgrade, the archive post — rides that cookie past the rule. **So there is no
// siteverify, no token sent anywhere and no secret**: the token the widget
// yields grants nothing the cookie does not, and it is discarded here.
//
// **On demand only** (the door-unmetered fence): the script is Cloudflare's,
// cross-origin — `sw.ts` leaves it alone and the precache never sees it — and
// it is injected by `loadTurnstile` at the multiplayer button's click or at a
// challenge, never when this module is imported and never at a screen's mount.
// A build with no site key never asks for it at all.
//
// **One predicate, one seam.** `isEdgeChallenge` is the test every site that
// talks to the server asks of a response (`api.ts`'s `call` and record
// download, `sse.ts`'s stream reader; the socket's probe is a `call`).
// `passChallenge` is the seam they ask to pass it: the overlay
// (`Challenge.tsx`) registers the handler, and concurrent challenges share one
// pass, so a person sees ONE widget however many requests met the edge.
//
// This module imports nothing of `api.ts`, which imports it: the predicate and
// the seam are below the request layer, and the refusal a site builds from them
// is that site's.

import { turnstileSiteKey } from "./buildValues";

/** The widget's script: explicit rendering, and the `onload` callback below.
 *  Injected ONCE per page life by [`loadTurnstile`], never at module load. */
export const TURNSTILE_SCRIPT =
  "https://challenges.cloudflare.com/turnstile/v0/api.js?onload=__yokohamaTurnstile&render=explicit";

/** The global the script calls when it has arrived — the URL's `onload`. */
const ONLOAD = "__yokohamaTurnstile";

/** How long one challenge is given, the script's arrival included, before it
 *  is a failure. A managed widget passes a person in a second or two; an
 *  interactive one waits for a click, and ninety seconds is a person who has
 *  walked away. */
export const CHALLENGE_MS = 90_000;

/** What a site says when the edge asked for a check and nothing here can show
 *  one: a build with no site key, or no overlay mounted, or a context that
 *  must not show it now (`call`'s `interactive: false`). */
export const EDGE_CHECK = "This site asked for a check this build cannot show.";

/** What a site says when the check was shown and did not pass — a second
 *  challenge on the retried request, a widget that failed, or Cancel. */
export const EDGE_CHECK_REPEATED = "The site's check did not pass. Try again from the door.";

/** The options explicit rendering takes, as far as this client uses them. */
export interface TurnstileOptions {
  sitekey: string;
  /** Passed. The token is handed over and discarded: the cookie is the point. */
  callback: (token: string) => void;
  "error-callback": (code?: string) => void;
  "expired-callback": () => void;
}

/** The part of Cloudflare's `window.turnstile` this client calls. */
export interface TurnstileApi {
  render(container: HTMLElement, options: TurnstileOptions): string | undefined;
  remove(widgetId: string): void;
}

type Platform = Window & { turnstile?: TurnstileApi; [ONLOAD]?: () => void };

/** The script being fetched, or arrived — one per page life. */
let loading: Promise<TurnstileApi> | null = null;

/**
 * The widget's API, injecting its script the first time it is asked for.
 *
 * **ONCE**: every call while the script is on its way, and every call after it
 * arrived, is answered by the same promise and no second tag is written. A tag
 * that FAILED to load is taken away and forgotten, so the next demand — the
 * next click, the next challenge — asks again rather than inheriting a failure
 * for the life of the page; there is never more than one tag at a time.
 */
export function loadTurnstile(): Promise<TurnstileApi> {
  if (loading) return loading;
  const tag = document.createElement("script");
  const p = new Promise<TurnstileApi>((resolve, reject) => {
    const w = window as Platform;
    w[ONLOAD] = () => {
      const api = w.turnstile;
      if (api) resolve(api);
      else reject(new Error("the Turnstile script arrived without its API"));
    };
    tag.src = TURNSTILE_SCRIPT;
    tag.async = true;
    tag.addEventListener("error", () => {
      tag.remove();
      if (loading === p) loading = null;
      reject(new Error("the Turnstile script did not load"));
    });
  });
  loading = p;
  document.head.appendChild(tag);
  return p;
}

/**
 * Render the widget into `container` and wait for a person to pass it.
 *
 * Resolves when the widget's `callback` fires — the pre-clearance cookie is
 * set by then, and the token it is handed is DISCARDED. Rejects on the error
 * or expired callback, after [`CHALLENGE_MS`], when `signal` aborts (the
 * overlay's Cancel), and at once — loading nothing — in a build with no site
 * key. **The widget is removed whichever way it ends**, so a container is never
 * left holding a live one.
 */
export function challengeOnce(container: HTMLElement, signal?: AbortSignal): Promise<void> {
  const sitekey = turnstileSiteKey();
  if (!sitekey) return Promise.reject(new Error(EDGE_CHECK));
  return new Promise<void>((resolve, reject) => {
    let api: TurnstileApi | null = null;
    let widget: string | undefined;
    let over = false;
    const finish = (failure?: Error) => {
      if (over) return;
      over = true;
      clearTimeout(timer);
      signal?.removeEventListener("abort", onAbort);
      if (api && widget !== undefined) {
        try {
          api.remove(widget);
        } catch {
          /* a widget the script already took down is removed */
        }
      }
      if (failure) reject(failure);
      else resolve();
    };
    const onAbort = () => finish(new Error("the check was cancelled"));
    const timer = setTimeout(() => finish(new Error("the check timed out")), CHALLENGE_MS);
    if (signal?.aborted) {
      finish(new Error("the check was cancelled"));
      return;
    }
    signal?.addEventListener("abort", onAbort, { once: true });
    loadTurnstile().then(
      (t) => {
        if (over) return;
        api = t;
        widget = t.render(container, {
          sitekey,
          callback: () => finish(),
          "error-callback": (code) => finish(new Error(`the check failed${code ? ` (${code})` : ""}`)),
          "expired-callback": () => finish(new Error("the check expired")),
        });
      },
      (e: unknown) => finish(e instanceof Error ? e : new Error(String(e))),
    );
  });
}

/** What passes a challenge: the overlay's, while it is mounted. */
let handler: (() => Promise<void>) | null = null;
/** The pass in progress — what every concurrent challenge waits on. */
let passing: Promise<void> | null = null;
/** How many passes have landed on this page — what tells a request whose
 *  challenge is older than the clearance from one the edge refused under it. */
let passed = 0;

/** Register (or clear, with `null`) what [`passChallenge`] calls. ONE handler:
 *  there is one overlay, mounted once beside the app (`main.tsx`). */
export function setChallengeHandler(fn: (() => Promise<void>) | null): void {
  handler = fn;
}

/**
 * Pass the edge's challenge — the widget once, in the overlay.
 *
 * Rejects with [`EDGE_CHECK`] when no overlay is mounted (a build with no site
 * key mounts none), and with [`EDGE_CHECK_REPEATED`] when the overlay's check
 * did not pass. **Every challenge that arrives while one is being passed is
 * handed the SAME pass**, so the table, the health read and the listings a
 * screen asks together are one widget and not three.
 */
export function passChallenge(): Promise<void> {
  if (passing) return passing;
  const h = handler;
  if (!h) return Promise.reject(new Error(EDGE_CHECK));
  // Called NOW, so the overlay opens in the same turn the challenge was met; a
  // handler that throws is a check that did not pass, like one that rejects.
  let started: Promise<void>;
  try {
    started = h();
  } catch (e) {
    started = Promise.reject(e);
  }
  const p = started.then(
    () => {
      passed += 1;
    },
    () => {
      throw new Error(EDGE_CHECK_REPEATED);
    },
  );
  passing = p;
  const clear = () => {
    if (passing === p) passing = null;
  };
  p.then(clear, clear);
  return p;
}

/** How many checks have passed on this page. A site reads it before it sends:
 *  a challenge answered to a request that went out BEFORE a pass that has
 *  since landed is older than the clearance, and the retry needs no widget —
 *  so a screen whose requests straddle the pass still shows ONE. */
export function passesSoFar(): number {
  return passed;
}

/** Whether `res` is the EDGE's Managed Challenge: Cloudflare answers a
 *  challenged fetch `403` with `cf-mitigated: challenge` (readable by a
 *  same-origin fetch). Both, and never one of them: a 403 is also this API's
 *  own refusal of a locked table or a stranger's seat. */
export function isEdgeChallenge(res: Response): boolean {
  return res.status === 403 && res.headers.get("cf-mitigated") === "challenge";
}

/** Put the module back where a fresh page load finds it — for the CELLS. The
 *  script tag itself is the document's, and a cell that injected one removes
 *  it. */
export function resetTurnstileForTests(): void {
  loading = null;
  handler = null;
  passing = null;
  passed = 0;
  delete (window as Platform)[ONLOAD];
}
