// **The corpus, replayed against the LIVE Worker** (spec §5.3's live half,
// R-6.5c.12; the plan's Task 15). Run by the ORCHESTRATOR from the owner's
// workstation, over WARP, after the quota guard — never by a gate:
//
//   cd worker && node scripts/live-replay.mjs
//
// `crates/yokohama-web/tests/wire_fixtures.rs` walked the axum router through a
// deterministic two-player game and froze every payload the client reads under
// `web/src/wire/`; the pool replays the same walk through the objects
// (`test/corpus.test.ts`). This sends it to the deployed Worker, through the
// edge and Access, as the walk's own `.test` identities — and it is
// REPEATABLE, and safe to re-run after a run that failed half way, for as long
// as the walk's `wire` records are the bytes its two first sends saved (see
// "The reset" below):
//
// 1. **The operator's force** on each table the walk creates (`wire`, `wire2`,
//    `wire4`), `POST /api/admin/tables/<id>/expire?force` — the BARE key, since
//    a value is a 400 (ruling I-72) — sent with NO credential and no claimed
//    header, so WARP makes it the owner's own assertion: the credential is
//    never an operator (R-6.5c.16). A force leaves no marker, and a force on an
//    object holding nothing is 204 writing nothing. Anything but three 204s
//    and the run does not start.
// 2. **The house chat's `last` and `rev`**, read as the walk's own house-chat
//    reader: the base its payloads are taken back by (spec §9.3).
// 3. **Every row** of `walk.json`, each `walk-host.json` row after the walk
//    row its `after` names, then `archive-walk.json`, with the credential and
//    `X-Yokohama-Claimed-User: <who>` — a row with no `who` sends none.
// 4. **Every row's status, and every named row's body** against its committed
//    file — the clock frozen first (`freezeClock`), then three named
//    exemptions (`filterLobby`, `normalizeHouseChat`, `firstSendAccepts`, the
//    exported pure functions below) and `HOST_FIELDS`, each COUNTED when it is
//    spent; a host field that reads as the axum host's is refused, because an
//    exemption that differs nowhere is a hole a later host could hide behind.
//    **The walk's house post is DELETED by the run that made it** (M6.5c2
//    cycle D, R-D4): the delete row is sent carried forward to the number
//    that run's post was really given (`houseDeleteRow`, counted like the
//    rest), so every run answers 204 and no run leaves a post behind.
// 5. **The force again** once the replay has begun, however it ended; then the
//    report: rows run, named rows compared, the exemptions spent, and every
//    mismatch. Exit 1 on a mismatch, a refused force, or a run that stopped.
//
// **The reset** (R3's M-1). The two first sends' saves stay in the bucket for
// good, by the owner's direction, so every later run's first sends meet them:
// the once-rule's 200 `identical`. A `wire` record whose bytes have changed —
// a version bump, a walk that applies other moves, a record format change —
// meets them as a 409 instead. The run stops there, since every row after it
// reads a save it did not make, names the cause, and prints the four keys and
// a `npx wrangler r2 object delete` for each (`resetRecipe`), run in `worker/`
// with wrangler's own login: the owner's R2 key is read-only (spec §7).
//
// **The secret.** `.dev.vars` is read by path and its values are never
// printed, and the credential goes to the Worker's own origin alone — a row
// whose path would leave it is refused before anything is sent, and no
// redirect is followed. This script never prints a request BODY (the walk's
// bodies carry passwords and seeds), nor a redirect's `location` (Access's
// carries a token), and a reported value under a key that could hold a secret
// or a seed prints as `<redacted>`.

import { readFileSync, readdirSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { HOST_FIELDS, order } from "./corpus-walk.mjs";
import { KEYS, ORIGIN, readCredential } from "./dev-vars.mjs";

// The origin, the only one this script sends to, the credential and its
// reader (`dev-vars.mjs`), and the walk's order (`corpus-walk.mjs`): each
// stated once, and imported where the live tier and the pool need them too.
export { ORIGIN, order, readCredential };

/** `FROZEN_CREATED_AT` in `wire_fixtures.rs`: the reading every committed
 *  `created_at` and page cursor carries. */
export const FROZEN_CREATED_AT = 1_789_000_000;

/** `freeze_clock`'s floor: a reading at or under it is not a clock. */
const NOT_A_CLOCK = 1_700_000_000n;
const U64_MAX = 18_446_744_073_709_551_615n;

const REPO = new URL("../../", import.meta.url);

/** The lobby's pages: shared with the owner's tables and the e2e's. */
const LOBBY = new Set(["games", "games_next", "games_open", "games_playing"]);
/** The house chat's two payloads: shared with every run and every person. */
const HOUSE_CHAT = new Set(["house_chat_post", "house_chat"]);
/** `archive-walk.json`'s two FIRST sends, which land only once. */
const FIRST_SENDS = new Set(["archive_post", "archive_post_abandoned"]);

/**
 * How many times a lobby page is read again while it differs, and how long
 * apart. The mechanism it waits out: a table's slot push to the house rides
 * `waitUntil` AFTER the table's reply (spec §2.5), so a lobby GET sent right
 * after a create — or a join, or a keep — can reach the house before the push
 * does. The pool flushes with `deliverSlot` before every house row; a live run
 * cannot, so it reads again instead. Every re-read is counted and printed with
 * this bound, and a page still differing after the last is a MISMATCH.
 */
const LOBBY_REREADS = 5;
const LOBBY_PAUSE_MS = 500;

const REQUEST_TIMEOUT_MS = 60_000;

/** A value one side of a comparison does not have. */
const ABSENT = Symbol("absent");

// ---- the four named normalizations (`live-replay.test.mjs`) ----

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
function cursorNames(cursor, ours) {
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

// ---- the rows, and where they go ----

/** The three request scripts, from `web/src/wire-script/`. */
export function loadScripts() {
  const read = (name) => JSON.parse(readFileSync(new URL(`web/src/wire-script/${name}`, REPO), "utf8"));
  return { walk: read("walk.json"), host: read("walk-host.json"), archive: read("archive-walk.json") };
}

/** The committed corpus, `web/src/wire/*.json`, by file name. */
export function loadCorpus() {
  const dir = new URL("web/src/wire/", REPO);
  return Object.fromEntries(
    readdirSync(dir)
      .filter((f) => f.endsWith(".json"))
      .map((f) => [f.slice(0, -".json".length), readFileSync(new URL(f, dir), "utf8")]),
  );
}

/** Where `row` is sent: the Worker's origin, and nowhere else. */
export function urlOf(row) {
  const url = new URL(`${row.path}${row.query ? `?${row.query}` : ""}`, ORIGIN);
  if (url.origin !== ORIGIN) throw new Error(`the row ${row.method} ${row.path} leaves ${ORIGIN}: this script sends nothing anywhere else`);
  return url.href;
}

/** The tables the walk creates, by the ids its create bodies name. */
function tableIds(walk) {
  return walk.filter((row) => row.method === "POST" && row.path === "/api/games").map((row) => row.body.id);
}

/** One row, sent with the credential and its `who`, and its answer read
 *  whole. */
async function send(row, credential, fetcher) {
  const url = urlOf(row);
  const headers = { [KEYS[0]]: credential.id, [KEYS[1]]: credential.secret };
  if (row.who !== null) headers["X-Yokohama-Claimed-User"] = row.who;
  let body;
  if (row.body !== null) {
    body = JSON.stringify(row.body);
    headers["content-type"] = "application/json";
  }
  const res = await fetcher(url, { method: row.method, headers, body, redirect: "manual", signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS) });
  const bytes = new Uint8Array(await res.arrayBuffer());
  return { status: res.status, text: new TextDecoder().decode(bytes), bytes: bytes.length, contentType: res.headers.get("content-type") };
}

/** The operator's force on each id, bare: no credential and no claimed
 *  header. */
async function forceAll(ids, fetcher) {
  const forced = [];
  for (const id of ids) {
    try {
      const res = await fetcher(`${ORIGIN}/api/admin/tables/${encodeURIComponent(id)}/expire?force`, {
        method: "POST",
        headers: {},
        redirect: "manual",
        signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
      });
      const bytes = new Uint8Array(await res.arrayBuffer());
      forced.push({ id, status: res.status, said: said({ status: res.status, text: new TextDecoder().decode(bytes), bytes: bytes.length, contentType: res.headers.get("content-type") }) });
    } catch (e) {
      forced.push({ id, status: null, said: ` — no answer: ${errorText(e)}` });
    }
  }
  return forced;
}

// ---- reporting, and its redaction ----

/** A key whose value could be a secret or a seed. */
const SECRET_KEY = /^(seed|salt|hash|password|secret|token|cookie|authorization)$/i;

const isObject = (v) => v !== null && typeof v === "object" && !Array.isArray(v);

function redacted(value) {
  if (Array.isArray(value)) return value.map(redacted);
  if (isObject(value)) return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, SECRET_KEY.test(k) ? "<redacted>" : redacted(v)]));
  return value;
}

/** A value as the report prints it: redacted under a key that could hold a
 *  secret or a seed, at any depth, and cut short when long. */
function fmt(value, key) {
  if (value === ABSENT) return "(absent)";
  if (value === undefined) return "(undefined)";
  if (typeof key === "string" && SECRET_KEY.test(key)) return "<redacted>";
  const text = JSON.stringify(redacted(value));
  return text.length > 160 ? `${text.slice(0, 157)}… (${text.length} characters)` : text;
}

/** What an answer said, for a line about it: a JSON error's sentence, or the
 *  shape of any other body — never a redirect's `location`. */
function said(reply) {
  if (reply.status >= 300 && reply.status < 400) return " — a redirect (its location is not printed: Access's carries a token)";
  if (reply.bytes === 0) return "";
  try {
    const error = JSON.parse(reply.text)?.error;
    if (typeof error === "string") return ` — it said: ${error.length > 200 ? `${error.slice(0, 197)}…` : error}`;
  } catch {
    // Not JSON: its shape below.
  }
  return ` — a ${reply.contentType ?? "typeless"} body of ${reply.bytes} B`;
}

/** An answer's body as JSON, or `null` — for no body, or one that is not JSON. */
function parsed(reply) {
  try {
    return reply.bytes === 0 ? null : JSON.parse(reply.text);
  } catch {
    return null;
  }
}

/** `n` of `noun`, in the plural when it is not one. */
const count = (n, noun) => `${n} ${noun}${n === 1 ? "" : "s"}`;

function errorText(e) {
  return e instanceof Error ? `${e.name}: ${e.message}` : String(e);
}

/** Every leaf where `got` differs from `committed`, by dotted path. */
function differences(committed, got, path = []) {
  if (Array.isArray(committed) && Array.isArray(got)) {
    const out = [];
    for (let i = 0; i < Math.max(committed.length, got.length); i++) {
      out.push(...differences(i < committed.length ? committed[i] : ABSENT, i < got.length ? got[i] : ABSENT, [...path, i]));
    }
    return out;
  }
  if (isObject(committed) && isObject(got)) {
    const keys = [...new Set([...Object.keys(committed), ...Object.keys(got)])].sort();
    return keys.flatMap((k) =>
      differences(Object.hasOwn(committed, k) ? committed[k] : ABSENT, Object.hasOwn(got, k) ? got[k] : ABSENT, [...path, k]),
    );
  }
  return isDeepStrictEqual(committed, got) ? [] : [{ path: path.length > 0 ? path.join(".") : "(body)", committed, got }];
}

/**
 * One named reply against its committed file: the clock frozen, the host
 * fields exempted (each proven to differ), and a lobby page or a house chat
 * payload normalized. Answers the differences and the exemptions it spent.
 */
function judge(fixture, reply, context) {
  const committed = JSON.parse(context.corpus[fixture]);
  let got;
  try {
    got = reply.bytes === 0 ? null : JSON.parse(reply.text);
  } catch {
    return { diffs: [{ path: "(body)", committed: "JSON", got: `a ${reply.contentType ?? "typeless"} body of ${reply.bytes} B that is not JSON` }], spent: [] };
  }
  const spent = [];
  const diffs = [];
  try {
    got = freezeClock(got);
    for (const field of HOST_FIELDS[fixture] ?? []) {
      if (!isObject(got)) break;
      // An exemption is spent only on a value this host SENT, as a string,
      // that differs from the committed one: a field it dropped is a
      // mismatch, never `"off" → (absent)`. The rule's twin is the pool's,
      // `worker/test/corpus.test.ts`'s `hostFields` — the same rule over the
      // same fields, stated once in each of the two WORKER replays (R3's L-5;
      // ruling R-C3). The browser host's node replay
      // (`web/src/local/corpus.node.test.ts`) still spends an exemption on
      // `not.toEqual` alone, which an absent field passes: a recorded debt
      // (Task 7's review, M-5; ruling R-C20).
      if (typeof got[field] !== "string") {
        diffs.push({ path: field, reason: "exempted, but this host sent no string for it", committed: committed[field], got: got[field] });
      } else if (isDeepStrictEqual(got[field], committed[field])) {
        diffs.push({ path: field, reason: "exempted, but both hosts agree on it", committed: committed[field], got: got[field] });
      } else {
        spent.push(`HOST_FIELDS ${fixture}.${field}: ${fmt(committed[field])} → ${fmt(got[field])}`);
      }
    }
    if (LOBBY.has(fixture)) {
      const listed = Array.isArray(got?.games) ? got.games.length : 0;
      got = filterLobby(fixture, committed, got, context.ids);
      const compared = cursorNames(committed.next, new Set(context.ids));
      spent.push(`filterLobby ${fixture}: ${count(listed - got.games.length, "other row")} dropped; next ${compared ? "compared" : "taken back to the committed page's end"}`);
    }
    if (HOUSE_CHAT.has(fixture)) {
      const listed = Array.isArray(got?.messages) ? got.messages.length : 0;
      got = normalizeHouseChat(fixture, got, context.base, context.baseRev);
      spent.push(
        fixture === "house_chat_post"
          ? `normalizeHouseChat ${fixture}: seq taken back by ${context.base}`
          : `normalizeHouseChat ${fixture}: seq and last taken back by ${context.base}, rev by ${context.baseRev}; ${count(listed - got.messages.length, "earlier message")} dropped`,
      );
    }
  } catch (e) {
    return { diffs: [...diffs, { path: "(normalized)", reason: errorText(e) }], spent };
  }
  const strip = (v) => {
    if (!isObject(v)) return v;
    const copy = { ...v };
    for (const field of HOST_FIELDS[fixture] ?? []) delete copy[field];
    return copy;
  };
  diffs.push(...differences(strip(committed), strip(got)));
  return { diffs, spent };
}

// ---- the run ----

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

/**
 * The live replay. `fetcher`, `log` and `pause` are the network, the report's
 * sink and the wait between a lobby page's reads — the cells hand in fakes.
 * Answers the run's result; `holds` is whether every force answered 204,
 * every row ran, and nothing differed.
 */
export async function replay({ walk, host, archive, corpus, credential, fetcher = fetch, log = console.log, pause = sleep }) {
  const rows = order(walk, host, archive);
  const ids = tableIds(walk);
  const reader = rows.find(({ row }) => row.fixture === "house_chat")?.row;
  if (reader === undefined) throw new Error("no row of the scripts reads the house chat, so there is no base to read");
  // Every row's address, checked BEFORE anything is sent — the forces
  // included: a row that would leave the Worker's origin refuses the run.
  for (const { row } of rows) urlOf(row);
  const result = {
    holds: false,
    run: 0,
    total: rows.length,
    named: 0,
    spent: [],
    rereads: 0,
    mismatches: [],
    forcedBefore: [],
    forcedAfter: [],
    base: null,
    stopped: null,
    reset: null,
  };
  /** A mismatch, kept and printed as it is found. */
  const differs = (label, fixture, diff) => {
    result.mismatches.push({ label, fixture, ...diff });
    const key = diff.path.split(".").at(-1);
    const what = diff.reason ?? `committed ${fmt(diff.committed, key)} · got ${fmt(diff.got, key)}`;
    log(`  MISMATCH ${label} — ${fixture === null ? "" : `${fixture} `}${diff.path}: ${what}`);
  };

  log(`live-replay: ${ORIGIN} — ${KEYS.join(" and ")} to this origin only; the forces bare (over WARP, the owner's device answers Access)`);
  result.forcedBefore = await forceAll(ids, fetcher);
  log(`force before: ${result.forcedBefore.map((f) => `${f.id} ${f.status ?? "—"}${f.status === 204 ? "" : f.said}`).join(", ")}`);
  if (!result.forcedBefore.every((f) => f.status === 204)) {
    result.stopped = "a force before the run answered other than 204, so no row was sent";
    return finish(result, log);
  }

  try {
    const baseReply = await send(reader, credential, fetcher);
    const basePage = baseReply.status === 200 ? parsed(baseReply) : undefined;
    const last = basePage?.last;
    const baseRev = basePage?.rev;
    if (typeof last === "number" && typeof baseRev === "number") {
      result.base = last;
      log(`the house chat's base: last ${last}, rev ${baseRev}, read as ${reader.who}`);
    } else {
      // No base, no rows: the `finally` forces the ids again, and the report
      // follows it, as on every other road out.
      result.stopped = `the house chat's base could not be read: ${baseReply.status}${said(baseReply)}`;
    }
    // `houseShift`: set once the run's own house post is answered.
    const context = { corpus, ids, base: last, baseRev, houseShift: undefined };

    for (const { script, index, row: scripted } of result.stopped === null ? rows : []) {
      let row = scripted;
      try {
        row = houseDeleteRow(scripted, context.houseShift);
      } catch (e) {
        result.stopped = `${script} row ${index}: ${errorText(e)}`;
        break;
      }
      if (row !== scripted) result.spent.push(`houseDeleteRow ${row.fixture}: ${scripted.path} sent as ${row.path}`);
      const label = `${script} row ${index}: ${row.method} ${row.path}${row.query ? `?${row.query}` : ""} as ${row.who ?? "no one"}`;
      let reply;
      try {
        reply = await send(row, credential, fetcher);
      } catch (e) {
        result.stopped = label;
        differs(label, null, { path: "status", reason: `no answer: ${errorText(e)}` });
        break;
      }
      result.run += 1;
      if (result.run % 500 === 0) log(`… ${result.run} of ${result.total} rows`);

      if (row.fixture !== null && corpus[row.fixture] === undefined) throw new Error(`${row.fixture} is named by a script and not committed`);
      if (row.fixture !== null && FIRST_SENDS.has(row.fixture)) {
        result.named += 1;
        if (reply.status === 409) {
          // An earlier run's save of a `wire` record that has since changed:
          // every row after this one would read a save this run did not make.
          result.stopped =
            `${row.fixture} answered 409${said(reply)}. The bucket holds an earlier run's save of a wire record that has ` +
            "since changed (a version bump, a walk that applies other moves, a record format change): delete the " +
            "replay's four saves, then run it again";
          result.reset = resetRecipe(corpus, row.who);
          differs(label, row.fixture, { path: "(first send)", reason: `committed 201, or the once-rule's 200 · got 409${said(reply)}` });
          break;
        }
        const committed = JSON.parse(corpus[row.fixture]);
        // Not JSON reads as `null`: refused below, and reported by its shape.
        const got = parsed(reply);
        const verdict = firstSendAccepts(committed, reply.status, got);
        if (verdict === null) {
          differs(label, row.fixture, {
            path: "(first send)",
            reason: `committed 201 ${fmt(committed)}, or the once-rule's 200 with "identical": true · got ${reply.status}${got === null ? said(reply) : ` ${fmt(got)}`}`,
          });
        } else {
          result.spent.push(`firstSendAccepts ${row.fixture}: ${verdict === "landed" ? "201, the first send landed" : `200, the once-rule's "identical": true`}`);
        }
        continue;
      }

      if (reply.status !== row.status) {
        // Every row after it would read a state this run did not make.
        result.stopped = label;
        differs(label, null, { path: "status", reason: `committed ${row.status} · got ${reply.status}${said(reply)}` });
        break;
      }
      // The run's own house post: the number the delete row is carried to.
      if (row.fixture === "house_chat_post") {
        const seq = parsed(reply)?.seq;
        if (typeof seq === "number") context.houseShift = seq - JSON.parse(corpus.house_chat_post).seq;
      }
      if (row.fixture === null) continue;

      result.named += 1;
      let judged = judge(row.fixture, reply, context);
      for (let again = 0; LOBBY.has(row.fixture) && judged.diffs.length > 0 && again < LOBBY_REREADS; again++) {
        // The table's slot push may not have reached the house yet
        // (`LOBBY_REREADS` says why); what still differs after the last read
        // is reported below as a mismatch.
        await pause(LOBBY_PAUSE_MS);
        const reread = await send(row, credential, fetcher);
        result.rereads += 1;
        if (reread.status !== row.status) {
          judged = { diffs: [{ path: "status", reason: `re-read: committed ${row.status} · got ${reread.status}${said(reread)}` }], spent: [] };
          break;
        }
        judged = judge(row.fixture, reread, context);
      }
      result.spent.push(...judged.spent);
      for (const diff of judged.diffs) differs(label, row.fixture, diff);
    }
  } catch (e) {
    result.stopped = `the run stopped: ${errorText(e)}`;
  } finally {
    result.forcedAfter = await forceAll(ids, fetcher);
    log(`force after: ${result.forcedAfter.map((f) => `${f.id} ${f.status ?? "—"}${f.status === 204 ? "" : f.said}`).join(", ")}`);
  }
  return finish(result, log);
}

/** The report's summary, and the verdict. */
function finish(result, log) {
  const forcesHeld = (forces) => forces.length > 0 && forces.every((f) => f.status === 204);
  result.holds =
    result.stopped === null &&
    result.run === result.total &&
    result.mismatches.length === 0 &&
    forcesHeld(result.forcedBefore) &&
    forcesHeld(result.forcedAfter);
  log(`rows run: ${result.run} of ${result.total}`);
  log(`named rows compared: ${result.named}`);
  log(`exemptions spent (${result.spent.length}):`);
  for (const line of result.spent) log(`  ${line}`);
  log(
    `lobby re-reads: ${result.rereads} spent (the bound: ${LOBBY_REREADS} per lobby row, ${LOBBY_PAUSE_MS} ms apart; ` +
      "a page still differing after them is a mismatch)",
  );
  if (result.stopped !== null) log(`stopped: ${result.stopped}`);
  if (result.reset !== null) {
    log("the reset — in worker/, with wrangler's own login; then run the replay again:");
    for (const command of result.reset.commands) log(`  ${command}`);
  }
  log(`mismatches: ${result.mismatches.length === 0 ? "none" : result.mismatches.length}`);
  log(`live-replay: ${result.holds ? "HOLDS" : "DOES NOT HOLD"}`);
  return result;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  if (process.argv.length > 2) {
    console.error("usage: node scripts/live-replay.mjs   (no arguments: the origin is the deployed Worker's, and the credential is .dev.vars')");
    process.exit(2);
  }
  try {
    const result = await replay({ ...loadScripts(), corpus: loadCorpus(), credential: readCredential() });
    process.exit(result.holds ? 0 : 1);
  } catch (e) {
    console.error(`live-replay: ${errorText(e)}`);
    process.exit(1);
  }
}
