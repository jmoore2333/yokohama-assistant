# The computer player

The bot that plays Yokohama in the terminal and in the browser: what it is, the names it plays under, how to ask it
about a move, and how strong it has been measured to be. Every figure on this page is quoted as its record states it,
with the record named beside it; the record carries the full account.

## 1. What a contestant is

Every bot is a **contestant**: a named identity made of three parts, a controller (the search that picks a move), the
controller's knobs, and a weight table (the numbers the evaluator prices a position with). The name is the identity.
The save file, the arena's report and the bot's own explanation all carry the same echo of it (§2), and a change to
any part of it, one knob or one weight, makes a different contestant under a different name.

**A contestant is honest by construction.** It decides from a `PlayerView`, the game as its own seat sees it, and is
never handed the engine's raw state. The view masks the five things a seated player cannot know: the order of the
order deck, the order of the technology deck, the order cards boxed at setup, each rival's secret starting order until
it is fulfilled, and the setup seed that would regenerate them all. To decide, the bot samples one world consistent
with what its seat has seen and searches inside that guess. A sampler that cannot produce a world is a hard error,
never a quiet fall back to the true state.

**Every strength claim passes through one court, the arena.** Its cells are paired and seat-swapped: at two players a
design unit is one dealt world played twice with the seats exchanged, so the first-seat advantage cancels inside the
unit. A cell's figure is θ̂, the first contestant's share of the games averaged over units, so 0.5 is parity at two
players. The interval around it is a bootstrap over units, never over games, because the two games of a unit are not
independent. When every unit reads the same value, the arena does not print a zero-width interval: it reports a typed
refusal carrying a one-sided bound instead. The number of units is fixed before a cell runs, and a master seed whose
figures get quoted is drawn from the operating system's entropy and recorded before use.

The rules of evidence on this page are the project's own: a one-sided bound is called one-sided, an observed maximum
is "max observed under harness X" and never a bound, and no direction is stated where none was measured.

## 2. The named identities

- **`v4-step9-draft1`, the production contestant.** It is the terminal's default bot seat, the advisor behind
  `suggest`, and the bot of the browser client. Its echo, as the V4 freeze records it in §1
  ([`plans/2026-08-30-phase3.5c-v4-identity-freeze.md`](plans/2026-08-30-phase3.5c-v4-identity-freeze.md)):

  ```
  v4-step9-draft1@v2{b=4,eval_impl=3,k=1,m=8,weights=6957643967275421640}
  ```

  `@v2` is the lookahead controller's version, `b=4` its beam, `k=1` its one sampled world and `m=8` its retreat
  samples; `eval_impl=3` is the evaluator module's revision and `weights=` the digest of the weight table. The search
  plays its own turn out to the hand-over, four lines wide, and scores the position there.
- **`lookahead-v1` and `greedy-v1`, grandfathered.** The two earlier evaluator controllers run the V2 weight table
  byte for byte, untouched by V4, so a figure recorded against either name at V2 still describes the code. Each echo
  carries its controller's version: the 3.3 figures in §5, recorded at the V1 table, read `lookahead-v1@v1`.
  `lookahead-v1` is the same turn-boundary search as V4's; `greedy-v1` scores one move ahead.
- **The rungs: `uniform`, `retreat-averse` and `family-weighted`.** Rollout policies that sample a legal move rather
  than score one: uniformly, never volunteering a retreat, or weighted by move family. They are the ladder's floor,
  and `family-weighted` is also the policy the fuzzer plays.
- **The diagnostics and the research identities.** `atomic-diag-v1`, an atomic-depth search kept from the depth
  experiment, and `mcts-v1`, the tree search measured in §5 and not adopted, are labelled diagnostics. `v2-padded`,
  the V2 table padded to the sighted evaluator's width, and `v2-refit-padded`, a refit control, are research
  identities for arena weight comparisons. The terminal refuses all four as a bot seat, each kind with its own
  sentence.

**A name never re-points.** A different weight table is a different named identity, never a knob and never an edit,
and every figure stays tied to the full echo it was recorded under. The arena's runner seats every name above; its
grammar is in [`CLAUDE.md`](../CLAUDE.md)'s Commands section. A new table, in the terminal or in the browser, deals
the production contestant, and a save names the bot it was played against.

## 3. V4: production, and provisional

`v4-step9-draft1` shipped as itself on 2026-08-30, at the merge of pull request #30. The identity the confirmatory
field measured is the identity that plays, byte for byte and name for name, as the V4 close records in §3.1
([`plans/2026-08-30-phase3.5c-v4-close.md`](plans/2026-08-30-phase3.5c-v4-close.md)). Its table was frozen before the
field ran (the V4 freeze, §1 and §6), and it had one counted attempt in that field.

**It is production, and it is provisional.** The route ratified for reaching the bar runs through screening on
development fields, then a freeze of the production identity, then one confirmatory experiment (amendment D1 of
[`plans/2026-08-11-phase3.5c-scoping-amendment.md`](plans/2026-08-11-phase3.5c-scoping-amendment.md)). V4 is
production ahead of that post-screening freeze, so the freeze may name a different contestant.

**Games played against V4 keep their bot.** A later build plays an unfinished V4 game on exactly when it still derives
V4's exact descriptor, with the controller and evaluator versions unmoved. A build that moves either refuses the game
as drift and says why, rather than playing it as another bot ([`web-lane.md`](web-lane.md) §6 and §15). The terminal
and the browser apply the same rule.

## 4. Suggest and Why

Both are offered in the terminal's simulated games and at the browser's tables. Neither applies, records or saves
anything.

- **`suggest`** runs the production contestant on *your* seat's view and shows what it would play, the line it looked
  at, the arithmetic behind the score term by term with the products adding up to the total printed under them, and
  the runner-ups. Two presses at one position give the same answer, because the bot's randomness is keyed by the
  game, the seat and the move count.
- **`why`**, or `why <n>` for one event in the log, does the same for a move a bot already played. The game is
  replayed to that point and the bot's own decision is re-run under the same key, so the account is a reconstruction
  the engine can vouch for rather than a note someone stored.

**The honesty gate.** If the reconstruction does not reproduce the move the record says was played, `why` refuses and
names both moves and both versions, rather than showing confident arithmetic for a move nobody made. A rung's seat is
refused as well: a rung samples its move, so there is no arithmetic to show.

**It is the bot's arithmetic, not advice.** The search is one turn deep in one sampled world, the rivals' hidden cards
are guessed, and nothing here has been tuned on, or measured against, a human player. `suggest` is refused in a
live-tracked game, where the hidden cards are a model reconciled from the physical table rather than the engine's
deck. The terminal's words are in [`cli-guide.md`](cli-guide.md), the browser's in [`web-guide.md`](web-guide.md) §3.

## 5. The results ladder

One entry per strength milestone, oldest first. Each states the claim as it was registered before its field ran, the
figure as measured with its interval, and the record that holds both. Every field below is two-player.

**3.3, the first purposeful bot (2026-07-27).** The registered claim: `lookahead-v1`, then at the V1 table, beats the
`family-weighted` rung, read on either of two pre-committed shapes, an interval entirely above 0.5 or a sweep whose
one-sided bound lies above 0.5. The [3.3 close report](plans/2026-07-27-phase3.3-close-report.md) §1 records
θ̂ 1.000000 on the committed master and again on a master freshly drawn from OS entropy, all 200 units swept in each.
With no variation left to resample, the arena refused an interval; the 3.3 close report gives the rule-of-three bound
at both arms as 0.985 > 0.5, a ONE-SIDED bound and not a two-sided interval.
The same record gives the mean VP margins as +67.42 and +68.44.

**3.4, the architecture experiment (2026-08-05).** The registered claim: MCTS-v1 is adopted only if both node-matched
arms and the wall-matched cell land entirely above 0.5 and the capped arm is certified as the shipped rung's proxy;
every other outcome retains the shallow search. The
[3.4 decision record](plans/2026-08-04-phase3.4-architecture-decision.md) gives the first node-matched arm as
θ̂ 0.045000 [0.027500, 0.065000].
The second, on a freshly drawn master, reads θ̂ 0.050000 [0.030000, 0.072500] in the same record.
The wall-matched cell, against the shipped rung, reads θ̂ 0.070000 [0.035000, 0.105000] in the same record.
All three intervals lie entirely below 0.5: MCTS-v1 is NOT adopted, and the turn-boundary shallow search is retained
on the merits (§5.5 of that record). `mcts-v1` stays reachable as a labelled diagnostic.

**V2, the second weight table (2026-08-11).** The registered claim: `lookahead-v1` at the new table beats itself at V1,
at the shipped knobs, with an interval entirely above the null. The
[V2 freeze package](plans/2026-08-10-phase3.5c-freeze-package.md) §4 records θ̂ 0.6775, bootstrap [0.6275, 0.7250], at
N=200 units. The one-ply `greedy-v1` cell, reported and not gated, read 0.4825 [0.4325, 0.5300] in the same section.
V2 was accepted under a narrow claim, a materially stronger successor to V1 for the retained two-player turn-boundary
controller at its shipped knobs (the decision log's 2026-08-11 row in
[`guidingpath-archive-phase3.md`](../guidingpath-archive-phase3.md)).

**V3, two candidate features (2026-08-19).** The registered claim: each candidate is admitted only through its
pre-registered gates. The [V3 close handover](plans/2026-08-19-phase3.5c-v3-close-handover.md) records the cycle as
NULL: both candidates washed at their pre-registered gates, terminal-margin awareness at L2 and
`achievement_race_proximity` at L3. Nothing was admitted, frozen or shipped, and no play behaviour changed.

**V4, the sighted evaluator (2026-08-30).** The registered claim, in one confirmatory field with one counted attempt:
`v4-step9-draft1` beats both `v2-padded` (V2's behaviour) and `v2-refit-padded` (the refit control), each 95% interval
entirely above 0.5, with a healthy self-null; a θ̂ at or above the minimum meaningful edge of 0.55 was registered as an
annotation, not a condition, and shipping was further conditioned on a re-read of escrowed regression blocks (the
V4 freeze, §7). The V4 freeze §9.9 records the result, both θ̂ above that edge:
- against `v2-padded`, θ̂ 0.691 [0.667, 0.715] over 650 read units;
- against `v2-refit-padded`, θ̂ 0.885 [0.867, 0.902] over 650 read units, a clear the freeze says tells less than the
  first, because the control's own self-play is known to deadlock;
- the self-null, V4 against itself, θ̂ 0.498454 [0.473725, 0.523184] over 647 read units (§9.9, item f). Its interval
  contains 0.5, which is a health reading of the instrument and not a strength claim;
- three of the self-null's 1,300 games reached the turn cap, 0.23% (Wilson 95% [0.08%, 0.68%]), inside the
  diagnosis's self-play rate of 0.30% [0.14%, 0.65%]; they were counted, not refused, as the amended registration
  directs (§9.9, item f).

A first run of the same field was invalid: the self-null refused at its first turn-capped game, which does not spend
the attempt, and that run's superiority figures were sealed unread. A signed amendment then made a capped game a
counted event, and the counted attempt ran against the unchanged identity on a fresh master (the V4 freeze, §9.1 to
§9.8). The escrow re-read is recorded in the V4 close §2.6: three valid arms, each VOID and non-blocking, zero failed
admitted roots, and each arm's one admitted root read 0/20.

## 6. The bar, not claimed

The map that governs the computer player defines its milestone bar in §1.9
([`plans/2026-07-25-phase3-map.md`](plans/2026-07-25-phase3-map.md)):

> **The milestone bar** ("provably plays to win", 2P) requires **all four**: a constructed-scenario suite ·
> statistical superiority over preserved rungs (paired seeds, seat swaps, CIs) · decision explanations for every
> chosen move · owner acceptance play through the CLI.

The bar (map §1.9) is NOT claimed: a shipped milestone is a narrower claim than the bar.

V4's pass is the milestone's, not the bar's, as the V4 close says in §3.3. The bar is the owner's to declare, last,
after the confirmatory battery and the acceptance play that §7's route ends with; the execution ledger names that
declaration OD-11.

## 7. What is next

**The V4 vetting.** The board ([`handover.md`](handover.md), Track 1) records three complete games against the shipped
bot, with every bot move scribed by `why`. The owner annotates them turn by turn; a findings package follows, a dated
document that pairs each annotation with the scribe's and the reviewers' grounding. Then two held decisions are ruled
together (the V4 close, §3.2 and §3.4): D3, whether to open a bounded diagnostic of the opponent's reply inside the
shallow search, and W6, whether knob screening opens.

**Then V5, through its own pre-registration.** The route after the vetting is the scoping amendment's, as the board's
Track 1 lists it: screening on development fields, a freeze of the production identity, one confirmatory battery under
a signed pre-registration, then acceptance play and the bar. Which way V5 opens waits on the D3 and W6 rulings. A
weight change is a different contestant, never a tune, so V5 will carry its own name and V4 keeps its figures.

## 8. Known limits, direction-free

Each of these is recorded. None carries a claim about strength.

- **V4's self-play can deadlock.** In the V4 freeze §9.4, 6 of 2,000 V4 self-play games reached the turn cap, a rate
  of 0.300% with a Wilson 95% interval of [0.138%, 0.653%], which the freeze's later sections round to
  0.30% [0.14%, 0.65%]. The loop is a produce-and-hoard cycle in which the game's clocks never close (the V4 close,
  §2.6). Against `v2-padded` or `v2-refit-padded` the freeze §9.4 records 0 capped games in 1,000 each, and it reads
  V4's self-play rate as not distinguishable from V2's at that size.
- **V4's self-play games run long.** The V4 freeze §9.4 records completed V4 self-play games at a mean of 54.2 turns
  against V2's 39.0. Its longest completed game ran 3,296 turns, max observed under `arena caprate` at 1,000 units,
  where V2's self-play max observed under the same harness was 138. The deadlock is the far end of that tail, and the
  V4 close §2.6 reads the loop's moves in two V4-against-V4 playouts that reached the cap, one shape in both.
- **A large first-seat share in V4 self-play.** The V4 freeze §9.9 records seat 0 winning 848 of the self-null's
  1,294 read games, 65.5% (≈ [62.9%, 68.1%] as recorded), and the V4 close §3.3 carries it as a later diagnostic's
  question. The swapped design cancels it inside every θ̂, and V2's share was not read, so no comparison is made.
- **One world, one turn, two players.** The search reads one sampled guess at the hidden cards and looks one turn
  ahead. The strength milestone is two-player, and three- and four-player strength work lives above the bar (the map,
  §1.2). Nothing has been tuned on, or measured against, a human player (the CLI guide's `suggest` section).
- **The release roster has outgrown its target.** The fifteen release gates first ran past their ≤420 s target on the
  M5 laptop at the V4 close, at 492.17 s (the V4 close, §2.7). The latest sequence the board records read 478 s on the
  same machine, in the after-review record of [the S2c plan](plans/2026-10-13-security-s2c-nas-identity-plan.md)
  dated 2026-10-17. Restating the target is open as OD-6, and every wall figure belongs to the machine that measured
  it.

The full record of the strength lane, step by step, is the
[execution ledger](plans/2026-08-09-phase3.5c-execution-ledger.md); the decision log in
[`guidingpath.md`](../guidingpath.md) and its archives carries every ruling.
