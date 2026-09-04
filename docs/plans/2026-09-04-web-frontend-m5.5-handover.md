# Web frontend — M5.5 "the table reads": handover into execution

**Written 2026-09-04 on branch `web-frontend` at `3460cab` (the plan's commit), for the session
that RUNS the plan.** Nothing has been executed. The spec and the plan carry everything the
run needs; this file says where things stand, what to read, and what the planning session
knew that is not written into either.

## 0. Where things stand

- **The spec** `docs/plans/2026-09-04-web-frontend-m5.5-spec.md` (`652badc`) is ratified by the
  owner (2026-09-04, "no comments on substance") with two plan additions: the Portainer deploy
  to the NAS, and the final review done against the LIVE site by a clean subagent.
- **The plan** `docs/plans/2026-09-04-web-frontend-m5.5-plan.md` (`3460cab`): Task 0 (M5's
  owed close) then parts A → C → B (Tasks 1–12; there is no Task 10), the deploy (13), the
  live review (14), the fix wave and redeploy (15), the close (16). Every task carries its
  code and its cells. Ten planning deviations are listed at its end, for the close.
- **M5** is live as `yokohama-web:0.5.0-m5.1` at `https://yokohama.moore.nyc`; its close is
  Task 0 with the owner's play as the verdict (spec R9). Its SDD workspace
  `.superpowers/sdd/2026-09-01-web-frontend-m5-plan/` still exists (gitignored) and holds the
  task-16 brief Task 0 executes; Task 0 deletes it at its end.
- **The tree is clean** at `3460cab`; `crates/yokohama-core` is byte-identical to master;
  `SEMANTICS_REV` is 7. Master is an ancestor of the branch (96 commits behind it, 0 ahead).

## 1. How to pick this up

1. Read `CLAUDE.md`, then the spec, then the plan's header, Global Constraints, Execution
   model and Pre-flight (the plan's first 45 lines), then this file.
2. Run the plan's Pre-flight. Record the executing session's `Claude-Session` URL: it is the
   trailer on every commit (plan constraint 1; M5 ruling R-P3).
3. Use `superpowers:subagent-driven-development`: create
   `.superpowers/sdd/2026-09-04-web-frontend-m5.5-plan/progress.md` in the shape of M5's
   ledger (pre-flight scan of task pairs, then one ruling row per decision, R-P… before Task 0,
   R-T<n>… per task). Opus implementers and Opus per-task reviewers; Fable orchestrates and
   dispatches the Task 14 review. No Sonnet or Haiku.
4. Gate pictures (Tasks 4, 9, 12) go to the owner with `SendUserFile` and do not stop the run.
5. Task 13 is run by the orchestrator through the Portainer MCP (environment id 2) — the
   owner authorized it in the plan's ratification for a dev window; confirm the window in
   the message that precedes it. Task 14's brief is verbatim in the plan.

## 2. What the planning session knew that the documents do not say

- **The cube mock is gone with its session** (a scratchpad page, throwaway by design). Its
  readings are in the spec's §0.2 table. The one prior for the band's left edge is the
  mock's reading that Church's printed icons end near x 0.33 in the band's rows; Task 3
  measures it properly and the eye checks the candidate lines.
- **The repo's `grep` is a gitignore-aware ugrep function**: `/usr/bin/grep` for `-E`
  alternations and `--include` globs, or the pattern errors out.
- **The corpus is bot-free and deterministic**; the only fixture that will carry a bot seat
  is `preview_bot.json` (Task 11), which is safe because a preview pumps nothing.
- **The `at_k.json` anti-vacuity cell (Task 5)** may need the capture's `k` moved off
  `total / 2` if no port card left the port by the midpoint; the plan says how.
- **`Repl::new(session)` after `set_run_seats`** is expected to mark the bot seat in
  `seat_facts()`; Task 11's preview cell asserts it, and if it does not hold, mirror what
  `Games::create` does to the `Repl` short of pumping.
- **The e2e harness's `readScreen` skips a running walk from Task 9 on**; until then the
  pile cell (Task 4) runs against a client with no walk and needs no skip.
- **This Mac sleeps after one idle minute** — `caffeinate -i` in front of any long run
  (the e2e suite is ~60–90 s; a musl cross-compile from cold is minutes).
- **The owner's standing direction for pictures**: mock before asking, send screenshots,
  decide from the picture. The one taste question left in the plan is the band's left
  edge (Task 3), answered by measurement with the sheet as the check.

## 3. What the close owes, in one line each

The M5.5 close record (Task 16) is M6's entry artifact: it carries the walk as the spectator
primitive, M4's hosting list by reference, M6's first three questions (transport: SSE;
identity: verify the Access assertion at the origin or keep the Worker the only door; the
store: Neon only if the Worker takes a share of the state), M5's LEAVE items M6 inherits,
the two V5 notes for the AI lane (a sighted Keep; per-good requirement weights), and the
draw-order note as a guidingpath row. The memory file `yokohama-web-lane-status.md` is
updated once, at that close.
