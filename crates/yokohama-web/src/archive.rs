//! The archive (M6.5a, R-6.5a.9): a finished or abandoned game's record and
//! table document, written ONCE, in the file store's own layout under a
//! `SEMANTICS_REV` prefix, so an `rclone sync` of the bucket M6.5c adds is a
//! directory the CLI loads. One layout function for every implementation.
//!
//! Two implementations this cycle: [`NoArchive`] (unconditional, `kind() ==
//! "off"`) and [`DirArchive`] under the `server` feature (a directory beside the
//! store, `YOKOHAMA_WEB_ARCHIVE_DIR`). The bucket in M6.5c is the third, and it
//! writes the SAME relative paths [`paths`] computes, which is the property a
//! cell over both implementations holds.

use std::sync::Arc;

use crate::table::TableDoc;

/// Why a game is being archived — which of the two prefixes it lands under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// Game over, at the `persist` that wrote the last move (R-6.5a.11).
    Over,
    /// The creator's Abandon on a playing table with events (R-6.5.5).
    Abandoned,
}

impl Why {
    fn dir(self) -> &'static str {
        match self {
            Self::Over => "over",
            Self::Abandoned => "abandoned",
        }
    }
}

/// The archive seam. `put` is handed the record bytes the store holds and the
/// STRIPPED table document ([`strip_for_archive`]) — a game id is archived at
/// most once, and an implementation refuses a second put rather than
/// versioning (umbrella R-6.5.4).
pub trait Archive: Send + Sync {
    fn put(&self, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<(), String>;
    /// What `/api/health` reports: `"off"`, `"dir"`, or M6.5c's `"r2"`.
    fn kind(&self) -> &'static str;
}

/// The ONE layout: `rev<N>/<over|abandoned>/<id>.json` and
/// `rev<N>/<over|abandoned>/<id>.table.json` — the file store's own two names
/// under a semantics-rev prefix, so a synced archive is a directory the CLI and
/// this server both already read.
pub fn paths(rev: u32, why: Why, id: &str) -> (String, String) {
    let base = format!("rev{rev}/{}/{id}", why.dir());
    (format!("{base}.json"), format!("{base}.table.json"))
}

/// The stored document with the lock and the watcher roster REMOVED (R-6.5a.9):
/// a salt and a hash are a password to attack offline, and the watcher list is
/// nobody else's business. `archived_at` stays in, so an archived table says
/// when.
pub fn strip_for_archive(doc: &TableDoc) -> String {
    let mut copy = doc.clone();
    copy.lock = None;
    copy.unlocked = Vec::new();
    serde_json::to_string_pretty(&copy).expect("a table document serializes")
}

/// The archive of a host that keeps none.
pub struct NoArchive;

impl Archive for NoArchive {
    fn put(&self, _why: Why, _id: &str, _record: &str, _table: &str) -> Result<(), String> {
        Err("no archive is configured".to_string())
    }
    fn kind(&self) -> &'static str {
        "off"
    }
}

/// `Games` asks this before calling `put`, so `NoArchive::put` is never reached
/// on the happy path and its `Err` is a guard, not a failure.
pub fn is_on(a: &Arc<dyn Archive>) -> bool {
    a.kind() != "off"
}

#[cfg(feature = "server")]
pub use dir::DirArchive;

#[cfg(feature = "server")]
mod dir {
    use std::path::{Path, PathBuf};

    use super::{paths, Archive, Why};

    pub const ENV_VAR: &str = "YOKOHAMA_WEB_ARCHIVE_DIR";

    /// A directory in the archive layout — on the NAS, `/data/archive` on the
    /// store's own volume, so saves are collected during development.
    pub struct DirArchive {
        dir: PathBuf,
    }

    impl DirArchive {
        pub fn new(dir: impl Into<PathBuf>) -> Self {
            Self { dir: dir.into() }
        }

        /// `None` when the variable is unset (the archive is off); `Err` when it
        /// is set to a path that cannot be created — a bad value exits 1 naming
        /// the variable, like the chat variables.
        pub fn from_env() -> Result<Option<Self>, String> {
            let Some(raw) = std::env::var_os(ENV_VAR) else { return Ok(None) };
            let dir = PathBuf::from(raw);
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("{ENV_VAR}={} cannot be created: {e}", dir.display()))?;
            Ok(Some(Self { dir }))
        }

        /// Temp + rename, and NEVER over an existing file: a second put of the
        /// same game is a bug the caller made, and the archive says so instead
        /// of versioning (umbrella R-6.5.4 — "at most once").
        fn write_new(path: &Path, text: &str) -> Result<(), String> {
            if path.exists() {
                return Err(format!("{} already exists — a game is archived once", path.display()));
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, text).map_err(|e| format!("write {}: {e}", tmp.display()))?;
            std::fs::rename(&tmp, path).map_err(|e| format!("rename {}: {e}", path.display()))
        }
    }

    impl Archive for DirArchive {
        fn put(&self, why: Why, id: &str, record_json: &str, table_json: &str) -> Result<(), String> {
            let (record, table) = paths(yokohama_core::SEMANTICS_REV, why, id);
            Self::write_new(&self.dir.join(&record), record_json)?;
            Self::write_new(&self.dir.join(&table), table_json)
        }
        fn kind(&self) -> &'static str {
            "dir"
        }
    }
}
// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::{Lock, SeatDoc, TableStatus};
    use yokohama_core::PlayerColor;

    /// A two-person playing table with a lock and one watcher — the two things
    /// the archived copy must NOT carry.
    fn locked_doc() -> TableDoc {
        TableDoc {
            schema: 3,
            id: "sim-2".into(),
            name: None,
            created_by: "a@x.test".into(),
            created_at: 1_789_000_000,
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seats: vec![
                SeatDoc::Human { user: "a@x.test".into(), name: None },
                SeatDoc::Human { user: "b@x.test".into(), name: None },
            ],
            keeps: vec![Some(1), Some(2)],
            seed: 7,
            status: TableStatus::Playing,
            turn: None,
            lock: Some(Lock::new("hunter2")),
            unlocked: vec!["watcher@x.test".to_string()],
            archived_at: Some(1_789_000_001),
        }
    }

    #[test]
    fn the_layout_is_one_function_for_both_reasons() {
        assert_eq!(
            paths(7, Why::Over, "sim-1"),
            ("rev7/over/sim-1.json".into(), "rev7/over/sim-1.table.json".into())
        );
        assert_eq!(
            paths(7, Why::Abandoned, "sim-1"),
            ("rev7/abandoned/sim-1.json".into(), "rev7/abandoned/sim-1.table.json".into())
        );
    }

    #[test]
    fn a_stripped_table_carries_no_lock_and_no_watcher() {
        let doc = locked_doc();
        let (salt, hash) = {
            let l = doc.lock.as_ref().unwrap();
            (l.salt.clone(), l.hash.clone())
        };
        let json = strip_for_archive(&doc);
        assert!(!json.contains(&salt) && !json.contains(&hash), "the salt and hash left with the lock");
        assert!(!json.contains("watcher@x.test"), "the watcher's address left with the roster");
        assert!(json.contains("a@x.test") && json.contains("b@x.test"), "anti-vacuity: the seats are still there");
        assert!(json.contains("\"archived_at\": 1789000001"), "an archived table says when: {json}");
        // And the stripped copy still READS as a table document.
        let back: TableDoc = serde_json::from_str(&json).unwrap();
        assert_eq!(back.lock, None);
        assert!(back.unlocked.is_empty());
    }

    #[test]
    fn no_archive_is_off_and_never_takes_a_put() {
        assert_eq!(NoArchive.kind(), "off");
        let a: Arc<dyn Archive> = Arc::new(NoArchive);
        assert!(!is_on(&a));
        assert!(NoArchive.put(Why::Over, "x", "{}", "{}").is_err(), "the guard, not a write");
    }

    #[cfg(feature = "server")]
    #[test]
    fn the_directory_archive_writes_both_files_and_refuses_a_second_put() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir);
        assert_eq!(a.kind(), "dir");
        a.put(Why::Abandoned, "sim-3", "{\"record\":1}", "{\"table\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.json")).unwrap(), "{\"record\":1}");
        assert_eq!(
            std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.table.json")).unwrap(),
            "{\"table\":1}"
        );
        let second = a.put(Why::Abandoned, "sim-3", "{\"record\":2}", "{\"table\":2}");
        assert!(second.is_err(), "a second put of the same game is refused, never versioned");
        assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.json")).unwrap(), "{\"record\":1}");
        // The same id under the OTHER reason is a different path — `paths` says
        // so — and lands; the "once" rule is per (id, why), and the callers
        // (R-6.5a.11) are what keep a game to one reason.
        a.put(Why::Over, "sim-3", "{\"record\":3}", "{\"table\":3}").unwrap();
        // Both leaves' paths are asserted, not one: the mutation battery found a
        // `put` that hand-built its `over/` path survived the `abandoned/` pins.
        assert_eq!(std::fs::read_to_string(dir.join("rev7/over/sim-3.json")).unwrap(), "{\"record\":3}");
        assert_eq!(std::fs::read_to_string(dir.join("rev7/over/sim-3.table.json")).unwrap(), "{\"table\":3}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
