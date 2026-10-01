/**
 * The table object's SQLite (spec §2.4): two tables, and the only SQL the shell
 * runs on them. The house object keeps its chat in the same `docs` table, and
 * the chat's load-trap count in the same `meta` (spec §2.5); its copies live
 * in two tables of its own, which `house.ts` writes.
 *
 * - `docs(kind, json, written_at)` holds `record`, `table` and `chat` — each the
 *   exact text the module's `read` answered, never parsed, so a seed above 2^53
 *   keeps its digits — and the host's own `picker` row. In the house object it
 *   holds one row, `house`: the house chat, as the module's `read` answered it.
 *   `written_at` is the platform's `Date.now()` at the write (the
 *   orchestrator's ruling I-43), so a table's last activity is the newest
 *   `written_at` of the layer's three and costs no row of its own.
 * - `meta(k, v)` holds the object's own facts, under the keys `MetaKey` names.
 *   It holds no `id`: the object's name is `ctx.id.name`.
 *
 * **Neither table exists until the first write batch creates it**: every
 * batch calls `ensureTables` inside its `transactionSync`, and the constructor
 * writes nothing — so probing an id leaves `sqlite_master` empty. A read before
 * that asks `sqlite_master` (`hasTables`) and reads absence as empty.
 *
 * The writers — `upsertDoc`, `deleteDoc`, `clearDocs`, `setMeta`, `deleteMeta`,
 * `clearMetaBut` — run inside a batch's `transactionSync`, after
 * `ensureTables`.
 */

export type DocKind = "record" | "table" | "chat" | "picker" | "house";

/** Every key `meta` holds: spec §2.4's; `poison`, the kind whose load
 *  trapped (the plan's Task 9); `copy_owed`, the `house_copy` traps in a row
 *  while a copy is owed (the orchestrator's ruling on Task 12's review);
 *  `expiry_failed`, an idle table's expiries in a row that did not end it,
 *  which set its retry (Task 14); and `forced`, set by an operator's force in
 *  its batch to the time it ran (`Date.now()`, in milliseconds), so that when
 *  the REMOVAL it leaves is acknowledged, the object's end names the force
 *  (Task 14's L-4). */
export type MetaKey =
  | "generation"
  | "revision"
  | "slot"
  | "acked"
  | "copy_owed"
  | "archive_owed"
  | "marker"
  | "held"
  | "expiry_failed"
  | "load_traps"
  | "poison"
  | "wakes"
  | "forced";

/**
 * The storages whose two tables `hasTables` has FOUND (F-4's rows read; spec
 * §2.7): it answers from here after that. Its scan reads the whole of
 * `sqlite_master`, and it runs inside every `getMeta` and every `readDocs`:
 * MEASURED in the pool before this memo, six scans and 30 of a warm
 * request's 32 rows read. Only `deleteAll()` drops the tables once they exist, and its one
 * caller forgets them (`forgetTables`, from `Table.end`) — with ONE unstated
 * exception this sentence owns: `Table.batch` runs `ensureTables` inside a
 * `transactionSync` with no catch, so a `hasTables` that answered true inside
 * a batch which then throws leaves `known` naming tables the rollback removed
 * (the next batch's `ensureTables` creates them again; a read in between
 * answers from the memo). Keyed by the
 * object's `SqlStorage`, which is one object for the life of an instance and
 * a new one after an eviction (measured in the pool), so a new instance
 * scans once again.
 */
const known = new WeakSet<SqlStorage>();

/** Whether the first write batch has run. Both tables are created by one
 *  `ensureTables`, inside one transaction, so it asks for both. It names them:
 *  `sqlite_master` also lists the platform's own `_cf_METADATA` once an alarm
 *  has been set. Answered from `known` once both are found, and never
 *  remembered while they are absent: an object that holds nothing scans at
 *  every read, and the tables a later batch creates are found by the next. */
export function hasTables(sql: SqlStorage): boolean {
  if (known.has(sql)) return true;
  const { n } = sql
    .exec<{ n: number }>("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name IN ('docs', 'meta')")
    .one();
  if (n === 2) known.add(sql);
  return n === 2;
}

/** The two tables forgotten (`known`), once `deleteAll()` has dropped them
 *  (`Table.end`): the next `hasTables` scans, so a straggling acknowledgement,
 *  a force's answer and a later create read them absent. */
export function forgetTables(sql: SqlStorage): void {
  known.delete(sql);
}

export function ensureTables(sql: SqlStorage): void {
  sql.exec("CREATE TABLE IF NOT EXISTS docs (kind TEXT PRIMARY KEY, json TEXT NOT NULL, written_at INTEGER NOT NULL)");
  sql.exec("CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT NOT NULL)");
}

/** Every `docs` row's text, by kind — empty before the first write batch. */
export function readDocs(sql: SqlStorage): Map<DocKind, string> {
  if (!hasTables(sql)) return new Map();
  const rows = sql.exec<{ kind: DocKind; json: string }>("SELECT kind, json FROM docs").toArray();
  return new Map(rows.map((row) => [row.kind, row.json]));
}

/** Whether a `docs` row of `kind` is stored — `false` before the first write
 *  batch — asked of ONE row, and reading none of its text (Task 14's L-8):
 *  where a caller needs only that, `readDocs` would read every document, a
 *  record's whole game among them. */
export function hasDoc(sql: SqlStorage, kind: DocKind): boolean {
  return hasTables(sql) && sql.exec("SELECT 1 FROM docs WHERE kind = ? LIMIT 1", kind).toArray().length === 1;
}

export function upsertDoc(sql: SqlStorage, kind: DocKind, json: string, writtenAt: number): void {
  sql.exec(
    "INSERT INTO docs (kind, json, written_at) VALUES (?, ?, ?) " +
      "ON CONFLICT (kind) DO UPDATE SET json = excluded.json, written_at = excluded.written_at",
    kind,
    json,
    writtenAt,
  );
}

export function deleteDoc(sql: SqlStorage, kind: DocKind): void {
  sql.exec("DELETE FROM docs WHERE kind = ?", kind);
}

/** Every `docs` row deleted. */
export function clearDocs(sql: SqlStorage): void {
  sql.exec("DELETE FROM docs");
}

/** `k`'s value — `undefined` when it is not set, or before the first write
 *  batch. */
export function getMeta(sql: SqlStorage, k: MetaKey): string | undefined {
  if (!hasTables(sql)) return undefined;
  return sql.exec<{ v: string }>("SELECT v FROM meta WHERE k = ?", k).toArray()[0]?.v;
}

export function setMeta(sql: SqlStorage, k: MetaKey, v: string): void {
  sql.exec("INSERT INTO meta (k, v) VALUES (?, ?) ON CONFLICT (k) DO UPDATE SET v = excluded.v", k, v);
}

export function deleteMeta(sql: SqlStorage, k: MetaKey): void {
  sql.exec("DELETE FROM meta WHERE k = ?", k);
}

/** Every `meta` row deleted but those under `keep`. */
export function clearMetaBut(sql: SqlStorage, keep: MetaKey[]): void {
  sql.exec(`DELETE FROM meta WHERE k NOT IN (${keep.map(() => "?").join(", ")})`, ...keep);
}
