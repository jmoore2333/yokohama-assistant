/// <reference types="vitest/config" />
import { defineConfig, mergeConfig } from "vite";
import { configDefaults } from "vitest/config";
import viteConfig from "./vite.config";

// The component suite: the client's own half of the M2.5 harness.
//
// A SEPARATE config file rather than a `test` block inside `vite.config.ts`,
// because that file is what the production build reads and the build should not
// have to parse a test runner's options to produce `dist`. It MERGES the build
// config rather than restating it, so the React plugin, the aliases and the
// module resolution are the ones the shipped bundle uses — a component test
// that ran under different resolution would be testing a different module
// graph than the one a player loads.
//
// `include` is scoped to `src`, deliberately: `e2e/` is Playwright's, driving a
// real browser against a real server, and vitest picking those files up would
// run them in jsdom where `page` does not exist.
export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      environment: "jsdom",
      globals: true,
      setupFiles: ["./src/test/setup.ts"],
      include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
      // M6.5b: the wasm corpus replay (`src/local/*.node.test.ts`) is its own
      // project — environment `node`, no setup file, and it loads a module this
      // run does not build. `include` above would otherwise sweep it into jsdom.
      //
      // It EXTENDS vitest's own defaults rather than replacing them: an
      // `exclude` given here wins outright, so a bare list would put
      // `node_modules` and `dist` back into the glob.
      //
      // **M6.5b2 adds the real-build cell** (`src/precache.node.test.ts`),
      // which runs a real `vite build` into a temporary directory. It is the
      // node project's for the same two reasons: it reads `node:os` and
      // `node:child_process`-shaped APIs, and it needs the generated wasm glue
      // that CI's toolchain-free `web-client` job does not build. Named
      // separately because the `src/local/` pattern above does not reach it.
      exclude: [
        ...configDefaults.exclude,
        "src/local/*.node.test.ts",
        "src/precache.node.test.ts",
      ],
      // DOM cleanup between tests is `cleanup()` in `src/test/setup.ts`, not
      // these — an earlier comment here claimed otherwise. What these three do
      // is undo what a test did to the ENVIRONMENT: `vi.fn` spies, `vi.stubEnv`
      // and `vi.stubGlobal`. The last matters most here, because every test
      // replaces the global `fetch`; without it, the first test to stub one
      // would leave it installed for every test after it.
      restoreMocks: true,
      unstubEnvs: true,
      unstubGlobals: true,
    },
  }),
);
