# Operating the Cloudflare host — `worker/RUNBOOK.md`

**This is `deploy/RUNBOOK.md` §6, moved here VERBATIM on 2026-09-28** (M6.5c2, R-6.5c2.15), so each host's
directory holds its own guide and its own runbook:
- this directory for the Worker, with [`README.md`](README.md) beside this file;
- `deploy/` for the NAS and the self-host image, with [`deploy/README.md`](../deploy/README.md) and
  [`deploy/RUNBOOK.md`](../deploy/RUNBOOK.md).

Only the section references moved:
- `§6.N` there is `§N` here;
- `§6` there is this file's opening, below;
- the three references to the NAS's own sections now name their file.

A sentence that M6.5c2's code made false is corrected where it stands, and a fact M6.5c2 added is
written beside the text it belongs with. Each carries a dated note, so the moved text and the changes
can be told apart. `deploy/RUNBOOK.md` §6 keeps the map, so a record that cites "RUNBOOK §6.7" still
resolves. Deployments from the twenty-first on are recorded here from §15.

---

## The Worker — hosted multiplayer on Cloudflare (M6.5c)

**`yokohama-web`, a Cloudflare Worker with three kinds of Durable Object, at
`yokohama.moore.nyc`**, behind the same `yokohama` Access application as the NAS. The authority is
`docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` (§2 the mechanism, §3 the deployment);
the code is `worker/` — a TypeScript shell — over `crates/yokohama-worker`, the table layer compiled
to WebAssembly. The NAS (`deploy/RUNBOOK.md` §0–§3) is unchanged and keeps `yokohama-nas.moore.nyc`.

What answers what:

- **The static release** — `web/dist` and the card art — is served by Cloudflare's asset layer and
  invokes the Worker ZERO times. The Worker sees only `/api/*`, `/enter`, and a path that has no file,
  which it answers 404 with an empty body (spec §2.3).
- **The Worker** is stateless. It verifies the Access assertion, removes every client header named
  `x-yokohama-…`, and forwards to one of three SQLite-backed classes: `Table`, one per table;
  `House`, ONE, for the lobby and the house chat; and `Archive`, one per caller, for a browser's saves.
- **A few answers are this host's own words** (spec §4's host-written class, and this cycle's
  additions). Everything else a client reads is the layer's.
  - a path with no file: **404** with an empty body — never `index.html`;
  - any method but GET or HEAD on `/enter` or `/api/art`, and any method but POST on `/api/leave`:
    **405**, empty; `/enter`: **302** to `/#/lobby` at source `access` and under `open`, and under
    `oidc` or `workos` BOTH halves of the sign-in (the auth cycle; §23);
  - a body over 2 MiB: **400** `the body could not be read: length limit exceeded` — the NAS's own
    sentence, read off it on 2026-09-23;
  - a bad variable: **500** naming it (§3);
  - **a WebSocket upgrade from another site's page — its `Origin` present and not
    `https://yokohama.moore.nyc`: 403 `this socket is for the site's own pages`**, unupgraded. A
    WebSocket is outside CORS, so without it a hostile page in a signed-in player's browser could
    read a table's turn and timing frames (Task 11's review, escalation 1). A request with no
    `Origin` — curl, the deploy readings — passes;
  - **any method but GET or HEAD from another site's page — an `Origin` present and not
    `https://yokohama.moore.nyc`, `null` included, or a `Sec-Fetch-Site` of `cross-site` or
    `same-site`: 403 `this request is for the site's own pages`** (security S2b, finding B5), read
    before the caller is: the operator's expire was a CORS simple request a hostile page could send
    with ambient authentication — the `CF_Authorization` cookie, or the owner's WARP identity. A
    request with neither header — curl, Node's `fetch`, the live replay — passes;
  - **past a rate limit, any method but GET or HEAD: 429 `too many requests from you just now — wait a
    moment and try again`**, with `Retry-After: 60` for a create and `Retry-After: 10` for any other
    write (security S2b, finding C2; §2, §7), answered before any object;
  - any method but GET on `…/events`, `…/stream` or `/api/stream`: **405**, empty; a GET of
    `…/stream` or `/api/stream` that is not an upgrade: **426** `this host streams over
    WebSocket`;
  - the create's two: **400** `a game id, when given, must be a string`, **500** `the create's id
    does not name the table it was routed to`. *(Corrected 2026-09-28, M6.5c2: this line named
    three, the first a host sentence, `a game must be created from a JSON object`. Since M6.5c2 a
    body that is not a JSON object reaches the layer, whose own 400 answers it, `the request body
    must be a JSON object` — `api::NOT_AN_OBJECT` in `crates/yokohama-web/src/api.rs`; the module
    keeps that sentence only as an unreachable guard.)*
  - a create naming a table that holds a MARKER, or whose REMOVAL the lobby has not yet
    acknowledged: **409** `this name was used recently — try another` (spec §2.4; Tasks 12 and 14),
    so a new table's first copy can never overwrite a removal the house has not received;
  - **a table's MARKER** (spec §2.4 steps 2 and 5; Task 14): for 90 days after the layer deleted a
    table — its creator's Abandon, its expiry, or an operator's plain expire — EVERY request to it
    answers **404** with a sentence of a head and a tail, an upgrade included, answered unupgraded so
    the client's probe reads it. The heads: `this table was abandoned on 2026-10-22`, `this table
    expired on 2026-10-22 after 90 days without activity` (one day of TTL reads `1 day`), `this
    table waited 7 days to start and was closed on 2026-10-22` (M6.5c2 cycle E, R-6.5c2.42: a
    table that had not started when its alarm closed it — from 0.7.2 a dealing one too, security S3,
    D9), `this table was removed by an
    operator on 2026-10-22` — the UTC date of the delete. The tails: `— its
    game is in the archive` where its game was saved — stamped at game over, or saved by the delete
    itself, which with the switch at `r2` saves every game before it deletes it — whatever the
    switch reads now (the orchestrator's ruling I-71); `— no move had been made, so there was
    nothing to keep` where there was no game; and `— this server keeps no archive, so its game was
    not kept` for a game never saved, deleted while the switch reads `off`. After the 90 days, once
    the lobby has acknowledged the removal, the object ends and the id reads the layer's own `no
    game named …`;
  - the operator surface (spec §2.9; §3 below): **403** `this address is not an operator here`;
    **400** `?force takes no value — send ?force alone, or no query for an expire that saves first`
    for a force given any value, `?force=false` among them (the orchestrator's ruling I-72); **400**
    `` Expected request with `Content-Type: application/json` ``, the layer's own words, for an expire
    or a force whose type is not `application/json` (security S2b, finding B5); and a force's **503**
    `the lobby has not confirmed the removal — the alarm will finish it`;
  - a trapped engine: **500** `the table's engine stopped — try again`, `the lobby's engine stopped
    — try again` or `the archive's engine stopped — try again`; a table whose stored document traps
    twice: **500** `this table's stored <kind> cannot be loaded`;
  - **an object whose request failed on an error that is not a trap** — a reply the shell cannot
    read, a batch that did not commit, an object reset under `blockConcurrencyWhile`: **500** `the
    server could not answer — try again`, one sentence for the table, the house and the archive
    alike, logged `object-failed` (the orchestrator's F-1). Before it, the platform's own plain 500
    answered, which the client cannot read as `{"error": …}`;
  - a deleted table's sockets: `end {"reason":"deleted"}`, then a **4404** close whose reason is
    `this table was deleted`;
  - a socket past a cap (security S2b, finding D3): accepted, sent `end` with the cap's sentence and
    closed **4429** — `you have too many connections open to this table — close another tab and try
    again` past 8 of the caller's at a table, `this table has as many connections as it can take — try
    again later` past 64 in all, `this table has as many watchers as it can take — try again later` for
    a caller who holds no seat at it once 32 such sockets are open (S4-pre, from `0.7.3`, which deploys
    with stage 2; a seated caller is refused only by the 64 and by their own 8), and in the lobby `you
    have too many connections open to the lobby — close another tab and try again` past 8 and `the lobby
    has as many connections as it can take — try again later` past 1,024.

## 1. Build

Prerequisites: the pinned toolchain with its `wasm32-unknown-unknown` target, `wasm-bindgen-cli
0.2.128` and binaryen's `wasm-opt` (the same three as `deploy/RUNBOOK.md` §1's client build), and `npm ci` once in both
`web/` and `worker/`.

```sh
# the RELEASE browser module, and web/dist; the build's seven values (C-22) are read here
# when set (checked by web/scripts/build-values.mjs, a bad one fails the build naming
# it; unset or empty draws nothing, the release feed's derivation from a GitHub
# project link apart; the limits are in README.md §1.5). YOKOHAMA_SITE_URL names THIS
# host, so each host serves its own client build. YOKOHAMA_TURNSTILE_SITEKEY stays
# unset for the Access build (source `access`): a keyed build is for the open door and for
# a provider-signed worker door (source `oidc` or `workos`), whose build also carries
# YOKOHAMA_SIGNIN_NOTICE, the seventh value (the guard's check 12; §3's YOKOHAMA_TURNSTILE
# bullet; §23)
cd web && YOKOHAMA_PROJECT_URL=https://example.org/yokohama YOKOHAMA_SITE_BLURB="A few words." YOKOHAMA_SITE_NOTICE="A notice." YOKOHAMA_SITE_URL=https://yokohama.example.org npm run build
cd ../worker && npm run build    # build:module → build:assets → smoke:release
```

`worker/`'s `npm run build` is three steps, each refusing by name:

1. **`build:module`** — `crates/yokohama-worker` on `--profile release-wasm`, wasm-bindgen, `wasm-opt`,
   and the factory transform that gives every object its own instance (spec §2.2). A build whose
   optimizer changed nothing leaves `gen/.unoptimized`, which the deploy guard refuses. Every build
   stamps `gen/source.json` with a digest of the sources the module is compiled from — the
   workspace's manifest, lock and toolchain, and the five crates in its graph, the bridge (a path
   dependency of the CLI's) among them — taken before cargo runs; the deploy guard takes the
   digest again and compares (R3's M-3). A cell holds that list to the graph `cargo metadata`
   names (R3's R-N1).
2. **`build:assets`** — `worker/assets/` from `web/dist`, `crates/yokohama-web/assets/art/` and
   `worker/static`, verified entry by entry against the release's own precache list. A debug browser
   module is refused. The `sw.js` it copies names the release it came from, and is what the deploy
   guard compares with the current `web/dist/sw.js`. `web/dist/headers.json` is the one file of the
   dist it does not copy (security S2a, decision D-1): it reads the file and appends its pairs to the
   copied `_headers` as the `/*` block, the header set both hosts serve, and a dist without the file
   is refused, naming it.
3. **`smoke:release`** — the release module instantiated once, in Node: `init` with the archive off,
   then `GET /api/health` as `smoke@x.test`. It refuses unless the answer is 200, and refuses a debug
   module. Nothing else runs the release module before a deploy.

**The order matters twice.** `web/`'s `npm run e2e` leaves a DEBUG browser module in `web/dist`, so
`web/`'s `npm run build` comes first, every time. And `worker/`'s own tests run the DEBUG module:
after a release build, `npm run build:module:debug` comes before `npm test` in `worker/`.

## 2. Deploy

```sh
cd worker && npx wrangler whoami   # the owner's login, account BDVC
npm run deploy
#   node scripts/deploy-guard.mjs --config wrangler.jsonc && node scripts/secrets-guard.mjs --config wrangler.jsonc && wrangler deploy --config wrangler.jsonc
npm run deploy:open                # the same three steps, each with --config wrangler.open.jsonc (§19, §20) — on THIS Worker only as a known exercise of the review; steady after it (§3's C1 rule)
npm run deploy:dev                 # the same three steps, each with --config wrangler.dev.jsonc: the dev door on, around a live tier (§7)
npm run deploy:private             # the same three steps, each with --config wrangler.private.jsonc: the LATER private Worker (`yokohama-web-private`, source `access`), its domain `yokohama-test.moore.nyc` named at security S3's Task 7 (§26); the FIRST deploy was by hand without the secrets guard, every later one through this script
```

**The deploy guard** refuses, naming every failure: a missing, DEBUG or unoptimized module; a
module whose sources have changed since it was built (`gen/ was built from sources that have since
changed`), or that carries no stamp; an assets tree that is not the release its `sw.js` lists, or
that holds a debug browser module; an assets tree assembled from a `web/dist` that has changed since
— its `sw.js` against `web/dist/sw.js`, byte for byte, and the refusal names both releases — or
whose copy of a `worker/static` file is not that file (R3's M-3: `wrangler deploy` bundles `src/`
fresh but ships `gen/` and the assets tree as they sit, so a skipped rebuild would ship the old
module, or the old client);
`workers_dev` or `preview_urls` not `false`; the test clock named anywhere in `wrangler.jsonc`; an
`assets` block that is not spec §2.3's; the two configurations' `durable_objects`, `migrations`
or `r2_buckets` blocks differing, or either file lacking one; and `YOKOHAMA_WEB_ADMINS` among
`wrangler.jsonc`'s `vars`, the top level's or any environment's — the operator list is a secret
(§3), never a committed value. `npx wrangler deploy --dry-run` bundles and validates without
uploading.

*(Corrected 2026-09-28, M6.5c2: the guard reads the door since M6.5c2, so three clauses above no
longer hold as written.)* It refuses a `wrangler.jsonc` whose `vars` name no door, and prints the
door it reads before its verdict — `wrangler.jsonc deploys the worker door — Cloudflare Access,
verified by the shell`. Its URL rule is the door's: `preview_urls` must be `false` under both doors;
under `worker`, `workers_dev` must be `false` and `ACCESS_TEAM_DOMAIN` and `ACCESS_AUD` must be set;
under `open`, a `workers.dev` URL is allowed and `YOKOHAMA_DEV_DOOR=on` is refused. The pool has two
configurations, `wrangler.test.jsonc` and `wrangler.open.test.jsonc`, and each is held to
production's `assets`, `durable_objects`, `migrations` and `r2_buckets` blocks. And
`DEV_DOOR_CLIENT_ID` among `wrangler.jsonc`'s `vars` is refused beside the operator list, since it
is a secret too (§3).

*(Corrected 2026-10-11, security S2a: the clause above that allows a `workers.dev` URL under `open` no
longer holds.)* Every deploy script names its
configuration to the guard, to the secrets guard and to wrangler alike, `--config` explicit (finding
D18). The guard also refuses:
- **a redirect file**, `.wrangler/deploy/config.json`, in `worker/` or at the repository root: with no
  `--config`, wrangler can follow one to a configuration the guard never judged;
- **`workers_dev` not `false` under EVERY door** (finding C3): under `worker` a `workers.dev` URL goes
  around the hostname's Access application, and under `open` around the edge rule, the rate limit and
  Bot Fight Mode. A door the guard does not know is asked the same;
- **an open-door file that is not production's sibling**: every configuration judged by `--config`
  whose door is `open` is held to production exactly as `wrangler.open.jsonc` is — production but for
  the open door's own `vars`: the door `open`, no Access pair, no dev door, `YOKOHAMA_TURNSTILE` `on`;
- **an assets `_headers` that is not `worker/static/_headers` followed by the current
  `web/dist/headers.json`'s `/*` block**, as the assembly writes it (§1; decision D-1), or a `web/dist`
  with no `headers.json`, which the guard names.

*(Since security S2b, 2026-10-12.)* The guard also refuses:
- **production's Rate Limiting bindings at any figure but their literals** (check 13; finding C2):
  `wrangler.jsonc`'s `ratelimits` block must be exactly `CREATE_LIMIT` 10 per 60 s and `WRITE_LIMIT` 120
  per 10 s, namespaces `7301` and `7302` (`RATE_LIMITS`), read as values, and each pool file must bind
  exactly those two names, at any figure — the pool's are lenient, since `test/worker.test.ts` sends
  more creates through `handle` than production allows; the open and dev files carry production's block
  by the sibling rule. A `namespace_id` is a string, unique to the account, and a shared id shares
  counters across Workers: confirm no other Worker on the account uses `7301` or `7302` before the
  first deploy that binds them;
- **production's logs keeping query strings, or sampled** (check 14; finding D23):
  `observability.redact_query_string` must be `true`, and `head_sampling_rate` absent or `1` — at the
  top level and under `observability.logs`, whose `invocation_logs` must not be `false`, and the logs never switched off (`enabled`, `logs.enabled`, `logs.persist` never `false`);
- **the dev door open anywhere but `wrangler.dev.jsonc`** (finding B6; decision D-1): `YOKOHAMA_DEV_DOOR`
  `on` in production's file or in any other worker-door file is refused, the dev file must set it to
  exactly `"on"`, and the dev file is held to production but for that one variable;
- **an unlisted worker-door file that is not production exactly** (the S2a plan review's F-40): every
  configuration judged by `--config` whose door is `worker` and which is neither production nor the dev
  file is held to production with no allowed difference — a deploy of it would otherwise roll production
  back, or open a door no committed file opens.

A clean run prints its door line, then one sentence summing up what it held, these clauses among them:
`… at the open door no dev door, the dev door in wrangler.dev.jsonc alone, every unlisted file
production's sibling; … production's two rate limits at their figures, and both bound in each pool
file; production's logs redacting query strings, unsampled; …`.

**Then the secrets guard** (`scripts/secrets-guard.mjs`, decision D-11) asks the account for the
Worker's secret NAMES through `npx wrangler secret list` — never a value — and refuses a secret named
`YOKOHAMA_DIAGNOSTIC_SEEDS`, `YOKOHAMA_WEB_TEST_CLOCK` or, since the S2 stack review,
`YOKOHAMA_DEV_DOOR`: the two switches the guard refuses in a committed file and the one it judges
there, which a secret would carry past it: `secrets-guard: the Worker holds a secret named
<name> — <why>; delete it: npx wrangler secret delete <name>`. A listing that fails or does not parse
is a refusal, never a pass; a clean one prints `secrets-guard: <n> secrets on <file>'s Worker, none
named like a switch`.

**It also REQUIRES an arm's secrets, by the judged file's source** (the auth cycle's R-48, Task 7's
review): the shell answers every request 500 without them (`config.ts`'s `signin` and `session`), and the
deploy guard, which reads files, cannot see a secret. The command reads the judged file's door and source
with the deploy guard's own reader, from the working directory where wrangler resolves `--config`, and
beside the `worker` door requires in the listing: under `workos` BOTH `YOKOHAMA_OIDC_CLIENT_SECRET` (the
WorkOS API key) and `YOKOHAMA_SESSION_SECRET`; under `oidc` `YOKOHAMA_SESSION_SECRET` alone (the client
secret is optional there: PKCE); under `access` and at the open door nothing. A missing one refuses the
deploy, naming it and its line, `secrets-guard: <file>'s Worker holds no secret named <name>, which
YOKOHAMA_IDENTITY_SOURCE=<source> requires — <why>; without it the shell answers every request 500: npx
wrangler secret put <name> --config <file>`; a clean run then ends `…, and the <source> arm's <names> among
them`. A worker-door file with no source, or one the shell does not know, names no arm to judge the listing
against, so the secrets guard refuses it too, in the shell's sentence with the file named (`secrets-guard:
<file>'s YOKOHAMA_IDENTITY_SOURCE must be access, oidc or workos …; the secrets it requires cannot be judged
without one of the three`). A file whose door is absent, or neither `worker` nor `open`, is refused the same
way (`secrets-guard: <file>'s YOKOHAMA_WEB_IDENTITY must be worker or open …; the secrets it requires cannot be
judged without a door`). The deploy guard refuses such a file first in every deploy script, so only the secrets
guard run alone meets either. The listing names secrets and never holds a value, so the session secret's
32-byte floor stays the shell's to refuse.

**What the secrets guard costs.** Every deploy script is a network step from here on — the plain
`deploy`, the safe direction since security S2b, included. A logged-out clone runs `npx wrangler login`
once, and one with more than one account sets `CLOUDFLARE_ACCOUNT_ID` (`npx wrangler whoami` lists them
and caches nothing). A NEW Worker's first deploy cannot pass it: wrangler answers "Worker not found"
for a Worker that does not exist yet. **So a NEW Worker's first deploy runs the deploy guard and then
`wrangler deploy --config <file>` by hand, skipping the secrets guard alone — a Worker that does not
exist holds no secret, while the deploy guard reads files and judges the new file against its record in
`DEPLOYMENTS`, which holds each file to its own deployment identity: production's `yokohama-web` with the
bucket `yokohama-archive`, and the private Worker's `yokohama-web-private` with its own bucket and
rate-limit namespaces (the auth cycle's Task 7; plan decision D-1) — and from the second deploy on both
guards, as every `deploy` script in `package.json` chains them.** Security S2b's decision D-7, as corrected
at Task 9's review (the owner's R-6), read "with neither guard" while the deploy guard held this repository's
Worker alone, by its name and its bucket; the `DEPLOYMENTS` record superseded it, recorded forward here,
and the dated S2b plan stays as written. The deploy guard refuses any other Worker's file on its `name`
and its bucket, whatever its door: C3's sibling rule holds every open-door file to production, and F-40
every unlisted worker-door file (measured at that review: a renamed open file and a renamed production
file, each with its own bucket, both refused). This recipe was the private Worker's first deploy (§26,
step (5)); the deploy guard holds its record in `DEPLOYMENTS` since the auth cycle's Task 7, and §26's
step (2) named its hostname there:

```sh
node scripts/deploy-guard.mjs --config <file> && npx wrangler deploy --config <file>   # a NEW Worker's FIRST deploy: the deploy guard, not the secrets guard
node scripts/deploy-guard.mjs --config <file> && node scripts/secrets-guard.mjs --config <file> && npx wrangler deploy --config <file>   # from its second deploy on: both guards, as every deploy script in package.json runs them
```

- **A deploy restarts every object and closes every socket with 1006** (spec §5.5). The client
  probes once and reconnects, an open picker is re-fed — or dropped, when the deploy changed the
  engine's, the CLI's or the layer's sources (`picker-dropped`, §5) — and no acknowledged move is
  lost; but it is still a reconnect for everyone at every table, so deploy between games.
- **The migration `v1` is FROZEN from the first deploy on** (the orchestrator's ruling I-30): a
  class added later takes a new tag. Cloudflare's own docs now prefer a declarative `exports` field
  for NEW Workers, and keep the `migrations` array this Worker uses fully supported; the two may not
  be mixed.

## The archive's rename, S1c — once, in the deploy window (a plan, not a record)

**The archive's rename (security S1c, finding D26) — once, in the deploy window.** A browser's saves are filed
under `browser/<segment>/`, and the segment changed: the readable half cut at 48, then `-` and a twelve-character
digest. Every folder written before this release sits under the legacy segment and nothing reads it until it is
moved. (The recipe's two holes the reviewer measured on 2026-10-10 — a bare `mv` nesting silently, a split by a
sender the files never name — are closed by steps 1 to 4 as written here.) The window renames both hosts
together (`deploy/RUNBOOK.md`'s section of this name is the same recipe), so the steps name both; on this host the
R2 lines are the ones run.

1. **Before the window: the listing, then the table.** List every first-level folder on both hosts first — on the NAS `ls -A /data/archive/browser` (while the old container still runs, `docker exec yokohama-web ls -A /data/archive/browser` through Portainer; `-A`, since a local part may begin with a dot), in R2 the `browser/` and `test/browser/` prefixes — and give EVERY folder a row or a recorded ruling before the window: a folder whose name holds `_at_` is an address's legacy segment; a folder with no `_at_` is a bare claimed name from the dev or the open door (the NAS ran that door until 2026-09-28), whose address is the folder's own text; a folder nobody can name is left in place and recorded. Then run the `segment` subcommand over the WHOLE Access allow-list, over every `.test` identity whose folders exist under R2's `test/browser/` (the live replay's `red@wire.test` among them) and over every bare name found
   (`cargo run -p yokohama-web -- segment <every address>…`) and keep the three-column table. A legacy column two
   addresses SHARE is a collision, and the files inside name no sender — every browser-played table's
   `created_by` and seats read `local@browser` — so the owner rules on that folder BEFORE the window (by the
   players' word, or by filing a copy under each address); the steps below assume no shared legacy column and
   STOP if one appears.
2. **NAS — between the swap's stop and start.** Stop the old container. With nothing serving, for each legacy
   folder present under `/data/archive/browser/`:
   `[ ! -e browser/<current> ] && mv browser/<legacy> browser/<current>` — a GUARDED move, never a bare `mv`,
   which exits 0 and nests the old folder under a new one that already exists (a `<current>` folder can exist only where an S1c build already served on this volume — a re-run of the window, a test deploy — since the old build files under legacy segments alone) — and if `<current>` exists, merge FILE BY FILE — `cd browser/<legacy> && find . -type f | while read -r f; do mkdir -p "../<current>/$(dirname "$f")"; mv -n "$f" "../<current>/$f"; done` — then `find browser/<legacy> -type f` lists exactly the files `-n` refused, each compared by hand (a directory-level `mv -n` nests one level deeper than a second-level reading and is never used); read every file `-n` refused (it
   refuses without a word, and its exit code is no signal across `mv` versions, so the refused files are the
   ones still under `<legacy>` afterwards). When `find browser/<legacy> -type f` lists nothing more — every refused file ruled on and moved or removed by hand — remove the emptied skeleton, `find browser/<legacy> -depth -type d -exec rmdir {} +` (BusyBox's `rmdir` is native; the merge leaves the directories behind, and an empty legacy tree prints at step 4 exactly as an unmoved one does). Then start the new build.

   (`deploy/RUNBOOK.md`'s section of this name says how the lines reach the volume while the container is
   stopped.)
3. **Worker — after the deploy.** The Worker's deploy is one atomic switch, so the rename follows it. For each
   key under `browser/<legacy>/` AND `test/browser/<legacy>/` (spec §2.6's root for a `.test` address — the live
   replay's own — EXCEPT the replay's own four keys under `test/browser/red_at_wire.test/`: the `wire` record the replay sends carries the stack's `engine_version` 0.6.1 where those saves carry 0.6.0, so a moved save would meet the first replay's first sends as the 409 that stops a run (`worker/RUNBOOK.md` §7, "The replay's reset"); DELETE those four legacy keys instead, `npx wrangler r2 object delete yokohama-archive/test/browser/red_at_wire.test/rev7/<over|abandoned>/wire[.table].json --remote`, the reset in its legacy spelling, and the first replay's two first sends then land 201 and re-create them under the digested key) in `yokohama-archive` (the dashboard's object listing, or the API's
   `GET /accounts/{account}/r2/buckets/yokohama-archive/objects?prefix=browser/<legacy>/`):
   `npx wrangler r2 object get yokohama-archive/<new key> --remote --file <tmp-new>` FIRST — if the new key
   exists (an upload that landed after the deploy), compare the bytes: identical is the once-rule's retry, so
   delete the old key; different is a conflict the owner rules on — otherwise
   `npx wrangler r2 object get yokohama-archive/<key> --remote --file <tmp>`,
   `npx wrangler r2 object put yokohama-archive/<new key> --remote --file <tmp>`, read the new key back, then
   `npx wrangler r2 object delete yokohama-archive/<key> --remote`. Without `--remote`, wrangler acts on local
   storage and the bucket never moves. Run in `worker/` with wrangler's own login (the owner's R2 key is
   read-only); the archive writes no object metadata, so the copy loses nothing.
4. **Verify, both hosts.** A fresh upload from that address lands under `browser/<current>/`; on the NAS the whole shape reads clean — `find /data/archive/browser -mindepth 1 -type d | grep -vE '^/data/archive/browser/[a-z0-9._-]+-[0-9a-f]{12}(/rev[0-9]+(/(over|abandoned))?)?$'` prints NOTHING (a legacy folder at the first level, a nest at the second, or a merge's nest at the fourth all print, and a legacy skeleton of empty directories prints too — it means step 2's merge was not finished); in R2 the same shape over `browser/` AND `test/browser/` (a key listing, each key's second and fourth components read); the address's record count matches the files moved. Record the moves, and any ruling of step 1, in the deploy entry.

## 3. The variables — every one, and what its absence means

`wrangler deploy` REPLACES the Worker's variables with `wrangler.jsonc`'s `vars`, so that file is
the whole list; it leaves secrets alone.

| Variable | At the seventeenth deployment | Absent means |
|---|---|---|
| `YOKOHAMA_WEB_IDENTITY` | *not a variable at the seventeenth; added 2026-09-28 (M6.5c2) and first deployed at the twenty-first (§15):* the door, `worker` in `wrangler.jsonc` (R-6.5c2.5); `open` is the other door — no deployed Worker runs it, and a door window opens it on THIS Worker only as one of the review's known exercises — operationally the door stays steady after the security process (the C1 rule below; `wrangler.open.jsonc` deployed it for twelve minutes on 2026-09-30, §19's state 1, reverted the same hour, before the rule) | there is no default: the API answers the host-written 500 naming it, as it answers any bad value (the first bullet below), and the static release is still served; the deploy guard refuses a `wrangler.jsonc` that names no door |
| `YOKOHAMA_IDENTITY_SOURCE` | *not a variable at the seventeenth; added 2026-10-19 (the auth cycle, Task 1; design §4.1):* who vouches for an address under the `worker` door — `access`, `oidc` or `workos`, matched exactly after a trim, ONE per deployment. `wrangler.jsonc` and `wrangler.dev.jsonc` ship `workos` (since stage 1, §23; the hostname stays on Access until stage 2); `wrangler.private.jsonc` and the pool's `wrangler.test.jsonc` ship `access`. Under `access` the Worker runs exactly the Access assertion's reading as before; under `oidc` and `workos` it reads its own session cookie and no assertion | there is no default: under the `worker` door the API and `/enter` answer the host-written 500 naming it, and the deploy guard refuses a worker-door file that names none; **set beside `open` it is the 500 too** (a name is a claim there and no source vouches for it) |
| `YOKOHAMA_OIDC_ISSUER` | *not a variable at the seventeenth; added 2026-10-19 (the auth cycle, Task 1; design §4.1):* the issuer URL whose `/.well-known/openid-configuration` names the endpoints — an `oidc` deployment's, and nobody's in this repository's files | under `oidc` absent is the 500 naming it; **set beside `workos` is the 500** (WorkOS's issuer is `https://api.workos.com`, fixed); ignored under `access`, and the deploy guard requires it at `oidc` and refuses it at `workos`. The issuer and the four endpoints of its discovery document must be `https:` with no fragment, and the issuer with no query either (OpenID Connect Discovery 1.0 §3), `http:` accepted only on a loopback host (`localhost`, `127.0.0.1`, `[::1]`, for development; R-24) |
| `YOKOHAMA_OIDC_CLIENT_ID` | *not a variable at the seventeenth; added 2026-10-19 (the auth cycle, Task 1; design §4.1):* the client id the provider registered for this site — under `workos` the production environment's `client_…` id | under `oidc` and `workos` absent is the 500 naming it; ignored under `access`; the deploy guard requires it at both arms |
| `YOKOHAMA_OIDC_CLIENT_SECRET` | *not a variable at the seventeenth; added 2026-10-19 (the auth cycle, Task 1; design §4.1):* a SECRET (`wrangler secret put`), never a `vars` entry (check 7) — under `workos` the WorkOS API key the code exchange is authenticated with, from the production environment | **under `workos` required**: absent is the 500 naming it, and the secrets guard refuses the deploy first (R-48). **Under `oidc` optional**: absent is a public client — PKCE alone, the challenge sent only when no secret is set (R-25); with one, the provider is asked for `client_secret_post`, and an issuer that accepts only `client_secret_basic` is out of scope (R-20) |
| `YOKOHAMA_SESSION_SECRET` | *not a variable at the seventeenth; added 2026-10-19 (the auth cycle, Task 1; design §4.1):* a SECRET (`wrangler secret put`): the HMAC key of both cookies, 32 UTF-8 bytes or more, taken exactly as set (never trimmed) — `openssl rand -base64 48` | under `oidc` and `workos` absent or shorter than 32 bytes is the 500 naming it, and the secrets guard refuses the deploy on its absence (R-48). **Rotating it ends every session at once; since 2026-10-19 a session also ends at `YOKOHAMA_SESSION_MAX_DAYS` from its first issue, and `YOKOHAMA_DENIED` ends one address's (the two rows below). Before that day rotation was the only way to end a stolen one** (renewal re-stamps the time; R-16); the live instruments read it from their own environment (never an argument, never echoed) to mint `.test` sessions only — the replay sends the cookie, the live tier seeds each context's cookie jar — and it wins over `YOKOHAMA_SESSION_COOKIE`, the one-identity mode (`scripts/mint-session.mjs`, R-50, R-52–R-54; §23) |
| `YOKOHAMA_SESSION_DAYS` | *not a variable at the seventeenth; added 2026-10-19 (the auth cycle, Task 1; design §4.1):* `30` when absent — whole days the session cookie stands (renewed on a 2xx answer once less than half the configured days are left) | 30 days; a value that is not a whole number of days above zero (the TTLs' own rule) is the 500 naming it; read under `oidc` and `workos` only |
| `YOKOHAMA_SESSION_MAX_DAYS` | *not a variable at the seventeenth; added 2026-10-19 (the owner's ruling after stage 1's read; the auth cycle's R-16 amended):* `90` when absent — whole days of the ABSOLUTE cap on a session, counted from its first issue, which every renewal keeps: a cookie's expiry is the earlier of its issue plus `YOKOHAMA_SESSION_DAYS` and the first issue plus this, a renewal is sent only when it would extend the expiry, and at the cap the player signs in again. The cookie's version moved to `v2` with the field; a `v1` cookie reads as nobody | 90 days; a value that is not a whole number of days above zero is the 500 naming it; a cap below the days is not refused (such a session lives the cap); read under `oidc` and `workos` only |
| `YOKOHAMA_DENIED` | *not a variable at the seventeenth; added 2026-10-19 (the red button, the owner's ruling):* a SECRET (`wrangler secret put`, never a `vars` entry — check 7) — addresses and `@domain` entries separated by commas or newlines, trimmed and lowercased, empty entries dropped: a session naming one, or at a denied domain, is nobody (the 401 with the sign-in challenge, its cookie cleared with it), and the callback refuses the address after the exchange (`signin-refused`, reason `denied`, the address never logged). §24 is the recipe | absent: nobody is denied; an entry that is neither an address nor an `@domain` is SKIPPED, the log line `denied-skipped` naming its position among the entries kept and never its value (the owner's ruling, 2026-10-19: a typo is never an outage), and the rest stand; a domain entry denies that exact domain, not its subdomains; the list is read on EVERY request, so an entry added denies a standing session at its next request and one removed re-admits a session that still stands (only a cookie already cleared by a denied answer is gone), while lowering the cap reaches no cookie already minted — a session lives to its minted expiry; a malformed entry left standing logs `denied-skipped` on every request, since the configuration is read per request — put the list again; read under `oidc` and `workos` only |
| `ACCESS_TEAM_DOMAIN` | `bdvc.cloudflareaccess.com` — **required at source `access` only**; at `oidc` and `workos` the deploy guard refuses a file that sets it (the shell would ignore it) | no assertion can be verified: every caller is no one, and no key is fetched |
| `ACCESS_AUD` | the `yokohama` application's audience — required at source `access` only, as above | the same |
| `DEV_DOOR_CLIENT_ID` | the `yokohama-worker` credential's client id, its public half. *(Corrected 2026-09-28, M6.5c2: a Worker SECRET since the twenty-first deployment (§15), put in the same deploy that dropped the variable, and refused in `wrangler.jsonc`'s `vars` by the deploy guard; it was a committed variable until then, and stays in the history.)* | the dev door is shut, whatever `YOKOHAMA_DEV_DOOR` says |
| `YOKOHAMA_DEV_DOOR` | `on`, for this cycle's live tests. *(Corrected 2026-10-12, security S2b, finding B6: `off` in `wrangler.jsonc` — the standing state is Access alone, the shape a self-hoster copies; `wrangler.dev.jsonc`, production with this one variable `on`, opens it around a live tier — `npm run deploy:dev` before it, the plain `npm run deploy` after it (§7) — and the deploy guard refuses `on` in any other worker-door file. The dev door names a `.test` address only.)* | off: an assertion naming only the credential is no one |
| `YOKOHAMA_WEB_ARCHIVE` | `r2`, over the bucket `yokohama-archive` | off (`r2` needs the bucket bound as `ARCHIVE`) |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | `90` | no table ever expires |
| `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` | *not a variable at the seventeenth; added 2026-09-30 (M6.5c2 cycle E, R-6.5c2.42):* `7` in `wrangler.jsonc` — how long a table that has not started — open, or dealing with a keep outstanding — stands (below). *(From 0.7.2, security S3, D9: a dealing table too; before it a dealing table waited the 90 days.)* | 7 days; and with `YOKOHAMA_WEB_TABLE_TTL_DAYS` absent it is inert, since no table expires |
| `YOKOHAMA_WEB_ADMINS` | set, as a SECRET (`wrangler secret put`, the plan's ruling P-1), its value never shown — a refusal names an entry by its position, never its value; the deploy guard refuses `wrangler.jsonc` if its `vars` name it | the operator routes are the layer's 404 |
| `YOKOHAMA_WEB_COUNT_WAKES` | unset — the exit's window sets it for one deploy, and the next deploy unsets it (§7) | off: no wake is counted |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `on` | on |
| `YOKOHAMA_WEB_CHAT_KEEP` | `500` | 500 messages |
| `YOKOHAMA_WEB_CHAT_TEXT` | `500` | 500 characters |
| `YOKOHAMA_TURNSTILE` | *not a variable at the seventeenth; added 2026-10-01 (M6.5c2 cycle F, R-FP2, R-FP4):* a DEPLOY DECLARATION, `on` in `wrangler.jsonc`, `wrangler.dev.jsonc` and `wrangler.open.jsonc` (production and the dev file since stage 2's commit, 0.7.3), `off` in `wrangler.private.jsonc` and the pool's two. It rides into the Worker's variables like every `vars` entry, and nothing in the Worker reads it but the check that every variable is a string. The deploy guard's check 12 and the smoke's live mode read it (the bullet below). *(The auth cycle: stage 1 left it `off`; stage 2's commit (§23) set it `on` in `wrangler.jsonc` and `wrangler.dev.jsonc`, beside source `workos`, and the guard takes a keyed build alone from there.)* | off: the guard refuses a keyed build beside it, and the smoke's live mode owes no edge line |
| `YOKOHAMA_WEB_TEST_CLOCK` | **never set in production** — the pool's files only, `wrangler.test.jsonc` and, since M6.5c2, `wrangler.open.test.jsonc` *(corrected 2026-09-28)*; the deploy guard refuses `wrangler.jsonc` if it names it | the wall clock |
| `YOKOHAMA_DIAGNOSTIC_SEEDS` | *not a variable at the seventeenth; added 2026-10-09 (security S1a, finding B3; R-SEC8(b)):* **never set in production** — `on` in the pool's two files only, `wrangler.test.jsonc` and `wrangler.open.test.jsonc`, so the corpus walk's seeded tables with an open seat are honoured there; the deploy guard refuses any deployed configuration that names it (rule 4b, beside the clock's check 4) | off, the production rule: a creator's seed is honoured only at a table whose every human seat is the creator's, and refused otherwise in the layer's sentence ("a seed is honoured only at a table whose every human seat is yours — leave it blank and the server draws one") |

- **A bad value answers 500 naming its variable** (spec §2.6), the Worker's form of the NAS refusing
  to start: a switch other than `on` or `off`, an archive other than `r2` or `off`, `r2` with no
  bucket bound, a TTL — either — that is not a whole number of days, an operator entry that is not an address,
  or any value written without quotes. The static release is unaffected, and so are a few `/api/*`
  answers, which is where the rule is not "every `/api/*`":
  - three answers come before any configuration is read: 405 for any method but GET or HEAD on
    `/api/art`, 403 for an upgrade from another site's page, and 403 for any method but GET or HEAD
    from another site's page (security S2b, B5) — the list at the top of this runbook;
  - a bad value of any variable but the three chat variables is the Worker's own refusal: 500 on
    every other `/api/*` request;
  - a bad chat variable is read only by the objects' module (`init`). So the Worker's own
    `GET /api/art` still answers 200, and so do the objects' answers that come before a module is
    built: the 405 for any method but GET on `…/events`, `…/stream` or `/api/stream`, and the 426
    for a GET of either stream route that is not an upgrade. Every other `/api/*` is the object's
    500 naming the variable.
- **An address outside visible ASCII is no caller** (Task 2's review, note E1; Task 11's review,
  I1). The Worker judges the RAW `email` claim — and the raw claimed header — before it trims or
  lowercases anything, and then lowercases ASCII only, as the layer's `User::parse` does. So a
  person whose Access `email` holds any character but a tab, a space or visible ASCII — a no-break
  space, an ideographic space, or a Kelvin sign (U+212A), which JavaScript's `toLowerCase()` would
  turn into another user's `k` — meets the layer's 401, and the Worker forwards no identity header
  for them at all. The layer's own `resolve` reads such a header value as absent, as the NAS
  does. **And a dev-door claimed value must be an ADDRESS** by `User::parse`'s rule — one `@` with
  something on both sides, no spaces or control characters, at most 254 bytes — and since security
  S2b a `.test` one (`isTestAddress`: an `@` and the suffix `.test`, the receipt archive's own rule
  for the `test/` root; finding B6), or it is no one, and the layer's 401 answers (R1's L-2). A
  claimed `not-an-email` once named an Archive object whose `init` refused it, a 500 no spec lists.
- **A populated deployment never changes its door, and a public deployment starts empty** (security
  S2b, finding C1; spec §2.2's C1 row). The two doors key their callers in two spaces — a typed NAME
  under `open`, a verified ADDRESS under `worker` — and since S2b the layer keeps them apart
  (`ClaimPolicy::NamesOnly` refuses an address claim under `open`, so an `@` always means a verified
  address). A deployment flipped from one door to the other keeps every table, seat and archive folder
  keyed under the first door's space, which nobody under the second reaches as their own. So a
  deployment that holds tables or saves keeps its door, and a public open-door deployment is a NEW
  Worker with its own objects and its own bucket (R-SEC6), starting empty. The specification's words:
  "The door of the private Worker never flips; the public deployment starts empty (R-SEC6)." The door
  windows of cycles E and F (§19, §20) flipped this Worker for minutes, as the review's windows may still: the
  owner's clarification (2026-10-12) — a flip is fine in development and inside the known exercises (the
  remediation's instruments and their windows); operationally, once the security process is through and the
  repository is public, the doors stay steady, and a flip after that is an owner decision, not an operating step. *(Added 2026-10-19, the auth cycle: this rule is about the DOOR. Moving `yokohama-web` from source `access` to `workos` (§23) changes who vouches for an address, not the door — Access and WorkOS both vouch for verified addresses, so the key space does not move and the Worker keeps every table, seat and archive folder — which is why the public deployment is `yokohama-web` itself and not a new Worker, plan decision D-1, the owner's R-A12.)*
- **To deploy with no archive** (R-6.5c.14): set `"YOKOHAMA_WEB_ARCHIVE": "off"` in `wrangler.jsonc`
  and deploy (§2). The `ARCHIVE` binding stays as it is — the deploy guard holds it equal in the
  two files — and nothing is removed from the bucket. With the switch off:
  - `/api/health` reads `archive: "off"`;
  - game over saves nothing and arms no alarm for a save;
  - a creator's Abandon deletes without a save, taking with it any save the table still owes, which
    it logs as `archive-owed-dropped`;
  - a game saved while the switch read `r2` — stamped at game over — is still in the bucket, and
    its marker says so when it is abandoned or expires: `its game is in the archive` (the
    orchestrator's ruling I-71). Only a game never saved reads `its game was not kept`;
  - `POST /api/archive` answers the layer's 404 `this server keeps no archive`, which the client
    reads as the archive being off;
  - a game-over save still owed from when the archive was on is KEPT and not retried: no request
    uploads it, no alarm is re-armed for it, and any alarm that runs — one armed before the switch,
    or one for other work — logs `archive-owed-kept`. Set back to `r2`, the table's next alarm
    resumes it, or its next request once the last failure's rung has passed (R-D6, below).
- **The TTL** (R-6.5c.15): `wrangler.jsonc` sets `YOKOHAMA_WEB_TABLE_TTL_DAYS` to `90`, the owner's
  value, from Task 14's commit on; it is live from the seventeenth deployment (§11). A table's
  last activity is the newest write of its record, its table document or its chat; a table untouched
  for the TTL expires on its alarm as its creator's own Abandon — saved first while the archive is
  on, then deleted, leaving its marker (the list at the top of this runbook). A save that fails leaves the
  table in place and is retried on the backoff (30 s, 2 min, 10 min, then hourly). A table whose
  record key already holds another game's record is `held`: no retry can clear that, so it is never
  expired again, and waits for an operator's force. A poisoned table never expires on its own
  either. With the archive off, an expired game is deleted with no save of its own — one stamped
  while the archive was on is still in the bucket, and its marker says so — and a save still owed
  from when it was on goes with it, logged `archive-owed-dropped`. **A table created before the TTL
  was set carries no expiry deadline** until its next delivery to the lobby — a change of its copy,
  which re-arms its alarm — or until an alarm runs for other work; the operator's listing names it,
  and an operator's expire or force ends it. **Nor does a LOWERED TTL move a deadline already
  armed**: after a change such as 90 → 30, an idle table's alarm stays at its old deadline, and the
  table expires then, or sooner if an alarm runs for other work, since every run re-reads the TTL. A
  RAISED TTL needs nothing: the alarm at the old deadline re-reads it, finds nothing due, and
  re-arms.
- **The waiting-table TTL** (M6.5c2 cycle E, R-6.5c2.42; from 0.7.2, security S3, D9): a table that
  has not started — open, or dealing with a keep outstanding; its status `open` or `dealing` — is
  closed after `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` days without
  activity, `7` in `wrangler.jsonc` and by default, with the marker head `this table waited 7 days
  to start and was closed on …` and the tail `— no move had been made, so there was nothing to
  keep`; its `expired` line reads `reason: "waited"`. No write can read a table's status (the
  shell never parses a table document), so **every table's alarm is armed at its last activity
  plus the SHORTER of the two TTLs** — 7 days — and the alarm decides by the status its house copy
  reads: an `open` or `dealing` table is closed; any other is re-armed to its last activity plus
  `YOKOHAMA_WEB_TABLE_TTL_DAYS`, and expires there as above. So a playing table idle for 7 days
  costs one alarm run more than before — its house copy made once more, on a module of its own, to
  read the status — and the alarm set again. The status is read as stored, never repaired first: a
  table finalized but never flipped to `playing` that nobody opened for 7 days is closed as
  `waited`, a game with no move (design D-16). A table
  already waiting when this is first deployed keeps the deadline armed before it, its last activity
  plus 90 days, until a delivery to the lobby or an alarm for other work re-arms it — as with a
  lowered TTL, above. `/api/health` reports both TTLs, `table_ttl_days` and `open_table_ttl_days`
  (both `null` while no table expires; the NAS reports `null` for both, since it keeps no TTL).
- **To keep every table** (R-6.5c.15): remove `YOKOHAMA_WEB_TABLE_TTL_DAYS` from `wrangler.jsonc`'s
  `vars` and deploy (§2). With it absent no table expires and no expiry deadline is armed, whatever
  `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` reads; the markers and the operator surface work as before.
- **`YOKOHAMA_TURNSTILE`: what `on` means, and what it does not** (M6.5c2 cycle F, R-F6, R-FP2, R-FP4,
  R-FP7; spec §2.6). One client build per door state: a deployment behind Access ships a client built
  WITHOUT the Turnstile site key and declares `off` (the private file, and production until stage 2's
  commit); the open window's file ships a client built WITH it and declares `on`, and since stage 2's
  commit (§23; 0.7.3) production and the dev file do too. **`on` means** that this deploy ships that keyed build, at the open door
  (`YOKOHAMA_WEB_IDENTITY` `open`) or a provider-signed worker door (`worker` at source `oidc` or `workos`), and that the zone's ONE edge rule is ENABLED in front of it: a
  Managed Challenge, at the edge and at zero Worker invocations, for every request to the hostname
  whose path is not one of the release's own static names, `/api/*` and `/enter` among them. A person
  passes it once through the widget's pre-clearance. The deploy guard's check 12 refuses `on` over a
  keyless build (`build-values.json`'s `turnstile_sitekey`), refuses `on` or a keyed build at any
  identity but `open` or a provider-signed worker door (source `oidc` or `workos`, the auth cycle's
  Task 7) — under Access the edge is the bot defence; the client no longer reads a site key as the
  door, it reads the server's facts — and refuses `on` in a pool file. After an `on` deploy the
  smoke's live mode must read the rule's answer, or the deploy fails its smoke:

  ```sh
  node scripts/smoke-release.mjs --live https://yokohama.moore.nyc --config wrangler.open.jsonc
  ```

  A cookie-less `GET /api/health` with a browser User-Agent must answer 403 with
  `cf-mitigated: challenge`. Then, the same way, `GET /sw.js`, and `GET /sitemap.xml` where the local
  tree's `assets/build-values.json` reads `site_url: true`, must each answer 200 with NO
  `cf-mitigated` header: the rule must not challenge the release's own files, and the deployed release
  must serve them. Exit 0 is those answers, or, under
  `off`, no request at all. Exit 1 names what came back instead and the path it came back for, a
  refused argument or declaration, or a local tree with no stamp, which asks nothing. Run it from the
  tree that was deployed, since the stamp it reads is the local one. **Run it over WARP, or from any
  client Bot Fight Mode trusts** (cycle F's window, W-1; §20): off WARP, Bot Fight Mode answers a bare
  client's request for `/sitemap.xml` with its own 403 `cf-mitigated: challenge` before the rule is
  consulted, the sitemap control reads that as the rule over-challenging, and the smoke cannot tell the
  two apart. Its first run in a window is expected to fail, since the rule is created after the deploy. **`on` does NOT mean** that the Worker does
  anything: it reads no token, calls no siteverify, holds no secret, and answers every request as it
  does under `off`, and `/enter` is unchanged. Nor does the declaration create, enable or disable the
  rule. The operator does that through the API, by the rule's id: ENABLED before the hostname leaves
  Access, and DISABLED only after Access is back, since the live tiers' service token cannot pass a
  challenge. The rule's body comes from the assets tree, and `--verify` compares the zone's copy:

  ```sh
  node scripts/edge-rule.mjs > edge-rule.json            # the rule's JSON, from worker/assets/; exit 0
  node scripts/edge-rule.mjs --verify < rule-from-zone.json   # 0 equal, 1 different, 2 cannot judge
  ```

  The JSON is `{ "expression", "action": "managed_challenge", "description": "yokohama — the
  no-Access door's edge check (F)", "enabled": true }`. `--verify` compares the first three and
  ignores `enabled` and every key the zone adds, and beside its verdict it names the local tree it
  read: the release `sw.js` names and the stamp's `site_url`. Build the keyed client and assemble the tree BEFORE
  printing the rule. The expression lists the tree's top-level names, and `sitemap.xml` is there only
  when the build set `YOKOHAMA_SITE_URL`.

### The operator — `YOKOHAMA_WEB_ADMINS` and the curl playbook (spec §2.9)

**Who.** An operator is a PERSON — an Access login whose `email` claim is on the list — never the
`yokohama-worker` credential, even when it claims a listed address through the dev door. Over WARP,
`curl` is the owner's own login, as long as the request does NOT present the credential: Service Auth
outranks WARP (spec §0.3 row 1), and a request carrying it is the credential. So the playbook below
sends no header file.

**The list** is the secret `YOKOHAMA_WEB_ADMINS` (the plan's ruling P-1): comma-separated addresses,
read trimmed and lowercased, never written into a committed file. `wrangler secret put` reads the
value from stdin, and a deploy leaves a secret alone.

```sh
cd worker
printf '%s' 'someone@example.com' | npx wrangler secret put YOKOHAMA_WEB_ADMINS   # set, or replace the list
npx wrangler secret list                                                        # names only, never values
npx wrangler secret delete YOKOHAMA_WEB_ADMINS                                  # the surface off: its routes are the layer's 404
```

**A malformed entry takes every `/api/*` down** with a 500 naming the variable, as every bad
variable does (§3's first bullet): an entry must be one address — one `@` with something on both
sides, no spaces, at most 254 bytes — and the list must hold no empty entry. The 500 names the entry
by its POSITION and never its value, which is an address (R3's M-6): `YOKOHAMA_WEB_ADMINS entry 2 of
3 is not an address — …`, or `… entry 3 of 3 is empty` for a trailing comma. Put the list again,
corrected.

**Since stage 1 (source `workos`, 2026-10-19) the three calls need the SESSION COOKIE, not WARP:** the Worker names
a caller by its own `__Host-yokohama_session` cookie and nothing else, so a bare curl over WARP is nobody and meets
the 401 (§23's first read). Two ways: from the site's own DevTools console, signed in as the operator — the page's
`fetch` carries the cookie and passes the same-origin fence — `await (await fetch('/api/admin/tables')).json()` and
`await fetch('/api/admin/tables/<id>/expire', { method: 'POST', headers: { 'content-type': 'application/json' } })`;
or from the shell with the cookie's value copied from DevTools' cookie panel, `-b '__Host-yokohama_session=<value>'`
on each curl below (a request with no `Origin` passes the fence, §4). While the hostname is on Access (stage 1) the
curl still runs over WARP for the edge; after stage 2 WARP plays no part. The block below is the `access`-era text.

**The three calls**, over WARP as the operator:

```sh
# The browser User-Agent every curl at either host carries (the smoke's own; §7's Bot Fight Mode bullet says why).
# This is the ONE definition: §4, §5 and §7 use the variable as set here. It works from any directory in the clone
# (the `cd` is inside the substitution, to the repository's own worker/), and leaves the shell where it was.
BROWSER_UA=$(cd "$(git rev-parse --show-toplevel)/worker" && node -e 'import("./scripts/smoke-release.mjs").then((m) => console.log(m.BROWSER_UA))')
B=https://yokohama.moore.nyc
curl -s -A "$BROWSER_UA" "$B/api/admin/tables" | jq                                              # the listing
curl -s -A "$BROWSER_UA" -o /dev/null -w '%{http_code}\n' -X POST -H 'content-type: application/json' "$B/api/admin/tables/<id>/expire"        # expire: 204
curl -s -A "$BROWSER_UA" -w '\n%{http_code}\n' -X POST -H 'content-type: application/json' "$B/api/admin/tables/<id>/expire?force"   # force: 204 or 503
```

**The two POSTs carry `content-type: application/json`** (security S2b, finding B5): without it the
shell answers the layer's own 400, `` Expected request with `Content-Type: application/json` ``, so a
hostile page's form or text POST — which a browser sends without a preflight, with a signed-in
operator's cookie — can never reach an expire; curl sends no type unless told.

- **The listing** is every table the lobby holds a copy of, whatever its status — dealing ones
  included, which the lobby's own pages never show a stranger — each with `id`, `name`, `status`,
  `players`, `created_at` (unix SECONDS, the layer's), `last_active` (unix SECONDS, the shell's
  milliseconds cut where the row is built), `bytes` (the table object's SQLite size), `over`,
  `winner` and `archived_at` (unix seconds) — every time the listing carries is unix seconds.
  *(Corrected 2026-09-28, M6.5c2: `last_active` was the shell's MILLISECONDS until then; the slot
  and the house's copy still keep milliseconds.)* `last_active` and `bytes` are as of the table's
  last delivery to the lobby: a chat post or a move within a turn does not refresh them. The
  listing does not say that a table is `held` or poisoned; the log does (`held`,
  `expiry-blocked`, `poison`).
- **Expire** ends a table NOW, exactly as its expiry would — its creator's own Abandon, the save
  first, the marker after, with the head `removed by an operator`. It answers the table's own
  answer: 204; the layer's refusal naming the archive when the save fails (nothing is deleted); the
  marker's 404 for a table already ended; the poison 500 for a poisoned one.
- **Force** ends the object whatever it holds — a table, a poisoned one, a `held` one, a marker —
  with no save and NO marker, so the id is free at once: 204. A save the table still owed is
  dropped, logged `archive-owed-dropped`. When the lobby does not confirm the removal it answers
  503, a create at the id is 409 meanwhile, and the table's alarm finishes the force. On an id that
  holds nothing, 204, writing nothing. It is how a poisoned or `held` table leaves, how a table whose
  copy the lobby refused (`copy-refused`) leaves, and how the live corpus replay frees its fixed
  ids. **`?force` takes no value** (the orchestrator's ruling I-72): with one — `?force=false`,
  `?force=1` — the answer is the 400 naming the switch, which neither forces nor expires; `?force=`
  forces. Keys compare exactly, so `?Force`, or a second key beside `force`, is the layer's 404.

## 4. The credential

**`yokohama-worker`** is an Access Service Auth credential, admitted by the `yokohama` application's
policy "yokohama ST bypass" (`non_identity`, precedence 3). It was created 2026-09-23T01:01Z and
expires in 2031 (spec §0.5).

- **Where it lives.** The repository root's `.dev.vars`, gitignored since `952a68c`, holds two
  lines, `CF-Access-Client-Id=…` and `CF-Access-Client-Secret=…`, named as the headers Access
  expects. The client id is public, and the Worker holds it as the SECRET `DEV_DOOR_CLIENT_ID`
  since the twenty-first deployment (2026-09-28, §15), put in the deploy that dropped the variable,
  from `.dev.vars` through a mode-600 file that was deleted after. *(Corrected 2026-09-28, M6.5c2: `wrangler.jsonc`
  carried the client id as a variable until then.)* The secret is in no other file, and is never
  printed, committed or pasted.
- **What it does.** A request presenting both headers gets an assertion whose `common_name` is the
  client id and which names no `email`, and Service Auth outranks a WARP identity (spec §0.3 row 1).
  With `YOKOHAMA_DEV_DOOR` on, such a request names its caller in `X-Yokohama-Claimed-User` — a
  `.test` address only, since security S2b; without that header it is no one. Access also sets a
  `CF_Authorization` cookie, which carries the credential on later requests from the same browser.
- **The header-file recipe** — the secret never on argv, never on screen. From the repository root:

  ```sh
  H="$SCRATCH/access.headers"    # SCRATCH: a directory of your own, e.g. the session scratchpad
  node -e '
  const fs = require("fs");
  const v = Object.fromEntries(fs.readFileSync(".dev.vars", "utf8").split(/\r?\n/)
    .filter((l) => l.includes("=")).map((l) => [l.slice(0, l.indexOf("=")), l.slice(l.indexOf("=") + 1).trim()]));
  const names = ["CF-Access-Client-Id", "CF-Access-Client-Secret"];
  if (!names.every((n) => v[n])) { console.error(".dev.vars lacks one of " + names.join(", ")); process.exit(1); }
  fs.writeFileSync(process.argv[1], names.map((n) => n + ": " + v[n]).join("\n") + "\n", { mode: 0o600 });
  ' "$H" && chmod 600 "$H"
  curl -s -A "$BROWSER_UA" -H @"$H" https://yokohama.moore.nyc/api/health
  rm -f "$H"
  ```

- **In a browser.** `node scripts/read-deploy.mjs install|console <base-url> --headers "$H"` sends
  the two headers to the Worker's own origin ONLY: sent to every origin, as a browser context's
  `extraHTTPHeaders` would send them, they would reach third parties such as the Web Analytics
  beacon. The live Playwright tier never hands the credential to Playwright at all (§7; the
  orchestrator's ruling I-75): Node's own `fetch` presents it once, and each context carries the
  `CF_Authorization` cookie Access answers with, which the browser keeps to this origin. What that
  tier DOES send context-wide is the claimed address, `X-Yokohama-Claimed-User`, which is no secret
  — and which, as a custom header on the cross-origin beacon request, forces the CORS preflight whose
  two console lines the live walk forgives exactly.
- **To rotate it.** Zero Trust → Access controls → Service credentials → Service Tokens →
  `yokohama-worker` → **Rotate secret**. The client id stays the same, so `DEV_DOOR_CLIENT_ID` does
  not change; Access issues a new secret, and keeps the old one valid for the grace period you choose
  (one hour to 30 days, or revoke at once). Write the new secret into `.dev.vars` before the grace
  period ends. **Refresh** extends the expiry by a year, and a token can be turned OFF without being
  deleted. A NEW token has a new client id: put it as the secret `DEV_DOOR_CLIENT_ID` (`wrangler
  secret put`, reading the value from standard input, as §3 puts the operator list), and add the
  token to "yokohama ST bypass". *(Corrected 2026-09-28, M6.5c2: this said to update the committed
  variable and deploy, before the client id became a secret.)* Secrets issued since 2026-08-26
  have the shape `cfast_` and 48 characters, which secret scanners recognize.

## 5. Reading it

- **Health**, over WARP as the owner: `curl -s -A "$BROWSER_UA" https://yokohama.moore.nyc/api/health`. It reads
  `identity: "cf-access"`, `signal: "ws"`, and the archive switch.
- **The tail**: `npx wrangler tail yokohama-web --format json > "$SCRATCH/tail.json"`, left running
  beside the readings. Every event is one invocation, PRETTY-PRINTED across many lines: parse the
  file by whole JSON objects — `jq` reads them as a stream — never line by line (§11). The
  Worker's own invocations have `executionModel: "stateless"`; an object's have `"durableObject"`
  and its class as `entrypoint`. The request's headers arrive `REDACTED`, the assertion among them.
  Count and cost them with:

  ```sh
  jq -c '{model: .executionModel, entry: .entrypoint, url: .event.request.url, cpu: .cpuTime, wall: .wallTime, outcome}' \
    "$SCRATCH/tail.json" | sort | uniq -c
  ```

- **The shell's own log lines** — `trap`, `load-trap`, `poison`, `identity-failed`, `init-refused`,
  `frame-failed` and the rest (spec §2.8) — are the events' `logs`, one JSON object per line, and
  Workers Logs (observability is on) keeps them. Since M6.5c2 the layer's log lines reach this
  host's log through the log seam, under their own names: the module records each line the layer
  writes during a call, and the shell logs it as one of its own (`crates/yokohama-web/src/log.rs`
  names them — `archive-over-refused`, `archive-over-unstamped`, `archive-over-table-gone`,
  `archive-kept-earlier-completion` and `listing-skipped`). The one line the module drops is the
  `archive-over-refused` its own deferral of every game-over put would write, since the shell logs
  that save's upload itself. *(Corrected 2026-09-28, M6.5c2: this sentence said the layer's own
  `eprintln!` lines were mute on this host, which was true until the log seam.)* The house and the
  slot (spec §2.5; Task 12) add these:
  - **`slot-pushed`** `{id, generation, revision, removal, attempt}`: a table pushes its slot to the
    house — its scrubbed copy, or a REMOVAL when `removal` is true — logged before the RPC.
    `attempt` counts this slot's pushes that failed before it.
  - **`slot-acked`** `{id, generation, revision}`: the house acknowledged that push, whether it
    applied it or ignored it. On the happy path every `slot-pushed` has one.
  - **`slot-failed`** `{id, generation, revision, attempt, retry_in?, error}`: the push's RPC
    failed. With `retry_in` (milliseconds) the slot still holds it: the failure is counted and the
    alarm re-armed on the backoff — 30 s, 2 min, 10 min, then hourly. Without it, a newer slot has
    replaced it, with a push and a retry of its own, or the object has ended (a removal acknowledged
    meanwhile, then `deleteAll()`).
  - **`slot-ignored`** `{id, generation, revision, current}`, logged by the HOUSE: a push it did not
    apply, and acknowledged all the same. The reasons: a revision no higher than the current one's;
    a retired generation; another generation while one is current; or, with no current row
    (`current: null`), a generation that does not order after every retired one. A generation's
    time is the shell's clock in MILLISECONDS when the table's first batch drew it (the
    orchestrator's F-2), so that last case is a table created, forced and re-created within the
    same MILLISECOND as the original's create — the pool reads 5 to 12 ms between the two. A
    delete's marker refuses any create at its id for 90 days (409), so only a force leaves the way
    open. A table created by an earlier deployment keeps the generation it drew then, whose time is
    a reading in SECONDS, and every generation drawn since orders after it.
  - **`copy-refused`** `{id, error}`: `house_copy` refused the table's document, and `error` is the
    module's sentence, verbatim. No slot is written, so the house keeps listing the last copy it
    received. Only a corrupted store reaches it; the operator's force (Task 14) is its path.
  - **`trap`** with `call: "house_copy"` `{id, call, error}`: computing the copy trapped, or its
    module could not be built (`error` keeps the error's class). The request's documents commit and
    its reply stands. The copy is owed: `meta.copy_owed` counts the traps in a row, the alarm
    retries on the backoff, and every request computes the copy until it is made.
  - **`house-chat-dropped`** `{id: "house", traps, error}`: the house chat failed to load twice in
    a row, so its row was deleted and the lobby serves without it. A first failure logs `load-trap`
    with `kind: "house"`.
  - **`alarm-failed`** `{id, error}`: a `setAlarm` or `deleteAlarm` issued beside a batch was
    rejected.

  The request path and the wake (the whole-branch review's fix wave) add these:
  - **`object-failed`** `{target, path, error}`, logged by the WORKER: an object's `fetch` rejected
    — a throw that is not a trap, such as a reply the shell cannot read, a batch that did not
    commit, or an object reset under `blockConcurrencyWhile` — and the request answered 500 `the
    server could not answer — try again`. `target` is the route's kind — `table`, `create`, `house`
    or `archive` — and `path` the request's. A trap never logs it: each object logs its own `trap`
    and answers its own 500.
  - **`wake`** `{id, wakes}`: with `YOKOHAMA_WEB_COUNT_WAKES` on, a NEW module was loaded at a table
    that holds a table document, and its request's batch committed the count; `wakes` is the count
    after it. This is how `meta.wakes` is read off a live Worker (the orchestrator's ruling I-84):
    the object was evicted — hibernation, a deploy, a restart — and woke. With the switch off, no
    line.
  - **`picker-dropped`** `{id, stored, running}`: a new module found the table's `picker` row built
    under code other than its own — `stored` is the row's code, `null` for a row written before
    rows carried one (every row the fourteenth to the eighteenth deployments wrote), and `running`
    this build's, a digest of the engine's, the CLI's and the layer's sources — and cleared it
    without re-feeding a line (the orchestrator's rulings I-85 and I-89): under other code a line
    may mean something else, and no code vouches for an unstamped row. It is logged once the
    request's batch that deletes the row has committed, as `wake` is: a request whose own call
    traps commits nothing, so the row stays, and the next request's wake drops it and logs it
    (R1's N-2). A picker open across a deploy that changed that code starts over, and the player's
    next click is a 409. *(Measured at the twenty-fourth deployment, §18: the 409 is the answer to the
    click a browser sends, which carries `expect`; a bare `{"line"}` without it falls to the prompt the
    table now asks and is answered by that prompt — `Unrecognized input`, 200.)* So after the first deployment past the eighteenth, every picker open at it
    is dropped at its table's next request, logged with `stored: null`.
  - **`restore-refused`** `{id, status, body}`: the wake's `restore_picker` met the layer's own
    refusal — a stored record that does not parse, or does not replay — and the request went on to
    its own dispatch, whose answer stands. The picker row is kept.
  - **`frame-carried-bumps`** `{id, bumps, archive}`: a framing read, which only reads, carried
    bumps or an archive plan; this pass neither frames nor owes them. No known case makes one.
  - **`alarm-without-name`**: the platform delivered an alarm with no name, and it touched nothing.
    Only an object reached by no name gets one, and such an object writes nothing and arms nothing.

  The archive on R2 (spec §2.6; Task 13) adds these:
  - **`archive-landed`**, **`archive-identical`**, **`archive-conflict`**, **`archive-failed`**
    `{id, site, record_key, detail, attempt?}`: one upload's outcome. `site` is `abandon` (a
    creator's DELETE), `expiry` (an idle table's expiry), `operator` (an operator's plain expire),
    `game-over` (a save a game over owes, uploaded after the move's reply) or `browser`
    (`POST /api/archive`, where `id` is `archive:<12 hex>` — the first twelve hex of SHA-256 over the
    caller's address — never the object's name, `archive:<address>`).
    `detail` is the record's key, or for `failed` R2's message — or the host's own sentence
    `<record_key> refused a create as present, then read as absent`, when the record's
    create-if-absent put was refused and the record then read as absent. `attempt`, at the
    game-over site, counts this save's earlier tries that did not complete it, and at the expiry
    site the table's earlier expiries that did not end it. A failed
    game-over save is retried on the backoff — 30 s, 2 min, 10 min, then hourly — by the alarm, or
    by the table's first request once the rung has passed: a request inside the rung uploads
    nothing, so an R2 outage costs a request a read of the owed row and no attempt. *(Corrected
    2026-09-29, M6.5c2 cycle D, R-D6: until then every request re-attempted it, and each failure
    climbed the ladder.)* **`record_key` and `detail` are logged through `redactKey` since security
    S2b** (finding D23), as is every `record_key` below — `archive-stamped`'s, `archive-owed-kept`'s,
    `archive-owed-dropped`'s and `held`'s: a browser save's `browser/<readable>-<12 hex>/` segment
    reads `browser/<12 hex>/`, so a line names the caller's folder and never the address it spells;
    a server game's key carries no such segment and is logged as it is. *(Corrected 2026-10-12: until
    S2b `id` was `archive:<address>` and both fields carried the readable segment.)*
  - **`archive-stamped`** `{id, record_key, reading, stamped, over}`: a game-over save that landed
    (or was already there), stamped by the layer's own `stamp_archive` at the plan's recorded
    `reading`. `stamped: true` completes the save; `stamped: false, over: false` — an undo at a
    one-person table while the upload ran — means nothing is owed any more; `stamped: false,
    over: true` is retried. When the stamp answers other than 200, `stamp-refused` is logged in
    its place, below. *(Corrected 2026-09-28, M6.5c2: until then this line
    carried `{status, body}` in place of the two booleans; since M6.5c2 `archive-stamped` always has
    the five fields above.)* **Since M6.5c2 a `conflict` goes to the stamp as well** (R-6.5c2.11): the
    upload hands the record it met at the key to the module, and the layer stamps the table when that
    record is an earlier completion of this same game (`stamped: true`; the archive keeps the first
    ending, and the layer logs `archive-kept-earlier-completion`). Every other conflict is final,
    whatever the stamp answers, and is never retried. *(Added 2026-09-28.)*
  - **`stamp-refused`** `{id, status}`: a game-over save's stamp answered other than 200. After a
    `landed` or `identical` upload the save stays owed and is retried on the backoff; after a
    `conflict` it is final, as every conflict the layer did not stamp is. The line names no body,
    and the refusal is never thrown. The one refusal a save owed across an undo can meet is the
    foreign-bot rule's 409, for a game not over against a bot this build does not run, and its
    reason is on the wake's `restore-refused` line, whose `body` carries the layer's answer: a
    deploy makes every wake fresh. *(Added 2026-09-28, M6.5c2.)*
  - **`archive-owed-kept`** `{id, record_key, attempt}`: the alarm ran while the switch reads
    `off` and found a save still owed. The save is kept, and nothing re-arms the alarm for it; once
    the switch reads `r2` again, the table's next request or alarm resumes it.
  - **`archive-owed-dropped`** `{id, record_key, attempt}`: a delete made while the switch reads
    `off` — an abandon, or an expiry — took with it a save the table still owed, logged after the
    delete's batch committed; and an operator's force, which saves nothing whatever the switch
    reads, logs every owed save it drops, once it is gone and before its `forced`.
  - **`trap`** with `call` naming the stage that trapped: in a delete's first dispatch — an
    abandon's, an expiry's — `house_copy` (the delete reading the table's copy: its stamp, and at
    an expiry its creator),
    `events` (the reading that decides the marker's tail), `dispatch` or `expire`, or `read`; in its
    second dispatch, `receipt`, `dispatch` or `expire`, or `read`; in a game-over save's stamp,
    `receipt`, `stamp_archive` or `read`; in an archive object, `share_prefix` or `set_share` (the
    share's count, security S2b), `receipt` or `dispatch`.

  Expiry, the marker and the operator (spec §2.4, §2.9; Task 14) add these:
  - **`expired`** `{id, reason, kept}`: the layer deleted a table as its creator — its expiry
    (`reason: "expired"`), or an operator's plain expire (`"operator"`) — and its marker says `kept`:
    `archive`, `nothing` or `off`. A creator's own Abandon logs no line of its own.
  - **`expiry-failed`** `{id, status, body, attempt, retry_in}`: an idle table's expiry did not end
    it — its save failed, the engine trapped, or its wake could not load the table — and `body` is
    the answer it met. The table stays, and the alarm retries it `retry_in` milliseconds later, on
    the backoff. `attempt` counts the expiries before this one that did not end it. **A line that
    repeats every hour with the same `body` is read by that `body`.** One naming a stored
    DOCUMENT — `… table document does not parse`, `… table document is malformed`, `… record does
    not parse` — names a table no retry will end, and only an operator's force ends it. The
    archive's own refusal, `the archive could not be written (…) — the table was not deleted`,
    with R2's reason inside the parentheses, repeats for as long as R2 fails, a long outage
    included: **wait for the archive**. Once R2 answers, the next retry saves the game and the
    table expires. A force there DROPS the game the expiry was saving.
  - **`held`** `{id, record_key}`: an expiry's save met another game's record already at its key —
    a `conflict`, which no retry can clear. The table stays, and is never expired again; an
    operator's force is its path.
  - **`expiry-blocked`** `{id, why}`: an alarm found a table past its TTL that will not expire on
    its own — `why` is `held`, or `poisoned` — and armed nothing for it.
  - **`marker-ended`** `{id, reason, at, kept}`: a marker's 90 days were over and the lobby had
    acknowledged its removal, so the object ended (`deleteAll()`).
  - **`forced`** `{id}`: an operator's force ended the object (`deleteAll()`), once the lobby
    acknowledged its removal — or at once, when the lobby held nothing of the table.
    *(Corrected 2026-09-28, M6.5c2: since M6.5c2 a force flags its own removal, `meta.forced`, in
    its batch, so `forced` is exactly an operator's force and follows its `operator` line.)*
  - **`removal-ended`** `{id}`: a REMOVAL with no marker and no force's flag ended the object once
    the lobby acknowledged it: one the code before the marker wrote (the fifteenth and sixteenth
    deployments), or one a force made before M6.5c2 flagged its own. No code since leaves one.
    *(Added 2026-09-28, M6.5c2.)*
  - **`operator`** `{id?, action, operator}`: every operator action — `list`, logged by the house
    with no `id`, `expire` and `force`, logged by the table — with `operator` the first twelve hex of
    SHA-256 over the operator's address (`callerDigest`), never the address (security S2b, finding
    D23). *(Corrected 2026-10-12: until S2b the field was `address`, the address itself.)*

  Security S2b (the Worker's door) adds these:
  - **`limiter-failed`** `{error}`, logged by the WORKER: a Rate Limiting binding's `limit` threw,
    and the request went on unlimited — the limit bounds cost, never access (AD-6). `error` is the
    platform's own text, which may carry the limit key — an IP under `open` — if it echoes it
    (unverified offline; a residual owed to S2b's after review).
  - **`archive-count-failed`** `{id, error}`: the Archive object's R2 listing of the caller's share
    failed. No listing reaches the module, so the save is refused uncounted: the layer's 500 with
    `the caller's share was not counted before this save` inside it, which the outbox retries.
    `error` is R2's own text through `redactKey` (since the S2 stack review), so a listing error
    that spells the share's prefix names its folder by the digest alone.
  - an Archive object's **`init-refused`** `{id, error}` (one of the shell's own lines, above): `init`
    refused the object's caller — a claim the key grammar refuses, such as `..` — and the request is
    the layer's 401. `error` is the grammar's sentence with the refused claim named by its digest
    (S2b's after review, `4814c999`; the echo's cut and escaped forms since security S2c,
    `291b9e35`), and a configuration refusal's value is kept as it is. A well-formed address never
    meets this line, since `init` admits one.
- **The bucket**, through wrangler's own login: `npx wrangler r2 object get
  "yokohama-archive/<key>" --remote --file <path>`. A pair is `rev7/{over,abandoned}/<id>.json` and
  `.table.json`, and a browser's save sits under `browser/<caller_segment>/`. A game whose table's
  creator, or a browser save whose sender, has an address ending `.test` is keyed under the root
  `test/` instead, apart from the owner's paths (spec §2.6).
- **The Origin check**, over WARP: an upgrade from the site's own origin answers 101, one from
  another answers 403 with the host's sentence (reading 2d).

  ```sh
  WS=(-H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==')
  curl --http1.1 -s -A "$BROWSER_UA" -i -N --max-time 5 "${WS[@]}" -H 'Origin: https://yokohama.moore.nyc' https://yokohama.moore.nyc/api/stream | head -1
  curl --http1.1 -s -A "$BROWSER_UA" -i --max-time 5 "${WS[@]}" -H 'Origin: https://evil.example' https://yokohama.moore.nyc/api/stream | sed -n '1p;$p'
  ```

- **`node scripts/read-deploy.mjs`** reads a live deployment: `assets` checks every precache entry
  of the DEPLOYED `/sw.js` (200, the type's essence, the SHA-256), `/index.html` answering 200, and a
  missing file answering 404 with an empty body; `install` loads the site in headless Chromium and
  reads the service worker's install; `console` records one fresh load and one controlled load, with
  every console message and every failed or cross-origin request.

## 6. Rollback

- **At the fourteenth deployment there is no earlier version.** The fallback is the NAS, which is
  untouched. To take the Worker off its hostname, remove the custom domain in the dashboard
  (Workers & Pages → `yokohama-web` → Domains): the objects and their storage stay. **Never
  `wrangler delete`**, which deletes the Worker "and all associated Cloudflare developer platform
  resources" (the command's own reference): the objects ARE the store.
- **Later**, `npx wrangler rollback <version-id>` (the ids from `npx wrangler deployments list`)
  goes back to an earlier version, but never across a Durable Object migration: a version from
  before a class was added cannot be restored (Cloudflare, *Rollbacks — Bindings*). Nor can a
  version bound to an R2 bucket that no longer exists (the same section), and every version from
  the sixteenth deployment on binds `yokohama-archive`.
- **A rollback to the eighteenth deployment or earlier is a GENERATION-ORDER rollback** (the
  orchestrator's ruling I-89), the shape of `deploy/RUNBOOK.md` §5's schema-rollback bullets. From the deployment after
  the eighteenth on, a table draws its generation's time from the shell's clock in MILLISECONDS
  (the orchestrator's F-2), where the eighteenth and earlier draw the module's reading in SECONDS.
  A seconds generation orders BEFORE every milliseconds one, and the house applies a table's first
  copy only when its generation orders after every generation it has retired for that id. So a
  table forced under the newer code — or deleted, its marker's 90 days over — and created again
  under the rolled-back code has its every copy ignored (`slot-ignored`, `current: null`): the
  lobby does not list it until the retired generation is pruned — which the house does only inside
  a copy it APPLIES, for any table, once that generation is 90 days retired — and the table itself
  plays by its link. Ids never used, and tables the house still lists, are unaffected: a table
  keeps the generation it first drew. Rolling forward again needs nothing, since every
  milliseconds generation orders after every seconds one.
- **The same rollback drops every open picker the newer code stored.** Its rows carry a `code`
  key, which the older module's strict read of the row does not know, so it reads the row as no
  list and clears it, re-feeding nothing: the player's next click is a 409. Nothing is applied
  and nothing else is lost.
- **A rollback to a version from before M6.5c2 cycle D (the twenty-third deployment, `36038818`,
  or earlier) stops the chat wherever a message has been deleted.** Cycle D deletes a message as a
  TOMBSTONE — `"text": ""` and `"deleted": true`, its number, author and clock kept — and gives
  every chat document a revision, `rev`. The older module ignores both keys and then refuses the
  whole document on its shape rule, where an empty text is malformed; it never shows the tombstone
  as an empty message. So a `Table` object whose chat holds a tombstone answers every chat read and
  post 500 with the layer's `"<id>"'s chat document is malformed: message N's text is empty or
  carries a control character`, and the `House` object does the same for the lobby's chat; the
  game, its events and the lobby's listing answer. It does not heal on the older version — the trim
  that would drop the tombstone runs only on an append, and the append reads the document first —
  so the chat stays down until the roll forward, which reads the document whole. A chat with no
  deleted message is unaffected, except that the older module's append rewrites it without `rev`,
  which reads as 0 after the roll forward. The answer was measured on the layer at `014433df`
  (`deploy/RUNBOOK.md` §5); the module runs that layer, and no Worker version was rolled back to
  see it. *(Added 2026-09-29, M6.5c2 cycle D.)*

*(Added 2026-09-30, cycle E — §19.)*
- **A rollback across cycle E is a MARKER rollback too:** the twenty-fifth deployment (E) writes the `waited` marker reason (Task 6) and `TurnMirror.has_events` (Task 5). A version before it (`974e0a93…`, D, or earlier) renders a `waited` marker's head as `undefined — …` for as long as the marker lives (up to 90 days) and ignores `has_events` (a table's abandon then asks as before). The chat-tombstone caveat of cycle D stands beside it.
- **Across security S1c the archive's rename is ONE-WAY.** The window moved every `browser/<legacy>/`
  folder under its digested segment (the rename section above) and nothing in the S1c build reads a
  legacy folder, while a build from before S1c reads and files under the legacy segment alone. A
  rollback across S1c therefore loses no file but SPLITS an address's saves: new saves land under the
  legacy folder again, beside the renamed one, until the roll forward's window runs the recipe once
  more — step 2's merge branch (and step 3's new-key-first read) is written for exactly that second
  run. *(Added 2026-10-11, security S1c.)*

## 7. The live tiers and the quota guard

Two tiers read the DEPLOYED Worker, and neither is a gate: CI runs neither, and each plays on the
owner's account, spends its quota, and leaves rows and saves behind. They are the operator's to run —
the orchestrator's this cycle (Task 15) — from the owner's workstation, over WARP, with the
credential in the repository root's `.dev.vars` (§4), whose values neither prints.

**A live run opens and closes with a deploy since security S2b** (finding B6; decision D-1): the
production file ships the dev door `off`, and both tiers play through it, so a run begins with
`npm run deploy:dev` — `wrangler.dev.jsonc`, production with the dev door `on` — and ends with the
plain `npm run deploy`, which shuts it again. Both deploy the release build (§1), and each restarts
every object, so run them between games. Every line runs in a subshell from the repository root, the two
tiers' own lines below written the same way, so the block pastes whole: each line starts at the root
whatever the one before it did, and the closing deploy runs and shuts the dev door even when a tier fails.
Each comment stands on a line of its own: an interactive zsh reads `#` as a comment only with
`interactivecomments` set, which is off by default, and there a trailing `#` is a parse error that runs
nothing, while a comment line prints `command not found: #` and the next line runs (measured with stub
commands). The burst below stays outside this block: it reads `$BROWSER_UA`, which §3's operator
playbook sets, and spends the owner's write budget (the bullet says when to run it):

```sh
# before: the dev door on
(cd worker && npm run deploy:dev)
# the live Playwright tier, below
(cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link)
# the live replay, below …
(cd worker && node scripts/live-replay.mjs)
# … and again, once the first has finished
(cd worker && node scripts/live-replay.mjs)
# after: the dev door off again
(cd worker && npm run deploy)
```

The dev door is not exempt from the rate limits (decision D-3), and neither tier comes near them: the
local two-browser walk's busiest page peaks at 71 unsafe requests in 10 s, under `WRITE_LIMIT`'s 120
(`docs/web-lane.md` §16), and the live replay is not paced — since security S1b it runs 26 of the
corpus's 2,213 rows at the production door, in about 4 s (§21; decision D-8, withdrawn).

- **The live Playwright tier** — two specs, `two_browsers` and `joins_by_link`, played at
  `https://yokohama.moore.nyc` (`web/e2e/live.ts` holds its rules):

  ```sh
  (cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link)
  ```

  One Playwright run at a time. **How a live context is somebody** (the orchestrator's rulings I-75
  and I-76): Node's own `fetch` presents the credential ONCE, to `/api/health`, and Access answers
  with a `CF_Authorization` cookie, which seeds each context's storage — the browser keeps it to
  this origin, and the credential itself never reaches Playwright or another host. The claimed
  address goes context-wide as `X-Yokohama-Claimed-User`, because Playwright's request routing does
  not reach a page's own WebSocket upgrade and the socket must carry an identity; that header is no
  secret. **Tracing is OFF** under the live target, since a trace keeps a page's cookies: a failing
  live run keeps its screenshots and no trace. **Every live `request.*` call goes through
  `liveRequest`**, which rethrows an error's first line alone, because Playwright's "Call log"
  lists the context's cookie; and a live response never gets `expect(res).toBeOK()`, whose failure
  prints the same log — `expect(res.ok())` is the form. A failure whose message still holds
  `Call log:` is an exposed session: paste it nowhere, and delete `web/test-results/` after a
  failed live run.
- **What the live walk forgives, and nothing else** (`web/e2e/harness.ts`'s `watchForBreakage`):
  - live only, the Web Analytics beacon's CORS error: the claimed header is a custom header on the
    cross-origin beacon request, which forces a preflight the beacon's host does not answer. One
    blocked load logs TWO console lines — the CORS sentence at the page's URL, and `Failed to load
    resource: net::ERR_FAILED` at the beacon's (I-76) — and both are forgiven, each tied to the
    beacon's URL, and counted;
  - live only, since cycle F, the release feed's failure while the repository is private: GitHub's
    404 a real browser meets, or the CORS refusal the claimed header causes at GitHub's preflight —
    nothing else from the feed, and nothing at any other address — counted apart from the beacon's;
  - since security S2c (R-6) the walk forgives NO CSP line on any tier: the enforced policy carries
    the fixed nonce the edge copies onto its detection script, so a CSP line is a finding (the
    fallback, D-15, restores a forgiveness at `error` in the line's measured words). The walk reads
    every CSP line at ANY console type, live and locally, where there is no edge and no injected
    script;
  - in both modes, the deal's DESIGNED 409 on `GET …/api/games/<id>/deal` — a table that flipped to
    playing between two of the browser's requests, the refusal the dealing screen branches on — in
    both of its arms, the failed response and its console line (I-81), counted as `dealAnswers`.

  Every other status of 400 or more, and every other console error, is breakage.
- **The live corpus replay** (`worker/scripts/live-replay.mjs`, spec §5.3's live half): the walk the
  committed corpus was cut from, sent to the Worker as the walk's own `.test` identities, every
  status and every named body compared with `web/src/wire/`:

  ```sh
  # run it twice, the second once the first has finished
  (cd worker && node scripts/live-replay.mjs)
  ```

  **It walks the production door** (security S1b; R-SEC8(b)). A deployed Worker honours a
  creator's seed only at a table whose every human seat is the creator's, and the walk creates
  `wire` and `wire2` seeded beside an open seat. So the replay SENDS those two creates and holds
  each to the production rule's 400 in the layer's own sentence (`SEED_NOT_SOLO`), counted among
  the exemptions spent as `productionSeedRule <id>` — a 201 there is a mismatch, a diagnostic
  configuration deployed by mistake — and it SKIPS every other row at those two tables and the
  four lobby pages, whose committed bodies name them, counted and printed as "rows skipped under
  the production seed rule". The rest runs and is held to its committed answer: the host's own
  reads, the house chat's post, read and delete, the solo `wire4` table, the 404 and the archive
  walk. The whole walk is read elsewhere: by the pool, which replays every row with
  `YOKOHAMA_DIAGNOSTIC_SEEDS` on (§3; never set in production), and by the two-browser Playwright
  tier, which plays an unseeded two-person table to a winner on the live host.

  **Twice, one after the other**: the second run is the repeatability evidence, its two first sends
  answering the once-rule's 200. **Never beside the Playwright tier**: the two share the house
  chat's `seq`. It forces its three tables before and after itself with no credential, so over WARP
  the owner's own login is the operator (§3).

  **Since 0.6.3 the two house rows' watcher tag is a production value** (security S2c, D8): a
  deployed Worker salts the house chat from OS entropy, so the judge takes `house_chat_post`'s and
  `house_chat`'s watcher `author.tag` back to the committed one (`houseWatcherTag`, counted in the
  exemptions spent) only when it is four lowercase hex and the page's is the post reply's; a
  dropped, `null` or malformed tag, or a page that disagrees with the reply, fails the row.
- **The rate limit's live reading — the burst** (security S2b, finding C2; AD-7), over WARP as the
  owner, with the smoke's browser `User-Agent` (the next bullet):

  ```sh
  # BROWSER_UA: set once, in §3's operator playbook (the curl playbook).
  B=https://yokohama.moore.nyc
  seq 300 | xargs -P 16 -I@ curl -s -o /dev/null -w '%{http_code}\n' -X POST -H 'content-type: application/json' -A "$BROWSER_UA" "$B/api/chat" -d '{}' | sort | uniq -c
  curl -s -i -X POST -H 'content-type: application/json' -A "$BROWSER_UA" "$B/api/chat" -d '{}' | grep -i -e '^HTTP' -e '^retry-after' -e '"error"'
  ```

  Sixteen posts at a time, because the limit is a RATE: 120 posts must land inside one 10-second
  window, and a sequential loop, a new TLS connection a post, ran at about ten a second on a fast
  host at the S2 stack review, under the twelve a second the limit needs. (`-I@` and not `-I{}`:
  xargs rewrites every `{}` in the line, the body included.) It reads 400s and 429s: the limiter
  counts before the layer's 400, `{}` posts nothing, and once 120 posts are inside the window the
  rest are refused. The last line, sent from the same shell at once after the burst, inside the same
  window, shows one refusal whole — `429`, `Retry-After: 10` and `too many requests from you just
  now — wait a moment and try again`. Counting is per Cloudflare location and best effort, so a
  burst may need a second run to read its 429. `scripts/probe-burst.mjs` sends GETs alone and can
  read no 429. The burst spends the owner's own write budget, so for ten seconds the owner's writes
  are refused: run it between games.
- **Bot Fight Mode's 403.** The zone's Bot Fight Mode and JavaScript detections — ON, by the owner's
  decision — answer a bot-shaped request BEFORE Access sees it: 403, `cf-mitigated: challenge`, and
  a `text/html` "Just a moment…" body of about 5,259 B (§8, reading 1). Both tiers print a body
  that is not the API's JSON error by its type and size, never its text, so a 403 that reads as a
  `text/html` body of about 5,259 B is the zone's, and the cure is to run over WARP. **Every `curl` at
  either host OVER WARP carries a browser `User-Agent`** — the smoke's `BROWSER_UA`
  (`scripts/smoke-release.mjs`) — or meets Bot Fight Mode's 403 before Access sees it (the owner,
  2026-10-11: "when using curl to cloudflare, you need to include user agent details so it looks like
  a normal browser when a site is access protected or cloudflare will fight it"). Off WARP a bare client
  meets the 403 whatever its `User-Agent`: a browser `User-Agent`, the navigate headers and a fake
  Googlebot change nothing there, the TLS fingerprint being the tell (W-1, §20). **The smoke's live
  mode (§3, the `YOKOHAMA_TURNSTILE` bullet) is read the same way:** run it over WARP, or from any client Bot
  Fight Mode trusts, because off WARP its sitemap control reads Bot Fight Mode's 403, which it cannot tell
  from the edge rule's (cycle F's window, W-1; §20).

### The exit's window: counting wakes

The exit's headed game (`web/e2e/exit_game.spec.ts`, run as its header says) reads each wake off
`wrangler tail` as the `wake {id, wakes}` line (§5), which the objects log only while
`YOKOHAMA_WEB_COUNT_WAKES` reads `on` (the orchestrator's ruling I-84). `wrangler.jsonc` leaves it
unset (§3), so the window is two deploys, run in `worker/`, with no file edited:

```sh
# opens it:
node scripts/deploy-guard.mjs --config wrangler.jsonc && node scripts/secrets-guard.mjs --config wrangler.jsonc && npx wrangler deploy --config wrangler.jsonc --var YOKOHAMA_WEB_COUNT_WAKES:on
# closes it:
npm run deploy
```

- **Opening it is a deploy**, guarded like any other: the guard and the secrets guard run first, as
  `npm run deploy` runs them, each told the file by `--config` as wrangler is (§2), and `--var`
  deploys one more plain-text variable beside `wrangler.jsonc`'s `vars`, replacing none of them (read in wrangler 4.136.3's source). Like every deploy, it restarts every
  object and closes every socket (§2): open the window between games, before the exit's run.
- **Closing it is the exit's last step.** The plain deploy replaces the Worker's variables with
  the file's again, which name no `YOKOHAMA_WEB_COUNT_WAKES`, so the switch reads off. While the
  window is open, every table's wake writes `meta.wakes` in its request's batch and logs its line.
- A SECRET would switch it on as well, but a secret survives every later deploy until it is
  deleted; the `--var` deploy is the form the ordinary deploy closes.

### The quota guard

Workers Free counts requests per UTC day, and Durable Object rows read and written besides. The
owner's line for this cycle is **80,000 per UTC day, account-wide, for EACH of the two guarded
figures** — the Worker requests and the Durable Object rows written (the owner's ruling of
2026-09-23; the orchestrator's I-82 applies it): read the day's figures before a live run and again after it, and HOLD a
run that could cross the line until the next UTC day. The GraphQL Analytics API gives them — `POST
https://api.cloudflare.com/client/v4/graphql` with a token that can read the account's analytics,
or the Cloudflare MCP's `execute` — ONE dataset per call, since one error fails the whole call (the
MCP's `execute` throws at the first):

```graphql
query ($account: string!, $date: Date!) {
  viewer { accounts(filter: { accountTag: $account }) {
    workersInvocationsAdaptive(limit: 1000, filter: { date: $date }) {
      sum { requests } dimensions { scriptName }
    }
  } }
}
```

```graphql
query ($account: string!, $date: Date!) {
  viewer { accounts(filter: { accountTag: $account }) {
    durableObjectsPeriodicGroups(limit: 1000, filter: { date: $date }) { sum { rowsWritten rowsRead } }
  } }
}
```

`$date` is the UTC day, `YYYY-MM-DD` — the `filter: { date: "YYYY-MM-DD" }` form this cycle's
readings used, one dataset per call; the requests are summed over every script, since the line is
the account's; the figures lag by some minutes. **What a run costs**, read this way on
2026-09-24 (the ledger's quota readings):

- one `two_browsers` game — 2,862 moves, with `joins_by_link` and the cleanups — cost about
  **38,350 Worker requests** (about 13.4 a move: three live pages each refetching on every frame,
  and the harness's own probes), about **7,000 rows written** (2.5 a move) and about **1.34 million
  rows read** (470 a move). The game's length varies about twice over between runs, so a second one
  on a day that has already spent some 44,000 requests could cross the line;
- `joins_by_link` alone, about 100 requests; the replay, about 2,300 a run.

A past run that had no tail reads through the Workers Observability query API, `POST
/accounts/<account id>/workers/observability/telemetry/query` over the dataset `cloudflare-workers`,
grouped by `$workers.outcome` and `$metadata.type`, or `view: "events"` with filters. *(Read at the
twenty-fourth deployment, §18, and load-bearing for any count taken this way: an events-view result's
`count` is bounded by the request's `limit` — a TOTAL is a `view: "calculations"` query with a `count`
calculation, `chartType: "aggregate"`; a shell log line — `picker-dropped`, `archive-landed` and the
rest — is found by the full-text `needle` (`parameters.needle: {value: "picker-dropped"}`), its
fields under the event's `source` and its `$metadata.type` `cf-worker`, where the request rows are
`cf-worker-event` and carry `$workers.cpuTimeMs`/`wallTimeMs`; a filter on `source.event` or on
`$metadata.message includes` reads 0 over a real line.)*

### What a live run leaves on the account

- **Two 90-day MARKERS per Playwright run**, at fresh `sim-` ids: each spec abandons its table in
  its `afterEach`, and the marker answers every request there for 90 days, then ends on its alarm.
- **Saves under the bucket's `test/` root**: `test/rev7/over/` and `test/rev7/abandoned/` pairs for
  the `.test` identities' finished and abandoned games. The lifecycle rule
  `expire-test-after-30-days` expires them 30 days after they were written (R-6.5c2.12; set at the
  M6.5c2 close, §15). *(Corrected 2026-09-28: this said the choice was the owner's, carried to the
  cleanup cycle; then that the rule was to be set at the close.)*
- **The replay's own saves**: its two first sends' pairs,
  `test/browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.json` and
  `test/browser/red_at_wire.test-c91ad2a917f7/rev7/abandoned/wire.json`, each with its `.table.json`
  — kept for good, by the owner's direction — and the walk's own game-over pair under
  `test/rev7/over/`. *(Since security S1c the segment carries a digest of the address (finding D26);
  before S1c's deploy window these keys sat under `test/browser/red_at_wire.test/`, which the
  window DELETES rather than moves — the stack's version bump makes them the reset's case (step 3 of
  the rename, above) — so the first replay after S1c's deploy re-creates them under the digested
  key, and "kept for good" holds of the keys.)*
- **A PERMANENT house-chat post per replay run**: Red (`red@wire.test`), "anyone up for a second
  table?", which real people see in the lobby until 500 newer messages trim it. *(Corrected 2026-09-29,
  M6.5c2 cycle D: since cycle D the walk's `house_chat_delete` row DELETES that post in the same run —
  the replay re-targets `/api/chat/1` to the live document's own number — so what a run leaves is a
  TOMBSTONE, drawn as "message removed" in the lobby, and the house chat document's `rev` moves by
  one. A rollback to a version from before cycle D then meets the tombstone (§6's last bullet): the
  lobby's chat answers 500 until the roll forward. As of the twenty-fourth deployment the house chat
  holds two, from the close's two replay runs.)*
- While a Playwright run lasts, the live lobby lists its tables, each with a long name and a
  password.

### The replay's reset

Every run after the first meets the saves its two first sends left, and reads the once-rule's 200.
A `wire` record whose bytes have changed since — a version bump, a walk that applies other moves, a
record format change — meets them as a **409** instead. The replay then stops, names the cause, and
prints the reset (`resetRecipe`, R3's M-1), to run in `worker/` with wrangler's own login, since the
owner's R2 key is read-only (spec §7) — and then the replay again:

```sh
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.table.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test-c91ad2a917f7/rev7/abandoned/wire.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test-c91ad2a917f7/rev7/abandoned/wire.table.json" --remote
```

The keys are the committed replies' `path` under the `test/` root, and the replay prints them from
the corpus it runs, so its printed lines are the authority if the two ever disagree (security S1c
gave the segment its digest, finding D26).

The walk's own game-over pair under `test/rev7/over/` blocks nothing, since no payload reads it; a
later game over at that id with other bytes meets it and logs `archive-conflict`.

*(Added 2026-09-30, cycle E — the window's readings, §19.)*
- **The GraphQL forms:** the `date` form and the `datetime_geq/datetime_leq` form return the SAME figures (read side by side 2026-09-30 12:37Z: 29,211 / 205,709 / 20,839). Through the Cloudflare MCP the response is `{success, status, result}` and the figures are under `result.viewer` — a reader that looks for `data.viewer` reads an EMPTY list with no error, which is what an earlier session mistook for "the `date` form returns nothing today". **What does lag is the periodic rollup**: `durableObjectsPeriodicGroups` for a window minutes old reads empty; read a day's rows the day after, or a window hours old.
- **The Workers Observability query API** (`POST /accounts/{id}/workers/observability/telemetry/query`) is the per-window instrument the GraphQL rollup is not: every ad-hoc body needs a `queryId` (a 400 without one), `dry: true`, `view: "calculations"`, `chartType: "aggregate"`, a `timeframe {from, to}` in milliseconds and `parameters {datasets: ["cloudflare-workers"], filters, calculations: [{operator: "count"}], groupBys}`. The filters that split this host: `$metadata.service eq yokohama-web`; `$metadata.type eq cf-worker-event` (invocations, not log lines); `$workers.durableObjectId is_null` = the stateless Worker's requests, `exists` = the objects' invocations, `$workers.entrypoint` as a `groupBy` names the class (`Table`, `House`, `Archive`); `$workers.event.request.method` / `url` for the request's shape; a `needle {value}` finds a log line (`archive-landed`, `picker-dropped`, `object-failed`, `wake`). A calculation's aggregates are `[{groups, value}]`.
- **The rate-limit rule's language:** `ends_with(http.request.uri.path, "/join")` — a FUNCTION; the infix spelling `http.request.uri.path ends_with "/join"` is refused by the PUT. The zone's one rule after cycle E: `(http.request.uri.path eq "/") or (http.host eq "yokohama.moore.nyc" and (ends_with(http.request.uri.path, "/join") or ends_with(http.request.uri.path, "/unlock") or http.request.uri.path eq "/api/archive"))`, block, 20 per 10 s (the live replay's archive rows peak at 8; doubled and rounded to the Free plan's period). A run of the live replay under it HOLDS; a burst of 25 reads 20 × 403 then 5 × 429 (1015), clear again after the period.
- **The Bot Fight Mode reading from a bare client:** a `curl` without WARP meets a `text/html` 403 of about 5 KB on `/` and `/shell.bin` too, BEFORE Access — a negative reading of the door takes a real browser (the challenge runs there; `/sw.js` and the bundle are not challenged).

## 8. M6.5c's skeleton is live — the fourteenth deployment, 2026-09-23 (a record, not a plan)

The first deploy of the Worker (spec §3.2), from `756dc759` — Task 11's reviewed head, tree clean:
the Worker, the three classes in migration `v1`, the static assets, identity, the body bound and
the Origin check — and the spike's measurements (spec §0.3) re-read with the real code before
anything else lands on it. **Version `8b1bdcf8-610d-4b04-9e21-727e216cd61a`.** Every reading
matched the spike's in substance. One differs in form (reading 1, the zone's challenge page), and
the spike's one unexplained error (reading 6, the beacon) is explained; neither stops the cycle.

- **Before the deploy — the NAS's body bound, read live (2026-09-23, over WARP):** a
  2,097,153-byte `POST /api/games` answered **400**
  `{"error":"the body could not be read: length limit exceeded"}`, and exactly 2,097,152 bytes
  passed the bound (a 401, from identity). The Worker's `BODY_LIMIT_SENTENCE` is that text.
- **Build:** `web/`'s `npm run build` — wasm-opt 133, the browser module **1,175,106 B**,
  `grep -c debug_trap web/src/local/wasm/yokohama_wasm.js` **0**; `precache: 153 entries, 9,938,226
  B`, the cache `yokohama-release-10e4457c62975466`. `worker/`'s `npm run build` — the module
  **1,854,072 → 1,220,664 B** (wasm-opt 133); the assembly **153 entries verified**, 156 files,
  9,967,639 B; the release smoke **200**.
- **The deploy (`npm run deploy`):** the guard PASSED; **154 assets** uploaded in 2.65 s (the 156
  assembled files less `_headers` and `_redirects`, which configure the asset layer); Total Upload
  **1260.83 KiB / gzip 484.78 KiB**; **Worker Startup Time 2 ms**; the bindings `TABLE`, `HOUSE` and
  `ARCHIVER` (Durable Objects), `ASSETS`, and the eight variables of §3's table; the custom domain
  `yokohama.moore.nyc`.
- **Reading 1 — protection, from an unenrolled vantage** (a throwaway `alpine:3.22` container on the
  NAS, curl). **Nothing was served.** Two request shapes read differently, and both are recorded:
  - with curl's default User-Agent: `/` **403**, `/sw.js` **302** to the Access login
    (`bdvc.cloudflareaccess.com`), `/api/health` **403**, `/enter` **403**, `/index.html` **403**,
    `/assets/x.js` **302** to the login; a WebSocket upgrade with no credential **403**;
    `yokohama-web.bdvc.workers.dev` **404**. The 403s carry `cf-mitigated: challenge` and a
    5,259-byte "Just a moment…" body: the zone's Bot Fight Mode and JavaScript detections —
    kept ON by the owner's decision, 2026-09-23 — challenging a bot-shaped document request
    BEFORE Access sees it;
  - with a browser User-Agent and `Accept: text/html`: `/` and `/api/health` **302** to the Access
    login, whose `meta` reads `auth_status NONE`, `is_warp false`, `service_token_status false` —
    exactly the spike's row P;
  - the NAS host, from the same vantage, reads the same (`/` 403, `/api/health` 403, `/sw.js` 302),
    so the challenge page is the zone's behaviour and not this Worker's. Ruled a difference in
    form, recorded, not a finding that stops the cycle.
- **Reading 2 — identity on an upgrade** (`curl --http1.1` against `/api/stream`):
  - **2a**, the owner over WARP, no credential: **101**. It is the first run of the production key
    source, `keySource` over workerd's `fetch`, which no cell runs: it verified a real assertion;
  - **2b**, the credential with `X-Yokohama-Claimed-User: probe@deploy.test`: **101**;
  - **2c**, the credential alone: **401** `{"error":"no identity reached the server: this deployment
    expects X-Yokohama-User from the edge Worker"}` — the layer's own `worker`-mode sentence.

  All three as the spike (row 1).
- **Reading 2d — the Origin check, live**, over WARP, taken after the re-review of `756dc759`
  suggested it (§5 has the two commands):
  - an upgrade with `Origin: https://yokohama.moore.nyc`: **101** — a production request URL's
    origin is the site's own, so a page's socket is not refused;
  - an upgrade with `Origin: https://evil.example`: **403** `{"error":"this socket is for the site's
    own pages"}`, unupgraded;
  - a plain `GET /api/health` with that foreign `Origin`: **200** — a request that is not an
    upgrade is never checked.
- **Reading 3 — the static assets:**
  - `read-deploy.mjs assets`, over WARP: `/sw.js` 200, `cache-control: no-store`, **153** precache
    entries; **152 byte-exact** and **1 EDGE** — `/index.html`, into which the edge injected its
    `challenge-platform` script, and which is the release's bytes once that is cut away; `/index.html`
    **200**, never a redirect; `/assets/x.js` **404** with an empty body → **HOLDS**;
  - `read-deploy.mjs install`, the credential sent to the Worker's origin only: the service worker
    **activated**; its cache holds **154** entries for the 153 listed (the `/__release__` stamp is
    the 154th); `/index.html` cached at **2,387 B**, the release's bytes; the door reads "Ready to
    play offline." → **HOLDS**;
  - the tail (`wrangler tail --format json`, 1,605 lines): the Worker was invoked for `/api/me` ×6,
    `/api/stream` ×3, `/api/health` ×1 and `/assets/x.js` ×1 — the missing file, exactly ONCE — and
    ZERO times for any precache path, `/sw.js`, `/index.html`, `/art/*`, `/help/*` or the fonts.

  As the spike (row 7).
- **Reading 4 — cold start and CPU,** off the tail:
  - cold start, the deploy's "Worker Startup Time": **2 ms** (the spike: 1–3 ms);
  - the stateless Worker on `/api/*`, JWT verification included: **cpu 0–2 ms**, 1 ms typical, wall
    10–19 ms; the first `/api/stream` upgrade 2 ms cpu, 393 ms wall. R-6.5c.5 left this unmeasured:
    it is about 1 ms, far inside Free's 10 ms;
  - the objects: the house's first `/api/health` **7 ms** cpu, `/api/me` 0–2 ms, the first
    `/api/stream` upgrade **9 ms**;
  - outcomes: 19 `ok`, and 4 `canceled` — the upgrades curl's `--max-time` cut; `identity-failed`
    lines: **0**.
- **Reading 5 — health,** over WARP: `identity: "cf-access"`, `signal: "ws"`, `archive: "off"`,
  `multiplayer: true`, `semantics_rev` 7, `rules_hash` `d85445c479cb1ead`; `/api/me` names the
  owner's address, through the real assertion; `GET /` answers 200 at **3,325 B** — the edge's
  JavaScript-detection injection, as spike row 8 (2,387 B built).
- **Reading 6 — the Web Analytics beacon, diagnosed** (`read-deploy.mjs console`, two loads each):
  - over WARP, NO custom header anywhere — a real browser's shape: console messages **none**,
    failed requests **none**; the beacon script **200**; the RUM post to
    `https://yokohama.moore.nyc/cdn-cgi/rum` **204**;
  - with the credential sent to the Worker's origin only: the same, and **no error**;
  - **the diagnosis:** the spike's CORS error was an artifact of the spike's HARNESS, which sent the
    credential as CONTEXT-WIDE headers. Custom headers on the cross-origin beacon request force a
    CORS preflight the beacon's host does not answer. A real browser sends no custom header and
    sees no error, and neither does a harness that scopes the credential to this origin. The zone
    stays exactly as it is — Web Analytics, Bot Fight Mode and JavaScript detections ON — no
    allow-list is needed, and the live test tiers scope the credential per origin.
  - **Corrected forward** (the live tiers, §7; the orchestrator's ruling I-75): the live
    Playwright tier, built after this reading, sends the CLAIMED address context-wide, because a
    page's own WebSocket upgrade needs it and Playwright's request routing does not reach one. So
    the beacon's CORS error recurs there, as the same harness artifact, and that tier's walk
    forgives exactly its two console lines. "No allow-list is needed" holds for a real browser and
    for this reading script; the credential itself is kept to this origin by every tier.
- **Against the spike (spec §0.3):**
  - row 1, identity — the same;
  - row 2, startup and size — the same class: 2 ms, and 1,260.83 KiB against the spike's 1,172.84 KiB,
    because the Worker now carries the shell;
  - row 7, the static assets — the same: 152 exact plus `/index.html` edge-rewritten, no invocation
    for a real file, one for a missing file, the service worker installed whole;
  - row 8, the edge — the same 3,325 B injection, and the beacon's error explained above;
  - row P, protection — the same in substance, with the zone's challenge page the one new shape.

### What this deployment does NOT claim

- **The raw-address rule (§3; Task 2's note E1 and Task 11's I1) was not read live**: no Access
  identity with a non-ASCII address exists to try it with. The pool's cells are its evidence.
- **No game was played and no table was created** live. The lobby lists nothing until Task 12's
  copies; the archive is off until Task 13 binds the bucket; there is no expiry or operator surface
  until Task 14.
- **No phone and no browser off WARP.** Reading 1's unenrolled vantage was curl, not a browser.

### Rollback

§6: there is no earlier version; the NAS, untouched, is the fallback. The migration `v1` is FROZEN
from this deployment on: a class added later takes a new tag, and no version can be rolled back
across one.

## 9. The house and the slot are live — the fifteenth deployment, 2026-09-24 (a record, not a plan)

From `b24149b5`, Task 12's reviewed head, its fix round included, tree clean. It brings the house
over scrubbed copies, a durable ordered slot per table, the corpus replayed through the objects in
the pool, and `/api/art` projected onto the layer's nine maps. **Version
`e355c676-cfb2-400b-926d-05def2e725fd`.** Every reading holds. The readings ran with the credential
and claimed `.test` identities, the tail listening, and the header file deleted afterwards.

- **Build:** `worker/`'s `npm run build`. The module is **1,220,664 B**; the assembly **153 entries
  verified**, 156 files, 9,967,639 B; the release smoke **200**.
- **The deploy:** the guard PASSED its six checks, the comparison of the two configurations' object
  blocks among them. Total Upload **1288.14 KiB / gzip 493.52 KiB** (the fourteenth's: 1260.83
  KiB); **Worker Startup Time 2 ms**; the custom domain `yokohama.moore.nyc`.
- **Reading 1 — a locked create** as `probe@deploy.test` through the credential, its seed
  `9007199254740993` (above 2^53): **201** `sim-81e5fae99a122963`, status `open`, `locked: true`.
- **Reading 2 — the lobby, listed from the house's copies:** for the creator (`scope=mine`,
  `my_seats [0]`) and for `second@deploy.test` (`scope=open`, `my_seats []`). In both listings the
  seed's digits appear **0** times, runs of 32 or more hex characters **0** times, and the keys
  `salt`, `hash` and `seed` **0** times. What the lobby serves carries none of them, live. The
  scrub itself — the copy the house holds — is read in the pool.
- **Reading 3 — the lobby socket** as `watcher@deploy.test`, over `curl --http1.1`:
  `HTTP/1.1 101 Switching Protocols`. On `second`'s join (**200**, with the password) it received
  exactly `{"event":"house","data":{"seq":2,"chat":0}}`. `seq` is the house's in-memory count of
  the copies and removals it applied, so 2 matches the create's copy and then the join's.
- **Reading 4 — abandon:**
  - the creator's DELETE: **204**, and the id is gone from `scope=mine` (0 rows);
  - a re-create naming the SAME id: **201**. Under an unacknowledged removal it would be 409, so the
    removal was acknowledged and the object ended. The tail shows a NEW generation: `at`
    1790206923, with a new `rand`;
  - its DELETE: **204**.
- **Reading 5 — `/api/art`:** its keys, unsorted, are `orders, techs, sites, achievements, areas,
  five_power, mercenaries, backs, tokens` — the nine maps in the struct's order, with no
  `area_duplicates` and no `provenance`.
- **Reading 6 — the tail:** `slot-pushed`/`slot-acked` pairs for revisions 1 (the create), 2 (the
  join) and 3 (the REMOVAL), then for the re-created generation's 1 and 2 (its removal). No
  `copy-refused`, `trap` or `alarm-failed` line, and no alarm fired.
- **Against the spike (spec §0.3):**
  - row 2, startup: 2 ms, within the spike's 1–3 ms, and the fourteenth's 2 ms. The upload is
    larger than the spike's 1,172.84 KiB because the Worker carries the shell;
  - row 3, `getrandom` in an object: the same. Reading 1's create was LOCKED, so `Lock::new` drew
    its 16-byte salt through the module's `getrandom` inside the table object, and it worked: the
    create answered 201 `locked: true`, and `second`'s join with the password answered 200.

### What this deployment does NOT claim

- **Spike rows 4 and 5 were not re-read here:** no trap was raised live, and no object's CPU was
  measured.
- **No alarm fired, by design.** Every push was acknowledged, and the acknowledgement deletes the
  slot's alarm. So a retry's own delivery, and `ctx.id.name` inside a production `alarm()` (the
  plan's pool fact 8), were not read live. The pool's cells are their evidence.
- **The pool-only paths:** an owed copy (`copy_owed`) and its backoff, a refused copy, a failed
  push and its retry, and the house chat's load traps were not raised live.
- **The corpus replay ran in the pool only**; the live replay is Task 15's. The slot's `last_active`
  and `bytes` reached the house, but the operator's listing that shows them is Task 14's.

### Rollback

§6. The earlier version is now the fourteenth's, `8b1bdcf8-610d-4b04-9e21-727e216cd61a`, and no
migration was added, so `npx wrangler rollback` can return to it. Its code reads no copies, so its
lobby would list nothing; the copies stay in the house's storage. Every table and its game stay as
they are: the fourteenth's code never reads the `meta` keys this deployment added, and a pending
slot's alarm finds nothing to do there.

## 10. The archive is live — the sixteenth deployment, 2026-09-24 (a record, not a plan)

From `4d984f52`, Task 13's reviewed head, its fix round included, tree clean. It brings the archive
on R2: the bucket `yokohama-archive` bound as `ARCHIVE` and the switch at `r2`. An abandon's save
and a browser's are uploaded between two dispatches of the layer, the receipt loop (spec §2.6). A
game over's save is uploaded after the move's reply, and then stamped by the layer's own
`stamp_archive`. **Version `a8e887b5-6212-47ce-a493-abbd40b53cd6`.** Every reading holds. The
readings ran with the credential and claimed `.test` identities, the tail listening, and the header
file deleted afterwards.

- **The bucket, read back through the account API before it was bound** (2026-09-23): created
  2026-09-23T05:26:17Z, location ENAM, storage class Standard, the default jurisdiction, 0 objects.
  It is private: its managed `r2.dev` domain is disabled, and the planning session gave it no custom
  domain. Its one lifecycle rule is the default that aborts an unfinished multipart upload after 7
  days.
- **Build:** `worker/`'s `npm run build`. The module is **1,220,664 B**, the fifteenth's size, and
  nothing under `crates/` or `web/` changed between the two; the assembly **153 entries verified**,
  156 files, 9,967,639 B; the release smoke **200**, under its own configuration with the archive
  `off`.
- **The deploy:** the guard PASSED, naming "the same object bindings, migrations and bucket binding
  in both". Total Upload **1311.47 KiB / gzip 499.90 KiB** (the fifteenth's: 1288.14 KiB / 493.52
  KiB); **Worker Startup Time 2 ms**. The bindings now include `env.ARCHIVE (yokohama-archive)`, an
  R2 bucket, and `env.YOKOHAMA_WEB_ARCHIVE ("r2")`. Wrangler's OAuth login accepted the R2 binding,
  so the plan's fallback — a fresh `wrangler login` — was not needed.
- **Reading 1 — health**, over WARP as the owner: `identity: cf-access`, `signal: ws`,
  **`archive: r2`**.
- **Reading 2 — an abandoned game lands.** Through the credential, `red@deploy.test` created a
  LOCKED table, `t13-abandon-1790214892` (seed 20260831), and `blue@deploy.test` joined with the
  password (**200**); both keeps answered **200**. Red's lines `1, 2, A09, confirm, 1, 1, 1, 1, 1`
  took the record to 1, 1, 1, 2, 3, 4, 5, 6 and 7 events, the last reading `watching` with
  `to_move 1`, and Blue's `1` took it to 8. The creator's DELETE answered **204**, saved first and
  then deleted, and the id is gone from `scope=mine`.
- **Reading 3 — the pair, read back** with `npx wrangler r2 object get … --remote` through
  wrangler's own login. The pair sits under the `test/` root, never the owner's paths:
  - `test/rev7/abandoned/t13-abandon-1790214892.json` holds **8 events** in 1,301 B;
  - its `.table.json` reads `lock: null` and `unlocked: []` (`strip_for_archive`),
    `archived_at: 1790214893` and status `playing`;
  - the record LOADS in the CLI (`cargo run -p yokohama-cli -- <file>`), and `log` lists the eight
    moves. Red ends the additional-action phase, places 2 assistants in Customs (A09), places the
    President from hand in Customs, takes the area action declining every optional part, skips the
    5-Power token, skips the building and ends the additional-action phase; then Blue ends the
    additional-action phase.
- **Reading 4 — the conditional put's form, live** (spec §2.6, §3.2). The tail shows this line, and
  no `archive-failed` line:

  ```json
  {"event":"archive-landed","id":"t13-abandon-1790214892","site":"abandon","record_key":"test/rev7/abandoned/t13-abandon-1790214892.json","detail":"test/rev7/abandoned/t13-abandon-1790214892.json"}
  ```

  `TypeError` appears **0** times in the tail, and all **40** invocations read `outcome: "ok"`.
  **The form that shipped is `onlyIf: new Headers({"If-None-Match": "*"})`, and production R2
  ACCEPTED it**: the record's create-if-absent put landed. The plan's fallback, the object form
  `onlyIf: {etagDoesNotMatch: "*"}`, was not needed. Cloudflare's type for `onlyIf` admits both
  (`R2Conditional | Headers`).
- **Against the spike (spec §0.3):** row 2, startup: 2 ms, within the spike's 1–3 ms, as at the
  fourteenth and fifteenth deployments.
- **The owner's optional lifecycle rule on `test/` was asked and is OPEN.** Every live tier's saves
  accumulate under that prefix (spec §7); the rule is the owner's to set, and none is set.

### What this deployment does NOT claim

- **A finished game's save was not read live.** Only an abandon's save landed. The game-over site
  is read at the exit (Task 16), where a whole game is played: the save owed in the move's batch,
  the upload after the reply, `stamp_archive` at the recorded reading, and the `archive-stamped`
  line. The pool's cells are its evidence until then.
- **The refusal itself was read in the pool only.** Live, the conditional form was read only as
  ACCEPTED. A create-if-absent put meeting a present record, refused by the bucket and then re-read
  as `identical` or `conflict`, is proven against miniflare's R2 by the two race cells in
  `worker/test/archive.test.ts`. Production R2's answer to that precondition was not read.
- **No alarm fired**, so no retry of a failed upload, and no save owed across an eviction, was read
  live.
- **The CLI read was of a two-human record**: eight moves of an abandoned game, with no bot seat and
  no final scoring. A finished game's record is read at the exit.
- **A browser's save (`POST /api/archive`) and the switch's `off` paths were not read at this
  version**, `archive-owed-kept` and `archive-owed-dropped` among them. The corpus replay's
  `archive-walk.json` ran in the pool only; the live replay is Task 15's.

### Rollback

§6. The earlier version is now the fifteenth's, `e355c676-cfb2-400b-926d-05def2e725fd`. No
migration was added and that version binds no bucket, so `npx wrangler rollback` can return to it.
A version captures "its bundled code, static assets, bindings, and compatibility settings"
(Cloudflare, *Versions & deployments*), and its variables are bindings, so the fifteenth would run
with no bucket bound and the archive `off`: a game over saves nothing, and an Abandon deletes
without a save. Its code never reads `archive_owed`, so a save owed at the rollback waits in `meta`
— unless an Abandon deletes that table meanwhile, which takes the save with it, and no line names
it. "Resources connected to your Worker will not be changed during a rollback" (Cloudflare,
*Rollbacks*), so every save already in the bucket stays.

## 11. Expiry, the marker and the operator are live — the seventeenth deployment, 2026-09-24 (a record, not a plan)

From `c2e8d084`, Task 14's reviewed head, its fix round included. It brings expiry, the marker and
the operator surface. A table idle for `YOKOHAMA_WEB_TABLE_TTL_DAYS` days (`90`) ends as its
creator's own Abandon, saved first while the archive is on. A table the layer deleted answers every
request with its MARKER for 90 days. And the operator surface is gated on the secret
`YOKOHAMA_WEB_ADMINS`. **Two versions:** the code deploy made
`eb764c8b-69c6-475c-a30e-8815ff0e690f`, and the secret put made
**`0b7575a8-7b56-4c53-8619-a82b52b3a63b`**, which is the one live: all 43 of the tail's events
carry it. Every reading holds. The readings ran over WARP as the operator, with no credential, and
through the credential as claimed identities — `.test` addresses, and in reading (f) the
operator's own — the tail listening, and the header file deleted afterwards. The operator's address
is written nowhere here (the plan's ruling P-1).

- **Build:** `web/`'s `npm run build` — `precache: 153 entries, 9,938,226 B`, the cache
  `yokohama-release-10e4457c62975466`, the fourteenth's. `worker/`'s `npm run build` — the module
  **1,220,664 B**, 16 exports and no `debug_trap`, the sixteenth's size, and nothing under
  `crates/` or `web/` changed between the two; the assembly **153 entries verified**, 156 files,
  9,967,639 B; the release smoke **200**.
- **The deploy (`npm run deploy`):** the guard PASSED all seven checks, its line naming the new one
  last:

  ```text
  deploy-guard: the optimized release module; the assets tree is the release, with no debug browser module; no workers.dev or preview URL; no test clock; spec §2.3's assets block in both configurations; the same object bindings, migrations and bucket binding in both; no operator list in its vars
  ```

  Wrangler read 172 asset files and re-uploaded none. **Worker Startup Time 1 ms.** The bindings
  list names `env.YOKOHAMA_WEB_TABLE_TTL_DAYS ("90")` beside the three Durable Object classes, the
  bucket and the other variables.
- **The secret** (the plan's ruling P-1), after the deploy: `printf '%s' '<operator>' | npx
  wrangler secret put YOKOHAMA_WEB_ADMINS`, the value piped on stdin. Wrangler reported it
  uploaded, and `npx wrangler secret list` names exactly `YOKOHAMA_WEB_ADMINS`. A secret put
  deploys a new version at once, and that version, `0b7575a8-…`, is the one the readings ran
  against. `/api/health` then answered **200** with `archive: "r2"`, so the list parsed: a
  malformed one would have answered it with the 500 naming the variable (§3).
- **The premises**, over WARP as the operator: `GET /api/me` answered **200**
  `{"mode":"cf-access","user":"<the operator's address>"}`, and `/api/health` **200** with
  `identity: "cf-access"`, `signal: "ws"`, `archive: "r2"` and `semantics_rev` 7.
- **(a) and (b) — the listing.** Through the credential, `red@deploy.test` created
  `t14-force-1790224338` (**201**). `GET /api/admin/tables` as the operator answered **200** with
  its row, all ten fields:

  ```json
  {"id":"t14-force-1790224338","name":null,"status":"open","players":2,"created_at":1790224338,"last_active":1790224338723,"bytes":24576,"over":false,"winner":null,"archived_at":null}
  ```

  `created_at` is the layer's unix SECONDS and `last_active` the shell's MILLISECONDS: the same
  second. **The listing held ONLY that row.** The lobby holds no copy of a table from an earlier
  deployment, so no stale table existed to force.
- **(c) and (d) — a force frees the id at once.**
  `POST /api/admin/tables/t14-force-1790224338/expire?force` as the operator answered **204** with
  an empty body. A create at the same id then answered **201**: the tail shows a NEW generation,
  `at` 1790224339 with a new `rand`, and no `slot-ignored` line. Forced again: **204**.
- **(e) — the marker, live.** `t14-marker-1790224338`, created through the credential as
  `red@deploy.test` (**201**), was deleted by its creator with no move made (**204**). Then:
  - `GET …/table`, over WARP as the operator, answered **404** with the marker sentence:

    ```json
    {"error":"this table was abandoned on 2026-09-24 — no move had been made, so there was nothing to keep"}
    ```

  - a create at the id, through the credential: **409** `{"error":"this name was used recently —
    try another"}`;
  - forced: **204**. Its removal was already acknowledged, so the force pushed nothing and ended
    the object at once.
- **(f) and (g) — the gate refuses the dev door, live** (battery 32). The credential claiming the
  operator's address: `GET /api/me` **200** `{"mode":"cf-access","user":"<the operator's
  address>"}`, then `GET /api/admin/tables` **403** `{"error":"this address is not an operator
  here"}`. The credential claiming `red@deploy.test`, an address not on the list: **403**, the same
  body.
- **(h) — nothing left behind:** the listing then held **0** rows whose id starts `t14-`.
- **Three more readings** — the fix round's rulings I-72 and I-74, and spec §2.9's force on an
  object that holds nothing:
  - `POST /api/admin/tables/t14-nothing/expire?force=false`: **400** `{"error":"?force takes no
    value — send ?force alone, or no query for an expire that saves first"}`. The tail has ONE
    event for it, the stateless Worker's: no object was reached;
  - `GET /api/admin/tables?x=1`: **404** `{"error":"no such endpoint: /api/admin/tables"}`, the
    layer's own, answered by the house;
  - `POST /api/admin/tables/t14-nothing/expire?force`: **204**, with an `operator` line and no
    `forced` line: the object held nothing, so nothing was written.
- **The tail** (`wrangler tail --format json`): 43 events from 04:32:12 to 04:32:45 UTC, all
  `outcome: "ok"`, 0 exceptions. **It is PRETTY-PRINTED: parse it by whole JSON objects, never
  line by line** — a parse by lines tripped these readings once.
  - `operator` ×6, each with the operator's address: `list` ×2 and `force` ×4;
  - `forced` ×3: the two forces of `t14-force-1790224338` and the marker's. The force of
    `t14-nothing` logged none;
  - `slot-pushed` and `slot-acked` ×6, in pairs: three creates' copies (`removal: false`) and three
    REMOVALS (`removal: true`), for the two forces of a live table and the abandon. The marker's
    force pushed nothing;
  - `expired`, `archive-owed-dropped`, `trap` and `slot-ignored`: **0**. **No alarm fired**: every
    event is a request or one of the six `apply` calls a table made to the house;
  - of the 20 requests, the 17 that reached an object appear twice, the stateless Worker's event
    and the object's. The two 403s and the 400 appear once, answered by the Worker before any
    object.
- **A transitional `forced` line** (the review's L-4). From this deployment on, a `forced` line
  with no `operator` line before it would be the acknowledgement of a REMOVAL that a delete by the
  earlier code left unacknowledged. That code wrote no marker, so the acknowledgement ends the
  object as a force's does. **None was seen.**
- **Against the spike (spec §0.3):** row 2, startup: **1 ms**, within the spike's 1–3 ms; the
  fourteenth to the sixteenth deployments read 2 ms.

### What this deployment does NOT claim

- **No table expired live.** None had been idle for 90 days, and no alarm fired. The expiry — its
  save, `expired`, `expiry-failed`, `held` and `expiry-blocked` — and the head `this table expired
  on …` were read in the pool only.
- **An operator's plain expire** (no `?force`) was not called, so the head `this table was removed
  by an operator on …` was not read live. Nor were the tails `— its game is in the archive` and
  `— this server keeps no archive, so its game was not kept`: the one live marker was an abandon
  with no move made. It was forced before its 90 days, so `marker-ended` was not read either.
- **A force's 503** was not read: every push was acknowledged at once. No force met a poisoned or
  `held` table, or a save still owed.
- **The gate's refusal of an unlisted PERSON** was not read live. Reading (g) went through the
  credential, which the gate refuses whatever address it claims, and no Access login but the
  operator's was used. The node and pool cells are its evidence.
- **The listing's `name`, `over`, `winner` and `archived_at`** were read live only at their empty
  values. The pool's listing cell pins them at others (the review's M-2).

### Rollback

§6. The earlier version is now the sixteenth's, `a8e887b5-6212-47ce-a493-abbd40b53cd6`, and no
migration was added, so `npx wrangler rollback` can return to it. Its code has no expiry and no
operator surface: no table expires, and every `/api/admin` path is the layer's 404. An alarm this
deployment armed, for an expiry or a marker's end, runs the older alarm work there and ends
nothing. That code never reads a marker either:

- a table whose marker stands answers the layer's own `no game named …` in place of the marker
  sentence, and its object keeps its `meta` rows, since only this deployment's code ends a marker — unless the marker's removal was still waiting in the slot at the rollback: the older code pushes it, and its acknowledgement runs `deleteAll()`, which ends the object before its 90 days;
- a create at such an id is no longer refused once the lobby has acknowledged the removal. It
  reuses the generation the object still holds, which the lobby retired at the removal, so for 90
  days from the removal the lobby ignores the new table's copies (`slot-ignored`) and does not list
  it; the table itself plays by its link. A create whose body names no id is given a random `sim-`
  name, so only a create that names its id can meet one.

What a rollback does with the secret was not read.

## 12. The client knows its transport before it connects — the eighteenth deployment, 2026-09-24 (a record, not a plan)

From `d025ba43`, Task 15's second fix round, its scoped re-review clean. It brings the two client
fixes the first live run found (§7): no stream or socket opens before the deployment's transport
is known, and the keep-alive stops at hang-up and sends only while the socket is OPEN. The deal's
designed 409, the third finding, is the harness's to forgive, and no deployed code moved for it. **No
crate moved**: the Worker's code is the seventeenth's. **Version
`93f73009-0e3e-4c21-a73f-e164ee5d58a5`** — every event of its tail carries it. Every reading holds.
The readings ran through the credential as claimed `.test` identities, and over WARP as the owner
for the replay's forces.

- **Build:** `web/`'s `npm run build`, REQUIRED because the last local e2e had left the DEBUG module
  in `web/dist` — `precache: 153 entries, 9,938,696 B`, the cache
  `yokohama-release-b4f28a3400c79f9c`, and `grep -c debug_trap web/src/local/wasm/yokohama_wasm.js`
  **0**. `worker/`'s `npm run build` — the module **1,220,664 B**, unchanged; the assembly **153
  entries verified**, 156 files, 9,968,109 B; the release smoke **200**.
- **The deploy (`npm run deploy`):** the guard PASSED all seven checks. Wrangler uploaded **3** new or
  modified assets — the client bundle `index-J8fhXo2G.js` among them — and found 151 already
  uploaded. `/api/health` **200**: `identity: "cf-access"`, `signal: "ws"`, `archive: "r2"`.
- **Reading 1 — `joins_by_link` LIVE: PASSED** (1 passed, 4.3 s). Table `sim-55b67d78eca70411`, 6
  moves across two browsers in 4 s. The deal's designed 409 was forgiven on page a twice (its record
  and its console line) and on page b not at all; the beacon's CORS pair a 2 / b 2. **No 426 on
  `/api/stream`**: the transport wait held live. The 390 px readings printed as the first live run's
  did, but for the who-strip, now 47 px tall (71 px at the first run).
- **Reading 2 — the corpus replay, TWICE: both HOLD**, exit 0.
  - **Run 1:** the forces before `wire2 204, wire 204, wire4 204` (objects holding nothing answer
    204); the house chat's base `last 0`; **2,209 of 2,209 rows run, 86 named rows compared, 13
    exemptions spent** — the lobby filter ×4 with 0 foreign rows dropped; five host fields,
    `health.identity` `claimed → cf-access`, `signal` `sse → ws`, `archive` `off → r2`, and
    `me.mode` and `me_signed_out.mode` `claimed → cf-access`; the house chat ×2; the two first sends
    "201, the first send landed"; 0 lobby re-reads of the bound (5 per row, 500 ms apart);
    **mismatches none**; the forces after all 204.
  - **Run 2:** the same from base `last 1` (run 1's own post), the house chat taken back by 1 with 1
    other message dropped, and **both first sends "200, the once-rule's `identical: true`"** — the
    repeatability evidence.
- **Reading 3 — the tail over readings 1 and 2** (`wrangler tail --format json`, parsed by whole
  objects): **9,292 events** — `ok` 9,274, `canceled` 9, `responseStreamDisconnected` 4, `unknown`
  5 — and **0 exceptions**. The lines: `operator` ×10, `forced` ×6; `slot-pushed` 394 / `slot-acked`
  394 / `slot-ignored` 0; `archive-landed` 3, `archive-identical` 5, `archive-conflict` 2 (the
  replay's own 409 rows), `archive-failed` 0; and `trap`, `load-trap`, `held`, `expired`,
  `init-refused`, `frame-failed` and `identity-failed` all **0**.
- **The quota** (§7): before these readings, at 06:48 UTC — after the day's first live run, on the
  seventeenth deployment — **44,270** Worker requests account-wide (`yokohama-web` 38,390), 7,259 object rows written and 1,344,253 read; after
  readings 1 and 2, at 10:16 UTC, **51,467** (`yokohama-web` 42,524, 0 errors — the join and the two
  replays added about 4,100), 14,939 rows written and 1,562,634 read. The headroom to the owner's
  80,000 line was then about 28,500: the two-browser re-run, some 17,000–38,000 requests, waited for
  another UTC day.

### What this deployment does NOT claim

- **A two-browser game was not played on it.** The first live run's game (the seventeenth deployment,
  §7) played to a winner before these fixes; the console-clean whole game is the nineteenth
  deployment's (§13).
- **The keep-alive fix was not exercised live here**: `joins_by_link` lasted 4.3 s, under the 30 s
  keep-alive, and the replay opens no socket. Its live witness is the nineteenth deployment's
  two-browser re-run (§13).
- **The five `unknown` outcomes are unexplained.** The Workers Observability query API can classify
  them (§7); the ledger leaves them for the cleanup cycle.
- **No alarm fired** that the tail's reading names.

### Rollback

§6. The earlier version is the seventeenth's live one, `0b7575a8-7b56-4c53-8619-a82b52b3a63b`. No
migration was added and no crate moved, so `npx wrangler rollback` can return to it with the same
Worker code and the same variables. It brings back the client that opened the lobby's house channel
as SSE before `/api/health` answered — a 426 and a console error on this host — and that could send a
keep-alive into a closing socket.

## 13. The fix wave, and the exit's window — the nineteenth deployment, 2026-09-24 (a record, not a plan)

From `bd8ce497`, the fix wave's follow-up, every re-review and re-check clean (the close §4). It
brings the whole-branch review's fixes to the Worker and the client: an exception escaping an object
answers the API's 500 `the server could not answer — try again`, logged `object-failed`; a
generation's time is milliseconds; a token without a string `kid` is no one; a dev-door claim must be
an address; a picker row carries its code's digest and a row under other code, or none, is dropped;
the `wake` line; and the client's transport wait bounded by `BOUND_MS`. **It also opens the exit's
window**: `YOKOHAMA_WEB_COUNT_WAKES` reads `on`, so every counted wake writes `meta.wakes` and logs
`wake {id, wakes}` (§7, "The exit's window"). **Version `204aba6b-2794-4e95-8567-7558cf7c44b2`**
(deployment `f41c4d25…`). The window stayed open from 2026-09-24 10:49:45 UTC to the twentieth
deployment, across the two-day stop at the account's weekly limit. Every reading holds. The readings
ran through the credential as claimed `.test` identities.

- **Build:** `web/`'s `npm run build` — 153 precache entries, the cache
  `yokohama-release-3d7b48a6d2d78e74`, `debug_trap` **0**. `worker/`'s `npm run build` — the module
  **1,855,780 → 1,221,719 B** with `wasm-opt` 133 (the code stamp grew it by 1,055 B); the assembly
  **153 entries verified**; the release smoke **200**.
- **The deploy, which opens the window**, run in `worker/`:

  ```sh
  node scripts/deploy-guard.mjs && npx wrangler deploy --var YOKOHAMA_WEB_COUNT_WAKES:on
  ```

  The guard PASSED all **nine** checks, the two freshness checks among them — "built from the sources
  as they are" and "assembled from the current web/dist and worker/static". Wrangler uploaded **3**
  new or modified assets, the client bundle `index-DqiipaaU.js` among them, and found 151 already
  there. **Worker Startup Time 1 ms.** The bindings list shows `env.YOKOHAMA_WEB_COUNT_WAKES
  ("(hidden)")`: wrangler hides a `--var`'s value. `/api/health` **200**: `identity: "cf-access"`,
  `signal: "ws"`, `archive: "r2"`.
- **Reading 1 — the exit game, run 1** (2026-09-24 10:50 UTC, headed, `WAKE_PAUSE_MS=60000`). Its
  seating PASSED LIVE in 5 s: table `sim-856c9a6ea7a81c37` opened locked by `x@exit.test`;
  `y@exit.test` read x's lobby line, posted its own and joined through the lobby's Open-tables link
  with the password; both kept; `w@exit.test` unlocked it and watched with no hand. Then the spec's
  own precondition stopped it before the pause after move 5: x's window read hidden at 10:50:28 UTC.
  The three headed windows stacked on the display, and macOS marks a covered window hidden whatever
  Chromium's two occlusion switches read. Not a finding about the product. The `afterEach` abandoned
  the table.
- **Reading 2 — the headed walk at 390 px** (2026-09-24; headed Chromium at 390×844 @2×, two `.test`
  contexts, the orchestrator's eye on 16 + 16 screenshots). The door, the lobby with its back-link, the
  create form, the open table in the creator's view and a stranger's, the deal, the board and the
  folded chat panel: clean, no layout defect. ONE error strip provoked in the browser — the unlock
  strip, the open table's password refusal for a stranger (a wrong password at "sit here" → 403 on
  `/join`, "wrong password for this table"); not provoked: the create form's (a spaced name is a
  legal display name), the deal's (the page's guard prevents a second keep; the 409 read through the
  API) and the chat panel's (the input eluded the script; the empty post's 400 read through the
  API); the open table's own strip was not attempted. The console: the beacon's two-line artifact and
  the designed refusals — nothing else.
- **Reading 3 — the bucket, and a bot move reconstructed** (2026-09-24), synced with the owner's
  read-only key in ONE command's environment: `test/rev7/over/sim-552c786c4ba671ff.json` (180,493 B,
  the first live run's two-person game: 2,180 events, `over: true`, `winner: 0`, `archived_at`
  1790232277) and its table; the exit's and two bot games' abandoned pairs; the replay's saves. The
  finished record loads in the CLI, its `log` 2,180 lines long. The bot game `exit-bot-1790247143`
  loads, and **`why 0` reconstructs the bot's first move** — `v4-step9-draft1`, "Only legal
  option", decision seed `0x2e874e72bec9ed69`, the 34-row explain table, TOTAL 15: the honesty gate
  passed under this build's constants.
- **The stop.** The account's weekly limit stopped the session at ~10:58 UTC on 2026-09-24, and it
  resumed at ~06:04 UTC on 2026-09-26. The tail of the 24th had died at 10:55 UTC; a fresh one
  (`tail19b.json`) ran for the readings below. The WARP client read Connected, but its device's
  Access login had EXPIRED: every request from the workstation without the credential answered
  Access's 302 "Please authenticate via the warp client" (a curl-shaped one met Bot Fight Mode's
  challenge first). The owner re-authorized it on the 26th. No reading below needed it: the exit, the
  re-run, the deploys (wrangler's OAuth) and the bucket's sync do not.
- **Reading 4 — the exit game, run 2** (2026-09-26 06:08 UTC). Its seating PASSED again in 4 s
  (`sim-6a7ae105764b7f76`). The windows were placed side by side, but Chromium widens a window to its
  MINIMUM width, 500 px: asked 420×820 at 0, 426 and 852, they read back 500×820 at those lefts, 38 px
  down — still overlapping by 74 px — and x read hidden at 06:08:10 UTC. The precondition stopped the
  run again; not a product finding. Its failure context named `CF_Authorization` once, the cookie's
  NAME in a source comment, with 0 value-shaped matches. The spec then lays a 2×2 GRID when three
  minimum-width windows cannot share the width (`754b80ae`).
- **Reading 5 — the two-browser live re-run** (06:12–06:22 UTC, the fixed client): **PASSED**, 1
  passed in 10.1 min. Game `sim-1a8263683952c0b4` **finished in 2,510 moves**, 601 s, 0 idle looks,
  the largest hand seen 3. **The watcher held to 1,886 applied moves, asked `/events` 0 times and
  opened `/stream` ONCE at every checkpoint**; each player opened the table's stream twice, both
  before the first move. The deal's 409 forgiven 0 times; the beacon's CORS pair a 2 / b 2 / c 2;
  **no other console error on any of the three pages** — the first live run's "WebSocket is already
  in CLOSING or CLOSED state" lines are gone, and no 426 appeared. No "Call log".
- **Reading 6 — the exit game, run 3** (06:20–06:26 UTC, `754b80ae`'s grid): **PASSED**, 1 passed in
  5.2 min. Table `sim-6334a4d57da5499f`. Seating in 4 s, the grid read back x 634×391 at 0,38, y at
  640,38 and w at 0,435, under a 38 px menu-bar inset, each with a 618×301 viewport. **Forty moves in
  307 s**; four long pauses of **60.0 s**, after moves 5, 15, 25 and 35, with nothing asked
  meanwhile, each followed by the move's `POST …/input` answering in **69, 69, 75 and 67 ms** against
  an awake median of **58 ms** over the other 36 — the wake as the page sees it, about 10–15 ms.
  Table chat x 10 / y 10, the watcher's panel holding 20 lines; lobby chat x 1 / y 1; the watcher held
  to 31 applied moves with no hand; all three windows visible at 10 checkpoints; the table's stream
  opened w 1, x 3 → 3, y 3 → 3 — no reopen during play. The abandon: the address answers **404**
  `this table was abandoned on 2026-09-26 — its game is in the archive`, and x, y and w were told.
  Forgiven: the beacon 2 / 2 / 2, the deal's 409 on x twice. Every console clean. No "Call log".
- **Reading 7 — the wake witness**, off the tail (`tail19b.json`: 41,578 events on this version, 1,230
  of them the exit table's). **Four `wake` lines, `wakes` 1 → 2 → 3 → 4**, each on the first POST
  after a 60 s pause: 06:21:43.726 (CPU 8 ms, wall 19 ms), 06:23:00.569 (7 / 18), 06:24:17.630
  (9 / 60) and 06:25:34.840 (6 / 16). **The object hibernated in every pause and came back on the next
  move.** A wake costs the object **6–9 ms of CPU**, against a **2 ms** median (the maximum 15) over
  the 40 object-side move POSTs; the stateless Worker's own half is 2 ms. The table's other lines:
  `slot-pushed` 10, `slot-acked` 11, `archive-landed` 1 (the abandon's save), `trap` 0, 0 exceptions.
- **Reading 8 — the bucket, re-synced**: `test/rev7/abandoned/sim-6334a4d57da5499f.json` (3,465 B,
  the 40-move exit game) and its table; `test/rev7/over/sim-1a8263683952c0b4.json` (157,033 B, the
  re-run's game — a SECOND finished game) and its table; and the first live run's
  `sim-552c786c4ba671ff`, still there.
- **The quota** (§7): at 06:05 UTC on the 26th, **7,547** Worker requests account-wide
  (`yokohama-web` 0) and 81 rows written; at 06:26 UTC, **42,036** (`yokohama-web` 34,241), 6,466 rows
  written, 1,202,112 read, and 34,883 object invocations (18 non-`ok`, the cancels and disconnects
  kind). The re-run cost about 32,000 requests, **12.8 a move**; the exit game about 1,400 for 40 paced
  moves, **35 a move** with three pages and chat.
- **Against the spike (spec §0.3):** row 4 (a trap live) still never provoked — every tail read `trap`
  0; row 5 (an object's CPU) now read live — 2 ms a move, 6–9 ms a wake; row 6 (hibernation) SAME in
  substance — the spike saw a wake after 35 s idle, the exit saw one after each 60 s pause.

### What this deployment does NOT claim

- **The window was open for two days** (the stop): every counted wake in that time wrote `meta.wakes`
  in its batch and logged its line. No table is known to have been played meanwhile.
- **No `picker-dropped` line is recorded.** By design, every picker row an earlier deployment stored is
  dropped at its table's next request under this code (`stored: null`); the ledger's readings name
  no such row.
- **No alarm-driven line is named** in the readings of either tail, so `ctx.id.name` inside a
  production `alarm()` is still unread live.
- **The `slot-acked` count is one above `slot-pushed`** for the exit table (11 against 10); the ledger
  does not explain it.
- **The operator surface was not read on this version**: the workstation's WARP login had expired
  (above), and the operator is a person over WARP. The seventeenth deployment's operator readings
  stand.
- **The live corpus replay was not run on this version**; it last ran on the eighteenth. This
  version's server changes move no payload, and the pool replayed the whole corpus at `bd8ce497`.

### Rollback

§6. The earlier version is the eighteenth's, `93f73009-0e3e-4c21-a73f-e164ee5d58a5`. No migration
was added, so `npx wrangler rollback` can return to it — but it is a **generation-order rollback**
and it **drops every open picker** this version stored (§6's two bullets). A rollback also returns
the unbounded transport wait, the platform's plain 500 for an object's exception, and a generation's
time in seconds. A `--var` is a binding, so a rollback to the eighteenth also closes the exit's
window.

## 14. The exit's window closed — the twentieth deployment, 2026-09-26 (a record, not a plan)

From `754b80ae`, the code close head, whose shipped code is `bd8ce497`'s — the three commits after
that (`f2c27a3f`, `528b1285` and `754b80ae`) move only `web/e2e/` and the Playwright configuration.
It closes the exit's window: the plain deploy replaces the Worker's variables with
`wrangler.jsonc`'s, which name no `YOKOHAMA_WEB_COUNT_WAKES`, so the switch reads off. Deployed at
06:38:27 UTC on the owner's go, which came after the owner re-authorized the WARP client's login.
**Version `6da906a5-a088-442e-bc43-d94904630982`.**

```sh
npm run deploy   # in worker/, after npm run build in web/ and in worker/
```

- **The build:** `web/` and `worker/` rebuilt for release; `debug_trap` 0; the module 1,221,719 B
  through `wasm-opt` 133, the nineteenth's size; 153 assets verified.
- **The deploy:** the guard's nine checks passed; NO asset changed since the nineteenth, so the
  service worker is the nineteenth's, cache `yokohama-release-3d7b48a6d2d78e74`; startup 2 ms;
  `YOKOHAMA_WEB_COUNT_WAKES` is no longer among the bindings, so the window is CLOSED.
- **Health:** `/api/health` 200, `cf-access`, `ws`, `r2`.
- **The tail:** `tail19b.json`, running since the exit's second run, is recorded as stopped at
  this deployment.

### What this deployment does NOT claim

- Everything §13 does not claim, but for the open window.
- **No request was read for the absence of a `wake` line.** The switch's absence from the bindings
  list is the reading that the window is closed.

### Rollback

§6. The earlier version is the nineteenth's, `204aba6b-2794-4e95-8567-7558cf7c44b2` — the same
code with the window open. Rolling back to it re-opens the window.

---

## 15. M6.5c2's doors, refinish and figures are live, with the dev door's client id a secret — the twenty-first deployment, 2026-09-28 (a record, not a plan)

From `ea4f174d`, cycle C's reviewed head on `web-room-m6.5c2c` (stacked on cycles B and A over M6.5c;
the record of record is `docs/plans/2026-09-28-web-frontend-m6.5c2c-close.md`, and the spec's dated
§10.8 carries the corrections). It brings the whole of M6.5c2 to the Worker: the two doors
(`YOKOHAMA_WEB_IDENTITY` with no default, `worker` deployed, `open` proven in the pool and never
deployed), the refinish rule (a same-game conflict stamps; every other conflict is final), the
module's invariants and the layer's exported restatements, the `hasTables` memo (a warm request asks
`sqlite_master` nothing), the listing's one time unit, the force that flags itself, the scripts' LOW
findings, the `0.6.0` bump, the bot's version on every save and the versioned refusal, the lobby's
Finished group and the client's polish — and **`DEV_DOOR_CLIENT_ID` as a SECRET, put in this same
deploy, which drops it from `vars`** (A-1; spec §2.1). **Version `ce923774-aa81-4f2b-a8dc-b0fcb5a05f70`**,
deployed at **16:48:09 UTC**. Every reading holds. The readings ran over WARP as the owner and,
where a claimed identity was needed, through the credential.

- **Build, ONCE, for both hosts** (§1): `web/`'s `npm run build` after the close's e2e re-run —
  `debug_trap` **0**; `wasm-opt version 133` over both browser modules **1,814,905 → 1,197,069 B
  (−34.0%)**; the precache generator's four lines verbatim:

  ```
  precache: 153 entries, 10,010,662 B (release total ceiling 11,900,000 B)
  precache: module 1,197,069 B at /assets/yokohama_wasm_bg-Ciy339-r.wasm (release module ceiling 1,400,000 B; release 1,400,000, debug 3,200,000)
  precache: module 1,197,069 · bundle 464,467 · fonts 98,728 · shell 11,873 · icons 19,456 · help 764,742 · art 7,454,327
  precache: yokohama-release-605fe69f17100c46 -> dist/sw.js (29,045 B), 131 art files
  ```

  The release name is **`yokohama-release-605fe69f17100c46`**, and the same `web/dist` went to the
  NAS as `yokohama-web:0.6.0-m6.5c2` (`deploy/RUNBOOK.md` §3.17). `worker/`'s `npm run build`: the
  module **1,901,547 → 1,258,106 B (−33.8%)** with `wasm-opt` 133 (the root `Cargo.toml` paragraph
  read 1,256,770 at `d431dbc1`; +1,336 B since, the layer's Task 6 code and the version string —
  a reading to transcribe there in cycle D); `module-figures.mjs`: gzip (Node's zlib, level 9)
  491,934 B, 18 pages, 170 data segments, 123,047 B of data (the section 124,446 B encoded);
  `factory-glue: 16 exports (a release module: no debug_trap)`; `assemble-assets: 153 precache
  entries verified — every file present, every SHA-256 the release's`; `smoke-release: … GET
  /api/health 200` with `server_version 0.6.0` and `archive: "off"` (the smoke runs with the archive
  off by design).
- **The deploy, with the secret** (§2, §4), run in `worker/` with the tail already listening:

  ```sh
  S="$SCRATCH/dev-door.secrets"; umask 077
  id="$(grep '^CF-Access-Client-Id=' ../.dev.vars | cut -d= -f2- | tr -d '[:space:]')"
  printf 'DEV_DOOR_CLIENT_ID=%s\n' "$id" > "$S"; unset id
  node scripts/deploy-guard.mjs && npx wrangler deploy --secrets-file "$S"
  rm -f "$S"
  npx wrangler secret list
  ```

  The secrets file read `-rw-------`, 59 B, and was DELETED after the deploy (`ls` found nothing);
  its value is in no log and no document. **The guard PASSED every check** and printed the door
  first — `wrangler.jsonc deploys the worker door — Cloudflare Access, verified by the shell` —
  then its verdict: the optimized release module built from the sources as they are; the assets tree
  the release, with no debug browser module, assembled from the current `web/dist` and
  `worker/static`; a door named, and its rules (no preview URL; at the worker door no `workers.dev`
  URL and both Access values); no test clock; spec §2.3's assets block in all three configurations;
  the same object bindings, migrations and bucket binding in each; no operator list or dev-door
  client id in its `vars`. Wrangler (4.136.3) read 172 files, uploaded **6 new or modified assets**
  (`/index.html`, the stylesheet, the Worker chunk, the bundle, `/sw.js`, the module; 148 already
  there; 1.90 s), **Total Upload 1386.14 KiB / gzip 526.03 KiB, Worker Startup Time 2 ms**, listed
  `env.DEV_DOOR_CLIENT_ID ("(hidden)")` beside the nine `vars` (`YOKOHAMA_WEB_IDENTITY ("worker")`,
  the two Access values, `YOKOHAMA_DEV_DOOR ("on")`, the archive `r2`, the TTL 90, the three chat
  values), and deployed the trigger `yokohama.moore.nyc (custom domain)`. `wrangler secret list`
  names **`DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS`** — names only.
- **Reading 1 — health and the owner** (16:49 UTC, over WARP, a browser User-Agent): `/api/health`
  200 `{"server_version":"0.6.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"cf-access","signal":"ws","archive":"r2","lobby_chat":true,"chat_keep":500,"chat_text":500,"multiplayer":true}`;
  `/api/me` → `{"mode":"cf-access","user":"<the owner's address>"}` (recorded with that description
  in the value's place, R-6.5c2.16).
- **Reading 2 — the served release**: `/sw.js` 200, `text/javascript`, `content-length: 29045`,
  `cache-control: no-store`, naming ONE generation `yokohama-release-605fe69f17100c46`, byte-equal to
  `web/dist/sw.js`.
- **Reading 3 — the dev door works with the secret** (§4's header-file recipe, the file mode 600 and
  removed after): the credential + `X-Yokohama-Claimed-User: smoke@readme.test` → `/api/me`
  `{"mode":"cf-access","user":"smoke@readme.test"}`; the credential alone → `{"mode":"cf-access","user":null}`.
  So the secret reached the version that dropped the variable, and the dev door never shut.
- **Reading 4 — `picker-dropped`**: every picker row stored under the twentieth is dropped at its
  table's next request under this code (I-85, I-91). The tail read **0** such lines through the
  close's readings: the lobby held no table of the owner's and none `playing` (`scope=mine` and
  `scope=playing` both `count 0`), so no table object stored under earlier code was asked anything;
  the tables the live tiers created below were all created under this version.
- **Reading 5 — the rows read, the F-4 reading** (spec §2.7; `worker/README.md` §5.2). The
  Durable Objects analytics before and after a counted burst on ONE throwaway table,
  `sim-248e605df587a848`, created 201 at 16:51:17 UTC through the dev door as `burst@readme.test`
  (seats `{user}` + `{bot: true}`), ONE cold `GET …/table` (200, 95 ms), then **50 warm
  `GET …/table`, all 200, in 6 s**, then `DELETE` 204 and the marker's 404
  (`this table was abandoned on 2026-09-28 — no move had been made, so there was nothing to keep`).
  The day's figures moved from **266 rows read / 221 written / `yokohama-web` 3 requests** (16:42
  UTC, before the deploy) to **491 / 253 / 63** (16:53 and again 16:56 UTC, the analytics settled):
  **+225 rows read and +32 written for +60 Worker requests** — the burst's 50 warm reads, one cold
  read, the create, the delete and its 404, two house listings, and seven reads that touch no object
  (health, `/api/me` ×3, `/sw.js`). About **4 rows read per request all-in**, and consistent with the
  pool's MEASURED **2 rows a warm table request** (cycle C Task 5's `reads.test.ts`: 32 → 2) once the
  cold read (about 32), the create, the delete and the house's reads are set aside. M6.5c's live
  figure was ~470 rows read per MOVE in a three-page game (§13's re-run: 1.2 M over 2,510 moves);
  this cycle's per-move figure is Reading 7's.
- **Reading 6 — the `test/` lifecycle rule** (R-6.5c2.12; spec §2.10), set at 16:49 UTC:

  ```sh
  npx wrangler r2 bucket lifecycle add yokohama-archive expire-test-after-30-days test/ --expire-days 30
  npx wrangler r2 bucket lifecycle list yokohama-archive
  ```

  A prefix was given, so wrangler asked no whole-bucket question and `--force` was never passed
  (nor `lock add`, which is a retention rule). The list reads back TWO rules: `Default Multipart
  Abort Rule` (enabled, `(all prefixes)`, "Abort incomplete multipart uploads after 7 days") and
  **`expire-test-after-30-days` (enabled, prefix exactly `test/`, "Expire objects after 30 days")**.
  No expiring rule reads "(all prefixes)". Every save under `test/` — the live tiers', the replay's
  first sends' and the exit's — is gone 30 days after it was written; the owner's own paths are out
  of its reach by construction (§7's "What a live run leaves"). One observation: that one `npx
  wrangler` call resolved the Homebrew global (`wrangler 4.112.0` at `/opt/homebrew/bin/wrangler`)
  where every other call in `worker/` resolved the package's 4.136.3; both accept the same form, and
  the rule is the bucket's whatever CLI added it.
- **The live corpus replay, twice, sequentially** (§7), after the reset's four deletes
  (`test/browser/red_at_wire.test/rev7/{over,abandoned}/wire{,.table}.json` — Task 9's version bump
  had changed the walk's records, so the first sends would have met a 409): **run 1** (16:57–17:00
  UTC) forces 204 ×3 before and after, **2,209 of 2,209 rows run, 86 named rows compared,
  `firstSendAccepts` 201 for both first sends, mismatches none, HOLDS**; **run 2** (17:00–17:03 UTC)
  the same 2,209/86, both first sends **200 with the once-rule's `identical: true`**, mismatches
  none, HOLDS. The host-field exemption was spent as L-5 requires (`me_signed_out.mode: "open" →
  "cf-access"`), and the house chat's `seq` taken back by 8 then 9 (the permanent posts). Each run
  cost about 2,190 requests and about 21,800 rows read.
- **The tail, an instrument reading**: `wrangler tail --format json` was started before the deploy
  and held through the replays (1,139 events, every outcome `ok`, 0 exceptions, the replay's
  `operator`/`forced`/`slot-pushed`/`slot-acked`/`archive-*` lines as §5 names them). **It lost its
  connection at 17:04:02 UTC as the live Playwright tier started** ("Tail connection lost.
  Reconnecting (attempt 1 of 5)" three times) and received nothing of that run — a fresh tail was
  started for the re-run and the exit. A tail is not a log: what it did not carry is unobserved,
  not absent.
- **Reading 7 — the live Playwright tier on this version** (§7). **`joins_by_link` PASSED** (5.0 s;
  table `sim-7c76bcb64da1d170`, 6 moves across two browsers in 4 s; the deal's 409 forgiven a 2 / b
  0, the beacon's pair a 2 / b 2; the open table at 390 px: no overflow, the chat row 98 % of the
  row). **`two_browsers` read RED once and PASSED once.** Run 1 (17:04 UTC, both specs in one
  invocation, `workers: 1`) stopped at 13.9 s on `the watcher never asks /events while a stream is
  open`: ONE `/events` ask by the watcher at the twentieth watch check, which on this host is the
  socket arm's one probe after a close the client did not make (`web/src/signal.ts`; a hidden tab
  or a teardown probes nothing) — so the watcher's socket closed once, unexplained, within the first
  twenty applied moves. The tail had lost its connection at 17:04:02 UTC and carried nothing of that
  table, so the server side of that close is UNOBSERVED; the workstation read a load of 10.9 on 10
  cores (the close's mutation battery beside it). Run 2 (17:16–17:25 UTC, the spec alone, a fresh
  tail attached throughout): **game `sim-a34bc7df08d68d31` finished in 1,795 moves, 535 s, 0 idle
  looks, the largest hand seen 3; the watcher was held to 1,394 applied moves, asked `/events` 0
  times and opened `/stream` ONCE — 1 at every one of 69 checkpoints**; the players' streams a 2 /
  b 2, both before the first move; the deal's 409 forgiven 0 / 0 / 0; the beacon's pair 2 / 2 / 2;
  every console clean; no `Call log`. The tail's 1,065 sampled events of that table (wrangler
  samples a busy tail: 457 `/input` rows of 1,795 moves): `picker-dropped` 0, `object-failed` 0,
  `frame-failed` 0, `trap` 0, exceptions 0; the object's `/input` CPU median 4 ms, max 26; one stream
  upgrade row. **The rows read per move, LIVE: about 56** — 101,236 rows over the game's window
  (run 1's short game inside it) for 1,795 moves — against M6.5c's ~470 (§13's re-run: 1.2 M over
  2,510); rows written ≈ 2.5 a move and requests ≈ 13.8 a move, both unchanged, as spec §6 says.
  Run 1's one probe stands as recorded and unexplained (the close's §7); the re-run with the tail
  attached read none.

### What this deployment does NOT claim

- **No `picker-dropped` line was read.** Every picker row the twentieth stored is dropped at its
  table's next request under this code, but no such table existed: the lobby held no table of the
  owner's and none `playing` when this version went live, so the rule is proven in the pool
  (cycle C's Task 4 cells), not observed live.
- **Run 1's watcher socket close is unexplained.** The client's probe is the fact; what closed the
  socket is not, since the tail was disconnected at that moment. One clean 1,795-move re-run is the
  reading beside it, not an explanation.
- **The tail samples.** Its counts over a busy game are of what it carried; the analytics' figures
  (Readings 5 and 7) are the whole.
- **The rows-read figure is one three-page harness game's**, as M6.5c's was; a plain two-person game
  without the harness was not measured.
- **No alarm-driven line is named** in this tail either; `ctx.id.name` inside a production
  `alarm()` is still unread live (as §13).
- **The operator surface was read only by the replay's forces** (204 ×6), not by a listing.

### Rollback

§6. The earlier version is the twentieth's, `6da906a5-a088-442e-bc43-d94904630982`. No migration
was added, so `npx wrangler rollback` can return to it — but that version reads `DEV_DOOR_CLIENT_ID`
from its OWN bindings, where it was a variable, so the dev door on the twentieth works as it did;
what a rollback LOSES is everything §15's first paragraph lists (the doors, the refinish rule, the
memo, `0.6.0`'s saves and refusal), and it drops every open picker this version stored (§6's
bullet). The secret stays put through a rollback; nothing reads it there.

## 16. The exit's window open — the twenty-second deployment, 2026-09-28 (a record, not a plan)

From `ea4f174d`, the same code as the twenty-first. It opens the exit's window: `YOKOHAMA_WEB_COUNT_WAKES`
reads `on`, so every counted wake writes `meta.wakes` and logs `wake {id, wakes}` (§7, "The exit's
window"). **Version `43822b48-da53-48ab-9be9-96f6cb16a2a7`**, deployed at **17:26:40 UTC** between
games (the two-browser game had ended at 17:24:57), run in `worker/`:

```sh
node scripts/deploy-guard.mjs && npx wrangler deploy --var YOKOHAMA_WEB_COUNT_WAKES:on
```

- **The deploy:** the guard PASSED every check (the door line first); 172 assets read, NONE
  re-uploaded (the twenty-first's release); **Worker Startup Time 1 ms**; the bindings list shows
  `env.YOKOHAMA_WEB_COUNT_WAKES ("(hidden)")` beside the nine `vars` and `env.DEV_DOOR_CLIENT_ID`;
  `secret list` after it still names both secrets (a `--var` deploy inherits them). `/api/health`
  200 on the new version.
- **Reading 1 — the exit game** (17:26:55–17:32:13 UTC, headed, `WAKE_PAUSE_MS=60000`,
  `EXIT_SCREEN` 1280x832): **PASSED, 1 passed in 5.3 min.** Table `sim-d67dacbd33f2a287` ("exit test
  mulis5i6"). Its seating passed live: x opened it locked; y read x's lobby line, posted its own and
  joined through the lobby's Open-tables link with the password; both kept; w unlocked it and watched
  with no hand. The three windows a 2×2 grid under a 38 px inset — x 634×391 at (0,38), y at (640,38),
  w at (0,435), each a 618×301 viewport — visible at every checkpoint after opening. **Forty moves in
  311 s**; four long pauses of **60.0 s** after moves 5, 15, 25 and 35, nothing asked meanwhile, each
  followed by the move's `POST …/input` answering in **84, 243, 81 and 75 ms** against an awake median
  of **80 ms** — the wake as the page sees it. Table chat x 10 / y 10, the watcher's panel holding 20
  lines; lobby chat x 1 / y 1; the watcher held to its events with no hand. The abandon: the address
  answers **404** `this table was abandoned on 2026-09-28 — its game is in the archive`, x, y and w
  were told, and the lobby listed it no more. Forgiven: the beacon's pair 2 / 2 / 2, the deal's 409
  on x twice. Every console clean. No `Call log`. 317 s in all.
- **Reading 2 — the wake witness**, off the tail (`tail22.json`: 1,580 events on the twenty-first
  and this version, `ok` 1,571, `canceled` 6, `responseStreamDisconnected` 3, exceptions 0; 205 of
  them the exit table's). **Four `wake` lines, `wakes` 1 → 2 → 3 → 4**, each on the first POST after a
  60 s pause: 17:28:10.454 (CPU 11 ms, wall 33 ms), 17:29:28.098 (12 / 35), 17:30:48.350 (10 / 30)
  and 17:32:05.837 (9 / 32). **The object hibernated in every pause and came back on the next move.**
  A wake costs the object **9–12 ms of CPU**, against a **2 ms** median (the maximum 22) over the 32
  sampled object-side move POSTs; the stateless Worker's own half 1 ms (max 4). §13's exit read 6–9 ms
  a wake at a 2 ms median. The table's other lines: `slot-pushed` 7, `slot-acked` 6, `archive-landed`
  1 (the abandon's save); `trap` 0, `picker-dropped` 0, `object-failed` 0.
- **The quota** (§7), the day's figures: at 16:42 UTC, before any deploy, **12,568** Worker requests
  account-wide (`yokohama-web` 3), 221 rows written, 266 read; at 16:56, after the deploy's readings
  and the F-4 burst, 12,657 / 253 / 491; at 17:03, after both replays, 17,061 (`yokohama-web`
  4,447) / 7,713 / 44,129; at 17:25, after the two-browser runs, **42,640 (`yokohama-web` 29,918) /
  12,283 / 145,365**; at 17:32, after the exit, **43,334 (`yokohama-web` 30,564) / 12,443 / 147,887**.
  The re-run cost about 24,800 requests, **13.8 a move**; the exit about 650 for 40 paced moves,
  **16 a move** with three pages and chat (§13's exit read 35). Both guarded figures stayed under the
  80,000 line all day.

### What this deployment does NOT claim

- **The window was open for six minutes** (17:26:40 to the twenty-third's 17:32:48); the four wakes
  above are the only counted ones, and no other table is known to have been played meanwhile.
- **Everything §15 does not claim.**

### Rollback

§6. The earlier version is the twenty-first's, `ce923774-aa81-4f2b-a8dc-b0fcb5a05f70` — the same
code with the window closed. A `--var` is a binding, so a rollback to it closes the window.

## 17. The exit's window closed — the twenty-third deployment, 2026-09-28 (a record, not a plan)

From `ea4f174d`, the same code. It closes the exit's window: the plain deploy replaces the Worker's
variables with `wrangler.jsonc`'s, which name no `YOKOHAMA_WEB_COUNT_WAKES`, so the switch reads off.
Deployed at **17:32:48 UTC**, 35 s after the exit's abandon. **Version
`36038818-4aa8-4544-961a-0bec2674782c`** — the version the M6.5c2 cycle C close leaves live.

```sh
npm run deploy   # in worker/, the release still built
```

- **The deploy:** the guard's every check passed; 172 assets read, none re-uploaded (the release is
  still the twenty-first's, `yokohama-release-605fe69f17100c46`); **Worker Startup Time 3 ms**;
  `YOKOHAMA_WEB_COUNT_WAKES` is no longer among the bindings, so the window is CLOSED;
  `secret list` names `DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS`.
- **The tail:** `tail22.json`, running since 17:09 UTC, is recorded as stopped at 17:33 UTC.

### What this deployment does NOT claim

- Everything §15 and §16 do not claim, but for the open window.
- **No request was read for the absence of a `wake` line.** The switch's absence from the bindings
  list is the reading that the window is closed.

### Rollback

§6. The earlier version is the twenty-second's, `43822b48-da53-48ab-9be9-96f6cb16a2a7` — the same
code with the window open; rolling back to it re-opens the window. The twenty-first,
`ce923774-aa81-4f2b-a8dc-b0fcb5a05f70`, is the same code with it closed.

---

## 18. M6.5c2 cycle D is live — the twenty-fourth deployment, 2026-09-29 (a record, not a plan)

From `4212f6a0`, cycle D's reviewed head on `web-room-m6.5c2d` (stacked on cycle C over A and B; the
record of record is `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md`, and the spec's dated §10.10
carries the corrections). What cycle D brings to the Worker: **the R2 backoff on the request path**
(R-D6 — a save a game over owes is retried by the alarm or by the first request past the rung, never
by every request); **the held-request arm retired on every host** (R-D5); `table.ts` as a base and four
mixins, the four script dedups and the replay split (R-D7, inside the budget); the seat-email roster
off the wire (R-6.5c2.35); chat delete and moderation as tombstones with a revision counter (R-D4) and
the archive route's 1 MiB cap and check order; the client's defects by screen — the lobby and the open
table as COUNT hosts on the house frame's `chat`/`rev`, a lost-message gap after a reconnect closed;
backup and restore bounded and version-first; **the door's project link**, the first release built
with `YOKOHAMA_PROJECT_URL` (`https://github.com/jmoore2333/yokohama-assistant/`); and, because the
layer's sources changed, **a new picker code digest** — every picker open under the twenty-third is
dropped at its table's next request. **Version `974e0a93-d069-4bf5-8d75-684dd146d3fb`**, deployed at
**15:43:05 UTC** (`Deployed yokohama-web triggers` two seconds before). Every reading holds. The
readings ran over WARP as the owner and, where a claimed identity was needed, through the credential
(§4's header file, mode 600, deleted after).

- **Build, ONCE, for both hosts** (§1): `web/`'s `npm run build` with the link, the RELEASE module —
  `debug_trap` **0**; `wasm-opt version 133` over both browser modules **1,843,940 → 1,215,865 B
  (−34.1%)**; the precache generator's four lines verbatim:

  ```
  precache: 153 entries, 10,036,763 B (release total ceiling 11,900,000 B)
  precache: module 1,215,865 B at /assets/yokohama_wasm_bg-OvdimP7Y.wasm (release module ceiling 1,400,000 B; release 1,400,000, debug 3,200,000)
  precache: module 1,215,865 · bundle 471,772 · fonts 98,728 · shell 11,873 · icons 19,456 · help 764,742 · art 7,454,327
  precache: yokohama-release-d97c124af854a3ff -> dist/sw.js (29,045 B), 131 art files
  ```

  The release name is **`yokohama-release-d97c124af854a3ff`**, and the same `web/dist` went to the
  NAS as `yokohama-web:0.6.0-m6.5c2d` (`deploy/RUNBOOK.md` §3.18). `worker/`'s `npm run build`: the
  module **1,928,529 → 1,275,056 B (−33.9%)** with `wasm-opt` 133 (+16,950 B over §15's 1,258,106,
  the cycle's layer code); `module-figures.mjs`: gzip (Node's zlib, level 9) 498,330 B, 18 pages, 171
  data segments, 123,648 B of data (the section 125,055 B encoded) — transcribed to the root
  `Cargo.toml`'s `release-wasm` comment at the close; `factory-glue: 16 exports (a release module: no
  debug_trap)`; `assemble-assets: 153 precache entries verified — every file present, every SHA-256
  the release's` (156 files, 10,066,176 B); `smoke-release: … GET /api/health 200` (`server_version
  0.6.0`, `archive: "off"` by design); no `gen/.unoptimized`; `assets/sw.js` byte-equal to
  `web/dist/sw.js`.
- **The deploy** (§2), `npm run deploy` in `worker/` at 15:42:53Z, with a tail already listening: the
  guard printed the door first — `wrangler.jsonc deploys the worker door — Cloudflare Access, verified
  by the shell` — and **PASSED every check**. Wrangler (4.136.3) read 172 files, uploaded **6 new or
  modified assets** (`/index.html`, the stylesheet, the Worker chunk, the bundle, `/sw.js`, the module;
  148 already there; 2.00 s), **Total Upload 1408.40 KiB / gzip 534.17 KiB, Worker Startup Time 2 ms**,
  listed the nine `vars` unchanged (`YOKOHAMA_WEB_IDENTITY ("worker")`, the two Access values,
  `YOKOHAMA_DEV_DOOR ("on")`, the archive `r2`, the TTL 90, the three chat values) and deployed the
  trigger `yokohama.moore.nyc (custom domain)`. The secrets were INHERITED, none put: `wrangler secret
  list` names **`DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS`** — names only.
- **Reading 1 — health and the owner** (15:43:38Z, over WARP, a browser User-Agent): `/api/health` 200
  `{"server_version":"0.6.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"cf-access","signal":"ws","archive":"r2","lobby_chat":true,"chat_keep":500,"chat_text":500,"multiplayer":true}`;
  `/api/me` → `{"mode":"cf-access","user":"<the owner's address>"}` (R-6.5c2.16). The dev door before
  the deploy: the credential + `X-Yokohama-Claimed-User: smoke@readme.test` → `smoke@readme.test`.
- **Reading 2 — the served release**: `/sw.js` 200, `text/javascript`, `content-length: 29045`,
  `cache-control: no-store`, naming ONE generation `yokohama-release-d97c124af854a3ff`, byte-equal to
  `web/dist/sw.js`; `/` serves `index-oCCrOYjf.js` and `index-BO2LRdmb.css`; the module 200
  `application/wasm` 1,215,865 B; the bundle carries the project link (grep 1).
- **Reading 3 — `picker-dropped`, the FIRST live reading on any deployment** (I-85, I-91; the close's
  E.4, straw-poll row 110). Before the deploy, through the dev door as `smoke@readme.test`: a throwaway
  table `sim-8bb2ffbfb0b4d0a3` (201 at 15:41:46Z; `{user}` + `{bot: true}`), its deal read, `keep
  {"card": 1, "seat": 0}` 200 (a first try with the card's ID, 14, was the 400 `keep 14 — the dealt
  cards are numbered 1 and 2` — the position, as the walk sends it), `input "1"` (the additional-action
  skip), then `input "1"` again → **the placement picker `place_a` OPEN** (`selected [] · max_areas 3`),
  its row stored under the twenty-third's code. Then the deploy. Then the table's next request, `GET
  …/table` at 15:43:28Z — 200, and the object logged **`picker-dropped {id: sim-8bb2ffbfb0b4d0a3,
  stored: 505d6a43…6d14, running: 8b07f8ad…3af4}`**: `stored` NON-NULL (a row the stamped
  twenty-third wrote), `running` this build's digest of the engine's, the CLI's and the layer's sources.
  Read on the tail (event timestamp 1790696609400) and through the Workers Observability query API
  (the `needle` `picker-dropped`: one event, `$workers.scriptVersion.id` `974e0a93…`, the object
  `82396042…`, the trigger `GET /api/games/sim-8bb2ffbfb0b4d0a3/table`, timestamp 1790696609401).
  **The next click:** a bare `{"line":"A09"}` fell to the options prompt and read 200 `Unrecognized
  input — type 'help'.`; the click a browser sends, `{"line":"A09","expect":"place_a"}`, read **409
  `this game is asking "options", not "place_a" — the prompt moved on (a restart drops an open picker).
  Re-read the game and answer what it asks.`** — the layer's `expect` arm, live. The table then read
  `options` with its one event kept. The throwaway table was deleted (204 at 15:46:10Z; one move had
  been made, so its pair went to the archive under `test/`, and the marker answers 404 `this table was
  abandoned on 2026-09-29 — its game is in the archive`). Since the deploy the same API read
  `restore-refused` 0, `object-failed` 0, `trap` 0, `load-trap` 0, `frame-failed` 0, `identity-failed`
  0, `init-refused` 0, and no outcome other than `ok`.
- **Reading 4 — the query API's shape** (§7's dated note): a shell log line is found by the `needle`,
  its fields under `source`; a total is a `calculations` `count`; an events-view `count` is bounded by
  `limit`.
- **The live corpus replay, twice, sequentially** (§7; the close's F.2), with no reset owed — the walk's
  records did not move in cycle D, so BOTH runs' first sends read **200 with the once-rule's
  `identical: true`**: run 1 (15:51:33Z → 15:54:08Z) and run 2 (15:54:18Z → 15:56:50Z) each forced
  204 ×3 before and after, ran **2,213 of 2,213 rows** (§15's 2,209 + cycle D's four: the chat delete
  and Task 10's two fixtures' rows), compared **90 named rows** (§15's 86), spent 14 exemptions
  (`filterLobby` ×4 with 0 rows dropped; `HOST_FIELDS` ×5 — `health.identity/signal/archive`,
  `me.mode`, `me_signed_out.mode`; `normalizeHouseChat` ×2 — `seq`/`last` taken back by 12 then 13,
  `rev` by 0 then 2; `houseDeleteRow house_chat_delete: /api/chat/1 sent as /api/chat/13` then `/14`;
  `firstSendAccepts` ×2), lobby re-reads 0, **mismatches none, HOLDS ×2**. The house chat's base moved
  12/0 → 13/2 between the runs: each run leaves Red's post as a TOMBSTONE (§7's corrected bullet).
- **The tail, an instrument reading**: the first tail (15:41Z → 15:57Z, 30 events to 15:44Z) carried
  the deploy and the one `picker-dropped`; a FRESH tail was attached at 15:57:15Z for the live
  Playwright tier and read back with a health probe first (2 rows) — it **lost its connection ONCE
  during the game and reconnected** ("attempt 1 of 5"), and carried **1,048 events** to its stop at
  16:07Z: outcomes `ok` 1,041, `responseStreamDisconnected` 3, `canceled` 2, **exceptions 0**;
  entrypoints House 326 · worker 357 · Table 363; the game's sampled rows 685, its Table `/input`
  rows 145 of 1,171 moves (a busy tail is sampled), **`/input` CPU median 3 ms, max 25**; one stream
  upgrade; log lines `slot-pushed` 36 = `slot-acked` 36 and nothing else.
- **The live Playwright tier on this version** (§7; the close's F.3): `cd web &&
  YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link`, 15:57:41Z → 16:03:33Z,
  **`2 passed (5.8m)`**, no `Call log`. **`joins_by_link`**: table `sim-4faa5f85982ef3a3`, 6 moves
  across two browsers in 5 s; the deal's 409 forgiven a 2 / b 0; the beacon's pair a 2 / b 2; the open
  table at 390 px: overflow 0, the chat row 98% of the row. **`two_browsers`**: game
  **`sim-0ca21859ee962979` finished in 1,171 moves, 344 s, 0 idle looks, the largest hand seen 2; the
  watcher held to 837 applied moves, asked `/events` 0 times and opened `/stream` ONCE — 1 at every
  checkpoint**. §15's "run 1's watcher socket close" (row 109) is CLOSED by this clean run with the
  tail attached throughout. **The query API's counts for the two specs' window** (since 15:57:30Z;
  `calculations`): object invocations **Table 15,727 · House 1,159**; outcomes `ok` 32,863 ·
  `canceled` 42 · `responseStreamDisconnected` 14 over every invocation; the game's **Table `/input`
  invocations 1,638** for 1,171 applied moves (the harness's refused and repeated lines among them),
  its **Table `/stream` upgrades 5** (three pages and the reconnects at game over); **the Worker's
  stateless requests naming the game 15,276 ≈ 13.0 a move**.
- **One large replay, timed** (the close's F.4; straw-poll row 117): `POST /api/archive` at 16:06:20Z
  through the dev door as `smoke@readme.test`, the body `{record, table}` as strings — the largest
  finished record in the tree, `crates/yokohama-web/tests/fixtures/browser-played-2p.json` (54,103 B,
  595 events, `engine_version 0.5.0`) with its table document (554 B), 61,778 B on the wire: **201 in
  0.82 s total (0.81 s to the first byte; connect 0.04 s, TLS 0.06 s)** →
  `{"leaf":"over","path":"browser/smoke_at_readme.test/rev7/over/local-242a5429453c593c.json","identical":false}`.
  The cost, off the tail and the query API alike: the Worker's stateless row CPU 4 ms / wall 695 ms;
  the **`Archive` object's row CPU 73 ms / wall 566 ms**, `archive-landed {site: browser, record_key:
  test/browser/smoke_at_readme.test/rev7/over/local-242a5429453c593c.json}` (a `.test` caller, so the
  `test/` root and the 30-day rule). Recorded, never asserted.
- **The quota guard** (§7; the owner's line 80,000 a UTC day for each guarded figure), read through the
  GraphQL analytics, one dataset per call:

  | read at (UTC) | Worker requests, account | `yokohama-web` | rows written | rows read |
  |---|---|---|---|---|
  | 15:39:55 (before the deploy) | 14,466 | 26 | 202 | 706 |
  | 15:50:50 (after the deploy, the picker table, the NAS's game) | 14,548 | 47 | 248 | 1,099 |
  | 16:03:34 (after the two replays and the two live specs) | 34,446 | 19,474 | 10,265 | 106,068 |
  | 16:08:41 (settled; after the timed replay) | 36,821 | 21,057 | 10,702 | 115,814 |

  Both guarded figures stayed far under the line. The two replays and the two live specs together cost
  `yokohama-web` **+21,010 requests, +10,454 rows written, +114,715 rows read**; setting the replays
  aside at §15's per-run costs (~2,190 requests, ~3,750 rows written, ~21,800 rows read), the two
  specs' 1,177 moves read about **13 requests, ~2 rows written and ~52 rows read a move** — §15's
  13.8 / 2.5 / 56, the same order (Option A took the lobby's per-move chat read off the client).

### What this deployment does NOT claim

- **`picker-dropped` was read ONCE, on one table, with a non-null `stored`.** The `stored: null` case —
  a row written before the eighteenth deployment — has still never been observed live; no such row can
  exist any more.
- **The 409 was read from `curl`, not from a browser's click**: the client's `expect` is what a browser
  sends, and the layer's arm answered it; no headed picker was left open across the deploy.
- **The R2 backoff was not exercised live** (no outage): proven in the pool (Task 8), and the request
  path's rung is unread here.
- **The roster's rule was not tried by a live stranger** (a `.test` identity is the walk's own).
- **The tail samples**, and lost its connection once during the game; the query API's figures
  (`calculations`) are the whole.
- **The rows-read figure is one three-page harness game's**, as §15's was.
- **No alarm-driven line is named**; no `wake` counted (the window stays closed); the operator
  surface read only by the replay's forces (204 ×6).
- **The update path was not read**: no browser holding `605fe69f17100c46` was watched taking
  `d97c124af854a3ff` through the door's offer.
- **Every count here that came from the query API was taken over the events since a wall-clock
  instant**, not over a request id set; the two live specs' window also holds the tail's own probes.

### Rollback

§6. The earlier version is the twenty-third's, `36038818-4aa8-4544-961a-0bec2674782c`. No migration
was added, so `npx wrangler rollback` can return to it — and **a rollback to it, or earlier, is the
cycle-D TOMBSTONE rollback of §6's last bullet, and it BITES at once**: the close's two replay runs
left two deleted messages in the live HOUSE chat (§7's corrected bullet), so the twenty-third answers
500 on every lobby chat read and post until the roll forward. What a rollback LOSES besides: the R2
backoff, the roster off the wire, the tombstones' own reading, the client's count hosts and the
door's link; and it drops every picker this version stored (§6's bullet). The secrets stay put.

---

## 19. M6.5c2 cycle E is live, and the door window — the twenty-fifth to twenty-eighth deployments, 2026-09-30 (a record, not a plan)

Cycle E's release on the Worker (the code head `c39da5a1` — the plan `docs/plans/2026-09-29-web-frontend-m6.5c2e-plan.md`, its Task 10.C), deployed with the dev door ON and the worker door, then read as §5 says. The same build serves the NAS (`deploy/RUNBOOK.md` §3.19, the sixteenth). What changed for this host since §18: the shell precached from `/shell.bin` (R-6.5c2.41), served `no-store` by `worker/static/_headers`; the three build values in the bundle; `TableView.has_events` and `saves`; the lobby's three pagers; the waiting-table expiry with its second TTL and both TTLs on `/api/health`; a frame that fetches only what moved; `wrangler.open.jsonc` and `wrangler.gate.jsonc` beside `wrangler.jsonc` under the guard's `--config`.

- **Build, ONCE, for both hosts** (§1): `cd web && YOKOHAMA_PROJECT_URL=… YOKOHAMA_SITE_NOTICE='Not affiliated with Asmodee' YOKOHAMA_SITE_BLURB='…' npm run build` at 11:32Z — `debug_trap` 0; `wasm-opt 133`: **1,844,514 → 1,216,429 B (−34.1%)**; the precache's four lines: **153 entries, 10,044,462 B**, the module 1,216,429 B at `/assets/yokohama_wasm_bg-Cg7pVPYL.wasm`, `module 1,216,429 · bundle 478,907 · fonts 98,728 · shell 11,873 · icons 19,456 · help 764,742 · art 7,454,327`, **the release `yokohama-release-50ee8137624586b2`**, `sw.js` 29,296 B, 131 art files; `dist/shell.bin` 2,387 B, byte-equal to `dist/index.html`; the blurb, the notice and the link each once in the bundle; the browser module's `gzip -9` **472,169 B**. `cd worker && npm run build`: the module **1,931,148 → 1,276,830 B (−33.9%)**; `factory-glue: 16 exports`; `assemble-assets: 153 precache entries verified` (157 files, 10,076,558 B; `_headers`, `_redirects`, `index.html` and `sw.js` are not in the list — `shell.bin` is); `smoke-release … 200`; `module-figures.mjs`: `{bytes 1,276,830, gzip9 499,614, pages 18, segments 172, dataBytes 123,748, dataSection 125,164}` (the root `Cargo.toml`'s `release-wasm` comment carries both modules' figures).
- **The quota guard before the deploy** (§7; 11:33Z): 0 Worker requests, 0 rows written, 0 rows read for the UTC day (the figures lag by minutes; the day was fresh).
- **The deploy** (§2), `npm run deploy` in `worker/` at 11:34:24Z with a fresh tail listening (`wrangler tail --format json`, its `.err` read for "Reconnecting": 0): the guard's door line `wrangler.jsonc deploys the worker door — Cloudflare Access, verified by the shell`, every check passed, among them the two new: the open and gate files equal production but for their door; **7 assets uploaded** — `/index.html`, `/sw.js`, the module, **`/shell.bin`**, the stylesheet, the Worker chunk, the bundle (148 already there) — 1,414.57 KiB / gzip 536.67 KiB in 2.11 s; startup 2 ms; the bindings the three objects, the bucket, the assets, and **TEN plain-text vars**: the nine of §18 plus `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS ("7")`; both secrets inherited (`wrangler secret list` names `DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS`); **version `e0ac38e7-d50e-4041-95b3-e0b948fde980`**, 10 s wall.
- **Reading 1 — health and the owner** (11:34:56Z, over WARP, a browser User-Agent): `/api/health` 200 `{"server_version":"0.6.0","semantics_rev":7,"rules_hash":"d85445c479cb1ead","identity":"cf-access","signal":"ws","archive":"r2","lobby_chat":true,"chat_keep":500,"chat_text":500,"multiplayer":true,"table_ttl_days":90,"open_table_ttl_days":7}` — the two TTLs new; `/api/me` names the owner.
- **Reading 2 — the served release and the shell asset**: `/sw.js` names `yokohama-release-50ee8137624586b2`; **`/shell.bin` 200, `content-type: application/octet-stream`, `cache-control: no-store`, 2,387 B, sha256 `0c403d5c6de594c3b184ed8a0832dfee07fe7fc67bc108d4f10e7697f3b5900e` — equal to the inventory's digest — read twice a minute apart, `cf-cache-status: HIT` both times.** On Workers Assets a HIT is the platform's content-addressed asset cache serving the NEW release's bytes, as the digest shows a minute after the deploy; it is not the zone cache the NAS host sits behind, whose reading is `deploy/RUNBOOK.md` §3.19's (`BYPASS`). `/index.html` 200, 3,325 B against the release's 2,387: the edge's JavaScript Detections snippet, unchanged by design; `cf-cache-status: MISS`.
- **Reading 3 — THE UPDATE READING (Review Focus 1; R-6.5c2.41's proof; the plan's 10.D):** a headless persistent Chromium context over WARP, prepared BEFORE the deploy at D's release — `/` opened until "Ready to play offline", the worker `activated`, one cache `yokohama-release-d97c124af854a3ff`, then its `cf_clearance` cookie CLEARED (the state that failed on 2026-09-29: only `CF_Authorization` left). After the deploy, ONE plain reload at 11:34:54Z: the page's `/sw.js` re-fetched (no-store); the new worker's install fetched **`/shell.bin`** (200, `application/octet-stream`, no-store) and **no `/index.html`**; the new worker reached `installed` (waiting) beside D's `activated`; **two caches, `yokohama-release-d97c124af854a3ff` retained and `yokohama-release-50ee8137624586b2` new**; the door read "Ready to play offline" and **"A new version is ready. Reload to use it"**. The click at 11:37:13Z: the new worker `activated`, the controller E's, the door "Ready to play offline" with nothing saying "Try again", the navigation answered by the worker as `text/html; charset=utf-8` from the cached shell bytes. The instrument and its snapshots: the plan's ledger (`pinned-browsers.mjs`, `pinned/worker/{pinned,after-1,after-2}.json`).
- **The live corpus replay, twice, sequentially** (§7): run 1 at 11:39:49Z (138 s) and run 2 at 11:42:23Z (136 s), each **HOLDS** — the forces 3×204 before and after; 2,213 rows run, 90 named rows compared, 16 exemptions spent, among them the two new `HOST_FIELDS` **`health.table_ttl_days: null → 90` and `health.open_table_ttl_days: null → 7`**, `normalizeHouseChat` (the house chat taken back by 14 then 15, rev by 4 then 6), `houseDeleteRow` (`/api/chat/1` sent as `/api/chat/15`), and `firstSendAccepts` on both archive posts (200, "identical": true); mismatches none; no reset owed (Task 5's `has_events` in the walk's stored tables changed no committed record).
- **The live Playwright tier on this version** (§7; 11:45:02–11:53:40Z, 8.6 min): **2 passed** — `joins_by_link` (a table of 6 moves, 5.1 s; the deal's designed 409 and the beacon's CORS error forgiven, counted) and **`two_browsers`: game `sim-bd144e78340da0df` finished in 2,034 moves, 511 s, 0 idle looks; the watcher held to 1,465 applied moves, asked `/events` 0 times and opened `/stream` once.** The Workers Observability query API over the window (every ad-hoc query needs a `queryId`; `dry: true`; `$workers.durableObjectId is_null` splits the Worker's stateless requests from the objects' invocations; `$workers.entrypoint` names the object class): `yokohama-web` 36,003 invocations — the stateless requests 17,818 (200 ×17,780, 201 ×7, 204 ×3, 403 ×1, 409 ×1), the objects Table 16,230 · House 1,955; **the game's own stateless requests 16,082 = 7.9 per move (§18's reading was ~13)**; the needles `archive-landed` 2, `picker-dropped` 0, `object-failed` 0; the two `exception` outcomes read belonged to another service. Rows per move for the tier are NOT separable: the periodic rollup lags the window by hours, and the day's totals fold the four replays and the window in (the day's figures below).
- **The screenshots** (the plan's 10.G; 11:54:24Z, 9.3 s): `YOKOHAMA_WEB_README_SHOTS=1 YOKOHAMA_WEB_E2E_TARGET=live npx playwright test readme_shots` wrote the ten pictures into `worker/docs/img/` (the door at 1280 and at 390, the lobby, the open table, the join, the deal, the game as a player and as a watcher, the game on a phone, the 404 marker), each LOOKED AT; the lobby picture had masked the spec's own row (its mask was by text) — fixed at `4c39f603` (masked by table id) and re-shot. The spec's tables are abandoned in its `afterEach`.
- **The phones** (R-6.5c2.33; the plan's 10.H; the iPhone 17 Pro on iOS 26.4 and the iPad Pro 13-inch on iOS 27.0 through the mobile bridge, the Android emulator `Medium_Phone`): the socket WAKES after 60 s on the Home screen on all three (the bot's turn walked on return); a lock is untestable on the iOS simulators (the bridge has no lock press) and correct on Android after 58 s; NO keep-storage prompt on any (the door says the browser has not promised to keep games); the local game stays listed after a reload; **an iOS Home Screen app opens with its OWN empty storage** (iOS partitions it from Safari's); the Android emulator refuses the PWA install (Play services), and its returning Chrome was served its OLDER release until "Reload to use it" was tapped — the update path on Android Chrome too. Owner's-eye readings recorded, not fixed: the game header takes about 45% of the iPhone's portrait height; the iPad's tiles do not grow to the room; on Android the chat input sits under the keyboard and landscape leaves about 100 px of board.
- **The quota guard after day 1 and the window** (§7, GraphQL at 12:37Z; the `date` and the `datetime_geq/leq` forms agree): the UTC day — `yokohama-web` **29,211 requests** (the account 42,139), **rows read 205,709, rows written 20,839** — day 1's tier, the four live replays, the phones, the plain game and the window, all in; the day before (D's) 116,137 read / 10,806 written. Under the owner's line of 80,000.

### The door window (R-6.5c2.39, .43, .44; run on day 1 by the plan's ruling 10-a, 12:12–12:37Z)

Every step is the orchestrator's, through the Cloudflare API (the MCP) and `wrangler`; no secret value appears here; the PUT bodies carry the application's read-back fields with only the hostname lists changed.

- **The rate-limit rule (10.J; the zone's ONE rule, kept by its id; `ends_with(...)` is a FUNCTION in the rules language — the infix spelling is refused):** the expression now `(http.request.uri.path eq "/") or (http.host eq "yokohama.moore.nyc" and (ends_with(http.request.uri.path, "/join") or ends_with(http.request.uri.path, "/unlock") or http.request.uri.path eq "/api/archive"))`, action block, **20 requests per 10 s** (the live replay's archive rows peak at 8 in a window; doubled, then to the Free plan's minimum period); the burst — 25 wrong-password joins from the workstation through the dev door in 2.74 s, and again from a clean context under state 1 in 2.22 s — reads **20 × 403 then 5 × 429 (error 1015)** both times, 403 again after 11 s; the live replay under the rule HOLDS; **the rule STAYS.**
- **State 1 — no door but the name step (12:18–12:24Z):** the operator secret DELETED through the API (`DELETE …/scripts/yokohama-web/secrets/YOKOHAMA_WEB_ADMINS`; the `open` shell refuses to start beside it); `npm run deploy:open` (the guard's door line `wrangler.open.jsonc deploys the open door — no identity service…`; version `7b12ab22`); the Access application's `self_hosted_domains`/`destinations` PUT to `yokohama-nas.moore.nyc` + `yokohama-cli.moore.nyc` (the `aud` unchanged). Read from clean contexts: `/` 200 with the door and the blurb, `/api/health` `identity: open`, the name step, `GET /api/admin/tables` → the layer's 404 for any typed name — the owner's own address typed as a name INCLUDED (the open door's trust model: the name IS the person, and there is no operator); the burst from a clean context blocked at 20; a plain two-person game (`web/e2e/state1_game.spec.ts`, opt-in) of 17 events, its archive landing under the bucket's `test/` prefix (`archive-landed` 1); its stateless requests 147 = 8.6 per event with the setup in.
- **State 2 — the door on the API paths alone (12:24–12:31Z):** `npm run deploy:gate` FIRST (`wrangler.gate.jsonc deploys the worker door…`, `YOKOHAMA_DEV_DOOR ("off")`; version `d95668f0`), THEN the secret put back from `git config user.email` through stdin; the PUT with the two hostnames + the NINE paths (`yokohama.moore.nyc/api/admin`, `/api/archive`, `/api/art`, `/api/chat`, `/api/games`, `/api/labels`, `/api/me`, `/api/stream`, `/enter`). Read over WARP from a clean context: `/` 200 with zero `/api/` calls; **a local bot game of five acts with ZERO Worker invocations** (the query API); the owner's "Go to the lobby" → `/enter` → the lobby with no login page, a table created, both sockets opened, `/api/me` naming the owner, the operator listing 200. Read from a HEADED browser WITHOUT WARP: `/` 200 and `/shell.bin` 200 (public), an `X-Requested-With` fetch of `/api/me` and `/api/games` → **401 `WWW-Authenticate: Cloudflare-Access resource_metadata="https://yokohama.moore.nyc/.well-known/cloudflare-access-protected-resource/api/me"`**, a navigation to `/enter`, `/api/games` or `/api/admin/tables` → Access's login page. (curl from a bare client meets Bot Fight Mode's 403 before Access; a browser is the instrument.)
- **The revert (12:31:37–12:32:32Z):** `npm run deploy` (`wrangler.jsonc deploys the worker door…`, `YOKOHAMA_DEV_DOOR ("on")`; version `eb8c7839`, the twenty-eighth deployment — 26 and 27 were the window's); the PUT back to the three hostnames (`yokohama`, `yokohama-cli`, `yokohama-nas`), the `aud` unchanged; `wrangler secret list` names both secrets. After-check: `/api/me` over WARP names the owner; **a clean headed browser without WARP at `/` meets Access's login** (12:43Z; the door not drawn); the credential + a claimed header names its caller; a claimed header WITHOUT the credential names the owner, not the claim; the live replay ONCE HOLDS; the quota above.
- **The three PUT bodies** (`PUT /accounts/{account}/access/apps/11cbcf68-96cd-4310-baee-f2ade812155d`; the MCP or `curl` with an API token that manages Access) carry the application's read-back fields unchanged — `name: yokohama`, `type: self_hosted`, `session_duration: 730h`, `app_launcher_visible: true`, `allowed_idps` (the one identity provider's id), `auto_redirect_to_identity: true`, `http_only_cookie_attribute: true`, `policies` as `[{id, precedence}]` for the three (`sbaitso-friendly` 1, `yokohama-players` 2, `yokohama ST bypass` 3) — and differ ONLY in `domain`, `self_hosted_domains` and `destinations` (`[{type: "public", uri}]`, one per entry): the REVERT lists the three hostnames with `domain: yokohama.moore.nyc`; STATE 1 lists `yokohama-nas.moore.nyc` and `yokohama-cli.moore.nyc` with `domain: yokohama-nas.moore.nyc`; STATE 2 adds the nine `yokohama.moore.nyc/<path>` entries to state 1's list. Read the application back after every PUT: the `aud` must not change (it did not, three times), and a changed `aud` would orphan every `ACCESS_AUD` the Workers carry.
- **Forward note (2026-10-20, security S3's Task 7, §26):** the application's lists carry FOUR hostnames from this day — `yokohama-test.moore.nyc` beside the three — and a PUT replaces the whole list, so every body above, and every revert, keeps the test host.
- **The flip RECIPE (what a later operator runs, in this order):** to state 2 — `npm run deploy:gate`, then the Access application's destinations PUT with the nine paths (the aud never changes); back — `npm run deploy` (the dev door on), then the PUT with the three hostnames, then the after-check above. To state 1 — the secret deleted FIRST, then `npm run deploy:open`, then the PUT without `yokohama.moore.nyc`; back through state 2's order (the gate deploy BEFORE the secret put — the open shell would 500 beside it). Any deploy restarts every object and closes every socket: flip between games. *(Forward note, 2026-10-12: `deploy:gate` was retired in security S2b; this recipe reads `npm run deploy` for it, around `npm run deploy:dev` — production's own file ships the dev door off since S2b, so state 2's deploy and the revert's are both the plain `npm run deploy`, and the after-check's dev-door readings and live replay run between an `npm run deploy:dev` and a closing `npm run deploy`, §7. Since S2b this recipe runs on THIS Worker inside the review's known exercises, and after the security process only by an owner decision, never as an operating step — the owner's clarification (2026-10-12) of "the door of the private Worker never flips" (§3). Corrected 2026-10-17, the S2 stack review: this sentence said the recipe waited on an amendment of R-SEC6.)*

### What this deployment does NOT claim
- The live half of RF1 (the update path) on each host was read ONCE, in a headless persistent context on the workstation — not on the owner's Edge, whose pinned state is the one that failed on 2026-09-29; the owner's own browser is the last reading.
- `cf-cache-status: HIT` on `/shell.bin` from Workers Assets is INTERPRETED (content-addressed) from one deploy's read, not measured across a second deploy of a changed shell.
- Row 108's single-audience reading was taken as the owner's WARP identity, not as a second signed-in address; row 119's refusal of an UNLISTED signed-in person was not read (it needs a second identity only the owner can create).
- Rows written and read per move for the tier were not separable from the day's totals (the rollup lag); §18's ~2 / ~52 stand as the last per-move reading.
- A lock (as against the Home screen) is unread on iOS; the plain game's spec printed a wrong applied count on its live run (fixed after; the figures here are from the archived record's 17 events).

### Rollback
- `wrangler rollback` to `974e0a93…` (§18) restores D's release: **an older Worker version renders a `waited` marker's head as `undefined — …`** for up to 90 days (Task 6's marker reason is unknown to it); a table document with `has_events` reads as before (the older module ignores the key); the chat tombstone caveat of §6 stands; open pickers drop (`picker-dropped`).

---

## 20. M6.5c2 cycle F is live, and the public window — the twenty-ninth to thirty-first deployments, 2026-10-01 (a record, not a plan)

Cycle F's release on the Worker (the code head `a954ca79`; the plan `docs/plans/2026-09-30-web-frontend-m6.5c2f-plan.md`, its Tasks 11.A to 11.G), deployed with the dev door ON, the worker door and `YOKOHAMA_TURNSTILE` `off`, then read as §5 says; then the public window, which put the no-Access door in front of the Internet for 43 minutes and took it away. The NAS's is `deploy/RUNBOOK.md` §3.20, its seventeenth deployment, built from the same code with its own client build. What changed for this host since §19: the web presence (`robots.txt`, `llms.txt`, `sitemap.xml`) and `404.html`, with `not_found_handling` `404-page`; the build stamp `build-values.json` and the deploy guard's checks 10 to 12, among them the tree's `404.html` (11) and the Turnstile declaration against the stamp (12); the door's own update, the release notice and the resume rule in the client and the layer; and the edge rule and the Turnstile widget the window used, both OFF in production.

- **Build, ONCE for this host** (§1; 12:30:55–12:31:23Z): `cd web && npm run build` with the five build values — the site URL `https://yokohama.moore.nyc`, the release feed DERIVED from the GitHub project link, the project link, the notice, the 226-character blurb — and NO Turnstile key; rc 0 in 8 s; `debug_trap` 0 in the shipped module; `wasm-opt 133`: **1,859,615 → 1,225,791 B (−34.1%)**; the precache's lines: **153 entries, 10,065,402 B**, the module at `/assets/yokohama_wasm_bg-9hpjLwYr.wasm`, `module 1,225,791 · bundle 488,298 · fonts 98,728 · shell 14,060 · icons 19,456 · help 764,742 · art 7,454,327`, **the release `yokohama-release-1088dec85fe623b8`**, `sw.js` 29,296 B, 131 art files; the presence script wrote `robots.txt`, `llms.txt` and `sitemap.xml`; **the stamp** `{project_url, blurb, notice, release_feed, site_url: true; turnstile_sitekey: false}`; `dist/shell.bin` and `index.html` both 4,574 B and equal; the blurb, the notice and the link each once in the bundle. `cd worker && npm run build` (12 s): the module **1,946,252 → 1,286,127 B (−33.9%)**; `factory-glue: 16 exports`; `assemble-assets: 153 precache entries verified` (162 files, 10,103,767 B; not in the list: `404.html`, `_headers`, `_redirects`, `build-values.json`, `index.html`, `llms.txt`, `robots.txt`, `sitemap.xml`, `sw.js`); `smoke-release … 200` (`archive: off` in the in-process smoke, the configuration's shape).
- **The quota guard before anything** (§7; about 12:10Z, GraphQL through the Cloudflare MCP, one dataset per call): Worker requests account-wide **10,536 of the 80,000 line**, Durable Object rows written 176, read 180; the yokohama worker's own share was not among the named totals, so no request had reached it that day under Access. No HOLD. The figures lag by minutes.
- **The pre-flight account steps** (R-F7, each reversible, the orchestrator's): the Turnstile widget `moore.nyc (Spin)`, mode `managed`, domains `127.0.0.1`, `localhost` and `moore.nyc`, was `PUT` through the API at 12:07:20Z from pre-clearance `no_clearance` to **`managed`** and read back (`bot_fight_mode` false, unchanged); the widget's site key was never printed. The pinned browsers for the update reading were started BEFORE any deploy, over WARP, each opening the door at E's release until "Ready to play offline", the Worker's with a second tab holding a local game.
- **The zone's settings, read** (2026-09-30 and 10-01): **Challenge Passage ONE WEEK — the owner's, R-OWN-F3, set 2026-10-01** (`challenge_ttl` 1800 → 604800 at 04:26:42Z, read back; the dashboard's Security → Settings → Challenge Passage; the setting takes 300 seconds up to 31,536,000, defaults to 1800, and the API schema recommends 15 to 45 minutes and honours more on a best-effort basis); the owner's reason was that the widget "should be invisible to the user". **Bot Fight Mode ON**, kept on through the cycle. **Managed `robots.txt` OFF**, which a reading below confirms: the served `robots.txt` is the release's own.
- **The twenty-ninth deployment** (`npm run deploy` in `worker/`, 12:32:18–12:32:28Z, a fresh tail listening): the guard green — its door line `wrangler.jsonc deploys the worker door — Cloudflare Access, verified by the shell`, checks 10, 11 and 12 among them — the assets uploaded, `vars` with **`YOKOHAMA_TURNSTILE ("off")`**, the two TTLs 90 and 7, the dev door on; both secrets inherited (`DEV_DOOR_CLIENT_ID`, `YOKOHAMA_WEB_ADMINS`); **version `e60d62b7-fc70-42f2-9d68-d45f9711d05a`**, 10 s. `go` was touched for the Worker's pinned door at 12:33Z.
- **§5 readings at F's release** (12:33:10Z, over WARP): `/api/health` `0.6.0`, `semantics_rev` 7, `cf-access`, `ws`, `r2`, the TTLs 90 and 7; `/api/me` names the owner; `/sw.js` names `yokohama-release-1088dec85fe623b8`, equal to the build's; **`/shell.bin` 200 `application/octet-stream`, `cache-control: no-store`, `cf-cache-status: HIT` on both reads a minute apart** (the asset layer's own label, §19; the bytes are F's: the digest equals the inventory's, 4,574 B); `/index.html` 200, 5,512 B against the build's 4,574, the edge's snippet; `/` 200. **The presence files:** `/robots.txt` 200 `text/plain`, `max-age=3600`, 125 B, identical to the build's; `/llms.txt` 200, 979 B; **`/sitemap.xml` 200 `application/xml`, 162 B** (this host's type); `/build-values.json` 200 `application/json`, `max-age=0, must-revalidate` (the platform's default for an unlisted file); `/404.html` 200 `text/html`, 3,470 B, with `noindex`. **A navigation-shaped `/wp-login.php`** (`Sec-Fetch-Mode: navigate`, `Accept: text/html`, over WARP, so past Access) **→ 404 `text/html`, "Nothing here"**: the asset layer's 404 page. All of these were read over WARP; the window's W-1 below says why that matters.
- **THE UPDATE READING at the deploy** (the plan's 11.D; the pinned door, checks at 12:36:04Z): the pin had not taken a release before `go`; the deployed release (`1088…`) differed from the pinned (`50ee…`); the install fetched `/sw.js` then `/shell.bin` and no HTML; both generations stood; the game tab did not reload and its game was unchanged. **The checks of F's own code failed by construction, not by defect:** the pinned door's PAGE was E's bundle, served from E's cache, and the auto-apply and the game's update line are F's code, which cannot run until F's worker serves the page — after the FIRST apply. So the step from E to F read E's behaviour (the line; the click), and the no-click reading belongs to a step between two F releases: the window's deploy and its revert. E's click at 12:40Z applied F; the pin closed with `done` at 12:41Z: **15 checks, 10 held, 5 failed, 0 not run**, the five being the F-code expectations.
- **The live corpus replay twice, and the live tier** (§7; 12:50:25–13:03:36Z, over WARP, the dev door): the replay **HOLDS twice** (rc 0, mismatches none, lobby re-reads 0); a fresh tail read 96,938 rows with 0 "Reconnecting"; the live tier **passed in 8.0 minutes**: `joins_by_link` 6 moves across two browsers in 6 s, and `two_browsers` `sim-7e337d9f…` finished in **1,741 moves**, 470 s, the watcher held to 1,334 applied moves, asking `/events` 0 times and opening `/stream` once. What `joins_by_link` forgave in its two browsers, counted: the deal's 409 (2 and 0), the beacon's CORS error (2 and 2) and **the release feed's failure (2 and 0)** — the live contexts carry the claimed-user header on every request and GitHub's preflight refuses it, so a real browser sees the plain 404 where the harness sees a CORS block. **The tally** (`tally-report.mjs --moves 1741`): 12,938 requests, all `/api` 12,715 = **7.30 a move** (E's 7.9), peak 338 per 10 s; by route `/at` 2.30, `/log` 2.30, the state 1.53, `/input` 1.13, the rest at or under 0.01; not `/api` 223 (0.13); the oracle paths 1, the watcher's unlock.

### The public window (R-F7; F spec §2.6; 13:13:51–13:57:31Z, 43 minutes 40 seconds)

Every step is the orchestrator's, through the Cloudflare API (the MCP) and `wrangler`, each a dated line in the cycle's ledger. **The hostname left Access with the edge rule in front of it and the Worker open** — Turnstile on, no operator surface — and read as the Internet would meet it, then everything went back in the reverse order. No secret and no site key appears here.

- **The slot (12:52Z).** The zone's `http_request_firewall_custom` phase held FIVE rules, the Free plan's maximum, all `skip`. Three of them (the Applebot allowances) shared four user-agent clauses and differed in nothing but one host-scoped clause each, with identical actions, parameters and logging; so, behaviour-preserving: one was patched to carry another's host clause and that other was deleted, each recorded verbatim first (the ruleset's version 12 → 14). Four rules remained, one slot free, and **the edge rule is created at POSITION FIRST**, because a matching `skip` skips the custom rules after it, and a scanner sending that user agent would otherwise pass a rule placed after it.
- **Before** (13:04:55Z): Worker requests account-wide 35,137 of 80,000 (the yokohama worker 20,472 that day, the live tiers' cost), rows written 11,920, read 142,753; no HOLD. **The instrument's query:** the Workers Observability API (`POST /accounts/{id}/workers/observability/telemetry/query`, `view: "calculations"`, dataset `cloudflare-workers`, the filter `$metadata.service eq yokohama-web`, `calculations: [{key: "", operator: "count"}]`, `chartType: "aggregate"`) answered the live tier's window 12:56–13:04Z with **31,253**; the figure is `result.calculations[0].aggregates[0].count`, and every minute below is read with it.
- **The keyed build** (G.2): wrangler's OAuth token has no `challenge-widgets` scope, so the site key was read ONCE through the Cloudflare MCP and put in a mode-0600 scratch file the build command sourced, **removed at the revert** (a Turnstile site key is public by Cloudflare's design, and no document spells it). `npm run build` rc 0; the stamp **all six true**; **the release `yokohama-release-ce7bcc4e2b98b94e`** (the keyless build's `1088…`: the key is in the bundle once); the worker build rc 0; `node scripts/edge-rule.mjs` from the keyed tree **byte-identical** to the production tree's, since the key adds no file.
- **The thirtieth deployment, the open deploy** (G.3; 13:11:02–13:11:31Z). **Step 1, the operator secret deleted FIRST** (`DELETE …/workers/scripts/yokohama-web/secrets/YOKOHAMA_WEB_ADMINS`; the `open` shell refuses to start beside it); **step 2, `npm run deploy:open`**: the guard's door line `wrangler.open.jsonc deploys the open door — no identity service: anyone who reaches the Worker plays under any name`, `YOKOHAMA_WEB_IDENTITY ("open")`, `YOKOHAMA_WEB_ARCHIVE ("r2")`, the TTLs, the chat three, **`YOKOHAMA_TURNSTILE ("on")`**, no dev door and no Access pair; **version `4088d3a7-e4e5-4eac-b205-a3a6a744cc6b`**. `/api/health` three times over WARP read `identity "open"`, `0.6.0`, the TTLs 90 and 7, with no propagation lag. **The smoke's live mode FAILED here, as it must:** `wrangler.open.jsonc declares YOKOHAMA_TURNSTILE on, but a cookie-less GET …/api/health answered 200 with no cf-mitigated header — the edge rule … is not in front` — for the rule's absence, since the shell was on WARP and Access let it through.
- **The edge rule** (G.4; 13:13:09Z): `POST /zones/{zone}/rulesets/27fcc0c7398f42e3abbf549a917e5fa7/rules` with the JSON `node scripts/edge-rule.mjs` printed and `position {index: 1}`; the ruleset's version 14 → 15; **the rule id is `dc4c5d3e986144649617cbbaea58cd8a`**, action `managed_challenge`, description `yokohama — the no-Access door's edge check (F)`, enabled. Its expression, 504 characters, is the host and NOT a path in the ten top-level names (`/`, `/404.html`, `/build-values.json`, `/index.html`, `/llms.txt`, `/manifest.webmanifest`, `/robots.txt`, `/shell.bin`, `/sitemap.xml`, `/sw.js`) or under the six prefixes `/art/`, `/assets/`, `/fonts/`, `/help/`, `/icons/` and `/cdn-cgi/`. The phase's order: the rule first, then the four skips, five of five. `node scripts/edge-rule.mjs --verify` over the zone's own answer: **rc 0**, "the zone's rule equals the tree's — expression, action and description", and the tree it read, `sw.js` naming `yokohama-release-ce7bcc4e2b98b94e` with `site_url` true.
- **Access OFF** (G.5; 13:13:51Z): `PUT /accounts/{account}/access/apps/11cbcf68-96cd-4310-baee-f2ade812155d` with the application's read-back fields, the primary `domain` moved to `yokohama-nas.moore.nyc`, `self_hosted_domains` and `destinations` listing `yokohama-nas.moore.nyc` and `yokohama-cli.moore.nyc`, the three policies by `{id, precedence}`; read back: the two hostnames, **the `aud` UNCHANGED**, the policies `sbaitso-friendly` 1, `yokohama-players` 2 and `yokohama ST bypass` 3, 730 hours. **The smoke's live mode over WARP PASSED** (13:14:33Z): `/api/health` 403 `cf-mitigated: challenge` — "the edge rule … stands in front" — and `/sw.js` and `/sitemap.xml` 200 with no `cf-mitigated` header.
- **The curl shapes, off WARP** (13:17:06Z): `GET /enter`, `GET /api/health`, `POST /api/archive`, `HEAD` and `OPTIONS /api/health`, the no-cors `/api/health`, the navigate-shaped `/wp-login.php` and `/.env` — **every one 403 `cf-mitigated: challenge`**, `text/html`; the two prefix misses `/assets/zz0000.js` and `/art/zz0000.png` → **404 `text/html` with NO `cf-mitigated`**; `/robots.txt` and `/sw.js` 200.
- **The probe, default mode** (`worker/scripts/probe-burst.mjs`; 13:18:01–13:19:41Z, 200 rows, the five scanner paths `/.env`, `/wp-login.php`, `/admin`, `/xmlrpc.php` and a random `/<hex>.json`): 67 bare, 67 navigate and 66 no-cors, **all 200 → 403 `cf-mitigated: challenge`**; the minute's Observability count: **0**.
- **The probe, `--under-prefix`** (13:20:00–13:21:40Z, 200 rows, `/assets/<hex>.js` and `/art/<hex>.png`): **all 200 → 404**, `text/html`, 2,899 bytes, `cf-cache-status: HIT`, no `cf-mitigated`; the count: **0 — not 200.** The design expected the Worker's own invocation for a miss that is not a navigation (spec §2.5); **the reading is wider than the question: under the array form of `run_worker_first`, a miss under an allowed prefix is answered by the ASSET LAYER's cached 404 page in EVERY request shape, with no Worker invocation** (the ruling D-1). The residual the rule lets through costs nothing on the quota. The whole open window to this point, 13:13:50–13:22:54Z: **0 invocations**.
- **W-1 — Bot Fight Mode answers a BARE client first, on the allowed paths that are not static resources.** Off WARP, curl's `/`, `/index.html`, `/404.html`, `/build-values.json`, `/manifest.webmanifest`, `/shell.bin`, `/sitemap.xml` and `/fonts/fraunces` read 403 `cf-mitigated: challenge`, and `/llms.txt`, `/robots.txt`, `/sw.js` and an icon read 200 `HIT`; a browser User-Agent, the navigate headers and a fake Googlebot change nothing (the TLS fingerprint is the tell). **The rule exempts every one of them**; the zone's firewall events name the source. **So the smoke's live mode, whose positive control is that `/sitemap.xml` reads 200 cookie-less, holds only for a caller Bot Fight Mode trusts:** over WARP it passed at 13:14:33Z, and off WARP it fails on the sitemap row (rc 1, 13:23:13Z, "answered 403 with cf-mitigated: challenge"). Browsers pass Bot Fight Mode's JavaScript detection silently and verified crawlers are exempt, so the door and the presence stand for their audiences; the finding is about the instrument's premise. Every earlier presence reading in this section was over WARP.
- **The control** (13:25:51Z, a fresh Chromium with no cookies, off WARP): `GET /api/health` as a navigation met the interstitial "Just a moment..." and cleared **non-interactively in about 20 s** to the health JSON (`identity open`, the TTLs 90 and 7); the count for 13:25:30–13:27:00 was **2**, both `fetch GET /api/health ok` at 13:25:53 — the challenge platform's re-request after the solve. **The count counts.** The firewall events named the rule's `managed_challenge` ×2, `managed_challenge_non_interactive_solved` ×1 and `managed_challenge_bypassed` ×1 on that path.
- **Attribution** (the zone's `firewallEventsAdaptive`, 13:13–13:27Z, 232 events): `botFight managed_challenge bot_fight_mode` on the five scanner paths ×198 and on `/sitemap.xml`, `/`, `/build-values.json` and the rest of the curl shapes; **`firewallCustom managed_challenge`, rule `dc4c5d3e…`, ONLY on the browser's `/api/health` and `/favicon.ico`.** For a bare client from a residential address Bot Fight Mode acts first and the rule never gets to answer; the default burst's pass reads "every scanner row 403 and zero invocations, by the edge as a whole", and **the rule's ISOLATED proof is the over-WARP burst below.** Bot Fight Mode stayed on: nothing was switched off for a cleaner reading.
- **The browsers** (G.c to G.f; Chromium with its `cf_clearance` cleared):
  - *The door's button* (13:36:31Z): "Go to the lobby" loaded Turnstile's `api.js`, ran the widget in managed mode, and `POST /cdn-cgi/challenge-platform/h/b/c/<ray>` returned 200, **setting the pre-clearance cookie on this zone BEFORE the lobby's first metered call**; then the lobby's eight calls, among them `/api/me`, `/api/health`, `/api/chat`, `/api/games` and `/api/labels`, every one 200 — no 403 and no overlay. Console: 0 errors.
  - *A table and the socket* (13:37–13:38Z): "New table", "Deal" → `POST /api/games` 201, `sim-5fbc19824a241fad`; on a reload `wss://…/api/games/<id>/stream` **opened under the clearance**, the upgrade accepted.
  - *A link arrival* (13:38:46Z, a fresh context, straight to `#/t/<id>`): `GET /api/me` and `GET /api/health` **403 `cf-mitigated: challenge`** → the widget → `POST /cdn-cgi/challenge-platform/…` 200 → both reads 200: **1.3 s end to end**, the visitor on the lobby's name step. Console: the browser's two "Failed to load resource: 403" lines and Turnstile's own lines; one of ours to note, `OTS parsing error: Size of decompressed WOFF 2.0 is less than compressed size`, a font file the browser's sanitizer flags (a minor, not F's defect).
  - *The game's first moves* (13:39–13:40Z): keep a card, six decisions through the decision panel, the bot replying over the stream; the counts: the quiet minutes 13:28–13:36 **0**, the browser minutes 13:36–13:40 **52**, the window to 13:40:28 **99** — the control's 2 and this browser's play. The rule's events: `managed_challenge_bypassed` on every cleared call, **the WebSocket upgrade among them**, and `managed_challenge` ×1 each on the link arrival's two refusals — one challenge per fresh arrival, then bypasses.
  - *A challenged upload* (13:44–13:50Z, one local game, `local-ff549103c4e90202`): the name step typed; a local game dealt; the context taken OFFLINE and its cookies CLEARED; **149 decision clicks offline**, "The game is over."; the network restored → the `online` drain → **`POST /api/archive` 403 `cf-mitigated: challenge`** with no retry from the game page; the door's launch listed the game under "WAITING TO REACH THE ARCHIVE" and, in its first 12 s, ran no widget and sent no retry; the row read **"waiting for a human check"** with a **"Check now"** button; the click → one `POST /api/archive` 403 → the widget → **the overlay "Checking that this is a person…" on screen for under a second, no click** → `POST /cdn-cgi/challenge-platform/…` 200 → **`POST /api/archive` 201** (13:50:15Z), the section gone from the door; the Observability events read `POST /api/archive → 201 ok` at 13:50:15Z. One predicate, one overlay, one retry.
- **THE PINNED READING and W-2, EXPLAINED** (G.g; the ruling D-5). The pin on the Worker (`worker-open`, a fresh context at F's keyless release, `go` at 13:11:31Z) **did not observe the no-click update**: 15 checks, 10 held, 5 failed, 0 not run. Its plain reload at 13:11:35.8Z, 4 s after the deploy, fetched `/sw.js` (200, `HIT`) and nothing else, so it read the OLD file — propagation, a benign cause for that one check — and **each later recheck found the new worker and its install FAILED**: the zone's firewall events read `botFight managed_challenge` on `/art/manifest.json` with a browser User-Agent one second after each of four rechecks (the game tab's at 13:23:08Z and 13:38:08Z, the door's at 13:26:35Z and 13:41:35Z), none under WARP; and the door's own line read **"Could not prepare to play offline — /art/manifest.json: answered 403. Try again"** at 13:35Z. The pinned browser held `CF_Authorization` alone, **by the instrument's design** (it clears `cf_clearance`), and Bot Fight Mode challenges a clearance-less client's request for any allowed path that is not a static resource by extension — the precache's `/art/manifest.json` and `/shell.bin` among them. **W-3, owed to the wrap (`docs/web-lane.md` §15): under the no-Access door with Bot Fight Mode ON, a controlled browser whose clearance has lapsed cannot install a new release until something re-issues a clearance** — a controlled browser's navigations are answered from the cached shell, so the edge's JavaScript detection, which issues `cf_clearance` on an HTML response, never runs again for it, and the door's "Try again" refetches with no clearance. What restores one: a metered call that meets the rule's challenge (the widget sets one), or a navigation the edge answers itself. The window read the failure four times over 18 minutes; the fix is the wrap's decision (Bot Fight Mode off under the no-Access door, since the isolated bursts below prove the rule alone refuses scanners at no cost; a clearance-refreshing read; or the precache's extensions) and F changes no code. **The window therefore does NOT claim a pinned browser's observation of the no-click update.**
- **THE ISOLATED READINGS, over WARP** (13:51:54–13:57:27Z; `isolate-burst.sh` in the ledger): from a client Bot Fight Mode trusts, `/`, `/sitemap.xml`, `/build-values.json` and `/robots.txt` read **200 with no `cf-mitigated`**, and `/api/health` and `/wp-login.php` read **403 `cf-mitigated: challenge` — the rule's own**. **The default burst: 200 of 200 → 403** (13:53:00–13:54:40Z); **the `--under-prefix` burst: 200 of 200 → 404** (13:55:00–13:56:40Z); the counts for both: **0 invocations**; the firewall events 13:51–13:57: **`firewallCustom managed_challenge`, rule `dc4c5d3e…`, on the scanner paths ×58 (the adaptive sample of the 200) and NO `botFight` event at all**, plus the rule's bypasses on the browsers' traffic. **THE RULE'S OWN PROOF: from a client Bot Fight Mode trusts, every scanner-shaped request is challenged by the rule and none invokes the Worker; the allowed paths pass untouched; a cleared browser's calls are bypassed.** The whole window's invocations, 13:13:50–13:57:27Z: **157** — the control's 2 and the two browsers' play, the spectator view and the landing.
- **The revert, in the plan's order** (G.7). **(a) Access back FIRST** (13:57:31Z): `PUT` with the saved shape — `domain` `yokohama.moore.nyc`, the THREE hostnames, the policies by id — read back with **the `aud` UNCHANGED**; the window's total 43 minutes 40 seconds. **(b) The rule DISABLED by its id** (13:57:32Z): `PATCH …/rules/dc4c5d3e986144649617cbbaea58cd8a` with `enabled: false`, nothing else changed; the ruleset's version 15 → **16**; the rule stays at position 1, off, for the public end state, five of five. **(c) The shell half** (13:58:08–38Z): the keyless client build, the stamp `turnstile_sitekey: false` and **the release `yokohama-release-1088dec85fe623b8` AGAIN** (the build is reproducible); `npm run deploy` — the door line `wrangler.jsonc deploys the worker door — Cloudflare Access, verified by the shell`, `YOKOHAMA_WEB_IDENTITY ("worker")`, `YOKOHAMA_DEV_DOOR ("on")`, **`YOKOHAMA_TURNSTILE ("off")`**; **version `703e60d5-4eca-4fa1-9abf-06ab2ff2e398`, the thirty-first deployment**; `YOKOHAMA_WEB_ADMINS` put back from `git config user.email` through stdin (the address never printed), `secret list` naming `DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS`; `/api/health` `identity "cf-access"`; the smoke's live mode with `wrangler.jsonc` read "declares YOKOHAMA_TURNSTILE off — no edge line is owed, and no request was made"; the scratch key file removed.
- **THE NO-CLICK UPDATE, observed live at the revert** (G.g'; 13:59:22–29Z): an inspectable Chromium, its door page running F's keyed code, "Ready to play offline", with the reverted `/sw.js` already reading `1088…` on the network. **A plain reload** (the returning visitor's navigation): `installing` at 13:59:25 with **both generations cached** and the door reading "Preparing to play offline…"; **13:59:28 the door RELOADED ITSELF**; 13:59:29 the new worker `activated` and the door "Ready to play offline" — **no click, seven seconds from the navigation.** Not observed live: a stale SECOND tab's "Reload to use it" line — the two pages were in different browser contexts, so no tab shared the door's registration — which stays on its unit cells and the local e2e's two-tab cell.
- **The after-checks** (G.7 d–g): over WARP `/api/me` `mode cf-access` **naming the owner** and `GET /api/admin/tables` 200 as the owner; **the live replay once HOLDS** (14:01:18Z; 2,213 rows run, 90 named rows compared, mismatches none); **the stranger** (14:04Z, WARP OFF, a fresh context): `GET /` → 302 → the Access login → Google's sign-in, **the hostname-wide door back**; WARP reconnected after. **The quota after** (14:05:44Z): Worker requests account-wide 39,335 of 80,000 (the yokohama worker 23,390 that day, 0 errors), rows written 15,783, read 179,650; against the before-the-window read the hour cost the yokohama worker **2,918 requests** — the window's 157, the health reads and the live replay's 2,213 — and the **800 probe requests, four bursts of 200 at about 100 seconds each — 400 refused at the edge and 400 answered by the asset layer's 404 page — invoked the Worker zero times.**
- **THE WINDOW'S VERDICT** (13:13:51–13:57:31Z): (1) every scanner-shaped request was refused at the edge and none invoked the Worker — by Bot Fight Mode for a bare residential client, by the rule for a client Bot Fight Mode trusts — 400 requests, 0 invocations; (2) a miss under an allowed prefix is the asset layer's cached 404 page in every shape, 400 requests, 0 invocations; (3) the door's button pre-clears before the first metered call, a link arrival meets predicate → overlay → one retry in 1.3 s, a challenged upload reads "waiting for a human check" and lands on "Check now" through an overlay of under a second, and the WebSocket upgrade passes under the clearance; (4) the no-click update was observed at the revert in 7 s, and the pinned instrument's own miss is explained (W-2) and leaves W-3 owed; (5) the smoke's positive control holds only for a client Bot Fight Mode trusts (W-1); (6) `/favicon.ico` is challenged for an uncleared bare-URL load, then the asset layer's 404, no invocation; (7) the OTS WOFF2 console warning is a font-file note.

**The window's recipe, for the next operator** (in this order; the lane document's §10 is the same recipe in the lane's words):
1. **Before:** the quota guard; the widget read (mode `managed`, pre-clearance `managed`, the zone's domain among its hostnames); the zone's Challenge Passage, Bot Fight Mode and managed `robots.txt`; a FREE slot in the custom-rules phase (five on Free).
2. **The keyed build** (`cd web && npm run build` with the five values and `YOKOHAMA_TURNSTILE_SITEKEY`, then `cd worker && npm run build`), the key in a mode-0600 scratch file the build sources and the revert removes; `node scripts/edge-rule.mjs` from the keyed tree must equal the production tree's.
3. **The operator secret deleted, then `npm run deploy:open`;** `go` for the pin; the smoke's live mode fails here, by design.
4. **The edge rule created at position 1,** `--verify` rc 0.
5. **Access off,** the `aud` read back; then the smoke's live mode must PASS — **run over WARP, or from any client Bot Fight Mode trusts:** off WARP the sitemap control reads Bot Fight Mode's 403, which the smoke cannot tell from the rule's (W-1, D-2).
6. **The readings, each in its own named minute** for the query API: the curl shapes off WARP; the probe's two modes; the control; the browsers; the pinned reading; then the probe's two modes again OVER WARP, which isolates the rule's refusals from Bot Fight Mode's. **A pinned browser whose `cf_clearance` the instrument clears will meet W-3: its install of the new release fails on Bot Fight Mode's challenge of the precache's `/art/manifest.json` while Bot Fight Mode is on and the browser is off WARP** (the window read it four times). Read the pin over WARP, or with Bot Fight Mode off, or accept the failure line as the reading.
7. **The revert:** Access back FIRST, the rule DISABLED by its id, the keyless build, `npm run deploy`, the secret put back through stdin, the after-checks over WARP, ONE live replay, a stranger's check off WARP, the quota after, the scratch key file removed.

### What this deployment does NOT claim
- A PINNED browser's observation of the no-click update on this host: the pin failed, and W-2 explains why. The update was observed once, at the revert, in one cleared and inspectable browser, in 7 seconds.
- The stale second tab's "Reload to use it" line, live: the two pages were in different contexts. It stands on its unit cells and the local e2e's two-tab cell.
- That the rule alone refuses a BARE client from a residential address: Bot Fight Mode answers that client first. The rule's own refusals were read from a client Bot Fight Mode trusts (over WARP) and on a browser.
- That W-3 is closed: a lapsed clearance blocking a release's install was read at the no-Access door with Bot Fight Mode on, four times, and what to do about it is the wrap's decision.
- That the widget is invisible to every visitor: it ran with no overlay for the door's button and for about a second at a link arrival's, and a visitor Cloudflare scores otherwise would be shown a click.
- The release notice drawing anything: the repository is private, the feed answers 404, and no reading was taken with a public one.

### Rollback
- `wrangler rollback` to `eb8c7839…` (§19's twenty-eighth, E's release restored after its window) restores E's release; §19's caveats stand — **an older Worker version renders a `waited` marker's head as `undefined — …`**, a table document with `has_events` reads as before, the chat tombstone caveat of §6 stands, and open pickers drop (`picker-dropped`). **NEVER roll back to the thirtieth, `4088d3a7…`:** it is the window's open deploy, with no operator secret and the open door, and nothing in front of it once the rule is off.
- The edge rule is not part of a Worker version: it lives in the zone, DISABLED, and is enabled and disabled only by its id (`dc4c5d3e986144649617cbbaea58cd8a`) through the API, after the shell and before or after Access as the recipe orders.

---

## 21. Security S1 is live, and the archive's rename — the thirty-second and thirty-third deployments, 2026-10-11 (a record, not a plan)

The code is master `73469227` — security S1's three cycles merged as three merge commits (PR #50 `55641e3b`, PR #51 `aefc64f1`, PR #49 `73469227`) after the whole-stack review and its fix wave (the S1c plan's §"Review record") — tagged `v0.6.1` and **published by `release.yml` (run 38133636216; the release at 11:00:04Z) BEFORE this deploy was cut**, the owner's order. The NAS's deployment of the same code is `deploy/RUNBOOK.md` §3.21, run first, with the rename's NAS half inside its swap; this host's half of the rename followed this deploy. Every step the orchestrator's; no secret value appears here.

- **The build** (§1; 11:00Z, from the tagged master, after the NAS context was cut): `web/dist` with `YOKOHAMA_SITE_URL=https://yokohama.moore.nyc` — the stamp's five values, no Turnstile key, **the release `yokohama-release-05ba0edc21df1d5a`**, `debug_trap` 0; `npm run build`: the module, `assemble-assets` (153 precache entries verified, 162 files, 10,128,156 B), `smoke:release` **200 with `server_version 0.6.1`**.
- **The quota guard** (§7; GraphQL `workersInvocationsAdaptive` for the UTC day, read 11:13Z): `yokohama-web` **63 requests**, 0 errors — the day's live work was this window's alone.
- **THE UPDATE READING, prepared before the deploy** (`node web/scripts/pinned-browsers.mjs worker https://yokohama.moore.nyc web/scripts/pinned/worker-s1 --go-wait 90`; a fresh profile, pinned 11:07:22Z on `yokohama-release-1088dec85fe623b8`, the game tab `local-1e205cc2c8b9f836` at 3 events; `go` at 11:08:24Z after the health read): **11 checks, every check held** — no release taken before `go`, the deployed release `05ba0edc21df1d5a` not the pinned one, the door reloaded itself exactly once and reads "Ready to play offline", both generations stand, the install fetched `/shell.bin` and no HTML, the game tab drew "A new version is ready. Reload to use it", did not reload and kept its game.
- **The thirty-second deployment** (`npm run deploy`, 11:08:07–11:08:17Z; the guard's door line `wrangler.jsonc deploys the worker door — Cloudflare Access, verified by the shell`): 6 files uploaded, 154 already there; **version `ee8f7777-20e9-461c-805d-e6f3cb5f57d9`**; `/api/health` at 11:08:23Z **`server_version 0.6.1`**, `identity cf-access`, `signal ws`, `archive r2`, `table_ttl_days 90`, `open_table_ttl_days 7`; `/sw.js` names `yokohama-release-05ba0edc21df1d5a`.
- **The archive's rename, this host's half** (the recipe above, step 3; 11:09–11:10Z, `npx wrangler r2 object get/put/delete … --remote`, the NEW key read FIRST each time — none existed): `browser/<the owner's address segment>/` → `browser/<the owner's address segment>-d3aaf981a753/` (two abandoned pairs: 2,145 / 801 / 2,106 / 800 B), `browser/window-reader-g/` → `browser/window-reader-g-3e9364262694/` (one finished pair, 30,765 / 808 B; the bare name ruled onto its own derived segment), `test/browser/smoke_at_readme.test/` → `test/browser/smoke_at_readme.test-4bbc27c0eefa/` (one finished pair, 54,103 / 554 B) — **eight keys, each read back byte-identical before its old key was deleted**; **the replay's four legacy keys under `test/browser/red_at_wire.test/` DELETED, not moved** (the stack review's AH-F3: the stack's `engine_version` 0.6.1 against those saves' 0.6.0 would have met the first replay as the 409 that stops a run). The listing afterwards: `browser/` 6 keys, `test/browser/` 6 keys, every one in the digested shape (step 4's regex; the `test/rev7/` pairs are outside the rename).
- **The live corpus replay, twice** (§7; after the rename): run 1 at 11:11:00Z (4 s) — **HOLDS**: the forces 3 × 204 before and after, the house base `last 21, rev 18`, **26 rows run of 2,213, 2,187 skipped under the production seed rule** (`wire` and `wire2` answered 400 in `SEED_NOT_SOLO`'s words, as the rule expects), 19 named rows compared, 14 exemptions spent (the two rule rows, seven `HOST_FIELDS`, the three house-chat normalizations, `houseDeleteRow` sent as `/api/chat/22`, and **`firstSendAccepts` 201 for `archive_post` and `archive_post_abandoned` — the first sends landed under `test/browser/red_at_wire.test-c91ad2a917f7/`, D26's fresh upload under the new segment**), 0 lobby re-reads, no mismatch; run 2 at 11:11:53Z (2 s) — **HOLDS**, the two first sends reading the once-rule's 200 `"identical": true`.
- **The live acceptance on this host** (the owner's WARP identity): **B3** a seeded create beside an open seat → 400, nothing created; **B9** `probe.js` → 400 naming the dot — and **a table answer carries NO `cache-control` here** (the stack review's DC-F4: the Worker's own API answers take theirs in S2); **A1** `suggest` → 3,618 B with no seed-like key; **D28** a 300-byte line → 400 `a line is at most 256 bytes — this one is 300 bytes`; **B13** `HEAD /api/games/probe-s1c` → 200, empty, the table 200 after; **D12** a 16 KB id at `events` → 404, 97 B; **D1** `curl -N …/stream` as a second identity (the credential + `X-Yokohama-Claimed-User`, through the dev door) → **426 `this host streams over WebSocket`** — the SSE stream's end is the NAS's mechanism, not readable here (and not read there: one identity); B1's `why` at a solo bot table → the typed `no_bot_move_yet` refusal (the WITHHELD reading was not reached — the placement picker's selection is the client's, not a line). Four probe tables created and deleted (`probe-s1c`, `probe-s1c-5`, `probe-s1c-open` — the D1 table, joined by a second claimed identity — and `probe-s1c-seed`, the refused create); each `DELETE` 204, each `GET` 404 after.
- **The thirty-third deployment — the gate** (`npm run deploy:gate`, 11:16:11Z, after every probe that used the dev door; the guard's line `wrangler.gate.jsonc deploys the worker door…`, `YOKOHAMA_DEV_DOOR ("off")`): **version `fd34c477-f429-4123-acb4-b1d0bf13e96d`**; `/api/health` 0.6.1 as above; **a request carrying the credential and a claimed address now reads `{"mode":"cf-access","user":null}`** (the door is off), the owner over WARP still named; the two secrets `DEV_DOOR_CLIENT_ID` and `YOKOHAMA_WEB_ADMINS` still bound (`wrangler secret list`). **A live tier that needs the dev door (the replay's claimed identities) now needs `npm run deploy` first and `npm run deploy:gate` after** — the window's own order.
- **The five configuration changes** (spec §2.2 B4, B6; the board's list; each read back): (1) the Access application `yokohama`'s **`same_site_cookie_attribute` unset → `lax`** and (2) **`session_duration` 730h → 24h** (one PUT carrying the application's read-back fields; the three destinations and the three policies unchanged — `sbaitso-friendly` 24h, `yokohama-players`, `yokohama ST bypass`); (3) the gate deployed (above); (4) the repository's `.dev.vars` **mode 644 → 600 and its unused `TURNSTILE_SECRET` line removed** (five names remain); (5) the stopped NAS container `cloudflare-d` (`c5840a0f…`, exited 2026-08, its command line carrying the tunnel `bdvc-llc`'s token) **deleted through Portainer, and that tunnel's secret ROTATED** (`PATCH …/cfd_tunnel/2c2652c6-…` with a fresh `tunnel_secret`; the tunnel `down` since 2026-08-05 with 0 connections before and after; the live NAS tunnel `bdvc-nas` untouched). **One thing met on the way:** the workstation's WARP client read `Connected` while Access answered every bare `curl` with "Please authenticate via the warp client" — the device's Access identity had lapsed; `warp-cli debug access-reauth` and its `refresh-identity` page restored it (`/api/me` naming the owner on both hosts), and the window ran over WARP as §7 requires.

### What this deployment does NOT claim
- D1's live ending and B1's WITHHELD `why` were not read live on either host (above); their cells are the catchers.
- The Playwright live tier, the screenshots and the phones were not run (not in S1c's Step 10 list; the code they cover did not change in S1).
- The update reading is a headless fresh profile on the workstation, not the owner's own browser.
- Rows written and read per move were not separated from the day's totals.

### Rollback
- `wrangler rollback` to **`703e60d5-4eca-4fa1-9abf-06ab2ff2e398`** (2026-10-01, F's release, `worker/RUNBOOK.md` §20) restores 0.6.0; §6's bullets stand — and **§6's new one: the archive's rename is ONE-WAY**, so a rollback files new saves under the legacy segment beside the renamed keys until the roll forward's window runs the recipe again.

---

## 22. Security S2 is live — the thirty-fourth and thirty-fifth deployments, 2026-10-17 (a record, not a plan)

The code is master `5cbeb390` — security S2's three cycles merged as three merge commits (PR #55 `70ade983`, PR #54 `53f427a2`, PR #52 `5cbeb390`) after the stack review and its fix dispatch (the S2c plan's §"Review record") — tagged `v0.6.3` and **published by `release.yml` (run 39034825202; the release at 13:05:35Z) BEFORE this deploy was cut**, the owner's order. ONE window for S2 (the fold, R-8): the NAS's deployment of the same code is `deploy/RUNBOOK.md` §3.22, run FIRST as the canary (D-15), and this host was touched only on its clean reading. Every step the orchestrator's with the owner at the keyboard; no secret value appears here.

- **The build** (§1; 17:49Z, from the tagged tree, after the NAS context was cut): `web/dist` with `YOKOHAMA_SITE_URL=https://yokohama.moore.nyc` — the stamp's five values, no Turnstile key, **the release `yokohama-release-ef0f946478c75a51`**, 153 precache entries, 10,091,184 B, the module 1,250,253 B, `debug_trap` 0; `npm run build`: the module **1,985,926 → 1,316,058 B** (`wasm-opt 133`, −33.7 %), `factory-glue: 18 exports`, `assemble-assets: 153 precache entries verified` (162 files, 10,130,945 B; `_headers` carrying `headers.json`'s block), `smoke:release` **200 with `server_version 0.6.3`**.
- **The quota guard** (§7; GraphQL through the Cloudflare MCP, one dataset per call; the owner's OAuth consent in this session): read at 18:39Z, BEFORE the tiers — Worker requests account-wide **19,181** (this Worker's own 489), Durable Object rows written 300, read 1,117; **after the tiers (18:48:48Z): 31,523 account-wide (this Worker 12,777), rows written 3,506, read 58,730** — no HOLD; the two live tiers and the window's probes cost about 12,300 requests, a third of §7's recorded figure for one `two_browsers` game (the figures lag by minutes). Wrangler's own OAuth token is refused by the GraphQL and the observability APIs ("not authorized for that account"), so the MCP, or an analytics-read token, is the instrument.
- **THE UPDATE READING, prepared before the deploy** (`node web/scripts/pinned-browsers.mjs worker https://yokohama.moore.nyc web/scripts/pinned/worker-s2 --go-wait 90`; a fresh profile, pinned 18:00:07Z on `yokohama-release-05ba0edc21df1d5a`, the game tab `local-fa5ccdd007e3d8c4` at 3 events; `go` at 18:02:04Z after the health read): the door reloaded once at 18:02:07Z; **11 checks, every check held** (18:02:24Z) — no release taken before `go`, the deployed release `ef0f946478c75a51` not the pinned one, the door reloaded itself exactly once and reads "Ready to play offline", both generations stand, the install fetched `/shell.bin` and no HTML, the game tab drew "A new version is ready. Reload to use it", did not reload and kept its game; `done` at 18:04:39Z, `cookies: cleared before close` before the final `closed:` line, the profile's `Default/Cookies` holds no row.
- **The thirty-fourth deployment — the dev door ON for the live tiers** (`npm run deploy:dev`, 18:01:44–18:02:00Z; the guard's door line `wrangler.dev.jsonc deploys the worker door — Cloudflare Access, verified by the shell` and its one sentence; `secrets-guard: 2 secrets on wrangler.dev.jsonc's Worker, none named like a switch`): 6 files uploaded, 154 already there; `YOKOHAMA_DEV_DOOR ("on")`; **version `2b6edea3-2844-4b00-b003-1ba0377d5f4c`**; `/api/health` at 18:02:03Z **`server_version 0.6.3`**, `identity cf-access`, `signal ws`, `archive r2`, `table_ttl_days 90`, `open_table_ttl_days 7`; `/api/me` names the owner; `/sw.js` names `yokohama-release-ef0f946478c75a51`.
- **The header reads**: `/` (`cf-cache-status: HIT`, `cache-control: public, max-age=0, must-revalidate`), `/shell.bin` (`no-store`), `/nothing-here` (**404**) and the entry chunk `/assets/index-BvjgOIZi.js` (`immutable`) each carry the whole set — the policy with `'nonce-9fbW9ooNwE2u6pVVRviwRA'`, `x-frame-options: DENY`, `nosniff`, the referrer, permissions and opener policies; **`/api/health` carries `cache-control: no-store` and `x-content-type-options: nosniff` and nothing of the asset set** — `handle`'s own pair on a Worker-generated answer (S2a's B9 half, as designed: a `_headers` block never applies to one). The injected script's nonce: one `nonce="9fbW9ooNwE2u6pVVRviwRA"` attribute by `curl`, two in the browser's document.
- **THE CANARY on this host (D-15's second reading)** — a fresh profile's first visit, every console level, 18:02:32–18:02:51Z: **no CSP line**; the ONE console line `[error] Failed to load resource: the server responded with a status of 404 ()` at `https://api.github.com/repos/jmoore2333/yokohama-assistant/releases/latest` — the release feed again, GitHub answering 404 here where the NAS's visit minutes earlier read 403 at the same URL (the anonymous rate limit against the private repository; the walk forgives both forms); the second, SW-served navigation printed nothing and carried the set with `no-store`; the 404 page SW-served; **B4** the `data:` framer refused — `Framing 'https://yokohama.moore.nyc/' violates … "frame-ancestors 'none'"`; the profile's cookies cleared, none left. The three readings (the NAS's, this one, the Playwright tier's) agree: the policy enforced, the console clean of it.
- **The live replay's reset BEFORE the replay** (§7, "The replay's reset"; the stack's `engine_version` moved 0.6.1 → 0.6.3): the four keys under `test/browser/red_at_wire.test-c91ad2a917f7/rev7/{over,abandoned}/wire[.table].json` deleted with `npx wrangler r2 object delete … --remote` (18:02–18:03Z), each "Delete complete."
- **S2b's live acceptance** (spec §5.3; X2; 18:05:44–18:06:59Z, over WARP as the owner, a `wrangler tail --format json` running throughout): **C2, the burst** — 300 `POST /api/chat` at 16 a time (§7's recipe): **122 × 400 and 178 × 429**; one refusal whole, sent at once inside the window: `HTTP/2 429`, `retry-after: 10`, `{"error":"too many requests from you just now — wait a moment and try again"}`; **B5 and the operator playbook's JSON type** at `probe-s2-op` (created, named by the listing as `dealing`): a bodyless POST with no type → **400** `Expected request with \`Content-Type: application/json\`` (the layer's own, as §3's playbook says), a JSON POST from `Origin: https://evil.example` → **403** `this request is for the site's own pages` (B5, before identity), the JSON POST → **204**, the table 404 after; no save landed in R2 for it (an eventless table). **D3, the socket cap** at `probe-s2-sock` — nine WebSockets as one identity (`ws` from Node over WARP): eight open, each handed the `signal` frame; **the ninth accepted, sent `{"event":"end","data":{"reason":"gone","sentence":"you have too many connections open to this table — close another tab and try again"}}` and closed 4429** with that sentence as the reason; a tenth opened once one of the eight was closed. **AD-4, the lobby's refusal drawn** — nine lobby pages in one fresh profile: pages 1–8 the lobby, **page 9 drew `you have too many connections open to the lobby — close another tab and try again`** (`TOO_MANY_OWN_LOBBY`, on screen). **D14** `#/t/..` typed in the address bar landed on the door. **D23 in the logs**: the tail caught 105 invocations (85 `ok`, 20 `canceled` — the burst's refused connections) and **9 shell log lines — `operator` ×2, `slot-pushed` ×3, `slot-acked` ×3, `expired` ×1 — none holding an address or a query string**; **`limiter-failed`: 0** (R-5); the `init-refused` line was NOT observed — a later probe with the credential and an address-shaped claim read `{"mode":"cf-access","user":null}` (refused under `NamesOnly`, C1) under a second tail that caught no log line for it, so that line's digest stands on its cells (`r2.unit.test.ts`, the stack review's lens 1). (The tail's own request metadata shows the Access header and the cookie — the live tail's view, not what the persisted logs keep; `redact_query_string` is the guard's check 14.)
- **The two-identity block's Worker half** (WARP off for the slot; profile B as `deploy/RUNBOOK.md` §3.22 describes it): **B11** — B, signed in through Access on the NAS hostname, was signed in on this hostname by the one application without a second prompt (the single audience); **row 119** — `GET /api/admin/tables` as B → **403** `this address is not an operator here` (the operator gate's refusal of a non-operator, read live); **row 108** — A created `probe-108` as `[A, open]`, B joined seat 1 (200, `my_seats [1]`, `mine false`, `you true` at seat 1 in B's view): a second signed-in address at a single-audience table; deleted (204, 404 after). Every probe table of the window deleted; the second identity's browser profile deleted at the slot's end; WARP back on at 18:41:16Z, both hosts naming the owner over it with no re-auth needed.
- **The live corpus replay, twice** (§7; after the reset): run 1 at 18:42:21Z (6 s) — **HOLDS**: the forces before and after, the `HOST_FIELDS` exemptions, **the house watcher tag taken back `7e52 → a9f4` (the production salt, D8 — §7's new exemption, spent as designed)**, the house chat normalized by 23 earlier messages, `houseDeleteRow` sent as `/api/chat/24`, **`firstSendAccepts` 201 for `archive_post` and `archive_post_abandoned`** (the first sends after the reset), 0 lobby re-reads, no mismatch; run 2 at 18:42:27Z — **HOLDS**, the two first sends reading the once-rule's 200 `"identical": true`, the tag `7e52 → a9f4` again.
- **The live Playwright tier, once** (§7; 18:42:29–18:48:15Z, `YOKOHAMA_WEB_E2E_TARGET=live`, the two specs): `joins_by_link` **passed** (4.4 s; `sim-6e1ac8aebf5701d0`, 6 moves across two browsers; the deal's 409 forgiven a 2; the beacon's CORS error forgiven a 2, b 2; the release feed's failure forgiven a 2); `two_browsers` **passed** (5.6 min; `sim-cca429f57768d5a8`, **1,387 moves**, 336 s, the watcher held to 1,006 applied moves and opened `/stream` once, the players' stream opens a 2, b 2; the beacon forgiven a 2, b 2, c 2; the feed a 2, b 2, c 2); **2 passed (5.7 min), NO CSP line on any page — the enforced policy's third reading**; no `test-results/` left.
- **The thirty-fifth deployment — the dev door OFF** (`npm run deploy`, 18:48:35Z, after every probe that used the door; the guard's line `wrangler.jsonc deploys the worker door…`, `secrets-guard: 2 secrets … none named like a switch`, `YOKOHAMA_DEV_DOOR ("off")`): **version `f1a0fbca-434e-4bd0-8192-79886b1ff3d3`**; `/api/health` 0.6.3; the owner over WARP named; **the credential plus `X-Yokohama-Claimed-User: probe@window.test` now reads `{"mode":"cf-access","user":null}`** (with the door on, the same request had named `probe@window.test`). The two secrets (`DEV_DOOR_CLIENT_ID`, `YOKOHAMA_WEB_ADMINS`) stay bound.

### What this deployment does NOT claim
- The `init-refused` log line's digest was not read in a tail (above); its cells are the catchers. The origin's 408 was not read on either host (the edge answers a short body first).
- The screenshots and the phones were not run (not in Step 10's list).
- The update reading is a headless fresh profile on the workstation, not the owner's own browser.
- A public `why` (game over with a held seat) was not read live; the withheld one was, on the NAS.

### Rollback
- `wrangler rollback` to **`fd34c477-f429-4123-acb4-b1d0bf13e96d`** (2026-10-11, S1's gate deployment, `worker/RUNBOOK.md` §21) restores 0.6.1 with the dev door off; §6's bullets stand — the headers are stateless, the replay's four keys are re-created by the next first sends, and nothing in this window is one-way.

---

## 23. The public door opens — the switch to WorkOS (stage 1 a record, 2026-10-19; stage 2 a plan)

**The recipe for the window that moves `yokohama.moore.nyc` from the Access door to the WorkOS-signed Worker.** Its code is the auth cycle's (`docs/plans/2026-10-18-auth-cycle-design.md`, its plan beside it; the Worker's three identity sources are `docs/web-lane.md` §4's bullet); it keeps the fence of the lane's §10 public window's order — the edge rule is in front BEFORE the hostname leaves Access, so the API is never open and unchallenged — but not its steps, because the rule is DISABLED while Access is on (below). Orchestrator-only, through `wrangler`, the WorkOS dashboard or MCP and the Cloudflare API, each step a dated line in the cycle's ledger. **A plan, not a record, for stage 2: none of its steps has run but step 3's commit (`d6db6b3e`, 0.7.3); stage 1's record is below.** It runs in TWO stages (the owner's "both" direction of 2026-10-19, rulings R-51 and R-55): until the public repository is cleaned up Cloudflare Access stays on at the edge over the WorkOS-signed Worker, and the Worker at `workos` never reads Access's assertion. **The window runs against the PRODUCTION WorkOS environment**; the staging environment is not an issuer here.

**Why two stages work, and why the rule waits.** At source `workos` the shell names a caller by its own session cookie and nothing else (`answer`, step 5); Access's verifier never runs. Access, hostname-wide on `yokohama.moore.nyc`, still gates every request — `/api/*` and `/enter` included — at the edge, so a visitor passes Access's login, then `/enter` redirects to WorkOS, then the Worker's cookie names the player. The client tells the two challenges apart (`WWW-Authenticate: Yokohama-Session realm="/enter"` against Access's own). Being hostname-wide, the application already covers `/api/leave`; only a PATH-scoped application (the door window's state 2, §19) needs `node scripts/access-paths.mjs`'s eleven lines. **The edge rule stays DISABLED under Access** (`worker/scripts/edge-rule.mjs`: "enabled while the hostname is off Access, DISABLED under Access — the live tiers' service token cannot pass a challenge, and Access already stops every stranger"), so stage 1 ships the KEYLESS build with `YOKOHAMA_TURNSTILE` `off`, and the keyed build, `on` and the rule arrive together at stage 2.

### Stage 1 — the WorkOS-signed Worker, Access still in front, the rule disabled

1. **Before.** The quota guard (§7); the zone read as §20's step 1 reads it (Challenge Passage, Bot Fight Mode, managed `robots.txt`); the edge rule read BY ID — `dc4c5d3e986144649617cbbaea58cd8a`, at position 1 of the zone's `http_request_firewall_custom` ruleset `27fcc0c7398f42e3abbf549a917e5fa7`, DISABLED, five of five — and the Turnstile widget read (mode `managed`, pre-clearance `managed`, the zone's domain among its hostnames).
2. **The production WorkOS environment** (the dashboard, or the WorkOS MCP's `mutate` with a dry run first). Its URL settings: the **redirect URI** `https://yokohama.moore.nyc/enter`; the application's **Initiate login URI** (the docs' sign-in endpoint; `updateInitiateLoginUrl` with `url`, read back as the application's `initiateLoginUri`) the same, `https://yokohama.moore.nyc/enter` — **and the ENVIRONMENT-level external login URI (`updateAuthkitExternalLoginUri`, read as `userlandSettings.externalLoginUri`) EMPTY: a different field under a near-identical name, and with it set the hosted AuthKit page answers EVERY unauthenticated visit with a 307 to it, which `/enter` reads as a fresh start — a redirect loop, read on 2026-10-19 (the stage-1 record's last bullet)**; the **sign-out redirect** `https://yokohama.moore.nyc/` AND the **App homepage URL** (`appHomepageUrl`) the same (the leave URL names the site's root as `return_to`, and a logout against an environment without a homepage lands on a WorkOS error page, read on staging on 2026-10-18). The sign-in **methods**: the email code (Magic Auth) and passkeys on; each social provider — Google, GitHub, Apple, and Discord if the owner wants it — on only after the owner's own provider credentials are in the dashboard; **passwords off**. The **branding** (the transparency the owner asked for, R-A14): the theme and colours, and the logos, light and dark, which are uploaded in the dashboard (the MCP does not take the files). Read each setting back; record the environment's client id.
3. **The two secrets, put BEFORE the deploy** — the secrets guard REFUSES `npm run deploy` without them (R-48):
   ```sh
   cd worker
   npx wrangler secret put YOKOHAMA_OIDC_CLIENT_SECRET --config wrangler.jsonc    # the production environment's API key, at the prompt
   openssl rand -base64 48 | npx wrangler secret put YOKOHAMA_SESSION_SECRET --config wrangler.jsonc
   ```
   The API key is minted in the dashboard and pasted at the prompt or piped from the owner's gitignored root `.dev.vars`; it is never printed and never written to a TRACKED file, a command line or a log. The session secret is generated and piped, never shown. Neither is a `vars` entry (the deploy guard's check 7).
4. **The keyless build with the notice** (§1's order, the web client FIRST): `cd web && npm run build` with the SIX build values — `YOKOHAMA_PROJECT_URL`, `YOKOHAMA_SITE_BLURB`, `YOKOHAMA_SITE_NOTICE`, `YOKOHAMA_SITE_URL=https://yokohama.moore.nyc`, `YOKOHAMA_RELEASE_FEED` (unset derives it from a GitHub project link) and **`YOKOHAMA_SIGNIN_NOTICE`** (at most 300 characters: who signs the player in, that this site keeps only the verified address to seat them, and that finished games are archived — R-A14; R-55: stage 1 has sign-in links, so it carries the notice) — and **NO `YOKOHAMA_TURNSTILE_SITEKEY`** (R-51). Then `cd worker && npm run build`. The stamp must read `turnstile_sitekey` false and `signin_notice` true.
5. **The configuration files and the committed-file cells that pin today's state — ONE commit.** `wrangler.jsonc`: `YOKOHAMA_IDENTITY_SOURCE` `"workos"`, `YOKOHAMA_OIDC_CLIENT_ID` the production client id, `ACCESS_TEAM_DOMAIN` and `ACCESS_AUD` REMOVED (the guard refuses them beside an arm), no `YOKOHAMA_OIDC_ISSUER` (refused at `workos`), `YOKOHAMA_TURNSTILE` stays `"off"`, the dev door still `off`. `wrangler.dev.jsonc` follows, with its dev door `on`. **The open files stay as they are** (they omit the arm's variables, R-45; the open-file cell never moves) and `wrangler.private.jsonc` stays at `access`. Then `npm run test:scripts` goes red on exactly FIVE cells, measured on 2026-10-19, and they move in this commit: in `worker/scripts/deploy-guard.test.mjs` "pass every check that needs no build under --config wrangler.dev.jsonc", "pass the whole guard: production, the dev file and the private file over a keyless build, the open file over a keyed one" (its keyed-tree loop's expected sentence for production and the dev file changes: at `workos` a keyed tree beside `off` is refused for the declaration) and "name the worker door"; and in `worker/scripts/secrets-guard.test.mjs` the two command cells that run over the committed `wrangler.jsonc`, "passes a clean listing: exit 0 and the sentence" and "requires an arm's secrets, reading the judged file's source from the working directory (R-48)". Whatever else reads red moves with them; each change is read, not regenerated.
6. **The edge rule stays DISABLED** — step 1's read by id is the only touch. Under Access it must be: the live tiers' service token cannot pass a challenge, and Access already stops every stranger.
7. **`npm run deploy`.** The guard's door line must read `wrangler.jsonc deploys the worker door, source workos — WorkOS AuthKit, through its first-party API at the callback`, wrangler's vars listing `YOKOHAMA_TURNSTILE ("off")`, and the secrets guard `…, and the workos arm's YOKOHAMA_OIDC_CLIENT_SECRET and YOKOHAMA_SESSION_SECRET among them`. Record the version id. The smoke's live mode reads `smoke-release: wrangler.jsonc declares YOKOHAMA_TURNSTILE off — no edge line is owed, and no request was made`.
8. **The readings**, each in its own named minute for the query API (§7's counting): **Access's login first, then the WorkOS page**, branded; the redirect back to `/#/lobby` with the session cookie set; **`/api/me` naming the player**; a table dealt; a WebSocket opened under the cookie; **two invocations per sign-in in the logs** — the start and the callback, and nothing else of the sign-in; a refused sign-in landing on `/?signin=failed#/lobby`; a sign-out through `POST /api/leave` that clears the cookie and lands on the site's root; a local game's finished upload landing anonymously under a browser with no session; the console of a fresh profile clean of any CSP line (the policy is enforced, S2c); **one live replay HOLDS** — with `YOKOHAMA_SESSION_SECRET` in the instrument's environment (never an argument, never echoed) the replay mints a `.test` session per row's identity and sends it as `Cookie` beside `.dev.vars`' Access credential, which is the edge's pass; the live tier seeds each context's minted session into its cookie jar, never a header; a row or context whose identity is not a `.test` address is refused, and with only `YOKOHAMA_SESSION_COOKIE` set (the one-identity mode) the replay refuses, before anything is sent, every row that names a `who`, the cookie's own address included — so the committed walk, which names several, runs under the secret — and the live tier refuses a context for any email but the cookie's own address (`worker/scripts/mint-session.mjs`, R-50, R-39, R-52–R-54). The operator surface reads as a listed address signed in through WorkOS (`/api/admin/tables` 200).
9. **Rollback.** Stage 1's window commit REVERTED — both wrangler files and the moved cells — so `wrangler.jsonc` and `wrangler.dev.jsonc` are back at source `access` with the Access pair; the keyless build the production release came from redeployed with `npm run deploy` (its guard's door line names `access`); if stage 2 had deleted `DEV_DOOR_CLIENT_ID`, it is put back (the `yokohama-worker` credential's client id, §4). The two WorkOS secrets stay in place (a deploy leaves secrets alone, and the secrets guard asks nothing of them at `access`). Access never left and the rule was never enabled, so nothing else moves.

### Stage 2 — the keyed build, the rule, then the hostname off the Access application (a later session, with the owner)

1. **Before.** Stage 1 deployed and read; the owner present; the rule read by id, still DISABLED; the quota guard.
2. **The keyed build with the notice:** `cd web && npm run build` with the SEVEN build values — stage 1's six and **`YOKOHAMA_TURNSTILE_SITEKEY`** (read once; kept in a mode-0600 scratch file the build sources, and removed at the end — the key is public by Cloudflare's design and no document spells it) — then `cd worker && npm run build`. The stamp must read `turnstile_sitekey` and `signin_notice` true, and `node scripts/edge-rule.mjs` from the keyed tree must print the production tree's rule byte for byte.
3. **`YOKOHAMA_TURNSTILE` `"on"` in `wrangler.jsonc` and `wrangler.dev.jsonc`**, committed with stage 2's moving cells. Measured on 2026-10-19 against today's `access` files, the stage-2 shape reddens SIX cells in all: stage 1's five and "declare YOKOHAMA_TURNSTILE off in five and on in the open file alone". Stage 1's commit already moved its five, so stage 2 adds "declare YOKOHAMA_TURNSTILE off in five …" and moves "pass the whole guard: production, the dev file and the private file over a keyless build, the open file over a keyed one" AGAIN (production and the dev file now pass over a keyed tree and are refused over a keyless one). The open-file cell never moves.
4. **The edge rule ENABLED by its id**, before the deploy: `PATCH /zones/{zone}/rulesets/27fcc0c7398f42e3abbf549a917e5fa7/rules/dc4c5d3e986144649617cbbaea58cd8a` with `enabled: true` and nothing else changed (§20's revert disabled it the same way); the ruleset's version moves by one and the rule stays at position 1. Read the zone's copy back and pipe it to `node scripts/edge-rule.mjs --verify`: exit 0, the line beside it naming the tree's release and its `site_url`. **Access is still on, so the live tiers are out from here** — their service token cannot pass the challenge.
5. **`npm run deploy`**, then the smoke over WARP, or from any client Bot Fight Mode trusts (§20, W-1). The guard's door line is stage 1's, wrangler's vars listing reads `YOKOHAMA_TURNSTILE ("on")`, and the smoke's live mode now requires the edge line. With Access still in front, record what it reads (whether the rule or Access answers a cookie-less `/api/health` first is not yet known) rather than treating a miss as the deploy's failure; it must PASS at the next step.
6. **`yokohama.moore.nyc` OFF the Access application, LAST** (§10's step 5, the Cloudflare API): the application's `self_hosted_domains` and `destinations` keep the NAS, CLI and TEST hostnames — **`yokohama-nas.moore.nyc`, `yokohama-cli.moore.nyc` and `yokohama-test.moore.nyc` stay on Access** (the test host since security S3's Task 7, 2026-10-20, §26 — a PUT replaces the whole list) — the primary `domain` moved to the NAS's hostname, the policies by id, the `aud` read back unchanged. The rule is already in front, so the API is never open and unchallenged. The smoke's live mode must now PASS.
7. **The readings, as a stranger:** a fresh profile off WARP reads the whole path with no Access — the overlay, the WorkOS page, the lobby, a dealt table, the socket, the sign-out; a profile with the PREVIOUSLY RELEASED client (a pinned profile at `0.6.3`, `web/scripts/pinned-browsers.mjs`; W-3 applies to its install, §20) lands a waiting game anonymously under the game's own id, with no update installed (the lane's §15, W-3's upload half); and **the friend's game** (`local-03bf781aa067cf0a`, the design's §1: played signed in to Access, never uploaded, since a solo game makes no server call to keep the 24-hour session alive) — whether the friend's browser still holds its waiting row, and whether the anonymous landing above is what lands it, is the reading; if the browser no longer holds it, the game is not recoverable by this switch.
8. **The dev door's secret.** `DEV_DOOR_CLIENT_ID` deleted (`npx wrangler secret delete DEV_DOOR_CLIENT_ID --config wrangler.jsonc`): the dev door is inert at `workos` and its secret is one of the public door's preconditions (`docs/web-lane.md` §8). The scratch key file removed.
9. **Rollback.** The hostname back on the Access application FIRST (the saved shape: `domain` `yokohama.moore.nyc`, the four hostnames (the test host included since 2026-10-20, §26), the policies by id, the `aud` unchanged); THEN the rule DISABLED by its id (`enabled: false`, nothing else changed — under Access it must be); then stage 1's keyless build and `off` redeployed, which is stage 2's commit reverted; then, if the Worker is to leave `workos` as well, stage 1's rollback. A rollback that reaches `access` also puts `DEV_DOOR_CLIENT_ID` back, or the dev door names no one at the next live tier.

### Stage 1 — the record: the thirty-sixth deployment, 2026-10-19 (the orchestrator, the owner away)

- **Before** (steps 1–3): the production WorkOS environment configured over the MCP — the redirect URI
  `https://yokohama.moore.nyc/enter` (`redir_01M59G8YCJPZB5NNW2FZYXWVEN`), the logout URI `https://yokohama.moore.nyc/`,
  the external login URI `/enter` (the WRONG field — corrected the same afternoon, this record's last bullet), magic
  auth and passkeys on, passwords off, sign-up open, `appHomepageUrl` set; the
  branding (`displayName` Yokohama, theme Dark, radius Small, logo style Icon over the owner's six uploads, page
  `#0d1013`, buttons `#6f9bff` on `#0d1013`, links `#6f9bff`, the heading "Sign in to Yokohama", the legal text = the
  transparency sentence); Google, GitHub, Apple and Discord LEFT OFF (production needs the owner's own provider
  credentials). The two secrets put (the API key piped from the owner's gitignored root `.dev.vars`, never printed;
  the session secret `openssl rand -base64 48`): `wrangler secret list` reads FOUR names. The edge rule
  `dc4c5d3e986144649617cbbaea58cd8a` read by id: `enabled: false`, left so.
- **The build** (step 4; ~08:50 local, from `3991feb3`, the `0.7.0` bump): `web/dist` with the SIX values — the
  project link, the 226-character blurb, `Not affiliated with Asmodee`, `YOKOHAMA_SITE_URL=https://yokohama.moore.nyc`,
  the feed derived, **`YOKOHAMA_SIGNIN_NOTICE`** the transparency sentence — and NO site key: **the release
  `yokohama-release-8a45949bc58fdd41`**, 153 precache entries, 10,093,013 B, the module 1,250,304 B (`wasm-opt 133`,
  1,892,714 → 1,250,304 B, −33.9 %), `debug_trap` 0, the stamp `signin_notice: true`, `turnstile_sitekey: false`;
  `npm run build`: `factory-glue: 18 exports (a release module: no debug_trap)`, `assemble-assets: 153 precache entries
  verified` (162 files, 10,132,797 B), `smoke:release` **200 with `server_version 0.7.0`**.
- **The commit** (step 5): `18d63f28` — `wrangler.jsonc` and `wrangler.dev.jsonc` at `YOKOHAMA_IDENTITY_SOURCE`
  `"workos"` with `YOKOHAMA_OIDC_CLIENT_ID` `client_01M58WM6MNRQPY96302487DQF7` and the Access pair removed,
  `YOKOHAMA_TURNSTILE` still `"off"`; the five moving cells (`test:scripts` 781/781 after). The guard's four runs:
  `wrangler.jsonc`, `.dev` and `.private` exit 0 — the door line **`source workos — WorkOS AuthKit, through its
  first-party API at the callback`**; `wrangler.open.jsonc` exit 1 over the keyless tree with its own Turnstile-on
  sentence (expected: it deploys over a keyed tree alone).
- **The deploy** (step 7; ~09:05 local): `npm run deploy` — the deploy guard clean, the secrets guard **"4 secrets on
  wrangler.jsonc's Worker, none named like a switch, and the workos arm's YOKOHAMA_OIDC_CLIENT_SECRET and
  YOKOHAMA_SESSION_SECRET among them"**, 8 assets uploaded (152 already there), **Worker version
  `ddedbb86-5502-4005-9912-a0fa85d527c5`**. The roster at the bump: 15/15 green, 497 s as one sequence (MacBook Air,
  Apple M5; the evaluator gate 253 s). Tagged `v0.7.0` on `18d63f28`.
- **The readings, from the owner's workstation over WARP — the OWNER's Access identity, so no Access login appears
  here:** `/` 200 with `sw.js` naming `yokohama-release-8a45949bc58fdd41`, the enforced policy with its nonce,
  `X-Frame-Options: DENY`; `/build-values.json` the seven booleans; `/api/health` 200 (`server_version 0.7.0`);
  **`/enter` 302 to `https://api.workos.com/user_management/authorize?client_id=client_01M58WM6MNRQPY96302487DQF7&
  redirect_uri=https%3A%2F%2Fyokohama.moore.nyc%2Fenter&response_type=code&provider=authkit&state=<43 chars>`** — the
  WorkOS start, live. The Access application `yokohama` (`11cbcf68-96cd-4310-baee-f2ade812155d`) read over the API:
  `self_hosted_domains` the three hostnames, five policies (four allows and the service-token bypass) — Access is ON
  over the public hostname. **NOT read, for want of a stranger's device and the owner's inbox:** Access's login as a
  stranger meets it, the WorkOS page's branding, the email code, the redirect back with the cookie, `/api/me` naming
  the player, a dealt table, the socket under the cookie, the sign-out, two invocations per sign-in in the logs — the
  owner's checklist (`docs/handover.md`, Track 4) takes them first thing.
- **Rollback** (step 9): `git revert 18d63f28` (both wrangler files and the five cells), `npm run deploy` of the same
  keyless tree; the four secrets may stay. The rule was never enabled; the Access application was never touched.
- **The owner's first read, the same afternoon (2026-10-19 ~15:00Z, this workstation, WARP on and off alike):
  `ERR_TOO_MANY_REDIRECTS` on the multiplayer card, the address bar on the WorkOS host.** Traced from the shell with
  `curl -L` and a cookie jar over WARP: `/enter` 302 → `api.workos.com/user_management/authorize` 302 →
  `heroic-peak-11.authkit.app/bootstrap` 307 (its `__Host-state-<sid>` cookie set) → its `/` with that cookie sent →
  **307 to `https://yokohama.moore.nyc/enter` with no `code` and no `error`** → a fresh start, and so on until curl
  gave up at twelve hops. The identical request against the STAGING client id rendered the sign-in page (200), so the
  difference was the production environment's configuration: the run had set the ENVIRONMENT's external login URI
  (`updateAuthkitExternalLoginUri`; Step 3 of the plan and this record's first bullet) for what the docs call the
  sign-in endpoint, which is the APPLICATION's Initiate login URI (`updateInitiateLoginUrl`), left null. **Fixed in
  the environment over the MCP with the owner's go-ahead, one variable at a time, each read back:** (A) the external
  login URI cleared (`externalLoginUri: null`) — the chain ends at the hosted page, 200, "Sign in to Yokohama",
  "Continue with email"; (B) the Initiate login URI set (`url` = `https://yokohama.moore.nyc/enter`) — the chain
  unchanged, and a DIRECT visit to the hosted page's root now goes `/` 307 → `api.workos.com/user_management/
  initiate_login` 302 → `/enter` → a fresh start → the page, no loop. No code moved: the Worker's two halves behaved
  as written throughout (a callback never redirects to the issuer, so an HTTP loop could only be the issuer's doing).
  Read after the fix in the owner's Chrome: the lobby's state-A card, its link `/enter?return_to=%2F%23%2Flobby`, the
  branded page with the icon, the heading and the email field. **Still not read:** the email code and everything after
  it (the owner's checklist, item 1). The production API key's `lastUsedAt` was null before this read — no callback
  had ever reached an exchange.
- **A second finding from the same read (the ledger's D32) — a PREVIOUSLY RELEASED client storms `/api/me` under this
  Worker.** The owner's Chrome profile held the 0.6.3 shell in its service worker's cache
  (`yokohama-release-ef0f946478c75a51`) beside the new release's; a tab opened straight at `/#/lobby` ran it, and that
  client has no reader for `WWW-Authenticate: Yokohama-Session` (the string is absent from `v0.6.3`'s `api.ts`), so
  the Worker's caller-less 401 (R-44) took the layer's-own-401 path — `settle` fires `unauthorized`, `signedOut` →
  `leave` → `refresh` → `/api/me` again, the `leaving` guard holding only for a sign-out the player asked for:
  **1,000+ `GET /api/me` 401 in under a minute (the tracker's cap), about 80 a second, "Connecting to the server…" on
  screen and no update line within the first ten seconds.** The same URL under the new bundle (`index-D30P-rVA.js`,
  after a visit to the door applied the waiting release) made ONE `/api/me` and one `/api/health` and drew the sign-in
  card. The bundle name was not captured during the storm itself; the reading rests on the two caches, the mechanism
  in `v0.6.3`, and the new client's one request. Every invocation is paid, and nothing on the API bounds a caller-less
  read. Who it reaches: any player with the 0.6.3 shell cached who opens the lobby or a table link before the door —
  the Access allow-list today, everyone after stage 2. For the owner's ruling (the board): a per-IP bound on
  caller-less `/api/me` in the Worker (S3), or accepted as the transition's cost since the door applies the release.
  - **RULED 2026-10-20 (S3's brainstorm; the ledger's D32 `accepted` with this leg):** a bound INSIDE the Worker spares
    no invocation — a 429 the Worker answers is paid like the 401 it replaces — so the bound is the EDGE's: `/api/me`
    was added as a fourth leg to the zone's one rate-limit rule (ruleset `0264d3bd466c4fb8941513cba5a37d1f`, phase
    `http_ratelimit`, rule `0cbb21be3dc44896bb780926f35e4639`, version 4 → 5 at 04:22:40Z; 20 per 10 s by `ip.src` and
    `cf.colo.id`, mitigation 10 s, action block — the figures and the three other legs unchanged, read back), which
    turns a stale tab's ~80 a second into about one a second at the Worker and refuses the rest at zero invocations.
    Read live at once from the workstation over WARP: 25 `GET /api/me` → 21 × 401 (the Worker's challenge), then 429
    with `retry-after: 10` from the edge; `/robots.txt` 200 in the same minute. The new client reads `/api/me` once a
    page load, so a player never meets the leg. The hole is CLOSED only by each 0.6.3 browser opening the front page
    once (the door applies the release); the owner asks the two testers to. A Worker-side bound is struck from the
    options: it was never a quota answer.
- **The owner's sign-in, read the same afternoon (15:33–15:41Z): the rest of checklist item 1 but the table and the
  socket.** The sign-up by email code created the one production user; the Worker's API key `yokohama-worker` was
  used at the same second (its first use ever — the callback's exchange ran); three Magic Auth sessions from Chrome
  over WARP, each ended at a sign-out through `POST /api/leave` (WorkOS reads them `UserlandSessionRevoked`), the
  sign-out landing on the site's root. **Passkeys — the enrollment gap, read against the docs
  (`https://workos.com/docs/authkit/passkeys`):** a passkey is created at sign-up or by progressive enrollment, and
  the hosted page has no self-service screen for one afterwards; the docs word progressive enrollment for users
  "still using password-based authentication". Production has passkeys ON, passwords OFF and Magic Auth alone, and
  no AuthKit custom domain (the hosted page is `heroic-peak-11.authkit.app`; the docs want the custom domain BEFORE
  passkeys in production, since a passkey binds to the domain it was registered on). Tried with the owner's go-ahead:
  `isPasskeyProgressiveEnrollmentEnabled` ON (`updateAuthkitSettings`), a fresh hosted page, the owner's third
  sign-in at 15:40Z — **no prompt; straight to the lobby**; the sign-up at 15:33Z went from creation to session in 17
  seconds, so no enrollment step there either. So under this configuration an account can never create a passkey,
  and the passkey button a player sees at sign-in is the browser's side of WebAuthn with nothing to use. The paths
  that exist: a page of our own against the Widgets API (`registerPasskey`/`verifyPasskey`/`passkeys`/
  `deletePasskey`, a widget or access token the Worker would mint, elevated access on the writes — no ready widget
  covers passkeys; User Security does MFA and password, User Profile name, picture and email), or passkeys OFF until
  a custom domain exists. **The owner ruled the same hour: passkeys OFF** — `isPasskeyAuthEnabled` and
  `isPasskeyProgressiveEnrollmentEnabled` both `false` over the MCP (`updateAuthkitSettings`), read back; the hosted
  page fetched after it still renders the email form. Passkeys return with an AuthKit custom domain and an enrollment
  path, which is a cycle of its own.

**What stage 2's recipe does not claim:** nothing in it is READ yet. The Turnstile overlay's behaviour at the sign-in navigation, the WorkOS page's branding, two invocations per sign-in and the stranger's whole path are each a reading the window owes. The session cookie needs `https` or `localhost` (`README.md` §1.5), which the deployed host always is.

---

## 24. The red button — `YOKOHAMA_DENIED` (2026-10-19; a recipe, read once in the cells, not yet live)

**What it is.** One secret on the Worker naming the addresses, and the `@domains`, that are nobody here: a session
naming one is answered as a caller-less request is — the 401 with the sign-in challenge, the client's sign-in card,
and the cookie it sent cleared on that answer — and the sign-in's callback refuses the address after the exchange,
so WorkOS vouching for it mints nothing (`signin-refused`, reason `denied`; the address is never logged). Their
tables stay until their TTL or your expire (step 3). **The owner's ruling (2026-10-19, after stage 1's read):** a
red button for the spammer, the insulting address and the reported player, with no moderation workflow and no
reputation record; if abuse turns out systemic the mechanics are revisited against the real need. **A secret, never
a variable:** the list is personal data and the repository is public, so it enters no committed file (the deploy
guard's check 7 refuses it in any `vars` block), and `wrangler secret list` prints names only.

**Who it does not reach.** Under `access` the list is not read (Access names the caller). The anonymous archive is
untouched: a denied player's finished local game still files anonymously, as a stranger's does. A denied operator is
denied. The browser's passkey button, the WorkOS session and the WorkOS user record are not touched by it — the
deny-list is this site's; deleting the user at WorkOS is optional and does not keep them out, since sign-up is open
and the same address can be minted again, which is exactly what the callback's refusal is for.

**The recipe.** Entries one per line (or comma-separated), each an address or `@domain`; a trailing newline is fine
(empty entries are dropped). The value is read from stdin and never echoed; the file is yours to remove after.

```sh
cd worker
printf '%s\n' 'spammer@example.com' '@disposable.example' > /tmp/denied.txt
npx wrangler secret put YOKOHAMA_DENIED --config wrangler.jsonc < /tmp/denied.txt   # a put is live at once: it deploys a new version
npx wrangler secret list --config wrangler.jsonc                                    # the name among them; never a value
npx wrangler deployments list --config wrangler.jsonc | head -5                     # the new version id — the read-back that it took
rm /tmp/denied.txt
```

Cloudflare's own rule: `wrangler secret put` "creates a new version of the Worker and deploys it immediately" (only a
Worker on gradual deployments needs `wrangler versions secret put` and a `versions deploy`; this one is not). If a
put ever fails to take here, `npm run deploy` of the current tree carries it. **A malformed entry is SKIPPED, never a refusal** — unlike `YOKOHAMA_WEB_ADMINS` (§3's first bullet), whose
malformed entry is the 500: here the log line `denied-skipped` names the entry's position among the entries kept
and never its value, and the rest stand (the owner's ruling on a side agent's note, 2026-10-19: a typo in the ban
list must never be an outage). Read Workers Logs after a put: the configuration is read on every request, so a
malformed entry left standing logs `denied-skipped` once per request until the list is put again — an entry skipped
is an entry that denies nobody, so put the list again, corrected. The list reaches standing sessions too: an entry
added denies one at its next request, and one removed re-admits a session that still stands. An `@domain` entry denies that exact domain and no subdomain — list a subdomain as
its own entry. To clear the button: `npx wrangler secret delete YOKOHAMA_DENIED --config
wrangler.jsonc`, or a put with the entries you keep.

**Reading that it works.** The denied player's next request is the 401; in Workers Logs a sign-in they attempt reads
`signin-refused` with `reason: denied` and no address. The cells pin the rest (`test/signin.test.ts`, "the red
button"; `src/config.unit.test.ts`, the deny-list's parse).

**Their tables (step 3).** As the listed operator, with your session cookie (the operator playbook's `workos` note
above §"The three calls"): list the tables, expire theirs — `POST /api/admin/tables/<id>/expire` archives the table
first and ends it, 204; `?force` drops a save that cannot be made. Players still seated receive the ended signal.

**An automation worth having** (the owner, 2026-10-19): a skill that walks an agent through the sign-in in a
browser, takes the cookie from the owner, and runs the three calls — recorded here, not built.

## 25. The thirty-seventh deployment — `0.7.1`, the absolute session cap and the red button (2026-10-19, a record)

- **The code:** `security-auth` at `f47909d4` — the change `24728653`, the bump `617377af`, the review's fix round
  `76a7052e`, the re-review's docs round `f47909d4`; the Worker's TypeScript and module are `76a7052e`'s, since the
  last commit moved documents alone. The design's R-16 and §4.2 are amended in place; the plan's Review record carries
  the review (Fable, scoped, on an export: 0 Critical, 4 Important, 10 Minor, 21 of 25 mutants red), the fix round
  (seven of seven mutants red) and the re-review (nine of nine, docs only). The owner's rulings of the afternoon:
  gap 1 and gap 2 built now; a malformed deny-list entry skipped with a log line, never a refusal.
- **What changed for a player:** the session cookie is `v2` with `first`, capped at 90 days from its first issue
  (`YOKOHAMA_SESSION_MAX_DAYS`, absent — 90 applies), its `Max-Age` the life it has; every `v1` cookie reads as
  nobody, so the one signed-in account (the owner's) meets the sign-in card and signs in once more. **The red button
  `YOKOHAMA_DENIED` is wired and NOT SET** — the secret does not exist on the Worker (`wrangler secret list` reads
  four), so nobody is denied: armed, unpressed; §24 is the recipe.
- **The build (~17:20Z):** `cd web && npm run build` with the SIX values read from the stage-1 bundle (the project
  link, the blurb, `Not affiliated with Asmodee`, the site URL, the feed derived, the sign-in notice) and no site key:
  the release `yokohama-release-8769ddacec481321`, 153 precache entries, 10,093,133 B, the module 1,250,424 B
  (`wasm-opt 133`), the stamp `signin_notice: true`, `turnstile_sitekey: false`. `cd worker && npm run build`:
  `factory-glue: 18 exports (a release module: no debug_trap)`, `assemble-assets: 153 precache entries verified`
  (162 files, 10,132,917 B), `smoke-release: … GET /api/health 200: {"server_version":"0.7.1", …}`.
- **The deploy:** `npm run deploy` — the deploy guard's door line `wrangler.jsonc deploys the worker door, source
  workos — WorkOS AuthKit, through its first-party API at the callback`; the secrets guard `4 secrets on
  wrangler.jsonc's Worker, none named like a switch, and the workos arm's YOKOHAMA_OIDC_CLIENT_SECRET and
  YOKOHAMA_SESSION_SECRET among them`; 6 assets uploaded (154 already there, `index-D32RRaUa.js` among the new);
  **Worker version `d78b48f8-20a1-4e39-a1ac-c723073d094d`**. Both wrangler files unchanged since stage 1 (source
  `workos`, the client id, `YOKOHAMA_TURNSTILE` off; no days or cap variable, so 30 and 90 apply). Access still in
  front of the hostname and the edge rule still disabled: stage 2 (§23) untouched.
- **The readings, from the owner's workstation over WARP with the browser UA:** `/api/health` 200 `server_version
  0.7.1` (`identity cf-access`, `archive r2`, the TTLs 90 and 7); `/enter` 302 to the WorkOS authorize URL with
  `__Host-yokohama_login=v2.…; Max-Age=600`; the owner's Chrome at `/#/lobby`: `/api/me` 401 with
  `Yokohama-Session realm="/enter"` and the "Sign in to play multiplayer" card — the service worker still serving
  the stage-1 bundle until a door visit applies the new release. **Not read:** the owner's new `v2` session end to end
  (the sign-in, the cookie's `first`, a renewal's `Max-Age`); the cap at its 90 days, by its nature; the button
  pressed (no secret put). The e2e's one red (the whole-game spec on the open door's `/api/leave` 404 — the auth
  cycle's Task 6; the board's rulings outstanding 12) is a test-fence matter, not this deploy's.
- **The suites at `f47909d4`** (the Rust crates byte-identical to `0.7.0` but the version): `cargo test` web 559,
  wasm 10, worker 81; clippy clean; web typecheck clean, vitest 1,553 in 74, `test:wasm` 391 in 16, `npm run e2e`
  80 of 81 (above); worker typecheck clean, `test:scripts` 794 in 32, the pool 298 in 14 over the debug module. The
  roster was NOT run: no Rust logic moved; the lane's §2 names the three web-lane crates for a version move, which
  ran. The tag `v0.7.1` on `f47909d4`, pushed with the branch.
- **Rollback:** `git revert f47909d4 76a7052e 617377af 24728653` is the full undo of the code; the cheaper one is a
  redeploy of stage 1's tree (`18d63f28`'s build) with `npm run deploy` — every `v2` cookie then reads as nobody in
  turn, one re-sign-in each way. The secrets are untouched either way; the deny-list, if ever put, stays put and is
  read by nothing at stage 1's code.


## 26. The private test Worker — `yokohama-web-private` at `yokohama-test.moore.nyc` (2026-10-20, security S3 Task 7; a recipe and a record)

- **The recipe, in the design's order (D-9: never a minute where the hostname answers a stranger).** (1) Access FIRST: the hostname is a destination of the one `yokohama` application (`11cbcf68-96cd-4310-baee-f2ade812155d`), read back over the API — `self_hosted_domains` and `destinations` both list it and the `aud` is the file's `ACCESS_AUD`; on a refusal or an API error steps 2–6 are SKIPPED and the deploy is taken up after the close by this recipe (R-S3-12). (2) The file's `routes` and the guard's record (`DEPLOYMENTS` in `scripts/deploy-guard.mjs`) name the hostname (`4ef280da`). (3) The bucket: `npx wrangler r2 bucket create yokohama-archive-private`, then production's rule `npx wrangler r2 bucket lifecycle add yokohama-archive-private expire-test-after-30-days test/ --expire-days 30`, read back with `npx wrangler r2 bucket lifecycle list yokohama-archive-private` (the default multipart-abort rule and the `test/` rule). (4) The private Worker's OWN keyless build: `cd web && YOKOHAMA_PROJECT_URL=… YOKOHAMA_SITE_BLURB='…' YOKOHAMA_SITE_NOTICE='Not affiliated with Asmodee' YOKOHAMA_SITE_URL=https://yokohama-test.moore.nyc npm run build` — the project link, blurb and notice as production's, the site URL THIS host's (§1: each host serves its own client), the feed derived, NO `YOKOHAMA_SIGNIN_NOTICE` (its sign-in is Access's, so the WorkOS sentence would be false here) and no site key — then `cd worker && npm run build`. (5) The FIRST deploy by hand (S2b's D-7): `node scripts/deploy-guard.mjs --config wrangler.private.jsonc && npx wrangler deploy --config wrangler.private.jsonc` — the deploy guard runs, the secrets guard is skipped (a Worker that does not exist holds no secret); every later deploy is `npm run deploy:private`, both guards. (6) The readings over WARP as the owner, below.
- **The Access application now lists FOUR hostnames** — `yokohama.moore.nyc`, `yokohama-cli.moore.nyc`, `yokohama-nas.moore.nyc`, `yokohama-test.moore.nyc` — and a PUT replaces the whole list: every recipe that writes the lists (§19's PUT bodies, §23's steps 6 and 9, the lane's §10 step 7 and its stage-2 paragraph) keeps the test host from this day on; the forward notes sit at those sites.
- **The record (2026-10-20).** Access read back at 04:41:55Z after the owner's dashboard change and again at 10:0x Z before the deploy (four hostnames in both lists, `aud` `5fa7a989…`, session 24h). The code half `4ef280da` on `security-s3` (its review: 0 Critical, 0 Important, 6 Minor). The bucket created at 10:5x local with both rules. The build 10:08:10–10:08:39Z: `wasm-opt 133`, 1,901,149 → 1,257,076 B (−33.9 %), the precache 153 entries / 10,100,845 B (module 1,257,076 · bundle 492,437 · fonts 98,728 · shell 14,079 · icons 19,456 · help 764,742 · art 7,454,327), **the release `yokohama-release-b41736f304b7d374`**, `sw.js` 31,615 B; `worker`: the module 1,995,195 → 1,323,063 B (−33.7 %), `factory-glue: 18 exports (a release module: no debug_trap)`, `assemble-assets: 153 precache entries verified` (162 files, 10,142,709 B), `smoke-release` 200 at `server_version 0.7.1`; the stamp `signin_notice: false`, `turnstile_sitekey: false`, `site_url: true`, and the sitemap names the test host alone. The deploy 10:15:29–10:15:41Z: the guard's door line `wrangler.private.jsonc deploys the worker door, source access — Cloudflare Access, verified by the shell`; 160 assets (2.62 s), 1,505.12 KiB (gzip 568.15 KiB), startup 2 ms, the bindings and vars as the file names, the trigger `yokohama-test.moore.nyc (custom domain)`, **version `251dba8b-152b-4a7c-a67a-abf07cce483e`**. The readings over WARP: `/api/health` 200 `{"server_version":"0.7.1","semantics_rev":7,"identity":"cf-access","signal":"ws","archive":"r2","table_ttl_days":90,"open_table_ttl_days":7, …}`; `/api/me` `{"mode":"cf-access","user":"<the owner's address>"}`; `/` 200 with the enforced header set (the CSP with its nonce, COOP, permissions, referrer, nosniff, HSTS); `/api/health` `cache-control: no-store`; `sw.js` naming `yokohama-release-b41736f304b7d374`; `/api/stream` upgraded (101); a 2P table against the bot created (201), dealt, a card kept (playing), one move made (`Red: End additional-action phase`, the log's entry 0), then abandoned (204) into the private archive (the table's 404 names it). **Owed to the owner:** the off-WARP reading — a device off WARP meets `bdvc.cloudflareaccess.com`'s login at the hostname.
- **DNS, a LAN trap.** Cloudflare's resolver (1.1.1.1) answered the new name within a minute of the deploy (104.21.92.62 / 172.67.187.23, production's anycast pair); a LAN resolver authoritative for `moore.nyc` (the NAS's split horizon) answers NOERROR with no record until it is given one, so the workstation read the site with `curl --resolve yokohama-test.moore.nyc:443:104.21.92.62`; WARP-routed DNS is unaffected. A local record for the test host is the owner's checklist item.
- **Rollback.** `npx wrangler delete --config wrangler.private.jsonc` removes the Worker and its custom domain; the bucket and the Access destination stay, harmless. The private Worker runs `0.7.1` until the 0.7.2 window's `npm run deploy:private` (§27).

## 27. The thirty-eighth deployment — `0.7.2`, security S3 (2026-10-20, a record)

- **The code:** `security-s3` at `4dc9a7b5` (the design `docs/plans/2026-10-19-security-s3-design.md` §7.4–§7.5; the release commit `92703601`; the tag `v0.7.2`, published by `release.yml` run 39414383669 at 11:29:03Z before either deploy). What changed for the Worker: the password throttle inside `Games` (D2 — twenty wrong passwords per table and caller per ten seconds, 429 in one sentence, no header), the creator's reopen route at `Open` only with the client's control and the dealing screen's Abandon (D9), a dealing table expiring on the waiting-table TTL (D9; the Worker ends the unread corner by design, D-16), the service worker's typed `challenged` to one window elected by LOAD address and the door's one automatic pass per page behind its own route check (W-3; the live reading of the pass itself is the public day's, D-17). The NAS went first (`deploy/RUNBOOK.md` §3.23).
- **The private Worker first** (§26's recipe, every later deploy through both guards): its own keyless build at 0.7.2 (the release `yokohama-release-b5fa1fc286de3e2e`, 153 precache entries, `smoke-release` 200), `npm run deploy:private` through BOTH guards (`secrets-guard: 0 secrets … none named like a switch`) at 11:32:06–11:32:18Z — 6 assets uploaded (154 already), **version `cbb16f3b-0013-4d55-afd4-00e8c030e705`**; read over WARP: `/api/health` `server_version 0.7.2` (identity cf-access, archive r2), `/sw.js` the release; **D2 on the Worker** at a locked table `sim-22a87adc3076d26e`: 21 wrong passwords → 20 × 403 then 429 with the sentence, `no-store`, no `retry-after`; **D9 on the Worker**: the reopen arms (an open seat 409 NO_JOINER; the creator's own seat 409 OWN_SEAT; seat 9 400); abandoned 204.
- **Production:** the six-value build (the sign-in notice, `YOKOHAMA_SITE_URL=https://yokohama.moore.nyc`, no key) — the release **`yokohama-release-18059a9aeb00abae`**, 153 entries / 10,101,405 B, `smoke-release` 200 at 0.7.2; `npm run deploy` through both guards (the door line `source workos — WorkOS AuthKit`; `4 secrets … the workos arm's YOKOHAMA_OIDC_CLIENT_SECRET and YOKOHAMA_SESSION_SECRET among them`) at 11:33:40Z — 6 assets uploaded (154 already), **version `59cb8e19-9358-4ee1-a1a5-fa8330403d9e`**; read over WARP: `/api/health` 200 `server_version 0.7.2`, `no-store`; `/sw.js` the release; `/api/me` without the Worker's session answers `no signed-in user — sign in at /enter`. **Not read on production:** D2 and D9 through the API (they need the Worker's own session — the owner's browser read; the identical module was read on the private Worker above), and W-3's pass (the public day's, D-17).
- **Rollback:** `wrangler rollback` to the previous version (`0.7.1`, §25) on either Worker; the code's undo is `git revert` of the branch's commits.
