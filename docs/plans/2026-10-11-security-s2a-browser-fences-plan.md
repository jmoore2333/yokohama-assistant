# Security S2a — the browser's fences: implementation plan, and the first of S2's two windows

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan
> task-by-task. Every code task subagent-driven — **Opus** for every reviewer, **Opus** for every implementer but
> Tasks 5 and 6, whose briefs carry the whole judgment and run **Sonnet** (the owner's ruling X3, 2026-10-11, over
> S1's tiers) — and a **Mythos** orchestrator that conducts the plan review at the top (Tasks 1 and 2's cells
> executed, every figure measured — the owner's ruling X4) and the whole-branch review at the end ITSELF, on
> exports; the suites, the documents' close, the SDD ledger, the branch, the PR, the tag and the deploy window are
> the orchestrator's too (Task 9). Steps use checkbox (`- [ ]`) syntax. **Read the spec first**
> (`docs/plans/2026-10-08-security-remediation-spec.md`: §1 the rulings R-SEC5, R-SEC8(g) and R-SEC10, §2.2 the
> rows B4, B9, C3, the Lows' row and "the CSP", §3.3, §5.3 the live acceptance, §6 the fences, §9.2 the stacked
> branch), then this plan's §"Names shared across tasks" — an implementer sees only its own task, and that
> section is how it learns what its neighbours produce. Every `file:line` here was read at master `1e9c092d`
> (S1's wrap, 2026-10-11); **find by content once an earlier task has moved a file.** The implementer PROBES the
> brief's code before its own review and reports `DONE_WITH_CONCERNS` when a sentence here proves false.

**Goal:** Close the browser-side rows of the security review — a framed, signed-in page that acts for its victim
(B4), the service worker's synthesized navigations that drop every header the host set (D13), the Worker's API
answers with no cache directive (B9's Worker half, the S1 stack review's DC-F4), a hash router that admits `..`
into an API path (D14), a browser profile holding a session cookie that a `docker build` would copy (D15), a
deploy that follows a redirect file the guard never judged (D18) and a guard that lets an open-door configuration
ship a `workers.dev` URL (C3) — ship the c1 review's header set from both hosts with the Content-Security-Policy
in Report-Only for one release, record D16 as `accepted` on the owner's word, bump the version to 0.6.2, and run
the first of S2's two windows: both hosts deployed, the headers read live, the Report-Only policy's violations read
on a first visit, and the two live readings S1 never took (D1's stream end, B1's withheld `why`) taken with a
second Access identity.

**Architecture:** One header set, generated ONCE where the build's values are known. (1) `web/scripts/headers.mjs`
computes the set — the policy with its `connect-src` hosts derived from the checked build values, the inline style
of the 404 page hashed into `style-src`, and the five companion headers — and the client build writes it to
`dist/headers.json`; nothing else spells a directive. (2) The Worker's asset layer carries it as a `/*` block the
assembly appends to `_headers`, and the deploy guard holds the assembled file to the static text plus the current
build's block; `_headers` never applies to a Worker-generated answer, so `handle` marks every `/api` answer
`no-store` and `nosniff` itself. (3) The NAS reads `dist/headers.json` at start and ONE axum layer sets the set on
every answer, `no-store` under `/api` beside it. (4) The service worker copies the stored shell's headers onto every
navigation it synthesizes, so a returning visitor gets the headers the host set on `/shell.bin`, its `no-store` among
them; the entry point refuses to render inside a frame. (5) The hash router admits exactly the store's id grammar,
pinned to a fixture the server emits.
(6) The deploy tooling refuses a redirect file, a `workers.dev` URL under any door and a Worker secret named like
a diagnostic switch; the pinned instrument leaves no cookie behind; the Docker context carries no secret file and
no profile.

**Tech Stack:** TypeScript and node ESM (`web/scripts`, `web/src`, `web/e2e`, `worker/src`, `worker/scripts`; the
vitest, node and Playwright suites); Rust 1.97.1 (`crates/yokohama-web`: `assets.rs`, `lib.rs`, `tests/start_refusal.rs`,
`tests/wire_fixtures.rs`); the committed wire corpus (ONE new file, `web/src/wire/id_cases.json`, Task 5; the version
fields in Task 8); `deploy/smoke-image.sh` and `deploy/Dockerfile.dockerignore`; the documents (`docs/web-lane.md`,
the spec's §9.6, `worker/RUNBOOK.md`, `worker/README.md`, the ledger); the release bump's steps (`docs/web-lane.md` §7).

**Spec:** `docs/plans/2026-10-08-security-remediation-spec.md` — §2.2's rows B4, B9 (the Worker's `respond`
half), C3, the Lows' row (D13, D14, D15, D16, D18) and "the CSP"; R-SEC5 (every control holds with Access
removed), R-SEC8(g) (done at S1's wrap), R-SEC10 (live validation is the acceptance); §3.3; §5.3; §6 the fences;
§9.2 the stacked branch. The S1 stack review's DC-F4 (the Worker's table answer carries no `Cache-Control`). The
corrections this plan makes to the spec are §"Spec corrections (to be recorded as §9.6)" below — recorded forward
by Task 7.

## The owner's rulings for S2 (2026-10-11), and what each binds

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

## The decisions this plan takes (the orchestrator's; the owner read them at the plan review, 2026-10-11)

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
  is not a transport at all, whatever its `User-Agent` (W-1, `worker/RUNBOOK.md` §21, 2026-10-01: "a browser
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

## Spec corrections (to be recorded as §9.6 by Task 7, dated 2026-10-11)

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

## Global Constraints

- `SEMANTICS_REV` (`crates/yokohama-core/src/game/semantics.rs`) stays **7**; `crates/yokohama-core`,
  `crates/yokohama-oracle`, `crates/yokohama-difftest`, `crates/yokohama-bridge`, `crates/gen-data`,
  `crates/yokohama-wasm`, `crates/yokohama-worker` and `crates/yokohama-cli` are byte-identical to `1e9c092d`
  (`git diff 1e9c092d -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest
  crates/yokohama-bridge crates/gen-data crates/yokohama-wasm crates/yokohama-worker crates/yokohama-cli` empty at
  every commit but Task 8's version bump, which moves `Cargo.lock`'s member lines and nothing under `crates/`).
  The Rust that moves is `crates/yokohama-web` alone: `assets.rs`, `lib.rs`, `main.rs`, `tests/start_refusal.rs`,
  `tests/wire_fixtures.rs`.
- No bot behaviour moves; no layer route changes its body or its status: this cycle adds HEADERS to answers and
  fences to the client, the deploy tooling and the instrument. The one wire change is additive — a NEW fixture
  file, `web/src/wire/id_cases.json` (Task 5; one regeneration, its diff read: the new file and nothing else; the
  two corpus-replay lists that name every file no script serves — `NOT_SERVED` in `web/src/local/corpus.node.test.ts`
  and `THIRD_COLUMN` in `worker/test/corpus.test.ts` — gain its name by hand) — and the version fields in Task 8
  (`health.json`'s `server_version`, the envelope's `app_version`, `archive-walk.json`'s records; `docs/web-lane.md`
  §7's rule). Any other moved byte, in any task, is a finding.
- **The front door stays static** (the owner, A1): nothing in this plan adds a Worker invocation to a door visit,
  a local game or a release file; the headers for static answers ride `_headers`, applied by the asset layer, and
  the client makes no new request. Bot Fight Mode stays on.
- The policy ships as `Content-Security-Policy-Report-Only` in this cycle (`CSP_MODE = "report-only"`, pinned at
  the literal); the e2e build ALONE rewrites its own `dist/headers.json` under the enforcing name (D-9);
  `X-Frame-Options: DENY` and the four companions are ENFORCED from this release. No directive carries
  `'unsafe-inline'` or `'unsafe-eval'` (a cell; `'wasm-unsafe-eval'` is another keyword, and required).
- Every constant carries the owner's comment rule from S1b: a concise in-code comment defending the figure or the
  list (`CSP_MODE`, `CLOUDFLARE`, `NOT_CARRIED`, `ID_MAX`, `FORBIDDEN_SECRETS`, `REDIRECT_FILE`). The comments in
  this plan's blocks are that text; an implementer transcribes them.
- TDD: the failing cell first, then the code, then green; `cargo clippy --workspace --all-targets -- -D warnings`
  clean after every task that touches Rust; `npm run typecheck` clean in `web/` and `worker/` after every task
  that touches TypeScript; no `cargo fmt`; stage with explicit paths, never `git add -A`; one commit per task at
  its end, the task's suites green at that commit; a reported SHA is never amended — every later change is a new
  commit.
- The branch is `security-s2`, cut from master `1e9c092d` by the orchestrator (Task 9 Step 2); its PR (#52,
  `security-s2` → `master`) is opened at Step 7 and merges at S2's wrap, after S2c (X1).
- Every task that touches what the web lane compiles runs the lane's rows (`docs/web-lane.md` §2) whole before
  its commit — `cargo test -p yokohama-web`, `-p yokohama-wasm`, `-p yokohama-worker`; `cd web && npm run
  build:wasm && npm run typecheck && npm test -- --run && npm run test:wasm`; `cd worker && npm run
  build:module:debug && npm run typecheck && npm run test:scripts && npm test` — and records every count; Tasks 1,
  4 and 8 run `npm run e2e` too (the client's build, its shell and its version moved).
- Every dispatched agent: the full report to a file under `docs/temp/security-review-2026-10-08/s2a-sdd/`
  (gitignored), a three-line reply; no `rm -rf`, `rm -r` or bulk `rm -f`; no call to the Cloudflare account, a
  read-only one included (the secrets guard's command is the orchestrator's to run); no Monitor, no event waits;
  long commands in the foreground or polled; reviewers run their mutants on a `git archive` export, never in the
  implementer's tree, and run Rust on a PRIVATE `CARGO_TARGET_DIR` per export (a shared target is not isolation:
  two exports overwrite each other's test binaries — measured at the plan review); the probe-and-name-concerns
  paragraph in every implementer brief.

## Review Focus

Five inputs the spec implies and this plan pins, each to the task that owns the code:

1. **A first visit whose HTML the edge rewrote.** Bot Fight Mode's JavaScript Detections inject an inline script
   into every HTML answer at the edge; under the Report-Only policy it is REPORTED, never blocked, so the visit
   renders and the console reads two `info` lines at the page per first visit ("Executing inline script violates the
   following Content Security Policy directive 'script-src …' … The policy is report-only, …" — one injection, two
   lines, measured on a JSD-shaped injection at the plan review). Task 4's live-tier forgiveness names exactly that
   line, read at ANY console level, and counts it; the window reads it on both hosts (Task 9 Step 9); the
   pre-flight reads what enforcement would block (Task 9 Step 1).
2. **A returning visitor whose navigation the service worker answers.** The synthesized page carries every header
   the stored `/shell.bin` carried, the host's set among them, with the HTML type and without the three the bytes
   no longer match. Task 4's node cell over the real built worker.
3. **A Worker-generated answer under `/api`** — a table's frame, the house's 404, the body limit's 400, a 401,
   `/api/art`'s HEAD — carries `no-store` and `nosniff`; a 101 is returned as it came; `/enter`'s 302 and a path
   outside `/api` are not this wrapper's. Task 2's pool cell, whose rows answer 200, 400, 401 and 404 (the pool's
   configuration has no operator list and no broken archive, so a 403 and a 500 are not rows: the wrapper is
   status-blind, and M8 is its catcher).
4. **A link whose id is `..`, `x.table`, `a b` or sixty-five characters** lands on the door, never on an API path;
   `sim-…` and `local-…` ids still open their tables. Task 5's cells, pinned to the store's verdicts.
5. **A stray `.wrangler/deploy/config.json`, a `workers.dev` URL in an open-door file, or a Worker secret named
   `YOKOHAMA_DIAGNOSTIC_SEEDS`** — each refuses the deploy, named. Task 6's cells.

## Execution shape

- Reviewers **Opus** for every task; implementers **Opus** for Tasks 1, 2, 3, 4, 7 and 8 and **Sonnet** for Tasks 5
  and 6 (X3). Task 9: the orchestrator, **Mythos**, who conducts the "before" plan review (Tasks 1 and 2's cells
  executed against scratch implementations of this plan's blocks in an export; the header bytes measured; the
  Playwright tier run once under `enforce`; every named mutant of Tasks 1 and 2 read red) and the "after"
  whole-branch review itself, then the branch's PR, the tag, the release and the window.
- Every implementer dispatch carries the probe-and-name-concerns paragraph: try the mechanism on cases the brief's
  cells do not cover, with vacuity as the first lens — for each cell, which mutation makes it fail? — and report
  `DONE_WITH_CONCERNS` naming any sentence of this plan the code contradicts; a `DONE_WITH_CONCERNS` correctness
  item is ruled on and fixed as a new commit BEFORE the task's review. Every fix-round dispatch requires the hunks
  quoted and `git diff` run before the commit.
- The SDD ledger (`docs/temp/security-review-2026-10-08/s2a-sdd/progress.md`) records each task's tier, its
  commit, its review verdict and every `Ruling:` line, as S1c's did.

## Names shared across tasks

| name | where | meaning |
|---|---|---|
| `CSP_MODE: "report-only" \| "enforce"` (= `"report-only"`) · `CSP_HEADER` · `CLOUDFLARE` · `styleHashesOf(html) -> string[]` · `cspOf({siteUrl, releaseFeed, styleHashes}) -> string` · `headerSet({siteUrl, releaseFeed, styleHashes, mode?}) -> [string, string][]` · `headersBlock(set) -> string` · `headersJson(set) -> string` · `parseHeadersJson(text) -> [string, string][]` | `web/scripts/headers.mjs` (+ `headers.d.mts`) | the one place a directive is spelled; the set as ordered `[name, value]` pairs; the `_headers` `/*` block; the file's text and its reader (Task 1) |
| `dist/headers.json` | written by `vite build` (`web/vite.config.ts`'s stamp plugin) beside `build-values.json`; not precached | the set both hosts serve, as `headersJson` writes it (Task 1) |
| the `/*` block of `worker/assets/_headers` | appended by `assemble-assets.mjs` from `web/dist/headers.json`; held by `deploy-guard.mjs`'s check 9 | the static release's headers on the Worker (Task 2) |
| `API_HEADERS` · `isApiPath(pathname) -> boolean` · `withApiHeaders(res) -> Response` · `answer(req, env, ctx, keys)` (the former body of `handle`) | `worker/src/index.ts` | `no-store` and `nosniff` on every `/api` answer but a 101 (Task 2) |
| `assets::response_headers(root: &Path) -> Result<Vec<(HeaderName, HeaderValue)>, String>` · `AppState.headers: Arc<Vec<(HeaderName, HeaderValue)>>` · `AppState::with_headers(self, headers) -> Self` · the layer `response_headers` (replacing `no_store`) | `crates/yokohama-web/src/assets.rs`, `lib.rs` | the file read, the state's copy, the one layer that sets the set on every answer and `no-store` under `/api` (Task 3) |
| `NOT_CARRIED: ReadonlySet<string>` · `shellResponse(hit: Response) -> Response` (changed: takes the stored Response) | `web/src/sw.ts` | the pass-through (Task 4) |
| `framed(win) -> boolean` · `FRAMED_SENTENCE` | `web/src/framed.ts` | the entry point's refusal to render inside a frame (Task 4) |
| `isReportOnlyDetectionLine(text, pageUrl, base) -> boolean` · `watch.reportOnlyForgiven` · the console handler reads a CSP line at any type (`CSP_VIOLATION`, module-private) | `web/e2e/harness.ts` | the live tier's one new forgiveness, counted; the Report-Only lines READ (Task 4) |
| the e2e build's `dist/headers.json` rewritten under `Content-Security-Policy` | `web/scripts/e2e-build.mjs` | the local tier served the policy ENFORCED (Task 4; decision D-9) |
| `id_cases.json` · `wire.idCases: readonly {raw: string; safe: boolean}[]` | `web/src/wire/`, emitted by `crates/yokohama-web/tests/wire_fixtures.rs`; `web/src/wire/contract.ts` | the store's verdicts over the id case list (Task 5) |
| `ID_MAX = 64` · `isSafeId(id) -> boolean` | `web/src/tableId.ts` | `store::is_safe_id` restated, pinned to `wire.idCases` (Task 5) |
| `REDIRECT_FILE = ".wrangler/deploy/config.json"` · the `workers_dev` rule under every door · `--config` on every deploy command | `worker/scripts/deploy-guard.mjs`, `worker/package.json` | C3 and D18 (Task 6) |
| `FORBIDDEN_SECRETS` · `judgeSecrets(listingJson) -> string[]` · `node scripts/secrets-guard.mjs --config <file>` | `worker/scripts/secrets-guard.mjs` | the deploy's secrets listing judged (Task 6) |
| `leaveNoSession(context, log, checks, stateDir)` in place of `await context.close()` | `web/scripts/pinned-browsers.mjs` (+ `pinned-browsers.d.mts`) | the instrument leaves no cookie, and still writes its verdict when the browser died first (Task 6) |
| the route-cell helpers `app_with(multiplayer, archive, dist)` (`lib.rs` tests :1728), `get_as`, `post_as`, `a_game`, `WHO`, `start_as`, `stream_frames`; the binary harness of `tests/start_refusal.rs` | `crates/yokohama-web` | what Task 3's cells reuse |
| the pool helpers `worker(path, sent: Sent)`, `person(email)`, `JSON_TYPE`, `CREATE_BODY`, `BODY_LIMIT`, `tableStub`, `call`, `socket` | `worker/test/worker.test.ts`, `worker/test/helpers.ts` | what Task 2's cells reuse |
| the node harness `boot(answer, hold, options)`, `worker.request(url, mode)`, `worker.asked`, `BODIES`, `ENTRIES` | `web/src/sw.node.test.ts` | what Task 4's cell extends (an `answer` that may return `headers`) |

## File structure

- Create `web/scripts/headers.mjs`, `web/scripts/headers.d.mts`, `web/scripts/headers.node.test.ts`; modify
  `web/vite.config.ts` (the stamp plugin `stampBuildValues` :82–94, its comment :62–73: `headers.json` written too),
  `web/scripts/precache.mjs` (:117 `NOT_PRECACHED`), `web/src/precache.node.test.ts` (the real-build cell: its own
  `NOT_PRECACHED` :63, its count :224 and two assertions; the three-build stamp cell :245–: the header reads) — Task 1.
- Modify `worker/scripts/assemble-assets.mjs` (`assembleInto`: `headers.json` skipped in the copy loop, the block
  appended after it; a missing `headers.json` refused; `writeFileSync` imported), `worker/scripts/deploy-guard.mjs`
  (`staleAssets` :479–499: `_headers` held to static + block; the import line; check 9's header comment),
  `worker/src/index.ts` (`handle` → `answer` + the wrapper), `worker/scripts/assemble-assets.test.mjs` (new cells; the
  committed-headers cell's names fence; the `unlisted` lists :88, :103 and `report.files` :82 UNCHANGED),
  `worker/scripts/deploy-guard.test.mjs` (the fixture trees carry `headers.json` and the block; the "static file that
  has changed" cell's sentence :482–494; new cells), `worker/test/worker.test.ts` (the header cells) — Task 2.
- Modify `crates/yokohama-web/src/assets.rs` (`response_headers`, `HEADERS_FILE`, its cell),
  `crates/yokohama-web/src/lib.rs` (`AppState.headers` beside `dist` :448; the base constructor; `with_dist` :653;
  `production` :501–513 reads the file; `no_store` :1185–1193 (doc :1177) → `response_headers`; `app()` ending
  :1231; the cell :3942–4010 extended; new cells), `crates/yokohama-web/src/main.rs` (the one stderr line after
  `AppState::new`, :180–186), `crates/yokohama-web/tests/start_refusal.rs` (one binary cell, three arms; `get`),
  `deploy/smoke-image.sh` (`door()` :102–131: the header reads) — Task 3.
- Modify `web/src/sw.ts` (`shellResponse` :417–419 and its call :385; `NOT_CARRIED`; the header comment's pinned
  list), `web/src/sw.node.test.ts` (`boot`'s `answer` :185–193 and the fake's `Response` :252–256; a cell),
  `web/src/main.tsx`, `web/scripts/e2e-build.mjs` (the enforcing override); create `web/src/framed.ts`,
  `web/src/framed.test.ts`, `web/e2e/framed.spec.ts`; modify `web/e2e/harness.ts` (`watchForBreakage` :83–130, its
  declared return type :86; `isReportOnlyDetectionLine`), `web/e2e/harness.node.test.ts` (the cells beside the
  beacon's — NOT `live.node.test.ts`, which does not mock the Playwright configuration and would fail to load where
  CI installs no Chromium), `web/e2e/two_browsers.spec.ts` :463–470, `web/e2e/joins_by_link.spec.ts` :541–545,
  `web/e2e/exit_game.spec.ts` :649–652 (the `forgiven` prints) — Task 4.
- Modify `crates/yokohama-web/tests/wire_fixtures.rs` (`id_cases()` beside `identity_cases()` :2764; `all.push`
  :2475; `CONSTRUCTED_WITH_AN_AT` :2964; a verdict cell), `web/src/wire/contract.ts` (:115, :445, :568, :957 —
  `idCases`), `web/src/route.ts` (`parseRoute` :45–63), `web/src/route.test.ts` (:27–43),
  `web/src/local/corpus.node.test.ts` (`NOT_SERVED` :155), `worker/test/corpus.test.ts` (`THIRD_COLUMN` :99); create
  `web/src/tableId.ts`, `web/src/tableId.test.ts`; the regenerated `web/src/wire/id_cases.json`
  (`worker/test/corpus.gen.ts` is generated on `pretest` and gitignored — never committed) — Task 5.
- Modify `worker/scripts/deploy-guard.mjs` (`doorRules` :398–429 and `ABSENT` :209–212; `configFailures` :605–647;
  `deployGuard` :651–703; the header comment's check 3 :19–36; the success sentence :732–739),
  `worker/scripts/deploy-guard.test.mjs` (:535 rewritten; new cells), `worker/package.json` (:18–20); create
  `worker/scripts/secrets-guard.mjs`, `worker/scripts/secrets-guard.test.mjs`; modify `web/scripts/pinned-browsers.mjs`
  (`leaveNoSession`, called in `run` :665; the header comment), `web/scripts/pinned-browsers.d.mts` (its declaration),
  `web/scripts/pinned-browsers.node.test.ts` (three cells), `deploy/Dockerfile.dockerignore` — Task 6.
- Modify `docs/web-lane.md` (§2, §4, §7, §10, §13, §15, the contract-file list :414, the wake window's recipe :79),
  `docs/plans/2026-10-08-security-remediation-spec.md` (§9.6 appended — never an edit above it), `worker/RUNBOOK.md`
  (§2's commands, the guard's list and the secrets guard's network note; §7's live-tier note and its wake-window
  recipe :803–804), `worker/README.md` (the deploy commands; the wake-window recipe :1718–1719), `deploy/README.md`
  (the headers, one paragraph; the build line's BuildKit sentence) — Task 7.
- Modify `Cargo.toml`, `Cargo.lock`, `web/package.json`, `web/package-lock.json`, `worker/package.json`,
  `worker/package-lock.json`, `web/src/wire/health.json`, `web/src/wire-script/archive-walk.json`,
  `web/src/wire-script/backup-envelope.json`, `deploy/Dockerfile.runtime:18`, `deploy/README.md`'s four example tags
  (:114, :135, :173, :211) — Task 8.
- The ledger, guidingpath, the board, `CLAUDE.md`, `docs/web-lane.md` §8 (the two readings the block takes) and
  this plan's §"Review record" — Task 9.

## Task order and why

Task 1 first: it owns the generator and `dist/headers.json`, which Tasks 2 and 3 consume. Tasks 2 (the Worker)
and 3 (the NAS) are independent of each other and both wait for Task 1. Task 4 (the client) waits for Task 1 too:
its node cell feeds the worker a `/shell.bin` that carries the set's names, and its e2e cell reads `X-Frame-Options`
off the local binary, which is Task 3's. Task 5 (the id grammar) touches the wire fixtures, the contract and the
router and the two corpus-replay lists — nothing the others touch. Task 6 (the tooling) touches the guard's door
rules, its redirect check and `package.json`, the instrument and the Docker ignore; Task 2 touches the SAME guard file
and its test (`staleAssets`, the import line, check 9's comment), so the two run one after the other, never in one
tree at once (Task 9 Step 3). Task 7 writes the documents the code changed. Task 8 bumps the version last, so the regenerated `server_version` fixtures read the
shipping head. Task 9 is the orchestrator's frame and the window.

---

### Task 1: The header set, generated once — `web/scripts/headers.mjs` and `dist/headers.json` — Opus (B4's static half, "the CSP"; rulings A1; decisions D-1, D-2, D-4; Review Focus 1)

**Files:**
- Create: `web/scripts/headers.mjs`, `web/scripts/headers.d.mts`, `web/scripts/headers.node.test.ts`
- Modify: `web/vite.config.ts` (the stamp plugin `stampBuildValues`, :82–94, and its comment :62–73),
  `web/scripts/precache.mjs` (:117 `NOT_PRECACHED`), `web/src/precache.node.test.ts` (the real-build cell and the
  three-build stamp cell)
- Test: `cd web && npm run test:wasm` (the node project runs `scripts/**/*.node.test.ts`), `npm run typecheck`

**Interfaces:**
- Consumes: `checkSiteUrl`, `checkReleaseFeed` (`web/scripts/build-values.mjs`) — the values `vite.config.ts`
  already computes; `web/public/404.html`.
- Produces: every name in §"Names shared across tasks"' first two rows; `dist/headers.json`.

- [ ] **Step 1: The failing cells.** `web/scripts/headers.node.test.ts`:

```ts
// **The header set both hosts serve, generated once** (security S2a; spec §2.2 B4 and "the CSP"; the c1 review's
// §"The header set to recommend"; decisions D-1, D-2, D-4). These cells pin the set's SHAPE — its names and their
// order, every directive and its sources, what the build values add, what the 404 page's style hashes to — so a
// directive loosened by hand, or a host added without its reason, fails here before either host serves it.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  CLOUDFLARE,
  CSP_HEADER,
  CSP_MODE,
  cspOf,
  headerSet,
  headersBlock,
  headersJson,
  parseHeadersJson,
  styleHashesOf,
} from "./headers.mjs";

/** No build value set, no inline style: the smallest policy. */
const NONE = { siteUrl: null, releaseFeed: null, styleHashes: [] as string[] };

/** A policy as `directive → its source list`, in the policy's own order. */
const directives = (csp: string): Map<string, string[]> =>
  new Map(
    csp.split("; ").map((d) => {
      const [name, ...sources] = d.split(" ");
      return [name!, sources];
    }),
  );

describe("the policy", () => {
  it("ships Report-Only in this release, under the mode's own header name (X2: the wrap flips it)", () => {
    expect(CSP_MODE).toBe("report-only");
    expect(headerSet(NONE)[0]![0]).toBe("Content-Security-Policy-Report-Only");
    expect(headerSet({ ...NONE, mode: "enforce" })[0]![0]).toBe("Content-Security-Policy");
    expect(CSP_HEADER).toEqual({ "report-only": "Content-Security-Policy-Report-Only", enforce: "Content-Security-Policy" });
    expect(() => headerSet({ ...NONE, mode: "report" as never })).toThrow(/report-only or enforce/);
  });

  it("names every directive once, in one order, with the sources the c1 review derived from the built bundle", () => {
    const d = directives(cspOf(NONE));
    expect([...d.keys()]).toEqual([
      "default-src", "script-src", "style-src", "img-src", "font-src", "connect-src", "frame-src",
      "worker-src", "manifest-src", "object-src", "base-uri", "form-action", "frame-ancestors",
    ]);
    expect(d.get("default-src")).toEqual(["'self'"]);
    // `'wasm-unsafe-eval'`: the module Worker compiles the engine (`assets/worker-*.js`).
    expect(d.get("script-src")).toEqual(["'self'", "'wasm-unsafe-eval'", ...CLOUDFLARE.script]);
    expect(d.get("style-src")).toEqual(["'self'"]);
    expect(d.get("img-src")).toEqual(["'self'"]);
    expect(d.get("font-src")).toEqual(["'self'"]);
    expect(d.get("connect-src")).toEqual(["'self'", ...CLOUDFLARE.connect]);
    expect(d.get("frame-src")).toEqual([...CLOUDFLARE.frame]);
    expect(d.get("worker-src")).toEqual(["'self'"]);
    expect(d.get("manifest-src")).toEqual(["'self'"]);
    expect(d.get("object-src")).toEqual(["'none'"]);
    expect(d.get("base-uri")).toEqual(["'none'"]);
    expect(d.get("form-action")).toEqual(["'self'"]);
    expect(d.get("frame-ancestors")).toEqual(["'none'"]);
  });

  it("never carries 'unsafe-inline' or 'unsafe-eval', whatever the values", () => {
    const full = {
      siteUrl: "https://yokohama.moore.nyc",
      releaseFeed: "https://api.github.com/repos/o/r/releases/latest",
      styleHashes: ["'sha256-abc='"],
    };
    // A QUOTED keyword: `'wasm-unsafe-eval'` is another keyword, and required — a bare substring test is red
    // against this very policy.
    for (const values of [NONE, full]) expect(cspOf(values)).not.toMatch(/'unsafe-(?:inline|eval)'/);
  });

  it("derives connect-src from the build values: the site's socket host, and the feed's ORIGIN, never its path", () => {
    const d = directives(
      cspOf({
        siteUrl: "https://yokohama.moore.nyc",
        releaseFeed: "https://api.github.com/repos/jmoore2333/yokohama-assistant/releases/latest",
        styleHashes: [],
      }),
    );
    expect(d.get("connect-src")).toEqual(["'self'", "wss://yokohama.moore.nyc", "https://api.github.com", ...CLOUDFLARE.connect]);
    const other = directives(cspOf({ siteUrl: null, releaseFeed: "https://feed.example.test/latest.json", styleHashes: [] }));
    expect(other.get("connect-src")).toEqual(["'self'", "https://feed.example.test", ...CLOUDFLARE.connect]);
    // `checkSiteUrl` admits a port, and the socket source keeps it (`.host`, not `.hostname`).
    const ported = directives(cspOf({ siteUrl: "https://yokohama.test:8443", releaseFeed: null, styleHashes: [] }));
    expect(ported.get("connect-src")).toEqual(["'self'", "wss://yokohama.test:8443", ...CLOUDFLARE.connect]);
  });

  it("names Cloudflare's three hosts, and no other host outside 'self' (decision D-2)", () => {
    expect(CLOUDFLARE).toEqual({
      script: ["https://challenges.cloudflare.com", "https://static.cloudflareinsights.com"],
      frame: ["https://challenges.cloudflare.com"],
      connect: ["https://cloudflareinsights.com"],
    });
    const hosts = [...cspOf(NONE).matchAll(/https:\/\/[a-z0-9.-]+/g)].map((m) => m[0]).sort();
    expect(hosts).toEqual([
      "https://challenges.cloudflare.com", "https://challenges.cloudflare.com",
      "https://cloudflareinsights.com", "https://static.cloudflareinsights.com",
    ]);
  });
});

describe("the inline style of the 404 page (decision D-4)", () => {
  it("hashes exactly the text between <style> and </style>, as a browser does, one hash per element", () => {
    const text = "\n  body { color: red }\n";
    const html = `<!doctype html><style>${text}</style><p>x</p>`;
    expect(styleHashesOf(html)).toEqual([`'sha256-${createHash("sha256").update(text, "utf8").digest("base64")}'`]);
    expect(styleHashesOf("<p>none</p>")).toEqual([]);
    expect(styleHashesOf("<style>a{}</style><style>b{}</style>")).toHaveLength(2);
    // The HTML parser normalizes CR and CRLF to LF before the element's text exists, so a browser hashes the LF
    // form whatever the file's line endings — an `autocrlf` checkout must hash to the same (measured in Chromium:
    // a CRLF page is styled under the LF hash and blocked under its own).
    expect(styleHashesOf("<style>\r\n  a{}\r\n</style>")).toEqual(styleHashesOf("<style>\n  a{}\n</style>"));
  });

  it("names the committed 404 page's one style block in style-src", () => {
    const page = readFileSync(new URL("../public/404.html", import.meta.url), "utf8");
    const hashes = styleHashesOf(page);
    expect(hashes, "web/public/404.html carries exactly one <style>").toHaveLength(1);
    expect(hashes[0]).toMatch(/^'sha256-[A-Za-z0-9+/]+=*'$/);
    expect(directives(cspOf({ ...NONE, styleHashes: hashes })).get("style-src")).toEqual(["'self'", hashes[0]]);
  });
});

describe("the set, its block and its file", () => {
  it("is six headers in one order, the five companions at the c1 review's values", () => {
    expect(headerSet(NONE).map(([name]) => name)).toEqual([
      "Content-Security-Policy-Report-Only", "X-Frame-Options", "X-Content-Type-Options",
      "Referrer-Policy", "Permissions-Policy", "Cross-Origin-Opener-Policy",
    ]);
    const values = Object.fromEntries(headerSet(NONE));
    expect(values["X-Frame-Options"]).toBe("DENY");
    expect(values["X-Content-Type-Options"]).toBe("nosniff");
    expect(values["Referrer-Policy"]).toBe("strict-origin-when-cross-origin");
    expect(values["Permissions-Policy"]).toBe("camera=(), microphone=(), geolocation=(), payment=(), usb=()");
    expect(values["Cross-Origin-Opener-Policy"]).toBe("same-origin");
  });

  it("renders the _headers block the asset layer parses: a /* line, then two-space-indented Name: value lines", () => {
    const lines = headersBlock(headerSet(NONE)).split("\n");
    expect(lines[0]).toBe("/*");
    expect(lines.slice(1, 7).every((l) => /^  [A-Za-z-]+: \S/.test(l))).toBe(true);
    expect(lines[1]).toBe(`  Content-Security-Policy-Report-Only: ${cspOf(NONE)}`);
    expect(lines.at(-1), "ends with one newline").toBe("");
    expect(lines).toHaveLength(8);
  });

  it("writes the file as ordered pairs and reads it back, and refuses any other shape by name", () => {
    const set = headerSet(NONE);
    expect(parseHeadersJson(headersJson(set))).toEqual(set);
    expect(headersJson(set).endsWith("\n")).toBe(true);
    for (const bad of [
      "{}", "[[1, 2]]", '[["X-Frame-Options"]]', '["X-Frame-Options: DENY"]', "nonsense", "[]",
      // What the Rust reader refuses, refused here too — one file must not pass the Worker's assembly and stop the
      // NAS at start: a name outside the token grammar; a value with a control character, which `headersBlock`
      // would render as a second `_headers` rule.
      '[["X Frame", "DENY"]]', '[["Bad Name:", "v"]]', '[["X-Frame-Options", "DE\\u0000NY"]]',
      '[["X-Note", "ok\\n  X-Frame-Options: ALLOWALL"]]',
      // Three more the Rust reader refuses — a pair of three; a lone surrogate (`JSON.parse` admits the escape,
      // `serde_json` does not); a name past `http`'s 65,535-byte bound — and the two this side alone refuses, a
      // tab in a value and an empty value (Task 1's review, measured against http 1.5.0 and serde_json 1.0.150).
      '[["X-A", "B", "C"]]', '[["X-A", "\\ud800"]]', `[["${"a".repeat(65_536)}", "v"]]`,
      '[["X-A", "a\\tb"]]', '[["X-A", ""]]',
    ]) {
      expect(() => parseHeadersJson(bad), bad).toThrow(/headers\.json/);
    }
  });
});
```

And in `web/src/precache.node.test.ts` (one import added: `parseHeadersJson` from `../scripts/headers.mjs`;
`createHash` is already imported), the real-build cell (the one that runs `vite build`; find it by its `vite build`
call): its own `NOT_PRECACHED` (:63 — the generator's exclusion restated, deliberately not imported) gains
`"headers.json"`, so the loop over it pins that the precache list the built `sw.js` carries names neither
`/headers.json` nor `/build-values.json`; the comment above the count reads "the list is the tree LESS the six"; the
count at :224 reads `toBe(3)` with the message `"exactly the three files the build emits and the release never lists
are left out"` (without these two the cell is red before any mutant — 153 entries against 154, measured); and two
assertions after the build: `dist/headers.json` exists, and `parseHeadersJson` reads six pairs whose first name is
`Content-Security-Policy-Report-Only`.

And the file's three-build stamp cell (`stamps which build values a build carries…`, :245) reads each build's
`headers.json` and its served `dist/404.html` beside the stamp — without it the build's WIRING (`styleHashes: []`,
`siteUrl: null`, `releaseFeed: null` in `vite.config.ts`'s call) passes every cell, and a release would ship an
unhashed 404 style or a `connect-src` without its socket host (measured: three survivors). `let lastCsp = "";` is
declared beside `const stampOf`, and after it `const connect = (): string[] => /connect-src ([^;]*)/.exec(lastCsp)![1]!.split(" ");`;
inside `stampOf`, after the stamp is read:

```ts
      // The build's header set, read off the SAME build: the served 404 page's style, hashed into it (after the
      // parser's newline normalization, as the generator hashes) — style-src EXACTLY `'self'` and that hash, and
      // no `'unsafe-…'` anywhere in the BUILT policy: a source appended in `vite.config.ts`'s call would pass a
      // `toContain` (Task 1's review, mutant M7e).
      const csp = parseHeadersJson(readFileSync(join(out, "headers.json"), "utf8"))[0]![1];
      const served = /<style\b[^>]*>([\s\S]*?)<\/style>/.exec(readFileSync(join(out, "404.html"), "utf8"))![1]!;
      const servedHash = `'sha256-${createHash("sha256").update(served.replace(/\r\n?/g, "\n"), "utf8").digest("base64")}'`;
      expect(/style-src ([^;]*)/.exec(csp)![1]!.split(" "), "the served 404 page's style, hashed, and nothing else").toEqual(["'self'", servedHash]);
      expect(csp, "the built policy carries no unsafe keyword").not.toMatch(/'unsafe-(?:inline|eval)'/);
      lastCsp = csp;
```

and after the first `stampOf` expectation ("every value set, the feed derived from the GitHub link"):
`expect(connect(), "the site's socket host and the derived feed's origin").toEqual(["'self'",
"wss://yokohama.example.org", "https://api.github.com", "https://cloudflareinsights.com"]);`; after the second ("the
same link with the feed off"): `expect(connect(), "no site, no feed").toEqual(["'self'", "https://cloudflareinsights.com"]);`.

- [ ] **Step 2: Run them to verify they fail.** `cd web && npx vitest run -c vitest.node.config.ts
scripts/headers.node.test.ts` — RED: the module does not exist. The real-build assertions RED: no `headers.json`.

- [ ] **Step 3: The code.** `web/scripts/headers.mjs`:

```js
// **The header set both hosts serve, generated ONCE where the build's values are known** (security S2a; spec §2.2
// B4, D13 and "the CSP"; the c1 review's §"The header set to recommend"; decision D-1).
//
// This file is the one place a directive is spelled. `vite build` calls `headerSet` with the checked build values
// (`vite.config.ts`) and writes the result to `dist/headers.json`; the Worker's assembly appends it to `_headers`
// as the `/*` block the asset layer applies to every static answer (`worker/scripts/assemble-assets.mjs`), and
// the NAS binary reads the file at start and sets every pair on every answer (`crates/yokohama-web/src/assets.rs`,
// `response_headers`). Neither host holds a copy of the policy's text, so the two cannot drift.
//
// **Why each source is there** was checked against the built bundle by the review: `'wasm-unsafe-eval'` because
// the module Worker compiles the engine with `WebAssembly.instantiateStreaming`; no `'unsafe-eval'` and no
// `'unsafe-inline'`, because the bundle holds no `eval` and the shell no inline script (its JSON-LD block is a
// data block, which `script-src` does not govern); `connect-src 'self'` covers `/api`, `/art`, the precache and
// the Turnstile pre-clearance's fetch to `/cdn-cgi/`; the site's `wss://` host is the Worker's socket (CSP3
// reads `'self'` as covering it, and the explicit source keeps older browsers honest); the release feed's ORIGIN
// is the door's one cross-origin fetch; the 404 page's inline style is admitted by its hash (D-4), so the page
// stays self-contained and the policy strict.
import { createHash } from "node:crypto";

/** The policy's mode, a CONSTANT and not a build value (decision D-1): `report-only` ships
 *  `Content-Security-Policy-Report-Only` — this release, 0.6.2, one release in the wild whose violations are read
 *  in the window and the Playwright tier (D-9) — and `enforce` ships `Content-Security-Policy`, which S2's wrap
 *  flips to, with the cell beside it. A build value would let a deploy weaken the policy without a commit. */
export const CSP_MODE = "report-only";

/** The two names the policy ships under, by mode. */
export const CSP_HEADER = Object.freeze({
  "report-only": "Content-Security-Policy-Report-Only",
  enforce: "Content-Security-Policy",
});

/** Cloudflare's hosts, named because both deployments sit behind the zone (decision D-2): `challenges` is
 *  Turnstile's widget and the Managed Challenge (a script and an iframe); `static.cloudflareinsights.com` is the
 *  Web Analytics beacon the zone injects into every HTML answer, kept on by the owner's standing rule, and
 *  `cloudflareinsights.com` is where that beacon posts. The one script this list cannot name is Bot Fight Mode's
 *  INLINE detection script, which carries no nonce a static shell could match: it is the known Report-Only
 *  line of this release, read in the window, and the enforcement decision is the wrap's. */
export const CLOUDFLARE = Object.freeze({
  script: Object.freeze(["https://challenges.cloudflare.com", "https://static.cloudflareinsights.com"]),
  frame: Object.freeze(["https://challenges.cloudflare.com"]),
  connect: Object.freeze(["https://cloudflareinsights.com"]),
});

/**
 * The CSP hash of every inline `<style>` in `html`, in document order: `'sha256-<base64>'` over EXACTLY the
 * element's text — the bytes between `<style…>` and `</style>`, the tags excluded — after the HTML parser's
 * newline normalization (CR and CRLF read as LF), which is what a browser hashes when it meets the element
 * (decision D-4; `web/public/404.html` is the one page that has one). The normalization is what keeps an
 * `autocrlf` checkout's build from hashing bytes no browser hashes (measured: a CRLF page is styled under the LF
 * hash and blocked under its own).
 *
 * @param {string} html
 * @returns {string[]}
 */
export function styleHashesOf(html) {
  return [...html.matchAll(/<style\b[^>]*>([\s\S]*?)<\/style>/g)].map(
    (m) => `'sha256-${createHash("sha256").update(m[1].replace(/\r\n?/g, "\n"), "utf8").digest("base64")}'`,
  );
}

/**
 * The policy's directives, in one order, as one header value.
 *
 * @param {{ siteUrl: string | null, releaseFeed: string | null, styleHashes: readonly string[] }} values
 *   `siteUrl` and `releaseFeed` as `build-values.mjs`'s checkers answer them (an origin; a feed URL or `null`).
 * @returns {string}
 */
export function cspOf({ siteUrl, releaseFeed, styleHashes }) {
  const connect = ["'self'"];
  if (siteUrl !== null) connect.push(`wss://${new URL(siteUrl).host}`);
  if (releaseFeed !== null) connect.push(new URL(releaseFeed).origin);
  connect.push(...CLOUDFLARE.connect);
  return [
    "default-src 'self'",
    `script-src 'self' 'wasm-unsafe-eval' ${CLOUDFLARE.script.join(" ")}`,
    ["style-src 'self'", ...styleHashes].join(" "),
    "img-src 'self'",
    "font-src 'self'",
    `connect-src ${connect.join(" ")}`,
    `frame-src ${CLOUDFLARE.frame.join(" ")}`,
    "worker-src 'self'",
    "manifest-src 'self'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'self'",
    "frame-ancestors 'none'",
  ].join("; ");
}

/**
 * The set, as `[name, value]` pairs in the order both hosts send them: the policy under its mode's name, then
 * the five companions — `X-Frame-Options: DENY` (enforced from this release, whatever the policy's mode: it is
 * what closes B4 while `frame-ancestors` is only reported), `nosniff` (the zone adds it at the edge for answers
 * the edge serves; the service worker's synthesized navigation needs it from the stored entry), the referrer
 * policy, a Permissions-Policy that turns off what the app never asks for, and COOP `same-origin` (the app opens
 * no popups; Access's login is a top-level redirect and Turnstile an iframe).
 *
 * @param {{ siteUrl: string | null, releaseFeed: string | null, styleHashes: readonly string[], mode?: string }} values
 * @returns {[string, string][]}
 */
export function headerSet({ siteUrl, releaseFeed, styleHashes, mode = CSP_MODE }) {
  if (!Object.hasOwn(CSP_HEADER, mode)) {
    throw new Error(`headers: the mode is report-only or enforce — got ${JSON.stringify(mode)}`);
  }
  return [
    [CSP_HEADER[mode], cspOf({ siteUrl, releaseFeed, styleHashes })],
    ["X-Frame-Options", "DENY"],
    ["X-Content-Type-Options", "nosniff"],
    ["Referrer-Policy", "strict-origin-when-cross-origin"],
    ["Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=(), usb=()"],
    ["Cross-Origin-Opener-Policy", "same-origin"],
  ];
}

/** The `/*` block `_headers` carries, in the asset layer's grammar: the path line, then each header on its own
 *  two-space-indented line, one trailing newline. Appended by the Worker's assembly, never written by hand.
 *
 *  @param {readonly (readonly [string, string])[]} set
 *  @returns {string} */
export function headersBlock(set) {
  return `${["/*", ...set.map(([name, value]) => `  ${name}: ${value}`)].join("\n")}\n`;
}

/** `dist/headers.json`'s text: the pairs, pretty-printed, one trailing newline.
 *  @param {readonly (readonly [string, string])[]} set
 *  @returns {string} */
export function headersJson(set) {
  return `${JSON.stringify(set, null, 2)}\n`;
}

/** A header name: RFC 9110's token grammar, which is what `http::HeaderName` accepts on the NAS. */
const HEADER_NAME = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;

/** `http::HeaderName`'s length bound (`MAX_HEADER_NAME_LEN`, 65,535 bytes — a token is ASCII, so characters):
 *  a longer name would pass the assembly and stop the NAS at start. */
const HEADER_NAME_MAX = 65_535;

/** A control character in a value (`\u0000`–`\u001f`, `\u007f`): what would let one pair render as two
 *  `_headers` lines. Stricter than `http::HeaderValue`, which admits a tab (`http` 1.5.0's `is_valid`:
 *  `b >= 32 && b != 127 || b == b'\t'`) — the strict side is this one, deliberately (see `parseHeadersJson`). */
const CONTROL = /[\u0000-\u001f\u007f]/;

/** `dist/headers.json` read back: an array of at least one `[string, string]` pair — each name a token of at
 *  most 65,535 bytes, each value non-empty and free of control characters, every string well-formed — or a throw
 *  naming the file. The Worker's assembly and the guard read it through this, as the NAS's `response_headers`
 *  reads it in Rust (`serde_json`, then `http::HeaderName::from_bytes` and `HeaderValue::from_str`), and this
 *  side refuses EVERYTHING that side refuses — the same name grammar and its length bound; a lone surrogate,
 *  whose escape `JSON.parse` admits and `serde_json` does not; a pair of any other arity — and two value shapes
 *  `http` admits besides, a tab and an empty value: so a file that passes the assembly always passes the NAS's
 *  reader at start, and the one-host-down release (passed the assembly, stopped the NAS) cannot arise. Measured
 *  by Task 1's review against `http` 1.5.0 and `serde_json` 1.0.150; the build writes none of these shapes.
 *  @param {string} text
 *  @returns {[string, string][]} */
export function parseHeadersJson(text) {
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error("headers.json does not parse");
  }
  const pair = (p) =>
    Array.isArray(p) &&
    p.length === 2 &&
    p.every((s) => typeof s === "string" && s !== "" && s.isWellFormed()) &&
    p[0].length <= HEADER_NAME_MAX &&
    HEADER_NAME.test(p[0]) &&
    !CONTROL.test(p[1]);
  if (!Array.isArray(parsed) || parsed.length === 0 || !parsed.every(pair)) {
    throw new Error(
      "headers.json is not the client build's header set — an array of [name, value] pairs, each name a token, each value free of control characters",
    );
  }
  return parsed.map(([name, value]) => [name, value]);
}
```

`web/scripts/headers.d.mts` (the node project type-checks `vite.config.ts`'s imports, as `build-values.d.mts` does):

```ts
export declare const CSP_MODE: "report-only" | "enforce";
export declare const CSP_HEADER: Readonly<Record<"report-only" | "enforce", string>>;
export declare const CLOUDFLARE: Readonly<{ script: readonly string[]; frame: readonly string[]; connect: readonly string[] }>;
export interface HeaderValues {
  siteUrl: string | null;
  releaseFeed: string | null;
  styleHashes: readonly string[];
}
export declare function styleHashesOf(html: string): string[];
export declare function cspOf(values: HeaderValues): string;
export declare function headerSet(values: HeaderValues & { mode?: "report-only" | "enforce" }): [string, string][];
export declare function headersBlock(set: readonly (readonly [string, string])[]): string;
export declare function headersJson(set: readonly (readonly [string, string])[]): string;
export declare function parseHeadersJson(text: string): [string, string][];
```

`web/vite.config.ts`: import `headerSet`, `headersJson`, `styleHashesOf` from `./scripts/headers.mjs`; beside the
`stamp` constant (:75–81; the plugin `stampBuildValues` is :82–94):

```ts
// **The header set** (security S2a, decision D-1): the policy and its five companions, from the SAME checked
// locals the `define` block reads — the site's socket host and the feed's origin in `connect-src`, the 404
// page's inline style hashed into `style-src` — written to `dist/headers.json` beside the stamp. The Worker's
// assembly appends it to `_headers`; the NAS reads it at start. Neither host spells a directive of its own.
const headers = headerSet({
  siteUrl,
  releaseFeed,
  styleHashes: styleHashesOf(readFileSync(new URL("./public/404.html", import.meta.url), "utf8")),
});
```

and in the stamp plugin's `writeBundle`, after the stamp: `writeFileSync(join(outDir, "headers.json"),
headersJson(headers));` — the plugin's name and comment say it writes both files.

`web/scripts/precache.mjs:117`: `NOT_PRECACHED` gains `"headers.json"`, with one clause in the comment above it:
the header set's file is read by the hosts, never by a browser, and a browser that installed it would install
nothing it uses.

- [ ] **Step 4: Run the cells, the build and the suites.** `cd web && npm run typecheck && npm run test:wasm`
(the new file and the real-build cell green) `&& npm test -- --run`; then `npm run build` (the client, so Tasks 2
and 3's dists carry `headers.json`); `npm run e2e` — the local binary serves no policy yet (Task 3), so the walk
reads as before; recorded.

- [ ] **Step 5: The mutants** (each red at least at the cell it names, every other red cell recorded — M1 reds the
first, the block and the set cells together). **M1** `CSP_MODE = "enforce"` — the first cell red; **M2**
`'unsafe-inline'` appended to `style-src` — the no-unsafe cell red (the directive and 404-style cells with it);
**M2b** `'unsafe-eval'` added to `script-src` — the no-unsafe and directive cells red; **M3** the feed's whole URL
in `connect-src` instead of its origin — the derivation cell red; **M3b** `.hostname` for `.host` in the socket
source — the derivation cell's port row red; **M4** `styleHashesOf` hashing the whole element, tags included — the
hash cell red; **M4b** the CR normalization dropped — the hash cell's CRLF row red; **M5** `headersJson` writing an
object keyed by name — the round-trip cell red (and the order lost); **M5b** `parseHeadersJson` accepting a name
with a space, or a value with a newline — the round-trip cell's bad rows red; **M5c** `p.length >= 2`, `isWellFormed`
dropped, or the name bound dropped — the round-trip cell's three Rust-refused rows red, one each; **M5d** a tab or an empty
value admitted — its two rows red; **M6** `headers.json` dropped from
`NOT_PRECACHED` in the generator — the real-build cell red at `toEqual(entries)` (the list names it); **M7**
`writeBundle` not writing `headers.json` — the real-build cell red ("the build emits the header set"); **M7b**
`styleHashes: []`, **M7c** `siteUrl: null`, **M7d** `releaseFeed: null` in `vite.config.ts`'s call — the stamp
cell red at each (the 404 hash; the socket host; the feed origin); **M7e** `"'unsafe-inline'"` appended to `styleHashes` in
that call — the stamp cell red at the built policy (Task 1's review: a `toContain` passed it).

- [ ] **Step 6: Commit.** `git add web/scripts/headers.mjs web/scripts/headers.d.mts web/scripts/headers.node.test.ts
web/vite.config.ts web/scripts/precache.mjs web/src/precache.node.test.ts` · `git commit -m "web: the header set
both hosts serve, generated once at the client build from the checked build values into dist/headers.json — the
c1 review's policy in Report-Only, the 404 page's style hashed, Cloudflare's three hosts named (S2a Task 1)"`.

---

### Task 2: The Worker's half — the `/*` block appended to `_headers`, the guard holding it, `no-store` and `nosniff` on every `/api` answer — Opus (B4's static half on the Worker; B9's Worker half, DC-F4; ruling A2; decisions D-1, D-3; Review Focus 3)

**Files:**
- Modify: `worker/scripts/assemble-assets.mjs` (`assembleInto`: one line in the copy loop, the block after it; a
  new export `assembledHeaders`; `writeFileSync` in the `node:fs` import :28), `worker/scripts/deploy-guard.mjs`
  (`staleAssets` :479–499; the import line; check 9's header comment — and nothing Task 6 touches),
  `worker/src/index.ts` (`handle` becomes `answer`; `API_HEADERS`, `isApiPath`, `withApiHeaders`, the new `handle`),
  `worker/scripts/assemble-assets.test.mjs` (new cells; the committed-headers cell; the `unlisted` expectations :88
  and :103 and `report.files` :82 UNCHANGED), `worker/scripts/deploy-guard.test.mjs` (the fixture trees; the
  "static file that has changed" cell :482–494; new cells), `worker/test/worker.test.ts` (the header cells)
- Test: `cd worker && npm run typecheck && npm run test:scripts && npm test`

**Interfaces:**
- Consumes: `headersBlock`, `parseHeadersJson` (`web/scripts/headers.mjs`, Task 1) — imported as
  `../../web/scripts/headers.mjs` (the two packages already import across the tree: `web/e2e/door_unmetered.spec.ts`
  reads `worker/scripts/precache-list.mjs`); `web/dist/headers.json`.
- Produces: the `/*` block of `worker/assets/_headers`; `API_HEADERS`, `isApiPath`, `withApiHeaders`.

- [ ] **Step 1: The failing cells.**

(a) `worker/scripts/assemble-assets.test.mjs` — the synthetic `dist` trees of its cells gain `"headers.json":
headersJson(headerSet({ siteUrl: null, releaseFeed: null, styleHashes: [] }))` (import both from
`../../web/scripts/headers.mjs`), and:

```js
  it("appends the header set's block to _headers: the static text, then /* with the pairs of web/dist/headers.json — and ships the file itself to no one", () => {
    const out = join(tree({}), "assets");
    const set = headerSet({ siteUrl: "https://yokohama.test", releaseFeed: null, styleHashes: [] });
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES), "headers.json": headersJson(set) });
    const statics = tree({ _headers: "/sw.js\n  Cache-Control: no-store\n", _redirects: "" });
    assemble({ ...sources(dist), statics, out });
    const text = readFileSync(join(out, "_headers"), "utf8");
    expect(text.startsWith("/sw.js\n  Cache-Control: no-store\n"), "the static text first, byte for byte").toBe(true);
    expect(text).toBe(assembledHeaders("/sw.js\n  Cache-Control: no-store\n", headersJson(set)));
    const block = rules(text).get("/*");
    expect(Object.keys(block)).toEqual(set.map(([name]) => name.toLowerCase()));
    expect(block["x-frame-options"]).toBe("DENY");
    expect(block["content-security-policy-report-only"]).toContain("wss://yokohama.test");
    // An INPUT of the assembly, never an asset: the zone's WAF edge rule is derived from the tree's top level
    // (`edge-rule.test.mjs`'s drift cell), and a browser gains nothing from the file — its pairs ride every answer.
    expect(existsSync(join(out, "headers.json")), "headers.json is not copied into the assets tree").toBe(false);
  });

  it("refuses a dist without headers.json, naming the file, and leaves no tree behind", () => {
    const out = join(tree({}), "assets");
    const dist = tree({ "index.html": "<!doctype html>", "sw.js": swWith(ENTRIES) });
    expect(() => assemble({ ...sources(dist), out })).toThrow(/web\/dist\/headers\.json|headers\.json is absent/);
    expect(existsSync(out)).toBe(false);
  });
```

(`rules()` is the committed-headers describe's parser — lift it to the file's top so both describes read it; `out`
is declared as every existing cell declares it.) The `unlisted` expectations at :88 and :103, `report.files` (:82)
and the byte list KEEP their values: the assembly reads `headers.json` and ships its block, never the file. The
committed-headers cell gains one assertion — no rule of the static file names a header of the generated set: two
rules naming one header send it twice (measured in workerd), and one rule naming it twice joins the values with a
comma:

```js
    const generated = headerSet({ siteUrl: null, releaseFeed: null, styleHashes: [] }).map(([name]) => name.toLowerCase());
    for (const [path, rule] of rules(committed("../static/_headers"))) {
      for (const name of Object.keys(rule)) expect(generated, `${path} names ${name}, which the generated /* block carries`).not.toContain(name);
    }
```

(b) `worker/scripts/deploy-guard.test.mjs` — the fixture builder `worker(changes, since)` (a passing package laid
in a scratch repository; `join(root, "..")` is that repository; `naming(...)` the file's matcher) changes in three
places: `"static/_headers"` becomes a PATH block, `"/sw.js\n  Cache-Control: no-store\n"` (the committed file's
shape — the `/*` block is the assembly's, never the static file's); `"../web/dist/headers.json"` is laid as
`headersJson(headerSet({ siteUrl: null, releaseFeed: null, styleHashes: [] }))` (both imported from
`../../web/scripts/headers.mjs`); and `"assets/_headers"` is `assembledHeaders(<that static text>, <that json>)`
(imported from `./assemble-assets.mjs`) — so "passes a release module, a verified tree and a production
configuration" still passes; and the existing cell "refuses a static file that has changed since the assembly"
(:482–494) now expects, for its `_headers` row, `worker/assets/_headers is not worker/static/_headers with the
current web/dist/headers.json's block — assemble the assets again: npm run build`. Then:

```js
  it("holds the assembled _headers to worker/static/_headers plus the current web/dist/headers.json's block (security S2a)", () => {
    // The block missing: the static text copied as it was, as the assembly did before this cycle.
    const stale = worker({}, { "assets/_headers": "/sw.js\n  Cache-Control: no-store\n" });
    expect(deployGuard(stale, join(stale, ".."))).toEqual(
      naming(/_headers is not worker\/static\/_headers with the current web\/dist\/headers\.json/),
    );
    // The build moved: web/dist/headers.json now names a socket host the assembled block does not.
    const moved = worker({}, {
      "../web/dist/headers.json": headersJson(headerSet({ siteUrl: "https://moved.test", releaseFeed: null, styleHashes: [] })),
    });
    expect(deployGuard(moved, join(moved, ".."))).toEqual(naming(/_headers is not worker\/static\/_headers with the current/));
  });

  it("refuses when web/dist/headers.json is absent, naming it", () => {
    const root = worker({}, { "../web/dist/headers.json": null });
    expect(deployGuard(root, join(root, ".."))).toEqual(naming(/web\/dist\/headers\.json is absent/));
  });
```

(c) `worker/test/worker.test.ts`, a new describe:

```ts
describe("every answer under /api carries no-store and nosniff (security S2a, B9's Worker half; the S1 stack review's DC-F4)", () => {
  // `_headers` never applies to a Worker-generated answer (Cloudflare's rule), so the shell sets the two headers
  // a JSON body needs on the way out — the shell's own answers and the objects' alike (decision D-3).
  it("on the shell's own answers and the house's; never on /enter or a path outside /api", async () => {
    const rows: [string, Sent][] = [
      ["/api/health", {}],
      ["/api/me", {}],
      ["/api/art", {}],
      ["/api/art", { method: "HEAD" }],
      ["/api/nothing", {}],
      ["/api/admin/tables", {}],
      ["/api/games", { method: "POST", headers: JSON_TYPE, body: new Uint8Array(BODY_LIMIT + 1).fill(97) }],
      ["/api/games/x", { method: "DELETE" }],
    ];
    for (const [path, init] of rows) {
      const res = await worker(path, init);
      expect(
        [res.headers.get("cache-control"), res.headers.get("x-content-type-options")],
        `${init.method ?? "GET"} ${path} answered ${res.status}`,
      ).toEqual(["no-store", "nosniff"]);
    }
    for (const path of ["/enter", "/t/some-table", "/assets/x.js", "/apiary"]) {
      const res = await worker(path);
      expect(res.headers.get("cache-control"), `${path} is not the API's`).toBeNull();
    }
  });

  it("on a table object's answer through the Worker, signed in", async () => {
    // The pool's person, as the identity cells mint one; a create, then the table's frame.
    const token = await person(RED);
    const created = await worker("/api/games", { method: "POST", token, headers: JSON_TYPE, body: CREATE_BODY });
    expect(created.status).toBe(201);
    const id = (JSON.parse(created.text) as { id: string }).id;
    const frame = await worker(`/api/games/${id}/table`, { token });
    expect(frame.status).toBe(200);
    expect([frame.headers.get("cache-control"), frame.headers.get("x-content-type-options")]).toEqual(["no-store", "nosniff"]);
  });
});
```

(`worker(path, sent: Sent)` is the file's own driver — the rows are typed `Sent`, not `RequestInit`, or `npm run
typecheck` fails (TS2345, measured) — and `person(email)` its minted assertion, as the identity cells use them; the
table's document is `…/table`, since a bare `/api/games/<id>` on a table `CREATE_BODY` leaves open answers 409. The
existing upgrade cell — "an upgrade to a table's stream forwards, and the object's 101 comes back with
its first frame" — is the 101's catcher: a wrapper that rebuilt a 101 would throw in workerd, since a `Response`
cannot be constructed with that status.)

- [ ] **Step 2: Run them to verify they fail.** `cd worker && npm run test:scripts` — (a) RED (no block; the
missing file tolerated), (b) RED; `npm test` — (c) RED on `/api/health`'s first row.

- [ ] **Step 3: The code.**

`worker/scripts/assemble-assets.mjs`: import `headersBlock` and `parseHeadersJson` from
`../../web/scripts/headers.mjs`, and `writeFileSync` joins `copyFileSync` in the `node:fs` import (:28); a new export
beside `verifyTree`:

```js
/**
 * `worker/assets/_headers` as the assembly writes it (security S2a, decision D-1): the committed static text
 * (`worker/static/_headers`, the cache rules) followed by the header set's `/*` block, generated from
 * `web/dist/headers.json`. ONE function, so the deploy guard's check 9 recomputes exactly what the assembly
 * wrote. The static file must not carry a `/*` block of its own: a header applied twice has its values joined
 * with a comma.
 */
export function assembledHeaders(staticText, headersJsonText) {
  const set = parseHeadersJson(headersJsonText);
  const base = staticText === "" || staticText.endsWith("\n") ? staticText : `${staticText}\n`;
  return (
    `${base}\n# The header set both hosts serve (security S2a) — GENERATED from web/dist/headers.json by\n` +
    `# scripts/assemble-assets.mjs; the source is web/scripts/headers.mjs. Do not edit.\n${headersBlock(set)}`
  );
}
```

and in `assembleInto`'s copy loop (:107–118), one line before `const target = under + file;`:

```js
      // An INPUT of the assembly, never an asset (security S2a, decision D-1): its pairs become the /* block below.
      if (source === "web/dist" && file === "headers.json") continue;
```

then after the copy loop and before `walk(out)`:

```js
  // **The header set's block** (security S2a, decision D-1): appended to the copied `_headers`, from the client
  // build's `headers.json`. A dist without the file is not a build of 0.6.2 or later, and a tree without the
  // block would ship a release with no policy: refused, and `assemble` leaves no tree behind.
  const generated = join(dist, "headers.json");
  if (!existsSync(generated)) {
    throw new Error(`${dist}/headers.json is absent — the client build is older than 0.6.2, or not a build: cd web && npm run build`);
  }
  const target = join(out, "_headers");
  const statik = existsSync(target) ? readFileSync(target, "utf8") : "";
  writeFileSync(target, assembledHeaders(statik, readFileSync(generated, "utf8")));
```

`worker/scripts/deploy-guard.mjs`, `staleAssets` (:479–499): the loop over `worker/static`'s files compares every
file byte for byte but `_headers`, which is compared with `assembledHeaders(the static text, web/dist/headers.json's
text)`; `web/dist/headers.json` absent is its own failure:

```js
  const generated = join(repo, "web", "dist", "headers.json");
  const statics = join(root, "static");
  for (const file of existsSync(statics) ? readdirSync(statics).sort() : []) {
    const copy = join(tree, file);
    const source = readFileSync(join(statics, file));
    if (file === "_headers") {
      if (!existsSync(generated)) {
        failures.push(`web/dist/headers.json is absent, so nothing says which header set worker/${shown}/_headers should carry — ${REBUILD}`);
        continue;
      }
      const want = assembledHeaders(source.toString("utf8"), readFileSync(generated, "utf8"));
      if (!existsSync(copy) || readFileSync(copy, "utf8") !== want) {
        failures.push(`worker/${shown}/_headers is not worker/static/_headers with the current web/dist/headers.json's block — assemble the assets again: npm run build`);
      }
      continue;
    }
    if (!existsSync(copy) || !readFileSync(copy).equals(source)) {
      failures.push(`worker/${shown}/${file} is not worker/static/${file} — assemble the assets again: npm run build`);
    }
  }
```

(`assembledHeaders` imported from `./assemble-assets.mjs`, beside `precacheEntriesFrom` and `verifyTree`. Check
9's header comment in the file's top block gains one sentence.)

`worker/src/index.ts`: the current `handle` is renamed `answer` (its doc comment stays above it, its first line
amended: "**The Worker** … in this order —, wrapped by [`handle`]"), and above it:

```ts
/** The two headers every answer under `/api` carries (security S2a, finding B9's Worker half; the S1 stack
 *  review's DC-F4). A `_headers` block never applies to a Worker-generated answer — Cloudflare's rule for static
 *  assets — so the shell sets them itself, ON THE WAY OUT, for its own answers and the objects' alike:
 *  `no-store`, because the zone caches by file extension and a table named to look like a file would be a
 *  private frame kept at the edge (the NAS's one `Router::layer` has said the same since S1c); `nosniff`, so an
 *  API body is never read as a script. The whole set the static release carries (`_headers`' `/*` block) buys
 *  nothing on a JSON body, and the shell holds no copy of it (decision D-3). */
export const API_HEADERS: ReadonlyArray<readonly [string, string]> = [
  ["Cache-Control", "no-store"],
  ["X-Content-Type-Options", "nosniff"],
];

/** Whether `pathname` is the API's — `/api` exactly, or under `/api/` — and never `startsWith("/api")`, which
 *  would claim `/apiary` (the NAS's layer and `swRoute.ts` draw the same line). */
export function isApiPath(pathname: string): boolean {
  return pathname === "/api" || pathname.startsWith("/api/");
}

/** `res` with [`API_HEADERS`] set — a fresh `Response` over the same body and status, since an object's answer
 *  may be immutable — or `res` ITSELF for a 101, which carries its socket, cannot be rebuilt (the constructor
 *  refuses the status) and is sent nothing a cache could keep. */
export function withApiHeaders(res: Response): Response {
  if (res.status === 101) return res;
  const headers = new Headers(res.headers);
  for (const [name, value] of API_HEADERS) headers.set(name, value);
  return new Response(res.body, { status: res.status, statusText: res.statusText, headers });
}

/** The Worker's entry: [`answer`]'s reply, with the API's two headers on every answer under `/api`. */
export async function handle(req: Request, env: Env, ctx: ExecutionContext, keys: KeySource): Promise<Response> {
  const res = await answer(req, env, ctx, keys);
  return isApiPath(new URL(req.url).pathname) ? withApiHeaders(res) : res;
}
```

- [ ] **Step 4: Run the cells and the lane's rows**; `cd web && npm run build && cd ../worker && npm run build`
(the assembly now appends the block; the smoke passes); the guard over the real tree: `node
scripts/deploy-guard.mjs` passes and its door line prints.

- [ ] **Step 5: The mutants.** **M8** `withApiHeaders` dropped from `handle` — (c)'s first row red; **M9**
`isApiPath` as `startsWith("/api")` — (c)'s `/apiary` row red; **M10** the 101 not excepted — the upgrade cell
red (workerd refuses the constructor: `RangeError: Responses may only be constructed with status codes in the range
200 to 599` — measured); **M11** the block not appended at assembly — (a)'s first cell red (the guard's cells lay
their trees with `assembledHeaders` themselves, so they stay green); **M12** `staleAssets` comparing `_headers` to
the static text alone — (b)'s first cell red, and every `deployGuard(worker())` cell with it (about fifty); **M13**
a missing `headers.json` tolerated at assembly — (a)'s second cell red; **M13b** `headers.json` copied into the
assets tree — (a)'s first cell red at the file's presence, and the two assembly cells whose `report.files`/`report.unlisted`
lists name every file in the tree ("lays out dist…", "lands the 404 page…") with it; `edge-rule.test.mjs`'s drift cell
reads the COMMITTED tree, so it catches the mutant only after a real `npm run build` (the pre-flight's reading, 2026-10-11).

- [ ] **Step 6: Commit.** `git add worker/scripts/assemble-assets.mjs worker/scripts/assemble-assets.test.mjs
worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs worker/src/index.ts worker/test/worker.test.ts`
· `git commit -m "worker: the header set's /* block appended to _headers from web/dist/headers.json and held by
the guard; every answer under /api carries no-store and nosniff from handle, a 101 as it came (S2a Task 2)"`.

---

### Task 3: The NAS's half — `dist/headers.json` read at start, one layer setting the set on every answer — Opus (B4's static half on the NAS; decision D-1; Review Focus 1)

**Files:**
- Modify: `crates/yokohama-web/src/assets.rs` (`response_headers`, `HEADERS_FILE`, its cell),
  `crates/yokohama-web/src/lib.rs` (`AppState.headers` beside `dist` :448; the constructors' base; `with_dist` :653;
  `production` :501–513; `no_store` :1185–1193 (its doc :1177) → `response_headers`; `app()` ending :1231; the cell
  :3942–4010; new cells), `crates/yokohama-web/src/main.rs` (one stderr line after `AppState::new`, :180–186),
  `crates/yokohama-web/tests/start_refusal.rs` (one binary cell, three arms; a `get` helper), `deploy/smoke-image.sh`
  (`door()` :102–131)
- Test: `cargo test -p yokohama-web` (the lib and `start_refusal`), `cargo clippy --workspace --all-targets -- -D
  warnings`

**Interfaces:**
- Consumes: `dist/headers.json` (Task 1's shape: an array of `[name, value]` pairs).
- Produces: `assets::response_headers`, `assets::HEADERS_FILE`, `AppState.headers`, `AppState::with_headers`, the
  layer `response_headers` (the former `no_store`, renamed — `no_store` no longer exists).

- [ ] **Step 1: The failing cells.**

(a) `assets.rs`'s `mod tests`:

```rust
    /// **The header set is the client build's file, read in order** (security S2a, decision D-1): an array of
    /// `[name, value]` pairs becomes header pairs in that order; an ABSENT file is an empty set (a dev run with
    /// an older client build still serves; `main` says so once); a file of any other shape is refused by name,
    /// because a build that wrote the wrong thing must not be served without its policy in silence.
    #[test]
    fn response_headers_reads_the_pairs_in_order_and_refuses_any_other_shape() {
        // The file's own scratch idiom (:419, :518, :667): `tempfile` is not a dependency, and the owner's
        // dependency fence keeps it so.
        let dir = std::env::temp_dir().join(format!("yokohama-headers-{}-a", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch dist");
        assert_eq!(response_headers(&dir).expect("absent is empty"), vec![]);
        std::fs::write(
            dir.join(HEADERS_FILE),
            r#"[["Content-Security-Policy-Report-Only", "default-src 'self'"], ["X-Frame-Options", "DENY"]]"#,
        )
        .unwrap();
        let read = response_headers(&dir).expect("the pairs");
        assert_eq!(read.len(), 2);
        assert_eq!(read[0].0.as_str(), "content-security-policy-report-only");
        assert_eq!(read[0].1.to_str().unwrap(), "default-src 'self'");
        assert_eq!(read[1].0.as_str(), "x-frame-options");
        for (bad, why) in [
            ("{}", "an object"),
            ("[]", "an empty list"),
            (r#"[["X-Frame-Options"]]"#, "a one-member pair"),
            (r#"[["X Frame", "DENY"]]"#, "a name with a space"),
            ("[[\"X-Frame-Options\", \"DE\\u0000NY\"]]", "a value with a control character"),
            ("nonsense", "not JSON"),
        ] {
            std::fs::write(dir.join(HEADERS_FILE), bad).unwrap();
            let err = response_headers(&dir).expect_err(why);
            assert!(err.contains("headers.json"), "{why}: the refusal names the file: {err}");
        }
    }
```

(b) `lib.rs`'s `mod tests`, beside `every_answer_under_api_is_no_store_and_the_static_release_is_not` (:3942):

```rust
    /// **Every answer carries the header set `dist/headers.json` holds** (security S2a, finding B4's static half,
    /// D13's host half; decision D-1): the shell, `/shell.bin`, a fingerprinted asset, the 404 page a miss with an
    /// extension is answered with, the SPA fallback, `/enter`'s 302, and every answer under `/api` — a 200, a
    /// 401, a 404 and a stream — all read every pair, and the `/api` rows read `no-store` beside them, from ONE
    /// layer, so no answer is the exception a reviewer finds. The dist is a scratch tree, so the pairs here are
    /// the cell's own two and not the build's six: what is pinned is that the file's pairs reach every answer.
    #[tokio::test]
    async fn every_answer_carries_the_dist_header_set_and_the_api_rows_no_store_beside_it() {
        let dist = std::env::temp_dir().join(format!("yokohama-headers-{}-b", std::process::id()));
        let _ = std::fs::remove_dir_all(&dist);
        std::fs::create_dir_all(&dist).expect("a scratch dist");
        for (name, body) in [
            ("index.html", "<!doctype html><div id=root></div>"),
            ("shell.bin", "<!doctype html><div id=root></div>"),
            ("404.html", "<!doctype html><title>Nothing here</title>"),
        ] {
            std::fs::write(dist.join(name), body).unwrap();
        }
        std::fs::create_dir(dist.join("assets")).unwrap();
        std::fs::write(dist.join("assets/app-abc.js"), "console.log(1)").unwrap();
        std::fs::write(
            dist.join(assets::HEADERS_FILE),
            r#"[["X-Frame-Options", "DENY"], ["Content-Security-Policy-Report-Only", "default-src 'self'"]]"#,
        )
        .unwrap();
        let router = app_with(true, false, &dist).await;
        a_game(&router, "x", &[]).await;
        let rows: &[(&str, &str, bool)] = &[
            ("GET", "/", false),
            ("HEAD", "/", false),
            ("GET", "/shell.bin", false),
            ("GET", "/assets/app-abc.js", false),
            ("GET", "/assets/missing-x.js", false),
            ("GET", "/t/anything", false),
            ("GET", "/enter", false),
            ("GET", "/api/health", true),
            ("GET", "/api/me", true),
            ("GET", "/api/nothing", true),
            ("GET", "/api/games", true),
            ("GET", "/api/games/x", true),
            ("GET", "/api/games/x/stream", true),
        ];
        for (method, path, api) in rows {
            let mut req = Request::builder().method(*method).uri(*path);
            if *api {
                req = req.header(identity::CLAIMED_HEADER, WHO);
            }
            let res = router.clone().oneshot(req.body(Body::empty()).unwrap()).await.expect("the router answers");
            let h = res.headers();
            assert_eq!(h.get("x-frame-options").and_then(|v| v.to_str().ok()), Some("DENY"), "{method} {path} answered {}", res.status());
            assert_eq!(
                h.get("content-security-policy-report-only").and_then(|v| v.to_str().ok()),
                Some("default-src 'self'"),
                "{method} {path}"
            );
            if *api {
                assert_eq!(h.get(http::header::CACHE_CONTROL).and_then(|v| v.to_str().ok()), Some("no-store"), "{method} {path}");
            }
        }
    }

    /// **A state built without a dist serves no security header, and `with_headers` is how a cell gives it
    /// one** — the mutant that reads a set from anywhere but the state fails the second half.
    #[tokio::test]
    async fn an_in_memory_state_serves_no_header_set_and_with_headers_gives_it_one() {
        let bare = app(Arc::new(AppState::in_memory()));
        let res = bare.oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap()).await.unwrap();
        assert!(res.headers().get("x-frame-options").is_none(), "no dist, no set");
        let given = app(Arc::new(AppState::in_memory().with_headers(vec![(
            http::HeaderName::from_static("x-frame-options"),
            http::HeaderValue::from_static("DENY"),
        )])));
        let res = given.oneshot(Request::builder().uri("/api/health").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(res.headers().get("x-frame-options").and_then(|v| v.to_str().ok()), Some("DENY"));
    }
```

(The stream row: `oneshot` returns once the headers are written and the body is dropped unread — the S1c cell's
own note; the implementer confirms the row returns. `GET /api/games/x` as `WHO` is the creator's frame, 200;
`GET /api/games` with the header is 200 too — both rows are about the headers, not the status, so the status is
printed and not asserted.)

(c) `crates/yokohama-web/tests/start_refusal.rs` — in the binary harness (`Scratch::new(cell)` :92, which lays a
scratch dist; `Server::spawn(door, &scratch)` :191 and `Server::spawn_on(door, &scratch, port)` :198, which set
`YOKOHAMA_WEB_DIST` to `scratch.dist()` and clear every inherited `YOKOHAMA_` variable; `listening(what)` :283, the
bounded wait for the listening line, before any request; `finish()` :257 → `(ExitStatus, String, String)` for a
server's stdout and stderr; `exited(what)` :268 for an exit that is read; `free_port()` :312 and `post` :322, both
`#[cfg(feature = "open-door")]`, since CI compiles the lib without that feature), with one helper added beside
`post`, under the same gate: `get(port, path) -> (u16, http::HeaderMap)`, a plain `GET` with no identity header, over
the same `TcpStream` reader `post` uses:

```rust
/// **The production path reads `dist/headers.json`** (security S2a, decision D-1) — `main → AppState::new →
/// production → assets::response_headers`, which no in-process cell runs: `/` answers every pair of a scratch
/// dist's file and `/api/health` the pairs plus `no-store`; with NO file the binary still starts, prints one line
/// naming `headers.json` on stderr and answers `/api/health` 200 without the set; with a MALFORMED file it exits 1
/// before it listens, naming the file.
#[cfg(feature = "open-door")]
#[test]
fn the_binary_serves_the_dist_header_set_and_refuses_a_malformed_one() {
    let pairs = r#"[["X-Frame-Options", "DENY"], ["Content-Security-Policy-Report-Only", "default-src 'self'"]]"#;
    // (1) The file present: both pairs on `/`, and `no-store` beside them on `/api/health`. The scratch dist holds
    // an `index.html`, else `/` is the 404 page (which carries the set too, but 200 is what this arm reads).
    let scratch = Scratch::new("headers-present");
    std::fs::write(scratch.dist().join("headers.json"), pairs).unwrap();
    std::fs::write(scratch.dist().join("index.html"), "<!doctype html><div id=root></div>").unwrap();
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    server.listening("present");
    let (status, headers) = get(port, "/");
    assert_eq!(status, 200);
    assert_eq!(headers.get("x-frame-options").and_then(|v| v.to_str().ok()), Some("DENY"));
    assert_eq!(
        headers.get("content-security-policy-report-only").and_then(|v| v.to_str().ok()),
        Some("default-src 'self'")
    );
    let (status, headers) = get(port, "/api/health");
    assert_eq!(status, 200);
    assert_eq!(headers.get("x-frame-options").and_then(|v| v.to_str().ok()), Some("DENY"));
    assert_eq!(headers.get("cache-control").and_then(|v| v.to_str().ok()), Some("no-store"));
    drop(server);
    // (2) No file: the binary starts, says so once on stderr, serves without the set.
    let scratch = Scratch::new("headers-absent");
    let port = free_port();
    let mut server = Server::spawn_on("open", &scratch, port);
    server.listening("absent");
    let (status, headers) = get(port, "/api/health");
    assert_eq!(status, 200);
    assert!(headers.get("x-frame-options").is_none(), "no file, no set");
    let (_, _, stderr) = server.finish();
    assert!(stderr.contains("headers.json"), "the start says once what it could not read: {stderr}");
    drop(server);
    // (3) A malformed file: exit 1 before the listening line, the file named — read as the keyed-build cell
    // reads its exit.
    let scratch = Scratch::new("headers-malformed");
    std::fs::write(scratch.dist().join("headers.json"), "{}").unwrap();
    let (status, _, stderr) = Server::spawn("open", &scratch).exited("a malformed headers.json");
    assert_eq!(status.code(), Some(1));
    assert!(stderr.contains("headers.json"), "{stderr}");
}
```

(The harness's own names, as the plan review ran them — the block as first written compiled with five errors, and a
`get` straight after `spawn_on` raced the bind, hence `listening` before each request. The keyed-build cell
`a_keyed_build_behind_cf_access_exits_1_before_it_listens_saying_the_rule_s_own_sentence` is the exit's shape,
`the_binary_refuses_a_seed_beside_an_open_seat_and_opens_the_same_table_unseeded` the start's.)

(d) `deploy/smoke-image.sh`, in `door()` after `get /`: a `header <path> <name>` helper (`curl -sS --max-time 10
-D - -o /dev/null "${base}${path}" | tr -d '\r' | awk -v n="$2" -F': ' 'tolower($1)==tolower(n){print $2}'`) and
four reads — three `expect`s: `/`'s `x-frame-options` is `DENY`, `/api/health`'s `cache-control` is `no-store` and
its `x-content-type-options` is `nosniff`; and a `test -n` on `/`'s `content-security-policy-report-only` (printed,
not compared: the build's values are the image's). `bash -n deploy/smoke-image.sh` clean. CI's `docker.yml` runs
the smoke against the self-host image; the NAS's runtime image is assembled by Portainer, so no local image exists
to smoke, and the NAS's reading is the window's (Task 9 Step 9(a)).

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --lib -- response_headers header_set
2>&1 | grep -E '^error|test result|panicked' | head` (the filters AFTER `--`: before it, cargo reads the second as an
argument, and the usage error would pass for the RED) — `response_headers`, `HEADERS_FILE`, `with_headers` not
found; stubbed: (a) RED on the first pair, (b) RED on `GET /` (no header), (c) RED.

- [ ] **Step 3: The code.**

`assets.rs`, beside `cache_control`:

```rust
/// The file the client build writes beside `build-values.json` (security S2a, decision D-1): the header set as
/// an array of `[name, value]` pairs, in the order both hosts send them.
pub const HEADERS_FILE: &str = "headers.json";

/// **The header set both hosts serve, read from `dist/headers.json`** (security S2a, decision D-1). The client
/// build generates the set once from its checked values (`web/scripts/headers.mjs`) — the policy with its
/// `connect-src` hosts, the 404 page's style hash, `X-Frame-Options: DENY` and the four other companions — and
/// writes it as ordered `[name, value]` pairs; the Worker's asset layer serves the same pairs from `_headers`.
/// This host reads them ONCE at construction and [`crate::app`]'s one layer sets every pair on every answer.
///
/// An ABSENT file is an empty set — a dev run with an older or absent client build still serves, and `main`
/// prints one line saying so — and a file of any other shape is `Err`, naming the file: a build that wrote the
/// wrong thing is refused at start rather than served without its policy. A name or a value `http` would
/// refuse is the same refusal, so a header this host could not send is never half-sent.
pub fn response_headers(root: &Path) -> Result<Vec<(http::HeaderName, http::HeaderValue)>, String> {
    let path = root.join(HEADERS_FILE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{} could not be read: {e}", path.display())),
    };
    let pairs: Vec<(String, String)> = serde_json::from_str(&text).map_err(|e| {
        format!("{} is not the client build's header set (an array of [name, value] pairs): {e}", path.display())
    })?;
    if pairs.is_empty() {
        return Err(format!("{} holds no header at all", path.display()));
    }
    pairs
        .into_iter()
        .map(|(name, value)| {
            let header = http::HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| format!("{}: {name:?} is not a header name", path.display()))?;
            let value = http::HeaderValue::from_str(&value)
                .map_err(|_| format!("{}: the value of {name} is not a header value", path.display()))?;
            Ok((header, value))
        })
        .collect()
}
```

`lib.rs`: `AppState` gains, beside `dist` (:448):

```rust
    /// The header set every answer carries (security S2a, decision D-1): `dist/headers.json`'s pairs, read once
    /// where `dist` is resolved — the production constructor and [`Self::with_dist`] — and set by [`app`]'s
    /// one response layer. Empty for a state with no dist, and for a dist without the file (the client build is
    /// older than 0.6.2, or absent — `main` says so once); a cell gives a state a set with [`Self::with_headers`].
    pub headers: Arc<Vec<(http::HeaderName, http::HeaderValue)>>,
```

the base constructor initialises it `Arc::new(Vec::new())`; `with_dist` (:653) reads the file —
`headers: Arc::new(assets::response_headers(&dist).expect("a dist's headers.json is the client build's"))` — the
panic is a test fixture's error (every caller of `with_dist` is a cell); the production constructor `production`
(:501–513 — the one that already threads `sweep::check_archive(..)?` and returns `Result<Self, String>`; :642 is
inside `with_store_chat_archive_and_log`, the `pub(crate)` base every test constructor shares, which returns `Self`
and is NOT where the read goes) reads the file once the state is built, with `?`, so a malformed file is the `Err`
`main` already exits 1 on:

```rust
        sweep::check_archive(&ttls, archive::is_on(&archive))?;
        let state = Self::with_store_chat_archive_and_log(store, identity, chat, archive, Arc::new(log::StderrLog))
            .with_multiplayer(multiplayer)
            .with_ttls(ttls);
        // The header set (security S2a, decision D-1): read ONCE here, where `dist` is resolved; a malformed file
        // is this `Err`, which `main` prints and exits 1 on. An absent file is an empty set — `main` says so once.
        let headers = assets::response_headers(&state.dist)?;
        Ok(state.with_headers(headers))
```

and `main.rs`, after `AppState::new` (:180–186), says once when the set is empty — NEVER the library, whose prints
go through its log (`log::tests::the_layer_prints_only_through_its_log` scans every `src/*.rs` but `main.rs` for a
print and fails on an `eprintln!` in `lib.rs` — measured):

```rust
    // **No header set** (security S2a, decision D-1): a dist without `headers.json` — a client build older than
    // 0.6.2, or none — still serves, and says so once here, where the layer's own prints do not go.
    if state.headers.is_empty() {
        eprintln!(
            "{} has no {} — no security header is served; the client build is older than 0.6.2 or absent",
            state.dist.display(),
            yokohama_web::assets::HEADERS_FILE
        );
    }
```

`with_headers`:

```rust
    /// A header set chosen for it — how a cell gives an in-memory state the pairs a dist would (security S2a).
    pub fn with_headers(self, headers: Vec<(http::HeaderName, http::HeaderValue)>) -> Self {
        Self { headers: Arc::new(headers), ..self }
    }
```

`no_store` (:1177–1192) becomes:

```rust
/// **Every answer carries the header set, and every answer under `/api` is `no-store`** (security S2a, decision
/// D-1; S1c's finding B9). The set is the client build's (`dist/headers.json`, read once into
/// [`AppState::headers`]) — the same pairs the Worker's asset layer serves from `_headers` — set on EVERY answer,
/// static and API, the fallback's 404 and `/enter`'s 302 included: one layer over the whole router, so no answer
/// is the exception a reviewer finds, and `insert`, so a handler's own header of the same name is the set's.
/// `no-store` stays keyed on the path, as it was: the zone caches by file EXTENSION by default, and the review
/// read a victim's private frame from the edge under a table named `evening.js`. `Router::layer`, not
/// `route_layer`: the layer must wrap the fallback (`/`, `/shell.bin`, the assets, `/t/…`) as well as the routes.
#[cfg(feature = "server")]
async fn response_headers(
    State(state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = req.uri().path();
    let api = path == "/api" || path.starts_with("/api/");
    let mut res = next.run(req).await;
    for (name, value) in state.headers.iter() {
        res.headers_mut().insert(name.clone(), value.clone());
    }
    if api {
        res.headers_mut().insert(http::header::CACHE_CONTROL, http::HeaderValue::from_static("no-store"));
    }
    res
}
```

and `app()`'s `.layer(axum::middleware::from_fn(no_store))` becomes
`.layer(axum::middleware::from_fn_with_state(Arc::clone(&state), response_headers))`. The S1c cell
`every_answer_under_api_is_no_store_and_the_static_release_is_not` keeps its name and its rows; its doc gains
one sentence naming the layer's new name.

- [ ] **Step 4: Run the cells, the crate, clippy and the lane's rows**; `wire_fixtures` green without the flag (no
fixture carries a response header); and `cargo clippy -p yokohama-web --no-default-features --features server
--all-targets -- -D warnings`, the lib as CI compiles it (`web.yml` :256), where the gated `get`, like `post` and
`free_port`, must raise no dead-code warning (the pre-flight runs this first, Task 9 Step 1).

- [ ] **Step 5: The mutants.** **M14** the loop over `state.headers` dropped — (b)'s first cell red on `GET /`;
**M15** the old `from_fn(no_store)` left in `app()` — the same; **M16** `with_dist` not reading the file — (b)'s
first cell red (`app_with` builds through it); **M17** `response_headers` answering `Ok(Vec::new())` on a
malformed file — (a) red and (c)'s malformed arm red; **M18** the production constructor not reading the file —
(c)'s first arm red (the one catcher of the production path, as `start_refusal.rs` is for the seed rule);
**M19** `route_layer` in place of `layer` — (b) red first at `GET /` (the fallback's static rows: `/`, `/shell.bin`,
the assets, `/t/anything`), and the S1c cell at `/api/`; `/enter` is a route and stays green (measured — the reviewer
re-reads which rows the fallback answers, as S1c's M12b did).

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/src/assets.rs crates/yokohama-web/src/lib.rs
crates/yokohama-web/src/main.rs crates/yokohama-web/tests/start_refusal.rs deploy/smoke-image.sh` · `git commit -m "web: the NAS reads
dist/headers.json at start and one layer sets the header set on every answer, no-store under /api beside it; the
binary refuses a malformed file and says once when there is none; the smoke reads the headers (S2a Task 3)"`.

---

### Task 4: The client — the service worker carries the stored shell's headers, the entry point refuses a frame, the e2e build serves the policy ENFORCED, the live tier forgives one measured line — Opus (B4's part 3, D13; rulings A1, A5; decisions D-5, D-9; Review Focus 1, 2)

**Files:**
- Modify: `web/src/sw.ts` (`shellResponse` :417–419, its call :385, `NOT_CARRIED` new, the header comment's
  pinned list), `web/src/sw.node.test.ts` (`boot`'s `answer` :185–193 and the fake's `Response` :252–256; a new
  cell beside :690), `web/src/main.tsx`, `web/scripts/e2e-build.mjs` (the override, after the client build),
  `web/e2e/harness.ts` (`watchForBreakage` :83–130 and its declared return type :86; `isReportOnlyDetectionLine`
  new; the `watch` record), `web/e2e/harness.node.test.ts` (the cells beside the beacon's), `web/e2e/two_browsers.spec.ts`
  (:463–470), `web/e2e/joins_by_link.spec.ts` (:541–545), `web/e2e/exit_game.spec.ts` (:649–652) — the three
  `forgiven` prints
- Create: `web/src/framed.ts`, `web/src/framed.test.ts`, `web/e2e/framed.spec.ts`
- Test: `cd web && npm run typecheck && npm test -- --run && npm run test:wasm && npm run e2e`

**Interfaces:**
- Consumes: the hosts' header set on `/shell.bin` (Tasks 2 and 3); `X-Frame-Options: DENY` and the policy from the
  local binary (Task 3) for the e2e cells; `CSP_HEADER`, `headersJson`, `parseHeadersJson` (`web/scripts/headers.mjs`,
  Task 1) for the override.
- Produces: `NOT_CARRIED`, `shellResponse(hit)`; `framed`, `FRAMED_SENTENCE`; the e2e build's enforced
  `dist/headers.json`; `isReportOnlyDetectionLine`, `watch.reportOnlyForgiven`.

**What the plan review measured, which this task rests on** (2026-10-11; the record is
`docs/temp/security-review-2026-10-08/s2a-plan-review/`): Chromium 143–153 and Chrome 155 log a Report-Only
violation at console type `info` — "Executing inline script violates the following Content Security Policy directive
'script-src …'. Either the 'unsafe-inline' keyword, a hash ('sha256-…'), or a nonce ('nonce-...') is required to
enable inline execution. The policy is report-only, so the violation has been logged but no further action has been
taken." — and an enforced one at `error`, with "The action has been blocked." in place of the last sentence;
`frame-src`'s and `frame-ancestors`' Report-Only lines log at `error`; a violation raised INSIDE the module Worker
(the wasm compile without `'wasm-unsafe-eval'`) prints on no console channel in either mode. A framer on
`about:blank`, a `data:` URL or an https page is stopped by Chromium's Local Network Access check
(`net::ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`) before a loopback host answers; a framer on a second loopback
origin gets the answer, its `X-Frame-Options`, then `ERR_BLOCKED_BY_RESPONSE`.

- [ ] **Step 1: The failing cells.**

(a) `web/src/sw.node.test.ts`: `boot`'s `answer` may return `headers?: Record<string, string>`, and `fetchFake`
sets them beside `content-type` (`new Response(said.body, { status: said.status, headers: { "content-type":
said.type, ...(said.headers ?? {}) } })`). Beside the navigation cell (:690):

```ts
  // **D13 (security S2a; decision D-5): a navigation the worker synthesizes carries the headers the host set on
  // `/shell.bin`** — the policy, `X-Frame-Options` and the rest — else every header the host sends protects a
  // first visit only, and the service worker undoes B4 on every returning visit a SAME-site page frames (a
  // cross-site framer meets the network's `X-Frame-Options` in default Chrome, which partitions the worker away
  // from it). Three are NOT carried: the type, which is the worker's HTML; and `content-length` and
  // `content-encoding`, which describe the ENCODED bytes the server sent, where the stored body is the decoded
  // bytes the install digested. A header no cell names is carried too: the pass-through is not an allow-list.
  it("carries the stored shell's headers onto a navigation, the HTML type over them, and not the three the bytes no longer match", async () => {
    const worker = await boot((path) => {
      const known = BODIES[path];
      if (!known) return { status: 404, body: NOT_ON_THE_ORIGIN, type: "text/plain" };
      const headers: Record<string, string> =
        path === "/shell.bin"
          ? {
              "x-frame-options": "DENY",
              "content-security-policy-report-only": "default-src 'self'",
              "cache-control": "no-store",
              "x-a-header-the-set-has-not-got-yet": "kept",
              "content-encoding": "br",
              "content-length": "9999",
            }
          : {};
      return { status: 200, body: known.body, type: known.type, headers };
    });
    await worker.install();
    const page = (await worker.request("https://yokohama.test/t/anything", "navigate"))!;
    expect(page.status).toBe(200);
    expect(page.headers.get("content-type")).toBe("text/html; charset=utf-8");
    expect(page.headers.get("x-frame-options")).toBe("DENY");
    expect(page.headers.get("content-security-policy-report-only")).toBe("default-src 'self'");
    expect(page.headers.get("cache-control")).toBe("no-store");
    expect(page.headers.get("x-a-header-the-set-has-not-got-yet"), "carried although no cell names it: not an allow-list").toBe("kept");
    expect(page.headers.get("content-encoding"), "the stored body is decoded bytes").toBeNull();
    expect(page.headers.get("content-length")).toBeNull();
    expect(await page.text()).toBe(BODIES["/shell.bin"]!.body);
    // Anti-vacuity: the stored entry DOES carry the three, so their absence above is the worker's doing.
    const stored = (await worker.request("https://yokohama.test/shell.bin", "cors"))!;
    expect(stored.headers.get("content-encoding")).toBe("br");
    expect(stored.headers.get("content-type")).toBe("application/octet-stream");
  });
```

(The ternary is typed `Record<string, string>`: untyped, it is not assignable to `boot`'s `headers?` under
`tsconfig.node.json` — TS2345, measured — and the red appears only at the typecheck gate.)

(b) `web/src/framed.test.ts`:

```ts
// **A page inside another page's frame does not render** (security S2a, finding B4's third gap; ruling A5). The
// host's `X-Frame-Options: DENY` is the fence; this is the client's own refusal, for a browser or a frame that
// did not honour it, and it costs a comparison.
import { describe, expect, it } from "vitest";
import { FRAMED_SENTENCE, framed } from "./framed";

describe("framed", () => {
  it("is false when the window is its own top, true when it is not", () => {
    const top = {};
    expect(framed({ top, self: top })).toBe(false);
    expect(framed({ top, self: {} })).toBe(true);
    expect(framed({ top: null, self: {} })).toBe(true);
  });
  it("has one sentence, which names no address and no other page", () => {
    expect(FRAMED_SENTENCE).toBe("Yokohama does not run inside another page. Open it in a tab of its own.");
  });
});
```

(c) `web/e2e/framed.spec.ts` — two cells against the local binary, which serves Task 3's layer: the host's refusal
(spec §2.2 B4's catcher column), and the client's own.

```ts
// **A framed door does not act** (security S2a, finding B4; spec §2.2 B4's catcher). The review framed `#/lobby`
// at low opacity from another site and two clicks landed on Abandon and its confirm. Now the local binary answers
// the shell with `X-Frame-Options: DENY` (Task 3) and the browser refuses to render it inside a frame: the shell's
// bytes are fetched, nothing in them is parsed, so none of the release's scripts is asked for and nothing under
// `/api/` is touched. The control at the end navigates the SAME page to the door directly and reads the release's
// scripts being asked for, so "no script was asked for" is the frame's refusal and not a server that is down.
// The second cell strips the host's fences from the framed document and reads the client's own refusal — the one
// sentence `main.tsx` puts in `#root` instead of mounting — which is the catcher of the `framed(window)` branch.
//
// **The framer is a second LOOPBACK origin** (measured, the plan review): a framer on `about:blank`, a `data:` URL
// or an https page is stopped by Chromium's Local Network Access check,
// `net::ERR_BLOCKED_BY_LOCAL_NETWORK_ACCESS_CHECKS`, before the host answers, so a cell framing from one would time
// out with AND without the fence. The worker is OFF (the `./sw` fixture): the question is what the HOST answers a
// framed navigation.
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import { BASE_URL, haveBrowser } from "../playwright.config";
import { expect, test } from "./sw";

// No browser, and `YOKOHAMA_WEB_E2E_OPTIONAL=1`: SKIP, visibly — the guard
// `every_spec_can_skip.spec.ts` reads this directory for.
test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

/** The host's frame fences, stripped by the second cell's route so the client's own is what answers: the header,
 *  and the policy under both names (the e2e build serves it ENFORCED, and its `frame-ancestors 'none'` refuses
 *  the frame as the header does). */
const FRAME_FENCES = ["x-frame-options", "content-security-policy", "content-security-policy-report-only"];

let framer: Server;
let framerUrl = "";
test.beforeAll(async () => {
  framer = createServer((_req, res) => {
    res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    res.end(`<!doctype html><title>elsewhere</title><iframe id="f" src="${BASE_URL}/#/lobby" width="900" height="700"></iframe>`);
  });
  await new Promise<void>((r) => framer.listen(0, "127.0.0.1", r));
  framerUrl = `http://localhost:${(framer.address() as AddressInfo).port}/`;
});
test.afterAll(() => framer.close());

/** How long the frame is watched after its document answered, for a request it would make if it had rendered. */
const SETTLE_MS = 2_000;

test("a page on another origin that frames the lobby gets a refused frame: X-Frame-Options DENY on the answer, the policy ENFORCED beside it, and none of the release asked for", async ({ page }) => {
  const origin = new URL(BASE_URL).origin;
  const asked: string[] = [];
  page.on("request", (r) => {
    if (r.url().startsWith(origin)) asked.push(new URL(r.url()).pathname);
  });
  const shell = page.waitForResponse((r) => r.url() === `${BASE_URL}/`);
  await page.goto(framerUrl);
  const answer = await shell;
  expect(answer.headers()["x-frame-options"], "the host's fence").toBe("DENY");
  expect(answer.headers()["content-security-policy"], "the e2e build serves the policy ENFORCED (decision D-9)").toBeDefined();
  await page.waitForTimeout(SETTLE_MS);
  expect(asked.filter((p) => p.startsWith("/assets/")), "a refused frame parses nothing, so it asks for no script").toEqual([]);
  expect(asked.filter((p) => p === "/api" || p.startsWith("/api/")), "and touches the API not at all").toEqual([]);

  // The control: the same page, navigated to the door directly, asks for the release's scripts.
  asked.length = 0;
  await page.goto(`${BASE_URL}/#/`);
  await page.getByRole("heading", { name: /Yokohama/ }).first().waitFor();
  expect(asked.some((p) => p.startsWith("/assets/")), "the door itself loads its scripts").toBe(true);
});

test("the client's own refusal: with the host's frame fences stripped from the framed document, the page says its one sentence and mounts nothing", async ({ page }) => {
  await page.route(`${BASE_URL}/`, async (route) => {
    const response = await route.fetch();
    const headers = { ...response.headers() };
    for (const name of FRAME_FENCES) delete headers[name];
    await route.fulfill({ response, headers });
  });
  await page.goto(framerUrl);
  const frame = page.frameLocator("#f");
  await expect(frame.locator("#root")).toHaveText("Yokohama does not run inside another page. Open it in a tab of its own.");
  await expect(frame.locator("h1")).toHaveCount(0);
});
```

(The door's heading is `web/src/FrontDoor.tsx:106`, `<h1>Yokohama</h1>`. `every_spec_declares_its_worker.spec.ts`
is satisfied by the `./sw` import and no own context; `every_spec_can_skip.spec.ts` by the skip line. The plan
review ran the two cells against a stand-in serving the real built dist: 2 passed three times over, and red
"Expected "DENY", Received undefined" with the fence off; the second cell red with `main.tsx`'s branch deleted.)

(d) `web/e2e/harness.node.test.ts`, at the file's end, after its last describe (the plan review placed it after :1277) —
this file mocks `../playwright.config`, which is
why it runs where CI's `web-server` job installs no Chromium; `live.node.test.ts` does not, and a cell placed there
would take the whole file down in that job (measured: "no Chromium for the browser walk", 16 cells lost). Through
the real `watchForBreakage`, with the file's `Handler` type (:168) and `harnessAt(live)` loader (:193), `LIVE_ORIGIN`
and `baseUrlFor` from `./live` (already imported), and `isReportOnlyDetectionLine` added to the `./harness` import
(:27):

```ts
// ---- the Report-Only lines, as Chromium 143–153 words and levels them (MEASURED, the S2a plan review) ---------

/** A page whose console emits a line of the type the cell chooses — `watchForBreakage` reads the type, so the
 *  `fakePage` above, which emits `error` alone, cannot drive the Report-Only arm. */
function consolePage() {
  const handlers = new Map<string, Handler[]>();
  const page = {
    on(event: string, handler: Handler) {
      handlers.set(event, [...(handlers.get(event) ?? []), handler]);
      return page;
    },
  };
  const emit = (event: string, arg: unknown) => (handlers.get(event) ?? []).forEach((h) => h(arg));
  return {
    page: page as unknown as Page,
    line: (type: string, text: string, url: string) =>
      emit("console", { type: () => type, text: () => text, location: () => ({ url, lineNumber: 0, columnNumber: 0 }) } as unknown as ConsoleMessage),
  };
}

/** Chromium's Report-Only refusal of an inline script under the shipped `script-src`, word for word (the hash is
 *  the injected script's own, and varies). */
const RO_INLINE =
  "Executing inline script violates the following Content Security Policy directive 'script-src 'self' 'wasm-unsafe-eval' https://challenges.cloudflare.com https://static.cloudflareinsights.com'. Either the 'unsafe-inline' keyword, a hash ('sha256-aBV9DC+smatNUlnkSmZOl+jfLQV1VY8nHmiO1eoMzIU='), or a nonce ('nonce-...') is required to enable inline execution. The policy is report-only, so the violation has been logged but no further action has been taken.";
/** The same refusal of an inline STYLE — another directive's line. */
const RO_STYLE = RO_INLINE.replace("Executing inline script", "Applying inline style").replace("'script-src ", "'style-src ");
/** The ENFORCED refusal: Chromium's last sentence changes, and the line is an `error`. */
const ENFORCED_INLINE = RO_INLINE.replace(
  "The policy is report-only, so the violation has been logged but no further action has been taken.",
  "The action has been blocked.",
);

describe("the Report-Only lines (type info) are read, and the live tier forgives exactly one (security S2a, decision D-9)", () => {
  afterEach(() => {
    vi.doUnmock("./live");
    vi.resetModules();
  });
  it("locally every CSP line is breakage, whatever its console type", async () => {
    const { watchForBreakage } = await harnessAt(false);
    const fake = consolePage();
    const watch = watchForBreakage(fake.page);
    const local = baseUrlFor(false);
    fake.line("info", RO_INLINE, `${local}/`);
    fake.line("info", RO_STYLE, `${local}/`);
    expect(watch.failures).toHaveLength(2);
    expect(watch.reportOnlyForgiven).toBe(0);
  });
  it("live forgives the Report-Only inline-script line at a page of the deployment, counted apart, and nothing beside it", async () => {
    const { watchForBreakage } = await harnessAt(true);
    const fake = consolePage();
    const watch = watchForBreakage(fake.page);
    fake.line("info", RO_INLINE, `${LIVE_ORIGIN}/`);
    expect([watch.failures, watch.reportOnlyForgiven, watch.forgiven]).toEqual([[], 1, 0]);
    fake.line("info", RO_STYLE, `${LIVE_ORIGIN}/`);
    fake.line("error", ENFORCED_INLINE, `${LIVE_ORIGIN}/`);
    fake.line("info", RO_INLINE, "https://elsewhere.test/");
    fake.line("info", "an ordinary info line", `${LIVE_ORIGIN}/`);
    expect(watch.failures).toHaveLength(3);
    expect(watch.reportOnlyForgiven).toBe(1);
  });
  it("isReportOnlyDetectionLine: the measured line at the deployment's ORIGIN, and nothing that merely starts with it", () => {
    expect(isReportOnlyDetectionLine(RO_INLINE, `${LIVE_ORIGIN}/`, LIVE_ORIGIN)).toBe(true);
    expect(isReportOnlyDetectionLine(RO_INLINE, `${LIVE_ORIGIN}.evil.test/`, LIVE_ORIGIN), "a longer host is another origin").toBe(false);
    expect(isReportOnlyDetectionLine(RO_INLINE, "not a url", LIVE_ORIGIN)).toBe(false);
    expect(isReportOnlyDetectionLine(ENFORCED_INLINE, `${LIVE_ORIGIN}/`, LIVE_ORIGIN), "an ENFORCED refusal is breakage").toBe(false);
    expect(
      isReportOnlyDetectionLine(RO_INLINE.replace("inline script", "script from 'https://evil.test/x.js'"), `${LIVE_ORIGIN}/`, LIVE_ORIGIN),
      "a refused external script is breakage",
    ).toBe(false);
  });
});
```

- [ ] **Step 2: Run them to verify they fail.** `cd web && npx vitest run -c vitest.node.config.ts
src/sw.node.test.ts e2e/harness.node.test.ts` — (a) RED on `x-frame-options` (null), (d) RED (no such export;
`reportOnlyForgiven` undefined); `npx vitest run src/framed.test.ts` — RED (no module); (c) RED until Task 3's
layer is in (`Expected "DENY", Received undefined`); with Task 3 in, its first cell RED at the policy assertion until
this task's override is in (the e2e build still serves Report-Only), and its second cell RED until `main.tsx`
refuses (the lobby renders in the frame).

- [ ] **Step 3: The code.**

`web/src/sw.ts`: `shellResponse` takes the stored `Response`:

```ts
/** Headers the stored entry carries that the navigation's answer must NOT (security S2a, decision D-5): the type,
 *  which is set to HTML below (R-6.5c2.41) — listed for the reader: the `set` below is what replaces it — and the
 *  two that describe the ENCODED bytes the server sent, where the stored body is the decoded bytes the install
 *  digested. Everything else is carried, `Cache-Control: no-store` among it, so a header the hosts gain later
 *  reaches a returning visitor without an edit here. */
const NOT_CARRIED: ReadonlySet<string> = new Set(["content-type", "content-length", "content-encoding"]);

/** The shell's cached bytes as the page a navigation asked for: a 200 with the HTML type the hosts serve
 *  `/index.html` with, set HERE — the stored entry carries `/shell.bin`'s `application/octet-stream`
 *  (R-6.5c2.41) — and EVERY OTHER header the host set on `/shell.bin` (finding D13, security S2a): the policy,
 *  `X-Frame-Options`, the rest. Before this the synthesized page carried the type alone, so every header the
 *  host sent protected a first visit only. (`forEach`, not `for…of`: the worker's `tsconfig` gives `Headers` no
 *  iterator.) */
function shellResponse(hit: Response): Response {
  const headers = new Headers();
  hit.headers.forEach((value, name) => {
    if (!NOT_CARRIED.has(name.toLowerCase())) headers.append(name, value);
  });
  headers.set("content-type", "text/html; charset=utf-8");
  return new Response(hit.body, { status: 200, headers });
}
```

and the dispatch (:385) passes the hit: `fromCache(SHELL_PATH, request, (hit) => shellResponse(hit))`. The
header comment's "pinned by `sw.node.test.ts`" list gains: "a navigation carries the stored shell's headers but
the type and the two encoding headers". (A `for…of` over `hit.headers` fails `npm run typecheck` under
`tsconfig.sw.json`'s `ES2022` + `WebWorker` lib — TS2488, measured — while passing vitest.)

`web/src/framed.ts`:

```ts
// **Not inside another page** (security S2a, finding B4's third gap; ruling A5). The host's
// `X-Frame-Options: DENY` is what keeps a signed-in page out of a hostile frame; this is the client's own
// refusal for a frame that was rendered anyway, and it costs one comparison at boot.

/** Whether this document is inside another page's frame: its window is not the top one. A `top` of `null`
 *  (a detached frame) reads as framed too — the safe side. */
export function framed(win: { top: unknown; self: unknown }): boolean {
  return win.top !== win.self;
}

/** What the page says instead of rendering, when it is framed. */
export const FRAMED_SENTENCE = "Yokohama does not run inside another page. Open it in a tab of its own.";
```

`web/src/main.tsx`: import `framed`, `FRAMED_SENTENCE`; after the `root` check:

```tsx
// **Inside another page's frame, nothing renders** (security S2a, finding B4): the host's `X-Frame-Options`
// keeps the page out of a frame, and this is the page's own word for the frame that got through.
if (framed(window)) {
  root.textContent = FRAMED_SENTENCE;
} else {
  createRoot(root).render(/* as today */);
}
```

`web/scripts/e2e-build.mjs`: `readFileSync`, `writeFileSync` from `node:fs` and `CSP_HEADER`, `headersJson`,
`parseHeadersJson` from `./headers.mjs` imported at the top; after the `spawnSync` of `build:client` and before
`process.exit(run.status ?? 1)`:

```js
// **The e2e build serves the policy ENFORCED** (security S2a, decision D-9): Chromium logs a Report-Only violation
// at console type `info`, which `watchForBreakage` does not read, and a violation inside the module Worker prints
// nowhere in either mode — so the local walk is the enforcement reading only under `Content-Security-Policy`. An
// override that can only strengthen: the shipped `CSP_MODE` stays the constant, with its cell, and this rewrites
// the e2e build's `dist/headers.json` alone. (A post-e2e `dist` carries the enforced set: the Worker's assembly
// already refuses a post-e2e dist for its debug module, and the NAS recipe rebuilds.)
if (run.status === 0) {
  const file = new URL("../dist/headers.json", import.meta.url);
  const set = parseHeadersJson(readFileSync(file, "utf8")).map(([name, value]) =>
    name === CSP_HEADER["report-only"] ? [CSP_HEADER.enforce, value] : [name, value],
  );
  writeFileSync(file, headersJson(set));
}
```

and the file's header comment gains a paragraph saying so. Its pin is framed.spec.ts's first cell (the policy
asserted under the enforcing name).

`web/e2e/harness.ts`: beside `isBeaconCorsError`:

```ts
/** Any CSP violation line, enforced or Report-Only, as Chromium 143–153 words it (MEASURED, the S2a plan review):
 *  "… violates the following [report-only ]Content Security Policy directive …". Report-Only logs at `info`
 *  (`frame-src`'s and `frame-ancestors`' at `error`), enforced at `error` — so the console handler reads a CSP
 *  line at ANY type: the type gate alone would read nothing of a Report-Only release. */
const CSP_VIOLATION = /\bviolates the following (?:report-only )?Content Security Policy directive\b/;

/** The ONE Report-Only line the live tier forgives: an INLINE script refused by `script-src` — Cloudflare's
 *  injected detection script — exactly as Chromium words it, its hash free (MEASURED on an emulated injection; the
 *  real script's line is read in the window, Task 9 Step 9(c)). */
const REPORT_ONLY_INLINE_SCRIPT =
  /^Executing inline script violates the following Content Security Policy directive 'script-src [^\n]*'\. Either the 'unsafe-inline' keyword, a hash \('sha256-[A-Za-z0-9+/]+=*'\), or a nonce \('nonce-\.\.\.'\) is required to enable inline execution\. The policy is report-only, so the violation has been logged but no further action has been taken\.$/;

/**
 * **Whether a console line is the live tier's third known line** (security S2a, decision D-9): the policy ships
 * Report-Only in this release, and Bot Fight Mode's JavaScript Detections inject an INLINE script into every HTML
 * answer the edge serves — a first load — which no static policy can name (Cloudflare copies a nonce onto it only
 * from a header a static shell cannot vary). Chromium logs the refusal at `info`, located at the page. Forgiven
 * ONLY in that shape and at a page of the deployment's own ORIGIN (`new URL(pageUrl).origin`, never a prefix test,
 * which `https://yokohama.moore.nyc.evil.test/` would pass): an enforced refusal, a refused external script or
 * any other directive's line is breakage, as the local tier reads every CSP line (there is no edge locally, so no
 * inline script, so no line — and the local tier is served the policy ENFORCED).
 */
export function isReportOnlyDetectionLine(text: string, pageUrl: string, base: string): boolean {
  let same = false;
  try {
    same = new URL(pageUrl).origin === new URL(base).origin;
  } catch {
    same = false;
  }
  return same && REPORT_ONLY_INLINE_SCRIPT.test(text);
}
```

and in `watchForBreakage`: the declared return type (:86) gains `reportOnlyForgiven: number` and the record
`reportOnlyForgiven: 0`; the handler's first two lines `if (m.type() !== "error") return; const text = m.text();`
become `const text = m.text(); if (m.type() !== "error" && !CSP_VIOLATION.test(text)) return;`; after the feed's
arm, `if (LIVE && isReportOnlyDetectionLine(text, m.location().url, BASE_URL)) { watch.reportOnlyForgiven += 1;
return; }`; the doc comment's "TWO more errors are forgiven" becomes THREE, naming this one and saying a CSP line is
read at any type; the three live specs that print `forgiven` — `two_browsers.spec.ts:463–470`,
`joins_by_link.spec.ts:541–545`, `exit_game.spec.ts:649–652` — print this counter too.

- [ ] **Step 4: Run the cells, then the whole client tier**: `npm run typecheck && npm test -- --run && npm run
test:wasm`, then `npm run e2e` — **the local walk now runs under the policy ENFORCED, which `e2e-build.mjs` writes
into the e2e build (decision D-9): an enforced violation is a console `error`, which `watchForBreakage` fails on,
and a violation inside the module Worker fails the walk itself (no game is dealt), so a green walk is the local
enforcement reading.** A red walk names the directive; the fix is a source the policy genuinely needs (reported
`DONE_WITH_CONCERNS` with the line, for the orchestrator's ruling — never `'unsafe-inline'`). Record the walk's wall
and the count of console lines. (The plan review's reading: 77 of 81 cells green under the policy enforced on the
real NAS binary — a release-client subset — the other four red identically with no header set, or a flake that
passed 3/3; nothing the covered flows need is blocked today.)

- [ ] **Step 5: The mutants.** **M20** `shellResponse` copying no header (today's code) — (a) red on
`x-frame-options`; **M21** `content-encoding` carried — (a) red; **M22** the type not overridden — the S1
navigation cell red (octet-stream); **MX1** an allow-list of the set's six names in place of `NOT_CARRIED` — (a) red
at the extra header; **M23** `framed` always false — (b) red; **M23b** the `if (framed(window))` branch deleted from
`main.tsx` (the client rebuilt) — (c)'s second cell red (the lobby renders in the frame); **M24** the regex's
`report-only` clause dropped — (d)'s live cell red at the enforced row (forgiven, not failed); **M25** the origin
check dropped — (d)'s live cell red at the other-page row, and the pure cell at the longer host; **M24b** the
handler's type gate restored (`if (m.type() !== "error") return;`) — (d)'s local cell red (0 failures); **M24c** the
`LIVE &&` dropped from the arm — (d)'s local cell red (forgiven locally); **M24d** `e2e-build.mjs` writing no
override — (c)'s first cell red at the policy assertion; **M26** Task 3's layer dropped in the export — (c)'s first
cell red on `x-frame-options` (the e2e cell reads the host; the reviewer runs it once against a binary with the
layer removed).

- [ ] **Step 6: Commit.** `git add web/src/sw.ts web/src/sw.node.test.ts web/src/main.tsx web/src/framed.ts
web/src/framed.test.ts web/e2e/framed.spec.ts web/scripts/e2e-build.mjs web/e2e/harness.ts
web/e2e/harness.node.test.ts web/e2e/two_browsers.spec.ts web/e2e/joins_by_link.spec.ts web/e2e/exit_game.spec.ts` ·
`git commit -m "web: the service worker carries the stored shell's headers onto every navigation it synthesizes;
the entry point refuses to render inside a frame, with its e2e catcher; the e2e build serves the policy ENFORCED so
the local walk is the enforcement reading; the live tier reads a CSP line at any level and forgives exactly the
measured Report-Only line of Cloudflare's injected detection script (S2a Task 4)"`.

---

### Task 5: D14 — the hash router admits the store's id grammar, pinned to an emitted fixture — Sonnet (D14; ruling A4; decision D-6; Review Focus 4)

**Files:**
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs` (`id_cases()` beside `identity_cases()` :2764; the
  `all.push` list :2475; `CONSTRUCTED_WITH_AN_AT` :2964; a verdict cell beside :2801), `web/src/wire/contract.ts`
  (the four `identityCases` sites :115, :445, :568, :957 — `idCases` beside each), `web/src/route.ts` (`parseRoute`
  :45–63 and the comment above it), `web/src/route.test.ts` (:27–43), `web/src/local/corpus.node.test.ts`
  (`NOT_SERVED` :155 — the files no script serves; `"id_cases"` after `"identity_cases"`, and the contracts bullet
  in the comment above it names it beside `identity_cases`), `worker/test/corpus.test.ts` (`THIRD_COLUMN` :99 —
  the files no script names, an exact set; `"id_cases"` between `"house_frame"` and `"identity_cases"`, with a line
  in the comment above it)
- Create: `web/src/tableId.ts`, `web/src/tableId.test.ts`; the regenerated `web/src/wire/id_cases.json`
- Test: `cargo test -p yokohama-web --test wire_fixtures`, `cd web && npm run typecheck && npm test -- --run && npm
  run test:wasm`; `cd worker && npm run build:module:debug && npm test` (the pool's `corpus.test.ts` asserts the
  no-script set exactly; `worker/test/corpus.gen.ts` is generated on `pretest` and gitignored — it never moves, and
  is never added)

**Interfaces:**
- Consumes: `yokohama_web::store::is_safe_id` (`store.rs:186–193`; the `store` module is `pub`, `lib.rs:75`).
- Produces: `id_cases.json`, `wire.idCases`, `tableId.ts`'s `isSafeId` and `ID_MAX`.

**The judgment this brief carries, so a Sonnet implementer changes nothing else:** the fixture is the authority
and the client is held to it both ways, exactly as `nameRule.ts` is held to `identity_cases.json`; `parseRoute`
stays total (an id the grammar refuses is the DOOR, never an exception); no route, status or body moves; the one
existing cell that changes is `#/t/a%20b` (D-6); the two corpus-replay lists gain `id_cases` and nothing else;
nothing else in `route.ts` is touched.

- [ ] **Step 1: The failing cells.**

(a) `wire_fixtures.rs`, beside `identity_cases()`:

```rust
/// **The store's id grammar, as verdicts over a case list** (security S2a, finding D14; decision D-6): the hash
/// router's `#/t/<id>` admits exactly what `store::is_safe_id` admits, and this file is how the client's copy
/// (`web/src/tableId.ts`) is held to the server's both ways, as the name step is held by `identity_cases`. Not a
/// payload any route serves — a CONTRACT. The rows the rule turns on: the two id shapes the client itself draws
/// (`sim-` and `local-` with hex), the length bound at 64 and 65, the reserved suffixes in both cases, the leading
/// character, every traversal spelling, a space (what `#/t/a%20b` decodes to) and a non-ASCII letter.
fn id_cases() -> Value {
    use yokohama_web::store::is_safe_id;
    let (a64, a65) = ("a".repeat(64), "a".repeat(65));
    let inputs = [
        "sim-0123456789abcdef", "local-1e205cc2c8b9f836", "x", "7", "evening-2", "evening_2", "evening.2", "x.tablet",
        a64.as_str(), a65.as_str(),
        "x.table", "X.TABLE", "x.chat", "x.Chat",
        "_house", "house", "-x", ".x", "_x", "x-", "x.", "x_",
        ".", "..", "../x", "x/..", "a/b", "x/", "/x",
        "a b", " x", "x ", "", "a@b", "a%20b", "B\u{e9}n", "\u{212a}elvin", "a\u{200b}b", "x\n",
    ];
    Value::Array(inputs.iter().map(|raw| json!({ "raw": raw, "safe": is_safe_id(raw) })).collect())
}

/// **What the rows say, where a person would check first** — the premises the client's pin rests on: the two
/// drawn shapes pass, the bound is 64, a reserved suffix is refused in any case, every traversal spelling and the
/// space are refused.
#[test]
fn the_id_cases_carry_the_store_s_verdicts() {
    let rows = id_cases();
    let rows = rows.as_array().expect("a list");
    let safe = |raw: &str| rows.iter().find(|r| r["raw"] == raw).unwrap_or_else(|| panic!("no row for {raw:?}"))["safe"] == true;
    for ok in ["sim-0123456789abcdef", "local-1e205cc2c8b9f836", "x", "evening.2", "x.tablet", &"a".repeat(64)] {
        assert!(safe(ok), "{ok:?} is a usable id");
    }
    for bad in [&"a".repeat(65), "x.table", "X.TABLE", "x.chat", "_house", "-x", ".", "..", "../x", "a/b", "a b", "", "a@b", "a%20b", "B\u{e9}n"] {
        assert!(!safe(bad), "{bad:?} is refused");
    }
}
```

`all.push(("id_cases".to_string(), id_cases()));` beside the identity line (:2475); `CONSTRUCTED_WITH_AN_AT` gains
`("id_cases", None, "the id grammar's contract: its one @ is a refused input, and no route serves the file")`.

(b) `web/src/tableId.test.ts`, in `nameRule.test.ts`'s shape:

```ts
// The hash router's id rule, pinned to the store's (security S2a, finding D14; decision D-6). The table is the
// LAYER's verdicts — `store::is_safe_id` over every row, emitted by `wire_fixtures` into
// `web/src/wire/id_cases.json` and refused when it is stale — so this cell holds the client's copy to the
// server's both ways: an id the server would store under, the client opens; one the server refuses, the client
// sends to the door before any request is made.
import { describe, expect, it } from "vitest";
import { ID_MAX, isSafeId } from "./tableId";
import { wire } from "./wire/contract";

const cases = wire.idCases;

const shown = (s: string): string =>
  `"${[...s].map((c) => { const cp = c.codePointAt(0)!; return cp >= 0x20 && cp <= 0x7e ? c : `\\u${cp.toString(16).padStart(4, "0")}`; }).join("")}"`;

/** Copied from the committed table's `raw` column, so the table must carry every one or the pin proves nothing
 *  about them. */
const REQUIRED = [
  "sim-0123456789abcdef", "local-1e205cc2c8b9f836", "x", "7", "evening-2", "evening_2", "evening.2", "x.tablet",
  "a".repeat(ID_MAX), "a".repeat(ID_MAX + 1),
  "x.table", "X.TABLE", "x.chat", "x.Chat",
  "_house", "house", "-x", ".x", "_x", "x-", "x.", "x_",
  ".", "..", "../x", "x/..", "a/b", "x/", "/x",
  // The Kelvin sign and the zero-width space are `\u` escapes, as the Rust half spells them: an invisible code
  // point in code is an escape (the repository's rule), and an NFKC editor would fold a raw Kelvin sign into `K`.
  "a b", " x", "x ", "", "a@b", "a%20b", "Bén", "\u212aelvin", "a\u200bb", "x\n",
];

describe("the id rule", () => {
  it("gives the store's own verdict on every input the store judged", () => {
    for (const c of cases) expect(isSafeId(c.raw), shown(c.raw)).toBe(c.safe);
  });
  it("is pinned on the inputs the rule turns on, so the cell above is not vacuous", () => {
    const raws = new Set(cases.map((c) => c.raw));
    for (const raw of REQUIRED) expect(raws.has(raw), `id_cases.json lacks ${shown(raw)}`).toBe(true);
    expect(cases.some((c) => c.safe), "an accepted id").toBe(true);
    expect(cases.some((c) => !c.safe), "a refusal").toBe(true);
  });
});
```

(c) `web/src/route.test.ts`, the parse cell (:27–45): the `#/t/a%20b` expectation becomes `{ kind: "door" }` with
its comment rewritten — decoded still, because `hashOf` encodes, but a space is outside the store's grammar
(security S2a, D14), so the decoded id is judged and a hand-typed link to an id no table can have lands on the
door; new rows: `#/t/..` → door; `#/t/x.table` → door; `#/t/%2E%2E` → door (decodes to `..`); `#/t/sim-0123` →
table (unchanged); `#/t/local-1e205cc2c8b9f836` → `{ kind: "table", id: … }`; `#/t/sim%2D0123` → `{ kind: "table",
id: "sim-0123" }` (the DECODED id is judged: the encoded segment holds a `%` the grammar refuses); and
`#/t/aaaaaaaaaa.table` → door (a reserved suffix after a long head — a rule that judged a prefix alone would admit
it).

- [ ] **Step 2: Run them to verify they fail.** `cargo test -p yokohama-web --test wire_fixtures` — RED: the
committed set lacks `id_cases.json` (the test refuses a fixture it emits that is not committed); `cd web && npx
vitest run src/tableId.test.ts src/route.test.ts` — RED (no module; `a%20b` still a table).

- [ ] **Step 3: The code.** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures` writes
`web/src/wire/id_cases.json` — READ the diff with `cat -v` (the fixture holds the raw Kelvin sign and zero-width
space, as `identity_cases.json` does): one new file of 39 rows, 13 of them `safe`, and nothing else. With the file
in place `npm run test:wasm`'s `corpus.node.test.ts` and the pool's `corpus.test.ts` are RED at the unexpected
`id_cases` (measured: "the files the script does not name: expected [ …(31) ] to deeply equal [ …(30) ]"), so
`NOT_SERVED` and `THIRD_COLUMN` gain `"id_cases"` as the Files block says. Then:

`web/src/wire/contract.ts`: `import idCases from "./id_cases.json";` beside `identityCases` (:115); `interface
IdCase { raw: string; safe: boolean }` and `const _idCases: readonly IdCase[] = idCases;` beside the identity's
(:445); `_idCases` in the list at :568; `idCases: idCases as readonly IdCase[],` beside `identityCases` (:957).

`web/src/tableId.ts`:

```ts
// The store's id grammar, as the hash router applies it (security S2a, finding D14; decision D-6).
//
// **A restatement, pinned.** `store::is_safe_id` (`crates/yokohama-web/src/store.rs`) is the authority — it is
// what every game's documents are keyed under — and `tableId.test.ts` holds this file to its verdicts over
// `web/src/wire/id_cases.json`, which `wire_fixtures` emits and refuses when it is stale. The router asks it so a
// link whose id the store would refuse — `..`, `x.table`, a space — lands on the door and never becomes an API
// path: `encodeURIComponent("..")` is `..`, and `/api/games/../chat` resolves to the HOUSE chat.

/** The longest id, in bytes — every accepted id is ASCII, so in characters too (`is_safe_id`'s `id.len() <= 64`). */
export const ID_MAX = 64;

/** The suffixes the store keeps for its own documents (`RESERVED_ID_SUFFIXES`), refused in any case. */
const RESERVED_SUFFIXES = [".table", ".chat"];

/** `is_safe_id`, arm by arm: not empty, at most 64 bytes, a letter or digit first, then letters, digits, `.`, `_`
 *  and `-` only, and not spelling another document's suffix. The alphabet test comes before the suffix test and
 *  makes the ASCII fold below exact: nothing outside ASCII survives it. */
export function isSafeId(id: string): boolean {
  if (id === "" || id.length > ID_MAX) return false;
  if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(id)) return false;
  const lower = id.replace(/[A-Z]/g, (c) => c.toLowerCase());
  return !RESERVED_SUFFIXES.some((suffix) => lower.endsWith(suffix));
}
```

`web/src/route.ts`, `parseRoute`'s table arm: after decoding, `const id = decodeURIComponent(m[1]!); return
isSafeId(id) ? { kind: "table", id } : { kind: "door" };` — with the comment above it gaining: "and an id the
store's grammar refuses is the door too (security S2a, D14): `..` would otherwise reach `/api/games/../chat`".
(`isLocal`'s `local-` ids and the server's `sim-` ids pass the grammar; `id_cases.json` carries one of each.)

- [ ] **Step 4: Run the cells, `wire_fixtures` without the flag, the lane's rows** — `npm run test:wasm` (the node
replay's `NOT_SERVED`) and the pool (`THIRD_COLUMN`) among them.

- [ ] **Step 5: The mutants.** **M27** `isSafeId` without the suffix rule — (b) red at `x.table`; **M28** the
alphabet widened to admit a space — (b) red at `a b`; **M29** `parseRoute` not asking `isSafeId` — (c) red first at
the changed `#/t/a%20b` row; **M29b** `isSafeId` asked of the ENCODED segment — (c)'s `%2D` row red; **M29d** the
first eight characters alone judged — (c)'s `.table`-tail row red; **M30** `ID_MAX = 65` — (b) red at the 65-row;
**M31** the fixture's `safe` column inverted in the emitter — the Rust verdict cell red (the emitter is read by the
cell before the client is).

- [ ] **Step 6: Commit.** `git add crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/id_cases.json
web/src/wire/contract.ts web/src/tableId.ts web/src/tableId.test.ts web/src/route.ts web/src/route.test.ts
web/src/local/corpus.node.test.ts worker/test/corpus.test.ts` · `git commit -m "web: the hash router admits the store's id grammar —
tableId.ts pinned both ways to id_cases.json, which wire_fixtures emits from is_safe_id; a link to .. or x.table
lands on the door (S2a Task 5)"`.

---

### Task 6: The deploy tooling — `workers_dev` under every door, the redirect file and `--config`, the secrets listing, the instrument's cookies, the Docker context — Sonnet (C3, D18, D15; ruling A6; decision D-11; Review Focus 5)

**Files:**
- Modify: `worker/scripts/deploy-guard.mjs` (`doorRules` :398–429 — the `open` skip at :406–407 and its comment,
  and `ABSENT.workers_dev`'s reason :210; `configFailures` :605–647 — the sibling rule for an unlisted open-door
  file; `deployGuard` :651–703 — the redirect file; the header comment's check 3 :19–36; the success sentence
  :732–739), `worker/scripts/deploy-guard.test.mjs` (:535 "allows a workers.dev URL at the open door only…"
  rewritten; new cells), `worker/package.json` (:18–20), `web/scripts/pinned-browsers.mjs` (`leaveNoSession`, called
  in place of `await context.close()` in `run` :665; the header comment's cookie paragraph),
  `web/scripts/pinned-browsers.d.mts` (its declaration), `web/scripts/pinned-browsers.node.test.ts` (three cells),
  `deploy/Dockerfile.dockerignore`
- Create: `worker/scripts/secrets-guard.mjs`, `worker/scripts/secrets-guard.test.mjs`
- Test: `cd worker && npm run test:scripts`; `cd web && npm run test:wasm`

**Interfaces:**
- Consumes: nothing of this cycle's other tasks. Task 2 edits `worker/scripts/deploy-guard.mjs` too (`staleAssets`,
  the file's import line, check 9's header comment) and its test; this task touches none of those, and the two tasks
  run one after the other, never in one tree at once (Task 9 Step 3).
- Produces: `REDIRECT_FILE`, the door rule; `FORBIDDEN_SECRETS`, `judgeSecrets`, the `secrets-guard` command;
  `leaveNoSession`.

**The judgment this brief carries, so a Sonnet implementer changes nothing else:** five small deliverables, each
with its cell, each named in the commit; no rule the guard has today is weakened; the deploy commands keep their
order — the guard, then the secrets guard, then wrangler — and every one of the three names its file to both
guards and wrangler; the secrets guard's command is never run by the implementer (it asks the owner's account) —
its cells run it with a fake `npx`.

- [ ] **Step 1: The failing cells.**

(a) `deploy-guard.test.mjs` — the fixture builder `worker(changes, since)` (:165: a passing package laid in a
scratch repository), its `naming(...)` matcher (:132) and its `PRODUCTION`, `OPEN` and `OPEN_FILE` texts (:71, :86,
:97). The cell at :535, "allows a workers.dev URL at the open door only, and refuses preview URLs at both", becomes
— the open file laid into `wrangler.jsonc`'s slot, as the old cell laid it, so that ONLY the skip's removal turns it
green:

```js
  // A workers.dev URL goes around the hostname's Access application at the
  // worker door and, at the open door, around the edge rule, the rate limit and
  // Bot Fight Mode (security S2a, C3): refused under EVERY door. Preview URLs
  // are refused at both: a preview version is not the deployed code, whatever
  // the door. Mutants: the open skip left in (M32); preview_urls exempted at
  // the open door.
  it("refuses a workers.dev URL under every door, and preview URLs at both", () => {
    expect(deployGuard(worker({ "wrangler.jsonc": OPEN.replace('"workers_dev": false', '"workers_dev": true') })), "the open door").toEqual([
      expect.stringMatching(/^wrangler\.jsonc must set "workers_dev": false — got true; .*neither the edge rule, the rate limit nor Bot Fight Mode applies/),
    ]);
    for (const [door, config] of [
      ["worker", PRODUCTION],
      ["open", OPEN],
    ]) {
      expect(deployGuard(worker({ "wrangler.jsonc": config.replace('"preview_urls": false', '"preview_urls": true') })), door).toEqual([
        expect.stringMatching(/^wrangler\.jsonc must set "preview_urls": false/),
      ]);
    }
  });
```

(What the skip's removal closes is production's slot read as `open`, and an UNLISTED open-door file: the committed
`wrangler.open.jsonc` flipped to `workers_dev: true` and judged under `--config` was ALREADY refused, as sibling
drift — measured.) New cells:

```js
  it("refuses a redirect file beside the package or at the repository root, naming it (security S2a, D18)", () => {
    for (const where of ["", "../"]) {
      const root = worker({ [`${where}.wrangler/deploy/config.json`]: '{ "configPath": "../../elsewhere/wrangler.jsonc" }' });
      expect(deployGuard(root, join(root, ".."))).toEqual(naming(/\.wrangler\/deploy\/config\.json exists .* redirect/));
    }
  });

  it("holds an unlisted open-door --config file to production as a sibling (security S2a, C3)", () => {
    // Production's fixture has no TTL key; an open-door file adding one has drifted from production.
    const drifted = OPEN_FILE.replace(
      '"YOKOHAMA_TURNSTILE": "on"',
      '"YOKOHAMA_TURNSTILE": "on",\n    "YOKOHAMA_WEB_TABLE_TTL_DAYS": "30"',
    );
    const root = worker({ "wrangler.public.jsonc": drifted });
    expect(configFailures(root, "wrangler.public.jsonc")).toEqual(
      naming(/wrangler\.public\.jsonc's vars\.YOKOHAMA_WEB_TABLE_TTL_DAYS differs from wrangler\.jsonc's/),
    );
    // The pass-control: the same file undrifted passes — a WRONG `differs` map (the gate's, or an empty one) would
    // refuse it, which `arrayContaining` above cannot see.
    expect(configFailures(worker({ "wrangler.public.jsonc": OPEN_FILE }), "wrangler.public.jsonc")).toEqual([]);
  });

  it("every deploy command names its file to the guard, the secrets guard and wrangler alike (security S2a, D18)", () => {
    const { scripts } = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
    for (const [name, file] of [["deploy", "wrangler.jsonc"], ["deploy:open", "wrangler.open.jsonc"], ["deploy:gate", "wrangler.gate.jsonc"]]) {
      expect(scripts[name], name).toBe(
        `node scripts/deploy-guard.mjs --config ${file} && node scripts/secrets-guard.mjs --config ${file} && wrangler deploy --config ${file}`,
      );
    }
  });
```

(`configFrom` already accepts `--config wrangler.jsonc` for the first script: its regex admits the bare name.)

(b) `worker/scripts/secrets-guard.test.mjs`:

```js
// **The deploy's secrets, judged** (security S2a, decision D-11; the S1 stack review's unverified HI item): the
// deploy guard refuses `YOKOHAMA_DIAGNOSTIC_SEEDS` and `YOKOHAMA_WEB_TEST_CLOCK` in a committed file, and a
// Worker SECRET of either name would reach the shell's `env` past it. `judgeSecrets` reads `wrangler secret
// list`'s JSON and refuses the two by name; the command runs it before `wrangler deploy`. The command cells run
// the script as the deploy does, with a fake `npx` first on `PATH` — no account is asked.
import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { FORBIDDEN_SECRETS, judgeSecrets } from "./secrets-guard.mjs";

/** `wrangler secret list`'s shape: a pretty-printed array of `{name, type}`, never a value. */
const listing = (...names) => JSON.stringify(names.map((name) => ({ name, type: "secret_text" })), null, "  ");

describe("judgeSecrets", () => {
  it("passes the two secrets this Worker keeps, and an empty listing", () => {
    expect(judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", "DEV_DOOR_CLIENT_ID"))).toEqual([]);
    expect(judgeSecrets("[]")).toEqual([]);
  });
  it("refuses each forbidden name, naming it and why — and both, one sentence each, when both are there", () => {
    for (const name of Object.keys(FORBIDDEN_SECRETS)) {
      const failures = judgeSecrets(listing("YOKOHAMA_WEB_ADMINS", name));
      expect(failures).toHaveLength(1);
      expect(failures[0]).toContain(name);
      expect(failures[0]).toContain(FORBIDDEN_SECRETS[name]);
    }
    expect(judgeSecrets(listing("YOKOHAMA_DIAGNOSTIC_SEEDS", "YOKOHAMA_WEB_TEST_CLOCK")), "one sentence per name").toHaveLength(2);
    expect(FORBIDDEN_SECRETS).toHaveProperty("YOKOHAMA_DIAGNOSTIC_SEEDS");
    expect(FORBIDDEN_SECRETS).toHaveProperty("YOKOHAMA_WEB_TEST_CLOCK");
  });
  it("refuses a listing it cannot read rather than passing it", () => {
    for (const bad of ["", "nonsense", "{}", '[{"type":"secret_text"}]', "[1]", "[null]"]) {
      const failures = judgeSecrets(bad);
      expect(failures, bad).toHaveLength(1);
      expect(failures[0]).toMatch(/wrangler secret list/);
    }
  });
});

describe("as a command — the deploy's step, with a fake npx first on PATH, so no account is asked", () => {
  const SCRIPT = fileURLToPath(new URL("./secrets-guard.mjs", import.meta.url));
  /** The script run as `npm run deploy` runs it; `npx` is a shell shim that does what the cell says. */
  const run = (shim, args = ["--config", "wrangler.jsonc"]) => {
    const dir = mkdtempSync(join(tmpdir(), "secrets-guard-shim-"));
    const npx = join(dir, "npx");
    writeFileSync(npx, `#!/bin/sh\n${shim}\n`);
    chmodSync(npx, 0o755);
    const r = spawnSync(process.execPath, [SCRIPT, ...args], { encoding: "utf8", env: { ...process.env, PATH: `${dir}:${process.env.PATH}` } });
    return { status: r.status, stdout: r.stdout, stderr: r.stderr };
  };
  it("passes a clean listing: exit 0 and the sentence", () => {
    const r = run(`echo '${listing("YOKOHAMA_WEB_ADMINS", "DEV_DOOR_CLIENT_ID")}'`);
    expect(r.status).toBe(0);
    expect(r.stdout).toMatch(/2 secrets on wrangler\.jsonc's Worker/);
  });
  it("refuses a forbidden name: exit 1, the name on stderr — the && chain stops here", () => {
    const r = run(`echo '${listing("YOKOHAMA_DIAGNOSTIC_SEEDS")}'`);
    expect(r.status).toBe(1);
    expect(r.stderr).toContain("YOKOHAMA_DIAGNOSTIC_SEEDS");
  });
  it("refuses when wrangler itself fails (no account cached): exit 1, the account's variable named, nothing judged", () => {
    const r = run(`echo 'You are not authenticated' >&2; exit 1`);
    expect(r.status).toBe(1);
    expect(r.stderr).toMatch(/wrangler secret list` failed/);
    expect(r.stderr).toContain("CLOUDFLARE_ACCOUNT_ID");
  });
  it("refuses a banner line before the JSON", () => {
    expect(run(`echo ' wrangler 4.136.3'; echo '${listing("A")}'`).status).toBe(1);
  });
  it("refuses a bad argument list", () => {
    expect(run(`echo '[]'`, ["--config", "../wrangler.jsonc"]).status).toBe(1);
    expect(run(`echo '[]'`, []).status).toBe(1);
  });
});
```

(c) `web/scripts/pinned-browsers.node.test.ts`, three cells — `leaveNoSession` joins the file's import from
`./pinned-browsers.mjs`, and its `Check` type is already imported:

```ts
describe("the profile leaves no session behind (security S2a, D15)", () => {
  /** A context that records what is asked of it; `clearCookies` resolves, or rejects as a closed browser does. */
  const fakeContext = (clear: "resolves" | "rejects") => {
    const calls: string[] = [];
    return {
      calls,
      context: {
        clearCookies: async () => {
          calls.push("clearCookies");
          if (clear === "rejects") throw new Error("Target page, context or browser has been closed");
        },
        close: async () => {
          calls.push("close");
        },
      },
    };
  };
  it("clears the context's cookies, then closes it, and logs the line the window reads", async () => {
    const { calls, context } = fakeContext("resolves");
    const lines: string[] = [];
    const checks: Check[] = [];
    await leaveNoSession(context, (s: string) => lines.push(s), checks, "/tmp/state");
    expect(calls).toEqual(["clearCookies", "close"]);
    expect(lines).toEqual(["cookies: cleared before close"]);
    expect(checks).toEqual([]);
  });
  it("a browser that died first cannot be cleared: one FAILED check naming the profile to delete, and the close still happens", async () => {
    const { calls, context } = fakeContext("rejects");
    const lines: string[] = [];
    const checks: Check[] = [];
    await leaveNoSession(context, (s: string) => lines.push(s), checks, "/tmp/state");
    expect(calls).toEqual(["clearCookies", "close"]);
    expect(checks).toEqual([{ check: "the profile's cookies were cleared", ok: false, saw: "Target page, context or browser has been closed" }]);
    expect(lines).toEqual(["cookies: NOT cleared — delete /tmp/state/profile by hand"]);
  });
  it("is kept out of every image build, with the secrets files — the patterns `**/`, since the files may sit in any directory of the context", () => {
    const ignore = readFileSync(new URL("../../deploy/Dockerfile.dockerignore", import.meta.url), "utf8").split("\n");
    for (const line of ["**/.dev.vars", "**/.dev.vars.*", "**/.env", "**/.env.*", "web/scripts/pinned/"]) {
      expect(ignore, `deploy/Dockerfile.dockerignore lacks ${line}`).toContain(line);
    }
    for (const anchored of [".dev.vars", ".dev.vars.*", ".env", ".env.*"]) {
      expect(ignore, `${anchored} anchored at the context root excludes nothing under web/ or worker/`).not.toContain(anchored);
    }
  });
});
```

(A source-read cell — `indexOf("await context.clearCookies()")` before `indexOf("await context.close()")` — was the
first draft, and the plan review passed it with the call commented out and with the call in a dead branch beside
a comment holding the needle; the recording fake runs the function.)

- [ ] **Step 2: Run them to verify they fail.** `cd worker && npm run test:scripts` — (a) RED (the open fixture
passes with `workers_dev: true`; no redirect refusal; the unlisted file not held; the scripts differ), (b) RED
(no module); `cd web && npm run test:wasm` — (c) RED (no `leaveNoSession`; the ignore file lacks the lines).

- [ ] **Step 3: The code.**

`deploy-guard.mjs`: in `doorRules` (:398–429), the two lines `// An open door has no Access application for a
workers.dev URL to go around.` and `if (key === "workers_dev" && door === "open") continue;` are REMOVED, and
`ABSENT.workers_dev`'s reason (:210) becomes `"with no route, wrangler serves an absent one as a public workers.dev
URL — where neither the edge rule, the rate limit nor Bot Fight Mode applies, so /api/games/<id>/join would take
unlimited guesses (security S2a, C3)"` (the sentence the cell matches: `${name} must set "workers_dev": false — got
true; <the reason>`). The header comment's check 3 (:19–36) says `"workers_dev": false` under EVERY door and why at
each — "under `worker` a `workers.dev` URL would go around the hostname's Access application; under `open`, the
edge rule, the rate limit and Bot Fight Mode (security S2a, C3)" — its "under `open`, a `workers.dev` URL is
allowed, there being no door to go around" removed, and gains a line **3b**: "No `.wrangler/deploy/config.json`
beside the package or at the repository root (`REDIRECT_FILE`, D18); every open-door file judged by `--config` is
held to production as its sibling (C3)". The success sentence (:735) reads "a door named, and its rules: no preview
URL and no workers.dev URL under any door, at the worker door both Access values, at the open door no dev door,
every open-door file production's sibling; no redirect file; …". In `configFailures`, after the two `siblingDrift`
lines (:645–646): an unlisted judged file whose door is `open` is a sibling too —

```js
  // **Every open-door file is production's sibling** (security S2a, C3): a public configuration written later
  // and judged by `--config` is held to production's blocks and vars as the window's file is, not only to the
  // assets and object blocks. (S4's public Worker, its own deployment, will need a rule of its own.)
  if (production && judged && config !== PRODUCTION_FILE && config !== GATE_FILE && config !== OPEN_FILE && doorOf(judged.value) === "open") {
    siblingDrift(config, production.value, judged.value, OPEN_DIFFERS, failures);
  }
```

In `deployGuard`, before `failures.push(...configFailures(root, config))`:

```js
  // **A redirect file would send `wrangler deploy` to a configuration this guard never judged** (security S2a,
  // finding D18). With no `--config`, wrangler looks UP the tree from the working directory for
  // `.wrangler/deploy/config.json` — a build tool's file, which the Cloudflare Vite plugin writes — and follows its
  // `configPath` when the file sits beside the configuration it found: a bare `wrangler deploy` in `worker/` follows
  // `worker/.wrangler/deploy/config.json` (measured, wrangler 4.136.3). One ABOVE that configuration is wrangler's
  // own refusal ("do not share the same base path"), and an explicit `--config` skips the lookup altogether, which
  // is why the deploy scripts name theirs. Both places are refused here — the package's for the hand-run recipes,
  // the repository root's for a run from a directory with no configuration of its own, where the lookup would
  // follow it.
  for (const dir of [root, repo]) {
    if (existsSync(join(dir, REDIRECT_FILE))) {
      failures.push(`${relative(repo, join(dir, REDIRECT_FILE)) || REDIRECT_FILE} exists — wrangler would follow its redirect and deploy a configuration this guard did not judge; remove it`);
    }
  }
```

with, beside `PRODUCTION_FILE`:

```js
/** The redirect file wrangler follows when no `--config` is given — written by the Cloudflare Vite plugin, never by
 *  this repository; named exactly, so a stray one is refused by name (security S2a, D18). */
export const REDIRECT_FILE = ".wrangler/deploy/config.json";
```

and `relative` from `node:path`.

`worker/package.json` (:18–20):

```json
    "deploy": "node scripts/deploy-guard.mjs --config wrangler.jsonc && node scripts/secrets-guard.mjs --config wrangler.jsonc && wrangler deploy --config wrangler.jsonc",
    "deploy:open": "node scripts/deploy-guard.mjs --config wrangler.open.jsonc && node scripts/secrets-guard.mjs --config wrangler.open.jsonc && wrangler deploy --config wrangler.open.jsonc",
    "deploy:gate": "node scripts/deploy-guard.mjs --config wrangler.gate.jsonc && node scripts/secrets-guard.mjs --config wrangler.gate.jsonc && wrangler deploy --config wrangler.gate.jsonc"
```

`worker/scripts/secrets-guard.mjs`:

```js
// **The deploy's secrets, judged before `wrangler deploy`** (security S2a, decision D-11). The deploy guard
// refuses the diagnostic seeds and the test clock in a committed configuration (its rules 4 and 4b); a Worker
// SECRET of either name reaches the shell's `env` past both, since a deploy leaves secrets alone and the guard
// reads files. So the deploy asks wrangler for the names — `wrangler secret list` prints a JSON array of
// `{name, type}`, never a value — and refuses the two. Pure but for the one `execFileSync`, which the command
// alone makes; the cells drive `judgeSecrets` over captured listings and the command over a fake `npx`.
//
// The listing asks the ACCOUNT: every deploy script — `deploy:gate`, the safe direction, included — is a network
// step from here on, a fresh clone needs `npx wrangler whoami` once (or `CLOUDFLARE_ACCOUNT_ID`), and a NEW Worker's
// first deploy cannot pass it (wrangler answers "Worker not found") — S4's public Worker needs its own first-deploy
// path. A failed listing is a refusal, never a pass.
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
    console.error("secrets-guard: `wrangler secret list` failed — run `npx wrangler whoami` once, or set CLOUDFLARE_ACCOUNT_ID; nothing was judged");
    process.exit(1);
  }
  const failures = judgeSecrets(listing);
  if (failures.length > 0) {
    console.error(failures.join("\n"));
    process.exit(1);
  }
  console.log(`secrets-guard: ${JSON.parse(listing).length} secrets on ${args[1]}'s Worker, none named like a switch`);
}
```

`web/scripts/pinned-browsers.mjs`: a new export beside `writeVerdict` (:336), using the file's `check` record (:169)
and its `path` import:

```js
/** **No session leaves with the profile** (security S2a, finding D15): the context's cookies — the deployment's
 *  `CF_Authorization` among them — are cleared before the profile is closed, so the gitignored state directory
 *  holds no credential at rest once a run is done. A browser that died first cannot be cleared: that is a FAILED
 *  check naming the profile to delete by hand, and the close and the verdict still happen — a throw here would
 *  lose `verdict.json` exactly when a run broke (measured: a SIGKILLed browser, no verdict). */
export async function leaveNoSession(context, log, checks, stateDir) {
  try {
    await context.clearCookies();
    log("cookies: cleared before close");
  } catch (e) {
    checks.push(check("the profile's cookies were cleared", false, e instanceof Error ? e.message : String(e)));
    log(`cookies: NOT cleared — delete ${path.join(stateDir, "profile")} by hand`);
  }
  await context.close();
}
```

and in `run` (:665) `await context.close();` becomes `await leaveNoSession(context, log, checks, stateDir);`. The
header comment's "**No cookie VALUE is ever written**" paragraph gains "and the profile keeps no cookie past
`done`: the context clears them before it closes, and a browser that died first is a FAILED check naming the
profile (security S2a, D15)". `web/scripts/pinned-browsers.d.mts` gains:

```ts
/** The close, with the cookies cleared first; a failed clear is a FAILED check, and the close still happens. */
export declare function leaveNoSession(
  context: { clearCookies(): Promise<void>; close(): Promise<void> },
  log: (line: string) => void,
  checks: Check[],
  stateDir: string,
): Promise<void>;
```

`deploy/Dockerfile.dockerignore`, a block after `.vscode/`:

```
# **Secrets and sessions never enter a build context** (security S2a, finding D15) — under BuildKit, which reads
# this file as `deploy/Dockerfile`'s own ignore (the legacy builder reads none; Docker 23 builds with BuildKit by
# default). `.dev.vars` is wrangler's file of credentials (the repository root holds one) and `.env*` is Vite's;
# either may sit in any directory of the context — web/, worker/, the root — so the patterns are `**/`: anchored
# at the root they would exclude the root's alone (measured: `web/.env*` and `worker/.dev.vars` entered the
# context). `web/scripts/pinned/` is the pinned-browser instrument's state, whose Chromium profiles held the
# deployment's Access session cookie, and `COPY web/ ./` in the client stage would have carried it into the image's
# layers. A .gitignore governs git, not a build context.
**/.dev.vars
**/.dev.vars.*
**/.env
**/.env.*
web/scripts/pinned/
```

- [ ] **Step 4: Run the cells; the guard over the real configurations** (`node scripts/deploy-guard.mjs --config
wrangler.jsonc` passes and prints its door line; `--config wrangler.gate.jsonc` the same; `--config
wrangler.open.jsonc`'s no-build checks are `[]` — `configFailures(root, "wrangler.open.jsonc")` in a node one-liner;
the whole guard over the open file refuses a KEYLESS tree at check 12 by design (`YOKOHAMA_TURNSTILE` on, no site
key in the build) and passes over a keyed build); `node scripts/secrets-guard.mjs --config wrangler.jsonc` is the
ORCHESTRATOR's to run (it asks the account); `bash -n deploy/smoke-image.sh` untouched here.

- [ ] **Step 5: The mutants.** **M32** the `open` skip left in `doorRules` — (a)'s first cell red; **M33** the
redirect check reading `root` alone — (a)'s redirect cell red at `repo`; **M34** the sibling rule not run for the
unlisted open file — (a)'s sibling cell red; **M34b** the sibling rule run with `GATE_DIFFERS`, or `{}` — the
sibling cell's pass-control red; **M35** `judgeSecrets` passing a listing that does not parse — (b)'s third cell
red; **M35b** `process.exit(1)` dropped after the failures print — (b)'s command cells' forbidden-name row red;
**M36** `FORBIDDEN_SECRETS` without the test clock — (b)'s second cell red; **M37** `clearCookies()` moved after
`close()` — (c)'s first cell red (the call order); **M37b** the clear commented out — (c)'s first cell red; **M37c**
the `try` dropped — (c)'s second cell red (a throw, no close); **M38** `web/scripts/pinned/` dropped from the ignore
file — (c)'s third cell red; **M38b** the patterns anchored (`.env` for `**/.env`) — (c)'s third cell red.

- [ ] **Step 6: Commit.** `git add worker/scripts/deploy-guard.mjs worker/scripts/deploy-guard.test.mjs
worker/scripts/secrets-guard.mjs worker/scripts/secrets-guard.test.mjs worker/package.json
web/scripts/pinned-browsers.mjs web/scripts/pinned-browsers.d.mts web/scripts/pinned-browsers.node.test.ts
deploy/Dockerfile.dockerignore` · `git commit -m "tooling: workers_dev false under every door and the sibling rule
for every open-door file; a redirect file refused and --config named to both guards and wrangler alike; the deploy
judges the Worker's secrets by name and refuses an unread listing; the pinned instrument clears its cookies and
still writes its verdict when the browser died first; the Docker context excludes the secrets files anywhere in the
tree, and the profiles (S2a Task 6)"`.

---

### Task 7: The documents the code changed — Opus (D10's rule; spec corrections 1–12)

**Files:**
- Modify: `docs/web-lane.md` (§2 counts, §4 standing rules, §7 the release-version rule, §10 operating facts, §13
  the API map, §15 by design, the contract-file list :414, the wake window's recipe :79),
  `docs/plans/2026-10-08-security-remediation-spec.md` (§9.6 appended — never an edit above it), `worker/RUNBOOK.md`
  (§2's commands, the guard's list and the secrets guard's network note; §7's live-tier note and the wake window's
  recipe :803–804 — the dated deploy RECORDS that spell the old command stay as written), `worker/README.md` (the
  deploy commands wherever `npm run deploy` is spelled; the wake window's recipe :1718–1719), `deploy/README.md`
  (one paragraph on the headers; the build line)
- Test: `cargo test -p yokohama-web --test lane_document`, `cd worker && npm run test:scripts` (the
  `lane-variables` cells), and every count re-read at Task 6's head

- [ ] **Step 1: The lane document.** §4's standing rules gain one bullet in the register of the rows around it:
**both hosts serve ONE header set, generated at the client build** (`web/scripts/headers.mjs` → `dist/headers.json`;
the policy with its `connect-src` hosts from the build values, the 404 page's style hashed, `X-Frame-Options:
DENY`, nosniff, the referrer policy, a Permissions-Policy and COOP) — the Worker's asset layer carries it as the
`/*` block the assembly appends to `_headers` and the guard holds (`headers.json` itself is an input of the
assembly, never an asset), and **a `_headers` block never applies to a Worker-generated answer**, so `handle`
marks every `/api` answer `no-store` and `nosniff` itself; the NAS reads the file at start and ONE layer sets the
set on every answer, `no-store` under `/api` beside it; the service worker carries the stored shell's headers onto
every navigation it synthesizes (the type set to HTML, the two encoding headers dropped, `Cache-Control: no-store`
carried), so a returning visitor gets the headers the host set on `/shell.bin`; the entry point refuses to render
inside a frame; **the policy is Report-Only in 0.6.2 and enforced at S2's wrap** — the e2e build alone serves it
ENFORCED, so the local walk is the enforcement reading (Chromium logs a Report-Only violation at `info`, which the
walk would not read) — and Bot Fight Mode's injected detection script is its one known line (A1: never
`'unsafe-inline'`; the wrap decides between accepting the refusal, a fixed nonce and `no-transform` from the
reading, and the policy carries no `report-to`, so the reading is the window's own sessions); the hash router admits
exactly the store's id grammar, pinned to `id_cases.json` as the name rule is pinned to `identity_cases.json`. §7:
the tag is cut on the commit the hosts run — master's, or, on a stack that merges at its wrap (security S2), the
deployed branch commit, which the wrap's merge brings onto master (the owner, 2026-10-11: D-7); its version line
reads 0.6.2. §10's operating facts: the deploy commands name their file to the guard, the secrets guard and wrangler
alike, the guard refuses a redirect file and a `workers.dev` URL under every door, and every deploy script asks the
account for the Worker's secret NAMES before it ships (a network step; a fresh clone runs `npx wrangler whoami`
once); the pinned instrument clears its cookies at `done`; a `curl` at an Access-protected host OVER WARP carries a
browser `User-Agent` or meets Bot Fight Mode's 403 (the owner, 2026-10-11), and off WARP is not a transport at all
(W-1). §13's rows: the Worker's `/api` answers carry `no-store` and `nosniff` (B9's Worker half, DC-F4), the NAS's
the whole set; the static release's headers on both hosts. §15 gains four rows: **D16 `accepted`** with the owner's
sentence and its two consequences; **B6's token duration `accepted`** (the owner's sentence; the ledger row is S2b's
to rewrite); the Report-Only release and the known line; `_headers`' rule and why the shell writes its own two. The
contract-file list (:414) gains `id_cases.json`; the wake window's recipe (:79) reads `node scripts/deploy-guard.mjs
--config wrangler.jsonc && node scripts/secrets-guard.mjs --config wrangler.jsonc && npx wrangler deploy --config
wrangler.jsonc --var YOKOHAMA_WEB_COUNT_WAKES:on`. §2's counts re-read at Task 6's head (the web lib, `start_refusal`,
`wire_fixtures`, vitest, `test:wasm`, `test:scripts`, the pool, the e2e count — each with the head it was read at).
`lane_document`'s cells and `lane-variables.test.mjs` run.

- [ ] **Step 2: The spec's §9.6**, appended under §9 after §9.5, dated 2026-10-11, titled "S2's brainstorm and
S2a's plan: the three-cycle split, the owner's twelve rulings and the four taken at the plan review, eleven
decisions, six adjudications and twelve corrections": the rulings section of this plan verbatim (the plan review's
two bullets included), the decisions D-1–D-11 verbatim, the spec corrections 1–12 verbatim; the run's rulings
paragraph is the close's (Task 9 Step 6), as §9.5's was S1c's. Nothing above §9.6 is edited.

- [ ] **Step 3: The runbooks and the two READMEs.** `worker/RUNBOOK.md` §2: the command block gains the two other
scripts as `package.json` spells them (today it holds `npm run deploy` with its one-line gloss); the guard's list
gains the redirect file, `workers_dev` under every door, the sibling rule for every open-door file, the `_headers`
expectation (static plus the current build's block) and the secrets guard's step with its sentence — and ONE
paragraph on what the secrets guard costs: every deploy script is a network step from here on, a fresh clone runs
`npx wrangler whoami` once (or sets `CLOUDFLARE_ACCOUNT_ID`), and a NEW Worker's first deploy cannot pass it
("Worker not found"), so S4's public Worker needs its own first-deploy path; the hand-run recipe of §7's wake window
(:803–804) names `--config wrangler.jsonc` to the guard, the secrets guard and wrangler, as the lane document's does
(the dated deploy records that spell the old command are records, and stay). §7 gains, beside Bot Fight Mode's 403:
every `curl` at either host OVER WARP carries a browser `User-Agent` (the smoke's `BROWSER_UA`), and off WARP a bare
client meets the 403 whatever its `User-Agent` (W-1) — and the live tier's third forgiveness: the Report-Only
refusal of the injected inline script, Chromium's `info` line read and counted apart, printed with the beacon's
and the feed's counts. `worker/README.md`: the deploy commands wherever spelled (`npm run deploy`'s gloss :1614; the
open-door paragraph :365–371), and the wake window's recipe (:1718–1719) with `--config` and the secrets guard.
`deploy/README.md`: one paragraph — the NAS reads `dist/headers.json` from the client build it serves and sets the
set on every answer; a dist without the file serves none and the binary says so once at start; a malformed file
refuses to start — and the build line says "BuildKit (Docker 23's default), which reads `deploy/Dockerfile.dockerignore`".

- [ ] **Step 4: Commit.** `git add docs/web-lane.md docs/plans/2026-10-08-security-remediation-spec.md
worker/RUNBOOK.md worker/README.md deploy/README.md` · `git commit -m "docs: the lane document's S2a rows (the
header set, the Worker's own two, the pass-through, the id grammar, D16 and the token accepted, the tag on the
deployed commit), the spec's §9.6, the runbooks' commands and recipes, the secrets guard's cost and the curl
User-Agent note (S2a Task 7)"`.

---

### Task 8: The release bump to 0.6.2 — Opus (ruling X2; decisions D-7, D-10; `docs/web-lane.md` §7)

**Files:**
- Modify: `Cargo.toml` (`[workspace.package] version` :16), `Cargo.lock` (the nine members' lines),
  `web/package.json`, `web/package-lock.json`, `worker/package.json`, `worker/package-lock.json`,
  `web/src/wire/health.json` (`server_version`), `web/src/wire-script/archive-walk.json` (its seven records'
  version) and `web/src/wire-script/backup-envelope.json` (`app_version`) — the two files the regenerations rewrite
  (measured: `sed 's/0.6.2/0.6.1/'` reproduces HEAD byte for byte) — `deploy/Dockerfile.runtime:18` (the example
  tag), `deploy/README.md` (the four example tags `0.6.1-s1` → `0.6.2-s2a`, :114, :135, :173, :211)

- [ ] **Step 1: The bump, as §7 measured it.** (1) `Cargo.toml`'s version to `0.6.2`, then `cargo update
--workspace --offline` — the nine members' lock lines and nothing else (read the diff). (2) `npm version 0.6.2
--no-git-tag-version --ignore-scripts` in `web/` and in `worker/`. (3) `UPDATE_WIRE_FIXTURES=1 cargo test -p
yokohama-web --test wire_fixtures` and `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup`; READ the
diff: the version and nothing else — any other byte is a finding reported `DONE_WITH_CONCERNS`. (4)
`deploy/Dockerfile.runtime`'s example tag `0.6.2-s2a`, and `deploy/README.md`'s four example tags the same. (5)
`cd web && npm run build:wasm` and `cd worker && npm run build:module:debug` BEFORE the suites.

- [ ] **Step 2: Every suite, at this head.** The per-edit suites of `CLAUDE.md` and every row of
`docs/web-lane.md` §2, `npm run e2e` included (the client's version string moved) — recorded with counts in the
task report; `the_two_npm_packages_carry_the_workspace_version` green; `cargo test -p yokohama-cli` reads **862**
and 2 ignored (the CLI is untouched; its version string is the workspace's).

- [ ] **Step 3: Commit.** `git add Cargo.toml Cargo.lock web/package.json web/package-lock.json
worker/package.json worker/package-lock.json web/src/wire/health.json web/src/wire-script/archive-walk.json
web/src/wire-script/backup-envelope.json deploy/Dockerfile.runtime deploy/README.md` (every file `git status` shows
the two regenerations rewrote, each named in the task report — the two wire-script files are the measured set) ·
`git commit -m "release: 0.6.2
— security S2a (the browser's fences; the header set in Report-Only); the wire's server_version and the envelope's
app_version regenerated, the version alone (S2a Task 8)"`.

---

### Task 9: The pre-flight, the branch, the dispatches, the suites, the after review, the close, the PR, the tag, and the first of S2's two windows — ORCHESTRATOR (spec §5, §9.2; rulings X1, X2, X4, X5, A1; decisions D-7, D-8, D-9)

- [ ] **Step 1: The pre-flight — what the plan review left of the orchestrator's "before" review (X4).** The plan
  review of 2026-10-11 (§"Review record") executed Tasks 1 and 2's cells and read M1–M13 on four exports, measured
  the header bytes, the assembled `_headers` and the block as the asset layer parses it, tried the framed cell, read
  the guard over the real configurations and ran a release-client subset of the Playwright tier under `enforce` on
  the real NAS binary; its corrections are at this plan's source. What remains, on a `git archive` export of master
  `1e9c092d` (`scratchpad/export-1e9c092d`, `node_modules` linked in, Rust on a PRIVATE `CARGO_TARGET_DIR`): (a)
  **Tasks 1 and 2's CORRECTED cells executed once more** against scratch implementations of this plan's blocks (lifted
  by script, as S1b's `blocks.py`), every named mutant M1–M13 with its lettered variants read red at least at the cell
  it names, every other red cell recorded; (b) **the DEBUG-client `npm run e2e` under `enforce`** — Task 4's
  `e2e-build.mjs` override spliced, the client built, the local binary with Task 3's layer spliced in — every console
  line recorded (what enforcement blocks TODAY, the input to the wrap's decision, D-9), then the same tier with
  Task 4's widened filter under `report-only`, which must be green; (c) `cargo clippy -p yokohama-web
  --no-default-features --features server --all-targets -- -D warnings` with Task 3's gated `get` spliced in (the lib
  as CI compiles it); (d) every cell that cannot go red on its mutant, every name here the tree does not spell, every
  sentence of the spec's rows or this plan the code contradicts — fixed in this plan at the source BEFORE Task 1 is
  dispatched; the scratch discarded with the export; the findings file
  `docs/temp/security-review-2026-10-08/s2a-before/findings.md`. Tasks 3–8's cells are the reviewers' (X4).

- [ ] **Step 2: The branch.** `git checkout -b security-s2 1e9c092d` (= master); the plan and the board committed on
  it first ("docs: S2a's plan and the board…"); S2a's commits stack on it; PR #52 (`security-s2` → `master`)
  opened at Step 7, merged at S2's wrap (X1).

- [ ] **Step 3: The dispatches.** Tasks 1, 2, 3, 4, 7, 8 Opus/Opus; Tasks 5 and 6 Sonnet/Opus (X3); each reviewer
  rerunning the task's mutants on an export, Rust on a private `CARGO_TARGET_DIR`; the probe-and-name-concerns
  paragraph in every implementer brief; "no call to the Cloudflare account, read-only included" beside the removal
  rule in every brief; the SDD ledger under `docs/temp/security-review-2026-10-08/s2a-sdd/`; the heartbeat protocol
  for every wait past an hour. A `DONE_WITH_CONCERNS` correctness item is ruled on and fixed as a new commit before
  the review; a fix round quotes its hunks; a reported SHA is never amended. The order: Task 1 first; Tasks 2 and 3
  may run in parallel once Task 1 is reviewed (they touch no common file); Task 4 after Task 3's commit (its e2e
  cells read the binary); Task 5 in parallel with any of them (its files are its own); **Task 6 after Task 2's
  commit** — both edit `worker/scripts/deploy-guard.mjs` and its test, in different functions, and two agents in one
  tree lose each other's edits (the M6.5c lesson); Task 7 after Tasks 1–6 (§2's counts are re-read at Task 6's head);
  Task 8 last (it shares `worker/package.json` with Task 6).

- [ ] **Step 4: The per-edit suites and the roster at the code head (after Task 8)**, in this order, every row rc 0,
  each count against the board's "Gates at HEAD": `cargo test -p yokohama-core` (unmoved: 1,587 / 29 ignored) ·
  `cargo test -p yokohama-cli` (862 / 2 — untouched) · oracle · difftest · `eval_probe` · `solver` · bridge and its
  four node suites · `cargo clippy --workspace --all-targets -- -D warnings` · **the web lane's rows**
  (`docs/web-lane.md` §2), `wire_fixtures` green WITHOUT the flag, `npm run e2e` included. **Then the fifteen-gate
  roster, ONCE, as one uninterrupted sequence, release binaries pre-built** (spec §5.2: "any cycle that bumps the
  version" — the plan review's AD-3; S1c ran it at `04713bed`, after its own bump, and the S1 wrap's non-re-run
  rested on that run), its machine-tagged wall recorded beside `04713bed`'s 484 s; core and the CLI are
  byte-identical (§"Global Constraints"), so a gate that moves is a finding. Record every count; a count that
  differs is a finding.

- [ ] **Step 5: The "after" review — the orchestrator's own**, whole-branch, on a `git archive` export of the
  branch head: the fences of §"Global Constraints" (`git diff 1e9c092d -- crates/yokohama-core crates/yokohama-oracle
  crates/yokohama-difftest crates/yokohama-bridge crates/gen-data crates/yokohama-wasm crates/yokohama-worker
  crates/yokohama-cli` empty; `SEMANTICS_REV` 7); the corpus moved by ONE new file (Task 5) and Task 8's version
  fields, each diff read; every mutant M1–M38 and their lettered variants re-run; the argument of each row re-derived
  from the code — that no directive carries `'unsafe-inline'` or `'unsafe-eval'` (`'wasm-unsafe-eval'` is required),
  that the set's names on `/shell.bin` on the local binary and in the assembled `_headers` are the same six, that
  `handle` wraps every `/api` answer and no other, that `response_headers` is a `layer` over the fallback, that
  `shellResponse` copies all but three, that the e2e build serves the policy ENFORCED while the shipped `CSP_MODE` is
  `report-only`, that `parseRoute` asks `isSafeId` on the DECODED id, that the guard's `open` skip is gone, that
  `leaveNoSession` clears before it closes and closes when it cannot clear, that the deploy scripts name one file
  three times. One fix wave as new commits; a scoped re-review of the fix.

- [ ] **Step 6: The close's documents.** The ledger (`docs/security-ledger.md`): **B4** `fixed <commit>` — the
  header set on both hosts (Tasks 1–3's cells named), the `window.top` refusal and the pass-through (Task 4's),
  the Access half done 2026-10-11 at S1's wrap; `frame-ancestors` Report-Only until the wrap, `X-Frame-Options`
  enforced now (correction 11); **D13** `fixed` (Task 4's node cell); **B9**'s row gains its Worker half `fixed`
  (Task 2's pool cell; DC-F4); **D14** `fixed` (Task 5's three cells); **D15** `fixed` (Task 6's recording-fake
  cells and the ignore cell); **D16 `accepted (the owner, 2026-10-11): a finished game uploads whoever restored it —
  "even if they pull back a game and finish it … it's still useful to get more completed games, because we're using
  those for training detail later on with the v5+ computer player development"; the upload counts against the
  restorer's share (a 507 is a wait), and a restored copy may sit beside the original device's under another
  segment, deduplicated by digest when the archive is read`**; **D18** `fixed` (Task 6's redirect cell and the
  scripts cell); **C3** `fixed` (Task 6's door cell and the sibling cell); the `Updated:` line for 2026-10-1x.
  guidingpath's decision-log row for S2a's close and its current-state sentence (S2a on the stack; the window's
  state); the board's Track 4 (S2a landed; the window; S2b next); `docs/web-lane.md` §8's "What the windows did NOT
  read" paragraph, once rows 108 and 119 are read (Step 9(d)); `CLAUDE.md` at most one standing sentence, if one is
  genuinely new — the candidate: "Both hosts serve ONE header set generated at the client build
  (`web/scripts/headers.mjs` → `dist/headers.json`): the Worker's asset layer carries it as `_headers`' `/*` block
  and `handle` marks every `/api` answer `no-store` and `nosniff` — a `_headers` block never applies to a
  Worker-generated answer — the NAS's one layer sets it on every answer, and the service worker carries the stored
  shell's headers onto every navigation it synthesizes; the policy is Report-Only until S2's wrap, and the e2e build
  alone serves it enforced, since Chromium logs a Report-Only violation at `info`, which no error watcher reads
  (security S2a, 2026-10-11)"; this plan's §"Review record" with the readings. The project memory updated the same
  way.

- [ ] **Step 7: The push and the PR.** `git push origin security-s2`; PR #52 (`security-s2` → `master`) with the
  cycle's section (the rulings; the eight rows and their catchers; the one new corpus file; the suites' readings,
  the e2e and the roster included; the mutants' readings; the twelve spec corrections; "the merge at S2's wrap").
  Nothing merges.

- [ ] **Step 8: The tag and the release — BEFORE either deploy** (D-7, taken by the owner; the owner's standing order
  from S1's wrap). On the branch's deployed commit (Task 8's, or the fix wave's head): the annotated tag `v0.6.2`,
  subject `Yokohama CLI v0.6.2 — security S2a: the browser's fences, the header set in Report-Only`, pushed;
  `release.yml` builds and publishes (it runs on any `v*` tag push, no branch test — read at the plan review); the
  release's assets read on GitHub before Step 9 begins. The wrap's merge brings the tagged commit onto master.

- [ ] **Step 9: The window — the orchestrator's alone, both hosts, one window; the owner at the keyboard for the
  block.** **It opens by naming the two-identity block (d) and agreeing its slot with the owner** (the owner's ruling
  on D-8, 2026-10-11: the two Access sign-ins are the owner's) — recorded before any deploy begins, as is its
  precondition: Claude in Chrome reachable in both profiles, WARP off for the slot. Every `curl` here carries a
  browser `User-Agent` (the release smoke's `BROWSER_UA`) and runs OVER WARP (the owner, 2026-10-11); off WARP a
  `curl` is not a transport (W-1), and the block uses none. **The quota guard first** (`worker/RUNBOOK.md` §7 — the
  UTC day's Worker requests and Durable Object rows through GraphQL, `workersInvocationsAdaptive` and
  `durableObjectsPeriodicGroups`), read again after the live Playwright tier; a run that could cross the owner's
  80,000 line (I-82) HOLDS to the next UTC day. (a) **The NAS**: **the update reading, pinned FIRST** — `node
  web/scripts/pinned-browsers.mjs nas https://yokohama-nas.moore.nyc web/scripts/pinned/nas-s2a --go-wait 90` (a
  fresh state directory), its `PINNED` read before the NAS stops; then the runtime image from the tagged commit
  (`deploy/RUNBOOK.md` §1–§3; the tag `0.6.2-s2a`), deployed through Portainer; health reads `server_version: 0.6.2`;
  **the four header reads Task 3 gave the smoke, taken LIVE right after the health read** and before the update
  reading's `go` (`curl -sS -A "$UA" -D - -o /dev/null` on `/` and `/api/health`): `x-frame-options: DENY`, the
  policy present under `content-security-policy-report-only`, `/api/health`'s `cache-control: no-store` and
  `x-content-type-options: nosniff` — the NAS has no local image to smoke (its image is assembled by Portainer; AD-4);
  then `touch web/scripts/pinned/nas-s2a/go`, and `touch …/done` once its checks print. (b) **The Worker**: `cd web
  && npm run build` (with the site URL), `cd worker && npm run build`; **the update reading pinned** — the same
  instrument, `worker https://yokohama.moore.nyc web/scripts/pinned/worker-s2a --go-wait 90` — before `npm run
  deploy` (the dev door ON for the live tiers; the secrets guard's first live run prints its sentence); health reads
  0.6.2; `go` after the health read, `done` once its checks print; **the live corpus replay TWICE** after its reset
  recipe (`worker/RUNBOOK.md` §7), HOLDING; **the live Playwright tier once** (`two_browsers`, `joins_by_link`), its
  `reportOnlyForgiven` count printed beside the beacon's and the feed's; the quota guard read again; each update
  reading's `verdict.json` reads "every check held", its `log.txt` holds the `cookies: cleared before close` line
  before its final `closed:` line, and its profile's `Default/Cookies` holds no row; then `npm run deploy:gate` last.
  (c) **The live acceptance** (spec §5.3 and this cycle's; every probe that creates state removes it): the response
  headers of `/`, `/shell.bin`, the entry chunk (`/assets/index-<hash>.js`, whose hash is new in 0.6.2 — an unchanged
  hashed asset may answer from the edge cache with its OLD headers) and `/api/health` on BOTH hosts (`curl -sI -A
  "$UA"`), against spec correction 10's expected set, and the 404 page (`/nothing-here`: its policy names the style
  hash, D-4's one live reading); **B4** — a page on another origin framing `https://yokohama.moore.nyc/#/lobby` and
  the NAS's lobby, opened in Chrome (Claude in Chrome) — if Access answers the framed request (a cross-site frame
  carries no `SameSite=Lax` cookie), the app's own header is read from the headers row and the frame read from a
  sibling `*.moore.nyc` page if one is at hand; the frames refused, nothing acts; **D13** — on a FRESH profile (an
  existing install takes the new worker only through the update offer), a SW-served navigation's headers read in
  DevTools' network panel on each host (the synthesized response carries the set, `no-store` among it); **the
  Report-Only reading** — a FRESH Chrome profile's first visit to each host, its console read at EVERY level, Verbose
  included: two `info` lines at the page per first visit expected ("Executing inline script violates the following
  Content Security Policy directive 'script-src …' … The policy is report-only, …" — Cloudflare's detection script,
  one injection, two lines), anything else a finding named in the close; a second navigation (SW-served) prints
  nothing; the real line's text recorded against `REPORT_ONLY_INLINE_SCRIPT` (a red live tier is a finding, never a
  hole); **B9's Worker half** — a table answer on the Worker reads `cache-control: no-store` and
  `x-content-type-options: nosniff` (DC-F4 closed live); **D14** — `#/t/..` typed in the address bar lands on the
  door; **A1 and B3 re-read** as regressions (cheap: `suggest` carries no seed-like key; a seeded create beside an
  open seat is 400). (d) **The two-identity block (D-8; the owner's ruling: live, in the slot agreed at the window's
  start)**: WARP off; Chrome profile A signed in as the owner and profile B as `myfreemovies@gmail.com` through
  Access's login on the NAS hostname (and on the Worker's, for the §8 rows); every probe a page-context `fetch` or
  `EventSource` from the profile's own tab (Claude in Chrome's `javascript_tool`) — the cookie never extracted,
  printed or logged; the `umask 077` file as the named fallback (D-8). **D1** — A creates `[A, open]` (no lock: an
  open table admits any stream, `api.rs:2082`); B opens the table's stream from its tab; A joins seat 1 ITSELF (a
  person may hold two seats): B's stream ends after one `gone` frame (the cell is
  `a_refused_stream_writes_gone_once_and_closes`; the ledger's D1 row names exactly this end). **B1's withheld
  `why`** — A creates `seats: [{"bot": true}, {"user": A}]` with `start_seat: 0` and keeps (`POST …/keep {"seat": 1,
  "card": 1}`): the bot's first turn is pumped at once, no driving; `GET …/why` read by B as a watcher, and by A as
  its opponent, answers the typed `withheld` account (the move played, the claim, one sentence, no search values);
  the game-over half (the holder served) stands on
  `why_is_withheld_while_the_bots_starting_order_is_hidden_and_served_once_it_is_public` and is not read live (about
  130 human moves). **The web-lane §8 rows 108 and 119** read as B on the WORKER host after B's sign-in there (B's
  cookie is the hostname's): 108, a second signed-in address at a single-audience table; 119, `GET /api/admin/tables`
  as B — the operator gate's refusal of a non-operator; the second identity reads both, no third is needed. Every
  probe table deleted, and any the server filed under `rev7/abandoned/` removed by name (`deploy/RUNBOOK.md` §3.21's
  recipe); WARP back on, `warp-cli debug access-reauth` if Access answers "Please authenticate via the warp client";
  the fallback file, if one was made, deleted. (e) **The ledger's live column** filled for every S2a row; the deploy
  records (`deploy/RUNBOOK.md` §3.22, `worker/RUNBOOK.md` §22) name the tag, both `server_version` readings, the
  header reads, the Report-Only reading's lines, the block's slot and its two readings and the §8 rows; **the
  rollback bullets**: `wrangler rollback` to `fd34c477` (the gate, 0.6.1) and the NAS's `yokohama-web:0.6.1-s1` — the
  headers are stateless, nothing in this window is one-way.

- [ ] **Step 10: The hand-off into S2b.** The board's Track 4 rewritten: S2a landed on `security-s2` (the head,
  PR #52 open, the tag, the window's readings; the Report-Only reading as the wrap's input, with the facts the plan
  review added to A1's decision — the policy carries no `report-to`, so the reading is the window's own sessions,
  and Cloudflare injects no script under `Cache-Control: no-transform`, a fourth shape to weigh); the next session's
  first step — "S2b's brainstorm and plan, against `security-s2`'s head: B5, B6's `.test` arm (the token
  `accepted`), C1 with D24, C2 (the GA `ratelimits` binding; the figures from the measured calls per move for a
  couple of games at once; `routableName` widened to `is_safe_id`), D3's socket cap, B12's Worker object quota
  (the NAS's figures), D23 (what the shell logs; `head_sampling_rate`), the `escaped-key` and 48-cut WASM cells,
  the crafted-name cell (correction 9), an unlisted WORKER-door `--config` file's sibling rule (outside A6's text —
  the plan review's F-40), S4's first-deploy path past the secrets guard; the owner's recommendations-as-default
  ruling"; the project memory updated the same way; `docs/temp/security-review-2026-10-08/s2a-sdd/` and
  `s2a-before/` left in place with every report. The next session begins at the board.

---

## Review record

### The plan review (2026-10-11) — four Sonnet lenses on exports, one Opus assembly, the orchestrator's adjudication

**Method.** The owner's shape ("two to four sonnet agents, with an opus agent to assemble their results back in
something it can hand back to you with the context for adjudication and plan addendum"). Four Sonnet reviewers ran
in parallel, one lens each — the header set and the two hosts (Tasks 1–3), the client and the browser (Tasks 4–5),
the deploy tooling and the window (Tasks 6, 8, 9), and the documents against the code (every task) — each on its own
`git archive` export of `1e9c092d`, this plan's blocks lifted by script, spliced where the plan says and EXECUTED;
every platform claim that decided a design was measured with a minimal harness (Chromium 143–153 and Chrome 155
under Playwright 1.62.1; workerd through wrangler 4.136.3; the Worker pool; the real NAS binary; BuildKit 0.33.1;
wrangler dry-runs with no account). One Opus assembler deduplicated by mechanism, read every contradiction at the
code (read-only, `1e9c092d`, and wrangler's own source for the redirect lookup), ranked, and wrote each correction in
the plan's words with six questions for adjudication. The orchestrator re-ran the two probes whose readings decide
designs — the console level of a Report-Only violation across five Chromium builds, and which framers observe an
`X-Frame-Options` refusal of a loopback host — read both as the lenses had, ruled the six, and spliced every
correction at the source (four scripts of exact-match replacements; the plan grew from 2,264 to about 2,900 lines).
The reports and the packet: `docs/temp/security-review-2026-10-08/s2a-plan-review/` (`assembled.md` carries the
eleven contradictions and how each was settled; `addendum-draft.md` the assembler's draft of this entry). **86
findings, 56 after the dedup — 1 Critical, 25 Important, 22 Minor, 8 Information; 5 unverified, two of them in
part.** Nothing shipped a hole: the Critical was a false fence, and every Important a cell that could not catch its
mutant, a sentence an implementer would have transcribed, or a step that could not run as written.

**Readings.** The architecture held under measurement: the `/*` block rode every static answer of the Worker's asset
layer — `/` through the `_redirects` rewrite, `/shell.bin` and its HEAD, both 404 shapes, a 304 — and no Worker-made
answer, nine static requests for zero invocations; the NAS layer set the set on every answer, the SSE heads
included; the service worker's pass-through closed a real same-site frame that today's worker renders;
`store::is_safe_id` and the plan's `isSafeId` agreed on 46,017 strings with 0 differences. The set adds 765 B to a
static answer on the HTTP/1.1 wire (the policy's line 536 characters, its value 497) and 58 B to a Worker `/api`
answer; the `_headers` block is 774 B; 153 precache requests × 765 B is 1.2% of the 10 MB release. The built client
under the policy ENFORCED passed 77 of 81 e2e cells (a release-client subset) on the real NAS binary, the other four
red identically with no header set or a flake that passed 3/3 — nothing the covered flows need is blocked today. With
the corrections spliced: the web lib 494 → 497, `start_refusal` 5 → 6, `wire_fixtures` 17 → 18, the pool 224 → 226,
vitest 1,507 → 1,511; `test:wasm` 353 → 363–367 and `test:scripts` 543 → 547–553 by splice. The named mutants M1–M38
went red where named but M2 (its catcher red with no mutation), M6 (undiscriminated until the real-build cell's own
list moved), M11 (one catcher, not two) and M12 (about fifty cells, not one); every survivor the lenses found — the
build's wiring, an allow-list, a commented-out call, the guard's exit, `.hostname`, a static rule naming a set header,
the encoded id, a wrong sibling map, two `judgeSecrets` rows — now has its catcher and its lettered mutant.

**The Critical (F-1; HH-F1, CB-1).** D-9 rested on a browser behaviour that does not exist. Chromium 143–153 and
Chrome 155 log a Report-Only violation at console type `info`, worded "Executing inline script violates the following
Content Security Policy directive … The policy is report-only, so the violation has been logged but no further action
has been taken." with no `[Report Only]` prefix; `watchForBreakage` read `error` alone; and a violation inside the
module Worker printed on no channel in either mode — so the local walk under the shipped policy would have read
nothing, the forgiveness was unreachable and matched no real line, and the window would have read two `info` lines,
not one. The orchestrator re-ran the lens's probe: `[info]` under Report-Only and `[error]` enforced on every build.
**Ruled (AD-1): both halves** — the e2e build serves the policy ENFORCED through an override in `e2e-build.mjs` that
can only strengthen (pinned by the framed cell's policy assertion), and the live tier reads a CSP line at any console
level, forgiving exactly the measured Report-Only inline-script line at the deployment's own ORIGIN, counted apart.
Only an enforced local walk sees a violation raised inside the module Worker, and only the widened filter makes the
live tier the counted reading of the Report-Only release the wrap's decision needs.

**The Importants, each corrected at the source.** F-2 the framed cell could not pass: Chromium's Local Network Access
check stops an `about:blank` framer of `127.0.0.1` before the host answers (re-run by the orchestrator: variants A, D,
E blocked, B and C — a loopback framer — read the 200 and its `DENY`), so the framer became a second loopback origin.
F-3 `main.tsx`'s refusal had no catcher — a second e2e cell strips the host's fences and reads the sentence. F-4 the
build's wiring survived three mutants — the stamp cell reads each build's `headers.json` and 404 page. F-5 the
pass-through cell passed the allow-list D-5 refuses — a header no cell names is carried. F-6 the Docker-ignore
patterns were anchored at the root, and BuildKit let `web/.env*` and `worker/.dev.vars` into the context — `**/`.
F-7 the D15 source-read cell passed a commented-out call — a recording fake over `leaveNoSession`. F-8 the secrets
guard's command half had no cell — five with a fake `npx`, and a failed listing is a refusal. F-9 the inserted
`clearCookies()` lost the verdict on a dead browser — a guarded tail. F-10 the update readings were ordered after the
deploys they straddle — pinned first, `go` after the health read. F-11 no quota guard preceded the metered live tiers.
F-12 the NAS smoke had no image to run on — **ruled (AD-4): the four header reads are taken live after the health
read**, and D-1's parenthetical names the self-host image's CI smoke. F-13 the two-identity block's readings named no
means that runs — D1 respelled at an open table A fills itself, B1 at a bot-first table, rows 108 and 119 as the
second identity on the Worker host. F-14 "no common file" was false for Tasks 2 and 6 — serialized. F-15 the live-tier
cell's file would have taken down CI's browserless job — moved to `harness.node.test.ts`. F-16 the no-unsafe cell was
red against the plan's own policy (`'wasm-unsafe-eval'`) — a quoted-keyword match. F-17 the real-build cell was red
for the wrong reason — its own list and count move. F-18 `headers.json` would have landed in the Worker's assets and
moved the edge rule — **ruled (AD-2): skipped in the copy loop, an input of the assembly and never an asset**. F-19
four unlisted Task 2 edits. F-20 the pool cells failed the typecheck and one answered 409. F-21 `tempfile` was not a
dependency — the crate's own scratch idiom. F-22 the notice in the library tripped the log seam — `production` reads
the file, `main.rs` prints the line. F-23 the binary cell could not run — the harness's own names, `listening` before
each request, the `open-door` gate. F-24 two Task 4 blocks failed the typecheck. F-25 Task 5 named a gitignored file
and missed the two corpus-replay lists. F-26 (unverified) the block's transport rested on a mechanism the
repository's W-1 contradicts, and the cookie is `HttpOnly` — **ruled (AD-5): page-context probes inside each
signed-in profile as the default, a `umask 077` file as the named fallback, nothing extracted into a tool result**;
the owner's User-Agent rule stands for every `curl` the window runs over WARP, beside W-1's reading of what a `curl`
off WARP meets.

**The Minors and the Information**, each at the source: six sentences about the mutants (F-27); the cargo filter after
`--` (F-28); a port row (F-29); the 404 style hashed after CR normalization (F-30); `parseHeadersJson` refusing what the
Rust reader refuses (F-31); the static `_headers` naming none of the set (F-32); Review Focus 3's 403 and 500 dropped,
the wrapper being status-blind (F-33, ruled: the sentence, not two rows the pool cannot stage); the Kelvin sign and
the zero-width space as `\u` escapes (F-34); two route rows pinning the DECODED id (F-35); three `forgiven` prints,
the return type, `FrontDoor.tsx` (F-36); the log line's name and place (F-37); the open file's keyed-tree nuance
(F-38); the D18 comment saying what wrangler does — a redirect beside the package is followed, one above it is
wrangler's own refusal, `--config` skips the lookup — and the recipes given `--config` (F-39); the sibling cell's
pass-control (F-40); `REDIRECT_FILE`'s comment, the header comment, the reason string, the `:535` cell printed whole
(F-41); two `judgeSecrets` rows (F-42); `docs/web-lane.md` §7 recording D-7's exception (F-43); Task 8's two
wire-script files and the README's tags (F-44); thirteen line references (F-45); §8 moved to the close, the runbook's
command block, the smoke's presence read (F-46); spec corrections 11 and 12 (F-47) and **the roster — ruled (AD-3):
run once at S2a's code head after Task 8**, as S1c did at `04713bed` after its own bump; D13 as a same-site exposure
and a fresh-profile reading (F-48). Information: the keyless bundle never uses `challenges.cloudflare.com` and the
policy carries no `report-to` — D-2 stands as the owner took it, and the no-report and `no-transform` facts go to the
wrap's decision (F-49); a shared `CARGO_TARGET_DIR` is not isolation (F-50); S1's two pinned profiles hold live Access
cookies until 2026-10-12 near 11:05Z, left to lapse unless the owner says otherwise (F-51); `wrangler secret list`
makes every deploy script a network step and cannot pass a new Worker's first deploy — recorded for Task 7 and S4
(F-52); the window reads the entry chunk and the 404 page (F-53); the pass-through carries `/shell.bin`'s `no-store`
— **ruled (AD-6): kept, and said so in D-5** (F-54); the premises found true (F-55, `assembled.md` §6 — not to be
re-tried); the review's conduct (F-56, below).

**The owner's rulings at this review, taken (2026-10-11).** D-7 taken; D-8 done live with the owner, the block's
slot agreed at the window's start; D-2 and D-1 as proposed — transcribed in §"The owner's rulings for S2".

**Conduct.** Two reviewers broke the brief's rules and said so in their reports: the documents lens ran the secrets
guard in its export, which asked the owner's Cloudflare account for the Worker's secret names (read-only; two names,
no value) in a review the brief made offline, and one of its commands carried a stray `rm -rf /dev/null`, typed in
error (no effect); the client lens removed a scratch directory of its own with `rm -rf`. No repository file but the
reports was written and no live host was touched. Every dispatch from here names "no call to the Cloudflare account,
read-only included" beside the removal rule (§"Global Constraints").

### The "before" review (Task 9 Step 1), 2026-10-11 — what the plan review left, on an export of `1e9c092d`

**Method.** A `git archive` export of master `1e9c092d` in the session's scratchpad (`node_modules` linked, the export's
own `target/`); Tasks 1–4's blocks lifted from this plan by `blocks.py` and spliced by `apply_t1..4.py` (the material
under `docs/temp/security-review-2026-10-08/s2a-before/`, gitignored, with `logs/`); the one prose-only piece,
`start_refusal.rs`'s `get`, written as an implementer would (25 lines in `post`'s shape); one scratch-only addition, a
`CSP_LOG` console dump in the harness, for the enforce run's record. **The owner's ruling mid-run:** the mutant pass
stops — each task's reviewer runs its mutants on its export and the after review runs M1–M38 ("no need to redo the
same work that will be done here"); Task 2's seven had run and are recorded, Task 1's were not run.

**Readings.** (a) Task 1's cells **12/12** green on the first run (`headers.node.test.ts` 10; the real-build and
three-build stamp cells, 6.4 s); Task 2's pool describe **2/2** green on the first run; Task 2's scripts suite **543
passed, 1 skipped, 3 failed of 547** — the three `lane-variables` cells, red on any export because `git ls-files` finds
no `.git` (the real tree is green); the plan review's prediction was 547–553. Task 2's mutants M8–M13b: every one red
where named (M8 both pool cells; M12 fifty-one cells beyond the three; M13b the appends cell AND the two assembly list
cells — F-1, corrected at its sentence: the edge-rule drift cell reads the committed tree). (b) The DEBUG-client
`npm run e2e` under the policy ENFORCED (Task 4's override, Task 3's layer, Task 1's set): **81 passed, 0 failed,
9.2 min** — S1b's 79 plus Task 4's two framed cells, green on the plan's blocks — and the console carried **no CSP line
of any kind**: seven `Failed to load resource` lines, all for statuses cells stage (404, 503, 409, 403). Nothing the
covered flows need is blocked under enforcement today — the wrap's input (D-9). The Report-Only re-run was not taken: a
green enforced walk implies a green Report-Only walk under the same policy (no violation, no line) — the
orchestrator's ruling, in the ledger. (c) `cargo clippy -p yokohama-web --no-default-features --features server
--all-targets -- -D warnings` with the gated `get`: **clean**; the workspace lint clean. Beyond the trim, free once
the test binary existed: Task 3's three lib cells and its binary cell, **4/4** green.

**Corrections.** One, F-1 (Task 2 Step 5's M13b sentence). Two environment notes, no correction (E-1 the
`lane-variables` cells on an export; E-2 a splice gap of the orchestrator's own — the `it.each` dists — not the
plan's). Among the cells run, none failed to go red on its mutant; no name the tree does not spell was met; no
sentence of this plan was contradicted by the code. The findings file: `docs/temp/security-review-2026-10-08/s2a-before/findings.md`.

### The run (Tasks 1–8) — what each task's review found that this plan did not know

(to be written; the SDD ledger `docs/temp/security-review-2026-10-08/s2a-sdd/progress.md` carries every ruling)

### The after review (Task 9 Step 5)

(to be written)

### The window (Task 9 Step 9) — the thirty-fourth and thirty-fifth Worker deployments and the NAS's nineteenth

(to be written: the header reads per host and path; the Report-Only reading's console lines; the two-identity
block's two readings and the §8 rows; the live tiers' counts; the rollback points)
