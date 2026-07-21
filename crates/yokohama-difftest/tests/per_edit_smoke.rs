//! Per-edit differential smoke (Phase-2.9 review addendum, cause 3): the
//! full engine-vs-oracle enumeration comparison on a handful of SHORT driven
//! games, run in the ordinary (debug, non-`#[ignore]`d) tier.
//!
//! Motivation: every other engine-vs-oracle comparison lives behind a
//! `#[ignore]` release gate, so the Tier-2 rules the differential guards were
//! left unchecked BETWEEN merges. This closes that gap cheaply: a couple of
//! seeds x ~300 steps x {2p, 3p} x both policies, comparing `get_legal_moves`
//! against `oracle_moves` at every visited state. It is deliberately kept
//! small enough to run in the debug suite (measured well under a second).

use yokohama_core::Rules;
use yokohama_difftest::compare::compare_moves;
use yokohama_difftest::drive::{drive_game, Policy, StepOutcome};

#[test]
fn per_edit_differential_smoke_short_games_clean() {
    let rules = Rules::standard();
    // (seed, players, policy) — two seeds, both 2p and 3p, both policies.
    let configs = [
        (0u64, 2u8, Policy::Uniform),
        (0, 2, Policy::RetreatAverse),
        (1, 3, Policy::Uniform),
        (1, 3, Policy::RetreatAverse),
    ];
    let mut comparisons: u64 = 0;

    for (seed, players, policy) in configs {
        drive_game(seed, players, policy, &rules, Some(300), |state, engine_moves, step| {
            let oracle = yokohama_oracle::oracle_moves(state, &rules);
            let cmp = compare_moves(state, &rules, engine_moves, &oracle);
            comparisons += 1;
            assert!(
                !cmp.is_divergent(),
                "seed {seed} {players}p {} step {step}: engine-vs-oracle divergence:\n  \
                 engine_only={:?}\n  oracle_only={:?}\n  duplicates={:?}",
                policy.as_str(),
                cmp.engine_only,
                cmp.oracle_only,
                cmp.duplicates
            );
            Ok(StepOutcome::Continue)
        })
        .unwrap_or_else(|e| panic!("seed {seed} {players}p {}: driver failure: {e}", policy.as_str()));
    }

    // Non-vacuity: the smoke must have actually compared states.
    assert!(comparisons > 0, "per-edit differential smoke compared nothing");
}
