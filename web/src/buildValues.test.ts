// The door's three build values (the link, the blurb, the notice) as the CLIENT reads them (M6.5c2, spec §2.3).
//
// **The define is stubbed in every cell, and that is measured, not caution**:
// under vitest 4.1.11 a Vite `define` is a runtime GLOBAL rather than a static
// replacement, so `vi.stubGlobal` reaches it — and without the stub a
// `YOKOHAMA_PROJECT_URL` exported in the shell would leak into the run and turn
// the first cell red on a developer's machine. `unstubGlobals` puts it back.

import { describe, expect, it, vi } from "vitest";
import { linkFor, projectLink, siteBlurb, siteNotice } from "./buildValues";

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
