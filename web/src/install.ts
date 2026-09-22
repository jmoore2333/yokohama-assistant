// Getting the app onto a home screen (M6.5b2, R-6.5b2.9; spec §2.4).
//
// **Two platforms, two mechanisms, and neither is nagged.** Chromium fires
// `beforeinstallprompt`, which this module captures so the door can offer a
// real Install button at a moment the player chose. WebKit fires nothing and
// offers no programmatic install at all, so the door shows one short
// instruction naming Share and Add to Home Screen — dismissible, remembered
// per browser the way the chat fold is, never a modal and never repeated.
//
// **The manifest and the icons are NOT here.** They are static files the page
// links (`web/public/manifest.webmanifest`, `web/public/icons/`), shipped and
// precached by the release's own worker. This module is the LOGIC over them
// and touches no asset.
//
// **How iOS is detected, and why it is two signals rather than one.**
//
//   - A UA/platform read alone is the wrong shape twice over. iPadOS 13 and
//     later report `platform: "MacIntel"` and a desktop Safari UA, so a
//     platform test reads an iPad as a Mac; the touch points are what tell
//     them apart (a desktop Mac reports 0, an iPad reports 5). And a UA is a
//     string any browser may choose to send.
//   - So the corroborating signal is STRUCTURAL: `onbeforeinstallprompt` is a
//     Chromium-only window property, and its absence is the same fact the
//     instruction exists for — this browser will never offer a button. It
//     costs nothing, it cannot be spoofed by a UA string, and it is the
//     property the other arm of this module is built on.
//
// Both are required. Neither alone would do: the property is absent in every
// WebKit and every Gecko, desktop included, and the UA is what says the share
// sheet in question is a phone's.
//
// **Every storage access is wrapped**, `chatFold.ts`'s shape: a private window
// throws on the very first read, and a browser that will not remember the
// dismissal must still show the instruction rather than take the door down.

import { useEffect, useState } from "react";
import { readItem, writeItem } from "./browserStorage";

/** The one key this module writes, `chatFold.ts`'s single-key idiom. */
export const DISMISSED_KEY = "yokohama.install.dismissed";

/** What the door should offer.
 *
 *  `"none"` covers three different browsers — one that has already been
 *  dismissed, one that cannot install at all, and one whose prompt has been
 *  spent — and that is deliberate: the door's answer to all three is the same
 *  and drawing three different silences would be three ways of saying nothing. */
export type InstallState = "available" | "ios-instructions" | "installed" | "none";

/** The event Chromium fires. `prompt()` is the whole of what it is kept for. */
interface InstallPromptEvent extends Event {
  prompt?: () => Promise<unknown>;
}

let captured: InstallPromptEvent | null = null;
/** The browser said the app was installed. Read BEFORE `display-mode`, because
 *  the tab that did the installing is not itself standalone. */
let installedSeen = false;
/** Everything [`captureInstallPrompt`] wired, so a reset is total. */
let undo: (() => void)[] = [];
const listeners = new Set<() => void>();

const notify = (): void => {
  for (const f of [...listeners]) f();
};

/** Dismissed on THIS PAGE, whether or not the browser would remember it.
 *
 *  **The half that makes the control work where nothing can be written**
 *  (review I-2). `dismissInstall` used to write, swallow the failure and tell
 *  its listeners — who re-read a storage that had stored nothing and drew the
 *  instruction again. A player clicked a control and the page did not change.
 *  In a private window the life of this flag IS the life of the session. */
let dismissedInPage = false;

/** Whether the offer has been waved away, by either memory.
 *
 *  **`readItem` and not `storage?.getItem(…) !== null`** (review I-1): an
 *  absent storage short-circuits the member expression to `undefined`, and
 *  `undefined !== null` is TRUE — so a browser with cookies blocked read as a
 *  browser that had already dismissed the offer, and neither affordance was
 *  ever drawn. The shared accessor answers `null` for every unreadable
 *  storage, which is the answer "nothing is stored under that key". */
function dismissed(): boolean {
  return dismissedInPage || readItem(DISMISSED_KEY) !== null;
}

/** `navigator`, whatever this browser has, as something safe to read from.
 *
 *  A plain read, because a page that has been handed a partial `navigator` —
 *  which a cell does, and a hardened browser extension can — must answer
 *  "cannot install" rather than throw on the door's first paint. */
function nav(): Record<string, unknown> {
  try {
    return (globalThis.navigator ?? {}) as unknown as Record<string, unknown>;
  } catch {
    return {};
  }
}

/** Already on a home screen, by either of the two signals a browser may give:
 *  `display-mode: standalone` is the standard one, `navigator.standalone` is
 *  what older iOS answers and what some installed WebKit apps still only
 *  answer. */
function standalone(): boolean {
  try {
    const mm = (globalThis as { matchMedia?: (q: string) => { matches: boolean } }).matchMedia;
    if (mm && mm("(display-mode: standalone)").matches) return true;
  } catch {
    /* no `matchMedia`, or a browser that refuses the query */
  }
  return nav().standalone === true;
}

/** Safari (or another WebKit browser) on iOS or iPadOS, not already installed.
 *
 *  The two signals are the file header's. The in-app wrappers are excluded by
 *  name because the instruction names SAFARI'S share sheet: Chrome and Firefox
 *  on iOS have their own menus and their own wording, and an instruction that
 *  names the wrong control is worse than none. */
function iosSafari(): boolean {
  const n = nav();
  const ua = String(n.userAgent ?? "");
  const platform = String(n.platform ?? "");
  const touch = Number(n.maxTouchPoints ?? 0);
  const ios =
    /iPad|iPhone|iPod/.test(platform) ||
    /iPad|iPhone|iPod/.test(ua) ||
    // iPadOS 13+: a Mac platform with a touchscreen is an iPad.
    (platform === "MacIntel" && touch > 1);
  if (!ios) return false;
  // **Another browser's iOS app**: each has its own menu and its own wording,
  // and an instruction naming the wrong control is worse than none.
  if (/CriOS|FxiOS|EdgiOS|OPiOS|YaBrowser|DuckDuckGo/.test(ua)) return false;
  // **An in-app WEBVIEW has no share sheet at all** (review M-4), so an
  // instruction there names a control that does not exist. Named first, then
  // the general rule.
  if (/FBAN|FBAV|FBIOS|Instagram|\bLine\/|MicroMessenger|Twitter|Snapchat/i.test(ua)) return false;
  // **The general rule, and the reason it is this one**: Safari on iOS carries
  // a `Safari/` token and a bare `WKWebView` embed does not. A home-screen web
  // app drops the token too — and is already answered `"installed"` above, so
  // the only thing left here without it is an embed.
  if (!/Safari\//.test(ua)) return false;
  // The structural half: a browser that HAS this property is Chromium and will
  // offer a button of its own.
  return !("onbeforeinstallprompt" in globalThis);
}

/**
 * Start listening (idempotent).
 *
 * **Called from `main.tsx` AND from the door's own hook.** Chromium fires
 * `beforeinstallprompt` once, early, and a listener attached later hears
 * nothing — so the entry point is where it belongs. The hook asks too, because
 * a page that mounted the door without going through `main.tsx` (every
 * component cell) would otherwise never be listening at all, and the flag is
 * what keeps the second caller from wiring a second listener.
 */
export function captureInstallPrompt(): void {
  if (undo.length > 0) return;
  const target = globalThis as unknown as Partial<EventTarget>;
  if (typeof target.addEventListener !== "function") return;

  const onPrompt = (event: Event): void => {
    // **Prevented, and that is what makes the button possible.** Left alone,
    // Chromium draws its own bar and the event is spent; the door would then
    // have a control with nothing behind it.
    event.preventDefault();
    captured = event as InstallPromptEvent;
    notify();
  };
  const onInstalled = (): void => {
    installedSeen = true;
    captured = null;
    notify();
  };
  globalThis.addEventListener("beforeinstallprompt", onPrompt);
  globalThis.addEventListener("appinstalled", onInstalled);
  undo = [
    () => globalThis.removeEventListener("beforeinstallprompt", onPrompt),
    () => globalThis.removeEventListener("appinstalled", onInstalled),
  ];
}

/**
 * What the door should offer, in the ORDER the reasons rank.
 *
 * 1. **Installed** beats everything: there is nothing to offer, and a browser
 *    running inside the installed app must not be told to install it.
 * 2. **Dismissed** silences the WHOLE offer, button included. A dismissal is a
 *    statement about being asked, and re-offering after one is the nagging
 *    R-6.5b2.9 rules out.
 * 3. **A captured prompt** is a real control.
 * 4. **iOS Safari** gets the instruction.
 *
 * **Steps 2 and 3 are in this order deliberately, and the cost is named.** The
 * other order — the button offered even after a dismissal — was written first
 * on the reasoning that the dismissal was about one sentence on one platform.
 * The cost of THIS order is a Chromium browser that has somehow stored the key
 * never being offered its button again; the cost of the other is a player who
 * said "not now" being asked again by a different control. The second is the
 * one the ruling forbids, and the first is close to unreachable: the dismiss
 * control is drawn only in the `ios-instructions` arm, which needs the absence
 * of the very property Chromium fires the prompt from.
 */
export function installState(): InstallState {
  if (installedSeen || standalone()) return "installed";
  if (dismissed()) return "none";
  if (captured) return "available";
  if (iosSafari()) return "ios-instructions";
  return "none";
}

/** Never again, on this browser — and never again on this PAGE whatever the
 *  browser does with the write.
 *
 *  A browser that will not remember shows the line again on the next visit,
 *  which is a smaller failure than a door that did not draw. That sentence is
 *  now a mechanism rather than a claim: the flag is what holds the dismissal
 *  when the write does not land, and it is a page's memory, so a reload really
 *  does offer it again. */
export function dismissInstall(): void {
  dismissedInPage = true;
  writeItem(DISMISSED_KEY, "1");
  notify();
}

/**
 * Show the browser's own install dialog.
 *
 * **The event is spent by a single use**, so it is dropped before the prompt
 * is even shown: a second click cannot re-use it, and Chromium fires a fresh
 * one on a later visit if the app is still not installed. The offer therefore
 * comes down on the click rather than on the answer — which is also the only
 * honest thing to draw, since the dialog's outcome is not reported here.
 */
export async function promptInstall(): Promise<void> {
  const event = captured;
  if (!event) return;
  captured = null;
  notify();
  try {
    await event.prompt?.();
  } catch {
    // A browser may refuse to show it (already installed, not user-initiated,
    // the event already used). The control is a convenience; the app works.
  }
}

/** Told when the answer may have changed. Returns its own removal. */
export function onInstallChange(f: () => void): () => void {
  listeners.add(f);
  return () => listeners.delete(f);
}

/** [`installState`] as a React value, re-read whenever it may have moved.
 *
 *  It asks for the capture as well, so a door mounted on a page whose entry
 *  point did not is still listening — see [`captureInstallPrompt`]. */
export function useInstallState(): InstallState {
  const [state, setState] = useState<InstallState>(() => installState());
  useEffect(() => {
    captureInstallPrompt();
    // Read again: the platform answer is computed from globals that a mount
    // between render and effect may have changed, and the first paint's value
    // was a guess about the same browser.
    setState(installState());
    return onInstallChange(() => setState(installState()));
  }, []);
  return state;
}

/**
 * Forget everything this module holds — **for the CELLS, and named so.**
 *
 * The captured event, the installed flag and the window listeners are a page's
 * state with a page's lifetime, so there is no production caller: a page done
 * with the offer is a page that is closing. A test file gets one module
 * registry for all its cells, and a listener left behind by one would capture
 * the next one's event.
 */
export function resetInstallForTests(): void {
  for (const f of undo) f();
  undo = [];
  captured = null;
  installedSeen = false;
  dismissedInPage = false;
  listeners.clear();
}
