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
 *  review, escalation c): R2's message for `failed`, the record's key for the
 *  other three. */
export type Outcome = { outcome: "landed" | "identical" | "conflict" | "failed"; detail: string };

/** `meta.archive_owed` (spec §2.4, §2.6): a game-over plan the layer did not
 *  re-dispatch, the P-2 reading pinned for the call that made it, and how many
 *  of its uploads have failed. */
export type ArchiveOwed = { plan: Plan; reading: number; attempt: number };

/** `stamp_archive`'s 200 (the orchestrator's ruling I3, refined at Task 6):
 *  whether the stamp LANDED, and whether the game is over at retry time. */
export type Stamp = { stamped: boolean; over: boolean };

/** The two calls `executePlan` makes of R2's bucket binding. */
export type Bucket = Pick<R2Bucket, "get" | "put">;

/** Where a plan was made: the game-over site's owed save, the abandon's
 *  receipt loop, or a browser's save through its caller's archive object. */
export type Site = "game-over" | "abandon" | "browser";

/**
 * The shell's line for an upload's outcome (spec §2.8; the layer's own lines
 * are mute on this host): `archive-<outcome>`, naming the object, the site,
 * the record's key and the outcome's detail, and `extra` beside them.
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
 *    `conflict` — either way nothing is written; absent, go on;
 * 2. the TABLE key put UNCONDITIONALLY: overwriting a half pair's table is the
 *    repair, and a re-plan's new `archived_at` lands instead of conflicting;
 * 3. the RECORD key put create-if-absent. When that precondition fails — a
 *    concurrent writer put a record between 1 and 3 — the record is read
 *    again: these very bytes `identical`, other bytes a `conflict`;
 * 4. anything else that fails is `failed`, carrying R2's message.
 *
 * The table goes first and the record's key is the once-marker, so an upload
 * that stops half way leaves a table with no record — which the next put
 * repairs — and never a record without its table. The records are compared as
 * TEXT. The conditional form is `onlyIf` with an `If-None-Match: *` header;
 * the first deploy that touches R2 reads it live (spec §3.2).
 */
export async function executePlan(bucket: Bucket, plan: Plan): Promise<Outcome> {
  const at = (outcome: "landed" | "identical" | "conflict"): Outcome => ({ outcome, detail: plan.record_key });
  try {
    const held = await textAt(bucket, plan.record_key);
    if (held !== null) return at(held === plan.record ? "identical" : "conflict");
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
    return at(raced === plan.record ? "identical" : "conflict");
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
