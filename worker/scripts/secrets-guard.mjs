// **The deploy's secrets, judged before `wrangler deploy`** (security S2a, decision D-11). The deploy guard
// refuses the diagnostic seeds and the test clock in a committed configuration (its rules 4 and 4b) and judges the
// dev door's switch there (its door rules); a Worker SECRET of any of the three names reaches the shell's `env`
// past them, since a deploy leaves secrets alone and the guard reads files. So the deploy asks wrangler for the
// names — `wrangler secret list` prints a JSON array of `{name, type}`, never a value — and refuses the three (the
// dev door's since the S2 stack review, lens 1, Minor 3).
//
// **And it requires an arm's secrets** (the auth cycle, ruling R-48; Task 7's review, M-9): under a sign-in arm the
// shell answers every request 500 without them (`config.ts`'s `signin` and `session`), and the deploy guard, which
// reads files, cannot see a secret. So the command reads the judged file's door and source with the deploy guard's
// own reader (`readConfig`, `doorOf`, `sourceOf`, imported from `deploy-guard.mjs`, not copied) — from the working
// directory, where wrangler resolves the same `--config` — and requires, beside the worker door: under `workos`
// both `YOKOHAMA_OIDC_CLIENT_SECRET` (the WorkOS API key) and `YOKOHAMA_SESSION_SECRET`; under `oidc` the session
// secret alone (the client secret is optional there: PKCE); under `access`, and at the open door, nothing. A
// missing one refuses the deploy, naming it and its `npx wrangler secret put <name> --config <file>` line. A file
// whose door is absent or neither `worker` nor `open` has no door to judge the secrets against, and a worker door
// with no source, or one the shell does not know, names no arm to judge them against, so each refuses too, in the
// shell's words with the file named, as the deploy guard's own refusal of the same file reads (the deploy guard
// refuses it first in every deploy script; this guard run alone must not pass a reading it cannot make). The
// listing names secrets and never holds a value, so the session secret's 32-byte floor stays the shell's to refuse.
// A judged file the command cannot read refuses before wrangler is asked.
//
// Pure but for the file read and the one `execFileSync`, which the command alone makes; the cells drive
// `judgeSecrets` over captured listings and the command over a fake `npx`.
//
// The listing asks the ACCOUNT: every deploy script — the plain `deploy`, the safe direction, included — is a network
// step from here on — a logged-out clone runs `npx wrangler login` once, and one with more than one account sets
// `CLOUDFLARE_ACCOUNT_ID` (`npx wrangler whoami` lists them and caches nothing; wrangler 4.136.3's source, Task 6's
// review) — and a NEW Worker's first deploy cannot pass it (wrangler answers "Worker not found"): that deploy runs
// `wrangler deploy --config <file>` by hand without this guard — a Worker that does not exist holds no secret; the
// deploy guard still judges a file whose deployment it records (`DEPLOYMENTS`, production's and the private
// Worker's since the auth cycle's Task 7) and passes no other file under another Worker's name — and this guard from
// the second deploy on (D-7 as corrected at S2b Task 9's review; `worker/RUNBOOK.md` §2). A failed listing is a
// refusal, never a pass.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { doorOf, readConfig, SOURCES, sourceOf } from "./deploy-guard.mjs";

/** The names a secret must never have, and why. */
export const FORBIDDEN_SECRETS = Object.freeze({
  YOKOHAMA_DIAGNOSTIC_SEEDS:
    "a creator's seed would be honoured beside another person (R-SEC8(b)), and the same switch sets the layer's " +
    "SeedPolicy::Diagnostic, under which every room's chat salt is the committed FIXED_CHAT_SALT, so anyone could " +
    "confirm a watcher's tag by hashing a guessed address; diagnostic seeds are the pool's",
  YOKOHAMA_WEB_TEST_CLOCK: "the test clock pins the layer's reading; it is the pool's and never deploys",
  YOKOHAMA_DEV_DOOR:
    "the dev door's switch is the configuration's — off in wrangler.jsonc and, of the files a deploy names, on in " +
    "wrangler.dev.jsonc alone, as the deploy guard judges them — and a live tier opens it with `npm run deploy:dev`, " +
    "a var and never a secret",
});

/** The secrets a sign-in arm needs at the shell's first request, by `YOKOHAMA_IDENTITY_SOURCE` (the auth cycle,
 *  ruling R-48): the client secret is the WorkOS API key the code exchange is authenticated with, and optional
 *  under `oidc` (PKCE); the session secret signs every session cookie under either arm. */
export const REQUIRED_SECRETS = Object.freeze({
  workos: Object.freeze(["YOKOHAMA_OIDC_CLIENT_SECRET", "YOKOHAMA_SESSION_SECRET"]),
  oidc: Object.freeze(["YOKOHAMA_SESSION_SECRET"]),
});

/** What each required secret is, as its refusal says. */
const REQUIRED_WHY = {
  YOKOHAMA_OIDC_CLIENT_SECRET: "the WorkOS API key the code exchange is authenticated with",
  YOKOHAMA_SESSION_SECRET: "the key that signs every session cookie",
};

/** The secrets the judged file requires (`{ file, door, source }`, as the deploy guard reads them): an arm's, beside
 *  the worker door — the shell reads no source at the open door (`config.ts`'s `source`) — and none otherwise, or
 *  when no file is handed. */
export function requiredSecrets(judged) {
  if (judged?.door !== "worker" || typeof judged.source !== "string" || !Object.hasOwn(REQUIRED_SECRETS, judged.source)) return [];
  return REQUIRED_SECRETS[judged.source];
}

/** The refusal of a file whose door the shell refuses (`config.ts`'s `door`: absent, or neither `worker` nor
 *  `open`), in the shell's sentence with the file named; `[]` for either door, or no file. */
function doorRefusal(judged) {
  if (judged === undefined || judged.door === "worker" || judged.door === "open") return [];
  const shell =
    judged.door === undefined
      ? "must be worker or open — it is not set, and there is no default: a door is a deployment decision"
      : `must be worker or open — got ${JSON.stringify(judged.door)}`;
  return [`secrets-guard: ${judged.file}'s YOKOHAMA_WEB_IDENTITY ${shell}; the secrets it requires cannot be judged without a door`];
}

/** The refusal of a worker-door file whose source the shell refuses (`config.ts`'s `source`: absent, or not one of
 *  the deploy guard's `SOURCES`), in the shell's sentence with the file named; `[]` for any other door, or no file. */
function sourceRefusal(judged) {
  if (judged?.door !== "worker" || (typeof judged.source === "string" && Object.hasOwn(SOURCES, judged.source))) return [];
  const shell =
    judged.source === undefined
      ? "must be access, oidc or workos under the worker door — it is not set, and there is no default: who vouches for an address is a deployment decision"
      : `must be access, oidc or workos — got ${JSON.stringify(judged.source)}`;
  return [`secrets-guard: ${judged.file}'s YOKOHAMA_IDENTITY_SOURCE ${shell}; the secrets it requires cannot be judged without one of the three`];
}

/** `wrangler secret list`'s output judged against the judged file (`judged`: `{ file, door, source }`, R-48; absent,
 *  the forbidden names alone): one sentence per forbidden name, then the refusal of a file whose door the shell
 *  refuses (`doorRefusal`), then that of a worker-door file whose source the shell refuses (`sourceRefusal`), then
 *  one per secret the file requires that the listing lacks, and `[]` when there is none of the three; ONE sentence when the listing cannot be read — a listing that does not parse is not a
 *  pass, and nothing else is judged of it. */
export function judgeSecrets(listingJson, judged) {
  let listing;
  try {
    listing = JSON.parse(listingJson);
  } catch {
    return ["secrets-guard: `wrangler secret list` printed something that is not JSON — nothing was judged"];
  }
  if (!Array.isArray(listing) || !listing.every((s) => s !== null && typeof s === "object" && typeof s.name === "string")) {
    return ["secrets-guard: `wrangler secret list` printed something that is not a list of {name} — nothing was judged"];
  }
  const forbidden = listing
    .filter((s) => Object.hasOwn(FORBIDDEN_SECRETS, s.name))
    .map((s) => `secrets-guard: the Worker holds a secret named ${s.name} — ${FORBIDDEN_SECRETS[s.name]}; delete it: npx wrangler secret delete ${s.name}`);
  const held = new Set(listing.map((s) => s.name));
  const missing = requiredSecrets(judged)
    .filter((name) => !held.has(name))
    .map(
      (name) =>
        `secrets-guard: ${judged.file}'s Worker holds no secret named ${name}, which YOKOHAMA_IDENTITY_SOURCE=${judged.source} requires — ` +
        `${REQUIRED_WHY[name]}; without it the shell answers every request 500: npx wrangler secret put ${name} --config ${judged.file}`,
    );
  return [...forbidden, ...doorRefusal(judged), ...sourceRefusal(judged), ...missing];
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  if (!(args.length === 2 && args[0] === "--config" && /^wrangler[A-Za-z0-9.-]*\.jsonc$/.test(args[1]))) {
    console.error(`secrets-guard: expected --config wrangler<name>.jsonc — got ${JSON.stringify(args)}`);
    process.exit(1);
  }
  // The judged file's door and source (R-48), read where wrangler resolves `--config`: the working directory — the
  // worker package under `npm run deploy`. A file that cannot be read refuses before wrangler is asked.
  const unread = [];
  const config = readConfig(process.cwd(), args[1], unread);
  if (config === undefined) {
    console.error(`secrets-guard: ${unread.join("; ")} — nothing was judged`);
    process.exit(1);
  }
  const judged = { file: args[1], door: doorOf(config.value), source: sourceOf(config.value) };
  let listing;
  try {
    listing = execFileSync("npx", ["wrangler", "secret", "list", "--config", args[1]], { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
  } catch {
    // No cached account, no network, or wrangler's own refusal: a deploy does not proceed on an unread listing.
    console.error("secrets-guard: `wrangler secret list` failed — `npx wrangler login` if logged out; with more than one account set CLOUDFLARE_ACCOUNT_ID (`npx wrangler whoami` lists them); nothing was judged");
    process.exit(1);
  }
  const failures = judgeSecrets(listing, judged);
  if (failures.length > 0) {
    console.error(failures.join("\n"));
    process.exit(1);
  }
  const required = requiredSecrets(judged);
  const arm = required.length > 0 ? `, and the ${judged.source} arm's ${required.join(" and ")} among them` : "";
  console.log(`secrets-guard: ${JSON.parse(listing).length} secrets on ${args[1]}'s Worker, none named like a switch${arm}`);
}
