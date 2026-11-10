import { cloudflareTest } from "@cloudflare/vitest-plugin";
import { configDefaults, defineConfig } from "vitest/config";

// **The POOL** (spec §5.2): Cloudflare's Workers Vitest integration,
// `@cloudflare/vitest-plugin`, whose entry is the `cloudflareTest()`
// Vite plugin. Every cell runs INSIDE workerd, against the Worker a pool
// configuration describes — never `wrangler.jsonc`, which is production's.
//
// **The plugin and `wrangler` move as a PAIR** (2026-11-10). The plugin pins
// `wrangler` and `miniflare` at exact versions and Cloudflare publishes the
// two together, so `package.json` holds both by a caret and ONE command moves
// them, with the one-week floor every routine move takes (`CONTRIBUTING.md`,
// Dependencies):
//   npm update wrangler @cloudflare/vitest-plugin --package-lock-only --before=<the date a week ago>
// It moves the pair FORWARD: on a lockfile that already holds a release newer
// than the date npm refuses (ETARGET, measured), so a step back starts from
// the older lockfile (`git checkout <commit> -- package-lock.json`).
// Moving one alone makes npm install a second wrangler under the plugin and a
// second miniflare and workerd under wrangler (measured: wrangler 4.158.0
// beside plugin 1.6.1), and the pool then tests a toolchain other than the one
// that deploys. The lockfile still fixes what `npm ci` installs.
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
