import { DurableObject } from "cloudflare:workers";
import type { Exports, Reply } from "../gen/factory.js";
import { guardAlarmName } from "./alarm";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { Instance, readReply, type Replied } from "./module";
import { deleteDoc, deleteMeta, ensureTables, getMeta, readDocs, setMeta, upsertDoc } from "./storage";

/** The caller, as the Worker resolved it (the plan's internal headers). */
export const USER_HEADER = "X-Yokohama-User";
/** A create's object name, percent-encoded (the plan's internal headers). */
export const TABLE_ID_HEADER = "X-Yokohama-Table-Id";

/** The trap 500's sentence, host-written (spec §2.4, §4). */
const TRAP = "the table's engine stopped — try again";

/** `Reply.picker` for a list the module cleared: the row is deleted (the
 *  orchestrator's ruling I-21). */
const CLEARED = "[]";

/** The layer's documents a table object stores, in the order the wake loads
 *  them (spec §2.4). The fourth `docs` kind, `picker`, is the host's own. */
const LAYER_DOCS = ["record", "table", "chat"] as const;
type LayerDoc = (typeof LAYER_DOCS)[number];

/** A module call's reply, and the reading its clock was pinned to (the plan's
 *  ruling P-2). */
export interface Pinned {
  reply: Replied;
  at: number;
}

/** The request's own call: its pinned reply, and each layer document read
 *  back out of the module after it — `undefined` for one the module does not
 *  hold. The map is empty where the store refuses the id (`readLayerDocs`). */
interface Called extends Pinned {
  docs: Map<LayerDoc, string | undefined>;
}

/**
 * What a request's batch writes beside the documents, from its wake (spec
 * §2.4): the restore's reply, whose picker row the request's own supersedes,
 * and the wake's own `meta`.
 */
export interface BatchParts {
  /** `restore_picker`'s reply, when this request woke a new module at an
   *  object that holds a record; else `null`. */
  restored: Pinned | null;
  /** `meta.wakes += 1`: this request woke a new module at an object that
   *  holds a table, with `YOKOHAMA_WEB_COUNT_WAKES` on. */
  countWake: boolean;
  /** This request's wake succeeded after a load trap: `load_traps` and
   *  `poison` are cleared. */
  clearTraps: boolean;
}

/** One request, as `fetch` read it — its body included. */
interface Incoming {
  id: string;
  user: string | undefined;
  method: string;
  path: string;
  query: string;
  contentType: string | undefined;
  create: boolean;
  body: Uint8Array;
}

/**
 * **The table object** (spec §2.4): one table's documents in its own SQLite,
 * and the table layer compiled to WebAssembly answering the table's requests —
 * every `/api/games/{id}…`, and the create that names it. Spec §2.3's Worker
 * reaches it by the table's id, so the object's name is the table's id
 * (`ctx.id.name`, the plan's pool fact 8), and `meta` stores none.
 */
export class Table extends DurableObject<Env> {
  /** The Worker's variables, read once — `shellConfig` reads `env` and
   *  nothing else. */
  private readonly config: ShellConfig | { error: string };
  /** The module (spec §2.2), built at the first request that needs it. */
  private instance: Instance | undefined;

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    // Nothing is written here: the first write batch creates the tables.
    this.config = shellConfig(env);
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const create = req.method === "POST" && url.pathname === "/api/games";
    const incoming: Incoming = {
      id: this.ctx.id.name ?? requestedId(url.pathname, req.headers, create),
      user: req.headers.get(USER_HEADER) ?? undefined,
      method: req.method,
      path: url.pathname,
      query: url.search.slice(1),
      contentType: req.headers.get("content-type") ?? undefined,
      create,
      // The body, read once: the LAST await before the reply. The wake, the
      // call and the batch all run after it, in `answer`'s one synchronous
      // turn, so no other request interleaves between a wake and the dispatch
      // it serves (spec §2.4; plan review m4).
      body: new Uint8Array(await req.arrayBuffer()),
    };
    return this.answer(incoming);
  }

  async alarm(): Promise<void> {
    return this.onAlarm(this.ctx.id.name);
  }

  /**
   * The alarm's work, for the name the platform handed `alarm()`. An alarm
   * with no name — `guardAlarmName` says when that happens — is logged and
   * touches nothing. Nothing in this object arms an alarm yet, so a named one
   * has nothing to do.
   */
  async onAlarm(name: string | undefined): Promise<void> {
    if (guardAlarmName(name) === "skip") {
      log("alarm-without-name");
      return;
    }
  }

  /**
   * The request path (spec §2.4; the plan's Task 9, steps 3 to 7), in one
   * synchronous turn: the poison check, the wake, the call, the ONE batch, and
   * the reply.
   */
  private answer(incoming: Incoming): Response {
    const { id } = incoming;
    const sql = this.ctx.storage.sql;
    // Two consecutive load traps poison the table: it is not loaded again, and
    // every request is the poison 500 naming the kind (spec §2.4 "Traps").
    const traps = Number(getMeta(sql, "load_traps") ?? "0");
    if (traps >= 2) {
      // The batch that counts a trap names its kind, so `poison` is set here;
      // the fallback `document` only satisfies the type.
      const kind = getMeta(sql, "poison") ?? "document";
      log("poison", { id, kind, traps });
      return respond(500, errorBody(`this table's stored ${kind} cannot be loaded`));
    }
    // A configuration the shell refuses builds no module. Spec §2.3 has the
    // Worker refuse it before it forwards; the object answers the same
    // refusal itself. Configuration, not data, so never a load trap (the
    // orchestrator's ruling I-29).
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id, error: config.error });
      return respond(500, errorBody(config.error));
    }
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, null)));
    try {
      const woken = this.wake(id, instance, config, traps);
      if (woken instanceof Response) return woken;
      const called = this.call(incoming, instance);
      if (called instanceof Response) return called;
      this.commit(called, woken);
      return respond(called.reply.status, called.reply.body);
    } catch (e) {
      // A throw that is not a trap — a document the module refuses to load, a
      // reply the shell cannot read, a batch that did not commit — can leave
      // the module holding what storage does not, and the object outlives a
      // throw. Dropped, the module is rebuilt from what IS stored, so nothing
      // the client saw fail is ever kept.
      instance.drop();
      throw e;
    }
  }

  /**
   * The wake (spec §2.4): a NEW module gets the object's stored documents and
   * its picker re-fed. Answers what the request's batch owes the wake, or the
   * reply that ends the request.
   *
   * - `get()` answers the module, building it and running `init` when there
   *   is none. `init`'s refusal, a sentence naming a variable, is the 500 that
   *   carries it (the orchestrator's ruling I-29); a trap while building is the
   *   trap 500. Neither is a LOAD trap: no stored document was read.
   * - Each stored `record`, `table` and `chat` row is `load`ed, as its exact
   *   text.
   * - Where a record is stored, `restore_picker(id, <picker row> ?? "null")`
   *   runs ONCE, pinned, before any dispatch (Task 4's wake contract). It
   *   builds the table's handle first — the replay — so a stored record that
   *   traps traps here. Its 200 has an empty body. A refusal that is not a
   *   trap is the layer's own error: logged, the row left as the module left
   *   it, and the request goes on to its own dispatch, whose answer stands —
   *   the same error, on a route that needs the replay.
   * - A trap in a load or in the restore is a LOAD trap: the module is
   *   dropped, `load_traps` counts it and `poison` names the kind, in a batch
   *   of their own, and the answer is the trap 500.
   */
  private wake(id: string, instance: Instance, config: ShellConfig, traps: number): BatchParts | Response {
    let built: { mod: Exports; fresh: boolean };
    try {
      built = instance.get();
    } catch (e) {
      if (typeof e === "string") {
        log("init-refused", { id, error: e });
        return respond(500, errorBody(e));
      }
      if (e instanceof WebAssembly.RuntimeError) {
        log("trap", { id, call: "init", error: String(e) });
        return respond(500, errorBody(TRAP));
      }
      throw e;
    }
    const { mod, fresh } = built;
    if (!fresh) return { restored: null, countWake: false, clearTraps: false };
    const sql = this.ctx.storage.sql;
    const stored = readDocs(sql);
    let kind: LayerDoc = "record";
    let restored: Pinned | null = null;
    try {
      for (const k of LAYER_DOCS) {
        const text = stored.get(k);
        if (text === undefined) continue;
        kind = k;
        mod.load(k, id, text);
      }
      if (stored.has("record")) {
        kind = "record";
        restored = this.pinned(mod, (m) => m.restore_picker(id, stored.get("picker") ?? "null"));
      }
    } catch (e) {
      if (!(e instanceof WebAssembly.RuntimeError)) throw e;
      instance.drop();
      const count = traps + 1;
      this.ctx.storage.transactionSync(() => {
        ensureTables(sql);
        setMeta(sql, "load_traps", String(count));
        setMeta(sql, "poison", kind);
      });
      log("load-trap", { id, kind, traps: count, error: String(e) });
      return respond(500, errorBody(TRAP));
    }
    if (restored !== null && restored.reply.status !== 200) {
      log("restore-refused", { id, status: restored.reply.status, body: restored.reply.body });
    }
    return { restored, countWake: config.countWakes && stored.has("table"), clearTraps: traps > 0 };
  }

  /**
   * The request's own call (the plan's Task 9, step 5): P-2's pin, then
   * `create` or `dispatch`, then the layer's three documents read back out of
   * the module — all under ONE `Instance.guarded`, so a trap in any of them
   * drops the module and answers the trap 500 with nothing committed: a move a
   * trapped call made was never acknowledged. The documents are read before
   * anything else is computed from the module (the orchestrator's ruling
   * I-42).
   */
  private call(incoming: Incoming, instance: Instance): Called | Response {
    const { id } = incoming;
    let stage = incoming.create ? "create" : "dispatch";
    const done = instance.guarded((mod) => {
      const pinned = this.pinned(mod, (m) =>
        incoming.create
          ? m.create(incoming.user, incoming.body, id, incoming.contentType)
          : m.dispatch(incoming.user, incoming.method, incoming.path, incoming.query, incoming.contentType, incoming.body),
      );
      stage = "read";
      return { ...pinned, docs: readLayerDocs(mod, id) };
    });
    if ("trapped" in done) {
      log("trap", { id, call: stage, error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    return done.ok;
  }

  /**
   * The request's ONE batch (the plan's Task 9, step 6), after EVERY call
   * whatever its method or status — a GET can pump a bot's owed move and
   * persist it:
   *
   * - each of `record`, `table` and `chat` that the module holds differently
   *   from its row is upserted, with `written_at` = `Date.now()` (the
   *   orchestrator's ruling I-43), and one it no longer holds is deleted;
   * - the `picker` row takes `Reply.picker` verbatim, the request's over the
   *   wake's, and `"[]"` deletes it (the orchestrator's ruling I-21);
   * - the wake's `wakes` count, and its clearing of a load trap.
   *
   * Every write goes in ONE `transactionSync`, which also creates the tables,
   * and a request with nothing to write runs none — so a probe creates
   * nothing.
   */
  private commit(called: Called, parts: BatchParts): void {
    const sql = this.ctx.storage.sql;
    const stored = readDocs(sql);
    const now = Date.now();
    const writes: Array<() => void> = [];
    for (const [kind, text] of called.docs) {
      if (text === stored.get(kind)) continue;
      writes.push(text === undefined ? () => deleteDoc(sql, kind) : () => upsertDoc(sql, kind, text, now));
    }
    const picker = called.reply.picker ?? parts.restored?.reply.picker;
    if (picker === CLEARED) {
      if (stored.has("picker")) writes.push(() => deleteDoc(sql, "picker"));
    } else if (picker !== undefined) {
      writes.push(() => upsertDoc(sql, "picker", picker, now));
    }
    if (parts.countWake) {
      writes.push(() => setMeta(sql, "wakes", String(Number(getMeta(sql, "wakes") ?? "0") + 1)));
    }
    if (parts.clearTraps) {
      writes.push(() => {
        deleteMeta(sql, "load_traps");
        deleteMeta(sql, "poison");
      });
    }
    if (writes.length === 0) return;
    this.ctx.storage.transactionSync(() => {
      ensureTables(sql);
      for (const write of writes) write();
    });
  }

  /** One module call pinned to P-2's reading, answered as plain data with
   *  that reading. */
  private pinned(mod: Exports, f: (mod: Exports) => Reply): Pinned {
    const at = this.reading();
    mod.set_clock(at);
    return { reply: readReply(f(mod)), at };
  }

  /** P-2's reading (`nextReading`): the module's time, in seconds. `answer`
   *  refuses a configuration the shell cannot read before any module call, so
   *  this never meets one. */
  private reading(): number {
    if ("error" in this.config) throw new Error(this.config.error);
    return nextReading(this.config.clock);
  }
}

/**
 * The id a request names — a create's `X-Yokohama-Table-Id`, percent-decoded,
 * or the path's segment after `/api/games/` — standing in for the object's
 * name only where the platform reports none. Measured in the pool for a name
 * over 1,024 bytes, which is what spec §2.3's routing makes of a path's
 * segment, or a create body's `id`, that long. The layer's ids are at most 64
 * bytes, so it answers its own refusal for the id, and nothing is written.
 */
function requestedId(path: string, headers: Headers, create: boolean): string {
  if (create) return decodeURIComponent(headers.get(TABLE_ID_HEADER) ?? "");
  return path.slice("/api/games/".length).split("/")[0] ?? "";
}

/**
 * The layer's three documents as the module holds them. `read` refuses one of
 * these kinds — it throws its sentence, where a trap throws a `RuntimeError` —
 * only for an id the store will not use: nothing can be stored under such an
 * id, so there is nothing to compare, and the map is empty.
 */
function readLayerDocs(mod: Exports, id: string): Map<LayerDoc, string | undefined> {
  const docs = new Map<LayerDoc, string | undefined>();
  for (const kind of LAYER_DOCS) {
    try {
      docs.set(kind, mod.read(kind, id));
    } catch (e) {
      if (typeof e !== "string") throw e;
      return new Map();
    }
  }
  return docs;
}

/** A reply as the plan's Task 9 writes it (step 7): a body is JSON, and an
 *  empty one is no body. */
function respond(status: number, body: string): Response {
  return new Response(body || null, { status, headers: body ? { "content-type": "application/json" } : {} });
}

/** The layer's error shape, for the refusals this object answers itself:
 *  the sentences spec §4 lists as host-written, and a configuration's. */
function errorBody(sentence: string): string {
  return JSON.stringify({ error: sentence });
}

/** One line of the shell's log (spec §2.8): what the layer cannot log on this
 *  host. */
function log(event: string, fields: Record<string, unknown> = {}): void {
  console.log(JSON.stringify({ event, ...fields }));
}
