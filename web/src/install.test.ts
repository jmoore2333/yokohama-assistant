// Two platforms, two mechanisms, and neither is nagged (R-6.5b2.9, spec §2.4).
//
// **The platform is the fixture here**, which is why every cell says out loud
// what browser it is pretending to be: the module reads `navigator` and
// `matchMedia` and nothing else, so a cell that did not stub them would be
// asserting about jsdom's own identity rather than about a decision.

import { fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { stubDesktopMac, stubIosSafari, stubIpadOs, stubStandalone } from "./test/platform";
import {
  DISMISSED_KEY,
  captureInstallPrompt,
  dismissInstall,
  installState,
  promptInstall,
  resetInstallForTests,
} from "./install";

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
    vi.stubGlobal("localStorage", {
      getItem: () => {
        throw new Error("the operation is insecure");
      },
      setItem: () => {
        throw new Error("the operation is insecure");
      },
    });
    stubIosSafari();
    expect(installState()).toBe("ios-instructions");
    expect(() => dismissInstall()).not.toThrow();
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
