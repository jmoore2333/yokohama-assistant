// **The live Worker's credential, and where it may go** (M6.5c2 cycle D,
// Task 9; the M6.5c2 spec §2.8's "`.dev.vars` is parsed in one module"): the
// repository root's `.dev.vars`, which holds the Service Auth credential; its
// two keys; the ONE reader; and the ONE origin the credential is ever
// presented to. Two npm packages import it by relative path — the live replay
// (`worker/scripts/live-replay.mjs`) and the live tier (`web/e2e/live.ts`) —
// and `dev-vars.d.mts` beside it is what TypeScript reads for the second: an
// `.mjs` import reads no `.d.ts` (TS7016). It imports nothing but node's own
// modules, so neither package installs anything for it.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** The deployed Worker's origin: the only one the credential is presented
 *  to — every request the live replay sends, and the live tier's `BASE_URL`
 *  — and the host `wrangler.jsonc`'s route binds (`dev-vars.test.mjs`). */
export const ORIGIN = "https://yokohama.moore.nyc";

/** The repository root's `.dev.vars`, which holds the credential (the plan's
 *  Global Constraints: two `KEY=VALUE` lines, named as the headers). */
export const DEV_VARS = fileURLToPath(new URL("../../.dev.vars", import.meta.url));

/** The credential's two keys, named as the headers Access expects. */
export const KEYS = ["CF-Access-Client-Id", "CF-Access-Client-Secret"];

/**
 * The credential in `file`: its `CF-Access-Client-Id=` and
 * `CF-Access-Client-Secret=` lines, among any others. A file or a key that is
 * absent — or a value that is empty — is refused by NAME: the file and the
 * key, never a value.
 */
export function readCredential(file = DEV_VARS) {
  let text;
  try {
    text = readFileSync(file, "utf8");
  } catch {
    throw new Error(`the credential file ${file} is absent or unreadable — it holds the ${KEYS[0]}= and ${KEYS[1]}= lines (worker/RUNBOOK.md §4)`);
  }
  const values = new Map();
  for (const line of text.split(/\r?\n/)) {
    const eq = line.indexOf("=");
    if (eq !== -1) values.set(line.slice(0, eq).trim(), line.slice(eq + 1).trim());
  }
  const read = (key) => {
    const value = values.get(key);
    if (!value) throw new Error(`${file} holds no ${key}= line — the credential is its ${KEYS[0]}= and ${KEYS[1]}= lines (worker/RUNBOOK.md §4)`);
    return value;
  };
  return { id: read(KEYS[0]), secret: read(KEYS[1]) };
}
