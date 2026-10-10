// **The live replay's named exemptions** (M6.5c2 cycle D, Task 9): one of the
// five parts of `live-replay.mjs`, which no script but it and the other parts
// imports; its cells are `live-replay.test.mjs`'s. The ways the live Worker is
// entitled to answer otherwise than the axum host the corpus was cut from,
// each a PURE function — a real clock (`freezeClock`), a lobby and a house
// chat other people share (`filterLobby`, `normalizeHouseChat`,
// `houseDeleteRow`), and the archive's two first sends, which land only once
// (`firstSendAccepts`, `resetRecipe`) — and the fixtures each applies to. The
// host fields are `HOST_FIELDS`, `../corpus-walk.mjs`'s. Beside them, the
// production seed rule (security S1b): the walk's seeded tables a deployed
// Worker will not deal, their creates held to its 400 in `SEED_NOT_SOLO`'s
// words and every other row at them, and every lobby page, skipped and counted
// (`productionSeedRule`, `isProductionSeedCreate`,
// `skippedUnderProductionSeedRule`).

import { isDeepStrictEqual } from "node:util";

/** `FROZEN_CREATED_AT` in `wire_fixtures.rs`: the reading every committed
 *  `created_at` and page cursor carries. */
export const FROZEN_CREATED_AT = 1_789_000_000;

/** `freeze_clock`'s floor: a reading at or under it is not a clock. */
const NOT_A_CLOCK = 1_700_000_000n;
const U64_MAX = 18_446_744_073_709_551_615n;

/** The lobby's pages: shared with the owner's tables and the e2e's. */
export const LOBBY = new Set(["games", "games_next", "games_open", "games_playing"]);
/** The house chat's two payloads: shared with every run and every person. */
export const HOUSE_CHAT = new Set(["house_chat_post", "house_chat"]);
/** `archive-walk.json`'s two FIRST sends, which land only once. */
export const FIRST_SENDS = new Set(["archive_post", "archive_post_abandoned"]);

/** `tables::SEED_NOT_SOLO` (`crates/yokohama-web/src/tables.rs`): the sentence
 *  a production host refuses a seeded create with when any human seat is not
 *  the creator's or a seat is open (security S1a, R-SEC8(b)). Held to the Rust
 *  source by `live-replay.test.mjs`. */
export const SEED_NOT_SOLO = "a seed is honoured only at a table whose every human seat is yours — leave it blank and the server draws one";

/**
 * The tables a PRODUCTION host cannot deal from the walk (security S1b, the
 * production-door walk): each create row that carries a seed and seats anyone
 * but its creator — an open seat, or a user other than its `who`. The pool
 * deals them under `YOKOHAMA_DIAGNOSTIC_SEEDS`; a deployed Worker answers the
 * create 400 in `SEED_NOT_SOLO`'s words, so the create is sent and held to
 * that answer, and every row at the table is skipped and counted.
 */
export function productionSeedRule(walk) {
  const ids = walk
    .filter((row) => row.method === "POST" && row.path === "/api/games" && row.body !== null && row.body.seed !== undefined)
    .filter((row) => (row.body.seats ?? []).some((s) => s.open === true || (typeof s.user === "string" && s.user !== row.who)))
    .map((row) => row.body.id);
  return new Set(ids);
}

/** The create of one of `skipped`'s tables: sent, and held to the 400. */
export function isProductionSeedCreate(row, skipped) {
  return row.method === "POST" && row.path === "/api/games" && row.body !== null && skipped.has(row.body.id);
}

/**
 * Whether a production host cannot be asked `row` under the rule: a row at
 * one of `skipped`'s tables (`/api/games/<id>` and everything under it, the
 * create excepted), or a lobby page, whose committed body names them.
 */
export function skippedUnderProductionSeedRule(row, skipped) {
  if (isProductionSeedCreate(row, skipped)) return false;
  const m = /^\/api\/games\/([^/?]+)/.exec(row.path);
  if (m !== null && skipped.has(m[1])) return true;
  return row.fixture !== null && LOBBY.has(row.fixture);
}

/**
 * `value` with every `created_at` and every page cursor's time read as the
 * frozen reading, as `wire_fixtures.rs`'s `freeze_clock` does — at any depth,
 * and REFUSING what it refuses: a `created_at` that is not a whole
 * non-negative number, or not past 1,700,000,000; a cursor that is not
 * `<created_at>,<id>`, or whose time is not one. `null` is the last page and
 * carries no clock. Nothing else is touched, and `value` is not rewritten.
 */
export function freezeClock(value) {
  if (Array.isArray(value)) return value.map(freezeClock);
  if (value === null || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value).map(([key, v]) => {
      if (key === "created_at") return [key, frozenAt(v)];
      if (key === "next") return [key, frozenCursor(v)];
      return [key, freezeClock(v)];
    }),
  );
}

function frozenAt(v) {
  if (!Number.isSafeInteger(v) || v < 0) throw new Error(`created_at is not a unix timestamp: ${JSON.stringify(v)}`);
  if (BigInt(v) <= NOT_A_CLOCK) {
    throw new Error(`created_at reads ${v}, which is not a clock — freezing it would hide a field that has stopped being stamped`);
  }
  return FROZEN_CREATED_AT;
}

function frozenCursor(v) {
  if (typeof v !== "string") return v;
  const comma = v.indexOf(",");
  if (comma === -1) throw new Error(`a page cursor is <created_at>,<id>: ${JSON.stringify(v)}`);
  const at = v.slice(0, comma);
  // Rust's `u64` parse: digits, an optional leading `+`, and no overflow.
  if (!/^\+?[0-9]+$/.test(at) || BigInt(at) > U64_MAX) {
    throw new Error(`a page cursor's first half is a unix timestamp: ${JSON.stringify(v)}`);
  }
  const seen = BigInt(at);
  if (seen <= NOT_A_CLOCK) {
    throw new Error(`a page cursor reads ${seen}, which is not a clock — freezing it would hide a cursor that has stopped carrying one`);
  }
  return `${FROZEN_CREATED_AT},${v.slice(comma + 1)}`;
}

/** Whether a page cursor names one of `ours`: `<created_at>,<id>`. */
export function cursorNames(cursor, ours) {
  if (typeof cursor !== "string") return false;
  const comma = cursor.indexOf(",");
  return comma !== -1 && ours.has(cursor.slice(comma + 1));
}

/**
 * A lobby page read on the SHARED lobby (spec §5.3's live half): `games`
 * filtered to the replay's own `ids`, and `next` compared only where the
 * committed cursor names one of them (`games_next.json`). Elsewhere the
 * committed page ends at `null`, and a live cursor that runs on into other
 * people's tables is taken back to it — only a cursor or `null`, so a `next`
 * of any other shape, or none at all, still differs.
 */
export function filterLobby(name, committed, got, ids) {
  if (!LOBBY.has(name)) throw new Error(`${name} is not a lobby page`);
  if (got === null || typeof got !== "object" || !Array.isArray(got.games)) {
    throw new Error(`${name}: the reply carries no games list`);
  }
  const ours = new Set(ids);
  const page = { ...got, games: got.games.filter((row) => ours.has(row?.id)) };
  const cursorOrEnd = typeof got.next === "string" || got.next === null;
  if (Object.hasOwn(got, "next") && cursorOrEnd && !cursorNames(committed.next, ours)) page.next = committed.next;
  return page;
}

/**
 * The house chat's two payloads read on the SHARED house chat (spec §9.3):
 * `seq` and `last` taken back by `base` — the `last` read at the run's start —
 * and the page's messages filtered to those after `base`. `last` is taken back
 * and never recomputed from what was kept: a person talking in the lobby
 * during a run moves it, and that run fails.
 *
 * **By `seq` alone, since M6.5c2 cycle D** (R-6.5c2.35; the orchestrator's
 * ruling, seq-only): the filter also kept only messages whose `author.user`
 * was one of the replay's identities, and that filter went with `user` — no
 * author carries an address any more, and "ours" cannot be told from a name,
 * which anyone may choose. It existed to drop OLDER messages by other
 * identities below the base, which `seq > base` drops on its own; a foreign
 * message ABOVE the base moves `last` and already failed the run, so the
 * verdict is unchanged — and that message is now KEPT, so the page's compare
 * fails beside it.
 *
 * **The page's `rev` is taken back by `baseRev`** (M6.5c2 cycle D, R-D4) — the
 * revision read at the run's start beside `last` — for `last`'s reason: every
 * message and every delete anybody ever made on the shared house chat is in
 * it. A delete by another person during a run moves it, and that run fails.
 */
export function normalizeHouseChat(name, got, base, baseRev) {
  if (!HOUSE_CHAT.has(name)) throw new Error(`${name} is not a house chat payload`);
  if (name === "house_chat_post") {
    if (typeof got?.seq !== "number") throw new Error(`${name}: the reply's seq is not a number`);
    return { ...got, seq: got.seq - base };
  }
  if (typeof got?.last !== "number") throw new Error(`${name}: the reply's last is not a number`);
  if (typeof got.rev !== "number") throw new Error(`${name}: the reply's rev is not a number`);
  if (!Array.isArray(got.messages)) throw new Error(`${name}: the reply carries no messages list`);
  const messages = got.messages
    .filter((m) => typeof m?.seq === "number" && m.seq > base)
    .map((m) => ({ ...m, seq: m.seq - base }));
  return { ...got, last: got.last - base, rev: got.rev - baseRev, messages };
}

/**
 * The walk's house DELETE row as a live run sends it (M6.5c2 cycle D, R-D4):
 * `DELETE /api/chat/{n}` with `n` carried forward by `shift` — the number this
 * run's own house post was given, less the committed one — so the run takes
 * down exactly the post it made. Any other row is answered unchanged.
 *
 * **Carried by the post's own number and not by the base**, though the two
 * agree on a quiet house: a person who posts between the base read and the
 * run's post moves the run's post one further, and a delete carried by the
 * base would then name THEIR message — a 403, since the replay did not write
 * it, and the run's own post left behind. The run fails in that case anyway
 * (`last` moves); this keeps it from leaving a post as it does.
 *
 * **The once-rule does not arise here** (the plan's question): each run
 * deletes the post it made itself, so the first run and every later one
 * answer the committed 204 — never a 404 for a message an earlier run took.
 * `shift` is `undefined` until this run's post has been answered; a delete row
 * met before it is refused, since it would name a number nobody gave.
 */
export function houseDeleteRow(row, shift) {
  const m = row.method === "DELETE" ? /^\/api\/chat\/(\d+)$/.exec(row.path) : null;
  if (m === null) return row;
  if (typeof shift !== "number") throw new Error(`${row.path}: the run's own house post was not answered, so the delete names no number of its own`);
  return { ...row, path: `/api/chat/${Number(m[1]) + shift}` };
}

/**
 * What to delete when a first send meets an earlier run's save of a `wire`
 * record that has since changed (R3's M-1): each first send's record key, as
 * its committed reply names the path, and its `.table.json` twin — under the
 * root spec §2.6 gives the sender `who`, `test/` for an EMAIL ending `.test`
 * and the bucket's top for anything else, a plain name ending `.test` among
 * them — and a `wrangler r2 object delete` for each.
 */
export function resetRecipe(corpus, who) {
  // The module's own rule: `crates/yokohama-worker/src/receipt.rs`'s `is_test_identity`.
  const root = who.includes("@") && who.endsWith(".test") ? "test/" : "";
  const keys = [...FIRST_SENDS].flatMap((fixture) => {
    const { path } = JSON.parse(corpus[fixture]);
    return [`${root}${path}`, `${root}${path.replace(/\.json$/, ".table.json")}`];
  });
  return { keys, commands: keys.map((key) => `npx wrangler r2 object delete "yokohama-archive/${key}" --remote`) };
}

/**
 * One of `archive-walk.json`'s two first sends (spec §5.3): the committed
 * 201 — `"landed"` — or, on every run after the first, the once-rule's 200
 * with the same body but `"identical": true` — `"identical"`. Anything else
 * is `null`.
 */
export function firstSendAccepts(committed, status, got) {
  if (status === 201 && isDeepStrictEqual(got, committed)) return "landed";
  if (status === 200 && got?.identical === true && isDeepStrictEqual({ ...got, identical: committed.identical }, committed)) {
    return "identical";
  }
  return null;
}
