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
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import {
  afterClickChecks,
  afterReloadChecks,
  awaitGo,
  doorReadiness,
  goCheck,
  GO_WAIT_DEFAULT_MINUTES,
  pin,
  PIN_CHECK,
  pinnedArgs,
  SENTENCES,
  USAGE,
  verdictOf,
  writeVerdict,
  type Check,
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
  it("says its usage as the plan's commands spell it, the opt-out named", () => {
    expect(USAGE).toBe(
      "usage: node web/scripts/pinned-browsers.mjs <label> <origin> <state-dir> [--no-second-tab] [--go-wait <minutes>]",
    );
  });
  it("takes a label, an origin and a state directory, wants the second tab, and waits three hours for go", () => {
    expect(pinnedArgs(["worker", "https://yokohama.moore.nyc", "web/scripts/pinned/worker"])).toEqual({
      label: "worker",
      origin: "https://yokohama.moore.nyc",
      stateDir: "web/scripts/pinned/worker",
      secondTab: true,
      goWaitMinutes: 180,
    });
    expect(pinnedArgs(["dry", "http://127.0.0.1:8491", "/tmp/x"]).origin, "plain http on loopback, a dry run").toBe(
      "http://127.0.0.1:8491",
    );
  });
  it.each([
    [[], "usage"],
    [["worker", "https://yokohama.moore.nyc"], "usage"],
    [["worker", "https://yokohama.moore.nyc", "d", "extra"], "usage"],
    [["worker", "https://yokohama.moore.nyc", "d", "--no-second-tab", "--no-second-tab"], "usage"],
    [["worker", "https://yokohama.moore.nyc", "--no-second-tab"], "usage"],
    [["a label", "https://yokohama.moore.nyc", "d"], "label"],
    [["worker", "yokohama.moore.nyc", "d"], "origin"],
    [["worker", "http://yokohama.moore.nyc", "d"], "origin"],
    [["worker", "https://yokohama.moore.nyc/", "d"], "origin"],
    [["worker", "https://yokohama.moore.nyc/#/lobby", "d"], "origin"],
    [["worker", "https://yokohama.moore.nyc", ""], "usage"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait"], "usage"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "60", "--go-wait", "60"], "usage"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "0"], "--go-wait takes whole minutes from 1 to 1440"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "1.5"], "--go-wait takes whole minutes from 1 to 1440"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "-5"], "--go-wait takes whole minutes from 1 to 1440"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "180m"], "--go-wait takes whole minutes from 1 to 1440"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "1441"], "--go-wait takes whole minutes from 1 to 1440"],
    [["worker", "https://yokohama.moore.nyc", "d", "--go-wait", "--no-second-tab"], "--go-wait takes whole minutes from 1 to 1440"],
  ])("refuses %j", (argv, why) => {
    expect(() => pinnedArgs(argv)).toThrow(why);
  });
  it("takes --no-second-tab after the three as the ONE way to run without the stale-tab half", () => {
    expect(pinnedArgs(["nas", "https://yokohama-nas.moore.nyc", "d", "--no-second-tab"]).secondTab).toBe(false);
  });
  // **The wait for go is an argument** (cycle F's residual fix; the Opus
  // re-review): E's own pin-to-go gaps were 31.6 minutes (the Worker) and
  // 35.4 (the NAS), past the thirty minutes the fix wave ruled, so the bound
  // is `--go-wait <minutes>`, three hours unless given, before or after
  // `--no-second-tab`. Mutants: the argument unread; the default left at 30.
  it("takes --go-wait <minutes>, before or after --no-second-tab", () => {
    const base = ["worker", "https://yokohama.moore.nyc", "d"];
    expect(pinnedArgs([...base, "--go-wait", "240"])).toMatchObject({ secondTab: true, goWaitMinutes: 240 });
    expect(pinnedArgs([...base, "--no-second-tab", "--go-wait", "45"])).toMatchObject({ secondTab: false, goWaitMinutes: 45 });
    expect(pinnedArgs([...base, "--go-wait", "45", "--no-second-tab"])).toMatchObject({ secondTab: false, goWaitMinutes: 45 });
    expect(pinnedArgs([...base, "--go-wait", "1440"]).goWaitMinutes, "a day at most").toBe(1440);
    expect(pinnedArgs([...base, "--no-second-tab"]).goWaitMinutes, "the default beside the other flag").toBe(GO_WAIT_DEFAULT_MINUTES);
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
  secondTab: true,
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

const failed = (r: AfterReload) => afterReloadChecks(r).filter((c) => c.ok === false).map((c) => c.check);
const notRun = (r: AfterReload) => afterReloadChecks(r).filter((c) => c.ok === null).map((c) => c.check);
const TAB_CHECKS = ["the game tab draws the update line", "the game tab did not reload", "the game tab's game is unchanged"];

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
  it("FAILS a second tab that was wanted and did not open, its three checks not run (Task 8 review, Important 2)", () => {
    const r = { ...GOOD_RELOAD, tab: null, tabBefore: null };
    expect(failed(r)).toEqual(["the second tab holds a local game"]);
    expect(notRun(r)).toEqual(TAB_CHECKS);
    expect(verdictOf(afterReloadChecks(r)).ok, "the verdict fails").toBe(false);
  });
  it("runs without the second tab only when told: its three checks NOT RUN, none failed, and no claim that every check held", () => {
    const r = { ...GOOD_RELOAD, secondTab: false, tab: null, tabBefore: null };
    expect(failed(r)).toEqual([]);
    expect(notRun(r)).toEqual(TAB_CHECKS);
    const v = verdictOf(afterReloadChecks(r));
    expect(v).toEqual({
      ok: true,
      complete: false,
      summary: "11 checks: 8 held, 0 failed, 3 not run (no second tab, by --no-second-tab)",
    });
    expect(v.summary).not.toContain("every check held");
  });
});

describe("verdictOf — the run's one sentence", () => {
  const held = (n: string): Check => ({ check: n, ok: true, saw: null });
  it("says every check held only when every check ran and held", () => {
    expect(verdictOf(afterReloadChecks(GOOD_RELOAD))).toEqual({ ok: true, complete: true, summary: "11 checks, every check held" });
  });
  it("fails on any failed check, naming the counts", () => {
    const v = verdictOf([held("a"), { check: "b", ok: false, saw: 1 }, { check: "c", ok: null, saw: "why" }]);
    expect(v).toEqual({ ok: false, complete: false, summary: "3 checks: 1 held, 1 failed, 1 not run (why)" });
  });
  it("fails a run with no check at all", () => {
    expect(verdictOf([])).toEqual({ ok: false, complete: false, summary: "no check ran" });
  });
});

const GOOD_CLICK: AfterClick = {
  before: { url: "https://h.example/#/t/local-1", events: 9 },
  tab: { lines: [], loads: 1, url: "https://h.example/#/t/local-1", events: 9 },
  release: "yokohama-release-2222222222222222",
  eventsAfterAMove: 12,
};

// **The run ends with a verdict, never a browser left open** (cycle F's fix
// wave; Task 8's review, Minors 1 and 2 — before the window). Two decisions
// the browser half takes, read here over a scratch state directory and a fake
// door text: `PINNED` — the orchestrator's word to deploy — only over a door
// that read "Ready to play offline", and the wait for `go` bounded, its
// timeout a failed check `verdict.json` names. Neither launches anything.
describe("the pin and the wait for go", () => {
  const made: string[] = [];
  afterEach(() => {
    for (const dir of made.splice(0)) rmSync(dir, { recursive: true, force: true });
  });
  const stateDir = (): string => {
    const dir = mkdtempSync(join(tmpdir(), "pinned-cell-"));
    made.push(dir);
    return dir;
  };

  // Mutant: PINNED written whatever the door read.
  it.each<[string, string[]]>([
    ["still installing", [SENTENCES.installing]],
    ["failed", ["Could not prepare to play offline — the install did not finish. Try again"]],
    ["drawing no line", []],
    ["drawing a line it does not know", ["Ready to play offline."]],
  ])("writes no PINNED over a door %s, and answers the failed check naming what it read", (_what, lines) => {
    const dir = stateDir();
    expect(pin(dir, lines)).toEqual({ check: PIN_CHECK, ok: false, saw: lines });
    expect(existsSync(join(dir, "PINNED")), "no PINNED").toBe(false);
  });
  it("writes PINNED over a door that reads Ready to play offline, and answers no check", () => {
    const dir = stateDir();
    expect(pin(dir, [SENTENCES.ready], () => new Date("2026-10-01T12:00:00.000Z"))).toBeNull();
    expect(readFileSync(join(dir, "PINNED"), "utf8")).toBe("2026-10-01T12:00:00.000Z\n");
  });

  it("waits three hours for go by default, and its check names the bound in minutes", () => {
    expect(GO_WAIT_DEFAULT_MINUTES).toBe(180);
    expect(goCheck(180)).toBe("go was touched within 180 minutes of PINNED");
    expect(goCheck(45)).toBe("go was touched within 45 minutes of PINNED");
  });
  // A minute made 20 ms long, so three "minutes" are 60 ms. Mutant: the wait
  // unbounded — this cell then never ends, and vitest's own timeout fails it.
  it("gives up on a go that never comes at its bound, answering the failed check that names the bound", async () => {
    const dir = stateDir();
    const late = await awaitGo(dir, 3, { minuteMs: 20, pollMs: 10 });
    expect(late).toEqual({
      check: "go was touched within 3 minutes of PINNED",
      ok: false,
      saw: `no go in ${dir} within 3 minutes: the deploy's reading never began`,
    });
  }, 2_000);
  it("answers no check for a go that is there", async () => {
    const dir = stateDir();
    writeFileSync(join(dir, "go"), "");
    expect(await awaitGo(dir, 3, { minuteMs: 20, pollMs: 10 })).toBeNull();
  });
  // What the run then does with either: `verdict.json` names it and the bound
  // it used, and the verdict fails — which is the instrument's exit 1.
  it("writes verdict.json naming the timeout and the bound, failed", async () => {
    const dir = stateDir();
    const late = await awaitGo(dir, 3, { minuteMs: 20, pollMs: 10 });
    const verdict = writeVerdict(dir, { label: "worker", origin: "https://h.example", secondTab: true, goWaitMinutes: 3 }, [late!]);
    expect(verdict.ok, "the instrument exits 1").toBe(false);
    expect(JSON.parse(readFileSync(join(dir, "verdict.json"), "utf8"))).toEqual({
      label: "worker",
      origin: "https://h.example",
      secondTab: true,
      goWaitMinutes: 3,
      ok: false,
      complete: true,
      summary: "1 checks: 0 held, 1 failed, 0 not run",
      checks: [late],
    });
  }, 2_000);
});

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
