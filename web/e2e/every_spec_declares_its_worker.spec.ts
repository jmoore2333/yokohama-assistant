// **The service-worker opt-out, held by the suite instead of by memory**
// (M6.5b2, spec §2.3).
//
// `npm run e2e` runs a PRODUCTION build, and the client registers a service
// worker in one. A spec that took `test` from `@playwright/test` would
// therefore register a worker, start a ten-megabyte precache per context, and
// turn every reload in that file into a cache-first navigation — quietly
// changing what its assertions are about, without failing.
//
// That is precisely the shape of defect `every_spec_can_skip.spec.ts` exists
// for, and it has already happened once in this suite for the browser-absence
// guard: the convention was recorded in prose, enforced by nothing, and broken
// by the next spec anybody added. So this reads the directory AT RUN TIME and
// checks whatever it finds.
//
// **A spec may opt out, but it has to SAY which way.** One marker, two values:
//
//   @yokohama-service-worker: on    — this spec registers the worker on purpose
//                                     (the offline spec, M6.5b2 Task 10)
//   @yokohama-service-worker: none  — this spec opens no browser page at all
//
// Anything else must import `test` from `./sw`.
//
// **And the second half is checked too**, because it was the easier one to get
// wrong: a context a spec builds with `browser.newContext()` never passes
// through the fixture, since the fixture overrides `context` and that is a
// different object. `e2e/sw.ts` exports `newContextWithoutWorker` for those,
// and the two specs that make their own contexts — `joins_by_link` and
// `two_browsers`, five sites between them — use it. An earlier draft of this
// file said that fact was "held by reading"; it is held by the second
// expectation below.

import { expect, test } from "@playwright/test";
import { readFileSync, readdirSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { haveBrowser } from "../playwright.config";

test.skip(!haveBrowser, "no Chromium: the browser walk ran nothing");

/** The import that flags every page in the spec's context. */
const FIXTURE = /import\s*\{[^}]*\btest\b[^}]*\}\s*from\s*"\.\/sw"/;

/** …or an explicit declaration that this spec does something else. */
const MARKER = /@yokohama-service-worker:\s*(on|none)\b/;

/** A context built by the spec itself, which the fixture cannot reach.
 *
 *  `newContextWithoutWorker(browser)` does not contain this text, so the
 *  helper's callers are not flagged by it. */
const OWN_CONTEXT = /browser\.newContext\(/;

const self = fileURLToPath(import.meta.url);
const here = dirname(self);

test("every browser spec says what it does about the service worker", () => {
  const specs = readdirSync(here)
    .filter((f) => f.endsWith(".spec.ts"))
    .sort();

  // Anti-vacuity, the sibling guard's: a filter over an empty listing passes
  // without reading a file, so a wrong directory or a renamed suffix would read
  // as "all clear" for ever. The listing has to contain THIS file — a fact this
  // file can establish about itself rather than a name it has to remember — and
  // it has to contain others.
  expect(specs, "read the wrong directory: this file is not in its own listing").toContain(
    basename(self),
  );
  expect(specs.length, "no other spec files found next to this one").toBeGreaterThan(1);

  // And the fixture has to exist and do what its name says, or the import every
  // spec is checked for would be an import of nothing.
  const fixture = readFileSync(join(here, "sw.ts"), "utf8");
  expect(fixture, "e2e/sw.ts does not set the flag the client reads").toContain(
    "__YOKOHAMA_NO_SW__",
  );

  const sources = new Map(
    specs
      .filter((f) => f !== basename(self))
      .map((f) => [f, readFileSync(join(here, f), "utf8")] as const),
  );

  const checked = [...sources].filter(([, src]) => !MARKER.test(src));
  // The other half of the anti-vacuity: if every spec carried a marker there
  // would be nothing left to check, and this test would pass by exempting the
  // whole suite.
  expect(checked.length, "every spec is exempt, so this guard checks nothing").toBeGreaterThan(1);

  const missing = checked.filter(([, src]) => !FIXTURE.test(src)).map(([f]) => f);
  expect(
    missing,
    'these specs take `test` from "@playwright/test" and carry no ' +
      "`@yokohama-service-worker:` marker, so each of their contexts registers the release's " +
      "worker and starts a precache — import { test, expect } from \"./sw\" instead",
  ).toEqual([]);

  // **The fixture reaches the fixture's context and no other.** A spec that
  // calls `browser.newContext()` gets a context the override never touched, so
  // its pages register the worker however carefully the spec imported `test`.
  const ownContext = checked.filter(([, src]) => OWN_CONTEXT.test(src)).map(([f]) => f);
  expect(
    ownContext,
    "these specs build their own context with `browser.newContext(`, which the `context` fixture " +
      "cannot reach — use `newContextWithoutWorker(browser)` from \"./sw\" instead",
  ).toEqual([]);
  // Anti-vacuity for that half: the helper it points at has to exist and has to
  // be the thing that flags nobody.
  expect(fixture, "e2e/sw.ts exports no newContextWithoutWorker").toContain(
    "export async function newContextWithoutWorker",
  );
});
