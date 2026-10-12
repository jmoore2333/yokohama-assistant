// **The deploy's secrets, judged before `wrangler deploy`** (security S2a, decision D-11). The deploy guard
// refuses the diagnostic seeds and the test clock in a committed configuration (its rules 4 and 4b); a Worker
// SECRET of either name reaches the shell's `env` past both, since a deploy leaves secrets alone and the guard
// reads files. So the deploy asks wrangler for the names — `wrangler secret list` prints a JSON array of
// `{name, type}`, never a value — and refuses the two. Pure but for the one `execFileSync`, which the command
// alone makes; the cells drive `judgeSecrets` over captured listings and the command over a fake `npx`.
//
// The listing asks the ACCOUNT: every deploy script — the plain `deploy`, the safe direction, included — is a network
// step from here on — a logged-out clone runs `npx wrangler login` once, and one with more than one account sets
// `CLOUDFLARE_ACCOUNT_ID` (`npx wrangler whoami` lists them and caches nothing; wrangler 4.136.3's source, Task 6's
// review) — and a NEW Worker's first deploy cannot pass it (wrangler answers "Worker not found"): that deploy runs
// `wrangler deploy --config <file>` by hand with neither guard — a Worker that does not exist holds no secret, and
// the deploy guard holds this repository's Worker alone, its name and bucket, until S4 gives it the new Worker's
// identity — and this guard from the second deploy on (D-7 as corrected at S2b Task 9's review; `worker/RUNBOOK.md`
// §2). A failed listing is a refusal, never a pass.
import { execFileSync } from "node:child_process";
import { pathToFileURL } from "node:url";

/** The names a secret must never have, and why. */
export const FORBIDDEN_SECRETS = Object.freeze({
  YOKOHAMA_DIAGNOSTIC_SEEDS: "a creator's seed would be honoured beside another person (R-SEC8(b)); diagnostic seeds are the pool's",
  YOKOHAMA_WEB_TEST_CLOCK: "the test clock pins the layer's reading; it is the pool's and never deploys",
});

/** `wrangler secret list`'s output judged: `[]` when no secret is forbidden; one sentence per forbidden name;
 *  ONE sentence when the listing cannot be read — a listing that does not parse is not a pass. */
export function judgeSecrets(listingJson) {
  let listing;
  try {
    listing = JSON.parse(listingJson);
  } catch {
    return ["secrets-guard: `wrangler secret list` printed something that is not JSON — nothing was judged"];
  }
  if (!Array.isArray(listing) || !listing.every((s) => s !== null && typeof s === "object" && typeof s.name === "string")) {
    return ["secrets-guard: `wrangler secret list` printed something that is not a list of {name} — nothing was judged"];
  }
  return listing
    .filter((s) => Object.hasOwn(FORBIDDEN_SECRETS, s.name))
    .map((s) => `secrets-guard: the Worker holds a secret named ${s.name} — ${FORBIDDEN_SECRETS[s.name]}; delete it: npx wrangler secret delete ${s.name}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2);
  if (!(args.length === 2 && args[0] === "--config" && /^wrangler[A-Za-z0-9.-]*\.jsonc$/.test(args[1]))) {
    console.error(`secrets-guard: expected --config wrangler<name>.jsonc — got ${JSON.stringify(args)}`);
    process.exit(1);
  }
  let listing;
  try {
    listing = execFileSync("npx", ["wrangler", "secret", "list", "--config", args[1]], { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
  } catch {
    // No cached account, no network, or wrangler's own refusal: a deploy does not proceed on an unread listing.
    console.error("secrets-guard: `wrangler secret list` failed — `npx wrangler login` if logged out; with more than one account set CLOUDFLARE_ACCOUNT_ID (`npx wrangler whoami` lists them); nothing was judged");
    process.exit(1);
  }
  const failures = judgeSecrets(listing);
  if (failures.length > 0) {
    console.error(failures.join("\n"));
    process.exit(1);
  }
  console.log(`secrets-guard: ${JSON.parse(listing).length} secrets on ${args[1]}'s Worker, none named like a switch`);
}
