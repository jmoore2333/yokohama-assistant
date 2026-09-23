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
import {
  archiveAnswer,
  onServerFacts,
  recallServerFacts,
  rememberArchiveOff,
  rememberServerFacts,
  resetFactsForTest,
  serverFacts,
  serverFactsTakenAt,
} from "./facts";
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

  // **The Worker's transport survives a reload** (M6.5c, spec §2.7). A stored
  // `signal` is checked against the union like every other field (review M-8)
  // and a value outside it falls back to the stream — so without `"ws"` in the
  // check, every reload would start on the stream: a 426 from the Worker (spec
  // §2.3) and a trip round the ladder before the health read put it right.
  it("remembers the socket transport across a reload", () => {
    rememberServerFacts(health({ signal: "ws" }));
    resetFactsForTest();
    expect(recallServerFacts()?.signal).toBe("ws");
  });

  // **R-6.5c.14's one writer** (spec §2.6). The archive route's 404 is the
  // deployment saying it keeps no archive — news about ONE field. The record is
  // rewritten with `archive: "off"` and everything else as it was, the time it
  // was taken included: the other fields are no fresher than before. Neither
  // older writer does that — `rememberServerFacts` needs a whole `Health` and
  // restamps it, and `patchServerFacts` writes nothing down (plan review m2).
  it("writes the archive OFF into the remembered record, keeping its age and every other field", async () => {
    const at = Date.now() - 2 * 3600_000;
    // Every field away from its default, so a rewrite that put one back shows.
    rememberServerFacts(
      health({
        archive: "dir",
        signal: "ws",
        lobby_chat: false,
        chat_text: 280,
        multiplayer: false,
        identity: "worker",
      }),
      at,
    );
    const before = serverFacts();
    let heard = 0;
    const stop = onServerFacts(() => {
      heard += 1;
    });
    try {
      rememberArchiveOff();
    } finally {
      stop();
    }
    // The screens drawing the fact — the send control, the Abandon label —
    // redraw from it.
    expect(heard).toBe(1);
    expect(serverFacts()).toEqual({ ...before, archive: "off" });
    expect(serverFactsTakenAt()).toBe(at);
    expect(archiveAnswer()).toBe("off");

    // And the WRITTEN copy, read the way a fresh page load reads it: a module
    // evaluated anew over the same storage.
    vi.resetModules();
    const reloaded = await import("./facts");
    expect(reloaded.serverFacts()).toEqual({ ...before, archive: "off" });
    expect(reloaded.serverFactsTakenAt()).toBe(at);
  });
});
