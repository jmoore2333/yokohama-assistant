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

/// Why a put was refused: a DIFFERENT record is already at this path, the
/// write itself failed, or the caller's share is full.
///
/// Three variants rather than one string, because the caller answers them
/// differently and must: a conflict is a 409 the client KEEPS its game on; a
/// failure is a 500, "the archive could not be reached — the game was kept"
/// (R-6.5b.7), and the outbox retries it; and a full share (security S1b,
/// finding B12) is a 507 with nothing written, which the client keeps its game
/// on and the outbox retries. A 409 that read as a failure, or a failure that
/// read as a 409, is the shape that deletes somebody's only complete copy.
#[derive(Debug)]
pub enum PutError {
    /// The path holds a different record. `key` is the path RELATIVE to the
    /// archive's root, for the operator (M6.5c2 cycle D, row 56 — an abandon's
    /// refusal carries it to a client, and the root is the server's own);
    /// `held` is the record already there where the archive read it —
    /// [`DirArchive`] always does, and `None` where it cannot say (the Worker's
    /// receipt, until its shell hands the held text back) — so the game-over
    /// site can tell an earlier completion of the SAME game from another game
    /// (M6.5c2, R-6.5c2.11). The ROUTE words its own sentence and never this
    /// one, and `held` is in neither: it is a whole record, the setup seed in it.
    Conflict { key: String, held: Option<String> },
    /// The write did not happen. Carries the implementation's own reason, which
    /// names any path RELATIVE to the archive's root (M6.5c2 cycle D, row 56),
    /// for `key`'s reason.
    Failed(String),
    /// The caller's share is full (security S1b, finding B12): nothing was
    /// written. Carries the share's two bounds for the route's sentence.
    Quota { records: usize, bytes: u64 },
}

impl std::fmt::Display for PutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { key, .. } => write!(f, "{key} holds a different record — a game is archived once"),
            Self::Failed(e) => f.write_str(e),
            Self::Quota { records, bytes } => write!(f, "the caller's archive is full ({records} records or {bytes} bytes)"),
        }
    }
}

/// A browser caller's share of this archive (security S1b, finding B12;
/// R-SEC8(f)): 256 records and 128 MiB under `browser/<caller>/`, counted
/// before every put. Set from real games: a finished game is 25–60 KB
/// pretty-printed, so 256 is years of one person's play, and 128 MiB is that
/// count at the body cap (`api::ARCHIVE_BODY_LIMIT`, 512 KiB) — so neither
/// bound is dead. The archive is a collection of played games, not an import
/// path for bulk uploads, and on the NAS it shares the volume every live game
/// saves to; the review filed 19 MB in 1.3 s against it.
pub const QUOTA_RECORDS: usize = 256;
/// [`QUOTA_RECORDS`]'s companion in bytes: 128 MiB.
pub const QUOTA_BYTES: u64 = 128 << 20;

/// What one browser caller may hold: records and bytes under its prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Quota {
    pub records: usize,
    pub bytes: u64,
}

impl Quota {
    /// The production share — every `DirArchive` a server builds.
    pub const PRODUCTION: Quota = Quota { records: QUOTA_RECORDS, bytes: QUOTA_BYTES };
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
/// server's games to one reason (umbrella R-6.5.4). **A stamp lost after a
/// landed game-over put is NOT a duplicate on its own** (M6.5c2 cycle D, row
/// 11, correcting the M6.5a close's edge): since M6.5b (R-6.5b.13) the next
/// abandon files an unstamped game whose table MIRROR says over under `over/`,
/// where the landed copy answers [`PutOutcome::Identical`] and the delete goes
/// ahead. A second copy under `abandoned/` needs the stamp lost AND a mirror
/// that no longer says over, at once: a STALE mirror (its own write failed or
/// lost a race), or an undo past game over at a one-person table, where the
/// CLI's undo applies and the mirror then truly reads not over (measured in
/// M6.5c2 cycle D and recorded in its close,
/// `docs/plans/2026-09-29-web-frontend-m6.5c2d-close.md`). That residual is
/// `Games::delete`'s to state, and it does. What it does NOT prevent: the same
/// save sent by two different callers lands twice, under two prefixes (spec
/// §2.4 — the training side dedups by digest).
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
    /// Whether the record at `prefix/rev/why/id` holds exactly `record_json`
    /// (security S1b, finding A2) — asked by the archive route BEFORE the
    /// replay, so a retry of a filed record costs a read and no replay.
    /// `false` by default: an implementation that cannot read its own files
    /// synchronously (the Worker's receipt archive) replays as before and
    /// meets the once-rule's `Identical` at the put instead.
    fn holds(&self, _prefix: &str, _why: Why, _id: &str, _record_json: &str) -> bool {
        false
    }
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
    use std::io::Write;
    use std::path::{Component, Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{paths, Archive, PutError, PutOutcome, Quota, Why};

    pub const ENV_VAR: &str = "YOKOHAMA_WEB_ARCHIVE_DIR";

    /// A directory in the archive layout — on the NAS, `/data/archive` on the
    /// store's own volume, so saves are collected during development.
    pub struct DirArchive {
        dir: PathBuf,
        quota: Quota,
        /// **One put at a time through this archive** (security S1b, finding
        /// B12): `put` takes it at the top and holds it across the marker
        /// read, the count and both writes. The count is a read and the writes
        /// come after it, so without it N puts in flight from one caller could
        /// each read the count under the bound and all land. It serializes the
        /// puts of ONE value in ONE process, and every server builds one
        /// `DirArchive`, through `from_env`.
        puts: std::sync::Mutex<()>,
    }

    impl DirArchive {
        pub fn new(dir: impl Into<PathBuf>) -> Self {
            Self { dir: dir.into(), quota: Quota::PRODUCTION, puts: std::sync::Mutex::new(()) }
        }

        /// `None` when the variable is unset (the archive is off); `Err` when it
        /// is set to a path that cannot be created — a bad value exits 1 naming
        /// the variable, like the chat variables.
        pub fn from_env() -> Result<Option<Self>, String> {
            let Some(raw) = std::env::var_os(ENV_VAR) else { return Ok(None) };
            let dir = PathBuf::from(raw);
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("{ENV_VAR}={} cannot be created: {e}", dir.display()))?;
            Ok(Some(Self { dir, quota: Quota::PRODUCTION, puts: std::sync::Mutex::new(()) }))
        }

        /// This archive under a share of the caller's — the builder a cell takes
        /// to fill one; every server builds [`Quota::PRODUCTION`].
        pub fn with_quota(mut self, quota: Quota) -> Self {
            self.quota = quota;
            self
        }

        /// What `prefix` holds, less the put's OWN two paths (`own`): records
        /// (`.json` files that are not `.table.json`) and the bytes of every
        /// `.json` file under it, at any depth. Zero where the prefix's
        /// directory does not exist yet; a read that fails is a `Failed`, never
        /// a zero.
        ///
        /// **A put's own paths never count against it** (security S1b, finding
        /// B12): a crash between the two writes leaves the pair's table with no
        /// record, and the put that repairs it would otherwise count that table
        /// twice, on disk here and again in what it adds, so a repair at the
        /// byte margin was refused for ever and the outbox never left its retry.
        fn usage(&self, prefix: &str, own: &[PathBuf]) -> Result<(usize, u64), PutError> {
            fn walk(dir: &Path, own: &[PathBuf], records: &mut usize, bytes: &mut u64) -> std::io::Result<()> {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let meta = entry.metadata()?;
                    if meta.is_dir() {
                        walk(&entry.path(), own, records, bytes)?;
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if !name.ends_with(".json") || own.contains(&entry.path()) {
                        continue;
                    }
                    *bytes += meta.len();
                    if !name.ends_with(".table.json") {
                        *records += 1;
                    }
                }
                Ok(())
            }
            let root = self.dir.join(prefix);
            if !root.exists() {
                return Ok((0, 0));
            }
            let (mut records, mut bytes) = (0usize, 0u64);
            walk(&root, own, &mut records, &mut bytes).map_err(|e| PutError::Failed(format!("count {prefix}: {e}")))?;
            Ok((records, bytes))
        }

        /// A name beside `path` that no other put is using — the pid and a
        /// counter, as `FsStore::publish` draws its own, and for its reason: two
        /// puts sharing ONE temp name interleave inside it, and whichever
        /// publishes second publishes a mix of both (M6.5c2 cycle D, row 56).
        fn temp_beside(path: &Path) -> PathBuf {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            path.with_extension(format!("json.{}.{n}.tmp", std::process::id()))
        }

        /// `text`, whole and flushed, at a fresh temp name beside `rel`'s path —
        /// the step both publishes share. Every sentence names `rel`, RELATIVE
        /// to the root (see [`PutError::Failed`]).
        fn write_temp(&self, rel: &str, text: &str) -> Result<(PathBuf, PathBuf), PutError> {
            let path = self.dir.join(rel);
            if let Some(parent) = path.parent() {
                let shown = Path::new(rel).parent().map_or_else(String::new, |p| p.display().to_string());
                std::fs::create_dir_all(parent).map_err(|e| PutError::Failed(format!("mkdir {shown}: {e}")))?;
            }
            let tmp = Self::temp_beside(&path);
            let written = std::fs::File::create(&tmp).and_then(|mut f| {
                f.write_all(text.as_bytes())?;
                f.sync_all()
            });
            if let Err(e) = written {
                let _ = std::fs::remove_file(&tmp);
                return Err(PutError::Failed(format!("write {rel}: {e}")));
            }
            Ok((path, tmp))
        }

        /// The TABLE's publish: temp + rename, over whatever is there.
        ///
        /// Over, on purpose: the once-rule is about the RECORD — the same bytes
        /// again are a retry and not a bug — and the table is written first
        /// precisely so a half pair (a table with no record, a crash between the
        /// two writes) is REPAIRED by the next put, which a refuse-if-present
        /// write could not do.
        fn write_over(&self, rel: &str, text: &str) -> Result<(), PutError> {
            let (path, tmp) = self.write_temp(rel, text)?;
            std::fs::rename(&tmp, &path).map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                PutError::Failed(format!("rename {rel}: {e}"))
            })
        }

        /// The RECORD's publish — the marker's — and never over anything: the
        /// complete temp file is HARD-LINKED into place, which the filesystem
        /// refuses when the path is taken (M6.5c2 cycle D, row 56). `Ok(false)`
        /// is that refusal; the temp name is removed either way.
        ///
        /// **This closes the marker's read-then-write race** (M6.5b close
        /// §6.2, dossier row 56). A link and not a rename, because `rename`
        /// REPLACES what it finds: the marker read answers "absent", the table
        /// lands, and another put of the same path can land its record in
        /// between — so the second of two such puts replaced the first and
        /// both answered `Landed`. A link is the create-if-absent `rename` is
        /// not, and it publishes a file that was whole before it had this name,
        /// so no partial record is ever visible at the path.
        ///
        /// **It needs a filesystem with hard links.** The NAS's btrfs volume
        /// and the workstations' APFS both have them; on one without, EVERY
        /// put is refused — a `Failed` naming the path, never a fall-back to
        /// writing over.
        fn publish_new(&self, rel: &str, text: &str) -> Result<bool, PutError> {
            let (path, tmp) = self.write_temp(rel, text)?;
            let linked = std::fs::hard_link(&tmp, &path);
            let _ = std::fs::remove_file(&tmp);
            match linked {
                Ok(()) => Ok(true),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
                Err(e) => Err(PutError::Failed(format!("link {rel}: {e}"))),
            }
        }

        /// **The record is the marker, and reading it has THREE answers, not
        /// two**: the same bytes are a retry (`Some(Identical)`), different bytes
        /// a conflict, and ABSENT (`None`) is the only road to a write.
        ///
        /// A read that failed for any OTHER reason — a path that is a
        /// directory, a file this process may not read, a truncated write that
        /// left invalid UTF-8 — must NOT fall through. A fall-through would go on
        /// to write the table and try the record, which is the one outcome this
        /// protocol exists to prevent (spec §2.4 — "a 409 must never delete the
        /// only complete copy"), reached without even a 409.
        fn read_marker(&self, rel: &str, record_json: &str) -> Result<Option<PutOutcome>, PutError> {
            match std::fs::read_to_string(self.dir.join(rel)) {
                Ok(existing) if existing == record_json => Ok(Some(PutOutcome::Identical)),
                Ok(existing) => Err(PutError::Conflict { key: rel.to_string(), held: Some(existing) }),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(PutError::Failed(format!("read {rel}: {e}"))),
            }
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
            // One put at a time (the `puts` field's doc). A poisoned lock is
            // still a lock here, since nothing behind it is left half-written
            // that the once-rule does not already handle.
            let _serialized = self.puts.lock().unwrap_or_else(|e| e.into_inner());
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
            if let Some(outcome) = self.read_marker(&record, record_json)? {
                return Ok(outcome);
            }
            // **A browser caller's share, counted before the writes** (security
            // S1b, finding B12) — AFTER the marker read, so a retry of a filed
            // record answers `Identical` once the share is full. A server's own
            // games (the empty prefix) are never counted, and nor are this
            // pair's own two paths (`usage`'s doc: a half pair's repair).
            if prefix.starts_with("browser/") {
                let own = [self.dir.join(&record), self.dir.join(&table)];
                let (records, bytes) = self.usage(prefix, &own)?;
                let adding = (record_json.len() + table_json.len()) as u64;
                if records >= self.quota.records || bytes + adding > self.quota.bytes {
                    return Err(PutError::Quota { records: self.quota.records, bytes: self.quota.bytes });
                }
            }
            // The table FIRST — overwriting a half pair's table is the repair —
            // and the record second, which is what makes the pair complete.
            self.write_over(&table, table_json)?;
            if self.publish_new(&record, record_json)? {
                return Ok(PutOutcome::Landed);
            }
            // The path was TAKEN between the marker read and the link: another
            // put's record landed there. Answered as though it had been there at
            // the read — a retry of these bytes, or a conflict — and a path that
            // is taken and still reads absent (a dangling link) is a failure.
            self.read_marker(&record, record_json)?.ok_or_else(|| {
                PutError::Failed(format!("link {record}: the path is taken and reads as absent"))
            })
        }
        fn holds(&self, prefix: &str, why: Why, id: &str, record_json: &str) -> bool {
            let Ok((record, _)) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id) else { return false };
            if !Path::new(&record).components().all(|c| matches!(c, Component::Normal(_))) {
                return false;
            }
            matches!(self.read_marker(&record, record_json), Ok(Some(PutOutcome::Identical)))
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
                SeatDoc::Human { user: "a@x.test".into(), name: None, bound_at_create: false },
                SeatDoc::Human { user: "b@x.test".into(), name: None, bound_at_create: false },
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

    /// **`holds` reads the marker and nothing else** (finding A2): true for the
    /// bytes a put landed at exactly that path, false for other bytes, another
    /// id, the other leaf, and an archive that keeps nothing — the default
    /// every implementation that cannot read its own files synchronously
    /// inherits, the Worker's receipt archive among them.
    #[cfg(feature = "server")]
    #[test]
    fn holds_is_true_for_the_landed_bytes_at_exactly_their_path() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_holds_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir);
        assert!(!a.holds("browser/a_at_b/", Why::Over, "local-1", "{\"r\":1}"), "nothing landed yet");
        a.put("browser/a_at_b/", Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}").unwrap();
        assert!(a.holds("browser/a_at_b/", Why::Over, "local-1", "{\"r\":1}"));
        assert!(!a.holds("browser/a_at_b/", Why::Over, "local-1", "{\"r\":2}"), "other bytes");
        assert!(!a.holds("browser/a_at_b/", Why::Over, "local-2", "{\"r\":1}"), "another id");
        assert!(!a.holds("browser/a_at_b/", Why::Abandoned, "local-1", "{\"r\":1}"), "the other leaf");
        assert!(!a.holds("browser/c_at_d/", Why::Over, "local-1", "{\"r\":1}"), "another caller");
        assert!(!NoArchive.holds("", Why::Over, "local-1", "{\"r\":1}"), "the default is false");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **`holds` never reads outside its root** (security S1b, finding A2): an
    /// absolute prefix that points at a real file holding exactly the bytes
    /// asked about reads `false`, and so does a `..` id. The catcher for
    /// `DirArchive::holds` without its component check, which
    /// `holds_is_true_for_the_landed_bytes_at_exactly_their_path` cannot see
    /// because every path it asks about is a plain name under the root.
    #[cfg(feature = "server")]
    #[test]
    fn holds_never_reads_outside_its_root() {
        let root = std::env::temp_dir().join(format!("yokohama_archive_holds_root_{}", std::process::id()));
        let elsewhere =
            std::env::temp_dir().join(format!("yokohama_archive_holds_elsewhere_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&elsewhere);
        let target = elsewhere.join(format!("rev{}/over/x.json", yokohama_core::SEMANTICS_REV));
        std::fs::create_dir_all(target.parent().expect("a parent")).unwrap();
        std::fs::write(&target, "held-bytes").unwrap();
        let a = DirArchive::new(&root);
        let absolute = format!("{}/", elsewhere.display());
        assert!(!a.holds(&absolute, Why::Over, "x", "held-bytes"), "an absolute prefix is never read");
        assert!(!a.holds("", Why::Over, "..", "held-bytes"), "nor a `..` id");
        std::fs::remove_dir_all(&elsewhere).ok();
        std::fs::remove_dir_all(&root).ok();
    }

    /// **A browser caller's share is counted before a put, and the count
    /// never touches a retry or the server's own games** (finding B12;
    /// R-SEC8(f); Review Focus 2). Under a two-record share two pairs land, a
    /// third is refused as `Quota` with nothing written, the first pair's
    /// bytes again are `Identical` (the marker read precedes the count),
    /// another caller's share is its own, and three server games land under
    /// the empty prefix. Then the BYTES: a hundred-byte share takes one
    /// eighty-byte pair and refuses the next.
    #[cfg(feature = "server")]
    #[test]
    fn a_browser_callers_share_is_counted_before_a_put_and_a_retry_survives_a_full_one() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_quota_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir).with_quota(Quota { records: 2, bytes: 1 << 20 });
        let prefix = "browser/a_at_b/";
        assert!(matches!(a.put(prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}"), Ok(PutOutcome::Landed)));
        assert!(matches!(a.put(prefix, Why::Abandoned, "local-2", "{\"r\":2}", "{\"t\":2}"), Ok(PutOutcome::Landed)));
        let third = a.put(prefix, Why::Over, "local-3", "{\"r\":3}", "{\"t\":3}");
        assert!(matches!(third, Err(PutError::Quota { records: 2, bytes }) if bytes == 1 << 20), "{third:?}");
        let (record, table) = paths(yokohama_core::SEMANTICS_REV, prefix, Why::Over, "local-3").unwrap();
        assert!(!dir.join(&record).exists() && !dir.join(&table).exists(), "nothing was written");
        assert!(matches!(a.put(prefix, Why::Over, "local-1", "{\"r\":1}", "{\"t\":1}"), Ok(PutOutcome::Identical)), "a retry survives a full share");
        assert!(matches!(a.put("browser/c_at_d/", Why::Over, "local-3", "{\"r\":3}", "{\"t\":3}"), Ok(PutOutcome::Landed)), "another caller's share");
        for id in ["sim-1", "sim-2", "sim-3"] {
            assert!(matches!(a.put("", Why::Over, id, "{\"r\":9}", "{\"t\":9}"), Ok(PutOutcome::Landed)), "the server's own games are never counted");
        }
        let b = DirArchive::new(&dir).with_quota(Quota { records: 100, bytes: 100 });
        let forty = format!("{{\"r\":\"{}\"}}", "x".repeat(32));
        assert_eq!(forty.len(), 40, "the premise");
        assert!(matches!(b.put("browser/e_at_f/", Why::Over, "local-1", &forty, &forty), Ok(PutOutcome::Landed)), "eighty bytes under a hundred");
        let over = b.put("browser/e_at_f/", Why::Over, "local-2", &forty, &forty);
        assert!(matches!(over, Err(PutError::Quota { records: 100, bytes: 100 })), "a hundred and sixty over a hundred: {over:?}");
        assert_eq!(Quota::PRODUCTION, Quota { records: QUOTA_RECORDS, bytes: QUOTA_BYTES });
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **A put's own two paths never count against it** (security S1b, finding
    /// B12). A crash between the two writes leaves a half pair, the table with
    /// no record, and the put that repairs it counted that table twice: on
    /// disk, and again in what it adds. At the byte margin the repair was then
    /// refused for ever, and the outbox never left its retry. Here a 40-byte
    /// table at the pair's OWN path under an 80-byte share takes the 80-byte
    /// pair, since the finished pair is exactly the share. The control is the
    /// same share with a 40-byte table of ANOTHER id, which is the caller's
    /// and refuses the pair.
    #[cfg(feature = "server")]
    #[test]
    fn a_half_pairs_repair_never_counts_its_own_table_against_the_share() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_half_pair_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let forty = format!("{{\"r\":\"{}\"}}", "x".repeat(32));
        assert_eq!(forty.len(), 40, "the premise");
        let a = DirArchive::new(&dir).with_quota(Quota { records: 100, bytes: 80 });
        let plant = |prefix: &str, id: &str| {
            let (_, table) = paths(yokohama_core::SEMANTICS_REV, prefix, Why::Over, id).unwrap();
            std::fs::create_dir_all(dir.join(&table).parent().expect("a parent")).unwrap();
            std::fs::write(dir.join(&table), &forty).unwrap();
        };
        plant("browser/g_at_h/", "local-1");
        let repair = a.put("browser/g_at_h/", Why::Over, "local-1", &forty, &forty);
        assert!(matches!(repair, Ok(PutOutcome::Landed)), "the repair lands at exactly the share: {repair:?}");
        plant("browser/i_at_j/", "local-9");
        let control = a.put("browser/i_at_j/", Why::Over, "local-1", &forty, &forty);
        assert!(matches!(control, Err(PutError::Quota { records: 100, bytes: 80 })), "another id's table counts: {control:?}");
        std::fs::remove_dir_all(&dir).ok();
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
        assert_eq!(key, "rev7/abandoned/sim-3.json", "relative to the root (M6.5c2 cycle D, row 56)");
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
    /// **The marker is PUBLISHED, never written over** (M6.5c2 cycle D, row
    /// 56) — the catcher for a record written in place, or renamed over
    /// whatever reached its path after the marker was read.
    ///
    /// The marker read answers "absent", the table lands, and only then is the
    /// record written — so another put of the same path can land its own record
    /// in between. `rename` replaces what it finds, so the second of two such
    /// puts REPLACED the first and both answered `Landed`: the once-rule broken
    /// with no conflict said. The record is now linked into place from a
    /// complete temp file, which the filesystem refuses when the path is taken,
    /// so no partial record is ever visible at it and nothing there is replaced.
    ///
    /// **The race, made deterministic without a second thread**: the record's
    /// path is a symlink to the pair's OWN table path, so it reads absent at the
    /// marker read (its target does not exist yet) and present once the table
    /// has landed — the shape of another put landing in between. The put must
    /// answer `Conflict` holding what is there and leave both files alone. A
    /// rename over the link answers `Landed`; a write in place follows the link
    /// and puts the RECORD into the table's file.
    #[cfg(all(feature = "server", unix))]
    #[test]
    fn the_marker_is_written_atomically() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_marker_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let a = DirArchive::new(&dir);
        let (record, table) = paths(yokohama_core::SEMANTICS_REV, "", Why::Over, "sim-5").unwrap();
        let leaf = dir.join(&record).parent().expect("a leaf").to_path_buf();
        std::fs::create_dir_all(&leaf).unwrap();
        std::os::unix::fs::symlink("sim-5.table.json", dir.join(&record)).unwrap();
        assert!(!dir.join(&record).exists(), "premise: the marker reads absent");

        let out = a.put("", Why::Over, "sim-5", "{\"record\":1}", "{\"table\":1}");
        let Err(PutError::Conflict { held, .. }) = out else {
            panic!("a path taken after the marker read is a conflict, never replaced: {out:?}");
        };
        assert_eq!(held.as_deref(), Some("{\"table\":1}"), "held is what reached the path");
        assert_eq!(std::fs::read_to_string(dir.join(&table)).unwrap(), "{\"table\":1}", "the table's file holds the table");
        assert!(
            std::fs::symlink_metadata(dir.join(&record)).unwrap().file_type().is_symlink(),
            "what took the path is still there",
        );
        let temps: Vec<String> = std::fs::read_dir(&leaf)
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(temps.is_empty(), "no temp file is left behind: {temps:?}");

        // Anti-vacuity: an ordinary put still lands, as a plain file.
        let landed = a.put("", Why::Over, "sim-6", "{\"record\":6}", "{\"table\":6}");
        assert!(matches!(landed, Ok(PutOutcome::Landed)), "{landed:?}");
        let (six, _) = paths(yokohama_core::SEMANTICS_REV, "", Why::Over, "sim-6").unwrap();
        assert!(std::fs::symlink_metadata(dir.join(&six)).unwrap().file_type().is_file());
        assert_eq!(std::fs::read_to_string(dir.join(&six)).unwrap(), "{\"record\":6}");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// **A failed or refused put names its path RELATIVE to the archive root**
    /// (M6.5c2 cycle D, row 56). The abandon wraps the archive's own sentence
    /// into a 500 a client reads ("the archive could not be written (…)"), so
    /// an absolute path there was the server's filesystem layout on the wire;
    /// the relative one is still exact for an operator, who knows the root.
    /// Three arms: an unreadable marker, a table that cannot be published, and
    /// a conflict's key.
    #[cfg(feature = "server")]
    #[test]
    fn a_failed_put_names_a_relative_path() {
        let dir = std::env::temp_dir().join(format!("yokohama_archive_relative_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let root = dir.display().to_string();
        let a = DirArchive::new(&dir);
        let rel = |id: &str| paths(yokohama_core::SEMANTICS_REV, "", Why::Over, id).unwrap();

        let (record, _) = rel("sim-7");
        std::fs::create_dir_all(dir.join(&record)).unwrap();
        let out = a.put("", Why::Over, "sim-7", "{\"record\":7}", "{\"table\":7}");
        let Err(PutError::Failed(said)) = out else { panic!("an unreadable marker fails: {out:?}") };
        assert!(said.starts_with(&format!("read {record}: ")) && !said.contains(&root), "{said}");

        let (_, table) = rel("sim-8");
        std::fs::create_dir_all(dir.join(&table)).unwrap();
        std::fs::write(dir.join(&table).join("x"), "a table's path taken by a full directory").unwrap();
        let out = a.put("", Why::Over, "sim-8", "{\"record\":8}", "{\"table\":8}");
        let Err(PutError::Failed(said)) = out else { panic!("an unpublishable table fails: {out:?}") };
        assert!(said.contains(&table) && !said.contains(&root), "{said}");

        let (record, _) = rel("sim-9");
        a.put("", Why::Over, "sim-9", "{\"record\":9}", "{\"table\":9}").unwrap();
        let out = a.put("", Why::Over, "sim-9", "{\"record\":10}", "{\"table\":10}");
        let Err(conflict) = out else { panic!("different bytes conflict") };
        assert_eq!(conflict.to_string(), format!("{record} holds a different record — a game is archived once"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
