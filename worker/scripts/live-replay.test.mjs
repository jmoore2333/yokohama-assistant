import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  FROZEN_CREATED_AT,
  ORIGIN,
  filterLobby,
  firstSendAccepts,
  freezeClock,
  houseDeleteRow,
  loadCorpus,
  loadScripts,
  normalizeHouseChat,
  order,
  readCredential,
  replay,
  resetRecipe,
  urlOf,
} from "./live-replay.mjs";

// **The live corpus replay's cells** (spec §5.3's live half; the plan's Task
// 15). Every reply body a cell compares is a COMMITTED payload, or one derived
// from it the way the live Worker differs from the axum host the corpus was cut
// from — a real clock, its own host fields, a lobby and a house chat other
// people share. Nothing here sends a request: the driver's cells hand it a
// fake host, and the credential's cells read a scratch file of dummy values.

const CORPUS = loadCorpus();
const SCRIPTS = loadScripts();
const committed = (name) => JSON.parse(CORPUS[name]);

/** The replay's three table ids, as its scripts create them. */
const IDS = ["wire2", "wire", "wire4"];
/** A reading of the Worker's clock: a real one, past the frozen reading. */
const CLOCK = 1_790_300_000;
/** The house chat's `last` when a run starts. */
const BASE = 4321;
/** The house chat's `rev` when a run starts (M6.5c2 cycle D): unlike `last`,
 *  and it must be, so a revision taken back by the wrong base fails. */
const BASE_REV = 9876;

describe("freezeClock", () => {
  // `wire_fixtures.rs`'s `freeze_clock`, which the committed files were cut
  // under: every `created_at` and every page cursor's time read the frozen
  // reading, wherever they sit.
  it("freezes every created_at and every cursor's time, at any depth, and leaves its input alone", () => {
    const got = {
      created_at: CLOCK,
      games: [{ created_at: CLOCK + 7, id: "wire" }],
      next: `${CLOCK + 7},wire`,
      messages: [{ seq: 1, created_at: CLOCK + 9 }],
    };
    const before = structuredClone(got);
    expect(freezeClock(got)).toEqual({
      created_at: FROZEN_CREATED_AT,
      games: [{ created_at: FROZEN_CREATED_AT, id: "wire" }],
      next: `${FROZEN_CREATED_AT},wire`,
      messages: [{ seq: 1, created_at: FROZEN_CREATED_AT }],
    });
    expect(got, "the reply read is not rewritten in place").toEqual(before);
  });
  // `null` is the last page and carries no clock; nothing else is a clock to
  // the emitter — an `archived_at` is left as it reads.
  it("leaves a null next and every other field as it reads", () => {
    const got = { next: null, events: 3, archived_at: CLOCK, seq: 2 };
    expect(freezeClock(got)).toEqual(got);
  });
  // The emitter PANICS here: freezing a field that has stopped being stamped
  // would hide it.
  it("refuses a created_at or a cursor that is not a clock, as freeze_clock panics", () => {
    expect(() => freezeClock({ created_at: "1790300000" })).toThrow("created_at is not a unix timestamp");
    expect(() => freezeClock({ created_at: -1 })).toThrow("created_at is not a unix timestamp");
    expect(() => freezeClock({ games: [{ created_at: 5 }] })).toThrow(
      "created_at reads 5, which is not a clock — freezing it would hide a field that has stopped being stamped",
    );
    expect(() => freezeClock({ next: "wire" })).toThrow("a page cursor is <created_at>,<id>");
    expect(() => freezeClock({ next: "soon,wire" })).toThrow("a page cursor's first half is a unix timestamp");
    expect(() => freezeClock({ next: "5,wire" })).toThrow("a page cursor reads 5, which is not a clock");
  });
});

describe("filterLobby", () => {
  /** Another person's table on the shared lobby: a committed row under an id
   *  the replay never creates. */
  const foreign = (page) => ({ ...page.games[0], id: "sim-00000000000000ff" });

  it("drops a foreign row, and keeps the committed null next where the live lobby has more pages", () => {
    const page = committed("games_open");
    expect(page.next, "the premise: the committed page is the last").toBeNull();
    const got = { ...page, games: [...page.games, foreign(page)], next: `${FROZEN_CREATED_AT},sim-00000000000000ff` };
    expect(filterLobby("games_open", page, got, IDS)).toEqual(page);
  });
  // `games_next.json`'s cursor names `wire`, one of the replay's own, so the
  // live cursor is COMPARED rather than taken back.
  it("compares games_next.json's cursor, which names one of the replay's tables", () => {
    const page = committed("games_next");
    expect(page.next, "the premise: the committed cursor names wire").toBe(`${FROZEN_CREATED_AT},wire`);
    const got = { ...page, games: [...page.games, foreign(page)] };
    expect(filterLobby("games_next", page, got, IDS)).toEqual(page);
    const wrong = { ...page, next: `${FROZEN_CREATED_AT},wire2` };
    expect(filterLobby("games_next", page, wrong, IDS).next, "a cursor at the wrong table is not taken back").toBe(
      `${FROZEN_CREATED_AT},wire2`,
    );
  });
  it("refuses a name that is not a lobby page, and a reply with no games list", () => {
    expect(() => filterLobby("table", committed("table"), committed("table"), IDS)).toThrow("table is not a lobby page");
    expect(() => filterLobby("games", committed("games"), committed("error"), IDS)).toThrow("games: the reply carries no games list");
  });
});

describe("normalizeHouseChat", () => {
  it("takes the post's seq back by the base: base + 1 reads 1", () => {
    const post = committed("house_chat_post");
    expect(post.seq, "the premise").toBe(1);
    expect(normalizeHouseChat("house_chat_post", { ...post, seq: BASE + 1 }, BASE, BASE_REV)).toEqual(post);
  });
  // **The anti-vacuity partner of the foreign case below** (the orchestrator's
  // seq-only ruling): this run's OWN message above the base is kept, an older
  // one below it is dropped, and the run HOLDS — the normalized page is the
  // committed one, whole.
  it("takes the page's last and every seq back, drops an earlier run's message, keeps its own, and holds", () => {
    const page = committed("house_chat");
    const ours = page.messages[0];
    const got = { last: BASE + 1, rev: BASE_REV + 1, messages: [{ ...ours, seq: BASE - 3 }, { ...ours, seq: BASE + 1 }] };
    const normalized = normalizeHouseChat("house_chat", got, BASE, BASE_REV);
    expect(normalized.messages, "only this run's own message, at seq 1").toEqual(page.messages);
    expect([normalized.last, normalized.rev], "last by the base, rev by the base revision").toEqual([1, 1]);
    expect(normalized, "the run holds").toEqual(page);
  });
  // M6.5c2 cycle D (R-D4): the revision counts every message and every delete
  // anybody made on the shared house chat, so it is taken back by the
  // revision read at the start — never by `last`'s base, which the house
  // chat's deletes leave behind it. A delete by someone else during the run
  // moves it, and the run differs. Mutant: `rev` taken back by `base`.
  it("takes the page's rev back by the base revision, and a delete by someone else during the run differs", () => {
    const page = committed("house_chat");
    const ours = { ...page.messages[0], seq: BASE + 1 };
    expect(normalizeHouseChat("house_chat", { last: BASE + 1, rev: BASE_REV + 1, messages: [ours] }, BASE, BASE_REV)).toEqual(page);
    const theirs = normalizeHouseChat("house_chat", { last: BASE + 1, rev: BASE_REV + 2, messages: [ours] }, BASE, BASE_REV);
    expect(theirs.rev, "another person's delete moved it").toBe(2);
    expect(theirs).not.toEqual(page);
  });
  // **A message after the base that is not this run's is KEPT, and the run
  // differs twice** (M6.5c2 cycle D, R-6.5c2.35). The filter used to drop it by
  // its author's address, which no author carries any more; nothing else can
  // say whose it is (a name is anyone's to choose). The verdict never rested on
  // the drop: `last` is taken back and NEVER recomputed from what was kept, so
  // a person talking in the lobby during a run moved it then and fails the run
  // now — and the page itself no longer matches either.
  it("keeps a message after the base that is not this run's, so the run differs by last and by the page", () => {
    const page = committed("house_chat");
    const ours = page.messages[0];
    const theirs = { ...ours, author: { ...ours.author, name: "Someone Else" }, seq: BASE + 2 };
    const normalized = normalizeHouseChat(
      "house_chat",
      { last: BASE + 2, rev: BASE_REV + 2, messages: [{ ...ours, seq: BASE + 1 }, theirs] },
      BASE,
      BASE_REV,
    );
    expect(normalized.last, "last is not the committed page's").toBe(2);
    expect(normalized.last).not.toBe(page.last);
    expect(normalized.messages).toHaveLength(2);
    expect(normalized.messages).not.toEqual(page.messages);
  });
  // On a SECOND run the base IS the first run's own post — `seq > base` is
  // what drops it. Since M6.5c2 cycle D that post is a TOMBSTONE: the first
  // run deleted it before it ended.
  it("drops the replay's own message AT the base: the previous run's post, deleted", () => {
    const page = committed("house_chat");
    const ours = page.messages[0];
    const previous = { ...ours, seq: BASE, text: "", deleted: true };
    const got = { last: BASE + 1, rev: BASE_REV + 1, messages: [previous, { ...ours, seq: BASE + 1 }] };
    expect(normalizeHouseChat("house_chat", got, BASE, BASE_REV)).toEqual(page);
  });
  it("refuses a name that is not the house chat, and a seq, last or rev that is not a number", () => {
    expect(() => normalizeHouseChat("chat", committed("chat"), BASE, BASE_REV)).toThrow("chat is not a house chat payload");
    expect(() => normalizeHouseChat("house_chat_post", committed("error"), BASE, BASE_REV)).toThrow(
      "house_chat_post: the reply's seq is not a number",
    );
    expect(() => normalizeHouseChat("house_chat", committed("error"), BASE, BASE_REV)).toThrow(
      "house_chat: the reply's last is not a number",
    );
    expect(() => normalizeHouseChat("house_chat", { last: BASE, messages: [] }, BASE, BASE_REV)).toThrow(
      "house_chat: the reply's rev is not a number",
    );
  });
});

// M6.5c2 cycle D (R-D4): the walk's house post is deleted by the run that made
// it, by the number that run's post was really given. Mutants: the path left
// as the walk has it (the first run's delete names a stranger's message, or
// nothing); the shift ignored; a row that is not the house delete rewritten.
describe("houseDeleteRow", () => {
  const house = SCRIPTS.walk.find((row) => row.fixture === "house_chat_delete");
  it("carries the house delete to the run's own post, and leaves every other row alone", () => {
    expect(house, "premise: the walk deletes its house post").toBeDefined();
    expect(house.path, "premise: the corpus's post was the house's first").toBe("/api/chat/1");
    expect(houseDeleteRow(house, BASE)).toEqual({ ...house, path: `/api/chat/${BASE + 1}` });
    expect(houseDeleteRow(house, 0), "a shift of nothing").toEqual(house);
    for (const row of SCRIPTS.walk.filter((r) => r !== house)) expect(houseDeleteRow(row, BASE)).toBe(row);
  });
  it("refuses the delete before the run's post has been answered", () => {
    expect(() => houseDeleteRow(house, undefined)).toThrow("the run's own house post was not answered");
  });
});

describe("firstSendAccepts", () => {
  it("accepts the committed 201, the first send landing", () => {
    for (const name of ["archive_post", "archive_post_abandoned"]) {
      expect(firstSendAccepts(committed(name), 201, committed(name)), name).toBe("landed");
    }
  });
  // The once-rule's answer to `archive_post` IS committed:
  // `archive_post_identical.json`.
  it("accepts the once-rule's 200: the same body with identical true", () => {
    expect(firstSendAccepts(committed("archive_post"), 200, committed("archive_post_identical"))).toBe("identical");
    const abandoned = committed("archive_post_abandoned");
    expect(firstSendAccepts(abandoned, 200, { ...abandoned, identical: true })).toBe("identical");
  });
  it("refuses a 200 without identical true, a 409, and a body that differs anywhere else", () => {
    const post = committed("archive_post");
    expect(firstSendAccepts(post, 200, post), "a 200 still reading identical: false").toBeNull();
    expect(firstSendAccepts(post, 409, committed("archive_409"))).toBeNull();
    expect(firstSendAccepts(post, 201, committed("archive_post_identical")), "a 201 saying identical").toBeNull();
    expect(firstSendAccepts(post, 200, committed("archive_post_abandoned")), "another leaf").toBeNull();
    expect(firstSendAccepts(post, 201, { ...post, path: "browser/other/rev7/over/wire.json" })).toBeNull();
    // The once-rule's 200 is the SAME body but `identical`: a save filed at
    // another path is no repeat of this one.
    expect(firstSendAccepts(post, 200, { ...committed("archive_post_identical"), path: "browser/other/rev7/over/wire.json" })).toBeNull();
  });
});

// R3's M-1: the replay's two first sends leave their saves in the bucket for
// good (the owner's direction), so a `wire` record whose bytes change — a
// version bump, a walk that applies other moves — meets the earlier save as a
// 409 on every later run. The recipe names the four keys to delete.
describe("resetRecipe", () => {
  it("names the four keys the two first sends leave, under test/, and a delete for each", () => {
    const recipe = resetRecipe(CORPUS, "red@wire.test");
    expect(recipe.keys).toEqual([
      "test/browser/red_at_wire.test/rev7/over/wire.json",
      "test/browser/red_at_wire.test/rev7/over/wire.table.json",
      "test/browser/red_at_wire.test/rev7/abandoned/wire.json",
      "test/browser/red_at_wire.test/rev7/abandoned/wire.table.json",
    ]);
    expect(recipe.commands).toEqual(recipe.keys.map((key) => `npx wrangler r2 object delete "yokohama-archive/${key}" --remote`));
  });
  // The root is spec §2.6's: `test/` for a sender whose address ends `.test`
  // with its dot, and the bucket's top otherwise. Mutant: the root read off
  // anything but the sender's full address.
  it("keys a sender outside .test at the bucket's top", () => {
    expect(resetRecipe(CORPUS, "red@wire.contest").keys[0]).toBe("browser/red_at_wire.test/rev7/over/wire.json");
  });
  // The module's own rule (`receipt.rs`'s `is_test_identity`): a test identity
  // is an EMAIL in `.test`, so a plain name that ends `.test` — a legal name
  // under the open door — is keyed at the bucket's top, as the module keys its
  // save. Mutant: the old rule, `who.endsWith(".test")` alone.
  it("keys a plain name that ends .test at the bucket's top: no @, no test identity", () => {
    expect(resetRecipe(CORPUS, "wire.test").keys[0]).toBe("browser/red_at_wire.test/rev7/over/wire.json");
  });
});

describe("readCredential", () => {
  let dir;
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "live-replay-cred-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));
  const scratch = (text) => {
    const file = join(dir, ".dev.vars");
    writeFileSync(file, text);
    return file;
  };

  it("reads the two keys among the file's others, CRLF or not", () => {
    const file = scratch("R2_ACCESS_KEY_ID=other\r\nCF-Access-Client-Id=dummy-id.access\r\n\r\nCF-Access-Client-Secret=dummy-secret-value\r\n");
    expect(readCredential(file)).toEqual({ id: "dummy-id.access", secret: "dummy-secret-value" });
  });
  it("refuses an absent file, naming the file and both keys", () => {
    const file = join(dir, "nothing-here");
    expect(() => readCredential(file)).toThrow(file);
    expect(() => readCredential(file)).toThrow(/CF-Access-Client-Id.*CF-Access-Client-Secret/);
  });
  it("refuses a missing or empty key, naming the file and the key and never a value", () => {
    const missing = scratch("CF-Access-Client-Id=dummy-id.access\n");
    expect(() => readCredential(missing)).toThrow(`${missing} holds no CF-Access-Client-Secret= line`);
    let said = "";
    try {
      readCredential(scratch("CF-Access-Client-Id=\nCF-Access-Client-Secret=dummy-secret-value\n"));
    } catch (e) {
      said = String(e);
    }
    expect(said, "an empty value is absent").toContain("holds no CF-Access-Client-Id= line");
    expect(said, "the other key's value is never in the sentence").not.toContain("dummy-secret-value");
  });
});

describe("order and urlOf", () => {
  it("runs each host row after the walk row it names, and the archive rows last", () => {
    const walk = [{ n: "w0" }, { n: "w1" }, { n: "w2" }];
    const host = [{ n: "h1", after: 1 }, { n: "h-", after: -1 }];
    const archive = [{ n: "a0" }];
    expect(order(walk, host, archive).map(({ script, index, row }) => `${script}:${index}:${row.n}`)).toEqual([
      "walk-host.json:1:h-",
      "walk.json:0:w0",
      "walk.json:1:w1",
      "walk-host.json:0:h1",
      "walk.json:2:w2",
      "archive-walk.json:0:a0",
    ]);
    expect(() => order(walk, [{ n: "h7", after: 7 }], archive)).toThrow("a host row names no walk row to follow");
  });
  it("addresses every row at the Worker's origin, and refuses one that would leave it", () => {
    expect(urlOf({ path: "/api/games", query: "scope=open" })).toBe(`${ORIGIN}/api/games?scope=open`);
    expect(() => urlOf({ path: "//elsewhere.example/api/games", query: "" })).toThrow("leaves");
  });
});

// ---- the driver, against a fake host ---------------------------------------

/** `value` as the live Worker's clock stamps it: every frozen reading a real
 *  one, which `freezeClock` must take back. */
function unfreeze(value) {
  if (Array.isArray(value)) return value.map(unfreeze);
  if (value === null || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value).map(([key, v]) => {
      if (key === "created_at" && v === FROZEN_CREATED_AT) return [key, CLOCK];
      if (key === "next" && typeof v === "string") return [key, v.replace(String(FROZEN_CREATED_AT), String(CLOCK))];
      return [key, unfreeze(v)];
    }),
  );
}

/** What the Worker answers a named row with, derived from its committed file
 *  exactly where the Worker is entitled to differ (spec §5.3's live half). */
function liveBody(fixture) {
  const body = unfreeze(committed(fixture));
  if (fixture === "health") return { ...body, identity: "cf-access", signal: "ws", archive: "r2" };
  if (fixture === "me" || fixture === "me_signed_out") return { ...body, mode: "cf-access" };
  if (fixture.startsWith("games")) {
    // Somebody else's table, older than the replay's, and a cursor that runs
    // on past the committed page's end.
    const other = { ...body.games[0], id: "sim-00000000000000ff" };
    return { ...body, games: [...body.games, other], next: body.next ?? `${CLOCK},sim-00000000000000ff` };
  }
  if (fixture === "house_chat_post") return { ...body, seq: body.seq + BASE };
  if (fixture === "house_chat") {
    const earlier = { ...body.messages[0], seq: BASE - 1 };
    return {
      last: body.last + BASE,
      rev: body.rev + BASE_REV,
      messages: [earlier, ...body.messages.map((m) => ({ ...m, seq: m.seq + BASE }))],
    };
  }
  if (fixture === "archive_post_abandoned") return { ...body, identical: true };
  return body;
}

const json = (status, body) =>
  new Response(status === 204 ? null : JSON.stringify(body), { status, headers: { "content-type": "application/json" } });

/**
 * A fake Worker. Forces answer `forceStatus`; every other request takes the
 * next answer in `plan`, which is built in the replay's own order: the house
 * chat's base read, then one answer per row. `edit(plan)` perturbs it.
 */
function fakeHost({ forceStatus = 204, edit = () => {} } = {}) {
  const rows = order(SCRIPTS.walk, SCRIPTS.host, SCRIPTS.archive);
  const houseChat = committed("house_chat");
  const plan = [() => json(200, { ...houseChat, last: BASE, rev: BASE_REV, messages: [] })];
  for (const { row } of rows) {
    const status = row.fixture === "archive_post_abandoned" ? 200 : row.status;
    plan.push(() => json(status, row.fixture === null ? {} : liveBody(row.fixture)));
  }
  edit(plan, rows);
  const requests = [];
  const fetcher = async (url, init) => {
    requests.push({ url, method: init.method, headers: { ...init.headers }, body: init.body, redirect: init.redirect });
    if (/\/api\/admin\/tables\/[^/]+\/expire\?force$/.test(url)) {
      return forceStatus === 204 ? json(204, null) : json(forceStatus, committed("not_seated_403"));
    }
    const next = plan.shift();
    if (!next) throw new Error(`the fake host has no answer planned for ${init.method} ${url}`);
    return next();
  };
  return { fetcher, requests, rows };
}

/** Runs the replay against `host`, its report captured. */
async function run(host) {
  const lines = [];
  const result = await replay({
    ...SCRIPTS,
    corpus: CORPUS,
    credential: { id: "dummy-id.access", secret: "dummy-secret-value" },
    fetcher: host.fetcher,
    log: (line) => lines.push(line),
    pause: async () => {},
  });
  return { result, text: lines.join("\n") };
}

/** Where row `n` of `walk.json` sits in the plan: after the base read, the
 *  art row, and each host row that precedes it. */
const planIndex = (rows, script, n) => 1 + rows.findIndex((r) => r.script === script && r.index === n);

describe("replay", () => {
  it("holds against a Worker that differs only where it may, and names every exemption it spent", async () => {
    const host = fakeHost();
    const { result, text } = await run(host);
    expect(result.mismatches).toEqual([]);
    expect(result.holds).toBe(true);
    expect(result.run, "every row of the three scripts ran").toBe(SCRIPTS.walk.length + SCRIPTS.host.length + SCRIPTS.archive.length);
    expect(result.named, "every named row was compared").toBe(
      [...SCRIPTS.walk, ...SCRIPTS.host, ...SCRIPTS.archive].filter((row) => row.fixture !== null).length,
    );
    expect(result.spent.map((s) => s.split(":")[0]).sort()).toEqual(
      [
        "HOST_FIELDS health.archive",
        "HOST_FIELDS health.identity",
        "HOST_FIELDS health.signal",
        "HOST_FIELDS me.mode",
        "HOST_FIELDS me_signed_out.mode",
        "filterLobby games",
        "filterLobby games_next",
        "filterLobby games_open",
        "filterLobby games_playing",
        "firstSendAccepts archive_post",
        "firstSendAccepts archive_post_abandoned",
        "normalizeHouseChat house_chat",
        "normalizeHouseChat house_chat_post",
        "houseDeleteRow house_chat_delete",
      ].sort(),
    );
    // The run's own post is the one it deletes (R-D4): the fake's post was
    // given the committed number plus the base.
    const deleted = host.requests.filter((r) => r.method === "DELETE" && r.url.startsWith(`${ORIGIN}/api/chat/`));
    expect(deleted.map((r) => r.url)).toEqual([`${ORIGIN}/api/chat/${BASE + 1}`]);
    expect(result.forcedBefore.map((f) => `${f.id} ${f.status}`)).toEqual(IDS.map((id) => `${id} 204`));
    expect(result.forcedAfter.map((f) => `${f.id} ${f.status}`)).toEqual(IDS.map((id) => `${id} 204`));
    expect(text).toContain("live-replay: HOLDS");
    expect(text).toContain("the house chat's base: last 4321, rev 9876, read as blue@wire.test");
  });

  it("sends the credential to the Worker's origin alone, the forces bare, and a claimed header only for a row's who", async () => {
    const host = fakeHost();
    await run(host);
    const forces = host.requests.filter((r) => r.url.includes("/api/admin/"));
    expect(forces.map((r) => `${r.method} ${r.url}`)).toEqual(
      [...IDS, ...IDS].map((id) => `POST ${ORIGIN}/api/admin/tables/${id}/expire?force`),
    );
    for (const force of forces) {
      expect(Object.keys(force.headers), "a force is the owner's own assertion, over WARP").toEqual([]);
    }
    const others = host.requests.filter((r) => !r.url.includes("/api/admin/"));
    expect(others.length).toBeGreaterThan(2000);
    for (const r of others) {
      expect(new URL(r.url).origin).toBe(ORIGIN);
      expect(r.headers["CF-Access-Client-Id"]).toBe("dummy-id.access");
      expect(r.headers["CF-Access-Client-Secret"]).toBe("dummy-secret-value");
      expect(r.redirect, "a redirect is answered, never followed").toBe("manual");
    }
    // The base read, then each row: its claimed header is its `who`, and a row
    // with no `who` sends none.
    const [base, ...sent] = others;
    expect(base.headers["X-Yokohama-Claimed-User"]).toBe("blue@wire.test");
    expect(sent).toHaveLength(host.rows.length);
    host.rows.forEach(({ row }, i) => {
      expect(sent[i].headers["X-Yokohama-Claimed-User"], `${row.method} ${row.path}`).toBe(row.who ?? undefined);
    });
    expect(host.rows.some(({ row }) => row.who === null), "the premise: a row with no who exists").toBe(true);
  });

  it("refuses to start unless all three forces answer 204, and sends no row", async () => {
    const host = fakeHost({ forceStatus: 403 });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.run).toBe(0);
    expect(host.requests.every((r) => r.url.includes("/api/admin/"))).toBe(true);
    expect(text).toContain("wire2 403");
    expect(text).toContain("live-replay: DOES NOT HOLD");
  });

  it("reports a field that differs by its fixture and path, runs on, and forces the ids again", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 27);
        expect(rows[i - 1].row.fixture, "the premise").toBe("options");
        const body = liveBody("options");
        plan[i] = () => json(200, { ...body, over: !body.over });
      },
    });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path}`)).toEqual(["options over"]);
    expect(result.run, "a body that differs changes no state, so the replay runs on").toBe(host.rows.length);
    expect(result.forcedAfter).toHaveLength(3);
    expect(text).toContain("walk.json row 27: GET /api/games/wire as red@wire.test — options over: committed false · got true");
  });

  it("stops at a status that differs, forces the ids again, and never prints a request body", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        plan[planIndex(rows, "walk.json", 9)] = () => json(403, committed("not_seated_403"));
      },
    });
    const { result, text } = await run(host);
    // The last row sent is the one it stopped at.
    const stoppedAt = host.requests.filter((r) => !r.url.includes("/api/admin/")).at(-1);
    expect(`${stoppedAt?.method} ${stoppedAt?.url}`).toBe(`POST ${ORIGIN}/api/games/wire/join`);
    expect(stoppedAt?.body, "the premise: the stopped row's request carried the password").toContain("sesame");
    expect(result.holds).toBe(false);
    expect(result.run, "the rows after it would read a state it did not make").toBe(planIndex(host.rows, "walk.json", 9));
    expect(result.forcedAfter.map((f) => f.status)).toEqual([204, 204, 204]);
    expect(text).toContain("walk.json row 9: POST /api/games/wire/join as blue@wire.test — status: committed 200 · got 403");
    expect(text).toContain("you are not seated at this table");
    expect(text).not.toContain("sesame");
  });

  // R3's M-1: a first send's 409 is the bucket holding an EARLIER run's save
  // of a `wire` record that has since changed. The run stops there — every
  // row after it reads the save it did not make — names the cause, prints the
  // reset recipe after the forces, and exits non-zero. Mutants: the run going
  // on; no recipe; the recipe's keys at the bucket's top.
  it("stops at a first send's 409, names the cause, and prints the reset recipe", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "archive-walk.json", 0);
        expect(rows[i - 1].row.fixture, "the premise").toBe("archive_post");
        plan[i] = () => json(409, committed("archive_409"));
      },
    });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.run, "no row after it").toBe(planIndex(host.rows, "archive-walk.json", 0));
    expect(result.stopped).toBe(
      "archive_post answered 409 — it said: a different record is archived under this id. The bucket holds an earlier " +
        "run's save of a wire record that has since changed (a version bump, a walk that applies other moves, a record " +
        "format change): delete the replay's four saves, then run it again",
    );
    expect(result.forcedAfter.map((f) => f.status)).toEqual([204, 204, 204]);
    const lines = text.split("\n");
    const verdict = lines.indexOf("live-replay: DOES NOT HOLD");
    for (const command of resetRecipe(CORPUS, "red@wire.test").commands) {
      const at = lines.indexOf(`  ${command}`);
      expect(at, command).toBeGreaterThan(lines.findIndex((l) => l.startsWith("force after:")));
      expect(at, "before the verdict").toBeLessThan(verdict);
    }
  });

  it("refuses an exemption the Worker did not need: a host field reading as the axum host's", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        plan[planIndex(rows, "walk.json", 28)] = () => json(200, { ...liveBody("health"), identity: "open" });
      },
    });
    const { result } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path} ${m.reason}`)).toEqual([
      "health identity exempted, but both hosts agree on it",
    ]);
  });

  it("re-reads a lobby page the house has not yet received a copy for, and counts it", async () => {
    const stale = () => {
      const body = liveBody("games_open");
      return json(200, { ...body, games: body.games.filter((g) => g.id !== "wire") });
    };
    const once = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 4);
        plan.splice(i, 0, stale);
      },
    });
    const first = await run(once);
    expect(first.result.holds).toBe(true);
    expect(first.result.rereads).toBe(1);
    // The count spent, and the bound it was spent against.
    expect(first.text).toContain("lobby re-reads: 1 spent (the bound: 5 per lobby row, 500 ms apart");

    const never = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 4);
        plan.splice(i, 1, stale, stale, stale, stale, stale, stale);
      },
    });
    const second = await run(never);
    // A page that never converges within the bound is a MISMATCH.
    expect(second.result.holds).toBe(false);
    expect(second.result.rereads).toBe(5);
    expect(second.result.mismatches.map((m) => `${m.fixture} ${m.path}`)).toContain("games_open games.0.id");
    expect(second.text).toContain("lobby re-reads: 5 spent (the bound: 5 per lobby row, 500 ms apart");
    expect(second.text).toContain("live-replay: DOES NOT HOLD");
  });

  it("redacts a value that could be a seed or a secret when it reports one", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 18);
        expect(rows[i - 1].row.fixture, "the premise").toBe("table");
        plan[i] = () => json(200, { ...liveBody("table"), seed: "13377331133773311337" });
      },
    });
    const { result, text } = await run(host);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path}`)).toEqual(["table seed"]);
    expect(text).toContain("table seed: committed (absent) · got <redacted>");
    expect(text).not.toContain("13377331133773311337");
  });

  it("prints its verdict AFTER the second force when the base cannot be read", async () => {
    const host = fakeHost({ edit: (plan) => void (plan[0] = () => json(403, committed("not_seated_403"))) });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.run, "no row is sent without a base").toBe(0);
    expect(text).toContain("the house chat's base could not be read: 403 — it said: you are not seated at this table");
    const lines = text.split("\n");
    const forcedAfter = lines.findIndex((l) => l.startsWith("force after:"));
    expect(forcedAfter, "the ids are forced again").toBeGreaterThan(-1);
    expect(lines.findIndex((l) => l === "live-replay: DOES NOT HOLD"), "the verdict is the report's last word").toBeGreaterThan(forcedAfter);
  });

  it("refuses a row that would leave the Worker's origin before sending anything, the forces included", async () => {
    const host = fakeHost();
    const walk = [...SCRIPTS.walk, { ...SCRIPTS.walk[2], path: "//elsewhere.example/api/games/wire/table" }];
    await expect(
      replay({
        ...SCRIPTS,
        walk,
        corpus: CORPUS,
        credential: { id: "dummy-id.access", secret: "dummy-secret-value" },
        fetcher: host.fetcher,
        log: () => {},
        pause: async () => {},
      }),
    ).rejects.toThrow(`leaves ${ORIGIN}`);
    expect(host.requests).toEqual([]);
  });

  // An exemption is spent only on a value that DIFFERS: a host field the
  // Worker did not send is no exemption, and must not read as one.
  it("refuses a host field the Worker did not send, as a mismatch and not an exemption spent", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const { archive: _dropped, ...health } = liveBody("health");
        plan[planIndex(rows, "walk.json", 28)] = () => json(200, health);
      },
    });
    const { result } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path} ${m.reason}`)).toEqual([
      "health archive exempted, but this host sent no string for it",
    ]);
    expect(result.spent.filter((s) => s.startsWith("HOST_FIELDS health.archive"))).toEqual([]);
  });
});
