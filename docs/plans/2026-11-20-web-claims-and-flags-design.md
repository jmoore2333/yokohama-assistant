# Design — the claim on the card and the flag tally (issues #75, #76) (2026-11-20)

**Status: DRAFT, not yet accepted.** Nothing is built from this until the owner accepts it; the plan is written after.

**What this is.** The first of two cycles answering feedback from play on the browser client. This one makes two things readable that a table shows at a glance: who has claimed which achievement (#75) and how many flags of each country a seat has (#76). The second cycle, the area-action choosers (#77, widened), has its own spec and plan, written against the tree this one merges (§11). Branch `web-evaluator-feedback`, cut from master `e600f5f4`, on the lab.

## 1. Intent

People playing on the browser client report that a claim does not look like a claim and that there is no way to follow one's flags. Both are things the physical game answers with pieces and face-up cards, and both are already in the model or one read away from it: neither needs a rule the engine does not have.

## 2. Goals, non-goals, anti-goals

**Goals.**
- G1. A claimed achievement shows each claimant's assistant on the card, in claim order, the first claim told apart from the later ones; the card is lit in the turn walk when the claim lands.
- G2. Every seat shows its five per-country counts of face-up flags (fulfilled orders and technologies), the count the engine awards mercenaries and scores the country bonus from.
- G3. The viewer's own seat also shows the flags of the orders in hand, the starting order included, apart from the counted ones and marked as not yet counting.

**Non-goals** (each is an additive field or a later cycle, none is closed off): how far a country is from its next mercenary; the country bonus as it stands; the terminal client's players panel (cycle 2 moves the terminal's text once, on purpose); any chooser (#77).

**Anti-goals.** A count worked out in `web/`. A per-country count of a hidden hand on any payload: at the opening a hand is the starting order alone, so the count would name that card's country. A change that moves `SEMANTICS_REV`. A layout that depends on what a card's picture shows.

## 3. What binds

- **The layer computes nothing about the game** (`docs/web-lane.md` §4): the counts are the model's, built in `screen_model` from the engine's own counting, and the client draws them.
- **The mask is a property of the model** (`HandPanel`'s doc, `crates/yokohama-cli/src/screen_model.rs`): `cards` is empty whenever `hidden`. The in-hand count follows the same rule and lives on `HandPanel` only, which a spectator's frame drops whole; `PlayerModel` reaches every seat and every spectator and carries the face-up count alone.
- **`SEMANTICS_REV` stays 7.** A read-only count changes no legality, application, scoring or setup; the one engine edit (§4.1) is behaviour-preserving, and §8 names what shows it.
- **The wire corpus is a contract**: regenerated once, and the diff read (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`).
- **The terminal's rendering does not move**: the CLI's capture goldens and `guide_sync` stay green unedited, which is how that is known.
- **No step depends on reading a picture.** The claim's place on the card is fixed by the card's box. Layout cells run with no art (`art: null`), and the look with the art in place is the owner's reading at the gate.

## 4. Approaches

### 4.1 Where the count comes from (#76)

The engine counts flags in two places, each with its own inline loop over `completed_orders` and `technologies`: `check_mercenary_eligibility` (`game/cards.rs`) and `calculate_country_bonus` (`game/scoring.rs`). Neither is reachable from `screen_model`.

| | (a) One function, three callers | (b) A new function beside the two loops | (c) Counted in `screen_model` |
|---|---|---|---|
| What moves | A public count in core; the two engine sites call it; the model calls it | A public count in core; the engine sites untouched | Nothing in core |
| "The model's count is the engine's" | By construction | By cells that compare outcomes | By nothing: a third statement in another crate |
| Engine files edited | Two functions, their loops replaced by one call | None | None |

**Picked: (a).** It is the only one where the acceptance sentence "each seat's five counts equal the engine's" is true by construction, and it leaves one statement of the count where there were two. Its cost is an edit inside two engine functions; the existing cells over both (`calculate_country_bonus_covers_the_table`, the country-bonus set cells, the two mercenary-award cells in `apply_move.rs`) stay green unedited, and the release roster's fuzz and differential gates read the engine against the oracle before the merge. (b) keeps the engine byte-identical at the price of a third loop; (c) is what §3's first rule forbids in spirit, one crate over. The evaluator's own count in `game/eval.rs` reads a `PlayerView` and belongs to a frozen identity: it is not touched.

### 4.2 Where the in-hand count rides (#76)

On `HandPanel`, as `Option`: `None` whenever `hidden`. Rejected: a masked field on `PlayerModel`, which every viewer receives, where one wrong `hidden` would leak; and a count the client adds up from `HandPanel.cards`, which is the layer computing.

### 4.3 How the claim is drawn (#75)

**Picked: a band on the card's picture box**, anchored to the box and not to the picture. Rejected: cubes aligned to the printed first-place value, which needs per-picture coordinates for twelve pictures; and a cube added to the caption's chips, since the feedback is that the caption makes a claim read as one more number.

## 5. The design

### 5.1 The claim on the card (#75) — client only

- `AchievementCardView` (`web/src/Cards.tsx`) draws a claim band along the bottom of the picture box, over the picture or over the blank a card without art shows. One cube per entry of `model.claimed_by`, in that order, the board's own `CubeIcon` in the seat's colour.
- The first claim stands in a ringed place at the band's leading edge beside `first_place_vp`; the later claims follow after a divider beside `subsequent_vp`. Both numbers are the model's. No claim, no band.
- The words stay: each cube carries its claim as accessible name and title, as the board's pieces do, in the form `Red claimed first — 6 VP` and `Blue claimed — 3 VP`.
- The caption keeps the VP chip, the `unclaimed` chip and the progress rows. The `first Red` / `also Blue` chips retire: the band says it.
- **Three call sites, not the two the issue names**: the Goals strip, the dashboard's Achievements panel and the goals index (`web/src/Index.tsx`). One component covers all three.
- **The walk.** `Changes` (`web/src/diffFrames.ts`) gains `goals`, the achievement types whose `claimed_by` is longer in `b` than in `a`, read from the two frames' Achievements panels and nothing else. It joins `any`, and `heldChanges` (`web/src/walk.ts`) unions it with the rest. The Goals strip and the dashboard panel light a card in that set with the treatment an offer slot takes. The index is not on the table during a walk and lights nothing.
- Four cubes and both numbers fit on one line at the Goals strip's narrowest card width.

### 5.2 The flag tally (#76) — core, model, wire, client

- **Core.** One public count of a seat's face-up flags, five numbers in `Country::ALL` order, in the shape `MercenarySupply` has (one named field per country); and the read it is built from, the countries of a list of order ids, public too, so the hand's count is the same read. `check_mercenary_eligibility` and `calculate_country_bonus` take their counts from it.
- **Model** (`crates/yokohama-cli/src/screen_model.rs`). `PlayerModel` gains the face-up count, on every seat, never masked: the cards are face up. `HandPanel` gains the in-hand count as an `Option`: the count over `order_cards_in_hand`, which holds the kept starting order, and `None` whenever `hidden`.
- **Wire.** Both fields ride every frame-bearing payload. `web/src/types.ts` declares them and `web/src/wire/contract.ts`'s key sets name them. The regenerated corpus's diff is those two keys wherever a seat or a hand is, and nothing else.
- **Client** (`web/src/Dashboard.tsx`). Each seat's row in the players panel gains five cells in the engine's country order, each the country's mark and its count, a zero drawn dim. On the row of the seat whose hand the frame carries, and only when that hand's count is present, each country with flags in hand shows `+n` beside its count, styled as not counting and titled `n in hand — counts when the order is fulfilled`.
- `diffFrames` gains no key for this: a seat's row already lights when `orders_filled` or `technologies` moves, and the hand's own set covers an arrival.

## 6. The cells

**Core.** The count over a seat with known cards reads literal values per country. Over self-play at 2P, 3P and 4P, at every position and seat, the five counts sum to the seat's fulfilled orders plus its technologies. The existing engine cells over the two call sites are unedited and green.

**Model.** The face-up count is present for a seat whose hand is hidden. It does not count an order in hand (a position whose only flag of one country is in hand reads zero for it). The in-hand count includes the starting order. A hidden hand's count is `None`.

**Wire.** `PlayerModel`'s key set has the face-up count and no in-hand field; `HandPanel`'s has the in-hand count. A frame built for another seat and a spectator's frame carry no in-hand count of the masked hand, pinned by key set and, at the opening position, by value: every flag count in the frame is zero but the viewer's own hand's.

**Client, #76.** A seat with no flags, with flags of some countries, and the viewer's row with flags in hand, each from a corpus payload; a spectator's payload draws no `+n` anywhere.

**Client, #75.** A card with no claim, with one claim and with four (one per seat at 4P), in the strip, the panel and the index, with art and without: the cubes in claim order, the first in the ringed place, the words as §5.1 gives them. A walk across a claim lights that card and no other. The corpus is a two-player walk whose cards carry at most one claim, so the four-claim card is one new corpus file built in process, as `why_withheld.json` is; a payload is never hand-authored.

**Mutants the cells must catch.** The in-hand count present on a hidden hand. The face-up count reading the hand. The starting order left out. `goals` reading "differs" instead of "grew". The cubes in seat order instead of claim order. The first claim's place given to the lowest seat.

## 7. The documents

`web/src/Help.tsx` and `docs/web-guide.md` where a sentence describes the goals or a seat's row. `docs/web-lane.md`: §2's corpus count and any list of the model's fields. `docs/engine.md` if it lists the engine's public reads. At the close: a decision-log row in `guidingpath.md` and the board. The CLI guide pair does not move.

## 8. The suites and the gates

Core changes, so every per-edit suite runs (`CLAUDE.md`'s Commands) with the lint gate, and the web lane's suites with them (`docs/web-lane.md` §2), the browser walk included since the client changes what a browser shows. The new corpus file moves the two exact-set cells that list the files no script names (the node replay's and the Worker pool's). Before the merge: the fifteen release gates as one sequence; its fuzz and differential gates are what read §4.1's edit.

## 9. Left to the builder

Names of the new type, functions and fields. The band's exact dimensions and the lit treatment's class. The shape of the in-process four-claim fixture. Whether the country's mark is the mercenary token's art with the country's word as its title, or the word alone where there is no art.

## 10. The wrap

Tasks, the whole-branch review and its fix wave on the lab; then the branch reaches origin once, as its pull request. The release number and the deploys are the wrap's, by `docs/web-lane.md` §7, and not a task of the plan.

## 11. What follows: the choosers (#77, widened)

Decided in this cycle's design conversation, recorded here so the next spec starts from it:
- The Church's wall of rows is one case of a wider one. Measured at `e600f5f4`, rows per area-action list at the moment a player reads it: the Church up to 64 (seven saved games and 900 random games); Chinatown, on the 3–4P board only, a median of 31 and up to 1,237 over 600 random games, one decision in three above 40 rows; the Port up to 37; the Laboratory up to 25; the Employment Agency up to 31, 99% at 12 or fewer.
- The answer is a chooser by ingredients, drawn with the goods and yen tokens in the browser, and it is chosen in the REPL, as Placement Option A is, so the terminal gains it too: the Port's orders, the Church and Chinatown are named for the terminal as well as the browser.
- Its spec and plan are written against the tree this cycle merges.
