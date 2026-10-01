// **The probe burst — ORCHESTRATOR-ONLY** (M6.5c2 cycle F, R-F5; spec §2.5,
// Review Focus 5): scanner-shaped requests at a deployment, one TSV row per
// answer, read against the Workers Observability query API's stateless count
// for the probe's own minute (`worker/RUNBOOK.md` §7). **Two modes, two
// readings, each in its own named minute — never mixed** (Task 8's review,
// Important 3):
//
//   - **the default, the scanner's paths** (below). With the edge rule ON
//     (`scripts/edge-rule.mjs`) THE PASS is every row 403 `cf-mitigated:
//     challenge` and the minute's invocations ZERO; a control read in its own
//     minute — one `/api/health` from a cleared browser, which must count one —
//     proves the count counts;
//   - **`--under-prefix`, the residual**: misses UNDER the rule's allowed
//     prefixes, `/assets/<hex>.js` and `/art/<hex>.png`. The rule lets an allowed
//     prefix through by design. The reading, taken in cycle F's window on
//     2026-10-01 and twice (ruling D-1): every row 404 with no `cf-mitigated`,
//     in EVERY shape, `text/html` — the asset layer's cached 404 page, the
//     page's bytes — and the minute's invocations ZERO, 200 rows each time. The
//     design had expected the Worker's own miss answer and an invocation per
//     request, a navigate row excepted (spec §2.5's open question); a nonzero
//     count here would mean that reading changed. The window READS this cost;
//     it is the prefix allow-list's residual, and it is free.
//
//   node scripts/probe-burst.mjs <origin> [--under-prefix | --paths a,b,c] [--count N] [--pace ms]
//
// The window's two commands (Task 11.G.a), from `worker/`, each in its own
// minute, as cycle F ran them (off WARP, then again over it):
//
//   node scripts/probe-burst.mjs https://yokohama.moore.nyc --count 200 --pace 500 > probe/burst.tsv
//   node scripts/probe-burst.mjs https://yokohama.moore.nyc --under-prefix --count 200 --pace 500 > probe/under-prefix.tsv
//
// `--paths` names paths of the caller's own, for which no reading is stated;
// it does not mix with `--under-prefix`.
//
// **What it costs**: `--count` requests, one at a time, `--pace` milliseconds
// apart — the window's 200 at two a second take about 100 s. Each request the
// edge lets through is a Worker invocation on the deployment's quota, so it is
// run in a named window with the quota guard before it, and NEVER by a gate,
// a cell or an implementer: its cells (`probe-burst.test.mjs`) inject the
// transport and send nothing.
//
// **The three shapes**, each request one of them:
//   - `bare`: no header at all — what a scanner's plain client sends beyond the
//     socket's own `Host` and `Connection`;
//   - `navigate`: a browser's User-Agent with `Sec-Fetch-Mode: navigate` and
//     `Accept: text/html` — a browser's document load, the shape the asset
//     layer's 404 page was expected to answer ALONE (under an allowed prefix
//     it answers every shape);
//   - `no-cors`: a browser's User-Agent with `Sec-Fetch-Mode: no-cors` and
//     `Accept: */*` — a scanner impersonating a browser, and the shape Bot
//     Fight Mode's answer cannot be confused with.
// The User-Agent is `smoke-release.mjs`'s `BROWSER_UA`, one string for both.
//
// **The default paths** are the scanner's five — `/.env`, `/wp-login.php`,
// `/admin`, `/xmlrpc.php` and a random `/<hex>.json` — none under an allowed
// prefix; **`--under-prefix`'s** are the two prefix misses alone (Task 7's
// review: a miss there passes the rule; it reads the asset layer's 404 page
// with no invocation, above). `<hex>` is sixteen random hex digits, one per
// run, printed on stderr with the paths.
//
// **`node:http`, never `fetch`, and no redirect is ever followed.** MEASURED
// (Node 26.10, a loopback server): Node's `fetch` sends `sec-fetch-mode: cors`
// on every request and OVERWRITES one the caller sets — so the navigate and
// no-cors shapes could not be sent through it — and adds `accept`,
// `accept-language`, `user-agent: node` and `accept-encoding` to a request
// that set none. `http.request` sends exactly the headers given, and answers a
// 3xx as the 3xx it is: the first answer is the one read, as the smoke's live
// mode reads it.
//
// **Its output**: the TSV on stdout — `path`, `shape`, `status`,
// `cf-mitigated`, `cf-cache-status`, `cf-ray`, `content-type`, `bytes` (the
// body's length as received; no `Accept-Encoding` is sent, so it is not
// compressed), a `-` for an absent header, and `error` in the status column
// for a request that got no answer — and on stderr the run's first and last
// moments (UTC, for the query API's minute), its mode and that mode's stated
// reading, its paths, and a count of answers by status and shape. Exit 0 when every request was answered, 1 when any was
// not, 2 for arguments it refuses.

import { randomBytes } from "node:crypto";
import { request as httpRequest } from "node:http";
import { request as httpsRequest } from "node:https";
import { pathToFileURL } from "node:url";
import { BROWSER_UA } from "./smoke-release.mjs";

export const USAGE = "usage: node scripts/probe-burst.mjs <origin> [--under-prefix | --paths a,b,c] [--count N] [--pace ms]";

/** What each mode's run should read, said on stderr at its start. */
const READING = {
  scanner: "the pass: every row 403 cf-mitigated: challenge, and the minute's invocations ZERO",
  "under-prefix":
    "the residual's reading: every row 404 with no cf-mitigated, the asset layer's cached 404 page in every " +
    "shape (text/html), not the Worker's answer; the minute's invocations ZERO (read 2026-10-01, cycle F's " +
    "window: 200 of 200 rows, twice) — a nonzero count means a miss reached the Worker",
  paths: "the caller's own paths: no reading is stated",
};

/** The three shapes, by name, in the order each path is asked in. */
export const SHAPES = Object.freeze({
  bare: Object.freeze({}),
  navigate: Object.freeze({ "user-agent": BROWSER_UA, "sec-fetch-mode": "navigate", accept: "text/html" }),
  "no-cors": Object.freeze({ "user-agent": BROWSER_UA, "sec-fetch-mode": "no-cors", accept: "*/*" }),
});

/** The TSV's columns, in order. */
export const COLUMNS = ["path", "shape", "status", "cf-mitigated", "cf-cache-status", "cf-ray", "content-type", "bytes"];

/** The pace's default: two requests a second. */
const PACE_MS = 500;
/** The most requests one run may make. */
const MAX_COUNT = 1000;
/** The longest pace a run may ask for. */
const MAX_PACE_MS = 60_000;
/** How long one request may take before it is an error row. */
const REQUEST_TIMEOUT_MS = 15_000;
/** The hosts plain `http` is taken for: a dry run against the local binary. */
const LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"]);

/** The shape's headers, as a copy the caller may keep. */
export function requestHeaders(shape) {
  if (!Object.hasOwn(SHAPES, shape)) throw new Error(`probe-burst: no shape ${JSON.stringify(shape)}`);
  return { ...SHAPES[shape] };
}

/** `hex`, checked: sixteen lowercase hex digits. */
function hexOf(hex) {
  if (!/^[0-9a-f]{16}$/.test(hex)) throw new Error(`probe-burst: the random path's hex is sixteen lowercase hex digits — got ${JSON.stringify(hex)}`);
  return hex;
}

/** The default mode's paths: the scanner's five, the random one from `hex`.
 *  None is under an allowed prefix — the reading whose pass is zero. */
export function defaultPaths(hex) {
  return ["/.env", "/wp-login.php", "/admin", "/xmlrpc.php", `/${hexOf(hex)}.json`];
}

/** `--under-prefix`'s paths: the two misses under the rule's allowed
 *  prefixes, from `hex` — the residual's reading, never mixed with the
 *  scanner's. */
export function underPrefixPaths(hex) {
  return [`/assets/${hexOf(hex)}.js`, `/art/${hexOf(hex)}.png`];
}

/** A whole number in `[lo, hi]` from the flag's value, or a refusal naming
 *  the flag. */
function whole(flag, raw, lo, hi, what) {
  if (raw === undefined || !/^\d+$/.test(raw) || Number(raw) < lo || Number(raw) > hi) {
    throw new Error(`probe-burst: ${flag} takes ${what}, a whole number from ${lo} to ${hi} — got ${JSON.stringify(raw ?? null)}`);
  }
  return Number(raw);
}

/** The origin, checked: `https` with no path, query, fragment or credentials;
 *  plain `http` only on a loopback host. */
function originOf(raw) {
  let url;
  try {
    url = new URL(raw);
  } catch {
    url = undefined;
  }
  const loopback = url?.protocol === "http:" && LOOPBACK.has(url.hostname);
  if (!url || !(url.protocol === "https:" || loopback)) {
    throw new Error(`probe-burst: the first argument is the deployment's https origin (plain http on a loopback host) — got ${JSON.stringify(raw)}`);
  }
  if (url.pathname !== "/" || url.search !== "" || url.hash !== "" || url.username !== "" || url.password !== "" || raw.endsWith("/")) {
    throw new Error(`probe-burst: the origin takes no path, query or fragment and no credentials — got ${JSON.stringify(raw)}`);
  }
  return url.origin;
}

/**
 * The command line: `<origin> [--under-prefix | --paths a,b,c] [--count N]
 * [--pace ms]`, as `{ origin, mode, count, pace, paths }` — `mode` is
 * `scanner` (the default), `under-prefix` or `paths`. `hex` names the run's
 * random paths. Every flag at most once, `--under-prefix` and `--paths` never
 * together; anything else is refused rather than ignored.
 */
export function probeArgs(argv, hex) {
  if (argv.length === 0) throw new Error(USAGE);
  const origin = originOf(argv[0]);
  const seen = new Set();
  let count;
  let pace = PACE_MS;
  let paths;
  for (let i = 1; i < argv.length; i += 1) {
    const flag = argv[i];
    if (flag !== "--count" && flag !== "--pace" && flag !== "--paths" && flag !== "--under-prefix") {
      throw new Error(`probe-burst: unexpected argument ${JSON.stringify(flag)} — ${USAGE}`);
    }
    if (seen.has(flag)) throw new Error(`probe-burst: ${flag} given twice`);
    seen.add(flag);
    if (seen.has("--under-prefix") && seen.has("--paths")) {
      throw new Error("probe-burst: --under-prefix does not mix with --paths — each is its own reading, in its own minute");
    }
    if (flag === "--under-prefix") continue;
    const raw = argv[i + 1];
    i += 1;
    if (flag === "--count") count = whole("--count", raw, 1, MAX_COUNT, "the requests to make");
    else if (flag === "--pace") pace = whole("--pace", raw, 0, MAX_PACE_MS, "the milliseconds between requests");
    else {
      const list = (raw ?? "").split(",");
      if (raw === undefined || raw === "" || list.some((p) => !/^\/[^\s#]*$/.test(p))) {
        throw new Error(`probe-burst: --paths takes absolute paths, comma-separated, with no space or fragment — got ${JSON.stringify(raw ?? null)}`);
      }
      paths = list;
    }
  }
  const mode = seen.has("--under-prefix") ? "under-prefix" : paths ? "paths" : "scanner";
  if (mode === "under-prefix") paths = underPrefixPaths(hex);
  paths ??= defaultPaths(hex);
  return { origin, mode, count: count ?? paths.length * Object.keys(SHAPES).length, pace, paths };
}

/** Which path and shape each of `count` requests is: every shape of a path,
 *  then the next path, wrapping. */
export function plan(paths, count) {
  const shapes = Object.keys(SHAPES);
  return Array.from({ length: count }, (_, i) => [paths[Math.floor(i / shapes.length) % paths.length], shapes[i % shapes.length]]);
}

/** One TSV cell: `-` for nothing, and no tab or line break inside. */
const cell = (v) => (v === null || v === undefined || v === "" ? "-" : String(v).replace(/[\t\r\n]+/g, " "));

/** One row from an answer — `{ status, headers, bytes }`, `headers` anything
 *  with `get(name)`: a `Headers`, or a `Response`'s. */
export function tsvRow(path, shape, answer) {
  const h = (name) => answer.headers.get(name);
  return [path, shape, answer.status, h("cf-mitigated"), h("cf-cache-status"), h("cf-ray"), h("content-type"), answer.bytes]
    .map(cell)
    .join("\t");
}

/** The row of a request that got no answer. */
export function errorRow(path, shape) {
  return [path, shape, "error", null, null, null, null, null].map(cell).join("\t");
}

/**
 * One GET of `url` with exactly `headers`, through `request` — `node:http`'s
 * or `node:https`'s by the URL's scheme unless a cell passes its own — and
 * its answer as `{ status, headers: Headers, bytes }`. The first answer is
 * the one returned: `http.request` follows no redirect.
 */
export function send(url, headers, request = new URL(url).protocol === "http:" ? httpRequest : httpsRequest) {
  return new Promise((resolve, reject) => {
    const req = request(url, { method: "GET", headers }, (res) => {
      let bytes = 0;
      res.on("data", (chunk) => {
        bytes += chunk.length;
      });
      res.on("error", reject);
      res.on("end", () => {
        const answered = new Headers();
        for (const [name, value] of Object.entries(res.headers)) {
          for (const v of Array.isArray(value) ? value : [value]) if (v !== undefined) answered.append(name, String(v));
        }
        resolve({ status: res.statusCode ?? 0, headers: answered, bytes });
      });
    });
    req.setTimeout(REQUEST_TIMEOUT_MS, () => req.destroy(new Error(`no answer in ${REQUEST_TIMEOUT_MS / 1000} s`)));
    req.on("error", reject);
    req.end();
  });
}

/**
 * The run: every planned request in turn, `pace` milliseconds between the
 * starts of two, each row printed through `io.out` as it lands. Answers how
 * many got no answer. `io` is the outside world — `request`, `sleep`, `now`,
 * `out`, `err` — so the cells run it over fakes.
 */
export async function probeBurst({ origin, mode, count, pace, paths }, io) {
  const { request, sleep, now, out, err } = io;
  const steps = plan(paths, count);
  err(`probe-burst: mode ${mode} — ${READING[mode]}`);
  err(`probe-burst: ${count} requests to ${origin}, ${pace} ms apart, over ${paths.join(" ")}`);
  err(`probe-burst: started ${new Date(now()).toISOString()}`);
  out(COLUMNS.join("\t"));
  const tally = new Map();
  let failed = 0;
  for (let i = 0; i < steps.length; i += 1) {
    const [path, shape] = steps[i];
    const started = now();
    let key;
    try {
      const answer = await send(`${origin}${path}`, requestHeaders(shape), request);
      out(tsvRow(path, shape, answer));
      key = `${answer.status}${answer.headers.get("cf-mitigated") === null ? "" : ` ${answer.headers.get("cf-mitigated")}`} ${shape}`;
    } catch (e) {
      failed += 1;
      out(errorRow(path, shape));
      err(`probe-burst: ${shape} ${path} got no answer — ${e instanceof Error ? e.message : String(e)}`);
      key = `error ${shape}`;
    }
    tally.set(key, (tally.get(key) ?? 0) + 1);
    if (i < steps.length - 1) await sleep(Math.max(0, pace - (now() - started)));
  }
  err(`probe-burst: ended ${new Date(now()).toISOString()}`);
  for (const [key, n] of [...tally].sort(([a], [b]) => a.localeCompare(b))) err(`probe-burst: ${n} × ${key}`);
  return failed;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  let args;
  try {
    args = probeArgs(process.argv.slice(2), randomBytes(8).toString("hex"));
  } catch (e) {
    console.error(e instanceof Error ? e.message : String(e));
    process.exit(2);
  }
  const failed = await probeBurst(args, {
    sleep: (ms) => new Promise((r) => setTimeout(r, ms)),
    now: () => Date.now(),
    out: (line) => process.stdout.write(`${line}\n`),
    err: (line) => process.stderr.write(`${line}\n`),
  });
  process.exit(failed === 0 ? 0 : 1);
}
