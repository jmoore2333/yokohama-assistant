# Security S2c — the NAS's identity, and S2's wrap: implementation plan (2026-10-13)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> A dispatched implementer reads its own task, §"Global Constraints", §"Names shared across tasks" and the rulings;
> it PROBES every code block of its brief before reporting, and a sentence it finds false is a
> `DONE_WITH_CONCERNS` corrected in THIS PLAN first (the S2a run's rule).

**Goal:** close S2's third cycle — the NAS's stream caps and body deadline (D3), the chat and name character rule
with per-room provenance (D8), the withheld `why` served without a search, the inherited residuals, and B10's host
half as the owner re-ruled it — then wrap S2: the policy enforced with a fixed nonce, `0.6.3`, the stack review,
the three merges, the tag and release, ONE window carrying S2a's live acceptance and its two never-read readings beside
S2b's and S2c's (the fold, R-8 — `v0.6.2` is never tagged), and the hand-off into the auth cycle.

**Architecture:** every change is in the layer (`crates/yokohama-web`), its client (`web/`), the Worker shell's
redaction helper, the deploy documents and one additive extraction in the CLI's `why` module; nothing enters the
dependency tree. The NAS binary stays a trusting origin behind an authenticating edge; the host half makes that
model mechanically true (a private network, cloudflared's own validation, a hardened container). Streams are
counted OPEN at admission by a slot the SSE body drops; the body read has a deadline at its one site; the character
rule is a transcribed Unicode table; the tag is a salted digest computed in the layer so all three hosts agree; the
withheld `why` is built from the record before any search.

**Tech Stack:** Rust 1.97.1 (axum 0.8, tokio, tokio-stream, sha2, serde), TypeScript (Vite, vitest, Playwright),
the Worker shell (wrangler), cloudflared, Portainer on the NAS. No new crate, no new npm package.

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` — §2.2's S2 rows, §9.6's S2c list (line 524),
§9.7's S2b state and the order from here (line 1005); §1's rulings R-SEC4, R-SEC5, R-SEC8, R-SEC10; §5's acceptance
rules; §6's fences. This plan's first section is the brainstorm's rulings of 2026-10-13 and binds beside the spec.
The close appends §9.8 to the spec from this plan's rulings, decisions and corrections, verbatim (Task 10 Step 5).

---

## The owner's rulings for S2c (2026-10-13), and what each binds

The brainstorm of 2026-10-13 settled this cycle's eight forks (the eighth on 2026-10-15) from first principles; the plan transcribes them and
nothing re-opens them. The owner's words are quoted where they were given.

- **R-1 — B10: host half only, the code half ACCEPTED.** cloudflared's `originRequest.access` (`required: true`,
  the team name, the AUD tag) on both ingress rules; a Docker network only `yokohama-web` and `cloudflared-nas`
  join; `CapDrop: ALL`, `no-new-privileges` and a memory limit on the container; the README's one rule rewritten
  as that recipe with a negative probe; nothing enters the dependency tree. The owner: "host half only, code half
  accepted - noting that my surveys tell me most homelab users will not be using access at all, but we're including
  it to maintain parity and in the event they want to host this somewhere more robust they can leverage the
  identity work we're doing already." The reasoning the owner asked for and accepted, recorded for the public
  reader: the code half uniquely closes one path, a request reaching port 8080 without passing cloudflared, which
  on this NAS the private network closes for nothing and which a tunnel-based self-hoster meets only by publishing
  the port by accident on a LAN, where the open door already makes every name a claim; a verifier would add an
  outbound TLS dependency at boot, a JWT parser on every request, two required variables and the first C/asm build
  in the musl binary (`ring`), about 20 crates on a server half of 80; Cloudflare's own guidance requires origin
  validation "unless your application is connected to Access through Cloudflare Tunnel", which is this recipe. The
  owner's words on the tree: "I'm a bit wary of adding so many new dependencies, right now the project is extremely
  lean on dependencies"; "I'm reticent to add a C/asm build into this". **This is the owner's amendment of R-SEC4's
  sentence** ("what protects them is the origin verifying the assertion, and cloudflared's `access.required`"): the
  controls are cloudflared's validation and the private network; the re-open trigger is a NAS hostname served
  without Access, or a real self-host population asking for the binary to defend itself (the zero-dependency
  trusted-proxy peer check is the recorded cheap shape, not taken). The reserved mode name `cf-access-jwt` is
  retired as a reservation (Task 8). Binds Tasks 8 and 10.
- **R-2 — D8: the per-room salted tag, and the character rule.** Watcher and house authors carry a tag the server
  derives from a per-document salt and the caller's key, drawn `name · tag`; table names, watcher names and chat
  text refuse control, format and line-separator characters. The owner's note, transcribed because it bounds what
  the tag promises: "after S2 closes we are looking at adding another identity provider option … but people can
  still choose to not use access or auth0 or anything else in the identity layer at all in their build/deploy stage.
  So for those that have no identity provider, they would still just put in their name and there wouldn't be a
  problem with having that per room salted tag, even though players can just use whatever name they want to claim
  someone else's identity, using a different 'attack' vector, but achieving the same end result - granted that's a
  known and accepted risk for those where their environment has no need for identity verification." Under the open
  door the tag says "the same key as before" and nothing more; the plan's sentences say so (Task 4). Binds Tasks 3,
  4 and 5.
- **R-3 — HI-F2 accepted as documented.** No code: the ledger's B1 row gains the reasoning (the only party is the
  player; the archive keeps the first finished record and answers a different one under the same key with 409;
  the three documents already state the solo exception). Binds Task 8.
- **R-4 — the residuals.** S2c takes: the lobby's silent cap refusal (AD-4) folded into D3's client arm;
  `claimRedacted` rewriting only a span equal to the raw claim, with a configuration-refusal row, and the `g` flag on
  `redactKey`; the `BROWSER_UA` recipes; the §9.5 gap; the lane's §1 and §14 deployed-state sentences; and the FIVE
  surviving comment items the orchestrator read at the head (`crates/yokohama-web/tests/wire_fixtures.rs:2998`'s
  "these two" over a four-row list, false; `worker/src/index.ts:114`'s "in this order" list, to verify;
  `web/src/wire/contract.ts:447`'s `IdCase` undocumented; `web/scripts/headers.mjs:177`'s parse refusal naming
  neither the new reasons nor the name bound; `worker/src/index.ts:84`'s 143-character line). *Read at the code by
  the plan review (2026-10-15): `headers.mjs:177`'s refusal already names the bound and omits only 'non-empty';
  `index.ts:114`'s list is true as written and needs no edit; `index.ts:84` is 143 characters.* It reads at the window:
  the `limiter-failed` line's `String(e)`. It leaves recorded where they are: the open-door write key and the
  seat-key rule (S4's, D-2/D-3 of S2b), the lobby's total arm, the copied pool helpers, the two one-row loops, and the
  test and design notes of both SDD ledgers. **And a standing step for the launch:** before the tag, the after review
  re-reads every deferred list of S2a, S2b and S2c against the head and takes any surviving sentence that is false
  or stale, leaving only test and design notes behind — the owner: "others may not have access to all the spec/plan
  docs that we have when we move this to public so it would be important to have the comments in code and docs
  that are 'final' to be reflective of their actual state." Binds Tasks 5, 7 and 10.
- **R-5 — AD-6 stands: a Rate Limiting binding that throws is logged and the request proceeds.** The limiter is a
  cost bound, not an access control; the window reads the count of `limiter-failed` lines, expected zero. Binds Task
  8 (the ledger's C2 row) and Task 10 (the window).
- **R-6 — the wrap ships `enforce` with a FIXED NONCE; "accept the refusal" is the recorded fallback.** Cloudflare
  reads a nonce from the response's CSP header and sets it on the scripts it injects (its JavaScript detections
  documentation), so a static header with a committed nonce keeps Bot Fight Mode's detection script running and the
  console clean; the policy's inline-script rule becomes nominal while its source lists and `frame-ancestors` stay
  strict, which the constant's comment says. If the first window's Report-Only reading shows any line but the known
  detection script, STOP for the owner before Task 9; if the second window finds the injected script without the
  nonce, the fallback is one constant and the harness's forgiveness at `error` (Task 9's recipe). `'unsafe-inline'`
  is dominated by the nonce; `Cache-Control: no-transform` is out because it also stops the analytics beacon the
  owner keeps on. Binds Tasks 9 and 10. *Re-pointed by R-8 (2026-10-15): there is no first window and no
  Report-Only reading; the STOP and the fallback are read at the one window's NAS canary, under the enforced policy
  (D-15), before the Worker deploys.*
- **R-7 — NO pre-flight this cycle; the body-read deadline stays.** The owner asked whether the pre-flight earns its
  cost when the task agents run that work anyway: this cycle has no figure to measure (the caps mirror the Worker's,
  the deadline and the nonce are constants, the Cf count is pinned by the cell that transcribes it), and the
  one-token transcription slips it caught in S2a and S2b are what the implementer's own probe catches one task later
  at the same cost. X4 was S1c's shape for a cycle with figures to measure, not a standing step. The plan review's
  lenses still EXECUTE the plan's blocks (R-1's method, where the real finds came from), and the implementers probe.
  The deadline: the binary applies no read deadline of any kind today (`axum::serve` sets no timer, so hyper's
  header timeout is off, and nothing bounds a body's arrival); a 10-second deadline at the one read site turns an
  unbounded hold into a rate, which the edge and the caps already bound — "a small bound against a small risk, of
  the kind a binary meant for strangers' LANs should carry". Binds Tasks 2 and 10.
- **R-8 — the two windows are folded into ONE (2026-10-15), re-ruling X2 of 2026-10-11.** The owner opened the
  session with "we can update this plan as you review so that we run the 0.6.3 in this cycle", accepting the
  previous session's close recommendation, transcribed because it fixes the window's order: "At the 0.6.3 window,
  deploy the NAS first and read the console on a fresh profile at every level before the Worker is touched. The NAS
  sits on the same zone, so it meets the same injections, and only you use it, so an enforced policy that blocks
  something breaks your own session and nobody else's. A clean NAS read is the wild reading; the Worker's deploy
  proceeds on it. A line that does appear is the pre-registered fallback, one constant, before the Worker ships."
  What it changes: `v0.6.2` is never tagged and never deployed (the hosts run 0.6.1 until this window; the next
  release is 0.6.3); the S2a plan's Task 9 Steps 8–10 are SUPERSEDED, their readings taken inside this window — the
  header reads, the live acceptance (c), the two-identity block (d) with its two never-read readings (D1, B1's
  withheld `why`), the deploy records; the Report-Only reading is replaced by the NAS CANARY under the enforced
  policy (D-15), which gates the Worker's deploy; Task 9 has no window precondition. What it keeps: X2's "nominal
  amount of work" was the flag, and the flag is Task 9's as before; the release before either deploy; the roster at
  the bump; the regeneration for the version. Recorded as spec correction 12. Binds Tasks 9 and 10.

**The design the owner approved (2026-10-13), in one line each:** the branch `security-s2c` on `security-s2b`'s head
`2ea0edf3`, a PR against it, one merge to master at the wrap (X1); ten tasks, Opus reviewers throughout, Sonnet
implementers for the two documents tasks (X3); D3's figures 8 per caller per room, 64 per table, the house 8 per
caller and 256 total, OPEN streams only, the (n+1)-th a 429 before any frame; the client ends a signal on a 429 with
the server's sentence and the lobby draws an ended signal; the body deadline 10 s, 408; the character rule Cc, Cf, Zl,
Zp with Cf transcribed from Unicode 17.0.0 (the toolchain's own tables, `char::UNICODE_VERSION`) and a count pin,
multi-person emoji sequences accepted as the loss; the tag four hex of SHA-256 over a per-document salt and the key;
the withheld `why` built from the record before any search, the withheld-mismatch sentence retired; the wrap's
mechanics as Task 9 and Task 10 spell them. **Amended 2026-10-15 (R-8):** there is no first window — `v0.6.2` is
never tagged — and Task 9 waits for nothing; the one window at Task 10 Step 10 opens with the NAS as the canary.

## The decisions this plan takes (the orchestrator's; D-1–D-15)

- **D-1 — the stream caps are a slot the SSE body holds.** `streams::Streams` counts per room and per
  (room, caller), OPEN streams only: `admit` hands back a `Slot` whose `Drop` decrements both counts and removes a
  zero entry (B8's lesson: the maps never hold more than the open streams), and the stream adapter `Admitted<S>`
  carries the slot inside the response body, so hyper's drop of an abandoned connection frees the slot and nothing
  else can. The verdicts in the Worker's order (`sockets.unit.test.ts`): the room's total first, then the caller's.
  The refusal is `ApiError::too_many_requests` (429) with one of four constant sentences, answered BEFORE any frame,
  after the admission read (`api::events_at`) has answered its own 403 or 404, so a cap never masks a refusal about
  the table. No `Retry-After`: the client ends the signal rather than waiting (D-2).
- **D-2 — a 429 on a stream ENDS the client's signal.** `web/src/signal.ts`'s stream OPEN (line 479) reads every status but 404, 403
  and the edge's expired session as "cannot reach the server" and climbs the backoff ladder; a 429 there would be a
  silent retry storm against the cap, so that arm gains 429 with `reason: "refused"` and the server's sentence. The
  woken RE-READ (line 316) does not climb the ladder — a failure there keeps the stream and retries on the next frame —
  and a 429 there is never the cap's (the NAS caps streams, the Worker limits unsafe methods only), so it stays as it
  is: a transient edge 429 on a listing read must not end live updates (plan review L2-F4). `Lobby.tsx`'s `usePolled` passes an `onEnd` that draws the sentence and stops — which is S2b's AD-4 on
  both hosts at once, since the Worker's lobby socket ends on an `end` frame through the same `onEnd`.
- **D-3 — the body deadline lives at the ONE read site, not in a middleware.** Every `/api` request with a body
  reaches `adapter` (`lib.rs` line 1049, `to_bytes(body, 2 << 20)`): the catch-all `/api/{*rest}` and `/api/games`
  route there, and the three separate routes (`events`, `stream`, `house_stream`) and `enter` are GETs. So
  `tokio::time::timeout(state.body_deadline, to_bytes(..))` at that site covers every body, changes no other
  answer (a 2 MiB overrun still answers the adapter's own 400), and answers `ApiError::request_timeout` (408) with
  `BODY_TOO_SLOW` when the deadline passes. The design message of 2026-10-13 said "one middleware"; this is the
  exact form, and the brainstorm's reasoning (every read site under one clock; GETs, streams and the handler's
  compute never under it) holds of it.
- **D-4 — the character rule is Cc, Cf, Zl and Zp, with Cf transcribed in-source.** `text::FORMAT_RANGES` is
  Unicode 17.0.0's General_Category Cf, 21 ranges and 170 code points, transcribed from the UCD's
  `extracted/DerivedGeneralCategory.txt` (dated 2025-07-24) — the toolchain's own Unicode version (`rustc 1.97.1`
  reports `char::UNICODE_VERSION` = 17.0.0) — with a cell pinning the count and the ranges' order; `char::is_control`
  is Cc (65 code points); U+2028 and U+2029 are Zl and Zp. No crate, since the layer compiles to wasm and the fence
  covers it. U+200D (the joiner) is Cf, so a multi-person emoji sequence is refused in chat while a single emoji
  (U+1F642), a flag (two regional indicators) and a variation selector (U+FE0F, category Mn) pass — the loss the
  owner accepted; the non-joiner U+200C (Persian, Indic spelling) is the same category and refused too — for the
  owner at the close (O-2). The rule applies through `tables::check_name` (table names, display names, watcher names on WRITE)
  and `chat::check_text`; a STORED document reads under the rule it was written under — `tables::check_stored_name`,
  `is_control` alone, at `tables::check_shape`'s two name sites, `chat::check_shape`'s author name and
  `chat::normalized` — so a document this server wrote before the cycle (a pasted U+200F, a family emoji's joiners)
  still reads and never answers 500; the new rule meets it the next time the name is written (plan review L1-F1); the client sends text unvalidated and draws the 400's sentence.
- **D-5 — the tag: a per-document salt the seed policy fixes for fixtures.** `ChatDoc.salt: Option<String>`
  (serde default), filled on the document's first APPEND after this cycle from `Games::chat_salt()`: 16 bytes of OS
  entropy as 32 hex under `SeedPolicy::SoloOnly` (every production constructor), and the constant `FIXED_CHAT_SALT`
  under `SeedPolicy::Diagnostic` (the fixtures, the corpus walk, the pool and `init_diagnostic()` on both wasm
  hosts), so no constructor on any host changes and the wire corpus and the pool stay byte-stable. The tag is the
  first four hex of SHA-256 over `salt`, a newline and the key; a document without a salt (written before this cycle,
  not yet appended to) carries `tag: null` and the client draws no tag. Watcher authors carry it; seat and bot
  authors do not (a seat is server-bound). Four hex is readable; under the open door a name can be searched for a
  colliding tag, which the owner's note accepts.
- **D-6 — ONE additive extraction in the CLI's `why` module.** `account_parts` locates the event (the index `arg`
  names or the last explainable bot move, by replay), reads the mover and its contestant, THEN decides. The layer
  must read the requester's visibility between those halves, and `last_explainable_bot_move` is private. The first
  half becomes `pub fn locate(..) -> Result<Located, Refusal>` and the second `pub fn account_from(Located, ..)`;
  `account_parts` is `account_from(locate(..)?, ..)` and the CLI's own `why` output is byte-identical (pinned by
  the CLI's existing `why` cells). This is the one byte under `crates/yokohama-cli` this cycle, recorded as a
  correction of spec §6's CLI sentence; `cargo test -p yokohama-cli` runs at the task and the roster is owed anyway
  by the bump (D-10). The alternative, a second "last explainable bot move" rule in the layer, is a second derivation
  of a CLI fact, which the house forbids.
- **D-7 — the withheld-mismatch sentence is retired.** With the search run only at `Visibility::Public`, a mismatch
  can no longer be found while withheld, so `MISMATCH_WITHHELD` and `Why::refused_at` are deleted with their cells,
  and the structural catcher takes their place: a record whose bot move was tampered answers the WITHHELD payload at
  a withheld visibility (the old flow answered the mismatch refusal there) and the Mismatch refusal verbatim at a
  public one. No timing is measured.
- **D-8 — HI-F2 is a ledger row (R-3).** The B1 row's note becomes `accepted` text with the reasoning; no code.
- **D-9 — the nonce is a committed constant named for what it is.** `CLOUDFLARE_INJECTION_NONCE` in
  `web/scripts/headers.mjs`, 22 base64 characters drawn once by the implementer (alphanumeric, AD-13), rides `script-src` as
  `'nonce-<value>'`; `CSP_MODE` flips to `enforce`; the e2e build's rewrite is an identity on an enforced set and is
  kept for a report-only build; the harness's Report-Only forgiveness (`isReportOnlyDetectionLine`,
  `reportOnlyForgiven`) is removed so any CSP line is breakage on every tier. The fallback (R-6, timed by
  D-15): restore the forgiveness from this commit's parent in the ENFORCED line's words, read at `error`, and drop
  the nonce token at the next release — never under a published tag.
- **D-10 — the bump to `0.6.3` is Task 9's last step; the roster runs at the code head after it.** The lane's §7
  recipe (the five steps); the fixture regeneration for `server_version` and the backup envelope's `app_version` is
  this cycle's SECOND regeneration (Task 4's is the first), each diff read — the owner's corpus note (2026-10-09)
  bounds regenerations that change the wire's SHAPE; a version line is mechanical. Spec §5.2 asks the fifteen gates
  when the workspace version moves (AD-3's precedent: S2a ran them after its bump).
- **D-11 — no pre-flight (R-7).** Task 10 has no pre-flight step; the plan review's lenses execute the blocks; every
  implementer probes its brief.
- **D-12 — the host half's catcher is the window.** `cloudflared tunnel ingress validate` over the new file; the
  container's `HostConfig` read back from Portainer; the negative probe from the terminal container (a connection to
  `yokohama-web:8080` fails); the hostnames serving as the owner. cloudflared's validation is CONFIG-READ, not
  exercised: exercising it needs a request that reaches the tunnel without passing Access, which the design forbids.
  The deploy record says so.
- **D-13 — `BROWSER_UA` is added to RECIPES, never to dated records.** The operating sections of `worker/RUNBOOK.md`
  hold seven recipe `curl` lines without it — §3's operator playbook (497–499), §4's credential read (561), §5's health
  read (587) and §5's upgrade probes (792–793); §2 has none — and `BROWSER_UA` is already defined once, at §7's burst
  bullet (line 966); `deploy/RUNBOOK.md` §3 already sets its own `UA` and passes `-A "$UA"` on every recipe, and the
  lane holds no recipe `curl` at all (plan review L2-R7), so neither moves; a `curl` inside a numbered
  deployment record stays as it was run.
- **D-14 — the close re-reads the deferred lists (R-4).** Task 10 Step 4's after review walks the three SDD ledgers'
  deferred and parked items against the head and takes every surviving false or stale sentence before the tag.
- **D-15 — the canary order at the one window (R-8).** The NAS deploys first, with the enforced policy, and a FRESH
  profile's first visit to its hostname is read at EVERY console level before the Worker is touched — the NAS meets
  the zone's injections exactly as the Worker does, and only the owner uses it. Three readings, pre-registered:
  **(i)** no CSP line, and the injected detection script carries the nonce (`curl -A "$BROWSER_UA" -s
  https://yokohama-nas.moore.nyc/ | grep -o 'nonce="[^"]*"'` over WARP equals the constant — `deploy/RUNBOOK.md`
  §3.15 measured the edge's rewrite on every `curl` of `/` through this tunnel — or the browser's VIEW-SOURCE, never the
  Elements panel, since a browser hides a `nonce` attribute's value from the DOM once a policy is enforced)
  — the wild reading is CLEAN and the Worker deploys; **(ii)** exactly one CSP line, and it names Bot Fight Mode's
  inline detection script (the edge did not copy the nonce) — R-6's fallback, "accept the refusal": the token is
  INERT (an enforced policy blocks that script with the token or without), so the Worker deploys the SAME build and
  the window continues; the line's text is recorded, the live Playwright tier's one run reports it as the one
  finding, and the fallback commit lands after the window on master by the owner's word — the harness's forgiveness
  restored at `error` from the line as read, the nonce constant's comment rewritten to say the edge did not honour
  it, the token itself dropped at the NEXT release (dropping it under a published tag would change the shipped
  header); **(iii)** ANY other line — an injection nobody listed, blocked — is the STOP for the owner before the
  Worker deploys: the NAS is the owner's alone, its rollback is `yokohama-web:0.6.1-s1` with the pre-S2c
  `HostConfig`, and the enforcement shape returns to the owner with the line in hand. A nonce present and a CSP line
  printed matches neither (i) nor (ii) and is (iii). The Worker's own console read
  after its deploy takes the same three readings; a Worker reading that differs from the NAS's is itself a finding
  (one zone, one header set).

## Spec corrections (to be recorded as §9.8 by Task 10, dated 2026-10-13 and 2026-10-15)

1. **R-SEC4 is amended by the owner (R-1):** B10's code half is `accepted`; the controls behind Access are
   cloudflared's `access.required` and a network only the tunnel joins; `cf-access-jwt` is not built, and spec §2.2's
   B10 (code half) row reads as the host half's recipe plus the README's rule.
2. **D3's NAS half** is `8` per caller per room, `64` per table, the house `8` per caller and `256` total, OPEN
   streams only, a 429 before any frame; the client ends a signal on it (D-1, D-2).
3. **"The body read timeout"** is a 10-second deadline at the adapter's one read site, answering 408 (D-3).
4. **D8's fix** is the Cc/Cf/Zl/Zp rule with Cf transcribed from Unicode 17.0.0, and a per-document salted tag on
   watcher and house authors (D-4, D-5); the report's "label house-chat names as self-chosen in the client" is met by
   the tag.
5. **The explain surface's two rules:** the withheld `why` runs no search (the visibility is read first; the payload
   is built from the record; `MISMATCH_WITHHELD` retired, D-6, D-7); HI-F2 is accepted (R-3).
6. **§6's CLI sentence** ("the CLI is touched only at `suggest::stream_seed`") gains S2c's one extraction in
   `crates/yokohama-cli/src/why.rs`, behaviour-preserving, nowhere the bot decides (D-6).
7. **X4's pre-flight is not run this cycle** (R-7); the plan review's executing lenses and the implementers' probes
   are the pre-run checks.
8. **The enforcement shape** is a fixed nonce (R-6, D-9); "accept the refusal" is the recorded fallback; `0.6.3`.
9. **§9.5's gap** — S1c's run's-rulings paragraph never appended — is filled by this close from the S1c plan's
   §"Review record" (Task 10 Step 5).
10. **AD-6 is confirmed by the owner** (R-5).
11. **The corpus regenerates twice this cycle**, Task 4's for the tag and Task 9's for the version, each diff read
    (D-10).
12. **X2 is re-ruled by the owner (R-8, 2026-10-15): ONE window and one version.** §9.6's X2 ("two windows, two
    versions… S2a ships `0.6.2` with the policy in Report-Only; the wrap ships `0.6.3` enforcing"), its correction 1
    ("Two windows and two versions"), its correction 11 ("`frame-ancestors 'none'` rides the Report-Only policy in
    0.6.2") and §9.7's "the first window (`v0.6.2`, from the tag's tree)" read with this: `v0.6.2` was built (S2a's
    Task 8) and never tagged or deployed; the hosts go from 0.6.1 to 0.6.3 in one window; the Report-Only reading is
    replaced by the NAS canary under the enforced policy (D-15); S2a's live acceptance (§5.3 for S2a) and the
    two-identity block's readings are taken in that window; B4's frame fence is `X-Frame-Options: DENY` alone until
    0.6.3, and both headers from it. The same fold reads §2.2's "the CSP" row ("shipped as
    `Content-Security-Policy-Report-Only` for one release, then enforced … a release's console read") as one release,
    0.6.3, enforced at its first deploy, and §9.7's "the second window" (the R-SEC6 clarification's "S2's second window
    included" and the closing paragraph's "the second window") as the one window.
13. **§9.6's gap** — S2a's run's-rulings paragraph, which the S2a plan said was the close's (its Task 9 Step 6),
    never appended — is filled by this close from the S2a plan's §"Review record" (the run and the after review),
    as correction 9 fills §9.5's (Task 10 Step 5; the plan review's AD-12, 2026-10-15).

## Global Constraints

- **The fences (spec §6, as corrected 6 above):** no byte under `crates/yokohama-core`, `crates/yokohama-oracle`,
  `crates/yokohama-difftest`, `crates/yokohama-bridge`, `crates/yokohama-wasm`, `crates/yokohama-worker` or
  `crates/gen-data`; `crates/yokohama-cli` is touched in ONE file, `src/why.rs`, by Task 6's extraction and nowhere
  else; `SEMANTICS_REV` stays **7**; no bot behaviour moves; the wire corpus (`web/src/wire/*.json`,
  `web/src/wire-script/*.json`) regenerates EXACTLY twice (Task 4 for the tag, Task 9 for the version), each diff read
  and named in the task's report, and `cargo test -p yokohama-web --test wire_fixtures` runs clean WITHOUT
  `UPDATE_WIRE_FIXTURES` at every other task's head; `Cargo.lock` moves only on the nine members' version lines at
  Task 9 (the close pins `git diff 2ea0edf3 --stat -- Cargo.lock` to those lines).
- **The version:** `0.6.2` (built by S2a, never tagged or deployed — R-8) until Task 9's last step, `0.6.3` from it; the fifteen-gate roster runs ONCE, at Task 9's
  bump commit (D-10); the window is the orchestrator's with the owner (Task 10).
- **Every figure at its literal, with its defence in a concise in-code comment** (the owner's S1b rule): `8`/`64`/`8`/
  `256` for the streams; `Duration::from_secs(10)` for the body deadline; `408` and `429`; `21`/`170` for the Cf
  table; `4` hex for the tag; the nonce's `22` characters; `536870912` bytes (512 MiB) for the container's memory.
- **Every caller value a sentence repeats is bounded** (the echo rule): no sentence this plan adds repeats a caller
  value — the character rule's refusals name the RULE, never the offending character.
- **The invisible-code-point rule** (CLAUDE.md, M6.5c2): every invisible non-ASCII code point in code or a test is a
  `\u{…}` escape; `FORMAT_RANGES` is integers; a grep for a raw U+200B, U+202E, U+2028 or U+FEFF over the CODE files
  (`*.rs *.ts *.tsx *.mjs *.js` under `crates web/src web/scripts web/e2e worker/src worker/scripts`) reads nothing after
  Task 3 — the corpus's `id_cases.json`, `identity_cases.json` and `seed_cases.json` carry raw ones by design, as two
  dated plans do.
- **The suites after every task:** `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`;
  `cargo test -p yokohama-web --no-default-features --features server --lib identity::`;
  `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`;
  `cargo clippy --workspace --all-targets -- -D warnings`; `cd web && npm run typecheck && npx vitest run && npm run
  test:wasm`; `cd worker && npm run typecheck && npm run test:scripts && npm test` — `npm run build:module:debug`
  FIRST whenever the layer moved (Tasks 1–4 and 6 move it; the build is idempotent). Task 6 adds `cargo test -p
  yokohama-cli`. Task 9 adds `cd web && npm run build` and `npm run e2e`. Every count recorded against the board's
  "Gates at HEAD".
- **Mutants run three times, by design, and never a fourth:** the implementer's own probe of its brief, the task
  reviewer's battery on an export, and the after review's battery over every named mutant. No pre-flight (R-7).
- **Reviewers run their mutants on an EXPORT** (`git archive`), never in the implementer's tree; one writer in the
  tree at a time (a documents implementer waits for a code implementer's DONE); every agent writes its full report
  to a file under `docs/temp/security-review-2026-10-08/s2c-sdd/` and replies with the path and three lines; no
  agent runs `rm -rf`, `rm -r` or a bulk `rm -f`, arms a Monitor or waits on an event; **no agent makes any call to
  the Cloudflare account, read-only included**; no agent messages another agent.
- **Every ruling here is transcribed, never paraphrased**, and a sentence an implementer finds false is a
  `DONE_WITH_CONCERNS`, corrected in THIS PLAN first and then transcribed.
- **There is no first window (R-8).** Task 9 has no window precondition; the wild reading is the NAS canary under the
  enforced policy at Task 10 Step 10(a) (D-15), and it gates the Worker's deploy: clean, or the detection-script line
  (the fallback, recorded, the window continues), or anything else (STOP for the owner before the Worker deploys).

## Review Focus

Five inputs the spec implies and no task's tests exercised as first drafted — each now has its cell in the task
named:

1. **A seat holder's OWN stream refused by the cap** ends the signal as `refused` with the server's sentence — never a
   silent retry — and lands the player in the lobby with that sentence on its strip, as a 403 does and as the Worker's
   4429 close does (every table consumer of an `End` leaves: `App.tsx:1467`, `OpenTable.tsx:164`, `Dealing.tsx:259`;
   the player's other tabs keep the table). Task 5's signal cell asserts the reason; the App cell "lands on the lobby
   with the cap's sentence when the table's own stream is refused 429" asserts the landing (plan review L2-F1).
2. **A body whose `Content-Length` promises more than the client sends** must answer 408 within the deadline and
   release the connection, never hold it — Task 2's stalled-channel cell IS this input.
3. **A chat line at exactly the character cap that carries a format character** is refused by the CHARACTER rule's
   sentence (it is within the cap), and one character over the cap by the LENGTH sentence — Task 3's order cell pins
   that the refusal names the rule a person can act on.
4. **A chat document written before this cycle**, read on every host, carries `tag: null` for its watcher authors
   and nothing panics, and a name it stored under the old rule (a zero-width space, a joiner) still reads — the read
   side keeps the old rule (Task 3, plan review L1-F1); its first new message gives it a salt and later reads carry tags — Task 4's `view_for`-with-
   `None` cell and the append cell.
5. **A visit where Cloudflare injects no detection script** (every load but a first) under the enforced policy
   prints no console line, and any CSP line that does appear is breakage, never forgiveness — Task 9's harness change,
   the NAS canary (D-15) and the live tier's reading at the window.

## Execution shape

- **Branch:** `security-s2c`, cut from `security-s2b`'s head `2ea0edf3`; this plan and the board's Track 4 as the
  first commit; the SDD ledger at `docs/temp/security-review-2026-10-08/s2c-sdd/progress.md`; the PR against
  `security-s2b`; the merges at the wrap (Task 10 Step 8).
- **Tiers (X3):** Opus implementers for Tasks 1–6 and 9; Sonnet implementers for Tasks 7 and 8; an Opus reviewer for
  every task; the after review's battery one Opus agent; the stack review four Sonnet lenses and the orchestrator's
  verification.
- **Order:** 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9, Task 10 the orchestrator's throughout. Tasks 7 and 8 may run beside
  the review of Task 5 or 6 (never beside each other's or a code task's implementation: one writer in the tree).
- **Each task:** the failing cells first, the code, the suites, one commit per step group; the reviewer's named
  mutants red at the named cell; a fix round a NEW commit; the plan corrected at the source before a transcription.

## Names shared across tasks

- `crates/yokohama-web/src/streams.rs` (server-only): `pub struct Streams`, `pub enum Room { Table(String), House }`,
  `pub struct Slot`, `Streams::admit(self: &Arc<Self>, room: Room, caller: &identity::User) -> Result<Slot, &'static
  str>`, `Streams::open(&self, room: &Room) -> usize`; the constants `STREAMS_PER_CALLER` (8), `STREAMS_PER_TABLE`
  (64), `HOUSE_STREAMS_PER_CALLER` (8), `HOUSE_STREAMS_TOTAL` (256) and the four sentences `TOO_MANY_STREAMS_CALLER`,
  `TOO_MANY_STREAMS_TABLE`, `TOO_MANY_HOUSE_CALLER`, `TOO_MANY_HOUSE_TOTAL`.
- `crates/yokohama-web/src/lib.rs`: `ApiError::too_many_requests(message: impl Into<String>) -> Self` (429),
  `ApiError::request_timeout(message: impl Into<String>) -> Self` (408); `AppState.streams: Arc<streams::Streams>`,
  `AppState.body_deadline: std::time::Duration`, `AppState::with_body_deadline(self, d) -> Self`; the constants
  `BODY_READ_DEADLINE` (10 s) and `BODY_TOO_SLOW`; the stream adapter `Admitted<S>`.
- `crates/yokohama-web/src/text.rs` (every host): `pub const FORMAT_RANGES: [(u32, u32); 21]`, `pub const
  FORMAT_COUNT: usize = 170`, `pub fn is_format(c: char) -> bool`, `pub fn is_line_separator(c: char) -> bool`,
  `pub fn invisible(s: &str) -> Option<char>`, `pub const INVISIBLE_RULE: &str`.
- `crates/yokohama-web/src/chat.rs`: `ChatDoc.salt: Option<String>`; `pub const TAG_HEX: usize = 4`; `pub fn
  tag_of(salt: &str, key: &str) -> String`; `AuthorView::Watcher { name, you, tag: Option<String> }`;
  `Message::view_for(&self, reader: &User, salt: Option<&str>) -> MessageView`.
- `crates/yokohama-web/src/games.rs`: `pub const FIXED_CHAT_SALT: &str`; `Games::chat_salt(&self) -> String`.
- `crates/yokohama-web/src/tables.rs`: `Games::chat_append_salted(&self, id: &str, author: Author, text: &str) ->
  Result<(Message, Option<String>), GameError>`, `Games::house_append_salted(&self, author: Author, text: &str) ->
  Result<(Message, Option<String>), GameError>`; `chat_append`/`house_append` unchanged in signature.
- `crates/yokohama-cli/src/why.rs`: `pub struct Located { pub index: usize, pub seat: PlayerId, pub state: GameState,
  pub recorded: Move, pub contestant: Contestant, pub recorded_version: Option<u32>, pub current_version: u32 }`;
  `pub fn locate(mode: TrackingMode, record: &GameRecord, rules: &Rules, arg: Option<usize>) -> Result<Located,
  Refusal>`; `pub fn account_from(located: Located, bots: &BotSeats, rules: &Rules) -> Result<WhyAccount, Refusal>`;
  `account_parts` unchanged in signature and output.
- `crates/yokohama-web/src/explain.rs`: `Why::withheld_from(l: &yokohama_cli::why::Located, rules: &Rules) -> Why`;
  `Provenance::located(l: &Located) -> Provenance`; `Why::refused_at` and `MISMATCH_WITHHELD` REMOVED.
- `web/src/types.ts`: `ChatAuthor`'s watcher arm gains `tag: string | null`.
- `web/scripts/headers.mjs`: `export const CLOUDFLARE_INJECTION_NONCE` (22 base64 characters); `CSP_MODE = "enforce"`.
- `worker/src/archive.ts`: `claimRedacted(sentence: string, claim: string, digest: string): string`;
  `worker/src/redact.ts`: `SEGMENT` with the `g` flag.
- The deploy: `deploy/cloudflared-config.yml`'s `originRequest.access` blocks (`teamName: bdvc`; the AUD
  `5fa7a989f4f4970771769d68ff273497c125f0374253c0ccf817952dc9c3e8c7`, the one application's, spec §2.3 B11);
  the container's `HostConfig` additions `"CapDrop": ["ALL"]`, `"SecurityOpt": ["no-new-privileges:true"]`,
  `"Memory": 536870912`; the image tag `yokohama-web:0.6.3-s2`.

## File structure

- **Create:** `crates/yokohama-web/src/streams.rs` — the caps and the slot, server-only (one responsibility: who may
  open a stream, and the count of the open ones). `crates/yokohama-web/src/text.rs` — the character rule and its
  table, every host (one responsibility: which characters a person's text may carry).
- **Modify, the layer:** `lib.rs` (the two error constructors; two `AppState` fields and a builder; the two stream
  routes and `Admitted`; the adapter's deadline; cells), `chat.rs` (the salt, the tag, `view_for`, `check_text`),
  `tables.rs` (`check_name`; the two append sites fill the salt; the read site hands the salt to `view_for`),
  `games.rs` (`chat_salt`), `api.rs` (`why`'s flow), `explain.rs` (`withheld_from`; the removals; cells),
  `identity.rs` (the module doc's reserved-mode sentence).
- **Modify, the CLI:** `crates/yokohama-cli/src/why.rs` (the extraction; one cell).
- **Modify, the client:** `web/src/signal.ts`, `web/src/Lobby.tsx`, `web/src/Chat.tsx`, `web/src/types.ts`, their
  cells, `web/src/wire/*.json` (regenerated twice), `web/src/wire/contract.ts` (a doc comment).
- **Modify, the Worker shell:** `worker/src/archive.ts`, `worker/src/redact.ts`, their cells, `worker/src/index.ts`
  (two comments), `worker/scripts/` nothing.
- **Modify, the build and the e2e:** `web/scripts/headers.mjs`, `web/scripts/headers.node.test.ts`,
  `web/scripts/e2e-build.mjs`, `web/e2e/harness.ts` and the specs that print the retired counter.
- **Modify, the documents:** `deploy/cloudflared-config.yml`, `deploy/README.md`, `deploy/RUNBOOK.md`,
  `deploy/Dockerfile.runtime`, `worker/RUNBOOK.md`, `docs/web-lane.md`, `docs/security-ledger.md`,
  `crates/yokohama-web/tests/wire_fixtures.rs` (one comment), `Cargo.toml`, `web/package.json`, `worker/package.json`
  and their locks; at the close `CLAUDE.md`, `guidingpath.md`, `docs/handover.md`, the spec's §9.8.

## Task order and why

1. **D3's stream caps** first: a server-only module with no dependency on the rest, and the figure every later
   reading quotes.
2. **The body deadline**: one site, one field, beside Task 1's state changes.
3. **The character rule** before the tag: the tag's cells post watcher names and chat text that must already pass
   the rule, so the rule's sentences are settled first.
4. **The tag**: the wire moves here, the corpus regenerates (the first of two), the pool replays it.
5. **The client**: the 429 arm, the lobby's end, the tag's label — after the wire it draws exists.
6. **The withheld `why`**: the CLI's extraction and the layer's flow, independent of 1–5; placed here so the two
   documents tasks that follow can run beside its review.
7. **The residuals**: the Worker's redaction, the recipes, the five comment survivors — documents and two small TS
   edits, a Sonnet implementer.
8. **The host half's documents and the ledger**: the cloudflared block, the README's recipe, the runbook's
   HostConfig, the reserved-mode sentence, the rows — a Sonnet implementer; the Portainer changes themselves are the
   window's.
9. **The wrap's code**: the nonce and `enforce`, the harness, the lane, then the bump as the last code commit, so the
   roster reads the whole cycle once.
10. **The orchestrator's task**: the plan review, the run, the after review and its re-read, the close, the PR, the
    stack review, the merges, the tag and release, the one window, the hand-off.

---

### Task 1: D3's NAS half — the stream caps, OPEN streams only, a 429 before any frame — Opus (decision D-1; Review Focus 1's server side)

**Files:**
- Create: `crates/yokohama-web/src/streams.rs`
- Modify: `crates/yokohama-web/src/lib.rs` — the module list (`#[cfg(feature = "server")]` then `pub mod streams;` INSIDE the
  existing server region — after `pub mod assets;`, before the `// ---- end server ----` marker; beside `signal`, past
  the marker, it opens a region that `platform_calls_stay_behind_the_server_feature` reads as eight ungated items), `ApiError`'s constructors (beside `bad_request`, wherever that `impl ApiError` block sits — `grep -n
  "pub fn bad_request" crates/yokohama-web/src`), `AppState` (a field, every constructor that builds the struct
  literally — `grep -n "signals" crates/yokohama-web/src/lib.rs` finds the field at 435 and the ONE constructor, which uses the
  shorthand `signals,`; the `..self` builders inherit it), `stream` (line 928 today) and
  `house_stream` (line 999), a new adapter `Admitted<S>` beside `UntilLast` (line 841).
- Test: the cells below in `lib.rs`'s test module and `streams.rs`'s; and `lib.rs`'s
  `every_api_error_message_is_capped_at_construction`, whose `built` vec gains the new constructor (Step 4).

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `streams::{Streams, Room, Slot}` and the eight constants (§"Names shared across tasks");
  `ApiError::too_many_requests`; `AppState.streams`. Task 5's client arm reads the 429 and its sentence.

- [ ] **Step 1: Write the failing cells.** In `streams.rs` (the module does not exist: the red is a compile error):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn user(s: &str) -> crate::identity::User {
        crate::identity::User::parse(s).expect("a key")
    }

    /// **The verdicts, the room's total first** (the Worker's order, `sockets.unit.test.ts`): one caller's ninth
    /// stream at a table is refused with the caller's sentence; the sixty-fifth of a table whatever its caller with
    /// the table's; the house's figures are its own; and a dropped slot frees the count — OPEN streams alone are
    /// counted, which is what D-4 of S2b measured the Worker needed. The maps hold nothing for a room with no open
    /// stream (B8's lesson).
    #[test]
    fn the_caps_refuse_the_ninth_of_a_caller_and_the_sixty_fifth_of_a_table_and_a_dropped_slot_frees_the_count() {
        let streams = Arc::new(Streams::default());
        let table = Room::Table("t".to_string());
        let mut held = Vec::new();
        for n in 1..=STREAMS_PER_CALLER {
            held.push(streams.admit(table.clone(), &user("red@wire.test")).unwrap_or_else(|e| panic!("stream {n}: {e}")));
        }
        assert_eq!(streams.admit(table.clone(), &user("red@wire.test")).unwrap_err(), TOO_MANY_STREAMS_CALLER);
        let blue = streams.admit(table.clone(), &user("blue@wire.test"));
        assert!(blue.is_ok(), "another caller is unaffected");
        assert_eq!(streams.open(&table), STREAMS_PER_CALLER + 1);
        drop(held.pop());
        assert!(streams.admit(table.clone(), &user("red@wire.test")).is_ok(), "a closed stream frees the slot");
        // The table's total: the c-callers fill the table to 64 beside red's seven and blue's one; the sixty-fifth, a
        // ninth caller's first, is refused with the TABLE's sentence (the total's verdict comes first).
        let mut fill = Vec::new();
        for caller in 1..=8 {
            for _ in 0..STREAMS_PER_CALLER {
                let who = format!("c{caller}@wire.test");
                if let Ok(slot) = streams.admit(table.clone(), &user(&who)) { fill.push(slot); }
            }
        }
        assert_eq!(streams.open(&table), STREAMS_PER_TABLE);
        assert_eq!(streams.admit(table.clone(), &user("ninth@wire.test")).unwrap_err(), TOO_MANY_STREAMS_TABLE);
        drop(fill);
        drop(held);
        drop(blue);
        assert_eq!(streams.open(&table), 0, "every slot released");
        assert!(streams.counts.lock().unwrap().by_room.is_empty() && streams.counts.lock().unwrap().by_caller.is_empty(), "nothing kept for a quiet room");
        // The house: its own per-caller figure and its own total sentence.
        let mut house = Vec::new();
        for _ in 0..HOUSE_STREAMS_PER_CALLER { house.push(streams.admit(Room::House, &user("red@wire.test")).unwrap()); }
        assert_eq!(streams.admit(Room::House, &user("red@wire.test")).unwrap_err(), TOO_MANY_HOUSE_CALLER);
        for caller in 1..=31 {
            for _ in 0..HOUSE_STREAMS_PER_CALLER { house.push(streams.admit(Room::House, &user(&format!("h{caller}@wire.test"))).unwrap()); }
        }
        assert_eq!(streams.open(&Room::House), HOUSE_STREAMS_TOTAL);
        assert_eq!(streams.admit(Room::House, &user("late@wire.test")).unwrap_err(), TOO_MANY_HOUSE_TOTAL);
        assert_eq!((STREAMS_PER_CALLER, STREAMS_PER_TABLE, HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL), (8, 64, 8, 256));
    }
}
```

In `lib.rs`'s test module, beside `stream_status`:

```rust
    /// `GET …/stream` as a RESPONSE the caller holds — its body is the SSE stream, never polled here — so the
    /// admission slot stays taken for as long as the response lives (security S2c, D3): dropping it is what frees
    /// the slot, exactly as hyper's drop of an abandoned connection does.
    async fn stream_response(state: &Arc<AppState>, uri: &str, who: &str) -> Response {
        app(Arc::clone(state))
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .header(identity::CLAIMED_HEADER, who)
                    .header("accept", "text/event-stream")
                    .body(Body::empty())
                    .expect("a GET"),
            )
            .await
            .expect("the router answers")
    }

    /// **The caps at the routes** (security S2c, finding D3's NAS half; decision D-1): a caller's ninth table
    /// stream is a 429 BEFORE any frame, in the API's error shape with the caller's sentence; the admission read's
    /// own refusal still comes first (a stranger at a locked table reads 403, never 429, with every slot free);
    /// a dropped response frees the slot; the house stream is capped under its own sentence.
    #[tokio::test]
    async fn a_callers_ninth_stream_is_a_429_before_any_frame_and_a_dropped_response_frees_it() {
        let state = Arc::new(AppState::in_memory());
        let (status, body) = post_as(&state, WHO, "/api/games", serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seats": [{"user": WHO}, {"open": true}] })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let id = body["id"].as_str().expect("an id").to_string();
        let uri = format!("/api/games/{id}/stream");
        let mut held = Vec::new();
        for _ in 0..streams::STREAMS_PER_CALLER {
            let res = stream_response(&state, &uri, WHO).await;
            assert_eq!(res.status(), StatusCode::OK);
            held.push(res);
        }
        let ninth = stream_response(&state, &uri, WHO).await;
        assert_eq!(ninth.status(), StatusCode::TOO_MANY_REQUESTS);
        let bytes = axum::body::to_bytes(ninth.into_body(), usize::MAX).await.expect("a body");
        let got: serde_json::Value = serde_json::from_slice(&bytes).expect("the API's error shape");
        assert_eq!(got["error"], streams::TOO_MANY_STREAMS_CALLER);
        assert_eq!(state.streams.open(&streams::Room::Table(id.clone())), streams::STREAMS_PER_CALLER, "the refused one took no slot");
        assert_eq!(stream_status(&state, &uri, "blue@wire.test").await, StatusCode::OK, "another caller is unaffected");
        drop(held.pop());
        assert_eq!(stream_status(&state, &uri, WHO).await, StatusCode::OK, "a dropped response freed its slot");
        // The house, under its own sentence.
        let mut house = Vec::new();
        for _ in 0..streams::HOUSE_STREAMS_PER_CALLER {
            house.push(stream_response(&state, "/api/stream", WHO).await);
        }
        let ninth = stream_response(&state, "/api/stream", WHO).await;
        assert_eq!(ninth.status(), StatusCode::TOO_MANY_REQUESTS);
        let bytes = axum::body::to_bytes(ninth.into_body(), usize::MAX).await.expect("a body");
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["error"], streams::TOO_MANY_HOUSE_CALLER);
        drop(house);
        assert_eq!(state.streams.open(&streams::Room::House), 0);
    }

    /// **A cap never masks the admission read's own refusal** (M27's catcher, which the plan's route cell lacks):
    /// a caller holding the whole per-caller figure at a table that is then deleted reads 404, never 429, and the
    /// refused request takes no slot.
    #[tokio::test]
    async fn a_cap_does_not_mask_the_admission_reads_refusal() {
        let state = Arc::new(AppState::in_memory());
        let (status, body) = post_as(&state, WHO, "/api/games", serde_json::json!({ "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seats": [{"user": WHO}, {"open": true}] })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let id = body["id"].as_str().expect("an id").to_string();
        let uri = format!("/api/games/{id}/stream");
        let mut held = Vec::new();
        for _ in 0..streams::STREAMS_PER_CALLER {
            held.push(stream_response(&state, &uri, WHO).await);
        }
        let (status, _) = delete_as(&state, WHO, &format!("/api/games/{id}")).await;
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(stream_status(&state, &uri, WHO).await, StatusCode::NOT_FOUND, "the table's refusal comes first");
        assert_eq!(state.streams.open(&streams::Room::Table(id.clone())), streams::STREAMS_PER_CALLER, "the refused request took no slot (a stranger's too)");
        drop(held);
    }
```

(`WHO`, `post_as`, `delete_as`, `stream_status` are the file's; the second cell is M27's catcher, plan review
L1-F2; `stream_status`'s drop of its own response at the end of the call
is what makes its `OK` rows free their slots at once, which the cell relies on. If `stream_status` returns before
the response is dropped — it returns `res.status()` by value, so the response drops at the function's end — the
premise holds; the reviewer confirms by reading it.)

- [ ] **Step 2: Run them red.** `cargo test -p yokohama-web --lib streams::` — expected: a compile error
  (`unresolved module streams`).

- [ ] **Step 3: `streams.rs`.**

```rust
//! Open-stream caps on the NAS (security S2c, finding D3's NAS half; S2b's Worker half is the socket caps in
//! `worker/src/table/sockets.ts` and `house.ts`, decision D-4 there).
//!
//! **Counted at admission, OPEN streams only.** [`Streams::admit`] hands back a [`Slot`] that the SSE body holds
//! ([`crate::Admitted`]) and releases on its drop — so a connection the client abandons frees its slot exactly when
//! hyper drops the body, never before and never later, and nothing else can free one. The review's measurement:
//! 500 streams as 500 watchers in 3.1 s, one chat post fanned out to 500 admission reads; each table-stream frame
//! costs a `spawn_blocking` read, and the pool is 512 threads, which is why the house's total is 256 here where the
//! Worker's lobby holds 1,024 sockets per object.
//!
//! **The figures mirror the Worker's** so the two hosts read alike: 8 per caller a room, 64 a table. The verdicts in
//! the Worker's order — the room's total first, then the caller's — each a constant sentence and never a caller
//! value. The maps hold an entry only while a stream is open (B8's lesson: a map nobody has written costs nothing).
//!
//! **The per-caller key is the identity the door hands over** — the verified address under Access, the claimed name
//! under the open door, where a name rotation escapes the per-caller figure and meets the room's total alone (S2b's
//! D-3, recorded for S4); the room's total is the bound a process relies on.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::identity::User;

/// Streams one caller may hold open at one table: a table page holds one, the owner's "a couple of games at once"
/// a few; eight leaves room for every tab of one person and refuses a script. The Worker's `SOCKETS_PER_CALLER`.
pub const STREAMS_PER_CALLER: usize = 8;
/// Streams one table may hold open in all: four seats and their watchers at eight each. The Worker's
/// `SOCKETS_PER_TABLE`.
pub const STREAMS_PER_TABLE: usize = 64;
/// Lobby streams one caller may hold open. The Worker's `LOBBY_PER_CALLER`.
pub const HOUSE_STREAMS_PER_CALLER: usize = 8;
/// Lobby streams the process holds open in all: 32 people at eight, half the blocking pool a house bump could
/// otherwise occupy (the Worker's lobby object holds 1,024 because it pays no admission read per socket).
pub const HOUSE_STREAMS_TOTAL: usize = 256;

pub const TOO_MANY_STREAMS_CALLER: &str = "too many open streams from you at this table — close one and try again";
pub const TOO_MANY_STREAMS_TABLE: &str = "this table has too many open streams — try again in a moment";
pub const TOO_MANY_HOUSE_CALLER: &str = "too many open lobby streams from you — close one and try again";
pub const TOO_MANY_HOUSE_TOTAL: &str = "the lobby has too many open streams — try again in a moment";

/// A room a stream is about: one table, or the house.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Room {
    Table(String),
    House,
}

#[derive(Default)]
struct Counts {
    by_room: HashMap<Room, usize>,
    by_caller: HashMap<(Room, String), usize>,
}

/// The counts, one per process — `AppState.streams`.
#[derive(Default)]
pub struct Streams {
    counts: Mutex<Counts>,
}

/// One admitted stream's place in the counts, released on drop.
pub struct Slot {
    streams: Arc<Streams>,
    room: Room,
    caller: String,
}

impl Streams {
    /// The verdict for one more stream by `caller` in `room`: the room's total first, then the caller's — `Err`
    /// carries the sentence the 429 repeats; `Ok` holds the slot until it is dropped.
    pub fn admit(self: &Arc<Self>, room: Room, caller: &User) -> Result<Slot, &'static str> {
        let (per_caller, total, caller_sentence, total_sentence) = match room {
            Room::Table(_) => (STREAMS_PER_CALLER, STREAMS_PER_TABLE, TOO_MANY_STREAMS_CALLER, TOO_MANY_STREAMS_TABLE),
            Room::House => (HOUSE_STREAMS_PER_CALLER, HOUSE_STREAMS_TOTAL, TOO_MANY_HOUSE_CALLER, TOO_MANY_HOUSE_TOTAL),
        };
        let key = (room.clone(), caller.as_str().to_string());
        let mut counts = self.counts.lock().expect("streams lock");
        if counts.by_room.get(&room).copied().unwrap_or(0) >= total {
            return Err(total_sentence);
        }
        if counts.by_caller.get(&key).copied().unwrap_or(0) >= per_caller {
            return Err(caller_sentence);
        }
        *counts.by_room.entry(room.clone()).or_insert(0) += 1;
        *counts.by_caller.entry(key.clone()).or_insert(0) += 1;
        Ok(Slot { streams: Arc::clone(self), room, caller: key.1 })
    }

    /// Open streams in `room` — for the cells and a reading.
    pub fn open(&self, room: &Room) -> usize {
        self.counts.lock().expect("streams lock").by_room.get(room).copied().unwrap_or(0)
    }
}

fn release<K: std::hash::Hash + Eq>(map: &mut HashMap<K, usize>, key: &K) {
    if let Some(n) = map.get_mut(key) {
        *n -= 1;
        if *n == 0 {
            map.remove(key);
        }
    }
}

impl std::fmt::Debug for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Slot").field("room", &self.room).finish_non_exhaustive()
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        // Poison-tolerant, in `games::enter`'s form (the plan review's AD-18): nothing under this lock panics today,
        // and an `expect` here would turn a poisoned lock into a panic inside a drop.
        let mut counts = self.streams.counts.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        release(&mut counts.by_room, &self.room);
        release(&mut counts.by_caller, &(self.room.clone(), std::mem::take(&mut self.caller)));
    }
}
```

- [ ] **Step 4: The error constructor, the state field, the adapter and the two routes.**

```rust
// lib.rs — beside `ApiError::bad_request` (the same impl block, the same shape):
    /// A caller over a cap (security S2c, D3's NAS half): 429, the sentence a constant; no `Retry-After`, since the
    /// client ends a signal on it rather than waiting (the client's decision D-2).
    pub fn too_many_requests(message: impl Into<String>) -> Self {
        Self { status: StatusCode::TOO_MANY_REQUESTS, message: Self::capped(message.into()), table_status: None, foreign_bot: None }
    }
```

The constructor caps its message, as every `ApiError` constructor does (plan review L1-R2): the cell
`every_api_error_message_is_capped_at_construction` counts the impl block's `message: impl Into<String>` signatures
and its `Self::capped(message.into())` calls against its `built` vec, so the vec gains the new constructor after
`ApiError::server_error(long.clone()),`, and the cell's comment "the three built above. A fourth constructor" reads
"the four built above. A fifth constructor" (Task 2 adds the fifth):

```rust
            ApiError::too_many_requests(long.clone()),
```

```rust
// lib.rs — AppState, a field after `signals`, set to `Arc::new(streams::Streams::default())` in every constructor
// that builds the struct (the base constructor; the `..self` builders inherit it):
    /// The open-stream counts (security S2c, finding D3's NAS half): one per process, read by the two stream
    /// routes at admission.
    pub streams: Arc<streams::Streams>,
```

```rust
// lib.rs — beside `UntilLast` (line 841 today):
/// A stream that holds its admission slot until it is dropped (security S2c, D3): the slot lives inside the SSE
/// body, so hyper's drop of an abandoned connection is what frees it, and nothing else can.
#[cfg(feature = "server")]
struct Admitted<S> {
    inner: S,
    _slot: streams::Slot,
}

#[cfg(feature = "server")]
impl<S: tokio_stream::Stream + Unpin> tokio_stream::Stream for Admitted<S> {
    type Item = S::Item;
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<S::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}
```

```rust
// lib.rs — `stream`, after the admission read and before the subscription:
    api::events_at(&state.games, &id, &user)?;
    // **The cap, after the admission read's own verdict** (security S2c, D3): a 403 or a 404 about the table is
    // answered first and takes no slot; the (n+1)-th stream of a caller, or of the table, is a 429 before any
    // frame. The slot rides the body (`Admitted`), so an abandoned connection frees it when hyper drops the body.
    let slot = Arc::clone(&state.streams)
        .admit(streams::Room::Table(id.clone()), &user)
        .map_err(ApiError::too_many_requests)?;
    let rx = state.signals.subscribe(&id);
    …
    Ok(Sse::new(Admitted { inner: UntilLast { inner: Box::pin(frames), done: false }, _slot: slot })
        .keep_alive(KeepAlive::new().interval(heartbeat)))
```

```rust
// lib.rs — `house_stream` answers a Result now, so the cap's 429 has a shape:
async fn house_stream(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    use axum::response::sse::{Event, KeepAlive, Sse};
    use tokio_stream::StreamExt;

    // **The cap** (security S2c, D3): the house's own figures and sentences; identity is still the whole of the
    // admission, and the slot rides the body as a table stream's does.
    let slot = Arc::clone(&state.streams).admit(streams::Room::House, &user).map_err(ApiError::too_many_requests)?;
    let rx = state.signals.subscribe_house();
    let frames = tokio_stream::wrappers::WatchStream::from_changes(rx).map(|h| {
        Ok::<_, std::convert::Infallible>(Event::default().event("house").data(signal::house_frame(h.seq, h.chat, h.rev)))
    });
    Ok(Sse::new(Admitted { inner: frames, _slot: slot }).keep_alive(KeepAlive::new().interval(state.heartbeat)))
}
```

(If `WatchStream`'s `map` is not `Unpin` under the compiler's reading, `Box::pin(frames)` as `stream` already does
— `Pin<Box<S>>` is `Unpin` and `Stream`. The reviewer records which form compiled.)

- [ ] **Step 5: Run the cells green.** `cargo test -p yokohama-web --lib -- streams:: a_callers_ninth` —
  expected: both green; every stream cell in the file still green (the caps are far above what any cell opens).

- [ ] **Step 6: The suites (§"Global Constraints"), then commit.**

```bash
git add crates/yokohama-web/src/streams.rs crates/yokohama-web/src/lib.rs
git commit -m "web: open-stream caps on the NAS — 8 per caller a room, 64 a table, the house 8 and 256, OPEN streams only by a slot the SSE body holds, a 429 before any frame (security S2c, D3's NAS half)"
```

**Named mutants (the reviewer's, on an export):** M23 `>=` → `>` at the caller's verdict → the module cell red at
the ninth; M24 `Slot::drop` emptied → the module cell red at "a closed stream frees the slot" and the route cell at
"freed its slot"; M25 the two verdicts swapped → the module cell red at the sixty-fifth (the sentence); M26 the
house arm counting into the table's figures (`Room::House` mapped to the table constants) → the house rows red at
the 65th instead of the 257th; M27 the route's slot taken BEFORE `events_at` → `a_cap_does_not_mask_the_admission_reads_refusal` red
at the 404 (a caller holding eight slots at a deleted table reads 429 instead of the table's own refusal); the
stranger-at-a-locked-table cells stay green under it, and `state.streams.open` reads ZERO after a refused request
either way, because the `?` on `events_at` drops the local slot — so only a caller already holding slots shows M27
(plan review L1-F2); M28 `house_stream` back to `impl IntoResponse` without the cap → the house
rows red.

---

### Task 2: The body-read deadline at the adapter — 10 seconds, 408 — Opus (R-7; decision D-3; Review Focus 2)

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` — the adapter's body read (line 1049 today), `AppState` (a field and a
  builder), `ApiError` (a constructor), the two constants.
- Test: the cell below in `lib.rs`'s test module; `every_api_error_message_is_capped_at_construction`'s `built` vec
  (Step 3).

**Interfaces:**
- Consumes: Task 1's `AppState` shape (the field list).
- Produces: `BODY_READ_DEADLINE`, `BODY_TOO_SLOW`, `ApiError::request_timeout`, `AppState.body_deadline`,
  `AppState::with_body_deadline`.

- [ ] **Step 1: The failing cell.**

```rust
    /// **A body that does not arrive is answered, not awaited** (security S2c, D3's companion, "the request body
    /// read timeout" the S1 stack review left unverified; decision D-3; Review Focus 2): a POST whose headers
    /// promise a body and whose body then STALLS — one chunk, then a channel nobody closes — is answered 408 with
    /// `BODY_TOO_SLOW` once the deadline passes, in the API's error shape, under a deadline the cell shortens to
    /// 50 ms. The same body completed in two chunks under the deadline reaches the route and is answered by it (a
    /// 201: the two chunks are one valid create), which pins that the chunks are joined and the deadline is the read's
    /// alone. The whole cell runs under its own 2-second bound so a mutant that awaits forever reads as RED.
    #[tokio::test]
    async fn a_stalled_request_body_is_answered_408_after_the_deadline_and_a_slow_but_arriving_one_reaches_the_route() {
        use tokio_stream::wrappers::ReceiverStream;
        let state = Arc::new(AppState::in_memory().with_body_deadline(std::time::Duration::from_millis(50)));
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(4);
        tx.send(Ok(b"{\"players\":".to_vec())).await.expect("the first chunk");
        let stalled = Request::builder()
            .method("POST")
            .uri("/api/games")
            .header(identity::CLAIMED_HEADER, WHO)
            .header("content-type", "application/json")
            .body(Body::from_stream(ReceiverStream::new(rx)))
            .expect("a request");
        let res = tokio::time::timeout(std::time::Duration::from_secs(2), app(Arc::clone(&state)).oneshot(stalled))
            .await
            .expect("answered within the cell's bound: the deadline, not the body, ended the read")
            .expect("the router answers");
        drop(tx);
        assert_eq!(res.status(), StatusCode::REQUEST_TIMEOUT);
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.expect("a body");
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["error"], BODY_TOO_SLOW);

        // The anti-vacuity: two chunks that both arrive under the deadline are one body to the route.
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<Vec<u8>, std::io::Error>>(4);
        let feeder = tokio::spawn(async move {
            tx.send(Ok(b"{\"players\":2,\"colors\":[\"Red\",\"Blue\"],".to_vec())).await.expect("chunk 1");
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            tx.send(Ok(b"\"start_seat\":0,\"seats\":[{\"user\":\"red@wire.test\"},{\"open\":true}]}".to_vec())).await.expect("chunk 2");
        });
        let slow = Request::builder()
            .method("POST")
            .uri("/api/games")
            .header(identity::CLAIMED_HEADER, WHO)
            .header("content-type", "application/json")
            .body(Body::from_stream(ReceiverStream::new(rx)))
            .expect("a request");
        let res = app(Arc::clone(&state)).oneshot(slow).await.expect("the router answers");
        feeder.await.expect("the feeder");
        assert_eq!(res.status(), StatusCode::CREATED, "both chunks reached the route as one body");
        assert_eq!(BODY_READ_DEADLINE, std::time::Duration::from_secs(10));
        assert_eq!(AppState::in_memory().body_deadline, BODY_READ_DEADLINE, "the production figure on every constructor");
    }
```

(`tokio_stream::wrappers::ReceiverStream` is in the crate's `tokio-stream`; if the `sync` feature alone does not
expose it, enable no new feature — `ReceiverStream` is unconditional in tokio-stream 0.1 — and the reviewer confirms
by reading the lock's version.)

- [ ] **Step 2: Run it red.** `cargo test -p yokohama-web --lib a_stalled_request_body` — expected: a compile error
  on `with_body_deadline`/`BODY_TOO_SLOW`; after stubbing them, the first `oneshot` would hang until the cell's
  2-second `timeout` fires — that `expect` is the red.

- [ ] **Step 3: The deadline.**

```rust
// lib.rs — beside HEARTBEAT / REPLAY_WAIT, each constant under its own `#[cfg(feature = "server")]` as REPLAY_WAIT is
// (the fence cell names an ungated item in a server region):
/// How long a request body may take to arrive, in all (security S2c, D3's companion; decision D-3): every
/// legitimate body is under 512 KiB and arrives in one burst over the tunnel; ten seconds is a slow phone on a bad
/// link, and a stall past it is a connection somebody is holding, which this turns from an unbounded hold into a
/// rate the edge and the caps already bound. `axum::serve` sets no timer, so without this the binary bounds no
/// read at all.
#[cfg(feature = "server")]
pub const BODY_READ_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
/// The 408's sentence.
#[cfg(feature = "server")]
pub const BODY_TOO_SLOW: &str = "the request body did not arrive within 10 seconds";
```

```rust
// lib.rs — AppState: a field after `replay_wait`, `BODY_READ_DEADLINE` on every constructor, and a builder ABOVE
// `with_replay_gate`'s doc comment, never between that comment and its fn (plan review I-14):
    /// The body read's deadline — [`BODY_READ_DEADLINE`]; a cell shortens it through [`Self::with_body_deadline`].
    pub body_deadline: std::time::Duration,

    pub fn with_body_deadline(self, deadline: std::time::Duration) -> Self {
        Self { body_deadline: deadline, ..self }
    }
```

```rust
// lib.rs — ApiError, beside `too_many_requests`:
    /// A body that did not arrive in time (security S2c, decision D-3): 408 with [`BODY_TOO_SLOW`].
    pub fn request_timeout(message: impl Into<String>) -> Self {
        Self { status: StatusCode::REQUEST_TIMEOUT, message: Self::capped(message.into()), table_status: None, foreign_bot: None }
    }
```

And the capped-construction cell's `built` vec gains the fifth after `too_many_requests`, its comment reading "the five
built above. A sixth constructor" (plan review L1-R2):

```rust
            ApiError::request_timeout(long.clone()),
```

```rust
// lib.rs — the adapter's read (line 1049 today):
    let (parts, body) = req.into_parts();
    // **The ONE body read, under a deadline** (security S2c, decision D-3): every `/api` request with a body reaches
    // this site — the catch-all and `/api/games` route here; the three separate routes and `/enter` are GETs — so
    // one `timeout` bounds every body, and the limit arm below answers exactly as it did.
    let bytes = match tokio::time::timeout(state.body_deadline, axum::body::to_bytes(body, 2 << 20)).await {
        Ok(Ok(b)) => b,
        Ok(Err(e)) => return ApiError::bad_request(format!("the body could not be read: {e}")).into_response(),
        Err(_elapsed) => return ApiError::request_timeout(BODY_TOO_SLOW).into_response(),
    };
```

- [ ] **Step 4: Run the cell green**, then the file's adapter cells (`cargo test -p yokohama-web --lib`), which read
  the same answers as before for every body that arrives.

- [ ] **Step 5: The suites, then commit.**

```bash
git add crates/yokohama-web/src/lib.rs
git commit -m "web: a request body has ten seconds to arrive — one deadline at the adapter's single read site, 408 with its sentence; a stalled body no longer holds a connection (security S2c, D3's companion)"
```

**Named mutants:** M29 the `timeout` removed → the cell's own 2-second bound reds the first `expect`; M30 the
status 400 in the elapsed arm → the cell red at `REQUEST_TIMEOUT`; M31 the deadline applied after `to_bytes`
completes (a `sleep` instead of a `timeout`) → the anti-vacuity row red (the slow body answered 408); M32 the
constructors leaving `body_deadline` at zero → every adapter cell red at once (a zero deadline answers 408 to
everything).

### Task 3: D8's character rule — control, format and line-separator characters refused in names and chat — Opus (R-2; decision D-4; Review Focus 3)

**Files:**
- Create: `crates/yokohama-web/src/text.rs`
- Modify: `crates/yokohama-web/src/lib.rs` (the module list, beside `pub mod echo;`), `crates/yokohama-web/src/tables.rs:522-531`
  (`check_name`, and `check_stored_name` beside it — the rule as it stood, for READS; `check_shape`'s two name sites
  call it), `crates/yokohama-web/src/chat.rs:430-441` (`check_text`; `check_shape`'s author-name site and `normalized`
  call `crate::tables::check_stored_name`, and `use crate::tables::check_name;` goes), `crates/yokohama-web/src/lib.rs`
  (two cells assert the old WRITE sentence: `the_post_refuses_in_the_spec_s_order` and
  `the_house_checks_the_name_and_reads_its_switch_before_the_text`), and any other cell that asserts the old sentences
  by their literal text (`grep -rn "no control characters" crates/yokohama-web/src`; the READ side's literal in
  `check_shape_bounds_the_tables_own_name` STAYS).
- Test: the cells below in `text.rs`, `tables.rs` and `chat.rs`, and the compatibility cell in `lib.rs` (Step 5).

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `text::invisible(&str) -> Option<char>`, `text::INVISIBLE_RULE`, `text::FORMAT_RANGES`,
  `text::FORMAT_COUNT`, `text::is_format`, `text::is_line_separator` — Task 4's cells post names and text that must
  pass this rule.

- [ ] **Step 1: Write the failing cells in `text.rs`** (the module does not exist yet, so the crate does not compile:
  that is the red).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// **The table IS Unicode 17.0.0's General_Category Cf** — the toolchain's own version (`char::UNICODE_VERSION`),
    /// so `is_control`'s Cc and this table's Cf are read from the same edition: 21 sorted, disjoint ranges, 170 code
    /// points, every bound a scalar value, and the predicate counts exactly those over the whole space. A toolchain
    /// that moves its tables fails the first line, which says what to do.
    #[test]
    fn the_format_table_is_unicode_17s_cf_sorted_disjoint_and_170_long() {
        assert_eq!(char::UNICODE_VERSION, (17, 0, 0), "the toolchain's Unicode tables moved: re-transcribe FORMAT_RANGES from that version's DerivedGeneralCategory.txt and move this pin");
        assert_eq!(FORMAT_RANGES.len(), 21);
        let total: u32 = FORMAT_RANGES.iter().map(|&(a, b)| b - a + 1).sum();
        assert_eq!(total as usize, FORMAT_COUNT);
        assert_eq!(FORMAT_COUNT, 170, "the UCD's count for 17.0.0 (2025-07-24)");
        for w in FORMAT_RANGES.windows(2) {
            assert!(w[0].1 < w[1].0, "sorted and disjoint: {:?} then {:?}", w[0], w[1]);
        }
        for &(a, b) in &FORMAT_RANGES {
            assert!(a <= b, "{a:04X}..{b:04X}");
            assert!(char::from_u32(a).is_some() && char::from_u32(b).is_some(), "{a:04X}..{b:04X} must be scalar values");
        }
        let counted = (0..=0x10FFFF).filter_map(char::from_u32).filter(|c| is_format(*c)).count();
        assert_eq!(counted, FORMAT_COUNT, "the predicate and the table disagree");
        assert_eq!((0..=0x10FFFF).filter_map(char::from_u32).filter(|c| c.is_control()).count(), 65, "Cc, the toolchain's");
    }

    /// **What the rule refuses and what it admits** (finding D8, measured: a watcher posted as `"Bob\u{200B}"` and
    /// read back as the seat holder "Bob"; `"\u{202E}boB"` reversed the line). One representative from every family
    /// — zero-width, bidi override and isolate, the BOM, the soft hyphen, a tag character, a musical format
    /// character, the Mongolian vowel separator, the two line separators, a control — against visible text in four
    /// scripts, an emoji, a flag (two regional indicators) and a keycap (a variation selector, category Mn, passes).
    /// The joiner is Cf and is refused: a multi-person emoji sequence is the accepted loss (decision D-4).
    #[test]
    fn invisible_refuses_control_format_and_line_separators_and_admits_visible_text() {
        for bad in [
            "a\u{200B}b", "\u{202E}evil", "x\u{2066}y", "\u{FEFF}bom", "soft\u{00AD}hyphen", "tag\u{E0041}",
            "music\u{1D173}", "\u{180E}", "\u{110BD}", "\u{E0001}", "line\u{2028}two", "para\u{2029}two",
            "a\tb", "ctl\u{7}", "join\u{200D}er", "\u{061C}", "\u{FFF9}",
        ] {
            assert!(invisible(bad).is_some(), "{bad:?} must be refused");
        }
        for good in ["héllo", "日本語", "Кириллица", "العربية", "🙂 ok", "\u{1F1EF}\u{1F1F5}", "1\u{FE0F}\u{20E3}", "Ben Castle", "a..b-c_d", "x y"] {
            assert_eq!(invisible(good), None, "{good:?} is visible text");
        }
        assert_eq!(invisible("ok\u{202E}x"), Some('\u{202E}'), "the first offending character, for a cell — never for a sentence");
        assert_eq!(INVISIBLE_RULE, "no control, invisible or direction-changing character");
    }
}
```

- [ ] **Step 2: Run them to see the red.** `cargo test -p yokohama-web --lib text::` — expected: `0 passed` with the file
  outside the module tree; write `pub mod text;` in Step 1 (beside `pub mod echo;`) and the red is the compile error
  on `invisible`, `FORMAT_RANGES` and the rest.

- [ ] **Step 3: Write `text.rs`.**

```rust
//! The character rule for a person's text (security S2c, finding D8; decision D-4): a name, a display name, a
//! watcher's name and a chat line carry NO control (Cc), format (Cf) or line-separator (Zl, Zp) character. `Cc` is
//! `char::is_control`; `Cf` is the table below; `Zl` and `Zp` are two code points. Nothing else is judged here —
//! confusable letters are provenance's problem, which the tag answers (`chat::tag_of`), not this rule's. **What the
//! rule does not reach**, measured at the plan review (L1-F4): the default-ignorable LETTERS and marks outside Cc/Cf
//! (U+115F, U+1160, U+3164, U+FFA0 the Hangul fillers, U+034F, U+17B4, U+17B5) and U+2800 (Braille blank, So) pass,
//! so `"Bob\u{3164}"` reads as `Bob`; the tag is what tells such a watcher from the seat holder (a seat carries none).
//! And U+200C, the zero-width NON-joiner that Persian and several Indic scripts spell with, is Cf and is refused.
//!
//! **Why a table and not a crate:** this module compiles to every host — the NAS binary, the browser's wasm and
//! the Worker's — and the fence admits nothing new to that tree; the Cf category is 21 ranges, which a reader can
//! check against the UCD in a minute. **Transcribed from Unicode 17.0.0**, the toolchain's own version
//! (`char::UNICODE_VERSION`, pinned by a cell): `https://www.unicode.org/Public/17.0.0/ucd/extracted/DerivedGeneralCategory.txt`
//! (dated 2025-07-24), the lines whose category is `Cf`, in the file's order. A later toolchain that moves its
//! tables fails the pin, which names this comment.

/// Unicode 17.0.0's `General_Category=Cf`, 21 ranges, 170 code points — see the module doc for the source.
pub const FORMAT_RANGES: [(u32, u32); 21] = [
    (0x00AD, 0x00AD),
    (0x0600, 0x0605),
    (0x061C, 0x061C),
    (0x06DD, 0x06DD),
    (0x070F, 0x070F),
    (0x0890, 0x0891),
    (0x08E2, 0x08E2),
    (0x180E, 0x180E),
    (0x200B, 0x200F),
    (0x202A, 0x202E),
    (0x2060, 0x2064),
    (0x2066, 0x206F),
    (0xFEFF, 0xFEFF),
    (0xFFF9, 0xFFFB),
    (0x110BD, 0x110BD),
    (0x110CD, 0x110CD),
    (0x13430, 0x1343F),
    (0x1BCA0, 0x1BCA3),
    (0x1D173, 0x1D17A),
    (0xE0001, 0xE0001),
    (0xE0020, 0xE007F),
];

/// How many code points the table covers — the UCD's own count for the category, pinned by a cell that sums the
/// ranges AND counts the predicate over the whole scalar space.
pub const FORMAT_COUNT: usize = 170;

/// The rule, as every refusal words it: the sentence names the rule, never the character (the echo rule's spirit —
/// an invisible character quoted back would be invisible in the refusal too).
pub const INVISIBLE_RULE: &str = "no control, invisible or direction-changing character";

/// `General_Category=Cf` in Unicode 17.0.0.
pub fn is_format(c: char) -> bool {
    let u = c as u32;
    FORMAT_RANGES.iter().any(|&(a, b)| (a..=b).contains(&u))
}

/// `Zl` (U+2028 LINE SEPARATOR) and `Zp` (U+2029 PARAGRAPH SEPARATOR): invisible, and a line break a "one line"
/// rule must refuse as it refuses `\n`.
pub fn is_line_separator(c: char) -> bool {
    matches!(c, '\u{2028}' | '\u{2029}')
}

/// The first character of `s` the rule refuses, or `None` for text a person can see whole. The character is for a
/// CELL to assert on; a sentence carries [`INVISIBLE_RULE`].
pub fn invisible(s: &str) -> Option<char> {
    s.chars().find(|&c| c.is_control() || is_format(c) || is_line_separator(c))
}
```

And in `lib.rs`'s module list, beside `pub mod echo;`: `pub mod text;` (the module is unconditional — every host).

- [ ] **Step 4: Run the cells.** `cargo test -p yokohama-web --lib text::` — expected: 2 passed.

- [ ] **Step 5: Write the failing cells at the two sites.** In `tables.rs`'s test module, beside the existing
  `check_name` cells:

```rust
    /// **A name is visible whole** (security S2c, finding D8 — measured in review: `"Bob\u{200B}"` read as the seat
    /// holder "Bob"): a zero-width space, a bidi override and a BOM are refused with the rule's sentence; the
    /// control-character refusal keeps that same sentence; a visible name in another script passes. The sentence is
    /// pinned at its literal.
    #[test]
    fn a_name_with_an_invisible_or_direction_changing_character_is_refused_by_the_rule() {
        for bad in ["Bob\u{200B}", "Re\u{202E}d", "\u{FEFF}Owner", "Jeremy\u{2066}", "a\tb", "two\u{2028}lines"] {
            assert_eq!(check_name(bad), Err("a name has no control, invisible or direction-changing character".to_string()), "{bad:?}");
        }
        assert_eq!(check_name("  Ben   Castle "), Ok("Ben   Castle".to_string()), "trimmed, as before");
        assert_eq!(check_name("日本語の名前"), Ok("日本語の名前".to_string()));
        assert_eq!(check_name("🙂 Ben"), Ok("🙂 Ben".to_string()));
        assert!(check_name(&"n".repeat(65)).unwrap_err().starts_with("a name is 1 to 64 characters"), "the length rule speaks first, as before");
    }
```

In `chat.rs`'s test module, beside `check_text_trims_and_bounds_by_characters_not_bytes`:

```rust
    /// **A line is visible whole, and the LENGTH rule speaks first** (security S2c, finding D8; Review Focus 3):
    /// the review's reversed line is refused with the rule's sentence; a line of exactly the cap with one zero-width
    /// space inside it is refused by the CHARACTER sentence (it is within the cap), and one character over the cap
    /// by the LENGTH sentence — so a person reads the rule they can act on. An emoji line passes; a multi-person
    /// emoji sequence (a joiner, U+200D, category Cf) does not — the accepted loss (decision D-4).
    #[test]
    fn a_line_with_an_invisible_character_is_refused_after_the_length_rule() {
        let limits = ChatLimits { text: 10, ..ChatLimits::default() };
        let rule = "a message is one line, with no control, invisible or direction-changing character".to_string();
        assert_eq!(check_text("\u{202E}emag siht", &limits), Err(rule.clone()));
        assert_eq!(check_text("gg\u{200B} resign", &limits), Err(rule.clone()));
        let at_cap = format!("{}\u{200B}", "x".repeat(9));
        assert_eq!(at_cap.chars().count(), 10, "premise: exactly the cap");
        assert_eq!(check_text(&at_cap, &limits), Err(rule.clone()), "within the cap: the character rule");
        let over = format!("{}\u{200B}", "x".repeat(10));
        assert_eq!(check_text(&over, &limits), Err("a message is at most 10 characters".to_string()), "over the cap: the length rule, first");
        assert_eq!(check_text("🙂 gg", &limits), Ok("🙂 gg".to_string()));
        assert_eq!(check_text("\u{1F468}\u{200D}\u{1F469}", &limits), Err(rule), "a joined sequence carries a format character");
    }
```

(`ChatLimits` has `Default` — keep 500, text 500 — and `check_text` checks the length at `chat.rs:435` before the
character at `:438`, so the order cell runs as written.)

In `lib.rs`'s test module, the compatibility catcher (plan review L1-F1; AD-1). It is green before Step 7 and after
it, and red under the plan's first draft, where `check_shape` read names through the new `check_name` and `GET` and
`POST /api/chat` answered 500 (M7a):

```rust
    /// **A document this server wrote under the OLD name rule still reads** (S2c Task 3's compatibility catcher): a
    /// seat's display name and a house watcher's name carrying a zero-width space, stored before the cycle, are
    /// served as they were — `check_shape` is the READ-side rule (`check_stored_name`, `is_control` alone); the new
    /// rule is the write side's. Without this split the lobby chat and the table answered 500 on every route.
    #[tokio::test]
    async fn a_document_stored_under_the_old_name_rule_still_reads_and_the_house_chat_still_takes_a_post() {
        let state = Arc::new(AppState::in_memory());
        let (status, t) = post_as(&state, "red@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
            "id": "pre", "name": "pre", "display_name": "Captain Red",
        })).await;
        assert_eq!(status, StatusCode::CREATED, "{t}");
        let (status, m) = post_as(&state, "green@wire.test", "/api/chat", serde_json::json!({ "text": "hi", "name": "Bob" })).await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        let store = Arc::clone(state.games.store());
        let (json, ver) = store.read(Doc::Table, "pre").expect("a store").expect("the table");
        let json = json.replace("Captain Red", "Captain\\u200BRed");
        store.write_if_version(Doc::Table, "pre", &json, Some(ver)).expect("written");
        let (json, ver) = store.read(Doc::House, "house").expect("a store").expect("the chat");
        let json = json.replace("\"Bob\"", "\"Bob\\u200B\"");
        store.write_if_version(Doc::House, "house", &json, Some(ver)).expect("written");
        assert!(state.games.read_table("pre").expect("the table still reads").is_some());
        let (status, body) = get_as(&state, "red@wire.test", "/api/chat").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let (status, body) = post_as(&state, "red@wire.test", "/api/chat", serde_json::json!({ "text": "still here?" })).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        let (status, body) = post_as(&state, "red@wire.test", "/api/chat", serde_json::json!({ "text": "x", "name": "Bob\u{200B}" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "the WRITE side refuses the same name: {body}");
    }
```

- [ ] **Step 6: Run them red.** `cargo test -p yokohama-web --lib tables::tests::a_name_with_an_invisible` and
  `--lib chat::tests::a_line_with_an_invisible` — expected: both FAIL on the first `Err` assertion (today's sentence
  is "a name has no control characters" / "a message is one line, with no control characters", and `\u{200B}` passes).

- [ ] **Step 7: Change the two rules.**

```rust
// tables.rs:522 — check_name, the body:
pub(crate) fn check_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err("a name is 1 to 64 characters".to_string());
    }
    // **Visible whole** (security S2c, finding D8): `is_control` alone let a zero-width space and a bidi override
    // through, and a watcher posted as a seat holder. The rule is `text::invisible`'s — every host, one table.
    if crate::text::invisible(name).is_some() {
        return Err(format!("a name has {}", crate::text::INVISIBLE_RULE));
    }
    Ok(name.to_string())
}
```

And beside it the READ side (plan review L1-F1; AD-1): `check_shape`'s two name sites in `tables.rs`,
`chat::check_shape`'s author-name site and `chat::normalized` call `check_stored_name` in place of `check_name`, and
`chat.rs`'s `use crate::tables::check_name;` goes; `check_name`'s doc comment "applied on WRITE (create, join, keep)
and on READ ([`check_shape`])" becomes "applied on WRITE (create, join, keep, a watcher's chat name); a stored
document reads under [`check_stored_name`]":

```rust
// tables.rs — beside check_name: the READ side keeps the rule as it stood (plan review L1-F1):
/// **The READ-side name rule: the rule as it stood before S2c** (`is_control` alone). `check_shape` runs on every
/// stored document, and a document this server wrote under the old rule (a display name with a pasted U+200F, a
/// watcher posted as "Bob\u{200B}") must still read; the new rule is the WRITE side's ([`check_name`]).
pub(crate) fn check_stored_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err("a name is 1 to 64 characters".to_string());
    }
    if name.chars().any(char::is_control) {
        return Err("a name has no control characters".to_string());
    }
    Ok(name.to_string())
}
```

```rust
// chat.rs:430 — check_text, the control arm:
    if crate::text::invisible(text).is_some() {
        return Err(format!("a message is one line, with {}", crate::text::INVISIBLE_RULE));
    }
```

- [ ] **Step 8: Run the two files' suites and the corpus.** `cargo test -p yokohama-web --lib tables::` and
  `--lib chat::` — expected: green, the two new cells included; any cell asserting the old sentences by literal is
  moved to the new literal (grep first). `cargo test -p yokohama-web --test wire_fixtures` WITHOUT the flag — expected:
  clean (no fixture carries either old sentence; measured at planning: `grep -rl "no control characters"
  web/src/wire/` is empty).

- [ ] **Step 9: The suites (§"Global Constraints"), then commit.**

```bash
git add crates/yokohama-web/src/text.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/chat.rs
git commit -m "web: names and chat lines refuse control, format and line-separator characters — Unicode 17's Cf transcribed in-source with its count pinned (security S2c, D8)"
```

**Named mutants (the reviewer's, on an export):** M1 `is_format` always `false` → the `invisible` cell red on
`"a\u{200B}b"`; M2 one range dropped from `FORMAT_RANGES` → the table cell red on the count; M3 `check_name` back to
`is_control` alone → the tables cell red; M4 `check_text` likewise → the chat cell red; M5 the length check moved
after the character check → the order cell red at "over the cap"; M6 `is_line_separator` always `false` → the
`invisible` cell red on `"line\u{2028}two"`; M7 `INVISIBLE_RULE` reworded → both site cells red at their literal;
M7a `check_shape` back to `check_name` → the compatibility cell red at the 500 (plan review L1-F1).

---

### Task 4: D8's tag — a per-document salt, a four-hex tag on watcher and house authors, the wire and the corpus — Opus (R-2; decision D-5; Review Focus 4)

**Files:**
- Modify: `crates/yokohama-web/src/chat.rs` (`ChatDoc`, `AuthorView`, `Message::view_for`, `tag_of`),
  `crates/yokohama-web/src/games.rs` (`FIXED_CHAT_SALT`, `Games::chat_salt`), `crates/yokohama-web/src/tables.rs`
  (`append_locked`, the ONE site both `chat_append` at line 1084 and `house_append` at line 1112 reach — their calls
  at 1096 and 1116 — returning the document's salt beside the message; `chat_append_salted` and `house_append_salted`,
  with the two old functions kept as thin wrappers so the ~15 test call sites do not move),
  `crates/yokohama-web/src/api.rs` (the two POST replies, lines 1171 and 1248, the `view_for` sites that hold no
  document), `crates/yokohama-web/src/chat.rs:422` (`ChatDoc::page_for`, the third `view_for` site, with `self.salt` in
  scope); `grep -n view_for crates/yokohama-web/src` lists exactly those three,
  `web/src/types.ts:1090-1093` (`ChatAuthor`), `web/src/wire/contract.ts:740` (the watcher key list, Step 6),
  `web/src/wire/*.json` (regenerated once).
- Test: cells in `chat.rs`, `games.rs` and `tables.rs`; the corpus check; the pool.

**Interfaces:**
- Consumes: Task 3's rule (the cells' names and text pass it).
- Produces: `AuthorView::Watcher { name, you, tag: Option<String> }` on the wire as `"tag": "<4 hex>" | null`; Task
  5's label reads it. `Games::chat_salt()`.

- [ ] **Step 1: Write the failing cells in `chat.rs`.**

```rust
    /// **The tag is four hex of SHA-256 over the room's salt, a newline and the key** (security S2c, finding D8;
    /// decision D-5): deterministic; different by salt (two rooms never share a tag) and by key (two posters never
    /// share one, bar a 1-in-65,536 collision); never longer than four. The reference value is the hash computed by
    /// hand once (`printf 'salt\nred@wire.test' | sha256sum | cut -c1-4`).
    #[test]
    fn a_tag_is_four_hex_of_the_salted_key_and_differs_by_salt_and_by_key() {
        let t = tag_of("salt", "red@wire.test");
        assert_eq!(t.len(), TAG_HEX);
        assert!(t.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()), "{t}");
        assert_eq!(t, tag_of("salt", "red@wire.test"), "deterministic");
        assert_ne!(t, tag_of("other", "red@wire.test"), "a different room, a different tag");
        assert_ne!(t, tag_of("salt", "blue@wire.test"), "a different key, a different tag");
        assert_eq!(TAG_HEX, 4);
    }

    /// **A watcher's view carries the tag; a seat's and a bot's never; a document with no salt carries `None`**
    /// (Review Focus 4: a document written before this cycle, read on any host). The reader's own message is
    /// tagged too — "you · a1b2" tells the reader their own tag so they can recognise it on their next message.
    #[test]
    fn view_for_tags_watcher_authors_only_and_none_without_a_salt() {
        let red = User::parse("red@wire.test").unwrap();
        let watcher = Message { seq: 1, created_at: 0, author: Author::Watcher { user: "green@wire.test".into(), name: Some("Green".into()) }, text: "hi".into(), deleted: false };
        let seat = Message { seq: 2, created_at: 0, author: Author::Seat { seat: 0, user: "red@wire.test".into(), name: Some("Red".into()) }, text: "gl".into(), deleted: false };
        let bot = Message { seq: 3, created_at: 0, author: Author::Bot { seat: 1, name: None }, text: "…".into(), deleted: false };
        match watcher.view_for(&red, Some("salt")).author {
            AuthorView::Watcher { tag, you, .. } => { assert_eq!(tag, Some(tag_of("salt", "green@wire.test"))); assert!(!you); }
            other => panic!("{other:?}"),
        }
        match watcher.view_for(&red, None).author {
            AuthorView::Watcher { tag, .. } => assert_eq!(tag, None, "no salt yet: no tag, and no panic"),
            other => panic!("{other:?}"),
        }
        assert!(matches!(seat.view_for(&red, Some("salt")).author, AuthorView::Seat { you: true, .. }), "a seat is server-bound: no tag field at all");
        assert!(matches!(bot.view_for(&red, Some("salt")).author, AuthorView::Bot { .. }));
        let own = Message { seq: 4, created_at: 0, author: Author::Watcher { user: "red@wire.test".into(), name: None }, text: "me".into(), deleted: false };
        match own.view_for(&red, Some("salt")).author {
            AuthorView::Watcher { tag, you, name } => { assert_eq!(tag, Some(tag_of("salt", "red@wire.test"))); assert!(you); assert_eq!(name, None); }
            other => panic!("{other:?}"),
        }
    }

    /// **The salt is on the wire as a field a reader never sees and an old document reads as absent.**
    #[test]
    fn a_document_without_a_salt_field_deserializes_with_none_and_a_view_carries_no_salt() {
        let old = r#"{"schema":1,"id":"t","last":0,"rev":0,"messages":[]}"#;
        let doc: ChatDoc = serde_json::from_str(old).expect("an old document");
        assert_eq!(doc.salt, None);
        let json = serde_json::to_string(&MessageView { seq: 1, created_at: 0, author: AuthorView::Watcher { name: None, you: false, tag: Some("abcd".into()) }, text: String::new(), deleted: false }).unwrap();
        assert!(json.contains(r#""tag":"abcd""#) && !json.contains("salt"), "{json}");
    }
```

(If `ChatDoc.schema`'s current value is not `1`, write the constant `CHAT_SCHEMA` into the literal; the cell is
about the absent `salt`, not the schema.)

In `games.rs`'s test module:

```rust
    /// **The salt follows the SEED policy** (decision D-5, the seed rule's shape): under `Diagnostic` — the
    /// fixtures, the corpus walk, the pool and `init_diagnostic()` on both wasm hosts — it is the constant, so the
    /// corpus and the pool's replay of it carry the same tags byte for byte; under `SoloOnly`, every production
    /// constructor's, it is 32 hex of OS entropy, fresh each time. No constructor on any host moves.
    #[test]
    fn the_chat_salt_is_fixed_under_diagnostic_and_fresh_entropy_under_solo_only() {
        let diagnostic = crate::AppState::in_memory().games;
        assert_eq!(diagnostic.chat_salt(), FIXED_CHAT_SALT);
        assert_eq!(diagnostic.chat_salt(), FIXED_CHAT_SALT, "the same every time");
        assert_eq!(FIXED_CHAT_SALT.len(), 32);
        let production = crate::AppState::in_memory().with_seed_policy(SeedPolicy::SoloOnly).games;
        let (a, b) = (production.chat_salt(), production.chat_salt());
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()), "{a}");
        assert_ne!(a, b, "entropy, not a constant");
        assert_ne!(a, FIXED_CHAT_SALT);
    }
```

(`in_memory()` builds `Diagnostic` — `lib.rs` line 464's constructor chain; if `.games` is not reachable as a public
field from the test module, use the crate-visible accessor the neighbouring cells use.)

In `tables.rs`'s test module, beside the chat cells:

```rust
    /// **The first append gives a document its salt, and a second keeps it** (decision D-5): two watchers' tags
    /// differ, one watcher's two messages share one, and the stored document carries the salt the views were
    /// derived from. Under `in_memory()` the salt is the constant, which is what makes the corpus byte-stable.
    #[test]
    fn the_first_append_salts_the_document_and_tags_are_stable_within_it() {
        // Build the state and a table exactly as the file's watcher-chat cell does (the cell that posts as a
        // WATCHER with a `name` — `grep -n "Watcher" crates/yokohama-web/src/tables.rs` in the test module), then:
        //   post as green@wire.test with name "Green"; post as blue@wire.test with name "Blue"; post as green again.
        // Read the page as red@wire.test and assert:
        //   the three author views are Watcher; views[0].tag == views[2].tag; views[0].tag != views[1].tag;
        //   every tag is Some and 4 hex;
        //   the stored ChatDoc (read through the store as the file's other chat cells read it) has salt == Some(FIXED_CHAT_SALT.to_string()).
        // NOTE (plan review L1-R11): the file's `games()` helper is `SoloOnly` (entropy) — build this cell's state with
        // `games().with_seed_policy(SeedPolicy::Diagnostic)`; and no table-level WATCHER chat cell exists to copy (a
        // watcher posts only at an unlocked table), so append `Author::Watcher` through `chat_append` directly, as
        // `a_house_append_bumps_…` (~line 5794) does for the house. About 33 lines.
    }

    /// **The POST's reply carries the tag a later page will** (the plan's gap: `view_for` has three call sites, two
    /// of them the replies, which hold a message and no document): the first message of a fresh room is answered
    /// with the room's tag, equal to the page's.
    #[test]
    fn a_post_reply_carries_the_same_tag_as_the_page_that_follows() {
        let games = games().with_seed_policy(crate::games::SeedPolicy::Diagnostic);
        let green = who("green@t.test");
        let author = Author::Watcher { user: "green@t.test".into(), name: Some("Green".into()) };
        let (message, salt) = games.house_append_salted(author, "first words").expect("a house post");
        let reply = message.view_for(&green, salt.as_deref());
        let (page, _) = games.house_page(&green, 0).expect("the house");
        assert_eq!(page.len(), 1);
        assert_eq!(reply.author, page[0].author, "reply and page draw the same author, tag included");
        assert!(matches!(&reply.author, crate::chat::AuthorView::Watcher { tag: Some(_), you: true, .. }), "{reply:?}");
    }
```

The implementer writes this cell's body against the file's own helpers (the plan does not repeat four hundred lines
of fixture setup); the ASSERTIONS above are the cell, and the reviewer checks each is present. The second cell,
`a_post_reply_carries_the_same_tag_as_the_page_that_follows`, is the replies' catcher: they are two of `view_for`'s
three sites and hold no document (plan review L1-F3).

- [ ] **Step 2: Run them red.** `cargo test -p yokohama-web --lib chat::tests::a_tag_is` etc. — expected: compile
  errors (`tag_of`, `TAG_HEX`, `salt`, `tag` do not exist).

- [ ] **Step 3: The layer.**

```rust
// chat.rs — ChatDoc gains the salt (after `rev`, before `messages`):
    /// **The room's salt** (security S2c, finding D8; decision D-5): what a watcher's or house poster's tag is
    /// derived from, with their key. Given on the document's first append after this cycle by `Games::chat_salt`;
    /// `None` on a document written before, until then. Never served: the tag is served, and it is one way.
    #[serde(default)]
    pub salt: Option<String>,
// … and every literal of the struct gains `salt: None`: `ChatDoc::empty` (`chat.rs:381`), the test helper `fn doc`
// (`chat.rs:568`) and `tables.rs:5562`'s; the cell `a_page_marks_you_on_the_readers_own_messages_and_carries_no_user`
// (`chat.rs:841`) gains `"tag": null` in its two watcher `json!` literals.

// chat.rs — beside the other `pub const`s:
/// A tag's length in hex characters: four reads at a glance beside a name, and 65,536 values keep two posters in
/// one room apart in all but a chance collision.
pub const TAG_HEX: usize = 4;

/// **The tag a watcher's or house poster's key reads as in ONE room** (security S2c, finding D8; the owner's R-2):
/// the first four hex of SHA-256 over the room's salt, a newline and the key. Stable within the room, unlinkable
/// across rooms (another salt), and not invertible to an address: the salt is the room's and is never served, so
/// a stranger cannot hash a guessed address and confirm it (B7's fence). Under the open door it says "the same key
/// as before" and nothing more — a name is a claim there by design, the owner's accepted risk (2026-10-13).
pub fn tag_of(salt: &str, key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(salt.as_bytes());
    h.update(b"\n");
    h.update(key.as_bytes());
    let digest = h.finalize();
    digest.iter().take(TAG_HEX / 2).map(|b| format!("{b:02x}")).collect()
}

// chat.rs — AuthorView's watcher arm:
    Watcher { name: Option<String>, you: bool, tag: Option<String> },

// chat.rs — view_for takes the room's salt:
    pub fn view_for(&self, reader: &User, salt: Option<&str>) -> MessageView {
        let you = |user: &String| user == reader.as_str();
        let author = match &self.author {
            Author::Seat { seat, user, name } => AuthorView::Seat { seat: *seat, name: name.clone(), you: you(user) },
            Author::Watcher { user, name } => AuthorView::Watcher { name: name.clone(), you: you(user), tag: salt.map(|s| tag_of(s, user)) },
            Author::Bot { seat, name } => AuthorView::Bot { seat: *seat, name: name.clone() },
        };
        MessageView { seq: self.seq, created_at: self.created_at, author, text: self.text.clone(), deleted: self.deleted }
    }
```

```rust
// games.rs — beside `SeedPolicy`:
/// The chat salt every `Diagnostic` state uses (decision D-5): the fixtures', the corpus walk's, the pool's and
/// `init_diagnostic()`'s, so a fixture's tags are the same bytes on every host. 32 hex, as a production salt is.
pub const FIXED_CHAT_SALT: &str = "4c5f8e2a9d1b7c3e6f0a2b4d8e1c3a5f";

impl Games {
    /// **A new chat document's salt** (security S2c, finding D8; decision D-5): 16 bytes of OS entropy as 32 hex
    /// under `SeedPolicy::SoloOnly` — every production constructor — and [`FIXED_CHAT_SALT`] under `Diagnostic`.
    /// The seed rule's shape, deliberately: the policy already says "this state is a fixture", and a second switch
    /// for the salt would be one more thing every constructor on every host had to set.
    pub fn chat_salt(&self) -> String {
        match self.seed_policy {
            SeedPolicy::Diagnostic => FIXED_CHAT_SALT.to_string(),
            SeedPolicy::SoloOnly => format!("{:016x}{:016x}", crate::tables::draw_seed(), crate::tables::draw_seed()),
        }
    }
}
```

```rust
// tables.rs — `append_locked`, the ONE site both `chat_append` (line 1084 today) and `house_append` (line 1112) reach,
// salts the document on its first append and returns the salt beside the message; the two writers gain `*_salted`
// forms the POST replies call, and keep their signatures as thin wrappers so the ~15 test call sites do not move
// (plan review L1-F3; AD-4):
    /// +1 per append and per delete, so `rev - last` moves exactly when a held
    /// message was deleted. The counts come back beside the message for the
    /// caller's cache.
    fn append_locked(&self, at: ChatAt<'_>, author: Author, text: String) -> Result<(Message, ChatCounts, Option<String>), GameError> {
        let (mut doc, version) = self.read_chat(at)?;
        doc.id = at.key().to_string();
        if doc.salt.is_none() {
            // **The first append salts the room** (security S2c, D8; decision D-5) — a read never writes, so a
            // document from before this cycle carries `None` until somebody posts.
            doc.salt = Some(self.chat_salt());
        }
        let message = Message { seq: doc.last + 1, created_at: (self.clock)(), author, text, deleted: false };
        doc.last = message.seq;
        doc.rev += 1;
        doc.messages.push(message.clone());
        doc.trim(self.chat.limits.keep);
        self.write_chat(at, &doc, version)?;
        Ok((message, doc.counts(), doc.salt.clone()))
    }

    /// **The one writer** (R-6.4.15 (b)): the text rule, the table's own lock,
    // …
    /// `a_chat_bump_precedes_no_write`. A bump ahead of its write sends every
    /// browser at the table to fetch a message that has not landed, and the
    /// message that then lands is announced to nobody.
    pub fn chat_append(&self, id: &str, author: Author, text: &str) -> Result<Message, GameError> {
        self.chat_append_salted(id, author, text).map(|(message, _)| message)
    }

    /// [`Self::chat_append`] and the document's salt beside the message — what the POST's reply needs to draw the
    /// poster's own tag as a later page will (security S2c, D8): the salt is given by this very append on a
    /// document's first message, so the reply cannot read it off anything the caller holds.
    pub fn chat_append_salted(
        &self,
        id: &str,
        author: Author,
        text: &str,
    ) -> Result<(Message, Option<String>), GameError> {
        let text = check_text(text, &self.chat.limits).map_err(GameError::BadRequest)?;
        self.require_table(id)?;
        self.with_table_lock(id, || {
            let (table, _) = self
                .read_table(id)?
                .ok_or_else(|| GameError::NotFound(format!("no game named \"{id}\"", id = crate::echo::echo(id))))?;
            let (message, counts, salt) = self.append_locked(ChatAt::Table { id, players: table.players }, author, text)?;
            // `message.seq` IS the document's `last`: the trim drops the oldest and
            // leaves the number alone, which is the whole of R-6.4.9.
            self.chat_counts.lock().expect("games lock").insert(id.to_string(), counts);
            // AFTER the write, never before (the gated cell).
            self.signal.bump_table(id);
            Ok((message, salt))
        })
    }

    // …
    /// seat author a refusal here: the house has no seats, and the shape rule
    /// says so rather than this function restating it.
    pub fn house_append(&self, author: Author, text: &str) -> Result<Message, GameError> {
        self.house_append_salted(author, text).map(|(message, _)| message)
    }

    /// [`Self::house_append`] and the document's salt (see [`Self::chat_append_salted`]).
    pub fn house_append_salted(&self, author: Author, text: &str) -> Result<(Message, Option<String>), GameError> {
        let text = check_text(text, &self.chat.limits).map_err(GameError::BadRequest)?;
        // `enter`, poisoned or not: the house's lock guards no data.
        let _serialized = enter(&self.house_lock);
        let (message, _, salt) = self.append_locked(ChatAt::House, author, text)?;
        self.signal.bump_house_chat();
        Ok((message, salt))
    }
// chat.rs — ChatDoc::page_for (the one read that builds views) hands its own salt through:
        self.messages.iter().map(|m| m.view_for(reader, self.salt.as_deref())).collect()
// api.rs — both POST replies take the salt the append gave or kept (the poster's own tag, as the next page draws it):
    let (message, salt) = games.chat_append_salted(id, author, &text)?;      // house: games.house_append_salted(author, &text)?
    Ok(message.view_for(user, salt.as_deref()))
```

(`SeedPolicy` is `Games.seed_policy` — `games.rs` line 421; if the field is `pub(crate)` the method reads it directly.)

```ts
// web/src/types.ts — the watcher arm:
  | { kind: "watcher"; name: string | null; you: boolean; tag: string | null }
```

- [ ] **Step 4: Run the cells green.** `cargo test -p yokohama-web --lib -- chat:: games:: tables::` — expected: green,
  the five new cells, and the reply cell, included; every `view_for` call site compiles with the salt argument.

- [ ] **Step 5: Regenerate the corpus ONCE, and READ the diff.**
  `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then `git diff --stat web/src/wire` and
  `git diff web/src/wire`. Expected: only fixtures with a watcher author change, and every hunk adds exactly one
  `"tag": "<4 hex>"` line per watcher author (`chat.json`, `chat_post_watcher.json`, `house_chat.json`,
  `house_chat_post.json`, `chat_after.json` and `chat_deleted.json`/`house_chat_delete.json` if they carry a
  watcher); no other field moves. Then `cargo test -p yokohama-web --test wire_fixtures` WITHOUT the flag — clean
  (the flagged run reports 4 failed on purpose: those cells assert the flag is unset). The report names every file
  the diff touched.

- [ ] **Step 6: The pool replays the corpus with the same tags.** `cd worker && npm run build:module:debug && npm
  test` — expected: both projects green (the module's `init_diagnostic` is `Diagnostic`, so its first append salts
  with the constant and its tags equal the corpus's). `cd web && npm run typecheck && npx vitest run` — expected:
  the typecheck green once `web/src/wire/contract.ts:740` reads
  `const CHAT_AUTHOR_WATCHER = ["kind", "name", "you", "tag"] as const;` — the key list `contract.ts:793` checks
  against `keyof` the arm fails TS4104 without it, and `contract.test.ts`'s "ChatPage, ChatMessage and each author by
  its kind" fails on the list (both sides are sorted, so the order is free); vitest green. On a fresh export
  `npm run build:wasm` runs first (`typecheck-worker.mjs` refuses without `src/local/wasm/yokohama_wasm.d.ts`).

- [ ] **Step 7: The suites, then commit** (the corpus in the same commit as the code that moved it, as every
  regeneration is).

```bash
git add crates/yokohama-web/src/chat.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/api.rs web/src/types.ts web/src/wire
git commit -m "web: watcher and house authors carry a per-room tag — four hex of SHA-256 over a document salt and the key, the salt given on first append, fixed under the diagnostic seed policy; the corpus regenerated once (security S2c, D8)"
```

**Named mutants:** M8 `tag_of` ignores the salt → the tag cell red at "a different room"; M9 the append never fills
the salt → the tables cell red (every tag `None`) and the corpus diff empty; M9a the reply passes `None` → the
reply cell red (plan review L1-F3); M10 `view_for` tags seat authors →
the view cell's seat row red (the `matches!` on `Seat` has no `tag`, so the mutant is a compile error — the reviewer
records it as structurally impossible, which is the point of the enum shape); M11 `chat_salt` returns the constant
under `SoloOnly` → the games cell red at "entropy, not a constant"; M12 `TAG_HEX` 12 → the tag cell red at the
length; M13 the salt serialized into `MessageView` → the serde cell red.

---

### Task 5: The client — a 429 ends a signal, the lobby draws an ended signal, watcher and house authors show their tag — Opus (R-2, R-4; decision D-2; Review Focus 1)

**Files:**
- Modify: `web/src/signal.ts:479` (the stream-open refusal arm; the re-read arm at 316 stays, AD-6),
  `web/src/Lobby.tsx:387` (`usePolled`'s call
  gains `onEnd`; the sentence lands in an `ended` state beside `error` — line 266 — because `take` (every
  listing that lands, the opening read among them) and `copyLink` clear `error`, and an end that arrived before the
  opening listings would be wiped; the foot strip draws `error ?? ended`), `web/src/Chat.tsx:470-474` (the
  watcher label), `web/src/types.ts` (done in Task 4).
- Test: `web/src/signal.test.ts` (one cell beside line 230's), `web/src/Lobby.test.tsx` (two cells),
  `web/src/App.test.tsx` (one cell, after "lands on the lobby with the sentence when the hand-over's table refuses"),
  `web/src/Chat.test.tsx` (the authors cell at line 82 extended; one new cell).

**Interfaces:**
- Consumes: Task 1's 429 and sentences; Task 4's `tag` on the wire (the corpus fixtures carry it after Task 4's
  regeneration, so `wire.chat.messages[2].author.tag` is a four-hex string in the cells).
- Produces: nothing for later tasks.

- [ ] **Step 1: The failing signal cell** — beside the 404/403 cell (line 230), the same shape:

```ts
  it("ends on a 429 — the stream cap's refusal — with the server's sentence, and never backs off over it", async () => {
    // **The NAS's stream cap and the Worker's socket cap end a signal alike** (security S2c, D3; decision D-2):
    // a 429 at admission is an answer about THIS caller's open streams, and asking again every few seconds would be
    // a retry storm against the very cap. The consumer draws the sentence: the table screens land in the lobby with
    // it (Review Focus 1).
    const ends: End[] = [];
    const sentence = "too many open streams from you at this table — close one and try again";
    const stub = stubApi(() => ({ status: 429, body: { error: sentence } }));
    const { unmount } = renderHook(() => useSignal("wire", true, () => {}, (e) => ends.push(e)));
    await settle();
    expect(ends).toEqual([{ reason: "refused", sentence }]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
    });
    expect(stub.matching("GET", "/stream")).toHaveLength(1);
    unmount();
  });
```

And in `web/src/App.test.tsx`, after "lands on the lobby with the sentence when the hand-over's table refuses", the
landing Review Focus 1 now names (plan review L2-F1; AD-5: the behaviour is the code's own, so the cell pins it and
no code changes for it):

```ts
  it("lands on the lobby with the cap's sentence when the table's own stream is refused 429 (Review Focus 1, as the code does it)", async () => {
    // **Every table consumer leaves on ANY `End`** (`App`'s `toLobby(e.sentence)`, `OpenTable`'s and `Dealing`'s
    // `onLeave`): a 429 at admission ends as `refused` and the player lands in the lobby with the server's sentence,
    // as a 403 does and as the Worker's `4429` close does (an `end` frame, `gone`). The table page does not stay.
    window.location.hash = "#/t/wire";
    const sentence = "too many open streams from you at this table — close one and try again";
    vi.useFakeTimers();
    try {
      app((call) => {
        if (call.path.endsWith("/api/games/wire/stream")) return { status: 429, body: { error: sentence } };
        if (call.path.endsWith("/table")) return wire.tableOpen;
        return undefined;
      });
      await until(() => screen.queryByRole("heading", { name: "Lobby" }) !== null, "the lobby was drawn");
      expect(screen.getByText(sentence)).toBeInTheDocument();
      expect(window.location.hash).toBe("#/lobby");
    } finally {
      vi.useRealTimers();
    }
  });
```

- [ ] **Step 2: Run it red.** `cd web && npx vitest run src/signal.test.ts -t "ends on a 429"` — expected: FAIL
  (`ends` is empty; the stub was asked more than once).

- [ ] **Step 3: The open arm.** In `signal.ts`, the stream-open site (479 today) reads:

```ts
        if (e instanceof ApiError && (e.status === 404 || e.status === 403 || e.status === 429 || isSessionExpired(e))) {
```

with one sentence added to the comment above it: "— and a 429, the stream cap's refusal at admission
(security S2c, D3): an answer about this caller's open streams, which asking again cannot change; the NAS answers it
on `/stream` and `/api/stream`, the Worker's lobby ends on a frame instead (`End`), and both reach the consumer's
`onEnd` as `refused`."

The arm's neighbours state their count of the ending refusals, and they move with it (plan review L2-F5): `End`'s
doc ("None of the three changes by being asked again") and the open arm's own "The THREE refusals that END this"
read four, as measured below; `endSentence`'s doc, "The two refusals that END a signal", reads four likewise; the
re-read arm's comment at 316, whose arm does not change (AD-6), keeps its three:

```ts
/** Why a signal STOPPED, as opposed to failing and backing off.
 *
 *  The server said this stream is over (`deleted`), the resource is gone (404),
 *  it is not ours to read (403) or the caller's open streams are at the cap (429,
 *  security S2c). None of the four changes by being asked again, while the screen goes on saying "reconnecting…" about something that
 *  is not coming back.
 *
 *  **The socket arm reads a refusal at ADMISSION as `gone` whatever its
// …
        throw new Error("the stream closed");
      } catch (e) {
        if (stopped || mine.signal.aborted) return;
        // **The FOUR refusals that END this rather than slowing it down** (404, 403, the edge's expired session, and the stream cap's 429). See
        // `End`; nothing is scheduled after any of these lines.
        //
        // **The third is the EDGE's, and it was the review's Important 2**
```

- [ ] **Step 4: Run it green; then the file's suite.** `npx vitest run src/signal.test.ts`, and
  `npx vitest run src/App.test.tsx -t "refused 429"`.

- [ ] **Step 5: The failing lobby cells.** In `Lobby.test.tsx`, beside "says so when you sit at no table" (line 363),
  two cells under fake timers, as the file's cells run (`vi.useFakeTimers(); try { … } finally { vi.useRealTimers(); }`,
  `until(...)` in place of `findByText`): the ended signal, and the race — the opening listings held behind a gate
  until after the end, the sentence surviving their landing (plan review L2-F2, L2-R6):

```ts
  it("draws an ended signal's sentence in its error strip and asks nothing more (the lobby's refusal, AD-4)", async () => {
    // Stub every listing as the file's cells do, and the house signal — the request the lobby's `usePolled` source
    // opens (`GET /api/stream` on this host) — as a 429 with the cap's sentence; the sentence is the server's.
    const sentence = "too many open lobby streams from you — close one and try again";
    vi.useFakeTimers();
    try {
      const { stub } = show((call) => (call.path.endsWith("/api/stream") ? { status: 429, body: { error: sentence } } : undefined));
      await until(() => screen.queryByText(sentence) !== null, "the sentence was drawn");
      await act(async () => {
        await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]! * 4);
      });
      // The stream was opened once and never again; the listings were fetched once at mount.
      expect(stub.matching("GET", "/api/stream")).toHaveLength(1);
      expect(screen.getByText(sentence)).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it("keeps the ended signal's sentence when the opening listings land AFTER the end (take clears the strip)", async () => {
    const sentence = "too many open lobby streams from you — close one and try again";
    vi.useFakeTimers();
    try {
      const hold = gate<void>();
      const { stub } = show((call) => {
        if (call.path.endsWith("/api/stream")) return { status: 429, body: { error: sentence } };
        if (isGamesList(call)) return hold.promise.then(() => lists(call)!) as unknown as Reply;
        return undefined;
      });
      await until(() => stub.matching("GET", "/api/stream").length === 1, "the stream was refused");
      await flush();
      await until(() => screen.queryByText(sentence) !== null, "the sentence was drawn");
      hold.resolve();
      await flush();
      await until(() => within(section("Your tables")).queryByRole("link", { name: "wire" }) !== null, "the listings landed");
      expect(screen.queryByText(sentence)).not.toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });
```

The lobby's `usePolled` source IS the house stream (`Lobby.tsx:391`, `stream: "/api/stream"`), so the plan's premise
holds and the cells drive `GET /api/stream` through the file's `show(handler)`; `until`, `gate`, `isGamesList`,
`lists`, `flush` and `section` are the file's own helpers (plan review L2-R6).

- [ ] **Step 6: Run them red**, then wire `onEnd`:

```tsx
// Lobby.tsx — the usePolled call (line 387 today) gains a fourth argument:
  const link = usePolled<Reading>(
    source,
    active,
    onReading,
    // **An ended signal is said, once, where the lobby says its refusals** (security S2c, D3; S2b's AD-4): the
    // stream cap's 429 on the NAS, or the socket cap's `end` frame on the Worker, carries a sentence the player can
    // act on; before this the lobby read "live" with a dead stream. `usePolled` opens nothing after an end.
    useCallback((e: End) => setEnded(e.sentence ?? "the lobby's live updates have stopped — reload to reconnect"), []),
  );
```

(`source`, `active` and `onReading` stand for the three arguments the call passes today — keep them as they are;
`End` is imported from `./signal` — `import { usePolled, type End } from "./signal";` at line 61; `ended`/`setEnded`
is a new `useState<string | null>(null)` beside `error` at line 266, its doc saying why it is kept apart; before the
component's `return (`, `const foot = error ?? ended;`, and the foot strip at line 774 draws
`{foot && !(serverUp === false && foot === UNREACHED) && <Refusal sentence={foot} />}` — the three edits beside the call
as the plan review measured them, L2-F2, below.)

```tsx
import { Refusal } from "./Refusal";
import { ReleaseNotice } from "./ReleaseNotice";
import { hashOf, navigate } from "./route";
import { usePolled, type End } from "./signal";
import type { ReadinessStatus } from "./swRegister";
import { mayAbandon, rowAction, rowMeta, seatChips, type Section } from "./tables";
import type { GamesList, TableView } from "./types";
// …
  /** The lobby's own strip, at the foot: a reading where every scope was
   *  refused, the pager's, a copy the browser refused, and Abandon's. */
  const [error, setError] = useState<string | null>(null);
  /** Why the house signal STOPPED, once it has (security S2c, D3; AD-4): kept apart from `error` because `take` —
   *  every listing that lands, the opening read among them — clears the strip, and a signal that ended before the
   *  opening read landed would otherwise have its sentence wiped and read "live" over a dead stream again. */
  const [ended, setEnded] = useState<string | null>(null);
// …
    </a>
  );

  /** The foot strip: the last refusal, else why the signal stopped. */
  const foot = error ?? ended;
  return (
    <div className="lobby">
      <LobbyBand crumbs={["Lobby"]} corner={corner ?? (me === null ? toDevice : null)} />
// …
            (one refused listing is said in its own section, `strip`), the
            pager's, a copy the browser refused, and Abandon's. **Not a second "Can't reach the server"
            under state C's card**, which says it already. */}
        {foot && !(serverUp === false && foot === UNREACHED) && <Refusal sentence={foot} />}
      </div>
    </div>
  );
```

- [ ] **Step 7: Run them green; then the file's suite.**

- [ ] **Step 8: The tag's label.** In `Chat.test.tsx`, the authors cell (line 82) gains, after the watcher's rows:

```ts
    // **The watcher's tag, beside the name** (security S2c, D8; decision D-5): four hex the server derived from the
    // room's salt and the poster's key, so two posters with one chosen name read apart and one poster reads the same
    // across their messages. The separator is the marker's, as the "spectating" needle is.
    if (green.author.tag === null) throw new Error("the corpus watcher carries a tag since S2c's Task 4");
    expect(green.author.tag).toMatch(/^[0-9a-f]{4}$/);
    expect(rows[2]!).toHaveTextContent(`· ${green.author.tag}`);
    // A seat carries no tag field and draws none.
    expect(rows[0]!.textContent).not.toMatch(/· [0-9a-f]{4}/);
```

and one new cell:

```ts
  it("draws the reader's own tag as 'you · tag', a nameless watcher's as 'someone · tag', and no tag where the room has none yet", async () => {
    open();
    const [red, blue, green] = wire.chat.messages;
    if (red?.author.kind !== "seat" || blue?.author.kind !== "seat" || green?.author.kind !== "watcher") {
      throw new Error("the corpus page is red's, blue's, then the watcher's");
    }
    const own: ChatPage = {
      ...wire.chat,
      messages: [
        { ...green, author: { ...green.author, name: null, you: true, tag: "a1b2" } },
        { ...green, seq: green.seq + 1, author: { ...green.author, name: null, you: false, tag: "c3d4" } },
        { ...green, seq: green.seq + 2, author: { ...green.author, tag: null } },
      ],
    };
    chatStub((call) => (call.method === "GET" ? own : undefined));
    render(<Chat scope={TABLE} count={own.last} colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    expect(rows.map((r) => r.querySelector(".chat-who")!.textContent)).toEqual([
      "you · a1b2 · spectating",
      "someone · c3d4 · spectating",
      `${green.author.name} · spectating`,
    ]);
  });
```

(The exact `textContent` strings depend on how the label composes its spans; the implementer reads the rendered
text in the red run and pins the composition the code below produces — the three facts to pin are: the tag follows
the name or its fallback with ` · `, a `null` tag draws nothing, and the "spectating" marker follows.)

- [ ] **Step 9: Run them red, then the label:**

```tsx
// Chat.tsx — the watcher arm of `label` (lines 470–474 today):
    return (
      <span className={`chat-who${scope.kind === "table" ? " watching" : ""}`}>
        {a.name ?? (a.you ? "you" : "someone")}
        {/* **The tag** (security S2c, D8): the server's four hex for this poster in this room, drawn where a
            name's provenance would be — "you · a1b2" tells the reader their own tag. `null` before the room's
            first message since the tag landed (an old document): nothing drawn, nothing claimed. */}
        {a.tag !== null && <span className="meta"> &middot; {a.tag}</span>}
        {scope.kind === "table" && <span className="meta"> &middot; spectating</span>}
      </span>
    );
```

- [ ] **Step 10: Run them green; `npm run typecheck && npx vitest run`; then commit.**

```bash
git add web/src/signal.ts web/src/signal.test.ts web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/App.test.tsx web/src/Chat.tsx web/src/Chat.test.tsx
git commit -m "web client: a 429 ends a signal with the server's sentence (the stream cap; the lobby draws its ended signal — AD-4 on both hosts); watcher and house authors show their room tag (security S2c, D3 and D8)"
```

**Named mutants:** M33 the 429 arm dropped from the stream site (line 479) → the signal cell red (the stub asked
again); M34 retired with the re-read arm (AD-6): a 429 at the re-read keeps the stream, which the file's existing "says
reconnecting when a woken re-read FAILS, keeps the stream" cell already pins; M35 the lobby's `onEnd` not passed → the
lobby cells red; M35a `onEnd` writes `setError` → the race cell red (plan review L2-F2); M36 the tag drawn for a
`null` → the new chat cell's third row red; M37 the tag drawn on seats → the authors cell's seat row red.

---

### Task 6: The withheld `why` runs no search — the CLI's `locate`, the layer's record-built answer, the withheld-mismatch sentence retired — Opus (R-2's neighbour, the explain surface's first rule; decisions D-6, D-7)

**Files:**
- Modify: `crates/yokohama-cli/src/why.rs:392-470` (`account_parts` split into `locate` and `account_from`; one cell),
  `crates/yokohama-web/src/explain.rs` (`Provenance::located`, `Why::withheld_from`; `MISMATCH_WITHHELD` at 356 and
  `Why::refused_at` at 565 REMOVED, with the module doc's sentences at 23–24, 351–357, 563, 607–608, 684–686 and the
  cells at 1235–1270 rewritten), `crates/yokohama-web/src/api.rs:1701-1750` (`why`'s flow; the doc at 1687–1699),
  `crates/yokohama-web/src/lib.rs` (the two cells at 6966 and 7044 rewritten, with their doc comments at 6905 and
  6979–6980); and the retired names' six other sites: `explain.rs:556-557` (`Why::refused`'s doc, an intra-doc link to
  `refused_at`) and `:866-867` (`SuggestRefusal::message`'s doc, a link to `MISMATCH_WITHHELD`) — both links break
  when the items go; `web/src/types.ts:897` (`RefusalRow`'s doc: "the one exception among `why`'s twelve … withheld
  carries the server's `MISMATCH_WITHHELD`"); `docs/web-lane.md:166` (the explanation-surface bullet's
  `refused_at`/`MISMATCH_WITHHELD` passage and "The other eleven refusals read the same at either visibility", false
  now that a withheld requester reaches no search refusal at all) and `:755` (the `/why` row's "a `mismatch` refusal
  served while the account is withheld names no move"). The exit check:
  `grep -rn 'MISMATCH_WITHHELD\|refused_at\b' crates web/src docs/web-lane.md CLAUDE.md` reads nothing outside
  `docs/plans/`.
- Test: the CLI cell; the explain cells; the two lib cells.

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: `yokohama_cli::why::{Located, locate, account_from}`; `explain::Why::withheld_from`.

- [ ] **Step 1: The failing CLI cell.** In `why.rs`'s test module:

```rust
    /// **`account_parts` is `locate` then `account_from`, and the halves meet exactly** (security S2c, decision
    /// D-6 — the layer reads the requester's visibility between them, so a withheld answer never reaches
    /// `decide`): on the module's vs-bot fixture the located index, seat, recorded move and versions equal the
    /// account's; a record with no bot move refuses from `locate` with `NoBotMoveYet`; an out-of-range index with
    /// `OutOfRange` — every pre-search refusal is `locate`'s. `locate` is pure replay: this cell holds no bot seats
    /// at all for its first call.
    #[test]
    fn locate_is_the_pre_search_half_of_account_parts_and_the_halves_agree() {
        // The fixture is `the_account_parts_are_what_the_screen_is_made_of`'s own three lines:
        let mut r = bot_repl(4, PROVISIONAL_DEFAULT_BOT);
        let rules = Rules::standard();
        let branching = play_to_a_branching_bot_move(&mut r);
        let (session, bots) = (&r.session, &r.bots);
        let none_yet = human_repl(4);
        assert!(matches!(locate(TrackingMode::Simulation, none_yet.session.record(), &rules, None), Err(Refusal::NoBotMoveYet)));
        let located = locate(TrackingMode::Simulation, session.record(), &rules, None).expect("a bot move to locate");
        let account = account_parts(TrackingMode::Simulation, session.record(), &rules, bots, None).expect("an account");
        assert_eq!((located.index, located.seat), (account.reconstruction.index, account.reconstruction.seat));
        assert_eq!(located.recorded, account.mv);
        assert_eq!(located.recorded_version, account.reconstruction.recorded_version);
        assert_eq!(located.current_version, account.reconstruction.current_version);
        assert_eq!(located.contestant.echo(), account.reconstruction.contestant.echo());
        let again = account_from(locate(TrackingMode::Simulation, session.record(), &rules, None).unwrap(), bots, &rules).unwrap();
        assert_eq!(again.reconstruction.decision_seed, account.reconstruction.decision_seed);
        assert_eq!(again.mv, account.mv);
        assert!(matches!(locate(TrackingMode::Simulation, session.record(), &rules, Some(10_000)), Err(Refusal::OutOfRange { .. })));
        assert!(matches!(locate(TrackingMode::Live, session.record(), &rules, None), Err(Refusal::LiveMode)));
        let at = locate(TrackingMode::Simulation, session.record(), &rules, Some(branching)).expect("the branching bot move");
        let RecordedEvent::Move(recorded) = &session.record().events[branching] else { panic!("fixture: a move") };
        let version = PROVISIONAL_DEFAULT_BOT.descriptor().1;
        assert_eq!((at.index, at.seat, at.contestant), (branching, PlayerId::new(1), PROVISIONAL_DEFAULT_BOT));
        assert_eq!((at.current_version, at.recorded_version), (version, Some(version)));
        assert_eq!(&at.recorded, recorded);
    }
```

(`TrackingMode`'s variant names are the CLI's — `grep -n "enum TrackingMode" crates/yokohama-cli/src`; the fixture
setup is the file's own, named in the comment.)

- [ ] **Step 2: Run it red.** `cargo test -p yokohama-cli why::` — expected: a compile error (`locate`,
  `account_from`, `Located` do not exist).

- [ ] **Step 3: The extraction** — the first half of `account_parts` (lines 392 to the `read_seat` match's end)
  moves into `locate`, the second (from `bots.decision_seed` on) into `account_from`; NO line's logic changes.

```rust
/// **Where `why` looks, before any search** (security S2c, decision D-6): the index `arg` names or the last
/// explainable bot move, the state that move was decided in (`state_at(record, n)`), its mover and the mover's
/// contestant, with every pre-search refusal in [`account_parts`]'s order — pure replay, so a caller that stops
/// here has paid nothing a hidden hand could vary. The layer's `why` reads the requester's visibility on this and
/// reaches [`account_from`] only when the account may be served.
pub struct Located {
    pub index: usize,
    pub seat: PlayerId,
    pub state: GameState,
    pub recorded: Move,
    pub contestant: Contestant,
    pub recorded_version: Option<u32>,
    pub current_version: u32,
}

pub fn locate(mode: TrackingMode, record: &GameRecord, rules: &Rules, arg: Option<usize>) -> Result<Located, Refusal> {
    if matches!(mode, TrackingMode::Live) {
        return Err(Refusal::LiveMode);
    }
    let events = record.events.len();
    let index = match arg {
        Some(asked) if asked < events => asked,
        Some(asked) => return Err(Refusal::OutOfRange { asked, events }),
        None => last_explainable_bot_move(record, rules)?,
    };
    let RecordedEvent::Move(recorded) = &record.events[index] else {
        return Err(Refusal::NotAMove { index, kind: event_kind(&record.events[index]) });
    };
    // (the existing comment on `state_at` stays here, verbatim)
    let state = state_at(record, index, rules).map_err(|e| Refusal::Replay { index, message: e.message })?;
    let seat = state.current_player;
    let contestant = match read_seat(record, seat) {
        SeatReading::Explainable(contestant) => contestant,
        SeatReading::Human => return Err(Refusal::HumanSeat { index, seat }),
        SeatReading::Rung(rung) => return Err(Refusal::Rung { index, seat, rung }),
        SeatReading::Diagnostic(why) => return Err(Refusal::Diagnostic { index, seat, why }),
        SeatReading::Unknown { controller, why } => return Err(Refusal::UnknownController { index, seat, controller, why }),
    };
    let (_, current_version, _) = contestant.descriptor();
    let recorded_version = seat_provenance(record, seat).map(|p| p.version);
    Ok(Located { index, seat, state, recorded: recorded.clone(), contestant, recorded_version, current_version })
}

/// The search half: the decision reconstructed from the located state under the pump's own key, gated against the
/// record, the explanation and the forced fact — [`account_parts`]'s second half, verbatim.
pub fn account_from(located: Located, bots: &BotSeats, rules: &Rules) -> Result<WhyAccount, Refusal> {
    let Located { index, seat, state, recorded, contestant, recorded_version, current_version } = located;
    // (the existing comment on THE PUMP'S OWN KEY stays here, verbatim)
    let decision_seed = bots.decision_seed(index);
    let decision = bot::decide(contestant, &state, rules, seat, decision_seed)
        .map_err(|e| Refusal::Reconstruction { index, seat, why: e.to_string() })?;
    let (controller, _, _) = contestant.descriptor();
    if decision.mv != recorded {
        return Err(Refusal::Mismatch(Box::new(Mismatch {
            index,
            seat,
            controller: contestant.echo(),
            reconstructed: format!("{:?}", decision.mv),
            recorded: describe::describe_move(&recorded, &state, rules),
            recorded_version,
            current_version,
        })));
    }
    let explain = decision.explain.ok_or_else(|| Refusal::Unaccounted { index, contestant: controller.clone() })?;
    // (the existing comment on THE FORCED-POSITION FACT stays here, verbatim)
    let forced = get_legal_moves(&state, rules).len() == 1;
    Ok(WhyAccount {
        reconstruction: Reconstruction { contestant, index, seat, decision_seed, recorded_version, current_version, forced },
        mv: recorded,
        explain,
        state,
    })
}

pub fn account_parts(mode: TrackingMode, record: &GameRecord, rules: &Rules, bots: &BotSeats, arg: Option<usize>) -> Result<WhyAccount, Refusal> {
    account_from(locate(mode, record, rules, arg)?, bots, rules)
}
```

(`Contestant` is `Copy` — `arena/stats.rs:150`. `descriptor()` is read twice now, in `locate` for `current_version` and
in `account_from` for `controller`; today it is read once, AFTER `bot::decide`. Both reads are pure, so nothing
observable moves. `WhyAccount.mv` held `decision.mv` today, and it is the located `recorded` now: equal, since the
`decision.mv != recorded` gate has just passed. `account_parts`' four-line doc comment moves down to sit above the
final `account_parts`, or it lands on `pub struct Located`. The two "(the existing comment … stays here, verbatim)"
markers are placeholders the implementer expands, not code. `cargo test -p yokohama-cli` must read every `why` cell green UNCHANGED: the
rendering is byte-identical by construction.)

- [ ] **Step 4: Run the CLI's suite.** `cargo test -p yokohama-cli` — expected: green, the count +1 (the new cell).

- [ ] **Step 5: The failing layer cells.** In `explain.rs`'s test module, the withheld-bytes cell (line 1196) gains
  a last block, and the `refused_at` cells (1235–1270) are replaced:

```rust
        // **And the RECORD alone builds those same bytes** (security S2c, decision D-6): `withheld_from` over
        // `locate`'s reading, with no search run, serializes to the withheld payload `account` produced.
        let located = yokohama_cli::why::locate(repl.session.mode, repl.session.record(), &rules, None).expect("located");
        let from_record = serde_json::to_string(&Why::withheld_from(&located, &rules)).expect("json");
        let from_account = serde_json::to_string(&Why::account(&a, &rules, Visibility::Withheld)).expect("json");
        assert_eq!(from_record, from_account, "the record-built withheld half differs from the account-built one");
```

```rust
    /// **A tampered record is answered WITHHELD while the mover's card is hidden, and refused as a mismatch once
    /// public** (security S2c, decision D-7 — the unit pin of the two halves; the route cells in `lib.rs` catch their order: the old flow
    /// searched first and answered the withheld-mismatch refusal here; the new one never reaches the search, so
    /// the withheld payload is what a withheld requester gets, whatever the record says). The public half keeps
    /// the Mismatch refusal verbatim — the honesty gate fires the moment an account could be served.
    #[test]
    fn a_tampered_record_is_withheld_while_hidden_and_a_mismatch_once_public() {
        // A vs-bot session whose recorded bot move is replaced by another legal move.
        use yokohama_core::{get_legal_moves, state_at, RecordedEvent};
        let (repl, _) = vs_bot(1);
        let rules = Rules::standard();
        let mut tampered = repl.session.record().clone();
        // The bot's last DECISION (a position offering another legal move), swapped for another member of the
        // list, the record cut after it and re-stamped over its own end (as the route cell
        // `a_why_on_a_drifted_record_is_withheld_from_a_requester_who_may_not_see_the_account` builds its drift).
        let k = (0..tampered.events.len()).rev().find(|&k| {
            matches!(tampered.events[k], RecordedEvent::Move(_)) && {
                let at = state_at(&tampered, k, &rules).expect("a prefix");
                at.current_player == PlayerId::new(0) && get_legal_moves(&at, &rules).len() > 1
            }
        }).expect("the bot has decided");
        let before = state_at(&tampered, k, &rules).expect("the position");
        let RecordedEvent::Move(decided) = tampered.events[k].clone() else { unreachable!() };
        let other = get_legal_moves(&before, &rules).into_iter().find(|m| *m != decided).expect("another legal move");
        tampered.events[k] = RecordedEvent::Move(other);
        tampered.events.truncate(k + 1);
        let end = state_at(&tampered, k + 1, &rules).expect("the drifted record replays");
        tampered.stamp_terminal_digest(&end);
        let drifted = yokohama_cli::repl::Repl::new(
            yokohama_cli::session::Session::from_record(tampered, rules.clone()).expect("the drifted record loads"),
        );
        let (mode, tampered, bots) = (drifted.session.mode, drifted.session.record(), drifted.bots());
        let located = yokohama_cli::why::locate(mode, tampered, &rules, Some(k)).expect("located, pre-search");
        let withheld_now = drifted.session.state().clone();
        assert_eq!(visibility_of(&withheld_now, located.seat, &[]), Visibility::Withheld, "premise");
        let w = Why::withheld_from(&located, &rules);
        assert!(w.withheld.is_some() && w.refusal.is_none() && w.ok.is_none(), "withheld, not refused: {}", serde_json::to_string(&w).unwrap());
        assert_eq!(w.withheld.as_ref().unwrap().text, yokohama_cli::describe::describe_move(&located.recorded, &located.state, &rules), "the RECORDED move's text, never the reconstruction's");
        let public = yokohama_cli::why::account_from(located, bots, &rules);
        let Err(refusal) = public else { panic!("the tampered move must not reconstruct") };
        let r = Why::refused(&refusal);
        assert_eq!(r.refusal.as_ref().unwrap().kind, "mismatch");
        assert!(r.refusal.as_ref().unwrap().message.contains(&format!("{decided:?}")), "{}", r.refusal.as_ref().unwrap().message);
    }
```

In `lib.rs`, the two cells at 6966 and 7044 (the route's withheld-mismatch rows) are rewritten to the same two
facts THROUGH THE ROUTE: the SEAT HOLDER's `GET …/why` at the running tampered table (the bot's card hidden from the other seat)
reads `withheld` an object, `refusal` and `ok` null, `withheld.provenance.index == k`, and the swapped move's Debug
nowhere in the body — the cell renamed `a_why_on_a_drifted_record_is_withheld_from_a_requester_who_may_not_see_the_account`,
its old name being false after the change;
the seat holder's at game over reads the mismatch refusal's `kind` and its sentence naming both moves, and a watcher
there reads `withheld`. The first cell also asks, at the same withheld visibility, `?event=<a human move>` and reads
the `human_seat` refusal: a pre-search refusal still reaches a withheld requester (the plan review's AD-14).

- [ ] **Step 6: Run them red.** Compile errors on `withheld_from`; the tampered cell red on "withheld, not refused".

- [ ] **Step 7: The layer.**

```rust
// explain.rs — Provenance from a located reading:
impl Provenance {
    fn located(l: &yokohama_cli::why::Located) -> Self {
        Self {
            contestant: l.contestant.echo(),
            evaluator_version: yokohama_core::game::eval::EVAL_IMPL_VERSION,
            index: l.index,
            seat: l.seat,
            recorded_version: l.recorded_version,
            current_version: l.current_version,
        }
    }
}

// explain.rs — Why, beside `account`:
    /// **The withheld half from the RECORD alone** (security S2c, the explain surface's first rule; decision D-6):
    /// built before any search from what `why::locate` read by replay — the move played, the claim, the sentence —
    /// so a withheld answer costs the requester's clock nothing a hidden hand could vary (the S1 stack review's
    /// timing channel: 1 ms–4.4 s of search, a function of the hand, closed by not running it). The same bytes
    /// [`Self::account`] produces at `Visibility::Withheld`, pinned.
    pub fn withheld_from(l: &yokohama_cli::why::Located, rules: &Rules) -> Self {
        Self {
            ok: None,
            withheld: Some(WhyWithheld {
                provenance: Provenance::located(l),
                text: yokohama_cli::describe::describe_move(&l.recorded, &l.state, rules),
                reason: WITHHELD_REASON,
            }),
            refusal: None,
        }
    }
```

`MISMATCH_WITHHELD` (356) and `refused_at` (565–578) are deleted; every doc sentence that cites them (23–24, 351–357,
563, 607–608, 684–686; `api.rs` 1687–1699) says instead: "a refusal the search finds is reachable only at a public
visibility, since the search runs only there (security S2c)".

```rust
// api.rs — `why`, from the foreign check on:
    if let Some(refused) = explain::foreign_why(&record, games.rules(), q.event) {
        return Ok(refused);
    }
    // **The visibility is read BEFORE any search** (security S2c, the explain surface's first rule; D-6, D-7):
    // `locate` is pure replay — the index, the mover, its contestant and every pre-search refusal — and costs
    // nothing a hidden hand could vary; a withheld answer is built from it and never reaches `decide`.
    let located = match yokohama_cli::why::locate(mode, &record, games.rules(), q.event) {
        Ok(l) => l,
        Err(refusal) => return Ok(explain::Why::refused(&refusal)),
    };
    let visibility = explain::visibility_of(&now, located.seat, &held);
    if visibility == explain::Visibility::Withheld {
        return Ok(explain::Why::withheld_from(&located, games.rules()));
    }
    Ok(match yokohama_cli::why::account_from(located, &bots, games.rules()) {
        Ok(account) => explain::Why::account(&account, games.rules(), visibility),
        Err(refusal) => explain::Why::refused(&refusal),
    })
```

- [ ] **Step 8: Run the layer's cells green**, then `cargo test -p yokohama-web --test wire_fixtures` WITHOUT the
  flag — expected: clean (`why_withheld.json` is the same bytes: the pin at Step 5 says so; `why.json` unmoved).

- [ ] **Step 9: The suites (the CLI's included), then commit** — one commit for the CLI's extraction, one for the
  layer, so the fence diff names the CLI's change by itself.

```bash
git add crates/yokohama-cli/src/why.rs
git commit -m "cli: why::account_parts split into locate (pure replay, every pre-search refusal) and account_from (the search); behaviour-preserving, the why cells byte-identical (security S2c, D-6)"
git add crates/yokohama-web/src/explain.rs crates/yokohama-web/src/api.rs crates/yokohama-web/src/lib.rs web/src/types.ts docs/web-lane.md
git commit -m "web: the withheld why is built from the record before any search — the visibility read first, the withheld-mismatch sentence retired, a tampered record withheld while hidden and a mismatch once public (security S2c, the timing channel closed)"
```

**Named mutants:** M38 `api::why` calling `account_from` before the visibility read → the two `lib.rs` route cells
red (`…is_withheld_from_a_requester_who_may_not_see_the_account` on "withheld, not refused"; the game-over cell on the
watcher's `withheld`) — the explain cell calls the halves directly and cannot see their order; M39 `withheld_from`'s text from a reconstruction (unreachable without a search; the
reviewer records it as structurally impossible); M40 `locate` with `arg = None` picking the LAST move rather than
the last explainable bot move → the CLI's `why` cells red; M41 `account_from` skipping the mismatch gate → the CLI's
mismatch cells red; M42 `visibility_of`'s seat replaced by `now.current_player` → the explain visibility cells red.

### Task 7: The residuals — `claimRedacted` by equality, the `g` flag, the `BROWSER_UA` recipes, the five comment survivors — Sonnet implementer, Opus reviewer (R-4; decisions D-13, D-14)

**Files:**
- Modify: `worker/src/archive.ts:236-246` (`claimRedacted` and its call at line 198), `worker/src/redact.ts:30-44`
  (`SEGMENT`, `redactKey`), `worker/src/redact.unit.test.ts`, `worker/test/open/door.test.ts` (two cells
  beside the `init-refused` claim cell); `worker/RUNBOOK.md` (§3, §4 and §5's seven recipe `curl` lines; the
  variable's definition moved from §7); `crates/yokohama-web/tests/wire_fixtures.rs:2996-3000`,
  `worker/src/index.ts:84` and `:114-140`, `web/src/wire/contract.ts:447`, `web/scripts/headers.mjs:177`.
- Test: the Worker cells below; `cd web && npx vitest run -c vitest.node.config.ts scripts/headers.node.test.ts` for
  the headers message (`web/` has no `test:scripts`); the documents by the reviewer's read.

**Interfaces:** none consumed; nothing produced for later tasks.

- [ ] **Step 1: The two failing Worker cells.** In `worker/src/redact.unit.test.ts`:

```ts
  it("redacts EVERY browser segment in a message, not the first alone (the g flag)", () => {
    const two = 'copy browser/alice-0123456789ab/x.json to browser/bob-fedcba987654/y.json failed';
    expect(redactKey(two)).toBe("copy browser/0123456789ab/x.json to browser/fedcba987654/y.json failed");
    expect(redactKey("test/browser/alice-0123456789ab/x.json")).toBe("test/browser/0123456789ab/x.json");
    expect(redactKey("rev7/over/abc.json")).toBe("rev7/over/abc.json");
  });
```

In `worker/test/open/door.test.ts`, beside the `init-refused` claim cell at line 335 ("logs a claim the key grammar
refuses by its digest, and no line spells the claim" — the `open` project; `archive.test.ts` has no such cell), with
the configuration refusal driven as `table.test.ts:530-548`'s `init-module` arm drives it
(`vi.spyOn(configModule, "moduleConfig").mockReturnValue(bad)`, `lobby_chat: "maybe"`). The Archive object caches its
module (`this.instance ??= …`), so the configuration row uses a NAME no other cell uses (`"cfg refused"`; the claim
`ben` broke the neighbour "an address claimed at the open door … a name keeps every door"). Beside it, a second cell:
"logs a refused claim the echo cut or escaped by its digest too" (`a@b@` + 80 `x`, `a@b@"quoted`,
`a@b@back\slash`; each persisted line contains `"<digest>"` and not the claim's first six characters) — as the
plan review measured them (L2-R3, L2-F3):

```ts
  // Security S2c, R-4 (S2b's re-review parked it): `claimRedacted` rewrites the quoted span that IS the claim and no
  // other, so a refusal whose sentence quotes a VALUE that is not the claim (a configuration variable) is logged
  // whole. Drive the archive object as the cell above does, with the module's configuration refused as
  // `table.test.ts`'s `init-module` arm refuses it. Mutant: the first quoted span rewritten whatever it holds.
  it("rewrites only the quoted span that IS the claim, and leaves a configuration refusal's value as it is", async () => {
    // **A claim no other cell uses**: the Archive object caches the module it built (`this.instance ??=`), so the
    // refused configuration stays in THIS name's object for the rest of the file.
    const claim = "cfg refused";
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    const bad = JSON.stringify({ identity: "open", archive: "off", caller: null, lobby_chat: "maybe", chat_keep: null, chat_text: null });
    const spy = vi.spyOn(configModule, "moduleConfig").mockReturnValue(bad);
    try {
      const { lines } = await logsDuring(() => through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save }));
      const digest = await callerDigest(claim);
      const refused = lines.filter((l) => (l as { event?: unknown }).event === "init-refused") as Array<{ id: string; error: string }>;
      expect(refused.map((l) => l.id)).toEqual([ARCHIVE_PREFIX + digest]);
      expect(refused[0]!.error, "the value is the configuration's, kept verbatim").toBe('YOKOHAMA_WEB_LOBBY_CHAT must be on or off \u2014 got "maybe"');
      expect(refused[0]!.error, "and no digest stands in its place").not.toContain(digest);
    } finally {
      spy.mockRestore();
    }
  });

  // Security S2c, R-4: equality with the raw claim alone would let a refused claim the echo CUT (over 64 characters)
  // or ESCAPED (a quote, a backslash) into the persisted line, so the span is compared as the echo wrote it, read
  // back. Mutant: `claimRedacted` back to `sentence.includes(`"${claim}"`)`.
  it("logs a refused claim the echo cut or escaped by its digest too", async () => {
    const save = JSON.stringify(ARCHIVE_WALK[0]!.body);
    for (const claim of ["a@b@" + "x".repeat(80), 'a@b@"quoted', "a@b@back\\slash"]) {
      const { lines } = await logsDuring(() => through("/api/archive", { method: "POST", claim, headers: JSON_TYPE, body: save }));
      const digest = await callerDigest(claim);
      const refused = lines.filter((l) => (l as { event?: unknown }).event === "init-refused") as Array<{ error: string }>;
      expect(refused, claim.slice(0, 12)).toHaveLength(1);
      expect(refused[0]!.error, "the claim is named by its digest").toContain(`"${digest}"`);
      expect(refused[0]!.error, "and not spelled").not.toContain(claim.slice(0, 6));
    }
  });
```

- [ ] **Step 2: Run them red.** `cd worker && npx vitest run -c vitest.scripts.config.ts src/redact.unit.test.ts` (the
  default config's two projects include `test/**` only; the two-segment row fails: today the second segment is
  untouched) and `npm test -- door` (row (2) fails: today the first quoted span is rewritten whatever it holds).

- [ ] **Step 3: The two changes.**

```ts
// worker/src/redact.ts — the g flag, used by `replace` alone (a `g` regex keeps `lastIndex` across `test`/`exec`
// calls; this module calls neither on it — the comment says so):
/** … Matched wherever `browser/` starts the key, follows a root (`test/`), or follows any character no key segment
 *  holds — a quote or a space, where R2's message spells the key inside a sentence — and EVERY such segment in a
 *  message, since one R2 sentence can name two keys (security S2c, the residual S2b's re-review parked). The `g`
 *  flag is safe here because only `replace` reads this expression (never `test` or `exec`, which would carry
 *  `lastIndex` between calls). */
const SEGMENT = /(^|[^A-Za-z0-9._-])browser\/[^/]*-([0-9a-f]{12})\//g;
```

```ts
// worker/src/archive.ts — claimRedacted by equality with the claim AS THE ECHO WRITES IT (QUOTED keeps its place
// above, gains the `g` flag, and stays used):
/** A quoted span as the echo rule writes one — escapes `\"` and `\\` inside — of at most 1,024 characters: the
 *  layer's echo repeats at most 64 characters (`echo::ECHO_MAX`), each escaped as `{:?}` escapes it to at most ten,
 *  `\u{10ffff}`, and `…` after a cut. Global, and read by `replace` alone. */
const QUOTED = /"(?:[^"\\]|\\.){0,1024}"/g;

/** A span's text as `{:?}` wrote it, read back: `\u{…}`, `\n`, `\r`, `\t`, `\0`, and `\` before any other character. */
function unescaped(inner: string): string {
  return inner.replace(/\\(?:u\{([0-9a-f]+)\}|(.))/g, (_, hex: string | undefined, c: string) =>
    hex !== undefined ? String.fromCodePoint(parseInt(hex, 16)) : ({ n: "\n", r: "\r", t: "\t", "0": "\0" } as Record<string, string>)[c] ?? c,
  );
}

/** The claim as the layer's echo shows it, unescaped: its first 64 characters, and `…` after a cut (`echo::echo`). */
function echoed(claim: string): string {
  const chars = Array.from(claim);
  return chars.length > 64 ? chars.slice(0, 64).join("") + "…" : claim;
}

/** `sentence` with every quoted span EQUAL to the caller's claim as the echo rule writes it replaced by `digest`,
 *  quoted (security S2c, R-4); a span that is anything else — a configuration refusal's value, `LOBBY_CHAT "maybe"` —
 *  is left as it is. The span is read back through the escapes and compared with the claim cut as the echo cuts it,
 *  so a claim over 64 characters, or one carrying a quote or a backslash, is redacted as a plain one is; plain
 *  equality on the raw claim left those in the persisted line (S2b's D23; the plan review, L2-F3). */
function claimRedacted(sentence: string, claim: string, digest: string): string {
  return sentence.replace(QUOTED, (span) => (unescaped(span.slice(1, -1)) === echoed(claim) ? `"${digest}"` : span));
}
// … and its one call (line 198 today): the object's caller, the very value `digest` was computed from in `fetch`
// (`archive.ts:91-93`): `claimRedacted(e, callerOf(this.ctx.id.name) ?? "", digest)`.
```

(`dispatch` holds no claim: the call reads it as `fetch` does, `callerOf(this.ctx.id.name)`; `digest !== undefined`
already implies it is non-null. The comparison is ONE equality per span, against the claim as the echo writes it: a
claim over 64 characters or with an escaped character is redacted like any other, which plain equality on the raw
claim did not do — the persisted `init-refused` line would have carried it, S2b's D23 (plan review L2-F3, measured
with three claims). That reads R-4's "a span equal to the raw claim" as equal to the claim as the echo writes it,
the only form in which a claim appears in a sentence — for the owner at the close (O-1).)

- [ ] **Step 4: Run them green; then `npm run typecheck && npm run test:scripts && npm test`.**

- [ ] **Step 5: The `BROWSER_UA` recipes (D-13).** In `worker/RUNBOOK.md`: MOVE the variable's one definition — §7's
  `BROWSER_UA=$(node -e 'import("./scripts/smoke-release.mjs")…')` at line 966 — up to the first recipe that needs it
  (§3's operator playbook, before line 497), leaving line 966 a pointer back to it, then add `-A "$BROWSER_UA"` to the
  seven recipe lines: 497, 498, 499 (§3's playbook), 561 (§4's credential read), 587 (§5's health read), 792 and 793
  (§5's upgrade probes). `deploy/RUNBOOK.md` and `docs/web-lane.md` do not move: §3 already sets its own `UA` and
  passes `-A "$UA"` on every recipe (lines 298–330), and the lane holds no recipe `curl`. A `curl` inside a dated record (worker §8–§21, deploy §3.17–§3.21)
  is never edited. The report lists every line changed.

- [ ] **Step 6: The five comment survivors (R-4), each read at the code before it is written:**
  1. `crates/yokohama-web/tests/wire_fixtures.rs:2998`: "Not empty, and the plan expected it to be: these two were
     on the corpus before this cycle, and neither names a person." over a FOUR-row list. Rewrite to name the four:
     "Not empty: `identity_cases` and `why` were on the corpus when this pin was written (M6.5c2 cycle D), `why_withheld`
     joined with the explain surface at the requester's visibility (the security core PR, A1/B1) and `id_cases` with
     the hash router's id grammar (S2a, Task 5); none of the four names a person." (`5304a43f`, `60030076`, `b36dd351`) — confirm each row's
     cycle with `git log -S'"why_withheld"' --oneline -- crates/yokohama-web/tests/wire_fixtures.rs` (and the same
     for `id_cases`) and write what the log says.
  2. `worker/src/index.ts:114` ("stateless, and in this order —") and `answer`'s own "in this order" doc: verified
     true at the plan review (L2-R11): every numbered step of `answer` appears once, in the listed order; no edit.
  3. `web/src/wire/contract.ts:447`: give `IdCase` the doc comment `IdentityCase` has the shape of — "One input of
     the store's id grammar and the layer's answer (security S2a, Task 5): `raw` as a client could send it in a route
     or a hash, `safe` whether `is_safe_id` admits it. Not a route's payload: a CONTRACT, emitted by `wire_fixtures`,
     which the client's hash router (`tableId.ts`) is pinned to as the name step is pinned to `identityCases`." —
     adjusted to the interface's actual fields (read them).
  4. `web/scripts/headers.mjs:177`: the parse refusal names every reason the reader checks — read `parseHeadersJson`
     and word the message as the list of them, the message at `d69c3a7d` already names the bound, and of the nine reasons
     `parseHeadersJson` checks it omits only "non-empty", so it reads: "headers.json is not the client build's header
     set — a non-empty array of [name, value] pairs, each name a token of at most 65,535 bytes, each value non-empty
     and free of control characters, every string well-formed" — and the cell that names it (`headers.node.test.ts`
     line 160's "refuses any other shape by name", which asserts only `toThrow(/headers\.json/)`) GAINS an assertion of
     that literal over `parseHeadersJson("[]")`. The cell runs under `cd web && npx vitest run -c vitest.node.config.ts
     scripts/headers.node.test.ts` (or `npm run test:wasm`); `web/` has no `test:scripts`.
  5. `worker/src/index.ts:84`: wrap the 143-character line at the file's width.

- [ ] **Step 7: The suites, then commit** (documents and the two TS edits may be one commit; the recipes a second).

```bash
git add worker/src/archive.ts worker/src/redact.ts worker/src/redact.unit.test.ts worker/test/open/door.test.ts worker/src/index.ts web/src/wire/contract.ts web/scripts/headers.mjs web/scripts/headers.node.test.ts crates/yokohama-web/tests/wire_fixtures.rs
git commit -m "worker, web: claimRedacted rewrites only the span equal to the claim; every browser segment in a message redacted; five surviving comment sentences made true (security S2c, the residuals)"
git add worker/RUNBOOK.md
git commit -m "docs: every recipe curl carries BROWSER_UA; dated records untouched (security S2c, D-13)"
```

**Named mutants:** M14 the `g` flag dropped → the two-segment cell red; M15 `claimRedacted` back to the first quoted
span → the configuration row red; M16 the equality made a prefix test, in either direction → the rows claim `"may"`
and claim `"maybe-x"`, each against the sentence quoting `"maybe"`, one of them red under each direction; M16a the
comparison made against the RAW claim (the plan's first draft) → the cut-or-escaped cell red on all three claims.

---

### Task 8: B10's host half in the documents, the reserved mode retired, the ledger's rows — Sonnet implementer, Opus reviewer (R-1, R-3, R-5; decisions D-8, D-12)

**Files:**
- Modify: `deploy/cloudflared-config.yml`, `deploy/update.sh` (one line: the swap's `docker run` gains the hardening,
  L4-F5), `deploy/README.md` (§1 lines 39–41, §4's `docker run` and its "127.0.0.1:8080" bullet,
  §6), `deploy/RUNBOOK.md` §3 (the container create body at lines 237–245 today; §0 "What is true before you start"
  does NOT name the shared network — it is §0's "bind address" bullet (lines 49–56: "the server does not verify
  Access's signed assertion, so anything that reached 8080 some other way could send the header") that restates the rule
  Step 2 rewrites, and it gains one sentence: "since security S2c cloudflared also validates the assertion on both ingress
  rules and `README.md` §1's recipe is the rule"; §3's closing paragraph, "Do not publish 8080 to the LAN", stays true), `crates/yokohama-web/src/identity.rs:17-18` (the module doc's last sentence),
  `docs/security-ledger.md` (the rows B10, D3, D8, B1, C2 and the Updated line).
- Test: `cargo test -p yokohama-web --lib identity::` (the doc comment compiles); the reviewer's read of every
  sentence against the code and the configuration; the window's readings (Task 10 Step 10) are the live catcher.

**Interfaces:** none consumed or produced.

- [ ] **Step 1: The cloudflared block — an INSERTION into the file as it stands, never a rewrite.**
  `deploy/cloudflared-config.yml` is 111 lines today, 100 of them comments (the header recipe, the `cf-edge` note,
  each rule's PROTECTED-BY and heartbeat reasoning, the `yokohama.moore.nyc` note). A whole-file block that replaces it
  would drop all of them against this plan's own sentence that "the file's existing comments stay" (lens 4, L4-F1;
  the block as first written parsed but was a different file). The implementer makes exactly three changes:

  (a) the comment below, inserted between `ingress:` and the first rule's `# --- Yokohama: the BROWSER CLIENT` banner;
  (b) after the `service:` line of BOTH rules (`yokohama-nas.moore.nyc`, `yokohama-cli.moore.nyc`), the same block;
  (c) the header recipe's steps 1 and 2 (lines 8–9) reworded, because step 1 says every service sits on
  `cf-internal` and, after the window, the terminal does not: "1. put the service's container on a docker network
  `cloudflared-nas` also joins (`cf-internal` is for `yokohama-web` and cloudflared ALONE since S2c; the terminal has
  its own)" and "2. add an ingress block below, above the catch-all, with the same `originRequest.access` block".
  The comment at lines 62–66 ("the origin does not verify Access's signed assertion … This tunnel is the one path…")
  stays true and gains nothing.

```yaml
  # Security S2c (finding B10's host half; the owner's R-1, 2026-10-13): each rule below carries
  # `originRequest.access`, so cloudflared validates the Access assertion ITSELF before it proxies a request -- the
  # signature, the issuer and the audience -- and a request that reached this connector without passing Access (a
  # bypass policy, a hostname added later) is refused here, in Cloudflare's own code. `teamName` is the team's
  # `<team>.cloudflareaccess.com` prefix and `audTag` the one application's audience (one application carries the
  # three hostnames, spec B11); both are the Worker's committed values (`worker/wrangler.jsonc`), not secrets. With
  # `required: true` a request carrying no valid token answers 403 from cloudflared. Read it back with
  # `cloudflared tunnel ingress validate` before the connector restarts (`deploy/RUNBOOK.md` section 3).
```

```yaml
    originRequest:
      access:
        required: true
        teamName: bdvc
        audTag:
          - 5fa7a989f4f4970771769d68ff273497c125f0374253c0ccf817952dc9c3e8c7
```

  MEASURED on the export (`ruby -ryaml -e 'YAML.load_file(ARGV[0])'`, no `yaml` module in the machine's Python): the
  spliced file parses, the three rules read `required: true`, `teamName: bdvc` and the AUD
  `5fa7a989…c3e8c7` (equal to `worker/wrangler.jsonc:107`'s `ACCESS_AUD`; `ACCESS_TEAM_DOMAIN` at :106 is
  `bdvc.cloudflareaccess.com`, so `teamName: bdvc` is the prefix cloudflared's documented schema takes — verify at the
  window with `cloudflared tunnel ingress validate`, which this plan's D-12 already names). If the terminal hostname is
  covered by a DIFFERENT Access application, its own AUD goes in its block — the orchestrator confirms at the window
  from the Zero Trust dashboard and corrects the file as a window commit; the plan's premise is B11's one application
  (`deploy/cloudflared-config.yml:51-53` and `deploy/RUNBOOK.md` §0: "carries all three hostnames as destinations
  under ONE audience").

- [ ] **Step 2: The README's one rule becomes the recipe.** `deploy/README.md` §1, the paragraph at lines 39–41
  today (line 38 is blank; the paragraph's last sentence is "Publish the port to nothing but the tunnel.") ("**The one rule this door rests on:** …") becomes:

> **The one rule this door rests on, and how to make it true:** the tunnel must be the ONLY path to the container's
> port. The server does not verify Access's signed assertion (`Cf-Access-Jwt-Assertion`) — by the owner's ruling of
> 2026-10-13 (security S2c, R-1): a verifier in the binary would add an outbound TLS dependency, a token parser on
> every request and the first C build in the musl binary to close a path the recipe below closes for nothing — so
> anything that reaches port 8080 without passing Access can send the header and be anyone. The recipe:
> 1. **A Docker network only the tunnel and this server join.** Create it (`docker network create cf-internal`),
>    start `cloudflared` on it, start this server on it with NO port published (`YOKOHAMA_WEB_NETWORK=cf-internal` and
>    `YOKOHAMA_WEB_PUBLISH=` empty for `deploy/update.sh`, or `--network cf-internal` and no `-p` at all), and put every
>    other container — a terminal, a registry, anything — on a
>    different network. cloudflared may join several networks; this server joins one.
> 2. **cloudflared validates the assertion itself.** In its configuration, each ingress rule for an Access-covered
>    hostname carries `originRequest.access` with `required: true`, your `teamName` and the application's `audTag`
>    (`deploy/cloudflared-config.yml` is the owner's). cloudflared then refuses, in its own code, any request that
>    reaches it without a valid token for that application.
> 3. **A hardened container:** `--cap-drop ALL --security-opt no-new-privileges:true --memory 512m` on §4's `docker run`
>    (`deploy/update.sh` passes the same three; the owner's NAS carries them as `CapDrop`, `SecurityOpt` and `Memory` in
>    `deploy/RUNBOOK.md` §3's create body).
> 4. **Read it back:** from a shell in any OTHER container on the host, first a CONTROL that proves the tool works —
>    `wget -qO- --timeout=3 http://127.0.0.1:<that container's own port>/` must answer — then `wget -qO-
>    --timeout=3 http://yokohama-web:8080/api/health` must FAIL with a resolution or connection error ("bad address"
>    or "refused", never "not found"), and so must the same address by the server's IP on `cf-internal`
>    (`docker inspect`; 172.21.0.4 on the owner's NAS) — a name that does not resolve proves nothing about a route.
>    List the network's members (`docker network inspect cf-internal`): the tunnel and this server, nothing else. As
>    yourself through the hostname, `/api/me` names you. Repeat after any change to the host. What this does not
>    close is §4's second bullet: on a shared host, a process that can route to the container's address.

(The following sentence of §1, "Publish the port to nothing but the tunnel.", is absorbed by step 1; §4's `docker run` gains the three hardening flags of step 3 and its bullet on
`127.0.0.1:8080` and §6 "Behind a proxy" (whose `http://localhost:8080` variant publishes the port that step 1 forbids)
gain one cross-reference each to §1's recipe; `deploy/update.sh`'s `run_args` gains the same three flags — MEASURED
on the export: `YOKOHAMA_WEB_NETWORK=cf-internal YOKOHAMA_WEB_PUBLISH= bash deploy/update.sh --dry-run v0.6.3` today
prints a `docker run` with `--network cf-internal` and NO `--cap-drop`, `--security-opt` or `--memory`, so every
self-hoster update would silently undo step 3; with the one added line it prints all three. The `worker` mode's three
"It must" lines stay: that door's contract is the same rule.)

- [ ] **Step 3: The runbook's create body.** `deploy/RUNBOOK.md` §3's JSON (lines 236–245 today) gains, inside
  `HostConfig`, with a comment line above the block in the prose:

```json
  "HostConfig": { "NetworkMode": "cf-internal",
                  "RestartPolicy": { "Name": "unless-stopped" },
                  "CapDrop": ["ALL"],
                  "SecurityOpt": ["no-new-privileges:true"],
                  "Memory": 536870912,
                  "Mounts": [{ "Type": "volume", "Source": "yokohama-web-data",
                               "Target": "/data" }] } }
```

With the prose: "**Hardened since security S2c (B10's host half, 2026-10-13):** the container drops every
capability (it binds 8080 as uid 10001 and needs none), may not gain privileges, and is bounded at 512 MiB — no
recorded reading exceeds a few tens of megabytes (`deploy/RUNBOOK.md` §3.3: 12.0 MB container peak; §3.21's B8 reading
flat at 0.57 MB RSS over 1,000 requests), and the window reads `docker stats` across one capped 4,096-event replay; a
container that hits the bound is restarted by its policy, which is the right answer for a memory leak and a wrong
figure alike (read `docker stats` at the window and raise it with a reason, never silently)." And the terminal
container's network: "**The terminal container is NOT on `cf-internal`** since S2c: it joins its own network, which
cloudflared also joins (Portainer: the container's Networks), so a shell in it cannot reach this server's port; the
negative probe in `README.md` §1 step 4 is read at every deploy." In the same paragraph that says "back up the old one first, and restart"
the runbook gains: "and read the new file back with `cloudflared tunnel ingress validate` inside the connector's
container BEFORE the restart (security S2c; `access.required` is config-read, not exercised — a wrong `teamName` or
`audTag` answers 403 on both hostnames)" — `cloudflared-config.yml`'s new comment already points here for it. The
deploy record §3.22 is the window's (Task 10; R-8: no first window, so §3.21 is the last record today).

- [ ] **Step 4: The reserved mode retired.** `crates/yokohama-web/src/identity.rs` lines 17–18 (line 19 is blank), the module doc's last
  sentence: "Reserved and unbuilt: `cf-access-jwt` and `neon`, each a new arm here." becomes: "Reserved and unbuilt:
  `neon`, a new arm here. `cf-access-jwt` was reserved and is NOT built, by the owner's ruling of 2026-10-13 (security
  S2c, R-1, amending R-SEC4): behind Access the tunnel validates the assertion and a private network is the one path
  to the port — `deploy/README.md` §1 — and the binary stays a trusting origin, as `worker` is." `cargo test -p
  yokohama-web --lib identity::` green; `cargo doc`-safe.

- [ ] **Step 5: The ledger's rows** (`docs/security-ledger.md`), each edited in place in the row's own columns:
  - **B10:** the cycle column `S2 (code) + S3 (host)` becomes `S2c (host half; code half accepted)` (S3 no longer owns the
    host half, and the row would otherwise contradict its own status), the live column's "the shared network read live
    2026-10-08" is kept and the window appends its readings; status `the host half fixed at S2's wrap window (the record: deploy/RUNBOOK.md §3.22); the code half
    ACCEPTED by the owner (2026-10-13, R-1 — R-SEC4 amended): cloudflared's originRequest.access on both ingress rules
    (deploy/cloudflared-config.yml), a network only cloudflared and the server join, CapDrop ALL / no-new-privileges
    / 512 MiB; the README §1 recipe with the negative probe; re-open trigger: a NAS hostname served without Access,
    or a self-host population asking for the binary to defend itself (the trusted-proxy peer check the cheap shape)`;
    catcher `the window's readings: cloudflared tunnel ingress validate; the HostConfig read back; the negative probe
    from the terminal container; /api/me as the owner` (config-read, not exercised — D-12).
  - **D3:** the row's two sentences the cycle falsifies are REPLACED, not appended to — "The NAS's stream cap and its
    body-read timeout stay `open`, S2c's (spec §9.7 correction 11)." and "Recorded, not closed: the lobby draws no
    refusal — `Lobby.tsx` passes no `onEnd` … (AD-4, owed before any open door is deployed)" (its lobby-total
    "parked" note stays); the NAS half goes in their place: `the NAS's half fixed <Task 1's SHA> (S2c Task 1): 8 streams per caller a room, 64
    a table, the house 8 per caller and 256 in all, OPEN streams only (a slot the SSE body drops), the (n+1)-th a 429
    with its sentence before any frame; the client ends a signal on 429 (S2c Task 5) — which is also AD-4's lobby
    refusal drawn on both hosts; the body read has a 10-second deadline at the adapter, 408 (S2c Task 2)`; catchers, by NAME as the C2 row does: `streams.rs` `the_caps_refuse_the_ninth_of_a_caller_and_the_sixty_fifth_of_a_table_and_a_dropped_slot_frees_the_count`,
    `lib.rs` `a_callers_ninth_stream_is_a_429_before_any_frame_and_a_dropped_response_frees_it`,
    `a_cap_does_not_mask_the_admission_reads_refusal` and
    `a_stalled_request_body_is_answered_408_after_the_deadline_and_a_slow_but_arriving_one_reaches_the_route`,
    `web/src/signal.test.ts` "ends on a 429 — the stream cap's refusal …", `web/src/Lobby.test.tsx` "draws an ended
    signal's sentence …" and "keeps the ended signal's sentence when the opening listings land AFTER the end …", and
    `web/src/App.test.tsx` "lands on the lobby with the cap's sentence when the table's own stream is refused 429 …"
    (the implementer copies each name from the task's block and `grep -n`s it).
  - **D8:** `fixed <Task 3's and Task 4's SHAs> (S2c Tasks 3–5): names, display names, watcher names and chat text
    refuse Cc, Cf (Unicode 17.0.0's table in text.rs, 170 code points pinned), Zl and Zp; watcher and house authors
    carry a four-hex tag of SHA-256 over a per-document salt and the key, drawn "name · tag"; under the open door the
    tag says "the same key as before" and nothing more (the owner's accepted risk, R-2)`; catchers by name: `text.rs` `the_format_table_is_unicode_17s_cf_sorted_disjoint_and_170_long`,
    `invisible_refuses_control_format_and_line_separators_and_admits_visible_text`; `tables.rs`
    `a_name_with_an_invisible_or_direction_changing_character_is_refused_by_the_rule`; `chat.rs`
    `a_line_with_an_invisible_character_is_refused_after_the_length_rule`, `a_tag_is_four_hex_of_the_salted_key_and_differs_by_salt_and_by_key`,
    `view_for_tags_watcher_authors_only_and_none_without_a_salt`, `a_document_without_a_salt_field_deserializes_with_none_and_a_view_carries_no_salt`,
    `the_first_append_salts_the_document_and_tags_are_stable_within_it`; `games.rs`
    `the_chat_salt_is_fixed_under_diagnostic_and_fresh_entropy_under_solo_only`; `tables.rs`
    `a_post_reply_carries_the_same_tag_as_the_page_that_follows`; `lib.rs`
    `a_document_stored_under_the_old_name_rule_still_reads_and_the_house_chat_still_takes_a_post` (the read side's
    catcher, Task 3); `web/src/Chat.test.tsx` the tag's label cell.
  - **B1** (the HI-F2 note; the row's last sentence "The design option … is an S2 candidate" is replaced by this one, and
    the catcher column gains Task 6's `a_tampered_record_is_withheld_while_hidden_and_a_mismatch_once_public` and the CLI's
    `locate_is_the_pre_search_half_of_account_parts_and_the_halves_agree`): `HI-F2 ACCEPTED (the owner, 2026-10-13, R-3): at a solo table undo past game over
    stays; the only party is the player; the archive keeps the first finished record and answers a different one
    under the same key 409; the three documents state the solo exception`.
  - **C2** (AD-6): `AD-6 confirmed by the owner (2026-10-13, R-5): a throwing limiter is logged and the request
    proceeds; the window reads the limiter-failed count, expected zero`.
  - **The Updated line:** S2c's documents, the date, the head, the rows moved.

- [ ] **Step 6: Commit.**

```bash
git add deploy/cloudflared-config.yml deploy/update.sh deploy/README.md deploy/RUNBOOK.md crates/yokohama-web/src/identity.rs docs/security-ledger.md
git commit -m "deploy, docs: B10's host half — cloudflared validates the assertion on both ingress rules, the private network and the hardened container as the README's recipe with its negative probe; the code half accepted by the owner's ruling (R-SEC4 amended); the reserved cf-access-jwt mode retired; the ledger's rows (security S2c, R-1, R-3, R-5)"
```

**The reviewer's read (documents have no mutants):** every sentence tried against the code and the files — the AUD
and team name against `worker/wrangler.jsonc` lines 106–107; the YAML parsed (`ruby -ryaml -e 'YAML.load_file(ARGV[0])' deploy/cloudflared-config.yml` — the machine's
Python has no `yaml` module and `pip install` is not to be run — or `cloudflared tunnel ingress validate` if the
binary is installed locally — NO call to the Cloudflare account); `bash -n deploy/update.sh` and the `--dry-run` line
above; the README's four steps against `update.sh`'s
variables; the ledger's cell names against the cells Tasks 1–5 wrote (`grep -n` each).

### Task 9: The wrap's code — the policy enforced with a fixed nonce, the harness, the lane's sentences, the bump to 0.6.3 — Opus (R-6; decisions D-9, D-10; Review Focus 5)

**Files:**
- Modify: `deploy/smoke-image.sh:137-140` (the image smoke reads the policy under `content-security-policy-report-only`
  and fails on empty — `update.sh` step 4 and a hand dispatch of `docker.yml` would refuse every 0.6.3 image; the three
  lines read `content-security-policy`), `worker/scripts/assemble-assets.test.mjs:199` (`block["content-security-policy-report-only"]`
  over `headerSet`'s default reads `undefined` once enforced → `block["content-security-policy"]`),
  `web/src/precache.node.test.ts:135-139` (the served set's first name; inside `describe.skipIf(!existsSync(glue))`, so
  it is SILENTLY skipped until `npm run build:wasm` has run), `web/scripts/headers.d.mts` (the new export's declaration,
  C-33), `web/e2e/harness.node.test.ts` (C-35),
  `web/scripts/headers.mjs:21-40` and `:50-66` (`CSP_MODE`, the nonce constant, `cspOf`'s `script-src`),
  `web/scripts/headers.node.test.ts:35-50` and `:51-78` (the mode and directive cells; a nonce cell),
  `web/scripts/e2e-build.mjs:1-13` and `:30-36` (the comment and its cell's title), `web/e2e/harness.ts:76-125` and
  `:244-268` (`watchForBreakage`'s forgiveness arm and `isReportOnlyDetectionLine` removed; every spec that prints
  `reportOnlyForgiven` — `grep -rn reportOnlyForgiven web/e2e`), `docs/web-lane.md` (§2's e2e row, §7's "0.6.2 at
  this writing", the security-S2a header sentences), `deploy/README.md` (its `0.6.2` lines), `worker/RUNBOOK.md` (the
  live walk's forgiven-line bullet, §5 line 924 today), `deploy/Dockerfile.runtime:18`
  (the example tag), `Cargo.toml`, `web/package.json`, `worker/package.json` and their locks, `web/src/wire/*.json`
  and the backup envelope fixture (regenerated for the version).
- Test: the three header cells; `npm run test:scripts`; `npm run e2e` (the local tier served the policy enforced, as
  before, now with the nonce); `cargo test -p yokohama-web` (`wire_fixtures` and `the_two_npm_packages_carry_the_workspace_version`).

**Interfaces:** consumes nothing; produces the shipped `dist/headers.json` under `Content-Security-Policy` with the
nonce, which Task 10's window reads on both hosts.

**Precondition: none from a window (R-8).** The reading R-6 named as this task's input is taken at the one window's
NAS canary instead (D-15), under the policy this task ships; the STOP and the fallback are read there, before the
Worker deploys. This task ships D-9's shape as written.

- [ ] **Step 1: The failing header cells.** In `web/scripts/headers.node.test.ts`, the mode cell (line 43 today)
  becomes:

```ts
  it("ships ENFORCED since 0.6.3, under the policy's own header name (X2: S2's wrap flipped it)", () => {
    expect(CSP_MODE).toBe("enforce");
    expect(headerSet(NONE)[0]![0]).toBe("Content-Security-Policy");
    expect(headerSet({ ...NONE, mode: "report-only" })[0]![0]).toBe("Content-Security-Policy-Report-Only");
    expect(CSP_HEADER).toEqual({ "report-only": "Content-Security-Policy-Report-Only", enforce: "Content-Security-Policy" });
    expect(() => headerSet({ ...NONE, mode: "report" as never })).toThrow(/report-only or enforce/);
  });

  it("carries one fixed nonce in script-src for the detection script Cloudflare injects, named for what it is", () => {
    expect(CLOUDFLARE_INJECTION_NONCE).toMatch(/^[A-Za-z0-9+/]{22}$/);
    const d = directives(cspOf(NONE));
    expect(d.get("script-src")).toEqual(["'self'", "'wasm-unsafe-eval'", `'nonce-${CLOUDFLARE_INJECTION_NONCE}'`, ...CLOUDFLARE.script]);
    // The nonce rides script-src ALONE: a style nonce would let the edge inject styles too, which nothing asks for.
    for (const [name, values] of d) if (name !== "script-src") expect(values.join(" ")).not.toContain("nonce-");
  });
```

(`NONE`, `directives`, `cspOf`, `CLOUDFLARE` are the file's existing helpers and imports, and
`CLOUDFLARE_INJECTION_NONCE` joins the import list; `web/scripts/headers.d.mts` declares the module by hand and gains
`export declare const CLOUDFLARE_INJECTION_NONCE: string;` — `tsc` fails TS2305 without it; the directive cell at line
51 that lists `script-src`'s sources gains the nonce token in the same position; the cell at line 79, "never carries
'unsafe-inline' or 'unsafe-eval'", is unchanged and must stay green.)

- [ ] **Step 2: Run them red.** `cd web && npx vitest run -c vitest.node.config.ts scripts/headers.node.test.ts` —
  expected: the mode cell fails on `"enforce"`, the nonce cell on the missing export, and the directive cell at
  line 51 on the missing token.

- [ ] **Step 3: `headers.mjs`.** Draw the nonce ONCE and commit it:
  `node -e 'console.log(require("node:crypto").randomBytes(16).toString("base64").replace(/=+$/, ""))'` → 22
  characters, redrawn until it matches `/^[A-Za-z0-9]{22}$/` — no `+` or `/`, so whether the edge's parser takes them
  never arises (the plan review's AD-13; the cell's regex stays as it is, admitting both). Then:

```js
/** The policy's mode, a CONSTANT and not a build value (decision D-1 of S2a): `enforce` ships
 *  `Content-Security-Policy` — since 0.6.3, S2's wrap (security S2c, R-6) — and `report-only` ships
 *  `Content-Security-Policy-Report-Only`, the shape 0.6.2 was built with and never deployed: S2's two windows were
 *  folded into one, and the wild reading is the NAS canary at 0.6.3's window, under the enforced policy (S2c's plan,
 *  D-15).
 *  A build value would let a deploy weaken the policy without a commit. */
export const CSP_MODE = "enforce";

/** **A nonce that never changes, named for what it is** (security S2c, the owner's R-6). Cloudflare reads a nonce
 *  from the response's CSP header and sets it on the scripts it injects — Bot Fight Mode's inline detection script
 *  on a first load — so a static header carrying this value keeps that script running under the enforced policy
 *  and the console clean. A fixed nonce is a password everyone can read: the policy's INLINE-script rule is
 *  therefore nominal, and what the policy still enforces is its source lists, `object-src 'none'`, `base-uri
 *  'none'` and `frame-ancestors 'none'` — which, with no HTML sink in the client, is where its value was. Drawn
 *  once with `randomBytes(16)`, base64 without padding, 22 characters, redrawn until alphanumeric; pinned by its
 *  cell. The recorded fallback, should the edge ever inject the script WITHOUT this nonce: restore the harness's
 *  forgiveness from this commit's parent in the ENFORCED line's words, read at `error`, and drop the token below at
 *  the next release (S2c's plan, D-9 and D-15). */
export const CLOUDFLARE_INJECTION_NONCE = "<the 22 characters drawn above>";
```

and in `cspOf`:

```js
    `script-src 'self' 'wasm-unsafe-eval' 'nonce-${CLOUDFLARE_INJECTION_NONCE}' ${CLOUDFLARE.script.join(" ")}`,
```

The `CLOUDFLARE` constant's comment (lines 32–38 today, "The one script this list cannot name is Bot Fight Mode's
INLINE detection script, which carries no nonce a static shell could match…") is rewritten: "Bot Fight Mode's INLINE
detection script carries whatever nonce the response's CSP header names — `CLOUDFLARE_INJECTION_NONCE`, below."

- [ ] **Step 4: Run the header cells green**, then `npm run test:scripts` (the `_headers` assembly and the deploy
  guard's cells read the new set; a cell that pinned the Report-Only NAME in an assembled `_headers` or in the
  precache's served set — `grep -rniI "report-only" web/scripts web/src web/e2e worker/scripts worker/test deploy`, read by
  eye — moves to the enforced name. The survivors that stay: `headers.mjs`'s mode table and its comment,
  `e2e-build.mjs` (the report-only build's rewrite), `headers.d.mts`'s `CSP_HEADER` type, `framed.spec.ts:27`'s
  `FRAME_FENCES` list, and `sw.node.test.ts:726,740` (an inline fixture header, independent of the mode). The first
  draft's grep was case-sensitive and `web/`-only and missed `deploy/smoke-image.sh` and
  `worker/scripts/assemble-assets.test.mjs`; plan review P-4).

  The files that move with the name, as the plan review measured them (lens 3: `test:scripts` 586 green,
  `headers.node.test.ts` 12 passed):
  - `deploy/smoke-image.sh:138-140`, the image smoke `update.sh`'s step 4 and a hand dispatch of `docker.yml` run:

```diff
-  csp="$(header / content-security-policy-report-only)"
-  test -n "$csp" || fail "GET / carries no content-security-policy-report-only"
-  echo "smoke:   GET / content-security-policy-report-only = $csp"
+  csp="$(header / content-security-policy)"
+  test -n "$csp" || fail "GET / carries no content-security-policy"
+  echo "smoke:   GET / content-security-policy = $csp"
```

  - `worker/scripts/assemble-assets.test.mjs:199`: `expect(block["content-security-policy-report-only"])` →
    `expect(block["content-security-policy"])`.
  - `web/src/precache.node.test.ts:135-139`: the comment's "the policy first under its Report-Only name" → "the
    policy first under the enforced name", and `toBe("Content-Security-Policy-Report-Only")` →
    `toBe("Content-Security-Policy")` — the cell sits inside `describe.skipIf(!existsSync(glue))`, so it runs only
    after `npm run build:wasm`.
  - `web/scripts/headers.node.test.ts`'s two more cells, "is six headers in one order …" (`:140`) and "renders the
    _headers block …" (`:155`):

```ts
describe("the set, its block and its file", () => {
  it("is six headers in one order, the five companions at the c1 review's values", () => {
    expect(headerSet(NONE).map(([name]) => name)).toEqual([
      "Content-Security-Policy", "X-Frame-Options", "X-Content-Type-Options",
      "Referrer-Policy", "Permissions-Policy", "Cross-Origin-Opener-Policy",
    ]);
    const values = Object.fromEntries(headerSet(NONE));
// …
    const lines = headersBlock(headerSet(NONE)).split("\n");
    expect(lines[0]).toBe("/*");
    expect(lines.slice(1, 7).every((l) => /^  [A-Za-z-]+: \S/.test(l))).toBe(true);
    expect(lines[1]).toBe(`  Content-Security-Policy: ${cspOf(NONE)}`);
    expect(lines.at(-1), "ends with one newline").toBe("");
    expect(lines).toHaveLength(8);
  });
```

- [ ] **Step 5: The e2e build and the harness.** `web/scripts/e2e-build.mjs`: the header comment (lines 1–13) and
  the rewrite's comment (30–36) say the rewrite is now an IDENTITY on the shipped set — kept so a report-only build
  (a rollback to 0.6.2's shape) still walks enforced locally; its cell's title (the "Pinned by" cell the comment
  names) says the same (the identity is argued by the build itself, since the file is enforced before the rewrite runs;
  `framed.spec.ts` sees only the served header, so no cell can compare the file byte for byte). The S2a paragraph of the
  header comment is lines 20–31 and the rewrite's comment 38–46; the script is top-level code with no function to call.
  `web/e2e/harness.ts`: remove `REPORT_ONLY_INLINE_SCRIPT`, `isReportOnlyDetectionLine`, the `reportOnlyForgiven`
  field and its arm in `watchForBreakage` (lines 119–121), and in `web/e2e/harness.node.test.ts` the whole
  `describe` "the Report-Only lines (type info) are read, and the live tier forgives exactly one": its import of
  `isReportOnlyDetectionLine` goes, the first cell drops its `reportOnlyForgiven` assertion, the third ("live forgives the Report-Only inline-script line … counted apart") is
  REWRITTEN as "live forgives NO CSP line, whatever its level or its page" (four lines in, `[4, 0, 0]` out), and the
  regex cell goes (as the plan review measured it, the block after this step); the file's doc comment
  (76–84) says: "since security S2c every CSP line is breakage on every tier: the enforced policy carries the fixed
  nonce Cloudflare copies onto the script it injects, so nothing is forgiven, and a CSP line at the live tier is a
  finding (R-6's fallback restores the forgiveness at `error`)". Every spec that printed `reportOnlyForgiven` stops
  (`grep -rn reportOnlyForgiven web/e2e` reads nothing after). `cd web && npm run typecheck` green.

`web/e2e/harness.node.test.ts`, as the plan review measured it (77 passed):

```ts
vi.mock("../playwright.config", async () => ({ BASE_URL: (await import("./live")).baseUrlFor(false) }));

import type { ConsoleMessage, Page, Response as PwResponse } from "@playwright/test";
import { BEACON_PREFIX, capSentence, isBeaconCorsError, isDealAnswer, isDealAnswerLine, isReleaseFeedError, tally } from "./harness";
import {
  EXIT_LAUNCH_ARGS,
  LIVE_ORIGIN,
// …
  "The action has been blocked.",
);

describe("the CSP lines (any console type) are read on every tier, and nothing is forgiven (security S2a, decision D-9; S2c)", () => {
  afterEach(() => {
    vi.doUnmock("./live");
    vi.resetModules();
// …
    // The wording with `report-only` before the policy's name (the regex's optional clause), at `info`: read too.
    fake.line("info", RO_INLINE.replace("following Content Security Policy", "following report-only Content Security Policy"), `${local}/`);
    expect(watch.failures, "the report-only wording, read at info").toHaveLength(3);
  });
  it("locally a CSP line is breakage even when a cell's expected status digits appear in its hash — the status filter never swallows it", async () => {
    const { watchForBreakage } = await harnessAt(false);
// …
    fake.line("error", ENFORCED_INLINE.replace("aBV9DC+", "a409DC+"), `${baseUrlFor(false)}/`);
    expect(watch.failures, "an enforced refusal whose hash spells 409").toHaveLength(1);
  });
  it("live forgives NO CSP line, whatever its level or its page (security S2c: the enforced policy carries the nonce)", async () => {
    const { watchForBreakage } = await harnessAt(true);
    const fake = consolePage();
    const watch = watchForBreakage(fake.page);
    fake.line("info", RO_INLINE, `${LIVE_ORIGIN}/`);
    fake.line("info", RO_STYLE, `${LIVE_ORIGIN}/`);
    fake.line("error", ENFORCED_INLINE, `${LIVE_ORIGIN}/`);
    fake.line("info", RO_INLINE, "https://elsewhere.test/");
    expect([watch.failures.length, watch.forgiven, watch.feedForgiven]).toEqual([4, 0, 0]);
    fake.line("info", "an ordinary info line", `${LIVE_ORIGIN}/`);
    expect(watch.failures, "an ordinary info line is still read as nothing").toHaveLength(4);
  });
});
```

- [ ] **Step 6: The local tier, enforced with the nonce.** `cd web && npm run e2e` — expected: every spec green (81
  before this cycle; Task 5 may have added none to the e2e), no CSP line (there is no edge locally, so no injected
  script, and the nonce is inert). Record the count and the wall.

- [ ] **Step 7: The lane's, the README's and the runbook's sentences (R-8: 0.6.2 was never deployed, so no sentence may
  say it ran).** `docs/web-lane.md`: §2's e2e row ("rewrites that build's `dist/headers.json` so the policy is served
  ENFORCED" → the rewrite is an identity since 0.6.3); the security-S2a bullet "**0.6.2 ships its policy Report-Only,
  with one known line**" → "**0.6.2 was built Report-Only and never deployed; 0.6.3 ships the policy ENFORCED with a
  fixed nonce**" — the two windows folded into one (R-8), the nonce the edge copies onto the script it injects (R-6,
  D-9), the canary read on the NAS first (D-15), the fallback named, B4's frame fence both headers from 0.6.3 (spec
  correction 12); §7's "`0.6.2` at this writing" → `0.6.3`
  (it wraps across a line break, so a one-line grep misses it); §2's e2e row's later clause "the harness forgives
  EXACTLY four known shapes — … and, since security S2a, the Report-Only refusal of the inline detection script …" →
  three shapes, the fourth retired by S2c; §4's header bullet (line 161), all three of its passages — "**The policy is
  Report-Only in 0.6.2 and enforced at S2's wrap** … B4 rests on `X-Frame-Options` alone", "**The e2e build alone serves
  it ENFORCED** … the local walk is the enforcement reading", and "**Bot Fight Mode's injected detection script is the
  release's one known line** … the reading is the window's own sessions" — rewritten as the 929 bullet is: 0.6.3 ships
  the policy ENFORCED with a fixed nonce; nothing is forgiven; B4's frame fence is both headers. `deploy/README.md`: "In 0.6.2 the policy is sent as
  `Content-Security-Policy-Report-Only`: a browser logs a violation and blocks nothing, and a later release enforces
  it." → "Since 0.6.3 the policy is sent as `Content-Security-Policy`, enforced, with a fixed nonce for the script
  Cloudflare injects (`web/scripts/headers.mjs`); 0.6.2 was built with it Report-Only and never deployed."; "(older
  than 0.6.2, or none)" stays (it names the build that first carried the file); each `yokohama-web:0.6.2-s2a` →
  `yokohama-web:0.6.3-s2` where it names the current release (the image tag examples), never in a dated sentence.
  `worker/RUNBOOK.md`'s live-walk bullet "live only, since security S2a (decision D-9), the Report-Only refusal of the
  INLINE detection script…" (§7 "The live tiers and the quota guard", line 924 today; the bullet runs 924–930) → "since security S2c (R-6) the walk forgives NO CSP line on any
  tier: the enforced policy carries the fixed nonce the edge copies onto its detection script, so a CSP line is a
  finding (the fallback, D-15, restores a forgiveness at `error` in the line's measured words)".

- [ ] **Step 8: The bump (the lane's §7, the five steps), as the LAST code commit of the cycle.**
  1. `Cargo.toml` `[workspace.package] version = "0.6.3"`; `cargo update --workspace --offline` — the lock's nine
     member lines and nothing else (`git diff --stat Cargo.lock`).
  2. `cd web && npm version 0.6.3 --no-git-tag-version --ignore-scripts`; the same in `worker/`.
  3. `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` and `UPDATE_WIRE_FIXTURES=1 cargo test
     -p yokohama-web --lib backup` — READ the diff: the version and nothing else. The corpus lives in TWO
     directories, and the pathspec `web/src/wire` does not match `web/src/wire-script`: the bump moves
     `web/src/wire/health.json`, `web/src/wire-script/archive-walk.json` and `web/src/wire-script/backup-envelope.json`
     (measured), and the records are single long lines that a line-based `grep -v 0.6.` cannot read. The check:
     `for f in $(git diff --name-only web/src/wire web/src/wire-script crates/yokohama-web/tests/fixtures); do git show
     HEAD:$f | sed 's/0\.6\.2/0.6.3/g' | diff -q - $f; done` prints nothing.
  4. `deploy/Dockerfile.runtime` line 18's example tag → `yokohama-web:0.6.3-s2`.
  5. `cd web && npm run build:wasm && cd ../worker && npm run build:module:debug`, then the suites:
     `cargo test -p yokohama-web` (`the_two_npm_packages_carry_the_workspace_version` and `wire_fixtures` clean
     without the flag), the lane's rows, `cd web && npm run build` (the RELEASE client, so `web/dist` carries the
     enforced set for the window; `worker/gen` and `worker/assets` by `cd worker && npm run build`).

- [ ] **Step 9: Commit — two commits.**

```bash
git add web/scripts/headers.mjs web/scripts/headers.d.mts web/scripts/headers.node.test.ts web/scripts/e2e-build.mjs web/src/precache.node.test.ts web/e2e docs/web-lane.md deploy/README.md deploy/smoke-image.sh worker/RUNBOOK.md worker/scripts/assemble-assets.test.mjs
git commit -m "web: the policy ENFORCED with a fixed nonce for the detection script Cloudflare injects; the harness forgives no CSP line on any tier; the lane's sentences (security S2c, R-6; the fallback recorded)"
git add Cargo.toml Cargo.lock web/package.json web/package-lock.json worker/package.json worker/package-lock.json web/src/wire web/src/wire-script crates/yokohama-web/tests/fixtures deploy/Dockerfile.runtime docs/web-lane.md
git commit -m "release: 0.6.3 — security S2 (the browser's fences, the Worker's door, the NAS's identity; the policy enforced); the wire's version lines regenerated and read"
```

**Named mutants:** M17 `CSP_MODE` left `report-only` → the mode cell red; M18 the nonce token dropped from
`script-src` → the nonce cell red; M19 a nonce of 16 characters → the shape row red; M20 the nonce copied into
`style-src` → the "script-src ALONE" row red; M21 the harness's forgiveness arm restored → `grep reportOnlyForgiven`
non-empty, the reviewer's read; M22 the version bumped in `Cargo.toml` alone → `the_two_npm_packages_carry_the_workspace_version`
red.

---

### Task 10: The plan review, the branch, the dispatches, the suites and the roster, the after review and its re-read, the close, the PR, the stack review, the merges, the tag and release, the one window, the hand-off into the auth cycle — ORCHESTRATOR (R-4, R-6, R-7, R-8, X1, X3; decisions D-10–D-15)

- [ ] **Step 0: The plan review (this session or the next) — the owner's R-1 method: a Workflow if the owner opts in
  by name, else the same lenses and assembler dispatched as agents, none messaging another.** Four Sonnet lenses on `git archive` exports of the plan's committed head (`d69c3a7d` for this review), each EXECUTING the
  plan's blocks for its slice and measuring every claim, then one Opus assembler that dedups, reads every
  contradiction at the code and hands the orchestrator a packet with adjudications: lens 1 the layer's Rust (Tasks
  1–4: the cells spliced and run; the `Admitted` adapter's `Unpin` bounds against the real stream types; the stalled
  body cell's channel; the Cf table against the UCD file; the salt's fixture determinism through the corpus walk
  and the pool); lens 2 the client and the Worker shell (Tasks 5 and 7: the signal and lobby cells run in vitest;
  the `claimRedacted` and `g`-flag cells in the pool; `usePolled`'s `onEnd` reaching the lobby's refusal); lens 3
  the `why` extraction and the wrap's code (Tasks 6 and 9: the CLI's `why` cells byte-identical across the
  extraction; the tampered-record cell; the header cells and the e2e build under the nonce; the bump's five steps
  dry-run on the export); lens 4 the documents against the code and the decisions (the rulings transcribed against
  this session's words; the cloudflared block's YAML and the README's recipe against `update.sh`; the ledger's rows
  against the cells; placeholders; the window's recipes). Every lens writes its report under
  `docs/temp/security-review-2026-10-08/s2c-plan-review/`, replies with the path and three lines, runs no `rm -rf`,
  arms no Monitor, makes no call to the Cloudflare account, messages no agent. The packet's items are adjudicated
  by the orchestrator and SPLICED INTO THIS PLAN at the source; the record is this plan's §"Review record".

- [ ] **Step 1: The branch and the first commit** (no pre-flight, R-7). `git checkout -b security-s2c 2ea0edf3`;
  this plan, the board's Track 4 and the S2a plan's open window placeholder (its §"Review record"'s last heading,
  closed with the fold's forward note) as the first commit; the plan review's corrections as the second; the SDD
  ledger opened at `docs/temp/security-review-2026-10-08/s2c-sdd/progress.md`.

- [ ] **Step 2: The dispatches (superpowers:subagent-driven-development).** Tasks 1–9 in order; Opus implementers
  and reviewers, Tasks 7 and 8's implementers Sonnet; each brief cut from this plan with §"Names shared across
  tasks" and §"Global Constraints"; each implementer PROBES its brief and reports `DONE_WITH_CONCERNS` on a false
  sentence; each review on an export with the task's named mutants; a fix round a NEW commit; one writer in the tree
  at a time; `npm run build:module:debug` before every pool run after a task that moved the layer. Every ruling a
  `Ruling:` line in the ledger; every plan correction at the source first. Task 9 waits for nothing: the
  window's reading is taken after it (R-8, D-15).

- [ ] **Step 3: The suites at the final head, and the roster at the bump.** The per-edit suites and the web lane's
  rows (`docs/web-lane.md` §2): core, CLI, oracle, difftest, bridge (the lib/bin suite AND its four node suites:
  `node bridge/tests/outbox_race.js && node bridge/tests/wrapper_transparency.js && node bridge/tests/safe_table_parity.js && node bridge/tests/connected_indicator.js`),
  eval_probe, solver, clippy; `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`;
  `cargo test -p yokohama-web --no-default-features --features server --lib identity::`;
  `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown`; `web`: `typecheck`,
  `vitest run`, `test:wasm`, `npm run e2e`; `worker`: `typecheck`, `test:scripts`, `npm test`. **The fifteen-gate
  roster ONCE, at Task 9's bump commit, as one uninterrupted sequence, release binaries pre-built, single-threaded
  where the command says so, the machine tagged** (CLAUDE.md's rule; AD-3's precedent; target ≤420 s, the last
  readings 487 s and 484 s). The counts recorded against the board's; `docs/web-lane.md` §2's counts re-counted;
  `git diff 2ea0edf3 --stat -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest
  crates/yokohama-bridge crates/yokohama-wasm crates/yokohama-worker crates/gen-data` EMPTY and
  `git diff 2ea0edf3 --stat -- crates/yokohama-cli` naming `src/why.rs` alone; `SEMANTICS_REV` 7.

- [ ] **Step 4: The after review, its one wave, and the deferred lists' re-read (D-14).** The orchestrator's own
  re-derivation of every row from the code; one Opus battery over every named mutant (M1–M42 and the lettered
  additions the review added) on an export — the third and last walk; one fix wave for the deferred minors,
  re-reviewed; **then the re-read:** the S2a, S2b and S2c SDD ledgers' deferred and parked items, each surviving
  sentence tried against the head, every false or stale COMMENT or DOC sentence taken into the wave (a test or design
  note stays recorded), the list of what was read and what was taken in the close.

- [ ] **Step 5: The close.** The ledger (Task 8's rows confirmed at the head, the SHAs filled); the board's Track 4
  (S2c landed; the wrap's steps below as the next; the auth cycle after); guidingpath's rows dated at the close — S2c
  landed, R-SEC4 amended by the owner, the enforcement shape — and its current state, whose "the first of S2's two
  windows … is still OWED" (`guidingpath.md:33`) R-8 falsified (the 2026-10-15 R-8 row itself landed with the plan
  review's corrections, AD-10); CLAUDE.md's standing
  sentences: the browser-lane paragraph's clause "the policy is Report-Only until S2's wrap, and the e2e build alone serves it
  enforced, since Chromium logs a Report-Only violation at `info` and a violation inside the module Worker prints
  nowhere (security S2a, 2026-10-12)" — the WHOLE clause, through the parenthesis, since the quoted prefix ends in a
  `since` that would otherwise explain a policy no longer Report-Only (CLAUDE.md line 85) — → "the policy is
  ENFORCED since 0.6.3 with a fixed nonce for the detection script the edge injects (`web/scripts/headers.mjs`), so any
  CSP line on any tier is a finding (security S2c, R-6)", and one new standing sentence
  under the browser lane's paragraph: "**B10's code half is accepted by the owner (2026-10-13): the NAS binary is a
  trusting origin behind an authenticating edge, and the one rule — the tunnel is the only path to its port — is
  made true by `deploy/README.md` §1's recipe (a private network, cloudflared's own validation, a hardened
  container) and read by its negative probe at every deploy; `cf-access-jwt` is not built.**"; the spec's §9.8
  (this plan's rulings, decisions and corrections verbatim, dated 2026-10-13) AND the §9.5 gap: S1c's run's-rulings
  paragraph, appended under §9.5 from `docs/plans/2026-10-10-security-s1c-plumbing-plan.md`'s §"Review record"
  (the run and the after review entries), dated at the close and marked as the gap S2a's Task 9 named; AND §9.6's gap
  (correction 13): S2a's run's-rulings paragraph, appended under §9.6 from
  `docs/plans/2026-10-11-security-s2a-browser-fences-plan.md`'s §"Review record" (the run and the after review
  entries), dated at the close; the lane's §1
  deployed-state bullet (it still reads "LIVE at `0.6.0` on cycle F's release" with the Worker version `703e60d5`, the NAS
  `0.6.0-m6.5c2f` and "the merge of the stack … waits on the owner's word" — S1's wrap left it, so the rewrite is
  0.6.0 → "0.6.3 from the window on") and §14's "defaults both deployments run" lead-in ("As cycle F's release set them"); this
  plan's §"Review record" (the plan review, the run, the after review, the re-read); the memory file. The SDD
  ledger stays in place.

- [ ] **Step 6: The PR.** `security-s2c` pushed; a PR against `security-s2b`, its body naming the rows, the readings
  and the roster's wall; the merge is S2's wrap (Step 8).

- [ ] **Step 7: The stack review — four Sonnet lenses in a Workflow, verified by the orchestrator (X3: as S1's; the
  owner's opt-in at this step).** Load `workflow-authoring` first. Four `agent()` calls in `parallel`, Sonnet, each
  over a `git archive` export of `security-s2c`'s head with `master` (`1e9c092d`) checked out beside it for the
  diff, each writing its report to `docs/temp/security-review-2026-10-08/s2-stack-review/<dimension>.md` and
  returning a typed findings list (`file`, `line`, `claim`, `reproduction`, `severity`); the four dimensions: (1)
  **hidden information and identity** — S2b's door rows (B5, B6, C1, D23) and S2c's `why` flow and tag: every
  payload, sentence and log line read for a value that narrows a hidden card or names a caller to another; the
  tampered-record cell's premise tried; (2) **the cost bounds** — S2b's limits and caps and S2c's streams and
  deadline: every bound tried past its figure, every slot traced from take to release (an abandoned connection, a
  panicking frame), the stalled body over a real socket; (3) **the client and the headers** — S2a's set under
  `enforce` with the nonce, the signal's four ending statuses, the lobby's end, the tag's label: the local tier run,
  the assembled `_headers` and the NAS's `headers.json` diffed; (4) **the documents' claims against the code** — the
  lane, the ledger, the spec's §9.6–§9.8, CLAUDE.md's new sentences, both runbooks and the README's recipe, every
  doc comment the three cycles wrote, and the three SDD ledgers' deferred lists against the head (the false-sentence
  lens, the top yield of every close). Each brief forbids messaging other agents, `rm -rf`/`rm -r`/bulk `rm -f`,
  Monitors and event waits, and any call to the Cloudflare account, and requires the full report to its file with a
  three-line reply. **The orchestrator verifies every finding by trying its mechanism on its own export**; confirmed
  findings are fixed as NEW commits on `security-s2c` with their catchers, and the fix commits get a scoped second
  run of the dimension that found them. The record: this plan's §"Review record" and the ledger.

- [ ] **Step 8: The merges** (X1; the owner's rulings of 2026-10-09 and 2026-10-11). In order, each a MERGE COMMIT
  through `gh pr merge --merge`, never a squash, so every task's SHA the records cite survives: the S2c PR
  (`security-s2c` → `security-s2b`); PR #54 (`security-s2b` → `security-s2`); PR #52 (`security-s2` → `master`).
  Master has not moved past the stack's base `1e9c092d` (checked 2026-10-13), so master's content is byte-identical
  to `security-s2c`'s head after the three; if master HAS moved by then, the stack is rebased onto it first by the
  orchestrator with the owner's word, never merged with a conflict resolved silently. Before the first merge the orchestrator rewrites the titles and bodies of
  #52 and #54 (`gh pr edit`; a read of GitHub today shows #52 titled "…the header set in Report-Only; 0.6.2"), because
  `gh pr merge --merge` writes the PR's title into the merge commit's body and master would carry a sentence the fold
  made false (R-4's rule for the public launch, L4-F9).

- [ ] **Step 9: The tag and the release, BEFORE either deploy** (the owner's standing order from S1's wrap). On
  master: `git tag -a v0.6.3 -m "Yokohama CLI v0.6.3 — security S2: the browser's fences, the Worker's door, the NAS's identity; the policy enforced" && git push origin v0.6.3`;
  `release.yml` runs on any `v*` tag; the release's assets read on GitHub before the window opens.

- [ ] **Step 10: The window — ONE for S2 (R-8), the orchestrator with the owner at the keyboard, both hosts, the NAS
  first as the canary (D-15).** It OPENS by naming the two-identity block (c) and agreeing its slot (S2a's D-8; the
  owner's two Access sign-ins), with Claude in Chrome recorded as reachable in both profiles before any deploy (S2a's
  opening precondition); WARP off only for that slot; every `curl` over WARP with the smoke's `BROWSER_UA` (set as `worker/RUNBOOK.md` §7's burst bullet sets it:
  `cd worker && BROWSER_UA=$(node -e 'import("./scripts/smoke-release.mjs").then((m) => console.log(m.BROWSER_UA))')`); the
  quota guard first (`worker/RUNBOOK.md` §7) and again after the live Playwright tier, a run that could cross the
  owner's 80,000 line (I-82) HOLDING to the next UTC day (S2a's Step 9); `cd web && npm run build` (with the site URL) and
  `cd worker && npm run build` first; the live replay's reset recipe (`worker/RUNBOOK.md` §7, "The replay's reset" — the lane's §7 bump step 6 sends you there; the
  RUNBOOK has no numbered step 6) BEFORE the replay (S2a's Task 8
  moved the archive fixture's records). Every probe that creates state removes it — every probe table deleted (S2a's
  (c)), (a)'s scratch table and the tag's watcher post included. The hosts run 0.6.1 at the open (`v0.6.2` was never deployed), so every
  "0.6.2" in S2a's recipes reads 0.6.3 here and both update readings go 0.6.1 → 0.6.3.
  (a) **The NAS, in this order:** the update reading pinned FIRST (`node web/scripts/pinned-browsers.mjs nas
  https://yokohama-nas.moore.nyc web/scripts/pinned/nas-s2 --go-wait 90`), its `PINNED` read before the NAS stops;
  **the network split** in Portainer (the
  terminal container moved off `cf-internal` onto its own network, cloudflared joined to both; the terminal's
  hostname still serving) and **the cloudflared file** applied (the NAS's running `config.yml` BACKED UP first —
  `deploy/RUNBOOK.md` §3, and the rollback bullet below needs it; `cloudflared tunnel ingress validate` inside the
  connector's container, then its restart; its log read for the ingress rules and no refusal); the runtime image
  `yokohama-web:0.6.3-s2` from the tagged master (`deploy/RUNBOOK.md` §1–§3) and the container RECREATED with the
  hardened `HostConfig` (read back through Portainer: `CapDrop`, `SecurityOpt`, `Memory`); health `server_version:
  0.6.3`; the header reads after the health read (`content-security-policy` carrying `'nonce-<the constant>'`, the
  five companions, `/api/health`'s `no-store` and `nosniff`); **then `touch …/nas-s2/go` — NOW, before the canary and the
  probes (S2a's Step 9(a) put it here, and the instrument's own header says "touch `go` as soon as the deploy's smoke
  passes": from the deploy on, either pinned tab's own 15-minute check finds the release, and the check "the door had
  not taken a release before `go`" then reads the interval's, not the reload's — S1's NAS reading held it with `go` at
  11:06:36Z, 31 seconds after the server's "listening" line, `deploy/RUNBOOK.md` §3.21; the first draft put `go` after the whole list, an hour on)**;
  **THE CANARY (D-15):** a fresh Chrome profile's first
  visit to `https://yokohama-nas.moore.nyc/`, the console at EVERY level, a second (SW-served) navigation printing
  nothing, and the injected script's nonce read
  (`curl -A "$BROWSER_UA" -s https://yokohama-nas.moore.nyc/ | grep -o 'nonce="[^"]*"'` over WARP — `deploy/RUNBOOK.md`
  §3.15 measured the edge's rewrite on every `curl` of `/` through this tunnel; or the browser's VIEW-SOURCE, never the
  Elements panel, since a browser hides a `nonce` attribute's value from the DOM once a policy is enforced) — (i) clean: the Worker's
  deploy proceeds; (ii) the detection script's line alone: the fallback, recorded, the window continues; (iii) any
  other line: STOP, the owner decides before (b), the NAS rolled back to `yokohama-web:0.6.1-s1` or left (it is the
  owner's alone); **the network read and the negative probe** (`GET /networks/cf-internal` through Portainer lists exactly
  `cloudflared-nas` and `yokohama-web`; from the terminal container's shell a control against its own listener
  answers, then `wget -qO- --timeout=3 http://yokohama-web:8080/api/health` and the same by the server's IP fail with a
  resolution or connection error — the repository knows nothing of the terminal image (no Dockerfile in it), so whether
  `wget` is there is a window-time read, and a probe that fails because the tool is missing proves nothing); **S2a's readings on this host** (S2a's Task 9 Step 9(a) and (c),
  superseded into here): the response headers of `/`, `/shell.bin`, the entry chunk and `/api/health` against S2a's
  spec correction 10's set under the ENFORCED name, the 404 page's style hash; B4's framed page in Chrome (refused; if Access
  answers the framed request — a cross-site frame carries no `SameSite=Lax` cookie — the app's own header is read
  from the headers row);
  D13's SW-served navigation on the fresh profile carrying the set; A1 and B3 re-read; **S2c's probes:** nine
  `curl -N` streams as one identity at a scratch table, the ninth a 429 with `TOO_MANY_STREAMS_CALLER` (then closed);
  a POST with `-H 'Content-Length: 100'` and a stalled stdin, its answer RECORDED with its body — the server's
  408 is the one whose body is `{"error":"the request body did not arrive within 10 seconds"}` after ten seconds; any
  other answer is the edge's or the connector's (both may buffer a body before the origin sees it), recorded as such,
  with the cell and the plan review's real-TCP probe (10.0 s, lens 1) the deadline's catchers; a name with `\u{200B}`
  refused 400 by the rule's sentence; a watcher's post read back with its four-hex tag by a second identity;
  `docker stats` for the container's memory under the probes (the 512 MiB bound's margin recorded); then `done` once
  the update reading's checks print.
  (b) **The Worker**, on a clean or a fallback canary: the update reading pinned (`worker https://yokohama.moore.nyc
  web/scripts/pinned/worker-s2 --go-wait 90`); `npm run deploy:dev` (the dev door ON for the live tiers — S2b's
  bracket; the secrets guard's sentence); health 0.6.3; **`touch …/worker-s2/go` right after the health read, `done`
  once its checks print** (S2a's Step 9(b); the first draft carried neither, so the run would end on `--go-wait`'s
  bound with a failed verdict); the header reads (the nonce in `content-security-policy` on
  `/`, `/shell.bin`, the entry chunk and `/api/health`'s own pair); the same console read on a fresh profile (D-15's
  three readings; a reading that differs from the NAS's is a finding); the live corpus replay TWICE after its reset,
  HOLDING; the live Playwright tier once (`two_browsers`, `joins_by_link`), NO CSP line expected on a clean canary (on
  the fallback, exactly the recorded line, counted as the finding); **S2a's readings on this host:** the header
  reads above; B9's Worker half on a table answer (`no-store`, `nosniff`); D14's `#/t/..`; B4's framed page (with (a)'s Access
  caveat); D13's SW
  navigation; **S2b's live acceptance** (spec §5.3; X2): the 429 burst at the 121st of 140 (`worker/RUNBOOK.md` §7's
  recipe, AD-7), the socket cap (nine sockets as one identity, the ninth's `end` and 4429), the operator playbook's
  JSON type (a bodyless POST to expire → 403; the JSON POST → 204; a foreign-Origin POST → 403), D23's redaction in
  the Workers logs (no address, no query string; the `init-refused` line a digest), the `limiter-failed` count (zero
  — R-5), the lobby's refusal drawn (the ninth lobby socket's sentence on screen — AD-4); `npm run deploy` (the
  production file, the dev door OFF) last and read live (the credential plus a `.test` claim names nobody); the
  update readings: `verdict.json` reads "every check held", `log.txt` holds the `cookies: cleared before close` line
  before its final `closed:` line, and the profile's `Default/Cookies` holds no row.
  (c) **The two-identity block** (S2a's D-8 and AD-5, superseded into here; its slot agreed at the open): profile A
  the owner's, profile B `myfreemovies@gmail.com`, WARP off for this slot only, every probe a page-context
  `fetch`/`EventSource` from the profile's own tab through Claude in Chrome, the cookie never extracted (a `umask
  077` cookie file the named fallback); on the NAS hostname: D1 — A creates `[A, open]` (no lock: an open table admits any
  stream, `api.rs:2082`); B opens the table's stream from its tab; A joins seat 1 ITSELF (a person may hold two seats):
  B's stream ends after ONE `gone` frame and closes (`a_refused_stream_writes_gone_once_and_closes`; the ledger's D1 row
  names exactly this end, and its live column reads "not read live 2026-10-11"); B1's withheld `why` at a bot-first table — A creates
  `seats: [{"bot": true}, {"user": A}]` with `start_seat: 0` and keeps (`POST …/keep {"seat": 1, "card": 1}`); B as a
  watcher, and A as the bot's opponent, ask before the bot's kept card is public and read the WITHHELD payload, its
  response time recorded (S2c's "without delay" reading) beside a PUBLIC `why`'s — A's at a FINISHED vs-bot table on the
  NAS if one exists (a public `why` needs game over with a held seat, `explain::visibility_of`, about 130 human moves
  away at a new table); with none, the withheld time is recorded against the search's 1 ms–4.4 s; rows 108 and 119 as B on the Worker (the rows are the web-lane straw poll's, `docs/plans/2026-09-28-web-lane-straw-poll.md`
  lines 149 and 160, which `docs/web-lane.md` §8 "What the windows did NOT read" names: **108**, a second signed-in
  address at a single-audience table; **119**, `GET /api/admin/tables` as B — the operator gate's refusal of a
  non-operator — S2a's Step 9(d)); every probe table deleted, any the server filed under `rev7/abandoned/` removed by
  name (`deploy/RUNBOOK.md` §3.21's recipe), and a cookie file, if the fallback made one, deleted; WARP back on
  (`warp-cli debug access-reauth` if Access answers "Please authenticate via the warp client").
  (d) **The ledger's live column** for S2a's, S2b's and S2c's rows; the deploy records (`deploy/RUNBOOK.md` §3.22,
  `worker/RUNBOOK.md` §22, each dated, each naming the tag and both `server_version` readings, the canary's reading
  (i)/(ii)/(iii) with its console lines verbatim, the header reads, the two-identity block's slot and its two
  readings (D1, B1) and the §8 rows, and, for the NAS, the network split, the cloudflared validation as
  CONFIG-READ (D-12), the HostConfig and the negative probe); the rollback bullets (`wrangler rollback` to the 0.6.1
  gate deployment `fd34c477`; the NAS's `yokohama-web:0.6.1-s1` image and the pre-S2c `HostConfig`; the cloudflared
  file's previous revision, which is the backup (a) made); the S2a plan's §"Review record" window heading already
  points here (Step 1).

- [ ] **Step 11: The hand-off into the auth cycle.** The board's Track 4 rewritten: S2 COMPLETE and live (the merge
  SHAs, the tag `v0.6.3` — `v0.6.2` never cut, R-8 — the window's readings, the live column); the next session's first step — "the AUTH CYCLE's
  brainstorm, its own branch from master after S2's wrap, stacked under nothing (the owner, 2026-10-12; spec §9.7's
  closing paragraph): the providers reviewed and picked (Auth0, Clerk, Stytch or the like) for the public Worker
  behind the Turnstile on the layer's `worker` door; one model per deployment, Access kept live until the switch
  and kept as an operating provider after it; the new work shell-side — the second verifier arm, a Worker-minted
  session cookie for `EventSource` and WebSocket, the client's sign-in flow, the header set's allowances if the login
  is embedded, the deploy guard's second identity (D-7 of S2b); S3 stacked on it, then S4"; guidingpath's current
  state; the project memory updated the same way; `docs/temp/security-review-2026-10-08/s2c-sdd/` and
  `s2-stack-review/` left in place with every report; the session's exports named for the owner to remove (bulk
  deletes being forbidden to agents). The next session begins at the board.

---

## Review record

Filled by the plan review (Step 0), the run (Steps 1–4) and the after review and its re-read (Step 4): each entry
dated, each finding with its adjudication and the correction's place in the text above, each reading with the head
it was read at.

### The plan review (2026-10-15) — four Sonnet lenses on exports, one Opus assembly, the orchestrator's adjudication

**The run.** Step 0 as agents, not a Workflow, none messaging another: four Sonnet lenses, each on its own `git archive`
export of `d69c3a7d`, EXECUTING the plan's blocks for its slice and measuring each claim — lens 1 the layer's Rust
(Tasks 1–4), lens 2 the client and the Worker shell (Tasks 5 and 7), lens 3 the `why` extraction and the wrap's code
(Tasks 6 and 9, with one local Playwright run and the bump dry-run), lens 4 the documents against the code and the
decisions (Tasks 8 and 10, the rulings, the fold and the window) — then one Opus assembler that read the four reports,
their patches and the plan whole, re-read every contradiction at the code, and wrote the packet; the orchestrator
adjudicated it, and the corrections were spliced into the text above at the source. The record is under
`docs/temp/security-review-2026-10-08/s2c-plan-review/`: the briefs, `lens-1-rust.md` … `lens-4-docs.md` with their
patches and logs, `assembled.md` (the packet) and `splicer-report.md` (each correction's place in this text, and the
few applied with a drift fix).

**The counts.** 72 lens findings (19, 19, 20 and 14), three merged as findings with two witnesses, seven added by the
assembler at the code and one split, for 77: Critical 6 (5 mechanism, 1 run-as-written), Important 32 (8 and 24),
Minor 25 (4 and 21), Information 14; 19 adjudications; 2 items for the owner; 17 claims no offline check reaches. A
correction a lens measured green is in the text above in its measured form; those the packet marked "(proposed)" —
among them C-2's echo-equality, measured at the function level only, and the window's recipes — went in as the
packet wrote them.

**What the review found, in one paragraph.** Six Criticals: the name rule applied on READ (`check_shape` runs on every
stored document) turned a pre-cycle name carrying a joiner or a zero-width space into a 500, the house chat's GET and
POST for everyone (P-1); `claimRedacted`'s equality with the raw claim left a claim the echo cut or escaped in the
persisted `init-refused` line, S2b's D23 regressed (P-2); the lobby's `take` and `copyLink` clear the strip the ended
signal's sentence landed in, so the lobby read "live" over a dead stream again (P-3); the image smoke and the
assembly cell read the policy under the Report-Only name, so every 0.6.3 image fails `update.sh`'s step 4 and the cell
fails `test:scripts` (P-4); two of `view_for`'s three sites are the POST replies, which hold no document, so a reply
drew no tag where the next page drew one (P-5); and the window's `go` sat after every NAS probe and was missing on the
Worker, so the NAS's update reading would have read the tabs' own interval check rather than the reload, and the
Worker's would have ended on `--go-wait`'s bound with a failed verdict (P-6). The other mechanism findings: a refused seat holder lands in the
lobby, since every table consumer leaves on an `End` (P-7); M27's catcher was vacuous, since the `?` on `events_at`
drops the slot (P-8); the character rule passes the Hangul fillers and U+2800 and refuses U+200C (P-9); M38's named
catcher cannot see the order it exists for (P-12); the retired names had six more sites, two of them intra-doc links
that break (P-13); the lane held three passages Step 7 did not list (P-14); the fold had changed S2a's D1 reading into
a different property (P-15); and the woken re-read never climbs the ladder, so a 429 arm there would end live updates
on a transient edge 429 (P-39). The run-as-written class was the usual one: cells with no fixture, a `Debug` bound, a
`cfg` attribute, two uncapped constructors, a shadowed comparison value, an undeclared export, a cell in the wrong
file, a whole-file YAML block that would have dropped the file's 100 comment lines, `update.sh` silently undoing the
README's hardening, a pathspec that misses `web/src/wire-script`, and step and line numbers the fold left behind.

**The adjudications, decided by the orchestrator (the owner's standing word of 2026-10-11, "my instinct is to use the
suggestions you make"; each took the assembler's recommendation):**
- **AD-1 (a):** writes refuse Cc, Cf, Zl and Zp; reads keep the old rule through `check_stored_name`, so a stored
  document reads under the rule it was written under (C-1).
- **AD-2 (c):** echo-equality — each quoted span, read back through the escapes, against the claim cut as the echo
  cuts it, one equality per span; lens 2's prefix form, which loses a configuration value, is not used (C-2).
- **AD-3 (a):** a separate `ended` state drawn as `error ?? ended`; a ref guarding `take` would still lose to
  `copyLink` (C-3).
- **AD-4 (a):** the `*_salted` writers with the old ones as wrappers, so about fifteen test call sites stay (C-5).
- **AD-5 (a):** pin what the code does — the lobby with the sentence, as a 403 and the Worker's 4429 land — with no
  fourth `End` reason (C-7).
- **AD-6 (b):** the 429 at the stream's open alone; the re-read keeps the stream on a transient 429; M34 retired
  (C-39-signal).
- **AD-7 (a):** R-2's rule and its sentence kept; the module doc and D-4 say what passes and what U+200C costs (C-9).
- **AD-8 (a):** the three hardening flags in `update.sh` and README §4's `docker run`, so a self-hoster's update keeps
  them (C-11).
- **AD-9 (a):** `worker/RUNBOOK.md`'s seven recipe lines alone, its one definition moved up; `deploy/RUNBOOK.md`'s
  working `UA` stays (C-28).
- **AD-10 (a):** the R-8 row lands in guidingpath with this review's corrections, written by the orchestrator, since
  guidingpath's current state says the first window is still owed (C-62).
- **AD-11 (a):** #52's and #54's titles and bodies rewritten before the merges, inside Step 8's own authorization
  (C-55).
- **AD-12 (a):** §9.6's missing S2a paragraph filled at this close, as spec correction 13 (C-58).
- **AD-13 (b):** the nonce redrawn until alphanumeric, which costs nothing and removes the question of `+` and `/` at
  the edge's parser; the cell's regex unchanged (Task 9 Step 3).
- **AD-14 (a):** one route assertion that a withheld requester still reads a pre-search refusal, `human_seat` (Task 6
  Step 5).
- **AD-15 (a)→(b):** B1's public comparator is a finished vs-bot table on the NAS, else the search's recorded
  1 ms–4.4 s (C-61).
- **AD-16 (a):** the headers refusal's true message, pinned by an assertion (C-47).
- **AD-17 (a):** the stalled-body probe recorded with its body, so the server's 408 is told from an edge's answer
  (C-63).
- **AD-18 (b):** `Slot::drop`'s lock poison-tolerant in `games::enter`'s form (`enter` itself takes a `Mutex<()>`), so
  a poisoned lock never panics inside a drop (Task 1 Step 3).
- **AD-19 (a):** the 512 MiB sentence reworded to the recorded readings, with one capped-replay `docker stats` reading
  at the window (C-56b).

**For the owner at the close (packet §5; nothing in this plan says the owner has ruled):** **O-1**, R-4's "a span
equal to the raw claim": the plan reads it as equal to the claim as the layer's echo writes it (AD-2), the only form in
which a cut or escaped claim appears in a sentence, where the literal reading leaves such a claim in the persisted line.
**O-2**, R-2's categories: the rule refuses U+200C, which Persian and several Indic scripts spell with, and passes the
default-ignorable letters (the Hangul fillers, U+034F, U+17B4, U+17B5) and U+2800; the plan takes (i), both accepted as
recorded (AD-7), with (ii) admitting U+200C and (iii) extending the rule to the seven letters as the alternatives.

**The fold's audit (packet §6).** S2a's Task 9 Step 9(a)–(e), read item by item against Step 10, found the fold
lossy: the opening precondition (Claude in Chrome in both profiles), the quota guard's 80,000 HOLD, (a)'s `PINNED`
read, the second SW navigation's reading, the Access frame caveat, (b)'s update-reading criteria, (c)'s rows 108 and
119 with their readings, the `rev7/abandoned/` removal, `warp-cli`, the fallback cookie file and (d)'s records were
lost, and are restored above (C-59, with lens 4's ranges); `go` and `done` were misplaced on the NAS and missing on the
Worker (C-6); D1 had become a different reading and is S2a's again (C-39); B1's table recipe and A's read were lost
and are back, with a public comparator (C-61); and lens 4's own patch dropped (d)'s "every probe table deleted", which
the preamble now carries. The sentences that still assumed two windows are corrected at their sites: the shipped
`CSP_MODE` comment's past tense (C-60), the spec's §2.2 CSP row and §9.7's "second window" twice (correction 12), the
lane's §2 and §4 passages (C-14), guidingpath's current state (C-62) and PR #52's title (C-55).

**Unreached (packet §7):** 17 claims no offline check reaches, each with the window-time read that settles it in the
packet: cloudflared's `access` schema and its 403, the terminal hostname's Access application, Cloudflare's quoted
guidance, the edge copying the nonce onto the script it injects and its parser's alphabet, a second inline surface,
`cf-internal`'s two members and the terminal image's `wget`, the NAS kernel taking the `HostConfig`, the 512 MiB
margin, the tunnel freeing a closed tab's slot, the 408 through the edge, the edge's answer to a stream 429, a stored
format character on the NAS volume, a public `why` to time, and the release builds, the pool after the bump, the
image smoke under Docker and the roster.
