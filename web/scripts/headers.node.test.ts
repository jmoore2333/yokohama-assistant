// **The header set both hosts serve, generated once** (security S2a; spec §2.2 B4 and "the CSP"; the c1 review's
// §"The header set to recommend"; decisions D-1, D-2, D-4). These cells pin the set's SHAPE — its names and their
// order, every directive and its sources, what the build values add, what the 404 page's style hashes to — so a
// directive loosened by hand, or a host added without its reason, fails here before either host serves it.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  CLOUDFLARE,
  CSP_HEADER,
  CSP_MODE,
  cspOf,
  headerSet,
  headersBlock,
  headersJson,
  parseHeadersJson,
  styleHashesOf,
} from "./headers.mjs";

/** No build value set, no inline style: the smallest policy. */
const NONE = { siteUrl: null, releaseFeed: null, styleHashes: [] as string[] };

/** A policy as `directive → its source list`, in the policy's own order. */
const directives = (csp: string): Map<string, string[]> =>
  new Map(
    csp.split("; ").map((d) => {
      const [name, ...sources] = d.split(" ");
      return [name!, sources];
    }),
  );

describe("the policy", () => {
  it("ships Report-Only in this release, under the mode's own header name (X2: the wrap flips it)", () => {
    expect(CSP_MODE).toBe("report-only");
    expect(headerSet(NONE)[0]![0]).toBe("Content-Security-Policy-Report-Only");
    expect(headerSet({ ...NONE, mode: "enforce" })[0]![0]).toBe("Content-Security-Policy");
    expect(CSP_HEADER).toEqual({ "report-only": "Content-Security-Policy-Report-Only", enforce: "Content-Security-Policy" });
    expect(() => headerSet({ ...NONE, mode: "report" as never })).toThrow(/report-only or enforce/);
  });

  it("names every directive once, in one order, with the sources the c1 review derived from the built bundle", () => {
    const d = directives(cspOf(NONE));
    expect([...d.keys()]).toEqual([
      "default-src", "script-src", "style-src", "img-src", "font-src", "connect-src", "frame-src",
      "worker-src", "manifest-src", "object-src", "base-uri", "form-action", "frame-ancestors",
    ]);
    expect(d.get("default-src")).toEqual(["'self'"]);
    // `'wasm-unsafe-eval'`: the module Worker compiles the engine (`assets/worker-*.js`).
    expect(d.get("script-src")).toEqual(["'self'", "'wasm-unsafe-eval'", ...CLOUDFLARE.script]);
    expect(d.get("style-src")).toEqual(["'self'"]);
    expect(d.get("img-src")).toEqual(["'self'"]);
    expect(d.get("font-src")).toEqual(["'self'"]);
    expect(d.get("connect-src")).toEqual(["'self'", ...CLOUDFLARE.connect]);
    expect(d.get("frame-src")).toEqual([...CLOUDFLARE.frame]);
    expect(d.get("worker-src")).toEqual(["'self'"]);
    expect(d.get("manifest-src")).toEqual(["'self'"]);
    expect(d.get("object-src")).toEqual(["'none'"]);
    expect(d.get("base-uri")).toEqual(["'none'"]);
    expect(d.get("form-action")).toEqual(["'self'"]);
    expect(d.get("frame-ancestors")).toEqual(["'none'"]);
  });

  it("never carries 'unsafe-inline' or 'unsafe-eval', whatever the values", () => {
    const full = {
      siteUrl: "https://yokohama.moore.nyc",
      releaseFeed: "https://api.github.com/repos/o/r/releases/latest",
      styleHashes: ["'sha256-abc='"],
    };
    // A QUOTED keyword: `'wasm-unsafe-eval'` is another keyword, and required — a bare substring test is red
    // against this very policy.
    for (const values of [NONE, full]) expect(cspOf(values)).not.toMatch(/'unsafe-(?:inline|eval)'/);
  });

  it("derives connect-src from the build values: the site's socket host, and the feed's ORIGIN, never its path", () => {
    const d = directives(
      cspOf({
        siteUrl: "https://yokohama.moore.nyc",
        releaseFeed: "https://api.github.com/repos/jmoore2333/yokohama-assistant/releases/latest",
        styleHashes: [],
      }),
    );
    expect(d.get("connect-src")).toEqual(["'self'", "wss://yokohama.moore.nyc", "https://api.github.com", ...CLOUDFLARE.connect]);
    const other = directives(cspOf({ siteUrl: null, releaseFeed: "https://feed.example.test/latest.json", styleHashes: [] }));
    expect(other.get("connect-src")).toEqual(["'self'", "https://feed.example.test", ...CLOUDFLARE.connect]);
    // `checkSiteUrl` admits a port, and the socket source keeps it (`.host`, not `.hostname`).
    const ported = directives(cspOf({ siteUrl: "https://yokohama.test:8443", releaseFeed: null, styleHashes: [] }));
    expect(ported.get("connect-src")).toEqual(["'self'", "wss://yokohama.test:8443", ...CLOUDFLARE.connect]);
  });

  it("names Cloudflare's three hosts, and no other host outside 'self' (decision D-2)", () => {
    expect(CLOUDFLARE).toEqual({
      script: ["https://challenges.cloudflare.com", "https://static.cloudflareinsights.com"],
      frame: ["https://challenges.cloudflare.com"],
      connect: ["https://cloudflareinsights.com"],
    });
    const hosts = [...cspOf(NONE).matchAll(/https:\/\/[a-z0-9.-]+/g)].map((m) => m[0]).sort();
    expect(hosts).toEqual([
      "https://challenges.cloudflare.com", "https://challenges.cloudflare.com",
      "https://cloudflareinsights.com", "https://static.cloudflareinsights.com",
    ]);
  });
});

describe("the inline style of the 404 page (decision D-4)", () => {
  it("hashes exactly the text between <style> and </style>, as a browser does, one hash per element", () => {
    const text = "\n  body { color: red }\n";
    const html = `<!doctype html><style>${text}</style><p>x</p>`;
    expect(styleHashesOf(html)).toEqual([`'sha256-${createHash("sha256").update(text, "utf8").digest("base64")}'`]);
    expect(styleHashesOf("<p>none</p>")).toEqual([]);
    expect(styleHashesOf("<style>a{}</style><style>b{}</style>")).toHaveLength(2);
    // The HTML parser normalizes CR and CRLF to LF before the element's text exists, so a browser hashes the LF
    // form whatever the file's line endings — an `autocrlf` checkout must hash to the same (measured in Chromium:
    // a CRLF page is styled under the LF hash and blocked under its own).
    expect(styleHashesOf("<style>\r\n  a{}\r\n</style>")).toEqual(styleHashesOf("<style>\n  a{}\n</style>"));
  });

  it("names the committed 404 page's one style block in style-src", () => {
    const page = readFileSync(new URL("../public/404.html", import.meta.url), "utf8");
    const hashes = styleHashesOf(page);
    expect(hashes, "web/public/404.html carries exactly one <style>").toHaveLength(1);
    expect(hashes[0]).toMatch(/^'sha256-[A-Za-z0-9+/]+=*'$/);
    expect(directives(cspOf({ ...NONE, styleHashes: hashes })).get("style-src")).toEqual(["'self'", hashes[0]]);
  });
});

describe("the set, its block and its file", () => {
  it("is six headers in one order, the five companions at the c1 review's values", () => {
    expect(headerSet(NONE).map(([name]) => name)).toEqual([
      "Content-Security-Policy-Report-Only", "X-Frame-Options", "X-Content-Type-Options",
      "Referrer-Policy", "Permissions-Policy", "Cross-Origin-Opener-Policy",
    ]);
    const values = Object.fromEntries(headerSet(NONE));
    expect(values["X-Frame-Options"]).toBe("DENY");
    expect(values["X-Content-Type-Options"]).toBe("nosniff");
    expect(values["Referrer-Policy"]).toBe("strict-origin-when-cross-origin");
    expect(values["Permissions-Policy"]).toBe("camera=(), microphone=(), geolocation=(), payment=(), usb=()");
    expect(values["Cross-Origin-Opener-Policy"]).toBe("same-origin");
  });

  it("renders the _headers block the asset layer parses: a /* line, then two-space-indented Name: value lines", () => {
    const lines = headersBlock(headerSet(NONE)).split("\n");
    expect(lines[0]).toBe("/*");
    expect(lines.slice(1, 7).every((l) => /^  [A-Za-z-]+: \S/.test(l))).toBe(true);
    expect(lines[1]).toBe(`  Content-Security-Policy-Report-Only: ${cspOf(NONE)}`);
    expect(lines.at(-1), "ends with one newline").toBe("");
    expect(lines).toHaveLength(8);
  });

  it("writes the file as ordered pairs and reads it back, and refuses any other shape by name", () => {
    const set = headerSet(NONE);
    expect(parseHeadersJson(headersJson(set))).toEqual(set);
    expect(headersJson(set).endsWith("\n")).toBe(true);
    for (const bad of [
      "{}", "[[1, 2]]", '[["X-Frame-Options"]]', '["X-Frame-Options: DENY"]', "nonsense", "[]",
      // What the Rust reader refuses, refused here too — one file must not pass the Worker's assembly and stop the
      // NAS at start: a name outside the token grammar; a value with a control character, which `headersBlock`
      // would render as a second `_headers` rule.
      '[["X Frame", "DENY"]]', '[["Bad Name:", "v"]]', '[["X-Frame-Options", "DE\\u0000NY"]]',
      '[["X-Note", "ok\\n  X-Frame-Options: ALLOWALL"]]',
    ]) {
      expect(() => parseHeadersJson(bad), bad).toThrow(/headers\.json/);
    }
  });
});
