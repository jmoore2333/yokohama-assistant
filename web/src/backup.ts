// Backup and restore, the PAGE's half (M6.5b2, R-6.5b2.10 – .12; spec §2.5).
//
// **One envelope, one game or all of them**, so there is one format rather than
// two, and the raw `<id>.json` export stays exactly as it is beside it: that
// file is what the terminal loads, and this one is what this app restores.
//
// ## Why writing the envelope is the PAGE's job and not the module's
//
// A module that will not load is when you most need your games out. `localPair`
// posts a message to the Worker and the Worker awaits the module's
// initialization before it handles ANY message, so an export that sourced its
// strings that way would die with the module exactly as a wasm export would.
//
// So [`backupEverything`] reads **IndexedDB directly from the page**, which
// needs no Worker and no module: the documents are stored as opaque text and
// this page can open the same database. The Worker path is kept for the one
// case the direct read gets wrong — a game with a move applied but not yet
// written, which is the very move a recovery exists to rescue — so the control
// PREFERS the Worker's snapshot when the Worker answers within a short bound
// and falls back to the database when it does not.
//
// ## What is decided here and what is decided in Rust
//
// Rust decides whether a game is VALID and whether it is ADDRESSABLE
// (`crates/yokohama-web/src/backup.rs`, shared with the archive route, wrapped
// as the module's `restore_read`). This file decides what to do about what is
// ALREADY on this device — the clash rule — because that is a question about
// this browser's IndexedDB and has to be one transaction with the write.
//
// **Nothing here parses a record or a table document.** A record's terminal
// digest is over its exact bytes and its seed is a `u64` that `JSON.parse`
// rounds; the envelope carries both as strings and this module moves strings.
// The ENVELOPE itself is parsed, because this module wrote it.

import { localAdopt, localPair, localRestoreRead } from "./local/client";
import {
  forgetGame,
  openDb,
  readAll,
  readCompat,
  readPair,
  restoreOne,
  type CompatNote,
  type RestoreOutcome,
} from "./local/db";
import { serverFacts } from "./local/facts";

/** The envelope's `kind` and version — the Rust side's `ENVELOPE_KIND` and
 *  `ENVELOPE_VERSION`. Two statements of one pair of constants, held together
 *  by the committed fixture both sides read. */
export const ENVELOPE_KIND = "yokohama-backup";
export const ENVELOPE_VERSION = 1;

/** One game as it goes into a file, and as it comes out. */
export interface BackupGame {
  id: string;
  /** **Optional**: a game still at the deal has a table document and no
   *  record, and a required field would make "Back up everything" either omit
   *  those games or fail the whole export. */
  record: string | null;
  table: string;
  semantics_rev?: number | null;
  rules_hash?: string | null;
}

/** A backup file. */
export interface Envelope {
  kind: string;
  envelope: number;
  created_at: number;
  origin: string;
  app_version: string;
  semantics_rev: number;
  rules_hash: string;
  games: BackupGame[];
}

/** What the module says about one game in a file — `Verdict`'s four arms. */
export type Verdict =
  | { kind: "accepted"; players: number; events: number; over: boolean }
  | { kind: "dealing"; players: number }
  | { kind: "unplayable"; reason: string }
  | { kind: "rejected"; reason: string };

/** One row of the module's report. */
export interface ReportRow {
  id: string;
  verdict: Verdict;
}

/** What a restore did about one game — the five outcomes the report renders,
 *  and the only vocabulary `Backup.tsx` knows. */
export type Outcome = RestoreOutcome | "unplayable" | "rejected";

export interface RestoreReport {
  games: { id: string; outcome: Outcome; reason?: string }[];
}

/** How long the Worker is given to answer with its snapshot before the export
 *  falls back to reading IndexedDB (spec §2.5's ordering).
 *
 *  **Short on purpose**: the Worker answers a `pair` from memory once the
 *  module is up, so a wait longer than this is a module that is not coming. */
const WORKER_BOUND_MS = 1_500;

/** Settle with `null` if `p` has not answered in `ms`.
 *
 *  A rejection is `null` too: a Worker that refuses is a Worker that did not
 *  answer, and the whole point of the fallback is that the caller does not have
 *  to know which kind of silence it met. */
function withinBound<T>(p: Promise<T>, ms: number): Promise<T | null> {
  return new Promise((resolve) => {
    const timer = setTimeout(() => resolve(null), ms);
    p.then(
      (v) => {
        clearTimeout(timer);
        resolve(v);
      },
      () => {
        clearTimeout(timer);
        resolve(null);
      },
    );
  });
}

/**
 * Wrap pairs into a file. **Strings in, string out, nothing parsed.**
 *
 * The exporter provenance — the rev, the hash and the version — is what this
 * browser was last TOLD (`local/facts.ts`), not what it asks for now: the one
 * source that is still there with the module dead. Where it has never been
 * told, the file says `0` and `""` rather than a guess, and nothing reads
 * those for a decision: a restore re-derives every verdict from the games.
 */
export function buildEnvelope(pairs: { id: string; record: string | null; table: string }[]): string {
  const f = serverFacts();
  const env: Envelope = {
    kind: ENVELOPE_KIND,
    envelope: ENVELOPE_VERSION,
    created_at: Math.floor(Date.now() / 1000),
    origin: typeof location === "undefined" ? "" : location.host,
    app_version: f.appVersion,
    semantics_rev: f.semanticsRev,
    rules_hash: f.rulesHash,
    games: pairs.map((p) => ({
      id: p.id,
      record: p.record,
      table: p.table,
      semantics_rev: f.semanticsRev,
      rules_hash: f.rulesHash,
    })),
  };
  return JSON.stringify(env, null, 2) + "\n";
}

/** One game's two documents, the Worker's snapshot PREFERRED. */
async function pairOf(id: string, stored: { record: string | null; table: string | null }) {
  const live = await withinBound(localPair(id), WORKER_BOUND_MS);
  const record = live?.record ?? stored.record;
  const table = live?.table ?? stored.table;
  return { record, table };
}

/** Every game in this browser, as a file. */
export async function backupEverything(): Promise<string> {
  const db = await openDb();
  const rows = await readAll(db);
  const byId = new Map<string, { record: string | null; table: string | null }>();
  for (const r of rows) {
    // A compatibility row is this browser's own note and is NOT exported: the
    // receiving device re-derives its own verdict from the pair.
    if (r.doc === "compat") continue;
    const at = byId.get(r.id) ?? { record: null, table: null };
    byId.set(r.id, { ...at, [r.doc]: r.text });
  }
  const pairs: { id: string; record: string | null; table: string }[] = [];
  for (const [id, stored] of byId) {
    const both = await pairOf(id, stored);
    // A game with no table document is not a game — half a pair on disk, from
    // a write that did not complete. It is left where it is rather than
    // exported as something a restore would refuse.
    if (both.table === null) continue;
    pairs.push({ id, record: both.record, table: both.table });
  }
  pairs.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return buildEnvelope(pairs);
}

/** One game, as a file. */
export async function backupOne(id: string): Promise<string> {
  const db = await openDb();
  const stored = await readPair(db, id);
  const both = await pairOf(id, stored);
  if (both.table === null) throw new Error(`there is no game named ${id} on this device`);
  return buildEnvelope([{ id, record: both.record, table: both.table }]);
}

/** The module's report, or a rejection carrying the validator's own sentence
 *  about the FILE. */
async function readReport(envelope: string): Promise<ReportRow[]> {
  const text = await localRestoreRead(envelope);
  return JSON.parse(text) as ReportRow[];
}

/**
 * Put a backup file's games on this device.
 *
 * The order is what makes it safe (spec §2.5):
 *
 * 1. **Duplicate ids inside one envelope are refused BEFORE anything is
 *    written** — and here rather than only in Rust, because this refusal has to
 *    work with the module dead, and because nothing should be written for a
 *    file that is asking for two different things under one name.
 * 2. The module says what each game IS.
 * 3. For each game the module did not reject, ONE IndexedDB transaction does
 *    the clash check and both writes.
 * 4. **A transaction that fails reports that game and leaves the others
 *    alone**, which is why the loop catches per game rather than around itself.
 * 5. Everything that landed is adopted by the running tab and the local
 *    listings re-read.
 */
export async function restorePairs(envelope: string): Promise<RestoreReport> {
  // **A sentence a player can read, and not the engine's.** `JSON.parse` on a
  // file that is not one throws "Unexpected token … in JSON at position 0",
  // which says nothing about what was chosen or what to do about it. The
  // module's own refusals ARE carried verbatim — they are about a backup file
  // — but this one is about a file that is not a backup at all.
  let parsed: Envelope;
  try {
    parsed = JSON.parse(envelope) as Envelope;
  } catch {
    throw new Error("this file is not a backup this app wrote");
  }
  if (!Array.isArray(parsed?.games)) {
    throw new Error("this file is not a backup this app wrote");
  }
  const seen = new Set<string>();
  for (const g of parsed.games) {
    if (seen.has(g.id)) throw new Error(`this file has two games called ${g.id} — duplicate id`);
    seen.add(g.id);
  }

  const report = await readReport(envelope);
  // **A game the validator did not judge is not a game to write**, and dropping
  // it from the report silently is the failure this guard exists against: the
  // shipped validator maps one row per game, so a count that disagrees means
  // the report and the file are not about the same file.
  if (report.length !== parsed.games.length) {
    throw new Error(
      `this file carries ${parsed.games.length} games and the report judged ${report.length}`,
    );
  }
  const byId = new Map(parsed.games.map((g) => [g.id, g]));
  const db = await openDb();
  const games: RestoreReport["games"] = [];
  const landed: string[] = [];

  for (const row of report) {
    const game = byId.get(row.id);
    if (game === undefined) {
      // The module reported a game this file does not carry, which can only be
      // a report and a file that are not the same file.
      games.push({ id: row.id, outcome: "rejected", reason: "this file does not carry that game" });
      continue;
    }
    if (row.verdict.kind === "rejected") {
      games.push({ id: row.id, outcome: "rejected", reason: row.verdict.reason });
      continue;
    }
    // **The compatibility row, beside the two opaque documents** (R-6.5b2.12):
    // an unplayable game lands, is listed with its reason, and can be saved out
    // again. Its bytes are never rewritten, which is what "never migrated"
    // means — this row is a note beside them.
    const compat: CompatNote | null =
      row.verdict.kind === "unplayable"
        ? {
            semantics_rev: game.semantics_rev ?? null,
            rules_hash: game.rules_hash ?? null,
            reason: row.verdict.reason,
            restored_at: Math.floor(Date.now() / 1000),
          }
        : null;
    try {
      const outcome = await restoreOne(db, row.id, {
        table: game.table,
        ...(game.record === null ? {} : { record: game.record }),
        ...(compat === null ? {} : { compat: JSON.stringify(compat) }),
      });
      if (outcome === "restored") landed.push(row.id);
      games.push({
        id: row.id,
        // An unplayable game that LANDED says so, because "restored" would
        // promise a game that opens. One that was already here, or clashed,
        // says what happened to it — those answers are about this device.
        outcome: outcome === "restored" && compat !== null ? "unplayable" : outcome,
        ...(outcome === "clash"
          ? { reason: "a different game is already on this device under that name" }
          : compat !== null && outcome === "restored"
            ? { reason: compat.reason }
            : {}),
      });
    } catch (e) {
      games.push({
        id: row.id,
        outcome: "rejected",
        reason: e instanceof Error ? e.message : String(e),
      });
    }
  }

  // **The running tab reconciles** (spec §2.5), and a failure to is not a
  // failure to restore: the games are on the device, and the next visit loads
  // them. Reported by the listing's own fault line rather than by turning a
  // successful restore into an error.
  if (landed.length > 0) await localAdopt(landed).catch(() => 0);
  return { games };
}

/** One unplayable game on this device, as the door lists it. */
export interface UnplayableGame {
  id: string;
  note: CompatNote;
}

/**
 * The games this build cannot open, read STRAIGHT FROM IndexedDB.
 *
 * **Its own durable mechanism, which the module's listing cannot supply**
 * (spec §2.5): the layer skips any table document whose shape check fails, and
 * a restored cross-rev pair can be foreign on both sides — so the row would be
 * in no listing at all. The compatibility row is what puts it in this one.
 */
export async function listUnplayable(): Promise<UnplayableGame[]> {
  const db = await openDb();
  const rows = await readCompat(db);
  const out: UnplayableGame[] = [];
  for (const r of rows) {
    try {
      out.push({ id: r.id, note: JSON.parse(r.text) as CompatNote });
    } catch {
      // A note this build cannot read is still a game that will not open, and
      // saying so with no reason beats dropping the row.
      out.push({
        id: r.id,
        note: { semantics_rev: null, rules_hash: null, reason: "", restored_at: 0 },
      });
    }
  }
  out.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  return out;
}

/** Remove an unplayable game from this device — all three rows.
 *
 *  Unfenced, for `forgetGame`'s reason: a game the module never loaded is a
 *  game no tab holds. */
export async function forgetUnplayable(id: string): Promise<void> {
  const db = await openDb();
  await forgetGame(db, id);
}

/**
 * Hand `text` to the browser as a download named `name`.
 *
 * The same Blob-and-anchor path "Save game file" uses, and here rather than in
 * the component so both controls take one route to the disk.
 */
export function download(name: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: "application/json" }));
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  // Attached for the click and revoked on the next tick: a detached anchor and
  // a synchronous revoke both work in Chromium and have both cancelled the
  // download in other engines.
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}
