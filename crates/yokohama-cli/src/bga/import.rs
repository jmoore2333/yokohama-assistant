//! The `yokohama-cli import` entry point (plan T6, extended by H2's T8–T10).
//!
//! Command shape (plan §1): `yokohama-cli import <capture.ndjson> [--table
//! <id>] [--answers] [--through <move_id>] [--out <record.json>] [--report
//! <path>]`. Dispatch in `main.rs` is on `args[1] == "import"` as an EXACT
//! keyword; every other `args[1]` keeps its "resume this save" meaning.
//!
//! Outcomes, honestly separated:
//! - `--answers`: print the H1 evidence sheet and stop.
//! - Otherwise (H2): resolve the setup (evidence + declared assumptions +
//!   replay-decided hypotheses), build the live-mode `Session`, and run the
//!   catch-up REPLAY through the checked apply boundary, cross-checked against
//!   BGA counters (T9) and BGA's final scoring at game end. A hypothesis slot
//!   (a claimed achievement matching two engine types) runs the replay once
//!   per candidate; exactly one surviving assignment is the accepted answer,
//!   zero reports the best-progress STOP, more than one is its own error.

use super::resolve::{resolve, Resolved};
use super::setup::{build_session, render_sheet, synthesize, Evidence};
use super::translate::{ReplayReport, Translator};
use super::ImportError;
use std::path::PathBuf;
use yokohama_bridge::feed::{read_and_collapse, CollapsedFeed};
use yokohama_bridge::sanitize::sanitize;
use yokohama_core::Rules;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportArgs {
    pub path: PathBuf,
    pub table: Option<u64>,
    pub answers_only: bool,
    pub through: Option<u64>,
    pub out: Option<PathBuf>,
    pub report: Option<PathBuf>,
    /// H3: tail the capture as the relay appends, re-replaying and re-rendering
    /// as state advances (`follow.rs`). A flag on `import` rather than its own
    /// keyword because design §2.3 makes catch-up and live-follow the SAME
    /// translator on two feeds — one entry point, one place that knows how a
    /// `Session` is built.
    pub follow: bool,
}

/// The full `import` flag surface, in the order the usage string lists them.
/// EXPORTED so `guide_sync` can pin the guide pair against it both ways (every
/// flag documented in both formats, every `--flag` the guides carry parses) —
/// the argv surface was outside guide_sync's slice, so the guide could document
/// a flag the parser lacks with the whole suite green (F7 / lens-4). Kept in
/// step with `parse_args`'s match arms and the `usage` string below; the unit
/// test `every_documented_flag_is_a_real_parser_flag` pins that coupling.
pub const FLAGS: &[&str] = &["--table", "--answers", "--through", "--out", "--report", "--follow"];

/// Parse everything after the `import` keyword.
pub fn parse_args(args: &[String]) -> Result<ImportArgs, ImportError> {
    let usage = "import <capture.ndjson> [--table <id>] [--answers] [--through <move_id>] \
                 [--out <record.json>] [--report <path>] [--follow]";
    let mut path: Option<PathBuf> = None;
    let mut table: Option<u64> = None;
    let mut answers_only = false;
    let mut through: Option<u64> = None;
    let mut out: Option<PathBuf> = None;
    let mut report: Option<PathBuf> = None;
    let mut follow = false;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--follow" => follow = true,
            "--table" => {
                let v = it.next().ok_or_else(|| ImportError::Usage {
                    detail: format!("--table needs a value; {usage}"),
                })?;
                table = Some(v.parse::<u64>().map_err(|_| ImportError::Usage {
                    detail: format!("--table `{v}` is not a table id; {usage}"),
                })?);
            }
            "--answers" => answers_only = true,
            "--through" => {
                let v = it.next().ok_or_else(|| ImportError::Usage {
                    detail: format!("--through needs a move id; {usage}"),
                })?;
                through = Some(v.parse::<u64>().map_err(|_| ImportError::Usage {
                    detail: format!("--through `{v}` is not a move id; {usage}"),
                })?);
            }
            "--out" => {
                let v = it.next().ok_or_else(|| ImportError::Usage {
                    detail: format!("--out needs a path; {usage}"),
                })?;
                out = Some(PathBuf::from(v));
            }
            "--report" => {
                let v = it.next().ok_or_else(|| ImportError::Usage {
                    detail: format!("--report needs a path; {usage}"),
                })?;
                report = Some(PathBuf::from(v));
            }
            other if other.starts_with('-') => {
                return Err(ImportError::Usage {
                    detail: format!("unknown flag `{other}`; {usage}"),
                });
            }
            other => {
                if path.replace(PathBuf::from(other)).is_some() {
                    return Err(ImportError::Usage {
                        detail: format!("more than one capture path; {usage}"),
                    });
                }
            }
        }
    }
    if follow {
        // The batch-only flags name products of a run that ENDS; the follower
        // does not end on its own, so combining them would silently mean
        // something other than what was typed. Refused, never reinterpreted.
        for (flag, set) in [
            ("--answers", answers_only),
            ("--through", through.is_some()),
            ("--report", report.is_some()),
        ] {
            if set {
                return Err(ImportError::Usage {
                    detail: format!(
                        "`--follow` cannot be combined with `{flag}` — the follower tails the \
                         live capture and has no batch bound or one-shot report; {usage}"
                    ),
                });
            }
        }
    }
    Ok(ImportArgs {
        path: path.ok_or_else(|| ImportError::Usage {
            detail: format!("missing capture path; {usage}"),
        })?,
        table,
        answers_only,
        through,
        out,
        report,
        follow,
    })
}

/// What a run produced.
#[derive(Debug)]
pub struct RunOutcome {
    pub text: String,
    pub session_built: bool,
    /// Full replay reached the engine's game end with the final-score
    /// cross-check green.
    pub replay_complete: bool,
}

/// Per-column `(player_name, total)` pairs from a finalScoring `table` value,
/// aligned BY COLUMN INDEX: column `i`'s name comes from `header[i]` and its
/// total from `total_row[i]`, and a column missing either is dropped WITHOUT
/// shifting its neighbours. The rev-5 ancestor zipped two independently
/// FILTERED lists, which stayed aligned only because every header cell in the
/// corpus (the Dummy's included) carries `player_name` — H2 review NOTE-3's
/// "align by column index before trusting the cross-check", owed before the
/// endgame cross-check became reachable at rev 6.
fn totals_from_table(table: &[serde_json::Value]) -> Option<Vec<(String, i64)>> {
    let header = table.first()?.as_array()?;
    let total_row = table.iter().find_map(|row| {
        let row = row.as_array()?;
        (row.first()?.as_str()? == "Total").then_some(row.clone())
    })?;
    let mut out = Vec::new();
    for (i, cell) in header.iter().enumerate().skip(1) {
        let Some(name) = cell
            .get("args")
            .and_then(|a| a.get("player_name"))
            .and_then(|p| p.as_str())
        else {
            continue;
        };
        let total = match total_row.get(i) {
            Some(serde_json::Value::Number(x)) => x.as_i64(),
            Some(serde_json::Value::String(s)) => s.parse().ok(),
            _ => None,
        };
        if let Some(t) = total {
            out.push((name.to_string(), t));
        }
    }
    Some(out)
}

/// BGA's own final-scoring table, parsed from the `tableWindow` notification:
/// per-column totals keyed by the header names (see [`totals_from_table`]).
fn bga_final_totals(feed: &CollapsedFeed) -> Option<Vec<(String, i64)>> {
    for mv in feed.moves.iter().rev() {
        for pk in [mv.socket.as_ref(), mv.history.as_ref()].into_iter().flatten() {
            for n in &pk.notifications {
                if n.kind != "tableWindow"
                    || n.args.get("id").and_then(|i| i.as_str()) != Some("finalScoring")
                {
                    continue;
                }
                let table = n.args.get("table")?.as_array()?;
                return totals_from_table(table);
            }
        }
    }
    None
}

pub(crate) struct AttemptOutcome {
    pub(crate) label: String,
    pub(crate) report: ReplayReport,
    pub(crate) session: crate::session::Session,
    pub(crate) stop: Option<String>,
}

fn run_replay_attempt(
    resolved: &Resolved,
    answers: &super::setup::SetupAnswers,
    feed: &CollapsedFeed,
    through: Option<u64>,
    rules: &Rules,
    label: String,
) -> Result<AttemptOutcome, ImportError> {
    let session = build_session(answers, rules.clone())?;
    let seats = answers.seats.clone();
    let mapper = super::orders::OrderMapper::from_rules(rules)?;
    let mut stream = super::facts::extract(
        feed,
        &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
        &mapper,
    )?;
    if let Some(bound) = through {
        stream.facts.retain(|tf| tf.move_id <= bound);
        stream.checkpoints.retain(|cp| cp.move_id <= bound);
    }
    let mut translator = Translator::new(session, resolved, stream, feed)?;
    let stop = match translator.run() {
        Ok(()) => None,
        Err(e) => Some(e.to_string()),
    };
    let (session, report) = translator.into_session();
    Ok(AttemptOutcome { label, report, session, stop })
}

/// The whole hypothesis-looped replay over an already-collapsed feed — the
/// shared core of the batch `run` and the H3 follower. `best` is the attempt a
/// caller should look at: the first SURVIVOR when any attempt ran the whole
/// stream clean, else the best-progress attempt (whose `stop` carries the
/// honest STOP story). Batch and follow differ only in what they do with
/// MULTIPLE survivors: batch refuses (an ended capture that does not
/// discriminate needs owner input), the follower keeps tailing (mid-game,
/// undiscriminated hypotheses are the EXPECTED state until the claims arrive).
pub(crate) struct Replays {
    pub(crate) resolved: Resolved,
    pub(crate) attempts: Vec<AttemptOutcome>,
    pub(crate) survivors: Vec<usize>,
    pub(crate) best: usize,
}

pub(crate) fn replay_attempts(
    answers: super::setup::SetupAnswers,
    feed: &CollapsedFeed,
    rules: &Rules,
    through: Option<u64>,
) -> Result<Replays, ImportError> {
    let resolved = {
        let seats = answers.seats.clone();
        let mapper = super::orders::OrderMapper::from_rules(rules)?;
        let stream = super::facts::extract(
            feed,
            &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
            &mapper,
        )?;
        resolve(answers, feed, &stream, rules)?
    };

    // Hypothesis combinations: the cartesian product over the (small) list.
    let mut combos: Vec<Vec<(usize, yokohama_core::AchievementType)>> = vec![Vec::new()];
    for h in &resolved.hypotheses {
        let mut next = Vec::new();
        for combo in &combos {
            for cand in &h.candidates {
                let mut c = combo.clone();
                c.push((h.category, *cand));
                next.push(c);
            }
        }
        combos = next;
    }

    let mut attempts: Vec<AttemptOutcome> = Vec::new();
    for combo in &combos {
        let mut answers = resolved.answers.clone();
        let mut label_parts = Vec::new();
        for (cat, ty) in combo {
            answers.achievements[*cat] = Evidence::Evidenced {
                value: *ty,
                provenance: format!("hypothesis candidate for claimed category {cat}"),
            };
            label_parts.push(format!("{}={:?}", ["A", "B", "C"][*cat], ty));
        }
        let label = if label_parts.is_empty() {
            "no hypotheses".to_string()
        } else {
            label_parts.join(", ")
        };
        attempts.push(run_replay_attempt(&resolved, &answers, feed, through, rules, label)?);
    }

    let survivors: Vec<usize> = attempts
        .iter()
        .enumerate()
        .filter(|(_, a)| a.stop.is_none())
        .map(|(i, _)| i)
        .collect();
    let best = match survivors.first() {
        Some(&first) => first,
        None => attempts
            .iter()
            .enumerate()
            .max_by_key(|(_, a)| a.report.last_move_id)
            .map(|(i, _)| i)
            .expect("at least one attempt ran"),
    };
    Ok(Replays { resolved, attempts, survivors, best })
}

/// The terminal cross-check text (the honesty dividend): engine final scores
/// against BGA's own `finalScoring` totals, per seat. Empty when the engine
/// state is not at game end.
pub(crate) fn final_check_text(
    session: &crate::session::Session,
    resolved: &Resolved,
    feed: &CollapsedFeed,
) -> String {
    let mut final_check = String::new();
    if session.is_over() {
        let engine_scores = session.final_scores();
        match bga_final_totals(feed) {
            Some(bga) => {
                final_check.push_str("  final-score cross-check (engine vs BGA):\n");
                for (pid, score) in &engine_scores {
                    let seat = pid.0 as usize;
                    // The seat name is a captured display name; the lookup keys
                    // on the raw value but the DISPLAYED name is escaped (F1).
                    let name = &resolved.answers.seats[seat].name;
                    let shown = sanitize(name);
                    let bga_total = bga.iter().find(|(n, _)| n == name).map(|(_, t)| *t);
                    match bga_total {
                        Some(t) if t == *score as i64 => {
                            final_check
                                .push_str(&format!("    seat {seat} ({shown}): {score} == BGA {t} ✓\n"));
                        }
                        Some(t) => {
                            final_check.push_str(&format!(
                                "    seat {seat} ({shown}): engine {score} vs BGA {t} — MISMATCH\n"
                            ));
                        }
                        None => final_check
                            .push_str(&format!("    seat {seat} ({shown}): no BGA column found\n")),
                    }
                }
            }
            None => final_check.push_str("  no BGA finalScoring table in the capture\n"),
        }
    }
    final_check
}

fn render_report(
    outcome: &AttemptOutcome,
    resolved: &Resolved,
    feed: &CollapsedFeed,
    final_check: &str,
) -> String {
    let mut s = String::new();
    let r = &outcome.report;
    s.push_str(&format!("replay attempt [{}]\n", outcome.label));
    s.push_str(&format!(
        "  progressed through BGA move {} — {} engine moves, {} reveal events, {} checkpoints green\n",
        r.last_move_id, r.engine_moves, r.reveal_events, r.checkpoints_passed
    ));
    match &outcome.stop {
        None => s.push_str("  ran to the end of the captured stream\n"),
        Some(stop) => s.push_str(&format!("  STOP: {stop}\n")),
    }
    for d in &r.diffs {
        s.push_str(&format!(
            "  T9 diff at move {}: seat {} `{}` BGA {} vs engine {}\n",
            d.move_id, d.seat, d.counter, d.bga, d.engine
        ));
    }
    s.push_str(final_check);
    s.push_str(&render_report_sections(r, resolved));
    let _ = feed;
    s
}

/// The report's three DECLARED-HONESTY sections — the notes (including the D1
/// church / D2 extra-tech open-rules declarations), the declared assumptions,
/// and the deliberately-unchecked T9 counters. Shared by the batch report
/// (`render_report`) and the live-follow frame (`follow::recompute`) so the two
/// surfaces cannot drift (F4 / lens-4): the runthrough operator watching the
/// spectate frame must see WHY the panel's counts differ from BGA's screen, and
/// that explanation lives in exactly these lines. Every captured-derived string
/// is escaped at this display boundary (F1).
pub(crate) fn render_report_sections(report: &ReplayReport, resolved: &Resolved) -> String {
    let mut s = String::new();
    for n in &report.notes {
        // A note can embed a captured notification `kind` (the post-game
        // cleanup note), so it is escaped here.
        s.push_str(&format!("  note: {}\n", sanitize(n)));
    }
    if !resolved.assumptions.is_empty() {
        s.push_str("  declared assumptions (census §2):\n");
        for a in &resolved.assumptions {
            s.push_str(&format!(
                "    - {}: picked {} from [{}] — inert because {}\n",
                sanitize(&a.what),
                sanitize(&a.picked),
                sanitize(&a.candidates),
                sanitize(&a.why_inert),
            ));
        }
    }
    s.push_str("  deliberately-unchecked counters (T9):\n");
    for (k, why) in &report.unchecked_counters {
        s.push_str(&format!("    - {k}: {why}\n"));
    }
    s
}

pub fn run(args: &ImportArgs, rules: Rules) -> Result<RunOutcome, ImportError> {
    let text = std::fs::read_to_string(&args.path).map_err(|e| ImportError::Io {
        path: args.path.display().to_string(),
        detail: e.to_string(),
    })?;
    let feed = read_and_collapse(&text, args.table)?;
    let answers = synthesize(&feed, &rules)?;
    let mut out_text = render_sheet(&answers);
    out_text.push_str(&format!(
        "  feed: {} moves ({} socket-enriched), {} supersession event(s), {} snapshot(s)\n",
        feed.report.moves,
        feed.report.enriched,
        feed.report.superseded.len(),
        feed.report.snapshots,
    ));
    if args.answers_only {
        return Ok(RunOutcome { text: out_text, session_built: false, replay_complete: false });
    }

    // ---- H2: resolution + the hypothesis-looped replay ---------------------
    let reps = replay_attempts(answers, &feed, &rules, args.through)?;
    let Replays { resolved, attempts, survivors, best } = &reps;
    let chosen = match survivors.as_slice() {
        [one] => *one,
        [] => *best, // Best progress carries the honest STOP story.
        several => {
            // Distinguishable only if their final states differ; with our
            // constraint translation every survivor replayed the same moves,
            // so surviving twice means the claim did not discriminate.
            return Err(ImportError::Sheet {
                detail: format!(
                    "{} hypothesis assignments survived the whole replay ({:?}) — the capture \
                     does not discriminate them; owner input needed",
                    several.len(),
                    several.iter().map(|i| attempts[*i].label.clone()).collect::<Vec<_>>()
                ),
            });
        }
    };
    let outcome = &attempts[chosen];

    // ---- terminal cross-check (the honesty dividend) -----------------------
    let replay_complete = outcome.stop.is_none() && outcome.session.is_over();
    let final_check = final_check_text(&outcome.session, resolved, &feed);

    out_text.push_str(&render_report(outcome, resolved, &feed, &final_check));
    if attempts.len() > 1 {
        out_text.push_str("  hypothesis attempts:\n");
        for a in attempts {
            out_text.push_str(&format!(
                "    [{}] → {}\n",
                a.label,
                match &a.stop {
                    None => "replayed clean".to_string(),
                    Some(s) => format!("stopped: {}", s.lines().next().unwrap_or(s)),
                }
            ));
        }
    }

    if let Some(out_path) = &args.out {
        outcome.session.save(out_path).map_err(|e| ImportError::Session { detail: e })?;
        out_text.push_str(&format!("  record saved to {}\n", out_path.display()));
    }
    if let Some(report_path) = &args.report {
        std::fs::write(report_path, &out_text).map_err(|e| ImportError::Io {
            path: report_path.display().to_string(),
            detail: e.to_string(),
        })?;
    }
    Ok(RunOutcome { text: out_text, session_built: true, replay_complete })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// H2 review NOTE-3, discharged at rev 6 before the endgame cross-check
    /// became reachable: finalScoring names and totals align BY COLUMN INDEX.
    /// The middle "spacer" column here has no `player_name`; the rev-5
    /// zip-of-filtered-lists paired Player_B with the spacer's total 99 and
    /// the Dummy with Player_B's 41 — this pin fails against that shape.
    #[test]
    fn final_totals_align_by_column_index_across_an_unnamed_column() {
        let table: Vec<serde_json::Value> = serde_json::from_value(serde_json::json!([
            [
                "",
                { "str": "${player_name}", "args": { "player_name": "Player_A" }, "type": "header" },
                { "str": "spacer, no player_name", "args": {}, "type": "header" },
                { "str": "${player_name}", "args": { "player_name": "Player_B" }, "type": "header" },
                { "str": "${player_name}", "args": { "player_name": "Dummy player" }, "type": "header" }
            ],
            ["Total", 154, 99, 41, 9]
        ]))
        .unwrap();
        assert_eq!(
            totals_from_table(&table).unwrap(),
            vec![
                ("Player_A".to_string(), 154),
                ("Player_B".to_string(), 41),
                ("Dummy player".to_string(), 9),
            ],
            "a column missing player_name must be dropped without shifting its neighbours"
        );
    }

    /// The corpus's actual shape: every column named, totals numeric, string
    /// totals accepted.
    #[test]
    fn final_totals_read_the_corpus_shape_and_string_numbers() {
        let table: Vec<serde_json::Value> = serde_json::from_value(serde_json::json!([
            [
                "",
                { "str": "${player_name}", "args": { "player_name": "Player_A" }, "type": "header" },
                { "str": "${player_name}", "args": { "player_name": "Player_B" }, "type": "header" }
            ],
            ["Points", 10, 20],
            ["Total", "154", 41]
        ]))
        .unwrap();
        assert_eq!(
            totals_from_table(&table).unwrap(),
            vec![("Player_A".to_string(), 154), ("Player_B".to_string(), 41)]
        );
    }

    #[test]
    fn args_parse_the_documented_shape() {
        let a = parse_args(&strs(&[
            "cap.ndjson", "--table", "895724349", "--answers", "--out", "rec.json",
            "--through", "50", "--report", "rep.txt",
        ]))
        .unwrap();
        assert_eq!(a.path, PathBuf::from("cap.ndjson"));
        assert_eq!(a.table, Some(895_724_349));
        assert!(a.answers_only);
        assert_eq!(a.through, Some(50));
        assert_eq!(a.out, Some(PathBuf::from("rec.json")));
        assert_eq!(a.report, Some(PathBuf::from("rep.txt")));
        assert!(!a.follow, "a batch invocation must not read as a follow");
    }

    /// H3 review SHOULD-2: `--follow` parsing and its refusal set were
    /// user-visible, guide-documented, and untested. `--follow` composes with
    /// the flags that make sense for a run that never ends on its own
    /// (`--table`, `--out`) and is REFUSED — with the offending flag named in
    /// the error — alongside each batch-only flag (`--answers`, `--through`,
    /// `--report`). Discrimination verified by mutation: deleting the refusal
    /// loop in `parse_args` fails the three refusal assertions here.
    #[test]
    fn follow_parses_with_its_allowed_flags_and_refuses_the_batch_only_ones() {
        let a = parse_args(&strs(&[
            "cap.ndjson", "--follow", "--table", "895724349", "--out", "rec.json",
        ]))
        .unwrap();
        assert!(a.follow);
        assert_eq!(a.table, Some(895_724_349));
        assert_eq!(a.out, Some(PathBuf::from("rec.json")));
        assert!(!a.answers_only);
        assert_eq!(a.through, None);
        assert_eq!(a.report, None);

        for (extra, named) in [
            (vec!["--answers"], "--answers"),
            (vec!["--through", "50"], "--through"),
            (vec!["--report", "rep.txt"], "--report"),
        ] {
            let mut argv = vec!["cap.ndjson", "--follow"];
            argv.extend(extra);
            match parse_args(&strs(&argv)).unwrap_err() {
                ImportError::Usage { detail } => assert!(
                    detail.contains(named) && detail.contains("--follow"),
                    "refusal for {named} does not name the conflict: {detail}"
                ),
                other => panic!("expected a Usage refusal for {named}, got {other:?}"),
            }
        }
        // Flag order must not matter to the refusal: `--follow` last.
        assert!(matches!(
            parse_args(&strs(&["cap.ndjson", "--answers", "--follow"])).unwrap_err(),
            ImportError::Usage { .. }
        ));
    }

    /// F7 (lens-4): the exported `FLAGS` set is what `guide_sync` pins the guide
    /// pair against, so it must equal the parser's REAL flags — every one is
    /// accepted (never an "unknown flag") when given with a plausible value.
    /// This is the coupling that lets guide_sync trust `FLAGS`; a flag added to
    /// the parser but not to `FLAGS` (or vice versa) is caught here or there.
    #[test]
    fn every_documented_flag_is_a_real_parser_flag() {
        for flag in FLAGS {
            let argv = match *flag {
                "--table" | "--through" => vec!["c.ndjson", flag, "1"],
                "--out" | "--report" => vec!["c.ndjson", flag, "x"],
                _ => vec!["c.ndjson", flag], // valueless: --answers, --follow
            };
            let parsed = parse_args(&strs(&argv));
            assert!(
                !matches!(&parsed, Err(ImportError::Usage { detail }) if detail.contains("unknown flag")),
                "FLAGS lists `{flag}` but the parser rejects it as unknown: {parsed:?}"
            );
        }
    }

    #[test]
    fn unknown_flags_and_missing_paths_are_usage_errors() {
        assert!(matches!(
            parse_args(&strs(&["cap.ndjson", "--force"])).unwrap_err(),
            ImportError::Usage { .. }
        ));
        assert!(matches!(parse_args(&[]).unwrap_err(), ImportError::Usage { .. }));
        assert!(matches!(
            parse_args(&strs(&["a.ndjson", "b.ndjson"])).unwrap_err(),
            ImportError::Usage { .. }
        ));
    }

    #[test]
    fn a_missing_file_is_a_typed_io_error() {
        let args = parse_args(&strs(&["/nonexistent/cap.ndjson"])).unwrap();
        assert!(matches!(
            run(&args, Rules::standard()).unwrap_err(),
            ImportError::Io { .. }
        ));
    }

    /// F1 (lens-1 MUST), the BATCH end-to-end: a capture whose `payload.source`
    /// carries terminal control bytes is untrusted input that reaches the
    /// terminal on the normal error path (`main.rs`'s `eprintln!`). The file →
    /// feed → `ImportError::Feed` → Display chain must scrub it: the run refuses
    /// (unknown source) and the refusal string carries ZERO ESC bytes.
    #[test]
    fn a_hostile_payload_source_reaches_the_error_path_with_no_escape_bytes() {
        let dir = std::env::temp_dir().join(format!(
            "bga-hostile-source-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let cap = dir.join("hostile.ndjson");
        // The ESC/BEL are JSON-escaped in the FILE (``), so JSON parsing
        // decodes them into a source value that genuinely carries the control
        // bytes — the real attack shape, reaching the source switch (and OUR
        // sanitizer) rather than being rejected as a raw control byte at parse
        // time. body_text present so the parser reaches the source switch.
        let line = "{\"received_at_ms\":1,\"kind\":\"packets\",\"payload\":\
                    {\"source\":\"\\u001b[2J\\u0007pwn\",\"body_text\":\"{}\"}}\n";
        std::fs::write(&cap, line).unwrap();
        let args = parse_args(&strs(&[&cap.display().to_string()])).unwrap();
        let err = run(&args, Rules::standard()).unwrap_err();
        let shown = err.to_string();
        assert!(!shown.contains('\u{1b}'), "no raw ESC byte on the error path: {shown:?}");
        assert!(!shown.contains('\u{07}'), "no raw BEL byte on the error path: {shown:?}");
        assert!(shown.contains("unknown payload source"), "it is the source refusal: {shown:?}");
        assert!(shown.contains("\\u{1b}"), "the ESC shows as a visible escape: {shown:?}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_excerpt_import_prints_the_sheet_with_answers_only() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let args = parse_args(&strs(&[path, "--answers"])).unwrap();
        let outcome = run(&args, Rules::standard()).unwrap();
        assert!(!outcome.session_built);
        assert!(outcome.text.contains("BGA setup answer sheet — table 895724349"));
        assert!(outcome.text.contains("the wizard would still ask"));
    }

    /// Two imports of one capture must produce byte-identical records — the
    /// property H3's follow-vs-batch pins ride on, and one that was FALSE
    /// before the H3 branch: `resolve_tech_id` picked assumption-resolved tech
    /// identities off `TechRegistry`'s HashMap iteration order, which is
    /// seeded per hasher instance, so two `Rules::standard()` constructions
    /// could (and did, flakily) disagree on a lab reveal's card. Each run here
    /// gets its OWN `Rules` construction so the test discriminates exactly
    /// that.
    #[test]
    fn two_imports_of_one_capture_emit_byte_identical_records() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let dir = std::env::temp_dir().join(format!(
            "bga-import-det-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let mut records = Vec::new();
        for i in 0..2 {
            let out = dir.join(format!("det-{i}.json"));
            let args = parse_args(&strs(&[path, "--out", &out.display().to_string()])).unwrap();
            run(&args, Rules::standard()).unwrap();
            records.push(std::fs::read(&out).unwrap());
        }
        assert_eq!(records[0], records[1], "two imports of one capture disagreed");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_excerpt_replay_builds_a_session_and_reports_its_bound() {
        // The excerpt covers moves 1..17: setup resolves via the key model,
        // the session builds, and the bounded replay runs the early turns.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let args = parse_args(&strs(&[path])).unwrap();
        let outcome = run(&args, Rules::standard()).unwrap();
        assert!(outcome.session_built, "{}", outcome.text);
        assert!(outcome.text.contains("replay attempt"), "{}", outcome.text);
    }
}
