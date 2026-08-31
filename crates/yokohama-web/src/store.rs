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

use std::path::{Path, PathBuf};

/// A game's name. Also a filename, which is why [`is_safe_id`] exists.
pub type GameId = String;

pub trait SessionStore: Send + Sync {
    /// Every game this store holds, in a stable order.
    fn list(&self) -> Result<Vec<GameId>, String>;
    /// The stored `GameRecord` JSON.
    fn read(&self, id: &str) -> Result<String, String>;
    /// Replace `id`'s record. Must not leave a partial file behind.
    fn write(&self, id: &str, json: &str) -> Result<(), String>;
    fn delete(&self, id: &str) -> Result<(), String>;
}

/// Whether `id` may be used as a storage key.
///
/// An allowlist, and deliberately not a denylist or an escaping pass: `..`,
/// `/`, a leading `-`, a NUL and every encoding trick are excluded by not being
/// on the list rather than by being enumerated. The same shape (and the same
/// reasoning) as `yokohama_bridge::capture::safe_table_id`, which guards the
/// one other place in this repo where an outside string becomes a filename.
pub fn is_safe_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

/// The local implementation: one JSON file per game in a directory.
pub struct FsStore {
    dir: PathBuf,
}

impl FsStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, id: &str) -> Result<PathBuf, String> {
        if !is_safe_id(id) {
            return Err(format!("{id:?} is not a usable game name"));
        }
        Ok(self.dir.join(format!("{id}.json")))
    }
}

impl SessionStore for FsStore {
    fn list(&self) -> Result<Vec<GameId>, String> {
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
                let p = e.path();
                if p.extension().and_then(|s| s.to_str()) != Some("json") {
                    return None;
                }
                let id = p.file_stem()?.to_str()?.to_string();
                is_safe_id(&id).then_some(id)
            })
            .collect();
        ids.sort();
        Ok(ids)
    }

    fn read(&self, id: &str) -> Result<String, String> {
        std::fs::read_to_string(self.path(id)?).map_err(|e| e.to_string())
    }

    /// Atomic write-then-rename with an `fsync`, the same discipline
    /// `Session::save` uses and for the same reason: this rewrites the ONLY
    /// copy of a game after every applied move, so a crash mid-write must not
    /// be able to destroy it. The target is never opened for writing.
    fn write(&self, id: &str, json: &str) -> Result<(), String> {
        use std::io::Write;
        let path = self.path(id)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        {
            let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.path(id)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(format!("no game named {id:?}"))
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

/// An in-memory store, for tests and for a run that must leave no trace.
#[derive(Default)]
pub struct MemStore {
    games: std::sync::Mutex<std::collections::BTreeMap<GameId, String>>,
}

impl SessionStore for MemStore {
    fn list(&self) -> Result<Vec<GameId>, String> {
        Ok(self.games.lock().expect("store lock").keys().cloned().collect())
    }
    fn read(&self, id: &str) -> Result<String, String> {
        self.games
            .lock()
            .expect("store lock")
            .get(id)
            .cloned()
            .ok_or_else(|| format!("no game named {id:?}"))
    }
    fn write(&self, id: &str, json: &str) -> Result<(), String> {
        if !is_safe_id(id) {
            return Err(format!("{id:?} is not a usable game name"));
        }
        self.games.lock().expect("store lock").insert(id.to_string(), json.to_string());
        Ok(())
    }
    fn delete(&self, id: &str) -> Result<(), String> {
        self.games
            .lock()
            .expect("store lock")
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| format!("no game named {id:?}"))
    }
}

/// The default local directory: `~/.yokohama/games`, or `./yokohama-games` when
/// there is no home to speak of.
pub fn default_dir() -> PathBuf {
    std::env::var_os("YOKOHAMA_WEB_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| Path::new(&h).join(".yokohama").join("games")))
        .unwrap_or_else(|| PathBuf::from("yokohama-games"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        ] {
            assert!(!is_safe_id(bad), "{bad:?} must be refused");
        }
        assert!(!is_safe_id(&"a".repeat(65)), "an unbounded name must be refused");
        assert!(is_safe_id(&"a".repeat(64)));
    }

    #[test]
    fn a_refused_id_never_reaches_the_filesystem() {
        let store = FsStore::new(std::env::temp_dir().join("yokohama-web-test-never-created"));
        assert!(store.read("../../etc/passwd").is_err());
        assert!(store.write("../../etc/passwd", "{}").is_err());
        assert!(store.delete("../../etc/passwd").is_err());
    }

    #[test]
    fn an_absent_directory_is_an_empty_store_rather_than_an_error() {
        let store = FsStore::new(std::env::temp_dir().join("yokohama-web-test-absent-dir"));
        assert_eq!(store.list().expect("an absent dir lists empty"), Vec::<GameId>::new());
    }

    #[test]
    fn the_memory_store_round_trips_and_refuses_what_the_fs_store_refuses() {
        let store = MemStore::default();
        assert!(store.read("nope").is_err());
        store.write("g1", "{\"a\":1}").expect("a write");
        store.write("g2", "{}").expect("a write");
        assert_eq!(store.list().unwrap(), vec!["g1".to_string(), "g2".to_string()]);
        assert_eq!(store.read("g1").unwrap(), "{\"a\":1}");
        store.delete("g1").expect("a delete");
        assert!(store.delete("g1").is_err(), "deleting twice is an error, not a silent ok");
        assert!(store.write("../escape", "{}").is_err());
    }
}
