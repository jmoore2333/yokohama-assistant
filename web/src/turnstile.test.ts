// Turnstile's client half, alone (cycle F, R-F6; spec §2.6; R-FP8).
//
// **The widget is a third party's, and so is its shape**: `window.turnstile`
// below is a FAKE held beside these cells — the API Cloudflare documents for
// explicit rendering (`render`, `remove`, the callbacks) — and never a wire
// payload. The script tag is never fetched: jsdom loads no external script, so
// the cells play the platform's part themselves, setting the global and
// calling the `onload` callback the URL names.
//
// What is pinned: the script is injected ON DEMAND and ONCE (the door costs
// nothing until a click or a challenge — the door-unmetered fence); the widget
// resolves on its callback and is removed either way; the 90-second bound; the
// seam with no handler is the build that cannot show a check (`EDGE_CHECK`);
// concurrent challenges share ONE widget; and the predicate is the status AND
// the header, never one of them.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CHALLENGE_MS,
  EDGE_CHECK,
  EDGE_CHECK_REPEATED,
  TURNSTILE_SCRIPT,
  challengeOnce,
  isEdgeChallenge,
  loadTurnstile,
  passChallenge,
  passesSoFar,
  resetTurnstileForTests,
  setChallengeHandler,
  type TurnstileApi,
  type TurnstileOptions,
} from "./turnstile";

/** A widget as Cloudflare's script hands it over: `render` records what it was
 *  asked and hands back an id; `remove` records which id went. */
function fakeTurnstile() {
  const rendered: { container: HTMLElement; options: TurnstileOptions }[] = [];
  const removed: string[] = [];
  const api: TurnstileApi = {
    render: (container, options) => {
      rendered.push({ container, options });
      return `widget-${rendered.length}`;
    },
    remove: (id) => {
      removed.push(id);
    },
  };
  return { api, rendered, removed, last: () => rendered.at(-1)!.options };
}

type Platform = Window & { turnstile?: TurnstileApi; __yokohamaTurnstile?: () => void };

/** The script ARRIVING: the global it defines, then the `onload` it names. */
function arrive(api: TurnstileApi): void {
  const w = window as Platform;
  w.turnstile = api;
  w.__yokohamaTurnstile?.();
}

const scripts = (): HTMLScriptElement[] =>
  [...document.querySelectorAll("script")].filter((s) => s.src.startsWith("https://challenges.cloudflare.com/"));

/** Let a chain of promise turns settle. */
const turns = async (n = 5): Promise<void> => {
  for (let i = 0; i < n; i += 1) await Promise.resolve();
};

/** What the edge answers a request it challenges, and two that it does not. */
const challenged = () =>
  new Response("<html>challenge</html>", {
    status: 403,
    headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
  });

beforeEach(() => {
  resetTurnstileForTests();
});
afterEach(() => {
  vi.useRealTimers();
  resetTurnstileForTests();
  for (const s of scripts()) s.remove();
  delete (window as Platform).turnstile;
});

describe("the loader", () => {
  // FIRST in the file, before any cell has asked for the script: importing the
  // module is not a request.
  it("injects nothing when the module is imported", () => {
    expect(scripts()).toEqual([]);
    expect((window as Platform).__yokohamaTurnstile).toBeUndefined();
  });

  it("injects the script ONCE across two calls, with the exact URL, and resolves on its onload", async () => {
    expect(TURNSTILE_SCRIPT).toBe(
      "https://challenges.cloudflare.com/turnstile/v0/api.js?onload=__yokohamaTurnstile&render=explicit",
    );
    const first = loadTurnstile();
    const second = loadTurnstile();
    expect(scripts()).toHaveLength(1);
    expect(scripts()[0]!.src).toBe(TURNSTILE_SCRIPT);
    const fake = fakeTurnstile();
    arrive(fake.api);
    await expect(first).resolves.toBe(fake.api);
    await expect(second).resolves.toBe(fake.api);
    // A third call after the arrival is the same widget, and still one tag.
    await expect(loadTurnstile()).resolves.toBe(fake.api);
    expect(scripts()).toHaveLength(1);
  });

  // A tag that failed to load is not a loaded script: it goes, and the next
  // demand asks again — still never two tags at once.
  it("takes a tag that failed to load away, and injects again on the next demand", async () => {
    const first = loadTurnstile();
    scripts()[0]!.dispatchEvent(new Event("error"));
    await expect(first).rejects.toThrow();
    expect(scripts()).toEqual([]);
    const again = loadTurnstile();
    expect(scripts()).toHaveLength(1);
    const fake = fakeTurnstile();
    arrive(fake.api);
    await expect(again).resolves.toBe(fake.api);
  });
});

describe("one challenge", () => {
  beforeEach(() => {
    vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", "site-key-fixture");
  });

  it("renders explicitly into the container with the build's key, resolves on the callback and removes the widget", async () => {
    const el = document.createElement("div");
    const fake = fakeTurnstile();
    let done = false;
    const check = challengeOnce(el).then(() => {
      done = true;
    });
    arrive(fake.api);
    await turns();
    expect(fake.rendered).toHaveLength(1);
    expect(fake.rendered[0]!.container).toBe(el);
    expect(fake.last().sitekey).toBe("site-key-fixture");
    expect(done, "nothing resolves before the widget says so").toBe(false);
    expect(fake.removed).toEqual([]);
    // The token is handed over and DISCARDED: the cookie the widget set is the
    // point, and nothing here sends a token anywhere.
    fake.last().callback("a-token-nobody-reads");
    await check;
    expect(fake.removed).toEqual(["widget-1"]);
  });

  it("rejects on the error callback and on the expired callback, removing the widget each time", async () => {
    for (const fire of [
      (o: TurnstileOptions) => o["error-callback"]("300030"),
      (o: TurnstileOptions) => o["expired-callback"](),
    ]) {
      const fake = fakeTurnstile();
      // A fresh page's loader each time, so this fake is the one that arrives.
      resetTurnstileForTests();
      const check = challengeOnce(document.createElement("div"));
      arrive(fake.api);
      await turns();
      fire(fake.last());
      await expect(check).rejects.toThrow();
      expect(fake.removed).toEqual(["widget-1"]);
    }
  });

  it(`rejects once ${CHALLENGE_MS / 1000} s have passed with no answer, and removes the widget`, async () => {
    expect(CHALLENGE_MS).toBe(90_000);
    vi.useFakeTimers();
    const fake = fakeTurnstile();
    const check = challengeOnce(document.createElement("div"));
    const outcome = check.then(
      () => "resolved",
      () => "rejected",
    );
    arrive(fake.api);
    await vi.advanceTimersByTimeAsync(CHALLENGE_MS - 1);
    expect(fake.removed).toEqual([]);
    await vi.advanceTimersByTimeAsync(1);
    expect(await outcome).toBe("rejected");
    expect(fake.removed).toEqual(["widget-1"]);
    expect(vi.getTimerCount()).toBe(0);
    // A callback that arrives after the bound changes nothing.
    fake.last().callback("late");
    expect(await outcome).toBe("rejected");
  });

  it("rejects and removes the widget when its signal aborts — the overlay's Cancel", async () => {
    const fake = fakeTurnstile();
    const abort = new AbortController();
    const check = challengeOnce(document.createElement("div"), abort.signal);
    arrive(fake.api);
    await turns();
    abort.abort();
    await expect(check).rejects.toThrow();
    expect(fake.removed).toEqual(["widget-1"]);
  });

  it("asks for no script at all in a build with no key", async () => {
    vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", null);
    await expect(challengeOnce(document.createElement("div"))).rejects.toThrow(EDGE_CHECK);
    expect(scripts()).toEqual([]);
  });
});

describe("the seam", () => {
  it("rejects with EDGE_CHECK when no overlay is mounted", async () => {
    expect(EDGE_CHECK).toBe("This site asked for a check this build cannot show.");
    await expect(passChallenge()).rejects.toThrow(EDGE_CHECK);
  });

  it("resolves when the handler does, and turns any refusal of the handler's into EDGE_CHECK_REPEATED", async () => {
    expect(EDGE_CHECK_REPEATED).toBe("The site's check did not pass. Try again from the door.");
    expect(passesSoFar()).toBe(0);
    setChallengeHandler(() => Promise.resolve());
    await expect(passChallenge()).resolves.toBeUndefined();
    // A pass that landed is counted — what tells a challenge older than the
    // clearance from one refused under it (`api.ts`'s retry) — and one that
    // did not pass is not.
    expect(passesSoFar()).toBe(1);
    setChallengeHandler(() => Promise.reject(new Error("the widget said 300030")));
    await expect(passChallenge()).rejects.toThrow(EDGE_CHECK_REPEATED);
    expect(passesSoFar()).toBe(1);
    setChallengeHandler(null);
    await expect(passChallenge()).rejects.toThrow(EDGE_CHECK);
  });

  // **The widget ONCE** (spec §2.6): the lobby's probe is three listings, and
  // a table link's screen asks the table and the health together — every one
  // of them meets the same challenge, and one person passes it once.
  it("hands every challenge that arrives while one is being passed the SAME pass", async () => {
    let finish: () => void = () => {};
    const handler = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          finish = resolve;
        }),
    );
    setChallengeHandler(handler);
    const a = passChallenge();
    const b = passChallenge();
    expect(handler).toHaveBeenCalledTimes(1);
    finish();
    await expect(Promise.all([a, b])).resolves.toEqual([undefined, undefined]);
    // A pass that has settled is over: the next challenge is a new one.
    void passChallenge();
    expect(handler).toHaveBeenCalledTimes(2);
  });
});

describe("the predicate", () => {
  it("is a 403 carrying cf-mitigated: challenge, and nothing else", () => {
    expect(isEdgeChallenge(challenged())).toBe(true);
    // A 403 of the layer's own — a locked table, a stranger's seat.
    expect(isEdgeChallenge(new Response('{"error":"not seated"}', { status: 403 }))).toBe(false);
    // The header on another status is not the edge's challenge.
    expect(isEdgeChallenge(new Response("", { status: 401, headers: { "cf-mitigated": "challenge" } }))).toBe(false);
    // Another mitigation is not a check a person can pass.
    expect(isEdgeChallenge(new Response("", { status: 403, headers: { "cf-mitigated": "block" } }))).toBe(false);
  });
});
