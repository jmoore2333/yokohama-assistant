# Phase 3 execution kickoff prompt (2026-07-25)

Companion to the accepted Phase-3 map (`docs/plans/2026-07-25-phase3-map.md`). Paste everything
below the line into a fresh session to start Phase 3 execution. The map, not this prompt, is the
governing document — if they ever disagree, the map wins.

---

ultracode

We're starting Phase 3 of yokohama-assistant: the computer player. This is an execution
session, not a mapping session — the map exists and is accepted.

**Orient first, in this order:** `guidingpath.md` (authoritative status; its decision log
carries every rules ruling), then **`docs/plans/2026-07-25-phase3-map.md` — the governing
document for everything you do here.** The map's §1 settled inputs are not up for
re-litigation; its §4 execution model is your operating contract; its §5 gates are the
definition of done per sub-phase; its §2 verification base is measured fact you can build on.
CLAUDE.md's conventions (TDD, dated docs in `docs/plans/`, the `SEMANTICS_REV` bump contract,
the oracle independence rule, the two-tier gate policy) all bind.

**Your assignment:** run the ladder from 3.0 through 3.4 under the map's autonomy envelope,
one sub-phase at a time: brainstorm/spec → plan → TDD execution → PR with detailed
documentation → dynamic-workflow review → merge on proper review → next sub-phase. Use dynamic
workflows for the orchestration and the review passes. Do not begin 3.5 — it is
owner-in-the-loop; stop and hand back to me when 3.4's architecture decision is recorded.

**Model dispatch (my direction, also in map §4):** you (Fable) are the overall orchestrator
and run the plan-development agents. Opus 5 is sufficient for most development and review
agents — default to it. Sonnet 5 agents are fine for the PR-review fan-out provided an Opus 5
reviewer adjudicates their findings. Clean-eyes verification agents run at Opus 5 or above.
Escalate a tier only where capability changes outcomes.

**Review lifecycle per PR:** the pr-review-toolkit suite fanned out via workflow; findings AND
their resolutions posted as PR comments as they happen (the audit trail is the point — it is
what lets us go back in time and pick another path); Opus 5 adjudication; fixes applied under
the receiving-code-review discipline (verify, don't blindly apply); all seven release-tier
gates green before merge. Merged branches are preserved — branch deletion is a joint task with
me, never autonomous.

**Autonomy and stops:** within 3.0–3.4 you may move through rules rulings, `SEMANTICS_REV`
bumps (expected at least once — not blocking by itself), and `guidingpath.md` updates without
stopping, when highly confident, research-backed, and independently verified by a clean-eyes
subagent — record each as it lands. Hard stops that come back to me: radical direction
changes, genuinely table-dependent rules ambiguity, anything outside a sub-phase's mapped
purpose or gate (that is the re-map signal), and branch deletion.

**Standing rules from the map:** oracle co-evolution for any engine-touching change (the
independence rule strictly held — rules sources only, types-only dependency); every published
branching/perf figure states its measurement policy; the corpus is never precious (records pin
`semantics_rev`, regeneration is scripted); the compute ladder is M5 Air → 96 GB workstation →
cloud, and hardware never selects the architecture.

**The live gate may complete at any time.** When I report it played, the 3.5 dependency
dissolves; disposition table findings per the map's §7 exposure map (rules corrections →
`SEMANTICS_REV` + oracle co-evolution + corpus regeneration; observability facts → the 3.0
mask list).

**Truth-pass status:** map §10's three items are ALL APPLIED (2026-07-25) — guidingpath's
4.5× row is corrected in place and the pre-L2 branching row is flagged SUPERSEDED. Your only
remaining obligation there: 3.2 re-verifies the A/B at arena scale and records those numbers
beside the correction.

**First deliverable:** the 3.0 (observation & belief) spec, brainstormed against the map's 3.0
envelope, then its implementation plan, then TDD execution through to a reviewed, merged PR.
Then continue up the ladder.
