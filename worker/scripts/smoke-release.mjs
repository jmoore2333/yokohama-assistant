// **The release module's smoke** — `npm run build`'s LAST step (the
// orchestrator's ruling at Task 7's review). Nothing else instantiates the
// RELEASE module before a deploy: the pool runs the DEBUG build, whose trap
// cells need `debug_trap`, and `cargo test` runs the crate natively. So this
// builds one instance of it in Node, once, and asks it one question:
//
//   createModule(the compiled gen/yokohama_worker_bg.wasm)   (gen/factory.js)
//   init(the `off` configuration)
//   dispatch("smoke@x.test", "GET", "/api/health", "", undefined, <no bytes>)
//
// and REFUSES — exit 1, naming what it saw — unless the answer is 200. It
// refuses a module that exports `debug_trap` as well: that is the debug build,
// which never deploys. The live readings after a deploy stay the second
// catcher.
//
// **Its LIVE mode, the edge line** (M6.5c2 cycle F, R-F6, R-FP7; spec §2.6;
// the orchestrator's ruling on the smoke) is a second command, run after a
// deploy against the DEPLOYED origin:
//
//   node scripts/smoke-release.mjs --live <origin> --config <wrangler file>
//
// It reads the named configuration's `vars.YOKOHAMA_TURNSTILE`, the deploy's
// DECLARATION. Under `on`, a cookie-less `GET <origin>/api/health` with a
// browser User-Agent must answer 403 with `cf-mitigated: challenge` — the
// zone's edge rule (`scripts/edge-rule.mjs`) standing in front of the API —
// or the smoke FAILS, exit 1, naming the rule: a switch that says `on` over a
// missing rule is not a state this project ships. Under `off`, or absent, it
// makes no request and says no edge line is owed. Its first run in a window
// is EXPECTED to fail, before the rule is created (`worker/RUNBOOK.md` §3).

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { parse, printParseErrorCode } from "jsonc-parser";
import { DESCRIPTION } from "./edge-rule.mjs";

/** `init`'s configuration: the worker door — the one this repository
 *  deploys (M6.5c2) — the archive off, no caller, the chat variables unset:
 *  the layer's defaults. */
export const SMOKE_CONFIG = JSON.stringify({
  identity: "worker",
  archive: "off",
  caller: null,
  lobby_chat: null,
  chat_keep: null,
  chat_text: null,
});

/** The smoke over the module in `gen`: a line saying what it answered, or a
 *  throw naming why it is refused. */
export async function smokeRelease(gen) {
  for (const file of ["factory.js", "yokohama_worker_bg.wasm"]) {
    if (!existsSync(join(gen, file))) {
      throw new Error(`smoke-release: gen/${file} is absent — build the release module: npm run build`);
    }
  }
  const { createModule } = await import(pathToFileURL(join(gen, "factory.js")).href);
  const mod = createModule(new WebAssembly.Module(readFileSync(join(gen, "yokohama_worker_bg.wasm"))));
  if (typeof mod.debug_trap === "function") {
    throw new Error("smoke-release: gen/factory.js is the DEBUG module (it exports debug_trap) — build the release: npm run build");
  }
  try {
    mod.init(SMOKE_CONFIG);
  } catch (e) {
    throw new Error(`smoke-release: init refused the off configuration — ${e instanceof Error ? e.message : String(e)}`);
  }
  const reply = mod.dispatch("smoke@x.test", "GET", "/api/health", "", undefined, new Uint8Array());
  const { status, body } = reply;
  reply.free();
  if (status !== 200) throw new Error(`smoke-release: GET /api/health answered ${status}, not 200: ${body}`);
  return `smoke-release: the release module answered GET /api/health 200: ${body}`;
}

/** A browser's User-Agent: what a person's first request carries, so the
 *  answer read is the edge rule's, not one aimed at a bare client. */
export const BROWSER_UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";

/** The live mode's arguments — `{ origin, config }` — or `null` for none,
 *  the in-process smoke. Exactly `--live <origin> --config <file>`: the
 *  origin an `https` one with no path, query or fragment, the file a bare
 *  `wrangler<name>.jsonc` in this package (the deploy guard's rule). Anything
 *  else is refused rather than ignored. */
export function liveArgs(argv) {
  const args = argv.slice(2);
  if (args.length === 0) return null;
  if (!(args.length === 4 && args[0] === "--live" && args[2] === "--config" && /^wrangler[A-Za-z0-9.-]*\.jsonc$/.test(args[3]))) {
    throw new Error(
      `smoke-release: expected no arguments, or --live <origin> --config wrangler<name>.jsonc — got ${JSON.stringify(args)}`,
    );
  }
  let url;
  try {
    url = new URL(args[1]);
  } catch {
    url = undefined;
  }
  if (!url || url.protocol !== "https:" || url.pathname !== "/" || url.search !== "" || url.hash !== "" || url.username !== "") {
    throw new Error(
      `smoke-release: --live takes the deployment's https origin, with no path, query or fragment — got ${JSON.stringify(args[1])}`,
    );
  }
  return { origin: url.origin, config: args[3] };
}

/** `vars.YOKOHAMA_TURNSTILE` of the configuration `config` in the package at
 *  `root`: `on` or `off`, absent read as off — read as the deploy guard's
 *  check 12 reads it, so anything else is refused, never read as off. */
function declaration(root, config) {
  const file = join(root, config);
  if (!existsSync(file)) throw new Error(`smoke-release: ${config} is absent`);
  const errors = [];
  const value = parse(readFileSync(file, "utf8"), errors, { allowTrailingComma: true, disallowComments: false });
  if (errors.length > 0) {
    throw new Error(`smoke-release: ${config} does not parse: ${errors.map((e) => printParseErrorCode(e.error)).join(", ")}`);
  }
  const declared = value?.vars?.YOKOHAMA_TURNSTILE;
  if (declared === undefined || declared === "off") return "off";
  if (declared === "on") return "on";
  throw new Error(`smoke-release: ${config}'s vars.YOKOHAMA_TURNSTILE must be "on" or "off" (absent is off) — got ${JSON.stringify(declared)}`);
}

/** The edge line for the deployment at `origin` under the configuration
 *  `config` (in the package at `root`): a line saying what it read, or a
 *  throw naming why it fails. `fetchImpl` is the network; the cells pass a
 *  fake. */
export async function liveSmoke(origin, root, config, fetchImpl = fetch) {
  if (declaration(root, config) === "off") {
    return `smoke-release: ${config} declares YOKOHAMA_TURNSTILE off — no edge line is owed, and no request was made`;
  }
  const url = `${origin}/api/health`;
  const said = `smoke-release: ${config} declares YOKOHAMA_TURNSTILE on`;
  let response;
  try {
    // No cookie: fetch carries none of its own, so no clearance rides along.
    // `manual`: the first answer is the one read — an Access redirect is not a challenge.
    response = await fetchImpl(url, { method: "GET", redirect: "manual", headers: { "user-agent": BROWSER_UA } });
  } catch (e) {
    throw new Error(`${said}, but GET ${url} failed — ${e instanceof Error ? e.message : String(e)}`);
  }
  await response.body?.cancel();
  const mitigated = response.headers.get("cf-mitigated");
  const answered = `${response.status} ${mitigated === null ? "with no cf-mitigated header" : `with cf-mitigated: ${mitigated}`}`;
  if (response.status !== 403 || mitigated !== "challenge") {
    throw new Error(
      `${said}, but a cookie-less GET ${url} answered ${answered} — the edge rule ${JSON.stringify(DESCRIPTION)} is not ` +
        "challenging the API: create or enable it by its id (node scripts/edge-rule.mjs; worker/RUNBOOK.md §3)",
    );
  }
  return `${said}, and a cookie-less GET ${url} answered 403 cf-mitigated: challenge — the edge rule ${JSON.stringify(DESCRIPTION)} stands in front of the API`;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const live = liveArgs(process.argv);
    console.log(
      live === null
        ? await smokeRelease(fileURLToPath(new URL("../gen/", import.meta.url)))
        : await liveSmoke(live.origin, fileURLToPath(new URL("..", import.meta.url)), live.config),
    );
  } catch (e) {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(1);
  }
}
