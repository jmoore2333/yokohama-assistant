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
 * The writers — `upsertDoc`, `deleteDoc`, `setMeta`, `deleteMeta` — run inside
 * a batch's `transactionSync`, after `ensureTables`.
 */

export type DocKind = "record" | "table" | "chat" | "picker" | "house";

/** Every key `meta` holds: spec §2.4's, and `poison`, the kind whose load
 *  trapped (the plan's Task 9). */
export type MetaKey =
  | "generation"
  | "revision"
  | "slot"
  | "acked"
  | "archive_owed"
  | "marker"
  | "held"
  | "load_traps"
  | "poison"
  | "wakes";

/** Whether the first write batch has run. Both tables are created by one
 *  `ensureTables`, inside one transaction, so it asks for both. It names them:
 *  `sqlite_master` also lists the platform's own `_cf_METADATA` once an alarm
 *  has been set. */
export function hasTables(sql: SqlStorage): boolean {
  const { n } = sql
    .exec<{ n: number }>("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name IN ('docs', 'meta')")
    .one();
  return n === 2;
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
