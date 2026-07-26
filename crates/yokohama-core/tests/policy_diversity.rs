//! The policy-diversity gate — Phase 3.2, spec
//! `docs/plans/2026-07-25-phase3.2-arena-spec.md` §7 and §12 gate 2, governed
//! by §0-R1 and §6.
//!
//! WHAT THIS BINARY IS FOR. Spec §7 lands the map's promise that "the
//! family-weighted policy is still exercised at 3-4P from 3.2 on — by the
//! fuzzer". It ADDS weighted cells; it replaces nothing. The existing
//! `fuzz_full_games` uniform corpus stays exactly where it is, because the two
//! policies cover DISJOINT halves of the apply surface (§7, measured): retreat
//! aversion applies zero nonzero-recovery retreats while uniform under-covers
//! area actions ~4x and builds 13-20x, so substituting one for the other is a
//! coverage regression against Phase 2.9's hardened arms, not an upgrade.
//!
//! THE FOUR CELLS, and what each is for:
//!
//!   * `weighted_cells_complete_with_invariants` — the fuzz-driver contract:
//!     full games under the shipped weighted policy complete at 2/3/4P with
//!     `check_invariants` after every applied move.
//!   * `four_p_trigger_gate_frozen_floor` — §0-R1's restated trigger gate. The
//!     map's original text asked for "ChurchTrackFull > 0%", which does not
//!     discriminate: uniform itself satisfies it (1/500 at one harness, 7/2000
//!     at another). The restatement gates the weighted 4P rate's Wilson LOWER
//!     bound against a FROZEN uniform ceiling — see
//!     [`UNIFORM_4P_CHURCH_CEILING`].
//!   * `coverage_floors_hold` — §7(c)'s applied-move coverage floors, whose
//!     VALUES were measured before they were set (see [`Floors`]).
//!   * `tails_are_recorded_per_phase` — §6's volume tails, RECORDED per
//!     (player count x phase). §0-R2 is explicit that these are an
//!     OBSERVATION and not a bound: no shape in E1's 28-shape screen that
//!     measurably fixed the 4P trigger collapse kept tails below six figures,
//!     the mechanism explains why (on-board assistant density is simultaneously
//!     what makes play score-productive and what inflates the retreat product),
//!     and no bound is claimed or known. This cell therefore asserts
//!     non-emptiness and prints the distribution; the honest cost bound that
//!     replaced "tails bounded" lives in `action_source_worstcase.rs`.
//!
//! EVERY CELL RUNS ON THE COMMITTED DIAGNOSTIC MASTER [`MASTER`]. That is spec
//! §3's gate-cell exemption: a regression test wants reproducibility, which is
//! exactly what `SeedSchedule::diagnostic`'s NAME declares, and an OS-entropy
//! gate would flake at >= alpha by construction. The OS-entropy rule binds
//! published, bot-facing evaluation runs — `examples/arena.rs` is where those
//! live.
//!
//! NO FRESH UNIFORM COMPARATOR CELL RUNS HERE, deliberately (§7, §0-R1): the
//! trigger gate is a frozen-baseline design, and a two-sided disjoint-intervals
//! design against a fresh uniform comparator was REJECTED by simulation (it
//! flakes 9.1-9.3% of runs at N=500 and needs N >= 750-1000 per cell, whose
//! comparator alone costs 28-56s). A fresh uniform 4P trigger rate is still
//! RECORDED at every close, by `examples/arena.rs ab` — a drift watch on the
//! frozen constant, never the assert's comparator.
//!
//! THREADING: this binary does NOT need `--test-threads=1`. Its four gate tests
//! share one memoized cell set ([`cells`]) which is written once, under a
//! `OnceLock`, by whichever test reaches it first and read immutably by the
//! rest; the computation is a pure function of (master, scale), so which thread
//! runs it cannot change the figures. Each test formats its report block into
//! ONE string and prints it with one `println!`, so blocks stay legible under
//! `--nocapture` without serializing the run.
//!
//! TIER: release, `cargo test --release -p yokohama-core --test policy_diversity
//! -- --ignored`. The per-edit slice is `the_weighted_cells_run_at_smoke_scale`,
//! which is not `#[ignore]`d — a release-only binary rots invisibly.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use yokohama_core::{
    run_raw_game, trigger_name, EndGameTrigger, QuantileSketch, RolloutPolicy, Rules, SeedSchedule,
    TriggerTally, TurnPhase, WilsonInterval,
};

/// This binary's committed diagnostic master (plan Task 10).
///
/// It is used for BOTH roles a raw-state cell needs — the world-seed schedule
/// and the per-run `rng_master` that keys the seat policy streams — which is
/// safe because `run_raw_game` folds the world seed into every seat stream
/// (`mix(SALT_RAW_STREAM, &[seed.0, rng_master, seat])`, the T6 review's F1
/// fix). Holding `rng_master` fixed across the games of a cell therefore does
/// NOT hand seat i the same stream in every game, and one committed constant
/// reproduces the whole run.
const MASTER: u64 = 0xD17E_2026_0726_0002;

/// The shipped policy under test at every seat of every cell.
const POLICY: RolloutPolicy = RolloutPolicy::FAMILY_WEIGHTED_DEFAULT;

/// Per-move `check_invariants` — ON, the fuzz-driver contract §7 asks these
/// cells to keep. Named rather than inlined so the one place it could be
/// weakened is visible.
const INVARIANTS: bool = true;

/// The player counts §7 requires weighted cells at.
const PLAYER_COUNTS: [u8; 3] = [2, 3, 4];

/// The trigger whose 4P rate §0-R1's gate is written on.
const CHURCH: &str = trigger_name(&EndGameTrigger::ChurchTrackFull);

// FROZEN uniform 4P ChurchTrackFull upper bound: 7/2000, Wilson95 hi 0.72% — E1
// held-out master 16045690984833335230, N=2000, release, raw-state screening
// driver (spec §0-R1). Flake of this one-sided gate at N=500 ≈ 0.2% at the
// held-out true rates. Drift-watched by examples/arena ab's fresh uniform rate
// at every close.
const UNIFORM_4P_CHURCH_CEILING: f64 = 0.0072;

// ---------------------------------------------------------------------------
// Scale and floors
// ---------------------------------------------------------------------------

/// Games per player-count cell, at each tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Scale {
    games: u32,
}

impl Scale {
    /// The shipped release tier. N=500 is §0-R1's sizing: at the held-out true
    /// rates it puts the weighted 4P Wilson lower bound near 2.4% against the
    /// 0.72% ceiling, for a computed failure probability of about 0.2%.
    const GATE: Self = Self { games: 500 };

    /// Development / per-edit plumbing size (plan Task 10: N=40).
    const SMOKE: Self = Self { games: 40 };
}

/// The applied-move coverage shares §7(c) gates on.
///
/// SET BY MEASUREMENT, NEVER INVENTED (spec §1 critic disposition #6). The
/// procedure, executed in this task and reproducible from this binary's own
/// output: run the three [`Scale::GATE`] cells, read each share off the pooled
/// [`CoverageTally`](yokohama_core::CoverageTally) accessors, take the MINIMUM
/// across the three player counts, halve it, and round DOWN to one significant
/// figure. One floor per metric rather than one per count: the minimum is
/// already the binding cell, and a floor per count is three times the surface
/// to re-measure for no extra catch.
///
/// The measured shares that produced these values are recorded on each field.
/// They are a regression net at roughly half the observed value, not a claim
/// about the policy — a share drifting down by a factor of two is a policy or
/// engine change worth failing on, and this is the tier that would see it.
#[derive(Clone, Copy, Debug)]
struct Floors {
    nonzero_recovery: f64,
    area_action: f64,
    build: f64,
}

impl Floors {
    /// The release tier's floors. See the type doc for the setting rule.
    ///
    /// Measured at this HEAD, release, master [`MASTER`], N=500/cell, the
    /// shipped weighted defaults — the run these values were read off is the
    /// same one `coverage_floors_hold` reproduces on every gate run.
    const GATE: Self = Self {
        // Measured 2P 0.0121 · 3P 0.0131 · 4P 0.0143. Min 0.0121 (2P), half
        // 0.00605, rounded down to one significant figure.
        nonzero_recovery: 0.006,
        // Measured 2P 0.1211 · 3P 0.1193 · 4P 0.1169. Min 0.1169 (4P), half
        // 0.05845, rounded down to one significant figure.
        area_action: 0.05,
        // Measured 2P 0.0087 · 3P 0.0059 · 4P 0.0041. Min 0.0041 (4P), half
        // 0.00205, rounded down to one significant figure. The tightest of the
        // three, and the one a regression would reach first: builds are the
        // arm uniform play under-covers 13-20x, which is why §7 gates them at
        // all.
        build: 0.002,
    };

    /// The smoke tier's floors: the gate's, halved again (plan Task 10). N=40
    /// per cell is a small enough sample that a share can wander well below its
    /// N=500 value without anything being wrong, and the smoke's subject is the
    /// plumbing, not the distribution.
    const SMOKE: Self = Self {
        nonzero_recovery: Self::GATE.nonzero_recovery / 2.0,
        area_action: Self::GATE.area_action / 2.0,
        build: Self::GATE.build / 2.0,
    };
}

// ---------------------------------------------------------------------------
// Cells
// ---------------------------------------------------------------------------

/// One (player count x N) weighted cell, folded down as it runs.
///
/// The per-game [`ArenaGameStats`](yokohama_core::ArenaGameStats) is DROPPED
/// after folding: it carries a whole `GameRecord` and a per-decision-point
/// volume vector, and retaining 500 of them per cell would cost far more than
/// the figures are worth. Records are not emitted at all here — §7's cells are
/// the fuzzer's shape, and the corpus is `arena_gate`'s business.
struct Cell {
    players: u8,
    games_requested: u32,
    trigger: TriggerTally,
    applied: u64,
    retreat_picks: u64,
    nonzero_recovery_retreats: u64,
    area_actions: u64,
    builds: u64,
    decision_points: u64,
    tails: Vec<PhaseTail>,
    /// Games that did not complete, or completed without a recorded trigger.
    /// Collected rather than panicked so `weighted_cells_complete_with_invariants`
    /// is the test that reports them, and so a failure does not have to be
    /// re-run once per test in the shared cell set.
    failures: Vec<String>,
    elapsed: Duration,
}

/// One phase's volume tail within a cell.
struct PhaseTail {
    phase: TurnPhase,
    sketch: QuantileSketch,
    /// Whether ANY observation in this row carried an upper-bounded (placement)
    /// term. The caveat rides the measured flag rather than a `phase ==
    /// Placement` test, so the row says what is true of the observations rather
    /// than what the reader assumed about the phase.
    any_upper_bound: bool,
}

impl Cell {
    fn share(&self, count: u64) -> f64 {
        if self.applied == 0 {
            0.0
        } else {
            count as f64 / self.applied as f64
        }
    }

    fn nonzero_recovery_share(&self) -> f64 {
        self.share(self.nonzero_recovery_retreats)
    }

    fn area_action_share(&self) -> f64 {
        self.share(self.area_actions)
    }

    fn build_share(&self) -> f64 {
        self.share(self.builds)
    }

    fn retreat_pick_share(&self) -> f64 {
        self.share(self.retreat_picks)
    }

    fn church(&self) -> (u64, WilsonInterval) {
        self.trigger.rate(CHURCH)
    }

    /// The tails in `TurnPhase` execution order.
    fn ordered_tails(&self) -> Vec<&PhaseTail> {
        let mut rows: Vec<&PhaseTail> = self.tails.iter().collect();
        rows.sort_by_key(|row| phase_order(row.phase));
        rows
    }

    fn observe_volume(&mut self, phase: TurnPhase, total: u64, upper_bound: bool) {
        let row = match self.tails.iter_mut().position(|row| row.phase == phase) {
            Some(index) => &mut self.tails[index],
            None => {
                self.tails.push(PhaseTail {
                    phase,
                    sketch: QuantileSketch::default(),
                    any_upper_bound: false,
                });
                self.tails.last_mut().expect("just pushed")
            }
        };
        row.sketch.observe(total);
        row.any_upper_bound |= upper_bound;
    }
}

/// Report order for a phase row.
///
/// Exhaustive with no `_` arm, the repo's convention for a report key
/// (`trigger_name`, `is_move_legal`): a new `TurnPhase` variant is a compile
/// error here rather than a row that silently sorts to an arbitrary position.
fn phase_order(phase: TurnPhase) -> u8 {
    match phase {
        TurnPhase::AdditionalActionFirst => 0,
        TurnPhase::Placement => 1,
        TurnPhase::Movement => 2,
        TurnPhase::AreaAction => 3,
        TurnPhase::FivePowerBonus => 4,
        TurnPhase::Building => 5,
        TurnPhase::Recovery => 6,
        TurnPhase::AdditionalActionSecond => 7,
        TurnPhase::TurnEnd => 8,
    }
}

/// Run one cell.
///
/// Each player count starts a FRESH schedule on the same master, so the three
/// cells draw the same world seeds. That is deliberate and carries no
/// cross-cell dependence: a world seed feeds `new_game`, whose board, decks and
/// deal differ entirely by player count, so "the same seed" names a
/// reproducible run and nothing more.
fn run_cell(players: u8, scale: Scale, rules: &Rules) -> Cell {
    let mut schedule = SeedSchedule::diagnostic(MASTER);
    let seats = vec![POLICY; players as usize];
    let mut cell = Cell {
        players,
        games_requested: scale.games,
        trigger: TriggerTally::default(),
        applied: 0,
        retreat_picks: 0,
        nonzero_recovery_retreats: 0,
        area_actions: 0,
        builds: 0,
        decision_points: 0,
        tails: Vec::new(),
        failures: Vec::new(),
        elapsed: Duration::ZERO,
    };

    let start = Instant::now();
    for _ in 0..scale.games {
        let seed = schedule.next_world();
        match run_raw_game(seed, players, rules, &seats, MASTER, INVARIANTS) {
            Ok(stats) => {
                // A completed game with no trigger would leave every rate's
                // denominator honest but the corpus quietly short of the
                // ending it was run to reach; it is recorded as a failure
                // rather than silently observed.
                if stats.trigger.is_none() {
                    cell.failures.push(format!(
                        "{players}P seed {:#018x}: completed with no end-game trigger",
                        seed.0
                    ));
                }
                cell.trigger.observe(stats.trigger.as_ref());
                cell.applied += stats.coverage.applied();
                cell.retreat_picks += stats.coverage.retreat_picks();
                cell.nonzero_recovery_retreats += stats.coverage.nonzero_recovery_retreats();
                cell.area_actions += stats.coverage.area_actions();
                cell.builds += stats.coverage.builds();
                cell.decision_points += stats.volumes.len() as u64;
                for obs in &stats.volumes {
                    cell.observe_volume(obs.phase, obs.total, obs.has_upper_bound_term);
                }
            }
            // Not observed by the tally: spec §4's failure taxonomy, and
            // `TriggerTally::observe`'s own contract — a game that did not
            // complete is not an observation of the trigger distribution, and
            // counting it would deflate every rate.
            Err(e) => cell.failures.push(format!("{players}P seed {:#018x}: {e}", seed.0)),
        }
    }
    cell.elapsed = start.elapsed();
    cell
}

/// The three cells at one scale, computed at most once per process.
///
/// All four gate tests read the same cell set, so the binary pays for its games
/// once rather than four times. The two tiers get separate slots because a
/// debug `cargo test` runs only the smoke and a release `-- --ignored` run only
/// the gate tests, and neither should ever be served the other's numbers.
fn cells(scale: Scale) -> &'static [Cell] {
    static GATE: OnceLock<Vec<Cell>> = OnceLock::new();
    static SMOKE: OnceLock<Vec<Cell>> = OnceLock::new();
    let slot = if scale == Scale::GATE {
        &GATE
    } else {
        // A third scale would otherwise be served the smoke's numbers in
        // silence, which is the one failure mode of a memo keyed by anything
        // other than its whole input.
        assert_eq!(scale, Scale::SMOKE, "no memo slot for this scale");
        &SMOKE
    };
    slot.get_or_init(|| {
        let rules = Rules::standard();
        PLAYER_COUNTS.iter().map(|&players| run_cell(players, scale, &rules)).collect()
    })
}

fn cell_at(scale: Scale, players: u8) -> &'static Cell {
    cells(scale)
        .iter()
        .find(|cell| cell.players == players)
        .unwrap_or_else(|| panic!("no {players}P cell at scale {scale:?}"))
}

/// The measurement header every printed block carries, so a figure scraped out
/// of this binary's output cannot lose its measurement policy.
fn header(scale: Scale) -> String {
    format!(
        "policy={} (retreat_ppm/recovery_min_of at the shipped defaults)  N={}/cell  \
         counts=2/3/4P  master={MASTER:#018x} (diagnostic)  driver=run_raw_game (raw state, \
         no view/sampler)  invariants={INVARIANTS}  profile={}",
        POLICY.as_str(),
        scale.games,
        if cfg!(debug_assertions) { "debug" } else { "release" },
    )
}

// ---------------------------------------------------------------------------
// Gate cells
// ---------------------------------------------------------------------------

/// §7(a) + §12 gate 2's first clause: the weighted cells COMPLETE, under the
/// per-move invariants contract.
///
/// The invariants half is not decoration. [`INVARIANTS`] is passed to every
/// `run_raw_game` call in this binary, and `check_invariants` runs after each
/// applied move; a violation surfaces as a driver error, which lands in
/// `Cell::failures` and fails here by name and seed. The anti-vacuity check is
/// the applied-move count: a cell that completed 500 games having applied zero
/// moves would satisfy "no failures" while having run no invariant checks at
/// all, so the move total is asserted positive and printed.
#[test]
#[ignore = "release gate: 1500 weighted games; run with cargo test --release -p yokohama-core --test policy_diversity -- --ignored"]
fn weighted_cells_complete_with_invariants() {
    let scale = Scale::GATE;
    let mut block = format!("\nweighted cells (completion + invariants)\n  {}\n", header(scale));
    for cell in cells(scale) {
        block.push_str(&format!(
            "  {}P: {}/{} completed  {} applied moves  {} decision points  {:.2}s\n",
            cell.players,
            cell.trigger.games(),
            cell.games_requested,
            cell.applied,
            cell.decision_points,
            cell.elapsed.as_secs_f64(),
        ));
    }
    println!("{block}");

    for cell in cells(scale) {
        assert!(
            cell.failures.is_empty(),
            "{}P cell did not complete cleanly ({} failures); first: {}",
            cell.players,
            cell.failures.len(),
            cell.failures[0],
        );
        assert_eq!(
            cell.trigger.games(),
            u64::from(cell.games_requested),
            "{}P cell completed {} of {} games",
            cell.players,
            cell.trigger.games(),
            cell.games_requested,
        );
        assert!(
            cell.applied > 0,
            "{}P cell applied no moves, so no invariant check ran",
            cell.players,
        );
    }
}

/// §0-R1's restated trigger gate, at 4P.
///
/// The map's original clause ("ChurchTrackFull > 0%") does not discriminate —
/// uniform satisfies it at two independent harnesses — so the gate is stated
/// against a FROZEN, HELD-OUT uniform ceiling instead: the weighted 4P rate's
/// 95% Wilson LOWER bound must exceed [`UNIFORM_4P_CHURCH_CEILING`].
///
/// ONE-SIDED AND FROZEN, by design. The rejected alternative — a fresh uniform
/// comparator with disjoint intervals — flakes 9.1-9.3% of runs at N=500 and
/// needs N >= 750-1000 per cell, whose comparator games alone cost 28-56s. This
/// design has a computed flake of about 0.2% at the held-out true rates, stated
/// on the constant, and its cost is the weighted cell it already runs.
///
/// The denominator is asserted before the bound: the 0.2% flake figure is a
/// statement about N=500, so a cell that quietly ran fewer games would be gated
/// by a different, unstated flake.
///
/// MUTATION-PROVEN, and the mutation is the interesting one. Bumping the
/// constant would only prove that `assert!` works; instead [`POLICY`] was
/// switched to `RolloutPolicy::Uniform` and this cell re-run, which tests
/// §0-R1's actual claim — that uniform does NOT satisfy the restatement. It
/// measured 4/500 = 0.80%, Wilson95 [0.31%, 2.04%], and FAILED at a margin of
/// −0.41pp, against the weighted policy's +1.11pp. (Reverted by inverse edit,
/// L7.) Note what discriminates and what does not: uniform's POINT estimate
/// here, 0.80%, is above the 0.72% ceiling — it is the LOWER BOUND that falls
/// below, which is precisely why the gate is written on the lower bound.
#[test]
#[ignore = "release gate: run with cargo test --release -p yokohama-core --test policy_diversity -- --ignored"]
fn four_p_trigger_gate_frozen_floor() {
    let scale = Scale::GATE;
    let cell = cell_at(scale, 4);
    let (hits, interval) = cell.church();
    let mut counts: Vec<String> =
        cell.trigger.counts().iter().map(|(name, n)| format!("{name}={n}")).collect();
    if counts.is_empty() {
        counts.push("none recorded".to_string());
    }
    println!(
        "\n4P trigger gate (frozen floor)\n  {}\n  \
         {CHURCH}: {hits}/{} = {:.2}%  Wilson95 [{:.2}%, {:.2}%]\n  \
         frozen uniform ceiling: {:.2}% (7/2000 Wilson95 hi, E1 held-out master \
         16045690984833335230, N=2000, release, raw-state screening driver)\n  \
         margin: lower bound - ceiling = {:+.2}pp\n  all triggers: {}\n",
        header(scale),
        cell.trigger.games(),
        100.0 * hits as f64 / cell.trigger.games().max(1) as f64,
        100.0 * interval.lo,
        100.0 * interval.hi,
        100.0 * UNIFORM_4P_CHURCH_CEILING,
        100.0 * (interval.lo - UNIFORM_4P_CHURCH_CEILING),
        counts.join(" "),
    );

    assert_eq!(
        cell.trigger.games(),
        u64::from(scale.games),
        "the frozen-floor gate's stated flake is computed at N={}; this cell observed {}",
        scale.games,
        cell.trigger.games(),
    );
    assert!(
        interval.lo > UNIFORM_4P_CHURCH_CEILING,
        "weighted 4P {CHURCH} Wilson95 lower bound {:.4} does not exceed the frozen uniform \
         ceiling {UNIFORM_4P_CHURCH_CEILING:.4} ({hits}/{} observed)",
        interval.lo,
        cell.trigger.games(),
    );
}

/// §7(c)'s applied-move coverage floors.
///
/// The floors are [`Floors::GATE`], whose VALUES came from this cell's own
/// measurement (the type doc carries the rule and the measured shares). Every
/// player count must clear the same floor, which is why the floor is set from
/// the MINIMUM measured share: the binding cell keeps a factor of two of
/// headroom and the others keep more.
///
/// The retreat-pick share is PRINTED but not gated: §7 names three floors, and
/// retreat picks are the family the weighted policy's first knob is defined by,
/// so a floor on them would gate the knob's setting rather than the coverage it
/// buys.
///
/// MUTATION-PROVEN by the same method as the trigger gate, and the mutation is
/// again the one that means something: [`POLICY`] was switched to
/// `RolloutPolicy::RetreatAverse` and these cells re-run. The nonzero-recovery
/// share went to EXACTLY 0.0000 at all three counts — 0 of 334,472 / 571,280 /
/// 847,623 applied moves — and the floor failed. That is §7's disjoint-halves
/// claim made executable here rather than merely cited: the rung whose identity
/// is "never volunteer a retreat" cannot pass a floor written on retreats, which
/// is exactly why 3.2 ADDS weighted cells instead of substituting a policy.
/// (Reverted by inverse edit, L7.)
#[test]
#[ignore = "release gate: run with cargo test --release -p yokohama-core --test policy_diversity -- --ignored"]
fn coverage_floors_hold() {
    let scale = Scale::GATE;
    check_coverage(scale, Floors::GATE);
}

/// §6's volume tails, RECORDED per (player count x phase).
///
/// ASSERTS NO TAIL VALUE, and that is §0-R2's finding, not laziness: the map's
/// "option-count tails bounded" mis-aims the criterion. No shape that
/// measurably fixed the 4P trigger collapse kept tails below six figures — one
/// variable (on-board assistant density) drives both the score-productive play
/// and the retreat product — so no bound is claimed and none is known. This run
/// reproduces that shape rather than contradicting it: the 4P Movement row's
/// max is 221,197 at this master, six figures as predicted. What replaced the
/// bound is a hard per-decision COST bound, which lives in
/// `action_source_worstcase.rs`, and this recording.
///
/// What IS asserted is that the recording is COMPLETE — one volume observation
/// per applied move, per cell — so the printed distributions cannot silently
/// become a tail over some unnamed subset.
///
/// Rows carrying an upper-bounded term print the caveat, per §6: placement's
/// `placement_upper_len` is a strict over-estimate wherever a fee gate punched
/// a hole in the affordable subsets. The bias direction travels with it — it
/// inflates uniform-like shapes' tails, so it biases AGAINST the weighted
/// policy, which is the safe direction for the claim these rows support.
#[test]
#[ignore = "release gate: run with cargo test --release -p yokohama-core --test policy_diversity -- --ignored"]
fn tails_are_recorded_per_phase() {
    let scale = Scale::GATE;
    print_tails(scale);
    assert_tails_recorded(scale);
}

// ---------------------------------------------------------------------------
// Shared cell bodies (gate and smoke)
// ---------------------------------------------------------------------------

fn check_coverage(scale: Scale, floors: Floors) {
    let mut block = format!("\napplied-move coverage shares\n  {}\n", header(scale));
    block.push_str(&format!(
        "  floors: nonzero-recovery > {:.4}  area-action > {:.4}  build > {:.4}\n",
        floors.nonzero_recovery, floors.area_action, floors.build,
    ));
    for cell in cells(scale) {
        block.push_str(&format!(
            "  {}P: applied={}  nonzero-recovery={:.4}  area-action={:.4}  build={:.4}  \
             (retreat-pick={:.4}, printed not gated)\n",
            cell.players,
            cell.applied,
            cell.nonzero_recovery_share(),
            cell.area_action_share(),
            cell.build_share(),
            cell.retreat_pick_share(),
        ));
    }
    println!("{block}");

    for cell in cells(scale) {
        for (name, share, floor) in [
            ("nonzero-recovery", cell.nonzero_recovery_share(), floors.nonzero_recovery),
            ("area-action", cell.area_action_share(), floors.area_action),
            ("build", cell.build_share(), floors.build),
        ] {
            assert!(
                share > floor,
                "{}P {name} share {share:.6} is not above the floor {floor:.6} \
                 ({} applied moves)",
                cell.players,
                cell.applied,
            );
        }
    }
}

fn print_tails(scale: Scale) {
    let mut block = format!("\nper-decision-point volume tails\n  {}\n", header(scale));
    for cell in cells(scale) {
        block.push_str(&format!(
            "  {}P ({} decision points over {} games)\n",
            cell.players,
            cell.decision_points,
            cell.trigger.games(),
        ));
        for row in cell.ordered_tails() {
            let Some(q) = row.sketch.quantiles() else { continue };
            block.push_str(&format!(
                "    {:<24} n={:<8} median={:<8} p90={:<8} p99={:<10} max={}{}\n",
                format!("{:?}", row.phase),
                q.n,
                q.median,
                q.p90,
                q.p99,
                q.max,
                if row.any_upper_bound {
                    "   [UPPER BOUND: a placement family contributed placement_upper_len, a \
                     strict over-estimate where a fee gate punched a hole; the bias inflates \
                     tails, i.e. against the weighted policy]"
                } else {
                    ""
                },
            ));
        }
    }
    println!("{block}");
}

fn assert_tails_recorded(scale: Scale) {
    for cell in cells(scale) {
        assert!(!cell.tails.is_empty(), "{}P cell recorded no volume tails", cell.players);
        assert!(
            cell.decision_points > 0,
            "{}P cell recorded no decision points",
            cell.players,
        );
        // "RECORDED" has to mean recorded COMPLETELY, or a row that dropped
        // most of its observations would still pass a non-emptiness check
        // while reporting a tail taken over a subset nobody named. The driver
        // pushes a volume and observes a coverage move in the same loop
        // iteration and treats a `None` volume as a hard `DeadEnd`, so the two
        // totals are equal by construction — which makes this an executable
        // statement of that contract, not an incidental coincidence. It is
        // still not a BOUND on any tail value (§0-R2): it says every decision
        // point is in the distribution, not how large the distribution goes.
        assert_eq!(
            cell.decision_points, cell.applied,
            "{}P recorded {} decision-point volumes for {} applied moves",
            cell.players, cell.decision_points, cell.applied,
        );
        for row in &cell.tails {
            assert!(
                !row.sketch.is_empty(),
                "{}P {:?} tail row exists with no observations",
                cell.players,
                row.phase,
            );
            assert!(
                row.sketch.quantiles().is_some(),
                "{}P {:?} tail row reports no quantiles",
                cell.players,
                row.phase,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Per-edit smoke
// ---------------------------------------------------------------------------

/// The whole battery at [`Scale::SMOKE`] — the one test in this binary that is
/// NOT `#[ignore]`d.
///
/// Its subject is this harness's plumbing: that the cells still run, that the
/// coverage shares are still computable and above a halved floor, and that the
/// tails still record. It deliberately does NOT run the frozen-floor trigger
/// gate, and that is a property of the gate rather than a gap: at N=40 with a
/// true rate near 3.7% the Wilson lower bound sits far below 0.72% no matter
/// how healthy the policy is, so asserting it at smoke scale would fail on a
/// perfectly good build. The gate's discriminating power IS its N, which is
/// exactly why it is release-tier. The 4P rate is printed here anyway, as a
/// development signal.
#[test]
fn the_weighted_cells_run_at_smoke_scale() {
    let scale = Scale::SMOKE;
    for cell in cells(scale) {
        assert!(
            cell.failures.is_empty(),
            "{}P smoke cell did not complete cleanly ({} failures); first: {}",
            cell.players,
            cell.failures.len(),
            cell.failures[0],
        );
        assert_eq!(cell.trigger.games(), u64::from(cell.games_requested));
    }
    check_coverage(scale, Floors::SMOKE);
    print_tails(scale);
    assert_tails_recorded(scale);

    let four_p = cell_at(scale, 4);
    let (hits, interval) = four_p.church();
    let timings: Vec<String> = cells(scale)
        .iter()
        .map(|cell| format!("{}P {:.2}s", cell.players, cell.elapsed.as_secs_f64()))
        .collect();
    println!(
        "\n4P {CHURCH} at smoke scale: {hits}/{} Wilson95 [{:.2}%, {:.2}%] \
         (PRINTED, not gated — see this test's doc)\n  smoke cell cost: {}\n",
        four_p.trigger.games(),
        100.0 * interval.lo,
        100.0 * interval.hi,
        timings.join("  "),
    );
}
