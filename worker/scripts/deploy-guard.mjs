// **The deploy guard.** `npm run deploy` is `node scripts/deploy-guard.mjs &&
// wrangler deploy`, and `npm run deploy:open` the same over `--config
// wrangler.open.jsonc` (R-6.5c2.39; checks 2–7 then judge THAT file, and 5 and
// 6 hold it to production's blocks), so nothing ships unless all nine hold. Every check that
// fails is named, not only the first.
//
// 1. `gen/factory.js` exists and does not contain `debug_trap(` — the RELEASE
//    module. M6.5b's `grep -c debug_trap` rule, for this module: the release
//    factory still NAMES `debug_trap` in the object it returns (`typeof
//    debug_trap === "function" ? …`), and only the debug glue defines and calls
//    it. And no `gen/.unoptimized` sits beside it: the build leaves that marker
//    when its optimizer step changed nothing (`scripts/build-module.mjs`).
// 2. The assets directory `wrangler.jsonc` names holds a `sw.js`, `verifyTree`
//    passes over it, and no browser module in it is a DEBUG build
//    (`debugModuleIn`) — so the tree wrangler uploads IS the release the
//    service worker installs, however the tree was made. A debug `web/dist`
//    carries a precache list that matches it, so the digests alone would pass
//    one (task 7 review, Important 1).
// 3. `wrangler.jsonc` names its door (M6.5c2, spec §2.1; R-6.5c2.5):
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
// 4. `YOKOHAMA_WEB_TEST_CLOCK` appears nowhere in `wrangler.jsonc` — read as
//    text, so an environment's `vars` or a comment cannot carry it past.
// 5. The `assets` block is EXACTLY spec §2.3's, in `wrangler.jsonc` AND in
//    both pool configurations, `wrangler.test.jsonc` and
//    `wrangler.open.test.jsonc` (M6.5c2): `html_handling` and
//    `not_found_handling` `none`, `run_worker_first` `["/api/*", "/enter"]`,
//    the binding `ASSETS`, and no key beyond those and `directory`. Battery
//    rows 13 and 14 (plan review M5): an asset-layer fallback answers a
//    missing file WITHOUT invoking the Worker, and the pool reads only its
//    own two files, so no pool cell can see production's block drift.
// 6. The `durable_objects`, `migrations` and `r2_buckets` blocks are present
//    in production's file and in each pool file, and each pool's EQUALS
//    production's, read as values (the orchestrator's ruling at Task 9's
//    review, and Task 13's bucket): the pool reads only its own two files, so
//    a class, a binding, a migration or a bucket there that production lacks
//    would be tested green and never shipped — and production's
//    `YOKOHAMA_WEB_ARCHIVE=r2` with no bucket bound is the 500 naming that
//    variable (spec §2.6).
// 7. No `vars` block in `wrangler.jsonc` — the top level's, or any
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
//    tree. Check 2 verifies the tree against ITS OWN `sw.js`, which a tree
//    assembled from an earlier `web/dist` passes; the bytes are compared, not
//    the release names alone, because the name digests the precache list and
//    a change to `web/src/sw.ts` alone keeps it.
//
// Checks 3–7 need no build, and `deploy-guard.test.mjs` runs them over the
// committed files — the per-commit catcher; the live reading at each deploy
// is the second.
//
// Every file is read with `jsonc-parser`, which knows a string from a
// comment: stripping `//` before `JSON.parse` would corrupt every string that
// holds one (every URL), and would choke on a trailing comma.

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join, posix } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parse, printParseErrorCode } from "jsonc-parser";
import { debugModuleIn, precacheEntriesFrom, verifyTree } from "./assemble-assets.mjs";
import { sourceDigest } from "./build-module.mjs";

/** Spec §2.3's handling, and the binding `/api/art` reads the manifest
 *  through. */
const ASSETS = {
  html_handling: "none",
  not_found_handling: "none",
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

/** Check 7, over `wrangler.jsonc`: every `vars` block, the top level's and
 *  each environment's, read for the secrets' KEYS. */
function secretsInVars(name, value, failures) {
  const blocks = [["vars", value.vars]];
  for (const [name, env] of Object.entries(objectOf(value.env) ?? {})) {
    if (objectOf(env)) blocks.push([`env.${name}.vars`, env.vars]);
  }
  for (const [where, vars] of blocks) {
    if (!objectOf(vars)) continue;
    for (const [key, why] of Object.entries(SECRETS)) {
      if (Object.hasOwn(vars, key)) failures.push(`${name}'s ${where} name ${key} — ${why}`);
    }
  }
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

/** Check 3, over `wrangler.jsonc`: the door, and its rules. */
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

/** Check 9: the tree `shown` at `tree` against the current `web/dist/sw.js`
 *  and `worker/static`. Asked only of a tree that holds a `sw.js` — check 2
 *  names a missing one. */
function staleAssets(root, repo, tree, shown, failures) {
  const dist = join(repo, "web", "dist", "sw.js");
  const rebuild = "run npm run build in web/, then in worker/";
  if (!existsSync(dist)) {
    failures.push(`web/dist/sw.js is absent, so nothing says whether worker/${shown} is the current release — ${rebuild}`);
  } else {
    const [assembled, current] = [readFileSync(join(tree, "sw.js"), "utf8"), readFileSync(dist, "utf8")];
    if (assembled !== current) {
      const [was, now] = [releaseOf(assembled), releaseOf(current)];
      const names =
        was === now ? `both name ${was}, so its service worker's own code moved` : `it holds ${was}, and web/dist ${now}`;
      failures.push(`worker/${shown} was assembled from a web/dist that has since changed — ${names}: ${rebuild}`);
    }
  }
  const statics = join(root, "static");
  for (const file of existsSync(statics) ? readdirSync(statics).sort() : []) {
    const copy = join(tree, file);
    if (!existsSync(copy) || !readFileSync(copy).equals(readFileSync(join(statics, file)))) {
      failures.push(`worker/${shown}/${file} is not worker/static/${file} — assemble the assets again: npm run build`);
    }
  }
}

/** Checks 3 to 7: the ones that need no build. `config` is the file the
 *  deploy ships (`--config`, default `wrangler.jsonc`): checks 3, 4 and 7
 *  judge it alone; check 5 holds production, the open file and each pool to
 *  spec §2.3's assets block, and check 6 holds the open file and each pool
 *  to production's object blocks, whichever file is named. */
export function configFailures(root, config = PRODUCTION_FILE) {
  const failures = [];
  const production = readConfig(root, PRODUCTION_FILE, failures);
  const judged = config === PRODUCTION_FILE ? production : readConfig(root, config, failures);
  if (judged) {
    doorRules(config, judged.value, failures);
    if (judged.text.includes("YOKOHAMA_WEB_TEST_CLOCK")) {
      failures.push(`${config} names YOKOHAMA_WEB_TEST_CLOCK — the test clock is set in the pool's configurations and nowhere else`);
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
  for (const name of [OPEN_FILE, GATE_FILE, ...POOLS]) {
    const other = name === config && judged ? judged : readConfig(root, name, failures);
    if (other) {
      if (name !== config || config !== PRODUCTION_FILE) assetsBlock(name, other.value, failures);
      others.push([name, other]);
    }
  }
  if (production) objectBlocks(production, others, failures);
  return failures;
}

/** All nine checks over the worker package at `root`, inside the repository
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

  // Checks 3–5 report a configuration that does not parse; check 2 only needs
  // to know where the tree is, and says nothing more when it cannot.
  const directory = readConfig(root, config, [])?.value.assets?.directory;
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
      "at the open door no dev door; no test clock; spec §2.3's assets block in all three configurations; " +
      "the same object bindings, migrations and bucket binding in each; no operator list or dev-door client id in its vars",
  );
}
