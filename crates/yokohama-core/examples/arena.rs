//! The arena runner — Phase 3.2 (spec `docs/plans/2026-07-25-phase3.2-arena-spec.md`
//! §4's `examples/arena.rs`, §11's truth-pass invocation, §12's throughput
//! obligation).
//!
//! Three subcommands, one job each:
//!
//!   * **`cell`** — run ONE matchup and print its report. The general instrument:
//!     any two rungs, any supported player count, any unit count, optionally
//!     emitting a corpus. This is the invocation `.gitignore`'s corpus comment
//!     points at — the corpus is reproducible from a committed command rather
//!     than from history.
//!   * **`ab`** — the §11 truth-pass: uniform vs retreat-averse at 2P AND 4P at
//!     stated N, plus the fresh uniform 4P trigger rate that drift-watches
//!     §0-R1's frozen constant. The report artifact this prints is committed.
//!   * **`throughput`** — the arena driver's OWN per-decision and per-game cost.
//!     Spec §12 requires 3.2 to produce this number for itself and forbids
//!     composing it from anyone else's: the screening figures in the research
//!     notes (2.48 µs/decision for the weighted shape) were measured on a
//!     raw-state driver with no validation and no invariants, and this driver
//!     observes, samples a world, validates every move, and checks invariants.
//!     Sizing anything from the wrong one of those is critic #9's hazard.
//!
//! MEASUREMENT POLICY IS PRINTED, NOT ASSUMED. Every subcommand leads with a
//! header naming the build profile, the semantics rev, the master and HOW it was
//! obtained, the policies with their version and knobs, N, and whether per-move
//! invariants ran. A figure scraped out of this output carries its conditions
//! with it, which is the standing rule for every number this project records.
//!
//! MASTERS. `--master <u64>` is a DIAGNOSTIC master (reproduction, fixtures,
//! gate cells); `--os-entropy` draws one and prints it, which is the rule for
//! bot-facing evaluation runs whose figures get quoted (spec §3). The default is
//! diagnostic, because the common use of this binary is reproducing something.
//!
//! CONTESTANTS, not just policies, since Phase 3.3 (spec §5.3): `<x>` and `<y>`
//! accept the 3.2 rungs AND the evaluator controllers (`greedy-v1`,
//! `lookahead-v1`, `atomic-diag-v1`), each with its own knob grammar — and since
//! Phase 3.4 the architecture contestant `mcts-v1`, plus a `cap=` knob on
//! `lookahead-v1` (the shared scored-successor budget) — and since V4 step 8
//! the NAMED IDENTITIES `v2-padded` / `v2-refit-padded`, knobless arms that
//! differ from `lookahead-v1` in their weight TABLE alone, so a §11 weight
//! comparison is a runnable cell rather than a document. A cell mixing the rung
//! family with an evaluator/architecture one is swapped-balanced but NOT
//! setup-identical — they resolve a dealt Order pair by different Keep rules —
//! and both the header and the report echo say which kind of cell ran.
//!
//! THE GRAMMAR ITSELF LIVES IN CORE since Phase 3.5c W1-B: this runner parses
//! `<x>`/`<y>` through [`Contestant::from_str_name`] (`game/arena/grammar.rs`),
//! which the evaluator probe also calls since W1-C and the CLI's bot seats
//! WILL call at their W5 retype (today they still parse rung names through
//! `RolloutPolicy::from_str_name`) — so a spec typed at the runner or the probe
//! means the same thing, and unknown names, knobs and knob VALUES are errors
//! there exactly as they were here.
//!
//! THE TWO AXES (3.4 spec §0.3). Every `cell` invocation now prints, per SIDE:
//! the realized scored-successor spend with its root share and cap counters
//! (deterministic, off the `CellReport`), and the realized in-cell µs/decision
//! (load-sensitive, off the driver's clock and deliberately NOT on the report —
//! see `CellWall`). A node-matched cell hands the tree arm a wall advantage by
//! construction, so both axes have to be visible or the matching cannot be read.
//!
//! ```text
//! cargo run --release -p yokohama-core --example arena -- throughput
//! cargo run --release -p yokohama-core --example arena -- ab
//! cargo run --release -p yokohama-core --example arena -- cell fw-selfplay 4 \
//!     family-weighted family-weighted 25 --emit crates/yokohama-core/corpus
//! cargo run --release -p yokohama-core --example arena -- cell greedy-vs-fw 2 \
//!     greedy-v1 family-weighted 50
//! cargo run --release -p yokohama-core --example arena -- cell s1-vs-s2 2 \
//!     lookahead-v1:b=4 atomic-diag-v1:d=2 20
//! cargo run --release -p yokohama-core --example arena -- cell c2-node-matched 2 \
//!     mcts-v1:cap=750 lookahead-v1:b=4,cap=750 200 --os-entropy
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use yokohama_core::{
    corpus_path, emit_record, greedy, lookahead, mcts, run_cell_emitting_timed, run_raw_game,
    trigger_name, ArenaError, CellReport, CellSpec, CellWall, Contestant, IntervalOutcome,
    RolloutPolicy, Rules, SeedSchedule, SeedScheduleKind, TriggerTally, SEMANTICS_REV,
};

const USAGE: &str = "\
usage: arena <subcommand> [options]

  cell <name> <players> <x> <y> <units> [--emit <dir>] [--invariants|--no-invariants]
                                        [--master <u64> | --os-entropy]
      Run one matchup. <x> is the contestant under test (ONE seat); <y> fills the
      other players-1 seats. <units> counts DESIGN UNITS, not games: a 2P unit is
      the swapped pair (2 games), a 3-4P unit is the rotation block (players games).
      A cell mixing a rung with an evaluator contestant is swapped-balanced but NOT
      setup-identical (the two resolve a deal by different Keep rules); the report's
      spec echo prints setup-identical=yes|no per cell.

  ab [--units-2p <n>] [--blocks-4p <n>] [--drift-games <n>]
     [--master <u64> | --os-entropy]
      The truth-pass A/B: uniform vs retreat-averse at 2P and 4P, plus a fresh
      uniform 4P selfplay trigger cell that drift-watches the frozen ChurchTrackFull
      ceiling. Defaults: 400 pairs, 100 blocks, 200 drift games.

  throughput [--games <n>] [--master <u64> | --os-entropy]
      Per-game and per-decision cost of THIS driver, per rung and per gate cell
      shape. Default 3 games per rung/count cell.

contestants (<x> and <y>):
  RUNGS (3.2, sampler-side policies)
    uniform | retreat-averse | family-weighted
    family-weighted:p=<ppm>,k=<n>   e.g. family-weighted:p=50000,k=5
        p is the retreat probability in PARTS PER MILLION (50000 = 5%), k is the
        number of iid draws each area's recovery count is the minimum of. Both knobs
        are optional and default to the shipped FAMILY_WEIGHTED_DEFAULT (166667, 5).
  EVALUATOR CONTESTANTS (3.3)
    greedy-v1[:m=<n>]                    one-ply argmax of evaluator v1; m = retreat samples (8)
    lookahead-v1[:b=<n>,k=<n>,m=<n>,cap=<n>]
                                         S1 turn-boundary search; b = beam (4), k = worlds (1)
        cap is Phase 3.4's TOTAL scored-successor budget per decision, root pass
        included. OMIT IT for the shipped rung: an absent cap is byte-identical to
        the 3.3 arm and emits no provenance token at all.
    atomic-diag-v1[:d=<n>,k=<n>,m=<n>]   S2 DIAGNOSTIC atomic depth-d arm; d = depth in moves (2)
        (also spelled lookahead-atomic-diag-v1, its provenance controller name)
  ARCHITECTURE CONTESTANT (3.4)
    mcts-v1[:cap=<n>,c=<ppm>,scale=<n>,m=<n>,leaf=<mode>]
        A UCT tree over one sampled world. cap is the SAME budget knob as above and
        is how a cell is sized (spec §5's cells are named mcts-v1@<cap>): a capped
        seat runs simulations at a ceiling and lets the budget decide, an uncapped
        one runs the default simulation count. c is UCB1's exploration constant in
        PARTS PER MILLION (1414214 ~ sqrt 2), scale the value squash's scale in
        evaluator-score units (100), m the retreat samples (8), and leaf one of
        hand-off (default) | midturn-diag | constant-diag.
      All four run the frozen evaluator weight table; a weight change is a version
      bump, not a cell knob.
  NAMED IDENTITIES (V4 step 8) -- the one exception to the line above, and it is
  an exception by NAME rather than by knob: a named identity is a distinct
  contestant whose table is part of what its name means, never a `weights=` knob
  on an arm above.
    v2-padded                            lookahead at the shipped knobs on WeightTable::V2
                                         (the shipped table, padded) -- the same seat
                                         `lookahead-v1` plays, said as an identity
    v2-refit-padded                      the same seat on the §8 B-cell REFIT of the sixteen
                                         incumbents (no candidate column admitted); NOT a
                                         shipped table
      Both are KNOBLESS: they run one transcribed knob set (b=4, k=1, m=8, uncapped)
      so a cell pitting them against each other varies the TABLE and nothing else.
      Their echoes carry the name and the resolved `weights=` identity. They are
      ARENA arms: examples/eval_probe selects its table with --weights instead, the
      driven harness refuses them, and they are not CLI bot seats.
      Unknown contestant names, unknown knobs and unknown knob VALUES are errors.
";

/// Spec §0-R1's frozen uniform 4P ChurchTrackFull ceiling: 7/2000, Wilson-95
/// upper 0.72% — E1 held-out master B `16045690984833335230`, N=2000, release,
/// raw-state screening driver. Printed beside the fresh rate as a DRIFT WATCH.
/// The gate that asserts against it lives in `tests/policy_diversity.rs`; this
/// binary only ever records.
const UNIFORM_4P_CHURCH_CEILING: f64 = 0.0072;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("cell") => cmd_cell(&args[1..]),
        Some("ab") => cmd_ab(&args[1..]),
        Some("throughput") => cmd_throughput(&args[1..]),
        _ => Err(USAGE.to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(2);
    }
}

// ---------------------------------------------------------------------------
// argv
// ---------------------------------------------------------------------------

/// Split argv into positionals and `--flag [value]` pairs.
///
/// `valued` names the flags that consume the next argument; anything else
/// starting with `--` is a bare flag and maps to an empty string. An unknown
/// flag is an ERROR rather than a silent no-op — a mistyped `--os-entropi` that
/// quietly left the run on a diagnostic master would put a fixed master behind a
/// figure published as freshly drawn.
fn split_args(
    args: &[String],
    valued: &[&str],
    bare: &[&str],
) -> Result<(Vec<String>, BTreeMap<String, String>), String> {
    let mut positional = Vec::new();
    let mut flags: BTreeMap<String, String> = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if let Some(name) = arg.strip_prefix("--") {
            if valued.contains(&name) {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| format!("--{name} needs a value\n\n{USAGE}"))?;
                flags.insert(name.to_string(), value.clone());
                i += 2;
                continue;
            }
            if bare.contains(&name) {
                flags.insert(name.to_string(), String::new());
                i += 1;
                continue;
            }
            return Err(format!("unknown flag --{name}\n\n{USAGE}"));
        }
        positional.push(arg.clone());
        i += 1;
    }
    Ok((positional, flags))
}

/// `--master <u64>` (diagnostic) or `--os-entropy` (drawn and recorded).
fn master_from(flags: &BTreeMap<String, String>) -> Result<SeedScheduleKind, String> {
    match (flags.get("master"), flags.contains_key("os-entropy")) {
        (Some(_), true) => Err("--master and --os-entropy are mutually exclusive".to_string()),
        (Some(raw), false) => parse_u64(raw).map(SeedScheduleKind::Diagnostic),
        (None, true) => Ok(SeedScheduleKind::OsEntropy),
        (None, false) => Ok(SeedScheduleKind::Diagnostic(DEFAULT_MASTER)),
    }
}

/// The default diagnostic master when neither flag is given. Distinct from the
/// gate binaries' masters on purpose: a figure produced by an unflagged run of
/// this tool must not be mistakable for a gate cell's.
const DEFAULT_MASTER: u64 = 0xA3E1_2026_0726_0A11;

fn parse_u64(raw: &str) -> Result<u64, String> {
    let cleaned = raw.replace('_', "");
    let parsed = match cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => cleaned.parse::<u64>(),
    };
    parsed.map_err(|e| format!("{raw:?} is not a u64: {e}"))
}

// ---------------------------------------------------------------------------
// Printing
// ---------------------------------------------------------------------------

fn profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

/// The header's contestant token, delegated to the library's own composer
/// (public since Phase 3.5c W5) exactly as this file's PARSING already
/// delegates to `Contestant::from_str_name`. A private copy here would be a
/// second spelling of a format the goldens pin, free to drift from the report
/// echo printed three lines further down.
fn contestant_echo(contestant: Contestant) -> String {
    contestant.echo()
}

fn master_echo(kind: SeedScheduleKind) -> String {
    match kind {
        SeedScheduleKind::Diagnostic(m) => format!("diagnostic:{m:#018x}"),
        SeedScheduleKind::OsEntropy => "os-entropy (drawn per run, printed below)".to_string(),
    }
}

/// The measurement-policy header every subcommand leads with.
fn print_header(what: &str, kind: SeedScheduleKind, rows: &[(&str, String)]) {
    println!("=== {what} ===");
    println!(
        "  build={} semantics_rev={SEMANTICS_REV} master={} single-threaded=yes",
        profile(),
        master_echo(kind),
    );
    for (key, value) in rows {
        println!("  {key}={value}");
    }
    if profile() == "debug" {
        println!(
            "  WARNING: this is a DEBUG build. Every wall figure below is unoptimized and is not \
             comparable to a release figure; re-run with --release before recording anything."
        );
    }
}

fn describe_interval(interval: &IntervalOutcome) -> String {
    match interval {
        IntervalOutcome::Bootstrap { lo, hi, b, resample_seed } => {
            format!("bootstrap 95% [{lo:.6}, {hi:.6}] (B={b}, resample_seed={resample_seed:#018x})")
        }
        IntervalOutcome::RefusedZeroWidth { one_sided_lower, method } => {
            format!("REFUSED (zero width); one-sided 95% lower bound {one_sided_lower:.6} — {method}")
        }
    }
}

fn print_report(report: &CellReport) {
    println!("  {}", report.spec_echo);
    println!("  theta_hat = {:.6}   [null for this cell is printed in the cross-checks]", report.theta_hat);
    println!("  interval  = {}", describe_interval(&report.interval));
    println!(
        "  seat0 wins {}/{} · top-VP ties decided for seat 0: {} · mean VP margin {:+.2}",
        report.seat0_wins, report.games, report.ties_to_seat0, report.vp_margin_mean,
    );
    for row in &report.cross_checks {
        println!("    - {row}");
    }
    print_triggers("  ", &report.trigger);
}

/// The realized unit-share distribution and its variance — the 3.3 spec §4.1
/// obligation that "the cell REPORTS its realized Var_pair".
///
/// WHY THIS IS NOT IN [`print_report`], which `ab` also calls: the 3.2 A/B
/// truth-pass artifact (`docs/plans/2026-07-26-phase3.2-ab-report.md`) is a
/// VERBATIM capture of that subcommand's output, so a line added to the shared
/// printer would silently put the committed artifact out of date with its own
/// reproduce command. The obligation is a `cell` obligation — §4.1's experiment
/// is a `cell` invocation — so it is discharged here.
///
/// BOTH divisors are printed because neither is "the" variance without saying
/// which: the `n − 1` form is what the unit-t cross-check's `sd` squares to, the
/// `n` form is the population variance of the units as realized. They differ in
/// the third-to-fourth decimal at cell sizes; a quoted figure should name its
/// divisor. The name `Var_pair` is 2P's — at 3–4P the unit is a rotation block
/// and [`yokohama_core::pairs_needed_for_mme`] does not apply to it — so the
/// label says `unit` and notes where the pair reading holds.
fn print_unit_variance(report: &CellReport) {
    let shares = &report.x_unit_shares;
    let n = shares.len();
    if n == 0 {
        println!("  unit shares: none tabulated");
        return;
    }
    let mut sorted = shares.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).expect("unit shares are finite"));
    let mut histogram: Vec<(f64, u32)> = Vec::new();
    for share in sorted {
        match histogram.last_mut() {
            Some((value, count)) if *value == share => *count += 1,
            _ => histogram.push((share, 1)),
        }
    }
    let spread: Vec<String> = histogram.iter().map(|(v, c)| format!("{c}×{v:.4}")).collect();
    let mean = shares.iter().sum::<f64>() / n as f64;
    let ss: f64 = shares.iter().map(|s| (s - mean) * (s - mean)).sum();
    println!("  unit shares (n={n}, X's win share per unit): {}", spread.join(" · "));
    println!(
        "  realized unit variance = {:.6} (population, /n) · {:.6} (sample, /(n-1)) \
         — spec §4.1's Var_pair at 2P",
        ss / n as f64,
        if n > 1 { ss / (n - 1) as f64 } else { f64::NAN },
    );
}

/// Spec §0.3's TWO AXES for one cell: the deterministic budget meters off the
/// report, and the load-sensitive per-side wall off the driver's clock.
///
/// The wall lives HERE and only here (the T7-wall-line convention): it is not a
/// `CellReport` field, because that report is byte-identity-pinned and a wall
/// figure is not reproducible by construction. Both rows print their `None`s
/// rather than skipping them — a contestant with no budget meter is a fact about
/// the cell, and a blank line reads as a zero.
fn print_budget_axes(report: &CellReport, wall: &CellWall) {
    let y_decisions = report.applied_by_seat.iter().sum::<u64>() - report.x_applied_moves;
    let per_decision = |total: Option<u64>, decisions: u64| -> String {
        match total {
            None => "n/a".to_string(),
            Some(_) if decisions == 0 => "n/a".to_string(),
            Some(t) => format!("{:.1}", t as f64 / decisions as f64),
        }
    };
    let opt = |v: Option<u64>| v.map_or_else(|| "n/a".to_string(), |n| n.to_string());

    println!("  budget axes (spec §0.3) — scored successors are the currency, root pass included:");
    for (side, decisions, scored, root, binds, starved, side_wall) in [
        (
            "X",
            report.x_applied_moves,
            report.x_candidates_scored,
            report.x_root_scored,
            report.x_budget_binds,
            report.x_budget_starved_decisions,
            wall.x,
        ),
        (
            "Y",
            y_decisions,
            report.y_candidates_scored,
            report.y_root_scored,
            report.y_budget_binds,
            report.y_budget_starved_decisions,
            wall.y,
        ),
    ] {
        println!(
            "    {side}: {decisions} decisions · scored {} ({} /dec, root share {}) · \
             cap binds {} · starved {} · wall {:.1} us/dec",
            opt(scored),
            per_decision(scored, decisions),
            match (root, scored) {
                (Some(r), Some(s)) if s > 0 => format!("{:.3}", r as f64 / s as f64),
                _ => "n/a".to_string(),
            },
            opt(binds),
            opt(starved),
            side_wall.micros_per_decision(),
        );
    }
    println!(
        "    read: the SCORED figures are deterministic in the master and reproduce; the WALL \
         figures are load-sensitive, are not on the CellReport, and are never asserted. Y sums \
         the players-1 field seats. A cap that never binds is not a matched budget."
    );
}

/// End-trigger rates with 95% Wilson intervals, per spec §6 — never pooled
/// across player counts, which is exactly the pooling that blinded the old fuzz
/// assertion to the 4P collapse.
fn print_triggers(indent: &str, tally: &TriggerTally) {
    let games = tally.games();
    println!("{indent}  end-trigger rates over {games} games (95% Wilson):");
    if tally.counts().is_empty() {
        println!("{indent}    (no trigger recorded on any game)");
    }
    for (name, count) in tally.counts() {
        let (seen, interval) = tally.rate(name);
        debug_assert_eq!(seen, *count);
        println!(
            "{indent}    {name:<22} {seen:>5}/{games:<5} = {:.4}  [{:.4}, {:.4}]",
            seen as f64 / games as f64,
            interval.lo,
            interval.hi,
        );
    }
}

// ---------------------------------------------------------------------------
// `cell`
// ---------------------------------------------------------------------------

fn cmd_cell(args: &[String]) -> Result<(), String> {
    let (positional, flags) =
        split_args(args, &["emit", "master"], &["os-entropy", "invariants", "no-invariants"])?;
    if positional.len() != 5 {
        return Err(format!("cell takes 5 positional arguments\n\n{USAGE}"));
    }
    let name = positional[0].clone();
    let players: u8 = positional[1].parse().map_err(|e| format!("<players>: {e}"))?;
    // The contestant grammar lives in CORE since Phase 3.5c W1-B
    // (`Contestant::from_str_name`, `game/arena/grammar.rs`) — this runner and
    // the probe (W1-C) parse the same specs through the same code, the CLI's
    // bot seats join at their W5 retype, and its refusals are the user-facing
    // text they always were.
    let x = Contestant::from_str_name(&positional[2])?;
    let y = Contestant::from_str_name(&positional[3])?;
    let units: u32 = positional[4].parse().map_err(|e| format!("<units>: {e}"))?;
    let master = master_from(&flags)?;
    let emit: Option<PathBuf> = flags.get("emit").map(PathBuf::from);

    // Invariants default ON — every driver in this repo contracts for per-move
    // `check_invariants`, and spec §4's off-switch is for BULK corpus generation
    // whose records are verified by strict replay instead. `--invariants` is
    // accepted (and is a no-op) so a caller can state the default explicitly;
    // turning it off has to be typed out.
    if flags.contains_key("invariants") && flags.contains_key("no-invariants") {
        return Err("--invariants and --no-invariants are mutually exclusive".to_string());
    }
    let invariants = !flags.contains_key("no-invariants");

    print_header(
        &format!("cell {name}"),
        master,
        &[
            ("players", players.to_string()),
            ("x", contestant_echo(x)),
            ("y", contestant_echo(y)),
            ("units", units.to_string()),
            ("games", (u32::from(players) * units).to_string()),
            (
                "setup-identical",
                // Spec §5.5, stated in the header as well as in the report echo:
                // a mixed-Keep cell's units are swapped-balanced but not
                // setup-identical, and that is a fact about the RUN a reader
                // should meet before the figures.
                if x.keep_kind() == y.keep_kind() {
                    "yes (Keep-homogeneous)".to_string()
                } else {
                    "no (mixed Keep — swapped-balanced, seed-paired at new_game)".to_string()
                },
            ),
            ("per-move invariants", invariants.to_string()),
            (
                "emit",
                emit.as_ref().map_or_else(|| "off".to_string(), |d| d.display().to_string()),
            ),
        ],
    );

    let rules = Rules::standard();
    let spec = CellSpec { name: name.clone(), players, x, y, units, master };
    let started = Instant::now();
    let mut written = 0u32;
    let (report, wall) = run_cell_emitting_timed(&spec, &rules, invariants, &mut |stats, arr| {
        let Some(dir) = &emit else { return Ok(()) };
        let path = corpus_path(dir, SEMANTICS_REV, players, &name, stats.seed, arr);
        emit_record(&path, &stats.record)
            .map_err(|e| ArenaError::Driver(format!("emit {}: {e}", path.display())))?;
        written += 1;
        Ok(())
    })
    .map_err(|e| format!("cell failed: {e}"))?;
    let elapsed = started.elapsed();

    print_report(&report);
    print_unit_variance(&report);
    print_budget_axes(&report, &wall);
    println!(
        "  wall {:.2}s for {} games ({:.1} ms/game, {} build)",
        elapsed.as_secs_f64(),
        report.games,
        elapsed.as_secs_f64() * 1000.0 / f64::from(report.games),
        profile(),
    );
    if let Some(dir) = &emit {
        println!("  emitted {written} records under {}", dir.display());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// `ab` — the §11 truth-pass
// ---------------------------------------------------------------------------

fn cmd_ab(args: &[String]) -> Result<(), String> {
    let (positional, flags) = split_args(
        args,
        &["units-2p", "blocks-4p", "drift-games", "master"],
        &["os-entropy"],
    )?;
    if !positional.is_empty() {
        return Err(format!("ab takes no positional arguments\n\n{USAGE}"));
    }
    let units_2p: u32 = parse_flag(&flags, "units-2p", 400)?;
    let blocks_4p: u32 = parse_flag(&flags, "blocks-4p", 100)?;
    let drift_games: u32 = parse_flag(&flags, "drift-games", 200)?;
    let master = master_from(&flags)?;

    print_header(
        "A/B truth-pass: uniform vs retreat-averse (spec §11)",
        master,
        &[
            ("2P units (pairs)", units_2p.to_string()),
            ("4P units (rotation blocks)", blocks_4p.to_string()),
            ("drift-watch games", drift_games.to_string()),
            ("per-move invariants", "true (cells) / false (drift watch)".to_string()),
            (
                "what this is",
                "the in-repo, arena-produced record of the preserved bracket — expected to \
                 reproduce E2's blowout (4000/4000 games at 2P, 400/400 across 100 blocks at 4P)"
                    .to_string(),
            ),
        ],
    );

    let rules = Rules::standard();
    let started = Instant::now();

    for (players, units) in [(2u8, units_2p), (4u8, blocks_4p)] {
        let spec = CellSpec {
            name: format!("ab-uniform-vs-ra-{players}p"),
            players,
            x: Contestant::Policy(RolloutPolicy::Uniform),
            y: Contestant::Policy(RolloutPolicy::RetreatAverse),
            units,
            master,
        };
        println!("\n-- {players}P: X = uniform, Y = retreat-averse --");
        let cell_started = Instant::now();
        let report = yokohama_core::run_cell(&spec, &rules).map_err(|e| format!("{players}P cell failed: {e}"))?;
        print_report(&report);
        println!(
            "  wall {:.2}s for {} games",
            cell_started.elapsed().as_secs_f64(),
            report.games,
        );
    }

    drift_watch(&rules, master, drift_games)?;

    println!("\ntotal wall {:.2}s ({} build)", started.elapsed().as_secs_f64(), profile());
    Ok(())
}

/// The §0-R1 drift watch: a FRESH uniform 4P trigger rate, recorded beside the
/// frozen ceiling the `policy_diversity` gate asserts against.
///
/// COMPARABILITY IS THE WHOLE POINT, so the harness is matched to the one the
/// frozen constant came from: uniform at EVERY seat (not the mixed A/B cell
/// above, whose rate is a different quantity), the RAW-STATE screening driver,
/// invariants off. One difference is stated rather than hidden: this driver
/// validates every move with `is_move_legal` where E1's did not. That cannot
/// move the distribution — validation consumes no randomness and rejects
/// nothing a policy drew from the legal families — but it is a difference, and
/// unstated differences are how comparability claims rot.
///
/// This RECORDS. It never asserts: §0-R1's design is a frozen baseline
/// precisely because a fresh comparator at gate N flakes 9.1–9.3% of runs.
fn drift_watch(rules: &Rules, master: SeedScheduleKind, games: u32) -> Result<(), String> {
    println!("\n-- drift watch: FRESH uniform 4P selfplay (raw-state screening driver) --");
    let (mut schedule, drawn) = match master {
        SeedScheduleKind::Diagnostic(m) => (SeedSchedule::diagnostic(m), m),
        SeedScheduleKind::OsEntropy => SeedSchedule::from_os_entropy(),
    };
    let seat_policies = [RolloutPolicy::Uniform; 4];
    let mut tally = TriggerTally::default();
    let started = Instant::now();

    for _ in 0..games {
        let seed = schedule.next_world();
        let stats = run_raw_game(seed, 4, rules, &seat_policies, drawn, false)
            .map_err(|e| format!("drift-watch game failed: {e}"))?;
        tally.observe(stats.trigger.as_ref());
    }

    println!(
        "  master={drawn:#018x} games={games} policy=uniform@all-seats driver=raw-state \
         invariants=off profile={}",
        profile(),
    );
    print_triggers("", &tally);
    let (seen, interval) = tally.rate(trigger_name(&yokohama_core::EndGameTrigger::ChurchTrackFull));
    println!(
        "  DRIFT WATCH: ChurchTrackFull {seen}/{games} = {:.4}, Wilson 95% [{:.4}, {:.4}] against \
         the FROZEN ceiling {UNIFORM_4P_CHURCH_CEILING:.4} (7/2000 Wilson-hi, E1 held-out master B \
         16045690984833335230, N=2000, release, raw-state screening driver)",
        seen as f64 / f64::from(games),
        interval.lo,
        interval.hi,
    );
    println!(
        "  read: this is a RECORDED comparison, never an assertion — at N={games} the interval is \
         wide, and the frozen constant is what `tests/policy_diversity.rs` gates on. What would be \
         actionable here is the fresh Wilson interval sitting entirely ABOVE the frozen ceiling, \
         which would mean the baseline itself has moved and the constant needs re-deriving."
    );
    println!("  wall {:.2}s", started.elapsed().as_secs_f64());
    Ok(())
}

fn parse_flag(flags: &BTreeMap<String, String>, name: &str, default: u32) -> Result<u32, String> {
    match flags.get(name) {
        Some(raw) => raw.parse::<u32>().map_err(|e| format!("--{name}: {e}")),
        None => Ok(default),
    }
}

// ---------------------------------------------------------------------------
// `throughput` — the number 3.2 must produce for itself
// ---------------------------------------------------------------------------

/// Per-game and per-decision cost of THIS driver.
///
/// Two blocks, because they answer two different sizing questions:
///
///   * **per-rung selfplay** — one rung at every seat, honest driver, with and
///     without per-move invariants. This is the figure to size a bulk corpus or
///     a fuzz-shape cell from, and the invariants-on/off pair is what makes the
///     off-switch's value concrete rather than assumed.
///   * **gate cell shapes** — the exact `CellSpec`s `tests/arena_gate.rs` runs,
///     measured through `run_cell` at small N. This is the block the gate's own
///     sizing comes from: ms/game on the cell shape that will actually run,
///     including its tabulation, rather than a per-rung average that no cell has.
///
/// NOT COMPOSABLE with anyone else's throughput number, and stated here because
/// spec §12 says so: the screening figures in the research notes ran a raw-state
/// driver with no `observe`, no belief sampler, no `is_move_legal`, and no
/// invariants. This driver does all four per decision.
fn cmd_throughput(args: &[String]) -> Result<(), String> {
    let (positional, flags) = split_args(args, &["games", "master"], &["os-entropy"])?;
    if !positional.is_empty() {
        return Err(format!("throughput takes no positional arguments\n\n{USAGE}"));
    }
    let games: u32 = parse_flag(&flags, "games", 3)?;
    let master = master_from(&flags)?;

    print_header(
        "throughput (spec §12: the arena driver's own cost)",
        master,
        &[
            ("games per cell", games.to_string()),
            ("driver", "run_arena_game — observe -> sample -> handle -> pick -> is_move_legal".to_string()),
            ("timing", "std::time::Instant, whole games, no warm-up discarded".to_string()),
            (
                "NOT comparable to",
                "the research notes' 2.48 µs/decision — that is a raw-state driver with no \
                 validation and no invariants (spec §12, critic #9: no additive composition)"
                    .to_string(),
            ),
        ],
    );

    let rules = Rules::standard();
    let base = match master {
        SeedScheduleKind::Diagnostic(m) => m,
        SeedScheduleKind::OsEntropy => {
            let drawn = SeedSchedule::from_os_entropy().1;
            // Recorded, per spec §3 and this header's own promise (T9 review
            // F2): an unrecorded os-entropy master makes every figure below
            // unreproducible.
            println!("os-entropy master drawn this run: {drawn:#018x} (re-run via --master)");
            drawn
        }
    };

    println!(
        "\n{:<18} {:>7} {:>7} {:>11} {:>12} {:>10} {:>13}",
        "rung", "players", "inv", "turns/game", "moves/game", "ms/game", "us/decision",
    );
    for invariants in [true, false] {
        for players in [2u8, 4] {
            for rung in [
                RolloutPolicy::Uniform,
                RolloutPolicy::RetreatAverse,
                RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
            ] {
                let mut schedule = SeedSchedule::diagnostic(base ^ u64::from(players));
                let mut turns = 0u64;
                let mut moves = 0u64;
                let started = Instant::now();
                for _ in 0..games {
                    let seed = schedule.next_world();
                    let seat_policies = vec![rung; players as usize];
                    // The RAW driver is deliberately NOT used here: this block
                    // measures the honest chain, which is what every arena cell
                    // pays. Selfplay of one rung is expressed by giving every
                    // seat the same policy through the honest factory.
                    let stats = yokohama_core::run_arena_game(
                        seed,
                        players,
                        &rules,
                        &mut |seat, _n| {
                            Box::new(yokohama_core::PolicyAgent::new(
                                seat_policies[seat.0 as usize],
                                yokohama_core::agent_stream_seed(
                                    seed,
                                    seat,
                                    yokohama_core::Slot(seat.0),
                                ),
                            ))
                        },
                        invariants,
                    )
                    .map_err(|e| format!("{players}P {} throughput game failed: {e}", rung.as_str()))?;
                    turns += u64::from(stats.turns);
                    moves += stats.moves_applied;
                }
                let elapsed = started.elapsed().as_secs_f64();
                println!(
                    "{:<18} {players:>7} {:>7} {:>11.1} {:>12.1} {:>10.2} {:>13.2}",
                    rung.as_str(),
                    if invariants { "on" } else { "off" },
                    turns as f64 / f64::from(games),
                    moves as f64 / f64::from(games),
                    elapsed * 1000.0 / f64::from(games),
                    elapsed * 1e6 / moves as f64,
                );
            }
        }
    }

    println!("\n-- gate cell shapes (run_cell, per-move invariants ON) --");
    println!(
        "{:<28} {:>7} {:>6} {:>6} {:>10} {:>10}",
        "cell", "players", "units", "games", "ms/unit", "ms/game",
    );
    let shapes: [(&str, u8, RolloutPolicy, RolloutPolicy, u32); 5] = [
        ("null-ra-vs-ra", 2, RolloutPolicy::RetreatAverse, RolloutPolicy::RetreatAverse, 4),
        ("edge-ra-vs-fw", 2, RolloutPolicy::RetreatAverse, RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, 4),
        (
            "edge-fw-vs-fw",
            2,
            RolloutPolicy::FamilyWeighted { retreat_ppm: 50_000, recovery_min_of: 5 },
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
            4,
        ),
        ("bracket-ra-vs-uniform", 2, RolloutPolicy::RetreatAverse, RolloutPolicy::Uniform, 4),
        ("corpus-4p-ra-vs-fw", 4, RolloutPolicy::RetreatAverse, RolloutPolicy::FAMILY_WEIGHTED_DEFAULT, 2),
    ];
    for (name, players, x, y, units) in shapes {
        // Rung shapes: these are the 3.2 gate cells' costs, so they stay
        // Policy-vs-Policy. Evaluator cell costs belong to 3.3's own gate binary.
        let spec = CellSpec {
            name: name.to_string(),
            players,
            x: Contestant::Policy(x),
            y: Contestant::Policy(y),
            units,
            master: SeedScheduleKind::Diagnostic(base),
        };
        let started = Instant::now();
        let report =
            yokohama_core::run_cell(&spec, &rules).map_err(|e| format!("shape {name} failed: {e}"))?;
        let elapsed = started.elapsed().as_secs_f64();
        println!(
            "{name:<28} {players:>7} {units:>6} {:>6} {:>10.1} {:>10.1}",
            report.games,
            elapsed * 1000.0 / f64::from(units),
            elapsed * 1000.0 / f64::from(report.games),
        );
    }
    println!(
        "\nread: ms/game on a cell shape is what sizes a gate cell — it includes the tabulation \
         and the seating loop, which a per-rung selfplay row does not."
    );

    throughput_architecture_shapes(&rules, base)?;
    Ok(())
}

/// The 3.4 shapes: the node-matched C2 cell and its C0 control, measured through
/// the same `run_cell` path at ONE unit each.
///
/// Sized at one unit deliberately — this block exists so the experiment can be
/// SIZED (spec §5 budgets ~1,300 registered units), and a tree seat at B\* is
/// three orders costlier per decision than a rung, so a throughput block that
/// ran these at the rung block's scale would dominate the whole subcommand.
///
/// This is also the only place the per-side WALL is exercised outside `cell`,
/// which is the point: §0.3's µs/decision comparison is between X and Y INSIDE
/// one cell, and a per-contestant selfplay row cannot produce it.
fn throughput_architecture_shapes(rules: &Rules, base: u64) -> Result<(), String> {
    /// The experiment's budget unit B\* (spec §5) — the shape whose cost matters.
    /// Release only, like the whole subcommand: the C2 shape at this cap is
    /// order-minutes in a debug build (a cap=24 cell measured 364 ms/game debug).
    const B_STAR: u32 = 750;

    println!("\n-- architecture contestant shapes (3.4, run_cell, invariants ON, 1 unit each) --");
    println!(
        "{:<34} {:>6} {:>10} {:>13} {:>13}",
        "cell", "games", "ms/game", "X us/dec", "Y us/dec",
    );
    let shapes: [(&str, Contestant, Contestant); 2] = [
        (
            "c2-shape-mcts-vs-capped-s1",
            Contestant::MctsV1 {
                cap: Some(B_STAR),
                c_ppm: mcts::DEFAULT_EXPLORATION_PPM,
                scale: mcts::DEFAULT_SQUASH_SCALE,
                m: greedy::DEFAULT_RETREAT_SAMPLES,
                leaf: mcts::LeafRule::default(),
            },
            Contestant::LookaheadV1 {
                b: lookahead::DEFAULT_BEAM,
                k: lookahead::DEFAULT_WORLDS,
                m: greedy::DEFAULT_RETREAT_SAMPLES,
                cap: Some(B_STAR),
            },
        ),
        ("c0-shape-capped-s1-vs-shipped-s1", {
            Contestant::LookaheadV1 {
                b: lookahead::DEFAULT_BEAM,
                k: lookahead::DEFAULT_WORLDS,
                m: greedy::DEFAULT_RETREAT_SAMPLES,
                cap: Some(B_STAR),
            }
        }, Contestant::LOOKAHEAD_V1_DEFAULT),
    ];
    for (name, x, y) in shapes {
        let spec = CellSpec {
            name: name.to_string(),
            players: 2,
            x,
            y,
            units: 1,
            master: SeedScheduleKind::Diagnostic(base),
        };
        let started = Instant::now();
        let (report, wall) = run_cell_emitting_timed(&spec, rules, true, &mut |_s, _a| Ok(()))
            .map_err(|e| format!("shape {name} failed: {e}"))?;
        let elapsed = started.elapsed().as_secs_f64();
        println!(
            "{name:<34} {:>6} {:>10.1} {:>13.1} {:>13.1}",
            report.games,
            elapsed * 1000.0 / f64::from(report.games),
            wall.x.micros_per_decision(),
            wall.y.micros_per_decision(),
        );
    }
    println!(
        "read: the two us/dec columns are §0.3's WALL axis for one cell, and the asymmetry is the \
         point — a NODE-matched cell hands the tree arm a wall advantage by construction, which \
         is what the wall cell C3 exists to re-match. Load-sensitive; never asserted."
    );
    Ok(())
}
