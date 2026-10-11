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
