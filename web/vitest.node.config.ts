import { defineConfig } from "vitest/config";

// **The wasm corpus replay, and it is its OWN project** (M6.5b, spec §6.3).
//
// Environment `node`, no `setupFiles`, and one file in the glob. Every one of
// those is a difference from `vitest.config.ts` rather than a preference:
//
// - `node`, because the module is loaded through `createRequire` and there is
//   no DOM in the claim at all. jsdom would only be a slower way to run it.
// - NO setup file. `src/test/setup.ts` installs jest-dom matchers and a DOM
//   cleanup, which a node project has nothing to do with — and it is the file
//   that would drag `@testing-library` into a suite whose whole subject is a
//   compiled Rust module.
// - TWO files since M6.5b2 — `src/local/*.node.test.ts` and
//   `src/precache.node.test.ts` — both of which `vitest.config.ts` EXCLUDES. So
//   `npm test` keeps its meaning and its count, and the module built by
//   `npm run build:wasm` is a dependency of ONE command rather than of a
//   per-edit gate. The second runs a real `vite build`, so this project's wall
//   is now seconds rather than under one.
//
// It does NOT merge `vite.config.ts`, unlike its sibling: there is no React
// plugin, no alias and no browser resolution in play, and merging the build
// config would put a `test.include` from the other project in scope.
export default defineConfig({
  test: {
    environment: "node",
    // The real-build cell runs `vite build`, which on a cold cache is tens of
    // seconds — well past vitest's 5 s default.
    testTimeout: 180_000,
    include: ["src/local/*.node.test.ts", "src/precache.node.test.ts"],
  },
});
