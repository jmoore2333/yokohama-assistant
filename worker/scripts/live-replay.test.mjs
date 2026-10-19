import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { order } from "./corpus-walk.mjs";
import { ORIGIN, readCredential, readIdentity, readSessionCookie, readSessionSecret } from "./dev-vars.mjs";
import { replay } from "./live-replay.mjs";
import {
  FROZEN_CREATED_AT,
  filterLobby,
  firstSendAccepts,
  freezeClock,
  houseDeleteRow,
  isProductionSeedCreate,
  normalizeHouseChat,
  productionSeedRule,
  resetRecipe,
  SEED_NOT_SOLO,
  skippedUnderProductionSeedRule,
} from "./replay/exemptions.mjs";
import { loadCorpus, loadScripts, urlOf } from "./replay/scripts.mjs";
import { send } from "./replay/send.mjs";

// **The live corpus replay's cells** (spec §5.3's live half; the plan's Task
// 15). Every reply body a cell compares is a COMMITTED payload, or one derived
// from it the way the live Worker differs from the axum host the corpus was cut
// from — a real clock, its own host fields, a lobby and a house chat other
// people share — or the production seed rule's refusal, in the layer's own
// sentence (`SEED_NOT_SOLO`, held to the Rust source below). Nothing here
// sends a request: the driver's cells hand it a fake host, and the
// credential's cells read a scratch file of dummy values.

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

// **The house rows' watcher tag on a production host** (security S2c, finding
// D8; the Task 4 review's I-1). A deployed Worker is `SoloOnly`, so the live
// house chat's first append after 0.6.3 draws an entropy salt and the run's own
// poster, red, reads a tag that is not the committed `7e52`. The judge takes it
// back only when it is four lowercase hex AND the page's tag is the one the
// post's reply carried, each take-back counted in `spent`; anything else stays
// a mismatch. Through the replay and its fake production host, as the driver's
// cells below are. Mutants: the take-back spent on an absent or `null` tag
// (M9c, cells 2 and 3); the reply/page agreement ignored (M9d, cell 4).
describe("houseWatcherTag", () => {
  /** `walk.json`'s two house rows: the run's own post, then the page after it. */
  const POST_ROW = 43;
  const PAGE_ROW = 44;
  /** The tag a fresh salt gave red in the review's measurement: four hex, not the committed one. */
  const PRODUCTION = "8196";
  const DROPPED = Symbol("dropped");
  const tagged = (author, tag) => {
    if (tag !== DROPPED) return { ...author, tag };
    const { tag: _dropped, ...rest } = author;
    return rest;
  };
  /** The fake production host with the post's reply and the page's own message carrying these tags. */
  const hostWith = (postTag, pageTag) =>
    fakeHost({
      edit: (plan, rows) => {
        const at = (n) => rows.find((r) => r.script === "walk.json" && r.index === n).row;
        expect([at(POST_ROW).fixture, at(PAGE_ROW).fixture], "the premise: the walk's house post and page").toEqual(["house_chat_post", "house_chat"]);
        const post = liveBody("house_chat_post");
        plan[planIndex(rows, "walk.json", POST_ROW)] = () => json(at(POST_ROW).status, { ...post, author: tagged(post.author, postTag) });
        const page = liveBody("house_chat");
        const messages = page.messages.map((m) => (m.seq > BASE ? { ...m, author: tagged(m.author, pageTag) } : m));
        plan[planIndex(rows, "walk.json", PAGE_ROW)] = () => json(at(PAGE_ROW).status, { ...page, messages });
      },
    });
  const tagSpends = (result) => result.spent.filter((s) => s.startsWith("houseWatcherTag"));
  const where = (result) => result.mismatches.map((m) => `${m.fixture} ${m.path}`);

  it("takes a production tag back to the committed one, counted in spent, when the reply and the page carry the same four hex", async () => {
    expect(committed("house_chat_post").author.tag, "the premise: the committed tag").toBe("7e52");
    expect(committed("house_chat").messages[0].author.tag, "and the page's").toBe("7e52");
    const { result, text } = await run(hostWith(PRODUCTION, PRODUCTION));
    expect(result.mismatches).toEqual([]);
    expect(result.holds).toBe(true);
    expect(tagSpends(result)).toEqual([
      'houseWatcherTag house_chat_post.author.tag: "7e52" → "8196"',
      'houseWatcherTag house_chat.messages.0.author.tag: "7e52" → "8196"',
    ]);
    expect(text).toContain("live-replay: HOLDS");
  });

  it("refuses an ABSENT tag as a mismatch, and spends nothing on it", async () => {
    const { result } = await run(hostWith(DROPPED, DROPPED));
    expect(result.holds).toBe(false);
    expect(where(result)).toEqual(["house_chat_post author.tag", "house_chat messages.0.author.tag"]);
    expect(tagSpends(result)).toEqual([]);
  });

  it("refuses a null tag as a mismatch, and spends nothing on it", async () => {
    const { result } = await run(hostWith(null, null));
    expect(result.holds).toBe(false);
    expect(where(result)).toEqual(["house_chat_post author.tag", "house_chat messages.0.author.tag"]);
    expect(tagSpends(result)).toEqual([]);
  });

  it("refuses a page whose tag is not the post's reply's: two production values, and a production reply beside the committed page", async () => {
    const other = await run(hostWith(PRODUCTION, "a1b2"));
    expect(other.result.holds).toBe(false);
    expect(where(other.result), "the reply's own tag is spent; the page is held to it").toEqual(["house_chat messages.0.author.tag"]);
    expect(other.result.mismatches[0].reason).toContain("the post's reply carried");
    expect(tagSpends(other.result)).toEqual(['houseWatcherTag house_chat_post.author.tag: "7e52" → "8196"']);
    const committedPage = await run(hostWith(PRODUCTION, "7e52"));
    expect(committedPage.result.holds).toBe(false);
    expect(where(committedPage.result), "a page reading as the corpus does not agree with a reply that did not").toEqual([
      "house_chat messages.0.author.tag",
    ]);
  });

  // A tag is `TAG_HEX` LOWERCASE hex and nothing else: a rule loosened to any
  // run of hex, or to either case, passed the four cells above (M9e).
  it("refuses a MALFORMED tag as a mismatch at both rows, and spends nothing on it: five hex, and four with an uppercase digit", async () => {
    for (const bad of ["81960", "819A"]) {
      const { result } = await run(hostWith(bad, bad));
      expect(result.holds, bad).toBe(false);
      expect(where(result), bad).toEqual(["house_chat_post author.tag", "house_chat messages.0.author.tag"]);
      expect(tagSpends(result), bad).toEqual([]);
    }
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

describe("productionSeedRule", () => {
  // The walk's three creates: `wire2` and `wire` seat an open seat beside a
  // seed, `wire4` seats its creator four times — the solo shape the rule honours.
  it("names the walk's seeded creates beside an open seat, and never the solo one", () => {
    expect([...productionSeedRule(SCRIPTS.walk)].sort()).toEqual(["wire", "wire2"]);
    const creates = SCRIPTS.walk.filter((r) => r.method === "POST" && r.path === "/api/games");
    expect(creates.map((r) => r.body.id).sort(), "the premise").toEqual(["wire", "wire2", "wire4"]);
  });
  it("names a seeded create beside another person too, and an unseeded one never", () => {
    const seeded = (seats) => [{ method: "POST", path: "/api/games", who: "red@wire.test", body: { id: "t", seed: "1", seats } }];
    expect([...productionSeedRule(seeded([{ user: "red@wire.test" }, { user: "blue@wire.test" }]))]).toEqual(["t"]);
    expect([...productionSeedRule(seeded([{ user: "red@wire.test" }, { bot: true }]))]).toEqual([]);
    expect([...productionSeedRule(seeded([{ user: "red@wire.test" }, { user: "red@wire.test" }]))]).toEqual([]);
    const unseeded = [{ method: "POST", path: "/api/games", who: "red@wire.test", body: { id: "t", seats: [{ user: "red@wire.test" }, { open: true }] } }];
    expect([...productionSeedRule(unseeded)]).toEqual([]);
    const nullSeed = [{ ...unseeded[0], body: { ...unseeded[0].body, seed: null } }];
    expect([...productionSeedRule(nullSeed)], "a null seed is none: a server deals that create unseeded").toEqual([]);
  });
  it("skips every row at a skipped table and, while it names one, every lobby page, and never the create itself or a kept table's row", () => {
    const skipped = new Set(["wire", "wire2"]);
    const at = (path, fixture = null, method = "GET") => ({ method, path, fixture, body: null, who: "red@wire.test" });
    expect(skippedUnderProductionSeedRule(at("/api/games/wire"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games/wire/input", null, "POST"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games/wire2/table"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games", "games_open"), skipped)).toBe(true);
    expect(skippedUnderProductionSeedRule(at("/api/games", "games_next"), skipped)).toBe(true);
    // A rule that names no table skips no lobby page: each is sent, and judged
    // through `filterLobby` with the re-reads. Mutant: the guard dropped.
    expect(skippedUnderProductionSeedRule(at("/api/games", "games_open"), new Set()), "a rule naming no table skips no lobby page").toBe(false);
    expect(skippedUnderProductionSeedRule(at("/api/games/wire4"), skipped)).toBe(false);
    expect(skippedUnderProductionSeedRule(at("/api/health", "health"), skipped)).toBe(false);
    expect(skippedUnderProductionSeedRule(at("/api/archive", "archive_post", "POST"), skipped)).toBe(false);
    const create = SCRIPTS.walk.find((r) => r.method === "POST" && r.path === "/api/games" && r.body.id === "wire");
    expect(skippedUnderProductionSeedRule(create, skipped), "the create is SENT, and held to its 400").toBe(false);
    expect(isProductionSeedCreate(create, skipped)).toBe(true);
    expect(isProductionSeedCreate(at("/api/games/wire"), skipped)).toBe(false);
  });
  // **The rows a production-door run sends, pinned** (security S1b): the fake
  // host and the driver's first cell both plan with the functions under test,
  // so a rule that skipped one row more passed every other cell. Computed here
  // over the committed scripts with the three functions, in the pre-flight's
  // line shape, and compared line by line to the list the review read
  // (`replay/fixtures/production-door-kept-rows.txt`, its header and footer
  // kept). Mutant: the skip rule widened to `wire4`'s four keep rows.
  it("keeps exactly the twenty-six rows the review read, of 2,213", () => {
    const skipped = productionSeedRule(SCRIPTS.walk);
    const rows = order(SCRIPTS.walk, SCRIPTS.host, SCRIPTS.archive);
    const kept = rows.filter(({ row }) => !skippedUnderProductionSeedRule(row, skipped));
    const lines = [
      `skipped tables: ${[...skipped].sort().join(", ")}; rows ${rows.length} (walk ${SCRIPTS.walk.length}, host ${SCRIPTS.host.length}, archive ${SCRIPTS.archive.length})`,
      ...kept.map(({ script, index, row }) => {
        const held = isProductionSeedCreate(row, skipped) ? "  <- the create, held to its 400" : "";
        const path = `${row.path}${row.query ? `?${row.query}` : ""}`;
        return `${script} row ${index}: ${row.method} ${path} as ${row.who ?? "no one"} -> ${row.status} ${row.fixture ?? "(unnamed)"}${held}`;
      }),
      `kept ${kept.length}, skipped ${rows.length - kept.length}, total ${rows.length}`,
    ];
    const file = readFileSync(new URL("./replay/fixtures/production-door-kept-rows.txt", import.meta.url), "utf8");
    expect(file.endsWith("\n"), "the list ends in one newline").toBe(true);
    expect(lines).toEqual(file.slice(0, -1).split("\n"));
    expect([kept.length, rows.length - kept.length, rows.length]).toEqual([26, 2187, 2213]);
  });
  // The sentence is `tables::SEED_NOT_SOLO`'s, read off the Rust source so
  // the two cannot drift (the edge rule's drift cell is the precedent).
  it("holds SEED_NOT_SOLO to the layer's own constant", () => {
    const source = readFileSync(new URL("../../crates/yokohama-web/src/tables.rs", import.meta.url), "utf8");
    const m = /pub const SEED_NOT_SOLO: &str =\s*"([^"]+)";/.exec(source);
    expect(m, "the constant is where the plan read it").not.toBeNull();
    expect(SEED_NOT_SOLO).toBe(m[1]);
  });
});

// R3's M-1: the replay's two first sends leave their saves in the bucket for
// good (the owner's direction), so a `wire` record whose bytes change — a
// version bump, a walk that applies other moves — meets the earlier save as a
// 409 on every later run. The recipe names the four keys to delete.
describe("resetRecipe", () => {
  // The walk caller's segment, read off the committed reply rather than
  // spelled: since security S1c (D26) it ends in a digest of the address.
  const SEGMENT = committed("archive_post").path.split("/")[1];
  it("names the four keys the two first sends leave, under test/, and a delete for each", () => {
    expect(SEGMENT, "premise: the readable half, then a twelve-character digest").toMatch(/^red_at_wire\.test-[0-9a-f]{12}$/);
    const recipe = resetRecipe(CORPUS, "red@wire.test");
    expect(recipe.keys).toEqual([
      `test/browser/${SEGMENT}/rev7/over/wire.json`,
      `test/browser/${SEGMENT}/rev7/over/wire.table.json`,
      `test/browser/${SEGMENT}/rev7/abandoned/wire.json`,
      `test/browser/${SEGMENT}/rev7/abandoned/wire.table.json`,
    ]);
    expect(recipe.commands).toEqual(recipe.keys.map((key) => `npx wrangler r2 object delete "yokohama-archive/${key}" --remote`));
  });
  // The root is spec §2.6's: `test/` for a sender whose address ends `.test`
  // with its dot, and the bucket's top otherwise. Mutant: the root read off
  // anything but the sender's full address.
  it("keys a sender outside .test at the bucket's top", () => {
    expect(resetRecipe(CORPUS, "red@wire.contest").keys[0]).toBe(`browser/${SEGMENT}/rev7/over/wire.json`);
  });
  // The module's own rule (`receipt.rs`'s `is_test_identity`): a test identity
  // is an EMAIL in `.test`, so a plain name that ends `.test` — a legal name
  // under the open door — is keyed at the bucket's top, as the module keys its
  // save. Mutant: the old rule, `who.endsWith(".test")` alone.
  it("keys a plain name that ends .test at the bucket's top: no @, no test identity", () => {
    expect(resetRecipe(CORPUS, "wire.test").keys[0]).toBe(`browser/${SEGMENT}/rev7/over/wire.json`);
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

describe("readSessionCookie and readIdentity", () => {
  const COOKIE = "__Host-yokohama_session=v1.eyJhZGRyZXNzIjoicmVkQHQudGVzdCJ9.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
  let dir;
  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), "live-replay-identity-"));
  });
  afterEach(() => rmSync(dir, { recursive: true, force: true }));
  const creds = () => {
    const file = join(dir, ".dev.vars");
    writeFileSync(file, "CF-Access-Client-Id=dummy-id.access\nCF-Access-Client-Secret=dummy-secret-value\n");
    return file;
  };

  it("is null for an unset or empty variable, and the minted line verbatim otherwise", () => {
    expect(readSessionCookie({})).toBeNull();
    expect(readSessionCookie({ YOKOHAMA_SESSION_COOKIE: "" })).toBeNull();
    expect(readSessionCookie({ YOKOHAMA_SESSION_COOKIE: COOKIE })).toBe(COOKIE);
  });
  it("refuses a value that is not a minted session line, naming the variable and never the value", () => {
    for (const bad of ["red@t.test", "yokohama_session=v1.a.b", `${COOKIE}\r\nX-Evil: 1`, `${COOKIE}; other=1`, "__Host-yokohama_session=v2.a.b"]) {
      let said = "";
      try {
        readSessionCookie({ YOKOHAMA_SESSION_COOKIE: bad });
      } catch (e) {
        said = String(e);
      }
      expect(said, JSON.stringify(bad)).toContain("YOKOHAMA_SESSION_COOKIE");
      expect(said, "no value is ever in the sentence").not.toContain("Evil");
      expect(said).not.toContain("red@t.test");
    }
  });
  // **The `.test` fence applies to a cookie handed in** (review I-1): a copied browser cookie for a real address would run
  // the instruments as that person. The payload is base64url JSON, read without the secret.
  const lineFor = (address) =>
    `__Host-yokohama_session=v1.${Buffer.from(JSON.stringify({ address, sid: null, issued: 1, expires: 2 }), "utf8").toString("base64url")}.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`;
  it("refuses a cookie for an address that is not a .test one, naming the variable and the address and the fence", () => {
    for (const address of ["jamoore@gmail.com", "RED@T.TEST", "red@t.test "]) {
      expect(() => readSessionCookie({ YOKOHAMA_SESSION_COOKIE: lineFor(address) }), address).toThrow(
        `YOKOHAMA_SESSION_COOKIE: mint-session: only a .test address may be minted — got ${address}`,
      );
      // The same through the one reader both instruments call.
      expect(() => readIdentity({ YOKOHAMA_SESSION_COOKIE: lineFor(address) }, join(dir, "nothing-here")), address).toThrow("YOKOHAMA_SESSION_COOKIE");
    }
    expect(readSessionCookie({ YOKOHAMA_SESSION_COOKIE: lineFor("red@t.test") })).toBe(lineFor("red@t.test"));
  });
  it("refuses a cookie whose payload is not JSON with an address, as no minted line", () => {
    const line = (json) => `__Host-yokohama_session=v1.${Buffer.from(json, "utf8").toString("base64url")}.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA`;
    for (const bad of ["not json", "[]", "{}", '{"address":5}', "null"]) {
      expect(() => readSessionCookie({ YOKOHAMA_SESSION_COOKIE: line(bad) }), bad).toThrow("is not a minted session cookie line");
    }
  });
  it("with no cookie the credential is required, as ever", () => {
    expect(readIdentity({}, creds())).toEqual({ secret: null, cookie: null, credential: { id: "dummy-id.access", secret: "dummy-secret-value" } });
    expect(() => readIdentity({}, join(dir, "nothing-here"))).toThrow(/credential file/);
  });
  it("with a cookie the credential rides too when the file is there, and is null when the file is absent", () => {
    const env = { YOKOHAMA_SESSION_COOKIE: COOKIE };
    expect(readIdentity(env, creds())).toEqual({ secret: null, cookie: COOKIE, credential: { id: "dummy-id.access", secret: "dummy-secret-value" } });
    expect(readIdentity(env, join(dir, "nothing-here"))).toEqual({ secret: null, cookie: COOKIE, credential: null });
  });
  // **The secret** (ruling R-50): the instrument mints a session per identity, and the secret wins over the one cookie.
  it("reads the session secret from the environment, null when unset or empty", () => {
    expect(readSessionSecret({})).toBeNull();
    expect(readSessionSecret({ YOKOHAMA_SESSION_SECRET: "" })).toBeNull();
    expect(readSessionSecret({ YOKOHAMA_SESSION_SECRET: "fixture-secret-0123456789abcdef0123" })).toBe("fixture-secret-0123456789abcdef0123");
  });
  it("with the secret the cookie is not read at all (it wins, even over a malformed one), and the credential is optional", () => {
    const env = { YOKOHAMA_SESSION_SECRET: "fixture-secret-0123456789abcdef0123", YOKOHAMA_SESSION_COOKIE: COOKIE };
    expect(readIdentity(env, join(dir, "nothing-here"))).toEqual({ secret: "fixture-secret-0123456789abcdef0123", cookie: null, credential: null });
    expect(readIdentity(env, creds())).toEqual({
      secret: "fixture-secret-0123456789abcdef0123",
      cookie: null,
      credential: { id: "dummy-id.access", secret: "dummy-secret-value" },
    });
    expect(readIdentity({ ...env, YOKOHAMA_SESSION_COOKIE: "not a cookie" }, join(dir, "nothing-here")).cookie).toBeNull();
  });
  it("with a cookie, a file that is there and malformed is still refused, not read as absent", () => {
    const file = join(dir, ".dev.vars");
    writeFileSync(file, "CF-Access-Client-Id=dummy-id.access\n");
    expect(() => readIdentity({ YOKOHAMA_SESSION_COOKIE: COOKIE }, file)).toThrow("holds no CF-Access-Client-Secret= line");
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
  if (fixture === "health") return { ...body, identity: "cf-access", signal: "ws", archive: "r2", table_ttl_days: 90, open_table_ttl_days: 7 };
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

/** The tables a production host will not deal from the walk: its two seeded
 *  creates beside an open seat (`productionSeedRule`'s first cell). */
const SKIPPED = new Set(["wire", "wire2"]);

/**
 * A fake PRODUCTION Worker. Forces answer `forceStatus`; every other request
 * takes the next answer in `plan`, which is built in the replay's own order
 * over the rows the production seed rule SENDS: the house chat's base read,
 * then a 400 in `SEED_NOT_SOLO`'s words for each skipped table's create and
 * the committed answer for every kept row. A skipped row has NO answer: a
 * request at its address is refused by name before the plan is touched, so a
 * replay that sends one fails there. `edit(plan, rows)` perturbs it.
 */
function fakeHost({ forceStatus = 204, edit = () => {} } = {}) {
  const rows = order(SCRIPTS.walk, SCRIPTS.host, SCRIPTS.archive);
  const houseChat = committed("house_chat");
  const plan = [() => json(200, { ...houseChat, last: BASE, rev: BASE_REV, messages: [] })];
  const unplanned = new Set();
  for (const { row } of rows) {
    if (isProductionSeedCreate(row, SKIPPED)) {
      plan.push(() => json(400, { error: SEED_NOT_SOLO }));
    } else if (skippedUnderProductionSeedRule(row, SKIPPED)) {
      unplanned.add(`${row.method} ${urlOf(row)}`);
    } else {
      const status = row.fixture === "archive_post_abandoned" ? 200 : row.status;
      plan.push(() => json(status, row.fixture === null ? {} : liveBody(row.fixture)));
    }
  }
  edit(plan, rows);
  const requests = [];
  const fetcher = async (url, init) => {
    requests.push({ url, method: init.method, headers: { ...init.headers }, body: init.body, redirect: init.redirect });
    if (/\/api\/admin\/tables\/[^/]+\/expire\?force$/.test(url)) {
      return forceStatus === 204 ? json(204, null) : json(forceStatus, committed("not_seated_403"));
    }
    const next = unplanned.has(`${init.method} ${url}`) ? undefined : plan.shift();
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

/** Where row `n` of `script` sits in the plan: after the base read, among the
 *  rows the production seed rule SENDS. A row it skips has no place there, and
 *  is refused rather than read as the base read's slot. */
const planIndex = (rows, script, n) => {
  const at = rows.filter(({ row }) => !skippedUnderProductionSeedRule(row, SKIPPED)).findIndex((r) => r.script === script && r.index === n);
  if (at === -1) throw new Error(`${script} row ${n} is not a row the production seed rule sends`);
  return 1 + at;
};

describe("replay", () => {
  it("holds against a production host that refuses the two seeded creates and answers every kept row, and names every exemption it spent", async () => {
    const host = fakeHost();
    const { result, text } = await run(host);
    expect(result.mismatches).toEqual([]);
    expect(result.holds).toBe(true);
    const skipped = new Set(["wire", "wire2"]);
    const sent = host.rows.filter(({ row }) => !skippedUnderProductionSeedRule(row, skipped));
    expect(result.run, "every row the rule sends ran").toBe(sent.length);
    expect(result.skipped, "and the rest were skipped, counted").toBe(host.rows.length - sent.length);
    expect(result.run + result.skipped).toBe(result.total);
    expect(result.named).toBe(sent.filter(({ row }) => row.fixture !== null && !isProductionSeedCreate(row, skipped)).length);
    expect(result.spent.map((s) => s.split(":")[0]).sort()).toEqual(
      [
        "HOST_FIELDS health.archive",
        "HOST_FIELDS health.identity",
        "HOST_FIELDS health.open_table_ttl_days",
        "HOST_FIELDS health.signal",
        "HOST_FIELDS health.table_ttl_days",
        "HOST_FIELDS me.mode",
        "HOST_FIELDS me_signed_out.mode",
        "firstSendAccepts archive_post",
        "firstSendAccepts archive_post_abandoned",
        "normalizeHouseChat house_chat",
        "normalizeHouseChat house_chat_post",
        "houseDeleteRow house_chat_delete",
        "productionSeedRule wire",
        "productionSeedRule wire2",
      ].sort(),
    );
    expect(host.requests.filter((r) => /\/api\/games\/wire2?(\/|$)/.test(r.url) && !r.url.includes("/api/admin/"))).toEqual([]);
    // The run's own post is the one it deletes (R-D4): the fake's post was
    // given the committed number plus the base.
    const deleted = host.requests.filter((r) => r.method === "DELETE" && r.url.startsWith(`${ORIGIN}/api/chat/`));
    expect(deleted.map((r) => r.url)).toEqual([`${ORIGIN}/api/chat/${BASE + 1}`]);
    expect(result.forcedBefore.map((f) => `${f.id} ${f.status}`)).toEqual(IDS.map((id) => `${id} 204`));
    expect(result.forcedAfter.map((f) => `${f.id} ${f.status}`)).toEqual(IDS.map((id) => `${id} 204`));
    expect(text).toContain("rows skipped under the production seed rule: ");
    expect(text).toContain("live-replay: HOLDS");
    expect(text).toContain("the house chat's base: last 4321, rev 9876, read as blue@wire.test");
  });

  // A host that HONOURED a seeded create beside an open seat is not under
  // the production rule — a diagnostic configuration deployed by mistake.
  it("refuses a host that honours a seeded create beside an open seat", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 1);
        expect(rows.find((r) => r.script === "walk.json" && r.index === 1).row.body.id, "the premise").toBe("wire");
        plan[i] = () => json(201, { id: "wire" });
      },
    });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.label.split(":")[0]} ${m.path}`)).toEqual(["walk.json row 1 (production seed rule)"]);
    expect(text).toContain("expected 400 in SEED_NOT_SOLO's words · got 201");
  });

  it("sends the credential to the Worker's origin alone, the forces with only the JSON type, and a claimed header only for a row's who", async () => {
    const host = fakeHost();
    await run(host);
    const forces = host.requests.filter((r) => r.url.includes("/api/admin/"));
    expect(forces.map((r) => `${r.method} ${r.url}`)).toEqual(
      [...IDS, ...IDS].map((id) => `POST ${ORIGIN}/api/admin/tables/${id}/expire?force`),
    );
    for (const force of forces) {
      expect(force.headers, "a force is the owner's own assertion, over WARP: no credential, only the JSON type since B5").toEqual({ "content-type": "application/json" });
    }
    // The base read, then each row the production seed rule sends.
    const sentRows = host.rows.filter(({ row }) => !skippedUnderProductionSeedRule(row, SKIPPED));
    const others = host.requests.filter((r) => !r.url.includes("/api/admin/"));
    expect(others).toHaveLength(1 + sentRows.length);
    for (const r of others) {
      expect(new URL(r.url).origin).toBe(ORIGIN);
      expect(r.headers["CF-Access-Client-Id"]).toBe("dummy-id.access");
      expect(r.headers["CF-Access-Client-Secret"]).toBe("dummy-secret-value");
      expect(r.redirect, "a redirect is answered, never followed").toBe("manual");
    }
    // The base read, then each sent row: its claimed header is its `who`, and
    // a row with no `who` sends none.
    const [base, ...sent] = others;
    expect(base.headers["X-Yokohama-Claimed-User"]).toBe("blue@wire.test");
    expect(sent).toHaveLength(sentRows.length);
    sentRows.forEach(({ row }, i) => {
      expect(sent[i].headers["X-Yokohama-Claimed-User"], `${row.method} ${row.path}`).toBe(row.who ?? undefined);
    });
    expect(sentRows.some(({ row }) => row.who === null), "the premise: a sent row with no who exists").toBe(true);
  });

  // **The minted session** (auth cycle, Task 8; ruling R-39). Under a sign-in arm the Worker names a caller by its own
  // cookie, so the instruments send the minted one — and, when Access still stands at the edge, the credential too, as the
  // EDGE's pass and not as an identity. A walk row's `who` is a claimed name, which the cookie makes a second identity.
  describe("under a minted session cookie", () => {
    const COOKIE = "__Host-yokohama_session=v1.eyJhZGRyZXNzIjoicmVkQHQudGVzdCJ9.AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    // The committed walk names a `who` on its house reader and on the rows that decide the production seed rule, so it
    // cannot run under a cookie: these cells run a three-row walk that names no one, against a fetcher of their own.
    const row = (method, path, status, fixture, body = null) => ({ who: null, method, path, query: "", body, status, fixture });
    const MINI = {
      walk: [row("GET", "/api/chat", 200, "house_chat"), row("POST", "/api/games", 201, null, { id: "solo" }), row("GET", "/api/health", 200, null)],
      host: [],
      archive: [],
    };
    const miniHost = () => {
      const requests = [];
      const fetcher = async (url, init) => {
        requests.push({ url, method: init.method, headers: { ...init.headers } });
        if (url.includes("/api/admin/")) return json(204, null);
        if (url.endsWith("/api/chat")) return json(200, { ...committed("house_chat"), last: BASE, rev: BASE_REV, messages: [] });
        return json(url.endsWith("/api/games") ? 201 : 200, {});
      };
      return { fetcher, requests };
    };
    const runAs = async (host, extra, scripts = MINI) => {
      const lines = [];
      const result = await replay({ ...scripts, corpus: CORPUS, fetcher: host.fetcher, log: (l) => lines.push(l), pause: async () => {}, ...extra });
      return { result, text: lines.join("\n") };
    };
    const rowsSent = (host) => host.requests.filter((r) => !r.url.includes("/api/admin/"));

    it("cookie alone: every row carries the Cookie and nothing of Access's, and no claimed name; the forces are unchanged", async () => {
      const host = miniHost();
      await runAs(host, { credential: null, cookie: COOKIE });
      const sent = rowsSent(host);
      expect(sent, "the base read and the three rows").toHaveLength(4);
      for (const r of sent) {
        expect(r.headers.Cookie).toBe(COOKIE);
        expect(r.headers["CF-Access-Client-Id"]).toBeUndefined();
        expect(r.headers["CF-Access-Client-Secret"]).toBeUndefined();
        expect(r.headers["X-Yokohama-Claimed-User"]).toBeUndefined();
      }
      for (const f of host.requests.filter((r) => r.url.includes("/api/admin/"))) {
        expect(f.headers, "a force is still only the JSON type").toEqual({ "content-type": "application/json" });
      }
    });

    it("cookie with the credential: both ride, the credential as the edge's pass, and still no claimed name", async () => {
      const host = miniHost();
      await runAs(host, { credential: { id: "dummy-id.access", secret: "dummy-secret-value" }, cookie: COOKIE });
      expect(rowsSent(host)).toHaveLength(4);
      for (const r of rowsSent(host)) {
        expect(r.headers.Cookie).toBe(COOKIE);
        expect(r.headers["CF-Access-Client-Id"]).toBe("dummy-id.access");
        expect(r.headers["CF-Access-Client-Secret"]).toBe("dummy-secret-value");
        expect(r.headers["X-Yokohama-Claimed-User"]).toBeUndefined();
      }
    });

    it("refuses a row with a claimed name beside the cookie, before anything is sent, in the sentence", async () => {
      const host = miniHost();
      await expect(runAs(host, { credential: null, cookie: COOKIE }, SCRIPTS)).rejects.toThrow(
        "the session cookie names the caller — a claimed name beside it is two identities",
      );
      expect(host.requests, "not even a force").toEqual([]);
    });

    // `send` is the second line behind `replay`'s pre-check: a row with a `who` is refused where it would be sent, so no
    // other caller of `send` can put a claimed name beside a cookie.
    // **The secret** (ruling R-50): the committed walk's shape — rows naming several callers — runs under a sign-in arm
    // when the instrument MINTS a session per `who` from the deployment's secret.
    describe("under the session secret", () => {
      const SECRET = "0123456789abcdef0123456789abcdef";
      const named = (who, method, path, status, fixture, body = null) => ({ ...row(method, path, status, fixture, body), who });
      const TWO_WHOS = {
        walk: [
          named("blue@wire.test", "GET", "/api/chat", 200, "house_chat"),
          named("red@wire.test", "POST", "/api/games", 201, null, { id: "solo" }),
          named(null, "GET", "/api/health", 200, null),
          named("red@wire.test", "GET", "/api/health", 200, null),
          named("blue@wire.test", "GET", "/api/health", 200, null),
        ],
        host: [],
        archive: [],
      };
      const decoded = async (cookieHeader) => {
        const { readSession } = await import("../src/session.ts");
        return readSession(cookieHeader, SECRET, Math.floor(Date.now() / 1000));
      };

      it("sends a different cookie per who, each readable as its address, no claimed header, and none for a row with no who", async () => {
        const host = miniHost();
        await runAs(host, { credential: null, cookie: null, secret: SECRET }, TWO_WHOS);
        const sent = rowsSent(host);
        expect(sent, "the base read and the five rows").toHaveLength(6);
        // base read (blue), the reader row (blue), create (red), health (no one), health (red), health (blue).
        const cookies = sent.map((r) => r.headers.Cookie);
        expect(cookies[3], "a row with no who sends no cookie").toBeUndefined();
        const addressOf = async (i) => (await decoded(cookies[i]))?.address;
        expect([await addressOf(0), await addressOf(1), await addressOf(2), await addressOf(4), await addressOf(5)]).toEqual([
          "blue@wire.test",
          "blue@wire.test",
          "red@wire.test",
          "red@wire.test",
          "blue@wire.test",
        ]);
        expect(new Set(cookies.filter(Boolean)).size, "two identities, two distinct cookies").toBe(2);
        expect(cookies[0], "one cookie per address, not one per row").toBe(cookies[5]);
        for (const r of sent) {
          expect(r.headers["X-Yokohama-Claimed-User"]).toBeUndefined();
          expect(r.headers["CF-Access-Client-Id"]).toBeUndefined();
        }
        for (const f of host.requests.filter((r) => r.url.includes("/api/admin/"))) {
          expect(f.headers, "a force is still only the JSON type").toEqual({ "content-type": "application/json" });
        }
      });

      it("a session is thirty days from the real clock, sid null", async () => {
        const host = miniHost();
        const before = Math.floor(Date.now() / 1000);
        await runAs(host, { credential: null, cookie: null, secret: SECRET }, TWO_WHOS);
        const after = Math.floor(Date.now() / 1000);
        const s = await decoded(rowsSent(host)[0].headers.Cookie);
        expect(s.sid).toBeNull();
        // Seconds, from the real clock: a millisecond clock would stand for millennia, a fixed one for nothing.
        expect(s.issued).toBeGreaterThanOrEqual(before);
        expect(s.issued).toBeLessThanOrEqual(after);
        expect(s.expires - s.issued).toBe(30 * 86_400);
      });

      it("with the credential too, the Access headers ride beside the minted cookie as the edge's pass", async () => {
        const host = miniHost();
        await runAs(host, { credential: { id: "dummy-id.access", secret: "dummy-secret-value" }, cookie: null, secret: SECRET }, TWO_WHOS);
        for (const r of rowsSent(host)) {
          expect(r.headers["CF-Access-Client-Id"]).toBe("dummy-id.access");
          expect(r.headers["CF-Access-Client-Secret"]).toBe("dummy-secret-value");
          expect(r.headers["X-Yokohama-Claimed-User"]).toBeUndefined();
        }
        expect(rowsSent(host).filter((r) => r.headers.Cookie)).toHaveLength(5);
      });

      it("refuses a who that is not a .test address with the script's own fence sentence, before anything is sent", async () => {
        const host = miniHost();
        const real = { ...TWO_WHOS, walk: [...TWO_WHOS.walk.slice(0, 3), named("jamoore@gmail.com", "GET", "/api/health", 200, null)] };
        await expect(runAs(host, { credential: null, cookie: null, secret: SECRET }, real)).rejects.toThrow(
          "walk.json row 3: mint-session: only a .test address may be minted — got jamoore@gmail.com",
        );
        expect(host.requests, "not even a force").toEqual([]);
      });

      it("wins over the one cookie: the cookie is not sent, and a who beside it is no refusal", async () => {
        const host = miniHost();
        await runAs(host, { credential: null, cookie: COOKIE, secret: SECRET }, TWO_WHOS);
        const sent = rowsSent(host);
        expect(sent).toHaveLength(6);
        for (const r of sent) expect(r.headers.Cookie).not.toBe(COOKIE);
        expect((await decoded(sent[2].headers.Cookie)).address).toBe("red@wire.test");
      });

      it("never prints the secret", async () => {
        const host = miniHost();
        const { text } = await runAs(host, { credential: null, cookie: null, secret: SECRET }, TWO_WHOS);
        expect(text).not.toContain(SECRET);
        expect(text).toContain("a session minted per identity");
      });
    });

    it("send itself refuses a row with a claimed name beside the cookie, and sends nothing", async () => {
      let calls = 0;
      const fetcher = async () => ((calls += 1), json(200, {}));
      const named = { ...row("GET", "/api/health", 200, null), who: "red@t.test" };
      await expect(send(named, null, fetcher, COOKIE)).rejects.toThrow("a claimed name beside it is two identities");
      await expect(send(named, { id: "i", secret: "s" }, fetcher, COOKIE)).rejects.toThrow("a claimed name beside it is two identities");
      expect(calls).toBe(0);
      // And the same row without a cookie is the dev door's: the claimed header beside the credential.
      await send(named, { id: "i", secret: "s" }, async (url, init) => ((calls += 1), expect(init.headers["X-Yokohama-Claimed-User"]).toBe("red@t.test"), json(200, {})), null);
      expect(calls).toBe(1);
    });

    it("names the row it refuses and never prints the cookie", async () => {
      let said = "";
      try {
        await runAs(miniHost(), { credential: null, cookie: COOKIE }, SCRIPTS);
      } catch (e) {
        said = String(e);
      }
      expect(said).toMatch(/walk\.json row \d+/);
      expect(said).not.toContain(COOKIE);
      expect(said).not.toContain("eyJ");
    });

    it("with no cookie a run still needs its credential, and sends the claimed names as before", async () => {
      const host = miniHost();
      await expect(runAs(host, { credential: null, cookie: null }, SCRIPTS)).rejects.toThrow(/credential/);
      expect(host.requests).toEqual([]);
    });
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
        const i = planIndex(rows, "walk.json", 28);
        expect(rows.find((r) => r.script === "walk.json" && r.index === 28).row.fixture, "the premise").toBe("health");
        const body = liveBody("health");
        plan[i] = () => json(200, { ...body, semantics_rev: body.semantics_rev + 1 });
      },
    });
    const { result, text } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path}`)).toEqual(["health semantics_rev"]);
    expect(result.run, "a body that differs changes no state, so the replay runs on").toBe(result.total - result.skipped);
    expect(result.forcedAfter).toHaveLength(3);
    expect(text).toContain("walk.json row 28: GET /api/health as red@wire.test — health semantics_rev: committed 7 · got 8");
  });

  it("stops at a status that differs, forces the ids again, and never prints a request body", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        plan[planIndex(rows, "walk.json", 43)] = () => json(403, committed("not_seated_403"));
      },
    });
    const { result, text } = await run(host);
    // The last row sent is the one it stopped at.
    const stoppedAt = host.requests.filter((r) => !r.url.includes("/api/admin/")).at(-1);
    expect(`${stoppedAt?.method} ${stoppedAt?.url}`).toBe(`POST ${ORIGIN}/api/chat`);
    const said = JSON.parse(stoppedAt?.body ?? "{}").text;
    expect(said, "the premise: the stopped row's request carried the house post's text").toBe("anyone up for a second table?");
    expect(result.holds).toBe(false);
    expect(result.run, "the rows after it would read a state it did not make").toBe(planIndex(host.rows, "walk.json", 43));
    expect(result.forcedAfter.map((f) => f.status)).toEqual([204, 204, 204]);
    expect(text).toContain("walk.json row 43: POST /api/chat as red@wire.test — status: committed 201 · got 403");
    expect(text).toContain("you are not seated at this table");
    expect(text).not.toContain(said);
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
        expect(rows.find((r) => r.script === "archive-walk.json" && r.index === 0).row.fixture, "the premise").toBe("archive_post");
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

  it("redacts a value that could be a seed or a secret when it reports one", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        const i = planIndex(rows, "walk.json", 28);
        expect(rows.find((r) => r.script === "walk.json" && r.index === 28).row.fixture, "the premise").toBe("health");
        plan[i] = () => json(200, { ...liveBody("health"), seed: "13377331133773311337" });
      },
    });
    const { result, text } = await run(host);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path}`)).toEqual(["health seed"]);
    expect(text).toContain("health seed: committed (absent) · got <redacted>");
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
      "health archive exempted, but this host sent no string or number for it",
    ]);
    expect(result.spent.filter((s) => s.startsWith("HOST_FIELDS health.archive"))).toEqual([]);
  });

  // The two TTLs (R-6.5c2.42) are the exempted fields a host sends as NUMBERS,
  // and `null` is what the corpus's host sends for them: a Worker that sent
  // `null` too has sent no value that differs, so the exemption is not spent.
  // Mutant: the rule's type test read as "anything but a string is refused"
  // (the TTLs refused whatever the Worker sent), or as "anything present"
  // (`null` spent).
  it("refuses a TTL the Worker sent as null, and spends the other it sent as a number", async () => {
    const host = fakeHost({
      edit: (plan, rows) => {
        plan[planIndex(rows, "walk.json", 28)] = () => json(200, { ...liveBody("health"), table_ttl_days: null });
      },
    });
    const { result } = await run(host);
    expect(result.holds).toBe(false);
    expect(result.mismatches.map((m) => `${m.fixture} ${m.path} ${m.reason}`)).toEqual([
      "health table_ttl_days exempted, but this host sent no string or number for it",
    ]);
    expect(result.spent.filter((s) => s.startsWith("HOST_FIELDS health.") && s.includes("ttl_days"))).toEqual([
      "HOST_FIELDS health.open_table_ttl_days: null → 7",
    ]);
  });
});
