//! The `yokohama-cli import` entry point (plan T6).
//!
//! Command shape (plan §1): `yokohama-cli import <capture.ndjson> [--table
//! <id>] [--answers] [--out <record.json>]`. Dispatch in `main.rs` is on
//! `args[1] == "import"` as an EXACT keyword; every other `args[1]` keeps its
//! "resume this save" meaning, so the argv contract the guide documents is
//! unchanged. `--through` and `--report` are the catch-up translator's flags
//! and refuse with a typed `NotImplemented` until H2 lands — accepting a flag
//! that silently does nothing would be a lie.
//!
//! Outcomes, honestly separated:
//! - `--answers`: print the evidence sheet and stop. This is H1's standalone
//!   value — the sheet answers the wizard's questions where evidence exists
//!   and names the reason wherever it does not.
//! - Without `--answers`: additionally build the live-mode `Session` WHERE THE
//!   EVIDENCE SUFFICES, and save its record to `--out` when given. When the
//!   evidence does not suffice the sheet still prints, followed by the
//!   `Incomplete` refusal naming exactly what the wizard would still ask —
//!   a real result, not a failure (plan §7 risk 3: the wizard asking a human
//!   the un-evidenced questions is the designed fallback).

use super::setup::{build_session, render_sheet, synthesize};
use super::ImportError;
use std::path::PathBuf;
use yokohama_bridge::feed::read_and_collapse;
use yokohama_core::Rules;

/// Provenance stamp for bridge-imported records (plan §1: the map §1.5/§1.6
/// comparison-only fence rides the run-id prefix — these records are
/// comparison corpus, never a tuning input).
pub const BGA_IMPORT_VERSION: &str = "bga-import-v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportArgs {
    pub path: PathBuf,
    pub table: Option<u64>,
    pub answers_only: bool,
    pub out: Option<PathBuf>,
}

/// Parse everything after the `import` keyword.
pub fn parse_args(args: &[String]) -> Result<ImportArgs, ImportError> {
    let usage = "import <capture.ndjson> [--table <id>] [--answers] [--out <record.json>]";
    let mut path: Option<PathBuf> = None;
    let mut table: Option<u64> = None;
    let mut answers_only = false;
    let mut out: Option<PathBuf> = None;
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
            "--out" => {
                let v = it.next().ok_or_else(|| ImportError::Usage {
                    detail: format!("--out needs a path; {usage}"),
                })?;
                out = Some(PathBuf::from(v));
            }
            "--through" | "--report" => {
                return Err(ImportError::NotImplemented { flag: arg.clone() });
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
        out,
    })
}

/// What a run produced. `session_built == false` with `Ok` means the sheet
/// printed and the `Incomplete` refusal is part of `text` — the caller picks
/// the exit code, the refusal is not an error.
#[derive(Debug)]
pub struct RunOutcome {
    pub text: String,
    pub session_built: bool,
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
        return Ok(RunOutcome { text: out_text, session_built: false });
    }
    match build_session(&answers, rules) {
        Ok(session) => {
            if let Some(out_path) = &args.out {
                session.save(out_path).map_err(|e| ImportError::Session { detail: e })?;
                out_text.push_str(&format!(
                    "  live-mode session built; record saved to {}\n",
                    out_path.display()
                ));
            } else {
                out_text.push_str("  live-mode session built (pass --out <record.json> to save it)\n");
            }
            Ok(RunOutcome { text: out_text, session_built: true })
        }
        Err(ImportError::Incomplete { missing }) => {
            out_text.push_str(&format!(
                "  no Session built — the evidence does not yet cover: {}\n  (run the wizard for \
                 the missing answers, or import with --answers to use this sheet beside it)\n",
                missing.join("; ")
            ));
            Ok(RunOutcome { text: out_text, session_built: false })
        }
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn args_parse_the_documented_shape() {
        let a = parse_args(&strs(&["cap.ndjson", "--table", "895724349", "--answers", "--out", "rec.json"])).unwrap();
        assert_eq!(a.path, PathBuf::from("cap.ndjson"));
        assert_eq!(a.table, Some(895_724_349));
        assert!(a.answers_only);
        assert_eq!(a.out, Some(PathBuf::from("rec.json")));
    }

    #[test]
    fn h2_flags_refuse_rather_than_silently_no_op() {
        assert_eq!(
            parse_args(&strs(&["cap.ndjson", "--through", "50"])).unwrap_err(),
            ImportError::NotImplemented { flag: "--through".to_string() }
        );
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
    fn the_excerpt_import_prints_the_sheet_and_the_honest_refusal() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../yokohama-bridge/tests/data/excerpt-895724349.ndjson"
        );
        let args = parse_args(&strs(&[path])).unwrap();
        let outcome = run(&args, Rules::standard()).unwrap();
        assert!(!outcome.session_built);
        assert!(outcome.text.contains("BGA setup answer sheet — table 895724349"));
        assert!(outcome.text.contains("no Session built"), "{}", outcome.text);
        // --answers stops before session construction and says so via the sheet.
        let args2 = parse_args(&strs(&[path, "--answers"])).unwrap();
        let outcome2 = run(&args2, Rules::standard()).unwrap();
        assert!(!outcome2.session_built);
        assert!(outcome2.text.contains("the wizard would still ask"));
    }
}
