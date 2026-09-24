import { DurableObject } from "cloudflare:workers";
import type { Exports, Reply } from "../gen/factory.js";
import { adminRoute, OPERATOR_HEADER, UNCONFIRMED } from "./admin";
import { earliest, guardAlarmName } from "./alarm";
import { nextReading } from "./clock";
import { moduleConfig, shellConfig, type ShellConfig } from "./config";
import type { Env } from "./env";
import { closeReason, END_DELETED, END_GONE, endAtAdmission, frame, sentenceOf } from "./frames";
import { expiryDeadline, gameOf, keptOf, markerEnd, markerSentence, ttlDaysOf, type Ending, type Marker, type Reason } from "./marker";
import { Instance, pinnedTo, readReply, type Bump, type Replied } from "./module";
import {
  executePlan,
  logOutcome,
  parsePlan,
  parseStamp,
  stampArm,
  type ArchiveOwed,
  type Outcome,
  type Plan,
  type Site,
} from "./r2";
import { errorBody, log, respond } from "./respond";
import { advances, cmpGen, copyDigest, HOUSE_NAME, nextRetry, type Ack, type Generation, type Slot } from "./slot";
import {
  clearDocs,
  clearMetaBut,
  deleteDoc,
  deleteMeta,
  ensureTables,
  getMeta,
  hasTables,
  readDocs,
  setMeta,
  upsertDoc,
} from "./storage";

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

/** A create's 409 at an id whose object holds a MARKER, or a REMOVAL the
 *  house has not acknowledged (spec §2.4; host-written, §4). */
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
 * every `/api/games/{id}…`, and the create that names it — with the saves its
 * game owes the archive (spec §2.6): the abandon's (`abandon`), and game
 * over's (`deliverArchive`). An idle table expires on its alarm (`expiry`);
 * a table the layer deleted keeps a MARKER for 90 days, which answers every
 * request (`ended`); and an operator's expire reaches it past the Worker's
 * gate (`operate`, spec §2.9). Spec §2.3's Worker reaches it by the table's
 * id, so the object's name is the table's id (`ctx.id.name`, the plan's pool
 * fact 8), and `meta` stores none.
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
  /** The owed saves' uploads in flight, by their plan's key:
   *  `deliverArchive` hands a second caller of the same owed save the upload
   *  already under way, so one plan is never uploaded twice at once, and a
   *  caller that awaits it — a test's `settle` — waits for what it settles. */
  private readonly uploads = new Map<string, Promise<void>>();

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
      user: req.headers.get(USER_HEADER) ?? undefined,
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
    // in its first dispatch, under it all the same.
    if (incoming.method === "DELETE") return this.ctx.blockConcurrencyWhile(() => this.abandon(incoming));
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
   * 3. expires the table once its last activity is a TTL old, or re-reads
   *    that activity and re-arms to the table's next deadline (`expiry`).
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
    await Promise.all([this.deliverSlot(), this.deliverArchive()]);
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
   * and `deleteAll()`, which at this compatibility date deletes its alarm too.
   * Logged `marker-ended` when it ends a marker, and `forced` when there is
   * none — which only a force leaves, since every delete through the layer
   * writes its marker in the batch that writes its REMOVAL.
   */
  private async end(marker: Marker | undefined): Promise<void> {
    const id = this.ctx.id.name;
    this.instance?.drop();
    await this.ctx.storage.deleteAll();
    if (marker === undefined) log("forced", { id });
    else log("marker-ended", { id, reason: marker.reason, at: marker.at, kept: marker.kept });
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
   * - the alarm: when the slot is cleared, re-armed to what is still due — a
   *   copy still owed (`copy_owed`), or a save (`archive_owed`, while the
   *   archive is on), each at its rung from now; the table's expiry, or a
   *   marker's end (`deadlines`) — the earliest of them, or deleted when
   *   nothing is; while a newer slot still waits, left where that slot's own
   *   batch or failure put it (one convention for the rung, `nextRetry`'s).
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
   * A REMOVAL acknowledged ends the object (`end`) once the delete's MARKER
   * has stood its 90 days, or at once when no marker stands — a force's
   * removal (spec §2.4 step 6, §2.9). While the marker stands, the batch below
   * clears the slot and re-arms the alarm to the marker's end.
   */
  private async acknowledged(pushed: Slot): Promise<void> {
    const sql = this.ctx.storage.sql;
    if (!hasTables(sql)) return;
    const state = slotState(sql);
    const holds = state.slot !== undefined && samePush(state.slot, pushed);
    if (holds && pushed.copy === null) {
      const marker = markerOf(sql);
      if (marker === undefined || Date.now() >= markerEnd(marker)) return this.end(marker);
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
      holds ? earliest(earliest(copyRetryDue(copyOwedOf(sql)), this.archiveRung(sql)), this.deadlines(sql)) : undefined,
    );
  }

  /**
   * A push of `pushed` that failed — the RPC threw — logged `slot-failed`. If
   * the slot still holds it, ONE batch counts the failure in the slot and
   * re-arms the alarm to its next retry (`nextRetry`: 30 s, 2 min, 10 min,
   * then hourly) — or to an owed save's, if that comes first (`earliest`). A
   * slot written meanwhile has its own push and its own retry, and an object
   * that has ceased to exist has none.
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
    this.batch(
      (sql) => setMeta(sql, "slot", JSON.stringify({ ...slot, attempt: slot.attempt + 1 })),
      earliest(retryDue(slot), this.archiveRung(this.ctx.storage.sql)),
    );
  }

  /**
   * The alarm's half of the orchestrator's ruling I-42: the house copy
   * recomputed from the STORED table document on a module of its own, and a
   * slot written — with its alarm, in one batch — when it differs from the one
   * last put in motion. A copy that trapped in its request is delivered so.
   * A trap here too is counted in `copy_owed` and re-armed on the backoff
   * (`nextRetry`: 30 s, 2 min, 10 min, then hourly — the ceiling a failing
   * push has), and every request computes the copy while it is owed
   * (`commit`): spec §2.4's "retries on the alarm or the next request". A
   * refusal is logged and arms nothing: it would repeat. A poisoned table is
   * never re-instantiated (spec §2.4), and nor is anything built for an
   * object that holds no table document; a configuration the shell refuses is
   * logged, as every request logs it.
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
    this.batch(() => {
      for (const write of writes) write();
    }, earliest(owedDue(owed), this.archiveRung(sql)));
  }

  /**
   * **The owed save's ONE delivery routine** (spec §2.6's game-over site): a
   * request whose batch leaves a save owed, and the alarm, call it. It uploads
   * the plan `archive_owed` holds NOW (`executePlan`), logs the outcome, and
   * settles what the outcome leaves (`settleUpload`) — or, when that plan's
   * upload is already in flight, answers that upload. It does nothing with no
   * save owed, on an object the platform gives no name, under a configuration
   * the shell refuses, or on a poisoned table, which is never re-instantiated
   * (spec §2.4).
   *
   * **While the switch reads `off`** the owed save is KEPT and not uploaded,
   * and nothing re-arms the alarm for it (spec §2.6): the alarm logs
   * `archive-owed-kept`, and no request calls this. Switched back on, the next
   * request or alarm resumes it.
   */
  private async deliverArchive(): Promise<void> {
    const id = this.ctx.id.name;
    const sql = this.ctx.storage.sql;
    const owed = archiveOwedOf(sql);
    if (id === undefined || owed === undefined || "error" in this.config) return;
    if (Number(getMeta(sql, "load_traps") ?? "0") >= 2) return;
    if (!this.archiveOn()) {
      log("archive-owed-kept", { id, record_key: owed.plan.record_key, attempt: owed.attempt });
      return;
    }
    const key = owed.plan.key;
    const inFlight = this.uploads.get(key);
    if (inFlight !== undefined) return inFlight;
    const upload = this.upload(id, owed).finally(() => this.uploads.delete(key));
    this.uploads.set(key, upload);
    return upload;
  }

  /** One upload of `owed`'s plan, its outcome logged with the failures before
   *  it, and what it leaves settled — the slot the settling batch writes
   *  pushed before this resolves. */
  private async upload(id: string, owed: ArchiveOwed): Promise<void> {
    // Only the receipt archive defers, and `shellConfig` refuses `r2` without
    // the bucket bound, so it is bound here.
    const outcome = await executePlan(this.env.ARCHIVE as R2Bucket, owed.plan);
    logOutcome(id, "game-over", owed.plan, outcome, { attempt: owed.attempt });
    if (this.settleUpload(id, owed, outcome)) await this.deliverSlot();
  }

  /**
   * What an owed save's upload leaves (spec §2.6's game-over site), after its
   * await, in one synchronous turn. Other requests ran during the upload's
   * I/O — nothing blocks them here — so it acts only while `archive_owed`
   * still holds the plan it uploaded: a delete that saved has cleared it, and
   * a later game over has replaced it.
   *
   * - **`failed`**: counted in `archive_owed` and retried by the alarm on the
   *   backoff (`retryArchive`).
   * - **`conflict`**: final at every site — `archive_owed` cleared, and the
   *   alarm re-armed to what is still due, or deleted when nothing is.
   * - **`landed` or `identical`**: the module — woken first when it is new —
   *   takes the outcome as its receipt and stamps with the layer's own code at
   *   the RECORDED reading (`stamp`), so its put makes the stored plan and
   *   meets the receipt. Its answer decides (`stampArm`): `stamped` completes
   *   the save and clears `archive_owed`; `no-longer-over` clears it too;
   *   `retry` — the game over, the stamp not landed — keeps it owed, counted,
   *   its plan the one the stamp re-made at that reading when the module
   *   deferred one. The stamped table row, `archive_owed` and the slot the
   *   changed copy owes go in ONE batch (`commit`). A trap is the side
   *   channel's (spec §2.4): logged, the module discarded, and the save
   *   retried.
   * - **A plan the wake deferred** — a new module's `restore_picker` pumping a
   *   bot run to game over — is owed rather than a clear, as every reply's
   *   plan is (the plan's ruling P-2). It is unreachable today: a save is
   *   owed only for a game already over, so the wake here finds it over, or —
   *   the undo case — back at a human's decision, and owes no bot run. Were
   *   a pump to move the record there, the receipt's plan would no longer
   *   match, the stamp would retry, and the plan it re-made — of that same
   *   stored record, at the recorded reading — is the one owed.
   *
   * **The undo case, and what it leaves** (the orchestrator's ruling I3;
   * Task 6's escalation e, RECORDED and not fixed this cycle). An undo at a
   * one-person table that lands during the upload's I/O leaves the game not
   * over at the stamp: nothing is owed, and the first completion's copy stays
   * in the bucket unstamped. A replay to the SAME end meets it as
   * `identical`, and stamps. A replay to a DIFFERENT end meets it as a
   * `conflict` — final, so that game is never stamped: the creator's later
   * abandon plans the same record key and is refused for good, and an expiry
   * is `held` until the operator's `?force`. The NAS stamps inside `persist`,
   * where no undo can land between the put and the stamp; a rule that stamps
   * on such a conflict is the owner's to make.
   *
   * Answers whether its batch wrote a slot.
   */
  private settleUpload(id: string, owed: ArchiveOwed, outcome: Outcome): boolean {
    const sql = this.ctx.storage.sql;
    const current = archiveOwedOf(sql);
    if (current?.plan.key !== owed.plan.key) return false;
    if (outcome.outcome === "failed") {
      this.retryArchive(current);
      return false;
    }
    if (outcome.outcome === "conflict") {
      this.batch((sql) => deleteMeta(sql, "archive_owed"), this.dueBesidesArchive(sql));
      return false;
    }
    const config = this.config;
    const traps = Number(getMeta(sql, "load_traps") ?? "0");
    // `deliverArchive` checked both before the upload; a load trap during
    // its I/O can have poisoned the table since.
    if ("error" in config || traps >= 2) return false;
    const instance = (this.instance ??= new Instance(moduleConfig(this.env, null)));
    let bumps: Bump[];
    let done: Committed;
    try {
      const woken = this.wake(id, instance, config, traps);
      if (woken instanceof Response) {
        // The wake logged why the module could not be loaded. The upload
        // stands; the stamp is retried.
        this.retryArchive(current);
        return false;
      }
      const completed = this.stamp(id, instance, current, outcome);
      if (completed === undefined) {
        this.retryArchive(current, woken.clearTraps);
        return false;
      }
      const { status, body, archive: replanned } = completed.reply;
      const stamp = status === 200 ? parseStamp(body) : undefined;
      log("archive-stamped", { id, record_key: current.plan.record_key, reading: current.reading, ...(stamp ?? { status, body }) });
      const arm = stamp === undefined ? "retry" : stampArm(stamp);
      done = this.commit(
        id,
        completed,
        woken,
        arm === "retry"
          ? {
              owe: {
                plan: replanned === undefined ? current.plan : parsePlan(replanned),
                reading: current.reading,
                attempt: current.attempt + 1,
              },
              due: this.retryAt(current),
            }
          : (owing(woken.restored) ?? "clear"),
      );
      bumps = [...(woken.restored?.reply.bumps ?? []), ...completed.reply.bumps];
    } catch (e) {
      // As `answer`'s: the module may hold what storage does not.
      instance.drop();
      throw e;
    }
    if (bumps.length > 0) this.sendFrames(id, bumps);
    return done.pushing;
  }

  /**
   * An owed save's completion, in the module: the upload's `outcome`
   * installed as the receipt, `stamp_archive(id)` pinned to the RECORDED
   * reading (`pinnedTo`, which restores the setting after it), and the layer's
   * documents read back — under ONE `guarded`. Answers the call, or
   * `undefined` when it trapped, logged.
   */
  private stamp(id: string, instance: Instance, owed: ArchiveOwed, outcome: Outcome): Called | undefined {
    let stage = "receipt";
    const done = instance.guarded((mod) => {
      mod.receipt(owed.plan.key, JSON.stringify(outcome));
      stage = "stamp_archive";
      const reply = pinnedTo(mod, owed.reading, (m) => m.stamp_archive(id));
      stage = "read";
      return { reply, at: owed.reading, docs: readLayerDocs(mod, id) };
    });
    if ("trapped" in done) {
      log("trap", { id, call: stage, error: done.trapped });
      return undefined;
    }
    return done.ok;
  }

  /**
   * `owed` still owed, one more failure counted, in ONE batch — with a load
   * trap's reset when this turn's wake cleared one — and the alarm re-armed to
   * its retry (`retryAt`), unless a load trap since has poisoned the table,
   * which re-arms nothing (spec §2.4).
   */
  private retryArchive(owed: ArchiveOwed, clearTraps = false): void {
    const sql = this.ctx.storage.sql;
    const poisoned = Number(getMeta(sql, "load_traps") ?? "0") >= 2;
    this.batch(
      (sql) => {
        setMeta(sql, "archive_owed", JSON.stringify({ ...owed, attempt: owed.attempt + 1 }));
        if (clearTraps) clearLoadTraps(sql);
      },
      poisoned ? undefined : this.retryAt(owed),
    );
  }

  /** When `owed` is retried after another failure, from now: `owed.attempt`
   *  counts the failures BEFORE this one, so `nextRetry` of it is the rung
   *  after this failure — 30 s after the first, as a slot's — or sooner, when
   *  something else owed is due first. */
  private retryAt(owed: ArchiveOwed): number {
    const due = Date.now() + nextRetry(owed.attempt);
    return Math.min(due, this.dueBesidesArchive(this.ctx.storage.sql) ?? due);
  }

  /** Whether saves are uploaded: the switch reads `r2`. */
  private archiveOn(): boolean {
    return !("error" in this.config) && this.config.archive === "r2";
  }

  /** An owed save's retry, from now — at the rung after its failures, or the
   *  first rung when none has failed — while the switch reads `r2`, and
   *  `undefined` when none is owed or the switch reads `off`: while off, the
   *  alarm is not re-armed for it (spec §2.6). */
  private archiveRung(sql: SqlStorage): number | undefined {
    const owed = this.archiveOn() ? archiveOwedOf(sql) : undefined;
    return owed === undefined ? undefined : Date.now() + nextRetry(Math.max(owed.attempt - 1, 0));
  }

  /**
   * The answer an ENDED table gives before anything is read, writing nothing
   * (spec §2.4): while a MARKER stands, every request is 404 with the
   * marker's sentence — an upgrade too, answered unupgraded, so the client's
   * probe reads it (spec §2.7) — and a create at the id is 409; with no
   * marker, a create at an id whose REMOVAL the house has not acknowledged —
   * a force's — is 409 too, so a new table's first copy can never overwrite a
   * removal the house has not received. `undefined` for every other request.
   */
  private ended(create: boolean): Response | undefined {
    const sql = this.ctx.storage.sql;
    const marker = markerOf(sql);
    if (marker !== undefined) return create ? respond(409, errorBody(USED_RECENTLY)) : respond(404, errorBody(markerSentence(marker)));
    if (create && slotState(sql).slot?.copy === null) return respond(409, errorBody(USED_RECENTLY));
    return undefined;
  }

  /**
   * What a request needs before its wake, or the answer that ends it first:
   * the poison check and the configuration — then the module (spec §2.4).
   */
  private ready(id: string): { config: ShellConfig; instance: Instance; traps: number } | Response {
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
   * 10) for every request but a DELETE (`abandon`), in one synchronous turn:
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
   * **The abandon** — every DELETE (spec §2.4 step 1, §2.6) — run WHOLE under
   * `blockConcurrencyWhile` (`fetch`), which delivers no other event to this
   * object until it returns, so the module and the rows it answers from are
   * this request's across the upload's await: the wake; the receipt loop of
   * every delete through the layer (`deleting`), over the caller's own
   * DELETE; then the ONE batch — the marker in it when the layer deleted the
   * table — the frames and the reply, as `answer`'s. Only the WAKE's plan, or
   * the reading's, can be owed by the batch: the dispatch's was dispatched
   * again.
   *
   * **A throw or a timeout RESETS this object** (Cloudflare's State API:
   * `blockConcurrencyWhile` terminates and resets an object whose callback
   * throws, or runs past 30 seconds), where `answer`'s throw fails only its
   * request. So a throw that is not a trap — a batch that does not commit, a
   * reply `readReply` refuses, a plan `parsePlan` refuses — resets the table
   * object, and the upload's up to four R2 round trips (a read, two puts, a
   * re-read) run inside that budget. Nothing is lost: the first dispatch
   * deletes nothing until its save is made, and a reset object is rebuilt
   * from what is stored. An expiry runs under the same block, with the same
   * budget (`expiry`, `operate`).
   */
  private async abandon(incoming: Incoming): Promise<Response> {
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
      const deleted = await this.deleting(id, instance, config, {
        site: "abandon",
        reason: { reason: "abandoned" },
        viewer: () => incoming.user,
        call: "dispatch",
        f: (m) => m.dispatch(incoming.user, incoming.method, incoming.path, incoming.query, incoming.contentType, incoming.body),
      });
      if (deleted instanceof Response) {
        if (woken.clearTraps) this.batch(clearLoadTraps);
        return deleted;
      }
      const { called, reading, ending } = deleted;
      done = this.commit(id, called, woken, owing(woken.restored) ?? owing(reading), ending);
      reply = called.reply;
      bumps = [...(woken.restored?.reply.bumps ?? []), ...(reading?.reply.bumps ?? []), ...reply.bumps];
    } catch (e) {
      // As `answer`'s: the module may hold what storage does not.
      instance.drop();
      throw e;
    }
    return this.finish(incoming, reply, bumps, done);
  }

  /**
   * **The receipt loop of every delete through the layer** (spec §2.4 step 1,
   * §2.6) — a creator's abandon, and an expiry, the alarm's or an operator's
   * — after the wake, under its caller's `blockConcurrencyWhile`:
   *
   * 1. P-2's reading, taken ONCE, and under ONE `guarded`: the viewer, read
   *    first (an expiry's, off the table's house copy); then, pinned to that
   *    reading, the call that decides what the marker says was kept (the
   *    orchestrator's ruling I-64) — `events(id, viewer, "")` as the table's
   *    creator, before anything is deleted — and the delete's own first
   *    dispatch; then the layer's documents read back. The `events` call
   *    shares the dispatch's P-2 reading, so a delete still takes one;
   * 2. when that dispatch deferred a put — the layer refuses the delete,
   *    deleting nothing, until its save is made — the plan is carried out
   *    (`executePlan`) and its outcome logged, then installed as the receipt,
   *    and the SAME call dispatched AGAIN at the SAME reading (`redispatch`),
   *    so the layer's put makes the very plan the receipt is for. Its answer
   *    is the answer: a delete that proceeds, or the layer's own refusal
   *    naming the archive with R2's reason inside it — a `conflict` among
   *    them, final at every site;
   * 3. the ENDING the delete's batch writes as its marker, should the layer
   *    have deleted the table: `kept` from the reading, the archive's switch
   *    and the save's outcome (`gameOf`, `keptOf`) — never from the table
   *    document, which is never parsed.
   *
   * A delete that owes no save — an open or dealing table, one with no move
   * made, one already stamped, the archive off, a caller the layer refuses —
   * completes in its first dispatch. A trap is the trap 500, persisting
   * nothing.
   *
   * The viewer is the table's creator whenever the layer deletes: the
   * abandon's caller, whom the layer's `delete` refuses unless they created
   * the table, and for an expiry the creator its house copy names
   * (`creatorIn`) — so the `events` call that decides a marker answers 200.
   * The stored record, `gameOf`'s fallback, would decide only where `creatorIn`
   * finds no creator and the layer deletes all the same, and no stored table
   * reaches that: `house_copy` refuses a stored document the layer's own
   * delete also refuses to read (both run `check_shape` over the same stored
   * text), or a copy its scrub broke, which the module's own cells show no
   * stored document reaches.
   */
  private async deleting(id: string, instance: Instance, config: ShellConfig, how: Deleting): Promise<Deleted | Response> {
    const stored = readDocs(this.ctx.storage.sql);
    // An expiry's viewer reads the house copy; an abandon's calls nothing.
    let stage = "house_copy";
    const done = instance.guarded((mod) => {
      const at = this.reading();
      const viewer = how.viewer(mod);
      // The pin is the reading's, as `pinned` counts a pin with its call.
      stage = "events";
      mod.set_clock(at);
      const reading = viewer === undefined ? undefined : readReply(mod.events(id, viewer, ""));
      stage = how.call;
      const reply = readReply(how.f(mod));
      stage = "read";
      return { reading, first: { reply, at, docs: readLayerDocs(mod, id) } };
    });
    if ("trapped" in done) {
      log("trap", { id, call: stage, error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    const { reading, first } = done.ok;
    let called: Called = first;
    let outcome: Outcome | undefined;
    let plan: Plan | undefined;
    const deferred = first.reply.archive;
    if (deferred !== undefined) {
      plan = parsePlan(deferred);
      // Only the receipt archive defers, and `shellConfig` refuses `r2`
      // without the bucket bound, so it is bound here.
      outcome = await executePlan(this.env.ARCHIVE as R2Bucket, plan);
      logOutcome(id, how.site, plan, outcome, how.attempt === undefined ? {} : { attempt: how.attempt });
      const again = this.redispatch(id, instance, first, plan, outcome, how.f, how.call);
      if (again instanceof Response) return again;
      called = again;
    }
    const saved = outcome?.outcome === "landed" || outcome?.outcome === "identical";
    return {
      called,
      reading: reading === undefined ? null : { reply: reading, at: first.at },
      outcome,
      plan,
      ending: { ...how.reason, kept: keptOf(gameOf(reading, stored.has("record")), config.archive, saved) },
    };
  }

  /**
   * The receipt loop's second dispatch: `outcome` installed as `plan`'s
   * receipt, then the SAME call `f` dispatched pinned to the FIRST
   * dispatch's reading (`pinnedTo`, which restores the setting after it) —
   * spec §2.6's "one clock reading per plan": a reading of its own would stamp
   * another `archived_at`, and its plan would not be the receipt's — then the
   * layer's documents read back, all under ONE `guarded`, as `call`'s. The
   * answer carries both dispatches' bumps (a deferred put signals nothing,
   * but a call's bumps are never dropped). A trap is the trap 500, persisting
   * nothing, logged with `call` naming the dispatch.
   */
  private redispatch(
    id: string,
    instance: Instance,
    first: Called,
    plan: Plan,
    outcome: Outcome,
    f: (mod: Exports) => Reply,
    call: string,
  ): Called | Response {
    let stage = "receipt";
    const done = instance.guarded((mod) => {
      mod.receipt(plan.key, JSON.stringify(outcome));
      stage = call;
      const reply = pinnedTo(mod, first.at, f);
      stage = "read";
      return { reply, at: first.at, docs: readLayerDocs(mod, id) };
    });
    if ("trapped" in done) {
      log("trap", { id, call: stage, error: done.trapped });
      return respond(500, errorBody(TRAP));
    }
    const again = done.ok;
    return { ...again, reply: { ...again.reply, bumps: [...first.reply.bumps, ...again.reply.bumps] } };
  }

  /**
   * **An expiry** (spec §2.4 step 1; R-6.5c.15) — the alarm's, once the TTL
   * has run out (`expiry`), and an operator's plain expire (`operate`) — as
   * the table's creator's own abandon: the receipt loop (`deleting`) over the
   * module's host-local `expire(id)`, which dispatches the layer's
   * `DELETE /api/games/{id}` as the stored table's `created_by`, so the
   * answer, the archive rule and the leaf are the layer's. It runs under its
   * caller's `blockConcurrencyWhile`, as an abandon does; then the ONE batch
   * — the marker in it when the layer deleted the table, logged `expired` —
   * and the frames. The pushes and the upload the batch owes are the
   * caller's. Answers the layer's answer, or the shell's before it (a poison,
   * a trap), with the batch and the save's outcome when the delete was
   * dispatched.
   */
  private async expire(id: string, reason: Reason): Promise<Expired> {
    const answered = async (res: Response): Promise<Expired> => ({ status: res.status, body: await res.text() });
    const ready = this.ready(id);
    if (ready instanceof Response) return answered(ready);
    const { config, instance, traps } = ready;
    let expired: Expired;
    let bumps: Bump[];
    try {
      const woken = this.wake(id, instance, config, traps);
      if (woken instanceof Response) return answered(woken);
      const deleted = await this.deleting(id, instance, config, {
        site: reason.reason === "expired" ? "expiry" : "operator",
        reason,
        viewer: (mod) => creatorIn(mod, id),
        call: "expire",
        f: (m) => m.expire(id),
        ...(reason.reason === "expired" ? { attempt: expiryFailedOf(this.ctx.storage.sql) } : {}),
      });
      if (deleted instanceof Response) {
        if (woken.clearTraps) this.batch(clearLoadTraps);
        return answered(deleted);
      }
      const { called, reading, ending, outcome, plan } = deleted;
      const done = this.commit(id, called, woken, owing(woken.restored) ?? owing(reading), ending);
      expired = { status: called.reply.status, body: called.reply.body, done, outcome, plan };
      bumps = [...(woken.restored?.reply.bumps ?? []), ...(reading?.reply.bumps ?? []), ...called.reply.bumps];
    } catch (e) {
      // As `answer`'s: the module may hold what storage does not.
      instance.drop();
      throw e;
    }
    this.sendFrames(id, bumps);
    const marker = expired.done?.marker;
    if (marker !== undefined) log("expired", { id, reason: marker.reason, kept: marker.kept });
    return expired;
  }

  /**
   * The alarm's expiry (spec §2.4 "Expiry"; R-6.5c.15), after its
   * deliveries. With a TTL (`ttlDaysOf`) and a document stored, from the
   * table's last activity — the newest `written_at` of its record, table and
   * chat, RE-READ at every run, because the expiry deadline only moves later
   * and nothing re-arms the alarm for a write that makes nothing due:
   *
   * - a table `held` or poisoned never expires on its own (spec §2.4; the
   *   operator's force is its path): past its TTL it is named in the log,
   *   `expiry-blocked`, and no expiry deadline is armed for it;
   * - before `last activity + TTL` — an alarm armed for other work, or at a
   *   deadline a later activity has since moved — the alarm is re-armed to
   *   the table's next deadline (`nextDue`), and a count of failed expiries
   *   that later activity has made stale is dropped;
   * - otherwise the table expires (`expire`) under `blockConcurrencyWhile`,
   *   and an expiry that did not end it is settled (`unexpired`) before the
   *   block ends. The delete's side channels — its removal's push, an owed
   *   save's upload — follow the block as a request's follow its reply
   *   (`waitUntil`, `finish`), and the alarm armed in the delete's batch
   *   finishes them if they cannot.
   */
  private async expiry(id: string): Promise<void> {
    const sql = this.ctx.storage.sql;
    const config = this.config;
    if ("error" in config || !hasTables(sql)) return;
    const ttl = ttlDaysOf(config);
    const activity = lastActive(sql);
    if (ttl === null || activity === null) return;
    const deadline = expiryDeadline(activity, ttl);
    const blocked = getMeta(sql, "held") !== undefined ? "held" : poisoned(sql) ? "poisoned" : undefined;
    if (blocked !== undefined) {
      if (Date.now() >= deadline) log("expiry-blocked", { id, why: blocked });
      return;
    }
    if (Date.now() < deadline) {
      if (expiryFailedOf(sql) > 0) this.batch((sql) => deleteMeta(sql, "expiry_failed"));
      this.arm(this.nextDue(sql));
      return;
    }
    const done = await this.ctx.blockConcurrencyWhile(async () => {
      const expired = await this.expire(id, { reason: "expired", ttl_days: ttl });
      if (expired.done?.marker === undefined) this.unexpired(id, expired);
      return expired.done;
    });
    if (done?.pushing) this.ctx.waitUntil(this.deliverSlot());
    if (done?.archiving) this.ctx.waitUntil(this.deliverArchive());
  }

  /**
   * An expiry the layer did not end (spec §2.4 step 1): the table stays, and
   * one batch says what follows, the alarm re-armed beside it to the next
   * deadline (`nextDue`):
   *
   * - a `conflict` — the record key already holds another game, which no
   *   retry can clear — writes `held`, logged: the expiry deadline is armed no
   *   more, and the table waits for the operator's force;
   * - anything else — a save that failed, a trap, a wake that could not load
   *   the table — is counted in `expiry_failed` and retried on the backoff,
   *   30 s, 2 min, 10 min, then hourly (`expiryDue`), logged `expiry-failed`
   *   with the answer it met.
   */
  private unexpired(id: string, expired: Expired): void {
    const sql = this.ctx.storage.sql;
    if (expired.outcome?.outcome === "conflict" && expired.plan !== undefined) {
      const recordKey = expired.plan.record_key;
      this.batch((sql) => {
        setMeta(sql, "held", JSON.stringify({ at: Date.now(), record_key: recordKey }));
        deleteMeta(sql, "expiry_failed");
      });
      this.arm(this.nextDue(sql));
      log("held", { id, record_key: recordKey });
      return;
    }
    const attempt = expiryFailedOf(sql);
    this.batch((sql) => setMeta(sql, "expiry_failed", String(attempt + 1)));
    this.arm(this.nextDue(sql));
    log("expiry-failed", { id, status: expired.status, body: expired.body, attempt, retry_in: nextRetry(attempt) });
  }

  /**
   * **An operator's expire** (spec §2.9), reached only with
   * `X-Yokohama-Operator`, which the Worker sets past its gate; logged
   * `operator` with the action and the operator's address.
   *
   * - `?force`: `force`.
   * - A plain expire: while a marker stands, the marker's 404, as every
   *   request meets it. Otherwise the table expires NOW by spec §2.4's path
   *   (`expire`), as its creator's own abandon with reason `operator` — the
   *   save first when one is owed, the marker after — under
   *   `blockConcurrencyWhile`, and the answer is the layer's: its 204, or its
   *   refusal when the save fails, which writes nothing of the alarm's
   *   (`held`, a retry's count).
   */
  private async operate(id: string, address: string, force: boolean): Promise<Response> {
    log("operator", { id, action: force ? "force" : "expire", address });
    if (force) return this.force(id);
    const marker = markerOf(this.ctx.storage.sql);
    if (marker !== undefined) return respond(404, errorBody(markerSentence(marker)));
    const expired = await this.ctx.blockConcurrencyWhile(() => this.expire(id, { reason: "operator" }));
    if (expired.done?.pushing) this.ctx.waitUntil(this.deliverSlot());
    if (expired.done?.archiving) this.ctx.waitUntil(this.deliverArchive());
    return respond(expired.status, expired.body);
  }

  /**
   * **The operator's `?force`** (spec §2.9): the object ends WHATEVER it holds
   * — a table, a poisoned one, a `held` one, a marker — with no layer and no
   * save, and it leaves NO marker.
   *
   * - With nothing stored: 204 at once, writing nothing.
   * - Where the house holds nothing of the table — it was never sent a copy,
   *   or it has acknowledged the table's removal — the object ends at once
   *   (`end`), and 204.
   * - Otherwise ONE batch deletes every document, and every `meta` key but
   *   the slot's own four — the marker, an owed save and `held` among them —
   *   and writes the REMOVAL, unless one already waits, with its retry's
   *   alarm. The module is dropped and every socket ended as a delete ends
   *   them, and the removal is pushed IN the request (`deliverSlot`): its
   *   acknowledgement, with no marker standing, ends the object
   *   (`acknowledged`). The answer is 204 once the storage is gone, or 503
   *   `the lobby has not confirmed the removal — the alarm will finish it`
   *   when the push failed — and the alarm, re-armed by that failure, does.
   *   Until then a create at the id is the 409 of a removal the house has not
   *   acknowledged (`ended`).
   *
   * An owed save it drops is logged `archive-owed-dropped`, as a delete's is.
   * It holds no `blockConcurrencyWhile`: a request that lands while the push
   * is out finds no document to answer from, and every push and
   * acknowledgement is keyed by its pair (spec §2.5).
   */
  private async force(id: string): Promise<Response> {
    const sql = this.ctx.storage.sql;
    if (!hasTables(sql)) return respond(204, "");
    const state = slotState(sql);
    const owed = archiveOwedOf(sql);
    const drop = () => {
      if (owed !== undefined) log("archive-owed-dropped", { id, record_key: owed.plan.record_key, attempt: owed.attempt });
    };
    if (state.generation === undefined || (state.slot === undefined && !differs(state, null))) {
      this.endSockets(id);
      await this.end(undefined);
      drop();
      return respond(204, "");
    }
    const removal = state.slot?.copy === null ? undefined : { generation: state.generation, revision: state.revision + 1, copy: null };
    this.batch(
      (sql) => {
        clearDocs(sql);
        clearMetaBut(sql, ["generation", "revision", "slot", "acked"]);
        for (const write of this.owedWrites(id, { draw: undefined, slot: removal, traps: 0 })) write();
      },
      removal === undefined ? undefined : Date.now() + nextRetry(0),
    );
    this.instance?.drop();
    this.endSockets(id);
    drop();
    await this.deliverSlot();
    return hasTables(sql) ? respond(503, errorBody(UNCONFIRMED)) : respond(204, "");
  }

  /**
   * The side channels, after the request's batch (spec §2.4), and the reply:
   * the output gate holds every send until the batch is durable (spec §0.4),
   * and `sendFrames` never throws, so nothing it meets costs the request its
   * reply. Then the house push, when the batch wrote a slot, and the upload of
   * a save the table owes: each outlives the reply (`waitUntil`), and the alarm
   * armed beside the batch that made it due finishes it if it cannot.
   */
  private finish(incoming: Incoming, reply: Replied, bumps: Bump[], done: Committed): Response {
    const { id } = incoming;
    this.sendFrames(id, bumps);
    if (done.pushing) this.ctx.waitUntil(this.deliverSlot());
    if (done.archiving) this.ctx.waitUntil(this.deliverArchive());
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
      if (ended) {
        this.closeDeleted();
        return;
      }
      for (const ws of this.ctx.getWebSockets()) {
        if (ws.readyState !== WebSocket.OPEN) continue;
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

  /** Every open socket ended as a deleted table's: `end {"reason":"deleted"}`,
   *  then a 4404 close with this host's short reason. */
  private closeDeleted(): void {
    for (const ws of this.ctx.getWebSockets()) {
      if (ws.readyState !== WebSocket.OPEN) continue;
      ws.send(frame("end", END_DELETED));
      ws.close(4404, closeReason(DELETED_REASON));
    }
  }

  /** A force's end of every socket (`closeDeleted`), which never throws: the
   *  storage is already gone, or going, so nothing here may cost the force its
   *  answer. A throw is logged `frame-failed`. */
  private endSockets(id: string): void {
    try {
      this.closeDeleted();
    } catch (e) {
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
   * with the switch on, which the request then uploads — one this batch
   * wrote, or one owed before it: the next request resumes it (spec §2.6);
   * and the marker it wrote.
   */
  private commit(id: string, called: Called, parts: BatchParts, archive?: ArchiveWrite, ending?: Ending): Committed {
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
    if (parts.countWake) {
      writes.push(() => setMeta(sql, "wakes", String(Number(getMeta(sql, "wakes") ?? "0") + 1)));
    }
    if (parts.clearTraps) writes.push(() => clearLoadTraps(sql));
    // Computed BEFORE the batch, from the documents already read out of the
    // module (the orchestrator's ruling I-42), and written LAST in it, so the
    // slot's size and last activity count the documents this batch writes.
    // A copy still owed is computed by every request until it is made (spec
    // §2.4: a side channel's trap "retries on the alarm or the next request").
    const owed = after !== before || copyOwedOf(sql) > 0 ? this.owed(id, after, called.at) : undefined;
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
    const archiving = owes && this.archiveOn();
    if (writes.length === 0) return { pushing: false, archiving, marker };
    this.batch(() => {
      for (const write of writes) write();
    }, alarm);
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
   *   (Task 5's escalation 1, ruled). One that TRAPS — or fails in any other
   *   way than a refusal (`copyOf`) — is logged, owes no slot, and is still
   *   owed: `traps` counts it on the `copy_owed` before it. Any other outcome
   *   owes no copy, and `traps` is 0.
   * - **The slot** is owed when the copy DIFFERS from the one last put in
   *   motion (`differs`), at the next revision.
   */
  private owed(id: string, table: string | undefined, at: number): Owed {
    const sql = this.ctx.storage.sql;
    const state = slotState(sql);
    const draw = state.generation === undefined ? drawGeneration(at) : undefined;
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
  private owedWrites(id: string, owed: Owed): Array<() => void> {
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
   *   (`recopy`). A move that leaves the table document alone makes none.
   * - **What it holds**: the compiled module is the isolate's, shared, so
   *   the instance adds its own linear memory, which starts at 18 pages —
   *   1.125 MiB, 121,001 bytes of them the module's data, copied in from its
   *   168 data segments (the release build, read off its memory and data
   *   sections; the data SECTION is 122,381 bytes encoded, its segment
   *   headers included) — and the one table document it is handed. Building
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
  private batch(writes: (sql: SqlStorage) => void, alarm?: number | null): void {
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
  private arm(alarm: number | null | undefined): void {
    const id = this.ctx.id.name;
    if (id === undefined || alarm === undefined) return;
    const armed = alarm === null ? this.ctx.storage.deleteAlarm() : this.ctx.storage.setAlarm(alarm);
    armed.catch((e: unknown) => log("alarm-failed", { id, error: String(e) }));
  }

  /**
   * What spec §2.4's one alarm waits for besides a delivery's retry: a
   * standing marker's end (`markerEnd`), or else the table's expiry
   * (`expiryDue`) — `undefined` when neither is due.
   */
  private deadlines(sql: SqlStorage): number | undefined {
    const marker = markerOf(sql);
    return marker === undefined ? this.expiryDue(sql) : markerEnd(marker);
  }

  /**
   * When the table expires (spec §2.4 "Expiry"): its last activity plus the
   * TTL — or, while its expiries keep failing, the retry after them on the
   * backoff (`nextRetry`), if that is later. `undefined` when no expiry is
   * due: no TTL; nothing stored, or a marker standing; or a table `held` or
   * poisoned, which never expires on its own (spec §2.4).
   */
  private expiryDue(sql: SqlStorage): number | undefined {
    if (!hasTables(sql) || "error" in this.config) return undefined;
    const ttl = ttlDaysOf(this.config);
    if (ttl === null || getMeta(sql, "marker") !== undefined || getMeta(sql, "held") !== undefined || poisoned(sql)) return undefined;
    const activity = lastActive(sql);
    if (activity === null) return undefined;
    const deadline = expiryDeadline(activity, ttl);
    const failed = expiryFailedOf(sql);
    return failed > 0 ? Math.max(deadline, Date.now() + nextRetry(failed - 1)) : deadline;
  }

  /** What is still due once an owed save is discharged — a slot still
   *  waiting, or a copy still owed, each at its rung from now; the table's
   *  expiry, or a marker's end (`deadlines`) — as the alarm's next deadline,
   *  or `null` when nothing is (spec §2.4: "the success that discharges it
   *  re-arms it to the next deadline"). */
  private dueBesidesArchive(sql: SqlStorage): number | null {
    const slot = slotState(sql).slot;
    const waiting = slot === undefined ? null : Date.now() + nextRetry(Math.max(slot.attempt - 1, 0));
    return earliest(earliest(waiting, copyRetryDue(copyOwedOf(sql)) ?? undefined), this.deadlines(sql)) ?? null;
  }

  /** Everything that is due, an owed save's retry among them: the alarm's
   *  next deadline, or `null` when nothing is. */
  private nextDue(sql: SqlStorage): number | null {
    return earliest(this.dueBesidesArchive(sql), this.archiveRung(sql)) ?? null;
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
  /** The `copy_owed` this batch leaves: the `house_copy` traps in a row,
   *  this one counted — the copy still owed, for the alarm (at the rung
   *  after them) and every request to recompute — or 0, nothing owed. */
  traps: number;
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

/** When `slot` is next retried after a push of it failed, from now:
 *  `slot.attempt` counts the failures BEFORE this one, so `nextRetry` of it
 *  is the rung after this failure — 30 s after the first. */
function retryDue(slot: Slot): number {
  return Date.now() + nextRetry(slot.attempt);
}

/** `meta.copy_owed`: the `house_copy` traps in a row — 0 when no copy is
 *  owed. */
function copyOwedOf(sql: SqlStorage): number {
  return Number(getMeta(sql, "copy_owed") ?? "0");
}

/** An owed copy's retry, from now, at the rung after its `traps`-th trap in
 *  a row — `null` when none is owed. */
function copyRetryDue(traps: number): number | null {
  return traps > 0 ? Date.now() + nextRetry(traps - 1) : null;
}

/** When the alarm must run for what `owed` leaves due: a new slot's first
 *  retry (30 s), or an owed copy's retry — `undefined`, the alarm left where
 *  it is, when it leaves nothing due. */
function owedDue(owed: Owed): number | undefined {
  if (owed.slot !== undefined) return Date.now() + nextRetry(0);
  return copyRetryDue(owed.traps) ?? undefined;
}

/** What a request's batch leaves its side channels (`commit`). */
interface Committed {
  /** The batch wrote a slot, which the request pushes. */
  pushing: boolean;
  /** A save is owed after the batch, and the switch reads `r2`: the request
   *  uploads it after its reply. */
  archiving: boolean;
  /** The marker the batch wrote, the layer having deleted the table. */
  marker: Marker | undefined;
}

/** How a delete through the layer is made (`Table.deleting`). */
interface Deleting {
  /** Where its save's plan is made, as the outcome's line names it. */
  site: Site;
  /** How the table ends, should the layer delete it. */
  reason: Reason;
  /** Whom the reading that decides `kept` is taken as: the table's creator. */
  viewer: (mod: Exports) => string | undefined;
  /** The delete's own call — the caller's DELETE, or `expire(id)` — made
   *  twice when it defers a put. */
  f: (mod: Exports) => Reply;
  /** That call's name, in a trap's line. */
  call: string;
  /** An expiry's failures before this one, on the outcome's line. */
  attempt?: number;
}

/** What a delete through the layer did (`Table.deleting`). */
interface Deleted {
  /** The last dispatch: the first, or the second when a put was deferred. */
  called: Called;
  /** The `events` call `kept` was decided by, pinned to the delete's one P-2
   *  reading — a plan it deferred is owed as the wake's would be — or `null`
   *  when there was no viewer to make it as. */
  reading: Pinned | null;
  /** A deferred put's outcome, and its plan. */
  outcome: Outcome | undefined;
  plan: Plan | undefined;
  /** What the marker records, should the layer have deleted the table. */
  ending: Ending;
}

/** What an expiry answered (`Table.expire`): the layer's answer, or the
 *  shell's before it — and, when the delete was dispatched, its batch and its
 *  save's outcome. */
interface Expired {
  status: number;
  body: string;
  done?: Committed;
  outcome?: Outcome;
  plan?: Plan;
}

/** What a batch does to `archive_owed` (spec §2.6): a plan owed, with when
 *  its alarm must run; or a save discharged — stamped, no longer owed, or a
 *  `conflict`, final at every site — and cleared. */
type ArchiveWrite = { owe: ArchiveOwed; due: number } | "clear";

/**
 * The plan `p`'s call deferred — game over's put, which no call runs again —
 * owed from the call's batch at the reading pinned for that call (the plan's
 * ruling P-2), its first retry at the first rung, 30 s; `undefined` when the
 * call deferred none.
 */
function owing(p: Pinned | null): ArchiveWrite | undefined {
  const deferred = p?.reply.archive;
  if (p === null || deferred === undefined) return undefined;
  return { owe: { plan: parsePlan(deferred), reading: p.at, attempt: 0 }, due: Date.now() + nextRetry(0) };
}

/** `meta.archive_owed` — the shell's own JSON, the plan's record and table
 *  strings inside it — or `undefined` when no save is owed. */
function archiveOwedOf(sql: SqlStorage): ArchiveOwed | undefined {
  const text = getMeta(sql, "archive_owed");
  return text === undefined ? undefined : (JSON.parse(text) as ArchiveOwed);
}

/** `meta.marker` — the shell's own JSON — or `undefined` when none stands. */
function markerOf(sql: SqlStorage): Marker | undefined {
  const text = getMeta(sql, "marker");
  return text === undefined ? undefined : (JSON.parse(text) as Marker);
}

/** `meta.expiry_failed`: the table's expiries in a row that did not end it —
 *  0 when none has failed. */
function expiryFailedOf(sql: SqlStorage): number {
  return Number(getMeta(sql, "expiry_failed") ?? "0");
}

/** Whether two load traps in a row have poisoned the table (spec §2.4). */
function poisoned(sql: SqlStorage): boolean {
  return Number(getMeta(sql, "load_traps") ?? "0") >= 2;
}

/**
 * The creator of the table `mod` holds under `id` — whom an expiry reads the
 * table as (`deleting`) — off its HOUSE COPY: the table document with its seed
 * zeroed (spec §2.5's scrub), so it may be parsed, where the table document
 * itself never is (the plan's strings rule; the orchestrator's ruling I-64).
 * `undefined` when no table is stored, or the module refuses the copy, as it
 * refuses a stored document that does not parse or that `check_shape`
 * rejects. A trap is the caller's: it runs under their `guarded`.
 */
function creatorIn(mod: Exports, id: string): string | undefined {
  let copy: string | undefined;
  try {
    copy = mod.house_copy(id);
  } catch (e) {
    if (typeof e === "string") return undefined;
    throw e;
  }
  if (copy === undefined) return undefined;
  const createdBy = (JSON.parse(copy) as { created_by?: unknown }).created_by;
  return typeof createdBy === "string" ? createdBy : undefined;
}
