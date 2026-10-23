// **The deploy's secrets, judged** (security S2a, decision D-11; the S1 stack review's unverified HI item): the
// deploy guard refuses `YOKOHAMA_DIAGNOSTIC_SEEDS` and `YOKOHAMA_WEB_TEST_CLOCK` in a committed file and judges
// `YOKOHAMA_DEV_DOOR` there, and a Worker SECRET of any of the three names would reach the shell's `env` past it.
// `judgeSecrets` reads `wrangler secret list`'s JSON and refuses the three by name (the dev door's since the S2
// stack review, lens 1, Minor 3); the command runs it before `wrangler deploy`. The command cells run the script as
// the deploy does, with a fake `npx` first on `PATH` — no account is asked.
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { FORBIDDEN_SECRETS, REQUIRED_SECRETS, judgeSecrets } from "./secrets-guard.mjs";

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
    // The S2 stack review (lens 1, Minor 3): the dev door's switch, and the salt in the seeds name's reason.
    expect(FORBIDDEN_SECRETS).toHaveProperty("YOKOHAMA_DEV_DOOR");
    expect(judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", "YOKOHAMA_DEV_DOOR"))).toEqual([
      `secrets-guard: the Worker holds a secret named YOKOHAMA_DEV_DOOR — ${FORBIDDEN_SECRETS.YOKOHAMA_DEV_DOOR}; delete it: npx wrangler secret delete YOKOHAMA_DEV_DOOR`,
    ]);
    expect(FORBIDDEN_SECRETS.YOKOHAMA_DIAGNOSTIC_SEEDS, "the seeds switch also fixes the chat salt").toContain("chat salt");
    expect(Object.keys(FORBIDDEN_SECRETS), "the three names, and no other").toEqual(["YOKOHAMA_DIAGNOSTIC_SEEDS", "YOKOHAMA_WEB_TEST_CLOCK", "YOKOHAMA_DEV_DOOR"]);
  });
  it("refuses a listing it cannot read rather than passing it", () => {
    for (const bad of ["", "nonsense", "{}", '[{"type":"secret_text"}]', "[1]", "[null]"]) {
      const failures = judgeSecrets(bad);
      expect(failures, bad).toHaveLength(1);
      expect(failures[0]).toMatch(/wrangler secret list/);
    }
  });
});

// **An arm's secrets, required** (the auth cycle, ruling R-48; Task 7's review, M-9): under a sign-in arm the shell
// answers every request 500 without its secrets (`config.ts`'s `signin` and `session`), and neither guard read
// them. `judgeSecrets` is handed the judged file's door and source and REQUIRES, by the listing's names: under
// `workos` both `YOKOHAMA_OIDC_CLIENT_SECRET` and `YOKOHAMA_SESSION_SECRET`; under `oidc` the session secret alone
// (the client secret optional: PKCE); under `access` or the open door nothing. Read as the shell reads them: the
// source counts only beside the worker door. Mutants: either name left out of `workos`'s list; the client secret
// required under `oidc`; the requirement asked without the door; the requirement asked of an unreadable listing.
describe("judgeSecrets under a sign-in arm (the auth cycle, R-48)", () => {
  const put = (name, source, file = "wrangler.jsonc") =>
    `secrets-guard: ${file}'s Worker holds no secret named ${name}, which YOKOHAMA_IDENTITY_SOURCE=${source} requires — ` +
    `${name === "YOKOHAMA_SESSION_SECRET" ? "the key that signs every session cookie" : "the WorkOS API key the code exchange is authenticated with"}` +
    `; without it the shell answers every request 500: npx wrangler secret put ${name} --config ${file}`;
  const worker = (source, file = "wrangler.jsonc") => ({ file, door: "worker", source });

  it("names the two arms' secrets, and no other", () => {
    expect(REQUIRED_SECRETS).toEqual({
      workos: ["YOKOHAMA_OIDC_CLIENT_SECRET", "YOKOHAMA_SESSION_SECRET"],
      oidc: ["YOKOHAMA_SESSION_SECRET"],
    });
  });
  it("requires both secrets under workos, naming each missing one and its put line", () => {
    expect(judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", "YOKOHAMA_OIDC_CLIENT_SECRET", "YOKOHAMA_SESSION_SECRET"), worker("workos"))).toEqual([]);
    expect(judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", "YOKOHAMA_SESSION_SECRET"), worker("workos")), "no client secret").toEqual([
      put("YOKOHAMA_OIDC_CLIENT_SECRET", "workos"),
    ]);
    expect(judgeSecrets(listing("YOKOHAMA_OIDC_CLIENT_SECRET"), worker("workos")), "no session secret").toEqual([
      put("YOKOHAMA_SESSION_SECRET", "workos"),
    ]);
    expect(judgeSecrets("[]", worker("workos", "wrangler.dev.jsonc")), "neither, in the file named").toEqual([
      put("YOKOHAMA_OIDC_CLIENT_SECRET", "workos", "wrangler.dev.jsonc"),
      put("YOKOHAMA_SESSION_SECRET", "workos", "wrangler.dev.jsonc"),
    ]);
  });
  it("requires the session secret under oidc, and the client secret is optional there (PKCE)", () => {
    expect(judgeSecrets(listing("YOKOHAMA_SESSION_SECRET"), worker("oidc")), "the session secret alone").toEqual([]);
    expect(judgeSecrets(listing("YOKOHAMA_SESSION_SECRET", "YOKOHAMA_OIDC_CLIENT_SECRET"), worker("oidc")), "with a client secret").toEqual([]);
    expect(judgeSecrets(listing("YOKOHAMA_OIDC_CLIENT_SECRET"), worker("oidc")), "no session secret").toEqual([
      put("YOKOHAMA_SESSION_SECRET", "oidc"),
    ]);
  });
  it("requires nothing new under access, at the open door, or beside a door the shell does not know", () => {
    expect(judgeSecrets("[]", worker("access")), "access").toEqual([]);
    expect(judgeSecrets("[]", { file: "wrangler.open.jsonc", door: "open", source: undefined }), "the open door").toEqual([]);
    expect(judgeSecrets("[]", { file: "wrangler.open.jsonc", door: "open", source: "workos" }), "a stray source beside open").toEqual([]);
    expect(judgeSecrets("[]", { file: "wrangler.jsonc", door: undefined, source: "workos" }), "no door").toEqual([]);
    expect(judgeSecrets("[]"), "no judged file: the forbidden names alone").toEqual([]);
  });
  // A worker-door file whose source the shell refuses (`config.ts`'s `source`) names no arm to judge the secrets
  // against, and a guard that passes on a reading it cannot make is a hole: it refuses, in the shell's words with the
  // file named, as the deploy guard does. Mutants: the source unjudged (both rows pass); only an absent source refused;
  // only an unknown one refused.
  it("refuses a worker-door file with no source, or one the shell does not know, quoting the shell's sentence", () => {
    const none = "the secrets it requires cannot be judged without one of the three";
    expect(judgeSecrets("[]", worker(undefined)), "no source").toEqual([
      "secrets-guard: wrangler.jsonc's YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos under the worker door — it is not set, " +
        `and there is no default: who vouches for an address is a deployment decision; ${none}`,
    ]);
    for (const [bad, shown] of [["WorkOS", '"WorkOS"'], ["", '""'], [5, "5"], [null, "null"]]) {
      expect(judgeSecrets("[]", worker(bad, "wrangler.dev.jsonc")), JSON.stringify(bad)).toEqual([
        `secrets-guard: wrangler.dev.jsonc's YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got ${shown}; ${none}`,
      ]);
    }
    // Beside a forbidden name, both sentences, the forbidden one first; a listing that cannot be read judges neither.
    const both = judgeSecrets(listing("YOKOHAMA_DEV_DOOR"), worker(undefined));
    expect([both.length, both[0]]).toEqual([2, expect.stringContaining("YOKOHAMA_DEV_DOOR")]);
    expect(both[1]).toContain("it is not set");
    expect(judgeSecrets("nonsense", worker(undefined))).toHaveLength(1);
  });
  it("keeps the forbidden-name rules under an arm, and judges nothing of a listing it cannot read", () => {
    expect(judgeSecrets(listing("YOKOHAMA_OIDC_CLIENT_SECRET", "YOKOHAMA_SESSION_SECRET", "YOKOHAMA_DEV_DOOR"), worker("workos"))).toEqual([
      `secrets-guard: the Worker holds a secret named YOKOHAMA_DEV_DOOR — ${FORBIDDEN_SECRETS.YOKOHAMA_DEV_DOOR}; delete it: npx wrangler secret delete YOKOHAMA_DEV_DOOR`,
    ]);
    const unread = judgeSecrets("nonsense", worker("workos"));
    expect(unread, "one sentence, nothing judged").toHaveLength(1);
    expect(unread[0]).toMatch(/wrangler secret list/);
  });
});

describe("as a command — the deploy's step, with a fake npx first on PATH, so no account is asked", () => {
  const SCRIPT = fileURLToPath(new URL("./secrets-guard.mjs", import.meta.url));
  /** The shim directories, removed after each cell (the deploy guard's cells remove theirs the same way). */
  const dirs = [];
  afterEach(() => {
    for (const d of dirs.splice(0)) rmSync(d, { recursive: true, force: true });
  });
  /** The worker package, where `npm run deploy` runs the script — and where wrangler resolves `--config`. */
  const WORKER = fileURLToPath(new URL("..", import.meta.url));
  /** The script run as `npm run deploy` runs it, in `cwd` (the package by default); `npx` is a shell shim that does
   *  what the cell says. */
  const run = (shim, args = ["--config", "wrangler.jsonc"], cwd = WORKER) => {
    const dir = mkdtempSync(join(tmpdir(), "secrets-guard-shim-"));
    dirs.push(dir);
    const npx = join(dir, "npx");
    writeFileSync(npx, `#!/bin/sh\n${shim}\n`);
    chmodSync(npx, 0o755);
    const r = spawnSync(process.execPath, [SCRIPT, ...args], { cwd, encoding: "utf8", env: { ...process.env, PATH: `${dir}:${process.env.PATH}` } });
    return { status: r.status, stdout: r.stdout, stderr: r.stderr };
  };
  /** A working directory holding `wrangler.jsonc` with `vars` — the judged file, as the command reads it. */
  const judgedIn = (vars) => {
    const cwd = mkdtempSync(join(tmpdir(), "secrets-guard-cwd-"));
    dirs.push(cwd);
    writeFileSync(join(cwd, "wrangler.jsonc"), `// a deployment\n{ "name": "yokohama-web", "vars": ${JSON.stringify(vars)} }\n`);
    return cwd;
  };
  // The committed file is at source `workos` since the window's stage 1, so a clean listing carries the arm's two
  // secrets beside the Worker's own two (R-48).
  it("passes a clean listing: exit 0 and the sentence", () => {
    const r = run(`echo '${listing("YOKOHAMA_WEB_ADMINS", "DEV_DOOR_CLIENT_ID", "YOKOHAMA_OIDC_CLIENT_SECRET", "YOKOHAMA_SESSION_SECRET")}'`);
    expect(r.status, r.stderr).toBe(0);
    expect(r.stdout).toMatch(/4 secrets on wrangler\.jsonc's Worker/);
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
  // R-48: the command reads the judged file's door and source where wrangler resolves `--config` — the working
  // directory, the worker package under `npm run deploy` — and requires the arm's secrets in the listing.
  it("requires an arm's secrets, reading the judged file's source from the working directory (R-48)", () => {
    const workos = judgedIn({ YOKOHAMA_WEB_IDENTITY: "worker", YOKOHAMA_IDENTITY_SOURCE: "workos" });
    const missing = run(`echo '${listing("YOKOHAMA_WEB_ADMINS", "YOKOHAMA_OIDC_CLIENT_SECRET")}'`, undefined, workos);
    expect(missing.status, "the session secret missing: the && chain stops here").toBe(1);
    expect(missing.stderr).toContain("holds no secret named YOKOHAMA_SESSION_SECRET, which YOKOHAMA_IDENTITY_SOURCE=workos requires");
    expect(missing.stderr).toContain("npx wrangler secret put YOKOHAMA_SESSION_SECRET --config wrangler.jsonc");
    const both = run(`echo '${listing("YOKOHAMA_OIDC_CLIENT_SECRET", "YOKOHAMA_SESSION_SECRET")}'`, undefined, workos);
    expect(both.status, both.stderr).toBe(0);
    expect(both.stdout).toMatch(/2 secrets on wrangler\.jsonc's Worker, none named like a switch, and the workos arm's YOKOHAMA_OIDC_CLIENT_SECRET and YOKOHAMA_SESSION_SECRET among them/);
    // A file at source access asks nothing new; the COMMITTED file (source workos since the window's stage 1) with an
    // empty listing names both missing secrets.
    const access = run(`echo '[]'`, undefined, judgedIn({ YOKOHAMA_WEB_IDENTITY: "worker", YOKOHAMA_IDENTITY_SOURCE: "access" }));
    expect(access.status, access.stderr).toBe(0);
    const committed = run(`echo '[]'`);
    expect(committed.status, "the committed file at workos, no secrets").toBe(1);
    expect(committed.stderr).toContain("YOKOHAMA_OIDC_CLIENT_SECRET");
    expect(committed.stderr).toContain("YOKOHAMA_SESSION_SECRET");
  });
  // Run alone, the command refuses a worker-door file it cannot judge an arm for (the deploy guard refuses it first in
  // every deploy script); the open door asks nothing new.
  it("refuses a worker-door file with no source or an unknown one, and passes the open door", () => {
    const unset = run(`echo '[]'`, undefined, judgedIn({ YOKOHAMA_WEB_IDENTITY: "worker" }));
    expect(unset.status, "no source").toBe(1);
    expect(unset.stderr).toContain(
      "secrets-guard: wrangler.jsonc's YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos under the worker door — it is not set, and there is no default",
    );
    const unknown = run(`echo '[]'`, undefined, judgedIn({ YOKOHAMA_WEB_IDENTITY: "worker", YOKOHAMA_IDENTITY_SOURCE: "WorkOS" }));
    expect(unknown.status, "an unknown source").toBe(1);
    expect(unknown.stderr).toContain(`secrets-guard: wrangler.jsonc's YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos — got "WorkOS"`);
    const open = run(`echo '[]'`, undefined, judgedIn({ YOKOHAMA_WEB_IDENTITY: "open" }));
    expect([open.status, open.stderr], "the open door").toEqual([0, ""]);
  });
  it("refuses a judged file it cannot read, before asking wrangler", () => {
    const empty = mkdtempSync(join(tmpdir(), "secrets-guard-cwd-"));
    dirs.push(empty);
    const r = run(`echo 'npx was asked' >&2; echo '[]'`, undefined, empty);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("secrets-guard: wrangler.jsonc is absent — nothing was judged");
    expect(r.stderr, "wrangler is never asked").not.toContain("npx was asked");
  });
  it("refuses a bad argument list", () => {
    expect(run(`echo '[]'`, ["--config", "../wrangler.jsonc"]).status).toBe(1);
    expect(run(`echo '[]'`, []).status).toBe(1);
  });
});
