// The BROWSER, faked — what a cell says when the thing under test reads
// `navigator` or `matchMedia` rather than the DOM.
//
// **In one place because three files read it.** `install.ts` decides from the
// platform, `storage.ts` from `navigator.storage`, and the door draws both; a
// per-file copy of these stubs would drift, and the drift would show up as a
// cell that thinks it is on an iPhone while the module thinks it is not.
//
// Everything here goes through `vi.stubGlobal`, which the suite restores after
// every cell (`unstubGlobals` in `vitest.config.ts`), so a cell that stubs a
// platform leaves none behind.

import { vi } from "vitest";

/** Whatever `navigator` currently is, as a plain object to add a field to. */
const navigatorNow = (): Record<string, unknown> => ({
  ...(globalThis.navigator as unknown as Record<string, unknown>),
});

/** jsdom has no `matchMedia` at all, so every cell that cares supplies one. */
export function stubDisplayMode(standalone: boolean): void {
  vi.stubGlobal("matchMedia", (query: string) => ({
    media: query,
    matches: standalone && query.includes("standalone"),
  }));
}

/** Safari on an iPhone: the UA and the platform a real one sends, and no
 *  `onbeforeinstallprompt` on the window — which is the structural half of the
 *  detection and the reason jsdom is a usable stand-in for WebKit here. */
export function stubIosSafari(): void {
  vi.stubGlobal("navigator", {
    userAgent:
      "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 " +
      "(KHTML, like Gecko) Version/17.0 Mobile/15E148 Safari/604.1",
    platform: "iPhone",
    maxTouchPoints: 5,
  });
  stubDisplayMode(false);
}

/** An iPad on iPadOS 13+, which reports itself as a Mac and is told apart by
 *  its touch points alone. */
export function stubIpadOs(): void {
  vi.stubGlobal("navigator", {
    userAgent:
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 " +
      "(KHTML, like Gecko) Version/17.0 Safari/605.1.15",
    platform: "MacIntel",
    maxTouchPoints: 5,
  });
  stubDisplayMode(false);
}

/** A desktop Safari: `MacIntel` with NO touch points. The cell that must not
 *  be told to tap Share. */
export function stubDesktopMac(): void {
  vi.stubGlobal("navigator", {
    userAgent:
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 " +
      "(KHTML, like Gecko) Version/17.0 Safari/605.1.15",
    platform: "MacIntel",
    maxTouchPoints: 0,
  });
  stubDisplayMode(false);
}

/** The same browser with the app already on the home screen. BOTH signals are
 *  set, because a browser answers one or the other and never reliably both:
 *  `display-mode` is the standard, `navigator.standalone` is what older iOS
 *  answers. */
export function stubStandalone(): void {
  stubDisplayMode(true);
  vi.stubGlobal("navigator", { ...navigatorNow(), standalone: true });
}

/** A `navigator.storage` this cell scripts. Fields left out are ABSENT, which
 *  is the shape of a browser that implements half of the API — Safari answered
 *  `persisted()` for two releases before it answered `estimate()`. */
export function stubStorage(storage: unknown): void {
  vi.stubGlobal("navigator", { ...navigatorNow(), storage });
}
