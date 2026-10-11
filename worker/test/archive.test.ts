import { env } from "cloudflare:workers";
import { evictDurableObject, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it, vi } from "vitest";
import type { Archive } from "../src/archive";
import * as clockModule from "../src/clock";
import type { House } from "../src/house";
import type { ArchiveOwed } from "../src/r2";
import { HOUSE_NAME } from "../src/slot";
import type { Table } from "../src/table";
import { OWED_GAME_OVER_JSON } from "./fixtures.gen";
import { upsertDoc } from "../src/storage";
import { failingPuts, failPushes, heldInTurn, heldPuts, holdApply, R2_UNREACHABLE, swapBucket } from "./gates";
import {
  ANOTHER_RECORD,
  BLUE,
  DAY_MS,
  LOCAL,
  OWED,
  RED,
  alarmOf,
  archiveOff,
  archivedAt,
  backdate,
  bucketText,
  call,
  docOf,
  expiryOf,
  holding,
  logsDuring,
  markerOf,
  noTtl,
  owedGameOver,
  rebuilt,
  replaceOnce,
  revisionOf,
  seedPlaying,
  segmentOf,
  settle,
  sqlOf,
  storedRows,
  watchBatches,
  type Play,
} from "./helpers";
import { ARCHIVE_WALK } from "./walk";

// **The archive on R2** (spec §2.6, R-6.5c.9, R-6.5c.14; the plan's Task 13).
//
// Storage and the bucket are per FILE (pool fact 5), so every cell uses table
// ids and callers of its own. The committed owed-game-over pair is written
// under its own id, `local-242a5429453c593c`, by the first game-over cell
// (plan review m12); every other cell that needs it writes it under a name of
// its own, with the table document's `id` rewritten to that name in its TEXT,
// so the document's key is still its object's name — m12's reason, which is
// `write_table` keying by the document's `id`. Every patch is restored in a
// `finally` (pool fact 2), and every cell that holds a push or an upload then
// settles its table there too: a cell that fails with one still in flight
// would otherwise leave it running into the file's teardown, and the run
// never ends (measured).
//
// The clock TICKS where a cell reads the readings (P-2): the exported
// `nextReading` is replaced at MODULE scope (pool fact 4; the orchestrator's
// rulings I-43/I-44), so a re-dispatch or a stamp that took a reading of its
// own would stamp a different `archived_at` than the one its receipt is for.

/** The walk's first seat — at the one-person table, both seats. */
const WIRE_RED = "red@wire.test";

/**
 * The walk's own finished game at a ONE-PERSON table (the orchestrator's
 * ruling on Task 6's escalation b): `archive-walk.json`'s first send carries
 * the finished record and its table, and the table's second seat is given to
 * the first seat's player. With one distinct human, that human holds the
 * acting seat at game over, so the layer admits their undo — which the
 * owed-game-over pair's bot seat forbids.
 */
function oneSeatGame(name: string): Promise<DurableObjectStub<Table>> {
  const { record, table } = ARCHIVE_WALK[0]!.body as { record: string; table: string };
  const hot = replaceOnce(replaceOnce(table, '"user": "blue@wire.test"', `"user": "${WIRE_RED}"`), '"id": "wire"', `"id": "${name}"`);
  return holding(name, { table: hot, record });
}

/**
 * The walk's last input lines, from the decision two undos reach at the
 * one-person table: `1`, `0`, `confirm`, `1` end the game again with the
 * walk's own record, byte for byte; `1`, `1`, `confirm`, `1` answer the
 * second line otherwise and end the SAME game with another record. The
 * refinish cells assert both as premises.
 */
const ORIGINAL_END = ["1", "0", "confirm", "1"] as const;
const DIFFERENT_END = ["1", "1", "confirm", "1"] as const;

/** `lines` played in order through `input`, each held to `play`'s premise;
 *  answers the last play. */
async function playAll(
  input: (line: string) => Promise<Response>,
  play: (res: Response) => Promise<Play>,
  lines: readonly string[],
): Promise<Play> {
  let last: Play | undefined;
  for (const line of lines) last = await play(await input(line));
  return last!;
}

/** `meta.archive_owed` — the shell's own JSON, whose plan's record and table
 *  are strings inside it — or `undefined` when no save is owed. */
async function owedOf(stub: DurableObjectStub<Table>): Promise<ArchiveOwed | undefined> {
  const rows = await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'meta'").one().n === 1;
    return exists ? sql.exec<{ v: string }>("SELECT v FROM meta WHERE k = 'archive_owed'").toArray() : [];
  });
  return rows[0] === undefined ? undefined : (JSON.parse(rows[0].v) as ArchiveOwed);
}

/** The owed save's gate (`not_before`, R-D6) moved one millisecond into the
 *  past: its rung has passed. An edit of the row's TEXT, never a parse and
 *  re-serialization — the plan inside it carries a record. */
async function rungPassed(stub: DurableObjectStub<Table>): Promise<void> {
  const gate = (await owedOf(stub))?.not_before;
  if (gate === undefined) throw new Error("premise: a save is owed, and a failure has gated it");
  await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const [row] = sql.exec<{ v: string }>("SELECT v FROM meta WHERE k = 'archive_owed'").toArray();
    const moved = replaceOnce(row!.v, `"not_before":${gate}`, `"not_before":${Date.now() - 1}`);
    state.storage.transactionSync(() => sql.exec("UPDATE meta SET v = ? WHERE k = 'archive_owed'", moved));
  });
}

/** Every reading P-2 hands out, ticking one second each from `from`. */
function ticking(from: number): { readings: number[]; restore(): void } {
  const readings: number[] = [];
  let next = from;
  const spy = vi.spyOn(clockModule, "nextReading").mockImplementation(() => {
    readings.push(next);
    return next++;
  });
  return { readings, restore: () => spy.mockRestore() };
}

/** How long a cell waits for something it EXPECTS, under vitest's 5 s per
 *  cell, so the failure names what it waited for. */
const WAIT_MS = 2_000;

/** `promise`'s answer, or a failure naming `what` once `WAIT_MS` pass: for a
 *  reply that must come while something else is held. */
async function within<T>(promise: Promise<T>, what: string): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error(`${what}: not within ${WAIT_MS} ms`)), WAIT_MS);
  });
  try {
    return await Promise.race([promise, late]);
  } finally {
    if (timer !== undefined) clearTimeout(timer);
  }
}

/** Polls `check` until it holds; fails naming `what` after `WAIT_MS`. */
async function until(check: () => boolean | Promise<boolean>, what: string): Promise<void> {
  const deadline = Date.now() + WAIT_MS;
  while (!(await check())) {
    if (Date.now() > deadline) throw new Error(`${what}: not within ${WAIT_MS} ms`);
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

/** The shell's archive lines among `lines` (spec §2.8): every receipt
 *  outcome, every stamp, and an owed save kept while the switch is off. */
function archiveLines(lines: unknown[]): unknown[] {
  return lines.filter((l) => String((l as { event?: unknown }).event).startsWith("archive-"));
}

/** A game-over outcome's line: `attempt` is how many uploads of this plan had
 *  failed before this one. */
function overLine(outcome: string, id: string, recordKey: string, attempt: number, detail = recordKey) {
  return { event: `archive-${outcome}`, id, site: "game-over", record_key: recordKey, detail, attempt };
}

/** A stamp's line: the plan's recorded reading, and the module's two facts. */
function stampLine(id: string, recordKey: string, reading: number, stamped: boolean, over: boolean) {
  return { event: "archive-stamped", id, record_key: recordKey, reading, stamped, over };
}

/** The house, reached as the Worker reaches it: by its one name. */
function houseStub(): DurableObjectStub<House> {
  return env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME));
}

/** The archive object `caller`'s saves go to, as the Worker names it. */
function archiveStub(caller: string): DurableObjectStub<Archive> {
  return env.ARCHIVER.get(env.ARCHIVER.idFromName(`archive:${caller}`));
}

/** A browser's save of `body` through `caller`'s archive object, answered
 *  whole. */
async function save(caller: string, body: unknown): Promise<{ status: number; text: string }> {
  const res = await archiveStub(caller).fetch("https://yokohama.test/api/archive", {
    method: "POST",
    headers: { "X-Yokohama-User": caller, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  return { status: res.status, text: await res.text() };
}

describe("the abandon's receipt loop (spec §2.6)", () => {
  // Battery 10. Every put fails, so the upload is `failed` and the re-dispatch
  // is refused in the layer's own sentence, R2's message inside it. Mutants:
  // the failed outcome installed as landed, or the refusal answered with a
  // delete — the table gone; the refusal written back as a batch.
  it("an abandon whose put fails leaves the object, its storage and the layer's refusal in place (battery 10)", async () => {
    const id = "ab-fail";
    const stub = await seedPlaying(id);
    const before = await storedRows(stub);
    const failing = await swapBucket(stub, failingPuts(Infinity));
    try {
      const { result: res, lines } = await logsDuring(() => call(stub, RED, "DELETE", `/api/games/${id}`));
      expect([res.status, await res.json()]).toEqual([
        500,
        { error: `the archive could not be written (${R2_UNREACHABLE}) — the table was not deleted` },
      ]);
      expect(archiveLines(lines)).toEqual([
        { event: "archive-failed", id, site: "abandon", record_key: `test/rev7/abandoned/${id}.json`, detail: R2_UNREACHABLE },
      ]);
    } finally {
      await failing.restore();
    }
    expect(await storedRows(stub), "nothing written").toEqual(before);
    expect(await bucketText(`test/rev7/abandoned/${id}.json`)).toBeNull();
    expect((await call(stub, RED, "GET", `/api/games/${id}`)).status, "the table still answers").toBe(200);
  });

  // Battery 17. The abandon takes P-2's reading ONCE: the re-dispatch pins
  // the same one, so its plan is the plan the receipt is for, and the delete
  // proceeds. Mutant: the re-dispatch pinned to a reading of its own — its
  // `archived_at` differs, its plan is another, and it is deferred again (a
  // 500, and a second reading taken). The pair lands under the `test/` root,
  // RED being a test identity (spec §2.6).
  //
  // This row's catchers are the TABLE's sites, this cell and the owed save's
  // stamp below. The ARCHIVE object pins its second dispatch to its one
  // reading too (`archive.ts`), and no cell can see that pin: a browser's
  // save puts the table it was sent, which the layer does not stamp, so its
  // plan carries no reading and a second reading changes nothing — measured
  // by the whole-branch review's battery (its mutant r17b answered the same
  // 201 with two readings handed out), and recorded as equivalent.
  it("an abandon whose put lands deletes, its pair keyed under test/ at the one reading it took (battery 17)", async () => {
    const id = "ab-land";
    const stub = await seedPlaying(id);
    const record = await docOf(stub, "record");
    const ticks = ticking(1_795_000_000);
    let answered: { result: Response; lines: unknown[] };
    try {
      answered = await logsDuring(() => call(stub, RED, "DELETE", `/api/games/${id}`));
    } finally {
      ticks.restore();
    }
    expect(answered.result.status).toBe(204);
    expect(ticks.readings, "one reading, pinned for the first dispatch and again for the re-dispatch").toEqual([1_795_000_000]);
    const recordKey = `test/rev7/abandoned/${id}.json`;
    expect(archiveLines(answered.lines)).toEqual([
      { event: "archive-landed", id, site: "abandon", record_key: recordKey, detail: recordKey },
    ]);
    expect(await bucketText(recordKey), "the stored record's bytes").toBe(record);
    expect(archivedAt(await bucketText(`test/rev7/abandoned/${id}.table.json`)), "the table stamped at that reading").toBe(1_795_000_000);
    expect(await docOf(stub, "table"), "deleted").toBeUndefined();
  });

  // Battery 20, through the objects. The first abandon's RECORD put fails,
  // leaving a half pair — the table landed, the record not — and the abandon
  // refused. The second abandon, at a new reading, lands: its table
  // OVERWRITES the stale one. Mutant: the table put create-if-absent (the R2
  // order reversed) — the stale table stands.
  it("a half pair, then a second abandon, lands and overwrites the stale table (battery 20)", async () => {
    const id = "ab-half";
    const stub = await seedPlaying(id);
    const record = await docOf(stub, "record");
    const [recordKey, tableKey] = [`test/rev7/abandoned/${id}.json`, `test/rev7/abandoned/${id}.table.json`];
    const ticks = ticking(1_796_000_000);
    try {
      const failing = await swapBucket(stub, failingPuts(1, (key) => key === recordKey));
      try {
        const first = await logsDuring(() => call(stub, RED, "DELETE", `/api/games/${id}`));
        expect(first.result.status, "premise: the record's put failed, so the abandon is refused").toBe(500);
      } finally {
        await failing.restore();
      }
      expect(await bucketText(recordKey), "premise: a half pair — no record").toBeNull();
      expect(archivedAt(await bucketText(tableKey)), "premise: … and the first abandon's table").toBe(1_796_000_000);
      const second = await logsDuring(() => call(stub, RED, "DELETE", `/api/games/${id}`));
      expect(second.result.status).toBe(204);
      expect(archiveLines(second.lines)).toEqual([{ event: "archive-landed", id, site: "abandon", record_key: recordKey, detail: recordKey }]);
    } finally {
      ticks.restore();
    }
    expect(archivedAt(await bucketText(tableKey)), "the stale table overwritten by the second abandon's").toBe(1_796_000_001);
    expect(await bucketText(recordKey)).toBe(record);
  });

  // Spec §2.6 step 3, against the pool's own R2 (miniflare): the record's
  // create-if-absent put meets a record another writer put after the
  // upload's first read — the put wrapper writes it through the real bucket
  // just before the conditional put — and the BUCKET refuses the create, so
  // the record is read again. Other bytes are a `conflict`, which refuses the
  // abandon in the layer's own sentence and leaves the racer's record; these
  // very bytes are `identical`, and the abandon proceeds. The node cells
  // prove the order over a bucket in memory; this proves R2 honours the form
  // (`onlyIf` with `If-None-Match: *`). Mutant: the record put unconditional
  // — the racer's record overwritten, and the upload `landed`.
  it.each([
    ["other bytes", "ab-race-other", "conflict"],
    ["the very bytes", "ab-race-same", "identical"],
  ] as const)("a create-if-absent put that meets a record put before it is refused by the bucket: %s", async (_what, id, outcome) => {
    const stub = await seedPlaying(id);
    const record = await docOf(stub, "record");
    const table = await docOf(stub, "table");
    const recordKey = `test/rev7/abandoned/${id}.json`;
    const raced = outcome === "conflict" ? ANOTHER_RECORD : undefined;
    const racing = await swapBucket(stub, async (real, key, value, options) => {
      if (key === recordKey && options?.onlyIf !== undefined) await real.put(key, raced ?? value);
      return real.put(key, value, options);
    });
    let answered: { result: Response; lines: unknown[] };
    try {
      answered = await logsDuring(() => call(stub, RED, "DELETE", `/api/games/${id}`));
    } finally {
      await racing.restore();
    }
    expect(archiveLines(answered.lines)).toEqual([{ event: `archive-${outcome}`, id, site: "abandon", record_key: recordKey, detail: recordKey }]);
    if (outcome === "conflict") {
      expect([answered.result.status, await answered.result.json()]).toEqual([
        500,
        { error: `the archive could not be written (${recordKey} holds a different record — a game is archived once) — the table was not deleted` },
      ]);
      expect(await bucketText(recordKey), "the racer's record stands").toBe(ANOTHER_RECORD);
      expect([await docOf(stub, "table"), await docOf(stub, "record")], "nothing deleted, and nothing written").toEqual([table, record]);
    } else {
      expect(answered.result.status).toBe(204);
      expect(await bucketText(recordKey)).toBe(record);
    }
  });

  // Spec §2.4 step 1: an abandon runs WHOLE under `blockConcurrencyWhile`, so
  // nothing lands between its two dispatches — a move that did would change
  // the record, and the second dispatch's plan would not be the receipt's. A
  // move sent while the abandon's upload is held waits for it, and then meets
  // the table deleted. The cell gives the move a quarter of `WAIT_MS` to land
  // before it releases the upload, then reads BOTH answers: a move that is
  // slower than that can hide the mutant, never fail correct code. Mutant: the
  // abandon outside `blockConcurrencyWhile` — the move lands (200) and the
  // abandon is deferred again (500).
  it("an abandon blocks the table: a move sent during its upload waits, and meets the table deleted", async () => {
    const id = "ab-block";
    const stub = await seedPlaying(id);
    const puts = heldPuts();
    const swapped = await swapBucket(stub, puts.through);
    try {
      const deleting = call(stub, RED, "DELETE", `/api/games/${id}`);
      await until(() => puts.reached(), "the abandon's upload reaches the bucket");
      const moving = call(stub, RED, "POST", `/api/games/${id}/input`, '{"line":"1"}');
      await new Promise((resolve) => setTimeout(resolve, WAIT_MS / 4));
      puts.open();
      const [deleted, moved] = await Promise.all([deleting, moving]);
      expect([deleted.status, moved.status], "the abandon, then the move").toEqual([204, 404]);
    } finally {
      puts.open();
      await swapped.restore();
      await settle(stub);
    }
  });

  // The orchestrator's ruling I-57 (spec §2.4 step 2): a delete that saved
  // supersedes an owed game-over save, and its ONE batch clears it. The game
  // over's own upload failed; the creator's abandon of the finished game
  // saves it under `over/`. The removal's push is held, so the batch is read
  // before the house's acknowledgement clears the slot. With the archive on,
  // nothing is dropped, so no `archive-owed-dropped` line. Mutants: the
  // delete's batch leaves `archive_owed` — a save owed by a table that is
  // gone; the dropped-save line logged with the switch on.
  it("the delete's batch clears an owed save (the orchestrator's ruling I-57)", async () => {
    const id = "go-abandoned";
    const stub = await owedGameOver(id);
    const failing = await swapBucket(stub, failingPuts(1));
    try {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    } finally {
      await failing.restore();
    }
    expect((await owedOf(stub))?.attempt, "premise: the game-over save is owed").toBe(1);
    const removal = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, removal]]);
    try {
      const { result: res, lines } = await logsDuring(() => call(stub, LOCAL, "DELETE", `/api/games/${id}`));
      expect(res.status).toBe(204);
      const recordKey = `rev7/over/${id}.json`;
      expect(archiveLines(lines), "the finished game saved under over/, and nothing dropped").toEqual([
        { event: "archive-landed", id, site: "abandon", record_key: recordKey, detail: recordKey },
      ]);
      expect(lines.filter((l) => (l as { event?: unknown }).event === "archive-owed-dropped"), "the switch on: nothing dropped").toEqual([]);
      expect(await owedOf(stub), "the delete's batch cleared the owed save").toBeUndefined();
      expect(await docOf(stub, "table"), "premise: in the same batch that removed the table").toBeUndefined();
      held.release(id, removal);
    } finally {
      held.restore();
      await settle(stub);
    }
  });
});

describe("game over (spec §2.6)", () => {
  // Steps 1–3 of the brief, in order, on the committed pair under its own id.
  // Every house push and the upload are held, so each batch is read alone.
  // Mutants: the wake's plan not owed (only a move's); owed in a batch of its
  // own, or with the call's reading rather than the wake's; no alarm in the
  // batch; the upload awaited inside the request (the reply waits on the
  // gate); the receipt not installed, or the stamp pinned to a reading of its
  // own — the stamp's plan is then another, and it defers again; the stamped
  // row, the cleared `archive_owed` and the changed copy's slot written in
  // more than ONE batch; the stamped table not written back.
  it("owes the wake's plan in THAT request's batch, uploads after the reply, and stamps at the recorded reading in one batch", async () => {
    const stub = await owedGameOver();
    const [recordKey, tableKey] = [`rev7/over/${OWED}.json`, `rev7/over/${OWED}.table.json`];
    const ticks = ticking(1_792_000_000);
    const pushes = holdApply();
    const puts = heldPuts();
    const swapped = await swapBucket(stub, puts.through);
    try {
      const batches = await watchBatches(stub);
      const before = Date.now();
      const { result: res, lines } = await logsDuring(() =>
        within(call(stub, LOCAL, "GET", `/api/games/${OWED}/table`), "the GET answers while its game over's upload is held"),
      );
      expect(res.status).toBe(200);
      // 1. The WAKE's plan (its `restore_picker` pumped the bot's run to the
      //    end), owed in the request's one batch, at the wake's reading — the
      //    first one handed out; the dispatch took the second.
      expect({ batches: batches.batches, outside: batches.outside }).toEqual({ batches: 1, outside: [] });
      const owed = await owedOf(stub);
      const record = await docOf(stub, "record");
      expect(ticks.readings, "premise: the wake's reading, then the dispatch's").toEqual([1_792_000_000, 1_792_000_001]);
      expect(owed).toEqual({
        plan: { key: expect.stringMatching(/^[0-9a-f]{64}$/), record_key: recordKey, table_key: tableKey, record, table: expect.any(String) },
        reading: 1_792_000_000,
        attempt: 0,
      });
      expect(archivedAt(owed!.plan.table), "the plan's table, stamped at its reading").toBe(1_792_000_000);
      expect(await alarmOf(stub), "its retry armed in that batch").toBeGreaterThanOrEqual(before + 30_000);
      // 2. The upload comes AFTER the reply: the reply is in, and the upload
      //    is at the gate with nothing landed.
      expect(archiveLines(lines), "no outcome yet").toEqual([]);
      await until(() => puts.reached(), "the upload reaches the bucket");
      expect(await bucketText(recordKey)).toBeNull();
      // 3. The receipt, then the layer's own stamp at the RECORDED reading —
      //    the stamped row, the cleared `archive_owed` and the changed copy's
      //    slot in ONE batch.
      const stampedIn = batches.batches;
      const revision = await revisionOf(stub);
      const done = await logsDuring(async () => {
        puts.open();
        await until(async () => (await owedOf(stub)) === undefined, "the save completes");
      });
      expect(batches.batches - stampedIn, "one batch").toBe(1);
      expect(archiveLines(done.lines)).toEqual([
        overLine("landed", OWED, recordKey, 0),
        stampLine(OWED, recordKey, 1_792_000_000, true, true),
      ]);
      expect(await docOf(stub, "table"), "the stored table IS the table the plan archived").toBe(owed!.plan.table);
      const [slot] = await sqlOf(stub, "SELECT v FROM meta WHERE k = 'slot'");
      expect(JSON.parse(String(slot?.v)) as { revision: number; copy: string }).toMatchObject({
        revision: revision + 1,
        copy: expect.stringContaining('"archived_at":1792000000'),
      });
      expect(await bucketText(recordKey)).toBe(owed!.plan.record);
      expect(await bucketText(tableKey)).toBe(owed!.plan.table);
    } finally {
      puts.open();
      pushes.restore();
      ticks.restore();
      await swapped.restore();
      await settle(stub);
    }
  });

  // Step 4. The upload after the reply fails, and then the ALARM's upload
  // fails too: the alarm that ran is spent, so only the failure's own re-arm
  // — at the backoff's second rung, 2 min — leaves one. Then the alarm lands
  // it. On the one-person table, whose copy the house has acknowledged before
  // the game ends: the owed-game-over pair's never reached the house through
  // this shell, so the first alarm's recopy would push it, and that push's
  // acknowledgement re-arms the alarm for the owed save too — hiding a failure
  // that re-arms nothing (measured). It runs with no TTL (`noTtl`): under one,
  // every alarm run ends by re-arming to everything still due, this retry
  // among them, which would hide the same failure. Mutants: a failure that
  // re-arms nothing, or counts nothing (the retry at 30 s again); an alarm
  // that does not deliver the owed save.
  it("a failing put is retried by the alarm, on the backoff, and lands", async () => {
    const ttl = noTtl();
    try {
      const id = "retry-seat";
      const stub = await oneSeatGame(id);
      const recordKey = `test/rev7/over/${id}.json`;
      const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
      expect((await input("undo")).status, "premise: the one person's undo").toBe(200);
      await settle(stub);
      const failing = await swapBucket(stub, failingPuts(2));
      try {
        const failedFrom = Date.now();
        const first = await logsDuring(async () => {
          expect((await input("1")).status, "premise: the last move ends the game again").toBe(200);
          await settle(stub);
        });
        const failedTo = Date.now();
        expect(archiveLines(first.lines), "premise: the upload after the reply failed").toEqual([
          overLine("failed", id, recordKey, 0, R2_UNREACHABLE),
        ]);
        expect((await owedOf(stub))?.attempt, "the failure is counted").toBe(1);
        // The first rung, 30 s, from a moment of the failing request's.
        const retry = await alarmOf(stub);
        expect(retry, "and its retry armed at the first rung").toBeGreaterThanOrEqual(failedFrom + 30_000);
        expect(retry).toBeLessThanOrEqual(failedTo + 30_000);
        const before = Date.now();
        const second = await logsDuring(async () => {
          expect(await runDurableObjectAlarm(stub)).toBe(true);
          await settle(stub);
        });
        expect(archiveLines(second.lines), "the alarm retried it").toEqual([overLine("failed", id, recordKey, 1, R2_UNREACHABLE)]);
        expect(await alarmOf(stub), "the second failure re-armed at the second rung").toBeGreaterThanOrEqual(before + 120_000);
      } finally {
        await failing.restore();
      }
      const owed = await owedOf(stub);
      const third = await logsDuring(async () => {
        expect(await runDurableObjectAlarm(stub)).toBe(true);
        await settle(stub);
      });
      expect(archiveLines(third.lines)).toEqual([overLine("landed", id, recordKey, 2), stampLine(id, recordKey, owed!.reading, true, true)]);
      expect(await owedOf(stub)).toBeUndefined();
      expect(archivedAt(await docOf(stub, "table"))).toBe(owed!.reading);
      expect(await bucketText(recordKey)).toBe(await docOf(stub, "record"));
    } finally {
      ttl.restore();
    }
  });

  // **An outage costs a request a read and nothing else** (R-D6, R-6.5c2.30's
  // "a short backoff after a failure"; Review Focus 3). R2 is down: game
  // over's first attempt, after the reply, fails and is counted, and the
  // failure sets the owed save's gate (`not_before`) at the rung its alarm is
  // armed for. TWENTY requests inside that rung each find the save owed and
  // put NOTHING: no line, no count, the owed row as it was. The alarm — run
  // at once, so still inside the rung — ignores the gate and puts once; it
  // fails, and the gate moves to the second rung. R2 back, the alarm lands the
  // save and stamps, and a request after that finds nothing owed and puts
  // nothing. On the one-person table, for the reason the cell above gives: the
  // owed-game-over pair's copy never reached the house, so the first alarm's
  // recopy would put a slot in motion, and its 30 s retry — due first — is
  // where `retryAt` would set the second gate (measured). Mutants: the gate
  // ignored on a request (a put and a failure batch per request); the gate
  // honoured on the alarm (no put at the rung); the gate set at game over's
  // first attempt (no put after the reply); a failure that sets no gate, or
  // one that moves it no further.
  it("an owed save is attempted once per rung, not once per request, while R2 is down (Review Focus 3)", async () => {
    const id = "outage-seat";
    const stub = await oneSeatGame(id);
    const recordKey = `test/rev7/over/${id}.json`;
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    const read = () => call(stub, WIRE_RED, "GET", `/api/games/${id}/table`);
    expect((await input("undo")).status, "premise: the one person's undo").toBe(200);
    await settle(stub);
    let down = true;
    let puts = 0;
    const swapped = await swapBucket(stub, (real, key, value, options) => {
      puts += 1;
      return down ? Promise.reject(new Error(R2_UNREACHABLE)) : real.put(key, value, options);
    });
    try {
      const failedFrom = Date.now();
      const first = await logsDuring(async () => {
        expect((await input("1")).status, "premise: the last move ends the game again").toBe(200);
        await settle(stub);
      });
      const failedTo = Date.now();
      expect(archiveLines(first.lines), "premise: game over's first attempt, after the reply, failed").toEqual([
        overLine("failed", id, recordKey, 0, R2_UNREACHABLE),
      ]);
      expect(puts, "premise: that attempt put once").toBe(1);
      const owed = await owedOf(stub);
      expect(owed?.attempt, "the failure is counted").toBe(1);

      const burst = await logsDuring(async () => {
        for (let n = 0; n < 20; n++) {
          expect((await read()).status).toBe(200);
          await settle(stub);
        }
      });
      expect(puts, "twenty requests inside the rung put nothing").toBe(1);
      expect(archiveLines(burst.lines), "and log no outcome").toEqual([]);
      expect(await owedOf(stub), "the owed row as it was").toEqual(owed);
      expect(owed?.not_before, "the failure set the gate at the first rung").toBeGreaterThanOrEqual(failedFrom + 30_000);
      expect(owed?.not_before).toBeLessThanOrEqual(failedTo + 30_000);
      expect(Date.now(), "premise: the twenty ran inside the rung").toBeLessThan(owed!.not_before!);

      const before = Date.now();
      const rung = await logsDuring(async () => {
        expect(await runDurableObjectAlarm(stub)).toBe(true);
        await settle(stub);
      });
      const after = Date.now();
      expect(after, "premise: the alarm ran inside the rung too").toBeLessThan(owed!.not_before!);
      expect(archiveLines(rung.lines), "the alarm ignores the gate").toEqual([overLine("failed", id, recordKey, 1, R2_UNREACHABLE)]);
      expect(puts, "and puts once").toBe(2);
      const again = await owedOf(stub);
      expect(again?.attempt, "the second failure is counted").toBe(2);
      expect(again?.not_before, "and moves the gate to the second rung").toBeGreaterThanOrEqual(before + 120_000);
      expect(again?.not_before).toBeLessThanOrEqual(after + 120_000);

      down = false;
      const landed = await logsDuring(async () => {
        expect(await runDurableObjectAlarm(stub)).toBe(true);
        await settle(stub);
      });
      expect(archiveLines(landed.lines), "R2 back: the alarm lands the save and stamps").toEqual([
        overLine("landed", id, recordKey, 2),
        stampLine(id, recordKey, owed!.reading, true, true),
      ]);
      expect(puts, "premise: the table's put and the record's").toBe(4);
      const later = await logsDuring(async () => {
        expect((await read()).status).toBe(200);
        await settle(stub);
      });
      expect(await owedOf(stub), "a request after the landed put finds nothing owed").toBeUndefined();
      expect([puts, archiveLines(later.lines)], "and puts nothing").toEqual([4, []]);
    } finally {
      await swapped.restore();
      await settle(stub);
    }
  });

  // Spec §2.4's one alarm, "the earliest of" what is due. At the one-person
  // table, the replayed last move ends the game: its copy changes (`turn`)
  // and its save is owed, in the same batch, and both leave after the reply.
  // The push is held until the upload has failed and armed its retry; then
  // the push's acknowledgement clears the slot, with no copy owed — and must
  // re-arm the alarm to the owed save's retry, not delete it. (The
  // owed-game-over pair cannot show it: its stored table already reads
  // `over`, so its game over changes no copy.) Mutant: the acknowledgement
  // re-arming for an owed copy alone — the alarm gone, and the save waiting
  // for a request that may never come.
  it("an acknowledgement that clears the slot keeps the alarm while a save is owed", async () => {
    const id = "ack-seat";
    const stub = await oneSeatGame(id);
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    expect((await input("undo")).status, "premise: the one person's undo").toBe(200);
    await settle(stub);
    const r = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, r]]);
    const failing = await swapBucket(stub, failingPuts(1));
    let before = 0;
    try {
      expect((await input("1")).status, "premise: the last move ends the game again").toBe(200);
      await until(() => held.reached(id, r), "the game over's copy reaches its gate");
      await until(async () => (await owedOf(stub))?.attempt === 1, "the upload fails, counted");
      before = Date.now();
      held.release(id, r);
    } finally {
      held.restore();
      await failing.restore();
      await settle(stub);
    }
    const after = Date.now();
    expect(await sqlOf(stub, "SELECT v FROM meta WHERE k IN ('slot', 'copy_owed')"), "premise: the slot cleared, no copy owed").toEqual([]);
    // A window, not a floor: the table's expiry, 90 days out, satisfies any
    // floor, and it is where the alarm would stand had the save been forgotten.
    const alarm = await alarmOf(stub);
    expect(alarm, "the alarm kept for the save's retry").toBeGreaterThanOrEqual(before + 30_000);
    expect(alarm).toBeLessThanOrEqual(after + 30_000);
  });

  // The other side of the same rule: a save DISCHARGED while a slot still
  // waits leaves the alarm for the slot. At the one-person table the game ends
  // (a slot and an owed save), the upload is held, and the undo lands during
  // it; both pushes fail, so the undo's slot waits for its retry. Released,
  // the upload lands and the stamp answers the game no longer over: the save
  // is cleared and no copy changes. Mutant: a discharge that deletes the alarm
  // whatever else waits — the slot's retry lost.
  it("a save discharged while a failed slot waits keeps the alarm for the slot", async () => {
    const id = "wait-seat";
    const stub = await oneSeatGame(id);
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    expect((await input("undo")).status, "premise: the one person's undo").toBe(200);
    await settle(stub);
    const puts = heldPuts();
    const swapped = await swapBucket(stub, puts.through);
    const pushes = await failPushes(stub, 2);
    let before = 0;
    try {
      expect((await input("1")).status, "premise: the last move ends the game again").toBe(200);
      await until(() => puts.reached(), "the game-over upload reaches the bucket");
      expect((await input("undo")).status, "premise: the undo, during the upload").toBe(200);
      const undone = await revisionOf(stub);
      await until(async () => {
        const [slot] = await sqlOf(stub, "SELECT v FROM meta WHERE k = 'slot'");
        const waiting = slot === undefined ? undefined : (JSON.parse(String(slot.v)) as { revision: number; attempt: number });
        return waiting?.revision === undone && waiting.attempt === 1;
      }, "the undo's push fails, and its slot waits");
      before = Date.now();
      puts.open();
      await until(async () => (await owedOf(stub)) === undefined, "the save is discharged");
    } finally {
      puts.open();
      // Each swap replaced the `env` it found, so they are undone in reverse.
      await pushes.restore();
      await swapped.restore();
      await settle(stub);
    }
    const after = Date.now();
    expect(archivedAt(await docOf(stub, "table")), "premise: no longer over, nothing stamped").toBeNull();
    // A window, not a floor, for the reason the cell above gives.
    const alarm = await alarmOf(stub);
    expect(alarm, "the alarm kept for the waiting slot's retry").toBeGreaterThanOrEqual(before + 30_000);
    expect(alarm).toBeLessThanOrEqual(after + 30_000);
  });

  // Step 5. The upload after the reply fails; the object is EVICTED, which
  // discards the swapped bucket and the module; the alarm's retry builds a new
  // module, whose wake reads a NEW reading — and the stamp is pinned to the
  // RECORDED one, so its plan is the stored plan. Mutant: the stamp at the
  // reading of the moment — another plan, deferred again, and the save still
  // owed.
  it("a retry after an eviction stamps at the RECORDED reading", async () => {
    const id = "go-evict";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const ticks = ticking(1_797_000_000);
    try {
      // Not restored, by design: the eviction below discards the swapped
      // copy with the instance. Should an assertion fail before it, the
      // one-shot `failingPuts(1)` stays on that instance — at most one more
      // failed put, on an id no other cell uses.
      await swapBucket(stub, failingPuts(1));
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
      const owed = await owedOf(stub);
      expect([owed?.reading, owed?.attempt], "premise: the wake's plan, at its reading, failed once").toEqual([1_797_000_000, 1]);
      await evictDurableObject(stub);
      const handedOut = ticks.readings.length;
      const { lines } = await logsDuring(async () => {
        expect(await runDurableObjectAlarm(stub)).toBe(true);
        await settle(stub);
      });
      expect(ticks.readings.length, "premise: the retry read NEW readings — its recopy's and its new module's wake").toBeGreaterThan(handedOut);
      expect(archiveLines(lines)).toEqual([overLine("landed", id, recordKey, 1), stampLine(id, recordKey, 1_797_000_000, true, true)]);
    } finally {
      ticks.restore();
    }
    expect(await owedOf(stub)).toBeUndefined();
    expect(archivedAt(await docOf(stub, "table")), "stamped at the RECORDED reading").toBe(1_797_000_000);
  });

  // Step 6, and "a landed-but-unstamped game-over copy retried at a NEW
  // reading answers identical, and stamps" (spec §5.2), through a REAL undo:
  // at a one-person table the undo is admitted at game over.
  //
  // - The game ends; its upload is held, and the one person's undo lands
  //   DURING it — a game-over upload runs after the reply, outside
  //   `blockConcurrencyWhile`. The upload lands; the stamp answers
  //   `{"stamped": false, "over": false}`, and nothing is owed any more.
  //   Mutant: that answer read as a failed stamp — the save retried for ever.
  // - The same last move ends the game again, at a NEW reading: the record key
  //   already holds these very bytes, so the upload is `identical` — written
  //   nothing, the first completion's table standing — and the stamp lands at
  //   the new reading. Mutants: identical not stamped; the table rewritten on
  //   identical.
  it("a game no longer over is owed nothing; its landed copy, retried at a NEW reading, answers identical and stamps", async () => {
    const id = "one-seat";
    const stub = await oneSeatGame(id);
    const [recordKey, tableKey] = [`test/rev7/over/${id}.json`, `test/rev7/over/${id}.table.json`];
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    const play = async (res: Response) => {
      expect(res.status, "premise: the input applied").toBe(200);
      return res.json<Play>();
    };
    const ticks = ticking(1_798_000_000);
    let first: ArchiveOwed | undefined;
    let again: number | undefined;
    try {
      const over = await play(await call(stub, WIRE_RED, "GET", `/api/games/${id}`));
      expect([over.events, over.prompt.kind], "premise: the walk's finished game").toEqual([1036, "game_over"]);
      expect((await play(await input("undo"))).events, "premise: the one person's undo is admitted at game over").toBe(1035);
      expect(await owedOf(stub), "premise: an undo defers nothing").toBeUndefined();

      const puts = heldPuts();
      const swapped = await swapBucket(stub, puts.through);
      try {
        expect((await play(await input("1"))).prompt.kind, "premise: the last move ends the game again").toBe("game_over");
        first = await owedOf(stub);
        expect(first?.plan.record_key).toBe(recordKey);
        await until(() => puts.reached(), "the game-over upload reaches the bucket");
        expect((await play(await within(input("undo"), "the undo answers while the upload is held"))).events).toBe(1035);
        const released = await logsDuring(async () => {
          puts.open();
          await settle(stub);
        });
        expect(archiveLines(released.lines)).toEqual([
          overLine("landed", id, recordKey, 0),
          stampLine(id, recordKey, first!.reading, false, false),
        ]);
      } finally {
        puts.open();
        await swapped.restore();
        await settle(stub);
      }
      expect(await owedOf(stub), "no longer over: nothing is owed").toBeUndefined();
      expect(archivedAt(await docOf(stub, "table")), "and nothing stamped").toBeNull();
      expect(await bucketText(recordKey), "the first completion's copy landed").toBe(first!.plan.record);

      // An `identical` upload puts nothing, so no put gate can hold it: the
      // plan's reading is the move's, the one reading that request took.
      const handedOut = ticks.readings.length;
      const replayed = await logsDuring(async () => {
        expect((await play(await input("1"))).prompt.kind).toBe("game_over");
        await settle(stub);
      });
      again = ticks.readings[handedOut];
      expect(again, "premise: the game over again, at a NEW reading").toBeGreaterThan(first!.reading);
      expect(await docOf(stub, "record"), "premise: the same last move made the same record").toBe(first!.plan.record);
      expect(archiveLines(replayed.lines)).toEqual([
        overLine("identical", id, recordKey, 0),
        stampLine(id, recordKey, again!, true, true),
      ]);
    } finally {
      ticks.restore();
    }
    expect(await owedOf(stub)).toBeUndefined();
    expect(archivedAt(await docOf(stub, "table")), "stamped at the new reading").toBe(again);
    expect(archivedAt(await bucketText(tableKey)), "identical writes nothing: the first completion's table stands").toBe(first!.reading);
  });

  // "A Conflict is final at every site" (spec §2.6) — kept by R-6.5c2.11
  // (M6.5c2, spec §2.5; Review Focus 3) for every conflict the layer does not
  // stamp. What the key holds goes to the stamp as every conflict now does;
  // where it is no earlier completion of this game the layer logs its
  // refusal and stamps nothing, so the conflict is FINAL: `archive_owed`
  // cleared, nothing retried, the bucket untouched, the alarm re-armed to the
  // table's expiry. Two such held texts, each of which a retry would meet
  // again (nothing this shell runs deletes an archived key): another game's
  // record (the walk's, another seed), which only a caller-chosen id can
  // reach, and a text that is no record. Mutants: a
  // conflict retried (still owed, an alarm
  // at a rung); `held` read as the same game (a stamp); a conflict's stamp
  // answer read through `stampArm` (a retry).
  it.each([
    ["another game's record", "go-conflict", (ARCHIVE_WALK[0]!.body as { record: string }).record],
    ["a text that is no record", "go-conflict-text", ANOTHER_RECORD],
  ] as const)("a conflict at game over with %s is final: logged, nothing owed, nothing stamped", async (_what, id, held) => {
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    await env.ARCHIVE!.put(recordKey, held);
    const { lines } = await logsDuring(async () => {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    });
    // The plan's reading is the pool's frozen clock (`wrangler.test.jsonc`).
    expect(archiveLines(lines)).toEqual([
      overLine("conflict", id, recordKey, 0),
      { event: "archive-over-refused", id, error: `${recordKey} holds a different record — a game is archived once` },
      stampLine(id, recordKey, 1_789_000_000, false, true),
    ]);
    expect(await owedOf(stub), "final: nothing owed").toBeUndefined();
    expect(archivedAt(await docOf(stub, "table")), "nothing stamped").toBeNull();
    expect(await bucketText(recordKey), "the archive keeps the record it held").toBe(held);
    expect(await bucketText(`rev7/over/${id}.table.json`), "a conflict writes nothing").toBeNull();
    expect(await alarmOf(stub), "no retry for a save discharged: the alarm is the table's expiry").toBe(await expiryOf(stub));
  });

  // **Abandoning a FINISHED table** (M6.5c2 cycle E, R-6.5c2.42; the plan's
  // dossier item 21), the plain case: the game over's save landed ONCE, at
  // game over, and the layer stamped the table; its creator's Abandon then
  // deletes it and saves nothing again. The refinish cells below reach the
  // same Abandon through a race; this is the case with none. A PIN — the rule
  // is the layer's (`delete_locked` skips its save for a stamped table, held
  // by `abandoning_an_open_dealing_moveless_or_archived_table_archives_nothing`)
  // and this cell holds the Worker's host to it. Mutants: the Abandon of a
  // stamped table saving again (a second `archive-landed`, or anything under
  // `abandoned/`); the game over's save disturbed; the marker naming anything
  // but the archive.
  it("abandoning a finished table archived at game over: 204, no second save, nothing under abandoned/, the marker names the archive", async () => {
    const id = "finished-abandoned";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const over = await logsDuring(async () => {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    });
    expect(archiveLines(over.lines), "archived once, at game over, and stamped").toEqual([
      overLine("landed", id, recordKey, 0),
      stampLine(id, recordKey, 1_789_000_000, true, true),
    ]);
    expect(archivedAt(await docOf(stub, "table")), "premise: stamped at game over").toBe(1_789_000_000);
    const record = await docOf(stub, "record");
    const abandoned = await logsDuring(async () => {
      const res = await call(stub, LOCAL, "DELETE", `/api/games/${id}`);
      await settle(stub);
      return res;
    });
    expect(abandoned.result.status).toBe(204);
    expect(archiveLines(abandoned.lines), "no second save").toEqual([]);
    expect(await bucketText(`rev7/abandoned/${id}.json`), "nothing under abandoned/").toBeNull();
    expect(await bucketText(`rev7/abandoned/${id}.table.json`)).toBeNull();
    expect(await bucketText(recordKey), "the game over's save stands, its record's bytes").toBe(record);
    expect(await markerOf(stub)).toEqual({ at: expect.any(Number), reason: "abandoned", kept: "archive" });
    const after = await call(stub, LOCAL, "GET", `/api/games/${id}/table`);
    expect(after.status).toBe(404);
    expect((await after.json<{ error: string }>()).error).toMatch(/^this table was abandoned on \d{4}-\d{2}-\d{2} — its game is in the archive$/);
  });

  // **The refinish race, overlapping** (M6.5c2, R-6.5c2.11; spec §2.5; review
  // M3, o2; Review Focus 3). At the one-person table the game ends (completion
  // A) and its upload is held; the one person's two undos land DURING it, and
  // they play a DIFFERENT end (completion B), whose upload starts too —
  // `uploads` keys each by its plan, and two endings are two plans, so the map
  // does not make B wait on A (o2, asserted). A is let through first and
  // lands; its plan is no longer the one owed, so it settles nothing. B then
  // meets A's record and answers `conflict`, carrying it, and the layer,
  // finding an earlier completion of THIS game, stamps. The table ends stamped
  // at B's reading, the archive keeps the FIRST ending, and the creator's
  // Abandon deletes it, saving nothing again — a stamped game owes no second
  // save. Overlapping, B's table put ran after A's, so the bucket's TABLE may
  // be B's beside A's record: best-effort (spec §2.5), and not asserted; the
  // next cell pins the pair. Before the fix, measured here: B's conflict final
  // and no stamp; the plan's D6 also read the Abandon then refused for good.
  // Mutants: the conflict arm left final; `held` dropped anywhere between R2
  // and the layer.
  it("(the refinish race) an undo during a held upload, then a different end: stamped, the first ending kept, abandonable", async () => {
    const id = "refinish-overlap";
    const stub = await oneSeatGame(id);
    const recordKey = `test/rev7/over/${id}.json`;
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    const play = async (res: Response) => {
      expect(res.status, "premise: the input applied").toBe(200);
      return res.json<Play>();
    };
    const ticks = ticking(1_799_000_000);
    let first: ArchiveOwed | undefined;
    let second: ArchiveOwed | undefined;
    let released: unknown[] = [];
    try {
      await play(await input("undo"));
      await play(await input("undo"));
      expect((await play(await call(stub, WIRE_RED, "GET", `/api/games/${id}`))).events, "premise: two undos from the walk's end").toBe(1034);
      await settle(stub);
      const puts = heldInTurn();
      const swapped = await swapBucket(stub, puts.through);
      try {
        expect((await playAll(input, play, ORIGINAL_END)).prompt.kind, "premise: completion A").toBe("game_over");
        first = await owedOf(stub);
        await until(() => puts.arrived() === 1, "A's upload reaches the bucket");
        await play(await within(input("undo"), "the first undo answers while A's upload is held"));
        await play(await within(input("undo"), "the second undo answers while A's upload is held"));
        expect((await playAll(input, play, DIFFERENT_END)).prompt.kind, "premise: completion B").toBe("game_over");
        second = await owedOf(stub);
        expect(second!.plan.record, "premise: B is another ending").not.toBe(first!.plan.record);
        await until(() => puts.arrived() === 2, "B's upload reaches the bucket");
        expect(await runInDurableObject(stub, (t: Table) => t["uploads"].size), "(o2) both in flight: the map does not serialize them").toBe(2);
        released = (
          await logsDuring(async () => {
            puts.openFirst();
            await until(async () => (await bucketText(recordKey)) === first!.plan.record, "A lands");
            puts.openSecond();
            await settle(stub);
          })
        ).lines;
      } finally {
        puts.openAll();
        await swapped.restore();
        await settle(stub);
      }
    } finally {
      ticks.restore();
    }
    expect(archiveLines(released)).toEqual([
      overLine("landed", id, recordKey, 0),
      overLine("conflict", id, recordKey, 0),
      { event: "archive-kept-earlier-completion", id, key: recordKey },
      stampLine(id, recordKey, second!.reading, true, true),
    ]);
    expect(await owedOf(stub), "nothing owed").toBeUndefined();
    expect(archivedAt(await docOf(stub, "table")), "stamped, at B's reading").toBe(second!.reading);
    expect(await bucketText(recordKey), "the archive keeps the FIRST ending").toBe(first!.plan.record);
    const abandoned = await logsDuring(async () => {
      const res = await call(stub, WIRE_RED, "DELETE", `/api/games/${id}`);
      await settle(stub);
      return res;
    });
    expect(abandoned.result.status, "abandonable: a stamped game owes no second save").toBe(204);
    expect(archiveLines(abandoned.lines), "and its Abandon saves nothing again").toEqual([]);
    expect(await bucketText(`test/rev7/abandoned/${id}.json`), "nothing under abandoned/").toBeNull();
    expect(await markerOf(stub)).toMatchObject({ reason: "abandoned", kept: "archive" });
  });

  // **The refinish race, one upload after the other** (M6.5c2, R-6.5c2.11;
  // spec §2.5; review o1; Review Focus 3). A's upload is held, the undos land
  // during it, and A lands — its game no longer over, so its stamp answers
  // `{"stamped": false, "over": false}` and nothing is owed. Then the different
  // end B: its upload reads the record key FIRST (`executePlan`) and meets A's
  // record, so the conflict WRITES NOTHING and the pair in the bucket stays
  // one game's: A's record beside A's table, told from B's by its
  // `archived_at` (the clock ticks). The layer stamps, and the table expires
  // as any stamped table does: its alarm is its expiry deadline, and at its
  // TTL it ends `expired`, keeping its archive and saving nothing again.
  // Before the fix, measured here: the table unstamped after B; the plan's D6
  // also read its expiry then `held`. Mutants: the conflict arm left final;
  // `held` dropped anywhere between R2 and the layer; the table put before
  // the record is read (B's table in the bucket).
  it("(the refinish race) the pair stays one game's, and the stamped table expires normally", async () => {
    const id = "refinish-seq";
    const stub = await oneSeatGame(id);
    const [recordKey, tableKey] = [`test/rev7/over/${id}.json`, `test/rev7/over/${id}.table.json`];
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    const play = async (res: Response) => {
      expect(res.status, "premise: the input applied").toBe(200);
      return res.json<Play>();
    };
    const ticks = ticking(1_799_100_000);
    let first: ArchiveOwed | undefined;
    let second: unknown[] = [];
    try {
      await play(await input("undo"));
      await play(await input("undo"));
      await settle(stub);
      const puts = heldPuts();
      const swapped = await swapBucket(stub, puts.through);
      try {
        expect((await playAll(input, play, ORIGINAL_END)).prompt.kind, "premise: completion A").toBe("game_over");
        first = await owedOf(stub);
        expect(first!.plan.record, "premise: A is the walk's own record").toBe((ARCHIVE_WALK[0]!.body as { record: string }).record);
        await until(() => puts.reached(), "A's upload reaches the bucket");
        await play(await within(input("undo"), "the first undo answers while A's upload is held"));
        await play(await within(input("undo"), "the second undo answers while A's upload is held"));
        const released = await logsDuring(async () => {
          puts.open();
          await settle(stub);
        });
        expect(archiveLines(released.lines), "premise: A landed with its game no longer over").toEqual([
          overLine("landed", id, recordKey, 0),
          stampLine(id, recordKey, first!.reading, false, false),
        ]);
      } finally {
        puts.open();
        await swapped.restore();
        await settle(stub);
      }
      second = (
        await logsDuring(async () => {
          expect((await playAll(input, play, DIFFERENT_END)).prompt.kind, "premise: completion B").toBe("game_over");
          await settle(stub);
        })
      ).lines;
    } finally {
      ticks.restore();
    }
    const stamped = archivedAt(await docOf(stub, "table"));
    expect(stamped, "stamped, at B's own reading").not.toBeNull();
    expect(stamped, "premise: B's reading is not A's, so the two plans' tables differ").not.toBe(first!.reading);
    expect(await docOf(stub, "record"), "premise: B is another ending").not.toBe(first!.plan.record);
    expect(archiveLines(second)).toEqual([
      overLine("conflict", id, recordKey, 0),
      { event: "archive-kept-earlier-completion", id, key: recordKey },
      stampLine(id, recordKey, stamped!, true, true),
    ]);
    expect(await owedOf(stub), "nothing owed").toBeUndefined();
    expect(await bucketText(recordKey), "the archive keeps the FIRST ending").toBe(first!.plan.record);
    expect(archivedAt(await bucketText(tableKey)), "a conflict writes nothing: the pair is A's").toBe(first!.reading);
    expect(await alarmOf(stub), "the stamped table's alarm is its expiry deadline").toBe(await expiryOf(stub));
    await backdate(stub, Date.now() - 90 * DAY_MS - 60_000);
    const expired = await logsDuring(async () => {
      expect(await runDurableObjectAlarm(stub)).toBe(true);
      await settle(stub);
    });
    const ends = expired.lines.filter((l) => !["slot-pushed", "slot-acked", "wake"].includes(String((l as { event?: unknown }).event)));
    expect(ends, "expiring normally: no save owed at its end, never held").toEqual([{ event: "expired", id, reason: "expired", kept: "archive" }]);
    expect(await markerOf(stub)).toMatchObject({ reason: "expired", kept: "archive" });
    expect(await bucketText(`test/rev7/abandoned/${id}.json`), "nothing saved again under abandoned/").toBeNull();
  });

  // **A refused stamp is retried, never thrown** (M6.5c2, spec §2.4's R2-o1;
  // cycle A's `settleUpload` item). An owed game-over save whose game was then
  // undone, and which then meets a deploy whose bot is not the game's: the
  // stored record is the owed pair's own cut, where the bot is to move, with
  // its bot's version moved 2 → 3 in the TEXT (`replaceOnce`; the strings
  // rule). The alarm's upload lands; the wake's rebuild and the stamp's are
  // both refused by the layer's foreign-bot rule, a 409. The shell logs
  // `stamp-refused {id, status}`, throws nothing, and keeps the save owed —
  // counted, its alarm re-armed to the next rung and its gate set there, so a
  // request before it re-stamps nothing (R-D6): final-for-now, bounded by
  // the creator's Abandon or the table's expiry, which never rebuild (spec
  // §2.4; the layer's own cell holds the Abandon). Mutants: the 409 body
  // handed to `parseStamp` (a throw inside the alarm); the refusal read as a
  // settled stamp (the save cleared); the retry left ungated.
  it("a stamp the layer refuses is logged and retried on the backoff, and nothing throws inside the alarm", async () => {
    const id = "go-foreign";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const failing = await swapBucket(stub, failingPuts(1));
    try {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    } finally {
      await failing.restore();
    }
    const owed = await owedOf(stub);
    expect(owed?.attempt, "premise: the game-over save is owed, its first upload failed").toBe(1);
    const foreign = replaceOnce(OWED_GAME_OVER_JSON, '"version":2', '"version":3');
    await runInDurableObject(stub, (_t, state) => {
      state.storage.transactionSync(() => upsertDoc(state.storage.sql, "record", foreign, Date.now()));
    });
    await evictDurableObject(stub);
    const before = Date.now();
    const ran = await logsDuring(async () => {
      expect(await runDurableObjectAlarm(stub), "the alarm ran, and threw nothing").toBe(true);
      await settle(stub);
    });
    const after = Date.now();
    expect(ran.lines, "premise: the layer's foreign-bot rule refuses the wake's rebuild").toContainEqual(
      expect.objectContaining({ event: "restore-refused", id, status: 409, body: expect.stringContaining('"foreign_bot"') }),
    );
    expect(archiveLines(ran.lines), "the upload landed, and no stamp settled").toEqual([overLine("landed", id, recordKey, 1)]);
    expect(ran.lines, "the refusal logged").toContainEqual({ event: "stamp-refused", id, status: 409 });
    const still = await owedOf(stub);
    expect(still, "still owed, one more failure counted").toEqual({ ...owed, attempt: 2, not_before: expect.any(Number) });
    // Gated at the retry its batch armed (R-D6, `retryAt`): the rung after two
    // failures, 2 min, or sooner when something else is due first — here the
    // slot the alarm's recopy put in motion, whose retry is 30 s, unless its
    // acknowledgement came before the stamp's batch.
    expect(still!.not_before, "gated at its retry").toBeGreaterThanOrEqual(before + 30_000);
    expect(still!.not_before).toBeLessThanOrEqual(after + 120_000);
    expect(archivedAt(await docOf(stub, "table")), "nothing stamped").toBeNull();
    const alarm = await alarmOf(stub);
    expect(alarm, "re-armed at the rung after two failures").toBeGreaterThanOrEqual(before + 120_000);
    expect(alarm).toBeLessThanOrEqual(after + 120_000);
  });

  // Spec §5.2: "a chat post arriving during a slow … R2 put lands". It must be
  // the GAME-OVER upload, which runs after the reply outside any
  // `blockConcurrencyWhile` — an abandon's runs under it, and nothing lands
  // during that one (plan review m6). Mutant: the game-over upload run under
  // `blockConcurrencyWhile`, or inside the request — the post waits on the
  // gate.
  it("a chat post arriving during the game-over upload lands", async () => {
    const id = "go-chat";
    const stub = await owedGameOver(id);
    const puts = heldPuts();
    const swapped = await swapBucket(stub, puts.through);
    try {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await until(() => puts.reached(), "the game-over upload reaches the bucket");
      const chat = await within(call(stub, LOCAL, "POST", `/api/games/${id}/chat`, '{"text":"good game"}'), "the chat post answers while the upload is held");
      expect(chat.status).toBe(201);
      expect(await bucketText(`rev7/over/${id}.json`), "premise: the upload is still held").toBeNull();
    } finally {
      puts.open();
      await swapped.restore();
      await settle(stub);
    }
    expect(await docOf(stub, "chat")).toContain("good game");
    expect(await owedOf(stub)).toBeUndefined();
    expect(archivedAt(await docOf(stub, "table")), "stamped at the reading the save was owed at: the pool's frozen one").toBe(1_789_000_000);
  });
});

describe("the browser's save (spec §2.6)", () => {
  // Through the caller's own archive object, under `blockConcurrencyWhile`:
  // the layer's 201, its 200 `identical`, and its 409, each after the upload
  // it depends on, the pair keyed under the `test/` root (a `.test` caller).
  // A second caller's put fails: the layer's own refusal, R2's message inside
  // it. Mutants: no loop (the deferral answered as a 500); an outcome
  // installed as another; the failed receipt installed as landed.
  it("answers the layer's 201, 200 identical and 409, and its refusal when the put fails", async () => {
    const caller = "saver@t.test";
    const path = `browser/${await segmentOf(caller)}/rev7/over/wire.json`;
    const recordKey = `test/${path}`;
    const { record } = ARCHIVE_WALK[0]!.body as { record: string };
    const outcomes = await logsDuring(async () => [
      await save(caller, ARCHIVE_WALK[0]!.body),
      await save(caller, ARCHIVE_WALK[1]!.body),
      await save(caller, ARCHIVE_WALK[2]!.body),
    ]);
    expect(outcomes.result.map((r) => [r.status, JSON.parse(r.text)])).toEqual([
      [201, { leaf: "over", path, identical: false }],
      [200, { leaf: "over", path, identical: true }],
      [409, { error: "a different record is archived under this id" }],
    ]);
    const line = (outcome: string) => ({ event: `archive-${outcome}`, id: `archive:${caller}`, site: "browser", record_key: recordKey, detail: recordKey });
    expect(archiveLines(outcomes.lines)).toEqual([line("landed"), line("identical"), line("conflict")]);
    expect(await bucketText(recordKey), "the record as sent").toBe(record);

    const failer = "failer@t.test";
    const failing = await swapBucket(archiveStub(failer), failingPuts(Infinity));
    try {
      const refused = await logsDuring(() => save(failer, ARCHIVE_WALK[0]!.body));
      expect([refused.result.status, JSON.parse(refused.result.text)]).toEqual([
        500,
        { error: `the archive could not be written: ${R2_UNREACHABLE}` },
      ]);
    } finally {
      await failing.restore();
    }
    expect(await bucketText(`test/browser/${await segmentOf(failer)}/rev7/over/wire.json`)).toBeNull();
  });
});

describe("side channels during a slow push or upload (spec §5.2)", () => {
  // The other half of spec §5.2's line: the HOUSE push. Red ends the turn —
  // the copy changes (`turn.to_move`), so a slot is pushed after the reply —
  // and blue's chat post answers while that push is held. Mutant: the push
  // awaited inside the request, or under `blockConcurrencyWhile` — the move,
  // or the post, waits on the gate.
  it("a chat post arriving during a slow house push lands", async () => {
    const id = "push-chat";
    const stub = await seedPlaying(id);
    const input = (line: string) => call(stub, RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    // Red's lines at this seed, measured: the walk's own for `wire` (rows
    // 49–51 and 53–56), none of which changes the table document. The next
    // `1` (row 57) ends the turn.
    for (const line of ["2", "A09", "confirm", "1", "1", "1", "1"]) expect((await input(line)).status, line).toBe(200);
    await settle(stub);
    const r = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, r]]);
    try {
      expect((await within(input("1"), "red's last line answers while its push is held")).status).toBe(200);
      await until(() => held.reached(id, r), "the changed copy's push reaches its gate");
      const chat = await within(call(stub, BLUE, "POST", `/api/games/${id}/chat`, '{"text":"your move"}'), "the chat post answers while the push is held");
      expect(chat.status).toBe(201);
      expect(held.settled(id, r), "premise: the push is still held").toBe(false);
      held.release(id, r);
    } finally {
      held.restore();
      await settle(stub);
    }
    const [acked] = await sqlOf(stub, "SELECT v FROM meta WHERE k = 'acked'");
    expect((JSON.parse(String(acked?.v)) as { revision: number }).revision, "the held push then landed").toBe(r);
  });
});

describe("the switch (R-6.5c.14; spec §2.6)", () => {
  // `off`: the layer's existing off-branch everywhere. Health reads `off`;
  // game over saves nothing, owes nothing and arms no alarm (this game over
  // changes no copy, so nothing else arms one); an abandon deletes in its
  // first dispatch, putting nothing; the archive route is the layer's 404.
  // Mutants: the module handed `r2` whatever the switch reads; an alarm armed
  // for a save the switch keeps from being owed; an abandon that still
  // uploads.
  it("off: health reads off, game over and an abandon put nothing, and the route is the layer's 404", async () => {
    const off = archiveOff();
    try {
      await rebuilt(houseStub());
      const health = await houseStub().fetch("https://yokohama.test/api/health", { headers: { "X-Yokohama-User": RED } });
      expect((await health.json<{ archive: string }>()).archive).toBe("off");

      const id = "off-over";
      const stub = await owedGameOver(id);
      const over = await logsDuring(async () => {
        expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
        await settle(stub);
      });
      expect((await docOf(stub, "record"))?.length, "premise: the wake pumped the game to its end").toBeGreaterThan(OWED_GAME_OVER_JSON.length);
      expect(archiveLines(over.lines)).toEqual([]);
      expect(await owedOf(stub)).toBeUndefined();
      expect(await alarmOf(stub), "no alarm armed").toBeNull();
      expect(await bucketText(`rev7/over/${id}.json`)).toBeNull();

      const abandoned = await seedPlaying("off-abandon");
      const deleted = await logsDuring(() => call(abandoned, RED, "DELETE", "/api/games/off-abandon"));
      expect([deleted.result.status, archiveLines(deleted.lines)]).toEqual([204, []]);
      expect(await bucketText("test/rev7/abandoned/off-abandon.json")).toBeNull();

      const route = await save("off@t.test", ARCHIVE_WALK[0]!.body);
      expect([route.status, JSON.parse(route.text)]).toEqual([404, { error: "this server keeps no archive" }]);
    } finally {
      off.restore();
    }
    await rebuilt(houseStub());
  });

  // Battery 27, and its resumption. A save owed while the switch was `r2`
  // is KEPT while it reads `off`: the alarm logs it and neither uploads nor
  // re-arms for it — it re-arms to the table's expiry, the one deadline still
  // kept while off (spec §2.6: "at the latest its expiry") — and a request
  // retries nothing. Switched back on, the next request resumes it. The gate
  // the first failure set (R-D6; inside it a request uploads nothing — the
  // outage cell above) is moved into the past BEFORE the off period, a
  // premise, since the cell runs in well under 30 s: so the switch, and not
  // the gate, is what stops the off period's request, and nothing stands
  // between the switch coming back on and the resumption. Mutants: the owed
  // save retried while off; the alarm re-armed for it while off; a request
  // that never resumes it, or a gate that never opens to a request.
  it("an owed save is kept, not retried, while off (battery 27), and resumes at the next request once on", async () => {
    const id = "off-owed";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const failing = await swapBucket(stub, failingPuts(1));
    const failedFrom = Date.now();
    try {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    } finally {
      await failing.restore();
    }
    const failedTo = Date.now();
    await rungPassed(stub);
    const owed = await owedOf(stub);
    expect(owed?.attempt, "premise: a save is owed").toBe(1);
    const retry = await alarmOf(stub);
    expect(retry, "premise: its retry is armed at the first rung").toBeGreaterThanOrEqual(failedFrom + 30_000);
    expect(retry).toBeLessThanOrEqual(failedTo + 30_000);

    const off = archiveOff();
    try {
      await evictDurableObject(stub);
      const ran = await logsDuring(async () => {
        expect(await runDurableObjectAlarm(stub)).toBe(true);
        await settle(stub);
      });
      expect(archiveLines(ran.lines)).toEqual([{ event: "archive-owed-kept", id, record_key: recordKey, attempt: 1 }]);
      expect(await bucketText(recordKey), "not uploaded while off").toBeNull();
      expect(await owedOf(stub), "kept as it was").toEqual(owed);
      expect(await alarmOf(stub), "no retry re-armed for it: the alarm is the table's expiry").toBe(await expiryOf(stub));
      const read = await logsDuring(async () => {
        expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
        await settle(stub);
      });
      expect(archiveLines(read.lines), "a request while off retries nothing").toEqual([]);
      expect(await bucketText(recordKey)).toBeNull();
    } finally {
      off.restore();
    }

    await evictDurableObject(stub);
    const resumed = await logsDuring(async () => {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    });
    expect(archiveLines(resumed.lines)).toEqual([overLine("landed", id, recordKey, 1), stampLine(id, recordKey, owed!.reading, true, true)]);
    expect(await owedOf(stub)).toBeUndefined();
    expect(archivedAt(await docOf(stub, "table"))).toBe(owed!.reading);
  });

  // Spec §2.6: with the switch `off`, a delete makes no save of its own, so a
  // game-over save still owed from when it was `r2` goes with the table — and
  // the shell logs it, once the delete's batch has committed. The owed save is
  // arranged as battery 27's is; then the switch reads off, and the creator
  // abandons. The removal's push is held, so the batch is read before the
  // house's acknowledgement clears the slot. Mutants: the line missing; the
  // save still owed by a table that is gone.
  it("an abandon while the switch is off drops an owed save, and logs it", async () => {
    const id = "off-dropped";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    const failing = await swapBucket(stub, failingPuts(1));
    try {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    } finally {
      await failing.restore();
    }
    expect((await owedOf(stub))?.attempt, "premise: a save is owed").toBe(1);
    const off = archiveOff();
    const removal = (await revisionOf(stub)) + 1;
    const held = holdApply([[id, removal]]);
    try {
      await evictDurableObject(stub);
      const { result: res, lines } = await logsDuring(() => call(stub, LOCAL, "DELETE", `/api/games/${id}`));
      expect(res.status, "the delete, in its first dispatch").toBe(204);
      expect(archiveLines(lines)).toEqual([{ event: "archive-owed-dropped", id, record_key: recordKey, attempt: 1 }]);
      expect(await owedOf(stub), "no save owed by a table that is gone").toBeUndefined();
      expect(await docOf(stub, "table"), "premise: in the batch that removed the table").toBeUndefined();
      held.release(id, removal);
    } finally {
      held.restore();
      off.restore();
      await settle(stub);
    }
    expect(await bucketText(recordKey), "nothing uploaded").toBeNull();
  });
});
