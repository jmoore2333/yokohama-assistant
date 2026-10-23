import type { Exports, Reply } from "../../gen/factory.js";
import { UNCONFIRMED } from "../admin";
import type { ShellConfig } from "../config";
import {
  expiryDays,
  expiryDeadline,
  expiryReason,
  gameOf,
  keptOf,
  markerSentence,
  openTtlDaysOf,
  ttlDaysOf,
  type Ending,
  type Reason,
} from "../marker";
import { pinnedTo, readReply, type Bump, type Instance, type Replied } from "../module";
import { executePlan, logOutcome, parsePlan, type Outcome, type Plan, type Site } from "../r2";
import { callerDigest, redactKey } from "../redact";
import { errorBody, log, respond } from "../respond";
import { nextRetry } from "../slot";
import { clearDocs, clearMetaBut, deleteMeta, getMeta, hasDoc, hasTables, readDocs, setMeta } from "../storage";
import type { Called, Committed, Incoming, Pinned, TableParts } from "../table";
import { archiveOwedOf, owing } from "./archiving";
import { differs, lastActive, slotState } from "./delivery";
import { clearLoadTraps, markerOf, readLayerDocs, TRAP } from "./shared";

/** A create's 409 at an id whose object holds a MARKER, or a REMOVAL the
 *  house has not acknowledged (spec §2.4; host-written, §4). */
const USED_RECENTLY = "this name was used recently — try another";

/**
 * **The table's end** (spec §2.4, §2.6, §2.9), mixed into the table object
 * over its base (`TableBase`, `src/table.ts`; M6.5c2 cycle D, Task 9): the
 * answer an ENDED table gives (`ended`), the creator's abandon and the receipt
 * loop every delete through the layer runs (`deleting`), the expiry — the
 * alarm's and an operator's — and the operator's force.
 */
export function withLifecycle<T extends TableParts>(Base: T) {
  abstract class Lifecycle extends Base {
    /**
     * The answer an ENDED table gives before anything is read, writing nothing
     * (spec §2.4): while a MARKER stands, every request is 404 with the
     * marker's sentence — an upgrade too, answered unupgraded, so the client's
     * probe reads it (spec §2.7) — and a create at the id is 409; with no
     * marker, a create at an id whose REMOVAL the house has not acknowledged —
     * a force's — is 409 too, so a new table's first copy can never overwrite a
     * removal the house has not received. `undefined` for every other request.
     */
    protected ended(create: boolean): Response | undefined {
      const sql = this.ctx.storage.sql;
      const marker = markerOf(sql);
      if (marker !== undefined) return create ? respond(409, errorBody(USED_RECENTLY)) : respond(404, errorBody(markerSentence(marker)));
      if (create && slotState(sql).slot?.copy === null) return respond(409, errorBody(USED_RECENTLY));
      return undefined;
    }

    /**
     * **The abandon** — the table's own DELETE (spec §2.4 step 1, §2.6;
     * `abandons`: since M6.5c2 cycle D a chat message's DELETE is not one) — run WHOLE under
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
    protected async abandon(incoming: Incoming): Promise<Response> {
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
     * 1. P-2's reading, taken ONCE, and under ONE `guarded`: the table's house
     *    copy, read first at both sites (`copyFacts`) — its stamp, and an
     *    expiry's viewer, the creator it names; then, pinned to that reading,
     *    the call that decides whether there was a game (the orchestrator's
     *    ruling I-64) — `events(id, viewer, "")` as the table's creator, before
     *    anything is deleted — and the delete's own first dispatch; then the
     *    layer's documents read back. The `events` call shares the dispatch's
     *    P-2 reading, so a delete still takes one;
     * 2. when that dispatch deferred a put — the layer refuses the delete,
     *    deleting nothing, until its save is made — the plan is carried out
     *    (`executePlan`) and its outcome logged, then installed as the receipt,
     *    and the SAME call dispatched AGAIN at the SAME reading (`redispatch`),
     *    so the layer's put makes the very plan the receipt is for. Its answer
     *    is the answer: a delete that proceeds, or the layer's own refusal
     *    naming the archive with R2's reason inside it — a `conflict` among
     *    them, final at every site;
     * 3. the ENDING the delete's batch writes as its marker, should the layer
     *    have deleted the table: `kept` from the copy's stamp, the save's
     *    outcome, the reading and the archive's switch (`gameOf`, `keptOf`) — a
     *    table stamped, or saved by this delete, kept `archive` whatever the
     *    switch reads (the orchestrator's ruling I-71) — never from the table
     *    document, which is never parsed.
     *
     * A delete that owes no save — an open or dealing table, one with no move
     * made, one already stamped, the archive off, a caller the layer refuses —
     * completes in its first dispatch. A trap is the trap 500, persisting
     * nothing.
     *
     * The viewer is the table's creator whenever the layer deletes: the
     * abandon's caller, whom the layer's `delete` refuses unless they created
     * the table, and for an expiry the creator its house copy names. Its
     * `events` call answers 200 but for a stored record that parses and will
     * not replay — written at another `SEMANTICS_REV`, or corrupted — which
     * `events` must rebuild at a playing table where the layer's delete reads
     * no further than its JSON. That reading is the layer's error and the
     * delete proceeds, so `gameOf` falls back to the stored record, which
     * reads as a game, even with no event in it: `kept` is then `archive`
     * where the table was stamped, the save landed or the switch reads `r2`,
     * and `off` otherwise. A copy the module refuses names no creator and no
     * stamp, and no stored table reaches that where the layer deletes:
     * `house_copy` refuses a stored document the layer's own delete also
     * refuses to read (both run `check_shape` over the same stored text), or a
     * copy its scrub broke, which the module's own cells show no stored
     * document reaches.
     *
     * Whether a record is stored, the fallback `gameOf` reads, is asked of ONE
     * row where `kept` is computed (`hasDoc`; Task 14's L-8): the loop reads no
     * document of its own.
     */
    private async deleting(id: string, instance: Instance, config: ShellConfig, how: Deleting): Promise<Deleted | Response> {
      // Both sites read the house copy: its stamp, and an expiry's viewer.
      let stage = "house_copy";
      const done = instance.guarded((mod) => {
        const at = this.reading();
        const copy = copyFacts(mod, id);
        const viewer = how.viewer(copy);
        // The pin is the reading's, as `pinned` counts a pin with its call.
        stage = "events";
        mod.set_clock(at);
        const reading = viewer === undefined ? undefined : readReply(mod.events(id, viewer, ""));
        stage = how.call;
        const reply = readReply(how.f(mod));
        stage = "read";
        return { copy, reading, first: { reply, at, docs: readLayerDocs(mod, id) } };
      });
      if ("trapped" in done) {
        log("trap", { id, call: stage, error: done.trapped });
        return respond(500, errorBody(TRAP));
      }
      const { copy, reading, first } = done.ok;
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
      const kept = keptOf({
        stamped: copy?.stamped ?? false,
        saved: outcome?.outcome === "landed" || outcome?.outcome === "identical",
        game: gameOf(reading, hasDoc(this.ctx.storage.sql, "record")),
        archive: config.archive,
      });
      return {
        called,
        reading: reading === undefined ? null : { reply: reading, at: first.at },
        outcome,
        plan,
        ending: { ...how.reason, kept },
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
     * has run out or a table has waited out the waiting-table TTL (`expiry`,
     * R-6.5c2.42), and an operator's plain expire (`operate`) — as
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
        const alarmed = reason.reason === "expired" || reason.reason === "waited";
        const deleted = await this.deleting(id, instance, config, {
          site: alarmed ? "expiry" : "operator",
          reason,
          viewer: (copy) => copy?.createdBy,
          call: "expire",
          f: (m) => m.expire(id),
          ...(alarmed ? { attempt: expiryFailedOf(this.ctx.storage.sql) } : {}),
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
     * The alarm's expiry (spec §2.4 "Expiry"; R-6.5c.15; R-6.5c2.42), after
     * its deliveries. With a TTL (`ttlDaysOf`) and a document stored, from the
     * table's last activity — the newest `written_at` of its record, table and
     * chat, RE-READ at every run, because the expiry deadline only moves later
     * and nothing re-arms the alarm for a write that makes nothing due:
     *
     * - a table `held` or poisoned never expires on its own (spec §2.4; the
     *   operator's force is its path): past its TTL it is named in the log,
     *   `expiry-blocked`, and no expiry deadline is armed for it;
     * - before the deadline the alarm is ARMED at — `last activity` plus the
     *   shorter of the two TTLs (`expiryDays`), since no batch can read the
     *   table's status — an alarm armed for other work, or at a deadline a
     *   later activity has since moved, is re-armed to the table's next
     *   deadline (`nextDue`), and a count of failed expiries that later
     *   activity has made stale is dropped;
     * - past it, the table's STATUS decides (the plan's R-E8): read off its
     *   house copy (`waiting`, through `factsOf`, `copyFacts`'s parse), a
     *   table that has not started — `open`, or `dealing` with a keep
     *   outstanding (security S3, D9) — is closed as one that
     *   `waited` once the waiting-table TTL is past, and any table `expired`
     *   once the TTL is (`expiryReason`); a table that is neither is
     *   re-armed with the TTL itself as its expiry's deadline (`nextDue`,
     *   `waiting` read `false`), so the alarm does not fire again at a
     *   deadline it has already decided;
     * - a table to end expires (`expire`) under `blockConcurrencyWhile`, and
     *   an expiry that did not end it is settled (`unexpired`) before the
     *   block ends. The delete's side channels — its removal's push, an owed
     *   save's upload — follow the block as a request's follow its reply
     *   (`waitUntil`, `finish`), and the alarm armed in the delete's batch
     *   finishes them if they cannot.
     */
    protected async expiry(id: string): Promise<void> {
      const sql = this.ctx.storage.sql;
      const config = this.config;
      if ("error" in config || !hasTables(sql)) return;
      const ttl = ttlDaysOf(config);
      const activity = lastActive(sql);
      if (ttl === null || activity === null) return;
      const openTtl = openTtlDaysOf(config);
      const blocked = getMeta(sql, "held") !== undefined ? "held" : poisoned(sql) ? "poisoned" : undefined;
      if (blocked !== undefined) {
        if (Date.now() >= expiryDeadline(activity, ttl)) log("expiry-blocked", { id, why: blocked });
        return;
      }
      if (Date.now() < expiryDeadline(activity, expiryDays(ttl, openTtl, undefined))) {
        if (expiryFailedOf(sql) > 0) this.batch((sql) => deleteMeta(sql, "expiry_failed"));
        this.arm(this.nextDue(sql));
        return;
      }
      const waiting = this.waiting(id);
      const reason = expiryReason(Date.now(), activity, ttl, openTtl, waiting);
      if (reason === undefined) {
        this.arm(this.nextDue(sql, waiting));
        return;
      }
      const done = await this.ctx.blockConcurrencyWhile(async () => {
        const expired = await this.expire(id, reason);
        if (expired.done?.marker === undefined) this.unexpired(id, expired);
        return expired.done;
      });
      if (done?.pushing) this.ctx.waitUntil(this.deliverSlot());
      if (done?.archiving) this.ctx.waitUntil(this.deliverArchive({ fromRequest: false }));
    }

    /**
     * Whether the table has not started (R-6.5c2.42; security S3, D9) — open,
     * or dealing with a keep outstanding: its status `open` or `dealing`, as
     * its house copy says it — read by `factsOf`, the parse `copyFacts` makes
     * and the strings rule's one sanctioned parse — the copy made from the
     * STORED table document on a module of its own (`copyOf`), so this
     * object's module — its wake, and whatever a wake owes — is untouched and
     * nothing is written. A copy that cannot be made — no table document, a
     * refusal, a trap — reads as not waiting, so the TTL itself decides: the
     * alarm's `recopy`, which ran first, made a copy from the stored document
     * too and logged any failure.
     *
     * The stored document is read as it stands, never settled first: a table
     * finalized but unflipped — every keep made, its record stored, its
     * status still `dealing` — reads as waiting here and, if nobody read it
     * for the whole waiting-table TTL, ends as one that `waited`, where the
     * NAS's sweep settles first and keeps it (design §3.2 rule 2, D-16).
     */
    private waiting(id: string): boolean {
      const table = readDocs(this.ctx.storage.sql).get("table");
      if (table === undefined) return false;
      const made = this.copyOf(id, table);
      if (!("copy" in made) || made.copy === undefined) return false;
      const status = factsOf(made.copy).status;
      return status === "open" || status === "dealing";
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
        log("held", { id, record_key: redactKey(recordKey) });
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
     * `operator` with the action and the operator's digest (security S2b, D23).
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
    protected async operate(id: string, address: string, force: boolean): Promise<Response> {
      log("operator", { id, action: force ? "force" : "expire", operator: await callerDigest(address) });
      if (force) return this.force(id);
      const marker = markerOf(this.ctx.storage.sql);
      if (marker !== undefined) return respond(404, errorBody(markerSentence(marker)));
      const expired = await this.ctx.blockConcurrencyWhile(() => this.expire(id, { reason: "operator" }));
      if (expired.done?.pushing) this.ctx.waitUntil(this.deliverSlot());
      if (expired.done?.archiving) this.ctx.waitUntil(this.deliverArchive({ fromRequest: true }));
      return respond(expired.status, expired.body);
    }

    /**
     * **The operator's `?force`** (spec §2.9): the object ends WHATEVER it holds
     * — a table, a poisoned one, a `held` one, a marker — with no layer and no
     * save, and it leaves NO marker.
     *
     * - With nothing stored: 204 at once, writing nothing.
     * - Where the object holds no generation — no copy of the table was ever
     *   drawn — or the house has acknowledged the table's removal, the object
     *   ends at once (`end`, told it is a force's), and 204. A table whose
     *   first copy trapped holds a generation all the same and takes the push
     *   below, which the house acknowledges although it never held the row.
     * - Otherwise ONE batch deletes every document, and every `meta` key but
     *   the slot's own four — the marker, an owed save and `held` among them —
     *   writes the REMOVAL, unless one already waits, with its retry's alarm,
     *   and flags the object as forced (`meta.forced`, its time in
     *   milliseconds), so the end that REMOVAL's acknowledgement brings, in
     *   this request or a later alarm, is logged `forced` (Task 14's L-4). The
     *   module is dropped and every socket ended as a delete ends them, and the
     *   removal is pushed IN the request (`deliverSlot`): its acknowledgement,
     *   with no marker standing, ends the object (`acknowledged`). The answer
     *   is 204 once the storage is gone, or 503
     *   `the lobby has not confirmed the removal — the alarm will finish it`
     *   when the push failed — and the alarm, re-armed by that failure, does.
     *   Until then a create at the id is the 409 of a removal the house has not
     *   acknowledged (`ended`).
     *
     * An owed save it drops is logged `archive-owed-dropped`, as a delete's is,
     * once it is gone and before `forced` on both paths: after the batch that
     * drops it, or by `end`.
     * It holds no `blockConcurrencyWhile`: a request that lands while the push
     * is out finds no document to answer from, and every push and
     * acknowledgement is keyed by its pair (spec §2.5).
     */
    private async force(id: string): Promise<Response> {
      const sql = this.ctx.storage.sql;
      if (!hasTables(sql)) return respond(204, "");
      const state = slotState(sql);
      if (state.generation === undefined || (state.slot === undefined && !differs(state, null))) {
        this.endSockets(id);
        await this.end(undefined, true);
        return respond(204, "");
      }
      const owed = archiveOwedOf(sql);
      const removal = state.slot?.copy === null ? undefined : { generation: state.generation, revision: state.revision + 1, copy: null };
      this.batch(
        (sql) => {
          clearDocs(sql);
          clearMetaBut(sql, ["generation", "revision", "slot", "acked"]);
          // The REMOVAL is a force's: `end` logs it `forced` (Task 14's L-4).
          setMeta(sql, "forced", String(Date.now()));
          for (const write of this.owedWrites(id, { draw: undefined, slot: removal, traps: 0 })) write();
        },
        removal === undefined ? undefined : Date.now() + nextRetry(0),
      );
      this.instance?.drop();
      this.endSockets(id);
      if (owed !== undefined) log("archive-owed-dropped", { id, record_key: redactKey(owed.plan.record_key), attempt: owed.attempt });
      await this.deliverSlot();
      return hasTables(sql) ? respond(503, errorBody(UNCONFIRMED)) : respond(204, "");
    }
  }
  return Lifecycle;
}

/** How a delete through the layer is made (`Table.deleting`). */
interface Deleting {
  /** Where its save's plan is made, as the outcome's line names it. */
  site: Site;
  /** How the table ends, should the layer delete it. */
  reason: Reason;
  /** Whom the reading that decides `kept` is taken as — the table's creator —
   *  given what its house copy says (`copyFacts`). */
  viewer: (copy: CopyFacts | undefined) => string | undefined;
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

/** `meta.expiry_failed`: the table's expiries in a row that did not end it —
 *  0 when none has failed. */
export function expiryFailedOf(sql: SqlStorage): number {
  return Number(getMeta(sql, "expiry_failed") ?? "0");
}

/** Whether two load traps in a row have poisoned the table (spec §2.4). */
export function poisoned(sql: SqlStorage): boolean {
  return Number(getMeta(sql, "load_traps") ?? "0") >= 2;
}

/** What a table's HOUSE COPY says that a delete through the layer reads
 *  (`copyFacts`), and the alarm's expiry (`waiting`). */
interface CopyFacts {
  /** The table's creator — `created_by` — whom an expiry reads it as. */
  createdBy: string | undefined;
  /** The table's save was made: `archived_at` is set (the orchestrator's
   *  ruling I-71). */
  stamped: boolean;
  /** The table's `status` — `open`, `dealing` or `playing` — which decides
   *  whether it has not started: `open` or `dealing` (R-6.5c2.42; security
   *  S3, D9). */
  status: string | undefined;
}

/**
 * What the table `mod` holds under `id` says of itself on its HOUSE COPY — the
 * table document through spec §2.5's scrub (`factsOf`). `undefined` when no
 * table is stored, or the module refuses the copy, as it refuses a stored
 * document that does not parse or that `check_shape` rejects. A trap is the
 * caller's: it runs under their `guarded`.
 */
function copyFacts(mod: Exports, id: string): CopyFacts | undefined {
  let copy: string | undefined;
  try {
    copy = mod.house_copy(id);
  } catch (e) {
    if (typeof e === "string") return undefined;
    throw e;
  }
  return copy === undefined ? undefined : factsOf(copy);
}

/**
 * A HOUSE COPY's facts — the copy `house_copy` made, off this object's module
 * (`copyFacts`) or off a module of its own (`copyOf`, for `waiting`) — which
 * the table object parses HERE and nowhere else, where the table document
 * itself is never parsed (the plan's strings rule; the orchestrator's rulings
 * I-64 and I-71; R-6.5c2.42): its creator, whom an expiry reads the table as;
 * whether its save was made; and its status. A copy carries no seed, and
 * since M6.5c2 may carry a bot descriptor whose `weights` is above 2^53, which
 * this parse rounds; it may be parsed because nothing here reads `params` and
 * no parsed copy is written back.
 */
function factsOf(copy: string): CopyFacts {
  const parsed = JSON.parse(copy) as { created_by?: unknown; archived_at?: unknown; status?: unknown };
  return {
    createdBy: typeof parsed.created_by === "string" ? parsed.created_by : undefined,
    stamped: typeof parsed.archived_at === "number",
    status: typeof parsed.status === "string" ? parsed.status : undefined,
  };
}
