# Phase B analysis — close handover (the next session starts here)

**Written 2026-08-18 at the close of the Phase-B analysis session. Phase B (amendment §3) is
COMPLETE: the owner games were played and packaged 2026-08-17 (the previous handover), and this
session delivered the analysis half — the D7 triage with its separator and family-1 verdicts,
the driven-fixture harness (MERGED, PR #23 → master `f39e6c2`), the V2-play counters baseline,
the four-family probe readout, and the canon batch awaiting owner rulings. The next phase is
the amendment §3's Phase C: freeze the four driven families — and it STOPS at the owner-gated
points D7's routings now make concrete.**

## 0. Read order for a fresh session

`CLAUDE.md` (updated at the merge: the harness architecture bullet, the runner command, core
counts 1090+29+1) → guidingpath's 2026-08-18 decision row → THIS document → the D7 triage
(`docs/plans/2026-08-17-phase-b-d7-triage.md`, §3's readings are the session's measurement
result) → the counters/probes report (`...-counters-probes-report.md`) → the canon batch
(`...-canon-batch.md`) → the ledger's PHASE B ANALYSIS + HARNESS EXECUTION rows (the full
execution trail). Then **verify, never trust**: `git status` clean, master at or past
`f39e6c2`, and the per-edit suites — core **1090 + 29 ignored + 1 doc-test** (aggregate 1091)
· cli **771 + 2** · oracle **191** · difftest **52 + 5** · eval_probe **36** · bridge **81** ·
node 4/4 · clippy clean (own rc). Corpus caveats unchanged (FOURTEEN bga pins over two
captures; the six harness registry roots live in gitignored `docs/temp/owner-games/` saves —
absent files are typed misses, never test failures).

## 1. WHAT I NEED FROM YOU (the owner) — each ask, and exactly where to put the answer

**ASK 1 — the canon batch: four inline rulings.**
*Where:* `docs/plans/2026-08-17-phase-b-canon-batch.md` — each of the four items (C-A majority
participation · C-B recoverable-flip pricing · C-C tech-ability value vs majority chips · C-D
the 100% lab-rush opening) ends with an **`Owner ruling: ____`** line. Rule INLINE at that
line (a word is enough — "yes", "yes but…", "no, instead…"), amend anything in place, and
commit or hand the file back. The amendment's markup pattern, same as the ratified amendment.
*Why it gates:* amendment D2's preference-vs-rules split — a driven family whose pass
criterion is a preference may support a GO or pivot ONLY through a canon ruling. C-A/C-B feed
family 3's controls, C-C gates the largest representation lane, C-D decides whether opening
diversity becomes a probe target. Phase C's family freeze wants these before the
preference-shaped families freeze; the rules-derivable family (1) proceeds regardless.

**ASK 2 — confirm (or override) the not-a-major-merge call.**
*Where:* a one-line reply in the next session (or a note anywhere in this file's margin).
*The call as made:* the harness merge moves NO play behavior (instrument only; the arena split
is byte-verified; `SEMANTICS_REV` 7 untouched), so by your own D2 criterion no owner
diagnostic game rides it. The next game-triggering merge is the first behavior-moving one —
expected out of Phase C (a separation fix, a V3 admission, or W6 knob work).

**ASK 3 — nothing else is owner-gated right now.** The next hard gates per the amendment are
Phase C's STOP conditions (any V3 mini-cycle or the D3 capability path — both routings are now
concrete, see §3) and, later, Phase E's pre-registration signature. D9/OD-8 (the acceptance
format) remains open at your leisure, as ruled.

## 2. What this session landed (all on master)

1. **The D7 triage** (`2026-08-17-phase-b-d7-triage.md`) — every owner-games finding
   re-derived from the saves by a six-agent evidence workflow before routing; an 11-item
   corrections ledger (headliners: "+336" was 351; the −983 table is game 1's; e228 was not
   the final move); the branch-0 filing with full value-level distributions; **§3's measured
   verdicts (see §3 below)**.
2. **The driven-fixture harness, MERGED** — `game/driven.rs` + `game/driven_runner.rs` +
   `examples/driven.rs`: rooted runs over the honest chain (the arena loop split
   behavior-preservingly, record-free play half), `root_table` (the separator instrument, full
   15-counter surface, sampled-world identity recorded), `adjudicate_root` (forced-candidate
   playouts, whole-table refusal on any playout failure), `RootSource`/provenance/registry
   (six session roots, verified seeds + recorded moves, typed `Stale`), the runner
   (reproduce-by-default; the robustness label pinned to its output). T1–T5 under the standing
   cadence + five adjudicated fix waves; roster 15/15 at 487.8s (WSL); Fable whole-branch
   verdict MERGE-WITH-NOTES, all notes folded. PR #23's body is the merge's own record
   (including the named history wart).
3. **The V2-play counters baseline + probe readout** (`...-counters-probes-report.md`) — 400
   games at recorded master `0xE7A1202608170001`: R0's early-vector findings resolved
   in-ecology (EVAL TOTAL EARLY 0.5915 excludes chance); game length UNCHANGED vs V1-play
   while stage-proxy accuracy moved 0.5090 → 0.6660 (clock-fill shape, not length); church
   endings 98.25%; `customs_majority_margin` newly separates from its shipped mirror late;
   `achievement_race_proximity`'s edge moved MID/LATE. Tie-share stays an instrument gap
   (recorded, not patched).
4. **The canon batch** — §1's ASK 1.
5. **Process:** the opus/fable cross-model cadence (owner-directed 2026-08-17) ran the whole
   harness build — the Fable design review caught two MUSTs before implementation
   (`root_table`'s unimplementable construction path; the record seam), the T1 key-component
   pass and the whole-branch pass each earned their keep. Recorded in the dispatch-policy
   memory and the ledger; the Opus-max intensity was phase-scoped by the owner, not standing.

## 3. The measured verdicts Phase C builds on (the triage §3 in one paragraph)

**The tie-mass defect class has TWO mechanisms, each with a frozen exemplar root.** g1:113
PERSISTS at every beam (36/25/56-way ties, band never below 41 of 139) ⇒ REPRESENTATION — the
fix path is the D11 debt-list admission target ("Placement-root value separation":
assistant-LOCATION value the hand-off vector cannot see), branch 0's fix-as-a-bug with its
regression pin, evidence ladder first. g3:63 DISAPPEARS at b=8 into a unique +160 church-line
argmax ⇒ HORIZON — exemplar for the D3/depth lane (already ruled in-scope; Phase-C STOP gates
any build). The e93 control held at every beam, and every b=4 run REPRODUCED the recorded
move. **Family 1's dead-position question is ANSWERED for the session roots:** all three
(g1:228, g3:178, g1:239) are genuinely dead — 0 wins in any of 8 replications for ANY of
148/139/55 candidates, field spread ≤4 VP — so F5 resolves as honest indifference, and the
final-round family's ground-truth instrument is proven on real material. F2 (the one-sided
horizon) stays branch 3 with the verified 351→28/36→194→−9 series; F3/F4 stay canon-gated
representation asks; F8's family-4 green + T3 first contact stand as positive evidence.

## 4. Phase C — what the next session opens (amendment §3, now concrete)

Freeze the four driven families: per-family root GENERATORS (predicate + master + ordinal +
generating ecology — the harness's `Generated` arm is the type; family 1's
`FinalRoundAlreadyTriggered` predicate ships), declared reply policies (the adversarial
fourth menu slot gets DECLARED at freeze), escrowed disjoint blocks for post-fix re-reads,
CONFIRMATORY vs EXISTENCE-DEMONSTRATION labels, pre-declared R/failure counts — then run and
route through D7. **STOP at any V3 mini-cycle or D3-capability routing — both are now live
possibilities on the §3 verdicts (the e113 admission ladder and the e63 depth exemplar).**
Inputs ready: the canon rulings (ASK 1), the six registry roots + the separator readings, the
V2-play baseline, the harness end to end. Also carried: the branch-4 CLI list (handover §5's
five + the why-counters candidate), the tie-share instrument gap, T4-review NOTE-4 (no 3–4P
adjudication has ever run), and the harness follow-ups recorded in the ledger (none blocking).

## 5. Housekeeping state

Branch `phase3.5c-harness` pushed and preserved; the local worktree `../yokohama-harness-wt`
REMOVED at close (recreate with `git worktree add` if needed — the two provisioning symlinks
were its only extras). The A3E1 0818 serials 1–3 are consumed (the triage §3's adjudication
masters); E7A1 0817 0001 is consumed (the counters corpus). Wall figures this session are all
WSL-tagged. `docs/temp/owner-games/` is unchanged and NAS-synced per D10.
