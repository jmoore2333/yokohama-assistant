import { DurableObject } from "cloudflare:workers";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { Instance, pinnedTo, type Replied } from "./module";
import { executePlan, logOutcome, parsePlan, type Outcome, type Plan } from "./r2";
import { errorBody, log, respond } from "./respond";
import { USER_HEADER } from "./table";

/** Every archive object's name is this and the caller's address (spec §2.3):
 *  one object per caller. */
export const ARCHIVE_PREFIX = "archive:";

/** The archive object's trap 500, host-written in the table's shape (spec
 *  §2.4, "Traps"). */
const TRAP = "the archive's engine stopped — try again";

/** One `POST /api/archive`, as `fetch` read it — its body included. */
interface Incoming {
  user: string | undefined;
  method: string;
  path: string;
  query: string;
  contentType: string | undefined;
  body: Uint8Array;
}

/**
 * **The archive object** (spec §2.6): one object per caller, which answers
 * that caller's `POST /api/archive` — a browser's save.
 *
 * Its module's `init` configuration carries `caller`, the address the object
 * is named by. With the archive `r2`, the receipt archive reads that address
 * WHOLE to tell a test identity's save from the owner's (spec §2.6, the
 * `test/` root). It WRITES NOTHING of its own: it has no storage, and with the
 * archive `off` the layer's own answer is its 404 "this server keeps no
 * archive".
 *
 * **The receipt loop** (`save`), run WHOLE under `blockConcurrencyWhile`, so
 * one caller's saves go one at a time and no other event reaches the module
 * across the upload's await:
 *
 * 1. the save dispatched, pinned to P-2's reading, taken ONCE — the layer's
 *    strict replay and checks, then its put, deferred as a plan;
 * 2. with a plan, the upload (`executePlan`) and its outcome logged, then
 *    installed as the receipt and the SAME request dispatched AGAIN at the
 *    SAME reading. That answer is the answer: the layer's 201, its 200
 *    `identical`, its 409, or its refusal naming the archive with R2's
 *    reason inside it;
 * 3. with none — a save the layer refused before any put, or the archive
 *    off — the first answer is the answer.
 *
 * A browser's save costs its strict replay twice (spec §2.6). A throw inside
 * the loop — a receipt the module refused, thrown on (`dispatch`), or a plan
 * `parsePlan` refuses — or a loop running past 30 seconds RESETS this object
 * (Cloudflare's State API, for `blockConcurrencyWhile`). It holds nothing of
 * its own, so nothing is lost: the save is the caller's to send again.
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
    const incoming: Incoming = {
      user: req.headers.get(USER_HEADER) ?? undefined,
      method: req.method,
      path: url.pathname,
      query: url.search.slice(1),
      contentType: req.headers.get("content-type") ?? undefined,
      body: new Uint8Array(await req.arrayBuffer()),
    };
    const id = this.ctx.id.name;
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id, error: config.error });
      return respond(500, errorBody(config.error));
    }
    return this.ctx.blockConcurrencyWhile(() => this.save(id, incoming, config));
  }

  /** The receipt loop (the class's doc), under `blockConcurrencyWhile`. */
  private async save(id: string | undefined, incoming: Incoming, config: ShellConfig): Promise<Response> {
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, callerOf(id))));
    const at = nextReading(config.clock);
    const first = this.dispatch(id, instance, incoming, at);
    if (first instanceof Response) return first;
    if (first.archive === undefined) return respond(first.status, first.body);
    const plan = parsePlan(first.archive);
    // Only the receipt archive defers, and `shellConfig` refuses `r2` without
    // the bucket bound, so it is bound here.
    const outcome = await executePlan(this.env.ARCHIVE as R2Bucket, plan);
    logOutcome(id, "browser", plan, outcome);
    const again = this.dispatch(id, instance, incoming, at, { plan, outcome });
    return again instanceof Response ? again : respond(again.status, again.body);
  }

  /**
   * The request dispatched pinned to `at` (`pinnedTo`, which restores the
   * setting after it) — for the second dispatch, after `receipt`'s outcome is
   * installed as its plan's receipt — under ONE `guarded`. A trap is the
   * archive's trap 500. `init`'s refusal, thrown as its sentence
   * (`Instance.get`), is the 500 that carries it: only the first dispatch can
   * meet it, since the second runs on the module the first built, so a string
   * the second throws — a receipt the module refused — is thrown on.
   */
  private dispatch(
    id: string | undefined,
    instance: Instance,
    incoming: Incoming,
    at: number,
    receipt?: { plan: Plan; outcome: Outcome },
  ): Replied | Response {
    let done: { ok: Replied } | { trapped: string };
    // The call a trap is logged under, as the table names its stages.
    let stage = receipt === undefined ? "dispatch" : "receipt";
    try {
      done = instance.guarded((mod) => {
        if (receipt !== undefined) {
          mod.receipt(receipt.plan.key, JSON.stringify(receipt.outcome));
          stage = "dispatch";
        }
        return pinnedTo(mod, at, (m) =>
          m.dispatch(incoming.user, incoming.method, incoming.path, incoming.query, incoming.contentType, incoming.body),
        );
      });
    } catch (e) {
      if (typeof e !== "string" || receipt !== undefined) throw e;
      log("init-refused", { id, error: e });
      return respond(500, errorBody(e));
    }
    if ("trapped" in done) {
      log("trap", { id, call: stage, error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    return done.ok;
  }
}

/** The caller an archive object serves: its name less the prefix, or `null`
 *  for an object reached by no such name. */
function callerOf(name: string | undefined): string | null {
  return name?.startsWith(ARCHIVE_PREFIX) ? name.slice(ARCHIVE_PREFIX.length) : null;
}
