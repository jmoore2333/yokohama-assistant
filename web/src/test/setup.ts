// What every component test gets before it runs.
//
// Three things, and each is here because a test that arranged it itself would
// arrange it slightly differently:
//
// 1. **`jest-dom`'s matchers**, so an assertion about the DOM reads as one
//    (`toBeDisabled()`, `toHaveTextContent()`) rather than as a property check
//    on a node.
// 2. **`fetch` is REMOVED, not mocked to a default.** jsdom in Node 18+ ships a
//    real `fetch` that would happily try to reach `/api/...` and fail
//    asynchronously, after the assertion has already run — the failure then
//    surfaces as an unhandled rejection in a later test. A component that
//    fetches without a test saying what the server answers should fail LOUDLY
//    and in the test that caused it.
// 3. **`IntersectionObserver`/`matchMedia` are absent in jsdom**, and are
//    stubbed only if something needs them. Nothing does today; when something
//    does, it is stubbed here rather than in each test.

import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

beforeEach(() => {
  vi.stubGlobal(
    "fetch",
    vi.fn(async (input: RequestInfo | URL) => {
      throw new Error(
        `this test hit the network: ${String(input)} — stub \`fetch\` with ` +
          `\`stubApi\` (src/test/harness.tsx) and say what the server answers`,
      );
    }),
  );
});

afterEach(() => {
  cleanup();
});
