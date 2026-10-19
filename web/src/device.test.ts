// The device id (the auth cycle, Task 6; design §4.4 as the plan corrected it):
// sixteen lowercase hex digits this browser draws once and sends as
// `X-Yokohama-Device` on every archive post, so a game finished with no one
// signed in files under one prefix per browser rather than one per game.
//
// **Kept in `localStorage`, never the outbox's `META` store**: that store is
// keyed by GAME id and swept by `forgetGame` and the backups, so a device row
// there would be swept or exported. A browser that clears its storage is a new
// device, which is fine.
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DEVICE_KEY, deviceId, resetDeviceForTests } from "./device";

const SIXTEEN_HEX = /^[0-9a-f]{16}$/;

describe("deviceId", () => {
  beforeEach(() => {
    localStorage.removeItem(DEVICE_KEY);
    resetDeviceForTests();
  });
  afterEach(() => {
    resetDeviceForTests();
  });

  // Mutants: a draw on every call (the two calls differ, and the second draw
  // is counted); the value not written down (storage holds nothing); a value
  // kept only in the module's memory (a fresh page, the memory reset, draws
  // again).
  it("draws sixteen lowercase hex once and keeps it", () => {
    const draws = vi.spyOn(crypto, "getRandomValues");
    const first = deviceId();
    expect(first).toMatch(SIXTEEN_HEX);
    expect(deviceId(), "two calls are one device").toBe(first);
    expect(localStorage.getItem(DEVICE_KEY), "the browser holds it").toBe(first);
    expect(draws, "drawn once").toHaveBeenCalledTimes(1);
    // A page loaded later has none of this module's memory and reads the same
    // device off the browser.
    resetDeviceForTests();
    expect(deviceId()).toBe(first);
    expect(draws, "and never drawn again").toHaveBeenCalledTimes(1);
  });

  // The Worker reads a header that is not exactly sixteen lowercase hex as
  // absent (Task 5's `isDeviceId`), so a stored value of any other shape would
  // file every game under the game's own id for ever: it is drawn again.
  // Mutants: a fifteen-hex value kept; an uppercase value kept; a throwing
  // storage reaching the caller; a per-page value drawn on every call.
  it("re-draws a stored value that is not sixteen hex, and survives a storage that throws", () => {
    for (const stale of ["0123456789abcde", "0123456789ABCDEF", "0123456789abcdef0", "", "not a device id!"]) {
      localStorage.setItem(DEVICE_KEY, stale);
      const drawn = deviceId();
      expect(drawn, JSON.stringify(stale)).toMatch(SIXTEEN_HEX);
      expect(drawn, JSON.stringify(stale)).not.toBe(stale);
      expect(localStorage.getItem(DEVICE_KEY), JSON.stringify(stale)).toBe(drawn);
    }

    // A private window, a blocked site: every read and write throws. The id is
    // then this page's own — one value for the page's life — and nothing throws.
    // (A storage of its own, not a spy on `Storage.prototype`: the setup file
    // stands a plain object in for `localStorage` in every cell.)
    const refused = vi.fn(() => {
      throw new DOMException("storage refused", "SecurityError");
    });
    vi.stubGlobal("localStorage", {
      getItem: refused,
      setItem: refused,
      removeItem: refused,
      clear: refused,
      key: refused,
      length: 0,
    });
    const page = deviceId();
    expect(refused, "premise: the storage was asked, and refused").toHaveBeenCalled();
    expect(page).toMatch(SIXTEEN_HEX);
    expect(deviceId(), "one value for the page").toBe(page);
  });
});
