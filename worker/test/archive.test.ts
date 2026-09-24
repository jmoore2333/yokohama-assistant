import { env } from "cloudflare:workers";
import { evictDurableObject, runDurableObjectAlarm, runInDurableObject } from "cloudflare:test";
import { describe, expect, it, vi } from "vitest";
import type { Archive } from "../src/archive";
import * as clockModule from "../src/clock";
import * as configModule from "../src/config";
import type { Env } from "../src/env";
import type { House } from "../src/house";
import type { ArchiveOwed } from "../src/r2";
import { HOUSE_NAME } from "../src/slot";
import { ensureTables, upsertDoc } from "../src/storage";
import type { Table } from "../src/table";
import { OWED_GAME_OVER_JSON, OWED_GAME_OVER_TABLE_JSON } from "./fixtures.gen";
import { failingPuts, failPushes, heldPuts, holdApply, R2_UNREACHABLE, swapBucket } from "./gates";
import { BLUE, RED, call, logsDuring, revisionOf, seedPlaying, settle, sqlOf, storedRows, tableStub, watchBatches, type Play } from "./helpers";
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

/** The committed pair's creator, and its one human seat. */
const LOCAL = "local@browser";
/** The committed pair's own id. */
const OWED = "local-242a5429453c593c";
/** The walk's first seat — at the one-person table, both seats. */
const WIRE_RED = "red@wire.test";

/** A record no game here makes: what another game's copy, archived under
 *  this id, holds. */
const ANOTHER_RECORD = '{"engine_version":"another","events":[]}';

/** `text` with `from` replaced by `to`, where `from` occurs exactly once — an
 *  edit of a document's TEXT, never a parse, so a seed above 2^53 keeps its
 *  digits. */
function replaceOnce(text: string, from: string, to: string): string {
  const at = text.indexOf(from);
  if (at < 0 || text.includes(from, at + 1)) throw new Error(`premise: ${JSON.stringify(from)} occurs exactly once`);
  return text.slice(0, at) + to + text.slice(at + from.length);
}

/** A table document's `archived_at`, read off its text: `null` while it is
 *  unstamped. */
function archivedAt(table: string | null | undefined): number | null {
  const m = /"archived_at": (null|\d+)/.exec(table ?? "");
  if (m === null) throw new Error(`premise: a table document naming archived_at — got ${table?.slice(0, 80)}`);
  return m[1] === "null" ? null : Number(m[1]);
}

/** A table object named `name` holding `docs` as their exact text, written in
 *  one batch before its first request — whose WAKE loads them. */
async function holding(name: string, docs: { table: string; record: string }): Promise<DurableObjectStub<Table>> {
  const stub = tableStub(name);
  await runInDurableObject(stub, (_t, state) => {
    state.storage.transactionSync(() => {
      ensureTables(state.storage.sql);
      upsertDoc(state.storage.sql, "table", docs.table, Date.now());
      upsertDoc(state.storage.sql, "record", docs.record, Date.now());
    });
  });
  return stub;
}

/**
 * The committed owed-game-over pair (`crates/yokohama-worker`'s fixture): the
 * finished browser-played game cut back to the start of the bot's trailing
 * run, so the first request's WAKE pumps the bot to the end and reaches the
 * game-over site (plan review M4). Its creator is `local@browser`, not a
 * `.test` address, so its save is keyed at the bucket's top — this file's
 * local bucket.
 */
function owedGameOver(name: string = OWED): Promise<DurableObjectStub<Table>> {
  const table = name === OWED ? OWED_GAME_OVER_TABLE_JSON : replaceOnce(OWED_GAME_OVER_TABLE_JSON, `"id": "${OWED}"`, `"id": "${name}"`);
  return holding(name, { table, record: OWED_GAME_OVER_JSON });
}

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

/** A `docs` row's exact text — `undefined` when none is stored. */
async function docOf(stub: DurableObjectStub<Table>, kind: "table" | "record" | "chat"): Promise<string | undefined> {
  const rows = await runInDurableObject(stub, (_t, state) => {
    const sql = state.storage.sql;
    const exists = sql.exec("SELECT count(*) AS n FROM sqlite_master WHERE type = 'table' AND name = 'docs'").one().n === 1;
    return exists ? sql.exec<{ json: string }>("SELECT json FROM docs WHERE kind = ?", kind).toArray() : [];
  });
  return rows[0]?.json;
}

/** The object at `key` in this file's bucket, as its text: `null` when absent. */
async function bucketText(key: string): Promise<string | null> {
  const object = await env.ARCHIVE!.get(key);
  return object === null ? null : object.text();
}

/** The pending alarm, or `null`. */
function alarmOf(stub: DurableObjectStub<Table>): Promise<number | null> {
  return runInDurableObject(stub, (_t, state) => state.storage.getAlarm());
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

/**
 * The switch set to `off` at MODULE scope (pool fact 4), in both of the
 * shell's readers — `shellConfig`, and the module's `init` configuration,
 * which reads its own copy — as if `YOKOHAMA_WEB_ARCHIVE` read `off`. An object
 * reads its configuration when it is built, so a cell rebuilds the objects it
 * asks (`rebuilt`).
 */
function archiveOff(): { restore(): void } {
  const { shellConfig, moduleConfig } = configModule;
  const off = (e: Env): Env => ({ ...e, YOKOHAMA_WEB_ARCHIVE: "off" });
  const shell = vi.spyOn(configModule, "shellConfig").mockImplementation((e) => shellConfig(off(e)));
  const init = vi.spyOn(configModule, "moduleConfig").mockImplementation((e, caller) => moduleConfig(off(e), caller));
  return {
    restore() {
      shell.mockRestore();
      init.mockRestore();
    },
  };
}

/** `stub`'s object built afresh at its next event, so it reads the
 *  configuration in force: running, then evicted. */
async function rebuilt(stub: DurableObjectStub<Table> | DurableObjectStub<House>): Promise<void> {
  await runInDurableObject(stub as unknown as DurableObjectStub, () => {});
  await evictDurableObject(stub);
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
      expect(await docOf(stub, "table"), "nothing deleted").toBeDefined();
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
  // before the house's acknowledgement ends the object. With the archive on,
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
  // that re-arms nothing (measured). Mutants: a failure that re-arms nothing,
  // or counts nothing (the retry at 30 s again); an alarm that does not
  // deliver the owed save.
  it("a failing put is retried by the alarm, on the backoff, and lands", async () => {
    const id = "retry-seat";
    const stub = await oneSeatGame(id);
    const recordKey = `test/rev7/over/${id}.json`;
    const input = (line: string) => call(stub, WIRE_RED, "POST", `/api/games/${id}/input`, JSON.stringify({ line }));
    expect((await input("undo")).status, "premise: the one person's undo").toBe(200);
    await settle(stub);
    const failing = await swapBucket(stub, failingPuts(2));
    try {
      const first = await logsDuring(async () => {
        expect((await input("1")).status, "premise: the last move ends the game again").toBe(200);
        await settle(stub);
      });
      expect(archiveLines(first.lines), "premise: the upload after the reply failed").toEqual([
        overLine("failed", id, recordKey, 0, R2_UNREACHABLE),
      ]);
      expect((await owedOf(stub))?.attempt, "the failure is counted").toBe(1);
      expect(await alarmOf(stub), "and its retry armed").not.toBeNull();
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
    expect(await sqlOf(stub, "SELECT v FROM meta WHERE k IN ('slot', 'copy_owed')"), "premise: the slot cleared, no copy owed").toEqual([]);
    expect(await alarmOf(stub), "the alarm kept for the save's retry").toBeGreaterThanOrEqual(before + 30_000);
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
    expect(archivedAt(await docOf(stub, "table")), "premise: no longer over, nothing stamped").toBeNull();
    expect(await alarmOf(stub), "the alarm kept for the waiting slot's retry").toBeGreaterThanOrEqual(before + 30_000);
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
      // Not restored: the eviction below discards the swapped copy.
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

  // "A Conflict is final at every site" (spec §2.6): another game's record
  // already holds this id's key. Game over logs it and clears `archive_owed`,
  // writes nothing to the bucket, stamps nothing — and, the save discharged,
  // leaves no alarm for it. Mutants: a conflict retried (still owed, with an
  // alarm); a conflict stamped.
  it("a conflict at game over is final: logged, nothing owed, nothing stamped", async () => {
    const id = "go-conflict";
    const stub = await owedGameOver(id);
    const recordKey = `rev7/over/${id}.json`;
    await env.ARCHIVE!.put(recordKey, ANOTHER_RECORD);
    const { lines } = await logsDuring(async () => {
      expect((await call(stub, LOCAL, "GET", `/api/games/${id}/table`)).status).toBe(200);
      await settle(stub);
    });
    expect(archiveLines(lines)).toEqual([overLine("conflict", id, recordKey, 0)]);
    expect(await owedOf(stub), "final: nothing owed").toBeUndefined();
    expect(archivedAt(await docOf(stub, "table")), "nothing stamped").toBeNull();
    expect(await bucketText(recordKey), "the archive keeps the record it held").toBe(ANOTHER_RECORD);
    expect(await bucketText(`rev7/over/${id}.table.json`), "a conflict writes nothing").toBeNull();
    expect(await alarmOf(stub), "no alarm for a save discharged").toBeNull();
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
    expect(archivedAt(await docOf(stub, "table"))).not.toBeNull();
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
    const path = "browser/saver_at_t.test/rev7/over/wire.json";
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
    expect(await bucketText("test/browser/failer_at_t.test/rev7/over/wire.json")).toBeNull();
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
  // re-arms for it, and a request retries nothing. Switched back on, the next
  // request resumes it. Mutants: the owed save retried while off; the alarm
  // re-armed for it while off; a request that never resumes it.
  it("an owed save is kept, not retried, while off (battery 27), and resumes at the next request once on", async () => {
    const id = "off-owed";
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
    expect(owed?.attempt, "premise: a save is owed").toBe(1);
    expect(await alarmOf(stub), "premise: its retry is armed").not.toBeNull();

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
      expect(await alarmOf(stub), "and no alarm re-armed for it").toBeNull();
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
  // house's acknowledgement ends the object. Mutants: the line missing; the
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
