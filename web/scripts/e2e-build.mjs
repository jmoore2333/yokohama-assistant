// `npm run e2e`'s client build with the door's two text build values set
// (R-6.5c2.40), so every e2e run draws the blurb and the footer's notice and
// the layout cells (`noSidewaysScroll`, the 44px reads, the title-never-meets-
// the-board cell) see them. An explicit value in the environment wins; an
// empty one counts as unset, as sh's `${VAR:-default}` reads it.
//
// **And the release feed OFF** (cycle F, R-F2): unset, the feed derives from a
// GitHub project link, so a developer's shell that exports
// `YOKOHAMA_PROJECT_URL` would send every e2e page to api.github.com — an
// anonymous request a run must not spend, and one whose answer (or refusal)
// would put a line on screens the cells measure. A cell or a picture that wants
// a feed sets `YOKOHAMA_RELEASE_FEED` in the environment and routes it. No site
// key is set: the plain door is the one these cells walk.
//
// A script and not the `e2e` line itself, because the blurb holds an
// apostrophe and macOS's `sh` (bash in POSIX mode) fails to parse one inside
// `"${VAR:-...}"` (measured, 2026-09-29).
//
// **And the policy ENFORCED** (security S2a, decision D-9) — since 0.6.3 an
// IDENTITY on the shipped set: the shipped `CSP_MODE` is `enforce` (security
// S2c, R-6), so the build's own `dist/headers.json` already carries the policy
// under `Content-Security-Policy` and the rewrite below changes nothing. It is
// kept so a report-only build — a rollback to 0.6.2's shape — still walks
// ENFORCED locally: enforced, a violation in the page is a console `error` the
// walk fails on, and a violation inside the module Worker — which prints on no
// console in either mode — is a refusal the walk meets as a game that never
// comes; so a green walk is the local enforcement reading. Only the e2e build
// is rewritten: the shipped `CSP_MODE` stays the constant in `headers.mjs`,
// with its cell. Pinned by `e2e/framed.spec.ts`'s first cell, which reads the
// policy under the enforcing name; the identity itself is argued by the build,
// since the file is enforced before the rewrite runs, and no cell compares the
// file byte for byte (`framed.spec.ts` sees only the served header).

import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { CSP_HEADER, headersJson, parseHeadersJson, readHeadersText } from "./headers.mjs";

const DEFAULTS = {
  YOKOHAMA_SITE_BLURB:
    "Yokohama, the board game, in your browser. Play the bot here, or play with friends. An independent open-source project, unaffiliated with the game's designer and publisher, working toward a better computer player for Yokohama.",
  YOKOHAMA_SITE_NOTICE: "Not affiliated with Asmodee",
  YOKOHAMA_RELEASE_FEED: "off",
};

const env = { ...process.env };
for (const [name, value] of Object.entries(DEFAULTS)) {
  if (!env[name]) env[name] = value;
}
const run = spawnSync("npm", ["run", "build:client"], { env, stdio: "inherit" });
if (run.error) console.error(`e2e-build: could not run npm: ${run.error.message}`);
// **The e2e build serves the policy ENFORCED** (security S2a, decision D-9) — since 0.6.3 an IDENTITY on the
// shipped set, whose `CSP_MODE` is `enforce` (security S2c, R-6): no pair carries the Report-Only name, so every
// pair comes back as it went in. Kept so a report-only build (a rollback to 0.6.2's shape) still walks enforced: a
// Report-Only violation is a LINE and not a block — Chromium logs it at console type `info`, which the harness
// reads but which stops nothing — and a violation inside the module Worker prints nowhere in either mode, so only
// an ENFORCED walk, where an INLINE or SOURCE refusal is an `error` the harness fails on (an enforced `eval`
// refusal prints no console line, only an EvalError — the built client has neither `eval(` nor `new Function(`)
// and a Worker-internal one fails the walk itself, is the enforcement reading: the local walk is served
// `Content-Security-Policy`. An override that can only strengthen: the shipped `CSP_MODE` stays the constant, with
// its cell, and this rewrites the e2e build's `dist/headers.json` alone. (A post-e2e `dist` is the e2e build's —
// its debug module and its build values: the Worker's assembly already refuses a post-e2e dist for its debug
// module, and the NAS recipe rebuilds.)
if (run.status === 0) {
  const file = new URL("../dist/headers.json", import.meta.url);
  const set = parseHeadersJson(readHeadersText(file)).map(([name, value]) =>
    name === CSP_HEADER["report-only"] ? [CSP_HEADER.enforce, value] : [name, value],
  );
  writeFileSync(file, headersJson(set));
}
process.exit(run.status ?? 1);
