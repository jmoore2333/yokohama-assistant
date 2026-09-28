//! **The receipt archive** (M6.5c, R-6.5c.9; spec §2.6): the layer's `Archive`
//! on a Durable Object, whose put the SHELL carries out — and `stamp_archive`
//! and `expire`, the two exports that finish what it defers.
//!
//! **Why a receipt.** An R2 upload is asynchronous, and the layer calls
//! `Archive::put` inside a request and acts on its answer before the request
//! returns. So [`ReceiptArchive::put`] never uploads. It looks for a RECEIPT —
//! the outcome of an upload the shell has already made for this exact plan —
//! and answers with it. With none, it records the PLAN and answers
//! `Failed("deferred to the host")`, and the plan rides out in the call's
//! `Reply.archive` ([`drain_plan`], which `from_the_layer`, the one drain path,
//! calls after every call). The shell uploads and hands the outcome to
//! [`receipt_inner`]. Then it dispatches the SAME request again — an abandon, a
//! browser's save — whose put meets the receipt; or, at game over, where a move
//! cannot be run twice, it calls [`stamp_archive_inner`].
//!
//! **A plan is its four parts** — both keys and the SHA-256 of both bodies —
//! and its `key` is a SHA-256 over them, so a receipt answers only the put it
//! was made for. At the layer's two server sites — game over and the abandon —
//! it stamps `archived_at` from its clock inside the table body it puts, so a
//! put there at another reading is another plan: the shell pins the clock to a
//! plan's reading for every call that drives it (ruling P-2), and records that
//! reading beside the plan, which has no field for it. A browser's save puts
//! the table it was sent, which the layer does not stamp, so its plan does not
//! move with the reading.
//!
//! **The `test/` root** (spec §2.6, §5): a plan whose address is an EMAIL in
//! `.test` — the reserved domain every test identity uses, and never a plain
//! name (M6.5c2, review R2.2-m4: under the open door `a.test` is a legal name)
//! — is keyed under `test/` instead of the bucket's top, so the live test
//! tiers never write into the owner's training data ([`is_test_identity`]).
//! The address is read WHOLE: a server game's table's `created_by`, or the
//! address a browser's save's Archive object was initialized with — never the
//! path's `caller_segment`, which the 64-character cap can strip of its
//! `.test`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use yokohama_web::archive::{caller_segment, paths, Archive, PutError, PutOutcome, Why};
use yokohama_web::games::Games;
use yokohama_web::identity::User;
use yokohama_web::store::Doc;
use yokohama_web::table::TableDoc;
use yokohama_web::ApiError;

use crate::config::ARCHIVE_VAR;
use crate::{dispatch_inner, error_body, from_the_layer, read_inner, sha256_hex, ReplyData};

/// The root a test identity's plan is keyed under (spec §2.6).
const TEST_ROOT: &str = "test/";

/// The reserved domain every test identity's address ends in.
const TEST_DOMAIN: &str = ".test";

/// Whether `address` is a TEST identity's (spec §2.6; M6.5c2, review R2.2-m4):
/// an EMAIL in the reserved `.test` domain. Every address this reads is a key
/// the layer's `User::parse` wrote — a table's `created_by`, or the caller
/// `init` normalized — and under the key grammar a plain name never holds an
/// `@`, so a plain name is never one, whatever it ends in: under the open door
/// `a.test` is a legal name, and the `test/` prefix EXPIRES after 30 days
/// (R-6.5c2.12).
fn is_test_identity(address: &str) -> bool {
    address.contains('@') && address.ends_with(TEST_DOMAIN)
}

/// What a put that met no receipt answers: a `Failed`, whose words each of the
/// layer's three sites puts inside its own sentence — the abandon's and the
/// browser save's refusals, and game over's log line.
pub(crate) const DEFERRED: &str = "deferred to the host";

/// A browser's save on an instance given no caller: every object but an
/// Archive object.
const NO_CALLER: &str =
    "a browser's save is filed by its Archive object's address, and this instance was given none";

/// A browser's save whose path segment is not this instance's caller's: the
/// layer names the SENDER in the path, and the root is read off the object's
/// address, so the two must be one caller (R1's L-1). Unreachable through the
/// Worker, which names the Archive object by the address it forwards as the
/// sender.
const ANOTHER_CALLER: &str =
    "a browser's save is filed by its Archive object's address, and this save was sent by another caller";

/// This instance's receipt archive — the SAME `Arc` its `Games` puts through —
/// or `None` with the archive `off`. [`crate::init_inner`] replaces it each
/// time it rebuilds the `Games`, so a new instance holds no receipt and no
/// pending plan.
pub(crate) static RECEIPTS: Mutex<Option<Arc<ReceiptArchive>>> = Mutex::new(None);

/// The layer's `Archive` under `YOKOHAMA_WEB_ARCHIVE=r2` (spec §2.6): a put is
/// answered from a receipt, or recorded as a plan and deferred.
pub struct ReceiptArchive {
    /// The outcomes the shell has installed, by plan key. The put a receipt
    /// answers SPENDS it, and a put for any other plan does not see it.
    receipts: Mutex<HashMap<String, Outcome>>,
    /// The plan of the last put that met no receipt, until [`drain_plan`]
    /// hands it to the call's reply.
    pending: Mutex<Option<Plan>>,
    /// The Archive object's address, as `init` normalized it; `None` on every
    /// other object.
    caller: Option<String>,
}

/// A deferred put, as `Reply.archive` carries it (§"Names shared across
/// tasks"): the key a receipt for it must name, the two keys to write, and
/// both bodies exactly as the layer handed them over.
#[derive(Serialize)]
pub struct Plan {
    /// SHA-256, in lowercase hex, over
    /// `record_key|table_key|sha256(record)|sha256(table)` ([`plan_key`]).
    pub key: String,
    pub record_key: String,
    pub table_key: String,
    /// The record's exact text: the store's bytes for a server's own game,
    /// and the bytes a browser sent for its save.
    pub record: String,
    /// The stripped table document's exact text.
    pub table: String,
}

/// An upload's outcome, as the shell hands it to [`receipt_inner`]
/// (§"Names shared across tasks"): one of four words, a detail — R2's
/// message, or the key — and, on a `conflict`, the record the key HELD, as its
/// exact text (M6.5c2, R-6.5c2.11), which the layer's refinish rule compares.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub outcome: Kind,
    pub detail: String,
    /// The record the key already held — beside a `conflict` only
    /// ([`parse_outcome`] refuses it elsewhere), and only where the shell read
    /// it. `None` reads as not the same game.
    #[serde(default)]
    pub held: Option<String>,
}

/// The four outcomes of an upload carried out in `DirArchive::put`'s order
/// (spec §2.6).
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Both keys were written.
    Landed,
    /// The record key already held these very bytes.
    Identical,
    /// The record key holds a different record.
    Conflict,
    /// The upload did not happen; the detail is R2's message.
    Failed,
}

impl ReceiptArchive {
    /// An archive with no receipt and no pending plan, filing a browser's save
    /// by `caller` — already normalized by `init`.
    pub(crate) fn new(caller: Option<String>) -> Self {
        Self { receipts: Mutex::new(HashMap::new()), pending: Mutex::new(None), caller }
    }

    /// `"test/"` for a test identity's plan ([`is_test_identity`]), else `""`
    /// — by the FULL address.
    /// `prefix` is one of the two the layer passes (`Archive::put`'s doc): `""`
    /// for a server's own game, whose address is its table's `created_by`
    /// (`strip_for_archive` keeps it), and `browser/<segment>/` for a browser's
    /// save, whose address is this archive's `caller`.
    ///
    /// A plan with no address to read is REFUSED rather than keyed at the
    /// bucket's top, where a test identity's save would join the owner's
    /// training data. So is a plan under any other prefix: the layer passes no
    /// third, and a plan this archive cannot root is keyed nowhere. And so is a
    /// browser's save whose segment is not `caller_segment` of this archive's
    /// `caller` (R1's L-1): the layer names the SENDER in the path, the root is
    /// the object's, and a save rooted by one caller and filed under another's
    /// name would put a test identity's save in an address's training data, or
    /// an address's save under `test/`. Segments collide by design
    /// (`caller_segment`'s doc), so a match proves the path is this object's
    /// segment, not that the sender spelled its address as the object does.
    fn root(&self, prefix: &str, table_json: &str) -> Result<&'static str, PutError> {
        let test = match prefix {
            "" => is_test_identity(
                &creator_of(table_json)
                    .ok_or_else(|| PutError::Failed("the table document names no creator".to_string()))?,
            ),
            browser if browser.starts_with("browser/") => {
                let caller = self.caller.as_deref().ok_or_else(|| PutError::Failed(NO_CALLER.to_string()))?;
                // `init` normalized the caller through this same parse, so it
                // reads; one that did not would root nothing.
                let own = User::parse(caller).map(|user| format!("browser/{}/", caller_segment(&user)));
                if own.as_deref() != Ok(browser) {
                    return Err(PutError::Failed(ANOTHER_CALLER.to_string()));
                }
                is_test_identity(caller)
            }
            other => return Err(PutError::Failed(format!("the archive keeps no root for the prefix {other:?}"))),
        };
        Ok(if test { TEST_ROOT } else { "" })
    }
}

impl Archive for ReceiptArchive {
    /// Spec §2.6's put, in its four steps:
    ///
    /// 1. the two keys `archive::paths` computes, with the rev
    ///    `DirArchive::put` passes, under [`ReceiptArchive::root`];
    /// 2. the plan's key over its four parts ([`plan_key`]);
    /// 3. a receipt for that key, REMOVED and answered: `landed` and
    ///    `identical` as themselves, `conflict` as a conflict at the record
    ///    key carrying the record it held, `failed` as its detail;
    /// 4. with no receipt, the plan recorded and the put deferred.
    fn put(
        &self,
        prefix: &str,
        why: Why,
        id: &str,
        record_json: &str,
        table_json: &str,
    ) -> Result<PutOutcome, PutError> {
        let root = self.root(prefix, table_json)?;
        let (record_key, table_key) = paths(yokohama_core::SEMANTICS_REV, prefix, why, id).map_err(PutError::Failed)?;
        let (record_key, table_key) = (format!("{root}{record_key}"), format!("{root}{table_key}"));
        let key = plan_key(&record_key, &table_key, record_json, table_json);
        if let Some(receipt) = self.receipts.lock().expect("the receipts").remove(&key) {
            return match receipt.outcome {
                Kind::Landed => Ok(PutOutcome::Landed),
                Kind::Identical => Ok(PutOutcome::Identical),
                // The held record's text, as the shell read it from the key
                // (M6.5c2, R-6.5c2.11): the layer's refinish rule compares it.
                Kind::Conflict => Err(PutError::Conflict { key: record_key, held: receipt.held }),
                Kind::Failed => Err(PutError::Failed(receipt.detail)),
            };
        }
        *self.pending.lock().expect("the pending plan") = Some(Plan {
            key,
            record_key,
            table_key,
            record: record_json.to_string(),
            table: table_json.to_string(),
        });
        Err(PutError::Failed(DEFERRED.to_string()))
    }

    fn kind(&self) -> &'static str {
        "r2"
    }
}

/// A plan's key: SHA-256 over `record_key|table_key|sha256(record)|sha256(table)`,
/// each digest in lowercase hex. No part can hold a `|` — a key joins a safe id
/// and a caller segment, and a digest is hex — so the four cannot run together.
fn plan_key(record_key: &str, table_key: &str, record: &str, table: &str) -> String {
    sha256_hex(&format!("{record_key}|{table_key}|{}|{}", sha256_hex(record), sha256_hex(table)))
}

/// A table document's `created_by`, read from its JSON object — `None` for
/// anything that is not an object with a string there.
fn creator_of(table_json: &str) -> Option<String> {
    let doc: Value = serde_json::from_str(table_json).ok()?;
    doc.get("created_by")?.as_str().map(str::to_string)
}

/// The pending plan, as `Reply.archive`'s JSON, TAKEN so the next call starts
/// with none. `None` when no put was deferred, and always with the archive
/// `off`.
pub(crate) fn drain_plan() -> Option<String> {
    let guard = RECEIPTS.lock().expect("the host's receipts");
    let plan = guard.as_ref()?.pending.lock().expect("the pending plan").take()?;
    Some(serde_json::to_string(&plan).expect("a plan serializes"))
}

/// `receipt(planKey, outcome)` (spec §2.6): install the shell's outcome for the
/// plan whose `key` is `plan_key`, replacing any earlier one. The put that plan
/// was made for SPENDS it — the same request dispatched again at the same
/// reading, or [`stamp_archive_inner`] — and a put for any other plan does not
/// see it.
///
/// Refused, with a sentence: an outcome that is not the object the shell writes
/// — `{"outcome": "landed" | "identical" | "conflict" | "failed", "detail": …}`,
/// a `conflict` optionally with `"held": <text>`, and no other key — and any
/// receipt with the archive `off`, where no put is deferred.
pub fn receipt_inner(plan_key: &str, outcome: &str) -> Result<(), String> {
    let guard = RECEIPTS.lock().expect("the host's receipts");
    let archive = guard
        .as_ref()
        .ok_or_else(|| format!("{ARCHIVE_VAR} is off, so no put is deferred and no receipt is taken"))?;
    let outcome = parse_outcome(outcome)?;
    archive.receipts.lock().expect("the receipts").insert(plan_key.to_string(), outcome);
    Ok(())
}

/// The outcome, read ONLY from its object form. serde's derived `Deserialize`
/// also reads a struct from a positional ARRAY, a form `deny_unknown_fields`
/// does not govern, so an array is refused before serde reads it — `picker.rs`'s
/// rule for its row.
///
/// `held` beside anything but a `conflict` is refused (M6.5c2): the shell
/// sets it only where the key held other bytes, so a landed, identical or
/// failed receipt carrying a record is a shell defect. The sentence names no
/// byte of the record.
fn parse_outcome(json: &str) -> Result<Outcome, String> {
    let value: Value = serde_json::from_str(json).map_err(|e| format!("a receipt's outcome is not JSON: {e}"))?;
    if !value.is_object() {
        return Err(format!("a receipt's outcome is the object {{\"outcome\", \"detail\"}}, not {json}"));
    }
    let outcome: Outcome =
        serde_json::from_value(value).map_err(|e| format!("a receipt's outcome does not parse: {e}"))?;
    if outcome.held.is_some() && !matches!(outcome.outcome, Kind::Conflict) {
        return Err("a receipt carries the held record only beside a conflict".to_string());
    }
    Ok(outcome)
}

/// `stamp_archive(id)` (spec §2.6's game-over site; R-6.5c.9): finish a
/// game-over save the shell has uploaded, with the layer's own code —
/// `Games::archive_over_retry`, which runs `archive_if_over` again over the
/// stored record. At the reading the plan was made at (the shell pins it,
/// ruling P-2) that put meets the receipt the shell installed, and on `landed`
/// or `identical` the layer stamps `archived_at`, retrying the stamp once on a
/// version conflict.
///
/// **The host calls this ONLY for an `archive_owed` plan that `persist`
/// started** — never as a sweep of unstamped tables (ruling E2):
/// `archive_over_retry` reads "over" from the resident handle but archives the
/// store's bytes, and the two are one finished game only where `persist` wrote
/// those bytes at game over — the write that deferred the plan.
///
/// **The 200 is `{"stamped": <bool>, "over": <bool>}`** (ruling I3, and the
/// orchestrator's ruling on I3's undo case):
///
/// - `stamped` says whether the stamp LANDED: whether the stored table's
///   `archived_at` is set after the retry. `archive_over_retry` answers
///   `Ok(())` either way, because inside `archive_if_over` a refused put or a
///   failed stamp is a log line — which reaches the shell since M6.5c2, but is
///   a report and not an answer — so this reads the table the stamp is written
///   to;
/// - `over` says whether the game is over at retry time: the resident
///   handle's reading, the one `archive_if_over` takes, and never the table's
///   `turn` mirror, a cache that can be stale.
///
/// The shell reads its arms from the pair and the upload's outcome together
/// (`settledArm`, `worker/src/r2.ts`):
///
/// - `stamped: true` — the save is complete: clear `archive_owed`. After a
///   `conflict` receipt this is the refinish rule (M6.5c2, R-6.5c2.11): the
///   record the key held was an earlier completion of THIS game — the same
///   seed, configuration and choices, which the layer compares — the archive
///   keeps it, and the table is stamped;
/// - `stamped: false, over: false` — the game is no longer over (below):
///   clear it too;
/// - `stamped: false, over: true` — the stamp did not land. After a `conflict`
///   receipt the conflict is FINAL: cleared, and never retried. The key holds
///   another game's record, a text that is no record, or a record the shell
///   did not hand on, which no retry can change — or, rarely, this game's,
///   with the stamp write itself failed, which spec §2.5 leaves final too:
///   every conflict but the one the layer stamped. Otherwise it is a `failed`
///   receipt, a put that met none (its plan, deferred again, in this reply's
///   `archive`), or a stamp write that failed, and the alarm retries.
///
/// **No longer over** (ruling E3): after an undo at a one-person table,
/// `archive_if_over` puts nothing and stamps nothing. A later completion puts
/// afresh; where the first completion's copy has landed with different bytes,
/// that put is a `Conflict` carrying the first completion, and the layer
/// stamps (R-6.5c2.11) — the archive keeps the first completion, as the
/// directory archive does. A game abandoned
/// instead, whose copy landed under `over/` without its stamp, is archived
/// again under `abandoned/` — the directory archive's own behaviour
/// (`Archive`'s doc), kept for parity.
///
/// An error is the layer's own, in its error shape: an open or dealing table
/// answers `handle`'s `Err(Open | Dealing)`, and an id with no table its 404.
pub fn stamp_archive_inner(id: &str) -> ReplyData {
    from_the_layer(|games| {
        // `over` is read off the handle the retry has just found or built, so
        // this rebuilds nothing, and nothing between the two moves the game.
        let over = games.archive_over_retry(id).and_then(|()| {
            let handle = games.handle(id)?;
            let over = handle.lock().expect("a game").session.is_over();
            Ok(over)
        });
        match over {
            Ok(over) => {
                let reply = Stamp { stamped: stamped(games, id), over };
                (200, serde_json::to_string(&reply).expect("a stamp serializes"), None)
            }
            Err(e) => {
                let e = ApiError::from(e);
                (e.status.as_u16(), error_body(&e), None)
            }
        }
    })
}

/// [`stamp_archive_inner`]'s 200 body. A struct rather than `json!`, whose
/// map sorts its keys: the fields serialize in this order, `stamped` first.
#[derive(Serialize)]
struct Stamp {
    stamped: bool,
    over: bool,
}

/// Whether `id`'s stored table carries `archived_at`: the document read
/// through the store and deserialized, so a table that cannot be read is not
/// stamped.
fn stamped(games: &Games, id: &str) -> bool {
    games
        .read_document(Doc::Table, id)
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<TableDoc>(&json).ok())
        .is_some_and(|doc| doc.archived_at.is_some())
}

/// `expire(id)` (spec §2.4 step 1; R-6.5c.15): expiry, and the operator's
/// plain expire, as the creator's own Abandon — the layer's
/// `DELETE /api/games/{id}` dispatched as the stored table's `created_by`. The
/// creator is always seated (spec §0.2), so the request passes both of the
/// layer's `delete` checks, and the answer, the archive rule and the leaf are
/// the layer's: a save is owed exactly where the creator's Abandon owes one,
/// and deferred as its is.
///
/// With no creator to act as — no table stored under `id`, or one whose
/// document names none — it answers what the layer answers for that table:
/// `Games::handle`'s refusal, which reads the table through the same
/// `read_table` as `delete` (a 404 for no table, the layer's 500 for a document
/// that does not read).
///
/// **One arm answers otherwise.** Where a game is still RESIDENT under `id`
/// while its table is gone or names no creator, `handle` answers `Ok`, and
/// this answers a host-written 500: `"{id:?} has a game in play but no table
/// document that names its creator"`, not the refusal the layer's `delete`
/// would give: a 404 for a table that is gone, and a 500 for a table document
/// that does not read, which one whose `created_by` is not a string is. No
/// public layer call answers the table read's own refusal without a caller or
/// a rebuild, and restating the layer's sentence here would be a copy that
/// drifts (the orchestrator's ruling on the review's I1: the sentence joins
/// spec §4's host-written list). The shell cannot reach that state. `load` only
/// creates, so it never replaces a table. The layer's own writes of
/// `Doc::Table` under a live id — a join, a keep, an unlock, a stamp, the turn
/// mirror — keep a document that names its creator, and its delete drops the
/// handle with the table. The only calls that could replace or remove a table
/// under a resident game are the house's `put_copy` and `remove_copy`, which
/// the shell makes on the house's own instance, where no game is played.
pub fn expire_inner(id: &str) -> ReplyData {
    let creator = read_inner("table", id).ok().flatten().as_deref().and_then(creator_of);
    match creator {
        Some(creator) => dispatch_inner(Some(&creator), "DELETE", &format!("/api/games/{id}"), "", None, b""),
        None => from_the_layer(|games| {
            let e = match games.handle(id) {
                Err(e) => ApiError::from(e),
                // A game is resident only where its table was read, and the
                // layer's delete drops the two together. Only the house's
                // `put_copy` or `remove_copy` could replace or remove the table
                // under a resident game, and the shell calls those on the
                // house's own instance, where no game is ever played.
                Ok(_) => ApiError::server_error(format!(
                    "{id:?} has a game in play but no table document that names its creator"
                )),
            };
            (e.status.as_u16(), error_body(&e), None)
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use yokohama_core::{replay_from, state_at, GameRecord, PlayerId, Rules};
    use yokohama_web::archive::{caller_segment, strip_for_archive, Archive, PutError, Why};
    use yokohama_web::identity::User;
    use yokohama_web::table::TableDoc;
    use yokohama_web::ApiError;

    use super::ReceiptArchive;
    use crate::tests::{call, config, serial, tagged};
    use crate::{
        create_inner, error_body, events_inner, expire_inner, init_inner, load_inner, read_inner, receipt_inner,
        restore_picker_inner, set_clock, stamp_archive_inner, ReplyData,
    };

    const RED: &str = "red@t.test";
    const BLUE: &str = "blue@t.test";
    /// The committed browser-played pair's creator, and its one human seat.
    const LOCAL: &str = "local@browser";
    /// The committed pair's own id, which the owed-game-over fixture keeps
    /// (plan review m12).
    const OWED: &str = "local-242a5429453c593c";

    /// `init`'s configuration with the archive ON, as the shell builds it
    /// (§"Names shared across tasks"): `caller` is the Archive object's
    /// address, and `null` on every other object. The worker door.
    fn r2(caller: Option<&str>) -> String {
        r2_at(caller, "worker")
    }

    /// [`r2`] at `door` (M6.5c2, spec §2.1): `worker` or `open`.
    fn r2_at(caller: Option<&str>, door: &str) -> String {
        json!({"identity": door, "archive": "r2", "caller": caller, "lobby_chat": null, "chat_keep": null, "chat_text": null})
            .to_string()
    }

    /// The clock pinned to a reading until the guard drops, and then the
    /// setting it replaced restored — ruling P-2's pin-and-restore, kept even
    /// when a cell panics, so a failure never hands the next cell a frozen
    /// clock.
    struct Pinned(Option<f64>);

    impl Drop for Pinned {
        fn drop(&mut self) {
            set_clock(self.0);
        }
    }

    fn pin(seconds: u64) -> Pinned {
        Pinned(set_clock(Some(seconds as f64)))
    }

    /// SHA-256 in lowercase hex — computed HERE, so no cell checks the
    /// module's key against the module's own digest.
    fn sha(text: &str) -> String {
        format!("{:x}", Sha256::digest(text.as_bytes()))
    }

    /// The reply's deferred plan, parsed.
    fn plan_of(r: &ReplyData) -> Value {
        let plan = r.archive.as_deref().unwrap_or_else(|| panic!("the reply carries a deferred plan: {} {}", r.status, r.body));
        serde_json::from_str(plan).expect("a plan is JSON")
    }

    fn key(plan: &Value) -> &str {
        plan["key"].as_str().expect("a plan's key")
    }

    /// The plan's shape (§"Names shared across tasks") and its two keys, with
    /// its `key` recomputed here from the four parts spec §2.6 names:
    /// SHA-256 over `record_key|table_key|sha256(record)|sha256(table)`.
    fn assert_plan(plan: &Value, record_key: &str, table_key: &str) {
        let mut fields: Vec<&str> = plan.as_object().expect("a plan is an object").keys().map(String::as_str).collect();
        fields.sort_unstable();
        assert_eq!(fields, ["key", "record", "record_key", "table", "table_key"], "{plan}");
        assert_eq!(
            (plan["record_key"].as_str(), plan["table_key"].as_str()),
            (Some(record_key), Some(table_key)),
            "the plan's two keys"
        );
        let (record, table) = (plan["record"].as_str().unwrap(), plan["table"].as_str().unwrap());
        let want = sha(&format!("{record_key}|{table_key}|{}|{}", sha(record), sha(table)));
        assert_eq!(key(plan), want, "the plan's key is over its four parts");
    }

    /// The outcome JSON the shell hands `receipt` (§"Names shared across
    /// tasks").
    fn outcome(kind: &str, detail: &str) -> String {
        json!({"outcome": kind, "detail": detail}).to_string()
    }

    fn stored(kind: &str, id: &str) -> Option<String> {
        read_inner(kind, id).unwrap()
    }

    /// The stored table's `archived_at`: `None` while it is unstamped.
    fn archived_at(id: &str) -> Option<u64> {
        let doc: TableDoc = serde_json::from_str(&stored("table", id).expect("a stored table")).unwrap();
        doc.archived_at
    }

    fn worker_fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures")
    }

    fn web_fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../yokohama-web/tests/fixtures")
    }

    fn read(path: PathBuf) -> String {
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    /// A browser's save of the committed finished pair: `POST /api/archive`'s
    /// body, the record and the table as the two strings the route takes.
    fn the_committed_save() -> String {
        json!({
            "record": read(web_fixtures().join("browser-played-2p.json")),
            "table": read(web_fixtures().join("browser-played-2p.table.json")),
        })
        .to_string()
    }

    /// The committed owed-game-over pair, loaded under its own id as the wake
    /// path loads a table's documents.
    fn load_owed_game_over() {
        load_inner("table", OWED, &read(worker_fixtures().join("owed-game-over.table.json"))).unwrap();
        load_inner("record", OWED, &read(worker_fixtures().join("owed-game-over.json"))).unwrap();
    }

    /// [`load_owed_game_over`], then `GET …/record`, whose rebuild of the cold
    /// handle pumps the bot's run and persists it — the game-over site. Answers
    /// the GET's reply, each premise asserted.
    fn pump_owed_game_over() -> ReplyData {
        load_owed_game_over();
        let r = call(LOCAL, "GET", &format!("/api/games/{OWED}/record"), None);
        assert_eq!(r.status, 200, "premise: the record route serves only a FINISHED game — {}", r.body);
        let full: GameRecord = serde_json::from_str(&read(web_fixtures().join("browser-played-2p.json"))).unwrap();
        let pumped: GameRecord = serde_json::from_str(&r.body).unwrap();
        assert_eq!(pumped.events.len(), full.events.len(), "premise: the pump played to the committed pair's END");
        assert_eq!(tagged(&r), format!(r#"[{{"bump":"{OWED}"}}]"#), "premise: the pump persisted");
        r
    }

    /// How far [`table`] builds a table.
    #[derive(Clone, Copy, PartialEq)]
    enum Stage {
        /// The creator seated, the other seat open.
        Open,
        /// The joiner seated, nothing kept.
        Dealing,
        /// Both seats kept: the game is dealt, and no move is made.
        Playing,
        /// The creator's first `1` applied: one event.
        Moved,
    }

    /// A 2P table at `id`, created by `creator` and joined by `joiner`, built
    /// up to `upto` with each step's premise asserted. The seed is `CREATE`'s,
    /// 20260831, at which the creator's first `1` APPLIES the opening move (the
    /// arrangement `picker.rs` measured).
    fn table(id: &str, creator: &str, joiner: &str, upto: Stage) {
        let create = json!({"players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": 20260831,
                            "seats": [{"user": creator}, {"open": true}]})
        .to_string();
        let r = create_inner(Some(creator), create.as_bytes(), id, Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        if upto == Stage::Open {
            return;
        }
        let r = call(joiner, "POST", &format!("/api/games/{id}/join"), Some(r#"{"seat":1}"#));
        assert_eq!(r.status, 200, "{}", r.body);
        if upto == Stage::Dealing {
            return;
        }
        for (user, seat) in [(creator, 0), (joiner, 1)] {
            let keep = json!({"seat": seat, "card": 1}).to_string();
            let r = call(user, "POST", &format!("/api/games/{id}/keep"), Some(&keep));
            assert_eq!(r.status, 200, "{}", r.body);
        }
        if upto == Stage::Playing {
            return;
        }
        let r = call(creator, "POST", &format!("/api/games/{id}/input"), Some(r#"{"line":"1"}"#));
        let s: Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!((r.status, &s["events"]), (200, &json!(1)), "premise: the first `1` APPLIES the opening move — {}", r.body);
    }

    /// A playing table with one event — which the layer's abandon archives.
    fn playing_with_a_move(id: &str, creator: &str, joiner: &str) {
        table(id, creator, joiner, Stage::Moved);
    }

    /// COPIED from `partial_of` in `crates/yokohama-web/tests/wire_fixtures.rs`:
    /// a private helper of another crate's test binary, which cannot be
    /// imported (ruling R-06). Truncate, replay to the cut, stamp the terminal
    /// digest for THAT state, serialize.
    fn partial_of(record_text: &str, events: usize) -> String {
        use yokohama_core::{state_at, GameRecord};
        let mut rec: GameRecord = serde_json::from_str(record_text).expect("a record");
        assert!(rec.events.len() > events, "anti-vacuity: the partial must be shorter");
        rec.events.truncate(events);
        let state = state_at(&rec, events, &Rules::standard()).expect("the partial replays");
        rec.stamp_terminal_digest(&state);
        serde_json::to_string(&rec).expect("a record")
    }

    /// No receipt: the abandon is refused as deferred, with its plan and NOTHING
    /// deleted; the receipt for the EXACT plan lets the SAME request, at the SAME
    /// pinned reading, delete — and its bumps say `end`.
    #[test]
    fn an_abandon_defers_then_completes_on_its_receipt_at_the_pinned_reading() {
        let _s = serial();
        init_inner(&r2(None)).unwrap();
        let health = call(RED, "GET", "/api/health", None).body;
        assert!(health.contains(r#""archive":"r2""#), "premise: the switch is on — {health}");
        playing_with_a_move("abandon", RED, BLUE);
        let record = stored("record", "abandon").expect("premise: the game's record");
        let table = stored("table", "abandon").expect("premise: its table");
        let _p = pin(1_789_000_100);

        let first = call(RED, "DELETE", "/api/games/abandon", None);
        assert_eq!(
            (first.status, first.body.as_str(), tagged(&first).as_str()),
            (500, r#"{"error":"the archive could not be written (deferred to the host) — the table was not deleted"}"#, "[]"),
            "the layer's own refusal, with the deferral inside it, and nothing signalled",
        );
        let plan = plan_of(&first);
        assert_plan(&plan, "test/rev7/abandoned/abandon.json", "test/rev7/abandoned/abandon.table.json");
        assert_eq!(plan["record"], record, "the archived record is the stored bytes");
        let mut stamped: TableDoc = serde_json::from_str(&table).unwrap();
        stamped.archived_at = Some(1_789_000_100);
        assert_eq!(plan["table"], strip_for_archive(&stamped), "the stripped table, stamped at the pinned reading");
        assert_eq!(
            (stored("table", "abandon"), stored("record", "abandon")),
            (Some(table), Some(record)),
            "nothing deleted and nothing written",
        );

        receipt_inner(key(&plan), &outcome("landed", "test/rev7/abandoned/abandon.json")).unwrap();
        let second = call(RED, "DELETE", "/api/games/abandon", None);
        assert_eq!(
            (second.status, second.body.as_str(), tagged(&second).as_str()),
            (204, "", r#"[{"end":"abandon"}]"#),
            "the SAME request at the SAME reading meets the receipt and deletes",
        );
        assert_eq!(second.archive, None, "the receipt answered the put, so nothing is deferred");
        assert_eq!((stored("table", "abandon"), stored("record", "abandon")), (None, None), "deleted");
    }

    /// **A game dealt on this host names it, and the plan carries it**
    /// (M6.5c2, spec §2.4): the record `finalize` wrote here says
    /// `run_id: "web/worker"`, and the plan an abandon defers holds those
    /// stored bytes — `run` and all — for R2.
    #[test]
    fn a_game_dealt_here_names_web_worker_and_its_plan_carries_the_stored_bytes() {
        let _s = serial();
        init_inner(&r2(None)).unwrap();
        playing_with_a_move("named", RED, BLUE);
        let record = stored("record", "named").expect("the game's record");
        let v: Value = serde_json::from_str(&record).unwrap();
        assert_eq!(v["run"]["run_id"], "web/worker", "finalize stamps where it was played");
        let _p = pin(1_789_000_950);
        let first = call(RED, "DELETE", "/api/games/named", None);
        assert_eq!(first.status, 500, "premise: the abandon defers its put — {}", first.body);
        assert_eq!(plan_of(&first)["record"], record, "the plan's record is the stored bytes, `run` and all");
    }

    /// A receipt whose digests differ is ABSENT (a re-dispatch at a different
    /// reading stamps a different `archived_at`, so its plan's key differs).
    #[test]
    fn a_receipt_for_another_plan_is_not_this_plans() {
        let _s = serial();
        init_inner(&r2(None)).unwrap();
        playing_with_a_move("other", RED, BLUE);
        let _p = pin(1_789_000_200);
        let at_r = plan_of(&call(RED, "DELETE", "/api/games/other", None));
        receipt_inner(key(&at_r), &outcome("landed", "test/rev7/abandoned/other.json")).unwrap();

        set_clock(Some(1_789_000_201.0));
        let next = call(RED, "DELETE", "/api/games/other", None);
        assert_eq!(next.status, 500, "still deferred: {}", next.body);
        let at_next = plan_of(&next);
        assert_plan(&at_next, "test/rev7/abandoned/other.json", "test/rev7/abandoned/other.table.json");
        assert_ne!(key(&at_next), key(&at_r), "a NEW plan, under a key of its own");
        assert_eq!(at_next["record"], at_r["record"], "the same record: only the table's stamp moved");
        let stamp = |plan: &Value| -> Value {
            serde_json::from_str::<Value>(plan["table"].as_str().unwrap()).unwrap()["archived_at"].clone()
        };
        assert_eq!((stamp(&at_r), stamp(&at_next)), (json!(1_789_000_200u64), json!(1_789_000_201u64)));
        assert!(stored("table", "other").is_some(), "nothing deleted");

        // The receipt was not SPENT by the put it did not match: at the reading
        // it was made for, the same request meets it and completes.
        set_clock(Some(1_789_000_200.0));
        let done = call(RED, "DELETE", "/api/games/other", None);
        assert_eq!((done.status, tagged(&done).as_str()), (204, r#"[{"end":"other"}]"#), "{}", done.body);
    }

    /// The `test/` root, and the truncation it must not be fooled by (battery 33).
    #[test]
    fn the_test_root_reads_the_full_address_never_the_truncated_segment() {
        let _s = serial();
        let _p = pin(1_789_000_300);
        // A SERVER game's root reads its table's `created_by` — the CREATOR's
        // address and no other seat's: each joiner here is at the other root.
        // The domain is `.test` with its dot: `t.contest` ends in `test` and is
        // not a test identity's.
        for (id, creator, joiner, root) in [
            ("dot-test", RED, "blue@example.com", "test/"),
            ("dot-com", "red@example.com", BLUE, ""),
            ("contest", "red@t.contest", BLUE, ""),
        ] {
            init_inner(&r2(None)).unwrap();
            playing_with_a_move(id, creator, joiner);
            let r = call(creator, "DELETE", &format!("/api/games/{id}"), None);
            assert_eq!(r.status, 500, "premise: {id}'s abandon deferred — {}", r.body);
            let base = format!("{root}rev7/abandoned/{id}");
            assert_plan(&plan_of(&r), &format!("{base}.json"), &format!("{base}.table.json"));
        }

        // A BROWSER's save reads the address its Archive object was initialized
        // with, WHOLE — never the 64-character segment in the path, which the
        // cap has stripped of its `.test` here.
        let long = format!("{}@t.test", "a".repeat(63));
        let truncated = caller_segment(&User::parse(&long).unwrap());
        assert_eq!((long.len(), truncated.len()), (70, 64), "premise: the address is longer than the cap");
        assert!(!truncated.ends_with(".test"), "premise: the segment has lost `.test` — {truncated}");
        let save = the_committed_save();
        for (caller, sender, root) in [
            (long.as_str(), long.as_str(), "test/"),
            // `init` normalizes its caller as the layer normalizes a sender
            // (`User::parse`), so an address passed in capitals is still a test
            // identity's.
            ("Red@T.TEST", RED, "test/"),
            ("red@example.com", "red@example.com", ""),
        ] {
            init_inner(&r2(Some(caller))).unwrap();
            let r = call(sender, "POST", "/api/archive", Some(&save));
            assert_eq!(
                (r.status, r.body.as_str()),
                (500, r#"{"error":"the archive could not be written: deferred to the host"}"#),
                "{caller}: premise: the save deferred",
            );
            let segment = caller_segment(&User::parse(sender).unwrap());
            let base = format!("{root}browser/{segment}/rev7/over/{OWED}");
            assert_plan(&plan_of(&r), &format!("{base}.json"), &format!("{base}.table.json"));
        }

        // With NO caller — every object but an Archive object — a browser's
        // save has no address to read, and is REFUSED rather than filed at the
        // bucket's top, where a test identity's save would join the owner's
        // training data.
        init_inner(&r2(None)).unwrap();
        let r = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(
            (r.status, r.body.as_str(), r.archive.as_deref()),
            (
                500,
                r#"{"error":"the archive could not be written: a browser's save is filed by its Archive object's address, and this instance was given none"}"#,
                None
            ),
        );
    }

    /// **A plain name never takes the `test/` root** (M6.5c2, review
    /// R2.2-m4). Under the open door `a.test` is a legal name (the key
    /// grammar), and the `test/` prefix EXPIRES after 30 days (R-6.5c2.12): a
    /// player so named must not have their saves filed where the test tiers'
    /// are pruned. The test root takes an EMAIL in `.test`, whose `@` a plain
    /// name never has — at both roots: a server game's creator, and a browser
    /// save's caller. The partner at each root is an address in `.test`,
    /// which still takes it. Mutant: the old `ends_with(".test")` alone.
    #[test]
    fn a_plain_name_never_takes_the_test_root() {
        let _s = serial();
        let _p = pin(1_789_001_200);
        // A server game's root: its creator's. The joiner is a plain name at
        // both rows, and the creator's root is the one read.
        for (creator, root) in [("a.test", ""), (RED, "test/")] {
            init_inner(&r2_at(None, "open")).unwrap();
            playing_with_a_move("plain", creator, "joiner");
            let r = call(creator, "DELETE", "/api/games/plain", None);
            assert_eq!(r.status, 500, "{creator}: premise: the abandon deferred — {}", r.body);
            let base = format!("{root}rev7/abandoned/plain");
            assert_plan(&plan_of(&r), &format!("{base}.json"), &format!("{base}.table.json"));
        }
        // A browser save's root: its Archive object's caller's.
        let save = the_committed_save();
        for (caller, root) in [("joiner.test", ""), (RED, "test/")] {
            init_inner(&r2_at(Some(caller), "open")).unwrap();
            let r = call(caller, "POST", "/api/archive", Some(&save));
            assert_eq!(
                (r.status, r.body.as_str()),
                (500, r#"{"error":"the archive could not be written: deferred to the host"}"#),
                "{caller}: premise: the save deferred",
            );
            let segment = caller_segment(&User::parse(caller).unwrap());
            let base = format!("{root}browser/{segment}/rev7/over/{OWED}");
            assert_plan(&plan_of(&r), &format!("{base}.json"), &format!("{base}.table.json"));
        }
    }

    /// **The root and the path name ONE caller** (R1's L-1, fail-closed). The
    /// layer files a browser's save under its SENDER's segment,
    /// `browser/<caller_segment>/`, and this archive roots it by the address
    /// its Archive object was initialized with — so the two must be one
    /// caller. Probe P2's two mismatches, a test identity's save at another
    /// address's object and the reverse, are refused in the layer's words and
    /// defer nothing: the first would have filed a test identity's save in
    /// that address's training data, the second an address's save under
    /// `test/`. The anti-vacuity partner is the matching pair, whose plan is
    /// deferred under its root.
    #[test]
    fn a_browsers_save_sent_by_another_caller_than_the_objects_is_refused() {
        let _s = serial();
        let _p = pin(1_789_001_000);
        let save = the_committed_save();
        let refused = json!({
            "error": "the archive could not be written: a browser's save is filed by its Archive object's address, \
                      and this save was sent by another caller"
        })
        .to_string();
        for (object, sender) in [("someone@example.com", RED), (RED, "someone@example.com")] {
            init_inner(&r2(Some(object))).unwrap();
            let r = call(sender, "POST", "/api/archive", Some(&save));
            assert_eq!(
                (r.status, r.body.as_str(), r.archive.as_deref()),
                (500, refused.as_str(), None),
                "{sender} at {object}'s object: refused, and nothing deferred",
            );
        }
        init_inner(&r2(Some(RED))).unwrap();
        let r = call(RED, "POST", "/api/archive", Some(&save));
        assert_eq!(r.status, 500, "premise: the matching pair defers — {}", r.body);
        let base = format!("test/browser/red_at_t.test/rev7/over/{OWED}");
        assert_plan(&plan_of(&r), &format!("{base}.json"), &format!("{base}.table.json"));
    }

    /// The `test/` root's cell's twin for OUTCOMES: a browser's save meets each
    /// kind of receipt as the layer answers that outcome (spec §2.6: "a 201, a
    /// 200 `identical`, or a 409") — and its reply's `path` is relative to the
    /// root, as the committed `archive_post.json`'s is (spec §4). A receipt is
    /// SPENT by the put it answers: the same save once more is deferred afresh.
    #[test]
    fn a_browsers_save_answers_each_receipt_as_the_layer_answers_its_outcome() {
        let _s = serial();
        let _p = pin(1_789_000_800);
        let save = the_committed_save();
        let path = format!("browser/red_at_t.test/rev7/over/{OWED}.json");
        for (kind, detail, status, body) in [
            ("landed", "a key", 201, json!({"leaf": "over", "path": path, "identical": false})),
            ("identical", "a key", 200, json!({"leaf": "over", "path": path, "identical": true})),
            ("conflict", "a key", 409, json!({"error": "a different record is archived under this id"})),
            ("failed", "R2 is unreachable", 500, json!({"error": "the archive could not be written: R2 is unreachable"})),
        ] {
            init_inner(&r2(Some(RED))).unwrap();
            let plan = plan_of(&call(RED, "POST", "/api/archive", Some(&save)));
            assert_eq!(plan["record_key"], format!("test/{path}"), "{kind}: premise: the KEY is under the root");
            receipt_inner(key(&plan), &outcome(kind, detail)).unwrap();
            let r = call(RED, "POST", "/api/archive", Some(&save));
            let answered: Value = serde_json::from_str(&r.body).unwrap();
            assert_eq!((r.status, answered), (status, body), "{kind}");
            assert_eq!(r.archive, None, "{kind}: the receipt answered the put");
            let again = call(RED, "POST", "/api/archive", Some(&save));
            assert_eq!(
                (again.status, again.archive.as_deref().map(|_| key(&plan_of(&again)).to_string())),
                (500, Some(key(&plan).to_string())),
                "{kind}: the receipt is spent, so the same plan is deferred again",
            );
        }
    }

    /// **The owed-game-over FIXTURE** (plan review M3): the committed finished pair,
    /// cut back to the START of its trailing run of bot moves, so a cold rebuild
    /// pumps the game to its END. Task 13's pool cell needs these exact bytes, and
    /// only Rust can cut them (`state_at`, `stamp_terminal_digest` — the seed and the
    /// bot's agent seed are both above 2^53, so no TypeScript may touch them). The
    /// cut IS deterministic, so this cell is a DRIFT check: it rebuilds the partial
    /// and compares it with the committed file, and writes it only under
    /// `UPDATE_WORKER_FIXTURES=1`. The table is the committed pair's, under its OWN
    /// id `local-242a5429453c593c` (plan review m12), WITH the record's bot in its
    /// turn mirror — what a persist under the dealing build writes (M6.5c2, P-4),
    /// read through the layer's `bots::recorded`, the source `mirror_of` reads.
    /// The two files differ by exactly that field: the cell's premise holds the
    /// pair's table to a byte-for-byte round trip through `TableDoc`. Without the
    /// field the rebuild's persist would rewrite the table, and the pool's
    /// owed-game cells would lose their premises.
    #[test]
    fn the_owed_game_over_fixture_is_the_committed_pair_cut_at_its_last_bot_run() {
        let _s = serial();
        let full = read(web_fixtures().join("browser-played-2p.json"));
        let table = read(web_fixtures().join("browser-played-2p.table.json"));
        let doc: Value = serde_json::from_str(&table).unwrap();
        assert_eq!(
            (doc["id"].as_str(), doc["seats"][1]["kind"].as_str()),
            (Some(OWED), Some("bot")),
            "premise: the pair's own id, and the bot at seat 1",
        );
        let record: GameRecord = serde_json::from_str(&full).unwrap();
        let mut owed: yokohama_web::table::TableDoc = serde_json::from_str(&table).unwrap();
        assert_eq!(
            serde_json::to_string_pretty(&owed).unwrap(),
            table,
            "premise: the pair's table round-trips through `TableDoc` byte for byte, so the owed table differs by its `bot` alone",
        );
        owed.turn.as_mut().expect("a finished game's mirror").bot =
            yokohama_web::bots::recorded(&record).into_iter().next().map(|(_, d)| d);
        let table = serde_json::to_string_pretty(&owed).unwrap();
        let rules = Rules::standard();
        // Each event's mover: the player to move at the state before it, the
        // attribution the layer's undo reads (`undo_allowed` in
        // `crates/yokohama-web/src/api.rs`: `state_at(record, k).current_player`),
        // walked forward once.
        let mut state = state_at(&record, 0, &rules).expect("the pair replays");
        let mut movers = Vec::with_capacity(record.events.len());
        for k in 0..record.events.len() {
            movers.push(state.current_player);
            state = replay_from(&state, &record.events[k..=k], &rules).expect("the pair replays");
        }
        let bot = PlayerId::new(1);
        // The SMALLEST cut after which every event is the bot's: one past the
        // last event that is not.
        let n = movers.iter().rposition(|&p| p != bot).map_or(0, |k| k + 1);
        assert_eq!(n, 588, "premise: the trailing run of the bot's moves starts at 588 (re-review m-e)");
        assert!(n < movers.len(), "anti-vacuity: the run is not empty");
        let partial = partial_of(&full, n);
        let dir = worker_fixtures();
        if std::env::var_os("UPDATE_WORKER_FIXTURES").is_some() {
            std::fs::write(dir.join("owed-game-over.json"), &partial).unwrap();
            std::fs::write(dir.join("owed-game-over.table.json"), &table).unwrap();
        }
        assert_eq!(
            read(dir.join("owed-game-over.json")),
            partial,
            "the committed partial is the cut this cell builds (regenerate with UPDATE_WORKER_FIXTURES=1)",
        );
        assert_eq!(
            read(dir.join("owed-game-over.table.json")),
            table,
            "the committed table is the pair's with the record's bot in its mirror (regenerate with UPDATE_WORKER_FIXTURES=1)",
        );
    }

    /// Game over defers too, and the stamp completes it (spec §2.6's game-over
    /// site): the request that reaches it answers 200 — a move, or here the
    /// `GET …/record` whose cold rebuild pumps the game's last moves —
    /// `archived_at` stays unset, the plan rides out; the receipt then
    /// `stamp_archive` at the plan's reading stamps it.
    ///
    /// Its reply says whether the stamp landed (ruling I3): `stamped: false`
    /// under a `failed` receipt, `true` after a `landed` one — each with
    /// `over: true`, the game being over throughout.
    #[test]
    fn game_over_defers_its_put_and_stamp_archive_completes_it() {
        let _s = serial();
        init_inner(&r2(None)).unwrap();
        let _p = pin(1_789_000_400);
        // The game-over site meets the deferred put: the reply is the record
        // route's 200, and the plan rides out in it — under `over/`, at the
        // bucket's top, because the pair's creator is `local@browser`.
        let r = pump_owed_game_over();
        let plan = plan_of(&r);
        assert_plan(&plan, &format!("rev7/over/{OWED}.json"), &format!("rev7/over/{OWED}.table.json"));
        assert_eq!(plan["record"], r.body, "the archived bytes are the stored bytes the route served");
        assert_eq!(archived_at(OWED), None, "archived_at stays unset: a retry is owed");

        // The retry with no receipt yet puts the SAME plan again, at the same
        // reading — in its own reply — and stamps nothing.
        let s = stamp_archive_inner(OWED);
        assert_eq!((s.status, s.body.as_str()), (200, r#"{"stamped":false,"over":true}"#));
        assert_eq!(s.archive, r.archive, "the same plan: one reading, one stored pair");

        // A FAILED upload's receipt: no stamp, and the reply says so.
        receipt_inner(key(&plan), &outcome("failed", "R2 said no")).unwrap();
        let s = stamp_archive_inner(OWED);
        assert_eq!((s.status, s.body.as_str(), s.archive.as_deref()), (200, r#"{"stamped":false,"over":true}"#, None));
        assert_eq!(archived_at(OWED), None);

        // A LANDED one, at the reading the plan was made at: stamped at it.
        receipt_inner(key(&plan), &outcome("landed", &format!("rev7/over/{OWED}.json"))).unwrap();
        let s = stamp_archive_inner(OWED);
        assert_eq!(
            (s.status, s.body.as_str(), tagged(&s).as_str(), s.archive.as_deref()),
            (200, r#"{"stamped":true,"over":true}"#, "[]", None)
        );
        assert_eq!(archived_at(OWED), Some(1_789_000_400));

        // A second stamp puts nothing: no receipt is left, so a put would have
        // deferred a plan into this reply.
        let s = stamp_archive_inner(OWED);
        assert_eq!((s.status, s.body.as_str(), s.archive.as_deref()), (200, r#"{"stamped":true,"over":true}"#, None));

        // No game at all: the layer's own refusal, in its error shape.
        let s = stamp_archive_inner("nobody");
        assert_eq!((s.status, s.body.as_str()), (404, r#"{"error":"no game named \"nobody\""}"#));

        // A table with no game yet — open, or dealing — answers `handle`'s
        // `Err(Open | Dealing)` in the layer's error shape, `status` and all:
        // the very refusal the layer's own session route answers there.
        for (id, stage, refusal) in
            [("opening", Stage::Open, ApiError::open()), ("dealt", Stage::Dealing, ApiError::dealing())]
        {
            table(id, RED, BLUE, stage);
            let layer = call(RED, "GET", &format!("/api/games/{id}"), None);
            assert_eq!((layer.status, &layer.body), (409, &error_body(&refusal)), "premise: the layer's own refusal");
            let s = stamp_archive_inner(id);
            assert_eq!((s.status, s.body, s.archive), (layer.status, layer.body, None), "{id}");
        }
    }

    /// **Every reply carries the plan its OWN call deferred** (plan review M4):
    /// the wake's `restore_picker` and an `events` read each rebuild a cold
    /// handle, whose owed pump reaches the game-over site — and the plan rides
    /// out in THAT reply, the one whose batch the shell commits it in.
    #[test]
    fn the_wakes_restore_picker_and_an_events_read_carry_the_plan_their_pump_deferred() {
        let _s = serial();
        let _p = pin(1_789_000_700);
        let mut plans = Vec::new();
        for what in ["restore_picker", "events"] {
            init_inner(&r2(None)).unwrap();
            load_owed_game_over();
            let r = match what {
                "restore_picker" => restore_picker_inner(OWED, "null"),
                _ => events_inner(OWED, Some(LOCAL), ""),
            };
            assert_eq!(
                (r.status, tagged(&r)),
                (200, format!(r#"[{{"bump":"{OWED}"}}]"#)),
                "{what}: premise: the rebuild pumped the bot's run and persisted it — {}",
                r.body,
            );
            let plan = plan_of(&r);
            assert_plan(&plan, &format!("rev7/over/{OWED}.json"), &format!("rev7/over/{OWED}.table.json"));
            plans.push(plan);
        }
        assert_eq!(plans[0], plans[1], "one reading, one stored pair: one plan");
    }

    /// **`over` tells the undo case from a failed stamp** (the orchestrator's
    /// ruling on this task's first concern): `stamped: false` alone cannot,
    /// and the shell answers the two differently. A game no longer over at
    /// retry time — an undo at a one-person table after its plan was made —
    /// answers `{"stamped": false, "over": false}` and puts nothing, so the
    /// shell clears `archive_owed`. The anti-vacuity partner is the same
    /// reading and the same receipt over the game still over:
    /// `{"stamped": true, "over": true}`.
    ///
    /// **The undo cannot come through the API at this fixture.** At its end
    /// the BOT holds the acting seat, and the acting-seat fence refuses the
    /// one human's `undo` (asserted). So the cell runs what an admitted `undo`
    /// line runs in `api::input` — `Repl::step("undo")` on the resident
    /// handle, then `Games::persist` — inside `from_the_layer`, where its bump
    /// is drained.
    ///
    /// `over` is the HANDLE's reading, the one `archive_if_over` takes, and
    /// never the table's `turn` mirror, a cache that can be stale: under the
    /// committed table, whose mirror says the game is over, a record cut where
    /// the human is to move answers `over: false`.
    #[test]
    fn stamp_archive_says_whether_the_game_is_over_so_an_undo_is_not_a_failed_stamp() {
        let _s = serial();
        let _p = pin(1_789_000_900);
        for undone in [true, false] {
            init_inner(&r2(None)).unwrap();
            let plan = plan_of(&pump_owed_game_over());
            if undone {
                let refused = call(LOCAL, "POST", &format!("/api/games/{OWED}/input"), Some(r#"{"line":"undo"}"#));
                assert_eq!(refused.status, 403, "premise: the fence refuses the API's undo here — {}", refused.body);
                assert_eq!(stored("record", OWED).as_deref(), plan["record"].as_str(), "premise: and it undid nothing");
                let undo = crate::from_the_layer(|games| {
                    let handle = games.handle(OWED).expect("the resident game");
                    let mut repl = handle.lock().expect("a game");
                    let _ = repl.step("undo");
                    assert!(!repl.session.is_over(), "premise: the undo reopened the game");
                    games.persist(OWED, &repl).expect("the undo is written");
                    (200, String::new(), None)
                });
                assert_eq!(
                    (tagged(&undo), undo.archive),
                    (format!(r#"[{{"bump":"{OWED}"}}]"#), None),
                    "premise: the undo persisted and signalled as the API's does, and deferred nothing",
                );
            }
            receipt_inner(key(&plan), &outcome("landed", plan["record_key"].as_str().unwrap())).unwrap();
            let s = stamp_archive_inner(OWED);
            let want = if undone { r#"{"stamped":false,"over":false}"# } else { r#"{"stamped":true,"over":true}"# };
            assert_eq!((s.status, s.body.as_str(), s.archive.as_deref()), (200, want, None), "undone: {undone}");
            assert_eq!(archived_at(OWED), (!undone).then_some(1_789_000_900), "undone: {undone}");
        }

        // The handle's reading, never the mirror's: the committed table says
        // `over: true`, and the record under it is cut at 587, where the human
        // is to move — so the rebuild pumps nothing and the game is not over.
        init_inner(&r2(None)).unwrap();
        let table = read(worker_fixtures().join("owed-game-over.table.json"));
        let mirror: Value = serde_json::from_str(&table).unwrap();
        assert_eq!(mirror["turn"]["over"], json!(true), "premise: the table's mirror says the game is over");
        load_inner("table", OWED, &table).unwrap();
        load_inner("record", OWED, &partial_of(&read(web_fixtures().join("browser-played-2p.json")), 587)).unwrap();
        let s = stamp_archive_inner(OWED);
        assert_eq!(
            (s.status, s.body.as_str(), tagged(&s).as_str(), s.archive.as_deref()),
            (200, r#"{"stamped":false,"over":false}"#, "[]", None),
        );
    }

    /// `root`'s two refusals that no request reaches (the review's M2): a
    /// server game's table that names no creator, and a prefix the layer never
    /// passes. Each is a `Failed`, and defers NOTHING: no plan is keyed at the
    /// bucket's top, where a test identity's save would join the owner's
    /// training data. Driven on `put` directly, because the layer hands `put`
    /// only a stripped table and one of its two prefixes. The archive HAS a
    /// test caller, so neither refusal is the absent caller's, and a `root`
    /// that fell back to it would key the plan under `test/` instead.
    #[test]
    fn a_plan_the_archive_cannot_root_is_refused_and_deferred_nowhere() {
        let archive = ReceiptArchive::new(Some(RED.to_string()));
        for (prefix, table, sentence) in [
            ("", "{}", "the table document names no creator"),
            ("", r#"{"created_by":7}"#, "the table document names no creator"),
            ("operator/", r#"{"created_by":"red@t.test"}"#, r#"the archive keeps no root for the prefix "operator/""#),
        ] {
            let out = archive.put(prefix, Why::Over, "t", "{}", table);
            assert!(matches!(&out, Err(PutError::Failed(m)) if m == sentence), "{prefix:?} over {table}: {out:?}");
            assert!(archive.pending.lock().unwrap().is_none(), "{prefix:?} over {table}: nothing deferred");
        }
    }

    /// The table [`expire_answers_exactly_as_the_creators_delete_at_every_status`]
    /// builds for `status` on the current instance: its id and its creator.
    fn table_at(status: &str) -> (&'static str, &'static str) {
        let stage = match status {
            "open" => Stage::Open,
            "dealing" => Stage::Dealing,
            "playing" => Stage::Playing,
            "playing with a move" => Stage::Moved,
            "over" | "over and stamped" => {
                let plan = plan_of(&pump_owed_game_over());
                if status == "over and stamped" {
                    receipt_inner(key(&plan), &outcome("landed", plan["record_key"].as_str().unwrap())).unwrap();
                    assert_eq!(stamp_archive_inner(OWED).body, r#"{"stamped":true,"over":true}"#, "premise: stamped at game over");
                }
                return (OWED, LOCAL);
            }
            other => panic!("no arrangement for {other:?}"),
        };
        table("expiring", RED, BLUE, stage);
        ("expiring", RED)
    }

    /// `expire` IS the creator's own delete: same reply, same plan, same leaf, at
    /// every status (open, dealing, playing with and without moves, over).
    ///
    /// Each status is built on one instance and its documents snapshotted; the
    /// creator's `DELETE` runs there, and `expire` on a FRESH instance holding
    /// the same documents — at the same pinned reading, so a deferred save's
    /// plan is the same plan. The whole replies are compared: status, body,
    /// bumps, plan and picker.
    #[test]
    fn expire_answers_exactly_as_the_creators_delete_at_every_status() {
        let _s = serial();
        let _p = pin(1_789_000_500);
        for status in ["open", "dealing", "playing", "playing with a move", "over", "over and stamped"] {
            init_inner(&r2(None)).unwrap();
            let (id, creator) = table_at(status);
            let docs: Vec<(&str, String)> =
                ["table", "record", "chat"].into_iter().filter_map(|kind| stored(kind, id).map(|text| (kind, text))).collect();
            let deleted = call(creator, "DELETE", &format!("/api/games/{id}"), None);

            init_inner(&r2(None)).unwrap();
            for (kind, text) in &docs {
                load_inner(kind, id, text).unwrap();
            }
            let expired = expire_inner(id);
            assert_eq!(expired, deleted, "{status}: expire is the creator's DELETE, reply for reply");
            assert_eq!(stored("table", id).is_none(), expired.status == 204, "{status}: deleted exactly when answered 204");

            // Each status's premise — the table's status, and what the
            // creator's delete answered and owed — so no status is compared
            // vacuously.
            let doc: Value = serde_json::from_str(&docs[0].1).unwrap();
            let leaf = deleted.archive.as_ref().map(|_| plan_of(&deleted)["record_key"].as_str().unwrap().to_string());
            let want = match status {
                "open" => ("open", 204, None),
                "dealing" => ("dealing", 204, None),
                "playing" | "over and stamped" => ("playing", 204, None),
                "playing with a move" => ("playing", 500, Some("test/rev7/abandoned/expiring.json".to_string())),
                _ => ("playing", 500, Some(format!("rev7/over/{OWED}.json"))),
            };
            assert_eq!((doc["status"].as_str().unwrap(), deleted.status, leaf), want, "{status}: premise — {}", deleted.body);
        }

        // No table, so no creator to act as: the layer's own 404, the one a
        // DELETE of that id answers any caller.
        init_inner(&r2(None)).unwrap();
        let (expired, deleted) = (expire_inner("nobody"), call(RED, "DELETE", "/api/games/nobody", None));
        assert_eq!(expired, deleted);
        assert_eq!((expired.status, expired.body.as_str()), (404, r#"{"error":"no game named \"nobody\""}"#));
    }

    /// A `conflict` receipt is final where the layer says so: the abandon is
    /// refused naming the archive; the game-over stamp is not written.
    ///
    /// The receipt's `detail` differs from the record key on purpose: the
    /// conflict the layer words is `PutError::Conflict { key: record_key, .. }`,
    /// the key the plan was made for.
    #[test]
    fn a_conflict_receipt_refuses_the_abandon_and_leaves_game_over_unstamped() {
        let _s = serial();
        let _p = pin(1_789_000_600);
        // The abandon: refused in the layer's sentence, the conflict inside it,
        // and nothing deleted.
        init_inner(&r2(None)).unwrap();
        playing_with_a_move("clash", RED, BLUE);
        let plan = plan_of(&call(RED, "DELETE", "/api/games/clash", None));
        receipt_inner(key(&plan), &outcome("conflict", "the key holds other bytes")).unwrap();
        let r = call(RED, "DELETE", "/api/games/clash", None);
        let sentence = "the archive could not be written (test/rev7/abandoned/clash.json holds a different record — \
                        a game is archived once) — the table was not deleted";
        assert_eq!((r.status, r.body.clone()), (500, json!({"error": sentence}).to_string()));
        assert_eq!((tagged(&r).as_str(), r.archive.as_deref()), ("[]", None), "the receipt answered the put");
        assert!(stored("table", "clash").is_some() && stored("record", "clash").is_some(), "nothing deleted");

        // Game over: the stamp is not written.
        init_inner(&r2(None)).unwrap();
        let plan = plan_of(&pump_owed_game_over());
        receipt_inner(key(&plan), &outcome("conflict", "the key holds other bytes")).unwrap();
        let s = stamp_archive_inner(OWED);
        assert_eq!((s.status, s.body.as_str(), s.archive.as_deref()), (200, r#"{"stamped":false,"over":true}"#, None));
        assert_eq!(archived_at(OWED), None, "nothing stamped");
    }

    /// **R-6.5c2.11 through the receipt** (M6.5c2, spec §2.5; review M3; Review
    /// Focus 3): a `conflict` receipt carries the record the key held, and the
    /// stamp's put meets it as `PutError::Conflict { held }` — the layer's
    /// same-game rule reads it in Rust, never the shell. A record of THIS game
    /// (the same seed, configuration and choices) stamps: here the committed
    /// pair cut short, whose events differ from the stored record's, because
    /// the rule compares a game's setup and never its moves — in the race it
    /// is the game's earlier completion (the pool's refinish cells). A held
    /// text that does not parse as a record (that same record cut off by its
    /// last byte), another game's record, or a conflict whose held record the
    /// shell did not read stays unstamped, `over: true`. Mutants: `held`
    /// dropped between the receipt and the put (the first row unstamped);
    /// `held` read as always the same game (the other rows stamped).
    #[test]
    fn a_conflict_receipt_carrying_a_record_of_this_game_stamps() {
        let _s = serial();
        let _p = pin(1_789_001_100);
        let full = read(web_fixtures().join("browser-played-2p.json"));
        let same_game = partial_of(&full, 587);
        let cut_off = &same_game[..same_game.len() - 1];
        let other_game = {
            let mut record: GameRecord = serde_json::from_str(&same_game).unwrap();
            record.seed = record.seed.wrapping_add(1);
            serde_json::to_string(&record).unwrap()
        };
        assert!(serde_json::from_str::<GameRecord>(cut_off).is_err(), "premise: the cut-off text is no record");
        for (what, held, stamped) in [
            ("a record of this game", Some(same_game.as_str()), true),
            ("a text that is no record", Some(cut_off), false),
            ("another game's record", Some(other_game.as_str()), false),
            ("a record the shell did not read", None, false),
        ] {
            init_inner(&r2(None)).unwrap();
            let plan = plan_of(&pump_owed_game_over());
            assert_ne!(plan["record"].as_str(), held, "{what}: premise: the key holds OTHER bytes than the plan's");
            let receipt = match held {
                Some(text) => json!({"outcome": "conflict", "detail": plan["record_key"], "held": text}),
                None => json!({"outcome": "conflict", "detail": plan["record_key"]}),
            };
            receipt_inner(key(&plan), &receipt.to_string()).unwrap();
            let s = stamp_archive_inner(OWED);
            let want = if stamped { r#"{"stamped":true,"over":true}"# } else { r#"{"stamped":false,"over":true}"# };
            assert_eq!((s.status, s.body.as_str(), s.archive.as_deref()), (200, want, None), "{what}");
            assert_eq!(archived_at(OWED), stamped.then_some(1_789_001_100), "{what}");
        }
    }

    /// The held record rides a CONFLICT and nothing else (M6.5c2): the shell
    /// sets it only where the key held other bytes, and a landed, identical or
    /// failed receipt carrying a whole game is a shell defect, refused rather
    /// than ignored — in a sentence naming no byte of the record. `held` is
    /// the record's TEXT, so a number there is refused too. Mutant: `held`
    /// accepted beside any outcome.
    #[test]
    fn a_receipt_carries_the_held_record_only_beside_a_conflict() {
        let _s = serial();
        init_inner(&r2(None)).unwrap();
        receipt_inner("a-key", r#"{"outcome":"conflict","detail":"x","held":"{}"}"#).unwrap();
        receipt_inner("a-key", r#"{"outcome":"conflict","detail":"x"}"#).unwrap();
        for kind in ["landed", "identical", "failed"] {
            let err = receipt_inner("a-key", &json!({"outcome": kind, "detail": "x", "held": "{\"seed\":1}"}).to_string())
                .unwrap_err();
            assert_eq!(err, "a receipt carries the held record only beside a conflict", "{kind}");
        }
        assert!(receipt_inner("a-key", r#"{"outcome":"conflict","detail":"x","held":5}"#).is_err(), "held is the record's TEXT");
    }

    /// **The layer's log reaches the reply — but a DEFERRED put is not a line**
    /// (M6.5c2, spec §2.6). On this host every game-over put is deferred first:
    /// `ReceiptArchive::put` records the plan and answers `Failed(DEFERRED)`,
    /// which the layer logs as `archive-over-refused` — true of the put and
    /// false of the save, whose plan rides this very reply for the shell to
    /// upload and log itself (`archive-landed` and the rest). So `ShellLog`
    /// drops that one line, by this crate's OWN word, and forwards every other:
    /// the retry that meets a `conflict` receipt is the layer's refusal, carried
    /// whole. Mutants: the drop removed (the first reply carries a line); every
    /// line dropped, or the drain left out of `from_the_layer` (the second
    /// carries none).
    #[test]
    fn a_deferred_put_is_not_a_line_and_a_refused_retry_is_the_layers() {
        let _s = serial();
        let _p = pin(1_789_000_700);
        init_inner(&r2(None)).unwrap();
        let r = pump_owed_game_over();
        let plan = plan_of(&r);
        assert!(r.logs.is_empty(), "the deferral is its plan's to report: {:?}", r.logs);

        receipt_inner(key(&plan), &outcome("conflict", "the key holds other bytes")).unwrap();
        let s = stamp_archive_inner(OWED);
        assert_eq!(s.body, r#"{"stamped":false,"over":true}"#, "premise: the conflict left the save unstamped");
        let error = format!("rev7/over/{OWED}.json holds a different record — a game is archived once");
        assert_eq!(
            s.logs,
            vec![crate::log::LogLine {
                event: yokohama_web::log::OVER_PUT_REFUSED,
                fields: json!({ "id": OWED, "error": &error }),
            }],
        );
        // Through the getter the shell reads: one `{"event", "fields"}` per line.
        let text = crate::Reply::from(s.clone()).logs();
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap(),
            json!([{ "event": "archive-over-refused", "fields": { "id": OWED, "error": &error } }]),
        );
    }

    /// **The layout's refusal reaches this archive as a FAILURE, and no plan is
    /// recorded** (M6.5c2, spec §2.1's defence in depth; review R2-m3). No
    /// caller can hand the layer a `.` or `..` component — the key grammar and
    /// `is_safe_id` close both — so the put is called directly, as the layer
    /// would call it with an id that slipped through: a `Failed`, never a
    /// conflict the shell would read as "a different record is there", and
    /// never a plan it would upload.
    #[test]
    fn a_path_the_layout_refuses_is_a_failure_and_records_no_plan() {
        let archive = ReceiptArchive::new(None);
        let table = json!({ "created_by": RED }).to_string();
        let put = archive.put("", Why::Over, "..", "{}", &table);
        assert!(matches!(&put, Err(PutError::Failed(why)) if why.contains("..")), "{put:?}");
        assert!(archive.pending.lock().unwrap().is_none(), "nothing deferred, so nothing uploads");
        // Anti-vacuity: the same put under a safe id IS deferred, so the arm
        // above read the layout's refusal and not an archive that plans nothing.
        let _ = archive.put("", Why::Over, "sim-1", "{}", &table);
        assert!(archive.pending.lock().unwrap().is_some(), "a safe id is planned");
    }

    /// `receipt` takes exactly the outcome the shell writes (§"Names shared
    /// across tasks") — an OBJECT with one of the four words and a detail — and
    /// only where a put can have been deferred; and `init` takes an archive
    /// caller only as a key — an email or (M6.5c2) a plain name.
    #[test]
    fn a_receipt_is_the_outcome_object_and_needs_the_archive_on() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let err = receipt_inner("a-key", &outcome("landed", "x")).unwrap_err();
        assert!(err.contains("YOKOHAMA_WEB_ARCHIVE"), "{err}");

        init_inner(&r2(None)).unwrap();
        for bad in [
            r#"{"outcome":"Landed","detail":"x"}"#,
            r#"{"outcome":"landed"}"#,
            r#"{"outcome":"landed","detail":"x","at":1}"#,
            r#"["landed","x"]"#,
            "landed",
        ] {
            assert!(receipt_inner("a-key", bad).is_err(), "{bad} is not an outcome");
        }
        for kind in ["landed", "identical", "conflict", "failed"] {
            receipt_inner("a-key", &outcome(kind, "x")).unwrap_or_else(|e| panic!("{kind}: {e}"));
        }

        let err = init_inner(&r2(Some(".."))).unwrap_err();
        assert!(err.contains("caller"), "{err}");
        // Anti-vacuity, and the open door's premise (M6.5c2): a plain name is a
        // caller the key grammar accepts.
        init_inner(&r2(Some("  Ben   Castle "))).expect("a plain name is a caller");
    }
}
