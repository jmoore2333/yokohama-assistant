// **The deploy guard.** `npm run deploy` is `node scripts/deploy-guard.mjs &&
// wrangler deploy`, and `npm run deploy:open` the same over `--config
// wrangler.open.jsonc` (R-6.5c2.39; checks 2–7 and 10–12 then judge THAT file, and 5 and
// 6 hold it to production's blocks), so nothing ships unless all twelve hold. Every check that
// fails is named, not only the first.
//
// 1. `gen/factory.js` exists and does not contain `debug_trap(` — the RELEASE
//    module. M6.5b's `grep -c debug_trap` rule, for this module: the release
//    factory still NAMES `debug_trap` in the object it returns (`typeof
//    debug_trap === "function" ? …`), and only the debug glue defines and calls
//    it. And no `gen/.unoptimized` sits beside it: the build leaves that marker
//    when its optimizer step changed nothing (`scripts/build-module.mjs`).
// 2. The assets directory the deployed configuration names holds a `sw.js`, `verifyTree`
//    passes over it, and no browser module in it is a DEBUG build
//    (`debugModuleIn`) — so the tree wrangler uploads IS the release the
//    service worker installs, however the tree was made. A debug `web/dist`
//    carries a precache list that matches it, so the digests alone would pass
//    one (task 7 review, Important 1).
// 3. The deployed configuration (`wrangler.jsonc`, or `--config`) names its door (M6.5c2, spec §2.1; R-6.5c2.5):
//    `YOKOHAMA_WEB_IDENTITY` in its top-level `vars` — the deploy's — is
//    `worker` or `open`, read as the shell reads it (trimmed, then matched
//    exactly), and there is no default, so a configuration naming none is
//    refused rather than deployed to answer every request the 500. Per door:
//    `"preview_urls": false` under both, since a preview version is not the
//    deployed code, whatever the door; under `worker`, `"workers_dev": false`
//    — a `workers.dev` URL would go around the hostname's Access application
//    — and `ACCESS_TEAM_DOMAIN` and `ACCESS_AUD` set, without which no
//    assertion is verified and every caller is no one; under `open`, a
//    `workers.dev` URL is allowed, there being no door to go around, and
//    `YOKOHAMA_DEV_DOOR` is not `on`, the dev door being the worker door's
//    test instrument (the Worker refuses the pair at run time too). Under a
//    door it does not know, `"workers_dev": false` is asked as under `worker`.
//    Absent is not false: with no route, an absent `workers_dev` is a public
//    `workers.dev` URL, and an absent `preview_urls` leaves the Worker's
//    Version URLs as they were — public, if they were ever on. The guard
//    prints the door it reads before its verdict (`doorLine`).
// 4. `YOKOHAMA_WEB_TEST_CLOCK` appears nowhere in the deployed configuration — read as
//    text, so an environment's `vars` or a comment cannot carry it past.
// 4b. `YOKOHAMA_DIAGNOSTIC_SEEDS` appears nowhere in the deployed configuration (R-SEC8(b)).
// 5. The `assets` block is EXACTLY spec §2.3's as cycle F amends it, in
//    `wrangler.jsonc`, the open and gate files AND both pool configurations,
//    `wrangler.test.jsonc` and `wrangler.open.test.jsonc` (M6.5c2):
//    `html_handling` `none`, `not_found_handling` `404-page` (cycle F, R-F5;
//    spec §2.5), `run_worker_first` `["/api/*", "/enter"]`, the binding
//    `ASSETS`, and no key beyond those and `directory`. Battery rows 13 and 14
//    (plan review M5): the asset layer answers what it answers WITHOUT
//    invoking the Worker, and the pool reads only its own two files, so no
//    pool cell can see production's block drift. `404-page` answers a miss
//    with the static `404.html` instead of a blank — read on 2026-10-01 in
//    every request shape for a miss under an allowed prefix, with no Worker
//    invocation (`worker/RUNBOOK.md` §20); a miss outside the allowed set is
//    the edge rule's to refuse, and what answers one without the rule was not
//    counted (the design expects the Worker for a request that is not a
//    navigation, spec §2.5) — so the page is a courtesy and the fence against
//    a scanner is the edge rule, never this.
//    `single-page-application` stays refused: it answered `index.html` with a
//    200 for a missing script, which a precache would store as the script.
// 6. The `durable_objects`, `migrations` and `r2_buckets` blocks are present
//    in production's file and in each pool file, and each pool's EQUALS
//    production's, read as values (the orchestrator's ruling at Task 9's
//    review, and Task 13's bucket): the pool reads only its own two files, so
//    a class, a binding, a migration or a bucket there that production lacks
//    would be tested green and never shipped — and production's
//    `YOKOHAMA_WEB_ARCHIVE=r2` with no bucket bound is the 500 naming that
//    variable (spec §2.6).
// 7. No `vars` block in the deployed configuration — the top level's, or any
//    environment's — names a SECRET: `YOKOHAMA_WEB_ADMINS` (the plan's ruling
//    P-1, as the orchestrator ruled at Task 14's review), whose value is an
//    address committed to the repository, or `DEV_DOOR_CLIENT_ID` (M6.5c2,
//    R-6.5c2.5). Both are set as secrets, which a deploy leaves alone. Read as
//    a KEY, so a comment may name either, and read in production's file
//    alone, so the pool keeps its test client id (review m9).
// 8. `gen/` is the build of the tree as it is now (R3's M-3): `gen/source.json`
//    — the stamp `npm run build` writes, the digest of what the module is
//    compiled from, taken before cargo ran (`build-module.mjs`'s
//    `sourceDigest`, ONE function for the two) — equals that digest taken
//    now. `wrangler deploy` bundles `src/` fresh and ships `gen/` as it sits,
//    so a crate changed and deployed without a rebuild would ship the old
//    module, whose exports the new shell may call and not find.
// 9. The assets tree is the assembly of the CURRENT `web/dist` and
//    `worker/static` (R3's M-3): its `sw.js` — the copy the assembly made,
//    which names the release it came from — is byte for byte
//    `web/dist/sw.js`, and each file of `worker/static` is its copy in the
//    tree. `_headers` is held instead to `assembledHeaders` of the static
//    file and the current `web/dist/headers.json` — the static text, then
//    the header set's `/*` block (security S2a, decision D-1) — and a
//    `web/dist` with no `headers.json` is refused, naming it. Check 2
//    verifies the tree against ITS OWN `sw.js`, which a tree
//    assembled from an earlier `web/dist` passes; the bytes are compared, not
//    the release names alone, because the name digests the precache list and
//    a change to `web/src/sw.ts` alone keeps it.
// 10. The deployed configuration does not turn on Workers Cache (M6.5c2 cycle
//    F, R-F3, R-FP13; spec §2.1): a `cache` block whose `enabled` is not
//    `false` is refused WHEREVER it sits — the top level, an environment, a
//    named export, or any nesting a later Wrangler may read one at — found by
//    walking the parsed configuration at every depth, arrays included, and
//    named by its whole path (Task 1 fix round 1: spec §2.1's "read as text
//    and as values", ruled a structural read that does not depend on the
//    guard knowing the places). With it on, "requests served from the
//    Worker's cache are billed at the same per-request rate as requests that
//    invoke the Worker. This includes requests to static assets" (Workers
//    pricing, footnote 4), and the door stops being free. The block is an
//    opt-in, so no block is off; the parse drops comments, so a comment is
//    prose.
// 11. The assets tree holds `404.html` (cycle F, R-F5, R-FP13; spec §2.5) —
//    the page check 5's `404-page` answers a browser's miss with; without it
//    the asset layer has nothing to serve and the miss is a blank again.
//    Read off the tree the deploy uploads, never `web/dist`.
// 12. The deploy's Turnstile DECLARATION agrees with the build it ships
//    (cycle F, R-FP3, R-FP4; spec §2.6): the judged configuration's top-level
//    `vars.YOKOHAMA_TURNSTILE` — `on`, `off`, or absent, which is `off`; any
//    other value is refused rather than read as `off` — against the tree's
//    `build-values.json`, the stamp the client build writes (six booleans
//    naming which build values were set, never a value), whose
//    `turnstile_sitekey` says whether the bundle carries a site key. `on`
//    exactly when it does: `on` over a keyless build is a door whose
//    challenged fetches have no widget to pass them, and a key under `off`
//    renders the widget under Access (R-FP4: one client build per door
//    state). A tree with no stamp, or a stamp that is not one, is refused
//    naming the file. Never read by the Worker at run time. **And the
//    identity rule** (Task 7, the orchestrator's amendment from Task 6's
//    re-review): `on`, and a keyed stamp, stand only beside
//    `YOKOHAMA_WEB_IDENTITY` `open`, read as the shell reads the door; under
//    any other each is refused with ONE sentence naming the identity and the
//    declaration, and the stamp comparison is not asked there. The client
//    infers the open door from the presence of a site key
//    (`web/src/outbox.ts`'s keyed-build rule), so a keyed build behind Access
//    would silently never upload a nameless visitor's games. The open file's
//    `on` is one of its allowed differences from production (T8-a), and a POOL
//    file's `on` is refused whichever file is judged: the pool never deploys,
//    so never a keyed build.
//
// Checks 2, 9, 11 and 12 read the assets tree the judged configuration names,
// and 9, 11 and 12 only an ASSEMBLED one — a tree with a `sw.js`; check 2 names
// a tree without one, and its sentence is the one to act on.
//
// Checks 3–7 need no build, and `deploy-guard.test.mjs` runs them over the
// committed files — the per-commit catcher; the live reading at each deploy
// is the second.
//
// Every file is read with `jsonc-parser`, which knows a string from a
// comment: stripping `//` before `JSON.parse` would corrupt every string that
// holds one (every URL), and would choke on a trailing comma.

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, posix } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parse, printParseErrorCode } from "jsonc-parser";
import { readHeadersText } from "../../web/scripts/headers.mjs";
import { assembledHeaders, debugModuleIn, precacheEntriesFrom, verifyTree } from "./assemble-assets.mjs";
import { sourceDigest } from "./build-module.mjs";

/** Spec §2.3's handling as cycle F amends it (`not_found_handling`, spec
 *  §2.5), and the binding `/api/art` reads the manifest through. */
const ASSETS = {
  html_handling: "none",
  not_found_handling: "404-page",
  run_worker_first: ["/api/*", "/enter"],
  binding: "ASSETS",
};
const ASSET_KEYS = [...Object.keys(ASSETS), "directory"].sort();

/** 1-based line and column of `offset` in `text`. */
function position(text, offset) {
  const before = text.slice(0, offset).split("\n");
  return `line ${before.length}, column ${before.at(-1).length + 1}`;
}

/** The configuration named `name` under `root`, parsed — or `undefined`, with
 *  the reason in `failures`. */
function readConfig(root, name, failures) {
  const file = join(root, name);
  if (!existsSync(file)) {
    failures.push(`${name} is absent`);
    return undefined;
  }
  const text = readFileSync(file, "utf8");
  const errors = [];
  const value = parse(text, errors, { allowTrailingComma: true, disallowComments: false });
  if (errors.length > 0) {
    const each = errors.map((e) => `${printParseErrorCode(e.error)} at ${position(text, e.offset)}`);
    failures.push(`${name} does not parse: ${each.join("; ")}`);
    return undefined;
  }
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    failures.push(`${name} is not an object`);
    return undefined;
  }
  return { text, value };
}

/** Check 5, over one configuration. */
function assetsBlock(name, value, failures) {
  const block = value.assets;
  if (block === null || typeof block !== "object" || Array.isArray(block)) {
    failures.push(`${name} has no assets block — spec §2.3's is required`);
    return;
  }
  const keys = Object.keys(block).sort();
  if (JSON.stringify(keys) !== JSON.stringify(ASSET_KEYS)) {
    failures.push(`${name}'s assets block has the keys ${keys.join(", ")} — spec §2.3 names ${ASSET_KEYS.join(", ")}`);
  }
  for (const [key, want] of Object.entries(ASSETS)) {
    if (JSON.stringify(block[key]) !== JSON.stringify(want)) {
      failures.push(`${name}'s assets.${key} must be ${JSON.stringify(want)} — got ${JSON.stringify(block[key]) ?? "nothing"}`);
    }
  }
  if (typeof block.directory !== "string" || block.directory === "") {
    failures.push(`${name}'s assets.directory must name the directory to upload`);
  }
}

/** Why an absent key is not `false` (check 3). */
const ABSENT = {
  workers_dev: "with no route, wrangler serves an absent one as a public workers.dev URL",
  preview_urls: "wrangler leaves an absent one as the Worker already has it — public, if it was ever on",
};

/** `value` as JSON with every object's keys in order, so two blocks compare
 *  by value whatever order their keys were written in. */
function canonical(value) {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${canonical(value[k])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

/** Production's configuration, the reference the others are held to. */
const PRODUCTION_FILE = "wrangler.jsonc";

/** The open door's deployment configuration (R-6.5c2.39): the state-1 window,
 *  held to production's blocks like a pool file, and judged whole by
 *  `--config wrangler.open.jsonc`. */
const OPEN_FILE = "wrangler.open.jsonc";

/** The gate window's configuration (R-E10): production with the dev door
 *  "off" — held to production's blocks like the open file. */
const GATE_FILE = "wrangler.gate.jsonc";

/** What check 6 compares, and what each block's absence leaves undeclared. */
const OBJECT_BLOCKS = {
  durable_objects: "binds the three object classes",
  migrations: "migrates the three object classes",
  r2_buckets: "binds the bucket yokohama-archive as ARCHIVE",
};

/** The pool's configurations, one per door (M6.5c2, spec §2.1): each is held
 *  to spec §2.3's assets block (check 5) and to production's object blocks
 *  (check 6). */
const POOLS = ["wrangler.test.jsonc", "wrangler.open.test.jsonc"];

/** Check 6, over production and each pool file that parsed (`pools`, as
 *  `[name, config]` pairs). A block a file lacks is named for that file; a
 *  pool's block that differs from production's is named for that pool. */
function objectBlocks(production, pools, failures) {
  for (const [key, what] of Object.entries(OBJECT_BLOCKS)) {
    const missing = [[PRODUCTION_FILE, production], ...pools].filter(([, config]) => config.value[key] === undefined);
    for (const [name] of missing) failures.push(`${name} has no ${key} block — spec §3.1 ${what}`);
    if (missing.length > 0) continue;
    const p = canonical(production.value[key]);
    for (const [name, pool] of pools) {
      const t = canonical(pool.value[key]);
      if (p !== t) {
        failures.push(`${name}'s ${key} differs from wrangler.jsonc's — it would ship or test what production does not ship: ${t} against ${p}`);
      }
    }
  }
}

/** What check 7 refuses among production's `vars`: the SECRETS, and why. */
const SECRETS = {
  YOKOHAMA_WEB_ADMINS: "the operator list is a secret (wrangler secret put), never a committed value",
  DEV_DOOR_CLIENT_ID: "the dev door's client id is a secret (wrangler secret put), never a committed value",
};

/** `value` when it is an object that is not an array — a `vars` block or an
 *  environment — else `undefined`. */
function objectOf(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value) ? value : undefined;
}

/** Every `vars` block of a configuration that is an object, with where it
 *  sits: the top level's (`vars`) and each environment's (`env.<name>.vars`).
 *  Check 7 and rule 4b's key half read the same blocks. */
function varsBlocks(value) {
  const blocks = [["vars", value.vars]];
  for (const [name, env] of Object.entries(objectOf(value.env) ?? {})) {
    if (objectOf(env)) blocks.push([`env.${name}.vars`, env.vars]);
  }
  return blocks.filter(([, vars]) => objectOf(vars));
}

/** Check 7, over the deployed configuration: every `vars` block, the top level's and
 *  each environment's, read for the secrets' KEYS. */
function secretsInVars(name, value, failures) {
  for (const [where, vars] of varsBlocks(value)) {
    for (const [key, why] of Object.entries(SECRETS)) {
      if (Object.hasOwn(vars, key)) failures.push(`${name}'s ${where} name ${key} — ${why}`);
    }
  }
}

/** Whether the judged configuration names `key` — rule 4b's question, and the
 *  test clock's. Its TEXT, as ever, so a comment or a string naming the key is
 *  refused too; AND its parsed `vars` blocks' keys (the S1 stack review's
 *  HI-F3): a key spelled with a JSON `\u` escape is not in the text, and the
 *  parser — this file's own `jsonc-parser`, as wrangler's — reads it as the
 *  key, so the text search alone shipped `YOKOHAMA_DIAGNOSTIC_SEEDS` on with
 *  no failure. */
function namesVar(judged, key) {
  return judged.text.includes(key) || varsBlocks(judged.value).some(([, vars]) => Object.hasOwn(vars, key));
}

/** The two doors (M6.5c2, spec §2.1), and what the guard says each one is. */
const DOORS = {
  worker: "Cloudflare Access, verified by the shell",
  open: "no identity service: anyone who reaches the Worker plays under any name",
};

/** The door a configuration's top-level `vars` names — the deploy's, so an
 *  environment's names none here — read as the shell reads the variable
 *  (`config.ts`'s `door`): a string trimmed, and nothing else changed. Any
 *  other value is handed back as it is, and `undefined` means none. */
function doorOf(value) {
  const raw = objectOf(value.vars)?.YOKOHAMA_WEB_IDENTITY;
  return typeof raw === "string" ? raw.trim() : raw;
}

/** Whether `door` is one of the two, compared exactly. */
function known(door) {
  return typeof door === "string" && Object.hasOwn(DOORS, door);
}

/** What each deployable sibling may differ in: `vars` keys, each with the
 *  value the sibling must hold (`undefined`: absent). Everything else is
 *  production's, exactly. */
const GATE_DIFFERS = { YOKOHAMA_DEV_DOOR: "off" };
const OPEN_DIFFERS = {
  YOKOHAMA_WEB_IDENTITY: "open",
  ACCESS_TEAM_DOMAIN: undefined,
  ACCESS_AUD: undefined,
  YOKOHAMA_DEV_DOOR: undefined,
  // The window ships a keyed build (cycle F, R-FP4): check 12 holds it there.
  YOKOHAMA_TURNSTILE: "on",
};

/** T8-a: `sibling` equals `production` (comments are already gone, keys are
 *  compared sorted) except the `vars` keys in `differs`, each of which must
 *  hold exactly its stated value. Each drift names the key. */
function siblingDrift(name, production, sibling, differs, failures) {
  const shown = (v) => (v === undefined ? "nothing" : canonical(v));
  const drift = (key, mine, theirs) =>
    failures.push(`${name}'s ${key} differs from wrangler.jsonc's — a deploy of it would roll production back: ${shown(mine)} against ${shown(theirs)}`);
  for (const key of new Set([...Object.keys(production), ...Object.keys(sibling)])) {
    if (key === "vars") continue;
    if (canonical(production[key]) !== canonical(sibling[key])) drift(key, sibling[key], production[key]);
  }
  const p = objectOf(production.vars) ?? {};
  const v = objectOf(sibling.vars) ?? {};
  for (const key of new Set([...Object.keys(p), ...Object.keys(v)])) {
    if (Object.hasOwn(differs, key)) {
      if (canonical(v[key]) !== canonical(differs[key])) {
        failures.push(`${name}'s vars.${key} must be ${differs[key] === undefined ? "absent" : JSON.stringify(differs[key])} — got ${shown(v[key])}`);
      }
    } else if (canonical(p[key]) !== canonical(v[key])) {
      drift(`vars.${key}`, v[key], p[key]);
    }
  }
}

/** Check 10, over the deployed configuration's parsed value: EVERY property
 *  named `cache`, at any depth — objects and arrays alike, a `cache` block's
 *  own contents included — refused unless its value's `enabled` is `false`.
 *  Named by its dotted path (an array element by its index; the top level with
 *  no prefix), so two blocks are two sentences, in the file's order. The value
 *  is `parse`'s, the same read Wrangler makes: comments are gone, and of two
 *  duplicate keys the last is the one read, so no path is reported twice. */
export function cacheBlock(name, value, failures) {
  const walk = (node, prefix) => {
    if (Array.isArray(node)) {
      node.forEach((item, i) => walk(item, `${prefix}${i}.`));
      return;
    }
    if (!objectOf(node)) return;
    for (const [key, child] of Object.entries(node)) {
      if (key === "cache" && objectOf(child)?.enabled !== false) {
        failures.push(
          `${name} enables Workers Cache (${prefix}cache.enabled is not false) — with it on, static-asset requests are ` +
            "billed at the Worker request rate (Workers pricing, footnote 4); remove the block",
        );
      }
      walk(child, `${prefix}${key}.`);
    }
  };
  walk(value, "");
}

/** Check 3, over the deployed configuration: the door, and its rules. */
function doorRules(name, value, failures) {
  const door = doorOf(value);
  if (door === undefined) {
    failures.push(`${name} names no door — its vars must set YOKOHAMA_WEB_IDENTITY to worker or open, and there is no default`);
  } else if (!known(door)) {
    failures.push(`${name}'s YOKOHAMA_WEB_IDENTITY must be worker or open — got ${JSON.stringify(door)}`);
  }
  for (const [key, why] of Object.entries(ABSENT)) {
    // An open door has no Access application for a workers.dev URL to go around.
    if (key === "workers_dev" && door === "open") continue;
    if (value[key] !== false) {
      failures.push(`${name} must set "${key}": false — got ${JSON.stringify(value[key]) ?? "nothing"}; ${why}`);
    }
  }
  const vars = objectOf(value.vars) ?? {};
  if (door === "worker") {
    for (const key of ["ACCESS_TEAM_DOMAIN", "ACCESS_AUD"]) {
      if (typeof vars[key] !== "string" || vars[key].trim() === "") {
        failures.push(
          `${name}'s vars must set ${key} under the worker door — without it no assertion is verified, and every caller is no one`,
        );
      }
    }
  }
  // Read as the shell reads a switch (`config.ts`'s `folded`): trimmed, in any case.
  const devDoor = vars.YOKOHAMA_DEV_DOOR;
  if (door === "open" && typeof devDoor === "string" && devDoor.trim().replace(/[A-Z]/g, (c) => c.toLowerCase()) === "on") {
    failures.push(
      `${name}'s vars set YOKOHAMA_DEV_DOOR=on beside YOKOHAMA_WEB_IDENTITY=open — the dev door is the worker door's test instrument, and the Worker refuses the pair`,
    );
  }
}

/** The line the guard prints about the door `wrangler.jsonc` under `root`
 *  names (R-6.5c2.5), whatever else the guard refuses — or `undefined` when
 *  the file is absent or does not parse, or names neither door, which check 3
 *  refuses. */
export function doorLine(root, config = PRODUCTION_FILE) {
  const judged = readConfig(root, config, []);
  const door = judged === undefined ? undefined : doorOf(judged.value);
  return known(door) ? `${config} deploys the ${door} door — ${DOORS[door]}` : undefined;
}

/** Check 8: `gen/source.json` against the digest of the sources now. Asked
 *  only of a module that is there — check 1 names a missing one. */
function staleModule(root, repo, failures) {
  const stamp = join(root, "gen", "source.json");
  if (!existsSync(stamp)) {
    failures.push("gen/source.json is absent — this module was not built by npm run build: build the release: npm run build");
    return;
  }
  let built;
  try {
    built = JSON.parse(readFileSync(stamp, "utf8"))?.digest;
  } catch {
    built = undefined;
  }
  if (typeof built !== "string") {
    failures.push("gen/source.json is not the build's stamp — build the release again: npm run build");
  } else if (built !== sourceDigest(repo)) {
    failures.push("gen/ was built from sources that have since changed — build the release again: npm run build");
  }
}

/** The release a built service worker names, off the banner
 *  `web/scripts/precache.mjs` writes — or a stand-in when it names none,
 *  since the bytes, not the name, decide check 9. */
function releaseOf(sw) {
  return /the release is (yokohama-release-[0-9a-f]+)\./.exec(sw)?.[1] ?? "no release name";
}

/** Whether `path` is a regular file (a directory of that name is not). */
const isFile = (path) => statSync(path, { throwIfNoEntry: false })?.isFile() === true;

/** What a refusal of the assets tree's contents asks for: the client built,
 *  then the tree assembled from it (checks 9, 11 and 12). */
const REBUILD = "run npm run build in web/, then in worker/";

/** Check 9: the tree `shown` at `tree` against the current `web/dist/sw.js`
 *  and `worker/static`. Asked only of a tree that holds a `sw.js` — check 2
 *  names a missing one. */
function staleAssets(root, repo, tree, shown, failures) {
  const dist = join(repo, "web", "dist", "sw.js");
  if (!existsSync(dist)) {
    failures.push(`web/dist/sw.js is absent, so nothing says whether worker/${shown} is the current release — ${REBUILD}`);
  } else {
    const [assembled, current] = [readFileSync(join(tree, "sw.js"), "utf8"), readFileSync(dist, "utf8")];
    if (assembled !== current) {
      const [was, now] = [releaseOf(assembled), releaseOf(current)];
      const names =
        was === now ? `both name ${was}, so its service worker's own code moved` : `it holds ${was}, and web/dist ${now}`;
      failures.push(`worker/${shown} was assembled from a web/dist that has since changed — ${names}: ${REBUILD}`);
    }
  }
  // Check 9's `_headers` arm, OUTSIDE the loop over the static files (Task 2's review, I-1: inside it, a tree whose
  // `worker/static` had no `_headers` was never held, and a missing `headers.json` never named): the assembled
  // `_headers` is held to `assembledHeaders(the static text — "" when the static file is absent, as the assembly
  // reads it —, web/dist/headers.json's text)` whether or not the static file exists.
  const generated = join(repo, "web", "dist", "headers.json");
  const statics = join(root, "static");
  const staticHeaders = join(statics, "_headers");
  const headersCopy = join(tree, "_headers");
  if (!existsSync(generated)) {
    failures.push(`web/dist/headers.json is absent, so nothing says which header set worker/${shown}/_headers should carry — ${REBUILD}`);
  } else {
    // The reader's refusals — not JSON, not the set's shape, not UTF-8, a BOM — are FAILURES this guard names,
    // never a throw out of `deployGuard`: the deploy stops either way, but the contract is the list (Task 2's C-1).
    let want = null;
    try {
      want = assembledHeaders(existsSync(staticHeaders) ? readFileSync(staticHeaders, "utf8") : "", readHeadersText(generated));
    } catch (e) {
      failures.push(`web/dist/headers.json is not the client build's header set (${e instanceof Error ? e.message : String(e)}) — ${REBUILD}`);
    }
    if (want !== null && (!existsSync(headersCopy) || readFileSync(headersCopy, "utf8") !== want)) {
      failures.push(`worker/${shown}/_headers is not worker/static/_headers with the current web/dist/headers.json's block — assemble the assets again: npm run build`);
    }
  }
  for (const file of existsSync(statics) ? readdirSync(statics).sort() : []) {
    if (file === "_headers") continue;
    const copy = join(tree, file);
    const source = readFileSync(join(statics, file));
    if (!existsSync(copy) || !readFileSync(copy).equals(source)) {
      failures.push(`worker/${shown}/${file} is not worker/static/${file} — assemble the assets again: npm run build`);
    }
  }
}

/** Check 11: the assets tree at `tree` (printed as `shown`) holds the 404 page
 *  `not_found_handling: "404-page"` serves. */
function notFoundPage(tree, failures, shown = tree) {
  if (!isFile(join(tree, "404.html"))) {
    failures.push(`${shown}/404.html is absent — not_found_handling "404-page" answers a browser's miss with it: ${REBUILD}`);
  }
}

/** What check 12's identity rule says, after naming the file, the
 *  declaration and the identity (the orchestrator's amendment, Task 7). */
const KEYED_IS_OPEN =
  "a keyed build is the open door (R-FP4): the widget and the client's keyed-build rule assume no Access in front of the API";

/** The identity a configuration names, as check 12's sentences show it. */
function identityShown(door) {
  if (door === undefined) return "nothing";
  return typeof door === "string" && door !== "" ? door : JSON.stringify(door);
}

/** Check 12: the judged configuration `config`'s (parsed `value`) Turnstile
 *  declaration against the stamp in the tree at `tree` (printed as `shown`),
 *  and against its door. Both are read whatever the other says, so a bad
 *  declaration and a missing stamp are two sentences. The identity rule
 *  comes first: `on`, or a keyed stamp, under any door but `open` is ONE
 *  sentence naming the identity and the declaration, and the stamp is
 *  compared with the declaration only at the open door — at any other, its
 *  sentences' advice (build with the key, set the var on) is wrong. */
function turnstileConsistency(config, value, tree, failures, shown = tree) {
  const declared = objectOf(value.vars)?.YOKOHAMA_TURNSTILE;
  const readable = declared === undefined || declared === "on" || declared === "off";
  if (!readable) {
    failures.push(`${config}'s vars.YOKOHAMA_TURNSTILE must be "on" or "off" (absent is off) — got ${JSON.stringify(declared)}`);
  }
  const door = doorOf(value);
  const open = door === "open";
  const on = declared === "on";
  if (on && !open) {
    failures.push(
      `${config} sets YOKOHAMA_TURNSTILE on under identity ${identityShown(door)} — ${KEYED_IS_OPEN}; ` +
        "deploy the key only with wrangler.open.jsonc",
    );
  }
  const file = join(tree, "build-values.json");
  if (!isFile(file)) {
    failures.push(`${shown}/build-values.json is absent, so nothing says whether the build carries a Turnstile site key — ${REBUILD}`);
    return;
  }
  let stamp;
  let why;
  try {
    stamp = JSON.parse(readFileSync(file, "utf8"));
  } catch {
    why = "it does not parse";
  }
  if (why === undefined && objectOf(stamp) === undefined) why = "it is not an object";
  if (why === undefined && typeof stamp.turnstile_sitekey !== "boolean") why = "its turnstile_sitekey is not a boolean";
  if (why !== undefined) {
    failures.push(`${shown}/build-values.json is not the client build's stamp (${why}) — ${REBUILD}`);
    return;
  }
  if (!readable) return;
  if (!open) {
    if (!on && stamp.turnstile_sitekey) {
      failures.push(
        `${config} leaves YOKOHAMA_TURNSTILE off under identity ${identityShown(door)} but the assets tree's build carries a ` +
          `site key — ${KEYED_IS_OPEN}; build the client without the key, and deploy the key only with wrangler.open.jsonc`,
      );
    }
    return;
  }
  if (on && !stamp.turnstile_sitekey) {
    failures.push(
      `${config} sets YOKOHAMA_TURNSTILE on but the assets tree's build carries no site key (build-values.json) — ` +
        "build the client with YOKOHAMA_TURNSTILE_SITEKEY set",
    );
  } else if (!on && stamp.turnstile_sitekey) {
    // Asked only at the open door, which has no Access in front of it: a
    // keyed build there is the open door's own, and its declaration says so.
    failures.push(
      `${config} leaves YOKOHAMA_TURNSTILE off but the assets tree's build carries a site key — a keyed build at ` +
        "the open door declares on: set the var on, or build the client without the key",
    );
  }
}

/** The pool's two files never deploy, so never a keyed build (cycle F,
 *  R-FP4): `YOKOHAMA_TURNSTILE` there is `off` or absent, whichever file the
 *  guard judges. */
function poolDeclaration(name, value, failures) {
  const declared = objectOf(value.vars)?.YOKOHAMA_TURNSTILE;
  if (declared !== undefined && declared !== "off") {
    failures.push(
      `${name}'s vars.YOKOHAMA_TURNSTILE must be "off" (absent is off) — the pool never deploys, so it never ships a keyed build; ` +
        `got ${JSON.stringify(declared)}`,
    );
  }
}

/** Checks 3 to 7: the ones that need no build. `config` is the file the
 *  deploy ships (`--config`, default `wrangler.jsonc`): checks 3, 4 and 7
 *  judge it alone; check 5 holds production, the open file and each pool to
 *  spec §2.3's assets block, and check 6 holds the open file and each pool
 *  to production's object blocks, whichever file is named. Check 12's pool
 *  half rides here too (cycle F): neither pool file declares `on`. */
export function configFailures(root, config = PRODUCTION_FILE) {
  const failures = [];
  const production = readConfig(root, PRODUCTION_FILE, failures);
  const judged = config === PRODUCTION_FILE ? production : readConfig(root, config, failures);
  if (judged) {
    doorRules(config, judged.value, failures);
    if (namesVar(judged, "YOKOHAMA_WEB_TEST_CLOCK")) {
      failures.push(`${config} names YOKOHAMA_WEB_TEST_CLOCK — the test clock is set in the pool's configurations and nowhere else`);
    }
    if (namesVar(judged, "YOKOHAMA_DIAGNOSTIC_SEEDS")) {
      failures.push(`${config} names YOKOHAMA_DIAGNOSTIC_SEEDS — diagnostic seeds are set in the pool's configurations and nowhere else`);
    }
    secretsInVars(config, judged.value, failures);
    // The gate file exists to shut the dev door: anything but "off" defeats it.
    if (config === GATE_FILE) {
      const gate = objectOf(judged.value.vars)?.YOKOHAMA_DEV_DOOR;
      if (gate !== "off") {
        failures.push(`${config}'s vars must set YOKOHAMA_DEV_DOOR to "off" — its only purpose is the worker door with the dev door shut; got ${JSON.stringify(gate) ?? "nothing"}`);
      }
    }
  }
  if (production) assetsBlock(PRODUCTION_FILE, production.value, failures);
  const others = [];
  // The named configuration is held to the same blocks even when it is none of
  // the listed ones (`configFrom` accepts any `wrangler*.jsonc`).
  const held = [OPEN_FILE, GATE_FILE, ...POOLS];
  if (config !== PRODUCTION_FILE && !held.includes(config)) held.push(config);
  for (const name of held) {
    const other = name === config && judged ? judged : readConfig(root, name, failures);
    if (other) {
      assetsBlock(name, other.value, failures);
      if (POOLS.includes(name)) poolDeclaration(name, other.value, failures);
      others.push([name, other]);
    }
  }
  if (production) objectBlocks(production, others, failures);
  // A deployable sibling is production with ONE thing changed (T8-a): a deploy
  // REPLACES the Worker's variables wholesale, so a sibling that fell behind
  // production would roll it back. Judged when the file is the one deployed.
  if (production && judged && config === GATE_FILE) siblingDrift(config, production.value, judged.value, GATE_DIFFERS, failures);
  if (production && judged && config === OPEN_FILE) siblingDrift(config, production.value, judged.value, OPEN_DIFFERS, failures);
  return failures;
}

/** All twelve checks over the worker package at `root`, inside the repository
 *  at `repo`: `[]` when every one holds. */
export function deployGuard(root, repo = join(root, ".."), config = PRODUCTION_FILE) {
  const failures = [];

  const factory = join(root, "gen", "factory.js");
  if (!existsSync(factory)) {
    failures.push("gen/factory.js is absent — build the release module: npm run build");
  } else {
    if (readFileSync(factory, "utf8").includes("debug_trap(")) {
      failures.push("gen/factory.js is the DEBUG module (it defines debug_trap) — build the release: npm run build");
    }
    staleModule(root, repo, failures);
  }
  if (existsSync(join(root, "gen", ".unoptimized"))) {
    failures.push(
      "gen/.unoptimized marks an UNOPTIMIZED module — the release build's optimizer step changed nothing " +
        "(web/scripts/wasm-opt.mjs skips when binaryen is absent and YOKOHAMA_WASM_OPT_OPTIONAL=1); " +
        "install binaryen and build the release: npm run build",
    );
  }

  // Checks 3–5 report a configuration that does not parse; checks 2 and 9–12
  // only need its values, and say nothing more when they cannot be read.
  const judged = readConfig(root, config, [])?.value;
  if (judged) cacheBlock(config, judged, failures);
  const directory = judged?.assets?.directory;
  if (typeof directory === "string" && directory !== "") {
    const shown = posix.normalize(directory);
    const tree = join(root, directory);
    const sw = join(tree, "sw.js");
    if (!existsSync(sw)) {
      failures.push(`${shown}/sw.js is absent — assemble the assets: npm run build`);
    } else {
      try {
        verifyTree(tree, precacheEntriesFrom(readFileSync(sw, "utf8")));
      } catch (e) {
        failures.push(`${shown}: ${e instanceof Error ? e.message : String(e)}`);
      }
      staleAssets(root, repo, tree, shown, failures);
      notFoundPage(tree, failures, shown);
      turnstileConsistency(config, judged, tree, failures, shown);
    }
    const debug = existsSync(tree) ? debugModuleIn(tree) : undefined;
    if (debug) {
      failures.push(
        `${shown}/${debug} is a DEBUG browser module (it exports debug_trap) — ` +
          "build the release (cd web && npm run build), then assemble the assets again: npm run build:assets",
      );
    }
  }

  failures.push(...configFailures(root, config));
  return failures;
}

/** The configuration the CLI was asked to judge: `--config <file>` (a bare
 *  file name in this directory), default `wrangler.jsonc`. Any other argument,
 *  or a path, is refused rather than ignored — a guard that judged one file
 *  while `wrangler deploy` shipped another would be worse than none. */
export function configFrom(argv) {
  const args = argv.slice(2);
  if (args.length === 0) return PRODUCTION_FILE;
  if (args.length === 2 && args[0] === "--config" && /^wrangler[A-Za-z0-9.-]*\.jsonc$/.test(args[1])) return args[1];
  throw new Error(`deploy-guard: expected no arguments or --config wrangler<name>.jsonc — got ${JSON.stringify(args)}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const root = fileURLToPath(new URL("..", import.meta.url));
  let config;
  try {
    config = configFrom(process.argv);
  } catch (e) {
    console.error(e.message);
    process.exit(1);
  }
  const door = doorLine(root, config);
  if (door !== undefined) console.log(door);
  const failures = deployGuard(root, join(root, ".."), config);
  if (failures.length > 0) {
    console.error(`deploy-guard: refused —\n${failures.map((f) => `  ${f}`).join("\n")}`);
    process.exit(1);
  }
  console.log(
    "deploy-guard: the optimized release module, built from the sources as they are; the assets tree is the release, " +
      "with no debug browser module, assembled from the current web/dist and worker/static; " +
      "a door named, and its rules: no preview URL, and at the worker door no workers.dev URL and both Access values, " +
      "at the open door no dev door; no test clock; no diagnostic seeds; spec §2.3's assets block in production, the open and gate files and both pool files; the open and gate files equal production but for their door, and the open file's declaration; " +
      "the same object bindings, migrations and bucket binding in each; no operator list or dev-door client id in its vars; " +
      "no Workers Cache; the 404 page in the tree; YOKOHAMA_TURNSTILE as the build's stamp says, on only at the open door and never in the pool",
  );
}
