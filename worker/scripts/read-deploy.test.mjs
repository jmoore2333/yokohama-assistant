import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  checkEntry,
  credentialFor,
  cutAtQuery,
  cutUrlsIn,
  edgeInjections,
  essence,
  get,
  locationOf,
  parseHeaderFile,
  precacheList,
  requestLog,
} from "./read-deploy.mjs";

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
const enc = (text) => new TextEncoder().encode(text);

const INDEX = "<!doctype html><html><body><div id=root></div></body></html>";
const D1 = "a".repeat(64);
const D2 = "b".repeat(64);

describe("precacheList", () => {
  // The list as `web/scripts/precache.mjs` prints it into `sw.js`.
  it("reads every entry's path, digest and type", () => {
    const sw = `var define_PRECACHE_default = [{ path: "/index.html", digest: "${D1}", type: "text/html" }, { path: "/art/x.webp", digest: "${D2}", type: "image/webp" }];\nclients.matchAll({ type: "window" });`;
    expect(precacheList(sw)).toEqual([
      { path: "/index.html", digest: D1, type: "text/html" },
      { path: "/art/x.webp", digest: D2, type: "image/webp" },
    ]);
  });
  // The count is the anti-vacuity: a list whose printed shape drifted must not
  // read as a shorter list.
  it("refuses a worker with no entry, or a digest it could not pair", () => {
    expect(() => precacheList("self.addEventListener('install', () => {});")).toThrow(/no precache entries/);
    const drifted = `[{ path: "/index.html", digest: "${D1}", type: "text/html" }, { path: "/a", digest: "${D2}", kind: "x" }]`;
    expect(() => precacheList(drifted)).toThrow(/2 digests but read 1 entries/);
  });
});

describe("essence", () => {
  it("drops the parameters and the case", () => {
    expect(essence("Text/HTML; charset=utf-8")).toBe("text/html");
    expect(essence(null)).toBe("");
  });
});

// The credential's header file (RUNBOOK §6): `Name: value` per line. The
// value is the secret, so no error may carry it.
describe("parseHeaderFile", () => {
  it("reads Name: value lines, skipping blank ones", () => {
    expect(parseHeaderFile("CF-Access-Client-Id: id.access\n\nCF-Access-Client-Secret: s3cr3t\n")).toEqual([
      ["CF-Access-Client-Id", "id.access"],
      ["CF-Access-Client-Secret", "s3cr3t"],
    ]);
  });
  it("refuses a line that is not a header, naming its NUMBER and never its text", () => {
    let message = "";
    try {
      parseHeaderFile("CF-Access-Client-Id: id.access\nCF-Access-Client-Secret=s3cr3t\n");
    } catch (e) {
      message = e.message;
    }
    expect(message).toBe("the header file's line 2 is not `Name: value`");
    expect(message).not.toContain("s3cr3t");
  });
  it("refuses an empty file", () => {
    expect(() => parseHeaderFile("\n")).toThrow("the header file holds no header");
  });
});

// Spec §0.3 row 8: the zone's JavaScript detections inject a script into HTML
// that a plain fetch reads, and Web Analytics may inject its beacon. The
// reading names each, and says whether the body is the release's once they
// are cut away — never a silent pass, never a false alarm.
describe("edgeInjections", () => {
  it("cuts the challenge-platform and beacon scripts, naming each", () => {
    const injected = INDEX.replace(
      "</body>",
      '<script>(function(){var s=document.createElement("script");s.src="/cdn-cgi/challenge-platform/scripts/jsd/main.js";})();</script>' +
        '<script defer src="https://static.cloudflareinsights.com/beacon.min.js" data-cf-beacon="{}"></script></body>',
    );
    expect(edgeInjections(injected)).toEqual({ text: INDEX, found: ["challenge-platform", "cloudflareinsights"] });
  });
  it("leaves a page's own scripts alone", () => {
    const own = INDEX.replace("</body>", '<script type="module" src="/assets/index.js"></script></body>');
    expect(edgeInjections(own)).toEqual({ text: own, found: [] });
  });
});

describe("checkEntry", () => {
  const entry = { path: "/index.html", digest: sha256(enc(INDEX)), type: "text/html" };
  const answer = (overrides = {}) => ({ status: 200, contentType: "text/html; charset=utf-8", body: enc(INDEX), ...overrides });

  it("passes a 200 of the listed type whose body hashes to the listed digest", () => {
    expect(checkEntry(entry, answer())).toEqual({ ok: true, reasons: [] });
  });
  it("names a wrong status, a wrong type and a wrong body", () => {
    expect(checkEntry(entry, answer({ status: 307, contentType: null, body: enc("") }))).toEqual({
      ok: false,
      reasons: ["answered 307", "answered no content type, not text/html", `its body is ${sha256(enc(""))}`],
    });
  });
  // The edge's rewrite of HTML is named for what it is, and the verdict is the
  // release's bytes once the injections are cut: `ok` stays false, so the
  // reading never counts it byte-exact.
  it("names an HTML body the edge rewrote, and whether the release is underneath", () => {
    const injected = INDEX.replace("</body>", '<script>/cdn-cgi/challenge-platform/x</script></body>');
    expect(checkEntry(entry, answer({ body: enc(injected) }))).toEqual({
      ok: false,
      reasons: [`its body is ${sha256(enc(injected))}`],
      edge: { found: ["challenge-platform"], releaseUnderneath: true },
    });
    const other = "<!doctype html><p>not the release</p>" + '<script>/cdn-cgi/challenge-platform/x</script>';
    expect(checkEntry(entry, answer({ body: enc(other) })).edge).toEqual({
      found: ["challenge-platform"],
      releaseUnderneath: false,
    });
  });
});

// The credential goes to the deployment's own origin and nowhere else: sent to
// every origin it would reach the Web Analytics beacon's hosts, and add a CORS
// preflight that is itself a console error. Mutant: a host or prefix compare.
describe("credentialFor", () => {
  const pairs = [["CF-Access-Client-Id", "id.access"], ["CF-Access-Client-Secret", "s3cr3t"]];
  const origin = "https://yokohama.moore.nyc";
  it("answers the headers for the deployment's own origin", () => {
    expect(credentialFor("https://yokohama.moore.nyc/assets/x.js", origin, pairs)).toEqual({
      "CF-Access-Client-Id": "id.access",
      "CF-Access-Client-Secret": "s3cr3t",
    });
  });
  it.each([
    "https://static.cloudflareinsights.com/beacon.min.js",
    "https://yokohama.moore.nyc.evil.test/x",
    "https://nas.yokohama.moore.nyc/x",
    "https://yokohama.moore.nyc:8443/x",
    "http://yokohama.moore.nyc/x",
  ])("answers nothing for %s", (url) => {
    expect(credentialFor(url, origin, pairs)).toBeNull();
  });
  it("answers nothing without a credential", () => {
    expect(credentialFor("https://yokohama.moore.nyc/", origin, [])).toBeNull();
  });
});

// M8 (Task 11's review): every fetch the script makes itself goes through
// `credentialFor`, so the header file reaches the deployment's origin however
// a path is written. Mutant: `get` sending the file's headers unconditionally.
describe("get", () => {
  const pairs = [["CF-Access-Client-Id", "id.access"], ["CF-Access-Client-Secret", "s3cr3t"]];
  /** A fetcher that records what it was asked, and answers 200. */
  function recording() {
    const asked = [];
    const fetcher = async (url, init) => {
      asked.push({ url: String(url), headers: init.headers, redirect: init.redirect });
      return new Response("ok");
    };
    return { asked, fetcher };
  }
  it("sends the credential to the base's origin, following no redirect", async () => {
    const { asked, fetcher } = recording();
    await get("https://yokohama.moore.nyc", "/sw.js", pairs, fetcher);
    expect(asked).toEqual([
      {
        url: "https://yokohama.moore.nyc/sw.js",
        headers: { "CF-Access-Client-Id": "id.access", "CF-Access-Client-Secret": "s3cr3t" },
        redirect: "manual",
      },
    ]);
  });
  it.each(["//evil.test/x", "https://evil.test/x"])("sends nothing of it where %s resolves to another origin", async (path) => {
    const { asked, fetcher } = recording();
    await get("https://yokohama.moore.nyc", path, pairs, fetcher);
    expect(asked[0].headers).toEqual({});
  });
});

// M9 (Task 11's review): CDP reuses a request's id across its redirects, so
// the log keeps ONE record per HOP — each `redirectResponse` closes the hop
// before it, with its own status and headers — and the reading can name the
// request that failed, not only where it was sent. Mutant: records keyed by
// the id alone, so the 3xx hop's status and headers are lost.
describe("requestLog", () => {
  const sent = (requestId, url, redirectResponse) => ({
    requestId,
    request: { url, method: "POST" },
    type: "Fetch",
    initiator: { type: "script", stack: { callFrames: [{ url: "https://site.test/beacon.js", lineNumber: 7 }] } },
    ...(redirectResponse ? { redirectResponse } : {}),
  });
  it("records each redirect hop with its own status and headers, and never a cookie", () => {
    const log = requestLog();
    log.sent(sent("1", "https://cloudflareinsights.com/cdn-cgi/rum"), 2);
    log.sent(
      sent("1", "https://bdvc.cloudflareaccess.com/login", {
        status: 302,
        headers: { location: "https://bdvc.cloudflareaccess.com/login", "set-cookie": "CF_Authorization=bearer" },
        fromServiceWorker: false,
      }),
      2,
    );
    log.failed({ requestId: "1", errorText: "net::ERR_FAILED", corsErrorStatus: { corsError: "MissingAllowOriginHeader" } });
    expect(log.records).toEqual([
      {
        load: 2,
        url: "https://cloudflareinsights.com/cdn-cgi/rum",
        method: "POST",
        type: "Fetch",
        initiator: { type: "script", url: "https://site.test/beacon.js", line: 7 },
        status: 302,
        headers: { location: "https://bdvc.cloudflareaccess.com/login" },
        fromServiceWorker: false,
        redirectedTo: "https://bdvc.cloudflareaccess.com/login",
      },
      {
        load: 2,
        url: "https://bdvc.cloudflareaccess.com/login",
        method: "POST",
        type: "Fetch",
        initiator: { type: "script", url: "https://site.test/beacon.js", line: 7 },
        redirectedFrom: "https://cloudflareinsights.com/cdn-cgi/rum",
        failed: "net::ERR_FAILED",
        blockedReason: undefined,
        corsError: { corsError: "MissingAllowOriginHeader" },
      },
    ]);
  });
  it("records a response on its request's current hop, cookies dropped", () => {
    const log = requestLog();
    log.sent(sent("7", "https://yokohama.moore.nyc/"), 1);
    log.received({ requestId: "7", response: { status: 200, headers: { "content-type": "text/html", cookie: "x", "Set-Cookie": "y" }, fromServiceWorker: true } });
    expect(log.records).toEqual([
      expect.objectContaining({ url: "https://yokohama.moore.nyc/", status: 200, headers: { "content-type": "text/html" }, fromServiceWorker: true }),
    ]);
  });
});

// N1 (Task 11's re-review; the orchestrator's ruling): Access's eager
// redirects walk a browser through the application's hostnames with its token
// in each hop's URL, so no URL the script prints goes past its `?` (or `#`),
// and a `location` header keeps its origin and path alone. Mutants: a hop's
// URL, its `redirectedTo`, its initiator's URL or its `location` kept whole —
// the token lands on stdout.
describe("what the script prints can carry no token", () => {
  it("cuts a URL at its query or its fragment", () => {
    expect(cutAtQuery("https://h.test/cdn-cgi/access/authorized?token=T#f")).toBe("https://h.test/cdn-cgi/access/authorized");
    expect(cutAtQuery("https://h.test/a#frag")).toBe("https://h.test/a");
    expect(cutAtQuery("https://h.test/a")).toBe("https://h.test/a");
  });
  // A console message quotes the URL it is about. Mutant: the text printed
  // as it came.
  it("cuts every URL inside a console message's text", () => {
    const text =
      "Access to fetch at 'https://login.test/cdn-cgi/access/authorized?token=TOKEN-3' from origin " +
      "'https://h.test' has been blocked by CORS policy; see https://h.test/x?y=TOKEN-3#z.";
    expect(cutUrlsIn(text)).toBe(
      "Access to fetch at 'https://login.test/cdn-cgi/access/authorized' from origin 'https://h.test' " +
        "has been blocked by CORS policy; see https://h.test/x",
    );
  });
  it("keeps a location's origin and path alone, resolving a relative one against its hop", () => {
    expect(locationOf("https://login.test/cdn-cgi/access/login/h?kid=K&meta=M", "https://h.test/x")).toBe(
      "https://login.test/cdn-cgi/access/login/h",
    );
    expect(locationOf("/cdn-cgi/access/authorized?token=T", "https://h.test/x?y=1")).toBe("https://h.test/cdn-cgi/access/authorized");
  });
  it("records an eager redirect chain with no token anywhere in it", () => {
    const log = requestLog();
    const hop = (url, redirectResponse) => ({
      requestId: "9",
      request: { url, method: "GET" },
      type: "Document",
      initiator: { type: "script", url: "https://h.test/page.js?session=TOKEN-9" },
      ...(redirectResponse ? { redirectResponse } : {}),
    });
    log.sent(hop("https://h.test/?from=TOKEN-9"), 1);
    log.sent(
      hop("https://cli.h.test/cdn-cgi/access/authorized?token=TOKEN-9", {
        status: 302,
        headers: { Location: "https://cli.h.test/cdn-cgi/access/authorized?token=TOKEN-9" },
      }),
      1,
    );
    log.sent(hop("https://h.test/", { status: 302, headers: { location: "/?token=TOKEN-9" } }), 1);
    log.received({ requestId: "9", response: { status: 200, headers: { location: "/next?token=TOKEN-9" } } });
    expect(JSON.stringify(log.records)).not.toContain("TOKEN-9");
    expect(
      log.records.map((r) => ({ url: r.url, to: r.redirectedTo, from: r.redirectedFrom, location: r.headers?.Location ?? r.headers?.location, initiator: r.initiator.url })),
    ).toEqual([
      {
        url: "https://h.test/",
        to: "https://cli.h.test/cdn-cgi/access/authorized",
        from: undefined,
        location: "https://cli.h.test/cdn-cgi/access/authorized",
        initiator: "https://h.test/page.js",
      },
      {
        url: "https://cli.h.test/cdn-cgi/access/authorized",
        to: "https://h.test/",
        from: "https://h.test/",
        location: "https://cli.h.test/",
        initiator: "https://h.test/page.js",
      },
      { url: "https://h.test/", to: undefined, from: "https://cli.h.test/cdn-cgi/access/authorized", location: "https://h.test/next", initiator: "https://h.test/page.js" },
    ]);
  });
});

// The script's own usage line: it runs as a command, and an argument it does
// not know is answered with the usage and exit 2, loading nothing else.
describe("the command", () => {
  it("answers --help with its usage and exit 2", () => {
    const script = fileURLToPath(new URL("./read-deploy.mjs", import.meta.url));
    const run = spawnSync(process.execPath, [script, "--help"], { encoding: "utf8" });
    expect([run.status, run.stderr.trim(), run.stdout]).toEqual([
      2,
      "usage: node scripts/read-deploy.mjs assets|install|console <base-url> [--headers <file>]",
      "",
    ]);
  });
});
