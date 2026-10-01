// The release notice's reading (M6.5c2 cycle F, R-F2, R-FP5; spec §2.2): the
// version comparison, and the hook that asks the feed at most once per page
// life and at most once an hour per browser.
//
// **A feed fixture is not a wire payload.** `web/src/wire/` holds what THIS
// project's server says; the release feed is a third party's (GitHub's
// `releases/latest`), so its shape is held here, beside the cells that read it
// (`{ "tag_name", "html_url" }`, the two fields the client uses).
//
// **The build values are stubbed in every cell**, as `buildValues.test.ts`
// does: under vitest a Vite `define` is a runtime global, so `vi.stubGlobal`
// reaches it, and a `YOKOHAMA_*` exported in a developer's shell cannot leak in.
// `fetch` is the setup file's throwing stub unless a cell hands the hook its own
// through the seam, so a read that strays to the network fails loudly.

import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  isNewer,
  parseVersion,
  releaseFeedUrl,
  resetReleaseNoticeForTests,
  useReleaseNotice,
} from "./releaseFeed";

const FEED = "https://api.github.com/repos/you/yokohama-assistant/releases/latest";
const RELEASE_PAGE = "https://github.com/you/yokohama-assistant/releases/tag/v0.7.0";
const KEY = "yokohama.release.checked";
const HOUR = 60 * 60 * 1000;

/** What GitHub's `releases/latest` says, as far as the client reads it. */
const latest = (tag: unknown = "v0.7.0", url: unknown = RELEASE_PAGE) => ({ tag_name: tag, html_url: url });

/** A `fetch` that answers every call with `body` as JSON. */
const answering = (body: unknown, status = 200) =>
  vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => new Response(JSON.stringify(body), { status }));

/** Stub the build: the feed, the project link and the version this site runs. */
function build(over: { feed?: string | null; project?: string | null; version?: string } = {}) {
  vi.stubGlobal("__YOKOHAMA_RELEASE_FEED__", over.feed === undefined ? FEED : over.feed);
  vi.stubGlobal("__YOKOHAMA_PROJECT_URL__", over.project === undefined ? null : over.project);
  vi.stubGlobal("__YOKOHAMA_VERSION__", over.version ?? "0.6.0");
}

/** The stored answer, as the hook writes it. */
const stored = (checkedAt: number, tag = "v0.7.0", url = RELEASE_PAGE) =>
  localStorage.setItem(KEY, JSON.stringify({ checkedAt, tag, url }));

beforeEach(() => {
  resetReleaseNoticeForTests();
  build();
});
afterEach(() => {
  resetReleaseNoticeForTests();
  vi.useRealTimers();
});

describe("parseVersion", () => {
  it("reads three numbers, the v optional", () => {
    expect(parseVersion("v0.7.0")).toEqual([0, 7, 0]);
    expect(parseVersion("0.7.0")).toEqual([0, 7, 0]);
    expect(parseVersion("v10.20.300")).toEqual([10, 20, 300]);
  });

  it.each([
    ["empty", ""],
    ["two numbers", "v0.7"],
    ["four numbers", "v0.7.0.1"],
    ["a pre-release suffix", "v0.7.0-rc1"],
    ["build metadata", "0.7.0+build5"],
    ["a doubled v", "vv0.7.0"],
    ["a capital V", "V0.7.0"],
    ["surrounding space", " v0.7.0 "],
    ["a word in a place", "v0.7.x"],
    ["a name", "nightly"],
    ["a number too long to compare exactly", "v1234567890.0.0"],
  ])("reads %s as nothing, never a false alarm", (_what, raw) => {
    expect(parseVersion(raw)).toBeNull();
  });
});

describe("isNewer", () => {
  it("is true for a tag ahead of this build, in any place", () => {
    expect(isNewer("v0.7.0", "0.6.0")).toBe(true);
    expect(isNewer("v0.6.1", "0.6.0")).toBe(true);
    expect(isNewer("v1.0.0", "0.9.9")).toBe(true);
  });

  // The three places are compared as NUMBERS, in order: a string compare would
  // say "0.10.0" is behind "0.9.0".
  it("compares numbers and not text", () => {
    expect(isNewer("v0.10.0", "0.9.0")).toBe(true);
    expect(isNewer("v0.9.0", "0.10.0")).toBe(false);
  });

  // **The equal cell**: a notice that said "a newer release is available: v0.6.0.
  // This site runs 0.6.0" would fire on every build that is current.
  it("is false for the release this build IS", () => {
    expect(isNewer("v0.6.0", "0.6.0")).toBe(false);
    expect(isNewer("0.6.0", "v0.6.0")).toBe(false);
  });

  it("is false for an older tag", () => {
    expect(isNewer("v0.5.9", "0.6.0")).toBe(false);
    expect(isNewer("v0.6.0", "0.6.1")).toBe(false);
    expect(isNewer("v0.9.9", "1.0.0")).toBe(false);
  });

  it("is false when either side does not parse", () => {
    expect(isNewer("nightly", "0.6.0")).toBe(false);
    expect(isNewer("v0.7.0-rc1", "0.6.0")).toBe(false);
    expect(isNewer("v0.7.0", "dev")).toBe(false);
    expect(isNewer("", "")).toBe(false);
  });
});

describe("releaseFeedUrl", () => {
  it("is the build's feed, and null where the build has none", () => {
    expect(releaseFeedUrl()).toBe(FEED);
    build({ feed: null });
    expect(releaseFeedUrl()).toBeNull();
  });
});

describe("useReleaseNotice", () => {
  it("answers the newer release the feed names, asking it the way GitHub's API wants", async () => {
    const fetchImpl = answering(latest());
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    expect(result.current, "nothing is drawn before the answer").toBeNull();
    await waitFor(() => expect(result.current).toEqual({ tag: "v0.7.0", url: RELEASE_PAGE }));
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(fetchImpl).toHaveBeenCalledWith(FEED, {
      cache: "no-store",
      headers: { accept: "application/vnd.github+json" },
    });
  });

  it("answers nothing for the release this build is, and for an older one", async () => {
    for (const tag of ["v0.6.0", "v0.5.0"]) {
      resetReleaseNoticeForTests();
      localStorage.clear();
      const fetchImpl = answering(latest(tag));
      const { result, unmount } = renderHook(() => useReleaseNotice(fetchImpl));
      await waitFor(() => expect(fetchImpl).toHaveBeenCalledTimes(1));
      await act(async () => {});
      expect(result.current).toBeNull();
      unmount();
    }
  });

  it("answers nothing for a tag that does not parse", async () => {
    const fetchImpl = answering(latest("nightly"));
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(fetchImpl).toHaveBeenCalledTimes(1));
    await act(async () => {});
    expect(result.current).toBeNull();
  });

  // **Any failure draws nothing, and nothing retries** (spec §2.2): a server
  // error, a network error, a body that is not JSON, and a body with no tag.
  it.each([
    ["a 500", () => answering({ message: "boom" }, 500)],
    ["a 404", () => answering({ message: "Not Found" }, 404)],
    // The status is read on its own: an error page that still names a newer tag
    // (a proxy's cached body, a misrouted feed) is not an answer.
    ["a 500 whose body still names a newer tag", () => answering(latest(), 500)],
    ["a network error", () => vi.fn(async () => Promise.reject(new TypeError("Failed to fetch")))],
    ["a body that is not JSON", () => vi.fn(async () => new Response("<html>", { status: 200 }))],
    ["a body with no tag", () => answering({ html_url: RELEASE_PAGE })],
    ["a tag that is not a string", () => answering(latest(7))],
  ])("answers nothing on %s, asks once and never again", async (_what, make) => {
    const fetchImpl = make();
    const { result, rerender } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(fetchImpl).toHaveBeenCalledTimes(1));
    await act(async () => {});
    expect(result.current).toBeNull();
    rerender();
    await act(async () => {});
    expect(fetchImpl, "nothing retried within the page life").toHaveBeenCalledTimes(1);
    expect(localStorage.getItem(KEY), "a failure is not an answer, so nothing is kept").toBeNull();
  });

  it("never fetches when the build has no feed", async () => {
    build({ feed: null });
    const fetchImpl = answering(latest());
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await act(async () => {});
    expect(result.current).toBeNull();
    expect(fetchImpl, "no feed, no request").not.toHaveBeenCalled();
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  // **Once per page life, across the screens**: the door and the lobby each draw
  // the notice, and a player who goes from one to the other is in the same page.
  it("asks once per page life, however many times it is mounted", async () => {
    const fetchImpl = answering(latest());
    const first = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(first.result.current).not.toBeNull());
    first.unmount();
    const second = renderHook(() => useReleaseNotice(fetchImpl));
    expect(second.result.current, "the second screen draws it at once").toEqual({
      tag: "v0.7.0",
      url: RELEASE_PAGE,
    });
    await act(async () => {});
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("asks once when two are mounted while the first is still waiting", async () => {
    let answer: (r: Response) => void = () => {};
    const fetchImpl = vi.fn(() => new Promise<Response>((resolve) => (answer = resolve)));
    const a = renderHook(() => useReleaseNotice(fetchImpl));
    const b = renderHook(() => useReleaseNotice(fetchImpl));
    await act(async () => {});
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    await act(async () => answer(new Response(JSON.stringify(latest()), { status: 200 })));
    await waitFor(() => expect(a.result.current).not.toBeNull());
    expect(b.result.current).toEqual({ tag: "v0.7.0", url: RELEASE_PAGE });
  });

  it("keeps what the feed said, with the time it said it", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(1_000_000);
    const fetchImpl = answering(latest());
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(result.current).not.toBeNull());
    expect(JSON.parse(localStorage.getItem(KEY) ?? "null")).toEqual({
      checkedAt: 1_000_000,
      tag: "v0.7.0",
      url: RELEASE_PAGE,
    });
  });

  // **The stored answer, and not only the time** (F-13): a reload inside the
  // hour must not make the notice vanish, and must not cost a request.
  it("draws a stored newer answer under an hour old, with no request", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(10 * HOUR);
    stored(10 * HOUR - 30 * 60 * 1000);
    const fetchImpl = answering(latest("v9.9.9"));
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(result.current).toEqual({ tag: "v0.7.0", url: RELEASE_PAGE }));
    expect(fetchImpl, "the stored answer is drawn from, not checked again").not.toHaveBeenCalled();
  });

  it("asks once more when the stored answer is over an hour old, and keeps the new one", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(10 * HOUR);
    stored(10 * HOUR - HOUR - 1);
    const fetchImpl = answering(latest("v0.8.0", "https://github.com/you/yokohama-assistant/releases/tag/v0.8.0"));
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(result.current?.tag).toBe("v0.8.0"));
    expect(fetchImpl).toHaveBeenCalledTimes(1);
    expect(JSON.parse(localStorage.getItem(KEY) ?? "null")).toMatchObject({ checkedAt: 10 * HOUR, tag: "v0.8.0" });
  });

  // The stored TAG is compared with the build at read time, so a site that has
  // taken the release since stops saying so, inside the same hour.
  it("draws nothing from a stored answer this build has caught up with, and asks nothing", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(10 * HOUR);
    stored(10 * HOUR - 60_000, "v0.7.0");
    build({ version: "0.7.0" });
    const fetchImpl = answering(latest("v9.9.9"));
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await act(async () => {});
    expect(result.current).toBeNull();
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it("asks when the stored time is in the future, which is a clock set back", async () => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(10 * HOUR);
    stored(10 * HOUR + 5 * 60 * 1000);
    const fetchImpl = answering(latest());
    renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(fetchImpl).toHaveBeenCalledTimes(1));
  });

  it.each([
    ["not JSON", "{nope"],
    ["not an object", "7"],
    ["no time", JSON.stringify({ tag: "v0.7.0", url: RELEASE_PAGE })],
    ["a time that is not a number", JSON.stringify({ checkedAt: "now", tag: "v0.7.0", url: RELEASE_PAGE })],
    ["no tag", JSON.stringify({ checkedAt: 1, url: RELEASE_PAGE })],
  ])("asks when what is stored is %s", async (_what, raw) => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(1_000);
    localStorage.setItem(KEY, raw);
    const fetchImpl = answering(latest());
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(result.current).not.toBeNull());
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  // A broken storage means "check" (the `chatFold.ts` idiom): the notice still
  // draws, and nothing throws out of the hook.
  it("checks, and still draws, when the browser's storage throws", async () => {
    const broken = {
      getItem: () => {
        throw new Error("blocked");
      },
      setItem: () => {
        throw new Error("blocked");
      },
    };
    vi.stubGlobal("localStorage", broken);
    const fetchImpl = answering(latest());
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(result.current).toEqual({ tag: "v0.7.0", url: RELEASE_PAGE }));
    expect(fetchImpl).toHaveBeenCalledTimes(1);
  });

  it("checks, and still draws, when the browser has no storage at all", async () => {
    vi.stubGlobal("localStorage", undefined);
    const fetchImpl = answering(latest());
    const { result } = renderHook(() => useReleaseNotice(fetchImpl));
    await waitFor(() => expect(result.current?.tag).toBe("v0.7.0"));
  });

  describe("where the notice links", () => {
    it("falls back to the project link's releases page when the feed names none", async () => {
      build({ project: "https://github.com/you/yokohama-assistant/" });
      const fetchImpl = answering({ tag_name: "v0.7.0" });
      const { result } = renderHook(() => useReleaseNotice(fetchImpl));
      await waitFor(() =>
        expect(result.current).toEqual({ tag: "v0.7.0", url: "https://github.com/you/yokohama-assistant/releases" }),
      );
    });

    // The feed is a third party's, and its `html_url` becomes an `href`: only an
    // absolute http(s) URL is taken, so `javascript:` never reaches a link.
    it.each([
      ["a javascript: URL", "javascript:alert(1)"],
      ["a data: URL", "data:text/html,<b>x</b>"],
      // The classic bypass of a check that only asks for a host: a script URL
      // that HAS one. Only the scheme refuses it.
      ["a javascript: URL that has a host", "javascript://example.org/%0Aalert(1)"],
      ["an ftp: URL", "ftp://example.org/releases"],
      ["a relative path", "/releases/tag/v0.7.0"],
      ["text that is not a URL", "the latest one"],
      ["a number", 7],
      ["a URL with a space in it", "https://example.org/a b"],
    ])("never takes %s from the feed as the link", async (_what, html) => {
      build({ project: "https://github.com/you/yokohama-assistant" });
      const fetchImpl = answering(latest("v0.7.0", html));
      const { result } = renderHook(() => useReleaseNotice(fetchImpl));
      await waitFor(() =>
        expect(result.current).toEqual({ tag: "v0.7.0", url: "https://github.com/you/yokohama-assistant/releases" }),
      );
    });

    it("has no link at all when the feed names none and the build has no project link", async () => {
      const fetchImpl = answering({ tag_name: "v0.7.0" });
      const { result } = renderHook(() => useReleaseNotice(fetchImpl));
      await waitFor(() => expect(result.current).toEqual({ tag: "v0.7.0", url: "" }));
    });
  });
});
