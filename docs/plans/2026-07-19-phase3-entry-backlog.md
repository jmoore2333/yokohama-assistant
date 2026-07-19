# Phase-3 entry backlog — Codex PR-#1 review, verified triage (2026-07-19)

Codex reviewed PR #1 (choice-encoding) against the bar Jeremy set for the next phase: the rules engine must be authoritative enough to train on (no bad data), and the ASCII CLI must be good enough that playtesting is pleasant. Every load-bearing claim below was independently verified against source by the orchestrator (session 2026-07-19); verdicts are recorded per finding. **This document is the REQUIRED spec input for the next dated plan** (brainstorm → design → plan per repo convention), exactly as `2026-07-18-phase2-entry-backlog.md` was for choice encoding.

Jeremy's dispositions (2026-07-19): PR #1 merged as-is (no finding is a regression introduced by the branch); clippy adopted as a gate (findings fixed on the branch pre-merge); rustfmt deliberately DEFERRED (1,049-diff-block whole-repo reformat — revisit at a phase boundary); AGENTS.md's four stale rules corrected (Jeremy-authorized edit of his untracked file).

## Tier 1 — engine authority (protects training data; VERIFIED findings)

1. **Checked public transition boundary.** `apply_move` validates per-arm rules but not phase (exactly one `turn_phase` check exists — the recovery gate) or membership in `get_legal_moves`; `replay_from` → `Session::load` → `record_stats` call it directly, bypassing the Session legal-list gate. A tampered or bot-constructed record can apply out-of-phase moves, unvalidated movement paths (no connectivity/assistant-presence check in `apply_movement` — crafted-path teleport looks real), and area IDs the invariant checker never bounds-checks against the board. Shape (per Codex, consistent with the T7/I1 apply-layer-guard precedent): make the raw transition explicitly internal/unchecked; introduce ONE checked public transition used by replay, loads, and Phase-3 callers; `Session` keeps its prechecked fast path. Includes: board-membership invariants for all stored AreaIds; a STRICT replay mode for training (reject, don't warn). NOTE: guidingpath's §5.3-7 row justification ("replay-through-apply_move validates untrusted records more strongly than a phase check") is WRONG for the load path — correct the row when this lands.
2. **Station token placement on acquisition.** Documented v1 limitation (`cards.rs`, design §5.7 out-of-scope) — but rulebook-mandated, so a live game diverges the moment Station is bought, and the tech is dead in self-play/training. Encode as a queued placement choice on acquisition (fits the pending-choice architecture directly). Until it lands, "phase gate unblocked" is unblocked-modulo-Station.
3. **Route enumeration by outcome, not just cost.** The I3 Dijkstra keeps one min-total-cost path per destination; equal-cost paths with DIFFERENT credit vectors (1¥ canal toll to bank vs. 1¥ fee to Blue; or fees to different opponents) are strategically distinct actions collapsed to the lexicographic winner. Enumerate one move per unique (destination, mover cost, recipient-credit vector). Extends the 2026-07-19 I3 decision-log row (which covers only deliberately-pricier routes).
4. **Head-of-queue normalization.** `push_choice` singleton-resolves at push time only; a queued choice that BECOMES singleton (two WarehouseTakes, one assistant: the second offers only Decline) still prompts — a wasted prompt/search node contradicting the design's intent, and a slight metrics distortion. Normalize after each resolution (auto-resolve consecutive singleton-Decline heads; the T12 delta-window argument already covers this class as no-ops).
5. **Winner/tie-break API.** `calculate_final_scores` returns seat-ordered totals; the official tie-break (KB 01-rules-reference.md:556 — "Highest VP wins. Tiebreaker: Closest to Start Player (clockwise)") is unimplemented, and the CLI announces no winner. Phase-3 search needs an authoritative terminal utility incl. ties; the CLI should announce winner + breakdown.
6. **Record integrity for training.** `engine_version` is always CARGO_PKG_VERSION (0.1.0) and replay warns-and-continues on mismatch. Add a rules/data hash (registries + board derivation) to `GameRecord` and a strict-replay mode for training pipelines.

## Tier 2 — ASCII/CLI depth (the "do ASCII well" goal; VERIFIED gaps)

7. **Consolidated status view**: `players_panel` omits assistants (hand/board/warehouse), Shophouse/Trading-House tokens (hand/warehouse) — the core economy is invisible. One rich status panel.
8. **Rewards where decisions happen**: building options say "space N" with no printed reward (`describe.rs:268`); board cells show ownership but not site rewards; the guide's `S03 [1vp 2vp+1yen - 2tea] TH 8vp` site line describes a format NO view renders (site_compact_line was never wired — phase-2 T9 note). Movement options omit yen cost + recipient; area-action prompts omit current power/production preview.
9. **Achievements view**: no command shows the three active achievements + claim state.
10. **log_tail historical fidelity**: history rendered with CURRENT state (phase-2 T11 riding minor) — state-dependent descriptions (TH take price) can be wrong for old events.

## Tier 3 — hygiene and rulings

11. Stale cli-guide lines (both md + html): the "Known display gap" box for the 2p Church faith-7 dummy (T12 FIXED it — tracks_view renders X; the box says it renders open) and the unrendered site-line documentation (align with whatever Tier-2 item 8 ships).
12. Delete `TechEffect::base_industry_cost` (zero callers, "best estimates" values differing from the verified registry — the "misleading, not just dead" class).
13. Open rulings to close before calling the ruleset frozen: I5 place-0 (pending Jeremy since 2026-07-18); the from-hand Tram question (design out-of-scope note); Language School Yen vs. WarehouseTake reward ordering (needs a focused ruling + test).
14. rustfmt adoption decision (deferred 2026-07-19; one-time whole-repo reformat commit).

## Verified-accurate review claims already handled

- Clippy: 11 findings fixed on the branch pre-merge (2026-07-19); `cargo clippy --workspace --all-targets -- -D warnings` is now a gate.
- AGENTS.md stale rules: corrected in place (Jeremy-authorized).
- The tampered-record ReturnPresidentToHand demonstration: that is the I2 regression test's own fixture — accurate, and exactly the class Tier-1 item 1 closes generally.
