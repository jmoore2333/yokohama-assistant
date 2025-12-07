# A1 Notes and Feedback

Notes after reviewing the A1 foundation and implementation plans, with cross-checks against the yokohama-kb drafts.

## Foundation (types/state) feedback
- Player/area IDs: `assert!`-based constructors will panic on invalid input; consider returning `Result` (or `TryFrom`) for API boundaries to avoid panics in CV-ingested data.
- GameConfig: encode player-count–dependent setup (areas removed, order deck trims, track thresholds) directly in a helper to reduce duplication across setup code.
- Area/board: BuildingSite VP values and five-power token assignments vary per randomized board; surface a small `BoardLayoutBuilder` to keep initialization consistent and validated (adjacency, canal crossings, site rewards, tokens present).
- Power cap: Power currently capped at 5 in several helpers; ensure cap is applied uniformly and that effects which modify power (Station, techs) happen before the cap.
- PlayerState starts each trade good at 1; confirm against rulebook (base game starting goods are 1/1/1/1) and against removal profiles for 2–3p.
- Hidden information: Starting Order card visibility is modeled, but types should clearly carry the “visible_to” semantics for any future hidden-info variants.
- Tracks: Arrival-order tie-breakers are present; ensure the indexing aligns with Faith/Import values per kb (Church 4–11, Customs 1–7+).

## Implementation (logic/engine) feedback
- Many TODOs/shortcuts remain: tech lookups (MiningTech, Gaslight/Electrical, Ball/Language School, Station teleport, Stagecoach/Postal), Port/Lab payments, Telephone recovery, fifth-tech surcharge, Power-5 Port VP, etc. Highlight and schedule them in early A1-Implementation.
- Movement: `is_possible_action_area` currently blocks when an opponent president is present even if Postal System/Station should allow; movement costs ignore Stagecoach; path validation doesn’t ensure assistants along the whole path (only destination check). Needs full assistant-chain + tech-aware traversal.
- Placement: Legal move generation explodes combinations and ignores Newspaper/Telegram and costs for entering opponent-president areas. Add pruning and tech-aware placement limits.
- Area actions: Commercial/Port/Lab/Chinatown/Track logic is partial—no payments/validations, no Laboratory industry-cost application, no Port slot occupancy checks, no Employment Agency inventory limits.
- Five-power: Movement bonus token is unhandled; ensure removal from board and effect application are consistent and idempotent.
- Building: Shophouse/Trading House builds skip yen costs (Trading House cost logic exists) and don’t handle Gaslight/Electrical bonuses or tech-triggered effects.
- Achievements: Validation/claiming is present, but not yet integrated into apply_move legal/execute flow.
- End-game triggers: Order-deck-empty trigger requires “deck empty AND a Port slot empty”; make sure refills/empty-slot detection occur after card removals.
- Scoring: Country-set scoring currently greedily consumes icons and may over-credit duplicates; confirm against rulebook (distinct countries per set, max one per country per set). Tech majority uses industry sum but ignores zero-card edge cases/ties beyond distance tiebreaker—ensure deterministic ordering.
- Performance: `GameState` clones often in apply_move; consider arena/immutable deltas for MCTS throughput once correctness is locked.

## KB alignment / rule clarifications
- Power calculation order: President + assistants + buildings + Station + tech modifiers, then cap at 5; verify vs kb tables before coding.
- Port: Hand limit 3; second-card cost (2 yen or 1 imported good); Port B only in 3–4p; Port Power 5 gives +3 VP—needs implementation.
- Laboratory: Position surcharges (+1/+2) and fifth-tech penalty (2 yen or 1 imported good) must be enforced; industry value from power (max 5) + 0.5/yen + 1/imported good.
- Church: Power 2 yields 1 Faith (special case); one assistant from hand is required; Winery bonus VP when placing; track triggers vary by player count.
- Customs: Max imported goods to pay is power-limited; one assistant from hand required; Brickyard bonus VP when placing.
- Employment Agency: Token pickup limits scale with power; warehouse/hand counts must be enforced.
- Mercenaries: Usage once per turn; power excludes president; sub-turn returns to Additional Action; supply per country fixed.
- Recover/Telephone: Recovery should allow leaving one assistant if Telephone owned.
- Final round: Trigger when any of the five conditions met; finish current round, then one final round; game ends when player before start player finishes in TurnEnd.

## Suggested next steps
- Promote all TODOs to explicit tasks in A1-Implementation with acceptance criteria per rule reference.
- Add unit tests for: movement with Stagecoach/Postal/Station; Laboratory cost/position; Port hand-limit and second-card payment; Church/Customs faith/imported calculations; Gaslight/Electrical VP bonuses; Telephone recovery; Port Power-5 VP.
- Tighten data constructors (builders/validators) to prevent invalid setup states, especially adjacency, site rewards, and token placement.

