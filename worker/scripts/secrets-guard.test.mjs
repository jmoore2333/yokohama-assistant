// **The deploy's secrets, judged** (security S2a, decision D-11; the S1 stack review's unverified HI item): the
// deploy guard refuses `YOKOHAMA_DIAGNOSTIC_SEEDS` and `YOKOHAMA_WEB_TEST_CLOCK` in a committed file, and a
// Worker SECRET of either name would reach the shell's `env` past it. `judgeSecrets` reads `wrangler secret
// list`'s JSON and refuses the two by name; the command runs it before `wrangler deploy`. The command cells run
// the script as the deploy does, with a fake `npx` first on `PATH` — no account is asked.
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { FORBIDDEN_SECRETS, judgeSecrets } from "./secrets-guard.mjs";

/** `wrangler secret list`'s shape: a pretty-printed array of `{name, type}`, never a value. */
const listing = (...names) => JSON.stringify(names.map((name) => ({ name, type: "secret_text" })), null, "  ");

describe("judgeSecrets", () => {
  it("passes the two secrets this Worker keeps, and an empty listing", () => {
    expect(judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", "DEV_DOOR_CLIENT_ID"))).toEqual([]);
    expect(judgeSecrets("[]")).toEqual([]);
  });
  it("refuses each forbidden name, naming it and why — and both, one sentence each, when both are there", () => {
    for (const name of Object.keys(FORBIDDEN_SECRETS)) {
      const failures = judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", name));
      expect(failures).toHaveLength(1);
      expect(failures[0]).toContain(name);
      expect(failures[0]).toContain(FORBIDDEN_SECRETS[name]);
    }
    expect(judgeSecrets(listing("YOKOHAMA_DIAGNOSTIC_SEEDS", "YOKOHAMA_WEB_TEST_CLOCK")), "one sentence per name").toHaveLength(2);
    expect(FORBIDDEN_SECRETS).toHaveProperty("YOKOHAMA_DIAGNOSTIC_SEEDS");
    expect(FORBIDDEN_SECRETS).toHaveProperty("YOKOHAMA_WEB_TEST_CLOCK");
  });
  it("refuses a listing it cannot read rather than passing it", () => {
    for (const bad of ["", "nonsense", "{}", '[{"type":"secret_text"}]', "[1]", "[null]"]) {
      const failures = judgeSecrets(bad);
      expect(failures, bad).toHaveLength(1);
      expect(failures[0]).toMatch(/wrangler secret list/);
    }
  });
});

describe("as a command — the deploy's step, with a fake npx first on PATH, so no account is asked", () => {
  const SCRIPT = fileURLToPath(new URL("./secrets-guard.mjs", import.meta.url));
  /** The shim directories, removed after each cell (the deploy guard's cells remove theirs the same way). */
  const dirs = [];
  afterEach(() => {
    for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
  });
  /** The script run as `npm run deploy` runs it; `npx` is a shell shim that does what the cell says. */
  const run = (shim, args = ["--config", "wrangler.jsonc"]) => {
    const dir = mkdtempSync(join(tmpdir(), "secrets-guard-shim-"));
    dirs.push(dir);
    const npx = join(dir, "npx");
    writeFileSync(npx, `#!/bin/sh\n${shim}\n`);
    chmodSync(npx, 0o755);
    const r = spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8", env: { ...process.env, PATH: `${dir}:${process.env.PATH}` } });
    return { status: r.status, stdout: r.stdout, stderr: r.stderr };
  };
  it("passes a clean listing: exit 0 and the sentence", () => {
    const r = run(`echo '${listing("YOKOHAMA_WEB_ADMINS", "DEV_DOOR_CLIENT_ID")}'`);
    expect(r.status).toBe(0);
    expect(r.stdout).toMatch(/2 secrets on wrangler\.jsonc's Worker/);
  });
  it("refuses a forbidden name: exit 1, the name on stderr — the && chain stops here", () => {
    const r = run(`echo '${listing("YOKOHAMA_DIAGNOSTIC_SEEDS")}'`);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("YOKOHAMA_DIAGNOSTIC_SEEDS");
  });
  it("refuses when wrangler itself fails (no account cached): exit 1, the account's variable named, nothing judged", () => {
    const r = run(`echo 'You are not authenticated' >&2; exit 1`);
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/wrangler secret list` failed/);
    expect(r.stderr).toContain("CLOUDFLARE_ACCOUNT_ID");
  });
  it("refuses a banner line before the JSON", () => {
    expect(run(`echo ' wrangler 4.136.3'; echo '${listing("A")}'`).status).toBe(1);
  });
  it("refuses a bad argument list", () => {
    expect(run(`echo '[]'`, ["--config", "../wrangler.jsonc"]).status).toBe(1);
    expect(run(`echo '[]'`, []).status).toBe(1);
  });
});
