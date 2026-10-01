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
//!
//! **Since M6.5b every path starts with a CALLER PREFIX** (R-6.5b.15): `""` for
//! the games this server plays itself, and `browser/<`[`caller_segment`]`>/` for
//! a game a browser sends to [`crate::api::archive_post`]. One tree, with whose
//! save it is written into the path, because the owner reviews saves by who
//! sent them.

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
    /// The leaf this reason files under — and the word `leaf` on the wire.
    pub(crate) fn dir(self) -> &'static str {
        match self {
            Self::Over => "over",
            Self::Abandoned => "abandoned",
        }
    }
}

/// What a put did (spec §2.4): wrote both files, or found the SAME record
/// already at the path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutOutcome {
    /// Both files were written.
    Landed,
    /// The record at this path holds these very bytes — a retry after a reply
    /// the caller never saw. A HALF PAIR (the table alone, a crash between the
    /// two writes) is repaired on the way to [`Self::Landed`] instead.
    Identical,
}

/// Why a put was refused: a DIFFERENT record is already at this path, or the
/// write itself failed.
///
/// Two variants rather than one string, because the caller answers them
/// differently and must: a conflict is a 409 the client KEEPS its game on, and
/// a failure is "the archive could not be reached — the game was kept"
/// (R-6.5b.7). A 409 that read as a failure, or a failure that read as a 409,
/// is the shape that deletes somebody's only complete copy.
#[derive(Debug)]
pub enum PutError {
    /// The path holds a different record. `key` is the path, for the operator;
    /// `held` is the record already there where the archive read it —
    /// [`DirArchive`] always does, and `None` where it cannot say (the Worker's
    /// receipt, until its shell hands the held text back) — so the game-over
    /// site can tell an earlier completion of the SAME game from another game
    /// (M6.5c2, R-6.5c2.11). The ROUTE words its own sentence and never this
    /// one, and `held` is in neither: it is a whole record, the setup seed in it.
    Conflict { key: String, held: Option<String> },
    /// The write did not happen. Carries the implementation's own reason.
    Failed(String),
}

impl std::fmt::Display for PutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { key, .. } => write!(f, "{key} holds a different record — a game is archived once"),
            Self::Failed(e) => f.write_str(e),
        }
    }
}

/// The archive seam. `put` is handed the record bytes the store holds and the
/// STRIPPED table document ([`strip_for_archive`]).
///
/// **"At most once" is per PATH, and the path is `prefix/rev/leaf/id`**:
/// [`paths`] puts `over/` and `abandoned/` under different leaves and each
/// caller under its own prefix, an implementation answers
/// [`PutOutcome::Identical`] for the same bytes again and refuses DIFFERENT
/// bytes rather than versioning, and the two server sites of R-6.5a.11 — the
/// `archived_at` stamp they read and write — are what keep one of this
/// server's games to one reason (umbrella R-6.5.4). A stamp lost after a landed
/// put is therefore a DUPLICATE under the other leaf at the next abandon, not a
/// refusal; the M6.5a close records that edge. What it does NOT prevent: the
/// same save sent by two different callers lands twice, under two prefixes
/// (spec §2.4 — the training side dedups by digest).
pub trait Archive: Send + Sync {
    /// `prefix` is `""` for this server's own games and
    /// `browser/<`[`caller_segment`]`>/` (R-6.5b.15) for a game a browser sent;
    /// it is the leading path component and every implementation joins it
    /// through [`paths`] rather than building a name itself.
    ///
    /// **The TABLE is written first and the RECORD's path is the once-marker**
    /// (spec §2.4): the record is the copy that matters, so a crash between the
    /// two writes leaves a table with no record — which the next put REPAIRS —
    /// and never a record whose table never landed.
    fn put(
        &self,
        prefix: &str,
        why: Why,
        id: &str,
        record_json: &str,
        table_json: &str,
    ) -> Result<PutOutcome, PutError>;
    /// What `/api/health` reports: `"off"`, `"dir"`, or M6.5c's `"r2"`.
    fn kind(&self) -> &'static str;
}

/// The ONE layout: `<prefix>rev<N>/<over|abandoned>/<id>.json` and
/// `<prefix>rev<N>/<over|abandoned>/<id>.table.json` — the file store's own two
/// names under a semantics-rev prefix, so a synced archive is a directory the
/// CLI and this server both already read.
///
/// **It refuses a path with a `.` or `..` component** (M6.5c2, spec §2.1's
/// defence in depth; review R2-m3). Every caller builds its arguments from a
/// segment [`caller_segment`] rendered and an id `is_safe_id` admitted, and
/// neither can spell one — so this is the second fence and not the first, and
/// an `Err` is a caller's bug named rather than a path written. The refusal
/// does not echo the path: it can reach a client's 500.
pub fn paths(rev: u32, prefix: &str, why: Why, id: &str) -> Result<(String, String), String> {
    let base = format!("{prefix}rev{rev}/{}/{id}", why.dir());
    if let Some(step) = base.split('/').find(|part| matches!(*part, "." | "..")) {
        return Err(format!("an archive path cannot hold a {step:?} component"));
    }
    Ok((format!("{base}.json"), format!("{base}.table.json")))
}

/// R-6.5b.15: a path-safe, READABLE rendering of a caller's key — `@` becomes
/// `_at_`, anything outside `[a-z0-9._-]` becomes `_` (a name's space too),
/// capped at 64 characters.
///
/// **Why the segment is never empty, `.` or `..`** — restated for M6.5c2's key
/// grammar (spec §2.1; review H1, which MEASURED the hole an unconstrained name
/// opened: `..` rendered as the segment `..`, and a browser's save landed in
/// this server's own `rev7/over/`). [`crate::identity::User::parse`] admits two
/// shapes, and each closes it by its own arithmetic:
/// - an EMAIL has a non-empty local part and domain either side of one `@`,
///   which renders as four characters, so its segment is at least six
///   characters of this alphabet;
/// - a PLAIN NAME begins with a letter or a digit — the grammar's rule, which
///   exists for exactly this — and that character is kept, so its segment
///   begins with `[a-z0-9]`.
///
/// Neither holds a `/`, and the cap only shortens a segment that is already
/// non-empty. [`paths`] and `DirArchive::put` refuse a `.` or `..` component
/// besides, so this argument is no longer the only fence between a key and a
/// path outside its prefix.
///
/// **It COLLIDES, on purpose, and spec §8 records it**: `a/b@c` and `a_b@c`
/// render alike, and two addresses longer than the cap can share a prefix — and
/// with it each other's "already archived" refusal. Readable was the trade: the
/// owner reviews saves by who sent them and a hash would hide that.
pub fn caller_segment(user: &crate::identity::User) -> String {
    let mut out = String::new();
    for c in user.as_str().chars() {
        match c {
            '@' => out.push_str("_at_"),
            c if c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-') => {
                out.push(c)
            }
            // Every replacement is ASCII, so `truncate` below is always on a
            // character boundary however odd the address was.
            _ => out.push('_'),
        }
    }
    out.truncate(64);
    out
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
    fn put(
        &self,
        _prefix: &str,
        _why: Why,
        _id: &str,
        _record: &str,
        _table: &str,
    ) -> Result<PutOutcome, PutError> {
        Err(PutError::Failed("no archive is configured".to_string()))
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
    use std::path::{Component, Path, PathBuf};

    use super::{paths, Archive, PutError, PutOutcome, Why};

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

        /// Temp + rename, over whatever is there.
        ///
        /// **The existence check that used to live here moved UP into `put`**
        /// (M6.5b, spec §2.4), and it had to: the once-rule is about the RECORD
        /// — the same bytes again are a retry and not a bug — and the TABLE is
        /// written first precisely so a half pair can be repaired, which a
        /// refuse-if-present write could not do.
        fn write_over(path: &Path, text: &str) -> Result<(), PutError> {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| PutError::Failed(format!("mkdir {}: {e}", parent.display())))?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, text)
                .map_err(|e| PutError::Failed(format!("write {}: {e}", tmp.display())))?;
            std::fs::rename(&tmp, path)
                .map_err(|e| PutError::Failed(format!("rename {}: {e}", path.display())))
        }
    }

    impl Archive for DirArchive {
        fn put(
            &self,
            prefix: &str,
            why: Why,
            id: &str,
            record_json: &str,
            table_json: &str,
        ) -> Result<PutOutcome, PutError> {
            let (record, table) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id).map_err(PutError::Failed)?;
            // **Contained, LEXICALLY** (M6.5c2, spec §2.1; review R2-m3): every
            // component of both RELATIVE paths is a plain name, so joining them
            // under `self.dir` lands under `self.dir` — no `..`, and no root,
            // which `Path::join` would let REPLACE `self.dir` outright. Lexical
            // because it runs before the write and `canonicalize` needs the file
            // to exist; over the relative paths, not the joined one, because the
            // configured directory is the operator's and may itself be relative
            // (`YOKOHAMA_WEB_ARCHIVE_DIR=../saves`).
            for rel in [&record, &table] {
                if !Path::new(rel).components().all(|c| matches!(c, Component::Normal(_))) {
                    return Err(PutError::Failed("an archive path must stay under its root".to_string()));
                }
            }
            let record_path = self.dir.join(&record);
            // **The record is the marker, and it has THREE answers, not two**:
            // the same bytes are a retry, different bytes a conflict, and
            // ABSENT is the only road to a write.
            //
            // A read that failed for any OTHER reason — a path that is a
            // directory, a file this process may not read, a truncated write
            // that left invalid UTF-8 — must NOT fall through. `rename` needs
            // write permission on the DIRECTORY and not on the file, so a
            // fall-through would REPLACE an archived record with different
            // bytes and answer `Landed`: the one outcome this protocol exists
            // to prevent (spec §2.4 — "a 409 must never delete the only
            // complete copy"), reached without even a 409.
            match std::fs::read_to_string(&record_path) {
                Ok(existing) if existing == record_json => return Ok(PutOutcome::Identical),
                Ok(existing) => {
                    return Err(PutError::Conflict { key: record_path.display().to_string(), held: Some(existing) })
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(PutError::Failed(format!(
                        "read {}: {e}",
                        record_path.display()
                    )))
                }
            }
            // The table FIRST — overwriting a half pair's table is the repair —
            // and the record second, which is what makes the pair complete.
            Self::write_over(&self.dir.join(&table), table_json)?;
            Self::write_over(&record_path, record_json)?;
            Ok(PutOutcome::Landed)
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

    /// R-6.5b.15, restated for M6.5c2's key grammar (spec §2.1; review H1): a
    /// caller's key rendered into ONE path segment that is path-safe and still
    /// readable — the owner reviews saves by who sent them, and a hash would
    /// hide that.
    ///
    /// **The containment property is arithmetic, with one arm per shape of
    /// key.** An email's local part and domain are each non-empty and `@`
    /// becomes four characters, so its segment is at least six characters of
    /// `[a-z0-9._-]`. A plain name begins with a letter or a digit, which the
    /// segment keeps, so its segment begins with `[a-z0-9]`. Neither holds a
    /// `/`, and the cap can only shorten a segment that is already non-empty —
    /// so no segment is empty, `.` or `..`. The names that WOULD break the
    /// second arm (`.`, `..`, `. .`, `-`, `.x`) are fed to the grammar and
    /// must be refused there.
    #[test]
    fn a_caller_segment_is_path_safe_and_readable() {
        use crate::identity::User;
        let seg = |s: &str| caller_segment(&User::parse(s).unwrap());
        assert_eq!(seg("red@example.test"), "red_at_example.test");
        assert_eq!(seg("A/B@c"), "a_b_at_c", "a slash is replaced, the address lowercased");
        assert_eq!(seg("../../x@y"), ".._.._x_at_y");
        assert_eq!(seg("..@.."), ".._at_..");
        assert_eq!(seg("  Ben   Castle "), "ben_castle", "a name's one space becomes `_`");
        let long = format!("{}@{}", "a".repeat(200), "b".repeat(50));
        assert_eq!(seg(&long).len(), 64, "capped");
        for name in [".", "..", ". .", "-", ".x"] {
            assert!(User::parse(name).is_err(), "{name:?} must never be a key");
        }
        let n32 = "n".repeat(32);
        for name in ["ben castle", "7", "x", "a.test", "z..", "r2-d2_x.y", n32.as_str()] {
            let v = seg(name);
            assert!(v.starts_with(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit()), "{name:?} → {v:?}");
        }
        for s in ["../../x@y", "..@..", long.as_str(), "a/b@c", "ben castle", "z..", "7"] {
            let v = seg(s);
            assert!(!v.is_empty() && v != "." && v != ".." && !v.contains('/'), "{v:?}");
            assert!(
                v.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-')),
                "{v:?}"
            );
        }
    }

    /// **The layout refuses a `.` or `..` component** (M6.5c2, spec §2.1's
    /// defence in depth; review R2-m3) — anywhere, the prefix and the id alike
    /// — and nothing else: a dot INSIDE a component is a name's business.
    #[test]
    fn the_layout_refuses_a_dot_or_dot_dot_component() {
        for (prefix, id) in [("browser/../", "sim-1"), ("browser/./", "sim-1"), ("", ".."), ("", "."), ("../", "x")] {
            let refused = paths(7, prefix, Why::Over, id);
            assert!(refused.is_err(), "{prefix:?} + {id:?}: {refused:?}");
        }
        assert_eq!(
            paths(7, "browser/z../", Why::Over, "a..b"),
            Ok(("browser/z../rev7/over/a..b.json".into(), "browser/z../rev7/over/a..b.table.json".into())),
            "dots inside a component are not a step",
        );
    }

    /// **The review's measured probe, as a regression cell** (M6.5c2, review
    /// H1). In an export of `6ce33fd6` a widened grammar with no leading-
    /// character rule let the name `..` become the segment `..`, and a put
    /// under `browser/../` LANDED at the server's own `rev7/over/<id>.json` —
    /// after which the server's own game-over put answered `Conflict`. Three
    /// fences now stand between a name and that path, each asserted where it
    /// stands: the grammar refuses the name, [`paths`] refuses the component,
    /// and the directory archive refuses the put and writes nothing.
    #[cfg(feature = "server")]
    #[test]
    fn a_dot_dot_segment_never_lands_a_save_in_the_servers_own_tree() {
        assert!(crate::identity::User::parse("..").is_err(), "fence 1: `..` is never a key");
        let hostile = "browser/../";
        assert!(paths(yokohama_core::SEMANTICS_REV, hostile, Why::Over, "sim-victim").is_err(), "fence 2");
        let dir = std::env::temp_dir().join(format!("yokohama_archive_probe_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = DirArchive::new(&dir);
        let put = a.put(hostile, Why::Over, "sim-victim", "{\"planted\":1}", "{\"t\":1}");
        assert!(matches!(&put, Err(PutError::Failed(_))), "fence 3 — a FAILURE, never a landing: {put:?}");
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none(), "nothing was written at all");
        // The server's own game-over put at that id lands, as it did not in the
        // probe: the archive keeps the GAME'S bytes and not the planted ones.
        assert!(matches!(a.put("", Why::Over, "sim-victim", "{\"game\":1}", "{\"t\":1}"), Ok(PutOutcome::Landed)));
        let (own, _) = paths(yokohama_core::SEMANTICS_REV, "", Why::Over, "sim-victim").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join(&own)).unwrap(), "{\"game\":1}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **The directory archive's own check is LEXICAL, over the relative path**
    /// (M6.5c2, spec §2.1; review R2-m3), and this is the input that reaches it
    /// past [`paths`]: an ABSOLUTE prefix has no `.` or `..` component, and
    /// `Path::join` would let it REPLACE the archive root outright. No caller
    /// builds one; this is what holds when one does. (A check for
    /// `Component::ParentDir` alone would pass this path: it has none.)
    #[cfg(feature = "server")]
    #[test]
    fn the_directory_archive_refuses_a_path_that_would_leave_its_root() {
        let base = std::env::temp_dir().join(format!("yokohama_archive_lexical_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (root, elsewhere) = (base.join("root"), base.join("elsewhere"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        let absolute = format!("{}/", elsewhere.display());
        assert!(paths(7, &absolute, Why::Over, "sim-1").is_ok(), "premise: the layout alone lets it through");
        let put = DirArchive::new(&root).put(&absolute, Why::Over, "sim-1", "{\"r\":1}", "{\"t\":1}");
        assert!(matches!(&put, Err(PutError::Failed(_))), "{put:?}");
        assert!(std::fs::read_dir(&elsewhere).unwrap().next().is_none(), "nothing written outside the root");
        assert!(std::fs::read_dir(&root).unwrap().next().is_none(), "nor inside it");
        std::fs::remove_dir_all(&base).ok();
    }

    /// The layout under a PREFIX (M6.5b, R-6.5b.15) — and the umbrella's O-9
    /// claim, "the implementations write the same relative paths", extended to
    /// a non-empty one.
    ///
    /// [`NoArchive`] has no paths at all and M6.5c's bucket is the second
    /// implementation this will cover; what it holds today is that
    /// [`DirArchive`] DERIVES its two names from [`paths`] — prefix included —
    /// rather than building them itself, which is what the anti-vacuity arm
    /// below is for: an implementation that dropped the argument would write
    /// the unprefixed name and every other assertion here would still pass.
    #[test]
    fn the_layout_takes_a_prefix_and_both_implementations_agree_under_one() {
        assert_eq!(
            paths(7, "", Why::Over, "sim-1"),
            Ok(("rev7/over/sim-1.json".into(), "rev7/over/sim-1.table.json".into()))
        );
        assert_eq!(
            paths(7, "browser/a_at_b/", Why::Abandoned, "local-1"),
            Ok((
                "browser/a_at_b/rev7/abandoned/local-1.json".into(),
                "browser/a_at_b/rev7/abandoned/local-1.table.json".into()
            ))
        );
        #[cfg(feature = "server")]
        {
            let dir =
                std::env::temp_dir().join(format!("yokohama_archive_prefix_{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            let a = DirArchive::new(&dir);
            let prefix = "browser/a_at_b/";
            a.put(prefix, Why::Abandoned, "local-1", "{\"r\":1}", "{\"t\":1}").unwrap();
            let (record, table) =
                paths(yokohama_core::SEMANTICS_REV, prefix, Why::Abandoned, "local-1").unwrap();
            assert_eq!(std::fs::read_to_string(dir.join(&record)).unwrap(), "{\"r\":1}");
            assert_eq!(std::fs::read_to_string(dir.join(&table)).unwrap(), "{\"t\":1}");
            let (unprefixed, _) = paths(yokohama_core::SEMANTICS_REV, "", Why::Abandoned, "local-1").unwrap();
            assert!(!dir.join(&unprefixed).exists(), "the prefix was ignored");
            std::fs::remove_dir_all(&dir).ok();
        }
    }

    /// Containment (R-6.5b.15) and the PUT PROTOCOL (spec §2.4) in one place,
    /// because the second needs a written pair and the first writes three.
    ///
    /// Containment is asserted by CANONICALISING both sides rather than by
    /// reading the string: `..` inside a path is resolved by the filesystem, so
    /// a segment that escaped would canonicalise outside the root while still
    /// reading as though it were under it.
    #[cfg(feature = "server")]
    #[test]
    fn a_traversal_shaped_caller_writes_inside_the_root() {
        let dir =
            std::env::temp_dir().join(format!("yokohama_archive_root_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = DirArchive::new(&dir);
        let root = std::fs::canonicalize(&dir).unwrap();
        for who in ["../../x@y", "a/b@c", "..@.."] {
            let seg = caller_segment(&crate::identity::User::parse(who).unwrap());
            let prefix = format!("browser/{seg}/");
            a.put(&prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}").unwrap();
            let (record, _) = paths(yokohama_core::SEMANTICS_REV, &prefix, Why::Over, "local-1").unwrap();
            let written = std::fs::canonicalize(dir.join(&record)).unwrap();
            assert!(written.starts_with(&root), "{who:?} wrote {written:?} outside {root:?}");
        }
        // The put protocol (spec §2.4): the same bytes again is IDENTICAL,
        // different bytes a CONFLICT — because a 409 must never be the answer a
        // client deletes its only complete copy on.
        let seg = caller_segment(&crate::identity::User::parse("a/b@c").unwrap());
        let prefix = format!("browser/{seg}/");
        assert!(matches!(
            a.put(&prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}"),
            Ok(PutOutcome::Identical)
        ));
        let again = a.put(&prefix, Why::Over, "local-1", "{\"r\":2}", "{\"t\":2}");
        assert!(matches!(again, Err(PutError::Conflict { .. })), "{again:?}");
        // A half pair — the table landed, the record did not — is REPAIRED by
        // the retry, which is what the two-file put is not atomic about.
        let (record, table) =
            paths(yokohama_core::SEMANTICS_REV, &prefix, Why::Abandoned, "local-2").unwrap();
        std::fs::create_dir_all(dir.join(&table).parent().unwrap()).unwrap();
        std::fs::write(dir.join(&table), "{\"t\":9}").unwrap();
        assert!(matches!(
            a.put(&prefix, Why::Abandoned, "local-2", "{\"r\":9}", "{\"t\":9}"),
            Ok(PutOutcome::Landed)
        ));
        assert_eq!(std::fs::read_to_string(dir.join(&record)).unwrap(), "{\"r\":9}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_layout_is_one_function_for_both_reasons() {
        assert_eq!(
            paths(7, "", Why::Over, "sim-1"),
            Ok(("rev7/over/sim-1.json".into(), "rev7/over/sim-1.table.json".into()))
        );
        assert_eq!(
            paths(7, "", Why::Abandoned, "sim-1"),
            Ok(("rev7/abandoned/sim-1.json".into(), "rev7/abandoned/sim-1.table.json".into()))
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
        assert!(
            matches!(NoArchive.put("", Why::Over, "x", "{}", "{}"), Err(PutError::Failed(_))),
            "the guard, not a write — and a FAILURE, never a conflict"
        );
    }

    /// The third answer to the marker read (review fix, spec §2.4): a record
    /// that is THERE but cannot be read is a `Failed`, and nothing is written.
    ///
    /// **Why this is the centrepiece and not an edge.** The two-arm read this
    /// replaces treated every `Err` as absence and fell through to the write —
    /// and the write would have SUCCEEDED, because `rename` needs write
    /// permission on the directory and not on the file, so an archived record
    /// that had become unreadable (a mode change, a truncated write that left
    /// invalid UTF-8) would have been replaced by a different record under an
    /// `Ok(Landed)`. That is losing the only complete copy without even the 409
    /// the protocol exists to raise.
    ///
    /// A DIRECTORY at the record's path is the portable way to arrange a read
    /// that fails without being absent: every platform refuses it, and none
    /// refuses it as `NotFound`.
    ///
    /// **MUTATION-PROVEN, and the assertion that fires is the anti-vacuity
    /// one.** Restoring the two-arm read fails this cell at `nothing was
    /// written`: the fall-through wrote the TABLE and only then did the
    /// record's own write fail, so the pair was left with a clobbered table
    /// beside an unreadable record — worse than either half of the answer
    /// asserted here, and invisible to a cell that only read the return value.
    #[cfg(feature = "server")]
    #[test]
    fn a_record_that_cannot_be_read_is_a_failure_and_writes_nothing() {
        let dir =
            std::env::temp_dir().join(format!("yokohama_archive_unreadable_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir);
        let (record, table) = paths(yokohama_core::SEMANTICS_REV, "", Why::Over, "sim-4").unwrap();
        std::fs::create_dir_all(dir.join(&record)).unwrap();
        let out = a.put("", Why::Over, "sim-4", "{\"record\":1}", "{\"table\":1}");
        assert!(
            matches!(&out, Err(PutError::Failed(m)) if m.contains("sim-4.json")),
            "a marker read that is not 'absent' is a FAILURE naming the path, never a fall-through: {out:?}"
        );
        // **The anti-vacuity, and the whole point**: the TABLE is written first,
        // so a put that returned before the read was settled would have left one
        // behind — and the record's own path still holds what it held.
        assert!(!dir.join(&table).exists(), "nothing was written");
        assert!(dir.join(&record).is_dir(), "and the marker path is untouched");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(feature = "server")]
    #[test]
    fn the_directory_archive_writes_both_files_and_refuses_a_second_put() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir);
        assert_eq!(a.kind(), "dir");
        a.put("", Why::Abandoned, "sim-3", "{\"record\":1}", "{\"table\":1}").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.json")).unwrap(), "{\"record\":1}");
        assert_eq!(
            std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.table.json")).unwrap(),
            "{\"table\":1}"
        );
        // **Since M6.5b "a second put" has two answers, not one** (spec §2.4):
        // the same bytes again are a retry after a lost reply and answer
        // `Identical`, DIFFERENT bytes are the conflict. Both are asserted
        // here, because an implementation that answered `Identical` to
        // everything would pass a cell that only looked for "not written twice".
        let same = a.put("", Why::Abandoned, "sim-3", "{\"record\":1}", "{\"table\":1}");
        assert!(matches!(same, Ok(PutOutcome::Identical)), "{same:?}");
        // **`held` is the file's bytes, and the sentence never carries them**
        // (M6.5c2, R-6.5c2.11): the game-over site compares the held record in
        // Rust, while this sentence is the abandon's refusal ON THE WIRE
        // (`Games::delete`), where a whole record — its setup seed in it — must
        // never ride.
        let Err(conflict) = a.put("", Why::Abandoned, "sim-3", "{\"record\":2}", "{\"table\":2}") else {
            panic!("a second put of a DIFFERENT record is refused, never versioned");
        };
        let said = conflict.to_string();
        let PutError::Conflict { key, held } = conflict else { panic!("a conflict, not a failure: {said}") };
        assert_eq!(held.as_deref(), Some("{\"record\":1}"), "the record already there, read from the file");
        assert_eq!(key, dir.join("rev7/abandoned/sim-3.json").display().to_string());
        assert_eq!(said, format!("{key} holds a different record — a game is archived once"));
        assert!(!said.contains("\"record\""), "the sentence names the key and never the held record: {said}");
        assert_eq!(std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.json")).unwrap(), "{\"record\":1}");
        assert_eq!(
            std::fs::read_to_string(dir.join("rev7/abandoned/sim-3.table.json")).unwrap(),
            "{\"table\":1}",
            "a refused put leaves the table alone too"
        );
        // The same id under the OTHER reason is a different path — `paths` says
        // so — and lands; the "once" rule is per (id, why), and the callers
        // (R-6.5a.11) are what keep a game to one reason.
        a.put("", Why::Over, "sim-3", "{\"record\":3}", "{\"table\":3}").unwrap();
        // Both leaves' paths are asserted, not one: the mutation battery found a
        // `put` that hand-built its `over/` path survived the `abandoned/` pins.
        assert_eq!(std::fs::read_to_string(dir.join("rev7/over/sim-3.json")).unwrap(), "{\"record\":3}");
        assert_eq!(std::fs::read_to_string(dir.join("rev7/over/sim-3.table.json")).unwrap(), "{\"table\":3}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
