// The headless walk: the browser analogue of `piped_dogfood`.
//
// The component suite renders React into jsdom, which is a DOM implementation
// and not a browser: it has no layout, no paint, no real focus model, and it
// does not fetch an image. So the two things the component suite structurally
// cannot see are the two this exists for — that the SCREEN can be played
// through to a winner, and that what it draws actually loads.
//
// It drives the real Rust binary serving the real `dist` build. Nothing is
// stubbed: the server plays the game, the client draws it, and the walk clicks
// what is on the screen.

import { defineConfig, devices } from "@playwright/test";
import { chromium } from "@playwright/test";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/**
 * A port of its own, and NEVER a shared one.
 *
 * The first draft used 8099 with `reuseExistingServer: !CI`, and a stale
 * `yokohama-web` left listening on 8099 by an earlier session answered instead
 * — with the operator's real `~/.yokohama/games` behind it. The walk then read
 * a saved game it had never played and failed on a seed from someone else's
 * afternoon, which is a confusing failure and, worse, a walk writing into a
 * directory it does not own. So: an unusual port, and reuse is OFF, so a
 * collision is a loud bind failure rather than a silent conversation with a
 * stranger.
 */
const PORT = 8479;
export const BASE_URL = `http://127.0.0.1:${PORT}`;

/**
 * A browser has to be INSTALLED, and absence is loud by default.
 *
 * The same rule the repo already applies to the gitignored BGA captures: a
 * missing prerequisite PANICS with a named message, because a silent skip makes
 * "verified" indistinguishable from "not run" — and this suite's whole job is
 * to make "the suites are green" a claim about the browser.
 * `YOKOHAMA_WEB_E2E_OPTIONAL=1` turns absence back into a skip, for a machine
 * that deliberately has no browser.
 */
const browserPath = chromium.executablePath();
/** Exported so the specs can SKIP rather than vanish — see below. */
export const haveBrowser = Boolean(browserPath) && existsSync(browserPath);
const optional = process.env.YOKOHAMA_WEB_E2E_OPTIONAL === "1";

if (!haveBrowser && !optional) {
  throw new Error(
    "no Chromium for the browser walk. Install it once with `npx playwright install chromium`, " +
      "or set YOKOHAMA_WEB_E2E_OPTIONAL=1 to turn its absence into a skip on a machine that " +
      "deliberately has none. It is loud by default because a silent skip makes a green run " +
      "indistinguishable from a run that tested nothing.",
  );
}
if (!haveBrowser) {
  // eslint-disable-next-line no-console
  console.warn(
    "YOKOHAMA_WEB_E2E_OPTIONAL=1 and no Chromium is installed: the browser walk ran NOTHING. " +
      "The claim 'the suites are green' does not cover the browser in this run.",
  );
}

// **The opt-out SKIPS; it does not hide.**
//
// The first draft set `testIgnore: /.*/` under the opt-out, which leaves
// Playwright with zero discovered tests — and Playwright treats that as an
// error, so the documented escape hatch exited 1 anyway. On the one machine it
// exists for, the operator's only remaining move was to drop the command from
// the gate list, which is the silent skip the whole design was avoiding.
//
// Every BROWSER spec carries `test.skip(!haveBrowser, …)` instead, so the
// reporter prints one SKIPPED test per browser test — eleven at this writing,
// a reading rather than a contract — with the reason attached, and the run
// exits 0. A skip you can count is not a skip you can miss.
//
// **And the convention is enforced rather than remembered.** It was written
// down here once and then broken anyway: M3 added a third spec without the
// line, and the opt-out went back to `4 failed, 7 skipped` — while this comment
// (which said "seven" of what were by then eleven tests) and two other tracked
// documents went on describing behaviour the suite no longer had.
// `e2e/every_spec_can_skip.spec.ts` now reads the spec directory AT RUN TIME
// and names any file missing the guard. It declares no browser fixture, so it
// is the one spec that runs — and therefore checks — in the very configuration
// this paragraph is about.
/** A single scratch games directory for the walk, emptied before it starts.
 *  The server persists every applied move as its `GameRecord`, and a walk must
 *  not write into whatever the operator is actually playing. */
function gamesDir(): string {
  const dir = join(tmpdir(), `yokohama-e2e-${PORT}`);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

export default defineConfig({
  testDir: "./e2e",
  // **The screenshot generator is opt-in, and it is not a test.**
  //
  // `shots.spec.ts` WRITES into `web/public/help/`, so leaving it in the default
  // run would make every `npm run e2e` dirty the working tree — against this
  // repo's standing rule that a clean `git status` is the steady state — and
  // would silently re-cut documentation as a side effect of running the suite.
  //
  // It stays a `.spec.ts` on purpose rather than being renamed out of the glob:
  // `every_spec_can_skip.spec.ts` reads this directory at run time and checks
  // whatever `.spec.ts` files it finds, so a generator hidden behind a different
  // suffix would escape the browser-absence guard that every other file here
  // has to satisfy. Ignored from the RUN, still checked as a file.
  //
  //     cd web && npm run build && YOKOHAMA_WEB_SHOTS=1 npx playwright test shots.spec.ts
  testIgnore: [
    ...(process.env.YOKOHAMA_WEB_SHOTS === "1" ? [] : ["**/shots.spec.ts"]),
    ...(process.env.YOKOHAMA_WEB_GATES === "1" ? [] : ["**/gates.spec.ts"]),
  ],
  fullyParallel: false,
  workers: 1,
  // A whole game is hundreds of clicks against a real server. The default 30s
  // is a per-test timeout, not a per-action one.
  timeout: 10 * 60 * 1000,
  expect: { timeout: 10_000 },
  // A bounded per-ACTION timeout, separate from the per-test one. Without it a
  // locator that resolved and then vanished between two calls waits out the
  // whole ten-minute test budget, and the walk reports "timeout" where the
  // truth is "the screen changed under me".
  reporter: process.env.CI ? "line" : "list",
  use: {
    baseURL: BASE_URL,
    actionTimeout: 15_000,
    navigationTimeout: 30_000,
    ...devices["Desktop Chrome"],
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  ...(haveBrowser
    ? {
        webServer: {
          // Run from the REPO ROOT, where the server's own defaults resolve:
          // `web/dist` for the client and `crates/yokohama-web/assets/art` for
          // the pictures. Naming them here would be a second copy of a default
          // that already has a home.
          // The client build is the npm script's job (`npm run e2e` is
          // `npm run build && playwright test`), because Playwright starts the
          // server and the server serves whatever `web/dist` holds. Running
          // `npx playwright test` directly is therefore a walk over a STALE or
          // ABSENT build — so the server is asked for its own opinion first,
          // and refuses with the command to run.
          command: "cargo run --release -p yokohama-web",
          cwd: "..",
          url: `${BASE_URL}/api/health`,
          // A release build from cold is minutes; a warm one is seconds.
          timeout: 15 * 60 * 1000,
          reuseExistingServer: false,
          stdout: "pipe",
          stderr: "pipe",
          env: {
            YOKOHAMA_WEB_PORT: String(PORT),
            // ONE directory, reused and cleared, rather than `mkdtempSync`.
            // This module is loaded by every Playwright invocation — including
            // `--list` and each worker — so a fresh temp directory per load
            // leaked one per run, and the one holding the walk's records was
            // whichever load happened to be the server's.
            YOKOHAMA_WEB_DIR: gamesDir(),
          },
        },
      }
    : {}),
});
