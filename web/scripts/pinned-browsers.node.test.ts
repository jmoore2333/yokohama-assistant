// **The pinned-browser instrument's cells** (M6.5c2 cycle F, Task 8; R-F3,
// Review Focus 1). The instrument drives a real Chromium against a LIVE host,
// so it is the orchestrator's alone and nothing here launches a browser: the
// command line is run with no arguments and must refuse with its usage line
// before it loads Playwright, and the readings it decides by — the door's
// exact sentences, the two snapshots' checks — are pure functions read over
// fixtures.
//
// Node and not Playwright, so `npm run test:wasm` runs it where CI installs no
// Chromium.

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  afterClickChecks,
  afterReloadChecks,
  doorReadiness,
  pinnedArgs,
  SENTENCES,
  USAGE,
  type AfterClick,
  type AfterReload,
  type TabReading,
} from "./pinned-browsers.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(HERE, "pinned-browsers.mjs");

describe("the command line", () => {
  it("refuses no arguments with exit 2 and the usage line, launching nothing", () => {
    const run = spawnSync(process.execPath, [SCRIPT], { encoding: "utf8", timeout: 30_000 });
    expect(run.status).toBe(2);
    expect(run.stderr).toBe(`${USAGE}\n`);
    expect(run.stdout).toBe("");
  });
  it("says its usage as the plan's commands spell it", () => {
    expect(USAGE).toBe("usage: node web/scripts/pinned-browsers.mjs <label> <origin> <state-dir>");
  });
  it("takes a label, an origin and a state directory", () => {
    expect(pinnedArgs(["worker", "https://yokohama.moore.nyc", "web/scripts/pinned/worker"])).toEqual({
      label: "worker",
      origin: "https://yokohama.moore.nyc",
      stateDir: "web/scripts/pinned/worker",
    });
    expect(pinnedArgs(["dry", "http://127.0.0.1:8491", "/tmp/x"]).origin, "plain http on loopback, a dry run").toBe(
      "http://127.0.0.1:8491",
    );
  });
  it.each([
    [[], "usage"],
    [["worker", "https://yokohama.moore.nyc"], "usage"],
    [["worker", "https://yokohama.moore.nyc", "d", "extra"], "usage"],
    [["a label", "https://yokohama.moore.nyc", "d"], "label"],
    [["worker", "yokohama.moore.nyc", "d"], "origin"],
    [["worker", "http://yokohama.moore.nyc", "d"], "origin"],
    [["worker", "https://yokohama.moore.nyc/", "d"], "origin"],
    [["worker", "https://yokohama.moore.nyc/#/lobby", "d"], "origin"],
    [["worker", "https://yokohama.moore.nyc", ""], "usage"],
  ])("refuses %j", (argv, why) => {
    expect(() => pinnedArgs(argv)).toThrow(why);
  });
  it("names no absolute path of anyone's machine: Playwright is found beside it", () => {
    const source = readFileSync(SCRIPT, "utf8");
    expect(source).toContain('createRequire(new URL("../package.json", import.meta.url))');
    expect(source).not.toMatch(/["'`](\/Users\/|\/home\/|[A-Z]:\\)/);
  });
});

describe("the door's sentences, exactly as cycle F's door draws them (Task 1's report)", () => {
  it("are the four the instrument reads", () => {
    expect(SENTENCES.installing).toBe("Preparing to play offline…");
    expect(SENTENCES.ready).toBe("Ready to play offline");
    expect(SENTENCES.update).toBe("A new version is ready. Reload to use it");
    expect(SENTENCES.failed.test("Could not prepare to play offline — the install did not finish. Try again")).toBe(true);
  });
  it("are matched whole: a period, a prefix or a suffix is another sentence", () => {
    expect(doorReadiness(["Ready to play offline"])).toBe("ready");
    expect(doorReadiness(["Ready to play offline."])).toBe("unknown");
    expect(doorReadiness(["Ready to play offline now"])).toBe("unknown");
    expect(doorReadiness(["Preparing to play offline…"])).toBe("installing");
    expect(doorReadiness(["Preparing to play offline..."])).toBe("unknown");
    expect(doorReadiness(["Could not prepare to play offline — a 503. Try again"])).toBe("failed");
    expect(doorReadiness([])).toBe("none");
    expect(doorReadiness(["Ready to play offline", SENTENCES.update])).toBe("ready");
  });
});

/** The game tab in a good reading. */
const GOOD_TAB: TabReading = { lines: [SENTENCES.update], loads: 0, url: "https://h.example/#/t/local-1", events: 9 };

/** A good reading after the door's plain reload, which each cell spoils once. */
const GOOD_RELOAD: AfterReload = {
  pinnedRelease: "yokohama-release-1111111111111111",
  cachesBeforeGo: ["yokohama-release-1111111111111111"],
  release: "yokohama-release-2222222222222222",
  caches: ["yokohama-release-1111111111111111", "yokohama-release-2222222222222222"],
  waiting: null,
  active: "activated",
  doorLines: ["Ready to play offline"],
  doorLoads: 2,
  serviceWorkerPaths: ["/sw.js", "/shell.bin", "/assets/index-x.js"],
  tab: GOOD_TAB,
  tabBefore: { url: "https://h.example/#/t/local-1", events: 9 },
};

const failed = (r: AfterReload) => afterReloadChecks(r).filter((c) => !c.ok).map((c) => c.check);

describe("after the plain reload: the door takes the release by itself, the game tab says so", () => {
  it("passes a reading where every fact holds", () => {
    expect(failed(GOOD_RELOAD)).toEqual([]);
  });
  it.each<[string, Partial<AfterReload>, string]>([
    [
      "a release the door took before go, by a check of its own",
      { cachesBeforeGo: ["yokohama-release-1111111111111111", "yokohama-release-2222222222222222"] },
      "the door had not taken a release before go",
    ],
    ["no new release deployed", { release: GOOD_RELOAD.pinnedRelease }, "the deployed release is not the pinned one"],
    ["a door that did not reload itself", { doorLoads: 1 }, "the door reloaded itself exactly once"],
    ["a door that reloaded twice", { doorLoads: 3 }, "the door reloaded itself exactly once"],
    ["a door still offering the click", { doorLines: ["Ready to play offline", SENTENCES.update] }, "the door draws no update line"],
    ["a door still installing", { doorLines: [SENTENCES.installing] }, "the door reads exactly Ready to play offline"],
    ["a worker still waiting", { waiting: "installed" }, "no worker waits, and the active one is activated"],
    ["the old generation dropped", { caches: ["yokohama-release-2222222222222222"] }, "both generations stand"],
    ["an install that never fetched the shell", { serviceWorkerPaths: ["/sw.js"] }, "the install fetched /shell.bin and no HTML"],
    ["an install that fetched the HTML", { serviceWorkerPaths: ["/shell.bin", "/index.html"] }, "the install fetched /shell.bin and no HTML"],
    ["an install that fetched the root", { serviceWorkerPaths: ["/shell.bin", "/"] }, "the install fetched /shell.bin and no HTML"],
  ])("fails %s", (_what, spoil, check) => {
    expect(failed({ ...GOOD_RELOAD, ...spoil })).toEqual([check]);
  });
  it.each<[string, Partial<TabReading>, string]>([
    ["a game tab with no line", { lines: [] }, "the game tab draws the update line"],
    ["a game tab that reloaded", { loads: 1 }, "the game tab did not reload"],
    ["a game tab on another address", { url: "https://h.example/#/" }, "the game tab's game is unchanged"],
    ["a game tab whose game moved", { events: 10 }, "the game tab's game is unchanged"],
  ])("fails %s", (_what, spoil, check) => {
    expect(failed({ ...GOOD_RELOAD, tab: { ...GOOD_TAB, ...spoil } })).toEqual([check]);
  });
  it("reads no game tab where none was opened", () => {
    const checks = afterReloadChecks({ ...GOOD_RELOAD, tab: null, tabBefore: null }).map((c) => c.check);
    expect(checks.some((c) => c.startsWith("the game tab"))).toBe(false);
  });
});

const GOOD_CLICK: AfterClick = {
  before: { url: "https://h.example/#/t/local-1", events: 9 },
  tab: { lines: [], loads: 1, url: "https://h.example/#/t/local-1", events: 9 },
  release: "yokohama-release-2222222222222222",
  eventsAfterAMove: 12,
};

const clickFailed = (r: AfterClick) => afterClickChecks(r).filter((c) => !c.ok).map((c) => c.check);

describe("after the game tab's click: the same game, under the new release", () => {
  it("passes a reading where every fact holds", () => {
    expect(clickFailed(GOOD_CLICK)).toEqual([]);
  });
  it.each<[string, Partial<AfterClick>, string]>([
    ["a tab that did not reload", { tab: { ...GOOD_CLICK.tab, loads: 0 } }, "the game tab reloaded once"],
    ["a tab still drawing the line", { tab: { ...GOOD_CLICK.tab, lines: [SENTENCES.update] } }, "the game tab draws no update line"],
    ["another game", { tab: { ...GOOD_CLICK.tab, url: "https://h.example/#/" } }, "the same game, at the same move"],
    ["a lost move", { tab: { ...GOOD_CLICK.tab, events: 8 } }, "the same game, at the same move"],
    ["a game that did not go on", { eventsAfterAMove: 9 }, "the game goes on"],
    ["a move that could not be made", { eventsAfterAMove: null }, "the game goes on"],
  ])("fails %s", (_what, spoil, check) => {
    expect(clickFailed({ ...GOOD_CLICK, ...spoil })).toEqual([check]);
  });
});
