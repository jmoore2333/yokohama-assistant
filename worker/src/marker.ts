/**
 * **The marker, and the TTLs** (spec §2.4 "Delete, expiry and the MARKER";
 * R-6.5c.8, R-6.5c.15, R-6.5c2.42): what a table object keeps for 90 days
 * after the layer deleted its table — how it ended, when, and what became of
 * its game — the sentence it answers every request with meanwhile, and the
 * deadlines an idle table expires at: the TTL, and the shorter one a table
 * that has not started has. Pure — no Workers API — so its cells run in
 * node (`marker.unit.test.ts`); `table.ts` applies it.
 */
import type { ShellConfig } from "./config";

/** A day, in milliseconds. */
export const DAY_MS = 24 * 60 * 60 * 1000;

/** How long a marker stands (spec §2.4 step 6). */
export const MARKER_DAYS = 90;

/** What became of the table's game (spec §2.4 step 2): it is in the archive;
 *  there was no game to keep; or there was one, and this server keeps no
 *  archive. */
export type Kept = "archive" | "nothing" | "off";

/** How a table ended through the layer: its creator's Abandon, its expiry
 *  after `ttl_days` without activity, its closing after `ttl_days` not yet
 *  started — open, or dealing with a keep outstanding (R-6.5c2.42; security
 *  S3, D9) — or an operator's plain expire. Only the alarm's two sentences
 *  name their days, so only their markers keep them. */
export type Reason = { reason: "abandoned" | "operator" } | { reason: "expired" | "waited"; ttl_days: number };

/** How a table ended, and what was kept: a marker without its time. */
export type Ending = Reason & { kept: Kept };

/** `meta.marker` (spec §2.4 step 2), the shell's own JSON. `at` is the
 *  platform's `Date.now()` at the delete's batch (the orchestrator's rulings
 *  I-43 and I-44). */
export type Marker = { at: number } & Ending;

/** Spec §2.4 step 5's three tails, by `kept`. */
const TAILS: Record<Kept, string> = {
  archive: " — its game is in the archive",
  nothing: " — no move had been made, so there was nothing to keep",
  off: " — this server keeps no archive, so its game was not kept",
};

/**
 * The sentence every request meets while `m` stands (spec §2.4 step 5; §4
 * lists it as host-written): a HEAD naming how the table ended and the UTC
 * date of `at`, written `YYYY-MM-DD`, then a TAIL saying what was kept. One
 * day of TTL is "1 day". A table that waited (R-6.5c2.42) had not started —
 * open, or dealing with a keep outstanding — or, finalized but unflipped and
 * read by nobody, had no move made (design §3.2 rule 2, D-16), so it had no
 * game, and its tail is `nothing`'s.
 */
export function markerSentence(m: Marker): string {
  const day = new Date(m.at).toISOString().slice(0, 10);
  return head(m, day) + TAILS[m.kept];
}

function head(m: Marker, day: string): string {
  switch (m.reason) {
    case "abandoned":
      return `this table was abandoned on ${day}`;
    case "operator":
      return `this table was removed by an operator on ${day}`;
    case "expired":
      return `this table expired on ${day} after ${days(m.ttl_days)} without activity`;
    case "waited":
      return `this table waited ${days(m.ttl_days)} to start and was closed on ${day}`;
  }
}

/** `n` days, in words: one day is "1 day". */
function days(n: number): string {
  return n === 1 ? "1 day" : `${n} days`;
}

/** When `m` stops standing (spec §2.4 step 6): 90 days after its `at`. The
 *  object ends then, once the house has acknowledged its removal. */
export function markerEnd(m: Marker): number {
  return m.at + MARKER_DAYS * DAY_MS;
}

/**
 * The TTL, in whole days, or `null`: no table ever expires (R-6.5c.15). The
 * ONE reader of it (the orchestrator's ruling I-34), in a module of its own so
 * a pool cell can replace it at module scope (the plan's pool fact 4).
 */
export function ttlDaysOf(config: ShellConfig): number | null {
  return config.ttlDays;
}

/**
 * The waiting-table TTL, in whole days (R-6.5c2.42): how long a table that
 * has not started — open, or dealing with a keep outstanding (security S3,
 * D9) — stands without activity. Its ONE reader, beside the
 * TTL's and for the same reason. Inert while `ttlDaysOf` reads `null`: every
 * reader of it asks that first.
 */
export function openTtlDaysOf(config: ShellConfig): number {
  return config.openTtlDays;
}

/** When a table whose last activity was `lastActive` expires: `ttlDays` whole
 *  days after it. */
export function expiryDeadline(lastActive: number, ttlDays: number): number {
  return lastActive + ttlDays * DAY_MS;
}

/**
 * The days after its last activity at which a table's expiry is DUE
 * (R-6.5c2.42; the plan's R-E8). The alarm is armed from SQL alone, which
 * cannot read a table's status (the strings rule), so while `waiting` is
 * unread — `undefined`, as it is at every batch — or read `true`, it is the
 * SHORTER of the two TTLs; once the alarm has read the status and found the
 * table not waiting, it is the TTL itself.
 */
export function expiryDays(ttlDays: number, openTtlDays: number, waiting: boolean | undefined): number {
  return waiting === false ? ttlDays : Math.min(ttlDays, openTtlDays);
}

/**
 * How the alarm ends a table at `now` (R-6.5c2.42; the plan's R-E8), its
 * status read — `waiting` when it has not started: `open`, or `dealing` with a
 * keep outstanding (security S3, D9) — or `undefined` while it stands: a table
 * that has not started is closed as one that `waited` once its last activity
 * is `openTtlDays` old, whatever else is past; any table `expired` once it is
 * `ttlDays` old — a waiting one too, where the TTL is the shorter.
 */
export function expiryReason(now: number, lastActive: number, ttlDays: number, openTtlDays: number, waiting: boolean): Reason | undefined {
  if (waiting && now >= expiryDeadline(lastActive, openTtlDays)) return { reason: "waited", ttl_days: openTtlDays };
  if (now >= expiryDeadline(lastActive, ttlDays)) return { reason: "expired", ttl_days: ttlDays };
  return undefined;
}

/**
 * Whether the table held a game to keep (the orchestrator's ruling I-64): its
 * reading — `events(id, creator, "")`, taken before the delete — says it was
 * playing, or over, and that a move was made. A finished game reads `over`
 * (the layer's `events_of`), and is a game. The reading carries no seed, so it
 * may be parsed; the table document never is.
 *
 * With no 200 reading the stored record is the evidence: whether one was
 * stored at all. `table.ts` says why the shell never needs it.
 */
export function gameOf(reading: { status: number; body: string } | undefined, recordStored: boolean): boolean {
  if (reading === undefined || reading.status !== 200) return recordStored;
  const { status, events } = JSON.parse(reading.body) as { status?: unknown; events?: unknown };
  return (status === "playing" || status === "over") && typeof events === "number" && events > 0;
}

/** What a delete through the layer knew when it ended a table: whether the
 *  table's copy carried the layer's stamp (`archived_at`), whether this
 *  delete's own save landed or was already there, whether there was a game to
 *  keep (`gameOf`), and what the archive's switch reads. */
export interface Keeping {
  stamped: boolean;
  saved: boolean;
  game: boolean;
  archive: "r2" | "off";
}

/**
 * What the marker says was kept (spec §2.4 step 2, as the orchestrator's
 * rulings I-64 and I-71 read it):
 *
 * - `archive` when the table was already stamped — the layer's own stamp,
 *   which says the bucket holds the game whatever the switch reads now, so it
 *   is asked BEFORE the switch (I-71) — or when the delete's own save landed
 *   or was already there;
 * - otherwise `nothing` when there was no game;
 * - otherwise, for a game, `archive` while the archive is on — the layer saves
 *   every unstamped game it deletes, so one deleted with no save was stamped,
 *   the answer where no copy could be read — and `off` while it is off.
 */
export function keptOf({ stamped, saved, game, archive }: Keeping): Kept {
  if (stamped || saved) return "archive";
  if (!game) return "nothing";
  return archive === "r2" ? "archive" : "off";
}
