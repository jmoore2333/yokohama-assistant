//! The honest-agent surface (Phase 3.0): what a bot is allowed to see.
//!
//! `HonestAgent`'s method signatures are the honesty boundary. They cannot
//! name `GameState` or `PendingSetup`, so a bot CANNOT read the deck order,
//! the boxed pile, or an opponent's hidden starting order -- not by
//! discipline, by types. The setup seed is likewise unreadable *directly*, but
//! the public board is an injective fingerprint of it: a disclosed residual,
//! closed by seed hygiene and exercised by the seed-recovery red team (see
//! SEED HYGIENE below), not by these signatures.
//!
//! Scope, stated honestly (spec §3.5):
//!   * per-decision channel -- closed by these signatures;
//!   * construction channel -- closed HERE by `run_honest_game` building
//!     agents through a factory it invokes itself, after `new_game`, passing
//!     only `(seat, player_count)`. An agent handed a `GameRecord` outside the
//!     driver would still see everything (a record carries `seed` + `choices`),
//!     which is why "bots never receive records" is a review-enforced rule;
//!   * coalition channel -- two agents sharing state via `Rc`/`Arc` pool their
//!     views. Documented, not prevented.
//!
//! SEED HYGIENE (spec §1-B / L7, binding for real evaluation): `new_game`'s
//! board is generated from the setup seed BEFORE the deck is shuffled, so the
//! public board is an injective fingerprint of that seed -- small sequential
//! seeds are brute-forceable in milliseconds, recovering the entire hidden
//! set. Any driver evaluating agents for real (3.2's arena, any strength
//! claim) MUST draw its game seeds uniformly over the full u64 and RECORD
//! them. Honesty-suite tests use small fixed seeds deliberately: their agents
//! are not adversarial, and the residual is proven executable by the
//! seed-recovery red-team test instead.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::game::apply_move::apply_move_unchecked;
use crate::game::belief::BeliefSampler;
use crate::game::end_game::is_game_over;
use crate::game::invariants::check_invariants;
use crate::game::legal_moves::get_legal_moves;
use crate::game::observe::observe;
use crate::game::rules::Rules;
use crate::game::scoring::{final_result, GameResult};
use crate::game::setup::{finalize_setup, new_game, StartingOrderChoice};
use crate::types::{
    EndGameTrigger, GameConfig, Move, OrderCardId, PlayerId, PlayerView, TurnPhase,
};

/// Non-termination guard, deliberately the same value as the fuzz driver's
/// `MAX_TURNS` (`crates/yokohama-core/tests/common/mod.rs:43`): uniform-random
/// play wanders (observed max 6036 turns at 2P seed 191), and the two loops
/// must not disagree about what "stuck" means.
pub const HONEST_MAX_TURNS: u32 = 10_000;

/// A player that acts only on what its seat may legitimately see.
pub trait HonestAgent {
    /// Setup: choose which of the two dealt Order cards to keep (index 0 or
    /// 1). The agent sees ONLY its own deal -- `PendingSetup` carries every
    /// seat's cards and the seeded deck, so the driver never passes it.
    fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8;

    /// Play: choose one of `legal`.
    ///
    /// PRECONDITION: `legal` is non-empty. `run_honest_game` returns a dead-end
    /// error before it would ever call an agent on an empty list, so an
    /// implementation may index `legal` without a bounds check (as
    /// `TrivialHonestAgent`'s uniform pick does). A caller reusing this trait
    /// (3.1+) owes the same guarantee.
    ///
    /// Being handed the engine's own legal list is leak-free for Keep-shaped
    /// states: legality is a pure function of the acting seat's legitimate
    /// view (verified exhaustively and by the independent oracle; pinned
    /// forever by the view-measurability suite). It is NOT true for a live
    /// `Unknown` seat, whose tracked hand under-counts by one -- see spec L8.
    fn choose_move(&mut self, view: &PlayerView, legal: &[Move], rules: &Rules) -> Move;
}

/// A cloneable tally of which arm `TrivialHonestAgent::choose_move` took. Every
/// clone shares one set of counters, so a driver can read the totals AFTER the
/// agent has been moved into `run_honest_game` and dropped (a game's agents are
/// built by the driver's factory and never handed back).
///
/// Its reason to exist is ANTI-VACUITY. A full-game release sweep can complete
/// every game while the belief sampler never actually drives a decision: the
/// fallback arm resolves to the same true legal list, so a suite that only
/// asserted "the game finished" would pass even if `from_view` errored on every
/// call. These counters make the two arms observable, so a gate can PROVE the
/// sampler path fired.
#[derive(Clone, Default)]
pub struct AgentPathTally {
    sampled: Arc<AtomicU64>,
    fallback: Arc<AtomicU64>,
}

impl AgentPathTally {
    /// Decisions resolved by the belief-sampler arm: a move drawn from a
    /// SAMPLED world's own legal list that was also in the seat's true list.
    pub fn sampled_pick_count(&self) -> u64 {
        self.sampled.load(Ordering::Relaxed)
    }

    /// Decisions resolved by the fallback arm: `from_view` errored, the sampled
    /// world had no legal move, or its pick was not in the true legal list (the
    /// documented L8 live-`Unknown` case). The fallback still returns a legal
    /// move -- only which arm produced it differs. Provably zero for Keep-shaped
    /// games, where the sampled and true legal lists coincide (spec F5).
    pub fn fallback_count(&self) -> u64 {
        self.fallback.load(Ordering::Relaxed)
    }
}

/// The 3.0 gate's agent: keeps a random card, then samples ONE world from its
/// view and picks uniformly from that world's legal moves.
pub struct TrivialHonestAgent {
    rng: ChaCha8Rng,
    tally: AgentPathTally,
}

impl TrivialHonestAgent {
    pub fn new(seed: u64) -> Self {
        Self { rng: ChaCha8Rng::seed_from_u64(seed), tally: AgentPathTally::default() }
    }

    /// A cloneable handle to this agent's arm tally. Clone it out BEFORE the
    /// agent is moved into `run_honest_game` to read the counters after the
    /// game -- the release sweep's mechanism for proving the sampler path (not
    /// the fallback) drove the decisions.
    pub fn tally(&self) -> AgentPathTally {
        self.tally.clone()
    }
}

impl HonestAgent for TrivialHonestAgent {
    fn choose_starting_order(&mut self, _seat: PlayerId, _deal: &[OrderCardId; 2]) -> u8 {
        self.rng.gen_range(0..2u8)
    }

    fn choose_move(&mut self, view: &PlayerView, legal: &[Move], rules: &Rules) -> Move {
        // The sampler seed comes from THIS AGENT'S OWN RNG stream and nothing
        // else. Deriving it from any state-flavoured value (`state_digest`
        // hashes the hidden members) would correlate the sample with the truth
        // it must be blind to.
        let sampler_seed: u64 = self.rng.gen();
        if let Ok(mut sampler) = BeliefSampler::from_view(view, rules, sampler_seed) {
            let world = sampler.sample();
            let world_legal = get_legal_moves(&world, rules);
            if !world_legal.is_empty() {
                let pick = world_legal[self.rng.gen_range(0..world_legal.len())].clone();
                // Roll the sampled world forward by the chosen move: the one
                // load-bearing use of a sample in 3.0, and exactly what 3.3's
                // search will do with it. Spelled as an explicit
                // `cfg!(debug_assertions)` block rather than `debug_assert!`,
                // whose argument is not evaluated in release at all -- this
                // way it reads as the debug-only CHECK it is. Skipping it
                // cannot change what this function returns:
                // `apply_move_unchecked` clones and never mutates, and the
                // rolled-forward world is discarded either way.
                if cfg!(debug_assertions) {
                    assert!(
                        apply_move_unchecked(&world, &pick, rules).is_ok(),
                        "a move drawn from a sampled world's own legal list must apply in it"
                    );
                }
                if legal.contains(&pick) {
                    self.tally.sampled.fetch_add(1, Ordering::Relaxed);
                    return pick;
                }
            }
        }
        // Fall back to the true legal list (equal to the sampled one for
        // Keep-shaped states; a live `Unknown` seat is the documented L8 case).
        self.tally.fallback.fetch_add(1, Ordering::Relaxed);
        legal[self.rng.gen_range(0..legal.len())].clone()
    }
}

/// Outcome of one honest game. Deliberately carries NO terminal `GameState`:
/// a terminal state holds the whole hidden set, and a stats struct in a
/// caller's hands is one convenience method away from an agent's memory.
/// Validation failures are hard errors, not counters.
#[derive(Debug, Clone)]
pub struct HonestGameStats {
    pub seed: u64,
    pub players: u8,
    pub turns: u32,
    pub moves_applied: u64,
    pub trigger: Option<EndGameTrigger>,
    pub result: GameResult,
    pub branching_by_phase: HashMap<TurnPhase, (u64, u64)>,
}

/// Play one full game with one `HonestAgent` per seat.
///
/// `make_agent` is invoked BY THIS FUNCTION, after `new_game`, with only
/// `(seat, player_count)` -- the caller never gets a chance to construct an
/// agent from privileged setup data.
pub fn run_honest_game(
    seed: u64,
    players: u8,
    rules: &Rules,
    make_agent: &mut dyn FnMut(PlayerId, u8) -> Box<dyn HonestAgent>,
) -> Result<HonestGameStats, String> {
    let pending = new_game(GameConfig::for_player_count(players), rules, seed)
        .map_err(|e| format!("new_game failed (seed {seed}, {players}P): {e}"))?;

    let mut agents: Vec<Box<dyn HonestAgent>> =
        (0..players).map(|i| make_agent(PlayerId(i), players)).collect();

    let mut choices: Vec<StartingOrderChoice> = Vec::with_capacity(players as usize);
    for (i, agent) in agents.iter_mut().enumerate() {
        let seat = PlayerId(i as u8);
        let deal = pending.deals[i];
        let keep = agent.choose_starting_order(seat, &deal);
        if keep > 1 {
            return Err(format!(
                "seat {} returned keep index {keep}: must be 0 or 1 (seed {seed}, {players}P)",
                seat.0
            ));
        }
        choices.push(StartingOrderChoice::Keep(keep));
    }

    let mut state = finalize_setup(pending, &choices)
        .map_err(|e| format!("finalize_setup failed (seed {seed}, {players}P): {e}"))?;

    // Guard the INCOMING state so setup corruption is not misattributed to the
    // first move (the same discipline as the fuzz driver).
    if let Err(reason) = check_invariants(&state, rules) {
        return Err(format!("corrupt starting state (seed {seed}, {players}P): {reason}"));
    }

    let mut branching_by_phase: HashMap<TurnPhase, (u64, u64)> = HashMap::new();
    let mut moves_applied: u64 = 0;

    loop {
        if is_game_over(&state) {
            return Ok(HonestGameStats {
                seed,
                players,
                turns: state.turn_number,
                moves_applied,
                trigger: state.trigger_reason,
                result: final_result(&state, rules),
                branching_by_phase,
            });
        }
        if state.turn_number > HONEST_MAX_TURNS {
            return Err(format!(
                "non-termination: turn {} exceeded HONEST_MAX_TURNS (seed {seed}, {players}P)",
                state.turn_number
            ));
        }

        let seat = state.current_player;
        let legal = get_legal_moves(&state, rules);
        let entry = branching_by_phase.entry(state.turn_phase).or_insert((0, 0));
        entry.0 += legal.len() as u64;
        entry.1 += 1;
        if legal.is_empty() {
            return Err(format!(
                "dead end: no legal moves in a non-terminal state (seed {seed}, {players}P, turn {}, {:?})",
                state.turn_number, state.turn_phase
            ));
        }

        let view = observe(&state, seat)
            .map_err(|e| format!("observe failed for seat {} (seed {seed}): {e}", seat.0))?;
        // Plumbing pin: an agent must never be handed another seat's view.
        assert_eq!(view.viewer, state.current_player, "view/seat mismatch in run_honest_game");

        let chosen = agents[seat.0 as usize].choose_move(&view, &legal, rules);
        if !legal.contains(&chosen) {
            return Err(format!(
                "seat {} chose a move that is not in the legal list (seed {seed}, {players}P, turn {}, {:?}): {chosen:?}",
                seat.0, state.turn_number, state.turn_phase
            ));
        }

        state = apply_move_unchecked(&state, &chosen, rules).map_err(|e| {
            format!(
                "apply_move_unchecked rejected a legal move (seed {seed}, {players}P, seat {}): {e}",
                seat.0
            )
        })?;
        if let Err(reason) = check_invariants(&state, rules) {
            return Err(format!(
                "invariant violated after seat {}'s move (seed {seed}, {players}P): {reason}",
                seat.0
            ));
        }
        moves_applied += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::rules::Rules;

    /// The brief specified `Move::EndTurn` for this test; no such variant
    /// exists at HEAD (Recovery is automatic and there is no explicit
    /// end-of-turn move -- `types/moves.rs`). Substituted a STRUCTURALLY
    /// impossible move instead of a merely out-of-phase one: church spaces are
    /// faith 4..=11, so `space: u8::MAX` is not in any legal list in any phase
    /// of any state, which keeps this test's premise true even if a future
    /// phase makes some choice-resolution move broadly legal.
    fn never_legal_move() -> Move {
        Move::ChooseChurchSpace { space: u8::MAX }
    }

    #[test]
    fn trivial_honest_agents_play_full_games_at_every_player_count() {
        let rules = Rules::standard();
        for players in [2u8, 3, 4] {
            let stats = run_honest_game(7, players, &rules, &mut |seat, _n| {
                Box::new(TrivialHonestAgent::new(1000 + seat.0 as u64))
            })
            .unwrap_or_else(|e| panic!("{players}P honest game failed: {e}"));
            assert_eq!(stats.players, players);
            assert!(stats.moves_applied > 0);
            assert!(stats.trigger.is_some(), "a completed game names its end trigger");
            assert_eq!(stats.result.standings.len(), players as usize);
        }
    }

    #[test]
    fn an_agent_returning_an_illegal_move_is_an_attributed_hard_error() {
        struct Cheater;
        impl HonestAgent for Cheater {
            fn choose_starting_order(&mut self, _s: PlayerId, _d: &[OrderCardId; 2]) -> u8 {
                0
            }
            fn choose_move(&mut self, _v: &PlayerView, _l: &[Move], _r: &Rules) -> Move {
                never_legal_move()
            }
        }
        let rules = Rules::standard();
        let err = run_honest_game(1, 2, &rules, &mut |_s, _n| Box::new(Cheater)).unwrap_err();
        assert!(err.contains("not in the legal"), "got: {err}");
        assert!(err.contains("seat"), "the error attributes the offending seat: {err}");
    }

    #[test]
    fn an_agent_returning_a_bad_keep_index_is_an_attributed_hard_error() {
        struct BadKeep;
        impl HonestAgent for BadKeep {
            fn choose_starting_order(&mut self, _s: PlayerId, _d: &[OrderCardId; 2]) -> u8 {
                7
            }
            fn choose_move(&mut self, _v: &PlayerView, l: &[Move], _r: &Rules) -> Move {
                l[0].clone()
            }
        }
        let rules = Rules::standard();
        let err = run_honest_game(1, 2, &rules, &mut |_s, _n| Box::new(BadKeep)).unwrap_err();
        assert!(err.contains("keep index"), "got: {err}");
    }

    /// Every test agent below that must play a game THROUGH to its end picks
    /// uniformly at random, never `legal[0]`.
    ///
    /// Measured, not assumed: an always-first-legal-move policy is an exact
    /// no-op cycle -- `EndAdditionalPhase`, the always-legal empty
    /// `PlaceThreeAreas { placements: [] }`, `ReturnPresidentToHand`
    /// recovering nothing, `EndAdditionalPhase` -- so no assistant is ever
    /// placed, the `AreaAction` phase never occurs, no card ever leaves the
    /// deck and no end trigger can approach. Both games below ran to turn
    /// 10001 and tripped `HONEST_MAX_TURNS`. Uniform-random is the fuzz
    /// driver's own policy, whose termination is evidenced across its 1500
    /// seeds (`tests/common/mod.rs:43`).
    fn pick_uniform(rng: &mut ChaCha8Rng, legal: &[Move]) -> Move {
        legal[rng.gen_range(0..legal.len())].clone()
    }

    #[test]
    fn the_driver_hands_each_agent_its_own_seat_view() {
        // The plumbing pin: a view for the wrong seat would silently give an
        // agent someone else's information.
        struct SeatChecker {
            seat: PlayerId,
            rng: ChaCha8Rng,
        }
        impl HonestAgent for SeatChecker {
            fn choose_starting_order(&mut self, seat: PlayerId, _d: &[OrderCardId; 2]) -> u8 {
                assert_eq!(seat, self.seat, "setup decision handed to the wrong seat");
                0
            }
            fn choose_move(&mut self, view: &PlayerView, l: &[Move], _r: &Rules) -> Move {
                assert_eq!(view.viewer, self.seat, "agent got another seat's view");
                assert_eq!(view.current_player, self.seat, "asked to move out of turn");
                pick_uniform(&mut self.rng, l)
            }
        }
        let rules = Rules::standard();
        run_honest_game(3, 3, &rules, &mut |seat, _n| {
            Box::new(SeatChecker { seat, rng: ChaCha8Rng::seed_from_u64(seat.0 as u64) })
        })
        .unwrap();
    }

    #[test]
    fn the_setup_decision_sees_only_its_own_two_cards() {
        // `PendingSetup` carries every seat's deal AND the seeded deck; an
        // honest agent must be handed only its own slice.
        use std::sync::{Arc, Mutex};
        /// Every setup decision the driver made, in the order it made them.
        type DealLog = Arc<Mutex<Vec<(PlayerId, [OrderCardId; 2])>>>;
        let seen: DealLog = Arc::new(Mutex::new(Vec::new()));
        struct Recorder {
            log: DealLog,
            rng: ChaCha8Rng,
        }
        impl HonestAgent for Recorder {
            fn choose_starting_order(&mut self, seat: PlayerId, deal: &[OrderCardId; 2]) -> u8 {
                self.log.lock().unwrap().push((seat, *deal));
                0
            }
            fn choose_move(&mut self, _v: &PlayerView, l: &[Move], _r: &Rules) -> Move {
                pick_uniform(&mut self.rng, l)
            }
        }
        let rules = Rules::standard();
        let log = Arc::clone(&seen);
        run_honest_game(9, 2, &rules, &mut |seat, _n| {
            Box::new(Recorder {
                log: Arc::clone(&log),
                rng: ChaCha8Rng::seed_from_u64(500 + seat.0 as u64),
            })
        })
        .unwrap();

        let entries = seen.lock().unwrap();
        assert_eq!(entries.len(), 2, "one setup decision per seat");
        // Each seat saw exactly 2 cards, and no seat saw another's deal.
        assert_ne!(entries[0].1, entries[1].1, "deals are disjoint");
    }

    #[test]
    fn the_honest_loop_shares_the_fuzz_drivers_termination_guard() {
        // `tests/common/mod.rs` cannot be imported from `src`, so this loop is
        // a deliberate sibling of the fuzz driver's. The constants that define
        // "stuck" must not drift apart; 3.2 folds the two together.
        assert_eq!(HONEST_MAX_TURNS, 10_000);
    }
}
