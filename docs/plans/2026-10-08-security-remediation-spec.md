# Security remediation — the specification (2026-10-08)

**What this is.** The design for closing the 2026-10-08 security review of the web lane — the Cloudflare Worker deployment, the NAS deployment and the Rust→WebAssembly stack they share — written from the review's findings and the brainstorm the owner and the orchestrator held the same day. The review's detailed report, with its exploit mechanics and live readings, is **gitignored** at `docs/temp/security-review-2026-10-08/report.md` (the six reviewer reports and the live-facts record beside it) and is never committed; this specification carries every finding by its id and a neutral title with the mechanism a fix needs and nothing more. The public tree (S4, §7) decides later what of the security record crosses; the owner's instinct is that the work is finished before anything is public.

**Where it starts.** `master` at `d0816172`, the live `v0.6.0` code. Four branches in order, each planned against the merged tree: `security-s1-core-replay` (the core PR, §3.1), then `security-s1`, `security-s2`, `security-s3`. Every PR merges on the owner's word after a whole-branch review.

**The one-paragraph shape.** The perimeter held under execution — the Worker's Access JWT verification, its header strip rule and socket origin check, the layer's content-type gate, the id grammar, the SQL, the R2 keys, the lock hashing, the admission matrix, the gameplay allowlist, the client's DOM handling — and what failed was the project's own hidden-information fence (a derived value on the suggest wire inverts to the setup seed; the `why` surface, an own-turn undo and a creator-chosen seed leak beside it) and the absence of cost bounds on untrusted records (one legal-move upload costs 30+ s of replay on the NAS; a hot-seat table in the engine's worst-case state answers 58 MB frames). Around those: clickjacking and a cross-site operator POST under an Access cookie that is SameSite=None; a dev door that can claim any real address; a NAS origin trusting an unverified header on a shared network; and, for the day the door opens, an open-door key space that collides with Access-era identities. This specification fixes the code and configuration in three cycles, keeps the Lows in a ledger allocated to the cycle that touches their files, and leaves the public set to S4.

## 0. Read order and sources

1. This file, §1 first (the rulings), then §2 (the findings by cycle), §3 (the cycles and their order).
2. The gitignored report `docs/temp/security-review-2026-10-08/report.md` — §2–§5 the findings with their reproductions, §6 what held, §8 the live validation results, §9 the remediation plan this specification restates. An implementer or reviewer who needs an exploit's exact shape reads it there.
3. `docs/security-ledger.md` — the committed ledger (§4): one row per finding id, its status at every close.
4. `docs/web-lane.md` — §2 the suites, §4 the standing rules (the Worker host's rules, the identity bullet, the table document, the ONE mutation, persistence), §8 the public door's preconditions, §13 the API map, §16 the metered-action ledger.
5. `CLAUDE.md` — the `SEMANTICS_REV` contract, the ORACLE INDEPENDENCE RULE, the two-tier gate policy and the fifteen release gates, the conventions (TDD, dated artifacts frozen at their close).
6. The engine's checked boundary: `crates/yokohama-core/src/game/apply_move.rs` (`apply_move` = membership in `get_legal_moves` then the raw transition), `move_legality.rs` (`is_move_legal`, `PathMode`, the module doc's "witnesses, not list membership"), `route_enum.rs` (`movement_routes_family`), `record.rs` (`replay_strict`).

## 1. The rulings

Each row is the owner's decision from the 2026-10-08 session, the owner's words quoted where they were given, and where it lands.

| ruling | the owner's words | where it lands |
|---|---|---|
| **R-SEC1 — a core change is a scoped, dedicated PR with a clean-eyes review before and after, front-loaded at the top of S1.** | "modifying yokohama-core is generally a major change, the byte identical check and the engine/oracle split are critical boundaries"; "keeping the core change to a scoped and dedicated PR with a dedicated clean-eyes review cycle before and after is worth following, in line with repo conventions. If held, it can proceed at the top of S1 since any core changes should be front loaded so the testing sequences don't have different data downstream." | §3.1; the plan `docs/plans/2026-10-08-security-s1-core-replay-plan.md` |
| **R-SEC2 — `SEMANTICS_REV` stays 7 and the oracle is untouched.** The replay check is ADDITIVE (`replay_strict` and `apply_move` unedited) and PROVEN to accept exactly the records `replay_strict` accepts, by a differential cell; a direct swap to `is_move_legal`, which is wider on a president move's witness path, is off the table. | (the orchestrator's analysis, accepted: "Will this change result in a change to semantic_rev?") | §3.1, §6 |
| **R-SEC3 — A1 is fixed at the source and on the wire; the corpus regenerates once.** | "The A1 explanation makes sense and I have no issue with the corpus regeneration." | §2 A1; S1 |
| **R-SEC4 — one Access application for the lab; the control is JWT verification at the origin.** A self-hoster runs one host behind one application and never meets the multi-audience case; what protects them is the origin verifying the assertion, and cloudflared's `access.required`. The application split is deferred hardening. | "I don't think the multiple audiences are necessary for our development use cases but we can change that either now or in production if there is a reason beyond our dev lab use cases. For example, if someone were installing this on their own host, would they fall into that trap." | §2 B10, B11; S2, S3 |
| **R-SEC5 — every control holds with Access removed.** Access is a convenience; when it is used it is reliable; the application is tested at the live tier. | "The access identity platform is a convenience rather than something we should rely on exclusively, although when it's being used it should be reliable"; "ultimately the test of the application is at the live tier." | §5 (the `open` pool project and the live probes in every cycle's acceptance) |
| **R-SEC6 — the public set is S4, after this work; the detailed review never ships; the public Worker is its own deployment.** Three deployments at the public day: the private NAS, the private Worker behind Access, and a new internet-facing Worker with its own script, object namespaces and bucket. | "we can think of that as an S4"; "my instinct would be to have it handled before we move into the public set"; "when I make a repo for this project public, I'll also have an open internet facing worker — which makes the final checks for the metered truths, unmetered front door, and other verifications important". | §3.5, §7 |
| **R-SEC7 — the Lows are kept, in a ledger, allocated to the cycle that touches their files.** | "I don't want to lose sight of many of the excellent findings in the low/hardening section." | §2 (the D rows), §4 |
| **R-SEC8 — the product rules** (proposed as defaults, accepted by "go"): (a) at a multi-person table an undo whose unit drew from a hidden deck is refused; (b) the seed field is honoured only when every human seat is the creator's, refused otherwise with a plain sentence; (c) under `open` an email-shaped claim is refused, names stay names; (d) the dev door is off by default and on only around a live tier; (e) hot seat stays, the option list is folded on the wire and capped; (f) the archive quota and body cap are set in S1's plan from measured game sizes; (g) the Access cookie goes to Lax and the application session to a day, both reversible; (h) the ledger is committed from the start with neutral titles and ids. | "go" | §2, §4, S1–S3 |
| **R-SEC9 — one specification, one plan per cycle, each plan written against the merged tree when its turn comes.** | "are you suggesting running through the three spec writing process now?" — answered: one spec, three plans | §3, §8 |
| **R-SEC10 — live validation is the acceptance, in maintenance windows.** The review's approved probes (§5.3) are re-run after each cycle's deploy; a probe that creates state creates scratch state and removes it. | "you can run the test against live host"; "the system is in maintenance today no other users should be online, clear to test against live as you see fit to run through the review and remediation process." | §5.3 |

## 2. The findings, by cycle

Each row: the id (the review's), a neutral title, the files, the fix in one line, and the catcher that pins it. Tiers as the review set them: A High, B Medium, C the public door, D Low. The mechanism's detail is the report's.

### 2.1 S1 — the leaks and the cost bounds (Rust: the layer, the CLI's suggest derivation, the core PR)

| id | title | files | the fix | the catcher |
|---|---|---|---|---|
| **A1** | a derived value on the suggest wire inverts to the setup seed | `crates/yokohama-cli/src/suggest.rs` (`stream_seed`), `crates/yokohama-web/src/explain.rs` (`Suggest.stream_seed`), `web/src/Explain.tsx`, `web/src/types.ts`, the corpus | drop the field from the web wire and the client; make `suggest::stream_seed` a one-way digest of the same components (determinism kept, invertibility gone); regenerate the corpus once (R-SEC3) | a corpus pin that attempts the inversion of every integer-looking field of `suggest.json` and `why.json` against the walk's seed and finds nothing; the existing value-search pin stays |
| **B1** | the `why` surface serves the bot's view to its opponent and to spectators | `crates/yokohama-web/src/explain.rs`, `api.rs` (`why`), `crates/yokohama-core/src/game/eval.rs` (read only) | compute the served explanation at the requester's visibility: the mover's still-hidden starting order masked; the continuation's fulfil lines masked as `rank_all` masks the ranking; `decision_seed` dropped from the wire | a cell that walks a vs-bot game and asserts no `why` payload's feature values, continuation or provenance narrows the unseen cards to the bot's held order (the review's narrowing script as a test) |
| **B2** | an own-turn undo un-draws a hidden deck card at a shared table | `crates/yokohama-web/src/api.rs` (`undo_allowed`, R-P6) | at a table with more than one person, refuse an undo whose unit shrank the order or tech deck (compare `state_at(record, len-1)`'s deck lengths with the current state's) — R-SEC8(a) | a cell: take at a lab, read the refill, undo → 403 with the sentence; a two-person table's undo of a non-drawing move still succeeds; a solo table's undo of a drawing move still succeeds |
| **B3** | a creator-chosen setup seed on a table that seats another human | `crates/yokohama-web/src/tables.rs` (`create`), `web/src/NewGame.tsx` | honour `seed` only when every human seat is the creator's; refuse otherwise with a plain sentence; the form hides the field for such tables — R-SEC8(b) | a create cell for each seat shape; the whole-game gate's claimed-identities arm unchanged |
| **B7** | a nameless joiner's address is served to the creator as `invited` | `crates/yokohama-web/src/table.rs` (`SeatView::of`), `tables.rs` (`join`) | record how a seat was bound (`bound_at_create` on `SeatDoc::Human`, serde default false) and fill `invited` only for creator-bound seats | the leakage pin extended to a walk whose joiner sends no name; `the_stored_table_document_carries_every_seats_address` stays |
| **B8** | the per-id lock map grows without bound on refused writes | `crates/yokohama-web/src/games.rs` (`lock_for`, `prune_lock`), `tables.rs` (`join`, `keep`, `unlock`, `delete`), `api.rs` (`matches`) | refuse an `{id}` over 64 bytes before dispatch; validate with `is_safe_id` and read the table before taking a per-id lock; prune on every exit when the map's reference is the last | a cell that fires refused writes at fabricated ids and asserts the map's length after; the doc sentence "a table nobody has written costs nothing" made true |
| **B9** | API answers carry no cache directive and a hosted id may end in a file extension | `crates/yokohama-web/src/lib.rs` (the adapter), `api.rs`, `store.rs`/`tables.rs` (the hosted id rule); the Worker's `respond` in S2 | `Cache-Control: no-store` on every API answer from the adapter and the `events`/stream routes; refuse a hosted id whose last dot-suffix is a known extension | a cell over every route's answer headers; a create cell for `x.js`, `x.css`, `x.png`, `x.bin`; the live probe (§5.3, V2) reads `DYNAMIC` after the deploy |
| **B12** | the archive has no per-caller quota and shares the NAS volume with live games | `crates/yokohama-web/src/api.rs` (`archive_post`), `archive.rs`, `crates/yokohama-worker/src/receipt.rs` | a per-caller quota in count and bytes enforced before the put (a directory count on the NAS, a counter in the Archive object on the Worker, S2); the body cap lowered toward measured game sizes — numbers set in S1's plan (R-SEC8(f)) | a cell that fills a caller's quota and reads the refusal; the body cap's cell |
| **A2** | one archive upload of legal moves costs unbounded replay | `crates/yokohama-web/src/backup.rs` (`check_pair_with`), `api.rs` (`archive_post`), `lib.rs` (the adapter); the core PR's `replay_strict_with` | bound the record's event count before any replay (the cap from the plan's measurement of finished games at 2/3/4P); replay with the core PR's predicate check; run `/api/archive` and the other compute routes under `spawn_blocking` behind a small semaphore on the NAS; answer "identical" by comparing stored bytes before replaying | a cell replaying the review's worst-case record under the cap → refused by count; a timing cell over the cap-sized worst case asserting a bound; the two-worker stall reproduction as a cell behind the semaphore |
| **A3** | a hot-seat table in the worst-case state answers frames of tens of megabytes | `crates/yokohama-web/src/client.rs` (`ClientState`, `HistoricalFrame::run`), `crates/yokohama-cli/src/screen_model.rs` (`option_rows`), `session.rs` (`undo`) | fold the retreat family on the wire into one entry with its count, members on demand; cap any per-frame option list; a replay checkpoint so `/at` and `undo` do not replay from setup; the replay routes under `spawn_blocking` — R-SEC8(e) | a cell at the spread state asserting the frame's byte size under a bound and the option rows folded; the existing whole-game gate unchanged |
| D1, D5, D6, D7, D11, D25–D29 | the Lows in these files: a refused stream ends; the lockdown dump's seed line kept off the web reply; the executor hand-off; the watcher roster's cap behaviour; the record route's "over" read under the lock; `validate_config` and `seat_disagreement` inside `check_pair`; an injective caller segment; the `/at` 404; a bound on `Input.line` and a 64-character echo; the two false fence sentences corrected | as the report names them | one line each in S1's plan, each with its cell | — |

### 2.2 S2 — the door and the client (both identity modules, the Worker shell, the client, the configuration)

| id | title | files | the fix | the catcher |
|---|---|---|---|---|
| **B4** | no frame protection on either host while the Access cookie is SameSite=None | `worker/static/_headers`, `crates/yokohama-web/src/assets.rs` + `lib.rs` (a response layer), `web/src/sw.ts` (`shellResponse`), `web/src/main.tsx`; the Access application's cookie setting | `Content-Security-Policy: frame-ancestors 'none'` and `X-Frame-Options: DENY` from both hosts; the service worker passes the stored `/shell.bin` headers through on a navigation (D13); a `window.top` check before render; SameSite=Lax and a 24-hour session on the application — R-SEC8(g) | the `sw.node.test.ts` cell that a navigation carries the headers; a headers cell per host; the review's framed-page reproduction as an e2e cell against the local binary |
| **B5** | the operator's expire and force answer a cross-site simple POST | `worker/src/index.ts` (`handle`), `admin.ts` | refuse every method but GET and HEAD when `Origin` is present and not `url.origin`, or when `Sec-Fetch-Site` is `cross-site` or `same-site`; require `Content-Type: application/json` on `POST /api/admin/…` | pool cells: a bodyless, a form and a text/plain POST from a foreign origin → 403 and the table intact; curl with no `Origin` still passes |
| **B6** | the dev door claims any address; its credential and policy reach further than its purpose | `worker/src/identity.ts` (`resolveIdentity`'s dev arm), `worker/wrangler*.jsonc`, `worker/RUNBOOK.md`; the `.dev.vars` file's mode; the service token's duration | the dev arm accepts `.test` addresses only; production runs from `wrangler.gate.jsonc` (dev door off) and the door is switched on only around a live tier — R-SEC8(d); `chmod 600` on `.dev.vars` and the unused `TURNSTILE_SECRET` removed; the service token rotated to a shorter duration | a pool cell: the credential plus a non-`.test` claim names no one; the deploy guard's door line pinned |
| **C1** | the open door's key space collides with Access-era identities | `crates/yokohama-web/src/identity.rs` (`admit`), `worker/src/identity.ts` (`claimedIdentity`), `worker/RUNBOOK.md` | under `open` an email-shaped claim is refused, names stay names — R-SEC8(c); the runbook rule that a populated deployment never changes its door and a public deployment starts empty | the open pool project: a claimed address reads as nobody; the NAS binary's `identity::` cells |
| **B10** (code half) | the `cf-access` origin trusts an unverified header | `crates/yokohama-web/src/identity.rs` (the reserved `cf-access-jwt` arm), `main.rs`, a JWT verifier under the `server` feature, `deploy/README.md` | build `cf-access-jwt`: verify `Cf-Access-Jwt-Assertion` (signature against the team's keys, `aud`, `iss`, `exp`), take the address from the token, name no one for a service token; make it the self-host guide's default behind Access — R-SEC4 | cells mirroring the Worker's JWT battery (alg, kid, signature, aud, iss, exp, a service token) against a test key set; the smoke image's door check |
| **C2** (Worker half) | creates and most metered routes are unthrottled; refused names instantiate objects | `worker/src/index.ts`, `route.ts` (`routableName`), `wrangler.jsonc` (a rate-limiting binding) | a Workers rate-limiting binding keyed by the caller under `worker` and by claim plus IP under `open`, on creates and every unsafe method; `routableName` widened to `is_safe_id`'s grammar so an unusable name never reaches a Table object | pool cells for the limit and for an unusable name landing at the house |
| **C3** | the deploy guard lets an open-door configuration ship a `workers.dev` URL | `worker/scripts/deploy-guard.mjs` | require `workers_dev: false` under every door; run the sibling rule for every open-door file | the guard's node cells over a copied open file with the flag flipped |
| D3, D13, D14, D15, D16, D18, D23 | the Lows in these files: socket and stream caps; the service worker's header pass-through; the hash router's id grammar; the pinned instrument clearing its profile and the Docker ignore entries; restored games excluded from the automatic upload; `--config wrangler.jsonc` explicit and a guard refusal of a redirect file; what the shell logs of addresses | as the report names them | one line each in S2's plan, each with its cell | — |
| the CSP | the header set in the client review's §"The header set" | `worker/static/_headers`, the NAS response layer | shipped as `Content-Security-Policy-Report-Only` for one release, then enforced | the headers cells; a release's console read |

### 2.3 S3 — the hosts and operations

| id | title | files / where | the fix | the catcher |
|---|---|---|---|---|
| **B10** (host half) | `yokohama-web` shares its network with the terminal and the tunnel, with no capability drop or memory limit | the NAS (Portainer), `deploy/cloudflared-config.yml`, `deploy/update.sh`, `deploy/README.md` | a network shared only with `cloudflared-nas`; `CapDrop: ALL`, `no-new-privileges`, a memory limit; cloudflared's `originRequest.access` (`required: true`, `teamName`, `audTag`) on both ingress rules; the "one path" sentences corrected | the deploy readings; `smoke-image.sh` |
| **B11** | one audience and 30-day sessions across three origins | the Access application | the session shortened (S2's R-SEC8(g)); the application split deferred by R-SEC4, recorded in the ledger as accepted | the live reading |
| D2, D4, D9, D17, D19, D21, D22, D30, D31 | the Lows here: the NAS unlock limiter; a `Host` allow-list; a seat the creator can reopen; `release.yml`'s permissions, pins and tag rules; the build context's integrity check; cloudflared's metrics bind; the Access policy's address list reviewed; the stopped tunnel container's token; the LAN-published services | as the report names them | one line each in S3's plan | — |

### 2.4 Deferred to S4 (named here, not specified)

C4 and D20 (the art carve-outs and the BGA excerpt, licensing and third-party data), the fresh-history repository, the document curation, the contributor and licensing hygiene, the public Worker as its own deployment and its live acceptance (the metered and unmetered truths, Turnstile, the edge rule, the open-door preconditions of `docs/web-lane.md` §8).

## 3. The cycles and their order

### 3.1 The core PR — `security-s1-core-replay` (R-SEC1, R-SEC2)

One addition to `crates/yokohama-core`, no edit to an existing function:

- `move_legality::is_listed_move(state, mv, rules) -> bool` — the single-candidate answer to "is `mv` in `get_legal_moves(state, rules)`", exact: for `Move::MovePresident` and `Move::PlacePresident` the conjunction of `is_move_legal(…, PathMode::Canonical)` (the phase and queue gates) and membership in `route_enum::movement_routes_family` (the family's exact list, small, compute-per-call — the witness path is part of what the list prints, so the family's list is the only exact check); for every other variant `is_move_legal(…, PathMode::Canonical)`, where the module's contract says acceptance and membership coincide.
- `record::MoveCheck { Listed, Predicate }` and `record::replay_strict_with(record, rules, check)` — the strict replay's preamble (rules hash, semantics rev), per-event fold and invariant check, and terminal-digest check, with `Listed` folding through `apply_move` exactly as `replay_strict` does and `Predicate` folding through `is_listed_move` then `apply_move_unchecked`. `replay_strict` itself is untouched.
- The proof, a per-edit test binary `tests/replay_check_equivalence.rs`: (1) at every decision point of random games at 2/3/4P, every listed move satisfies `is_listed_move`, and every constructed non-member — the legality tests' mutant shapes, the non-canonical witnesses the `RulesLegal` contract accepts, every listed move checked one state later — reads exactly as `get_legal_moves(..).contains(..)`; (2) for every recorded random game and for the tampered records `game_record.rs` already builds, `replay_strict_with(.., Predicate)` returns the same `Ok` terminal digest or the same `Err` index and message as `replay_strict`; (3) the mutation battery: `is_listed_move` without the family membership fails (1) on a non-canonical witness, and `Predicate` without the phase gate fails (2).
- What it does NOT change: `SEMANTICS_REV`, the oracle, `apply_move`, `get_legal_moves`, `replay_strict`, any corpus byte (the wire corpus cells pin this).
- Reviews: the plan review executes the plan's cells in an export before implementation (the "before"); the whole-branch review and the mutation battery after; then the fifteen release gates and every per-edit suite, the web lane's rows included, at the reviewed head.

### 3.2 S1 — the leaks and the cost bounds

§2.1's rows. Its plan is written after the core PR merges, against the merged tree. The wire corpus regenerates exactly once in this cycle (A1). The CLI is touched in one place, `suggest::stream_seed`'s derivation, and the goldens that print it regenerate once; the `decide` path and every bot figure are untouched.

### 3.3 S2 — the door and the client

§2.2's rows. Its plan is written against S1's merged tree. The Access configuration changes (R-SEC8(g)) are the orchestrator's, made before the cycle's live acceptance and recorded in the runbook.

### 3.4 S3 — the hosts and operations

§2.3's rows. Its plan is written against S2's merged tree; the NAS steps are the orchestrator's through Portainer and the runbooks' recipes.

### 3.5 The dependency chain, stated once

The core PR → S1 → S2 → S3 → (S4: the art replacement lane finishes → the fresh history is cut → the repository goes public → the public Worker deploys and opens). The door of the private Worker never flips; the public deployment starts empty (R-SEC6).

## 4. The ledger

`docs/security-ledger.md`, committed from the start (R-SEC8(h)), one row per finding id: the id, a neutral title, the tier, the subsystem, the cycle, the status, the catcher, the live reading. Statuses: `open`, `fixed <commit>` (with the catcher named), `accepted <reason>` (the owner's), `deferred <cycle>`. The ledger is rewritten at every cycle's close and is the handover between cycles: a session that asks "what remains" reads it. The detail stays in the gitignored report until a row is fixed; a fixed row's close document may describe the mechanism, which is ordinary practice; the live readings with hostnames and ids stay in the gitignored report.

## 5. Testing and acceptance

### 5.1 Per edit
The project's suites as `CLAUDE.md` and `docs/web-lane.md` §2 list them, with the lint gate. Every catcher in §2 is MUTATION-PROVEN at its task's review: the fix reverted or weakened, the cell red.

### 5.2 Per cycle
The whole-branch review on an export, one fix wave, the mutation battery, the ONE recorded sequence at the reviewed head; the fifteen release gates when core or the workspace version moved (the core PR, and any cycle that bumps the version); the `open` pool project green in every cycle (R-SEC5).

### 5.3 The live acceptance (R-SEC10)
After each cycle's deploy, in a maintenance window, the review's probes re-run from the owner's workstation with the recorded expectations: the identity-header probes on both hosts (the credential names nobody on the NAS; the Worker's dev door names only `.test`); a `.js`-named scratch table's frame reads `DYNAMIC` with `no-store` (B9); a scratch table's suggest payload carries no invertible field (A1); a cross-origin bodyless POST to the operator route is refused (B5); a framed page cannot act (B4); the worst-case record is refused by count before any replay, on both hosts (A2); and, from S2 on, the response headers of `/`, `/shell.bin`, an asset and `/api/health` on both hosts. Every probe that creates state removes it; nothing is left cached.

## 6. The fences

- `SEMANTICS_REV` is 7 throughout; the oracle (`crates/yokohama-oracle`) is never touched; `apply_move`, `get_legal_moves` and `replay_strict` are unedited (R-SEC2).
- The five fenced crates stay byte-identical in every cycle but the core PR, whose whole diff is §3.1's addition; the CLI is touched only at `suggest::stream_seed` (A1) and nowhere the bot decides.
- The wire corpus regenerates once, in S1, from A1; the core PR changes no corpus byte.
- No bot behaviour moves: `decide`, the arena, the evaluator and `why`'s reconstruction are untouched.
- The detailed review report is never committed; this specification and the ledger carry neutral titles and the mechanism a fix needs.

## 7. What this specification does NOT do

S4 — the public set (§2.4). The browser terminal's own hardening (`yokohama-cli.moore.nyc`, outside the repository). The LAN-published services on the NAS beyond recording them (D31). Real accounts under the open door (a name stays a label there). The Access application split (R-SEC4, deferred). The fifteen-gate roster's growth: the core PR's differential cell is per-edit, not a sixteenth gate.

## 8. Process

The project's subagent-driven method with the owner's tiers: Sonnet for implementation-focused tasks whose brief carries the judgment, Opus where judgment is needed and for every task review, Fable for the whole-branch review. Every dispatched agent writes its full report to a file and replies in three lines; reviewers run their mutants on an export, never in the implementer's tree; no agent runs `rm -rf`, `rm -r` or a bulk `rm -f`, arms a Monitor or waits on an event; implementers probe the brief's code before their own review and report `DONE_WITH_CONCERNS` when a brief's claim is false. Every account step — a deploy, an Access change, a zone rule, a Portainer change, a live probe — is the orchestrator's alone, in a maintenance window, each a dated line in the cycle's ledger.

## 9. Review record

(The plan review's findings and this specification's corrections, dated, go here; the file is frozen at S3's close.)

### 9.1 — 2026-10-08, the core PR plan's "before" review (the orchestrator's own, in an export; the plan's §"Review record" carries the findings R-1–R-9 and every reading)

The plan's cells were executed against a scratch implementation of its own code blocks and its six mutants read. Nine findings, three HIGH, every one corrected in the plan: the record-level tamper cell was vacuous as written (its seed held no walked move), its repeated-hop witness never exercised the family conjunct, and one mechanism sentence was false (a Movement move's successor never queues a choice, so the phase cell could not meet the queued case); the rest are a doc-truth sentence, a duplication, a weak counter, two cites and one noted non-defect. What this specification said that the review corrects, recorded forward:

- **§3.1, the proof's shape.** The move-level half (1) lives in `move_legality.rs`'s `mod tests` (two cells and a helper), the binary `tests/replay_check_equivalence.rs` holds the record-level half (2); and (2) now covers four shapes beyond `game_record.rs`'s five tampers — a non-canonical witness `PathMode::Canonical` accepts, a Movement-family member planted at an AreaAction event, an invariant break, a move after the game's end. The battery sentence (3) reads: `is_listed_move` without the family membership fails (1) on a non-canonical witness AND (2) on the witness tamper; the family alone, without `is_move_legal`'s gates, fails (1) on a real family member under a queued choice or outside Movement AND (2) on the planted member.
- **§3.1 and §2.1 A2, the cost claim.** "A cost that no longer follows the legal families' sizes" is true of the retreat family and not of the two families `is_move_legal` delegates (area actions, choice resolutions — max 154 and 114 measured), which stay materialized and small. Measured in release on the Mac: the review's 4P worst-case record of 23,606 events replays in 54 ms under the predicate check against 29.0 s under `replay_strict`, the same digest (3P 10.5 s → 40 ms; 2P 11.8 s → 22 ms; the null-turn 4P body 107 ms → 41 ms). S1's plan sizes A2's event-count cap and the semaphore from these figures.
- **§3.1, "every listed move checked one state later".** Kept, and joined by the real family members judged where the list cannot hold them — the queue gate's precedence has its own counter now.
- **The worst-case bodies** the review built are kept, gitignored, in `docs/temp/security-review-2026-10-08/worst-case/` (four archive-POST bodies, `{ "record": …, "table": … }`); the plan's Task 4 Step 6 re-measures from there at the reviewed head.
- **Process.** The branch `security-remediation-spec` is pushed and open as PR #47; this review is that PR's review, and the branch merges on the owner's word.

### 9.2 — 2026-10-09, the core PR's close and the owner's branching ruling for the remainder

The core PR merged as PR #48 (master `9ab8c430`); its plan's §"Review record" (R-1–R-18) and guidingpath's 2026-10-09 row are
the record. The core half of A2 exists and is proven (`replay_strict_with(.., MoveCheck::Predicate)`, 54 ms against 28.5 s on the
review's 4P worst case); the wiring, the cap and the semaphore are S1's, so the ledger's A2 stays `open`. **The owner's ruling at
the merge, recorded forward against "Where it starts" and §3:** the core set had unique treatment — its own PR, reviewed before and
after — because it modified the protected core files; the remainder does not, so **S1 proper, then S2, then S3 run STACKED on ONE
new branch cut from `9ab8c430` and merge back to master from there, not per cycle**. The per-cycle whole-branch reviews stand
(§5.2), and R-SEC9's "each plan written against the merged tree when its turn comes" reads "against the stack's head". The
post-merge documents of this close were committed directly on master by the owner's word.

### 9.3 — 2026-10-09, S1a's plan: the cycle split, the corpus, and ten corrections

**The cycle split.** S1 (§2.1) spans three subsystems, so the plan splits it by finding family into three cycles, each planned against the stack's head (`security-s1`, cut from master `e9824ec9`, the code of `9ab8c430`). **S1a — the leaks** (the plan this section records) takes A1, B1, B2, B3, B7, D5, D10 and D11, and moves the CLI's `suggest::stream_seed`; the layer's `explain.rs`, `api.rs` (`why`, `undo_allowed`, `input`, `record`), `tables.rs`, `table.rs` and `games.rs`; the client's explain and new-game surfaces; the Worker's seed switch; and the corpus (twice, as the next paragraph says). **S1b — the cost bounds** takes A2, A3, B12, D6, D25, D27 and D29 (`archive_post`'s doc), and moves `backup.rs`, `api.rs` (archive, `/at`), `lib.rs` (the executor hand-off and the semaphore), `archive.rs` (the quota), `client.rs` (the frame's option list) and the client's frame types, regenerates the corpus once (A3's frames) and drops the `CLAUDE.md` parenthetical "S1 wires the web layer to it" (the core PR's note). **S1c — the plumbing** takes B8, B9, D1, D7, D12, D26, D28 and D29 (`is_token`'s doc), and moves `games.rs` (the lock map), `lib.rs` (`Cache-Control`, the stream's end), `store.rs` and `tables.rs` (the hosted id rule), `grammar.rs`, `archive.rs` (`caller_segment`) and the error echoes.

**The corpus** regenerates once per cycle that changes the wire's shape, read each time (the owner's note of 2026-10-09, recorded as correction 1 below). Within S1a it regenerates twice, each regeneration its own task's and read on its own: Task 1's (A1's `suggest.json`, B1's `why.json` and the new `why_withheld.json`) and Task 3's (B7's `invited`, which flips to `null` on the walk's nameless joiner in every table-bearing fixture); Tasks 2, 4 and 5 add no fixture and change no committed byte.

What this specification said that the plan's review corrects, recorded forward (the plan's §"Spec corrections", verbatim):

1. **§6, the corpus.** "The wire corpus regenerates once, in S1" reads "once per cycle of S1 that changes the
   wire's shape, read each time" (the owner, 2026-10-09).
2. **§6 and §3.2, the CLI fence, against §2.1 A3.** "The CLI is touched only at `suggest::stream_seed`" and
   A3's file list (`screen_model.rs`, `session.rs`) contradict each other. The fence's substance is "nowhere the
   bot decides" (§6's own last clause): S1a touches the CLI at `suggest::stream_seed` and its manifest alone;
   S1b's plan decides A3's CLI edits under that reading, preferring the layer where a layer-side change is
   equal.
3. **§2.1 B1, the served shape** (the owner confirmed it 2026-10-09, after asking whether the hidden card
   matters past the opening round: it does — the bot's kept starting order stays in its hand, masked from every
   other seat by `observe`, until the bot FULFILS it, mid-game in every recorded game, and the review named the
   card from the FIRST bot move's feature values alone). "Compute the served explanation at the requester's
   visibility" is met by WITHHOLDING the account while the mover's starting order is hidden from the requester
   (and, for a WATCHER, at game over too, since the record route refuses a spectator — the review of Task 1,
   I-1), not by a masked recompute: the explanation is read at the search's hand-off state, which the layer does not hold, a
   recompute at the decision state would be a second evaluation derivation (forbidden), and every served value —
   the terms, `total`, the ranking's values, `root_candidates`, `forced` — is a function of the hidden card.
   The continuation is withheld too, and the reason is on record: a plan computed with the card known is
   re-searchable per candidate card, so a served plan with masked fulfil lines still narrows (the review's own
   caveat on `rank_all`'s doc, generalized). The fulfil-line masking the spec asks for applies to the served
   (public) continuation, for consistency with the ranking.
4. **§2.1 B3 and R-SEC8(b), the fixtures** (the owner chose the named `Diagnostic` policy, 2026-10-09: "a good
   idea especially for future development and through these other phases"). The corpus walk (`wire_fixtures.rs`'s `new_game_body`: a seeded
   create with an OPEN seat), 79 lib cells (`two_people(id, seed)`), the Worker crate's `CREATE` constant and
   the Worker pool's replay of the walk all open seeded multi-person tables, and the corpus's determinism
   REQUIRES it. The rule is kept as ruled and the fixtures opt into a named `SeedPolicy::Diagnostic` — on the
   NAS binary never, on the Worker through `YOKOHAMA_DIAGNOSTIC_SEEDS`, `on` only in the pool's two test files
   and refused by the deploy guard in any deployable file. The alternative the review offered — honour the seed
   and carry `seeded: true` on `TableView` with a sentence on the join screen — is recorded as the fallback the
   owner may prefer; it needs no switch and leaves the creator's advantage disclosed rather than closed.
5. **§2.1 D5, the catcher.** The lockdown's dump is produced only at the moment of a fault inside `Repl::step`;
   the one fault a cell can reach through the API is a bot whose own deal is unrecorded (a record with an
   `Unknown` choice at the bot's seat), which Task 5's cell installs; the filter is unit-pinned beside it.
6. **§2.1 D11, the fix.** "The record route's 'over' read under the lock" is met and joined by a comparison:
   the stored record's event count against the handle's AND its terminal digest against the digest of the
   handle's LIVE state — what a successful final save stamps; the record's own `terminal_digest` field does not
   track the live state, `None` for a game dealt in process and the loaded value for a rebuilt one (Task 5's
   fix round, F1) — (the count alone is defeated by undos under failing store writes and a different line replayed to the same
   length — measured in Task 5, six undos and the same 356 events, the running record served with its seed), so a
   finished game whose last save failed answers a 409 naming the failed save rather than the running record with
   its seed.
7. **§5.3, the live acceptance, and §9.2's stacked branch.** Each S1 cycle ends with a pushed PR whose merge
   waits for S1's wrap (the owner, 2026-10-09: "the plan will need to end with a PR, it can leave the merge
   undone"); a test deploy of the branch's build to either host and the review's probes are allowed in the
   maintenance window the owner opened for 2026-10-09/10 ("pushing the PR and running any checks or deploys are
   fine for testing"). So S1a's probes — a scratch table's `suggest` carries no invertible field; `why` reads
   `withheld` at a scratch vs-bot table and the full account at a finished one; a seeded create beside an open
   seat is a 400 on both hosts — run at S1a's close from a test deploy when the window still holds, else at the
   stack's deploy after S3. Every probe that creates state removes it.
8. **§6 and §3.2, the CLI fence, and the guide pair** (the before review, 2026-10-09). The captured `suggest`
   screen in `docs/cli-guide.md`/`.html` is held BYTE-EXACT to the fixture's own render by the CLI's regeneration
   pin (`suggest::tests::the_guide_suggest_capture_is_what_the_fixture_renders`), and A1's digest moves the key
   line and, at that fixture, the answer (a tie's keyed draw). "The CLI is touched only at `suggest::stream_seed`"
   reads "…and the guide pair's captured `suggest` screen, re-captured from one run of the fixture, never
   edited" — CLAUDE.md's guide rule for a user-visible CLI change.
9. **§2.1 B7, the catcher and the stored document** (the before review). The corpus cannot carry the B7 case —
   the walk's joiner joins WITH a name — so the fence is the layer's cells (the unit cell's three shapes, the
   nameless-joiner route cell), not a corpus pin; a text rule on `invited` cannot tell a seat the creator typed
   from one that was joined. The STORED document of a creator-bound seat gains `"bound_at_create": true` (a
   joined seat's bytes are unchanged), and a table stored before the deploy stops echoing the creator's typed
   addresses for still-nameless seats, once — the compatibility default, chosen.
10. **§2.1 B3 and R-SEC8(b), the third host** (the before review). The browser host (`crates/yokohama-wasm`)
    builds its `Games` through `Games::with_parts` too and reads `SoloOnly` like the NAS and the Worker; its
    harness — the crate's seeded cell and the browser half's walk replay — opts in by name through a second
    export, `init_diagnostic()`, the Worker switch's shape. Correction 4's list of opt-ins reads: on the NAS
    binary never; on the Worker through `YOKOHAMA_DIAGNOSTIC_SEEDS`, `on` in the pool's two test files; on the
    browser host through `init_diagnostic()`, which the client never calls.

**The S1 figures** (measured 2026-10-09 for S1b's plan — A2's event-count cap and semaphore, B12's body cap and quota), as the plan carries them, verbatim:

**Method.** A scratch example in a `git archive` export of `e9824ec9` (never committed), release profile, the
Mac (Darwin 27.0.0 arm64): uniform-random legal play to the game's end at 2/3/4P, 200 seeds each, every game
finishing (no guard hit at 60,000 moves); the record stamped and serialized compact and pretty — **pretty is what
a browser sends**, `Session::to_record_json` is `to_string_pretty`. The bot's own games through
`examples/arena cell s1sizes 2 v4-step9-draft1 v4-step9-draft1 3 --emit … --master 20261009` (six games).

| shape | events: median / p90 / p99 / max | bytes at the max game: compact / pretty | bytes per event (mean): compact / pretty |
|---|---|---|---|
| 2P random, 200 seeds | 5,099 / 7,624 / 12,853 / 15,342 (seed 104) | 655,008 / 1,496,479 | 42.9 / 98.3 |
| 3P random, 200 seeds | 6,736 / 10,071 / 14,080 / 18,625 (seed 160) | 812,811 / 1,882,221 | 43.4 / 99.9 |
| 4P random, 200 seeds | 8,849 / 11,774 / 15,621 / 16,867 (seed 4) | 736,517 / 1,695,364 | 43.8 / 101.0 |
| 2P bot vs bot (V4 × V4), 6 games | 287 – 416 | 19,127 / 41,984 at the longest | ~46 / ~101 |
| the owner's human-vs-bot games (3) | 243 – 252 | — / 25,818 – 26,075 on disk | — / ~104 |
| the browser fixture (`tests/fixtures/browser-played-2p.json`) | 595 | — / 54,103 | — / 91 |
| the review's worst-case body (`spread_4p_11`) | 23,606 | 1,046,122 (compact, passing turns) / — | 44 / — |

**Reading.** Two kinds of finished game exist: the ones people and the bot play (250–600 events, under 60 KB
pretty-printed) and random play (5,000–19,000 events, 0.5–1.9 MB), which no browser game produces. The present
1 MiB body cap already refuses a random 3P game pretty-printed (1.88 MB), and nobody has noticed, because no
browser game is one. Under the predicate replay the worst admitted record costs 54 ms whatever the cap, so the
event cap is a PLAUSIBILITY bound — what a finished game is — and a storage bound, not a CPU bound.

**What S1b's plan transcribes, unless the owner rules the other way:**

| knob | value | why |
|---|---|---|
| A2 event cap | **4,096 events** | seven times the longest game people or the bot have played (595), refused by COUNT before any replay; the worst-case body (23,606) and every random-play record fall outside |
| B12 body cap (`ARCHIVE_BODY_LIMIT`) | **512 KiB** | eight times the longest pretty-printed game measured (54 KB); above ~5,000 pretty-printed events, so the count cap speaks first on every real record |
| B12 per-caller quota | **256 records and 128 MiB** per caller segment | 256 finished games is years of play; 128 MiB is the count times the body cap, so neither bound is dead |
| A2 semaphore (the NAS) | **2 permits**, acquire bounded at **2 s** then `503` with `Retry-After: 1` | the NAS has 4 cores (the review's V9 read); two blocking replays leave two cores to the runtime; at ≤ 55 ms per admitted replay two permits clear ~36 worst-case uploads a second, far above the edge's 20 per 10 s per client; the bounded wait keeps queued bodies from piling up behind a burst |

The alternative sizing, recorded: an event cap of 24,000 with the body cap raised to the adapter's 2 MiB admits
every finished game of any play style measured, at the same ≤ 55 ms — a wider notion of "a finished game" and a
body cap moving UP, against the review's "lowered toward measured game sizes"; the choice is the owner's
(R-SEC8(f)), to be taken at S1b's plan.

### 9.4 — 2026-10-09, S1b's plan: the cost bounds' decisions and nine corrections

S1b's plan (`docs/plans/2026-10-09-security-s1b-cost-bounds-plan.md`) takes eight decisions of its own and corrects nine statements of this specification, recorded forward. Both sections follow VERBATIM, as the plan wrote them on 2026-10-09; the run's later rulings are the paragraph after them.

**The decisions this plan takes** (the orchestrator's, through the PR and the hand-off; the owner reads them at the plan review)

- **D-1 — the predicate wiring's catcher is a cost bound, and it is the only one it can have.** The core PR proved
  `replay_strict_with(.., Predicate)` accepts exactly what `replay_strict` accepts, so no functional cell can tell
  a `Predicate` fold from a `Listed` one. The catcher is a DEBUG-profile ceiling over the committed cap-sized
  spread fixture (Task 1): under the predicate the 4,096-event replay folds in under a second; under the materialized
  list the 235 of its positions that are seat 0's Movement decisions at the 442,368-move family run to
  minutes (198 s in debug, measured). The bound is MEASURED in the before review and set at five times the
  predicate's reading (0.55 s → 3 s).
- **D-2 — the cap-sized spread fixture is cut from the review's own worst-case body**, the first 4,096 events of
  `docs/temp/security-review-2026-10-08/worst-case/spread_4p_11_body.json` re-stamped with their terminal digest
  and paired with a four-seat hot-seat table of the browser's own player — committed under
  `crates/yokohama-web/tests/fixtures/` (outside the corpus, as the browser-played pair is, since it carries a
  seed) with a provenance file naming the cut. The generator the review used (its `sec_probe_cost.rs`'s p3c probe)
  is DESCRIBED in the provenance file from the review's report — the source was the reviewer's scratch and is
  not retained; it is not a test.
- **D-3 — A3's CLI edit is `session.rs` alone, and it is where the fix belongs** (spec correction 2 of §9.3 left the
  choice to this plan, "preferring the layer where a layer-side change is equal"): the record has ONE owner, the
  `Session`, and only it can keep checkpoints consistent across `apply`, `undo` and a rebuild; a layer-side ladder
  would be a second structure invalidated by hand. The fold on the wire is the LAYER's, built from the factored
  action source (`families`/`iter_family`/`family_len`), and `screen_model.rs` is not touched: the frame builder
  takes the folded list as it takes any list.
- **D-4 — "members on demand" (spec A3) is the guided retreat the wire already serves**: `PromptState::Retreat`
  asks per occupied area how many to recover, so the retreat family's members never need listing; no new route.
- **D-5 — the quota's refusal is a 507 (`Insufficient Storage`)**, which the outbox retries and now shows: a full
  archive is the server's state, not the record's, and an operator who makes room should see the row land on the
  next drain. The outbox's contract (`web/src/outbox.ts:171–185`) ends a row on 400/409 alone; a 403 is the edge's
  challenge and a 429 the zone's rate limit, so neither can carry the quota.
- **D-6 — the live corpus replay narrows to the production-door walk under R-SEC8(b).** `walk.json`'s `wire` and
  `wire2` are seeded creates beside an OPEN seat, which a production host answers 400 (`SEED_NOT_SOLO`); 2,180 of
  the walk's 2,201 rows are at those two tables and cannot run there by the owner's own rule. The replay now
  sends each such create and holds it to the 400 in `SEED_NOT_SOLO`'s words — the live probe of the rule — skips
  every row at those tables and the lobby pages that name them (counted and printed), and runs the rest: health,
  me, labels, the house chat and its delete, `wire4` (a solo hot seat, honoured), the identity refusals and the
  eight archive rows (the once-rule's first sends included). The whole walk stays verified against the module
  through the Worker's pool (`YOKOHAMA_DIAGNOSTIC_SEEDS`, both doors) and against the browser module through
  `corpus.node.test.ts`; the lobby's and a two-person table's live reading is the live Playwright tier's. **The
  alternative, recorded:** a diagnostic-seeds deploy for the window through a `--var` override, as the wake window
  does — refused here because the owner ruled the switch off in every deployable configuration and the window
  would honour a creator's seed at a multi-person production table for its minutes.
- **D-7 — the Worker's double replay per upload stays** (≤ 110 ms after A2; the Worker's archive object takes its
  quota in S2 by spec B12) and the unknown-controller-beside-a-human residual of D25 is recorded, not closed: the
  rebuild path tolerates it the same way (`bots::seat_disagreement`), and a second rule would be a second statement.
- **D-8 — S1a's residuals:** the `record` route's lock scope gains its parked-read pin here (Task 5, the file it
  touches); M-3 (a `why` refusal bypasses the visibility read) and the served `why` chart's jsdom-only pin go to
  S1c with D12, the refusal-body row.

**Spec corrections** (to be recorded as §9.4 by Task 8, dated 2026-10-09)

1. **§2.1 A2, the timing cell.** "A timing cell over the cap-sized worst case asserting a bound" is a DEBUG-profile
   ceiling over a committed cap-sized spread fixture, measured first and set at five times the predicate's
   reading (D-1) — and it is the predicate wiring's ONLY catcher, since the two replays accept the same records
   by the core PR's proof. The release-profile reading over the review's 23,606-event body stays a measurement
   in the review record, never a cell.
2. **§2.1 A3's files and §6's CLI fence.** The CLI is touched in `crates/yokohama-cli/src/session.rs` alone — the
   checkpoint ladder and the predicate fold, behaviour-preserving (the states a session reaches are the ones
   `replay` reaches, by the core PR's proof; pinned by the CLI's suite and a new agreement cell) — and NOT in
   `screen_model.rs`: the fold is the layer's, from the factored action source (D-3). "The CLI is touched only at
   `suggest::stream_seed`" reads, for S1b, "…and in `session.rs`, nowhere the bot decides".
3. **§2.1 A3, "members on demand".** Served by the wire's existing guided retreat (`PromptState::Retreat`); no new
   route (D-4).
4. **§2.1 B12, the quota's refusal.** A 507 the outbox shows and retries (D-5); the per-caller counting is the
   NAS directory archive's, under `browser/<caller>/` alone — a server's own games are never counted.
5. **§5.3, the live corpus replay.** Narrowed to the production-door walk under R-SEC8(b) (D-6); the probe "a
   seeded create beside an open seat is a 400 on both hosts" is now the replay's own row.
6. **§2.1 D25's scope.** `check_pair_with` gains the record mirror (`deny_unknown_fields` over the engine record's
   eleven keys), the locked-down refusal, `validate_config` and `bots::seat_disagreement`; `archive_post` gains
   the foreign-bot refusal; `tables::check_shape` gains the name rule on `name`. Two residuals recorded: a record
   naming an unparseable controller beside a HUMAN seat is accepted, as the rebuild accepts it (D-7); and a
   table document's `created_by` or seat address naming another person is accepted — the archive attributes a
   file by the UPLOADER's folder (`browser/<caller>/`), and the names inside a document are data. (The plan's
   draft refused such a pair as forged attribution; the pre-flight read that rule refusing the committed corpus's
   own eight archive rows and the live replay's, which post `red@wire.test`'s hosted tables by design, so it was
   dropped — F7.)
7. **§2.1 A2, "every untrusted-record entry".** `check_pair_with` is the ONE entry (its two callers, the archive
   route and a restore); the resident rebuild, `/at` and `undo` fold the server's OWN records through the same
   predicate fold (Task 5) — not untrusted, the same cost.
8. **§2.1 D6's "compute routes".** Every dispatch leaves the executor (the blocking pool), and the GATE is on the
   five replay routes — `POST /api/archive`, `GET …/at`, `…/at/{k}`, `…/suggest`, `…/why` — never on `input`: a
   move must land while a burst of replays waits, and an undo is cheap once the ladder stands.
9. **§2.1 A2's Worker half.** The Worker's archive object still replays an upload twice (D-7): ≤ 110 ms after this
   cycle; its quota and the receipt loop's single dispatch are S2's.

#### 2026-10-10 — the run's rulings: residuals and corrections

S1b's execution found what the plan did not know, and the controller ruled on each item. The lane document (`docs/web-lane.md` §2, §4, §13, §15) says the settled ones; these are the residuals and the corrections to the plan's text above, recorded forward:

1. **A restore accepts a record naming a foreign bot that the archive route refuses.** Correction 6 placed the refusal after the validator, in the route (`bots::foreign`), and the game routes refuse such a game at play, so the backup envelope, which holds a device's own games, keeps it. Every sentence that calls `check_pair` "one validator, two callers" (the M6.5b2 spec's §2.5; D25) is scoped to what a VALID GAME is, not to whether this server runs its bot.
2. **The record mirror denies unknown keys at the record's TOP level only.** `GameConfig`, `RunProvenance` and `SeatProvenance`, and the moves' bodies, stay lenient by design, so that a reader of saves tolerates a newer field there; a key beside an event's tag is refused by the event enum.
3. **`Session::fold_event` is a hand-kept copy of core's private `fold_one_event_with(Predicate)`**, because core is frozen. Exposing the single-event fold from core would retire the copy.
4. **The hang-up cell simulates the disconnect** under a `timeout`; the behaviour over a real TCP connection rests on the reviewer's probe, not on a cell.
5. **`events`' hand-off to the blocking pool has no catcher**: no cell fails if the read goes back inline.
6. **The permit is taken before the layer's identity check and before the route's refusals**, so a refused request still queues (first in, first out). Behind Access every caller is signed in, which bounds what this costs.
7. **D27's empty-run shortcut changes cost and no byte of any answer**, so it cannot be pinned by a cell without a count of the events folded.
8. **A sentence in the fenced CLI crate is false of the web's rows since the fold.** `build_decision`'s doc in `screen_model.rs` (:2463–2464) reads "`legal` is the ENGINE's list verbatim — this builder never generates or filters moves"; its first clause is false of the web's folded, capped list, which the layer builds from the factored action source, and true of the CLI's own callers, and its second clause is true everywhere. The field doc at :977 ("One per move in the legal list the caller supplied") holds: it is true of the list the builder is handed. The CLI's own rows are whole. Left for S1c or the CLI's next touch.
9. **The CLI's `option_rows` enumeration at the spread decision** (debug 287 ms, 17.7 MB, as the run measured it) is the CLI's own cost and outside this cycle.
10. **Plan gaps the run fixed in code**: the shared-table undo's `trailing_state` (the undo half of A3), the HEAD request's bypass of the gate, the permit held in the async frame instead of the pool's closure, the put's check-then-write race on the quota, the half pair's double count, the catchers for M29, M20, M21–22 and the `fold_to` insertion, and the plan's account of the corpus (correction 1 of §9.3 stands, but the regeneration did not leave every frame payload at `0`: it added `options_omitted` to 27 frame-bearing payload files, 22 reading `0` throughout, three retreat fixtures folded to their recover-nothing member and reading 2, 3 and 3, and two carrying the fold's count on a non-deciding viewer's frame, `run.json`'s frame 0 reading 3 and `spectator.json` reading 2; nothing was renumbered, because the retreat family is the list's tail).
11. **The `CHECKPOINT_EVERY` figures in the pre-flight were wrong and are corrected in the code's comment**: a state is a few kilobytes in memory (about 15 KB serialized), and a position read or an undo folds at most sixty-three events from a complete ladder.
12. **The plan-text corrections** (the sentences about the RED readings, M6's rows, M17's mechanism, M19's status, M31's iteration, `record.rs`'s range, the renumbering and the `git add` lines) belong to the plan's own §"Review record" at the close, not to this specification.

### 9.5 — 2026-10-10, S1c's plan: the plumbing's rulings and eight corrections

S1c's plan (`docs/plans/2026-10-10-security-s1c-plumbing-plan.md`) transcribes the owner's seven rulings for the cycle, takes ten decisions of its own and makes eight corrections, recorded forward: seven of this specification and one of the board's residual list. The three sections follow VERBATIM, as the plan reads at `b58026d7` (D-6 re-copied there in Task 7's fix round; the rest as transcribed at `05334847`, unchanged since): the rulings as the brainstorm of 2026-10-10 settled them, and the decisions and corrections with the plan's amendments in place — D-1 and D-2 from Task 1's run, D-6 from Task 7's, D-7 from Task 6's, D-8 and the eighth correction on the pre-flight's reading — so what follows is what the cycle executed, not its first draft. The run's rulings are the close's paragraph after them, as §9.4's 2026-10-10 paragraph was S1b's.

**The owner's rulings for this cycle (2026-10-10), and what each binds**

The brainstorm of 2026-10-10 settled seven questions; the plan transcribes them and nothing re-opens them.

1. **D7, the watcher roster's cap: `accepted`.** The 257th unlock answering 409 is the design (`tables.rs:756–760`,
   M6.3 §3.4): a roster that forgot its oldest entry would re-lock a watcher mid-game. Filling it needs 256
   distinct identities, which only the dev door (B6) or the public door (C1, C2) supplies — S2's rows. The ledger
   row reads `accepted` with the owner's sentence (Task 9 Step 6); no code moves; the cell that pins the 409 at
   the cap stays as it is.
2. **D26, the caller segment: readable plus a digest, and a rename.** The segment keeps its readable form (cut at
   48) and appends `-` and the first twelve hex characters of a SHA-256 over the whole normalized address. Every
   existing folder on the NAS and every key under `browser/` in R2 is renamed ONCE, by the orchestrator, in the
   deploy window, from the recipe Task 5 writes into both runbooks and the `segment` subcommand it adds to the
   server binary. No legacy read path is kept in the layer.
3. **B9: no dot in a creator-chosen id, and `no-store` under `/api`.** A hosted create refuses any id containing
   `.` rather than keeping Cloudflare's extension list current; the storage rule `is_safe_id` keeps admitting
   dots, so a table stored before this cycle still loads. Every answer under `/api` — the adapter's, the three
   transport routes', the art manifest's and the fallback's 404 — carries `Cache-Control: no-store`; the static
   release (`/assets/…`, `/art/…`, `/shell.bin`) keeps its own rule (`assets.rs`).
4. **D12 and D28: one echo rule.** One helper cuts an echoed caller value to 64 characters and escapes it; every
   site that formats one uses it; `ApiError`'s constructors cap any message at 1,024 characters. The gameplay line
   is at most 256 bytes and carries no control character; the refusal of a long line names the length, never the
   text. The pre-flight measures the longest legal line (Task 9 Step 1) and records the margin.
5. **M-3 and the `why` chart.** A mismatch refusal served to a requester the account is withheld from carries the
   typed kind and a fixed sentence naming no move. The served chart's real-browser pin stays a RECORDED residual:
   it is test coverage, not a finding, and the e2e tier already runs twelve minutes.
6. **The fenced CLI doc sentence: taken.** Two doc comments in `crates/yokohama-cli/src/screen_model.rs` are made
   true (Task 7); the fence is restated as "no CLI behaviour or user-visible text moves" and the CLI suite is the
   gate.
7. **The wrap of S1.** `security-s1c` is cut from `security-s1b`'s head with a stacked PR; the whole-stack review
   before the master merge is a **four-agent Sonnet workflow run, its findings verified by the orchestrator**
   (the owner's explicit opt-in to the Workflow tool for that step, 2026-10-10; in place of a Fable whole-stack
   review); the fifteen-gate roster runs at the stack's head, as a merge to master owes it; the workspace version
   bumps to **0.6.1** at the wrap (Task 8), since the CLI's `suggest` derivation and the web wire both changed in
   S1; **the deploys are approved** — both hosts in one maintenance window, with the live acceptance, the rename
   and the five configuration changes the board lists.

**The decisions this plan takes** (the orchestrator's; the owner reads them at the plan review)

- **D-1 — the echo helper escapes without quoting, so every existing sentence is byte-identical for a short,
  clean value.** `echo(value)` returns the first 64 characters as `{:?}` renders a `str`, less its two quotes — NOT
  `str::escape_debug`, which writes `it\'s` and leaves a combining mark after the first character raw, so an
  apostrophe in a name would have read differently (the implementer's C1, 2026-10-10) — with a `…` appended when
  it cut; a site that quoted with `{x:?}` now writes `\"{x}\"` with `x = echo(x)` as a named argument, and
  `no such endpoint: {}` stays unquoted. The Worker's TS cells pin that 404's text through the module (`worker/test/worker.test.ts:677`,
  `open/door.test.ts:273`, `:280`), so a quoting change there would ripple across three suites for nothing.
- **D-2 — the message cap is TWO layers: the constructors, and the boundary.** A serde error names an unknown
  field verbatim (`deny_unknown_fields`), axum's path sentences name the segment, and a site the inventory misses
  is the one that bites. The first layer is 1,024 characters at `bad_request`/`not_found`/`server_error` and in
  the `From<GameError>` arms, so the in-memory sentence is bounded. The second (Task 1's fix round 1, 2026-10-10 —
  the owner's side note read the first layer's twenty-two holes: `api.rs`'s own struct literals, `busy()`,
  `unauthorized`, the Worker's one) is `api::error_value`, the ONE serializer behind `error_body`,
  `error_response` and `IntoResponse`, so whatever built an `ApiError` — a handler's literal included — its
  sentence leaves the layer capped, and the browser module (through `dispatch`) and the Worker module (through
  `error_body`) inherit it unedited. A private marker field was the alternative, refused: it would have forced an
  edit in the Worker crate (fenced until Task 5) and covered no host that serializes a message itself. The
  per-site 64 keeps the sentences readable. The one refusal written by hand outside both layers is the Worker's
  `identity_inner` 401 (its `lib.rs:381`, built from `User::parse`'s sentence, whose caller value the echo rule
  bounds) — recorded, S2's door rows' territory.
- **D-3 — B8's two halves are one helper and one check.** `Games::with_table_lock(id, body)` takes the lock,
  runs the body, drops its clone and prunes, so "prune on every exit" cannot be forgotten at a site; every writer
  reads the table before calling it (an absent table is a 404 that never touches the map); `tables.rs` then
  contains no `lock_for(` call, which a source-scan cell pins. The `{id}` check sits in `api::dispatch` after the
  match: an id `is_safe_id` refuses is a 404 in the words the layer already uses for an absent game — today it is
  the store's 500 (`matches`' doc, `api.rs:276–281`).
- **D-4 — the stream's end is a stream wrapper, not a dependency.** `tokio-stream` 0.1.19 has no inclusive
  `take_while`, and `futures-util` is not a direct dependency of the crate. A fifteen-line `UntilLast` stream over
  `Pin<Box<_>>` yields the frame marked last and then ends; the `then` closure marks both `end` arms.
- **D-5 — the permit is skipped when the identity did not resolve, and nothing else moves.** Public routes
  (`/api/health`, `/api/me`, `/api/labels`, `/api/art`) answer without identity through the layer, so the adapter
  cannot 401 before dispatch; the five replay routes all need identity, so a replay-route request whose identity
  is `Err` dispatches WITHOUT a permit and the layer answers its 401 having run no replay. The route's own
  refusals (a missing game, a stranger) stay under the permit: behind Access every caller is signed in, and a
  pre-read would be a second statement of the route's rule.
- **D-6 — the legacy segment is kept as a named function, in the layer, for the rename and for a cell.**
  `archive::legacy_caller_segment` is the pre-S1c mapping verbatim; `caller_segment` differs from it for EVERY
  address (a cell), so an unmigrated folder is detectable; the server binary's `segment <address>…` subcommand
  prints both for the window's recipe. No code FILES under a legacy segment or READS one — `caller_segment` itself
  calls `legacy_caller_segment` for its readable half, so "read by nothing" (the draft's words) was false of the
  function and true of the prefix (Task 7's implementer, 2026-10-11).
- **D-7 — the drifted record for M-3's route cell is built inside the cell**, from a vs-bot game the cell plays:
  the bot's last recorded move is swapped for another member of the legal list at that position, the record
  written back through the store, and a fresh `AppState` built over the same store (the restart that evicts the
  live handle). The construction was MEASURED by Task 6's implementer (2026-10-11), not by the pre-flight: the draft's swap of the
  bot's LAST MOVE never reached the route — that move was forced (a one-member list, so no other member exists)
  and the rebuild refused the cut record's stale terminal digest with a 500 — so the cell swaps the bot's last
  DECISION (the first position, from the end, whose legal list has two members; event 3, a Laboratory for a
  Skip at the corpus seed) and re-stamps the cut record's digest; a bare `why` on the drifted record reads the
  WITHHELD account of the later forced move, because the rebuild's pump replays forced moves — correct. A third
  cell pins Review Focus 5's game-over half (a route passing an EMPTY seat list survived the first two).
- **D-8 — the corpus moves ONCE, in Task 5, and nowhere else.** No frame, header or sentence a fixture carries
  changes for a short, clean value — except the archive route's reply, whose `path` names the caller segment:
  `archive_post.json`, `archive_post_identical.json` and `archive_post_abandoned.json` carry
  `browser/red_at_wire.test/…` and move to the digested segment (Task 5 regenerates once and reads the diff: three
  `path` values and nothing else). The `why` fixture that carries a `mismatch` (`why_refusals.json`) is a
  CONSTRUCTION — `wire_fixtures.rs::every_refusal_payload` builds the twelve from `explain::refusal_samples()`
  through `RefusalDto::of`, never through the route — so Task 6 moves nothing. A moved byte anywhere else is a
  finding. (The pre-flight's reading, 2026-10-10: the draft expected Task 6 to be the one candidate and Task 5
  none; the plan's own greps, run, read the opposite.)
- **D-9 — the Worker crate is NOT fenced this cycle.** Its receipt cells compute `caller_segment` dynamically
  but one pins the legacy cap (`receipt.rs:825–827`: `(70, 64)` and the stripped `.test`); Task 5 rewrites that
  premise. The five fenced crates of spec §6 (core, oracle, difftest, bridge, gen-data) and `yokohama-wasm` stay
  byte-identical.
- **D-10 — the version bump is a task of this cycle (Task 8), on the stack, before the roster**, so the roster,
  the workflow review and the merge all read the head that ships. The tag `v0.6.1` is cut on master after the
  merge (`docs/web-lane.md` §7).

**Spec corrections** (to be recorded as §9.5 by Task 7, dated 2026-10-10)

1. **B9's hosted-id rule is "no dot", not "a known extension"** (the owner's ruling 3): the list is Cloudflare's
   to change and the client never sends an id.
2. **D7 is `accepted`, not fixed** (ruling 1), with the owner's sentence in the ledger.
3. **D26's segment is readable plus a digest, with a one-time rename on both hosts** (ruling 2); spec §2.1's
   "an injective caller segment" is met in practice (a 48-bit digest collision is the only collision left).
4. **S1b's residual list on the board was stale by two items**: the live replay's lobby skip IS guarded on an
   empty rule and its 26 kept rows ARE pinned (`worker/scripts/live-replay.test.mjs:297`, the final wave
   `17085e72`, W9 and W10). S1c does not take them; the board is corrected at the close.
5. **The `{id}` an unsafe path segment carries is a 404, no longer the store's 500** (D-3); `matches`' doc at
   `api.rs:276–281` is rewritten to say so.
6. **The CLI is touched in ONE file this cycle, `screen_model.rs`, two doc comments** (ruling 6) — spec §6's
   "the CLI is touched only at `suggest::stream_seed`" is read, as S1b read it for `session.rs`, as "nowhere the
   bot decides and nowhere the user sees".
7. **The served `why` chart's real-browser pin stays recorded** (ruling 5).
8. **The corpus moves once in S1c, in Task 5, not in Task 6** (the pre-flight, 2026-10-10): the archive route's
   reply names the caller segment in `path`, so the three `archive_post*.json` fixtures regenerate once; the
   `why_refusals.json` `mismatch` row is a construction and Task 6 moves nothing (decision D-8). Still one
   regeneration per cycle, as §9.3's correction 1 allows — a value moved, not the wire's shape.

### 9.6 — 2026-10-11, S2's brainstorm and S2a's plan: the three-cycle split, the owner's twelve rulings and the four taken at the plan review, eleven decisions, six adjudications and twelve corrections

S2a's plan (`docs/plans/2026-10-11-security-s2a-browser-fences-plan.md`) transcribes the brainstorm of 2026-10-11 — S2 cut into three cycles between the hosts, and the owner's twelve rulings (X1–X6 and A1–A6) — with the owner's rulings on the plan's four flagged decisions at its review (D-7 taken, D-8 done live with the owner, D-2 and D-1 standing as proposed) and the plan review's six adjudications (AD-1–AD-6); it takes eleven decisions of its own and makes twelve corrections of this specification, recorded forward. The three sections follow VERBATIM, as the plan reads at `7d7c6dec` — unchanged since `5a03a609` but for one numeral, D-8's citation of W-1 corrected to `worker/RUNBOOK.md` §20 on 2026-10-12 (Task 7's concern 4); every other correction the run made at the source moved the plan's other sections — so what follows is what the cycle executed. The run's rulings are the close's paragraph after them (the plan's Task 9 Step 6), as §9.4's 2026-10-10 paragraph was S1b's.

**The owner's rulings for S2 (2026-10-11), and what each binds**

The brainstorm of 2026-10-11 settled the shape of S2 and this cycle's twelve questions; the plan transcribes them
and nothing re-opens them. The owner's words are quoted where they were given.

- **S2 is THREE cycles, cut between the hosts** — S2a "the browser's fences" (this plan), S2b "the Worker's door"
  (B5, B6, C1 with D24, C2, D3's socket cap, B12's Worker quota, D23), S2c "the NAS's identity" (B10, D3's stream
  cap and the body read timeout, D8, the explain surface's two rules, the inherited residuals, the wrap). The owner
  asked whether the items could be brought together under fourteen tasks a cycle; counted by shared mechanism,
  two cycles put fifteen tasks in the second, so three: "if so, then we should go with three".
- **X1 — the branch.** `security-s2` from master `1e9c092d`; S2a on it; `security-s2b` and `security-s2c`
  stacked; a PR per sub-cycle against the stack; ONE merge to master at S2's wrap ("this is how I would structure
  it — so we're not delayed if this work delays us further").
- **X2 — two windows, two versions.** S2a ships `0.6.2` with the policy in Report-Only; the wrap ships `0.6.3`
  enforcing ("the delay is a non issue, but it seems like a nominal amount of work to set the flag so the two
  windows is fine").
- **X3 — the tiers.** Opus reviewers for every task; Sonnet implementers only where the brief carries the whole
  judgment (Tasks 5 and 6 here); the wrap's stack review as S1's ("agreed").
- **X4 — the pre-flight.** S1c's shape: the first two tasks' cells executed and every figure measured, the rest to
  the task reviewers ("that should work").
- **X5 — the two live readings never taken** (D1; B1's withheld `why`) — the owner: "this is tricky, we're on a
  warp connected endpoint … I added myfreemovies@gmail.com and I can sign that into chrome so it can be tested
  with claude in chrome if we disable the warp-cli but moving back and forth between the two identities would be
  complicated depending on how the task itself is structured — unless you can use a service token for one of those
  identities." A service token names nobody on the NAS (the review's V1), so the structure is decision D-8's one
  block, done LIVE with the owner (the owner's ruling at this plan's review, below).
- **X6 — the service token stays long-lived.** "The service token is a dev function, it's not important to how
  this will be released as an open project … purely there for easier testing and dev, I don't see an issue with
  leaving it with the longer duration on this authenticated endpoint." B6's "shorten/rotate" half reads
  `accepted` in those words (S2b's ledger row). The owner's framing of the end state, kept for S2b and S4: "this
  project will be when hosted by me on the cloudflare worker open door without identity but we will have another
  one using identity and the NAS version I host will always be behind access but for others that want to host it
  internally or otherwise they can use it or not depending on how they manage their network."
- **A1 — the header set, taken as the c1 review wrote it**, with three choices inside it: the `connect-src` hosts
  generated from the build values; `404.html`'s inline style kept and HASHED (decision D-4); Bot Fight Mode's
  injected detection script accepted as the one known Report-Only line, never `'unsafe-inline'`. The owner's
  constraint, binding on every task: "we want to always make sure that our front door/landing page is always
  static assets and we don't get to worker invocations until after the turnstile is passed for multiplayer events
  on the cloudflare worker … If the impact of the decision in A1 will not impact user's ability to use the site on
  a first visit, then this decision is fine. Bot Fight Mode needs to stay on." The asset layer applies `_headers`
  with zero invocations; Report-Only blocks nothing; the enforcement question is the wrap's, from this release's
  reading, between three shapes (accept the refusal if first visits are not challenged more; a fixed nonce, which
  Cloudflare copies onto the script it injects by parsing the header; `'unsafe-inline'`).
- **A2 — the Worker's `no-store` lives in `handle`**, one place on the way out, for every answer but a 101
  ("agreed").
- **A3 — D16 REVERSED: `accepted`.** "If a game has finished it should auto-upload, even if they pull back a game
  and finish it, presumably they don't finish it identically, and so it's still useful to get more completed games,
  because we're using those for training detail later on with the v5+ computer player development." No code
  moves; the ledger row reads `accepted` with that sentence and two consequences recorded (the upload counts
  against the restorer's share, a 507 being a wait; the once-rule is per caller segment, so a restored copy may sit
  beside the original device's under another segment, deduplicated by digest when the archive is read).
- **A4 — D14**: the hash router admits `is_safe_id`'s grammar, pinned to an emitted fixture ("confirmed").
- **A5 — the service worker's pass-through and the `window.top` check** ("confirmed").
- **A6 — the guard's three rules**: `workers_dev: false` under every door with the sibling rule for every
  open-door file; `--config` explicit and a redirect file refused; the deploy lists the Worker's secrets and refuses
  one named like a switch ("agreed").
- **S2b's and S2c's values** are ruled at their planning sessions, the orchestrator's recommendations the default
  ("my instinct is to use the suggestions you make"); the rate-limit figures derive from the measured calls per
  move for a couple of games at once ("as long as … those values align with a user playing a couple of games at a
  time"); **B10 under a dependency fence**: "we have a minimal set now, and when we get to running with apache
  2.0 we want to make sure we remain compliant" — minimal, Apache-2.0-compatible crates, the `cargo tree` diff
  recorded, nothing new in the wasm or fenced crates.
- **The owner's rulings on the plan's four flagged decisions (2026-10-11, at the plan's review).** **D-7 taken**: "we can
  cut the release on the branch that's fine since we're deploying it out" — the tag `v0.6.2` is cut on the deployed
  `security-s2` commit and the release published before either deploy; `docs/web-lane.md` §7 records the exception
  (Task 7). **D-8 is done LIVE with the owner**: "we'll have to work on this in real time for the sign ins so we'll just
  have to note that at the start and get an idea when that gets slotted" — the two-identity block needs the owner at the
  keyboard for the two Access sign-ins, so Task 9 Step 9 opens by naming the block and agreeing its slot in the window
  before any deploy begins; the block's readings are taken in that slot or recorded as not taken, never improvised, by
  the means D-8 names. **D-2 and D-1 stand as proposed** ("not sure what I can add to this", twice): Cloudflare's three
  hosts named in the policy; a dist without `headers.json` serves no set on the NAS with one stderr line and refuses the
  Worker's assembly (the review corrected D-1's supporting parenthetical about the smoke, not the decision).
- **The plan review's six adjudications (the orchestrator's, 2026-10-11; the record is §"Review record")**: AD-1 the
  local Playwright tier is served the policy ENFORCED and the live tier reads a CSP line at any console level, forgiving
  the one measured Report-Only line (D-9 rewritten); AD-2 `headers.json` is an input of the Worker's assembly, never an
  asset; AD-3 the fifteen-gate roster runs once at S2a's code head, after the bump (spec §5.2, as S1c's did at
  `04713bed`); AD-4 the NAS's header reading is live, after the health read; AD-5 the block's probes run inside each
  signed-in profile (D-8); AD-6 the pass-through carries `Cache-Control` and says so (D-5).

**The decisions this plan takes** (the orchestrator's; the owner read them at the plan review, 2026-10-11)

- **D-1 — the set is generated ONCE, at the client build, from the checked build values.** `web/scripts/headers.mjs`
  is the one place a directive is spelled; `vite build` writes the set to `dist/headers.json` beside
  `build-values.json`; the Worker's assembly appends it to `_headers` as the `/*` block and the deploy guard
  recomputes that expectation; the NAS reads the file at start. Neither host holds a copy of the policy text, so
  the two cannot drift, and a changed feed host or site address changes `connect-src` without an edit. A build
  without the file is REFUSED at the Worker's assembly (the tree would ship no headers) and served WITHOUT the set
  on the NAS with one line on stderr at start (a dev run with an older or absent client build still serves; the
  self-host image's smoke reads the headers in CI, `docker.yml`, so that image cannot ship without them; the NAS's
  runtime image, assembled by Portainer from the tagged commit's own `dist`, is read live in the window — Task 9 Step
  9(a)).
- **D-2 — Cloudflare's three hosts are always named.** `script-src` carries `https://challenges.cloudflare.com`
  (Turnstile and the Managed Challenge) and `https://static.cloudflareinsights.com` (the Web Analytics beacon the
  zone injects into every HTML answer), `frame-src` the first, `connect-src` `https://cloudflareinsights.com` (the
  beacon's post). Both deployments sit behind the zone, the beacon stays on by the owner's standing rule ("known
  errors over disabling features"), and naming the hosts is a precise allowance, not a weakening of substance. The
  one script the policy cannot name is Bot Fight Mode's inline detection script, which carries no nonce a static
  shell could match: it is the known Report-Only line of this release (A1), read in the window.
- **D-3 — the Worker's API answers carry `no-store` and `nosniff`, not the whole set.** Cloudflare's rule: a
  `_headers` block never applies to a Worker-generated answer, so `handle` wraps every answer under `/api` on the
  way out with the two headers that matter for a JSON body (cache deception; MIME confusion) and returns a 101 as it
  came. The whole set on JSON would buy nothing, and the shell holds no `headers.json` to read. The static release
  — `/`, `/shell.bin`, every asset, the 404 page — carries the whole set from `_headers`.
- **D-4 — `404.html` stays self-contained; its inline style is hashed.** The page is designed to make one request
  (its icon). CSP admits an inline `<style>` by the SHA-256 of its exact text, so the generator hashes
  `web/public/404.html`'s style block at build and `style-src` names it; the page's text and the hash move
  together or the policy reports it. Moving the style to a file was the alternative, refused for the request it adds
  and the design it undoes.
- **D-5 — the service worker copies every stored header but three.** `shellResponse(hit)` carries the stored
  `/shell.bin` entry's headers — the host's `/*` set among them — onto the navigation it synthesizes, except
  `content-type` (set to HTML, as before), `content-length` and `content-encoding` (the stored body is the
  DECODED bytes the install digested). `Cache-Control: no-store` IS carried: `/shell.bin`'s directive lands on
  every synthesized navigation, which keeps a signed-in document out of the HTTP and back/forward caches — the
  conservative side for B4 and D13 (the bfcache cost is unmeasured: Playwright's Chromium disables it either way).
  A named allow-list was the alternative, refused: a header the set gains later would be dropped in silence — and
  the cell carries a header no cell names, so an allow-list cannot pass it.
- **D-6 — the hash router's grammar is pinned to the store's by an emitted fixture**, as the name rule is pinned
  to `identity_cases.json`: `wire_fixtures` emits `id_cases.json` from `store::is_safe_id` over a case list, and
  `web/src/tableId.ts` is held to its verdicts both ways. `parseRoute` stays total — an id the grammar refuses is
  the door, never an exception and never an API path. The one existing cell this changes is `#/t/a%20b`, which read
  as a table named `a b` and now reads as the door: a space is outside the grammar.
- **D-7 — the tag `v0.6.2` is cut on the DEPLOYED commit of `security-s2`**, since nothing merges to master before
  S2's wrap (X1) and the hosts deploy after S2a (X2). `docs/web-lane.md` §7's "on master" is read as "on the
  commit the hosts run": the wrap's merge brings the tagged commit onto master, and `release.yml` runs on any `v*`
  tag. The release is published BEFORE either deploy (the owner's standing order from S1's wrap).
- **D-8 — the two-identity block of the window** (X5), done LIVE with the owner (the owner's ruling above): WARP
  off for one bounded block whose slot is agreed at the window's start and recorded; two Chrome profiles signed in
  through Access's login — the owner's and `myfreemovies@gmail.com` — on the NAS hostname (and, for the §8 rows,
  on the Worker's); **each probe runs INSIDE the profile it speaks for**: a page-context `fetch` or `EventSource`
  at the host from that profile's own tab (Claude in Chrome's `javascript_tool`) — Chrome's TLS and Chrome's
  cookie, nothing extracted, nothing printed, nothing logged; nothing switches identity mid-probe. If a probe must
  leave the browser, the owner copies that profile's `CF_Authorization` from DevTools into a `umask 077` file used
  as `curl -H @file` and deleted after — the value never in a tool result, a shell history or `ps`. The cookie is
  `HttpOnly`, so no page script reads it and no tool of ours returns it without logging it; and off WARP a `curl`
  is not a transport at all, whatever its `User-Agent` (W-1, `worker/RUNBOOK.md` §20, 2026-10-01: "a browser
  User-Agent, the navigate headers and a fake Googlebot change nothing — the TLS fingerprint is the tell"). The
  owner's rule for every `curl` the window runs OVER WARP stands as given: it carries a browser `User-Agent` (the
  release smoke's `BROWSER_UA`) or meets Bot Fight Mode's 403 before Access sees it (the owner, 2026-10-11: "when
  using curl to cloudflare, you need to include user agent details so it looks like a normal browser when a site is
  access protected or cloudflare will fight it"). The probes are Task 9 Step 9(d)'s: D1 at an open table A fills
  itself; B1 at a bot-first table; rows 108 and 119 as the second identity on the Worker host. A service token
  cannot stand in: Cloudflare strips the identity header on a token request, so it names nobody on the NAS (V1).
  WARP back on at the block's end. Nothing else in this cycle depends on it.
- **D-9 — the local Playwright tier is the enforcement reading, served the policy ENFORCED; the live tier reads
  the Report-Only release.** Measured at the plan review (Chromium 143–153, Chrome 155): Chromium logs a
  Report-Only violation at console type `info` — "Executing inline script violates the following Content Security
  Policy directive 'script-src …'. … The policy is report-only, so the violation has been logged but no further
  action has been taken." — and an enforced one at `error`; `watchForBreakage` read `error` alone; and a violation
  raised INSIDE the module Worker (the wasm compile without `'wasm-unsafe-eval'`) prints on no console channel in
  either mode. So the local walk is the enforcement reading only when it is served the policy ENFORCED:
  `web/scripts/e2e-build.mjs` rewrites the e2e build's `dist/headers.json` under `Content-Security-Policy` after
  the client build — an override that can only strengthen; the shipped `CSP_MODE` stays the constant, with its cell
  — so an enforced violation is a console `error` the walk fails on, and a Worker-internal one fails the walk itself
  (no game is dealt). No local forgiveness is added. The LIVE tier reads every CSP line at ANY console level and
  gains exactly one forgiveness, counted apart as the beacon's is: the Report-Only refusal of Cloudflare's injected
  inline script on a first load (D-2), matched on its measured text and the page's ORIGIN; a red live tier at the
  window is a finding, never a hole. The pre-flight's enforce run (Task 9 Step 1) stays: it reads what enforcement
  blocks before anything ships (the review's reading: 77 of 81 cells green on the real NAS binary, the other four
  red identically with no header set).
- **D-10 — the version bump is a task of this cycle (Task 8), on the branch, before the after review**, so the
  review and the deploy read the head that ships (S1c's D-10).
- **D-11 — the secrets guard runs `wrangler secret list` and judges its JSON with a pure function.** The deploy
  guard proper stays offline; a second script in the deploy commands asks wrangler for the names and refuses
  `YOKOHAMA_DIAGNOSTIC_SEEDS` and `YOKOHAMA_WEB_TEST_CLOCK` — the two switches the guard already refuses in a
  committed file, which a secret would carry past it (the S1 stack review's unverified HI item).

**Spec corrections** (to be recorded as §9.6 by Task 7, dated 2026-10-11)

1. **S2 runs as three cycles, cut between the hosts** (the owner, 2026-10-11): S2a the browser's fences, S2b the
   Worker's door, S2c the NAS's identity — §3.3's one plan becomes three, each written against the stack's head.
   **Two windows and two versions** (X2): §5.3's "after each cycle's deploy" is per sub-cycle where one deploys;
   S2a ships `0.6.2` in Report-Only and the wrap `0.6.3` enforcing.
2. **B4's Access half (R-SEC8(g)) was done at S1's wrap** (2026-10-11: the cookie `SameSite=Lax`, the session
   24h) — §3.3's "made before the cycle's live acceptance" was met before S2a began.
3. **A `_headers` block never applies to a Worker-generated answer** (Cloudflare's documented rule for Workers
   static assets), so §2.2 B4's `worker/static/_headers` is the STATIC half; the API's headers are `handle`'s
   (D-3), and the asset layer applies `_headers` with zero Worker invocations (the owner's A1 constraint).
4. **D16 is `accepted`** (the owner, A3): a restored finished game uploads as any finished game does, for the
   training corpus; the attribution to the restorer is accepted, the record's own `created_by` and seat users
   naming its players.
5. **D24 collapses into C1** (S2b): once an email-shaped claim is refused under `open`, a claim is an ASCII name
   and always a legal header; under `worker` and `cf-access` the identity rides the cookie, never a client header.
6. **B6's "service token rotated to a shorter duration" is `accepted` as it stands** (the owner, X6).
7. **The policy names Cloudflare's analytics and challenge hosts** (D-2); **the JS detections' inline script is
   the known Report-Only line**; the enforcement shape is the wrap's decision from this release's reading (A1).
8. **The two live readings not taken (D1; B1's withheld `why`) are read with a second Access identity by the
   cookie block** (D-8), not a service token (V1).
9. **The stack review's "a crafted bare name equal to another address's current segment" is closed by the rename
   plus the digest suffix** — every segment now ends in twelve hex of its own key's digest, so no legacy folder
   exists for a name to equal; a cell is S2b's, beside the 48-cut WASM cell.
10. **§5.3's "from S2 on, the response headers of `/`, `/shell.bin`, an asset and `/api/health` on both hosts"**:
    the expected reading is the set of Task 1 on `/`, `/shell.bin` and an asset on both hosts; on `/api/health`
    the whole set plus `no-store` on the NAS, and `no-store` plus `nosniff` on the Worker (D-3).
11. **B4 is enforced by `X-Frame-Options: DENY` alone until the wrap**: `frame-ancestors 'none'` rides the
    Report-Only policy in 0.6.2 and is enforced in 0.6.3 (X2); §2.2 B4's "from both hosts" is met by the header
    in this release and by both in the next.
12. **The wire corpus moves in S2a** by one new contract file (`id_cases.json`, Task 5) and the version fields
    (Task 8); §6's "once, in S1" was S1's.

### 9.7 — 2026-10-12, S2b's brainstorm and plan: the owner's rulings R-1–R-6, thirteen decisions and twelve corrections

S2b's plan (`docs/plans/2026-10-12-security-s2b-worker-door-plan.md`) transcribes the brainstorm of 2026-10-12 — the owner's six rulings for the cycle, R-1–R-6, with X1, X3, X4 and X6 standing from 2026-10-11 — takes thirteen decisions of its own (D-8 withdrawn at the plan review and kept by its number, so D-9–D-13 keep theirs) and makes twelve corrections of this specification, recorded forward. The three sections follow VERBATIM, as the plan reads at `8ded960b`, the last of the three corrections the run made inside them at the source: `6e23702c` set the write limit at 120 per 10 s on the pre-flight's reading (the first text's 60 did not stand; the burst recipe's 75 POSTs and "the 61st" moved with it), `226e50e9` corrected D-4 at Task 3's review (the socket caps count OPEN sockets only), and `8ded960b` corrected D-7 and correction 9 at Task 9's review (a new Worker's first deploy runs `wrangler deploy` with neither guard, the deploy guard holding this repository's Worker alone) — so what follows is what the cycle executed, not its first draft. The plan's three headings are set as this section's bold lines, as §9.5 and §9.6 set theirs; every other line is the plan's byte for byte. The run's rulings are the close's paragraph after them (the plan's Task 10 Step 6), as §9.4's 2026-10-10 paragraph was S1b's.

**The owner's rulings for S2b (2026-10-12), and what each binds**

The brainstorm of 2026-10-12 opened with the board's "NEXT" still the first S2 window, un-run; the owner opened S2b's
brainstorm instead and ruled on seven decisions and the method. Every word below is the owner's, quoted as given,
and nothing re-opens them. X1 (one stack, `security-s2b` stacked on `security-s2`, a PR against the stack, ONE merge
at S2's wrap), X3 (Opus reviewers; Sonnet implementers only where the brief carries the whole judgment), X4 (the
pre-flight executes the first two tasks' cells and measures every figure) and X6 (the service token stays
long-lived; B6's "shorten/rotate" half reads `accepted`) stand from 2026-10-11 unchanged.

- **R-1 — the method and the hand-off.** "Once you're ready with the plan, please run the plan through a four sonnet
  agent ultracode review, with an opus agent for collating and deduplicating the results and verifying for you - so
  you get a list of items to review and update the plan to. Afterwards, with the plan presented and memory updated -
  we can take it from there. If the plan is dependent on the deploy being done to the updated version - let me know
  and I'll run that with another agent before runnign this plan through. If it doesn't make a difference, then we
  can just run through this plan and run the deploy afterwards - either way let me know in the handover. I'll review
  before beginning the next session." **The answer, recorded:** this plan does not depend on the 0.6.2 deploy. S2b
  has no window and no version (X2); its code goes live at the wrap in 0.6.3; the first window still tags and
  deploys S2a's commit `af7f30d5`, whenever it is slotted, and S2b's branch is untouched by it. The window is OWED,
  any slot, and the board says so. **It is run from the `v0.6.2` tag's tree** — a `git archive v0.6.2` export, or a
  clean checkout of `af7f30d5` — never from a `security-s2b` checkout: S2b's tree has no `deploy:gate`, ships the dev
  door OFF (the live replay and the live Playwright tier would find it shut) and binds the rate limits before the
  owner has confirmed that namespaces 7301 and 7302 are free. The board's "or the head after the parked sentences,
  if you take them" falls away: S2b's Task 8 takes them, on `security-s2b`. So the deploy does not have to run
  before this plan, and this plan does not have to wait for it (verified at the plan review, PF-09).
- **R-2 — D-1, the dev door's standing state: option (b).** "Understood - agreed B on D-1." The owner's framing,
  which (b) is built to honour: "I don't think of the gate as a dev function entirely, the dev door is different from
  having the access gate - people that use this may just want to have access in their environment - that's different
  from wanting it online or even on a LAN with no identity layer." So the Access-alone shape IS the production file,
  `wrangler.jsonc`, with the dev door OFF; the one file that opens the dev door is `wrangler.dev.jsonc`, deployed
  around a live tier and reverted by the plain deploy; `wrangler.gate.jsonc` goes, its content now production's.
- **R-3 — D-2, the open door's key rule: R-SEC8(c) stands, and the public identity model is out of scope.** The
  owner's questions, answered in the brainstorm and recorded in decision D-2: "For D-2, my understanding is even if
  we remove the access layer, we still request an identity from a user, and even though we don't verify it through a
  layer like access - we can have players come on and put in their email and play. If this produces an architectural
  dilemma, we may need to consider how to handle this - but the ideal was just to have the game be playable by anyone,
  and at first we would launch this where the public deployment was behind access for multiplayer but the home page
  was open but ideally we open a lobby to anyone that wants to play. This could create a problem where people can put
  in other people's email addresses, how would we handle that today with the codebase as it stands now?" Then, on a
  second person typing the same name at a full open deployment, the answer that under the open door the second
  person IS the first (the key is the typed name, normalized; nothing else is bound to anything). Then the ruling:
  "Access would not be used for when this is public it's not the right auth model at all - I don't think it is
  scalable to pay $7/month for an auth identity. auth0 allows 25,000 monthly active users on their free tier, we
  would have other options here. I don't think we need to get into that identity model yet for outside access that
  would be out of scope for now - we can keep it with the current design, so when we open things up, we have the home
  page wide open, but then the multiplayer gets access gated post turnstile and we can consider a middle ground like
  layering in auth0 or some other solution that fits to track users that won't carry a cost or affect our security
  posture", clarified: "let me be more clear, access would not be used by us specfically on the yokohama.moore.nyc -
  but we'll have another private deployment worker, and the NAS which is access, and others can deploy it however
  they want once its open sourced - but we will have a deployment for anyone to use, and that we will probably want
  to mint some kind of controls for - but that's not something we have to be making decisions on right now unless you
  feel that's important for this phase." **Recorded for S4, not decided here:** the public Worker at
  `yokohama.moore.nyc` runs the OPEN door behind the Turnstile with controls to be minted later; a second private
  Worker and the NAS stay behind Access; a self-hoster deploys as they like. Nothing in S2b presumes otherwise, and
  C1's name rule is exactly what that public open door runs.
- **R-4 — D-3, the rate-limit binding on Workers Free.** "D-3, I'm not sure if this rate limit mechanic works the same
  way on the cloudflare workers free tier - if it does than this is fine." The documentation gates the binding by no
  plan (the pricing page lists Free-plan limits for Workers, KV, D1, Durable Objects and R2 and none for rate
  limiting; the binding is GA since 2025-09-19), so this plan takes it; the live proof of plan availability is the
  wrap's deploy, which refuses a binding the plan does not allow before anything else runs, and the fallback if it
  ever refused is the review's Durable Object token bucket, NOT built ahead of the refusal (decision D-3).
- **R-5 — D-4, D-5, D-6 as proposed.** "the other items you note here I take no issue with" — the socket caps and
  their refusal shape, the live-count quota, the log digests with query-string redaction and the sampling rate kept.
- **R-6 — D-7, S4's first-deploy path: a documented recipe, and no guard code.** "noting D-7 is not really a call that
  is open to make hnless I'm misunderstanding - I agree building a guard that way before the worker exists doesn't
  make much sense and wouldn't fit the flow of how someone would set this up anyway unless we were getting into
  scripting the setup, which is a feature that isn't being considered/we probably aren't going to need that."
- **The values are the orchestrator's recommendations by the owner's standing ruling of 2026-10-11** ("my instinct is
  to use the suggestions you make"), with the rate-limit figures held to the owner's bound ("as long as … those
  values align with a user playing a couple of games at a time").

**The decisions this plan takes** (the orchestrator's; the owner read D-1–D-7 at the brainstorm, 2026-10-12)

- **D-1 — the dev door is OFF in the production file, and ONE file opens it.** `wrangler.jsonc` sets
  `YOKOHAMA_DEV_DOOR` to `off`; `wrangler.dev.jsonc` is production with that one variable `on`, deployed by
  `npm run deploy:dev` before a live tier and reverted by the plain `npm run deploy` after it, as `deploy`/`deploy:gate`
  were before with the directions swapped so the safe direction is the plain command; `wrangler.gate.jsonc` is deleted,
  its content being production's now; `npm run deploy:gate` goes with it (the dated deploy records that spell it are
  records, and stay). The guard (D-10) holds the dev file to production with exactly `DEV_DIFFERS`
  (`YOKOHAMA_DEV_DOOR: "on"`), refuses `on` in any OTHER worker-door file, requires exactly `"on"` in the dev file,
  and holds every unlisted worker-door `--config` file to production exactly (F-40). A self-hoster copying the
  production file now copies Access alone. The pool's `wrangler.test.jsonc` keeps its dev door `on`: the dev-door
  cells need it, and the pool never deploys.
- **D-2 — the open door's key rule lives in the layer's mode rule, as a named policy; the Worker shell judges no
  claim — its one change collapses a claim's space runs, so a key names one Archive object (D-5).**
  `identity::ClaimPolicy { NamesOnly, Diagnostic }`, the seed rule's own shape (`SeedPolicy`, security S1a): under
  `IdentityMode::Open` with `NamesOnly`, a key with the email shape is refused by `admit` with ONE sentence,
  `identity::ADDRESS_NOT_A_NAME`; under `cf-access` and `worker` the policy changes nothing, since an address is the
  only admitted shape there. Every production constructor builds `NamesOnly` — `AppState::production` (the NAS's
  `main`), and the Worker module's `identity_of`, which has no diagnostic arm at all; `AppState::with_store` and the
  `in_memory*` builders build `Diagnostic`, so the corpus walk (`wire_fixtures`, the open door as `red@wire.test`) and
  the layer's cells stand and the corpus does not move. The Worker shell forwards an address-shaped claim as it does
  today, and the layer refuses it inside the objects — so the NAS and the Worker answer the SAME sentence (the
  review's second refusal in `claimedIdentity` would make the Worker answer the absent-caller sentence instead). The
  client's name step, which exists under the open door alone, refuses any input containing an `@` before the key
  grammar runs and before any request (`nameRule::openDoorVerdict`), so every `@` is answered in the door's own words
  and never with the grammar's sentence about writing an address; pinned over `identity_cases.json`'s `@` rows; the
  key grammar and that fixture are unchanged. Two consequences recorded: under the open door no claim is ever a test
  identity, so an open deployment's browser saves file at its bucket's TOP root — the public Worker's own bucket, by
  R-SEC6; and D24 collapses, since a plain name is ASCII by the grammar and always a legal header (§9.7 correction
  10). What this rule does NOT do, said once: it stops no impersonation under the open door, where a name is a label
  and a key and never a credential; it keeps the two key spaces disjoint, an `@` always meaning a verified address,
  so a door flip over shared storage can never hand an Access player's seats and archive folder to whoever types
  their address. Under the open door the local Playwright tier — the release binary, `production`, `NamesOnly` —
  plays by NAMES, and the live tier by `.test` addresses through the dev door: the harness's identity follows the
  target (Task 5b). A seat is a reservation, not a caller: the layer does not judge a seat's user by the door, and
  the new-table form's "their name" field refuses an `@` under the open door as the name step does (Task 5b); a
  layer rule for seats belongs to S4, with the public open door.
- **D-3 — two Rate Limiting bindings, keyed by caller or by IP, with the figures from the measured calls per move.**
  `CREATE_LIMIT` (namespace 7301) 10 per 60 s on `POST /api/games`; `WRITE_LIMIT` (namespace 7302) 120 per 10 s on
  every other unsafe method, `/api/archive` and the operator's POST included. The key: under `worker` a caller's
  digest (`u:` and SHA-256 of the address in hex), under `open` the connecting IP for creates (`ip:<cf-connecting-ip>`)
  and the IP plus the claim's digest for writes; a request with no caller is keyed by IP under either door; an absent
  `cf-connecting-ip` reads `unknown` (the pool). A refusal is 429 with `Retry-After: 10` and the host-written sentence
  `TOO_MANY`, answered before any object is reached, under `handle`'s two API headers; reads and upgrades are never
  limited (D3 caps sockets). The basis is `docs/web-lane.md` §16's reading of cycle F's live tier — 7.30 `/api`
  requests a move over 1,741 moves in 470 s, `/input` 1.13 a move, so a seat at machine speed writes about 21 per
  10 s ON AVERAGE (the tier's peak of 338 `/api` requests in 10 s holds about 52 unsafe ones, so the pre-flight's
  per-page peak decides whether 60 stands, Task 10 Step 1(a) — MEASURED 2026-10-12 at the pre-flight: the local
  two-browser walk's pages peak at 71 and 69 unsafe requests in a sliding 10 s window, almost all `POST …/input`, with
  a median fixed bucket of 56 and 52, so 60 would trip on ordinary buckets and does NOT stand; the write limit is **120
  per 10 s**, 1.7× the measured peak, the live tier being slower than the local walk, never faster); a person with two games is ESTIMATED, not measured,
  at about 4. The live replay needs no pacing: since S1b it sends 26 of the corpus's 2,213 rows at the production
  door — 17 writes, all as one identity, in about 4 s (`worker/RUNBOOK.md` §21; `docs/web-lane.md` §2) — so the
  write limit never binds it (D-8, withdrawn). The dev door is NOT exempt: the instruments play under the limit,
  which is what bounds a stolen credential too. The pool carries both bindings with a LENIENT figure (1,000,000 per
  60 s), because `test/worker.test.ts` sends more than ten creates a minute as one caller through `handle`
  (measured: at production's 10 per 60 s two of its cells answer 429) and miniflare's ratelimit plugin counts for
  real; the corpus replay is not the reason — `test/walk.ts` drives the objects directly and reaches `handle` for
  `/api/art` alone, so it never spends a limiter; the guard pins production's block to its literals and the pool's
  to the two names (D-10); the shell's logic is proven in the pool through a fake limiter handed in `env`, and the
  live refusal is read at the wrap's window by `worker/RUNBOOK.md` §7's burst recipe (140 credentialed POSTs as the
  owner over WARP, reading 429 and `Retry-After` from about the 121st inside one window; `probe-burst.mjs` sends GETs
  alone and can read no 429). Platform facts recorded: the binding is GA; `period` is 10 or 60 only; counters are
  per Cloudflare location and best effort; a `namespace_id` is a STRING in the configuration (wrangler 4.136.3
  refuses an integer by name), unique to the account, and a shared id shares counters across Workers — the owner
  confirms no other Worker on the account uses 7301 or 7302 before the wrap's deploy. A refused create answers
  `Retry-After: 60`, a refused write `10` — each its own window. A limiter that throws is logged `limiter-failed`
  and the request proceeds: the limit bounds cost, never access, and a platform fault must not take the game down.
  Under `open` the write key is (IP, claim) and a claim is free, so the write limit is per name, not an IP ceiling;
  creates are IP-only — the spec's own key. If the wrap's deploy ever refused the binding, the fallback is the
  review's Durable Object token bucket, not built ahead of the refusal (R-4).
- **D-4 — the socket caps, and a refusal the client already reads.** A table object admits at most
  `SOCKETS_PER_CALLER` (8) sockets per caller and `SOCKETS_PER_TABLE` (64) in all; the house `LOBBY_PER_CALLER` (8)
  and `LOBBY_TOTAL` (1,024); the platform's own ceiling is 32,768 per object and the client holds one socket per
  visible tab. Every socket is tagged with the table's id (as today) AND a caller tag, `u:` and SHA-256 of the
  caller in hex — computed in `fetch`, which is async, and carried in `Incoming` — so the count is
  `getWebSockets(tag).length`; a tag is at most 256 characters and keeps the address off the platform's tag store. A
  refused upgrade is ACCEPTED, sent `end` carrying the sentence (`endAtAdmission`), and closed `4429` — the shape
  `signal.ts` reads as `ended`: the game screen draws it (`useSignal`'s `onEnd` navigates with the sentence); the
  lobby does not — `Lobby.tsx` passes no `onEnd`, so a refused lobby socket leaves the word `live` and the last
  listing until reload, recorded and parked (AD-4: a client design question, owed before any open door is deployed);
  answered unupgraded, the client's ladder would reconnect and hammer. **The counts take OPEN sockets only**
  (`readyState === WebSocket.OPEN`, the `openCount` helper): a refused or ended socket stays `CLOSING` for as long as
  its client declines to answer the close — an unaccepted pool client never does, and a hostile one need not — so a
  count that included it would let a stranger's unanswered 403s lock a table's seated players out and one identity
  fill the lobby in two seconds (measured at Task 3's review: 60 dangling refusals, 68 held; 1,032 held in the lobby;
  a seated player refused after a stranger's 64). The CLOSING sockets still occupy the object until the platform
  tears them down — the pre-S2b shape, under the platform's 32,768 — recorded in §15. (As first drafted D-4 said a
  CLOSING socket "may still be counted" and "the caps carry that headroom": false for a client that never answers.)
- **D-5 — the Worker's quota is a live count, not a counter.** Before each save the archive object lists the caller's
  share prefix in R2 (`list({ prefix, limit: 1000 })`, one Class A operation) and hands the listing — `truncated` and
  every object's `key` and `size` — to the module through a new export, `set_share`; the receipt archive's `put`, for a
  `browser/` prefix, counts records and bytes over that listing LESS the plan's own two keys (the NAS's `usage` rule:
  a half pair's repair never counts its own table twice) and applies `Quota::PRODUCTION`'s arithmetic — the NAS's
  figures (256 records, 128 MiB), the NAS's sentence through the route's 507 — BEFORE deferring a plan, so a full share
  uploads nothing. A truncated listing is over the quota by construction (1,000 objects). No listing set is
  `Failed(SHARE_UNCOUNTED)`, the layer's 500 "the archive could not be written", which the outbox retries: fail
  closed — and every save begins with none, `share_prefix` clearing whatever an earlier request left (a request the
  layer refused before the put never spends its listing, so the next save must not). The prefix comes from a second
  new export, `share_prefix`, since the root (`test/` for a `.test` address) and the segment are the module's to
  compute. The object keeps no storage; the `test/` lifecycle's deletes correct the count by themselves; the
  object's serialized request path under `blockConcurrencyWhile` closes the check-then-write race the NAS needed a
  mutex for, PER OBJECT, and one key is one object: under `worker` an address is canonical, and under `open` the
  shell collapses a claim's space runs (`claimedIdentity`, Task 5), so two spellings of one name never race over one
  share (measured at the plan review: without the collapse, 257 records in a 256-record share). A server game's own
  save (the empty prefix) is never counted, as on the NAS. The 507 and its figures equal the NAS's; the ORDER does
  not: the NAS reads the once-marker before it counts, and the module, which cannot read R2, judges first and
  excludes only the put's own two keys — so a retry of a record already filed passes at an exactly full share and
  meets the 507 in a share already over its bound. Every POST to `/api/archive` lists once before the layer reads
  the body, so a save the layer refuses cheaply still costs one Class A operation; the write limit bounds it.
- **D-6 — what the shell logs, and the query string.** The two `operator` lines carry `operator: <12 hex>` — the
  first twelve of SHA-256 over the address — and no address; every line that carries a record key — the
  `record_key` field, and `logOutcome`'s `detail`, which IS the key for a landed, identical or conflicting outcome
  and begins with it in one failed sentence — carries it through `redactKey`, which replaces the
  `browser/<readable>-<12hex>/` component with `browser/<12hex>/`, so a caller's folder is still named and its
  address is not; and the Archive object names itself in every line by `archive:<12 hex>`, never by its object name
  `archive:<address>`. Every deployable configuration sets `observability.redact_query_string` to `true` — the
  installed wrangler schema has the key at that level — so an open-door socket's `claimed=` never persists in an
  invocation log; the guard holds both (check 14: `redact_query_string` true, and `head_sampling_rate` absent or 1).
  `head_sampling_rate` stays at its default of 1: the live readings count invocations through these logs (the
  Observability query API), and sampling would be privacy by volume at the instrument's cost.
- **D-7 — S4's first-deploy path is a recipe.** A Worker that does not exist holds no secret, so a new Worker's first
  deploy runs `wrangler deploy --config <file>` by hand with NEITHER guard, and the secrets guard from the second
  deploy on; the recipe is written where the guard's cost is recorded (`worker/RUNBOOK.md` §2, `worker/README.md`
  §9.3). No code: wrangler's "Worker not found" is a sentence, not a contract. **Corrected at Task 9's review
  (2026-10-12):** the deploy guard holds THIS repository's Worker — its `name`, its object classes and its bucket, by
  C3's sibling rule and this cycle's F-40 — so a NEW Worker's file (its own name, its own bucket, R-SEC6) is refused by
  the guard on its name and its bucket whatever its door (measured: a renamed open file and a renamed production file
  both refused); the guard judges another Worker's file only once S4 gives it that Worker's identity — S4's change,
  no guard code in S2b (R-6 stands). The first draft's "runs the deploy guard and `wrangler deploy` by hand" could
  not pass its own first step.
- **D-8 — withdrawn at the plan review (2026-10-12).** As drafted, the live replay paced itself at a `PACE_MS` of 150.
  The premise was stale: the replay sends 26 of 2,213 rows at the production door since S1b (17 writes, one
  identity, about 4 s), so the write limit never binds it and no pacing is owed; the arithmetic was wrong on its own
  premise too (about 7.8 minutes, a 1.1–1.3× margin, not 5.5 and 2×). No task implements it; the number stays so
  D-9–D-13 keep theirs.
- **D-9 — B5 takes both of the spec's rules, before identity.** In `answer`, after the route and the upgrade's own
  `Origin` check and before `shellConfig`: an unsafe method (anything but GET and HEAD) with an `Origin` present and
  not `url.origin`, or with `Sec-Fetch-Site` of `cross-site` or `same-site`, is 403 `CROSS_SITE`; `null`, `none` and
  `same-origin` pass, since `Origin: null` is a sandboxed or redirected sender and is not this origin, while a
  user-initiated navigation (`none`) is not a page's. The operator's POST additionally requires `application/json` by
  MIME essence, the layer's own gate, or is 400 in the layer's own words (`Expected request with \`Content-Type:
  application/json\``), so a form can never reach it. The operator's curl playbook and the live replay's `forceAll`
  gain the header. The upgrade's existing check stays where it is; a `fetch` from the site's own pages carries
  `Origin: <own>` and `Sec-Fetch-Site: same-origin` and passes; curl, Node's `fetch` and Playwright's request context
  send neither and pass.
- **D-10 — the guard's new rules, one check each.** Check 13: production's `ratelimits` block equals `RATE_LIMITS`
  (the two bindings, their namespace ids and figures) as values, and each pool file's `ratelimits` names exactly
  `CREATE_LIMIT` and `WRITE_LIMIT`, in any order, with any figures; check 14: production's
  `observability.redact_query_string` is `true` and its `head_sampling_rate` absent or 1; the dev-door rules of D-1
  inside check 3; and F-40's sibling rule: an unlisted `--config` file whose door is `worker` is held to production
  with NO allowed difference. The dev and open files inherit `ratelimits` and `observability` from production by the
  existing sibling comparison, which covers every top-level key but `vars`. Check 13 reads the top level:
  `ratelimits` is not inherited by a named environment (wrangler's schema), and the repository deploys none; a named
  environment would need its own block, and the check would walk it.
- **D-11 — the route's name grammar is the store's, pinned to the emitted fixture.** `routableName` becomes
  `isSafeId` restated in `worker/src/route.ts` exactly as `web/src/tableId.ts` restates it (the same six arms less the
  house's, for the same reason), held both ways to `web/src/wire/id_cases.json` by a node cell; a name the store would
  refuse is the house's, whose `dispatch` answers the layer's own 404 (security S1c, B8), and never a Table object.
  The create's body `id` and the operator's `{id}` go through the same function, as today. A dotted id the store
  admits (`x.js`) still names a Table object before the layer's `DOTTED_ID` refuses the create — one stateless object
  per refused create, inside `CREATE_LIMIT` (recorded, not closed).
- **D-12 — no version, no window, no roster; the suites and the lane's rows.** Spec §5.2 asks the fifteen-gate
  roster "when core or the workspace version moved"; S2b moves neither, so the roster is not owed. The per-edit
  suites and the web lane's rows (`docs/web-lane.md` §2) run at the final head; `cargo test -p yokohama-web` and
  `-p yokohama-worker` carry the Rust cells; `npm run build:module:debug` precedes every pool run after Task 4
  moves the module. The fences: no byte under `crates/yokohama-core`, `-cli`, `-oracle`, `-difftest`, `-bridge`,
  `-wasm` or `gen-data`; `SEMANTICS_REV` 7; the wire corpus unmoved (no new file, no regenerated payload) — pinned
  by `wire_fixtures` running clean without `UPDATE_WIRE_FIXTURES`.
- **D-13 — the inherited residuals this cycle takes.** S2a's four parked sentences (`web/scripts/headers.mjs` ×2,
  `crates/yokohama-web/src/lib.rs` ×2) and the `with_headers` cell's "no dist, no set" message, with their true
  forms as the S2a SDD ledger recorded them; D2's `console.<type>` label pinned by a `fakePage` cell; the fully
  escaped configuration key as committed rows (the S1 stack review's HI-F3 probe, which ran as a temporary copy and
  was deleted); the crafted-name cell (§9.6 correction 9) and the 48-cut WASM cell; and a new pin that the outbox
  shows a 429's sentence — it already read a 429 as a wait (`reasonFor` falls through to `offline`), and the arm adds
  the server's words — because D-3 makes a 429 reachable on `/api/archive` for the first time from the Worker's own
  shell (the zone rule's 1015 answered it before, as a `text/html` page).

**Spec corrections** (to be recorded as §9.7 by Task 9, dated 2026-10-12)

1. **B6's "production runs from `wrangler.gate.jsonc` (dev door off)"** is met by the production file ITSELF
   shipping the dev door off; the gate file is retired and `wrangler.dev.jsonc` opens the door around a live tier
   (D-1; the owner's R-2).
2. **C2's "by claim plus IP under `open`"** reads: by IP for creates and by IP plus claim for writes; by a
   digest of the caller under `worker`; by IP with no caller (D-3).
3. **B12's "a counter in the Archive object on the Worker"** is a live count by an R2 listing before each save, fed
   to the module, with the NAS's figures and arithmetic (the arithmetic and the sentence; the order differs, D-5).
4. **D23's fix** is the shell's digests and key redaction plus `observability.redact_query_string: true`; the
   sampling rate stays 1 (D-6).
5. **B5's JSON requirement changes two recipes**: the operator's curl playbook and the live replay's force gain
   `content-type: application/json` (D-9).
6. **The live replay needs no pacing** (D-8, withdrawn): it sends 26 rows at the production door since S1b; §5.3's
   live acceptance for S2b is read at the wrap's window, there being no S2b window (X2).
7. **The open door's key rule is a named `ClaimPolicy`** in the layer, `NamesOnly` in production and `Diagnostic`
   in the fixtures — the seed rule's shape — and the Worker shell is not a second place for it (D-2); the shell's one
   change is collapsing a claim's space runs, so one name is one Archive object (D-5) — spec §2.2's C1 row names
   `claimedIdentity`.
8. **S4's identity input, recorded on the owner's words (R-3)**: the public Worker at `yokohama.moore.nyc` runs the
   open door behind the Turnstile, controls to be minted later; a second private Worker and the NAS stay behind
   Access; Access is not the public deployment's model.
9. **The first-deploy path of a new Worker is a recipe** (D-7; the owner's R-6), not a guard rule — `wrangler deploy
   --config <file>` by hand with neither guard, the secrets guard from the second deploy on; the deploy guard holds
   this repository's Worker alone (its name and bucket) until S4 gives it the new Worker's identity.
10. **D24 collapses by the grammar**: a plain name's alphabet is ASCII, so once an address is refused under `open`
    every claim is a legal header; the client's name step refuses the rest before any request (D-2).
11. **D3's Worker half is the socket cap** (D-4); the stream cap and the body-read timeout are the NAS's, S2c's.
12. **The fifteen-gate roster is not owed by S2b** (D-12): core and the workspace version do not move.
