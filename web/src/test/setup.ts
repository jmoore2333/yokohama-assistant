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
// 4. **`Element.prototype.scrollIntoView` is a no-op, because jsdom has none.**
//    `TurnAccount` scrolls the walk's lit row into view (spec §3.2.5), and
//    without this every cell that renders a running walk dies on
//    "scrollIntoView is not a function". A no-op is the honest stub: jsdom
//    implements no layout, so there is nothing a real one could do — and the
//    cell that is ABOUT the scroll installs its own recorder over this one.
// 5. **`localStorage` is an in-memory `Storage`, because Node's is not one.**
//    jsdom does not install one under this environment (`window.localStorage`
//    is `undefined`), so `globalThis.localStorage` resolved to NODE 26's own
//    getter — which warns `ExperimentalWarning: localStorage is not available
//    because --localstorage-file was not provided` and then returns
//    `undefined`. The gate printed that twice per run, once per worker, and a
//    gate that prints a warning it does not mean teaches its readers to ignore
//    warnings. The read itself was always harmless: `sections.ts` treats a
//    storage that is not there as the default set, which is the behaviour its
//    own cells pin by passing a fake one. This gives the browser-shaped object
//    the client expects, fresh for every test.

import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, vi } from "vitest";
import { cleanup } from "@testing-library/react";

Element.prototype.scrollIntoView = () => {};

/** The `Storage` interface, in a `Map`. Nothing here persists past the test. */
function memoryStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (k: string) => items.get(k) ?? null,
    key: (i: number) => [...items.keys()][i] ?? null,
    removeItem: (k: string) => void items.delete(k),
    setItem: (k: string, v: string) => void items.set(k, String(v)),
  };
}

beforeEach(() => {
  vi.stubGlobal("localStorage", memoryStorage());
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
