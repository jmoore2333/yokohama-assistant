//! The PRE-CHANGE golden for the Policy arm of the arena — Phase 3.3 Task T0,
//! spec `docs/plans/2026-07-26-phase3.3-evaluator-search-spec.md` §5.1, reworked
//! at the T0 review (orchestrator fix wave) per the reviewer's must-fix —
//! EXTENDED at Phase 3.4 Task T0 (spec
//! `docs/plans/2026-08-04-phase3.4-search-arch-spec.md` §6) with three fresh
//! pre-change goldens captured on the pre-3.4 tree; see the "Phase 3.4 T0"
//! section below. The 3.3 goldens and their tests stand untouched beside them.
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
//! OR A CAPTURED FIELD VANISHED — THE ARM'S PLAY OR TABULATION CHANGED AND THE
//! RECORDED FIGURES MEASURED THROUGH IT ARE INVALIDATED. Fix the code. Never
//! re-capture the fixtures without an owner-visible ruling** — a re-capture
//! silently converts "the arms are unchanged" into "the arms are whatever they
//! are now". Nothing in this crate rewrites a fixture: on a mismatch the test
//! writes the produced JSON to `$CARGO_TARGET_TMPDIR` and tells you the path;
//! copying it over the committed file is a deliberate human act, not a flag.
//!
//! ## The 3.3 cells
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
//! ## Phase 3.4 T0 — fresh pre-change goldens (spec 2026-08-04 §6)
//!
//! Phase 3.4's T5 widens the seam again (`Contestant::MctsV1`, a `cap` knob on
//! `LookaheadV1`, new `CellReport` budget meters). The 3.3 fixtures cannot
//! carry that claim alone, for the reason lens 2 S10 names: their captured-key
//! list is the 3.3-era ELEVEN keys, which predate the per-seat / explanation /
//! cap-counter fields — and produced-only keys are ignored by design, so a
//! T5 defect confined to `applied_by_seat`, `x_applied_moves`,
//! `x_explanations`, `x_ply_cap_hits` or `x_free_cap_hits` would pass every
//! 3.3 golden silently. So T0 captures THREE fresh goldens on the pre-3.4 tree
//! (commit `524c126`, `SEMANTICS_REV` 5) with [`CAPTURED_KEYS_CURRENT`]
//! widened to every field `CellReport` carried at CAPTURE time (all sixteen,
//! pre-seam; the T5 additions are produced-only keys these fixtures tolerate
//! by design — six-lens N5):
//!
//! - **2P Policy pair cell**: `retreat-averse` vs `family-weighted` default,
//!   4 pair units = 8 games — same matchup as 3.3 (one knobless + one knobbed
//!   echo in one artifact), new master;
//! - **4P Policy rotation-block cell**: same matchup, 2 rotation blocks =
//!   8 games — the slots-{0..3} plumbing;
//! - **one EVALUATOR cell**: `lookahead-v1` (X) vs `greedy-v1` (Y) at shipped
//!   knobs, 2P, 2 pair units = 4 games — small N because a Lookahead seat is
//!   ~three orders costlier per decision than a rung; this is the cell that
//!   makes the five 3.3-era fields non-null in a fixture, which no Policy cell
//!   can (`x_explanations` et al. are `None` BY RULE for a rung, and the new
//!   fixtures pin those `null`s too).
//!
//! Masters: the `A3E1` arena family, date field `2026_0804`, serials from
//! `0001` — no collision with 3.2/3.3's `0726_*`/`0727_*` literals (the date
//! field differs, so the serial space restarts cleanly; verified by grep over
//! every `0xA3E1` literal in the crate at capture time).
//!
//! THE EVALUATOR CELL'S INTERVAL IS EXPECTED DEGENERATE, and that is honest:
//! Lookahead-v1 swept Greedy-v1 100/100 units at the 3.3 evaluator gate's
//! lattice B, so at any typical master every unit reads 1.0 and the interval is
//! the all-won zero-width refusal. Probing masters for a "real interval" here
//! would be a search for an ATYPICAL master and would misrepresent the arm the
//! golden exists to freeze. The pinning authority for this cell is therefore
//! NOT the share vector — it is the play-sensitive captured values
//! (`vp_margin_mean`, `applied_by_seat`, `x_applied_moves`, `x_explanations`,
//! `trigger`, `seat0_wins`, the exact refusal bound), every one of which the
//! subset compare holds exactly; the anti-vacuity guard for this cell asserts
//! the refusal SHAPE plus nonzero play instead of a real interval.
//!
//! The comparison contract, the mismatch workflow and the mutation-proof below
//! carry over UNCHANGED — the 3.4 tests reuse the same `subset_diff` and the
//! same completeness check, parameterized only by the key list, the echo
//! tokens and the interval expectation.
//!
//! ## Tier and cost
//!
//! PER-EDIT, never `#[ignore]`d. Measured at 3.3 capture (Apple M5, quiet,
//! pre-built): the 2P cell 0.74–0.76 s debug; the 4P cell ~2.3 s debug (T0
//! reviewer's measurement, reproduced at the fix wave); whole binary ~3 s —
//! affordable against the ~14 s debug lib suite, and a golden that only ran in
//! a release gate would let a seam re-key sit green for a whole task. The 3.4
//! additions re-measured at their capture (Apple M5, quiet, debug, pre-built):
//! all four Policy cells ~3.2 s wall together; the evaluator cell **13.1 s
//! alone** — a Lookahead seat per game is the cost driver, which is why it is
//! sized at 2 units — and the whole six-test binary ~13.5 s wall (tests run
//! threaded, so the evaluator cell IS the wall).
//!
//! ## Mutation-proof (run at 3.3 authoring, reverted; still discriminating
//! under the subset compare)
//!
//! The move a seam re-key can plausibly make is invisible to the echo. The
//! compare was proven against the CRN trap `stats.rs`'s own null-cell test
//! names: keying the per-seat agent stream by `X_SLOT` instead of the seating's
//! `slot` (`run_cell_emitting`'s `agent_stream_seed(seed, seat, slot)` — the
//! exact line a contestant re-key touches). The `spec_echo` came back
//! BYTE-IDENTICAL and the play moved underneath it: `x_unit_shares`
//! `[1.0, 0.5, 0.5, 0.5]` → `[0.5, 0.0, 1.0, 0.5]`, `theta_hat` 0.625 → 0.5,
//! `vp_margin_mean` 7.125 → 0.375, one end trigger `ChurchTrackFull` →
//! `OrderDeckEmpty`. Every one of those is a CAPTURED VALUE, so the subset
//! compare fails exactly as the byte compare did; the echo pins alone would not
//! have caught it. The 3.4 goldens ride the SAME compare machinery, so the
//! proof transfers; the widened key list only ADDS discriminating fields.
//!
//! ## Capture provenance
//!
//! - **Capture tree (3.3 fixtures):** commit `0a6e736` (2P cell) and the
//!   T0-review fix-wave commit (4P cell) — both PRE-T6, before any
//!   contestant-seam change; engine behavior identical (`SEMANTICS_REV` 5,
//!   docs/test-only commits between).
//! - **Capture tree (3.4 fixtures):** commit `524c126` — the pre-3.4 tree,
//!   before any T5 seam edit; `SEMANTICS_REV` 5.
//! - **Capture command** (per cell, no auto-update path): run
//!   `cargo test -p yokohama-core --test contestant_golden`; on mismatch the
//!   produced JSON lands in the workspace `target/tmp/` at the printed path;
//!   `cp` it over the fixture — the human act — and re-run to green.
//! - **Serialization:** `serde_json::to_string_pretty` of the `CellReport`
//!   (pretty so a mismatch diff says WHAT moved — the artifacts are ~2 KB).
//!   The compare parses both sides to `serde_json::Value`, so file-edge bytes
//!   (trailing newline) are irrelevant.
//!
//! ## Rev-6 re-capture (2026-08-08, branch `rules-parity-rev6`)
//!
//! ALL FIVE fixtures were re-captured at `SEMANTICS_REV` 6 — the owner-visible
//! ruling the mismatch message demands is the 2026-08-08 rules-parity ruling
//! set (guidingpath decision log, four rows dated 2026-08-08): D1 church yen
//! cost 2, D2 extra-tech surcharge from the 4th card, D3 movement-icon
//! to-hand, D5 2P dummy in the tech majority. Those are REACHABLE-STATE
//! behaviour changes by design, so every self-play trajectory these cells
//! freeze legitimately moved; the goldens now pin the rev-6 arms. The claim
//! these fixtures carry is unchanged in KIND — "the contestant-seam plumbing
//! does not move play" — but its baseline is now the rev-6 tree. The 2P
//! pre-3.4 cell additionally moved master (serial `0001` → `0005`, see
//! [`MASTER_2P_34`]): its old master degenerates to a constant unit vector
//! under rev-6 play, which the anti-vacuity guard refuses by design.
//!
//! ## Rev-7 re-capture (2026-08-08, branch `rules-parity-rev7`)
//!
//! ALL FIVE fixtures were re-captured again at `SEMANTICS_REV` 7. The
//! owner-visible ruling the mismatch message demands is the 2026-08-08 D6 row
//! in guidingpath's decision log: the Church/Customs track pawn is taken
//! AREA-FIRST (from the activated area when the acting player has own
//! assistant(s) there, from hand otherwise), a deliberate BGA-parity choice
//! recorded as a potential rulebook diff. That is a reachable-state behaviour
//! change by design, so every self-play trajectory these cells freeze
//! legitimately moved; the goldens now pin the rev-7 arms. The claim these
//! fixtures carry is unchanged in KIND — "the contestant-seam plumbing does
//! not move play" — with its baseline moved to the rev-7 tree.
//!
//! Read the size of the move honestly: D6 does NOT change the legal set at
//! the Church/Customs action itself (availability is source-blind), so these
//! cells did not move because the arms were offered different choices there.
//! They moved because a rung's sampler consumes the SAME keyed stream against
//! a state whose assistant counts differ by one from the take onward — every
//! downstream draw shifts, and the trajectories diverge from that point.
//!
//! The 4P prewiden cell additionally moved master (serial `0036` → `0038`,
//! see [`MASTER_4P`]): its old master degenerates to a constant unit vector
//! under rev-7 play, the same anti-vacuity refusal rev 6 hit on the two 2P
//! cells. The other four kept their masters.
//!
//! ## On `compose_echo`
//!
//! `compose_echo` mirrors `stats.rs`'s private `policy_echo` body — it is a
//! RESTATEMENT for reachability (the formatter is private), not an independent
//! derivation, and the honest statement of where the pin's authority lives is:
//! the hand-written literals below plus the captured artifacts. If `policy_echo`
//! and `compose_echo` drift together, the literals and fixtures still fail.
//! `compose_contestant_echo` extends the same restatement over the widened
//! `Contestant::descriptor` surface for the 3.4 evaluator cell, on the same
//! honesty terms — plus a hand-written PREFIX literal for each evaluator arm,
//! so the human-readable knob set is still pinned by characters typed here
//! (only the `weights=` table-identity hash rides the composed tail).

use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use yokohama_core::{
    run_cell, CellReport, CellSpec, Contestant, IntervalOutcome, RolloutPolicy, Rules,
    SeedScheduleKind,
};

/// The 2P cell's committed diagnostic master. See the module doc. Originally
/// serial `0033` (the 3.3 capture); RE-PROBED at the SEMANTICS_REV-6
/// re-capture (module doc, "Rev-6 re-capture"): under rev-6 play `0033`
/// degenerates to a constant unit vector, which the anti-vacuity guard
/// refuses. `0037` is the family's next free serial (`0034`/`0035` were
/// burned at the 3.3 capture, `0036` is the 4P cell's) and its FIRST probe
/// gave distinct shares (1×0.0 · 3×0.5) with a real bootstrap interval — no
/// serial burned.
const MASTER_2P: u64 = 0xA3E1_2026_0726_0037;
/// The 4P rotation-block cell's committed diagnostic master. Originally
/// `0036` (the 3.3 capture, itself the third probe there). RE-PROBED at the
/// SEMANTICS_REV-7 re-capture (module doc, "Rev-7 re-capture"): under rev-7
/// play `0036` degenerates to a CONSTANT unit vector and the anti-vacuity
/// guard refuses it — the same refusal rev 6 hit on the two 2P cells, landing
/// on this one instead. `0038` is the family's next free serial in the `0726`
/// date field (`0037` is the 2P cell's) and its FIRST probe gave distinct
/// shares (1×0.5 · 1×0.75) with a real bootstrap interval [0.50, 0.75] — no
/// serial burned.
const MASTER_4P: u64 = 0xA3E1_2026_0726_0038;

const CELL_NAME_2P: &str = "t0-prewiden-golden-ra-vs-fw";
const CELL_NAME_4P: &str = "t0-prewiden-golden-ra-vs-fw-4p";

/// The committed pre-widening artifacts.
const GOLDEN_2P: &str = include_str!("fixtures/cellreport_golden_prewiden.json");
const GOLDEN_4P: &str = include_str!("fixtures/cellreport_golden_prewiden_4p.json");

// ---------------------------------------------------------------------------
// Phase 3.4 T0 constants (module doc, "Phase 3.4 T0" section)
// ---------------------------------------------------------------------------

/// The 3.4 2P Policy pair cell's committed diagnostic master — `A3E1` family,
/// date field `2026_0804` (capture date). Originally serial `0001` (first
/// probe at the 3.4 capture). RE-PROBED at the SEMANTICS_REV-6 re-capture
/// (2026-08-08, rules-parity owner rulings — see the module doc's "Rev-6
/// re-capture" section): under rev-6 play serial `0001` degenerates to a
/// constant unit vector (all 0.5), which the anti-vacuity guard rightly
/// refuses, so the cell moved to the next free serial. `0002`–`0003` stay
/// with the other two cells and `0004` is `architecture_gate`'s; `0005` was
/// the first probe at rev 6 and gave distinct shares (1×0.0 · 3×0.5) with a
/// real bootstrap interval — no serial burned.
const MASTER_2P_34: u64 = 0xA3E1_2026_0804_0005;
/// The 3.4 4P rotation-block cell's master — same family/date, serial `0002`.
/// First probe: distinct shares and a real interval; no serial burned.
const MASTER_4P_34: u64 = 0xA3E1_2026_0804_0002;
/// The 3.4 evaluator cell's master — same family/date, serial `0003`. No
/// interval probe applies here BY DESIGN (module doc: the expected shape is the
/// all-won refusal, and probing for anything else would select an atypical
/// master).
const MASTER_EVAL_34: u64 = 0xA3E1_2026_0804_0003;

const CELL_NAME_2P_34: &str = "t0-pre34-golden-ra-vs-fw";
const CELL_NAME_4P_34: &str = "t0-pre34-golden-ra-vs-fw-4p";
const CELL_NAME_EVAL_34: &str = "t0-pre34-golden-lookahead-vs-greedy";

/// The committed pre-3.4 artifacts (capture tree `524c126`).
const GOLDEN_2P_34: &str = include_str!("fixtures/cellreport_golden_pre34.json");
const GOLDEN_4P_34: &str = include_str!("fixtures/cellreport_golden_pre34_4p.json");
const GOLDEN_EVAL_34: &str = include_str!("fixtures/cellreport_golden_pre34_eval.json");

/// The knobless rung's verbatim echo — no `{...}` group at all.
const ECHO_KNOBLESS: &str = "retreat-averse@v1";
/// The knobbed rung's verbatim echo at the SHIPPED defaults (BTreeMap knob
/// order — alphabetical, not declaration order).
const ECHO_KNOBBED_DEFAULT: &str = "family-weighted@v1{recovery_min_of=5,retreat_ppm=166667}";
/// The knobbed rung at EXPLICIT non-default knobs — E1's `p = 0.05` cell.
const ECHO_KNOBBED_EXPLICIT: &str = "family-weighted@v1{recovery_min_of=5,retreat_ppm=50000}";

/// The evaluator arms' echo PREFIXES, hand-written (module doc, "On
/// `compose_echo`"): everything up to the `weights=` table-identity hash, which
/// is the one component that must ride the composed descriptor. BTreeMap knob
/// order (alphabetical).
const ECHO_LOOKAHEAD_PREFIX: &str = "lookahead-v1@v1{b=4,eval_impl=2,k=1,m=8,weights=";
const ECHO_GREEDY_PREFIX: &str = "greedy-v1@v1{eval_impl=2,m=8,placement_cap=512,weights=";

/// The 3.3-era captured-key list — every top-level field `CellReport` had at
/// the 3.3 capture. The 3.3 fixtures are held to THIS list (they were captured
/// before the per-seat/explanation/cap fields existed, so demanding the wider
/// list of them would be demanding fields their tree could not produce).
const CAPTURED_KEYS_33: [&str; 11] = [
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

/// all sixteen fields `CellReport` carried at CAPTURE time (2026-08-04, commit
/// 524c126, pre-seam) — the T5 seam additions (the x_/y_ budget meters) are
/// produced-only keys these fixtures cannot carry, tolerated BY DESIGN (six-lens
/// review N5: the earlier "every field today" wording was false at any post-seam
/// HEAD and is retired).
/// the 3.3 list plus the five fields 3.3's own tasks added after its T0
/// (`applied_by_seat`, `x_applied_moves`, `x_explanations`, `x_ply_cap_hits`,
/// `x_free_cap_hits`). The 3.4 fixtures are held to this list, so a fixture
/// that silently failed to capture one of the newer fields — the exact
/// vacuity lens 2 S10 names — fails loudly. If `CellReport` grows a field and
/// this list is not extended at the same task, the completeness check cannot
/// notice (a produced-only key is tolerated by design); the T5-declared budget
/// meters are exactly such sanctioned additions and get their own tests there.
const CAPTURED_KEYS_CURRENT: [&str; 16] = [
    "spec_echo",
    "games",
    "x_unit_shares",
    "theta_hat",
    "interval",
    "cross_checks",
    "seat0_wins",
    "ties_to_seat0",
    "vp_margin_mean",
    "applied_by_seat",
    "x_applied_moves",
    "x_explanations",
    "x_ply_cap_hits",
    "x_free_cap_hits",
    "trigger",
    "master",
];

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

/// The 3.4 cells' token list — [`echo_tokens`] generalized over the contestant
/// echoes and the game count, plus the `setup-identical=` token the 3.3
/// capture tree did not print (it exists on today's tree, so today's goldens
/// pin it; retro-pinning it onto the 3.3 tests would change what THOSE tests
/// assert, which T0 leaves untouched).
fn echo_tokens_34(
    cell_name: &str,
    players: u8,
    units: u32,
    master: u64,
    x_echo: &str,
    y_echo: &str,
    setup_identical: &str,
) -> Vec<String> {
    vec![
        format!("cell={cell_name} "),
        format!("players={players} "),
        format!("x={x_echo} "),
        format!("y={y_echo} "),
        format!("units={units} "),
        format!("games={} ", u32::from(players) * units),
        format!("master=diagnostic:{master:#x} "),
        "bootstrap_B=10000 ".to_string(),
        "single-threaded=yes ".to_string(),
        "fixed-N=yes ".to_string(),
        // The echo's final token: end-of-line anchored by the echo's own shape,
        // so no trailing delimiter exists to pin; the value domain is closed
        // (yes|no). Kept as its own entry so a token inserted between fixed-N
        // and setup-identical cannot silently break an adjacency-glued pair.
        format!("setup-identical={setup_identical}"),
    ]
}

fn golden_spec_2p() -> CellSpec {
    CellSpec {
        name: CELL_NAME_2P.to_string(),
        players: 2,
        x: Contestant::Policy(RolloutPolicy::RetreatAverse),
        y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
        units: 4,
        master: SeedScheduleKind::Diagnostic(MASTER_2P),
    }
}

fn golden_spec_4p() -> CellSpec {
    CellSpec {
        name: CELL_NAME_4P.to_string(),
        players: 4,
        x: Contestant::Policy(RolloutPolicy::RetreatAverse),
        y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
        units: 2,
        master: SeedScheduleKind::Diagnostic(MASTER_4P),
    }
}

fn golden_spec_2p_34() -> CellSpec {
    CellSpec {
        name: CELL_NAME_2P_34.to_string(),
        players: 2,
        x: Contestant::Policy(RolloutPolicy::RetreatAverse),
        y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
        units: 4,
        master: SeedScheduleKind::Diagnostic(MASTER_2P_34),
    }
}

fn golden_spec_4p_34() -> CellSpec {
    CellSpec {
        name: CELL_NAME_4P_34.to_string(),
        players: 4,
        x: Contestant::Policy(RolloutPolicy::RetreatAverse),
        y: Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT),
        units: 2,
        master: SeedScheduleKind::Diagnostic(MASTER_4P_34),
    }
}

fn golden_spec_eval_34() -> CellSpec {
    CellSpec {
        name: CELL_NAME_EVAL_34.to_string(),
        players: 2,
        x: Contestant::LOOKAHEAD_V1_DEFAULT,
        y: Contestant::GREEDY_V1_DEFAULT,
        units: 2,
        master: SeedScheduleKind::Diagnostic(MASTER_EVAL_34),
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

/// [`compose_echo`] over the widened `Contestant::descriptor` surface — same
/// restatement honesty; the evaluator callers additionally pin a hand-written
/// prefix literal (module doc, "On `compose_echo`").
fn compose_contestant_echo(contestant: Contestant) -> String {
    let (name, version, params) = contestant.descriptor();
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

/// What the captured cell's interval must look like for the golden to be
/// non-vacuous — the anti-vacuity guard, parameterized because the two arms
/// degenerate differently (module doc, "Phase 3.4 T0").
enum IntervalShape {
    /// A real bootstrap interval (`hi > lo`): the Policy cells' requirement,
    /// probed for at capture. A constant unit vector here pins almost no play.
    RealBootstrap,
    /// The all-won zero-width refusal: the evaluator cell's EXPECTED shape
    /// (Lookahead-v1 sweeps Greedy-v1 — lattice B, 100/100). The pinning
    /// authority is the play-sensitive captured values, and the caller asserts
    /// nonzero play separately.
    AllUnitsWonRefusal,
}

/// The per-golden comparison contract: which keys the fixture must carry,
/// which echo tokens must hold, and which interval shape is non-vacuous.
/// Grouped so [`assert_cell_matches_golden`] stays within clippy's argument
/// budget without an `#[allow]`.
struct GoldenContract<'a> {
    captured_keys: &'a [&'a str],
    echo_tokens: Vec<String>,
    interval: IntervalShape,
}

/// Run one golden cell and hold it to its fixture under the comparison
/// contract. Shared by all five golden tests. Returns the report so a caller
/// can make its own extra structural reads (the evaluator cell's nonzero-play
/// guard).
fn assert_cell_matches_golden(
    spec: &CellSpec,
    golden_str: &str,
    master: u64,
    stem: &str,
    capture_commit: &str,
    contract: &GoldenContract,
) -> CellReport {
    let rules = Rules::standard();
    let report =
        run_cell(spec, &rules).unwrap_or_else(|e| panic!("the golden cell failed to run: {e}"));

    // Cheap structural reads FIRST, so the common failure modes (unit shape,
    // master plumbing, invariant default) report themselves in one line. None
    // of these is the authority — the subset compare below is.
    assert_eq!(
        report.games,
        u32::from(spec.players) * spec.units,
        "games is units x players by construction",
    );
    assert_eq!(report.x_unit_shares.len(), spec.units as usize, "one share per design unit");
    assert_eq!(report.master, master, "the cell ran on a different master than the golden");
    for token in &contract.echo_tokens {
        assert!(
            report.spec_echo.contains(token),
            "the echo lost the pinned token {token:?}: {}",
            report.spec_echo,
        );
    }
    match contract.interval {
        IntervalShape::RealBootstrap => match report.interval {
            IntervalOutcome::Bootstrap { lo, hi, .. } => assert!(
                hi > lo,
                "the golden cell must carry a real interval, got [{lo}, {hi}]",
            ),
            ref other => panic!(
                "the golden cell degenerated to a refusal ({other:?}) — recapture the CELL \
                 DESIGN (a constant unit vector pins no play), not the fixture",
            ),
        },
        IntervalShape::AllUnitsWonRefusal => match report.interval {
            IntervalOutcome::RefusedZeroWidth { .. } => {
                assert!(
                    report.x_unit_shares.iter().all(|&s| s == 1.0),
                    "the refusal-shaped golden expects the ALL-WON degeneracy, got shares {:?}",
                    report.x_unit_shares,
                );
            }
            ref other => panic!(
                "the evaluator golden's expected shape is the all-won refusal (module doc) but \
                 the cell produced {other:?} — the arm's play moved at capture time; re-read the \
                 cell design before touching anything",
            ),
        },
    }

    let produced = serde_json::to_string_pretty(&report).expect("CellReport serializes");
    let produced_v: Value = serde_json::from_str(&produced).expect("produced JSON parses");
    let golden_v: Value =
        serde_json::from_str(golden_str).expect("the committed fixture parses as JSON");

    // FIXTURE COMPLETENESS — the subset compare's one blind spot is a fixture
    // that captures too little (an empty `{}` placeholder passes every subset
    // walk vacuously; found live at the 3.3 fix wave). The fixture must carry
    // every top-level field in the contract's key list, so a truncated or
    // placeholder fixture fails loudly and the produced JSON is still written
    // for the manual capture step.
    let golden_obj = golden_v.as_object().unwrap_or_else(|| {
        let path = actual_path(stem);
        let _ = fs::write(&path, format!("{produced}\n"));
        panic!(
            "the committed fixture is not a JSON object (placeholder?) — produced JSON written \
             to {} for the manual capture step (module doc)",
            path.display(),
        );
    });
    for key in contract.captured_keys {
        if !golden_obj.contains_key(*key) {
            let path = actual_path(stem);
            let wrote = fs::write(&path, format!("{produced}\n"))
                .map(|()| format!("produced JSON written to {}", path.display()))
                .unwrap_or_else(|e| format!("could not write {}: {e}", path.display()));
            panic!(
                "the committed fixture is INCOMPLETE: missing captured key {key:?} \
                 (a partial fixture would make the subset compare vacuous). {wrote}",
            );
        }
        // A key hollowed to `{}` passes every subset walk vacuously — the nested
        // analogue of the whole-fixture placeholder rejected above. Reject it
        // wherever the produced value actually carries fields to pin (an empty
        // produced object stays legal: there is nothing to capture).
        if golden_obj.get(*key).and_then(Value::as_object).is_some_and(|c| c.is_empty())
            && produced_v.get(*key).and_then(Value::as_object).is_some_and(|p| !p.is_empty())
        {
            let path = actual_path(stem);
            let wrote = fs::write(&path, format!("{produced}\n"))
                .map(|()| format!("produced JSON written to {}", path.display()))
                .unwrap_or_else(|e| format!("could not write {}: {e}", path.display()));
            panic!(
                "the committed fixture is HOLLOW at captured key {key:?}: an empty object \
                 where the produced value has fields (an empty-object value passes the \
                 subset compare vacuously). {wrote}",
            );
        }
    }

    if let Some(divergence) = subset_diff(&golden_v, &produced_v, "") {
        let path = actual_path(stem);
        let wrote = fs::write(&path, format!("{produced}\n"))
            .map(|()| format!("produced JSON written to {}", path.display()))
            .unwrap_or_else(|e| format!("could not write {}: {e}", path.display()));
        panic!(
            "THE {} ARM MOVED: {divergence}.\n\
             A value captured in the pre-change golden (commit {capture_commit}) changed, or a \
             captured field vanished — which means figures measured through this arm no longer \
             describe this code. NOTE: purely ADDITIVE CellReport fields and ADDED spec_echo \
             tokens are invisible to this test BY DESIGN (the sanctioned seam-task delta) — if \
             you are seeing this, the change was NOT additive. Fix the code; do NOT re-capture \
             the fixture without an owner-visible ruling (module doc). {wrote}",
            spec.name,
        );
    }
    report
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

/// §5.1: the pre-widening 2P pair-unit `CellReport`, every captured field exact.
#[test]
fn the_policy_arm_cell_report_matches_the_prewiden_golden() {
    let spec = golden_spec_2p();
    assert_cell_matches_golden(
        &spec,
        GOLDEN_2P,
        MASTER_2P,
        "cellreport_golden_prewiden",
        "0a6e736",
        &GoldenContract {
            captured_keys: &CAPTURED_KEYS_33,
            echo_tokens: echo_tokens(&spec.name, spec.players, spec.units, MASTER_2P),
            interval: IntervalShape::RealBootstrap,
        },
    );
}

/// The 4P ROTATION-BLOCK companion (T0 review, should-fix 1): the rotation
/// path's slots {0..3} and block arithmetic are different plumbing from the 2P
/// pair, and a re-key mis-map confined to them would pass the 2P golden
/// silently. Same contract, own fixture and master.
#[test]
fn the_policy_arm_4p_rotation_cell_matches_the_prewiden_golden() {
    let spec = golden_spec_4p();
    assert_cell_matches_golden(
        &spec,
        GOLDEN_4P,
        MASTER_4P,
        "cellreport_golden_prewiden_4p",
        "the T0-review fix-wave commit",
        &GoldenContract {
            captured_keys: &CAPTURED_KEYS_33,
            echo_tokens: echo_tokens(&spec.name, spec.players, spec.units, MASTER_4P),
            interval: IntervalShape::RealBootstrap,
        },
    );
}

/// Phase 3.4 T0: the pre-3.4 2P Policy pair-unit golden, held to the FULL
/// current key list — this is the fixture that pins the Policy arm's
/// `applied_by_seat` values and its `null` explanation/cap fields, which the
/// 3.3 goldens structurally cannot (their fixtures predate the fields).
#[test]
fn the_policy_arm_2p_cell_matches_the_pre34_golden() {
    let spec = golden_spec_2p_34();
    assert_cell_matches_golden(
        &spec,
        GOLDEN_2P_34,
        MASTER_2P_34,
        "cellreport_golden_pre34",
        "524c126",
        &GoldenContract {
            captured_keys: &CAPTURED_KEYS_CURRENT,
            echo_tokens: echo_tokens_34(
                &spec.name,
                spec.players,
                spec.units,
                MASTER_2P_34,
                ECHO_KNOBLESS,
                ECHO_KNOBBED_DEFAULT,
                "yes",
            ),
            interval: IntervalShape::RealBootstrap,
        },
    );
}

/// Phase 3.4 T0: the pre-3.4 4P rotation-block golden, full key list — the
/// slots-{0..3} plumbing under the widened pins (a re-key defect confined to
/// per-seat tabulation on the rotation path would pass every other golden).
#[test]
fn the_policy_arm_4p_rotation_cell_matches_the_pre34_golden() {
    let spec = golden_spec_4p_34();
    assert_cell_matches_golden(
        &spec,
        GOLDEN_4P_34,
        MASTER_4P_34,
        "cellreport_golden_pre34_4p",
        "524c126",
        &GoldenContract {
            captured_keys: &CAPTURED_KEYS_CURRENT,
            echo_tokens: echo_tokens_34(
                &spec.name,
                spec.players,
                spec.units,
                MASTER_4P_34,
                ECHO_KNOBLESS,
                ECHO_KNOBBED_DEFAULT,
                "yes",
            ),
            interval: IntervalShape::RealBootstrap,
        },
    );
}

/// Phase 3.4 T0: the EVALUATOR golden — `lookahead-v1` (X) vs `greedy-v1` (Y)
/// at shipped knobs, 2P, 2 pair units = 4 games. The one golden whose fixture
/// carries NON-NULL `x_explanations`/`x_ply_cap_hits`/`x_free_cap_hits`, so a
/// T5 defect in the evaluator arm's tally plumbing has a pre-change artifact to
/// fail against. Interval expectation and its honesty: module doc, "Phase 3.4
/// T0". Keep-homogeneous (both `EvaluatorScored`), hence `setup-identical=yes`.
#[test]
fn the_evaluator_arm_cell_matches_the_pre34_golden() {
    let spec = golden_spec_eval_34();
    let x_echo = compose_contestant_echo(spec.x);
    let y_echo = compose_contestant_echo(spec.y);
    // The hand-written half of the evaluator echo pins (module doc, "On
    // `compose_echo`"): the knob set verbatim, with only the table-identity
    // hash riding the composed tail.
    assert!(
        x_echo.starts_with(ECHO_LOOKAHEAD_PREFIX),
        "the lookahead echo lost its pinned knob prefix: {x_echo}",
    );
    assert!(
        y_echo.starts_with(ECHO_GREEDY_PREFIX),
        "the greedy echo lost its pinned knob prefix: {y_echo}",
    );

    let report = assert_cell_matches_golden(
        &spec,
        GOLDEN_EVAL_34,
        MASTER_EVAL_34,
        "cellreport_golden_pre34_eval",
        "524c126",
        &GoldenContract {
            captured_keys: &CAPTURED_KEYS_CURRENT,
            echo_tokens: echo_tokens_34(
                &spec.name,
                spec.players,
                spec.units,
                MASTER_EVAL_34,
                &x_echo,
                &y_echo,
                "yes",
            ),
            interval: IntervalShape::AllUnitsWonRefusal,
        },
    );

    // The refusal-shaped cell's nonzero-play guard (module doc): the interval
    // pins nothing here, so real play must be evidenced by the counters the
    // subset compare then holds exactly.
    assert!(report.x_applied_moves > 0, "the evaluator cell applied no X moves");
    assert_eq!(
        report.x_explanations,
        Some(report.x_applied_moves),
        "spec 0.3(b)'s explanation identity must hold on the captured cell",
    );
    assert_eq!(report.x_ply_cap_hits, Some(0), "a capped line would be valued mid-turn");
    assert_eq!(report.x_free_cap_hits, Some(0), "structural for an S1 contestant");
    assert!(
        report.applied_by_seat.iter().all(|&n| n > 0),
        "every seat must have played: {:?}",
        report.applied_by_seat,
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
    // The 3.4 Policy fixtures carry the same matchup, so they pin the same
    // strings on today's tree.
    for golden in [GOLDEN_2P, GOLDEN_4P, GOLDEN_2P_34, GOLDEN_4P_34] {
        assert!(
            golden.contains(&format!("x={ECHO_KNOBLESS} ")),
            "a golden lost the knobless rung's delimited echo",
        );
        assert!(
            golden.contains(&format!("y={ECHO_KNOBBED_DEFAULT} ")),
            "a golden lost the knobbed rung's delimited echo",
        );
    }
    // The evaluator fixture pins the two evaluator arms' hand-written knob
    // prefixes the same way (the tail is the table-identity hash).
    assert!(
        GOLDEN_EVAL_34.contains(&format!("x={ECHO_LOOKAHEAD_PREFIX}")),
        "the evaluator golden lost the lookahead echo prefix",
    );
    assert!(
        GOLDEN_EVAL_34.contains(&format!("y={ECHO_GREEDY_PREFIX}")),
        "the evaluator golden lost the greedy echo prefix",
    );

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
    assert!(
        GOLDEN_2P_34.contains(&format!("cell={CELL_NAME_2P_34} ")),
        "the 3.4 2P golden lost its name",
    );
    assert!(
        GOLDEN_4P_34.contains(&format!("cell={CELL_NAME_4P_34} ")),
        "the 3.4 4P golden lost its name",
    );
    assert!(
        GOLDEN_EVAL_34.contains(&format!("cell={CELL_NAME_EVAL_34} ")),
        "the evaluator golden lost its name",
    );
}
