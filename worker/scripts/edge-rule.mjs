// **The no-Access door's edge rule** (M6.5c2 cycle F, R-F6, R-FP7; spec §2.5,
// §2.6): ONE WAF custom rule in the `moore.nyc` zone's
// `http_request_firewall_custom` phase — Managed Challenge on the Worker's
// hostname for every request whose path is not one of the release's own
// static names. It runs at the edge, before Bot Fight Mode and the Access
// check, at zero Worker invocations, so `/api/*`, `/enter`, `/.env`,
// `/wp-login.php` and a random path are all decided there. A person passes it
// once through the Turnstile widget's pre-clearance (`web/src/turnstile.ts`).
// It is the no-Access deployment's FENCE: enabled while the hostname is off
// Access, DISABLED under Access (the live tiers' service token cannot pass a
// challenge, and Access already stops every stranger). The operator creates
// it through the API from this script's JSON and keeps it by its id.
//
// **The allow-list is DERIVED from the assets tree's top level** — the tree
// `npm run build:assets` assembles and `wrangler deploy` uploads
// (`worker/assets/`): every top-level FILE as `"/<name>"`, `/` as well when
// `index.html` is there (`worker/static/_redirects` serves the root as it),
// every top-level DIRECTORY as the prefix `"/<name>/"`, and Cloudflare's own
// `/cdn-cgi/` ALWAYS. `_headers` and `_redirects` are the asset layer's
// configuration, never served, and never listed. So a release that adds a
// top-level name changes the expression, and the drift cell in
// `edge-rule.test.mjs`, which writes the names out by hand, fails until the
// cell and the zone's rule move together.
//
// **The order is fixed, since `--verify` compares the expression as a
// string**: the files sorted by code unit (so `/` first), then the
// directories' prefixes sorted the same way, then `/cdn-cgi/` LAST.
//
// The command line (from `worker/`):
//
//   node scripts/edge-rule.mjs                   print the rule's JSON; 0
//   node scripts/edge-rule.mjs --verify < r.json compare the zone's rule; 0 equal, 1 different
//
// and 2 for a run that cannot judge: another argument, a tree that is not
// assembled, or input that is not JSON. `--verify` compares `expression`,
// `action` and `description`, and ignores every key the zone adds (`id`,
// `version`, `last_updated`, `ref`) and `enabled`, which the operator flips by
// the rule's id. This file imports nothing local, so a cell can run a copy of
// it beside a fixture tree.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

/** The Worker's hostname: the route `wrangler.open.jsonc` (and production's,
 *  which it equals there) names — held to both by a cell. */
export const HOST = "yokohama.moore.nyc";

/** What the rule does: Cloudflare's Managed Challenge. */
export const ACTION = "managed_challenge";

/** The rule's description, as the zone shows it and the smoke names it. */
export const DESCRIPTION = "yokohama — the no-Access door's edge check (F)";

/** Cloudflare's own paths: always allowed, whatever the tree holds. */
export const ALWAYS = "/cdn-cgi/";

/** The asset layer's two configuration files: in the tree, never served. */
const NEVER_LISTED = new Set(["_headers", "_redirects"]);

/** A name the rules language takes inside a string literal as it is. */
const PLAIN = /^[A-Za-z0-9._~-]+$/;

/** The allow-list of the tree at `tree`: `{ files, prefixes }` in the rule's
 *  order. Refuses a tree that is not there, and a top-level name the rule
 *  could not carry in a string literal unescaped. */
export function allowList(tree) {
  if (!existsSync(tree)) throw new Error(`edge-rule: ${tree} is absent — assemble the assets: npm run build:assets`);
  const files = [];
  const prefixes = [];
  for (const name of readdirSync(tree)) {
    if (NEVER_LISTED.has(name)) continue;
    if (!PLAIN.test(name)) {
      throw new Error(
        `edge-rule: the tree's top-level name ${JSON.stringify(name)} is not one the rule can carry — ` +
          "letters, digits, '.', '_', '~' and '-' only",
      );
    }
    if (statSync(join(tree, name)).isDirectory()) {
      if (`/${name}/` !== ALWAYS) prefixes.push(`/${name}/`);
    } else {
      files.push(`/${name}`);
      if (name === "index.html") files.push("/");
    }
  }
  const byCodeUnit = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
  return { files: files.sort(byCodeUnit), prefixes: [...prefixes.sort(byCodeUnit), ALWAYS] };
}

/** The rule's expression over `allow` for the hostname `host`. */
export function expression(allow, host) {
  if (!/^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$/.test(host)) {
    throw new Error(`edge-rule: ${JSON.stringify(host)} is not a hostname`);
  }
  const path = "http.request.uri.path";
  const allowed = [
    ...(allow.files.length > 0 ? [`${path} in {${allow.files.map((f) => `"${f}"`).join(" ")}}`] : []),
    ...allow.prefixes.map((p) => `starts_with(${path}, "${p}")`),
  ];
  return `(http.host eq "${host}" and not (${allowed.join(" or ")}))`;
}

/** The rule as the zone's `http_request_firewall_custom` ruleset takes it. */
export function ruleJson(allow, host) {
  return { expression: expression(allow, host), action: ACTION, description: DESCRIPTION, enabled: true };
}

/** What `verify` compares; the zone's other keys are its own. */
const COMPARED = ["expression", "action", "description"];

/** The differences between a rule read from the zone and the tree's: one
 *  sentence per compared key, `[]` when they agree. */
export function verify(ruleFromZone, allow, host) {
  if (ruleFromZone === null || typeof ruleFromZone !== "object" || Array.isArray(ruleFromZone)) {
    return ["the zone's rule is not a JSON object — pipe in the one rule, not a list or the API's envelope"];
  }
  const want = ruleJson(allow, host);
  const shown = (v) => (v === undefined ? "nothing" : JSON.stringify(v));
  return COMPARED.filter((key) => ruleFromZone[key] !== want[key]).map(
    (key) => `${key}: the zone's ${shown(ruleFromZone[key])} — the tree's ${JSON.stringify(want[key])}`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  const refuse = (sentence) => {
    console.error(`edge-rule: ${sentence}`);
    process.exit(2);
  };
  if (!(args.length === 0 || (args.length === 1 && args[0] === "--verify"))) {
    refuse(`expected no arguments or --verify — got ${JSON.stringify(args)}`);
  }
  let allow;
  try {
    allow = allowList(fileURLToPath(new URL("../assets", import.meta.url)));
  } catch (e) {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(2);
  }
  if (args.length === 0) {
    process.stdout.write(`${JSON.stringify(ruleJson(allow, HOST), null, 2)}\n`);
  } else {
    let zone;
    try {
      zone = JSON.parse(readFileSync(0, "utf8"));
    } catch (e) {
      refuse(`--verify reads the zone's rule as JSON on standard input — ${e instanceof Error ? e.message : String(e)}`);
    }
    const differences = verify(zone, allow, HOST);
    if (differences.length > 0) {
      console.error(`edge-rule: the zone's rule differs from the tree's —\n${differences.map((d) => `  ${d}`).join("\n")}`);
      process.exit(1);
    }
    console.log("edge-rule: the zone's rule equals the tree's — expression, action and description");
  }
}
