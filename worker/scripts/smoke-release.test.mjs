import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { DESCRIPTION } from "./edge-rule.mjs";
import { BROWSER_UA, liveArgs, liveSmoke, SMOKE_CONFIG, smokeRelease } from "./smoke-release.mjs";

/** The smallest valid WebAssembly module: the magic number and version 1. */
const EMPTY_WASM = Uint8Array.of(0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00);

const dirs = [];
afterEach(() => {
  for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
});

/**
 * A `gen/` holding a fake factory. Its module records every call on
 * `globalThis.__smoke`, and answers the health request with `status` and
 * `body` — or throws `initThrows` from `init`. `debug` makes it the debug
 * build's shape: `debug_trap` is a function.
 */
function gen({ status = 200, body = '{"server_version":"0.5.0"}', initThrows, debug = false, factory = true, wasm = true } = {}) {
  const root = mkdtempSync(join(tmpdir(), "smoke-release-"));
  dirs.push(root);
  const dir = join(root, "gen");
  mkdirSync(dir);
  if (wasm) writeFileSync(join(dir, "yokohama_worker_bg.wasm"), EMPTY_WASM);
  if (factory) {
    writeFileSync(
      join(dir, "factory.js"),
      `export function createModule(compiled) {
  const calls = (globalThis.__smoke = []);
  calls.push(["createModule", compiled instanceof WebAssembly.Module]);
  return {
    init(config) {
      calls.push(["init", config]);
      ${initThrows === undefined ? "" : `throw ${JSON.stringify(initThrows)};`}
    },
    dispatch(...args) {
      calls.push(["dispatch", ...args.map((a) => (a instanceof Uint8Array ? "bytes:" + a.length : a))]);
      return { status: ${status}, body: ${JSON.stringify(body)}, free() { calls.push(["free"]); } };
    },
    debug_trap: ${debug ? "function () {}" : "undefined"},
  };
}
`,
    );
  }
  return dir;
}

describe("the release module's smoke (the orchestrator's ruling, task 7's review)", () => {
  it("passes a release module that answers the health request 200, having asked it exactly that", async () => {
    const line = await smokeRelease(gen());
    expect(line).toBe('smoke-release: the release module answered GET /api/health 200: {"server_version":"0.5.0"}');
    expect(globalThis.__smoke).toEqual([
      ["createModule", true],
      ["init", SMOKE_CONFIG],
      ["dispatch", "smoke@x.test", "GET", "/api/health", "", undefined, "bytes:0"],
      ["free"],
    ]);
    expect(JSON.parse(SMOKE_CONFIG)).toEqual({
      identity: "worker",
      archive: "off",
      caller: null,
      lobby_chat: null,
      chat_keep: null,
      chat_text: null,
    });
  });

  // The refusal path the ruling asks for. Mutant: the status never read.
  it("refuses a health answer that is not 200, naming the status and the body", async () => {
    await expect(smokeRelease(gen({ status: 500, body: '{"error":"no"}' }))).rejects.toThrow(
      'smoke-release: GET /api/health answered 500, not 200: {"error":"no"}',
    );
  });
  it("refuses when init refuses the off configuration, naming the module's sentence", async () => {
    await expect(smokeRelease(gen({ initThrows: "YOKOHAMA_WEB_ARCHIVE must be r2 or off" }))).rejects.toThrow(
      "smoke-release: init refused the off configuration — YOKOHAMA_WEB_ARCHIVE must be r2 or off",
    );
  });
  // `npm run build` builds the release, but the script can be run over any
  // `gen/`; the pool's debug module is not the one that deploys.
  it("refuses the DEBUG module", async () => {
    await expect(smokeRelease(gen({ debug: true }))).rejects.toThrow(/is the DEBUG module/);
  });
  it("refuses when no module has been built", async () => {
    await expect(smokeRelease(gen({ factory: false }))).rejects.toThrow(
      /^smoke-release: gen\/factory\.js is absent — build the release module: npm run build$/,
    );
    await expect(smokeRelease(gen({ wasm: false }))).rejects.toThrow(/gen\/yokohama_worker_bg\.wasm is absent/);
  });
});

// **The edge line — the smoke's LIVE mode** (cycle F, R-F6, R-FP7; spec §2.6;
// the orchestrator's ruling on the smoke). A separate command, run against the
// DEPLOYED origin after a deploy:
//
//   node scripts/smoke-release.mjs --live <origin> --config <wrangler file>
//
// It reads the named configuration's `vars.YOKOHAMA_TURNSTILE`. Under `on` a
// cookie-less `GET <origin>/api/health` with a browser User-Agent must answer
// 403 with `cf-mitigated: challenge` — the zone's edge rule standing in front
// of the API — or the smoke FAILS naming the rule; under `off` (or absent) no
// edge line is owed and no request is made. No cell touches the network: the
// fetch is a fake answering a fixture `Response`.
describe("the smoke's live mode: the edge line", () => {
  const ORIGIN = "https://yokohama.moore.nyc";

  /** A worker package holding `wrangler.x.jsonc` whose `vars` are `vars`,
   *  and an assembled tree's stamp, `assets/build-values.json`, whose
   *  `site_url` is `siteUrl` — none at all when it is `null`. */
  function pkg(vars, siteUrl = false) {
    const root = mkdtempSync(join(tmpdir(), "smoke-live-"));
    dirs.push(root);
    writeFileSync(
      join(root, "wrangler.x.jsonc"),
      `// a configuration\n{\n  "name": "yokohama-web",\n  "vars": ${JSON.stringify(vars)},\n}\n`,
    );
    if (siteUrl !== null) {
      mkdirSync(join(root, "assets"));
      writeFileSync(
        join(root, "assets", "build-values.json"),
        `${JSON.stringify({ project_url: false, blurb: false, notice: false, release_feed: false, site_url: siteUrl, turnstile_sitekey: true }, null, 2)}\n`,
      );
    }
    return root;
  }

  /** A fetch that records each request and answers `response`. */
  function fake(response) {
    const calls = [];
    const fetch = async (url, init) => {
      calls.push({ url: String(url), init });
      if (response instanceof Error) throw response;
      return response;
    };
    return { calls, fetch };
  }

  /** A fetch that records each request and answers it by its path, from
   *  `routes` (path → a fresh Response): the edge's answer for the API and
   *  the asset layer's for the release's own files. */
  function routed(routes) {
    const calls = [];
    const fetch = async (url, init) => {
      calls.push({ url: String(url), init });
      const answer = routes[new URL(String(url)).pathname];
      if (answer === undefined) throw new Error(`the cell answers no ${String(url)}`);
      return answer();
    };
    return { calls, fetch };
  }

  const challenge = () => new Response("<html>Just a moment...</html>", { status: 403, headers: { "cf-mitigated": "challenge" } });
  const served = () => new Response("// the release", { status: 200 });
  /** What every request the live mode makes carries: no cookie, a GET, a browser's agent, no redirect followed. */
  const INIT = { method: "GET", redirect: "manual", headers: { "user-agent": BROWSER_UA } };
  const HEALTH_LINE =
    "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE on, and a cookie-less GET https://yokohama.moore.nyc/api/health " +
    `answered 403 cf-mitigated: challenge — the edge rule "${DESCRIPTION}" stands in front of the API`;

  // The pass. Mutant: the request made with a cookie, another method, a
  // non-browser agent, or following a redirect.
  it("passes on under a 403 with cf-mitigated: challenge at /api/health, having asked it and then /sw.js, each one cookie-less GET", async () => {
    const { calls, fetch } = routed({ "/api/health": challenge, "/sw.js": served });
    const line = await liveSmoke(ORIGIN, pkg({ YOKOHAMA_WEB_IDENTITY: "open", YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", fetch);
    expect(line).toBe(
      `${HEALTH_LINE}\n` +
        "smoke-release: and a cookie-less GET https://yokohama.moore.nyc/sw.js answered 200 with no cf-mitigated header — the release's own file is not challenged\n" +
        "smoke-release: /sitemap.xml not asked — assets/build-values.json says this build wrote none (site_url false)",
    );
    expect(calls).toEqual([
      { url: "https://yokohama.moore.nyc/api/health", init: INIT },
      { url: "https://yokohama.moore.nyc/sw.js", init: INIT },
    ]);
    expect(BROWSER_UA).toMatch(/^Mozilla\/5\.0 .* Chrome\/\d+/);
  });

  // **The positive control** (the final review's Important 2): a rule that
  // challenges the release's OWN files — printed from a tree without the
  // sitemap, or edited in the zone — must fail the smoke, which before read
  // only that the API was challenged. `/sw.js` always; `/sitemap.xml` exactly
  // when the LOCAL stamp says the build wrote one. Mutants: the static reads
  // missing; the challenge unread on them; the sitemap asked whatever the
  // stamp, or never.
  it("asks /sitemap.xml too when the local stamp says the build wrote one, and passes when neither file is challenged", async () => {
    const { calls, fetch } = routed({ "/api/health": challenge, "/sw.js": served, "/sitemap.xml": served });
    const line = await liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }, true), "wrangler.x.jsonc", fetch);
    expect(line).toBe(
      `${HEALTH_LINE}\n` +
        "smoke-release: and a cookie-less GET https://yokohama.moore.nyc/sw.js answered 200 with no cf-mitigated header — the release's own file is not challenged\n" +
        "smoke-release: and a cookie-less GET https://yokohama.moore.nyc/sitemap.xml answered 200 with no cf-mitigated header — the release's own file is not challenged",
    );
    expect(calls).toEqual([
      { url: "https://yokohama.moore.nyc/api/health", init: INIT },
      { url: "https://yokohama.moore.nyc/sw.js", init: INIT },
      { url: "https://yokohama.moore.nyc/sitemap.xml", init: INIT },
    ]);
  });
  it("FAILS on when the rule challenges the release's own /sw.js, naming the path, and asks nothing after it", async () => {
    const { calls, fetch } = routed({ "/api/health": challenge, "/sw.js": challenge, "/sitemap.xml": served });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }, true), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE on, but a cookie-less GET https://yokohama.moore.nyc/sw.js " +
        `answered 403 with cf-mitigated: challenge — the edge rule "${DESCRIPTION}" challenges the release's own /sw.js: print it ` +
        "again from the deployed tree and update it by its id (node scripts/edge-rule.mjs; --verify; worker/RUNBOOK.md §3)",
    );
    expect(calls.map((c) => c.url)).toEqual(["https://yokohama.moore.nyc/api/health", "https://yokohama.moore.nyc/sw.js"]);
  });
  it("FAILS on when the rule challenges /sitemap.xml under a stamp that says the build wrote one", async () => {
    const { fetch } = routed({ "/api/health": challenge, "/sw.js": served, "/sitemap.xml": challenge });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }, true), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      /^smoke-release: wrangler\.x\.jsonc declares YOKOHAMA_TURNSTILE on, but a cookie-less GET https:\/\/yokohama\.moore\.nyc\/sitemap\.xml answered 403 with cf-mitigated: challenge — the edge rule .* challenges the release's own \/sitemap\.xml: /,
    );
  });
  // **The control requires a 200** (cycle F's residual fix; the Fable
  // re-review): an answer with no `cf-mitigated` header is not enough, since a
  // 404 for a `/sitemap.xml` the local stamp promises is a deployed tree that
  // is not the local one, and a `/sw.js` that is not 200 is a broken release.
  // Mutants: any non-mitigated status passed; the status unread on one path.
  it("FAILS on when the release's own file answers anything but 200, naming the path and the status", async () => {
    const missing = () => new Response("<!doctype html>", { status: 404 });
    const { fetch } = routed({ "/api/health": challenge, "/sw.js": served, "/sitemap.xml": missing });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }, true), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE on, but a cookie-less GET https://yokohama.moore.nyc/sitemap.xml " +
        "answered 404 with no cf-mitigated header — the release's own /sitemap.xml must answer 200: the deployed release is " +
        "not the local tree, or the deploy is broken (run the smoke from the tree that was deployed)",
    );
    const broken = routed({ "/api/health": challenge, "/sw.js": () => new Response("", { status: 503 }) });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", broken.fetch)).rejects.toThrow(
      /GET https:\/\/yokohama\.moore\.nyc\/sw\.js answered 503 with no cf-mitigated header — the release's own \/sw\.js must answer 200: /,
    );
    const moved = routed({ "/api/health": challenge, "/sw.js": () => new Response("", { status: 302, headers: { location: "/" } }) });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", moved.fetch)).rejects.toThrow(
      /\/sw\.js answered 302 with no cf-mitigated header — the release's own \/sw\.js must answer 200/,
    );
    // A challenge is still named as a challenge, whatever its status.
    const odd = routed({ "/api/health": challenge, "/sw.js": () => new Response("", { status: 200, headers: { "cf-mitigated": "challenge" } }) });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", odd.fetch)).rejects.toThrow(
      /\/sw\.js answered 200 with cf-mitigated: challenge — the edge rule .* challenges the release's own \/sw\.js/,
    );
  });
  // A reading that cannot say which files the deployed release holds asks
  // nothing: the positive control would otherwise silently shrink.
  it("refuses on with no local stamp, or one that is not the build's, asking nothing", async () => {
    const { calls, fetch } = routed({ "/api/health": challenge, "/sw.js": served });
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }, null), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE on, but assets/build-values.json is absent, so nothing says " +
        "whether the deployed build wrote a sitemap — run the smoke from the tree that was deployed (npm run build)",
    );
    const odd = pkg({ YOKOHAMA_TURNSTILE: "on" });
    writeFileSync(join(odd, "assets", "build-values.json"), '{"site_url": "yes"}');
    await expect(liveSmoke(ORIGIN, odd, "wrangler.x.jsonc", fetch)).rejects.toThrow(
      /^smoke-release: wrangler\.x\.jsonc declares YOKOHAMA_TURNSTILE on, but assets\/build-values\.json is not the client build's stamp \(its site_url is not a boolean\)/,
    );
    expect(calls).toEqual([]);
  });
  // **Battery row 32** (the smoke passes `on` over a 200): this cell. A
  // switch that says `on` over a missing rule is not a state this project
  // ships. Mutants: the status unread; the header unread; either read alone.
  it("FAILS on over a 200, naming the rule", async () => {
    const { fetch } = fake(new Response('{"server_version":"0.6.0"}', { status: 200 }));
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE on, but a cookie-less GET https://yokohama.moore.nyc/api/health " +
        `answered 200 with no cf-mitigated header — the edge rule "${DESCRIPTION}" is not challenging the API: create or enable ` +
        "it by its id (node scripts/edge-rule.mjs; worker/RUNBOOK.md §3)",
    );
  });
  it("FAILS on over a 403 that is not the challenge, and over a challenge that is not a 403", async () => {
    const access = fake(new Response("Forbidden", { status: 403 }));
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", access.fetch)).rejects.toThrow(
      /answered 403 with no cf-mitigated header — the edge rule .* is not challenging the API/,
    );
    const blocked = fake(new Response("", { status: 403, headers: { "cf-mitigated": "block" } }));
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", blocked.fetch)).rejects.toThrow(
      /answered 403 with cf-mitigated: block — the edge rule/,
    );
    const odd = fake(new Response("", { status: 302, headers: { "cf-mitigated": "challenge", location: "/" } }));
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", odd.fetch)).rejects.toThrow(
      /answered 302 with cf-mitigated: challenge — the edge rule/,
    );
  });
  it("FAILS on when the request itself fails, naming the error", async () => {
    const { fetch } = fake(new TypeError("fetch failed"));
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "on" }), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE on, but GET https://yokohama.moore.nyc/api/health failed — fetch failed",
    );
  });
  // `off` reads nothing. Mutant: the request made whatever the declaration.
  it("reads nothing under off or absent: no request, and a line saying no edge line is owed", async () => {
    for (const vars of [{ YOKOHAMA_TURNSTILE: "off" }, { YOKOHAMA_WEB_IDENTITY: "worker" }]) {
      const { calls, fetch } = fake(new Response("", { status: 200 }));
      const line = await liveSmoke(ORIGIN, pkg(vars), "wrangler.x.jsonc", fetch);
      expect(line, JSON.stringify(vars)).toBe(
        "smoke-release: wrangler.x.jsonc declares YOKOHAMA_TURNSTILE off — no edge line is owed, and no request was made",
      );
      expect(calls, JSON.stringify(vars)).toEqual([]);
    }
  });
  // Read as the guard's check 12 reads it: anything but on, off or absent is
  // refused, never read as off — and nothing is asked.
  it("refuses a declaration that is neither on nor off, and a configuration it cannot read", async () => {
    const { calls, fetch } = fake(challenge());
    await expect(liveSmoke(ORIGIN, pkg({ YOKOHAMA_TURNSTILE: "On" }), "wrangler.x.jsonc", fetch)).rejects.toThrow(
      'smoke-release: wrangler.x.jsonc\'s vars.YOKOHAMA_TURNSTILE must be "on" or "off" (absent is off) — got "On"',
    );
    await expect(liveSmoke(ORIGIN, pkg({}), "wrangler.absent.jsonc", fetch)).rejects.toThrow(
      /^smoke-release: wrangler\.absent\.jsonc is absent$/,
    );
    const root = pkg({});
    writeFileSync(join(root, "wrangler.x.jsonc"), "{ not json");
    await expect(liveSmoke(ORIGIN, root, "wrangler.x.jsonc", fetch)).rejects.toThrow(/^smoke-release: wrangler\.x\.jsonc does not parse/);
    expect(calls).toEqual([]);
  });
  // The command line: no argument is the in-process smoke, unchanged; the
  // live mode is exactly `--live <origin> --config <wrangler file>`.
  it("reads the live mode's arguments exactly, and refuses anything else", () => {
    expect(liveArgs(["node", "smoke-release.mjs"])).toBeNull();
    expect(liveArgs(["node", "smoke-release.mjs", "--live", "https://yokohama.moore.nyc", "--config", "wrangler.open.jsonc"])).toEqual({
      origin: "https://yokohama.moore.nyc",
      config: "wrangler.open.jsonc",
    });
    expect(liveArgs(["node", "smoke-release.mjs", "--live", "https://yokohama.moore.nyc/", "--config", "wrangler.jsonc"])).toEqual({
      origin: "https://yokohama.moore.nyc",
      config: "wrangler.jsonc",
    });
    for (const bad of [
      ["--live", "https://yokohama.moore.nyc"],
      ["--config", "wrangler.open.jsonc", "--live", "https://yokohama.moore.nyc"],
      ["--live", "https://yokohama.moore.nyc", "--config", "../wrangler.jsonc"],
      ["--live", "https://yokohama.moore.nyc", "--config", "other.jsonc"],
      ["--force"],
    ]) {
      expect(() => liveArgs(["node", "smoke-release.mjs", ...bad]), bad.join(" ")).toThrow(
        /^smoke-release: expected no arguments, or --live <origin> --config wrangler<name>\.jsonc — got /,
      );
    }
    for (const origin of ["http://yokohama.moore.nyc", "https://yokohama.moore.nyc/api", "https://yokohama.moore.nyc?x=1", "yokohama.moore.nyc"]) {
      expect(() => liveArgs(["node", "smoke-release.mjs", "--live", origin, "--config", "wrangler.jsonc"]), origin).toThrow(
        /^smoke-release: --live takes the deployment's https origin, with no path, query or fragment — got /,
      );
    }
  });
});
