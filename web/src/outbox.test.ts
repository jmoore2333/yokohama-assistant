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
import { resetFactsForTest } from "./local/facts";
import { resetLocalHostForTests } from "./local/client";
import { openDb, readOutboxRow } from "./local/db";
import {
  archiveDisposition,
  drain,
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
import { stubApi, type ApiCall, type Reply } from "./test/harness";
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
      if (req.path === "/api/games" && req.method === "GET") {
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
});
afterEach(() => {
  resetLocalHostForTests();
  resetOutboxForTests();
  resetFactsForTest();
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
