# Security S2b — the Worker's door: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. Every code task subagent-driven — **Opus** for every reviewer, **Opus** for every implementer but
> Task 8, whose brief carries the whole judgment and runs **Sonnet** (the owner's ruling X3, 2026-10-11) — and a
> **Mythos** orchestrator that conducts the plan review at the top (the owner's four-Sonnet-lens Workflow with an
> Opus assembler, ruling R-1 below), the pre-flight (Tasks 1 and 2's cells executed and every figure measured — the
> owner's ruling X4; NO mutants: those are the implementers' probes, the reviewers' and the after review's, and the
> overlap among those three is accepted by design — the owner's rule of 2026-10-12) and the whole-branch review at the
> end ITSELF, on exports; the suites, the documents' close, the
> SDD ledger, the branch and the PR against the stack are the orchestrator's too (Task 10). Steps use checkbox
> (`- [ ]`) syntax. **Read the spec first** (`docs/plans/2026-10-08-security-remediation-spec.md`: §1 the rulings
> R-SEC5, R-SEC6, R-SEC8(c), R-SEC8(d) and R-SEC10, §2.2 the rows B5, B6, C1, C2, B12's Worker half and the Lows'
> row for D3, D23 and D24, §3.3, §3.5, §6 the fences, §9.2 the stacked branch, §9.6 the three-cycle split), then
> this plan's §"Names shared across tasks" — an implementer sees only its own task, and that section is how it learns
> what its neighbours produce. Every `file:line` here was read at `security-s2`'s head `85c8dd02` (S2a's close,
> 2026-10-12); **find by content once an earlier task has moved a file.** The implementer PROBES the brief's code
> before its own review and reports `DONE_WITH_CONCERNS` when a sentence here proves false. **This plan was reviewed
> and corrected at the source on 2026-10-12** (the four-lens Workflow and its Opus packet; §"Review record" at the
> end): every block a lens measured red as first drafted carries its measured-green form here, and a block marked
> "(proposed)" in the record is one nobody has run yet — the implementer probes it first.

**Goal:** Close the Worker's door rows of the security review — the operator's expire and force answering a
cross-site simple POST (B5), the dev door claiming any real address and standing open in production (B6), the open
door admitting an address-shaped claim onto the key space Access verifies (C1, with D24 collapsing into it), creates
and writes unthrottled and names the store refuses instantiating objects (C2), sockets uncapped per caller and per object (D3),
the Worker's archive object taking saves with no quota (B12's Worker half), and the shell's logs persisting addresses
beside unredacted query strings (D23) — plus the cells and sentences the earlier cycles left owed: the fully escaped
configuration key, the crafted name equal to another address's segment, the 48-character cut in WASM against native,
S2a's four parked sentences and its `with_headers` message, and one new pin that the outbox shows a 429's sentence.
No version moves and no window opens in S2b: by the owner's ruling X2 (two windows, two versions) S2b's code goes
out at S2's wrap, in 0.6.3, and this cycle's live acceptance is read there; the next deploy is S2a's own window,
`v0.6.2`.

**Architecture:** Seven mechanisms, each in the host that owns the row. (1) The Worker's `answer` refuses, before
identity, every unsafe method whose `Origin` is present and not its own or whose `Sec-Fetch-Site` reads `cross-site`
or `same-site`, and requires `application/json` on the operator's POST; the route's name grammar becomes the store's
own, restated and pinned to the committed `id_cases.json`. (2) Two Rate Limiting bindings, creates and writes, keyed
by a digest of the caller under `worker` and by the connecting IP (creates) or IP plus claim (writes) under `open`,
answer 429 with `Retry-After` before any object is reached; production carries the figures, the pool a lenient copy,
and the guard pins production's block to its literals. (3) A socket is admitted only under a per-caller and a
per-object cap, on tables and in the lobby, refused in the shape the client already reads: accepted, an `end` frame
carrying a sentence, a 4429 close. (4) The archive object lists the caller's share in R2 before each save and hands
the listing to the module, whose receipt archive applies the production quota's own arithmetic, so the 507 and its
figures are byte-identical to the NAS's and the object stays storage-free. (5) The layer's mode rule gains a named
`ClaimPolicy`: `NamesOnly` in every production constructor refuses an address-shaped key under the open door with one
sentence, `Diagnostic` keeps the fixtures and the corpus walk standing; the client's name step and the new-table form
refuse an `@` under the open door before any request; the Worker module runs `NamesOnly`, and the Worker shell's one
change collapses a claim's space runs so one name is one Archive object. (6) The dev door admits a `.test` address only;
the production file ships the dev door OFF and a dev file opens it around a live tier; the guard holds the dev file
as production's sibling with that one difference and every unlisted worker-door file to production exactly. (7) The
shell's operator lines carry a digest, its archive lines a key whose readable half is dropped (the `record_key` field
and `logOutcome`'s `detail` both) and the Archive object names itself by a digest, and every deployable
configuration redacts query strings from persisted logs.

**Tech Stack:** TypeScript and node ESM (`worker/src`, `worker/scripts`, `worker/test`, `web/src`, `web/e2e`; the
vitest, node and Playwright suites; the Workers pool `@cloudflare/vitest-plugin` 1.2.3 over miniflare, whose
ratelimit plugin counts per key); Rust 1.97.1 (`crates/yokohama-web`: `identity.rs`, `lib.rs`, `archive.rs`,
`tests/start_refusal.rs`; `crates/yokohama-worker`: `receipt.rs`, `lib.rs`, `config.rs`); wrangler 4.136.3 (the
`ratelimits` block, GA since 2025-09-19; `observability.redact_query_string`, in the installed schema); the committed
wire corpus (UNMOVED: no new contract file, no version field); the documents (`docs/web-lane.md`, the spec's §9.7,
`worker/RUNBOOK.md`, `worker/README.md`, `deploy/README.md` §2's name rule, the ledger).

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` — §2.2's rows B5, B6, C1, C2 (the Worker half),
B12 (the Worker's object), and the Lows' row for D3 (the socket cap), D23 and D24; R-SEC5 (every control holds with
Access removed), R-SEC6 (the public Worker is its own deployment), R-SEC8(c) (names stay names under `open`),
R-SEC8(d) (the dev door off by default); §3.3; §5.1–§5.2; §6 the fences; §9.2 the stacked branch; §9.6's three-cycle
split and correction 5 (D24 collapses into C1), 6 (B6's token `accepted`) and 9 (the crafted-name cell is S2b's). The
S1 stack review's F-40 (an unlisted worker-door `--config` file's sibling rule) and HI-F3's own probe (the fully
escaped key, committed as rows). The corrections this plan makes to the spec are §"Spec corrections (to be recorded
as §9.7)" below — recorded forward by Task 9.

## The owner's rulings for S2b (2026-10-12), and what each binds

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

## The decisions this plan takes (the orchestrator's; the owner read D-1–D-7 at the brainstorm, 2026-10-12)

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
  deploy runs the deploy guard and `wrangler deploy --config <file>` by hand, and the secrets guard from the second
  deploy on; the recipe is written where the guard's cost is recorded (`worker/RUNBOOK.md` §2, `worker/README.md`
  §9.3). No code: wrangler's "Worker not found" is a sentence, not a contract.
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

## Spec corrections (to be recorded as §9.7 by Task 9, dated 2026-10-12)

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
9. **The first-deploy path of a new Worker is a recipe** (D-7; the owner's R-6), not a guard rule.
10. **D24 collapses by the grammar**: a plain name's alphabet is ASCII, so once an address is refused under `open`
    every claim is a legal header; the client's name step refuses the rest before any request (D-2).
11. **D3's Worker half is the socket cap** (D-4); the stream cap and the body-read timeout are the NAS's, S2c's.
12. **The fifteen-gate roster is not owed by S2b** (D-12): core and the workspace version do not move.

## Global Constraints

- **The fences (spec §6; D-12):** no byte under `crates/yokohama-core`, `crates/yokohama-cli`, `crates/yokohama-oracle`,
  `crates/yokohama-difftest`, `crates/yokohama-bridge`, `crates/yokohama-wasm` or `crates/gen-data`; `SEMANTICS_REV`
  stays **7**; the wire corpus (`web/src/wire/*.json`, `web/src/wire-script/*.json`) is UNMOVED — `cargo test -p
  yokohama-web --test wire_fixtures` runs clean without `UPDATE_WIRE_FIXTURES`; no bot behaviour moves.
- **No version bump, no window, no roster** (X2; D-12): the workspace stays `0.6.2`; the per-edit suites and the web
  lane's rows (`docs/web-lane.md` §2) run at the final head; the Worker's module is rebuilt (`npm run
  build:module:debug`) before every pool run once Task 4 has moved `crates/yokohama-worker` (Tasks 1–3 may run it
  too: the build is idempotent, 25.6 s, byte-identical output).
- **Every figure at its literal, with its defence in a concise in-code comment** (the owner's S1b rule): `120`/`10`
  and `10`/`60` for the two limits; `"60"`/`"10"` for the two `Retry-After`s; `7301`/`7302`; `8`/`64`/`8`/`1024` for
  the sockets; `256`/`128 << 20` unchanged in the layer; `4429` for the close code; `12` for the digest's hex length.
- **Every caller value a shell sentence repeats is bounded** (the echo rule's spirit on the TS side): no new shell
  sentence repeats a caller value at all — the sentences this plan adds are constants.
- **The pool's two projects both run** (`npm test` = `worker` and `open`), and `npm run test:scripts`, `npm run
  typecheck` and both Rust crates' suites run after every task; the open project's cells claim NAMES (D-2).
- **Mutants run three times, by design, and never a fourth:** the implementer's own probe of its brief, the task
  reviewer's battery on an export, and the after review's battery over every named mutant. The overlap among those
  three is accepted (the owner, 2026-10-12): the plan does not dedupe it, no step waits on it, and the pre-flight runs
  NONE — it executes cells and measures figures (Task 10 Step 1).
- **Reviewers run their mutants on an EXPORT** (`git archive`), never in the implementer's tree; one writer in the
  tree at a time; every agent writes its report to a file and replies with the path and three lines; no agent runs
  `rm -rf`, `rm -r` or a bulk `rm -f`, arms a Monitor or waits on an event; **no agent makes any call to the
  Cloudflare account, read-only included** (the S2a plan review's conduct finding).
- **Every ruling here is transcribed, never paraphrased**, and a sentence an implementer finds false is a
  `DONE_WITH_CONCERNS`, corrected in THIS PLAN first and then transcribed (the S2a run's rule).

## Review Focus

Five inputs the spec implies and no task's tests exercised as first drafted — each now has its cell in the task
named:

1. **`Origin: null` on an unsafe method** (a sandboxed frame or a cross-origin redirect's resubmission) must be
   refused, since `null` is not this origin — Task 1's cross-site cell carries the row.
2. **A connecting IP absent under `open`** (the pool, a misconfigured edge) must key by the literal `unknown`
   rather than throw or skip the limit — Task 2's key cell carries the row.
3. **A listing of exactly 1,000 objects that is NOT truncated** — 500 records and their tables in the cell — is over
   the quota, and a truncated one is over by construction — Task 4's arithmetic cells carry both rows.
4. **A socket the object has just closed** is returned by `getWebSockets` in the SAME synchronous turn as its
   `close()` (readyState 2) and is gone by the next request in the pool (measured); a client-initiated `close` drops
   the count within one RPC and delivers no `close` event to the client. So the cap cell closes a socket from the
   client side and POLLS, bounded, until the count falls, then opens again: it asserts the slot frees, never an order
   against the close's completion, which no pool event marks. A close awaiting a peer that never answers stays CLOSING for
   as long as the peer likes (measured at Task 3's review), so the caps count OPEN sockets only (D-4, corrected) —
   Task 3's cap cells carry both rows.
5. **An address claim with a `.test` suffix under the OPEN door** must be refused like any address (the `test/` root
   is the dev door's and a server game's, never the open door's) — Task 5's layer cell carries the row.

## Execution shape

- **Branch:** `security-s2b`, cut from `security-s2`'s head `85c8dd02` (X1), at the start of the run; the plan and
  the board are its first commit; the pre-flight's corrections its second; Tasks 1–5, 5b, 6–9 follow, one implementer in the
  tree at a time; the PR targets `security-s2` and merges at S2's wrap.
- **Order:** Task 10 Step 0 (the owner's plan review, this session) → Step 1 (the pre-flight: Tasks 1 and 2's cells
  on an export, every figure measured, no mutants) → Tasks 1–5, 5b, 6–9 → Step 4 (the suites and the lane's rows) →
  Step 5 (the after review and its one wave) → Step 6 (the close) → Step 7 (the PR) → Step 8 (the hand-off into S2c).
- **Exports:** a `git archive` export runs `git init && git add .` (a throwaway local repository) before
  `npm run test:scripts`: `lane-variables.test.mjs` reads `git ls-files`, and a bare export reads three failures
  (558 passed, 3 failed where a git tree reads 561).
- **Tiers:** Opus implementers and reviewers throughout; Task 8's implementer Sonnet (X3).
- **The SDD ledger:** `docs/temp/security-review-2026-10-08/s2b-sdd/progress.md`, with every brief, report and
  review beside it; the plan review's packet under `s2b-plan-review/`.

## Names shared across tasks

| name | where | what it is |
|---|---|---|
| `CROSS_SITE` | `worker/src/index.ts` | the 403's sentence: `this request is for the site's own pages` |
| `crossSite(method, headers, origin)` | `worker/src/index.ts` (exported, pure) | true when the method is unsafe and the request is another site's (D-9) |
| `ADMIN_NEEDS_JSON` | `worker/src/admin.ts` | the 400's sentence for the operator's POST without `application/json`: `` Expected request with `Content-Type: application/json` `` (the layer's own words) |
| `isJsonEssence(contentType)` | `worker/src/admin.ts` (exported, pure) | the MIME essence is `application/json`, parameters dropped, case folded |
| `isSafeId(id)` | `worker/src/route.ts` (exported) | the store's grammar, restated; `routableName` is REMOVED and every caller uses this |
| `ID_MAX` | `worker/src/route.ts` (exported) | `64`, the longest id in bytes |
| `CREATE_LIMIT`, `WRITE_LIMIT` | `worker/src/env.ts`, every `wrangler*.jsonc` | the two `RateLimit` bindings |
| `limitKey(door, identity, ip, kind)` | `worker/src/limits.ts` (exported, pure, async) | the key a request is counted under (D-3); `kind` is `create` or `write` |
| `limited(req, target, identity, door, limits)` | `worker/src/limits.ts` (exported) | the 429, or `undefined` to proceed; `limits` is `ShellConfig.limits` |
| `limiterFor(method, target, limits)`, `Limiter`, `Limits` | `worker/src/limits.ts`, `worker/src/config.ts` | which binding an unsafe method spends; the binding's one method; the two bindings as the configuration reads them |
| `TOO_MANY`, `RETRY_AFTER_CREATE`, `RETRY_AFTER_WRITE` | `worker/src/limits.ts` | `too many requests from you just now — wait a moment and try again`; `"60"`; `"10"` — each kind's own window |
| `IP_HEADER`, `UNKNOWN_IP` | `worker/src/limits.ts` (exported) | `"cf-connecting-ip"`; `"unknown"` |
| `RATE_LIMITS` | `worker/scripts/deploy-guard.mjs` (exported) | production's expected `ratelimits` block as a value |
| `SOCKETS_PER_CALLER`, `SOCKETS_PER_TABLE` | `worker/src/table/sockets.ts` | 8 and 64 |
| `LOBBY_PER_CALLER`, `LOBBY_TOTAL` | `worker/src/house.ts` | 8 and 1,024 (pinned at their literals in a pool cell of `caps.test.ts`: the pool imports `house.ts`, node cannot) |
| `CLOSE_TOO_MANY` | `worker/src/frames.ts` | the close code 4429 |
| `TOO_MANY_OWN_TABLE`, `TOO_MANY_TABLE`, `TOO_MANY_OWN_LOBBY`, `TOO_MANY_LOBBY` | `worker/src/frames.ts` | the four refusal sentences the `end` frame carries |
| `capVerdict(own, total, caps)` | `worker/src/table/sockets.ts` (exported, pure) | `own`, `total` or `undefined`: which cap a new socket would cross |
| `callerTag(user)` | `worker/src/redact.ts` (exported, async) | `u:` and SHA-256 of the caller in hex; the socket tag and the limit key's caller half |
| `callerDigest(address)` | `worker/src/redact.ts` (exported, async) | the first twelve hex of SHA-256 over the address |
| `DIGEST_HEX` | `worker/src/redact.ts` (exported) | `12` |
| `redactKey(key)` | `worker/src/redact.ts` (exported, pure) | the record key with `browser/<readable>-<12hex>/` read as `browser/<12hex>/` |
| `Incoming.callerTag` | `worker/src/table.ts`, `worker/src/house.ts` | the caller tag, present on an upgrade |
| `share_prefix()`, `set_share(listing)` | `crates/yokohama-worker/src/lib.rs` (exports), `gen/factory.js` | the share's prefix, or `None`; the listing handed to the module |
| `SHARE_UNCOUNTED` | `crates/yokohama-worker/src/receipt.rs` | `the caller's share was not counted before this save` |
| `Listing { truncated, objects: [{ key, size }] }` | `receipt.rs` (serde, `deny_unknown_fields`) | what `set_share` parses |
| `ClaimPolicy { NamesOnly, Diagnostic }` | `crates/yokohama-web/src/identity.rs` | the open door's key rule (D-2) |
| `ADDRESS_NOT_A_NAME` | `identity.rs` | `under the open door a name is a name — an address is not accepted here: choose a plain name` |
| `AppState.claims`, `with_claim_policy` | `crates/yokohama-web/src/lib.rs` | the policy the extractor resolves with |
| `openDoorVerdict(raw)` | `web/src/nameRule.ts` (Task 5b) | the open door's refusal of any raw `@` FIRST, then `normalizeName` |
| `ADDRESS_NOT_A_NAME_HERE` | `web/src/nameRule.ts` (Task 5b) | the client's own refusal sentence: `Under the open door a name is a name — an address is not accepted here. Choose a plain name.` |
| `claimedIdentity(raw)` | `worker/src/identity.ts` (Task 5) | trimmed, ASCII-lowercased, each run of spaces collapsed — the key grammar's `typed_form`, so every spelling of a key names ONE Archive object |
| `DEV_FILE`, `DEV_DIFFERS` | `worker/scripts/deploy-guard.mjs` (module-private constants) | `wrangler.dev.jsonc`; `{ YOKOHAMA_DEV_DOOR: "on" }` |
| `isTestAddress(email)` | `worker/src/identity.ts` (exported) | an address ending `.test` (the receipt archive's rule) |

## File structure

- **Create:** `worker/src/limits.ts` (+ `limits.unit.test.ts`), `worker/src/redact.ts` (+ `redact.unit.test.ts`),
  `worker/src/table/sockets.unit.test.ts`, `worker/wrangler.dev.jsonc`, `worker/test/door.test.ts` (B5 and the
  limiter through `handle`), `worker/test/caps.test.ts` (the socket caps).
- **Modify:** `worker/src/index.ts`, `admin.ts`, `route.ts` (+ `route.unit.test.ts`, `admin.unit.test.ts`),
  `identity.ts` (+ `identity.unit.test.ts`; Tasks 5 and 6), `env.ts`, `config.ts` (+ `config.unit.test.ts`),
  `marker.unit.test.ts` (Task 2), `table.ts`, `table/sockets.ts`, `house.ts`, `archive.ts` (Tasks 4 and 7),
  `frames.ts` (+ `frames.unit.test.ts`), `r2.ts` (+ `r2.unit.test.ts`), `table/lifecycle.ts`, `table/archiving.ts`
  (Task 7); `worker/wrangler.jsonc`, `wrangler.open.jsonc`, `wrangler.test.jsonc`, `wrangler.open.test.jsonc`,
  `worker/package.json`; `worker/scripts/deploy-guard.mjs` (+ `.test.mjs`), `secrets-guard.mjs`, `factory-glue.mjs`,
  `factory.d.ts.template`, `live-replay.test.mjs`, `live-replay.mjs` (Task 1: one log sentence; its logic UNCHANGED, D-8 withdrawn), `worker/test/worker.test.ts` (Task 1: two sentences),
  `replay/send.mjs`; `worker/test/sockets.test.ts`, `archive.test.ts`, `operator.test.ts` (Tasks 1 and 7),
  `gates.ts` (Task 4), `open/door.test.ts`; `crates/yokohama-worker/src/receipt.rs`, `lib.rs`;
  `crates/yokohama-web/src/identity.rs`, `lib.rs`, `archive.rs`, `main.rs` (its doc, Task 5),
  `tests/start_refusal.rs`, `tests/wire_fixtures.rs` (one call site, Task 5); `web/src/nameRule.ts` (+ `.test.ts`),
  `SignIn.tsx` (+ `SignIn.test.tsx`), `App.test.tsx`, `NewGame.tsx` (+ `NewGame.test.tsx`), `web/e2e/harness.ts`
  and the twelve specs that sign in by address (all Task 5b), `outbox.ts` (+ `.test.ts`), `web/scripts/headers.mjs`,
  `web/e2e/harness.node.test.ts`; the documents (Task 9: `docs/security-ledger.md`, the spec, `docs/web-lane.md`,
  `worker/RUNBOOK.md`, `worker/README.md`, `deploy/README.md`). `worker/scripts/factory-glue.test.mjs` is NOT touched
  (it derives everything from `EXPECTED`), nor is `deploy/RUNBOOK.md` (it quotes no playbook).
- **Delete:** `worker/wrangler.gate.jsonc`.

## Task order and why

1. **B5 and the name grammar** — the Worker's `answer` is the file every later shell task touches; its refusal order
   is pinned first so the limiter (Task 2) slots behind it.
2. **C2's binding** — needs Task 1's `isSafeId` (a refused name never reaches the limiter's object call) and the
   bindings every configuration then carries; the pre-flight's figures fix its literals.
3. **D3's socket caps** — independent of 1–2 but shares `redact.ts`'s `callerTag` with Task 2's key, so it follows.
4. **B12's Worker quota** — the module moves (`npm run build:module:debug` from here on); independent of 1–3.
5. **C1 with D24** — the layer, the Worker module and the open pool project (plus the shell's space-run collapse,
   whose race cell needs Task 4's quota); the open pool project's cells move to names here.
   **5b. C1's client and the Playwright identities** — the name step and the new-table form, the two web cells C1
   inverts, and the e2e harness's target-dependent identities; its gate is the local Playwright tier green. Split
   from Task 5 at the plan review (AD-1) so each stays reviewable; Task 5's suites are green WITHOUT it, since the
   server-side change breaks no jsdom cell.
6. **B6** — the configurations and the guard; after Task 2 so the dev file is born with the bindings; the guard's
   door rules of D-1 and F-40's sibling rule land here (check 13 is Task 2's, beside the bindings; check 14 is
   Task 7's, beside the redaction).
7. **D23** — the digests ride `redact.ts` (Task 3) and the configuration (Task 6).
8. **The owed cells and sentences** — Sonnet; touches tests, comments and one two-line arm in `outbox.ts`.
9. **The documents** — after every code task.
10. **The orchestrator** — the plan review (Step 0, this session), the pre-flight, the dispatches, the suites, the
    after review, the close, the PR, the hand-off.

---

### Task 1: B5 — the cross-site refusal and the operator POST's content type; the route's name grammar — Opus (B5; C2's name rule; decisions D-9, D-11; Review Focus 1)

**Files:**
- Modify: `worker/src/index.ts` (`answer`, the block after the upgrade's `Origin` check; `objectFor`'s create arm)
- Modify: `worker/src/admin.ts` (`ADMIN_NEEDS_JSON`, `isJsonEssence`; `adminRoute` imports `isSafeId`)
- Modify: `worker/src/route.ts` (`isSafeId` replaces `routableName`; `LONE_SURROGATE` and `TABLE_NAME_MAX_BYTES` go)
- Modify: `worker/scripts/replay/send.mjs` (`forceAll`'s header)
- Test: `worker/src/route.unit.test.ts`, `worker/src/admin.unit.test.ts`, `worker/scripts/live-replay.test.mjs`,
  `worker/test/operator.test.ts` (its POSTs gain the JSON type, Step 10b), `worker/test/worker.test.ts` (two sentences, Step 10c)
- Modify: `worker/scripts/live-replay.mjs` (one log sentence, Step 10)
- Create: `worker/test/door.test.ts` (the Worker's door through `handle`: B5 here, the limiter in Task 2)

**Interfaces:**
- Consumes: `handle`/`answer` (`index.ts`), `respond`/`errorBody` (`respond.ts`), `adminRoute`/`operatorGate`
  (`admin.ts`), the pool helpers `seedPlaying`, `tableStub`, `RED`, `BLUE` (`test/helpers.ts`), the token signer
  (`test/tokens.ts`), `web/src/wire/id_cases.json` (read from `../../web/src/wire/` in the node cell).
- Produces: `CROSS_SITE`, `crossSite(method, headers, origin)`, `ADMIN_NEEDS_JSON`, `isJsonEssence(contentType)`,
  `isSafeId(id)`, `ID_MAX` — the names Task 2 and the documents use; `worker/test/door.test.ts` as the file Task 2's
  limiter cells join.

- [ ] **Step 1: The name grammar's failing cells (node).** In `worker/src/route.unit.test.ts`: (a) line 2 becomes
  `import { abandons } from "./route";` — the block below brings `isSafeId`, `ID_MAX` and `route` in with its own
  import, and a second `route` import is a parse error; (b) delete the whole `describe("routableName", …)` block
  (its two cells pin the rule `isSafeId` replaces); (c) in the cell `/api/games/{id} and below go to the table named by
  the segment` the row `route("GET", "/api/games/caf%C3%A9/table")` now expects `{ kind: "house" }` (a percent escape
  is outside the store's alphabet). Then append (measured green at the plan review; it typechecks under workerd's
  types):

```ts
import { isSafeId, ID_MAX, route } from "./route";

// The project's types are workerd's: no node module and no `import.meta.url` (`fence.unit.test.ts` says why), so `node:fs`
// is imported through a variable specifier and typed here; the file runs in node (`npm run test:scripts`).
const FS = "node:fs";
const fs = (await import(/* @vite-ignore */ FS)) as { readFileSync(path: URL, encoding: "utf8"): string };

/** The store's verdicts, emitted by `wire_fixtures` (security S2a, D14): `[{ raw, safe }]`. */
const ID_CASES = JSON.parse(
  fs.readFileSync(new URL("../../web/src/wire/id_cases.json", (import.meta as unknown as { url: string }).url), "utf8"),
) as Array<{ raw: string; safe: boolean }>;

describe("isSafeId (security S2b, C2's name rule; decision D-11)", () => {
  it("agrees with the store's verdict on every committed row, both ways", () => {
    expect(ID_CASES.length).toBeGreaterThanOrEqual(39);
    for (const { raw, safe } of ID_CASES) expect(isSafeId(raw), JSON.stringify(raw)).toBe(safe);
    expect(ID_CASES.some((c) => c.safe) && ID_CASES.some((c) => !c.safe), "both verdicts are represented").toBe(true);
  });

  // Mutants: the leading-character test dropped (`_x` admitted); the suffix test case-sensitive (`x.TABLE`
  // admitted); the bound off by one (65 admitted); the alphabet widened to `%` (a percent-encoded segment admitted).
  it("refuses what the old routableName admitted and the store refuses: a space, a leading underscore, a reserved suffix, a percent escape, a dot segment", () => {
    for (const bad of ["a%20b", "_x", "x.table", "x.TABLE", "x.chat", "%C3%BC", "..", ".", "", "a".repeat(ID_MAX + 1)]) {
      expect(isSafeId(bad), JSON.stringify(bad)).toBe(false);
    }
    for (const good of ["sim-0123456789abcdef", "x", "evening.2", "x.tablet", "a".repeat(ID_MAX)]) expect(isSafeId(good), good).toBe(true);
    expect(ID_MAX).toBe(64);
  });

  it("routes a segment the store refuses to the house, raw and never decoded", () => {
    for (const bad of ["a%20b", "_x", "x.table", "%C3%BC", "..", "a".repeat(65)]) {
      expect(route("GET", `/api/games/${bad}/table`), bad).toEqual({ kind: "house" });
    }
    expect(route("GET", "/api/games/sim-0123456789abcdef/table")).toEqual({ kind: "table", name: "sim-0123456789abcdef" });
  });
});
```

- [ ] **Step 2: Run them to see the fail.** `cd worker && npx vitest run -c vitest.scripts.config.ts src/route.unit.test.ts` —
  `src/**/*.unit.test.ts` runs in node under `vitest.scripts.config.ts` (its `include`); the pool's `test/**/*.test.ts`
  glob never sees it. Expected: FAIL — `isSafeId is not a function` (two cells), the third new cell's assertion (`a%20b` routed to a
  table) and the flipped `caf%C3%A9` row: 4 failed, 25 passed.

- [ ] **Step 3: Replace `routableName` with `isSafeId`** in `worker/src/route.ts`:

```ts
/** The longest id, in bytes — every accepted id is ASCII, so in characters too (`store::is_safe_id`'s
 *  `id.len() <= 64`). */
export const ID_MAX = 64;

/** The suffixes the store keeps for its own documents (`RESERVED_ID_SUFFIXES`), refused in any case. */
const RESERVED_SUFFIXES = [".table", ".chat"];

/**
 * **The store's id grammar, restated** (security S2b, finding C2; decision D-11): `store::is_safe_id`
 * (`crates/yokohama-web/src/store.rs`) is the authority, `web/src/tableId.ts` restates it for the hash router, and
 * this restates it for the shell's routing, held both ways to `web/src/wire/id_cases.json` by
 * `route.unit.test.ts`. Arm by arm: not empty, at most 64 bytes, a letter or digit first, then letters, digits,
 * `.`, `_` and `-` only, and not spelling another document's suffix. The alphabet test comes before the suffix
 * test and makes the ASCII fold exact. The store's sixth arm, `is_the_house` (`_house`), is not restated: its
 * leading `_` fails the first-character test, and the fixture carries the row.
 *
 * Before this a segment of any text up to 64 bytes named a Table object (`routableName`), so `a%20b`, `_x`,
 * `x.table` and `%C3%BC` each instantiated an object and a module, then stored nothing (the review's W1-4). A name
 * the store refuses is the house's now, whose `dispatch` answers the layer's own 404 for an unusable id (security
 * S1c, B8), and no object is made for it.
 */
export function isSafeId(id: string): boolean {
  if (id === "" || id.length > ID_MAX) return false;
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) return false;
  const lower = id.replace(/[A-Z]/g, (c) => c.toLowerCase());
  return !RESERVED_SUFFIXES.some((suffix) => lower.endsWith(suffix));
}
```

  Delete `routableName`, `LONE_SURROGATE` and `TABLE_NAME_MAX_BYTES`; in `route()` the table arm reads
  `isSafeId(name) ? { kind: "table", name } : { kind: "house" }`; `admin.ts`'s `adminRoute` imports `isSafeId` for its
  `{id}`; `index.ts`'s `objectFor` create arm reads `if (named !== undefined && !isSafeId(named)) return house();`.
  Then `grep -rn "routableName(\|TABLE_NAME_MAX_BYTES\|LONE_SURROGATE" worker/src worker/test worker/scripts` prints
  nothing (`adminRoute`'s doc in `admin.ts` names `routableName` and is rewritten to `isSafeId`; `route.ts` named it only in
  the deleted function's own doc; the prose of `isSafeId`'s doc and the cell's title still name the old function by
  name, deliberately — hence the call form `routableName(` in the grep).

- [ ] **Step 4: Run the node cells.** Expected: PASS, every row.

- [ ] **Step 5: B5's failing cells (the pool).** Create `worker/test/door.test.ts`:

```ts
import { createExecutionContext } from "cloudflare:test";
import { env } from "cloudflare:workers";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { Archive } from "../src/archive";
import { ADMIN_NEEDS_JSON } from "../src/admin";
import type { Env } from "../src/env";
import { House } from "../src/house";
import type { KeySource } from "../src/identity";
import { CROSS_SITE, handle } from "../src/index";
import { Table } from "../src/table";
import { RED, seedPlaying } from "./helpers";
import { personClaims, signer, type Signer } from "./tokens";

// **The Worker's door** (security S2b): what `handle` refuses before any object is reached — a cross-site
// unsafe method (B5), the operator's POST without `application/json` (B5), and in Task 2 the rate limit (C2).
// Every request goes through `handle`, as the Worker's own `fetch` does. Storage and the house are per FILE
// (pool fact 5): every cell uses ids of its own.

const ORIGIN = "https://yokohama.test";
const FOREIGN = "https://evil.example";
const JSON_TYPE = { "content-type": "application/json" };
const BOSS = "boss@t.test";
const OPS: Env = { ...env, YOKOHAMA_WEB_ADMINS: BOSS };

let key: Signer;
beforeAll(async () => {
  key = await signer("k1");
});
const at = () => ({ teamDomain: env.ACCESS_TEAM_DOMAIN!, aud: env.ACCESS_AUD!, now: Math.floor(Date.now() / 1000) });
const person = (email: string) => key.sign(personClaims(email, at()));
const keys: KeySource = async () => key.set();

interface Sent {
  method?: string;
  headers?: Record<string, string>;
  body?: string | null;
  token?: string;
  env?: Env;
}

async function worker(path: string, sent: Sent = {}): Promise<{ status: number; text: string; headers: Headers }> {
  const headers = new Headers(sent.headers);
  if (sent.token !== undefined) headers.set("Cf-Access-Jwt-Assertion", sent.token);
  const req = new Request(ORIGIN + path, { method: sent.method ?? "GET", headers, body: sent.body ?? null });
  const res = await handle(req, sent.env ?? env, createExecutionContext(), keys);
  return { status: res.status, text: await res.text(), headers: res.headers };
}

/** Which objects `f`'s requests reached: a spy on each class's `fetch`, on the PROTOTYPE (pool fact 1). */
async function reaching<T>(f: () => Promise<T>): Promise<{ result: T; reached: number; tables: number }> {
  const spies = [Table, House, Archive].map((c) => vi.spyOn(c.prototype, "fetch"));
  try {
    const result = await f();
    return { result, reached: spies.reduce((n, s) => n + s.mock.calls.length, 0), tables: spies[0]!.mock.calls.length };
  } finally {
    for (const s of spies) s.mockRestore();
  }
}

describe("B5 — an unsafe method from another site is refused before identity", () => {
  // Mutants: the `Sec-Fetch-Site` arm dropped (the no-Origin cross-site row passes); `startsWith` for the origin
  // compare (`https://yokohama.test.evil` passes); the safe-method set emptied (the GET row is refused); the check
  // placed after the operator gate (cell 1's operator rows read the gate's 403) or after identity (the key-source cell).
  it("refuses a bodyless, a form and a text/plain POST from a foreign origin, and a cross-site or same-site sender with no Origin, reaching no object", async () => {
    const id = "door-b5-foreign";
    await seedPlaying(id);
    const shapes: Array<[string, Record<string, string>, string | null]> = [
      ["no body", { Origin: FOREIGN }, null],
      ["a form", { Origin: FOREIGN, "content-type": "application/x-www-form-urlencoded" }, "force=1"],
      ["text/plain", { Origin: FOREIGN, "content-type": "text/plain" }, "{}"],
      ["JSON from a foreign origin", { Origin: FOREIGN, ...JSON_TYPE }, "{}"],
      ["a sibling host", { Origin: "https://yokohama-nas.moore.nyc" }, null],
      ["an origin that merely starts with ours", { Origin: `${ORIGIN}.evil` }, null],
      ["Origin: null", { Origin: "null" }, null],
      ["cross-site with no Origin", { "Sec-Fetch-Site": "cross-site" }, null],
      ["same-site with no Origin", { "Sec-Fetch-Site": "same-site" }, null],
    ];
    for (const [label, headers, body] of shapes) {
      const { result, reached } = await reaching(() =>
        worker(`/api/admin/tables/${id}/expire?force`, { method: "POST", headers, body, token: undefined, env: OPS }),
      );
      expect([result.status, result.text], label).toEqual([403, JSON.stringify({ error: CROSS_SITE })]);
      expect(reached, `${label}: no object reached`).toBe(0);
    }
    // Order, pinned two ways: a cross-site sender who is also nobody reads CROSS_SITE (never the object's 401), and a
    // refused sender's assertion is never verified (the key source is not asked — the next cell).
    const both = await worker(`/api/games/${id}/input`, { method: "POST", headers: { Origin: FOREIGN, ...JSON_TYPE }, body: '{"line":"1"}' });
    expect([both.status, both.text]).toEqual([403, JSON.stringify({ error: CROSS_SITE })]);
    // And the table stands.
    const table = await worker(`/api/games/${id}/table`, { token: await person(RED) });
    expect(table.status).toBe(200);
  });

  // The "before identity" pin (the M6.2 lesson, the two-defect probe): with no assertion `identify` returns NO_ONE
  // without asking the key source, so a 401 alone pins nothing — this cell asks the key source whether it was asked.
  it("never verifies the assertion of a cross-site sender: the key source is not asked", async () => {
    const id = "door-b5-keys";
    await seedPlaying(id);
    let asked = 0;
    const spy: KeySource = async () => { asked += 1; return key.set(); };
    const req = new Request(ORIGIN + `/api/games/${id}/input`, {
      method: "POST",
      headers: { Origin: FOREIGN, ...JSON_TYPE, "Cf-Access-Jwt-Assertion": await person(RED) },
      body: '{"line":"1"}',
    });
    const res = await handle(req, env, createExecutionContext(), spy);
    expect([res.status, await res.text(), asked]).toEqual([403, JSON.stringify({ error: CROSS_SITE }), 0]);
  });

  it("passes the site's own pages, bare clients and every read", async () => {
    const id = "door-b5-own";
    await seedPlaying(id);
    const own = { Origin: ORIGIN, "Sec-Fetch-Site": "same-origin", ...JSON_TYPE };
    const posted = await worker(`/api/games/${id}/input`, { method: "POST", headers: own, body: '{"line":"1"}', token: await person(RED) });
    expect(posted.status, "a page's own POST").toBe(200);
    // A chat POST answers 201 (`api.rs`'s `chat_post`; `sockets.test.ts` says the same).
    const bare = await worker(`/api/games/${id}/chat`, { method: "POST", headers: JSON_TYPE, body: '{"text":"hi"}', token: await person(RED) });
    expect(bare.status, "curl sends no Origin and no Sec-Fetch-Site").toBe(201);
    const none = await worker(`/api/games/${id}/chat`, { method: "POST", headers: { "Sec-Fetch-Site": "none", ...JSON_TYPE }, body: '{"text":"hi"}', token: await person(RED) });
    expect(none.status, "a user-initiated send is not a page's").toBe(201);
    const read = await worker("/api/health", { headers: { Origin: FOREIGN, "Sec-Fetch-Site": "cross-site" } });
    expect(read.status, "a GET is never refused here").toBe(200);
    const head = await worker("/api/health", { method: "HEAD", headers: { Origin: FOREIGN } });
    expect(head.status, "nor a HEAD").toBe(200);
  });
});

describe("B5 — the operator's POST needs application/json", () => {
  // Mutants: the essence compared whole (`application/json; charset=utf-8` refused); the gate dropped (a bare POST
  // expires); the gate placed before the operator gate (a stranger's bare POST reads this 400, not the gate's 403).
  it("refuses the expire without the type, in the layer's own words, and takes it with the type, parameters and case folded", async () => {
    const id = "door-b5-json";
    await seedPlaying(id);
    const boss = await person(BOSS);
    for (const [label, headers] of [["no type", {}], ["text/plain", { "content-type": "text/plain" }], ["a form", { "content-type": "application/x-www-form-urlencoded" }]] as Array<[string, Record<string, string>]>) {
      const { result, reached } = await reaching(() => worker(`/api/admin/tables/${id}/expire`, { method: "POST", headers, token: boss, env: OPS }));
      expect([result.status, result.text], label).toEqual([400, JSON.stringify({ error: ADMIN_NEEDS_JSON })]);
      expect(reached, `${label}: no object reached`).toBe(0);
    }
    // A stranger meets the gate first: the type is asked of an operator.
    const stranger = await worker(`/api/admin/tables/${id}/expire`, { method: "POST", token: await person(RED), env: OPS });
    expect(stranger.status).toBe(403);
    const typed = await worker(`/api/admin/tables/${id}/expire`, { method: "POST", headers: { "content-type": "Application/JSON; charset=utf-8" }, token: boss, env: OPS });
    expect(typed.status, "the essence, parameters dropped and case folded").toBe(204);
  });
});

describe("C2's name rule — a name the store refuses never makes a Table object", () => {
  it("routes an unusable segment and an unusable create id to the house, which answers the layer's refusal", async () => {
    const red = await person(RED);
    // ".." is normalised away by the URL (to /api/table) before routing: route.unit.test.ts holds it.
    for (const bad of ["a%20b", "_x", "x.table", "%C3%BC", "a".repeat(65)]) {
      const { result, tables } = await reaching(() => worker(`/api/games/${bad}/table`, { token: red }));
      expect(tables, `${bad}: no table object`).toBe(0);
      expect(result.status, bad).toBe(404);
    }
    const body = { players: 2, colors: ["Red", "Blue"], start_seat: 0, id: "x.table", seats: [{ user: RED }, { open: true }] };
    const { result, tables } = await reaching(() => worker("/api/games", { method: "POST", headers: JSON_TYPE, body: JSON.stringify(body), token: red }));
    expect(tables, "a create naming an unusable id reaches no table object").toBe(0);
    expect([result.status, result.text], "the layer's own refusal of the id (probed at the plan review: 400, its sentence, echo-bounded)").toEqual([
      400,
      JSON.stringify({ error: '"x.table" is not a usable game id' }),
    ]);
  });
});
```

- [ ] **Step 6: Run the pool to see the fails.** `cd worker && npm run build:module:debug && node scripts/test-assets.mjs && npx vitest run test/door.test.ts`
  (`test-assets.mjs` is `npm test`'s `pretest`, which writes `test/fixtures.gen.ts`; a bare `npx vitest` skips it).
  Expected: FAIL — `CROSS_SITE` and `ADMIN_NEEDS_JSON` import as `undefined` (an ESM import of a missing name is no
  error: the cells fail as assertions against `'{}'`); the foreign rows answer the operator gate's
  403 (`this address is not an operator here`: they carry no token), not 204.

- [ ] **Step 7: The refusal in `answer`.** In `worker/src/index.ts`, place `SAFE_METHODS`, `CROSS_SITE` and
  `crossSite` at MODULE level beside `FOREIGN_SOCKET` (`index.ts:40`; an `export` is illegal inside a function);
  only the one `if` line below goes in `answer`, after `const origin = …`'s upgrade check and before
  `const config = shellConfig(env);`:

```ts
/** The methods a browser sends without a preflight that only READ: `crossSite` never refuses one. */
const SAFE_METHODS: ReadonlySet<string> = new Set(["GET", "HEAD"]);

/** The 403 an unsafe method from another site answers (security S2b, finding B5; host-written). */
export const CROSS_SITE = "this request is for the site's own pages";

/**
 * **Whether an unsafe request is another site's** (security S2b, finding B5; decision D-9). Every layer POST
 * parses `application/json` by MIME essence, so a cross-site page cannot reach one without a preflight nobody
 * answers; the operator's expire and force were the shell's own, answered on method, path and query alone, and
 * a bodyless, form or text/plain POST is a CORS simple request a page sends with ambient authentication — the
 * `CF_Authorization` cookie, or the owner's WARP identity, which no cookie attribute touches. So, for every method
 * but GET and HEAD: an `Origin` present and not this origin — `null` included, a sandboxed frame's or a
 * redirected sender's — or a `Sec-Fetch-Site` of `cross-site` or `same-site` (a sibling `*.moore.nyc` host) is
 * refused. A request with no `Origin` and no `Sec-Fetch-Site` is not a page's — curl, Node's `fetch`, the live
 * replay, Playwright's request context — and passes; a page's own `fetch` carries this origin and `same-origin`.
 * Read BEFORE identity, like the upgrade's own `Origin` check above it: a refused sender learns nothing of who the
 * cookie names, and a sender that is both another site's and nobody reads this sentence (the order pin).
 */
export function crossSite(method: string, headers: Headers, origin: string): boolean {
  if (SAFE_METHODS.has(method)) return false;
  const sent = headers.get("Origin");
  if (sent !== null && sent !== origin) return true;
  const site = headers.get("Sec-Fetch-Site");
  return site === "cross-site" || site === "same-site";
}
```

  and in `answer`, directly after the upgrade's refusal: `if (crossSite(req.method, req.headers, url.origin)) return respond(403, errorBody(CROSS_SITE));`.
  Add a step "1b" to `answer`'s doc list between 1a and 2, in the doc's own voice, saying what the function's doc says;
  and the list's step 6a, which says a listed person's route is "an expire the table's", gains, after "(the
  orchestrator's ruling I-72),": "an expire whose type is not `application/json` the 400 `ADMIN_NEEDS_JSON`, in the
  layer's own words (security S2b, finding B5)," (the implementer's probe, Task 1's run: the doc was incomplete).

- [ ] **Step 8: The operator POST's type.** In `worker/src/admin.ts`:

```ts
/** The 400 the operator's POST answers without `application/json` (security S2b, finding B5; decision D-9): the
 *  layer's own sentence for the same gate (`api::parse_json`, axum's words), so an operator reads one refusal
 *  whichever host they hit. */
export const ADMIN_NEEDS_JSON = "Expected request with `Content-Type: application/json`";

/** Whether `contentType`'s MIME essence — the part before any `;`, trimmed, ASCII-folded — is `application/json`:
 *  the layer's gate (`api::parse_json`) also takes any `application/*+json`; the shell's is narrower on purpose —
 *  `application/json` alone, which is all an operator sends and all B5 needs, since a CORS-simple request carries
 *  neither. A form (`application/x-www-form-urlencoded`), `text/plain` and no type at all are refused, which is what
 *  makes the expire need a preflight no one answers. */
export function isJsonEssence(contentType: string | null): boolean {
  if (contentType === null) return false;
  const essence = contentType.split(";", 1)[0]!.trim();
  return essence.replace(/[A-Z]/g, (c) => c.toLowerCase()) === "application/json";
}
```

  and in `index.ts`'s admin branch, after `const admin = adminRoute(…)` and its `refused` arm:
  `if (admin.kind === "expire" && !isJsonEssence(req.headers.get("content-type"))) return respond(400, errorBody(ADMIN_NEEDS_JSON));`.
  Append to `worker/src/admin.unit.test.ts` a cell over `isJsonEssence`: `application/json`, `Application/JSON`,
  `application/json; charset=utf-8`, ` application/json ` true; `text/plain`, `application/x-www-form-urlencoded`,
  `application/json-patch+json` (the layer would take it; the shell does not), `null` false.

- [ ] **Step 9: Run the pool file and the unit cells.** Expected: PASS — the create-with-unusable-id row reads
  `[400, {"error":"\"x.table\" is not a usable game id"}]` (probed at the plan review; the house's `dispatch` answers
  the layer's own refusal, one house call and zero tables).

- [ ] **Step 10: The live replay's force gains the type.** In `worker/scripts/replay/send.mjs`'s `forceAll`, the
  `headers: {}` becomes `headers: { "content-type": "application/json" }` with the comment `// The operator's POST
  needs application/json since security S2b (B5): a bare POST is a 400.`; the module comment ("the operator's force,
  sent bare", `send.mjs:3-4`) and `forceAll`'s doc ("bare: no credential and no claimed header", `:26-27`) say "with
  only the JSON type". In `worker/scripts/live-replay.test.mjs` the EXISTING cell `sends the credential to the
  Worker's origin alone, the forces bare, and a claimed header only for a row's who` (`:566`) asserts
  `Object.keys(force.headers)` is `[]` and goes red as written (measured): its loop over `forces` becomes
  `expect(force.headers, "a force is the owner's own assertion, over WARP: no credential, only the JSON type since B5").toEqual({ "content-type": "application/json" })`
  (the key set AND the value), and "the forces bare" in its title reads "the forces with only the JSON type"; every
  other request's `content-type` is `application/json` exactly when the row has a body (the existing contract, already
  asserted). `worker/scripts/live-replay.mjs`'s run log line (`:140`, "the forces bare (over WARP, …)") reads "the
  forces with only the JSON type (over WARP, …)" — one sentence, pinned by no cell; D-8's withdrawal left the file's
  LOGIC unchanged, not a false log line. Run `npm run test:scripts`. Expected: PASS.

- [ ] **Step 10c: The sentence the Origin cells now contradict.** `worker/test/worker.test.ts:606-607`'s comment "A plain
  request is not an upgrade and is never refused for its `Origin`" and the cell title at `:630` ("…a plain request is
  never refused for its Origin") are false for an unsafe method since B5; the cell stays green (it sends a GET). Both
  read "a plain READ is never refused for its `Origin` (an unsafe method from another site is B5's, `door.test.ts`)".

- [ ] **Step 10b: The operator cells send the type.** The JSON gate turns 14 cells of `worker/test/operator.test.ts`
  red as it stands (measured: `14 failed | 210 passed` in the worker project, all in that file): the helper `expire` (`:97-99`) and the
  direct `worker(\`${path}?force=\`, …)` in `takes ?force bare` gain `headers: { "content-type": "application/json" }`.
  Rows refused before the type is asked (a stranger's 403, `?force=false`'s 400, the unrouted 404s) need none.
  Expected after: 224/224 in the worker project (231 over the pool's two projects).

- [ ] **Step 11: The whole Worker set.** `npm run typecheck && npm test && npm run test:scripts`. Expected: every
  count as the board's Gates read them plus this task's cells; the open project's existing `boss@t.test` operator cell
  still reads the layer's 404 (an unknown path is `NoRoute` before identity).

- [ ] **Step 12: Commit.**

```bash
git add worker/src/index.ts worker/src/admin.ts worker/src/route.ts worker/src/route.unit.test.ts worker/src/admin.unit.test.ts worker/test/door.test.ts worker/test/operator.test.ts worker/test/worker.test.ts worker/scripts/replay/send.mjs worker/scripts/live-replay.mjs worker/scripts/live-replay.test.mjs
git commit -m "worker: an unsafe method from another site is refused before identity and the operator's POST needs application/json (B5); the route's name grammar is the store's, pinned to id_cases.json (C2's name rule) (S2b Task 1)"
```

**Mutants the reviewer runs on an export (red where named):** M1 the `Sec-Fetch-Site` arm dropped → Step 5's two
no-Origin rows; M2 `sent.startsWith(origin)` → the `.evil` row; M3 `SAFE_METHODS` emptied → the GET and HEAD rows;
M4 the check moved to just before `objectFor` (after the operator gate) → Step 5's first cell (the operator rows read
the gate's 403, not CROSS_SITE); M4b the check moved after `identify` → the key-source cell (`asked` reads 1; the
both-defects row alone is GREEN under M4b, since `identify` with no assertion asks nothing — measured); M5 `isJsonEssence` comparing the whole string →
the charset row; M6 the type gate moved before `operatorGate` → the stranger row reads 400; M7 `isSafeId` without the
leading-character test → `_x`; M8 the suffix test case-sensitive → `x.TABLE`; M9 the create arm back to a length test →
the create row's `tables` count 1; M10 `forceAll` without the header → the replay cell.

### Task 2: C2 — the two Rate Limiting bindings, the key, the 429, the configurations, the guard's check 13 — Opus (C2; decisions D-3, D-10; Review Focus 2)

**Files:**
- Create: `worker/src/limits.ts`, `worker/src/limits.unit.test.ts`, `worker/src/redact.ts`, `worker/src/redact.unit.test.ts`
- Modify: `worker/src/env.ts`, `worker/src/config.ts` (+ `config.unit.test.ts`), `worker/src/index.ts` (`answer`)
- Modify: `worker/wrangler.jsonc`, `worker/wrangler.gate.jsonc` (until Task 6 deletes it), `worker/wrangler.open.jsonc`,
  `worker/wrangler.test.jsonc`, `worker/wrangler.open.test.jsonc`
- Modify: `worker/scripts/deploy-guard.mjs` (+ `deploy-guard.test.mjs`): check 13
- Test: `worker/test/door.test.ts` (the limiter through `handle`), `worker/test/open/door.test.ts` (the open door's keys)

**Interfaces:**
- Consumes: Task 1's `isSafeId` (a refused name is the house's before the limiter asks); `callerName`, `Identity`
  (`identity.ts`); `Door`, `shellConfig` (`config.ts`); `Target` (`route.ts`); `respond`/`errorBody`.
- Produces: `limitKey`, `limited`, `TOO_MANY`, `RETRY_AFTER_CREATE`, `RETRY_AFTER_WRITE`, `IP_HEADER`, `UNKNOWN_IP` (`limits.ts`); `callerTag`,
  `callerDigest`, `redactKey`, `DIGEST_HEX` (`redact.ts`, for Tasks 3 and 7); `ShellConfig.limits`; `RATE_LIMITS`
  (the guard); the `ratelimits` block in every configuration.

- [ ] **Step 1: `redact.ts` and its failing cells (node).** Create `worker/src/redact.unit.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { callerDigest, callerTag, DIGEST_HEX, redactKey } from "./redact";

/** SHA-256 in hex, computed HERE, so no cell checks the module's digest against its own. */
async function sha(text: string): Promise<string> {
  const d = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
  return [...d].map((b) => b.toString(16).padStart(2, "0")).join("");
}

describe("redact.ts (security S2b, decisions D-3, D-4, D-6)", () => {
  it("tags a caller by the full digest and names them by its first twelve hex", async () => {
    expect(await callerTag("red@wire.test")).toBe(`u:${await sha("red@wire.test")}`);
    expect(await callerDigest("red@wire.test")).toBe((await sha("red@wire.test")).slice(0, 12));
    expect(DIGEST_HEX).toBe(12);
    expect((await callerTag("a".repeat(254))).length, "a tag is at most 256 characters, whatever the address").toBe(66);
  });

  // Mutants: the regex anchored at the start only (a `test/` root's key kept its readable half); the digest group
  // widened (`[0-9a-f]+`: the `red-cafe` row — a four-hex tail is no digest; the hex-readable row alone does NOT
  // catch it, since `[^/]*` backtracks to the same `-` either way, measured); the replacement dropping the slash.
  it("drops the readable half of a browser segment and leaves every other key as it is", () => {
    expect(redactKey("browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.json")).toBe("browser/c91ad2a917f7/rev7/over/wire.json");
    expect(redactKey("test/browser/red_at_wire.test-c91ad2a917f7/rev7/abandoned/wire.table.json")).toBe("test/browser/c91ad2a917f7/rev7/abandoned/wire.table.json");
    expect(redactKey("browser/deadbeef0123-456789abcdef/rev7/over/x.json"), "a readable half that is hex is still the readable half").toBe("browser/456789abcdef/rev7/over/x.json");
    expect(redactKey("rev7/over/sim-0123456789abcdef.json"), "a server game's key names no caller").toBe("rev7/over/sim-0123456789abcdef.json");
    expect(redactKey("browser/no-digest-here/rev7/over/x.json"), "a legacy folder with no digest is left whole: nothing files under one since S1c").toBe("browser/no-digest-here/rev7/over/x.json");
    expect(redactKey("browser/red-cafe/rev7/over/x.json"), "a four-hex tail is no digest: the readable half stays").toBe("browser/red-cafe/rev7/over/x.json");
  });
});
```

- [ ] **Step 2: `redact.ts`.**

```ts
/**
 * **What the shell says about a caller, and what it never says** (security S2b, finding D23; decisions D-3, D-4,
 * D-6). The shell's own lines persist in Workers Observability, and three of them carried an address or a folder
 * that spells one: the two `operator` lines and every `archive-*` line's `record_key`. A digest names the caller
 * well enough to follow a session through the logs and ties to nothing outside them; the same digest is the
 * sockets' per-caller tag (D-4) and the rate limit's caller half (D-3), so a caller's address reaches no platform
 * store but the identity header the objects read. Pure but for `crypto.subtle`, which is why two of these are async.
 */

/** How many hex characters of the digest name a caller in a log line: 48 bits, the archive segment's own suffix
 *  (`archive::SEGMENT_DIGEST`, security S1c), so a redacted key's folder half still equals the folder's suffix. */
export const DIGEST_HEX = 12;

async function sha256Hex(text: string): Promise<string> {
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)));
  return [...digest].map((b) => b.toString(16).padStart(2, "0")).join("");
}

/** The tag a caller's sockets carry and the key their writes are counted under: `u:` and the whole digest, 66
 *  characters for any address (a tag is at most 256; an address at most 254, which with its prefix would be the
 *  limit exactly, and would put the address in the platform's tag store). */
export async function callerTag(user: string): Promise<string> {
  return `u:${await sha256Hex(user)}`;
}

/** The twelve hex a log line names a caller by. */
export async function callerDigest(address: string): Promise<string> {
  return (await sha256Hex(address)).slice(0, DIGEST_HEX);
}

/** A browser save's segment, as `archive::caller_segment` writes it since security S1c: the readable half, `-`,
 *  twelve hex. Matched after `browser/` at the key's start or after a root (`test/`). */
const SEGMENT = /(^|\/)browser\/[^/]*-([0-9a-f]{12})\//;

/** `key` with the segment's readable half dropped — `browser/<12 hex>/…` — so a line still names the folder and
 *  not the address it spells. A key with no such segment (a server game's; a legacy folder) is returned as it is. */
export function redactKey(key: string): string {
  return key.replace(SEGMENT, "$1browser/$2/");
}
```

- [ ] **Step 3: Run the redact cells.** Expected: PASS.

- [ ] **Step 4: `limits.ts`'s failing cells (node).** Create `worker/src/limits.unit.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { callerTag } from "./redact";
import { IP_HEADER, limiterFor, limitKey, RETRY_AFTER_CREATE, RETRY_AFTER_WRITE, TOO_MANY, UNKNOWN_IP } from "./limits";

describe("limitKey (security S2b, C2; decision D-3)", () => {
  // Mutants: the worker door keyed by the raw address (an `@` in the key); the open door's writes keyed by IP
  // alone; a missing IP throwing or reading `null`.
  it("keys a verified caller by their digest, an open-door claim by IP for a create and IP plus claim for a write, and nobody by IP", async () => {
    const person = { kind: "person", email: "red@wire.test" } as const;
    const claimed = { kind: "claimed", name: "ben" } as const;
    const nobody = { kind: "none" } as const;
    expect(await limitKey("worker", person, "203.0.113.9", "create")).toBe(await callerTag("red@wire.test"));
    expect(await limitKey("worker", person, "203.0.113.9", "write")).toBe(await callerTag("red@wire.test"));
    expect(await limitKey("worker", { kind: "dev", email: "a@e2e.test" }, null, "write")).toBe(await callerTag("a@e2e.test"));
    expect(await limitKey("open", claimed, "203.0.113.9", "create")).toBe("ip:203.0.113.9");
    expect(await limitKey("open", claimed, "203.0.113.9", "write")).toBe(`ip:203.0.113.9|${await callerTag("ben")}`);
    expect(await limitKey("open", claimed, null, "write")).toBe(`ip:${UNKNOWN_IP}|${await callerTag("ben")}`);
    expect(await limitKey("worker", nobody, "203.0.113.9", "write")).toBe("ip:203.0.113.9");
    expect(await limitKey("open", nobody, null, "create")).toBe(`ip:${UNKNOWN_IP}`);
    expect(IP_HEADER).toBe("cf-connecting-ip");
    expect([RETRY_AFTER_CREATE, RETRY_AFTER_WRITE], "each kind's own window, in seconds").toEqual(["60", "10"]);
    expect(TOO_MANY).toBe("too many requests from you just now — wait a moment and try again");
  });

  it("spends the create limiter on a create, the write limiter on every other unsafe method, and nothing on a read", () => {
    const create = { limit: async () => ({ success: true }) };
    const write = { limit: async () => ({ success: true }) };
    const limits = { create, write };
    expect(limiterFor("POST", { kind: "create" }, limits)).toEqual({ limiter: create, kind: "create" });
    expect(limiterFor("POST", { kind: "table", name: "x" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("DELETE", { kind: "table", name: "x" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "archive" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("POST", { kind: "admin", rest: "/tables/x/expire" }, limits)).toEqual({ limiter: write, kind: "write" });
    expect(limiterFor("GET", { kind: "table", name: "x" }, limits)).toBeUndefined();
    expect(limiterFor("HEAD", { kind: "house" }, limits)).toBeUndefined();
  });
});
```

- [ ] **Step 5: `limits.ts`.**

```ts
import type { Door, Limits } from "./config";
import { callerName, type Identity } from "./identity";
import { callerTag } from "./redact";
import { errorBody, respond } from "./respond";
import type { Target } from "./route";

/** The 429's sentence (security S2b, finding C2; host-written). */
export const TOO_MANY = "too many requests from you just now — wait a moment and try again";
/** A refused create's `Retry-After`, in seconds: the creates' window (10 per 60 s). */
export const RETRY_AFTER_CREATE = "60";
/** A refused write's `Retry-After`, in seconds: the writes' window (120 per 10 s). */
export const RETRY_AFTER_WRITE = "10";
/** Where the edge puts the connecting IP. */
export const IP_HEADER = "cf-connecting-ip";
/** The IP's stand-in when the header is absent — the pool, or an edge that did not set it. */
export const UNKNOWN_IP = "unknown";

/** What a binding is to this module: `limit` and nothing else, so a cell hands in a fake. */
export type Limiter = Pick<RateLimit, "limit">;

/**
 * **The key a request is counted under** (decision D-3). Under `worker` every caller is a verified address, so
 * the caller's digest (`callerTag`), whatever their IP: a person on two devices is one budget. Under `open` a
 * claim is free, so a create is counted by the connecting IP alone (a flood of names from one address is one
 * budget) and a write by IP and claim together (a household's players each get their own). A request with no
 * caller is counted by IP under either door — the layer answers it 401, and its object call is still a cost. An
 * absent IP header reads `unknown`: one shared budget for a request that reached this Worker without the edge.
 */
export async function limitKey(door: Door, identity: Identity, ip: string | null, kind: "create" | "write"): Promise<string> {
  const who = callerName(identity);
  const at = `ip:${ip ?? UNKNOWN_IP}`;
  if (who === undefined) return at;
  if (door === "worker") return callerTag(who);
  return kind === "create" ? at : `${at}|${await callerTag(who)}`;
}

/** Which binding an unsafe request spends: the create's for `POST /api/games`, the writes' for every other method
 *  but GET and HEAD — the operator's POST, `/api/archive` and every DELETE among them; `undefined` for a read. */
export function limiterFor(method: string, target: Target, limits: Limits): { limiter: Limiter; kind: "create" | "write" } | undefined {
  if (method === "GET" || method === "HEAD") return undefined;
  return target.kind === "create" ? { limiter: limits.create, kind: "create" } : { limiter: limits.write, kind: "write" };
}

/**
 * The 429 a request past its limit answers, or `undefined` to proceed (security S2b, finding C2; decision D-3).
 * Asked in `answer` after the caller is known and BEFORE any object is reached — so a refused request costs the
 * stateless Worker's invocation and no Durable Object's. The platform counts per Cloudflare location, best
 * effort: a bound on cost, never a fence a feature may lean on. `Retry-After` is the refused kind's own window.
 */
export async function limited(req: Request, target: Target, identity: Identity, door: Door, limits: Limits): Promise<Response | undefined> {
  const spend = limiterFor(req.method, target, limits);
  if (spend === undefined) return undefined;
  const key = await limitKey(door, identity, req.headers.get(IP_HEADER), spend.kind);
  const { success } = await spend.limiter.limit({ key });
  if (success) return undefined;
  const res = respond(429, errorBody(TOO_MANY));
  res.headers.set("Retry-After", spend.kind === "create" ? RETRY_AFTER_CREATE : RETRY_AFTER_WRITE);
  return res;
}
```

- [ ] **Step 6: The bindings in `env.ts` and `config.ts`.** In `env.ts`, beside `ARCHIVE?`:

```ts
  /** The two Rate Limiting bindings (security S2b, finding C2; decision D-3): creates, 10 per 60 s, and writes,
   *  120 per 10 s, in production; the pool binds both with a lenient figure. Optional in the TYPE because a cell
   *  builds an `Env` by hand; `shellConfig` refuses a configuration lacking either, so no deployment runs without
   *  them (fail closed, as `ARCHIVE` under `r2`). */
  CREATE_LIMIT?: RateLimit;
  WRITE_LIMIT?: RateLimit;
```

  In `config.ts`: `export interface Limits { create: Pick<RateLimit, "limit">; write: Pick<RateLimit, "limit"> }`;
  `ShellConfig` gains `limits: Limits`; a reader

```ts
/** The two bindings, both required (security S2b, C2): a configuration that lacks one is refused by name, the
 *  Worker's form of the NAS refusing to start, so no deployment runs unthrottled by omission. */
function limits(env: Env): Limits {
  for (const name of ["CREATE_LIMIT", "WRITE_LIMIT"] as const) {
    if (env[name] === undefined) refuse(`the ${name} rate-limit binding is missing — every deployable configuration binds it in its ratelimits block (security S2b)`);
  }
  return { create: env.CREATE_LIMIT!, write: env.WRITE_LIMIT! };
}
```

  called in `shellConfig` after `strings(env)` and placed last in the `ShellConfig` literal (the door stays first).
  **Every hand-built `Env` and `ShellConfig` in a unit cell gains the two bindings**, or `shellConfig` refuses it and
  `tsc` refuses the literal (measured: TS2741 on the literal, TS2352 on an untyped fake under `as Env`).
  `config.unit.test.ts`'s `env()` fixture spreads a module-level
  `const FAKE_CREATE: RateLimit = { limit: async () => ({ success: true }) }; const FAKE_WRITE: RateLimit = { limit: async () => ({ success: true }) }; const LIMITS = { CREATE_LIMIT: FAKE_CREATE, WRITE_LIMIT: FAKE_WRITE };`
  (the `RateLimit` annotation is what keeps the `as Env` cast from TS2352; TWO objects, so the uncrossed check below can
  see a crossing — one shared fake could not, the pre-flight read), and `marker.unit.test.ts`'s `ShellConfig`
  literal (line 111 — a `ShellConfig` built by hand, which a grep for `shellConfig(` cannot find) gains
  `limits: { create: { limit: async () => ({ success: true }) }, write: { limit: async () => ({ success: true }) } }`;
  `grep -rln "shellConfig\|moduleConfig\|ShellConfig" worker/src/*.unit.test.ts` lists both (and `admin.unit.test.ts`,
  a comment). `config.unit.test.ts` gains three cells (written out, not described): an `env()` lacking `WRITE_LIMIT`
  → `{ error: "the WRITE_LIMIT rate-limit binding is missing — every deployable configuration binds it in its ratelimits block (security S2b)" }`;
  one lacking both → `CREATE_LIMIT` named first; and `accepted(env()).limits.create`/`.write` are `FAKE_CREATE`/`FAKE_WRITE`
  by identity (`toBe`), uncrossed. The pool-side identity cell — `const c = shellConfig(env); if ("error" in c) throw new Error(c.error); expect(c.limits.create).toBe(env.CREATE_LIMIT); expect(c.limits.write).toBe(env.WRITE_LIMIT);` (the narrowing line is what `tsc` needs — TS2339 without it, measured at the pre-flight; the pool, which never type-checks, ran the cell green either way)
  — joins `test/door.test.ts`'s last cell (Step 9), because it needs `cloudflare:workers`; a node cell cannot read the
  pool's `env`.

- [ ] **Step 7: The limiter in `answer`.** After `const who = callerName(identity); if (who !== undefined) headers.set(…)`
  and before `let forwarded: Forwarded;` (proposed form of the `try`; the `catch` is the plan review's AD-6 (b)):

```ts
  // The rate limit (security S2b, finding C2; decision D-3): after the caller is known, before any object. A
  // limiter that throws is logged and the request proceeds: the limit bounds cost, never access (AD-6) — a binding
  // that rejected would otherwise reject `handle`, the platform's own error page, and nothing logged (measured).
  let refused: Response | undefined;
  try {
    refused = await limited(req, target, identity, config.door, config.limits);
  } catch (e) {
    log("limiter-failed", { error: String(e) });
  }
  if (refused !== undefined) return refused;
```

  and a step "5a" in `answer`'s doc list.

- [ ] **Step 8: The configurations.** `wrangler.jsonc` gains, after `"r2_buckets"`:

```jsonc
  // Two Rate Limiting bindings (security S2b, finding C2; decision D-3), counted per Cloudflare location by the
  // key `src/limits.ts` derives: a caller's digest under `worker`; under `open` the connecting IP for a create and
  // IP plus claim for a write. The figures: ten creates a minute is a person opening tables as fast as a form
  // allows; a hundred and twenty writes per ten seconds is twelve a second, thirty times a fast player's four — the
  // local two-browser walk's pages peak at 71 unsafe requests in 10 s at machine speed (measured at S2b's pre-flight;
  // the live tier is slower, never faster), and the live replay, which sends 26 rows at the production door since
  // S1b, never reaches it. A `namespace_id` is a string here and unique to the ACCOUNT,
  // and a shared id shares counters across Workers. The deploy guard's check 13 holds this block to these literals,
  // and the pool's two files to the two NAMES, with a lenient figure of their own.
  "ratelimits": [
    { "name": "CREATE_LIMIT", "namespace_id": "7301", "simple": { "limit": 10, "period": 60 } },
    { "name": "WRITE_LIMIT", "namespace_id": "7302", "simple": { "limit": 120, "period": 10 } }
  ],
```

  `wrangler.gate.jsonc` and `wrangler.open.jsonc` gain the same block byte for byte (the sibling rule compares it);
  `wrangler.test.jsonc` and `wrangler.open.test.jsonc` gain:

```jsonc
  // The two bindings production carries (security S2b, C2), with a LENIENT figure: `test/worker.test.ts` sends more
  // than ten creates a minute as one caller through `handle`, and miniflare's ratelimit plugin counts for real (the
  // corpus replay reaches `handle` for `/api/art` alone and spends nothing). The shell's refusal path is proven
  // through a fake limiter handed in `env` (`test/door.test.ts`); production's figures are the deploy guard's
  // check 13, which holds this file to the two names alone.
  "ratelimits": [
    { "name": "CREATE_LIMIT", "namespace_id": "7301", "simple": { "limit": 1000000, "period": 60 } },
    { "name": "WRITE_LIMIT", "namespace_id": "7302", "simple": { "limit": 1000000, "period": 60 } }
  ],
```

  — the comment above is `wrangler.test.jsonc`'s; `wrangler.open.test.jsonc` carries the same block under its own first
  sentence, since `test/worker.test.ts` runs in the OTHER project and the open project passes at production's figures
  (measured at Task 2's run): `// The two bindings production carries (security S2b, C2), with the LENIENT figure of
  its sibling wrangler.test.jsonc: the open project's cells pass at production's figures today, and one figure for
  both pool files keeps them siblings. The shell's refusal path is proven through a fake limiter handed in env
  (test/open/door.test.ts's key cell); production's figures are the deploy guard's check 13, which holds this file to
  the two names alone.`

- [ ] **Step 9: The pool cells (worker project), appended to `worker/test/door.test.ts`:**

```ts
import { callerTag } from "../src/redact";
import { RETRY_AFTER_CREATE, RETRY_AFTER_WRITE, TOO_MANY } from "../src/limits";
import type { Limiter } from "../src/limits";
import { shellConfig } from "../src/config";

/** A fake binding that records every key and answers as told. */
function limiter(answer: (key: string) => boolean): Limiter & { asked: string[] } {
  const asked: string[] = [];
  return { asked, limit: async ({ key }) => { asked.push(key); return { success: answer(key) }; } };
}

describe("C2 — the rate limit, before any object", () => {
  // Mutants: `limiterFor` spending the write limiter on a create; `limited` ignoring `success`; the 429 without
  // `Retry-After`; the call placed after `object.fetch` returns (a refused request still reaches the house or a table —
  // after `objectFor` alone is an equivalent mutant: it builds a stub); a GET asking the limiter.
  it("spends the create limiter on a create and the write limiter on a write, keyed by the caller's digest, and asks nothing for a read", async () => {
    const create = limiter(() => true);
    const write = limiter(() => true);
    const limited: Env = { ...env, CREATE_LIMIT: create as unknown as RateLimit, WRITE_LIMIT: write as unknown as RateLimit };
    const red = await person(RED);
    const body = { players: 2, colors: ["Red", "Blue"], start_seat: 0, id: "door-c2-keys", seats: [{ user: RED }, { open: true }] };
    expect((await worker("/api/games", { method: "POST", headers: JSON_TYPE, body: JSON.stringify(body), token: red, env: limited })).status).toBe(201);
    expect((await worker("/api/games/door-c2-keys/chat", { method: "POST", headers: JSON_TYPE, body: '{"text":"hi"}', token: red, env: limited })).status).toBe(201);
    expect((await worker("/api/games/door-c2-keys/table", { token: red, env: limited })).status).toBe(200);
    expect(create.asked).toEqual([await callerTag(RED)]);
    expect(write.asked).toEqual([await callerTag(RED)]);
  });

  it("answers a refused request 429 with the kind's own Retry-After and the sentence, under the API's two headers, reaching no object", async () => {
    const limited: Env = { ...env, CREATE_LIMIT: limiter(() => false) as unknown as RateLimit, WRITE_LIMIT: limiter(() => false) as unknown as RateLimit };
    const red = await person(RED);
    const id = "door-c2-refused";
    await seedPlaying(id);
    const { result, reached } = await reaching(() => worker(`/api/games/${id}/input`, { method: "POST", headers: JSON_TYPE, body: '{"line":"1"}', token: red, env: limited }));
    expect([result.status, result.text]).toEqual([429, JSON.stringify({ error: TOO_MANY })]);
    expect(result.headers.get("Retry-After"), "a write's window").toBe(RETRY_AFTER_WRITE);
    expect(result.headers.get("Cache-Control")).toBe("no-store");
    expect(result.headers.get("X-Content-Type-Options")).toBe("nosniff");
    expect(reached, "no object reached").toBe(0);
    const create = await reaching(() => worker("/api/games", { method: "POST", headers: JSON_TYPE, body: '{"players":2,"colors":["Red","Blue"],"start_seat":0,"seats":[{"user":"red@t.test"},{"open":true}]}', token: red, env: limited }));
    expect([create.result.status, create.reached]).toEqual([429, 0]);
    expect(create.result.headers.get("Retry-After"), "a create's window").toBe(RETRY_AFTER_CREATE);
  });

  // AD-6 (b): a limiter that throws is logged and the request proceeds — the limit bounds cost, never access.
  it("logs a limiter that throws once and lets the request through", async () => {
    const broken = { asked: [] as string[], limit: async () => { throw new Error("binding unavailable"); } };
    const limited: Env = { ...env, WRITE_LIMIT: broken as unknown as RateLimit };
    const id = "door-c2-broken";
    await seedPlaying(id);
    const { result, lines } = await logsDuring(() => reaching(async () => worker(`/api/games/${id}/input`, { method: "POST", headers: JSON_TYPE, body: '{"line":"1"}', token: await person(RED), env: limited })));
    expect([result.result.status, result.reached], "the request reached its object").toEqual([200, 1]);
    expect(lines.filter((l) => (l as { event: string }).event === "limiter-failed"), "exactly one line").toHaveLength(1);
  });

  it("keys a request with no caller by the connecting IP, and by `unknown` without one", async () => {
    const write = limiter(() => true);
    const limited: Env = { ...env, WRITE_LIMIT: write as unknown as RateLimit };
    await worker("/api/chat", { method: "POST", headers: { ...JSON_TYPE, "cf-connecting-ip": "203.0.113.9" }, body: '{"text":"hi"}', env: limited });
    await worker("/api/chat", { method: "POST", headers: JSON_TYPE, body: '{"text":"hi"}', env: limited });
    expect(write.asked).toEqual(["ip:203.0.113.9", "ip:unknown"]);
  });

  it("binds both limiters in the pool, a lenient one answers success, and shellConfig hands the bindings through by identity", async () => {
    expect(typeof env.CREATE_LIMIT!.limit).toBe("function");
    expect(await env.WRITE_LIMIT!.limit({ key: "door-c2-smoke" })).toEqual({ success: true });
    const c = shellConfig(env);
    if ("error" in c) throw new Error(c.error); // the narrowing `tsc` needs: `shellConfig` answers `ShellConfig | { error }`
    expect(c.limits.create).toBe(env.CREATE_LIMIT);
    expect(c.limits.write).toBe(env.WRITE_LIMIT);
  });
});
```

  (`logsDuring` is `test/helpers.ts`'s; import it from `./helpers` as `sockets.test.ts` does.) And in
  `worker/test/open/door.test.ts` (the open project), which gains its OWN copy of the `limiter` fake and imports
  `callerTag` (`../../src/redact`) and `import type { Limiter } from "../../src/limits";` (`Env` is already imported
  there) — the fake lives in the other project's file and cannot be shared across the pool's one-file rule; the cell
  goes at the end of the file's last `describe`:

```ts
  it("keys an open-door create by the connecting IP alone, and a write by IP and the claim's digest", async () => {
    const create = limiter(() => true);
    const write = limiter(() => true);
    const limited: Env = { ...env, CREATE_LIMIT: create as unknown as RateLimit, WRITE_LIMIT: write as unknown as RateLimit };
    const ip = { "cf-connecting-ip": "203.0.113.9" };
    const body = { players: 2, colors: ["Red", "Blue"], start_seat: 0, id: "open-c2-keys", seats: [{ user: "ben" }, { open: true }] };
    expect((await through("/api/games", { method: "POST", claim: "ben", headers: { ...JSON_TYPE, ...ip }, body: JSON.stringify(body), env: limited })).status).toBe(201);
    expect((await through("/api/games/open-c2-keys/chat", { method: "POST", claim: "ben", headers: { ...JSON_TYPE, ...ip }, body: '{"text":"hi"}', env: limited })).status).toBe(201);
    expect(create.asked).toEqual(["ip:203.0.113.9"]);
    expect(write.asked).toEqual([`ip:203.0.113.9|${await callerTag("ben")}`]);
  });
```

- [ ] **Step 10: Run the pool, both projects.** `npm run build:module:debug && npm test`. Expected: PASS.

- [ ] **Step 11: The guard's check 13 and its cells.** In `deploy-guard.mjs`, beside `OBJECT_BLOCKS`:

```js
/** Production's two Rate Limiting bindings as values (security S2b, finding C2; decision D-3), the literals the
 *  shell's comment defends. The dev and open files carry the same block by the sibling rule; a pool file is held
 *  to the two NAMES alone, its figures being lenient by design (`test/worker.test.ts`'s creates through `handle`).
 *  Read at the top level: `ratelimits` is not inherited by a named environment (wrangler's schema), and the
 *  repository deploys none. */
export const RATE_LIMITS = [
  { name: "CREATE_LIMIT", namespace_id: "7301", simple: { limit: 10, period: 60 } },
  { name: "WRITE_LIMIT", namespace_id: "7302", simple: { limit: 120, period: 10 } },
];

/** Check 13: production's block at its literals; each pool file naming exactly the two bindings. */
function rateLimits(production, pools, failures) {
  if (canonical(production.value.ratelimits) !== canonical(RATE_LIMITS)) {
    failures.push(
      `wrangler.jsonc's ratelimits block is not the two bindings at their figures — CREATE_LIMIT 10 per 60 s and WRITE_LIMIT 120 per 10 s, namespaces 7301 and 7302 (security S2b, C2); got ${canonical(production.value.ratelimits) ?? "nothing"}`,
    );
  }
  for (const [name, pool] of pools) {
    const names = Array.isArray(pool.value.ratelimits) ? pool.value.ratelimits.map((b) => objectOf(b)?.name).sort() : [];
    if (canonical(names) !== canonical(["CREATE_LIMIT", "WRITE_LIMIT"])) {
      failures.push(`${name} must bind exactly CREATE_LIMIT and WRITE_LIMIT under ratelimits, at any figure (test/worker.test.ts sends a dozen creates a minute as one caller through handle) — got ${canonical(names)}`);
    }
  }
}
```

  called in `configFailures` after `objectBlocks(…)`: `if (production) rateLimits(production, others.filter(([n]) => POOLS.includes(n)), failures);`.
  The header comment gains check 13's paragraph (and the note that it reads the top level, since `ratelimits` is not
  inherited by a named environment); the success sentence — the closing `console.log` of `deploy-guard.mjs`; the guard
  has NO `--help` — names it, and "all twelve" becomes "all thirteen" at `deploy-guard.mjs:4` and `:692`.
  `deploy-guard.test.mjs`: the fixtures
  `PRODUCTION`, `OPEN`, `GATE` and both pool configs gain the block (production's at the literals; the pools' lenient),
  and two new cells: production with `limit: 121` refused naming `ratelimits`; a pool file lacking `WRITE_LIMIT`
  refused naming the file; the committed five files pass through the existing `pass every check` cells once the
  fixtures carry the block.

- [ ] **Step 12: Run the node and script suites.** `npm run test:scripts && npm run typecheck`. Expected: PASS.
  `node scripts/deploy-guard.mjs --config wrangler.jsonc` over the real tree prints the door line; a tree with no
  assembled `assets/` also prints check 1's sentence (the DEBUG module) and check 2's (`assets/sw.js is absent`), one
  with assets but no `web/dist` check 9's two — none this task's;
  `configFailures(root)` over the committed files returns `[]` (the committed-file cells in `test:scripts` read it).

- [ ] **Step 13: Commit.**

```bash
git add worker/src/limits.ts worker/src/limits.unit.test.ts worker/src/redact.ts worker/src/redact.unit.test.ts worker/src/env.ts worker/src/config.ts worker/src/config.unit.test.ts worker/src/marker.unit.test.ts worker/src/index.ts worker/wrangler.jsonc worker/wrangler.gate.jsonc worker/wrangler.open.jsonc worker/wrangler.test.jsonc worker/wrangler.open.test.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/test/door.test.ts worker/test/open/door.test.ts
git commit -m "worker: two Rate Limiting bindings — creates 10 per 60 s, writes 120 per 10 s — keyed by the caller's digest under worker and by IP or IP plus claim under open, a 429 before any object; the guard's check 13 (C2) (S2b Task 2)"
```

**Mutants (red where named):** M11 `limiterFor` spending the write limiter on a create → Step 9's first cell;
M12 `limited` ignoring `success` → the 429 cell; M13 no `Retry-After` → the 429 cell; M13b the two `Retry-After`s
crossed → the 429 cell's two header assertions; M14 the limiter consulted after `object.fetch` returns → the 429 cell's
`reached` (placing it merely after `objectFor` is an EQUIVALENT mutant: `objectFor` only builds a stub); M14c the
`catch` removed → the rejecting-limiter cell; M15 a GET asking the limiter → the first cell's `asked` arrays; M16 the worker key as the raw
address → the first cell (`@` in the key); M17 the open write keyed by IP alone → the open cell; M18 `UNKNOWN_IP`
read as `null` → the IP cell; M19 `limits()` not refusing an absent binding → the config cell; M20 `RATE_LIMITS`'
`limit` 121 → the guard cell; M21 the pool-names check dropped → the guard's pool cell; M22 `redactKey`'s regex anchored
at the start → the `test/` row.

### Task 3: D3 — the socket caps on a table and in the lobby, refused in the shape the client reads — Opus (D3's Worker half; decision D-4; Review Focus 4)

**Files:**
- Modify: `worker/src/frames.ts` (+ `frames.unit.test.ts`): `CLOSE_TOO_MANY`, the four sentences
- Modify: `worker/src/table/sockets.ts`: `capVerdict`, the caps, `admit`'s new arm and tags
- Modify: `worker/src/table.ts`: `Incoming.callerTag`, computed in `fetch`; `admit`'s abstract signature; `finish`
- Modify: `worker/src/house.ts`: `LOBBY_PER_CALLER`, `LOBBY_TOTAL`, `Incoming.callerTag`, `admit`'s arm and tags
- Create: `worker/test/caps.test.ts`
- Test: `worker/src/table/sockets.unit.test.ts` (new, node: `capVerdict`), `worker/src/frames.unit.test.ts` (the
  literal pin of the close code and the four sentences), `worker/test/sockets.test.ts` (one existing cell: the tags as
  a set)

**Interfaces:**
- Consumes: `callerTag` (`redact.ts`, Task 2); `endAtAdmission`, `closeReason`, `frame` (`frames.ts`);
  `ctx.getWebSockets(tag)`; the pool's `openTab`-style helpers (`test/sockets.test.ts`, copied into `caps.test.ts`).
- Produces: `SOCKETS_PER_CALLER` (8), `SOCKETS_PER_TABLE` (64), `LOBBY_PER_CALLER` (8), `LOBBY_TOTAL` (1024),
  `CLOSE_TOO_MANY` (4429), `TOO_MANY_OWN_TABLE`, `TOO_MANY_TABLE`, `TOO_MANY_OWN_LOBBY`, `TOO_MANY_LOBBY`,
  `capVerdict(own, total, caps)`; the socket tag shape `[id, callerTag]` / `[LOBBY_TAG, callerTag]` the documents name.

- [ ] **Step 1: The pure verdict's failing cells (node).** Create `worker/src/table/sockets.unit.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { capVerdict, SOCKETS_PER_CALLER, SOCKETS_PER_TABLE } from "./sockets";

describe("capVerdict (security S2b, D3; decision D-4)", () => {
  // Mutants: `>` for `>=` on either bound; the own cap read before the total (the total's sentence lost when both
  // are hit); the literals moved.
  it("refuses the ninth of one caller's and the sixty-fifth in all, the total's verdict first, and admits under both", () => {
    const caps = { perCaller: SOCKETS_PER_CALLER, total: SOCKETS_PER_TABLE };
    expect(capVerdict(7, 63, caps)).toBeUndefined();
    expect(capVerdict(8, 63, caps)).toBe("own");
    expect(capVerdict(0, 64, caps)).toBe("total");
    expect(capVerdict(8, 64, caps), "both hit: the table's own sentence, since closing a tab would not help").toBe("total");
    expect([SOCKETS_PER_CALLER, SOCKETS_PER_TABLE]).toEqual([8, 64]);
  });
});
```

- [ ] **Step 2: The constants and the verdict.** In `worker/src/frames.ts`:

```ts
/** The close code of a socket refused for its count (security S2b, finding D3): the shape of the 4403/4404
 *  closes, one more mirror of an HTTP status. */
export const CLOSE_TOO_MANY = 4429;

/** The four sentences a cap's `end` frame carries (host-written): the caller's own count, and the object's. */
export const TOO_MANY_OWN_TABLE = "you have too many connections open to this table — close another tab and try again";
export const TOO_MANY_TABLE = "this table has as many connections as it can take — try again later";
export const TOO_MANY_OWN_LOBBY = "you have too many connections open to the lobby — close another tab and try again";
export const TOO_MANY_LOBBY = "the lobby has as many connections as it can take — try again later";
```

  In `worker/src/table/sockets.ts`:

```ts
/** How many sockets one caller may hold on one table (security S2b, finding D3; decision D-4): the client holds
 *  one per VISIBLE tab and none in a background tab (R-P8), and only OPEN sockets count (`openCount`: a socket in
 *  CLOSING stays there as long as its client declines to answer the close), so eight is tabs plus a reconnect's
 *  overlap, never a budget a feature may lean on. */
export const SOCKETS_PER_CALLER = 8;
/** How many sockets one table object holds in all: four seats and their spectators at eight each. The review's
 *  reproduction held 300 and paid 301 module calls per bump; the platform's own ceiling is 32,768. */
export const SOCKETS_PER_TABLE = 64;

/** Which cap a new socket would cross — `own` for the caller's, `total` for the object's — or `undefined` to admit.
 *  The total is judged first: at both, closing one's own tab would not help. */
export function capVerdict(own: number, total: number, caps: { perCaller: number; total: number }): "own" | "total" | undefined {
  if (total >= caps.total) return "total";
  if (own >= caps.perCaller) return "own";
  return undefined;
}

/** How many of `sockets` are OPEN — the count both caps read (Task 3's review). A refused or ended socket stays
 *  CLOSING for as long as its client declines to answer the close (an unaccepted pool client never does; a hostile
 *  one need not), and a count that included it would let a stranger's refusals lock a table's seated players out and
 *  one identity fill the lobby (measured). The CLOSING sockets still occupy the object until the platform tears them
 *  down: the pre-S2b shape, under the platform's 32,768. */
export function openCount(sockets: WebSocket[]): number {
  return sockets.filter((ws) => ws.readyState === WebSocket.OPEN).length;
}
```

- [ ] **Step 3: Run the node cell.** Expected: PASS.

- [ ] **Step 4: The pool's failing cells.** Create `worker/test/caps.test.ts`, with `openTab` and `upgrade` copied
  from `test/sockets.test.ts` (the pool's one-file rule, pool fact 5: this file has its own ids), plus:

```ts
import { CLOSE_TOO_MANY, TOO_MANY_OWN_LOBBY, TOO_MANY_OWN_TABLE, TOO_MANY_TABLE } from "../src/frames";
import { callerTag } from "../src/redact";
import { LOBBY_TAG } from "../src/house";
import { SOCKETS_PER_CALLER, SOCKETS_PER_TABLE } from "../src/table/sockets";

/** The `end` frame a cap sends, as the socket carries it, byte for byte. */
const endWith = (sentence: string) => `{"event":"end","data":${JSON.stringify({ reason: "gone", sentence })}}`;

/** A table at `name` with red seated and one seat OPEN, so any signed-in caller may open its stream. */
async function openTable(name: string): Promise<DurableObjectStub<Table>> {
  const stub = tableStub(name);
  const res = await call(stub, RED, "POST", "/api/games", CREATE_BODY, name);
  expect(res.status).toBe(201);
  return stub;
}

/** How many sockets the object holds under `tag` — or in all with none — read inside it. */
function held(stub: DurableObjectStub<Table> | DurableObjectStub<House>, tag?: string): Promise<number> {
  return runInDurableObject(stub as DurableObjectStub, (_o, state) => state.storage.sql !== undefined ? (tag === undefined ? state.getWebSockets().length : state.getWebSockets(tag).length) : -1);
}

/** How many OPEN sockets the object holds under `tag` — the count the caps read (`openCount`). */
function heldOpen(stub: DurableObjectStub<Table> | DurableObjectStub<House>, tag?: string): Promise<number> {
  return runInDurableObject(stub as DurableObjectStub, (_o, state) => (tag === undefined ? state.getWebSockets() : state.getWebSockets(tag)).filter((ws) => ws.readyState === WebSocket.OPEN).length);
}

describe("D3 — the socket caps (security S2b, decision D-4)", () => {
  // Mutants: the per-caller arm dropped (the ninth admitted); the tag omitted from `acceptWebSocket` (the count
  // reads 0 and the ninth is admitted); the refusal answered unupgraded (no 101: the client's ladder would hammer);
  // the close code 4404 (a deleted table's code); the literal 9.
  it("admits a caller's eighth socket and refuses the ninth with an end frame and a 4429 close, another caller unaffected, and frees the slot once a socket has closed", async () => {
    const id = "caps-own";
    const stub = await openTable(id);
    const tabs: Tab[] = [];
    for (let i = 0; i < SOCKETS_PER_CALLER; i += 1) tabs.push(await openTab(stub, GREEN, id));
    expect(await held(stub, await callerTag(GREEN))).toBe(SOCKETS_PER_CALLER);
    const ninth = await openTab(stub, GREEN, id);
    expect(ninth.res.status, "accepted, so the browser reads the frame").toBe(101);
    expect(await ninth.until(1)).toEqual([endWith(TOO_MANY_OWN_TABLE)]);
    expect(await ninth.closed()).toEqual({ code: CLOSE_TOO_MANY, reason: closeReason(TOO_MANY_OWN_TABLE) });
    expect(await held(stub, await callerTag(GREEN)), "the refused socket is not kept").toBe(SOCKETS_PER_CALLER);
    const other = await openTab(stub, BLUE, id);
    expect((await other.until(1))[0]).toContain('"event":"signal"');
    // Review Focus 4: a slot frees once the close has been processed (polled, bounded: no event marks it in the pool —
    // a client-initiated close delivers no `close` event, so `await tabs[0]!.closed()` would time out; measured).
    tabs[0]!.ws.close(1000);
    for (let i = 0; i < 20 && (await held(stub, await callerTag(GREEN))) >= SOCKETS_PER_CALLER; i += 1) await new Promise((r) => setTimeout(r, 50));
    const again = await openTab(stub, GREEN, id);
    expect((await again.until(1))[0], "admitted once a socket has gone").toContain('"event":"signal"');
  });

  it("refuses the sixty-fifth socket of a table whatever its caller, with the table's own sentence", async () => {
    const id = "caps-total";
    const stub = await openTable(id);
    const callers = Array.from({ length: SOCKETS_PER_TABLE / SOCKETS_PER_CALLER }, (_, i) => `c${i}@t.test`);
    const tabs: Tab[] = [];
    for (const who of callers) for (let i = 0; i < SOCKETS_PER_CALLER; i += 1) tabs.push(await openTab(stub, who, id));
    expect(await held(stub)).toBe(SOCKETS_PER_TABLE);
    const more = await openTab(stub, "late@t.test", id);
    expect(await more.until(1)).toEqual([endWith(TOO_MANY_TABLE)]);
    expect((await more.closed()).code).toBe(CLOSE_TOO_MANY);
    // The review's Important 1: a refused socket whose client never answers the close stays CLOSING and is NOT
    // counted — counted, a stranger's unanswered refusals would lock the seated players out (measured: 60 dangling
    // refusals, 68 held). Eight refusals left dangling (their client side never accepted), then one socket closes: the
    // next caller is admitted past them.
    const dangling: Response[] = [];
    for (let i = 0; i < 8; i += 1) dangling.push(await upgrade(stub, "dangling@t.test", id));
    expect(dangling.map((r) => r.status), "accepted, closing").toEqual(Array(8).fill(101));
    expect(await held(stub), "the dangling refusals are held, CLOSING").toBeGreaterThan(SOCKETS_PER_TABLE);
    expect(await heldOpen(stub), "and not counted").toBe(SOCKETS_PER_TABLE);
    tabs[0]!.ws.close(1000);
    for (let i = 0; i < 20 && (await heldOpen(stub)) >= SOCKETS_PER_TABLE; i += 1) await new Promise((r) => setTimeout(r, 50));
    const back = await openTab(stub, "late@t.test", id);
    expect((await back.until(1))[0], "admitted past the dangling refusals").toContain('"event":"signal"');
    expect(dangling.length, "held for the cell's life, like the lobby's kept").toBe(8);
  });

  it("caps the lobby per caller the same way, under its own sentence, and counts only OPEN sockets toward its total", async () => {
    const house = env.HOUSE.get(env.HOUSE.idFromName(HOUSE_NAME));
    const lobby = (who: string) => house.fetch("https://yokohama.test/api/stream", { headers: { Upgrade: "websocket", "X-Yokohama-User": who } });
    const kept: Response[] = []; // held for the cell's life: an unaccepted client socket that is dropped is garbage-collected, and its slot with it (measured: 8 held, then 0 — the cell flaked 2 of 5 runs without this)
    for (let i = 0; i < 8; i += 1) kept.push(await lobby("lob@t.test"));
    expect(kept.map((r) => r.status)).toEqual(Array(8).fill(101));
    const ninth = await lobby("lob@t.test");
    expect(ninth.status).toBe(101);
    const ws = ninth.webSocket!;
    ws.accept();
    const first = await new Promise<string>((resolve) => ws.addEventListener("message", (e) => resolve(String(e.data)), { once: true }));
    expect(first).toBe(endWith(TOO_MANY_OWN_LOBBY));
    const closed = await new Promise<{ code: number; reason: string }>((resolve) => ws.addEventListener("close", (e) => resolve({ code: e.code, reason: e.reason }), { once: true }));
    expect(closed, "the same close as a table's").toEqual({ code: CLOSE_TOO_MANY, reason: closeReason(TOO_MANY_OWN_LOBBY) });
    // The review's Important 2: the cap is PER CALLER — a second caller is admitted while the first is full (under a
    // site-wide lobby of eight this reads 0 OPEN under the second caller's tag; measured at the review).
    const other = await lobby("lob2@t.test");
    expect(other.status).toBe(101);
    kept.push(other);
    const otherWs = other.webSocket!;
    otherWs.accept();
    const listing = new Promise<string>((resolve) => otherWs.addEventListener("message", (e) => resolve(String(e.data)), { once: true }));
    // A lobby socket is silent on connect (spec §2.5; `slot.test.ts`'s cell), so a house chat post is the positive
    // control (ruling R-10): its frame reaches every admitted lobby socket, where a refused one's first frame is `end`.
    const posted = await house.fetch("https://yokohama.test/api/chat", { method: "POST", headers: { "X-Yokohama-User": "lob2@t.test", "content-type": "application/json" }, body: '{"text":"hello"}' });
    expect(posted.status, await posted.text()).toBe(201);
    expect(await listing, "the second caller hears the lobby's frame, not an end").toContain('"event":"house"');
    expect(await heldOpen(house, await callerTag("lob2@t.test")), "one OPEN socket under the second caller's tag").toBe(1);
    expect(await held(house, await callerTag("lob@t.test"))).toBe(8);
    expect(await held(house, LOBBY_TAG), "every lobby socket still carries the lobby tag: the first caller's eight and the second's one").toBe(9);
    // The lobby's TOTAL count site (the re-review's gap — a mutant per count site): the first caller leaves 1,024
    // refusals dangling, never answering their closes, and a third caller is still admitted; a total that counted
    // CLOSING sockets would answer the lobby's sentence (measured: one identity filled the lobby in 1.9 s in the
    // review's probe; the whole cell takes about 0.4 s here and under 3 s there — hence its own 15 s bound).
    for (let i = 0; i < LOBBY_TOTAL; i += 1) kept.push(await lobby("lob@t.test"));
    expect(await held(house), "held, CLOSING").toBeGreaterThanOrEqual(LOBBY_TOTAL);
    expect(await heldOpen(house), "and not counted: the first caller's eight and the second's one").toBe(9);
    const third = await lobby("lob3@t.test");
    expect(third.status).toBe(101);
    kept.push(third);
    const thirdWs = third.webSocket!;
    thirdWs.accept();
    const heard = new Promise<string>((resolve) => thirdWs.addEventListener("message", (e) => resolve(String(e.data)), { once: true }));
    const again = await house.fetch("https://yokohama.test/api/chat", { method: "POST", headers: { "X-Yokohama-User": "lob3@t.test", "content-type": "application/json" }, body: '{"text":"again"}' });
    expect(again.status, await again.text()).toBe(201);
    expect(await heard, "admitted past 1,024 dangling refusals").toContain('"event":"house"');
    expect([LOBBY_PER_CALLER, LOBBY_TOTAL], "the lobby's literals (node cannot import house.ts: it loads cloudflare:workers)").toEqual([8, 1024]);
  }, 15_000);
});
```

  (`closeReason`, `BLUE`, `RED`, `CREATE_BODY`, `call`, `tableStub`, `runInDurableObject`, `env` imported as
  `test/sockets.test.ts` imports them; `Tab` and `GREEN` are LOCAL to that file and are copied with `openTab`;
  `HOUSE_NAME` is `../src/slot`'s — Task 3's run; `LOBBY_PER_CALLER` and `LOBBY_TOTAL`
  imported from `../src/house` and pinned HERE, in the pool — `house.ts:1` imports `cloudflare:workers`, which node
  cannot load, so no `*.unit.test.ts` can pin them; `LOBBY_TOTAL` is pinned at its literal; the lobby cell's dangling rows
  exercise the house's TOTAL count site, not the 1,025th admission, which `capVerdict`'s node cell proves; the lobby
  cell carries its own bound as `it`'s third argument, `15_000`, since its dangling rows take about 2 s.) **The house rule for frames** (`sockets.test.ts:27-30`: a sent frame is compared with
  LITERAL bytes): `frames.unit.test.ts`, in Files with no cell so far, gains a cell pinning `CLOSE_TOO_MANY` to the
  literal `4429` and the four sentences to their literal text (measured: M27 goes red there; it stays GREEN in
  `caps.test.ts`, whose comparisons build both sides from the imported constants).

- [ ] **Step 5: Run them to see the fail.** `npx vitest run test/caps.test.ts`. Expected: FAIL — the total cell: the
  sixty-fifth is admitted and framed `signal`; the own cell fails earlier, at its tag count (`held` reads 0 against 8:
  no caller tag yet); the lobby cell by timeout (measured at Task 3's run).

- [ ] **Step 6: The caller tag in `fetch`, and `admit`'s arm.** In `worker/src/table.ts`, `Incoming` gains
  `/** The caller's tag on an upgrade (security S2b, D3): `u:` and their digest, computed here because `admit` runs
  in `answer`'s synchronous turn and a digest is async. */ callerTag?: string;`, and `fetch` sets it:
  `callerTag: transport === "stream" && user !== undefined ? await callerTag(user) : undefined` — with `user` read
  into a local before `incoming` is built (`const user = forwardedCaller(req.headers, this.config);`); the abstract
  `admit(id, user, callerTag, reply)`; `finish` passes `incoming.callerTag`. In `sockets.ts`'s `admit`, when
  `status === 200`:

```ts
      if (status === 200) {
        // **The caps** (security S2b, finding D3; decision D-4): the caller's own count under their tag, and the
        // object's in all. A refused socket is ACCEPTED all the same, sent `end` with the sentence and closed
        // 4429 — the shape `signal.ts` reads as `ended`; the game screen draws it, the lobby does not (`Lobby.tsx`
        // passes no `onEnd`; recorded, AD-4). Answered unupgraded, the client's ladder would reconnect and hammer. A
        // tag outlives hibernation, so the count does too; OPEN sockets alone are counted (`openCount`: a dangling
        // close must not lock anyone out).
        const own = callerTag === undefined ? 0 : openCount(this.ctx.getWebSockets(callerTag));
        const over = capVerdict(own, openCount(this.ctx.getWebSockets()), { perCaller: SOCKETS_PER_CALLER, total: SOCKETS_PER_TABLE });
        this.ctx.acceptWebSocket(server, callerTag === undefined ? [id] : [id, callerTag]);
        if (over !== undefined) {
          const sentence = over === "own" ? TOO_MANY_OWN_TABLE : TOO_MANY_TABLE;
          server.send(frame("end", endAtAdmission(sentence)));
          server.close(CLOSE_TOO_MANY, closeReason(sentence));
          return new Response(null, { status: 101, webSocket: client });
        }
        server.serializeAttachment({ user } satisfies Attachment);
        server.send(frame("signal", body));
      } else {
        this.ctx.acceptWebSocket(server, [id]); // the shared accept, moved INTO this arm as its first statement
        …
      }
```

  (`sockets.ts:59` accepts once today, BEFORE the `if`: that shared line moves into the `else` arm — left where it
  is, the 200 arm would accept twice; deleted, the `else` arm would send on an unaccepted socket; the existing cells
  catch both at once.) **Order matters and is pinned by the first cell's `held == 8` after eight opens and by the
  "not kept" assertion:** the counts are read BEFORE `acceptWebSocket`, so the refused socket does not count itself;
  it is accepted (the frame needs a live socket) and closed, and `getWebSockets` drops it by the next request. The
  house's `admit(instance, user, callerTag)` is the same arm with `LOBBY_TAG`, `LOBBY_PER_CALLER`, `LOBBY_TOTAL`,
  `TOO_MANY_OWN_LOBBY`, `TOO_MANY_LOBBY` and `openCount` (imported from `./table/sockets`), and `house.ts`'s `fetch` computes `callerTag` for an upgrade as the
  table's does. `sockets.ts:50-52`'s doc, "nothing in this object reads the tag, since `sendFrames` frames every
  socket the object holds", becomes "the cap reads the caller's tag (`getWebSockets(callerTag)`); `sendFrames` still
  frames every socket the object holds".

- [ ] **Step 7: Run the pool file, then the whole pool.** Expected: PASS; `test/sockets.test.ts` changes ONE existing
  cell — `every socket is accepted under the table's id as its tag` (`:206`) asserts `[[id], [id]]` today and goes red;
  it now asserts the tags as a SET, `expect(tags.map((t) => t.join("|")).sort()).toEqual([\`${id}|${await callerTag(RED)}\`, \`${id}|${await callerTag(BLUE)}\`].sort())`
  (`getWebSockets()` is not in acceptance order), and its `getWebSockets(id).length` line stays at 2 — and `house.ts`'s
  existing lobby cells are unchanged (every lobby socket still carries `LOBBY_TAG`, which `frameLobby` reads).

- [ ] **Step 8: Commit.**

```bash
git add worker/src/frames.ts worker/src/frames.unit.test.ts worker/src/table/sockets.ts worker/src/table/sockets.unit.test.ts worker/src/table.ts worker/src/house.ts worker/test/caps.test.ts worker/test/sockets.test.ts
git commit -m "worker: a table admits eight sockets per caller and sixty-four in all, the lobby eight and 1,024, a refused upgrade accepted, ended with its sentence and closed 4429 (D3's Worker half) (S2b Task 3)"
```

**Mutants (red where named):** M23 the own arm dropped → the ninth admitted; M24 the tag omitted at
`acceptWebSocket` → the count reads 0; M25 the refusal unupgraded (a 4xx `Response`) → `openTab`'s own unupgraded throw, before the 101 assertion is
reached (measured); M26 the
counts read AFTER `acceptWebSocket` → the eighth tab refused (`expected 7 to be 8`; measured) and the total cell's 56;
M27 `4404` for `CLOSE_TOO_MANY` → `frames.unit.test.ts`'s literal pin (GREEN in `caps.test.ts` alone, whose sides are
both the constant — measured); M28 `>` for `>=` → the ninth admitted; M29 the lobby arm dropped → the lobby cell; M28b `openCount` reading every socket (`.length`) → the total cell's
admission past the dangling refusals (`back` is framed `end`; the `heldOpen` row is the cell's own filter and stays green); M29b the lobby's own count over every lobby socket (a
site-wide eight) → the lobby cell's second caller (its house-frame row reads `end`; the 0-OPEN row is never reached —
the review's R1); M29c the lobby's TOTAL count over every socket (`.length` at the house's total site alone) → the
lobby cell's third caller (the re-review's gap: M28b breaks all four count sites at once and pins no single one —
the "mutant per value class" lesson).

### Task 4: B12's Worker half — the share counted from a live listing, the module's arithmetic — Opus (B12; decision D-5; Review Focus 3)

**Files:**
- Modify: `crates/yokohama-worker/src/receipt.rs` (`Listing`, `ListedObject`, `SHARE_UNCOUNTED`, `ReceiptArchive.share`,
  `set_share_inner`, `share_prefix_inner`, `put`'s quota arm; cells in its test module)
- Modify: `crates/yokohama-worker/src/lib.rs` (the exports `share_prefix`, `set_share`)
- Modify: `worker/scripts/factory-glue.mjs` (`EXPECTED` + 2), `worker/scripts/factory.d.ts.template` (+ 2 members);
  `worker/scripts/factory-glue.test.mjs` is NOT touched — it pins no literal, deriving everything from `EXPECTED`
- Modify: `worker/src/archive.ts` (`save`: the listing before the first dispatch)
- Test: `worker/test/archive.test.ts` (four cells; Task 8 adds a fifth), `worker/test/gates.ts` (`failingList`,
  `ListThrough`, `swapBucket`'s `list` parameter)

**Interfaces:**
- Consumes: `yokohama_web::archive::{caller_segment, Quota, PutError}`, `User::parse`; the receipt test module's
  `r2(caller)`, `call`, `the_committed_save()`, `plan_of`, `OWED`, `RED`; the pool's `bucketText`, `segmentOf`,
  `swapBucket`, `ARCHIVE_WALK`, `WIRE_RED`.
- Produces: the module exports `share_prefix(): string | undefined` and `set_share(listing: string): void` (throws its
  sentence); `SHARE_UNCOUNTED`; the shell's `archive-count-failed` log line.

- [ ] **Step 1: The module's failing cells.** In `receipt.rs`'s test module, beside the root cells. The module's
  import (`receipt.rs:509`, `use super::ReceiptArchive;`) becomes
  `use super::{set_share_inner, share_prefix_inner, ReceiptArchive, SHARE_UNCOUNTED};` (written with the cells, so
  Step 2's RED is one E0432 naming the three missing items), and
  **the first line of each new cell is `let _s = serial();`** — every cell of the module that touches `GAMES` takes it
  (18 do), and without it 3 cells fail in every run (measured 10/10). Every block below was measured green at the
  plan review (the crate 81/81):

```rust
    /// The shell's listing of a share prefix, as `set_share` takes it: `records` pairs, each a record and its
    /// table, under `prefix`; the shell's own `truncated`.
    fn listing(prefix: &str, records: usize, truncated: bool) -> String {
        let objects: Vec<Value> = (0..records)
            .flat_map(|i| {
                [
                    json!({"key": format!("{prefix}rev7/over/g{i}.json"), "size": 1_000}),
                    json!({"key": format!("{prefix}rev7/over/g{i}.table.json"), "size": 100}),
                ]
            })
            .collect();
        json!({"truncated": truncated, "objects": objects}).to_string()
    }

    /// **A browser caller's share, counted over the listing the shell handed in** (security S2b, B12's Worker
    /// half; decision D-5): the production quota's own arithmetic — `DirArchive::usage`'s rule, records being
    /// `.json` keys that are not `.table.json`, bytes every `.json` key's size, the put's OWN two keys excluded —
    /// applied BEFORE a plan is deferred, so a full share uploads nothing, and the route's 507 is the NAS's own
    /// sentence. Mutants: the own-keys exclusion dropped (the half pair's repair refused); `>` for `>=` on the
    /// record count; `truncated` ignored; the arm placed after the deferral (a plan rides out beside the 507).
    #[test]
    fn a_browser_save_is_held_to_the_production_quota_over_the_shells_listing() {
        let _s = serial();
        init_inner(&r2(Some(RED))).unwrap();
        let save = the_committed_save();
        let prefix = format!("test/browser/{}/", caller_segment(&User::parse(RED).unwrap()));
        assert_eq!(share_prefix_inner(), Some(prefix.clone()), "the prefix the shell lists is the object's own share");

        set_share_inner(&listing(&prefix, 256, false)).unwrap();
        let full = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(full.status, 507, "{}", full.body);
        assert_eq!(full.body, json!({"error": yokohama_web::api::archive_full_sentence(256, 128 << 20)}).to_string());
        assert!(full.archive.is_none(), "a full share defers no plan");

        set_share_inner(&listing(&prefix, 255, false)).unwrap();
        let open = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(open.status, 500, "{}", open.body);
        assert!(open.archive.is_some(), "under the quota the plan rides out");

        // The put's own two keys never count against it. A retry of a record ALREADY FILED, in a share of exactly
        // 256: its own pair is among the listing and never counts. (A lone table beside 255 records moves neither
        // bound — a table is never a record and 60,000 bytes is far from 128 MiB — so that row pins nothing.)
        let mut filed: Value = serde_json::from_str(&listing(&prefix, 255, false)).unwrap();
        filed["objects"].as_array_mut().unwrap().extend([
            json!({"key": format!("{prefix}rev7/over/{OWED}.json"), "size": 1_000}),
            json!({"key": format!("{prefix}rev7/over/{OWED}.table.json"), "size": 100}),
        ]);
        set_share_inner(&filed.to_string()).unwrap();
        assert!(call(RED, "POST", "/api/archive", Some(&save)).archive.is_some(), "a retry of a filed record is not a 257th");

        // The half pair's repair AT THE BYTE MARGIN: the table alone is on the share and the share is at its bound
        // less this save, so only the exclusion lets the repair through.
        let sent: Value = serde_json::from_str(&save).unwrap();
        let adding = (sent["record"].as_str().unwrap().len() + sent["table"].as_str().unwrap().len()) as u64;
        let margin = json!({"truncated": false, "objects": [
            {"key": format!("{prefix}rev7/over/margin.json"), "size": (128u64 << 20) - adding},
            {"key": format!("{prefix}rev7/over/{OWED}.table.json"), "size": 60_000}]})
        .to_string();
        set_share_inner(&margin).unwrap();
        assert!(call(RED, "POST", "/api/archive", Some(&save)).archive.is_some(), "the repair at the margin is not refused");

        // The bytes bound: one object whose size reaches the share's bytes less this save.
        let big = json!({"truncated": false, "objects": [{"key": format!("{prefix}rev7/over/big.json"), "size": (128u64 << 20) - 1}]}).to_string();
        set_share_inner(&big).unwrap();
        assert_eq!(call(RED, "POST", "/api/archive", Some(&save)).status, 507, "the bytes bound");

        // Truncated: over by construction; exactly 1,000 objects untruncated: 500 records and their tables, over.
        set_share_inner(&listing(&prefix, 3, true)).unwrap();
        assert_eq!(call(RED, "POST", "/api/archive", Some(&save)).status, 507, "a truncated listing is a full share");
        set_share_inner(&listing(&prefix, 500, false)).unwrap();
        assert_eq!(call(RED, "POST", "/api/archive", Some(&save)).status, 507, "1,000 objects");

        // No listing: fail closed, in the layer's 500 with the module's sentence inside it.
        let uncounted = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(uncounted.status, 500, "{}", uncounted.body);
        assert!(uncounted.body.contains(SHARE_UNCOUNTED), "{}", uncounted.body);
        assert!(uncounted.archive.is_none(), "nothing deferred without a count");

        // A listing a request left behind when the layer refused it before any put is never spent by the next save:
        // `share_prefix` opens every save with no count (fail closed). Without the clear, the next save read
        // `status=500 … deferred_plan=true` against the stale listing (measured).
        let _ = share_prefix_inner();
        set_share_inner(&listing(&prefix, 0, false)).unwrap();
        assert_eq!(call(RED, "POST", "/api/archive", Some("{}")).status, 400, "premise: refused before any put");
        let _ = share_prefix_inner();
        let stale = call(RED, "POST", "/api/archive", Some(&save));
        assert!(stale.body.contains(SHARE_UNCOUNTED), "fail closed: {} {}", stale.status, stale.body);
    }

    /// A server's own save (the empty prefix) is never counted (the NAS's rule), and a browser's listing is spent by
    /// the put that read it: the SAME request dispatched again meets the receipt BEFORE the share arm and needs no
    /// count; a third finds neither. (`r2(None)` is the WORKER door, where a plain name is nobody, so the server
    /// half plays as addresses; the browser half pins its plan key with `pin`, or the second call would get a new
    /// plan key and read 500, not the receipt.)
    #[test]
    fn a_server_games_save_is_never_counted_and_a_listing_is_spent_by_one_put() {
        let _s = serial();
        let _p = pin(1_789_001_300);
        init_inner(&r2(None)).unwrap();
        playing_with_a_move("own", RED, BLUE);
        assert_eq!(share_prefix_inner(), None, "no caller, no share");
        let r = call(RED, "DELETE", "/api/games/own", None);
        assert_eq!(r.status, 500, "premise: the abandon deferred — {}", r.body);
        let plan = plan_of(&r);
        receipt_inner(key(&plan), &outcome("landed", plan["record_key"].as_str().unwrap())).unwrap();
        assert_eq!(call(RED, "DELETE", "/api/games/own", None).status, 204);

        // A browser's save spends its listing in the put that read it; the SAME request dispatched again meets the
        // receipt BEFORE the share arm and needs no count; a third finds neither.
        init_inner(&r2(Some(RED))).unwrap();
        let save = the_committed_save();
        set_share_inner(&listing("", 0, false)).unwrap();
        let first = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(first.status, 500, "premise: the counted save defers its plan — {}", first.body);
        let plan = plan_of(&first);
        receipt_inner(key(&plan), &outcome("landed", plan["record_key"].as_str().unwrap())).unwrap();
        let again = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(again.status, 201, "the receipt is met first and asks for no listing — {}", again.body);
        let third = call(RED, "POST", "/api/archive", Some(&save));
        assert!(third.body.contains(SHARE_UNCOUNTED), "{}", third.body);
    }

    /// `set_share` refuses what the shell never writes — an array, an unknown key, a missing field, a negative
    /// size — in a sentence, and refuses everything with the archive off.
    #[test]
    fn set_share_refuses_a_listing_of_the_wrong_shape_and_the_archive_off() {
        let _s = serial();
        init_inner(&r2(Some(RED))).unwrap();
        for bad in [r#"[]"#, r#"{"truncated": false}"#, r#"{"truncated": false, "objects": [], "cursor": "x"}"#, r#"{"truncated": false, "objects": [{"key": "k", "size": -1}]}"#] {
            assert!(set_share_inner(bad).is_err(), "{bad}");
        }
        init_inner(&json!({"identity": "worker", "archive": "off", "caller": null, "lobby_chat": null, "chat_keep": null, "chat_text": null, "diagnostic_seeds": true}).to_string()).unwrap();
        assert!(set_share_inner(&listing("browser/x-000000000000/", 0, false)).unwrap_err().contains("is off"));
        assert_eq!(share_prefix_inner(), None);
    }
```

- [ ] **Step 2: Run them to see the fail.** `cargo test -p yokohama-worker receipt::` — the names do not exist.

- [ ] **Step 3: The module.** In `receipt.rs`:

```rust
/// A browser save with no count before it (security S2b, B12's Worker half): the shell lists the share before
/// every save and hands the listing to `set_share`; a put that finds none refuses rather than upload uncounted,
/// and the route's 500 carries this, which the outbox retries.
pub(crate) const SHARE_UNCOUNTED: &str = "the caller's share was not counted before this save";

/// One object of the shell's listing: its key and size, as R2's `list` reports them.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListedObject {
    key: String,
    size: u64,
}

/// The shell's listing of the share prefix (decision D-5): `truncated` as R2 reported it — a listing of more
/// than 1,000 objects is over the quota by construction — and every object's key and size.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Listing {
    truncated: bool,
    objects: Vec<ListedObject>,
}

impl Listing {
    /// `DirArchive::usage`'s arithmetic over a listing: records are `.json` keys that are not `.table.json`,
    /// bytes every `.json` key's size, and the put's own two keys never count (a half pair's repair). A
    /// truncated listing counts as `usize::MAX` records.
    fn usage(&self, own: [&str; 2]) -> (usize, u64) {
        if self.truncated {
            return (usize::MAX, u64::MAX);
        }
        let mut records = 0usize;
        let mut bytes = 0u64;
        for object in self.objects.iter().filter(|o| o.key.ends_with(".json") && !own.contains(&o.key.as_str())) {
            bytes = bytes.saturating_add(object.size);
            if !object.key.ends_with(".table.json") {
                records += 1;
            }
        }
        (records, bytes)
    }
}
```

  `ReceiptArchive` gains `share: Mutex<Option<Listing>>` (built `None`), and `put`, after the receipt lookup and
  before `pending`:

```rust
        // **The share** (security S2b, B12's Worker half; decision D-5): for a browser's save, the listing the
        // shell handed in, TAKEN — one count serves one put — and judged by the production quota's arithmetic
        // before any plan is deferred. None means the shell did not count (its list failed, or a defect): refused,
        // never uploaded uncounted. A server's own save (the empty prefix) is never counted, as on the NAS.
        if prefix.starts_with("browser/") {
            let listing = self.share.lock().expect("the share").take().ok_or_else(|| PutError::Failed(SHARE_UNCOUNTED.to_string()))?;
            let (records, bytes) = listing.usage([record_key.as_str(), table_key.as_str()]);
            let quota = yokohama_web::archive::Quota::PRODUCTION;
            let adding = (record_json.len() + table_json.len()) as u64;
            if records >= quota.records || bytes.saturating_add(adding) > quota.bytes {
                return Err(PutError::Quota { records: quota.records, bytes: quota.bytes });
            }
        }
```

  and the two inner functions:

```rust
/// `share_prefix()`: the prefix the shell lists before a browser's save — the root for this object's caller
/// (`test/` for a `.test` address, as `root` reads it) then `browser/<segment>/` — or `None` with no caller or the
/// archive off, when nothing is counted; and it clears any listing an earlier request left.
pub fn share_prefix_inner() -> Option<String> {
    let guard = RECEIPTS.lock().expect("the host's receipts");
    let archive = guard.as_ref()?;
    // A save begins with NO count: a listing an earlier request set and the layer refused before any put must not be
    // spent by this one if its own listing fails (fail closed).
    *archive.share.lock().expect("the share") = None;
    let caller = archive.caller.as_deref()?;
    let user = User::parse(caller).ok()?;
    let root = if is_test_identity(caller) { TEST_ROOT } else { "" };
    Some(format!("{root}browser/{}/", caller_segment(&user)))
}

/// `set_share(listing)`: the shell's listing of the share, installed for the next put to spend. Refused, with a
/// sentence: a listing that is not the object `{"truncated", "objects": [{"key", "size"}]}` — an array, an
/// unknown key, a missing field, a size that is no `u64` — and any listing with the archive off.
pub fn set_share_inner(listing: &str) -> Result<(), String> {
    let guard = RECEIPTS.lock().expect("the host's receipts");
    let archive = guard
        .as_ref()
        .ok_or_else(|| format!("{ARCHIVE_VAR} is off, so no share is counted and no listing is taken"))?;
    let value: Value = serde_json::from_str(listing).map_err(|e| format!("a share listing is not JSON: {e}"))?;
    if !value.is_object() {
        return Err("a share listing is the object {\"truncated\", \"objects\"}".to_string());
    }
    let parsed: Listing = serde_json::from_value(value).map_err(|e| format!("a share listing does not parse: {e}"))?;
    *archive.share.lock().expect("the share") = Some(parsed);
    Ok(())
}
```

  and in `lib.rs`, beside `receipt`:

```rust
/// [`receipt::share_prefix_inner`] for the shell.
#[wasm_bindgen]
pub fn share_prefix() -> Option<String> {
    receipt::share_prefix_inner()
}

/// [`receipt::set_share_inner`] for the shell: a refusal is thrown as its sentence.
#[wasm_bindgen]
pub fn set_share(listing: &str) -> Result<(), JsValue> {
    receipt::set_share_inner(listing).map_err(|e| JsValue::from_str(&e))
}
```

  `init_inner` builds the receipt archive with `share: Mutex::new(None)` through `ReceiptArchive::new`. `EXPECTED` in
  `factory-glue.mjs` gains `"share_prefix", "set_share"` after `"expire"`, the template two members
  (`share_prefix: typeof G.share_prefix; set_share: typeof G.set_share;`); `factory-glue.test.mjs` pins no literal and
  is not touched.

  **Every existing cell that makes a browser's save through the receipt archive sets a count first** — the new put
  arm refuses a browser save with no listing, and four existing cells save without one (measured: `76 passed; 4
  failed` without this — the four cells named): `set_share_inner(&listing("", 0, false)).unwrap();` before the `call(…, "POST", "/api/archive", …)`
  in `the_test_root_reads_the_full_address_never_the_truncated_segment` (the loop over three callers),
  `a_plain_name_never_takes_the_test_root` (the browser loop), `a_browsers_save_sent_by_another_caller_than_the_objects_is_refused`
  (the matching pair; the two refused pairs fail in `root()` first and need none) and
  `a_browsers_save_answers_each_receipt_as_the_layer_answers_its_outcome` (the first save, and again before `again`,
  the receipt having been spent).

- [ ] **Step 4: Run the Rust cells and the glue cells.** `cargo test -p yokohama-worker && cd worker && npm run
  build:module:debug && npm run test:scripts`. Expected: PASS (the crate's count 77 + 3 = 80; `cargo test -p yokohama-worker receipt::`
  prints 23 passed, 57 filtered out — record what the command prints).

- [ ] **Step 5: The pool's failing cells.** In `worker/test/gates.ts`, beside `failingPuts`:

```ts
/** A bucket whose `list` REJECTS as an unreachable R2 answers, every other call reaching the real bucket. */
export function failingList(): ListThrough {
  return () => Promise.reject(new Error(R2_UNREACHABLE));
}
```

  with `swapBucket(stub, put, list?)` extended to swap `list` as it swaps `put` — three lines, measured:
  `export type ListThrough = (real: R2Bucket, options: R2ListOptions | undefined) => Promise<R2Objects>;`;
  `swapBucket`'s new parameter `list: ListThrough = (real, options) => real.list(options)`; the swapped bucket's
  `list: (options?: R2ListOptions) => list(real, options)`. In `worker/test/archive.test.ts`, whose `./gates` import
  gains `failingList`:

```ts
describe("B12's Worker half — the share counted from a live listing (security S2b, decision D-5)", () => {
  /** A browser's save of the walk's finished pair, through red's archive object, as the Worker forwards it. */
  function browserSave(stub: DurableObjectStub<Archive>, who: string): Promise<Response> {
    const body = JSON.stringify(ARCHIVE_WALK[0]!.body);
    return stub.fetch("https://yokohama.test/api/archive", { method: "POST", headers: { "X-Yokohama-User": who, "content-type": "application/json" }, body });
  }
  const archiveStub = (who: string) => env.ARCHIVER.get(env.ARCHIVER.idFromName(`archive:${who}`));

  // Mutants: the listing not handed to the module (every save SHARE_UNCOUNTED); the prefix wrong (an empty count
  // under 256 fakes); `truncated` not forwarded (the truncated-listing cell below).
  it("refuses a 257th record with the NAS's 507 and uploads nothing, and lands the 256th", async () => {
    const who = "quota@t.test";
    const prefix = `test/browser/${await segmentOf(who)}/`;
    for (let i = 0; i < 255; i += 1) {
      await env.ARCHIVE!.put(`${prefix}rev7/over/g${i}.json`, "{}");
      await env.ARCHIVE!.put(`${prefix}rev7/over/g${i}.table.json`, "{}");
    }
    const landed = await browserSave(archiveStub(who), who);
    expect(landed.status, await landed.clone().text()).toBe(201);
    const key = `${prefix}rev7/over/${(ARCHIVE_WALK[0]!.body as { table: string }).table.match(/"id": "([^"]+)"/)![1]}.json`;
    expect(await bucketText(key)).not.toBeNull();
    // Now 256 records. A 257th is a second save at another id: the walk's first send names the table `wire` in its
    // TABLE text alone (the record carries no id — `replaceOnce` over it would throw), so rewrite the table only.
    const second = JSON.stringify({
      record: (ARCHIVE_WALK[0]!.body as { record: string }).record,
      table: replaceOnce((ARCHIVE_WALK[0]!.body as { table: string }).table, '"id": "wire"', '"id": "wire-257"'),
    });
    const refused = await archiveStub(who).fetch("https://yokohama.test/api/archive", { method: "POST", headers: { "X-Yokohama-User": who, "content-type": "application/json" }, body: second });
    expect(refused.status).toBe(507);
    expect(JSON.parse(await refused.text())).toEqual({ error: "your archive here is full — 256 records or 128 MiB; nothing was written, and the game is kept on this device" });
    expect(await bucketText(`${prefix}rev7/over/wire-257.json`), "nothing uploaded").toBeNull();
    expect(await bucketText(`${prefix}rev7/over/wire-257.table.json`), "not even the table").toBeNull();
  });

  it("refuses a save whose share could not be listed, in the layer's 500 with the module's sentence, uploading nothing", async () => {
    const who = "unlisted@t.test";
    const stub = archiveStub(who);
    const swapped = await swapBucket(stub, (real, k, v, o) => real.put(k, v, o), failingList());
    try {
      const { result, lines } = await logsDuring(() => browserSave(stub, who));
      expect(result.status).toBe(500);
      expect(await result.text()).toContain("the caller's share was not counted before this save");
      expect(lines.some((l) => (l as { event: string }).event === "archive-count-failed")).toBe(true);
    } finally {
      await swapped.restore();
    }
    expect(await bucketText(`test/browser/${await segmentOf(who)}/rev7/over/wire.json`)).toBeNull();
  });

  // The SHELL forwarding `truncated` (M32 covers the module ignoring it): no pool listing exceeds 1,000 objects, so
  // a bucket whose `list` answers truncated is swapped in through `swapBucket`'s list parameter (Task 4's run: the
  // comment's mutant survived 242/242 without this cell; measured red under it — `expected 201 to be 507`).
  it("a listing R2 reports truncated is a full share: the 507, nothing uploaded", async () => {
    const who = "truncated@t.test";
    const stub = archiveStub(who);
    const swapped = await swapBucket(stub, (real, k, v, o) => real.put(k, v, o), async (real, options) => ({ ...(await real.list(options)), truncated: true, cursor: "more" }) as R2Objects);
    try {
      const res = await browserSave(stub, who);
      expect(res.status, await res.clone().text()).toBe(507);
    } finally {
      await swapped.restore();
    }
    expect(await bucketText(`test/browser/${await segmentOf(who)}/rev7/over/wire.json`)).toBeNull();
  });

  // The SHELL forwarding `size` (the review of Task 4: the shell forwards three values per listing in one statement
  // — `truncated`, `key`, `size` — and the lesson is a mutant per value class; `size: 0` survived 243/243 without
  // this cell, measured red under it — `expected 201 to be 507`). The sizes alone reach the bytes bound.
  it("a listing whose sizes reach 128 MiB is a full share: the 507, nothing uploaded", async () => {
    const who = "sized@t.test";
    const stub = archiveStub(who);
    const swapped = await swapBucket(stub, (real, k, v, o) => real.put(k, v, o), async (real, options) => ({ ...(await real.list(options)), objects: [{ key: `${options!.prefix}rev7/over/big.json`, size: 128 * 1024 * 1024 }] }) as unknown as R2Objects);
    try {
      const res = await browserSave(stub, who);
      expect(res.status, await res.clone().text()).toBe(507);
    } finally {
      await swapped.restore();
    }
    expect(await bucketText(`test/browser/${await segmentOf(who)}/rev7/over/wire.json`)).toBeNull();
  });
});
```

  (`ARCHIVE_WALK[0]`'s body shape and `replaceOnce` are the file's own idioms — see `oneSeatGame` above it; the
  walk's first send names the table `wire` — the record carries no id, counted at the plan review — and the 507
  sentence equals `archive_full_sentence(256, 128 << 20)`, both read at the plan review.)

- [ ] **Step 6: The shell.** In `worker/src/archive.ts`'s `save`, before `const first = this.dispatch(…)`
  (measured: the `try` is what keeps the open project's R-C10 cell at 401 — `guarded` rethrows anything that is not a
  trap, and `init`'s refusal of a key the grammar rejects is a thrown STRING that only `dispatch`'s own catch turns
  into `callerRefused`'s 401; a bare `guarded` call here read `[500, "the server could not answer — try again"]` and
  reset the object):

```ts
    // **The share, counted live** (security S2b, B12's Worker half; decision D-5): the object's share prefix from
    // the module, one R2 listing (a Class A operation), and the listing handed to the module, whose put applies
    // the production quota's own arithmetic before any plan is deferred. A listing that fails is logged and NOT
    // handed over: the put then refuses the save uncounted (`SHARE_UNCOUNTED`), the layer's 500, which the outbox
    // retries. A `test/` address lists under `test/`; a server game's own save is never counted (its prefix is
    // empty, and `share_prefix` answers `undefined` for every object but an Archive object's).
    let prefix: { ok: string | undefined } | { trapped: string };
    try {
      prefix = instance.guarded((mod) => mod.share_prefix());
    } catch (e) {
      // `init` refused this object's caller (the key grammar's): nothing to count, and `dispatch` below refuses it
      // exactly as it always did — the layer's 401 through `callerRefused`, never a thrown string.
      if (typeof e !== "string") throw e;
      prefix = { ok: undefined };
    }
    if ("trapped" in prefix) {
      log("trap", { id, call: "share_prefix", error: prefix.trapped });
      return respond(500, errorBody(TRAP));
    }
    if (prefix.ok !== undefined) {
      let listed: R2Objects | undefined;
      try {
        listed = await (this.env.ARCHIVE as R2Bucket).list({ prefix: prefix.ok, limit: 1000 });
      } catch (e) {
        log("archive-count-failed", { id, error: String(e) });
      }
      if (listed !== undefined) {
        const listing = JSON.stringify({ truncated: listed.truncated, objects: listed.objects.map((o) => ({ key: o.key, size: o.size })) });
        const set = instance.guarded((mod) => mod.set_share(listing));
        if ("trapped" in set) {
          log("trap", { id, call: "set_share", error: set.trapped });
          return respond(500, errorBody(TRAP));
        }
      }
    }
```

  The class doc's receipt loop gains this as its step 0; `objectFor`'s archive comment in `index.ts` is unchanged.

- [ ] **Step 7: Run the pool.** `npm run build:module:debug && npm test`. Expected: PASS, the corpus replay's eight
  archive rows included (each lists the walk caller's share — non-empty once the first save has landed), and
  `test/open/door.test.ts`'s R-C10 cell ("a claim the key grammar refuses is the layer's 401 at the archive, as on the
  NAS") still reads 401.

- [ ] **Step 8: Commit.**

```bash
git add crates/yokohama-worker/src/receipt.rs crates/yokohama-worker/src/lib.rs worker/scripts/factory-glue.mjs worker/scripts/factory.d.ts.template worker/src/archive.ts worker/test/archive.test.ts worker/test/gates.ts
git commit -m "worker: a browser's save is held to the production quota over a live listing of its share — the shell lists, the module counts with the NAS's arithmetic, a full share uploads nothing and a save uncounted is refused (B12's Worker half) (S2b Task 4)"
```

**Mutants (red where named):** M30 the own-keys exclusion dropped (or either key alone, M30b/M30c) → the
filed-retry and margin rows (the original lone-table row survived all three — measured); M31 `>` for `>=` on records
→ the 256 row; M32 `truncated` ignored → the truncated row; M33 the quota arm after the deferral → "defers no plan";
M34 the listing not TAKEN → the uncounted row (a stale listing serves twice); M35 `share_prefix` without the `test/`
root → the prefix row; M36 the shell listing nothing → every pool save `SHARE_UNCOUNTED`; M37 the list failure
swallowed into an empty listing → the failing-list cell lands a save; M37b the `share_prefix` call outside the `try`
→ the open project's R-C10 cell reads 500; M37c `share_prefix` not clearing the share → the stale-listing row; M37d the shell writing `truncated: false`
whatever R2 reported → the truncated-listing pool cell (`expected 201 to be 507`; survived every other cell, measured);
M37e the shell forwarding `size: 0` → the sized-listing pool cell (the review's X0: survived 243/243 without it).

### Task 5: C1 with D24 — the open door's key rule, a named policy; the Worker module; the shell's one-key-one-object collapse; the open project's cells — Opus (C1, D24; decision D-2; AD-3; Review Focus 5)

**Files:**
- Modify: `crates/yokohama-web/src/identity.rs` (`ClaimPolicy`, `ADDRESS_NOT_A_NAME`, `resolve`, `admit`, the extractor;
  cells), `crates/yokohama-web/src/lib.rs` (`AppState.claims`, the builders, `with_claim_policy`, the adapter; cells),
  `crates/yokohama-web/src/main.rs` (its doc: "two different emails ARE two different people"),
  `crates/yokohama-web/src/archive.rs` (`SEGMENT_DIGEST`'s open-door doc), `crates/yokohama-web/tests/wire_fixtures.rs`
  (one `resolve` call site, `:414`), `crates/yokohama-web/tests/start_refusal.rs` (one binary cell)
- Modify: `crates/yokohama-worker/src/lib.rs` (`identity_of`, the `:736` call; cells), `crates/yokohama-worker/src/receipt.rs`
  (the test rows that send an ADDRESS through the open door move to the worker door)
- Modify: `worker/src/identity.ts` (`claimedIdentity` collapses space runs, AD-3), `worker/src/identity.unit.test.ts`
  (one expectation), `worker/src/index.ts` (the `case "archive"` comment's two lines, Step 7b)
- Modify: `worker/test/open/door.test.ts` (the rows that claim an address: moved or inverted; C1's own cells; the
  spelling-race cell)
- The CLIENT (`web/src/nameRule.ts`, `SignIn.tsx`, `nameRule.test.ts`, `NewGame.tsx`, the two web cells C1 inverts and
  the e2e identities) is **Task 5b's**, not this task's.

**Interfaces:**
- Consumes: `User::is_email`, `IdentityMode`, `resolve`'s callers (the ADAPTER — every `/api` row but three — and the
  `Identity` extractor's three transport routes; the Worker module's `identity_of` and its `:736` call; the browser
  host passes its caller straight to `dispatch` and is UNTOUCHED).
- Produces: `identity::ClaimPolicy::{NamesOnly, Diagnostic}`, `identity::ADDRESS_NOT_A_NAME`, `AppState.claims`,
  `AppState::with_claim_policy`; `claimedIdentity`'s collapsed key (one name, one Archive object, one write-limiter key).

- [ ] **Step 1: The layer's failing cells.** In `identity.rs`'s test module:

```rust
    /// **The open door's key rule** (security S2b, finding C1; R-SEC8(c); decision D-2): under `open` with
    /// `NamesOnly` — every production constructor — a key with the email shape is refused with one sentence, a
    /// `.test` address included (the `test/` root is the dev door's and a server game's, never the open door's); a
    /// plain name is admitted; under `Diagnostic` — the fixtures and the corpus walk — every key the grammar
    /// accepts is a caller, as before. Under `cf-access` and `worker` the policy changes nothing. Mutants: the arm
    /// reading `Diagnostic` as `NamesOnly` (the corpus walk's `red@wire.test` refused); the sentence the absent
    /// caller's; the `.test` suffix excepted.
    #[test]
    fn under_open_names_only_refuses_an_address_and_diagnostic_admits_every_key() {
        let at = |mode: IdentityMode, policy: ClaimPolicy, value: &str| resolve(mode, policy, &headers(&[(mode.header(), value)]));
        assert_eq!(at(IdentityMode::Open, ClaimPolicy::NamesOnly, "  Ben   Castle ").unwrap().as_str(), "ben castle");
        for address in ["red@wire.test", "Owner@Example.com", "a+b@x.test", "someone@example.test"] {
            assert_eq!(at(IdentityMode::Open, ClaimPolicy::NamesOnly, address).unwrap_err(), ADDRESS_NOT_A_NAME, "{address}");
            assert_eq!(at(IdentityMode::Open, ClaimPolicy::Diagnostic, address).unwrap().as_str(), address.trim().to_ascii_lowercase());
        }
        for mode in [IdentityMode::CfAccess, IdentityMode::Worker] {
            for policy in [ClaimPolicy::NamesOnly, ClaimPolicy::Diagnostic] {
                assert_eq!(at(mode, policy, "Red@Wire.TEST").unwrap().as_str(), "red@wire.test", "{}", mode.name());
                assert_eq!(at(mode, policy, "ben").unwrap_err(), resolve(mode, policy, &headers(&[])).unwrap_err(), "{}: a name is nobody", mode.name());
            }
        }
        // The grammar still speaks first: what is no key at all is refused in its own words under every policy.
        assert_eq!(at(IdentityMode::Open, ClaimPolicy::NamesOnly, "..").unwrap_err(), User::parse("..").unwrap_err());
        assert_eq!(ADDRESS_NOT_A_NAME, "under the open door a name is a name — an address is not accepted here: choose a plain name");
    }
```

  and in `lib.rs`'s test module, beside the seed-policy cells (`state.games.seed_policy()` at the production and
  in-memory constructors):

```rust
    /// **Every production constructor refuses an address under the open door; the fixtures admit every key**
    /// (security S2b, C1; decision D-2) — the seed rule's shape (B3): `production` builds `NamesOnly`,
    /// `in_memory` builds `Diagnostic`, and the adapter (every `/api` row but three) and the extractor (the three
    /// transport routes) resolve with the state's policy, so a route-level cell reads the refusal in the layer's
    /// own 401 and `/api/me` answers `user: null` for the same claim.
    #[tokio::test]
    async fn the_production_state_refuses_an_address_claim_under_the_open_door_and_the_fixtures_admit_one() {
        assert_eq!(AppState::in_memory().claims, identity::ClaimPolicy::Diagnostic);
        // Built as `main` builds it — the seed-policy cell's argument list (`the_production_state_honours_a_seed_at_a_solo_table_only`), the door `Open`.
        let production = AppState::production(Arc::new(MemStore::default()), identity::IdentityMode::Open, chat::ChatConfig::default(), Arc::new(archive::NoArchive), Multiplayer(true), sweep::Ttls::none()).expect("a production state over memory");
        assert_eq!(production.claims, identity::ClaimPolicy::NamesOnly);

        let state = Arc::new(AppState::with_store(Arc::new(MemStore::default()), identity::IdentityMode::Open).with_claim_policy(identity::ClaimPolicy::NamesOnly));
        let create = serde_json::json!({"players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seats": [{"user": "red@wire.test"}, {"open": true}]});
        let (status, body) = post_as(&state, "red@wire.test", "/api/games", create.clone()).await;
        assert_eq!((status, body["error"].as_str()), (StatusCode::UNAUTHORIZED, Some(identity::ADDRESS_NOT_A_NAME)));
        let me = get_as(&state, "red@wire.test", "/api/me").await;
        assert_eq!(me.1["user"], serde_json::Value::Null, "/api/me never refuses: it answers nobody");
        let named = serde_json::json!({"players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seats": [{"user": "ben"}, {"open": true}]});
        assert_eq!(post_as(&state, "ben", "/api/games", named).await.0, StatusCode::CREATED);
    }

    /// The EXTRACTOR's three transport routes resolve with the state's policy too — the adapter serves every other
    /// `/api` row, so a cell over `/api/games` alone would leave the extractor's call unpinned (M44 survived the whole
    /// suite that way at the plan review; this cell is where it goes red).
    #[tokio::test]
    async fn the_extractors_routes_resolve_with_the_states_policy() {
        let state = Arc::new(
            AppState::with_store(Arc::new(MemStore::default()), identity::IdentityMode::Open)
                .with_claim_policy(identity::ClaimPolicy::NamesOnly),
        );
        let (status, body) = get_as(&state, "red@wire.test", "/api/games/nobody/events").await;
        assert_eq!((status, body["error"].as_str()), (StatusCode::UNAUTHORIZED, Some(identity::ADDRESS_NOT_A_NAME)));
        let (status, _) = get_as(&state, "ben", "/api/games/nobody/events").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "a name passes the extractor and meets the missing game");
    }
```

  (`post_as`/`get_as` are the file's request helpers, `lib.rs:3206` and `:3215`.)

- [ ] **Step 2: Run them to see the fail.** `cargo test -p yokohama-web identity:: && cargo test -p yokohama-web
  the_production_state_refuses` — `ClaimPolicy` does not exist.

- [ ] **Step 3: The policy.** In `identity.rs`:

```rust
/// **The open door's key rule** (security S2b, finding C1; R-SEC8(c); decision D-2) — the seed rule's shape
/// (`games::SeedPolicy`): which KEYS the open door admits as a caller. Under `cf-access` and `worker` it changes
/// nothing, since an address is the only shape admitted there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimPolicy {
    /// Production: under `open` a caller is a plain NAME, and a key with the email shape is refused with
    /// [`ADDRESS_NOT_A_NAME`]. An `@` then always means a verified address, on every host, so a door flip over
    /// shared storage can hand nobody an Access player's seats or archive folder by typing their address. It stops
    /// no impersonation under the open door, where a name is a label and a key and never a credential.
    NamesOnly,
    /// The fixtures' and the corpus walk's: every key the grammar accepts is a caller, as before S2b. Built by
    /// `AppState::with_store` and the `in_memory*` constructors; `AppState::production` never builds it, and the
    /// Worker module has no arm for it.
    Diagnostic,
}

/// The 401 an address-shaped claim meets under the open door with [`ClaimPolicy::NamesOnly`]. One sentence on
/// both hosts, since the rule lives here and nowhere in a shell; the client's name step refuses the same input
/// in its own words before any request.
pub const ADDRESS_NOT_A_NAME: &str =
    "under the open door a name is a name — an address is not accepted here: choose a plain name";
```

  `resolve(mode, policy, headers)` and `admit(mode, policy, user)` gain the parameter; `admit`'s match:

```rust
    match mode {
        IdentityMode::Open if policy == ClaimPolicy::NamesOnly && user.is_email() => Err(ADDRESS_NOT_A_NAME.to_string()),
        IdentityMode::Open => Ok(user),
        IdentityMode::CfAccess | IdentityMode::Worker if user.is_email() => Ok(user),
        IdentityMode::CfAccess | IdentityMode::Worker => Err(absent(mode)),
    }
```

  The ADAPTER (`lib.rs`'s `adapter`, `:1032`; every `/api` row but three, routes at `:1257-1258`) and the `Identity`
  extractor (`identity.rs:282`; `…/events`, `…/stream`, `/api/stream`) BOTH resolve with
  `resolve(state.identity, state.claims, &parts.headers)` — two sites, not one. In `lib.rs`: `AppState` gains
  `pub claims: identity::ClaimPolicy`; the constructors chain `new → production → with_store_chat_archive_and_log`,
  and only `with_store_chat_and_archive` opts into `Diagnostic` today (the seed rule's shape), so: the literal in
  `with_store_chat_archive_and_log` builds `claims: ClaimPolicy::NamesOnly`, which makes `production` (which calls it
  directly) `NamesOnly` by construction; `with_store_chat_and_archive` appends `.with_claim_policy(ClaimPolicy::Diagnostic)`
  after `.with_seed_policy(…)`, and `with_store`, `with_store_and_chat` and the `in_memory*` builders inherit it;
  `with_claim_policy(self, policy) -> Self`. Every other `resolve(`/`admit(` call site gains the policy —
  `grep -rn "identity::resolve\|identity::admit\|resolve(\|admit(" crates/yokohama-web crates/yokohama-worker --include='*.rs'`
  (the crate's `src` alone misses two): `tests/wire_fixtures.rs:414` becomes
  `yokohama_web::identity::resolve(self.state.identity, self.state.claims, &axum::http::HeaderMap::new())` (E0061 and
  a commit that does not compile otherwise); the cells in `identity.rs` pass `ClaimPolicy::Diagnostic` where they test
  the grammar and the mode and `NamesOnly` where they test this rule; and the seed cell
  `the_production_state_honours_a_seed_at_a_solo_table_only` (`lib.rs:2851`), which builds `production` at the OPEN
  door and posts as `WHO = "red@wire.test"` (`:1283`), claims `"red"` instead — its seat user and its two `post_as`
  callers (three sites) — as `start_refusal.rs`'s seed cell already does (it read 401 where it expected 400 otherwise; measured).
  Four docs become true in the same commit: `identity.rs:240` ("under `open` with `Diagnostic` every key the grammar
  accepts is a caller; with `NamesOnly` an address is refused"), `main.rs:92-97` ("two different emails ARE two
  different people" — under the open door, two different NAMES), `archive.rs:202-206` (`SEGMENT_DIGEST`'s open-door
  argument, restated now that the open door admits no address) and `AppState::production`'s doc (`NamesOnly`).

- [ ] **Step 4: Run the layer's suite.** `cargo test -p yokohama-web`. Expected: PASS, every existing cell standing —
  those that build `production` at the open door claim names; `wire_fixtures` clean without the flag (the corpus does
  not move: 117 files byte-identical, measured).

- [ ] **Step 5: The binary.** In `tests/start_refusal.rs`, beside the seed rule's cell under `open`
  (`#[cfg(feature = "open-door")]`): spawn the server on `open`; `post(port, "red@wire.test", "/api/games", &body)`
  reads `(401, {"error": ADDRESS_NOT_A_NAME})`; `post(port, "ben", "/api/games", &named_body)` reads 201. Run the
  binary tests: `cargo test -p yokohama-web --test start_refusal`. Expected: PASS (the catcher of `main → production →
  NamesOnly`, which no in-process cell runs).

- [ ] **Step 6: The Worker module.** `identity_of` in `crates/yokohama-worker/src/lib.rs` imports
  `use yokohama_web::identity::{admit, resolve, ClaimPolicy, User};` and passes `ClaimPolicy::NamesOnly` to both
  `admit` (the fallback for a value no header can carry, `:359`) and `resolve` (`:363`), with the comment `// The
  production rule, and no other: the module has no diagnostic arm (security S2b, C1; decision D-2) — the pool's open
  project claims names.`; its doc (`lib.rs:338`) says the module runs `NamesOnly`; `lib.rs:736` becomes
  `resolve(IdentityMode::Worker, ClaimPolicy::NamesOnly, &http::HeaderMap::new())`. Cells in the module's tests: under
  `open`, `identity_inner(Some("red@wire.test"))` reads `(401, {"error": ADDRESS_NOT_A_NAME})` and
  `identity_inner(Some("ben"))` 200; under `worker`, an address still resolves; and the fallback path, which nothing
  pinned (M42a survived 81/81 — measured):

```rust
        // The fallback for a value no header can carry takes the same rule at its own `admit`.
        let carried = identity_inner(Some("red@wire.test\r"));
        assert_eq!((carried.status, carried.body.as_str()), (401, want.as_str()), "the fallback path");
```

  **Then the rows that send an address through the open door move to the worker door, where the `test/` root still
  applies.** The addresses arrive through LOOP variables (`receipt.rs:881, 891`), not literals — a grep for
  `r2_at(Some(".*@` prints nothing at `85c8dd02` already (exit 1) and pins nothing — so each row gets its own door,
  with BLUE as the partner (`"joiner"` is nobody at the worker door). The two loop heads in
  `a_plain_name_never_takes_the_test_root` become (measured: the crate green with them):

```rust
        for (creator, joiner, root, door) in [("a.test", "joiner", "", "open"), (RED, BLUE, "test/", "worker")] {
            init_inner(&r2_at(None, door)).unwrap();
            playing_with_a_move("plain", creator, joiner);
```

```rust
        for (caller, root, door) in [("joiner.test", "", "open"), (RED, "test/", "worker")] {
            init_inner(&r2_at(Some(caller), door)).unwrap();
```

  The cell's doc gains "the partner at each root is an address in `.test`, which still takes it — at the worker door,
  since under the open door an address is no caller (C1)". The check is the named test, red before the move and green
  after: `cargo test -p yokohama-worker a_plain_name_never_takes_the_test_root`. Then `cargo test -p yokohama-worker`.
  Expected: PASS.

- [ ] **Step 7: The open pool project.** In `worker/test/open/door.test.ts`, three cells hold address claims, and a
  grep for `@` finds only two of them (the Review Focus 1 cell's `?claimed=a%2Bb%40x.test` carries no literal `@`);
  row by row:
  - `:160` (`evil@x.test`, the forged-header row): not a claim — those headers are stripped; UNCHANGED.
  - `:184–222`, the Review Focus 1 cell: its `spaced` half (`ben castle`, `?claimed=ben+castle`) stays — it still kills
    the raw-read and `decodeURIComponent` mutants; its `plus` half (`a+b@x.test`) becomes a REFUSAL row: an upgrade of
    `/api/stream?claimed=a%2Bb%40x.test` reads 401 unupgraded, with the body's sentence probed and pinned.
  - `:271`, `:279` (`boss@t.test` at the operator routes): PASS unchanged (`dispatch` answers `NoRoute` before
    identity); KEEP them — they show an operator-looking address is no operator under `open`.
  - `:317`, `:345` (the `test/`-root rows, `["red@t.test", "address-abandon", "test/"]` and `["red@t.test", "test/"]`):
    leave the loops; the cell's title loses "; an address in .test inside it", and its anti-vacuity partner sentence
    goes — the partner survives in the worker project (`archive.test.ts`'s browser-save cell, as `saver@t.test`, and
    battery 17).
  - C1's OWN cells, new: an address claim at `/api/me` reads `{"mode":"open","user":null}`; at `POST /api/games` the
    layer's 401 with `ADDRESS_NOT_A_NAME`; at `POST /api/archive` the Archive object's 401 with the same sentence —
    through the module's `identity_of` and `dispatch`, NOT `callerRefused` (disabling `callerRefused` reddens only
    R-C10); at an upgrade of `/api/stream?claimed=<address>` a 401 unupgraded; and `ben` keeps every one of those
    doors. Under the open door a plain name's save files at the bucket's TOP root (`browser/<segment>/`), which a cell
    reads once (measured true).
  Run `npx vitest run --project open`. Expected: PASS.

- [ ] **Step 7b: One name, one Archive object — `claimedIdentity` collapses space runs (AD-3).** Two spellings of one
  open-door name (`race y`, `race   y`) are two Archive objects today (`claimedIdentity` keeps space runs,
  `identity.ts:144-152`, pinned at `identity.unit.test.ts:456` — the M6.5c2 choice), while the layer's `typed_form`
  collapses them into ONE share: two concurrent saves at 255 records both landed through `handle` — `[201, 201]`, 257
  records in a 256-record share (measured). In `worker/src/identity.ts`:

```ts
export function claimedIdentity(raw: string | null): Identity {
  const lowered = raw === null ? undefined : address(raw);
  // The key grammar's own `typed_form`: each run of spaces is one, so every spelling of a key names ONE Archive object
  // (security S2b, D-5's race) and spends ONE write-limiter key (D-3).
  const name = lowered?.split(" ").filter((part) => part !== "").join(" ");
  return name === undefined || name === "" ? { kind: "none" } : { kind: "claimed", name };
}
```

  Its doc paragraph "Space runs are NOT collapsed here … the only cost is a second object" is deleted; `worker/src/index.ts`'s
  `case "archive"` comment (the two lines reading "under `open` a claim's space runs are left for the key grammar, so two
  spellings of one key name two objects (`claimedIdentity`'s doc, `identity.ts`)") reads "under `open` a claim's space
  runs are collapsed (`claimedIdentity`), so every spelling of one key names ONE object" (Task 5's run: the comment
  went false with the collapse);
  `identity.unit.test.ts:456` expects `name: "ben castle"` (the only cell in the node, pool and open suites that
  moves — measured). `test/open/door.test.ts` gains the race cell: 255 records pre-filled under `race y`'s share, then
  `Promise.all` of `through("/api/archive", { claim: "race y" })` and `{ claim: "race   y" }`, the second with its
  table id rewritten; the statuses, sorted, are `[201, 507]` and the share holds 256 records (its source is the
  packet's `artifacts/lens2_race_handle.probe.ts.txt`; measured `[201, 507]` with the collapse) — AND, the
  deterministic row (Task 5's run): a `vi.spyOn(Archive.prototype, "fetch")` around the `Promise.all`, each call's
  `this.ctx.id.name` read before `mockRestore`, asserted `["archive:race y", "archive:race y"]` — the two saves reached
  ONE object. Without the collapse the `[201, 507]` row is red only about half the time (measured: 9 of 11 runs alone,
  3 of 6 inside the project — two objects race on R2 listing timing and the 507 sometimes arrives anyway), while the
  one-object row reads `["archive:race   y", "archive:race y"]` every run. Run
  `npm run test:scripts && npx vitest run --project open`. Expected: PASS.

- [ ] **Step 8: (moved to Task 5b — the client's name step, the new-table form, the two inverted web cells and the
  e2e identities.)**

- [ ] **Step 9: Every suite the change reaches.** `cargo test -p yokohama-web && cargo test -p yokohama-worker && cd
  worker && npm run build:module:debug && npm run typecheck && npm run test:scripts && npm test`. Expected: PASS; the
  corpus unmoved (`git status` shows no file under `web/src/wire/`). The web `vitest` suite is NOT reached by this
  task (the server-side change breaks no jsdom cell): it is Task 5b's gate.

- [ ] **Step 10: Commit.**

```bash
git add crates/yokohama-web/src/identity.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs crates/yokohama-web/src/archive.rs crates/yokohama-web/tests/wire_fixtures.rs crates/yokohama-web/tests/start_refusal.rs crates/yokohama-worker/src/lib.rs crates/yokohama-worker/src/receipt.rs worker/src/identity.ts worker/src/identity.unit.test.ts worker/src/index.ts worker/test/open/door.test.ts
git commit -m "layer: under the open door a name is a name — ClaimPolicy::NamesOnly in every production constructor refuses an address-shaped key with one sentence, Diagnostic keeps the fixtures; the Worker shell collapses a claim's space runs so one name is one Archive object (C1, D24) (S2b Task 5)"
```

**Mutants (red where named):** M38 `Diagnostic` read as `NamesOnly` → the corpus walk and a hundred cells;
M39 the `with_store_chat_archive_and_log` literal building `Diagnostic` → the constructor cell and the binary cell;
M40 the sentence the absent caller's → the identity cell; M41 `.test` excepted → the identity cell's `red@wire.test`
row; M42a the fallback `admit` passing `Diagnostic` → the `\r` row; M42b `resolve` passing `Diagnostic` → the module
cell and the open project's archive cell; M44 the extractor resolving with a literal policy → the `…/events` cell;
M44b the adapter's call with a literal policy → the route-level cell and the binary cell; M44c `claimedIdentity`
keeping space runs → the race cell's one-object row and `identity.unit.test.ts:456` every run (its `[201, 507]` row
about half the time — timing, measured). (M43 is Task 5b's.)

### Task 5b: C1's client — the name step and the new-table form refuse an `@`; the two web cells C1 inverts; the Playwright identities follow the target — Opus (C1; decision D-2; AD-1, AD-8)

**Why a task of its own (the plan review's Critical, PF-01):** the local Playwright tier runs the RELEASE binary at
the open door (`playwright.config.ts:241, 258`: `cargo run --release -p yokohama-web` with `YOKOHAMA_WEB_IDENTITY: "open"`,
so `main → AppState::new → production → NamesOnly` after Task 5), and `signIn` types `WALKER = "walker@e2e.test"`
into the name step (`harness.ts:342, 373`); twelve of the twenty specs carry address identities, and the
`openDoorVerdict` below refuses every one of them (measured at the review). The same specs run LIVE, where the dev
door admits only a `.test` ADDRESS (Task 6). Two web cells assert the opposite of C1 (`SignIn.test.tsx:37`) or type
`me.json`'s `red@wire.test` (`App.test.tsx:2637`): 2 red of 1,513 (measured). Nothing in Task 5 repairs any of
this, so the client's whole move is here, with `npm run e2e` as its gate.

**Files:**
- Modify: `web/src/nameRule.ts` (`openDoorVerdict`, `ADDRESS_NOT_A_NAME_HERE`), `web/src/nameRule.test.ts`,
  `web/src/SignIn.tsx` (`submit`; the comment at `:10-13`), `web/src/SignIn.test.tsx` (`:37` inverted + one sibling
  cell), `web/src/App.test.tsx` (`:2637`), `web/src/NewGame.tsx` (`personError` `:208`, the body `:236`),
  `web/src/NewGame.test.tsx` (`:129`)
- Modify: `web/e2e/harness.ts` (`who`, `WALKER`), and every spec that signs in by address — `exit_game`,
  `it_explains`, `joins_by_link`, `offline`, `plays_a_whole_game`, `plays_locally`, `readme_shots`, `setup_screens`,
  `state1_game`, `the_screen_itself`, `the_table`, `two_browsers` (the review's list — seven carry an address LITERAL,
  `grep -ln "@[a-z0-9.-]*\.test" web/e2e/*.spec.ts`; the rest sign in through `signIn()`/`newGameForm()`'s default
  `WALKER` and move with it; thirteen sites seed `CLAIMED_USER_KEY` or send `x-yokohama-claimed-user` — the
  implementer re-derives all three lists before editing)

**Interfaces:**
- Consumes: `normalizeName`, `NameVerdict`, `cases`/`shown` (`nameRule.test.ts`'s fixtures over `identity_cases.json`);
  `LIVE` (`web/e2e/live.ts`, already imported by the harness); Task 5's `ADDRESS_NOT_A_NAME` (the server's sentence
  the client's mirrors).
- Produces: `openDoorVerdict`, `ADDRESS_NOT_A_NAME_HERE`; `harness.ts`'s `who(name)`; a local tier that plays by names
  and a live tier that plays by `.test` addresses.

- [ ] **Step 1: The name step's failing cells.** In `web/src/nameRule.test.ts`, the import becomes
  `import { ADDRESS_NOT_A_NAME_HERE, NAME_MAX, WHITE, normalizeName, openDoorVerdict } from "./nameRule";`, and:

```ts
describe("openDoorVerdict (security S2b, C1; decision D-2)", () => {
  // Mutants: the `@` test dropped (every address row admitted); the test on the KEY rather than the raw input (a
  // raw `@` that fails the grammar would get the grammar's sentence about writing an address, at a door that refuses
  // every address — AD-8 (d)).
  it("refuses every committed row whose raw input holds an @, in the door's own words, and agrees with normalizeName on every other row", () => {
    const withAt = cases.filter((c) => c.raw.includes("@"));
    expect(withAt.length).toBeGreaterThanOrEqual(4);
    for (const c of withAt) expect(openDoorVerdict(c.raw), shown(c.raw)).toEqual({ ok: false, reason: ADDRESS_NOT_A_NAME_HERE });
    for (const c of cases.filter((c) => !c.raw.includes("@"))) expect(openDoorVerdict(c.raw), shown(c.raw)).toEqual(normalizeName(c.raw));
  });
});
```

  `cd web && npx vitest run src/nameRule.test.ts`. Expected: FAIL — `openDoorVerdict` is not exported.

- [ ] **Step 2: The verdict.** In `web/src/nameRule.ts`:

```ts
/** The open door's own refusal of an address (security S2b, finding C1; decision D-2), in this client's words:
 *  the server's `ADDRESS_NOT_A_NAME` says the same thing in its own. */
export const ADDRESS_NOT_A_NAME_HERE = "Under the open door a name is a name — an address is not accepted here. Choose a plain name.";

/** The name step's verdict (it exists under the open door alone): any `@` in the RAW input is refused FIRST, in the
 *  door's own words — before the key grammar, whose sentence for a malformed address would tell an open-door
 *  player how to write one — and then the key grammar's verdict (the server's `ClaimPolicy::NamesOnly` refuses the
 *  same key with its own sentence). `normalizeName` itself stays the KEY grammar, pinned to `identity_cases.json`,
 *  which still lists addresses as valid KEYS: they are, under Access. */
export function openDoorVerdict(raw: string): NameVerdict {
  if (raw.includes("@")) return { ok: false, reason: ADDRESS_NOT_A_NAME_HERE };
  return normalizeName(raw);
}
```

  `SignIn.tsx`'s `submit` calls `openDoorVerdict(value)`, and its comment at `:10-13` names `openDoorVerdict`.
  `NewGame.tsx`: when `byName`, `personError` (`:208`) and the request body (`:236`) go through `openDoorVerdict`
  (AD-8 (a): the "their name" seat field keyed any address `normalizeName` accepts, and after C1 a seat bound to an
  address at an open table could never be taken); `NewGame.test.tsx` GAINS an open-door cell (Task 5b's run: its `:129` cell is the Access one — `cf-access`, an
  email field, `Blue@Wire.test` — where `byName` is false and an address is right, so it is UNCHANGED): under the
  open door, typing `Blue@Wire.test` into "Blue player's name" shows `ADDRESS_NOT_A_NAME_HERE` in place, marks the
  field invalid, holds Deal and posts nothing — M43d's catcher. Run the two files' cells. Expected: PASS.

- [ ] **Step 3: The two cells C1 inverts.** `SignIn.test.tsx:37` becomes "refuses an address before any request, in
  the open door's words, and stores nothing": type `" Red@Wire.TEST "`; the alert reads `ADDRESS_NOT_A_NAME_HERE`;
  `CLAIMED_USER_KEY` and `DISPLAY_NAME_KEY` stay null; `onSignedIn` is never called — with a sibling cell that takes
  `ben`. `App.test.tsx:2637` types `"red"`, and its stub answers `/api/me` with `{ ...wire.me, user: "red" }` once the
  key is set, so the committed `me.json` stays UNMOVED (the corpus fence). Run
  `npx vitest run src/nameRule.test.ts src/SignIn.test.tsx src/App.test.tsx src/NewGame.test.tsx`, then the whole
  `npx vitest run`. Expected: PASS (1,511 plus this task's cells; record the count).

- [ ] **Step 4: The Playwright identities follow the target.** In `web/e2e/harness.ts`:
  `export const who = (name: string, domain = "e2e"): string => (LIVE ? \`${name}@${domain}.test\` : name);` and
  `export const WALKER = who("walker");` with the doc "under the open door the local tier's binary is `production`,
  `NamesOnly` (security S2b, C1): a caller is a plain NAME; the live tier's dev door names a `.test` ADDRESS (B6), so
  an identity follows the target". Every address literal in the twelve specs goes through `who` — so does every
  `CLAIMED_USER_KEY` a spec seeds and every `x-yokohama-claimed-user` a `request.*` sends; a spec whose live
  identities carry their own domain keeps it through the second argument (`exit_game`'s `who("x", "exit")` for
  `x@exit.test`, `readme_shots`'s `who("ben", "readme")` for `ben@readme.test` — the README's alt text names
  `ben@readme.test`, and a live re-shot must still draw it; Task 5b's run). `state1_game.spec.ts`, the
  open door's own window instrument, takes plain names on every target and loses its header sentence "the name step
  takes an address as well as a plain name". The archive-segment restatements (`offline.spec.ts:126-129`,
  `plays_locally.spec.ts:68`) follow the key (`ben` → `ben-<12 hex>`; the readable half has no `_at_`).

- [ ] **Step 5: The gate.** `cd web && npm run typecheck && npm run e2e` (the local tier: the release binary, Chromium;
  81 cells today — record the count). `npm run typecheck`'s fourth step needs `npm run build:wasm` first, or
  `YOKOHAMA_WASM_OPTIONAL=1` checks the first three, which cover this slice. Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add web/src/nameRule.ts web/src/nameRule.test.ts web/src/SignIn.tsx web/src/SignIn.test.tsx web/src/App.test.tsx web/src/NewGame.tsx web/src/NewGame.test.tsx web/e2e/harness.ts web/e2e/*.spec.ts
git commit -m "web: under the open door the name step and the new-table form refuse an @ before any request, in the door's own words; the Playwright identities follow the target — names locally, .test addresses live (C1's client) (S2b Task 5b)"
```

  (`git add web/e2e/*.spec.ts` stages only the specs the task changed; `git status` first — explicit paths, never
  `-A`.)

**Mutants (red where named):** M43 `openDoorVerdict` without the `@` test → the client cell; M43b `submit` back on
`normalizeName` → the inverted SignIn cell; M43c the `@` test on the key rather than the raw input → the rows whose
raw input holds an `@` and fails the grammar (AD-8 (d)); M43d `NewGame.tsx`'s seat field back on `normalizeName` →
`NewGame.test.tsx`; M43e `who` ignoring `LIVE` → the local tier signs in as an address and reads
`ADDRESS_NOT_A_NAME_HERE` (the e2e gate itself).

### Task 6: B6 — the dev door admits a `.test` address only; the production file ships it OFF; `wrangler.dev.jsonc`; the guard's door rules and F-40 — Opus (B6; decisions D-1, D-10; rulings R-2, X6)

**Files:**
- Modify: `worker/src/identity.ts` (`isTestAddress`, the dev arm), `worker/src/identity.unit.test.ts`
- Modify: `worker/wrangler.jsonc` (`YOKOHAMA_DEV_DOOR: "off"`, its comment); Create: `worker/wrangler.dev.jsonc`;
  Delete: `worker/wrangler.gate.jsonc`
- Modify: `worker/package.json` (`deploy:dev` replaces `deploy:gate`), `worker/scripts/secrets-guard.mjs` (its header's
  "safe direction" sentence)
- Modify: `worker/scripts/deploy-guard.mjs` (`DEV_FILE`, `DEV_DIFFERS`, the door rules, F-40's sibling rule, the header
  and the success sentence — the guard has NO `--help`), `worker/scripts/deploy-guard.test.mjs` (the gate fixture and
  cells become the dev file's, three existing cells edited by hand; two new cells)

**Interfaces:**
- Consumes: `isAddress`, `address` (`identity.ts`); the guard's `siblingDrift`, `doorRules`, `configFailures`,
  `canonical`, `objectOf`.
- Produces: `isTestAddress(email)`; the scripts `npm run deploy` (production, dev door OFF) and `npm run deploy:dev`
  (the dev door ON, around a live tier); `DEV_FILE`, `DEV_DIFFERS`; the guard's sentences the documents quote.

- [ ] **Step 1: The dev arm's failing cells (node).** In `worker/src/identity.unit.test.ts`, beside the R1 L-2 cells:

```ts
  // **The dev door names a `.test` address only** (security S2b, finding B6; R-SEC8(d)): the receipt archive's own
  // rule for the `test/` root (`is_test_identity`: an `@` and the suffix `.test`), so the credential can reach no
  // real seat and no training root. Mutants: the suffix test dropped (a real address named); `endsWith("test")`
  // (`x@contest` named); the test on the raw value (` red@wire.test ` refused, which `address` trims first).
  it("names a .test address through the dev door and nobody for any other address", async () => {
    const token = await key.sign(credentialClaims(CLIENT, at));
    // `a@.test` is ADMITTED: the rule is the suffix, not the domain's shape (`isAddress` and `User::email` both take
    // any non-empty domain; probed at the plan review).
    for (const claimed of ["red@wire.test", " Red@Wire.TEST ", "a@e2e.test", "probe@deploy.test", "a@.test"]) {
      expect(await resolve(token, { "X-Yokohama-Claimed-User": claimed }), claimed).toEqual({ kind: "dev", email: claimed.trim().toLowerCase() });
    }
    for (const claimed of ["owner@example.com", "red@wire.contest", "red@test", "red@wire.test.evil", "ben"]) {
      expect(await resolve(token, { "X-Yokohama-Claimed-User": claimed }), claimed).toEqual({ kind: "none" });
    }
    expect(isTestAddress("red@wire.test")).toBe(true);
    expect(isTestAddress("a.test")).toBe(false);
  });
```

  The cell file's import list from `./identity` gains `isTestAddress,`.

- [ ] **Step 2: Run to see the fail.** Expected: FAIL — `owner@example.com` is named (and `isTestAddress` is not
  exported).

- [ ] **Step 3: The arm.** In `worker/src/identity.ts`:

```ts
/** The reserved domain every test identity's address ends in — `crates/yokohama-worker/src/receipt.rs`'s
 *  `TEST_DOMAIN`, restated. */
const TEST_DOMAIN = ".test";

/** Whether a normalized address is a TEST identity's (security S2b, finding B6; R-SEC8(d)): an `@` and the suffix
 *  `.test` — the receipt archive's own rule for the `test/` root, so what the dev door can name is exactly what
 *  the bucket keeps apart from the owner's training data. A plain name ending `.test` (`a.test`) is not one, as the
 *  module reads it. */
export function isTestAddress(email: string): boolean {
  return email.includes("@") && email.endsWith(TEST_DOMAIN);
}
```

  and the dev arm reads `if (claimed !== undefined && isAddress(claimed) && isTestAddress(claimed)) return { kind: "dev", email: claimed };`,
  the doc comment above `resolveIdentity` gaining: "and, at the dev door, a claimed address outside `.test` (B6): the
  credential names test identities and nobody else".

- [ ] **Step 4: Run the identity cells.** Expected: PASS. Then the pool's `worker.test.ts` and `operator.test.ts`
  dev-door cells, which claim `.test` addresses throughout (PROBE: `grep -n "Claimed-User" test/*.ts | grep -v "\.test"`
  prints nothing).

- [ ] **Step 5: The configurations.** `wrangler.jsonc`'s dev-door entry becomes:

```jsonc
    // OFF (security S2b, finding B6; R-SEC8(d)): the standing state is Access alone, the shape a self-hoster behind
    // their own application runs. The dev door — the `yokohama-worker` credential naming a `.test` address — is the
    // live tiers' instrument and opens for them alone: `npm run deploy:dev` ships `wrangler.dev.jsonc`, this file
    // with this one variable `on`, before a live tier, and the plain `npm run deploy` reverts it after. The deploy
    // guard refuses `on` in this file and requires it in that one.
    "YOKOHAMA_DEV_DOOR": "off",
```

  `wrangler.dev.jsonc` is created as a COPY of `wrangler.jsonc` whose header comment reads:

```jsonc
// **The Worker `yokohama-web` with the dev door OPEN** (security S2b, finding B6; R-SEC8(d)) — production's
// configuration in every respect but one: `YOKOHAMA_DEV_DOOR` is "on", so the `yokohama-worker` credential plus a
// claimed `.test` address names a caller, which is how the live corpus replay and the live Playwright tier play as
// several people. `npm run deploy:dev` ships it before a live tier, and the plain `npm run deploy` reverts to
// production's (`worker/RUNBOOK.md` §7). The deploy guard judges it like any configuration (`--config
// wrangler.dev.jsonc`), holds it to production in every key but this variable, and refuses it if the dev door is
// not "on". A `--var` deploy was the alternative and is not taken: a `--var` prints the binding as "(hidden)", so
// a deploy could not be read back; a committed file can.
```

  and whose `vars` entry reads `"YOKOHAMA_DEV_DOOR": "on",` with the comment `// Open: the point of this file.`.
  `wrangler.gate.jsonc` is deleted (`git rm`). `package.json`: the `deploy:gate` script becomes
  `"deploy:dev": "node scripts/deploy-guard.mjs --config wrangler.dev.jsonc && node scripts/secrets-guard.mjs --config wrangler.dev.jsonc && wrangler deploy --config wrangler.dev.jsonc"`.
  `secrets-guard.mjs`'s header: "every deploy script — `deploy:gate`, the safe direction, included — is a network
  step" becomes "every deploy script — the plain `deploy`, the safe direction, included — is a network step"; and its
  lines 11–13, "…a NEW Worker's first deploy cannot pass it (wrangler answers "Worker not found") — S4's public Worker
  needs its own first-deploy path.", become "…a NEW Worker's first deploy cannot pass it (wrangler answers "Worker not
  found"): that deploy runs the deploy guard and `wrangler deploy --config <file>` by hand, and this guard from the
  second deploy on (D-7; `worker/RUNBOOK.md` §2)." (`RUNBOOK.md:195-199` says the same, in Task 9.)

- [ ] **Step 6: The guard's failing cells.** In `deploy-guard.test.mjs`: the fixture `GATE` becomes
  `DEV = PRODUCTION.replace('"ACCESS_AUD": "test-aud"', '"ACCESS_AUD": "test-aud",\n    "YOKOHAMA_DEV_DOOR": "on"')`,
  the tree's `"wrangler.gate.jsonc": GATE` becomes `"wrangler.dev.jsonc": DEV`, every cell naming the gate file
  names the dev file with its verdicts inverted (`"off"` refused there, `"on"` required — the `YOKOHAMA_DEV_DOOR`
  cells alone invert), the three-command loop reads `["deploy:dev", "wrangler.dev.jsonc"]`, and three existing cells
  need a hand the rule above does not reach (measured: the guard file then passes 75/75):
  1. the last two lines of "refuses the dev door on beside the open door…" (`:646-647`, `worked`, expecting
     `toEqual([])` at `wrangler.jsonc`) contradict B6 yet name no gate file — they become
     `expect(deployGuard(worker({ "wrangler.jsonc": worked })), "on at the worker door, in production's file").toEqual(naming(/^wrangler\.jsonc's vars set YOKOHAMA_DEV_DOOR=on — the dev door is deployed by wrangler\.dev\.jsonc alone/));`;
  2. `gateOn` (`:1033`) would `.replace` a string the renamed fixture no longer holds, silently matching nothing — it
     becomes `devOn = DEV.replace('"YOKOHAMA_DEV_DOOR": "on"', '"YOKOHAMA_DEV_DOOR": "on", "YOKOHAMA_TURNSTILE": "on"')`;
  3. the `YOKOHAMA_TURNSTILE` row `["wrangler.gate.jsonc", "off"]` (`:1217`) is RENAMED only; its declaration stays
     `off` (it is not a dev-door cell, so it does not invert).
  Then the two new cells below:

```js
  // Security S2b, B6 (decision D-1): the production file ships the dev door OFF; the dev file alone opens it.
  it("refuses YOKOHAMA_DEV_DOOR=on in the production file and in any worker-door file but wrangler.dev.jsonc, and requires exactly on there", () => {
    const on = PRODUCTION.replace('"ACCESS_AUD": "test-aud"', '"ACCESS_AUD": "test-aud",\n    "YOKOHAMA_DEV_DOOR": "On"');
    expect(deployGuard(worker({ "wrangler.jsonc": on }))).toEqual(
      naming(/^wrangler\.jsonc's vars set YOKOHAMA_DEV_DOOR=on — the dev door is deployed by wrangler\.dev\.jsonc alone \(npm run deploy:dev\), around a live tier, and reverted after \(R-SEC8\(d\)\)/),
    );
    for (const off of ['"off"', '"ON "', "nothing"]) {
      const file = off === "nothing" ? PRODUCTION : DEV.replace('"YOKOHAMA_DEV_DOOR": "on"', `"YOKOHAMA_DEV_DOOR": ${off}`);
      expect(deployGuard(worker({ "wrangler.dev.jsonc": file }), undefined, "wrangler.dev.jsonc"), off).toEqual(
        naming(/^wrangler\.dev\.jsonc's vars must set YOKOHAMA_DEV_DOOR to "on" — its only purpose is the worker door with the dev door open; got /),
      );
    }
    expect(deployGuard(worker(), undefined, "wrangler.dev.jsonc")).toEqual([]);
    expect(doorLine(worker(), "wrangler.dev.jsonc")).toBe("wrangler.dev.jsonc deploys the worker door — Cloudflare Access, verified by the shell");
  });

  // F-40 (the S2a plan review): an unlisted worker-door file is production's sibling with NO allowed difference.
  it("holds an unlisted worker-door --config file to production exactly, and passes one that equals it", () => {
    const drifted = PRODUCTION.replace('"ACCESS_AUD": "test-aud"', '"ACCESS_AUD": "old-aud"');
    expect(deployGuard(worker({ "wrangler.staging.jsonc": drifted }), undefined, "wrangler.staging.jsonc")).toEqual(
      naming(/^wrangler\.staging\.jsonc's vars\.ACCESS_AUD differs from wrangler\.jsonc's/),
    );
    expect(deployGuard(worker({ "wrangler.staging.jsonc": PRODUCTION }), undefined, "wrangler.staging.jsonc")).toEqual([]);
  });
```

  (`naming` is the file's own matcher over the failures array; `worker(files)` lays the tree.)

- [ ] **Step 7: The guard.** In `deploy-guard.mjs`: `GATE_FILE` → `const DEV_FILE = "wrangler.dev.jsonc";` with the doc
  "the live tiers' configuration (security S2b, B6; decision D-1): production with the dev door `on`, held to
  production's blocks like the open file"; `GATE_DIFFERS` → `const DEV_DIFFERS = { YOKOHAMA_DEV_DOOR: "on" };`. In
  `doorRules`, after the open door's dev-door refusal:

```js
  // The dev door is the live tiers' instrument and opens in ONE file (security S2b, B6; R-SEC8(d); decision D-1).
  if (door === "worker" && name !== DEV_FILE && typeof devDoor === "string" && devDoor.trim().replace(/[A-Z]/g, (c) => c.toLowerCase()) === "on") {
    failures.push(
      `${name}'s vars set YOKOHAMA_DEV_DOOR=on — the dev door is deployed by wrangler.dev.jsonc alone (npm run deploy:dev), around a live tier, and reverted after (R-SEC8(d))`,
    );
  }
```

  In `configFailures`, the gate block becomes the dev file's: `if (config === DEV_FILE) { const dev = objectOf(judged.value.vars)?.YOKOHAMA_DEV_DOOR; if (dev !== "on") failures.push(`${config}'s vars must set YOKOHAMA_DEV_DOOR to "on" — its only purpose is the worker door with the dev door open; got ${JSON.stringify(dev) ?? "nothing"}`); }`;
  `held` reads `[OPEN_FILE, DEV_FILE, ...POOLS]`; the sibling calls: `config === DEV_FILE → siblingDrift(config, production.value, judged.value, DEV_DIFFERS, failures)`; and F-40, beside the open-door sibling rule:

```js
  // **Every unlisted worker-door file is production's sibling with NO allowed difference** (the S2a plan review's
  // F-40; security S2b): a configuration judged by `--config` whose door is `worker` and which is neither
  // production nor the dev file is held to production exactly — a deploy of it would otherwise roll production
  // back or open a door no committed file opens.
  if (production && judged && config !== PRODUCTION_FILE && config !== DEV_FILE && config !== OPEN_FILE && doorOf(judged.value) === "worker") {
    siblingDrift(config, production.value, judged.value, {}, failures);
  }
```

  The header comment's check 3 gains the dev-door sentence and 3b the dev file; the success sentence (the closing
  `console.log`; there is no `--help`) reads "the open and dev files" where it said "the open and gate files", as do
  the header's check 5 and line 44, and gains the clause "at the open door no dev door, the dev door in
  wrangler.dev.jsonc alone, every unlisted file production's sibling".

- [ ] **Step 8: Run the guard suite and the guard itself.** `npm run test:scripts`; then
  `node scripts/deploy-guard.mjs --config wrangler.jsonc` and `--config wrangler.dev.jsonc` over the tree print their
  door lines (`wrangler.jsonc deploys the worker door …`, `wrangler.dev.jsonc deploys the worker door …`); a tree
  prepared for the pool also prints checks 1 and 9 (and 8 after a crate edit, until `build:module` re-stamps
  `gen/source.json`), none this task's.

- [ ] **Step 9: Commit.**

```bash
git add worker/src/identity.ts worker/src/identity.unit.test.ts worker/wrangler.jsonc worker/wrangler.dev.jsonc worker/package.json worker/scripts/secrets-guard.mjs worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs
git rm worker/wrangler.gate.jsonc
git commit -m "worker: the dev door names a .test address only and ships OFF in the production file; wrangler.dev.jsonc opens it around a live tier (npm run deploy:dev), the guard holding it as production's one-difference sibling and every unlisted worker-door file to production exactly (B6, F-40) (S2b Task 6)"
```

**Mutants (red where named):** M45 the suffix test dropped → `owner@example.com` named; M46 `endsWith("test")` →
`red@wire.contest`; M47 the guard's production rule dropped → the `On` row passes; M48 the dev file's `"on"`
requirement dropped → the `"off"` row passes; M49 F-40's rule dropped → the drifted staging file passes; M50
`DEV_DIFFERS` empty → the dev file itself refused for its one difference.

### Task 7: D23 — the shell's lines carry digests, the keys lose their readable half, the configurations redact query strings; the guard's check 14 — Opus (D23; decisions D-6, D-10)

**Files:**
- Modify: `worker/src/house.ts` (the `operator` line), `worker/src/table/lifecycle.ts` (`operate`'s line; `held`'s and the
  dropped-save line's `record_key`), `worker/src/table/archiving.ts` (two `record_key` sites), `worker/src/table.ts`
  (two `record_key` sites), `worker/src/r2.ts` (`logOutcome`: `record_key` AND `detail`), `worker/src/archive.ts` (the
  object's log `id` becomes a digest)
- Modify: `worker/wrangler.jsonc`, `worker/wrangler.dev.jsonc`, `worker/wrangler.open.jsonc` (`observability`)
- Modify: `worker/scripts/deploy-guard.mjs` (+ `.test.mjs`): check 14
- Test: `worker/src/r2.unit.test.ts` (two rows), `worker/test/operator.test.ts` (six expectations), `worker/test/archive.test.ts`
  (the browser-save cell's lines)

**Interfaces:**
- Consumes: `callerDigest`, `redactKey` (`redact.ts`, Task 2); `callerOf`, `ARCHIVE_PREFIX` (`archive.ts`/`index.ts`,
  the object-name helpers); the configurations as Task 6 left them.
- Produces: the log shape `operator: <12 hex>` (never `address`); every record key redacted, in `record_key` and in
  `detail`; the Archive object's log `id` `archive:<12 hex>`; the configuration key
  `observability.redact_query_string: true`; the guard's check 14.

**What the plan first missed (PF-03, PF-04 — D23 would have been marked fixed over a live leak):** `r2.ts:68` logs
`detail: outcome.detail`, and for a landed, identical or conflicting outcome that IS `plan.record_key` (`:131, 135`),
while the one failed case's sentence begins with it (`:148`); and `archive.ts:81` sets `const id = this.ctx.id.name`,
which is `archive:<address>` (`index.ts:368`), logged by `init-refused` (`:84, :145`), `logOutcome` (`:101`) and `trap`
(`:149`) — pinned as such at `archive.test.ts:1193` and documented at `RUNBOOK.md:608`. Both are this task's.

- [ ] **Step 1: The failing cells.** In `worker/src/r2.unit.test.ts`, beside `logOutcome`'s existing cell: build the
  outcome as `executePlan` does — `{ outcome: "landed", detail: plan.record_key }` over the plan
  `{ record_key: "test/browser/red_at_wire.test-c91ad2a917f7/rev7/over/wire.json", … }` — and assert the parsed line's
  `record_key` AND `detail` both equal `test/browser/c91ad2a917f7/rev7/over/wire.json`, and that the whole serialized
  line contains neither `red_at_wire` nor `browser/red`; a second row builds a failed outcome as `executePlan` builds
  its first failure (`` `${plan.record_key} refused a create as present, then read as absent` ``) and asserts its
  `detail` reads redacted. In `worker/test/operator.test.ts`, every `address: BOSS` expectation — six, at lines 286,
  343, 460, 522, 567 and 697 — becomes `operator: await callerDigest(BOSS)` with NO field named `address`, and no
  serialized line contains `boss@t.test`. In `worker/test/archive.test.ts`'s browser-save cell (`:1193`):
  `const digest = await callerDigest(caller);`, `id: \`archive:${digest}\``, `record_key: redactKey(recordKey)`,
  `detail: redactKey(recordKey)`, and
  `expect(JSON.stringify(outcomes.lines), "no persisted line spells the caller's address").not.toContain(caller);`
  (measured: pool 227/227 with Step 3 whole; the no-address assertion fails on three lines without the `archive.ts`
  half).

- [ ] **Step 2: Run to see the fails.** Expected: FAIL — the lines carry the address and the readable segment.

- [ ] **Step 3: The lines.** `house.ts`: `log("operator", { action: "list", operator: await callerDigest(operator) });`
  (`fetch` is async). `lifecycle.ts`'s `operate`: `log("operator", { id, action: force ? "force" : "expire", operator: await callerDigest(address) });`.
  Every `log(` whose fields include `record_key:` — `r2.ts`'s `logOutcome`, `archiving.ts`'s `archive-owed-kept`
  and `archive-stamped`, `lifecycle.ts`'s `held` and `archive-owed-dropped`, `table.ts`'s two `archive-owed-dropped`
  sites (`grep -n "record_key:" worker/src/*.ts worker/src/table/*.ts` lists every one; seven) — wraps the key in
  `redactKey(…)`. `r2.ts`'s `logOutcome` logs `record_key: redactKey(plan.record_key)` **and**
  `detail: redactKey(outcome.detail)`: a landed, identical or conflicting outcome's detail IS the record's key
  (`executePlan`'s `at()` and `against()`), and the one failed case's begins with it; `SEGMENT` reads both because
  `browser/` follows a `/`; R2's own failure message (`e.message`) is logged as it is. A doc sentence at `logOutcome`:
  "The key is logged REDACTED in both fields (security S2b, D23): the segment's readable half dropped, its digest kept,
  so a line names the folder and not the address it spells." `worker/src/archive.ts`: `fetch` keeps the object's whole
  name for the module and logs a digest —
  `const name = this.ctx.id.name; const caller = callerOf(name); const id = caller === null ? name : ARCHIVE_PREFIX + (await callerDigest(caller));`
  — passing `name` to `moduleConfig(this.env, callerOf(name))` (`save(name, id, incoming, config)`) and `id` to every
  `log(...)` and `logOutcome(...)` this object writes, with the comment "Every line this object logs names it by a
  digest: its name is `archive:<address>`, and a log persists."

- [ ] **Step 4: Run the three suites.** `npx vitest run -c vitest.scripts.config.ts src/r2.unit.test.ts && npm test`.
  Expected: PASS.

- [ ] **Step 5: The configurations and check 14.** Production's `"observability": { "enabled": true }` becomes

```jsonc
  // Logs persist in Workers Observability (`worker/RUNBOOK.md` §5, §7). `redact_query_string` (security S2b,
  // finding D23; decision D-6): an open-door socket's `claimed=` rides the upgrade's URL, and an invocation log
  // keeps the URL; redacted, no name persists there. The sampling rate stays its default, 1: the live readings
  // count invocations through these logs, and sampling would be privacy by volume at the instrument's cost. The
  // shell's own lines carry a caller's digest and a redacted key, never an address (`src/redact.ts`).
  "observability": { "enabled": true, "redact_query_string": true },
```

  byte for byte in `wrangler.dev.jsonc` and `wrangler.open.jsonc` (the sibling rule); the comment's last sentence is
  true only once Step 3's `archive.ts` half has landed. The guard's check 14, in `configFailures` after
  `rateLimits(…)` (the sampling clause is AD-10 (a), proposed — nothing held the rate before; `head_sampling_rate: 0.1`
  passed the guard):

```js
  // Check 14 (security S2b, D23; decision D-6): production's logs redact query strings and are not sampled.
  const observed = production ? objectOf(production.value.observability) : undefined;
  if (production && observed?.redact_query_string !== true) {
    failures.push("wrangler.jsonc's observability must set redact_query_string to true — an open-door socket's claimed= would persist in the invocation logs otherwise (security S2b, D23)");
  }
  if (production && observed?.head_sampling_rate !== undefined && observed.head_sampling_rate !== 1) {
    failures.push("wrangler.jsonc's observability must leave head_sampling_rate at 1 — the live readings count invocations through these logs (security S2b, D-6)");
  }
```

  with its header paragraph, the success sentence's clause, and "all thirteen" → "all fourteen" at
  `deploy-guard.mjs:4` and `:692`; `deploy-guard.test.mjs`: `PRODUCTION` gains the key (and so do the dev and open
  fixtures); three rows: the key absent, `false`, and `head_sampling_rate: 0.1`, each refused in its sentence.

- [ ] **Step 6: Run the script suite and the guard.** `npm run test:scripts && node scripts/deploy-guard.mjs --config wrangler.jsonc`.
  Expected: PASS for the script suite; the guard prints the door line, and a tree prepared for the pool also prints
  checks 1 and 9 (and 8), none this task's.

- [ ] **Step 7: Commit.**

```bash
git add worker/src/house.ts worker/src/table/lifecycle.ts worker/src/table/archiving.ts worker/src/table.ts worker/src/r2.ts worker/src/archive.ts worker/src/r2.unit.test.ts worker/test/operator.test.ts worker/test/archive.test.ts worker/wrangler.jsonc worker/wrangler.dev.jsonc worker/wrangler.open.jsonc worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs
git commit -m "worker: the operator lines carry a digest, every record key its folder's digest alone in both fields, the archive object a digest for its name, and every deployable configuration redacts query strings from its persisted logs and keeps them unsampled, the guard's check 14 (D23) (S2b Task 7)"
```

**Mutants (red where named):** M51 `address` kept on the listing's line → the operator cell; M52 `redactKey`
dropped at `logOutcome`'s `record_key` → the r2 cell; M52b `detail` unredacted → the r2 cell; M52c the Archive
object logging its name → the archive cell's no-address assertion; M53 `redact_query_string: false` in production →
the guard cell; M54 check 14 dropped → the absent-key row; M54b the sampling clause dropped → the `0.1` row.

### Task 8: The owed cells and sentences — the fully escaped key, the crafted name, the 48-cut in WASM, S2a's parked sentences, D2's label, the outbox's 429 — Sonnet (decision D-13; §9.6 correction 9)

**Files:**
- Modify: `worker/scripts/deploy-guard.test.mjs` (rule 4b's fully escaped rows)
- Modify: `crates/yokohama-web/src/archive.rs` (one cell: the crafted name)
- Modify: `worker/test/archive.test.ts` (one cell: the 48-cut in WASM against the native literal)
- Modify: `web/scripts/headers.mjs` (two sentences), `crates/yokohama-web/src/lib.rs` (the `headers` field's doc and
  the `with_headers` cell's message)
- Modify: `web/e2e/harness.node.test.ts` (D2's label cell)
- Modify: `web/src/outbox.ts` (`reasonFor`'s 429 arm), `web/src/outbox.test.ts` (one cell)

**Interfaces:**
- Consumes: `caller_segment`, `User::parse`; `segmentOf` (the pool helper); the harness's `fakePage`; the outbox's
  `reasonFor`; the S2a SDD ledger's recorded true forms (`docs/temp/security-review-2026-10-08/s2a-sdd/progress.md`,
  the "PARKED" paragraph) — quoted below so the implementer need not open it.
- Produces: tests and sentences only; one two-line code change in `outbox.ts`.

- [ ] **Step 1: The fully escaped key, as committed rows.** In `deploy-guard.test.mjs`, beside rule 4b's escaped-key
  rows (the S1 stack review's HI-F3 escaped the last letter alone; its re-review's probe escaped every letter and the
  `env`/`vars` KEYS themselves, ran as a temporary copy and was deleted — committed here):

```js
  /** `name` with EVERY character as a `\uXXXX` escape, as a JSON key may be spelled. */
  const escaped = (name) => [...name].map((c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`).join("");

  it("refuses the diagnostic seeds and the test clock spelled with every letter escaped, at the top level, in an environment, and with the env and vars keys themselves escaped", () => {
    for (const [key, sentence] of [["YOKOHAMA_DIAGNOSTIC_SEEDS", /names YOKOHAMA_DIAGNOSTIC_SEEDS/], ["YOKOHAMA_WEB_TEST_CLOCK", /names YOKOHAMA_WEB_TEST_CLOCK/]]) {
      const top = PRODUCTION.replace('"YOKOHAMA_WEB_IDENTITY": "worker"', `"YOKOHAMA_WEB_IDENTITY": "worker", "${escaped(key)}": "on"`);
      const env = PRODUCTION.replace('"main"', `"${escaped("env")}": { "production": { "${escaped("vars")}": { "${escaped(key)}": "on" } } },\n  "main"`);
      for (const file of [top, env]) expect(deployGuard(worker({ "wrangler.jsonc": file })), key).toEqual(naming(sentence));
    }
  });
```

  Run `npm run test:scripts`. Expected: PASS at once (the guard parses keys; the rows PIN it — a mutant that read the
  file as text with a literal search goes red here).

- [ ] **Step 2: The crafted name.** In `crates/yokohama-web/src/archive.rs`'s tests:

```rust
    /// **A crafted bare name equal to another key's segment names its own folder** (the S1 stack review's
    /// archive lens; spec §9.6 correction 9): since S1c every segment ends in twelve hex of its OWN key's digest, so
    /// a name spelled like another key's whole segment gets a segment of its own — the readable half, `-`, its
    /// own digest — and no legacy folder exists for a name to equal (the window renamed every one). A segment too
    /// long to be a name (a cut address's, over 32) is no key at all.
    #[test]
    fn a_name_spelled_like_another_keys_segment_files_under_its_own_segment() {
        use crate::identity::User; // the test module does not import it (E0433 ×4–6 without this; measured)
        // The digest is over the whole KEY, not its readable half: a literal three independent computations agree
        // on (native, Python, the WASM module). For a crafted NAME the key IS the readable half, so a mutant
        // digesting the readable half survives the relative assertions below; this literal row is where it dies.
        assert_eq!(
            caller_segment(&User::parse("alice@corp.test").unwrap()),
            "alice_at_corp.test-dde4a5b96f79",
            "the digest is over the whole KEY (native, Python and the WASM module agree)"
        );
        for key in ["ben castle", "alice@corp.test"] {
            let target = caller_segment(&User::parse(key).unwrap());
            let crafted = User::parse(&target).unwrap_or_else(|e| panic!("{target} is a legal name: {e}"));
            let own = caller_segment(&crafted);
            assert_ne!(own, target, "{key}: the crafted name's folder is not the target's");
            assert!(own.starts_with(&format!("{target}-")), "{key}: the target is the crafted name's readable half");
        }
        let long = caller_segment(&User::parse("a-very-long-local-part-for-the-forty-eight-cut@wire.test").unwrap());
        assert!(User::parse(&long).is_err(), "a cut address's segment is longer than a name may be: no key at all");
    }
```

  Run `cargo test -p yokohama-web a_name_spelled_like`. Expected: PASS.

- [ ] **Step 3: The 48-cut in WASM against native.** In `worker/test/archive.test.ts`, a cell whose caller is
  `a-very-long-local-part-for-the-forty-eight-cut@wire.test` (56 characters; its readable half, `…cut_at_wire.test`,
  is cut at 48 to `a-very-long-local-part-for-the-forty-eight-cut_a`): a browser save through its archive object
  lands, and the key it lands under — read by listing the bucket under `test/browser/` — has the segment
  `a-very-long-local-part-for-the-forty-eight-cut_a-2e9a7b138059`, the NATIVE `caller_segment`'s answer (the readable
  mapping over the address, cut at 48, then `-` and the first twelve hex of SHA-256 over the whole address:
  `2e9a7b138059`) and ALSO equal to `await segmentOf(address)`. The literal was confirmed three ways at the plan
  review — native, Python twice and the WASM module in the pool — so no re-derivation is owed. A WASM build whose
  `readable.truncate(48)` (a byte cut of an all-ASCII string; the pool helper's `.slice(0, 48)`) or whose sha2 diverged
  from native would land elsewhere.

- [ ] **Step 4: S2a's four parked sentences and the message, exactly as the S2a ledger recorded their true forms** (each
  replaced span's END is stated, since the lines break mid-sentence):
  - `web/scripts/headers.mjs:192-196`, from "Every read of the file by a HOST" (the line breaks after "Every") up to
    the dash before "so the "passes the assembly, passes the NAS" sentence": → "Every JavaScript read of the file on a
    release's path — the Worker's assembly, the deploy guard, the e2e build's override — goes through this; the NAS
    reads it in Rust —" (the splice that keeps the tail was measured green);
  - `web/scripts/headers.mjs:155-157`, `parseHeadersJson`'s doc: insert "on a release's path" after "A FILE", and after
    "never `readFileSync(.., "utf8")`" insert "— the two precache cells read their own build's file with
    `readFileSync`; they are cells, not the release's path —" (`precache.node.test.ts:137, 287` do read the file that
    way; "hand it text of their own" was imprecise);
  - `crates/yokohama-web/src/lib.rs:449-453`, the `headers` field's doc, becomes: "Empty for a state built by the base
    constructor, which reads no header file (an in-memory state HAS a dist and serves no set), and for a dist without
    the file (the client build is older than 0.6.2, or absent — `main` says so once); a cell gives a state a set with
    [`Self::with_headers`]."; and the `with_headers` cell's message (`lib.rs:4128`) becomes
    `"the base constructor reads no header file, so no set"`;
  - D2's label: in `web/e2e/harness.node.test.ts`'s first Report-Only cell, after `fake.line("info", RO_INLINE, …)`,
    add `expect(watch.failures[0], "the label is the message's own type").toBe(\`console.info: ${RO_INLINE}\`);` —
    `consolePage()` already emits a line of any type (`harness.node.test.ts:1283-1299`), so no `fakePage.info` is
    needed (adding one would overlap `consolePage`).

- [ ] **Step 5: The outbox shows a 429's sentence.** The outbox ALREADY reads a 429 as a wait — `reasonFor` falls
  through to `offline` (`outbox.ts:196`); what the arm adds is the server's words. In `web/src/outbox.ts`'s
  `reasonFor`, beside the 507 arm (the arms run 401, 400/409, 413, 507, then this, then offline):

```ts
    // A 429 is the server's rate limit (security S2b, finding C2): the moment's, retried like an offline row, and
    // shown with the server's sentence.
    if (e.status === 429) return { reason: "offline", sentence: e.message };
```

  `reasonFor`'s doc (`outbox.ts:176-186`) lists the 429 beside the 507. In `outbox.test.ts`, beside the 507 cell,
  "reads a 429 as a wait that carries the server's sentence, not as a refusal". Run
  `cd web && npx vitest run src/outbox.test.ts && npx vitest run -c vitest.node.config.ts e2e/harness.node.test.ts`
  — the node project: `vitest.config.ts` includes `src/` alone, so the plan's first draft (`npx vitest run
  src/outbox.test.ts e2e/harness.node.test.ts`) printed "2 files / 65 tests" without ever running the harness cell
  (measured). Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add worker/scripts/deploy-guard.test.mjs crates/yokohama-web/src/archive.rs crates/yokohama-web/src/lib.rs worker/test/archive.test.ts web/scripts/headers.mjs web/e2e/harness.node.test.ts web/src/outbox.ts web/src/outbox.test.ts
git commit -m "tests and sentences: the fully escaped configuration key as committed rows, the crafted-name and 48-cut cells, S2a's four parked sentences and its message, D2's label pinned, the outbox reading a 429 as a wait (S2b Task 8)"
```

**Mutants (red where named):** M55 the guard reading the file as text for rule 4b → the escaped rows; M56 the
segment's digest over the readable half rather than the key → the crafted-name cell's LITERAL row (and
`a_caller_segment_is_path_safe_readable_and_injective`) — it survives the relative assertions (measured); M56′ the
digest suffix dropped → the crafted cell's `assert_ne!`; M57 `readable.slice(0, 40)` in the pool helper → the 48-cut
cell; M58 the 429 arm dropped → the outbox cell reads `offline` without a sentence; M59 the harness labelling every
line `error` → D2's cell.

### Task 9: The documents the code changed — Opus (D10's rule; spec corrections 1–12; the ledger's rows)

**Files:**
- Modify: `docs/security-ledger.md` (the rows B5, B6, C1, C2, D3, D23, D24; B12's Worker half inside B12's row; the
  Updated line), `docs/plans/2026-10-08-security-remediation-spec.md` (§9.7 appended, append-only),
  `docs/web-lane.md` (§1, §2, §3, §4, §5, §8, §10, §12, §13, §15, §16), `worker/RUNBOOK.md` (§2, §3, §5, §7, a forward
  note at §19), `worker/README.md` (§1.5, §7.2, §7.5, §7.6, §9.3, the two-doors recipe), `deploy/README.md` (§2's
  name rule, `:84`). `deploy/RUNBOOK.md` quotes no operator playbook (checked at the review) and is not touched.
  **The task schedules SENTENCES, not sections** (the review found eleven live sentences outside the first draft's
  section list): the list below is the inventory, and the implementer greps each named line at Task 8's head before
  editing, since the earlier tasks move lines.

**Interfaces:**
- Consumes: every name in §"Names shared across tasks" and every sentence the guard, the shell and the layer now
  print (quoted from the code at Task 8's head, never from this plan).
- Produces: the documents the close reads; `cargo test -p yokohama-web --test lane_document` (6) and
  `worker/scripts/lane-variables.test.mjs` green on the final text.

- [ ] **Step 1: The ledger.** Rewrite the rows (never append history): **B5** `fixed <commit>` (Task 1) — the
  cross-site refusal before identity and the operator POST's JSON gate, catchers `door.test.ts`'s B5 cells by name;
  **B6** `fixed <commit>` (Task 6) — the `.test` arm, the production file OFF, `wrangler.dev.jsonc`, the guard's rules;
  the token's duration `accepted` (X6, the owner's sentence); the `.dev.vars` mode and the `TURNSTILE_SECRET` line
  done at S1's wrap (2026-10-11); catchers the identity cells and the guard cells by name; **C1** `fixed <commit>`
  (Tasks 5 and 5b) — `ClaimPolicy::NamesOnly` in every production constructor, the client's `openDoorVerdict` (the
  name step and the new-table form), the Worker module `NamesOnly`, `claimedIdentity`'s collapse; the runbook rule
  that a populated deployment never changes its door and a public deployment starts empty lives in
  `worker/RUNBOOK.md` §3 — the file spec §2.2's C1 row names — with README §1.5 pointing to it; catchers by name; the
  live column: "the open door is not deployed; read at S4's own deployment"; **C2** `fixed <commit>` (Tasks 1–2) — the
  two bindings and their figures, the name grammar; the live column: "the wrap window's burst recipe
  (`worker/RUNBOOK.md` §7): about the 121st credentialed POST in one 10 s window reads 429, `Retry-After: 10` and the
  sentence; plan availability the wrap's deploy"; **D3** `fixed <commit>` (Task 3; the Worker's socket cap — the NAS's
  stream cap and body timeout stay S2c's, the row says so; the lobby draws no refusal, parked); **D23** `fixed
  <commit>` (Task 7) — the two digests, `redactKey` over `record_key` AND `detail`, the Archive object's digest name,
  `redact_query_string` and the sampling pin; **D24** `fixed` by collapse into C1 (Task 5b; the client's name step;
  §9.7 correction 10); **B12**'s row gains the Worker half: `fixed <commit>` (Task 4) with its catchers, and the
  sentence that the per-object serialization closes the race because one key is one object (Task 5's collapse). The
  Updated line names the head.

- [ ] **Step 2: The spec's §9.7**, appended after §9.6: the heading "§9.7 — 2026-10-12, S2b's brainstorm and plan: the
  owner's rulings R-1–R-6, thirteen decisions and twelve corrections", an intro naming the plan file and the head the
  three sections equal, then this plan's §"The owner's rulings for S2b", §"The decisions this plan takes" and §"Spec
  corrections" VERBATIM (byte for byte; `diff <(sed -n …) <(sed -n …)` in the report), and the run's rulings paragraph
  left for the close (Task 10 Step 6 appends it — the gap §9.5 showed must not recur).

- [ ] **Step 3: The lane document.** §4's Worker-host bullet gains the door's rules: the cross-site refusal before
  identity and the operator POST's JSON gate (B5); the two bindings, their keys and figures, the 429 and its two
  `Retry-After`s (C2); the socket caps and their refusal shape (D3); the live-count quota with the NAS's figures
  (B12); the dev door's `.test` arm and the production file OFF with the dev file (B6); the digests and the redaction
  (D23). §4's identity bullet gains `ClaimPolicy` (C1): `NamesOnly` in production, `Diagnostic` in the fixtures, the
  sentence, that the browser host passes its caller to dispatch unjudged, and that the Worker shell collapses a
  claim's space runs. Every live `deploy:gate`/`wrangler.gate.jsonc` outside a dated record becomes
  `deploy:dev`/`wrangler.dev.jsonc` — §1 (`:44`), §2 (`:78`; it also says the production file ships the dev door
  off), §3 (`:140`), §5 (`:217`, trap 36), §10 (`:454`). §2's counts are re-counted at the final head (Task 10 Step
  4). §8's dev-door row gains its state after S2b (the production file OFF; the dev file; the quoted sentence reads
  "delete it at the real flip"), and §8's "Still owed … a quota that is anyone's to spend" changes with C2. §12's
  `YOKOHAMA_DEV_DOOR` row reads `off` in production and names `wrangler.dev.jsonc`; §12's "names any address"
  (`:638`) becomes "names a `.test` address". §13's intro gains the 403 and the 429 every unsafe method may answer
  before dispatch, and the operator row the JSON requirement; §13's routing rule (`:693`, `:745`) is `isSafeId` — "a
  name the store refuses is the house's" — so the stream row's first half (an unsafe id reaching the table object) is
  now unreachable through the Worker; §13's `POST /api/archive` row (`:731`) says the Worker's archive object refuses
  the 257th record too; the 64-byte routing sentence at `:159` follows. §15: the quota entry (`:901`) is now the
  Worker's too, and the section gains the by-design entries — the per-location best-effort limit; the sampling rate
  kept at 1 and held by the guard; names only under the open door, with the owner's S4 shape recorded (R-3); the
  socket caps count OPEN sockets only, so a dangling refusal cannot lock anyone out, while the CLOSING sockets occupy
  the object until the platform's teardown (its 32,768); the open-door write key is per (IP, name) and not an IP ceiling; the
  object-wide caps can be filled under `open` by the very caller they bound (8 claims × 8 sockets fill a table;
  128 × 8 the lobby); a refused socket is held, uncounted, while it closes; the lobby refusal is silent until reload (AD-4); the
  R2 list runs before the layer's cheap refusals (AD-9). §16 gains one sentence under the columns: the two limits sit
  above the tier's per-page peaks (the figures from Task 10 Step 1); nothing about a paced replay (D-8 withdrawn).

- [ ] **Step 4: The runbooks and the README.** `worker/RUNBOOK.md` §2: the command block reads `deploy`,
  `deploy:dev`, `deploy:open` as `package.json` spells them; the guard's list gains checks 13 and 14, the dev-door rule
  and F-40's sentence; the secrets guard's cost paragraph gains D-7's recipe ("a NEW Worker's first deploy runs the
  deploy guard and `wrangler deploy --config <file>` by hand — a Worker that does not exist holds no secret — and the
  secrets guard from the second deploy on"), and its first-deploy sentence at `:195-199` ("S4's public Worker needs
  its own first-deploy path") says the same. §3's variables table: `YOKOHAMA_DEV_DOOR` `off`, with `wrangler.dev.jsonc`
  named; C1's runbook rule (a populated deployment never changes its door; a public deployment starts empty); the
  operator playbook's two POST lines gain `-H 'content-type: application/json'` with one sentence saying why (B5).
  §5's log shapes: the `operator` line (`:682`) becomes `{id?, action, operator}` with `operator` the 12-hex digest;
  the `archive-*`, `held` and `archive-owed-*` lines (`:604-641`, `:667`) carry redacted `record_key` and `detail`;
  `id` (`:608`) is `archive:<12 hex>`. §7: the live tiers open with `npm run deploy:dev` and close with
  `npm run deploy`; the replay's duration is UNCHANGED (D-8 withdrawn); the burst recipe goes in (AD-7):
  `B=https://yokohama.moore.nyc; for i in $(seq 140); do curl -s -o /dev/null -w '%{http_code}\n' -X POST -H 'content-type: application/json' -A "$BROWSER_UA" "$B/api/chat" -d '{}'; done | sort | uniq -c`
  over WARP as the owner — 400s, then 429s from about the 121st inside one 10-second window (the limiter counts before
  the layer's 400; counting is per location and best effort, so it may need a second run; `{}` posts nothing). A
  forward note at §19's flip recipe (`:2089`): "`deploy:gate` was retired in security S2b; this recipe reads
  `npm run deploy` for it, around `npm run deploy:dev`". `worker/README.md` §1.5: "Cloudflare's rate-limiting rules
  are the tool" becomes the Worker's own limits, and the `test/` root "takes only an address ending `.test`", never
  under `open`; §1.5 and §7.6: the dev door's `.test` rule, OFF in production, the dev file ("The dev door is a
  development door, and it is on this cycle" becomes "off in production; `wrangler.dev.jsonc` opens it"); the open
  door's name rule with a pointer to the RUNBOOK §3 rule (C1's row); §7.2 (`:929`): "sixteen" becomes "eighteen" and
  names `share_prefix` and `set_share`; §7.5 gains the receipt loop's step 0 and the put's quota arm; §9.3: checks 13
  and 14, the dev file, F-40, D-7's recipe, "all twelve" → "all fourteen" (`:1621` too); the two-doors recipe:
  `deploy:dev`/`deploy` in place of `deploy`/`deploy:gate`. `deploy/README.md` §2 (`:84`): delete "- an email address
  is also a name;" and add "- an address is refused here: under the open door a name is a name, and an `@` always
  means a verified address on every other door (security S2b, C1);". The dated deploy records that spell
  `deploy:gate` are records and stay.

- [ ] **Step 5: The two document cells and a read-through.** `cargo test -p yokohama-web --test lane_document` and
  `cd worker && npx vitest run -c vitest.scripts.config.ts scripts/lane-variables.test.mjs`. Expected: PASS. Then
  `grep -n "deploy:gate\|wrangler\.gate" docs/web-lane.md worker/README.md worker/RUNBOOK.md deploy/README.md` — NO
  `-v` filter (the first draft's filter hid live lines that merely cite a section number): every hit left is in
  `worker/RUNBOOK.md` §19–§22 (dated deploy records) or `docs/web-lane.md` §8's cycle-E row, each a dated fact — read
  by SECTION, not by line number, since this task's edits move the lines.

- [ ] **Step 6: Commit.**

```bash
git add docs/security-ledger.md docs/plans/2026-10-08-security-remediation-spec.md docs/web-lane.md worker/RUNBOOK.md worker/README.md deploy/README.md
git commit -m "docs: security S2b — the ledger's seven rows, the spec's §9.7, the lane document's door rules, the runbooks' deploy:dev and the operator playbook's content type, the README's dev door and open-door name rule (S2b Task 9)"
```

### Task 10: The plan review, the pre-flight, the branch, the dispatches, the suites, the after review, the close, the PR and the hand-off into S2c — ORCHESTRATOR (rulings R-1, X1, X3, X4; decisions D-12, D-13)

- [ ] **Step 0: The plan review (this session, 2026-10-12) — the owner's R-1.** A Workflow of four Sonnet lenses on
  `git archive` exports of `85c8dd02` with this plan copied in, each EXECUTING the plan's blocks for its slice and
  measuring every platform claim, then one Opus assembler that dedups, reads every contradiction at the code and
  hands the orchestrator a packet with adjudications: lens 1 the Worker shell (Tasks 1–3: B5, C2, D3 — the cells
  spliced and run in the pool and in node; the binding's shape against the installed wrangler schema and miniflare's
  plugin); lens 2 the module and the layer (Tasks 4–5: B12, C1 — the Rust cells run; the corpus walk and the open
  project's rows that move; the browser host untouched); lens 3 the tooling and the configurations (Tasks 6–8: B6,
  D23, the owed cells — the guard cells run; `wrangler deploy --dry-run` over a tree carrying the new blocks; the
  replay's pacing cell); lens 4 the documents against the code and the decisions (the rulings transcribed against this
  session's words; every figure's arithmetic; the task order's interfaces; placeholders; the window's independence).
  Every lens writes its report under `docs/temp/security-review-2026-10-08/s2b-plan-review/`, replies with the path and
  three lines, runs no `rm -rf`, arms no Monitor, makes no call to the Cloudflare account. The packet's items are
  adjudicated by the orchestrator and SPLICED INTO THIS PLAN at the source (as S2a's were); the record is this plan's
  §"Review record". **Done 2026-10-12** — the packet landed (80 merged findings, 11 adjudications), every correction is
  in the text above, and the record below says what was decided.

- [ ] **Step 1: The pre-flight (the next session; X4) — SHORT, and no mutants.** The owner's rule (2026-10-12): the
  pre-flight does not re-walk what the implementers' probes, the task reviewers and the after review's battery will
  walk; the overlap among those three is accepted, and the pre-flight is only what X4 asks — Tasks 1 and 2's cells
  executed once at the branch head, and the figures measured. The lenses already executed Tasks 1–3's corrected
  slice green on `85c8dd02` (the packet's `artifacts/lens1-slice-1-3-corrected.patch`: worker `typecheck` clean,
  `npm test` 13 files / 238, `test:scripts` 29 files / 574), so the pre-flight is a RE-RUN of that slice to confirm it
  still holds, not a search. On an export of `security-s2`'s head with the reviewed plan (`git init && git add .`
  first, PF-76): Tasks 1 and 2's cells spliced from the plan's blocks and executed (`npm run build:module:debug`, the
  pool's both projects, `test:scripts`, `typecheck`); then the figures: (a) the local two-browser walk's per-page peak
  of writes in any 10-second window — `YOKOHAMA_WEB_TALLY=<file> npx playwright test two_browsers` then a one-line
  node pass over the tally's JSON lines grouping `method !== "GET"` rows by `page` into 10 s windows (the report's
  `WINDOW_MS`), the peak printed per page; the bound to beat is about 52 unsafe requests in a page's peak window
  (derived from the cycle-F tier's 338-request peak, PF-69); D-3's 60 holds only if every peak sits under it with
  margin, else the figure moves HERE first; (b) — withdrawn with D-8 (nothing to pace); (c) `npx wrangler deploy
  --dry-run --config wrangler.jsonc` over the tree with the new `ratelimits` and `observability` keys — MEASURED at the
  review: exit 0 in 1.3 s, no token, no login, for the production, dev, open and pool files; an integer
  `namespace_id` and `period: 30` are refused, a misspelled key warns; it cannot say whether the account allows the
  binding or whether 7301/7302 are free (the wrap's deploy is that proof) — re-run only if the blocks change; (d) the
  pool's binding answers `limit()` (Task 2's smoke cell, measured true). A block the pre-flight finds red is corrected
  in this plan first, as a NEW paragraph in the Review record, and the run proceeds; nothing else is re-litigated here.

- [ ] **Step 2: The branch and the first commits.** `git checkout -b security-s2b 85c8dd02`; the plan and the board's
  Track 4 as the first commit; the pre-flight's corrections (if any) as the second; the SDD ledger opened at
  `docs/temp/security-review-2026-10-08/s2b-sdd/progress.md`.

- [ ] **Step 3: The dispatches (superpowers:subagent-driven-development).** Tasks 1–5, 5b, 6–9 in order; Opus
  implementers and reviewers, Task 8's implementer Sonnet; each brief cut from this plan with §"Names shared across
  tasks" and the Global Constraints; each implementer PROBES its brief and reports `DONE_WITH_CONCERNS` on a false
  sentence; each review on an export with the task's named mutants; a fix round is a NEW commit; one writer in the
  tree at a time (a reviewer's export never the tree); `npm run build:module:debug` before every pool run from Task 4
  on. Every ruling a `Ruling:` line in the ledger; every plan correction at the source first. Task 5b's gate is the
  local Playwright tier.

- [ ] **Step 4: The suites at the final head.** The per-edit suites and the web lane's rows (`docs/web-lane.md` §2):
  core, CLI, oracle, difftest, bridge (the lib/bin suite AND its four node suites:
  `node bridge/tests/outbox_race.js && node bridge/tests/wrapper_transparency.js && node bridge/tests/safe_table_parity.js && node bridge/tests/connected_indicator.js`),
  eval_probe, solver, clippy; `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`;
  `cargo test -p yokohama-web --no-default-features --features server --lib identity::` (the public build's spelling)
  and `cargo check -p yokohama-web --no-default-features --target wasm32-unknown-unknown` (the browser host's
  dependency check, web-lane §2); `web`: `typecheck`, `vitest run`, `test:wasm`, `npm run e2e` (the local Playwright
  tier, by names after Task 5b); `worker`: `typecheck`, `test:scripts`, `npm test` (both projects). No roster (D-12).
  The counts recorded against the board's, and `docs/web-lane.md` §2's counts re-counted;
  `git diff 85c8dd02 --stat -- crates/yokohama-core crates/yokohama-cli crates/yokohama-oracle
  crates/yokohama-difftest crates/yokohama-bridge crates/yokohama-wasm crates/gen-data web/src/wire web/src/wire-script`
  EMPTY; `SEMANTICS_REV` 7.

- [ ] **Step 5: The after review and its one wave.** The orchestrator's own re-derivation of every row from the code;
  one Opus battery over every named mutant (M1–M59 and the lettered additions the review added) on an export — the
  third and last walk of the mutants, overlapping the implementers' probes and the task reviews by design; one fix wave
  for the deferred minors, re-reviewed; the rest parked with true forms in the ledger.

- [ ] **Step 6: The close.** The ledger (Task 9's rows confirmed at the head), the board's Track 4 (S2b landed; the
  window owed; S2c next), guidingpath's 2026-10-12 row and current state, CLAUDE.md's standing sentences (the dev door
  OFF in production and `deploy:dev` around a live tier; under the open door a name is a name; the two limits and the
  caps as plausibility bounds; the quota's live count), this plan's §"Review record" (the pre-flight, the run, the after
  review) and the spec's §9.7 run's-rulings paragraph, the memory file; the SDD ledger left in place.

- [ ] **Step 7: The PR.** `security-s2b` pushed; a PR against `security-s2`, its body naming the rows and the readings;
  the merge S2's wrap (X1).

- [ ] **Step 8: The hand-off into S2c.** The board's Track 4 rewritten: S2b landed; **the first window still OWED**
  (S2a's commit tagged `v0.6.2` and deployed, the readings taken, any slot — S2b changed nothing it depends on; **the
  window runs from the `v0.6.2` tag's tree, never from a `security-s2b` checkout**, R-1); S2c's brainstorm next
  against the stack's head: B10's code half under the dependency fence, D3's stream cap and the body read timeout on
  the NAS, D8, the explain surface's two rules (the withheld-`why` timing channel; HI-F2's design option), the
  inherited residuals (the lobby's silent cap refusal, AD-4, owed before any open door is deployed), the wrap (0.6.3
  enforcing; the Report-Only reading's four shapes; the stack review; the merges; the second window with the live
  acceptance of S2b's rows — the 429 burst recipe (AD-7), the socket cap, the operator playbook, the dev door OFF read
  live). The memory updated the same way.

---

## Review record

### The plan review (2026-10-12) — four Sonnet lenses on exports, one Opus assembly, the orchestrator's adjudication

**The run.** One Workflow (`wf_3f867545-969`; 5 agents, 983 tool uses, about 90 minutes): four Sonnet lenses, each
on its own `git archive` export of `85c8dd02` with this plan copied in, EXECUTING the plan's blocks for its slice —
lens 1 the Worker shell (Tasks 1–3), lens 2 the module and the layer (Tasks 4–5), lens 3 the tooling and the owed
cells (Tasks 6–8, Task 5's client, the pacing), lens 4 the documents against the code and the decisions — then one
Opus assembler that read all four reports and the plan whole, checked 478 files of its export against `85c8dd02`
(0 differ), re-read every contradiction at the code, and wrote the packet. The lens reports and the packet are under
`docs/temp/security-review-2026-10-08/s2b-plan-review/` (`lens-1-shell.md` … `lens-4-docs.md`, `assembled.md`), with
the three artifacts worth keeping copied to `artifacts/` there: lens 1's measured-green corrected diff for Tasks 1–3,
lens 2's `receipt.rs` splice with the stale-listing probe and the clear, and lens 2's race probe through `handle`.

**The counts.** 106 lens findings merged into 80: Critical 1, Important 39, Minor 27, Information 13; 14 platform or
live claims no offline check reaches (packet §8); 11 adjudications. Of the 80, every one whose correction a lens
measured green is in the text above in its measured form; the few marked "(proposed)" — AD-6's `catch`, AD-10's
sampling clause — are named as such at their blocks.

**What the review found, in one paragraph.** Only two MECHANISMS of the design were wrong, both coverage gaps: the
open door's spelling race (two spellings of a name were two Archive objects over one share; 257 records in a 256-record
share, measured) and D23's two missed fields (`logOutcome`'s `detail` and the Archive object's own log `id`, both
spelling the address). Everything else was the plan not running as written: a duplicate import that failed to parse,
`node:` imports that failed `tsc` under workerd's types, fourteen operator cells and one live-replay cell turned red by
Task 1's own gates, four chat literals of 200 that are 201, three Rust cells missing `serial()`, a put arm that
reddened four existing cells, a `replaceOnce` that threw, a `guarded` call that turned R-C10's 401 into a 500, a
`--help` the guard does not have, a replay pacing (D-8) built on a 138-second figure from before S1b when the replay
now sends 26 rows, a `probe-burst.mjs` that sends GETs alone and can read no 429, and — the Critical — a C1 rule that
turned the whole local Playwright tier red (twelve of twenty specs sign in as addresses at the open door) with no task
to migrate it. Three mutants were green as written (M4, M14, M27) and have real catchers now.

**The adjudications, decided by the orchestrator (the owner's standing ruling of 2026-10-11: "my instinct is to use
the suggestions you make"; each took the assembler's recommendation):**
- **AD-1 (b):** the client and the Playwright identities are their own Task 5b, after Task 5, gated by `npm run e2e`.
- **AD-2 (a):** D-8 withdrawn; `PACE_MS` gone from every site; D-3's premise is the replay as it is (26 rows).
- **AD-3 (a):** `claimedIdentity` collapses space runs, in Task 5 (Step 7b), with the race cell; D-5 says "per object,
  and one key is one object".
- **AD-4 (b):** the lobby's silent refusal is RECORDED (D-4, §15) and parked; the client change is owed before any
  open door is deployed — a client design question for the owner, not folded in.
- **AD-5 (a):** two `Retry-After`s, `"60"` for a create and `"10"` for a write.
- **AD-6 (b):** a limiter that throws is logged `limiter-failed` and the request proceeds — the limit bounds cost,
  never access. (The house habit elsewhere is fail-closed; the owner may overrule to (a) at the same cost.)
- **AD-7 (b):** the live 429 is read by a curl recipe in `worker/RUNBOOK.md` §7, not a new script (R-6's spirit).
- **AD-8 (a)+(c), (d):** the new-table form refuses an `@` as the name step does; seats are recorded as reservations
  the layer does not judge; `openDoorVerdict` refuses any raw `@` before the grammar.
- **AD-9 (a):** the R2 list before the layer's cheap refusals is accepted and recorded in §15.
- **AD-10 (a):** check 14 also pins `head_sampling_rate` absent or 1.
- **AD-11 (b):** dotted create ids are recorded in D-11, not routed to the house; the Goal reads "names the store
  refuses".

**The R-1 answer, verified by lens 4 and the assembler:** this plan does not depend on the 0.6.2 deploy; the window
may run before or after, PROVIDED it runs from the `v0.6.2` tag's tree (never a `security-s2b` checkout, which has no
`deploy:gate`, ships the dev door OFF and binds the rate limits before the namespace check). Recorded in R-1 above and
in Task 10 Step 8.

**The pre-flight, re-scoped by the owner (2026-10-12):** it runs no mutants — the implementers' probes, the task
reviews and the after review's battery each walk them, and that threefold overlap is accepted so the run never stalls
on de-duplicating it; the pre-flight is X4's two tasks' cells once and the figures (Task 10 Step 1). Since the lenses
already ran Tasks 1–3's corrected slice green, it is a confirmation re-run, not a search.

**Conduct (packet §7).** No lens ran a bulk `rm`, armed a Monitor, waited on an event, wrote to the repository beyond
its report, or called the Cloudflare account. Three small departures, each named in the packet: lens 1 removed two
single files it had created; lens 2 wrote an 11-byte file to `/tmp` and an empty folder beside its export; lens 3
listed the names in the wrangler config folders (opened nothing) and ran eight `wrangler deploy --dry-run`s with no
token, which its brief allowed; lens 4 read lenses 1 and 3's reports before finishing (weakening independence by one
finding, which it then verified at the tree). The exports (about 7 GB, mostly `target/`) and the stray files are the
orchestrator's to remove; they are listed in packet §7.

**Premises found true (packet §6; do not re-try):** 40, among them — `crossSite`'s nine foreign shapes all 403 before
the operator gate; `isSafeId` agrees with all 39 `id_cases.json` rows; the 507 sentence is byte-equal to
`archive_full_sentence(256, 128 << 20)`; miniflare's R2 truncates at 1,001 and not at 1,000; the wire corpus's 117
files are byte-identical under Task 5; the browser host is untouched and builds for `wasm32`; the 48-cut literal is
right three ways; wrangler 4.136.3 refuses an integer `namespace_id` and knows `redact_query_string`; `dispatch`
answers `NoRoute` before identity; M1–M59 are each defined once.

**Unverified (packet §8; the wrap's deploy and the live window are the proofs):** that Workers Free allows the
binding and 7301/7302 are free; that `redact_query_string` removes `claimed=` from persisted logs and whether
Observability records an object's NAME elsewhere; that the edge sets `cf-connecting-ip`; Playwright's request context
sending neither `Origin` nor `Sec-Fetch-Site`; CLOSING and zombie socket lifetimes in production; the local tier
itself (nobody ran Playwright); the lobby's rendering of the refusal; R2 itself at exactly 1,000 and its pricing;
R2's own failure message embedding a key; input-gate behaviour of an awaited digest at the top of `fetch`; the live
429; R-2–R-6's words (only R-1 was checked against the relayed request); the release module with 18 exports; the
32,768-socket and `Origin: null` documentation claims.

### The pre-flight (2026-10-12, the same session, after the owner's go) — two agents on two exports, no mutants

**The run.** Two agents in parallel, each on its own `git archive` export in the session's scratchpad, neither
touching the tree: an Opus agent spliced Tasks 1 and 2's blocks from the plan verbatim (Task 1 from the committed
text `faba814e`; Task 2 from the working tree after the write-figure correction below, so the 120 figure is what it
measured) and ran every step in order, RED and GREEN; a Sonnet agent ran the local two-browser Playwright walk with
the tally on an export of `85c8dd02`. Reports: `docs/temp/security-review-2026-10-08/s2b-before/preflight-splice-report.md`
(the splice as `preflight-splice.patch`) and `preflight-tally-report.md` (the tally as `preflight-tally.ndjson`).
The `wrangler deploy --dry-run` reading (c) was not re-run: the blocks it measured did not change shape (a figure
inside `simple.limit` moved).

**Figure (a) — 60 does not stand; the write limit is 120 per 10 s.** The walk (`1 passed (3.4m)`; 1,870 moves,
198 s) tallied 13,095 requests, 2,112 of them POSTs, all but seven `POST /api/games/<id>/input`. Per page, the peak
of unsafe `/api` requests in a sliding 10 s window: a 71, b 69, the watcher 2 (fixed buckets 65, 68, 2); the median
fixed bucket 56 and 52 — the peak is sustained, not a spike, so 60 would trip on ordinary buckets of the very
instrument the limit must survive. The orchestrator's ruling (the SDD ledger): `WRITE_LIMIT` is **120 per 10 s** at
every site of this plan — 1.7× the measured peak, the live tier being slower than the local walk, never faster; the
window, `Retry-After: "10"`, the create limit and the namespaces are unmoved; the burst recipe reads about the 121st
of 140. Seventeen exact replacements, listed in the ledger's ruling.

**The splice — Task 1 green as written; Task 2 two one-token corrections, both in Step 9's block, folded in above.**
Task 1: Steps 2, 4, 6, 9, 10, 10b and 11 as the plan says, every cell green on the first run after each step.
Task 2: (C1) the throwing-limiter cell held `await person(RED)` inside a non-async arrow — a PARSE error that dropped
all of `door.test.ts` (`Test Files 1 failed`); `reaching(async () => …)` is the fix, measured 12 files / 237. (C2)
figure (d)'s cell read `c.limits` off an un-narrowed `shellConfig` result — TS2339 under `npm run typecheck` while the
pool ran it green (the pool never type-checks); the narrowing line `if ("error" in c) throw new Error(c.error);` is
the fix, at the cell and in Step 6's prose. Figure (d): the pool's binding answers `limit()` — the smoke cell green,
and a read-only probe confirmed the pool's two bindings are distinct objects. Nine sentences the measurement
contradicted are corrected at their sites: Step 2's and Step 6's expected failures (import-as-`undefined`, not a
missing export; 4 failed / 25 passed; the `pretest` fixture a bare `npx vitest` skips), Step 10b's counts (14 failed /
210, then 224/224 — the 216/230 were the lens's Tasks 1–3 splice, carried over), the config fixture's one shared fake
(now two, so "uncrossed" can be seen), the 429 cell's second header asserted, the in-code mutant comment that named
M14's equivalent form, "three new cells" (two), the open project's missing `Limiter` import and the cell's place,
Step 3's module-doc sentence, Step 12's guard output on a tree with no assets. Two things the pre-flight did not
verify, left to Task 2's reviewer: the pool comment's "more than ten creates a minute as one caller" (nine literal
call sites; loops untraced) and the Rust suites (no crate byte moved).

**Final counts at the splice's head (Tasks 1–2 only, the 120 figure):** worker `typecheck` clean; `npm test` 12
files / 237 (worker 11 / 229, open 1 / 8); `test:scripts` 28 files / 571 passed, 1 skipped. Not comparable to the
lens's 238 / 574, which carried Task 3.

### The run and the after review

(to be written at the close)
