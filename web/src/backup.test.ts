// Backup and restore, the page's half (M6.5b2, R-6.5b2.10 – .12).
//
// **`fake-indexeddb` and not a double**, for `db.test.ts`'s reason: what the
// clash rule leans on is transaction SEMANTICS — that the check and both writes
// commit together, and that two overlapping `readwrite` transactions are
// serialised — and a `Map` would answer those the way its author expected.
//
// **Every string here comes from the committed envelope fixture**
// (`web/src/wire-script/backup-envelope.json`, emitted and pinned by
// `crates/yokohama-web/src/backup.rs`), so no payload is hand-authored and the
// page's reader and the Rust validator are held to one document. The ONE edit
// is a table document's `id` rewritten, which is `fakeWorker.ts`'s own rule.
//
// **What these cells are about is the PAGE.** The verdicts come from a fake
// (see `fakeWorker.ts`'s own header): the validator is pinned by that Rust
// file's cells, by `crates/yokohama-wasm`'s native cell, and by
// `backup.node.test.ts`, which runs the COMPILED module over this very
// fixture. What is asserted here is the clash rule, the transaction, the
// duplicate refusal, the compatibility row and what the controls source their
// strings from.
import "fake-indexeddb/auto";
import { IDBFactory } from "fake-indexeddb";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  backupEverything,
  backupOne,
  buildEnvelope,
  WORKER_BOUND_MS,
  forgetUnplayable,
  listUnplayable,
  removeForeign,
  restorePairs,
  unplayableIds,
  type Envelope,
} from "./backup";
import { deletePair, openDb, readPair, restoreOne, takeGeneration, writeDocs } from "./local/db";
import { rememberServerFacts, resetFactsForTest } from "./local/facts";
import { resetLocalHostForTests } from "./local/client";
import { breakTheWorker, currentWorker, stubWorker, workerAnswersWith } from "./test/fakeWorker";
import { wire } from "./wire/contract";
import committed from "./wire-script/backup-envelope.json";

const fixture = committed as unknown as Envelope;
const PLAYED_ID = "local-242a5429453c593c";

/** The accepted game's two documents, and the cross-rev game's — four real
 *  strings this build really produced. */
const RECORD_TEXT = fixture.games[0]!.record!;
const TABLE_TEXT = fixture.games[0]!.table;
const DEALING_TABLE_TEXT = fixture.games[1]!.table;
/** A DIFFERENT record: the fixture's cross-rev game. What it is is not what
 *  these cells are about — only that its bytes differ from `RECORD_TEXT`. */
const OTHER_RECORD = fixture.games[2]!.record!;
const OTHER_TABLE = fixture.games[2]!.table;
/** The snapshot a Worker hands back, standing for "a move applied and not yet
 *  written". Again: a different string is the whole of the premise. */
const RECORD_WITH_ONE_MORE_MOVE = OTHER_RECORD;

/** The accepted table document under another name. A textual edit of the ONE
 *  id it carries, never a `JSON.parse` round trip: the document holds a `u64`
 *  seed that would round. */
function tableFor(id: string): string {
  const out = TABLE_TEXT.replace(`"${PLAYED_ID}"`, `"${id}"`);
  if (out === TABLE_TEXT) throw new Error("the re-address changed nothing — the fixture moved");
  return out;
}

/** **A file ANOTHER DEVICE wrote**, with each game's own revision on it.
 *
 *  Built as an object rather than by editing `buildEnvelope`'s output, because
 *  that is what it stands for: this device has never seen these games, so
 *  nothing here could have told our own builder what revision they are at. The
 *  cells that assert what OUR builder produces call `backupOne` instead
 *  (review I-2). */
function fileFromAnotherDevice(
  games: { id: string; record: string | null; table: string; rev?: number; hash?: string }[],
): string {
  const env: Envelope = {
    kind: "yokohama-backup",
    envelope: 1,
    created_at: 1_789_000_000,
    origin: "another.device",
    app_version: "0.0.0-elsewhere",
    semantics_rev: 6,
    rules_hash: "deadbeefdeadbeef",
    games: games.map((g) => ({
      id: g.id,
      record: g.record,
      table: g.table,
      semantics_rev: g.rev ?? null,
      rules_hash: g.hash ?? null,
    })),
  };
  return JSON.stringify(env, null, 2) + "\n";
}

async function putPair(id: string, record: string | null, table: string): Promise<void> {
  const db = await openDb();
  await writeDocs(db, id, 0, { table, ...(record === null ? {} : { record }) });
}

const readBack = async (id: string) => readPair(await openDb(), id);

beforeEach(() => {
  // A FRESH database per cell: `fake-indexeddb/auto` installs one factory for
  // the whole run, and every cell here is about what is already stored.
  globalThis.indexedDB = new IDBFactory();
  resetLocalHostForTests();
  resetFactsForTest();
  stubWorker();
});

afterEach(() => {
  resetLocalHostForTests();
});

describe("backing up", () => {
  // The whole reason backup is NOT a wasm export (spec §2.5, §10). The FIRST
  // draft of this cell handed `buildEnvelope` strings it already had, so it
  // proved only that wrapping strings needs no wasm — while `localPair` posts to
  // the Worker, which awaits the module before handling ANY message. The real
  // claim is that the CONTROL works, sourcing its own strings, with the module
  // dead.
  it("backs up every stored game with the module unavailable", async () => {
    await putPair("local-1", RECORD_TEXT, tableFor("local-1"));
    await putPair("local-2", null, DEALING_TABLE_TEXT); // a game still at the deal
    breakTheWorker(); // the module will not load
    const text = await backupEverything(); // the control, not the helper
    const games = JSON.parse(text).games as Envelope["games"];
    expect(games.map((g) => g.id).sort()).toEqual(["local-1", "local-2"]);
    expect(games.find((g) => g.id === "local-1")!.record).toBe(RECORD_TEXT);
    expect(games.find((g) => g.id === "local-2")!.record).toBeNull();
  });

  // And the Worker is still PREFERRED when it answers, because a game with a
  // move applied but not yet written is exactly what reading the database alone
  // would omit — the move the recovery exists to rescue.
  //
  // **What the Worker answers is no longer "its memory, whatever it is"**
  // (PR #37 fix wave, R-F1): it is its memory only where this tab OWNS the game
  // at the current generation, and the committed pair everywhere else
  // (`local/exportPair.ts`, whose own cells hold that rule). So an answer here
  // stands for the owner's snapshot, and the page still takes it — the page
  // cannot tell the two apart and does not need to.
  it("prefers the Worker's snapshot when the Worker answers", async () => {
    await putPair("local-3", RECORD_TEXT, tableFor("local-3"));
    workerAnswersWith("local-3", {
      record: RECORD_WITH_ONE_MORE_MOVE,
      table: tableFor("local-3"),
    });
    const games = JSON.parse(await backupEverything()).games as Envelope["games"];
    expect(games[0]!.record).toBe(RECORD_WITH_ONE_MORE_MOVE);
  });

  // **One pair from ONE source** (R-F1; review F1's second half). The export
  // used to complete the Worker's answer field by field from the database —
  // `live?.record ?? stored.record` — so a Worker saying "this game is at the
  // deal, with no record" beside a store holding a played record came out as
  // that record paired with the DEALING table: two snapshots of one game
  // stitched into a pair neither of them ever was. The answer is taken whole.
  it("takes the Worker's pair WHOLE, never one document from each", async () => {
    await putPair("local-4", RECORD_TEXT, tableFor("local-4"));
    workerAnswersWith("local-4", { record: null, table: DEALING_TABLE_TEXT });

    const every = (JSON.parse(await backupEverything()) as Envelope).games;
    expect(every).toHaveLength(1);
    expect(every[0], "Back up everything").toMatchObject({
      id: "local-4",
      record: null,
      table: DEALING_TABLE_TEXT,
    });

    const one = (JSON.parse(await backupOne("local-4")) as Envelope).games;
    expect(one[0], "Back up, for one game").toMatchObject({
      id: "local-4",
      record: null,
      table: DEALING_TABLE_TEXT,
    });
  });

  // **M-3: the Worker's budget is ONE deadline for the whole export.** The
  // failure this is about is not a Worker that DIES — that rejects at once and
  // was already fast — but one that STARTS and whose module never initialises,
  // so every `ask` sits unanswered. Per game, "Back up everything" would then
  // cost the bound times the number of games, with nothing on screen saying so.
  //
  // Two assertions, and the COUNT is the discriminating one: once the first
  // game has spent the budget, the others are not asked at all.
  it("spends the Worker's budget once for the whole export", async () => {
    for (const id of ["local-a", "local-b", "local-c"]) {
      await putPair(id, RECORD_TEXT, tableFor(id));
    }
    const worker = stubWorker({ pair: () => "pending" }); // the module never came up
    // **The bound is counted, not timed** (ledger Task 8 L195). This read a
    // real wall clock against 3,000 ms and flaked once in nine full-suite
    // runs — a per-edit gate failing for a reason that is not about the code,
    // and the figure it compared against (two bounds) sat uncomfortably close
    // to the three the defect costs. `withinBound` arms exactly one timer per
    // ask, so COUNTING the long timers this export arms says the same thing
    // exactly, under any load: one for the whole export, not one per game.
    const timers = vi.spyOn(globalThis, "setTimeout");
    const text = await backupEverything();
    const bounds = timers.mock.calls.filter(
      ([, ms]) => typeof ms === "number" && ms > WORKER_BOUND_MS / 2,
    ).length;
    timers.mockRestore();
    const asked = worker.instances.flatMap((w) => w.posted).filter((r) => r.kind === "pair").length;
    expect(asked, "one game spends the budget; the rest are not asked").toBe(1);
    expect(bounds, "one bound is armed for the whole export, not one per game").toBe(1);
    // And the export is COMPLETE: the database's copies are what a silent
    // Worker falls back to, which is the whole point of the fallback.
    const games = (JSON.parse(text) as Envelope).games;
    expect(games.map((g) => g.id)).toEqual(["local-a", "local-b", "local-c"]);
    // **And each one is the stored pair WHOLE** (R-F1): with no answer from the
    // Worker there is one source left, and both documents come from it.
    for (const g of games) {
      expect({ record: g.record, table: g.table }, g.id).toEqual({
        record: RECORD_TEXT,
        table: tableFor(g.id),
      });
    }
  });

  it("backs up ONE game, and refuses a name this device does not have", async () => {
    await putPair("local-6", RECORD_TEXT, tableFor("local-6"));
    const one = JSON.parse(await backupOne("local-6")) as Envelope;
    expect(one.games.map((g) => g.id)).toEqual(["local-6"]);
    expect(one.games[0]!.table).toBe(tableFor("local-6"));
    await expect(backupOne("local-nothing")).rejects.toThrow(/no game named/);
  });

  // The envelope's rev and hash are EXPORTER PROVENANCE and come from what this
  // browser was last TOLD — never from a Worker, which is the thing that may be
  // dead. Before anything has told it, the file says so rather than guessing.
  it("carries the last health read as its provenance, and says nothing where there is none", async () => {
    await putPair("local-7", RECORD_TEXT, tableFor("local-7"));
    breakTheWorker();
    const blank = JSON.parse(await backupEverything()) as Envelope;
    expect(blank.semantics_rev).toBe(0);
    expect(blank.rules_hash).toBe("");

    rememberServerFacts(wire.health);
    const told = JSON.parse(await backupEverything()) as Envelope;
    expect(told.semantics_rev).toBe(wire.health.semantics_rev);
    expect(told.rules_hash).toBe(wire.health.rules_hash);
    expect(told.app_version).toBe(wire.health.server_version);
    expect(told.kind).toBe("yokohama-backup");
    expect(told.envelope).toBe(1);
  });
});

describe("restoring", () => {
  // R-6.5b2.11: a clash refuses and NAMES the row; identical bytes are a no-op.
  // The check and BOTH writes are one IndexedDB transaction — two tabs fall
  // between a read for absence and a later write.
  it("refuses a clash by name and calls identical bytes already here", async () => {
    const table = tableFor("local-1");
    await putPair("local-1", RECORD_TEXT, table);
    const same = await restorePairs(
      buildEnvelope([{ id: "local-1", record: RECORD_TEXT, table }]),
    );
    expect(same.games[0]).toMatchObject({ id: "local-1", outcome: "already-here" });

    const other = await restorePairs(
      buildEnvelope([{ id: "local-1", record: OTHER_RECORD, table }]),
    );
    expect(other.games[0]).toMatchObject({ id: "local-1", outcome: "clash" });
    expect(other.games[0]!.reason).toMatch(/already on this device/);
    expect(await readBack("local-1")).toEqual({ record: RECORD_TEXT, table }); // untouched
  });

  // Run it twice and nothing is harmed — the property that makes a restore
  // usable when you are not sure the first one worked.
  it("is safe to run twice", async () => {
    const env = buildEnvelope([
      { id: "local-9", record: RECORD_TEXT, table: tableFor("local-9") },
    ]);
    expect((await restorePairs(env)).games[0]!.outcome).toBe("restored");
    expect((await restorePairs(env)).games[0]!.outcome).toBe("already-here");
  });

  // A HALF-PRESENT pair is a clash, not an empty slot: writing the missing half
  // beside someone else's would produce a pair that agrees with nothing.
  it("treats a half-present pair as a clash", async () => {
    await putPair("local-4", null, tableFor("local-4")); // table only, no record
    const r = await restorePairs(
      buildEnvelope([{ id: "local-4", record: OTHER_RECORD, table: OTHER_TABLE }]),
    );
    expect(r.games[0]).toMatchObject({ id: "local-4", outcome: "clash" });
    expect(await readBack("local-4")).toEqual({ record: null, table: tableFor("local-4") });
  });

  // Two games under one id in one file, refused before anything is written —
  // and refused by the PAGE, so a file this bad is turned away even with the
  // module dead. Rust refuses it too (`backup.rs`'s own cell).
  it("refuses duplicate ids inside one envelope", async () => {
    const env = buildEnvelope([
      { id: "local-5", record: RECORD_TEXT, table: tableFor("local-5") },
      { id: "local-5", record: OTHER_RECORD, table: tableFor("local-5") },
    ]);
    await expect(restorePairs(env)).rejects.toThrow(/duplicate/);
    expect(await readBack("local-5")).toEqual({ record: null, table: null });
  });

  // **The page reports what the validator said and writes nothing for a
  // rejection**, which is this cell's claim and the whole of it. WHICH games
  // are rejected is Rust's rule, pinned by
  // `backup.rs::a_restored_game_must_be_addressable_as_a_local_one`; the
  // verdict here is supplied by the fake, named in the arrangement.
  it("writes nothing for a game the validator rejected, and carries its reason", async () => {
    const table = tableFor("local-8");
    const env = buildEnvelope([{ id: "local-8", record: RECORD_TEXT, table }]);
    stubWorker({
      restore: () => [
        { id: "local-8", verdict: { kind: "rejected", reason: "\"local-8\" is not a safe name" } },
      ],
    });
    const r = await restorePairs(env);
    expect(r.games[0]).toMatchObject({ id: "local-8", outcome: "rejected" });
    expect(r.games[0]!.reason).toMatch(/not a safe name/);
    expect(await readBack("local-8")).toEqual({ record: null, table: null });
  });

  // R-6.5b2.12: a cross-rev game LANDS, and lands with a compatibility row
  // beside it — the mechanism that puts it in a listing the module's own
  // cannot carry. Its bytes are stored exactly as the file had them.
  it("keeps an unplayable game, with a row saying why", async () => {
    const table = tableFor("local-old");
    const env = fileFromAnotherDevice([
      { id: "local-old", record: OTHER_RECORD, table, rev: 6, hash: "deadbeefdeadbeef" },
    ]);
    const r = await restorePairs(env);
    expect(r.games[0]).toMatchObject({ id: "local-old", outcome: "unplayable" });
    expect(r.games[0]!.reason).toMatch(/semantics rev 6/);
    expect(await readBack("local-old")).toEqual({ record: OTHER_RECORD, table });

    const rows = await listUnplayable();
    expect(rows.map((g) => g.id)).toEqual(["local-old"]);
    expect(rows[0]!.note).toMatchObject({ semantics_rev: 6, rules_hash: "deadbeefdeadbeef" });
    expect(rows[0]!.note.reason).toMatch(/semantics rev 6/);

    // And the pair is still exportable, which is what "downloadable" means.
    const out = JSON.parse(await backupOne("local-old")) as Envelope;
    expect(out.games[0]!.record).toBe(OTHER_RECORD);

    // Twice is still harmless for an unplayable game too — the compatibility
    // row makes the id PRESENT, so the second run reads the pair as identical
    // and writes nothing rather than stacking a second note on it.
    const again = await restorePairs(env);
    expect(again.games[0]!.outcome).toBe("already-here");
    expect((await listUnplayable()).map((g) => g.id)).toEqual(["local-old"]);
  });

  // **The per-game revision is what THIS DEVICE knows about THAT GAME** (spec
  // §2.5, review I-2), and this is the cell that could not exist before: the
  // builder PRODUCES the rev-6 pair, from the compatibility row, with nothing
  // in the cell setting it after the fact. The header stays the exporter's.
  it("re-exports a cross-rev game at ITS revision, not at this build's", async () => {
    rememberServerFacts(wire.health); // this browser is at rev 7
    await restorePairs(
      fileFromAnotherDevice([
        {
          id: "local-old",
          record: OTHER_RECORD,
          table: tableFor("local-old"),
          rev: 6,
          hash: "deadbeefdeadbeef",
        },
      ]),
    );
    const out = JSON.parse(await backupOne("local-old")) as Envelope;
    expect(out.semantics_rev).toBe(wire.health.semantics_rev); // the header: who wrote the file
    expect(out.games[0]!.semantics_rev).toBe(6); // the game: what it is
    expect(out.games[0]!.rules_hash).toBe("deadbeefdeadbeef");

    // And through "Back up everything", which is the other builder caller.
    const all = JSON.parse(await backupEverything()) as Envelope;
    expect(all.games[0]!.semantics_rev).toBe(6);
  });

  // A game this device CAN open knows nothing special about itself — it is at
  // this build's revision by construction — so it says nothing rather than
  // repeating the header about a game the header is not about.
  it("says nothing per game where there is no compatibility row", async () => {
    rememberServerFacts(wire.health);
    await putPair("local-fine", RECORD_TEXT, tableFor("local-fine"));
    const out = JSON.parse(await backupOne("local-fine")) as Envelope;
    expect(out.games[0]!.semantics_rev).toBeNull();
    expect(out.games[0]!.rules_hash).toBeNull();
    // The HEADER still says who wrote it, which is the field that has an answer.
    expect(out.semantics_rev).toBe(wire.health.semantics_rev);
  });

  // **I-1: a game's rows are ONE list, and both deletes remove all of them.**
  // The path is ordinary: a cross-rev pair whose TABLE parses lists as a
  // playable row too, opening it hits the broken-save screen, and its one
  // control is Abandon — the fenced delete. With the compatibility row left
  // behind, "Will not open here" kept a row with no documents under it whose
  // Back up threw.
  it("takes the compatibility row with the game when the game is deleted", async () => {
    await restorePairs(
      fileFromAnotherDevice([
        { id: "local-old", record: OTHER_RECORD, table: tableFor("local-old"), rev: 6 },
      ]),
    );
    expect((await listUnplayable()).map((g) => g.id)).toEqual(["local-old"]);

    const db = await openDb();
    expect(await deletePair(db, "local-old", 0)).toBe("deleted");
    expect(await listUnplayable()).toEqual([]);
    expect(await readBack("local-old")).toEqual({ record: null, table: null });
  });

  // And the fence still holds: a STALE delete removes nothing at all, the
  // compatibility row included.
  it("a stale delete leaves every row of the game alone", async () => {
    await restorePairs(
      fileFromAnotherDevice([
        { id: "local-old", record: OTHER_RECORD, table: tableFor("local-old"), rev: 6 },
      ]),
    );
    const db = await openDb();
    await takeGeneration(db, "local-old"); // another tab took it
    expect(await deletePair(db, "local-old", 0)).toBe("stale");
    expect((await listUnplayable()).map((g) => g.id)).toEqual(["local-old"]);
    expect((await readBack("local-old")).record).toBe(OTHER_RECORD);
  });

  // A compatibility row is this browser's own note and is NOT a document the
  // module can be handed — so it never goes into a backup either.
  it("does not export a compatibility row as a game", async () => {
    await restorePairs(
      buildEnvelope([{ id: "local-old", record: OTHER_RECORD, table: tableFor("local-old") }]),
    );
    const out = JSON.parse(await backupEverything()) as Envelope;
    expect(out.games.map((g) => g.id)).toEqual(["local-old"]);
    expect(out.games[0]!.record).toBe(OTHER_RECORD);
  });

  // **A transaction that fails reports THAT game and leaves the others alone.**
  // Arranged by restoring one game whose table is already present under other
  // bytes — a clash — beside one that is absent.
  it("reports each game on its own", async () => {
    await putPair("local-a", RECORD_TEXT, tableFor("local-a"));
    const r = await restorePairs(
      buildEnvelope([
        { id: "local-a", record: OTHER_RECORD, table: tableFor("local-a") },
        { id: "local-b", record: RECORD_TEXT, table: tableFor("local-b") },
      ]),
    );
    expect(r.games.map((g) => [g.id, g.outcome])).toEqual([
      ["local-a", "clash"],
      ["local-b", "restored"],
    ]);
    expect((await readBack("local-b")).record).toBe(RECORD_TEXT);
  });

  // The running tab reconciles: the games that LANDED are adopted, and only
  // those — a clash and a rejection changed nothing for the Worker to load.
  it("adopts only what landed", async () => {
    await putPair("local-a", RECORD_TEXT, tableFor("local-a"));
    const worker = stubWorker();
    await restorePairs(
      buildEnvelope([
        { id: "local-a", record: OTHER_RECORD, table: tableFor("local-a") },
        { id: "local-b", record: RECORD_TEXT, table: tableFor("local-b") },
      ]),
    );
    const adopts = worker.instances
      .flatMap((w) => w.posted)
      .filter((r): r is Extract<typeof r, { kind: "adopt" }> => r.kind === "adopt");
    expect(adopts).toHaveLength(1);
    expect(adopts[0]!.ids).toEqual(["local-b"]);
  });

  // **The check and both writes are ONE transaction**, and this is the cell
  // that can see it: two restores of the same ABSENT game, their transactions
  // created in the same tick. IndexedDB serialises overlapping `readwrite`
  // transactions over one store, so the loser reads what the winner wrote.
  //
  // Split into a readonly check and a later write, both would find the slot
  // empty and both would say `restored` — which is what a second tab
  // overwriting the first's game looks like from here. Measured: the split
  // fails this cell and passes every other one in the file.
  it("is one transaction, so two restores of one absent game do not both write", async () => {
    const db = await openDb();
    const docs = { record: RECORD_TEXT, table: tableFor("local-race") };
    const both = await Promise.all([
      restoreOne(db, "local-race", docs),
      restoreOne(db, "local-race", docs),
    ]);
    expect([...both].sort()).toEqual(["already-here", "restored"]);
    expect(await readBack("local-race")).toEqual({ record: RECORD_TEXT, table: docs.table });
  });

  // A file that is not a backup at all gets a sentence a player can read, and
  // never `JSON.parse`'s "Unexpected token" — which names nothing they chose
  // and nothing they can do.
  it("refuses something that is not a backup file, in words", async () => {
    await expect(restorePairs("this is a photograph")).rejects.toThrow(
      "this file is not a backup this app wrote",
    );
    await expect(restorePairs('{"kind":"yokohama-backup"}')).rejects.toThrow(
      "this file is not a backup this app wrote",
    );
  });

  // A report that does not cover the file is refused BEFORE anything is
  // written: a game the validator did not judge is not a game to write, and
  // dropping it silently is the failure the guard exists against.
  it("refuses a report that judged a different number of games", async () => {
    stubWorker({ restore: () => [] });
    const env = buildEnvelope([
      { id: "local-d", record: RECORD_TEXT, table: tableFor("local-d") },
    ]);
    await expect(restorePairs(env)).rejects.toThrow(/carries 1 games and the report judged 0/);
    expect(await readBack("local-d")).toEqual({ record: null, table: null });
  });

  // A file this build cannot read at all is a refusal about the FILE: nothing
  // is written and no per-game row is reported.
  it("refuses a file the validator will not read, and writes nothing", async () => {
    stubWorker({ restore: () => ({ refused: "this is envelope version 2; this build reads version 1" }) });
    const env = buildEnvelope([
      { id: "local-c", record: RECORD_TEXT, table: tableFor("local-c") },
    ]);
    await expect(restorePairs(env)).rejects.toThrow(/envelope version 2/);
    expect(await readBack("local-c")).toEqual({ record: null, table: null });
  });
});

// ---- the connections this module opens are given back (final review I-5) ---
//
// **`unplayableIds()` runs on EVERY local write** (`OnThisDevice`'s `read`),
// so a 400-move game left 400 connections open for the life of the page. The
// sibling module fixed exactly this shape in its own fix round (`outbox.ts`'s
// `withStore`); this cell is that claim for this file, and its premise — more
// than one open — is what keeps the equality from being two zeros.
describe("the database handles", () => {
  it("closes every connection it opens, across every public call", async () => {
    await putPair("local-handles", RECORD_TEXT, tableFor("local-handles"));
    const opens = vi.spyOn(indexedDB, "open");
    const closes = vi.spyOn(IDBDatabase.prototype, "close");
    try {
      await backupEverything();
      await backupOne("local-handles");
      await unplayableIds();
      await listUnplayable();
      await forgetUnplayable("local-handles");
      expect(opens.mock.calls.length, "the premise: these really do open connections").toBeGreaterThan(1);
      expect(closes.mock.calls.length).toBe(opens.mock.calls.length);
    } finally {
      opens.mockRestore();
      closes.mockRestore();
    }
  });
});

// ---- a game the MODULE refuses to open (M6.5c2, spec §2.4) ----------------
//
// **Remove takes the module's fenced delete, never this database directly.** A
// cross-rev row is this browser's own compatibility note and `forgetUnplayable`
// removes it straight from IndexedDB; a foreign row is the module's LISTING,
// loaded into every tab's Worker at its boot, so a delete behind the Worker's
// back would leave the row listed. The layer's delete never rebuilds (it reads
// the table and the record's event count), which is why it works on a game
// `live_handle` refuses; the Worker commits it only for a game this tab holds.
describe("removing a game this build's bot cannot continue", () => {
  const ID = "local-9";
  /** The Worker messages that matter here, in the order they were sent. */
  const steps = () =>
    currentWorker()
      .posted.map((p) => (p.kind === "call" ? `${p.method} ${p.path}` : p.kind))
      .filter((s) => s === "hold" || s === "release" || s.startsWith("DELETE"));

  it("holds the game, deletes it through the module, and gives the hold back", async () => {
    await putPair(ID, RECORD_TEXT, tableFor(ID));
    await removeForeign(ID);
    expect(steps()).toEqual(["hold", `DELETE /api/games/${ID}`, "release"]);
    // **And nothing behind the Worker's back**: this fake Worker deletes
    // nothing itself, so the stored pair still being here is what says the
    // page did not reach for the database.
    expect((await readBack(ID)).record).toBe(RECORD_TEXT);
  });

  it("refuses in a sentence, and sends nothing, when another tab holds the game", async () => {
    stubWorker({ hold: () => false });
    await expect(removeForeign(ID)).rejects.toThrow(
      "this game is open in another tab \u2014 close it there, then remove it",
    );
    expect(steps()).toEqual(["hold"]);
  });

  it("gives the hold back even when the delete is refused", async () => {
    stubWorker({
      call: (req) =>
        req.method === "DELETE" ? { status: 403, body: { error: "you are not seated at this table" } } : undefined,
    });
    await expect(removeForeign(ID)).rejects.toThrow("you are not seated at this table");
    expect(steps()).toEqual(["hold", `DELETE /api/games/${ID}`, "release"]);
  });
});
