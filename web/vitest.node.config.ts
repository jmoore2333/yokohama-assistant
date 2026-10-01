import { defineConfig } from "vitest/config";

// **The wasm corpus replay, and it is its OWN project** (M6.5b, spec §6.3).
//
// Environment `node`, no `setupFiles`, and globs of its own. Every one of
// those is a difference from `vitest.config.ts` rather than a preference:
//
// - `node`, because the module is loaded through `createRequire` and there is
//   no DOM in the claim at all. jsdom would only be a slower way to run it.
// - NO setup file. `src/test/setup.ts` installs jest-dom matchers and a DOM
//   cleanup, which a node project has nothing to do with — and it is the file
//   that would drag `@testing-library` into a suite whose whole subject is a
//   compiled Rust module.
// - Every `src/**/*.node.test.ts`, which `vitest.config.ts` EXCLUDES by the
//   same suffix. So `npm test` keeps its meaning and its count, and the module
//   built by `npm run build:wasm` is a dependency of ONE command rather than of
//   a per-edit gate. Since M6.5b2 that is the wasm corpus replay, the
//   real-build precache cell (a real `vite build`), the driven service worker
//   (esbuild, which refuses to run under jsdom at all) and the restore
//   validator through the module — so this project's wall is seconds rather
//   than under one.
// - And every `e2e/**/*.node.test.ts` (M6.5c): the live tier's node cells —
//   the one console error it forgives, and the credential's path into a
//   browser — which drive `e2e/`'s helpers through fakes, with no browser.
//   `vitest.config.ts` never sees `e2e/`: its `include` is `src`.
// - And every `scripts/**/*.node.test.ts` (M6.5c2): a build script's own
//   cells, beside the script — `project-url.mjs`, which `vite.config.ts` calls.
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
    include: ["src/**/*.node.test.ts", "e2e/**/*.node.test.ts", "scripts/**/*.node.test.ts"],
  },
});
