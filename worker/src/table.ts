import { DurableObject } from "cloudflare:workers";
import type { Exports, Reply } from "../gen/factory.js";
import { adminRoute, OPERATOR_HEADER } from "./admin";
import { earliest, guardAlarmName } from "./alarm";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { CLAIMED_USER_HEADER } from "./identity";
import { expiryDays, expiryDeadline, markerEnd, openTtlDaysOf, ttlDaysOf, type Ending, type Marker } from "./marker";
import { Instance, readReply, type Bump, type Replied } from "./module";
import { errorBody, log, respond } from "./respond";
import { abandons } from "./route";
import { nextRetry, type Generation, type Slot } from "./slot";
import { deleteDoc, deleteMeta, ensureTables, forgetTables, getMeta, hasTables, readDocs, setMeta, upsertDoc } from "./storage";
import { archiveOwedOf, owing, withArchiving, type ArchiveWrite } from "./table/archiving";
import { copyOwedOf, copyRetryDue, differs, drawGeneration, lastActive, owedDue, slotState, withDelivery, type Owed } from "./table/delivery";
import { expiryFailedOf, poisoned, withLifecycle } from "./table/lifecycle";
import { clearLoadTraps, LAYER_DOCS, markerOf, readLayerDocs, TRAP, type LayerDoc } from "./table/shared";
import { isUpgrade, transportOf, withSockets, type Transport } from "./table/sockets";

/** `house.ts` and `index.ts` import it from here; `table/sockets.ts` holds it
 *  since the split (M6.5c2 cycle D, Task 9). */
export { isUpgrade } from "./table/sockets";

/** The caller, as the Worker resolved it (the plan's internal headers). */
export const USER_HEADER = "X-Yokohama-User";

/**
 * The caller an object was handed (the plan's internal headers; M6.5c2, spec
 * §2.1): the ONE header its door names, and no other — `X-Yokohama-User`, a
 * caller the Worker verified, under `worker`; `X-Yokohama-Claimed-User`, a name
 * a caller claimed, under `open`. The Worker strips every client
 * `x-yokohama-*` header and sets its door's alone, so the two agree while the
 * Worker and the object read one configuration — and were they ever to
 * disagree, a claim would reach no caller rather than pass as a verified one.
 * With a configuration the shell refuses there is no door and no caller: the
 * object answers that refusal's 500 before it reads one.
 */
export function forwardedCaller(headers: Headers, config: ShellConfig | { error: string }): string | undefined {
  if ("error" in config) return undefined;
  return headers.get(config.door === "open" ? CLAIMED_USER_HEADER : USER_HEADER) ?? undefined;
}
/** A create's object name, percent-encoded (the plan's internal headers). */
export const TABLE_ID_HEADER = "X-Yokohama-Table-Id";

/** `Reply.picker` for a list the module cleared: the row is deleted (the
 *  orchestrator's ruling I-21). */
const CLEARED = "[]";

/** The 426 a `GET …/stream` that is not an upgrade answers (spec §2.3;
 *  host-written, §4) — and the house's `/api/stream`'s too (the
 *  orchestrator's ruling I-48). */
export const NOT_AN_UPGRADE = "this host streams over WebSocket";

/** A module call's reply, and the reading its clock was pinned to (the plan's
 *  ruling P-2). */
export interface Pinned {
  reply: Replied;
  at: number;
}

/** The request's own call: its pinned reply, and each layer document read
 *  back out of the module after it — `undefined` for one the module does not
 *  hold. The map is empty where the store refuses the id (`readLayerDocs`). */
export interface Called extends Pinned {
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
   *  `poison` are cleared — by the commit, or, when the call then traps, by a
   *  batch that holds only that. */
  clearTraps: boolean;
}

/** One request, as `fetch` read it — its body included. */
export interface Incoming {
  id: string;
  user: string | undefined;
  method: string;
  path: string;
  query: string;
  contentType: string | undefined;
  create: boolean;
  /** `…/events` (a GET) or `…/stream` (an upgrade), answered through the
   *  module's `events`; absent on every other request. */
  transport?: Transport;
  body: Uint8Array;
}

/**
 * A constructor of the table object's base, or of the base with some of its
 * parts mixed in: each mixin (`withDelivery`, `withArchiving`, `withLifecycle`,
 * `withSockets`) takes one and answers a subclass of it. The rest parameter of
 * `any[]` is TypeScript's own requirement of a mixin's base (TS2545), and
 * `abstract` lets it be `TableBase`, which declares what the parts add.
 */
export type TableParts = abstract new (...args: any[]) => TableBase;

/**
 * **The table object's base** (M6.5c2 cycle D, Task 9): its fields and its
 * constructor, the request path — `fetch`, the wake, the call, the ONE batch
 * and what it owes — and the alarm with its deadlines. The rest is mixed in
 * over it, one part a file under `src/table/`: the slot's delivery
 * (`withDelivery`), the owed save's (`withArchiving`), the deletes, the expiry
 * and the operator (`withLifecycle`), and the sockets (`withSockets`) — each a
 * class whose methods stay methods, on its own prototype in `Table`'s chain,
 * because the pool patches prototypes (`src/table.unit.test.ts`).
 */
export abstract class TableBase extends DurableObject<Env> {
  /** The Worker's variables, read once — `shellConfig` reads `env` and
   *  nothing else. */
  protected readonly config: ShellConfig | { error: string };
  /** The module (spec §2.2), built at the first request that needs it. */
  protected instance: Instance | undefined;
  /** The house pushes in flight, by `pushKey`: `deliverSlot` hands a second
   *  caller of the same slot the push already under way, so one slot is
   *  never pushed twice at once, and a caller that awaits it — the corpus
   *  replay's flush, a test's `settle` — waits for its acknowledgement. */
  protected readonly pushes = new Map<string, Promise<void>>();
  /** The owed saves' uploads in flight, by their plan's key:
   *  `deliverArchive` hands a second caller of the same owed save the upload
   *  already under way, so one plan is never uploaded twice at once, and a
   *  caller that awaits it — a test's `settle` — waits for what it settles. */
  protected readonly uploads = new Map<string, Promise<void>>();

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    // Nothing is written here: the first write batch creates the tables.
    this.config = shellConfig(env);
    // The client's keep-alive (spec §2.7): every `ping` a socket sends is
    // answered `pong` by the platform, and never reaches `webSocketMessage`.
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  // **What the parts add that another part calls**, declared here so that
  // each calls every other through `this`: the request path and the alarm
  // call into all four parts, and the parts into the base and into one
  // another. Each is documented where it is implemented. A part that stopped
  // implementing one leaves `Table` uncompilable (TS2654); a declaration here
  // emits nothing, so each method is still defined ONCE on the chain.

  /** The slot's ONE delivery routine (`withDelivery`, `table/delivery.ts`). */
  abstract deliverSlot(): Promise<void>;
  /** The alarm's house copy, recomputed from the stored table (`withDelivery`). */
  protected abstract recopy(id: string): void;
  /** The owed save's ONE delivery routine (`withArchiving`, `table/archiving.ts`). */
  protected abstract deliverArchive(how: { fromRequest: boolean }): Promise<void>;
  /** Whether saves are uploaded (`withArchiving`). */
  protected abstract archiveOn(): boolean;
  /** An owed save's retry, from now (`withArchiving`). */
  protected abstract archiveRung(sql: SqlStorage): number | undefined;
  /** The answer an ENDED table gives (`withLifecycle`, `table/lifecycle.ts`). */
  protected abstract ended(create: boolean): Response | undefined;
  /** The table's own DELETE (`withLifecycle`). */
  protected abstract abandon(incoming: Incoming): Promise<Response>;
  /** The alarm's expiry (`withLifecycle`). */
  protected abstract expiry(id: string): Promise<void>;
  /** An operator's expire (`withLifecycle`). */
  protected abstract operate(id: string, address: string, force: boolean): Promise<Response>;
  /** An upgrade's answer (`withSockets`, `table/sockets.ts`). */
  protected abstract admit(id: string, user: string | undefined, reply: Replied): Response;
  /** The frames a request owes its sockets (`withSockets`). */
  abstract sendFrames(id: string, bumps: Bump[]): void;
  /** A force's end of every socket (`withSockets`). */
  protected abstract endSockets(id: string): void;

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    // An operator's expire (spec §2.9), past the Worker's gate: the header is
    // the gate's word, since the Worker removes a client's. It is answered
    // before the marker's 404, because a force ends a marker too.
    const operator = req.headers.get(OPERATOR_HEADER);
    const admin = operator === null ? undefined : adminRoute(req.method, url.pathname, url.search.slice(1));
    if (operator !== null && admin?.kind === "expire") return this.operate(this.ctx.id.name ?? admin.name, operator, admin.force);
    const create = req.method === "POST" && url.pathname === "/api/games";
    const ended = this.ended(create);
    if (ended !== undefined) return ended;
    const transport = transportOf(url.pathname);
    // The transport routes' own refusals (spec §2.3), answered before the
    // table is read, as a router answers them. A non-`GET` is 405 with an
    // empty body, the layer's own 405 shape. (axum's `get` serves a HEAD
    // through the GET handler; the spec rules 405 for every non-`GET`, and
    // no client sends a HEAD here.)
    if (transport !== undefined && req.method !== "GET") return respond(405, "");
    if (transport === "stream" && !isUpgrade(req.headers)) return respond(426, errorBody(NOT_AN_UPGRADE));
    const incoming: Incoming = {
      id: this.ctx.id.name ?? requestedId(url.pathname, req.headers, create),
      user: forwardedCaller(req.headers, this.config),
      method: req.method,
      path: url.pathname,
      query: url.search.slice(1),
      contentType: req.headers.get("content-type") ?? undefined,
      create,
      transport,
      // The body, read once: the LAST await before the reply but for an
      // abandon's upload, which `blockConcurrencyWhile` shields (`abandon`).
      // The wake, the call and the batch all run after it, in `answer`'s one
      // synchronous turn, so no other request interleaves between a wake and
      // the dispatch it serves (spec §2.4; plan review m4).
      body: new Uint8Array(await req.arrayBuffer()),
    };
    // An abandon runs WHOLE under `blockConcurrencyWhile` (spec §2.4 step 1,
    // §2.6): its save is uploaded between its two dispatches, and nothing else
    // may reach this object in between. A delete that owes no save completes
    // in its first dispatch, under it all the same. **The table's own path
    // only** (M6.5c2 cycle D, R-D4): a chat message's DELETE is an ordinary
    // request (`abandons`).
    if (abandons(incoming.method, incoming.path)) return this.ctx.blockConcurrencyWhile(() => this.abandon(incoming));
    return this.answer(incoming);
  }

  async alarm(): Promise<void> {
    return this.onAlarm(this.ctx.id.name);
  }

  /**
   * The alarm's work, for the name the platform handed `alarm()` (spec §2.4:
   * ONE alarm per object, set to the earliest thing due). An alarm with no
   * name — `guardAlarmName` says when that happens — is logged and touches
   * nothing. A named one, while a MARKER stands, is the marker's
   * (`markerAlarm`). Otherwise it:
   *
   * 1. recomputes the house copy from the STORED table document (`recopy`),
   *    which is how a copy that trapped in its request is delivered (the
   *    orchestrator's ruling I-42);
   * 2. delivers the slot (`deliverSlot`) — a retry after a failed push, or a
   *    push an abort cut off — and, beside it, an owed save
   *    (`deliverArchive`): a game over whose upload failed, or was cut off.
   *    With the archive `off`, the owed save is kept and logged;
   * 3. expires the table once its last activity is a TTL old — or, while it
   *    still waits for players, the waiting-table TTL old (R-6.5c2.42), which
   *    its status decides here — or re-reads that activity and re-arms to the
   *    table's next deadline (`expiry`).
   *
   * The batches the first two write re-arm the alarm — a new slot's, a failed
   * push's, an acknowledgement's, a failed upload's, a settled save's — to the
   * earliest thing due. The third re-arms it to the table's next deadline, or
   * arms nothing for the expiry of a table that may not expire (`held`,
   * poisoned). With no TTL, a run with nothing due leaves none.
   */
  async onAlarm(name: string | undefined): Promise<void> {
    if (guardAlarmName(name) === "skip") {
      log("alarm-without-name");
      return;
    }
    // The guard runs only a named alarm.
    const id = name as string;
    const marker = markerOf(this.ctx.storage.sql);
    if (marker !== undefined) return this.markerAlarm(marker);
    this.recopy(id);
    await Promise.all([this.deliverSlot(), this.deliverArchive({ fromRequest: false })]);
    await this.expiry(id);
  }

  /**
   * The alarm while a MARKER stands (spec §2.4 steps 4 and 6): a REMOVAL that
   * still waits is pushed, and its acknowledgement either ends the object —
   * the marker's 90 days are over — or re-arms the alarm to their end
   * (`acknowledged`); a failed push re-arms to its retry (`failed`). With the
   * removal acknowledged, the object ends at the marker's end, and an alarm
   * that runs before it re-arms to it.
   */
  private async markerAlarm(marker: Marker): Promise<void> {
    if (slotState(this.ctx.storage.sql).slot !== undefined) return this.deliverSlot();
    if (Date.now() < markerEnd(marker)) {
      this.arm(markerEnd(marker));
      return;
    }
    await this.end(marker);
  }

  /**
   * The object ends (spec §2.4 step 6; spec §2.9's force): its module dropped,
   * `deleteAll()`, which at this compatibility date deletes its alarm too, and
   * the tables forgotten (`forgetTables`), so the next read scans for them and
   * finds none. Logged:
   *
   * - `marker-ended` when it ends a marker;
   * - `forced` when an operator's force ended it: at once, on its immediate
   *   path, which writes no batch and passes `forced`; or through the REMOVAL
   *   its batch flagged (`meta.forced`), once acknowledged;
   * - `removal-ended` for a REMOVAL with no marker and no flag (Task 14's L-4):
   *   one the code before the marker wrote (the fifteenth and sixteenth
   *   deployments), or one a force made before the flag (from the seventeenth
   *   deployment until the one that carries it), acknowledged since. Every
   *   delete through the layer writes its marker in the batch that writes its
   *   REMOVAL, and a force flags its own, so no code since leaves such a
   *   removal, and every `forced` line follows its force's `operator` line.
   *
   * An owed save it drops is logged `archive-owed-dropped` first, once
   * `deleteAll()` has dropped it — only a force's immediate path meets one
   * here, since a delete's batch and a force's clear it — so a force logs the
   * two in ONE order on both its paths.
   */
  protected async end(marker: Marker | undefined, forced = false): Promise<void> {
    const id = this.ctx.id.name;
    const sql = this.ctx.storage.sql;
    const owed = archiveOwedOf(sql);
    const flagged = forced || getMeta(sql, "forced") !== undefined;
    this.instance?.drop();
    await this.ctx.storage.deleteAll();
    forgetTables(sql);
    if (owed !== undefined) log("archive-owed-dropped", { id, record_key: owed.plan.record_key, attempt: owed.attempt });
    if (marker !== undefined) log("marker-ended", { id, reason: marker.reason, at: marker.at, kept: marker.kept });
    else if (flagged) log("forced", { id });
    else log("removal-ended", { id });
  }

  /**
   * What a request needs before its wake, or the answer that ends it first:
   * the poison check and the configuration — then the module (spec §2.4).
   */
  protected ready(id: string): { config: ShellConfig; instance: Instance; traps: number } | Response {
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
    // A configuration the shell refuses builds no module. Spec §2.6 has every
    // `/api/*` answer 500 naming the variable, and the plan's Task 11 has the
    // Worker do so (its step 2) before it forwards; the object answers the
    // same refusal itself. Configuration, not data, so never a load trap (the
    // orchestrator's ruling I-29).
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id, error: config.error });
      return respond(500, errorBody(config.error));
    }
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, null)));
    return { config, instance, traps };
  }

  /**
   * The request path (spec §2.4; the plan's Task 9, steps 3 to 7, and Task
   * 10) for every request but the table's own DELETE (`abandon`), in one synchronous turn:
   * the poison check, the wake, the call, the ONE batch, the frames, and the
   * reply — or, for a `…/stream` upgrade, the admission (`admit`). A plan the
   * call deferred is game over's, and so is one the wake's `restore_picker`
   * deferred: neither call is run again, so the batch OWES it (`owing`; the
   * plan's ruling P-2), and the save is uploaded after the reply.
   */
  private answer(incoming: Incoming): Response {
    const ready = this.ready(incoming.id);
    if (ready instanceof Response) return ready;
    const { id } = incoming;
    const { config, instance, traps } = ready;
    let reply: Replied;
    let bumps: Bump[];
    let done: Committed;
    try {
      const woken = this.wake(id, instance, config, traps);
      if (woken instanceof Response) return woken;
      const called = this.call(incoming, instance);
      if (called instanceof Response) {
        // The call trapped, so it persists nothing. But the wake's load
        // succeeded, and spec §2.4 resets the count on ANY successful load:
        // the reset is written alone, as the request's one batch (the
        // orchestrator's ruling, Task 9 fix round 1).
        if (woken.clearTraps) this.batch(clearLoadTraps);
        return called;
      }
      done = this.commit(id, called, woken, owing(called) ?? owing(woken.restored));
      reply = called.reply;
      // Every bump the request's calls made: the wake's — `restore_picker`'s,
      // a cold rebuild's owed bot move among them — and the call's own (the
      // orchestrator's ruling, Task 9's review).
      bumps = [...(woken.restored?.reply.bumps ?? []), ...reply.bumps];
    } catch (e) {
      // A throw that is not a trap — a document the module refuses to load, a
      // reply the shell cannot read, a batch that did not commit — can leave
      // the module holding what storage does not, and the object outlives a
      // throw. Dropped, the module is rebuilt from what IS stored, so nothing
      // the client saw fail is ever kept.
      instance.drop();
      throw e;
    }
    return this.finish(incoming, reply, bumps, done);
  }

  /**
   * The side channels, after the request's batch (spec §2.4), and the reply:
   * the output gate holds every send until the batch is durable (spec §0.4),
   * and `sendFrames` never throws, so nothing it meets costs the request its
   * reply. Then the house push, when the batch wrote a slot, and the upload of
   * a save the table owes: each outlives the reply (`waitUntil`), and the alarm
   * armed beside the batch that made it due finishes it if it cannot.
   */
  protected finish(incoming: Incoming, reply: Replied, bumps: Bump[], done: Committed): Response {
    const { id } = incoming;
    this.sendFrames(id, bumps);
    if (done.pushing) this.ctx.waitUntil(this.deliverSlot());
    if (done.archiving) this.ctx.waitUntil(this.deliverArchive({ fromRequest: true }));
    return incoming.transport === "stream" ? this.admit(id, incoming.user, reply) : respond(reply.status, reply.body);
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
   *   the same error, on a route that needs the replay. A row it cleared for
   *   its CODE — built under code other than this build's (the orchestrator's
   *   ruling I-85) — is deleted by the request's batch, which logs
   *   `picker-dropped {id, stored, running}` once it has committed (`commit`):
   *   the row's code (`null` for a row that names none) and this build's. A
   *   request whose call then traps commits no such batch, so it logs no
   *   drop, the row stays, and the next wake drops it and logs it (R1's N-2).
   * - A trap in a load or in the restore is a LOAD trap: the module is
   *   dropped, `load_traps` counts it and `poison` names the kind, in a batch
   *   of their own, and the answer is the trap 500.
   */
  protected wake(id: string, instance: Instance, config: ShellConfig, traps: number): BatchParts | Response {
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
      this.batch((sql) => {
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
   * `create`, `dispatch` or — for a transport route — `events`, then the
   * layer's three documents read back out of the module — all under ONE
   * `Instance.guarded`, so a trap in any of them drops the module and answers
   * the trap 500 with nothing committed: a move a trapped call made was never
   * acknowledged. The documents are read before anything else is computed
   * from the module (the orchestrator's ruling I-42).
   *
   * `…/events` hands `events` its query; `…/stream` reads none (axum's
   * `stream` handler takes no query), so its admission is always
   * `events(id, user, "")` (spec §2.4).
   */
  private call(incoming: Incoming, instance: Instance): Called | Response {
    const { id, transport } = incoming;
    let stage = incoming.create ? "create" : transport === undefined ? "dispatch" : "events";
    const done = instance.guarded((mod) => {
      const pinned = this.pinned(mod, (m) =>
        incoming.create
          ? m.create(incoming.user, incoming.body, id, incoming.contentType)
          : transport === undefined
            ? m.dispatch(incoming.user, incoming.method, incoming.path, incoming.query, incoming.contentType, incoming.body)
            : m.events(id, incoming.user, transport === "events" ? incoming.query : ""),
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
   *   wake's, and `"[]"` deletes it (the orchestrator's ruling I-21) — which
   *   is how a delete takes an open picker's row with it: the module answers
   *   `"[]"` for a table it deletes with a list open (Task 4). A row the
   *   wake's restore dropped for its code (`Reply.dropped`) is logged
   *   `picker-dropped {id, stored, running}` once the batch has committed, as
   *   the `wake` line is (R1's N-2);
   * - the wake's `wakes` count — logged `wake {id, wakes}`, the new count,
   *   once the batch has committed (the orchestrator's ruling I-84: the one
   *   place the count can be read off a live Worker) — and its clearing of a
   *   load trap;
   * - **what the house is owed** (spec §2.5, `owed`), when the table
   *   document changed or a copy is still owed (`copy_owed`): the table's
   *   first generation, and the slot with its revision — the new copy, or a
   *   REMOVAL when the layer deleted the table — with its alarm at +30 s set
   *   beside the batch. A copy `house_copy` traps on is counted in
   *   `copy_owed` and arms the alarm at its rung, for a fresh module to
   *   recompute (the orchestrator's ruling I-42);
   * - **what the archive is owed** (spec §2.6), `archive`: a plan written to
   *   `archive_owed` with its retry's alarm, or a save discharged — cleared,
   *   and the alarm re-armed to what is still due (`dueBesidesArchive`), or
   *   deleted when nothing is. A delete the layer made clears it whatever
   *   `archive` says (spec §2.4 step 2; the orchestrator's ruling I-57): with
   *   the archive on, the delete's own save superseded it, and with the
   *   archive off, the owed save goes with the table, logged
   *   `archive-owed-dropped` after the batch. Any other alarm this batch sets
   *   is brought forward to an owed save's retry when that is sooner
   *   (`earliest`);
   * - **the MARKER** (spec §2.4 step 2), when the layer deleted the table:
   *   `ending` at `Date.now()` (the orchestrator's rulings I-43/I-44), with
   *   `held` and `expiry_failed` gone with the table — beside the REMOVAL and
   *   its alarm, and the cleared `archive_owed`, so the whole ending is this
   *   ONE batch. Every delete comes through a DELETE — the creator's, or an
   *   expiry's as the creator — whose caller passes its `ending`; the layer
   *   deletes a table on no other route.
   *
   * Every write goes in ONE `batch`, and a request with nothing to write runs
   * none — so a probe creates nothing. Answers whether the batch wrote a
   * slot, which the request then pushes; whether a save is owed after it,
   * with the switch on, which the request then hands to `deliverArchive` —
   * one this batch wrote, or one owed before it: the next request past its
   * gate resumes it (spec §2.6; R-D6); and the marker it wrote.
   */
  protected commit(id: string, called: Called, parts: BatchParts, archive?: ArchiveWrite, ending?: Ending): Committed {
    const sql = this.ctx.storage.sql;
    const stored = readDocs(sql);
    const now = Date.now();
    const writes: Array<() => void> = [];
    for (const [kind, text] of called.docs) {
      if (text === stored.get(kind)) continue;
      writes.push(text === undefined ? () => deleteDoc(sql, kind) : () => upsertDoc(sql, kind, text, now));
    }
    // An empty map is an id the store refused (`readLayerDocs`): nothing is
    // stored under it, so nothing changed.
    const before = stored.get("table");
    const after = called.docs.size === 0 ? before : called.docs.get("table");
    const deleted = before !== undefined && after === undefined;
    const picker = called.reply.picker ?? parts.restored?.reply.picker;
    if (picker === CLEARED) {
      if (stored.has("picker")) writes.push(() => deleteDoc(sql, "picker"));
    } else if (picker !== undefined) {
      writes.push(() => upsertDoc(sql, "picker", picker, now));
    }
    const wakes = parts.countWake ? Number(getMeta(sql, "wakes") ?? "0") + 1 : undefined;
    if (wakes !== undefined) writes.push(() => setMeta(sql, "wakes", String(wakes)));
    if (parts.clearTraps) writes.push(() => clearLoadTraps(sql));
    // Computed BEFORE the batch, from the documents already read out of the
    // module (the orchestrator's ruling I-42), and written LAST in it, so the
    // slot's size and last activity count the documents this batch writes.
    // A copy still owed is computed by every request until it is made (spec
    // §2.4: a side channel's trap "retries on the alarm or the next request").
    const owed = after !== before || copyOwedOf(sql) > 0 ? this.owed(id, after) : undefined;
    if (owed !== undefined) writes.push(...this.owedWrites(id, owed));
    // The owed save as it stood before this batch.
    const holding = archiveOwedOf(sql);
    const marker: Marker | undefined = deleted && ending !== undefined ? { at: now, ...ending } : undefined;
    if (marker !== undefined) {
      writes.push(() => setMeta(sql, "marker", JSON.stringify(marker)));
      if (getMeta(sql, "held") !== undefined) writes.push(() => deleteMeta(sql, "held"));
      if (expiryFailedOf(sql) > 0) writes.push(() => deleteMeta(sql, "expiry_failed"));
    }
    let alarm: number | null | undefined = owed === undefined ? undefined : owedDue(owed);
    let owes = holding !== undefined;
    if (deleted || archive === "clear") {
      owes = false;
      if (holding !== undefined) {
        writes.push(() => deleteMeta(sql, "archive_owed"));
        alarm = earliest(alarm ?? null, this.dueBesidesArchive(sql) ?? undefined);
      }
    } else if (archive !== undefined) {
      owes = true;
      writes.push(() => setMeta(sql, "archive_owed", JSON.stringify(archive.owe)));
      alarm = earliest(archive.due, alarm ?? undefined);
    } else {
      alarm = earliest(alarm, this.archiveRung(sql));
    }
    // A save owed after this batch goes to `deliverArchive` from the request,
    // which returns at once while the last failure's gate (`not_before`) has
    // not passed (R-D6). So while R2 is down, a request pays a read of the
    // owed row there and nothing else: no upload and no failure batch — one
    // attempt per rung, the alarm's or the first request's after it. Before
    // R-D6 every request re-attempted, and each failure climbed the ladder.
    const archiving = owes && this.archiveOn();
    if (writes.length === 0) return { pushing: false, archiving, marker };
    this.batch(() => {
      for (const write of writes) write();
    }, alarm);
    if (wakes !== undefined) log("wake", { id, wakes });
    // The module's own JSON, `{"stored", "running"}`: two codes, and no record
    // or table document. A drop always has a write — the row it cleared, or
    // the request's own row over it — so it never meets the early return.
    const dropped = parts.restored?.reply.dropped;
    if (dropped !== undefined) {
      log("picker-dropped", { id, ...(JSON.parse(dropped) as { stored: string | null; running: string }) });
    }
    // A delete with the archive off made no save of its own, so the owed save
    // went with the table: logged once the batch that dropped it has committed
    // (spec §2.6). With the archive on, the delete's own save superseded it.
    if (deleted && holding !== undefined && !this.archiveOn()) {
      log("archive-owed-dropped", { id, record_key: holding.plan.record_key, attempt: holding.attempt });
    }
    return { pushing: owed?.slot !== undefined, archiving, marker };
  }

  /**
   * What a change of the table document owes the house (spec §2.5), decided
   * before any write: `table` is the document the change left — `undefined`
   * when the layer deleted the table.
   *
   * - **The generation** is drawn when `meta` holds none (`drawGeneration`):
   *   `Date.now()` in MILLISECONDS — the shell's own time base (the
   *   orchestrator's rulings I-43/I-44), never P-2's reading in seconds — and
   *   64 random bits, for every table this shell creates by the create's own
   *   batch. The house lists a re-created table only when its generation
   *   orders after the one it retired, so a table forced and created again
   *   within one SECOND is listed every time, where the second's reading tied
   *   the two and the random half decided (the orchestrator's F-2); only a
   *   re-create in the same millisecond as the original's create still
   *   ties.
   * - **The copy** is `house_copy` of the document (`copyOf`), or `null`, a
   *   REMOVAL. One `house_copy` refuses is logged — its sentence VERBATIM —
   *   and owes nothing: the house keeps listing the last copy it received, a
   *   state only a corrupted store reaches, whose path is the operator's force
   *   (Task 5's escalation 1, ruled). One that TRAPS — or fails in any other
   *   way than a refusal (`copyOf`) — is logged, owes no slot, and is still
   *   owed: `traps` counts it on the `copy_owed` before it. Any other outcome
   *   owes no copy, and `traps` is 0.
   * - **The slot** is owed when the copy DIFFERS from the one last put in
   *   motion (`differs`), at the next revision.
   */
  protected owed(id: string, table: string | undefined): Owed {
    const sql = this.ctx.storage.sql;
    const state = slotState(sql);
    const draw = state.generation === undefined ? drawGeneration(Date.now()) : undefined;
    const generation = state.generation ?? (draw as Generation);
    let copy: string | null = null;
    if (table !== undefined) {
      const made = this.copyOf(id, table);
      if ("refused" in made) {
        log("copy-refused", { id, error: made.refused });
        return { draw, traps: 0 };
      }
      if ("trapped" in made) {
        log("trap", { id, call: "house_copy", error: made.trapped });
        return { draw, traps: copyOwedOf(sql) + 1 };
      }
      // `house_copy` answers `undefined` only where no table is stored, and
      // the module it ran on was handed one.
      if (made.copy === undefined) return { draw, traps: 0 };
      copy = made.copy;
    }
    if (!differs(state, copy)) return { draw, traps: 0 };
    return { draw, slot: { generation, revision: state.revision + 1, copy }, traps: 0 };
  }

  /** The writes `owed` asks of a batch: its generation; `copy_owed` — the
   *  new count while the copy is owed, deleted once it is not; then its slot
   *  and revision, the slot's last activity and size read INSIDE the batch,
   *  after every document write before it. */
  protected owedWrites(id: string, owed: Owed): Array<() => void> {
    const sql = this.ctx.storage.sql;
    const writes: Array<() => void> = [];
    const { draw, slot: next, traps } = owed;
    if (draw !== undefined) writes.push(() => setMeta(sql, "generation", JSON.stringify(draw)));
    if (traps > 0) writes.push(() => setMeta(sql, "copy_owed", String(traps)));
    else if (copyOwedOf(sql) > 0) writes.push(() => deleteMeta(sql, "copy_owed"));
    if (next !== undefined) {
      writes.push(() => {
        const slot: Slot = {
          id,
          generation: next.generation,
          revision: next.revision,
          copy: next.copy,
          last_active: lastActive(sql) ?? Date.now(),
          bytes: sql.databaseSize,
          attempt: 0,
        };
        setMeta(sql, "revision", String(next.revision));
        setMeta(sql, "slot", JSON.stringify(slot));
      });
    }
    return writes;
  }

  /**
   * `house_copy` over `table`, on a SECOND module instance built for this one
   * call and holding only that document (the orchestrator's ruling I-42, as
   * adapted in Task 12). A trap there discards THAT instance, so this
   * object's own — which the request's frames read next — is never emptied
   * by it: an emptied module reads this table as a 404, which would end every
   * live socket (`sendFrames`). Every throw but a refusal answers `trapped`,
   * building the instance included: the copy is a side channel, and must
   * never cost the request its reply.
   *
   * **What the second instance costs**, and when:
   *
   * - **When it is made**: only when a copy must be computed — a request
   *   whose batch changes the table document (such as a create, a join, a
   *   keep, an unlock, the turn passing or game over; not a delete, whose
   *   slot is the REMOVAL) and an alarm run that finds a table document
   *   (`recopy`) — and, at an alarm past the deadline an expiry is armed at,
   *   once more for the table's status (`waiting`, R-6.5c2.42). A move that
   *   leaves the table document alone makes none.
   * - **What it holds**: the compiled module is the isolate's, shared, so
   *   the instance adds its own linear memory — its starting size is the
   *   release module's reading in the root `Cargo.toml`'s `release-wasm`
   *   comment, the ONE place the module's figures are measured (R1's
   *   cleanup debt 6) — and the one table document it is handed. Building
   *   it, `init`, the load and the copy measured about 0.15 ms a call in the
   *   pool.
   * - **When it is dropped**: when this returns. Nothing keeps it — `scratch`
   *   is this call's local, and its copy leaves as a string — so the
   *   isolate's garbage collector reclaims it, memory and all, whenever it
   *   next runs. One is made per call.
   *
   * ONE id through `house_copy`, `put_copy` and `remove_copy` (Task 5's
   * escalation 3): the copy is made of the document stored under `id` — this
   * object's name, which the layer wrote into the document as its `id` — and
   * the house stores it under the slot's `id`, this same name, and removes it
   * by that key, while its listing reads each row's id from the DOCUMENT.
   */
  protected copyOf(id: string, table: string): { copy: string | undefined } | { refused: string } | { trapped: string } {
    const scratch = new Instance(moduleConfig(this.env, null));
    try {
      const done = scratch.guarded((mod) => {
        mod.load("table", id, table);
        return mod.house_copy(id);
      });
      return "trapped" in done ? done : { copy: done.ok };
    } catch (e) {
      // `load` or `house_copy` refusing, with its sentence thrown as a string.
      if (typeof e === "string") return { refused: e };
      // Anything else — the instance could not be built (a `RangeError` when
      // its memory cannot be allocated), or any throw that is not the
      // module's — fails the side channel as a trap does, never the request
      // (the orchestrator's ruling on Task 12's review); `String(e)` keeps the
      // error's class for the log line.
      return { trapped: String(e) };
    }
  }

  /**
   * A write batch: `writes` inside ONE `transactionSync`, which first creates
   * the tables, and `alarm` beside it in the same synchronous turn, before
   * any `await` (spec §2.4): a time sets the alarm, `null` deletes it, and
   * `undefined` leaves it where it is. Every write this object makes goes
   * through here.
   *
   * An object the platform gives no name writes NOTHING, whatever it is asked
   * (the orchestrator's ruling, Task 9 fix round 1), and arms no alarm: no
   * alarm could ever serve what it stored, because the guard skips a nameless
   * alarm (`guardAlarmName`). So the rule holds here, by construction, and
   * not through what the layer happens to refuse.
   */
  protected batch(writes: (sql: SqlStorage) => void, alarm?: number | null): void {
    const id = this.ctx.id.name;
    if (id === undefined) return;
    const sql = this.ctx.storage.sql;
    this.ctx.storage.transactionSync(() => {
      ensureTables(sql);
      writes(sql);
    });
    this.arm(alarm);
  }

  /** The alarm, as `batch` sets it beside its writes: a time sets it, `null`
   *  deletes it, `undefined` leaves it where it is; a rejection is logged
   *  `alarm-failed`. An object the platform gives no name arms nothing, as it
   *  writes nothing (`batch`). */
  protected arm(alarm: number | null | undefined): void {
    const id = this.ctx.id.name;
    if (id === undefined || alarm === undefined) return;
    const armed = alarm === null ? this.ctx.storage.deleteAlarm() : this.ctx.storage.setAlarm(alarm);
    armed.catch((e: unknown) => log("alarm-failed", { id, error: String(e) }));
  }

  /**
   * What spec §2.4's one alarm waits for besides a delivery's retry: a
   * standing marker's end (`markerEnd`), or else the table's expiry
   * (`expiryDue`, `waiting` as it reads it) — `undefined` when neither is
   * due.
   */
  protected deadlines(sql: SqlStorage, waiting?: boolean): number | undefined {
    const marker = markerOf(sql);
    return marker === undefined ? this.expiryDue(sql, waiting) : markerEnd(marker);
  }

  /**
   * When the table expires (spec §2.4 "Expiry"; R-6.5c2.42): its last
   * activity plus the days `expiryDays` gives — the SHORTER of the two TTLs
   * from SQL alone, since no batch can read whether the table still waits for
   * players (the strings rule), and the TTL itself once the alarm has read
   * its status and passes `waiting` as `false` (`expiry`) — or, while its
   * expiries keep failing, the retry after them on the backoff (`nextRetry`),
   * if that is later. `undefined` when no expiry is due: no TTL, whatever the
   * waiting-table TTL reads; nothing stored, or a marker standing; or a table
   * `held` or poisoned, which never expires on its own (spec §2.4).
   */
  private expiryDue(sql: SqlStorage, waiting?: boolean): number | undefined {
    if (!hasTables(sql) || "error" in this.config) return undefined;
    const ttl = ttlDaysOf(this.config);
    if (ttl === null || getMeta(sql, "marker") !== undefined || getMeta(sql, "held") !== undefined || poisoned(sql)) return undefined;
    const activity = lastActive(sql);
    if (activity === null) return undefined;
    const deadline = expiryDeadline(activity, expiryDays(ttl, openTtlDaysOf(this.config), waiting));
    const failed = expiryFailedOf(sql);
    return failed > 0 ? Math.max(deadline, Date.now() + nextRetry(failed - 1)) : deadline;
  }

  /** What is still due once an owed save is discharged — a slot still
   *  waiting, or a copy still owed, each at its rung from now; the table's
   *  expiry, or a marker's end (`deadlines`, `waiting` as it reads it) — as
   *  the alarm's next deadline, or `null` when nothing is (spec §2.4: "the
   *  success that discharges it re-arms it to the next deadline"). */
  protected dueBesidesArchive(sql: SqlStorage, waiting?: boolean): number | null {
    const slot = slotState(sql).slot;
    const pushing = slot === undefined ? null : Date.now() + nextRetry(Math.max(slot.attempt - 1, 0));
    return earliest(earliest(pushing, copyRetryDue(copyOwedOf(sql)) ?? undefined), this.deadlines(sql, waiting)) ?? null;
  }

  /** Everything that is due, an owed save's retry among them: the alarm's
   *  next deadline, or `null` when nothing is. `waiting` is the table's status
   *  as the alarm read it — `false` re-arms the expiry at the TTL itself
   *  (`expiry`, R-6.5c2.42) — and every other caller leaves it unread. */
  protected nextDue(sql: SqlStorage, waiting?: boolean): number | null {
    return earliest(this.dueBesidesArchive(sql, waiting), this.archiveRung(sql)) ?? null;
  }

  /** One module call pinned to P-2's reading, answered as plain data with
   *  that reading. */
  protected pinned(mod: Exports, f: (mod: Exports) => Reply): Pinned {
    const at = this.reading();
    mod.set_clock(at);
    return { reply: readReply(f(mod)), at };
  }

  /** P-2's reading (`nextReading`): the module's time, in seconds. `answer`
   *  refuses a configuration the shell cannot read before any module call, so
   *  this never meets one. */
  protected reading(): number {
    if ("error" in this.config) throw new Error(this.config.error);
    return nextReading(this.config.clock);
  }
}

/**
 * **The table object** (spec §2.4): one table's documents in its own SQLite,
 * and the table layer compiled to WebAssembly answering the table's requests —
 * every `/api/games/{id}…`, and the create that names it — with the saves its
 * game owes the archive (spec §2.6): the abandon's (`abandon`), and game
 * over's (`deliverArchive`). An idle table expires on its alarm (`expiry`);
 * a table the layer deleted keeps a MARKER for 90 days, which answers every
 * request (`ended`); and an operator's expire reaches it past the Worker's
 * gate (`operate`, spec §2.9). Spec §2.3's Worker reaches it by the table's
 * id, so the object's name is the table's id (`ctx.id.name`, the plan's pool
 * fact 8), and `meta` stores none.
 */
export class Table extends withSockets(withLifecycle(withArchiving(withDelivery(TableBase)))) {}

/**
 * The id a request names — a create's `X-Yokohama-Table-Id`, percent-decoded,
 * or the path's segment after `/api/games/` — standing in for the object's
 * name only where the platform reports none, so that the reply is the layer's
 * answer for that id. The pool reports none, measured, for a name over 1,024
 * bytes — what spec §2.3's routing makes of a path's segment, or a create
 * body's `id`, that long — and for an object reached only through
 * `idFromString`. A nameless object writes nothing, whatever the layer answers
 * (`batch`).
 */
function requestedId(path: string, headers: Headers, create: boolean): string {
  if (create) return decodeURIComponent(headers.get(TABLE_ID_HEADER) ?? "");
  return path.slice("/api/games/".length).split("/")[0] ?? "";
}

/** What a request's batch leaves its side channels (`commit`). */
export interface Committed {
  /** The batch wrote a slot, which the request pushes. */
  pushing: boolean;
  /** A save is owed after the batch, and the switch reads `r2`: the request
   *  uploads it after its reply. */
  archiving: boolean;
  /** The marker the batch wrote, the layer having deleted the table. */
  marker: Marker | undefined;
}
