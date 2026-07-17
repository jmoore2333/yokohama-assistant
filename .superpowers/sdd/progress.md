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
- Task 11b six-slot re-model: d20b31d (base 34ba39e; implemented by Jeremy's external agent task11b-boards; review clean — spec ✅, quality approved; 187 green zero warnings, orchestrator-verified)
- Task 12 wire Lab: a39cde0 (base d20b31d; review clean — all 7 amendments verified incl. OTHER-copies-only bonus counting and acting-board-only resolution; 201 green zero warnings, orchestrator-verified)
- Task 13 Church: e397ff1 (base a39cde0; review clean — all 8 rulings verified, mutation-style test check passed, structural no-mutation-before-validation; 218 green zero warnings, orchestrator-verified. Known ⚠️ until T17: legacy Church placeholder in legal_moves can emit moves the executor now rejects — closed by T17 enumeration + Skip)
- Docs commits: 3e3e0db, d1bbfb9, abb5615, 02f1724, a58f2d3 (+ this handoff's docs commit)

## In flight

- (none)

## Orchestration map (plan Execution Model + Jeremy's 2026-07-17 directive: where plan code is stale/under-specified, the ORCHESTRATOR authors the missing spec in the brief — or escalates to a fable subagent — rather than handing stale plan code to sonnet)

- ORCHESTRATOR-DIRECT (no subagent): T17 Step-1 optionality ruling (resolve vs KB+rulebook BEFORE dispatch, decision-log row, ruling verbatim in dispatch); manifest human gate before T18 (manifest VALIDATED already — confirm Jeremy's exceptions-only box-check verbally); T18 Steps 4–5 fuzz-failure diagnosis/fixes (open-ended debugging, one commit per fix, never weaken invariants); T20 close-out narrative + honest gate claim. T13/T14 Step-1 rule checks are ALREADY RESOLVED (decision log rows 2026-07-17) — orchestrator passes rulings verbatim in dispatches.
- STALE PLAN CODE the orchestrator must re-author in briefs: T12 (plan line 671 "search lab_a then lab_b" OVERRIDDEN — acting-area board only; slots[3] refill assertion now slots[5]); T14 (plan steps assume old 8-slot model — amendment 7 + decision-log row are the spec); T16 (all example counts/PortSlot API stale — orchestrator re-derives exact counts for 6-slot/card-id model, reviewer independently re-derives); T17 customs/church enumeration expectations (re-derive under re-models); T18 Step-1 invariant snippet (conservation formula amendment 8: NO +hidden term; .low/.mid/.high API gone; plan's "tech conservation = 20" WRONG → 30 physical ids).
- FABLE escalation: any task whose review loop shows sonnet failing on judgment (per SDD BLOCKED handling); otherwise tiers per plan table (sonnet T12-T19 implementation, T18.1-3 scaffold).

## RESOLVED RULING — area-action optionality (T17 Step 1; orchestrator, 2026-07-17; decision-log row to be added to guidingpath.md after T13 lands to avoid edit collision)

Step 3 (Area Action) is MANDATORY in sequence: rulebook p.7 labels steps 4 and 5 "(optional)", step 3 carries no label, and only Movement Option B "skips steps 3-6". But every area text tolerates zero-effect resolution (Silk Mill "1 Power: No effect"; Port power-1/full-hand "may not take"; p.7 Seiji example — inability ⇒ that component is skipped, steps 4-6 still proceed), and the six choice areas (Port, Laboratory, Employment Agency, Chinatown, Church, Customs) are worded permissively ("may take/gain"). Engine encoding:
1. New `AreaAction::Skip` variant = carry out the area action declining all optional components. ALWAYS legal when the acting area is one of the six permissive areas (guarantees no AreaAction dead-end — e.g. Church with no assistant in hand). Sets acting area + action power exactly like a substantive action (steps 4/5 gate off that power per p.7 Seiji precedent; Recovery collects from the area regardless of yield, p.7 step 6).
2. Production areas + Bank: NO Skip — their deterministic imperative gain ("Gain …") is the sole legal move; zero-effect at low power is inherent (Silk Mill power 1).
3. Empty substantive encodings (Port {[],None}, Chinatown {[]}, Employment {0,0,0}, Customs {0}) are ILLEGAL — validators reject, enumeration never emits; Skip is the single canonical decline. (Church all-false stays substantive: faith = power, placement per T13 rules.)
4. v1 simplification (decision-log flag): Skip grants no area-attached bonuses — Port's 5-power +3 VP requires an actual take (full-hand 5-power Port loses it). Conservative; revisit at Phase 2 choice encoding.
SCOPE SHIFT: `AreaAction::Skip` variant + apply arm land in T16 (Port/Lab are permissive — their real enumeration must emit Skip from day one or empty-board/dry-deck states dead-end); T17 extends Skip to the other four areas and retires placeholders.

## Next (in order)

- **Task 12 wire Lab** (sonnet; brief at .superpowers/sdd/task-12-brief.md). Dispatch amendments (prepared): Move carries card_id → resolve slot by searching THE ACTING AREA'S board only (not both — cross-board buying is illegal; mercenary area resolution like Port's); validate_lab_action currently hard-rejects Lab B / only reads lab_a — rework for the resolved board (now 6 slots); drop tech_deck param (state.tech_deck); acquisition bonuses AFTER success: Patent +2 VP and Letterpress +1 Yen per owned OTHER copy (stack per copy, exclude the just-acquired card), Exposition self-trigger +10 VP (plan gap, rulebook-explicit); Station placement on acquisition = v1 limitation (station_location untouched, comment); check_mercenary_eligibility recheck with card country (make pub(crate) if needed); fifth-tech penalty machinery exists.
- **Task 13 Church** (sonnet): amendments in plan §Amendments item 6 (2P dummy blocks faith-7; riders no-op; v1 auto-place highest eligible ≤ faith; use faith_to_index/next_arrival_order; Winery +3 VP per plan). PLUS: fix `calculate_church_faith` to LINEAR power→faith 1,2,3,4,5 (Jeremy-adjudicated; rulebook text was a typo) with a pinning test for power 2 → 2, and correct the KB faith table (01-rules-reference.md ~line 330) in the same commit.
- **Task 14 Customs** (sonnet): re-model per §Amendments item 7 (7 spaces, cost pairs, riders wh2/¥2/wh1/¥1 via the 7b helpers, 2P dummy space-4, replace goods_to_index/8-array; KB customs table correction in this commit).
- **Task 15 placement fees + mining** (sonnet, per plan) → **Task 16 Port/Lab enumeration** (sonnet; ORCHESTRATOR-DERIVED counts, reviewer re-derives independently):
  - Port moves = singles + pairs + Skip. Accessible cards = filled slots in power-accessible zones (P2→slots 0-1; P3→0-3; P4/5→0-5; P1→none, Skip only). Singles need hand ≤ 2; pairs need hand ≤ 1, distinct accessible cards, one move per affordable payment (TwoYen iff money ≥ 2; OneImportedGood iff imports ≥ 1). Worked pins: P4/all-6-filled/hand-0/money≥2/imports≥1 → 6 + C(6,2)×2 + 1 = 37; P2 same → 2 + 1×2 + 1 = 5; P2 hand-2 → 2 + 0 + 1 = 3; P1 → 1 (Skip).
  - Lab moves per filled ACTING-board slot: deficit = (industry_cost + surcharge[slot]).saturating_sub(min(power,5)); exact-cover mixes k=extra_imported ∈ 0..=min(deficit,imports) with extra_yen = 2·(deficit−k) ≤ money — no overpays; deficit 0 → single {0,0} move. If technologies.len() ≥ 4, each mix forks into affordable fifth_tech_payment variants (Some(true) needs +2 Yen headroom, Some(false) needs a spare import; neither affordable → mix emits nothing). Plus Skip always. Worked pin: P3, slot0 cost3 (deficit 0) + slot2 cost3 (+1 → deficit 1), money≥2, imports≥1, <4 techs → 1 + 2 + 1 Skip = 4.
  - T16 also lands `AreaAction::Skip` (variant + apply arm + Port/Lab emission) per the optionality ruling above; empty encodings (Port {[],None}) become validator-illegal in the same commit. → **Task 17 rest of enumeration** (ORCHESTRATOR FIRST resolves the is-area-action-optional ruling per plan Step 1 note, passes it in dispatch) → **manifest checkpoint** (already VALIDATED, but confirm Jeremy's box-check status) → **Task 18 fuzz gate** (18.1-3 sonnet scaffold, 18.4-5 ORCHESTRATOR-DIRECT debugging; conservation formula per §Amendments 8) → **Task 19 GameRecord** (sonnet) → **Task 20 close-out** (orchestrator).

## Open items on Jeremy

- Physical box check (exceptions-only): box-audit.html priority items (cards) + eyeball the 6-slot port/lab layouts. Church power-2 RESOLVED (linear 1-5, Jeremy 2026-07-17 — fix in Task 13). Port zone access + lab surcharges verbally confirmed by Jeremy same day.

## Minor findings for the final whole-branch review

- T13 review: church error string doesn't distinguish faith<4 vs all-occupied (matters when CLI surfaces reasons, Phase 2)
- T13 review: validate_church_action insufficient-resource paths (yen=true money=0) untested at executor level; next_arrival_order double-placement untested; all-8-full case untested
- T12 review: lab refill tests use single-card board — full-board slide+refill through execute_lab_action not exercised end-to-end (test_lab_slide covers the cascade at helper level; consider a full-board case in T16)
- T12 review: execute_lab_action returns Result<(),MoveError> vs codebase's &'static str executor convention (contained; deliberate for CardNotFound)
- T12 review: no LabBoard::find_card helper — inline .position() duplicates PortBoard::find_card pattern
- T11b review: surcharge table duplicated (types/cards.rs LabBoard::position_modifier vs game/actions/cards.rs lab_position_surcharge — pre-existing; collapse in T12 if natural)
- T11b review: no test takes both cards of one zone (e.g. slots 0+1, power 2) — reviewer hand-traced correct; add in T16 port enumeration tests
- integration.rs setup_2p_game() returns unused (OrderRegistry, TechRegistry) tuple
- legal_moves.rs internal generators still take Option<&TechRegistry> (retire in T16/17)
- apply_fulfill_order's registry param is Option but callers always pass Some (collapse opportunistically)
- Final review dispatch: use requesting-code-review skill's template + review-package script vs merge-base master
