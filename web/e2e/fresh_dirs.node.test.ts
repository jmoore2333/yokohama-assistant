// **The e2e walk's two scratch directories are emptied by the server's LAUNCH,
// and by nothing else** (cycle F, E's m-4; the review of `92b894a4`, m-4).
//
// `playwright.config.ts` is loaded by every Playwright process (the runner and
// each worker), by `--list`, and by a node cell that reads it. It used to EMPTY
// the games directory and the archive directory at each of those loads, so
// running `--list` or `npm run test:wasm` beside a local walk took that walk's
// games out from under it. `e2e/fresh_dirs.mjs` is now the one place that
// empties them, run by the `webServer` command once per run, in the process that
// starts the server — and a config load makes and deletes nothing.
//
// Node and not Playwright, like the other cells here (`npm run test:wasm`). The
// config is read through Playwright itself (`--list`, as the opt-in specs'
// collection cells do), with `TMPDIR` pointed at a scratch directory of the
// cell's own, so the cell never touches a directory a real walk is using. The
// names come from the ONE definition, `scratchDirs`, which the config calls too:
// a cell that planted files under names of its own would stay green after the
// config's names moved.
//
// **Where the first cell is vacuous, stated:** with no Chromium installed the
// config defines no `webServer` and so never computes the directories, which
// is how CI's `web-server` job runs this file. The source cell below reads the
// config's launch command and its imports, and does not depend on a browser.

import { spawnSync } from "node:child_process";
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { scratchDirs } from "./fresh_dirs.mjs";
import { LOCAL_PORT } from "./live";

const here = dirname(fileURLToPath(import.meta.url));
const SCRIPT = join(here, "fresh_dirs.mjs");

let scratch: string;
beforeEach(() => {
  scratch = mkdtempSync(join(tmpdir(), "fresh-dirs-cell-"));
});
afterEach(() => rmSync(scratch, { recursive: true, force: true }));

/** A directory holding a planted file and a nested one, so "emptied" means all
 *  of it. */
function plant(dir: string): string {
  mkdirSync(join(dir, "nested"), { recursive: true });
  const file = join(dir, "planted.txt");
  writeFileSync(file, "x");
  writeFileSync(join(dir, "nested", "deeper.txt"), "y");
  return file;
}

/** Runs the script with `env` over `scratch` as its temp directory. */
function runScript(env: Record<string, string>) {
  return spawnSync(process.execPath, [SCRIPT], {
    env: { ...process.env, TMPDIR: scratch, YOKOHAMA_WEB_DIR: "", YOKOHAMA_WEB_ARCHIVE_DIR: "", ...env },
    encoding: "utf8",
  });
}

describe("loading the config empties nothing", () => {
  // Mutant: `gamesDir()` / `archiveDir()` calling an emptying helper again — the
  // shape this cell was written red against.
  it("a planted file in each directory survives Playwright loading the config", () => {
    const { games, archive } = scratchDirs(LOCAL_PORT, scratch);
    const planted = [plant(games), plant(archive)];
    const out = spawnSync("npx", ["playwright", "test", "--list"], {
      cwd: join(here, ".."),
      env: {
        ...process.env,
        TMPDIR: scratch,
        YOKOHAMA_WEB_E2E_OPTIONAL: "1",
        YOKOHAMA_WEB_E2E_TARGET: "",
        YOKOHAMA_WEB_STATE1_GAME: "",
        YOKOHAMA_WEB_README_SHOTS: "",
        YOKOHAMA_WEB_EXIT: "",
      },
      encoding: "utf8",
    });
    expect(out.status, `playwright --list: ${out.stderr}`).toBe(0);
    for (const file of planted) expect(existsSync(file), `${file} survived the config load`).toBe(true);
    expect(existsSync(join(games, "nested", "deeper.txt")), "and so did its nested file").toBe(true);
  }, 120_000);

  it("the config's launch command empties them first and nothing in the config does", () => {
    const config = readFileSync(join(here, "..", "playwright.config.ts"), "utf8");
    expect(config, "the webServer's command runs the script before the server").toContain('command: "node web/e2e/fresh_dirs.mjs && cargo run --release -p yokohama-web"');
    expect(config, "the config names the two directories through scratchDirs").toContain("scratchDirs(");
    expect(config, "and deletes nothing itself").not.toMatch(/\brmSync\b/);
  });
});

describe("the script", () => {
  it("empties and recreates exactly the two directories the config names", () => {
    const { games, archive } = scratchDirs(LOCAL_PORT, scratch);
    plant(games);
    plant(archive);
    const sibling = join(scratch, "yokohama-e2e-other");
    const kept = plant(sibling);
    const ran = runScript({ YOKOHAMA_WEB_DIR: games, YOKOHAMA_WEB_ARCHIVE_DIR: archive });
    expect(ran.status, ran.stderr).toBe(0);
    for (const dir of [games, archive]) {
      expect(existsSync(dir), `${dir} exists again`).toBe(true);
      expect(readdirSync(dir), `${dir} is empty`).toEqual([]);
    }
    expect(existsSync(kept), "a neighbour in the same temp directory is untouched").toBe(true);
  });

  it("makes a directory that is not there yet", () => {
    const { games, archive } = scratchDirs(LOCAL_PORT, scratch);
    const ran = runScript({ YOKOHAMA_WEB_DIR: games, YOKOHAMA_WEB_ARCHIVE_DIR: archive });
    expect(ran.status, ran.stderr).toBe(0);
    expect(readdirSync(games)).toEqual([]);
    expect(readdirSync(archive)).toEqual([]);
  });

  // The guard checks the path it DELETES: a symlink is followed before the
  // check, and it is the checked real path that is emptied (fix round 1).
  it("follows a symlink inside the temp directory to a yokohama-e2e-* directory also inside it, and empties only that one", () => {
    const { games, archive } = scratchDirs(LOCAL_PORT, scratch);
    const target = join(scratch, "yokohama-e2e-target");
    plant(target);
    symlinkSync(target, games);
    plant(archive);
    const neighbour = plant(join(scratch, "yokohama-e2e-other"));
    const ran = runScript({ YOKOHAMA_WEB_DIR: games, YOKOHAMA_WEB_ARCHIVE_DIR: archive });
    expect(ran.status, ran.stderr).toBe(0);
    expect(readdirSync(target), "the link's target is the directory that was emptied").toEqual([]);
    expect(lstatSync(games).isSymbolicLink(), "the link itself is still a link").toBe(true);
    expect(readdirSync(archive)).toEqual([]);
    expect(existsSync(neighbour), "and a neighbour is untouched").toBe(true);
  });

  // Each refusal exits 2 NAMING the path, and — the two paths are checked
  // before either is emptied — leaves the good one's file in place, and the
  // refused directory's own.
  describe("refuses, naming the path and emptying neither", () => {
    type Refusal = { value: string; why: RegExp; guarded: string[] };
    /** Made outside `scratch` (in the real temp directory), so removed by hand. */
    const made: string[] = [];
    afterEach(() => {
      for (const dir of made.splice(0)) rmSync(dir, { recursive: true, force: true });
    });
    const outside = (): string => {
      const dir = mkdtempSync(join(tmpdir(), "fresh-dirs-outside-"));
      made.push(dir);
      return dir;
    };
    const cases: [string, (good: string) => Refusal][] = [
      [
        "a directory outside the temp directory",
        () => {
          const dir = join(outside(), "yokohama-e2e-x");
          return { value: dir, why: /outside the temp directory/, guarded: [plant(dir)] };
        },
      ],
      [
        // **The shape the first version let through** (the review of 11c8b60c,
        // Important 1): a symlink BEFORE a `..`. Collapsed as text it reads
        // `<temp>/yokohama-e2e-x`, but the operating system walks through the
        // link, so the `..` lands in the link target's PARENT, outside the temp
        // directory. Built as a STRING: `path.join` would collapse the `..`
        // before the script ever saw it, which is what the cell that stood here
        // did, so it never sent a `..` at all.
        "a `..` after a symlink, which the operating system resolves outside the temp directory",
        () => {
          const parent = outside();
          const target = join(parent, "inner");
          mkdirSync(target);
          const sentinel = join(target, "sentinel.txt");
          writeFileSync(sentinel, "z");
          symlinkSync(target, join(scratch, "yokohama-e2e-link"));
          const victim = plant(join(parent, "yokohama-e2e-x"));
          return { value: `${scratch}/yokohama-e2e-link/../yokohama-e2e-x`, why: /"\.\." segment/, guarded: [victim, sentinel] };
        },
      ],
      [
        // Belt and braces: any `..` is refused, even one that is harmless.
        "a `..` that is harmless, refused anyway",
        () => {
          const dir = join(scratch, "yokohama-e2e-x");
          return { value: `${scratch}/sub/../yokohama-e2e-x`, why: /"\.\." segment/, guarded: [plant(dir)] };
        },
      ],
      [
        "a symlink that points out of it",
        () => {
          const away = outside();
          const link = join(scratch, "yokohama-e2e-link");
          symlinkSync(away, link);
          return { value: link, why: /outside the temp directory/, guarded: [plant(away)] };
        },
      ],
      [
        // The check reads what is DELETED, which is the link's target.
        "a symlink named like the walk's that points at a directory that is not",
        () => {
          const precious = join(scratch, "precious");
          const file = plant(precious);
          symlinkSync(precious, join(scratch, "yokohama-e2e-link2"));
          return { value: join(scratch, "yokohama-e2e-link2"), why: /last segment/, guarded: [file] };
        },
      ],
      [
        "a name that is not the walk's",
        () => {
          const dir = join(scratch, "games");
          return { value: dir, why: /last segment/, guarded: [plant(dir)] };
        },
      ],
      [
        "the temp directory itself",
        () => ({ value: scratch, why: /last segment/, guarded: [plant(scratch)] }),
      ],
      [
        "a directory nested below the temp directory",
        () => {
          const dir = join(scratch, "a", "yokohama-e2e-x");
          return { value: dir, why: /directly inside/, guarded: [plant(dir)] };
        },
      ],
      ["the same directory twice", (good) => ({ value: good, why: /same directory/, guarded: [] })],
      ["an unset name", () => ({ value: "", why: /not set/, guarded: [] })],
    ];
    for (const [name, build] of cases) {
      it(name, () => {
        const { games } = scratchDirs(LOCAL_PORT, scratch);
        const goodFile = plant(games);
        const { value, why, guarded } = build(games);
        const ran = runScript({ YOKOHAMA_WEB_DIR: games, YOKOHAMA_WEB_ARCHIVE_DIR: value });
        expect(ran.status, "exits 2").toBe(2);
        expect(ran.stderr, "names the path").toContain(value === "" ? "YOKOHAMA_WEB_ARCHIVE_DIR" : value);
        expect(ran.stderr, "and the reason").toMatch(why);
        expect(existsSync(goodFile), "the good directory was not emptied either").toBe(true);
        for (const file of guarded) expect(existsSync(file), `${file} (the refused one's) is still there`).toBe(true);
      });
    }
  });
});
