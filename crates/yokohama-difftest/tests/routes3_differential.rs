//! Stage-3 three-way route sweep (Phase 2.9).
//!
//! The `#[ignore]`d sweep joins the pre-merge RELEASE gates (CLAUDE.md
//! two-tier policy — release-profile, large-N, before merges/phase closes):
//!
//! `cargo test --release -p yokohama-difftest --test routes3_differential -- --ignored`
//!
//! It drives seeded self-play under BOTH policies × 3 player counts × 8
//! seeds each and runs the three-way check (engine walk vs pruned-oracle
//! walk vs pruneless reference) at EVERY qualifying decision point
//! (Movement, empty queue, on-board president); zero divergences and a
//! non-zero qualifying count are the pass condition.
//!
//! The NON-ignored plumbing test below is the census discipline for that
//! sweep: it proves a short driven game actually HITS a qualifying state, so
//! the sweep can never vacuously pass by never qualifying.

use yokohama_core::Rules;
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::{drive_game, Policy, StepOutcome};
use yokohama_difftest::routes3::{run_routes3_game, three_way_route_check_with_moves};

#[test]
#[ignore = "release gate (two-tier policy): 48 full games with per-qualifying-step three-way enumeration"]
fn routes3_sweep_three_way_clean_across_policies_counts_seeds() {
    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut divergences: Vec<String> = Vec::new();
    let mut qualifying_total: u64 = 0;
    let mut max_labels: usize = 0;

    for policy in [Policy::Uniform, Policy::RetreatAverse] {
        for players in 2..=4u8 {
            for seed in 0..8u64 {
                let outcome = run_routes3_game(seed, players, policy, &rules, &artifact_dir)
                    .unwrap_or_else(|e| {
                        panic!("seed {seed} {players}p {}: driver failure: {e}", policy.as_str())
                    });
                qualifying_total += outcome.qualifying_steps;
                max_labels = max_labels.max(outcome.max_labels);
                if let Some(path) = outcome.divergence {
                    divergences.push(format!(
                        "seed {seed} {players}p {} step {} -> {}",
                        policy.as_str(),
                        outcome.stats.steps,
                        path.display()
                    ));
                }
            }
        }
    }

    println!(
        "routes3 sweep: {qualifying_total} qualifying decision points, organic max {max_labels} labels"
    );
    assert!(
        qualifying_total > 0,
        "sweep never reached a qualifying Movement state — vacuous pass (census discipline)"
    );
    assert!(
        divergences.is_empty(),
        "{} three-way diverging games (artifacts written):\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}

/// Census discipline (NOT ignored): a short driven game must actually
/// produce at least one qualifying state, and every qualifying state on the
/// slice must compare clean. Retreat-averse play places the President
/// quickly (it never volunteers the stay-in-hand retreat when a placement is
/// legal), so on-board Movement states appear within a short slice.
#[test]
fn routes3_plumbing_short_game_hits_qualifying_states() {
    let rules = Rules::standard();
    let mut qualifying: u64 = 0;
    drive_game(0, 2, Policy::RetreatAverse, &rules, Some(200), |state, moves, step| {
        if let Some(cmp) = three_way_route_check_with_moves(state, &rules, moves) {
            qualifying += 1;
            assert!(
                !cmp.is_divergent(),
                "step {step}: three-way divergence on the plumbing slice: {:#?}",
                cmp.diverged
            );
        }
        Ok(StepOutcome::Continue)
    })
    .expect("short retreat-averse game drives cleanly");
    assert!(
        qualifying >= 1,
        "200 steps of 2p retreat-averse play must hit at least one qualifying Movement state"
    );
}
