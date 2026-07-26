//! The PRE-CHANGE golden for the Policy arm of the arena — Phase 3.3 Task T0,
//! spec `docs/plans/2026-07-26-phase3.3-evaluator-search-spec.md` §5.1, reworked
//! at the T0 review (orchestrator fix wave) per the reviewer's must-fix.
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
//! a shift between trees. So this file captures the serialized `CellReport` of
//! two small Policy-vs-Policy diagnostic cells on the pre-widening tree — a 2P
//! PAIR-unit cell and a 4P ROTATION-BLOCK cell, because the two unit shapes
//! exercise different slot/arrangement plumbing and a re-key mis-map confined to
//! the N-seating rotation path would be invisible to a pair-only golden (T0
//! review, should-fix 1) — and asserts against them forever after.
//!
//! ## What "matches the golden" means (the comparison contract)
//!
//! The compare is a **fixture-subset structural equality**, not a whole-string
//! byte compare, for a reason the T0 review made unavoidable: T6 itself is
//! REQUIRED by spec §5.4/§5.5 to ADD fields to `CellReport` (per-seat applied
//! counts, the explanation tally) and to ADD a `setup-identical:` token to
//! `spec_echo` — a whole-report byte pin would therefore go red at T6 for
//! spec-mandated benign reasons, and its "the Policy arm moved" diagnosis would
//! be FALSE on the first day it fired. The contract instead:
//!
//! - every field captured in the fixture must exist in the produced report with
//!   an EQUAL value (recursively; a vanished or renamed field FAILS);
//! - arrays must match in exact length and element order (a grown
//!   `x_unit_shares` or `cross_checks` FAILS — additive tolerance applies to
//!   OBJECT keys only, which is where serde puts new struct fields);
//! - `spec_echo` is the one exception to value equality: it is checked as a set
//!   of pinned DELIMITED tokens (below), so an echo that GAINS a token stays
//!   green while one that loses or alters a pinned token fails;
//! - object keys present in the produced report but absent from the fixture are
//!   ignored BY DESIGN — that is precisely the sanctioned T6 delta, and new
//!   fields get their own tests at the task that adds them.
//!
//! **IF THIS TEST FAILS AFTER A CONTESTANT-SEAM CHANGE, A CAPTURED VALUE MOVED
//! OR A CAPTURED FIELD VANISHED — THE POLICY ARM'S PLAY OR TABULATION CHANGED
//! AND THE 3.2 RECORDED FIGURES ARE INVALIDATED. Fix the code. Never re-capture
//! the fixtures without an owner-visible ruling** — a re-capture silently
//! converts "the arms are unchanged" into "the arms are whatever they are now".
//! Nothing in this crate rewrites a fixture: on a mismatch the test writes the
//! produced JSON to `$CARGO_TARGET_TMPDIR` and tells you the path; copying it
//! over the committed file is a deliberate human act, not a flag.
//!
//! ## The cells
//!
//! Both `retreat-averse` (X) vs `family-weighted` at its shipped defaults (Y),
//! per-move invariants ON, committed diagnostic masters (the `A3E1` arena
//! family, serials `0033`/`0034` naming Phase 3.3; no collision with 3.2's
//! `0001`/`0005`-`0008`/`0A11`):
//!
//! - **2P, 4 pair units = 8 games**, master `0xA3E1_2026_0726_0033`;
//! - **4P, 2 rotation-block units = 8 games**, master `0xA3E1_2026_0726_0036` (serials `0034`/`0035` were probed first and produced degenerate or two-distinct-value unit vectors; `0036` is the first probe giving distinct shares and a real interval — the anti-vacuity guard below is what forced the probe) —
//!   the rotation path's slots {0..3} and block arithmetic, which the 2P cell
//!   cannot reach.
//!
//! The matchup is deliberately the two rungs whose echo shapes differ — Y is
//! KNOBBED (`family-weighted@v1{...}`), X is KNOBLESS (`retreat-averse@v1`) —
//! so §5.1's "verbatim `policy_echo` strings for one knobbed + one knobless
//! rung" ride inside the same artifacts rather than needing a second cell.
//!
//! `run_cell` is single-threaded BY CONSTRUCTION (`stats.rs` module doc), so
//! nothing here depends on `--test-threads`.
//!
//! ## Tier and cost
//!
//! PER-EDIT, never `#[ignore]`d. Measured at capture (Apple M5, quiet,
//! pre-built): the 2P cell 0.74–0.76 s debug; the 4P cell ~2.3 s debug (T0
//! reviewer's measurement, reproduced at the fix wave); whole binary ~3 s —
//! affordable against the ~14 s debug lib suite, and a golden that only ran in
//! a release gate would let the T6 re-key sit green for a whole task.
//!
//! ## Mutation-proof (run at authoring, reverted; still discriminating under
//! the subset compare)
//!
//! The move T6 can plausibly make is invisible to the echo. The compare was
//! proven against the CRN trap `stats.rs`'s own null-cell test names: keying the
//! per-seat agent stream by `X_SLOT` instead of the seating's `slot`
//! (`run_cell_emitting`'s `agent_stream_seed(seed, seat, slot)` — the exact line
//! the contestant re-key touches). The `spec_echo` came back BYTE-IDENTICAL and
//! the play moved underneath it: `x_unit_shares` `[1.0, 0.5, 0.5, 0.5]` →
//! `[0.5, 0.0, 1.0, 0.5]`, `theta_hat` 0.625 → 0.5, `vp_margin_mean` 7.125 →
//! 0.375, one end trigger `ChurchTrackFull` → `OrderDeckEmpty`. Every one of
//! those is a CAPTURED VALUE, so the subset compare fails exactly as the byte
//! compare did; the echo pins alone would not have caught it.
//!
//! ## Capture provenance
//!
//! - **Capture tree:** commit `0a6e736` (2P cell) and the T0-review fix-wave
//!   commit (4P cell) — both PRE-T6, before any contestant-seam change; engine
//!   behavior identical (`SEMANTICS_REV` 5, docs/test-only commits between).
//! - **Capture command** (per cell, no auto-update path): run
//!   `cargo test -p yokohama-core --test contestant_golden`; on mismatch the
//!   produced JSON lands in the workspace `target/tmp/` at the printed path;
//!   `cp` it over the fixture — the human act — and re-run to green.
//! - **Serialization:** `serde_json::to_string_pretty` of the `CellReport`
//!   (pretty so a mismatch diff says WHAT moved — the artifacts are ~2 KB).
//!   The compare parses both sides to `serde_json::Value`, so file-edge bytes
//!   (trailing newline) are irrelevant.
//!
//! ## On `compose_echo`
//!
//! `compose_echo` mirrors `stats.rs`'s private `policy_echo` body — it is a
//! RESTATEMENT for reachability (the formatter is private), not an independent
//! derivation, and the honest statement of where the pin's authority lives is:
//! the hand-written literals below plus the captured artifacts. If `policy_echo`
//! and `compose_echo` drift together, the literals and fixtures still fail.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use yokohama_core::{run_cell, CellSpec, IntervalOutcome, RolloutPolicy, Rules, SeedScheduleKind};

/// The 2P cell's committed diagnostic master. See the module doc.
const MASTER_2P: u64 = 0xA3E1_2026_0726_0033;
/// The 4P rotation-block cell's committed diagnostic master.
const MASTER_4P: u64 = 0xA3E1_2026_0726_0036;

const CELL_NAME_2P: &str = "t0-prewiden-golden-ra-vs-fw";
const CELL_NAME_4P: &str = "t0-prewiden-golden-ra-vs-fw-4p";

/// The committed pre-widening artifacts.
const GOLDEN_2P: &str = include_str!("fixtures/cellreport_golden_prewiden.json");
const GOLDEN_4P: &str = include_str!("fixtures/cellreport_golden_prewiden_4p.json");

/// The knobless rung's verbatim echo — no `{...}` group at all.
const ECHO_KNOBLESS: &str = "retreat-averse@v1";
/// The knobbed rung's verbatim echo at the SHIPPED defaults (BTreeMap knob
/// order — alphabetical, not declaration order).
const ECHO_KNOBBED_DEFAULT: &str = "family-weighted@v1{recovery_min_of=5,retreat_ppm=166667}";
/// The knobbed rung at EXPLICIT non-default knobs — E1's `p = 0.05` cell.
const ECHO_KNOBBED_EXPLICIT: &str = "family-weighted@v1{recovery_min_of=5,retreat_ppm=50000}";

/// The `spec_echo` tokens each golden pins, DELIMITED (leading `key=` and a
/// trailing space or end-anchored value) so `retreat-averse@v1` cannot be
/// satisfied by `retreat-averse@v11` or a knobbed variant (T0 review,
/// should-fix 3). An echo GAINING tokens (T6's `setup-identical:`) stays green;
/// losing or altering any of these fails.
fn echo_tokens(cell_name: &str, players: u8, units: u32, master: u64) -> Vec<String> {
    vec![
        format!("cell={cell_name} "),
        format!("players={players} "),
        format!("x={ECHO_KNOBLESS} "),
        format!("y={ECHO_KNOBBED_DEFAULT} "),
        format!("units={units} "),
        "games=8 ".to_string(),
        format!("master=diagnostic:{master:#x} "),
        "bootstrap_B=10000 ".to_string(),
        "single-threaded=yes ".to_string(),
        "fixed-N=yes".to_string(),
    ]
}

fn golden_spec_2p() -> CellSpec {
    CellSpec {
        name: CELL_NAME_2P.to_string(),
        players: 2,
        x: RolloutPolicy::RetreatAverse,
        y: RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        units: 4,
        master: SeedScheduleKind::Diagnostic(MASTER_2P),
    }
}

fn golden_spec_4p() -> CellSpec {
    CellSpec {
        name: CELL_NAME_4P.to_string(),
        players: 4,
        x: RolloutPolicy::RetreatAverse,
        y: RolloutPolicy::FAMILY_WEIGHTED_DEFAULT,
        units: 2,
        master: SeedScheduleKind::Diagnostic(MASTER_4P),
    }
}

/// See the module doc's `compose_echo` note: a restatement for reachability,
/// with the pin's authority in the literals + fixtures.
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
fn actual_path(stem: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("{stem}.actual.json"))
}

/// The fixture-subset structural compare (module doc: "What matches means").
/// Returns the path of the first divergence, or None on success. `spec_echo` at
/// the TOP LEVEL is skipped here (token-checked by the caller).
fn subset_diff(fixture: &Value, produced: &Value, path: &str) -> Option<String> {
    match (fixture, produced) {
        (Value::Object(f), Value::Object(p)) => {
            for (k, fv) in f {
                if path.is_empty() && k == "spec_echo" {
                    continue; // token-checked separately
                }
                let child = if path.is_empty() { k.clone() } else { format!("{path}.{k}") };
                match p.get(k) {
                    None => return Some(format!("captured field VANISHED: {child}")),
                    Some(pv) => {
                        if let Some(d) = subset_diff(fv, pv, &child) {
                            return Some(d);
                        }
                    }
                }
            }
            None
        }
        (Value::Array(f), Value::Array(p)) => {
            if f.len() != p.len() {
                return Some(format!(
                    "array length moved at {path}: captured {} vs produced {}",
                    f.len(),
                    p.len(),
                ));
            }
            for (i, (fv, pv)) in f.iter().zip(p.iter()).enumerate() {
                if let Some(d) = subset_diff(fv, pv, &format!("{path}[{i}]")) {
                    return Some(d);
                }
            }
            None
        }
        (f, p) => {
            if f == p {
                None
            } else {
                Some(format!("captured value moved at {path}: fixture {f} vs produced {p}"))
            }
        }
    }
}

/// Run one golden cell and hold it to its fixture under the comparison
/// contract. Shared by the 2P and 4P tests.
fn assert_cell_matches_golden(
    spec: &CellSpec,
    golden_str: &str,
    master: u64,
    stem: &str,
    capture_commit: &str,
) {
    let rules = Rules::standard();
    let report =
        run_cell(spec, &rules).unwrap_or_else(|e| panic!("the golden cell failed to run: {e}"));

    // Cheap structural reads FIRST, so the common failure modes (unit shape,
    // master plumbing, invariant default) report themselves in one line. None
    // of these is the authority — the subset compare below is.
    assert_eq!(report.games, 8, "both golden cells are sized to 8 games");
    assert_eq!(report.x_unit_shares.len(), spec.units as usize, "one share per design unit");
    assert_eq!(report.master, master, "the cell ran on a different master than the golden");
    for token in echo_tokens(&spec.name, spec.players, spec.units, master) {
        assert!(
            report.spec_echo.contains(&token),
            "the echo lost the pinned token {token:?}: {}",
            report.spec_echo,
        );
    }
    // A golden over a degenerate report would be stable while pinning almost
    // nothing about play: the captured cell must carry a real interval.
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
    let produced_v: Value = serde_json::from_str(&produced).expect("produced JSON parses");
    let golden_v: Value =
        serde_json::from_str(golden_str).expect("the committed fixture parses as JSON");

    // FIXTURE COMPLETENESS — the subset compare's one blind spot is a fixture
    // that captures too little (an empty `{}` placeholder passes every subset
    // walk vacuously; found live at the fix wave). The fixture must carry every
    // top-level field `CellReport` had at capture time, so a truncated or
    // placeholder fixture fails loudly and the produced JSON is still written
    // for the manual capture step.
    const CAPTURED_KEYS: [&str; 11] = [
        "spec_echo",
        "games",
        "x_unit_shares",
        "theta_hat",
        "interval",
        "cross_checks",
        "seat0_wins",
        "ties_to_seat0",
        "vp_margin_mean",
        "trigger",
        "master",
    ];
    let golden_obj = golden_v.as_object().unwrap_or_else(|| {
        let path = actual_path(stem);
        let _ = fs::write(&path, format!("{produced}\n"));
        panic!(
            "the committed fixture is not a JSON object (placeholder?) — produced JSON written \
             to {} for the manual capture step (module doc)",
            path.display(),
        );
    });
    for key in CAPTURED_KEYS {
        if !golden_obj.contains_key(key) {
            let path = actual_path(stem);
            let wrote = fs::write(&path, format!("{produced}\n"))
                .map(|()| format!("produced JSON written to {}", path.display()))
                .unwrap_or_else(|e| format!("could not write {}: {e}", path.display()));
            panic!(
                "the committed fixture is INCOMPLETE: missing captured key {key:?} \
                 (a partial fixture would make the subset compare vacuous). {wrote}",
            );
        }
    }

    if let Some(divergence) = subset_diff(&golden_v, &produced_v, "") {
        let path = actual_path(stem);
        let wrote = fs::write(&path, format!("{produced}\n"))
            .map(|()| format!("produced JSON written to {}", path.display()))
            .unwrap_or_else(|e| format!("could not write {}: {e}", path.display()));
        panic!(
            "THE POLICY ARM MOVED: {divergence}.\n\
             A value captured in the pre-widening golden (commit {capture_commit}) changed, or a \
             captured field vanished — which means the 3.2 figures recorded in map A2 no longer \
             describe this code. NOTE: purely ADDITIVE CellReport fields and ADDED spec_echo \
             tokens are invisible to this test BY DESIGN (T6's sanctioned delta) — if you are \
             seeing this, the change was NOT additive. Fix the code; do NOT re-capture the \
             fixture without an owner-visible ruling (module doc). {wrote}",
        );
    }
}

/// §5.1: the pre-widening 2P pair-unit `CellReport`, every captured field exact.
#[test]
fn the_policy_arm_cell_report_matches_the_prewiden_golden() {
    assert_cell_matches_golden(
        &golden_spec_2p(),
        GOLDEN_2P,
        MASTER_2P,
        "cellreport_golden_prewiden",
        "0a6e736",
    );
}

/// The 4P ROTATION-BLOCK companion (T0 review, should-fix 1): the rotation
/// path's slots {0..3} and block arithmetic are different plumbing from the 2P
/// pair, and a re-key mis-map confined to them would pass the 2P golden
/// silently. Same contract, own fixture and master.
#[test]
fn the_policy_arm_4p_rotation_cell_matches_the_prewiden_golden() {
    assert_cell_matches_golden(
        &golden_spec_4p(),
        GOLDEN_4P,
        MASTER_4P,
        "cellreport_golden_prewiden_4p",
        "the T0-review fix-wave commit",
    );
}

/// §5.1's second half: the verbatim `policy_echo` strings for one knobbed and
/// one knobless rung.
///
/// Plays no games — it reads the committed artifacts and the public
/// `descriptor()` surface; the strings must hold for the explicitly-knobbed
/// rung the golden cells do not contain.
#[test]
fn the_policy_echo_strings_are_pinned_verbatim() {
    // Composed from the public descriptor, matched against the literal.
    assert_eq!(compose_echo(RolloutPolicy::RetreatAverse), ECHO_KNOBLESS);
    assert_eq!(compose_echo(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT), ECHO_KNOBBED_DEFAULT);
    assert_eq!(
        compose_echo(RolloutPolicy::FamilyWeighted { retreat_ppm: 50_000, recovery_min_of: 5 }),
        ECHO_KNOBBED_EXPLICIT,
    );
    assert_eq!(compose_echo(RolloutPolicy::Uniform), "uniform@v1");

    // The same strings as they were actually SERIALIZED on the capture tree,
    // DELIMITED so a knobbed variant or a `@v11` cannot satisfy the knobless
    // check (T0 review, should-fix 3). None of these characters is
    // JSON-escaped, so the substring is present in the artifact verbatim.
    for golden in [GOLDEN_2P, GOLDEN_4P] {
        assert!(
            golden.contains(&format!("x={ECHO_KNOBLESS} ")),
            "a golden lost the knobless rung's delimited echo",
        );
        assert!(
            golden.contains(&format!("y={ECHO_KNOBBED_DEFAULT} ")),
            "a golden lost the knobbed rung's delimited echo",
        );
    }

    // ANTI-VACUITY: pin the distinction the knob group IS. A knobbed echo must
    // carry its brace group; a knobless one must not.
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

    // The cells' own names reach the echoes — a corpus path and a quoted figure
    // are keyed by them.
    assert!(GOLDEN_2P.contains(&format!("cell={CELL_NAME_2P} ")), "the 2P golden lost its name");
    assert!(GOLDEN_4P.contains(&format!("cell={CELL_NAME_4P} ")), "the 4P golden lost its name");
}
