# The engine

The rules of Yokohama as a Rust library, the one the terminal client, the browser client and the bot are built on: how
to play a game through its API, where legality is decided, what a player may see, how it is verified and where its rules
come from.

## 1. What the engine is

The engine is one crate, [`crates/yokohama-core`](../crates/yokohama-core). Its rules are pure functions over a game
state: `get_legal_moves` lists what the player to move may do, and `apply_move` returns a new state, cloning the old one
and never mutating it. The state, `GameState`, is plain serde-serializable data, complete enough to resume a game from
at any point. The rule functions read and write no files and render nothing; the library's only output is the lenient
`replay`'s warnings on standard error (§5) and the record files of the arena's corpus emitter, part of the
computer-player tooling that shares the crate.

The state carries no history. A game's past lives outside it, in a `GameRecord`: the seed, the configuration, the setup
choices and every event since, in order (§5); the state stays cheap to clone, which a search does at every node. Every
other surface asks the engine and computes no rule of its own: the terminal client and the browser layer
([`web-lane.md` §4](web-lane.md#4-the-architecture-and-the-standing-rules)) apply only moves from the engine's legal
list, and the bot ([`computer-player.md`](computer-player.md)) chooses among the moves the engine offers.

The same crate also holds the computer player: the evaluator (`eval`), the controllers (`greedy`, `lookahead`, `mcts`),
the arena that measures them and the driven-fixture harness. They are built on the functions this page describes and
are no part of the rules.

## 2. A game through the API

The smallest complete program: build the rules, deal a seeded two-player game, settle the starting orders, loop legal
moves → pick one → apply until the game is over, and read the result. It needs only `yokohama-core`, as a path
dependency on `crates/yokohama-core` in a clone; the crate re-exports its items at its root.

```rust
use yokohama_core::*;

fn main() {
    let rules = Rules::standard();
    let config = GameConfig::for_player_count(2);
    let pending = new_game(config, &rules, 42).expect("a valid config");
    let keep = [StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)];
    let mut state = finalize_setup(pending, &keep).expect("one choice per seat");

    let mut applied = 0;
    while !is_game_over(&state) {
        assert!(state.turn_number <= HONEST_MAX_TURNS, "no end in sight");
        let moves = get_legal_moves(&state, &rules);
        let mv = &moves[applied % moves.len()];
        state = apply_move(&state, mv, &rules).expect("a listed move is legal");
        applied += 1;
    }

    let result = final_result(&state, &rules);
    println!("{applied} moves over {} turns", state.turn_number);
    for s in &result.standings {
        println!("rank {}: {:?}, {} VP", s.rank, s.player, s.breakdown.total);
    }
}
```

Its last two lines of output:

```text
rank 1: PlayerId(0), 90 VP
rank 2: PlayerId(1), 68 VP
```

The rotating index stands in for a player. Always taking the first listed move would not do: it is a no-op cycle (end
the additional actions, place no assistant, recall the President recovering nothing) that never ends the game, as
[`agent.rs`](../crates/yokohama-core/src/game/agent.rs) records. The assertion bounds the loop by the engine's own
non-termination guard, `HONEST_MAX_TURNS`; no pick rule is promised to finish. `final_result` ranks the seats with score
breakdowns, breaking a tie clockwise from the start player.

For more, `cargo run --release -p yokohama-core --example selfplay -- <games> <players> <base_seed>` plays seeded random
games and checks every state ([`examples/selfplay.rs`](../crates/yokohama-core/examples/selfplay.rs)). Its driver is in
[`tests/common/mod.rs`](../crates/yokohama-core/tests/common/mod.rs), where `run_random_game` is the tree's simplest
complete game loop.

## 3. State, moves and the turn

`GameState` ([`types/state.rs`](../crates/yokohama-core/src/types/state.rs)) holds the whole table: the board, every
player's pieces, money and cards, the ports and laboratories, both decks, the tracks, the achievements, the end-game
flags and the queue of unresolved choices. A `Move` ([`types/moves.rs`](../crates/yokohama-core/src/types/moves.rs)) is
one decision of the player to move, from an additional action or a placement to a President move, an area action, a
build or the resolution of a queued choice. A turn is a phase machine, `TurnPhase`, whose phases run in this order:

`AdditionalActionFirst` → `Placement` → `Movement` → `AreaAction` → `FivePowerBonus` → `Building` → `Recovery` →
`AdditionalActionSecond`

`Recovery` offers no moves: the engine recovers the assistants itself, after first queueing one choice for a player who
owns the Telephone technology and has an assistant in the acting area. After `AdditionalActionSecond` the turn passes to
the next player's `AdditionalActionFirst`; the ninth phase, `TurnEnd`, is assigned only when the final round completes,
and a finished game rests in it. A mercenary runs a nested sub-turn, from the area action to recovery, then returns to
the phase it came from.

A rule effect that needs a player's decision is never resolved for them. It is queued in the state as a `PendingChoice`
([`types/choices.rs`](../crates/yokohama-core/src/types/choices.rs)), and while one heads the queue the legal moves are
that choice's resolutions and nothing else. An "any trade good" reward queues `AnyGood`, resolved by `ChooseAnyGood`
with no decline, since the gain is unconditional; a warehouse icon queues `WarehouseTake`, resolved by
`ChooseWarehouseTake` or `DeclineWarehouseTake`; a Church action that reaches two or more eligible faith spaces queues
`ChurchSpace`, while a single eligible space is taken at once.

## 4. Where legality is decided

**`get_legal_moves` is the authority** ([`game/legal_moves.rs`](../crates/yokohama-core/src/game/legal_moves.rs)): the
complete, ordered list of legal moves for the current state. Every other legality answer in the crate is defined
against it: tested as agreeing with it, or documented as deliberately wider (`is_move_legal`, below).

**`apply_move` is the checked boundary** ([`game/apply_move.rs`](../crates/yokohama-core/src/game/apply_move.rs)). It
rejects a move that is not in the current legal list with `MoveError::NotLegal`, then applies it. Behind it,
`apply_move_unchecked` is the raw transition: it checks only the structure of the move in hand, not whether the turn
phase allows it or the list holds it, so it returns `Ok` for some moves that are not legal. It is for a caller that has
just taken the move from the legal list, such as the fuzz driver, which would otherwise build the list twice per move.

**An untrusted record gets the exact single-move check.** `is_listed_move`
([`game/move_legality.rs`](../crates/yokohama-core/src/game/move_legality.rs)) answers whether one move is in the legal
list without building the list's largest family, the President-recall product.
`replay_strict_with(.., MoveCheck::Predicate)` ([`game/record.rs`](../crates/yokohama-core/src/game/record.rs)) is the
strict replay built on it, the one an uploaded archive or a restored backup should get;
[`tests/replay_check_equivalence.rs`](../crates/yokohama-core/tests/replay_check_equivalence.rs) holds it to accepting
and refusing exactly what `replay_strict` does. Its sibling `is_move_legal`, for a search, is deliberately wider: it
accepts any rules-legal path for a President move where the list prints one.

**The factored action source is a view for search, not a second authority.**
[`game/search_actions.rs`](../crates/yokohama-core/src/game/search_actions.rs) names the move families legal at a state
(`families`) and pages or samples within one (`family_len`, `iter_family`, `sample_family`) without materializing it. It
is purely additive; a release gate holds its families' union to `get_legal_moves`, as a multiset and in order.

## 5. Setup, seeds, saves and replay

**Setup is two calls** ([`game/setup.rs`](../crates/yokohama-core/src/game/setup.rs)). `new_game(config, &rules, seed)`
lays out the board, shuffles both decks, fills the ports and laboratories, draws one achievement per category and deals
each player two order cards, returning a `PendingSetup`; the same seed always produces the same setup.
`finalize_setup(pending, &choices)` applies each seat's `StartingOrderChoice`: `Keep(0)` or `Keep(1)` keeps that card as
the public starting order, and `Unknown` keeps the deal private, as when tracking a physical table. The decks live in
the state, last element on top, beside `orders_removed`, the order cards out of play. `Rules::content_hash()`
([`game/rules.rs`](../crates/yokohama-core/src/game/rules.rs)) is a stable hash of the card, site and token data and the
board derivation version.

**A `GameRecord` is the save format.** `GameRecord::new` stamps the engine version, the rules hash and the current
`SEMANTICS_REV`; `push_move` appends each applied move; `stamp_terminal_digest` records a digest of the final state.
Three functions read a record back:

- `replay` rebuilds the setup and folds the events in order, every move through `apply_move` and every card reveal of
  a live-tracked game through its own state method; on a rules-hash or revision mismatch it warns and replays anyway.
- `replay_strict` refuses a mismatched rules hash and a missing or mismatched revision, checks the invariants after
  every event, and verifies a stamped terminal digest.
- `state_at(record, k)` is the state after the first `k` events, so `state_at(record, n)` is the position in which event
  `n` was decided; a `k` past the end is an error, never clamped.

None of the three refuses on the engine version.

**`SEMANTICS_REV`** ([`game/semantics.rs`](../crates/yokohama-core/src/game/semantics.rs)) is the engine's rules
revision, currently **7**. It is bumped in the same commit as any behaviour-affecting change to legality, application,
scoring or setup; card data edits move the rules hash instead. Saves and records are keyed to it, not to the crate
version, so a save stops resuming across a bump, by design: the terminal client refuses to load a save from another
revision. Each revision's scope is a row in the decision log
([`method.md` §3](method.md#3-the-decision-log-and-the-semantics_rev-contract)).

## 6. What a player may see

A `GameState` holds more than any seated player knows. Five of its members are hidden from a seat: the setup seed, which
regenerates everything else; the order of the order deck and of the technology deck, whose sizes alone are public; the
contents of the cards removed at setup; and each opponent's starting order until it is fulfilled.

`observe(&state, seat)` ([`game/observe.rs`](../crates/yokohama-core/src/game/observe.rs)) produces that seat's
`PlayerView` ([`types/view.rs`](../crates/yokohama-core/src/types/view.rs)) with the five masked. It names every state
field, so a new one fails to compile there until it is classified, and it refuses a seat whose own deal was recorded as
`Unknown`. `BeliefSampler` ([`game/belief.rs`](../crates/yokohama-core/src/game/belief.rs)) turns a view back into full
states, `from_view(&view, &rules, seed)` then `sample()` per world, each consistent with what the seat has seen. A
sampled world is not safe to roll forward across a port or laboratory refill, which reveals a deck card.

**A bot takes a `PlayerView`, never a raw `GameState`.** Neither agent interface names `GameState` in a method
signature, so types hold the boundary: `HonestAgent` in [`game/agent.rs`](../crates/yokohama-core/src/game/agent.rs),
and `ArenaAgent` in [`game/arena.rs`](../crates/yokohama-core/src/game/arena.rs), the one the shipped bot and every
measured contestant implement. One channel stays open by nature: the public board is generated from the seed, so a small
seed can be recovered from it, and the hidden set with it; an evaluation draws full-range `u64` seeds and records them.
How the bot uses the view is in [`computer-player.md` §1](computer-player.md#1-what-a-contestant-is).

## 7. How it is verified

**An independent oracle.** [`crates/yokohama-oracle`](../crates/yokohama-oracle) is a second implementation of the legal
moves and the final scoring, written from the rules sources and the decision log, never from the engine's logic. Its
manifest depends on `yokohama-core` and `serde` alone, and every name it imports from the engine is a type or the
`Rules` data bundle; `tests/oracle_independence.rs` in the differential crate checks both against an allowlist.

**A differential harness.** [`crates/yokohama-difftest`](../crates/yokohama-difftest) drives seeded games through the
engine and compares the engine's `get_legal_moves` with the oracle's moves at every decision point, and its
`final_result` with the oracle's standings at every finished game, writing a self-contained artifact with the full state
on any disagreement.

**Self-play fuzzing and the choice census.** The fuzz gate plays random games to completion and runs `check_invariants`
([`game/invariants.rs`](../crates/yokohama-core/src/game/invariants.rs)) after every applied move: conservation, no
double claims, every field in range. The choice census checks that every `PendingChoice` variant and every move that
resolves one fires in organic random play. Further suites check the view's masking, the sampler's worlds and the
factored action source's equivalence to the legal list. The heavier checks are the release roster, listed by command in
[`method.md` §5](method.md#5-results) and run before a merge; the per-edit suites, run after every change, are in
[`CONTRIBUTING.md`](../CONTRIBUTING.md#how-a-change-lands).

## 8. The rules and their rulings

The engine's rules come from the knowledge base's [rules reference](../analysis-1/yokohama-kb/01-rules-reference.md) and
from the decision log in [`guidingpath.md`](../guidingpath.md#decision-log), which carries every rules ruling. Its older
rows live verbatim in archives, the engine era's in [`guidingpath-archive.md`](../guidingpath-archive.md), and an
archived row stays authoritative wherever it is cited. The register of the rules' ambiguities is
[`plans/2026-07-20-phase-2.9-ruling-list.md`](plans/2026-07-20-phase-2.9-ruling-list.md). Two English editions of the
rulebook, the OKAZU Brand edition and the Tasty Minstrel Games edition, differ in places; neither is in the tree, and
where they differ or are silent the decision log settles the rule
([`CONTRIBUTING.md`](../CONTRIBUTING.md#the-stops-and-the-rules-sources)). A few rulings deliberately follow Board Game
Arena's implementation so that games captured there replay through this engine, the disagreement recorded in the log's
2026-08-08 rows.

A rules question, where the engine allows, refuses or scores something differently from the rules or from your table,
goes in the repository's **Rules question** form, one of the two forms a
[new issue](https://github.com/jmoore2333/yokohama-assistant/issues/new/choose) offers. One rules defect is known,
tracked as
[issue #62](https://github.com/jmoore2333/yokohama-assistant/issues/62); its fix bumps `SEMANTICS_REV`.

## 9. The card data and the board

The card data's source of truth is the knowledge base's YAML
([`analysis-1/yokohama-kb/`](../analysis-1/yokohama-kb/00-README.md)): the 36 order cards, keyed by the number printed
in each card's corner, the technology cards and the building sites. `cargo run -p gen-data`, from the workspace root,
regenerates [`game/data.rs`](../crates/yokohama-core/src/game/data.rs) from it, and that file is never edited by hand.

The area board ([`game/board_gen.rs`](../crates/yokohama-core/src/game/board_gen.rs)) is a staggered pyramid, widest row
on top: rows of 4/3/2/1 tiles at two players, 5/4/3/2 at three and 6/5/4/3 at four. Each tile sits in the notch between
two above it and touches up to six neighbours; tiles meeting only at a corner are not adjacent. At three and four
players the Canal joins any two of its neighbours for President movement, at 1 Yen per crossing to the bank; no pawn is
ever placed on it or stops there.
