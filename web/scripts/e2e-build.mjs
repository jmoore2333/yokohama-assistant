// `npm run e2e`'s client build with the door's two text build values set
// (R-6.5c2.40), so every e2e run draws the blurb and the footer's notice and
// the layout cells (`noSidewaysScroll`, the 44px reads, the title-never-meets-
// the-board cell) see them. An explicit value in the environment wins; an
// empty one counts as unset, as sh's `${VAR:-default}` reads it.
//
// A script and not the `e2e` line itself, because the blurb holds an
// apostrophe and macOS's `sh` (bash in POSIX mode) fails to parse one inside
// `"${VAR:-...}"` (measured, 2026-09-29).

import { spawnSync } from "node:child_process";

const DEFAULTS = {
  YOKOHAMA_SITE_BLURB:
    "Yokohama, the board game, in your browser. Play the bot here, or play with friends. An independent open-source project, unaffiliated with the game's designer and publisher, working toward a better computer player for Yokohama.",
  YOKOHAMA_SITE_NOTICE: "Not affiliated with Asmodee",
};

const env = { ...process.env };
for (const [name, value] of Object.entries(DEFAULTS)) {
  if (!env[name]) env[name] = value;
}
const run = spawnSync("npm", ["run", "build:client"], { env, stdio: "inherit" });
process.exit(run.status ?? 1);
