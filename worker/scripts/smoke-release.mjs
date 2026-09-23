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

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

/** `init`'s configuration: the archive off, no caller, the chat variables
 *  unset — the layer's defaults. */
export const SMOKE_CONFIG = JSON.stringify({ archive: "off", caller: null, lobby_chat: null, chat_keep: null, chat_text: null });

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

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    console.log(await smokeRelease(fileURLToPath(new URL("../gen/", import.meta.url))));
  } catch (e) {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(1);
  }
}
