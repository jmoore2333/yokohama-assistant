// **The Access application's destinations** (R-6.5c2.39, the door window's
// state 2): every `/api/` prefix the deployment routes, plus `/enter`, one per
// line, host-prefixed and sorted — what the owner types into the application's
// destination list, derived from the routers rather than remembered. Every
// prefix needs an identity or is read after sign-in. **`health` among them
// since cycle F** (spec §2.5): it stood outside the application as the uptime
// probe and the client's first read, but the lobby reads it AFTER `/enter`,
// and under a state-2 door an unprotected path is one a stranger's script can
// meter. The deploy's smoke and any uptime probe carry the service credential,
// or watch `/` — a static file, which invokes nothing.
//
// The four files it reads are the four places a prefix is named: the Worker's
// `route()` (`games`, `archive`, `art`, `admin`), the operator surface, the
// house (`stream`, and the layer's routes it dispatches) and the layer's own
// `routes()` table (`health`, `me`, `labels`, `chat`, `games`, `archive`,
// `art`). The layer's file is read from its `routes()` table only: its test
// module names paths (`/api/nope`) that are not routes.
// `access-paths.test.mjs` holds the printed list to the ten destinations
// written out by hand, so a prefix added to any file fails there until that
// list names it; a second cell holds the assembly (every segment kept,
// `/enter` in, sorted, host-prefixed) over what the regex finds.

import { readFileSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";

/** The four files, from the repository root. */
export const ROUTER_FILES = [
  "worker/src/route.ts",
  "worker/src/admin.ts",
  "worker/src/house.ts",
  "crates/yokohama-web/src/api.rs",
];

const HOST = "yokohama.moore.nyc";

/** The `routes()` function of the layer's `api.rs`: from its signature to the
 *  first closing brace in column 0. Refuses a file with no such function, so
 *  a rename cannot quietly leave the list empty of the layer's prefixes. */
export function layerRoutesText(text) {
  const start = text.indexOf("pub fn routes(");
  if (start < 0) throw new Error("access-paths: api.rs has no `pub fn routes(` — the layer's route table moved; read it again");
  const end = text.indexOf("\n}\n", start);
  return text.slice(start, end < 0 ? undefined : end);
}

/** The `/api/<segment>` prefixes `text` names, as a Set. */
export function apiSegments(text) {
  return new Set([...text.matchAll(/\/api\/([a-z][a-z-]*)/g)].map((m) => m[1]));
}

/** The destinations for the repository at `repo`. */
export function accessPaths(repo) {
  const segments = new Set();
  for (const file of ROUTER_FILES) {
    const text = readFileSync(`${repo}/${file}`, "utf8");
    for (const s of apiSegments(file.endsWith("api.rs") ? layerRoutesText(text) : text)) segments.add(s);
  }
  return [...[...segments].sort().map((s) => `${HOST}/api/${s}`), `${HOST}/enter`];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const repo = fileURLToPath(new URL("../..", import.meta.url));
  process.stdout.write(`${accessPaths(repo).join("\n")}\n`);
}
