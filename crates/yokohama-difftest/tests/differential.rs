//! Release-tier differential sweep (Phase 2.9, Stage 1): engine
//! `get_legal_moves` vs oracle `oracle_moves` at every decision point of
//! seeded self-play.
//!
//! `#[ignore]`d: this is a pre-merge RELEASE gate (CLAUDE.md two-tier
//! policy: release-profile, large-N, before merges/phase closes — never the
//! per-edit debug suite). Run it with:
//!
//! `cargo test --release -p yokohama-difftest --test differential -- --ignored`
//!
//! The oracle is fully implemented, so this must pass clean. A short driven
//! differential also runs in the per-edit tier (`per_edit_smoke.rs`) so the
//! rules are not left unguarded between merges.

use yokohama_core::Rules;
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::Policy;
use yokohama_difftest::runner::run_differential_game;

#[test]
#[ignore = "release gate (CLAUDE.md two-tier policy): large-N full games; run with --release before merges/phase closes"]
fn differential_sweep_engine_vs_oracle() {
    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut divergences: Vec<String> = Vec::new();
    let mut total_steps: u64 = 0;

    for policy in [Policy::Uniform, Policy::RetreatAverse] {
        for players in 2..=4u8 {
            for seed in 0..10u64 {
                let outcome = run_differential_game(seed, players, policy, &rules, &artifact_dir)
                    .unwrap_or_else(|e| panic!("seed {seed} {players}p {}: driver failure: {e}", policy.as_str()));
                total_steps += outcome.stats.steps;
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

    // Anti-vacuity (review §4.1): a game that is `is_game_over` at setup
    // returns `steps: 0, completed: true` and is compared ZERO times, so an
    // all-empty sweep would pass silently. Assert the sweep actually drove
    // and compared moves -- one comparison happens per applied step, so a
    // nonzero step total proves the comparison ran. (routes3 already carries
    // the sibling `qualifying_total > 0` assertion.)
    assert!(
        total_steps > 0,
        "sweep applied 0 moves across all games — vacuous pass (nothing was compared)"
    );

    assert!(
        divergences.is_empty(),
        "{} diverging games (artifacts written):\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}
