// The door's THREE build values, as the BUILD checks them (the project link
// first; the blurb and the notice through `checkText`) (M6.5c2, R-6.5c2.6; spec
// §2.3): each class of bad value refused, and refused NAMING the variable —
// `vite.config.ts` calls `checkProjectUrl`, so its throw is `vite build`'s
// failure line, and a deployer reads which variable to fix.

import { describe, expect, it } from "vitest";
import {
  BUILD_VALUES,
  checkProjectUrl,
  checkText,
  PROJECT_URL_MAX,
  PROJECT_URL_VAR,
} from "./build-values.mjs";

describe("checkProjectUrl", () => {
  // Unset draws no link, and so does EMPTY: a Docker `ARG` nobody passed
  // reaches the build as an empty string, and that is a deployment that set
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
  });

  it("refuses one character over the cap, naming the variable and the cap", () => {
    expect(() => checkText("YOKOHAMA_SITE_BLURB", "a".repeat(401), 400)).toThrow(
      "YOKOHAMA_SITE_BLURB is longer than 400 characters",
    );
  });

  it("names the three values with their caps", () => {
    expect(BUILD_VALUES).toEqual({
      YOKOHAMA_PROJECT_URL: 200,
      YOKOHAMA_SITE_BLURB: 400,
      YOKOHAMA_SITE_NOTICE: 200,
    });
  });
});
