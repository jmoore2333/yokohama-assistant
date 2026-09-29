//! Where saved games live.
//!
//! A trait rather than a `PathBuf` because the hosted plan puts this server in
//! a container that **sleeps when idle and loses its disk** — Cloudflare
//! Containers and Modal both scale to zero. A session that exists only in a
//! process is a session that dies with it, so every game here is persisted as
//! its `GameRecord` and is fully reconstructible from that alone, which the
//! engine already guarantees (`Session::from_record` runs every refusal).
//!
//! The trait is byte-shaped, not path-shaped, so an object store is a second
//! implementation rather than a refactor.
//!
//! **Two things changed shape for M6.1, and both are about what a second
//! document costs.** A game is no longer one blob: beside the `GameRecord`
//! there is a table document saying who may sit at it, so the trait is keyed by
//! [`Doc`] rather than by an id alone — a second store, or a second directory,
//! would have made "the record and its table" two things that can disagree with
//! nothing able to say so. And the write is CONDITIONAL
//! ([`Store::write_if_version`]): the compare-and-swap `Games::persist` already
//! performed by hand, moved to the seam where the store can actually hold it.
//! Within a process that swap is atomic; ACROSS processes it detects a conflict
//! and does not prevent one, which is exactly the property `persist` has always
//! had — restated here rather than quietly widened.

#[cfg(feature = "server")]
use std::path::{Path, PathBuf};
// ---- end server ----

/// A game's name. Also a filename, which is why [`is_safe_id`] exists.
pub type GameId = String;

/// Which document under a game id.
///
/// They are separate documents with separate versions, so a write to one is
/// never a conflict on the other: a seat change, a move and a message are
/// different questions and land independently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Doc {
    /// The `GameRecord` JSON — `<id>.json`. What the engine replays; the only
    /// durable truth about a game's position.
    Record,
    /// The table document — who may sit at this game, and in which seat.
    /// `<id>.table.json`. Written by `Games::create` and by every `keep`; it is
    /// what `GET /api/games` lists over, so a record with no table beside it is
    /// a game this API does not serve (R-P7).
    Table,
    /// The table's chat — `<id>.chat.json` (M6.4, spec §2.1). Its own document
    /// so a message and a move are different questions that land
    /// independently, and so a message is never in the record: nothing the
    /// engine replays, and nothing an undo can reach (R-6.4.9).
    Chat,
    /// The ONE house chat, at a file no game id can spell: `_house.chat.json`.
    /// A leading underscore fails [`is_safe_id`]'s first-character rule, so it
    /// needs no reservation of its own and `list(Doc::Record)` excludes it by
    /// construction.
    House,
}

impl Doc {
    // Reached only by `FsStore::path`; the layer without `server` never spells a file name.
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    fn suffix(self) -> &'static str {
        match self {
            Self::Record => ".json",
            Self::Table => ".table.json",
            Self::Chat => ".chat.json",
            // The same suffix as `Chat`: what sets the house apart is the
            // underscore `FsStore::path` prepends, not what it ends with.
            Self::House => ".chat.json",
        }
    }
}

/// An opaque version of a stored document.
///
/// For the file and memory stores it is the digest of the exact bytes, so two
/// writers of identical bytes agree; a Postgres store would carry a row
/// version instead and nothing above this module would notice. Callers compare
/// one and pass it back; they never construct one.
///
/// In-process only and never persisted, which is why a hasher with no
/// stability guarantee across builds is exactly right here: the only
/// comparison is between two values computed by one running process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version(u64);

/// The version a store reports for these exact bytes.
pub fn version_of(json: &str) -> Version {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut h);
    Version(h.finish())
}

/// Why a conditional write did not land.
///
/// The split is the point: a CONFLICT means this writer is wrong about the
/// world and its handle should be dropped, while an I/O failure means the
/// writer is right and the disk is not. `Games::persist` acts on exactly that
/// difference — see `Games::conflict`.
#[derive(Debug)]
pub enum WriteError {
    /// The document is not at `expected`: `current` is what is there now, or
    /// `None` when the document is absent and a replace was asked for.
    Conflict { current: Option<Version> },
    Io(String),
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict { current: Some(_) } => {
                write!(f, "the stored document changed under this writer")
            }
            Self::Conflict { current: None } => write!(f, "the document is absent"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

pub trait Store: Send + Sync {
    /// Every id holding a `doc`, sorted.
    fn list(&self, doc: Doc) -> Result<Vec<GameId>, String>;
    /// The document and its version, or `None` when absent. An unreadable
    /// present file is an `Err`, never `None` — conflating "cannot be read"
    /// with "is not there" is what let a new game overwrite a real one, and the
    /// distinction is now in the return type rather than in a second method.
    fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String>;
    /// `expected: None` creates and refuses a present document; `Some(v)`
    /// replaces only if the current version is `v`. Atomic within a process;
    /// across processes it DETECTS a conflict and does not prevent one — the
    /// property `Games::persist` has always had, restated at the seam.
    fn write_if_version(
        &self,
        doc: Doc,
        id: &str,
        json: &str,
        expected: Option<Version>,
    ) -> Result<Version, WriteError>;
    /// Every document of the game — the record, the table and the chat; never
    /// the house. An `Err` only when NONE of them was there.
    fn delete(&self, id: &str) -> Result<(), String>;
}

/// The id suffixes the non-record documents reserve, which no game may end
/// with: [`Doc::Table`]'s `.table` and [`Doc::Chat`]'s `.chat`.
///
/// `<id>.table.json` IS `<id>`'s table document, and a dot is a legal character
/// in a name — `is_safe_id`'s own cell lists `A.b-c_d` as good — so without
/// this the game `mygame.table` would have its RECORD written to the exact path
/// `mygame`'s table occupies. On `FsStore` that is reachable data loss and not
/// merely confusion: `list(Doc::Record)` excludes the table suffix, so the game
/// vanishes from the list, and `delete("mygame")` — for a `mygame` that never
/// existed — removes the other game's record and answers `Ok`, which the route
/// serves as a 200. M6.4's chat document is the same hole under a second name,
/// closed by adding to this list rather than by a second rule.
///
/// `MemStore` keys `(Doc, GameId)` and has none of this. That the two
/// implementations of one trait diverge is exactly why the guard lives HERE,
/// above both, rather than in the store that has the problem.
const RESERVED_ID_SUFFIXES: [&str; 2] = [".table", ".chat"];

/// Whether `id` may be used as a storage key.
///
/// An allowlist, and deliberately not a denylist or an escaping pass: `..`,
/// `/`, a leading `-`, a NUL and every encoding trick are excluded by not being
/// on the list rather than by being enumerated. The same shape (and the same
/// reasoning) as `yokohama_bridge::capture::safe_table_id`, which guards the
/// one other place in this repo where an outside string becomes a filename.
///
/// The SUBTRACTIONS from that allowlist are two: `RESERVED_ID_SUFFIXES` —
/// read it for why a name may not end in `.table` or `.chat` — and the house
/// document's stored name, refused by name (`is_the_house`).
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        && !spells_another_document(id)
        && !is_the_house(id)
}

/// The name the house document is STORED under: an underscore, then
/// [`crate::chat::HOUSE`] — `_house`, so `FsStore` writes `_house.chat.json`.
///
/// **The underscore is the reservation**: no id [`is_safe_id`] admits starts
/// with one, so no game's document can reach the house's file, and the plain
/// id `house` stays an ordinary game name. Spelled HERE once, for the path and
/// for the check below (M6.5c2 cycle D, row 130).
fn house_stem() -> String {
    format!("_{}", crate::chat::HOUSE)
}

/// Whether `id` is the house's stored name, [`house_stem`], in any case — a
/// NAMED check, not the first-character rule's accident (M6.5c2 cycle D, row
/// 130).
///
/// Today the first-character rule refuses `_house` already, so this refuses
/// nothing new. What it pins is the TIE between the two: the check reads the
/// stem the path is built from, so an edit that dropped the underscore — or
/// renamed the house to a word with none — still leaves the house's stored
/// name refused as a game id, and a game can never be given the house's file.
/// Case-insensitive, as the suffixes are read, because APFS and NTFS fold case.
fn is_the_house(id: &str) -> bool {
    id.eq_ignore_ascii_case(&house_stem())
}

/// Whether `id` would spell another game's table or chat document.
///
/// **Case-INSENSITIVE**, because the filesystems this is written and tested on
/// are: on APFS and NTFS, `mygame.TABLE.json` and `mygame.table.json` are one
/// file, so a case-sensitive guard would close the hole on the deployment and
/// leave it open on both workstations.
///
/// The slice is safe because `is_safe_id` has already established that every
/// character is ASCII by the time `&&` reaches this call — the checks are
/// short-circuiting and ordered.
fn spells_another_document(id: &str) -> bool {
    RESERVED_ID_SUFFIXES
        .iter()
        .any(|s| id.len() >= s.len() && id[id.len() - s.len()..].eq_ignore_ascii_case(s))
}

/// The house document answers to exactly one id, on BOTH stores.
///
/// `FsStore` spells it `_house.chat.json` and would happily spell
/// `_house2.chat.json` beside it; `MemStore` keys `(Doc, GameId)` and asks only
/// `is_safe_id`, which `"house2"` passes. Either way a second house is a file
/// nothing lists, nothing deletes and nobody reads, so the rule lives above
/// both implementations, as the id guard does.
fn check_house_id(doc: Doc, id: &str) -> Result<(), String> {
    if doc == Doc::House && id != crate::chat::HOUSE {
        return Err(format!("the house chat has one id, {:?}, not {id:?}", crate::chat::HOUSE));
    }
    Ok(())
}

/// The local implementation: one JSON file per document in a directory.
#[cfg(feature = "server")]
pub struct FsStore {
    dir: PathBuf,
    /// Held across the read, the compare AND the write of
    /// [`Store::write_if_version`], which is what makes the swap atomic within
    /// this process. It does NOT reach across processes — see the trait's doc.
    lock: std::sync::Mutex<()>,
}

#[cfg(feature = "server")]
impl FsStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into(), lock: std::sync::Mutex::new(()) }
    }

    fn path(&self, doc: Doc, id: &str) -> Result<PathBuf, String> {
        check_house_id(doc, id)?;
        if doc == Doc::House {
            // The leading underscore is the whole of the reservation: no id
            // `is_safe_id` admits may start with one, so no game can reach this
            // file and it needs no suffix of its own. The stem is `house_stem`'s,
            // the one spelling `is_safe_id` also refuses BY NAME, so the file
            // and the check cannot come apart (M6.5c2 cycle D, row 130).
            // `check_house_id` above has already pinned `id` to the house's.
            return Ok(self.dir.join(format!("{}{}", house_stem(), doc.suffix())));
        }
        if !is_safe_id(id) {
            return Err(format!("{id:?} is not a usable game name"));
        }
        Ok(self.dir.join(format!("{id}{}", doc.suffix())))
    }

    /// The current document, with no lock of its own — the caller holds it.
    fn read_locked(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
        match std::fs::read_to_string(self.path(doc, id)?) {
            Ok(json) => {
                let v = version_of(&json);
                Ok(Some((json, v)))
            }
            // Absence is `NotFound` and NOTHING else. Every other error is a
            // file that exists and cannot be spoken for; answering "absent" for
            // one of those let `create` overwrite a real game — measured with a
            // `chmod 000`: the POST returned 201 and the original record was
            // gone.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Atomic write-then-rename with an `fsync`, the same discipline
    /// `Session::save` uses and for the same reason: this rewrites the ONLY
    /// copy of a game after every applied move, so a crash mid-write must not
    /// be able to destroy it. The target is never opened for writing.
    ///
    /// **The temp path carries a unique suffix, and that is not decoration.**
    /// It used to be `<id>.json.tmp` — a pure function of the id — so two
    /// writers for the same game shared one temp file and interleaved at every
    /// step: A's `File::create` truncates what B wrote, A's `rename` publishes
    /// B's partial bytes as A's record, and B's `rename` then fails ENOENT.
    /// Measured under six concurrent creates: an unparseable 869-byte record
    /// that `list` still returned and `create` refused to replace, and rounds
    /// where the durable record held a different game than the handle the
    /// 201'd client was playing. The per-game `Mutex<Repl>` does not cover it,
    /// because `create` and the cold-rebuild in `handle` both persist outside
    /// any shared lock. The store's own `lock` now serializes writers WITHIN a
    /// process; the unique name is what holds across processes, where no mutex
    /// reaches.
    ///
    /// A private counter and the pid, rather than a random name: no dependency,
    /// unique within a process by construction, and unique across processes
    /// because the pid is. A leftover temp from a crash is inert — it is never
    /// read, never listed (`list` takes only a `Doc`'s own suffix), and the
    /// next write for that id uses a different name.
    fn publish(&self, path: &Path, json: &str) -> Result<(), String> {
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension(format!(
            "{}.{}.tmp",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        {
            let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        let renamed = std::fs::rename(&tmp, path).map_err(|e| e.to_string());
        if renamed.is_err() {
            // Do not leave our own temp behind on a failed publish.
            let _ = std::fs::remove_file(&tmp);
        }
        renamed
    }
}

#[cfg(feature = "server")]
impl Store for FsStore {
    fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
        if doc == Doc::House {
            // Not a scan: the house is one known path, so the answer is whether
            // that file is there. An absent directory is still an empty answer,
            // and an unreadable entry is an error rather than an absence — the
            // same rule `read_locked` follows.
            let path = self.path(Doc::House, crate::chat::HOUSE)?;
            return match std::fs::metadata(&path) {
                Ok(_) => Ok(vec![crate::chat::HOUSE.to_string()]),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
                Err(e) => Err(e.to_string()),
            };
        }
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(e) => e,
            // An absent directory is an empty store, not a failure: the first
            // save creates it, and a fresh install must not have to be
            // provisioned before it can answer "what games do I have?".
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e.to_string()),
        };
        let mut ids: Vec<GameId> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.path().file_name()?.to_str()?.to_string();
                // `<id>.table.json` and `<id>.chat.json` end with `.json` too,
                // and stripping that suffix leaves `<id>.table` or
                // `<id>.chat` — which `is_safe_id` now REFUSES, by
                // [`RESERVED_ID_SUFFIXES`]. So this branch is redundant defence
                // rather than the thing that keeps those documents out of a
                // record listing, and it is kept as the second of two: the id
                // guard closes the name at the way in, and this closes the
                // listing even for a file some other writer left there. The
                // house needs neither: `_house` fails the id guard's first
                // character.
                if doc == Doc::Record
                    && (name.ends_with(Doc::Table.suffix())
                        || name.ends_with(Doc::Chat.suffix()))
                {
                    return None;
                }
                let id = name.strip_suffix(doc.suffix())?.to_string();
                is_safe_id(&id).then_some(id)
            })
            .collect();
        ids.sort();
        Ok(ids)
    }

    fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
        self.read_locked(doc, id)
    }

    fn write_if_version(
        &self,
        doc: Doc,
        id: &str,
        json: &str,
        expected: Option<Version>,
    ) -> Result<Version, WriteError> {
        let path = self.path(doc, id).map_err(WriteError::Io)?;
        let _swap = self.lock.lock().expect("store lock");
        let current = self.read_locked(doc, id).map_err(WriteError::Io)?.map(|(_, v)| v);
        match (expected, current) {
            // A create, and the document is absent: ours.
            (None, None) => {}
            // A replace, and the document is still what the caller last saw.
            (Some(want), Some(have)) if want == have => {}
            (_, current) => return Err(WriteError::Conflict { current }),
        }
        self.publish(&path, json).map_err(WriteError::Io)?;
        Ok(version_of(json))
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let mut removed = false;
        for doc in [Doc::Record, Doc::Table, Doc::Chat] {
            let path = self.path(doc, id)?;
            match std::fs::remove_file(&path) {
                Ok(()) => removed = true,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        if removed {
            Ok(())
        } else {
            Err(format!("no game named {id:?}"))
        }
    }
}

// ---- end server ----

/// An in-memory store, for tests and for a run that must leave no trace.
#[derive(Default)]
pub struct MemStore {
    docs: std::sync::Mutex<std::collections::BTreeMap<(Doc, GameId), String>>,
}

impl Store for MemStore {
    fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
        Ok(self
            .docs
            .lock()
            .expect("store lock")
            .keys()
            .filter(|(d, _)| *d == doc)
            .map(|(_, id)| id.clone())
            .collect())
    }

    fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
        check_house_id(doc, id)?;
        if !is_safe_id(id) {
            return Err(format!("{id:?} is not a usable game name"));
        }
        Ok(self
            .docs
            .lock()
            .expect("store lock")
            .get(&(doc, id.to_string()))
            .map(|json| (json.clone(), version_of(json))))
    }

    fn write_if_version(
        &self,
        doc: Doc,
        id: &str,
        json: &str,
        expected: Option<Version>,
    ) -> Result<Version, WriteError> {
        check_house_id(doc, id).map_err(WriteError::Io)?;
        if !is_safe_id(id) {
            return Err(WriteError::Io(format!("{id:?} is not a usable game name")));
        }
        // The map's own lock spans the read, the compare and the write, which
        // is the same guarantee `FsStore` buys with a separate mutex.
        let mut docs = self.docs.lock().expect("store lock");
        let key = (doc, id.to_string());
        let current = docs.get(&key).map(|json| version_of(json));
        match (expected, current) {
            (None, None) => {}
            (Some(want), Some(have)) if want == have => {}
            (_, current) => return Err(WriteError::Conflict { current }),
        }
        docs.insert(key, json.to_string());
        Ok(version_of(json))
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let mut docs = self.docs.lock().expect("store lock");
        let removed = [Doc::Record, Doc::Table, Doc::Chat]
            .into_iter()
            .filter(|doc| docs.remove(&(*doc, id.to_string())).is_some())
            .count();
        if removed > 0 {
            Ok(())
        } else {
            Err(format!("no game named {id:?}"))
        }
    }
}

/// The default local directory: `~/.yokohama/games`, or `./yokohama-games` when
/// there is no home to speak of.
#[cfg(feature = "server")]
pub fn default_dir() -> PathBuf {
    std::env::var_os("YOKOHAMA_WEB_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".yokohama").join("games")))
        .unwrap_or_else(|| PathBuf::from("yokohama-games"))
}

// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// An `FsStore` on a temporary directory that removes itself.
    ///
    /// A `Drop` rather than a trailing `remove_dir_all`, so a cell that PANICS
    /// still cleans up: a failing test that leaves a directory behind makes the
    /// next run's failure a different one.
    struct TempFs(FsStore, PathBuf);

    impl Drop for TempFs {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.1);
        }
    }

    impl Store for TempFs {
        fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> {
            self.0.list(doc)
        }
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
            self.0.read(doc, id)
        }
        fn write_if_version(
            &self,
            doc: Doc,
            id: &str,
            json: &str,
            expected: Option<Version>,
        ) -> Result<Version, WriteError> {
            self.0.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> {
            self.0.delete(id)
        }
    }

    /// Both implementations, so a cell written once binds both.
    ///
    /// A property asserted only against `MemStore` is a property of the test
    /// double; the server runs on `FsStore`, and the two have different
    /// failure modes (a directory that is not there, a file that cannot be
    /// read). Every cell below names which store it was running when it fails.
    fn both(name: &str) -> Vec<(&'static str, Arc<dyn Store>)> {
        let dir =
            std::env::temp_dir().join(format!("yokohama-store-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        vec![
            ("mem", Arc::new(MemStore::default()) as Arc<dyn Store>),
            ("fs", Arc::new(TempFs(FsStore::new(&dir), dir.clone())) as Arc<dyn Store>),
        ]
    }

    /// **The conditional write, in both of its refusals.**
    ///
    /// `expected: None` means "create", so a present document refuses rather
    /// than being overwritten; `Some(v)` means "replace what I last saw", so a
    /// version that has moved on refuses and NAMES what is there now. The
    /// second is what `Games::persist` performed by hand and now delegates.
    #[test]
    fn a_create_refuses_a_present_document_and_a_stale_version_conflicts() {
        for (name, store) in both("cas") {
            let v1 = store.write_if_version(Doc::Record, "g", "{\"a\":1}", None).expect(name);
            match store.write_if_version(Doc::Record, "g", "{\"a\":2}", None) {
                Err(WriteError::Conflict { current: Some(c) }) => assert_eq!(c, v1, "{name}"),
                other => panic!("{name}: a second create must conflict, got {other:?}"),
            }
            let v2 = store.write_if_version(Doc::Record, "g", "{\"a\":2}", Some(v1)).expect(name);
            assert_ne!(v1, v2);
            match store.write_if_version(Doc::Record, "g", "{\"a\":3}", Some(v1)) {
                Err(WriteError::Conflict { current: Some(c) }) => {
                    assert_eq!(c, v2, "{name}: names the current")
                }
                other => panic!("{name}: a stale version must conflict, got {other:?}"),
            }
            let (json, v) = store.read(Doc::Record, "g").expect(name).expect("present");
            assert_eq!((json.as_str(), v), ("{\"a\":2}", v2));
        }
    }

    /// **Two documents under one id are two documents.**
    ///
    /// The record and the table are written, versioned and listed
    /// independently — a game with no table is not a game with an empty one —
    /// and only `delete` takes both, because a game that is gone is gone.
    ///
    /// The last two assertions are the retired memory-store cell's: an unsafe
    /// id is refused by BOTH stores, not only by the one that touches a
    /// filesystem, and a second delete is an error rather than a silent ok.
    ///
    /// **One id here carries a DOT, and that is the point of running this cell
    /// on both stores.** `MemStore` keys `(Doc, GameId)` and cannot confuse two
    /// documents whatever the name; `FsStore` spells each as one filename in
    /// one directory and has to tell `a.b.json` from `a.b.table.json`. With
    /// only dotless ids the cell would pass against a store that could not do
    /// that — which is the divergence `both()` exists to catch.
    #[test]
    fn the_two_kinds_are_separate_documents_under_one_id() {
        for (name, store) in both("kinds") {
            // The dotted round trip first, so a store that cannot separate two
            // documents under a dotted name fails here rather than later.
            store.write_if_version(Doc::Record, "a.b", "{\"r\":0}", None).expect(name);
            store.write_if_version(Doc::Table, "a.b", "{\"t\":0}", None).expect(name);
            assert_eq!(
                store.read(Doc::Record, "a.b").expect(name).expect("present").0,
                "{\"r\":0}",
                "{name}: a dotted id's record is its own",
            );
            assert_eq!(
                store.read(Doc::Table, "a.b").expect(name).expect("present").0,
                "{\"t\":0}",
                "{name}: and so is its table",
            );

            store.write_if_version(Doc::Record, "g", "{\"r\":1}", None).expect(name);
            assert!(store.read(Doc::Table, "g").expect(name).is_none(), "{name}");
            store.write_if_version(Doc::Table, "g", "{\"t\":1}", None).expect(name);
            assert_eq!(
                store.list(Doc::Table).expect(name),
                vec!["a.b".to_string(), "g".to_string()],
            );
            store.write_if_version(Doc::Record, "h", "{\"r\":2}", None).expect(name);
            assert_eq!(
                store.list(Doc::Table).expect(name),
                vec!["a.b".to_string(), "g".to_string()],
                "{name}: h has no table",
            );
            assert_eq!(
                store.list(Doc::Record).expect(name),
                vec!["a.b".to_string(), "g".to_string(), "h".to_string()],
                "{name}: and a dotted record is listed as a record, not hidden as a table",
            );
            store.delete("g").expect(name);
            assert!(
                store.read(Doc::Record, "g").expect(name).is_none()
                    && store.read(Doc::Table, "g").expect(name).is_none(),
                "{name}: a delete takes both",
            );
            assert!(
                store.delete("g").is_err(),
                "{name}: deleting twice is an error, not a silent ok",
            );
            store.delete("a.b").expect(name);
            assert!(
                store.read(Doc::Record, "a.b").expect(name).is_none()
                    && store.read(Doc::Table, "a.b").expect(name).is_none(),
                "{name}: and a dotted id's delete takes both of ITS documents",
            );
            assert!(
                store.write_if_version(Doc::Record, "../escape", "{}", None).is_err(),
                "{name}: an unsafe id is refused",
            );
        }
    }

    /// **Three kinds under one dotted id, on both stores** (M6.4 §2.1). The
    /// M6.3-era cell above proved two; a store that separated two and confused
    /// the third would pass it unchanged.
    #[test]
    fn the_three_kinds_are_separate_documents_under_one_id() {
        for (name, store) in both("three-kinds") {
            store.write_if_version(Doc::Record, "a.b", "{\"r\":0}", None).expect(name);
            store.write_if_version(Doc::Table, "a.b", "{\"t\":0}", None).expect(name);
            store.write_if_version(Doc::Chat, "a.b", "{\"c\":0}", None).expect(name);
            assert_eq!(
                store.read(Doc::Chat, "a.b").expect(name).expect("present").0,
                "{\"c\":0}",
                "{name}",
            );
            assert_eq!(
                store.read(Doc::Record, "a.b").expect(name).expect("present").0,
                "{\"r\":0}",
                "{name}",
            );
            assert_eq!(
                store.list(Doc::Record).expect(name),
                vec!["a.b".to_string()],
                "{name}: the chat file is not a record",
            );
            assert_eq!(store.list(Doc::Chat).expect(name), vec!["a.b".to_string()], "{name}");
            store.delete("a.b").expect(name);
            assert!(
                store.read(Doc::Chat, "a.b").expect(name).is_none(),
                "{name}: delete removes the chat document too",
            );
        }
    }

    /// **The house is one file nothing can spell, and delete never touches it.**
    ///
    /// A second house id would be a second file nothing lists and nothing
    /// deletes, so both stores refuse one — `MemStore` too, where
    /// `is_safe_id("house2")` is perfectly true and only this rule stands
    /// between the two implementations disagreeing about what exists.
    #[test]
    fn the_house_document_is_one_fixed_file_outside_every_game() {
        for (name, store) in both("house") {
            assert!(
                store.list(Doc::House).expect(name).is_empty(),
                "{name}: no house before one is written",
            );
            store.write_if_version(Doc::House, crate::chat::HOUSE, "{\"h\":0}", None).expect(name);
            assert_eq!(
                store.list(Doc::House).expect(name),
                vec![crate::chat::HOUSE.to_string()],
                "{name}: and exactly one after",
            );
            assert_eq!(
                store.read(Doc::House, crate::chat::HOUSE).expect(name).expect("present").0,
                "{\"h\":0}",
                "{name}",
            );
            assert!(
                store.list(Doc::Record).expect(name).is_empty(),
                "{name}: the house is no record",
            );
            assert!(store.list(Doc::Table).expect(name).is_empty(), "{name}: and no table");
            assert!(store.read(Doc::House, "house2").is_err(), "{name}: the house has one id");
            // **And the WRITE site, which is the one that could leave a file
            // behind** (final review, minor 15). A refusal pinned through
            // `read` alone would go on passing against a store that wrote
            // `house2` happily and then declined to read it back — the exact
            // shape `MemStore` had before this rule was hoisted above both
            // implementations, since `is_safe_id("house2")` is perfectly true.
            assert!(
                store.write_if_version(Doc::House, "house2", "{}", None).is_err(),
                "{name}: and a second house is never WRITTEN",
            );
            assert!(
                store.list(Doc::House).expect(name).len() == 1,
                "{name}: the refused write left nothing behind",
            );
            store.write_if_version(Doc::Record, "g", "{\"r\":1}", None).expect(name);
            store.delete("g").expect(name);
            assert!(
                store.read(Doc::House, crate::chat::HOUSE).expect(name).is_some(),
                "{name}: a game's delete leaves the house",
            );
        }
    }

    /// **`.chat` is reserved exactly as `.table` is, case-insensitively.**
    #[test]
    fn an_id_ending_in_chat_is_refused() {
        for bad in ["mygame.chat", "mygame.CHAT", "x.Chat", ".chat"] {
            assert!(!is_safe_id(bad), "{bad:?} would spell another game's chat document");
        }
        for good in ["chat", "chatter", "my.chatty", "a.b"] {
            assert!(is_safe_id(good), "{good:?} is an ordinary name");
        }
    }

    /// **The house's stored name is refused by a NAMED check, tied to the
    /// file it names** (M6.5c2 cycle D, row 130).
    ///
    /// The underscore is the reservation — the first-character rule already
    /// refuses `_house` — and the check pins it by name: `house_stem` is the
    /// one spelling both the check and `FsStore`'s house path read, so an edit
    /// that dropped the underscore would leave the house's stored name still
    /// refused as a game id rather than free for a game to overwrite. The
    /// plain `house` stays an ordinary game id (a game so named must stay
    /// readable), and the house cell above is the other half: the house
    /// document written, listed and read on both stores.
    #[test]
    fn the_house_name_is_reserved_by_a_named_check() {
        assert_eq!(house_stem(), format!("_{}", crate::chat::HOUSE), "an underscore, then the house's id");
        for spelled in [house_stem(), house_stem().to_ascii_uppercase()] {
            assert!(is_the_house(&spelled) && !is_safe_id(&spelled), "{spelled:?} is the house's, never a game's");
        }
        for good in [crate::chat::HOUSE, "House", "houses", "house2", "my-house"] {
            assert!(!is_the_house(good) && is_safe_id(good), "{good:?} is an ordinary game id");
        }
        let dir = std::env::temp_dir().join(format!("yokohama_house_stem_{}", std::process::id()));
        let path = FsStore::new(&dir).path(Doc::House, crate::chat::HOUSE).expect("the house's path");
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(format!("{}{}", house_stem(), Doc::House.suffix()).as_str()),
            "the file IS the stem the check refuses",
        );
    }

    /// Two writers of identical bytes agree, and different bytes do not.
    ///
    /// The property `Games::persist` rests on: a version is a statement about
    /// CONTENT, so "is the store still what I left there?" is answerable
    /// without keeping the bytes.
    #[test]
    fn identical_bytes_from_two_writers_carry_one_version() {
        assert_eq!(version_of("{\"x\":1}"), version_of("{\"x\":1}"));
        assert_ne!(version_of("{\"x\":1}"), version_of("{\"x\":2}"));
    }

    /// The id guard is the only thing between a client-supplied name and the
    /// filesystem, so it is asserted against the shapes that matter rather than
    /// a happy path.
    #[test]
    fn only_safe_ids_are_usable_as_storage_keys() {
        for good in ["a", "game1", "2026-08-30_sim", "A.b-c_d"] {
            assert!(is_safe_id(good), "{good:?} should be usable");
        }
        for bad in [
            "",
            "..",
            "../etc/passwd",
            "a/b",
            "a\\b",
            "-leading-dash",
            "_leading-underscore",
            ".hidden",
            "with space",
            "nul\0byte",
            "sö",
            // **A name ending in `.table` spells another game's TABLE.**
            // `FsStore` gives both documents one filename each in one
            // directory, so a record stored for `mygame.table` is written to
            // exactly the path `mygame`'s table document occupies — and from
            // there `list(Doc::Record)` hides it, and `delete("mygame")`
            // removes it while answering `Ok`. Case-insensitively, because the
            // filesystems this is developed and tested on (APFS, NTFS) are.
            "mygame.table",
            "mygame.TABLE",
            ".table",
        ] {
            assert!(!is_safe_id(bad), "{bad:?} must be refused");
        }
        assert!(!is_safe_id(&"a".repeat(65)), "an unbounded name must be refused");
        assert!(is_safe_id(&"a".repeat(64)));

        // The guard and the documents have to reserve the SAME suffixes, and a
        // constant that merely looks right is how they drift apart. This
        // derives each one from its own arm: change either `Doc`'s suffixes or
        // `RESERVED_ID_SUFFIXES` alone and this fails. A loop rather than one
        // assertion per kind — with the LENGTHS asserted first, because `zip`
        // stops at the shorter one and would otherwise let a third reserved
        // suffix be added to the list and derived by nothing.
        let derived = [Doc::Table, Doc::Chat];
        assert_eq!(
            RESERVED_ID_SUFFIXES.len(),
            derived.len(),
            "every reserved suffix is derived from the document that appends it",
        );
        for (suffix, doc) in RESERVED_ID_SUFFIXES.iter().zip(derived) {
            assert_eq!(
                doc.suffix(),
                format!("{suffix}{}", Doc::Record.suffix()),
                "the id guard reserves exactly what {doc:?} appends",
            );
        }
    }

    /// Every door into the filesystem asks the id guard first — every document
    /// of a game, and the refusal is an `Io` rather than a `Conflict`.
    ///
    /// The distinction is not pedantry: `Games::persist` EVICTS the handle on a
    /// conflict and leaves it alone on an I/O failure, so a refused name
    /// arriving as the wrong variant would drop a live session over a request
    /// that never touched the store. And the sentence is read through
    /// [`WriteError`]'s own `Display`, which is what a caller that does not
    /// destructure the variant sees.
    #[test]
    fn a_refused_id_never_reaches_the_filesystem() {
        let store = FsStore::new(std::env::temp_dir().join("yokohama-web-test-never-created"));
        assert!(store.read(Doc::Record, "../../etc/passwd").is_err());
        assert!(store.read(Doc::Table, "../../etc/passwd").is_err());
        assert!(store.read(Doc::Chat, "../../etc/passwd").is_err());
        for doc in [Doc::Record, Doc::Table, Doc::Chat] {
            let refused = store.write_if_version(doc, "../../etc/passwd", "{}", None);
            match &refused {
                Err(e @ WriteError::Io(_)) => assert!(
                    e.to_string().contains("not a usable game name"),
                    "the refusal names the id guard: {e}",
                ),
                other => panic!("{doc:?}: an unsafe id must be refused, got {other:?}"),
            }
        }
        assert!(store.delete("../../etc/passwd").is_err());
    }

    #[test]
    fn an_absent_directory_is_an_empty_store_rather_than_an_error() {
        let store = FsStore::new(std::env::temp_dir().join("yokohama-web-test-absent-dir"));
        assert_eq!(
            store.list(Doc::Record).expect("an absent dir lists empty"),
            Vec::<GameId>::new(),
        );
        assert_eq!(store.list(Doc::Table).expect("an absent dir lists empty"), Vec::<GameId>::new());
    }

    /// Concurrent writers to ONE id never publish a torn or foreign record.
    ///
    /// The temp path used to be a pure function of the id, so writers shared
    /// one file: A's `File::create` truncated what B had written and A's
    /// `rename` published B's partial bytes as A's record. Measured under six
    /// concurrent creates, that produced an unparseable 869-byte record which
    /// `list` still returned and `create` then refused to replace.
    ///
    /// **What holds this now is two mechanisms, and this cell can only see
    /// one.** `write_if_version` holds the store's lock across the whole
    /// read-compare-write, so within this process the writers are serialized
    /// and cannot interleave at all; the unique temp name is what protects the
    /// same id written from a SECOND PROCESS, which no cell here can arrange.
    /// So this asserts the PROPERTY — every read of the finished file parses,
    /// and its content is exactly one of the values written, never a splice —
    /// rather than proving the mechanism that used to be its only defence.
    ///
    /// Each writer reads the current version and writes against it, which is
    /// how a real caller uses this seam; exactly one of them can win a round,
    /// and the losers must conflict rather than corrupt.
    #[test]
    fn concurrent_writers_never_publish_a_torn_record() {
        let dir = std::env::temp_dir().join(format!("yokohama-web-torn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = Arc::new(FsStore::new(&dir));

        // Long and DIFFERENT lengths, so a splice cannot accidentally equal a
        // whole value: a short write landing inside a longer one is exactly the
        // corruption being ruled out.
        let values: Vec<String> =
            (0..8).map(|i| format!("{{\"w\":{i},\"pad\":\"{}\"}}", "x".repeat(200 + i * 97))).collect();

        for _round in 0..12 {
            let handles: Vec<_> = values
                .iter()
                .cloned()
                .map(|v| {
                    let store = Arc::clone(&store);
                    std::thread::spawn(move || {
                        let expected =
                            store.read(Doc::Record, "g").expect("a read").map(|(_, v)| v);
                        store.write_if_version(Doc::Record, "g", &v, expected)
                    })
                })
                .collect();
            // At least one writer must succeed; a loser is allowed to conflict,
            // but never to leave the record unreadable.
            let ok = handles.into_iter().filter_map(|h| h.join().expect("a writer").ok()).count();
            assert!(ok > 0, "every concurrent writer failed");

            let (got, _) = store
                .read(Doc::Record, "g")
                .expect("the record is readable after concurrent writes")
                .expect("a record was published");
            assert!(
                values.contains(&got),
                "the published record is a splice, not one of the writes: {:?}",
                &got[..got.len().min(120)],
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An entry that exists but cannot be READ is never mistaken for absence.
    ///
    /// `create`'s duplicate guard asks this question, and answering "absent" on
    /// a permission error let a new game overwrite a real one — measured with
    /// `chmod 000`: the POST returned 201 and the original record was gone.
    /// Absence is `NotFound` and nothing else.
    ///
    /// Both halves, because the refusal has to survive the seam: `read` answers
    /// `Err` rather than `Ok(None)`, and a CREATE over that file is an
    /// `Io` error rather than a silent overwrite. The second is the one that
    /// destroys data, and it is now the store's own answer rather than a rule
    /// `Games::create` has to remember.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_entry_is_present_not_absent() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("yokohama-web-eacces-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = FsStore::new(&dir);
        store.write_if_version(Doc::Record, "locked", "{\"seed\":111}", None).expect("a write");

        let path = dir.join("locked.json");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).expect("chmod");
        // Running as root defeats the premise; skip rather than assert a
        // property the environment cannot exhibit.
        if std::fs::read_to_string(&path).is_ok() {
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }

        assert!(store.read(Doc::Record, "locked").is_err(), "the premise: it is unreadable");
        match store.write_if_version(Doc::Record, "locked", "{\"seed\":222}", None) {
            Err(WriteError::Io(_)) => {}
            other => panic!("an unreadable file must not be silently created over, got {other:?}"),
        }
        assert!(
            store.read(Doc::Record, "never-written").expect("absent answers").is_none(),
            "absent is absent",
        );

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("chmod");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
