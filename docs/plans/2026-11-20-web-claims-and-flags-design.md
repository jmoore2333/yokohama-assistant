# Design — the claim on the card and the flag tally (issues #75, #76) (2026-11-20)

**Status: ACCEPTED by the owner, 2026-11-20**, as revised the same day after an independent review of the first draft (§12), and with the pictures made a task of the plan (§7). The plan is `docs/plans/2026-11-20-web-claims-and-flags-plan.md`.

**What this is.** The first of two cycles answering feedback from play on the browser client. This one makes two things readable that a table shows at a glance: who has claimed which achievement (#75) and how many flags of each country a seat has (#76). The second cycle, the area-action choosers (#77, widened), has its own spec and plan, written against the tree this one merges (§11). Branch `web-evaluator-feedback`, cut from master `e600f5f4`, on the lab.

## 1. Intent

People playing on the browser client report that a claim does not look like a claim and that there is no way to follow one's flags. Both are things the physical game answers with pieces and face-up cards, and both are already in the model or one read away from it: neither needs a rule the engine does not have, and neither needs the engine to move.

## 2. Goals, non-goals, anti-goals

**Goals.**
- G1. A claimed achievement shows each claimant's assistant on the card, in claim order, the first claim told apart from the later ones; the card is lit in the turn walk when the claim lands.
- G2. Every seat shows its five per-country counts of face-up flags (fulfilled orders and technologies), the count the engine awards mercenaries and scores the country bonus from.
- G3. The viewer's own seat also shows the flags of the orders in hand, the starting order included, apart from the counted ones and marked as not yet counting.

**Non-goals** (each is an additive field or a later cycle, none is closed off): how far a country is from its next mercenary; the country bonus as it stands; the terminal client's players panel, which is owed by the chooser cycle or an issue of its own and is recorded in §11 so it is not lost; any chooser (#77).

**Anti-goals.** A count worked out in `web/`. A per-country count of a hidden hand on any payload: at the opening a hand is the starting order alone, so the count would name that card's country. Any byte of the five fenced crates moved. A layout that depends on what a card's picture shows.

## 3. What binds

- **The lane's fence, every cycle** (`docs/web-lane.md` §1): "the five fenced crates (`yokohama-core`, `yokohama-oracle`, `yokohama-difftest`, `yokohama-bridge`, `gen-data`) byte-identical, the CLI touched only by behaviour-preserving seam changes, `SEMANTICS_REV` 7, no bot behaviour moved". This cycle holds all four clauses. The CLI's change is a seam change: `screen_model` gains a count and two fields, the terminal draws neither, and the capture goldens and `guide_sync` stay green unedited, which is how that is known.
- **The layer computes nothing about the game** (`docs/web-lane.md` §4): "every model is `screen_model`'s". The counts are the model's and the client draws them.
- **The mask is a property of the model** (`HandPanel`'s doc, `crates/yokohama-cli/src/screen_model.rs`): `cards` is empty whenever `hidden`. The in-hand count follows the same rule and lives on `HandPanel` only; `PlayerModel` reaches every seat and every spectator and carries the face-up count alone. Where a hand is not hidden, its count is a function of the countries of that panel's own `cards` and of nothing else, so it carries nothing the existing mask does not already govern.
- **The wire corpus is a contract**: regenerated once, and the diff read (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`).
- **No step depends on a model reading a picture.** The claim's place on the card is fixed by the card's box. A layout cell measures boxes and is read as numbers. The look with the art in place, and any re-cut screenshot, is the owner's reading (§7).

## 4. Approaches

### 4.1 Where the count comes from (#76)

The engine counts flags inline, in private code: `check_mercenary_eligibility` (`game/cards.rs`) on every order fulfilment and Laboratory take, and `calculate_country_bonus` (`game/scoring.rs`) inside `final_result`. The evaluator keeps a third count over a player's view (`game/eval.rs`), and the oracle a fourth, independent by rule. None is reachable from `screen_model` as a count.

| | (a) One public count in core, the engine's two sites edited to call it | (b) A public count in core beside the two loops, with an equivalence cell | (c) Counted in `screen_model`, held by a drift pin |
|---|---|---|---|
| Fenced crates | Core's bytes move, and two engine functions lose their loops | Core's bytes move, additively | **Byte-identical** |
| What shows the model's count is the engine's | Construction, after the edit | An in-crate cell against the untouched loops | A pin over self-play against the engine's public outputs (§6) |
| What shows the engine still behaves | Existing core cells only: the oracle enumerates moves from the engine's own state and so cannot see a wrong award | Nothing to show: no engine function changes | Nothing to show |
| Process it owes | A ruling to cross the fence; a core change is "a scoped, dedicated PR with a clean-eyes review before and after" (the security remediation spec, R-SEC1) | The same | None beyond this cycle's |

**Picked: (c).** Editing the engine is not required to meet #76, and (c) is how this project already answers the same question twice. `screen_model::placement_fee` is a mirror of an engine computation the CLI "may not widen", held by a pin that applies a real placement and compares the money charged. The evaluator's own flag count is held to `final_result` by `check_drift` (`crates/yokohama-core/tests/eval_core.rs`), which runs at every position and whose doc records that no scoring surface was opened for it. The review built (c) on an export: the pin of §6 compared 211,700 seat-positions over 18 games at 2P, 3P and 4P in about four seconds, and caught both mutants planted against it.

(a) was the first draft's pick and is withdrawn: it crosses the fence for tidiness, it puts an edit on the path of every order fulfilment and Laboratory take while V4 is being vetted, and once its loops are deleted nothing in the tree can compare before with after. If one count in core is wanted later, its home is a cycle that opens core for its own reasons, in (b)'s shape: the Language School cycle (issue #62) is the next such.

### 4.2 Where the in-hand count rides (#76)

On `HandPanel`, as `Option`: `None` whenever `hidden`. Rejected: a masked field on `PlayerModel`, which every viewer receives, where one wrong `hidden` would leak; and a count the client adds up from `HandPanel.cards`, which is the layer deciding what counts.

### 4.3 How the claim is drawn (#75)

**Picked: a band on the card's picture box**, anchored to the box and not to the picture. Rejected: cubes aligned to the printed first-place value, which needs per-picture coordinates for twelve pictures; and a cube added to the caption's chips, since the feedback is that the caption makes a claim read as one more number.

## 5. The design

### 5.1 The claim on the card (#75) — client only

- `AchievementCardView` (`web/src/Cards.tsx`) draws a claim band along the bottom of the picture box, over the picture or over the blank a card without art shows. One cube per entry of `model.claimed_by`, in that order, the board's own `CubeIcon` in the seat's colour.
- The first claim stands in a ringed place at the band's leading edge beside `first_place_vp`; the later claims follow after a divider beside `subsequent_vp`. Both numbers are the model's. No claim, no band.
- The words stay, as a tile keeps its pieces' words: the band is one node whose accessible name lists the claims in order (`Red claimed first — 6 VP · Blue claimed — 3 VP`), the cubes themselves hidden from it, and each cube carries its own claim as a title.
- The caption keeps the VP chip, the `unclaimed` chip and the progress rows. The `first Red` / `also Blue` chips retire: the band says it.
- **Three call sites, not the two the issue names**: the Goals strip, the dashboard's Achievements panel and the goals index (`web/src/Index.tsx`). One component covers all three.
- **The walk.** `Changes` (`web/src/diffFrames.ts`) gains `goals`, the achievement types whose `claimed_by` is longer in `b` than in `a`, read from the two frames' Achievements panels and nothing else. It joins `any`, and `heldChanges` (`web/src/walk.ts`) unions it with the rest. The Goals strip and the dashboard panel light a card in that set with the treatment an offer slot takes. The index is not on the table during a walk and lights nothing. The module's own account of what the walk names, and of which sets only grow, gains `goals`.
- Four cubes and both numbers fit on one line at the Goals strip's narrowest card width.

### 5.2 The flag tally (#76) — model, wire, client

- **Model** (`crates/yokohama-cli/src/screen_model.rs`). One count type, five numbers in `Country::ALL` order with one named field per country, the shape `MercenarySupply` has. One function counts a seat's face-up flags, the countries of its `completed_orders` and its `technologies` read from the registries; one counts the countries of a list of order ids. `PlayerModel` gains the face-up count, on every seat, never masked: the cards are face up. `HandPanel` gains the in-hand count as an `Option`: the count over `order_cards_in_hand`, which holds the kept starting order, and `None` whenever `hidden`.
- **Wire.** Both fields ride every frame-bearing payload. `web/src/types.ts` declares them. `web/src/wire/contract.ts` names the face-up count in the seat's key set and gains a key set for the hand panel, which it does not have today, with its run-time check in `contract.test.ts`. The regenerated corpus's diff is those two keys wherever a seat or a hand is, and nothing else: on the review's export, 27 of the 110 files.
- **Client** (`web/src/Dashboard.tsx`). Each seat's row in the players panel gains five cells in the engine's country order, each the country's mark and its count, a zero drawn dim. On the row of the seat whose hand the frame carries, and only when that hand's count is present, each country with flags in hand shows `+n` beside its count, styled as not counting and titled `n in hand — counts when the order is fulfilled`.
- **A server one release back** omits both fields. The client then draws no tally and nothing else changes; it does not fail.
- `diffFrames` gains no key for this: a seat's row already lights when `orders_filled` or `technologies` moves, and the hand's own set covers an arrival.

## 6. The cells

**The drift pin — "each seat's five counts equal the engine's"** (in `screen_model`'s tests). Over seeded self-play at 2P, 3P and 4P, at every position and for every seat, three legs: the five counts sum to the seat's fulfilled orders plus its technologies; for each country, the mercenaries the seat holds, face up and spent, equal half its count rounded down while that country's supply lasts, and are at most that once it is out; and core's own set arithmetic (`eval::set_bonus`) over the five counts equals `final_result`'s country bonus for that seat at that position. The mercenary leg is the one that says WHICH country a flag belongs to: the bonus is symmetric in the countries and cannot. The cell asserts how many rows it compared and how many were non-zero, so an empty sweep fails.

**Model.** Literal counts for a seat with known cards. The face-up count is present for a seat whose hand is hidden. It does not count an order in hand (a position whose only flag of one country is in hand reads zero for it). The in-hand count includes the starting order. `a_masked_hand_carries_no_card_identity_across_the_wire` is extended: the masked panel's JSON carries no in-hand count. This is the only tier a hidden hand's count can be caught at, since the layer sends no hidden hand panel to a browser: a seat's own hand is unmasked and a spectator's is dropped.

**Wire.** The two key sets. In every payload of the corpus, a hand panel's count equals the countries of that same panel's `cards`. No payload's seat carries an in-hand field.

**Client, #76.** A seat with no flags, with flags of some countries, and the viewer's row with flags in hand, each from a corpus payload; a spectator's payload draws no `+n` anywhere; a payload with the fields removed draws no tally.

**Client, #75.** A card with no claim, with one claim and with four, in the strip, the panel and the index, with art and without: the cubes in claim order, the first in the ringed place, the words as §5.1 gives them. The corpus is a two-player walk whose cards carry at most one claim, so the four-claim card is one new corpus file built in process, as `why_withheld.json` is, and **its claim order is fixed here**: a 4P card claimed in the order seats 2, 0, 3, 1, so that neither seat order nor "the lowest seat first" can pass for claim order. The walk cells read frame pairs made as `diffFrames.test.ts` already makes them, a clone of a corpus frame with one field changed, because no committed pair spans a claim: a pair whose `claimed_by` grew lights that card and no other, and a pair whose `claimed_by` shrank or merely changed lights nothing.

**Layout** (the browser walk, `web/e2e/`). Four cubes and both numbers on one line at the Goals strip's narrowest card width, measured as bounding boxes in the manner of `gates.spec.ts`.

**Mutants the cells must catch.** The in-hand count present on a hidden hand. The face-up count reading the hand. The starting order left out. One technology dropped from the count. Two countries swapped. `goals` reading "differs" instead of "grew". The cubes in seat order instead of claim order. The first claim's place given to the lowest seat.

## 7. The documents

- `web/src/Help.tsx` and `docs/web-guide.md` where a sentence describes the goals or a seat's row.
- `docs/web-lane.md`: §2's corpus count (110 becomes 111) and any list of the model's fields.
- **The pictures are re-cut, as a task of the plan** (the owner, at acceptance). The help panel's five (`web/public/help/`, cut by `web/e2e/shots.spec.ts` and `docs/tools/mkhelpart.py`) and the Worker README's ten (`worker/docs/img/`, cut by `readme_shots.spec.ts`), three of which show a game, picture the Goals strip and the players panel as they were. Every run photographs a different game, so each picture's `alt` and caption (`web/src/Help.tsx`, `worker/README.md`) is re-read against the new picture, which is that spec's own rule. No agent opens a picture: the generator writes the page's accessibility snapshot as text beside each one, the alt and caption are checked against that text, and the pictures themselves are the owner's reading before they are committed. The README's committed pictures are the Worker's, so they are cut once the release is live.
- At the close: a decision-log row in `guidingpath.md` and the board. The CLI guide pair and `docs/engine.md` do not move.

## 8. The suites and the gates

Every per-edit suite with the lint gate (`CLAUDE.md`'s Commands), and the web lane's suites (`docs/web-lane.md` §2) since the change reaches `crates/yokohama-cli` and the client, the browser walk included. The new corpus file is listed in four places: the two exact-set lists of files no script names (`NOT_SERVED` in `web/src/local/corpus.node.test.ts`, `THIRD_COLUMN` in `worker/test/corpus.test.ts`), `contract.ts`'s imports and `contract.test.ts`. Before the merge, the fifteen release gates as one sequence, by the two-tier rule; no fenced crate's bytes move, so they are expected to read as at master. Origin's `test.yml` runs only on a core change, so nothing runs there by itself for this branch: `web.yml` is dispatched before the release, as the lane does.

## 9. Left to the builder

Names of the new type, functions and fields. The band's exact dimensions and the lit treatment's class. Whether the country's mark is the mercenary token's art with the country's word as its title, or the word alone where there is no art. The seeds and the game count of the drift pin, sized to stay a per-edit cell.

## 10. The wrap

Tasks, the whole-branch review and its fix wave on the lab; then the branch reaches origin once, as its pull request. The release number and the deploys are the wrap's, by `docs/web-lane.md` §7, and not a task of the plan. Two things a deploy of this change does, as any change under `crates/yokohama-cli/src` does: the Worker's picker code identity moves, so a picker left open on the Worker is dropped at its next restore and reopened by the player; and the deploy guard refuses the build until it is rebuilt.

## 11. What follows: the choosers (#77, widened)

Decided in this cycle's design conversation, recorded here so the next spec starts from it:
- **The measurement #77's acceptance asks for**, at `e600f5f4`, by a throwaway harness outside the tree: `get_legal_moves` at every area-action position with no choice queued, over uniform-random self-play (300 games at each of 2P, 3P and 4P, seeds 1000–1299, through the test helper `run_random_game_observed`) and over seven saved games replayed position by position with `state_at`. Rows per list: the Church up to 64, in both samples; Chinatown, on the 3–4P board only, a median of 31 and up to 1,237, about one decision in three above 40 rows; the Port up to 37; the Laboratory up to 25; the Employment Agency up to 31, 99% at 12 or fewer. The next spec re-measures rather than citing these.
- **The answer is a chooser by ingredients**, drawn with the goods and yen tokens in the browser, and **chosen in the REPL**, as Placement Option A is, so the terminal gains it too. The families named so far: the Church, Chinatown and the Laboratory, and for the terminal as well the buying of orders at the Port. Which families that cycle takes is settled in its own spec.
- **The terminal's flag counts** (§2's non-goal) are owed there or in an issue of their own: that cycle is the one that moves the terminal's text.
- The two cycles run in sequence, never in parallel: both regenerate the wire corpus, and each diff is readable only against the other's merged tree.

## 12. The review of the first draft

An independent review of the first draft (commit `bf3d182c`), asked for by the owner, ran both candidate changes on exports and answered three questions.
- **Is the engine edit required?** No: §4.1, rewritten. The first draft also said the fuzz and differential gates would read that edit; they would not have read its mercenary half, since the oracle enumerates from the engine's own state. A planted mutant in the award left the differential harness's per-edit suite green and was caught only by core's own cells.
- **What reacts to hashes and pins over the touched files?** Under this design: the wire corpus (the two keys, and one new file), the Worker's picker code identity and the deploy guard's source stamp (§10), and the corpus count in `docs/web-lane.md`. Not touched: `rules_hash`, `SEMANTICS_REV`, the terminal digests and `browser_played_pin`, the evaluator's and the lookahead's version constants, the grandfathered identities, the solver's cross-encoding pin, `gen-data`'s output, the frozen blocks and the arena corpus.
- **Does it reach the pickers or the area actions?** No. The engine's two counting sites are reached by order fulfilment, a Laboratory take and final scoring only, and this design edits neither. No legal-move generator, REPL sub-prompt or prompt kind reads the fields the model gains.
- The same review corrected this draft's cells (§6: the fixture's claim order, the frame pairs, the layout cell, the hand panel's missing key set, the tier a hidden hand is caught at) and its documents (§7: the pictures).
