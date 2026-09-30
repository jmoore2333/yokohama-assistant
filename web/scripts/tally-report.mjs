// **Requests per move, read out of a browser walk's tally** (cycle E, Task 7;
// R-E9, straw poll row 80) — and the per-10-second peaks R-6.5c2.43's
// rate-limit value is set above.
//
//   node scripts/tally-report.mjs <tally.jsonl> --moves <n>
//
// The tally is what `e2e/harness.ts`'s `tally(page, label)` writes when
// `YOKOHAMA_WEB_TALLY=<path>` is set: one JSON line per request a browser of
// the walk made, `{ t, page, method, route, status }`, `t` in milliseconds
// since the run's start. `--moves` is the game's applied moves — the record's
// event count, which the walk prints — and it is the caller's to give because
// a tally is a list of requests and knows no game.
//
// It prints three things, and each is named in its own heading:
//   (a) requests per move, by route (`METHOD route`) and in total;
//   (b) the most requests in any 10-second window, by route and in total —
//       the rate limit's unit is 10 seconds per client, so the peak is the
//       figure a threshold must clear, and a per-move average cannot say it;
//   (c) the same two restricted to the ORACLE paths — `…/join`, `…/unlock` and
//       `/api/archive`, the three a password or a replay answers and so the
//       three a rate limit exists for.
//
// **The API totals and the whole-page totals are different lines.** What is
// not `/api` — the shell, the scripts, the art, the fonts — is one row, kept
// out of the per-route table and the API figures: it is fetched once per page
// load and cached, it is not a move's cost, and on the Worker it is served from
// the assets binding rather than by the Worker. The whole-page line counts it,
// since the zone's rate limit covers every path.
//
// **ONE regex names a route** (`ROUTE_IDS`), and `harness.ts` imports it from
// here rather than holding a second: a game's id is `<id>`, a position's or a
// message's number `<n>`, and the query goes. The report re-applies it to
// every line, which is a no-op on a line the harness already named (the cell
// pins that) and what makes a tally from any writer group the same way.

import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

/** The rate limit's window, in milliseconds. */
export const WINDOW_MS = 10_000;

/**
 * **The one regex.** Global, with two arms: the game's id after
 * `/api/games/`, and a number after `/at/` or `/chat/` at the end of a path —
 * a scrubbed position (`…/at/{k}`) or a deleted message (`…/chat/{seq}`,
 * `/api/chat/{seq}`). The id arm is anchored at the start, so a path that
 * merely contains `/api/games/` elsewhere is not renamed.
 */
export const ROUTE_IDS = /^(\/api\/games\/)[^/?#]+|(\/(?:at|chat)\/)\d+$/g;

/**
 * A request's route: the path with its query and fragment dropped and its ids
 * named by [`ROUTE_IDS`].
 *
 * @param {string} path a path, with or without a query
 * @returns {string}
 */
export function normalizeRoute(path) {
  const bare = path.split(/[?#]/, 1)[0] ?? "";
  return bare.replace(ROUTE_IDS, (_m, games, tail) => (games ? `${games}<id>` : `${tail}<n>`));
}

/** The oracle paths (R-6.5c2.43): a join, an unlock, an archive upload. */
const ORACLE = /^(?:\/api\/games\/<id>\/(?:join|unlock)|\/api\/archive)$/;

/**
 * @param {string} route a route as [`normalizeRoute`] names it
 * @returns {boolean}
 */
export function isOracle(route) {
  return ORACLE.test(route);
}

/**
 * The most of `times` inside any half-open window `[t, t + windowMs)`.
 *
 * @param {number[]} times milliseconds, in any order
 * @param {number} windowMs
 * @returns {number}
 */
export function peakInWindow(times, windowMs) {
  const sorted = [...times].sort((x, y) => x - y);
  let peak = 0;
  let lo = 0;
  for (let hi = 0; hi < sorted.length; hi += 1) {
    while (/** @type {number} */ (sorted[hi]) - /** @type {number} */ (sorted[lo]) >= windowMs) lo += 1;
    peak = Math.max(peak, hi - lo + 1);
  }
  return peak;
}

/**
 * @typedef {{ t: number, page: string, method: string, route: string, status: number }} TallyLine
 */

/**
 * The tally's lines, each checked: a line that is not one is a refusal naming
 * its number, never a line quietly skipped — a report over fewer requests than
 * the walk made would be a smaller number than the true one.
 *
 * @param {string} text
 * @returns {TallyLine[]}
 */
export function parseTally(text) {
  /** @type {TallyLine[]} */
  const out = [];
  const rows = text.split("\n");
  rows.forEach((raw, i) => {
    if (raw.trim() === "") return;
    /** @type {unknown} */
    let v;
    try {
      v = JSON.parse(raw);
    } catch {
      throw new Error(`tally line ${i + 1} is not JSON: ${raw.slice(0, 80)}`);
    }
    const o = /** @type {Record<string, unknown>} */ (v);
    if (
      typeof v !== "object" ||
      v === null ||
      typeof o.t !== "number" ||
      typeof o.page !== "string" ||
      typeof o.method !== "string" ||
      typeof o.route !== "string" ||
      typeof o.status !== "number"
    ) {
      throw new Error(`tally line ${i + 1} is not { t, page, method, route, status }: ${raw.slice(0, 80)}`);
    }
    out.push({ t: o.t, page: o.page, method: o.method, route: o.route, status: o.status });
  });
  return out;
}

/**
 * @typedef {{ count: number, perMove: number, peak: number }} Figure
 * @typedef {Figure & { method: string, route: string, refused: number }} RouteFigure
 * @typedef {{
 *   moves: number,
 *   requests: number,
 *   spanMs: number,
 *   pages: Record<string, number>,
 *   routes: RouteFigure[],
 *   api: Figure,
 *   oracle: Figure,
 *   notApi: Figure,
 *   all: Figure,
 * }} Report
 */

/**
 * @param {TallyLine[]} lines
 * @param {number} moves
 * @returns {Figure}
 */
function figure(lines, moves) {
  return {
    count: lines.length,
    perMove: lines.length / moves,
    peak: peakInWindow(
      lines.map((l) => l.t),
      WINDOW_MS,
    ),
  };
}

/**
 * The report's figures over `lines` for a game of `moves` applied moves.
 * Routes are sorted by count, most first, then by name.
 *
 * @param {TallyLine[]} lines
 * @param {{ moves: number }} opts
 * @returns {Report}
 */
export function tallyReport(lines, { moves }) {
  const named = lines.map((l) => ({ ...l, route: normalizeRoute(l.route) }));
  const api = named.filter((l) => l.route.startsWith("/api/") || l.route === "/api");
  const notApi = named.filter((l) => !api.includes(l));
  /** @type {Map<string, typeof named>} */
  const byRoute = new Map();
  for (const l of api) {
    const key = `${l.method} ${l.route}`;
    const held = byRoute.get(key);
    if (held) held.push(l);
    else byRoute.set(key, [l]);
  }
  /** @type {RouteFigure[]} */
  const routes = [...byRoute.values()]
    .map((ls) => ({
      method: /** @type {typeof named[number]} */ (ls[0]).method,
      route: /** @type {typeof named[number]} */ (ls[0]).route,
      ...figure(ls, moves),
      // A 4xx, a 5xx, or no response at all (status 0): still a request the
      // server — or the edge — was asked to answer, so it is counted above.
      refused: ls.filter((l) => l.status === 0 || l.status >= 400).length,
    }))
    .sort((x, y) => y.count - x.count || `${x.route} ${x.method}`.localeCompare(`${y.route} ${y.method}`));
  /** @type {Record<string, number>} */
  const pages = {};
  for (const l of named) pages[l.page] = (pages[l.page] ?? 0) + 1;
  const times = named.map((l) => l.t);
  return {
    moves,
    requests: named.length,
    spanMs: times.length ? Math.max(...times) - Math.min(...times) : 0,
    pages,
    routes,
    api: figure(api, moves),
    oracle: figure(
      api.filter((l) => isOracle(l.route)),
      moves,
    ),
    notApi: figure(notApi, moves),
    all: figure(named, moves),
  };
}

/**
 * The report as the command prints it: one row per route, then the totals,
 * every row `name  requests  per-move  peak/10s [refused]`.
 *
 * @param {Report} r
 * @returns {string}
 */
export function formatReport(r) {
  const width = Math.max(44, ...r.routes.map((x) => x.method.length + 1 + x.route.length));
  /** @param {string} name @param {Figure} f @param {number} [refused] */
  const row = (name, f, refused) =>
    `${name.padEnd(width)}  ${String(f.count).padStart(7)}  ${f.perMove.toFixed(2).padStart(8)}  ${String(f.peak).padStart(8)}` +
    (refused === undefined ? "" : `  ${String(refused).padStart(7)}`);
  const pages = Object.entries(r.pages)
    .sort(([x], [y]) => x.localeCompare(y))
    .map(([p, n]) => `${p} ${n}`)
    .join(", ");
  return [
    `${r.requests} requests over ${r.moves} moves in ${(r.spanMs / 1000).toFixed(1)} s (by page: ${pages || "none"})`,
    "",
    `(a, b) requests per move and the most in any ${WINDOW_MS / 1000}-second window, by route`,
    `${"route".padEnd(width)}  ${"requests".padStart(7)}  ${"per move".padStart(8)}  ${"peak/10s".padStart(8)}  ${"refused".padStart(7)}`,
    ...r.routes.map((x) => row(`${x.method} ${x.route}`, x, x.refused)),
    "",
    "totals",
    row("all /api requests", r.api),
    row("not /api (the shell, scripts, art, fonts)", r.notApi),
    row("every request", r.all),
    "",
    "(c) the oracle paths",
    row("oracle paths (join, unlock, /api/archive)", r.oracle),
  ].join("\n");
}

/**
 * The command's arguments: the tally's path and `--moves <n>`, a positive
 * whole number.
 *
 * @param {string[]} argv
 * @returns {{ path: string, moves: number }}
 */
export function parseArgs(argv) {
  let path = null;
  let moves = null;
  for (let i = 0; i < argv.length; i += 1) {
    const a = argv[i];
    if (a === "--moves") {
      const raw = argv[i + 1] ?? "";
      i += 1;
      if (!/^[1-9]\d*$/.test(raw)) throw new Error(`--moves takes the game's applied moves, a positive whole number: ${JSON.stringify(raw)}`);
      moves = Number(raw);
    } else if (path === null && a !== undefined) path = a;
    else throw new Error(`unexpected argument ${JSON.stringify(a)}`);
  }
  if (path === null) throw new Error("usage: node scripts/tally-report.mjs <tally path> --moves <n>");
  if (moves === null) throw new Error("--moves <n> is required: the game's applied moves, which the walk prints");
  return { path, moves };
}

if (process.argv[1] !== undefined && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const { path, moves } = parseArgs(process.argv.slice(2));
    const report = tallyReport(parseTally(readFileSync(path, "utf8")), { moves });
    process.stdout.write(`${formatReport(report)}\n`);
  } catch (e) {
    process.stderr.write(`tally-report: ${e instanceof Error ? e.message : String(e)}\n`);
    process.exit(1);
  }
}
