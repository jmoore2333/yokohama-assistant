// Two platforms, two mechanisms, and neither is nagged (R-6.5b2.9, spec §2.4).
//
// **The platform is the fixture here**, which is why every cell says out loud
// what browser it is pretending to be: the module reads `navigator` and
// `matchMedia` and nothing else, so a cell that did not stub them would be
// asserting about jsdom's own identity rather than about a decision.

import { fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { stubDesktopMac, stubIosSafari, stubIpadOs, stubStandalone, stubStorage } from "./test/platform";
import {
  DISMISSED_KEY,
  captureInstallPrompt,
  dismissInstall,
  installState,
  promptInstall,
  resetInstallForTests,
} from "./install";

/** A `localStorage` whose every access raises, which is what iOS Safari with
 *  "Block All Cookies" and a sandboxed iframe both do. DIFFERENT from an
 *  absent one, and both paths are covered below. */
function throwingStorage(): void {
  vi.stubGlobal("localStorage", {
    getItem: () => {
      throw new Error("the operation is insecure");
    },
    setItem: () => {
      throw new Error("the operation is insecure");
    },
  });
}

describe("the install offer", () => {
  beforeEach(() => {
    resetInstallForTests();
    captureInstallPrompt();
  });
  afterEach(() => {
    resetInstallForTests();
  });

  it("offers a real button where the browser offers one", () => {
    fireEvent(window, new Event("beforeinstallprompt"));
    expect(installState()).toBe("available");
  });

  // **The capture is what makes the button possible at all.** An unprevented
  // `beforeinstallprompt` lets Chromium draw its own mini-infobar and the
  // event is then spent, so the door would have a button and no prompt behind
  // it.
  it("prevents the browser's own bar so the event can be used later", () => {
    const event = new Event("beforeinstallprompt", { cancelable: true });
    fireEvent(window, event);
    expect(event.defaultPrevented).toBe(true);
  });

  it("shows the iOS instruction only in Safari on iOS and only when not installed", () => {
    stubIosSafari();
    expect(installState()).toBe("ios-instructions");
    stubStandalone(); // already added to the home screen
    expect(installState()).toBe("installed");
  });

  // iPadOS 13 and later report `MacIntel`, so a platform test alone reads an
  // iPad as a desktop Mac and the one browser that cannot be offered a button
  // is offered nothing at all.
  it("reads an iPad that calls itself a Mac by its touch points", () => {
    stubIpadOs();
    expect(installState()).toBe("ios-instructions");
  });

  // A desktop Safari is `MacIntel` with NO touch points, and must not be told
  // to tap Share.
  it("says nothing on a desktop Mac", () => {
    stubDesktopMac();
    expect(installState()).toBe("none");
  });

  // **An iOS WEBVIEW has no Share sheet to tap** (review M-4). Each of these
  // passes the platform test and has no `onbeforeinstallprompt`, so without an
  // exclusion it is handed an instruction naming a control it does not have —
  // the failure the list exists to prevent.
  const webviews: [string, string][] = [
    ["Facebook's in-app browser", "[FBAN/FBIOS;FBAV/450.0.0.38.108;FBBV/1;FBDV/iPhone14,2]"],
    ["Instagram's", "Instagram 300.0.0.29.110 (iPhone14,2; iOS 17_0; en_US)"],
    ["Line's", "Line/13.14.0 LIAF"],
    // The generic shape, and the reason it is the general rule: Safari on iOS
    // carries a `Safari/` token and a bare WKWebView embed does not.
    ["a bare WKWebView embed", "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148"],
  ];
  for (const [what, ua] of webviews) {
    it(`says nothing in ${what}`, () => {
      stubIosSafari();
      vi.stubGlobal("navigator", {
        userAgent: `Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 ${ua}`,
        platform: "iPhone",
        maxTouchPoints: 5,
      });
      expect(installState()).toBe("none");
    });
  }

  // **The platform survives a later stub** (review M-3). `stubStorage` adds a
  // storage manager to whatever `navigator` currently is, and a spread that
  // copied nothing would silently drop the UA and the platform — so a cell
  // that stubs a browser and then its storage would be asserting about jsdom
  // again without saying so. This is that ordering, read through the one
  // answer that depends on the UA.
  it("keeps the platform when a later stub adds storage to it", () => {
    stubIosSafari();
    stubStorage({ persisted: async () => false });
    expect(installState()).toBe("ios-instructions");
  });

  it("never shows it again once dismissed", () => {
    stubIosSafari();
    dismissInstall();
    expect(installState()).toBe("none");
  });

  // **Remembered per BROWSER and not per page**, which is what "never
  // repeated" means: the module is reset here exactly as a reload resets it,
  // and the answer has to survive that.
  it("remembers the dismissal across a reload", () => {
    stubIosSafari();
    dismissInstall();
    resetInstallForTests();
    captureInstallPrompt();
    expect(installState()).toBe("none");
  });

  // The storage this runs in is the chat fold's: one key, and every access
  // wrapped, because a private window throws on the very first read.
  it("offers the instruction anyway when storage throws", () => {
    throwingStorage();
    stubIosSafari();
    expect(installState()).toBe("ios-instructions");
    expect(() => dismissInstall()).not.toThrow();
  });

  // **A browser with NO `localStorage` is not a browser that has dismissed
  // anything** (review I-1). The two unavailable paths are different paths: a
  // `getItem` that THROWS lands in a catch, and an ABSENT storage short-
  // circuits the read to `undefined` — which `!== null` answers true to, so a
  // browser with cookies blocked read as already-dismissed and neither
  // affordance was ever drawn. Only the throwing path had a cell.
  it("offers the instruction when there is no storage at all", () => {
    vi.stubGlobal("localStorage", undefined);
    stubIosSafari();
    expect(installState()).toBe("ios-instructions");
  });

  // **And the dismissal still works this visit** (review I-2). Where nothing
  // can be written, the module remembers in memory for the life of the page —
  // which in a private window is the whole of the session — so the control a
  // player clicks changes the screen rather than doing nothing at all.
  it("takes the instruction down this visit even when nothing can be stored", () => {
    vi.stubGlobal("localStorage", undefined);
    stubIosSafari();
    expect(installState()).toBe("ios-instructions");
    dismissInstall();
    expect(installState()).toBe("none");
  });

  it("takes it down this visit when the write THROWS", () => {
    throwingStorage();
    stubIosSafari();
    dismissInstall();
    expect(installState()).toBe("none");
  });

  // The in-memory half is a PAGE's memory and not a browser's: with nothing
  // written, a reload offers the instruction again. That is the smaller
  // failure the module's doc names, and it is now a mechanism rather than a
  // claim.
  it("offers it again on the next visit when nothing could be stored", () => {
    vi.stubGlobal("localStorage", undefined);
    stubIosSafari();
    dismissInstall();
    resetInstallForTests();
    captureInstallPrompt();
    expect(installState()).toBe("ios-instructions");
  });

  it("writes the dismissal under one key", () => {
    stubIosSafari();
    dismissInstall();
    expect(localStorage.getItem(DISMISSED_KEY)).not.toBeNull();
  });

  // **A dismissal silences the WHOLE offer, button included.** It is a
  // statement about being asked, and answering it with a different control is
  // the nagging R-6.5b2.9 rules out. The dismissal is per BROWSER, so the same
  // player on another device is offered the button as usual.
  it("stops offering the button too, once dismissed", () => {
    dismissInstall();
    fireEvent(window, new Event("beforeinstallprompt"));
    expect(installState()).toBe("none");
  });

  // Chromium fires `appinstalled` before `display-mode: standalone` is true in
  // the tab that did the installing, so the offer has to come down on the
  // event rather than on the next launch.
  it("is installed once the browser says so, without waiting for a relaunch", () => {
    fireEvent(window, new Event("beforeinstallprompt"));
    expect(installState()).toBe("available");
    fireEvent(window, new Event("appinstalled"));
    expect(installState()).toBe("installed");
  });

  it("prompts with the captured event, once, and takes the offer down", async () => {
    const event = new Event("beforeinstallprompt") as Event & { prompt?: () => Promise<void> };
    const prompt = vi.fn().mockResolvedValue(undefined);
    event.prompt = prompt;
    fireEvent(window, event);
    await promptInstall();
    expect(prompt).toHaveBeenCalledTimes(1);
    // The event is spent: a second click cannot re-use it, and the browser
    // fires a fresh one on a later visit if the app is still not installed.
    expect(installState()).toBe("none");
    await promptInstall();
    expect(prompt).toHaveBeenCalledTimes(1);
  });

  // A `prompt()` the browser refuses must not take the page down with it: the
  // control is a convenience and the app is fully usable without it.
  it("survives a prompt the browser refuses", async () => {
    const event = new Event("beforeinstallprompt") as Event & { prompt?: () => Promise<void> };
    event.prompt = vi.fn().mockRejectedValue(new Error("not allowed"));
    fireEvent(window, event);
    await expect(promptInstall()).resolves.toBeUndefined();
  });

  it("does nothing at all when nothing was captured", async () => {
    await expect(promptInstall()).resolves.toBeUndefined();
    expect(installState()).toBe("none");
  });

  // **The listener is installed ONCE**, however many door mounts ask for it —
  // a second registration would capture the same event twice and, once
  // `resetInstallForTests` stopped removing what it added, leave a listener
  // behind for the next cell in this file.
  it("captures once however many callers ask", () => {
    captureInstallPrompt();
    captureInstallPrompt();
    const event = new Event("beforeinstallprompt", { cancelable: true });
    const prevent = vi.spyOn(event, "preventDefault");
    fireEvent(window, event);
    expect(prevent).toHaveBeenCalledTimes(1);
  });
});

// ---------------------------------------------------------------------------
// The stubs themselves (review M-3).
//
// **A helper whose name promises to keep the browser it found has to be held
// to it.** `stubStorage` and `stubStandalone` ADD one field to whatever
// `navigator` currently is, and a spread over a real `Navigator` copies
// nothing — its properties live on the prototype — so against a real browser
// both helpers silently dropped the UA, the platform and the touch points.
// Harmless in the cells as they stand, and a trap for the next ordering that
// needs both.

describe("the platform stubs", () => {
  it("adds storage to the navigator it finds rather than replacing it", () => {
    const ua = navigator.userAgent;
    // The premise, asserted: jsdom really does have a UA to lose.
    expect(ua).not.toBe("");
    stubStorage({ persisted: async () => false });
    expect(navigator.userAgent).toBe(ua);
    expect((navigator as unknown as { storage?: unknown }).storage).toBeDefined();
  });

  it("adds the standalone flag to the navigator it finds", () => {
    const ua = navigator.userAgent;
    stubStandalone();
    expect(navigator.userAgent).toBe(ua);
    expect((navigator as unknown as { standalone?: boolean }).standalone).toBe(true);
  });
});
