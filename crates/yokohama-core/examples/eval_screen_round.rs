//! `eval_screen_round` — the committed ROUND SCRIPT for the evaluator-v2
//! protocol's §4.2 **L4** screen
//! (`docs/plans/2026-08-09-phase3.5c-evaluator-v2-protocol.md`).
//!
//! One command per round, run at BOTH rungs by default: the DRAFT table (the
//! iteration candidate) against the SHIPPED table (the incumbent), 2P, over
//! swapped-pair design units, through the honest driver, tabulated by
//! `eval_screen`'s harness. Since the V3 mini-cycle's adoption commit
//! (2026-08-19) those are `WeightTable::V3_DRAFT` and `WeightTable::V2`; through
//! the v2 rounds they were `V2_DRAFT` and `V1`, and the arm move is exactly what
//! "a round that wants a different candidate moves the draft in a commit"
//! means — the arms follow the cycle, the binary does not gain an argument.
//!
//! ```text
//! cargo run --release -p yokohama-core --example eval_screen_round -- \
//!     --master 0x3EE753CE7F4052C4
//! cargo run --release -p yokohama-core --example eval_screen_round -- \
//!     --rung lookahead --units 200 --master 0x3EE753CE7F4052C4
//! ```
//!
//! # THE v2 ROUNDS AND THEIR FREEZE (2026-08-10) — history, and how to re-read it
//!
//! Through rounds 1–3 the arms were `V2_DRAFT` against `V1`. The freeze closed
//! that loop and made `V2_DRAFT` an ALIAS of `V2`, so the binary ran candidate
//! `V2` against incumbent `V1` — the freeze's own **criteria 1–2**:
//!
//! ```text
//! cargo run --release -p yokohama-core --example eval_screen_round -- \
//!     --master 0xF524FFB9D041DCAE
//! ```
//!
//! **What that command reproduces, and what it does NOT.** Criteria 1–2 were run
//! PRE-COMMIT (freeze pre-registration §2), when nothing called T3's post-claim
//! filter. Re-running the command now measures the same two TABLES with the
//! filter active on BOTH arms, and prints `eval_impl=3` where the recorded
//! blocks print 2 — so it is a different measurement, not a reproduction, and
//! the recorded criteria figures (lookahead θ̂ 0.6775 [0.6275, 0.7250] above the
//! null; greedy θ̂ 0.4825 [0.4325, 0.5300] spanning it) belong to the pre-commit
//! tree.
//!
//! **And since the V3 arm move that command no longer runs those arms at all**
//! — it now screens `V3_DRAFT` against `V2`. Reproducing a v2-era block means
//! checking out the commit it names; the constants `V2_DRAFT` and `V1` are both
//! still here, retained verbatim, so that checkout parses and runs. This is the
//! tables' own retention contract applied to a binary: a recorded figure stays a
//! record of the arms it measured, never of the arms that came after.
//!
//! # THERE IS NO TABLE ARGUMENT, AND THAT IS THE DESIGN
//!
//! The candidate is `V3_DRAFT` and the incumbent is `V2`, both hardcoded, as is
//! the 2P player count. `game::eval_screen`'s module doc fixes the rule this
//! binary obeys — the weighted constructors are "called from tests and round
//! scripts — never from argv, never from the CLI, never from a gate" — because
//! `Contestant` carries no weight knob by L12 design, and an argv table here
//! would smuggle the same unversioned-weights-under-a-versioned-name confusion
//! in through a different door. A round that wants a DIFFERENT candidate moves
//! the draft const in a commit, which is what an adoption IS.
//!
//! # THERE IS NO `--os-entropy` ARM EITHER
//!
//! Protocol §4.3's flow draws the round's master from OS entropy AT ROUND OPEN
//! and records it in the ledger row BEFORE any screen runs — and never redraws
//! it after seeing a result, which is the pre-commitment that blocks
//! master-shopping. So this binary takes the RECORDED master and requires it:
//! drawing one here would put the draw after the decision to run, in a process
//! whose output is the thing the draw is supposed to be independent of. Every
//! screen therefore runs `Diagnostic(master)` and reproduces byte for byte; the
//! ledger row, not this file, is the draw's provenance.
//!
//! # WHAT IT PRINTS
//!
//! `ScreenReport::render()` — the full provenance block, the estimate, the
//! routing pin, the counters — followed by three labelled verdict lines:
//! the §4.2 classification, the §7 criterion-1 shape, and the §4.2 step-4
//! adoption clause. Nothing is asserted: this is an instrument, and the screen
//! is a REGRESSION NET rather than the positive evidence (§4.2 again — the
//! positive case is L1–L3's job, and a near-null screen does not block a
//! corpus-justified re-pricing).
//!
//! # NO TESTS LIVE HERE
//!
//! An example's `#[cfg(test)]` module runs under NO command this repo
//! documents unless that command is written down at four doc sites (the
//! `eval_probe` precedent). The logic worth testing is `game::eval_screen`'s
//! and is already tested there, per-edit; anything this file grows that needs a
//! test belongs in that module instead of here.

use yokohama_core::game::eval::WeightTable;
use yokohama_core::game::eval_screen::{
    hand_split_ordering_holds, run_screen, run_screen_with, ScreenReport, ScreenRung, ScreenSpec,
    GREEDY_PAUSE_LINE,
};
use yokohama_core::game::greedy::weight_table_identity;
use yokohama_core::{Rules, SeedScheduleKind};

const USAGE: &str = "\
usage: eval_screen_round --master <u64> [--rung greedy|lookahead|both]
                         [--units <n>] [--no-invariants]

  Runs the protocol §4.2 L4 screen for ONE round: WeightTable::V3_DRAFT (the
  candidate) against WeightTable::V2 (the incumbent), 2P, at the shipped knobs
  of each rung, and prints each cell's report plus its three verdict lines.

  --master <u64>   REQUIRED. Decimal, or hex with a 0x prefix; `_` separators
                   are ignored. This is the round's OS-entropy master, DRAWN AND
                   RECORDED IN THE LEDGER ROW BEFORE the screen runs (§4.3) and
                   never redrawn after seeing a result — which is why this binary
                   has no --os-entropy arm to draw one with.
  --rung <name>    greedy | lookahead | both   (default both: a round reports
                   both, since greedy is a preserved rung whose regression the
                   owner is entitled to see even though lookahead is what ships)
  --units <n>      DESIGN units, not games (default 200 — the protocol's N). A 2P
                   unit is the swapped pair, so a cell plays 2n games.
  --no-invariants  Turn per-move check_invariants OFF (default ON). The report
                   echoes which, so an invariants-off screen cannot impersonate
                   an invariants-on one in an artifact.

  There is NO weight-table argument, by design: eval_screen's constructors are
  for tests and round scripts and never for argv (L12 — a weight change is a
  version bump, not a cell parameter). Moving the candidate IS a commit.
";

/// Protocol §4.2's N for a round's screen.
const DEFAULT_UNITS: u32 = 200;

/// 2P for every screen the protocol registers (`ScreenSpec::players`' own doc:
/// 3–4P runs, but nothing has measured what a multi-table field means there).
const PLAYERS: u8 = 2;

fn main() {
    if let Err(message) = run(&std::env::args().skip(1).collect::<Vec<String>>()) {
        eprintln!("{message}");
        std::process::exit(2);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let mut master: Option<u64> = None;
    let mut rungs: Vec<ScreenRung> = vec![ScreenRung::Greedy, ScreenRung::Lookahead];
    let mut units = DEFAULT_UNITS;
    let mut invariants = true;

    let mut i = 0;
    while i < args.len() {
        // An unknown flag is an ERROR, never a silent no-op — `eval_probe`'s
        // rule and `examples/arena`'s before it: a mistyped `--unit 20` that
        // quietly ran 200 units would burn a round's master on a cell nobody
        // asked for, and §4.3 forbids drawing another.
        let value = |i: usize, name: &str| -> Result<&String, String> {
            args.get(i + 1).ok_or_else(|| format!("--{name} needs a value\n\n{USAGE}"))
        };
        match args[i].as_str() {
            "--master" => {
                master = Some(parse_u64(value(i, "master")?)?);
                i += 2;
            }
            "--rung" => {
                rungs = match value(i, "rung")?.as_str() {
                    "greedy" => vec![ScreenRung::Greedy],
                    "lookahead" => vec![ScreenRung::Lookahead],
                    "both" => vec![ScreenRung::Greedy, ScreenRung::Lookahead],
                    other => {
                        return Err(format!("unknown --rung {other:?} (greedy | lookahead | both)"))
                    }
                };
                i += 2;
            }
            "--units" => {
                units = value(i, "units")?
                    .parse::<u32>()
                    .map_err(|e| format!("--units: {e}"))?;
                i += 2;
            }
            "--no-invariants" => {
                invariants = false;
                i += 1;
            }
            other => return Err(format!("unknown argument {other:?}\n\n{USAGE}")),
        }
    }

    let master = master.ok_or_else(|| {
        format!(
            "--master is REQUIRED: protocol §4.3 draws the round's master at round open and \
             records it in the ledger row BEFORE the screen runs, so this binary takes the \
             recorded value rather than drawing one\n\n{USAGE}"
        )
    })?;

    let candidate = WeightTable::V3_DRAFT;
    let incumbent = WeightTable::V2;
    let rules = Rules::standard();

    println!("=== eval_screen_round ===");
    println!(
        "  candidate=WeightTable::V3_DRAFT {:#018x}   incumbent=WeightTable::V2 {:#018x}",
        weight_table_identity(&candidate) as u64,
        weight_table_identity(&incumbent) as u64,
    );
    println!(
        "  players={PLAYERS} units={units} invariants={} master=diagnostic:{master:#018x} \
         (RECORDED at round open per §4.3, never redrawn)",
        if invariants { "on" } else { "OFF" },
    );
    // RECORDED beside the figure, exactly as `ScreenReport` records it, because
    // the ladder's L2 asserts this constraint and a screen has no authority to
    // refuse — but a violating candidate must never be screened silently.
    println!(
        "  hand-split ordering (L14 core>on_board>excess, both vectors): candidate={} incumbent={}",
        verdict(hand_split_ordering_holds(&candidate)),
        verdict(hand_split_ordering_holds(&incumbent)),
    );
    if weight_table_identity(&candidate) == weight_table_identity(&incumbent) {
        println!(
            "  *** NULL SCREEN: the two identity digests are EQUAL, so the draft still holds \
             the incumbent's numbers and every cell below measures the INSTRUMENT rather than a \
             candidate. That is the initialized state — a round's adoption commit is what moves \
             the draft — so read these figures as an incumbent-vs-itself null, never as \
             evidence about a re-pricing."
        );
    }

    for rung in rungs {
        let spec = ScreenSpec {
            name: match rung {
                ScreenRung::Greedy => "round-greedy".to_string(),
                ScreenRung::Lookahead => "round-lookahead".to_string(),
            },
            rung,
            candidate,
            incumbent,
            players: PLAYERS,
            units,
            master: SeedScheduleKind::Diagnostic(master),
        };
        let report = if invariants {
            run_screen(&spec, &rules)
        } else {
            run_screen_with(&spec, &rules, false)
        }
        .map_err(|e| format!("screen {:?}: {e}", spec.name))?;

        println!();
        println!("{}", report.render());
        print_verdicts(&report);
    }
    Ok(())
}

/// The clauses a round's ledger row quotes, each labelled with the rule it
/// belongs to rather than left as a bare boolean. The greedy pause line gets a
/// COMPUTED verdict on greedy reports (Task-D review SHOULD-4: a clause named
/// in output with no verdict is a clause an operator has to eyeball); on the
/// lookahead rung the clause does not apply and no line prints.
fn print_verdicts(report: &ScreenReport) {
    println!(
        "  VERDICT position          = {:?}   (§4.2's classification against the {:.4} null; a \
         refusal classifies all-won ABOVE, all-lost BELOW)",
        report.position, report.null,
    );
    println!(
        "  VERDICT direction_above   = {}   (§7 criterion-1 shape: the interval entirely above \
         the null, OR an all-won refusal whose rule-of-three bound clears it)",
        report.direction_above(),
    );
    println!(
        "  VERDICT entirely_below    = {}   (§4.2 step-4 adoption clause: adopt only if this is \
         FALSE at the lookahead rung)",
        report.entirely_below_null(),
    );
    if report.provenance.rung == ScreenRung::Greedy {
        println!(
            "  VERDICT greedy_pause      = {}   (§4.2 step-4(c): interval entirely below the \
             HAND-SET {GREEDY_PAUSE_LINE} line pauses the round for recorded reconsideration — \
             a pause, never a block)",
            report.greedy_pause_triggered(),
        );
    }
}

fn verdict(ok: bool) -> &'static str {
    if ok {
        "ok"
    } else {
        "VIOLATED"
    }
}

/// `eval_probe`'s spelling, so a master typed for one instrument means the same
/// thing typed for the other.
fn parse_u64(raw: &str) -> Result<u64, String> {
    let cleaned = raw.replace('_', "");
    let parsed = match cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => cleaned.parse::<u64>(),
    };
    parsed.map_err(|e| format!("{raw:?} is not a u64: {e}"))
}
