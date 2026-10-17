// **The live replay's judging and its report** (M6.5c2 cycle D, Task 9): one
// of the five parts of `live-replay.mjs`, which no script but it and the other
// parts imports. A named reply against its committed file (`judge`), every
// value it reports redacted where it could hold a secret or a seed, and the
// report's summary and verdict (`finish`).

import { isDeepStrictEqual } from "node:util";
import { HOST_FIELDS } from "../corpus-walk.mjs";
import { LOBBY_PAUSE_MS, LOBBY_REREADS } from "./credential.mjs";
import { cursorNames, filterLobby, freezeClock, HOUSE_CHAT, houseWatcherTag, LOBBY, normalizeHouseChat } from "./exemptions.mjs";

/** A value one side of a comparison does not have. */
const ABSENT = Symbol("absent");

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
export function fmt(value, key) {
  if (value === ABSENT) return "(absent)";
  if (value === undefined) return "(undefined)";
  if (typeof key === "string" && SECRET_KEY.test(key)) return "<redacted>";
  const text = JSON.stringify(redacted(value));
  return text.length > 160 ? `${text.slice(0, 157)}… (${text.length} characters)` : text;
}

/** What an answer said, for a line about it: a JSON error's sentence, or the
 *  shape of any other body — never a redirect's `location`. */
export function said(reply) {
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
export function parsed(reply) {
  try {
    return reply.bytes === 0 ? null : JSON.parse(reply.text);
  } catch {
    return null;
  }
}

/** `n` of `noun`, in the plural when it is not one. */
const count = (n, noun) => `${n} ${noun}${n === 1 ? "" : "s"}`;

export function errorText(e) {
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
 * payload normalized — the house's watcher tag held to the post's reply
 * (`context.houseTag`) and taken back to the committed one. Answers the
 * differences and the exemptions it spent.
 */
export function judge(fixture, reply, context) {
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
      // An exemption is spent only on a value this host SENT, as a string or
      // a number (a number since the two TTLs, R-6.5c2.42), that differs from
      // the committed one: a field it dropped is a mismatch, never
      // `"off" → (absent)`. The rule's twin is the pool's,
      // `worker/test/corpus.test.ts`'s `hostFields` — the same rule over the
      // same fields, stated once in each of the two WORKER replays (R3's L-5;
      // ruling R-C3). The browser host's node replay
      // (`web/src/local/corpus.node.test.ts`) still spends an exemption on
      // `not.toEqual` alone, which an absent field passes: a recorded debt
      // (ruling R-C20 of `docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md`).
      if (typeof got[field] !== "string" && typeof got[field] !== "number") {
        diffs.push({ path: field, reason: "exempted, but this host sent no string or number for it", committed: committed[field], got: got[field] });
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
      const tag = houseWatcherTag(fixture, committed, got, context.houseTag);
      got = tag.got;
      spent.push(...tag.spent);
      diffs.push(...tag.diffs);
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

/** The report's summary, and the verdict. */
export function finish(result, log) {
  const forcesHeld = (forces) => forces.length > 0 && forces.every((f) => f.status === 204);
  result.holds =
    result.stopped === null &&
    result.run === result.total - result.skipped &&
    result.mismatches.length === 0 &&
    forcesHeld(result.forcedBefore) &&
    forcesHeld(result.forcedAfter);
  log(`rows run: ${result.run} of ${result.total}`);
  log(`rows skipped under the production seed rule: ${result.skipped}`);
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
