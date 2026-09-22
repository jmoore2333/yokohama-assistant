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
// PREFERS the snapshot of the Worker that OWNS the game, asked within a short
// bound, and falls back to the database when the Worker does not answer.
//
// **"The Worker answers" is not "the Worker owns the game"** (PR #37 fix wave,
// R-F1, amending spec §2.5 forward). Every tab's Worker loads a snapshot of
// every game at its boot and writes back only the games it holds, so a tab
// that does not own a game can hold a copy another tab has moved past. The
// Worker's `pair` arm is where that is decided (`local/exportPair.ts`): its
// memory where this tab holds the game at the current ownership generation,
// the committed pair everywhere else. This page takes whatever it answers
// WHOLE, and its own read whole when it does not — never one document from
// each.
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

/** How long the Worker is given, ACROSS ONE EXPORT, to answer with its
 *  snapshots before the export falls back to reading IndexedDB (spec §2.5's
 *  ordering; review M-3).
 *
 *  **Short on purpose**: once the module is up the Worker answers a `pair`
 *  from memory or from one IndexedDB read (R-F1), so a wait longer than this
 *  is a module that is not coming.
 *  **And shared, not per game**: a Worker that STARTS and never initialises
 *  answers nothing at all, and a per-game bound would make "Back up everything"
 *  take this long times the number of games.
 *
 *  **Exported for the cell that spends it** (ledger Task 8 L195): the cell that
 *  proves one bound is spent for a whole export advances a fake clock by this
 *  number, which is what took a wall-clock assertion — and its flake — out of a
 *  per-edit gate. */
export const WORKER_BOUND_MS = 1_500;

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

/** One game on its way into a file: the two strings, and what THIS DEVICE
 *  knows about which revision they are at. */
export interface OutgoingGame {
  id: string;
  record: string | null;
  table: string;
  /** **What this device knows about THIS GAME**, and `null` where it knows
   *  nothing — never the exporter's own rev and hash. See [`buildEnvelope`]. */
  semantics_rev?: number | null;
  rules_hash?: string | null;
}

/**
 * Wrap pairs into a file. **Strings in, string out, nothing parsed.**
 *
 * **Two different provenances, which is the whole reason there are two places
 * to put one** (spec §2.5; review I-2):
 *
 * - The HEADER's rev, hash and version are the EXPORTER's — what this browser
 *   was last TOLD (`local/facts.ts`), not what it asks for now, because the one
 *   source that is still there with the module dead is the one already written
 *   down. Where it has never been told, the file says `0` and `""` rather than
 *   a guess. It says only what wrote the file.
 * - EACH GAME's pair is what this device knows about that game, and `null`
 *   where it knows nothing. The only thing it ever knows is a COMPATIBILITY
 *   ROW — this device could not open that game, and the row records the
 *   revision the file it came from claimed. A game that opens here is at this
 *   build's revision by construction, and saying so would be restating the
 *   header; saying the header's value about a game the header is not about is
 *   what an earlier draft did, and it stamped a re-exported rev-6 game rev 7.
 *
 * Nothing reads either for a decision: a restore re-derives every verdict from
 * the records themselves (`backup.rs`'s `verdict_of`). They are there so a row
 * that will not open can say what it was written at.
 */
export function buildEnvelope(pairs: OutgoingGame[]): string {
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
      semantics_rev: p.semantics_rev ?? null,
      rules_hash: p.rules_hash ?? null,
    })),
  };
  return JSON.stringify(env, null, 2) + "\n";
}

/** What this device knows about each game it could not open, by id — the ONE
 *  source of a per-game revision (see [`buildEnvelope`]). */
async function knownRevisions(db: IDBDatabase): Promise<Map<string, CompatNote>> {
  const out = new Map<string, CompatNote>();
  for (const row of await readCompat(db)) {
    try {
      out.set(row.id, JSON.parse(row.text) as CompatNote);
    } catch {
      /* a note this build cannot read says nothing about the revision */
    }
  }
  return out;
}

/** One game's two documents: the Worker's answer WHOLE when it answers inside
 *  the bound, this page's own read WHOLE when it does not.
 *
 *  **One pair from one source** (PR #37 fix wave, R-F1). This used to complete
 *  the Worker's answer field by field from the database — `live?.record ??
 *  stored.record` — which could pair one snapshot's record with another's
 *  table: a Worker saying "at the deal, no record" beside a store holding a
 *  played record came out as that record and the DEALING table, a pair neither
 *  source ever held. What the Worker answers is already the right source (its
 *  own memory where it owns the game, the committed pair otherwise — see
 *  `local/exportPair.ts`), so a `null` in it is an answer and not a gap.
 *
 *  `deadline` is shared by a whole export (review M-3): a Worker that STARTS
 *  but whose module never initialises leaves every `ask` unanswered, and a
 *  per-game bound would cost 1.5 s times the number of games with nothing on
 *  screen saying so. Once it has passed, the database's copy is taken without
 *  asking again. */
async function pairOf(
  id: string,
  stored: { record: string | null; table: string | null },
  deadline: Deadline,
): Promise<{ record: string | null; table: string | null }> {
  const live = deadline.expired() ? null : await withinBound(localPair(id), deadline.left());
  return live ?? stored;
}

/** A budget for the Worker across ONE export. */
interface Deadline {
  expired: () => boolean;
  left: () => number;
}

const deadlineOf = (ms: number): Deadline => {
  const at = Date.now() + ms;
  return { expired: () => Date.now() >= at, left: () => Math.max(0, at - Date.now()) };
};

/**
 * Run `f` against this browser's database and GIVE THE CONNECTION BACK (final
 * review I-5).
 *
 * **One connection per public call, closed in a `finally`.** Every function
 * below used to `await openDb()` and keep the handle: [`unplayableIds`] runs on
 * EVERY local write (`OnThisDevice`'s listing), so a 400-move game left 400
 * connections open for the life of the page.
 *
 * **It is `outbox.ts`'s `withStore` shape with the refusals left ALONE**, and
 * that difference is the whole reason this is a second helper rather than a
 * shared one: the outbox answers a fallback because a browser with no usable
 * store simply has no outbox, while these calls are a player's own click and
 * every sentence they can meet — a store that will not open, a game that is not
 * here, a file that is not a backup — belongs on the screen that asked.
 */
async function withDb<T>(f: (db: IDBDatabase) => Promise<T>): Promise<T> {
  const db = await openDb();
  try {
    return await f(db);
  } finally {
    db.close();
  }
}

/** Every game in this browser, as a file. */
export async function backupEverything(): Promise<string> {
  return withDb(async (db) => {
    const rows = await readAll(db);
    const known = await knownRevisions(db);
    const byId = new Map<string, { record: string | null; table: string | null }>();
    for (const r of rows) {
      // A compatibility row is this browser's own note and is NOT exported as a
      // game: the receiving device re-derives its own verdict from the pair. What
      // it DOES contribute is that game's revision, above.
      if (r.doc === "compat") continue;
      const at = byId.get(r.id) ?? { record: null, table: null };
      byId.set(r.id, { ...at, [r.doc]: r.text });
    }
    const deadline = deadlineOf(WORKER_BOUND_MS);
    const pairs: OutgoingGame[] = [];
    for (const [id, stored] of byId) {
      const both = await pairOf(id, stored, deadline);
      // A game with no table document is not a game — half a pair on disk, from
      // a write that did not complete. It is left where it is rather than
      // exported as something a restore would refuse.
      if (both.table === null) continue;
      pairs.push({ id, record: both.record, table: both.table, ...revisionOf(known.get(id)) });
    }
    pairs.sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
    return buildEnvelope(pairs);
  });
}

/** What a compatibility row says about a game's revision, or nothing. */
const revisionOf = (note: CompatNote | undefined) => ({
  semantics_rev: note?.semantics_rev ?? null,
  rules_hash: note?.rules_hash ?? null,
});

/** One game, as a file. */
export async function backupOne(id: string): Promise<string> {
  return withDb(async (db) => {
    const stored = await readPair(db, id);
    const both = await pairOf(id, stored, deadlineOf(WORKER_BOUND_MS));
    if (both.table === null) throw new Error(`there is no game named ${id} on this device`);
    const known = await knownRevisions(db);
    return buildEnvelope([
      { id, record: both.record, table: both.table, ...revisionOf(known.get(id)) },
    ]);
  });
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
  const games: RestoreReport["games"] = [];
  const landed: string[] = [];
  // **The connection is given back before the tab is asked to reconcile**
  // (final review I-5). `localAdopt` below is the WORKER reading the same
  // database, and this page holding its own handle open across it is the
  // shape that makes an upgrade wait on the tab that asked for it.
  await withDb(async (db) => {
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
  });

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
  return withDb(async (db) => {
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
  });
}

/** The ids this device has a compatibility row for — the games that will NOT
 *  open here (M6.5b2, review M-2).
 *
 *  **What "On this device" hides a row for.** A restored cross-rev pair whose
 *  TABLE document parses is a row the layer's own listing builds happily, so
 *  the door listed the same game twice: once as playable and once under "Will
 *  not open here", with different controls on each. One game, one row, and the
 *  row that says why it will not open is the true one. */
export async function unplayableIds(): Promise<Set<string>> {
  return withDb(async (db) => {
    return new Set((await readCompat(db)).map((r) => r.id));
  });
}

/** Remove an unplayable game from this device — all of [`GAME_DOCS`].
 *
 *  Unfenced, for `forgetGame`'s reason: a game the module never loaded is a
 *  game no tab holds. */
export async function forgetUnplayable(id: string): Promise<void> {
  return withDb(async (db) => {
    await forgetGame(db, id);
  });
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
