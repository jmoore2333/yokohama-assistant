# Choice encoding, wizard robustness, canal topology: design (2026-07-18)

**Status: DESIGN — approved section-by-section in brainstorm (Jeremy, 2026-07-18).** Follow-on implementation plan is written after Jeremy's written-spec review, per the repo workflow brainstorm → spec → plan → TDD. This phase completes the **Phase-2 gate** (live-track one full physical game without fighting the tool); the roadmap's Phase 3 (computer player) begins after.

**Spec inputs consumed:** guidingpath.md § "Deferred by the Phase-2 final review — REQUIRED spec inputs for the next plan" — items 1–2 (choice encoding, wizard robustness) are gate-blocking; items 3–8 ride and are placed in §5.7. Item 8's canal work (final-review I4) was **pulled in-phase by Jeremy** (2026-07-18): the gate game may be 3–4 players, where an uncrossable Canal can refuse a legal table move.

## 1. Goal

Replace every v1 auto-resolution with a real, player-visible choice the engine offers and the CLI renders, so that a tracked physical game can never silently diverge from the table at a choice point. Concretely:

- any-good picks (v1: auto-copper), warehouse-icon takes (v1: auto-assistant), Church/Customs space choice (v1: auto-highest / auto-leftmost), movement icons on orders/site spaces/church riders (v1: no-op), the 5-Power Movement token (v1: recover-to-hand), University and Textile Mill¹ once-per-turn actions and Telephone's Recovery choice (v1: never offered), and the Skip-forfeits-Port-+3VP conservative reading (revisited as a rules adjudication, §5.3).
- the wizard-robustness cluster: parse-time card validation, re-ask flows, end-of-wizard review/edit.
- canal crossing (I4): rules-legal at 3–4p, currently impossible in the engine.

¹ Printed card title "Textile Mill"; the rulebook text calls it "Spinning Mill"; engine enum `TechEffect::SpinningMill`. The CLI follows the cardboard (cli-guide §8.2).

## 2. What exists (verified against source, 2026-07-18)

- **Auto-resolution sites**: `apply_any_good` (auto-copper) and `apply_warehouse_takes` (auto-assistant, silently partial) in `game/apply_move.rs`, called from `apply_fulfill_order`, `apply_space_reward` (shophouse build), and — via `CustomsTrack::rider` — `execute_customs_action`. Movement icons are documented no-ops at the same three sites. `apply_take_5_power`'s `Movement(n)` arm recovers n assistants to hand. `highest_eligible_church_space` / `eligible_customs_space_for_cost` auto-pick spaces inside `execute_church_action` / `execute_customs_action` (`game/actions/tracks.rs`).
- **Coded-but-unwired choice helpers**: `can_use_university`/`apply_university`, `can_use_spinning_mill`/`apply_spinning_mill` (`game/tech_effects.rs`), backed by `AdditionalActionState.university_used`/`spinning_mill_used`; `apply_recovery_with_leave(state, leave_one)` with the live dispatcher always passing `false`. Zero live call sites (decision log 2026-07-17, Task 20c).
- **Precedent for orthogonal flow state**: the mercenary sub-turn runs on `GameState` fields (`is_mercenary_action`, `mercenary_target_area`, `mercenary_return_phase`), not on fat `TurnPhase` variants; end-of-`apply_move` automation (auto-Recovery, mercenary return, end-trigger check) lives in one block at the bottom of `apply_move`.
- **Record v2** (`GameRecord.events: Vec<RecordedEvent>`): `Move(..)` plus reveal events; bound reveals push in the same `Session::apply` commit as their move; undo = replay-prefix, popping one trailing move with its contiguous preceding bound reveals. `describe_move` (`yokohama-cli/src/describe.rs`) is wildcard-free — new `Move` variants are compile errors until described. `Session::apply` gates on legal-list membership of the post-reveal staged state.
- **Data gaps found**: church move-riders (decision log 2026-07-17: `1,1,2,2,2,2,3,3` for faith 4–11) are absent from `ChurchTrack`; the canonical 5-Power pool (`board_gen::canonical_five_power_tokens`) is 10× Import(1), 2× each Copper(2)/Silk(2)/Seafood(3)/Tea(3)/Money(3) = 20 with **no Movement token** — `FivePowerEffect::Movement` exists only in the type and test fixtures, and the live reveal menu has no movement letter.
- **Canal (final-review I4)**: `board_gen` models the Canal as a full grid node; grid-orthogonal adjacency therefore never connects the areas flanking it; no path can include it (no pawns may be placed there), so the coded 1-Yen toll never fires. The crossing derivation exists as two inline copies (board_gen + reconcile_board). Known latent bug: the toll check iterates `path.windows(2)` and misses the start→path[0] edge. Rule (KB 01-rules-reference.md:400): "No pawns or tokens may be placed. During Movement step: Pay 1 Yen to cross."
- **Wizard**: `ask()` aborts setup on the third invalid answer; card typos in the live tail can surface only at `reconcile_setup`, after all ~30 questions.
- **Rulebook p.10 icon glossary (re-read for this design)**: Any-Trade-good — "Gain 1 Trade good of your choice. If there are more than 2 of these icons, you may choose different Trade goods." Warehouse — "Take a token from the Warehouse tile to your hand. The cost is the same as the Employment Agency action." Movement — "Move an Assistant pawn from your hand or from an Area to another Area. You may treat each movement separately. You do not need to pay other President pawns."
- **Rulebook p.11 Port at 5 Power**: "You may take an Order card from the Low and/or Mid and/or High area. In addition, you gain 3 Victory points."

## 3. Clarifying questions, resolved with Jeremy (2026-07-18)

| # | Question | Resolution |
|---|---|---|
| 1 | Canal/I4: ride post-gate (2p gate game) or pull in-phase? | **Pull in-phase** — the gate game may be 3–4p; canal work lands before the live gate. |
| 2 | Assistant-move entry UX: numbered-only, grammar, or two-step engine encoding? | **Flat engine moves + `m` grammar** with the numbered list as fallback (placement precedent). |
| 3 | Architecture: pending-choice queue, parameterized moves, or hybrid? | **Pending-choice queue** (BGA sub-state model, recommended). |

## 4. Approaches considered

1. **Pending-choice queue (chosen)** — executors push small `PendingChoice` items instead of auto-resolving; `get_legal_moves` surfaces only the head choice's options while the queue is non-empty; one small `Move` resolves one choice. Matches the BGA reference architecture guidingpath's evidence base endorses ("every sub-choice is a state emitting its own legal-option list"); no record-schema change; cascades and replay-prefix undo compose naturally; `GameState` stays cheap to clone (the queue is a handful of small entries at most).
2. *Parameterized composite moves* — fold choices into the triggering move's parameters. Rejected: movement icons cross-product the enumeration (two icons ≈ tens of thousands of moves — unusable option lists, fuzz throughput hit, MCTS width explosion) and cascaded rewards (church space → its riders) force nested parameter structures.
3. *Hybrid* (church/customs space as `AreaAction` parameters, queue for icon rewards only) — rejected: the queue must exist anyway, church enumeration multiplies ~8× (payment subsets × spaces), and a second mechanism costs uniformity for a few saved record events.

## 5. The design

### 5.1 The pending-choice queue

```rust
// types/state.rs
pub struct GameState {
    // ...
    /// Unresolved reward/recovery choices for the current player, FIFO.
    /// Serialized state (not history); length is bounded by one reward
    /// bundle's icon count plus its cascades (typically <= 3, always small).
    pub pending_choices: Vec<PendingChoice>,
}

pub enum PendingChoice {
    AnyGood,                          // gain 1 trade good of choice
    WarehouseTake,                    // take 1 warehouse token at Employment cost, or decline
    AssistantMove,                    // move 1 assistant (hand/area -> area), or decline
    ChurchSpace  { total_faith: u8 }, // pick an eligible empty faith space
    CustomsSpace { cost: u8 },        // pick within the printed-cost pair
    RecoveryLeave,                    // Telephone: leave 1 in the acting area, or recover all
}
```

Mechanics, in order of authority:

- **Gating**: `get_legal_moves` checks `pending_choices` FIRST. Non-empty ⇒ the legal list is exactly the head choice's resolution moves — in every phase, including mercenary sub-turns and Recovery. Empty ⇒ the existing per-phase generators run unchanged.
- **Pushing**: executors that today call `apply_any_good`/`apply_warehouse_takes`/the movement no-op/auto-space/`Movement(n)`-recover push choices instead. All rewards accrue to `current_player`; the queue never crosses a player hand-off.
- **Singleton auto-resolution**: a choice whose legal resolution set has exactly one element — including "nothing available/affordable" (e.g. a warehouse take with an empty warehouse and no affordable token) and "exactly one eligible church space" — resolves inline at push time and never enters the queue. No wasted prompt, no search node, and no divergence risk: the table player faced the same single outcome.
- **Deferral**: the end-of-`apply_move` automation block (auto-Recovery, mercenary return, phase hand-off) runs only when `pending_choices.is_empty()`. Resolving the last queued choice re-enters that block in the same `apply_move` call, so e.g. Building-space rewards resolve, then Recovery runs (or queues `RecoveryLeave`), then the mercenary return or `AdditionalActionSecond` transition fires — exactly today's ordering with pauses at choice points.
- **Cascades**: resolutions may push further choices (church space → its movement riders; customs space 1/3 → warehouse takes; `Movement(n)` token → n assistant moves). Cascade depth is data-bounded and small.
- **Record & undo**: choice resolutions are ordinary `RecordedEvent::Move` entries — **no schema change**. Replay folds them like any move; replay-prefix undo pops one choice at a time (good for mis-picks at the table); the triggering move's bound reveals stay grouped with it, and choice moves never draw cards, so the reveal-binding invariant is untouched. A mid-cascade save replays back to the mid-cascade prompt.
- **Invariants (added to `check_invariants` / the fuzz corpus)**: queue empty at every player hand-off and at `TurnEnd` (asserted in `advance_phase`); queue non-empty ⇒ `get_legal_moves` non-empty (decline/singleton-auto guarantee no dead ends); all existing conservation rules unchanged (assistant moves conserve the 20-pawn total; warehouse takes conserve per-player token pools).

### 5.2 Choice inventory and new Move variants

Each new variant gets one plain-words `describe_move` arm; `describe_move` stays wildcard-free so the compiler enforces coverage.

| PendingChoice | Resolution moves | Options | Fires from |
|---|---|---|---|
| `AnyGood` | `ChooseAnyGood { good: TradeGood }` | 4 | order rewards (`any_trade_good`), site space rewards (`any_good`) — one queue entry per icon (p.10: independent picks) |
| `WarehouseTake` | `ChooseWarehouseTake { item: WarehouseItem }`, `DeclineWarehouseTake` | ≤4 | order rewards, site spaces, Customs space-1 (×2) and space-3 (×1) riders |
| `AssistantMove` | `ChooseAssistantMove { from: MoveSource, to: AreaId }`, `DeclineAssistantMove` | up to ~300 + decline | order rewards (`movement`), site spaces, church riders, `FivePowerEffect::Movement(n)` — one entry per icon (p.10: "each movement separately") |
| `ChurchSpace` | `ChooseChurchSpace { space: u8 }` | 2–8 | `AreaAction::Church` after payment, when ≥2 eligible spaces |
| `CustomsSpace` | `ChooseCustomsSpace { space: u8 }` | 2 | `AreaAction::Customs` after payment, when both spaces of the cost pair are open |
| `RecoveryLeave` | `ChooseRecoveryLeave { leave_one: bool }` | 2 | entering Recovery with Telephone owned and ≥1 assistant in the acting area |
| — (not queued) | `UseUniversity`, `UseSpinningMill` | standing | `generate_additional_action_moves`, both windows |

Supporting types: `TradeGood { Copper, Silk, Tea, Seafood }`, `WarehouseItem { Assistant, Shophouse, TradingHouse }`, `MoveSource { Hand, Area(AreaId) }` (reuse existing enums if the plan finds equivalents; otherwise these are new small types in `types/`).

Per-choice semantics:

- **`ChooseAnyGood`** — gain 1 of the chosen good. No decline (the gain is unconditional). Mining Technology applies at resolution when the chosen good is Copper, as its own receipt (§5.3-2).
- **`ChooseWarehouseTake`** — legality per item: Assistant if `in_warehouse ≥ 1` (free); Shophouse if `in_warehouse ≥ 1` and money ≥ 2; Trading House if `in_warehouse ≥ 1` and money ≥ the printed 4/5/6/7 take progression indexed by tokens already out (hand + built) — re-priced per take, so consecutive TH takes cost 4 then 5, matching the Employment Agency. Charges at resolution. `DeclineWarehouseTake` always offered (§5.3-3). The take-progression pricing is extracted into one shared helper, collapsing the existing three copies (ledger minor) plus this new call site.
- **`ChooseAssistantMove`** — `from` ∈ {Hand (if `in_hand ≥ 1`)} ∪ {areas holding ≥1 of the mover's own board assistants}; `to` = any non-Canal area ≠ `from`. Opponent Presidents at the destination are ignored and unpaid (p.10); own President irrelevant; management-board (Church/Customs/Achievement) assistants are locked and are not sources; destination is always an area (no move-to-hand). `DeclineAssistantMove` always offered per icon (the icon is permissive).
- **`ChooseChurchSpace`** — `AreaAction::Church` validates exactly as today (faith ≥ 4, ≥1 eligible empty space at ≤ total faith respecting the 2p faith-7 dummy, assistant available in hand or the acting area per I3), pays the goods, computes `total_faith`, then: exactly one eligible space ⇒ place inline (unchanged v1 behavior, now by necessity rather than policy); ≥2 ⇒ queue. Resolution places the assistant (hand preferred, acting-area fallback — I3 semantics unchanged), awards the space VP + Winery bonus, and queues the space's movement riders (new data: `ChurchTrack::movement_rider(space)` = `1,1,2,2,2,2,3,3` for faith 4–11, decision log 2026-07-17).
- **`ChooseCustomsSpace`** — same shape: `AreaAction::Customs` validates (exact printed-cost match, ≥1 open unblocked space of that cost, assistant available), pays the Imported Goods; one open space of the pair ⇒ place inline; both ⇒ queue. Resolution places, awards the space VP + Brickyard bonus, applies the space's riders — Yen immediately, warehouse takes as queued `WarehouseTake` choices.
- **`ChooseRecoveryLeave`** — Recovery stops being unconditionally automatic: entering it with Telephone owned and ≥1 own assistant in the acting area queues this choice (turn_phase stays `Recovery`; `get_legal_moves`' queue-first check makes the Recovery arm interactive exactly then). `leave_one: true` leaves exactly 1 and recovers the rest — **including the count == 1 case, where it recovers zero**; the current helper wrongly recovers a lone assistant, which this fixes (§5.3-5). `false` = recover all (today's behavior). Without Telephone (or with no assistants to recover) Recovery stays fully automatic.
- **`UseUniversity` / `UseSpinningMill`** — standing options in `AdditionalActionFirst`/`AdditionalActionSecond` (and therefore inside mercenary-return windows, which are those phases). University: owns the tech, `!university_used`, money ≥ 1, warehouse assistants ≥ 1 ⇒ pay 1 Yen, assistant warehouse → hand. Spinning Mill: owns the tech, `!spinning_mill_used`, silk ≥ 1 ⇒ discard 1 Silk, gain 2 Yen. Both wire the existing tested helpers. Window-equivalence adjudication: §5.3-4.
- **`FivePowerEffect::Movement(n)`** — `apply_take_5_power` stops recovering to hand and queues n `AssistantMove` choices (CLAUDE.md's standing rule note: the token moves assistants, with the same icon semantics).

### 5.3 Rules adjudications (decision-log rows at merge; Jeremy veto applies)

1. **Port 5-Power +3 VP attaches to the action, not the take.** p.11: "You may take an Order card from the Low and/or Mid and/or High area. In addition, you gain 3 Victory points." — the VP sentence rides the 5-Power row, whose take is itself optional ("may"), and a full-hand player is barred from taking yet still acted at 5 Power. The engine grants +3 VP on any 5-Power Port area action — take or `Skip`, main turn or mercenary sub-turn — implemented at one site keyed on (acting area is a Port, power == 5). SUPERSEDES the v1 conservative flag on the 2026-07-17 optionality ruling ("Skip grants no area-attached bonuses").
2. **Mining Technology receipts are per-icon.** Each `ChooseAnyGood` resolving to Copper is its own copper receipt, firing the flat per-copy Mining bonus independently of the reward bundle's explicit `copper` field (whose receipt fires at the triggering move). Mirrors the per-exchange Chinatown precedent (T15b). Changes totals only where one reward bundle has both explicit copper and a copper-chosen any-good; the combined-receipt precompute in `apply_build_shophouse` is dismantled accordingly.
3. **Warehouse-icon takes are declinable.** p.10 words the take imperatively but prices it; a player who can pay cannot be compelled to. Offering `DeclineWarehouseTake` is over-permissive-safe (tracking must never reject a real table move) and costs search little.
4. **Once-per-turn techs are offered in the two additional-action windows only.** A mid-turn physical use (e.g. University between Placement and Movement) is state-equivalent to using it at the nearest window: both effects touch only fungible resources (money/goods/hand assistants), and the windows bracket every payment site. The tracker enters it at the window; the resulting state is identical.
5. **Telephone may keep a lone assistant.** "May leave 1 assistant in the action area" applies at count == 1 (recover zero). The current `apply_recovery_with_leave` recovers a lone assistant even with `leave_one = true`; its pinned test is corrected as part of the wiring.
6. **Movement-token pool composition is unverified.** The engine's canonical pool has no Movement token; the project's rules notes say the token exists. Goes on Jeremy's exceptions-only box check: count the 20 tokens by face. The engine encodes `Movement(n)` correctly regardless; if the box check confirms movement tokens, `canonical_five_power_tokens` gains them (data fix + fuzz rerun); the CLI reveal menu gains `m` now either way (§5.4).
7. **`MoveError::WrongPhase` is removed** (recommendation). The Session legal-list gate plus replay-through-`apply_move` validates untrusted records more strongly than a phase check; a dead variant is the "misleading, not just dead" class the 2026-07-18 deletion row targeted. (Alternative if vetoed: wire it as a defense-in-depth phase check in `apply_move`.)

### 5.4 CLI surfacing

- **Choice banner**: while the queue is non-empty, the prompt header's phase slot renders the head choice and the remaining count — e.g. `Choose: warehouse take (assistant free / shophouse 2¥ / TH 6¥) — 2 choices pending`. Rendered from the head `PendingChoice` alone (no provenance plumbing; the table sequence supplies context). New `describe_pending_choice` helper beside `describe_move`.
- **`m` grammar (assistant moves only)**: `m <area> <area>` and `m hand <area>`, parsed to `ChooseAssistantMove` and exact-matched against the legal list (never bypasses the generator; a miss explains why). Decline stays a numbered option. All other choice types are ≤8 options, numbered-only. Supersedes the "placement is the only direct-grammar phase" decision row (now: placement + assistant moves).
- **Delta lines** (riding item 5 lands here — the new moves must print trustworthy at-the-table confirmations): assistant moved from→to, warehouse item taken + cost, church/customs placement with space/VP/riders, buildings built, mercenaries and 5-Power tokens gained, assistants placed/recovered — extending the existing money/VP/goods/hand/tech/president/phase/deck lines.
- **Reveal menu**: the 5-Power flip prompt gains `m` = Movement token (entered as `Movement(2)` pending the box-check verdict, §5.3-6). Consistent with `reveal_five_power_token`'s replace-outright semantics: tracking never rejects a physically observed token.
- **Choreography unchanged by construction**: choice moves never draw, so the scratch-apply diff finds zero draws mid-cascade; reveals still bind to the triggering area-action move; the desync tripwire and undo grouping are untouched.
- **Docs**: `docs/cli-guide.md` + `.html` are updated in-phase — §8.6's auto-resolution list is retired, replaced by the choice prompts, the `m` grammar, the review step, the new reveal letter, and the canal fix note. The testing group runs on this guide; updating it is part of the definition of done.

### 5.5 Wizard robustness (gate-blocking item 2)

1. **Parse-time validation at every card question**: registry membership; duplicate checks against everything already entered (across Port A+B and the deals; across Lab A+B); site numbers in-registry and distinct; board tiles validated against the legal tile multiset as entered (an over-count is rejected at that question, not after the ~30-question tail). Tech-prefix ambiguity keeps its current wizard-time REJECT semantics (physical-identity recording, deliberate divergence from the REPL's lowest-id pick — now documented in the guide). `reconcile_setup` stays the unchanged backstop.
2. **Re-ask replaces 3-strikes everywhere** (sim and live tails): invalid input re-asks indefinitely with the validation error shown; the only exits are a valid answer or an explicit `abort`.
3. **End-of-wizard review/edit**: after the last question, a full summary of every answer (mode/players/colors/start seat/seed/save path; live: tile grid, sites, achievements, port/lab contents, deals + keeps, hidden seats). Then a `confirm` / `edit <section>` loop; editing re-runs that section's questions with the same parse-time validation. Only `confirm` proceeds to `reconcile_setup` + `finalize_setup`.

The wizard rework touches all four near-verbatim prefix-parse copies and the triplicated `tech_name` one-liner — the shared helpers land with it (riding item 4).

### 5.6 Canal topology (final-review I4, pulled in-phase)

The Canal becomes an **edge, not a node**, for movement:

- `board_gen` derives, for the Canal's grid cell, the pairs of areas straight across it (opposite orthogonal neighbors — in a 2-row grid possibly only the E–W pair), adds those pairs to the adjacency graph, and records them in `canal_crossings`. The Canal tile itself remains pawn-free and path-unreachable. The derivation becomes one shared function used by both `board_gen` and `reconcile_board` (closing the two-inline-copies riding note).
- The movement BFS needs no rule changes: crossing works exactly when the far-side area satisfies the unchanged own-assistant path rules. `calculate_movement_cost_full` already prices tolls via `is_canal_crossing`; the latent `path.windows(2)` bug (start→first-step edge unchecked) is fixed in the same wave, since it goes live the moment crossings are reachable. Canal tolls go to the bank (existing semantics, now finally exercised).
- Structurally unaffected: Tram/Station teleports (`path: vec![]`, no toll by design), `PlacePresident` (no path), mercenaries (no movement), 2p boards (no Canal).
- Task 1 of the canal cluster is the standing rules check — rulebook p.6 + board art: confirm crossing is straight-across-only and the toll is per crossing — adjudicated by the orchestrator with a decision-log row (escalate to Jeremy only if contested).
- Per Jeremy's call this ships inside this phase's plan as its own task cluster (no separate mini-plan file); the final review's "don't patch it casually" concern is met by this dedicated design section plus dedicated tasks with their own rules-check step.

### 5.7 Riding-backlog placement (spec items 3–8)

| Backlog item | Lands |
|---|---|
| 3a — load/replay errors report the failing event index (§5.5 promise) | `Session::load`/`replay` error path task |
| 3b — tech-board desync assert (parity with the port one) | with the `reveal.rs` extraction |
| 3c — `File::sync_all` before the save rename | save task (one-liner + test) |
| 3d — typed desync outcome replacing the `"Illegal:"` string coupling | `Session::apply` returns a typed error enum; repl updates |
| 4 — display/dedup sweep | `space_cost(i)` single-sourcing + 2p church faith-7 dummy marker in `tracks_view` land with the tracks-choice tasks; `reveal.rs` extraction from the 1100-line `repl.rs` lands with the grammar/banner work; prefix-parse + `tech_name` helpers with the wizard rework; president-label consistency with the delta-line work; canal-derivation dedup with §5.6 |
| 5 — delta_summary extensions | §5.4, with the choice moves |
| 6 — explicit `is_over()` gate in `Session::apply` | one guard + test |
| 7 — dogfooding smalls | `q` in HELP_TEXT; view commands stop reprinting the full prompt block; wizard-ambiguity divergence documented in the guide |
| 8 — carried engine items | I4: §5.6 (in-phase). `WrongPhase`: §5.3-7 (remove, pending veto). Box check + I5 confirmation: standing on Jeremy, unchanged |

Out of scope, unchanged: Station token placement on acquisition, the from-hand Tram question, TUI/egui, all Phase-3 bot work.

## 6. Testing and gates

- **TDD per task**; implementer reports must paste literal RED output before implementing (standing rule; enforced after a Phase-2 task skipped it).
- **Engine unit tests**: per choice type — push/gate (queue non-empty ⇒ only head options legal), singleton auto-resolution, decline paths, TH take re-pricing across consecutive takes, cascades (church space → riders; customs space-1 → 2 warehouse takes; `Movement(n)` → n moves), church rider data values, Telephone count==1, +3-VP-on-Skip at 5-power Port, per-icon mining receipts, University/SpinningMill once-per-turn flags, canal crossing legality + toll incl. the start-edge case.
- **Property tests**: the replay-determinism and reveal-equivalence suites extend to records containing choice moves; a mid-cascade save/load round-trip replays to the mid-cascade prompt.
- **Invariants**: §5.1's two new checks join the fuzz corpus's standing nine.
- **CLI scripted-session tests**: choice banner + numbered resolution; `m` grammar hits and misses; undo mid-cascade (one choice per undo) and undo through a triggering move with bound reveals; a live cascade end-to-end; wizard parse-time rejection, indefinite re-ask, `abort`, and review/edit round trips; the golden mini-game re-recorded (choice moves change the transcript).
- **Gates**: both suites green + zero workspace warnings after every task; `cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored` before merge; full 5000×3 selfplay sweep with **new baselines** at phase close — move distributions shift by design (random play now resolves choices; 3–4p movement gains canal edges; game lengths and per-phase branching will move). Then the **phase gate itself**: live-track one full physical game start to finish; with the canal fix any player count qualifies.

## 7. Execution model

Superpowers writing-plans → subagent-driven development, on a fresh branch off master (`choice-encoding`). Sonnet implementers with a per-task reviewer; fable only for the final whole-branch review; ledger current in `.superpowers/sdd/progress.md` (new phase section). Rules questions resolve against `analysis-1/yokohama-kb/` + the rulebook PDF; the orchestrator adjudicates with decision-log rows; genuinely contested facts go to Jeremy. Never touch `.codex/`, `.vscode/`, `AGENTS.md`, `tts-assets/`; `git add` specific paths only; commit style `feat:/fix:/test:/docs:`. `guidingpath.md` is updated at the phase transition; the phase is never claimed complete without the live-game gate.

## 8. Decisions for Jeremy's veto (decision-logged on merge)

1. Pending-choice queue as a `GameState` field (serialized state, not history); singleton choices auto-resolve without a record event.
2. Port 5-Power +3 VP granted on the action, not the take (reverses the v1 Skip-forfeits reading) — §5.3-1.
3. Per-icon Mining receipts — §5.3-2.
4. Warehouse takes declinable — §5.3-3.
5. University/Spinning Mill offered in the two additional-action windows only (state-equivalence argument) — §5.3-4.
6. Telephone leaves a lone assistant (corrects a pinned test) — §5.3-5.
7. `MoveError::WrongPhase` removed rather than wired — §5.3-7.
8. Canal ships as a task cluster inside this plan, not a separate mini-plan file — §5.6.
9. The `m` reveal-menu letter enters `Movement(2)` while the physical pool composition awaits the box check — §5.3-6.
10. Assistant-move entry grammar `m <src> <dst>` (supersedes "placement is the only direct-grammar phase").
