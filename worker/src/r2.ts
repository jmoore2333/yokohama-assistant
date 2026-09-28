import { log } from "./respond";

/**
 * **The archive's bucket side** (spec §2.6, R-6.5c.9): what the shell does with
 * a plan the module deferred, how it reads what the module answers, and the
 * line it logs for each upload. It touches the bucket it is handed and no
 * other Workers API, so its cells run in node over a bucket in memory
 * (`r2.unit.test.ts`); the pool proves the conditional put against
 * miniflare's R2.
 *
 * The module boundary is STRINGS (the orchestrator's ruling I-25): a plan
 * arrives as the JSON `Reply.archive` carries, and an outcome leaves as the
 * JSON `receipt` takes. A plan's `record` and `table` are the exact texts the
 * module wrote, and are only ever compared and uploaded here, never parsed: a
 * record carries a seed above 2^53.
 */

/** A deferred put, as `Reply.archive` carries it (the plan's names table):
 *  the key a receipt for it names, the two keys to write, and the two bodies
 *  as the layer handed them over. */
export type Plan = { key: string; record_key: string; table_key: string; record: string; table: string };

/** An upload's outcome, as `receipt` takes it (the plan's names table). The
 *  `detail` is MANDATORY — the module refuses a receipt without one (Task 6's
 *  review, escalation c): the record's key for `landed`, `identical` and
 *  `conflict`; for `failed`, R2's message — or, in the one case
 *  `executePlan` names, the host's own sentence. A `conflict` also carries
 *  `held`, the record the key holds, as its exact TEXT (M6.5c2, R-6.5c2.11):
 *  the module's receipt hands it to the layer, whose refinish rule compares
 *  it, in Rust; the module refuses `held` beside any other outcome. The shell
 *  never parses it (a record carries a seed above 2^53), and never logs it
 *  (`logOutcome` names the detail alone). */
export type Outcome = { outcome: "landed" | "identical" | "conflict" | "failed"; detail: string; held?: string };

/** `meta.archive_owed` (spec §2.4, §2.6): a game-over plan the layer did not
 *  re-dispatch, the P-2 reading pinned for the call that made it, and how many
 *  of its uploads have failed. */
export type ArchiveOwed = { plan: Plan; reading: number; attempt: number };

/** `stamp_archive`'s 200 (the orchestrator's ruling I3, refined at Task 6):
 *  whether the stamp LANDED, and whether the game is over at retry time. */
export type Stamp = { stamped: boolean; over: boolean };

/** The two calls `executePlan` makes of R2's bucket binding. */
export type Bucket = Pick<R2Bucket, "get" | "put">;

/** Where a plan was made: the game-over site's owed save; the receipt loop
 *  of a delete through the layer — a creator's abandon, an idle table's
 *  expiry, an operator's expire; or a browser's save through its caller's
 *  archive object. */
export type Site = "game-over" | "abandon" | "expiry" | "operator" | "browser";

/**
 * The shell's line for an upload's outcome (spec §2.8; the layer's own lines
 * reach this log through `readReply` since M6.5c2, under names of their own):
 * `archive-<outcome>`, naming the object, the site, the record's key and the
 * outcome's detail, and `extra` beside them. Never the record a conflict
 * holds: the line names the detail, and `held` stays the module's to read.
 */
export function logOutcome(id: string | undefined, site: Site, plan: Plan, outcome: Outcome, extra: Record<string, unknown> = {}): void {
  log(`archive-${outcome.outcome}`, { id, site, record_key: plan.record_key, detail: outcome.detail, ...extra });
}

/** The five fields of a plan, in the module's order. */
const PLAN_FIELDS = ["key", "record_key", "table_key", "record", "table"] as const;

/**
 * The plan `Reply.archive` carries — the module's five strings, and nothing
 * else — or a throw naming the field that is not. The sentence never echoes
 * the plan: its record is the whole game.
 */
export function parsePlan(json: string): Plan {
  const plan: unknown = JSON.parse(json);
  if (plan === null || typeof plan !== "object" || Array.isArray(plan)) {
    throw new Error("the module's archive plan is not an object");
  }
  for (const field of Object.keys(plan)) {
    if (!(PLAN_FIELDS as readonly string[]).includes(field)) {
      throw new Error(`the module's archive plan carries a field it has no use for: ${JSON.stringify(field)}`);
    }
  }
  for (const field of PLAN_FIELDS) {
    if (typeof (plan as Record<string, unknown>)[field] !== "string") {
      throw new Error(`the module's archive plan carries no string ${field}`);
    }
  }
  return plan as Plan;
}

/** The text at `key` — `null` when the bucket holds nothing there. */
async function textAt(bucket: Bucket, key: string): Promise<string | null> {
  const object = await bucket.get(key);
  return object === null ? null : object.text();
}

/**
 * **A plan carried out in `DirArchive::put`'s own order** (spec §2.6):
 *
 * 1. the RECORD key read: these very bytes are `identical`, other bytes a
 *    `conflict` carrying them — either way nothing is written; absent, go on;
 * 2. the TABLE key put UNCONDITIONALLY: overwriting a half pair's table is the
 *    repair, and a re-plan's new `archived_at` lands instead of conflicting;
 * 3. the RECORD key put create-if-absent. When that precondition fails — a
 *    concurrent writer put a record between 1 and 3 — the record is read
 *    again: these very bytes `identical`, other bytes a `conflict`, carrying
 *    them. A re-read that finds NO record is `failed`, carrying the host's own
 *    sentence, "`<record_key>` refused a create as present, then read as
 *    absent";
 * 4. anything else that fails is `failed`, carrying R2's message.
 *
 * The table goes first and the record's key is the once-marker, so an upload
 * that stops half way leaves a table with no record — which the next put
 * repairs — and never a record without its table. The records are compared as
 * TEXT, and a conflict hands the held text on, unparsed, for the module to
 * compare (R-6.5c2.11). Because the record key is read BEFORE the table is
 * put, a conflict at step 1 writes nothing: this plan's table never lands
 * beside a record that is not this plan's (spec §2.5, "The archive's pair").
 * A conflict at step 3 has already put this plan's table — the race R2 has no
 * cross-key atomicity to close, recorded as best-effort (spec §2.5). The
 * conditional form is `onlyIf` with an `If-None-Match: *` header; the first
 * deploy that touches R2 reads it live (spec §3.2).
 */
export async function executePlan(bucket: Bucket, plan: Plan): Promise<Outcome> {
  const at = (outcome: "landed" | "identical"): Outcome => ({ outcome, detail: plan.record_key });
  // What the key holds, against this plan's record — as TEXT: a conflict hands
  // the held text on, for the module to compare (R-6.5c2.11).
  const against = (held: string): Outcome =>
    held === plan.record ? at("identical") : { outcome: "conflict", detail: plan.record_key, held };
  try {
    const held = await textAt(bucket, plan.record_key);
    if (held !== null) return against(held);
    await bucket.put(plan.table_key, plan.table);
    const created = await bucket.put(plan.record_key, plan.record, { onlyIf: new Headers({ "If-None-Match": "*" }) });
    if (created !== null) return at("landed");
    const raced = await textAt(bucket, plan.record_key);
    if (raced === null) {
      // The create was refused because the key held a record, and the key
      // now holds none: only a delete in between could do that, and nothing
      // this shell runs deletes an archived key. Whatever did, the pair is
      // not known to be whole, so the upload is retried as a failure.
      return { outcome: "failed", detail: `${plan.record_key} refused a create as present, then read as absent` };
    }
    return against(raced);
  } catch (e) {
    return { outcome: "failed", detail: e instanceof Error ? e.message : String(e) };
  }
}

/** `stamp_archive`'s 200 body — the module's two booleans and nothing else —
 *  or a throw naming what it is instead. It carries no seed. */
export function parseStamp(body: string): Stamp {
  const stamp: unknown = JSON.parse(body);
  const keys = stamp !== null && typeof stamp === "object" && !Array.isArray(stamp) ? Object.keys(stamp).sort() : [];
  const { stamped, over } = (stamp ?? {}) as Record<string, unknown>;
  if (keys.join() !== "over,stamped" || typeof stamped !== "boolean" || typeof over !== "boolean") {
    throw new Error(`stamp_archive's reply is not {"stamped": bool, "over": bool}: ${body}`);
  }
  return { stamped, over };
}

/**
 * What an owed save's stamp leaves (the orchestrator's ruling I3, refined at
 * Task 6):
 *
 * - `stamped`: the stamp landed, and the save is complete;
 * - `no-longer-over`: the game is not over at retry time — an undo at a
 *   one-person table after the plan was made — so nothing is owed, and a later
 *   completion puts afresh (spec §2.6);
 * - `retry`: the game is over and the stamp did not land, so the save is still
 *   owed.
 */
export function stampArm(stamp: Stamp): "stamped" | "no-longer-over" | "retry" {
  if (stamp.stamped) return "stamped";
  return stamp.over ? "retry" : "no-longer-over";
}

/**
 * What an owed save's upload and its stamp leave TOGETHER (M6.5c2,
 * R-6.5c2.11; spec §2.5):
 *
 * - a `conflict` the stamp LANDED on is `stamped`, whether or not the game is
 *   still over: the key held an earlier completion of this same game, which
 *   the layer compared in Rust, and the save is complete;
 * - a `conflict` it did not land on is `final`, whatever the stamp answered:
 *   nothing owed, and nothing retried, as the M6.5c spec §2.6 ruled every
 *   conflict and spec §2.5 and §10.5 keep for every conflict the layer did not
 *   stamp. By the stamp's answer:
 *   - `{stamped: false, over: true}`: the key holds another game's record
 *     (which only a caller-chosen id can reach) or a text that is no record,
 *     which no retry could clear, since nothing this shell runs deletes an
 *     archived key; or the conflict carried no `held`, which `executePlan`
 *     never builds and the layer reads as not the same game; or, rarely, the
 *     key holds a record of this game and the stamp did not land — its write
 *     failed, or its put met no receipt — which a retry might have landed;
 *   - `{stamped: false, over: false}`: the game is no longer over, and owes
 *     nothing, as `no-longer-over` reads it;
 *   - no 200: the stamp was refused — a foreign bot's 409 among them — which a
 *     retry might have landed;
 * - every other outcome is `stampArm`'s, and a stamp that answered no 200 is
 *   `retry`.
 */
export function settledArm(outcome: Outcome, stamp: Stamp | undefined): "stamped" | "no-longer-over" | "retry" | "final" {
  if (outcome.outcome === "conflict") return stamp?.stamped ? "stamped" : "final";
  return stamp === undefined ? "retry" : stampArm(stamp);
}
