// The outbox (M6.5b2, R-6.5b2.13; spec §2.6): a finished local game that could
// not be uploaded is not lost and not forgotten.
//
// **`fake-indexeddb` and not a double**, for `db.test.ts`'s reason: the row
// this module writes is the memory, and a `Map` standing in for the store would
// answer "is it still there after a reload?" the way its author expected. Every
// cell below reopens the real database.
//
// **Nothing here fabricates a payload.** The archive's three answers and the
// health read are the committed corpus's, through the real `fetch` path, so the
// classification cells run `ApiError` exactly as a browser builds it.
import "fake-indexeddb/auto";
import { IDBFactory } from "fake-indexeddb";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  archiveAnswer,
  recallServerFacts,
  rememberServerFacts,
  resetFactsForTest,
  serverFactsTakenAt,
} from "./local/facts";
import { resetLocalHostForTests } from "./local/client";
import { BOUND_MS, CLAIMED_USER_KEY } from "./api";
import { applyBusy, resetDoorAutoApplyForTests } from "./swRegister";
import { resetTurnstileForTests, setChallengeHandler } from "./turnstile";
import { DB_NAME, openDb, readOutboxRow } from "./local/db";
import { STORAGE_BOUND_MS } from "./backup";
import {
  archiveDisposition,
  drain,
  drainVisible,
  enqueue,
  onArchived,
  noteGameOver,
  pending,
  reasonWords,
  reconsiderAll,
  recordLanded,
  resetOutboxForTests,
  sendNow,
  startOutbox,
} from "./outbox";
import { stubWorker, workerAnswersWith } from "./test/fakeWorker";
import { isGamesList, stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

const GAME = "local-7";
/** A record whose seed would round if anything parsed it — the needle every
 *  other local cell in this suite carries, for the same reason. */
const RECORD = '{"seed":18446744073709551615,"events":[1]}';
const TABLE = '{"schema":3,"id":"local-7"}';

/** The two hosts, scripted: the module answers a pair and a listing, the server
 *  answers health and the archive. */
function hosts(opts: {
  /** What `GET /api/health` says, or `null` for a server that never answers. */
  archive?: "off" | "dir" | null;
  /** What `POST /api/archive` answers. */
  onArchive?: () => Reply;
  /** The games the local listing reports. */
  games?: { id: string; over: boolean }[];
  pair?: { record: string | null; table: string | null };
  /** Which games' pair reads REJECT, and with what — how a cell arranges a
   *  local host that faults on one game and answers for the others. */
  pairFails?: Record<string, string>;
}) {
  const posts: string[] = [];
  const healths: number[] = [];
  /** Every `pair` message, by game — the count I-2 is about. */
  const pairAsks: string[] = [];
  stubWorker({
    pair: (id) => {
      pairAsks.push(id);
      if (opts.pairFails?.[id]) throw new Error(opts.pairFails[id]);
      return opts.pair ?? { record: RECORD, table: TABLE };
    },
    call: (req) => {
      if (isGamesList(req, "mine")) {
        return {
          status: 200,
          body: {
            games: (opts.games ?? []).map((g) => ({
              ...wire.games.games[0]!,
              id: g.id,
              name: null,
              over: g.over,
            })),
            next: null,
          },
        };
      }
      return undefined;
    },
  });
  const stub = stubApi((call: ApiCall) => {
    if (call.path.endsWith("/api/health")) {
      healths.push(1);
      if (opts.archive === null || opts.archive === undefined) throw new TypeError("Failed to fetch");
      return { ...wire.health, archive: opts.archive };
    }
    if (call.method === "POST" && call.path === "/api/archive") {
      posts.push(String(call.body && (call.body as { record?: string }).record));
      return (opts.onArchive ?? (() => wire.archivePost))();
    }
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });
  return { posts, healths, pairAsks, stub };
}

beforeEach(() => {
  vi.stubGlobal("indexedDB", new IDBFactory());
  resetLocalHostForTests();
  resetOutboxForTests();
  resetFactsForTest();
  // **A visitor who has CHOSEN A NAME** (cycle F, F-14): the corpus's health
  // names the open door, and under it a nameless visitor's post is refused by
  // the layer for certain — so the outbox no longer makes one (the cells at the
  // end of this file). Every cell before them is about a send, so its visitor
  // is the one the layer would answer.
  localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
});
afterEach(() => {
  resetLocalHostForTests();
  resetOutboxForTests();
  resetFactsForTest();
  resetTurnstileForTests();
  Object.defineProperty(document, "visibilityState", { value: "visible", configurable: true });
});

// ---- the three states an archive fact puts Abandon and the hook in ---------
describe("what a known archive licenses", () => {
  it("is three answers and not two", () => {
    expect(archiveDisposition("off")).toBe("delete");
    expect(archiveDisposition("dir")).toBe("send");
    expect(archiveDisposition("r2")).toBe("send");
    // **The middle one is the whole finding** (spec §0.3): before it existed,
    // "nothing has ever answered" and "this deployment keeps no archive" were
    // the same value, and Abandon deleted on both.
    expect(archiveDisposition("unknown")).toBe("ask");
  });
});

describe("a game that finishes", () => {
  // **THE cell for the case a first draft of this missed.** A fresh visitor can
  // install the shell, go offline and finish their FIRST game with the archive
  // fact never having been read — so an entry condition of "the archive is
  // known to be on" queues nothing, and coming back online drains an empty
  // list. `"unknown"` is that row.
  it("queues a first game finished before any health answer, and sends it on discovery", async () => {
    const offline = hosts({ archive: null });
    await noteGameOver(GAME);
    expect(offline.healths.length).toBeGreaterThan(0);
    expect(await pending()).toEqual([
      expect.objectContaining({ id: GAME, reason: "unknown" }),
    ]);

    // The server is there now, and the browser's own `online` event is one of
    // the three triggers (spec §2.6).
    const back = hosts({ archive: "dir" });
    const stop = startOutbox();
    window.dispatchEvent(new Event("online"));
    await vi.waitFor(async () => expect(await pending()).toEqual([]));
    expect(back.posts).toEqual([RECORD]);
    stop();
  });

  it("queues a game the archive would not take, and drains when the connection returns", async () => {
    let up = false;
    hosts({
      archive: "dir",
      onArchive: () => {
        if (!up) throw new TypeError("Failed to fetch");
        return wire.archivePost;
      },
    });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([expect.objectContaining({ id: GAME, reason: "offline" })]);

    up = true;
    await drain();
    expect(await pending()).toEqual([]);
  });

  // **The two reasons are different problems and only one has a remedy now.**
  it("tells being signed out from being offline", async () => {
    hosts({
      archive: "dir",
      onArchive: () => ({
        status: 401,
        body: { error: "no signed-in user: send X-Yokohama-Claimed-User" },
      }),
    });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([expect.objectContaining({ id: GAME, reason: "signed-out" })]);
  });

  // The EDGE's 401 is a different refusal with the same status — the turnstile
  // in front of the server, not this API — and it is the same remedy.
  it("reads the edge's own challenge as signed out too", async () => {
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    stubApi((call) => {
      if (call.path.endsWith("/api/health")) return { ...wire.health, archive: "dir" };
      return new Response("", {
        status: 401,
        headers: { "www-authenticate": "Cloudflare-Access resource_metadata=x" },
      });
    });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([expect.objectContaining({ id: GAME, reason: "signed-out" })]);
  });

  // **A permanent refusal is neither of the two remedies and must not retry for
  // ever.** It carries what the server said and leaves the game listed and
  // exportable rather than posting the same bytes again on every `online`.
  it("stops on a permanent refusal and carries the server's sentence", async () => {
    const bad = hosts({
      archive: "dir",
      onArchive: () => ({ status: 400, body: wire.archive400Replay }),
    });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([
      expect.objectContaining({
        id: GAME,
        reason: "refused",
        sentence: wire.archive400Replay.error,
      }),
    ]);

    const posted = bad.posts.length;
    const healths = bad.healths.length;
    await drain();
    expect(bad.posts).toHaveLength(posted);
    // And it does not even ASK the server what it is like: there is nothing
    // the answer could change.
    expect(bad.healths).toHaveLength(healths);
  });

  // A 409 says a DIFFERENT record is archived under this id. It is not offline
  // and not signed out, and sending these bytes again will never change it.
  it("reads a 409 as a refusal and not as a retry", async () => {
    hosts({ archive: "dir", onArchive: () => ({ status: 409, body: wire.archive409 }) });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([
      expect.objectContaining({ id: GAME, reason: "refused", sentence: wire.archive409.error }),
    ]);
  });

  it("queues nothing where the deployment is KNOWN to keep no archive", async () => {
    const off = hosts({ archive: "off" });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([]);
    expect(off.posts).toEqual([]);
  });

  // **The receipt, and it is what makes the hook idempotent against a reload**
  // (spec §2.6): the game is still over on the next visit, and the hook fires
  // again.
  it("sends once for one ending, however often the hook fires", async () => {
    const sent = hosts({ archive: "dir" });
    await noteGameOver(GAME);
    await noteGameOver(GAME);
    expect(sent.posts).toEqual([RECORD]);
    expect(await pending()).toEqual([]);
  });

  // **"does not re-enter on its OLD bytes"** (spec §2.6). A game undone and
  // played to a different ending is a different record, and the receipt is
  // about the bytes rather than about the name — so the new ending is offered
  // to the archive and the server's answer, whatever it is, is the player's to
  // read.
  it("sends again once the record has moved on", async () => {
    const sent = hosts({ archive: "dir" });
    await noteGameOver(GAME);
    const REPLAYED = '{"seed":18446744073709551615,"events":[1,2]}';
    workerAnswersWith(GAME, { record: REPLAYED, table: TABLE });
    await noteGameOver(GAME);
    expect(sent.posts).toEqual([RECORD, REPLAYED]);
  });

  it("owes nothing for a game with no record on this device", async () => {
    const none = hosts({ archive: "dir", pair: { record: null, table: null } });
    await noteGameOver(GAME);
    expect(none.posts).toEqual([]);
    expect(await pending()).toEqual([]);
  });
});

describe("draining", () => {
  // **Serialised, so two triggers cannot post twice** (spec §2.6). The `online`
  // event and the launch drain can land in the same tick, and a second pass
  // reading the store before the first has written its receipt would send the
  // same bytes twice.
  it("posts once when two triggers arrive together", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    const back = hosts({ archive: "dir" });
    await Promise.all([drain(), drain(), drain()]);
    expect(back.posts).toEqual([RECORD]);
    expect(await pending()).toEqual([]);
  });

  it("is what the player's own button runs", async () => {
    const offline = hosts({ archive: null });
    await noteGameOver(GAME);
    expect(offline.posts).toEqual([]);
    const back = hosts({ archive: "dir" });
    await sendNow(GAME);
    expect(back.posts).toEqual([RECORD]);
    expect(await pending()).toEqual([]);
  });

  // **Reasons are re-evaluated, not frozen** (spec §2.6). A row entered when
  // nothing had answered resolves the moment the answer arrives.
  it("re-reads the reason when the archive fact is discovered", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([expect.objectContaining({ reason: "unknown" })]);

    const back = hosts({ archive: "dir", games: [{ id: GAME, over: true }] });
    await reconsiderAll();
    expect(back.posts).toEqual([RECORD]);
    expect(await pending()).toEqual([]);
  });

  // ...or to nothing at all, where the deployment keeps no archive. A row that
  // stayed would be a door promising a send that is never going to happen.
  it("drops every waiting row where the deployment keeps no archive", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    expect(await pending()).toHaveLength(1);

    const off = hosts({ archive: "off", games: [{ id: GAME, over: true }] });
    await reconsiderAll();
    expect(await pending()).toEqual([]);
    expect(off.posts).toEqual([]);
  });

  // The sweep is over FINISHED games and not over every game: a game still
  // being played has no ending to archive.
  it("reconsiders a finished game that was never queued, and leaves a running one alone", async () => {
    const found = hosts({
      archive: "dir",
      games: [
        { id: GAME, over: true },
        { id: "local-running", over: false },
      ],
    });
    await reconsiderAll();
    expect(found.posts).toEqual([RECORD]);
    // And the receipt went down, so the next sweep is quiet.
    await reconsiderAll();
    expect(found.posts).toEqual([RECORD]);
  });

  it("leaves a refused row where it is when the fact is rediscovered", async () => {
    hosts({ archive: "dir", onArchive: () => ({ status: 400, body: wire.archive400Replay }) });
    await noteGameOver(GAME);
    const again = hosts({ archive: "dir", games: [{ id: GAME, over: true }] });
    await reconsiderAll();
    expect(again.posts).toEqual([]);
    expect(await pending()).toEqual([expect.objectContaining({ reason: "refused" })]);
  });
});

describe("the manual send", () => {
  // The control on a finished game's own screen was already there; what it
  // gains is the row. A send that lands from anywhere is a landed send.
  it("clears the waiting row", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    expect(await pending()).toHaveLength(1);
    await recordLanded(GAME, RECORD);
    expect(await pending()).toEqual([]);
    // And the receipt is down, so the hook does not send the same bytes again.
    const after = hosts({ archive: "dir" });
    await noteGameOver(GAME);
    expect(after.posts).toEqual([]);
  });
});

describe("a browser with no usable storage", () => {
  // A private window, a blocked site, a browser with no IndexedDB at all: the
  // outbox is a memory, and a browser that cannot remember has none. **Every
  // entry point answers rather than throwing**, because each of them is called
  // with `void` from a render or an event handler and a rejection there is an
  // unhandled rejection on a screen that is otherwise fine.
  it("answers everything and queues nothing", async () => {
    vi.stubGlobal("indexedDB", undefined);
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    stubApi((call) =>
      call.path.endsWith("/api/health") ? { ...wire.health, archive: "dir" } : wire.archivePost,
    );
    await expect(pending()).resolves.toEqual([]);
    await expect(noteGameOver(GAME)).resolves.toBeUndefined();
    await expect(drain()).resolves.toBeUndefined();
    await expect(reconsiderAll()).resolves.toBeUndefined();
    await expect(sendNow(GAME)).resolves.toBeUndefined();
  });
});

describe("the row a landed send leaves behind", () => {
  it("is a receipt and not a pending row", async () => {
    hosts({ archive: "dir" });
    await noteGameOver(GAME);
    const db = await openDb();
    expect(await readOutboxRow(db, GAME)).toEqual(
      expect.objectContaining({ id: GAME, state: "sent" }),
    );
  });
});

// ---- one bad game must not block the good ones (review I-1) ----------------
//
// **`localPair` rejects whenever the local host faults, traps or is torn down
// mid-message.** Before the fix that rejection escaped the drain's loop into
// one outer `catch {}`: the loop exited, nothing was written down, and every
// row behind it was never tried — and because each later trigger re-reads the
// store in the same order, the same row failed first every time. One game whose
// record reliably trapped the module would have blocked every other game's send
// for ever, with nothing on the door saying so.
describe("a row that cannot even be read", () => {
  const OTHER = "local-8";

  it("does not stop the rows behind it, and is recorded rather than dropped", async () => {
    // Two games waiting, and the FIRST one's pair read throws.
    hosts({ archive: null });
    await noteGameOver(GAME);
    await noteGameOver(OTHER);
    expect((await pending()).map((r) => r.id).sort()).toEqual([GAME, OTHER]);

    const back = hosts({
      archive: "dir",
      pairFails: { [GAME]: "the local host restarted — reloaded from the last saved move" },
    });
    await drain();

    // The good row went.
    expect(back.posts).toEqual([RECORD]);
    const rows = await pending();
    expect(rows.map((r) => r.id)).toEqual([GAME]);
    // **Recorded, not dropped** — and worded as what actually happened. "No
    // connection" would be a false sentence about a module that trapped.
    expect(rows[0]).toEqual(
      expect.objectContaining({
        id: GAME,
        reason: "offline",
        sentence: "the local host restarted — reloaded from the last saved move",
      }),
    );
    expect(rows[0]!.lastTried).toBeGreaterThan(0);
    // And the WORDS on the door are that sentence, not the connection's.
    expect(reasonWords(rows[0]!.reason, rows[0]!.sentence)).toBe(
      "the local host restarted — reloaded from the last saved move",
    );
  });

  it("says no connection where the failure carried no sentence of its own", async () => {
    hosts({ archive: "dir", onArchive: () => ({ status: 503, body: { error: "unavailable" } }) });
    await noteGameOver(GAME);
    const row = (await pending())[0]!;
    // A 503 is the moment and not the record, so it is retried — and the words
    // stay the connection's, because nothing local failed.
    expect(row.reason).toBe("offline");
    expect(reasonWords(row.reason, row.sentence)).toBe("no connection");
  });

  it("goes on through the sweep too, where the listing names a game the module cannot read", async () => {
    const found = hosts({
      archive: "dir",
      games: [
        { id: GAME, over: true },
        { id: OTHER, over: true },
      ],
      pairFails: { [GAME]: "the local host stopped" },
    });
    await reconsiderAll();
    expect(found.posts).toEqual([RECORD]);
    expect((await pending()).map((r) => r.id)).toEqual([GAME]);
  });
});

// ---- ONE snapshot per game per attempt (review I-2; spec §2.2) -------------
//
// **The receipt has to be checked against the very bytes that are posted.** An
// earlier draft read the pair to check the receipt and then `offer` read it
// AGAIN to post it: two Worker round-trips per game, and two answers to "what
// is this game" where the server is checking that the record and the table
// agree.
describe("the snapshot a game is sent as", () => {
  it("is read ONCE per game per attempt", async () => {
    const sent = hosts({ archive: "dir" });
    await noteGameOver(GAME);
    expect(sent.pairAsks).toEqual([GAME]);
    expect(sent.posts).toEqual([RECORD]);
  });

  it("is read once per game in a drain over several rows", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    await noteGameOver("local-8");
    const back = hosts({ archive: "dir" });
    await drain();
    expect(back.pairAsks.sort()).toEqual([GAME, "local-8"]);
  });

  it("is read once per game in the sweep", async () => {
    const found = hosts({ archive: "dir", games: [{ id: GAME, over: true }] });
    await reconsiderAll();
    expect(found.pairAsks).toEqual([GAME]);
  });
});

// ---- what a landed send tells the screen (review M-6) ---------------------
describe("a send that lands", () => {
  it("names the game and the SERVER's path, so a control is not offered twice", async () => {
    hosts({ archive: "dir" });
    const landed: [string, string][] = [];
    const off = onArchived((id, path) => landed.push([id, path]));
    try {
      await noteGameOver(GAME);
      expect(landed).toEqual([[GAME, wire.archivePost.path]]);
    } finally {
      off();
    }
  });

  it("says nothing when the send did not land", async () => {
    hosts({ archive: "dir", onArchive: () => ({ status: 400, body: wire.archive400Replay }) });
    const landed: string[] = [];
    const off = onArchived((id) => landed.push(id));
    try {
      await noteGameOver(GAME);
      expect(landed).toEqual([]);
    } finally {
      off();
    }
  });
});

// ---- the reason a row was given is not thrown away (review M-7) -----------
describe("a drain that learns nothing", () => {
  it("leaves each row's own reason where it is", async () => {
    // A row entered as "signed out" on a browser that has never had a landed
    // health read is not reachable in production; the RULE is what this pins —
    // a drain that cannot learn the fact must not overwrite what a row says.
    await enqueue(GAME, "signed-out");
    hosts({ archive: null });
    await drain();
    expect((await pending())[0]).toEqual(expect.objectContaining({ reason: "signed-out" }));
  });
});

// ---- the connections this module opens are given back (review M-2) --------
describe("the database handles", () => {
  it("closes every connection it opens, across a whole drain", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    await noteGameOver("local-8");

    const opens = vi.spyOn(indexedDB, "open");
    const closes = vi.spyOn(IDBDatabase.prototype, "close");
    hosts({ archive: "dir" });
    await drain();
    await pending();
    // The premise: this really did open connections, so the equality below is
    // not two zeros.
    expect(opens.mock.calls.length).toBeGreaterThan(1);
    expect(closes.mock.calls.length).toBe(opens.mock.calls.length);
  });
});

// ---- another tab holding the OLD storage version (M6.5c2 cycle D, row 12) --
//
// **The outbox's own open had the hang "Back up everything" had**: an
// M6.5b-era tab holds IndexedDB version 1 with no `versionchange` handler, so
// this build's open waits for as long as that tab lives, and the outbox's
// fallback — "a browser with no usable store has no outbox" — never arrived:
// the door's waiting list was a read that never settled. It opens through
// `backup.ts`'s bounded helper now, so a held store is answered like any
// store it cannot use, within the same bound. Staged as `backup.test.ts`'s
// Review Focus 5 cell stages it: this tab's Worker's open queued first, so
// only the timer can answer.
describe("another tab holding the old storage version", () => {
  const holdVersionOne = (): Promise<IDBDatabase> =>
    new Promise((resolve, reject) => {
      const r = indexedDB.open(DB_NAME, 1);
      r.onupgradeneeded = () => {
        const db = r.result;
        if (!db.objectStoreNames.contains("docs"))
          db.createObjectStore("docs", { keyPath: ["doc", "id"] });
        if (!db.objectStoreNames.contains("meta")) db.createObjectStore("meta", { keyPath: "id" });
      };
      r.onsuccess = () => resolve(r.result);
      r.onerror = () => reject(r.error);
    });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("answers its fallback within the storage bound, never a hang", async () => {
    const old = await holdVersionOne();
    const workersOpen = openDb(); // this tab's Worker, already waiting
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    let answer: unknown = "unanswered";
    void pending().then((p) => (answer = p));
    await vi.advanceTimersByTimeAsync(STORAGE_BOUND_MS - 1);
    expect(answer, "the premise: a queued open hears nothing before the bound").toBe("unanswered");
    await vi.advanceTimersByTimeAsync(1);
    expect(answer).toEqual([]);
    vi.useRealTimers();
    old.close();
    (await workersOpen).close();
  });
});

// ---- a deployment that keeps NO archive answers the route 404 (M6.5c) -----
//
// **R-6.5c.14, spec §2.6: "the client's dead end, closed".** A deployment whose
// archive is off answers `POST /api/archive` with the layer's 404, "this server
// keeps no archive". Before this branch `reasonFor` filed that 404 as
// "offline", so a browser that had read `archive: "dir"` less than an hour
// before the switch was thrown posted the same bytes on every reconnect until
// its remembered answer went stale. The 404 IS an answer about the deployment:
// the row goes, and the remembered fact becomes `off`.
//
// **The fact's AGE does not move** (plan review m2). The other fields of the
// remembered health are no fresher than they were, so the record keeps the time
// it was taken; only `archive` changes, in memory AND in the copy a reload reads.
describe("a deployment whose archive route answers 404", () => {
  const KEEPS_NONE = { status: 404, body: { error: "this server keeps no archive" } };

  /** A health answer this browser read a minute ago: fresh, so nothing asks
   *  again before the send, and old enough that a restamp would show. */
  const rememberedDir = (): number => {
    const at = Date.now() - 60_000;
    rememberServerFacts({ ...wire.health, archive: "dir" }, at);
    return at;
  };

  it("forgets the game, and remembers the archive OFF without making the fact younger (battery 28)", async () => {
    const at = rememberedDir();
    const off = hosts({ archive: "dir", onArchive: () => KEEPS_NONE });
    await noteGameOver(GAME);
    // The premise: the send happened, and the fact it went out on was the
    // remembered one — no health read moved `takenAt` in between.
    expect(off.posts).toEqual([RECORD]);
    expect(off.healths).toEqual([]);

    expect(await pending()).toEqual([]);
    // Forgotten, and not a receipt either: nothing landed.
    const db = await openDb();
    try {
      expect(await readOutboxRow(db, GAME)).toBeNull();
    } finally {
      db.close();
    }
    expect(archiveAnswer()).toBe("off");
    expect(serverFactsTakenAt()).toBe(at);
    // What the next page load reads: the WRITTEN copy changed, not only this
    // module's memory of it.
    resetFactsForTest();
    expect(recallServerFacts()?.archive).toBe("off");
  });

  // **By the status, never the words** (`reasonFor`'s own rule): a 404 from
  // something in front of the server carries no sentence of this API's, and
  // says the same thing — there is no archive route here to send to.
  it("reads the 404 by its status, not its sentence", async () => {
    rememberedDir();
    hosts({
      archive: "dir",
      onArchive: () => new Response("<html>Not Found</html>", { status: 404 }),
    });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([]);
    expect(archiveAnswer()).toBe("off");
  });

  // **Every site that sends** — and the drain is the one that matters most: a
  // row queued while the archive was on meets the switch thrown to off.
  it("forgets a waiting row the same way when a drain meets the 404", async () => {
    const at = rememberedDir();
    hosts({
      archive: "dir",
      onArchive: () => {
        throw new TypeError("Failed to fetch");
      },
    });
    await noteGameOver(GAME);
    expect(await pending()).toEqual([expect.objectContaining({ id: GAME, reason: "offline" })]);

    const off = hosts({ archive: "dir", onArchive: () => KEEPS_NONE });
    await drain();
    expect(off.posts).toEqual([RECORD]);
    expect(await pending()).toEqual([]);
    expect(archiveAnswer()).toBe("off");
    expect(serverFactsTakenAt()).toBe(at);
  });

  // **L-6: the FIRST 404 is the answer for every row after it** (M6.5c2 spec
  // §2.8; the M6.5c Task 8 review's Minor 3). `kind` is read once per pass, so
  // after row 1's 404 wrote "off" the rows behind it were each posted in full
  // and each told the same thing — a player who finished several games offline
  // on a deployment since switched off paid one whole record upload per game.
  it("stops posting after the first 404 in a drain, and forgets every waiting row", async () => {
    hosts({ archive: null });
    await noteGameOver(GAME);
    await noteGameOver("local-8");
    expect(await pending()).toHaveLength(2);

    rememberedDir();
    const off = hosts({ archive: "dir", onArchive: () => KEEPS_NONE });
    await drain();
    expect(off.posts, "ONE upload: the first 404 answers for the rest").toHaveLength(1);
    expect(await pending()).toEqual([]);
    expect(archiveAnswer()).toBe("off");
  });

  it("stops posting after the first 404 in the sweep too", async () => {
    rememberedDir();
    const off = hosts({
      archive: "dir",
      onArchive: () => KEEPS_NONE,
      games: [
        { id: GAME, over: true },
        { id: "local-8", over: true },
      ],
    });
    await reconsiderAll();
    expect(off.posts, "ONE upload in the sweep as well").toHaveLength(1);
    expect(archiveAnswer()).toBe("off");
  });
});

// ---- cycle F: the edge's challenge, and the nameless visitor (spec §2.6) ----
//
// **The outbox works when nobody is watching, and a challenge needs a person.**
// So a pass that cannot show the widget — the browser's `online` event, a
// launch in a hidden tab — sends with `interactive: false`, and the edge's
// challenge, met by its health read OR a send, marks EVERY waiting row
// "waiting for a human check" rather than one row at a time (F-15). Those rows
// wait for a VISIBLE tab — the door's launch, or the row's own Check now —
// where `drainVisible` shows the widget through the seam and sends again. The
// challenge is the EDGE's shape, held beside these cells; every answer the
// layer gives is the corpus's.
describe("the edge's challenge (cycle F)", () => {
  const SECOND = "local-8";
  const challenged = () =>
    new Response("<html>challenge</html>", {
      status: 403,
      headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
    });
  const visibility = (state: "visible" | "hidden") =>
    Object.defineProperty(document, "visibilityState", { value: state, configurable: true });
  /** The archive is ON, remembered a minute ago — no health read is owed. */
  const rememberedDir = () => rememberServerFacts({ ...wire.health, archive: "dir" }, Date.now() - 60_000);
  /** Two games waiting, each for a reason that is not the edge's. */
  const twoWaiting = async () => {
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "offline");
    await enqueue(SECOND, "unknown");
  };
  const reasons = async () => (await pending()).map((r) => [r.id, r.reason]).sort();

  // **F battery 28: a challenged health read marks nothing.** The health read is
  // a fresh context's FIRST request, so it is the one the edge challenges first;
  // an outbox that swallowed it as "nothing answered" left every row as it
  // was, and the next `online` asked again — and was challenged again.
  it("marks EVERY waiting row challenged when the background pass's HEALTH read meets the edge, and shows no widget (F battery 28)", async () => {
    await twoWaiting();
    const handler = vi.fn(() => Promise.resolve());
    setChallengeHandler(handler);
    const stub = stubApi(() => challenged());
    await drain({ interactive: false });
    expect(await reasons()).toEqual([
      [GAME, "challenged"],
      [SECOND, "challenged"],
    ]);
    expect(handler).not.toHaveBeenCalled();
    expect(stub.calls.map((c) => `${c.method} ${c.path}`)).toEqual(["GET /api/health"]);
  });

  it("marks EVERY waiting row challenged when a SEND meets the edge, posting nothing after it", async () => {
    await twoWaiting();
    rememberedDir();
    const handler = vi.fn(() => Promise.resolve());
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (call.method === "POST" && call.path === "/api/archive") return challenged();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await drain({ interactive: false });
    expect(await reasons()).toEqual([
      [GAME, "challenged"],
      [SECOND, "challenged"],
    ]);
    expect(handler).not.toHaveBeenCalled();
    // ONE post: the challenge answers for every row behind it.
    expect(stub.matching("POST", "/api/archive")).toHaveLength(1);
  });

  // **F battery 27: the non-interactive drain pops the widget.** The two
  // background triggers — a launch in a hidden tab and the browser's `online`
  // event — show nothing, and the second does not even ask: a challenged row
  // waits for a person, and asking again for it is a request the edge refuses.
  it("shows no widget from a hidden tab's launch, and the next online event retries no challenged row (F battery 27)", async () => {
    await twoWaiting();
    rememberedDir();
    visibility("hidden");
    const handler = vi.fn(() => Promise.resolve());
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (call.method === "POST" && call.path === "/api/archive") return challenged();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const stop = startOutbox();
    await vi.waitFor(async () =>
      expect(await reasons()).toEqual([
        [GAME, "challenged"],
        [SECOND, "challenged"],
      ]),
    );
    const asked = stub.calls.length;
    window.dispatchEvent(new Event("online"));
    await drain({ interactive: false }); // queued behind the event's own pass
    expect(stub.calls, "the online event asked nothing").toHaveLength(asked);
    expect(handler).not.toHaveBeenCalled();
    stop();
  });

  it("retries the challenged rows through the widget at a VISIBLE tab's launch, and clears them when the sends land", async () => {
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "challenged");
    await enqueue(SECOND, "challenged");
    rememberedDir();
    let cleared = false;
    const handler = vi.fn(() => {
      cleared = true;
      return Promise.resolve();
    });
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (call.method === "POST" && call.path === "/api/archive") return cleared ? wire.archivePost : challenged();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const stop = startOutbox();
    await vi.waitFor(async () => expect(await pending()).toEqual([]));
    // The widget ONCE, then the first send again and the second under the
    // clearance.
    expect(handler).toHaveBeenCalledTimes(1);
    expect(stub.matching("POST", "/api/archive")).toHaveLength(3);
    stop();
  });

  it("does the same on drainVisible itself — the row's Check now — and never from a hidden tab", async () => {
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "challenged");
    rememberedDir();
    let cleared = false;
    const handler = vi.fn(() => {
      cleared = true;
      return Promise.resolve();
    });
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (call.method === "POST" && call.path === "/api/archive") return cleared ? wire.archivePost : challenged();
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    // Hidden: nothing — a challenged row is not a background pass's.
    visibility("hidden");
    await drainVisible();
    expect(stub.calls).toHaveLength(0);
    expect(handler).not.toHaveBeenCalled();
    visibility("visible");
    await drainVisible();
    expect(handler).toHaveBeenCalledTimes(1);
    expect(await pending()).toEqual([]);
  });

  it("keeps a row challenged when the visible check is cancelled", async () => {
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "challenged");
    rememberedDir();
    setChallengeHandler(() => Promise.reject(new Error("cancelled")));
    stubApi(() => challenged());
    await drainVisible();
    expect(await reasons()).toEqual([[GAME, "challenged"]]);
  });

  it("says waiting for a human check", () => {
    expect(reasonWords("challenged")).toBe("waiting for a human check");
  });

  // **F battery 29: the nameless visitor's game posted** (F-14). Under the open
  // door a visitor who never chose a name finishes a bot game; the layer's
  // refusal of their post is certain, so the post is not made — the row says
  // "not signed in" (whose remedy names the name step) and nothing is sent.
  it("marks a nameless visitor's finished game signed-out under the open door, with NO request (F battery 29)", async () => {
    localStorage.removeItem(CLAIMED_USER_KEY);
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    rememberServerFacts({ ...wire.health, identity: "open", archive: "dir" }, Date.now() - 60_000);
    const stub = stubApi((call) => {
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await noteGameOver(GAME);
    expect(await reasons()).toEqual([[GAME, "signed-out"]]);
    expect(stub.calls, "nothing was sent").toHaveLength(0);

    // Anti-vacuity: the same visitor, NAMED, posts.
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    const named = stubApi((call) =>
      call.method === "POST" && call.path === "/api/archive" ? wire.archivePost : undefined,
    );
    await noteGameOver(SECOND);
    expect(named.matching("POST", "/api/archive")).toHaveLength(1);
  });

  it("still posts a nameless visitor's game under any other door", async () => {
    localStorage.removeItem(CLAIMED_USER_KEY);
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    rememberServerFacts({ ...wire.health, identity: "cf-access", archive: "dir" }, Date.now() - 60_000);
    const stub = stubApi((call) =>
      call.method === "POST" && call.path === "/api/archive" ? wire.archivePost : undefined,
    );
    await noteGameOver(GAME);
    expect(stub.matching("POST", "/api/archive")).toHaveLength(1);
    expect(await pending()).toEqual([]);
  });
});

// ---- Task 6 fix round 1 ------------------------------------------------------
//
// **A keyed build IS the open door** (R-FP4, one build per door state:
// production, behind Access, is built WITHOUT the site key, and only the open
// configuration deploys a build with one; the deploy guard's check 12 holds the
// declaration to the stamp and, from Task 7, to the open identity). So under a keyed build a
// visitor who has chosen no name is refused an upload for CERTAIN, and nothing
// needs to be asked to know it: no health read, no widget, no post — at game
// over, at a launch, in a drain. A keyless build (the Access builds, the NAS)
// cannot know its door without the read, and makes it as E did.
describe("a nameless visitor under a keyed build (Task 6 fix round 1, Important 1)", () => {
  const SECOND = "local-8";
  const challenged = () =>
    new Response("<html>challenge</html>", {
      status: 403,
      headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
    });
  const reasons = async () => (await pending()).map((r) => [r.id, r.reason]).sort();
  /** The remembered facts: the open door, keeping an archive, read `age` ms ago. */
  const remembered = (age: number) =>
    rememberServerFacts({ ...wire.health, identity: "open", archive: "dir" }, Date.now() - age);

  beforeEach(() => {
    localStorage.removeItem(CLAIMED_USER_KEY);
    vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", "site-key-fixture");
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
  });

  it("makes NO request at a fresh context's game over, and marks the game signed-out", async () => {
    const handler = vi.fn(() => Promise.resolve());
    setChallengeHandler(handler);
    const stub = stubApi(() => challenged());
    expect(serverFactsTakenAt(), "the premise: nothing has ever been read").toBeNull();
    await noteGameOver(GAME);
    expect(stub.calls, "no health read and no post").toHaveLength(0);
    expect(handler).not.toHaveBeenCalled();
    expect(await reasons()).toEqual([[GAME, "signed-out"]]);
  });

  it("makes NO request for a signed-out row at a visible launch, however stale the facts, nor for any row in a drain", async () => {
    await enqueue(GAME, "signed-out");
    await enqueue(SECOND, "offline");
    remembered(2 * 3_600_000);
    const handler = vi.fn(() => Promise.resolve());
    setChallengeHandler(handler);
    const stub = stubApi(() => challenged());
    const stop = startOutbox();
    await drainVisible(); // queued behind the launch's own pass
    await drain({ interactive: false });
    await sendNow(SECOND);
    expect(stub.calls, "nothing asked of the server").toHaveLength(0);
    expect(handler).not.toHaveBeenCalled();
    // The row a drain would have sent is written down for what it is waiting
    // on — a name — and the signed-out row stays as it was.
    expect(await reasons()).toEqual([
      [GAME, "signed-out"],
      [SECOND, "signed-out"],
    ]);
    stop();
  });

  it("retries a signed-out row through the widget at a visible launch once a name is stored", async () => {
    await enqueue(GAME, "signed-out");
    remembered(2 * 3_600_000);
    localStorage.setItem(CLAIMED_USER_KEY, wire.me.user!);
    let cleared = false;
    const handler = vi.fn(() => {
      cleared = true;
      return Promise.resolve();
    });
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (!cleared) return challenged();
      if (call.path.endsWith("/api/health")) return { ...wire.health, archive: "dir" };
      if (call.method === "POST" && call.path === "/api/archive") return wire.archivePost;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    const stop = startOutbox();
    await vi.waitFor(async () => expect(await pending()).toEqual([]));
    expect(handler).toHaveBeenCalledTimes(1);
    expect(stub.matching("POST", "/api/archive")).toHaveLength(1);
    stop();
  });

  it("keeps E's health read in a KEYLESS build, where the door is not known without it", async () => {
    vi.stubGlobal("__YOKOHAMA_TURNSTILE_SITEKEY__", null);
    const stub = stubApi((call) => {
      if (call.path.endsWith("/api/health")) return { ...wire.health, identity: "open", archive: "dir" };
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    await noteGameOver(GAME);
    expect(stub.calls.map((c) => `${c.method} ${c.path}`)).toEqual(["GET /api/health"]);
    // The read named the open door, and the post it would refuse is not made
    // (F-14, the first commit's rule).
    expect(await reasons()).toEqual([[GAME, "signed-out"]]);
  });
});

// **The check outlasts the read's 5 s wait** (Task 6 fix round 1, Important
// 2). The bound is the CALLER's wait, not the request's: a person passing the
// widget in 6 s pays for an answer, and the answer is kept when it lands — the
// fact written down, the rows sent again through the ordinary pass under the
// clearance, with no second widget — and the door's own apply is held until
// then, so a waiting release cannot reload the page under the overlay.
describe("a check that outlasts the archive fact's wait (Task 6 fix round 1, Important 2)", () => {
  const SECOND = "local-8";
  const challenged = () =>
    new Response("<html>challenge</html>", {
      status: 403,
      headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
    });
  /** A real turn of the event loop: the store runs on `setImmediate`, which
   *  the cell leaves real while it fakes the clock. */
  const tick = () => new Promise<void>((r) => setImmediate(r));
  const until = async (ok: () => boolean | Promise<boolean>, what: string) => {
    for (let i = 0; i < 400; i += 1) {
      if (await ok()) return;
      await tick();
    }
    throw new Error(`${what} never became true`);
  };

  afterEach(() => {
    vi.useRealTimers();
    resetDoorAutoApplyForTests();
  });

  it("returns at the bound, keeps the answer that lands at 6 s, lands the rows with no second widget, and holds the door's apply until then", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "challenged");
    await enqueue(SECOND, "challenged");
    expect(serverFactsTakenAt(), "the premise: a fresh context, nothing read").toBeNull();
    let cleared = false;
    const handler = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          setTimeout(() => {
            cleared = true;
            resolve();
          }, 6_000);
        }),
    );
    setChallengeHandler(handler);
    const stub = stubApi((call) => {
      if (!cleared) return challenged();
      if (call.path.endsWith("/api/health")) return { ...wire.health, archive: "dir" };
      if (call.method === "POST" && call.path === "/api/archive") return wire.archivePost;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    let returned = false;
    const drained = drainVisible().then(() => {
      returned = true;
    });
    await until(() => handler.mock.calls.length > 0, "the widget was asked for");

    await vi.advanceTimersByTimeAsync(BOUND_MS);
    await until(() => returned, "the drain returned at the bound");
    await drained;
    expect(cleared, "the overlay is still up").toBe(false);
    expect(applyBusy(), "the door's apply is held under the overlay").toBe(true);
    expect(serverFactsTakenAt()).toBeNull();

    await vi.advanceTimersByTimeAsync(6_000 - BOUND_MS);
    expect(cleared).toBe(true);
    await until(async () => (await pending()).length === 0, "the rows landed");
    expect(serverFactsTakenAt(), "the answer the check paid for is kept").not.toBeNull();
    expect(archiveAnswer()).toBe("dir");
    expect(handler, "no second widget").toHaveBeenCalledTimes(1);
    expect(stub.matching("POST", "/api/archive")).toHaveLength(2);
    await until(() => !applyBusy(), "the door's apply was released");
  });

  // **Fix round 2: the late read is BOUNDED** (CLAUDE.md, M6.5c I-86: "a read
  // that never settles is a state, not a delay"). Past the caller's wait, a
  // read that never answers would hold the door's apply for as long as it is
  // out — minutes, on a stalled connection. It gets the transport's own bound
  // after any pass in progress, and then the hold goes and the rows read
  // "waiting for a human check", exactly as a late refusal leaves them.
  it("releases the door's hold and marks the rows challenged when a late read never answers", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "offline");
    await enqueue(SECOND, "unknown");
    const stub = stubApi(() => new Promise<Response>(() => {}));
    let returned = false;
    const drained = drainVisible().then(() => {
      returned = true;
    });
    await until(() => stub.calls.length > 0, "the health read went out");
    await vi.advanceTimersByTimeAsync(BOUND_MS);
    await until(() => returned, "the drain returned at the bound");
    await drained;
    expect(applyBusy(), "held past the caller's wait").toBe(true);
    await vi.advanceTimersByTimeAsync(BOUND_MS - 1);
    expect(applyBusy(), "held through the late read's own bound").toBe(true);
    await vi.advanceTimersByTimeAsync(1);
    await until(() => !applyBusy(), "the hold was released at the bound");
    const reasons = (await pending()).map((r) => [r.id, r.reason]).sort();
    expect(reasons).toEqual([
      [GAME, "challenged"],
      [SECOND, "challenged"],
    ]);
    expect(stub.calls, "nothing asked again").toHaveLength(1);
  });

  // The bound's partner: a pass still in the overlay is the overlay's to end
  // (its own 90 s), and the late read's bound starts after it — so a person
  // who takes 12 s is not cut off at 10.
  it("does not cut a pass that is still in the overlay: a 12 s check lands its rows", async () => {
    vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
    stubWorker({ pair: () => ({ record: RECORD, table: TABLE }) });
    await enqueue(GAME, "challenged");
    let cleared = false;
    const handler = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          setTimeout(() => {
            cleared = true;
            resolve();
          }, 12_000);
        }),
    );
    setChallengeHandler(handler);
    stubApi((call) => {
      if (!cleared) return challenged();
      if (call.path.endsWith("/api/health")) return { ...wire.health, archive: "dir" };
      if (call.method === "POST" && call.path === "/api/archive") return wire.archivePost;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    void drainVisible();
    await until(() => handler.mock.calls.length > 0, "the widget was asked for");
    await vi.advanceTimersByTimeAsync(2 * BOUND_MS + 1_000);
    expect(applyBusy(), "held while the overlay is up").toBe(true);
    expect(await pending(), "not marked while the check is still running").toEqual([
      expect.objectContaining({ id: GAME, reason: "challenged" }),
    ]);
    await vi.advanceTimersByTimeAsync(12_000 - (2 * BOUND_MS + 1_000));
    await until(async () => (await pending()).length === 0, "the row landed");
    expect(handler).toHaveBeenCalledTimes(1);
    await until(() => !applyBusy(), "the hold was released");
  });
});
