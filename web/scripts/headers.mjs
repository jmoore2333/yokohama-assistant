// **The header set both hosts serve, generated ONCE where the build's values are known** (security S2a; spec §2.2
// B4, D13 and "the CSP"; the c1 review's §"The header set to recommend"; decision D-1).
//
// This file is the one place a directive is spelled. `vite build` calls `headerSet` with the checked build values
// (`vite.config.ts`) and writes the result to `dist/headers.json`; the Worker's assembly appends it to `_headers`
// as the `/*` block the asset layer applies to every static answer (`worker/scripts/assemble-assets.mjs`), and
// the NAS binary reads the file at start and sets every pair on every answer (`crates/yokohama-web/src/assets.rs`,
// `response_headers`). Neither host holds a copy of the policy's text, so the two cannot drift.
//
// **Why each source is there** was checked against the built bundle by the review: `'wasm-unsafe-eval'` because
// the module Worker compiles the engine with `WebAssembly.instantiateStreaming`; no `'unsafe-eval'` and no
// `'unsafe-inline'`, because the bundle holds no `eval` and the shell no inline script (its JSON-LD block is a
// data block, which `script-src` does not govern); `connect-src 'self'` covers `/api`, `/art`, the precache and
// the Turnstile pre-clearance's fetch to `/cdn-cgi/`; the site's `wss://` host is the Worker's socket (CSP3
// reads `'self'` as covering it, and the explicit source keeps older browsers honest); the release feed's ORIGIN
// is the door's one cross-origin fetch; the 404 page's inline style is admitted by its hash (D-4), so the page
// stays self-contained and the policy strict.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

/** The policy's mode, a CONSTANT and not a build value (decision D-1): `report-only` ships
 *  `Content-Security-Policy-Report-Only` — this release, 0.6.2, one release in the wild whose violations are read
 *  in the window and the Playwright tier (D-9) — and `enforce` ships `Content-Security-Policy`, which S2's wrap
 *  flips to, with the cell beside it. A build value would let a deploy weaken the policy without a commit. */
export const CSP_MODE = "report-only";

/** The two names the policy ships under, by mode. */
export const CSP_HEADER = Object.freeze({
  "report-only": "Content-Security-Policy-Report-Only",
  enforce: "Content-Security-Policy",
});

/** Cloudflare's hosts, named because both deployments sit behind the zone (decision D-2): `challenges` is
 *  Turnstile's widget and the Managed Challenge (a script and an iframe); `static.cloudflareinsights.com` is the
 *  Web Analytics beacon the zone injects into every HTML answer, kept on by the owner's standing rule, and
 *  `cloudflareinsights.com` is where that beacon posts. The one script this list cannot name is Bot Fight Mode's
 *  INLINE detection script, which carries no nonce a static shell could match: it is the known Report-Only
 *  line of this release, read in the window, and the enforcement decision is the wrap's. */
export const CLOUDFLARE = Object.freeze({
  script: Object.freeze(["https://challenges.cloudflare.com", "https://static.cloudflareinsights.com"]),
  frame: Object.freeze(["https://challenges.cloudflare.com"]),
  connect: Object.freeze(["https://cloudflareinsights.com"]),
});

/**
 * The CSP hash of every inline `<style>` in `html`, in document order: `'sha256-<base64>'` over EXACTLY the
 * element's text — the bytes between `<style…>` and `</style>`, the tags excluded — after the HTML parser's
 * newline normalization (CR and CRLF read as LF), which is what a browser hashes when it meets the element
 * (decision D-4; `web/public/404.html` is the one page that has one). The normalization is what keeps an
 * `autocrlf` checkout's build from hashing bytes no browser hashes (measured: a CRLF page is styled under the LF
 * hash and blocked under its own).
 *
 * @param {string} html
 * @returns {string[]}
 */
export function styleHashesOf(html) {
  return [...html.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/g)].map(
    (m) => `'sha256-${createHash("sha256").update(m[1].replace(/\r\n?/g, "\n"), "utf8").digest("base64")}'`,
  );
}

/**
 * The policy's directives, in one order, as one header value.
 *
 * @param {{ siteUrl: string | null, releaseFeed: string | null, styleHashes: readonly string[] }} values
 *   `siteUrl` and `releaseFeed` as `build-values.mjs`'s checkers answer them (an origin; a feed URL or `null`).
 * @returns {string}
 */
export function cspOf({ siteUrl, releaseFeed, styleHashes }) {
  const connect = ["'self'"];
  if (siteUrl !== null) connect.push(`wss://${new URL(siteUrl).host}`);
  if (releaseFeed !== null) connect.push(new URL(releaseFeed).origin);
  connect.push(...CLOUDFLARE.connect);
  return [
    "default-src 'self'",
    `script-src 'self' 'wasm-unsafe-eval' ${CLOUDFLARE.script.join(" ")}`,
    ["style-src 'self'", ...styleHashes].join(" "),
    "img-src 'self'",
    "font-src 'self'",
    `connect-src ${connect.join(" ")}`,
    `frame-src ${CLOUDFLARE.frame.join(" ")}`,
    "worker-src 'self'",
    "manifest-src 'self'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
  ].join("; ");
}

/**
 * The set, as `[name, value]` pairs in the order both hosts send them: the policy under its mode's name, then
 * the five companions — `X-Frame-Options: DENY` (enforced from this release, whatever the policy's mode: it is
 * what closes B4 while `frame-ancestors` is only reported), `nosniff` (the zone adds it at the edge for answers
 * the edge serves; the service worker's synthesized navigation needs it from the stored entry), the referrer
 * policy, a Permissions-Policy that turns off what the app never asks for, and COOP `same-origin` (the app opens
 * no popups; Access's login is a top-level redirect and Turnstile an iframe).
 *
 * @param {{ siteUrl: string | null, releaseFeed: string | null, styleHashes: readonly string[], mode?: string }} values
 * @returns {[string, string][]}
 */
export function headerSet({ siteUrl, releaseFeed, styleHashes, mode = CSP_MODE }) {
  if (!Object.hasOwn(CSP_HEADER, mode)) {
    throw new Error(`headers: the mode is report-only or enforce — got ${JSON.stringify(mode)}`);
  }
  return [
    [CSP_HEADER[mode], cspOf({ siteUrl, releaseFeed, styleHashes })],
    ["X-Frame-Options", "DENY"],
    ["X-Content-Type-Options", "nosniff"],
    ["Referrer-Policy", "strict-origin-when-cross-origin"],
    ["Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=(), usb=()"],
    ["Cross-Origin-Opener-Policy", "same-origin"],
  ];
}

/** The `/*` block `_headers` carries, in the asset layer's grammar: the path line, then each header on its own
 *  two-space-indented line, one trailing newline. Appended by the Worker's assembly, never written by hand.
 *
 *  @param {readonly (readonly [string, string])[]} set
 *  @returns {string} */
export function headersBlock(set) {
  return `${["/*", ...set.map(([name, value]) => `  ${name}: ${value}`)].join("\n")}\n`;
}

/** `dist/headers.json`'s text: the pairs, pretty-printed, one trailing newline.
 *  @param {readonly (readonly [string, string])[]} set
 *  @returns {string} */
export function headersJson(set) {
  return `${JSON.stringify(set, null, 2)}\n`;
}

/** A header name: RFC 9110's token grammar, which is what `http::HeaderName` accepts on the NAS. */
const HEADER_NAME = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;

/** `http::HeaderName`'s length bound (`MAX_HEADER_NAME_LEN`, 65,535 bytes — a token is ASCII, so characters):
 *  a longer name would pass the assembly and stop the NAS at start. */
const HEADER_NAME_MAX = 65_535;

/** A control character in a value (`\u0000`–`\u001f`, `\u007f`): what would let one pair render as two
 *  `_headers` lines. Stricter than `http::HeaderValue`, which admits a tab (`http` 1.5.0's `is_valid`:
 *  `b >= 32 && b != 127 || b == b'\t'`) — the strict side is this one, deliberately (see `parseHeadersJson`). */
const CONTROL = /[\u0000-\u001f\u007f]/;

/** `dist/headers.json` read back: an array of at least one `[string, string]` pair — each name a token of at
 *  most 65,535 bytes, each value non-empty and free of control characters, every string well-formed — or a throw
 *  naming the file. The Worker's assembly and the guard read it through this, as the NAS's `response_headers`
 *  reads it in Rust (`serde_json`, then `http::HeaderName::from_bytes` and `HeaderValue::from_str`), and this
 *  side refuses EVERYTHING that side refuses — the same name grammar and its length bound; a lone surrogate,
 *  whose escape `JSON.parse` admits and `serde_json` does not; a pair of any other arity — and two value shapes
 *  `http` admits besides, a tab and an empty value: so a file that passes the assembly always passes the NAS's
 *  reader at start, and the one-host-down release (passed the assembly, stopped the NAS) cannot arise. Measured
 *  by Task 1's review against `http` 1.5.0 and `serde_json` 1.0.150; the build writes none of these shapes. And a
 *  name the file repeats, compared case-insensitively, is refused on BOTH sides (S2a's after review): the NAS's
 *  layer `insert`s, so it would send the last pair, and the Worker's `_headers` would send the values joined with
 *  a comma (Cloudflare's documented rule for a header applied twice), so one file would read two ways. A FILE
 *  on a release's path reaches this through [`readHeadersText`], never `readFileSync(.., "utf8")`, which would put
 *  U+FFFD where the NAS's `std::fs::read_to_string` refuses the byte (Task 1's fix round, C-2). The two precache
 *  cells read their own build's file with `readFileSync`: they are cells, not the release's path, and the build
 *  writes valid UTF-8 and no BOM.
 *  @param {string} text
 *  @returns {[string, string][]} */
export function parseHeadersJson(text) {
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("headers.json does not parse");
  }
  const pair = (p) =>
    Array.isArray(p) &&
    p.length === 2 &&
    p.every((s) => typeof s === "string" && s !== "" && s.isWellFormed()) &&
    p[0].length <= HEADER_NAME_MAX &&
    HEADER_NAME.test(p[0]) &&
    !CONTROL.test(p[1]);
  if (!Array.isArray(parsed) || parsed.length === 0 || !parsed.every(pair)) {
    throw new Error(
      "headers.json is not the client build's header set — an array of [name, value] pairs, each name a token of at most 65,535 bytes, each value non-empty and free of control characters, every string well-formed",
    );
  }
  const seen = new Set();
  for (const [name] of parsed) {
    const key = name.toLowerCase();
    if (seen.has(key)) throw new Error(`headers.json names ${name} twice`);
    seen.add(key);
  }
  return parsed.map(([name, value]) => [name, value]);
}

/** `dist/headers.json`'s TEXT, read from `file` with its bytes decoded STRICTLY — a `TextDecoder` with `fatal`, and
 *  `ignoreBOM`, since the decoder's DEFAULT strips a leading BOM — so an invalid UTF-8 byte is a throw naming the
 *  file, and a BOM reaches `parseHeadersJson`, which refuses it: both are what the NAS refuses, the byte at its
 *  `std::fs::read_to_string` and the BOM at `serde_json`'s parse. `readFileSync(.., "utf8")` keeps a BOM too, but
 *  substitutes U+FFFD for the byte, hiding what the NAS refuses (measured on Node 26 and serde_json 1.0.150). Every
 *  JavaScript read of the file on a release's path — the Worker's assembly, the deploy guard, the e2e build's
 *  override — goes through this; the NAS reads it in Rust — so the "passes the assembly, passes the NAS" sentence
 *  above holds at the file, not only at its text (Task 1's fix round, C-2). A missing file throws as
 *  `readFileSync` does; the callers test for the file first.
 *  @param {string | URL} file
 *  @returns {string} */
export function readHeadersText(file) {
  const bytes = readFileSync(file);
  try {
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch {
    throw new Error(`headers.json is not valid UTF-8 — ${file}`);
  }
}
