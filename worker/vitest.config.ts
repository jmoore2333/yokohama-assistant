import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { defineConfig } from "vitest/config";

// **The POOL** (spec §5.2): Cloudflare's Workers Vitest integration,
// `@cloudflare/vitest-plugin` 1.2.3, whose entry is the `cloudflareTest()`
// Vite plugin. Every `test/**/*.test.ts` runs INSIDE workerd, against the
// Worker `wrangler.test.jsonc` describes — never `wrangler.jsonc`, which is
// production's.
//
// It runs over the BUILT module under `gen/` and rebuilds nothing: after a
// change to `crates/yokohama-worker`, `npm run build:module:debug` comes FIRST,
// or the pool tests the module as it was. The debug build is the one the pool
// needs — its trap cells call `debug_trap`, which a release build does not
// export.
//
// `npm run test:scripts` (`vitest.scripts.config.ts`) runs the node half; the
// two globs do not overlap (the orchestrator's ruling I-31).
export default defineConfig({
  plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.test.jsonc" } })],
  test: {
    include: ["test/**/*.test.ts"],
  },
});
