//! **The self-host's sweep** (M6.5c2 cycle F, R-F9, R-F10; spec §2.4): the
//! axum host ends idle tables under the Worker's two TTL names and meanings,
//! through the creator's own delete, listing what it will end before it ends
//! any.
//!
//! **Two launch variables, the Worker's** (`worker/src/config.ts`):
//! [`TABLE_VAR`], whole days above zero a table stands idle before it expires —
//! unset, no sweep runs, the behaviour every NAS deployment before this cycle
//! had; and [`OPEN_VAR`], whole days above zero, default
//! [`OPEN_DEFAULT_DAYS`], inert without the first, for a table whose status is
//! still `open` (a seat unfilled). A `dealing` table waits the first, as on the
//! Worker (ruling 9-b). A bad value stops the server naming the variable, and a
//! table TTL with no archive directory stops it naming both ([`check_archive`]):
//! a sweep that discards games is not a configuration this host offers —
//! stricter than the Worker, whose marker can say "this server keeps no
//! archive", by choice.
//!
//! **Activity** is [`Store::last_written`](crate::store::Store::last_written):
//! the latest write among the game's three documents — the record, the table
//! and the chat — the three the Worker folds its `written_at` stamps over. A
//! table whose activity the store cannot tell (`Ok(None)`, the trait's
//! default) is never a candidate.
//!
//! **A pass** ([`pass`]) reads every table document and its activity, decides
//! each through ONE pure function ([`ending`], the Worker's `expiryReason`),
//! logs a [`crate::log::SWEEP_LISTED`] line for every table it is about to end
//! BEFORE it ends any, and then ends each through [`Games::delete`] as the
//! table's creator — the creator's own Abandon — so a finished game already
//! stamped is never put twice, a mid-play game lands under `abandoned/`, an
//! open table with no game archives nothing, and a failed archive put REFUSES
//! the delete and keeps the table for the next pass.
//!
//! **No marker** (R-F10): a removed table answers "no such game", as an Abandon
//! leaves it on this host; the log line is the record. The Worker's marker is
//! its own.
//!
//! **A race, named** (spec §2.4, §9 F-25): the pass decides from the activity
//! it read and then calls the delete, which re-checks the seat and the creator
//! but not idleness. A move that lands in the seconds between is played, and
//! the delete then files the game WITH it under `abandoned/`; a move whose save
//! lands once the delete has tombstoned the table is refused out loud ("…was
//! deleted while this move was in flight"). The archive keeps the game either
//! way.
//!
//! **Where it lives** (spec §2.4, §9 F-11): everything above the gated region
//! makes no platform call — the decision is pure, and a pass reaches the disk
//! only through the store — so it is tested without a runtime and builds for
//! the browser host too. The task that runs a pass every hour, [`run`], is the one
//! `server`-only item, in ONE gated region at the end of the code, closed by
//! its marker, because the crate's platform-call scan gates only the column-0
//! item after each attribute.

use crate::games::Games;
use crate::identity::User;
use crate::log;
use crate::store::Doc;
use crate::table::{TableDoc, TableStatus};

/// The table TTL's variable: whole days above zero a table stands idle before
/// it expires. Unset, no table expires and no sweep runs.
pub const TABLE_VAR: &str = "YOKOHAMA_WEB_TABLE_TTL_DAYS";
/// The waiting-table TTL's variable: whole days above zero a table whose
/// status is `open` stands idle before it is closed; inert without
/// [`TABLE_VAR`].
pub const OPEN_VAR: &str = "YOKOHAMA_WEB_OPEN_TABLE_TTL_DAYS";
/// A waiting table's TTL when [`OPEN_VAR`] is absent — the Worker's.
pub const OPEN_DEFAULT_DAYS: u64 = 7;
/// One day, in seconds: the unit both TTLs are counted in.
pub const DAY: u64 = 86_400;
/// The archive directory's variable, spelled here for [`check_archive`]'s
/// sentence: the archive's own constant sits in its `server`-only module.
const ARCHIVE_VAR: &str = "YOKOHAMA_WEB_ARCHIVE_DIR";
/// The largest whole number of days either variable admits: the Worker's
/// `Number.isSafeInteger` bound, so one value reads one way on both hosts.
const MAX_DAYS: u64 = (1 << 53) - 1;

/// The host's two TTLs, in whole days.
///
/// `table` is `None` where no table expires — no sweep runs — and `open` is
/// then `None` too, so `/api/health` reports both `null`, as the Worker does
/// (`worker/src/config.ts`'s `healthTtls`). With `table` set, `open` is the
/// waiting-table TTL, [`OPEN_DEFAULT_DAYS`] unless its variable says otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ttls {
    pub table: Option<u64>,
    pub open: Option<u64>,
}

impl Ttls {
    /// No TTL: every table stands until its creator abandons it.
    pub fn none() -> Self {
        Self { table: None, open: None }
    }

    /// The two variables read through `lookup` — `main.rs` hands it the
    /// process environment; a cell, a list. Pure: this function asks nothing
    /// but `lookup`.
    ///
    /// Each value is trimmed, then must be whole ASCII digits above zero and at
    /// most 2^53 − 1, the Worker's `wholeDays` rule; anything else is the `Err`
    /// sentence naming the variable, which `main` exits 1 on. [`OPEN_VAR`] is
    /// read whether or not [`TABLE_VAR`] is set, so a bad value is refused
    /// before it could ever be in force — and then dropped when the first is
    /// unset, since it is inert alone.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let table = lookup(TABLE_VAR).map(|raw| whole_days(TABLE_VAR, &raw)).transpose()?;
        let open = lookup(OPEN_VAR).map(|raw| whole_days(OPEN_VAR, &raw)).transpose()?;
        Ok(Self { table, open: table.map(|_| open.unwrap_or(OPEN_DEFAULT_DAYS)) })
    }
}

/// `raw`, trimmed, as whole days above zero — or the refusal naming `name`.
/// The two TTLs' one rule, the Worker's: a sign, a fraction, an exponent, zero
/// or a value past 2^53 − 1 is refused, where `str::parse` alone would take
/// `+5`.
fn whole_days(name: &str, raw: &str) -> Result<u64, String> {
    let value = raw.trim();
    let digits = !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit());
    match value.parse::<u64>() {
        Ok(days) if digits && (1..=MAX_DAYS).contains(&days) => Ok(days),
        _ => Err(format!("{name} must be a whole number of days above zero — got {value:?}")),
    }
}

/// How a pass ends a table, and how many whole days it had stood idle
/// (floored). The reason is the Worker's word: `waited` for a table still
/// waiting for players, `expired` for any other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ending {
    Expired { days: u64 },
    Waited { days: u64 },
}

impl Ending {
    /// The reason a log line carries: `expired` or `waited`.
    pub fn reason(self) -> &'static str {
        match self {
            Self::Expired { .. } => "expired",
            Self::Waited { .. } => "waited",
        }
    }

    /// The whole days the table had stood idle.
    pub fn days(self) -> u64 {
        match self {
            Self::Expired { days } | Self::Waited { days } => days,
        }
    }
}

/// **The decision** — the Worker's `expiryReason` (`worker/src/marker.ts`),
/// restated: with no table TTL, nothing ends; a table whose status is `open`
/// is closed as one that WAITED once its last activity is the open TTL old
/// ([`OPEN_DEFAULT_DAYS`] when `ttls.open` is unset), whatever else is past;
/// any table EXPIRES once it is the table TTL old — an open one too, where
/// that TTL is the shorter. `dealing` and `playing` wait the table TTL.
///
/// `last_active` and `now` are unix seconds. Nothing wraps: an activity after
/// `now` is idle zero days, and a deadline past the range of a `u64` is never
/// reached.
pub fn ending(status: TableStatus, last_active: u64, now: u64, ttls: &Ttls) -> Option<Ending> {
    let table_days = ttls.table?;
    let open_days = ttls.open.unwrap_or(OPEN_DEFAULT_DAYS);
    let days = now.saturating_sub(last_active) / DAY;
    let past = |ttl: u64| {
        ttl.checked_mul(DAY).and_then(|span| last_active.checked_add(span)).is_some_and(|deadline| now >= deadline)
    };
    if status == TableStatus::Open && past(open_days) {
        return Some(Ending::Waited { days });
    }
    past(table_days).then_some(Ending::Expired { days })
}

/// **A table TTL with no archive directory refuses to start** (R-F9): the
/// sentence `main` exits 1 on, naming both variables. `AppState::new` asks it
/// once the archive is resolved and before the store is built. The waiting
/// TTL alone is inert and needs nothing.
pub fn check_archive(ttls: &Ttls, archive_on: bool) -> Result<(), String> {
    if ttls.table.is_some() && !archive_on {
        return Err(format!(
            "{TABLE_VAR} is set but {ARCHIVE_VAR} is not — a sweep without an archive throws games away; set both or \
             neither"
        ));
    }
    Ok(())
}

/// **One pass**: list, then end (R-F9; spec §2.4's steps 1–4). Every table
/// document the store lists is read with its activity and decided by
/// [`ending`]; each table to end is logged [`log::SWEEP_LISTED`] BEFORE any
/// is ended; then each is ended through [`Games::delete`] as its creator
/// (`TableDoc::created_by`) and logged [`log::SWEEP_ENDED`] with what was
/// kept, or [`log::SWEEP_FAILED`] with the delete's sentence — a failed
/// archive put among them, which keeps the table for the next pass. Answers
/// each table it tried to end, in listing order, with the delete's result.
///
/// With no table TTL it reads nothing. A table whose activity the store
/// cannot tell (`Ok(None)`) is never a candidate; one whose document or
/// activity cannot be read is logged [`log::SWEEP_UNREAD`] and is not a
/// candidate this pass. The house is no table: `list(Doc::Table)` cannot name
/// it. It makes no platform call of its own — `now` is the caller's, the disk
/// is the store's — so a cell runs it without a runtime.
pub fn pass(games: &Games, ttls: &Ttls, now: u64) -> Vec<(String, Ending, Result<(), String>)> {
    if ttls.table.is_none() {
        return Vec::new();
    }
    let ids = match games.store().list(Doc::Table) {
        Ok(ids) => ids,
        Err(e) => {
            games.log.line(log::SWEEP_UNREAD, serde_json::json!({ "error": e }));
            return Vec::new();
        }
    };
    let unread = |id: &str, error: String| {
        games.log.line(log::SWEEP_UNREAD, serde_json::json!({ "id": id, "error": error }));
    };
    let mut due: Vec<(String, TableDoc, Ending)> = Vec::new();
    for id in ids {
        let doc = match games.read_table(&id) {
            Ok(Some((doc, _))) => doc,
            // Ended since the listing, by its creator or another writer.
            Ok(None) => continue,
            Err(e) => {
                unread(&id, e.to_string());
                continue;
            }
        };
        let last_active = match games.store().last_written(&id) {
            Ok(Some(at)) => at,
            // Activity unknown: never a candidate (R-FP6).
            Ok(None) => continue,
            Err(e) => {
                unread(&id, e);
                continue;
            }
        };
        if let Some(end) = ending(doc.status, last_active, now, ttls) {
            games.log.line(
                log::SWEEP_LISTED,
                serde_json::json!({
                    "id": id,
                    "status": status_word(doc.status),
                    "idle_days": end.days().to_string(),
                    "reason": end.reason(),
                }),
            );
            due.push((id, doc, end));
        }
    }
    // Every listing is out before the first delete (Review Focus 3).
    due.into_iter()
        .map(|(id, doc, end)| {
            let kept = kept(games, &id, &doc);
            let result = User::parse(&doc.created_by)
                .and_then(|creator| games.delete(&id, &creator).map_err(|e| e.to_string()));
            match &result {
                Ok(()) => games.log.line(
                    log::SWEEP_ENDED,
                    serde_json::json!({ "id": id, "reason": end.reason(), "kept": kept }),
                ),
                Err(e) => games.log.line(
                    log::SWEEP_FAILED,
                    serde_json::json!({ "id": id, "reason": end.reason(), "error": e }),
                ),
            }
            (id, end, result)
        })
        .collect()
}

/// A status as the table document spells it.
fn status_word(status: TableStatus) -> &'static str {
    match status {
        TableStatus::Open => "open",
        TableStatus::Dealing => "dealing",
        TableStatus::Playing => "playing",
    }
}

/// What the creator's delete will keep of this table, in the Worker's words
/// (`worker/src/marker.ts`'s `keptOf`): `archive` when the table is already
/// stamped — the archive holds it from game over — or holds a game the delete
/// files; `nothing` when there is no game (an open, dealing or moveless
/// table); `off` for a game with no archive to keep it.
///
/// "A game" is asked of the RECORD, never the mirror's `has_events` cache, as
/// the delete asks it (`tables::saves_on_abandon`); a record that cannot be
/// read or parsed is counted a game, and then the delete, which reads it too,
/// refuses and the line is `sweep-failed`. Read just before the delete, so a
/// first move landing in between can make `nothing` a game the delete files —
/// the race the module doc names.
fn kept(games: &Games, id: &str, doc: &TableDoc) -> &'static str {
    if doc.archived_at.is_some() {
        return "archive";
    }
    let has_events = match games.read_document(Doc::Record, id) {
        Ok(Some(json)) => {
            serde_json::from_str::<yokohama_core::GameRecord>(&json).map_or(true, |r| !r.events.is_empty())
        }
        Ok(None) => false,
        Err(_) => true,
    };
    if doc.status != TableStatus::Playing || !has_events {
        "nothing"
    } else if games.archive_on() {
        "archive"
    } else {
        "off"
    }
}

/// **The sweep's task** (R-F9; spec §2.4): one [`pass`] after `first_after`,
/// then one every `every`, each at the wall clock's unix seconds — the one
/// platform read the sweep makes, and the reason this item is `server`-only.
/// `main` spawns it beside the server when a table TTL is set, with an hour
/// and a minute. A pass blocks its worker thread as a request does: the layer
/// is synchronous, and a pass is a handful of reads and deletes an hour.
#[cfg(feature = "server")]
pub async fn run(
    state: std::sync::Arc<crate::AppState>,
    ttls: Ttls,
    every: std::time::Duration,
    first_after: std::time::Duration,
) {
    tokio::time::sleep(first_after).await;
    loop {
        pass(&state.games, &ttls, crate::table::now_unix());
        tokio::time::sleep(every).await;
    }
}
// ---- end server ----

#[cfg(test)]
mod tests {
    use super::*;

    /// The Worker's production pair (`worker/wrangler.jsonc`): 90 and 7.
    const WORKER: Ttls = Ttls { table: Some(90), open: Some(7) };
    /// A fixed "now", in unix seconds — any value well above the TTLs works.
    const NOW: u64 = 1_800_000_000;

    /// A lookup over `pairs`, the shape `main.rs` hands the environment as.
    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |name| pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    /// **The two variables read as the Worker reads them** (`worker/src/config.ts`'s
    /// `wholeDays`): unset is no sweep; the table TTL set gives the waiting TTL
    /// its default of 7 or its own value; the waiting TTL alone is inert —
    /// `none()` — but is still READ, so a bad value is refused before it could
    /// ever be in force. A value is trimmed first.
    /// Mutants: the default 7 dropped or moved; the open TTL reported while the
    /// table TTL is unset; the open TTL left unread when the first is unset.
    #[test]
    fn the_two_variables_read_as_the_worker_reads_them() {
        assert_eq!(Ttls::from_lookup(env(&[])), Ok(Ttls::none()));
        assert_eq!(Ttls::none(), Ttls { table: None, open: None });
        assert_eq!(Ttls::from_lookup(env(&[(TABLE_VAR, "90")])), Ok(WORKER), "the open TTL defaults to 7");
        assert_eq!(
            Ttls::from_lookup(env(&[(TABLE_VAR, " 30 "), (OPEN_VAR, "\t3\n")])),
            Ok(Ttls { table: Some(30), open: Some(3) }),
            "trimmed, as the Worker trims",
        );
        assert_eq!(Ttls::from_lookup(env(&[(OPEN_VAR, "3")])), Ok(Ttls::none()), "inert without the first");
        assert_eq!(
            Ttls::from_lookup(env(&[(OPEN_VAR, "x")])),
            Err(format!("{OPEN_VAR} must be a whole number of days above zero — got \"x\"")),
            "read whether or not the first is set",
        );
        let largest = ((1u64 << 53) - 1).to_string();
        assert_eq!(
            Ttls::from_lookup(env(&[(TABLE_VAR, &largest)])).map(|t| t.table),
            Ok(Some((1u64 << 53) - 1)),
            "the Worker's largest safe integer is a whole number of days",
        );
    }

    /// **A bad value stops the server naming the variable**, in the Worker's
    /// sentence (`wholeDays`): whole ASCII digits, above zero, at most 2^53 − 1
    /// (the Worker's `Number.isSafeInteger`), the trimmed value quoted. An empty
    /// value is SET, and refused, as the Worker refuses it.
    /// Mutants: `0` admitted; a sign or a fraction admitted (`parse` alone takes
    /// `+5`); 2^53 admitted; the untrimmed value quoted; the other variable named.
    #[test]
    fn a_bad_value_is_refused_naming_its_variable() {
        for name in [TABLE_VAR, OPEN_VAR] {
            for (raw, shown) in [
                ("x", "x"),
                ("0", "0"),
                ("", ""),
                ("  ", ""),
                ("-1", "-1"),
                ("+5", "+5"),
                ("1.5", "1.5"),
                (" 2e3 ", "2e3"),
                ("9007199254740992", "9007199254740992"),
                ("99999999999999999999999", "99999999999999999999999"),
            ] {
                let pairs = if name == TABLE_VAR { vec![(TABLE_VAR, raw)] } else { vec![(TABLE_VAR, "90"), (OPEN_VAR, raw)] };
                assert_eq!(
                    Ttls::from_lookup(env(&pairs)),
                    Err(format!("{name} must be a whole number of days above zero — got \"{shown}\"")),
                    "{name}={raw:?}",
                );
            }
        }
        assert_eq!(
            Ttls::from_lookup(env(&[(TABLE_VAR, "x")])),
            Err("YOKOHAMA_WEB_TABLE_TTL_DAYS must be a whole number of days above zero — got \"x\"".to_string()),
            "the sentence, verbatim (the self-host guide quotes it)",
        );
    }

    /// **The decision is the Worker's `expiryReason`** (`worker/src/marker.ts`):
    /// no table TTL, nothing ends; a `playing` table expires at exactly the table
    /// TTL and not a second before; the days an ending carries are the whole
    /// idle days, floored.
    /// Mutants: `>` for `>=`; the idle days rounded up; a table TTL of `None`
    /// read as zero.
    #[test]
    fn a_table_expires_at_the_table_ttl_to_the_second() {
        let at = |idle: u64| ending(TableStatus::Playing, NOW - idle, NOW, &WORKER);
        assert_eq!(at(90 * DAY), Some(Ending::Expired { days: 90 }));
        assert_eq!(at(90 * DAY - 1), None, "one second short of the day boundary");
        assert_eq!(at(90 * DAY + DAY - 1), Some(Ending::Expired { days: 90 }), "floored");
        assert_eq!(at(400 * DAY), Some(Ending::Expired { days: 400 }));
        for status in [TableStatus::Open, TableStatus::Dealing, TableStatus::Playing] {
            assert_eq!(ending(status, 0, u64::MAX, &Ttls::none()), None, "{status:?}: no TTL, no ending");
            assert_eq!(
                ending(status, 0, u64::MAX, &Ttls { table: None, open: Some(1) }),
                None,
                "{status:?}: the open TTL is inert without the first",
            );
        }
    }

    /// **`open` waits the shorter TTL and says it WAITED; `dealing` and
    /// `playing` wait the table TTL** (R-6.5c2.42; ruling 9-b: a dealing table
    /// has every seat taken). An open table past both TTLs is `Waited`, as the
    /// Worker's first arm reads; where the open TTL is the LONGER, the table TTL
    /// ends it first, as `Expired`.
    /// Mutants: `dealing` waiting the open TTL; `open` waiting the table TTL
    /// (battery row 16); the arms' order swapped; the open default not 7.
    #[test]
    fn an_open_table_waits_the_shorter_ttl_and_a_dealing_one_the_table_ttl() {
        let at = |status, idle: u64, ttls: &Ttls| ending(status, NOW - idle, NOW, ttls);
        assert_eq!(at(TableStatus::Open, 7 * DAY, &WORKER), Some(Ending::Waited { days: 7 }));
        assert_eq!(at(TableStatus::Open, 7 * DAY - 1, &WORKER), None, "one second short");
        assert_eq!(at(TableStatus::Open, 100 * DAY, &WORKER), Some(Ending::Waited { days: 100 }), "past both: waited");
        assert_eq!(at(TableStatus::Dealing, 8 * DAY, &WORKER), None, "a dealing table is not waiting for players");
        assert_eq!(at(TableStatus::Dealing, 90 * DAY, &WORKER), Some(Ending::Expired { days: 90 }));
        assert_eq!(at(TableStatus::Playing, 8 * DAY, &WORKER), None);
        let longer_open = Ttls { table: Some(10), open: Some(30) };
        assert_eq!(at(TableStatus::Open, 10 * DAY, &longer_open), Some(Ending::Expired { days: 10 }), "the shorter wins");
        assert_eq!(at(TableStatus::Open, 30 * DAY, &longer_open), Some(Ending::Waited { days: 30 }));
        let unset_open = Ttls { table: Some(90), open: None };
        assert_eq!(at(TableStatus::Open, 7 * DAY, &unset_open), Some(Ending::Waited { days: 7 }), "the default is 7");
        assert_eq!(at(TableStatus::Open, 7 * DAY - 1, &unset_open), None);
    }

    /// **The arithmetic cannot wrap**: an activity stamped after `now` (a clock
    /// that stepped back, a copied file) is idle zero days, and the largest TTL
    /// the variable admits never overflows a deadline into the past.
    /// Mutants: an unchecked `now - last_active`; an unchecked `days * DAY`.
    #[test]
    fn the_arithmetic_cannot_wrap() {
        assert_eq!(ending(TableStatus::Playing, NOW + 5, NOW, &WORKER), None);
        let largest = Ttls { table: Some((1 << 53) - 1), open: Some((1 << 53) - 1) };
        for status in [TableStatus::Open, TableStatus::Dealing, TableStatus::Playing] {
            assert_eq!(ending(status, NOW, NOW, &largest), None, "{status:?}");
            assert_eq!(ending(status, u64::MAX, u64::MAX, &largest), None, "{status:?}");
        }
        assert_eq!(ending(TableStatus::Playing, 0, u64::MAX, &WORKER), Some(Ending::Expired { days: u64::MAX / DAY }));
    }

    /// **A TTL with no archive directory refuses to start** (R-F9; Review Focus
    /// 3; battery row 15), naming both variables — a sweep that discards games
    /// is not a configuration the self-host offers. The waiting TTL alone is
    /// inert, so it needs no archive.
    /// Mutants: the check dropped; `open` read instead of `table`; the sentence
    /// reworded.
    #[test]
    fn a_ttl_without_an_archive_refuses_to_start() {
        assert_eq!(
            check_archive(&WORKER, false),
            Err("YOKOHAMA_WEB_TABLE_TTL_DAYS is set but YOKOHAMA_WEB_ARCHIVE_DIR is not — a sweep without an archive \
                 throws games away; set both or neither"
                .to_string()),
        );
        assert_eq!(check_archive(&WORKER, true), Ok(()));
        assert_eq!(check_archive(&Ttls::none(), false), Ok(()));
        assert_eq!(check_archive(&Ttls::none(), true), Ok(()));
        assert_eq!(check_archive(&Ttls { table: None, open: Some(3) }, false), Ok(()), "inert alone");
    }

    // ------------------------------------------------------------------
    // The pass, over a `Games` on a `MemStore` whose activity a cell stamps
    // ------------------------------------------------------------------

    use std::sync::Arc;

    use crate::archive::{Archive, Why};
    use crate::games::test_support::{one_move_in, play_to_the_end, playing, req, two_people, who, RecordingArchive, RecordingLog, CORPUS_SEED};
    use crate::store::{MemStore, Store};
    use crate::tables::SeatSpec;
    use yokohama_core::Rules;

    /// The layer a pass runs over: a `MemStore` the cell stamps, a frozen
    /// clock at [`NOW`], `archive`, and a log that remembers its lines.
    fn layer(archive: Arc<dyn Archive>) -> (Games, Arc<MemStore>, Arc<RecordingLog>) {
        let store = Arc::new(MemStore::default());
        let log = RecordingLog::recording();
        let clock: crate::games::Clock = Arc::new(|| NOW);
        let games = Games::new(Arc::new(Rules::standard()), store.clone() as Arc<dyn Store>)
            .with_clock(clock)
            .with_archive(archive)
            .with_log(log.clone());
        (games, store, log)
    }

    /// A table still waiting for players: red at seat 0, seat 1 open.
    fn open_table(games: &Games, id: &str) {
        let mut r = req(id);
        r.seats[1] = SeatSpec { user: None, bot: false, open: true };
        games.create(&r, &who("red@t.test")).expect("an open table");
    }

    /// The fields of every line logged since `from`, with their events.
    fn lines_since(log: &RecordingLog, from: usize) -> Vec<(&'static str, serde_json::Value)> {
        log.lines.lock().unwrap()[from..].to_vec()
    }

    /// **A pass LISTS every table it will end before it ends ANY** (R-F9;
    /// Review Focus 3; battery row 14) — forty idle tables, forty
    /// `sweep-listed` lines, then forty `sweep-ended` — and ends each through
    /// the creator's delete, as spec §2.4 lays the three kinds out: the
    /// finished game, stamped at game over, is not put twice; the mid-play
    /// game lands under `abandoned/` with its moves; an open table with no
    /// game archives nothing; a table one day idle is not touched.
    /// Mutants: a delete inside the listing loop; the creator's delete
    /// replaced by a bare store delete (the mid-play game unarchived); the
    /// `kept` word read off the mirror; a fresh table listed.
    #[test]
    fn a_pass_lists_every_table_it_will_end_before_it_ends_any() {
        let archive = RecordingArchive::recording();
        let (games, store, log) = layer(archive.clone());
        play_to_the_end(&games, "over");
        assert_eq!(archive.puts.lock().unwrap().len(), 1, "premise: the finished game was put at game over");
        let mid = one_move_in(&games, "mid");
        let moves = mid.session.record().events.len();
        open_table(&games, "open");
        one_move_in(&games, "fresh");
        let waiting: Vec<String> = (0..37).map(|n| format!("idle-{n:02}")).collect();
        for id in &waiting {
            open_table(&games, id);
            store.stamp_for_test(id, NOW - 8 * DAY);
        }
        store.stamp_for_test("over", NOW - 91 * DAY);
        store.stamp_for_test("mid", NOW - 91 * DAY);
        store.stamp_for_test("open", NOW - 8 * DAY);
        store.stamp_for_test("fresh", NOW - DAY);

        let from = log.events().len();
        let ended = pass(&games, &WORKER, NOW);

        assert_eq!(ended.len(), 40, "{ended:?}");
        assert!(ended.iter().all(|(_, _, r)| r.is_ok()), "{ended:?}");
        let lines = lines_since(&log, from);
        let events: Vec<&str> = lines.iter().map(|(e, _)| *e).collect();
        let mut listed_then_ended = vec![log::SWEEP_LISTED; 40];
        listed_then_ended.extend(vec![log::SWEEP_ENDED; 40]);
        assert_eq!(events, listed_then_ended, "every listing before any ending");
        let line = |event: &str, id: &str| {
            lines
                .iter()
                .find(|(e, f)| *e == event && f["id"] == id)
                .map(|(_, f)| f.clone())
                .unwrap_or_else(|| panic!("no {event} line for {id}"))
        };
        let listed = |id: &str, status: &str, days: &str, reason: &str| {
            serde_json::json!({ "id": id, "status": status, "idle_days": days, "reason": reason })
        };
        assert_eq!(line(log::SWEEP_LISTED, "over"), listed("over", "playing", "91", "expired"));
        assert_eq!(line(log::SWEEP_LISTED, "mid"), listed("mid", "playing", "91", "expired"));
        assert_eq!(line(log::SWEEP_LISTED, "open"), listed("open", "open", "8", "waited"));
        let kept = |id: &str, reason: &str, kept: &str| serde_json::json!({ "id": id, "reason": reason, "kept": kept });
        assert_eq!(line(log::SWEEP_ENDED, "over"), kept("over", "expired", "archive"), "stamped at game over");
        assert_eq!(line(log::SWEEP_ENDED, "mid"), kept("mid", "expired", "archive"), "filed by this delete");
        assert_eq!(line(log::SWEEP_ENDED, "open"), kept("open", "waited", "nothing"), "no game");
        assert!(!lines.iter().any(|(_, f)| f["id"] == "fresh"), "a table one day idle is not a candidate");
        // The lines as the NAS's log prints them — the self-host guide quotes
        // these shapes (`deploy/README.md` §8).
        assert_eq!(
            log::render(log::SWEEP_LISTED, &line(log::SWEEP_LISTED, "mid")),
            r#"{"event":"sweep-listed","id":"mid","idle_days":"91","reason":"expired","status":"playing"}"#,
        );
        assert_eq!(
            log::render(log::SWEEP_ENDED, &line(log::SWEEP_ENDED, "mid")),
            r#"{"event":"sweep-ended","id":"mid","kept":"archive","reason":"expired"}"#,
        );
        assert!(ended.contains(&("open".to_string(), Ending::Waited { days: 8 }, Ok(()))));
        assert!(ended.contains(&("mid".to_string(), Ending::Expired { days: 91 }, Ok(()))));

        let puts = archive.puts.lock().unwrap();
        let filed: Vec<(&str, Why)> = puts.iter().map(|p| (p.id.as_str(), p.why)).collect();
        assert_eq!(filed, [("over", Why::Over), ("mid", Why::Abandoned)], "the finished game not put twice");
        let record: yokohama_core::GameRecord = serde_json::from_str(&puts[1].record).expect("the filed record");
        assert_eq!(record.events.len(), moves, "the mid-play game is in the archive with its moves");
        drop(puts);
        for (id, _, _) in &ended {
            assert!(games.read_table(id).unwrap().is_none(), "{id} ended");
        }
        assert!(games.read_table("fresh").unwrap().is_some(), "the fresh table stands");
    }

    /// **A failed archive put refuses the delete and keeps the table for the
    /// next pass** (the creator's delete's own rule): a `sweep-failed` line
    /// carrying the delete's sentence, after its listing — and the next pass
    /// lists it again.
    /// Mutants: a failure logged as ended; the table removed anyway; a failed
    /// table remembered and skipped by the next pass.
    #[test]
    fn a_failed_archive_put_keeps_the_table_for_the_next_pass() {
        let (games, store, log) = layer(RecordingArchive::failing());
        one_move_in(&games, "mid");
        store.stamp_for_test("mid", NOW - 91 * DAY);
        for round in 0..2 {
            let from = log.events().len();
            let ended = pass(&games, &WORKER, NOW);
            assert_eq!(ended.len(), 1, "round {round}: {ended:?}");
            let (id, end, result) = &ended[0];
            assert_eq!((id.as_str(), *end), ("mid", Ending::Expired { days: 91 }));
            let error = result.clone().expect_err("the put failed");
            assert!(error.contains("the archive could not be written"), "{error}");
            let lines = lines_since(&log, from);
            assert_eq!(lines.iter().map(|(e, _)| *e).collect::<Vec<_>>(), [log::SWEEP_LISTED, log::SWEEP_FAILED]);
            assert_eq!(lines[1].1, serde_json::json!({ "id": "mid", "reason": "expired", "error": error }));
            assert_eq!(
                log::render(log::SWEEP_FAILED, &lines[1].1),
                format!(r#"{{"event":"sweep-failed","error":{},"id":"mid","reason":"expired"}}"#, serde_json::json!(error)),
            );
            assert!(games.read_table("mid").unwrap().is_some(), "round {round}: the table stands");
        }
    }

    /// **The house is never a table**: the house chat alone gives a pass
    /// nothing to list, however late it runs; and a GAME named `house` — an
    /// ordinary game name — is swept as a game while the house chat beside it
    /// is untouched.
    /// Mutants: the pass reading the house's document; the delete of a game
    /// named `house` taking the house chat.
    #[test]
    fn the_house_is_never_listed() {
        let (games, store, log) = layer(RecordingArchive::recording());
        let author = crate::chat::Author::Watcher { user: "red@t.test".to_string(), name: None };
        games.house_append(author, "hello").expect("a house message");
        assert!(games.read_document(Doc::House, crate::chat::HOUSE).unwrap().is_some(), "premise: the house is there");
        assert!(pass(&games, &WORKER, u64::MAX).is_empty());
        assert!(log.events().is_empty(), "{:?}", log.events());

        open_table(&games, crate::chat::HOUSE);
        store.stamp_for_test(crate::chat::HOUSE, NOW - 8 * DAY);
        let ended = pass(&games, &WORKER, NOW);
        assert_eq!(ended, [(crate::chat::HOUSE.to_string(), Ending::Waited { days: 8 }, Ok(()))]);
        assert!(games.read_document(Doc::House, crate::chat::HOUSE).unwrap().is_some(), "the house chat stands");
    }

    /// **A table whose activity is unknown is never ended** (R-FP6; battery
    /// row 17): over a store that keeps the trait's default — a test double
    /// forwarding the four methods and not `last_written` — a pass at the end
    /// of time ends nothing and logs nothing. The anti-vacuity partner is the
    /// same two tables on a `MemStore`, which say when they were written: the
    /// same pass ends both.
    /// Mutant: `Ok(None)` read as "idle since 1970".
    #[test]
    fn a_table_whose_activity_is_unknown_is_never_ended() {
        let build = |games: &Games| {
            open_table(games, "open");
            drop(playing(games, &two_people("dealt", CORPUS_SEED), &["red@t.test", "blue@t.test"]));
        };
        let (silent, _double) = crate::games::test_support::conflicting_games();
        let log = RecordingLog::recording();
        let silent = silent.with_log(log.clone());
        build(&silent);
        assert!(pass(&silent, &WORKER, u64::MAX).is_empty());
        assert!(log.events().is_empty(), "{:?}", log.events());
        for id in ["open", "dealt"] {
            assert!(silent.read_table(id).unwrap().is_some(), "{id} stands");
        }

        let (games, _store, _log) = layer(RecordingArchive::recording());
        build(&games);
        let ended: Vec<String> = pass(&games, &WORKER, u64::MAX).into_iter().map(|(id, _, _)| id).collect();
        assert_eq!(ended, ["dealt", "open"], "anti-vacuity: a store that says ends them both");
    }

    /// **A table the pass cannot read is named, never ended** — a document that
    /// does not parse, an activity the store cannot read — each a
    /// `sweep-unread` line with its id and the reason, the table left where it
    /// is; a listing that fails is one line with the reason and no id.
    /// Mutants: an unreadable table skipped without a word; an unreadable
    /// activity read as idle.
    #[test]
    fn a_table_the_pass_cannot_read_is_named_and_not_ended() {
        let (games, store, log) = layer(RecordingArchive::recording());
        games.load_document(Doc::Table, "bad", "{ not json").expect("a document that does not parse");
        store.stamp_for_test("bad", 0);
        assert!(pass(&games, &WORKER, NOW).is_empty());
        assert_eq!(log.events(), [log::SWEEP_UNREAD]);
        let error = log.fields(0)["error"].as_str().unwrap_or_default().to_string();
        assert!(error.contains("does not parse"), "{error}");
        assert_eq!(log.fields(0)["id"], "bad");
        assert!(games.read_document(Doc::Table, "bad").unwrap().is_some(), "left where it is");

        /// A `MemStore` whose activity cannot be read, and whose listing can
        /// be made to fail.
        #[derive(Default)]
        struct Unreadable {
            inner: MemStore,
            list_fails: std::sync::atomic::AtomicBool,
        }
        impl Store for Unreadable {
            fn list(&self, doc: Doc) -> Result<Vec<crate::store::GameId>, String> {
                if self.list_fails.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err("the directory went away".to_string());
                }
                self.inner.list(doc)
            }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, crate::store::Version)>, String> {
                self.inner.read(doc, id)
            }
            fn write_if_version(
                &self,
                doc: Doc,
                id: &str,
                json: &str,
                expected: Option<crate::store::Version>,
            ) -> Result<crate::store::Version, crate::store::WriteError> {
                self.inner.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> {
                self.inner.delete(id)
            }
            fn last_written(&self, _id: &str) -> Result<Option<u64>, String> {
                Err("the disk went away".to_string())
            }
        }
        let unreadable = Arc::new(Unreadable::default());
        let log = RecordingLog::recording();
        let games = Games::new(Arc::new(Rules::standard()), unreadable.clone() as Arc<dyn Store>).with_log(log.clone());
        open_table(&games, "open");
        assert!(pass(&games, &WORKER, u64::MAX).is_empty());
        assert_eq!(log.events(), [log::SWEEP_UNREAD]);
        assert_eq!(log.fields(0), serde_json::json!({ "id": "open", "error": "the disk went away" }));
        assert!(games.read_table("open").unwrap().is_some(), "left where it is");
        unreadable.list_fails.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(pass(&games, &WORKER, u64::MAX).is_empty());
        assert_eq!(log.events(), [log::SWEEP_UNREAD, log::SWEEP_UNREAD]);
        assert_eq!(log.fields(1), serde_json::json!({ "error": "the directory went away" }));
    }

    /// **The task waits `first_after`, then passes, and goes on passing every
    /// `every`** — by the wall clock, the one platform read the sweep makes:
    /// a table last written in 1970 is idle past any TTL. Nothing is ended
    /// before the first wait is out; the first pass ends the first table; a
    /// later pass ends one opened after it. Short durations stand in for the
    /// hour and the minute `main` passes.
    /// Mutants: a task that never passes; one that passes once and stops; one
    /// that skips the first wait.
    #[cfg(feature = "server")]
    #[tokio::test]
    async fn the_task_waits_then_passes_on_its_cadence() {
        use std::time::Duration;
        let store = Arc::new(MemStore::default());
        let state = Arc::new(
            crate::AppState::with_store(store.clone() as Arc<dyn Store>, crate::identity::IdentityMode::Open)
                .with_ttls(WORKER),
        );
        let ended = |id: &'static str| {
            let state = state.clone();
            async move {
                for _ in 0..250 {
                    if state.games.read_table(id).unwrap().is_none() {
                        return true;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                false
            }
        };
        open_table(&state.games, "first");
        store.stamp_for_test("first", 0);
        let task = tokio::spawn(run(state.clone(), state.ttls, Duration::from_millis(20), Duration::from_millis(300)));
        tokio::task::yield_now().await;
        assert!(state.games.read_table("first").unwrap().is_some(), "nothing ends before the first wait is out");
        assert!(ended("first").await, "the first pass ends an idle table");
        open_table(&state.games, "second");
        store.stamp_for_test("second", 0);
        assert!(ended("second").await, "a later pass ends the next");
        task.abort();
    }
}
