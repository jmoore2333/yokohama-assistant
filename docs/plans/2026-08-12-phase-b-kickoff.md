# Phase B kickoff — the handover into the strength checkpoint's exploratory layer

**Written 2026-08-12 at the close of the Phase-A session.** The next agent starts here after the
standard read order: `CLAUDE.md` → guidingpath's decision-log tail → the ratified amendment
(`docs/plans/2026-08-11-phase3.5c-scoping-amendment.md`, the forward authority) → the 3.5c
execution ledger's PHASE-A / W5 / W2 rows (`docs/plans/2026-08-09-phase3.5c-execution-ledger.md`).
Then **verify, never trust**: `git status` clean, HEAD at or past this document's commit, and the
per-edit suites at the current counts — core **1012 + 29 ignored + 1 doc-test** (aggregate 1013) ·
cli **771 + 2** (743 lib · 9 `bga_import` · 18 `guide_sync` · 1 dogfood; the cli lib wall is ~23s
now — the `why` every-move pin, expected) · oracle **191** · difftest **52 + 5** · bridge **81** ·
eval_probe **36** · node 4/4 · clippy clean (check clippy's OWN exit code, never a pipeline's).

## Where the lane stands

**Phase A is COMPLETE and merged** (amendment §3): W5 = PR #21 (`Contestant` bot seats, the
provisional `lookahead-v1` default via `bot::PROVISIONAL_DEFAULT_BOT`, `why`/`why <n>`
reconstruct-by-replay with the refuse-on-mismatch gate, `record::state_at` +
`Contestant::from_parts` + the public echo in core); W2 = PR #22 (the price-keyed `"trams"`
translator arm, the rev-7 comparison corpus — two records regenerated, `895724349` self-resolved
its achievement hypothesis, `896157897` used the owner answer `B=Orders5` — and N3's two
tech-ledger rows, now 14 shipped / 16 unobserved). Ten implement+review cycles, two MUSTs total,
both fixed and mutation-verified; both rosters 15/15. The README was refreshed the same day
(status through Phase A, machine-tagged roster baselines, V2 gate figures, new screenshots from
the W5 build — `docs/images/cli-frame-w5.png`, `cli-why-w5.png`).

**The owner has the diagnostic-games brief**: `docs/temp/2026-08-12-owner-diagnostic-games-walkthrough.html`
(gitignored — a briefing, not a maintained doc; regenerate rather than edit), delivered 2026-08-12.
It walks the two layer-1 games, the four families to watch, the flagging protocol (event index +
`why n` + one line), and the exact feedback package. **The games are the owner's physical action
and the session hard stop we ended at.**

## Phase B's four work items (amendment §3, D2 layer 1)

1. **The driven-fixture harness — its own named task, the standing implement+review cadence.**
   Sited in `yokohama-core` beside `game/eval_screen.rs`: in-crate cells over the public agent
   surface, called from tests and round scripts, never from argv/CLI/gates; an `examples/` runner
   on the `eval_probe`/`eval_screen_round` precedent; **OFF the fifteen-gate roster** (a
   graduating fixture joins `scenario_suite`'s never-delete registry instead). What it ADDS is
   small and named in the amendment D2: a play-from-supplied-`GameState`-to-terminal driver (the
   `drive_greedy_2p` pattern generalized — every current whole-game driver starts at `new_game`),
   the fixture/generator registry, named reply policies, rooted-run record provenance. What it
   REUSES, not rebuilds: `search_world`, `record::replay_from` (and W5's `state_at`),
   `eval_probe::snapshot_game`'s walk, `observation_leakage::decide_in_world`.
2. **Exploratory probes on the four families** (already-triggered final round · end-trigger
   timing under the inverted brake · exclusive-race awareness · achievement proximity) — unfrozen,
   cheap, never GO evidence; they inform layer 2's generators and controls.
3. **The two recorded owner games** — pending the owner; when the feedback package arrives,
   triage per D7 (branch 0 defect classes FIRST: tie-mass/value-separation degeneracy,
   candidate-generation gaps, cap/starvation truncation, sampler artifacts — the counter check is
   step one of every report), assemble any canon-question batch for the owner (preference
   findings enter layer 2 ONLY through canon rulings), and answer-collect the open D2 ask
   (how much owner play time the checkpoint may budget before tuning).
4. **The V2-play counters baseline slice** — cheap, orchestrator-runnable, and D8 wants it BEFORE
   W6 touches anything: 2P counters-only run at the shipped configuration on a fresh recorded
   diagnostic serial (end-trigger mix, median/quartile game length, stage-proxy distribution,
   nonzero-recovery share, tie share, cap/starvation counters, explanation identity, per-decision
   wall recorded-not-triggered). The R0 ecology is V1-play; this is the V2-play regime fact
   sheet W6's regime check will be judged against.

Items 1, 2 and 4 are dispatchable immediately (no owner gate inside Phase B); item 3 waits on the
owner. Suggested first move: dispatch the harness task, run the counters slice while it reviews.

## Standing constraints the next session must hold (hard-won this session)

- Dispatch: Opus implementers/reviewers at xhigh (max when warranted), Fable adjudicates;
  fresh implementer per task; the orchestrator NEVER touches the tree while a workflow runs.
- **Commit a fix wave BEFORE running revert-style mutation probes** (`git checkout --` wiped
  uncommitted fixes once this session); **read clippy's own rc, never a pipeline's** (a piped
  `tail` masked a red clippy once this session).
- `rg`, never bare recursive `grep` (gitignore-aware shell function skips `bridge-captures/`);
  and `rg -r` is REPLACE, not recursive.
- Wall figures are per-machine, always tagged; the ≤420s roster target is an M5 reading; this
  session's WSL roster runs were 498.5s (W5) and 511.7s (W2), beside the 508.9s baseline.
- Corpus pins are **FOURTEEN over TWO required captures** (`895724349` + `896199544`); the
  emitted rev-7 records live beside them (gitignored; the ledger's W2 row is their provenance;
  D10's NAS sidecar sync carries them).
- **Known open finding, deliberately not fixed:** `896199544` clears m128 but STOPS at the
  resolve stage (`no engine building site carries distinct spaces for observed build rewards
  [movement:2, vp:4] — FishingGround`, resolve.rs). Its own future small self-contained PR per
  the D5 process; until it lands, the only close human game emits no record and the tram
  matcher has no end-to-end corpus coverage (the seam pins guard it meanwhile).
- Owner-gated points ahead: nothing inside Phase B except the games; the next hard gates are
  Phase C's STOP conditions (a V3 mini-cycle or the D3 cross-handoff path) and Phase E's
  pre-registration signature.
