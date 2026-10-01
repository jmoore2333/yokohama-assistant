// The build's values as the CLIENT reads them: the door's three (the link, the
// blurb, the notice; M6.5c2, spec §2.3) and cycle F's — the release feed, the
// Turnstile site key and the build's own version (R-F2, R-FP2).
//
// **The define is stubbed in every cell, and that is measured, not caution**:
// under vitest 4.1.11 a Vite `define` is a runtime GLOBAL rather than a static
// replacement, so `vi.stubGlobal` reaches it — and without the stub a
// `YOKOHAMA_PROJECT_URL` exported in the shell would leak into the run and turn
// the first cell red on a developer's machine. `unstubGlobals` puts it back.

import { describe, expect, it, vi } from "vitest";
import {
  buildVersion,
  linkFor,
  projectLink,
  releaseFeed,
  siteBlurb,
  siteNotice,
  turnstileSiteKey,
} from "./buildValues";

describe("the build values", () => {
  it("is nothing when the build names none", () => {
    vi.stubGlobal("__YOKOHAMA_PROJECT_URL__", null);
    expect(projectLink()).toBeNull();
    expect(linkFor("")).toBeNull();
  });

  it("is the build's URL, and its text is the URL without its scheme", () => {
    vi.stubGlobal("__YOKOHAMA_PROJECT_URL__", "https://github.com/you/yokohama-assistant");
    expect(projectLink()).toEqual({
      href: "https://github.com/you/yokohama-assistant",
      text: "github.com/you/yokohama-assistant",
    });
    expect(linkFor("http://lan.example/yokohama")).toEqual({
      href: "http://lan.example/yokohama",
      text: "lan.example/yokohama",
    });
  });

  it("reads the blurb and the notice from their globals, and none as null", () => {
    vi.stubGlobal("__YOKOHAMA_SITE_BLURB__", "Two sentences. Right here.");
    vi.stubGlobal("__YOKOHAMA_SITE_NOTICE__", "Not affiliated with Asmodee");
    expect(siteBlurb()).toBe("Two sentences. Right here.");
    expect(siteNotice()).toBe("Not affiliated with Asmodee");
    vi.stubGlobal("__YOKOHAMA_SITE_BLURB__", null);
    vi.stubGlobal("__YOKOHAMA_SITE_NOTICE__", null);
    expect(siteBlurb()).toBeNull();
    expect(siteNotice()).toBeNull();
  });
});

// **Cycle F's three** (R-F2, R-FP2): each read from its own global, as the
// door's three are. The feed and the key are `null` where the build set none;
// the version is always a string, from `web/package.json`.
describe("the release feed, the site key and the version", () => {
  it("reads the feed and the key from their globals, and none as null", () => {
    vi.stubGlobal("__YOKOHAMA_RELEASE_FEED__", "https://feed.example/repos/you/app/releases/latest");
    vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", "site-key-fixture");
    expect(releaseFeed()).toBe("https://feed.example/repos/you/app/releases/latest");
    expect(turnstileSiteKey()).toBe("site-key-fixture");
    vi.stubGlobal("__YOKOHAMA_RELEASE_FEED__", null);
    vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", null);
    expect(releaseFeed()).toBeNull();
    expect(turnstileSiteKey()).toBeNull();
  });

  it("reads the build's version from its global", () => {
    vi.stubGlobal("__YOKOHAMA_VERSION__", "0.6.0");
    expect(buildVersion()).toBe("0.6.0");
  });

  // The define is `web/package.json`'s `version` (`vite.config.ts`), which the
  // workspace cell `the_two_npm_packages_carry_the_workspace_version` holds to
  // the crate's: with no stub the vitest global IS that file's value.
  it("is, unstubbed, the package's own version", async () => {
    const { default: pkg } = await import("../package.json");
    expect(buildVersion()).toBe(pkg.version);
  });
});
