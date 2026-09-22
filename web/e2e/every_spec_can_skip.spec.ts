// **The browser-absence opt-out, held by the suite instead of by memory.**
//
// `playwright.config.ts` drops the `webServer` block when there is no Chromium,
// so under `YOKOHAMA_WEB_E2E_OPTIONAL=1` a spec that does not skip does not
// merely run without a browser — it runs without a SERVER too, and fails. The
// documented behaviour ("absence becomes a skip that says out loud it tested
// nothing") therefore depends on every single spec file carrying
// `test.skip(!haveBrowser, …)`, and one file missing the line is enough to make
// the escape hatch exit non-zero.
//
// That has now happened twice. M2.5 wrote the line into both specs it had; M3
// added a third spec and did not, and the opt-out went back to `4 failed, 7
// skipped` while three tracked documents went on asserting it worked. The
// defect is not that someone forgot — it is that the convention was recorded in
// prose and enforced by nothing, so forgetting was the default.
//
// So the convention is executable. This reads the directory AT RUN TIME and
// checks whatever it finds: a spec added tomorrow is checked tomorrow, without
// anyone adding it to a list. A hand-written list of spec files would be the
// same defect one level up — a second place to remember, drifting from the
// first.
//
// **@yokohama-service-worker: none** — this spec opens no browser page at all,
// so it takes no service-worker fixture and says so in the one place
// `every_spec_declares_its_worker.spec.ts` reads (M6.5b2).
//
// **Why this one file does not carry the guard.** It asks the filesystem a
// question and declares no `page`, `context` or `browser` fixture, and
// Playwright builds fixtures lazily, so it needs neither a browser nor a
// server: it is the one spec that can honestly run in the very configuration it
// exists to protect, and skipping it there would leave the opt-out run once
// again checking nothing. The exemption is COMPUTED from `import.meta.url`
// rather than written down — it names this file by being this file — so it
// cannot quietly grow into a list.

import { expect, test } from "@playwright/test";
import { readFileSync, readdirSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

/** The line itself. Anchored at the start of a line, so a `test.skip` nested
 *  inside some test body — which skips one case, not the file — cannot satisfy
 *  it. */
const GUARD = /^test\.skip\(!haveBrowser,/m;

/** …and the import that makes the line mean anything. A file with the guard and
 *  no import does not compile, but checking both says which half is missing. */
const IMPORT = /import\s*\{[^}]*\bhaveBrowser\b[^}]*\}\s*from\s*"\.\.\/playwright\.config"/;

const self = fileURLToPath(import.meta.url);
const here = dirname(self);

test("every browser spec can skip itself when there is no browser", () => {
  const specs = readdirSync(here)
    .filter((f) => f.endsWith(".spec.ts"))
    .sort();

  // **Anti-vacuity, and it is the whole risk in a check like this.** A filter
  // over an empty listing passes without reading a single file, so a wrong
  // directory or a renamed suffix would read as "all clear" forever. The
  // listing has to contain THIS file — which is a fact this file can establish
  // about itself, not a name it has to remember — and it has to contain other
  // files too, or there is nothing here to check.
  expect(specs, "read the wrong directory: this file is not in its own listing").toContain(
    basename(self),
  );
  expect(specs.length, "no other spec files found next to this one").toBeGreaterThan(1);

  const sources = new Map(
    specs
      .filter((f) => f !== basename(self))
      .map((f) => [f, readFileSync(join(here, f), "utf8")] as const),
  );

  const noGuard = [...sources].filter(([, src]) => !GUARD.test(src)).map(([f]) => f);
  expect(
    noGuard,
    'these specs carry no `test.skip(!haveBrowser, "…")` at file level, so ' +
      "YOKOHAMA_WEB_E2E_OPTIONAL=1 fails on them instead of skipping them",
  ).toEqual([]);

  const noImport = [...sources].filter(([, src]) => !IMPORT.test(src)).map(([f]) => f);
  expect(
    noImport,
    'these specs do not import `haveBrowser` from "../playwright.config"',
  ).toEqual([]);
});
