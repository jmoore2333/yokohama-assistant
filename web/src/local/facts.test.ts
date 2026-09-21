// What this browser REMEMBERS about the server, and when it asks again
// (M6.5b2, spec §2.2).
//
// **The door asks the server nothing** (R-6.5b2.7), so the boot's health read
// is gone — and the archive fact it used to set drives the whole LOCAL channel:
// the send control's render condition, "Abandon and save", and the outbox's
// entry condition. A cycle that removed the read and gave the local channel
// nothing would make the measured bug of spec §0.3 permanent for ONLINE
// sessions too: a single-player visitor would never learn `archive: "dir"`,
// would see no send control, and would have Abandon delete silently.
//
// So the answer is remembered, WITH THE TIME IT WAS TAKEN, and re-read when it
// is older than an hour. Freshness is not decoration: a browser that once read
// `"off"` would otherwise authorize Abandon's plain delete for ever after the
// deployment turned its archive on, with no mechanism that could correct it.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { api, ensureArchiveFact } from "../api";
import { recallServerFacts, rememberServerFacts, resetFactsForTest } from "./facts";
import type { Health } from "../types";
import { wire } from "../wire/contract";

const health = (over: Partial<Health>): Health => ({ ...wire.health, ...over });

describe("the server's remembered facts", () => {
  beforeEach(() => {
    resetFactsForTest();
    localStorage.clear();
  });
  afterEach(() => {
    vi.restoreAllMocks();
    // The refusing-storage cell replaces the global; the suite's own
    // `beforeEach` installs a fresh in-memory one for the next test.
    vi.unstubAllGlobals();
    resetFactsForTest();
  });

  it("asks once, remembers, and does not ask again while the answer is fresh", async () => {
    const spy = vi.spyOn(api, "health").mockResolvedValue(health({ archive: "dir" }));
    expect(await ensureArchiveFact()).toBe("dir");
    expect(await ensureArchiveFact()).toBe("dir");
    expect(spy).toHaveBeenCalledTimes(1);
  });

  // And it must be able to go stale, or a browser that once read "off" keeps
  // authorizing Abandon's plain delete for ever after the archive is turned on.
  it("revalidates an answer older than an hour", async () => {
    rememberServerFacts(health({ archive: "off" }), Date.now() - 3 * 3600_000);
    const spy = vi.spyOn(api, "health").mockResolvedValue(health({ archive: "dir" }));
    expect(await ensureArchiveFact()).toBe("dir");
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("survives a reload without asking, because the answer is remembered", () => {
    rememberServerFacts(health({ archive: "dir" }));
    resetFactsForTest(); // what a fresh page load leaves
    expect(recallServerFacts()?.archive).toBe("dir");
  });

  it("reports the archive UNKNOWN rather than off when it has never asked", async () => {
    resetFactsForTest();
    localStorage.clear();
    // NOT `{archive: "off"}`: nothing has been read, and "off" is a claim about
    // the deployment that this browser has no grounds for.
    expect(recallServerFacts()).toBeNull();
    const spy = vi.spyOn(api, "health").mockRejectedValue(new Error("no server"));
    expect(await ensureArchiveFact()).toBe("unknown");
    expect(spy).toHaveBeenCalledTimes(1);
  });

  // **A read that does not answer falls back to what is remembered**, which is
  // the other half of the rule above and the reason `"unknown"` is not simply
  // "the read failed": a browser that HAS been told `"dir"` and then goes
  // offline still knows what this deployment keeps.
  it("keeps a stale answer when the revalidation does not answer", async () => {
    rememberServerFacts(health({ archive: "dir" }), Date.now() - 3 * 3600_000);
    vi.spyOn(api, "health").mockRejectedValue(new Error("no server"));
    expect(await ensureArchiveFact()).toBe("dir");
  });

  // **The fresh answer can be overridden**, which is what Abandon needs: a
  // decision that cannot be undone reads the server again whatever its age.
  it("asks again on demand, however fresh the remembered answer is", async () => {
    rememberServerFacts(health({ archive: "off" }));
    const spy = vi.spyOn(api, "health").mockResolvedValue(health({ archive: "dir" }));
    expect(await ensureArchiveFact()).toBe("off");
    expect(spy).not.toHaveBeenCalled();
    expect(await ensureArchiveFact({ force: true })).toBe("dir");
    expect(spy).toHaveBeenCalledTimes(1);
  });

  // **A storage that throws is not an error to report** — a private window, a
  // blocked site — and the `chatFold.ts` idiom is that the record is simply not
  // remembered. The read still happens; it just happens again next time.
  it("reads and writes nothing when storage refuses, and still answers", async () => {
    const refuse = () => {
      throw new Error("storage is blocked");
    };
    // The suite's own storage is a plain object, not a `Storage` instance, so
    // the refusal is installed as the global rather than on a prototype.
    vi.stubGlobal("localStorage", {
      get length() {
        return 0;
      },
      clear: refuse,
      getItem: refuse,
      key: refuse,
      removeItem: refuse,
      setItem: refuse,
    });
    vi.spyOn(api, "health").mockResolvedValue(health({ archive: "dir" }));
    expect(recallServerFacts()).toBeNull();
    expect(await ensureArchiveFact()).toBe("dir");
  });
});
