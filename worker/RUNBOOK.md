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
  - any method but GET or HEAD on `/enter` or `/api/art`: **405**, empty; `/enter`: **302** to
    `/#/lobby`;
  - a body over 2 MiB: **400** `the body could not be read: length limit exceeded` — the NAS's own
    sentence, read off it on 2026-09-23;
  - a bad variable: **500** naming it (§3);
  - **a WebSocket upgrade from another site's page — its `Origin` present and not
    `https://yokohama.moore.nyc`: 403 `this socket is for the site's own pages`**, unupgraded. A
    WebSocket is outside CORS, so without it a hostile page in a signed-in player's browser could
    read a table's turn and timing frames (Task 11's review, escalation 1). A request with no
    `Origin` — curl, the deploy readings — passes;
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
    table was removed by an operator on 2026-10-22` — the UTC date of the delete. The tails: `— its
    game is in the archive` where its game was saved — stamped at game over, or saved by the delete
    itself, which with the switch at `r2` saves every game before it deletes it — whatever the
    switch reads now (the orchestrator's ruling I-71); `— no move had been made, so there was
    nothing to keep` where there was no game; and `— this server keeps no archive, so its game was
    not kept` for a game never saved, deleted while the switch reads `off`. After the 90 days, once
    the lobby has acknowledged the removal, the object ends and the id reads the layer's own `no
    game named …`;
  - the operator surface (spec §2.9; §3 below): **403** `this address is not an operator here`;
    **400** `?force takes no value — send ?force alone, or no query for an expire that saves first`
    for a force given any value, `?force=false` among them (the orchestrator's ruling I-72); and a
    force's **503** `the lobby has not confirmed the removal — the alarm will finish it`;
  - a trapped engine: **500** `the table's engine stopped — try again`, `the lobby's engine stopped
    — try again` or `the archive's engine stopped — try again`; a table whose stored document traps
    twice: **500** `this table's stored <kind> cannot be loaded`;
  - **an object whose request failed on an error that is not a trap** — a reply the shell cannot
    read, a batch that did not commit, an object reset under `blockConcurrencyWhile`: **500** `the
    server could not answer — try again`, one sentence for the table, the house and the archive
    alike, logged `object-failed` (the orchestrator's F-1). Before it, the platform's own plain 500
    answered, which the client cannot read as `{"error": …}`;
  - a deleted table's sockets: `end {"reason":"deleted"}`, then a **4404** close whose reason is
    `this table was deleted`.

## 1. Build

Prerequisites: the pinned toolchain with its `wasm32-unknown-unknown` target, `wasm-bindgen-cli
0.2.128` and binaryen's `wasm-opt` (the same three as `deploy/RUNBOOK.md` §1's client build), and `npm ci` once in both
`web/` and `worker/`.

```sh
cd web && npm run build          # the RELEASE browser module, and web/dist
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
   guard compares with the current `web/dist/sw.js`.
3. **`smoke:release`** — the release module instantiated once, in Node: `init` with the archive off,
   then `GET /api/health` as `smoke@x.test`. It refuses unless the answer is 200, and refuses a debug
   module. Nothing else runs the release module before a deploy.

**The order matters twice.** `web/`'s `npm run e2e` leaves a DEBUG browser module in `web/dist`, so
`web/`'s `npm run build` comes first, every time. And `worker/`'s own tests run the DEBUG module:
after a release build, `npm run build:module:debug` comes before `npm test` in `worker/`.

## 2. Deploy

```sh
cd worker && npx wrangler whoami   # the owner's login, account BDVC
npm run deploy                     # node scripts/deploy-guard.mjs && wrangler deploy
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

- **A deploy restarts every object and closes every socket with 1006** (spec §5.5). The client
  probes once and reconnects, an open picker is re-fed — or dropped, when the deploy changed the
  engine's, the CLI's or the layer's sources (`picker-dropped`, §5) — and no acknowledged move is
  lost; but it is still a reconnect for everyone at every table, so deploy between games.
- **The migration `v1` is FROZEN from the first deploy on** (the orchestrator's ruling I-30): a
  class added later takes a new tag. Cloudflare's own docs now prefer a declarative `exports` field
  for NEW Workers, and keep the `migrations` array this Worker uses fully supported; the two may not
  be mixed.

## 3. The variables — every one, and what its absence means

`wrangler deploy` REPLACES the Worker's variables with `wrangler.jsonc`'s `vars`, so that file is
the whole list; it leaves secrets alone.

| Variable | At the seventeenth deployment | Absent means |
|---|---|---|
| `YOKOHAMA_WEB_IDENTITY` | *not yet a variable (added 2026-09-28, M6.5c2):* the door, `worker` in `wrangler.jsonc` since M6.5c2 (R-6.5c2.5), first deployed at that cycle's close; `open` is the other door, which no Worker of this repository runs | there is no default: the API answers the host-written 500 naming it, as it answers any bad value (the first bullet below), and the static release is still served; the deploy guard refuses a `wrangler.jsonc` that names no door |
| `ACCESS_TEAM_DOMAIN` | `bdvc.cloudflareaccess.com` | no assertion can be verified: every caller is no one, and no key is fetched |
| `ACCESS_AUD` | the `yokohama` application's audience | the same |
| `DEV_DOOR_CLIENT_ID` | the `yokohama-worker` credential's client id, its public half. *(Corrected 2026-09-28, M6.5c2: a Worker SECRET since M6.5c2, put at that cycle's close in the same deploy that drops the variable, and refused in `wrangler.jsonc`'s `vars` by the deploy guard; it was a committed variable until then, and stays in the history.)* | the dev door is shut, whatever `YOKOHAMA_DEV_DOOR` says |
| `YOKOHAMA_DEV_DOOR` | `on`, for this cycle's live tests | off: an assertion naming only the credential is no one |
| `YOKOHAMA_WEB_ARCHIVE` | `r2`, over the bucket `yokohama-archive` | off (`r2` needs the bucket bound as `ARCHIVE`) |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | `90` | no table ever expires |
| `YOKOHAMA_WEB_ADMINS` | set, as a SECRET (`wrangler secret put`, the plan's ruling P-1), its value never shown — a refusal names an entry by its position, never its value; the deploy guard refuses `wrangler.jsonc` if its `vars` name it | the operator routes are the layer's 404 |
| `YOKOHAMA_WEB_COUNT_WAKES` | unset — the exit's window sets it for one deploy, and the next deploy unsets it (§7) | off: no wake is counted |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `on` | on |
| `YOKOHAMA_WEB_CHAT_KEEP` | `500` | 500 messages |
| `YOKOHAMA_WEB_CHAT_TEXT` | `500` | 500 characters |
| `YOKOHAMA_WEB_TEST_CLOCK` | **never set in production** — the pool's files only, `wrangler.test.jsonc` and, since M6.5c2, `wrangler.open.test.jsonc` *(corrected 2026-09-28)*; the deploy guard refuses `wrangler.jsonc` if it names it | the wall clock |

- **A bad value answers 500 naming its variable** (spec §2.6), the Worker's form of the NAS refusing
  to start: a switch other than `on` or `off`, an archive other than `r2` or `off`, `r2` with no
  bucket bound, a TTL that is not a whole number of days, an operator entry that is not an address,
  or any value written without quotes. The static release is unaffected, and so are a few `/api/*`
  answers, which is where the rule is not "every `/api/*`":
  - two answers come before any configuration is read: 405 for any method but GET or HEAD on
    `/api/art`, and 403 for an upgrade from another site's page (the list at the top of this runbook);
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
  something on both sides, no spaces or control characters, at most 254 bytes — or it is no one,
  and the layer's 401 answers (R1's L-2). A claimed `not-an-email` once named an Archive object
  whose `init` refused it, a 500 no spec lists.
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
    or one for other work — logs `archive-owed-kept`. Set back to `r2`, the table's next request or
    alarm resumes it.
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
- **To keep every table** (R-6.5c.15): remove `YOKOHAMA_WEB_TABLE_TTL_DAYS` from `wrangler.jsonc`'s
  `vars` and deploy (§2). With it absent no table expires and no expiry deadline is armed; the
  markers and the operator surface work as before.

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

**The three calls**, over WARP as the operator:

```sh
B=https://yokohama.moore.nyc
curl -s "$B/api/admin/tables" | jq                                              # the listing
curl -s -o /dev/null -w '%{http_code}\n' -X POST "$B/api/admin/tables/<id>/expire"         # expire: 204
curl -s -w '\n%{http_code}\n' -X POST "$B/api/admin/tables/<id>/expire?force"            # force: 204 or 503
```

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
  since M6.5c2, put at that cycle's close in the deploy that drops the variable, from `.dev.vars`
  through a mode-600 file that is deleted after. *(Corrected 2026-09-28, M6.5c2: `wrangler.jsonc`
  carried the client id as a variable until then.)* The secret is in no other file, and is never
  printed, committed or pasted.
- **What it does.** A request presenting both headers gets an assertion whose `common_name` is the
  client id and which names no `email`, and Service Auth outranks a WARP identity (spec §0.3 row 1).
  With `YOKOHAMA_DEV_DOOR` on, such a request names its caller in `X-Yokohama-Claimed-User`; without
  that header it is no one. Access also sets a `CF_Authorization` cookie, which carries the
  credential on later requests from the same browser.
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
  curl -s -H @"$H" https://yokohama.moore.nyc/api/health
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

- **Health**, over WARP as the owner: `curl -s https://yokohama.moore.nyc/api/health`. It reads
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
    next click is a 409. So after the first deployment past the eighteenth, every picker open at it
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
    (`POST /api/archive`, where `id` is the archive object's name, `archive:<address>`).
    `detail` is the record's key, or for `failed` R2's message — or the host's own sentence
    `<record_key> refused a create as present, then read as absent`, when the record's
    create-if-absent put was refused and the record then read as absent. `attempt`, at the
    game-over site, counts this save's earlier tries that did not complete it, and at the expiry
    site the table's earlier expiries that did not end it. A failed
    game-over save is retried on the backoff — 30 s, 2 min, 10 min, then hourly — by the alarm, and
    by the table's next request.
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
    `receipt`, `stamp_archive` or `read`; in an archive object, `receipt` or `dispatch`.

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
  - **`operator`** `{id?, action, address}`: every operator action — `list`, logged by the house
    with no `id`, `expire` and `force`, logged by the table — with the operator's address.
- **The bucket**, through wrangler's own login: `npx wrangler r2 object get
  "yokohama-archive/<key>" --remote --file <path>`. A pair is `rev7/{over,abandoned}/<id>.json` and
  `.table.json`, and a browser's save sits under `browser/<caller_segment>/`. A game whose table's
  creator, or a browser save whose sender, has an address ending `.test` is keyed under the root
  `test/` instead, apart from the owner's paths (spec §2.6).
- **The Origin check**, over WARP: an upgrade from the site's own origin answers 101, one from
  another answers 403 with the host's sentence (reading 2d).

  ```sh
  WS=(-H 'Connection: Upgrade' -H 'Upgrade: websocket' -H 'Sec-WebSocket-Version: 13' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==')
  curl --http1.1 -s -i -N --max-time 5 "${WS[@]}" -H 'Origin: https://yokohama.moore.nyc' https://yokohama.moore.nyc/api/stream | head -1
  curl --http1.1 -s -i --max-time 5 "${WS[@]}" -H 'Origin: https://evil.example' https://yokohama.moore.nyc/api/stream | sed -n '1p;$p'
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

## 7. The live tiers and the quota guard

Two tiers read the DEPLOYED Worker, and neither is a gate: CI runs neither, and each plays on the
owner's account, spends its quota, and leaves rows and saves behind. They are the operator's to run —
the orchestrator's this cycle (Task 15) — from the owner's workstation, over WARP, with the
credential in the repository root's `.dev.vars` (§4), whose values neither prints.

- **The live Playwright tier** — two specs, `two_browsers` and `joins_by_link`, played at
  `https://yokohama.moore.nyc` (`web/e2e/live.ts` holds its rules):

  ```sh
  cd web && YOKOHAMA_WEB_E2E_TARGET=live npx playwright test two_browsers joins_by_link
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
  - in both modes, the deal's DESIGNED 409 on `GET …/api/games/<id>/deal` — a table that flipped to
    playing between two of the browser's requests, the refusal the dealing screen branches on — in
    both of its arms, the failed response and its console line (I-81), counted as `dealAnswers`.

  Every other status of 400 or more, and every other console error, is breakage.
- **The live corpus replay** (`worker/scripts/live-replay.mjs`, spec §5.3's live half): the walk the
  committed corpus was cut from, sent to the Worker as the walk's own `.test` identities, every
  status and every named body compared with `web/src/wire/`:

  ```sh
  cd worker && node scripts/live-replay.mjs   # and again, once the first has finished
  ```

  **Twice, one after the other**: the second run is the repeatability evidence, its two first sends
  answering the once-rule's 200. **Never beside the Playwright tier**: the two share the house
  chat's `seq`. It forces its three tables before and after itself with no credential, so over WARP
  the owner's own login is the operator (§3).
- **Bot Fight Mode's 403.** The zone's Bot Fight Mode and JavaScript detections — ON, by the owner's
  decision — answer a bot-shaped request BEFORE Access sees it: 403, `cf-mitigated: challenge`, and
  a `text/html` "Just a moment…" body of about 5,259 B (§8, reading 1). Both tiers print a body
  that is not the API's JSON error by its type and size, never its text, so a 403 that reads as a
  `text/html` body of about 5,259 B is the zone's, and the cure is to run over WARP.

### The exit's window: counting wakes

The exit's headed game (`web/e2e/exit_game.spec.ts`, run as its header says) reads each wake off
`wrangler tail` as the `wake {id, wakes}` line (§5), which the objects log only while
`YOKOHAMA_WEB_COUNT_WAKES` reads `on` (the orchestrator's ruling I-84). `wrangler.jsonc` leaves it
unset (§3), so the window is two deploys, run in `worker/`, with no file edited:

```sh
node scripts/deploy-guard.mjs && npx wrangler deploy --var YOKOHAMA_WEB_COUNT_WAKES:on   # opens it
npm run deploy                                                                        # closes it
```

- **Opening it is a deploy**, guarded like any other: the guard runs first, as `npm run deploy`
  runs it, and `--var` deploys one more plain-text variable beside `wrangler.jsonc`'s `vars`,
  replacing none of them (read in wrangler 4.136.3's source). Like every deploy, it restarts every
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
grouped by `$workers.outcome` and `$metadata.type`, or `view: "events"` with filters.

### What a live run leaves on the account

- **Two 90-day MARKERS per Playwright run**, at fresh `sim-` ids: each spec abandons its table in
  its `afterEach`, and the marker answers every request there for 90 days, then ends on its alarm.
- **Saves under the bucket's `test/` root**: `test/rev7/over/` and `test/rev7/abandoned/` pairs for
  the `.test` identities' finished and abandoned games. No lifecycle rule removes them yet: the
  owner ruled a 30-day rule on the prefix (R-6.5c2.12), set at the M6.5c2 close. *(Corrected
  2026-09-28: this said the choice was the owner's, carried to the cleanup cycle.)*
- **The replay's own saves**: its two first sends' pairs,
  `test/browser/red_at_wire.test/rev7/over/wire.json` and
  `test/browser/red_at_wire.test/rev7/abandoned/wire.json`, each with its `.table.json` — kept for
  good, by the owner's direction — and the walk's own game-over pair under `test/rev7/over/`.
- **A PERMANENT house-chat post per replay run**: Red (`red@wire.test`), "anyone up for a second
  table?", which real people see in the lobby until 500 newer messages trim it.
- While a Playwright run lasts, the live lobby lists its tables, each with a long name and a
  password.

### The replay's reset

Every run after the first meets the saves its two first sends left, and reads the once-rule's 200.
A `wire` record whose bytes have changed since — a version bump, a walk that applies other moves, a
record format change — meets them as a **409** instead. The replay then stops, names the cause, and
prints the reset (`resetRecipe`, R3's M-1), to run in `worker/` with wrangler's own login, since the
owner's R2 key is read-only (spec §7) — and then the replay again:

```sh
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/over/wire.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/over/wire.table.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/abandoned/wire.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/abandoned/wire.table.json" --remote
```

The walk's own game-over pair under `test/rev7/over/` blocks nothing, since no payload reads it; a
later game over at that id with other bytes meets it and logs `archive-conflict`.

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
