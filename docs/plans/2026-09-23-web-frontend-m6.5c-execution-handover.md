# M6.5c "the objects" — execution handover

**Written 2026-09-23 at the close of the brainstorm, the spec and the plan, for the FRESH SESSION —
a different orchestrator, in a clean context — that EXECUTES M6.5c.**
- Branch `web-room-m6.5c` carries only documentation and one `.gitignore` line; no source has moved.
- The spec went through three reviews (an adversarial read, a scoped re-read, and the plan's review) plus the owner's read.
- The plan went through one adversarial review and one fix wave.
- Everything below is a pointer or a measurement. **No decision here is open unless §2 lists it**, and where this document and the spec disagree, **the spec wins**.

---

## 0. Read order (the session's first half hour)

1. **The spec**, `docs/plans/2026-09-22-web-frontend-m6.5c-objects-spec.md` at `92592f2` — the authority. Read these parts:
   - §1, the sixteen rulings R-6.5c.1–16 with the owner's own words;
   - §2, the mechanism;
   - §5, the tests, and §5.7, the 39-row mutation battery;
   - §6, what is not claimed;
   - **§9.1–§9.4**, what each review changed, so every sentence can be traced to its reason.
   §0.3–§0.4 are what was MEASURED; do not re-derive any of it.
2. **The plan**, `docs/plans/2026-09-23-web-frontend-m6.5c-plan.md` at `ec7f268` — sixteen tasks. Read these sections WHOLE before dispatching anything:
   - Global Constraints;
   - Rulings P-1–P-3;
   - **Pool facts**;
   - Execution shape;
   - **Names shared across tasks**;
   - Revision.
   They are the cross-task contracts; an implementer sees only its own task.
3. **The reviews**, gitignored, read for their METHOD — every finding is already folded in:
   - `.superpowers/sdd/2026-09-22-web-frontend-m6.5c-spec/review-spec.md` and `rereview-spec.md`;
   - `.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/review-plan.md`.
   The plan review's scratch project lived in the planning session's scratchpad and may be gone. Its measurements are in the report and in the plan's Pool facts.
4. **`docs/temp/m6.5c-spike-readings-2026-09-22.md`** (gitignored) — the platform spike's readings, the source of spec §0.3.
5. **The entry handover**, `docs/plans/2026-09-22-web-frontend-m6.5c-handover.md` — what M6.5c inherited. Its §4 items are each placed in spec §7.
6. **`CLAUDE.md`** — the operating facts and the gate commands. **Re-count every suite at your own HEAD.**

---

## 1. Where things stand

- **Branch `web-room-m6.5c`**, stacked on `web-room-m6.5b2`. **The base for every diff is `2621fbe`.**
  - Its commits: `952a68c` (ignore `.dev.vars`); the spec's `f80e2f7`, `8dc9096`, `cf86c0e`, `1968104`, `cca0aed` and `92592f2`; the plan's `bcc68a2`, `6b26c87` and `ec7f268`; and this handover's commit.
  - Nothing is pushed. `git status` is clean.
- **LIVE on the NAS: `yokohama-web:0.5.0-m6.5b2.1`** at `yokohama-nas.moore.nyc` (RUNBOOK §3.16), on `claimed`. It is untouched by this cycle.
- **`yokohama.moore.nyc` has NO DNS record.** It is held for the Worker, and Task 11's custom domain creates it. The dead tunnel CNAME that was removed for the spike is recorded in the readings file; do not restore it.
- **No Worker is deployed.** The spike's throwaway Worker, its namespace and its domain were deleted.
- **Seven PRs are open and stacked, nothing is merged**, and the merge order is the owner's. This branch's PR opens at Task 16, against `web-room-m6.5b2`.
- **The account.** Workers FREE (R-6.5b.16), account BDVC.
  - **The Access application `yokohama`** (`11cbcf68…`, audience `5fa7a989…`) already names `yokohama.moore.nyc` as a destination.
  - **The Service Auth credential `yokohama-worker`** passes Access under the policy "yokohama ST bypass" (`non_identity`, precedence 3). Its client id and secret are in the repository root's **`.dev.vars`**, gitignored since `952a68c`, under the keys **`CF-Access-Client-Id`** and **`CF-Access-Client-Secret`**. **Never print it, never commit it, never paste it into a report** — the plan's Global Constraints give the header-file recipe.
- **wrangler 4.135** is logged in on this workstation with Workers scopes and **no R2 scope**. The bucket `yokohama-archive` was therefore created through the account API (the Cloudflare MCP's `execute`) on 2026-09-23; it is private and not yet bound.

---

## 2. What the owner ruled, and what is still THEIRS to answer

The sixteen rulings are in spec §1. The ones that most shape execution:
- **Private at the close; built for the public door.** It is deployed behind the hostname-wide Access application, and nothing is published.
- **A TypeScript shell over a per-object wasm-bindgen module**, not workers-rs (R-6.5c.2, chosen on evidence).
- **Identity, in order:** a validated Access assertion naming a PERSON; else the `yokohama-worker` credential with the dev door on, which honours `X-Yokohama-Claimed-User`; else none. **An operator must be a person**; the credential never is.
- **Host-local changes.** The layer changes by EXACTLY TWO items (R-6.5c.4). A third is a STOP.
- **Workers Free, and 30 s of CPU per object invocation**: the owner's assumption, backed by the spike's 376 ms `ok`.
- **The archive has a clean off switch** (`YOKOHAMA_WEB_ARCHIVE=r2|off`).
- **Tables expire**, as the creator's own Abandon, behind a 90-day marker.
- **A minimal operator surface**, and `?force` also makes the live replay repeatable.
- **Execution (2026-09-23):** SUBAGENT-DRIVEN for every task, with Opus as the floor for every implementer and reviewer, Fable for Task 16's whole-branch review, and the ORCHESTRATOR running every live step. The commit shape is **ONE GREEN COMMIT PER TASK**, with the red run quoted in the commit body and the ledger.

**The owner's answers (2026-09-23, after the plan review), and what is still open:**

| item | answer / status | where |
|---|---|---|
| The zone's **Web Analytics** and **Bot Fight Mode** (with its JavaScript detections) | **Both stay ON.** Zero console errors is not the goal: the beacon's CORS error is DIAGNOSED at the first deploy, FIXED by configuration if it can be (with the owner's approval), else accounted for EXACTLY in the live tests | Task 11 reading 6; Task 15 Step 5 |
| The **TTL** | **90 days** | Task 14 |
| The **operator list** as a Worker secret (P-1) | **Approved**; the owner's address | Task 14 |
| The **bucket** `yokohama-archive` | **CREATED 2026-09-23** by the planning session: ENAM, Standard, private | Task 13 binds it |
| A read-only **R2 key for `rclone`** | **OPEN — the owner creates it** in the dashboard (Object Read, this bucket only), into `.dev.vars` as `R2_ACCESS_KEY_ID` / `R2_SECRET_ACCESS_KEY`. It is not minted through the API, whose reply would land the secret in a transcript | Task 16's exit |
| A lifecycle rule on the bucket's `test/` prefix | OPEN, optional — ask | Task 13 |
| **The phones** | recorded, not gated | Task 16 |

---

## 3. How to execute

- **One task at a time, 1 → 16.** Task 8 may run beside the Rust tasks ONLY in a separate git worktree. The standing rule is that nothing runs in a tree while an implementer works there.
- **Each dispatch** gives the implementer its task's text, the spec's path, and the plan's four cross-task sections: Global Constraints, Pool facts, Names shared across tasks, Execution shape. A fresh Opus reviewer then checks the task, and the red run must be quoted in the commit.
- **The ORCHESTRATOR runs every step marked ORCHESTRATOR:**
  - the deploys, at Tasks 11, 12, 13, 14 and 16;
  - the bucket, the secrets and `wrangler tail`;
  - the live Playwright and replay runs at Task 15;
  - the exit.
  A subagent never touches the account or the credential.
- **The ledger is `.superpowers/sdd/2026-09-23-web-frontend-m6.5c-plan/`.** It already exists and holds `review-plan.md`. `progress.md` holds one line per task, one `Ruling:` line per deviation from the plan, and every deploy's readings. Every agent writes its full report to a file there and replies with the path and three lines; the notification channel truncates at about 4k.
- **Task 11's first deploy re-reads the spike with the real code.** A reading that differs from spec §0.3 is a FINDING: stop, and bring it to the owner before Task 12.
- **Every deploy is recorded in RUNBOOK §6**, continuing the container era's numbering. Task 11's is the fourteenth deployment.
- **The standing process rules** (the lane's memory):
  - read HEAD before following up an implementer's DONE;
  - never amend a reviewed SHA;
  - while an implementer works, arm no timers and run nothing in its tree;
  - wake a stalled agent by message rather than spawning another;
  - a subagent that backgrounds a long run and "waits for events" idles for ever, so every dispatch says to run in the FOREGROUND.

---

## 4. The traps this session already paid for

Each was found by measurement, is written into the spec or the plan, and must not be re-derived.

1. **The stock wasm-bindgen glue gives an isolate ONE instance.** `initSync` returns early once `wasm` is set, so a trapped instance could never be replaced and every object would share one `GAMES`. That is the factory transform's reason (spike O-1).
2. **The asset layer's defaults break the service worker.**
   - The default `html_handling` answers `/index.html` with a 307, and the worker installs with `redirect: "error"`.
   - The SPA fallback answers a missing script with HTML.
   - The config is `none` and `none`, plus `_redirects` and `_headers`. A missing file then invokes the Worker, which answers an empty 404.
3. **The test pool** (plan, Pool facts):
   - a method patched on a live instance makes the RPC THROW. Battery 25's catcher was vacuous TWICE: first under `evictDurableObject`, which drains, then under an instance patch;
   - patch prototypes, restore them before the alarm, and take fresh stubs after an abort;
   - storage is per FILE;
   - an eviction drops instance state;
   - close reasons are at most 123 bytes.
4. **The events `wait` cap is axum's HANDLER's** (`lib.rs:586-590`), not the layer's, so the module restates it, and its order.
5. **The finalizing keep emits TWO bumps** (`bump_table`, then `bump`). Frames are one per socket per reply.
6. **`play_to_the_end` drives a second `Repl`**, so the layer's new cells evict the resident handle first. Never inside `archive_over_retry`.
7. **The PlaceA path is `1`, `1`, `A09`, `confirm`.** The walk's `2` opens PlaceB.
8. **`"id": null` answered 201 for a table no object held**, and the layer accepts a positional ARRAY body. Both are refused on this host.
9. **A GET can write**: a cold handle pumps an owed bot move and persists it. The shell compares documents after EVERY module call.
10. **Seeds above 2^53.** The shell never `JSON.parse`s a record, and the Rust-made fixtures reach the pool as TEXT.
11. **The live replay's repeatability**:
    - `?force` of its three fixed ids, before and after, from the owner's WARP shell (the credential is never an operator);
    - both archive first sends accept the once-rule's 200;
    - the house chat is offset by its `last`;
    - the lobby is filtered.
12. **The zone's analytics beacon logs a console error on every page load**, and the owner keeps it ON. Task 11 diagnoses it; Task 15 runs clean after a configuration fix, or allow-lists exactly that error.

---

## 5. Facts not to re-derive (the spike of 2026-09-22 and the reviews)

| | |
|---|---|
| Protection, unenrolled | every path 302 to the Access login; a WebSocket upgrade 403; `workers.dev` 404 |
| Identity on a WebSocket | a person's email reaches the 101 over WARP. The credential's cookie alone carries a page-opened upgrade. Service Auth outranks WARP. Playwright's context headers reach a page-opened upgrade |
| The module | 1,172.84 KiB upload / 453.90 KiB gzip; startup 1–3 ms |
| CPU (Workers Free) | 376 ms `ok` for one heavy invocation; the cold path for a 595-event game 19–28 ms; `why` ≈ 50 ms per bot move; the routing Worker 0–1 ms, JWT only decoded — VERIFICATION is unmeasured until Task 11 |
| Hibernation | the wake count went 1 → 2 after 35 s idle; the socket stayed open and its attachment intact; ping → pong answered at the edge |
| Static assets | 152 of 153 byte-exact under the defaults, and all 154 with the config; the service worker installed completely from the Worker host; ZERO invocations for real files |
| The account (7 days) | Worker requests 17k–33k/day and object requests 300–1,800/day, both against 100k and shared by about 97 Workers; rows written not read |
| `getrandom` in an object | works through `crypto.getRandomValues`; `nodejs_compat` is no obstacle |
| R2 | `onlyIf: new Headers({"If-None-Match":"*"})` returns `null` on a present key (pool); the live form is read at Task 13 |
| `deleteAll()` | deletes the alarm at compatibility ≥ 2026-02-24 |

---

## 6. Definition of done

**Task 16's close checklist, whole:**
1. The Fable whole-branch review, ONE fix wave, and a scoped re-review.
2. The 39-row battery at that head, every mutation mapped to a catcher. Row 4 is recorded as uncatchable.
3. **The exit, in a window with no deploy:**
   - a HEADED two-person game with a watcher;
   - hibernation OBSERVED, with `wakes` rising;
   - the move-to-other-screen wall, with its CPU off the tail;
   - a finished and an abandoned game in the bucket, `rclone`-synced, loaded in the CLI, with `why` reconstructing a bot move;
   - the whole corpus byte-identical live, the replay run twice;
   - requests AND rows written per game, off the analytics.
4. The phones, recorded.
5. The counts, in one sequence.
6. The fences: the fenced crates, the CLI and `yokohama-wasm` byte-identical to `2621fbe`, the layer changed by exactly two items, the corpus unchanged, `SEMANTICS_REV` 7.
7. The close, on the M6.5b2 close's shape, whose §7 starts from spec §6.
8. `CLAUDE.md` and `docs/handover.md` updated.
9. The PR opened against `web-room-m6.5b2`, the merge order the owner's.

**After M6.5c** (the owner's direction, 2026-09-22): ONE cleanup cycle reconciles the NAS/Docker host with the Worker. That cycle's list already holds:
- the layer's positional-array quirk;
- a logging seam for the layer's mute `eprintln!`;
- a TTL for the NAS.
Then `CLAUDE.md` and `guidingpath.md` are refreshed, and the project refocuses on the computer player.
