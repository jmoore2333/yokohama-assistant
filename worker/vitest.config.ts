import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { configDefaults, defineConfig } from "vitest/config";

// **The POOL** (spec §5.2): Cloudflare's Workers Vitest integration,
// `@cloudflare/vitest-plugin` 1.2.3, whose entry is the `cloudflareTest()`
// Vite plugin. Every cell runs INSIDE workerd, against the Worker a pool
// configuration describes — never `wrangler.jsonc`, which is production's.
//
// **Two projects, one per door** (M6.5c2, spec §2.1; review R2.2-M2). Every
// object builds its configuration from its OWN constructor's `env` — the
// configuration file's — so a cell's env override at `handle` moves the
// Worker's door and never an object's, and one project cannot prove both:
// - `worker`, over `wrangler.test.jsonc` (`YOKOHAMA_WEB_IDENTITY=worker`,
//   production's door): every `test/**/*.test.ts` but `test/open/`;
// - `open`, over `wrangler.open.test.jsonc` (`YOKOHAMA_WEB_IDENTITY=open`):
//   `test/open/**/*.test.ts`.
// Each project runs its own workerd, with its own storage and bucket: pool
// fact 5 holds per project as it holds per file. `npm test` runs both, and
// `npx vitest run --project open` one. Vitest 4 removed the workspace file, so
// a workspace IS `test.projects`.
//
// It runs over the BUILT module under `gen/` and rebuilds nothing: after a
// change to `crates/yokohama-worker`, `npm run build:module:debug` comes FIRST,
// or the pool tests the module as it was. The debug build is the one the pool
// needs — its trap cells call `debug_trap`, which a release build does not
// export.
//
// `npm run test:scripts` (`vitest.scripts.config.ts`) runs the node half; the
// globs do not overlap (the orchestrator's ruling I-31).
export default defineConfig({
  test: {
    projects: [
      {
        plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.test.jsonc" } })],
        test: { name: "worker", include: ["test/**/*.test.ts"], exclude: [...configDefaults.exclude, "test/open/**"] },
      },
      {
        plugins: [cloudflareTest({ wrangler: { configPath: "./wrangler.open.test.jsonc" } })],
        test: { name: "open", include: ["test/open/**/*.test.ts"] },
      },
    ],
  },
});
