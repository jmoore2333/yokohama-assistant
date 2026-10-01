//! **The layer's log** (M6.5c2, spec §2.6): the lines the layer writes about
//! itself — a game-over save that did not land, a table the listing skipped —
//! through ONE seam on [`Games`](crate::games::Games), so every host hears them.
//!
//! Until M6.5c2 they were `eprintln!`, which reaches the NAS's container log and
//! prints NOTHING on `wasm32-unknown-unknown`: on the Worker a failed stamp in
//! `archive_over_retry` could not report, and in the browser a local game the
//! listing skipped left no trace anywhere. Each host now hands
//! `Games::with_log` its own:
//!
//! - the axum host, [`StderrLog`] (under `server`), beside `main.rs`'s own lines;
//! - the Worker module, a recorder whose lines ride each reply to the shell's
//!   log (`crates/yokohama-worker/src/log.rs`);
//! - the browser host, the console (`crates/yokohama-wasm`);
//! - every other `Games`, [`NoLog`] — the default `Games::with_parts` builds.
//!
//! **A line is an EVENT and its FIELDS**, the shape the Worker's shell logs
//! (`{"event": …, …fields}`), so one line reads the same on every host. The
//! fields are a JSON object and **every value in it is a string**: the Worker's
//! shell parses the lines it is handed, and a string rounds nothing, so the
//! strings rule's reason cannot arise (the shell refuses a line that breaks
//! this). The events are the constants below, so a host that treats one
//! specially — the Worker's recorder, which drops its own deferral — matches a
//! name and never a literal.

use serde_json::Value;

/// Where the layer's own lines go.
///
/// One method over an event and its fields, object-safe, answering nothing —
/// a [`Signal`](crate::signal::Signal)'s shape, for its reason: a write site
/// owes the CALL, and what becomes of the line is the host's.
pub trait Log: Send + Sync {
    /// One line: `event` is one of this module's constants, `fields` a JSON
    /// object of strings.
    fn line(&self, event: &'static str, fields: Value);
}

/// The log of a host that says nothing — the default. Not a test double: it is
/// what every cell's layer runs with.
pub struct NoLog;

impl Log for NoLog {
    fn line(&self, _event: &'static str, _fields: Value) {}
}

/// `{"event": …}` and then `fields`' own keys, on ONE line — `event` first, as
/// the Worker's shell writes its lines. A newline inside a field is escaped, so
/// a line is one line. Anything but an object in `fields` adds nothing.
pub fn render(event: &'static str, fields: &Value) -> String {
    let mut out = format!("{{\"event\":{}", Value::from(event));
    if let Value::Object(map) = fields {
        for (key, value) in map {
            out.push(',');
            out.push_str(&Value::from(key.as_str()).to_string());
            out.push(':');
            out.push_str(&value.to_string());
        }
    }
    out.push('}');
    out
}

/// A game-over put was refused — `{id, error}`, the archive's own reason. The
/// save stays OWED: `archived_at` is unset, which is what says a retry is due.
pub const OVER_PUT_REFUSED: &str = "archive-over-refused";
/// A game-over put landed and its stamp did not — `{id, error}`.
pub const OVER_UNSTAMPED: &str = "archive-over-unstamped";
/// A game-over put landed and its table was gone before the stamp — `{id}`.
pub const OVER_TABLE_GONE: &str = "archive-over-table-gone";
/// A game-over put met an EARLIER COMPLETION of the same game at its key, and
/// the table was stamped (R-6.5c2.11) — `{id, key}`: the archive keeps the
/// first ending.
pub const KEPT_EARLIER_COMPLETION: &str = "archive-kept-earlier-completion";
/// The lobby's listing skipped a table document — `{id, why, error}`, `why`
/// one of `does not parse`, `could not be read`, `is malformed`.
pub const LISTING_SKIPPED: &str = "listing-skipped";
/// The house chat document could not be read while the NAS seeded its house
/// frame's counters at start-up — `{error}`, the store's reason. A failed seed
/// leaves that process's house counters OFFSET from the document for the
/// process's life (`seed_house` runs once, `bump_house_chat` adds one per
/// post), so a lobby misses house messages until N + 1 posts on that process
/// (D's close §5, ruling 49); a restart re-seeds. Every house route refuses
/// the same document out loud.
pub const HOUSE_SEED_UNREAD: &str = "house-seed-unread";
/// The self-host's sweep will end this table (M6.5c2 cycle F, R-F9) —
/// `{id, status, idle_days, reason}`: `status` the table's (`open`,
/// `dealing`, `playing`), `idle_days` the whole days since its last write,
/// `reason` `expired` or `waited`. A pass logs one for EVERY table it will
/// end before it ends any (`crate::sweep::pass`).
pub const SWEEP_LISTED: &str = "sweep-listed";
/// The sweep ended a table through its creator's delete — `{id, reason,
/// kept}`: `kept` is the Worker's word for what was kept — `archive` (the
/// game is in the archive: stamped at game over, or filed by this delete),
/// `nothing` (no game: an open, dealing or moveless table), or `off` (a game
/// and no archive, which a self-host that sweeps never has: it refuses to
/// start without one).
pub const SWEEP_ENDED: &str = "sweep-ended";
/// The sweep's delete was refused — `{id, reason, error}`, the delete's own
/// sentence. A failed archive put refuses it; the table stays, and the next
/// pass tries again.
pub const SWEEP_FAILED: &str = "sweep-failed";
/// The sweep could not read a table's document or its activity, so the table
/// is not a candidate this pass — `{id, error}`; `{error}` alone when the
/// table listing itself failed. Without it a table nobody can read would
/// stand forever without a word.
pub const SWEEP_UNREAD: &str = "sweep-unread";

/// The NAS's log (M6.5c2): each line on stderr, beside `main.rs`'s own — the
/// container log an operator reads.
#[cfg(feature = "server")]
pub struct StderrLog;

#[cfg(feature = "server")]
impl Log for StderrLog {
    fn line(&self, event: &'static str, fields: Value) {
        eprintln!("{}", render(event, &fields));
    }
}
// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;

    /// A line is one JSON object, `event` first and then the fields, on ONE
    /// line — the Worker's shell's own shape — and a quote or a newline inside
    /// a field is escaped rather than breaking the line.
    #[test]
    fn a_line_is_one_object_event_first() {
        let line = render(OVER_PUT_REFUSED, &serde_json::json!({ "id": "t1", "error": "disk on fire" }));
        assert_eq!(line, r#"{"event":"archive-over-refused","error":"disk on fire","id":"t1"}"#);
        let odd = render(LISTING_SKIPPED, &serde_json::json!({ "id": "x", "why": "does not parse", "error": "a \"b\"\nc" }));
        assert!(!odd.contains('\n'), "one line: {odd}");
        let back: Value = serde_json::from_str(&odd).expect("one JSON object");
        assert_eq!((back["event"].as_str(), back["error"].as_str()), (Some(LISTING_SKIPPED), Some("a \"b\"\nc")));
        assert_eq!(render(OVER_TABLE_GONE, &Value::Null), r#"{"event":"archive-over-table-gone"}"#);
    }

    /// Ten events, ten names — and none is a line the Worker's shell writes
    /// ITSELF (`archive-${outcome}` in `worker/src/r2.ts`'s `logOutcome`,
    /// `archive-stamped` and `archive-owed-*` in `worker/src/table.ts`). The
    /// pool reads its archive lines as exact lists, so a layer line under a
    /// shell's name would read as the shell's.
    #[test]
    fn the_events_are_ten_names_and_none_is_the_shells() {
        let events = [
            OVER_PUT_REFUSED,
            OVER_UNSTAMPED,
            OVER_TABLE_GONE,
            KEPT_EARLIER_COMPLETION,
            LISTING_SKIPPED,
            HOUSE_SEED_UNREAD,
            SWEEP_LISTED,
            SWEEP_ENDED,
            SWEEP_FAILED,
            SWEEP_UNREAD,
        ];
        let mut unique = events.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), events.len(), "{events:?}");
        let shells = [
            "archive-landed",
            "archive-identical",
            "archive-conflict",
            "archive-failed",
            "archive-stamped",
            "archive-owed-dropped",
            "archive-owed-kept",
        ];
        for event in events {
            assert!(!shells.contains(&event), "{event} is a line the shell writes itself");
        }
    }

    /// **The layer prints through this seam and nowhere else** (M6.5c2, spec
    /// §2.6). Every `eprintln!` it had is a [`Log`] line now, and a new one
    /// would be a line the Worker and the browser never hear. Read from the
    /// crate's own source above each file's top-level test module. `main.rs`
    /// is the server binary's and prints its start-up; [`StderrLog`] is the
    /// one print the seam itself makes, and the anti-vacuity partner is that
    /// the scan SAW it.
    #[test]
    fn the_layer_prints_only_through_its_log() {
        // The print macros (`println!` also matches `eprintln!`), and either
        // standard stream reached through `io::` — the
        // `writeln!(std::io::stderr(), …)` the macros alone cannot see. A bare
        // `stderr()` imported by name is not matched.
        const PRINTS: [&str; 6] = ["println!", "eprint!(", "print!(", "dbg!(", "io::stderr(", "io::stdout("];
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let (mut prints, mut seam) = (Vec::new(), 0usize);
        for entry in std::fs::read_dir(&src).expect("src is readable") {
            let path = entry.expect("an entry").path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") || name == "main.rs" {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable");
            let lines: Vec<&str> = text.lines().collect();
            let end = lines.iter().position(|l| *l == "#[cfg(test)]").unwrap_or(lines.len());
            for (n, line) in lines[..end].iter().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if PRINTS.iter().any(|p| line.contains(p)) {
                    if name == "log.rs" {
                        seam += 1;
                    } else {
                        prints.push(format!("{name}:{}: {}", n + 1, line.trim()));
                    }
                }
            }
        }
        assert!(prints.is_empty(), "prints outside the log seam: {prints:#?}");
        assert_eq!(seam, 1, "anti-vacuity: the scan saw the one print it allows, `StderrLog`'s");
    }
}
