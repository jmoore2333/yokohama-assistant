//! **The house copy** (M6.5c, R-6.5c.8; spec §2.5): the table document a table
//! object hands the house object, and the house's side of it.
//!
//! The house object is a second layer instance whose store holds a SCRUBBED
//! COPY of every table's document. `GET /api/games?scope=…` on it is the
//! layer's own `Games::list` and `TableDoc::view` over those copies, the code
//! the NAS runs over the originals. [`house_copy_inner`] makes a copy of
//! exactly the documents that listing would read rather than skip, and a copy
//! lists as its original does, byte for byte.
//!
//! **The scrub** ([`house_copy_inner`]) replaces the document's three secrets
//! with placeholders that a listing reads exactly as it reads the originals:
//!
//! - the setup `seed` → 0. No row reads it;
//! - a `lock` → a `Lock` whose salt and hash are zeroed hex of the stored
//!   lengths. A row reads only that there is one (`locked`, `can_watch`), and
//!   `check_shape` accepts the placeholder as it accepts the lock;
//! - each `keeps[i]` → `Some(1)` if the seat kept, else `None`. A row reads
//!   only whether a seat has kept (`chosen`). `1` rather than `0`, because
//!   `check_shape` accepts a keep only as `None | Some(1 | 2)` and `Games::list`
//!   drops a document it refuses: a `Some(0)` scrub was measured taking a kept
//!   table off the lobby (spec §0.4).
//!
//! Everything else is as stored — `unlocked` included, which `can_watch`
//! reads.
//!
//! **`check_shape` runs twice, and either refusal carries its reason**, which
//! the shell logs (spec §2.8). The house's `list` would drop a document it
//! rejects, and would say so only through the layer's `eprintln!`, which is
//! mute on this host. So each defect is loud here, at the source, rather than
//! a row that silently leaves the lobby:
//!
//! - **on the stored ORIGINAL, first**: a defect in the table's own document.
//!   The NAS's listing drops such an original, and the scrub would repair two
//!   of its defects — a keep outside 1 and 2 becomes `1`, and a lock of the
//!   right lengths that is not hex becomes zeros. A check on the copy alone
//!   would accept both and hand the house a table the NAS drops, so the
//!   original is checked before the scrub (the orchestrator's ruling on
//!   Task 5's concern 2);
//! - **on the COPY, after the scrub**: a defect in the scrub itself. A correct
//!   scrub turns every document the first check accepts into a copy the second
//!   accepts, so this check fires only on a scrub defect, and no stored
//!   document reaches it through the real scrub. Its cell reaches it through
//!   the seam [`house_copy_with`], which takes the scrub as an argument, by
//!   handing it a defective one.
//!
//! The shell calls [`house_copy_inner`] on a TABLE object's instance, over the
//! table's own stored document, and [`put_copy_inner`], [`remove_copy_inner`]
//! and [`house_frame_inner`] on the HOUSE's.

use std::sync::Arc;

use serde::Serialize;
use yokohama_web::store::{Doc, MemStore, Store};
use yokohama_web::table::{Lock, TableDoc};
use yokohama_web::tables::check_shape;

use crate::STORE;

/// `id`'s house copy: its stored table document, scrubbed (the module doc) and
/// serialized — or `None` when no table document is stored under `id`. It is
/// `house_copy_with` with the real scrub, `scrub`.
///
/// Refused, with a sentence that carries the reason:
///
/// - a stored document that does not parse. Never `None`, which says that no
///   table is stored here, when one is (`Games::read_table`'s rule for the same
///   case);
/// - a stored document `check_shape` rejects, which the NAS's listing drops.
///   It is checked BEFORE the scrub, which could repair it (the module doc);
/// - a copy `check_shape` rejects: a scrub defect, which the house's listing
///   would drop.
///
/// Each of the last two carries `check_shape`'s own sentence.
pub fn house_copy_inner(id: &str) -> Result<Option<String>, String> {
    house_copy_with(id, scrub)
}

/// [`house_copy_inner`], with the scrub taken as an argument: the seam that
/// makes the check on the copy falsifiable (the orchestrator's ruling on the
/// follow-up's concern). No stored document reaches that check through the
/// real scrub, so its cell hands this function a defective one. Production
/// passes [`scrub`] and nothing else.
pub(crate) fn house_copy_with(id: &str, scrub: impl Fn(&mut TableDoc)) -> Result<Option<String>, String> {
    let Some((stored, _)) = store().read(Doc::Table, id)? else {
        return Ok(None);
    };
    let mut doc: TableDoc =
        serde_json::from_str(&stored).map_err(|e| format!("{id:?}'s table document does not parse: {e}"))?;
    check_shape(&doc)
        .map_err(|why| format!("{id:?}'s table document is malformed, so the house would not list it: {why}"))?;
    scrub(&mut doc);
    check_shape(&doc).map_err(|why| format!("{id:?}'s house copy is malformed, so the house would not list it: {why}"))?;
    Ok(Some(serde_json::to_string(&doc).expect("a table document serializes")))
}

/// The scrub, in place (the module doc). Every field is NAMED, with no `..`:
/// the three it replaces are bound, and every other is bound to `_` and kept
/// as stored. So a field added to `TableDoc` is a compile error here, and
/// whoever adds one decides whether the house may hold it.
fn scrub(doc: &mut TableDoc) {
    let TableDoc {
        schema: _,
        id: _,
        name: _,
        created_by: _,
        created_at: _,
        players: _,
        colors: _,
        start_seat: _,
        seats: _,
        keeps,
        seed,
        status: _,
        turn: _,
        lock,
        unlocked: _,
        archived_at: _,
    } = doc;
    *seed = 0;
    for keep in keeps.iter_mut().flatten() {
        *keep = 1;
    }
    if let Some(Lock { salt, hash }) = lock {
        *salt = "0".repeat(salt.len());
        *hash = "0".repeat(hash.len());
    }
}

/// REPLACE `id`'s copy in this instance's store, or create it: a `load` would
/// refuse a present one. The version is read first, and the write is the
/// store's own compare-and-swap against it.
///
/// The copy is stored as given, unchecked: the check is
/// [`house_copy_inner`]'s, at the source.
pub fn put_copy_inner(id: &str, json: &str) -> Result<(), String> {
    let store = store();
    let current = store.read(Doc::Table, id)?.map(|(_, version)| version);
    store.write_if_version(Doc::Table, id, json, current).map(|_| ()).map_err(|e| e.to_string())
}

/// Remove `id`'s copy from this instance's store, through `Store::delete`.
///
/// **Absent is not an error.** A table's slot holds only its latest change
/// (spec §2.5), so a table deleted before its first copy reached the house
/// sends a removal for a copy the house never held.
pub fn remove_copy_inner(id: &str) -> Result<(), String> {
    let store = store();
    if store.read(Doc::Table, id)?.is_none() {
        return Ok(());
    }
    store.delete(id)
}

/// The lobby socket's frame data (spec §2.5): `{"seq":…,"chat":…}`, the text the
/// NAS's `/api/stream` writes as each `house` event's data, with the layer's
/// own `u64` counters (`signal::House`, which is behind the layer's `server`
/// feature and not built here).
pub fn house_frame_inner(seq: u64, chat: u64) -> String {
    serde_json::to_string(&HouseFrame { seq, chat }).expect("two numbers serialize")
}

/// The house frame's two counters, in the order the NAS writes them.
#[derive(Serialize)]
struct HouseFrame {
    seq: u64,
    chat: u64,
}

/// This instance's store: `crate::STORE`, the same `Arc` its `Games` reads, so
/// what [`put_copy_inner`] writes is what `GET /api/games` lists.
fn store() -> Arc<MemStore> {
    let guard = STORE.lock().expect("the host's store");
    Arc::clone(guard.as_ref().expect("init() before any call into the store"))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::{json, Value};
    use yokohama_web::table::{Lock, TableDoc};
    use yokohama_web::tables::check_shape;

    use super::{house_copy_inner, house_copy_with, house_frame_inner, put_copy_inner, remove_copy_inner};
    use crate::tests::{call, config, serial};
    use crate::{create_inner, dispatch_inner, init_inner, load_inner, read_inner};

    const RED: &str = "red@t.test";
    const BLUE: &str = "blue@t.test";
    const STRANGER: &str = "stranger@t.test";
    const WATCHER: &str = "watcher@t.test";
    /// The committed finished pair's creator, and its one human seat.
    const LOCAL: &str = "local@browser";
    const VIEWERS: [&str; 5] = [RED, BLUE, STRANGER, WATCHER, LOCAL];
    const SCOPES: [&str; 3] = ["mine", "open", "playing"];

    /// The committed finished pair's id: its table document's own `id`, the key
    /// `write_table` writes it under, so it is loaded under no other (plan
    /// review m12).
    const OVER: &str = "local-242a5429453c593c";
    const PASSWORD: &str = "sesame";
    /// Review Focus 4: a table name and a display name with an accent, an emoji
    /// and CJK text.
    const CAFE: &str = "Café ☕ 東京";
    const ZOE: &str = "Zoë";
    /// The created tables are dealt from `SEED`, `SEED + 1`, …: large, and one
    /// per table, so each table's needles are its own.
    const SEED: u64 = 0x5eed_c0de_0000_0001;

    /// One table of [`arrange`]: its id, the seed it was dealt from, whether it
    /// is locked, and the seat that kept card 2, if one did.
    struct Arranged {
        id: &'static str,
        seed: u64,
        locked: bool,
        kept_two: Option<usize>,
    }

    /// Every captured page: `(viewer, scope, body)`, in [`listings`]' order.
    type Pages = Vec<(&'static str, &'static str, String)>;

    /// A two-seat create body: red at seat 0 and `second` at seat 1, dealt from
    /// `seed`, which travels as a STRING, as a browser sends it (`WireSeed`).
    fn two_seats(seed: u64, second: Value) -> Value {
        json!({
            "players": 2,
            "colors": ["Red", "Blue"],
            "start_seat": 0,
            "seed": seed.to_string(),
            "seats": [{ "user": RED }, second],
        })
    }

    /// RED's create of `id`, through `create` as the Worker sends it.
    fn create(id: &str, body: Value) {
        let r = create_inner(Some(RED), body.to_string().as_bytes(), id, Some("application/json"));
        assert_eq!(r.status, 201, "{id}: {}", r.body);
    }

    /// `POST /api/games/{id}/{action}` as `user`, answered 200.
    fn post(user: &str, id: &str, action: &str, body: Value) {
        let r = call(user, "POST", &format!("/api/games/{id}/{action}"), Some(&body.to_string()));
        assert_eq!(r.status, 200, "{user}'s {action} at {id}: {}", r.body);
    }

    /// Every status at both lock states, built through `create` and `dispatch`
    /// as the Worker builds them, and a finished game, unlocked: the COMMITTED
    /// pair `browser-played-2p` (595 events against the bot), loaded as it is.
    /// A game played to its end here would need a policy that ends one: the
    /// layer's own whole-game gate livelocks on some seeds for want of one.
    ///
    /// - `open-u`, `open-l`: red at seat 0, seat 1 open. `open-u` is named
    ///   [`CAFE`], and red's display name is [`ZOE`] (Review Focus 4).
    /// - `dealing-u`, `dealing-l`: red and blue seated by the create, and ONE
    ///   keep in, of card 2.
    /// - `playing-u`, `playing-l`: blue takes the open seat, both keep — one of
    ///   them card 2 — and the last keep deals the game. Then WATCHER gives
    ///   `playing-l`'s password, which puts them on its `unlocked` roster; the
    ///   layer refuses an unlock before `playing`.
    /// - [`OVER`]: over, with the bot the winner.
    fn arrange() -> Vec<Arranged> {
        let open = json!({ "open": true });
        let blue = json!({ "user": BLUE });
        let locked = |mut body: Value| {
            body["password"] = json!(PASSWORD);
            body
        };

        let mut cafe = two_seats(SEED, open.clone());
        cafe["name"] = json!(CAFE);
        cafe["display_name"] = json!(ZOE);
        create("open-u", cafe);
        create("open-l", locked(two_seats(SEED + 1, open.clone())));

        create("dealing-u", two_seats(SEED + 2, blue.clone()));
        post(RED, "dealing-u", "keep", json!({ "seat": 0, "card": 2 }));
        create("dealing-l", locked(two_seats(SEED + 3, blue)));
        post(BLUE, "dealing-l", "keep", json!({ "seat": 1, "card": 2 }));

        create("playing-u", two_seats(SEED + 4, open.clone()));
        post(BLUE, "playing-u", "join", json!({ "seat": 1 }));
        post(RED, "playing-u", "keep", json!({ "seat": 0, "card": 2 }));
        post(BLUE, "playing-u", "keep", json!({ "seat": 1, "card": 1 }));
        create("playing-l", locked(two_seats(SEED + 5, open)));
        post(BLUE, "playing-l", "join", json!({ "seat": 1, "password": PASSWORD }));
        post(RED, "playing-l", "keep", json!({ "seat": 0, "card": 1 }));
        post(BLUE, "playing-l", "keep", json!({ "seat": 1, "card": 2 }));
        post(WATCHER, "playing-l", "unlock", json!({ "password": PASSWORD }));

        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../yokohama-web/tests/fixtures");
        let committed = |name: &str| std::fs::read_to_string(fixtures.join(name)).expect("the committed pair");
        let record = committed("browser-played-2p.json");
        load_inner("table", OVER, &committed("browser-played-2p.table.json")).unwrap();
        load_inner("record", OVER, &record).unwrap();
        // The over table's seed, read out of its RECORD — the other document —
        // so a cell that finds it in the table's text has found something.
        let over_seed = serde_json::from_str::<Value>(&record).unwrap()["seed"].as_u64().expect("a u64 seed");

        let stored = |id: &str| serde_json::from_str::<Value>(&read_inner("table", id).unwrap().unwrap()).unwrap();
        for (id, status) in [
            ("open-u", "open"),
            ("open-l", "open"),
            ("dealing-u", "dealing"),
            ("dealing-l", "dealing"),
            ("playing-u", "playing"),
            ("playing-l", "playing"),
            (OVER, "playing"),
        ] {
            assert_eq!(stored(id)["status"], json!(status), "premise: {id}'s status");
        }
        assert_eq!(stored(OVER)["turn"]["over"], json!(true), "premise: the committed pair is over");
        assert_eq!(stored("playing-l")["unlocked"], json!([WATCHER]), "premise: the watcher is on the roster");

        vec![
            Arranged { id: "open-u", seed: SEED, locked: false, kept_two: None },
            Arranged { id: "open-l", seed: SEED + 1, locked: true, kept_two: None },
            Arranged { id: "dealing-u", seed: SEED + 2, locked: false, kept_two: Some(0) },
            Arranged { id: "dealing-l", seed: SEED + 3, locked: true, kept_two: Some(1) },
            Arranged { id: "playing-u", seed: SEED + 4, locked: false, kept_two: Some(0) },
            Arranged { id: "playing-l", seed: SEED + 5, locked: true, kept_two: Some(1) },
            Arranged { id: OVER, seed: over_seed, locked: false, kept_two: None },
        ]
    }

    /// `GET /api/games?scope=<scope>` as `viewer`, answered 200: the page's body.
    fn listing(viewer: &str, scope: &str) -> String {
        let r = dispatch_inner(Some(viewer), "GET", "/api/games", &format!("scope={scope}"), None, b"");
        assert_eq!(r.status, 200, "{viewer} at scope={scope}: {}", r.body);
        r.body
    }

    /// [`listing`] for every viewer at every scope.
    fn listings() -> Pages {
        VIEWERS.iter().flat_map(|v| SCOPES.iter().map(move |s| (*v, *s, listing(v, s)))).collect()
    }

    /// `viewer`'s page at `scope` among `pages`.
    fn page<'a>(pages: &'a Pages, viewer: &str, scope: &str) -> &'a str {
        let (_, _, body) = pages.iter().find(|(v, s, _)| *v == viewer && *s == scope).expect("a captured page");
        body
    }

    /// `id`'s row on a page of `GET /api/games`, if the page has one.
    fn row_in(page: &str, id: &str) -> Option<Value> {
        let page: Value = serde_json::from_str(page).unwrap();
        page["games"].as_array().expect("a page of rows").iter().find(|row| row["id"] == json!(id)).cloned()
    }

    /// `id`'s stored table document and its house copy, both as text.
    fn original_and_copy(id: &str) -> (String, String) {
        let original = read_inner("table", id).unwrap().expect("a stored table");
        let copy = house_copy_inner(id).unwrap().expect("a copy of a stored table");
        (original, copy)
    }

    /// **The listing is the NAS's** (spec §2.5, §5.1): through `GET /api/games`
    /// — the layer's own `Games::list` and `TableDoc::view` — the house copies
    /// list exactly the rows their originals list, BYTE for byte, for every
    /// viewer at every scope, over tables at every status at both lock states,
    /// kept human seats among them.
    ///
    /// The viewers: RED, the creator of every created table; BLUE, a seat holder
    /// who created none; STRANGER, seated nowhere; WATCHER, a stranger on
    /// `playing-l`'s `unlocked` roster after a correct unlock; and LOCAL, the
    /// over table's creator and seat holder, so its row is under `mine` too.
    #[test]
    fn the_scrubbed_copies_list_exactly_what_the_originals_list_for_every_viewer() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let tables = arrange();
        let originals = listings();
        let copies: Vec<(&str, String)> = tables.iter().map(|t| (t.id, original_and_copy(t.id).1)).collect();

        // A fresh instance holding nothing but the copies: the house.
        init_inner(&config("off")).unwrap();
        for (id, copy) in &copies {
            put_copy_inner(id, copy).unwrap();
        }
        let house = listings();
        assert_eq!(house.len(), VIEWERS.len() * SCOPES.len());
        for ((viewer, scope, original), (_, _, listed)) in originals.iter().zip(&house) {
            assert_eq!(listed, original, "{viewer} at scope={scope}");
        }

        // Anti-vacuity: every table is a row on some page, so no copy is
        // compared only by its absence …
        for t in &tables {
            let listed = originals.iter().any(|(_, _, body)| row_in(body, t.id).is_some());
            assert!(listed, "premise: {} is listed somewhere", t.id);
        }
        // … a kept human seat is in the comparison, as `chosen` …
        let dealing = row_in(page(&originals, RED, "mine"), "dealing-u").unwrap();
        assert_eq!(dealing["chosen"], json!([true, false]), "premise: red's keep, and blue's none");
        // … the roster is read: WATCHER may watch `playing-l`, and STRANGER may not …
        let watched = |viewer| row_in(page(&originals, viewer, "playing"), "playing-l").unwrap()["can_watch"].clone();
        assert_eq!((watched(WATCHER), watched(STRANGER)), (json!(true), json!(false)), "premise: the roster");
        // … and the finished game's row reads over, the bot its winner.
        let over = row_in(page(&originals, STRANGER, "playing"), OVER).unwrap();
        assert_eq!((&over["over"], &over["winner"]), (&json!(true), &json!(1)), "premise: over, the bot the winner");

        // Review Focus 4: the characters themselves in the copy — not escapes of
        // them — and verbatim on every page that lists the table, in both
        // instances.
        let (_, cafe) = copies.iter().find(|(id, _)| *id == "open-u").unwrap();
        assert!(cafe.contains(CAFE) && cafe.contains(ZOE), "{cafe}");
        for (viewer, scope, body) in originals.iter().chain(&house) {
            if *scope == "open" || (*viewer == RED && *scope == "mine") {
                assert!(body.contains(CAFE) && body.contains(ZOE), "{viewer} at scope={scope}: {body}");
            }
        }
    }

    /// The value pin (battery 3) and its anti-vacuity partner (plan review M6):
    /// the ORIGINAL stored document carries the decimal seed, the lock's salt and
    /// hash, and a keep of 2 — and no copy carries any of them. The PASSWORD and the
    /// seed in HEX are absence-only needles: the stored document never holds either
    /// (the lock is `Lock { salt, hash }`, and the seed is a decimal JSON number), so
    /// they are searched for in the copy and never claimed present in the original.
    #[test]
    fn no_house_copy_carries_a_secret_that_its_original_does() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        for t in arrange() {
            let (original, copy) = original_and_copy(t.id);
            let stored: Value = serde_json::from_str(&original).unwrap();
            let id = t.id;

            // The seed, read out of the original: the one this table was dealt
            // from, in the decimal the document writes it in.
            let seed = stored["seed"].as_u64().expect("a decimal seed");
            assert_eq!(seed, t.seed, "premise: {id}'s stored seed is the one it was dealt from");
            let mut needles = vec![seed.to_string()];
            // The lock's salt and hash, read out of the original: the real lock,
            // which the password opens.
            if t.locked {
                let lock: Lock = serde_json::from_value(stored["lock"].clone()).expect("a stored lock");
                assert!(lock.matches(PASSWORD), "premise: {id}'s stored salt and hash are its lock");
                needles.extend([lock.salt, lock.hash]);
            }
            for needle in &needles {
                assert!(original.contains(needle.as_str()), "premise: {id}'s original carries {needle}");
                assert!(!copy.contains(needle.as_str()), "{id}'s copy carries {needle}");
            }
            for needle in [PASSWORD.to_string(), format!("{seed:x}"), format!("{seed:X}")] {
                assert!(!copy.contains(needle.as_str()), "{id}'s copy carries {needle}");
            }

            // The keeps: the arranged keep of 2 is in the original, and every
            // keep in the copy is 1 or null.
            if let Some(seat) = t.kept_two {
                assert_eq!(stored["keeps"][seat], json!(2), "premise: {id}'s original holds a keep of 2");
            }
            let copied: Value = serde_json::from_str(&copy).unwrap();
            for keep in copied["keeps"].as_array().expect("the copy's keeps") {
                assert!(*keep == json!(1) || keep.is_null(), "{id}'s copy keeps {keep}");
            }
        }
    }

    /// A scrub defect is loud at the source (R-6.5c.8): a copy `check_shape`
    /// rejects is refused with its reason, never listed.
    ///
    /// The document is a table the layer wrote, with `"schema": 99` its ONE
    /// change. The scrub keeps `schema`, so the copy would carry the 99 too,
    /// but the check on the stored original refuses it first, with
    /// `check_shape`'s own sentence. (No stored document reaches the check on
    /// the copy through the real scrub; its cell hands `house_copy_with` a
    /// defective one.) The same document at the schema the layer wrote has a
    /// copy.
    #[test]
    fn house_copy_refuses_a_copy_that_check_shape_rejects() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        create("future", two_seats(SEED, json!({ "open": true })));
        assert!(house_copy_inner("future").unwrap().is_some(), "premise: the document as written has a copy");
        let stored = read_inner("table", "future").unwrap().unwrap();
        let future = stored.replacen("\"schema\": 3,", "\"schema\": 99,", 1);
        assert_ne!(future, stored, "premise: the layer wrote schema 3, and it is now 99");

        init_inner(&config("off")).unwrap();
        load_inner("table", "future", &future).unwrap();
        let err = house_copy_inner("future").unwrap_err();
        assert!(err.contains("schema"), "{err}");
        let why = check_shape(&serde_json::from_str::<TableDoc>(&future).unwrap()).unwrap_err();
        assert!(err.contains(&why), "`check_shape`'s own reason, verbatim — {why:?} in {err:?}");
    }

    /// **A stored document `check_shape` rejects is refused, even where its
    /// copy would pass** (the orchestrator's ruling on Task 5's concern 2). The
    /// scrub REPLACES the keeps and the lock, so it repairs a keep outside 1
    /// and 2, and a lock of the right lengths that is not hex. A check on the
    /// copy alone would hand the house a table that the NAS's listing drops.
    ///
    /// Each document is a locked table the layer wrote, with red's keep of
    /// card 2 in, stored under its own id with ONE field the scrub replaces
    /// made malformed, and written back as the layer writes a table
    /// (`to_string_pretty`). Two premises hold both halves of the case for
    /// each: `check_shape` rejects the document, and accepts the copy the scrub
    /// makes of it. The refusal is the check on the ORIGINAL, in its own words,
    /// carrying `check_shape`'s sentence. And the layer's own listing on this
    /// instance, the NAS's code, drops every one of them.
    #[test]
    fn house_copy_refuses_a_stored_document_check_shape_rejects_even_where_its_copy_would_pass() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let mut body = two_seats(SEED, json!({ "user": BLUE }));
        body["password"] = json!(PASSWORD);
        create("kept", body);
        post(RED, "kept", "keep", json!({ "seat": 0, "card": 2 }));
        assert!(house_copy_inner("kept").unwrap().is_some(), "premise: the document as written has a copy");
        let written: TableDoc = serde_json::from_str(&read_inner("table", "kept").unwrap().unwrap()).unwrap();
        assert!(written.keeps == [Some(2), None] && written.lock.is_some(), "premise: a keep of 2, and a lock");

        /// Make one field of a document malformed, in place.
        type Malform = fn(&mut TableDoc);
        let malformed: [(&str, Malform); 4] = [
            ("a keep of 0", |doc| doc.keeps[0] = Some(0)),
            ("a keep of 3", |doc| doc.keeps[0] = Some(3)),
            ("a salt that is not hex", |doc| {
                let lock = doc.lock.as_mut().unwrap();
                lock.salt = "g".repeat(lock.salt.len());
            }),
            ("a hash that is not hex", |doc| {
                let lock = doc.lock.as_mut().unwrap();
                lock.hash = "g".repeat(lock.hash.len());
            }),
        ];
        for (n, (what, malform)) in malformed.into_iter().enumerate() {
            let id = format!("kept-{n}");
            let mut doc = written.clone();
            doc.id = id.clone();
            malform(&mut doc);
            let why = check_shape(&doc).expect_err(&format!("premise: `check_shape` rejects {what}"));
            let mut copy = doc.clone();
            super::scrub(&mut copy);
            assert!(check_shape(&copy).is_ok(), "premise: with {what}, the copy alone passes");
            load_inner("table", &id, &serde_json::to_string_pretty(&doc).unwrap()).unwrap();
            let refusal = format!("{id:?}'s table document is malformed, so the house would not list it: {why}");
            assert_eq!(house_copy_inner(&id), Err(refusal), "{what}");
        }
        let mine = listing(RED, "mine");
        assert!(row_in(&mine, "kept").is_some(), "premise: the listing lists the table as the layer wrote it");
        for n in 0..4 {
            assert!(row_in(&mine, &format!("kept-{n}")).is_none(), "premise: the listing drops kept-{n}");
        }
    }

    /// **The check on the copy is falsifiable** (the orchestrator's ruling on
    /// the follow-up's concern). No stored document reaches it through the real
    /// scrub, so this cell hands `house_copy_with` a DEFECTIVE one: the real
    /// scrub, but writing a kept seat's keep as `Some(0)` where it writes
    /// `Some(1)`. That is the defect the spec's review measured dropping a kept
    /// table off the lobby (spec §0.4). Over a valid stored document it is
    /// REFUSED, in the copy's words, carrying `check_shape`'s sentence for the
    /// copy it made.
    ///
    /// The anti-vacuity partner: the same document passes the check on the
    /// original, and under the real scrub it has a copy — the one
    /// `house_copy_inner` answers. So the refusal is the copy check's, and the
    /// defect is its cause.
    #[test]
    fn a_defective_scrub_is_refused_by_the_check_on_the_copy() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        create("defect", two_seats(SEED, json!({ "user": BLUE })));
        post(RED, "defect", "keep", json!({ "seat": 0, "card": 2 }));
        let stored: TableDoc = serde_json::from_str(&read_inner("table", "defect").unwrap().unwrap()).unwrap();
        assert!(check_shape(&stored).is_ok() && stored.keeps[0] == Some(2), "premise: a valid document, a keep in");
        let real = house_copy_with("defect", super::scrub);
        assert!(matches!(&real, Ok(Some(_))), "premise: the real scrub makes a copy — {real:?}");
        assert_eq!(house_copy_inner("defect"), real, "premise: `house_copy_inner` is the seam with the real scrub");

        let defective = |doc: &mut TableDoc| {
            super::scrub(doc);
            for keep in doc.keeps.iter_mut().flatten() {
                *keep = 0;
            }
        };
        let mut made = stored.clone();
        defective(&mut made);
        let why = check_shape(&made).expect_err("premise: `check_shape` rejects the defective copy");
        let refusal = format!("\"defect\"'s house copy is malformed, so the house would not list it: {why}");
        assert_eq!(house_copy_with("defect", defective), Err(refusal));
    }

    /// **`house_frame` is the NAS's lobby frame, byte for byte** (spec §2.5): the
    /// `data` of the `house` event `house_stream` writes on `/api/stream`. That
    /// `format!` is behind the layer's `server` feature, which this crate does
    /// not build, so it is RESTATED here (the orchestrator's ruling R-02) — and
    /// the restatement is pinned to the layer's source text, so a change to the
    /// frame there reds this cell rather than reaching two clients as two
    /// shapes. The export takes `u32` (the orchestrator's ruling I-22) and widens
    /// it without loss.
    #[test]
    fn house_frame_is_the_sse_frames_data_byte_for_byte() {
        assert_eq!(house_frame_inner(3, 7), r#"{"seq":3,"chat":7}"#);
        // `house_stream`'s `format!`, restated from `crates/yokohama-web/src/lib.rs:720`.
        let sse = |seq: u64, chat: u64| format!("{{\"seq\":{},\"chat\":{}}}", seq, chat);
        for (seq, chat) in [(0, 0), (3, 7), (7, 3), (u64::from(u32::MAX) + 1, 1), (u64::MAX, u64::MAX)] {
            assert_eq!(house_frame_inner(seq, chat), sse(seq, chat), "({seq}, {chat})");
        }
        let layer = Path::new(env!("CARGO_MANIFEST_DIR")).join("../yokohama-web/src/lib.rs");
        let layer = std::fs::read_to_string(layer).expect("the layer's source");
        let line = r#".data(format!("{{\"seq\":{},\"chat\":{}}}", h.seq, h.chat)),"#;
        assert_eq!(layer.matches(line).count(), 1, "the layer writes the frame restated above: {line}");
        assert_eq!(crate::house_frame(3, 7), house_frame_inner(3, 7));
        assert_eq!(crate::house_frame(u32::MAX, u32::MAX), sse(u64::from(u32::MAX), u64::from(u32::MAX)));
    }

    /// `put_copy` REPLACES a copy the house already holds — where `load`, a
    /// create, refuses a present document — and `remove_copy` removes one, with
    /// absent not an error: a table deleted before its first copy reached the
    /// house sends it a removal for a copy it never held (spec §2.5; the slot
    /// holds only the latest change). The witness is the layer's own listing on
    /// the house instance, which is also what proves `STORE` is the store that
    /// listing reads.
    #[test]
    fn put_copy_replaces_a_copy_and_remove_copy_removes_one_absent_or_not() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        create("moved", two_seats(SEED, json!({ "open": true })));
        let open = house_copy_inner("moved").unwrap().unwrap();
        post(BLUE, "moved", "join", json!({ "seat": 1 }));
        let dealing = house_copy_inner("moved").unwrap().unwrap();
        assert_ne!(open, dealing, "premise: the join changed the copy");
        let (open_page, blue_page) = (listing(STRANGER, "open"), listing(BLUE, "mine"));
        assert!(row_in(&open_page, "moved").is_none() && row_in(&blue_page, "moved").is_some(), "premise: dealt");

        init_inner(&config("off")).unwrap();
        put_copy_inner("moved", &open).unwrap();
        assert!(row_in(&listing(STRANGER, "open"), "moved").is_some(), "the first copy creates the row");
        assert!(load_inner("table", "moved", &dealing).is_err(), "premise: `load` refuses a present document");
        put_copy_inner("moved", &dealing).unwrap();
        assert_eq!(read_inner("table", "moved").unwrap(), Some(dealing.clone()), "replaced, byte for byte");
        assert_eq!(
            (listing(STRANGER, "open"), listing(BLUE, "mine")),
            (open_page, blue_page),
            "the house lists the table as the table's own instance does",
        );

        remove_copy_inner("moved").unwrap();
        assert_eq!(read_inner("table", "moved").unwrap(), None);
        assert!(row_in(&listing(BLUE, "mine"), "moved").is_none(), "and off the listing");
        // Absent, twice over: removed already, and never held.
        remove_copy_inner("moved").unwrap();
        remove_copy_inner("never-held").unwrap();
    }

    /// The scrub's rule for `id`'s stored document, field by field: seed 0; a
    /// lock's salt and hash zeroed at their stored lengths, and no lock where
    /// there was none; a keep of 1 where a seat kept, and null where it did not;
    /// and every other field the original's, value for value.
    fn assert_scrubbed(id: &str) {
        let (original, copy) = original_and_copy(id);
        let (mut original, mut copy): (Value, Value) =
            (serde_json::from_str(&original).unwrap(), serde_json::from_str(&copy).unwrap());
        assert_eq!(copy["seed"], json!(0), "{id}'s seed");
        if original["lock"].is_null() {
            assert!(copy["lock"].is_null(), "{id}: no lock, and no placeholder");
        } else {
            for part in ["salt", "hash"] {
                let len = original["lock"][part].as_str().expect("a stored lock").len();
                assert_eq!(copy["lock"][part], json!("0".repeat(len)), "{id}'s {part}");
            }
        }
        let kept: Vec<Value> = original["keeps"]
            .as_array()
            .expect("the original's keeps")
            .iter()
            .map(|keep| if keep.is_null() { Value::Null } else { json!(1) })
            .collect();
        assert_eq!(copy["keeps"], json!(kept), "{id}'s keeps");
        for field in ["seed", "lock", "keeps"] {
            original.as_object_mut().unwrap().remove(field);
            copy.as_object_mut().unwrap().remove(field);
        }
        assert_eq!(copy, original, "{id}: every other field as stored");
    }

    /// **Everything else as stored** (spec §2.5). The listing cell cannot see a
    /// field no row reads: `schema`, or `archived_at`, whose stamp changes the
    /// copy with no visible change to a row. This build archives nothing, so no
    /// table the layer builds here is stamped, and the finished pair is stamped
    /// by hand for the second half.
    #[test]
    fn the_scrub_replaces_the_seed_the_lock_and_the_keeps_and_nothing_else() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        let tables = arrange();
        assert!(tables.iter().any(|t| t.locked) && tables.iter().any(|t| !t.locked), "premise: both lock states");
        for t in &tables {
            assert_scrubbed(t.id);
        }

        let mut stamped: Value = serde_json::from_str(&read_inner("table", OVER).unwrap().unwrap()).unwrap();
        stamped["archived_at"] = json!(1_789_956_600u64);
        init_inner(&config("off")).unwrap();
        load_inner("table", OVER, &serde_json::to_string_pretty(&stamped).unwrap()).unwrap();
        assert_scrubbed(OVER);
        let copy: Value = serde_json::from_str(&house_copy_inner(OVER).unwrap().unwrap()).unwrap();
        assert_eq!(copy["archived_at"], json!(1_789_956_600u64), "premise: the stamp is in the comparison");
    }

    /// `None` answers "no table document is stored here", and ONLY that. A
    /// stored document that does not parse is refused instead: it is sitting
    /// right there (`Games::read_table`'s rule for the same case).
    #[test]
    fn house_copy_is_none_only_where_no_table_is_stored() {
        let _s = serial();
        init_inner(&config("off")).unwrap();
        assert_eq!(house_copy_inner("nowhere"), Ok(None));
        for (id, text) in [("garbled", "not json"), ("hollow", "{}")] {
            load_inner("table", id, text).unwrap();
            let err = house_copy_inner(id).unwrap_err();
            assert!(err.contains("does not parse"), "{id}: {err}");
        }
    }
}
