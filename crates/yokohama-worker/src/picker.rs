//! **The picker across hibernation** (M6.5c, R-6.5c.7; spec §2.4): an open
//! multi-step picker survives a new module instance because the input lines
//! that built it are RE-FED. The mechanism is host-local; the layer is unchanged.
//!
//! The layer keeps an open picker only in the resident `Repl`, as `repl.sub`
//! (`yokohama_cli::repl::SubPrompt`). The record does not hold it, so a rebuilt
//! handle starts with the picker closed — and a table object is rebuilt on
//! almost every turn (R-6.5c.7). Serializing `SubPrompt` was declined: it has
//! no derives, so it would change the CLI and store a format coupled to an
//! internal type. Instead this module keeps, per table, the lines the picker
//! has taken since it opened. They ride out in `Reply.picker` for the shell to
//! store as the table's `picker` row, and [`restore_picker_inner`] re-feeds
//! them through the layer's own `api::input` when a new instance wakes.
//!
//! **The rule**, applied by [`around`] after every dispatch at or below
//! `/api/games/{id}`, and by [`watching`] across every other call on a game
//! (`crate::Scope`):
//!
//! - **A changed record clears the list**: a move, an undo, a finalize, a cold
//!   bot pump on a GET, a delete. A chat post, an unlock, a keep that does not
//!   finalize and a watcher arriving change no record, and leave it.
//! - **After a 200 `POST …/input` that left the record's bytes unchanged**, a
//!   picker still open appends the line as `(user, line, expect)`, and a closed
//!   one clears the list.
//! - **The 65th line clears the list**, and nothing more is recorded until
//!   that picker closes, so nothing partial is ever re-fed.
//!
//! **A list is only ever re-fed at the exact record it was built at, under the
//! code it was built under.** The row carries that record's SHA-256 and the
//! [`CODE`] it was built under — `build.rs`'s digest of the engine's, the
//! CLI's and the layer's sources, the code that gives a line its meaning (the
//! orchestrator's ruling I-85) — and a restore at any other record, or under
//! any other code, clears the list without re-feeding a line. At that record
//! and under that code, every line does again what it did when it was recorded
//! — it left the picker open and applied nothing — so no re-fed line can apply
//! a move. A re-fed line that fails or closes the picker clears the list, and
//! so does one that changes the record, whose move stays made
//! ([`restore_picker_inner`]).
//!
//! `Reply.picker` is the list's JSON when it grew, and `"[]"` when a list
//! existed and was cleared, which the shell reads as "delete the row". It is
//! `None` otherwise — including a record change with no list to clear — so the
//! shell never writes a row for nothing (the orchestrator's rulings I-21 and
//! S-04).

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use yokohama_cli::repl::SubPrompt;
use yokohama_web::api::{self, ApiRequest, ApiResponse};
use yokohama_web::games::{GameError, Games};
use yokohama_web::identity::User;
use yokohama_web::store::Doc;
use yokohama_web::ApiError;

use crate::{error_body, from_the_layer, sha256_hex, ReplyData, Scope};

/// The most lines a list holds: the line after them CLEARS it (spec §2.4).
const MAX_LINES: usize = 64;

/// `Reply.picker` for a list that existed and was cleared: the shell deletes
/// the table's `picker` row.
const CLEARED: &str = "[]";

/// This build's CODE: `build.rs`'s digest of the engine's, the CLI's and the
/// layer's sources, the code that gives a re-fed line its meaning (the
/// orchestrator's ruling I-85). Every list this module writes carries it. The
/// engine is among them because a line's NUMBER is the engine's order (R1's
/// R-1): a pure reorder of the legal list moves no `SEMANTICS_REV` — its own
/// rule forbids the bump, since no record replays differently — and a data
/// edit moves only `rules_hash`, across which a record still loads.
pub(crate) const CODE: &str = env!("YOKOHAMA_WORKER_PICKER_CODE");

/// Every table's picker, as this instance knows it. `None` until
/// [`crate::init_inner`], which replaces it with an empty map each time it
/// rebuilds the `Games` — a new instance knows no picker until one is restored
/// or a line opens one.
pub(crate) static PICKERS: Mutex<Option<HashMap<String, Picker>>> = Mutex::new(None);

/// What this instance knows of one table's picker. NO ENTRY means the picker
/// is closed, so a line that leaves one open is the line that OPENED it, and
/// begins a list.
pub(crate) enum Picker {
    /// Open, with every line it has taken in the list, the opening line first.
    Lines(PickerList),
    /// Open, with its list gone — cleared at its 65th line, or by a line that
    /// could not join it — so nothing more is recorded until it closes. A list
    /// begun partway through a picker would re-feed its first line at the main
    /// prompt, where it means something else.
    Dropped,
}

/// The stored `picker` row (spec §2.4): the lines to re-feed, the record they
/// were built at, and the code they were built under.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickerList {
    /// The [`CODE`] the list was built under. Absent from a row an earlier
    /// build wrote, before a row carried one — every row the fourteenth to the
    /// eighteenth deployments wrote — which a restore clears: no code vouches
    /// for its meaning (the orchestrator's rulings I-85 and I-89).
    #[serde(default)]
    pub code: Option<String>,
    /// The SHA-256 of the bytes of the record the list was built at, in
    /// lowercase hex.
    pub record: String,
    /// Every line since the picker opened, the opening line first.
    pub lines: Vec<PickerLine>,
}

/// One line a picker took: the caller, as the layer normalized the address,
/// and the input's two fields. `expect` is kept, so a re-fed line is checked
/// exactly as the client's was.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickerLine {
    pub user: String,
    pub line: String,
    pub expect: Option<String>,
}

/// The layer's `dispatch`, with spec §2.4's step around it. For a path at or
/// below `/api/games/{id}` the record is read before and after, and the rule
/// applied; `/api/games` itself — the listing and the create — names no table,
/// and neither does any other path. Answers the layer's response and
/// `Reply.picker`.
pub(crate) fn around(
    games: &Games,
    req: &ApiRequest<'_>,
    caller: Option<&User>,
    dispatch: impl FnOnce() -> ApiResponse,
) -> (ApiResponse, Option<String>) {
    let Some((id, below)) = game_of(req.path) else {
        return (dispatch(), None);
    };
    let before = record_of(games, id);
    let res = dispatch();
    let after = record_of(games, id);
    let changed = before != after;
    let picker = if req.method == "POST" && below == "/input" && res.status == 200 {
        after_input(games, id, changed, after.as_deref(), line_of(req, caller))
    } else {
        clear_if(changed, id)
    };
    (res, picker)
}

/// Spec §2.4's rule for a changed record, stated once for [`around`] and
/// [`watching`]: `id`'s entry removed, and `"[]"` when it held a list, whose
/// row the shell then deletes.
fn clear_if(changed: bool, id: &str) -> Option<String> {
    if !changed {
        return None;
    }
    with_pickers(|pickers| pickers.remove(id)).and_then(cleared)
}

/// The rule for a call that is no dispatch ([`crate::Scope::Table`]; R1's
/// cleanup debt 1): `id`'s record read before and after `call`, and on a
/// change the entry cleared ([`clear_if`]). `events`, `stamp_archive` and
/// `expire`'s arm with no creator reach `Games::handle` outside [`around`],
/// and a cold handle one of them rebuilds pumps an owed bot move and persists
/// it — a record change after which the picker is closed, since a rebuilt
/// `Repl` starts with it closed. [`restore_picker_inner`] answers under this
/// rule too, and its own `Reply.picker` wins over this one's
/// (`crate::from_the_layer`).
///
/// **The residual, named:** a rebuild at the SAME record — no bot owed, so no
/// pump — leaves a stale entry this rule cannot see; only an entry tied to the
/// handle it was built on would close it. It is unreachable here: a table
/// object holds one game, which `Games` evicts only past 64 resident handles,
/// on a conflicting write — a `MemStore` with one writer has none — or at its
/// delete, which removes the record too, a change this rule sees.
pub(crate) fn watching<T>(games: &Games, id: &str, call: impl FnOnce() -> T) -> (T, Option<String>) {
    let before = record_of(games, id);
    let out = call();
    let changed = record_of(games, id) != before;
    (out, clear_if(changed, id))
}

/// The rule after a 200 `POST …/input`. `line` is `None` only when this host
/// could not record the line — a caller, or a body, that the layer had just
/// accepted — and then no list could hold every line the picker took.
fn after_input(
    games: &Games,
    id: &str,
    changed: bool,
    record: Option<&str>,
    line: Option<PickerLine>,
) -> Option<String> {
    let open = is_open(games, id);
    with_pickers(|pickers| {
        let prior = pickers.remove(id);
        if !open {
            return prior.and_then(cleared);
        }
        let list = match (prior, line, record) {
            // The picker the list was built for, at the record it was built at:
            // the line joins the list.
            (Some(Picker::Lines(mut list)), Some(line), _) if !changed => {
                list.lines.push(line);
                list
            }
            // No entry is a closed picker, so this line opened one, and begins
            // a list.
            (None, Some(line), Some(record)) if !changed => {
                PickerList { code: Some(CODE.to_string()), record: sha256_hex(record), lines: vec![line] }
            }
            // Open, but a list could not hold every line it has taken.
            (prior, ..) => return dropped(pickers, id, prior),
        };
        if list.lines.len() > MAX_LINES {
            return dropped(pickers, id, Some(Picker::Lines(list)));
        }
        let json = serde_json::to_string(&list).expect("a picker list serializes");
        pickers.insert(id.to_string(), Picker::Lines(list));
        Some(json)
    })
}

/// Mark `id`'s open picker [`Picker::Dropped`], answering `"[]"` when that
/// clears a list.
fn dropped(pickers: &mut HashMap<String, Picker>, id: &str, prior: Option<Picker>) -> Option<String> {
    pickers.insert(id.to_string(), Picker::Dropped);
    prior.and_then(cleared)
}

/// `"[]"` for a list that existed, whose row the shell deletes, and `None` for
/// anything else — a table with no row is never told to delete one.
fn cleared(prior: Picker) -> Option<String> {
    matches!(prior, Picker::Lines(_)).then(|| CLEARED.to_string())
}

/// `restore_picker(id, row)` (spec §2.4): on a NEW instance, after `load`,
/// rebuild `id`'s picker from its stored `picker` row — the row's JSON, or
/// `"null"` when there is none.
///
/// **It ALWAYS builds the table's handle first.** That is the replay, so a
/// record that cannot be replayed fails here, at the wake; and a handle rebuilt
/// with a bot to move pumps that move and persists it before any row is read.
/// It answers through `from_the_layer`, the drain `dispatch` answers through,
/// so that pump's bumps are in this reply.
///
/// Then the row decides, and the answer is a 200 with no body:
///
/// - **no row**: nothing to restore, and `picker` is `None`;
/// - **a row that is not a list** — not JSON, not the list's object (serde's
///   positional array form is not), a key the list does not have, no lines,
///   or more than a list holds — is NO list:
///   `picker` is `"[]"`, so the shell deletes it. A malformed row is never a
///   panic: a lost picker costs a stale click a 409, and must never poison the
///   table (the orchestrator's ruling I-21);
/// - **a list built under other code** — its `code` not this build's
///   `CODE`, or absent from a row an earlier build wrote — is cleared before
///   a line is read, and `Reply.dropped` names the two codes for the shell's
///   `picker-dropped` line: under other code a line may mean something else,
///   and re-fed it could make a move nobody chose (the orchestrator's ruling
///   I-85). A picker open across a deploy that changes that code starts over,
///   and the player's next click is `Input::expect`'s 409;
/// - **a list built at another record** is cleared without re-feeding a line;
/// - **otherwise each line is re-fed** through `api::input` as its user, the
///   replies discarded. A line that fails, or leaves the picker closed,
///   clears the list and closes the picker, as a restart leaves it: nothing
///   was applied. A line that CHANGES the record has made a move — the layer
///   persisted it, and its bumps are drained into this reply, which the shell
///   commits like any other — and it clears the list and closes the picker
///   too. At the record and under the code the row names no line does that:
///   none changed the record when it was recorded. A list re-fed whole is
///   kept: `picker` is `None`, and the row stays as it is.
///
/// At a table with no game yet, where `handle()` answers `Err(Open | Dealing)`,
/// it answers 200 with nothing to report (plan review m13). Any other refusal
/// is the layer's own, in the API's error shape, and leaves the row alone.
pub fn restore_picker_inner(id: &str, row: &str) -> ReplyData {
    let mut dropped = None;
    let mut reply = from_the_layer(Scope::Table(id), |games| match games.handle(id) {
        Ok(_) => {
            let (picker, gone) = restore(games, id, row);
            dropped = gone;
            (200, String::new(), picker)
        }
        Err(GameError::Open | GameError::Dealing) => (200, String::new(), None),
        Err(e) => {
            let e = ApiError::from(e);
            (e.status.as_u16(), error_body(&e), None)
        }
    });
    reply.dropped = dropped;
    reply
}

/// `Reply.dropped`: the code a cleared row was built under — `None` for a row
/// that names none — and this build's.
#[derive(Serialize)]
struct Dropped<'a> {
    stored: Option<&'a str>,
    running: &'a str,
}

/// The row's verdict, once `id`'s handle is built: `Reply.picker`, and
/// `Reply.dropped` when the row was built under other code.
fn restore(games: &Games, id: &str, row: &str) -> (Option<String>, Option<String>) {
    let list = match serde_json::from_str::<Value>(row) {
        Ok(Value::Null) => return (None, None),
        Ok(value) => as_list(value),
        Err(_) => None,
    };
    // Under other code a line may mean something else: the list is cleared
    // before one is read.
    if let Some(stored) = list.as_ref().map(|list| list.code.as_deref()).filter(|&code| code != Some(CODE)) {
        let dropped = serde_json::to_string(&Dropped { stored, running: CODE }).expect("a dropped row's codes serialize");
        close(games, id);
        with_pickers(|pickers| pickers.remove(id));
        return (Some(CLEARED.to_string()), Some(dropped));
    }
    let record = record_of(games, id);
    let kept = list.filter(|list| {
        (1..=MAX_LINES).contains(&list.lines.len())
            && record.as_deref().is_some_and(|r| sha256_hex(r) == list.record)
            && list.lines.iter().all(|line| refeed(games, id, line, record.as_deref()))
    });
    match kept {
        Some(list) => {
            with_pickers(|pickers| pickers.insert(id.to_string(), Picker::Lines(list)));
            (None, None)
        }
        None => {
            close(games, id);
            with_pickers(|pickers| pickers.remove(id));
            (Some(CLEARED.to_string()), None)
        }
    }
}

/// `value` as a [`PickerList`], read ONLY from the object form `Reply.picker`
/// writes, every line an object too. serde's derived `Deserialize` also reads a
/// struct from a positional ARRAY, a form `deny_unknown_fields` does not
/// govern, so an array is refused here before serde reads it — `create.rs`'s
/// precedent for a create body.
fn as_list(value: Value) -> Option<PickerList> {
    let object_form = matches!(&value, Value::Object(row)
        if row.get("lines").and_then(Value::as_array).is_some_and(|lines| lines.iter().all(Value::is_object)));
    if !object_form {
        return None;
    }
    serde_json::from_value(value).ok()
}

/// Re-feed one line through the layer's own `api::input`, as its user, the
/// reply discarded. `true` only if the line answered, left the record as it
/// was, and left the picker open — what the line did when it was recorded.
fn refeed(games: &Games, id: &str, line: &PickerLine, record: Option<&str>) -> bool {
    let Ok(user) = User::parse(&line.user) else {
        return false;
    };
    let input = api::Input { line: line.line.clone(), expect: line.expect.clone() };
    api::input(games, &user, id, &input).is_ok() && record_of(games, id).as_deref() == record && is_open(games, id)
}

/// Close `id`'s picker, as a new instance's is closed: `repl.sub` is where an
/// open picker lives, and `SubPrompt::None` is what a rebuilt `Repl` starts
/// with. So the client's next picker click, which carries `expect`, is that
/// check's 409, and never a click into whatever a partial re-feed left open.
fn close(games: &Games, id: &str) {
    if let Ok(handle) = games.handle(id) {
        handle.lock().expect("a game").sub = SubPrompt::None;
    }
}

/// Whether `id`'s picker is open (`repl.sub`, spec §2.4). Called only where
/// the handle is RESIDENT — after a 200 `input`, whose route built it, and in a
/// restore, which built it first — so it never rebuilds one, which could pump a
/// bot's move.
fn is_open(games: &Games, id: &str) -> bool {
    games.handle(id).is_ok_and(|handle| !matches!(handle.lock().expect("a game").sub, SubPrompt::None))
}

/// The line a 200 `POST …/input` took: its caller, and its body read through
/// the layer's own `api::parse_json`, the parse that has just accepted it.
fn line_of(req: &ApiRequest<'_>, caller: Option<&User>) -> Option<PickerLine> {
    let input = api::parse_json::<api::Input>(req.content_type, req.body).ok()?;
    Some(PickerLine { user: caller?.as_str().to_string(), line: input.line, expect: input.expect })
}

/// A path at or below `/api/games/{id}`: the id, and what follows it — `""` for
/// the game itself, `"/input"` for the move. `None` for `/api/games` itself and
/// for every path outside it.
fn game_of(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/api/games/")?;
    let (id, below) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    (!id.is_empty()).then_some((id, below))
}

/// `id`'s stored record. `None` when there is none, or none the store can
/// read — and no list can have been built at either.
fn record_of(games: &Games, id: &str) -> Option<String> {
    games.read_document(Doc::Record, id).ok().flatten()
}

fn with_pickers<T>(f: impl FnOnce(&mut HashMap<String, Picker>) -> T) -> T {
    let mut guard = PICKERS.lock().expect("the host's pickers");
    f(guard.as_mut().expect("init() before any call into the layer"))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use yokohama_web::games::GameError;

    use super::CODE;
    use crate::tests::{call, config, serial, tagged, CREATE};
    use crate::{
        create_inner, events_inner, expire_inner, init_inner, load_inner, read_inner, restore_picker_inner,
        stamp_archive_inner, ReplyData, GAMES,
    };

    const RED: &str = "red@t.test";
    const BLUE: &str = "blue@t.test";
    const WATCHER: &str = "watcher@t.test";

    /// One `POST …/input`, with `expect` when the client would send it.
    fn input(user: &str, id: &str, line: &str, expect: Option<&str>) -> ReplyData {
        let body = match expect {
            Some(kind) => json!({ "line": line, "expect": kind }),
            None => json!({ "line": line }),
        };
        call(user, "POST", &format!("/api/games/{id}/input"), Some(&body.to_string()))
    }

    fn body(r: &ReplyData) -> Value {
        serde_json::from_str(&r.body).unwrap_or_else(|e| panic!("{e}: {}", r.body))
    }

    /// The `line` of each entry in the reply's picker list — `Reply.picker` read
    /// as the stored row's object, `{"record", "lines"}`.
    fn lines(r: &ReplyData) -> Vec<String> {
        let row: Value = serde_json::from_str(r.picker.as_deref().expect("the reply carries a picker list")).unwrap();
        row["lines"].as_array().expect("a list of lines").iter().map(|l| l["line"].as_str().unwrap().to_string()).collect()
    }

    /// SHA-256 of a record's bytes, in hex — computed HERE, so a cell never
    /// checks the module's digest against itself.
    fn digest_of(record: &str) -> String {
        format!("{:x}", Sha256::digest(record.as_bytes()))
    }

    fn record(id: &str) -> String {
        read_inner("record", id).unwrap().expect("a stored record")
    }

    fn table(id: &str) -> String {
        read_inner("table", id).unwrap().expect("a stored table")
    }

    /// A NEW instance, holding only what the wake path `load`s — the stored
    /// table and record — so nothing resident survives: no handle, no picker.
    fn fresh(id: &str, table: &str, record: &str) {
        init_inner(&config("off")).unwrap();
        load_inner("table", id, table).unwrap();
        load_inner("record", id, record).unwrap();
    }

    /// The MEASURED PlaceA arrangement (the plan's review, seed 20260831; the
    /// handover's trap 7), every premise asserted as it runs: a 2P table from
    /// `CREATE`, blue joined and both seats kept, then red's `1` APPLIES the
    /// opening move and red's second `1` opens the PlaceA picker with nothing
    /// selected. Answers the second `1`'s reply.
    ///
    /// Two of its replies change the record with no list to clear — the
    /// finalizing keep and the opening move — and carry no picker: `"[]"` is
    /// only ever the deletion of a row that exists (the orchestrator's rulings
    /// I-21 + S-04).
    fn at_place_a(id: &str) -> ReplyData {
        at_place_a_from(id, CREATE, r#"{"seat":1}"#)
    }

    /// [`at_place_a`] from another create body and join body: a LOCKED table's,
    /// whose joiner sends the password. The seated players' flow is otherwise
    /// the same.
    fn at_place_a_from(id: &str, create: &str, join: &str) -> ReplyData {
        let r = create_inner(Some(RED), create.as_bytes(), id, Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        let r = call(BLUE, "POST", &format!("/api/games/{id}/join"), Some(join));
        assert_eq!(r.status, 200, "{}", r.body);
        let r = call(RED, "POST", &format!("/api/games/{id}/keep"), Some(r#"{"seat":0,"card":1}"#));
        assert_eq!(r.status, 200, "{}", r.body);
        let r = call(BLUE, "POST", &format!("/api/games/{id}/keep"), Some(r#"{"seat":1,"card":1}"#));
        assert_eq!(r.status, 200, "{}", r.body);
        assert!(read_inner("record", id).unwrap().is_some(), "premise: the last keep finalized the game");
        assert_eq!(r.picker, None, "the finalize changed the record, and there was no list to clear");
        let r = input(RED, id, "1", None);
        let s = body(&r);
        assert_eq!(
            (r.status, &s["events"], &s["prompt"]["kind"]),
            (200, &json!(1), &json!("options")),
            "premise: the first `1` APPLIES the opening move — {}",
            r.body,
        );
        assert_eq!(r.picker, None, "a move with no list to clear");
        let r = input(RED, id, "1", None);
        let s = body(&r);
        assert_eq!(
            (r.status, &s["events"], &s["prompt"]["kind"], &s["prompt"]["selected"]),
            (200, &json!(1), &json!("place_a"), &json!([])),
            "premise: the second `1` opens the PlaceA picker and applies nothing — {}",
            r.body,
        );
        r
    }

    /// An open picker's lines grow the list — the row names the record it was
    /// built at, and each line's caller and `expect` — and a MOVE, which changes
    /// the record, clears it.
    #[test]
    fn an_open_picker_grows_the_list_and_a_move_clears_it() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let r = at_place_a("grow");
        let built_at = digest_of(&record("grow"));
        let want = format!(
            r#"{{"code":"{CODE}","record":"{built_at}","lines":[{{"user":"red@t.test","line":"1","expect":null}}]}}"#
        );
        assert_eq!(r.picker.as_deref(), Some(want.as_str()), "the line that OPENED the picker begins the list");
        let r = input(RED, "grow", "A09", Some("place_a"));
        assert_eq!((r.status, &body(&r)["prompt"]["selected"]), (200, &json!([9])), "premise: Customs is selected");
        assert_eq!(lines(&r), ["1", "A09"]);
        let row: Value = serde_json::from_str(r.picker.as_deref().unwrap()).unwrap();
        assert_eq!(row["record"], json!(built_at), "still the record the list was built at");
        assert_eq!(
            row["lines"][1],
            json!({ "user": RED, "line": "A09", "expect": "place_a" }),
            "a line's `expect` is kept, so a re-fed line is checked as the client's was",
        );
        let r = input(RED, "grow", "confirm", Some("place_a"));
        assert_eq!((r.status, &body(&r)["events"]), (200, &json!(2)), "premise: `confirm` APPLIES the placement");
        assert_ne!(digest_of(&record("grow")), built_at, "premise: the record changed");
        assert_eq!(r.picker.as_deref(), Some("[]"), "a move clears the list, and the shell deletes the row");
    }

    /// Chat reaches a table while its player thinks (M6.4, R-6.4.4), so a chat
    /// post between two picker clicks — a table bump that changes no record —
    /// leaves the list, whoever posts (spec §2.4; battery 21's native catcher).
    #[test]
    fn a_chat_post_between_two_clicks_leaves_the_list() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        assert_eq!(lines(&at_place_a("chat")), ["1"]);
        for who in [BLUE, RED] {
            let r = call(who, "POST", "/api/games/chat/chat", Some(r#"{"text":"still thinking?"}"#));
            assert_eq!((r.status, tagged(&r).as_str()), (201, r#"[{"bump_table":"chat"}]"#), "premise: {who}'s post bumped the table");
            assert_eq!(r.picker, None, "{who}'s post leaves the row alone");
        }
        let r = input(RED, "chat", "A09", None);
        assert_eq!(lines(&r), ["1", "A09"], "the list the first click began, grown by the second");
    }

    /// What is NOT a picker line leaves the list, and a delete clears it (spec
    /// §2.4). A watcher UNLOCKS a locked table and READS the game — a 200 POST
    /// that is not `…/input`, then a 200 GET — and blue's line is refused,
    /// because it is red's turn: none of the three joins the list or clears
    /// it. The creator's delete ends the table, and its record goes with it.
    #[test]
    fn a_watcher_and_a_refused_line_leave_the_list_and_a_delete_clears_it() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let locked = CREATE.replacen('{', r#"{"password":"sesame","#, 1);
        assert_eq!(lines(&at_place_a_from("watched", &locked, r#"{"seat":1,"password":"sesame"}"#)), ["1"]);
        let r = call(WATCHER, "POST", "/api/games/watched/unlock", Some(r#"{"password":"sesame"}"#));
        assert_eq!((r.status, &r.picker), (200, &None), "{}", r.body);
        let r = call(WATCHER, "GET", "/api/games/watched", None);
        assert_eq!((r.status, &body(&r)["prompt"]["kind"]), (200, &json!("watching")), "premise: the watcher reads the game");
        assert_eq!(r.picker, None);
        let r = input(BLUE, "watched", "A09", None);
        assert_eq!((r.status, &r.picker), (403, &None), "{}", r.body);
        assert_eq!(lines(&input(RED, "watched", "A09", None)), ["1", "A09"], "red's list, with red's lines only");
        let r = call(RED, "DELETE", "/api/games/watched", None);
        assert_eq!((r.status, tagged(&r).as_str()), (204, r#"[{"end":"watched"}]"#), "premise: the creator's delete");
        assert_eq!(r.picker.as_deref(), Some("[]"), "the table is gone, and its list with it");
    }

    /// **The 65th line CLEARS the list** (spec §2.4), so nothing partial is ever
    /// re-fed. The picker itself stays open and playable in this instance, but
    /// nothing more is recorded until it closes: a list begun in the middle of a
    /// picker would re-feed its lines at the main prompt, where they mean
    /// something else.
    #[test]
    fn the_sixty_fifth_line_clears_the_list() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        assert_eq!(lines(&at_place_a("cap")), ["1"]);
        // Lines 2 to 64: `A09` 63 times, toggling Customs in and out.
        for n in 2..=64 {
            let r = input(RED, "cap", "A09", None);
            let selected = if n % 2 == 0 { json!([9]) } else { json!([]) };
            assert_eq!((r.status, &body(&r)["prompt"]["selected"]), (200, &selected), "line {n}");
            assert_eq!(lines(&r).len(), n, "line {n} joins the list");
        }
        let r = input(RED, "cap", "A09", None);
        assert_eq!((r.status, &body(&r)["prompt"]["kind"]), (200, &json!("place_a")), "the picker itself stays open");
        assert_eq!(r.picker.as_deref(), Some("[]"), "the 65th line clears the list");
        let r = input(RED, "cap", "A09", None);
        assert_eq!((r.status, &body(&r)["prompt"]["kind"]), (200, &json!("place_a")));
        assert_eq!(r.picker, None, "the 66th begins no list: it would re-feed without the lines before it");
        let r = input(RED, "cap", "cancel", None);
        assert_eq!((r.status, &body(&r)["prompt"]["kind"]), (200, &json!("options")), "premise: `cancel` closes the picker");
        assert_eq!(r.picker, None, "no row to delete: the 65th line already cleared it");
        assert_eq!(lines(&input(RED, "cap", "1", None)), ["1"], "the next picker's opening line begins a list of its own");
    }

    /// Umbrella §4's H-4 acceptance, natively: a PlaceA picker mid-selection
    /// survives a new instance, and the next line commits the move the player
    /// intended.
    #[test]
    fn restore_picker_rebuilds_a_place_a_picker_so_confirm_commits_the_intended_move() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        at_place_a("wake");
        let r = input(RED, "wake", "A09", Some("place_a"));
        assert_eq!(body(&r)["prompt"]["selected"], json!([9]), "premise: Customs is selected");
        let row = r.picker.expect("the picker's row");
        let (stored_table, stored) = (table("wake"), record("wake"));

        // THE CONTROL: a new instance WITHOUT `restore_picker` has no picker — the
        // restart `Input::expect`'s doc describes. Its `confirm` is a line at the
        // main prompt; the client's, which carries `expect`, is the 409.
        fresh("wake", &stored_table, &stored);
        let r = input(RED, "wake", "confirm", None);
        let s = body(&r);
        assert_eq!(
            (r.status, &s["events"], &s["messages"]),
            (200, &json!(1), &json!(["Unrecognized input — type 'help'."])),
            "{}",
            r.body,
        );
        assert_eq!(record("wake"), stored, "nothing applied");
        // A list left over from the last instance would be cleared by this line,
        // with "[]": `init` empties the pickers as it rebuilds the `Games`.
        assert_eq!(r.picker, None, "no list in a new instance");
        let r = input(RED, "wake", "confirm", Some("place_a"));
        assert_eq!(r.status, 409, "{}", r.body);
        let refusal = body(&r)["error"].as_str().unwrap().to_string();
        assert!(refusal.starts_with(r#"this game is asking "options", not "place_a""#), "{refusal}");

        // With it, the client's clicks go on: the restored list grows, still at
        // the record it was built at, and a SECOND hibernation of the same
        // picker re-feeds the grown list whole. Then `confirm` commits the
        // placement the player chose.
        fresh("wake", &stored_table, &stored);
        let r = restore_picker_inner("wake", &row);
        assert_eq!(
            (r.status, r.picker.as_deref(), tagged(&r).as_str()),
            (200, None, "[]"),
            "the list is kept — its row unchanged — and nothing moved",
        );
        let mut grown = String::new();
        for want in [vec!["1", "A09", "A09"], vec!["1", "A09", "A09", "A09"]] {
            let r = input(RED, "wake", "A09", Some("place_a"));
            assert_eq!(lines(&r), want, "the restored list grows");
            grown = r.picker.expect("the grown row");
            let built_at = serde_json::from_str::<Value>(&grown).unwrap()["record"].clone();
            assert_eq!(built_at, json!(digest_of(&stored)), "still the record the list was built at");
        }
        fresh("wake", &stored_table, &stored);
        let r = restore_picker_inner("wake", &grown);
        assert_eq!((r.status, r.picker.as_deref()), (200, None), "the grown list is kept");
        assert_eq!(body(&call(RED, "GET", "/api/games/wake", None))["prompt"]["selected"], json!([9]));
        let r = input(RED, "wake", "confirm", Some("place_a"));
        let s = body(&r);
        assert_eq!((r.status, &s["events"]), (200, &json!(2)), "{}", r.body);
        assert_eq!(s["messages"][0], json!("Red: 1 assistant placed in Customs (A09)"));
        assert_eq!(r.picker.as_deref(), Some("[]"), "the restored list is this instance's, and the move cleared it");
    }

    /// **A list is only ever re-fed at the exact record it was built at** (the
    /// plan's guard, spec §2.4). A row built before a move, restored after it, is
    /// cleared WITHOUT re-feeding a line — and its first line, `1`, APPLIES a
    /// move at the later record (the anti-vacuity half measures it), so a
    /// re-fed row would have moved the game.
    #[test]
    fn a_list_built_at_another_record_is_cleared_without_being_refed() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        at_place_a("stale");
        let row = input(RED, "stale", "A09", None).picker.expect("the picker's row");
        assert_eq!(input(RED, "stale", "confirm", None).status, 200);
        let (stored_table, moved) = (table("stale"), record("stale"));
        let built_at = serde_json::from_str::<Value>(&row).unwrap()["record"].clone();
        assert_ne!(built_at, json!(digest_of(&moved)), "premise: the row names ANOTHER record");

        fresh("stale", &stored_table, &moved);
        let r = restore_picker_inner("stale", &row);
        assert_eq!((r.status, r.picker.as_deref(), tagged(&r).as_str()), (200, Some("[]"), "[]"), "cleared, no bump");
        assert_eq!(record("stale"), moved, "the record untouched");

        // Anti-vacuity: at this record, the row's first line applies a move.
        fresh("stale", &stored_table, &moved);
        let r = input(RED, "stale", "1", None);
        assert_eq!((r.status, &body(&r)["events"]), (200, &json!(3)), "{}", r.body);
        assert_ne!(record("stale"), moved);
    }

    /// **`CODE` is the digest `build.rs` states**, recomputed HERE from the
    /// rule in this cell's own words, so the constant is never checked against
    /// the script that made it: every file under `crates/yokohama-core/src`,
    /// `crates/yokohama-cli/src` and `crates/yokohama-web/src` whose name does
    /// not begin with a dot, in the byte order of its path from the repository
    /// root, each as its path, a NUL, its length in decimal, a NUL and its
    /// bytes; SHA-256 in lowercase hex. It fails on a constant digested from
    /// other roots — the engine's left out, as it was before R1's R-1 — or by
    /// another rule, and on a stale one: a build script that did not run again
    /// after a source changed.
    #[test]
    fn the_code_is_the_digest_of_the_sources_that_give_a_line_its_meaning() {
        fn walk(repo: &std::path::Path, rel: &str, files: &mut Vec<String>) {
            for entry in std::fs::read_dir(repo.join(rel)).unwrap() {
                let entry = entry.unwrap();
                let name = entry.file_name().into_string().unwrap();
                let child = format!("{rel}/{name}");
                if name.starts_with('.') {
                    continue;
                }
                if entry.file_type().unwrap().is_dir() {
                    walk(repo, &child, files);
                } else {
                    files.push(child);
                }
            }
        }
        let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut files = Vec::new();
        for root in ["crates/yokohama-core/src", "crates/yokohama-cli/src", "crates/yokohama-web/src"] {
            walk(&repo, root, &mut files);
        }
        files.sort();
        for premise in
            ["crates/yokohama-core/src/game/legal_moves.rs", "crates/yokohama-cli/src/repl.rs", "crates/yokohama-web/src/api.rs"]
        {
            assert!(files.iter().any(|f| f == premise), "premise: {premise} was read — {} files", files.len());
        }
        let mut hasher = Sha256::new();
        for path in &files {
            let bytes = std::fs::read(repo.join(path)).unwrap();
            hasher.update(path.as_bytes());
            hasher.update([0]);
            hasher.update(bytes.len().to_string().as_bytes());
            hasher.update([0]);
            hasher.update(&bytes);
        }
        assert_eq!(CODE, format!("{:x}", hasher.finalize()));
    }

    /// **A row built under other code is cleared, and not one line of it is
    /// re-fed** (the orchestrator's ruling I-85; R1's probe P1). The row is
    /// P1's: at the record the finalizing keep left — no event yet — its one
    /// line is `1`, which here APPLIES the opening move, standing in for a line
    /// recorded under code where `1` opened a picker. Stamped with another
    /// code, or with none (a row an earlier build wrote, as every row the
    /// fourteenth to the eighteenth deployments did: the orchestrator's ruling
    /// I-89), it is cleared: `"[]"` for the shell to delete, nothing applied,
    /// nothing bumped, and `dropped` naming both codes — `stored: null` for
    /// the unstamped row — for the shell's `picker-dropped` line.
    ///
    /// The anti-vacuity half stamps the same row with THIS code: it is
    /// re-fed, and its line, which changes the record, has made the move — the
    /// layer persisted it and its bump is in this reply, for the shell to
    /// commit — and the list is cleared. That is what a restore did before the
    /// row carried a code, whatever code had recorded the line.
    #[test]
    fn a_row_built_under_other_code_is_cleared_and_not_a_line_of_it_is_refed() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let r = create_inner(Some(RED), CREATE.as_bytes(), "code", Some("application/json"));
        assert_eq!(r.status, 201, "{}", r.body);
        assert_eq!(call(BLUE, "POST", "/api/games/code/join", Some(r#"{"seat":1}"#)).status, 200);
        assert_eq!(call(RED, "POST", "/api/games/code/keep", Some(r#"{"seat":0,"card":1}"#)).status, 200);
        assert_eq!(call(BLUE, "POST", "/api/games/code/keep", Some(r#"{"seat":1,"card":1}"#)).status, 200);
        let (stored_table, stored) = (table("code"), record("code"));
        let at = digest_of(&stored);
        let row = |code: Option<&str>| {
            let mut row = json!({ "record": at, "lines": [{ "user": RED, "line": "1", "expect": null }] });
            if let Some(code) = code {
                row["code"] = json!(code);
            }
            row.to_string()
        };
        let other = "0".repeat(64);
        assert_ne!(other, CODE, "premise: another code");
        for (what, stamped, stored_code) in [("another code", Some(other.as_str()), json!(other)), ("no code", None, Value::Null)] {
            fresh("code", &stored_table, &stored);
            let r = restore_picker_inner("code", &row(stamped));
            assert_eq!(
                (r.status, r.picker.as_deref(), tagged(&r).as_str()),
                (200, Some("[]"), "[]"),
                "{what}: cleared, and nothing bumped"
            );
            let dropped: Value = serde_json::from_str(r.dropped.as_deref().expect("the reply names the dropped row")).unwrap();
            assert_eq!(dropped, json!({ "stored": stored_code, "running": CODE }), "{what}");
            assert_eq!(record("code"), stored, "{what}: nothing applied");
            let s = body(&call(RED, "GET", "/api/games/code", None));
            assert_eq!((&s["events"], &s["prompt"]["kind"]), (&json!(0), &json!("options")), "{what}: nothing re-fed");
        }

        // Anti-vacuity: stamped with THIS code, the same line is re-fed, and it
        // moves the game.
        fresh("code", &stored_table, &stored);
        let r = restore_picker_inner("code", &row(Some(CODE)));
        assert_eq!(
            (r.status, r.picker.as_deref(), tagged(&r).as_str(), r.dropped.as_deref()),
            (200, Some("[]"), r#"[{"bump":"code"}]"#, None),
            "re-fed: the line made its move, persisted and bumped, and the list is cleared",
        );
        assert_ne!(record("code"), stored, "the move stays applied");
        assert_eq!(body(&call(RED, "GET", "/api/games/code", None))["events"], json!(1));
    }

    /// At a table with no game yet — `handle()` answers `Err(Open)` or
    /// `Err(Dealing)` — `restore_picker` answers 200 and does nothing: no
    /// picker, no bumps, no archive, and no document written (plan review m13).
    /// The contrast is an id with no table at all, which is the layer's own
    /// refusal, in its error shape.
    #[test]
    fn restore_picker_at_a_table_with_no_session_answers_200_and_does_nothing() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let dealt = CREATE.replace(r#"{"open":true}"#, r#"{"user":"blue@t.test"}"#);
        assert_eq!(create_inner(Some(RED), CREATE.as_bytes(), "open-t", Some("application/json")).status, 201);
        assert_eq!(create_inner(Some(RED), dealt.as_bytes(), "dealing-t", Some("application/json")).status, 201);
        for id in ["open-t", "dealing-t"] {
            let err = GAMES.lock().unwrap().as_ref().unwrap().handle(id).err();
            match (id, &err) {
                ("open-t", Some(GameError::Open)) | ("dealing-t", Some(GameError::Dealing)) => {}
                _ => panic!("premise: {id} has no session — {err:?}"),
            }
            let before = table(id);
            let r = restore_picker_inner(id, "null");
            assert_eq!(
                (r.status, r.body.as_str(), tagged(&r).as_str(), &r.archive, &r.picker),
                (200, "", "[]", &None, &None),
                "{id}",
            );
            assert_eq!(table(id), before, "{id}: nothing written");
            assert!(read_inner("record", id).unwrap().is_none(), "{id}: and no game made");
        }
        let r = restore_picker_inner("nowhere", "null");
        assert_eq!(
            (r.status, r.body.as_str(), tagged(&r).as_str(), &r.picker),
            (404, r#"{"error":"no game named \"nowhere\""}"#, "[]", &None),
        );
    }

    /// **A row that is not a list is NO list** (the orchestrator's ruling I-21):
    /// answered 200 with `"[]"`, so the shell deletes it — never a panic, because
    /// losing a picker costs a 409 on a stale click and must never poison a
    /// table. Nothing is re-fed or bumped, and the record is untouched.
    ///
    /// The last five rows are at THIS record and under THIS code, each with
    /// one thing wrong: no lines at all; a key the list does not have; 65
    /// lines, more than a list ever holds; and the list, or its line, in
    /// serde's POSITIONAL array form, which a derived `Deserialize` reads and
    /// `Reply.picker` never writes. None of them is a row built under other
    /// code, so none is reported as `dropped`. The final checks are their
    /// anti-vacuity partners: the same line, in a row with nothing wrong, IS
    /// re-fed and kept, and so is the longest list `after_input` writes, 64
    /// lines.
    #[test]
    fn a_row_that_is_not_a_list_is_cleared_and_nothing_is_refed() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        at_place_a("junk");
        let (stored_table, stored) = (table("junk"), record("junk"));
        let at = digest_of(&stored);
        let one = json!({ "user": RED, "line": "1", "expect": null });
        let rows = [
            "[]".to_string(),
            "not json".to_string(),
            "42".to_string(),
            json!({ "record": at }).to_string(),
            json!({ "lines": [one] }).to_string(),
            json!({ "code": CODE, "record": at, "lines": [] }).to_string(),
            json!({ "code": CODE, "record": at, "lines": [one], "extra": 1 }).to_string(),
            json!({ "code": CODE, "record": at, "lines": vec![one.clone(); 65] }).to_string(),
            json!([CODE, at, [[RED, "1", null]]]).to_string(),
            json!({ "code": CODE, "record": at, "lines": [[RED, "1", null]] }).to_string(),
        ];
        for row in &rows {
            fresh("junk", &stored_table, &stored);
            let r = restore_picker_inner("junk", row);
            assert_eq!((r.status, r.picker.as_deref(), tagged(&r).as_str()), (200, Some("[]"), "[]"), "{row}");
            assert_eq!(r.dropped, None, "{row}: no list, so no code to drop it for");
            assert_eq!(record("junk"), stored, "{row}: the record untouched");
            let kind = body(&call(RED, "GET", "/api/games/junk", None))["prompt"]["kind"].clone();
            assert_eq!(kind, json!("options"), "{row}: nothing re-fed");
        }
        // Anti-vacuity: the same line, in a well-formed row at this record, IS re-fed and kept.
        fresh("junk", &stored_table, &stored);
        let r = restore_picker_inner("junk", &json!({ "code": CODE, "record": at, "lines": [one] }).to_string());
        assert_eq!((r.status, r.picker.as_deref()), (200, None));
        assert_eq!(body(&call(RED, "GET", "/api/games/junk", None))["prompt"]["kind"], json!("place_a"));
        // And the longest list `after_input` writes — `1`, then `A09` 63 times — is a list.
        let mut longest = vec![one.clone()];
        longest.extend(vec![json!({ "user": RED, "line": "A09", "expect": null }); 63]);
        fresh("junk", &stored_table, &stored);
        let r = restore_picker_inner("junk", &json!({ "code": CODE, "record": at, "lines": longest }).to_string());
        assert_eq!((r.status, r.picker.as_deref()), (200, None), "64 lines are kept");
        let prompt = body(&call(RED, "GET", "/api/games/junk", None))["prompt"].clone();
        assert_eq!((&prompt["kind"], &prompt["selected"]), (&json!("place_a"), &json!([9])), "all 64 re-fed");
    }

    /// A re-fed line that FAILS, or that leaves the picker CLOSED, clears the
    /// list (spec §2.4) — and the picker starts empty, as a restart leaves it,
    /// so the client's next click is `Input::expect`'s 409 rather than a move.
    /// This module never writes either row (it records only 200s that left the
    /// picker open); each is a list a re-feed diverged from. Without the close,
    /// the first row's re-feed would leave Customs selected for `confirm` to
    /// place. Without the per-line check, the `1` after the second row's
    /// `cancel` would reopen the picker with nothing selected, and `confirm`
    /// would spend the placement on "Place no assistants".
    #[test]
    fn a_refed_line_that_fails_or_closes_the_picker_clears_the_list() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        at_place_a("diverge");
        let row: Value = serde_json::from_str(&input(RED, "diverge", "A09", None).picker.unwrap()).unwrap();
        let (stored_table, stored) = (table("diverge"), record("diverge"));
        let with = |more: &[Value]| {
            let mut row = row.clone();
            row["lines"].as_array_mut().unwrap().extend_from_slice(more);
            row.to_string()
        };
        let rows = [
            ("a line that fails — it is red's turn, not blue's", with(&[json!({ "user": BLUE, "line": "A09", "expect": null })])),
            (
                "a line that closes the picker",
                with(&[json!({ "user": RED, "line": "cancel", "expect": null }), json!({ "user": RED, "line": "1", "expect": null })]),
            ),
        ];
        for (what, row) in rows {
            fresh("diverge", &stored_table, &stored);
            let r = restore_picker_inner("diverge", &row);
            assert_eq!((r.status, r.picker.as_deref()), (200, Some("[]")), "{what}");
            let r = input(RED, "diverge", "confirm", Some("place_a"));
            assert_eq!(r.status, 409, "{what}: the picker starts empty — {}", r.body);
            assert_eq!(record("diverge"), stored, "{what}: nothing applied");
        }
    }

    /// **`restore_picker` drains exactly as `dispatch` does** (the plan's shared
    /// names; plan review M4). It builds the table's handle FIRST, and a cold
    /// handle rebuilt with a bot to move pumps that move and persists it — so the
    /// pump's bump is in THIS reply, the one the wake commits, and none is left
    /// in the buffer for the next call's. With no row there is no picker to
    /// report.
    #[test]
    fn restore_picker_drains_a_cold_handles_owed_pump_into_its_own_reply() {
        let _s = serial();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
        let stored_table = std::fs::read_to_string(dir.join("owed-pump.table.json")).expect("the committed fixture");
        let owed = std::fs::read_to_string(dir.join("owed-pump.json")).expect("the committed fixture");
        fresh("owed", &stored_table, &owed);
        let r = restore_picker_inner("owed", "null");
        assert_eq!((r.status, tagged(&r).as_str(), &r.picker), (200, r#"[{"bump":"owed"}]"#, &None), "{}", r.body);
        assert_ne!(record("owed"), owed, "the pump was persisted");
        let next = call(RED, "GET", "/api/games/owed/table", None);
        assert_eq!((next.status, tagged(&next).as_str()), (200, "[]"), "nothing left behind for the next call");
    }

    /// The committed owed-pump pair (`the_owed_pump_fixture_owes_the_bot_a_move`
    /// in `lib.rs`): the stored table, and a record whose last events leave the
    /// bot to move.
    fn owed_pump() -> (String, String) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/test/fixtures");
        let read = |file: &str| std::fs::read_to_string(dir.join(file)).expect("the committed fixture");
        (read("owed-pump.table.json"), read("owed-pump.json"))
    }

    /// **The picker's rule on every call** (R1's cleanup debt 1; spec §2.7),
    /// the arrangement the three cells below share. Over a game in play, a
    /// call that is no dispatch changes the record by REBUILDING a cold
    /// handle, whose owed bot pump persists a move; a rebuilt `Repl` starts
    /// with its picker closed, so the entry goes whole, a `Dropped` one too. This host never reaches that
    /// state (an entry implies a resident handle), which is why it is made by
    /// hand: the owed-pump pair loaded into a NEW instance, so the handle is
    /// cold, and an entry put into `PICKERS`. A list's row is deleted
    /// (`"[]"`); a `Dropped` picker has no row, so none is deleted.
    fn a_rebuild_that_pumps_clears_the_entry(what: &str, stored_table: &str, status: u16, call: impl Fn() -> ReplyData) {
        let (_, owed) = owed_pump();
        let stale = || {
            super::Picker::Lines(super::PickerList {
                code: Some(CODE.to_string()),
                record: "0".repeat(64),
                lines: vec![super::PickerLine { user: RED.to_string(), line: "1".to_string(), expect: None }],
            })
        };
        for (kind, entry, want) in [("a list", stale(), Some("[]")), ("a dropped picker", super::Picker::Dropped, None)] {
            fresh("owed", stored_table, &owed);
            super::with_pickers(|pickers| pickers.insert("owed".to_string(), entry));
            let r = call();
            assert_eq!(
                (r.status, tagged(&r).as_str()),
                (status, r#"[{"bump":"owed"}]"#),
                "{what}, {kind}: premise — the rebuild pumped the owed move: {}",
                r.body,
            );
            assert_ne!(record("owed"), owed, "{what}, {kind}: premise — and the pump changed the record");
            assert_eq!(r.picker.as_deref(), want, "{what}, {kind}: a list's row is deleted, and no row that never was");
            assert!(super::with_pickers(|pickers| !pickers.contains_key("owed")), "{what}, {kind}: the entry is gone");
        }
    }

    /// `events` reaches `Games::handle` outside `around`, and a cold handle it
    /// builds pumps the owed move: the entry is cleared. Mutant:
    /// `Scope::Dispatch` passed by `events_inner`.
    #[test]
    fn an_events_read_whose_rebuild_pumps_clears_the_picker() {
        let _s = serial();
        let (stored_table, _) = owed_pump();
        a_rebuild_that_pumps_clears_the_entry("events", &stored_table, 200, || events_inner("owed", Some(RED), ""));
    }

    /// `stamp_archive` — the game-over retry, which builds the handle to read
    /// `over` — the same. Mutant: `Scope::Dispatch` passed by
    /// `stamp_archive_inner`.
    #[test]
    fn a_stamp_whose_rebuild_pumps_clears_the_picker() {
        let _s = serial();
        let (stored_table, _) = owed_pump();
        a_rebuild_that_pumps_clears_the_entry("stamp_archive", &stored_table, 200, || stamp_archive_inner("owed"));
    }

    /// `expire`'s arm with NO creator. It reaches a rebuild through one
    /// document alone: one the layer reads and this module's JSON read of the
    /// creator cannot — an unknown key holding a number out of `f64`'s range,
    /// which serde's struct parse skips unread and a `serde_json::Value`
    /// refuses. The arm then answers its host-written 500, over a game the
    /// rebuild has just moved. Every other no-creator table is gone or
    /// unreadable, and changes no record. Mutant: `Scope::Dispatch` passed by
    /// that arm.
    #[test]
    fn an_expire_with_no_creator_whose_rebuild_pumps_clears_the_picker() {
        let _s = serial();
        let (stored_table, _) = owed_pump();
        let unread = stored_table.replacen('{', r#"{"unread":1e400,"#, 1);
        assert!(serde_json::from_str::<Value>(&unread).is_err(), "premise: this module's read finds no creator");
        let doc = serde_json::from_str::<yokohama_web::table::TableDoc>(&unread);
        assert!(doc.is_ok_and(|doc| doc.created_by == RED), "premise: the layer reads the table, and its creator");
        a_rebuild_that_pumps_clears_the_entry("expire", &unread, 500, || expire_inner("owed"));
    }

    /// The partner: a call that changes no record leaves a live list as it was —
    /// an `events` read of a RESIDENT game with the PlaceA picker open, whose
    /// handle is not rebuilt.
    #[test]
    fn a_call_that_changes_no_record_leaves_the_list() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        at_place_a("kept");
        let before = record("kept");
        let e = events_inner("kept", Some(RED), "");
        assert_eq!((e.status, e.picker.as_deref()), (200, None), "{}", e.body);
        assert_eq!(record("kept"), before, "premise: the read changed no record");
        assert!(
            super::with_pickers(|pickers| matches!(pickers.get("kept"), Some(super::Picker::Lines(_)))),
            "the list stands",
        );
    }
}
