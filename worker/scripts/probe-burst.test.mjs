// The probe's cells (M6.5c2 cycle F, R-F5; spec §2.5, Review Focus 5): its
// arguments, its three shapes, its TSV, its default paths and its transport —
// and NOTHING is sent in any of them. The transport is `node:http`'s
// `request`, injected here as a fake that records what it was asked and
// answers from a script, so no socket is opened, not even on loopback.

import { EventEmitter } from "node:events";
import { describe, expect, it } from "vitest";
import {
  COLUMNS,
  defaultPaths,
  errorRow,
  plan,
  probeArgs,
  probeBurst,
  requestHeaders,
  send,
  SHAPES,
  tsvRow,
  USAGE,
} from "./probe-burst.mjs";
import { BROWSER_UA } from "./smoke-release.mjs";

const HEX = "0123456789abcdef";

describe("probeArgs — the command line", () => {
  it("takes an https origin alone, with the defaults: one request per path and shape, at two a second", () => {
    expect(probeArgs(["https://yokohama.moore.nyc"], HEX)).toEqual({
      origin: "https://yokohama.moore.nyc",
      count: 7 * 3,
      pace: 500,
      paths: defaultPaths(HEX),
    });
  });
  it("takes the window's command exactly: --count 200 --pace 500", () => {
    const a = probeArgs(["https://yokohama.moore.nyc", "--count", "200", "--pace", "500"], HEX);
    expect([a.count, a.pace]).toEqual([200, 500]);
  });
  it("takes --paths as a comma-separated list of absolute paths, in place of the defaults", () => {
    expect(probeArgs(["https://x.example", "--paths", "/a,/b.php,/c?d=1"], HEX).paths).toEqual(["/a", "/b.php", "/c?d=1"]);
    expect(probeArgs(["https://x.example", "--paths", "/a"], HEX).count, "the default count follows the paths").toBe(3);
  });
  it("takes plain http on a loopback host alone — a dry run against the local binary", () => {
    expect(probeArgs(["http://127.0.0.1:8479"], HEX).origin).toBe("http://127.0.0.1:8479");
    expect(probeArgs(["http://localhost:8479"], HEX).origin).toBe("http://localhost:8479");
    expect(() => probeArgs(["http://yokohama.moore.nyc"], HEX)).toThrow(/https origin/);
  });
  it.each([
    [[], /usage/],
    [["yokohama.moore.nyc"], /https origin/],
    [["https://yokohama.moore.nyc/api"], /no path, query or fragment/],
    [["https://yokohama.moore.nyc/?x=1"], /no path, query or fragment/],
    [["https://user@yokohama.moore.nyc"], /no path, query or fragment/],
    [["https://x.example", "--count"], /--count takes/],
    [["https://x.example", "--count", "0"], /--count takes/],
    [["https://x.example", "--count", "1001"], /--count takes/],
    [["https://x.example", "--count", "2.5"], /--count takes/],
    [["https://x.example", "--pace", "-1"], /--pace takes/],
    [["https://x.example", "--pace", "60001"], /--pace takes/],
    [["https://x.example", "--paths", ""], /--paths takes/],
    [["https://x.example", "--paths", "/a,,/b"], /--paths takes/],
    [["https://x.example", "--paths", "a"], /--paths takes/],
    [["https://x.example", "--paths", "/a b"], /--paths takes/],
    [["https://x.example", "--paths", "/a#b"], /--paths takes/],
    [["https://x.example", "--count", "2", "--count", "3"], /twice/],
    [["https://x.example", "--follow"], /unexpected argument/],
    [["https://x.example", "https://y.example"], /unexpected argument/],
  ])("refuses %j, naming why", (argv, why) => {
    expect(() => probeArgs(argv, HEX)).toThrow(why);
  });
  it("says its usage in one line", () => {
    expect(USAGE).toBe("usage: node scripts/probe-burst.mjs <origin> [--count N] [--pace ms] [--paths a,b,c]");
  });
});

describe("the default paths", () => {
  it("are the scanner's five and the two under-prefix misses, the random ones from one hex", () => {
    expect(defaultPaths(HEX)).toEqual([
      "/.env",
      "/wp-login.php",
      "/admin",
      "/xmlrpc.php",
      `/${HEX}.json`,
      `/assets/${HEX}.js`,
      `/art/${HEX}.png`,
    ]);
  });
  it("refuses a hex that is not sixteen lowercase digits", () => {
    expect(() => defaultPaths("0123")).toThrow(/sixteen/);
    expect(() => defaultPaths("0123456789ABCDEF")).toThrow(/sixteen/);
  });
});

describe("the three shapes", () => {
  it("are named bare, navigate and no-cors, in that order", () => {
    expect(Object.keys(SHAPES)).toEqual(["bare", "navigate", "no-cors"]);
  });
  it("bare sets no header at all: what a bare client sends beyond the socket's own Host", () => {
    expect(requestHeaders("bare")).toEqual({});
  });
  it("navigate is a browser's document load: its User-Agent, Sec-Fetch-Mode navigate, Accept text/html", () => {
    expect(requestHeaders("navigate")).toEqual({ "user-agent": BROWSER_UA, "sec-fetch-mode": "navigate", accept: "text/html" });
  });
  it("no-cors is a scanner impersonating a browser: its User-Agent, Sec-Fetch-Mode no-cors, Accept */*", () => {
    expect(requestHeaders("no-cors")).toEqual({ "user-agent": BROWSER_UA, "sec-fetch-mode": "no-cors", accept: "*/*" });
  });
  it("hands out a copy, so a caller cannot change a shape for the next request", () => {
    const h = requestHeaders("navigate");
    h.accept = "x";
    expect(requestHeaders("navigate").accept).toBe("text/html");
  });
  it("refuses a shape it does not know", () => {
    expect(() => requestHeaders("cors")).toThrow(/no shape/);
  });
});

describe("plan — which path and shape each request is", () => {
  it("walks every shape of a path before the next path, and wraps", () => {
    expect(plan(["/a", "/b"], 7)).toEqual([
      ["/a", "bare"],
      ["/a", "navigate"],
      ["/a", "no-cors"],
      ["/b", "bare"],
      ["/b", "navigate"],
      ["/b", "no-cors"],
      ["/a", "bare"],
    ]);
  });
  it("covers every pair of the default paths and shapes within the window's 200", () => {
    const pairs = new Set(plan(defaultPaths(HEX), 200).map(([p, s]) => `${s} ${p}`));
    expect(pairs.size).toBe(7 * 3);
  });
});

describe("the TSV", () => {
  it("opens with the eight columns, in order", () => {
    expect(COLUMNS).toEqual(["path", "shape", "status", "cf-mitigated", "cf-cache-status", "cf-ray", "content-type", "bytes"]);
  });
  it("writes one row from a response: the edge's challenge", async () => {
    const body = "<!DOCTYPE html><title>Just a moment...</title>";
    const res = new Response(body, {
      status: 403,
      headers: { "cf-mitigated": "challenge", "cf-ray": "8c1a2b3c4d5e6f70-LHR", "content-type": "text/html; charset=UTF-8" },
    });
    const bytes = (await res.arrayBuffer()).byteLength;
    expect(tsvRow("/.env", "no-cors", { status: res.status, headers: res.headers, bytes })).toBe(
      `/.env\tno-cors\t403\tchallenge\t-\t8c1a2b3c4d5e6f70-LHR\ttext/html; charset=UTF-8\t${body.length}`,
    );
  });
  it("writes a dash for every header the answer lacks", () => {
    const res = new Response(null, { status: 404 });
    expect(tsvRow("/admin", "bare", { status: res.status, headers: res.headers, bytes: 0 })).toBe(
      "/admin\tbare\t404\t-\t-\t-\t-\t0",
    );
  });
  it("keeps a row one line of eight cells whatever a header carries", () => {
    const res = new Response(null, { status: 200, headers: { "content-type": "text/html;\tcharset=x" } });
    const row = tsvRow("/a", "bare", { status: 200, headers: res.headers, bytes: 0 });
    expect(row.split("\t")).toHaveLength(8);
    expect(row).not.toContain("\n");
  });
  it("writes a failed request as the status `error`, with no header", () => {
    expect(errorRow("/xmlrpc.php", "navigate")).toBe("/xmlrpc.php\tnavigate\terror\t-\t-\t-\t-\t-");
  });
});

/** A fake `node:http` `request`: records each call and answers with
 *  `answer(url, options)` — `{ status, headers, body }` or an `Error`. */
function fakeRequest(answer) {
  const calls = [];
  const request = (url, options, onResponse) => {
    calls.push({ url: String(url), options });
    const req = new EventEmitter();
    req.setTimeout = () => req;
    req.destroy = (e) => req.emit("error", e);
    req.end = () => {
      queueMicrotask(() => {
        const a = answer(String(url), options);
        if (a instanceof Error) {
          req.emit("error", a);
          return;
        }
        const res = new EventEmitter();
        res.statusCode = a.status;
        res.headers = a.headers ?? {};
        res.resume = () => res;
        onResponse(res);
        if (a.body) res.emit("data", Buffer.from(a.body));
        res.emit("end");
      });
      return req;
    };
    return req;
  };
  return { request, calls };
}

describe("send — one request, through node:http, never following a redirect", () => {
  it("asks once, with exactly the shape's headers, and answers a 302 as the 302 it is", async () => {
    const fake = fakeRequest(() => ({ status: 302, headers: { location: "https://elsewhere.example/" }, body: "" }));
    const a = await send("https://yokohama.moore.nyc/wp-login.php", requestHeaders("bare"), fake.request);
    expect(a.status).toBe(302);
    expect(a.headers.get("location")).toBe("https://elsewhere.example/");
    expect(fake.calls).toHaveLength(1);
    expect(fake.calls[0].url).toBe("https://yokohama.moore.nyc/wp-login.php");
    expect(fake.calls[0].options).toMatchObject({ method: "GET", headers: {} });
  });
  it("counts the body's bytes and reads its headers", async () => {
    const fake = fakeRequest(() => ({
      status: 403,
      headers: { "cf-mitigated": "challenge", "content-type": "text/html" },
      body: "abcdef",
    }));
    const a = await send("https://x.example/.env", requestHeaders("no-cors"), fake.request);
    expect([a.status, a.bytes, a.headers.get("cf-mitigated")]).toEqual([403, 6, "challenge"]);
    expect(fake.calls[0].options.headers).toEqual(requestHeaders("no-cors"));
  });
  it("rejects with the transport's error", async () => {
    const fake = fakeRequest(() => new Error("ECONNRESET"));
    await expect(send("https://x.example/a", {}, fake.request)).rejects.toThrow("ECONNRESET");
  });
});

describe("probeBurst — the whole run, over a fake transport", () => {
  it("asks for each planned path in its shape at the origin, paced, and prints the TSV", async () => {
    const fake = fakeRequest((url) => ({
      status: url.endsWith("/.env") ? 403 : 404,
      headers: url.endsWith("/.env") ? { "cf-mitigated": "challenge" } : {},
      body: "x",
    }));
    const slept = [];
    const out = [];
    const said = [];
    const failed = await probeBurst(
      { origin: "https://yokohama.moore.nyc", count: 4, pace: 500, paths: ["/.env", "/admin"] },
      {
        request: fake.request,
        sleep: async (ms) => void slept.push(ms),
        now: () => 1_000,
        out: (l) => out.push(l),
        err: (l) => said.push(l),
      },
    );
    expect(failed).toBe(0);
    expect(fake.calls.map((c) => [c.url, c.options.headers["sec-fetch-mode"] ?? "(none)"])).toEqual([
      ["https://yokohama.moore.nyc/.env", "(none)"],
      ["https://yokohama.moore.nyc/.env", "navigate"],
      ["https://yokohama.moore.nyc/.env", "no-cors"],
      ["https://yokohama.moore.nyc/admin", "(none)"],
    ]);
    expect(out).toEqual([
      COLUMNS.join("\t"),
      "/.env\tbare\t403\tchallenge\t-\t-\t-\t1",
      "/.env\tnavigate\t403\tchallenge\t-\t-\t-\t1",
      "/.env\tno-cors\t403\tchallenge\t-\t-\t-\t1",
      "/admin\tbare\t404\t-\t-\t-\t-\t1",
    ]);
    // Paced between requests, never after the last one.
    expect(slept).toEqual([500, 500, 500]);
    expect(said.join("\n")).toMatch(/4 requests/);
  });
  it("prints a failed request as an error row, names it on stderr, and counts it", async () => {
    const fake = fakeRequest(() => new Error("getaddrinfo ENOTFOUND"));
    const out = [];
    const said = [];
    const failed = await probeBurst(
      { origin: "https://x.example", count: 1, pace: 0, paths: ["/a"] },
      { request: fake.request, sleep: async () => {}, now: () => 0, out: (l) => out.push(l), err: (l) => said.push(l) },
    );
    expect(failed).toBe(1);
    expect(out[1]).toBe("/a\tbare\terror\t-\t-\t-\t-\t-");
    expect(said.join("\n")).toContain("getaddrinfo ENOTFOUND");
  });
});
