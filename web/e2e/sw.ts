// **The browser walk does not register the service worker, and that is a design
// decision rather than a discovery** (M6.5b2, spec §2.3).
//
// `npm run e2e` runs `build:client`, which is a PRODUCTION build, and
// `swRegister.ts` registers only in one. So without a rule every one of the
// walk's existing cells would register a worker and start a ten-megabyte
// precache per context, and every reload-based cell would start reading a
// cache-first navigation instead of the server's answer — quietly changing what
// several dozen assertions are about.
//
// So the client consults a flag before it registers, this fixture sets it, and
// every spec takes `test` from here instead of from `@playwright/test`.
// `every_spec_declares_its_worker.spec.ts` reads the directory at run time and
// names any file that does neither — the shape `every_spec_can_skip.spec.ts`
// already uses, and for its reason: a convention written down once gets broken,
// and this one breaks silently.
//
// **`browser.newContext()` does not get the fixture**, because the override
// below replaces the `context` FIXTURE and a context a spec makes for itself is
// not that one. Every spec that makes its own — the two multiplayer walks,
// the exit's game and `plays_locally`'s fresh browser — uses
// [`newContextWithoutWorker`].
// `every_spec_declares_its_worker.spec.ts` checks that too: it names any spec
// whose source contains `browser.newContext(`.

import { test as base, type Browser, type BrowserContext, type BrowserContextOptions } from "@playwright/test";

/** The flag `serviceWorkerWanted` reads, set before any script of the page
 *  runs. A string rather than a function, so it is the same text in the fixture
 *  and in the helper. */
export const NO_SERVICE_WORKER = "window.__YOKOHAMA_NO_SW__ = true;";

/** Playwright's `test`, with every page in the context flagged.
 *
 *  The `context` fixture is overridden rather than `page`, because `page`
 *  depends on `context` and a per-page init script would miss a second page a
 *  spec opened. */
export const test = base.extend({
  context: async ({ context }, use) => {
    await context.addInitScript(NO_SERVICE_WORKER);
    await use(context);
  },
});

export { expect } from "@playwright/test";

/** `browser.newContext()`, flagged.
 *
 *  A context a spec builds itself never passes through the fixture above, so
 *  this is the only way a multi-browser cell gets the same treatment. */
export async function newContextWithoutWorker(
  browser: Browser,
  options?: BrowserContextOptions,
): Promise<BrowserContext> {
  const context = await browser.newContext(options);
  await context.addInitScript(NO_SERVICE_WORKER);
  return context;
}
