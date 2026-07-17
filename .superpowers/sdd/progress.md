# SDD Progress Ledger — engine-completion plan

Plan: docs/plans/2026-07-16-engine-completion-plan.md (READ ITS "Amendments (2026-07-17)" SECTION — it overrides task text).
Branch: engine-completion (from master @ 60c79ab). Suite baseline before 11b: **183 passing (177 lib + 6 integration), zero warnings**.
Execution model: orchestrator (Fable) dispatches one fresh subagent per task (Agent tool; sonnet for implementation, haiku for scripted tasks 8/10-style, fable only for vision), reviews every diff + reruns the suite between tasks, resolves rules questions itself against KB+rulebook (rulings → guidingpath.md decision log). Task briefs via the superpowers SDD skill's task-brief script; do NOT paste plan text into dispatches.
Untracked files .codex/, .vscode/, AGENTS.md are Jeremy's — never add/commit them.

## Complete (all orchestrator-reviewed, suite green after each)

- Task 1 toolchain: 11bd460 (real fix: copied Cellar libexec rustup binary — Homebrew bin/rustup is an argv0-breaking bash wrapper; plain `cargo test -p yokohama-core` works)
- Task 2 doc banners: 4166593 · Task 3 CI workflow: 8bce140 (not pushed)
- Task 4 MoveError: 5808dad · Task 5 Rules context: 2b88fe5
- Task 6 card verification: 914fa4d (gate passed; YAML corrected: order ids = printed corner index, techs copies:=30, sites re-modeled; manifest all VALIDATED; Jeremy's physical box-check OPEN — box-audit.html delivered, exceptions-only feedback)
- Task 7 gen-data + Rules::standard(): d4185f5 (30 tech ids; SpaceReward movement/warehouse_tokens per icon ruling; regen deterministic)
- Task 7b reward application (inserted): ed85133 (warehouse auto-assistant, any-good auto-copper shared helpers, movement no-op)
- Task 8 display: d0c7184 + efd930d (review fix: test-only import; agent's "zero warnings" claim was false)
- Task 9 decks+new_game: 01197e0 (setup constants 16 & 3/4 landed; NOTE: its 22/22 deck tests are being superseded by 11b → 16/18)
- Task 10 reveal helpers: 9fa69a7 (dual-pile extract per hidden-order fiction)
- Task 11 wire Port: fadc2d7 + 5a9a266 (review loop: total compaction; KB rightmost-refill ruling — plan's test expectation was wrong)
- Docs commits: 3e3e0db, d1bbfb9, abb5615, 02f1724, a58f2d3 (+ this handoff's docs commit)

## In flight

- **Task 11b six-slot re-model (inserted, sonnet, BASE = 5a9a266+docs)**: PortBoard → [Option<OrderCardId>;6] zones LOW,LOW,MID,MID,HIGH,HIGH; AreaAction::Port cards_taken: Vec<OrderCardId> (Jeremy chose card ids); LabBoard → 6 slots, surcharge [0,0,1,1,2,2]; setup fills 6+6 → 4P deck tests become 16 orders/18 techs; end_game port check over 6; KB port/lab lines corrected. REVIEW CHECKLIST when it reports: zone accessibility (2→Low,3→+Mid,4/5→all), compaction+rightmost refill exact-layout tests, Task-11 test intents preserved, no Task-12 scope creep (lab wiring stays stubbed), suite green + zero warnings, rerun suite yourself.

## Next (in order)

- **Task 12 wire Lab** (sonnet; brief at .superpowers/sdd/task-12-brief.md). Dispatch amendments (prepared): Move carries card_id → resolve slot by searching THE ACTING AREA'S board only (not both — cross-board buying is illegal; mercenary area resolution like Port's); validate_lab_action currently hard-rejects Lab B / only reads lab_a — rework for the resolved board (now 6 slots); drop tech_deck param (state.tech_deck); acquisition bonuses AFTER success: Patent +2 VP and Letterpress +1 Yen per owned OTHER copy (stack per copy, exclude the just-acquired card), Exposition self-trigger +10 VP (plan gap, rulebook-explicit); Station placement on acquisition = v1 limitation (station_location untouched, comment); check_mercenary_eligibility recheck with card country (make pub(crate) if needed); fifth-tech penalty machinery exists.
- **Task 13 Church** (sonnet): amendments in plan §Amendments item 6 (2P dummy blocks faith-7; riders no-op; avoid power-2 fixtures; v1 auto-place highest eligible ≤ faith; use calculate_church_faith/faith_to_index/next_arrival_order; Winery +3 VP per plan).
- **Task 14 Customs** (sonnet): re-model per §Amendments item 7 (7 spaces, cost pairs, riders wh2/¥2/wh1/¥1 via the 7b helpers, 2P dummy space-4, replace goods_to_index/8-array; KB customs table correction in this commit).
- **Task 15 placement fees + mining** (sonnet, per plan) → **Task 16 Port/Lab enumeration** (sonnet; port test counts re-derived for 6-slot/card-ids) → **Task 17 rest of enumeration** (ORCHESTRATOR FIRST resolves the is-area-action-optional ruling per plan Step 1 note, passes it in dispatch) → **manifest checkpoint** (already VALIDATED, but confirm Jeremy's box-check status) → **Task 18 fuzz gate** (18.1-3 sonnet scaffold, 18.4-5 ORCHESTRATOR-DIRECT debugging; conservation formula per §Amendments 8) → **Task 19 GameRecord** (sonnet) → **Task 20 close-out** (orchestrator).

## Open items on Jeremy

- Physical box check (exceptions-only): box-audit.html priority items + NEW: church AREA board power-2 faith diamond (board art says 2, rulebook text says 1 — engine currently encodes 1; one-line fix if box says 2) and eyeball the 6-slot port/lab layouts.

## Minor findings for the final whole-branch review

- integration.rs setup_2p_game() returns unused (OrderRegistry, TechRegistry) tuple
- legal_moves.rs internal generators still take Option<&TechRegistry> (retire in T16/17)
- apply_fulfill_order's registry param is Option but callers always pass Some (collapse opportunistically)
- Final review dispatch: use requesting-code-review skill's template + review-package script vs merge-base master
