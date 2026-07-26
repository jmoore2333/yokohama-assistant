//! Seeded engine self-play driver, mirroring the engine's own fuzz drivers
//! (`crates/yokohama-core/tests/common/mod.rs`, used by both
//! `tests/fuzz_full_games.rs` and `examples/selfplay.rs`): identical
//! `new_game` -> `finalize_setup` construction (per-player
//! `StartingOrderChoice::Keep` drawn from one seeded `ChaCha8Rng`), stepping
//! by choosing from `get_legal_moves` and applying via
//! `apply_move_unchecked` (every move is drawn from the legal list by
//! construction), `check_invariants` after every apply and on the incoming
//! state, and the same turn cap of 10000.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use yokohama_core::{
    apply_move_unchecked, check_invariants, finalize_setup, get_legal_moves, is_game_over,
    new_game, GameConfig, GameState, Move, Rules, StartingOrderChoice,
};

/// Turn cap (matches the engine fuzz drivers' `MAX_TURNS`): exceeding this
/// without `is_game_over` is treated as non-termination, not looped on.
pub const MAX_TURNS: u32 = 10_000;

/// Move-selection policy for self-play — **since Phase 3.2 Task 5 this is
/// `yokohama_core::RolloutPolicy`, re-exported**, not a local enum.
///
/// The two rungs this crate uses keep their exact behaviour and their exact
/// names: `as_str` still yields `"uniform"` / `"retreat-averse"`, which three
/// checked-in artifacts under `fixtures/` carry in both filename and payload, so
/// every recorded figure and every repro path is unchanged. What moved is
/// OWNERSHIP: the arena needs the same rungs, and two enums that must agree
/// forever is the shape a fidelity pin exists to compensate for. Now there is
/// one enum and the pin below checks the two IMPLEMENTATIONS instead — this
/// crate's list-based [`pick_move`] against core's handle-based
/// `RolloutPolicy::pick`.
///
/// `RetreatAverse` differs from `Uniform` ONLY in never VOLUNTEERING
/// retreats, implementing the external probe's policy from guidingpath.md's
/// 2026-07-20 sizing-correction row (Phase 3 section). Exact rule: at each
/// decision point, let F be the subset of legal moves that are NOT
/// `Move::ReturnPresidentToHand` with a nonzero total recovery count
/// (`recover.values().sum() > 0`). If F is non-empty, choose uniformly from
/// F; otherwise (nothing else is legal) fall back to choosing uniformly
/// from the full legal list. A `ReturnPresidentToHand` with an empty or
/// all-zero `recover` map recovers nothing and counts as a non-retreat —
/// the excluded family is exactly the Movement-Option-B count-vector
/// product that uniform-random play over-selects.
pub use yokohama_core::RolloutPolicy as Policy;

/// Parse a policy name from THIS crate's CLI, which accepts **exactly two**:
/// `uniform` and `retreat-averse` (plus the `retreat_averse` alias it has
/// always taken). Byte-identical accept set and error text to the `FromStr` it
/// replaced.
///
/// Deliberately NOT `Policy::from_str`, which core widened to three names: the
/// third rung (`family-weighted`) has no list-based implementation, and
/// [`pick_move`]'s arm for it is `unimplemented!`. Keeping the CLI's accept set
/// at two is what makes that arm unreachable from any subcommand rather than a
/// panic a user can trigger — and it keeps every sweep, stream and recorded
/// figure in this crate exactly reproducible.
pub fn parse_cli_policy(s: &str) -> Result<Policy, String> {
    match Policy::from_str_name(s) {
        Some(p @ (Policy::Uniform | Policy::RetreatAverse)) => Ok(p),
        _ => Err(format!("unknown policy {s:?} (expected uniform | retreat-averse)")),
    }
}

/// True for the moves `Policy::RetreatAverse` refuses to volunteer: a
/// Movement-Option-B recovery (`ReturnPresidentToHand`) that actually
/// recovers at least one assistant.
pub fn is_volunteered_retreat(m: &Move) -> bool {
    matches!(m, Move::ReturnPresidentToHand { recover } if recover.values().any(|&n| n > 0))
}

/// Pick one move from `moves` (must be non-empty) under `policy`.
///
/// LIST-BASED by design: this driver already holds the materialized
/// `get_legal_moves` output (the differential comparison is against it), so
/// there is nothing for a factored source to save here. Core's
/// `RolloutPolicy::pick` is the handle-based twin, and
/// [`tests::core_retreat_averse_agrees_with_pick_move_move_for_move_and_draw_for_draw`]
/// pins the two together.
pub fn pick_move<'a>(moves: &'a [Move], policy: Policy, rng: &mut ChaCha8Rng) -> &'a Move {
    match policy {
        Policy::Uniform => &moves[rng.gen_range(0..moves.len())],
        Policy::RetreatAverse => {
            let non_retreat: Vec<&Move> =
                moves.iter().filter(|m| !is_volunteered_retreat(m)).collect();
            if non_retreat.is_empty() {
                // Nothing but retreats is legal: fall back to the full list.
                &moves[rng.gen_range(0..moves.len())]
            } else {
                non_retreat[rng.gen_range(0..non_retreat.len())]
            }
        }
        // DEFENDED-UNREACHABLE, not unimplemented-by-omission. The weighted rung
        // is defined over move FAMILIES (a routes-vs-retreat scalar plus a
        // per-area recovery draw); a flat list of moves has no family structure
        // to weight, so there is no faithful list-based reading of it — the
        // arena is where it belongs. [`parse_cli_policy`] is this crate's only
        // way to name a policy from outside and it accepts exactly the two
        // above, so no subcommand can route here.
        //
        // (Deviation from spec §2, which said "typed error": every `pick_move`
        // caller takes `&Move` through fourteen call sites that would all grow a
        // `Result` for an arm none of them can reach. Recorded in the Task-5
        // commit body and folded into the spec at close-out.)
        Policy::FamilyWeighted { .. } => unimplemented!(
            "family-weighted is not supported by the list-based difftest driver; use \
             yokohama-core's arena"
        ),
    }
}

/// Summary of one driven game.
#[derive(Debug, Clone)]
pub struct DriveStats {
    pub seed: u64,
    pub players: u8,
    pub policy: Policy,
    /// Decision points visited (moves applied).
    pub steps: u64,
    /// `turn_number` of the last state seen.
    pub turns: u32,
    /// True when the game reached `is_game_over` (false when stopped by the
    /// observer or by `max_steps`).
    pub completed: bool,
}

/// Observer verdict per decision point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    /// Keep playing.
    Continue,
    /// Stop the drive cleanly (e.g. a divergence artifact was written).
    Stop,
}

/// Build a fresh seeded post-setup game exactly the way the engine fuzz
/// drivers do: per-player `StartingOrderChoice::Keep(rng.gen_range(0..2))`
/// from a `ChaCha8Rng` seeded with `seed`. Returns the RNG in its
/// post-setup state so the caller keeps drawing from the same stream.
pub fn new_seeded_game(seed: u64, players: u8, rules: &Rules) -> Result<(GameState, ChaCha8Rng), String> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let config = GameConfig::for_player_count(players);
    let pending = new_game(config, rules, seed)
        .map_err(|e| format!("seed={seed} players={players}p: new_game failed: {e}"))?;
    let choices: Vec<StartingOrderChoice> =
        (0..players).map(|_| StartingOrderChoice::Keep(rng.gen_range(0..2u8))).collect();
    let state = finalize_setup(pending, &choices)
        .map_err(|e| format!("seed={seed} players={players}p: finalize_setup failed: {e}"))?;
    Ok((state, rng))
}

/// Drive one seeded self-play game under `policy`, invoking
/// `observer(state, legal_moves, step_index)` at every decision point
/// BEFORE a move is picked (this is where the differential comparison
/// plugs in). Stops on: game over, observer `Stop`, `max_steps` decision
/// points, or a hard failure (`Err`): empty legal list in a non-terminal
/// state, `apply_move_unchecked` rejecting a generated move, an invariant
/// violation (checked on the starting state and after every apply, like the
/// engine fuzz drivers), observer error, or exceeding [`MAX_TURNS`].
pub fn drive_game(
    seed: u64,
    players: u8,
    policy: Policy,
    rules: &Rules,
    max_steps: Option<u64>,
    observer: impl FnMut(&GameState, &[Move], u64) -> Result<StepOutcome, String>,
) -> Result<DriveStats, String> {
    drive_game_to_end(seed, players, policy, rules, max_steps, observer).map(|(stats, _)| stats)
}

/// [`drive_game`], but also returning the LAST state reached — the terminal
/// state when `stats.completed` is true (this is what the Stage-2 scoring
/// differential compares on), or the state at the stop point for an
/// observer `Stop` / `max_steps` cutoff. Same loop, same failure contract;
/// `drive_game` is a thin wrapper that drops the state.
pub fn drive_game_to_end(
    seed: u64,
    players: u8,
    policy: Policy,
    rules: &Rules,
    max_steps: Option<u64>,
    mut observer: impl FnMut(&GameState, &[Move], u64) -> Result<StepOutcome, String>,
) -> Result<(DriveStats, GameState), String> {
    let (mut state, mut rng) = new_seeded_game(seed, players, rules)?;

    if let Err(reason) = check_invariants(&state, rules) {
        return Err(fail(seed, players, &state, &format!("corrupt starting state: {reason}")));
    }

    let mut steps: u64 = 0;
    loop {
        if is_game_over(&state) {
            let stats =
                DriveStats { seed, players, policy, steps, turns: state.turn_number, completed: true };
            return Ok((stats, state));
        }
        if state.turn_number > MAX_TURNS {
            return Err(fail(
                seed,
                players,
                &state,
                "non-termination: turn_number exceeded MAX_TURNS without is_game_over",
            ));
        }
        if max_steps.is_some_and(|cap| steps >= cap) {
            let stats =
                DriveStats { seed, players, policy, steps, turns: state.turn_number, completed: false };
            return Ok((stats, state));
        }

        let moves = get_legal_moves(&state, rules);
        if moves.is_empty() {
            return Err(fail(
                seed,
                players,
                &state,
                "dead end: get_legal_moves returned empty in a non-terminal state",
            ));
        }

        match observer(&state, &moves, steps)? {
            StepOutcome::Stop => {
                let stats = DriveStats {
                    seed,
                    players,
                    policy,
                    steps,
                    turns: state.turn_number,
                    completed: false,
                };
                return Ok((stats, state));
            }
            StepOutcome::Continue => {}
        }

        let mv = pick_move(&moves, policy, &mut rng).clone();
        // apply_move_unchecked: mv is drawn from get_legal_moves by
        // construction (same rationale as the engine fuzz drivers).
        match apply_move_unchecked(&state, &mv, rules) {
            Ok(next) => {
                if let Err(reason) = check_invariants(&next, rules) {
                    return Err(fail(
                        seed,
                        players,
                        &next,
                        &format!("invariant violated after applying {mv:?}: {reason}"),
                    ));
                }
                state = next;
                steps += 1;
            }
            Err(e) => {
                return Err(fail(
                    seed,
                    players,
                    &state,
                    &format!("apply_move_unchecked rejected a generated move {mv:?}: {e}"),
                ));
            }
        }
    }
}

/// Failure report: header plus the full state JSON (the state is
/// self-contained and resumable, so this is the whole repro).
fn fail(seed: u64, players: u8, state: &GameState, reason: &str) -> String {
    let state_json = serde_json::to_string_pretty(state)
        .unwrap_or_else(|e| format!("<state JSON serialization failed: {e}>"));
    format!(
        "seed={seed} players={players}p turn={} phase={:?}\nreason: {reason}\nstate JSON:\n{state_json}",
        state.turn_number, state.turn_phase,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use yokohama_core::AreaId;

    fn retreat(counts: &[(u8, u8)]) -> Move {
        let recover: BTreeMap<AreaId, u8> =
            counts.iter().map(|&(a, n)| (AreaId::new(a), n)).collect();
        Move::ReturnPresidentToHand { recover }
    }

    #[test]
    fn retreat_classification_matches_the_documented_rule() {
        assert!(is_volunteered_retreat(&retreat(&[(1, 2), (3, 0)])));
        // Recovering nothing is NOT a volunteered retreat...
        assert!(!is_volunteered_retreat(&retreat(&[])));
        assert!(!is_volunteered_retreat(&retreat(&[(1, 0)])));
        // ...and no other family ever is.
        assert!(!is_volunteered_retreat(&Move::PlaceThreeAreas { placements: vec![] }));
    }

    #[test]
    fn retreat_averse_never_picks_a_nonzero_retreat_when_alternatives_exist() {
        let moves = vec![retreat(&[(1, 1)]), Move::PlaceThreeAreas { placements: vec![] }, retreat(&[(2, 2)])];
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        for _ in 0..100 {
            let picked = pick_move(&moves, Policy::RetreatAverse, &mut rng);
            assert!(!is_volunteered_retreat(picked), "picked a volunteered retreat: {picked:?}");
        }
    }

    #[test]
    fn retreat_averse_falls_back_to_full_list_when_only_retreats_are_legal() {
        let moves = vec![retreat(&[(1, 1)]), retreat(&[(2, 2)])];
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let picked = pick_move(&moves, Policy::RetreatAverse, &mut rng);
        assert!(moves.contains(picked));
    }

    /// This crate's CLI accept set is EXACTLY two names, unchanged by the move
    /// of the enum into core — including the error text, which the usage strings
    /// in `main.rs` mirror.
    ///
    /// The `family-weighted` case is the load-bearing one: core parses it, this
    /// parser must not, and that is precisely what keeps `pick_move`'s
    /// `unimplemented!` arm unreachable from every subcommand.
    #[test]
    fn the_cli_parser_accepts_exactly_the_two_list_based_policies() {
        assert_eq!(parse_cli_policy("uniform").unwrap(), Policy::Uniform);
        assert_eq!(parse_cli_policy("retreat-averse").unwrap(), Policy::RetreatAverse);
        assert_eq!(parse_cli_policy("retreat_averse").unwrap(), Policy::RetreatAverse);

        assert_eq!(
            parse_cli_policy("greedy"),
            Err("unknown policy \"greedy\" (expected uniform | retreat-averse)".to_string()),
        );
        // Core knows the third rung; this CLI deliberately does not.
        assert!(Policy::from_str_name("family-weighted").is_some());
        assert!(parse_cli_policy("family-weighted").is_err());
    }

    /// The artifact-compatibility half of the re-export: three checked-in
    /// fixtures carry these names in their filenames AND their payloads, so a
    /// change here silently orphans them.
    #[test]
    fn policy_names_stay_byte_identical_for_artifact_compatibility() {
        assert_eq!(Policy::Uniform.as_str(), "uniform");
        assert_eq!(Policy::RetreatAverse.as_str(), "retreat-averse");
    }

    /// **THE FIDELITY PIN** (spec §2; T2 review probe 3). Core's handle-based
    /// `RolloutPolicy::RetreatAverse` and this crate's list-based [`pick_move`],
    /// given the SAME state, the SAME legal list and the SAME ChaCha8 stream,
    /// select the SAME move and leave the RNG at the SAME position.
    ///
    /// Position, not just the move: two implementations that agree on every pick
    /// while consuming different amounts of randomness would diverge the moment
    /// either one drives a game, and E2's recorded retreat-averse figures were
    /// measured with THIS implementation. `get_word_pos()` is the direct
    /// observation of that, which is why the assertion is on it rather than on a
    /// subsequent draw.
    ///
    /// ## The Placement partition, and why it is not a weakened claim
    ///
    /// At a Placement decision point the two CANNOT share a stream and the
    /// difference is structural, not incidental: `pick_move` indexes the
    /// materialized list with one draw, while core reaches placement through
    /// `placement_sample`'s reservoir, which spends one draw PER candidate
    /// exactly so the family is never materialized. Both are uniform over the
    /// realized moves; they are not the same draw, and making them one would
    /// mean materializing what the factored source exists not to materialize.
    /// So Placement points are asserted on what IS true of them (a legal
    /// placement move from the same list) and counted separately, and the
    /// identity claim is made — in full, on both halves — everywhere else.
    ///
    /// Both partitions carry a floor, so neither can quietly empty out.
    ///
    /// Also pinned here, because the index mapping above depends on it: the
    /// handle's per-family serving order is ELEMENTWISE `iter_family`'s. Core
    /// maps one drawn index onto the routes family while this crate maps the
    /// same index onto the legal list's non-retreat prefix; if those orders ever
    /// diverged the two would still agree in DISTRIBUTION and silently disagree
    /// on the move. Families above [`ORDER_COMPARE_CAP`] moves are skipped and
    /// counted — the retreat family reaches six figures, and materializing it to
    /// compare orders is the one thing this whole design forbids.
    ///
    /// MEASURED at HEAD (debug, seed 7, 4P, 600 decision points): 502
    /// non-Placement points, every one identical in move AND word position; 98
    /// Placement points; 698 elementwise order comparisons made, 0 skipped (no
    /// organic family on this game exceeded the cap).
    ///
    /// MUTATION-PROVEN 2026-07-26, both halves separately, both in core's
    /// `RolloutPolicy::pick` and both reverted by inverse edit (L7):
    ///
    /// - MOVE half — reversing the routes index mapping
    ///   (`handle.iter(routes).nth(routes_len - 1 - i)`), which leaves the DRAW
    ///   and therefore the whole distribution untouched: FAILED at step 2 with
    ///   "the two implementations picked different moves from one stream";
    /// - POSITION half — one extra `rng.gen::<u32>()` after the index draw,
    ///   which leaves every selected MOVE identical: FAILED at step 2 with "same
    ///   move, different stream consumption", word position 2 against 3.
    ///
    /// Neither mutation is caught by the other's assertion, which is why the pin
    /// asserts both rather than treating the move as evidence of the stream.
    #[test]
    fn core_retreat_averse_agrees_with_pick_move_move_for_move_and_draw_for_draw() {
        use yokohama_core::{iter_family, DecisionHandle, MoveFamily};

        /// Families larger than this are not order-compared: the retreat product
        /// reaches 442,368 and collecting it is exactly what the factored source
        /// exists to avoid.
        const ORDER_COMPARE_CAP: u64 = 4_096;

        let rules = Rules::standard();
        let (mut identical, mut placement, mut compared, mut skipped) = (0u32, 0u32, 0u32, 0u32);

        drive_game(7, 4, Policy::RetreatAverse, &rules, Some(600), |state, moves, step| {
            // ONE stream, two readers: `probe_b` is a clone taken before either
            // implementation draws, so "the same stream" is literal.
            let mut probe_a = ChaCha8Rng::seed_from_u64(0xD1FF_0000_0000_0000 ^ step);
            let mut probe_b = probe_a.clone();

            let listwise = pick_move(moves, Policy::RetreatAverse, &mut probe_a).clone();
            let handle = DecisionHandle::new(state, &rules);
            let factored = Policy::RetreatAverse
                .pick(&handle, &mut probe_b)
                .expect("a decision point yields a move");

            for family in handle.families() {
                if handle.size(family).value() > ORDER_COMPARE_CAP {
                    skipped += 1;
                    continue;
                }
                let from_handle: Vec<Move> = handle.iter(family).collect();
                let from_free: Vec<Move> = iter_family(state, &rules, family).collect();
                assert_eq!(
                    from_handle, from_free,
                    "step {step}: handle.iter({family:?}) is not elementwise iter_family's",
                );
                compared += 1;
            }

            if matches!(handle.families(), [MoveFamily::Placement]) {
                placement += 1;
                assert!(
                    moves.contains(&factored),
                    "step {step}: the reservoir returned a move outside the legal list",
                );
            } else {
                assert_eq!(
                    factored, listwise,
                    "step {step}: the two implementations picked different moves from one stream",
                );
                assert_eq!(
                    probe_a.get_word_pos(),
                    probe_b.get_word_pos(),
                    "step {step}: same move, different stream consumption ({listwise:?})",
                );
                identical += 1;
            }
            Ok(StepOutcome::Continue)
        })
        .expect("the driven game must not fail");

        println!(
            "retreat-averse fidelity: {identical} non-Placement points identical in move and word \
             position, {placement} Placement points (reservoir, uniform-equivalent by \
             construction); {compared} family orders compared, {skipped} over the {ORDER_COMPARE_CAP} cap",
        );
        // Floors: half the measured counts, so neither partition can empty out
        // and leave the other proving the whole claim on its own.
        assert!(identical >= 250, "only {identical} non-Placement identity points were reached");
        assert!(placement >= 45, "only {placement} Placement points were reached");
        assert!(compared >= 349, "only {compared} family orders were compared");
    }
}
