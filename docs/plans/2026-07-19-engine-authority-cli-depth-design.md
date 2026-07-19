# Engine-authority + CLI-depth design (2026-07-19)

**Status:** APPROVED section-by-section by Jeremy (2026-07-19, live session). Spec input: `docs/plans/2026-07-19-phase3-entry-backlog.md` (verified Codex PR-#1 triage — the REQUIRED input per repo convention). Next artifact: the dated implementation plan (superpowers writing-plans), then SDD execution on branch `engine-authority`.

## 1. Context and goals

Choice-encoding merged to master at 7110f55 (PR #1). The Phase-3 scope decision (Jeremy, 2026-07-19, after challenge): **the entry backlog is its own plan, landing before the computer-player spec.** The deciding argument is producer-consumer: Tier 1 defines the contract the bot consumes — the arena cannot count wins without the winner API; bots apply moves by construction (the standing I1 lesson) and need the checked boundary; route enumeration defines the action space a tuned evaluator depends on; training records need the rules-hash. Landing the contract first keeps the bot spec clean and stable.

Two goals:

1. **Engine authority** — every legal table move is representable (no projection collapses), every applied move passes one checked public boundary (no bot- or record-constructed move bypasses validation), and terminal results are authoritative (winner, official tie-break). These properties make self-play records trustworthy training data and the live gate game fully recordable.
2. **CLI depth** — the ASCII interface reaches playtest-pleasant: the economy visible, rewards printed where decisions happen, the achievement race visible, history rendered faithfully, semantic color.

**The live gate game (Phase-2 gate, never claimable by a plan) runs AFTER this plan merges** (Jeremy, 2026-07-19). No mid-branch build target; normal dependency ordering applies.

## 2. Non-goals

- **No bot/search work.** The computer-player spec starts after this plan lands, written against its contract. This plan's Tier-1 surfaces may be *extended* by the bot spec, not broken.
- **No rustfmt.** Deferred per Jeremy 2026-07-19 (whole-repo reformat; revisit at a later boundary).
- **No polished-CLI translation layer, no ASCII-art-first screens, no graphical front end.** Recorded as the future direction (§8-10 note, decision log 2026-07-19): a more polished, ASCII-art-heavier CLI with a translation layer for referencing areas/moves, possibly a graphical end downstream. This plan stays engine + simulator focused; T2.5's bounded color/art uplift does not foreclose any of it.
- **No presentation in core.** The one-way glass: machines consume `GameState`/`get_legal_moves`/records/`GameResult`; `describe`/`render` are pure state→text functions in `yokohama-cli`. Reward formatting, panels, and color never migrate into `yokohama-core`.

## 3. Scope overview

| Tier | Item | One-line shape |
|---|---|---|
| 1 | 1. Checked transition boundary | Safe `apply_move` (legal-list membership), raw → `apply_move_unchecked`, strict replay, board-membership invariants |
| 1 | 2. Station placement | Queued placement choice on tech acquisition; token aura + teleport come alive |
| 1 | 3. L2 route enumeration | One `MovePresident` per distinct (destination, mover cost, recipient-credit vector); canonical path per label |
| 1 | 4. Head-of-queue normalization | Auto-resolve heads that *become* `{Decline}`-only after earlier resolutions |
| 1 | 5. Winner/tie-break API | `GameResult` with official tie-break; CLI announces winner + breakdown |
| 1 | 6. Record rules-hash | Stable rules-data hash stamped into `GameRecord`; strict replay rejects mismatch |
| 1 | 7. Option B recovery enumeration | Per-area counts encoding; any-number recovery enumerated; retreat grammar |
| 2 | T2.1 Status panel | Full per-player economy: assistants/shophouses/trading-houses by zone |
| 2 | T2.2 Rewards in prompts | `site_compact_line` wired everywhere decisions happen; movement costs + recipients; action previews |
| 2 | T2.3 Achievements view | Three active achievements: condition, claim state, per-player progress |
| 2 | T2.4 Log fidelity | Historical descriptions via per-event `replay_from` fold (the a6faa87 `record_stats` machinery) |
| 2 | T2.5 Color/art uplift | Semantic ANSI color (seat colors, goods, VP) + box-drawing board; TTY-gated, `NO_COLOR` respected |
| 3 | Hygiene + pins | `base_industry_cost` delete; guide alignment; ruling pin tests; §5.3-7 row correction |

Estimated 16–22 TDD tasks; Tier 1 sequenced before Tier 2; Tier 3 rides where it touches.

## 4. Tier 1 — engine authority

### 4.1 Checked transition boundary, strict replay, board invariants (backlog item 1)

Current reality (verified 2026-07-19): `apply_move` validates per-arm rules but not phase (exactly one `turn_phase` check exists — the recovery gate, `apply_move.rs:158`) nor membership in `get_legal_moves`; `replay_from` → `Session::load` → `record_stats` call it directly, bypassing the Session legal-list gate. A tampered or bot-constructed record can apply out-of-phase moves, unvalidated movement paths, and area IDs never bounds-checked against the board.

Design:

- **The safe path gets the obvious name.** New public `apply_move(state, mv, rules)` = membership check against `get_legal_moves` + apply. The current raw transition is renamed `apply_move_unchecked` — public, because `Session`'s prechecked fast path and future search hot loops use it deliberately after choosing from an enumerated legal list.
- **Call-site audit (flag A — the riskiest migration step).** Every existing caller is sorted deliberately: checked (replay, `Session::load`, `record_stats`, any external/bot input) vs. unchecked (fuzz driver, selfplay example, Session's post-gate apply — all already choose from the enumerated list). Sorting wrong silently doubles enumeration cost; the release fuzz gate (~8s) is re-run as task evidence.
- **Board-membership invariants.** `check_invariants` gains bounds checks: every stored `AreaId` (president locations, `assistants.on_board` keys, shophouse/trading-house `built`, `station_location`, achievements, pending-choice references) must exist in `board_layout.areas`.
- **Strict replay mode** for training pipelines: every event's move must pass the checked transition; invariants run per event; `engine_version`/rules-hash mismatch **rejects** (typed error), never warns. Interactive `Session::load` stays on the checked path but keeps warn-and-continue on version mismatch (engineering call: no interactive-UX regression; strictness is the training entry point's contract).
- **Doc correction (backlog note):** guidingpath's §5.3-7 row justification ("replay-through-apply_move validates untrusted records more strongly than a phase check") is wrong for the load path and is corrected when this lands.

### 4.2 Station token placement on acquisition (backlog item 2)

Verified dead today: `station_location` is set only in tests; the token, its +1-power-for-all aura, and the teleport never occur in production play.

Design: acquiring the Station tech queues a placement choice through the existing pending-choice architecture. No Decline (the token must land); if exactly one eligible area exists, place inline — consistent with the pinned no-Decline-singleton invariant. Exact placement constraints (any area? Canal excluded? occupancy?) are verified from the card text at plan time; Jeremy adjudicates ambiguity. Live tracking records the table player's announced placement as an ordinary choice move; sim random-plays it. Until this lands the phase gate remains "unblocked-modulo-Station" — this item completes the unblocking.

### 4.3 Route enumeration at L2 — full route freedom (backlog item 3; Jeremy's ruling 2026-07-19)

Ruling (supersedes the I3 row's single-path limit): enumerate **one legal move per distinct (destination, mover cost, recipient-credit vector)** over simple paths. Income-denial routing (paying 2¥ in bank tolls to avoid handing the leader a 1¥ fee) is real play and becomes recordable; so does recipient choice among equal-cost routes.

Design: label-based search per destination — label = (mover cost, credit vector: `BTreeMap<Recipient, Yen>` where Recipient ∈ {Bank, PlayerId}). Per node, keep the set of distinct labels (bounded small: tolls ≤ crossings, fees ≤ opponent-president encounters). Emit one `MovePresident` per label with the **canonical (lexicographic-min) path** as representative — no `Move` schema change, the path disambiguates. A table player's non-canonical path with the same money flow is state-identical, so canonical recording is sound. The landed min-cost route remains the first-listed canonical option per destination (there is no "default" in random self-play — uniform draw over the legal list, as always). CLI: destination-first; route sub-prompt only when >1 label ("route paying 1¥ to bank" vs "route paying 1¥ to Blue"). Dominated routes appear in the legal list (authority = the list is the truth); search may filter later — a helper can land in the bot phase, not here.

### 4.4 Head-of-queue normalization (backlog item 4; builds on §8-1 as confirmed)

`push_choice` singleton-resolves at push time only; a queued choice that *becomes* singleton (two WarehouseTakes, one assistant: the second offers only Decline) still prompts. Design: after every apply that leaves the queue non-empty, recompute the head's option set; a structurally `{Decline}`-only head auto-resolves silently, repeating down the queue. Deterministic (pure function of state), replay-stable, covered by the delta-window no-op argument and the extended two-prong invariant test.

### 4.5 Winner/tie-break API (backlog item 5)

`final_result(&GameState, &Rules) -> GameResult`: per-player standings with the official tie-break (KB 01-rules-reference.md:556 — highest VP; ties broken by clockwise closeness to the start player, a unique total order, so ranks are never shared). `GameResult` carries winner, ranked standings, and a per-source score breakdown (if `calculate_final_scores` returns bare totals today, the breakdown struct is a small extension — verified at plan time). CLI game-over screen announces winner + breakdown. This is the bot phase's terminal utility.

### 4.6 Record rules-hash (backlog item 6)

`GameRecord` gains a stable hash over rules data: canonical serialization of `Rules::standard()` content (BTreeMap-ordered — hash stability is a known trap, handled by construction) plus a board-derivation version constant bumped on topology changes. Stamped at record creation; strict replay rejects mismatch; interactive load warns. `engine_version` stays informational.

### 4.7 Option B recovery enumeration (new item, found 2026-07-19; Jeremy's I5 wording)

Verified gap: the generator emits exactly one retreat move — recover **all** assistants from **all** areas (`legal_moves.rs:281,357`) — and `recover_assistants_from: Vec<AreaId>` cannot express partial recovery within an area. Jeremy's ruling: "any number of them" is legal.

Design: encoding becomes a per-area **counts map** (`BTreeMap<AreaId, u8>`); generation enumerates any-number recovery (product of (count+1) per area — placement-scale combinatorics, bounded and honest). **Schema break approved by Jeremy (2026-07-19): clean break, no migration shim** — the v1→v2 precedent; test fixtures updated; dogfood-era saves declared non-loadable and documented (no gate game has been played, no training data exists). CLI: numbered when small; a small retreat grammar (placement-grammar precedent — shape settled at plan time) when large. Delta lines already describe recovered assistants; verified against the new variants at plan time.

## 5. Tier 2 — CLI depth

Constraint on all items: the one-way glass (§2). All presentation in `yokohama-cli`.

- **T2.1 Status panel.** `players_panel` grows the full economy per player: assistants (hand/board/warehouse), shophouses (warehouse/built), trading houses (hand/warehouse/built + progression position), alongside money/VP/goods/orders/techs/president. One glance answers "what can they afford and build."
- **T2.2 Rewards where decisions happen.** The never-wired `site_compact_line` format (`S03 [1vp 2vp+1yen - 2tea] TH 8vp`) renders in building prompts (options show the space's printed reward, not "space N") and a sites/area-detail view; the guide's documentation of it becomes true (closing the Tier-3 stale-guide item in the same stroke). Movement options print cost + recipient — merged with L2's route sub-prompt display. Area-action prompts show current power + production preview ("Copper Mine at power 3 → +3 copper").
- **T2.3 Achievements view.** A command listing the three active achievements: condition, claim state, per-player progress (counting functions exist in `achievements.rs`).
- **T2.4 Log fidelity.** Historical descriptions computed against the state *at each event* by folding `replay_from` per event — the a6faa87 `record_stats` machinery — cached in `Session`, rebuilt on load/undo. No schema change; state-dependent descriptions (TH take price) render historically correct.
- **T2.5 Semantic color + board art (approved addition, 2026-07-19).** Seat identity is color (the Phase-2 names-dropped decision), so: player pieces/panels/president markers in seat colors; stable goods colors (copper/silk/tea/seafood/yen); VP and prompts styled; desync warnings red. Board view gains box-drawing grid + colored cells — monospace-safe, no full-screen TUI. A tiny ANSI style helper in the CLI crate; color **auto-disabled when stdout is not a TTY** and `NO_COLOR` respected — keeping piped smokes and expected-output tests byte-deterministic.

## 6. Tier 3 — hygiene and ruling pins

- Delete `TechEffect::base_industry_cost` (zero callers, "misleading, not just dead" class).
- Guide alignment: the stale 2p church-dummy "Known display gap" box; site-line docs align with T2.2 (rides that task).
- **Ruling pin tests** (each a test + decision-log presence, most landed as rows on design commit): Tram board-only; Language School Yen-first (3¥ + LS → afford-4¥-take); place-0 + any-number retreat per Jeremy's I5 wording; audit that no-return-to-start / no-same-area-action-twice has a pin (add if missing).
- §5.3-7 row justification correction (rides 4.1).

## 7. Testing, gates, and evidence

- TDD per task (RED first); suites + `cargo clippy --workspace --all-targets -- -D warnings` green, zero warnings, after every task.
- Release fuzz gate before merge (`cargo test --release -p yokohama-core --test fuzz_full_games -- --ignored`); watch gate time against the 4.1 call-site audit (flag A).
- Close-out: 5,000×3 selfplay sweep with **new baselines documented** — Movement branching will rise visibly (L2 + Option B, flag B); the design declares this expected so it never reads as a regression.
- **Organic census extension** (choice-encoding precedent): Station placement choices, multi-label route offerings, and partial Option-B recoveries must each fire organically within a bounded seed range.
- Strict replay round-trips: record → strict replay → PartialEq-identical state; tampered move / out-of-phase move / hash mismatch → typed rejection (not warn, not panic).
- Wizard/live surfaces unaffected by design (no reveal-choreography changes: choice moves never draw); regression suites stand guard.

## 8. Rulings and decisions recorded this session (2026-07-19, all Jeremy-decided live)

1. Scope: backlog = its own plan before the bot spec; gate game after merge.
2. Veto pass closed: §8-3/4/5/6/8 confirmed; §8-1 confirmed as-is; §8-10 confirmed with the future-polish direction note; §8-2/7/9 previously resolved.
3. I3 → **L2 full route freedom** (supersedes the min-cost-only limit; the min-cost route stays the first-listed canonical option).
4. I5 confirmed with expanded wording (place-0 legal; any-number retreat; president must still move legally; no same area action twice) — exposing the Option-B projection gap → item 7. Clean schema break approved.
5. From-hand Tram: **board-only** (president in hand only turn-1/no-techs or post-retreat, where the normal placement rule governs; Tram's benefit is pathless on-board movement).
6. Language School: fulfill-Yen is an ordinary fulfillment benefit, available for subsequent payments including a same-order warehouse take.

Engineering calls made in this design (orchestrator, flagged not vetoed): checked function takes the `apply_move` name; interactive load warn-and-continue on version mismatch (strict mode is the training contract); canonical-path-per-label recording; log fidelity via per-event `replay_from` fold; color TTY-gating.

## 9. Sequencing

Tier 1 first (4.1 early — it hardens everything after it; 4.2/4.3/4.7 next — the projection completers; 4.4/4.5/4.6 close the tier), Tier 2 second (T2.2 depends on L2's display needs; T2.4 on `replay_from`; T2.5 last so it styles finished views), Tier 3 riding its touchpoints. The implementation plan (next artifact) fixes exact task boundaries, briefs, and the per-task evidence demands per the proven SDD protocol.
