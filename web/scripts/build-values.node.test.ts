// The build's SIX values, as the BUILD checks them: the door's three (the
// project link first; the blurb and the notice through `checkText`) (M6.5c2,
// R-6.5c2.6; spec §2.3) and cycle F's three — the release feed (`checkReleaseFeed`,
// R-FP5), the Turnstile site key (`checkSiteKey`) and the site's own address
// (`checkSiteUrl`, R-F12): each class of bad value
// refused, and refused NAMING the variable — `vite.config.ts` calls the
// checkers, so a throw is `vite build`'s failure line, and a deployer reads
// which variable to fix.

import { describe, expect, it } from "vitest";
import {
  BUILD_VALUES,
  checkProjectUrl,
  checkReleaseFeed,
  checkSiteKey,
  checkSiteUrl,
  checkText,
  defaultFeedFor,
  PROJECT_URL_MAX,
  PROJECT_URL_VAR,
  RELEASE_FEED_MAX,
  RELEASE_FEED_VAR,
  SITE_KEY_MAX,
  SITE_KEY_VAR,
  SITE_URL_MAX,
  SITE_URL_VAR,
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

  it("names the six values with their caps", () => {
    expect(BUILD_VALUES).toEqual({
      YOKOHAMA_PROJECT_URL: 200,
      YOKOHAMA_SITE_BLURB: 400,
      YOKOHAMA_SITE_NOTICE: 200,
      YOKOHAMA_RELEASE_FEED: 300,
      YOKOHAMA_TURNSTILE_SITEKEY: 100,
      YOKOHAMA_SITE_URL: 200,
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

// **The site's address** (cycle F, R-F12; spec §2.7): the deployment's
// canonical ORIGIN, which the sitemap, `<link rel="canonical">` and `og:url`
// are built from — so, unlike the project link, a bare origin and nothing
// else: `https://` only, no path beyond a trailing `/`, no query, no fragment
// and no credentials. The value comes back as the origin itself (a trailing
// `/` gone), so every consumer writes `${siteUrl}/sitemap.xml` and cannot
// produce `//`.
describe("checkSiteUrl", () => {
  it("reads unset and empty as no address", () => {
    expect(checkSiteUrl(undefined)).toBeNull();
    expect(checkSiteUrl("")).toBeNull();
  });

  it("passes an https origin through, with or without a port", () => {
    expect(checkSiteUrl("https://yokohama.example.org")).toBe("https://yokohama.example.org");
    expect(checkSiteUrl("https://yokohama.example.org:8443")).toBe("https://yokohama.example.org:8443");
  });

  it("takes a trailing slash off, so no consumer writes a double slash", () => {
    expect(checkSiteUrl("https://yokohama.example.org/")).toBe("https://yokohama.example.org");
  });

  it("passes an origin at the cap exactly", () => {
    const url = `https://${"a".repeat(SITE_URL_MAX - "https://.test".length)}.test`;
    expect(url).toHaveLength(SITE_URL_MAX);
    expect(checkSiteUrl(url)).toBe(url);
  });

  it.each([
    ["a path", "https://yokohama.example.org/yokohama", /must be an origin/],
    ["a path under a slash", "https://yokohama.example.org//", /must be an origin/],
    ["a query", "https://yokohama.example.org/?a=1", /must be an origin/],
    ["an empty query", "https://yokohama.example.org?", /must be an origin/],
    ["a fragment", "https://yokohama.example.org/#top", /must be an origin/],
    ["credentials", "https://user:secret@yokohama.example.org", /holds credentials/],
    ["plain http", "http://yokohama.example.org", /must begin with https:\/\//],
    ["no scheme", "yokohama.example.org", /must begin with https:\/\//],
    ["another scheme", "ftp://yokohama.example.org", /must begin with https:\/\//],
    ["a space", "https://yokohama.example.org /", /whitespace or a control character/],
    ["a control character", "https://yokohama.example.org\u0007", /whitespace or a control character/],
    ["more than the cap", `https://${"a".repeat(SITE_URL_MAX)}.test`, /longer than 200 characters/],
    ["no host", "https://", /is not a URL|names no host/],
  ])("refuses %s, naming the variable", (_what, raw, why) => {
    expect(() => checkSiteUrl(raw)).toThrow(why);
    expect(() => checkSiteUrl(raw)).toThrow(SITE_URL_VAR);
    // The refusal is the SITE address's, never the project link's.
    expect(() => checkSiteUrl(raw)).not.toThrow(PROJECT_URL_VAR);
  });

  // A build log is no place for a password a deployer pasted into an address:
  // the credentials refusal names the variable and the reason, never the value.
  it("does not echo a credential into the refusal", () => {
    expect(() => checkSiteUrl("https://user:secret@yokohama.example.org")).not.toThrow(/secret/);
    expect(() => checkSiteUrl("https://user:secret@yokohama.example.org")).not.toThrow(/user/);
  });

  // **No refusal echoes a credential** (cycle F's fix wave; Task 5b's review,
  // tried by the final review): only the bare-origin shape reaches the
  // credentials arm, which never echoed the value — a value that ALSO carries
  // a path, a query, the wrong scheme, a space or too many characters meets
  // another arm first, and each of those printed it whole. Every arm now
  // prints it with everything from the scheme's `//` (or its colon) to the
  // LAST `@` cut. Mutant: any arm echoing the raw value.
  it.each([
    ["a path", "https://alice:hunter2pw@host.example/path"],
    ["a query", "https://alice:hunter2pw@host.example/?q=1"],
    ["plain http", "http://alice:hunter2pw@host.example"],
    ["no slashes after the scheme", "https:alice:hunter2pw@host.example"],
    ["a space", "https://alice:hunter2pw@host.example /"],
    ["more than the cap", `https://alice:hunter2pw@${"a".repeat(SITE_URL_MAX)}.test`],
    ["a password holding a slash", "https://alice:hunter2pw/x@host.example"],
    ["a password holding an @", "https://alice:hunter2pw@x@host.example/path"],
    ["a user alone, and a path", "https://alice@host.example/path"],
    ["the bare origin", "https://alice:hunter2pw@host.example"],
    // The residual fix (the Opus re-review): with no `scheme://`, a URL
    // parser reads `alice:` as the scheme, so the user name is cut too.
    ["no scheme at all", "alice:hunter2pw@host.example"],
  ])("refuses an address carrying credentials with %s, naming neither the user nor the password", (_what, raw) => {
    let said = "";
    try {
      checkSiteUrl(raw);
    } catch (e) {
      said = e instanceof Error ? e.message : String(e);
    }
    expect(said, "it is refused, naming the variable").toContain(SITE_URL_VAR);
    expect(said).not.toContain("alice");
    expect(said).not.toContain("hunter2pw");
  });
  it("cuts only the credentials, so the refusal still shows what was wrong", () => {
    expect(() => checkSiteUrl("https://alice:hunter2pw@host.example/path")).toThrow(
      `${SITE_URL_VAR} must be an origin, with no path, query or fragment beyond a trailing /: "https://***@host.example/path"`,
    );
    expect(() => checkSiteUrl("http://alice:hunter2pw@host.example")).toThrow(
      `${SITE_URL_VAR} must begin with https://: "http://***@host.example"`,
    );
    // The shared URL rules are the project link's and the feed's too.
    expect(() => checkProjectUrl("ftp://alice:hunter2pw@host.example")).toThrow(
      `${PROJECT_URL_VAR} must begin with https:// or http://: "ftp://***@host.example"`,
    );
    expect(() => checkReleaseFeed("http://alice:hunter2pw@host.example/feed", null)).toThrow(/"http:\/\/\*\*\*@host\.example\/feed"$/);
    // With no `scheme://` there is nothing to keep before the `@`: the user
    // name goes with the password (the residual fix).
    expect(() => checkSiteUrl("alice:hunter2pw@host.example")).toThrow(`${SITE_URL_VAR} must begin with https://: "***@host.example"`);
    expect(() => checkProjectUrl("alice:pw@github.com/o/r")).toThrow(
      `${PROJECT_URL_VAR} must begin with https:// or http://: "***@github.com/o/r"`,
    );
  });

  // The value is the origin, parsed: a mixed-case scheme and host come back in
  // lower case, so a canonical link is never spelled two ways.
  it("returns the parsed origin", () => {
    expect(checkSiteUrl("HTTPS://Yokohama.Example.org/")).toBe("https://yokohama.example.org");
  });
});
