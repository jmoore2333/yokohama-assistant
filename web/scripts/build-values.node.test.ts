// The build's FIVE values, as the BUILD checks them: the door's three (the
// project link first; the blurb and the notice through `checkText`) (M6.5c2,
// R-6.5c2.6; spec §2.3) and cycle F's two — the release feed (`checkReleaseFeed`,
// R-FP5) and the Turnstile site key (`checkSiteKey`): each class of bad value
// refused, and refused NAMING the variable — `vite.config.ts` calls the
// checkers, so a throw is `vite build`'s failure line, and a deployer reads
// which variable to fix.

import { describe, expect, it } from "vitest";
import {
  BUILD_VALUES,
  checkProjectUrl,
  checkReleaseFeed,
  checkSiteKey,
  checkText,
  defaultFeedFor,
  PROJECT_URL_MAX,
  PROJECT_URL_VAR,
  RELEASE_FEED_MAX,
  RELEASE_FEED_VAR,
  SITE_KEY_MAX,
  SITE_KEY_VAR,
} from "./build-values.mjs";

describe("checkProjectUrl", () => {
  // Unset draws no link, and so does EMPTY: an `ARG` given no
  // `--build-arg` is absent from `RUN`'s environment (unset), and
  // `--build-arg X=` is the empty string; each is a deployment that set
  // nothing, not a bad value.
  it("reads unset and empty as no link", () => {
    expect(checkProjectUrl(undefined)).toBeNull();
    expect(checkProjectUrl("")).toBeNull();
  });

  it("passes an absolute https or http URL through unchanged", () => {
    expect(checkProjectUrl("https://github.com/you/yokohama-assistant")).toBe(
      "https://github.com/you/yokohama-assistant",
    );
    expect(checkProjectUrl("http://lan.example/yokohama")).toBe("http://lan.example/yokohama");
  });

  it.each([
    ["no scheme", "github.com/you/yokohama-assistant", /must begin with https:\/\/ or http:\/\//],
    ["another scheme", "ftp://example.test/x", /must begin with https:\/\/ or http:\/\//],
    ["a space", "https://example.test/a b", /whitespace or a control character/],
    ["a control character", "https://example.test/\u0007", /whitespace or a control character/],
    ["more than the cap", `https://${"a".repeat(PROJECT_URL_MAX)}.test`, /longer than 200 characters/],
    ["no host", "https://", /is not a URL|names no host/],
  ])("refuses %s, naming the variable", (_what, raw, why) => {
    expect(() => checkProjectUrl(raw)).toThrow(why);
    expect(() => checkProjectUrl(raw)).toThrow(PROJECT_URL_VAR);
  });
});

describe("checkText", () => {
  it("reads unset and empty as none", () => {
    expect(checkText("YOKOHAMA_SITE_NOTICE", undefined, 200)).toBeNull();
    expect(checkText("YOKOHAMA_SITE_NOTICE", "", 200)).toBeNull();
  });

  it("passes words through unchanged, at the cap exactly", () => {
    expect(checkText("YOKOHAMA_SITE_NOTICE", "Not affiliated with Asmodee", 200)).toBe(
      "Not affiliated with Asmodee",
    );
    expect(checkText("YOKOHAMA_SITE_BLURB", "a".repeat(400), 400)).toBe("a".repeat(400));
  });

  it("refuses whitespace only, naming the variable", () => {
    expect(() => checkText("YOKOHAMA_SITE_NOTICE", "   ", 200)).toThrow("YOKOHAMA_SITE_NOTICE holds no words");
  });

  it("refuses a control character, naming the variable", () => {
    expect(() => checkText("YOKOHAMA_SITE_BLURB", "two\nlines", 400)).toThrow(
      "YOKOHAMA_SITE_BLURB holds a control character",
    );
    expect(() => checkText("YOKOHAMA_SITE_BLURB", "bell\u0007", 400)).toThrow(/holds a control character/);
    // Invisible format characters count: a zero-width space and a bidi override.
    expect(() => checkText("YOKOHAMA_SITE_BLURB", "a\u200Bb", 400)).toThrow(/holds a control character/);
    expect(() => checkText("YOKOHAMA_SITE_BLURB", "a\u202Eb", 400)).toThrow(/holds a control character/);
  });

  it("refuses one character over the cap, naming the variable and the cap", () => {
    expect(() => checkText("YOKOHAMA_SITE_BLURB", "a".repeat(401), 400)).toThrow(
      "YOKOHAMA_SITE_BLURB is longer than 400 characters",
    );
  });

  it("names the five values with their caps", () => {
    expect(BUILD_VALUES).toEqual({
      YOKOHAMA_PROJECT_URL: 200,
      YOKOHAMA_SITE_BLURB: 400,
      YOKOHAMA_SITE_NOTICE: 200,
      YOKOHAMA_RELEASE_FEED: 300,
      YOKOHAMA_TURNSTILE_SITEKEY: 100,
    });
  });
});

// **The release feed's three states** (cycle F, R-F2, R-FP5; spec §2.2), spelled
// so "empty" and "unset" cannot be confused with "off": unset (or empty, which
// is a `--build-arg X=` and means the same) derives the feed from a GitHub
// project link; a URL is that feed; the literal `off` is none.
describe("defaultFeedFor", () => {
  it("derives the latest-release endpoint from a GitHub project link, a trailing slash tolerated", () => {
    const want = "https://api.github.com/repos/you/yokohama-assistant/releases/latest";
    expect(defaultFeedFor("https://github.com/you/yokohama-assistant")).toBe(want);
    expect(defaultFeedFor("https://github.com/you/yokohama-assistant/")).toBe(want);
  });

  // The four link shapes the spec's proof names: a GitHub repository (above),
  // and three that are not one. A link that only LOOKS like GitHub's must not
  // point a build at another host's API.
  it.each([
    ["no project link", null],
    ["another host", "https://gitlab.com/you/yokohama-assistant"],
    ["a host that only ends like GitHub's", "https://evilgithub.com/you/yokohama-assistant"],
    ["a subdomain of GitHub", "https://gist.github.com/you/yokohama-assistant"],
    ["the owner's page alone", "https://github.com/you"],
    ["a path under the repository", "https://github.com/you/yokohama-assistant/tree/main"],
    ["plain http", "http://github.com/you/yokohama-assistant"],
  ])("derives nothing from %s", (_what, link) => {
    expect(defaultFeedFor(link)).toBeNull();
  });
});

describe("checkReleaseFeed", () => {
  const GITHUB = "https://github.com/you/yokohama-assistant";
  const DERIVED = "https://api.github.com/repos/you/yokohama-assistant/releases/latest";

  it("derives the feed from a GitHub project link when the variable is unset", () => {
    expect(checkReleaseFeed(undefined, GITHUB)).toBe(DERIVED);
  });

  it("reads empty as unset, which is what a bare --build-arg says", () => {
    expect(checkReleaseFeed("", `${GITHUB}/`)).toBe(DERIVED);
  });

  it("is no feed when unset and the project link is anything else, or unset", () => {
    expect(checkReleaseFeed(undefined, "https://example.org/yokohama")).toBeNull();
    expect(checkReleaseFeed(undefined, null)).toBeNull();
    expect(checkReleaseFeed("", null)).toBeNull();
  });

  it("is no feed for the literal off, whatever the project link", () => {
    expect(checkReleaseFeed("off", GITHUB)).toBeNull();
    expect(checkReleaseFeed("off", null)).toBeNull();
  });

  it("takes a URL as that feed, over the derived default", () => {
    expect(checkReleaseFeed("https://releases.example.org/latest.json", GITHUB)).toBe(
      "https://releases.example.org/latest.json",
    );
  });

  // `off` is the LITERAL: any other spelling is a value, and a value is a URL.
  it.each([["Off"], ["OFF"], ["false"], ["none"], ["off "]])("does not read %j as off", (raw) => {
    expect(() => checkReleaseFeed(raw, GITHUB)).toThrow(RELEASE_FEED_VAR);
  });

  it.each([
    ["no scheme", "releases.example.org/latest", /must begin with https:\/\//],
    ["plain http", "http://releases.example.org/latest", /must begin with https:\/\//],
    ["another scheme", "ftp://releases.example.org/latest", /must begin with https:\/\//],
    ["a space", "https://releases.example.org/a b", /whitespace or a control character/],
    ["a control character", "https://releases.example.org/\u0007", /whitespace or a control character/],
    ["more than the cap", `https://${"a".repeat(RELEASE_FEED_MAX)}.test`, /longer than 300 characters/],
    ["no host", "https://", /is not a URL|names no host/],
  ])("refuses %s, naming the variable", (_what, raw, why) => {
    expect(() => checkReleaseFeed(raw, GITHUB)).toThrow(why);
    expect(() => checkReleaseFeed(raw, GITHUB)).toThrow(RELEASE_FEED_VAR);
    // The refusal is the FEED's, never the project link's.
    expect(() => checkReleaseFeed(raw, GITHUB)).not.toThrow(PROJECT_URL_VAR);
  });

  it("passes a URL at the cap exactly", () => {
    const url = `https://${"a".repeat(RELEASE_FEED_MAX - "https://.test".length)}.test`;
    expect(url).toHaveLength(RELEASE_FEED_MAX);
    expect(checkReleaseFeed(url, null)).toBe(url);
  });
});

// **The Turnstile site key** (cycle F, R-F6): public, but still a value a
// deployer types — plain text, nothing the page could not carry as an
// attribute. Unset and empty are both "no key", which draws the plain link.
describe("checkSiteKey", () => {
  it("reads unset and empty as no key", () => {
    expect(checkSiteKey(undefined)).toBeNull();
    expect(checkSiteKey("")).toBeNull();
  });

  it("passes a plain key through unchanged, at the cap exactly", () => {
    expect(checkSiteKey("site-key-fixture_01")).toBe("site-key-fixture_01");
    expect(checkSiteKey("k".repeat(SITE_KEY_MAX))).toBe("k".repeat(SITE_KEY_MAX));
  });

  it.each([
    ["a space inside", "site key", /whitespace or a control character/],
    ["a leading space", " site-key", /whitespace or a control character/],
    ["a line break", "site\nkey", /whitespace or a control character/],
    ["a control character", "site\u0007key", /whitespace or a control character/],
    ["a zero-width space", "site\u200Bkey", /whitespace or a control character/],
    ["one character over the cap", "k".repeat(SITE_KEY_MAX + 1), /longer than 100 characters/],
  ])("refuses %s, naming the variable", (_what, raw, why) => {
    expect(() => checkSiteKey(raw)).toThrow(why);
    expect(() => checkSiteKey(raw)).toThrow(SITE_KEY_VAR);
  });
});
