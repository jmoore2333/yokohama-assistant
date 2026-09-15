# Web frontend — M6.2 "the lobby": implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the lobby the first screen, let a table be opened with seats to fill and an optional password, give a table an address a link can carry, and let a person holding only that link sit down and play — so that the exit criterion of the room plan's M6.2 line holds on the NAS: someone holding only a link joins a table and plays.

**Architecture:** Server-first, then client. The table document gains one status (`open`) and one seat kind (`open`), a per-seat display name, an optional salted-hash lock and the winner on its turn mirror; one new route, `POST …/{id}/join`, fills a seat under a per-table lock and flips the table to `dealing` in the same write when it fills the last one; the listing gains two scopes and a cursor and still reads table documents alone; `GET …/{id}/table` opens to any signed-in caller at an open table and nothing else widens. The client gains hash routes, a lobby with two sections, an open-table screen with a seat picker and a copyable link, and one generalized poll hook with four consumers whose transport M6.3 replaces. `games.rs` is cut along its seam first, so every later diff reads as change and not motion.

**Tech Stack:** axum 0.8 + tokio (server), serde (documents), `sha2` 0.10 (the table password, R-2.8), React 19 + TypeScript + Vite (client), vitest + Testing Library over jsdom (component tests), Playwright against the real release binary (browser walk), the Portainer Docker proxy (deploy).

**Spec:** `docs/plans/2026-09-15-web-frontend-m6.2-lobby-spec.md` — the authority; this plan argues from it and records where it deviates (§ "Deviations" at the end). Above it: `docs/plans/2026-09-14-web-frontend-m6-room-plan.md` (§0, R-P1–R-P11) and the M6.1 close (`docs/plans/2026-09-15-web-frontend-m6.1-close.md`, §6 and §8). Beside it: the M6.3 spec, whose §5.1 is the reason the hook's signature is fixed in Task 7 exactly as the M6.2 spec's §5.5 writes it.

## Global Constraints

- **Branch:** `web-room-m6.2` in the MAIN checkout `/Users/jmoore/Documents/Github/yokohama-assistant` (no worktree this cycle; the branch was cut from `web-room`'s tip `937ce92`, and its first two commits are `48ae908` — the CI change — and `9422a1d` — the two specs). Every command runs from the checkout root unless it says `cd web`. Stage with explicit paths (`git add <path>`), never `git add -A`. Commit messages end with `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` and the EXECUTING session's `Claude-Session:` trailer.
- **Blast radius (spec §8.5):** at the close, `git diff master -- crates/yokohama-core crates/yokohama-oracle crates/yokohama-difftest crates/yokohama-bridge crates/gen-data` is EMPTY, `SEMANTICS_REV` is **7**, and **`crates/yokohama-cli` is touched nowhere** — `cargo test -p yokohama-cli` stays green with its goldens and `guide_sync` untouched.
- **Per-edit gate (after EVERY task):** `cargo test -p yokohama-web && cargo clippy --workspace --all-targets -- -D warnings` when `crates/` moved; `cd web && npm run typecheck && npm test` when `web/` moved; `cd web && npm run e2e` when `web/` moved (it stays GREEN throughout: Task 7 re-points the browser harness in the same task that moves the first screen). The whole-game gate is per-edit locally and OUT of CI (`48ae908`).
- **The wire corpus is a contract.** Any task that changes a payload runs `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures`, then `cargo test -p yokohama-web --test wire_fixtures` WITHOUT the variable, then `cd web && npm run typecheck`, and its report names every fixture file whose diff changed and why. Tasks 3, 4, 5 and 6 regenerate it.
- **No hand-authored payload in a test** (spec §8.2). A component cell renders a fixture from `web/src/wire/` or nothing.
- **The spec's rulings**: all tables visible to the house, one optional password per table (R-2.1); start when full is automatic (R-2.2); the display name is a per-seat field beside the email (R-2.3); the game's name is a label and the id is drawn (R-2.4); a join link is `#/t/<id>` (R-2.5); abandon is the creator's delete (R-2.6); an open table is readable and joinable by any signed-in caller, and that is the ONLY widening of R-1.5 (R-2.7); `sha2` (R-2.8); the lock splits by table (R-2.9); the `games.rs` cut first (R-2.10).
- **Counts at HEAD `9422a1d`** (re-measure at Task 0 and again at Task 11, never copy): `cargo test -p yokohama-web` **122** (115 lib + 2 gate + 5 wire_fixtures), `cd web && npm test` **447** in 29 files, `npm run e2e` **38** (+2 opt-in), wire corpus **54** files.

---

## Execution model

| Seat | Model | Tasks |
|---|---|---|
| Implementer | **Opus** (xhigh) | 0–10 |
| Per-task reviewer | **Opus**, fresh context | every task |
| Orchestrator, adjudication | **Fable** | all |
| Deploy (outward-facing, owner-authorized) | the orchestrator through the Portainer MCP, as M6.1's Task 11 was | 10 |
| The final review against the LIVE version | **Fable**, a CLEAN subagent with the brief in Task 11 | 11 |

No Sonnet or Haiku dispatch (R-P11). The SDD workspace is `.superpowers/sdd/2026-09-15-web-frontend-m6.2-plan/progress.md` in the shape the M6.1 wave used (pre-flight scan, one ruling row per decision, one report and one review per task); `git status --short` must not list it. **Read HEAD before following up an implementer's DONE, and never amend a reviewed SHA — a fix is a new commit.**

**Pre-flight (the orchestrator, before Task 0):** `git status` clean at `9422a1d` or later on `web-room-m6.2`; `cd web && npm ci` done (the main checkout has no `node_modules` — the worktree's was discarded); `npx playwright install chromium` done; `cargo build --release -p yokohama-web` warm. Record the executing session's `Claude-Session` URL for every commit trailer.

---

## File structure

| File | Responsibility | Task |
|---|---|---|
| `crates/yokohama-web/src/tables.rs` (new), `games.rs`, `lib.rs` | The cut: the table lifecycle leaves `games.rs` behaviour-preserved | 1 |
| `crates/yokohama-web/src/table.rs`, `tables.rs`, `games.rs`, `Cargo.toml` | `SeatDoc::Open`, `name`, `TableStatus::Open`, `Lock`, `TurnMirror.winner`, `TABLE_SCHEMA` 2, the schema and shape arms, `check_password`, `TableView.locked/winner`, `sha2` | 2 |
| `crates/yokohama-web/src/tables.rs`, `games.rs`, `lib.rs`, `tests/wire_fixtures.rs` | `NewGame` re-shaped; `config_of` order; `open` at create; `GameError::{Open, Playing}`; the corpus re-cut | 3 |
| `crates/yokohama-web/src/tables.rs`, `games.rs`, `lib.rs`, `tests/wire_fixtures.rs` | `POST …/{id}/join`; the per-table locks; `Keep.name`; delete at `open`; the status refusals | 4 |
| `crates/yokohama-web/src/tables.rs`, `lib.rs`, `tests/wire_fixtures.rs` | `GET /api/games` scopes and paging; `table` at `open` for anyone; the route sweep at four statuses | 5 |
| `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts` | The walk through an open table; the new fixtures; the password value pin; the client's types and calls | 6 |
| `web/src/route.ts` (+ `.test.ts`, new), `web/src/signal.ts` (+ `.test.ts`), `web/src/tables.ts` (+ `.test.ts`), `web/src/Lobby.tsx` (+ `.test.tsx`, new), `web/src/App.tsx` (+ `.test.tsx`), `web/src/NewGame.tsx` (+ `.test.tsx`), `web/src/test/harness.tsx`, `web/e2e/harness.ts`, `web/src/styles.css` | Hash routes; `usePolled` with `Source`; `seatName`/`tableWord` widened; the lobby as the first screen; the form re-pointed; the browser harness re-pointed | 7 |
| `web/src/OpenTable.tsx` (+ `.test.tsx`, new), `web/src/App.tsx`, `web/src/Dealing.tsx` (+ `.test.tsx`), `web/src/styles.css` | The open table's two faces; keep carries the name; the `seatName` surfaces | 8 |
| `crates/yokohama-web/tests/play_a_whole_game.rs`, `web/e2e/joins_by_link.spec.ts` (new) | The two-identity gate arm through the lobby; the join-by-link browser walk | 9 |
| `deploy/RUNBOOK.md` (§3.8) | The fifth deployment, `yokohama-web:0.5.0-m6.2` | 10 |
| `docs/plans/2026-09-15-web-frontend-m6.2-close.md` (new), `CLAUDE.md`, `guidingpath.md`, `README.md`, `docs/handover.md` | The final review and the close | 11 |

---

## Plan-level rulings (deviations from the spec's letter, recorded here per R-P11)

- **R-plan-1 — `Games`'s fields become `pub(crate)` for the cut.** A second `impl Games` block in `tables.rs` needs `rules`, `store`, `live`, `written`, `table_lock`, `locks` and `deleted`; the alternative (accessors for each) is seven functions that exist to be called from one file.
- **R-plan-2 — `mirror_of` stays in `games.rs` as `pub(crate)`**, because `persist`'s `mirror_turn` (record lifecycle) and `finalize` (table lifecycle) both call it. `winner` is read from `repl.session.final_result().winner` — the same `GameResult` the result route projects (`outcome.rs:96`).
- **R-plan-3 — the lib test helpers `req`/`req_seeded`/`two_people` set `id`, not `name`** (spec §3.5). `NewGame.name` in those helpers is `None`; a cell about names sets it itself.
- **R-plan-4 — the lobby is ONE `usePolled` instance** whose `once` fetches both scopes with `Promise.all` and hands the pair to `onValue`; two instances would be two cadences and two backoffs for one screen.
- **R-plan-5 — the browser harness is re-pointed in Task 7**, the task that moves the first screen, so `npm run e2e` never goes red; the spec's §5.8 places it with the gates, which is where its NEW cells go (Task 9).
- **R-plan-6 — `display_name` on `NewGame` is written onto every seat whose `user` equals the creator** (spec §3.1); a creator naming a second seat by their own email gets the name on both, which is the hot-seat case and is right.

---

### Task 0: Pre-flight and baseline

**Files:**
- Create: `.superpowers/sdd/2026-09-15-web-frontend-m6.2-plan/progress.md` (gitignored)

- [ ] **Step 1: Confirm the checkout and the branch**

Run: `git status --short && git branch --show-current && git log --oneline -1 | cut -c1-60`
Expected: no output from status; `web-room-m6.2`; `9422a1d` or later.

- [ ] **Step 2: Re-count the baselines**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result'`
Expected: four lines — `115 passed`, `0 passed` (main.rs), `2 passed`, `5 passed`.

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests '`
Expected: `29 passed`, `447 passed`.

Run: `ls web/src/wire/*.json | wc -l`
Expected: `54`.

- [ ] **Step 3: Open the SDD ledger**

Write `progress.md` with: the plan path, the branch, the base commit, the counts above, and an empty "Rulings" table with columns `#`, `task`, `question`, `ruling`, `cost`. Confirm `git status --short` still prints nothing.

---

### Task 1: The cut — `tables.rs` leaves `games.rs`, behaviour preserved (R-2.10)

**Files:**
- Create: `crates/yokohama-web/src/tables.rs`
- Modify: `crates/yokohama-web/src/games.rs`, `crates/yokohama-web/src/lib.rs` (one `mod tables;` line and the `use` paths)
- Test: none new — the whole point is that the count does not move

**Interfaces:**
- Consumes: everything in `games.rs` at `9422a1d`.
- Produces: `crate::tables` exporting, byte-for-byte as they are today: `WireSeed`, `SeatSpec`, `NewGame`, `DealCard`, `TABLE_SCHEMA`, `bot_keep`, `draw_seed`, `config_of`, `check_shape`, `config_from`, `contestants_of`, `choices_of`, `cards_of`, `withhold_until_the_choice`, `first_human_seat`, `set_table`, and the `impl Games` methods `create`, `read_table`, `write_table`, `table`, `settle`, `settle_locked`, `finalize`, `flip_to_playing`, `deal`, `keep`, `list_for`, `delete`. `games.rs` keeps `GameError`, `DEALING_SENTENCE`, `Games` (struct, `new`, `store`), `mirror_of` (now `pub(crate)`), `handle`, `live_handle`, `persist`, `mirror_turn`, `conflict`, `evict_for_test`, `evict_down_to`, `bring_alive`, `MAX_RESIDENT`. `Games`'s fields become `pub(crate)` (R-plan-1).

- [ ] **Step 1: Record the baseline the cut must reproduce**

Run: `cargo test -p yokohama-web --lib 2>&1 | grep 'test result' && grep -c '#\[test\]\|#\[tokio::test\]' crates/yokohama-web/src/games.rs`
Expected: `115 passed`; the second number is the count of tests in `games.rs` — write it in the SDD ledger (the reviewer checks that `games.rs` + `tables.rs` sum to it after the cut).

- [ ] **Step 2: Create `tables.rs` and move the table lifecycle into it**

Create `crates/yokohama-web/src/tables.rs` with a module doc that says what it is (the table lifecycle: create, deal, keep, finalize, list, delete — the record lifecycle stays in `games.rs`) and MOVE, without editing any body: the free items listed under "Produces" above, and the `impl Games` methods listed there, into a second `impl Games` block. Move each test with its subject: a `#[test]` whose first call is to a moved function goes to `tables.rs`'s `mod tests`; the rest stay. The shared helpers (`games()`, `who`, `req_seeded`, `req`, `two_people`, `playing`, `CORPUS_SEED`, `ARGMAX_IS_CARD_2`, `stored_json`) go into a `#[cfg(test)] pub(crate) mod test_support` at the bottom of `games.rs`, and both `mod tests` blocks `use super::test_support::*` (or `crate::games::test_support::*`).

In `games.rs`, change the struct's fields to `pub(crate)`:

```rust
pub struct Games {
    pub(crate) rules: Arc<Rules>,
    pub(crate) store: Arc<dyn Store>,
    pub(crate) live: Mutex<HashMap<GameId, Arc<Mutex<Repl>>>>,
    pub(crate) written: Mutex<HashMap<GameId, Version>>,
    pub(crate) table_lock: Mutex<()>,
    pub(crate) deleted: Mutex<std::collections::HashSet<GameId>>,
}
```

and `fn mirror_of` to `pub(crate) fn mirror_of`. In `lib.rs`, add `pub mod tables;` beside `pub mod games;` and re-point every `use crate::games::{NewGame, …}` that names a moved item to `crate::tables::…` (grep `games::` in `lib.rs`, `client.rs`, `table.rs` and both `tests/*.rs`; `tests/wire_fixtures.rs` and `tests/play_a_whole_game.rs` reach `NewGame` through JSON and name nothing).

- [ ] **Step 3: Compile, run the suite, and prove the count**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: `115 passed` for the lib (now spread across two modules — `cargo test -p yokohama-web --lib -- games:: tables::` lists both), `2 passed`, `5 passed`; clippy clean.

Run: `git diff --stat`
Expected: `games.rs` shrinks by roughly what `tables.rs` grows; `lib.rs` a handful of lines.

- [ ] **Step 4: Commit**

```bash
git add crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs
git commit -m "refactor(web): games.rs is cut along its seam — the table lifecycle (create, deal, keep, finalize, list, delete) moves to tables.rs with its tests, the record lifecycle stays; no body edited, 115 lib cells before and after (R-2.10)"
```

---

### Task 2: The table document, widened (spec §2) and `sha2` (R-2.8)

**Files:**
- Modify: `crates/yokohama-web/src/table.rs`, `crates/yokohama-web/src/tables.rs` (`check_shape`, `TABLE_SCHEMA`), `crates/yokohama-web/src/games.rs` (`mirror_of`), `crates/yokohama-web/Cargo.toml`
- Test: `crates/yokohama-web/src/table.rs` (unit), `crates/yokohama-web/src/tables.rs` (the shape arms)

**Interfaces:**
- Produces: `SeatDoc::Human { user: String, name: Option<String> }`, `SeatDoc::Bot`, `SeatDoc::Open`; `TableStatus::{Open, Dealing, Playing}`; `TurnMirror { to_move, over, winner: Option<u8> }`; `Lock { salt: String, hash: String }` with `Lock::new(password: &str) -> Lock` and `Lock::matches(&self, given: &str) -> bool`; `TableDoc.lock: Option<Lock>`; `TableDoc::open_seats(&self) -> Vec<PlayerId>`, `is_full(&self) -> bool`, `check_password(&self, given: &str) -> bool`; `TableView.locked: bool`, `TableView.winner: Option<PlayerId>`; `TABLE_SCHEMA = 2`; `check_shape` with the six new arms.

- [ ] **Step 1: Write the failing unit tests in `table.rs`**

Append to `table.rs`'s `mod tests` (create the module if the file has none):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn doc() -> TableDoc {
        TableDoc {
            schema: 2,
            id: "t".into(),
            name: None,
            created_by: "red@t.test".into(),
            created_at: 1_789_000_000,
            players: 3,
            colors: vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green],
            start_seat: 0,
            seats: vec![
                SeatDoc::Human { user: "red@t.test".into(), name: Some("Red".into()) },
                SeatDoc::Open,
                SeatDoc::Open,
            ],
            keeps: vec![None, None, None],
            seed: 1,
            status: TableStatus::Open,
            turn: None,
            lock: None,
        }
    }

    #[test]
    fn a_schema_one_document_reads_with_every_new_field_at_its_default() {
        let json = r#"{"schema":1,"id":"old","name":"old","created_by":"red@t.test",
            "created_at":1789000000,"players":2,"colors":["Red","Blue"],"start_seat":0,
            "seats":[{"kind":"human","user":"red@t.test"},{"kind":"human","user":"blue@t.test"}],
            "keeps":[1,2],"seed":7,"status":"playing","turn":{"to_move":0,"over":false}}"#;
        let doc: TableDoc = serde_json::from_str(json).expect("a schema-1 document parses");
        assert!(doc.lock.is_none());
        assert!(doc.seats.iter().all(|s| matches!(s, SeatDoc::Human { name: None, .. })));
        assert_eq!(doc.turn.expect("a mirror").winner, None);
        assert!(!doc.is_full() || doc.open_seats().is_empty());
    }

    #[test]
    fn open_seats_and_is_full_read_the_seats() {
        let d = doc();
        assert_eq!(d.open_seats(), vec![PlayerId::new(1), PlayerId::new(2)]);
        assert!(!d.is_full());
        let mut full = d.clone();
        full.seats[1] = SeatDoc::Bot;
        full.seats[2] = SeatDoc::Human { user: "b@t.test".into(), name: None };
        assert!(full.is_full());
        assert!(full.open_seats().is_empty());
    }

    #[test]
    fn a_lock_matches_its_password_and_refuses_a_one_byte_variant_and_the_empty_string() {
        let lock = Lock::new("sesame");
        assert_eq!(lock.salt.len(), 32, "16 bytes as hex");
        assert_eq!(lock.hash.len(), 64, "SHA-256 as hex");
        assert!(lock.matches("sesame"));
        assert!(!lock.matches("sesamf"));
        assert!(!lock.matches("sesam"));
        assert!(!lock.matches(""));
        assert_ne!(Lock::new("sesame").salt, lock.salt, "two locks for one password differ");
        let mut d = doc();
        assert!(d.check_password("anything"), "no lock: any password passes");
        d.lock = Some(lock);
        assert!(d.check_password("sesame"));
        assert!(!d.check_password("open"));
    }

    #[test]
    fn the_view_carries_locked_and_winner_and_never_the_salt_or_the_hash() {
        let mut d = doc();
        d.lock = Some(Lock::new("sesame"));
        d.turn = Some(TurnMirror { to_move: 1, over: true, winner: Some(2) });
        let v = serde_json::to_value(d.view(&User::parse("red@t.test").unwrap())).unwrap();
        assert_eq!(v["locked"], true);
        assert_eq!(v["winner"], 2);
        let text = v.to_string();
        for key in ["salt", "hash", "\"lock\""] {
            assert!(!text.contains(key), "{key} must not reach the view: {text}");
        }
        assert_eq!(v["seats"][1]["kind"], "open");
        assert_eq!(v["seats"][0]["name"], "Red");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p yokohama-web --lib table::`
Expected: compile errors — `SeatDoc::Open`, `Lock`, `lock`, `winner`, `open_seats`, `is_full`, `check_password` do not exist.

- [ ] **Step 3: Add `sha2` and write the document changes**

In `crates/yokohama-web/Cargo.toml`, after the `tokio` line, add with its paragraph:

```toml
# M6.2 (owner-ruled 2026-09-15, spec R-2.8): the table password. The workspace
# has no hash crate — `fnv1a_64` in yokohama-core is a digest, not a password
# hash — and the threat is a shared game password behind Cloudflare Access, so
# a salted SHA-256 is the whole of it: no argon2, no constant-time crate (the
# compare is a fold of XORs in table.rs).
sha2 = "0.10"
```

In `table.rs`, replace the three types and add the new ones (doc comments as in the spec §2):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SeatDoc {
    Human {
        user: String,
        #[serde(default)]
        name: Option<String>,
    },
    Bot,
    /// An unfilled seat at an open table (M6.2). Never present once the table
    /// has left `Open` — `check_shape` refuses the pair.
    Open,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TableStatus {
    Open,
    Dealing,
    Playing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnMirror {
    pub to_move: u8,
    pub over: bool,
    /// The ENGINE's winner once `over` — the same value `/result` projects as
    /// `Placing.won`, never `standings[0]` (rank order; `outcome.rs:71`).
    #[serde(default)]
    pub winner: Option<u8>,
}

/// The table password, as stored: hex of 16 random bytes and hex of
/// SHA-256(salt_bytes ‖ password_bytes). Serialized for the DOCUMENT; `TableView`
/// has no field of this type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lock {
    pub salt: String,
    pub hash: String,
}

impl Lock {
    pub fn new(password: &str) -> Self {
        let salt_bytes: [u8; 16] = {
            let a = crate::tables::draw_seed().to_le_bytes();
            let b = crate::tables::draw_seed().to_le_bytes();
            let mut s = [0u8; 16];
            s[..8].copy_from_slice(&a);
            s[8..].copy_from_slice(&b);
            s
        };
        let salt = hex(&salt_bytes);
        let hash = hex(&digest(&salt_bytes, password));
        Self { salt, hash }
    }

    /// Constant-time over the hash bytes: every byte is compared and the
    /// mismatches are folded with OR, so the time does not depend on WHERE the
    /// first difference is.
    pub fn matches(&self, given: &str) -> bool {
        let Some(salt_bytes) = unhex(&self.salt) else { return false };
        let Some(stored) = unhex(&self.hash) else { return false };
        let computed = digest(&salt_bytes, given);
        if stored.len() != computed.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in stored.iter().zip(computed.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

fn digest(salt: &[u8], password: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(salt);
    h.update(password.as_bytes());
    h.finalize().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}
```

Add `#[serde(default)] pub lock: Option<Lock>` to `TableDoc`, and the three methods to `impl TableDoc`:

```rust
    pub fn open_seats(&self) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, s)| matches!(s, SeatDoc::Open))
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
    }

    pub fn is_full(&self) -> bool {
        self.open_seats().is_empty()
    }

    /// True when the table has no lock, or the lock matches.
    pub fn check_password(&self, given: &str) -> bool {
        self.lock.as_ref().is_none_or(|l| l.matches(given))
    }
```

Add to `TableView`: `pub locked: bool` and `pub winner: Option<PlayerId>`, filled in `view()` as `locked: self.lock.is_some()` and `winner: self.turn.and_then(|t| t.winner).map(PlayerId::new)`. Every `SeatDoc::Human { user }` pattern in `table.rs` and `tables.rs` becomes `SeatDoc::Human { user, .. }`; every construction gains `name: None` (the lib test helpers included).

In `tables.rs`: `const TABLE_SCHEMA: u32 = 2;` and the six arms at the TOP of `check_shape`, before the existing `players` check:

```rust
    if doc.schema > TABLE_SCHEMA {
        return Err(format!(
            "schema {} — this server reads up to {TABLE_SCHEMA}; a newer server wrote it",
            doc.schema
        ));
    }
    let open = doc.seats.iter().any(|s| matches!(s, SeatDoc::Open));
    if open && doc.status != TableStatus::Open {
        return Err(format!("an open seat at a table whose status is {:?}", doc.status));
    }
    if doc.status == TableStatus::Open && !open {
        return Err("status open with no open seat".to_string());
    }
    if let Some(lock) = &doc.lock {
        if lock.salt.len() != 32 || !lock.salt.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("the lock's salt is not 32 hex characters".to_string());
        }
        if lock.hash.len() != 64 || !lock.hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("the lock's hash is not 64 hex characters".to_string());
        }
    }
    for (i, seat) in doc.seats.iter().enumerate() {
        if let SeatDoc::Human { name: Some(name), .. } = seat {
            if let Err(why) = check_name(name) {
                return Err(format!("seat {i}'s name: {why}"));
            }
        }
    }
    if let Some(TurnMirror { winner: Some(w), .. }) = doc.turn {
        if usize::from(w) >= usize::from(doc.players) {
            return Err(format!("winner {w} at a {}-seat table", doc.players));
        }
    }
```

with, beside it, the ONE name rule the write paths reuse in Tasks 3 and 4:

```rust
/// The display-name rule, applied on WRITE (create, join, keep) and on READ
/// (`check_shape`), so a document this server wrote is never one it refuses.
pub(crate) fn check_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err("a name is 1 to 64 characters".to_string());
    }
    if name.chars().any(char::is_control) {
        return Err("a name has no control characters".to_string());
    }
    Ok(name.to_string())
}
```

In `games.rs`, `mirror_of`:

```rust
pub(crate) fn mirror_of(repl: &Repl) -> TurnMirror {
    let over = repl.session.is_over();
    TurnMirror {
        to_move: repl.session.state().current_player.0,
        over,
        winner: over.then(|| repl.session.final_result().winner.0),
    }
}
```

- [ ] **Step 4: Add the shape-arm cells in `tables.rs`**

Beside the existing `a_malformed_table_document_refuses_typed_and_leaves_the_server_alive` cell, add one that feeds `check_shape` hand-built documents (start from a valid `two_seat`-shaped `TableDoc` built in Rust, not JSON) and asserts each refusal's sentence: `schema: 3` → "newer server"; an `Open` seat at `Dealing` → "open seat"; `Open` status with no open seat → "no open seat"; a 31-character salt → "salt"; a name of 65 characters → "1 to 64"; a name with `\u{7}` → "control"; `winner: Some(2)` at 2P → "winner 2". Also extend the existing `the_turn_mirror_follows_every_persisted_move` cell (or add `the_mirror_names_the_winner_once_over`): play a game to the end through `playing(…)` + the REPL until `is_over()`, call `persist`, read the table document, assert `turn.winner == Some(final_result().winner.0)`, and that before the end it read `None`.

- [ ] **Step 5: Run the suite**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: lib `115 + 6` (the four `table::` cells, the shape-arm cell, the winner cell), gate `2`, wire `5` — the wire corpus is NOT yet regenerated (no served shape changed: `locked`/`winner`/`name` are added in Task 3's regeneration together with the create re-shape, so the diff is read once); clippy clean.

- [ ] **Step 6: Commit**

```bash
git add crates/yokohama-web/Cargo.toml Cargo.lock crates/yokohama-web/src/table.rs crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs
git commit -m "feat(web): the table document widens — an open seat kind and an open status, a per-seat display name, a salted-hash lock (sha2, R-2.8) compared in constant time, the winner on the turn mirror, schema 2 with the schema arm and five shape arms on the locked path (spec §2)"
```

---

### Task 3: Create re-shaped — open seats, the label, the drawn id, the display name, the password (spec §3.1, §3.4)

**Files:**
- Modify: `crates/yokohama-web/src/tables.rs` (`SeatSpec`, `NewGame`, `config_of`, `create`, the test helpers), `crates/yokohama-web/src/games.rs` (`GameError`), `crates/yokohama-web/src/lib.rs` (`ApiError`, the `From` impl), `crates/yokohama-web/tests/wire_fixtures.rs` (`new_game_body`), `web/src/wire/*.json` (regenerated)
- Test: `crates/yokohama-web/src/tables.rs`

**Interfaces:**
- Produces: `SeatSpec { user: Option<String>, bot: bool, open: bool }`; `NewGame { players, colors, start_seat, seed, seats, name: Option<String>, id: Option<String>, display_name: Option<String>, password: Option<String> }` (all `deny_unknown_fields`); `GameError::Open`, `GameError::Playing`, `OPEN_SENTENCE = "this table is still open"`, `PLAYING_SENTENCE = "this table is playing"`; `ApiError::open()`, `ApiError::playing()` (409 with `table_status` `"open"` / `"playing"`); `create` writes `TableStatus::Open` when any seat is open.

- [ ] **Step 1: Write the failing cells**

In `tables.rs`'s `mod tests`, first re-point the helpers per R-plan-3 (in `games.rs`'s `test_support`):

```rust
    pub(crate) fn req_seeded(id: &str, seed: u64) -> NewGame {
        NewGame {
            players: 2,
            colors: vec![PlayerColor::Red, PlayerColor::Blue],
            start_seat: 0,
            seed: Some(WireSeed(seed)),
            seats: vec![
                SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
                SeatSpec { user: None, bot: true, open: false },
            ],
            name: None,
            id: Some(id.to_string()),
            display_name: None,
            password: None,
        }
    }
```

(`req` and `two_people` are unchanged in body and now produce a request whose ID is the argument.) Then the cells:

```rust
    #[test]
    fn create_lands_open_when_any_seat_is_open_and_dealing_otherwise() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("open3");
        r.players = 3;
        r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green];
        r.seats = vec![
            SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: false, open: true },
            SeatSpec { user: None, bot: false, open: true },
        ];
        let v = games.create(&r, &red).expect("an open table");
        assert_eq!(v.status, TableStatus::Open);
        assert_eq!(v.seats[1], SeatDoc::Open);
        assert_eq!(v.chosen, vec![false, false, false]);
        let v = games.create(&req("bot2"), &red).expect("a bot table");
        assert_eq!(v.status, TableStatus::Dealing, "a 2P bot table is full at create");
    }

    #[test]
    fn create_refuses_the_new_arms_in_order_and_an_open_seat_is_not_human() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("o");
        r.seats[1] = SeatSpec { user: None, bot: true, open: true };
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("exactly one"));
        let mut r = req("o");
        r.seats = vec![
            SeatSpec { user: None, bot: false, open: true },
            SeatSpec { user: None, bot: true, open: false },
        ];
        let e = games.create(&r, &red).unwrap_err().to_string();
        assert!(e.contains("human"), "open seats and a bot: no human yet: {e}");
        let mut r = req("o");
        r.name = Some("x".repeat(65));
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("1 to 64"));
        let mut r = req("o");
        r.display_name = Some("\u{7}".into());
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("control"));
        let mut r = req("o");
        r.password = Some("   ".into());
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("at least one character"));
        let mut r = req("o");
        r.id = Some("bad id".into());
        assert!(games.create(&r, &red).unwrap_err().to_string().contains("not a usable"));
    }

    #[test]
    fn the_name_is_a_label_the_id_is_drawn_and_the_display_name_lands_on_the_creators_seats() {
        let games = games();
        let red = who("red@t.test");
        let mut r = two_people("ignored", CORPUS_SEED);
        r.id = None;
        r.name = Some("Thursday's game".into());
        r.display_name = Some("Jeremy".into());
        r.seats[1] = SeatSpec { user: Some("red@t.test".into()), bot: false, open: false };
        let v = games.create(&r, &red).expect("a table");
        assert!(v.id.starts_with("sim-"), "drawn: {}", v.id);
        assert_eq!(v.name.as_deref(), Some("Thursday's game"));
        for s in &v.seats {
            assert_eq!(s, &SeatDoc::Human { user: "red@t.test".into(), name: Some("Jeremy".into()) });
        }
        let mut r = two_people("named", CORPUS_SEED);
        r.display_name = Some("Jeremy".into());
        let v = games.create(&r, &red).expect("a table");
        assert_eq!(v.seats[1], SeatDoc::Human { user: "blue@t.test".into(), name: None }, "not blue's");
    }

    #[test]
    fn a_password_at_create_locks_the_table_and_the_view_says_only_that() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("locked");
        r.password = Some(" sesame ".into());
        let v = games.create(&r, &red).expect("a table");
        assert!(v.locked);
        let (doc, _) = games.read_table("locked").unwrap().unwrap();
        assert!(doc.check_password("sesame"), "trimmed on the way in");
        assert!(!doc.check_password(" sesame "));
    }
```

Extend `create_refuses_in_the_order_a_client_reads_and_binds_the_creator`: after the `"human"` assertion, add the three new arms in order — a bad `name`, a bad `password`, a bad `id` — each with its sentence, and keep the rest of the cell as it is (its `req("order")` now sets `id: "order"`, so the store read at the end still finds nothing under `Doc::Record, "order"`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p yokohama-web --lib tables::`
Expected: compile errors on `open`, `id`, `display_name`, `password`.

- [ ] **Step 3: Re-shape the request and the create**

In `tables.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatSpec {
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub bot: bool,
    /// An unfilled seat (M6.2, R-2.7): anyone signed in may take it.
    #[serde(default)]
    pub open: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewGame {
    pub players: u8,
    pub colors: Vec<PlayerColor>,
    #[serde(default)]
    pub start_seat: u8,
    #[serde(default)]
    pub seed: Option<WireSeed>,
    pub seats: Vec<SeatSpec>,
    /// A LABEL since M6.2 (R-2.4): free text, 1–64 characters. Never the id.
    #[serde(default)]
    pub name: Option<String>,
    /// The storage id, under `is_safe_id`; absent, the server draws one. Tests
    /// and the fixture harness set it; the browser never does.
    #[serde(default)]
    pub id: Option<String>,
    /// Written onto every seat the creator holds (R-2.3).
    #[serde(default)]
    pub display_name: Option<String>,
    /// The table password (R-2.1); stored as a `Lock`, never served.
    #[serde(default)]
    pub password: Option<String>,
}
```

`config_of` — the seat loop and the two arms after it become:

```rust
    let display_name = match &req.display_name {
        Some(raw) => Some(check_name(raw).map_err(|e| format!("display name: {e}"))?),
        None => None,
    };
    let mut seats: Vec<SeatDoc> = Vec::with_capacity(n);
    for (i, spec) in req.seats.iter().enumerate() {
        match (&spec.user, spec.bot, spec.open) {
            (Some(raw), false, false) => {
                let user = User::parse(raw).map_err(|e| format!("seat {i}: {e}"))?;
                let name = if user.as_str() == creator.as_str() { display_name.clone() } else { None };
                seats.push(SeatDoc::Human { user: user.as_str().to_string(), name });
            }
            (None, true, false) => seats.push(SeatDoc::Bot),
            (None, false, true) => seats.push(SeatDoc::Open),
            _ => return Err(format!("seat {i}: exactly one of user, bot or open")),
        }
    }
    // The wizard's rule, kept, with its predicate widened for open seats: a
    // table with NO human seat is refused here, before the creator check —
    // open seats are not human yet.
    if !seats.iter().any(|s| matches!(s, SeatDoc::Human { .. })) {
        return Err("at least one seat must stay human".to_string());
    }
```

(the creator check and the bot-at-3–4P check follow unchanged, matching `SeatDoc::Human { user, .. }`). After them, the new refusals in order:

```rust
    if let Some(raw) = &req.name {
        check_name(raw).map_err(|e| format!("name: {e}"))?;
    }
    if let Some(raw) = &req.password {
        let p = raw.trim();
        if p.is_empty() {
            return Err("a password is at least one character".to_string());
        }
        if p.chars().count() > 64 {
            return Err("a password is at most 64 characters".to_string());
        }
    }
```

In `create`, the id and the document:

```rust
        let id = match &req.id {
            Some(id) if !is_safe_id(id) => {
                return Err(GameError::BadRequest(format!("{id:?} is not a usable game id")))
            }
            Some(id) => id.clone(),
            None => format!("sim-{:016x}", draw_seed()),
        };
        …
        let status = if seats.iter().any(|s| matches!(s, SeatDoc::Open)) {
            TableStatus::Open
        } else {
            TableStatus::Dealing
        };
        let doc = TableDoc {
            schema: TABLE_SCHEMA,
            id,
            name: req.name.as_deref().map(str::trim).map(str::to_string),
            …,
            status,
            turn: None,
            lock: req.password.as_deref().map(str::trim).map(Lock::new),
        };
```

(`"not a usable game name"` becomes `"not a usable game id"`; the existing cell that asserted the old sentence — find it with `grep -n 'usable' crates/yokohama-web/src/*.rs` — is updated to the new one.)

In `games.rs`:

```rust
pub enum GameError {
    NotFound(String),
    /// A session route at a table that has not started (M6.1).
    Dealing,
    /// A session route at a table still filling its seats (M6.2, spec §3.4).
    Open,
    /// A table-phase route (join, deal, keep) at a table already playing or
    /// finished (M6.2, spec §3.4).
    Playing,
    Forbidden(String),
    Conflict(String),
    BadRequest(String),
    Server(String),
}
pub const DEALING_SENTENCE: &str = "this table is still dealing";
pub const OPEN_SENTENCE: &str = "this table is still open";
pub const PLAYING_SENTENCE: &str = "this table is playing";
```

with the two new `Display` arms writing their constants. In `lib.rs`, beside `ApiError::dealing()`:

```rust
    pub fn open() -> Self {
        Self { status: StatusCode::CONFLICT, message: games::OPEN_SENTENCE.to_string(), table_status: Some("open") }
    }
    pub fn playing() -> Self {
        Self { status: StatusCode::CONFLICT, message: games::PLAYING_SENTENCE.to_string(), table_status: Some("playing") }
    }
```

and in `From<GameError>`: `E::Open => Self::open(), E::Playing => Self::playing(),`. In `tables.rs`, `deal`'s `if doc.status == TableStatus::Playing { return Err(GameError::Conflict("this table is playing".into())) }` becomes `return Err(GameError::Playing)`, and both `deal` and `keep` gain, before it, `if doc.status == TableStatus::Open { return Err(GameError::Open); }`. In `handle` (`games.rs`), the `Some((doc, _)) if doc.status == TableStatus::Dealing => Err(GameError::Dealing)` arm gains a sibling `Some((doc, _)) if doc.status == TableStatus::Open => Err(GameError::Open)` BEFORE it.

- [ ] **Step 4: Regenerate the corpus and read the diff**

In `tests/wire_fixtures.rs`, `new_game_body` gains `"id": "wire"` (the name stays `"wire"` as the label). Then:

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire`
Expected: every `TableView`-carrying fixture changed (`games.json`, `table.json`, `table_dealing.json`, `deal.json`, `deal_waiting.json`, and any 4xx fixture embedding a table), each gaining `"locked": false`, `"winner": null` and `"name": null` on each human seat; NOTHING else changed (a changed `ClientState` fixture is a finding). `cd web && npm run typecheck` FAILS now (the client's `types.ts` does not declare the new keys) — expected until Task 6; record it in the ledger and do not "fix" it by widening the types here.

- [ ] **Step 5: Run the suite and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: lib `121 + 4` new cells, the order cell extended; gate `2`; wire `5`; clippy clean.

```bash
git add crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/wire_fixtures.rs web/src/wire
git commit -m "feat(web): create re-shaped — an open seat kind, the name as a label and the id drawn or client-supplied under is_safe_id, the creator's display name, the table password as a lock; open lands open and a full table deals; the two status arms Open and Playing beside Dealing, each a 409 carrying its status (spec §3.1, §3.4)"
```

---

### Task 4: Join, the per-table locks, the name on keep (spec §3.2, §3.4, §3.5)

**Files:**
- Modify: `crates/yokohama-web/src/tables.rs` (`join`, `keep`, the locks), `crates/yokohama-web/src/games.rs` (`Games.locks`, `lock_for`), `crates/yokohama-web/src/lib.rs` (the route, `Join`, `Keep.name`)
- Test: `crates/yokohama-web/src/tables.rs`, `crates/yokohama-web/src/lib.rs`

**Interfaces:**
- Produces: `Games::join(&self, id: &str, user: &User, seat: PlayerId, name: Option<&str>, password: Option<&str>) -> Result<TableView, GameError>`; `Games::keep(&self, id, user, seat, card, name: Option<&str>)`; `Games::lock_for(&self, id: &str) -> Arc<Mutex<()>>`; `POST /api/games/{id}/join` with body `Join { seat: u8, name: Option<String>, password: Option<String> }` (`deny_unknown_fields`) → 200 `TableView`; `Keep { seat, card, name: Option<String> }`.

- [ ] **Step 1: Write the failing cells**

In `tables.rs`'s `mod tests`, a helper and the cells:

```rust
    fn open_two(games: &Games, id: &str, password: Option<&str>) -> TableView {
        let mut r = req(id);
        r.seats[1] = SeatSpec { user: None, bot: false, open: true };
        r.password = password.map(str::to_string);
        games.create(&r, &who("red@t.test")).expect("an open table")
    }

    #[test]
    fn join_refuses_in_the_order_a_client_reads() {
        let games = games();
        let blue = who("blue@t.test");
        let s1 = PlayerId::new(1);
        assert!(matches!(games.join("nope", &blue, s1, None, None), Err(GameError::NotFound(_))));
        open_two(&games, "locked", Some("sesame"));
        assert!(matches!(games.join("locked", &blue, PlayerId::new(2), None, None), Err(GameError::BadRequest(_))));
        let e = games.join("locked", &blue, s1, Some(&"x".repeat(65)), Some("sesame")).unwrap_err();
        assert!(e.to_string().contains("1 to 64"), "{e}");
        let e = games.join("locked", &blue, PlayerId::new(0), None, Some("sesame")).unwrap_err();
        assert!(matches!(e, GameError::Conflict(_)) && e.to_string().contains("taken"), "{e}");
        let e = games.join("locked", &blue, s1, None, None).unwrap_err();
        assert!(matches!(e, GameError::Forbidden(_)) && e.to_string().contains("locked"), "{e}");
        let e = games.join("locked", &blue, s1, None, Some("open")).unwrap_err();
        assert!(matches!(e, GameError::Forbidden(_)) && e.to_string().contains("wrong password"), "{e}");
        let v = games.join("locked", &blue, s1, Some(" Alex "), Some("sesame")).expect("seated");
        assert_eq!(v.seats[1], SeatDoc::Human { user: "blue@t.test".into(), name: Some("Alex".into()) });
        assert_eq!(v.my_seats, vec![s1]);
        assert_eq!(v.status, TableStatus::Dealing, "the filling join flips");
        assert!(matches!(games.join("locked", &blue, s1, None, Some("sesame")), Err(GameError::Dealing)));
    }

    #[test]
    fn the_filling_join_writes_the_seat_and_the_flip_in_one_write() {
        let store = Arc::new(CountingStore::default());
        let games = Games::new(Arc::new(Rules::standard()), Arc::clone(&store) as Arc<dyn Store>);
        open_two(&games, "one", None);
        let before = store.writes();
        games.join("one", &who("blue@t.test"), PlayerId::new(1), None, None).expect("seated");
        assert_eq!(store.writes() - before, 1, "one write for the seat AND the flip");
        let (doc, _) = games.read_table("one").unwrap().unwrap();
        assert_eq!(doc.status, TableStatus::Dealing);
        assert!(doc.open_seats().is_empty());
    }

    #[test]
    fn a_person_may_take_two_seats_and_a_named_seat_is_taken() {
        let games = games();
        let red = who("red@t.test");
        let mut r = req("three");
        r.players = 3;
        r.colors = vec![PlayerColor::Red, PlayerColor::Blue, PlayerColor::Green];
        r.seats = vec![
            SeatSpec { user: Some("red@t.test".into()), bot: false, open: false },
            SeatSpec { user: Some("green@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: false, open: true },
        ];
        games.create(&r, &red).expect("a table");
        let e = games.join("three", &red, PlayerId::new(1), None, None).unwrap_err();
        assert!(e.to_string().contains("taken"), "a named seat is Human and therefore taken: {e}");
        let v = games.join("three", &red, PlayerId::new(2), None, None).expect("a second seat");
        assert_eq!(v.my_seats, vec![PlayerId::new(0), PlayerId::new(2)]);
    }

    #[test]
    fn keep_writes_the_name_when_the_seat_has_none_and_leaves_a_set_one_alone() {
        let games = games();
        let red = who("red@t.test");
        let blue = who("blue@t.test");
        let mut r = two_people("kn", CORPUS_SEED);
        r.display_name = Some("Jeremy".into());
        games.create(&r, &red).expect("a table");
        games.keep("kn", &blue, PlayerId::new(1), 1, Some("Alex")).expect("blue keeps with a name");
        let v = games.keep("kn", &red, PlayerId::new(0), 1, Some("Not Jeremy")).expect("red keeps");
        assert_eq!(v.seats[0], SeatDoc::Human { user: "red@t.test".into(), name: Some("Jeremy".into()) });
        assert_eq!(v.seats[1], SeatDoc::Human { user: "blue@t.test".into(), name: Some("Alex".into()) });
        let e = games.keep("kn", &red, PlayerId::new(0), 1, Some("\u{7}")).unwrap_err();
        assert!(matches!(e, GameError::Playing) || e.to_string().contains("control"), "a bad name is 400 before anything else: {e}");
    }

    #[test]
    fn a_finalize_at_one_table_does_not_block_a_join_at_another() {
        // Reuse the gated store beside `a_delete_cannot_interleave_with_the_keep_that_finalizes`:
        // park table A's RECORD write inside finalize, then join at table B from another
        // thread and expect it to answer before A's gate opens.
        …the gated-store construction from that cell, verbatim…
        let games = Arc::new(Games::new(Arc::new(Rules::standard()), Arc::clone(&gated) as Arc<dyn Store>));
        open_two(&games, "b", None);
        let mut a = two_people("a", CORPUS_SEED);
        games.create(&a, &who("red@t.test")).unwrap();
        games.keep("a", &who("red@t.test"), PlayerId::new(0), 1, None).unwrap();
        let g = Arc::clone(&games);
        let finalizer = std::thread::spawn(move || g.keep("a", &who("blue@t.test"), PlayerId::new(1), 1, None));
        gated.wait_until_parked(Doc::Record, "a");          // finalize is inside the record write
        let joined = games.join("b", &who("blue@t.test"), PlayerId::new(1), None, None);
        assert!(joined.is_ok(), "the join at B completed while A's finalize was parked: {joined:?}");
        gated.open(Doc::Record, "a");
        finalizer.join().unwrap().expect("A finalizes after the gate opens");
    }
```

`CountingStore` goes in `test_support`: a wrapper over `MemStore` implementing `Store` by delegation with an `AtomicUsize` incremented in `write_if_version`, and `writes()` reading it. The gated store's `wait_until_parked`/`open` are whatever that existing cell's two-gate store already exposes — read it and use its names; if it parks by doc kind only, park the record kind.

In `lib.rs`'s `mod tests`: `join_is_a_route_that_answers_200_with_the_table_and_400_on_an_unknown_key`, posting `{"seat":1,"name":"Alex","password":"sesame"}` to a locked open table created by another header and asserting 200 with `my_seats: [1]`, and `{"seat":1,"nope":1}` → 400 in the API's error shape; and `keep_carries_a_name` posting `{"seat":0,"card":1,"name":"Jeremy"}`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p yokohama-web --lib tables::join`
Expected: compile errors — no `join`, `keep` has four parameters.

- [ ] **Step 3: The locks and the join**

In `games.rs`, add the field `pub(crate) locks: Mutex<HashMap<GameId, Arc<Mutex<()>>>>` (initialized empty in `new`) and:

```rust
    /// The table's OWN lock (R-2.9): `join`, `keep`, `settle`, `finalize`,
    /// `flip_to_playing` and `delete` hold it; `create` holds `table_lock`
    /// (global) for id allocation and nothing else.
    pub(crate) fn lock_for(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().expect("games lock");
        Arc::clone(locks.entry(id.to_string()).or_insert_with(|| Arc::new(Mutex::new(()))))
    }
```

In `tables.rs`, replace every `let _serialized = self.table_lock.lock().expect("games lock");` in `keep`, `settle` and `delete` with:

```rust
        let lock = self.lock_for(id);
        let _serialized = lock.lock().expect("table lock");
```

(`create` keeps `table_lock`; `settle_locked`, `finalize`, `flip_to_playing` are called with the table's lock held, as their docs say.) Then `join`:

```rust
    pub fn join(
        &self,
        id: &str,
        user: &User,
        seat: PlayerId,
        name: Option<&str>,
        password: Option<&str>,
    ) -> Result<TableView, GameError> {
        let lock = self.lock_for(id);
        let _serialized = lock.lock().expect("table lock");
        let (mut doc, version) = self
            .read_table(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        match doc.status {
            TableStatus::Open => {}
            TableStatus::Dealing => return Err(GameError::Dealing),
            TableStatus::Playing => return Err(GameError::Playing),
        }
        let n = usize::from(seat.0);
        if n >= doc.seats.len() {
            return Err(GameError::BadRequest(format!(
                "seat {} at a {}-seat table",
                seat.0, doc.players
            )));
        }
        let name = match name {
            Some(raw) => Some(check_name(raw).map_err(GameError::BadRequest)?),
            None => None,
        };
        if !matches!(doc.seats[n], SeatDoc::Open) {
            return Err(GameError::Conflict(format!("seat {} is taken", seat.0)));
        }
        if doc.lock.is_some() {
            match password.map(str::trim) {
                None | Some("") => {
                    return Err(GameError::Forbidden(
                        "this table is locked — send its password".to_string(),
                    ))
                }
                Some(p) if !doc.check_password(p) => {
                    return Err(GameError::Forbidden("wrong password for this table".to_string()))
                }
                Some(_) => {}
            }
        }
        doc.seats[n] = SeatDoc::Human { user: user.as_str().to_string(), name };
        if doc.is_full() {
            doc.status = TableStatus::Dealing;
        }
        match self.write_table(&doc, Some(version)) {
            Ok(_) => Ok(doc.view(user)),
            Err(WriteError::Conflict { .. }) => Err(GameError::Conflict(format!(
                "{id:?} changed while this join was in flight"
            ))),
            Err(WriteError::Io(e)) => Err(GameError::Server(e)),
        }
    }
```

`keep` gains `name: Option<&str>` after `card`, checks it with `check_name` (400) right after the card check, and writes it only when the seat's `name` is `None`:

```rust
        let name = match name {
            Some(raw) => Some(check_name(raw).map_err(GameError::BadRequest)?),
            None => None,
        };
        doc.keeps[usize::from(n)] = Some(card);
        if let (Some(new), SeatDoc::Human { name: slot @ None, .. }) = (name, &mut doc.seats[usize::from(n)]) {
            *slot = Some(new);
        }
```

In `lib.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    pub seat: u8,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

async fn join(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<Join>,
) -> ApiResult<crate::table::TableView> {
    let seat = yokohama_core::PlayerId::try_new(req.seat)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(state.games.join(&id, &user, seat, req.name.as_deref(), req.password.as_deref())?))
}
```

`Keep` gains `#[serde(default)] pub name: Option<String>` and the `keep` handler passes `req.name.as_deref()`. The router gains `.route("/api/games/{id}/join", post(join))` after the `keep` line. Every other caller of `keep` (the gate tests, the fixture walk) passes `None`.

- [ ] **Step 4: Run the suite, the regeneration (expect NO diff), and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: lib `125 + 7`; the four M6.1 race cells green as they are; gate `2`; wire `5`.

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire`
Expected: NO file changed — nothing this task serves is on the walk yet (Task 6 puts it there). A changed file is a finding.

```bash
git add crates/yokohama-web/src/tables.rs crates/yokohama-web/src/games.rs crates/yokohama-web/src/lib.rs crates/yokohama-web/tests/play_a_whole_game.rs crates/yokohama-web/tests/wire_fixtures.rs
git commit -m "feat(web): POST …/{id}/join fills an open seat under the table's own lock — refused in the order a client reads, the filling join flips to dealing in the same write; the lock splits by table so a finalize at one table no longer blocks a join at another (R-2.9); keep carries the display name for a seat named by email (spec §3.2, §3.4, §3.5)"
```

---

### Task 5: The listing's scopes and cursor; the open table readable by anyone; the route sweep (spec §3.3, R-2.7, §6)

**Files:**
- Modify: `crates/yokohama-web/src/tables.rs` (`list`, `table`), `crates/yokohama-web/src/lib.rs` (`ListQuery`, `list_games`, `get_table`, `GamesList.next`), `crates/yokohama-web/tests/wire_fixtures.rs` (regenerate)
- Test: `crates/yokohama-web/src/tables.rs`, `crates/yokohama-web/src/lib.rs`

**Interfaces:**
- Produces: `Scope::{Mine, Open}` (`FromStr` over `"mine"` / `"open"`); `Games::list(&self, user: &User, scope: Scope, limit: usize, before: Option<(u64, String)>) -> Result<(Vec<TableView>, Option<String>), String>`; `GamesList { games: Vec<TableView>, next: Option<String> }`; `ListQuery { scope: Option<String>, limit: Option<usize>, before: Option<String> }` (`deny_unknown_fields`); `Games::table_for_anyone(&self, id, user) -> Result<TableDoc, GameError>` — at `Open` any caller, otherwise `table`'s rule.

- [ ] **Step 1: Write the failing cells**

In `tables.rs`:

```rust
    #[test]
    fn the_two_scopes_list_by_construction_newest_first_with_the_winner_once_over() {
        let games = games();
        let red = who("red@t.test");
        let blue = who("blue@t.test");
        open_two(&games, "o1", None);
        games.create(&two_people("mine-only", CORPUS_SEED), &red).unwrap();
        let mut r = req("o2");
        r.seats = vec![
            SeatSpec { user: Some("blue@t.test".into()), bot: false, open: false },
            SeatSpec { user: None, bot: false, open: true },
        ];
        games.create(&r, &blue).unwrap();
        let (mine, next) = games.list(&red, Scope::Mine, 20, None).unwrap();
        assert_eq!(mine.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["mine-only", "o1"]);
        assert!(next.is_none());
        let (open, _) = games.list(&red, Scope::Open, 20, None).unwrap();
        assert_eq!(open.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(), vec!["o2", "o1"]);
        assert!(open.iter().all(|t| t.status == TableStatus::Open));
        assert!(open[0].my_seats.is_empty() && open[1].my_seats == vec![PlayerId::new(0)]);
    }

    #[test]
    fn the_cursor_pages_without_gap_or_overlap() {
        let games = games();
        let red = who("red@t.test");
        for i in 0..5 {
            games.create(&req(&format!("p{i}")), &red).unwrap();
        }
        let mut seen: Vec<String> = Vec::new();
        let mut before = None;
        loop {
            let (page, next) = games.list(&red, Scope::Mine, 2, before.clone()).unwrap();
            assert!(page.len() <= 2);
            seen.extend(page.iter().map(|t| t.id.clone()));
            match next {
                Some(cursor) => {
                    let (at, id) = cursor.split_once(',').expect("created_at,id");
                    before = Some((at.parse::<u64>().unwrap(), id.to_string()));
                }
                None => break,
            }
        }
        seen.sort();
        assert_eq!(seen, (0..5).map(|i| format!("p{i}")).collect::<Vec<_>>());
    }

    #[test]
    fn a_listing_reads_no_record_and_builds_no_session() {
        struct RecordPanics(MemStore);
        impl Store for RecordPanics {
            fn list(&self, doc: Doc) -> Result<Vec<GameId>, String> { self.0.list(doc) }
            fn read(&self, doc: Doc, id: &str) -> Result<Option<(String, Version)>, String> {
                assert_ne!(doc, Doc::Record, "the listing read a RECORD for {id}");
                self.0.read(doc, id)
            }
            fn write_if_version(&self, doc: Doc, id: &str, json: &str, expected: Option<Version>) -> Result<Version, WriteError> {
                self.0.write_if_version(doc, id, json, expected)
            }
            fn delete(&self, id: &str) -> Result<(), String> { self.0.delete(id) }
        }
        let games = Games::new(Arc::new(Rules::standard()), Arc::new(RecordPanics(MemStore::default())));
        let red = who("red@t.test");
        open_two(&games, "o", None);
        games.list(&red, Scope::Mine, 20, None).unwrap();
        games.list(&red, Scope::Open, 20, None).unwrap();
    }

    #[test]
    fn an_open_table_is_readable_by_a_stranger_and_a_dealing_one_is_not() {
        let games = games();
        let stranger = who("nobody@t.test");
        open_two(&games, "o", None);
        let doc = games.table_for_anyone("o", &stranger).expect("an open table reads");
        assert!(doc.view(&stranger).my_seats.is_empty());
        games.create(&two_people("d", CORPUS_SEED), &who("red@t.test")).unwrap();
        assert!(matches!(games.table_for_anyone("d", &stranger), Err(GameError::Forbidden(_))));
    }
```

In `lib.rs`'s `mod tests`, extend the existing route sweep (find it: `grep -n 'not seated' crates/yokohama-web/src/lib.rs` inside `mod tests`, the cell that walks every session route as a stranger) into `every_session_route_refuses_a_stranger_at_every_status_except_table_and_join_at_open`: build one state with three tables by `red` — open (one open seat), dealing (two people, nobody kept), playing (two people, both kept) — then for each route in the list (`/table`, `/deal`, `/keep` (POST), `/join` (POST), `` (GET the frame), `/input` (POST), `/ports`, `/labs`, `/areas`, `/log`, `/events`, `/at/0`, `/suggest`, `/why`, `/index/orders`, `/result`, `` (DELETE)) and each table, call as `stranger@wire.test` and assert: at open, `/table` is 200 and `/join` (with `{"seat":1}`) is 200, everything else 403 with "not seated" (DELETE 403 too); at dealing and playing everything is 403 (with `/join` 403 as well — it is not admitted where the table read refuses). Then as `red` at the OPEN table: every session route but `/table`, `/join`, DELETE answers 409 with `"status":"open"`. Also `list_games_validates_its_query`: `?scope=nope` 400, `?limit=0` 400, `?before=junk` 400, `?nope=1` 400, `?scope=open&limit=1` 200 with `next` present or null.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p yokohama-web --lib tables::the_two_scopes`
Expected: no `Scope`, no `list`, no `table_for_anyone`.

- [ ] **Step 3: The listing, the cursor, the stranger's read**

In `tables.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Mine,
    Open,
}

impl std::str::FromStr for Scope {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "mine" => Ok(Self::Mine),
            "open" => Ok(Self::Open),
            other => Err(format!("no scope named {other:?} — the two are mine and open")),
        }
    }
}

pub const LIST_LIMIT_DEFAULT: usize = 20;
pub const LIST_LIMIT_MAX: usize = 100;

fn cursor_of(doc: &TableDoc) -> String {
    format!("{},{}", doc.created_at, doc.id)
}

impl Games {
    pub fn list(
        &self,
        user: &User,
        scope: Scope,
        limit: usize,
        before: Option<(u64, String)>,
    ) -> Result<(Vec<TableView>, Option<String>), String> {
        let mut rows: Vec<TableDoc> = Vec::new();
        for id in self.store.list(Doc::Table)? {
            …the existing read-parse-check_shape loop of `list_for`, unchanged…
            let keep = match scope {
                Scope::Mine => !doc.seats_of(user).is_empty(),
                Scope::Open => doc.status == TableStatus::Open,
            };
            if keep {
                rows.push(doc);
            }
        }
        rows.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| a.id.cmp(&b.id)));
        if let Some((at, id)) = before {
            // Strictly AFTER the cursor row in this order: older created_at, or equal
            // created_at with a greater id.
            rows.retain(|d| d.created_at < at || (d.created_at == at && d.id > id));
        }
        let next = (rows.len() > limit).then(|| cursor_of(&rows[limit - 1]));
        rows.truncate(limit);
        Ok((rows.iter().map(|doc| doc.view(user)).collect(), next))
    }

    /// `table`'s rule with R-2.7's one widening: an OPEN table reads for anyone.
    pub fn table_for_anyone(&self, id: &str, user: &User) -> Result<TableDoc, GameError> {
        let (doc, _) = self
            .settle(id)?
            .ok_or_else(|| GameError::NotFound(format!("no game named {id:?}")))?;
        if doc.status == TableStatus::Open || !doc.seats_of(user).is_empty() {
            return Ok(doc);
        }
        Err(GameError::Forbidden("you are not seated at this table".to_string()))
    }
}
```

`list_for(user)` becomes `self.list(user, Scope::Mine, LIST_LIMIT_MAX, None).map(|(rows, _)| rows)` (its callers in tests keep working) — or is deleted if nothing but tests calls it. In `lib.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub before: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GamesList {
    pub games: Vec<crate::table::TableView>,
    pub next: Option<String>,
}

async fn list_games(
    identity::Identity(user): identity::Identity,
    State(state): State<Arc<AppState>>,
    QueryArgs(q): QueryArgs<ListQuery>,
) -> ApiResult<GamesList> {
    use crate::tables::{Scope, LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX};
    let scope = match q.scope.as_deref() {
        None => Scope::Mine,
        Some(s) => s.parse::<Scope>().map_err(ApiError::bad_request)?,
    };
    let limit = q.limit.unwrap_or(LIST_LIMIT_DEFAULT);
    if limit == 0 || limit > LIST_LIMIT_MAX {
        return Err(ApiError::bad_request(format!("limit {limit} — 1 to {LIST_LIMIT_MAX}")));
    }
    let before = match q.before.as_deref() {
        None => None,
        Some(c) => {
            let (at, id) = c
                .split_once(',')
                .ok_or_else(|| ApiError::bad_request("before is <created_at>,<id>"))?;
            let at: u64 = at.parse().map_err(|_| ApiError::bad_request("before is <created_at>,<id>"))?;
            Some((at, id.to_string()))
        }
    };
    let (games, next) = state.games.list(&user, scope, limit, before).map_err(ApiError::server_error)?;
    Ok(Json(GamesList { games, next }))
}
```

and `get_table` calls `state.games.table_for_anyone(&id, &user)?` instead of `table`.

- [ ] **Step 4: Regenerate, read the diff, run, commit**

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire`
Expected: `games.json` alone changed — `"next": null` added. Anything else is a finding.

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings`
Expected: lib `132 + 4` plus the sweep and query cells; gate `2`; wire `5`.

```bash
git add crates/yokohama-web/src/tables.rs crates/yokohama-web/src/lib.rs web/src/wire/games.json
git commit -m "feat(web): GET /api/games gains two scopes (mine, open) and a created_at,id cursor over table documents alone; an open table's view reads for any signed-in caller (R-2.7) and nothing else widens — the stranger sweep runs at four statuses (spec §3.3, §6)"
```

---

### Task 6: The wire — the walk through an open table, the new fixtures, the password pin, the client's types (spec §7, §5.7)

**Files:**
- Modify: `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/*.json` (regenerated + new), `web/src/types.ts`, `web/src/api.ts`, `web/src/wire/contract.ts`, `web/src/wire/contract.test.ts`
- Test: `crates/yokohama-web/tests/wire_fixtures.rs`, `web/src/wire/contract.test.ts`

**Interfaces:**
- Produces (client): `SeatDoc = {kind:"human"; user; name: string | null} | {kind:"bot"} | {kind:"open"}`; `TableStatus = "open" | "dealing" | "playing"`; `TableView.locked: boolean`, `.winner: PlayerId | null`, `my_seats` doc changed; `GamesList.next: string | null`; `DealingErrorBody.status: "open" | "dealing" | "playing"`; `SeatRequest = {user} | {bot: true} | {open: true}`; `NewGameRequest` gains `id?`, `display_name?`, `password?`; `JoinRequest { seat: number; name?: string; password?: string }`; `api.join(game, req)`, `api.list(scope?: "mine" | "open", before?: string)`; `wire.tableOpen`, `wire.gamesOpen`, `wire.gamesNext`, `wire.join409NotOpen`, `wire.join409Taken`, `wire.join403Locked`, `wire.join403Password`, `wire.open409`.

- [ ] **Step 1: Extend the walk and add the value pin**

In `wire_fixtures.rs`, a constant beside `SETUP_SEED`: `const WIRE_PASSWORD: &str = "sesame";`. `new_game_body()` becomes a table with ONE open seat, locked:

```rust
fn new_game_body() -> Value {
    json!({
        "players": 2,
        "colors": ["Red", "Blue"],
        "start_seat": 0,
        "seed": SETUP_SEED,
        "seats": [{ "user": WALKER }, { "open": true }],
        "name": "wire",
        "id": "wire",
        "display_name": "Red",
        "password": WIRE_PASSWORD,
    })
}
```

and `walk()` gains, between the create and the first `deal`:

```rust
    let (status, table_open) = api.call_as(Some(STRANGER), "GET", "/api/games/wire/table", None).await;
    assert_eq!(status, StatusCode::OK, "an open table reads for a stranger: {table_open}");
    let games_open = api.call_as(Some(STRANGER), "GET", "/api/games?scope=open", None).await.1;
    let (status, open_409) = api.call("GET", "/api/games/wire", None).await;
    assert_eq!(status, StatusCode::CONFLICT, "{open_409}");
    let (status, join_403_locked) =
        api.call_as(Some(OTHER), "POST", "/api/games/wire/join", Some(json!({ "seat": 1 }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{join_403_locked}");
    let (status, join_403_password) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/join", Some(json!({ "seat": 1, "password": "open" })))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{join_403_password}");
    let (status, join_409_taken) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/join", Some(json!({ "seat": 0, "password": WIRE_PASSWORD })))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{join_409_taken}");
    let (status, joined) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/join",
            Some(json!({ "seat": 1, "name": "Blue", "password": WIRE_PASSWORD })))
        .await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    assert_eq!(joined["status"], "dealing", "the filling join flips: {joined}");
    let (status, join_409_not_open) = api
        .call_as(Some(OTHER), "POST", "/api/games/wire/join", Some(json!({ "seat": 1, "password": WIRE_PASSWORD })))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{join_409_not_open}");
    let games_next = api.get("/api/games?scope=mine&limit=1").await;   // after a SECOND table exists — see below
```

(`table_dealing` — the create's 201 — now shows an OPEN table; rename that binding to `table_created` and keep the fixture name `table_dealing` for the JOIN's 200 reply `joined`, so the committed file keeps meaning "the table as it reads while dealing".) For `games_next`, create a second, unnamed table by `WALKER` earlier in the walk (`{"players":2,…,"seats":[{"user":WALKER},{"bot":true}],"id":"wire2","seed":SETUP_SEED}`) so `scope=mine&limit=1` has a `next`. Push the new fixtures into `out` in the block that lists `table_dealing` etc.: `table_open`, `games_open`, `games_next`, `open_409`, `join_403_locked`, `join_403_password`, `join_409_taken`, `join_409_not_open`. The existing `deal`/`keep`/`table`/`games` steps follow unchanged (the second keep is OTHER's, seat 1, as today).

The value pin, beside `no_committed_payload_carries_the_setup_seed`:

```rust
#[tokio::test]
async fn no_payload_of_this_run_carries_the_password_or_its_hash() {
    let run = corpus().await;                       // this run's payloads, before comparison
    let api = Api::new();
    let (status, created) = api.call("POST", "/api/games", Some(new_game_body())).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    let (doc, _) = api.state.games.store().read(yokohama_web::store::Doc::Table, "wire")
        .expect("a readable store").expect("the table document");
    let doc: Value = serde_json::from_str(&doc).expect("JSON");
    let salt = doc["lock"]["salt"].as_str().expect("the document carries the salt").to_string();
    let hash = doc["lock"]["hash"].as_str().expect("the document carries the hash").to_string();
    assert_eq!(salt.len(), 32);
    assert_eq!(hash.len(), 64);
    // The corpus above was produced by a DIFFERENT Api with a different salt; search it
    // for the password text, and search THIS api's answers for this salt and hash.
    let mut offenders: Vec<String> = Vec::new();
    for (name, value) in &run {
        if serde_json::to_string(value).unwrap().contains(WIRE_PASSWORD) {
            offenders.push(name.clone());
        }
    }
    for (name, uri) in [("created", ""), ("table", "/table"), ("games", "?scope=open")] {
        let v = if uri.is_empty() { created.clone() } else {
            api.call("GET", &format!("/api/games{}", if uri.starts_with('?') { uri.to_string() } else { format!("/wire{uri}") }), None).await.1
        };
        let text = serde_json::to_string(&v).unwrap();
        if text.contains(&salt) || text.contains(&hash) {
            offenders.push(name.to_string());
        }
    }
    assert!(offenders.is_empty(), "the password, its salt or its hash reached: {offenders:?}");
}
```

- [ ] **Step 2: Regenerate and read the diff**

Run: `UPDATE_WIRE_FIXTURES=1 cargo test -p yokohama-web --test wire_fixtures && cargo test -p yokohama-web --test wire_fixtures && git status --short web/src/wire && ls web/src/wire/*.json | wc -l`
Expected: `54 + 8 = 62` files; the eight new ones present; `games.json` and `table_dealing.json` re-cut (the joined seat now carries `"name": "Blue"`, the creator's `"Red"`; `games.json` lists two tables); `deal.json`/`deal_waiting.json`/`table.json` carry the names and `"locked": true`. Report every changed file.

- [ ] **Step 3: The client's types, calls and contract**

`types.ts`: `SeatDoc`, `TableStatus`, `TableView` (+ `locked`, `winner`, the `my_seats` comment rewritten: "Empty for a caller not seated at an OPEN table (R-2.7); never empty at any other status, where a caller with no seat is a 403"), `GamesList.next`, `DealingErrorBody.status: "open" | "dealing" | "playing"`. `api.ts`: `SeatRequest` gains `| { open: true }`, `NewGameRequest` gains `id?: string; display_name?: string; password?: string;`, add `JoinRequest`, `api.join: (game, req: JoinRequest) => call<TableView>("POST", `/api/games/${id(game)}/join`, req)`, and `api.list: (scope: "mine" | "open" = "mine", before?: string) => call<GamesList>("GET", `/api/games?scope=${scope}${before ? `&before=${encodeURIComponent(before)}` : ""}`)`. `contract.ts`: import the eight new files and add them to `wire` (`tableOpen`, `gamesOpen`, `gamesNext`, `open409`, `join403Locked`, `join403Password`, `join409Taken`, `join409NotOpen`), add `"locked"` and `"winner"` to `TABLE`, `"next"` to `GAMES_LIST`, `"name"` to the human seat's keys wherever a seat key set is listed. `contract.test.ts`: a row per new fixture (`TableView` for `tableOpen`; `GamesList` for the two lists with `next` present in `gamesNext`; `DealingErrorBody` for `open409`, `join409NotOpen`; `ApiErrorBody` for the two 403s and `join409Taken`), plus one cell that `tableOpen.my_seats` is `[]` and `tableOpen.seats` contains an `open` seat and a named human seat.

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests '`
Expected: typecheck clean again; `447 + N` (the contract rows).

- [ ] **Step 4: Run everything and commit**

Run: `cargo test -p yokohama-web 2>&1 | grep 'test result' && cargo clippy --workspace --all-targets -- -D warnings && cd web && npm run typecheck && npm test 2>&1 | grep -E 'Tests '`

```bash
git add crates/yokohama-web/tests/wire_fixtures.rs web/src/wire web/src/types.ts web/src/api.ts
git commit -m "test(web): the wire walk goes through an open, locked table — a stranger reads it, a wrong password and a taken seat refuse, the filling join flips it; eight new fixtures, the password/salt/hash pin over this run's own values, and the client's types, calls and key sets follow (spec §7)"
```

---

### Task 7: The client — hash routes, the generalized poll, the lobby as the first screen, the form re-pointed, the harnesses re-pointed (spec §5.1–§5.3, §5.5, §5.6, §5.8)

**Files:**
- Create: `web/src/route.ts`, `web/src/route.test.ts`, `web/src/Lobby.tsx`, `web/src/Lobby.test.tsx`
- Modify: `web/src/signal.ts`, `web/src/signal.test.ts`, `web/src/tables.ts`, `web/src/tables.test.ts`, `web/src/App.tsx`, `web/src/App.test.tsx`, `web/src/NewGame.tsx`, `web/src/NewGame.test.tsx`, `web/src/test/harness.tsx`, `web/e2e/harness.ts`, `web/src/styles.css`
- Test: the `.test.*` beside each

**Interfaces:**
- Produces: `route.ts` — `type Route = {kind:"lobby"} | {kind:"new"} | {kind:"table"; id: string}`, `parseRoute(hash: string): Route`, `hashOf(route: Route): string`, `navigate(route: Route): void`, `useRoute(): Route`; `signal.ts` — `interface Source<T> { key: string; once: () => Promise<T>; stream: string }`, `interface End { reason: "deleted" | "gone" | "refused"; sentence?: string }`, `usePolled<T>(source: Source<T> | null, active: boolean, onValue: (v: T) => void, onEnd?: (end: End) => void): Link`, and `useSignal(game, active, onSignal)` unchanged in contract; `tables.ts` — `seatName` reads `name ?? user` and words an open seat, `tableWord` words `open` and over-with-winner; `Lobby` props `{ me: string; who?: ReactNode }`; `DISPLAY_NAME_KEY = "yokohama.display_name"` exported from `api.ts` with `displayName()`/`setDisplayName()`.

- [ ] **Step 1: `route.ts` with its cells**

`web/src/route.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { hashOf, parseRoute } from "./route";

describe("the hash routes", () => {
  it("parses the three routes and reads anything else as the lobby", () => {
    expect(parseRoute("")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/new")).toEqual({ kind: "new" });
    expect(parseRoute("#/t/sim-0123")).toEqual({ kind: "table", id: "sim-0123" });
    expect(parseRoute("#/t/")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/nope")).toEqual({ kind: "lobby" });
    expect(parseRoute("#/t/a%20b")).toEqual({ kind: "table", id: "a b" });
  });
  it("round-trips through hashOf", () => {
    for (const r of [{ kind: "lobby" }, { kind: "new" }, { kind: "table", id: "x-1" }] as const) {
      expect(parseRoute(hashOf(r))).toEqual(r);
    }
  });
});
```

`web/src/route.ts`:

```ts
import { useEffect, useState } from "react";

export type Route = { kind: "lobby" } | { kind: "new" } | { kind: "table"; id: string };

/** `#/` the lobby, `#/new` the form, `#/t/<id>` a table (R-2.5). Anything
 *  else is the lobby, so a stale or mistyped link lands somewhere useful. */
export function parseRoute(hash: string): Route {
  const path = hash.startsWith("#") ? hash.slice(1) : hash;
  if (path === "/new") return { kind: "new" };
  const m = /^\/t\/([^/]+)$/.exec(path);
  if (m) return { kind: "table", id: decodeURIComponent(m[1]!) };
  return { kind: "lobby" };
}

export function hashOf(route: Route): string {
  switch (route.kind) {
    case "lobby":
      return "#/";
    case "new":
      return "#/new";
    case "table":
      return `#/t/${encodeURIComponent(route.id)}`;
  }
}

export function navigate(route: Route): void {
  const next = hashOf(route);
  if (window.location.hash !== next) window.location.hash = next;
}

/** The route is the hash: read at mount, followed on `hashchange`, so back and
 *  forward work without any state of this component's own. */
export function useRoute(): Route {
  const [route, setRoute] = useState<Route>(() => parseRoute(window.location.hash));
  useEffect(() => {
    const onChange = () => setRoute(parseRoute(window.location.hash));
    window.addEventListener("hashchange", onChange);
    return () => window.removeEventListener("hashchange", onChange);
  }, []);
  return route;
}
```

Run: `cd web && npx vitest run src/route.test.ts`
Expected: PASS.

- [ ] **Step 2: `usePolled` — the hook generalized, its cells moved**

In `signal.ts`, keep `VISIBLE_MS`, `BACKOFF_MS`, `Link`, `nextDelay`, and replace the body of `useSignal` with the generic hook plus a one-line instance:

```ts
export interface Source<T> {
  /** The effect's ONLY dependency beside `active`: a stable string for the
   *  resource (a game id, "lobby"). `once` and the callbacks ride in refs. */
  key: string;
  once: () => Promise<T>;
  /** The resource's stream URL. Carried and UNUSED in M6.2; M6.3 reads it
   *  (its spec §5.1), which is why the shape is fixed here and not there. */
  stream: string;
}

export interface End {
  reason: "deleted" | "gone" | "refused";
  sentence?: string;
}

export function usePolled<T>(
  source: Source<T> | null,
  active: boolean,
  onValue: (v: T) => void,
  onEnd?: (end: End) => void,
): Link {
  const [link, setLink] = useState<Link>("live");
  const failures = useRef(0);
  const once = useRef(source?.once ?? null);
  once.current = source?.once ?? null;
  const latest = useRef(onValue);
  latest.current = onValue;
  const ended = useRef(onEnd);
  ended.current = onEnd;
  const key = source?.key ?? null;
  useEffect(() => {
    setLink("live");
    if (!key || !active) return;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let stopped = false;
    const visible = () => document.visibilityState === "visible";
    const schedule = (ms: number) => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => void ask(), ms);
    };
    const ask = async () => {
      if (stopped || !visible() || !once.current) return;
      try {
        const v = await once.current();
        if (stopped) return;
        failures.current = 0;
        setLink("live");
        latest.current(v);
        schedule(VISIBLE_MS);
      } catch (e) {
        if (stopped) return;
        if (e instanceof ApiError && (e.status === 404 || e.status === 403)) {
          ended.current?.({ reason: e.status === 404 ? "gone" : "refused", sentence: e.message });
          return;
        }
        setLink("reconnecting");
        const wait = nextDelay(failures.current);
        failures.current += 1;
        schedule(wait);
      }
    };
    const onVisibility = () => {
      if (visible()) {
        setLink("refreshing");
        schedule(0);
      } else if (timer) {
        clearTimeout(timer);
        timer = null;
      }
    };
    document.addEventListener("visibilitychange", onVisibility);
    if (visible()) schedule(VISIBLE_MS);
    return () => {
      stopped = true;
      if (timer) clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [key, active]);
  return link;
}

export function useSignal(
  game: string | null,
  active: boolean,
  onSignal: (e: Events) => void,
): Link {
  return usePolled<Events>(
    game ? { key: game, once: () => api.events(game), stream: `/api/games/${encodeURIComponent(game)}/stream` } : null,
    active,
    onSignal,
  );
}
```

(`ApiError` is imported from `./api`.) The 404/403-ends-the-poll arm is what closes the M6.1 LEAVE item for the one-shot half (the stream half is M6.3's); the game screen's consumer (Task 8, `App.tsx`) navigates to the lobby with the sentence on `onEnd`.

`signal.test.ts`: every existing cell stays and passes unchanged through `useSignal`. Add two cells against `usePolled` directly: (a) "a consumer re-rendering with a new `once` closure does not re-arm the timer" — `renderHook` with `initialProps: { n: 0 }`, `once: () => Promise.resolve(n)`, rerender with `n: 1` five times inside one `VISIBLE_MS`, advance to `VISIBLE_MS + 10`, assert exactly ONE call and that it saw the LATEST closure's value (`1`); (b) "a 404 from `once` ends the poll and calls `onEnd` with `gone` and the sentence" — the stub answers `{status: 404, body: {error: "no game named …"}}`, advance past one tick, assert `onEnd` called once with `{reason: "gone", sentence: "no game named …"}` and NO further requests after another `VISIBLE_MS * 3`.

- [ ] **Step 3: `tables.ts` widened, with its cells**

```ts
export function seatName(seat: SeatDoc): string {
  if (seat.kind === "bot") return "the bot";
  if (seat.kind === "open") return "open seat";
  return seat.name ?? seat.user;
}
export function tableWord(t: TableView): string {
  if (t.status === "open") {
    const filled = t.seats.filter((s) => s.kind !== "open").length;
    return `open · ${filled} of ${t.players} seated`;
  }
  if (t.over) return t.winner === null ? "over" : `over · ${holder(t, t.winner)} won`;
  if (t.status === "dealing") return "dealing";
  if (t.to_move === null) return "playing";
  return t.my_seats.includes(t.to_move) ? "your turn" : `waiting on ${holder(t, t.to_move)}`;
}
```

`tables.test.ts` gains: `seatName` prefers the name and falls back to the email (`wire.table.seats[0]` reads "Red", a seat with `name: null` reads its email); "open seat" for `wire.tableOpen.seats[1]`; `tableWord(wire.tableOpen)` is `"open · 1 of 2 seated"`; an over table with `winner: 1` reads `"over · <seatName of seat 1> won"` and with `winner: null` reads `"over"`. (Update the two existing expectations that read emails, since `wire.table`'s seats are now named "Red" and "Blue": `seatNames(wire.table)` is `"Red, Blue"`, and `tableWord(wire.table)` is `"waiting on Red"`.)

- [ ] **Step 4: `Lobby.tsx` with its cells**

```tsx
import { useCallback, useState, type ReactNode } from "react";
import { api, ApiError } from "./api";
import { navigate } from "./route";
import { usePolled } from "./signal";
import { seatNames, tableWord } from "./tables";
import type { GamesList, TableView } from "./types";

type Lists = { mine: GamesList; open: GamesList };

const fetchLists = async (): Promise<Lists> => {
  const [mine, open] = await Promise.all([api.list("mine"), api.list("open")]);
  return { mine, open };
};

export function Lobby({ me, who }: { me: string; who?: ReactNode }) {
  const [lists, setLists] = useState<Lists | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [more, setMore] = useState<TableView[]>([]);
  const link = usePolled<Lists>(
    { key: "lobby", once: fetchLists, stream: "/api/stream" },
    true,
    useCallback((v: Lists) => setLists(v), []),
  );
  const page = async (next: string) => {
    try {
      const page = await api.list("mine", next);
      setMore((m) => [...m, ...page.games]);
      setLists((l) => (l ? { ...l, mine: { ...l.mine, next: page.next } } : l));
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e));
    }
  };
  const row = (t: TableView) => (
    <li key={t.id}>
      <button type="button" className="ghost" onClick={() => navigate({ kind: "table", id: t.id })}>
        {t.name ?? t.id}
      </button>
      <span className="muted table-facts">
        {t.players}P &middot; {tableWord(t)} &middot; {seatNames(t)}
        {t.locked && <span className="meta lock" aria-label="password required"> 🔒</span>}
      </span>
    </li>
  );
  const mine = lists ? [...lists.mine.games, ...more] : [];
  return (
    <div className="setup lobby">
      {who && <div className="who-strip">{who}{link !== "live" && <span className="meta link" role="status">{link}&hellip;</span>}</div>}
      <h2>Lobby</h2>
      <button type="button" className="primary" onClick={() => navigate({ kind: "new" })}>
        New table
      </button>
      <section className="saved">
        <h3>Your games</h3>
        {lists === null ? <p className="muted">loading&hellip;</p>
          : mine.length === 0 ? <p className="muted">none yet — open a table, or take a seat at one below.</p>
          : <ul>{mine.map(row)}</ul>}
        {lists?.mine.next && (
          <button type="button" className="ghost" onClick={() => void page(lists.mine.next!)}>more</button>
        )}
      </section>
      <section className="saved open-tables">
        <h3>Open tables</h3>
        {lists === null ? null
          : lists.open.games.length === 0 ? <p className="muted">no table is waiting for players.</p>
          : <ul>{lists.open.games.map(row)}</ul>}
      </section>
      {error && <div className="error">{error}</div>}
    </div>
  );
}
```

(`me` is accepted for the `who` composition and for the cells; if unused after wiring, drop it and its prop.) `Lobby.test.tsx` — over `stubApi` answering `/api/games?scope=mine` with `wire.games`, `?scope=open` with `wire.gamesOpen`, and `?scope=mine&limit…`/`before=` with `wire.gamesNext`: both sections render; the open row shows the lock marker (`aria-label="password required"`) for the locked table and not the other; the whose-turn word and "over · … won" render from fixtures with `winner` set (build by spreading `wire.table` with `over: true, winner: 1`, which is not hand-authoring a payload — it is a fixture with one field changed, the pattern `tables.test.ts` already uses); "more" appears when `next` is non-null and its click issues exactly one `GET` carrying `before=`; clicking a row sets `window.location.hash` to `#/t/<id>`; New table sets `#/new`.

- [ ] **Step 5: `App.tsx` — the route decides the screen; `open` sets the hash**

Replace the `if (!state) { return dealing ? <Dealing…/> : <NewGame…/> }` block with a route switch. `const route = useRoute();` near the top (after `useMe`). An effect follows the route:

```tsx
  useEffect(() => {
    if (!door.me?.user) return;
    if (route.kind === "table") {
      if (gameId.current !== route.id && dealing !== route.id) {
        open(route.id).catch((e) => {
          setError(e instanceof ApiError ? e.message : String(e));
          navigate({ kind: "lobby" });
        });
      }
      return;
    }
    // `#/` or `#/new`: no game on screen.
    if (state || dealing) show(null);
  }, [route, door.me?.user]);   // `open`/`show` are stable callbacks; list them if the lint asks
```

`open(id)` gains, at its top, `navigate({ kind: "table", id });` and, for an OPEN table, a third branch: `if (doc.status === "open") { show(null); setOpenTable(id); return; }` with `const [openTable, setOpenTable] = useState<string | null>(null)` (cleared in `show`). The `!state` render becomes:

```tsx
  if (!state) {
    if (route.kind === "new") return <NewGame me={door.me.user} onCreated={(t) => open(t.id)} who={who} />;
    if (dealing) return <Dealing … as today … />;
    if (openTable) return <OpenTable game={openTable} me={door.me.user} onDealing={() => { setOpenTable(null); setDealing(openTable); }} onLeave={() => navigate({ kind: "lobby" })} who={who} />;
    return <Lobby me={door.me.user} who={who} />;
  }
```

(`OpenTable` is Task 8's; until then, render `<Lobby …/>` for that branch too and leave a `// Task 8` comment — the reviewer sees Task 8 remove it.) `NewGame`'s `onOpen` prop is removed (the list is gone). Leaving a game (`GameMenu`'s Games button, `Dealing`'s `onLeave`) navigates to the lobby. The in-game `useSignal` consumer gains `onEnd` → `setError(sentence); navigate({kind:"lobby"})` — pass it as the fourth argument through `usePolled` directly or add an `onEnd` parameter to `useSignal` (add the parameter; default `undefined`).

`App.test.tsx`: the `renderApp` harness (`web/src/test/harness.tsx`) opens a game by finding the row button named `state.id` — it now finds it in the LOBBY under Your games (the stub already answers `/api/games` with `gamesNamed(state.id)`; make it answer `scope=mine` and `scope=open` — `gamesNamed` for mine, `{games: [], next: null}` for open). After the click, `window.location.hash` must read `#/t/<id>`; add `beforeEach(() => { window.location.hash = ""; })` in `harness.tsx`'s render path. Cells to add: "the first screen after sign-in is the lobby" (heading "Lobby"); "`#/new` renders the form"; "a `#/t/<id>` the server refuses shows the sentence over the lobby and the hash returns to `#/`" (stub `…/table` → `{status: 403, body: {error: "you are not seated at this table"}}`).

- [ ] **Step 6: `NewGame.tsx` re-pointed**

- Remove the `saved`/`savedTables`/`confirming` state, the `onOpen` prop, the `TableFacts` import and the whole `<section className="saved">`.
- `SeatRow.kind` gains `"open"`; `defaultSeats(n)` returns `me` at 0 and, for `n === 2`, `bot` at 1, otherwise `open` at 1..n; the seat `<select>` gains `<option value="open">open seat</option>`; `seatRequests()` maps `open` → `{ open: true }`.
- The Name field's placeholder becomes "blank — listed by its id"; nothing about ids is typed.
- A **Your name** field: `const [displayName, setDisplayName] = useState(() => displayName() ?? "")` with `api.ts`'s new `DISPLAY_NAME_KEY = "yokohama.display_name"`, `displayName(): string | null` and `setDisplayName(v: string)` (localStorage, try/catch as `claimedUser` does); on submit `setDisplayName(displayName.trim())` and `display_name` in the body when non-empty.
- A **Password** box: `<label><input type="checkbox" checked={locked} onChange=…/> Ask for a password to join</label>` revealing `<input id="password" type="password" aria-label="Password" …/>`; `password` in the body when the box is ticked and the field is non-empty after trim.
- The `Deal` button's text becomes "Open table" when any seat row is `open` and stays "Deal" otherwise (a cell pins both).

`NewGame.test.tsx`: the saved-list cells are deleted (the lobby's cover them); the in-flight cell `a double-clicked Deal … posts create exactly once` stays green; new cells: the open kind's defaults at 2/3/4P; `{open: true}` in the posted body; the password box reveals the field and the body carries `password` only when ticked and non-empty; the display name is read from storage on mount and written on submit; a name with a space is posted as `name`.

- [ ] **Step 7: The browser harness re-pointed (R-plan-5)**

`web/e2e/harness.ts`:

```ts
export async function openFromList(page: Page, id: string): Promise<void> {
  await expect(page.getByRole("heading", { name: "Lobby" })).toBeVisible();
  await page.getByRole("button", { name: id, exact: true }).click();
}
export async function startGame(page: Page, opts: { players?: number } = {}): Promise<Game> {
  const players = opts.players ?? 2;
  await signIn(page);
  await expect(page.getByRole("heading", { name: "Lobby" })).toBeVisible();
  await page.getByRole("button", { name: "New table" }).click();
  await expect(page.getByRole("heading", { name: "New game" })).toBeVisible();
  …the rest as today, unchanged…
}
```

`two_browsers.spec.ts`'s create flow: after `signIn(a, …)` add `await a.getByRole("button", { name: "New table" }).click();` before selecting Seat 1. `plays_a_whole_game`, `the_playback`, `the_screen_itself`, `the_table`, `gates`, `shots` go through `startGame` and need nothing.

- [ ] **Step 8: Run all three client gates and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests ' && npm run e2e 2>&1 | tail -3`
Expected: typecheck clean; vitest `447 + N`; e2e `38 passed` (the walk is green because the harness moved with the screen).

```bash
git add web/src/route.ts web/src/route.test.ts web/src/signal.ts web/src/signal.test.ts web/src/tables.ts web/src/tables.test.ts web/src/Lobby.tsx web/src/Lobby.test.tsx web/src/App.tsx web/src/App.test.tsx web/src/NewGame.tsx web/src/NewGame.test.tsx web/src/api.ts web/src/test/harness.tsx web/e2e/harness.ts web/e2e/two_browsers.spec.ts web/src/styles.css
git commit -m "feat(web client): hash routes (#/, #/new, #/t/<id>) — the lobby is the first screen with your games and the open tables, the form loses its list and gains open seats, a display name and a password box, one usePolled hook with a Source shape M6.3 keeps, and the browser harness goes through the lobby (spec §5.1–§5.3, §5.5, §5.6, §5.8)"
```

---

### Task 8: The open table — the seat picker and the waiting face; the name on keep; the seat surfaces (spec §5.4, §5.6, §3.4)

**Files:**
- Create: `web/src/OpenTable.tsx`, `web/src/OpenTable.test.tsx`
- Modify: `web/src/App.tsx`, `web/src/Dealing.tsx`, `web/src/Dealing.test.tsx`, `web/src/styles.css`
- Test: `web/src/OpenTable.test.tsx`, `web/src/Dealing.test.tsx`

**Interfaces:**
- Produces: `OpenTable` props `{ game: string; me: string; onDealing: () => void; onLeave: () => void; who?: ReactNode }`; `Dealing` posts `name` with a keep when storage holds a display name.

- [ ] **Step 1: `OpenTable.tsx`**

```tsx
import { useCallback, useRef, useState, type ReactNode } from "react";
import { api, ApiError, displayName, setDisplayName } from "./api";
import { seatClass } from "./Cards";
import { usePolled } from "./signal";
import { seatName } from "./tables";
import type { PlayerColor, TableView } from "./types";

export function OpenTable({ game, me, onDealing, onLeave, who }: {
  game: string; me: string; onDealing: () => void; onLeave: () => void; who?: ReactNode;
}) {
  const [table, setTable] = useState<TableView | null>(null);
  const [name, setName] = useState(() => displayName() ?? "");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  const [copied, setCopied] = useState(false);
  const inFlight = useRef(false);
  const dealing = useRef(onDealing);
  dealing.current = onDealing;
  const leave = useRef(onLeave);
  leave.current = onLeave;
  const link = usePolled<TableView>(
    { key: game, once: () => api.table(game), stream: `/api/games/${encodeURIComponent(game)}/stream` },
    true,
    useCallback((t: TableView) => {
      if (t.status !== "open") { dealing.current(); return; }
      setTable(t);
    }, []),
    useCallback((end) => { setError(end.sentence ?? "this table is gone"); leave.current(); }, []),
  );
  const join = async (seat: number) => {
    if (inFlight.current || !table) return;
    inFlight.current = true;
    setError(null);
    try {
      const trimmed = name.trim();
      if (trimmed) setDisplayName(trimmed);
      const t = await api.join(game, {
        seat,
        ...(trimmed ? { name: trimmed } : {}),
        ...(table.locked && password.trim() ? { password: password.trim() } : {}),
      });
      if (t.status !== "open") dealing.current(); else setTable(t);
    } catch (e) {
      if (e instanceof ApiError && e.status === 409 && "status" in ((e.body as object) ?? {})) { dealing.current(); return; }
      if (e instanceof ApiError && e.status === 409) { setTable(await api.table(game)); }
      setError(e instanceof ApiError ? e.message : String(e));
    } finally {
      inFlight.current = false;
    }
  };
  if (!table) return <div className="setup open-table"><p className="muted">loading&hellip;</p></div>;
  const seated = table.my_seats.length > 0;
  const colors = table.colors as PlayerColor[];
  const url = `${window.location.origin}/#/t/${encodeURIComponent(game)}`;
  const open = table.seats.filter((s) => s.kind === "open").length;
  return (
    <div className="setup open-table">
      {who && <div className="who-strip">{who}{link !== "live" && <span className="meta link" role="status">{link}&hellip;</span>}</div>}
      <h2>{table.name ?? table.id}</h2>
      <p className="lede">{seated ? `waiting for ${open} more` : "take a seat"}{table.locked && " · password required"}</p>
      {!seated && (
        <>
          <div className="field"><label htmlFor="your-name">Your name</label>
            <input id="your-name" value={name} onChange={(e) => setName(e.target.value)} /></div>
          {table.locked && (
            <div className="field"><label htmlFor="join-password">Password</label>
              <input id="join-password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} /></div>
          )}
        </>
      )}
      <ul className="seats">
        {table.seats.map((s, i) => (
          <li key={i} className={`field seat-row ${seatClass(colors[i]!)}`}>
            <span className="meta">Seat {i} · {colors[i]}</span>
            {s.kind === "open" && !seated
              ? <button type="button" className="primary" onClick={() => void join(i)}>sit here</button>
              : <span>{seatName(s)}</span>}
          </li>
        ))}
      </ul>
      {seated && (
        <div className="field">
          <label htmlFor="join-link">Link</label>
          <input id="join-link" readOnly value={url} />
          <button type="button" className="ghost" onClick={() => { void navigator.clipboard?.writeText(url); setCopied(true); }}>
            {copied ? "copied" : "copy"}
          </button>
        </div>
      )}
      {seated && table.created_by === me && (
        confirming
          ? <><button type="button" className="ghost danger" aria-label={`confirm abandon ${table.name ?? table.id}`}
                onClick={() => void api.remove(game).then(() => leave.current()).catch((e) => setError(String(e)))}>confirm abandon</button>
             <button type="button" className="ghost" onClick={() => setConfirming(false)}>cancel</button></>
          : <button type="button" className="ghost danger" onClick={() => setConfirming(true)}>Abandon</button>
      )}
      <button type="button" className="ghost" onClick={() => leave.current()}>Lobby</button>
      {error && <div className="error">{error}</div>}
    </div>
  );
}
```

(`ApiError` needs a `body` field carrying the parsed error object for the `"status" in body` test: add `public readonly body: unknown` to the class in `api.ts` and set it in `call` where the error is thrown — the `DealingErrorBody` shape is what it carries on a status refusal.) `styles.css` gains `.open-table .seats { list-style: none; padding: 0 }` and nothing else new; the classes used are the setup screens' own.

`OpenTable.test.tsx` — over `wire.tableOpen` (the stranger's face) and `wire.table` spread to `status: "open"` with `my_seats: [0]` and an `open` seat (the seated face): the stranger sees the name field, the password field (locked) and one "sit here" button; clicking it posts `join` with `{seat: 1, name, password}`; a 403 reply (`wire.join403Password`) shows its sentence and keeps the fields; a `wire.join409Taken` reply refetches `/table` once; the seated face shows the link with the origin and `#/t/<id>`, the copy button, and Abandon only for the creator; abandon's first click shows confirm/cancel and the second posts DELETE once; a poll tick reading `dealing` calls `onDealing`; a 404 on the poll calls `onLeave` (through `onEnd`).

- [ ] **Step 2: `Dealing.tsx` — the name rides with the keep**

In `keep(card)`: `const n = displayName()?.trim(); const table = await api.keep(game, deal.seat, card, n || undefined);` and `api.keep` gains an optional fourth argument `name?: string` posted as `name` when present. `Dealing.test.tsx`: a cell with storage holding "Jeremy" posts `{seat, card, name: "Jeremy"}`; without it, no `name` key. The waiting line and the chosen/waiting sections already word seats through `holder`/`seatName`, so they read names now; one cell renders `wire.dealWaiting` and asserts the name "Blue" appears where the email did.

- [ ] **Step 3: `App.tsx` — the `OpenTable` branch (replacing Task 7's placeholder)**

Wire the `openTable` branch exactly as Task 7's render block shows, import `OpenTable`, and delete the Task 7 comment.

- [ ] **Step 4: Run the client gates and commit**

Run: `cd web && npm run typecheck && npm test 2>&1 | grep -E 'Test Files|Tests ' && npm run e2e 2>&1 | tail -3`
Expected: all green; e2e `38`.

```bash
git add web/src/OpenTable.tsx web/src/OpenTable.test.tsx web/src/App.tsx web/src/Dealing.tsx web/src/Dealing.test.tsx web/src/api.ts web/src/styles.css
git commit -m "feat(web client): the open table — a stranger's seat picker with a name and a password, a seated person's waiting face with the copyable link and the creator's two-click abandon, the hand-over to the deal on the flip; the keep carries the remembered name (spec §5.4, §5.6)"
```

---

### Task 9: The gates — the two-identity arm through the lobby; the join-by-link browser walk (spec §8.3, §8.4)

**Files:**
- Modify: `crates/yokohama-web/tests/play_a_whole_game.rs`
- Create: `web/e2e/joins_by_link.spec.ts`

- [ ] **Step 1: The gate arm**

In `two_people_play_to_a_winner_and_each_sees_only_their_own_hand`, replace the create body and the keep loop's opening:

```rust
    let (status, table) = api
        .call_as(red, "POST", "/api/games", Some(serde_json::json!({
            "players": 2, "colors": ["Red", "Blue"], "start_seat": 0,
            "seed": 20260914u64, "id": "two", "name": "two",
            "seats": [{ "user": red }, { "open": true }],
            "password": "sesame",
        })))
        .await;
    assert_eq!(status, StatusCode::CREATED, "{table}");
    assert_eq!(table["status"], "open", "{table}");
    let (status, listed) = api.call_as(blue, "GET", "/api/games?scope=open", None).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert!(listed["games"].as_array().unwrap().iter().any(|t| t["id"] == "two"), "blue finds it: {listed}");
    let (status, refused) = api.call_as(blue, "POST", "/api/games/two/join", Some(serde_json::json!({ "seat": 1 }))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "locked: {refused}");
    let (status, joined) = api
        .call_as(blue, "POST", "/api/games/two/join", Some(serde_json::json!({ "seat": 1, "name": "Blue", "password": "sesame" })))
        .await;
    assert_eq!(status, StatusCode::OK, "{joined}");
    assert_eq!(joined["status"], "dealing", "{joined}");
    assert_eq!(joined["seats"][1]["name"], "Blue", "{joined}");
```

The keep loop and everything after it are unchanged. The vs-bot arm is unchanged.

Run: `cargo test -p yokohama-web --test play_a_whole_game`
Expected: `2 passed` (this is the gate's OS-entropy game; a capped run is the LEAVE-listed behaviour and is re-run once, recorded).

- [ ] **Step 2: `joins_by_link.spec.ts`**

```ts
import { expect, test } from "@playwright/test";
import { haveBrowser } from "../playwright.config";
import { act, readScreen, rng, signIn, watchForBreakage } from "./harness";

test.skip(!haveBrowser, "no Chromium — YOKOHAMA_WEB_E2E_OPTIONAL=1; this run tested NOTHING");

test("a person holding only a link joins a locked open table and plays", async ({ browser }) => {
  test.setTimeout(5 * 60 * 1000);
  const a = await (await browser.newContext()).newPage();
  const b = await (await browser.newContext()).newPage();
  const watchA = watchForBreakage(a);
  const watchB = watchForBreakage(b);

  await signIn(a, "a@e2e.test");
  await a.getByRole("button", { name: "New table" }).click();
  await a.getByLabel("Seat 1").selectOption("open");
  await a.getByLabel("Name").fill("Link test");
  await a.getByLabel("Your name").fill("Ann");
  await a.getByLabel("Ask for a password to join").check();
  await a.getByLabel("Password").fill("sesame");
  const created = a.waitForResponse(
    (r) => /\/api\/games$/.test(new URL(r.url()).pathname) && r.request().method() === "POST",
  );
  await a.getByRole("button", { name: "Open table" }).click();
  const id = String(((await (await created).json()) as { id: string }).id);
  expect(id).toBeTruthy();
  await expect(a).toHaveURL(new RegExp(`#/t/${id}$`));
  await expect(a.getByLabel("Link")).toHaveValue(new RegExp(`/#/t/${id}$`));
  const navigationsOnA: string[] = [];
  a.on("framenavigated", (f) => { if (f === a.mainFrame()) navigationsOnA.push(f.url()); });

  // B arrives by the LINK alone: never through the lobby.
  await signIn(b, "b@e2e.test", `/#/t/${id}`);
  await expect(b.getByRole("heading", { name: "Link test" })).toBeVisible();
  await expect(b.getByText(/password required/)).toBeVisible();
  await b.getByLabel("Your name").fill("Bob");
  await b.getByLabel("Password").fill("wrong");
  await b.getByRole("button", { name: "sit here" }).click();
  await expect(b.getByText(/wrong password/)).toBeVisible();
  await b.getByLabel("Password").fill("sesame");
  await b.getByRole("button", { name: "sit here" }).click();

  // A's screen learns without a reload.
  await expect(a.getByText(/Bob/)).toBeVisible({ timeout: 15_000 });
  expect(navigationsOnA, "A did not navigate while B sat down").toEqual([]);

  // Both keep and play six moves between them.
  for (const p of [a, b]) {
    await p.locator(".app.setup-table.dealing button.deal-pick:not([disabled])").first().click({ timeout: 30_000 });
  }
  await expect(a.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  await expect(b.getByText(/end triggers:/)).toBeVisible({ timeout: 30_000 });
  const pick = rng(0x4c49_4e4b); // "LINK"
  let moves = 0;
  while (moves < 6) {
    for (const p of [a, b]) {
      const s = await readScreen(p);
      if (s.over) return;
      if (s.inert === null && (s.options > 0 || s.tiles > 0 || s.counts > 0 || s.confirm || s.only)) {
        await act(p, s, pick);
        moves += 1;
      }
    }
    await a.waitForTimeout(500);
  }

  // A's lobby lists it under Your games with Bob's name and whose turn it is.
  await a.getByRole("button", { name: "Lobby" }).or(a.getByRole("button", { name: "Games" })).first().click();
  const row = a.locator(".lobby .saved li", { hasText: "Link test" });
  await expect(row).toBeVisible();
  await expect(row).toContainText(/Bob/);
  await expect(row).toContainText(/your turn|waiting on/);
  watchA.assert();
  watchB.assert();
});
```

(`signIn`'s third parameter is the URL it opens; `watchForBreakage(page)` returns an object with `assert()` — read `harness.ts:50` for its real name and use it. `act(page, screen, pick)` is the harness's mover; read its signature at `harness.ts:317`.)

Run: `cd web && npm run e2e 2>&1 | tail -4`
Expected: `39 passed`; `every_spec_can_skip` counts the new file.

- [ ] **Step 3: Commit**

```bash
git add crates/yokohama-web/tests/play_a_whole_game.rs web/e2e/joins_by_link.spec.ts
git commit -m "test(web): the two-identity gate arm arrives through the lobby — an open, locked table found under scope=open and joined with the password; joins_by_link.spec.ts has a second browser arrive by the address alone, refuse on the wrong password, sit, and play six moves while the creator's screen learns without a reload (spec §8.3)"
```

---

### Task 10: Deploy `yokohama-web:0.5.0-m6.2` to the NAS (spec §9)

**Files:**
- Modify: `deploy/RUNBOOK.md` (a §3.8 addendum), the version in `crates/yokohama-web/Cargo.toml` if the owner moves it (the tag is the image's)

- [ ] **Step 1: Owner authorization.** The orchestrator asks; the deploy is outward-facing and runs through the Portainer MCP against environment 2 exactly as §3.7 records M6.1's. The owner deletes `m6.1-live-review` first, as the M6.1 close asked.

- [ ] **Step 2: Build and deliver** per `deploy/RUNBOOK.md` §1 (musl cross-compile), §2 (context tarball over `python3 -m http.server`), §3 (the container create body with `YOKOHAMA_WEB_IDENTITY=claimed`), reading the workstation's LAN address from `ifconfig` for the fetch.

- [ ] **Step 3: Verify from the shell** (WARP on): `curl -s -H 'X-Yokohama-Claimed-User: <owner email>' https://yokohama.moore.nyc/api/games?scope=open` answers `{"games":[…],"next":null}`; a create with one open seat and a password by one header, a join by a second header with the wrong password → 403, with the right one → 200 and `"status":"dealing"`; `/api/health` shows the new `server_version`.

- [ ] **Step 4: Record** §3.8 in the runbook in §3.7's shape: the commit built, the binary's size and the bundle name, the tarball's size and sha prefix, the container id and image digest, the two verifications with timestamps.

```bash
git add deploy/RUNBOOK.md
git commit -m "docs(deploy): the fifth deployment — yokohama-web:0.5.0-m6.2 (the lobby) on the NAS, verified from the shell: an open locked table created by one identity and joined by another"
```

---

### Task 11: The final review against the live site, and the close

**Files:**
- Create: `docs/plans/2026-09-15-web-frontend-m6.2-close.md` (dated the day it is written)
- Modify: `CLAUDE.md` (the Web bullets' counts and the M6.2 line), `guidingpath.md` (one decision-log row), `README.md` (the status line), `docs/handover.md` (Track 3)

- [ ] **Step 1: The mutation battery** (spec §8.4, ten mutations): an Opus subagent applies each to the local build one at a time, runs the named cells, records the first failing assertion verbatim, reverts. A mutation no cell catches is a finding for the close, not a reason to write a cell after the fact without saying so.

- [ ] **Step 2: The live review**: a CLEAN Fable subagent, no prior context, against `https://yokohama.moore.nyc` in BOTH layouts, from two browsers as two people: one creates an open locked table, the other arrives by the link alone, types the password, sits; both deal and play; the lobby shows whose turn it is; a stranger's list shows the open table and not the playing one; abandon at an open table; the mobile stack of the lobby and the open table read at 390 px. Verdict and findings in the close's §4.

- [ ] **Step 3: The counts in one sequence**: `cargo test -p yokohama-web`, clippy, `cargo test -p yokohama-cli` (untouched, 850 + 2), `cd web && npm run typecheck && npm test`, `npm run e2e`, `ls web/src/wire/*.json | wc -l`; the blast-radius diff EMPTY.

- [ ] **Step 4: The close** in the M6.1 close's shape (§1 what shipped, §2 the counts, §3 the corpus diff, §4 the reviews, §5 the rulings taken in execution with their costs, §6 the LEAVE list — inherited and this cycle's, §7 what is not claimed, §8 the handover into M6.3, which names what moved under the M6.3 spec's line citations). Then the doc pass: `CLAUDE.md`'s two Web bullets (counts, the join route, the lobby), one `guidingpath.md` row, `README.md`'s status line, `docs/handover.md` Track 3.

```bash
git add docs/plans/2026-09-15-web-frontend-m6.2-close.md CLAUDE.md guidingpath.md README.md docs/handover.md
git commit -m "docs(web): M6.2 close — the lobby shipped and live; the reviews' findings; the handover into M6.3"
```

---

## Deviations from the spec (recorded per R-P11; the close carries the costs)

- R-plan-1 through R-plan-6 above. The close lists any taken in execution beside them.

## Self-review (run before the plan is handed over)

- **Spec coverage.** §2 → Task 2; §3.1/§3.4 status arms → Task 3; §3.2/§3.5 → Task 4; §3.3/R-2.7/§6 sweep → Task 5; §3.6 → Task 1; §4 → Tasks 2 (lock), 3 (create), 4 (join), 6 (the pin); §5.1–§5.3, §5.5, §5.6, §5.8 → Task 7; §5.4, §5.6's surfaces, `Keep.name` on the client → Task 8; §5.7 → Task 6; §7 → Task 6; §8.1 → Tasks 2–5; §8.2 → Tasks 6–8; §8.3 → Tasks 7 (harness), 9; §8.4 → Task 11; §8.5 → every task's gate; §9 → Task 10; §10/§11 → the close.
- **Placeholders.** Three `…` ellipses stand for "the existing code, unchanged" and say so beside a named line (`config_of`'s tail, `create`'s document, the gated store's construction, `startGame`'s tail); none stands for work to be invented.
- **Type consistency.** `Games::join(id, user, seat, name: Option<&str>, password: Option<&str>)` in Task 4 is what `lib.rs`'s `join` handler calls with `req.name.as_deref()`, `req.password.as_deref()`; `keep`'s fifth parameter `name: Option<&str>` is passed `None` by the gate and the walk; `Games::list(user, scope, limit, before: Option<(u64, String)>)` returns `(Vec<TableView>, Option<String>)`, which `list_games` maps into `GamesList { games, next }`; `usePolled`'s `Source<T> { key, once, stream }` and `End` are the shapes `Lobby` and `OpenTable` pass and M6.3 keeps; `api.join(game, req: JoinRequest)` and `api.list(scope, before)` are what `OpenTable` and `Lobby` call.
