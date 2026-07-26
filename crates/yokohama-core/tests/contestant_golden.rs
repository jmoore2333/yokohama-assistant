//! The PRE-CHANGE golden for the Policy arm of the arena — Phase 3.3 Task T0,
//! spec `docs/plans/2026-07-26-phase3.3-evaluator-search-spec.md` §5.1.
//!
//! WHY THIS EXISTS, AND WHY IT HAD TO BE CAPTURED BEFORE T6. Task T6 re-keys
//! [`CellSpec`] from `x: RolloutPolicy` onto a wider `Contestant` enum
//! (`Policy` / `Greedy` / `Lookahead` / `AtomicDiag`). The claim that has to
//! survive that re-key is "the Policy arms behave byte-identically" — because
//! every 3.2 figure recorded in map §11 A2 (θ̂ 0.4725 on the RA-vs-FW edge cell,
//! the 0.6725 knob edge, the null cell, the bracket refusal) was measured
//! through the Policy path and is quoted as still true afterwards.
//!
//! That claim is **unverifiable by construction without a pre-change artifact**.
//! `stats.rs`'s `same_master_reproduces_the_report_byte_identically` runs the
//! same spec twice on ONE tree; it proves determinism and is completely blind to
//! a shift between trees. So this file captures the serialized `CellReport` of a
//! small Policy-vs-Policy diagnostic cell on the pre-widening tree and asserts
//! equality against it forever after. Today it is green by construction. Its
//! whole value is the day it goes red.
//!
//! **IF THIS TEST FAILS AFTER A CONTESTANT-SEAM CHANGE, THE POLICY ARM'S
//! BEHAVIOR MOVED AND THE 3.2 RECORDED FIGURES ARE INVALIDATED. Fix the code.
//! Never re-capture the fixture without an owner-visible ruling** — a re-capture
//! silently converts "the arms are unchanged" into "the arms are whatever they
//! are now", which is the one thing this file is here to prevent. Nothing in
//! this crate rewrites the fixture: on a mismatch the test writes the produced
//! JSON to `$CARGO_TARGET_TMPDIR` and tells you the path, and copying it over
//! the committed file is a deliberate human act, not a flag.
//!
//! ## The cell
//!
//! `retreat-averse` (X) vs `family-weighted` at its shipped defaults (Y), 2P,
//! 4 design units = 8 games, per-move invariants ON, on the committed diagnostic
//! master `0xA3E1_2026_0726_0033` (the `A3E1` arena family; serial `0033` names
//! Phase 3.3, and does not collide with 3.2's `0001`/`0005`-`0008`/`0A11`). The
//! master is `Diagnostic` for the reason `game::seeds` states in the
//! constructor's own name: a regression fixture wants reproducibility, and an
//! OS-entropy golden is a contradiction in terms.
//!
//! The matchup is deliberately the two rungs whose echo shapes differ — Y is
//! KNOBBED (`family-weighted@v1{...}`), X is KNOBLESS (`retreat-averse@v1`) —
//! so §5.1's "verbatim `policy_echo` strings for one knobbed + one knobless
//! rung" ride inside the same artifact rather than needing a second cell.
//!
//! `run_cell` is single-threaded BY CONSTRUCTION (`stats.rs` module doc), so
//! nothing here depends on `--test-threads`; the test harness may run this file
//! concurrently with others and the report is unaffected.
//!
//! ## Tier and cost
//!
//! PER-EDIT, never `#[ignore]`d. Measured at capture: **0.74 s / 0.76 s / 0.74 s
//! debug** across three runs of the whole binary (8 games at ~0.07 s/game debug
//! with per-move invariants on, plus a 10,000-replicate bootstrap over 4 units;
//! `cargo test -p yokohama-core --test contestant_golden`, Apple M5, quiet
//! machine, pre-built). That is off the debug suite's ~11 s critical path. A
//! golden that only ran in a release gate would let the T6 re-key sit green for
//! a whole task, which is why the cell is sized to afford the per-edit tier.
//!
//! ## Mutation-proof (run at authoring, reverted)
//!
//! A golden is worthless if it cannot see the move it exists to catch, and the
//! move T6 can plausibly make is invisible to the echo. So the byte compare was
//! proven against the CRN trap `stats.rs`'s own null-cell test names: keying the
//! per-seat agent stream by `X_SLOT` instead of the seating's `slot`
//! (`run_cell_emitting`'s `agent_stream_seed(seed, seat, slot)` — the exact line
//! the contestant re-key touches). The `spec_echo` came back BYTE-IDENTICAL and
//! the play moved underneath it: `x_unit_shares` `[1.0, 0.5, 0.5, 0.5]` →
//! `[0.5, 0.0, 1.0, 0.5]`, `theta_hat` 0.625 → 0.5, `vp_margin_mean` 7.125 →
//! 0.375, and one game's end trigger flipped `ChurchTrackFull` → `OrderDeckEmpty`.
//! The byte compare failed; the echo pins alone would not have. Mutation
//! reverted by restoring the file (empty diff verified).
//!
//! ## Capture provenance
//!
//! - **Capture tree:** commit `0a6e736` (branch `phase3.3-evaluator`), the T0
//!   HEAD — i.e. before ANY Phase-3.3 behavior change. `SEMANTICS_REV` 5.
//! - **Capture command** (two steps, no auto-update path):
//!   ```text
//!   cargo test -p yokohama-core --test contestant_golden
//!   cp target/tmp/cellreport_golden_prewiden.actual.json \
//!      crates/yokohama-core/tests/fixtures/cellreport_golden_prewiden.json
//!   ```
//!   Step 1 fails against a stale/placeholder fixture and leaves the produced
//!   JSON at the path it prints (`$CARGO_TARGET_TMPDIR` = the workspace
//!   `target/tmp`); step 2 is the human act. Re-running step 1 then passes.
//! - **Serialization:** `serde_json::to_string_pretty` of the `CellReport`.
//!   Pretty rather than compact deliberately: the artifact's job at T6 is to say
//!   WHAT moved, and a diff against one 4 KB line says nothing. `CellReport`
//!   derives `Serialize` only, so the comparison is string-vs-string; the
//!   fixture's single trailing newline is the only byte trimmed, and it is
//!   trimmed from the FIXTURE side so no produced byte is ever discarded.

use std::fs;
use std::path::PathBuf;

use yokohama_core::{run_cell, CellSpec, IntervalOutcome, RolloutPolicy, Rules, SeedScheduleKind};

/// This task's committed diagnostic master. See the module doc.
const MASTER: u64 = 0xA3E1_2026_0726_0033;

/// The cell name, which lands in `spec_echo` and (for emitted corpora) in the
/// record's `run_id`. Fixed here because it is part of the golden bytes.
const CELL_NAME: &str = "t0-prewiden-golden-ra-vs-fw";

/// Design units — pairs at 2P, so 8 games. Small on purpose: this is a per-edit
/// artifact, and a golden's discriminating power comes from being byte-exact,
/// not from N.
const UNITS: u32 = 4;

/// The committed pre-widening artifact.
const GOLDEN: &str = include_str!("fixtures/cellreport_golden_prewiden.json");

/// The knobless rung's verbatim echo — no `{...}` group at all.
const ECHO_KNOBLESS: &str = "retreat-averse@v1";

/// The knobbed rung's verbatim echo at the SHIPPED defaults. Knob order is
/// `BTreeMap` order (alphabetical), not declaration order, which is exactly the
/// kind of detail a re-key can perturb without anyone noticing.
const ECHO_KNOBBED_DEFAULT: &str = "family-weighted@v1{recovery_min_of=5,retreat_ppm=166667}";

/// The knobbed rung at EXPLICIT non-default knobs — E1's `p = 0.05` cell, which
/// `arena_gate.rs` uses as half of its guaranteed edge. Pinned here as well
/// because the golden cell cannot contain it, and "one knobbed rung" is only
/// convincing if a knob value actually reaches the string.
const ECHO_KNOBBED_EXPLICIT: &str = "family-weighted@v1{recovery_min_of=5,retreat_ppm=50000}";

/// The cell under golden. Kept as a function so both the report and any future
/// reader see one definition of what was captured.
fn golden_spec() -> CellSpec {
    CellSpec {
        name: CELL_NAME.to_string(),
        players: 2,
        x: RolloutPolicy::RetreatAverse,
        y: RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        units: UNITS,
        master: SeedScheduleKind::Diagnostic(MASTER),
    }
}

/// A DELIBERATE independent restatement of `stats.rs`'s private `policy_echo`,
/// in the spirit of `arena_gate.rs`'s independently-restated `Z_95`: this test
/// is stronger for composing the echo from the PUBLIC `descriptor()` surface
/// than it would be for importing the private formatter it is checking. The
/// composed string is then required to match a hand-written literal AND to
/// appear verbatim in the committed golden, so the two halves cannot drift
/// together.
fn compose_echo(policy: RolloutPolicy) -> String {
    let (name, version, params) = policy.descriptor();
    if params.is_empty() {
        return format!("{name}@v{version}");
    }
    let knobs: Vec<String> = params.iter().map(|(k, v)| format!("{k}={v}")).collect();
    format!("{name}@v{version}{{{}}}", knobs.join(","))
}

/// Where a mismatch drops the produced JSON. Not a re-capture switch — the
/// copy is manual, per the module doc.
fn actual_path() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("cellreport_golden_prewiden.actual.json")
}

/// §5.1: the pre-widening `CellReport`, byte for byte.
#[test]
fn the_policy_arm_cell_report_matches_the_prewiden_golden() {
    let rules = Rules::standard();
    let report = run_cell(&golden_spec(), &rules)
        .unwrap_or_else(|e| panic!("the golden cell failed to run: {e}"));

    // Cheap structural reads FIRST, so the common failure modes (someone
    // changed the unit shape, the master plumbing, or the invariant default)
    // report themselves in one line instead of as a 4 KB string diff. None of
    // these is the authority — the byte compare below is.
    assert_eq!(report.games, 8, "4 pairs is 8 games at 2P");
    assert_eq!(report.x_unit_shares.len(), UNITS as usize, "one share per design unit");
    assert_eq!(report.master, MASTER, "the cell ran on a different master than the golden");
    assert!(
        report.spec_echo.contains(ECHO_KNOBLESS) && report.spec_echo.contains(ECHO_KNOBBED_DEFAULT),
        "the echo lost a rung's verbatim descriptor: {}",
        report.spec_echo,
    );
    // A golden over a degenerate report would still be byte-stable while
    // pinning almost nothing about play, so the captured cell is required to be
    // the informative shape: a real interval over units that are not all equal.
    match report.interval {
        IntervalOutcome::Bootstrap { lo, hi, .. } => assert!(
            hi > lo,
            "the golden cell must carry a real interval, got [{lo}, {hi}]",
        ),
        ref other => panic!(
            "the golden cell degenerated to a refusal ({other:?}) — recapture the CELL DESIGN \
             (a constant unit vector pins no play), not the fixture",
        ),
    }

    let produced = serde_json::to_string_pretty(&report).expect("CellReport serializes");

    // The fixture's trailing newline is the file's, not the report's; trim it
    // from the GOLDEN side only, so no produced byte is ever discarded.
    let golden = GOLDEN.trim_end_matches('\n');
    if produced != golden {
        let path = actual_path();
        let wrote = fs::write(&path, format!("{produced}\n"))
            .map(|()| format!("produced JSON written to {}", path.display()))
            .unwrap_or_else(|e| format!("could not write {}: {e}", path.display()));
        panic!(
            "THE POLICY ARM MOVED. This report differs from the pre-widening golden captured at \
             commit 0a6e736, which means the 3.2 figures recorded in map A2 no longer describe \
             this code. Fix the code; do NOT re-capture the fixture without an owner-visible \
             ruling (see this file's module doc). {wrote}\n\n\
             produced {} bytes, golden {} bytes",
            produced.len(),
            golden.len(),
        );
    }
}

/// §5.1's second half: the verbatim `policy_echo` strings for one knobbed and
/// one knobless rung.
///
/// Plays no games — it reads the committed artifact and the public
/// `descriptor()` surface. That is on purpose: an echo pin that had to replay
/// the cell would be paying twice for one fact, and the strings must hold for
/// the explicitly-knobbed rung the golden cell does not contain.
#[test]
fn the_policy_echo_strings_are_pinned_verbatim() {
    // Composed from the public descriptor, matched against the literal.
    assert_eq!(compose_echo(RolloutPolicy::RetreatAverse), ECHO_KNOBLESS);
    assert_eq!(compose_echo(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT), ECHO_KNOBBED_DEFAULT);
    assert_eq!(
        compose_echo(RolloutPolicy::FamilyWeighted { retreat_ppm: 50_000, recovery_min_of: 5 }),
        ECHO_KNOBBED_EXPLICIT,
    );
    // The third rung, for completeness of the knobless family.
    assert_eq!(compose_echo(RolloutPolicy::Uniform), "uniform@v1");

    // And the same literals as they were actually SERIALIZED on the capture
    // tree. None of these characters is JSON-escaped, so the substring is
    // present in the artifact verbatim.
    assert!(
        GOLDEN.contains(ECHO_KNOBLESS),
        "the golden lost the knobless rung's echo {ECHO_KNOBLESS:?}",
    );
    assert!(
        GOLDEN.contains(ECHO_KNOBBED_DEFAULT),
        "the golden lost the knobbed rung's echo {ECHO_KNOBBED_DEFAULT:?}",
    );

    // ANTI-VACUITY: `contains` on a literal proves nothing if the two shapes
    // are indistinguishable, so pin the distinction the knob group IS. A
    // knobbed echo must carry its brace group; a knobless one must not.
    assert!(!ECHO_KNOBLESS.contains('{'), "a knobless rung must print no knob group");
    assert!(
        ECHO_KNOBBED_DEFAULT.ends_with('}') && ECHO_KNOBBED_DEFAULT.contains("retreat_ppm="),
        "a knobbed rung must print its knob values, not just its name",
    );
    // Knob order is BTreeMap (alphabetical) order, which is what makes the
    // string stable across a struct-field reorder.
    assert!(
        ECHO_KNOBBED_DEFAULT.find("recovery_min_of").unwrap()
            < ECHO_KNOBBED_DEFAULT.find("retreat_ppm").unwrap(),
        "knobs print in BTreeMap order",
    );

    // The cell's own name reaches the echo too — it is what a corpus path and a
    // quoted figure are keyed by.
    assert!(GOLDEN.contains(&format!("cell={CELL_NAME}")), "the golden lost its cell name");
}
