import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { parse } from "jsonc-parser";
import { describe, expect, it } from "vitest";
import { DEV_VARS, ORIGIN } from "./dev-vars.mjs";

// **The one origin** (M6.5c2 cycle D, Task 9). `ORIGIN` is stated once in
// code, in `dev-vars.mjs`, which the live replay and the live tier import; the
// route that binds the Worker to it is configuration, `wrangler.jsonc`'s, and
// cannot import a constant — so this pins the two together. The reader's own
// cells are the live replay's (`live-replay.test.mjs`) and the live tier's
// (`web/e2e/harness.node.test.ts`), both over this module.

describe("the one origin", () => {
  it("is the host wrangler.jsonc's only route binds, over https and with no path", () => {
    const text = readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8");
    const errors = [];
    const config = parse(text, errors, { allowTrailingComma: true, disallowComments: false });
    expect(errors, "wrangler.jsonc parses").toEqual([]);
    const url = new URL(ORIGIN);
    expect(url.origin, "ORIGIN is an origin: no path, no trailing slash").toBe(ORIGIN);
    expect(url.protocol).toBe("https:");
    expect(config.routes).toEqual([{ pattern: url.host, custom_domain: true }]);
  });

  it("reads the credential from the repository root's .dev.vars", () => {
    const root = new URL("../../", import.meta.url);
    // The premise: that directory IS the repository's root.
    expect(readFileSync(new URL("rust-toolchain.toml", root), "utf8"), "premise: the root holds rust-toolchain.toml").toContain("channel");
    expect(DEV_VARS).toBe(fileURLToPath(new URL(".dev.vars", root)));
  });
});
