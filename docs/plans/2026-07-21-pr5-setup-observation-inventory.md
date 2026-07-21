# PR #5 §1.3 — Setup-observation inventory (the "physical fact the engine derives, not observes" class)

**Status:** DELIVERABLE — the §1.3 sweep result. Written 2026-07-21 on branch `pr5-pre-phase3-hardening`
(stacked on PR #4 `d6b34cf`). Spec: `docs/plans/2026-07-21-pr5-pre-phase3-hardening-spec.md` §1.

**Method.** A four-angle adversarial sweep (seed-inward, observe-inward, rulebook-outside-in,
state-field-exhaustive) plus an adjudicator, cross-checked against the primary rulebook
(`analysis-1/00-yokohama-rulebook.pdf`) and verified at source by the orchestrator on the two points
where the angles disagreed. **Verdict: exactly two gaps, no third (high confidence).**

---

## The GAP test

A setup-time fact is a **gap** iff all three hold:
1. **Physically variable** — differs table-to-table (not a rules constant).
2. **Visible at setup** — face-up / known at game start; NOT legitimately face-down (a face-down deck's
   order is hidden and reconciled *during play*, so it is not a setup gap — the deck-order trap).
3. **Not captured** — no `SetupObservations` field carries it and the live wizard never asks.

Dispositions: **observe** (add observation + reconcile + wizard prompt), **instruct** (wizard tells the
operator to set it; cheaper, right when the value can't vary), **accept-and-document**.

---

## Dispositioned table — every setup-time fact

| Setup fact | Source | Observed by wizard? | Disposition |
|---|---|---|---|
| player_count (2/3/4) | config / choice | yes (`ask_player_count`) | n/a |
| Player colors, seat | config / choice | yes (`ask_colors`) | n/a |
| Start player (+ 3/4-Yen start) | config / choice | yes (`ask_start_player`) | n/a |
| Board tile arrangement (incl. Canal cell) | seed, overwritten | yes (`ask_tiles`→`ObservedBoard.tiles`) | n/a |
| Building-site card per area | seed, overwritten | yes (`ask_sites`→`ObservedBoard.sites`) | n/a |
| **5-Power token effect per area** | **seed only** | **NO** | **OBSERVE (NEW-1)** |
| Achievements (1 per A/B/C) | seed, overwritten | yes (`ask_achievements`) | n/a (wizard under-validates category, but cannot diverge a correct table) |
| Port A/B fills (6 / 12) | derived, overwritten | yes (`ask_orders`) | n/a |
| Lab A/B fills (6 / 12) | derived, overwritten | yes (`ask_techs`) | n/a |
| Own-seat starting deal (2) + keep | seed, own seat observed | yes (`ask_live_deal`→`deals`) | n/a |
| Opponents' secretly-kept order | seed | NO — **legitimately hidden** ("in secret", p.4 s12) | n/a |
| Order/tech deck internal order | seed | NO — **legitimately hidden** (face-down); reconciled in play (`arrange_*_on_top`) | n/a |
| Boxed order cards (2P:16/3P:5) + deck/removed split | seed | NO — **legitimately hidden** (boxed unseen); conserved-by-exchange on reveal | n/a |
| Which of two identical dup tiles removed (2P/3P) | seed | n/a — **immaterial** (same AreaType, no physical correlate) | n/a |
| **2P dummy pawns (church f7 / customs s4)** | **rules-fixed** | **NO** | **INSTRUCT (NEW-2)** |
| Per-player start resources (money 3/4, 1 ea good, 8/12 assistants, 2/6 shophouse, 0/4 TH, president in hand, 0 VP) | rules-fixed | n/a — fixed constants | n/a |
| Church/Customs tracks (empty apart from 2P dummy) | rules-fixed | n/a | n/a (dummy = NEW-2) |
| Foreign Mercenary supply (3/2/2/2/1, all face-up) | rules-fixed (full pool, not a subset) | n/a | n/a |
| Station token (unplaced at setup) | rules-fixed | n/a | n/a |
| Adjacency graph / canal_crossings | derived from observed tiles | n/a (function of layout) | n/a |
| has_port_b / has_lab_b | derived from player_count | n/a | n/a |
| setup_seed | RNG artifact | n/a (produces the fictions reconciliation overwrites) | n/a |
| Management-board positions | derived (follow tiles) | n/a | n/a |
| Turn-level/end-game init fields (turn_phase, mercenary_*, president_start_area, pending_choices, end_game_*, …) | rules-fixed / turn-level | n/a | n/a |

---

## Gap 1 — NEW-1: 5-Power token per-area assignment (disposition: **OBSERVE**)

**Passes all three prongs.**
1. *Variable:* `new_game` (setup.rs:88-92) shuffles the full canonical 20-token pool
   (`rules.five_power_tokens` = `canonical_five_power_tokens()`, len 20) and assigns a random N-of-20 per
   non-Canal area (`generate_board_layout` board_gen.rs:274-305). Both the subset and the arrangement vary.
2. *Visible:* **rulebook p.3, Game Setup step 5 (verbatim): "Shuffle all the 5-Power tokens face-down and
   place 1 FACE-UP on each Area Board on the 5-space… The remaining… returned to the box."** Face-up,
   verbatim-parallel to the face-up building-site step 4. Web-confirmed: a token flips face-down only
   *after* a player claims it (the B4 step: take → gain effect → flip face-down and keep).
3. *Not captured:* `SetupObservations` has no token field (setup.rs:262-271); `reconcile_board` **relays
   the seeded values** over the observed non-Canal ids in ascending order (setup.rs:541-565);
   `live_observations` omits tokens (wizard.rs:394-408); the board render shows only a boolean `*`
   (render.rs:1260), never the face.

**Important nuance (corrects the spec's framing).** The **live award is already correct**: at a 5-Power
claim, the live CLI holds `Pending::TokenReveal` (repl.rs:1432), asks the operator "what was it?", and
grants the bonus from the operator's letter (`reveal_five_power_token` swaps the observed effect in;
reveal.rs:127-140). The residual harm is **display/preview fidelity**, not a corrupted award: the board
shows `*` not the face, the visible arrangement can't be entered at setup, and the take-*decision* can be
previewed off the seeded guess. ⟹ guidingpath's *"treat 5-Power awards as unreliable"* caveat (line 72) is
**false** and must be removed.

**Fix (Jeremy: full model correction).** Observe token faces at setup (new observation field +
`reconcile_board` support + a wizard section, symmetric with building sites); render the face on the board;
retire the now-redundant claim-time reveal; correct the face-down mislabels (below). SEMANTICS_REV
decision made at implementation (setup-state derivation changes; sim behavior unchanged).

**The face-down mislabel — a transcription error that propagated (fix all of these with NEW-1):**
- `analysis-1/yokohama-kb/01-rules-reference.md:671` "Effects vary (revealed when taken)" — the root error.
- `crates/yokohama-core/src/types/state.rs:443-445` reveal_five_power_token comment "tokens are face-down until taken".
- `analysis-1/yokohama-kb/01-rules-reference-engine-mapped.md:454` (inherits the KB error).
- `analysis-1/yokohama-kb/03-state-schema.md:96` (inherits it).
- `guidingpath.md:72` and `:209` ("5-Power awards unreliable" caveat + carry-forward).

**Verified false alarm:** the `state-fields` angle claimed a distribution bug (2P always 10×ImportedGoods
via a truncate at board_gen.rs:448). The truncate helper `five_power_tokens_for_player_count` (board_gen.rs:440)
has **zero production callers** (only a unit test). `new_game` uses the full 20-pool. No distribution bug.

---

## Gap 2 — NEW-2: 2P dummy pawns (disposition: **INSTRUCT**)

`place_setup_dummies` (setup.rs:190-204) seats `PlayerId::DUMMY` on Church faith-7 and Customs space-4 at
2P **unconditionally** (gated only on player_count==2). Tracks are absent from `SetupObservations`, so
nothing observes them. **But the game-relevant fact — the positions — is rules-fixed, so it fails prong 1
(not physically variable).** The engine models it correctly by construction; the only risk is *one-
directional operator omission* (a table that forgot the physical pawns diverges from a correct engine —
losing two blocked spaces, a track's end count a pawn early, and both majority bonuses). This is a
verification/compliance gap, not an observation gap. PR #4 shipped the telling half
(`dummy_placement_notice`, wizard.rs:1041-1095, 2P+live). Remaining half: an optional cheap "did you place
them?" confirmation. (Rulebook p.9 NOTE — Jeremy's edition.)

---

## Third gap — NONE (confidence: high)

Every rulebook setup step (p.3 steps 1-9, p.4 steps 10-12) was enumerated against `GameState`/
`SetupObservations`. Rejected candidates: deck/tech order + boxed split (legitimately hidden, reconciled in
play); opponents' kept order (legitimately hidden by design); achievement one-per-category (observed;
wizard under-validation can't diverge a correct table); adjacency/canal (derived from observed tiles);
mercenary supply / start resources / Station (rules-fixed constants); management-board positions /
duplicate-tile removal (derived/immaterial); the "multiset-wrong" claim (verified false). No fact beyond
the two above passes all three prongs against a primary source.

---

## Consequence for the rest of §1

- **§1.1** is validated and expands to a **model correction** (open info was modeled as hidden): observe +
  render + retire the reveal + fix the mislabels. Not a gate-game corruption (awards already correct), but
  Jeremy chose to land it before the gate game and to do the full correction.
- **§1.2** is an **instruct/confirm** item, not a variable observation — the shipped telling half is
  adequate; add the cheap confirmation for robustness.
