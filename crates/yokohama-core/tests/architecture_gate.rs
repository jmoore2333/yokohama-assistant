//! The architecture gate — Phase 3.4, spec
//! `docs/plans/2026-08-04-phase3.4-search-arch-spec.md` §8.
//!
//! **What is here TODAY is the per-edit plumbing smoke**
//! ([`the_architecture_battery_runs_at_smoke_scale`], never `#[ignore]`d). The
//! release-tier cells §8 registers as roster gate #14 — MCTS determinism at a
//! C2-shaped cell run twice, the honest-counter identities, and the smoke-scale
//! null — arrive with their own task (T7) and land in THIS binary, together with
//! the four-doc-site registration a new `#[ignore]` requires. The smoke exists
//! from the first task that has something to smoke-test, on `arena_gate`'s and
//! `evaluator_gate`'s stated grounds: a binary whose only tests are release-only
//! rots invisibly between the day it is written and the day the gate first runs.
//!
//! **Why this smoke drives AGENTS rather than arena cells.** Every other gate
//! binary in the repo runs `run_cell` over [`Contestant`]s. MCTS-v1 became a
//! contestant at T5 (`e0ebe58`); this smoke still drives the AGENTS directly
//! because a plumbing smoke should not depend on the cell layer T7 will
//! exercise. So the two arms below drive
//! what exists: the MCTS agent through its own honest chain, and the capped S1
//! arm through the public `run_arena_game_capped` driver it already reaches. T7
//! replaces this file's contents with cells; it does not replace this test.
//!
//! Scale: both arms are DEVELOPMENT size (a bounded decision run, one capped
//! game), and neither asserts anything statistical. A smoke that asserted a
//! direction at two units would be asserting noise.

use std::sync::{Arc, Mutex};

use yokohama_core::game::lookahead::LookaheadAgent;
use yokohama_core::game::mcts::MctsAgent;
use yokohama_core::types::{GameConfig, Move, PlayerId};
use yokohama_core::{
    agent_stream_seed, apply_move_unchecked, finalize_setup, new_game, observe,
    run_arena_game_capped, ArenaAgent, ArenaGameStats, RecordedEvent, Rules, Slot,
    StartingOrderChoice, WorldSeed,
};

/// This binary's committed diagnostic master (spec §3's gate-cell exemption,
/// inherited from 3.2/3.3: a per-edit pin needs a fixed master, and nothing here
/// is a bot-facing evaluation run whose figure gets quoted).
const MASTER: u64 = 0x3C15_2026_0804_0001;

/// The experiment's budget unit, B\* (spec §5) — used here only so the smoke
/// exercises the arm at the size the cells will.
const B_STAR: u32 = 750;

/// Decisions the MCTS arm takes. Bounded rather than a whole game: the point is
/// that the wiring works and the meters move, and a full MCTS game at B\* is
/// release-tier work.
const MCTS_DECISIONS: u32 = 20;

/// The capped-S1 arm's cap. **8, and deliberately not a bigger one**: spec §5
/// records a MEASURED non-terminating band at caps ≈32–128 for this arm, so a
/// smoke sitting there would fail for a reason the experiment's
/// completion-failure surface already owns. At 8 the arm is starved at every
/// large decision and completes (T1: 5/5 masters, 56–260 turns).
const S1_CAP: u32 = 8;

/// Assistants a game actually put on the board, counted off the record's applied
/// moves — `lookahead`'s helper verbatim, and its reason: a final state may have
/// recovered every assistant it ever placed.
fn assistants_placed(stats: &ArenaGameStats) -> usize {
    stats
        .record
        .events
        .iter()
        .map(|e| match e {
            RecordedEvent::Move(Move::PlaceThreeAreas { placements }) => placements.len(),
            RecordedEvent::Move(Move::PlaceTwoSameArea { adjacent_bonus, .. }) => {
                2 + usize::from(adjacent_bonus.is_some())
            }
            _ => 0,
        })
        .sum()
}

/// **THE PER-EDIT SMOKE.** Both 3.4 arms run end to end at development scale:
/// MCTS-v1 through its honest chain under the shared budget cap, and the capped
/// S1 arm through the public arena driver. Nothing statistical is asserted.
#[test]
fn the_architecture_battery_runs_at_smoke_scale() {
    let rules = Rules::standard();

    // -- arm 1: MCTS-v1, both seats, under B* ------------------------------
    // `simulations` at a CEILING with the cap doing the bounding — the
    // CONTESTANT shape (spec §5), and deliberately not a simulation count with a
    // cap beside it: the sterile-simulation hang lived exactly in this shape, and
    // a smoke that never ran it would never smoke-test the guard. It stays cheap
    // because the cap is what bounds the work.
    let mut agent = MctsAgent::new(&rules, agent_stream_seed(WorldSeed(MASTER), PlayerId(0), Slot(0)))
        .with_simulations(u32::MAX)
        .with_cap(Some(B_STAR));
    let tally = agent.tally();
    let pending = new_game(GameConfig::for_player_count(2), &rules, MASTER).expect("2P setup");
    let mut state =
        finalize_setup(pending, &[StartingOrderChoice::Keep(0), StartingOrderChoice::Keep(0)])
            .expect("finalize");
    let mut placed = 0usize;
    for _ in 0..MCTS_DECISIONS {
        assert!(!state.is_game_over(), "the smoke must not run out of game");
        let seat = state.current_player;
        let view = observe(&state, seat).expect("observe");
        let report = agent.choose(&view, &rules).expect("the honest chain produces a move");
        if let Move::PlaceThreeAreas { placements } = &report.mv {
            placed += placements.len();
        }
        state = apply_move_unchecked(&state, &report.mv, &rules).expect("a chosen move applies");
    }
    println!(
        "mcts-v1 @cap {B_STAR}: {} decisions · {} scored ({} root) · binds {} · starved {} · \
         overshoot {:?} · {placed} assistants placed",
        tally.decisions(),
        tally.candidates_scored(),
        tally.root_scored(),
        tally.budget_binds(),
        tally.budget_starved_decisions(),
        tally.overshoot(),
    );
    assert_eq!(tally.decisions(), u64::from(MCTS_DECISIONS), "one decision per applied move");
    assert!(tally.candidates_scored() > 0, "the meter must have counted the search's spend");
    assert!(
        tally.root_scored() > 0 && tally.root_scored() < tally.candidates_scored(),
        "the mandatory root pass must be a real fraction of a real total",
    );
    assert!(
        tally.budget_binds() > 0,
        "anti-vacuity: at B* = {B_STAR} the cap must actually bind somewhere in {MCTS_DECISIONS} \
         decisions",
    );
    assert_eq!(
        tally.overshoot().map(|o| o.decisions),
        Some(u64::from(MCTS_DECISIONS)),
        "every capped decision contributes an overshoot sample, zero or not",
    );

    // -- arm 2: the capped S1 arm, one whole game through the arena driver --
    let world = WorldSeed(MASTER);
    let tallies: Arc<Mutex<Vec<_>>> = Arc::default();
    let sink = Arc::clone(&tallies);
    let mut make = |seat: PlayerId, _n: u8| -> Box<dyn ArenaAgent> {
        let agent = LookaheadAgent::new(&rules, agent_stream_seed(world, seat, Slot(seat.0)))
            .with_cap(Some(S1_CAP));
        sink.lock().expect("tallies").push(agent.tally());
        Box::new(agent)
    };
    let stats = run_arena_game_capped(world, 2, &rules, &mut make, true, 1_500)
        .expect("a capped S1 game must complete");
    let handles = tallies.lock().expect("tallies").clone();
    let scored: u64 = handles.iter().map(|t| t.candidates_scored()).sum();
    let binds: u64 = handles.iter().map(|t| t.budget_binds()).sum();
    println!(
        "lookahead-v1 @cap {S1_CAP}: turns {} · moves {} · scored {scored} · binds {binds} · \
         {} assistants placed",
        stats.turns,
        stats.moves_applied,
        assistants_placed(&stats),
    );
    assert!(stats.turns < 1_500, "the capped game must END inside the turn budget");
    assert!(assistants_placed(&stats) > 0, "and still place assistants");
    assert!(binds > 0, "anti-vacuity: at {S1_CAP} the cap must have denied work");
}
