import { runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { describe, expect, it, vi } from "vitest";
import * as markerModule from "../src/marker";
import type { Table } from "../src/table";
import { failingPuts, failPushes, holdApply, R2_UNREACHABLE, swapBucket } from "./gates";
import {
  ANOTHER_RECORD,
  BLUE,
  CREATE_BODY,
  DAY_MS,
  LOCAL,
  RED,
  alarmOf,
  archiveOff,
  archivedAt,
  backdate,
  bucketText,
  call,
  docOf,
  logsDuring,
  markerOf,
  operate,
  owedGameOver,
  rebuilt,
  replaceOnce,
  revisionOf,
  seedPlaying,
  settle,
  sqlOf,
  tableStub,
  tablesIn,
  watchBatches,
  type Play,
} from "./helpers";

// **Expiry and the marker** (R-6.5c.15, R-6.5c.8; spec §2.4 "The request path,
// the gates, and the alarm" and "Delete, expiry and the MARKER"; the plan's
// Task 14).
//
// Time moves by BACKDATING the stored times — every document's `written_at`,
// the marker's `at` — through `runInDurableObject`, then running the alarm:
// workerd's clock cannot be moved (spec §5.2). The pool runs with
// `YOKOHAMA_WEB_TABLE_TTL_DAYS` at production's 90 and
// `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` at production's 7 (`wrangler.test.jsonc`;
// the orchestrator's ruling I-34; R-6.5c2.42), and the module's clock frozen
// at 1_789_000_000. Storage and the house are per FILE (pool fact 5): every cell
// uses ids of its own. Every patch is restored in a `finally`, and a cell that
// holds a push releases it there too.

/** `wrangler.test.jsonc`'s TTL, in milliseconds. */
const TTL = 90 * DAY_MS;
/** `wrangler.test.jsonc`'s waiting-table TTL, in milliseconds (R-6.5c2.42):
 *  the SHORTER of the two, so it is where every table's alarm is armed —
 *  from SQL alone, which cannot read whether a table has started (the
 *  strings rule) — and the alarm decides by the table's status. */
const OPEN_TTL = 7 * DAY_MS;
/** An operator's address, as the Worker's gate would forward it. */
const BOSS = "boss@t.test";

/** Ninety days and a minute ago: a last activity past the TTL. */
const idle = () => Date.now() - TTL - 60_000;

/** The date the marker's sentence names: `at`'s UTC date, `YYYY-MM-DD` —
 *  computed here with the UTC getters, not with the shell's own formatting. */
function utcDate(at: number): string {
  const d = new Date(at);
  const two = (n: number) => String(n).padStart(2, "0");
  return `${d.getUTCFullYear()}-${two(d.getUTCMonth() + 1)}-${two(d.getUTCDate())}`;
}

/** Spec §2.4 step 5's heads — and R-6.5c2.42's fourth, a table that waited
 *  to start — and three tails, as LITERALS. */
const HEAD = {
  abandoned: (day: string) => `this table was abandoned on ${day}`,
  expired: (day: string) => `this table expired on ${day} after 90 days without activity`,
  operator: (day: string) => `this table was removed by an operator on ${day}`,
  waited: (day: string) => `this table waited 7 days to start and was closed on ${day}`,
} as const;
const TAIL = {
  archive: " — its game is in the archive",
  nothing: " — no move had been made, so there was nothing to keep",
  off: " — this server keeps no archive, so its game was not kept",
} as const;

/** The `docs` rows' kinds, sorted — none once the object has ended. */
async function docKinds(stub: DurableObjectStub<Table>): Promise<unknown[]> {
  if ((await tablesIn(stub)) === 0) return [];
  return (await sqlOf(stub, "SELECT kind FROM docs ORDER BY kind")).map((r) => r.kind);
}

/** The newest `written_at` of the record, the table and the chat: the table's
 *  last activity (spec §2.4). */
async function lastActivity(stub: DurableObjectStub<Table>): Promise<number> {
  const [row] = await sqlOf(stub, "SELECT max(written_at) AS at FROM docs WHERE kind IN ('record', 'table', 'chat')");
  return Number(row?.at);
}

/** What `user`'s read of the table answers under a marker: a 404, whose
 *  sentence is handed back. */
async function sentenceAt(stub: DurableObjectStub<Table>, id: string, user: string): Promise<string> {
  const res = await call(stub, user, "GET", `/api/games/${id}/table`);
  const text = await res.text();
  expect(res.status, text).toBe(404);
  return (JSON.parse(text) as { error: string }).error;
}

/** The shell's lines among `lines`, less the slot's push and acknowledgement
 *  — the house's side of every ending, which `test/slot.test.ts` pins — and
 *  a counted wake's `wake` line, which `test/table.test.ts` pins (the
 *  orchestrator's ruling I-84; the pool counts wakes). */
function ends(lines: unknown[]): unknown[] {
  return lines.filter((l) => !["slot-pushed", "slot-acked", "wake"].includes(String((l as { event?: unknown }).event)));
}

/** The alarm, run, with every line it logs and every side channel it started
 *  settled. */
async function alarmRun(stub: DurableObjectStub<Table>): Promise<{ result: boolean; lines: unknown[] }> {
  return logsDuring(async () => {
    const ran = await runDurableObjectAlarm(stub);
    await settle(stub);
    return ran;
  });
}

describe("expiry (R-6.5c.15; spec §2.4)", () => {
  // Battery 30. The table's last activity is moved back past the TTL, then a
  // chat post UPDATES the chat row — a write that makes nothing due, so it
  // leaves the alarm where the last acknowledgement put it (spec §2.4's third
  // rule). The alarm fires BEFORE the new last activity plus the deadline it
  // is armed at — 7 days, the shorter TTL (R-6.5c2.42) — so it must re-read
  // the activity and re-arm to it. Mutants: `written_at` not advanced
  // by a write (the premise fails, and the table would expire); an alarm that
  // expires without re-reading the activity; an early alarm that re-arms
  // nothing.
  it("a write keeps an active table: the alarm re-reads the last activity and re-arms to it (battery 30)", async () => {
    const id = "ex-active";
    const stub = await seedPlaying(id);
    expect((await call(stub, BLUE, "POST", `/api/games/${id}/chat`, '{"text":"first"}')).status).toBe(201);
    await settle(stub);
    // Which write was the last activity when the last acknowledgement ran
    // depends on when its push landed, so the premise is that the alarm is
    // one of the table's writes plus the shorter TTL — an expiry deadline.
    const deadlines = (await sqlOf(stub, "SELECT written_at FROM docs WHERE kind IN ('record', 'table', 'chat')")).map(
      (r) => Number(r.written_at) + OPEN_TTL,
    );
    expect(deadlines, "premise: the last acknowledgement armed an expiry deadline").toContain(await alarmOf(stub));
    await backdate(stub, idle());
    const before = Date.now();
    expect((await call(stub, BLUE, "POST", `/api/games/${id}/chat`, '{"text":"still here"}')).status).toBe(201);
    const activity = await lastActivity(stub);
    expect(activity, "the post advanced the chat row's written_at").toBeGreaterThanOrEqual(before);
    const { result, lines } = await alarmRun(stub);
    expect([result, lines], "the alarm ran, and ended nothing").toEqual([true, []]);
    expect(await markerOf(stub)).toBeUndefined();
    expect(await docKinds(stub)).toEqual(["chat", "record", "table"]);
    expect(await alarmOf(stub), "re-armed to the new last activity + the shorter TTL").toBe(activity + OPEN_TTL);
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status).toBe(200);
  });

  // Spec §2.4's second rule, on its own: the success that discharges a
  // delivery re-arms the alarm to the next deadline — for a table with
  // nothing else due, its expiry, at the shorter TTL (R-6.5c2.42). Mutant: the
  // acknowledgement deletes the alarm, and an idle table never expires.
  it("the acknowledgement of a table's copy re-arms the alarm to its expiry deadline", async () => {
    const id = "ex-armed";
    const stub = tableStub(id);
    expect((await call(stub, RED, "POST", "/api/games", CREATE_BODY, id)).status).toBe(201);
    await settle(stub);
    const [table] = await sqlOf(stub, "SELECT written_at FROM docs WHERE kind = 'table'");
    expect(await alarmOf(stub)).toBe(Number(table?.written_at) + OPEN_TTL);
  });

  // Battery 29. An idle table expires as its CREATOR's own abandon: the
  // layer's delete refuses anyone else, so the deletion itself says whose it
  // was, and its save lands under the creator's `test/` root. The record's put
  // records which documents the object held at that moment — the put runs in
  // the object, so it can read them — and every one was still there: the save
  // landed BEFORE the delete. Mutants: the delete made before its save (the
  // rows gone at the put, or no put at all); the expiry dispatched as anyone
  // but the creator (refused, nothing deleted); a marker other than
  // `expired`, 90 days, `archive`, or timed by anything but the platform's
  // clock at the delete (the orchestrator's rulings I-43/I-44: the module's
  // frozen reading is days behind it); no `expired` line; the REMOVAL left for
  // its alarm rather than pushed after the expiry.
  it("an idle table expires at its TTL as its creator's abandon, saved before it is deleted (battery 29)", async () => {
    const id = "ex-idle";
    const stub = await seedPlaying(id);
    const record = await docOf(stub, "record");
    const [recordKey, tableKey] = [`test/rev7/abandoned/${id}.json`, `test/rev7/abandoned/${id}.table.json`];
    await backdate(stub, idle());
    let state: DurableObjectState | undefined;
    await runInDurableObject(stub, (_t, s) => {
      state = s;
    });
    let heldAtPut: unknown[] | undefined;
    const swapped = await swapBucket(stub, (real, key, value, options) => {
      if (key === recordKey) heldAtPut = state!.storage.sql.exec("SELECT kind FROM docs ORDER BY kind").toArray().map((r) => r.kind);
      return real.put(key, value, options);
    });
    let ran: { result: boolean; lines: unknown[] };
    const before = Date.now();
    try {
      ran = await alarmRun(stub);
    } finally {
      await swapped.restore();
    }
    const after = Date.now();
    expect(ran.result).toBe(true);
    expect(heldAtPut, "the record's put ran while the table still held its documents").toEqual(["record", "table"]);
    expect(await bucketText(recordKey), "the stored record's bytes").toBe(record);
    expect(archivedAt(await bucketText(tableKey)), "stamped at the expiry's one reading").toBe(1_789_000_000);
    const marker = await markerOf(stub);
    expect(marker).toEqual({ at: expect.any(Number), reason: "expired", ttl_days: 90, kept: "archive" });
    expect(marker!.at as number, "the marker's time is the platform's, at the delete").toBeGreaterThanOrEqual(before);
    expect(marker!.at as number).toBeLessThanOrEqual(after);
    expect(ends(ran.lines)).toEqual([
      { event: "archive-landed", id, site: "expiry", record_key: recordKey, detail: recordKey, attempt: 0 },
      { event: "expired", id, reason: "expired", kept: "archive" },
    ]);
    expect(await docKinds(stub), "every document removed").toEqual([]);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k = 'slot'"), "the REMOVAL pushed after the expiry, and acknowledged").toEqual([]);
    expect(await sentenceAt(stub, id, RED)).toBe(HEAD.expired(utcDate(marker!.at as number)) + TAIL.archive);
  });

  // Spec §2.4 step 2 for an expiry: ONE batch removes the documents and the
  // open picker's row, writes the marker and the REMOVAL, and sets the
  // removal's retry beside it. The removal's push is held, so no
  // acknowledgement's batch lands inside the watch. Mutants: the marker, or the
  // removal, in a batch of its own; the picker row left behind; no alarm.
  it("an expiry ends the table in ONE batch: the documents and the picker gone, the marker and the REMOVAL written", async () => {
    const id = "ex-batch";
    const stub = await seedPlaying(id);
    const opened = await call(stub, RED, "POST", `/api/games/${id}/input`, '{"line":"1"}');
    expect([opened.status, (await opened.json<Play>()).prompt.kind], "premise: `1` opens PlaceA").toEqual([200, "place_a"]);
    expect(await docKinds(stub), "premise: an open picker is stored").toEqual(["picker", "record", "table"]);
    await backdate(stub, idle());
    const removal = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, removal]]);
    try {
      const batches = await watchBatches(stub);
      const before = Date.now();
      expect((await logsDuring(() => runDurableObjectAlarm(stub))).result).toBe(true);
      expect(batches.statements, "anti-vacuity: the watch saw the object's SQL").toBeGreaterThan(0);
      expect({ batches: batches.batches, outside: batches.outside }).toEqual({ batches: 1, outside: [] });
      expect(await docKinds(stub)).toEqual([]);
      const slot = JSON.parse(String((await sqlOf(stub, "SELECT v FROM meta WHERE k = 'slot'"))[0]?.v)) as { revision: number; copy: unknown };
      expect([slot.revision, slot.copy]).toEqual([removal, null]);
      expect(await markerOf(stub)).toMatchObject({ reason: "expired", kept: "archive" });
      expect(await alarmOf(stub), "the removal's retry").toBeGreaterThanOrEqual(before + 30_000);
      held.release(id, removal);
    } finally {
      held.restore();
      await settle(stub);
    }
  });

  // Battery 29's other half. Every put fails: the expiry's delete is refused
  // by the layer, so the table stays, whole, and the alarm retries on the
  // backoff — 30 s after the first failure, 2 min after the second (spec §2.4)
  // — and then lands. Mutants: the table deleted anyway; no retry armed (the
  // second alarm would not run); the ladder not climbed; the failure not
  // logged.
  it("a failed save at expiry leaves the table and retries on the backoff until it lands", async () => {
    const id = "ex-retry";
    const stub = await seedPlaying(id);
    const record = await docOf(stub, "record");
    const recordKey = `test/rev7/abandoned/${id}.json`;
    const body = JSON.stringify({ error: `the archive could not be written (${R2_UNREACHABLE}) — the table was not deleted` });
    await backdate(stub, idle());
    const failing = await swapBucket(stub, failingPuts(2));
    try {
      for (const [attempt, rung] of [
        [0, 30_000],
        [1, 120_000],
      ] as const) {
        const t = Date.now();
        const { result, lines } = await alarmRun(stub);
        expect(result).toBe(true);
        expect(ends(lines), `failure ${attempt + 1}`).toEqual([
          { event: "archive-failed", id, site: "expiry", record_key: recordKey, detail: R2_UNREACHABLE, attempt },
          { event: "expiry-failed", id, status: 500, body, attempt, retry_in: rung },
        ]);
        expect(await docKinds(stub), "the table stays").toEqual(["record", "table"]);
        expect(await markerOf(stub)).toBeUndefined();
        const alarm = await alarmOf(stub);
        expect(alarm, `retried at the rung after ${attempt + 1} failure(s)`).toBeGreaterThanOrEqual(t + rung);
        expect(alarm!, "and no later than that rung").toBeLessThan(t + rung + 60_000);
      }
    } finally {
      await failing.restore();
    }
    expect(await bucketText(recordKey), "nothing landed while the puts failed").toBeNull();
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status, "the table still answers").toBe(200);
    await backdate(stub, idle());
    const { lines } = await alarmRun(stub);
    expect(ends(lines)).toEqual([
      { event: "archive-landed", id, site: "expiry", record_key: recordKey, detail: recordKey, attempt: 2 },
      { event: "expired", id, reason: "expired", kept: "archive" },
    ]);
    expect(await bucketText(recordKey)).toBe(record);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k = 'expiry_failed'"), "the count goes with the table").toEqual([]);
  });

  // Battery 39. Another game's record already holds this table's key, so no
  // retry can clear it (spec §2.4 step 1): the object writes `held`, logs it,
  // and stops arming the expiry deadline. An alarm set for other work later
  // finds it held — named in the log, never a second attempt. And the table
  // is still a table: its players can go on, and when a move ends red's turn
  // (the walk's own lines for `wire`, rows 49–57, measured at this seed) its
  // copy is delivered — and that acknowledgement re-arms no expiry either.
  // Once the conflicting record is gone, its creator's abandon saves and ends
  // it, and `held` goes with the table. Mutants: `held` not written — the
  // conflict retried for ever, an alarm at a rung and a second
  // `archive-conflict`; an expiry deadline still armed for a held table (an
  // idle one's would lie in the past and fire at once, so the delivery after
  // new activity is where it shows); `held` left beside the marker.
  it("a conflict at expiry writes held, logs it, and arms the expiry no more (battery 39)", async () => {
    const id = "ex-held";
    const stub = await seedPlaying(id);
    const recordKey = `test/rev7/abandoned/${id}.json`;
    await env.ARCHIVE!.put(recordKey, ANOTHER_RECORD);
    await backdate(stub, idle());
    const first = await alarmRun(stub);
    expect(first.result).toBe(true);
    expect(ends(first.lines)).toEqual([
      { event: "archive-conflict", id, site: "expiry", record_key: recordKey, detail: recordKey, attempt: 0 },
      { event: "held", id, record_key: recordKey },
    ]);
    const held = (await sqlOf(stub, "SELECT v FROM meta WHERE k = 'held'")).map((r) => JSON.parse(String(r.v)) as unknown);
    expect(held).toEqual([{ at: expect.any(Number), record_key: recordKey }]);
    expect(await docKinds(stub), "the table stays").toEqual(["record", "table"]);
    expect(await markerOf(stub)).toBeUndefined();
    expect(await alarmOf(stub), "nothing else is due, and the expiry no longer is").toBeNull();
    await runInDurableObject(stub, (_t, s) => s.storage.setAlarm(Date.now() + 60_000));
    const again = await alarmRun(stub);
    expect(ends(again.lines), "named, and not attempted").toEqual([{ event: "expiry-blocked", id, why: "held" }]);
    expect(await alarmOf(stub)).toBeNull();
    const acked = await revisionOf(stub);
    for (const line of ["2", "A09", "confirm", "1", "1", "1", "1", "1"]) {
      expect((await call(stub, RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }))).status, line).toBe(200);
    }
    await settle(stub);
    expect(await revisionOf(stub), "premise: the turn ended, and its copy was delivered").toBe(acked + 1);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k = 'slot'"), "premise: acknowledged").toEqual([]);
    expect(await alarmOf(stub), "the acknowledgement re-armed no expiry for a held table").toBeNull();
    expect(await bucketText(recordKey), "the archive keeps the record it held").toBe(ANOTHER_RECORD);
    await env.ARCHIVE!.delete(recordKey);
    expect((await logsDuring(() => call(stub, RED, "DELETE", `/api/games/${id}`))).result.status, "the creator's abandon, saved").toBe(204);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k IN ('held', 'marker') ORDER BY k"), "held goes with the table").toEqual([{ k: "marker" }]);
    await settle(stub);
  });

  // Spec §2.4: "a POISONED table does not expire on its own". Its load traps,
  // so the alarm names it and arms nothing for it. Mutant: the expiry run on a
  // poisoned table (a wake that traps a third time, or a delete through the
  // layer it cannot load).
  it("a poisoned table does not expire on its own: the alarm names it and arms nothing", async () => {
    const id = "ex-poisoned";
    const stub = await seedPlaying(id);
    await runInDurableObject(stub, (_t, s) => {
      s.storage.transactionSync(() => {
        s.storage.sql.exec("INSERT INTO meta (k, v) VALUES ('load_traps', '2'), ('poison', 'record')");
      });
    });
    await backdate(stub, idle());
    const { result, lines } = await alarmRun(stub);
    expect([result, ends(lines)]).toEqual([true, [{ event: "expiry-blocked", id, why: "poisoned" }]]);
    expect(await docKinds(stub)).toEqual(["record", "table"]);
    expect(await markerOf(stub)).toBeUndefined();
    expect(await alarmOf(stub)).toBeNull();
  });

  // Spec §2.6 and §2.4: with the archive OFF an expired game is simply gone,
  // and the marker says so; no put is made. Mutants: a put made with the
  // switch off; the tail `archive` or `nothing` for a game the switch kept
  // from the archive.
  it("with the archive off, an idle game expires without a save, and its marker says so", async () => {
    const id = "ex-off";
    const stub = await seedPlaying(id);
    await backdate(stub, idle());
    const off = archiveOff();
    let ran: { result: boolean; lines: unknown[] };
    try {
      await rebuilt(stub);
      ran = await alarmRun(stub);
    } finally {
      off.restore();
    }
    expect([ran.result, ends(ran.lines)]).toEqual([true, [{ event: "expired", id, reason: "expired", kept: "off" }]]);
    const marker = await markerOf(stub);
    expect(marker).toEqual({ at: expect.any(Number), reason: "expired", ttl_days: 90, kept: "off" });
    expect(await bucketText(`test/rev7/abandoned/${id}.json`)).toBeNull();
    expect(await sentenceAt(stub, id, RED)).toBe(HEAD.expired(utcDate(marker!.at as number)) + TAIL.off);
  });

  // Spec §2.6: an expiry WHILE the archive is off takes with it a game-over
  // save still owed from when it was on, and the shell logs that. The owed
  // save is arranged as battery 27's is; the alarm then keeps it (the switch
  // reads off) and the expiry drops it. Mutants: the dropped save not logged;
  // the save still owed by a table that is gone.
  it("an expiry while the archive is off takes an owed save with it, and logs it", async () => {
    const id = "ex-dropped";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const failing = await swapBucket(stub, failingPuts(1));
    try {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    } finally {
      await failing.restore();
    }
    expect((await sqlOf(stub, "SELECT k FROM meta WHERE k = 'archive_owed'")).length, "premise: a save is owed").toBe(1);
    await backdate(stub, idle());
    const off = archiveOff();
    let ran: { result: boolean; lines: unknown[] };
    try {
      await rebuilt(stub);
      ran = await alarmRun(stub);
    } finally {
      off.restore();
    }
    expect(ends(ran.lines)).toEqual([
      { event: "archive-owed-kept", id, record_key: recordKey, attempt: 1 },
      { event: "archive-owed-dropped", id, record_key: recordKey, attempt: 1 },
      { event: "expired", id, reason: "expired", kept: "off" },
    ]);
    expect(await sqlOf(stub, "SELECT k FROM meta WHERE k = 'archive_owed'")).toEqual([]);
    expect(await bucketText(recordKey), "nothing uploaded").toBeNull();
  });

  // The orchestrator's ruling I-34: the TTL is read through ONE exported
  // reader, replaced here at module scope as if `YOKOHAMA_WEB_TABLE_TTL_DAYS`
  // were unset. Mutants: the TTL read around the reader; an unset TTL read
  // as zero days; an expiry deadline armed with no TTL.
  it("with no TTL, no table expires, and no expiry deadline is armed", async () => {
    const id = "ex-no-ttl";
    const noTtl = vi.spyOn(markerModule, "ttlDaysOf").mockReturnValue(null);
    try {
      const stub = await seedPlaying(id);
      expect(await alarmOf(stub), "the acknowledgements armed nothing: nothing is due").toBeNull();
      await backdate(stub, Date.now() - 1_000 * DAY_MS);
      await runInDurableObject(stub, (_t, s) => s.storage.setAlarm(Date.now() + 60_000));
      const { result, lines } = await alarmRun(stub);
      expect([result, lines]).toEqual([true, []]);
      expect(await docKinds(stub)).toEqual(["record", "table"]);
      expect(await markerOf(stub)).toBeUndefined();
      expect(await alarmOf(stub)).toBeNull();
    } finally {
      noTtl.mockRestore();
    }
  });
});

describe("the waiting-table expiry (R-6.5c2.42; the plan's R-E8; security S3, D9)", () => {
  // Review Focus 2. A table's alarm is ARMED from SQL alone, which cannot read
  // its status (the strings rule), at the SHORTER deadline — its last activity
  // plus 7 days — and DECIDED at the alarm by the status its house copy reads
  // (`copyFacts`, the one sanctioned parse). A table that has not started —
  // open, or dealing with a keep outstanding — waits the 7 days (security S3,
  // D9; design §3.2 rule 2 and D-5, replacing ruling 9-b's dealing table that
  // waited the 90). Three tables:
  //
  // - one still OPEN, a seat unfilled: armed at 7 days; advanced 7 days and
  //   run, it is gone, its marker the waited head, and `expired` logged with
  //   the reason;
  // - one PLAYING (dealt, and a move made): armed at 7 days all the same, and
  //   at that alarm NOT expired — re-armed to its last activity plus 90 days;
  // - one created `open` and FILLED on day 6 by a second identity, `dealing`
  //   with both keeps outstanding: it survives day 7, a day after the join
  //   (re-armed to the join plus 7), and is closed on day 13, the join's 7
  //   days past, as one that `waited`, its marker the waited head with
  //   `ttl_days` 7.
  //
  // Mutants: the alarm armed at the TTL (the waiting table's premise, 90 days
  // out); the status ignored — every table expired at its 7 days (the playing
  // table ends), or none (the waiting table stands); the playing table re-armed
  // at the 7 days already past, where it would fire again at once; `dealing`
  // read as not waiting — `open` alone (the filled table survives day 13); the
  // waited marker's reason, its days, or its head wrong.
  it("a table that has not started expires at 7 days — open, or filled and dealing; a playing one is re-armed to its 90 (Review Focus 2; security S3, D9)", async () => {
    const waiting = "wt-open";
    const open = tableStub(waiting);
    expect((await call(open, RED, "POST", "/api/games", CREATE_BODY, waiting)).status).toBe(201);
    await settle(open);
    expect(await alarmOf(open), "armed at the shorter deadline: the last activity plus 7 days").toBe(
      (await lastActivity(open)) + OPEN_TTL,
    );
    await backdate(open, Date.now() - OPEN_TTL - 60_000);
    const gone = await alarmRun(open);
    expect(gone.result).toBe(true);
    expect(ends(gone.lines), "expired, and logged with the reason").toEqual([{ event: "expired", id: waiting, reason: "waited", kept: "nothing" }]);
    const marker = await markerOf(open);
    expect(marker).toEqual({ at: expect.any(Number), reason: "waited", ttl_days: 7, kept: "nothing" });
    expect(await docKinds(open), "every document removed").toEqual([]);
    expect(await sentenceAt(open, waiting, RED)).toBe(HEAD.waited(utcDate(marker!.at as number)) + TAIL.nothing);

    const playing = "wt-playing";
    const dealt = await seedPlaying(playing);
    expect(await alarmOf(dealt), "armed from SQL at 7 days, whatever the status").toBe((await lastActivity(dealt)) + OPEN_TTL);
    await backdate(dealt, Date.now() - OPEN_TTL - 60_000);
    const kept = await alarmRun(dealt);
    expect([kept.result, ends(kept.lines)], "the alarm ran, and ended nothing").toEqual([true, []]);
    expect(await markerOf(dealt)).toBeUndefined();
    expect(await docKinds(dealt)).toEqual(["record", "table"]);
    expect(await alarmOf(dealt), "re-armed to the last activity plus 90 days").toBe((await lastActivity(dealt)) + TTL);
    expect((await call(dealt, RED, "GET", `/api/games/${playing}`)).status, "the table still answers").toBe(200);

    const late = "wt-filled";
    const filled = tableStub(late);
    expect((await call(filled, RED, "POST", "/api/games", CREATE_BODY, late)).status).toBe(201);
    await settle(filled);
    // Created on day 0; it is day 6, and blue fills the open seat.
    await backdate(filled, Date.now() - 6 * DAY_MS);
    const joined = await call(filled, BLUE, "POST", `/api/games/${late}/join`, '{"seat":1}');
    expect([joined.status, (await joined.json<{ status: string }>()).status], "filled on day 6: dealing, no keep made").toEqual([200, "dealing"]);
    await settle(filled);
    const survives = async (day: number, backTo: number) => {
      await backdate(filled, backTo);
      const run = await alarmRun(filled);
      expect([run.result, ends(run.lines)], `day ${day}: the alarm ran, and ended nothing`).toEqual([true, []]);
      expect(await markerOf(filled), `day ${day}`).toBeUndefined();
      expect((await call(filled, RED, "GET", `/api/games/${late}/table`)).status, `day ${day}: the table still answers`).toBe(200);
    };
    // Day 7: a day after the join, before its own 7 days.
    await survives(7, Date.now() - DAY_MS - 60_000);
    expect(await alarmOf(filled), "day 7: re-armed to the join plus 7 days").toBe((await lastActivity(filled)) + OPEN_TTL);
    // Day 12: six days after the join, a day short of its 7 — the table
    // stands, not closed as one that waited (the waited deadline a day early, at the alarm's gate and in `expiryReason`, reads red here).
    await survives(12, Date.now() - 6 * DAY_MS);
    // Day 13: the join's 7 days are past, and the table — dealing, no keep
    // made — has not started.
    await backdate(filled, Date.now() - OPEN_TTL - 60_000);
    const ended = await alarmRun(filled);
    expect(ends(ended.lines), "day 13: closed as one that waited").toEqual([{ event: "expired", id: late, reason: "waited", kept: "nothing" }]);
    const closed = await markerOf(filled);
    expect(closed).toEqual({ at: expect.any(Number), reason: "waited", ttl_days: 7, kept: "nothing" });
    expect(await docKinds(filled), "every document removed").toEqual([]);
    expect(await sentenceAt(filled, late, RED)).toBe(HEAD.waited(utcDate(closed!.at as number)) + TAIL.nothing);
  });

  // **The unread corner** (design §3.2 rule 2, D-16): a dealing table
  // finalized but unflipped — both keeps made, the record stored, its document
  // still at `dealing` — that NOBODY reads for the whole waiting-table TTL is
  // CLOSED on the Worker as one that `waited`. The alarm reads `waiting` from
  // the stored, un-settled document before the delete (`lifecycle.ts`'s
  // `expiry` and `waiting`), `settle` runs only inside the delete's `events`
  // read, and `gameOf` keeps a table only when a move was made — so the loss
  // is a game with no move, accepted over a shell that settles before it
  // reads. The NAS parts here, by design: its sweep settles first and keeps
  // the table (`crates/yokohama-web/src/sweep.rs`, the settle-first cell).
  //
  // The stuck document is made as the layer's settle cell makes it
  // (`tables.rs`, `a_table_left_dealing_with_every_keep_present_settles_on_the_next_read`):
  // a real finalize, its table text put back at `dealing` with no turn, and
  // the object rebuilt so its wake loads that text. No request sequence leaves
  // it here: a request's record and table reach SQL in ONE batch, so the
  // crash between the last keep's two writes is the NAS's window. The
  // anti-vacuity partner is the same shape READ once: the read repairs it to
  // `playing`, and that table's alarm at 7 days ends nothing.
  //
  // Mutants: `waiting` reading `open` alone (the unread table survives); a
  // shell that settles before it reads (the unread table survives — the
  // design's rejected alternative); a fixture that is not the repairable shape
  // (the read partner stays `dealing`).
  it("the unread corner (D-16): a finalized-but-unflipped dealing table nobody read is closed as waited, and one read is repaired and kept", async () => {
    /** The table at `id` finalized with no move, its stored document put back
     *  at `dealing` with no turn, its object rebuilt. */
    async function unflipped(id: string): Promise<DurableObjectStub<Table>> {
      const stub = tableStub(id);
      const steps: Array<[string, string, string]> = [
        [RED, "/api/games", CREATE_BODY],
        [BLUE, `/api/games/${id}/join`, '{"seat":1}'],
        [RED, `/api/games/${id}/keep`, '{"seat":0,"card":1}'],
        [BLUE, `/api/games/${id}/keep`, '{"seat":1,"card":1}'],
      ];
      for (const [user, path, body] of steps) {
        const res = await call(stub, user, "POST", path, body, path === "/api/games" ? id : undefined);
        expect(res.status, `${user} ${path}: ${await res.clone().text()}`).toBeLessThan(300);
      }
      const game = await call(stub, RED, "GET", `/api/games/${id}`);
      expect((await game.json<Play>()).events, "premise: playing, and no move made").toBe(0);
      await settle(stub);
      const table = (await docOf(stub, "table")) ?? "";
      const turns = table.match(/"turn": \{[^{}]*\}/g) ?? [];
      expect(turns.length, "premise: one turn mirror, flat").toBe(1);
      const stuck = replaceOnce(replaceOnce(table, '"status": "playing"', '"status": "dealing"'), turns[0]!, '"turn": null');
      await runInDurableObject(stub, (_t, state) => {
        state.storage.transactionSync(() => state.storage.sql.exec("UPDATE docs SET json = ? WHERE kind = 'table'", stuck));
      });
      await rebuilt(stub);
      expect(await docOf(stub, "table"), "premise: the stuck text is what the wake loads").toBe(stuck);
      expect(await docOf(stub, "record"), "premise: the record stored").toBeDefined();
      return stub;
    }

    const unread = "wt-unflipped";
    const stuck = await unflipped(unread);
    await backdate(stuck, Date.now() - OPEN_TTL - 60_000);
    const gone = await alarmRun(stuck);
    expect(ends(gone.lines), "closed as one that waited, its game with no move not kept").toEqual([
      { event: "expired", id: unread, reason: "waited", kept: "nothing" },
    ]);
    const marker = await markerOf(stuck);
    expect(marker).toEqual({ at: expect.any(Number), reason: "waited", ttl_days: 7, kept: "nothing" });
    expect(await docKinds(stuck), "every document removed").toEqual([]);
    expect(await bucketText(`test/rev7/abandoned/${unread}.json`), "nothing filed").toBeNull();

    const read = "wt-unflipped-read";
    const opened = await unflipped(read);
    const view = await call(opened, RED, "GET", `/api/games/${read}/table`);
    expect([view.status, (await view.json<{ status: string }>()).status], "premise: the stuck shape is repairable — one read flips it").toEqual([200, "playing"]);
    await settle(opened);
    await backdate(opened, Date.now() - OPEN_TTL - 60_000);
    const kept = await alarmRun(opened);
    expect([kept.result, ends(kept.lines)], "the alarm ran, and ended nothing").toEqual([true, []]);
    expect(await markerOf(opened)).toBeUndefined();
    expect(await docKinds(opened)).toEqual(["record", "table"]);
    expect(await alarmOf(opened), "re-armed to the last activity plus 90 days").toBe((await lastActivity(opened)) + TTL);
  });
});

describe("the marker (spec §2.4 steps 5–6)", () => {
  // Battery 31. From the delete's own batch, every request is the marker's 404
  // with its sentence — a read, `…/events`, a move, and an upgrade, which is
  // answered 404 UNUPGRADED so the client's probe reads it (spec §2.7) — for
  // the creator, another player and no one alike. A create at the id is 409:
  // before the house has acknowledged the removal, and after it, under the
  // FROZEN clock — the same second as the table it replaced (Task 12's M4,
  // closed here): the marker refuses the create, whatever the two tables'
  // generations would read. Mutants: the marker dropped (the
  // layer's bare "no game named …"); the upgrade accepted and closed; the 409
  // only while the removal waits; a 409 that writes.
  it("while a marker stands every request is its 404 — an upgrade unupgraded — and a create is 409, before the ack and after it (battery 31)", async () => {
    const id = "ex-marked";
    const stub = await seedPlaying(id);
    const removal = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, removal]]);
    let sentence = "";
    try {
      expect((await call(stub, RED, "DELETE", `/api/games/${id}`)).status).toBe(204);
      const marker = await markerOf(stub);
      sentence = HEAD.abandoned(utcDate(marker!.at as number)) + TAIL.archive;
      const answers = [
        await call(stub, RED, "GET", `/api/games/${id}`),
        await call(stub, BLUE, "GET", `/api/games/${id}/table`),
        await call(stub, RED, "GET", `/api/games/${id}/events`),
        await call(stub, RED, "POST", `/api/games/${id}/input`, '{"line":"1"}'),
        await call(stub, RED, "DELETE", `/api/games/${id}`),
        await stub.fetch(`https://yokohama.test/api/games/${id}/table`),
      ];
      for (const res of answers) expect([res.status, await res.json()]).toEqual([404, { error: sentence }]);
      const upgrade = await stub.fetch(`https://yokohama.test/api/games/${id}/stream`, {
        headers: { Upgrade: "websocket", "X-Yokohama-User": RED },
      });
      expect([upgrade.status, upgrade.webSocket, await upgrade.json()], "the upgrade, unupgraded").toEqual([404, null, { error: sentence }]);
      const before = await sqlOf(stub, "SELECT k, v FROM meta ORDER BY k");
      const waiting = await call(stub, RED, "POST", "/api/games", CREATE_BODY, id);
      expect([waiting.status, await waiting.json()], "a create while the removal waits").toEqual([
        409,
        { error: "this name was used recently — try another" },
      ]);
      expect(await sqlOf(stub, "SELECT k, v FROM meta ORDER BY k"), "the 409 wrote nothing").toEqual(before);
      held.release(id, removal);
    } finally {
      held.restore();
      await settle(stub);
    }
    const acked = await sqlOf(stub, "SELECT k FROM meta WHERE k = 'slot'");
    expect(acked, "premise: the house acknowledged the removal").toEqual([]);
    const again = await call(stub, RED, "POST", "/api/games", CREATE_BODY, id);
    expect([again.status, await again.json()], "a create in the same second, after the ack").toEqual([
      409,
      { error: "this name was used recently — try another" },
    ]);
    expect(await sentenceAt(stub, id, RED)).toBe(sentence);
  });

  // Battery 11. `deleteAll()` needs BOTH: the house's acknowledgement of the
  // removal AND the marker's 90 days (spec §2.4 step 6). A — acknowledged, not
  // 90 days: the acknowledgement re-arms the alarm to the marker's end, and
  // an alarm run then ends nothing. B — 90 days, not acknowledged: the pushes
  // fail, the marker is backdated past its end, and the alarm's run pushes
  // (and fails) rather than ending the object; once the push lands, the
  // acknowledgement ends it, logged. A returning player then reads the layer's
  // own "no game named …". Mutants: `deleteAll()` at the acknowledgement
  // whatever the days (A ends); at the 90 days whatever the acknowledgement
  // (B ends at its first run); no `marker-ended` line.
  it("a marker ends only once the removal is acknowledged AND its 90 days have passed (battery 11)", async () => {
    const a = "ex-end-ack";
    const stubA = await seedPlaying(a);
    expect((await call(stubA, RED, "DELETE", `/api/games/${a}`)).status).toBe(204);
    await settle(stubA);
    expect(await sqlOf(stubA, "SELECT k FROM meta WHERE k = 'slot'"), "premise: the removal is acknowledged").toEqual([]);
    const markerA = (await markerOf(stubA))!;
    expect(await alarmOf(stubA), "the acknowledgement re-armed to the marker's end").toBe((markerA.at as number) + 90 * DAY_MS);
    const early = await alarmRun(stubA);
    expect([early.result, ends(early.lines)]).toEqual([true, []]);
    expect(await markerOf(stubA), "not yet 90 days: the marker stands").toEqual(markerA);
    expect(await alarmOf(stubA)).toBe((markerA.at as number) + 90 * DAY_MS);

    const b = "ex-end-90";
    const stubB = await seedPlaying(b);
    const failing = await failPushes(stubB, 2);
    let at = 0;
    try {
      expect((await call(stubB, RED, "DELETE", `/api/games/${b}`)).status).toBe(204);
      await settle(stubB);
      const marker = (await markerOf(stubB))!;
      at = (marker.at as number) - 91 * DAY_MS;
      await runInDurableObject(stubB, (_t, s) => {
        s.storage.transactionSync(() => s.storage.sql.exec("UPDATE meta SET v = ? WHERE k = 'marker'", JSON.stringify({ ...marker, at })));
      });
      const unacked = await alarmRun(stubB);
      expect(unacked.result).toBe(true);
      expect(unacked.lines.map((l) => (l as { event: string }).event), "the alarm pushed the removal, and it failed").toEqual([
        "slot-pushed",
        "slot-failed",
      ]);
      expect(await tablesIn(stubB), "unacknowledged: the object stands").toBeGreaterThan(0);
      expect((await markerOf(stubB))?.at).toBe(at);
    } finally {
      await failing.restore();
    }
    const ended = await alarmRun(stubB);
    expect(ends(ended.lines)).toEqual([{ event: "marker-ended", id: b, reason: "abandoned", at, kept: "archive" }]);
    expect(await tablesIn(stubB), "acknowledged and 90 days: the object has ceased to exist").toBe(0);
    expect(await alarmOf(stubB)).toBeNull();
    const back = await call(stubB, RED, "GET", `/api/games/${b}/table`);
    expect([back.status, await back.json()]).toEqual([404, { error: `no game named "${b}"` }]);
  });
});

describe("the marker's tail, by what was kept, under each head (spec §2.4 step 2; §5.2)", () => {
  type Head = "abandoned" | "expired" | "operator";
  type Arranged = "saved" | "stamped" | "stamped-off" | "open" | "dealing" | "no-move" | "off";
  /** Spec §2.4 step 2 as the orchestrator's rulings I-64 and I-71 read it. */
  const KEPT: Record<Arranged, "archive" | "nothing" | "off"> = {
    saved: "archive",
    stamped: "archive",
    "stamped-off": "archive",
    open: "nothing",
    dealing: "nothing",
    "no-move": "nothing",
    off: "off",
  };
  /** The arrangements whose ending runs with the archive switched off. */
  const SWITCHED_OFF: readonly Arranged[] = ["off", "stamped-off"];

  /** A table at `id` as `arranged` names it, and its creator. */
  async function arrange(arranged: Arranged, id: string): Promise<{ stub: DurableObjectStub<Table>; creator: string }> {
    if (arranged === "saved" || arranged === "off") return { stub: await seedPlaying(id), creator: RED };
    if (arranged === "stamped" || arranged === "stamped-off") {
      const stub = await owedGameOver(id);
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
      expect(archivedAt(await docOf(stub, "table")), "premise: stamped at game over, at the pool's frozen reading").toBe(1_789_000_000);
      return { stub, creator: LOCAL };
    }
    const stub = tableStub(id);
    expect((await call(stub, RED, "POST", "/api/games", CREATE_BODY, id)).status).toBe(201);
    if (arranged !== "open") expect((await call(stub, BLUE, "POST", `/api/games/${id}/join`, '{"seat":1}')).status).toBe(200);
    if (arranged === "no-move") {
      expect((await call(stub, RED, "POST", `/api/games/${id}/keep`, '{"seat":0,"card":1}')).status).toBe(200);
      expect((await call(stub, BLUE, "POST", `/api/games/${id}/keep`, '{"seat":1,"card":1}')).status).toBe(200);
      const game = await call(stub, RED, "GET", `/api/games/${id}`);
      expect((await game.json<Play>()).events, "premise: playing, and no move made").toBe(0);
    }
    await settle(stub);
    return { stub, creator: RED };
  }

  /** The table ended by `head`: its creator's DELETE, its expiry, or an
   *  operator's plain expire. */
  async function end(head: Head, stub: DurableObjectStub<Table>, id: string, creator: string): Promise<void> {
    if (head === "abandoned") expect((await call(stub, creator, "DELETE", `/api/games/${id}`)).status).toBe(204);
    if (head === "operator") expect((await operate(stub, id, BOSS)).status).toBe(204);
    if (head === "expired") {
      await backdate(stub, idle());
      expect(await runDurableObjectAlarm(stub)).toBe(true);
    }
    await settle(stub);
  }

  // `stamped-off` is the review's probe (the orchestrator's ruling I-71): a
  // game stamped at game over while the archive was on, ended after the switch
  // reads off. Its game is in the bucket, so its tail is the archive's, and
  // the bucket still holds it. Mutants: a stamped game — deleted with no save
  // while the archive is on — read as `nothing` (the ruling's `status:
  // playing` taken literally: a finished game's reading says `over`); a
  // stamped game read as `off` once the switch is off (the stamp consulted
  // after the switch); a table with no move read as a game; the switch
  // ignored; a head under the wrong reason; `ttl_days` missing from an
  // expired marker. **The alarm's `open` and `dealing` arrangements end as
  // `waited`** (R-6.5c2.42; security S3, D9): a table that has not started —
  // open, or dealing with a keep outstanding — idle past both TTLs, is closed
  // as one that waited 7 days — its status decides, whatever else is past.
  // Mutants: the TTL asked before the status (`expired` here); `dealing` read
  // as not waiting (`expired` for the dealing arrangement).
  it.each(
    (["abandoned", "expired", "operator"] as const).flatMap((head) =>
      (["saved", "stamped", "stamped-off", "open", "dealing", "no-move", "off"] as const).map((arranged) => [head, arranged] as const),
    ),
  )("%s: %s", async (head, arranged) => {
    const id = `tail-${arranged}-${head}`;
    const { stub, creator } = await arrange(arranged, id);
    const record = await docOf(stub, "record");
    const off = SWITCHED_OFF.includes(arranged) ? archiveOff() : undefined;
    try {
      if (off !== undefined) await rebuilt(stub);
      await logsDuring(() => end(head, stub, id, creator));
    } finally {
      off?.restore();
    }
    const reason = head === "expired" && (arranged === "open" || arranged === "dealing") ? "waited" : head;
    const days = reason === "expired" ? { ttl_days: 90 } : reason === "waited" ? { ttl_days: 7 } : {};
    const marker = await markerOf(stub);
    expect(marker).toEqual({ at: expect.any(Number), reason, ...days, kept: KEPT[arranged] });
    expect(await sentenceAt(stub, id, creator)).toBe(HEAD[reason](utcDate(marker!.at as number)) + TAIL[KEPT[arranged]]);
    const abandonedKey = `${creator === LOCAL ? "" : "test/"}rev7/abandoned/${id}.json`;
    expect((await bucketText(abandonedKey)) !== null, "this ending made a save exactly when it had a game to save").toBe(arranged === "saved");
    if (arranged === "stamped" || arranged === "stamped-off") {
      expect(await bucketText(`rev7/over/${id}.json`), "the game over's save is still in the bucket, its record's bytes").toBe(record);
    }
  });
});
