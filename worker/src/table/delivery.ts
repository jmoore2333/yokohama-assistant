import { earliest } from "../alarm";
import { markerEnd } from "../marker";
import { log } from "../respond";
import { advances, cmpGen, copyDigest, HOUSE_NAME, nextRetry, type Ack, type Generation, type Slot } from "../slot";
import { deleteMeta, getMeta, hasTables, readDocs, setMeta } from "../storage";
import type { TableParts } from "../table";
import { markerOf } from "./shared";

/**
 * **The slot's delivery** (spec §2.5), mixed into the table object over its
 * base (`TableBase`, `src/table.ts`; M6.5c2 cycle D, Task 9): the ONE delivery
 * routine (`deliverSlot`), its push to the house and what the push's answer
 * owes — the acknowledgement's batch, or the failure's retry — and the
 * alarm's house copy, recomputed from the stored table (`recopy`).
 */
export function withDelivery<T extends TableParts>(Base: T) {
  abstract class Delivery extends Base {
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
     *   (m-g of the plan `docs/plans/2026-09-23-web-frontend-m6.5c-plan.md`) — and `deleteAll()` drops the tables;
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
    protected recopy(id: string): void {
      const sql = this.ctx.storage.sql;
      const config = this.config;
      if ("error" in config) {
        log("init-refused", { id, error: config.error });
        return;
      }
      if (!hasTables(sql) || Number(getMeta(sql, "load_traps") ?? "0") >= 2) return;
      const table = readDocs(sql).get("table");
      if (table === undefined) return;
      const owed = this.owed(id, table);
      const writes = this.owedWrites(id, owed);
      if (writes.length === 0) return;
      this.batch(() => {
        for (const write of writes) write();
      }, earliest(owedDue(owed), this.archiveRung(sql)));
    }
  }
  return Delivery;
}

/** What `acked` holds (spec §2.4): the pair last acknowledged, and the
 *  digest of its copy (`copyDigest`) — `null` for a REMOVAL. */
type Acked = Ack & { digest: string | null };

/** What a change of the table document owes the house (`Table.owed`). */
export interface Owed {
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

export function slotState(sql: SqlStorage): SlotState {
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
export function differs(state: SlotState, copy: string | null): boolean {
  if (state.slot !== undefined) return copy !== state.slot.copy;
  if (state.acked === undefined) return true;
  return (copy === null ? null : copyDigest(copy)) !== state.acked.digest;
}

/** A new generation at `at` — the shell's `Date.now()`, in milliseconds
 *  (`Table.owed`) — with 64 random bits. */
export function drawGeneration(at: number): Generation {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  return { at, rand: [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("") };
}

/** The table's last activity (spec §2.4): the newest `written_at` of its
 *  record, table and chat — `null` when none is stored. */
export function lastActive(sql: SqlStorage): number | null {
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
export function copyOwedOf(sql: SqlStorage): number {
  return Number(getMeta(sql, "copy_owed") ?? "0");
}

/** An owed copy's retry, from now, at the rung after its `traps`-th trap in
 *  a row — `null` when none is owed. */
export function copyRetryDue(traps: number): number | null {
  return traps > 0 ? Date.now() + nextRetry(traps - 1) : null;
}

/** When the alarm must run for what `owed` leaves due: a new slot's first
 *  retry (30 s), or an owed copy's retry — `undefined`, the alarm left where
 *  it is, when it leaves nothing due. */
export function owedDue(owed: Owed): number | undefined {
  if (owed.slot !== undefined) return Date.now() + nextRetry(0);
  return copyRetryDue(owed.traps) ?? undefined;
}
