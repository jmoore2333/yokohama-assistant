//! Release-tier differential sweep skeleton (Phase 2.9, Stage 1).
//!
//! `#[ignore]`d: this joins the pre-merge RELEASE gates (CLAUDE.md two-tier
//! policy: release-profile, large-N, before merges/phase closes — never the
//! per-edit debug suite) ONCE the independence-protected oracle track lands
//! the real `yokohama_oracle::oracle_moves`. Run it with:
//!
//! `cargo test --release -p yokohama-difftest --test differential -- --ignored`
//!
//! EXPECTED TO FAIL if run manually today: the oracle is still the empty
//! Stage-1 stub, so every engine move keys as engine-only at step 0 and
//! every game "diverges" immediately. That is the harness working as
//! specified against an unimplemented oracle, not a defect — the assertion
//! below is the contract the finished oracle must meet.

use yokohama_core::Rules;
use yokohama_difftest::artifact::default_artifacts_dir;
use yokohama_difftest::drive::Policy;
use yokohama_difftest::runner::run_differential_game;

#[test]
#[ignore = "release gate once the oracle lands; fails against the empty oracle stub by design (see module docs)"]
fn differential_sweep_engine_vs_oracle() {
    let rules = Rules::standard();
    let artifact_dir = default_artifacts_dir();
    let mut divergences: Vec<String> = Vec::new();

    for policy in [Policy::Uniform, Policy::RetreatAverse] {
        for players in 2..=4u8 {
            for seed in 0..10u64 {
                let outcome = run_differential_game(seed, players, policy, &rules, &artifact_dir)
                    .unwrap_or_else(|e| panic!("seed {seed} {players}p {}: driver failure: {e}", policy.as_str()));
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

    assert!(
        divergences.is_empty(),
        "{} diverging games (artifacts written):\n{}",
        divergences.len(),
        divergences.join("\n")
    );
}
