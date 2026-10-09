import { describe, expect, it } from "vitest";
import type { ShellConfig } from "./config";
import {
  DAY_MS,
  expiryDays,
  expiryDeadline,
  expiryReason,
  gameOf,
  keptOf,
  markerEnd,
  markerSentence,
  openTtlDaysOf,
  ttlDaysOf,
  type Marker,
} from "./marker";

/** 2026-10-22, 12:00 UTC: a time whose date reads the same in every zone. */
const NOON = Date.UTC(2026, 9, 22, 12, 0, 0);

// Spec §2.4 step 5 and §4: the marker's sentence is a HEAD and a TAIL, both
// host-written. Every one of the nine is written out here as a LITERAL, so a
// mutant in either half — a word, the dash, a tail under the wrong `kept`, a
// head under the wrong reason — fails a line of its own.
describe("the marker's sentence (spec §2.4 step 5, §4)", () => {
  it.each([
    ["abandoned", "archive", "this table was abandoned on 2026-10-22 — its game is in the archive"],
    ["abandoned", "nothing", "this table was abandoned on 2026-10-22 — no move had been made, so there was nothing to keep"],
    ["abandoned", "off", "this table was abandoned on 2026-10-22 — this server keeps no archive, so its game was not kept"],
    ["operator", "archive", "this table was removed by an operator on 2026-10-22 — its game is in the archive"],
    ["operator", "nothing", "this table was removed by an operator on 2026-10-22 — no move had been made, so there was nothing to keep"],
    ["operator", "off", "this table was removed by an operator on 2026-10-22 — this server keeps no archive, so its game was not kept"],
  ] as const)("%s, kept %s", (reason, kept, sentence) => {
    expect(markerSentence({ at: NOON, reason, kept })).toBe(sentence);
  });

  it.each([
    ["archive", "this table expired on 2026-10-22 after 90 days without activity — its game is in the archive"],
    ["nothing", "this table expired on 2026-10-22 after 90 days without activity — no move had been made, so there was nothing to keep"],
    ["off", "this table expired on 2026-10-22 after 90 days without activity — this server keeps no archive, so its game was not kept"],
  ] as const)("expired, kept %s", (kept, sentence) => {
    expect(markerSentence({ at: NOON, reason: "expired", ttl_days: 90, kept })).toBe(sentence);
  });

  // The head carries the marker's OWN `ttl_days`, whatever it is. Mutants: a
  // constant 90; the days read from anywhere but the marker. One day is "1
  // day", not "1 days".
  it("names the marker's own ttl_days in the expired head", () => {
    const expired = (ttl_days: number): Marker => ({ at: NOON, reason: "expired", ttl_days, kept: "nothing" });
    expect(markerSentence(expired(30))).toBe(
      "this table expired on 2026-10-22 after 30 days without activity — no move had been made, so there was nothing to keep",
    );
    expect(markerSentence(expired(1))).toBe(
      "this table expired on 2026-10-22 after 1 day without activity — no move had been made, so there was nothing to keep",
    );
  });

  // R-6.5c2.42's fourth head: a table still WAITING for players when the
  // alarm closed it — its status `open` — whose game there never was. The
  // head carries the marker's OWN `ttl_days`; one day is "1 day". Mutants: the
  // expired head under `waited`; a constant 7; "1 days".
  it("names a table that waited for players, with the marker's own ttl_days", () => {
    const waited = (ttl_days: number): Marker => ({ at: NOON, reason: "waited", ttl_days, kept: "nothing" });
    expect(markerSentence(waited(7))).toBe(
      "this table waited 7 days for players and was closed on 2026-10-22 — no move had been made, so there was nothing to keep",
    );
    expect(markerSentence(waited(1))).toBe(
      "this table waited 1 day for players and was closed on 2026-10-22 — no move had been made, so there was nothing to keep",
    );
  });

  // The date is the UTC date of `at`, written `YYYY-MM-DD`, in EVERY zone the
  // shell could run in. The runner's zone is set to one where the local date
  // differs from the UTC date at these instants, so a mutant reading the
  // local date fails here whatever zone the machine is in. Mutants: the local
  // date; a month off by one (`getUTCMonth` is 0-based); no zero padding.
  it("dates the marker in UTC, across midnight", () => {
    const zone = process.env.TZ;
    process.env.TZ = "America/Los_Angeles";
    try {
      const head = (at: number) => markerSentence({ at, reason: "abandoned", kept: "nothing" }).split(" — ")[0];
      const midnight = Date.UTC(2026, 9, 23, 0, 0, 0);
      expect(new Date(midnight).getDate(), "premise: the local date is still the 22nd").toBe(22);
      expect(head(midnight - 1)).toBe("this table was abandoned on 2026-10-22");
      expect(head(midnight)).toBe("this table was abandoned on 2026-10-23");
      expect(head(Date.UTC(2026, 0, 5, 3, 0, 0)), "a single-digit month and day, padded").toBe("this table was abandoned on 2026-01-05");
    } finally {
      if (zone === undefined) delete process.env.TZ;
      else process.env.TZ = zone;
    }
  });
});

// Spec §2.4 step 6: a marker stands for 90 days from its `at`.
describe("the marker's end, and the expiry deadline (spec §2.4)", () => {
  it("ends a marker 90 days after its at", () => {
    expect(markerEnd({ at: NOON, reason: "abandoned", kept: "archive" })).toBe(NOON + 90 * DAY_MS);
    expect(DAY_MS).toBe(86_400_000);
  });

  // Mutants: the TTL read as hours or seconds; the deadline from anything but
  // the last activity.
  it("puts an expiry deadline at the last activity plus the TTL in whole days", () => {
    expect(expiryDeadline(NOON, 90)).toBe(NOON + 90 * 86_400_000);
    expect(expiryDeadline(NOON, 1)).toBe(NOON + 86_400_000);
  });

  // The orchestrator's ruling I-34: the TTL is read through this ONE reader,
  // which a pool cell replaces at module scope for "no TTL, no expiry" — and
  // the waiting-table TTL through its own beside it (R-6.5c2.42).
  it("reads the TTL from the configuration, null when unset, and the waiting-table TTL beside it", () => {
    const config = (ttlDays: number | null, openTtlDays = 7): ShellConfig => ({
      door: "worker",
      archive: "r2",
      ttlDays,
      openTtlDays,
      admins: null,
      countWakes: false,
      devDoor: false,
      diagnosticSeeds: false,
      clock: { kind: "wall" },
    });
    expect(ttlDaysOf(config(90))).toBe(90);
    expect(ttlDaysOf(config(null))).toBeNull();
    expect(openTtlDaysOf(config(90, 3))).toBe(3);
    expect(openTtlDaysOf(config(90))).toBe(7);
  });
});

// R-6.5c2.42 (the plan's R-E8): the alarm is ARMED from SQL alone, which
// cannot read a table's status, so at the SHORTER of the two TTLs — and once
// the alarm has read the status and found the table not waiting, at the TTL
// itself. Mutants: the longer of the two; the TTL whatever was read; the
// waiting TTL whatever was read.
describe("the days an expiry is due after (R-6.5c2.42)", () => {
  it("is the shorter TTL while the status is unread or waiting, and the TTL once it is read not waiting", () => {
    expect(expiryDays(90, 7, undefined), "unread: armed from SQL").toBe(7);
    expect(expiryDays(90, 7, true), "waiting").toBe(7);
    expect(expiryDays(90, 7, false), "read, and not waiting").toBe(90);
    expect(expiryDays(5, 7, undefined), "a TTL shorter than the waiting TTL governs").toBe(5);
    expect(expiryDays(5, 7, true)).toBe(5);
  });
});

// R-6.5c2.42: how the alarm ends a table past the deadline it was armed at —
// by its STATUS first. A table still waiting for players is closed as one
// that waited once its waiting TTL is past, whatever else is; any other table
// expires only once the TTL itself is past. Each boundary is pinned at its
// millisecond. Mutants: the TTL asked before the status (an open table idle
// 100 days read `expired`); the status ignored (a playing table closed at 7
// days); `>` for `>=` at either deadline; the waiting TTL applied where the
// TTL is shorter.
describe("how the alarm ends a table (R-6.5c2.42)", () => {
  const at = (days: number, less = 0) => NOON + days * DAY_MS - less;
  it("closes a waiting table once its waiting TTL is past, whatever else is", () => {
    expect(expiryReason(at(7, 1), NOON, 90, 7, true), "a millisecond short").toBeUndefined();
    expect(expiryReason(at(7), NOON, 90, 7, true)).toEqual({ reason: "waited", ttl_days: 7 });
    expect(expiryReason(at(100), NOON, 90, 7, true), "past both: its status decides").toEqual({ reason: "waited", ttl_days: 7 });
  });
  it("expires any other table only once the TTL itself is past", () => {
    expect(expiryReason(at(7), NOON, 90, 7, false), "past the waiting TTL, not waiting").toBeUndefined();
    expect(expiryReason(at(90, 1), NOON, 90, 7, false), "a millisecond short").toBeUndefined();
    expect(expiryReason(at(90), NOON, 90, 7, false)).toEqual({ reason: "expired", ttl_days: 90 });
  });
  it("expires a waiting table under the TTL where the TTL is the shorter", () => {
    expect(expiryReason(at(5), NOON, 5, 7, true)).toEqual({ reason: "expired", ttl_days: 5 });
    expect(expiryReason(at(7), NOON, 5, 7, true), "then its waiting TTL passes too").toEqual({ reason: "waited", ttl_days: 7 });
  });
});

// The orchestrator's ruling I-64: whether there was a game to keep is read off
// `events(id, creator, "")` taken BEFORE the delete — `status` and `events` —
// and never off the table document. The reading of a FINISHED game says
// `over`, not `playing` (the layer's `events_of`), and a finished game is a
// game. Mutants: `over` not counted as a game; `events > 0` not required;
// the fallback inverted.
describe("whether there was a game to keep (the orchestrator's ruling I-64)", () => {
  const reading = (status: string, events: number) => ({
    status: 200,
    body: JSON.stringify({ chat: 0, events, status, to_move: status === "open" || status === "dealing" ? null : 0, your_turn: false }),
  });

  it("is a game when the table is playing or over and a move was made", () => {
    expect(gameOf(reading("playing", 1), false)).toBe(true);
    expect(gameOf(reading("over", 1036), false)).toBe(true);
  });

  it("is no game at an open or dealing table, or before the first move", () => {
    expect(gameOf(reading("open", 0), true)).toBe(false);
    expect(gameOf(reading("dealing", 0), true)).toBe(false);
    expect(gameOf(reading("playing", 0), true)).toBe(false);
  });

  // Unreachable through the shell (table.ts says why): with no reading, the
  // stored record is the evidence.
  it("falls back to whether a record was stored when there is no reading", () => {
    expect(gameOf(undefined, true)).toBe(true);
    expect(gameOf({ status: 403, body: '{"error":"x"}' }, false)).toBe(false);
  });
});

// Spec §2.4 step 2, as the orchestrator's rulings I-64 and I-71 read it: a
// table whose copy carries `archived_at` — the layer's own stamp, which is a
// fact of the bucket whatever the switch reads now — kept its game in the
// archive, and that is asked BEFORE the switch. Mutants: the stamp consulted
// after the switch (a game stamped at game over, then deleted with the archive
// off, reads `off`); a save that landed read as anything but `archive`; a game
// deleted with no save while the archive is on read as `nothing`; the switch
// ignored.
describe("what the marker says was kept (spec §2.4 step 2; rulings I-64, I-71)", () => {
  const facts = (f: Partial<{ stamped: boolean; saved: boolean; game: boolean; archive: "r2" | "off" }>) => ({
    stamped: false,
    saved: false,
    game: true,
    archive: "r2" as const,
    ...f,
  });

  it("is the archive for a table already stamped, whatever the switch reads now", () => {
    expect(keptOf(facts({ stamped: true, archive: "off" }))).toBe("archive");
    expect(keptOf(facts({ stamped: true, archive: "r2" }))).toBe("archive");
  });

  it("is the archive when the delete's save landed, or was already there", () => {
    expect(keptOf(facts({ saved: true }))).toBe("archive");
    expect(keptOf(facts({ saved: true, game: false })), "a landed save wins whatever the reading").toBe("archive");
  });

  it("is the archive for a game deleted with no save while the archive is on: it was stamped", () => {
    expect(keptOf(facts({}))).toBe("archive");
  });

  it("is off for a game never stamped, deleted while the archive is off", () => {
    expect(keptOf(facts({ archive: "off" }))).toBe("off");
  });

  it("is nothing when there was no game, whatever the switch", () => {
    expect(keptOf(facts({ game: false }))).toBe("nothing");
    expect(keptOf(facts({ game: false, archive: "off" }))).toBe("nothing");
  });
});
