import { DurableObject } from "cloudflare:workers";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { Instance, readReply, type Replied } from "./module";
import { errorBody, log, respond } from "./respond";
import { USER_HEADER } from "./table";

/** Every archive object's name is this and the caller's address (spec §2.3):
 *  one object per caller. */
export const ARCHIVE_PREFIX = "archive:";

/** The archive object's trap 500, host-written in the table's shape (spec
 *  §2.4, "Traps"). */
const TRAP = "the archive's engine stopped — try again";

/**
 * **The archive object, first cut** (spec §2.6; the plan's Task 11 — Task 13
 * brings the receipt loop and the bucket): one object per caller, which
 * answers that caller's `POST /api/archive`.
 *
 * Its module's `init` configuration carries `caller`, the address the object
 * is named by. Once the archive is `r2`, the receipt archive reads that
 * address WHOLE to tell a test identity's save from the owner's (spec §2.6,
 * the `test/` root). It WRITES NOTHING: it has no storage of its own, and with
 * the archive `off` the layer's own answer is its 404 "this server keeps no
 * archive".
 */
export class Archive extends DurableObject<Env> {
  /** The Worker's variables, read once. */
  private readonly config: ShellConfig | { error: string };
  /** The module (spec §2.2), built at the first request. It holds no
   *  document, so a new one needs nothing loaded. */
  private instance: Instance | undefined;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    this.config = shellConfig(env);
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const user = req.headers.get(USER_HEADER) ?? undefined;
    const contentType = req.headers.get("content-type") ?? undefined;
    const body = new Uint8Array(await req.arrayBuffer());
    const id = this.ctx.id.name;
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id, error: config.error });
      return respond(500, errorBody(config.error));
    }
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, callerOf(id))));
    let done: { ok: Replied } | { trapped: string };
    try {
      done = instance.guarded((mod) => {
        // P-2: the module's clock pinned before the call that can reach it.
        mod.set_clock(nextReading(config.clock));
        return readReply(mod.dispatch(user, req.method, url.pathname, url.search.slice(1), contentType, body));
      });
    } catch (e) {
      // `init`'s refusal, thrown as its sentence (`Instance.get`).
      if (typeof e !== "string") throw e;
      log("init-refused", { id, error: e });
      return respond(500, errorBody(e));
    }
    if ("trapped" in done) {
      log("trap", { id, call: "dispatch", error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    return respond(done.ok.status, done.ok.body);
  }
}

/** The caller an archive object serves: its name less the prefix, or `null`
 *  for an object reached by no such name. */
function callerOf(name: string | undefined): string | null {
  return name?.startsWith(ARCHIVE_PREFIX) ? name.slice(ARCHIVE_PREFIX.length) : null;
}
