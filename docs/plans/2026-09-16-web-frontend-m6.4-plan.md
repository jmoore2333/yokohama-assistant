# Web frontend — M6.4 "chat": implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Two players talk during a game and a third reads it as a spectator, and the same people can talk in the lobby — per-table messages in their own store document, delivered as a count on the signal the browser already holds, drawn in one folded panel at the foot of the side column.

**Architecture:** Server-first, then the client. A third document kind (`Doc::Chat`, `<id>.chat.json`) and one house file (`_house.chat.json`) hold messages whose author is a tagged object (`seat` | `watcher` | `bot`, the last produced by nothing). `Games::chat_append` is the one writer: the text rule, the table's own lock, a compare-and-swap, a trim to the configured keep, then `bump_table` AFTER the write. `Events` gains a fifth field `chat` (an in-process count seeded from the document), the house frame becomes `{seq, chat}`, and four routes (`GET`/`POST …/{id}/chat`, `GET`/`POST /api/chat`) admit exactly as the reads do, with a watcher's post as R-3.6's one named exception. Three launch variables (the lobby switch and the two retention caps) reach the client on `/api/health`. The client is one `Chat` component with four hosts, the game screen's stream held at every status behind a posting guard, and a "Your name" field at the door and the lobby.

**Tech Stack:** axum 0.8, serde (an internally tagged enum, the `SeatDoc` precedent), `tokio::sync::watch` (the house channel's value type changes), React 19 + TypeScript, vitest + jsdom over the committed wire corpus, Playwright against the release binary (three contexts; the live review HEADED per R-6.4.16), the Portainer Docker proxy for the release.

**Spec:** `docs/plans/2026-09-16-web-frontend-m6.4-chat-spec.md` — the authority; this plan argues from it and records where it deviates. Its fifteen rulings R-6.4.1–R-6.4.16 are owner direction. **Every line citation below is at `2194b57` (the spec's HEAD) and MUST be re-read at Task 0** — the spec was written on the day of M6.3's release and nothing has moved since, but the rule is the rule.

## Global Constraints

- **Branch:** `web-room-m6.4`, off `2194b57` (PR #33's tip); explicit `git add` paths; the `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and the executing `Claude-Session:` trailers on every commit.
- **Blast radius (spec §9.6):** the five fenced crates EMPTY in `git diff master`; `SEMANTICS_REV` **7**; `crates/yokohama-cli`'s diff EMPTY (no `pub` is needed this cycle); goldens and `guide_sync` untouched.
- **Per-edit gate:** `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings` when `crates/` moved; `cd web && npm run typecheck && npm test` when `web/` moved, and `npm run e2e` at Tasks 7 and 8. The whole-game gate (`cargo test -p yokohama-web --test play_a_whole_game`) runs per-edit locally and stays out of CI (M6.1 owner ruling); its vs-bot arm can livelock on a seed (owner item 2) — a red there is read before it is believed.
- **The wire corpus is a contract**: Tasks 4 and 5 regenerate (`UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then WITHOUT the variable, then `cd web && npm run typecheck`), every diff read and reported in the ledger. **Every `ClientState`, `HistoricalFrame`, `TableView` and `DealView` fixture must be byte-identical** at the end of Task 5.
- **The spec's rulings**, one line each: seats and watchers post, a bot never (R-6.4.1); one identity, the lobby's label rule, a seat's name is the seat's (R-6.4.2); the name at the door, per browser (R-6.4.3); the stream held at every visible status behind a posting guard (R-6.4.4); chat on four screens (R-6.4.5); `YOKOHAMA_WEB_LOBBY_CHAT` on|off, default on (R-6.4.6); a count on the signal (R-6.4.7); `YOKOHAMA_WEB_CHAT_KEEP` 500|all and `YOKOHAMA_WEB_CHAT_TEXT` 500, trimmed on append and never refused on read (R-6.4.8); `.chat` reserved, `_house.chat.json`, seq never reused (R-6.4.9); the panel last and folded everywhere (R-6.4.10); the watcher's one write (R-6.4.11); the email pin narrowed (R-6.4.12); the seat cells at 2P/3P/4P (R-6.4.13); nothing else (R-6.4.14); the tagged author and one writer (R-6.4.15); the live review headed (R-6.4.16).
- **Counts** are the M6.3 close's figures (server 191 = 181 + 2 + 8; CLI 851; client 587 in 35; e2e 39 in 8; corpus 71), re-measured at Task 0 and Task 10, never copied.
- **Two rules written in blood (handover §6):** every agent writes its full report to a transcript file and replies with the path; never send an implementer a follow-up without reading `git log -1` and `git status` first, and never rewrite a reviewed SHA.

---

## Execution model

As M6.3's: Opus implementers and per-task Opus reviewers, Fable orchestrating and running the whole-branch review clean; the SDD workspace `.superpowers/sdd/2026-09-16-web-frontend-m6.4-plan/progress.md`, gitignored. One outward-facing step needs the owner: the release (Task 9). The live review (Task 10) is HEADED (R-6.4.16) and runs from this workstation through WARP with three claimed identities.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/yokohama-web/src/chat.rs` (new), `store.rs`, `lib.rs` (`pub mod chat;`) | `Doc::{Chat, House}`, the second reservation, the three-kind delete, `ChatDoc`/`Message`/`Author`/`ChatLimits`, `check_shape`, `check_text`, `trim` | 1 |
| `crates/yokohama-web/src/games.rs`, `tables.rs`, `signal.rs` | `chat_append`, `house_append`, `chat_last` and its cache, `house_lock`, `House { seq, chat }`, `bump_house_chat`, `delete` clears the cache | 2 |
| `crates/yokohama-web/src/chat.rs` (`ChatConfig`), `lib.rs` (`AppState.chat`, `Health`), `main.rs` | The three variables, their parse, the health keys, the in-memory constructors | 3 |
| `crates/yokohama-web/src/lib.rs` | `ChatPost`, `ChatQuery`, `ChatPage`, the four handlers, `Events.chat`, the house frame, the grid, the sweep's exemption, the refusal-order probes | 4 |
| `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/*.json`, `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/api.ts` | The eleven fixtures, the narrowed pin, the client types and calls | 5 |
| `web/src/Chat.tsx` (+ `.test.tsx`, new), `web/src/chatFold.ts` (new), `web/src/styles.css` | The one panel | 6 |
| `web/src/App.tsx`, `Dealing.tsx`, `OpenTable.tsx`, `Lobby.tsx`, `SignIn.tsx`, `Help.tsx` (+ their tests) | The four hosts, the always-held stream and the posting guard, the name field, the help | 7 |
| `crates/yokohama-web/tests/play_a_whole_game.rs`, `web/e2e/two_browsers.spec.ts` | The gate's chat exchange; the three-browser exit | 8 |
| `deploy/RUNBOOK.md` (§3.10), `deploy/Dockerfile`, `deploy/Dockerfile.runtime` | The release `0.5.0-m6.4`, the optional variables | 9 |
| `docs/plans/<date>-web-frontend-m6.4-close.md`, `CLAUDE.md`, `guidingpath.md`, `README.md`, `docs/handover.md` | The battery, the headed live review, the close | 10 |

---

## Plan-level rulings (deviations from the spec's letter, recorded here per R-P11)

- **R-plan-1 — `Doc::House` takes exactly one id.** `FsStore::path(Doc::House, id)` answers the fixed file only for `id == chat::HOUSE` and `Err`s for any other, so a caller cannot spell a second house by accident; `MemStore` keys `(Doc::House, "house")` naturally. The spec's "whatever id is passed" becomes "the one id, refused otherwise".
- **R-plan-2 — the house channel's value becomes a struct.** `Signals.house` goes from `watch::Sender<u64>` to `watch::Sender<House>` with `House { seq: u64, chat: u64 }`; `bump` adds to `seq`, `bump_house_chat` to `chat`; the five existing house-receiver cells read `.seq` where they read the bare number. The stream writes `{"seq": N, "chat": M}`.
- **R-plan-3 — the text rule is one function, run twice.** `chat::check_text(&str, &ChatLimits) -> Result<String, String>` (trimmed and returned, the `check_name` shape); the ROUTE runs it after `admitted` and before it builds the author, so §3.2's order holds; `Games::chat_append` runs it again so a second caller (R-6.4.15's bot) is bounded by the same rule.
- **R-plan-4 — the fold is one module.** `web/src/chatFold.ts` holds `loadFolded(scope)`/`saveFolded(scope, folded)` under `yokohama.chat.folded.<scope>` with the `sections.ts` idiom (default TRUE; a storage that throws reads as folded), so the panel's own file holds only the panel.
- **R-plan-5 — the `chat_last` cache lives on `Games`.** `chat_last: Mutex<HashMap<GameId, u64>>`, seeded from the document on a miss under no lock (a read), written by `chat_append` under the table lock, removed by `delete`. The house has no cache: its frame carries the CHANNEL's counter and the house routes read the document.
- **R-plan-6 — the chat block in the fixture walk runs BEFORE the move loop**, immediately after `let mut state = as_actor(&api, created).await;` (`wire_fixtures.rs:760`), because `events.json` is a first-sighting capture INSIDE the loop (`:827-829`) and the spec wants it to read `chat: 3`.
- **R-plan-7 — the e2e's no-reopen claim is counted from the first game-frame reply.** For each player page a `request` listener counts `/stream` opens on this table's path; the count is READ once the game screen's first `GET /api/games/{id}` has replied and asserted unchanged at the end. The watcher's exact-one count stays as it is.

---

### Task 0: Pre-flight and baseline

- [ ] **Step 1: Re-read every citation.** `grep -n 'pub enum Doc\|RESERVED_ID_SUFFIX\|pub fn is_safe_id\|fn spells_a_table_document\|fn delete\|fn list' crates/yokohama-web/src/store.rs`; `grep -n 'pub struct Events\|fn events_of\|fn events_at\|async fn stream\|async fn house_stream\|fn admitted\|const NOT_SEATED\|fn spectators_write_nothing\|pub struct Health\|async fn health\|pub struct AppState\|fn admission_routes\|fn session_routes\|fn the_whole_admission_table\|fn a_spectator_is_refused_every_write' crates/yokohama-web/src/lib.rs`; `grep -n 'pub fn lock_for\|pub fn persist\|pub fn delete\|pub fn unlock\|fn check_name\|fn check_shape\|fn read_table\|fn write_table' crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs`; `grep -n 'house\|subscribe_house' crates/yokohama-web/src/signal.rs`; `grep -n 'const link = useSignal\|const send = useCallback\|arriving.current\|pending.current\|<aside className="side">\|<Dashboard' web/src/App.tsx`; `grep -n 'who-strip\|const take\|usePolled' web/src/Lobby.tsx web/src/OpenTable.tsx web/src/Dealing.tsx`; `grep -n 'let mut state = as_actor\|"events"\|fn no_payload_built_for_someone_else' crates/yokohama-web/tests/wire_fixtures.rs`. Write the current lines into the ledger's pre-flight table beside the spec's numbers.

- [ ] **Step 2: Re-count.** `cargo test -p yokohama-web 2>&1 | grep 'test result'`; `cd web && npm test 2>&1 | grep -E 'Test Files|Tests '`; `ls web/src/wire/*.json | wc -l`; `df -h . | tail -1` (owner item 4: a failed Playwright run writes a multi-GB trace; clear `web/test-results/` if the disk is under 12 GiB). Record them.

- [ ] **Step 3: Open the ledger** at `.superpowers/sdd/2026-09-16-web-frontend-m6.4-plan/progress.md` with the plan path, the base commit, the counts, and the empty rulings table.

---

### Task 1: The documents — `chat.rs`, `Doc::{Chat, House}`, the second reservation (spec §2.1–§2.3, R-6.4.8, R-6.4.9, R-6.4.15)

**Files:**
- Create: `crates/yokohama-web/src/chat.rs`
- Modify: `crates/yokohama-web/src/store.rs` (`Doc`, `suffix`, `RESERVED_ID_SUFFIX`, `is_safe_id`, `FsStore::path`, `FsStore::list`, `FsStore::delete`, `MemStore::delete`), `crates/yokohama-web/src/lib.rs` (`pub mod chat;`)
- Test: `crates/yokohama-web/src/chat.rs`, `crates/yokohama-web/src/store.rs`

**Interfaces:**
- Produces: `store::Doc::{Record, Table, Chat, House}`; `chat::HOUSE: &str = "house"`; `chat::CHAT_SCHEMA: u32 = 1`; `chat::ChatDoc { schema, id, last, messages }`; `chat::Message { seq, created_at, author, text }`; `chat::Author::{Seat { seat: u8, user: String, name: Option<String> }, Watcher { user, name }, Bot { seat, name }}`; `chat::ChatLimits { keep: Option<usize>, text: usize }` with `ChatLimits::default()` = 500/500; `chat::check_text(raw: &str, limits: &ChatLimits) -> Result<String, String>`; `chat::check_shape(doc: &ChatDoc, players: Option<u8>) -> Result<(), String>`; `ChatDoc::empty(id: &str) -> ChatDoc`; `ChatDoc::trim(&mut self, keep: Option<usize>)`.

- [ ] **Step 1: Write the failing store tests** (in `store.rs`'s `mod tests`, beside `the_two_kinds_are_separate_documents_under_one_id`):

```rust
    /// **Three kinds under one dotted id, on both stores** (M6.4 §2.1). The
    /// M6.3-era cell proved two; a store that separated two and confused the
    /// third would pass it.
    #[test]
    fn the_three_kinds_are_separate_documents_under_one_id() {
        for (name, store) in both("three-kinds") {
            store.write_if_version(Doc::Record, "a.b", "{\"r\":0}", None).expect(name);
            store.write_if_version(Doc::Table, "a.b", "{\"t\":0}", None).expect(name);
            store.write_if_version(Doc::Chat, "a.b", "{\"c\":0}", None).expect(name);
            assert_eq!(store.read(Doc::Chat, "a.b").expect(name).expect("present").0, "{\"c\":0}", "{name}");
            assert_eq!(store.read(Doc::Record, "a.b").expect(name).expect("present").0, "{\"r\":0}", "{name}");
            assert_eq!(store.list(Doc::Record).expect(name), vec!["a.b".to_string()], "{name}: the chat file is not a record");
            assert_eq!(store.list(Doc::Chat).expect(name), vec!["a.b".to_string()], "{name}");
            store.delete("a.b").expect(name);
            assert!(store.read(Doc::Chat, "a.b").expect(name).is_none(), "{name}: delete removes the chat document too");
        }
    }

    /// **The house is one file nothing can spell, and delete never touches it.**
    #[test]
    fn the_house_document_is_one_fixed_file_outside_every_game() {
        for (name, store) in both("house") {
            store.write_if_version(Doc::House, crate::chat::HOUSE, "{\"h\":0}", None).expect(name);
            assert_eq!(store.read(Doc::House, crate::chat::HOUSE).expect(name).expect("present").0, "{\"h\":0}", "{name}");
            assert!(store.list(Doc::Record).expect(name).is_empty(), "{name}: the house is no record");
            assert!(store.list(Doc::Table).expect(name).is_empty(), "{name}: and no table");
            assert!(store.read(Doc::House, "house2").is_err(), "{name}: the house has one id (R-plan-1)");
            store.write_if_version(Doc::Record, "g", "{\"r\":1}", None).expect(name);
            store.delete("g").expect(name);
            assert!(store.read(Doc::House, crate::chat::HOUSE).expect(name).is_some(), "{name}: a game's delete leaves the house");
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
```

Run: `cargo test -p yokohama-web store::tests 2>&1 | grep -E 'FAILED|error\[' | head` — Expected: compile errors (`Doc::Chat`, `Doc::House`, `crate::chat` missing).

- [ ] **Step 2: The store.** In `store.rs`:

```rust
pub enum Doc {
    Record,
    Table,
    /// The table's chat — `<id>.chat.json` (M6.4, spec §2.1). Its own document
    /// so a message and a move are different questions that land
    /// independently, and so it is never in the record.
    Chat,
    /// The ONE house chat, at a file no game id can spell: `_house.chat.json`.
    /// A leading underscore fails `is_safe_id`'s first-character rule, so it
    /// needs no reservation and `list(Doc::Record)` excludes it by construction.
    House,
}

impl Doc {
    fn suffix(self) -> &'static str {
        match self {
            Self::Record => ".json",
            Self::Table => ".table.json",
            Self::Chat => ".chat.json",
            Self::House => ".chat.json",
        }
    }
}

/// The id suffixes the non-record kinds reserve, which no game may end with.
const RESERVED_ID_SUFFIXES: [&str; 2] = [".table", ".chat"];

fn spells_another_document(id: &str) -> bool {
    RESERVED_ID_SUFFIXES.iter().any(|s| {
        id.len() >= s.len() && id[id.len() - s.len()..].eq_ignore_ascii_case(s)
    })
}
```

`is_safe_id`'s last term becomes `&& !spells_another_document(id)`; the doc comment on the constant is rewritten from `RESERVED_ID_SUFFIX`'s text to name both. **The derivation assert** (`store.rs:619-627`, `assert_eq!(Doc::Table.suffix(), format!("{RESERVED_ID_SUFFIX}{}", Doc::Record.suffix()))`) becomes a loop, so each reserved suffix is derived from its own arm and a drift in `Doc::Chat.suffix()` fails here rather than reopening the hole:

```rust
        for (suffix, doc) in RESERVED_ID_SUFFIXES.iter().zip([Doc::Table, Doc::Chat]) {
            assert_eq!(
                doc.suffix(),
                format!("{suffix}{}", Doc::Record.suffix()),
                "the id guard reserves exactly what {doc:?} appends",
            );
        }
``` `FsStore::path`:

```rust
    fn path(&self, doc: Doc, id: &str) -> Result<PathBuf, String> {
        if doc == Doc::House {
            // R-plan-1: one house, one id. A second name would be a second
            // file nothing lists and nothing deletes.
            if id != crate::chat::HOUSE {
                return Err(format!("the house chat has one id, {:?}, not {id:?}", crate::chat::HOUSE));
            }
            return Ok(self.dir.join(format!("_{id}{}", doc.suffix())));
        }
        if !is_safe_id(id) {
            return Err(format!("{id:?} is not a usable game name"));
        }
        Ok(self.dir.join(format!("{id}{}", doc.suffix())))
    }
```

`FsStore::list`: the redundant-defence branch becomes `if doc == Doc::Record && (name.ends_with(Doc::Table.suffix()) || name.ends_with(Doc::Chat.suffix())) { return None; }`; for `Doc::House`, `list` answers `vec![HOUSE.to_string()]` when the file exists and `vec![]` otherwise (a `std::fs::metadata` on the fixed path, before the directory scan). `FsStore::delete` and `MemStore::delete` iterate `[Doc::Record, Doc::Table, Doc::Chat]`; the trait doc's "Both documents" becomes "Every document of the game — the record, the table and the chat; never the house." `MemStore::list(Doc::House)` filters keys by kind as it does for the others.

- [ ] **Step 3: Write the failing `chat.rs` tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn seat(seq: u64, s: u8, text: &str) -> Message {
        Message { seq, created_at: 1, author: Author::Seat { seat: s, user: "red@t.test".into(), name: None }, text: text.into() }
    }
    fn doc(msgs: Vec<Message>) -> ChatDoc {
        let last = msgs.last().map_or(0, |m| m.seq);
        ChatDoc { schema: CHAT_SCHEMA, id: "t".into(), last, messages: msgs }
    }

    #[test]
    fn check_text_trims_and_bounds_by_characters_not_bytes() {
        let limits = ChatLimits { keep: Some(500), text: 5 };
        assert_eq!(check_text("  héllo ", &limits).unwrap(), "héllo");
        assert!(check_text("hello!", &limits).is_err(), "six characters over a cap of five");
        assert!(check_text("   ", &limits).is_err(), "empty after trimming");
        assert!(check_text("a\nb", &limits).is_err(), "a control character");
    }

    #[test]
    fn the_seat_bound_is_the_table_s_player_count_at_two_three_and_four() {
        for players in [2u8, 3, 4] {
            let ok = doc(vec![seat(1, players - 1, "hi")]);
            assert!(check_shape(&ok, Some(players)).is_ok(), "{players}P: the last seat is in range");
            let bad = doc(vec![seat(1, players, "hi")]);
            assert!(check_shape(&bad, Some(players)).is_err(), "{players}P: seat {players} would reach PlayerId::new");
        }
        let bot = doc(vec![Message { seq: 1, created_at: 1, author: Author::Bot { seat: 7, name: None }, text: "x".into() }]);
        assert!(check_shape(&bot, Some(2)).is_err(), "a bot author is bounded too");
    }

    #[test]
    fn the_house_carries_no_seat_author() {
        assert!(check_shape(&doc(vec![seat(1, 0, "hi")]), None).is_err());
        let w = Message { seq: 1, created_at: 1, author: Author::Watcher { user: "g@t.test".into(), name: Some("G".into()) }, text: "hi".into() };
        assert!(check_shape(&doc(vec![w]), None).is_ok());
    }

    #[test]
    fn seq_must_increase_and_last_must_cover_it() {
        let mut d = doc(vec![seat(1, 0, "a"), seat(2, 0, "b")]);
        assert!(check_shape(&d, Some(2)).is_ok());
        d.messages[1].seq = 1;
        assert!(check_shape(&d, Some(2)).is_err(), "a repeated seq");
        let mut d = doc(vec![seat(3, 0, "a")]);
        d.last = 2;
        assert!(check_shape(&d, Some(2)).is_err(), "last behind the newest");
    }

    #[test]
    fn malformed_fields_are_refused_but_length_is_not() {
        let long = "x".repeat(600);
        let d = doc(vec![seat(1, 0, &long)]);
        assert!(check_shape(&d, Some(2)).is_ok(), "a stored over-cap text is SERVED (R-6.4.8)");
        let many: Vec<Message> = (1..=600).map(|i| seat(i, 0, "m")).collect();
        assert!(check_shape(&doc(many), Some(2)).is_ok(), "an over-keep document is served and trimmed later");
        let mut bad_user = doc(vec![seat(1, 0, "a")]);
        bad_user.messages[0].author = Author::Seat { seat: 0, user: "not an email".into(), name: None };
        assert!(check_shape(&bad_user, Some(2)).is_err());
        let mut bad_name = doc(vec![seat(1, 0, "a")]);
        bad_name.messages[0].author = Author::Seat { seat: 0, user: "r@t.test".into(), name: Some("x".repeat(65)) };
        assert!(check_shape(&bad_name, Some(2)).is_err());
        let ctl = doc(vec![seat(1, 0, "a\u{7}b")]);
        assert!(check_shape(&ctl, Some(2)).is_err(), "a control character in a stored text");
        let mut newer = doc(vec![]);
        newer.schema = CHAT_SCHEMA + 1;
        assert!(check_shape(&newer, Some(2)).is_err());
    }

    #[test]
    fn trim_drops_the_oldest_and_keeps_last_counting() {
        let mut d = doc((1..=7).map(|i| seat(i, 0, "m")).collect());
        d.trim(Some(5));
        assert_eq!(d.messages.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![3, 4, 5, 6, 7]);
        assert_eq!(d.last, 7, "the number is never reused");
        d.trim(None);
        assert_eq!(d.messages.len(), 5, "`all` trims nothing");
    }

    #[test]
    fn the_author_serializes_tagged_like_a_seat_doc() {
        let m = seat(1, 0, "hi");
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["author"]["kind"], "seat");
        assert_eq!(v["author"]["seat"], 0);
        let w: Message = serde_json::from_value(serde_json::json!({
            "seq": 2, "created_at": 1, "text": "x",
            "author": { "kind": "watcher", "user": "g@t.test", "name": null }
        })).unwrap();
        assert!(matches!(w.author, Author::Watcher { .. }));
        let b: Message = serde_json::from_value(serde_json::json!({
            "seq": 3, "created_at": 1, "text": "x", "author": { "kind": "bot", "seat": 1, "name": null }
        })).unwrap();
        assert!(matches!(b.author, Author::Bot { seat: 1, .. }), "the arm no route produces still reads (R-6.4.15)");
    }
}
```

- [ ] **Step 4: `chat.rs`**

```rust
//! **The chat documents (M6.4, spec §2).** One per table (`<id>.chat.json`)
//! and one for the house (`_house.chat.json`): an append-only list whose
//! numbers start at 1 and are never reused, trimmed to a configured keep from
//! the OLDEST end, never in the record and never undone (R-6.4.9).
//!
//! **Every stored field is an INPUT** (the M6.1 lesson): `check_shape` runs on
//! read — as the table document's does — AND on write, which the table's does
//! not, because this process appends to a chat document on every message and a
//! malformed author written once would poison the next read under the table's
//! lock. A seat is bounded by the table's player count not because anything
//! here hands it to `PlayerId::new` (nothing does) but because the client
//! indexes `colors[seat]` with it, and an input is bounded where it enters.
//! Two things are deliberately NOT refused on read (R-6.4.8): a text longer than
//! the configured cap (served as stored) and a document longer than the
//! configured keep (trimmed on its next append), so lowering either number on
//! a running volume cannot take a table out.

use serde::{Deserialize, Serialize};

use crate::identity::User;
use crate::tables::check_name;

/// This document's own shape version, unrelated to `SEMANTICS_REV`.
pub const CHAT_SCHEMA: u32 = 1;
/// The one id the house document answers to (R-plan-1).
pub const HOUSE: &str = "house";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDoc {
    pub schema: u32,
    pub id: String,
    /// The newest number ever assigned — NOT `messages.len()`, and not the
    /// newest retained: a trim drops messages and leaves this alone, so a
    /// client's `after` cursor stays valid across it.
    pub last: u64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Message {
    pub seq: u64,
    /// Unix seconds. **Named `created_at` because the corpus freezer pins a
    /// clock by KEY NAME** (`wire_fixtures.rs:256-293`); a field spelled `at`
    /// would churn six fixtures on every regeneration.
    pub created_at: u64,
    pub author: Author,
    pub text: String,
}

/// Who said it. **Tagged, the `SeatDoc` shape, so an arm can be added
/// additively on both sides of the wire (R-6.4.15).** `Bot` is read and written
/// and PRODUCED BY NO ROUTE in this cycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Author {
    Seat { seat: u8, user: String, #[serde(default)] name: Option<String> },
    Watcher { user: String, #[serde(default)] name: Option<String> },
    Bot { seat: u8, #[serde(default)] name: Option<String> },
}

/// The two retention numbers (R-6.4.8), from the launch variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatLimits {
    /// Messages retained per document; `None` is `all`.
    pub keep: Option<usize>,
    /// Characters per message.
    pub text: usize,
}

impl Default for ChatLimits {
    fn default() -> Self {
        Self { keep: Some(500), text: 500 }
    }
}

impl ChatDoc {
    pub fn empty(id: &str) -> Self {
        Self { schema: CHAT_SCHEMA, id: id.to_string(), last: 0, messages: Vec::new() }
    }

    /// Drop the OLDEST until `keep` remain; `last` is untouched.
    pub fn trim(&mut self, keep: Option<usize>) {
        if let Some(k) = keep {
            if self.messages.len() > k {
                let drop = self.messages.len() - k;
                self.messages.drain(..drop);
            }
        }
    }
}

/// The text rule, applied on POST (R-plan-3: by the route and again by the
/// appender). Trimmed and returned, so " hi " and "hi" are one message.
pub fn check_text(raw: &str, limits: &ChatLimits) -> Result<String, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("a message needs some words".to_string());
    }
    if text.chars().count() > limits.text {
        return Err(format!("a message is at most {} characters", limits.text));
    }
    if text.chars().any(char::is_control) {
        return Err("a message is one line, with no control characters".to_string());
    }
    Ok(text.to_string())
}

/// The shape rule, on read AND on write. `players` is the table's count, or
/// `None` at the house, where no author holds a seat.
pub fn check_shape(doc: &ChatDoc, players: Option<u8>) -> Result<(), String> {
    if doc.schema > CHAT_SCHEMA {
        return Err(format!("schema {} — this server reads up to {CHAT_SCHEMA}; a newer server wrote it", doc.schema));
    }
    let mut prev = 0u64;
    for (i, m) in doc.messages.iter().enumerate() {
        if m.seq <= prev {
            return Err(format!("message {i} has seq {} after {prev}: numbers must increase", m.seq));
        }
        prev = m.seq;
        if m.text.is_empty() || m.text.chars().any(char::is_control) {
            return Err(format!("message {i}'s text is empty or carries a control character"));
        }
        let (user, name, seat) = match &m.author {
            Author::Seat { seat, user, name } => (Some(user), name.as_deref(), Some(*seat)),
            Author::Watcher { user, name } => (Some(user), name.as_deref(), None),
            Author::Bot { seat, name } => (None, name.as_deref(), Some(*seat)),
        };
        if let Some(u) = user {
            if User::parse(u).is_err() {
                return Err(format!("message {i}'s author {u:?} is not an address this server writes"));
            }
        }
        if let Some(n) = name {
            check_name(n).map_err(|why| format!("message {i}'s author name: {why}"))?;
        }
        match (seat, players) {
            (Some(s), Some(p)) if s >= p => return Err(format!("message {i} names seat {s} at a {p}-seat table")),
            (Some(s), None) => return Err(format!("message {i} names seat {s} in the house, which has no seats")),
            _ => {}
        }
    }
    if doc.last < prev {
        return Err(format!("last {} is behind the newest message {prev}", doc.last));
    }
    Ok(())
}
```

Add `pub mod chat;` to `lib.rs` (alphabetical, after `assets`). `check_name` is `pub(crate)` in `tables.rs` already.

- [ ] **Step 5: Run and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`

```bash
git add crates/yokohama-web/src/chat.rs crates/yokohama-web/src/store.rs crates/yokohama-web/src/lib.rs
git commit -m "feat(web): the chat documents — a third store kind per table and one house file no id can spell, the second reserved suffix, the tagged author with the bot arm no route produces, the shape rule that bounds every stored field and refuses nothing for length (spec §2, R-6.4.8, R-6.4.9, R-6.4.15)"
```

---

### Task 2: The append — `chat_append`, `house_append`, `chat_last`, the house counter (spec §2.4, §4, R-6.4.7, R-plan-2, R-plan-5)

**Files:**
- Modify: `crates/yokohama-web/src/signal.rs` (`House`, `bump_house_chat`, `subscribe_house`), `crates/yokohama-web/src/games.rs` (`Games.chat_last`, `Games.house_lock`, `Games.limits`, `Games::new`, `delete`'s cache removal), `crates/yokohama-web/src/tables.rs` (`chat_append`, `house_append`, `chat_last`, `read_chat`, `write_chat`, `read_house`, `write_house`; `delete`)
- Test: `crates/yokohama-web/src/signal.rs`, `crates/yokohama-web/src/tables.rs`

**Interfaces:**
- Consumes: `chat::{ChatDoc, Message, Author, ChatLimits, check_text, check_shape, HOUSE}`, `store::Doc::{Chat, House}` (Task 1).
- Produces: `signal::House { pub seq: u64, pub chat: u64 }`; `Signals::bump_house_chat(&self)`; `Signals::subscribe_house(&self) -> watch::Receiver<House>`; `Games::new(rules, store)` unchanged in signature, plus `Games::with_limits(rules, store, limits: ChatLimits)`; `Games::chat_append(&self, id: &str, author: Author, text: &str) -> Result<Message, GameError>`; `Games::house_append(&self, author: Author, text: &str) -> Result<Message, GameError>`; `Games::chat_last(&self, id: &str) -> u64`; `Games::chat_page(&self, id: &str, after: u64) -> Result<(Vec<Message>, u64), GameError>`; `Games::house_page(&self, after: u64) -> Result<(Vec<Message>, u64), GameError>`; `Games::limits(&self) -> ChatLimits`.

- [ ] **Step 1: The house channel's value (R-plan-2).** In `signal.rs`:

```rust
/// The house frame's two counters (M6.4, R-6.4.7). `seq` moves on every write a
/// lobby list reads (create, join, finalize, a move, a delete); `chat` moves on
/// a house message and on nothing else. Two counters and never a per-table
/// payload (R-3.4, amended).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct House {
    pub seq: u64,
    pub chat: u64,
}
```

`house: Mutex<Option<watch::Sender<House>>>`; `house_sender` builds `watch::channel(House::default()).0`; `bump` and `end` do `send_modify(|h| h.seq += 1)`; new:

```rust
    /// The house's CHAT counter only — a house message wakes every lobby and
    /// moves no table listing (R-6.4.7).
    pub fn bump_house_chat(&self) {
        self.house_sender().send_modify(|h| h.chat += 1);
    }
```

The cells: `bump_reaches_the_house_and_bump_table_does_not` reads `house.borrow_and_update().seq`; add

```rust
    #[test]
    fn a_house_chat_bump_moves_chat_and_leaves_seq() {
        let s = Signals::default();
        let mut house = s.subscribe_house();
        s.bump_house_chat();
        assert!(house.has_changed().unwrap());
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat), (0, 1));
        s.bump("t");
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat), (1, 1), "a table bump leaves the chat counter");
    }
```

Re-point the four house-receiver cells in `games.rs`/`tables.rs` that read the bare number (`grep -n 'subscribe_house' crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs`): they use `has_changed()` and stay; any `*house.borrow_and_update()` compared to a number becomes `.seq`. `house_stream` in `lib.rs` is Task 4's.

- [ ] **Step 2: Write the failing append cells** (in `tables.rs`'s `mod tests`, beside the bump cells):

```rust
    fn red_seat() -> Author {
        Author::Seat { seat: 0, user: "red@t.test".into(), name: Some("Red".into()) }
    }

    /// **An append lands, numbers from 1, and bumps the TABLE after its
    /// write** (spec §2.4).
    #[test]
    fn a_chat_append_writes_then_bumps_the_table_and_not_the_house() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        let mut rx = games.signals().subscribe(&id);
        let house = games.signals().subscribe_house();
        rx.borrow_and_update();
        let m = games.chat_append(&id, red_seat(), "  hello ").expect("an append");
        assert_eq!((m.seq, m.text.as_str()), (1, "hello"));
        assert!(rx.has_changed().unwrap(), "the table woke");
        assert!(!house.has_changed().unwrap(), "the house did not (R-6.4.7)");
        let (page, last) = games.chat_page(&id, 0).expect("a page");
        assert_eq!((page.len(), last, page[0].seq), (1, 1, 1));
        assert_eq!(games.chat_last(&id), 1);
        let (delta, _) = games.chat_page(&id, 1).expect("a page");
        assert!(delta.is_empty(), "after=last is empty");
    }

    /// **The text rule is the appender's too** (R-plan-3).
    #[test]
    fn an_append_refuses_an_empty_an_over_cap_and_a_control_text() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for bad in ["   ", "a\tb"] {
            assert!(matches!(games.chat_append(&id, red_seat(), bad), Err(GameError::BadRequest(_))), "{bad:?}");
        }
        let long = "x".repeat(501);
        assert!(matches!(games.chat_append(&id, red_seat(), &long), Err(GameError::BadRequest(_))));
        assert_eq!(games.chat_last(&id), 0, "nothing landed");
    }

    /// **The keep trims the OLDEST and `last` keeps counting** (R-6.4.8, R-6.4.9).
    #[test]
    fn the_keep_drops_the_oldest_and_never_reuses_a_number() {
        let games = Games::with_limits(
            Arc::new(Rules::standard()),
            Arc::new(MemStore::default()),
            ChatLimits { keep: Some(3), text: 500 },
        );
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for i in 1..=5 {
            games.chat_append(&id, red_seat(), &format!("m{i}")).expect("an append");
        }
        let (page, last) = games.chat_page(&id, 0).expect("a page");
        assert_eq!(page.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![3, 4, 5]);
        assert_eq!(last, 5);
        assert_eq!(games.chat_last(&id), 5);
    }

    /// **`all` keeps everything** (R-6.4.8).
    #[test]
    fn keep_all_trims_nothing() {
        let games = Games::with_limits(
            Arc::new(Rules::standard()),
            Arc::new(MemStore::default()),
            ChatLimits { keep: None, text: 500 },
        );
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for i in 1..=600 {
            games.chat_append(&id, red_seat(), &format!("m{i}")).expect("an append");
        }
        assert_eq!(games.chat_page(&id, 0).expect("a page").0.len(), 600);
    }

    /// **A document longer than the keep is read whole and trimmed on the
    /// next append, never refused** (R-6.4.8).
    #[test]
    fn an_over_keep_document_is_served_and_trimmed_on_the_next_append() {
        let store = Arc::new(MemStore::default());
        let wide = Games::with_limits(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>, ChatLimits { keep: None, text: 500 });
        let red = who("red@t.test");
        let id = wide.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        for i in 1..=10 {
            wide.chat_append(&id, red_seat(), &format!("m{i}")).expect("an append");
        }
        let narrow = Games::with_limits(Arc::new(Rules::standard()), store as Arc<dyn Store>, ChatLimits { keep: Some(4), text: 500 });
        assert_eq!(narrow.chat_page(&id, 0).expect("served, not refused").0.len(), 10);
        narrow.chat_append(&id, red_seat(), "m11").expect("an append");
        let (page, _) = narrow.chat_page(&id, 0).expect("a page");
        assert_eq!(page.iter().map(|m| m.seq).collect::<Vec<_>>(), vec![8, 9, 10, 11]);
    }

    /// **The count is seeded from the document on a cold miss** (R-plan-5).
    #[test]
    fn chat_last_is_seeded_from_the_document_when_this_process_has_not_seen_it() {
        let store = Arc::new(MemStore::default());
        let first = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        let red = who("red@t.test");
        let id = first.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        first.chat_append(&id, red_seat(), "one").expect("an append");
        first.chat_append(&id, red_seat(), "two").expect("an append");
        let second = Games::new(Arc::new(Rules::standard()), store as Arc<dyn Store>);
        assert_eq!(second.chat_last(&id), 2, "a fresh process reads the document once");
        assert_eq!(second.chat_last("nothing-here"), 0, "no document, no messages");
    }

    /// **The house append bumps the house's CHAT counter and no table.**
    #[test]
    fn a_house_append_bumps_the_house_chat_counter_only() {
        let games = games();
        let mut house = games.signals().subscribe_house();
        house.borrow_and_update();
        let m = games
            .house_append(Author::Watcher { user: "g@t.test".into(), name: Some("G".into()) }, "hi all")
            .expect("an append");
        assert_eq!(m.seq, 1);
        let h = *house.borrow_and_update();
        assert_eq!((h.seq, h.chat), (0, 1));
        assert!(matches!(games.house_append(Author::Seat { seat: 0, user: "r@t.test".into(), name: None }, "x"), Err(GameError::Server(_))), "a seat author is refused at the house by the shape rule");
        let (page, last) = games.house_page(0).expect("a page");
        assert_eq!((page.len(), last), (1, 1));
    }

    /// **A delete removes the chat document and forgets the count.**
    #[test]
    fn a_delete_removes_the_chat_and_its_count() {
        let games = games();
        let red = who("red@t.test");
        let id = games.create(&two_people("chatty", CORPUS_SEED), &red).expect("a table").id;
        games.chat_append(&id, red_seat(), "bye").expect("an append");
        games.delete(&id, &red).expect("a delete");
        assert!(games.store().read(Doc::Chat, &id).expect("a store").is_none());
        assert_eq!(games.chat_last(&id), 0);
    }
```

And the two GATED cells, the `FlipGate` shape (`tables.rs:3630-3735`) applied to the chat write:

```rust
    /// A [`MemStore`] that PARKS every `Doc::Chat` write and nothing else.
    struct ChatGate {
        inner: MemStore,
        at_write: Mutex<std::sync::mpsc::Sender<()>>,
        go: Mutex<std::sync::mpsc::Receiver<()>>,
    }
    impl Store for ChatGate {
        fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> { self.inner.list(doc) }
        fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> { self.inner.read(doc, id) }
        fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>) -> Result<Version, WriteError> {
            if doc == Doc::Chat || doc == Doc::House {
                let _ = self.at_write.lock().expect("the gate").send(());
                let _ = self.go.lock().expect("the gate").recv();
            }
            self.inner.write_if_version(doc, id, json, expected)
        }
        fn delete(&self, id: &str) -> Result<(), String> { self.inner.delete(id) }
    }

    /// **A chat bump precedes no write** (spec §2.4, mutation 1): the append's
    /// write is parked and the receiver is read QUIET while it sits there.
    #[test]
    fn a_chat_bump_precedes_no_write() {
        use std::sync::mpsc::channel;
        use std::time::Duration;
        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(ChatGate { inner: MemStore::default(), at_write: Mutex::new(at_tx), go: Mutex::new(go_rx) });
        let games = Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let red = who("red@t.test");
        let id = games.create(&two_people("parked", CORPUS_SEED), &red).expect("a table").id;
        let mut rx = games.signals().subscribe(&id);
        rx.borrow_and_update();
        let writer = {
            let (games, id) = (Arc::clone(&games), id.clone());
            std::thread::spawn(move || games.chat_append(&id, red_seat(), "parked"))
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the append reaches its write");
        assert!(!rx.has_changed().unwrap(), "the write is parked and the receiver is QUIET");
        go_tx.send(()).expect("the parked write is listening");
        writer.join().expect("the thread").expect("the append");
        assert!(rx.has_changed().unwrap(), "and wakes once it lands");
    }

    /// The same for the house (mutation 2).
    #[test]
    fn a_house_chat_bump_precedes_no_write() {
        use std::sync::mpsc::channel;
        use std::time::Duration;
        let (at_tx, at_rx) = channel();
        let (go_tx, go_rx) = channel();
        let store = Arc::new(ChatGate { inner: MemStore::default(), at_write: Mutex::new(at_tx), go: Mutex::new(go_rx) });
        let games = Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>));
        let mut house = games.signals().subscribe_house();
        house.borrow_and_update();
        let writer = {
            let games = Arc::clone(&games);
            std::thread::spawn(move || games.house_append(Author::Watcher { user: "g@t.test".into(), name: None }, "parked"))
        };
        at_rx.recv_timeout(Duration::from_secs(10)).expect("the append reaches its write");
        assert!(!house.has_changed().unwrap(), "quiet while parked");
        go_tx.send(()).expect("listening");
        writer.join().expect("the thread").expect("the append");
        assert_eq!(house.borrow_and_update().chat, 1);
    }
```

(`games()`, `who`, `two_people`, `CORPUS_SEED` are `games::test_support`'s; `Games::store()` exists — `wire_fixtures` uses it.) Run: expected compile errors naming the new functions.

- [ ] **Step 3: `Games` gains three fields and the appenders.** In `games.rs`, beside `signals`:

```rust
    /// The newest chat number per table this process has seen (R-plan-5):
    /// seeded from the document on a miss, advanced by every append, dropped by
    /// a delete. Read by the stream's per-bump closure so the count costs no
    /// document parse per connection. **Per PROCESS**: a second process's
    /// appends are invisible here until the store is a database (M6.5).
    pub(crate) chat_last: Mutex<HashMap<GameId, u64>>,
    /// Serializes the house's appends — the house has no `lock_for` entry
    /// because it is not a table.
    pub(crate) house_lock: Mutex<()>,
    /// The retention numbers (R-6.4.8), from the launch variables.
    pub(crate) limits: ChatLimits,
```

`Games::new` becomes `Self::with_limits(rules, store, ChatLimits::default())` and `with_limits` initializes all three (`HashMap::new()`, `Mutex::new(())`, `limits`); `pub fn limits(&self) -> ChatLimits`. In `tables.rs`, beside `read_table`/`write_table`:

```rust
    pub(crate) fn read_chat(&self, id: &str, players: Option<u8>) -> Result<(ChatDoc, Option<Version>), GameError> {
        let doc = if players.is_some() { Doc::Chat } else { Doc::House };
        let key = if players.is_some() { id } else { HOUSE };
        let Some((json, version)) = self.store.read(doc, key).map_err(GameError::Server)? else {
            return Ok((ChatDoc::empty(key), None));
        };
        let parsed: ChatDoc = serde_json::from_str(&json)
            .map_err(|e| GameError::Server(format!("{key:?}'s chat document does not parse: {e}")))?;
        check_shape(&parsed, players)
            .map_err(|why| GameError::Server(format!("{key:?}'s chat document is malformed: {why}")))?;
        Ok((parsed, Some(version)))
    }

    fn write_chat(&self, doc: &ChatDoc, players: Option<u8>, expected: Option<Version>) -> Result<Version, GameError> {
        check_shape(doc, players).map_err(GameError::Server)?;
        let kind = if players.is_some() { Doc::Chat } else { Doc::House };
        let json = serde_json::to_string_pretty(doc)
            .map_err(|e| GameError::Server(format!("the chat document does not serialize: {e}")))?;
        match self.store.write_if_version(kind, &doc.id, &json, expected) {
            Ok(v) => Ok(v),
            Err(WriteError::Conflict { .. }) => Err(GameError::Conflict(format!("{:?}'s chat changed while this message was in flight", doc.id))),
            Err(WriteError::Io(e)) => Err(GameError::Server(e)),
        }
    }

    /// **The one writer** (R-6.4.15 (b)): the text rule, the table's own lock, the
    /// read, the append, the trim, the compare-and-swap, THEN the bump.
    pub fn chat_append(&self, id: &str, author: Author, text: &str) -> Result<Message, GameError> {
        let text = check_text(text, &self.limits).map_err(GameError::BadRequest)?;
        let lock = self.lock_for(id);
        let _serialized = lock.lock().expect("table lock");
        let (table, _) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        let (mut doc, version) = self.read_chat(id, Some(table.players))?;
        let message = Message { seq: doc.last + 1, created_at: now_unix(), author, text };
        doc.last = message.seq;
        doc.messages.push(message.clone());
        doc.trim(self.limits.keep);
        self.write_chat(&doc, Some(table.players), version)?;
        self.chat_last.lock().expect("games lock").insert(id.to_string(), doc.last);
        // AFTER the write, never before (the gated cell).
        self.signals.bump_table(id);
        Ok(message)
    }

    pub fn house_append(&self, author: Author, text: &str) -> Result<Message, GameError> {
        let text = check_text(text, &self.limits).map_err(GameError::BadRequest)?;
        let _serialized = self.house_lock.lock().expect("house lock");
        let (mut doc, version) = self.read_chat(HOUSE, None)?;
        let message = Message { seq: doc.last + 1, created_at: now_unix(), author, text };
        doc.last = message.seq;
        doc.messages.push(message.clone());
        doc.trim(self.limits.keep);
        self.write_chat(&doc, None, version)?;
        self.signals.bump_house_chat();
        Ok(message)
    }

    /// The retained messages numbered above `after`, and `last`.
    pub fn chat_page(&self, id: &str, after: u64) -> Result<(Vec<Message>, u64), GameError> {
        let (table, _) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        let (doc, _) = self.read_chat(id, Some(table.players))?;
        Ok((doc.messages.into_iter().filter(|m| m.seq > after).collect(), doc.last))
    }

    pub fn house_page(&self, after: u64) -> Result<(Vec<Message>, u64), GameError> {
        let (doc, _) = self.read_chat(HOUSE, None)?;
        Ok((doc.messages.into_iter().filter(|m| m.seq > after).collect(), doc.last))
    }

    /// The count `Events.chat` carries (R-plan-5).
    pub fn chat_last(&self, id: &str) -> u64 {
        if let Some(n) = self.chat_last.lock().expect("games lock").get(id) {
            return *n;
        }
        // A miss: read once. The player count is unknown here and the seat
        // bound is a WRITE-side and route-side check; `None` would refuse a
        // seat author, so the document is parsed without the shape rule and
        // only its `last` is read.
        let n = match self.store.read(Doc::Chat, id) {
            Ok(Some((json, _))) => serde_json::from_str::<ChatDoc>(&json).map(|d| d.last).unwrap_or(0),
            Ok(None) => 0,
            // A store that cannot be read answers 0 and is NOT cached, so the
            // next call retries (spec §2.4); every route beside this one is
            // refusing on the same failure.
            Err(_) => return 0,
        };
        self.chat_last.lock().expect("games lock").insert(id.to_string(), n);
        n
    }
```

In `delete`, after `self.live.lock()…remove(id);` add `self.chat_last.lock().expect("games lock").remove(id);` — the store's `delete` already removes the chat file (Task 1). In `create` (`tables.rs`, after its table write succeeds, beside the bump at `:759`) add the same `remove(&doc.id)`, so a name deleted and reopened starts at 0 (spec §2.4); the `a_delete_removes_the_chat_and_its_count` cell gains a recreate-and-read-0 tail. Imports: `use crate::chat::{check_shape, check_text, Author, ChatDoc, ChatLimits, Message, HOUSE};` and `use crate::table::now_unix;` in `tables.rs`; `ChatLimits` in `games.rs`.

- [ ] **Step 4: Run and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`

```bash
git add crates/yokohama-web/src/signal.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/tables.rs
git commit -m "feat(web): the one chat writer — an append under the table's own lock, numbered from one and never reused, trimmed to the configured keep, swapped on the version read and bumped AFTER the write (proven with a gated store); the house append and its own counter on a two-counter house frame; the per-process count the stream reads (spec §2.4, §4, R-6.4.7, R-6.4.8, R-plan-2, R-plan-5)"
```

---

### Task 3: The switch and the limits — `ChatConfig`, `AppState.chat`, `Health`, `main.rs` (spec §5, R-6.4.6, R-6.4.8)

**Files:**
- Modify: `crates/yokohama-web/src/chat.rs` (`ChatConfig`), `crates/yokohama-web/src/lib.rs` (`AppState`, its constructors, `Health`, `health`), `crates/yokohama-web/src/main.rs`
- Test: `crates/yokohama-web/src/chat.rs`, `crates/yokohama-web/src/lib.rs`

**Interfaces:**
- Consumes: `chat::ChatLimits` (Task 1); `Games::with_limits` (Task 2).
- Produces: `chat::ChatConfig { pub lobby: bool, pub limits: ChatLimits }` with `Default` (on, 500/500), `ChatConfig::from_env() -> Result<Self, String>`, `ChatConfig::from_env_values(lobby: Option<&str>, keep: Option<&str>, text: Option<&str>) -> Result<Self, String>`; `pub const LOBBY_VAR: &str = "YOKOHAMA_WEB_LOBBY_CHAT"`, `KEEP_VAR`, `TEXT_VAR`; `AppState.chat: ChatConfig`; `AppState::new(identity, chat)`, `AppState::with_store_and_chat(store, identity, chat)`, `AppState::in_memory_with_chat(chat)` (the existing `in_memory`, `in_memory_with`, `in_memory_with_heartbeat`, `with_store` keep their signatures and default the chat); `Health { …, lobby_chat: bool, chat_keep: Option<u64>, chat_text: u64 }`.

- [ ] **Step 1: The failing config cells** (in `chat.rs`'s tests):

```rust
    #[test]
    fn the_three_variables_parse_with_their_defaults_and_refuse_by_name() {
        let d = ChatConfig::from_env_values(None, None, None).unwrap();
        assert_eq!((d.lobby, d.limits.keep, d.limits.text), (true, Some(500), 500));
        assert!(ChatConfig::from_env_values(Some("on"), None, None).unwrap().lobby);
        assert!(!ChatConfig::from_env_values(Some("off"), None, None).unwrap().lobby);
        assert!(!ChatConfig::from_env_values(Some(" OFF "), None, None).unwrap().lobby, "trimmed, case-insensitive");
        let e = ChatConfig::from_env_values(Some("maybe"), None, None).unwrap_err();
        assert!(e.contains(LOBBY_VAR) && e.contains("maybe"), "{e}");
        assert_eq!(ChatConfig::from_env_values(None, Some("all"), None).unwrap().limits.keep, None);
        assert_eq!(ChatConfig::from_env_values(None, Some("20"), None).unwrap().limits.keep, Some(20));
        for bad in ["0", "-1", "lots", ""] {
            let e = ChatConfig::from_env_values(None, Some(bad), None).unwrap_err();
            assert!(e.contains(KEEP_VAR), "{bad:?}: {e}");
        }
        assert_eq!(ChatConfig::from_env_values(None, None, Some("140")).unwrap().limits.text, 140);
        for bad in ["0", "x", "all"] {
            let e = ChatConfig::from_env_values(None, None, Some(bad)).unwrap_err();
            assert!(e.contains(TEXT_VAR), "{bad:?}: {e}");
        }
    }
```

And in `lib.rs`'s tests, beside the `health` cell (`grep -n 'assert_eq!(v\["signal"\], SIGNAL_KIND)'`):

```rust
    #[tokio::test]
    async fn health_carries_the_lobby_switch_and_the_two_caps() {
        let v = get_json(&Arc::new(AppState::in_memory()), "/api/health").await;
        assert_eq!(v["lobby_chat"], true);
        assert_eq!(v["chat_keep"], 500);
        assert_eq!(v["chat_text"], 500);
        let off = AppState::in_memory_with_chat(ChatConfig {
            lobby: false,
            limits: ChatLimits { keep: None, text: 140 },
        });
        let v = get_json(&Arc::new(off), "/api/health").await;
        assert_eq!(v["lobby_chat"], false);
        assert_eq!(v["chat_keep"], serde_json::Value::Null, "`all` is null on the wire");
        assert_eq!(v["chat_text"], 140);
    }
```

(Use whatever the file's existing health cell uses to GET a JSON body — read that cell and reuse its helper name; `get_json` is a stand-in for it.)

- [ ] **Step 2: `ChatConfig`** in `chat.rs`:

```rust
pub const LOBBY_VAR: &str = "YOKOHAMA_WEB_LOBBY_CHAT";
pub const KEEP_VAR: &str = "YOKOHAMA_WEB_CHAT_KEEP";
pub const TEXT_VAR: &str = "YOKOHAMA_WEB_CHAT_TEXT";

/// The three launch variables (R-6.4.6, R-6.4.8). **Each HAS a default**, unlike
/// the identity mode: a missing switch is not a missing door.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatConfig {
    pub lobby: bool,
    pub limits: ChatLimits,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self { lobby: true, limits: ChatLimits::default() }
    }
}

impl ChatConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_env_values(
            std::env::var(LOBBY_VAR).ok().as_deref(),
            std::env::var(KEEP_VAR).ok().as_deref(),
            std::env::var(TEXT_VAR).ok().as_deref(),
        )
    }

    /// The read split from the parse, the identity extractor's shape, so the
    /// unset case is testable without touching the process environment.
    pub fn from_env_values(lobby: Option<&str>, keep: Option<&str>, text: Option<&str>) -> Result<Self, String> {
        let lobby = match lobby.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
            None | Some("on") => true,
            Some("off") => false,
            Some(other) => return Err(format!("{LOBBY_VAR} must be on or off — got {other:?}")),
        };
        let keep = match keep.map(str::trim) {
            None => Some(500),
            Some("all") => None,
            Some(s) => match s.parse::<usize>() {
                Ok(n) if n > 0 => Some(n),
                _ => return Err(format!("{KEEP_VAR} must be a positive number of messages or `all` — got {s:?}")),
            },
        };
        let text = match text.map(str::trim) {
            None => 500,
            Some(s) => match s.parse::<usize>() {
                Ok(n) if n > 0 => n,
                _ => return Err(format!("{TEXT_VAR} must be a positive number of characters — got {s:?}")),
            },
        };
        Ok(Self { lobby, limits: ChatLimits { keep, text } })
    }
}
```

- [ ] **Step 3: `AppState` and `Health`.** In `lib.rs`: `pub chat: chat::ChatConfig` on `AppState`, doc'd "The lobby switch and the two retention caps (M6.4, R-6.4.6, R-6.4.8), read once at start; each has a default." `AppState::new(identity: IdentityMode, chat: ChatConfig)` → `Self::with_store_and_chat(Arc::new(FsStore::new(store::default_dir())), identity, chat)`; `with_store(store, identity)` → `Self::with_store_and_chat(store, identity, ChatConfig::default())`; `with_store_and_chat` builds `Games::with_limits(rules, store, chat.limits)` and sets `chat`; `in_memory_with_chat(chat)` → `with_store_and_chat(Arc::new(MemStore::default()), IdentityMode::Claimed, chat)`. `Health` gains

```rust
    /// Whether the house chat routes answer (M6.4, R-6.4.6). Off, the client
    /// draws no lobby panel and asks nothing.
    pub lobby_chat: bool,
    /// Messages retained per chat document; `null` for `all` (R-6.4.8).
    pub chat_keep: Option<u64>,
    /// Characters per message, so the client sizes its input from the
    /// server's rule and never from a copy (R-6.4.8).
    pub chat_text: u64,
```

filled from `state.chat` (`keep.map(|n| n as u64)`, `text as u64`). `main.rs`: after the identity match, the same shape:

```rust
    let chat = match yokohama_web::chat::ChatConfig::from_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    let state = Arc::new(AppState::new(identity, chat));
```

- [ ] **Step 4: Run and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`

```bash
git add crates/yokohama-web/src/chat.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/src/main.rs
git commit -m "feat(web): the lobby switch and the two retention caps as launch variables with defaults, refused by name on a bad value, carried on /api/health so the client sizes its input from the server's rule (spec §5, R-6.4.6, R-6.4.8)"
```

---

### Task 4: The routes — `GET`/`POST …/{id}/chat`, `GET`/`POST /api/chat`, `Events.chat`, the house frame, the grid, the sweep's exemption (spec §3, §4, §7, R-6.4.1, R-6.4.2, R-6.4.11)

**Files:**
- Modify: `crates/yokohama-web/src/lib.rs` (`Events`, `events_of`, `events_at`, `stream`'s expect message, `house_stream`, the four handlers, `ChatPost`, `ChatQuery`, `ChatPage`, `author_of`, `lobby_chat_on`, the router, `session_routes`, `admission_routes`, `Kind`, the grid cell, the stranger sweep's table, the write sweep)
- Test: `crates/yokohama-web/src/lib.rs`

**Interfaces:**
- Consumes: `Games::{chat_append, house_append, chat_page, house_page, chat_last}`, `signal::House` (Task 2); `AppState.chat` (Task 3); `chat::{Author, Message, check_text}`, `tables::check_name`.
- Produces: `Events.chat: u64`; the house frame `{"seq": N, "chat": M}`; `ChatPage { messages: Vec<Message>, last: u64 }`; `ChatPost { text, name? }`; `ChatQuery { after? }`; the routes `/api/games/{id}/chat` (GET → 200 `ChatPage`; POST → 201 `Message`) and `/api/chat` (the same, 404 when off).

- [ ] **Step 1: The failing route cells** (in `lib.rs`'s `mod tests`, using the file's own `post_as`/`get_as`/`start_as`/`start_locked_as`/`request_as` helpers — read their signatures first):

```rust
    /// **Seats and watchers post, a seat is named by its seat, a watcher by its
    /// body** (R-6.4.1, R-6.4.2), and the reply is the message.
    #[tokio::test]
    async fn a_seat_and_a_watcher_post_and_each_is_labelled_its_own_way() {
        let state = Arc::new(AppState::in_memory());
        start_locked_as(&state, "t", ENDED_SEED, &["red@wire.test", "blue@wire.test"], "sesame").await;
        // Blue joined by email with no name; give seat 0 a name through keep's
        // own path is not possible after finalize, so name it at create: use a
        // second table whose creator sends `display_name`.
        let (status, view) = post_as(&state, "grey@wire.test", "/api/games/t/unlock", serde_json::json!({ "password": "sesame" })).await;
        assert_eq!(status, StatusCode::OK, "{view}");

        let (status, m) = post_as(&state, "red@wire.test", "/api/games/t/chat", serde_json::json!({ "text": " hi " })).await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["seq"], 1);
        assert_eq!(m["text"], "hi", "trimmed");
        assert_eq!(m["author"]["kind"], "seat");
        assert_eq!(m["author"]["seat"], 0);
        assert_eq!(m["author"]["user"], "red@wire.test");

        let (status, m) = post_as(&state, "grey@wire.test", "/api/games/t/chat", serde_json::json!({ "text": "hello from the rail", "name": "Grey" })).await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["kind"], "watcher");
        assert_eq!(m["author"]["name"], "Grey");
        assert!(m["author"].get("seat").is_none(), "a watcher has no seat: {m}");

        let (status, page) = get_as(&state, "blue@wire.test", "/api/games/t/chat").await;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert_eq!(page["messages"].as_array().map(Vec::len), Some(2));
        assert_eq!(page["last"], 2);
        let (_, delta) = get_as(&state, "blue@wire.test", "/api/games/t/chat?after=1").await;
        assert_eq!(delta["messages"][0]["seq"], 2);
        let (_, none) = get_as(&state, "blue@wire.test", "/api/games/t/chat?after=9").await;
        assert_eq!(none["messages"].as_array().map(Vec::len), Some(0));
        assert_eq!(none["last"], 2);
    }

    /// **A seat's name is the SEAT's stored name and a body name is refused**
    /// (R-6.4.2, mutation 4) — by VALUE, over a seat whose stored name differs
    /// from the one in the body.
    #[tokio::test]
    async fn a_seated_caller_is_named_by_the_seat_and_may_not_send_a_name() {
        let state = Arc::new(AppState::in_memory());
        let (status, t) = post_as(&state, "red@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }],
            "id": "named", "name": "named", "display_name": "Captain Red",
        })).await;
        assert_eq!(status, StatusCode::CREATED, "{t}");
        let (status, refused) = post_as(&state, "red@wire.test", "/api/games/named/chat", serde_json::json!({ "text": "hi", "name": "Somebody Else" })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert_eq!(refused["error"], "a seated caller is named by their seat");
        let (status, m) = post_as(&state, "red@wire.test", "/api/games/named/chat", serde_json::json!({ "text": "hi" })).await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["name"], "Captain Red", "the stored name, at a DEALING table (R-6.4.5)");
    }

    /// **The refusal order, by adjacent-pair probes** (spec §3.2): admission,
    /// then the text, then the seat/name rule, then the watcher's name.
    #[tokio::test]
    async fn the_post_refuses_in_the_spec_s_order() {
        let state = Arc::new(AppState::in_memory());
        start_locked_as(&state, "t", ENDED_SEED, &["red@wire.test", "blue@wire.test"], "sesame").await;
        // admission before text: a shut-out stranger with an empty text is a 403, not a 400
        let (status, got) = post_as(&state, "green@wire.test", "/api/games/t/chat", serde_json::json!({ "text": "" })).await;
        assert_eq!((status, got["error"].as_str()), (StatusCode::FORBIDDEN, Some("this table is locked — unlock it first")));
        // text before the seat/name rule: an empty text AND a name → the text sentence
        let (status, got) = post_as(&state, "red@wire.test", "/api/games/t/chat", serde_json::json!({ "text": "  ", "name": "X" })).await;
        assert_eq!((status, got["error"].as_str()), (StatusCode::BAD_REQUEST, Some("a message needs some words")));
        // text before the watcher's name: over the cap AND a bad name → the text sentence
        post_as(&state, "grey@wire.test", "/api/games/t/unlock", serde_json::json!({ "password": "sesame" })).await;
        let long = "x".repeat(501);
        let (status, got) = post_as(&state, "grey@wire.test", "/api/games/t/chat", serde_json::json!({ "text": long, "name": "y".repeat(65) })).await;
        assert_eq!((status, got["error"].as_str()), (StatusCode::BAD_REQUEST, Some("a message is at most 500 characters")));
        // and the watcher's name alone is refused by check_name's sentence
        let (status, got) = post_as(&state, "grey@wire.test", "/api/games/t/chat", serde_json::json!({ "text": "ok", "name": "y".repeat(65) })).await;
        assert_eq!((status, got["error"].as_str()), (StatusCode::BAD_REQUEST, Some("a name is 1 to 64 characters")));
        // a control character
        let (status, got) = post_as(&state, "red@wire.test", "/api/games/t/chat", serde_json::json!({ "text": "a\nb" })).await;
        assert_eq!((status, got["error"].as_str()), (StatusCode::BAD_REQUEST, Some("a message is one line, with no control characters")));
        // an unknown key is the fenced 400
        let (status, _) = post_as(&state, "red@wire.test", "/api/games/t/chat", serde_json::json!({ "text": "hi", "seat": 1 })).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    /// **The house pair: any signed-in caller, 404 when off** (R-6.4.6).
    #[tokio::test]
    async fn the_house_chat_answers_anyone_signed_in_and_is_404_when_off() {
        let on = Arc::new(AppState::in_memory());
        let (status, m) = post_as(&on, "grey@wire.test", "/api/chat", serde_json::json!({ "text": "evening", "name": "Grey" })).await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!(m["author"]["kind"], "watcher");
        let (status, page) = get_as(&on, "green@wire.test", "/api/chat?after=0").await;
        assert_eq!((status, page["last"].as_u64()), (StatusCode::OK, Some(1)));
        let (status, _) = request_as(&on, "", "GET", "/api/chat", None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "no identity, no house");

        let off = Arc::new(AppState::in_memory_with_chat(ChatConfig { lobby: false, ..ChatConfig::default() }));
        for (method, body) in [("GET", None), ("POST", Some(serde_json::json!({ "text": "x" })))] {
            let (status, got) = request_as(&off, "grey@wire.test", method, "/api/chat", body).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{method}: {got}");
            assert_eq!(got["error"], "the lobby chat is off");
        }
        let mut house = off.games.signals().subscribe_house();
        assert!(!house.has_changed().unwrap(), "nothing moved the house counter");
    }

    /// **`Events.chat` on all three arms, and the stream after a chat bump
    /// carries the new count and an unmoved `events`** (spec §4, mutation 5).
    #[tokio::test]
    async fn events_carries_the_chat_count_at_open_dealing_and_playing() {
        let state = Arc::new(AppState::in_memory());
        let (status, _) = post_as(&state, "red@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
            "seats": [{ "user": "red@wire.test" }, { "open": true }], "id": "opent", "name": "opent",
        })).await;
        assert_eq!(status, StatusCode::CREATED);
        post_as(&state, "red@wire.test", "/api/games/opent/chat", serde_json::json!({ "text": "anyone?" })).await;
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/opent/events").await;
        assert_eq!((e["status"].as_str(), e["chat"].as_u64()), (Some("open"), Some(1)));
        let (_, e) = get_as(&state, "green@wire.test", "/api/games/opent/events").await;
        assert_eq!(e["chat"], 1, "a stranger at open reads the count (M6.3's R-4.3 arm)");
        let (status, _) = get_as(&state, "green@wire.test", "/api/games/opent/chat").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "and not the messages");

        let (status, _) = post_as(&state, "red@wire.test", "/api/games", serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0, "seed": "20260831",
            "seats": [{ "user": "red@wire.test" }, { "user": "blue@wire.test" }], "id": "dealt", "name": "dealt",
        })).await;
        assert_eq!(status, StatusCode::CREATED);
        post_as(&state, "blue@wire.test", "/api/games/dealt/chat", serde_json::json!({ "text": "choosing…" })).await;
        let (_, e) = get_as(&state, "red@wire.test", "/api/games/dealt/events").await;
        assert_eq!((e["status"].as_str(), e["chat"].as_u64()), (Some("dealing"), Some(1)));

        start_as(&state, "playt", ENDED_SEED, &["red@wire.test", "blue@wire.test"]).await;
        let (_, before) = get_as(&state, "red@wire.test", "/api/games/playt/events").await;
        assert_eq!(before["chat"], 0);
        let mut frames = stream_frames(&state, "/api/games/playt/stream", "blue@wire.test", 2).await;
        post_as(&state, "red@wire.test", "/api/games/playt/chat", serde_json::json!({ "text": "gl hf" })).await;
        let second = frames.remove(1);
        assert_eq!(second["chat"], 1, "{second}");
        assert_eq!(second["events"], before["events"], "a message moves no event");
    }

    /// **The house frame is two counters** (R-plan-2).
    #[tokio::test]
    async fn the_house_frame_carries_seq_and_chat_independently() {
        let state = Arc::new(AppState::in_memory());
        // read `stream_frames`' signature: it takes the state, the uri, the caller and how many frames to collect;
        // the house stream is silent on connect, so the first frame is the first bump
        let frames_after_chat = house_frames_after(&state, || async {
            post_as(&state, "grey@wire.test", "/api/chat", serde_json::json!({ "text": "hi" })).await;
        }).await;
        assert_eq!(frames_after_chat[0]["chat"], 1);
        assert_eq!(frames_after_chat[0]["seq"], 0);
    }
```

For the last cell, write a small `house_frames_after(state, act)` helper beside `stream_frames` if none fits: open `GET /api/stream` as a signed-in caller, run `act`, read ONE frame from the body and parse its `data:`. `stream_frames` (`lib.rs:5725`) is the model.

Then the grid: in `admission_routes()` add `("GET", "/chat", None, Kind::Chat)` and `("POST", "/chat", Some(serde_json::json!({ "text": "grid" })), Kind::ChatPost)`, with the two new `Kind` arms doc'd: `Chat` — "200 to a seat holder at EVERY status and to a watcher; 403 otherwise, with the lock sentence where the lock is what shuts them out"; `ChatPost` — "the same admission and a 201: the one write a watcher may make (R-6.4.11)". In the grid's `want` match, BEFORE the `_ if !seated && !watches` arm:

```rust
                        Kind::Chat if seated || watches => StatusCode::OK,
                        Kind::ChatPost if seated || watches => StatusCode::CREATED,
                        Kind::Chat | Kind::ChatPost => StatusCode::FORBIDDEN,
```

and after the `Kind::Signal` stream compare, a POST ≡ GET compare:

```rust
                    if kind == Kind::ChatPost {
                        let (read, _) = request_as(&state, who, "GET", &uri, None).await;
                        let expect = if got == StatusCode::CREATED { StatusCode::OK } else { got };
                        assert_eq!(read, expect, "{who} at {table}: the chat GET and POST are admitted alike");
                    }
```

`session_routes()` gains the same two rows (no kind) so the stranger sweep covers them; its expected-sentence table gains chat: `NOT_SEATED` at open and dealing, the lock sentence at the locked playing table (the reads' row). In `a_spectator_is_refused_every_write_before_the_record_is_ever_read`, after the `Kind::Write` loop:

```rust
            // **The ONE write a watcher may make, and it reads no record either**
            // (R-6.4.11): behind the same panicking store, grey's post lands.
            let (status, m) = post_as(&state, "grey@wire.test", &format!("/api/games/{table}/chat"), serde_json::json!({ "text": "from the rail" })).await;
            assert_eq!(status, StatusCode::CREATED, "{table}: {m}");
            assert_eq!(m["author"]["kind"], "watcher");
```

- [ ] **Step 2: The handlers.** In `lib.rs`, after `unlock`:

```rust
/// A message to post (M6.4, spec §3.2). `name` is a WATCHER's (and anyone's
/// at the house); a seated caller sending one is a 400, because a seat's name
/// is the seat's (R-6.4.2).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatPost {
    pub text: String,
    #[serde(default)]
    pub name: Option<String>,
}

/// The cursor: messages numbered ABOVE it (spec §3.1). Typed, so a negative
/// cursor is serde's 400.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatQuery {
    #[serde(default)]
    pub after: Option<u64>,
}

/// What the read answers: the retained messages above the cursor, and the
/// newest number ever assigned — the client's next cursor.
#[derive(Debug, Clone, Serialize)]
pub struct ChatPage {
    pub messages: Vec<chat::Message>,
    pub last: u64,
}

/// `GET /api/games/{id}/chat?after=N` — admitted exactly as a read is.
async fn chat_read(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    QueryArgs(q): QueryArgs<ChatQuery>,
) -> ApiResult<ChatPage> {
    admitted(&state, &id, &user)?;
    let (messages, last) = state.games.chat_page(&id, q.after.unwrap_or(0))?;
    Ok(Json(ChatPage { messages, last }))
}

/// `POST /api/games/{id}/chat` — the one write a watcher may make (R-6.4.11).
///
/// The order is the spec's §3.2 and is pinned by adjacent-pair probes:
/// admission, the text, the seat/name rule, the watcher's name, the append.
async fn chat_post(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<ChatPost>,
) -> Result<(StatusCode, Json<chat::Message>), ApiError> {
    let (table, viewer) = admitted(&state, &id, &user)?;
    let text = chat::check_text(&req.text, &state.chat.limits).map_err(ApiError::bad_request)?;
    let author = author_of(&table, viewer, &user, req.name.as_deref())?;
    let message = state.games.chat_append(&id, author, &text)?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// Who a poster IS, from the viewer the admission produced (R-6.4.2, R-6.4.15).
/// A seat's name is read off the TABLE and never off the body.
fn author_of(
    table: &crate::table::TableDoc,
    viewer: client::Viewer,
    user: &identity::User,
    name: Option<&str>,
) -> Result<chat::Author, ApiError> {
    match viewer {
        client::Viewer::Seat(seat) => {
            if name.is_some() {
                return Err(ApiError::bad_request("a seated caller is named by their seat"));
            }
            let stored = match table.seats.get(usize::from(seat.0)) {
                Some(crate::table::SeatDoc::Human { name, .. }) => name.clone(),
                _ => None,
            };
            Ok(chat::Author::Seat { seat: seat.0, user: user.as_str().to_string(), name: stored })
        }
        client::Viewer::Spectator => {
            let name = name.map(tables::check_name).transpose().map_err(ApiError::bad_request)?;
            Ok(chat::Author::Watcher { user: user.as_str().to_string(), name })
        }
    }
}

/// The house switch (R-6.4.6): a 404 with a sentence, before anything else.
fn lobby_chat_on(state: &AppState) -> Result<(), ApiError> {
    if state.chat.lobby {
        Ok(())
    } else {
        Err(ApiError::not_found("the lobby chat is off"))
    }
}

/// `GET /api/chat?after=N` — the house. Any signed-in caller.
async fn house_chat_read(
    identity::Identity(_user): identity::Identity,
    State(state): State<Arc<AppState>>,
    QueryArgs(q): QueryArgs<ChatQuery>,
) -> ApiResult<ChatPage> {
    lobby_chat_on(&state)?;
    let (messages, last) = state.games.house_page(q.after.unwrap_or(0))?;
    Ok(Json(ChatPage { messages, last }))
}

/// `POST /api/chat` — the house. Every author is a `Watcher`: the house has
/// no seats.
async fn house_chat_post(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    JsonBody(req): JsonBody<ChatPost>,
) -> Result<(StatusCode, Json<chat::Message>), ApiError> {
    lobby_chat_on(&state)?;
    let text = chat::check_text(&req.text, &state.chat.limits).map_err(ApiError::bad_request)?;
    let name = req.name.as_deref().map(tables::check_name).transpose().map_err(ApiError::bad_request)?;
    let author = chat::Author::Watcher { user: user.as_str().to_string(), name };
    let message = state.games.house_append(author, &text)?;
    Ok((StatusCode::CREATED, Json(message)))
}
```

In `admitted`'s doc block (`lib.rs:455-457`) the sentence "so the caller's lowest seat stands in for a value those routes never read" becomes "so the caller's lowest seat stands in — and since M6.4 the chat post READS it: before the deal a multi-seat caller posts as their lowest seat, after it as the acting seat (spec §3.2)". Router: `.route("/api/games/{id}/chat", get(chat_read).post(chat_post))` after `unlock`'s line, and `.route("/api/chat", get(house_chat_read).post(house_chat_post))` after `/api/stream`. The comment at `lib.rs:422` ("on all eighteen `/api/games` handlers") becomes twenty; re-read the sentence and correct its number.

- [ ] **Step 3: `Events.chat` and the house frame.** `Events` gains `pub chat: u64` (doc: "The table's newest message number, 0 with none (M6.4, R-6.4.7) — the fifth field, and the one that makes a message a count on the signal and a fetch for the text"). `events_of(repl, viewer, chat: u64)` takes it as a parameter; `events_at` computes `let chat = state.games.chat_last(id);` once at its top and passes it to all three arms (`Events { events: 0, to_move: None, status: "open", your_turn: false, chat }`, the dealing literal likewise, `events_of(&repl, viewer, chat)`). The stream's `expect("`Events` is four scalars and serializes")` becomes five. `house_stream`'s map becomes

```rust
    let frames = tokio_stream::wrappers::WatchStream::from_changes(rx).map(|h| {
        Ok::<_, std::convert::Infallible>(
            Event::default().event("house").data(format!("{{\"seq\":{},\"chat\":{}}}", h.seq, h.chat)),
        )
    });
```

and its doc gains: "two counters and never a per-table payload (R-3.4, amended by M6.4 R-6.4.7); a `chat` bump moves the second and the lobby, in `wake` mode, refetches on either." The `signal` handler's doc cites `chat` as the fifth field. Any cell that constructs an `Events { … }` literal gains `chat: 0`.

- [ ] **Step 4: Run and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`

```bash
git add crates/yokohama-web/src/lib.rs
git commit -m "feat(web): the four chat routes — a table's read and post admitted exactly as a read is, the one write a watcher may make, a seat named by its seat and never its body, the house pair behind its switch; Events gains its fifth field and the house frame its second counter; the grid, the stranger sweep and the write sweep extended (spec §3, §4, R-6.4.1, R-6.4.2, R-6.4.6, R-6.4.11)"
```

---

### Task 5: The wire — eleven fixtures, the narrowed pin, the client's types and calls (spec §8, §7, R-6.4.12, R-plan-6)

**Files:**
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/*.json` (regenerated), `web/src/types.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`, `web/src/api.ts`
- Test: `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/contract.test.ts`

**Interfaces:**
- Consumes: the four routes, `Events.chat`, `Health`'s three keys (Tasks 3–4); `AppState::in_memory_with_chat` (Task 3).
- Produces: fixtures `chat`, `chat_after`, `chat_post`, `chat_post_watcher`, `chat_400_long`, `chat_400_seated_name`, `chat_403_not_seated`, `chat_403_locked`, `house_chat`, `house_chat_post`, `house_chat_off_404` (`web/src/wire/<name>.json`); `types.ts`: `ChatAuthor`, `ChatMessage`, `ChatPage`, `ChatPost`, `ChatScope`, `Events.chat`, `Health.{lobby_chat, chat_keep, chat_text}`; `contract.ts`: `wire.chat`, `wire.chatAfter`, `wire.chatPost`, `wire.chatPostWatcher`, `wire.chat400Long`, `wire.chat400SeatedName`, `wire.chat403NotSeated`, `wire.chat403Locked`, `wire.houseChat`, `wire.houseChatPost`, `wire.houseChatOff404`, `KEYS.chatPage`, `KEYS.chatMessage`, `KEYS.chatAuthorSeat`, `KEYS.chatAuthorWatcher`; `api.ts`: `api.chat(scope, after)`, `api.postChat(scope, body)`, `lobbyChat()`, `setLobbyChat(b)`, `chatText()`, `setChatText(n)`, `clearDisplayName()`.

- [ ] **Step 1: The walk's chat block (R-plan-6).** In `wire_fixtures.rs`, immediately after `let mut state = as_actor(&api, created).await;` (`:760`) and before `let mut rng = Rng(WALK_SEED);`:

```rust
    // ---- M6.4: the table's chat, before the first move so `events.json`
    // reads the count (R-plan-6). Three authors, three arms exercised: two
    // seats and the watcher who unlocked above. ----------------------------
    let (status, chat_post) = api
        .call_as(Some(WALKER), "POST", "/api/games/wire/chat", Some(json!({ "text": "good luck" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{chat_post}");
    let (status, blue_post) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/chat", Some(json!({ "text": "you too" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{blue_post}");
    assert_eq!(blue_post["author"]["name"], "Blue", "the seat's stored name, from the join");
    let (status, chat_post_watcher) = api
        .call_as(Some(GREEN), "POST", "/api/games/wire/chat", Some(json!({ "text": "watching from the rail", "name": "Green" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{chat_post_watcher}");
    assert_eq!(chat_post_watcher["author"]["kind"], "watcher");
    let chat = api.get("/api/games/wire/chat").await;
    assert_eq!(chat["last"], 3, "{chat}");
    let chat_after = api.get("/api/games/wire/chat?after=2").await;
    assert_eq!(chat_after["messages"].as_array().map(Vec::len), Some(1), "{chat_after}");
    let (status, chat_400_long) = api
        .call_as(Some(WALKER), "POST", "/api/games/wire/chat", Some(json!({ "text": "x".repeat(501) })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{chat_400_long}");
    let (status, chat_400_seated_name) = api
        .call_as(Some(WALKER), "POST", "/api/games/wire/chat", Some(json!({ "text": "hi", "name": "Not Red" })))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{chat_400_seated_name}");
    let (status, chat_403_locked) = api.call_as(Some(STRANGER), "GET", "/api/games/wire/chat", None).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{chat_403_locked}");
    // The house: red posts, blue reads.
    let (status, house_chat_post) = api
        .call_as(Some(WALKER), "POST", "/api/chat", Some(json!({ "text": "anyone up for a second table?", "name": "Red" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{house_chat_post}");
    let house_chat = api.get_as(OTHER, "/api/chat").await;
    assert_eq!(house_chat["last"], 1, "{house_chat}");
    for (name, v) in [
        ("chat", chat), ("chat_after", chat_after), ("chat_post", chat_post),
        ("chat_post_watcher", chat_post_watcher), ("chat_400_long", chat_400_long),
        ("chat_400_seated_name", chat_400_seated_name), ("chat_403_locked", chat_403_locked),
        ("house_chat", house_chat), ("house_chat_post", house_chat_post),
    ] {
        out.push((name.to_string(), v));
    }
```

**The fourth identity (R-6.4.12)**: `const QUIET: &str = "quiet@wire.test";` beside `STRANGER` (`:348`), and immediately after GREEN's `unlock` in the walk (`grep -n '/api/games/wire/unlock' tests/wire_fixtures.rs`): `let (status, _) = api.call_as(Some(QUIET), "POST", "/api/games/wire/unlock", Some(json!({ "password": WIRE_PASSWORD }))).await; assert_eq!(status, StatusCode::OK, "the silent watcher knocks");` — and QUIET never calls anything else. `chat_403_not_seated` is captured in the OPENING block, right after `table_open` (a stranger reading the open table's chat): `let (status, chat_403_not_seated) = api.call_as(Some(GREEN), "GET", "/api/games/wire/chat", None).await; assert_eq!(status, StatusCode::FORBIDDEN);` pushed into the opening `out` vec beside `not_seated_403`. `house_chat_off_404` comes from a second state in `constructed()`: add `Api::with_chat(cfg)` (`AppState::in_memory_with_chat`) and push `("house_chat_off_404", the GET's body)` there. **The clock is already frozen**: `freeze_clock` (`:256`) pins every `created_at` in the tree by key name, which is why the message field carries that name; assert after regenerating that no `created_at` in `chat.json` differs from `FROZEN_CREATED_AT`.

- [ ] **Step 2: The narrowed pin (R-6.4.12).** Replace `no_payload_built_for_someone_else_carries_a_watchers_email` with:

```rust
/// **A silent watcher is named NOWHERE, and a posting one only as the author
/// of what they posted** (M6.4, R-6.4.12; narrowing M6.3's pin). The corpus is
/// walked as a JSON tree: grey never posts and may not appear; green posts once
/// and may appear only as `author.user` under `kind: "watcher"`.
#[tokio::test]
async fn a_silent_watcher_is_named_nowhere_and_a_posting_one_only_as_an_author() {
    fn walk(v: &Value, path: &mut Vec<String>, needle: &str, hits: &mut Vec<(String, bool)>) {
        match v {
            Value::String(s) if s.contains(needle) => {
                // Allowed exactly at `<…>.author.user` beside `kind: "watcher"`.
                let allowed = path.last().map(String::as_str) == Some("user")
                    && path.len() >= 2
                    && path[path.len() - 2] == "author";
                hits.push((path.join("."), allowed));
            }
            Value::Array(items) => {
                for (i, item) in items.iter().enumerate() {
                    path.push(i.to_string());
                    walk(item, path, needle, hits);
                    path.pop();
                }
            }
            Value::Object(map) => {
                for (k, item) in map {
                    path.push(k.clone());
                    walk(item, path, needle, hits);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let mut offenders = Vec::new();
    let mut allowed_hits = 0usize;
    for (name, value) in corpus().await {
        for (who, needle, may_post) in [
            ("the watcher", GREEN, true),
            ("a watcher who never spoke", QUIET, false),
            ("a caller who never knocked", STRANGER, false),
        ] {
            let mut hits = Vec::new();
            walk(&value, &mut Vec::new(), needle, &mut hits);
            for (path, allowed) in hits {
                if allowed && may_post {
                    // The sibling `kind` must say watcher: a seat author carrying
                    // green's address would be a seat green does not hold.
                    let author = path.rsplitn(2, '.').nth(1).map(|p| p.split('.').fold(&value, |v, k| &v[k.parse::<usize>().map(|i| Value::from(i)).unwrap_or(Value::from(k))]));
                    let _ = author; // the kind check below reads the object directly
                    allowed_hits += 1;
                } else {
                    offenders.push(format!("{name} names {who} at {path}"));
                }
            }
        }
    }
    assert!(offenders.is_empty(), "a watcher is named where they did not speak: {}", offenders.join(", "));
    // Anti-vacuity, both halves: the allowance is exercised (green posted, and
    // the corpus shows it), and the address IS written down where the search
    // would find it if a route leaked it (the table document, as before).
    assert!(allowed_hits >= 2, "green's post should appear in chat.json and chat_post_watcher.json; found {allowed_hits}");
    // The silent watcher IS written down where a leak would surface it: the
    // walk's own table document carries QUIET on `unlocked`.
    let (wire_table, _) = api_of_the_walk.state.games.store().read(yokohama_web::store::Doc::Table, "wire").expect("a store").expect("the walk's table");
    assert!(wire_table.contains(QUIET), "quiet never knocked — the silent-watcher search was looking for nobody");
    /* …then the existing anti-vacuity partner (the `watched` table's unlock) unchanged… */
}
```

Simplify the inner `author` lookup to a plain check: when `allowed`, index the value by the path's prefix to the author object and assert `author["kind"] == "watcher"` — write it as a small `fn at_path<'a>(v: &'a Value, path: &[String]) -> &'a Value` helper rather than the fold shown. `api_of_the_walk` stands for the `Api` the corpus was built over: `corpus()` builds its own, so either return it beside the payloads or open a second `Api`, replay create + unlock for QUIET, and read that store — the plan prefers the second (the pin then owns its premise). Keep the M6.3 partner (the `watched` table) verbatim.

- [ ] **Step 3: Regenerate and read the diff.** `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then `cargo test -p yokohama-web --test wire_fixtures`, then `git status --short web/src/wire/ | sort` and `git diff --stat web/src/wire/`. **Expected:** 11 new files; every `events*.json` changed (one key), `health.json` changed (three keys); NOTHING ELSE changed (`git diff --stat` shows only those). If any `ClientState`, `HistoricalFrame`, `TableView` or `DealView` file moved, stop and report — that is the byte-identical claim failing. Record the list in the ledger.

- [ ] **Step 4: The client's types, contract and calls.** `types.ts`:

```ts
/** Who said it (M6.4, R-6.4.2, R-6.4.15). Tagged like `SeatDoc`, so an arm can be
 *  added on both sides of the wire; `bot` is read and rendered and produced by
 *  no route yet. */
export type ChatAuthor =
  | { kind: "seat"; seat: PlayerId; user: string; name: string | null }
  | { kind: "watcher"; user: string; name: string | null }
  | { kind: "bot"; seat: PlayerId; name: string | null };

export interface ChatMessage {
  seq: number;
  /** Unix SECONDS; named as the table's clock is, because the corpus freezer
   *  pins a clock by key name. */
  created_at: number;
  author: ChatAuthor;
  text: string;
}

/** `GET …/chat?after=N`: the retained messages above N, and the newest number
 *  ever assigned — the next cursor, which survives a trim. */
export interface ChatPage {
  messages: ChatMessage[];
  last: number;
}

export interface ChatPost {
  text: string;
  /** A watcher's name, and anyone's at the house. A seated caller sends none:
   *  a seat's name is the seat's, and the server refuses one with a 400. */
  name?: string;
}

/** Where a chat panel is: one table, or the house. A client-side type. */
export type ChatScope = { kind: "table"; id: string } | { kind: "house" };
```

`Events` gains `chat: number` (doc: "The table's newest message number (M6.4, R-6.4.7): compared, never fetched from, like `events`."); `Health` gains `lobby_chat: boolean`, `chat_keep: number | null`, `chat_text: number`. `contract.ts`: import the eleven files; `wire` gains the eleven entries; `EVENTS` gains `"chat"`; `HEALTH` gains the three; new `CHAT_PAGE = ["messages", "last"]`, `CHAT_MESSAGE = ["seq", "created_at", "author", "text"]`, `CHAT_AUTHOR_SEAT = ["kind", "seat", "user", "name"]`, `CHAT_AUTHOR_WATCHER = ["kind", "user", "name"]`; `KeysOf` rows for `ChatPage` and `ChatMessage`; **and a §1c beside §1b (`contract.ts:339-344`)** — `ChatAuthor` is internally tagged, the one shape widening ruins, so each fixture author is assigned to ITS OWN member:

```ts
// §1c. The chat author is `#[serde(tag = "kind")]` like `PromptState`, so
// `Widen<ChatAuthor>` would accept `{kind: "nonsense"}`. Each fixture author
// against its own member, non-discriminant fields required again.
const _ca0: Widen<Extract<ChatAuthor, { kind: "seat" }>> = chat.messages[0]!.author;
const _ca1: Widen<Extract<ChatAuthor, { kind: "seat" }>> = chat.messages[1]!.author;
const _ca2: Widen<Extract<ChatAuthor, { kind: "watcher" }>> = chat.messages[2]!.author;
const _cp: Widen<Extract<ChatAuthor, { kind: "seat" }>> = chatPost.author;
const _cpw: Widen<Extract<ChatAuthor, { kind: "watcher" }>> = chatPostWatcher.author;
const _hc: Widen<Extract<ChatAuthor, { kind: "watcher" }>> = houseChat.messages[0]!.author;
void [_ca0, _ca1, _ca2, _cp, _cpw, _hc];
```

(`bot` has no fixture and is declared only.) `KEYS` exports the four lists. `contract.test.ts`:

```ts
  it("ChatPage, ChatMessage and each author by its kind", () => {
    const byKind = { seat: KEYS.chatAuthorSeat, watcher: KEYS.chatAuthorWatcher } as const;
    for (const page of [wire.chat, wire.chatAfter, wire.houseChat]) {
      expect(keys(page)).toEqual(listed(KEYS.chatPage));
      for (const m of page.messages) {
        expect(keys(m)).toEqual(listed(KEYS.chatMessage));
        const kind = m.author.kind as keyof typeof byKind;
        expect(byKind[kind], `an author kind the client knows: ${m.author.kind}`).toBeDefined();
        expect(keys(m.author)).toEqual(listed(byKind[kind]));
      }
    }
    for (const m of [wire.chatPost, wire.chatPostWatcher]) {
      expect(keys(m)).toEqual(listed(KEYS.chatMessage));
    }
  });
``` `api.ts`:

```ts
/** The house chat switch, from `/api/health` at boot (M6.4, R-6.4.6). */
let lobby = true;
export const lobbyChat = (): boolean => lobby;
export const setLobbyChat = (on: boolean): void => { lobby = on; };
/** Characters per message, from `/api/health` (R-6.4.8) — the input's `maxLength`. */
let textCap = 500;
export const chatText = (): number => textCap;
export const setChatText = (n: number): void => { textCap = n; };

export function clearDisplayName(): void {
  try { localStorage.removeItem(DISPLAY_NAME_KEY); } catch { /* nothing to clear */ }
}

const chatPath = (scope: ChatScope, after?: number): string =>
  (scope.kind === "house" ? "/api/chat" : `/api/games/${id(scope.id)}/chat`) + query({ after });
```

and on `api`: `chat: (scope: ChatScope, after = 0) => call<ChatPage>("GET", chatPath(scope, after))`, `postChat: (scope: ChatScope, body: ChatPost) => call<ChatMessage>("POST", chatPath(scope), body)`.

- [ ] **Step 5: Run and commit**

Run: `cargo test -p yokohama-web --test wire_fixtures 2>&1 | grep 'test result' && cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests '`

```bash
git add crates/yokohama-web/tests/wire_fixtures.rs web/src/wire/ web/src/types.ts web/src/wire/contract.ts web/src/wire/contract.test.ts web/src/api.ts
git commit -m "test(web): the chat on the wire — eleven fixtures from the walk (three authors, the delta, the two 400s, the two 403s, the house pair and its 404), events with its fifth key and health with its three, every frame fixture byte-identical; the watcher's-email pin narrowed to 'a silent watcher nowhere, a posting one only as an author'; the client's types, key rows and calls (spec §8, §7, R-6.4.12)"
```

---

### Task 6: The one panel — `Chat.tsx`, `chatFold.ts`, the styles (spec §6.1, R-6.4.10, R-plan-4)

**Files:**
- Create: `web/src/Chat.tsx`, `web/src/Chat.test.tsx`, `web/src/chatFold.ts`
- Modify: `web/src/styles.css`
- Test: `web/src/Chat.test.tsx`

**Interfaces:**
- Consumes: `api.chat`, `api.postChat`, `chatText`, `displayName` (`api.ts`, Task 5); `seatClass` (`Cards.tsx`); the fixtures `wire.chat`, `wire.chatAfter`, `wire.chatPost`, `wire.chatPostWatcher`, `wire.chat400Long`, `wire.houseChat`.
- Produces: `<Chat scope count? tick? colors? sendName heading? />` (`ChatProps` exported); `when(at: number, now?: number, locale?: string): string`; `loadFolded(scope: string): boolean`, `saveFolded(scope: string, folded: boolean): void`, `OPEN_KEY`.

- [ ] **Step 1: `chatFold.ts`** (the `sections.ts` idiom, default TRUE):

```ts
// Which chat panels this browser has OPENED (M6.4, R-6.4.10): ONE localStorage
// key holding a set of scopes — the `sections.ts` single-key idiom, never a key
// per table — newest 64 kept. Folded is the default everywhere (the panel is a
// bonus and play is the main attraction), so a storage that throws reads as
// folded and loses nothing.

export const OPEN_KEY = "yokohama.chat.open";
const KEEP = 64;

function browserStorage(): Storage | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

function openSet(storage: Storage | null): string[] {
  try {
    const raw = storage?.getItem(OPEN_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((s): s is string => typeof s === "string") : [];
  } catch {
    return [];
  }
}

/** `true` unless this browser has opened the panel for this scope. */
export function loadFolded(scope: string, storage: Storage | null = browserStorage()): boolean {
  return !openSet(storage).includes(scope);
}

export function saveFolded(scope: string, folded: boolean, storage: Storage | null = browserStorage()): void {
  try {
    const set = openSet(storage).filter((s) => s !== scope);
    if (!folded) set.push(scope);
    storage?.setItem(OPEN_KEY, JSON.stringify(set.slice(-KEEP)));
  } catch {
    // A per-browser convenience; losing it loses nothing.
  }
}
```

- [ ] **Step 2: The failing component cells**, `Chat.test.tsx`. The harness is `stubApi` and the corpus; each cell asserts one rule.

```tsx
// The chat panel (M6.4 spec §6.1): one component, told where it is and when to
// look, owning its list, its fetch and its post. Every payload is the corpus's.

import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Chat, when } from "./Chat";
import { DISPLAY_NAME_KEY, setChatText } from "./api";
import { OPEN_KEY } from "./chatFold";
import { stubApi, type ApiCall, type Reply } from "./test/harness";
import { wire } from "./wire/contract";

const TABLE = { kind: "table", id: "wire" } as const;
const colors = wire.table.colors;

/** A stub answering the table's chat from the corpus: everything at `after=0`,
 *  the delta at `after=2`, nothing above. */
const chatStub = (extra?: (c: ApiCall) => Reply | undefined) =>
  stubApi((call) => {
    const mine = extra?.(call);
    if (mine !== undefined) return mine;
    if (call.method === "GET" && call.path.includes("/api/games/wire/chat")) {
      const after = Number(new URL(call.path, "http://x").searchParams.get("after") ?? 0);
      if (after === 0) return wire.chat;
      if (after < wire.chat.last) return { ...wire.chat, messages: wire.chat.messages.filter((m) => m.seq > after) };
      return { messages: [], last: wire.chat.last };
    }
    throw new Error(`unexpected ${call.method} ${call.path}`);
  });

const open = () => localStorage.setItem(OPEN_KEY, JSON.stringify(["wire"]));

beforeEach(() => localStorage.clear());
afterEach(() => vi.unstubAllGlobals());

describe("the chat panel", () => {
  it("is folded by default, and remembers being opened", async () => {
    const stub = chatStub();
    const { unmount } = render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    const fold = await screen.findByRole("button", { name: "show" });
    expect(fold).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(fold);
    expect(screen.getByRole("button", { name: "hide" })).toHaveAttribute("aria-expanded", "true");
    expect(JSON.parse(localStorage.getItem(OPEN_KEY)!)).toContain("wire");
    unmount();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    expect(await screen.findByRole("button", { name: "hide" })).toBeInTheDocument();
    expect(stub.matching("GET", "/chat")).toHaveLength(2);
  });

  it("fetches everything on mount and draws the three authors their own way", async () => {
    open();
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    const rows = await screen.findAllByRole("listitem");
    expect(rows).toHaveLength(wire.chat.messages.length);
    // A seat: a chip in the seat's colour class and the seat's name or email.
    const red = wire.chat.messages[0]!;
    if (red.author.kind !== "seat") throw new Error("the first corpus message is red's");
    expect(rows[0]!.querySelector(`.chat-who.seat-${colors[red.author.seat]!.toLowerCase()}`)).not.toBeNull();
    expect(rows[0]!).toHaveTextContent(red.author.name ?? red.author.user);
    // The watcher: their given name, marked.
    const green = wire.chat.messages[2]!;
    if (green.author.kind !== "watcher") throw new Error("the third corpus message is the watcher's");
    expect(rows[2]!.querySelector(".chat-who.watching")).not.toBeNull();
    expect(rows[2]!).toHaveTextContent(green.author.name!);
    expect(rows[2]!).toHaveTextContent("watching");
    // The text, and a time on every row.
    expect(rows[1]!).toHaveTextContent(wire.chat.messages[1]!.text);
    for (const r of rows) expect(r.querySelector("time")).not.toBeNull();
  });

  it("fetches the delta when the count rises above what it holds, and NOTHING when it does not", async () => {
    open();
    const stub = chatStub();
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await screen.findAllByRole("listitem");
    const before = stub.matching("GET", "/chat").length;
    // Ten frames whose count did not move past what the panel holds (mutation 10).
    for (let i = 0; i < 10; i += 1) {
      rerender(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
      rerender(<Chat scope={TABLE} count={wire.chat.last - 1} colors={colors} sendName={false} />);
    }
    await act(async () => {});
    expect(stub.matching("GET", "/chat")).toHaveLength(before);
    // And a count above it fetches ONCE, with the cursor.
    rerender(<Chat scope={TABLE} count={wire.chat.last + 1} colors={colors} sendName={false} />);
    await waitFor(() => expect(stub.matching("GET", "/chat")).toHaveLength(before + 1));
    expect(stub.matching("GET", "/chat").at(-1)!.path).toContain(`after=${wire.chat.last}`);
  });

  it("fetches on a tick, folded or not, and never draws a held message twice", async () => {
    const stub = chatStub();
    const { rerender } = render(<Chat scope={TABLE} tick={0} colors={colors} sendName={false} />);
    await act(async () => {});
    const before = stub.matching("GET", "/chat").length;
    rerender(<Chat scope={TABLE} tick={1} colors={colors} sendName={false} />);
    await waitFor(() => expect(stub.matching("GET", "/chat")).toHaveLength(before + 1));
    // The delta comes back carrying one message the panel already holds (a
    // stub that answers `after=0` to everything): one row per seq, still.
    await userEvent.click(await screen.findByRole("button", { name: "show" }));
    expect(screen.getAllByRole("listitem")).toHaveLength(wire.chat.messages.length);
  });

  it("counts new messages on the folded header and clears them on unfold", async () => {
    chatStub();
    const { rerender } = render(<Chat scope={TABLE} count={0} colors={colors} sendName={false} />);
    await act(async () => {});
    rerender(<Chat scope={TABLE} count={wire.chat.last + 2} colors={colors} sendName={false} />);
    expect(await screen.findByLabelText("2 new")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "show" }));
    expect(screen.queryByLabelText(/new$/)).toBeNull();
  });

  it("posts on Enter, appends the reply, sends no name for a seat, and advances its cursor", async () => {
    open();
    localStorage.setItem(DISPLAY_NAME_KEY, "Red");
    const stub = chatStub((call) => (call.method === "POST" && call.path.endsWith("/chat") ? wire.chatPost : undefined));
    const { rerender } = render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await screen.findAllByRole("listitem");
    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "good luck{Enter}");
    const posts = await waitFor(() => {
      const p = stub.matching("POST", "/chat");
      expect(p).toHaveLength(1);
      return p;
    });
    expect(posts[0]!.body).toEqual({ text: "good luck" });
    expect(screen.getByRole("textbox", { name: "Say something" })).toHaveValue("");
    // The 201 was appended locally; the frame it causes is then a no-op.
    const before = stub.matching("GET", "/chat").length;
    rerender(<Chat scope={TABLE} count={wire.chatPost.seq} colors={colors} sendName={false} />);
    await act(async () => {});
    expect(stub.matching("GET", "/chat")).toHaveLength(before);
  });

  it("sends the browser's name for a watcher and at the house", async () => {
    open();
    localStorage.setItem(OPEN_KEY, JSON.stringify(["wire", "house"]));
    localStorage.setItem(DISPLAY_NAME_KEY, "Green");
    const stub = stubApi((call) => {
      if (call.method === "GET" && call.path.includes("/chat")) return wire.houseChat;
      if (call.method === "POST") return wire.chatPostWatcher;
      throw new Error(`unexpected ${call.method} ${call.path}`);
    });
    render(<Chat scope={{ kind: "house" }} tick={0} sendName />);
    const user = userEvent.setup();
    await user.type(await screen.findByRole("textbox", { name: "Say something" }), "hi all{Enter}");
    await waitFor(() => expect(stub.matching("POST", "/api/chat")).toHaveLength(1));
    expect(stub.matching("POST", "/api/chat")[0]!.body).toEqual({ text: "hi all", name: "Green" });
  });

  it("renders the server's refusal in its own line and keeps the draft", async () => {
    open();
    chatStub((call) => (call.method === "POST" ? { status: 400, body: wire.chat400Long } : undefined));
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await screen.findAllByRole("listitem");
    const user = userEvent.setup();
    await user.type(screen.getByRole("textbox", { name: "Say something" }), "too long{Enter}");
    expect(await screen.findByText(wire.chat400Long.error)).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Say something" })).toHaveValue("too long");
  });

  it("sizes its input from the server's cap", async () => {
    open();
    setChatText(140);
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    expect(await screen.findByRole("textbox", { name: "Say something" })).toHaveAttribute("maxlength", "140");
    setChatText(500);
  });

  it("dates a message that is not from today", () => {
    const now = Date.UTC(2026, 8, 16, 12, 0, 0);
    expect(when(Math.floor(now / 1000), now, "en-US")).not.toMatch(/Sep|16/);
    expect(when(Math.floor((now - 2 * 86_400_000) / 1000), now, "en-US")).toMatch(/Sep/);
  });

  it("opens no connection of its own (mutation 22)", async () => {
    open();
    const stub = chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await screen.findAllByRole("listitem");
    const streams = stub.calls.filter(
      (c) => c.path.endsWith("/stream") || c.headers["accept"] === "text/event-stream",
    );
    expect(streams).toEqual([]);
  });

  it("keeps a folded panel's controls out of the tab order", async () => {
    chatStub();
    render(<Chat scope={TABLE} count={wire.chat.last} colors={colors} sendName={false} />);
    await act(async () => {});
    expect(screen.getByRole("textbox", { name: "Say something", hidden: true })).toHaveAttribute("tabindex", "-1");
  });
});
```

(The time cell pins `now` and the locale, so it does not depend on the runner's clock or language.)

- [ ] **Step 3: `Chat.tsx`**

```tsx
// The chat panel (M6.4 spec §6.1): ONE component, four hosts. It owns its
// list, its fetch and its post; a host tells it only WHERE it is (a table or
// the house) and WHEN to look — a `count` where the host's frame carries the
// server's number, a `tick` where the host's hook drops the frame.
//
// **Folded by default everywhere (R-6.4.10).** The panel is a bonus and play is
// the main attraction, so it sits LAST in the side column and opens only when
// somebody opens it; the fold is remembered per browser per scope. Folded, a
// count-driven host fetches nothing and the header's number is the count
// minus what is held; a wake-driven host still asks the delta on each tick,
// because the delta is the only source it has (spec §6.1 says so).
//
// **The label is the lobby's rule** (R-6.4.2): a seat's colour chip and its name
// or email; a watcher's name or email under a "watching" marker at a table
// (not in the house, where everyone is one); a bot's chip and its name — an
// arm no route produces yet, drawn defensively (R-6.4.15).

import { useCallback, useEffect, useRef, useState } from "react";
import { api, ApiError, chatText, displayName } from "./api";
import { seatClass } from "./Cards";
import { loadFolded, saveFolded } from "./chatFold";
import type { ChatMessage, ChatPage, ChatScope, PlayerColor } from "./types";

export interface ChatProps {
  scope: ChatScope;
  /** The server's newest message number, where the host holds it (the game
   *  and the dealing screen read it off every frame). */
  count?: number;
  /** "Something may have changed", where the host's hook runs in wake mode
   *  and drops the frame (the open table, the lobby). */
  tick?: number;
  /** The table's colours; absent at the house. */
  colors?: PlayerColor[];
  /** Send the browser's remembered name with a post: a watcher, and anyone at
   *  the house. A seated caller sends none (R-6.4.2). */
  sendName: boolean;
  /** The heading level: `h2` in the side column (the panels' own), `h3` in the
   *  lobby, whose three sibling sections are `h3` under the screen's one `h2`. */
  heading?: "h2" | "h3";
}

/** `HH:MM` today; the date before it otherwise (R-P3's night away). `now` and
 *  `locale` are parameters so a cell can freeze both. */
export function when(at: number, now: number = Date.now(), locale?: string): string {
  const d = new Date(at * 1000);
  const n = new Date(now);
  const hm = d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  return d.toDateString() === n.toDateString()
    ? hm
    : `${d.toLocaleDateString(locale, { month: "short", day: "numeric" })} ${hm}`;
}

const scopeKey = (scope: ChatScope): string => (scope.kind === "house" ? "house" : scope.id);

export function Chat({ scope, count, tick, colors, sendName, heading = "h2" }: ChatProps) {
  const key = scopeKey(scope);
  const Heading = heading;
  const scopeRef = useRef(scope);
  scopeRef.current = scope;
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  /** The highest number held — a ref, read by effects that must not re-run
   *  when it moves. */
  const last = useRef(0);
  const [folded, setFolded] = useState(() => loadFolded(key));
  const foldedNow = useRef(folded);
  foldedNow.current = folded;
  const [unread, setUnread] = useState(0);
  const [draft, setDraft] = useState("");
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
  const box = useRef<HTMLOListElement | null>(null);

  /** One answer, from wherever it came: a mount read, a delta, a 201. Filters
   *  what is already held, so a message is never drawn twice; keeps the box
   *  at its bottom unless the reader has scrolled up. */
  const take = useCallback((page: ChatPage) => {
    const fresh = page.messages.filter((m) => m.seq > last.current);
    last.current = Math.max(last.current, page.last, ...fresh.map((m) => m.seq));
    if (fresh.length === 0) return;
    const el = box.current;
    const atBottom = !el || el.scrollHeight - el.scrollTop - el.clientHeight < 8;
    setMessages((old) => [...old, ...fresh]);
    if (foldedNow.current) setUnread((n) => n + fresh.length);
    if (atBottom)
      queueMicrotask(() => {
        const b = box.current;
        if (b) b.scrollTop = b.scrollHeight;
      });
  }, []);

  const fetchDelta = useCallback(async () => {
    try {
      take(await api.chat(scopeRef.current, last.current));
      setError(null);
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    }
  }, [take]);

  // The mount read — everything retained — and a reset when the scope changes.
  useEffect(() => {
    last.current = 0;
    setMessages([]);
    setUnread(0);
    setFolded(loadFolded(key));
    void fetchDelta();
  }, [key, fetchDelta]);

  // A count above what is held: fetch, unless folded (the header shows the
  // difference instead). At or below: nothing — the cell that catches a panel
  // fetching on every move.
  useEffect(() => {
    if (count === undefined) return;
    if (count > last.current && !folded) void fetchDelta();
  }, [count, folded, fetchDelta]);

  // A tick: the delta, folded or not.
  const firstTick = useRef(true);
  useEffect(() => {
    if (tick === undefined) return;
    if (firstTick.current) {
      firstTick.current = false;
      return;
    }
    void fetchDelta();
  }, [tick, fetchDelta]);

  const toggle = () => {
    const next = !folded;
    setFolded(next);
    saveFolded(key, next);
    if (!next) setUnread(0);
  };

  const send = async () => {
    const text = draft.trim();
    if (!text || inFlight.current) return;
    inFlight.current = true;
    setSending(true);
    setError(null);
    try {
      const name = sendName ? displayName()?.trim() : undefined;
      const m = await api.postChat(scopeRef.current, { text, ...(name ? { name } : {}) });
      take({ messages: [m], last: m.seq });
      setDraft("");
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
      setSending(false);
    }
  };

  const badge = count !== undefined ? Math.max(0, count - last.current) : unread;
  const title = scope.kind === "house" ? "Lobby chat" : "Chat";
  const label = (m: ChatMessage) => {
    const a = m.author;
    if (a.kind === "seat")
      return <span className={`chat-who ${seatClass(colors?.[a.seat])}`}>{a.name ?? a.user}</span>;
    if (a.kind === "bot")
      return <span className={`chat-who ${seatClass(colors?.[a.seat])}`}>{a.name ?? "bot"}</span>;
    return (
      <span className={`chat-who${scope.kind === "table" ? " watching" : ""}`}>
        {a.name ?? a.user}
        {scope.kind === "table" && <span className="meta"> · watching</span>}
      </span>
    );
  };

  return (
    <section className={`panel chat${folded ? " folded" : ""}`} data-scope={key}>
      <Heading>
        {title}
        {folded && badge > 0 && (
          <span className="chat-new" aria-label={`${badge} new`}>
            {badge}
          </span>
        )}
        <button
          type="button"
          className="ghost fold"
          aria-expanded={!folded}
          aria-controls={`chat-${key}`}
          onClick={toggle}
        >
          {folded ? "show" : "hide"}
        </button>
      </Heading>
      <div id={`chat-${key}`} className={`messages${folded ? " folded" : ""}`}>
        <ol className="chat-lines" ref={box} role="log" aria-live="polite">
          {messages.map((m) => (
            <li key={m.seq}>
              {label(m)}
              <time dateTime={new Date(m.created_at * 1000).toISOString()}>{when(m.created_at)}</time>
              <span className="chat-text">{m.text}</span>
            </li>
          ))}
        </ol>
        {error && <div className="error">{error}</div>}
        <form
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <input
            aria-label="Say something"
            value={draft}
            maxLength={chatText()}
            disabled={sending}
            tabIndex={folded ? -1 : 0}
            onChange={(e) => setDraft(e.target.value)}
          />
          <button type="submit" className="ghost" disabled={sending || !draft.trim()} tabIndex={folded ? -1 : 0}>
            send
          </button>
        </form>
      </div>
    </section>
  );
}
```

(The `tabIndex` on a folded panel's controls is R-T8-1's rule from the turn account: `max-height: 0; overflow: hidden` clips and does not hide.) Note the "listitem" role: `ol.chat-lines > li` is what the cells count; the error `div` and the form are outside the list.

- [ ] **Step 4: The styles.** In `styles.css`, after the turn-account block (`:1329`):

```css
/* ---- M6.4: the chat panel ---------------------------------------------- */
/* LAST in the side column and pushed to its foot when the column has slack
 * (R-6.4.10): `margin-top: auto` in `.side`'s flex column. In the lobby it is an
 * ordinary section and the margin is inert. */
.side .chat { margin-top: auto; }
.chat h2, .chat h3 { display: flex; align-items: center; gap: 0.35rem; }
.chat h3 { font-size: 10px; letter-spacing: 0.2em; text-transform: uppercase; color: var(--fg-faint); font-family: var(--mono); font-weight: 500; margin-bottom: 0.5rem; }
.chat h2 .fold, .chat h3 .fold { margin-left: auto; font-size: 10px; padding: 0 0.4rem; letter-spacing: 0; text-transform: none; }
.chat-new { font-family: var(--mono); font-size: 10px; line-height: 1.4; padding: 0 0.45rem; border-radius: 999px; background: var(--reachable); color: var(--bg-sunken); }
/* The body folds the turn account's way: clipped, never unmounted, so the
 * live region stays in the tree. */
.chat .messages { margin-top: 0.3rem; display: flex; flex-direction: column; max-height: 16rem; }
.chat .messages.folded { max-height: 0; overflow: hidden; margin-top: 0; }
.chat-lines { list-style: none; margin: 0; padding: 0; flex: 1 1 auto; min-height: 0; max-height: 12rem; overflow: auto; scrollbar-width: thin; }
.chat-lines li { display: grid; grid-template-columns: auto auto 1fr; align-items: baseline; gap: 0.4rem; font-size: 11.5px; padding: 0.15rem 0.2rem; border-top: 1px solid var(--line); overflow-wrap: anywhere; }
.chat-lines time { color: var(--fg-faint); font-family: var(--mono); font-size: 10px; }
.chat-who { white-space: nowrap; }
.chat-who .meta { color: var(--fg-faint); }
.chat form { display: flex; gap: 0.35rem; margin-top: 0.4rem; }
.chat form input { flex: 1 1 auto; min-width: 0; font: inherit; font-size: 11.5px; background: var(--bg-sunken); color: var(--fg); border: 1px solid var(--line); border-radius: 3px; padding: 0.2rem 0.4rem; }
.chat form .ghost { font-size: 10px; padding: 0 0.5rem; letter-spacing: 0; text-transform: none; }
```

Check every `var(--…)` named exists (`grep -n '^\s*--' web/src/styles.css | head -30`); substitute the file's own token where one here does not.

- [ ] **Step 5: Run and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests '`

```bash
git add web/src/Chat.tsx web/src/Chat.test.tsx web/src/chatFold.ts web/src/styles.css
git commit -m "feat(web client): the chat panel — one component that owns its list, its fetch and its post, told where it is and when to look; folded by default and remembered per browser; a count above what it holds fetches once and a count at or below it fetches nothing; the lobby's label rule with the watching marker (spec §6.1, R-6.4.2, R-6.4.10)"
```

---

### Task 7: The four hosts, the always-held stream, the name at the door, the help (spec §6.2–§6.6, R-6.4.3, R-6.4.4, R-6.4.5, R-6.4.6)

**Files:**
- Modify: `web/src/App.tsx`, `web/src/App.test.tsx`, `web/src/Dealing.tsx`, `web/src/Dealing.test.tsx`, `web/src/OpenTable.tsx`, `web/src/OpenTable.test.tsx`, `web/src/Lobby.tsx`, `web/src/Lobby.test.tsx`, `web/src/SignIn.tsx`, `web/src/SignIn.test.tsx`, `web/src/Help.tsx`, `web/src/Help.test.tsx`, `web/src/styles.css` (the who-strip's name field)
- Test: the six test files above

**Interfaces:**
- Consumes: `<Chat>` (Task 6); `lobbyChat`, `setLobbyChat`, `chatText`, `setChatText`, `displayName`, `setDisplayName`, `clearDisplayName` (Task 5); `Events.chat`.
- Produces: `App`'s `arrive` callback (the arrival handler, hoisted and named); `posting: MutableRefObject<boolean>`.

- [ ] **Step 1: The failing host cells.** `App.test.tsx`:

```tsx
  // ---- M6.4: the stream is held at EVERY status (R-6.4.4) --------------------
  it("holds the table's stream during the player's own turn and after the game is over", async () => {
    for (const state of [wire.options, wire.gameOver]) {
      const stream = fakeStream();
      const stub = await renderApp(state, (call) => {
        if (call.path.endsWith(`/api/games/${state.id}/stream`)) return stream.reply(call);
        if (call.method === "GET" && call.path.includes("/chat")) return wire.chat;
        return undefined;
      });
      await until(() => stream.opens() > 0, "the stream is opened with the prompt on screen");
      expect(stream.opens()).toBe(1);
      void stub;
      cleanup();
    }
  });

  // ---- M6.4: a frame landing during this browser's own post is parked ----
  it("parks a frame that lands while its own move is in flight, and does not walk it (mutation 12)", async () => {
    const stream = fakeStream();
    const reply = gate<Reply>();
    const stub = await renderApp(wire.options, (call) => {
      if (call.path.endsWith(`/api/games/${wire.options.id}/stream`)) return stream.reply(call);
      if (call.method === "POST" && call.path.endsWith("/input")) return reply.promise;
      if (call.method === "GET" && call.path.includes("/chat")) return wire.chat;
      return undefined;
    });
    await until(() => stream.opens() > 0, "the stream is open");
    const user = userEvent.setup();
    // The first option row's button — read `Decision.test.tsx` for the selector this suite uses.
    await user.click(await screen.findByRole("button", { name: optionName(wire.options, 0) }));
    const framesBefore = stub.matching("GET", `/api/games/${wire.options.id}`).length;
    // The server's own bump for this move reaches the stream BEFORE the reply does.
    stream.push("signal", { ...wire.events, events: wire.options.events + 1, chat: wire.chat.last });
    await flush();
    expect(stub.matching("GET", `/api/games/${wire.options.id}`), "nothing fetched while posting").toHaveLength(framesBefore);
    reply.resolve({ ...wire.watching, events: wire.options.events + 1 });
    await flush();
    // Re-compared after the reply: the count matches and nothing is fetched or walked.
    expect(stub.matching("GET", `/api/games/${wire.options.id}`)).toHaveLength(framesBefore);
    expect(stub.matching("GET", "/at/")).toHaveLength(0);
  });

  it("draws the chat panel last in the side column, folded, for a player and for a watcher", async () => {
    for (const state of [wire.options, wire.spectator]) {
      await renderApp(state, (call) => (call.method === "GET" && call.path.includes("/chat") ? wire.chat : undefined));
      const side = document.querySelector("aside.side")!;
      await waitFor(() => expect(side.lastElementChild).toHaveClass("chat"));
      expect(within(side as HTMLElement).getByRole("button", { name: "show" })).toBeInTheDocument();
      cleanup();
    }
  });

  it("reads the lobby switch and the text cap from health at boot", async () => {
    await renderApp(wire.options, (call) =>
      call.path.endsWith("/api/health") ? { ...wire.health, lobby_chat: false, chat_text: 77 } : undefined,
    );
    await waitFor(() => expect(lobbyChat()).toBe(false));
    expect(chatText()).toBe(77);
    setLobbyChat(true);
    setChatText(500);
  });
```

(`optionName` is whatever `Decision.test.tsx` already uses to name an option row's button; `wire.watching`, `wire.gameOver`, `wire.spectator` exist. `cleanup` is `@testing-library/react`'s.) `Lobby.test.tsx`:

```tsx
  it("draws the lobby chat as a fourth section, folded, and not at all when the switch is off", async () => {
    setLobbyChat(true);
    show((call) => (call.method === "GET" && call.path.startsWith("/api/chat") ? wire.houseChat : undefined));
    const sec = await screen.findByRole("heading", { name: "Lobby chat" });
    expect(sec.closest("section")).toHaveClass("lobby-chat");
    expect(within(sec.closest("section")!).getByRole("button", { name: "show" })).toBeInTheDocument();
    cleanup();
    setLobbyChat(false);
    show();
    await screen.findByRole("heading", { name: "Lobby" });
    expect(screen.queryByRole("heading", { name: "Lobby chat" })).toBeNull();
    setLobbyChat(true);
  });

  it("offers Your name beside the address and remembers it as typed", async () => {
    const { user } = show();
    const field = await screen.findByLabelText("Your name");
    await user.type(field, "Captain Red");
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBe("Captain Red");
    await user.clear(field);
    expect(localStorage.getItem(DISPLAY_NAME_KEY)).toBeNull();
  });

  it("asks the house chat for its delta on a house frame", async () => {
    setLobbyChat(true);
    localStorage.setItem(OPEN_KEY, JSON.stringify(["house"]));
    const stream = fakeStream();
    const { stub } = show((call) => {
      if (call.path.endsWith("/api/stream")) return stream.reply(call);
      if (call.method === "GET" && call.path.startsWith("/api/chat")) return wire.houseChat;
      return undefined;
    });
    await until(() => stream.opens() > 0, "the house stream is open");
    const before = stub.matching("GET", "/api/chat").length;
    stream.push("house", { seq: 1, chat: 1 });
    await waitFor(() => expect(stub.matching("GET", "/api/chat")).toHaveLength(before + 1));
  });
```

`Dealing.test.tsx`: "draws the chat panel last in the side column, folded" (the same shape as `App`'s, over `wire.deal`, the stub answering `/chat` with `wire.chat`). `OpenTable.test.tsx`: "draws the chat panel for a seated caller under the link, and NOT for a stranger" (`showSeated` → present; `show` over `wire.tableOpen` → `screen.queryByText("Chat")` null and NO `GET …/chat` in the stub's calls). `SignIn.test.tsx`: "remembers an optional name typed at the door" (type an email and a name, submit, `DISPLAY_NAME_KEY` set; type an email alone, submit, the key untouched). `Help.test.tsx`: a `section("Talking")` helper (the `startingSection` shape, parameterized) and three cells: it names the fold and the foot of the column; it says a watcher can talk and is marked; it says a message is never undone and never in the record; plus "the watching section says you can talk".

- [ ] **Step 2: `App.tsx`.** (a) State and refs beside `pending` (`:236`): `const posting = useRef(false); const [chatCount, setChatCount] = useState(0);`. (b) Hoist the arrival handler: the `useCallback(async (e: Events) => { … }, [landed])` passed to `useSignal` (`:656`) becomes `const arrive = useCallback(async (e: Events) => { … }, [landed]);` declared ABOVE `send`, with two edits inside: first line `setChatCount(e.chat);` (before any check — a count is harmless at once), and the parking check `if (arriving.current || posting.current) { pending.current = e; return; }`. (c) `send` (`:533`): after `setError(null);` add `posting.current = true;`; **`if (replied.id === gameId.current) setState(replied);` (`:547`) becomes `if (replied.id === gameId.current) { setState(replied); stateNow.current = replied; }`, and the hot-seat re-read's `setState(mine)` (`:576`) likewise writes `stateNow.current = mine`** — the arrival loop's own assignment at `:703` is the precedent, and without this the parked frame below is compared against the PRE-move count (React's commit is scheduled, not synchronous) and walked as an arrival; in `finally`, before `inFlight.current = false;` add:

```ts
        posting.current = false;
        // A frame that landed during this post was parked (R-6.4.4); it is
        // compared now, against the state the reply produced, so a self-bump
        // is a no-op and an opponent's move made meanwhile walks.
        const parked = pending.current;
        pending.current = null;
        if (parked) void arrive(parked);
```

and `arrive` joins `send`'s dependency list. (d) The hook call becomes `useSignal(state?.id ?? null, !!state, arrive, …)` and the doc block above it is rewritten: `active` is "there is a game on screen" (R-6.4.4); R-9.1's sentence about `busy` stays as the reason the POSTING guard exists; a paragraph names M-1 and M-3 as closed by this. (e) Boot (`:530`): `api.health().then((h) => { setSignalKind(h.signal); setLobbyChat(h.lobby_chat); setChatText(h.chat_text); }).catch(() => {});`. (f) Render: after `<Dashboard … />` inside `aside.side` (`:1284-1293`):

```tsx
        {/* LAST, and folded (R-6.4.10): a bonus under the account and the
            players, not the main attraction. `key` remounts it per game so a
            switched table starts from its own list. */}
        <Chat
          key={state.id}
          scope={{ kind: "table", id: state.id }}
          count={chatCount}
          colors={colors}
          sendName={spectating}
        />
```

with `import { Chat } from "./Chat";` and the three `api.ts` imports.

- [ ] **Step 3: `Dealing.tsx`, `OpenTable.tsx`, `Lobby.tsx`, `SignIn.tsx`.** Dealing: `const [chatCount, setChatCount] = useState(0);` and the `useSignal` handler (`:200`) becomes `(e) => { setChatCount(e.chat); if (e.status !== "dealing") void start(); }`; last inside its `aside.side` (`:330`, after the `others.map(...)`): `<Chat scope={{ kind: "table", id: game }} count={chatCount} colors={colors} sendName={false} />`. OpenTable: `const [tick, setTick] = useState(0);` and `take` (`:91`) gains `setTick((t) => t + 1);` as its FIRST line (before the status branch); after the link field block (`:243-…`): `{seated && <Chat scope={{ kind: "table", id: game }} tick={tick} colors={table.colors} sendName={false} />}`. Lobby: `const [tick, setTick] = useState(0);` and `take` (`:105`) gains `setTick((t) => t + 1);`; `const [name, setName] = useState(() => displayName() ?? "");`; inside the who-strip `div` after `{who}`:

```tsx
          <label className="your-name">
            Your name
            <input
              value={name}
              placeholder="optional"
              onChange={(e) => {
                setName(e.target.value);
                const trimmed = e.target.value.trim();
                if (trimmed) setDisplayName(trimmed);
                else clearDisplayName();
              }}
            />
          </label>
```

and after the Games-in-progress section (`:263-273`):

```tsx
      {/* The house chat (R-6.4.5, R-6.4.6): a fourth section, folded by default —
          "enter the lobby chat" is the unfold — and absent when the server
          says the lobby chat is off. Its tick is this screen's own wake. */}
      {lobbyChat() && (
        <section className="saved lobby-chat">
          <Chat scope={{ kind: "house" }} tick={tick} sendName heading="h3" />
        </section>
      )}
```

SignIn: `const [name, setName] = useState("");` and a second field under the email — `<div className="field"><label htmlFor="sign-in-name">Your name (optional)</label><input id="sign-in-name" value={name} onChange={(e) => setName(e.target.value)} /></div>` — and in `submit`, before `onSignedIn(normalized)`: `if (name.trim()) setDisplayName(name.trim());`. `styles.css`: `.who-strip .your-name { display: inline-flex; align-items: center; gap: 0.35rem; margin-left: 0.75rem; font-size: 11.5px; color: var(--fg-dim); } .who-strip .your-name input { font: inherit; font-size: 11.5px; width: 9rem; background: var(--bg-sunken); color: var(--fg); border: 1px solid var(--line); border-radius: 3px; padding: 0.1rem 0.35rem; }`.

- [ ] **Step 4: `Help.tsx`.** After the "Watching a game" section, a new `<h3>Talking</h3>` with two paragraphs:

> At the foot of the side column — under the account of the last turn and the players — is a <strong>Chat</strong> panel, folded until you open it, with a count of what has arrived while it was closed. Everyone at the table can talk: each seat's line carries the seat's colour and the name it joined under, or its address, and a watcher's line says <em>watching</em>. A message is never undone and never part of the game's record; the server keeps a few hundred per table and drops the oldest, and the table's chat goes with the table when it is abandoned.
>
> The lobby has a chat of its own, <strong>Lobby chat</strong>, under the three lists, folded the same way — open it to join in. Whoever runs the server can switch it off, and then the section is simply not there. The name you type beside <em>You are</em> on the lobby is the name your messages carry, and the name a seat you take is labelled with; leave it blank and your address is used.

In "Watching a game", after "…and the final score once there is one.": ` You can talk, too: the chat panel is at the foot of the column, and what you say is marked as coming from a watcher.` Keep every `<strong>`/`<em>` boundary spaced (the `Help.test.tsx` rule).

- [ ] **Step 5: Run and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests ' && npm run e2e 2>&1 | tail -3` — the e2e must still pass unchanged (the walks do not open the panel).

```bash
git add web/src/App.tsx web/src/App.test.tsx web/src/Dealing.tsx web/src/Dealing.test.tsx web/src/OpenTable.tsx web/src/OpenTable.test.tsx web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/SignIn.tsx web/src/SignIn.test.tsx web/src/Help.tsx web/src/Help.test.tsx web/src/styles.css
git commit -m "feat(web client): the four hosts — the game and the deal read the count off the frame, the open table and the lobby tick on their wake, the lobby's section absent under the switch; the game screen holds its stream at every status and parks a frame that lands during its own post; Your name at the door and beside the address; the help's Talking section (spec §6.2–§6.6, R-6.4.3, R-6.4.4, R-6.4.5)"
```

---

### Task 8: The gates — the whole-game arm talks; the three-browser exit, headless (spec §9.4, R-plan-7)

**Files:**
- Modify: `crates/yokohama-web/tests/play_a_whole_game.rs`, `web/e2e/two_browsers.spec.ts`

- [ ] **Step 1: The two-identity arm.** In `two_people_play_to_a_winner_and_each_sees_only_their_own_hand`, right after green's `unlock` (`:~445`) and before the move loop:

```rust
    // ---- M6.4: the three of them talk before the first move (spec §9.4) ----
    let (status, m) = api
        .call_as(green, "POST", "/api/games/two/chat", Some(serde_json::json!({ "text": "hello from the rail", "name": "Green" })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "the watcher's one write: {m}");
    assert_eq!(m["author"]["kind"], "watcher", "{m}");
    for (user, seat, text) in [(red, 0, "gl"), (blue, 1, "hf")] {
        let (status, m) = api
            .call_as(user, "POST", "/api/games/two/chat", Some(serde_json::json!({ "text": text })))
            .await;
        assert_eq!(status, StatusCode::CREATED, "{m}");
        assert_eq!((m["author"]["kind"].as_str(), m["author"]["seat"].as_u64()), (Some("seat"), Some(seat)), "{m}");
    }
    let (_, m) = api.call_as(blue, "GET", "/api/games/two/chat?after=2", None).await;
    assert_eq!(m["messages"][0]["author"]["name"], "Blue", "the seat's stored name from the join: {m}");
    for who in [red, blue, green] {
        let (status, page) = api.call_as(who, "GET", "/api/games/two/chat", None).await;
        assert_eq!(status, StatusCode::OK, "{who}: {page}");
        assert_eq!(page["messages"].as_array().map(Vec::len), Some(3), "{who} reads all three: {page}");
        let (_, e) = api.call_as(who, "GET", "/api/games/two/events", None).await;
        assert_eq!(e["chat"], 3, "{who}'s frame carries the count: {e}");
    }
```

Run: `cargo test -p yokohama-web --test play_a_whole_game two_people 2>&1 | tail -3` (the vs-bot arm is not run here; owner item 2).

- [ ] **Step 2: The three-browser exit (R-6.4.16 is the LIVE review's; this stays headless).** In `two_browsers.spec.ts`: (a) install the players' no-reopen counters right after `const a = …`/`const b = …` are created — `const streamOpens = { a: 0, b: 0 }; for (const [k, p] of [["a", a], ["b", b]] as const) p.on("request", (r) => { if (new URL(r.url()).pathname === streamPath) streamOpens[k] += 1; });` — which needs `streamPath` before `id` exists: declare `let streamPath = ""` up top and set it once `id` is known (the listener reads the variable at request time). (b) After `expect(await ownHand(c), "a watcher is dealt no hand").toEqual([]);`, the chat exchange:

```ts
  // ---- M6.4: the exit — two players talk, the watcher reads it and answers,
  // and the MOVER reads the answer during their own turn (spec §9.4) --------
  const chatOf = (p: Page) => p.locator("aside.side section.chat");
  const unfold = async (p: Page) => {
    const fold = chatOf(p).getByRole("button", { name: "show" });
    if (await fold.count()) await fold.click();
    await expect(chatOf(p).getByRole("button", { name: "hide" })).toBeVisible();
  };
  const say = async (p: Page, text: string) => {
    await unfold(p);
    await chatOf(p).getByRole("textbox", { name: "Say something" }).fill(text);
    await chatOf(p).getByRole("textbox", { name: "Say something" }).press("Enter");
    await expect(chatOf(p).getByRole("listitem").filter({ hasText: text })).toBeVisible();
  };
  const reads = async (p: Page, text: string) => {
    await unfold(p);
    await expect(chatOf(p).getByRole("listitem").filter({ hasText: text }), `${text} arrives without a reload`).toBeVisible({ timeout: 15_000 });
  };
  await say(a, "hello from a");
  await reads(b, "hello from a");
  await say(b, "and b");
  await reads(c, "hello from a");
  await reads(c, "and b");
  await say(c, "watching you both");
  await expect(chatOf(c).getByRole("listitem").filter({ hasText: "watching you both" })).toContainText("watching");
  // **The mover reads it during their OWN turn** — the cell that must fail
  // under the old `active` (R-6.4.4): whichever page shows a move, that page's
  // panel receives the watcher's line while its prompt is on screen.
  const mover = hasMove(await readScreen(a)) ? a : b;
  expect(hasMove(await readScreen(mover)), "the premise: it is this page's turn").toBe(true);
  await reads(mover, "watching you both");
  // The players' game-screen streams, read once the game is on screen (R-plan-7).
  const streamsAtStart = { a: streamOpens.a, b: streamOpens.b };
```

(c) At the end, beside the watcher's `expect(streams…).toBe(1)`:

```ts
  // **No reopen for a player either** (R-6.4.4, R-plan-7): the per-turn
  // teardown is gone, so the count read at the start is the count at the end.
  expect(streamOpens.a, "a never reopened this table's stream").toBe(streamsAtStart.a);
  expect(streamOpens.b, "b never reopened this table's stream").toBe(streamsAtStart.b);
```

`readScreen` presses the walk's skip once — after `say`/`reads` the panel is open and may cover nothing that `readScreen` reads; if a cell's `act` selector collides with the panel's "send" button, scope `act`'s reads to `.deck` (read `harness.ts:349`).

Run: `cd web && npm run e2e 2>&1 | tail -4` — Expected `39 passed`; the two-browser line prints its wall; record it beside 309 s in the ledger. Clear `web/test-results/` first if the disk is under 12 GiB (owner item 4).

```bash
git add crates/yokohama-web/tests/play_a_whole_game.rs web/e2e/two_browsers.spec.ts
git commit -m "test(web): the exit, played — the whole-game arm's three identities talk before the first move and every frame carries the count; the three-browser walk has two players talk, the watcher read and answer, the mover read the answer during their own turn, and no player reopen this table's stream (spec §9.4, R-6.4.4, R-plan-7)"
```

---

### Task 9: Release `yokohama-web:0.5.0-m6.4` (spec §10)

- [ ] **Step 1: Owner authorization**, then build and deliver per `deploy/RUNBOOK.md` §3.5. **The container's `Env` is unchanged**: the three variables have defaults. Record the image tag, the build commit, the container id and the time.
- [ ] **Step 2: Verify from the shell** through the tunnel with the identity header: `/api/health` carries `lobby_chat`, `chat_keep`, `chat_text`; `POST /api/chat {"text":"release check"}` answers 201 and `GET /api/chat` reads it back; a table's `…/stream` on-connect frame carries `chat`.
- [ ] **Step 3: Record §3.10** in the runbook (the deployment; the three variables as OPTIONAL with their defaults, and what `off`/`all` do), and add one sentence to each Dockerfile's `YOKOHAMA_WEB_IDENTITY` comment block (`deploy/Dockerfile:90`, `Dockerfile.runtime:54`): "The three chat variables (`YOKOHAMA_WEB_LOBBY_CHAT`, `_CHAT_KEEP`, `_CHAT_TEXT`) are the other way round: each has a default, so an image started without them runs the lobby chat on with 500/500 (M6.4, R-6.4.6, R-6.4.8)."

```bash
git add deploy/RUNBOOK.md deploy/Dockerfile deploy/Dockerfile.runtime
git commit -m "docs(deploy): the seventh deployment — yokohama-web:0.5.0-m6.4 (chat), the three optional variables and their defaults"
```

---

### Task 10: The battery, the HEADED live review, and the close (spec §9.5, §10, R-6.4.16)

- [ ] **Step 1: The mutation battery** (spec §9.5, eighteen mutations), one at a time on the local build, the first failing assertion verbatim, reverted; recorded in the ledger. An Opus subagent, per R-P11; its report to a file.
- [ ] **Step 2: The live review, HEADED (R-6.4.16).** A CLEAN Fable subagent, from this workstation through WARP (Access is the door; the NAS's `claimed` mode makes each typed email a person — WARP stays ON, the LAN path stays CLOSED): Playwright with `headless: false`, three contexts as three people, two playing and one arriving by link and unlocking, in both layouts (1440×900 and 390×844); every claim below read BY EYE on screen and photographed, the photographs under the SDD workspace: the panel sits at the column's foot folded with its count; the labels are chips and names, the watcher's marked; the mover's panel receives a line DURING their own turn; the folded count moves and clears on unfold; the lobby's fourth section is folded and opens; the name typed beside "You are" labels the next message; each player's network panel holds ONE stream across a whole turn boundary; zero console errors on all three. Then one pass in Claude in Chrome under the owner's own profile (one identity: the owner's) reading the same screens. The subagent runs everything in the FOREGROUND and never backgrounds a run to wait on it (the M6.3 lesson).
- [ ] **Step 3: The counts in one sequence**, the blast-radius diff (the five crates AND the CLI empty), `SEMANTICS_REV` 7, clippy.
- [ ] **Step 4: The close** in the M6.3 close's shape (§1–§8): what shipped, the counts, the corpus diff (eleven added, `events*` and `health` changed, every frame fixture byte-identical), the reviews, the execution rulings with their costs, the LEAVE list (the wake-driven lobby's list refetch per house message; the per-process count; the `Bot` arm; whatever the reviews leave), the owner items carried (2–4 of M6.3's), what it does NOT claim (spec §11 restated), and the handover into M6.5 — whose entry artifact is written beside it. Then the doc pass: `CLAUDE.md` (the web bullets: a CHAT paragraph in the architecture bullet, the counts, the corpus figure, the M6.4 headline), one `guidingpath.md` row, `README.md`, `docs/handover.md` Track 3.

```bash
git add docs/plans/<date>-web-frontend-m6.4-close.md docs/plans/<date>-web-frontend-m6.5-hosting-handover.md CLAUDE.md guidingpath.md README.md docs/handover.md
git commit -m "docs(web): M6.4 close — two players talk, a third reads and answers, and the lobby talks; the headed live review; the handover into M6.5"
```

---

## Deviations from the spec (recorded per R-P11; the close carries the costs)

- R-plan-1 through R-plan-7 above. The close lists any taken in execution beside them.

## Self-review (run before the plan is handed over)

- **Spec coverage.** §2.1–§2.3 → Task 1; §2.4, §4 (the channel and the two bump sites) → Task 2; §5 → Task 3; §3.1–§3.5, §4 (`Events.chat`, the house frame), §7 (the sweep, the grid, the seat-name fence) → Task 4; §8, §7 (the narrowed pin), §6.5 → Task 5; §6.1 → Task 6; §6.2–§6.4, §6.6 → Task 7; §9.4 → Task 8; §10 → Tasks 9 and 10; §9.5, §11, §12 → Task 10 and the close; §9.6 → every task's gate; R-6.4.13 → Task 1's seat-bound cell and Task 6's label cell over the corpus's colours (2P) with the Rust bound at 2P/3P/4P; R-6.4.15 → Task 1's `Bot` arm and Task 2's single writer; R-6.4.16 → Task 10.
- **Placeholders.** The `…` in Task 5's pin stands for the M6.3 partner kept verbatim, and the paragraph names it; `optionName` in Task 7 names the file whose selector to reuse; `get_json` in Task 3 names the existing health cell to copy from; `house_frames_after` in Task 4 is defined in the same step. No "TBD".
- **Type consistency.** `Doc::{Chat, House}` and `chat::HOUSE` (Task 1) are what Tasks 2 and 5 name; `Games::chat_append(id, author, &str)`, `house_append(author, &str)`, `chat_page(id, after) -> (Vec<Message>, u64)`, `house_page(after)`, `chat_last(id) -> u64`, `Games::with_limits(rules, store, limits)` (Task 2) are what Task 4's handlers and Task 3's `AppState` call; `signal::House { seq, chat }` (Task 2) is what `house_stream` maps (Task 4) and the client's fake pushes (Task 7's Lobby cell); `ChatConfig { lobby, limits }` (Task 3) is what `in_memory_with_chat` takes (Tasks 4, 5); `ChatPage { messages, last }` (Task 4) is `types.ts`'s `ChatPage` (Task 5) and `Chat`'s `take` argument (Task 6); `api.chat(scope, after)`/`api.postChat(scope, body)` and `lobbyChat`/`chatText`/`setLobbyChat`/`setChatText`/`clearDisplayName` (Task 5) are what Tasks 6 and 7 import; `<Chat scope count? tick? colors? sendName />` (Task 6) is what the four hosts render (Task 7); `arrive` and `posting` (Task 7) are what its own `send` uses.
