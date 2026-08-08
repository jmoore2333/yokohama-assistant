//! The capture file: one NDJSON line per item the extension delivered.
//!
//! Line shape, fixed by the design's §2.2 and by H1/H2's readers:
//!
//! ```text
//! {"received_at_ms":1754582400123,"kind":"snapshot","payload":{ … }}
//! ```
//!
//! `received_at_ms` is stamped by the RELAY (when this process took delivery),
//! not by the page. The extension stamps its own `captured_at_ms` inside the
//! payload, and the two are deliberately separate: a payload that sat in the
//! extension's outbox while the relay was down has an early capture time and a
//! late receive time, and a reader that conflated them would mis-order the
//! history it is trying to replay.

use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// The kinds of item the relay accepts. An unknown kind is refused rather than
/// written: the capture file's whole value is that a downstream reader can
/// trust every line, and a line nobody can interpret is worse than a 400 the
/// extension logs today.
pub const KINDS: [&str; 2] = ["snapshot", "packets"];

/// Longest table id we will turn into a filename.
const MAX_TABLE_ID: usize = 64;

/// Why a capture was refused.
#[derive(Debug)]
pub enum CaptureError {
    /// The table id would not make a safe filename component.
    UnsafeTableId(String),
    /// The POST body was not JSON.
    NotJson(serde_json::Error),
    /// The body parsed but is not the `{kind, payload}` envelope.
    BadEnvelope(String),
    /// The append itself failed (permissions, full disk).
    Io(std::io::Error),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CaptureError::UnsafeTableId(id) => write!(
                f,
                "table id {id:?} is not a safe filename component (expected 1-{MAX_TABLE_ID} \
                 characters of [A-Za-z0-9._-])"
            ),
            CaptureError::NotJson(e) => write!(f, "body is not valid JSON: {e}"),
            CaptureError::BadEnvelope(why) => write!(f, "bad capture envelope: {why}"),
            CaptureError::Io(e) => write!(f, "could not append to the capture file: {e}"),
        }
    }
}

impl std::error::Error for CaptureError {}

/// Validate a path segment before it becomes a filename.
///
/// Rejecting `/`, `\` and `..` is the point — the table id arrives from a URL
/// path and must never be able to name a file outside the capture directory.
/// The allowlist is the enforcement; the named traversal cases are just what
/// it happens to exclude.
pub fn safe_table_id(raw: &str) -> Result<&str, CaptureError> {
    // The FIRST character must be alphanumeric, which is a separate rule from
    // the allowlist and does separate work. An id of `-rf` or `--dir` reads as
    // an option the moment the filename is pasted into a shell command, and
    // `.ssh` is a dotfile the owner will never see in a listing. It also
    // subsumes `.` and `..` without naming them. Every BGA table id is a
    // number, so nothing legitimate is excluded.
    let starts_well = raw.chars().next().is_some_and(|c| c.is_ascii_alphanumeric());
    let ok = starts_well
        && raw.len() <= MAX_TABLE_ID
        && raw.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if ok {
        Ok(raw)
    } else {
        Err(CaptureError::UnsafeTableId(raw.to_string()))
    }
}

/// Milliseconds since the Unix epoch. A clock set before 1970 would be a
/// broken machine, and saturating at 0 keeps this total rather than panicking
/// inside a request handler.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Turn a POST body into the exact line to append.
///
/// The line is built with `format!` rather than by serializing a map, for one
/// reason: `serde_json`'s default map is a `BTreeMap`, so a serialized
/// envelope would come out alphabetically (`kind`, `payload`, `received_at_ms`)
/// and the line a human opens would not read in the order the design writes
/// it. The payload itself is still re-serialized and therefore still
/// alphabetized internally — see the crate doc on why that trade stands.
pub fn line_for(body: &[u8], received_at_ms: u64) -> Result<String, CaptureError> {
    let parsed: serde_json::Value = serde_json::from_slice(body).map_err(CaptureError::NotJson)?;

    let object = parsed
        .as_object()
        .ok_or_else(|| CaptureError::BadEnvelope("top level is not a JSON object".into()))?;

    let kind = object
        .get("kind")
        .and_then(|k| k.as_str())
        .ok_or_else(|| CaptureError::BadEnvelope("missing string field `kind`".into()))?;
    if !KINDS.contains(&kind) {
        return Err(CaptureError::BadEnvelope(format!(
            "unknown kind {kind:?} (expected one of {KINDS:?})"
        )));
    }

    let payload = object
        .get("payload")
        .ok_or_else(|| CaptureError::BadEnvelope("missing field `payload`".into()))?;

    Ok(format!(
        "{{\"received_at_ms\":{},\"kind\":{},\"payload\":{}}}",
        received_at_ms,
        serde_json::Value::String(kind.to_string()),
        payload
    ))
}

/// Append one line to `<dir>/<table_id>.ndjson`, creating both as needed.
///
/// Returns the file's size after the append, which is what the console log
/// prints — the owner watching the relay wants to see the number grow.
///
/// Callers serialize their appends (see `server::Relay`): `O_APPEND` gives an
/// atomic offset update but not an atomic multi-kilobyte write, and two
/// interleaved 30 KB snapshots would produce two unparseable lines.
pub fn append(dir: &Path, table_id: &str, line: &str) -> Result<u64, CaptureError> {
    let table_id = safe_table_id(table_id)?;
    std::fs::create_dir_all(dir).map_err(CaptureError::Io)?;
    let path = capture_path(dir, table_id);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(CaptureError::Io)?;
    // One `write_all` for line-plus-newline, never two: a newline written
    // separately is a newline that can land after somebody else's bytes.
    let mut bytes = line.as_bytes().to_vec();
    bytes.push(b'\n');
    file.write_all(&bytes).map_err(CaptureError::Io)?;
    file.flush().map_err(CaptureError::Io)?;
    file.metadata().map(|m| m.len()).map_err(CaptureError::Io)
}

/// Where a table's captures live. Public so the setup doc's path and the
/// tests' path come from the same function.
pub fn capture_path(dir: &Path, table_id: &str) -> PathBuf {
    dir.join(format!("{table_id}.ndjson"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_reads_in_the_designed_key_order() {
        let line = line_for(br#"{"kind":"snapshot","payload":{"z":1,"a":2}}"#, 1_754_582_400_123)
            .expect("valid envelope");
        assert!(
            line.starts_with(r#"{"received_at_ms":1754582400123,"kind":"snapshot","payload":"#),
            "got: {line}"
        );
        assert!(!line.contains('\n'), "a capture line must never contain a raw newline");
        let back: serde_json::Value = serde_json::from_str(&line).expect("round-trips");
        assert_eq!(back["payload"]["a"], 2);
    }

    #[test]
    fn a_payload_string_survives_verbatim() {
        // This is the fidelity claim H2 leans on: a notification response body
        // shipped as a raw string comes back byte-for-byte.
        let raw = r#"{"kind":"packets","payload":{"body_text":"{\"logs\":[{\"type\":\"cancel\"}]}"}}"#;
        let line = line_for(raw.as_bytes(), 1).expect("valid");
        let back: serde_json::Value = serde_json::from_str(&line).expect("round-trips");
        assert_eq!(back["payload"]["body_text"], r#"{"logs":[{"type":"cancel"}]}"#);
    }

    #[test]
    fn both_designed_kinds_are_accepted_and_nothing_else_is() {
        for kind in KINDS {
            let body = format!(r#"{{"kind":"{kind}","payload":null}}"#);
            assert!(line_for(body.as_bytes(), 1).is_ok(), "{kind} should be accepted");
        }
        let err = line_for(br#"{"kind":"moves","payload":null}"#, 1).expect_err("unknown kind");
        assert!(matches!(err, CaptureError::BadEnvelope(_)), "{err}");
    }

    #[test]
    fn an_envelope_missing_a_required_field_is_refused() {
        assert!(matches!(
            line_for(br#"{"payload":{}}"#, 1),
            Err(CaptureError::BadEnvelope(_))
        ));
        assert!(matches!(
            line_for(br#"{"kind":"snapshot"}"#, 1),
            Err(CaptureError::BadEnvelope(_))
        ));
        assert!(matches!(line_for(b"[1,2,3]", 1), Err(CaptureError::BadEnvelope(_))));
        assert!(matches!(line_for(b"not json", 1), Err(CaptureError::NotJson(_))));
    }

    #[test]
    fn table_ids_that_could_escape_the_capture_directory_are_refused() {
        for hostile in ["..", ".", "", "../secrets", "a/b", "a\\b", "a b", "tab\tid"] {
            assert!(safe_table_id(hostile).is_err(), "{hostile:?} must be refused");
        }
        // A leading `-` or `.` passes a character allowlist but is still a
        // hazard: the first becomes an option when the filename is pasted into
        // a shell command, and the second is a dotfile the owner will not see
        // in a directory listing. Requiring an alphanumeric first character
        // costs nothing - every BGA table id is a number.
        for hostile in ["-rf", "-", ".ssh", ".bashrc", "_private", "--dir"] {
            assert!(safe_table_id(hostile).is_err(), "{hostile:?} must be refused");
        }
        for fine in ["895608640", "table-1", "table_1", "unknown", "a.b"] {
            assert_eq!(safe_table_id(fine).expect("accepted"), fine);
        }
        let too_long = "9".repeat(MAX_TABLE_ID + 1);
        assert!(safe_table_id(&too_long).is_err());
    }

    #[test]
    fn appending_twice_leaves_two_lines() {
        let dir = std::env::temp_dir().join(format!("yokohama-bridge-capture-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        append(&dir, "895608640", r#"{"received_at_ms":1,"kind":"snapshot","payload":1}"#)
            .expect("first append");
        let size = append(&dir, "895608640", r#"{"received_at_ms":2,"kind":"packets","payload":2}"#)
            .expect("second append");

        let text = std::fs::read_to_string(capture_path(&dir, "895608640")).expect("read back");
        assert_eq!(text.lines().count(), 2);
        assert_eq!(text.len() as u64, size, "the reported size is the file's size");
        assert!(text.ends_with('\n'), "every line is terminated, so an append never merges lines");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
