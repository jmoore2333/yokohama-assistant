import { DurableObject } from "cloudflare:workers";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { Instance, pinnedTo, readReply, type Replied } from "./module";
import { executePlan, logOutcome, parsePlan, type Outcome, type Plan } from "./r2";
import { callerDigest } from "./redact";
import { errorBody, log, respond } from "./respond";
import { forwardedCaller } from "./table";

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
 * 0. the caller's share counted (security S2b, B12's Worker half): the
 *    module's `share_prefix`, one R2 listing of it, and the listing handed to
 *    the module's `set_share`, whose put holds the save to the production
 *    quota before any plan is deferred. A listing that fails is logged
 *    (`archive-count-failed`) and not handed over, so the put refuses the
 *    save uncounted;
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
 * the loop — a listing `set_share` refused (thrown by `save`'s own `guarded`
 * call), a receipt the module refused (thrown on by `dispatch`), or a plan
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
      user: forwardedCaller(req.headers, this.config),
      method: req.method,
      path: url.pathname,
      query: url.search.slice(1),
      contentType: req.headers.get("content-type") ?? undefined,
      body: new Uint8Array(await req.arrayBuffer()),
    };
    // Every line this object logs names it by a digest: its name is `archive:<address>`, and a log persists
    // (security S2b, D23). The module still reads the whole name's caller.
    const name = this.ctx.id.name;
    const caller = callerOf(name);
    const id = caller === null ? name : ARCHIVE_PREFIX + (await callerDigest(caller));
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id, error: config.error });
      return respond(500, errorBody(config.error));
    }
    return this.ctx.blockConcurrencyWhile(() => this.save(name, id, incoming, config));
  }

  /** The receipt loop (the class's doc), under `blockConcurrencyWhile`: `name` is the object's own, which the
   *  module's configuration reads its caller from; `id` is what every line logs it as. */
  private async save(name: string | undefined, id: string | undefined, incoming: Incoming, config: ShellConfig): Promise<Response> {
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, callerOf(name))));
    const at = nextReading(config.clock);
    // **The share, counted live** (security S2b, B12's Worker half; decision D-5): the object's share prefix from
    // the module, one R2 listing (a Class A operation), and the listing handed to the module, whose put applies
    // the production quota's own arithmetic before any plan is deferred. A listing that fails is logged and NOT
    // handed over: the put then refuses the save uncounted (`SHARE_UNCOUNTED`), the layer's 500, which the outbox
    // retries. A `test/` address lists under `test/`; a server game's own save is never counted (its prefix is
    // empty, and `share_prefix` answers `undefined` for every object but an Archive object's).
    let prefix: { ok: string | undefined } | { trapped: string };
    try {
      prefix = instance.guarded((mod) => mod.share_prefix());
    } catch (e) {
      // `init` refused this object's caller (the key grammar's): nothing to count, and `dispatch` below refuses it
      // exactly as it always did — the layer's 401 through `callerRefused`, never a thrown string.
      if (typeof e !== "string") throw e;
      prefix = { ok: undefined };
    }
    if ("trapped" in prefix) {
      log("trap", { id, call: "share_prefix", error: prefix.trapped });
      return respond(500, errorBody(TRAP));
    }
    if (prefix.ok !== undefined) {
      let listed: R2Objects | undefined;
      try {
        listed = await (this.env.ARCHIVE as R2Bucket).list({ prefix: prefix.ok, limit: 1000 });
      } catch (e) {
        log("archive-count-failed", { id, error: String(e) });
      }
      if (listed !== undefined) {
        const listing = JSON.stringify({ truncated: listed.truncated, objects: listed.objects.map((o) => ({ key: o.key, size: o.size })) });
        const set = instance.guarded((mod) => mod.set_share(listing));
        if ("trapped" in set) {
          log("trap", { id, call: "set_share", error: set.trapped });
          return respond(500, errorBody(TRAP));
        }
      }
    }
    const first = this.dispatch(id, instance, incoming, at);
    if (first instanceof Response) return first;
    if (first.archive === undefined) return respond(first.status, first.body);
    const plan = parsePlan(first.archive);
    // Only the receipt archive defers, and `shellConfig` refuses `r2` without
    // the bucket bound, so it is bound here.
    const outcome = await executePlan(this.env.ARCHIVE as R2Bucket, plan);
    logOutcome(id, "browser", plan, outcome);
    // The same reading, as the table's second dispatch takes it (spec §2.6).
    // No cell can observe this pin: a browser's save puts the table it was
    // sent, which the layer does not stamp, so its plan carries no reading —
    // measured equivalent by the whole-branch review's battery (r17b). Were
    // the layer ever to stamp a browser's save, the browser-save cell would
    // have to run on a ticking clock to see it.
    const again = this.dispatch(id, instance, incoming, at, { plan, outcome });
    return again instanceof Response ? again : respond(again.status, again.body);
  }

  /**
   * The request dispatched pinned to `at` (`pinnedTo`, which restores the
   * setting after it) — for the second dispatch, after `receipt`'s outcome is
   * installed as its plan's receipt — under ONE `guarded`. A trap is the
   * archive's trap 500. `init`'s refusal, thrown as its sentence
   * (`Instance.get`), is the 500 that carries it — unless it was the CALLER
   * that `init` refused (`callerRefused`), which is the layer's 401: only the
   * first dispatch can meet it, since the second runs on the module the first
   * built, so a string the second throws — a receipt the module refused — is
   * thrown on.
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
      return this.callerRefused(incoming.user) ?? respond(500, errorBody(e));
    }
    if ("trapped" in done) {
      log("trap", { id, call: stage, error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    return done.ok;
  }

  /**
   * **A caller the door refuses is the layer's 401, as on the NAS** (M6.5c2
   * cycle D, straw poll row 8; R-C10 fixed). Under `open` a claim is held only
   * to what a header can carry (spec §2.1), so a claim the key grammar refuses
   * — `..` — still names an archive object, whose `init` refuses it as the
   * object's caller. The NAS refuses the same request before any route, with
   * the identity 401. So when `init` refuses, the caller is asked of a module
   * built with NO caller: the module's own `identity` answers the layer's
   * 401 body for that caller — the very one its `dispatch` would write — and
   * that is the answer. Anything else, a 200 among it, means `init` refused
   * for another reason, and its 500 stands. A trap, or a throw, while asking
   * is `undefined` too: the 500 is still the truthful answer.
   *
   * The module is built only on this refusal and thrown away after it.
   */
  private callerRefused(user: string | undefined): Response | undefined {
    try {
      const judge = new Instance(moduleConfig(this.env, null));
      const done = judge.guarded((mod) => readReply(mod.identity(user)));
      if ("trapped" in done || done.ok.status !== 401) return undefined;
      return respond(401, done.ok.body);
    } catch {
      return undefined;
    }
  }
}

/** The caller an archive object serves: its name less the prefix, or `null`
 *  for an object reached by no such name. */
function callerOf(name: string | undefined): string | null {
  return name?.startsWith(ARCHIVE_PREFIX) ? name.slice(ARCHIVE_PREFIX.length) : null;
}
