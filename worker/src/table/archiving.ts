import { moduleConfig } from "../config";
import { Instance, pinnedTo, type Bump } from "../module";
import { executePlan, logOutcome, parsePlan, parseStamp, settledArm, type ArchiveOwed, type Outcome } from "../r2";
import { log } from "../respond";
import { nextRetry } from "../slot";
import { getMeta, setMeta } from "../storage";
import type { Called, Committed, Pinned, TableParts } from "../table";
import { clearLoadTraps, readLayerDocs } from "./shared";

/**
 * **The owed save** (spec §2.6's game-over site), mixed into the table object
 * over its base (`TableBase`, `src/table.ts`; M6.5c2 cycle D, Task 9): the ONE
 * delivery routine (`deliverArchive`) with its gate (R-D6), the upload, what
 * its outcome settles and the stamp, the retry on the backoff, and the switch.
 */
export function withArchiving<T extends TableParts>(Base: T) {
  abstract class Archiving extends Base {
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
     * request past the gate, or the next alarm, resumes it.
     *
     * **The gate** (R-D6, R-6.5c2.30's "a short backoff after a failure"): on
     * the REQUEST path (`fromRequest`) it returns at once while `not_before`,
     * which a failure sets at the rung its alarm is armed for, has not passed —
     * so while R2 is down a request pays a read of the owed row here and
     * nothing else, and the save is attempted once per rung, by the alarm or
     * the first request after it. The ALARM path ignores the gate: the alarm IS
     * the rung. Game over's first attempt meets no gate, since no failure has
     * set one.
     */
    protected async deliverArchive({ fromRequest }: { fromRequest: boolean }): Promise<void> {
      const id = this.ctx.id.name;
      const sql = this.ctx.storage.sql;
      const owed = archiveOwedOf(sql);
      if (id === undefined || owed === undefined || "error" in this.config) return;
      if (fromRequest && owed.not_before !== undefined && Date.now() < owed.not_before) return;
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
     * - **`landed`, `identical` or `conflict`**: the module — woken first when it
     *   is new — takes the outcome as its receipt, a conflict's with the record
     *   the key held (R-6.5c2.11), and stamps with the layer's own code at the
     *   RECORDED reading (`stamp`), so its put makes the stored plan and meets
     *   the receipt. What the two leave together decides (`settledArm`):
     *   `stamped` completes the save and clears `archive_owed` — after a
     *   conflict, only where the layer found the key holding an earlier
     *   completion of THIS game (the same seed, configuration and choices), which
     *   it compares in Rust, never here; `no-longer-over` clears it too; `final`
     *   — every conflict the layer did not stamp, whatever the stamp answered —
     *   clears it and retries nothing, the M6.5c spec §2.6 arm kept for all of
     *   them (spec §2.5, §10.5). Among them: another game's record at the key,
     *   which only a caller-chosen id can reach, or a text that is no record,
     *   which a retry would meet again while the key holds it (nothing this
     *   shell runs deletes an archived key; a key under `test/` alone expires,
     *   after 30 days); rarely, a record of this game whose stamp's write failed
     *   or whose put met no receipt, which a retry might have landed and the
     *   rule leaves final all the same; and a stamp the layer refused, which no
     *   retry of this save lands (`settledArm` names every case, and why);
     *   `retry` — after a `landed` or `identical` upload, the stamp refused (a
     *   reply that is no 200, logged `stamp-refused`) or not landed with the
     *   game over — keeps it owed, counted, its plan the one the stamp re-made
     *   at that reading when the module deferred one, and gated at its retry as
     *   a failed upload is (`not_before`, R-D6). A refused stamp is never
     *   thrown, and after a `landed` or `identical` upload it is retried on the
     *   backoff: the layer's foreign-bot rule refuses a game not over against a
     *   bot this build does not run, a 409, until the creator's Abandon or the
     *   table's expiry — neither of which rebuilds — clears the save (spec
     *   §2.4). The stamped table row, `archive_owed` and the slot the changed
     *   copy owes go in ONE batch (`commit`). A trap is the side channel's (spec
     *   §2.4): logged, the module discarded, and the save retried.
     * - **A plan the wake deferred** — a new module's `restore_picker` pumping a
     *   bot run to game over — is owed rather than a clear, as every reply's
     *   plan is (the plan's ruling P-2). It is unreachable today: a save is
     *   owed only for a game already over, so the wake here finds it over, or —
     *   the undo case — back at a human's decision, and owes no bot run. Were
     *   a pump to move the record there, the receipt's plan would no longer
     *   match. After a `landed` or `identical` upload the stamp would retry,
     *   and the plan it re-made — of that same stored record, at the recorded
     *   reading — is the one owed; after a `conflict` its
     *   `{stamped: false, over: true}` is `final`, and what is owed is the
     *   wake's own plan, at the wake's reading (`owing`).
     *
     * **The undo case** (the orchestrator's ruling I3; Task 6's escalation e;
     * made whole by R-6.5c2.11). An undo at a one-person table that lands during
     * the upload's I/O leaves the game not over at the stamp: nothing is owed,
     * and the first completion's copy stays in the bucket unstamped. A replay to
     * the SAME end meets it as `identical`, and stamps. A replay to a DIFFERENT
     * end meets it as a `conflict` carrying that first completion — the same
     * game — and stamps too: the archive keeps the first ending, and the table
     * is abandonable and expires as any stamped table does
     * (`test/archive.test.ts`, the refinish race). Two completions' uploads can
     * OVERLAP — `uploads` keys each by its plan, and two endings are two plans —
     * and then one completion's record can land beside the other's table: R2
     * has no cross-key atomicity, and that pair is best-effort (spec §2.5).
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
        // Only a 200 carries the two booleans; any other reply is the layer's
        // refusal in its error shape — a foreign bot's 409 among them — which
        // is logged and read as no stamp, never parsed (a throw here would
        // escape the upload, and an alarm with it).
        const stamp = status === 200 ? parseStamp(body) : undefined;
        if (stamp === undefined) log("stamp-refused", { id, status });
        else log("archive-stamped", { id, record_key: current.plan.record_key, reading: current.reading, ...stamp });
        const arm = settledArm(outcome, stamp);
        const retry = arm === "retry" ? this.retryAt(current) : undefined;
        done = this.commit(
          id,
          completed,
          woken,
          retry !== undefined
            ? {
                owe: {
                  plan: replanned === undefined ? current.plan : parsePlan(replanned),
                  reading: current.reading,
                  attempt: current.attempt + 1,
                  not_before: retry,
                },
                due: retry,
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
     * installed as the receipt, a conflict's with the record the key held
     * (R-6.5c2.11) — the outcome's JSON as `executePlan` built it, the held
     * record a string inside it and never parsed here — `stamp_archive(id)`
     * pinned to the RECORDED reading (`pinnedTo`, which restores the setting
     * after it), and the layer's documents read back — under ONE `guarded`.
     * Answers the call, whatever its status (`settleUpload` reads a reply that
     * is no 200 as a refused stamp), or `undefined` when it trapped, logged.
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
     * `owed` still owed, one more failure counted and its gate (`not_before`)
     * set to its retry (`retryAt`), in ONE batch — with a load trap's reset when
     * this turn's wake cleared one — and the alarm re-armed to that same retry,
     * unless a load trap since has poisoned the table, which re-arms nothing
     * (spec §2.4). A request before the retry uploads nothing (R-D6).
     */
    private retryArchive(owed: ArchiveOwed, clearTraps = false): void {
      const sql = this.ctx.storage.sql;
      const poisoned = Number(getMeta(sql, "load_traps") ?? "0") >= 2;
      const due = this.retryAt(owed);
      this.batch(
        (sql) => {
          setMeta(sql, "archive_owed", JSON.stringify({ ...owed, attempt: owed.attempt + 1, not_before: due }));
          if (clearTraps) clearLoadTraps(sql);
        },
        poisoned ? undefined : due,
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
    protected archiveOn(): boolean {
      return !("error" in this.config) && this.config.archive === "r2";
    }

    /** An owed save's retry, from now — at the rung after its failures, or the
     *  first rung when none has failed — while the switch reads `r2`, and
     *  `undefined` when none is owed or the switch reads `off`: while off, the
     *  alarm is not re-armed for it (spec §2.6). */
    protected archiveRung(sql: SqlStorage): number | undefined {
      const owed = this.archiveOn() ? archiveOwedOf(sql) : undefined;
      return owed === undefined ? undefined : Date.now() + nextRetry(Math.max(owed.attempt - 1, 0));
    }
  }
  return Archiving;
}

/** What a batch does to `archive_owed` (spec §2.6): a plan owed, with when
 *  its alarm must run; or a save discharged — stamped, no longer owed, or a
 *  `conflict`, final at every site — and cleared. */
export type ArchiveWrite = { owe: ArchiveOwed; due: number } | "clear";

/**
 * The plan `p`'s call deferred — game over's put, which no call runs again —
 * owed from the call's batch at the reading pinned for that call (the plan's
 * ruling P-2), its first retry at the first rung, 30 s; `undefined` when the
 * call deferred none.
 */
export function owing(p: Pinned | null): ArchiveWrite | undefined {
  const deferred = p?.reply.archive;
  if (p === null || deferred === undefined) return undefined;
  return { owe: { plan: parsePlan(deferred), reading: p.at, attempt: 0 }, due: Date.now() + nextRetry(0) };
}

/** `meta.archive_owed` — the shell's own JSON, the plan's record and table
 *  strings inside it — or `undefined` when no save is owed. A row without
 *  `not_before` — one no failure has gated, or one a build before R-D6 wrote
 *  — reads as ungated: `deliverArchive` tests the field for `undefined`. */
export function archiveOwedOf(sql: SqlStorage): ArchiveOwed | undefined {
  const text = getMeta(sql, "archive_owed");
  return text === undefined ? undefined : (JSON.parse(text) as ArchiveOwed);
}
