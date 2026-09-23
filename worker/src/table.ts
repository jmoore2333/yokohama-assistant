import { DurableObject } from "cloudflare:workers";
import type { Exports, Reply } from "../gen/factory.js";
import { guardAlarmName } from "./alarm";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { closeReason, END_DELETED, END_GONE, endAtAdmission, frame, sentenceOf } from "./frames";
import { Instance, readReply, type Bump, type Replied } from "./module";
import { errorBody, log, respond } from "./respond";
import { advances, cmpGen, copyDigest, HOUSE_NAME, nextRetry, type Ack, type Generation, type Slot } from "./slot";
import { deleteDoc, deleteMeta, ensureTables, getMeta, hasTables, readDocs, setMeta, upsertDoc } from "./storage";

/** The caller, as the Worker resolved it (the plan's internal headers). */
export const USER_HEADER = "X-Yokohama-User";
/** A create's object name, percent-encoded (the plan's internal headers). */
export const TABLE_ID_HEADER = "X-Yokohama-Table-Id";

/** The trap 500's sentence, host-written (spec §2.4, §4). */
const TRAP = "the table's engine stopped — try again";

/** `Reply.picker` for a list the module cleared: the row is deleted (the
 *  orchestrator's ruling I-21). */
const CLEARED = "[]";

/** The 426 a `GET …/stream` that is not an upgrade answers (spec §2.3;
 *  host-written, §4) — and the house's `/api/stream`'s too (the
 *  orchestrator's ruling I-48). */
export const NOT_AN_UPGRADE = "this host streams over WebSocket";

/** The close reason of a deleted table's sockets: short and fixed, because
 *  the `end` frame before the close carries the meaning. */
const DELETED_REASON = "this table was deleted";

/** A create's 409 at an id whose object still holds an unacknowledged
 *  REMOVAL (spec §2.4; host-written, §4). */
const USED_RECENTLY = "this name was used recently — try another";

/** The two transport routes this object answers itself rather than through
 *  `dispatch`, whose route table has no row for either (spec §2.3). */
type Transport = "events" | "stream";

/** What a socket carries across hibernation: the caller it was admitted as,
 *  exactly as `X-Yokohama-User` named them — the viewer every frame it is
 *  sent is computed for. */
interface Attachment {
  user: string | undefined;
}

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
   *  `poison` are cleared — by the commit, or, when the call then traps, by a
   *  batch that holds only that. */
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
  /** `…/events` (a GET) or `…/stream` (an upgrade), answered through the
   *  module's `events`; absent on every other request. */
  transport?: Transport;
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
  /** The house pushes in flight, by `pushKey`: `deliverSlot` hands a second
   *  caller of the same slot the push already under way, so one slot is
   *  never pushed twice at once, and a caller that awaits it — the corpus
   *  replay's flush, a test's `settle` — waits for its acknowledgement. */
  private readonly pushes = new Map<string, Promise<void>>();

  constructor(ctx: DurableObjectState, env: Env) {
    super(ctx, env);
    // Nothing is written here: the first write batch creates the tables.
    this.config = shellConfig(env);
    // The client's keep-alive (spec §2.7): every `ping` a socket sends is
    // answered `pong` by the platform, and never reaches `webSocketMessage`.
    ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const create = req.method === "POST" && url.pathname === "/api/games";
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
      user: req.headers.get(USER_HEADER) ?? undefined,
      method: req.method,
      path: url.pathname,
      query: url.search.slice(1),
      contentType: req.headers.get("content-type") ?? undefined,
      create,
      transport,
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
   * The alarm's work, for the name the platform handed `alarm()` (spec §2.4:
   * ONE alarm per object, set to the earliest thing due). An alarm with no
   * name — `guardAlarmName` says when that happens — is logged and touches
   * nothing. A named one:
   *
   * 1. recomputes the house copy from the STORED table document (`recopy`),
   *    which is how a copy that trapped in its request is delivered (the
   *    orchestrator's ruling I-42);
   * 2. delivers the slot (`deliverSlot`) — a retry after a failed push, or a
   *    push an abort cut off.
   *
   * The alarm is re-armed only by the batches those two write — a new
   * slot's, a failed push's, an acknowledgement's — so a run with nothing due
   * leaves none.
   */
  async onAlarm(name: string | undefined): Promise<void> {
    if (guardAlarmName(name) === "skip") {
      log("alarm-without-name");
      return;
    }
    // The guard runs only a named alarm.
    this.recopy(name as string);
    await this.deliverSlot();
  }

  /**
   * **The slot's ONE delivery routine** (spec §2.5): the request's side
   * channel after its reply, the alarm, and the corpus replay's flush all
   * call it. It pushes the slot `meta` holds NOW to the house's `apply` over
   * RPC — or, when that slot's push is already in flight, answers that push —
   * and resolves once the acknowledgement's batch, or the failure's, is
   * written. With no slot, or on an object the platform gives no name, it
   * does nothing.
   */
  async deliverSlot(): Promise<void> {
    const id = this.ctx.id.name;
    if (id === undefined) return;
    const slot = slotState(this.ctx.storage.sql).slot;
    if (slot === undefined) return;
    const key = pushKey(slot);
    const inFlight = this.pushes.get(key);
    if (inFlight !== undefined) return inFlight;
    const push = this.push(id, slot).finally(() => this.pushes.delete(key));
    this.pushes.set(key, push);
    return push;
  }

  /**
   * One push of `slot`, and what its answer owes: logged `slot-pushed` before
   * the RPC; then either the house's acknowledgement of THIS push — its
   * `apply` answers the pair it was handed, applied or ignored — logged
   * `slot-acked` (`acknowledged`), or the RPC's failure (`failed`).
   */
  private async push(id: string, slot: Slot): Promise<void> {
    log("slot-pushed", {
      id,
      generation: slot.generation,
      revision: slot.revision,
      removal: slot.copy === null,
      attempt: slot.attempt,
    });
    let ack: Ack;
    try {
      ack = await this.env.HOUSE.get(this.env.HOUSE.idFromName(HOUSE_NAME)).apply(slot);
    } catch (e) {
      this.failed(id, slot, e);
      return;
    }
    log("slot-acked", { id, generation: ack.generation, revision: ack.revision });
    await this.acknowledged(slot);
  }

  /**
   * The acknowledgement of `pushed` (spec §2.5), in ONE batch:
   *
   * - the slot is cleared ONLY if it still holds `pushed`'s pair — a newer
   *   copy, or a delete's REMOVAL, written meanwhile stays for its own push
   *   or the alarm to deliver;
   * - `acked` is set ONLY if the pair `advances` the `acked` it holds, with
   *   the pushed copy's digest — acknowledgements can arrive out of order, and
   *   a regressed `acked` would make a later change back to an older copy look
   *   already delivered;
   * - the alarm is re-armed to the next deadline — the slot still waiting's
   *   retry — or deleted when nothing is due.
   *
   * An acknowledgement that clears nothing and advances nothing writes
   * nothing. And the batch is written ONLY while the object still holds its
   * SQL tables (the orchestrator's ruling on Task 12), for two reasons:
   *
   * - a straggling acknowledgement that lands after a removal's `deleteAll()`
   *   must not recreate `meta` in an object that has ceased to exist
   *   (re-review m-g) — and `deleteAll()` drops the tables;
   * - the guard is the tables ALONE: one that also asked for a slot or a
   *   marker would skip the late acknowledgement of `r` that arrives after
   *   `r + 1` was acknowledged and cleared, so `advances` would never be
   *   consulted there, and the mutant that records `acked` without it would
   *   pass batteries 6 and 35 (measured).
   *
   * A REMOVAL acknowledged ends the object: `deleteAll()`, which at this
   * compatibility date deletes its alarm too, and its module is dropped. (The
   * plan's Task 14 puts the marker's 90 days between the two.)
   */
  private async acknowledged(pushed: Slot): Promise<void> {
    const sql = this.ctx.storage.sql;
    if (!hasTables(sql)) return;
    const state = slotState(sql);
    const holds = state.slot !== undefined && samePush(state.slot, pushed);
    if (holds && pushed.copy === null) {
      this.instance?.drop();
      await this.ctx.storage.deleteAll();
      return;
    }
    const advance = advances(state.acked ?? null, pushed.generation, pushed.revision);
    if (!holds && !advance) return;
    const acked: Acked = {
      generation: pushed.generation,
      revision: pushed.revision,
      digest: pushed.copy === null ? null : copyDigest(pushed.copy),
    };
    this.batch(
      (sql) => {
        if (holds) deleteMeta(sql, "slot");
        if (advance) setMeta(sql, "acked", JSON.stringify(acked));
      },
      holds ? null : retryDue(state.slot),
    );
  }

  /**
   * A push of `pushed` that failed — the RPC threw — logged `slot-failed`. If
   * the slot still holds it, ONE batch counts the failure in the slot and
   * re-arms the alarm to its next retry (`nextRetry`: 30 s, 2 min, 10 min,
   * then hourly). A slot written meanwhile has its own push and its own
   * retry, and an object that has ceased to exist has none.
   */
  private failed(id: string, pushed: Slot, error: unknown): void {
    // No slot at all once the object has ended: `getMeta` reads none.
    const slot = slotState(this.ctx.storage.sql).slot;
    const retrying = slot !== undefined && samePush(slot, pushed);
    log("slot-failed", {
      id,
      generation: pushed.generation,
      revision: pushed.revision,
      attempt: pushed.attempt,
      ...(retrying ? { retry_in: nextRetry(slot.attempt) } : {}),
      error: String(error),
    });
    if (!retrying) return;
    this.batch((sql) => setMeta(sql, "slot", JSON.stringify({ ...slot, attempt: slot.attempt + 1 })), retryDue(slot));
  }

  /**
   * The alarm's half of the orchestrator's ruling I-42: the house copy
   * recomputed from the STORED table document on a module of its own, and a
   * slot written — with its alarm, in one batch — when it differs from the one
   * last put in motion. A copy that trapped in its request is delivered so. A
   * trap or a refusal here is logged and arms nothing: the next change to the
   * table recomputes it, and a trap that repeated would otherwise re-arm the
   * alarm forever. A poisoned table is never re-instantiated (spec §2.4), and
   * nor is anything built for an object that holds no table document; a
   * configuration the shell refuses is logged, as every request logs it.
   */
  private recopy(id: string): void {
    const sql = this.ctx.storage.sql;
    const config = this.config;
    if ("error" in config) {
      log("init-refused", { id, error: config.error });
      return;
    }
    if (!hasTables(sql) || Number(getMeta(sql, "load_traps") ?? "0") >= 2) return;
    const table = readDocs(sql).get("table");
    if (table === undefined) return;
    const owed = this.owed(id, table, nextReading(config.clock));
    const writes = this.owedWrites(id, owed);
    if (writes.length === 0) return;
    this.batch(
      () => {
        for (const write of writes) write();
      },
      owed.slot === undefined ? undefined : Date.now() + nextRetry(0),
    );
  }

  /**
   * The request path (spec §2.4; the plan's Task 9, steps 3 to 7, and Task
   * 10), in one synchronous turn: the poison check, the wake, the call, the
   * ONE batch, the frames, and the reply — or, for a `…/stream` upgrade, the
   * admission (`admit`).
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
    // A create at an id whose slot holds a REMOVAL the house has not yet
    // acknowledged is refused, writing nothing, so a new table's first copy
    // can never overwrite a removal the house has not received (spec §2.4).
    if (incoming.create && slotState(sql).slot?.copy === null) return respond(409, errorBody(USED_RECENTLY));
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
    let reply: Replied;
    let bumps: Bump[];
    let pushing: boolean;
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
      pushing = this.commit(id, called, woken);
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
    // The side channels, after the batch (spec §2.4): the output gate holds
    // every send until the batch is durable (spec §0.4), and `sendFrames`
    // never throws, so nothing it meets costs the request its reply. Then the
    // house push, when the batch wrote a slot: it outlives the reply
    // (`waitUntil`), and the alarm the batch armed finishes it if it cannot.
    this.sendFrames(id, bumps);
    if (pushing) this.ctx.waitUntil(this.deliverSlot());
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
   * An upgrade's answer, from its admission — the request's own
   * `events(id, user, "")`, answered after the request's commit and its
   * frames (spec §2.4, "Sockets"):
   *
   * - **200**: the socket is accepted, hibernatable, with the caller as its
   *   attachment, and sent that reading at once — M6.3's first frame;
   * - **403 or 404**: the socket is ACCEPTED, sent an `end` that carries the
   *   layer's sentence (the orchestrator's ruling, Task 8's review), and
   *   closed 4403 or 4404 with that sentence as the reason — because a browser
   *   reports a refused upgrade as an anonymous failure it cannot tell from
   *   the network;
   * - **anything else** — the 401, an id the store will not use, a 500 — is
   *   answered unupgraded, exactly as the layer answered it (a browser cannot
   *   read a 401; spec §2.7's probe does).
   *
   * The socket is accepted after the request's frames were sent, so it is
   * sent only its first frame, which already reads whatever those frames
   * announced. Every socket accepted here is tagged with the table's id (spec
   * §2.4); nothing in this object reads the tag, since `sendFrames` frames
   * every socket the object holds.
   */
  private admit(id: string, user: string | undefined, reply: Replied): Response {
    const { status, body } = reply;
    if (status !== 200 && status !== 403 && status !== 404) return respond(status, body);
    const pair = new WebSocketPair();
    const [client, server] = [pair[0], pair[1]];
    this.ctx.acceptWebSocket(server, [id]);
    if (status === 200) {
      server.serializeAttachment({ user } satisfies Attachment);
      server.send(frame("signal", body));
    } else {
      const sentence = sentenceOf(body);
      server.send(frame("end", endAtAdmission(sentence)));
      server.close(status === 403 ? 4403 : 4404, closeReason(sentence ?? ""));
    }
    return new Response(null, { status: 101, webSocket: client });
  }

  /**
   * The frames a request owes its sockets (spec §2.4, "The framing"), sent
   * right after its commit. `bumps` is every bump the request's calls made,
   * the wake's and the call's, and the whole list is ONE frame per socket,
   * however many bumps it holds: a finalizing keep carries `bump_table` then
   * `bump` (the plan's Task 2), and two identical frames would each wake a
   * screen.
   *
   * - An `end` naming this table wins over every other bump: each socket is
   *   sent `end {"reason":"deleted"}` and closed 4404.
   * - Else, a `bump` or a `bump_table` naming it: each socket's viewer — its
   *   attachment's `user` — is read with `events(id, user, "")`, pinned to
   *   P-2's reading like every call that can reach the layer's clock. A 200 is
   *   sent as a `signal`; a 403 or a 404 — an admission lost under a live
   *   socket, such as a stranger's at an open table whose last seat has just
   *   filled — is the SSE's own `end {"reason":"gone"}` and a 4403 or 4404
   *   close with the layer's sentence as the reason; any other answer sends
   *   NOTHING, the SSE's comment arm, and the socket stays open.
   * - Anything else — no bump, or only the house's chat — sends nothing.
   *
   * A socket that is not open (one this object has closed, whose close has not
   * completed) is passed over.
   *
   * **It never throws**: whatever the request wrote is already committed, so
   * nothing here may cost the request its reply. A trap is a side channel's (spec §2.4
   * "Traps"): logged, the module discarded, and no socket framed after it —
   * `guarded` would answer the next socket from an EMPTY module, whose reading
   * of this table is a 404 that would end a live socket. Any other throw is
   * logged `frame-failed` and ends the framing the same way, and the module is
   * dropped for `answer`'s reason: it may hold what storage does not. Either
   * way the next request builds a new module and reloads it. A framing reply
   * that itself carries bumps or an archive plan is logged
   * `frame-carried-bumps`.
   */
  sendFrames(id: string, bumps: Bump[]): void {
    const ended = bumps.some((b) => "end" in b && b.end === id);
    const bumped = bumps.some((b) => ("bump" in b && b.bump === id) || ("bump_table" in b && b.bump_table === id));
    // Bumps come only from a call into this object's module, so the instance
    // is set whenever there are any; the check narrows the type.
    const instance = this.instance;
    if ((!ended && !bumped) || instance === undefined) return;
    try {
      for (const ws of this.ctx.getWebSockets()) {
        if (ws.readyState !== WebSocket.OPEN) continue;
        if (ended) {
          ws.send(frame("end", END_DELETED));
          ws.close(4404, closeReason(DELETED_REASON));
          continue;
        }
        const user = (ws.deserializeAttachment() as Attachment | null)?.user;
        const done = instance.guarded((mod) => this.pinned(mod, (m) => m.events(id, user, "")));
        if ("trapped" in done) {
          log("trap", { id, call: "frame", error: done.trapped });
          return;
        }
        const { status, body, bumps: carried, archive } = done.ok.reply;
        // A framing call only reads: the request's own calls have already
        // built the handle, so no known case pumps here. Should one ever
        // carry bumps or an archive plan, this pass neither frames nor owes
        // them, and the line records that it happened.
        if (carried.length > 0 || archive !== undefined) {
          log("frame-carried-bumps", { id, bumps: carried, archive: archive !== undefined });
        }
        if (status === 200) {
          ws.send(frame("signal", body));
        } else if (status === 403 || status === 404) {
          ws.send(frame("end", END_GONE));
          ws.close(status === 403 ? 4403 : 4404, closeReason(sentenceOf(body) ?? ""));
        }
      }
    } catch (e) {
      instance.drop();
      log("frame-failed", { id, error: String(e) });
    }
  }

  /**
   * A socket carries no move (spec §2.4): whatever a client sends is ignored,
   * so nothing a socket says reaches `dispatch` — a spectator's least of all.
   * The keep-alive's `ping` does not arrive here: the auto-response the
   * constructor sets answers it.
   */
  webSocketMessage(): void {}

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
   *   `"[]"` for a table it deletes with a list open (Task 4);
   * - the wake's `wakes` count, and its clearing of a load trap;
   * - **what the house is owed** (spec §2.5, `owed`), when the table
   *   document changed: the table's first generation, and the slot with its
   *   revision — the new copy, or a REMOVAL when the layer deleted the table
   *   — with its alarm at +30 s set beside the batch. A copy `house_copy`
   *   trapped on arms that alarm too, for a fresh module to recompute (the
   *   orchestrator's ruling I-42).
   *
   * Every write goes in ONE `batch`, and a request with nothing to write runs
   * none — so a probe creates nothing. Answers whether the batch wrote a
   * slot, which the request then pushes.
   */
  private commit(id: string, called: Called, parts: BatchParts): boolean {
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
    const picker = called.reply.picker ?? parts.restored?.reply.picker;
    if (picker === CLEARED) {
      if (stored.has("picker")) writes.push(() => deleteDoc(sql, "picker"));
    } else if (picker !== undefined) {
      writes.push(() => upsertDoc(sql, "picker", picker, now));
    }
    if (parts.countWake) {
      writes.push(() => setMeta(sql, "wakes", String(Number(getMeta(sql, "wakes") ?? "0") + 1)));
    }
    if (parts.clearTraps) writes.push(() => clearLoadTraps(sql));
    // Computed BEFORE the batch, from the documents already read out of the
    // module (the orchestrator's ruling I-42), and written LAST in it, so the
    // slot's size and last activity count the documents this batch writes.
    const owed = after === before ? undefined : this.owed(id, after, called.at);
    if (owed !== undefined) writes.push(...this.owedWrites(id, owed));
    if (writes.length === 0) return false;
    const due = owed !== undefined && (owed.slot !== undefined || owed.trapped) ? now + nextRetry(0) : undefined;
    this.batch(() => {
      for (const write of writes) write();
    }, due);
    return owed?.slot !== undefined;
  }

  /**
   * What a change of the table document owes the house (spec §2.5), decided
   * before any write: `table` is the document the change left — `undefined`
   * when the layer deleted the table — and `at` the P-2 reading its call was
   * pinned to.
   *
   * - **The generation** is drawn when `meta` holds none, at `at` with 64
   *   random bits: for every table this shell creates, by the create's own
   *   batch, whose reading is the table's `created_at`.
   * - **The copy** is `house_copy` of the document (`copyOf`), or `null`, a
   *   REMOVAL. One `house_copy` refuses is logged — its sentence VERBATIM —
   *   and owes nothing: the house keeps listing the last copy it received, a
   *   state only a corrupted store reaches, whose path is the operator's force
   *   (Task 5's escalation 1, ruled). One that TRAPS is logged, owes no slot,
   *   and is `trapped`.
   * - **The slot** is owed when the copy DIFFERS from the one last put in
   *   motion (`differs`), at the next revision.
   */
  private owed(id: string, table: string | undefined, at: number): Owed {
    const state = slotState(this.ctx.storage.sql);
    const draw = state.generation === undefined ? drawGeneration(at) : undefined;
    const generation = state.generation ?? (draw as Generation);
    let copy: string | null = null;
    if (table !== undefined) {
      const made = this.copyOf(id, table);
      if ("refused" in made) {
        log("copy-refused", { id, error: made.refused });
        return { draw, trapped: false };
      }
      if ("trapped" in made) {
        log("trap", { id, call: "house_copy", error: made.trapped });
        return { draw, trapped: true };
      }
      // `house_copy` answers `undefined` only where no table is stored, and
      // the module it ran on was handed one.
      if (made.copy === undefined) return { draw, trapped: false };
      copy = made.copy;
    }
    if (!differs(state, copy)) return { draw, trapped: false };
    return { draw, slot: { generation, revision: state.revision + 1, copy }, trapped: false };
  }

  /** The writes `owed` asks of a batch: its generation, then its slot and
   *  revision — the slot's last activity and size read INSIDE the batch,
   *  after every document write before it. */
  private owedWrites(id: string, owed: Owed): Array<() => void> {
    const sql = this.ctx.storage.sql;
    const writes: Array<() => void> = [];
    const { draw, slot: next } = owed;
    if (draw !== undefined) writes.push(() => setMeta(sql, "generation", JSON.stringify(draw)));
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
   * `house_copy` over `table`, on a module built for this one call and
   * holding only that document — measured at about 0.15 ms a call in the
   * pool. A trap there discards THAT module (the orchestrator's ruling
   * I-42), so this object's own — which the request's frames read next — is
   * never emptied by it: an emptied module reads this table as a 404, which
   * would end every live socket (`sendFrames`).
   *
   * ONE id through `house_copy`, `put_copy` and `remove_copy` (Task 5's
   * escalation 3): the copy is made of the document stored under `id` — this
   * object's name, which the layer wrote into the document as its `id` — and
   * the house stores it under the slot's `id`, this same name, and removes it
   * by that key, while its listing reads each row's id from the DOCUMENT.
   */
  private copyOf(id: string, table: string): { copy: string | undefined } | { refused: string } | { trapped: string } {
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
      throw e;
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
  private batch(writes: (sql: SqlStorage) => void, alarm?: number | null): void {
    const id = this.ctx.id.name;
    if (id === undefined) return;
    const sql = this.ctx.storage.sql;
    this.ctx.storage.transactionSync(() => {
      ensureTables(sql);
      writes(sql);
    });
    if (alarm === undefined) return;
    const armed = alarm === null ? this.ctx.storage.deleteAlarm() : this.ctx.storage.setAlarm(alarm);
    armed.catch((e: unknown) => log("alarm-failed", { id, error: String(e) }));
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

/** The transport route `path` names — `/api/games/{id}/events` or
 *  `/api/games/{id}/stream`, with `{id}` one non-empty segment — or
 *  `undefined` for every other path, which the layer's `dispatch` answers. */
function transportOf(path: string): Transport | undefined {
  const [root, api, games, id, route, ...rest] = path.split("/");
  if (root !== "" || api !== "api" || games !== "games" || !id || rest.length > 0) return undefined;
  return route === "events" || route === "stream" ? route : undefined;
}

/** Whether a request asks to become a WebSocket: its `Upgrade` token,
 *  compared case-insensitively, as RFC 6455 §4.2.1 reads it. */
export function isUpgrade(headers: Headers): boolean {
  return headers.get("Upgrade")?.toLowerCase() === "websocket";
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

/** A load trap forgotten: the count, and the kind whose load trapped. */
function clearLoadTraps(sql: SqlStorage): void {
  deleteMeta(sql, "load_traps");
  deleteMeta(sql, "poison");
}

/** What `acked` holds (spec §2.4): the pair last acknowledged, and the
 *  digest of its copy (`copyDigest`) — `null` for a REMOVAL. */
type Acked = Ack & { digest: string | null };

/** What a change of the table document owes the house (`Table.owed`). */
interface Owed {
  /** The table's first generation, which this batch draws. */
  draw: Generation | undefined;
  /** The slot this batch writes: a copy, or `null` — a REMOVAL — at its
   *  revision. */
  slot?: { generation: Generation; revision: number; copy: string | null };
  /** `house_copy` trapped: the copy is still owed. `commit` arms the alarm
   *  to recompute it; `recopy`, which the alarm runs, arms nothing. */
  trapped: boolean;
}

/** The table's slot state, as `meta` holds it (spec §2.4) — each value this
 *  shell wrote as JSON, none of them a record or a table document: the copy
 *  inside `slot` comes back as the exact string it went in as. Before the
 *  first batch there is none, and the revision is 0. */
interface SlotState {
  generation?: Generation;
  revision: number;
  slot?: Slot;
  acked?: Acked;
}

function slotState(sql: SqlStorage): SlotState {
  const parsed = <T>(k: "generation" | "slot" | "acked"): T | undefined => {
    const text = getMeta(sql, k);
    return text === undefined ? undefined : (JSON.parse(text) as T);
  };
  return {
    generation: parsed<Generation>("generation"),
    revision: Number(getMeta(sql, "revision") ?? "0"),
    slot: parsed<Slot>("slot"),
    acked: parsed<Acked>("acked"),
  };
}

/** Whether `copy` — `null`: a REMOVAL — differs from the one last put in
 *  motion (spec §2.5): the slot's while one waits, else the acknowledged
 *  one's, by its digest. With neither, anything differs. */
function differs(state: SlotState, copy: string | null): boolean {
  if (state.slot !== undefined) return copy !== state.slot.copy;
  if (state.acked === undefined) return true;
  return (copy === null ? null : copyDigest(copy)) !== state.acked.digest;
}

/** A new generation at the reading `at`, with 64 random bits. */
function drawGeneration(at: number): Generation {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  return { at, rand: [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("") };
}

/** The table's last activity (spec §2.4): the newest `written_at` of its
 *  record, table and chat — `null` when none is stored. */
function lastActive(sql: SqlStorage): number | null {
  return sql
    .exec<{ at: number | null }>("SELECT max(written_at) AS at FROM docs WHERE kind IN ('record', 'table', 'chat')")
    .one().at;
}

/** A push's key: its `(generation, revision)`. */
function pushKey(slot: Slot): string {
  return `${slot.generation.at}:${slot.generation.rand}:${slot.revision}`;
}

/** Whether two pushes carry the same `(generation, revision)`. */
function samePush(a: Ack, b: Ack): boolean {
  return cmpGen(a.generation, b.generation) === 0 && a.revision === b.revision;
}

/** When a waiting slot is next retried — `nextRetry` of its failures from
 *  now — or `null`, nothing due, with no slot. */
function retryDue(slot: Slot | undefined): number | null {
  return slot === undefined ? null : Date.now() + nextRetry(slot.attempt);
}
