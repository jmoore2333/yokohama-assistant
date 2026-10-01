// **The e2e walk's two scratch directories, emptied ONCE per run, by the server's
// launch** (cycle F, E's m-4).
//
// `playwright.config.ts`'s `webServer.command` is `node web/e2e/fresh_dirs.mjs &&
// cargo run --release -p yokohama-web`, so this runs in the process that starts
// the server, before the server reads either directory, and in no other. The
// config used to empty them at every load, which `--list`, each worker and a
// node cell that reads the config all do: a walk running beside any of them lost
// its games. The config now only COMPUTES the two paths, through `scratchDirs`
// below, and the cells that need them (`fresh_dirs.node.test.ts`) call the same
// function.
//
// It reads the two paths from the environment the config hands the server
// (`YOKOHAMA_WEB_DIR`, `YOKOHAMA_WEB_ARCHIVE_DIR`) and empties exactly those. It
// deletes nothing it was not handed, and REFUSES — exit 2, naming the variable
// and the path — unless each is, once resolved, a directory directly inside the
// system temp directory whose name starts `yokohama-e2e-`.
//
// **It deletes the path it CHECKED, and only that** (fix round 1; the review of
// `11c8b60c`, Important 1). The first version checked the value collapsed as
// TEXT and deleted the RAW value, and the two disagree whenever a symlink comes
// before a `..`: `<temp>/<link>/../yokohama-e2e-x` reads as a child of the temp
// directory as text, while the operating system walks through the link and the
// `..` lands in the link target's parent. So: a value with a `..` segment is
// refused outright, before anything is resolved; every other value is resolved
// with `realpathSync` (every symlink followed, the last one too; a value not
// there yet through its parent), the check reads THAT path, and `rmSync` and
// `mkdirSync` take THAT path. Both values are checked before either directory
// is emptied, so a refusal empties nothing.

import { lstatSync, mkdirSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const GAMES_ENV = "YOKOHAMA_WEB_DIR";
const ARCHIVE_ENV = "YOKOHAMA_WEB_ARCHIVE_DIR";
/** Every directory this script will touch starts with this. */
const PREFIX = "yokohama-e2e-";

/** The two directories a walk on `port` uses, under `root` (the system temp
 *  directory by default): the ONE definition of their names. */
export function scratchDirs(port, root = tmpdir()) {
  return { games: join(root, `${PREFIX}${port}`), archive: join(root, `${PREFIX}archive-${port}`) };
}

/** The real path `value` names and may be emptied, as `{ real }`, or `{ why }`
 *  it may not be. */
function check(value) {
  if (value === undefined || value === "") return { why: "it is not set" };
  // Refused before anything is resolved: a `..` means one thing as text and
  // another once a symlink comes before it, and nothing here needs one.
  if (value.split(/[\\/]/).includes("..")) return { why: 'it contains a ".." segment' };
  const abs = resolve(value); // no `..` is left for this to collapse
  let real;
  try {
    real = realpathSync(abs);
  } catch (e) {
    if (e.code !== "ENOENT") return { why: `it cannot be resolved (${e.code})` };
    if (isLink(abs)) return { why: "it is a symlink that points nowhere" };
    // Not there yet: resolved through its parent, which has to be.
    let parent;
    try {
      parent = realpathSync(dirname(abs));
    } catch {
      return { why: "its parent directory does not exist, so it is outside the temp directory" };
    }
    real = join(parent, basename(abs));
  }
  const root = realpathSync(tmpdir());
  const rel = relative(root, real);
  if (rel.startsWith("..") || isAbsolute(rel)) return { why: `it resolves outside the temp directory (${root}): ${real}` };
  if (!basename(real).startsWith(PREFIX)) return { why: `its last segment must start with "${PREFIX}": ${real}` };
  if (dirname(real) !== root) return { why: `it must be directly inside the temp directory (${root}): ${real}` };
  return { real };
}

/** Whether `path` is itself a symlink, followed or not. */
function isLink(path) {
  try {
    return lstatSync(path).isSymbolicLink();
  } catch {
    return false;
  }
}

/** Empties and recreates the two directories `env` names; the exit code. */
function main(env) {
  const checked = [GAMES_ENV, ARCHIVE_ENV].map((name) => ({ name, value: env[name], ...check(env[name]) }));
  const refused = checked.filter((c) => c.why !== undefined).map((c) => `fresh_dirs: refusing ${c.name}=${c.value ?? "(unset)"}: ${c.why}`);
  if (refused.length === 0 && checked[0].real === checked[1].real) {
    refused.push(`fresh_dirs: refusing ${GAMES_ENV} and ${ARCHIVE_ENV}: they name the same directory (${checked[0].value}, ${checked[1].value})`);
  }
  if (refused.length > 0) {
    for (const line of refused) console.error(line);
    return 2;
  }
  // The CHECKED real paths, never the raw values.
  for (const { real } of checked) {
    rmSync(real, { recursive: true, force: true });
    mkdirSync(real, { recursive: true });
  }
  return 0;
}

/** Whether `argv1` — the path node was started with — is this module,
 *  compared by REAL path (cycle F's fix wave; the final review, which tried
 *  it): node resolves the main module's symlinks before it sets
 *  `import.meta.url`, so a script started through a symlinked path, compared
 *  as given, never ran its `main` and exited 0 having emptied nothing — and
 *  the launch's `&& cargo run` went on over stale directories. */
function isMain(argv1) {
  if (argv1 === undefined) return false;
  try {
    return realpathSync(argv1) === realpathSync(fileURLToPath(import.meta.url));
  } catch {
    return false;
  }
}

if (isMain(process.argv[1])) {
  process.exit(main(process.env));
}
