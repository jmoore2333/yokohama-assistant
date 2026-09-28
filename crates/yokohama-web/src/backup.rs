//! The backup envelope and the ONE validator behind it (M6.5b2, R-6.5b2.10 –
//! .12; spec §2.5).
//!
//! **One envelope, one game or all of them**, so there is a single format
//! rather than two, and **one function decides what a valid game is** — the
//! archive route calls [`backup::check_pair`] and so does a restore, because a backup
//! and an upload that disagreed would mean a game you could send to the server
//! but not put back on your own device.
//!
//! ## Why this module is not behind the `server` feature
//!
//! The browser host calls it. `crates/yokohama-wasm` wraps [`backup::restore_report_json`]
//! as its one new export, so this file makes no platform call at all — no
//! clock, no filesystem, no environment — and the layer's own
//! `platform_calls_stay_behind_the_server_feature` scan is what holds that
//! (its test module below is cut out of the scan, which is why the fixture
//! emitter may read and write files).
//!
//! ## The three answers, and why there are three and not two
//!
//! [`backup::Verdict`] separates **rejected** from **unplayable** because refusing a
//! record at another `SEMANTICS_REV` outright would strand the only copy of a
//! game (R-6.5b2.12). A cross-rev pair RESTORES, lists with its reason, and can
//! be exported again; the bytes are never rewritten, which is what "never
//! migrated" means. `Dealing` is the third: a local game still at the deal has
//! a table document and **no record**, so a required `record` would make "Back
//! up everything" either omit those games silently or fail the whole export.
//!
//! ## What is checked HERE and what is checked by the page
//!
//! This module decides whether a game is *valid* and whether it is
//! *addressable* — [`backup::check_pair`] and the id contract in [`backup::restore_read`],
//! which `TableDoc::agrees_with` cannot supply because it compares SETUP fields
//! and knows nothing about the id or who may play. The **clash rule** is the
//! page's, because it is about what is already in this browser's IndexedDB and
//! has to be one transaction with the write (spec §2.5).

use serde::{Deserialize, Serialize};

/// The one `kind` this build writes and reads.
pub const ENVELOPE_KIND: &str = "yokohama-backup";

/// The envelope format's version. A file of another version is refused AS A
/// WHOLE rather than half-read: a reader that skipped fields it did not know
/// would restore some of a newer file's games and silently drop the rest.
pub const ENVELOPE_VERSION: u32 = 1;

/// The namespace a game played in this browser lives in
/// (`web/src/local/ids.ts`'s `LOCAL_PREFIX`). Restated here rather than
/// imported because the client owns the id-drawing and this crate owns the
/// checking; the pair is pinned by the fixture both sides read.
pub const LOCAL_PREFIX: &str = "local-";

/// The one address a local game is ever played by
/// (`web/src/local/ids.ts`'s `LOCAL_USER`, and `crates/yokohama-wasm`'s
/// caller). A restored game whose seats name somebody else would be listed and
/// then refused at every move, which is worse than refusing the import.
pub const LOCAL_USER: &str = "local@browser";

/// A backup file.
///
/// `deny_unknown_fields` for the reason every request body on this API carries
/// it: a key this build does not know is a fact the file is stating, and
/// ignoring it is how a restore quietly does something other than what the file
/// asked for.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub kind: String,
    pub envelope: u32,
    pub created_at: u64,
    pub origin: String,
    pub app_version: String,
    /// **EXPORTER PROVENANCE, not a description of the contents** (spec §2.5).
    /// One backup can hold games from several revisions, so each game carries
    /// its own pair below and this says only what wrote the file. Nothing reads
    /// it for a decision.
    pub semantics_rev: u32,
    pub rules_hash: String,
    pub games: Vec<BackupGame>,
}

/// One game in a backup.
///
/// **`record` is OPTIONAL and `table` is not**: `writeDocs` takes
/// `docs: { record?: string; table: string }` and stores exactly that, so a
/// game still at the deal has a table document and no record.
///
/// **Both are STRINGS and nobody parses them on the way past.** A record's
/// terminal digest is over its exact bytes and its seed is a `u64` that
/// `JSON.parse` rounds — the rule `recordText`, the archive route and the
/// IndexedDB rows already follow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupGame {
    pub id: String,
    #[serde(default)]
    pub record: Option<String>,
    pub table: String,
    /// What the exporting device's COMPATIBILITY ROW said about this game,
    /// carried so a row that will not open can say what it was written at.
    ///
    /// **`null` for every game that opens on the exporting device** (ruling
    /// I-2 of Task 8; whole-branch review, false sentence 5), whatever health
    /// reads that browser has landed: only a game the exporter could not open
    /// has a compat row, and only that row knows a revision this build did not
    /// write. The doc this replaces described the draft the ruling superseded
    /// — the exporter's own revision — which would have stamped every game
    /// with the sender's build.
    #[serde(default)]
    pub semantics_rev: Option<u32>,
    #[serde(default)]
    pub rules_hash: Option<String>,
}

/// What a restore can say about one game.
///
/// Internally tagged, so the wasm export's JSON is `{"kind":"accepted",…}` and
/// the client branches on one field rather than on which keys are present.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    /// A complete game this build can open.
    Accepted { players: u8, events: usize, over: bool },
    /// A game still at the deal: a table document and no record.
    Dealing { players: u8 },
    /// **Kept, listed and downloadable, never migrated or erased**
    /// (R-6.5b2.12). A record at another `SEMANTICS_REV`.
    Unplayable { reason: String },
    /// Not a game this build will write down at all: nothing is stored for it.
    Rejected { reason: String },
}

/// What a valid pair turns out to be. The archive route reads `over` off this
/// to choose its leaf; a restore reads `players`, `events` and `over` into the
/// row it reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairFacts {
    pub players: u8,
    pub events: usize,
    pub over: bool,
    /// **This build's own constant, and it can hold nothing else** (review
    /// M-5): [`check_pair`] returns `Ok` only where the record's revision
    /// EQUALS `SEMANTICS_REV`, so this is never news. It is the caller's
    /// convenience — a row can be reported without reaching for the constant —
    /// and is NOT the record's, which a later reader should not assume.
    pub semantics_rev: u32,
}

/// One row of a restore report, as the module hands it to the page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreRow {
    pub id: String,
    pub verdict: Verdict,
}

/// **THE function the archive route calls**, and the one a restore calls for a
/// game that carries a record (spec §2.5: "that becomes one function two
/// callers use, so a backup and an upload cannot disagree about what a valid
/// game is").
///
/// The checks are `archive_post`'s own, in `archive_post`'s own order and its
/// own words — a `semantics_rev` present and equal to this build's, a terminal
/// digest present, `replay_strict` clean, the table document parsing, passing
/// [`tables::check_shape`](crate::tables) and AGREEING with the record, and the
/// document's id being `is_safe_id`'s, because there it becomes a path
/// component. The ORDER is pinned through the real route by
/// `api::tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`,
/// which was written and run BEFORE these checks moved here.
///
/// **It builds a [`Rules`](yokohama_core::Rules) for the caller that has none.**
/// `AppState`'s own doc states this crate's discipline — "`Rules::standard()` is
/// not cheap and is immutable once built, so it is constructed once per process
/// and shared by reference" — and an earlier draft departed from it, paying one
/// per archive POST and one per record-carrying game in a restore (review M-4).
/// So `check_pair_with` is the body and takes the rules; this is the ONE
/// public entry, for a caller that has no `Games` to borrow them from. The two
/// in-crate callers pass what they already hold: `archive_post` the process's
/// shared rules, `restore_read` one built per ENVELOPE rather than per game.
pub fn check_pair(record: &str, table: &str) -> Result<PairFacts, String> {
    check_pair_with(record, table, &yokohama_core::Rules::standard())
}

/// [`check_pair`]'s body, over rules the caller already holds. One function,
/// two callers — the claim R-6.5b2.10 makes — with the construction of the
/// rules left to whoever is in a position not to repeat it.
pub(crate) fn check_pair_with(
    record: &str,
    table: &str,
    rules: &yokohama_core::Rules,
) -> Result<PairFacts, String> {
    let rec: yokohama_core::GameRecord =
        serde_json::from_str(record).map_err(|e| format!("the record does not parse: {e}"))?;
    match rec.semantics_rev {
        Some(rev) if rev == yokohama_core::SEMANTICS_REV => {}
        // **This sentence's audience is the ARCHIVE ROUTE** (review M-8): a
        // restore never produces it, because `verdict_of` reads the revision
        // before it calls this and answers `Unplayable` instead — refusing a
        // cross-rev record here would strand the only copy of a game. A future
        // caller reaching this function directly from the browser would read a
        // server's words on a device, which is the reason to say so here.
        Some(rev) => {
            return Err(format!(
                "this record is at semantics rev {rev}; this server archives rev {}",
                yokohama_core::SEMANTICS_REV
            ))
        }
        // `replay_strict`'s own sentence for unknown provenance, carried
        // verbatim rather than reworded: the engine is the authority on what a
        // record with no rev is, and it says so in one place.
        //
        // **Returned and never `expect_err`**, which is the ONE place this
        // extraction does not copy `archive_post` byte for byte. There the
        // panic was unreachable and cost nothing; here the same function reads
        // a FILE A PLAYER CHOSE, inside a Worker where a panic leaves the
        // module trapped for the life of the tab (M6.5b close §4, R5). An
        // engine that one day accepted a record with no provenance would take
        // the local host down rather than refuse the file.
        None => {
            return Err(match yokohama_core::replay_strict(&rec, rules) {
                Err(e) => e.message,
                Ok(_) => "the record carries no semantics provenance".to_string(),
            })
        }
    }
    if rec.terminal_digest.is_none() {
        return Err("the record carries no terminal digest".to_string());
    }
    let state = yokohama_core::replay_strict(&rec, rules).map_err(|e| {
        format!("the record does not replay: at event {}, {}", e.event_index, e.message)
    })?;
    let doc: crate::table::TableDoc = serde_json::from_str(table)
        .map_err(|e| format!("the table document does not parse: {e}"))?;
    crate::tables::check_shape(&doc)
        .map_err(|why| format!("the table document is malformed: {why}"))?;
    doc.agrees_with(&rec)
        .map_err(|why| format!("the table document does not agree with the record: {why}"))?;
    if !crate::store::is_safe_id(&doc.id) {
        return Err("the table's id is not a safe name".to_string());
    }
    Ok(PairFacts {
        players: doc.players,
        events: rec.events.len(),
        over: yokohama_core::is_game_over(&state),
        semantics_rev: yokohama_core::SEMANTICS_REV,
    })
}

/// The two fields the id contract needs, read WITHOUT requiring the whole
/// document to parse as a [`TableDoc`](crate::table::TableDoc).
///
/// **A restored cross-rev pair can be foreign on BOTH sides** (spec §2.5), and
/// a document this build cannot fully read is still a document whose id and
/// seats are legible. `seats` is the real [`SeatDoc`](crate::table::SeatDoc)
/// rather than a second statement of that shape.
#[derive(Debug, Deserialize)]
struct Addressable {
    id: String,
    seats: Vec<crate::table::SeatDoc>,
}

/// Just the revision, off a record this build may otherwise be unable to read.
///
/// No `deny_unknown_fields`: it reads ONE key out of a whole record on purpose.
#[derive(Debug, Deserialize)]
struct RecordRev {
    semantics_rev: Option<u32>,
}

/// **An imported game must be addressable as a local one** (spec §2.5).
///
/// `TableDoc::agrees_with` compares setup fields and knows nothing about the id
/// or who may play, so this is what it cannot supply: the id is in the `local-`
/// namespace and passes `is_safe_id`, it matches the id INSIDE the table
/// document, and the game admits `local@browser`. Without all three an accepted
/// import can be missing from "your games" or route its moves to the server.
fn check_addressable(id: &str, table: &str) -> Result<(), String> {
    if !id.starts_with(LOCAL_PREFIX) {
        return Err(format!(
            "{id:?} is not a game this device plays — a local game's name begins {LOCAL_PREFIX:?}"
        ));
    }
    if !crate::store::is_safe_id(id) {
        return Err(format!("{id:?} is not a safe name"));
    }
    let doc: Addressable = serde_json::from_str(table)
        .map_err(|e| format!("the table document does not parse: {e}"))?;
    if doc.id != id {
        return Err(format!("the backup calls this game {id:?} and its table document {:?}", doc.id));
    }
    if !doc.seats.iter().any(
        |s| matches!(s, crate::table::SeatDoc::Human { user, .. } if user == LOCAL_USER),
    ) {
        return Err(format!("no seat at this table is {LOCAL_USER:?}"));
    }
    Ok(())
}

/// The table document alone, for a game still at the deal.
///
/// **It does NOT loosen the archive route's own requirement for a record**
/// (spec §2.5): `check_pair` is untouched and the archive route never reaches
/// this path — a body with no record does not deserialize as an `ArchivePost`
/// at all.
fn check_table_alone(table: &str) -> Result<u8, String> {
    let doc: crate::table::TableDoc = serde_json::from_str(table)
        .map_err(|e| format!("the table document does not parse: {e}"))?;
    crate::tables::check_shape(&doc)
        .map_err(|why| format!("the table document is malformed: {why}"))?;
    Ok(doc.players)
}

fn verdict_of(game: &BackupGame, rules: &yokohama_core::Rules) -> Verdict {
    if let Err(reason) = check_addressable(&game.id, &game.table) {
        return Verdict::Rejected { reason };
    }
    let Some(record) = &game.record else {
        return match check_table_alone(&game.table) {
            Ok(players) => Verdict::Dealing { players },
            Err(reason) => Verdict::Rejected { reason },
        };
    };
    // **The revision is read BEFORE the pair is validated**, which is the whole
    // of R-6.5b2.12: `check_pair` refuses a foreign rev — it is the archive's
    // function and the archive is right to — and a restore that let that
    // refusal through would strand the only copy of the game.
    if let Ok(RecordRev { semantics_rev: Some(rev) }) = serde_json::from_str::<RecordRev>(record) {
        if rev != yokohama_core::SEMANTICS_REV {
            return Verdict::Unplayable {
                reason: format!(
                    "saved at semantics rev {rev}; this app plays rev {}",
                    yokohama_core::SEMANTICS_REV
                ),
            };
        }
    }
    match check_pair_with(record, &game.table, rules) {
        Ok(f) => Verdict::Accepted { players: f.players, events: f.events, over: f.over },
        Err(reason) => Verdict::Rejected { reason },
    }
}

/// Read a backup file and say what each game in it is.
///
/// **The envelope's own shape is checked before any game is**, and a refusal
/// here is about the FILE: nothing is written and no per-game verdict is
/// reported, because a file this build cannot read is not a file it can half
/// apply.
///
/// Duplicate ids are refused here too. The page refuses them as well and does
/// so before it asks — deliberately two statements of one rule, because the
/// page must be able to refuse a file with the module dead, and this function
/// is a public validator that has to be total on its own.
pub fn restore_read(envelope: &str) -> Result<Vec<(String, Verdict)>, String> {
    let env: Envelope = serde_json::from_str(envelope)
        .map_err(|e| format!("this is not a backup envelope this build reads: {e}"))?;
    if env.kind != ENVELOPE_KIND {
        return Err(format!(
            "this file says it is {:?}; a backup envelope says {ENVELOPE_KIND:?}",
            env.kind
        ));
    }
    if env.envelope != ENVELOPE_VERSION {
        return Err(format!(
            "this is envelope version {}; this build reads version {ENVELOPE_VERSION}",
            env.envelope
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for g in &env.games {
        if !seen.insert(g.id.as_str()) {
            return Err(format!("duplicate game {:?} in one backup", g.id));
        }
    }
    // **ONE `Rules` for the whole file** (review M-4), not one per game.
    let rules = yokohama_core::Rules::standard();
    Ok(env.games.iter().map(|g| (g.id.clone(), verdict_of(g, &rules))).collect())
}

/// [`restore_read`] as the JSON the module hands the page: a list of
/// `{"id":…,"verdict":{"kind":…,…}}`.
///
/// The wasm export is a wrapper over this and carries no logic of its own, so
/// everything that could be wrong is reachable from a native test.
pub fn restore_report_json(envelope: &str) -> Result<String, String> {
    let rows: Vec<RestoreRow> = restore_read(envelope)?
        .into_iter()
        .map(|(id, verdict)| RestoreRow { id, verdict })
        .collect();
    serde_json::to_string(&rows).map_err(|e| format!("the report could not be written: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The finished pair a browser really played (`tests/browser_played_pin.rs`
    /// is the other reader). It carries a seed, which is why it lives under
    /// `tests/fixtures/` rather than in `web/src/wire/`.
    const PLAYED_RECORD: &str = include_str!("../tests/fixtures/browser-played-2p.json");
    const PLAYED_TABLE: &str = include_str!("../tests/fixtures/browser-played-2p.table.json");
    const PLAYED_ID: &str = "local-242a5429453c593c";

    fn committed_browser_played_pair() -> (String, String) {
        (PLAYED_RECORD.to_string(), PLAYED_TABLE.to_string())
    }

    /// An envelope over the given `(id, record, table)` triples, with each
    /// table document RE-ADDRESSED to its id — the one edit a fixture is
    /// allowed here, and the same rule `web/src/test/fakeWorker.ts`'s `named`
    /// follows. Without it every row would be refused by the id contract and
    /// the cell would assert a verdict it never reached.
    fn envelope_of(games: &[(&str, &str, &str)]) -> String {
        serde_json::to_string(&Envelope {
            kind: ENVELOPE_KIND.to_string(),
            envelope: ENVELOPE_VERSION,
            created_at: 1_789_000_000,
            origin: "test".to_string(),
            app_version: "0.0.0-test".to_string(),
            semantics_rev: yokohama_core::SEMANTICS_REV,
            rules_hash: "0".to_string(),
            games: games
                .iter()
                .map(|(id, record, table)| BackupGame {
                    id: (*id).to_string(),
                    record: Some((*record).to_string()),
                    table: renamed(table, id),
                    semantics_rev: None,
                    rules_hash: None,
                })
                .collect(),
        })
        .expect("an envelope serializes")
    }

    /// A table document under another id. A one-key edit through `serde_json`,
    /// so it survives a re-cut fixture — and asserted to have changed
    /// something, because the fixtures are pretty-printed and a textual
    /// replacement written without the space matches nothing.
    fn renamed(table: &str, id: &str) -> String {
        let mut v: serde_json::Value = serde_json::from_str(table).expect("a table document");
        v.as_object_mut().expect("an object").insert("id".into(), id.into());
        let out = v.to_string();
        assert_ne!(out, table, "the rename must actually change the document");
        out
    }

    /// R-6.5b2.10: one validator, two callers. A backup and an upload cannot
    /// disagree about what a valid game is, and this cell is what keeps them
    /// from drifting — it calls BOTH and compares the verdict.
    ///
    /// **`archive_would_accept` is a SECOND ENTRY POINT and not evidence the
    /// route is unchanged** — it calls this very function. The route's own
    /// behaviour is pinned by
    /// `api::tests::the_archive_route_keeps_its_statuses_sentences_and_refusal_order`,
    /// which drives `dispatch` and was green before the extraction.
    #[test]
    fn the_archive_and_the_envelope_judge_a_pair_the_same_way() {
        let (record, table) = committed_browser_played_pair();
        assert!(check_pair(&record, &table).is_ok());
        assert!(crate::api::archive_would_accept(&record, &table), "the archive route's own read agrees");

        // **The committed fixtures are PRETTY-PRINTED**: they contain
        // `"players": 2` with a space. A replacement written without it matches
        // nothing, the "mangled" input is the original, and the cell asserts a
        // refusal that never happens. So every mutation asserts it CHANGED its
        // input before it asserts a verdict.
        let mangled = table.replace("\"players\": 2", "\"players\": 3");
        assert_ne!(mangled, table, "the mutation must actually change the fixture");
        assert!(check_pair(&record, &mangled).is_err(), "a table that does not agree with its record");
        assert!(!crate::api::archive_would_accept(&record, &mangled));
    }

    /// R-6.5b2.12: REJECTED and UNPLAYABLE are different answers. Refusing a
    /// foreign-rev record outright would strand the only copy of a game.
    #[test]
    fn a_foreign_rev_record_is_unplayable_and_a_mangled_one_is_rejected() {
        let (record, table) = committed_browser_played_pair();
        let foreign = record.replace("\"semantics_rev\": 7", "\"semantics_rev\": 6");
        assert_ne!(foreign, record, "the mutation must actually change the fixture");
        let env = envelope_of(&[
            ("local-g1", &foreign, &table),
            ("local-g2", "not json at all", &table),
        ]);
        let report = restore_read(&env).expect("the envelope itself is well formed");
        assert!(matches!(report[0].1, Verdict::Unplayable { .. }), "kept, listed, downloadable: {:?}", report[0].1);
        assert!(matches!(report[1].1, Verdict::Rejected { .. }), "not a record at all: {:?}", report[1].1);
    }

    /// **A game may say NOTHING about its own revision**, which is what the
    /// client writes for a game this device can open (review I-2): the browser
    /// knows only what a COMPATIBILITY ROW records, and a game that opens here
    /// has none. Both fields are `Option`, and this is the cell that says the
    /// wire really carries `null` rather than the field being absent by luck.
    #[test]
    fn a_game_that_says_nothing_about_its_revision_reads() {
        let (record, table) = committed_browser_played_pair();
        let env = envelope_of(&[(PLAYED_ID, &record, &table)]);
        assert!(
            env.contains("\"semantics_rev\":null") && env.contains("\"rules_hash\":null"),
            "the wire carries nulls, not absences: {}",
            &env[..env.len().min(400)],
        );
        assert!(
            matches!(restore_read(&env).unwrap()[0].1, Verdict::Accepted { .. }),
            "and they are read back without complaint",
        );
    }

    /// The envelope's own shape is checked before any game is. A future version
    /// is refused as a whole rather than half-read.
    #[test]
    fn an_envelope_of_another_version_is_refused_whole() {
        let original = envelope_of(&[]);
        let env = original.replace("\"envelope\":1", "\"envelope\":2");
        assert_ne!(env, original, "the mutation must actually change the envelope");
        let err = restore_read(&env).expect_err("a version this build does not know");
        assert!(err.contains("envelope"), "{err}");
    }

    /// The other two whole-file refusals, and the one that is not about a
    /// version: an unknown top-level key, a `kind` that is something else, and
    /// two games under one id.
    #[test]
    fn a_file_this_build_cannot_read_is_refused_before_any_game_is_judged() {
        let (record, table) = committed_browser_played_pair();
        let env = envelope_of(&[("local-1", &record, &table)]);

        let extra = env.replace("{\"kind\"", "{\"whats_this\":1,\"kind\"");
        assert_ne!(extra, env, "the mutation must actually change the envelope");
        let err = restore_read(&extra).expect_err("an unknown top-level key");
        assert!(err.contains("whats_this"), "{err}");

        let other = env.replace("\"yokohama-backup\"", "\"something-else\"");
        assert_ne!(other, env, "the mutation must actually change the envelope");
        let err = restore_read(&other).expect_err("another file's kind");
        assert!(err.contains("yokohama-backup"), "{err}");

        let twice = envelope_of(&[("local-1", &record, &table), ("local-1", &record, &table)]);
        let err = restore_read(&twice).expect_err("two games under one id");
        assert!(err.contains("duplicate"), "{err}");
    }

    /// **The id contract, which `TableDoc::agrees_with` cannot supply** (spec
    /// §2.5). Three ways an otherwise valid pair is not a game this device can
    /// address, each a REJECTION and not an unplayable row: an id outside the
    /// local namespace, an id that disagrees with its own document, and a table
    /// nobody on this device sits at.
    ///
    /// The positive control is first, so a rule that refused everything could
    /// not pass this cell.
    #[test]
    fn a_restored_game_must_be_addressable_as_a_local_one() {
        let (record, table) = committed_browser_played_pair();
        let good = envelope_of(&[(PLAYED_ID, &record, &table)]);
        assert!(
            matches!(restore_read(&good).unwrap()[0].1, Verdict::Accepted { players: 2, over: true, .. }),
            "the control: a re-addressed pair under its own id is accepted",
        );

        // `envelope_of` re-addresses the document, so these differ from the
        // control in exactly the thing each names.
        let foreign = envelope_of(&[("sim-abc", &record, &table)]);
        let why = match &restore_read(&foreign).unwrap()[0].1 {
            Verdict::Rejected { reason } => reason.clone(),
            other => panic!("a server id is not a local game: {other:?}"),
        };
        assert!(why.contains("local-"), "{why}");

        // The document names one game and the envelope calls it another.
        let mismatched = serde_json::to_string(&Envelope {
            kind: ENVELOPE_KIND.to_string(),
            envelope: ENVELOPE_VERSION,
            created_at: 0,
            origin: String::new(),
            app_version: String::new(),
            semantics_rev: yokohama_core::SEMANTICS_REV,
            rules_hash: String::new(),
            games: vec![BackupGame {
                id: "local-x".to_string(),
                record: Some(record.clone()),
                table: table.clone(),
                semantics_rev: None,
                rules_hash: None,
            }],
        })
        .unwrap();
        let why = match &restore_read(&mismatched).unwrap()[0].1 {
            Verdict::Rejected { reason } => reason.clone(),
            other => panic!("the id and the document disagree: {other:?}"),
        };
        assert!(why.contains(PLAYED_ID) && why.contains("local-x"), "{why}");

        // Nobody on this device sits at it.
        let theirs = table.replace("\"local@browser\"", "\"red@wire.test\"");
        assert_ne!(theirs, table, "the mutation must actually change the fixture");
        let elsewhere = envelope_of(&[(PLAYED_ID, &record, &theirs)]);
        let why = match &restore_read(&elsewhere).unwrap()[0].1 {
            Verdict::Rejected { reason } => reason.clone(),
            other => panic!("a table nobody here sits at: {other:?}"),
        };
        assert!(why.contains(LOCAL_USER), "{why}");
    }

    /// A game still at the deal has a table document and NO record, and it
    /// validates on the table alone — **without loosening the archive route's
    /// own requirement for one**, which is asserted here rather than asserted
    /// about: the very same document with a record that does not replay is
    /// still rejected.
    #[test]
    fn a_game_at_the_deal_validates_on_its_table_alone() {
        let dealing = dealing_table("local-dealing");
        let env = serde_json::to_string(&Envelope {
            kind: ENVELOPE_KIND.to_string(),
            envelope: ENVELOPE_VERSION,
            created_at: 0,
            origin: String::new(),
            app_version: String::new(),
            semantics_rev: yokohama_core::SEMANTICS_REV,
            rules_hash: String::new(),
            games: vec![
                BackupGame {
                    id: "local-dealing".to_string(),
                    record: None,
                    table: dealing.clone(),
                    semantics_rev: None,
                    rules_hash: None,
                },
                BackupGame {
                    id: "local-dealing2".to_string(),
                    record: Some("not json at all".to_string()),
                    table: renamed(&dealing, "local-dealing2"),
                    semantics_rev: None,
                    rules_hash: None,
                },
            ],
        })
        .unwrap();
        let report = restore_read(&env).unwrap();
        assert_eq!(report[0].1, Verdict::Dealing { players: 2 });
        assert!(matches!(report[1].1, Verdict::Rejected { .. }), "{:?}", report[1].1);
        // And the archive route still wants a record for the same document.
        assert!(!crate::api::archive_would_accept("not json at all", &dealing));
    }

    /// The committed table, re-addressed and put back to the deal: `status`
    /// `dealing` and no turn mirror, which is what a table document looks like
    /// before the first keep finalizes.
    fn dealing_table(id: &str) -> String {
        let mut v: serde_json::Value =
            serde_json::from_str(PLAYED_TABLE).expect("a table document");
        let o = v.as_object_mut().expect("an object");
        o.insert("id".into(), id.into());
        o.insert("status".into(), "dealing".into());
        o.insert("turn".into(), serde_json::Value::Null);
        let out = v.to_string();
        assert_ne!(out, PLAYED_TABLE, "the edit must actually change the document");
        out
    }

    // -----------------------------------------------------------------------
    // The committed envelope fixture
    // -----------------------------------------------------------------------

    /// Where the client's cells read the envelope from.
    ///
    /// **OUTSIDE `web/src/wire/`, deliberately** (spec §5): it carries a
    /// record, a record carries the setup seed, and the corpus is the one
    /// directory every leakage pin scans (`wire_fixtures.rs`'s `corpus()`). It
    /// sits beside `walk.json`, which is there for that same reason.
    const FIXTURE: &str = "../../web/src/wire-script/backup-envelope.json";

    /// The four games the fixture holds, one per [`Verdict`] arm — built from
    /// the committed pair so no payload in it is hand-authored.
    ///
    /// The unplayable one's record is TRUNCATED as well as re-stamped to rev 6:
    /// it is never replayed (the revision is read first), and a second copy of
    /// a 54 KB record in a file the browser test bundle imports buys nothing.
    fn fixture_envelope() -> Envelope {
        let mut foreign: serde_json::Value =
            serde_json::from_str(PLAYED_RECORD).expect("a record");
        {
            let o = foreign.as_object_mut().expect("an object");
            o.insert("semantics_rev".into(), 6.into());
            let events = o.get_mut("events").expect("events").as_array_mut().expect("a list");
            events.truncate(3);
        }
        let foreign = serde_json::to_string_pretty(&foreign).expect("a record");

        Envelope {
            kind: ENVELOPE_KIND.to_string(),
            envelope: ENVELOPE_VERSION,
            // The corpus's own frozen clock, so this file has no reason to
            // change when the day does.
            created_at: 1_789_000_000,
            origin: "wire.test".to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            semantics_rev: yokohama_core::SEMANTICS_REV,
            rules_hash: yokohama_core::Rules::standard().content_hash(),
            games: vec![
                BackupGame {
                    id: PLAYED_ID.to_string(),
                    record: Some(PLAYED_RECORD.to_string()),
                    table: PLAYED_TABLE.to_string(),
                    semantics_rev: Some(yokohama_core::SEMANTICS_REV),
                    rules_hash: Some(yokohama_core::Rules::standard().content_hash()),
                },
                BackupGame {
                    id: "local-at-the-deal".to_string(),
                    record: None,
                    table: dealing_table("local-at-the-deal"),
                    semantics_rev: Some(yokohama_core::SEMANTICS_REV),
                    rules_hash: Some(yokohama_core::Rules::standard().content_hash()),
                },
                BackupGame {
                    id: "local-another-rev".to_string(),
                    record: Some(foreign),
                    table: renamed(PLAYED_TABLE, "local-another-rev"),
                    semantics_rev: Some(6),
                    rules_hash: Some(yokohama_core::Rules::standard().content_hash()),
                },
                BackupGame {
                    id: "local-not-a-record".to_string(),
                    record: Some("this is not a record".to_string()),
                    table: renamed(PLAYED_TABLE, "local-not-a-record"),
                    semantics_rev: None,
                    rules_hash: None,
                },
            ],
        }
    }

    /// **The envelope fixture and its report, emitted and then PINNED.**
    ///
    /// The client's `backup.test.ts` and `Backup.test.tsx` read these two
    /// files, so the page's reader and this validator are held to one document
    /// rather than to two hand-written copies of an idea. `UPDATE_WIRE_FIXTURES=1`
    /// rewrites them, exactly as the corpus is rewritten, and any other run
    /// REFUSES when the committed bytes differ.
    ///
    /// The REPORT is emitted beside the envelope for a reason the envelope
    /// alone does not serve: `web/src/backup.node.test.ts` replays it through
    /// the COMPILED module, so the verdicts the client's fake answers with are
    /// the verdicts this build really gives.
    #[test]
    fn the_committed_envelope_fixture_is_this_builds_own() {
        let env = fixture_envelope();
        let text = serde_json::to_string_pretty(&env).expect("an envelope") + "\n";
        let report = restore_report_json(&text).expect("the fixture is a readable envelope");
        let report_pretty = serde_json::to_string_pretty(
            &serde_json::from_str::<serde_json::Value>(&report).expect("the report is JSON"),
        )
        .expect("a report")
            + "\n";

        // The four arms, by construction — an anti-vacuity partner for the
        // byte compare below, which would pass just as well over a fixture
        // whose every game was rejected.
        let rows = restore_read(&text).expect("a readable envelope");
        assert!(matches!(rows[0].1, Verdict::Accepted { players: 2, over: true, .. }), "{:?}", rows[0].1);
        assert_eq!(rows[1].1, Verdict::Dealing { players: 2 });
        assert!(matches!(rows[2].1, Verdict::Unplayable { .. }), "{:?}", rows[2].1);
        assert!(matches!(rows[3].1, Verdict::Rejected { .. }), "{:?}", rows[3].1);

        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let envelope_path = dir.join(FIXTURE);
        let report_path = dir.join("../../web/src/wire-script/backup-report.json");
        if std::env::var("UPDATE_WIRE_FIXTURES").is_ok() {
            std::fs::write(&envelope_path, &text).expect("the fixture is writable");
            std::fs::write(&report_path, &report_pretty).expect("the report is writable");
            return;
        }
        let committed = std::fs::read_to_string(&envelope_path).unwrap_or_else(|e| {
            panic!("{}: {e} — regenerate with UPDATE_WIRE_FIXTURES=1", envelope_path.display())
        });
        assert_eq!(
            committed, text,
            "the committed envelope fixture is not what this build emits — \
             regenerate with `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --lib backup` \
             and READ the diff",
        );
        let committed_report = std::fs::read_to_string(&report_path).unwrap_or_else(|e| {
            panic!("{}: {e} — regenerate with UPDATE_WIRE_FIXTURES=1", report_path.display())
        });
        assert_eq!(
            committed_report, report_pretty,
            "the committed restore report is not what this build gives for the committed envelope",
        );
    }
}
