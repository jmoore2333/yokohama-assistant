// The browser walk's shared figures, in ONE file (M6.5c2 cycle D, dossier row
// 56). `CAP` was four copies — one per walk — and `REV` two literals; a copy is
// a figure that can be changed in one spec and left behind in the next.
//
// Node code, read at import by the specs that walk a game; nothing here runs in
// the page.

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** A walk's own cap on moves: well above what a game takes (each walk prints
 *  its own figure), and low enough that a loop fails as a loop rather than as
 *  a timeout. A hit throws `capReached`, which names the cap, the turn and the
 *  end-trigger strip — never `deadEnd`'s stuck-table sentence (M6.5c2). */
export const CAP = 4000;

/** The committed health reply — the server's own statement of the semantics
 *  revision this build plays, frozen by `wire_fixtures` from the running
 *  layer. Read from the FILE rather than restated, so a revision bump that
 *  regenerates the corpus moves this with it. */
const HEALTH = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "src", "wire", "health.json");

const health = JSON.parse(fs.readFileSync(HEALTH, "utf8")) as { semantics_rev?: unknown };
if (typeof health.semantics_rev !== "number") {
  throw new Error(`${HEALTH} carries no numeric semantics_rev — the walk cannot name the archive's revision`);
}

/** This build's semantics revision as the archive names its directory level —
 *  `rev7` today. */
export const REV = `rev${health.semantics_rev}`;
