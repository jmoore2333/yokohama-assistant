# Phase B — the owner-games canon batch (four items for ruling)

**Status: RULED (2026-08-18, owner rulings inline on all four items).** C-A ratified (the
swing arithmetic holds at 2–4P, cleanest at 2P); C-B ratified toward the DISCOUNTED direction
(the owner confirmed the orchestrator's reading in-session the same day: flips are priced with
recoverability/opportunity-cost/phase awareness, and the full-swing step stops being the
reference behavior in family-3 controls); C-C opens the tech-ability representation lane in
principle (class-weighting acceptable; any admission still walks its own evidence ladder and
the Phase-C STOP); C-D rules the fixed lab-rush opening NOT acceptable (opening diversity
becomes a probe/family target). The guidingpath 2026-08-18 canon row is the decision-log
record; this file stays the verbatim rulings text.

**Written 2026-08-17, assembled from the feedback package's §3 and the scribe log's
candidate-canon section (owner statements verbatim where quoted). Format and force follow the
2026-08-09 batch: each ruling is a judgment PRIOR for narrowly-constructed 2P scenario
positions, never context-free strategy — the owner's standing caveat ("heavily dependent on the
game and board state") rides every item. Preference findings enter layer 2 ONLY through these
rulings (amendment D2); rules-derivable material needs no ceremony and is deliberately absent
here. Rule each item inline — a word is enough; amend anything in place.**

Evidence anchors below are the D7 triage's verified figures
(`docs/plans/2026-08-17-phase-b-d7-triage.md`), not the raw session notes.

---

## C-A — Majority participation value

**Proposed canon (owner statement, ruling-shaped):** a majority bonus ≈ one good turn of
value, AND participation itself has value beyond winning the bonus — at 2P, not taking a
church space means the opponent gets 6 and you forgo 3, a doubled swing "in a binary game";
"it's not just proximity — it's not just winning the bonus, it's playing the dummy and getting
some action"; the majorities force play across the whole board by design.

**Evidence:** customs exhibit owner 8 / bot 0 ("because it didn't take any time there", game
1); the bot's zero customs presence in both games while its church line banked real value.
Corpus rider: under V2 play, `customs_majority_margin` newly separates from the shipped
banked-bonus mirror late (+0.042 point AUC) — race POSITION carries information the awarded
bonus alone does not.

**Downstream if ratified:** family-3 fixture pass criteria may price participation/race
position, not only bonus-won arithmetic; a participation-shaped term joins the V3 candidate
list on its own evidence ladder. Nothing ships from this ruling alone.

**Questions:** (1) Ratify as stated? (2) Is the doubled-swing arithmetic 2P-specific, or does
participation value survive at 3–4P where a space taken denies only one of several rivals?

**Owner ruling:** Ratified. The swing arithmetic holds throughout the 2-3-4p sets, but it's much cleaner to derive in a 2p as it's a true binary choice. 

---

## C-B — Recoverable majority flips: full swing or discounted?

**The question (drafted in-game, both games exercised it):** when a majority flips on a public
move but the deficit is recoverable at known cost, is the honest value of the flip the FULL
bonus swing (the shipped step behavior), or a recoverability/proximity-discounted value? The
owner's live read at the game-1 Newspaper flip: "swung all the way to the other extreme — I'm
not sure that's fair, it could just get another card; but it was at least aware" — and the bot
did exactly that recovery four turns later (verified arc: Brickyard second-behind-the-dummy 5
→ Newspaper takes it 0 → Gaslight retakes 5 → Stagecoach passes the dummy into first 10).

**Evidence:** the verified g1 arc (triage §2-F6); round 3's finding that a plain
`tech_majority_margin` column DUPLICATES the shipped step (re-confirmed under V2 play this
session — the margin adds nothing), so the owner's corner requires a DIFFERENT arithmetic
(nearness/cost-to-flip discounting), which no corpus has measured.

**Downstream if ratified toward discounting:** defines the target arithmetic for a V3
candidate distinct from the measured-duplicate margin; family-3 controls stop treating the
full-swing step as the reference behavior. If ratified toward full-swing: the step is canon
and the owner's e89 discomfort is recorded as an accepted model simplification.

**Owner ruling:** Ratified, it's important to weigh the alternative options to getting points, and any action taken necessairly decreases the chances of other moves being able to earn points, so especially being aware of the phases of the game, even in the loose sense, can help optimize for that. This is also influenced by what's available on the tech track, it's potential value to me or even the other player, possible achievements, etc. 

---

## C-C — Tech abilities: prospective value or majority chips?

**The question (F3's root cause, owner hypothesis confirmed against the implementation):**
should technology abilities carry prospective value in the evaluator, or is majority-chip
pricing acceptable at this model class? No ability exists as a feature today; every purchase
prices as majority arithmetic (verified: Brickyard's customs rider at 0/0 in its own table;
the Ball-vs-Patent-System margin-0 coin flip at g3 e3; Gaslight's shophouse rider invisible
and the pipeline never built; Stagecoach's ability ignored).

**The owner's catalog (preserve with the ruling — "not a clean mapping all the time … it
depends on the strategy"):** Exposition = flat 10 VP, usually take, but NOT at the big
premium/wasted imports ("marginal gain that costs multiple turns"); University = 1-yen
warehouse assistant (saves EA trips); Winery = +3 VP per church-pawn action (the bot's three
church visits would have been 9 VP had it been out and taken); System of Stock = liquidity
floor (refill to 2 yen if ≤1 at turn start — "guaranteed liquidity for gears/orders/church,
not for big purchases"); abilities compound with achievements (bonus-VP-per-tech + a 4-techs
achievement "could be one of those 22-point swings"; the 5+-orders achievement made
order-aligned techs matter — both players took one).

**Downstream:** the largest representation question on the board. Any admission is
`Feature::ALL` blast radius, needs its own evidence ladder, and this ruling gates whether that
lane opens at all. A "majority chips are acceptable for now" ruling is a fully respectable
outcome and simply parks the lane with the reason recorded.

**Questions:** (1) Should ability value enter the model, in principle? (2) If yes, is
per-ability hand-modeling acceptable, or only a cheaper aggregate proxy (e.g.
ability-class-weighted tech count)? (3) Does the answer change for the OPPONENT's abilities
(the Language School yen rider priced as a threat — the F2 lane)?

**Owner ruling:** Ability value should enter the model, in principal. Without it, the tech card draws just for the benefit of majority bonuses is not a good strategy, even if the player lucks into something useful. I think that we can likely class-weight them, or associate with other horizon scanning like the examples given above, it would be complex now to lay out evidence for each of the movement cards and some of the placement cards can dramatically change the movement of pawns which may make it harder to build in other sections of the game. The opponent's abilities should be made aware, but it's again only useful with proper horizon planning to be aware of the opponents likely moves and give weighting to those.

---

## C-D — The opening: fixed lab-rush or board-read?

**The observation (owner data + statement):** across five games (two diagnostic + three
casual), the bot opened with the Laboratory rush five out of five times, four different cards
(verified structurally in both on-disk games: first turn places into Laboratory A, places the
President there, buys a tech; the casual-game evidence is banked testimony — the saves were
discarded at owner instruction). Game-4 banked data: the lab line won the opening placement by
−40/−42 over Customs+Church alternatives — real margins, not a tie. Owner: "maybe it just
thinks that's always the right idea — I do it often too — but 100% of the time may not be
right." Mechanism: the opening cannot read achievements or card synergies, so the one
feature-visible chip always wins.

**The question:** is a fixed lab-rush opening acceptable 2P policy for this model class, or
must an opening read the board (achievements out, order mix, synergy potential) such that 100%
concentration is itself a deficiency?

**Downstream:** if board-read is required, opening diversity becomes a driven family /probe
target (cheap to measure: first-turn move distribution over a corpus) and turn-1
achievement/synergy visibility joins the representation queue behind C-C. If acceptable, the
100% observation is recorded as consistent-with-canon and the probe is not built.

**Owner ruling:** A fixed lab-rush opening is not acceptable, and it only works if you're the first player or the other player hasn't done it themselves. It is just as valid depending on board layout to optimize for an order in hand, or some other achievement - those are generally the factors that would be good opening moves in a 2p game. That said, it could be a good idea a large number of times, but it does depend on the lab cards.

---

## Held out of this batch deliberately

- The owner's closing verdict (a coin flip to the engine IS a deficiency; playing to win =
  opponent awareness + multi-turn stacking) — already recorded as the bar-distance summary in
  guidingpath's 2026-08-17 row; it frames D7 routing and needs no ruling.
- F6's tech-majority tiering — an engine/eval FACT, verified in source, not a preference.
- The D2 cadence (one owner game per MAJOR merge) — already ruled 2026-08-17.
