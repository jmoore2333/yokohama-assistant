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
    table waited 7 days for players and was closed on 2026-10-22` (M6.5c2 cycle E, R-6.5c2.42: a
    table still waiting for players when its alarm closed it), `this table was removed by an
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
# the RELEASE browser module, and web/dist; the door's three build values are read
# here when set (checked by web/scripts/build-values.mjs, a bad one fails the build
# naming it; at most 200, 400 and 200 characters; unset or empty draws nothing)
cd web && YOKOHAMA_PROJECT_URL=https://example.org/yokohama YOKOHAMA_SITE_BLURB="A few words." YOKOHAMA_SITE_NOTICE="A notice." npm run build
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
| `YOKOHAMA_WEB_IDENTITY` | *not a variable at the seventeenth; added 2026-09-28 (M6.5c2) and first deployed at the twenty-first (§15):* the door, `worker` in `wrangler.jsonc` (R-6.5c2.5); `open` is the other door — no deployed Worker runs it outside a door WINDOW (`wrangler.open.jsonc` deployed it for twelve minutes on 2026-09-30, §19's state 1, reverted the same hour) | there is no default: the API answers the host-written 500 naming it, as it answers any bad value (the first bullet below), and the static release is still served; the deploy guard refuses a `wrangler.jsonc` that names no door |
| `ACCESS_TEAM_DOMAIN` | `bdvc.cloudflareaccess.com` | no assertion can be verified: every caller is no one, and no key is fetched |
| `ACCESS_AUD` | the `yokohama` application's audience | the same |
| `DEV_DOOR_CLIENT_ID` | the `yokohama-worker` credential's client id, its public half. *(Corrected 2026-09-28, M6.5c2: a Worker SECRET since the twenty-first deployment (§15), put in the same deploy that dropped the variable, and refused in `wrangler.jsonc`'s `vars` by the deploy guard; it was a committed variable until then, and stays in the history.)* | the dev door is shut, whatever `YOKOHAMA_DEV_DOOR` says |
| `YOKOHAMA_DEV_DOOR` | `on`, for this cycle's live tests | off: an assertion naming only the credential is no one |
| `YOKOHAMA_WEB_ARCHIVE` | `r2`, over the bucket `yokohama-archive` | off (`r2` needs the bucket bound as `ARCHIVE`) |
| `YOKOHAMA_WEB_TABLE_TTL_DAYS` | `90` | no table ever expires |
| `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` | *not a variable at the seventeenth; added 2026-09-30 (M6.5c2 cycle E, R-6.5c2.42):* `7` in `wrangler.jsonc` — how long a table still waiting for players stands (below) | 7 days; and with `YOKOHAMA_WEB_TABLE_TTL_DAYS` absent it is inert, since no table expires |
| `YOKOHAMA_WEB_ADMINS` | set, as a SECRET (`wrangler secret put`, the plan's ruling P-1), its value never shown — a refusal names an entry by its position, never its value; the deploy guard refuses `wrangler.jsonc` if its `vars` name it | the operator routes are the layer's 404 |
| `YOKOHAMA_WEB_COUNT_WAKES` | unset — the exit's window sets it for one deploy, and the next deploy unsets it (§7) | off: no wake is counted |
| `YOKOHAMA_WEB_LOBBY_CHAT` | `on` | on |
| `YOKOHAMA_WEB_CHAT_KEEP` | `500` | 500 messages |
| `YOKOHAMA_WEB_CHAT_TEXT` | `500` | 500 characters |
| `YOKOHAMA_TURNSTILE` | *not a variable at the seventeenth; added 2026-10-01 (M6.5c2 cycle F, R-FP2, R-FP4):* a DEPLOY DECLARATION, `off` in `wrangler.jsonc`, the gate's file and the pool's two, `on` in `wrangler.open.jsonc` alone. It rides into the Worker's variables like every `vars` entry, and nothing in the Worker reads it but the check that every variable is a string. The deploy guard's check 12 and the smoke's live mode read it (the bullet below) | off: the guard refuses a keyed build beside it, and the smoke's live mode owes no edge line |
| `YOKOHAMA_WEB_TEST_CLOCK` | **never set in production** — the pool's files only, `wrangler.test.jsonc` and, since M6.5c2, `wrangler.open.test.jsonc` *(corrected 2026-09-28)*; the deploy guard refuses `wrangler.jsonc` if it names it | the wall clock |

- **A bad value answers 500 naming its variable** (spec §2.6), the Worker's form of the NAS refusing
  to start: a switch other than `on` or `off`, an archive other than `r2` or `off`, `r2` with no
  bucket bound, a TTL — either — that is not a whole number of days, an operator entry that is not an address,
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
- **The waiting-table TTL** (M6.5c2 cycle E, R-6.5c2.42): a table still WAITING for players — its
  status `open`, a seat unfilled — is closed after `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` days without
  activity, `7` in `wrangler.jsonc` and by default, with the marker head `this table waited 7 days
  for players and was closed on …` and the tail `— no move had been made, so there was nothing to
  keep`; its `expired` line reads `reason: "waited"`. No write can read a table's status (the
  shell never parses a table document), so **every table's alarm is armed at its last activity
  plus the SHORTER of the two TTLs** — 7 days — and the alarm decides by the status its house copy
  reads: an `open` table is closed; any other is re-armed to its last activity plus
  `YOKOHAMA_WEB_TABLE_TTL_DAYS`, and expires there as above. So a dealing or playing table idle for 7
  days costs one alarm run more than before — its house copy made once more, on a module of its
  own, to read the status — and the alarm set again. A table
  already waiting when this is first deployed keeps the deadline armed before it, its last activity
  plus 90 days, until a delivery to the lobby or an alarm for other work re-arms it — as with a
  lowered TTL, above. `/api/health` reports both TTLs, `table_ttl_days` and `open_table_ttl_days`
  (both `null` while no table expires; the NAS reports `null` for both, since it keeps no TTL).
- **To keep every table** (R-6.5c.15): remove `YOKOHAMA_WEB_TABLE_TTL_DAYS` from `wrangler.jsonc`'s
  `vars` and deploy (§2). With it absent no table expires and no expiry deadline is armed, whatever
  `YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS` reads; the markers and the operator surface work as before.
- **`YOKOHAMA_TURNSTILE`: what `on` means, and what it does not** (M6.5c2 cycle F, R-F6, R-FP2, R-FP4,
  R-FP7; spec §2.6). One client build per door state: production, behind Access, ships a client built
  WITHOUT the Turnstile site key and declares `off`; the open window's file ships a client built WITH
  it and declares `on`. **`on` means** that this deploy ships that keyed build, at the open door
  (`YOKOHAMA_WEB_IDENTITY` `open`), and that the zone's ONE edge rule is ENABLED in front of it: a
  Managed Challenge, at the edge and at zero Worker invocations, for every request to the hostname
  whose path is not one of the release's own static names, `/api/*` and `/enter` among them. A person
  passes it once through the widget's pre-clearance. The deploy guard's check 12 refuses `on` over a
  keyless build (`build-values.json`'s `turnstile_sitekey`), refuses `on` or a keyed build under any
  identity but `open` — the client reads a site key as the open door, and behind Access would never
  upload a nameless visitor's games — and refuses `on` in a pool file. After an `on` deploy the
  smoke's live mode must read the rule's answer, or the deploy fails its smoke:

  ```sh
  node scripts/smoke-release.mjs --live https://yokohama.moore.nyc --config wrangler.open.jsonc
  ```

  A cookie-less `GET /api/health` with a browser User-Agent must answer 403 with
  `cf-mitigated: challenge`. Then, the same way, `GET /sw.js`, and `GET /sitemap.xml` where the local
  tree's `assets/build-values.json` reads `site_url: true`, must each answer with NO `cf-mitigated`
  header: the rule must not challenge the release's own files. Exit 0 is those answers, or, under
  `off`, no request at all. Exit 1 names what came back instead and the path it came back for, a
  refused argument or declaration, or a local tree with no stamp, which asks nothing. Run it from the
  tree that was deployed, since the stamp it reads is the local one. Its first run in a window is
  expected to fail, since the rule is created after the deploy. **`on` does NOT mean** that the Worker does
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
  since the twenty-first deployment (2026-09-28, §15), put in the deploy that dropped the variable,
  from `.dev.vars` through a mode-600 file that was deleted after. *(Corrected 2026-09-28, M6.5c2: `wrangler.jsonc`
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
    (`POST /api/archive`, where `id` is the archive object's name, `archive:<address>`).
    `detail` is the record's key, or for `failed` R2's message — or the host's own sentence
    `<record_key> refused a create as present, then read as absent`, when the record's
    create-if-absent put was refused and the record then read as absent. `attempt`, at the
    game-over site, counts this save's earlier tries that did not complete it, and at the expiry
    site the table's earlier expiries that did not end it. A failed
    game-over save is retried on the backoff — 30 s, 2 min, 10 min, then hourly — by the alarm, or
    by the table's first request once the rung has passed: a request inside the rung uploads
    nothing, so an R2 outage costs a request a read of the owed row and no attempt. *(Corrected
    2026-09-29, M6.5c2 cycle D, R-D6: until then every request re-attempted it, and each failure
    climbed the ladder.)*
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
  `test/browser/red_at_wire.test/rev7/over/wire.json` and
  `test/browser/red_at_wire.test/rev7/abandoned/wire.json`, each with its `.table.json` — kept for
  good, by the owner's direction — and the walk's own game-over pair under `test/rev7/over/`.
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
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/over/wire.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/over/wire.table.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/abandoned/wire.json" --remote
npx wrangler r2 object delete "yokohama-archive/test/browser/red_at_wire.test/rev7/abandoned/wire.table.json" --remote
```

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
- **The flip RECIPE (what a later operator runs, in this order):** to state 2 — `npm run deploy:gate`, then the Access application's destinations PUT with the nine paths (the aud never changes); back — `npm run deploy` (the dev door on), then the PUT with the three hostnames, then the after-check above. To state 1 — the secret deleted FIRST, then `npm run deploy:open`, then the PUT without `yokohama.moore.nyc`; back through state 2's order (the gate deploy BEFORE the secret put — the open shell would 500 beside it). Any deploy restarts every object and closes every socket: flip between games.

### What this deployment does NOT claim
- The live half of RF1 (the update path) on each host was read ONCE, in a headless persistent context on the workstation — not on the owner's Edge, whose pinned state is the one that failed on 2026-09-29; the owner's own browser is the last reading.
- `cf-cache-status: HIT` on `/shell.bin` from Workers Assets is INTERPRETED (content-addressed) from one deploy's read, not measured across a second deploy of a changed shell.
- Row 108's single-audience reading was taken as the owner's WARP identity, not as a second signed-in address; row 119's refusal of an UNLISTED signed-in person was not read (it needs a second identity only the owner can create).
- Rows written and read per move for the tier were not separable from the day's totals (the rollup lag); §18's ~2 / ~52 stand as the last per-move reading.
- A lock (as against the Home screen) is unread on iOS; the plain game's spec printed a wrong applied count on its live run (fixed after; the figures here are from the archived record's 17 events).

### Rollback
- `wrangler rollback` to `974e0a93…` (§18) restores D's release: **an older Worker version renders a `waited` marker's head as `undefined — …`** for up to 90 days (Task 6's marker reason is unknown to it); a table document with `has_events` reads as before (the older module ignores the key); the chat tombstone caveat of §6 stands; open pickers drop (`picker-dropped`).

---
