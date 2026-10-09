# The security ledger

**What this is.** One row per finding of the 2026-10-08 security review of the web lane, with its status at every close — the handover between the remediation cycles (`docs/plans/2026-10-08-security-remediation-spec.md`, §4). Titles here are neutral; the mechanism a fix needs is in the specification's §2, and the exploit detail and the live readings stay in the gitignored report (`docs/temp/security-review-2026-10-08/report.md`), never committed. **Rewrite a row at a close, never append history**: `open` → `fixed <commit>` with the catcher named, or `accepted <the owner's reason>`, or `deferred <cycle>`. A row closes only with a catcher a reviewer mutation-proved, or the owner's accepted-risk sentence.

**Updated:** 2026-10-08 — opened at the review's close; every row `open`.

Tiers as the review set them: **A** High, **B** Medium, **C** the public door, **D** Low. Cycle: `core` (the core PR), `S1`, `S2`, `S3`, `S4` (the public set, outside this specification).

| id | title | tier | subsystem | cycle | status | catcher | live reading |
|---|---|---|---|---|---|---|---|
| A1 | a derived value on the suggest wire inverts to the setup seed | A | layer wire, CLI suggest, client | S1 | open | — | confirmed on both hosts 2026-10-08 |
| A2 | one archive upload of legal moves costs unbounded replay | A (NAS) | layer, core PR | core + S1 | open | — | NAS: confirmed locally; Worker: contained by the object's CPU limit 2026-10-08 |
| A3 | a hot-seat table in the worst-case state answers frames of tens of megabytes | A (NAS) | layer, CLI screen model | S1 | open | — | NAS: confirmed locally |
| B1 | the `why` surface serves the bot's view to its opponent and to spectators | B | layer | S1 | open | — | — |
| B2 | an own-turn undo un-draws a hidden deck card at a shared table | B | layer | S1 | open | — | — |
| B3 | a creator-chosen setup seed on a table that seats another human | B | layer, client form | S1 | open | — | — |
| B4 | no frame protection on either host while the Access cookie is SameSite=None | B | both hosts' headers, service worker, Access | S2 | open | — | cookie attribute read live 2026-10-08 |
| B5 | the operator's expire and force answer a cross-site simple POST | B | Worker shell | S2 | open | — | — |
| B6 | the dev door claims any address; its credential and policy reach further than its purpose | B | Worker shell, secrets, Access | S2 | open | — | the credential names nobody on the NAS or the terminal (2026-10-08) |
| B7 | a nameless joiner's address is served to the creator | B | layer | S1 | open | — | — |
| B8 | the per-id lock map grows without bound on refused writes | B (NAS) | layer | S1 | open | — | — |
| B9 | API answers carry no cache directive and a hosted id may end in a file extension | B (NAS) | layer, both hosts | S1 (+S2 Worker) | open | — | confirmed on the NAS 2026-10-08 |
| B10 | the `cf-access` origin trusts an unverified header on a shared network | B | layer identity, NAS | S2 (code) + S3 (host) | open | — | the shared network read live 2026-10-08 |
| B11 | one audience and 30-day sessions across three origins | B | Access | S3 | open | — | cross-hostname replay confirmed 2026-10-08 |
| B12 | the archive has no per-caller quota and shares the NAS volume with live games | B | layer, Worker archive object | S1 (+S2) | open | — | — |
| C1 | the open door's key space collides with Access-era identities | C | layer identity, Worker shell, runbook | S2 | open | — | 0 tables on the Worker 2026-10-08 |
| C2 | creates and most metered routes are unthrottled; refused names instantiate objects | C | Worker shell, layer | S2 | open | — | — |
| C3 | the deploy guard lets an open-door configuration ship a `workers.dev` URL | C | deploy guard | S2 | open | — | — |
| C4 | README pictures carry carved-out art outside `ASSETS.md`'s table | C | licensing | S4 | deferred S4 | — | — |
| D1 | a refused stream keeps sending frames | D | layer | S1 | open | — | — |
| D2 | the NAS's password oracle is unthrottled | D | NAS, layer | S3 | open | — | — |
| D3 | no cap on streams or sockets per caller | D | layer, Worker objects | S2 | open | — | — |
| D4 | no `Host` validation on the NAS binary | D | layer | S3 | open | — | — |
| D5 | a lockdown's dump carries the seed into the `input` reply | D | layer | S1 | open | — | — |
| D6 | compute routes run on the async executor | D | layer | S1 | open | — | — |
| D7 | the watcher roster's cap locks later watchers out | D | layer | S1 | open | — | — |
| D8 | names and chat accept invisible format characters; house names have no provenance | D | layer, client | S2 | open | — | — |
| D9 | a joiner can freeze a table and the creator cannot reopen the seat | D | layer | S3 | open | — | — |
| D10 | the lane document understates who reads `…/table` | D | docs | S1 | open | — | — |
| D11 | the record route reads "over" outside the lock it serves under | D | layer | S1 | open | — | — |
| D12 | error bodies echo untrusted values verbatim | D | layer | S1 | open | — | — |
| D13 | the service worker drops the host's headers on synthesized navigations | D | client | S2 | open | — | — |
| D14 | the hash router accepts dot segments in a table id | D | client | S2 | open | — | — |
| D15 | the pinned-browser profiles hold a session cookie; Docker ignore entries missing | D | tooling | S2 | open | — | — |
| D16 | restored games are uploaded under the restorer's identity | D | client | S2 | open | — | — |
| D17 | `release.yml`'s permissions, pins and tag rules | D | CI | S3 | open | — | — |
| D18 | `npm run deploy` follows a redirect file the guard never sees | D | deploy | S2 | open | — | — |
| D19 | the NAS build context travels over plain HTTP with no integrity check | D | NAS ops | S3 | open | — | — |
| D20 | the BGA capture excerpt keeps two real handles and a chat line | D | fixtures | S4 | deferred S4 | — | — |
| D21 | cloudflared's metrics server binds every interface | D | NAS ops | S3 | open | — | — |
| D22 | the Access policy's address list and the 30-day session | D | Access | S3 | open | — | read live 2026-10-08 |
| D23 | observability persists addresses and query strings | D | Worker config | S2 | open | — | read live 2026-10-08 |
| D24 | a non-Latin-1 address cannot be a fetch header | D | client | S2 | open | — | — |
| D25 | `check_pair` verifies legality only, not the configuration or the provenance | D | layer | S1 | open | — | — |
| D26 | the archive's caller segment is not injective | D | layer | S1 | open | — | — |
| D27 | an out-of-range `/at` answers 500 where 404 is documented | D | layer | S1 | open | — | — |
| D28 | the input refusal echoes the whole line in `Debug` form | D | layer | S1 | open | — | — |
| D29 | two fence sentences in the code are false | D | layer docs | S1 | open | — | — |
| D30 | a stopped tunnel container carries its token on its command line | D | NAS ops | S3 | open | — | read live 2026-10-08 |
| D31 | Portainer, a registry and open-webui are published on the LAN | D | NAS ops | S3 | open | — | read live 2026-10-08 |
