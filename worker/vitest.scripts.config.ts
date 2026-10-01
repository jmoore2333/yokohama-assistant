import { defineConfig } from "vitest/config";

// **The NODE half of `worker/`'s tests** (the orchestrator's ruling I-31):
// the build scripts' cells and the shell's pure modules. `npm run
// test:scripts` runs them; `vitest.config.ts` runs the pool, over
// `test/**/*.test.ts` inside workerd.
//
// The split is by what a file needs, not by taste. A `scripts/*.test.mjs` cell
// works on text, on a temporary directory or on a committed file, and a
// `src/**/*.unit.test.ts` cell imports a module that touches nothing a Worker
// has — so neither needs the pool, the built module under `gen/`, or
// `.test-assets/`, and this command runs on a checkout that has built nothing.
export default defineConfig({
  test: {
    environment: "node",
    include: ["scripts/**/*.test.mjs", "src/**/*.unit.test.ts"],
  },
});
