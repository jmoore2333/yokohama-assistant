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
import { EXIT, EXIT_LAUNCH_ARGS, EXIT_SPEC, LIVE, LIVE_SPECS, LOCAL_PORT, README_SHOTS, README_SHOTS_SPEC, baseUrlFor } from "./e2e/live";

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
 *
 * The number itself is `e2e/live.ts`'s `LOCAL_PORT`, beside `baseUrlFor`, so
 * the origin a walk reads has one definition.
 */
const PORT = LOCAL_PORT;
/**
 * The origin every walk is played at — and under `LIVE` the deployed Worker's
 * (M6.5c, spec §5.4; `e2e/live.ts`). It is THIS constant that changes and not
 * only `use.baseURL` below: `watchForBreakage` flags failed responses whose
 * address starts with it, and `two_browsers.spec.ts` asks it for the event
 * count, so a live run with a local `BASE_URL` would watch nothing and ask a
 * port nobody is listening on (plan review M9). A node cell reads this line
 * (`e2e/harness.node.test.ts`).
 */
export const BASE_URL = baseUrlFor(LIVE);

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
// reporter prints one SKIPPED test per browser test — thirty-seven at this
// writing, which is every test in the glob except the structural one below, a
// reading rather than a contract — with the reason attached, and the run exits
// 0. A skip you can count is not a skip you can miss.
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
  return fresh(join(tmpdir(), `yokohama-e2e-${PORT}`));
}

/** Where the walk's server keeps its ARCHIVE (M6.5b, spec §6.5).
 *
 *  A browser game is copied to the SERVER by `POST /api/archive`, so the exit
 *  walk needs a deployment that keeps one — and the only honest way to read
 *  what landed is off the disk the server wrote it to. Its own scratch
 *  directory beside the games, for the same reason that one is scratch: a walk
 *  must not write into whatever the operator is actually keeping.
 *
 *  **A READER, not a maker.** `gamesDir` empties what it hands back, and this
 *  module is loaded by every Playwright process — the runner's, and each test
 *  worker's. A spec calling an emptying helper to find out where the file is
 *  would delete the file it came to read, so the emptying is left to the `env`
 *  map below and the spec reads this.
 *
 *  **Which happens at each config EVALUATION, not once per run** (whole-branch
 *  review, vacuity #3): every process that loads this module runs `fresh()`
 *  again, the shape `gamesDir` has had since M6.1. Harmless at `workers: 1`
 *  unless a worker restarts mid-run. */
export function archiveDir(): string {
  return join(tmpdir(), `yokohama-e2e-archive-${PORT}`);
}

/** An empty directory at `dir`. */
function fresh(dir: string): string {
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
  //
  // **The node cells beside the specs are vitest's, never Playwright's**
  // (M6.5c): `e2e/*.node.test.ts` runs under `npm run test:wasm`, and its name
  // matches Playwright's default `*.test.ts` glob — loaded here it calls
  // `vi.mock` with no vitest around it, and the whole run collects nothing.
  //
  // **The exit's headed game is opt-in the same way** (ruling I-88): ignored
  // from every run unless `YOKOHAMA_WEB_EXIT=1`, and still checked as a file by
  // the two structural specs. With the switch and no live target it is listed
  // and skips itself, saying why.
  //
  // **The README's screenshot generator is opt-in the same way**
  // (`YOKOHAMA_WEB_README_SHOTS=1`, Task 8): it writes into `worker/docs/img/`
  // (or `YOKOHAMA_WEB_README_SHOTS_DIR`), so a plain run must never collect
  // it. Unlike the exit's game it runs on the local target too — the dry run.
  testIgnore: [
    "**/*.node.test.ts",
    ...(process.env.YOKOHAMA_WEB_SHOTS === "1" ? [] : ["**/shots.spec.ts"]),
    ...(process.env.YOKOHAMA_WEB_GATES === "1" ? [] : ["**/gates.spec.ts"]),
    ...(EXIT ? [] : [`**/${EXIT_SPEC}`]),
    ...(README_SHOTS ? [] : [`**/${README_SHOTS_SPEC}`]),
  ],
  // **The live tier is TWO specs** (spec §5.4): the two multiplayer walks —
  // `LIVE_SPECS`, the one list — and the exit's game beside them when it is
  // asked for. Every other spec stays on the local binary, and these keep
  // their local run as well.
  ...(LIVE
    ? {
        testMatch: [...LIVE_SPECS, ...(EXIT ? [EXIT_SPEC] : []), ...(README_SHOTS ? [README_SHOTS_SPEC] : [])].map((f) => `**/${f}`),
      }
    : {}),
  fullyParallel: false,
  // One at a time, live as locally — which also makes the live run present
  // the credential once per worker process: once per green run, since
  // Playwright restarts the worker after a failing test (`e2e/live.ts`).
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
    // **No trace live.** MEASURED (Playwright 1.62.1): a retained trace keeps
    // an `APIRequestContext`'s headers — which is why the credential never
    // goes through one (`e2e/live.ts`) — and every page request's cookies, and
    // a live context's `CF_Authorization` cookie IS the Service Auth
    // credential's session. So a failing live run keeps NO trace. It still
    // keeps its screenshot (below — pixels, no header), the lines the specs
    // print, and its error — of which a thrown API call's would list the
    // cookie in its call log, so every live `request.*` call goes through
    // `liveRequest`, which withholds that log (`e2e/live.ts`).
    trace: LIVE ? "off" : "retain-on-failure",
    screenshot: "only-on-failure",
    // **The exit's headed windows, meant to stay VISIBLE when stacked** (R3's
    // R-N6; ruling I-91): the two switches `EXIT_LAUNCH_ARGS` names
    // (`e2e/live.ts`), which Playwright 1.62.1 already passes to every
    // Chromium — named here so the exit's precondition does not rest on a
    // default. On macOS they do not keep a covered window visible (the exit's
    // first headed run), so the spec lays its windows out so that none covers
    // another as well.
    // Every spec of an exit run is launched with them; no other run changes.
    ...(EXIT ? { launchOptions: { args: [...EXIT_LAUNCH_ARGS] } } : {}),
  },
  // **No server live**: the Worker is the server, and nothing local is
  // started, built or emptied.
  ...(haveBrowser && !LIVE
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
            // There is NO default identity source (M6.1 R-1.3): the server
            // exits 1 before it binds anything without this, so a walk that
            // omitted it would fail at start-up rather than in a test.
            // `open` is the self-host door and this walk's — the client attaches the
            // header from `localStorage`, and the walk puts it there by typing
            // into the sign-in screen (`harness.ts`'s `signIn`), which is the
            // only shape that also tests the screen.
            YOKOHAMA_WEB_IDENTITY: "open",
            // ONE directory, reused and cleared, rather than `mkdtempSync`.
            // This module is loaded by every Playwright invocation — including
            // `--list` and each worker — so a fresh temp directory per load
            // leaked one per run, and the one holding the walk's records was
            // whichever load happened to be the server's.
            YOKOHAMA_WEB_DIR: gamesDir(),
            // **The archive is ON for the walk** (M6.5b, spec §6.5), because
            // the one thing "Send to the archive" claims is that a copy of the
            // browser's own bytes reaches the server — and a deployment that
            // keeps none answers 404 and claims nothing. It also makes a
            // playing table's Abandon read "Abandon and save" on BOTH hosts,
            // which is the label the archive makes true.
            YOKOHAMA_WEB_ARCHIVE_DIR: fresh(archiveDir()),
          },
        },
      }
    : {}),
});
