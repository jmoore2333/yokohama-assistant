//! The contestant NAME GRAMMAR — Phase 3.5c W1-B (`docs/plans/2026-08-09-phase3.5c-scope-confirmation.md`
//! §5, the close-handover §B gap "`Contestant` has NO parser").
//!
//! One function turns a typed spec — `family-weighted:p=50000,k=5`,
//! `lookahead-v1:b=4,cap=750`, `mcts-v1:leaf=midturn-diag` — into a
//! [`Contestant`]. It was example-private inside `examples/arena.rs` from Phase
//! 3.3 through 3.4 and moved here with its BEHAVIOUR unchanged — every accepted
//! spec and every refusal string is pinned to what the example printed before
//! the move (`tests::FROZEN_BATTERY`, captured mechanically off the old code).
//! The runner delegates, and the probe's contestant arm delegates too since
//! W1-C (it narrows AFTER parsing — it refuses the arms it does not drive);
//! the CLI's bot-seat retype (W5) is the remaining consumer-to-be, so no second
//! spelling of the grammar survives outside the CLI's rung-name lookup. The public entry point is
//! [`Contestant::from_str_name`]; this module is the body.
//!
//! **UNKNOWN IS AN ERROR at every level** — an unknown name, an unknown knob, an
//! unknown knob VALUE, or any knob at all on a contestant that has none. That is
//! the contract, and the reason is not tidiness: a silently-ignored knob would
//! put a cell's published figure under knobs it did not run. Every error NAMES
//! what it saw, which is why the signature is `Result<_, String>` and not
//! [`RolloutPolicy::from_str_name`]'s `Option` — a rung name either is or is not
//! one of three literals, while this grammar has dozens of distinct ways to be
//! wrong (the frozen battery below exercises 37 of them) and the caller cannot
//! reconstruct which one fired from a bare `None`.
//!
//! **Every contestant name comes from its controller's own const**
//! ([`greedy::GREEDY_CONTROLLER`], [`lookahead::LOOKAHEAD_CONTROLLER`],
//! [`lookahead::ATOMIC_DIAG_CONTROLLER`], [`mcts::MCTS_CONTROLLER`]) and every
//! default from that controller's own default const, so the grammar cannot drift
//! from what provenance records. The one hand-typed name in the file is
//! [`ATOMIC_DIAG_ALIAS`], and it exists because the spec §5.3 runner grammar
//! spells the S2 arm short.

use crate::game::greedy::{self, DEFAULT_RETREAT_SAMPLES};
use crate::game::lookahead::{self, DEFAULT_ATOMIC_DEPTH, DEFAULT_BEAM, DEFAULT_WORLDS};
use crate::game::mcts::{self, LeafRule, DEFAULT_EXPLORATION_PPM, DEFAULT_SQUASH_SCALE};
use crate::game::policy::RolloutPolicy;

use super::{Contestant, NamedIdentity};

/// The short alias for the S2 diagnostic arm.
///
/// Its provenance name is [`lookahead::ATOMIC_DIAG_CONTROLLER`]
/// (`lookahead-atomic-diag-v1`) and BOTH spellings parse — the canonical one so
/// a cell can be typed from what a corpus record says, this one because the
/// runner grammar the spec §5.3 brief names is the short form and a name that
/// has to be looked up is a name that gets mistyped.
pub const ATOMIC_DIAG_ALIAS: &str = "atomic-diag-v1";

/// The body of [`Contestant::from_str_name`] — see that function's doc for the
/// contract and this module's for the reasoning.
pub(super) fn parse_contestant(spec: &str) -> Result<Contestant, String> {
    let (name, knobs) = match spec.split_once(':') {
        Some((name, knobs)) => (name, Some(knobs)),
        None => (spec, None),
    };

    if name == greedy::GREEDY_CONTROLLER {
        let mut m = DEFAULT_RETREAT_SAMPLES;
        for (key, value) in knob_pairs(knobs)? {
            match key.as_str() {
                "m" => m = parse_knob_u8(&key, &value)?,
                other => return Err(format!("unknown knob {other:?} for {name} (m)")),
            }
        }
        return Ok(Contestant::GreedyV1 { m });
    }

    if name == lookahead::LOOKAHEAD_CONTROLLER {
        let (mut b, mut k, mut m) = (DEFAULT_BEAM, DEFAULT_WORLDS, DEFAULT_RETREAT_SAMPLES);
        // `None` unless typed: the shipped rung is the UNCAPPED one, and the
        // default must stay the thing every 3.3 figure was measured on.
        let mut cap: Option<u32> = None;
        for (key, value) in knob_pairs(knobs)? {
            match key.as_str() {
                "b" => {
                    b = value.parse::<u16>().map_err(|e| format!("b must be a beam width: {e}"))?;
                    if b == 0 {
                        return Err("b=0 is not a beam".to_string());
                    }
                }
                "k" => k = parse_knob_u8(&key, &value)?,
                "m" => m = parse_knob_u8(&key, &value)?,
                "cap" => cap = Some(parse_knob_cap(&value)?),
                other => {
                    return Err(format!("unknown knob {other:?} for {name} (b | k | m | cap)"))
                }
            }
        }
        return Ok(Contestant::LookaheadV1 { b, k, m, cap });
    }

    if name == mcts::MCTS_CONTROLLER {
        let (mut c_ppm, mut scale, mut m, mut leaf) = (
            DEFAULT_EXPLORATION_PPM,
            DEFAULT_SQUASH_SCALE,
            DEFAULT_RETREAT_SAMPLES,
            LeafRule::default(),
        );
        let mut cap: Option<u32> = None;
        for (key, value) in knob_pairs(knobs)? {
            match key.as_str() {
                "cap" => cap = Some(parse_knob_cap(&value)?),
                "c" => {
                    c_ppm = value
                        .parse::<u32>()
                        .map_err(|e| format!("c must be parts per million: {e}"))?;
                }
                "scale" => {
                    scale =
                        value.parse::<i64>().map_err(|e| format!("scale must be an integer: {e}"))?;
                    if scale <= 0 {
                        return Err(format!("scale={scale} is not a squash scale"));
                    }
                }
                "m" => m = parse_knob_u8(&key, &value)?,
                // An unknown knob VALUE is an error on the same grounds as an
                // unknown knob NAME: a silently-defaulted leaf rule would put a
                // published cell under a mechanism it did not run.
                "leaf" => {
                    leaf = LeafRule::from_str_name(&value).ok_or_else(|| {
                        format!(
                            "unknown leaf mode {value:?} ({} | {} | {})",
                            LeafRule::HandOff.as_str(),
                            LeafRule::MidTurnDiag.as_str(),
                            LeafRule::ConstantDiag.as_str(),
                        )
                    })?;
                }
                other => {
                    return Err(format!(
                        "unknown knob {other:?} for {name} (cap | c | scale | m | leaf)"
                    ))
                }
            }
        }
        return Ok(Contestant::MctsV1 { cap, c_ppm, scale, m, leaf });
    }

    if name == lookahead::ATOMIC_DIAG_CONTROLLER || name == ATOMIC_DIAG_ALIAS {
        let (mut d, mut k, mut m) =
            (DEFAULT_ATOMIC_DEPTH, DEFAULT_WORLDS, DEFAULT_RETREAT_SAMPLES);
        for (key, value) in knob_pairs(knobs)? {
            match key.as_str() {
                "d" => {
                    d = parse_knob_u8(&key, &value)?;
                    if d == 0 {
                        return Err("d=0 searches nothing".to_string());
                    }
                }
                "k" => k = parse_knob_u8(&key, &value)?,
                "m" => m = parse_knob_u8(&key, &value)?,
                other => return Err(format!("unknown knob {other:?} for {name} (d | k | m)")),
            }
        }
        return Ok(Contestant::AtomicDiag { d, k, m });
    }

    // The NAMED IDENTITIES (V4 step 8, spec §13): knobless by design — see
    // [`NamedIdentity`]'s doc — so any knob group at all is refused here in the
    // knobless rungs' own sentence rather than falling through to an
    // unknown-contestant refusal that would name the wrong problem.
    if let Some(arm) = NamedIdentity::from_str_name(name) {
        if let Some(knobs) = knobs {
            return Err(format!("named identity {name:?} has no knobs, but {knobs:?} was given"));
        }
        return Ok(Contestant::NamedTable(arm));
    }

    parse_policy(name, knobs).map(Contestant::Policy)
}

/// `key=value` pairs from a `:`-suffix knob group, or an empty list when there
/// is none. Empty segments (a trailing comma) are skipped; a segment without
/// `=` is an error.
fn knob_pairs(knobs: Option<&str>) -> Result<Vec<(String, String)>, String> {
    let Some(knobs) = knobs else { return Ok(Vec::new()) };
    knobs
        .split(',')
        .filter(|k| !k.is_empty())
        .map(|knob| {
            knob.split_once('=')
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .ok_or_else(|| format!("knob {knob:?} is not <key>=<value>"))
        })
        .collect()
}

fn parse_knob_u8(key: &str, value: &str) -> Result<u8, String> {
    value.parse::<u8>().map_err(|e| format!("{key} must be a small count: {e}"))
}

/// What a zero budget is refused WITH, at every surface that reads one.
///
/// Single-sourced because [`Contestant::from_parts`] refuses `cap=0` in a
/// recorded params map on exactly the same grounds this parser refuses it on a
/// command line: the two surfaces describing one rule must not drift into two
/// different sentences about it.
pub(super) const CAP_ZERO_REFUSAL: &str =
    "cap=0 starves every decision; omit the knob for an uncapped seat";

/// The shared scored-successor budget (spec §3.4). `cap=0` is refused rather
/// than silently meaning "uncapped": OMITTING the knob is how a caller says
/// uncapped, and a budget of zero would starve the mandatory root pass at every
/// decision — a configuration nothing in the experiment wants and everything
/// downstream would have to caveat.
fn parse_knob_cap(value: &str) -> Result<u32, String> {
    let cap = value
        .parse::<u32>()
        .map_err(|e| format!("cap must be a scored-successor budget: {e}"))?;
    if cap == 0 {
        return Err(CAP_ZERO_REFUSAL.to_string());
    }
    Ok(cap)
}

/// A policy name, optionally with `:p=<ppm>,k=<n>` knobs.
///
/// The knob form exists so the two `FamilyWeighted` settings §0-R3(i)'s
/// guaranteed edge is built from are reachable from a command line without a
/// second enum. Knobs on a rung that has none is an error, not a silent ignore.
///
/// **W5's deferred repair:** the unknown-knob fall-through below now names the
/// contestant it was reading, `for {name}`, exactly as the four
/// evaluator/architecture arms in [`parse_contestant`] always did. It was the
/// one refusal in this file that made a reader go back to the source to learn
/// which contestant had rejected their knob; the W1 fix wave recorded the
/// repair and deferred it here because the string is owner-visible in the
/// runner. `FROZEN_BATTERY`'s `family-weighted:z=1` row moved with it — the
/// battery's only deliberate departure from HEAD `a9029b8`.
fn parse_policy(name: &str, knobs: Option<&str>) -> Result<RolloutPolicy, String> {
    let base = RolloutPolicy::from_str_name(name).ok_or_else(|| {
        format!(
            "unknown contestant {name:?} (rungs: uniform | retreat-averse | family-weighted; \
             evaluator: {} | {} | {ATOMIC_DIAG_ALIAS}; architecture: {}; named identities: {} | {})",
            greedy::GREEDY_CONTROLLER,
            lookahead::LOOKAHEAD_CONTROLLER,
            mcts::MCTS_CONTROLLER,
            NamedIdentity::V2Padded.name(),
            NamedIdentity::V2RefitPadded.name(),
        )
    })?;
    let Some(knobs) = knobs else { return Ok(base) };

    let RolloutPolicy::FamilyWeighted { mut retreat_ppm, mut recovery_min_of } = base else {
        return Err(format!("policy {name:?} has no knobs, but {knobs:?} was given"));
    };
    for (key, value) in knob_pairs(Some(knobs))? {
        match key.as_str() {
            "p" => {
                retreat_ppm = value
                    .parse::<u32>()
                    .map_err(|e| format!("p must be parts per million: {e}"))?;
                if retreat_ppm > 1_000_000 {
                    return Err(format!("p={retreat_ppm} exceeds 1_000_000 ppm"));
                }
            }
            "k" => recovery_min_of = parse_knob_u8(&key, &value)?,
            other => return Err(format!("unknown knob {other:?} for {name} (p | k)")),
        }
    }
    Ok(RolloutPolicy::FamilyWeighted { retreat_ppm, recovery_min_of })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The BEHAVIOUR-IDENTITY table: every row is what the example-private
    /// `parse_contestant` printed at HEAD `a9029b8`, captured mechanically
    /// before a byte of it moved.
    ///
    /// How it was captured: the parser block (`examples/arena.rs:213-417` at
    /// that commit) was copied VERBATIM into a throwaway example beside a main
    /// that ran this spec list and emitted these rows as Rust source — so the
    /// expectations are the old code's own output, not a transcription of what
    /// the old code was believed to do. The throwaway example was deleted once
    /// the promoted parser reproduced the same output byte for byte.
    ///
    /// `Ok` rows carry the `Debug` of the parsed [`Contestant`] (which pins
    /// every field, defaults included); `Err` rows carry the error string
    /// VERBATIM, because those strings are user-facing in the runner.
    ///
    /// **TWO GROUPS OF ROWS have deliberately departed from that capture**, and
    /// an UNMARKED departure is a regression rather than a re-capture:
    ///
    /// 1. W5 landed the refusal-text repair the W1 fix wave recorded and
    ///    deferred (see [`parse_policy`]), so `family-weighted:z=1` now names the
    ///    contestant the way every other arm's fall-through always did. Marked
    ///    in place below.
    /// 2. **V4 §7 step 8 T3** added the named-identity arms, so the
    ///    unknown-contestant refusal's ALTERNATIVES LIST gained a fourth
    ///    category — `; named identities: v2-padded | v2-refit-padded`. That
    ///    moved the EIGHT rows whose expectation is that sentence (`""`,
    ///    `"nope"`, `"greedy-v2"`, `" uniform"`, `"UNIFORM"`, `"uniform "`,
    ///    `"mcts-v1 "`, `":"`, `":m=4"` — nine expectations over eight distinct
    ///    sentences). Every SPEC and every accepted parse is untouched: what
    ///    changed is the list of names the refusal offers, and leaving it stale
    ///    would refuse `v2-refit-padde` with a menu that omits the arm the
    ///    typist wanted. The row COUNT and the refusal COUNT below are
    ///    unchanged, which is what keeps this an amendment to the capture
    ///    rather than a re-capture of it.
    const FROZEN_BATTERY: &[(&str, Result<&str, &str>)] = &[
        ("uniform", Ok("Policy(Uniform)")),
        ("retreat-averse", Ok("Policy(RetreatAverse)")),
        ("retreat_averse", Ok("Policy(RetreatAverse)")),
        ("family-weighted", Ok("Policy(FamilyWeighted { retreat_ppm: 166667, recovery_min_of: 5 })")),
        ("family_weighted", Ok("Policy(FamilyWeighted { retreat_ppm: 166667, recovery_min_of: 5 })")),
        ("greedy-v1", Ok("GreedyV1 { m: 8 }")),
        ("lookahead-v1", Ok("LookaheadV1 { b: 4, k: 1, m: 8, cap: None }")),
        ("atomic-diag-v1", Ok("AtomicDiag { d: 2, k: 1, m: 8 }")),
        ("lookahead-atomic-diag-v1", Ok("AtomicDiag { d: 2, k: 1, m: 8 }")),
        ("mcts-v1", Ok("MctsV1 { cap: None, c_ppm: 1414214, scale: 100, m: 8, leaf: HandOff }")),
        ("family-weighted:p=50000", Ok("Policy(FamilyWeighted { retreat_ppm: 50000, recovery_min_of: 5 })")),
        ("family-weighted:k=3", Ok("Policy(FamilyWeighted { retreat_ppm: 166667, recovery_min_of: 3 })")),
        ("family-weighted:p=50000,k=5", Ok("Policy(FamilyWeighted { retreat_ppm: 50000, recovery_min_of: 5 })")),
        ("family-weighted:p=166667,k=5", Ok("Policy(FamilyWeighted { retreat_ppm: 166667, recovery_min_of: 5 })")),
        ("family-weighted:p=1000000", Ok("Policy(FamilyWeighted { retreat_ppm: 1000000, recovery_min_of: 5 })")),
        ("family-weighted:p=1000001", Err("p=1000001 exceeds 1_000_000 ppm")),
        ("family-weighted:p=-1", Err("p must be parts per million: invalid digit found in string")),
        ("family-weighted:p=abc", Err("p must be parts per million: invalid digit found in string")),
        ("family-weighted:k=256", Err("k must be a small count: number too large to fit in target type")),
        // W5: the ONE refusal in this file that did not name the contestant it
        // was reading (the W1 fix wave recorded the repair and deferred it here
        // as an owner-visible error-text change). Every other arm's fall-through
        // already said `for <name>`; this row is the frozen battery's only
        // deliberate departure from HEAD `a9029b8`.
        ("family-weighted:z=1", Err("unknown knob \"z\" for family-weighted (p | k)")),
        ("family-weighted:", Ok("Policy(FamilyWeighted { retreat_ppm: 166667, recovery_min_of: 5 })")),
        ("family_weighted:p=1", Ok("Policy(FamilyWeighted { retreat_ppm: 1, recovery_min_of: 5 })")),
        ("uniform:p=5", Err("policy \"uniform\" has no knobs, but \"p=5\" was given")),
        ("uniform:", Err("policy \"uniform\" has no knobs, but \"\" was given")),
        ("retreat-averse:k=1", Err("policy \"retreat-averse\" has no knobs, but \"k=1\" was given")),
        ("greedy-v1:m=4", Ok("GreedyV1 { m: 4 }")),
        ("greedy-v1:m=0", Ok("GreedyV1 { m: 0 }")),
        ("greedy-v1:m=255", Ok("GreedyV1 { m: 255 }")),
        ("greedy-v1:m=256", Err("m must be a small count: number too large to fit in target type")),
        ("greedy-v1:m=abc", Err("m must be a small count: invalid digit found in string")),
        ("greedy-v1:x=1", Err("unknown knob \"x\" for greedy-v1 (m)")),
        ("greedy-v1:", Ok("GreedyV1 { m: 8 }")),
        ("greedy-v1:m=1,m=2", Ok("GreedyV1 { m: 2 }")),
        ("greedy-v1:m=4,", Ok("GreedyV1 { m: 4 }")),
        ("greedy-v1:m", Err("knob \"m\" is not <key>=<value>")),
        ("lookahead-v1:b=8", Ok("LookaheadV1 { b: 8, k: 1, m: 8, cap: None }")),
        ("lookahead-v1:k=2", Ok("LookaheadV1 { b: 4, k: 2, m: 8, cap: None }")),
        ("lookahead-v1:m=3", Ok("LookaheadV1 { b: 4, k: 1, m: 3, cap: None }")),
        ("lookahead-v1:cap=750", Ok("LookaheadV1 { b: 4, k: 1, m: 8, cap: Some(750) }")),
        ("lookahead-v1:b=4,k=1,m=8,cap=750", Ok("LookaheadV1 { b: 4, k: 1, m: 8, cap: Some(750) }")),
        ("lookahead-v1:cap=0", Err("cap=0 starves every decision; omit the knob for an uncapped seat")),
        ("lookahead-v1:b=0", Err("b=0 is not a beam")),
        ("lookahead-v1:b=abc", Err("b must be a beam width: invalid digit found in string")),
        ("lookahead-v1:b=65536", Err("b must be a beam width: number too large to fit in target type")),
        ("lookahead-v1:cap=abc", Err("cap must be a scored-successor budget: invalid digit found in string")),
        ("lookahead-v1:z=1", Err("unknown knob \"z\" for lookahead-v1 (b | k | m | cap)")),
        ("atomic-diag-v1:d=3", Ok("AtomicDiag { d: 3, k: 1, m: 8 }")),
        ("atomic-diag-v1:d=0", Err("d=0 searches nothing")),
        ("atomic-diag-v1:k=2", Ok("AtomicDiag { d: 2, k: 2, m: 8 }")),
        ("atomic-diag-v1:m=4", Ok("AtomicDiag { d: 2, k: 1, m: 4 }")),
        ("atomic-diag-v1:q=1", Err("unknown knob \"q\" for atomic-diag-v1 (d | k | m)")),
        ("lookahead-atomic-diag-v1:d=2,k=1,m=8", Ok("AtomicDiag { d: 2, k: 1, m: 8 }")),
        ("mcts-v1:cap=750", Ok("MctsV1 { cap: Some(750), c_ppm: 1414214, scale: 100, m: 8, leaf: HandOff }")),
        ("mcts-v1:c=1000000", Ok("MctsV1 { cap: None, c_ppm: 1000000, scale: 100, m: 8, leaf: HandOff }")),
        ("mcts-v1:scale=250", Ok("MctsV1 { cap: None, c_ppm: 1414214, scale: 250, m: 8, leaf: HandOff }")),
        ("mcts-v1:m=4", Ok("MctsV1 { cap: None, c_ppm: 1414214, scale: 100, m: 4, leaf: HandOff }")),
        ("mcts-v1:leaf=hand-off", Ok("MctsV1 { cap: None, c_ppm: 1414214, scale: 100, m: 8, leaf: HandOff }")),
        ("mcts-v1:leaf=midturn-diag", Ok("MctsV1 { cap: None, c_ppm: 1414214, scale: 100, m: 8, leaf: MidTurnDiag }")),
        ("mcts-v1:leaf=constant-diag", Ok("MctsV1 { cap: None, c_ppm: 1414214, scale: 100, m: 8, leaf: ConstantDiag }")),
        ("mcts-v1:cap=750,c=1414214,scale=100,m=8,leaf=hand-off", Ok("MctsV1 { cap: Some(750), c_ppm: 1414214, scale: 100, m: 8, leaf: HandOff }")),
        ("mcts-v1:leaf=bogus", Err("unknown leaf mode \"bogus\" (hand-off | midturn-diag | constant-diag)")),
        ("mcts-v1:leaf=hand-off:extra", Err("unknown leaf mode \"hand-off:extra\" (hand-off | midturn-diag | constant-diag)")),
        ("mcts-v1:scale=0", Err("scale=0 is not a squash scale")),
        ("mcts-v1:scale=-5", Err("scale=-5 is not a squash scale")),
        ("mcts-v1:cap=0", Err("cap=0 starves every decision; omit the knob for an uncapped seat")),
        ("mcts-v1:c=abc", Err("c must be parts per million: invalid digit found in string")),
        ("mcts-v1:scale=abc", Err("scale must be an integer: invalid digit found in string")),
        ("mcts-v1:w=1", Err("unknown knob \"w\" for mcts-v1 (cap | c | scale | m | leaf)")),
        ("", Err("unknown contestant \"\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        ("nope", Err("unknown contestant \"nope\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        ("greedy-v2", Err("unknown contestant \"greedy-v2\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        (" uniform", Err("unknown contestant \" uniform\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        ("UNIFORM", Err("unknown contestant \"UNIFORM\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        ("uniform ", Err("unknown contestant \"uniform \" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        ("mcts-v1 ", Err("unknown contestant \"mcts-v1 \" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        (":", Err("unknown contestant \"\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
        (":m=4", Err("unknown contestant \"\" (rungs: uniform | retreat-averse | family-weighted; evaluator: greedy-v1 | lookahead-v1 | atomic-diag-v1; architecture: mcts-v1; named identities: v2-padded | v2-refit-padded)")),
    ];

    fn rendered(spec: &str) -> Result<String, String> {
        Contestant::from_str_name(spec).map(|contestant| format!("{contestant:?}"))
    }

    #[test]
    fn the_promoted_grammar_reproduces_the_example_private_parser_exactly() {
        // Anti-vacuity: a table that shrank is a table that stopped covering
        // the arm someone deleted, and the assertion loop below would pass
        // silently on an empty slice.
        assert_eq!(FROZEN_BATTERY.len(), 77, "the frozen battery lost rows");
        assert_eq!(
            FROZEN_BATTERY.iter().filter(|(_, expected)| expected.is_err()).count(),
            37,
            "the frozen battery lost refusal rows (the module doc quotes this count)",
        );

        for (spec, expected) in FROZEN_BATTERY {
            let got = rendered(spec);
            let got: Result<&str, &str> = got.as_ref().map(String::as_str).map_err(String::as_str);
            assert_eq!(got, *expected, "spec {spec:?} parsed differently than at HEAD a9029b8");
        }
    }

    #[test]
    fn every_bare_name_is_its_controllers_shipped_default() {
        // The Debug table above pins the same facts as strings; this pins them
        // STRUCTURALLY, against the consts, so a renamed field or a reordered
        // struct cannot make both pass by coincidence.
        assert_eq!(Contestant::from_str_name("uniform"), Ok(Contestant::Policy(RolloutPolicy::Uniform)));
        assert_eq!(
            Contestant::from_str_name("retreat-averse"),
            Ok(Contestant::Policy(RolloutPolicy::RetreatAverse)),
        );
        assert_eq!(
            Contestant::from_str_name("family-weighted"),
            Ok(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)),
        );
        assert_eq!(
            Contestant::from_str_name(greedy::GREEDY_CONTROLLER),
            Ok(Contestant::GREEDY_V1_DEFAULT),
        );
        assert_eq!(
            Contestant::from_str_name(lookahead::LOOKAHEAD_CONTROLLER),
            Ok(Contestant::LOOKAHEAD_V1_DEFAULT),
        );
        assert_eq!(
            Contestant::from_str_name(lookahead::ATOMIC_DIAG_CONTROLLER),
            Ok(Contestant::ATOMIC_DIAG_DEFAULT),
        );
        assert_eq!(Contestant::from_str_name(mcts::MCTS_CONTROLLER), Ok(Contestant::MCTS_V1_DEFAULT));
    }

    #[test]
    fn the_short_atomic_diag_alias_is_the_provenance_name() {
        assert_ne!(ATOMIC_DIAG_ALIAS, lookahead::ATOMIC_DIAG_CONTROLLER);
        assert_eq!(
            Contestant::from_str_name(ATOMIC_DIAG_ALIAS),
            Contestant::from_str_name(lookahead::ATOMIC_DIAG_CONTROLLER),
        );
        // …and the alias carries the same knob grammar, not a knobless stub.
        assert_eq!(
            Contestant::from_str_name("atomic-diag-v1:d=3,k=2,m=4"),
            Contestant::from_str_name("lookahead-atomic-diag-v1:d=3,k=2,m=4"),
        );
        assert_eq!(
            Contestant::from_str_name("atomic-diag-v1:d=3,k=2,m=4"),
            Ok(Contestant::AtomicDiag { d: 3, k: 2, m: 4 }),
        );
    }

    #[test]
    fn an_absent_cap_is_not_any_set_cap() {
        // The T0-golden byte-inertness fact: OMITTING `cap` is the shipped 3.3
        // rung, and the descriptor emits no `cap=` token for it — which is what
        // keeps every recorded 3.3 figure describing this code. A grammar that
        // defaulted `cap` to some large number instead of `None` would pass a
        // Debug-string test written from ITS OWN output and silently re-label
        // every uncapped cell.
        let uncapped = Contestant::from_str_name("lookahead-v1").expect("bare name parses");
        assert_eq!(uncapped, Contestant::LOOKAHEAD_V1_DEFAULT);
        assert!(matches!(uncapped, Contestant::LookaheadV1 { cap: None, .. }));
        assert!(!uncapped.descriptor().2.contains_key("cap"), "uncapped S1 emitted a cap token");

        let capped = Contestant::from_str_name("lookahead-v1:cap=750").expect("cap knob parses");
        assert!(matches!(capped, Contestant::LookaheadV1 { cap: Some(750), .. }));
        assert_ne!(capped, uncapped);
        assert_eq!(capped.descriptor().2.get("cap"), Some(&750_i64));

        let tree = Contestant::from_str_name(mcts::MCTS_CONTROLLER).expect("bare name parses");
        assert!(matches!(tree, Contestant::MctsV1 { cap: None, .. }));
        assert!(!tree.descriptor().2.contains_key("cap"), "uncapped tree emitted a cap token");
        let capped_tree = Contestant::from_str_name("mcts-v1:cap=750").expect("cap knob parses");
        assert_eq!(capped_tree.descriptor().2.get("cap"), Some(&750_i64));
    }

    #[test]
    fn every_knob_moves_its_own_field_and_leaves_the_others_shipped() {
        assert_eq!(Contestant::from_str_name("greedy-v1:m=3"), Ok(Contestant::GreedyV1 { m: 3 }));
        assert_eq!(
            Contestant::from_str_name("lookahead-v1:b=7"),
            Ok(Contestant::LookaheadV1 { b: 7, k: DEFAULT_WORLDS, m: DEFAULT_RETREAT_SAMPLES, cap: None }),
        );
        assert_eq!(
            Contestant::from_str_name("lookahead-v1:k=3"),
            Ok(Contestant::LookaheadV1 { b: DEFAULT_BEAM, k: 3, m: DEFAULT_RETREAT_SAMPLES, cap: None }),
        );
        assert_eq!(
            Contestant::from_str_name("lookahead-v1:m=2"),
            Ok(Contestant::LookaheadV1 { b: DEFAULT_BEAM, k: DEFAULT_WORLDS, m: 2, cap: None }),
        );
        assert_eq!(
            Contestant::from_str_name("atomic-diag-v1:d=4"),
            Ok(Contestant::AtomicDiag { d: 4, k: DEFAULT_WORLDS, m: DEFAULT_RETREAT_SAMPLES }),
        );
        for leaf in [LeafRule::HandOff, LeafRule::MidTurnDiag, LeafRule::ConstantDiag] {
            assert_eq!(
                Contestant::from_str_name(&format!("mcts-v1:leaf={}", leaf.as_str())),
                Ok(Contestant::MctsV1 {
                    cap: None,
                    c_ppm: DEFAULT_EXPLORATION_PPM,
                    scale: DEFAULT_SQUASH_SCALE,
                    m: DEFAULT_RETREAT_SAMPLES,
                    leaf,
                }),
            );
        }
        assert_eq!(
            Contestant::from_str_name("mcts-v1:c=2000000,scale=50,m=1"),
            Ok(Contestant::MctsV1 {
                cap: None,
                c_ppm: 2_000_000,
                scale: 50,
                m: 1,
                leaf: LeafRule::HandOff,
            }),
        );
        // The two shipped `FamilyWeighted` settings §0-R3(i)'s guaranteed edge
        // is built from — the reason the rung knobs exist at all.
        assert_eq!(
            Contestant::from_str_name("family-weighted:p=50000,k=5"),
            Ok(Contestant::Policy(RolloutPolicy::FamilyWeighted {
                retreat_ppm: 50_000,
                recovery_min_of: 5,
            })),
        );
        // Typing the shipped knobs explicitly must land on the shipped const —
        // the round trip that would catch a defaults table drifting from the
        // policy's own.
        let RolloutPolicy::FamilyWeighted { retreat_ppm, recovery_min_of } =
            RolloutPolicy::FAMILY_WEIGHTED_DEFAULT
        else {
            unreachable!("FAMILY_WEIGHTED_DEFAULT is the weighted variant")
        };
        assert_eq!(
            Contestant::from_str_name(&format!(
                "family-weighted:p={retreat_ppm},k={recovery_min_of}"
            )),
            Ok(Contestant::Policy(RolloutPolicy::FAMILY_WEIGHTED_DEFAULT)),
        );
    }

    #[test]
    fn unknown_is_an_error_and_the_error_names_what_it_saw() {
        // The contract, stated as a property rather than as 30 frozen strings:
        // whatever the caller typed wrong appears in the message. A refusal
        // that did not name the token would send a reader back to the source.
        let cases = [
            ("no-such-contestant", "no-such-contestant"),
            ("greedy-v1:zzz=1", "zzz"),
            ("lookahead-v1:zzz=1", "zzz"),
            ("atomic-diag-v1:zzz=1", "zzz"),
            ("mcts-v1:zzz=1", "zzz"),
            ("family-weighted:zzz=1", "zzz"),
            // …and the rung arm names the CONTESTANT too, exactly as the four
            // evaluator/architecture arms always did (W5's deferred repair).
            ("family-weighted:zzz=1", "family-weighted"),
            ("mcts-v1:leaf=zzz", "zzz"),
            ("uniform:p=5", "uniform"),
            ("greedy-v1:zzz", "zzz"),
        ];
        for (spec, token) in cases {
            let message = Contestant::from_str_name(spec).expect_err("spec must be refused");
            assert!(
                message.contains(token),
                "refusal of {spec:?} did not name {token:?}: {message}",
            );
        }
    }

    #[test]
    fn a_knob_on_a_knobless_rung_is_refused_rather_than_ignored() {
        for spec in ["uniform:p=5", "retreat-averse:k=1", "uniform:", "retreat_averse:m=1"] {
            assert!(
                Contestant::from_str_name(spec).is_err(),
                "{spec:?} was accepted; a silently-ignored knob puts a cell's figure under knobs it did not run",
            );
        }
    }

    #[test]
    fn a_zero_budget_is_refused_on_both_sides_of_the_shared_cap() {
        for spec in ["lookahead-v1:cap=0", "mcts-v1:cap=0"] {
            let message = Contestant::from_str_name(spec).expect_err("cap=0 must be refused");
            assert!(message.contains("omit the knob"), "{spec:?}: {message}");
        }
    }

    /// **V4 §7 STEP 8 T3 — THE NAMED-IDENTITY ARMS ARE CONTESTANT NAMES**
    /// (spec §13's step-8 blast-radius addition: "the ARENA gains an
    /// explicit-table / named-identity contestant surface … so the §11 arms are
    /// runnable identities rather than names").
    ///
    /// The two wire names are written as LITERALS here, deliberately, on
    /// [`ATOMIC_DIAG_ALIAS`]'s and `contestant_golden`'s
    /// `ECHO_LOOKAHEAD_PREFIX` reasoning: these are the strings an owner types
    /// at a runner and a corpus reader reads back, so they are exactly the
    /// tokens a rename must be unable to slip past. The GREEN half of this wave
    /// pins the literals against the arms' own name consts, so there is still
    /// one source and this test is the check on it rather than a second one.
    ///
    /// **Born RED** at the pre-arm tree, where both names refuse with the
    /// grammar's typed unknown-contestant sentence — the refusal this surface
    /// replaces, recorded in the RED commit message.
    #[test]
    fn the_named_identity_arms_are_contestant_names_the_grammar_accepts() {
        for name in ["v2-padded", "v2-refit-padded"] {
            let parsed = Contestant::from_str_name(name)
                .unwrap_or_else(|e| panic!("{name:?} must be a contestant name: {e}"));
            // A named arm is an evaluator seat: it resolves its dealt pair by
            // `greedy::score_starting_order`, exactly as every other evaluator
            // contestant does, so a §11 cell against one stays Keep-homogeneous.
            assert_eq!(
                parsed.keep_kind(),
                crate::game::arena::KeepKind::EvaluatorScored,
                "{name:?} must keep by the evaluator rule",
            );
            // The echo carries the NAME and the table IDENTITY, composed through
            // the one `compose_echo` every contestant echoes through.
            let echo = parsed.echo();
            assert!(
                echo.starts_with(&format!("{name}@v")),
                "{name:?} must echo under its own name: {echo}",
            );
            assert!(echo.contains("weights="), "{name:?} must echo its table identity: {echo}");
        }

        // The two arms are DIFFERENT identities (the T2 review's carry): a cell
        // that could conflate them would report one arm's figures under the
        // other's name.
        let padded = Contestant::from_str_name("v2-padded").expect("v2-padded parses");
        let refit = Contestant::from_str_name("v2-refit-padded").expect("v2-refit-padded parses");
        assert_ne!(padded, refit, "the two named arms must not be the same contestant");
        assert_ne!(padded.echo(), refit.echo(), "the two named arms must not share an echo");
    }

    /// A named identity carries NO knobs, and typing one is refused rather than
    /// ignored — `eval_screen`'s rule (`ScreenRung` "has no knob fields: each
    /// variant runs its controller's SHIPPED knobs … The screen's subject is the
    /// WEIGHTS; a knob that moved in the same cell would confound the only thing
    /// it measures"), which is the whole reason these arms exist.
    ///
    /// **Born RED**: at the pre-arm tree the refusal is the unknown-contestant
    /// sentence, not the knobless one.
    #[test]
    fn a_knob_on_a_named_identity_arm_is_refused_rather_than_ignored() {
        for spec in ["v2-padded:b=8", "v2-refit-padded:m=1", "v2-padded:"] {
            let message = Contestant::from_str_name(spec)
                .map(|c| format!("{c:?}"))
                .expect_err("a knob on a knobless named arm must be refused");
            assert!(
                message.contains("has no knobs"),
                "{spec:?} must be refused as knobless, not as unknown: {message}",
            );
        }
    }
}
