// `env` from `cloudflare:workers` is typed `Cloudflare.Env`, which a project
// widens by redeclaring it (workers-types' own instruction). The pool's cells
// read the Worker's bindings through it, so it is the shell's own `Env`.
import type { Env as WorkerEnv } from "../src/env";

declare global {
  namespace Cloudflare {
    interface Env extends WorkerEnv {}
  }
}
