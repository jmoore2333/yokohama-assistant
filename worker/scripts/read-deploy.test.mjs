import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { checkEntry, credentialFor, edgeInjections, essence, parseHeaderFile, precacheList } from "./read-deploy.mjs";

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
