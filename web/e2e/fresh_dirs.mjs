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
// and the path — unless each is a directory directly inside the system temp
// directory whose name starts `yokohama-e2e-`, resolved through symlinks. Both
// are checked before either is emptied, so a refusal empties nothing.

import { existsSync, mkdirSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, relative, resolve } from "node:path";
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

/** Why `value` may not be emptied, or `null` when it may. */
function refusal(value) {
  if (value === undefined || value === "") return "it is not set";
  const abs = resolve(value);
  if (!basename(abs).startsWith(PREFIX)) return `its last segment must start with "${PREFIX}"`;
  const root = realpathSync(tmpdir());
  const parent = dirname(abs);
  if (!existsSync(parent)) return "its parent directory does not exist, so it is outside the temp directory";
  // Resolved through symlinks: a link named like a walk's directory that points
  // anywhere else resolves to the somewhere else, and is refused for it.
  const real = existsSync(abs) ? realpathSync(abs) : join(realpathSync(parent), basename(abs));
  const rel = relative(root, real);
  if (rel === "" || rel.startsWith("..") || resolve(root, rel) !== real) return `it resolves outside the temp directory (${root})`;
  if (dirname(real) !== root) return `it must be directly inside the temp directory (${root})`;
  return null;
}

/** Empties and recreates the two directories `env` names; the exit code. */
function main(env) {
  const named = [
    [GAMES_ENV, env[GAMES_ENV]],
    [ARCHIVE_ENV, env[ARCHIVE_ENV]],
  ];
  const refused = [];
  for (const [name, value] of named) {
    const why = refusal(value);
    if (why !== null) refused.push(`fresh_dirs: refusing ${name}=${value ?? "(unset)"}: ${why}`);
  }
  if (refused.length === 0 && resolve(named[0][1]) === resolve(named[1][1])) {
    refused.push(`fresh_dirs: refusing ${GAMES_ENV} and ${ARCHIVE_ENV}: they name the same directory (${named[0][1]})`);
  }
  if (refused.length > 0) {
    for (const line of refused) console.error(line);
    return 2;
  }
  for (const [, value] of named) {
    rmSync(value, { recursive: true, force: true });
    mkdirSync(value, { recursive: true });
  }
  return 0;
}

if (process.argv[1] !== undefined && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exit(main(process.env));
}
