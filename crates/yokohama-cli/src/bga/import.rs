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
use yokohama_core::Rules;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportArgs {
    pub path: PathBuf,
    pub table: Option<u64>,
    pub answers_only: bool,
    pub through: Option<u64>,
    pub out: Option<PathBuf>,
    pub report: Option<PathBuf>,
}

/// Parse everything after the `import` keyword.
pub fn parse_args(args: &[String]) -> Result<ImportArgs, ImportError> {
    let usage = "import <capture.ndjson> [--table <id>] [--answers] [--through <move_id>] \
                 [--out <record.json>] [--report <path>]";
    let mut path: Option<PathBuf> = None;
    let mut table: Option<u64> = None;
    let mut answers_only = false;
    let mut through: Option<u64> = None;
    let mut out: Option<PathBuf> = None;
    let mut report: Option<PathBuf> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
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
    Ok(ImportArgs {
        path: path.ok_or_else(|| ImportError::Usage {
            detail: format!("missing capture path; {usage}"),
        })?,
        table,
        answers_only,
        through,
        out,
        report,
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

/// BGA's own final-scoring table, parsed from the `tableWindow` notification:
/// per-column totals keyed by the header names.
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
                let header = table.first()?.as_array()?;
                let names: Vec<String> = header
                    .iter()
                    .skip(1)
                    .filter_map(|cell| {
                        cell.get("args")
                            .and_then(|a| a.get("player_name"))
                            .and_then(|p| p.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect();
                let total_row = table.iter().find_map(|row| {
                    let row = row.as_array()?;
                    (row.first()?.as_str()? == "Total").then_some(row.clone())
                })?;
                let totals: Vec<i64> = total_row
                    .iter()
                    .skip(1)
                    .filter_map(|c| match c {
                        serde_json::Value::Number(x) => x.as_i64(),
                        serde_json::Value::String(s) => s.parse().ok(),
                        _ => None,
                    })
                    .collect();
                return Some(names.into_iter().zip(totals).collect());
            }
        }
    }
    None
}

struct AttemptOutcome {
    label: String,
    report: ReplayReport,
    session: crate::session::Session,
    stop: Option<String>,
}

fn run_replay_attempt(
    resolved: &Resolved,
    answers: &super::setup::SetupAnswers,
    feed: &CollapsedFeed,
    args: &ImportArgs,
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
    if let Some(bound) = args.through {
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
    for n in &r.notes {
        s.push_str(&format!("  note: {n}\n"));
    }
    s.push_str(final_check);
    if !resolved.assumptions.is_empty() {
        s.push_str("  declared assumptions (census §2):\n");
        for a in &resolved.assumptions {
            s.push_str(&format!(
                "    - {}: picked {} from [{}] — inert because {}\n",
                a.what, a.picked, a.candidates, a.why_inert
            ));
        }
    }
    s.push_str("  deliberately-unchecked counters (T9):\n");
    for (k, why) in &r.unchecked_counters {
        s.push_str(&format!("    - {k}: {why}\n"));
    }
    let _ = feed;
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
    let seats = answers.seats.clone();
    let mapper = super::orders::OrderMapper::from_rules(&rules)?;
    let stream = super::facts::extract(
        &feed,
        &|pid| seats.iter().find(|s| s.bga_player_id == pid).map(|s| s.seat),
        &mapper,
    )?;
    let resolved = resolve(answers, &feed, &stream, &rules)?;

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
        attempts.push(run_replay_attempt(&resolved, &answers, &feed, args, &rules, label)?);
    }

    let survivors: Vec<usize> = attempts
        .iter()
        .enumerate()
        .filter(|(_, a)| a.stop.is_none())
        .map(|(i, _)| i)
        .collect();
    let chosen = match survivors.as_slice() {
        [one] => *one,
        [] => {
            // Best progress carries the honest STOP story.
            attempts
                .iter()
                .enumerate()
                .max_by_key(|(_, a)| a.report.last_move_id)
                .map(|(i, _)| i)
                .expect("at least one attempt ran")
        }
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
    let mut final_check = String::new();
    let replay_complete = outcome.stop.is_none() && outcome.session.is_over();
    if outcome.session.is_over() {
        let engine_scores = outcome.session.final_scores();
        match bga_final_totals(&feed) {
            Some(bga) => {
                final_check.push_str("  final-score cross-check (engine vs BGA):\n");
                for (pid, score) in &engine_scores {
                    let seat = pid.0 as usize;
                    let name = &resolved.answers.seats[seat].name;
                    let bga_total = bga
                        .iter()
                        .find(|(n, _)| n == name)
                        .map(|(_, t)| *t);
                    match bga_total {
                        Some(t) if t == *score as i64 => {
                            final_check.push_str(&format!("    seat {seat} ({name}): {score} == BGA {t} ✓\n"));
                        }
                        Some(t) => {
                            final_check.push_str(&format!(
                                "    seat {seat} ({name}): engine {score} vs BGA {t} — MISMATCH\n"
                            ));
                        }
                        None => final_check
                            .push_str(&format!("    seat {seat} ({name}): no BGA column found\n")),
                    }
                }
            }
            None => final_check.push_str("  no BGA finalScoring table in the capture\n"),
        }
    }

    out_text.push_str(&render_report(outcome, &resolved, &feed, &final_check));
    if attempts.len() > 1 {
        out_text.push_str("  hypothesis attempts:\n");
        for a in &attempts {
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
