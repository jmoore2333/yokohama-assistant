// **The live replay's rows, and where they go** (M6.5c2 cycle D, Task 9): one
// of the five parts of `live-replay.mjs`, which no script but it and the other
// parts imports. The three request scripts and the committed corpus, read from
// the repository; the order the rows run in (`order`, `../corpus-walk.mjs`'s,
// which the pool's replay runs too); the one address each is sent to; and the
// tables the walk creates.

import { readFileSync, readdirSync } from "node:fs";
import { ORIGIN } from "./credential.mjs";

export { order } from "../corpus-walk.mjs";

const REPO = new URL("../../../", import.meta.url);

/** The three request scripts, from `web/src/wire-script/`. */
export function loadScripts() {
  const read = (name) => JSON.parse(readFileSync(new URL(`web/src/wire-script/${name}`, REPO), "utf8"));
  return { walk: read("walk.json"), host: read("walk-host.json"), archive: read("archive-walk.json") };
}

/** The committed corpus, `web/src/wire/*.json`, by file name. */
export function loadCorpus() {
  const dir = new URL("web/src/wire/", REPO);
  return Object.fromEntries(
    readdirSync(dir)
      .filter((f) => f.endsWith(".json"))
      .map((f) => [f.slice(0, -".json".length), readFileSync(new URL(f, dir), "utf8")]),
  );
}

/** Where `row` is sent: the Worker's origin, and nowhere else. */
export function urlOf(row) {
  const url = new URL(`${row.path}${row.query ? `?${row.query}` : ""}`, ORIGIN);
  if (url.origin !== ORIGIN) throw new Error(`the row ${row.method} ${row.path} leaves ${ORIGIN}: this script sends nothing anywhere else`);
  return url.href;
}

/** The tables the walk creates, by the ids its create bodies name. */
export function tableIds(walk) {
  return walk.filter((row) => row.method === "POST" && row.path === "/api/games").map((row) => row.body.id);
}
